//! The loop, which is the same one every host on this boundary runs: a command in, the events it
//! produced answered until nothing is left, and the window told what changed (ADR 0246; filed here
//! by ADR 1908).

use std::collections::VecDeque;

use viewer_core::{Answer, Command, DocumentId, Query};
use viewer_host::PresentationPump as _;
use viewer_host::form::control_kind;
use viewer_host::native_window::NativeWindow;
use viewer_host::trace::Topic;

use crate::bridge::ffi::QtUpdate;

use super::forms::{Placement, placement};
use super::{DOCUMENT, Host, named, nothing_changed};

impl Host {
    /// What has changed since this was last called, which also clears it.
    pub(crate) fn take_update(&mut self) -> QtUpdate {
        let mut update = std::mem::replace(&mut self.update, nothing_changed());
        update.documents = std::mem::take(&mut self.documents_moved);
        update
    }

    /// One command, and everything it produces.
    pub(crate) fn dispatch(&mut self, command: Command) {
        self.pump(vec![command]);
    }

    /// Runs commands until nothing is left, reacting to what each produces, then refreshes.
    ///
    /// **The outer loop is the drawing thread's** (ADR 0668). A page that finished while the
    /// commands were being run answers the viewer with a `RenderReady`, which produces more events
    /// — so the queue is drained again rather than the answer waiting for the next turn of the
    /// timer.
    pub(super) fn pump(&mut self, queue: Vec<Command>) {
        self.drain(queue.into());
    }

    /// [`Self::pump`] over a queue already made: the reactions to events answered outside it put
    /// their commands on one.
    pub(super) fn drain(&mut self, mut queue: VecDeque<Command>) {
        loop {
            while let Some(command) = queue.pop_front() {
                // Table 166's `/M` is what §7.5.6's update writes it into, so this window reads
                // its clock on the way to a save rather than keeping one. Which command that is
                // is `viewer_host::modification::before`'s, once for all three windows, because
                // a host reading its clock somewhere else reads it at a different moment (ADR
                // 1160).
                if let Some(clock) = viewer_host::modification::before(&command) {
                    self.run(clock, &mut queue);
                }
                self.run(command, &mut queue);
            }
            self.ask_for_windows();
            self.take_the_thread_back();
            self.take_the_drawn(&mut queue);
            // A tab opened behind is in front for the first time once `Command::Focus` has run,
            // which is when the core answers `Query::Opening` for it (ADR 1303).
            if queue.is_empty() && std::mem::take(&mut self.state.showing.catalog_due) {
                self.obey_the_catalog(&mut queue, true);
                self.state.pump.owe_synchronizing();
            }
            // A tab brought to the front owes the core its presentation's mode and the window its
            // chrome once the change's own commands have run (ADR 2300).
            self.synchronize_if_owed(&mut queue);
            if queue.is_empty() {
                break;
            }
        }
        if std::mem::take(&mut self.panels_due) {
            self.build_panels();
        }
        self.refresh();
    }

    /// One command through the viewer, with what it produced put back on the queue.
    ///
    /// The body of [`Host::pump`]'s inner loop, named so that a command run *ahead* of another —
    /// Table 166's clock before a save — goes through the same trace and the same reactions as
    /// one a person sent.
    fn run(&mut self, command: Command, queue: &mut VecDeque<Command>) {
        let described = self
            .state
            .trace
            .on(Topic::Events)
            .then(|| viewer_host::trace::described(&command, 120));
        // **A command that changes the document changes what §14.7's tree says**, and
        // `Showing` cannot see it: an edit and a click move neither the page nor the
        // viewport. Which commands those are is one statement for all three windows
        // (`viewer_accessibility::republishes`, ADR 0623).
        if viewer_accessibility::republishes(&command) {
            self.spoken = None;
        }
        let supplied = matches!(command, Command::Supply { .. });
        // A command that waits behind a turn's read is handed on when the read is in (ADR 2344).
        let Some(events) = self.through_the_viewer(command) else {
            return;
        };
        if let Some(described) = described {
            self.state.trace.say(
                Topic::Events,
                format_args!("{described} -> {} event(s)", events.len()),
            );
        }
        for event in events {
            self.react(event, queue);
        }
        // Every event the supply caused has been seen, so an offer nothing took is spent.
        if supplied {
            self.state.arrivals.supplied();
        }
    }

