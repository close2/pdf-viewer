//! What is under the pointer on the page, as the three things a window shows a different cursor
//! for.
//!
//! ISO 32000-2 states no cursor at all, so the shape over each is a toolkit's convention and each
//! window names its own. *Which* of the three the pointer is over is one decision for every
//! window, and it is made here so that no two windows make it differently (ADR 2856).

use viewer_core::{Answer, Query, Viewer};

/// What the pointer is over, in the order a press would act on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Under {
    /// Nothing a press acts on by itself: a picture, a margin, no page.
    #[default]
    Nothing,
    /// A glyph of the page's text, where a press starts a selection.
    Text,
    /// §12.5.6.5's activation region, where a press follows the link.
    Link,
}

impl Under {
    /// What is under `at`, in device pixels of the viewport.
    ///
    /// **A link wins over the text it lies on**, because a press there follows the link rather
    /// than starting a selection — and a cursor is a promise about what a press will do. Both
    /// are queries, asked at pointer speed (ADR 0166, ADR 2856).
    #[must_use]
    pub fn at(viewer: &Viewer, at: (f32, f32)) -> Self {
        if matches!(viewer.query(Query::LinkAt(at)), Answer::Link(true)) {
            Self::Link
        } else if matches!(viewer.query(Query::TextAt(at)), Answer::OverText(true)) {
            Self::Text
        } else {
            Self::Nothing
        }
    }
}
