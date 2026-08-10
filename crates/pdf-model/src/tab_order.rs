//! ISO 32000-2 §12.5.1's navigation order: which annotation the tab key visits next.
//!
//! > Interactive PDF processors may permit the user to navigate through the annotations on a page
//! > by using the keyboard (in particular, the tab key).
//!
//! and Table 31's `/Tabs` is where a producer states which order that is.
//!
//! Table 31 names five values and §12.5.1 defines each of them. Two are about *geometry* (`R` and
//! `C`), one about §14.7's structure tree (`S`), one about the file's own array (`A`), and one is
//! a composition of two (`W`). All five are here, because a clause that states an algorithm is a
//! clause that can be implemented rather than approximated.
//!
//! # Where the geometry comes from, and why the clause says so
//!
//! §12.5.1, closing the list of five:
//!
//! > These descriptions assume the page is being viewed in the orientation specified by the
//! > Rotate entry.
//!
//! So "the topmost row" is topmost *on the screen*, not in the file's coordinates — a page with
//! `/Rotate 90` has its rows running down the paper. [`rotated`] is the whole of that: a `/Rect`'s
//! corners are put through §7.7.3.3's rotation before anything is sorted.
//!
//! # And where the direction comes from
//!
//! §12.5.1 again, of `R`:
//!
//! > The direction within a row is defined by the Direction entry in the viewer preferences
//! > dictionary
//!
//! Table 147's `/Direction`, `L2R` by its own default. It is the one place §12.2 reaches into
//! §12.5, and it is why [`order`] takes the document rather than only the page.
//!
//! # The two places the clause leaves a choice, both taken deliberately
//!
//! - **A page that states no `/Tabs` at all.** The clause makes the entry optional and states no
//!   default, so this is a processor's choice. It is `A`, the `/Annots` array's own order: it is
//!   the order the *file* states, it needs no geometry and no structure tree, and it is what a
//!   producer that thought about tab order and wrote the array in that order gets for free.
//! - **`S` where an annotation is not in the structure tree.** The clause says outright that this
//!   "is determined in a manner of the interactive PDF processor's choosing". Those annotations
//!   follow the ones the tree reached, in array order — the same fallback, so a document with no
//!   structure tree and `/Tabs /S` behaves like `A` rather than like nothing.
//!
//! Measured over the corpus: 61 documents state `/Tabs` on one of their first 50 pages — 95 pages
//! `/S`, 10 `/R`, 5 `/W`, and none `/C` or `/A`.

use pdf_syntax::{Document, ObjectId};

use crate::structure::{Child, Tree};
use crate::viewer_preferences::{Direction, ViewerPreferences};

/// One annotation's box on the screen: `(left, top, right, bottom)`, y downwards.
type Screen = (f32, f32, f32, f32);

/// Most annotations one page's order will be built for.
///
/// A bound on the answer rather than on the document: the order is asked for on a key press, and
/// a page claiming a million annotations must not turn one into a sort of a million rectangles.
/// The same order of magnitude as the other per-page bounds in this crate.
const MAX_ANNOTATIONS: usize = 4096;

/// How far apart two annotations' centres may be, as a fraction of the taller one's height, and
/// still count as being in the same row.
///
/// §12.5.1 says "rows running horizontally across the page" and does not say what a row is — a
/// page's annotations are rarely aligned to the pixel, and a rule that demanded exact equality
/// would make every widget its own row. Half the height is the choice, and it is the one every
/// form does: two fields side by side overlap vertically by much more than that, and two fields
/// on consecutive lines by much less.
const ROW_TOLERANCE: f32 = 0.5;

/// Table 31's `/Tabs`, as §12.5.1 defines the five values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tabs {
    /// `R`: "[a]nnotations shall be visited in rows running horizontally across the page."
    Row,
    /// `C`: "[a]nnotations shall be visited in columns running vertically up and down the page."
    Column,
    /// `S`: "[a]nnotations shall be visited in the order in which they appear in the structure
    /// tree."
    Structure,
    /// `A`: "[a]ll annotations shall be visited in the order in which they appear in the page
    /// `Annots` array." New in PDF 2.0, **and this crate's default for a page that states none** —
    /// see the module comment for why that is a choice rather than a reading.
    #[default]
    Array,
    /// `W`: "[w]idget annotations shall be visited in the order in which they appear in the page
    /// `Annots` array, followed by other annotation types in row order." New in PDF 2.0.
    Widgets,
}

