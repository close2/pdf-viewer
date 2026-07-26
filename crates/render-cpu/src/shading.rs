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

/// How different two corner colours may be before a triangle is split.
///
/// Below one part in five hundred, which is finer than an eight-bit channel can represent,
/// so the flat-filled result is indistinguishable from true interpolation once quantised.
const FLAT_ENOUGH: f32 = 1.0 / 512.0;

/// How many times a triangle may be split before it is drawn flat regardless.
///
/// Each level multiplies the triangle count by four, so this bounds one mesh triangle at
/// 4^6 = 4096 draws. Reached only by a triangle whose corner colours are far apart *and*
/// which covers a large area; the bound stops a pathological mesh from taking the renderer
/// with it.
const MAX_SUBDIVISION: u32 = 6;

/// Draws a mesh shading's triangles, clipped to a path.
///
/// `tiny-skia` has no Gouraud shading, so each triangle is subdivided until its corner
/// colours agree closely enough to fill flat. Subdivision is by quarters rather than
/// halves, so the pieces stay well-shaped: repeatedly splitting one edge produces slivers,
/// and slivers rasterise with visible seams between them.
#[expect(
    clippy::too_many_arguments,
    reason = "these are the parameters `fill_path` itself takes, threaded through one \
              level; bundling them into a struct would only move the list"
)]
pub(crate) fn fill_mesh(
    pixmap: &mut tiny_skia::Pixmap,
    shape: &tiny_skia::Path,
    triangles: &[pdf_render::Triangle],
    to_device: Transform,
    fill_rule: tiny_skia::FillRule,
    shape_transform: tiny_skia::Transform,
    clip: Option<&tiny_skia::Mask>,
    blend: tiny_skia::BlendMode,
    anti_alias: bool,
) {
    // The mesh is confined to the shape being filled, which is what a clip is for. Building
    // it once here rather than clipping each triangle keeps the edge of the shape
    // antialiased rather than stepped.
    let mut mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height());
    let Some(mask) = mask.as_mut() else {
        return;
    };
    mask.fill_path(shape, fill_rule, anti_alias, shape_transform);
    if let Some(clip) = clip {
        // `tiny-skia` intersects a mask with a *path*, not with another mask, so the two
        // coverages are combined directly. Multiplying is what intersection means for
        // coverage, and it keeps both edges antialiased rather than snapping either.
        for (own, outer) in mask.data_mut().iter_mut().zip(clip.data().iter()) {
            *own = u8::try_from(
                (u16::from(*own).saturating_mul(u16::from(*outer))).saturating_add(127) / 255,
            )
            .unwrap_or(0);
        }
    }

    // Triangles are carried into device space here rather than by `fill_path`, because the
    // seam repair below is a fixed distance in *pixels* and has no meaning in the mesh's
    // own coordinates.
    for triangle in triangles {
        let device = pdf_render::Triangle {
            points: triangle.points.map(|point| to_device.apply(point)),
            colours: triangle.colours,
        };
        draw_triangle(pixmap, device, Some(mask), blend, 0);
    }
    let _ = anti_alias;
}

/// How far each triangle is grown, in pixels, to close the seams between neighbours.
///
/// Two triangles sharing an edge do not tile exactly once rasterised: a pixel whose centre
/// falls on the shared edge belongs to neither, and shows through as a bright speck. On the
/// specification's own Coons test page that left nine white pinholes.
///
/// Growing each triangle by rather less than a pixel makes neighbours overlap instead. The
/// overlap is invisible because subdivision has already made the colours on either side of
/// a shared edge nearly equal — the same property that lets them be filled flat at all.
const SEAM_OVERLAP: f32 = 0.35;

/// Grows a triangle about its centroid by a fixed distance in device pixels.
fn grow(triangle: &pdf_render::Triangle) -> [tiny_skia::Point; 3] {
    let [a, b, c] = triangle.points;
    let centre = pdf_render::Point::new((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0);
    triangle.points.map(|point| {
        let (dx, dy) = (point.x - centre.x, point.y - centre.y);
        let length = dx.hypot(dy);
        if length <= f32::EPSILON {
            return tiny_skia::Point::from_xy(point.x, point.y);
        }
        tiny_skia::Point::from_xy(
            point.x + dx / length * SEAM_OVERLAP,
            point.y + dy / length * SEAM_OVERLAP,
        )
    })
}

/// Fills one triangle, in device space, splitting it first if its colours vary across it.
fn draw_triangle(
    pixmap: &mut tiny_skia::Pixmap,
    triangle: pdf_render::Triangle,
    mask: Option<&tiny_skia::Mask>,
    blend: tiny_skia::BlendMode,
    depth: u32,
) {
    if depth < MAX_SUBDIVISION && !triangle.is_flat(FLAT_ENOUGH) {
        for piece in triangle.subdivide() {
            draw_triangle(pixmap, piece, mask, blend, depth.saturating_add(1));
        }
        return;
    }

    let mut builder = tiny_skia::PathBuilder::new();
    let [a, b, c] = grow(&triangle);
    builder.move_to(a.x, a.y);
    builder.line_to(b.x, b.y);
    builder.line_to(c.x, c.y);
    builder.close();
    let Some(path) = builder.finish() else {
        return;
    };

    let paint = tiny_skia::Paint {
        shader: tiny_skia::Shader::SolidColor(crate::convert::color(triangle.average_colour())),
        blend_mode: blend,
        // Antialiasing every small triangle would show its edges as seams against its
        // neighbours, because two abutting antialiased edges do not sum to full coverage.
        // The shape's own outline is antialiased by the mask instead.
        anti_alias: false,
        ..tiny_skia::Paint::default()
    };
    pixmap.fill_path(
        &path,
        &paint,
        tiny_skia::FillRule::Winding,
        // The points are already in device space.
        tiny_skia::Transform::identity(),
        mask,
    );
}
