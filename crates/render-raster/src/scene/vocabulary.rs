//! The two vocabularies' words for the same thing: a display-list transform, colour, blend mode,
//! fill rule, path or image as raster's scene states it.
//!
//! Each is total in both directions or says why it is not; none reads the device or the view.

use std::sync::Arc;

use pdf_render::{BlendMode, Color, FillRule, Image, Path, PathCommand, Point, Transform};
use raster_scene::SceneBuilder;

/// The display list's transform, as raster's — same six coefficients, same
/// row-vector convention, same `then` order.
pub(crate) fn affine(t: Transform) -> raster_scene::Affine {
    raster_scene::Affine {
        a: t.a,
        b: t.b,
        c: t.c,
        d: t.d,
        e: t.e,
        f: t.f,
    }
}

pub(crate) fn point(p: Point) -> raster_scene::Point {
    raster_scene::Point::new(p.x, p.y)
}

pub(crate) fn colour(c: Color) -> raster_scene::Color {
    raster_scene::Color::new(c.r, c.g, c.b, c.a)
}

/// A colour quantised to straight-alpha RGBA8 bytes, rounding as every backend's
/// boundary does.
pub(crate) fn byte_colour(c: Color) -> [u8; 4] {
    let byte = |v: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped into 0..=255 before the cast"
        )]
        let quantised = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        quantised
    };
    [byte(c.r), byte(c.g), byte(c.b), byte(c.a)]
}

pub(crate) fn fill_rule(rule: FillRule) -> raster_scene::FillRule {
    match rule {
        FillRule::NonZero => raster_scene::FillRule::NonZero,
        FillRule::EvenOdd => raster_scene::FillRule::EvenOdd,
    }
}

/// The blend function half of a display-list blend mode, as raster's scene states it.
///
/// Table 134 and Table 135's sixteen go straight across. §11.7.4.3's special overprinting
/// blend mode is not one of them: its value is `C_b` or `C_s` per channel, which
/// [`compose`] states as a compositing operator, and the blend function beside that operator
/// is Normal — the builder refuses any other there (`doc/adr/1295`).
#[expect(
    clippy::match_same_arms,
    reason = "the overprinting arm shares Normal's blend function because its own value \
              travels as the compositing operator; merging it into Normal would state that \
              the two are the same mode"
)]
pub(crate) fn blend_mode(blend: BlendMode) -> raster_scene::BlendMode {
    match blend {
        BlendMode::Normal => raster_scene::BlendMode::Normal,
        BlendMode::Multiply => raster_scene::BlendMode::Multiply,
        BlendMode::Screen => raster_scene::BlendMode::Screen,
        BlendMode::Overlay => raster_scene::BlendMode::Overlay,
        BlendMode::Darken => raster_scene::BlendMode::Darken,
        BlendMode::Lighten => raster_scene::BlendMode::Lighten,
        BlendMode::ColorDodge => raster_scene::BlendMode::ColorDodge,
        BlendMode::ColorBurn => raster_scene::BlendMode::ColorBurn,
        BlendMode::HardLight => raster_scene::BlendMode::HardLight,
        BlendMode::SoftLight => raster_scene::BlendMode::SoftLight,
        BlendMode::Difference => raster_scene::BlendMode::Difference,
        BlendMode::Exclusion => raster_scene::BlendMode::Exclusion,
        BlendMode::Hue => raster_scene::BlendMode::Hue,
        BlendMode::Saturation => raster_scene::BlendMode::Saturation,
        BlendMode::Color => raster_scene::BlendMode::Color,
        BlendMode::Luminosity => raster_scene::BlendMode::Luminosity,
        // §11.7.4.3's special overprinting blend mode: [`compose`] carries its value.
        BlendMode::Overprint(_) => raster_scene::BlendMode::Normal,
    }
}

/// The compositing operator a display-list blend mode asks for: source-over for every
/// mode but §11.7.4.3's special overprinting one, which is destination-over in the channels
/// it keeps and source-over in the rest.
///
/// ISO 32000-2 §11.7.4.3 decides the blend function per component — "process colour
/// components with nonzero values shall replace the corresponding component values of the
/// backdrop; components with zero values leave the existing backdrop value unchanged" — and
/// `pdf_render::Overprint` carries the answer the interpreter took on the document's own
/// tints. Substituted into §11.3.3's formula, `B = C_b` is Porter-Duff destination-over and
/// `B = C_s` is source-over, which is the vocabulary [`raster_scene::Compose::keeping`]
/// states; its own documentation carries the derivation (`doc/adr/1295`).
pub(crate) fn compose(blend: BlendMode) -> raster_scene::Compose {
    match blend {
        BlendMode::Overprint(overprint) => raster_scene::Compose::keeping(overprint.kept()),
        _ => raster_scene::Compose::SrcOver,
    }
}

/// One mark with no compositing operator of its own — an image, or a stroke raster strokes
/// itself — drawn by `mark`, directly or under §11.7.4.3's special overprinting blend mode.
///
/// Under the mode the mark is the one element of an isolated group at alpha 1 under no clip,
/// mask or blend, composited by the mode's operator: onto a transparent backdrop the group
/// is the mark's own premultiplied colour, so compositing it is compositing the mark — the
/// construction [`super::Encoder::shaped`]'s halves use for §11.4.6, for the same reason. The mark
/// keeps its own clip and mask inside the group, where §11.3.7.2 multiplies them into the
/// source alpha the mode is written against.
///
/// `knockout_element` is [`super::Encoder::knockout_element`]: there the mode is Normal's arithmetic and
/// the mark is drawn as it stands.
pub(crate) fn overprinted(
    builder: &mut SceneBuilder,
    blend: BlendMode,
    knockout_element: bool,
    mark: impl FnOnce(&mut SceneBuilder) -> Result<(), raster_scene::SceneError>,
) -> Result<(), raster_scene::SceneError> {
    let operator = compose(blend);
    if operator == raster_scene::Compose::SrcOver || knockout_element {
        return mark(builder);
    }
    builder.group(
        raster_scene::GroupSpec {
            alpha: 1.0,
            blend: raster_scene::BlendMode::Normal,
            clip: None,
            knockout: false,
            mask: None,
            compose: operator,
            isolated: true,
        },
        mark,
    )
}

/// A display-list path as raster segments: both are cubics-only, so the mapping
/// is positional.
pub(crate) fn segments(path: &Path) -> Vec<raster_scene::Segment> {
    path.commands()
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo(p) => raster_scene::Segment::MoveTo(point(p)),
            PathCommand::LineTo(p) => raster_scene::Segment::LineTo(point(p)),
            PathCommand::CurveTo(c1, c2, to) => raster_scene::Segment::CubicTo {
                c1: point(c1),
                c2: point(c2),
                to: point(to),
            },
            PathCommand::Close => raster_scene::Segment::Close,
        })
        .collect()
}

/// A display-list image as raster's upload spec — the same layout (straight-alpha
/// RGBA8, top row first, no padding), so the `Arc` is shared, not copied.
pub(super) fn spec(image: &Image) -> raster_scene::ImageSpec {
    raster_scene::ImageSpec {
        width: image.width,
        height: image.height,
        data: Arc::clone(&image.data),
    }
}