impl Tabs {
    /// Table 31's entry, read from a page dictionary.
    ///
    /// A name the clause does not define is not one of the five, so it gets the default — the
    /// same reading Table 349's `/Trapped` and Table 147's `/PrintScaling` get, and for the same
    /// reason: a value outside a stated set has not stated one.
    #[must_use]
    pub fn read(document: &Document, page: &pdf_syntax::Dictionary) -> Self {
        match document.get_key(page, "Tabs").as_name().and_then(|name| {
            let bytes = name.as_bytes();
            (bytes.len() == 1).then(|| bytes[0])
        }) {
            Some(b'R') => Self::Row,
            Some(b'C') => Self::Column,
            Some(b'S') => Self::Structure,
            Some(b'W') => Self::Widgets,
            _ => Self::Array,
        }
    }
}

/// The annotations of one page, in the order §12.5.1 says the tab key visits them.
///
/// Object identifiers rather than dictionaries, because that is what a focus is: `viewer-core`
/// holds one `ObjectId` and §12.6.3's `/Fo` and `/Bl` are raised against it.
///
/// Every annotation the page names appears exactly once, whichever value is in force — the orders
/// are permutations of the `/Annots` array and not filters of it. An annotation the array names
/// twice is visited once, because a tab order with a cycle in it is not one.
#[must_use]
pub fn order(document: &Document, page: &crate::Page, page_id: ObjectId) -> Vec<ObjectId> {
    let annots = annotations(document, page);
    if annots.len() <= 1 {
        return annots;
    }
    let rotate = page.rotate;
    let box_ = page.display_box;
    match Tabs::read(document, &page.dict) {
        Tabs::Array => annots,
        Tabs::Row => rows(document, &annots, rotate, box_, direction(document)),
        Tabs::Column => columns(document, &annots, rotate, box_, direction(document)),
        Tabs::Structure => structure(document, &annots, page_id),
        Tabs::Widgets => {
            let (widgets, rest): (Vec<ObjectId>, Vec<ObjectId>) =
                annots.iter().partition(|id| is_widget(document, **id));
            let mut out = widgets;
            out.extend(rows(document, &rest, rotate, box_, direction(document)));
            out
        }
    }
}

/// The page's `/Annots`, de-duplicated, in the array's own order.
fn annotations(document: &Document, page: &crate::Page) -> Vec<ObjectId> {
    let object = document.get_key(&page.dict, "Annots");
    let Some(array) = object.as_array() else {
        return Vec::new();
    };
    let mut out: Vec<ObjectId> = Vec::new();
    for entry in array.iter().take(MAX_ANNOTATIONS) {
        if let Some(id) = entry.as_reference()
            && !out.contains(&id)
        {
            out.push(id);
        }
    }
    out
}

/// Table 147's `/Direction`, whose default is `L2R`.
fn direction(document: &Document) -> Direction {
    ViewerPreferences::read(document).direction
}

