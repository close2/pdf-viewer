//! What a document's scripts asked of the viewer, carried out after the script returns.

use super::layout::damage;
use super::{DocumentId, MAX_FOCUS_HOPS, MAX_PAGE_TURNS, Turn, Viewer};
use crate::command::{Command, PageTarget, Zoom};
use crate::event::Event;
use crate::open::Open;

impl Viewer {
    /// Hands one document the runner its reader's level and answer call for, and runs what a
    /// runner arriving late has missed (ADR 1616).
    ///
    /// A document handed a runner that runs has Table 224's `/CO` walked over its values, so that
    /// a total shows what its scripts compute from what is already typed; one whose open sequence
    /// has run has the sequence run again first — a new runner holds a new realm, and the
    /// functions Table 32's name tree defines are what its field scripts call. What either says
    /// goes out after the command, with every other script's sentence.
    pub(super) fn supply_scripts(&mut self, id: DocumentId) {
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let runner = crate::scripting::runner_for(&self.scripting, &mut open.consent);
        open.view.run_scripts_with(runner);
        if !crate::scripting::runs(&self.scripting, &open.consent) {
            return;
        }
        if open.presented {
            let page = open.page_index;
            open.view.run_open_scripts(&open.document, page);
            open.view.activate_form_on_merge(&open.document);
        }
        open.view.recalculate_with_runner(&open.document);
        open.stale();
    }

    /// Every sentence a document's scripts said since the last command, as one report per
    /// document: what Tier 0 did not run, what a runner refused, threw or logged, what `warn` says
    /// ran, and what *ask* withheld (RFC 0008 section 6.8, ADR 1616).
    ///
    /// Asked after every command rather than at each trigger's site, because a script runs at a
    /// keystroke, a commit, a focus, a page turn, the open sequence and a replay of the log, and a
    /// site that forgot to say would be a script that ran in silence. The view state keeps each
    /// sentence once, so what is new is what follows the count already said.
    pub(super) fn say_what_scripts_said(&mut self, events: &mut Vec<Event>) {
        for (id, open) in &mut self.documents {
            let all = open.view.script_reports();
            let said = open.scripts_said.min(all.len());
            if all.len() > said {
                events.push(Event::Reported {
                    document: *id,
                    page: None,
                    notes: all[said..].to_vec(),
                });
            }
            open.scripts_said = all.len();
        }
    }

    /// A script's `this.pageNum = n`, carried out as the page turn a person asks for: Table 198's
    /// `/C` of the page left and `/O` of the page reached run as they would (ADR 1643).
    ///
    /// Only the document in front turns: a document behind another has no page on the screen to
    /// turn, so its request stays held in its view state and is carried out after the first command
    /// that finds it in front. Held to [`MAX_PAGE_TURNS`] in one command.
    pub(super) fn carry_out_page_requests(&mut self, events: &mut Vec<Event>) {
        for _ in 0..MAX_PAGE_TURNS {
            let Some(open) = self.focused_mut() else {
                return;
            };
            let Some(page) = open.view.take_page_request() else {
                return;
            };
            if page == open.page_index {
                return;
            }
            self.go_to(PageTarget::Index(page), Turn::Requested, events);
        }
    }

    /// A script's `setFocus`, carried out: the widget the script's `Field` stood for takes the
    /// keyboard — the field's first, for a `Field` of every widget — on its own page, with Table
    /// 197's `/Bl` and `/Fo` raised as a tab raises them (ADRs 1615, 1688).
    ///
    /// Held to [`MAX_FOCUS_HOPS`] in one command, because a `/Fo` script may ask for the focus
    /// again and a document whose focus scripts hand it round in a ring would hold the command
    /// for ever.
    pub(super) fn carry_out_focus_requests(&mut self, events: &mut Vec<Event>) {
        for _ in 0..MAX_FOCUS_HOPS {
            let Some(id) = self.focused else { return };
            let Some(open) = self.documents.get_mut(&id) else {
                return;
            };
            let Some(request) = open.view.take_focus_request() else {
                return;
            };
            // The view state counted the widget in this same table when it took the request.
            let widget = pdf_model::view::widgets_by_field_name(&open.document)
                .get(&request.field)
                .and_then(|widgets| widgets.get(request.widget).copied());
            let Some(widget) = widget else {
                events.push(Event::Reported {
                    document: id,
                    page: None,
                    notes: vec![format!(
                        "a script asked for the keyboard focus on widget {} of {}, which this \
                         document does not have, so the focus stays where it is",
                        request.widget, request.field
                    )],
                });
                return;
            };
            // Table 166's `/P` is an indirect reference to the page, so it is read unresolved:
            // `get_key` would hand back the page dictionary, which names no page index.
            let page = open
                .document
                .get(widget)
                .as_dict()
                .and_then(|dictionary| {
                    dictionary
                        .get("P")
                        .and_then(pdf_syntax::Object::as_reference)
                })
                .and_then(|page| open.page_indices().get(&page).copied());
            if let Some(page) = page
                && page != open.page_index
            {
                self.go_to(PageTarget::Index(page), Turn::Requested, events);
            }
            self.focus_on(id, Some(widget), events);
        }
    }

