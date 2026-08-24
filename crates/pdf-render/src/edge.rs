//! What the *edge* of a shape thicker than one device pixel covers of the pixel it crosses:
//! ISO 32000-2 §10.7.4, one step past [`crate::sub_pixel`].
//!
//! # The three modules, and which sentence each answers
//!
//! [`crate::collapsed`] answers a subpath with **no** area, [`crate::sub_pixel`] a subpath whose
//! area is smaller than the rasteriser can measure, and this one a subpath far larger than a
//! pixel whose **boundary** falls part way across one. The first two are about a shape
//! disappearing; this is about the third sentence of the same paragraph:
//!
//! > The area covered by painted pixels shall always be at least as large as the area of the
//! > original shape.
//!
//! # Why a boundary pixel needs anything said about it at all
//!
//! Both backends of this tree anti-alias, which is a departure from the clause read literally and
//! is licensed by §10.7.1's NOTE that the algorithm "is not defined as part of PDF". The
//! departure replaces "paint the pixel" with **coverage proportional to area**, and
//! `doc/todo/_scan-conversion.md` has recorded it since the sixteenth session. What it does not
//! license is measuring that area *coarsely enough to reach zero*: an edge covering a tenth of its
//! pixel that paints nothing has not been anti-aliased, it has been dropped, and the painted area
//! is then smaller than the shape's.
//!
//! That is not hypothetical here. `tiny-skia`'s anti-aliased path scan converter supersamples four
//! times per axis, at 0.125, 0.375, 0.625 and 0.875, and an **axis-aligned** edge is seen the same
//! way by all four sub-rows — so the sixteenth of a pixel that is that converter's quantum for a
//! general shape becomes a **quarter** for the commonest shape in every PDF, and anything under an
//! eighth of a pixel becomes nothing. `render-quorra/examples/edge_coverage_ladder` reads it off
//! both backends with no document in the way, and ADR 0474 measured a whole page of it.
//!
//! # The geometry, derived from the clause and not from a renderer
//!
//! §10.7.4 defines the pixel as a **product of two intervals**:
//!
//! > for any point whose real-number coordinates are ( x , y ), let i = floor( x ) and
//! > j = floor( y ). The pixel that contains this point is the one identified as ( i, j ). The
//! > region belonging to that pixel is defined to be the set of points ( x′ , y′ ) such that
//! > i ≤ x′ &lt; i + 1 and j ≤ y′ &lt; j + 1.
//!
//! An axis-aligned rectangle is a product of two intervals too, and the same paragraph gives it
//! the same half-open form — "shapes to be painted by filling … are also treated as half-open
//! regions that include the boundaries along their 'floor' sides, but not along their 'ceiling'
//! sides", which changes no area because a boundary has none. The intersection of two products of
//! intervals is the product of the two intersections, and the area of a product of intervals is
//! the product of their lengths. So
//!
//! ```text
//!   coverage(i, j) = overlap_x(i) · overlap_y(j)
//! ```
//!
//! exactly, for every pixel of every axis-aligned rectangle, at every placement. That is
//! arithmetic out of the clause's own definition rather than a constant anybody tuned, which is
//! what [`rectangle_coverage`] states and what a backend is measured against.
//!
//! # What this module decides, and what it deliberately leaves to the backend
//!
//! It decides the **geometry**: whether a fill is one axis-aligned rectangle on the device's own
//! grid, and where that rectangle is ([`device_rectangle`]). Trap 2's rule is why that lives here
//! rather than in either rasteriser — "where two backends are the oracle, a decision either can
//! make alone is a decision neither has made".
//!
//! It does not decide **how** a backend paints the coverage, because the two have nothing in
//! common to decide: the graphics device already tracks a fraction to a level of 255 and has never
//! had the quantum, and `render-cpu`'s answer is to hand the rectangle to `tiny-skia`'s own
//! rectangle scan converter, which walks the nine pieces the product above implies — one interior
//! run, four edges, four corners — in 8.8 fixed point instead of supersampling a path.
//!
//! # What is declined, and why each
//!
//! - **A path that is not exactly one subpath**, for [`device_rectangle`] — which is the
//!   allocation-free case and nothing more. Several are [`device_rectangles`]', under §11.6.2's
//!   condition rather than §11.3.7.3's, and the paragraph below is that reading.
//! - **A transform that is not axis-preserving.** Under a rotation or a shear the shape is not a
//!   product of two intervals in *device* space, so the closed form above is not its coverage.
//!   [`crate::collapsed`] and [`crate::sub_pixel`] decline the same case for the same reason.
//! - **A rectangle whose device form is not finite, or has no area.** The first names no pixel and
//!   the second is [`crate::collapsed`]'s, which runs before this.
//!
//! # Several rectangles are *one object*, and that is a different clause
//!
//! This module declined a path stating more than one rectangle for four sessions on the ground
//! that "two rectangles drawn as two marks composite by §11.3.7.3's union", which made the case
//! wait on `doc/todo/11` item 5's seam. **§11.6.2 settles it instead, and in the opposite
//! direction** (ADR 0583). Its subject is exactly this — one graphics object described in a way
//! that would seem to cause overlaps — and its rule is a `shall`:
//!
//! > Single graphics objects, as defined in 8.2, "Graphics objects", shall be treated as
//! > elementary objects for transparency compositing purposes … Portions of an object shall not
//! > be composited with one another, even if they are described in a way that would seem to cause
//! > overlaps (such as a self-intersecting path, combined fill and stroke of a path, or a shading
//! > pattern containing an overlap or fold-over).
//!
//! A path's subpaths are portions of one object, so the construction this module was declining is
//! **forbidden** rather than merely a trade against a seam: §11.3.7.3's union is what the standard
//! says to do with *two objects*, and there is one object here. What the clause leaves is the
//! question of where the portions land, and [`device_rectangles`] answers only the half where that
//! question has no compositing in it at all — **rectangles whose device *pixel* footprints are
//! pairwise disjoint**, which is [`share_a_device_pixel`]. There no pixel receives two portions,
//! so drawing them one at a time composites nothing with anything and each is measured by the
//! closed form above. Where two portions do share a pixel, the caller keeps the scan converter it
//! has: one supersampled fill accumulates the whole path in a single conversion, which honours
//! §11.6.2 already and only measures it to a quarter.
//!
//! Two further things fall out and both are stated rather than assumed. **The fill rule stops
//! mattering** — [`device_rectangles`] requires the rectangles' interiors to be pairwise disjoint,
//! so every point lies in at most one of them and the non-zero and even-odd rules select the same
//! set. And **the budget is a cost guard rather than a condition**: [`RECTANGLES_PER_PATH`] bounds
//! the quadratic disjointness test, and a path above it is drawn the way it is drawn today.

