//! The one place a path reaches `tiny-skia`'s scan converter, and the range that bounds it.
//!
//! # Why this module exists
//!
//! ISO 32000-2 §10.7 leaves scan conversion to the device and states no bound on a
//! coordinate, and §7.3.3 hands the range of a number to the implementation outright:
//!
//! > The range and precision of numbers may be limited by the internal representations used in
//! > the computer on which the PDF processor is running; Annex C, "Advice on maximising
//! > portability", gives these limits for typical implementations.
//!
//! That annex is informative and gives no figure for a coordinate — its entry for real numbers
//! says only that computers "often" use IEEE 754. So the magnitude a page may state is this
//! processor's to decide, and a page whose content stream was damaged in transit states one
//! nobody decided.
//!
//! `tiny-skia`'s scan converter does its arithmetic in 16.16 fixed point, and its
//! anti-aliased path supersamples by four before it gets there. The library says the
//! consequence itself, in the comment above `DrawTiler`: its fixed-point types are limited by
//! 8192 and 32768, which means that it cannot render a path larger than 8192 onto a pixmap —
//! and again beside the constant that enforces it, 8K being one too big because `8K << 2` is
//! 32768 and too big for `Fixed`. What it bounds by that number is the **pixmap**, which it
//! tiles; what it does not bound is the **path**, whose coordinates run through the same
//! arithmetic. `SuperBlitter::blit_h` carries a comment admitting as much — a hack, it says,
//! until somebody works out why the cubics go beyond the bounds — and handles only the left
//! side of the overrun.
//!
//! A path that leaves the range therefore produces coverage the arithmetic does not define,
//! and on the right geometry it walks `AlphaRuns`' run buffer past its end and unwraps a
//! `None`. That is a **panic in a dependency reachable from a document**, and under
//! `[profile.release]`'s `panic = "abort"` it is the whole process: the four-hundred-and-
//! thirty-third session met it on two of 65 944 crawled documents (ADR 0269).
//!
//! # What is done about it, and why it is not a refusal
//!
//! The **non**-anti-aliased scan converter takes the same geometry without complaint, which
//! is checkable and was checked: over the reduction in ADR 0269, magnitudes from 10³ to 10³⁰
//! all return, and where the anti-aliased converter also returns the two agree to within the
//! anti-aliasing of the edges (13 250 820 against 13 252 311 of ink, 0.011%). `tiny-skia`
//! makes the same substitution one branch over for its own overflow test — `path_aa::fill_path`
//! calls `path::fill_path` when the clipped bounds would overflow the shift — so this is the
//! library's own remedy applied where its test does not reach.
//!
//! So a path outside the range is **drawn without anti-aliasing** rather than refused. A
//! refusal would cost the page: `CpuRasterError` stops the whole raster, so one damaged path
//! would take every mark after it, and this backend is the correctness oracle. What the
//! substitution costs is at most half a pixel of edge quality on a shape whose own extent is
//! thousands of pages across.

/// The largest device coordinate the anti-aliased scan converter's arithmetic can express.
///
/// `tiny-skia`'s own number, for its own stated reason: supersampling shifts a coordinate left
/// by two before it becomes 16.16 fixed point, and `8192 << 2` is 32768, which its `Fixed` type
/// cannot hold. The library applies it to the pixmap it draws into; this module applies it to
/// the geometry, which is the half it leaves open.
const SUPERSAMPLED_LIMIT: f32 = 8191.0;

/// Whether `bounds` lies inside [`SUPERSAMPLED_LIMIT`] on both axes.
fn within(bounds: Option<tiny_skia::Rect>) -> bool {
    // `None` is a rectangle the transform could not produce — a coordinate that overflowed or
    // is not finite — which is exactly the case this is here to keep out.
    bounds.is_some_and(|bounds| {
        bounds.left() >= -SUPERSAMPLED_LIMIT
            && bounds.top() >= -SUPERSAMPLED_LIMIT
            && bounds.right() <= SUPERSAMPLED_LIMIT
            && bounds.bottom() <= SUPERSAMPLED_LIMIT
    })
}

/// Whether `path` drawn under `at` stays inside the range, grown by `outset` first.
///
/// The outset is in the **path's** own space, which is where a stroke's width is stated, so it
/// is applied before `at` rather than after: a caller with a transform does not have to take a
/// singular value of it to say how far a stroke reaches. A fill passes zero.
fn expressible(path: &tiny_skia::Path, at: tiny_skia::Transform, outset: f32) -> bool {
    let reach = if outset.is_finite() {
        outset.max(0.0)
    } else {
        f32::MAX
    };
    within(
        path.bounds()
            .outset(reach, reach)
            .and_then(|bounds| bounds.transform(at)),
    )
}

/// Anti-aliasing, kept only where the scan converter's arithmetic reaches the geometry.
fn keep_anti_alias(anti_alias: bool, expressible: bool) -> bool {
    anti_alias && expressible
}

