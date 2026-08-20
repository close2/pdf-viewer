//! Moving between ISO 32000-2 Table 29's six arrangements, which is a host's job and not a clause's.
//!
//! §7.7.2's `/PageLayout` row states the layout that "shall be used when the document is opened"
//! and names six values; it says nothing whatever about a reader changing their mind afterwards.
//! So an *order* over the six is a user interface — this project's choice, written down as one —
//! and `viewer_core::Command::Layout` is how a host says which one the reader has moved to.
//!
//! **It is here because three hosts wanted the same function.** `viewer-gtk` and `viewer-qt` each
//! wrote it in the six-hundred-and-sixth session, with a comment in one saying the other had it
//! "deliberately"; `viewer-ui` would have been the third copy. Which key cycles is a toolkit's —
//! a `gdk::Key`, a `Qt::Key`, a winit `Key::Character` — and *what the next arrangement is* is
//! not, which is this crate's whole test for what belongs in it (ADR 0246).

use pdf_model::viewer_preferences::PageLayout;

/// The next of Table 29's six arrangements, in the order the table itself states them.
///
/// A cycle rather than a list with an end, so that one key reaches all six and the sixth press
/// returns to where it started. Table 29 prints them `SinglePage`, `OneColumn`, `TwoColumnLeft`,
/// `TwoColumnRight`, `TwoPageLeft`, `TwoPageRight`, and following the table's own order is the
/// one choice here that is not arbitrary.
#[must_use]
pub const fn next_layout(layout: PageLayout) -> PageLayout {
    match layout {
        PageLayout::SinglePage => PageLayout::OneColumn,
        PageLayout::OneColumn => PageLayout::TwoColumnLeft,
        PageLayout::TwoColumnLeft => PageLayout::TwoColumnRight,
        PageLayout::TwoColumnRight => PageLayout::TwoPageLeft,
        PageLayout::TwoPageLeft => PageLayout::TwoPageRight,
        PageLayout::TwoPageRight => PageLayout::SinglePage,
    }
}

#[cfg(test)]
mod tests {
    use pdf_model::viewer_preferences::PageLayout;

    use super::next_layout;

    /// Six presses reach all six and come back, which is what makes one key enough.
    #[test]
    fn the_cycle_visits_every_arrangement_once_and_returns() {
        let mut seen = Vec::new();
        let mut layout = PageLayout::SinglePage;
        for _ in 0..6 {
            seen.push(layout);
            layout = next_layout(layout);
        }
        assert_eq!(layout, PageLayout::SinglePage, "the sixth press comes home");
        assert_eq!(
            seen,
            vec![
                PageLayout::SinglePage,
                PageLayout::OneColumn,
                PageLayout::TwoColumnLeft,
                PageLayout::TwoColumnRight,
                PageLayout::TwoPageLeft,
                PageLayout::TwoPageRight,
            ],
            "Table 29's own order"
        );
    }
}
