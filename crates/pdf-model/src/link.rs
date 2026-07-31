//! ISO 32000-2 §12.5.6.5's link annotations, as regions a reader can activate.
//!
//! A link is the one annotation whose *point* is what happens when somebody clicks it: it
//! "represents either a hypertext link to a destination elsewhere in the document … or an
//! action to be performed". Its appearance is drawn by `annotation.rs` like any other
//! annotation's; what is here is the other half — where the region is, and where it goes.
//!
//! # The activation region is not always `/Rect`
//!
//! Table 176 gives `/QuadPoints`, "an array of 8×n numbers specifying the coordinates of n
//! quadrilaterals in default user space that comprise the region in which the link should be
//! activated", and then states three conditions under which it is ignored — and the third is
//! the one worth reading twice:
//!
//! > If this entry is not present, or the PDF processor does not recognise it, or if any
//! > coordinates in the QuadPoints array lie outside the region specified by Rect then the
//! > activation region for the link annotation shall be defined by its Rect entry.
//!
//! So a `/QuadPoints` that escapes its own `/Rect` is not a wider region: it is *no* region,
//! and the rectangle stands. Files get this wrong often enough that the sentence exists.

use pdf_syntax::{Dictionary, Document};

use crate::destination::Destination;
use crate::page::{Page, Pages};

/// A link's activation region and where it leads.
#[derive(Debug, Clone)]
pub struct Link {
    /// The activation region, in default user space, as one or more quadrilaterals.
    ///
    /// Quadrilaterals rather than a rectangle because the clause's region is a set of them —
    /// a link across a line break is two — and because a `/Rect` reduces to one.
    pub region: Vec<[f32; 8]>,
    /// Where activating it goes, where this reader can say.
    ///
    /// `None` for a link whose action is not a go-to: a URI, a launch, an ECMAScript action.
    /// Those are named rather than lost — see [`Self::actions`] and `crate::action`.
    pub destination: Option<Destination>,
    /// Everything activating it performs, in §12.6.2's order, from `/A` and its `/Next` chain.
    ///
    /// Empty for a link that states only a `/Dest`, which is a destination rather than an
    /// action — Table 176 makes the two exclusive, "not permitted if an A entry is present".
    /// [`Self::destination`] is what a caller needs to turn a click into a page; this is what
    /// it needs to change a layer (§12.6.4.13) or hide an annotation (§12.6.4.11), and it is
    /// performed by `crate::view::ViewState`.
    pub actions: Vec<crate::action::Action>,
}

impl Link {
    /// Whether a point in default user space is inside the activation region.
    #[must_use]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.region.iter().any(|quad| inside(quad, x, y))
    }
}

/// Every link annotation on a page, in the order the page lists them.
///
/// Order matters when two links overlap, and the clause states no rule for it — so the
/// document's own order stands, and [`at`] takes the last match, which is the one drawn on top.
#[must_use]
pub fn links(document: &Document, page: &Page) -> Vec<Link> {
    let annotations = document.get_key(&page.dict, "Annots");
    let Some(annotations) = annotations.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for annotation in annotations {
        let annotation = document.resolve(annotation);
        let Some(annotation) = annotation.as_dict() else {
            continue;
        };
        if document
            .get_key(annotation, "Subtype")
            .as_name()
            .is_none_or(|subtype| subtype.as_bytes() != b"Link")
        {
            continue;
        }
        let Some(rect) = rectangle(document, annotation) else {
            continue;
        };
        let actions = crate::action::read(
            document,
            annotation.get("A").unwrap_or(&pdf_syntax::Object::Null),
        );
        out.push(Link {
            region: region(document, annotation, rect),
            destination: destination(document, annotation, &actions),
            actions,
        });
    }
    out
}

/// The link a point activates, or `None`.
///
/// The *last* match, because a page lists its annotations in painting order and the one on top
/// is the one a reader sees under the cursor.
#[must_use]
pub fn at(links: &[Link], x: f32, y: f32) -> Option<&Link> {
    links.iter().rev().find(|link| link.contains(x, y))
}