/// [`tiny_skia::PixmapMut::fill_path`], with the range applied to `paint.anti_alias`.
pub(crate) fn fill(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    paint: &tiny_skia::Paint<'_>,
    fill_rule: tiny_skia::FillRule,
    at: tiny_skia::Transform,
    clip: Option<&tiny_skia::Mask>,
) {
    let mut paint = paint.clone();
    paint.anti_alias = keep_anti_alias(paint.anti_alias, expressible(path, at, 0.0));
    pixmap.fill_path(path, &paint, fill_rule, at, clip);
}

/// [`tiny_skia::PixmapMut::stroke_path`], with the range applied to `paint.anti_alias`.
///
/// The outset is a whole width times the miter limit rather than the half-width the stroke
/// actually reaches: a miter join extends past the outline by the limit, and over-estimating
/// here costs a path within a hair of the bound its anti-aliasing and nothing else.
pub(crate) fn stroke(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    paint: &tiny_skia::Paint<'_>,
    style: &tiny_skia::Stroke,
    at: tiny_skia::Transform,
    clip: Option<&tiny_skia::Mask>,
) {
    let mut paint = paint.clone();
    let outset = style.width * style.miter_limit.max(1.0);
    paint.anti_alias = keep_anti_alias(paint.anti_alias, expressible(path, at, outset));
    pixmap.stroke_path(path, &paint, style, at, clip);
}

/// [`tiny_skia::Mask::fill_path`], with the range applied to `anti_alias`.
pub(crate) fn mask_fill(
    mask: &mut tiny_skia::Mask,
    path: &tiny_skia::Path,
    fill_rule: tiny_skia::FillRule,
    anti_alias: bool,
    at: tiny_skia::Transform,
) {
    let anti_alias = keep_anti_alias(anti_alias, expressible(path, at, 0.0));
    mask.fill_path(path, fill_rule, anti_alias, at);
}

/// [`tiny_skia::Mask::intersect_path`], with the range applied to `anti_alias`.
pub(crate) fn mask_intersect(
    mask: &mut tiny_skia::Mask,
    path: &tiny_skia::Path,
    fill_rule: tiny_skia::FillRule,
    anti_alias: bool,
    at: tiny_skia::Transform,
) {
    let anti_alias = keep_anti_alias(anti_alias, expressible(path, at, 0.0));
    mask.intersect_path(path, fill_rule, anti_alias, at);
}

#[cfg(test)]
mod tests {
    use super::{SUPERSAMPLED_LIMIT, expressible};

    /// A triangle running from the page out to `reach`, which is what a damaged content
    /// stream states and what ADR 0269's two witnesses state.
    fn spike(reach: f32) -> tiny_skia::Path {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(10.0, 10.0);
        builder.line_to(reach, reach * 200.0);
        builder.line_to(20.0, 10.0);
        builder.close();
        builder
            .finish()
            .expect("three lines and a close form a path")
    }

    #[test]
    fn a_path_inside_the_range_keeps_its_anti_aliasing() {
        assert!(expressible(
            &spike(4.0),
            tiny_skia::Transform::identity(),
            0.0
        ));
    }

    #[test]
    fn a_path_outside_the_range_loses_it() {
        assert!(!expressible(
            &spike(SUPERSAMPLED_LIMIT + 1.0),
            tiny_skia::Transform::identity(),
            0.0
        ));
    }

    /// The bound is on the *device* coordinates, so a transform that brings the geometry back
    /// inside the range is a path that keeps its anti-aliasing.
    #[test]
    fn the_transform_is_what_the_range_is_read_after() {
        let path = spike(1.0e6);
        let at = tiny_skia::Transform::identity();
        assert!(!expressible(&path, at, 0.0));
        assert!(expressible(
            &path,
            tiny_skia::Transform::from_scale(1.0e-6, 1.0e-8),
            0.0
        ));
    }

    /// tiny-skia 0.12.0 aborts the process on this geometry with anti-aliasing on, and
    /// returns without it. Both halves are asserted, because the second is what makes the
    /// substitution a remedy rather than a silence.
    #[test]
    fn the_witness_geometry_is_drawn_without_anti_aliasing() {
        let mut pixmap = tiny_skia::Pixmap::new(368, 542).expect("a page-sized pixmap");
        let path = spike(1.0e7);
        let at = tiny_skia::Transform::from_translate(163.0, 319.0).pre_scale(1.0, -1.0);
        assert!(!expressible(&path, at, 0.0));
        let paint = tiny_skia::Paint {
            anti_alias: true,
            ..tiny_skia::Paint::default()
        };
        super::fill(
            &mut pixmap.as_mut(),
            &path,
            &paint,
            tiny_skia::FillRule::Winding,
            at,
            None,
        );
        assert!(
            pixmap.data().iter().any(|&byte| byte != 0),
            "the visible part of the spike is still drawn"
        );
    }
}
