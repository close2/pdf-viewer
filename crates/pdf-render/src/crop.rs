//! Narrowing a rectangular fill to the part of it a mask can admit.
//!
//! This is an optimisation with a proof rather than a rule the standard states, and both
//! halves matter. What it is *for* is `sh`, whose Table 76 in ISO 32000-2 §8.7.4.2 says
//!
//! > Paint the shape and colour shading described by a shading dictionary, subject to the
//! > current clipping path.
//!
//! and names no other bound, so a display list states such a command as a fill of the whole
//! page under whatever clip was in force. The same table then says what that costs — an
//! unbounded shading "paints the shading's gradient fill across the entire clipping region,
//! which may be time-consuming" — and this is the sentence answered.
//!
//! A producer that draws an illustration as thousands of small gradient patches —
//! `bug1721218_reduced.pdf` states **3490** of them — therefore hands a rasteriser three
//! thousand page-sized rectangles, each masked down to a few dozen pixels, and a rasteriser
//! evaluates its shader over the *path's* spans and multiplies the mask in afterwards. The
//! pixels the mask rejects are paid for at full price: on that page 10.4 M shaded pixels per
//! render produce 85 608 that survive, a ratio of 122, and cropping takes twenty renders of it
//! from **38.45 G instructions to 20.03 G** with the ink unchanged to the byte (ADR 0236).
//!
//! # Why the same pixels come out
//!
//! Replacing an axis-aligned rectangle with a smaller axis-aligned rectangle changes exactly
//! two things: pixels between the two rectangles lose their coverage, and pixels straddling a
//! *new* edge gain a partial one where they had a whole one. Both are confined to the region
//! outside the retained rectangle's interior, so a caller that passes an `admits` rectangle the
//! mask is provably zero outside of gets the same picture — every pixel whose coverage changed
//! is multiplied by zero.
//!
//! The three properties this relies on are the caller's to supply and are stated on
//! [`cropped_rectangle`]. It does its part in return: a side that is **not** narrowed keeps the
//! caller's own `f32`, because `f32::max` of a coordinate already inside the room returns that
//! coordinate and not a rounded copy of it. So an unnarrowed edge is not merely close to where
//! it was, it is the same number, and the rasteriser's arithmetic on it is the same arithmetic.

use crate::collapsed::subpath_extents;
use crate::geom::{Path, PathCommand, Point, Rect, Transform};
use crate::sub_pixel::is_axis_aligned_rectangle;

/// The rectangle `path` should be filled as instead, given that only `admits` can be marked.
///
/// `to_device` maps the path's own space onto the device grid `admits` is stated on.
/// `None` means draw what you had, which covers every case this cannot improve: a path that is
/// not one axis-aligned rectangle, a transform that would not keep it one, and a rectangle
/// already inside `admits`.
///
/// # What the caller must guarantee
///
/// - **The mask is zero outside `admits`.** That is what makes the crop invisible; a caller
///   whose mask reaches past it would lose the pixels between.
/// - **`admits` has a pixel of margin**, because a mask's support is computed from one
///   composition of transforms and drawn through another, and the two agree to within rounding
///   rather than exactly. `render-cpu`'s construction — outset by one, then rounded outward,
///   which is what its band already does to the rows — is the one this was written against.
/// - **`admits` is at least two device pixels across**, which the outset above provides. It is
///   what keeps the result clear of §10.7.4's sub-pixel rule: a crop that produced a rectangle
///   thinner than a pixel would be drawn by a different path through the backend and would not
///   be the same picture.
#[must_use]
pub fn cropped_rectangle(path: &Path, to_device: Transform, admits: Rect) -> Option<Path> {
    // Cheapest first: the hull is memoised on the path, so a fill whose clip does not narrow it
    // costs four multiplications and four comparisons and never walks the commands.
    let hull = path.hull()?;
    if !to_device.preserves_axes() || admits.contains(hull.mapped(to_device)) {
        return None;
    }
    let (min, max) = whole_rectangle(path)?;

    // Back into the path's own space, where the corners that survive are the caller's own.
    // Mapping the rectangle *forward* and cropping there would restate every corner.
    let room = admits.mapped(to_device.invert()?);
    let cropped = Rect {
        min: Point::new(min.x.max(room.min.x), min.y.max(room.min.y)),
        max: Point::new(max.x.min(room.max.x), max.y.min(room.max.y)),
    };
    if !(cropped.min.x < cropped.max.x && cropped.min.y < cropped.max.y) {
        // They do not meet. The mask admits nothing of this rectangle, which the caller could
        // act on, but saying so is a second meaning for `None` and the caller has its own
        // emptiness test; drawing the uncropped rectangle through a mask that is zero
        // everywhere over it is the same page.
        return None;
    }

    let mut narrowed = Path::new();
    narrowed.push(PathCommand::MoveTo(cropped.min));
    narrowed.push(PathCommand::LineTo(Point::new(
        cropped.max.x,
        cropped.min.y,
    )));
    narrowed.push(PathCommand::LineTo(cropped.max));
    narrowed.push(PathCommand::LineTo(Point::new(
        cropped.min.x,
        cropped.max.y,
    )));
    narrowed.push(PathCommand::Close);
    Some(narrowed)
}

