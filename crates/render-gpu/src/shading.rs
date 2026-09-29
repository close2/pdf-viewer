//! Turning a resolved shading into a Vello brush.
//!
//! Axial and radial shadings map onto Vello's own gradients, and a two-point radial
//! gradient is exactly PDF's model — the same correspondence the CPU backend found in
//! `tiny-skia`. That both rasterisers express these natively, in the same terms, is the
//! evidence that the display list's neutral form is the right one.
//!
//! Mesh shadings have no equivalent in either and are drawn as triangles by the caller.
//!
//! # `/Extend`, and what Vello's ramp admits
//!
//! Where a shading does not extend it paints nothing beyond that end, and neither
//! rasteriser's spread modes can say so. The construction is `render-cpu`'s (ADR 1387): the
//! ramp's stops are handed over at their own positions — ISO 32000-2 §8.7.4.5.3 places a
//! colour by its projection onto the axis alone, so a stop moved along the ramp is a colour
//! moved along the page — and a non-extended end is a hard stop to transparency, which `Pad`
//! then repeats.
//!
//! Vello does not evaluate stops per pixel. It bakes them into a ramp of 512 texels
//! (`vello_encoding`'s `make_ramp`), texel `i` holding the colour at `i / 511`, and the fine
//! shader clamps the parameter under `Pad` and reads texel `round(t · 511)`. Two things follow:
//!
//! - **A hard stop at 0 holds.** Texel 0 takes the first of the stops at offset 0, which is
//!   the transparent one, so the start is cut — half a texel inside the ramp, where every hard
//!   stop of this ramp lands.
//! - **A hard stop at 1 does not.** Texel 511 also takes the *first* stop at its offset, which
//!   would be the end colour, and `Pad` would then paint it forever. So the end's pair is
//!   placed [`END_NUDGE`] below 1: every texel before the last is still interpolated from the
//!   ramp's own stops, and the last is transparent — the same half texel as the start.
//!
//! What no construction here can remove is that quantisation: on this backend a stop lands
//! within half a texel, 1/1022 of the axis, of where the clause puts it. The CPU backend is the
//! oracle and is exact; this one is compared with it at that bound.
use pdf_render::{Color, Ramp, Shading, ShadingKind, TargetSpec, Transform};
use vello::peniko;

