//! The one piece of arithmetic every host does on a quadrilateral the viewer answers with.
//!
//! `viewer-core` answers in quadrilaterals — [`viewer_core::Selected::quads`],
//! [`viewer_core::Answer::Focus`], [`viewer_core::FormWidget::quad`],
//! [`viewer_core::PopupWindow::quad`] — because §7.7.3.3's `/Rotate` and Table 192's `/R` can each
//! turn a rectangle on the page and four corners say so where two do not. A *widget* is upright in
//! every toolkit there is, so a host that places one loses the rotation, and this is where it does.
//!
//! It is here because it was written twice. `viewer-gtk` and `viewer-qt` each carried a private
//! `bounds` with the same body and the same doc comment, which is this crate's own test — the
//! third copy of a function is where two hosts stop agreeing — and §12.5.6.14's popup window would
//! have been the fifth and sixth call.

/// The axis-aligned bound of a quadrilateral, in the device pixels it arrived in.
///
/// `(left, top, width, height)`. Said rather than hidden: a rotated widget gets an upright
/// control, and since ADR 0245 takes the widget appearances out from under a delegated form there
/// is nothing rotated left on the page for it to disagree with.
///
/// A quadrilateral whose corners coincide gives a zero width or height, which is not an error
/// here — it is the caller that decides whether a control with no area is placed, and
/// [`crate::popup::windows`] is one that decides it does not.
#[must_use]
pub fn bounds(quad: [f32; 8]) -> (f32, f32, f32, f32) {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    let left = xs.iter().copied().fold(f32::INFINITY, f32::min);
    let right = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let top = ys.iter().copied().fold(f32::INFINITY, f32::min);
    let bottom = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    (left, top, right - left, bottom - top)
}

/// Whether a quadrilateral the viewer answered with covers a point of the same viewport.
///
/// The **bounding box** of the four corners, and for the shape this is asked about that is exact
/// rather than an approximation: [`viewer_core::FormWidget::quad`] is built out of §12.5.2's
/// `/Rect`, which "shall be two opposite corners" of an upright rectangle, through §7.7.3.3's
/// `/Rotate` — and that entry's value "shall be a multiple of 90", so the rectangle is still
/// upright on the screen. A quadrilateral this test would get wrong is one no page can state.
///
/// Here rather than in a host because [`crate::form::clicked`] needs it and `viewer-ui` had
/// written it, which is this module's own rule one function later: the third copy is where two
/// hosts stop agreeing about which widget a press landed on.
#[must_use]
pub fn covers(quad: [f32; 8], (x, y): (f32, f32)) -> bool {
    let (left, top, width, height) = bounds(quad);
    (left..=left + width).contains(&x) && (top..=top + height).contains(&y)
}

#[cfg(test)]
mod tests {
    use super::{bounds, covers};

    /// A point inside the upright box the corners span, and one outside it.
    #[test]
    fn a_widget_covers_the_points_of_its_own_rectangle() {
        let quad = [10.0, 20.0, 40.0, 20.0, 40.0, 50.0, 10.0, 50.0];
        assert!(covers(quad, (25.0, 35.0)));
        // The edges belong to the widget: a `/Rect`'s corner is inside the rectangle it states.
        assert!(covers(quad, (10.0, 20.0)));
        assert!(covers(quad, (40.0, 50.0)));
        assert!(!covers(quad, (9.0, 35.0)));
        assert!(!covers(quad, (25.0, 51.0)));
    }

    #[test]
    fn an_upright_rectangle_is_its_own_bound() {
        let (x, y, width, height) = bounds([10.0, 20.0, 40.0, 20.0, 40.0, 50.0, 10.0, 50.0]);
        assert!((x - 10.0).abs() < f32::EPSILON);
        assert!((y - 20.0).abs() < f32::EPSILON);
        assert!((width - 30.0).abs() < f32::EPSILON);
        assert!((height - 30.0).abs() < f32::EPSILON);
    }

    /// §7.7.3.3's `/Rotate 90` turns the corners; the bound is what a widget can occupy.
    #[test]
    fn a_turned_rectangle_gives_the_upright_box_that_holds_it() {
        let (x, y, width, height) = bounds([40.0, 20.0, 40.0, 50.0, 10.0, 50.0, 10.0, 20.0]);
        assert!((x - 10.0).abs() < f32::EPSILON);
        assert!((y - 20.0).abs() < f32::EPSILON);
        assert!((width - 30.0).abs() < f32::EPSILON);
        assert!((height - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_collapsed_quadrilateral_has_no_area() {
        let (_, _, width, height) = bounds([5.0; 8]);
        assert!(width.abs() < f32::EPSILON);
        assert!(height.abs() < f32::EPSILON);
    }
}