use crate::collapsed::subpath_extents;
use crate::geom::{Path, Rect, Transform};
use crate::sub_pixel::is_axis_aligned_rectangle;

/// How many rectangles [`device_rectangles`] will state before it declines — a **cost guard and
/// not a condition**.
///
/// The disjointness tests it bounds are quadratic in this and are run once per fill command per
/// strip the rasteriser cuts the page into, so the guard is what keeps a path of ten thousand
/// rectangles from being asked about ten thousand times. A path above it is drawn by the
/// supersampled path converter, which is what every such path was drawn by before this
/// construction existed.
///
/// Chosen from the population rather than from taste: `pdf-model/examples/rectangular_path_census`
/// prints how many rectangles the corpus's multi-rectangle fills state, and this is above the
/// largest that is not already a drawing of its own.
pub const RECTANGLES_PER_PATH: usize = 32;

/// Which device rectangles a fill covers, where its coverage is a product of two overlaps —
/// ISO 32000-2 §10.7.4 and §11.6.2.
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceRectangles {
    /// One rectangle, which is the common case and allocates nothing.
    One(Rect),
    /// Several, in the path's own order and with pairwise disjoint interiors, so their areas add.
    Several(Vec<Rect>),
}

impl DeviceRectangles {
    /// The rectangles, in the path's own order.
    pub fn iter(&self) -> impl Iterator<Item = Rect> + '_ {
        let (one, several) = match self {
            Self::One(rect) => (Some(*rect), [].as_slice()),
            Self::Several(rects) => (None, rects.as_slice()),
        };
        one.into_iter().chain(several.iter().copied())
    }

    /// Whether two of them fall inside one device pixel — ISO 32000-2 §11.6.2.
    ///
    /// The condition under which portions of one object would be composited with one another if
    /// they were drawn one at a time, which the clause forbids: a pixel that receives two portions
    /// receives two compositing steps, and §11.3.7.3's union of two fractional coverages is less
    /// than the whole of what the object covers there. `false` is the licence to draw each portion
    /// as its own mark.
    ///
    /// A pixel footprint is the half-open pixel range §10.7.4's `floor` identifies, so two
    /// rectangles meeting exactly on a pixel boundary do **not** share one — which is the answer
    /// the clause's own definition gives rather than a margin chosen here.
    #[must_use]
    pub fn share_a_device_pixel(&self) -> bool {
        let Self::Several(rectangles) = self else {
            return false;
        };
        rectangles.iter().enumerate().any(|(index, rect)| {
            rectangles
                .get(index.saturating_add(1)..)
                .unwrap_or_default()
                .iter()
                .any(|other| footprints_meet(*rect, *other))
        })
    }
}

