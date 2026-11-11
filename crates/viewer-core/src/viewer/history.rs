//! The window's history of views, walked: a jump that leaves a view behind, and a step along the
//! history that restores one (ADR 2805; the history itself is `crate::history`).

use super::layout::resolve;
use super::{Turn, Viewer};
use crate::command::{PageTarget, Retrace};
use crate::event::Event;

impl Viewer {
    /// A person's [`crate::Command::GoTo`], and a script's `this.pageNum`: a page named outright —
    /// an index, the first or the last — is a jump and leaves the view behind; a turn to the next
    /// or previous page, or a number of pages from here, is a step and leaves none.
    pub(super) fn turn_to(&mut self, target: PageTarget, events: &mut Vec<Event>) {
        let named = matches!(
            target,
            PageTarget::Index(_) | PageTarget::First | PageTarget::Last
        );
        if named
            && let Some(open) = self.focused_mut()
            && resolve(open, target).is_some_and(|index| index != open.page_index)
        {
            open.leave();
        }
        self.go_to(target, Turn::Requested, events);
    }

    /// [`crate::Command::History`], and a script's `app.goBack` and `app.goForward`: the focused
    /// document's view restored from its history, the view it leaves recorded the other way, or
    /// a sentence where there is nothing to step to. Answers whether the view moved.
    pub(super) fn retrace(&mut self, way: Retrace, events: &mut Vec<Event>) -> bool {
        let Some(id) = self.focused else {
            return false;
        };
        let Some(open) = self.focused_mut() else {
            return false;
        };
        let current = open.viewing();
        let step = match way {
            Retrace::Back => open.history.back(current),
            Retrace::Forward => open.history.forward(current),
        };
        if let Some(view) = step {
            // The view restored is whole, so a destination's view still waiting for `settle` —
            // one a script asked for in the same run as its step back — is of a view left, and
            // applying it after the restore would undo the step.
            open.pending_views.clear();
            self.restore(view, events);
            return true;
        }
        events.push(Event::Reported {
            document: id,
            page: Some(current.page),
            notes: vec![
                match way {
                    Retrace::Back => {
                        "there is no earlier view to go back to: the history holds \
                                      the views a link, an outline item or a page number left, \
                                      and none is left behind this one"
                    }
                    Retrace::Forward => {
                        "there is no later view to go forward to: going forward \
                                         retraces a step back, and none has been taken since \
                                         the latest jump"
                    }
                }
                .to_owned(),
            ],
        });
        false
    }
}