    /// A script's `Bookmark.execute()`, carried out as a person's click on the item in the outline
    /// panel: §12.3.3's activation, its destination or its action, the item's scripts among them
    /// (ADRs 1904, 1934).
    ///
    /// Only the document in front activates, as only it turns a page. An item's own script may
    /// execute an item again, and what it asks is held for the next command rather than performed
    /// in this one, so a document whose items execute each other in a ring cannot hold a command.
    pub(super) fn carry_out_bookmark_requests(&mut self, events: &mut Vec<Event>) {
        let Some(open) = self.focused_mut() else {
            return;
        };
        for item in open.view.take_bookmark_requests() {
            self.activate(item, events);
        }
    }

    /// A script's changes to the window's view — `this.zoom`, `this.zoomType`, `this.layout`,
    /// `this.scroll` — carried out in the order asked, each as the person's own zoom, layout or
    /// scroll is (ADR 1736).
    ///
    /// Only the document in front changes its view, as only it turns a page: a document behind
    /// another keeps its requests in its view state until a command finds it in front.
    pub(super) fn carry_out_view_requests(&mut self, events: &mut Vec<Event>) {
        use pdf_model::view::{ViewChange, ZoomType};

        let Some(open) = self.focused_mut() else {
            return;
        };
        for change in open.view.take_view_requests() {
            match change {
                ViewChange::Zoom(percent) => {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "the realm holds a zoom inside 8.33 to 6400 per cent, which an \
                                  `f32` holds to far finer than a pixel"
                    )]
                    let scale = (percent / 100.0) as f32;
                    self.set_zoom(Zoom::Scale(scale), None, events);
                }
                ViewChange::ZoomType(ZoomType::FitPage) => {
                    self.set_zoom(Zoom::FitPage, None, events);
                }
                ViewChange::ZoomType(ZoomType::FitWidth) => {
                    self.set_zoom(Zoom::FitWidth, None, events);
                }
                ViewChange::ZoomType(ZoomType::FitHeight) => {
                    self.set_zoom(Zoom::FitHeight, None, events);
                }
                // `NoVary` holds the magnification a fitting mode resolved to, so that a later
                // resize leaves it where it is.
                ViewChange::ZoomType(ZoomType::NoVary) => {
                    let (viewport, scale) = (self.viewport, self.scale);
                    let fixed = self
                        .focused_mut()
                        .and_then(|open| open.magnification(viewport, scale))
                        .map(|device| device / scale);
                    if let Some(fixed) = fixed {
                        self.set_zoom(Zoom::Scale(fixed), None, events);
                    }
                }
                // §12.3.2.2's `/FitBH`, "the entire width of [the page's] bounding box", which
                // `settle` applies as it applies a destination's.
                ViewChange::ZoomType(ZoomType::FitVisibleWidth) => {
                    let viewport = self.viewport;
                    if let Some(open) = self.focused_mut() {
                        open.pending_views
                            .push(pdf_model::destination::View::FitBH { top: None });
                    }
                    events.push(damage(viewport));
                }
                // The realm refuses both before they cross (`ZoomType::is_drawn`).
                ViewChange::ZoomType(ZoomType::Preferred | ZoomType::ReflowWidth) => {}
                ViewChange::Layout(layout) => self.act(Command::Layout(layout), events),
                ViewChange::Scroll { page, x, y } => self.scroll_to_middle(page, x, y, events),
                // `this.gotoNamedDest`: the destination's page and Table 149's view, shown as a
                // link's are — the view left for `settle`, the page turned where it is not the one
                // showing (ADR 1751).
                ViewChange::Destination { page, view } => {
                    let (Ok(page), Some(open)) = (usize::try_from(page), self.focused_mut()) else {
                        continue;
                    };
                    open.pending_views = vec![view];
                    if open.page_index != page {
                        self.go_to(PageTarget::Index(page), Turn::Requested, events);
                    }
                }
            }
        }
    }

    /// `this.scroll(nX, nY)`: the point of default user space `(x, y)` on page `page` brought to
    /// the middle of the viewport — the page turned to first where it is not the one showing —
    /// and the scroll left for `settle` to clamp, as a wheel's is (ADR 1736).
    fn scroll_to_middle(&mut self, page: u32, x: f64, y: f64, events: &mut Vec<Event>) {
        let Ok(page) = usize::try_from(page) else {
            return;
        };
        if self
            .focused_mut()
            .is_some_and(|open| open.page_index != page)
        {
            self.go_to(PageTarget::Index(page), Turn::Requested, events);
        }
        let (viewport, scale) = (self.viewport, self.scale);
        let Some(open) = self.focused_mut() else {
            return;
        };
        if open.page_index != page {
            return;
        }
        let (Some(object), Some(size), Some(magnification)) = (
            open.page(page),
            open.page_size(page),
            open.magnification(viewport, scale),
        ) else {
            return;
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a point of a page's default user space, which is held in `f32` everywhere \
                      a page is placed on the screen"
        )]
        let (px, py) = pdf_model::content::page_space_at(&object, x as f32, y as f32);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a viewport's extent in device pixels, far inside an `f32`'s whole numbers"
        )]
        let middle = (viewport.0 as f32 / 2.0, viewport.1 as f32 / 2.0);
        // The display list's y counts up from the bottom of the page and the scroll down from its
        // top, as `Open::scroll_to` measures it.
        open.scroll = (
            px * magnification - middle.0,
            (size.height - py) * magnification - middle.1,
        );
        events.push(damage(viewport));
    }

    /// Every run a document's runner held on a person's answer and has since finished, applied as
    /// its trigger's outcome arriving late; a page whose ink it changed is drawn again (ADRs 1627,
    /// 1628).
    ///
    /// After every command rather than only after an answer, because a runner finishes what it
    /// held behind the answered one in order, and with nothing finished this does nothing.
    pub(super) fn apply_resumed_scripts(&mut self) {
        for open in self.documents.values_mut() {
            if open.view.apply_resumed(&open.document) {
                open.stale();
            }
        }
    }

    /// Every timer a document's script set counted down by a tick's milliseconds, and each one now
    /// due run at the page its document shows; a page is drawn again only where a run can have
    /// changed what it draws, as at the other sites (ADRs 1702, 1771).
    ///
    /// Every document's, not only the focused one's: a timer ends with its document and nothing
    /// else, and the page turn or focus a timer's script asks for waits in its view state until
    /// the document is in front, as any script's does.
    pub(super) fn run_timers(&mut self, millis: u32) {
        for open in self.documents.values_mut() {
            let page = open.page_index;
            if open.view.run_timers(&open.document, millis, page).changed {
                open.stale();
            }
        }
    }

    /// Every sound and every console request a document's script asked for, put after the
    /// command whose scripts asked (ADRs 1702, 1771).
    pub(super) fn ask_for_sounds(&mut self, events: &mut Vec<Event>) {
        for (id, open) in &mut self.documents {
            for sound in open.view.take_beeps() {
                events.push(Event::Beep {
                    document: *id,
                    sound,
                });
            }
            for request in open.view.take_console_requests() {
                events.push(Event::Console {
                    document: *id,
                    request,
                });
            }
        }
    }

    /// Every question a document's script is waiting on, put after the command that raised it
    /// (ADR 1628).
    ///
    /// A withdrawal is asked first, so that the card of a question whose wait ran out is dropped
    /// before the card of one a queued script asked since is put (ADR 1643).
    pub(super) fn put_script_questions(&mut self, events: &mut Vec<Event>) {
        for (id, open) in &mut self.documents {
            for over in open.view.take_form_overrides() {
                if open.consent.form_alerts.len() < crate::scripting::MAX_FORM_ALERTS {
                    open.consent
                        .form_alerts
                        .push_back(crate::scripting::override_notice(&over));
                }
            }
            if let Some(asks) = open.consent.asks.clone() {
                if asks.withdrawn() {
                    open.consent.runner_question_open = false;
                    events.push(Event::ScriptQuestionWithdrawn { document: *id });
                }
                // A notice waits on an answer the runner could mistake for its own, so a question
                // the runner's script puts is left in the runner until the notice is answered.
                if !open.consent.form_alert_open
                    && let Some(question) = asks.take_question()
                {
                    open.consent.runner_question_open = true;
                    events.push(Event::ScriptAsking {
                        document: *id,
                        question,
                    });
                }
            }
            if !open.consent.runner_question_open
                && !open.consent.form_alert_open
                && let Some(question) = open.consent.form_alerts.pop_front()
            {
                open.consent.form_alert_open = true;
                events.push(Event::ScriptAsking {
                    document: *id,
                    question,
                });
            }
        }
    }

    /// RFC 0008 section 6.3's one question per document, put the first time a document's
    /// [`crate::scripting::Withheld`] runner has been handed a script (ADR 1616).
    pub(super) fn ask_about_scripts(&mut self, events: &mut Vec<Event>) {
        for (id, open) in &self.documents {
            if let Some((script, first_line)) = open
                .consent
                .withheld
                .as_ref()
                .and_then(|withheld| withheld.question())
            {
                events.push(Event::AskingToRunScripts {
                    document: *id,
                    script,
                    first_line,
                });
            }
        }
    }
}

/// RFC 0008 section 6.5 step 1, run once for a document whose first frame a host has presented
/// (ADR 1602): what a script it ran changed is ink, so the page is interpreted again, and what the
/// sequence says goes out with every other script's sentence after the command.
pub(super) fn open_sequence(open: &mut Open) {
    if std::mem::replace(&mut open.presented, true) {
        return;
    }
    let page = open.page_index;
    // XFA 3.3 page 379: the completion of the merge activates a hybrid form's calculations, and it
    // is complete when the form is shown. A reader that asks first reaches its question through
    // the withheld runner here, as it does at the first of a field's scripts.
    let ran = open.view.run_open_scripts(&open.document, page) > 0;
    let calculated = open.view.activate_form_on_merge(&open.document);
    if ran || calculated {
        open.stale();
    }
}
