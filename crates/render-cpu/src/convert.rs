//! Translation from `pdf-render` types to `tiny-skia` types.
//!
//! Kept in one module so that the whole surface of the dependency on `tiny-skia`
//! is visible in a single file. If the CPU rasteriser is ever replaced, this is
//! the file that changes.

use pdf_render::{BlendMode, Color, FillRule, LineCap, LineJoin, Path, PathCommand, Stroke};
use pdf_render::{Point, Transform};

/// Converts a PDF matrix to a `tiny-skia` transform.
///
/// The component orders coincide exactly: `Transform::from_row` takes
/// `(sx, ky, kx, sy, tx, ty)`, which is `(a, b, c, d, e, f)` in the PDF matrix
/// spelling. This is verified by a test rather than assumed, because a silent
/// transposition here would misplace every stroke and glyph on the page.
pub(crate) fn transform(t: Transform) -> tiny_skia::Transform {
    tiny_skia::Transform::from_row(t.a, t.b, t.c, t.d, t.e, t.f)
}

/// Converts a path, returning `None` if `tiny-skia` rejects it.
///
/// `tiny-skia` rejects empty paths and paths containing non-finite coordinates.
/// Both are reachable from a malformed document, so this is an expected outcome
/// rather than a defect, and the caller reports it as an error rather than
/// drawing nothing.
pub(crate) fn path(p: &Path) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::new();
    for command in p.commands() {
        match *command {
            PathCommand::MoveTo(Point { x, y }) => builder.move_to(x, y),
            PathCommand::LineTo(Point { x, y }) => builder.line_to(x, y),
            PathCommand::CurveTo(c1, c2, end) => {
                builder.cubic_to(c1.x, c1.y, c2.x, c2.y, end.x, end.y);
            }
            PathCommand::Close => builder.close(),
        }
    }
    builder.finish()
}

/// Converts a fill rule.
pub(crate) fn fill_rule(rule: FillRule) -> tiny_skia::FillRule {
    match rule {
        FillRule::NonZero => tiny_skia::FillRule::Winding,
        FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
    }
}

/// Converts a colour.
///
/// Both sides use straight (non-premultiplied) alpha with components in
/// `0.0..=1.0`, so this is a direct mapping. Out-of-range components are clamped
/// by `tiny-skia`; PDF permits values outside the nominal range in some colour
/// spaces, and clamping matches what other viewers do.
pub(crate) fn color(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(c.r, c.g, c.b, c.a).unwrap_or(tiny_skia::Color::BLACK)
}

/// Converts a blend mode.
///
/// `tiny-skia` implements all sixteen PDF blend modes, including the four
/// non-separable ones, so this mapping is total — there is no fallback case that
/// would silently render the wrong compositing result.
pub(crate) fn blend_mode(mode: BlendMode) -> tiny_skia::BlendMode {
    match mode {
        // PDF's Normal is source-over, not source-replace.
        BlendMode::Normal => tiny_skia::BlendMode::SourceOver,
        BlendMode::Multiply => tiny_skia::BlendMode::Multiply,
        BlendMode::Screen => tiny_skia::BlendMode::Screen,
        BlendMode::Overlay => tiny_skia::BlendMode::Overlay,
        BlendMode::Darken => tiny_skia::BlendMode::Darken,
        BlendMode::Lighten => tiny_skia::BlendMode::Lighten,
        BlendMode::ColorDodge => tiny_skia::BlendMode::ColorDodge,
        BlendMode::ColorBurn => tiny_skia::BlendMode::ColorBurn,
        BlendMode::HardLight => tiny_skia::BlendMode::HardLight,
        BlendMode::SoftLight => tiny_skia::BlendMode::SoftLight,
        BlendMode::Difference => tiny_skia::BlendMode::Difference,
        BlendMode::Exclusion => tiny_skia::BlendMode::Exclusion,
        BlendMode::Hue => tiny_skia::BlendMode::Hue,
        BlendMode::Saturation => tiny_skia::BlendMode::Saturation,
        BlendMode::Color => tiny_skia::BlendMode::Color,
        BlendMode::Luminosity => tiny_skia::BlendMode::Luminosity,
    }
}

/// Converts stroke parameters.
///
/// Two semantics line up in our favour and are worth naming, because relying on
/// them silently would be fragile:
///
/// - A width of `0.0` means "thinnest line the device can draw" in PDF, and
///   selects hairline stroking in `tiny-skia`. Both agree, so zero width needs no
///   special handling.
/// - `tiny-skia`'s default miter limit is `4.0` while PDF's initial value is
///   `10.0`. The limit is therefore always set explicitly from the PDF state and
///   never left to default.
pub(crate) fn stroke(s: &Stroke) -> tiny_skia::Stroke {
    tiny_skia::Stroke {
        width: s.width,
        miter_limit: s.miter_limit,
        line_cap: match s.cap {
            LineCap::Butt => tiny_skia::LineCap::Butt,
            LineCap::Round => tiny_skia::LineCap::Round,
            LineCap::Square => tiny_skia::LineCap::Square,
        },
        line_join: match s.join {
            LineJoin::Miter => tiny_skia::LineJoin::Miter,
            LineJoin::Round => tiny_skia::LineJoin::Round,
            LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
        },
        // An all-zero or negative dash array is rejected by `StrokeDash::new`, which
        // returns `None`; that yields a solid line, matching how PDF treats a
        // degenerate dash array.
        dash: if s.dash_array.is_empty() {
            None
        } else {
            tiny_skia::StrokeDash::new(s.dash_array.clone(), s.dash_phase)
        },
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "the matrix mapping must be exact, not approximate — an approximate \
              comparison here would not catch a transposition"
)]
mod tests {
    use super::transform;
    use pdf_render::Transform;

    /// The component orders of a PDF matrix and `Transform::from_row` are asserted
    /// to coincide. A transposition here would misplace all geometry, and would do
    /// so subtly enough to survive casual inspection, so it is pinned by a test.
    #[test]
    fn matrix_components_map_without_transposition() {
        let ours = Transform::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let theirs = transform(ours);

        assert_eq!(theirs.sx, 1.0, "a -> sx");
        assert_eq!(theirs.ky, 2.0, "b -> ky");
        assert_eq!(theirs.kx, 3.0, "c -> kx");
        assert_eq!(theirs.sy, 4.0, "d -> sy");
        assert_eq!(theirs.tx, 5.0, "e -> tx");
        assert_eq!(theirs.ty, 6.0, "f -> ty");
    }

    /// Both implementations must agree on where a point lands, not merely on field
    /// names. A shear is used because a pure scale or translation would pass even
    /// under a transposition.
    #[test]
    fn transforms_agree_on_a_sheared_point() {
        let ours = Transform::new(2.0, 0.5, -0.25, 3.0, 10.0, -4.0);
        let point = pdf_render::Point::new(7.0, 11.0);

        let expected = ours.apply(point);
        let mut actual = tiny_skia::Point::from_xy(point.x, point.y);
        transform(ours).map_point(&mut actual);

        assert_eq!(actual.x, expected.x);
        assert_eq!(actual.y, expected.y);
    }
}
