//! Turning pages, presenting and the clock: §12.4.4's transitions and the events a page raises on
//! arrival and departure.

use pdf_model::action::Trigger;
use pdf_syntax::ObjectId;

use super::answers::label;
use super::geometry::annotations_on;
use super::layout::resolve;
use super::{DocumentId, Turn, Viewer};
#[cfg(doc)]
use crate::Command;
use crate::command::PageTarget;
use crate::event::Event;
use crate::interact;

impl Viewer {
    /// Moves the log's cursor, which is what undo and redo are.
    pub(super) fn move_cursor(&mut self, by: isize, events: &mut Vec<Event>) {
        let Some(id) = self.focused else { return };
        let Some(open) = self.focused_mut() else {
            return;
        };
        let Some(cursor) = open.cursor.checked_add_signed(by) else {
            return;
        };
        if cursor > open.log.len() || cursor == open.cursor {
            return;
        }
        let before = open.dirty();
        let crossed = open.cursor.min(cursor)..open.cursor.max(cursor);
        let attachments = open
            .log
            .get(crossed)
            .is_some_and(|entries| entries.iter().any(crate::open::Done::moves_attachments));
        open.cursor = cursor;
        open.replay();
        if open.dirty() != before {
            events.push(Event::Dirty {
                document: id,
                dirty: open.dirty(),
            });
        }
        if attachments {
            events.push(Event::AttachmentsChanged { document: id });
        }
    }

