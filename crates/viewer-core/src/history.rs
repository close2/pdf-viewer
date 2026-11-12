//! The window's history of views: where the reader was before each jump, so that they can go back
//! to it and forward again (ADR 2805).
//!
//! **The standard keeps no such history and names no gesture for one.** ISO 32000-2 §12.3.2 says
//! what a destination is — "a particular view of a document", its page, the window's location on
//! it and the magnification — and §12.6.4.2's go-to action and §12.3.3's outline items say when a
//! reader is taken to one; nothing in either clause says what becomes of the view the reader was
//! taken *from*. Keeping it is therefore a documented choice under principle 5, and so is each of
//! the three things that decide its shape:
//!
//! - **A view is a [`Viewing`]**: §12.3.2.1's three items as this crate holds them — the page, the
//!   magnification and where the page sits in the window — answered after the clamp, so that going
//!   back puts the reader exactly where they were rather than near it.
//! - **A jump is recorded, a step is not.** A destination followed — a link, an outline item, a
//!   go-to action, a script's named destination — and a page named outright (a page number, the
//!   first or the last page, a script's `this.pageNum`) each leave a view behind. A turn to the
//!   next or previous page, a scroll and a zoom move *from* where the reader is and are undone by
//!   the opposite step, so a history that recorded them would make going back walk the document a
//!   page at a time.
//! - **Going back and forward is two stacks, as a browser's is**: a jump clears what lay ahead, and
//!   a step back puts the view it leaves on the forward stack. Adobe's *JavaScript for Acrobat API
//!   Reference*, "app methods", calls the same thing the view stack (cited for `app.goBack`, never
//!   quoted).
//!
//! Each stack holds at most [`MAX_VIEWS`]; past it the oldest view is forgotten, because a
//! document's own `/Next` chains and timed scripts can jump without a person, and principle 3's
//! budgets bind what a document can make this program hold.

use crate::command::Viewing;
use crate::open::Open;

/// The most views either stack holds.
///
/// A choice, and a generous one: a person following links rarely goes back further than a few
/// dozen, and a document that jumps on its own for ever is held to this rather than to memory.
pub(crate) const MAX_VIEWS: usize = 256;

/// One open document's views behind the reader and ahead of them.
///
/// Per document rather than per window, because a view names a page *of a document*: a document
/// that is closed, or replaced by §12.6.4.4's embedded go-to, takes its views with it, and the
/// view it was replaced from is a page of a document no longer open.
#[derive(Debug, Default)]
pub(crate) struct History {
    /// The views left by jumps, the latest last.
    behind: Vec<Viewing>,
    /// The views left by steps back, the nearest last.
    ahead: Vec<Viewing>,
}

impl History {
    /// Records the view a jump left: it goes behind the reader, and what lay ahead is forgotten.
    ///
    /// A view equal to the latest one behind is not recorded twice — one jump that both turns the
    /// page and states where to look on it reaches this from both halves.
    pub(crate) fn left(&mut self, view: Viewing) {
        self.ahead.clear();
        if self.behind.last() == Some(&view) {
            return;
        }
        push_bounded(&mut self.behind, view);
    }

    /// The view to go back to from `current`, which goes ahead of the reader; `None` where no jump
    /// has left one.
    pub(crate) fn back(&mut self, current: Viewing) -> Option<Viewing> {
        let previous = self.behind.pop()?;
        push_bounded(&mut self.ahead, current);
        Some(previous)
    }

    /// The view to go forward to from `current`, which goes behind the reader; `None` where no
    /// step back has left one.
    pub(crate) fn forward(&mut self, current: Viewing) -> Option<Viewing> {
        let next = self.ahead.pop()?;
        push_bounded(&mut self.behind, current);
        Some(next)
    }

    /// How many views lie behind the reader and how many ahead.
    #[cfg(test)]
    pub(crate) fn depths(&self) -> (usize, usize) {
        (self.behind.len(), self.ahead.len())
    }
}

impl Open {
    /// The view this document is shown at: §12.3.2.1's three items as [`crate::Query::View`]
    /// answers them.
    pub(crate) fn viewing(&self) -> Viewing {
        Viewing {
            page: self.page_index,
            zoom: self.zoom,
            scroll: self.scroll,
        }
    }

    /// Records the view this document is shown at as one a jump is about to leave.
    pub(crate) fn leave(&mut self) {
        let view = self.viewing();
        self.history.left(view);
    }
}

/// Pushes `view`, forgetting the oldest where the stack is at [`MAX_VIEWS`].
fn push_bounded(stack: &mut Vec<Viewing>, view: Viewing) {
    if stack.len() == MAX_VIEWS {
        stack.remove(0);
    }
    stack.push(view);
}

#[cfg(test)]
mod tests {
    use super::{History, MAX_VIEWS};
    use crate::command::{Viewing, Zoom};

    fn page(page: usize) -> Viewing {
        Viewing {
            page,
            zoom: Zoom::FitPage,
            scroll: (0.0, 0.0),
        }
    }

    #[test]
    fn back_and_forward_walk_the_views_jumps_left_and_a_jump_forgets_what_lay_ahead() {
        let mut history = History::default();
        history.left(page(0));
        history.left(page(4));
        assert_eq!(history.back(page(9)), Some(page(4)));
        assert_eq!(history.back(page(4)), Some(page(0)));
        assert_eq!(history.back(page(0)), None);
        assert_eq!(history.forward(page(0)), Some(page(4)));
        assert_eq!(history.depths(), (1, 1));
        history.left(page(0));
        assert_eq!(
            history.depths(),
            (1, 0),
            "a jump forgets what lay ahead, and the same view is not recorded twice"
        );
        assert_eq!(history.forward(page(2)), None);
    }

    #[test]
    fn a_stack_past_its_bound_forgets_its_oldest_view() {
        let mut history = History::default();
        for index in 0..=MAX_VIEWS {
            history.left(page(index));
        }
        assert_eq!(history.depths(), (MAX_VIEWS, 0));
        let mut last = None;
        while let Some(view) = history.back(page(0)) {
            last = Some(view);
        }
        assert_eq!(last, Some(page(1)));
    }
}