/// The device-space rectangles a fill covers — ISO 32000-2 §10.7.4 and §11.6.2.
///
/// `to_device` maps the path's own space onto the device pixel grid. Returns `None` — the answer
/// for every path this construction cannot speak about, and the one that must cost nothing — where
/// the transform does not preserve axes, where any subpath is not an axis-aligned rectangle with
/// an area, where any device rectangle is not finite, where two of them **overlap**, or where
/// there are more than [`RECTANGLES_PER_PATH`] of them. The module comment argues each.
///
/// The first test is [`Path::narrowest_rectangle`], which is memoised, so a path that is not a
/// rectangle at all is rejected without walking its commands a second time: this is asked once per
/// fill command per strip the rasteriser cuts the page into. **One walk answers both questions**,
/// which is why the single-rectangle case is a variant here rather than a second entry point that
/// would walk the path again — measured, a second walk per declining fill is +0.18% of the
/// rasteriser on a page of text, against nothing measurable for the variant.
#[must_use]
pub fn device_rectangles(path: &Path, to_device: Transform) -> Option<DeviceRectangles> {
    // §10.7.4's pixels are the device's, and a rectangle is only a product of two device intervals
    // where a device axis is a path axis.
    if !to_device.preserves_axes() {
        return None;
    }
    // No subpath of this path is an axis-aligned rectangle, so none of it can be one.
    path.narrowest_rectangle()?;
    let commands = path.commands();
    // The first rectangle is held on its own so that the commonest shape in every PDF allocates
    // nothing; the `Vec` begins the moment a second one arrives.
    let mut one: Option<Rect> = None;
    let mut several: Vec<Rect> = Vec::new();
    for extent in subpath_extents(path) {
        if !is_axis_aligned_rectangle(&commands[extent.range()], extent.min, extent.max) {
            return None;
        }
        let a = to_device.apply(extent.min);
        let b = to_device.apply(extent.max);
        if !(a.x.is_finite() && a.y.is_finite() && b.x.is_finite() && b.y.is_finite()) {
            return None;
        }
        let rect = Rect::from_corners(a, b);
        if !(rect.width() > 0.0 && rect.height() > 0.0) {
            return None;
        }
        // Overlapping portions are the case §11.6.2's own sentence is about — "described in a way
        // that would seem to cause overlaps" — and the two fill rules answer them differently, so
        // the decomposition would have to carry a winding number. Declining leaves such a path to
        // the one scan conversion that already resolves it.
        match one.take() {
            None if several.is_empty() => {
                one = Some(rect);
                continue;
            }
            None => {}
            Some(first) => several.push(first),
        }
        if several.len() >= RECTANGLES_PER_PATH {
            return None;
        }
        if several.iter().any(|other| overlap_in_area(*other, rect)) {
            return None;
        }
        several.push(rect);
    }
    match (one, several.is_empty()) {
        (Some(rect), true) => Some(DeviceRectangles::One(rect)),
        (_, true) => None,
        _ => Some(DeviceRectangles::Several(several)),
    }
}

