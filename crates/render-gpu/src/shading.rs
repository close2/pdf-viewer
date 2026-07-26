//! Turning a resolved shading into a Vello brush.
//!
//! Axial and radial shadings map onto Vello's own gradients, and a two-point radial
//! gradient is exactly PDF's model — the same correspondence the CPU backend found in
//! `tiny-skia`. That both rasterisers express these natively, in the same terms, is the
//! evidence that the display list's neutral form is the right one.
//!
//! Mesh shadings have no equivalent in either and are drawn as triangles by the caller.
//!
//! # `/Extend` is handled exactly as on the CPU
//!
//! Where a shading does not extend it paints nothing beyond that end, and neither
//! rasteriser's spread modes can say so. Both get a transparent stop at the very edge of
//! the ramp, so that `Pad` repeats transparency. Keeping the two backends' workaround
//! identical is deliberate: they are meant to agree pixel for pixel, and two different
//! approximations of the same thing would not.

use pdf_render::{Color, Ramp, Shading, ShadingKind, Transform};
use vello::peniko;

/// How wide the transparent transition at a non-extended end is, as a fraction of the
/// ramp. Matches `render-cpu`, because the backends must agree.
const CUTOFF: f32 = 0.0005;

/// Builds a brush for a shading, or `None` for kinds the caller must draw itself.
///
/// The returned transform positions the brush; Vello applies it to the gradient rather
/// than to the shape, which is what keeps a pattern anchored to the page rather than to
/// the path being filled.
pub(crate) fn brush(
    shading: &Shading,
    to_device: Transform,
) -> Option<(peniko::Brush, Option<vello::kurbo::Affine>)> {
    let gradient = match &shading.kind {
        ShadingKind::Axial {
            start,
            end,
            ramp,
            extend,
        } => peniko::Gradient::new_linear(
            (f64::from(start.x), f64::from(start.y)),
            (f64::from(end.x), f64::from(end.y)),
        )
        .with_stops(stops(ramp, *extend).as_slice()),
        ShadingKind::Radial {
            start,
            start_radius,
            end,
            end_radius,
            ramp,
            extend,
        } => peniko::Gradient::new_two_point_radial(
            (f64::from(start.x), f64::from(start.y)),
            *start_radius,
            (f64::from(end.x), f64::from(end.y)),
            *end_radius,
        )
        .with_stops(stops(ramp, *extend).as_slice()),
        // A sampled shading is an image brush and a mesh is triangles; neither is a
        // gradient, and returning `None` makes the caller report or draw it.
        _ => return None,
    };

    let gradient = gradient.with_extend(peniko::Extend::Pad);
    let transform = crate::scene::affine(shading.transform.then(to_device));
    Some((peniko::Brush::Gradient(gradient), Some(transform)))
}

/// Builds the gradient stops for a ramp, honouring `/Extend`.
fn stops(ramp: &Ramp, extend: (bool, bool)) -> Vec<peniko::ColorStop> {
    let count = ramp.colours.len();
    let mut stops: Vec<peniko::ColorStop> = Vec::with_capacity(count.saturating_add(2));

    let (low, high) = match extend {
        (true, true) => (0.0, 1.0),
        (false, true) => (CUTOFF, 1.0),
        (true, false) => (0.0, 1.0 - CUTOFF),
        (false, false) => (CUTOFF, 1.0 - CUTOFF),
    };

    if !extend.0 {
        stops.push(stop(0.0, transparent(ramp.colour_at(0.0))));
    }
    for (index, colour) in ramp.colours.iter().enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the ramp length is a small constant"
        )]
        let t = index as f32 / count.saturating_sub(1).max(1) as f32;
        stops.push(stop((low + t * (high - low)).clamp(0.0, 1.0), *colour));
    }
    if !extend.1 {
        stops.push(stop(1.0, transparent(ramp.colour_at(1.0))));
    }
    stops
}

/// Keeps a colour's hue but paints nothing, so no fringe appears as it fades out.
fn transparent(colour: Color) -> Color {
    Color { a: 0.0, ..colour }
}

fn stop(offset: f32, colour: Color) -> peniko::ColorStop {
    peniko::ColorStop {
        offset,
        color: peniko::color::DynamicColor::from_alpha_color(crate::scene::color(colour)),
    }
}

/// How different two corner colours may be before a triangle is split.
///
/// The same threshold the CPU backend uses, because the two are meant to agree.
const FLAT_ENOUGH: f32 = 1.0 / 512.0;

/// How many times a triangle may be split before it is drawn flat regardless.
const MAX_SUBDIVISION: u32 = 6;

/// How far each triangle is grown, in pixels, to close the seams between neighbours.
///
/// The same value `render-cpu` uses, and it must stay the same or the backends stop
/// agreeing. Vello needs it more, not less: it antialiases every edge, so without the
/// overlap a mesh showed a white hairline along every shared edge rather than the CPU
/// backend's occasional dropped pixel. See `render-cpu`'s note for the measurements.
const SEAM_OVERLAP: f32 = 0.8;

/// Draws a mesh shading's triangles into the current layer.
///
/// The caller has already pushed a layer clipped to the shape being filled, so these need
/// no clipping of their own. Vello has no Gouraud shading either, so the triangles are
/// subdivided until their corner colours agree closely enough to fill flat — by quarters,
/// which keeps the pieces well-shaped where halving would produce slivers.
///
/// Everything here mirrors `render-cpu` deliberately, down to the thresholds: the two
/// backends are meant to agree pixel for pixel, and two different approximations of the
/// same thing would not.
pub(crate) fn fill_mesh(
    scene: &mut vello::Scene,
    triangles: &[pdf_render::Triangle],
    to_device: Transform,
) {
    // Carried into device space here rather than by the scene transform, because the seam
    // repair is a fixed distance in *pixels* and has no meaning in the mesh's own space.
    for triangle in triangles {
        let device = pdf_render::Triangle {
            points: triangle.points.map(|point| to_device.apply(point)),
            colours: triangle.colours,
        };
        draw_triangle(scene, device, 0);
    }
}

fn draw_triangle(scene: &mut vello::Scene, triangle: pdf_render::Triangle, depth: u32) {
    if depth < MAX_SUBDIVISION && !triangle.is_flat(FLAT_ENOUGH) {
        for piece in triangle.subdivide() {
            draw_triangle(scene, piece, depth.saturating_add(1));
        }
        return;
    }

    let [a, b, c] = grow(&triangle);
    let mut path = vello::kurbo::BezPath::new();
    path.move_to(a);
    path.line_to(b);
    path.line_to(c);
    path.close_path();

    scene.fill(
        peniko::Fill::NonZero,
        // The points are already in device space.
        vello::kurbo::Affine::IDENTITY,
        crate::scene::color(triangle.average_colour()),
        None,
        &path,
    );
}

/// Grows a triangle about its centroid by a fixed distance in device pixels.
fn grow(triangle: &pdf_render::Triangle) -> [vello::kurbo::Point; 3] {
    let [a, b, c] = triangle.points;
    let centre = pdf_render::Point::new((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0);
    triangle.points.map(|point| {
        let (dx, dy) = (point.x - centre.x, point.y - centre.y);
        let length = dx.hypot(dy);
        if length <= f32::EPSILON {
            return vello::kurbo::Point::new(f64::from(point.x), f64::from(point.y));
        }
        vello::kurbo::Point::new(
            f64::from(point.x + dx / length * SEAM_OVERLAP),
            f64::from(point.y + dy / length * SEAM_OVERLAP),
        )
    })
}
