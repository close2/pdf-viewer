//! Translation from `pdf-render` types to `tiny-skia` types.
//!
//! Kept in one module so that the whole surface of the dependency on `tiny-skia`
//! is visible in a single file. If the CPU rasteriser is ever replaced, this is
//! the file that changes.

use pdf_render::{
    BlendMode, Color, FillRule, LineCap, LineJoin, Path, PathCommand, Point, Rect, Stroke,
    Transform,
};

/// Converts a PDF matrix to a `tiny-skia` transform.
///
/// The component orders coincide exactly: `Transform::from_row` takes
/// `(sx, ky, kx, sy, tx, ty)`, which is `(a, b, c, d, e, f)` in the PDF matrix
/// spelling. This is verified by a test rather than assumed, because a silent
/// transposition here would misplace every stroke and glyph on the page.
pub(crate) fn transform(t: Transform) -> tiny_skia::Transform {
    tiny_skia::Transform::from_row(t.a, t.b, t.c, t.d, t.e, t.f)
}

/// Converts a rectangle, which the two crates spell the same way.
///
/// `tiny-skia` keeps a rectangle normalised by construction and this crate says a normalised
/// one has `min <= max`, so the corners map across as they stand.
pub(crate) fn from_skia_rect(r: tiny_skia::Rect) -> Rect {
    Rect::from_corners(
        Point::new(r.left(), r.top()),
        Point::new(r.right(), r.bottom()),
    )
}

/// Converts a rectangle the other way, returning `None` where `tiny-skia` will not hold it.
///
/// That library keeps a rectangle valid by construction — finite, and with both sides positive —
/// so a rectangle it refuses is one no painting operation could have used anyway.
pub(crate) fn to_skia_rect(r: Rect) -> Option<tiny_skia::Rect> {
    tiny_skia::Rect::from_ltrb(r.min.x, r.min.y, r.max.x, r.max.y)
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

/// Converts a `tiny-skia` path back to the display list's own.
///
/// The one place geometry travels back out of the rasteriser's library. It exists because
/// ISO 32000-2 §8.5.3.2's rule about a dash with no length is stated in `pdf-render`, so
/// that both backends apply the same one, and the path it is applied to is whatever
/// `tiny-skia`'s dasher produced.
///
/// `tiny-skia` stores quadratics — its dasher emits them for a curve it has cut — so they
/// are elevated to the cubic through the same curve, which is exact and is what glyph
/// loading already does for TrueType outlines.
pub(crate) fn from_skia_path(p: &tiny_skia::Path) -> Path {
    let mut out = Path::new();
    let mut current = Point::new(0.0, 0.0);
    let point = |p: tiny_skia::Point| Point::new(p.x, p.y);
    for segment in p.segments() {
        match segment {
            tiny_skia::PathSegment::MoveTo(p) => {
                current = point(p);
                out.push(PathCommand::MoveTo(current));
            }
            tiny_skia::PathSegment::LineTo(p) => {
                current = point(p);
                out.push(PathCommand::LineTo(current));
            }
            tiny_skia::PathSegment::QuadTo(c, p) => {
                let (c, end) = (point(c), point(p));
                // A quadratic's cubic control points are each a third of the way from an
                // endpoint towards the quadratic's own control point.
                let lift = |from: Point| {
                    Point::new(
                        from.x + 2.0 / 3.0 * (c.x - from.x),
                        from.y + 2.0 / 3.0 * (c.y - from.y),
                    )
                };
                out.push(PathCommand::CurveTo(lift(current), lift(end), end));
                current = end;
            }
            tiny_skia::PathSegment::CubicTo(c1, c2, p) => {
                current = point(p);
                out.push(PathCommand::CurveTo(point(c1), point(c2), current));
            }
            tiny_skia::PathSegment::Close => out.push(PathCommand::Close),
        }
    }
    out
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

/// Converts a blend mode, for the twelve this backend hands to `tiny-skia`.
///
/// ISO 32000-2 §11.3.5.2's twelve separable modes are the library's, and they agree with
/// Vello to the channel. §11.3.5.3's four are not: three of them are wrong there by 113 of
/// 255 (ADR 0046), so `crate::blend` computes all four and every call reaching this
/// function under one of them is drawing onto a *transparent* layer for it.
///
/// Which is why they map to `SourceOver` rather than to the library's own versions, and
/// why that is a derivation rather than a fallback: §11.3.6's compositing formula with
/// α<sub>b</sub> = 0 collapses to the source colour whatever `B(Cb, Cs)` is, so on an
/// empty layer every one of the sixteen modes *is* Normal.
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
        // Table 135's four, drawn onto transparency and composited by `crate::blend`.
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => {
            tiny_skia::BlendMode::SourceOver
        }
    }
}

/// Converts stroke parameters, resolving the width against the device.
///
/// The width comes from [`Stroke::device_width`] rather than from the field, because
/// ISO 32000-2 §8.4.3.2's zero-width minimum and §10.7.5's stroke adjustment are the
/// same decision and both backends have to make it the same way. `tiny-skia` would
/// answer §8.4.3.2 by itself — a width of `0.0` selects hairline stroking, which is one
/// device pixel — and at every scale the two answers coincide, which is what
/// `render-cpu/tests/stroke_width.rs` pins. Vello has no hairline mode at all, so
/// relying on the rasteriser's convention was the reason a zero-width stroke was
/// invisible on the GPU for fifteen sessions.
///
/// One semantic still needs naming: `tiny-skia`'s default miter limit is `4.0` while
/// PDF's initial value is `10.0`, so the limit is always set explicitly from the PDF
/// state and never left to default.
pub(crate) fn stroke(s: &Stroke, to_device: Transform) -> tiny_skia::Stroke {
    tiny_skia::Stroke {
        width: s.device_width(to_device),
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