    /// [`Command::Report`]: what the focused document says about itself, in words.
    ///
    /// **The one place `notes::about` is called, and it is not the open path.** Its clauses
    /// are claims about the *file* — §12.8's signatures above all, whose answer reads and digests
    /// the signed byte ranges — and `CLAUDE.md` principle 2 defers what page one does not need
    /// until something asks. This is the asking; [`crate::open::Open::about`] is the `OnceLock`
    /// that makes a second asking free. ADR 1044.
    ///
    /// Nothing at all where nothing is open, and nothing where the document has nothing to say:
    /// an empty report is not a sentence, and a host that printed one would be telling a reader
    /// something about a file that said nothing.
    ///
    /// **And after the sentences, the servers the signatures name**, as
    /// [`Event::SignaturePoliciesPublished`]: the report is where a signature's policy is read, so
    /// it is where a host first learns that a copy of it is published, and it comes after the
    /// sentence saying which policy so that the question a host may put reads in that order (ADR
    /// 1738).
    pub(super) fn report_the_document(&self, events: &mut Vec<Event>) {
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return;
        };
        let about = open.about(&self.trust);
        if !about.notes.is_empty() {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes: about.notes.clone(),
            });
        }
        if !about.published.is_empty() {
            events.push(Event::SignaturePoliciesPublished {
                document: id,
                policies: about.published.clone(),
            });
        }
    }

    /// §12.4.4: enters or leaves presentation mode, in every open document.
    ///
    /// Nothing at all when the mode is already what the host says it is, which is what lets a host
    /// send it on every frame if that is what its own state is easiest to mirror from.
    pub(super) fn present(&mut self, mode: crate::PresentationMode) {
        if self.presenting == mode {
            return;
        }
        self.presenting = mode;
        for open in self.documents.values_mut() {
            match mode {
                crate::PresentationMode::On => crate::presentation::enter(open),
                // §12.4.4.2 NOTE 2's restore changes what is *drawn*, so it invalidates the
                // display list rather than only the pixels made from it — the same thing
                // `Command::SetGroup` does, and for the same clause's reason.
                crate::PresentationMode::Off => {
                    if crate::presentation::leave(open) {
                        open.stale();
                    }
                }
            }
        }
    }

    /// Shows another page, or — during a presentation — another *state* of the page being shown.
    ///
    /// §12.4.4.2 puts a decision in front of every page turn a person asks for: "[i]f the user
    /// requests to navigate forward (such as an arrow key press) **and there is a current
    /// navigation node**", what happens is the node's own actions and not a page turn. So a
    /// presentation steps through a page's states first and turns the page when they run out,
    /// which is what NOTE 1's bullet points are for.
    ///
    /// `turn` is why the page is being turned, and it decides two things that are not the same
    /// question: §12.4.4.1's own clock is not a person requesting anything, so it never steps
    /// within a page — it advances the page, which is what `/Dur` says it does.
    pub(super) fn go_to(&mut self, target: PageTarget, turn: Turn, events: &mut Vec<Event>) {
        let requested = match target {
            PageTarget::Next => Some(true),
            PageTarget::Previous => Some(false),
            _ => None,
        };
        if turn == Turn::Requested
            && self.presenting == crate::PresentationMode::On
            && let Some(forward) = requested
        {
            let stepped = self.focused_mut().and_then(|open| {
                crate::presentation::step(open, forward)
                    .map(|step| (interact::navigate(open, &step.actions), step.onward))
            });
            if let (Some((outcome, onward)), Some(id)) = (stepped, self.focused) {
                // §12.4.4.2: "[i]f NA specifies an action that navigates to another page, the
                // following actions for navigating to another page take place, and Next should
                // not be present." A node whose actions have already moved the page has had its
                // turn, so errata issue #304's page turn is not owed a second one.
                let jumped = outcome.target.is_some();
                self.apply(id, outcome, events);
                if jumped || !onward {
                    return;
                }
            }
        }

        let Some(open) = self.focused_mut() else {
            return;
        };
        let Some(index) = resolve(open, target) else {
            return;
        };
        if index == open.page_index {
            return;
        }
        let left = open.page_index;
        open.page_index = index;
        // A new page starts at its top: carrying a scroll position across a page turn
        // would land a reader in the middle of a page they have not seen.
        open.scroll = (0.0, 0.0);
        // §12.4.4.1's clock is a property of the page, so a turn restarts it — which is
        // also NOTE 1's "[t]he user can advance the page manually before the specified
        // time has expired" doing the right thing without a second rule.
        open.shown_for = 0.0;
        self.announce_page(events);
        let Some(id) = self.focused else {
            return;
        };
        self.page_events(id, Some(left), events);
        // §12.4.4.2 makes random access forward — "[i]f the navigation request was forward, or if
        // the navigation request was for random access (such as by clicking on a link)" — so only
        // a request that names a page *behind* this one is backward.
        let forward = !matches!(target, PageTarget::Previous)
            && !matches!(target, PageTarget::Relative(delta) if delta < 0);
        self.arrive(id, forward, turn, events);
    }

    /// What §12.4.4 says happens on arriving at a page, where a presentation is running.
    ///
    /// The last paragraph of §12.4.4.2, then its step (c): the page's own navigation node becomes
    /// current and one request is performed against it, and then "[a]ny page transitions specified
    /// by the Trans entry of the page dictionary shall be performed".
    ///
    /// **The two are gated differently, and the difference is what a host has said.** The node
    /// walk needs [`crate::PresentationMode::On`], because walking the nodes changes §8.11's
    /// groups and NOTE 2's obligation to put them back is discharged by entering the mode; a host
    /// that only drives the clock has saved nothing to restore. The transition needs either that
    /// or an automatic advance, because §12.4.4.1's `/Dur` running out *is* a presentation running
    /// — which is ADR 0135's reading and the one thing a tick alone does say.
    pub(super) fn arrive(
        &mut self,
        id: DocumentId,
        forward: bool,
        turn: Turn,
        events: &mut Vec<Event>,
    ) {
        // A nested arrival is the outer one's business, and that is not only a bound on the
        // cascade: the clause's step (c) is about "the new page", so an `/NA` that jumped again
        // has one transition to perform and not one per page it passed through.
        if self.stepping {
            return;
        }
        self.stepping = true;
        if self.presenting == crate::PresentationMode::On {
            let outcome = self
                .focused_mut()
                .map(|open| {
                    let actions = crate::presentation::arrived(open, forward);
                    interact::navigate(open, &actions)
                })
                .unwrap_or_default();
            self.apply(id, outcome, events);
        }
        if turn == Turn::Automatic || self.presenting == crate::PresentationMode::On {
            self.play_transition(events);
        }
        self.stepping = false;
    }

    /// §12.4.4.1's `/Trans`: names the transition of the page a presentation has arrived at.
    ///
    /// > The transition style that shall be used when moving to this page from another during a
    /// > presentation
    ///
    /// *This* page, so it is the page arrived at whose `/Trans` plays, and the page is fetched
    /// from the tree rather than read from `Open::shown_page`, because the arrangement it reads
    /// is built in `settle` — after this. Reading it here would
    /// name the transition of the page just *left*, which is the same off-by-one §12.4.4.1's own
    /// wording rules out. One page-tree walk per advance.
    fn play_transition(&mut self, events: &mut Vec<Event>) {
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return;
        };
        let pages = pdf_model::Pages::new(&open.document);
        let Some(page) = pages.get(open.page_index) else {
            return;
        };
        if let Some(transition) = pdf_model::navigation::transition(&open.document, &page.dict) {
            // A transition no frame is shaped for is *named* rather than quietly drawn as a cut,
            // which is trap 5 in the one place a viewer is most tempted to be silent: the page
            // that arrives looks right, and only the file knows it asked for an effect. And one
            // drawn at a quantity this program chose says the quantity is ours (ADR 1299).
            // `crate::transition` decides which those are, because it is what draws them — and
            // it is asked the whole transition, because a style is not the whole of what
            // decides a frame.
            if let Some(note) = crate::transition::note(&transition) {
                let index = self.focused().map(|open| open.page_index);
                events.push(Event::Reported {
                    document: id,
                    page: index,
                    notes: vec![note],
                });
            }
            events.push(Event::Transition {
                document: id,
                transition,
            });
        }
    }

    /// Says which page is showing, where there is one.
    /// §12.4.4.1's `/Dur`: advances the page when it has been shown for as long as it asked.
    ///
    /// > the maximum length of time, in seconds, that the page shall be displayed before the
    /// > presentation automatically advances to the next page.
    ///
    /// A *maximum*, which is why the comparison is `>=` and why any page turn resets the clock.
    /// The last page does not advance and does not loop: §12.4.4 says nothing about what follows
    /// the end, and looping is a decision a host can make with a `GoTo` and this crate cannot
    /// unmake.
    pub(super) fn tick(&mut self, millis: u32, events: &mut Vec<Event>) {
        self.run_timers(millis);
        // Everything below is §12.4.4's advance timing, and the clause makes it a presentation's:
        // a page's `/Dur` and `/Trans` "specify how to display that page in presentation mode".
        // A window reading a document ticks for a script's timers too (ADR 1702), and those ticks
        // must not turn a page a person is reading.
        if self.presenting != crate::PresentationMode::On {
            return;
        }
        // Milliseconds in, seconds out, because the clause counts in seconds and a host counts
        // in whatever its event loop gives it. `f32` loses exactness above sixteen million
        // milliseconds — four and a half hours on one page — and what is being measured is a
        // duration a person perceives, which `pdf_model::navigation` narrows for the same reason.
        let seconds =
            f32::from(u16::try_from(millis.min(u32::from(u16::MAX))).unwrap_or(u16::MAX)) / 1000.0;
        let Some(open) = self.focused_mut() else {
            return;
        };
        open.shown_for += seconds;

        // Table 165's `/Dur` first, because it is the inner of the two clocks: "[t]he maximum
        // number of seconds before the interactive PDF processor shall automatically advance
        // forward to the next navigation node".
        //
        // **To a node, and never to a page**, which is where this entry and an arrow key part
        // company. Errata issue #304 makes a *user's* forward request with no `/Next` navigate to
        // the next page (§12.4.4.2 item (b)); Table 165 names the next navigation node as its own
        // destination, so a node with no successor has nothing to advance to and this clock stops
        // there. The page's own `/Dur` below is the one that turns a page.
        if self.presenting == crate::PresentationMode::On {
            let due = self
                .focused_mut()
                .is_some_and(|open| crate::presentation::node_due(open, seconds));
            if due {
                let stepped = self.focused_mut().and_then(|open| {
                    crate::presentation::step(open, true)
                        .map(|step| interact::navigate(open, &step.actions))
                });
                if let (Some(outcome), Some(id)) = (stepped, self.focused) {
                    self.apply(id, outcome, events);
                }
            }
        }

        let Some(open) = self.focused_mut() else {
            return;
        };
        let Some(page) = open.shown_page() else {
            return;
        };
        let Some(duration) = pdf_model::navigation::display_duration(&open.document, &page.dict)
        else {
            return;
        };
        if open.shown_for < duration || open.page_index.saturating_add(1) >= open.page_count {
            return;
        }
        // §12.4.4.1's own advance, which is a page turn and never a step within a page: `/Dur` is
        // "the maximum length of time … that the page shall be displayed before the presentation
        // automatically advances to the **next page**".
        self.go_to(PageTarget::Next, Turn::Automatic, events);
    }

    pub(super) fn announce_page(&mut self, events: &mut Vec<Event>) {
        self.announce_page_reading(true, events);
    }

    /// [`Self::announce_page`], with `read` false for the opening announcement: a section only
    /// from an outline already read (ADR 1543).
    pub(super) fn announce_page_reading(&mut self, read: bool, events: &mut Vec<Event>) {
        let (Some(id), Some(open)) = (self.focused, self.focused()) else {
            return;
        };
        let index = open.page_index;
        // §7.7.3's tree placed once for the life of the document rather than once per turn, and
        // no `Pages` built here at all: both were per-turn costs that scale with the document —
        // 1023 pages of ISO 32000-2 cost about six milliseconds of walk and a further third of
        // one to construct the tree, on the path an arrow key takes. See `Open::page_indices`.
        // **Read only where it is already read, when the document is opening**: the section is
        // not worth the outline and the whole tree in front of page one, and
        // `Viewer::prepared` announces the page again once they are read (ADR 1543).
        let section = open.section_at(index, read);
        events.push(Event::PageChanged {
            document: id,
            index,
            label: label(open, index),
            of: open.page_count,
            section,
        });
    }

    /// §12.6.3's four page-scoped events and Table 198's two, in the order the clause states.
    ///
    /// A page turn raises six things, and §12.6.3 states the order of four of them:
    ///
    /// > The action shall be executed after the O action in the page's additional - actions
    /// > dictionary (see "Table 198 - Entries in a page object's additional - actions
    /// > dictionary") and the OpenAction entry in the document Catalog (see "Table 29 - Entries
    /// > in the catalog dictionary"), if such actions are present.
    ///
    /// and, of `/PC`, that it shall be executed before the page's own `/C`. So leaving a page is
    /// its annotations' `/PC` then the page's `/C`, and
    /// arriving at one is the page's `/O` then its annotations' `/PO` — which is the sequence
    /// below.
    ///
    /// **`/PV` and `/PI` coincide with `/PO` and `/PC` here, and that is derived rather than
    /// conceded.** §12.6.3 says why the pair exists: "[t]he PV and PI entries allow a distinction
    /// between pages that are open and pages that are visible. At any one time, while more than
    /// one page may be visible, depending on the page layout." This viewer shows one page at a
    /// time, so the two sets have one member each and they are the same member. A host that drew
    /// a continuous tower of pages would separate them, and the place to do it is here.
    ///
    /// NOTE 1 is honoured by not consulting Table 167 at all: "[f]or these trigger events, the
    /// values of the flags specified by the annotation's F entry … have no bearing on whether a
    /// given trigger event occurs" — so a hidden annotation's `/PO` is performed.
    ///
    /// **Nothing cascades.** An action performed here may turn the page again; raising the same
    /// six events for *that* turn would let a document with `/PO` pointing at the next page walk
    /// the whole file, and §12.6.2 states no bound. `raising` is that bound, and it is a flag
    /// rather than a depth because one level is the only one the clause describes.
    pub(super) fn page_events(
        &mut self,
        id: DocumentId,
        left: Option<usize>,
        events: &mut Vec<Event>,
    ) {
        if self.raising {
            return;
        }
        self.raising = true;
        let mut raised: Vec<(ObjectId, Trigger)> = Vec::new();
        let mut closed = None;
        let mut opened = None;
        // A page turned takes the focus with it: the widget that held it is not on the screen
        // any more, so §12.6.3's `/Bl` is due before the page's own events. Raised here rather
        // than in `pointer` because this is where a page stops being shown, whatever caused it.
        if let Some(open) = self.focused_mut()
            && left.is_some_and(|index| index != open.page_index)
            && let Some(lost) = open.focus.take()
        {
            raised.push((lost, Trigger::Blur));
        }
        if let Some(open) = self.focused() {
            let pages = pdf_model::Pages::new(&open.document);
            if let Some(index) = left.filter(|index| *index != open.page_index) {
                closed = pages.get(index).map(|page| page.dict.clone());
                for annotation in annotations_on(open, &pages, index) {
                    raised.push((annotation, Trigger::PageClose));
                    raised.push((annotation, Trigger::PageInvisible));
                }
            }
            opened = pages.get(open.page_index).map(|page| page.dict.clone());
        }
        self.raise(id, raised, None, events);
        let runner = !matches!(self.scripting, crate::Scripting::Off);
        let turned =
            left.filter(|index| Some(*index) != self.focused().map(|open| open.page_index));
        for (page, event) in [
            (closed, pdf_model::action::PageTrigger::Close),
            (opened.clone(), pdf_model::action::PageTrigger::Open),
        ] {
            let Some(page) = page else { continue };
            let Some(open) = self.focused_mut() else {
                break;
            };
            let outcome = interact::page_trigger(open, &page, event, runner);
            self.apply(id, outcome, events);
            // Table 198's `/C` and Table 197's `/PC` of the page left, scripts and all, before
            // anything of the page reached (ADR 1752).
            if event == pdf_model::action::PageTrigger::Close
                && let Some(left) = turned
            {
                self.page_scripts(left, event);
            }
        }
        let mut raised: Vec<(ObjectId, Trigger)> = Vec::new();
        if let Some(open) = self.focused() {
            let pages = pdf_model::Pages::new(&open.document);
            for annotation in annotations_on(open, &pages, open.page_index) {
                raised.push((annotation, Trigger::PageOpen));
                raised.push((annotation, Trigger::PageVisible));
            }
        }
        self.raise(id, raised, None, events);
        // The open's own page is the open sequence's, run once page one is presented; a turn's is
        // run here, `/O` and then each annotation's `/PO` and `/PV`, in Table 197's order.
        if turned.is_some()
            && let Some(page) = self.focused().map(|open| open.page_index)
        {
            self.page_scripts(page, pdf_model::action::PageTrigger::Open);
        }
        self.raising = false;
    }

    /// Runs Table 198's `/O` or `/C` scripts of page `page` of the document in front, and its
    /// annotations' Table 197 `/PO` and `/PV` or `/PC` and `/PI`, under the runner the reader's
    /// level supplied; a page whose ink a script changed is drawn again, and only such a page
    /// (ADRs 1750, 1752, 1771).
    fn page_scripts(&mut self, page: usize, trigger: pdf_model::action::PageTrigger) {
        let Some(open) = self.focused_mut() else {
            return;
        };
        if open
            .view
            .run_page_scripts(&open.document, page, trigger)
            .changed
        {
            open.stale();
        }
    }
}
