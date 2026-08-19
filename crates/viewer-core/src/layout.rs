//! ISO 32000-2 Table 29's `/PageLayout`: which pages the viewport shows, and where each sits.
//!
//! ISO 32000-2 §7.7.2, Table 29's `/PageLayout` row:
//!
//! > A name object specifying the page layout shall be used when the document is opened
//!
//! and its six values, of which the table's own default is `SinglePage`. It states each one in a
//! phrase: `SinglePage`
//!
//! > Display one page at a time
//!
//! `OneColumn`
//!
//! > Display the pages in one column
//!
//! and the four that stand two side by side, `TwoColumnLeft` and `TwoColumnRight` in two columns
//! with the odd-numbered pages on the named side, `TwoPageLeft` and `TwoPageRight`
//!
//! > Display the pages two at a time
//!
//! with the odd-numbered pages on the named side likewise.
//!
//! **The standard's own words divide the six along two axes rather than one**, and reading them
//! that way is what makes this module small. *How many pages stand side by side*: one for
//! `SinglePage` and `OneColumn`, two for the other four. *Whether the view runs on past the
//! bottom of a page*: "in one column" and "in two columns" describe a column of pages that a
//! reader moves through, while "one page at a time" and "two at a time" say what is on the screen
//! **at a time** — so the first pair scroll across page boundaries and the second pair do not.
//! Nothing else in the six differs.
//!
//! **"Odd-numbered" is the page's number and not this crate's index.** §12.4.2 makes the first
//! page "page index 0" internally while every human-facing count in the standard starts at one,
//! so `TwoColumnLeft` puts index 0 — page one — in the *left* column, and `TwoColumnRight` puts
//! it in the right one with nothing beside it. That is the arrangement a bound book has, and it
//! is the arithmetic in [`row_of`] rather than a special case anywhere else.
//!
//! **Table 147's `/Direction` is deliberately not applied here.** That entry says a reading
//! direction "may be used to determine the relative positioning of pages when displayed side by
//! side or printed n-up" — a *may*, and `/PageLayout` has already said *left* and *right* in so
//! many words. Where one entry states a side outright and another permits a rearrangement, the
//! statement wins; a document wanting its odd pages on the right has `TwoColumnRight` to say so.
//! Recorded as a choice, because the alternative reading is available and this is not.

use pdf_model::viewer_preferences::PageLayout;

use crate::open::{Open, raster_extent};

/// The space between two neighbouring pages, in logical pixels.
///
/// The standard states no separation at all — it says pages are displayed in a column and says
/// nothing about what is between them — so this is a choice, and it is written down as one. It is
/// logical rather than device pixels for the reason §12.5.3's `NoZoom` gives: a gap is a thing a
/// person sees, and a doubled display should draw it at the same apparent size.
const GAP: f32 = 8.0;

/// The most pages one arrangement puts on the screen at once.
///
/// A bound rather than a limit on stupidity, in the same sense as `ZOOM_RANGE`: two columns of
/// four rows is what a window shows before the pages are too small to read, and without a bound a
/// document magnified to 2% would ask this crate to interpret hundreds of pages on one scroll —
/// which is exactly the eager work `CLAUDE.md`'s startup rules forbid. Pages past it are simply
/// not placed, and the row they are in scrolls into view like any other.
pub(crate) const MOST: usize = 8;

/// One page of the arrangement, placed in the viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    /// Which page, zero-based.
    pub(crate) page: usize,
    /// Where its raster's top-left corner sits in the viewport, in device pixels.
    pub(crate) origin: (f32, f32),
    /// The raster this page occupies at the magnification the placement was made at.
    pub(crate) raster: (u32, u32),
}

/// How many pages stand side by side under this layout.
pub(crate) const fn columns(layout: PageLayout) -> usize {
    match layout {
        PageLayout::SinglePage | PageLayout::OneColumn => 1,
        PageLayout::TwoColumnLeft
        | PageLayout::TwoColumnRight
        | PageLayout::TwoPageLeft
        | PageLayout::TwoPageRight => 2,
    }
}

