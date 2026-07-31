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

use pdf_render::{Color, Ramp, Shading, ShadingKind, TargetSpec, Transform};
use vello::peniko;

/// How wide the transparent transition at a non-extended end is, as a fraction of the
/// ramp. Matches `render-cpu`, because the backends must agree.
const CUTOFF: f32 = 0.0005;

/// Builds a brush for a shading, or `None` for kinds the caller must draw itself.
///
/// `page_to_path` maps page space into the space the path being drawn is stated in.
///
/// # Which space a brush is positioned in
///
/// Vello encodes a brush transform as `shape transform * brush transform`, so the affine
/// returned here is read in the *path's* space, not the device's — the same convention
/// `tiny-skia` uses, and the same trap. Handing it a device-space transform applies the
/// device transform twice, which mirrors a gradient about the page's horizontal centre at
/// a scale of 1.0 and displaces it by a scale-dependent amount at every other scale.
///
/// Both backends had that defect and therefore agreed with each other, which is worth
/// remembering: two implementations agreeing is evidence only when they can fail
/// independently. The scenes that compared them used gradients running along x, where a
/// y mirror is invisible.
pub(crate) fn brush(
    shading: &Shading,
    page_to_path: Transform,
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
    let transform = crate::scene::affine(shading.transform.then(page_to_path));
    Some((peniko::Brush::Gradient(gradient), Some(transform)))
}

/// Builds the gradient stops for a ramp, honouring `/Extend`.
fn stops(ramp: &Ramp, extend: (bool, bool)) -> Vec<peniko::ColorStop> {
    let mut stops: Vec<peniko::ColorStop> = Vec::with_capacity(ramp.stops.len().saturating_add(2));

    let (low, high) = match extend {
        (true, true) => (0.0, 1.0),
        (false, true) => (CUTOFF, 1.0),
        (true, false) => (0.0, 1.0 - CUTOFF),
        (false, false) => (CUTOFF, 1.0 - CUTOFF),
    };

    if !extend.0 {
        stops.push(stop(0.0, transparent(ramp.colour_at(0.0))));
    }
    for entry in ramp.stops.iter() {
        stops.push(stop(
            (low + entry.at * (high - low)).clamp(0.0, 1.0),
            entry.colour,
        ));
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

/// Draws a mesh shading's triangles into the current layer (ISO 32000-2 §8.7.4.5.5).
///
/// The caller has already pushed a layer clipped to the shape being filled, so this needs no
/// clipping of its own. Vello has no Gouraud primitive and neither has `tiny-skia`, so the
/// mesh is rasterised once by [`pdf_render::MeshRaster`] — the clause's own interpolation at
/// each device pixel's centre — and the result is drawn as an image at 1:1.
///
/// Everything here mirrors `render-cpu` deliberately, and now mirrors it exactly rather than
/// approximately: the two backends draw the *same bytes*, where before they drew two
/// subdivisions of the same triangles that had to be tuned to agree.
pub(crate) fn fill_mesh(
    scene: &mut vello::Scene,
    triangles: &[pdf_render::Triangle],
    to_device: Transform,
    target: TargetSpec,
) {
    let Some(raster) =
        pdf_render::MeshRaster::build(triangles, to_device, target.width, target.height)
    else {
        return;
    };
    let data = peniko::ImageData {
        data: peniko::Blob::new(std::sync::Arc::new(raster.image.data.to_vec())),
        format: peniko::ImageFormat::Rgba8,
        // Straight alpha, matching `pdf_render::Image`'s documented format.
        alpha_type: peniko::ImageAlphaType::Alpha,
        width: raster.image.width,
        height: raster.image.height,
    };
    // `Low` is nearest-neighbour. The raster is at device resolution and placed at whole
    // pixels, so no sample can be interpolated — and a filter would blur the mesh's edge
    // against the transparent pixels outside it.
    let brush = peniko::ImageBrush::new(data).with_quality(peniko::ImageQuality::Low);
    scene.draw_image(
        &brush,
        vello::kurbo::Affine::translate((f64::from(raster.left), f64::from(raster.top))),
    );
}