    /// A document opened: the first, one beside it, or one a reader named on its way to a tab.
    pub(super) fn opened(
        &mut self,
        document: DocumentId,
        pages: usize,
        queue: &mut VecDeque<Command>,
    ) {
        self.state
            .trace
            .say(Topic::Launch, format_args!("opened, {pages} page(s)"));
        // A document that arrived under the name this window held out for
        // `/NewWindow true` is a *second* document rather than this one reopened, so it
        // gets a tab of its own before anything else is said about it (ADR 1263).
        let behind = self.opened_beside(document);
        // Trap 5, and the same sentence the other two hosts say — see `viewer-gtk`.
        if pages == 0 {
            self.say(&viewer_host::no_pages(&named(&self.state.showing.path)));
        }
        self.state.showing.asking.opened();
        // A document opens at the window's levels, so the menu's ticks go back to them
        // too — the core's `Open` forgets the departures (ADR 1145).
        self.state.restrictions.opened();
        self.state.showing.report_due.opened();
        self.obey_the_catalog(queue, behind.is_none());
        self.state.showing.catalog_due = behind.is_some();
        self.build_panels();
        // A document named behind the first gives the front back once it has its tab —
        // this window's swap here and the core's focus on the queue, because this runs
        // inside the pump that opened it — and the next one waiting starts. The first
        // document's own start is its first frame, unless it has no page to put in one.
        if let Some(front) = behind
            && self.take_the_front(front, true)
        {
            queue.push_back(Command::Focus(front));
            self.panels_due = true;
        }
        if document != DOCUMENT || pages == 0 {
            self.later_open_the_next();
        }
    }

    /// §7.6.4.1's prompt, for the document in front or for one on its way to a tab.
    pub(super) fn password_required(&mut self, document: DocumentId) {
        if !viewer_host::documents::may_ask_about(&mut self.state.arrivals, document, DOCUMENT) {
            self.say(&viewer_host::documents::not_asked(document));
            return;
        }
        let (asked, about) = match self.state.arrivals.named_mut(document) {
            Some(arriving) => (
                arriving.asking.required(),
                viewer_host::documents::label(&arriving.named.path),
            ),
            None => (
                self.state.showing.asking.required(),
                named(&self.state.showing.path),
            ),
        };
        match asked {
            viewer_host::Ask::Prompt { attempt, of } => {
                let words = viewer_host::password::prompt(&about, attempt, of);
                // One `QLabel` for both, which is this toolkit's arrangement of the same
                // two sentences `viewer-gtk` puts in two labels and the card draws in two
                // colours.
                self.prompt = format!("{}\n{}", words.question, words.counted);
                self.prompting = document;
                self.update.password = true;
            }
            viewer_host::Ask::Exhausted => {
                self.say(viewer_host::password::EXHAUSTED);
                self.cancelled(document);
            }
        }
    }

    /// Recomputes what the window shows, and says what of it changed.
    ///
    /// Idempotent and called after every pump, which is what keeps the window a function of the
    /// viewer's state rather than of the order events arrived in. ADR 0244 finding 5 records why
    /// both native hosts make that choice: `Event::Damage` covers a moved page mapping in
    /// practice but is documented as "a bound on what changed", so a host that placed controls
    /// from it would be relying on a coincidence.
    fn refresh(&mut self) {
        self.update.frame = true;
        self.update.chrome = true;
        let fields = match self.state.viewer.query(Query::Fields) {
            Answer::Fields(fields) => fields,
            _ => Vec::new(),
        };
        let placed: Vec<Placement> = fields
            .iter()
            .flat_map(|field| {
                let kind = control_kind(&field.control);
                field
                    .widgets
                    .iter()
                    .filter_map(move |widget| placement(field, widget, &kind))
            })
            .collect();
        let keys: Vec<(String, pdf_syntax::ObjectId)> = placed.iter().map(Placement::key).collect();
        let was: Vec<(String, pdf_syntax::ObjectId)> =
            self.placed.iter().map(Placement::key).collect();
        if keys != was {
            self.update.controls = true;
            self.state.trace.say(
                Topic::Panel,
                format_args!(
                    "{} field(s) on the page, {} control(s) placed",
                    fields.len(),
                    placed.len()
                ),
            );
        }
        self.fields = fields;
        self.placed = placed;
        // §12.5.6.14's windows, which are widgets and not pixels: rebuilt when the set changes or
        // one of them moves, and left alone otherwise. Reported on what the *answer* held rather
        // than on what could be placed, so that a window with no area is a line rather than a
        // silence.
        let popups = match self.state.viewer.query(Query::Popups) {
            Answer::Popups(windows) => windows,
            _ => Vec::new(),
        };
        if popups != self.popups_shown {
            self.update.popups = true;
            if !popups.is_empty() || !self.popups_shown.is_empty() {
                self.state.trace.say(
                    Topic::Panel,
                    format_args!(
                        "{} of {} §12.5.6.14 popup window(s) placed",
                        viewer_host::popup::windows(&popups).len(),
                        popups.len()
                    ),
                );
            }
            self.popups_shown = popups;
        }
    }
}