/// The page a link leads to, resolved.
///
/// A convenience over [`Link::destination`] and [`Destination::page_index`], because every
/// caller wants the same two steps.
#[must_use]
pub fn target(document: &Document, pages: &Pages<'_>, link: &Link) -> Option<usize> {
    link.destination?.page_index(document, pages)
}

/// Table 176's `/Dest`, or the destination inside `/A`'s go-to action.
///
/// The clause makes them exclusive — `/Dest` is "not permitted if an A entry is present" — so a
/// file writing both has broken a rule, and `/Dest` is read first because it is the direct
/// statement of the same thing.
///
/// The `/A` half comes from the already-read action list rather than from the dictionary again,
/// which is what makes a go-to buried in a `/Next` chain reachable: §12.6.2 lets a link play a
/// sound and *then* jump, and reading only the outermost `/S` would miss the jump.
fn destination(
    document: &Document,
    annotation: &Dictionary,
    actions: &[crate::action::Action],
) -> Option<Destination> {
    if let Some(dest) = annotation.get("Dest")
        && let Some(destination) = Destination::read(document, dest)
    {
        return Some(destination);
    }
    actions.iter().find_map(|action| match action {
        crate::action::Action::GoTo(destination) => Some(*destination),
        _ => None,
    })
}

/// The activation region: `/QuadPoints` where the clause admits them, `/Rect` otherwise.
fn region(document: &Document, annotation: &Dictionary, rect: [f32; 4]) -> Vec<[f32; 8]> {
    let quads = document.get_key(annotation, "QuadPoints");
    let Some(items) = quads.as_array() else {
        return vec![corners(rect)];
    };
    let numbers: Vec<f32> = items
        .iter()
        .filter_map(|item| document.resolve(item).as_number().map(narrow))
        .filter(|value| value.is_finite())
        .collect();
    // "8×n numbers": a partial quadrilateral is not one, and a file short of eight has stated
    // no region at all.
    if numbers.is_empty() || !numbers.len().is_multiple_of(8) {
        return vec![corners(rect)];
    }
    let mut out = Vec::new();
    for quad in numbers.chunks_exact(8) {
        let mut points = [0.0f32; 8];
        points.copy_from_slice(quad);
        // The clause's third condition, and the one a lenient reader gets wrong: a coordinate
        // outside `/Rect` sends the *whole* annotation back to its rectangle.
        if points.chunks_exact(2).any(|point| {
            point.first().is_none_or(|x| *x < rect[0] || *x > rect[2])
                || point.get(1).is_none_or(|y| *y < rect[1] || *y > rect[3])
        }) {
            return vec![corners(rect)];
        }
        out.push(points);
    }
    out
}

/// A rectangle as a quadrilateral, in the clause's counterclockwise vertex order.
fn corners(rect: [f32; 4]) -> [f32; 8] {
    [
        rect[0], rect[1], rect[2], rect[1], rect[2], rect[3], rect[0], rect[3],
    ]
}

/// Whether a point is inside a quadrilateral, by the crossing-number rule.
///
/// The clause's quadrilaterals are "in counterclockwise order" but real files write them in
/// every order, and a fill rule that depends on winding would answer differently for the two.
/// Crossing number does not.
fn inside(quad: &[f32; 8], x: f32, y: f32) -> bool {
    let mut crossings = false;
    let points: Vec<(f32, f32)> = quad
        .chunks_exact(2)
        .filter_map(|pair| Some((*pair.first()?, *pair.get(1)?)))
        .collect();
    let count = points.len();
    for index in 0..count {
        let Some((x0, y0)) = points.get(index).copied() else {
            continue;
        };
        // The next vertex, wrapping to the first, which is what closes the quadrilateral.
        let next = index.saturating_add(1).checked_rem(count).unwrap_or(0);
        let Some((x1, y1)) = points.get(next).copied() else {
            continue;
        };
        if (y0 > y) != (y1 > y) {
            let span = y1 - y0;
            if span != 0.0 && x < (x1 - x0) * (y - y0) / span + x0 {
                crossings = !crossings;
            }
        }
    }
    crossings
}

