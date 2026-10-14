//! The loop, which is the same one every host on this boundary runs: a command in, the events it
//! produced answered until nothing is left, and the window told what changed (ADR 0246; filed here
//! by ADR 1908).

use std::collections::VecDeque;

use viewer_core::{Answer, Command, DocumentId, Event, Purpose, Query};
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
            }
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
        let events: Vec<Event> = if matches!(command, Command::GoTo(_)) {
            self.turn(command)
        } else {
            self.state.viewer.handle(command).collect()
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
    fn opened(&mut self, document: DocumentId, pages: usize, queue: &mut VecDeque<Command>) {
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
        self.restrictions.opened();
        self.state.showing.report_due.opened();
        self.obey_the_catalog(queue, behind.is_none());
        self.state.showing.catalog_due = behind.is_some();
        self.build_panels();
        // A document named behind the first gives the front back once it has its tab —
        // this window's swap here and the core's focus on the queue, because this runs
        // inside the pump that opened it — and the next one waiting starts. The first
        // document's own start is its first frame, unless it has no page to put in one.
        if let Some(front) = behind
            && self.take_the_front(front)
        {
            queue.push_back(Command::Focus(front));
            self.panels_due = true;
        }
        if document != DOCUMENT || pages == 0 {
            self.later_open_the_next();
        }
    }

    /// §7.6.4.1's prompt, for the document in front or for one on its way to a tab.
    fn password_required(&mut self, document: DocumentId) {
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

    /// Does what one event asks.
    pub(super) fn react(&mut self, event: Event, queue: &mut VecDeque<Command>) {
        match event {
            Event::Opened { document, pages } => self.opened(document, pages, queue),
            Event::OpenFailed { document, reason } => self.open_failed(document, &reason),
            // §7.6.4.1: "the interactive PDF processor should prompt for a password". The prompt
            // is a window, and a window is a host's — which is the whole reason this event exists
            // rather than a refusal. How many times to ask is `viewer_host::password`'s, because
            // the clause states no number and three hosts held three copies of the same three.
            //
            // Exhaustive over `Ask` on purpose: a case added there fails to compile here.
            //
            // A document on its way to a tab of its own counts its own prompts, because the count in
            // this window's fields is the one in front's.
            Event::PasswordRequired { document } => self.password_required(document),
            // Two events with nothing to do here, for two different reasons. A close drops
            // everything derived from the document and this host opens one document and never
            // closes it; damage is what a tier-1 host repaints from `Query::Frame`, and the
            // refresh at the end of every pump has already been scheduled by the time this
            // arrives.
            // A file held for a §12.7.8 named page's password is read and put down again, and the
            // name it was asked under is closed so this window stops holding it (ADR 1335).
            Event::Closed(document) if self.state.arrivals.is(document) => self.given_up(document),
            Event::Closed(_) | Event::Damage(_) => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => self.turned(index, label.as_deref(), of, section.as_deref()),
            // **Handed to the drawing thread rather than drawn here** (ADR 0668): `rasterize`
            // inside `QApplication::exec` would let a page written to draw for 27.6 s take the
            // window with it — no repaint, no key, and no thread from which `pdf_render::Interrupt`
            // could be raised. What comes back arrives in `take_the_drawn`.
            // The rows the window shows are drawn first, and the rest of the page after (ADR 1868).
            // A page zoomed past its window is held, and drawn as the window shows it
            // once the commands are run (ADRs 1892, 1930).
            Event::NeedsRender(request) => {
                self.state.retained.asked(&request);
                if !self.state.oversized.asked(&request, self.state.viewport) {
                    let seen = self.seen_of(&request);
                    self.state.drawing.ask_showing(request, seen);
                }
            }
            // §12.6.4.8: resolved against this document's own location where the action left it
            // partial, then opened, declined or put to the person by the one policy three windows
            // share — `CLAUDE.md`'s four levels over the act of starting another program on a
            // string the *document* chose (ADRs 1079, 1155). What this arm owns is the dialogue.
            Event::OpenUri { uri, .. } => self.follow_the_link(&uri),
            // §12.7.6.2: composed by `viewer-core`, and whether this machine transmits it is
            // `viewer_host::may_submit`'s one answer rather than this window's (ADRs 1062, 1291).
            Event::Submit {
                document,
                submission,
            } => self.submit(document, *submission),
            // Three of the five purposes are asked at one of four levels and §12.7.6.4's own
            // file is not, and the difference is what each does: an import puts another file's
            // *values* into the document being read, while a remote go-to, a thread in another
            // file and a named page each make this program parse a second PDF — which is the act
            // a person may want to be asked about (ADRs 1227, 1239).
            Event::NeedsFile {
                purpose,
                name,
                beside,
                ..
            } if viewer_host::under_remote_documents(purpose) => {
                self.remote(purpose, &name, beside, queue);
            }
            // An import named by an absolute URI is fetched rather than read, under the level a
            // submission is sent at (ADR 1527).
            Event::NeedsFile {
                document,
                purpose: Purpose::ImportData,
                name,
                ..
            } if viewer_host::import_is_fetched(&name) => self.fetch_import(document, name, queue),
            Event::NeedsFile { purpose, name, .. } => self.import(purpose, &name, queue),
            Event::SignaturePoliciesPublished { document, policies } => {
                self.fetch_policies(document, policies);
            }
            // §12.4.4.1: played since this host was given a clock, and named where it is not.
            //
            // A transition outside a presentation is not drawn at all — there is no clock to draw
            // it on — and a style `viewer_core::transition` does not shape is refused before two
            // pages are rasterised for it. Both are said rather than swallowed.
            Event::Transition { transition, .. } => self.arm_transition(transition),
            Event::Extracted {
                document,
                asked,
                name,
                bytes,
                fragment,
            } => self.extracted(document, asked, &name, bytes, fragment),
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            Event::Dirty { dirty, .. } => {
                self.state.showing.dirty = dirty;
                self.update.title = true;
            }
            Event::Searched {
                found,
                remaining,
                wrapped,
                ..
            } => self.searched(found, remaining, wrapped),
            Event::Reported { page, notes, .. } => self.reported(page, &notes),
            // `CLAUDE.md`: a document's restrictions are the reader's to set, and it shall always
            // be possible to turn them off. `CLAUDE.md`: a document's restrictions are the reader's
            // to set, and it shall always be possible to turn them off. The sentence is
            // `viewer_host::refused` rather than this host's own because it names the word the
            // argument parser takes (ADR 0604).
            Event::Refused { notes, .. } => self.say(&viewer_host::refused(&notes)),
            // The other two of `CLAUDE.md`'s four levels (ADR 0814). *Warn* is a sentence after an
            // edit that went ahead. *Ask* is the question this window puts, in the modal shape
            // §7.6.4.1's password already had: the words are `viewer_host::restriction`'s and the
            // dialogue is C++'s, because Rust does not call a Qt object (ADR 1145).
            Event::Warned { notes, .. } => self.say(&viewer_host::warned(&notes)),
            Event::Printing {
                pages, fidelity, ..
            } => self.showing_what_would_print(pages, fidelity),
            Event::Copied {
                logical,
                page_order,
                ..
            } => {
                if let Some(copied) = viewer_host::copied(logical, &page_order) {
                    self.say(&format!(
                        "copied {} characters in {}",
                        copied.text.chars().count(),
                        copied.order
                    ));
                    self.clipboard = copied.text;
                    self.update.clipboard = true;
                }
            }
            asking @ (Event::Asking { .. } | Event::AskingToRunScripts { .. }) => self.ask(&asking),
            // RFC 0008 section 4.2's `app.alert` and `app.response`: a dialogue of its own, since
            // its buttons are the script's and an entry may stand under its text (ADR 1628).
            Event::ScriptAsking { document, question } => {
                let name = self.state.documents.label_of(document);
                self.say(&viewer_host::script_asks::put(&name, &question));
                self.script_question = Some((document, name, question));
                self.update.script_question = true;
            }
            // The question's wait ran out and the runner answered it as a closed dialogue does:
            // taken here, so the dialogue's `poll_script_question` sees it gone and closes, and the
            // answer it then sends reaches nothing (ADR 1643).
            Event::ScriptQuestionWithdrawn { document } => self.withdraw_script_question(document),
            // `app.beep`: `QApplication::beep`, the one system sound, whichever of the five was
            // asked for — a flag for `clipboard`'s reason, since Rust never calls Qt (ADR 1702).
            Event::Beep { document, sound } => self.beep(document, sound),
            // This window's console is its log, so the request is said (ADR 1771).
            Event::Console { document, request } => self.say(&viewer_host::script_timers::console(
                &self.state.documents.label_of(document),
                request,
            )),
            // §7.11.4's list moved under the files tab: rebuilt from the same answer it was
            // built from, which is the only thing a window may do here this round — display
            // the list it already shows.
            Event::AttachmentsChanged { .. } => self.build_panels(),
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
