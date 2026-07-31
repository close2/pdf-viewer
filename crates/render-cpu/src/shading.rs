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
/// `page_to_path` maps page space into the space the path being drawn is stated in.
///
/// # Which space a paint is positioned in
///
/// `tiny-skia` *post-concatenates the drawing transform onto the paint's shader* —
/// `Pixmap::fill_path` and `Pixmap::stroke_path` both do it — so the transform handed to
/// a gradient or pattern is read in the path's own space, not the device's. Handing it a
/// device-space transform therefore applies the device transform twice.
///
/// That is not a hypothetical. It shipped: a gradient came out mirrored about the page's
/// horizontal centre at a scale of 1.0, where the y-flip happens to be its own inverse so
/// the second application cancels the geometry but not the flip, and displaced by a
/// scale-dependent amount at every other scale. A test in this crate pins a gradient's
/// value at two scales, because one scale cannot see it.
///
/// So the shading's transform, which maps its own space to *page* space, is carried the
/// rest of the way into the path's space here, and `tiny-skia` completes the journey to
/// the device. A pattern stays positioned relative to the page whatever the graphics
/// state held at fill time, which is what the specification asks for.
///
/// A sampled shading becomes a pattern, and `tiny-skia` patterns *borrow* their pixels, so
/// the caller lends somewhere to keep them. Everything else leaves `scratch` untouched.
pub(crate) fn shader<'a>(
    shading: &Shading,
    page_to_path: Transform,
    scratch: &'a mut Option<tiny_skia::Pixmap>,
) -> Option<tiny_skia::Shader<'a>> {
    let transform = crate::convert::transform(shading.transform.then(page_to_path));

    match shading.kind.as_ref() {
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
    let mut stops: Vec<tiny_skia::GradientStop> =
        Vec::with_capacity(ramp.stops.len().saturating_add(2));

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
    for stop in ramp.stops.iter() {
        let position = low + stop.at * (high - low);
        stops.push(tiny_skia::GradientStop::new(
            position.clamp(0.0, 1.0),
            crate::convert::color(stop.colour),
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
    } = shading.kind.as_ref()
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

/// Draws a mesh shading's triangles, clipped to a path (ISO 32000-2 §8.7.4.5.5).
///
/// The mesh is rasterised once by [`pdf_render::MeshRaster`] — the clause's own linear
/// interpolation, evaluated at each device pixel's centre — and the result is drawn as an
/// image confined to the shape. So the *colour* is `pdf-render`'s, identically on both
/// backends, and the *edge* is `tiny-skia`'s, antialiased as every other fill's is.
///
/// Until the forty-third session this subdivided each triangle until its corner colours
/// agreed to within 1/512 and filled the piece flat, then grew every piece by 0.8 pixels to
/// close the seams that left. `MeshRaster` has why that is gone.
#[expect(
    clippy::too_many_arguments,
    reason = "these are the parameters `fill_path` itself takes, threaded through one \
              level; bundling them into a struct would only move the list"
)]
pub(crate) fn fill_mesh(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    shape: &tiny_skia::Path,
    triangles: &[pdf_render::Triangle],
    to_device: Transform,
    fill_rule: tiny_skia::FillRule,
    shape_transform: tiny_skia::Transform,
    clip: Option<&tiny_skia::Mask>,
    blend: tiny_skia::BlendMode,
    anti_alias: bool,
) {
    let Some(raster) =
        pdf_render::MeshRaster::build(triangles, to_device, pixmap.width(), pixmap.height())
    else {
        return;
    };
    let Some(mut samples) = tiny_skia::Pixmap::new(raster.image.width, raster.image.height) else {
        return;
    };
    // `tiny-skia` pixmaps are premultiplied and `Image` is straight alpha, the same boundary
    // conversion `CpuRasterizer::draw_image` makes.
    for (target, source) in samples
        .pixels_mut()
        .iter_mut()
        .zip(raster.image.data.chunks_exact(4))
    {
        let alpha = source[3];
        *target = tiny_skia::PremultipliedColorU8::from_rgba(
            crate::premultiply(source[0], alpha),
            crate::premultiply(source[1], alpha),
            crate::premultiply(source[2], alpha),
            alpha,
        )
        .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
    }

    let paint = tiny_skia::Paint {
        shader: tiny_skia::Pattern::new(
            samples.as_ref(),
            tiny_skia::SpreadMode::Pad,
            // The raster is already at device resolution and is placed at whole pixels, so
            // no sample is ever interpolated and the filter cannot change a pixel.
            tiny_skia::FilterQuality::Nearest,
            1.0,
            tiny_skia::Transform::from_translate(
                // Both are device pixel indices, bounded by `MAX_EXTENT` at 2^24, so the
                // conversion is exact.
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a device pixel index is far below f32's exact integer limit"
                )]
                {
                    raster.left as f32
                },
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a device pixel index is far below f32's exact integer limit"
                )]
                {
                    raster.top as f32
                },
            ),
        ),
        blend_mode: blend,
        anti_alias,
        ..tiny_skia::Paint::default()
    };
    // The shape carries its own transform; the pattern's is in *device* space, which is what
    // a paint is read in — see `shader` above, and trap 2.
    let device_shape = shape.clone().transform(shape_transform);
    let Some(device_shape) = device_shape else {
        return;
    };
    pixmap.fill_path(
        &device_shape,
        &paint,
        fill_rule,
        tiny_skia::Transform::identity(),
        clip,
    );
}