/// Whether the view runs on past the bottom of a page — "in one column" rather than "at a time".
pub(crate) const fn continuous(layout: PageLayout) -> bool {
    matches!(
        layout,
        PageLayout::OneColumn | PageLayout::TwoColumnLeft | PageLayout::TwoColumnRight
    )
}

/// Whether page **one** stands in the right-hand column, which is what the two `…Right` values say.
const fn odd_on_the_right(layout: PageLayout) -> bool {
    matches!(
        layout,
        PageLayout::TwoColumnRight | PageLayout::TwoPageRight
    )
}

/// Which row of the arrangement a page falls in.
pub(crate) const fn row_of(layout: PageLayout, page: usize) -> usize {
    if columns(layout) == 1 {
        page
    } else if odd_on_the_right(layout) {
        // Page one is alone on the right of row zero, so every later row holds an even page
        // number beside the odd one after it: indices 1 and 2, then 3 and 4, and so on.
        page.saturating_add(1) / 2
    } else {
        page / 2
    }
}

/// The first page of a row, which is the one drawn leftmost.
const fn first_of(layout: PageLayout, row: usize) -> usize {
    if columns(layout) == 1 {
        row
    } else if odd_on_the_right(layout) {
        if row == 0 {
            0
        } else {
            row.saturating_mul(2).saturating_sub(1)
        }
    } else {
        row.saturating_mul(2)
    }
}

/// The pages of a row, in the order they are drawn from left to right.
pub(crate) fn pages_in(layout: PageLayout, row: usize, page_count: usize) -> Vec<usize> {
    let first = first_of(layout, row);
    // Row zero of an odd-on-the-right arrangement holds page one alone: the left-hand slot is
    // the empty place a bound book leaves in front of its cover.
    let width = if columns(layout) == 2 && odd_on_the_right(layout) && row == 0 {
        1
    } else {
        columns(layout)
    };
    (first..first.saturating_add(width))
        .take_while(|page| *page < page_count)
        .collect()
}

/// How many rows the whole arrangement has.
pub(crate) fn rows(layout: PageLayout, page_count: usize) -> usize {
    match page_count.checked_sub(1) {
        Some(last) => row_of(layout, last).saturating_add(1),
        None => 0,
    }
}

/// One row measured at a magnification: its pages with their rasters, and what it covers.
#[derive(Debug, Default)]
struct Row {
    /// Each page of the row with the raster it occupies, left to right.
    pages: Vec<(usize, (u32, u32))>,
    /// The row's width in device pixels, gaps between its pages included.
    width: f32,
    /// The row's height in device pixels, which is its tallest page's.
    height: f32,
}

/// Measures a row without interpreting anything: page extents and the magnification, no more.
fn measure(open: &Open, row: usize, magnification: f32, gap: f32) -> Row {
    let mut measured = Row::default();
    for page in pages_in(open.layout, row, open.page_count) {
        let Some(size) = open.page_size(page) else {
            continue;
        };
        let raster = raster_extent(size, magnification);
        if !measured.pages.is_empty() {
            measured.width += gap;
        }
        measured.width += px(raster.0);
        measured.height = measured.height.max(px(raster.1));
        measured.pages.push((page, raster));
    }
    measured
}

/// Where an extent sits along one axis: centred where there is slack, scrolled where there is not.
///
/// The arithmetic [`Open::origin`] has always used, moved here because a row is what it now
/// applies to and a row is this module's.
fn along(viewport: u32, extent: f32, scroll: f32) -> f32 {
    let slack = f64::from(viewport) - f64::from(extent);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a difference of two pixel counts, both bounded by MAX_EXTENT"
    )]
    let slack = slack as f32;
    if slack > 0.0 { slack / 2.0 } else { -scroll }
}

/// A pixel count as a float, without a lossy cast anybody has to read twice.
fn px(pixels: u32) -> f32 {
    crate::viewer::px(pixels)
}

/// The gap in device pixels, from the display's scale.
fn gap_at(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        GAP * scale
    } else {
        GAP
    }
}

