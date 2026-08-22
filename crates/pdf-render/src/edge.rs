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
//! - **A path that is not exactly one subpath.** Two rectangles drawn as two marks composite by
//!   §11.3.7.3's union where one scan conversion would have accumulated them, so a path stating
//!   several would come out light along any seam. That is `doc/todo/11` item 5's subject and this
//!   module must not enlarge it. One subpath also settles the fill rule: the two agree on it.
//! - **A transform that is not axis-preserving.** Under a rotation or a shear the shape is not a
//!   product of two intervals in *device* space, so the closed form above is not its coverage.
//!   [`crate::collapsed`] and [`crate::sub_pixel`] decline the same case for the same reason.
//! - **A rectangle whose device form is not finite, or has no area.** The first names no pixel and
//!   the second is [`crate::collapsed`]'s, which runs before this.

use crate::collapsed::subpath_extents;
use crate::geom::{Path, Rect, Transform};
use crate::sub_pixel::is_axis_aligned_rectangle;

/// The device-space rectangle a fill covers, where its coverage is a product of two overlaps —
/// ISO 32000-2 §10.7.4.
///
/// `to_device` maps the path's own space onto the device pixel grid. Returns `None` — the answer
/// for every path this construction cannot speak about, and the one that must cost nothing — where
/// the fill is not a single axis-aligned rectangle, where the transform would turn it into
/// something that is not one, or where the result names no region of the device. The module
/// comment argues each.
///
/// The first test is [`Path::narrowest_rectangle`], which is memoised, so a path that is not a
/// rectangle at all is rejected without walking its commands a second time: this is asked once per
/// fill command per strip the rasteriser cuts the page into.
#[must_use]
pub fn device_rectangle(path: &Path, to_device: Transform) -> Option<Rect> {
    // §10.7.4's pixels are the device's, and a rectangle is only a product of two device intervals
    // where a device axis is a path axis.
    if !to_device.preserves_axes() {
        return None;
    }
    // No subpath of this path is an axis-aligned rectangle, so none of it can be the one.
    path.narrowest_rectangle()?;
    let commands = path.commands();
    let mut extents = subpath_extents(path);
    let extent = extents.next()?;
    if extents.next().is_some() {
        return None;
    }
    if !is_axis_aligned_rectangle(&commands[extent.range()], extent.min, extent.max) {
        return None;
    }
    let a = to_device.apply(extent.min);
    let b = to_device.apply(extent.max);
    if !(a.x.is_finite() && a.y.is_finite() && b.x.is_finite() && b.y.is_finite()) {
        return None;
    }
    let rect = Rect::from_corners(a, b);
    (rect.width() > 0.0 && rect.height() > 0.0).then_some(rect)
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
    use super::{device_rectangle, rectangle_coverage};
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

    #[test]
    fn a_second_subpath_declines_because_two_marks_would_composite() {
        let mut path = rectangle(1.0, 1.0, 3.0, 3.0);
        path.extend(rectangle(5.0, 1.0, 7.0, 3.0).commands());
        assert_eq!(device_rectangle(&path, Transform::IDENTITY), None);
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
