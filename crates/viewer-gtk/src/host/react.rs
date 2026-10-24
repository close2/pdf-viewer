//! The loop: a command dispatched, the events it produced answered until none is left, and the window refreshed from what the viewer then says.

use std::collections::VecDeque;

use gtk4::prelude::*;
use viewer_core::{Answer, Command, DocumentId, Event, Query};

use viewer_host::PresentationPump as _;
use viewer_host::native_window::NativeWindow;
use viewer_host::trace::Topic;

use super::stand_in::{Pictures, place};
use super::window::named;
use super::{DOCUMENT, Host};

impl Host {
    /// One command, and everything it produces.
    pub(crate) fn dispatch(&mut self, command: Command) {
        self.pump(VecDeque::from([command]));
    }

    /// Runs commands until nothing is left, reacting to what each produces, then repaints.
    ///
    /// **The outer loop is the drawing thread's** (ADR 0668). A page that finished while the
    /// commands were being run answers the viewer with a `RenderReady`, which produces more events
    /// — so the queue is drained again rather than the answer waiting for the next wakeup.
    pub(super) fn pump(&mut self, mut queue: VecDeque<Command>) {
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
        self.pump_search();
        self.pump_presentation();
        self.pump_drawing();
    }

    /// One command through the viewer, with what it produced put back on the queue.
    ///
    /// The body of [`Self::pump`]'s inner loop, named so that a command run *ahead* of another —
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
        let behind = self.opened_beside(document, queue);
        // Trap 5: a page tree with no leaves is a *correctly read* document with nothing to show,
        // and a blank window is what a broken file looks like too. Said in all three hosts
        // (ADR 0564).
        if pages == 0 {
            self.say(&viewer_host::no_pages(&named(&self.state.showing.path)));
        }
        self.state.showing.asking.opened();
        // A document opens at the window's levels, so the menu's ticks go back to them
        // too — the core's `Open` forgets the departures and a menu that did not would
        // tick the last document's (ADR 1145).
        self.state.restrictions.opened();
        self.state.showing.report_due.opened();
        self.obey_the_catalog(queue, behind.is_none());
        self.state.showing.catalog_due = behind.is_some();
        // The core has already focused this document, so its panels are built now rather than
        // after the pump — unless a front given back below asks for them again.
        self.build_panels();
        self.panels_due = false;
        // A document named behind the first gives the front back once it has its tab, and
        // the next one waiting starts. The first document's own start is its first frame,
        // unless it has no page to put in one (ADR 1275).
        if let Some(front) = behind {
            self.show_document(front, queue, true);
        }
        if document != DOCUMENT || pages == 0 {
            self.later_open_the_next();
        }
    }

    /// §7.6.4.1's prompt, about the document the core named and counted against it.
    ///
    /// A document on its way to a tab of its own counts its own prompts, because the count in this
    /// window's fields is the one in front's; `viewer_host::documents::may_ask_about` says which
    /// documents a prompt may be put about at all (ADR 1332).
    pub(super) fn password_required(&mut self, document: DocumentId) {
        if !viewer_host::documents::may_ask_about(&mut self.state.arrivals, document, DOCUMENT) {
            self.say(&viewer_host::documents::not_asked(document));
            return;
        }
        let asked = match self.state.arrivals.named_mut(document) {
            Some(arriving) => arriving.asking.required(),
            None => self.state.showing.asking.required(),
        };
        match asked {
            viewer_host::Ask::Prompt { attempt, of } => {
                self.ask_for_a_password(document, attempt, of);
            }
            viewer_host::Ask::Exhausted => {
                self.say(viewer_host::password::EXHAUSTED);
                self.cancelled(document);
            }
        }
    }

    /// Puts the frame on the screen, the controls over it, and the chrome over both.
    ///
    /// Idempotent and called after every pump, which is what keeps the window a function of the
    /// viewer's state rather than of the order events arrived in.
    pub(super) fn refresh(&mut self) {
        let began = std::time::Instant::now();
        let generation = self.ui.probe.begin_refresh();
        let Pictures {
            placements,
            held,
            glimpsed,
            observation,
        } = self.placements(generation);
        if !placements.is_empty() {
            // Tier 1's whole cost, in the one place it is paid: `doc/ui-boundary.md` prices the
            // tier at "one copy per frame" and this is that copy, timed rather than assumed
            // (ADR 0244).
            let bytes: usize = placements.iter().map(|picture| picture.bytes).sum();
            self.state.trace.say(
                Topic::Frames,
                format_args!(
                    "{bytes} bytes into {} texture(s) in {:?}",
                    placements.len(),
                    began.elapsed()
                ),
            );
            let scale = self.scale;
            while self.ui.pictures.len() < placements.len() {
                let picture = gtk4::Picture::new();
                picture.set_content_fit(gtk4::ContentFit::Fill);
                picture.set_can_shrink(false);
                self.ui.fixed.put(&picture, 0.0, 0.0);
                self.ui.pictures.push(picture);
            }
            for (slot, picture) in self.ui.pictures.iter().zip(&placements) {
                place(&self.ui.fixed, slot, picture, scale);
            }
            // The widgets a smaller arrangement no longer needs: hidden rather than destroyed,
            // because the next scroll will want them back.
            for spare in self.ui.pictures.iter().skip(placements.len()) {
                spare.set_paintable(gtk4::gdk::Paintable::NONE);
                spare.set_visible(false);
            }
            self.ui.fixed.set_visible(true);
            self.ui.probe.applied(observation);
            if let Some((page, fraction)) = glimpsed {
                self.state.trace.say(
                    Topic::Frames,
                    format_args!(
                        "page {} shown as it stands, {:.0} % drawn",
                        page.saturating_add(1),
                        fraction * 100.0
                    ),
                );
            }
            if !self.state.presented && held > 0 {
                self.state.presented = true;
                self.state.read_look = None;
                self.state.trace.say(
                    Topic::Launch,
                    format_args!(
                        "first frame on the screen at {:?}",
                        self.state.trace.since_start()
                    ),
                );
                // The command line's later documents, and not one moment sooner: nothing page one
                // does not need happens before page one (`CLAUDE.md` section 2).
                self.later_open_the_next();
            }
        }
        let fields = match self.state.viewer.query(Query::Fields) {
            Answer::Fields(fields) => fields,
            _ => Vec::new(),
        };
        self.place_fields(&fields);
        // §12.5.6.14's windows, which have no appearance stream and are therefore nowhere in the
        // pixels above: they are furniture, and this host places real GTK widgets for them.
        let popups = match self.state.viewer.query(Query::Popups) {
            Answer::Popups(windows) => windows,
            _ => Vec::new(),
        };
        self.place_popups(&popups);
        self.refresh_chrome();
        // Last, and **only once a frame is actually on the screen**: §14.7's tree on AT-SPI. The
        // guard is the rule rather than an optimisation, and it was put here by a measurement —
        // without it `--trace` printed `accessibility bridge up` at 1.656 s and
        // `first frame on the screen` at 1.671, because `refresh` runs once on the first
        // allocation before the document is even open. `Bridge::new` spawns a thread and connects
        // to the session bus, which is exactly what `CLAUDE.md`'s startup section forbids in front
        // of page one (ADR 0623, and ADR 0214 for the rule).
        if self.state.presented {
            self.attend();
        }
    }
}