/// Where the top edge of the row holding the current page sits in the viewport.
///
/// The one place the two axes of the layout meet: a continuous arrangement measures the scroll
/// from this row's top and never centres it vertically, because there is always more document
/// above or below; a page-at-a-time arrangement centres a row shorter than the window exactly as
/// a single page has always been centred.
fn anchor_top(open: &Open, viewport: (u32, u32), row: &Row) -> f32 {
    if continuous(open.layout) && rows(open.layout, open.page_count) > 1 {
        -open.scroll.1
    } else {
        along(viewport.1, row.height, open.scroll.1)
    }
}

/// Lays one row into the viewport at a given top edge.
fn lay(open: &Open, viewport: (u32, u32), row: &Row, top: f32, gap: f32, out: &mut Vec<Placement>) {
    let mut x = along(viewport.0, row.width, open.scroll.0);
    for (page, raster) in &row.pages {
        out.push(Placement {
            page: *page,
            // Top-aligned inside the row. Two pages of unequal height standing side by side
            // share their top edge, which is what a spread of a book does and what a reader
            // reading across the fold expects; the standard states nothing about it.
            origin: (x, top),
            raster: *raster,
        });
        x += px(raster.0) + gap;
    }
}

/// The pages the viewport shows now, in page order, each with its place in it.
///
/// **Nothing here walks the whole page tree.** A row is measured only when it is about to be
/// placed, the walk stops as soon as a row starts below the viewport's bottom edge, and
/// [`MOST`] bounds it whatever the magnification — so a 500-page document costs exactly what a
/// 5-page one costs, which is `CLAUDE.md`'s startup rule stated as an algorithm.
pub(crate) fn place(
    open: &Open,
    viewport: (u32, u32),
    scale: f32,
    magnification: f32,
) -> Vec<Placement> {
    let gap = gap_at(scale);
    let count = rows(open.layout, open.page_count);
    if count == 0 {
        return Vec::new();
    }
    let anchor = row_of(open.layout, open.page_index).min(count.saturating_sub(1));
    let measured = measure(open, anchor, magnification, gap);
    let top = anchor_top(open, viewport, &measured);
    let mut out = Vec::new();
    lay(open, viewport, &measured, top, gap, &mut out);
    if !continuous(open.layout) {
        return out;
    }

    // Upwards from the anchor while there is viewport above its top edge, then downwards. Two
    // walks rather than one because the scroll is measured from the anchor's own top: a row
    // above it is at a negative offset that only its own height can say.
    let mut above = Vec::new();
    let mut edge = top;
    let mut row = anchor;
    while edge > 0.0 && row > 0 && out.len().saturating_add(above.len()) < MOST {
        row = row.saturating_sub(1);
        let measured = measure(open, row, magnification, gap);
        edge -= measured.height + gap;
        lay(open, viewport, &measured, edge, gap, &mut above);
    }
    let mut edge = top + measured.height + gap;
    let mut row = anchor;
    while edge < px(viewport.1)
        && row.saturating_add(1) < count
        && out.len().saturating_add(above.len()) < MOST
    {
        row = row.saturating_add(1);
        let measured = measure(open, row, magnification, gap);
        lay(open, viewport, &measured, edge, gap, &mut out);
        edge += measured.height + gap;
    }
    above.extend(out);
    above.sort_by_key(|placement| placement.page);
    above
}

/// Where one page's raster sits in the viewport, or `None` for a page the arrangement does not
/// show.
///
/// Answered out of [`place`] rather than by a second arithmetic, which is ADR 0118's rule: a
/// second opinion about the origin is how the origin comes to be wrong in one of the two places.
pub(crate) fn origin_of(
    open: &Open,
    page: usize,
    viewport: (u32, u32),
    scale: f32,
    magnification: f32,
) -> Option<(f32, f32)> {
    place(open, viewport, scale, magnification)
        .into_iter()
        .find(|placement| placement.page == page)
        .map(|placement| placement.origin)
}