/// The corners of the rectangle a path *is*, where it is one axis-aligned rectangle and
/// nothing else.
///
/// Stricter than [`Path::narrowest_rectangle`], which asks whether *some* subpath is one: a
/// path holding a rectangle and anything else is not a path that may be replaced by a
/// rectangle.
///
/// The corners are in the path's own space; whether the *device* sees a rectangle is the
/// caller's second question and [`Transform::preserves_axes`] is what answers it — which is
/// why the two are apart. It is public because a census asks exactly this question of a
/// corpus (`render-quorra/examples/rect_and_residue_census.rs`), and a census that carried its
/// own copy of the predicate would be measuring a second implementation of it.
#[must_use]
pub fn whole_rectangle(path: &Path) -> Option<(Point, Point)> {
    let commands = path.commands();
    let mut only: Option<(Point, Point)> = None;
    for extent in subpath_extents(path) {
        if only.is_some()
            || !is_axis_aligned_rectangle(&commands[extent.range()], extent.min, extent.max)
        {
            return None;
        }
        only = Some((extent.min, extent.max));
    }
    only
}

#[cfg(test)]
mod tests {
    use super::cropped_rectangle;
    use crate::geom::{Path, PathCommand, Point, Rect, Transform};

    /// The rectangle `re x y w h` writes.
    fn rectangle(min: Point, max: Point) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(min));
        path.push(PathCommand::LineTo(Point::new(max.x, min.y)));
        path.push(PathCommand::LineTo(max));
        path.push(PathCommand::LineTo(Point::new(min.x, max.y)));
        path.push(PathCommand::Close);
        path
    }

    #[test]
    fn a_page_sized_rectangle_is_cropped_to_the_room_the_mask_leaves() {
        let page = rectangle(Point::new(0.0, 0.0), Point::new(612.0, 792.0));
        let admits = Rect::from_corners(Point::new(100.0, 200.0), Point::new(110.0, 220.0));
        let cropped = cropped_rectangle(&page, Transform::IDENTITY, admits).expect("cropped");
        assert_eq!(cropped.hull(), Some(admits));
    }

    /// The property the exactness argument rests on: a side already inside the room is the
    /// caller's own number, not a restatement of it.
    ///
    /// Compared by bits throughout, because "the same number" is what is being asserted and
    /// `==` on a float would pass for a value merely near it.
    #[test]
    fn a_side_the_mask_does_not_narrow_keeps_its_own_bits() {
        // A tenth is not representable, so this is the nearest `f32` to it and not a decimal
        // any restatement of the coordinate would land on by accident.
        let odd = 0.1_f32;
        let source = rectangle(Point::new(odd, odd), Point::new(612.0, 792.0));
        let admits = Rect::from_corners(Point::new(-5.0, -5.0), Point::new(300.0, 400.0));
        let cropped = cropped_rectangle(&source, Transform::IDENTITY, admits).expect("cropped");
        let hull = cropped.hull().expect("a rectangle has a hull");
        assert_eq!(hull.min.x.to_bits(), odd.to_bits());
        assert_eq!(hull.min.y.to_bits(), odd.to_bits());
        assert_eq!(hull.max.x.to_bits(), 300.0_f32.to_bits());
        assert_eq!(hull.max.y.to_bits(), 400.0_f32.to_bits());
    }

    #[test]
    fn a_rectangle_the_mask_already_contains_is_left_alone() {
        let small = rectangle(Point::new(10.0, 10.0), Point::new(20.0, 20.0));
        let admits = Rect::from_corners(Point::new(0.0, 0.0), Point::new(612.0, 792.0));
        assert!(cropped_rectangle(&small, Transform::IDENTITY, admits).is_none());
    }

    #[test]
    fn a_shape_that_is_not_one_rectangle_is_left_alone() {
        let mut two = rectangle(Point::new(0.0, 0.0), Point::new(612.0, 792.0));
        two.extend(rectangle(Point::new(1.0, 1.0), Point::new(2.0, 2.0)).commands());
        let admits = Rect::from_corners(Point::new(100.0, 100.0), Point::new(110.0, 110.0));
        assert!(cropped_rectangle(&two, Transform::IDENTITY, admits).is_none());

        let mut curved = Path::new();
        curved.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        curved.push(PathCommand::CurveTo(
            Point::new(612.0, 0.0),
            Point::new(612.0, 792.0),
            Point::new(0.0, 792.0),
        ));
        curved.push(PathCommand::Close);
        assert!(cropped_rectangle(&curved, Transform::IDENTITY, admits).is_none());
    }

    /// A shear takes a rectangle to a parallelogram, and cropping the parallelogram's *bound*
    /// would cut the shape itself.
    #[test]
    fn a_transform_that_would_not_keep_it_a_rectangle_is_left_alone() {
        let page = rectangle(Point::new(0.0, 0.0), Point::new(612.0, 792.0));
        let sheared = Transform::new(1.0, 0.3, 0.0, 1.0, 0.0, 0.0);
        let admits = Rect::from_corners(Point::new(100.0, 100.0), Point::new(110.0, 110.0));
        assert!(cropped_rectangle(&page, sheared, admits).is_none());
    }

    /// The device grid is not the path's grid, and the crop has to arrive in the path's.
    #[test]
    fn the_room_is_stated_on_the_device_grid() {
        let page = rectangle(Point::new(0.0, 0.0), Point::new(612.0, 792.0));
        // Two device pixels per page unit, y flipped, as a target spec builds it.
        let to_device = Transform::new(2.0, 0.0, 0.0, -2.0, 0.0, 1584.0);
        let admits = Rect::from_corners(Point::new(200.0, 400.0), Point::new(220.0, 440.0));
        let cropped = cropped_rectangle(&page, to_device, admits).expect("cropped");
        let hull = cropped.hull().expect("a rectangle has a hull");
        assert_eq!(hull.min.x.to_bits(), 100.0_f32.to_bits());
        assert_eq!(hull.max.x.to_bits(), 110.0_f32.to_bits());
        assert_eq!(hull.min.y.to_bits(), 572.0_f32.to_bits());
        assert_eq!(hull.max.y.to_bits(), 592.0_f32.to_bits());
    }

    /// A rectangle the mask meets nowhere is left alone rather than cropped to nothing: an
    /// empty path is a second meaning for the return value, and the caller has its own test.
    #[test]
    fn a_rectangle_the_room_misses_entirely_is_left_alone() {
        let page = rectangle(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let admits = Rect::from_corners(Point::new(200.0, 200.0), Point::new(210.0, 210.0));
        assert!(cropped_rectangle(&page, Transform::IDENTITY, admits).is_none());
    }
}
