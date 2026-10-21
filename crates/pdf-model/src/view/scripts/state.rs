//! Script requests, reports and saved-state comparisons exposed to the host.

use super::{
    ConsoleRequest, FocusRequest, Keys, MAX_REPORTS, ObjectId, Overrides, Property, Saved,
    ViewChange, ViewState, WindowView,
};

impl ViewState {
    /// What Tier 0's dispatch did not run, and what every runner's script said, each sentence
    /// once, in the order it was met.
    ///
    /// A field's script that is not one call of the library, a call whose arguments the library
    /// refuses, a `/CO` entry whose `/C` is either, a runner's refusals, throws and stops — each
    /// named with its field, never raised as a dialog and never repeated (RFC 0008 section 6.8).
    #[must_use]
    pub fn script_reports(&self) -> &[String] {
        &self.script_reports
    }

    /// The widget a script's `setFocus` last asked the keyboard focus for, taken: `None` once a
    /// host has taken it, and where no script asked.
    ///
    /// The focus is the host's — which widget a key reaches, and the page turned or the view
    /// scrolled to show it, as Adobe's "Field methods" page describes `setFocus` — so a view state
    /// holds the request and a host carries it out after any call that ran scripts, raising Table
    /// 197's `/Bl` and `/Fo` as a press would (ADR 1615). The latest request stands: a script that
    /// asks twice has asked for the second. The widget is the one the script's `Field` stood for
    /// (ADR 1688).
    pub fn take_focus_request(&mut self) -> Option<FocusRequest> {
        self.scripting.focus.take()
    }

    /// The zero-based page a script's `this.pageNum = n` last turned to, taken: `None` once a host
    /// has taken it, and where no script turned one.
    ///
    /// Which page is shown is the host's, as the focus is, so a view state holds the request and a
    /// host carries it out after any call that ran scripts, as the page turn a person asks for —
    /// Table 198's `/C` of the page left and `/O` of the page reached run as they would for a
    /// person's turn. The page is one of the document's, checked when the script's run was applied;
    /// the latest request stands (ADR 1640).
    pub fn take_page_request(&mut self) -> Option<usize> {
        self.scripting.page.take()
    }

    /// Every change a script asked of the window's view since a host last took them, in the order
    /// asked: a zoom, a fitting mode, a page layout, a point scrolled to the middle (ADR 1736).
    ///
    /// The view is the host's, as the focus and the page are, so a view state holds the requests
    /// and a host carries them out after any call that ran scripts, as it carries a person's zoom
    /// or layout. A scroll names a page of the document, checked when the script's run was
    /// applied.
    pub fn take_view_requests(&mut self) -> Vec<ViewChange> {
        std::mem::take(&mut self.scripting.views)
    }

    /// Says what the window's view of the document now is, so that a script reads it as
    /// `this.zoom`, `this.zoomType` and `this.layout` (ADR 1736).
    ///
    /// A host calls it with the document and after each change of its view; the next event a
    /// runner is handed carries it. Until a host calls it, a script reads the layout the document
    /// asks to open with and no magnification.
    pub fn set_window_view(&mut self, view: WindowView) {
        self.scripting.window = Some(view);
    }

    /// Says what the host's keyboard holds, so that the next event a runner is handed carries it as
    /// `event.shift`, `event.modifier` and `event.keyDown` (ADR 1762).
    ///
    /// A host calls it with each pointer or keyboard event it raises a script's event for — a
    /// widget's `/D` with shift held, a list box's selection made with the arrows — and again
    /// when the keys are let go; until a host calls it every key reads as up.
    pub fn set_keys(&mut self, keys: Keys) {
        self.scripting.keys = keys;
    }

    /// The requests scripts made of the host's console since a host last took them, in the order
    /// made: `console.show`, `hide` and `clear` (ADR 1762). What a console shows is what scripts
    /// logged and every sentence [`Self::script_reports`] holds; a host without a console drops
    /// them.
    pub fn take_console_requests(&mut self) -> Vec<ConsoleRequest> {
        std::mem::take(&mut self.scripting.console)
    }

    /// Records each of a runner's sentences against the field it ran for.
    pub(in crate::view) fn report_each(&mut self, name: &str, sentences: Vec<String>) {
        for sentence in sentences {
            self.report(format!("{name}: {sentence}"));
        }
    }

    /// The properties scripts set on this widget's field that its appearance draws (ADR 1617).
    pub(in crate::view) fn scripted(&self, annotation: ObjectId) -> &[Property] {
        self.scripting
            .drawn
            .get(&annotation)
            .map_or(&[], Vec::as_slice)
    }

    /// Whether this widget's field is being typed into and has not been committed.
    pub(in crate::view) fn is_editing(&self, annotation: ObjectId) -> bool {
        self.uncommitted
            .values()
            .any(|widgets| widgets.iter().any(|(widget, _)| *widget == annotation))
    }

    /// Whether a script has set this field read-only (`Some(true)`), writable (`Some(false)`), or
    /// neither.
    pub(in crate::view) fn script_read_only(&self, name: &str) -> Option<bool> {
        self.scripting
            .overrides
            .get(name)
            .and_then(Overrides::read_only)
    }

    /// Records that a save has written everything this view state holds: what `this.dirty` is
    /// measured against from now on (ADR 1626).
    ///
    /// **What a host calls once [`ViewState::save`]'s update is in a file.** The host keeps its
    /// own mark of unsaved work, which is the reader's and which no script moves; this is the one a
    /// script reads.
    pub fn mark_saved(&mut self) {
        self.scripting.saved = Some(Box::new(self.written()));
    }

    /// The parts of this view state a save writes, copied.
    fn written(&self) -> Saved {
        Saved {
            edited: self.edited.clone(),
            imported: self.imported.clone(),
            reset: self.reset.clone(),
            added: self.added.clone(),
            retyped: self.retyped.clone(),
            filed: self.filed.clone(),
            unfiled: self.unfiled.clone(),
            drawn: self.scripting.drawn.clone(),
            annotations_hidden: self.scripting.annotations.hidden.clone(),
            popups_opened: self.written_windows(),
        }
    }

    /// Whether this view state holds work a save would write that no save has written:
    /// `this.dirty`.
    ///
    /// Measured against what [`Self::mark_saved`] last recorded, or against a view state as a
    /// document opens — holding no edit — where no save has been marked.
    pub(in crate::view) fn unsaved(&self) -> bool {
        match self.scripting.saved.as_deref() {
            Some(saved) => {
                saved.edited != self.edited
                    || saved.imported != self.imported
                    || saved.reset != self.reset
                    || saved.added != self.added
                    || saved.retyped != self.retyped
                    || saved.filed != self.filed
                    || saved.unfiled != self.unfiled
                    || saved.drawn != self.scripting.drawn
                    || saved.annotations_hidden != self.scripting.annotations.hidden
                    || saved.popups_opened != self.written_windows()
            }
            None => {
                !(self.edited.is_empty()
                    && self.imported.is_empty()
                    && self.reset.is_empty()
                    && self.added.is_empty()
                    && self.retyped.is_empty()
                    && self.filed.is_empty()
                    && self.unfiled.is_empty()
                    && self.scripting.drawn.is_empty()
                    && self.scripting.annotations.hidden.is_empty()
                    && self.written_windows().is_empty())
            }
        }
    }

    /// Records one sentence of [`Self::script_reports`], once.
    pub(in crate::view) fn report(&mut self, sentence: String) {
        if self.script_reports.len() < MAX_REPORTS && !self.script_reports.contains(&sentence) {
            self.script_reports.push(sentence);
        }
    }
}
