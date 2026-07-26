//! Turning a resolved shading into a `tiny-skia` shader.
//!
//! Three of the four shading kinds map onto something `tiny-skia` implements natively, so
//! they are handed over rather than rasterised here: axial becomes a linear gradient,
//! radial a two-circle radial gradient — `tiny-skia` takes both radii, which is exactly
//! PDF's model — and a sampled shading becomes a pattern.
//!
//! Mesh shadings have no native equivalent and are drawn by the caller as triangles.
//!
//! # `Extend` has no direct equivalent, and needs one
//!
//! PDF's `/Extend` says whether the shading continues past each end. Where it does not,
//! *nothing is painted* there — which is not the same as painting the end colour, and is
//! the difference between a band across part of a shape and a wash over all of it.
//!
//! `tiny-skia`'s spread modes cannot express it: `Pad` paints the end colour forever,
//! `Repeat` and `Reflect` tile. So a non-extended end gets a fully transparent stop at
//! the very edge of the ramp, carrying the same colour so that no fringe appears as it
//! fades. `Pad` then repeats *transparency* beyond that point, which is precisely what
//! `/Extend false` asks for. The cost is that the cut-off is a gradient a fraction of a
//! percent of the axis wide rather than a hard edge, which is well under a pixel on any
//! real page.

use pdf_render::{Color, Ramp, Shading, ShadingKind, Transform};

/// How wide the transparent transition at a non-extended end is, as a fraction of the
/// ramp. Small enough to be sub-pixel on any page, large enough to survive `f32`.
const CUTOFF: f32 = 0.0005;

/// Builds a shader for a shading, or `None` for kinds the caller must draw itself.
///
/// A sampled shading becomes a pattern, and `tiny-skia` patterns *borrow* their pixels, so
/// the caller lends somewhere to keep them. Everything else leaves `scratch` untouched.
pub(crate) fn shader<'a>(
    shading: &Shading,
    to_device: Transform,
    scratch: &'a mut Option<tiny_skia::Pixmap>,
) -> Option<tiny_skia::Shader<'a>> {
    // The shading's own space reaches the device through its own transform, not through
    // the transform of the path being filled: a pattern is positioned relative to the
    // page, not to whatever the graphics state happened to hold at fill time.
    let transform = crate::convert::transform(shading.transform.then(to_device));

    match &shading.kind {
        ShadingKind::Axial {
            start,
            end,
            ramp,
            extend,
        } => tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(start.x, start.y),
            tiny_skia::Point::from_xy(end.x, end.y),
            stops(ramp, *extend),
            tiny_skia::SpreadMode::Pad,
            transform,
        ),
        ShadingKind::Radial {
            start,
            start_radius,
            end,
            end_radius,
            ramp,
            extend,
        } => tiny_skia::RadialGradient::new(
            tiny_skia::Point::from_xy(start.x, start.y),
            *start_radius,
            tiny_skia::Point::from_xy(end.x, end.y),
            *end_radius,
            stops(ramp, *extend),
            tiny_skia::SpreadMode::Pad,
            transform,
        ),
        ShadingKind::Sampled { .. } => sampled_shader(shading, transform, scratch),
        // Meshes carry a colour per triangle corner, which no shader can express; the
        // caller subdivides and fills them. A kind added later lands here too, and
        // returning None makes the caller report it rather than draw nothing.
        _ => None,
    }
}

/// Builds the gradient stops for a ramp, honouring `/Extend`.
fn stops(ramp: &Ramp, extend: (bool, bool)) -> Vec<tiny_skia::GradientStop> {
    let count = ramp.colours.len();
    let mut stops: Vec<tiny_skia::GradientStop> = Vec::with_capacity(count.saturating_add(2));

    // A non-extended end is cut off by a transparent stop just inside the ramp, so `Pad`
    // repeats transparency beyond it. See the note at the top of this module.
    let (low, high) = match extend {
        (true, true) => (0.0, 1.0),
        (false, true) => (CUTOFF, 1.0),
        (true, false) => (0.0, 1.0 - CUTOFF),
        (false, false) => (CUTOFF, 1.0 - CUTOFF),
    };

    if !extend.0 {
        stops.push(transparent_stop(0.0, ramp.colour_at(0.0)));
    }
    for (index, colour) in ramp.colours.iter().enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the ramp length is a small constant"
        )]
        let t = index as f32 / (count.saturating_sub(1).max(1)) as f32;
        let position = low + t * (high - low);
        stops.push(tiny_skia::GradientStop::new(
            position.clamp(0.0, 1.0),
            crate::convert::color(*colour),
        ));
    }
    if !extend.1 {
        stops.push(transparent_stop(1.0, ramp.colour_at(1.0)));
    }
    stops
}

/// A stop that paints nothing, keeping the neighbouring colour so no fringe appears.
fn transparent_stop(position: f32, colour: Color) -> tiny_skia::GradientStop {
    tiny_skia::GradientStop::new(position, crate::convert::color(Color { a: 0.0, ..colour }))
}

/// Builds a pattern shader over a sampled shading's grid.
fn sampled_shader<'a>(
    shading: &Shading,
    transform: tiny_skia::Transform,
    scratch: &'a mut Option<tiny_skia::Pixmap>,
) -> Option<tiny_skia::Shader<'a>> {
    let ShadingKind::Sampled {
        domain,
        width,
        height,
        pixels,
    } = &shading.kind
    else {
        return None;
    };

    // The grid covers the domain rectangle, so the pattern is scaled from pixel
    // coordinates onto it and then carried into the shading's own space.
    let [x0, x1, y0, y1] = *domain;
    let (span_x, span_y) = (x1 - x0, y1 - y0);
    if span_x == 0.0 || span_y == 0.0 || *width == 0 || *height == 0 {
        return None;
    }

    let mut pixmap = tiny_skia::Pixmap::new(*width, *height)?;
    for (destination, colour) in pixmap.pixels_mut().iter_mut().zip(pixels.iter()) {
        *destination = crate::convert::color(*colour).premultiply().to_color_u8();
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "grid dimensions are bounded well inside f32's exact integer range"
    )]
    let to_domain = tiny_skia::Transform::from_row(
        span_x / *width as f32,
        0.0,
        0.0,
        span_y / *height as f32,
        x0,
        y0,
    );

    Some(tiny_skia::Pattern::new(
        scratch.insert(pixmap).as_ref(),
        tiny_skia::SpreadMode::Pad,
        tiny_skia::FilterQuality::Bilinear,
        1.0,
        transform.pre_concat(to_domain),
    ))
}