/// Whether an annotation is §12.5.6.19's widget, which is what `W` makes its first pass over.
fn is_widget(document: &Document, id: ObjectId) -> bool {
    let object = document.get(id);
    object.as_dict().is_some_and(|dict| {
        document
            .get_key(dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Widget")
    })
}

/// An annotation's `/Rect`, in the orientation §7.7.3.3's `/Rotate` puts the page on the screen.
///
/// Returns the box as `(left, top, right, bottom)` with **y downwards**, which is what "the
/// topmost row" and "the top of the first column" mean on a screen. `None` for an annotation with
/// no usable rectangle, which sorts after everything that has one rather than being dropped.
fn rotated(document: &Document, id: ObjectId, rotate: u16, page_box: [f32; 4]) -> Option<Screen> {
    let object = document.get(id);
    let dict = object.as_dict()?;
    let rect = document.get_key(dict, "Rect");
    let array = rect.as_array()?;
    let mut values = [0.0_f32; 4];
    for (slot, entry) in values.iter_mut().zip(array.iter()) {
        // A rectangle whose numbers are not numbers is not a rectangle.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a page coordinate that does not fit an f32 is a page nothing can draw"
        )]
        {
            *slot = document
                .resolve(entry)
                .as_number()
                .map(|value| value as f32)?;
        }
    }
    // §7.7.3.2: "the ordering of the coordinates is not guaranteed".
    let (x0, x1) = (values[0].min(values[2]), values[0].max(values[2]));
    let (y0, y1) = (values[1].min(values[3]), values[1].max(values[3]));

    // The page box in the same space, so the rotation can be expressed about it.
    let (px0, px1) = (page_box[0].min(page_box[2]), page_box[0].max(page_box[2]));
    let (py0, py1) = (page_box[1].min(page_box[3]), page_box[1].max(page_box[3]));

    // User space has y upwards; a screen has it downwards. Each rotation maps a corner
    // (x, y) to the screen point below, and the four are then re-ordered into a box.
    //
    // **These are `content::base_transform`'s four matrices with the raster's y flip folded in**,
    // not a second opinion about §7.7.3.3: that function takes (x, y) to (y, W - x) at 90° and
    // leaves the result y-up, and `TargetSpec` flips it, so screen y at 90° is x. A tab order
    // derived from a different rotation than the one on the screen would visit the annotations in
    // an order nobody could see, which is why this is written as an identity rather than
    // re-derived — and `the_geometry_orders_are_taken_after_the_pages_own_rotation` pins it.
    let map = |x: f32, y: f32| -> (f32, f32) {
        match rotate {
            90 => (y - py0, x - px0),
            180 => (px1 - x, y - py0),
            270 => (py1 - y, px1 - x),
            _ => (x - px0, py1 - y),
        }
    };
    let corners = [map(x0, y0), map(x1, y0), map(x0, y1), map(x1, y1)];
    let left = corners.iter().map(|c| c.0).fold(f32::INFINITY, f32::min);
    let right = corners
        .iter()
        .map(|c| c.0)
        .fold(f32::NEG_INFINITY, f32::max);
    let top = corners.iter().map(|c| c.1).fold(f32::INFINITY, f32::min);
    let bottom = corners
        .iter()
        .map(|c| c.1)
        .fold(f32::NEG_INFINITY, f32::max);
    Some((left, top, right, bottom))
}

/// Each annotation with its screen box, in array order, boxless ones last.
fn placed(
    document: &Document,
    annots: &[ObjectId],
    rotate: u16,
    page_box: [f32; 4],
) -> (Vec<(ObjectId, Screen)>, Vec<ObjectId>) {
    let mut boxed = Vec::new();
    let mut bare = Vec::new();
    for id in annots {
        match rotated(document, *id, rotate, page_box) {
            Some(rect) => boxed.push((*id, rect)),
            None => bare.push(*id),
        }
    }
    (boxed, bare)
}

/// `R`: rows running horizontally, the topmost first, each row in the reading direction.
fn rows(
    document: &Document,
    annots: &[ObjectId],
    rotate: u16,
    page_box: [f32; 4],
    direction: Direction,
) -> Vec<ObjectId> {
    let (mut boxed, bare) = placed(document, annots, rotate, page_box);
    // Topmost first; a stable sort keeps the array's order between two at the same height, which
    // is the only tie-break the clause leaves and the only one that costs nothing to justify.
    boxed.sort_by(|a, b| a.1.1.total_cmp(&b.1.1));

    let mut out = Vec::with_capacity(annots.len());
    let mut row: Vec<(ObjectId, Screen)> = Vec::new();
    let flush = |row: &mut Vec<(ObjectId, Screen)>, out: &mut Vec<ObjectId>| {
        row.sort_by(|a, b| match direction {
            Direction::RightToLeft => b.1.0.total_cmp(&a.1.0),
            Direction::LeftToRight => a.1.0.total_cmp(&b.1.0),
        });
        out.extend(row.drain(..).map(|(id, _)| id));
    };
    for entry in boxed {
        if let Some(first) = row.first()
            && !same_row(first.1, entry.1)
        {
            flush(&mut row, &mut out);
        }
        row.push(entry);
    }
    flush(&mut row, &mut out);
    out.extend(bare);
    out
}