/// Holds the scroll inside the arrangement, moving the current page when it crosses a row.
///
/// Two jobs in one function because they are one invariant: the scroll is measured from the top
/// of the current page's row, so a scroll that has left that row behind is a scroll whose *origin*
/// has to move with it. Nothing on the screen shifts when it does — the same distance is
/// subtracted from the scroll as is added by the new row — which is what makes a continuous view
/// feel like one surface rather than like a sequence of pages.
///
/// Returns the page that was current before, where it changed, so that the caller can raise
/// §12.6.3's page events for the turn exactly as an arrow key's turn raises them.
pub(crate) fn settle_scroll(
    open: &mut Open,
    viewport: (u32, u32),
    scale: f32,
    magnification: f32,
) -> Option<usize> {
    let gap = gap_at(scale);
    let count = rows(open.layout, open.page_count);
    if count == 0 {
        return None;
    }
    let was = open.page_index;
    let mut row = row_of(open.layout, open.page_index).min(count.saturating_sub(1));

    if continuous(open.layout) {
        // Forward while the current row has gone entirely above the window, and backwards while
        // its top edge has fallen below the window's. `MOST` bounds both, because a scroll of a
        // hundred pages in one message is a `GoTo` wearing a wheel's clothes.
        for _ in 0..MOST {
            let height = measure(open, row, magnification, gap).height + gap;
            if open.scroll.1 >= height && row.saturating_add(1) < count {
                open.scroll.1 -= height;
                row = row.saturating_add(1);
            } else if open.scroll.1 < 0.0 && row > 0 {
                row = row.saturating_sub(1);
                open.scroll.1 += measure(open, row, magnification, gap).height + gap;
            } else {
                break;
            }
        }
        let first = first_of(open.layout, row);
        if row_of(open.layout, open.page_index) != row {
            open.page_index = first.min(open.page_count.saturating_sub(1));
        }
    }

    let measured = measure(open, row, magnification, gap);
    // The last row is where the document ends, so the scroll stops at its bottom edge; every
    // other row hands what is left of the scroll to the row below through the loop above.
    let down = if continuous(open.layout) && row.saturating_add(1) < count {
        f32::MAX
    } else {
        (measured.height - px(viewport.1)).max(0.0)
    };
    let across = (measured.width - px(viewport.0)).max(0.0);
    open.scroll.0 = open.scroll.0.clamp(0.0, across);
    open.scroll.1 = open.scroll.1.clamp(0.0, down);
    (was != open.page_index).then_some(was)
}

#[cfg(test)]
mod tests {
    use super::{PageLayout, pages_in, row_of, rows};

    /// Table 29's own words about which side an odd-numbered page is on, as rows.
    ///
    /// The values are read off the clause rather than off this module: page **one** is odd, so
    /// `TwoColumnLeft` opens with pages one and two side by side and `TwoColumnRight` opens with
    /// page one alone on the right — the arrangement of a bound book, and the reason the two
    /// values exist separately at all.
    #[test]
    fn odd_numbered_pages_are_on_the_side_the_layout_names() {
        for layout in [PageLayout::SinglePage, PageLayout::OneColumn] {
            assert_eq!(pages_in(layout, 0, 10), vec![0], "one page to a row");
            assert_eq!(pages_in(layout, 3, 10), vec![3]);
            assert_eq!(rows(layout, 10), 10);
        }
        for layout in [PageLayout::TwoColumnLeft, PageLayout::TwoPageLeft] {
            assert_eq!(pages_in(layout, 0, 10), vec![0, 1], "pages one and two");
            assert_eq!(pages_in(layout, 1, 10), vec![2, 3]);
            assert_eq!(row_of(layout, 3), 1);
            assert_eq!(rows(layout, 10), 5);
            assert_eq!(pages_in(layout, 4, 9), vec![8], "an odd count ends short");
        }
        for layout in [PageLayout::TwoColumnRight, PageLayout::TwoPageRight] {
            assert_eq!(
                pages_in(layout, 0, 10),
                vec![0],
                "page one, alone, on the right"
            );
            assert_eq!(pages_in(layout, 1, 10), vec![1, 2]);
            assert_eq!(row_of(layout, 0), 0);
            assert_eq!(row_of(layout, 1), 1);
            assert_eq!(row_of(layout, 2), 1);
            assert_eq!(row_of(layout, 3), 2);
            assert_eq!(rows(layout, 10), 6);
        }
    }
}