/// The device-space rectangle a fill covers, where the fill is exactly one of them.
///
/// [`device_rectangles`] with its other variant declined, which is what a caller wants when it has
/// a shape it already believes is a single rectangle — a stroked outline, a test's own fixture.
#[must_use]
pub fn device_rectangle(path: &Path, to_device: Transform) -> Option<Rect> {
    match device_rectangles(path, to_device)? {
        DeviceRectangles::One(rect) => Some(rect),
        DeviceRectangles::Several(_) => None,
    }
}

/// Whether two rectangles enclose a common area, which is a positive overlap on both axes.
fn overlap_in_area(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// Whether the device pixels two rectangles reach have one in common.
fn footprints_meet(a: Rect, b: Rect) -> bool {
    overlap_in_area(footprint(a), footprint(b))
}

/// The whole device pixels a rectangle reaches, as a rectangle on the pixel grid.
fn footprint(rect: Rect) -> Rect {
    Rect::from_corners(
        crate::geom::Point::new(rect.min.x.floor(), rect.min.y.floor()),
        crate::geom::Point::new(rect.max.x.ceil(), rect.max.y.ceil()),
    )
}

/// The area of `rect` inside device pixel `(i, j)`, in `0.0..=1.0` — ISO 32000-2 §10.7.4.
///
/// The module comment derives it: the clause's pixel is `[i, i+1) × [j, j+1)`, an axis-aligned
/// rectangle is a product of two intervals, and the area of the intersection of two such products
/// is the product of the two one-dimensional overlaps. Under this tree's anti-aliasing departure
/// that area *is* the coverage the pixel is painted at, so this is what either backend's raster is
/// compared against.
///
/// `rect` is in device space, as [`device_rectangle`] returns it.
#[must_use]
pub fn rectangle_coverage(rect: Rect, i: f32, j: f32) -> f32 {
    overlap(rect.min.x, rect.max.x, i) * overlap(rect.min.y, rect.max.y, j)
}

/// The length of `[lo, hi)` inside the unit interval starting at `at`.
fn overlap(lo: f32, hi: f32, at: f32) -> f32 {
    (hi.min(at + 1.0) - lo.max(at)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{
        DeviceRectangles, RECTANGLES_PER_PATH, device_rectangle, device_rectangles,
        rectangle_coverage,
    };
    use crate::geom::{Path, PathCommand, Point, Rect, Transform};

    fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        path
    }

    #[test]
    fn a_rectangle_is_carried_onto_the_device_grid() {
        let path = rectangle(1.0, 2.0, 5.0, 7.0);
        let at = Transform::new(2.0, 0.0, 0.0, -2.0, 0.5, 20.0);
        let rect = device_rectangle(&path, at).expect("one axis-aligned rectangle");
        assert!((rect.min.x - 2.5).abs() < 1e-5, "{rect:?}");
        assert!((rect.max.x - 10.5).abs() < 1e-5, "{rect:?}");
        assert!((rect.min.y - 6.0).abs() < 1e-5, "{rect:?}");
        assert!((rect.max.y - 16.0).abs() < 1e-5, "{rect:?}");
    }

    /// [`device_rectangle`] is the one-subpath case and allocates nothing; several subpaths are
    /// [`device_rectangles`]', which the tests below are about.
    #[test]
    fn a_second_subpath_is_the_other_functions() {
        let mut path = rectangle(1.0, 1.0, 3.0, 3.0);
        path.extend(rectangle(5.0, 1.0, 7.0, 3.0).commands());
        assert_eq!(device_rectangle(&path, Transform::IDENTITY), None);
        let several = device_rectangles(&path, Transform::IDENTITY).expect("two rectangles");
        assert_eq!(several.iter().count(), 2);
    }

    /// The two entry points state one geometry, which is what lets a backend take the cheap one
    /// first and fall to the other without the two disagreeing about the same path.
    #[test]
    fn one_rectangle_is_the_same_answer_either_way() {
        let path = rectangle(1.0, 2.0, 5.0, 7.0);
        let at = Transform::new(2.0, 0.0, 0.0, -2.0, 0.5, 20.0);
        let one = device_rectangle(&path, at).expect("one axis-aligned rectangle");
        let several = device_rectangles(&path, at).expect("the same rectangle");
        assert_eq!(several, DeviceRectangles::One(one));
    }

    /// §11.6.2's own sentence names the overlapping case, and the two fill rules answer it
    /// differently, so the decomposition declines rather than choosing one.
    #[test]
    fn overlapping_rectangles_decline_because_the_fill_rule_would_decide_them() {
        let mut path = rectangle(1.0, 1.0, 4.0, 4.0);
        path.extend(rectangle(3.0, 3.0, 6.0, 6.0).commands());
        assert_eq!(device_rectangles(&path, Transform::IDENTITY), None);
    }

    /// Two portions in one device pixel are two compositing steps, which §11.6.2 forbids; two
    /// portions in different pixels are not, whether or not they touch.
    #[test]
    fn sharing_a_device_pixel_is_decided_by_the_pixel_grid_and_not_by_touching() {
        let apart = DeviceRectangles::Several(vec![
            Rect::from_corners(Point::new(1.0, 1.0), Point::new(3.0, 3.0)),
            Rect::from_corners(Point::new(3.0, 1.0), Point::new(5.0, 3.0)),
        ]);
        assert!(
            !apart.share_a_device_pixel(),
            "a boundary on a pixel line divides the pixels between them"
        );
        let abutting = DeviceRectangles::Several(vec![
            Rect::from_corners(Point::new(1.0, 1.0), Point::new(3.5, 3.0)),
            Rect::from_corners(Point::new(3.5, 1.0), Point::new(5.0, 3.0)),
        ]);
        assert!(
            abutting.share_a_device_pixel(),
            "a boundary inside a pixel puts both portions in it"
        );
    }

    /// The budget is a cost guard: above it the answer is `None` and the path is drawn the way it
    /// was before this construction existed.
    #[test]
    fn a_path_above_the_budget_declines() {
        let mut path = Path::new();
        for index in 0..=RECTANGLES_PER_PATH {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a loop counter far inside f32's exact integers"
            )]
            let x = index as f32 * 4.0;
            path.extend(rectangle(x, 0.0, x + 2.0, 2.0).commands());
        }
        assert_eq!(device_rectangles(&path, Transform::IDENTITY), None);
    }

    #[test]
    fn a_shear_declines_because_the_shape_is_no_longer_a_product_of_two_intervals() {
        let path = rectangle(1.0, 1.0, 3.0, 3.0);
        let shear = Transform::new(1.0, 0.0, 0.4, 1.0, 0.0, 0.0);
        assert_eq!(device_rectangle(&path, shear), None);
    }

    #[test]
    fn a_quarter_turn_keeps_the_rectangle_because_it_exchanges_the_axes() {
        let path = rectangle(1.0, 2.0, 5.0, 7.0);
        let turn = Transform::new(0.0, 1.0, -1.0, 0.0, 10.0, 0.0);
        let rect = device_rectangle(&path, turn).expect("a rectangle after a quarter turn");
        assert!((rect.width() - 5.0).abs() < 1e-5, "{rect:?}");
        assert!((rect.height() - 4.0).abs() < 1e-5, "{rect:?}");
    }

    #[test]
    fn a_triangle_is_not_a_rectangle() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(4.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(0.0, 4.0)));
        path.push(PathCommand::Close);
        assert_eq!(device_rectangle(&path, Transform::IDENTITY), None);
    }

    /// The closed form the module comment derives, at the four kinds of pixel a rectangle has.
    #[test]
    fn coverage_is_the_product_of_the_two_overlaps() {
        let rect = Rect::from_corners(Point::new(2.25, 3.5), Point::new(6.75, 9.125));
        // Interior: both overlaps whole.
        assert!((rectangle_coverage(rect, 4.0, 5.0) - 1.0).abs() < 1e-6);
        // A vertical edge: the x overlap alone.
        assert!((rectangle_coverage(rect, 2.0, 5.0) - 0.75).abs() < 1e-6);
        // A horizontal edge: the y overlap alone.
        assert!((rectangle_coverage(rect, 4.0, 9.0) - 0.125).abs() < 1e-6);
        // A corner: the product of the two.
        assert!((rectangle_coverage(rect, 6.0, 3.0) - 0.75 * 0.5).abs() < 1e-6);
        // Outside: nothing.
        assert!(rectangle_coverage(rect, 7.0, 5.0).abs() < 1e-6);
    }
}