/// `C`: columns running vertically, ordered by the reading direction, each top to bottom.
fn columns(
    document: &Document,
    annots: &[ObjectId],
    rotate: u16,
    page_box: [f32; 4],
    direction: Direction,
) -> Vec<ObjectId> {
    let (mut boxed, bare) = placed(document, annots, rotate, page_box);
    boxed.sort_by(|a, b| match direction {
        Direction::RightToLeft => b.1.0.total_cmp(&a.1.0),
        Direction::LeftToRight => a.1.0.total_cmp(&b.1.0),
    });

    let mut out = Vec::with_capacity(annots.len());
    let mut column: Vec<(ObjectId, Screen)> = Vec::new();
    let flush = |column: &mut Vec<(ObjectId, Screen)>, out: &mut Vec<ObjectId>| {
        column.sort_by(|a, b| a.1.1.total_cmp(&b.1.1));
        out.extend(column.drain(..).map(|(id, _)| id));
    };
    for entry in boxed {
        if let Some(first) = column.first()
            && !same_column(first.1, entry.1)
        {
            flush(&mut column, &mut out);
        }
        column.push(entry);
    }
    flush(&mut column, &mut out);
    out.extend(bare);
    out
}

/// Whether two screen boxes are on one row, by [`ROW_TOLERANCE`].
fn same_row(a: Screen, b: Screen) -> bool {
    let tallest = (a.3 - a.1).max(b.3 - b.1).max(f32::EPSILON);
    ((a.1 + a.3) - (b.1 + b.3)).abs() / 2.0 <= tallest * ROW_TOLERANCE
}

/// Whether two screen boxes are in one column, by the same rule turned ninety degrees.
fn same_column(a: Screen, b: Screen) -> bool {
    let widest = (a.2 - a.0).max(b.2 - b.0).max(f32::EPSILON);
    ((a.0 + a.2) - (b.0 + b.2)).abs() / 2.0 <= widest * ROW_TOLERANCE
}

/// `S`: §14.7's structure tree's order, with whatever it does not reach after it.
fn structure(document: &Document, annots: &[ObjectId], page_id: ObjectId) -> Vec<ObjectId> {
    let Some(tree) = Tree::of(document) else {
        return annots.to_vec();
    };
    let mut out: Vec<ObjectId> = Vec::new();
    // A truncated reading costs nothing here that the clause does not already permit: an
    // annotation the tree does not reach falls through to the array order below, "determined in
    // a manner of the interactive PDF processor's choosing".
    for child in tree.logical_order(document, page_id).items {
        if let Child::Object { object, .. } = child
            && annots.contains(&object)
            && !out.contains(&object)
        {
            out.push(object);
        }
    }
    // "The order for annotations that are not included in the structure tree is determined in a
    // manner of the interactive PDF processor's choosing" — array order, which is the same choice
    // a page with no `/Tabs` gets.
    let missed: Vec<ObjectId> = annots
        .iter()
        .copied()
        .filter(|id| !out.contains(id))
        .collect();
    out.extend(missed);
    out
}

/// The annotation after `from` in this page's order, wrapping at the end.
///
/// `None` for a page with no annotations. `from` being absent, or naming something the page does
/// not have, gives the first — which is what a tab key pressed with nothing focused means.
#[must_use]
pub fn next(order: &[ObjectId], from: Option<ObjectId>) -> Option<ObjectId> {
    step(order, from, 1)
}

/// The annotation before `from`, wrapping at the start. As [`next`], and shift-tab's answer.
#[must_use]
pub fn previous(order: &[ObjectId], from: Option<ObjectId>) -> Option<ObjectId> {
    step(order, from, -1)
}

fn step(order: &[ObjectId], from: Option<ObjectId>, by: isize) -> Option<ObjectId> {
    if order.is_empty() {
        return None;
    }
    let at = from.and_then(|id| order.iter().position(|other| *other == id));
    let Some(at) = at else {
        return if by > 0 {
            order.first().copied()
        } else {
            order.last().copied()
        };
    };
    let len = isize::try_from(order.len()).unwrap_or(isize::MAX);
    let next = isize::try_from(at)
        .unwrap_or(0)
        .saturating_add(by)
        .rem_euclid(len);
    order.get(usize::try_from(next).unwrap_or(0)).copied()
}