/// How far below 1 a non-extended end's hard stop is placed, as a fraction of the ramp.
///
/// Any offset strictly between texel 510's position (510/511) and 1 makes texel 511 the
/// transparent one; one part in a million is far inside that interval for any ramp width
/// Vello could choose, and far below a texel, so no interpolated texel moves.
const END_NUDGE: f32 = 1e-6;

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
    // Table 77's `/Background` is a colour the shading answers outside its own bounds, which
    // `peniko::Extend` cannot say: it pads, repeats or reflects. [`fill_with_background`] is
    // the answer and it is a *fill*'s door, so a shading arriving here with one is a stroke —
    // and refusing it loudly keeps the shortfall a report rather than a silently missing wash,
    // exactly as `render_cpu::shading::shader` does.
    if shading.background.is_some() {
        return None;
    }
    let gradient = match shading.kind.as_ref() {
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
///
/// The ramp's stops keep their positions; a non-extended end adds a transparent stop at the
/// end's own position — at 1 less [`END_NUDGE`] for the far end, for the reason the module
/// comment gives.
fn stops(ramp: &Ramp, extend: (bool, bool)) -> Vec<peniko::ColorStop> {
    let mut stops: Vec<peniko::ColorStop> = Vec::with_capacity(ramp.stops.len().saturating_add(2));

    if !extend.0 {
        stops.push(stop(0.0, transparent(ramp.colour_at(0.0))));
    }
    let end = if extend.1 { 1.0 } else { 1.0 - END_NUDGE };
    for entry in ramp.stops.iter() {
        stops.push(stop(entry.at.clamp(0.0, end), entry.colour));
    }
    if !extend.1 {
        stops.push(stop(end, transparent(ramp.colour_at(1.0))));
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
    patches: Option<&pdf_render::PatchMesh>,
    ramp: Option<&Ramp>,
    to_device: Transform,
    target: TargetSpec,
) {
    let Some(raster) = pdf_render::MeshRaster::build(
        triangles,
        patches,
        ramp,
        to_device,
        target.width,
        target.height,
    ) else {
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

/// Draws a shading washed with ISO 32000-2 §8.7.4.3 Table 77's `/Background` into the current
/// layer.
///
/// The third member of [`fill_mesh`]'s and [`fill_radial`]'s family and the general one, and it
/// mirrors `render-cpu` for the same reason those two do: [`pdf_render::ShadingRaster`]
/// evaluates the clause once, so the backends draw the *same bytes*. §11.6.7 puts the wash
/// inside the pattern's implicit transparency group — "filled with the specified background
/// colour before the sh operator is invoked" — so the two are one painting operation, and the
/// caller has already pushed a layer clipped to the shape that composites it.
pub(crate) fn fill_with_background(
    scene: &mut vello::Scene,
    shading: &Shading,
    page_to_device: Transform,
    within: (u32, u32, u32, u32),
    target: TargetSpec,
) {
    let Some(raster) = pdf_render::ShadingRaster::build(
        shading,
        page_to_device,
        within,
        (target.width, target.height),
    ) else {
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
    // `Low` is nearest-neighbour, for [`fill_mesh`]'s reason: the raster is at device
    // resolution and placed at whole pixels, so no sample can be interpolated.
    let brush = peniko::ImageBrush::new(data).with_quality(peniko::ImageQuality::Low);
    scene.draw_image(
        &brush,
        vello::kurbo::Affine::translate((f64::from(raster.left), f64::from(raster.top))),
    );
}

/// Whether a radial shading is §8.7.4.5.4's cone, where the clause and a gradient differ.
///
/// The condition and its proof live in `render_cpu::shading::is_a_cone` and this is the same
/// expression: the leading coefficient of the quadratic whose roots are the blend circles
/// through a point is `|c1 - c0|^2 - (r1 - r0)^2`, and only where it is **positive** can a
/// point lie on two blend circles at once. It is repeated rather than shared because a
/// backend does not depend on its sibling; both are three lines over
/// [`pdf_render::blend_parameter`]'s own arithmetic, which is where the clause is stated.
#[must_use]
pub(crate) fn is_a_cone(
    start: pdf_render::Point,
    start_radius: f32,
    end: pdf_render::Point,
    end_radius: f32,
) -> bool {
    let (dx, dy, dr) = (end.x - start.x, end.y - start.y, end_radius - start_radius);
    dr.mul_add(-dr, dx.mul_add(dx, dy * dy)) > 0.0
}

/// Draws a radial cone exactly into the current layer (ISO 32000-2 §8.7.4.5.4).
///
/// The counterpart of [`fill_mesh`] one shading type over, and it mirrors `render-cpu` for
/// the same reason: [`pdf_render::RadialRaster`] evaluates the clause once, so the two
/// backends draw the *same bytes* rather than two approximations that have to be tuned to
/// agree. The caller has already pushed a layer clipped to the shape, so `within` is the
/// shape's device bounds and nothing here clips.
///
/// Returns `false` when the raster is empty, so the caller can fall back to the gradient
/// rather than drawing nothing.
pub(crate) fn fill_radial(
    scene: &mut vello::Scene,
    radial: pdf_render::Radial<'_>,
    to_device: Transform,
    within: (u32, u32, u32, u32),
) -> bool {
    let Some(raster) = pdf_render::RadialRaster::build(radial, to_device, within) else {
        return false;
    };
    let data = peniko::ImageData {
        data: peniko::Blob::new(std::sync::Arc::new(raster.image.data.to_vec())),
        format: peniko::ImageFormat::Rgba8,
        // Straight alpha, matching `pdf_render::Image`'s documented format.
        alpha_type: peniko::ImageAlphaType::Alpha,
        width: raster.image.width,
        height: raster.image.height,
    };
    // `Low` is nearest-neighbour, for `fill_mesh`'s reason: the raster is at device
    // resolution and placed at whole pixels, so no sample can be interpolated.
    let brush = peniko::ImageBrush::new(data).with_quality(peniko::ImageQuality::Low);
    scene.draw_image(
        &brush,
        vello::kurbo::Affine::translate((f64::from(raster.left), f64::from(raster.top))),
    );
    true
}