/// Reads `/Rect`, normalising the corner order as §12.5.2 requires of every annotation.
fn rectangle(document: &Document, annotation: &Dictionary) -> Option<[f32; 4]> {
    let array = document.get_key(annotation, "Rect");
    let items = array.as_array()?;
    let mut values = [0f32; 4];
    for (slot, item) in values.iter_mut().zip(items) {
        let number = document.resolve(item).as_number()?;
        if !number.is_finite() {
            return None;
        }
        *slot = narrow(number);
    }
    Some([
        values[0].min(values[2]),
        values[1].min(values[3]),
        values[0].max(values[2]),
        values[1].max(values[3]),
    ])
}

/// Narrows a coordinate to the precision the rest of this crate works in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "annotation coordinates are default user space, bounded by the format at 14 400 \
              units, far inside f32's exact integer range"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::{at, links};
    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// A two-page document whose first page carries the links the tests ask about.
    fn linked(annotations: &str) -> Document {
        document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            &format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots {annotations} >>"
            ),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
        ])
    }

    /// A link's rectangle is its activation region, and a click outside it activates nothing.
    #[test]
    fn a_rectangle_is_the_region_and_it_has_edges() {
        let doc =
            linked("[<< /Type /Annot /Subtype /Link /Rect [10 10 50 30] /Dest [4 0 R /Fit] >>]");
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("page one");
        let found = links(&doc, &page);
        assert_eq!(found.len(), 1);

        assert!(at(&found, 20.0, 20.0).is_some(), "inside");
        assert!(at(&found, 5.0, 20.0).is_none(), "left of it");
        assert!(at(&found, 20.0, 40.0).is_none(), "above it");
        assert_eq!(
            at(&found, 20.0, 20.0).and_then(|link| super::target(&doc, &pages, link)),
            Some(1),
            "and it leads to the second page"
        );
    }

    /// `/QuadPoints` narrow the region — and a quadrilateral outside `/Rect` widens nothing.
    ///
    /// Both halves of Table 176's sentence in one test, because they pull opposite ways: a
    /// well-formed `/QuadPoints` is the region, and one whose coordinates "lie outside the
    /// region specified by Rect" sends the whole annotation back to the rectangle rather than
    /// extending it.
    #[test]
    fn quad_points_narrow_the_region_and_a_stray_one_is_ignored() {
        let inside_rect = linked(
            "[<< /Type /Annot /Subtype /Link /Rect [0 0 100 100] \
             /QuadPoints [10 10 30 10 30 30 10 30] /Dest [4 0 R /Fit] >>]",
        );
        let pages = crate::page::Pages::new(&inside_rect);
        let page = pages.get(0).expect("page one");
        let found = links(&inside_rect, &page);
        assert!(at(&found, 20.0, 20.0).is_some(), "inside the quadrilateral");
        assert!(
            at(&found, 80.0, 80.0).is_none(),
            "inside /Rect but outside the quadrilateral, which is the point of the entry"
        );

        let escaping = linked(
            "[<< /Type /Annot /Subtype /Link /Rect [0 0 100 100] \
             /QuadPoints [10 10 300 10 300 30 10 30] /Dest [4 0 R /Fit] >>]",
        );
        let pages = crate::page::Pages::new(&escaping);
        let page = pages.get(0).expect("page one");
        let found = links(&escaping, &page);
        assert!(
            at(&found, 80.0, 80.0).is_some(),
            "a quadrilateral outside /Rect returns the region to /Rect, which covers this point"
        );
        assert!(
            at(&found, 150.0, 20.0).is_none(),
            "and not to the stray one"
        );
    }

    /// A link with no go-to has no destination, and is still a link.
    ///
    /// A URI action needs a network this program does not have, and §12.6.4.5's launch action
    /// is absent for the reason principle 3 gives. Neither is a reason to forget the region:
    /// a viewer that knows where a link is can say it cannot follow it.
    #[test]
    fn a_link_that_is_not_a_go_to_keeps_its_region() {
        let doc = linked(
            "[<< /Type /Annot /Subtype /Link /Rect [10 10 50 30] \
             /A << /S /URI /URI (https://example.invalid/) >> >>]",
        );
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("page one");
        let found = links(&doc, &page);
        assert_eq!(found.len(), 1);
        assert!(at(&found, 20.0, 20.0).is_some());
        assert!(found.first().is_some_and(|link| link.destination.is_none()));
    }
}
