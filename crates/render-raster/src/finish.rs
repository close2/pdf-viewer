//! A page finished over its read-back: the passes ISO 32000-2 puts after compositing, run by both
//! of this backend's routes to a picture (ADR 3701).
//!
//! Some pages are not finished when the device has composited their marks. §11.4.7 converts a
//! page group's composited result out of its own blending space — two lists on one geometry for a
//! four-component space, a curve for a one-component and a cube for a three-component CIE-based
//! one; §10.8.3's simulated press multiplies spot planes into the process inks; and §10.5's
//! transfer functions map the finished components, chosen per pixel by §11.7.5.2's topmost
//! elementary object. `pdf_render` states each of those as a pass over a raster, and the CPU
//! oracle runs the same passes over its own (trap 2).
//!
//! **One sequence, two callers.** [`crate::QuorraRasterizer`]'s `rasterize` draws a page into a
//! read-back and calls [`finish`]; the window's present (`present::FrameSlot`) does the same for a
//! page that [`needs_finishing`] and puts the finished picture into its scene as an image in the
//! page's place. Before ADR 3701 the window translated the list's commands alone, and a page under
//! `/TR` showed the file's colours before §10.5 rather than the ones the device is to receive
//! (review 2248, B01 and B02).

use pdf_render::{DisplayList, Medium, Selection, TargetSpec, Transform};
use raster_scene::ResourceId;

use crate::{FrameCost, FunctionPaints, QuorraRasterError, cache, demultiply, premultiply};

/// Which pass a list drawn for [`finish`] belongs to, so a caller can account for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Plane {
    /// A plane of ink — the page itself, §11.4.7's black list or one of §10.8.3's spot planes —
    /// whose cost is the page's.
    Ink,
    /// §11.7.5.2's object shapes, which `pdf_render::resolve_transfers` and
    /// `pdf_render::resolve_press_transfers` draw to choose each pixel's function, and whose cost
    /// is not a page's.
    Shapes,
}

/// Whether the page's composited components leave the device through a pass of §11.4.7 or
/// §10.8.3 — a four-component pair, a curve, a cube or spot planes — which must run before the
/// medium is put under the page.
pub(crate) fn converts(list: &DisplayList) -> bool {
    list.blending().zip(list.black()).is_some()
        || list.grey_curve().is_some()
        || list.colour_cube().is_some()
        || list.separation().is_some()
        || list.press_conversion().is_some()
}

/// Whether a picture of the page the device composited is not yet the page: [`converts`], or
/// §10.5's transfer functions still to apply (§11.7.5.2).
pub(crate) fn needs_finishing(list: &DisplayList) -> bool {
    converts(list) || list.transfers().is_some()
}

/// Finishes `data`, the page `list` drawn at `target` as straight-alpha RGBA8 by `draw`, in the
/// order the clauses put the passes: §11.4.7's and §10.8.3's conversions over the premultiplied
/// raster, §14.11.2.1's crop, §11.4.7's medium under the result, and §10.5's transfer functions
/// last of all on the screen — on §10.8.3's simulated press they go on the planes, before the
/// conversion (ADR 3769). `imposed` says the device already composited the page onto its medium and crop
/// (ADR 2150), which it does only for a page that does not [`convert`](converts).
///
/// `draw` is asked for every further list at the same target: §11.4.7's black list and §10.8.3's
/// spot planes as [`Plane::Ink`], §11.7.5.2's shapes as [`Plane::Shapes`].
///
/// # Errors
///
/// Whatever `draw` refused.
pub(crate) fn finish(
    list: &DisplayList,
    target: TargetSpec,
    medium: Medium,
    data: &mut [u8],
    imposed: bool,
    mut draw: impl FnMut(&DisplayList, Plane) -> Result<Vec<u8>, QuorraRasterError>,
) -> Result<(), QuorraRasterError> {
    // ISO 32000-2 §11.4.7 puts a colour space under the whole page — "[a]ll page-level
    // compositing shall be done in the default blending colour space of the page, and the
    // entire result shall then ... be converted to the native colour space of the output
    // device before being composited with the context-dependent backdrop". Where that space has
    // four components §11.3.4 makes the compositing formula per component, so the page arrives as
    // *two* lists on the same geometry — the additive complements of cyan, magenta and yellow,
    // and the complement of black — and the second is drawn against the same device before
    // `pdf_render::blending` puts the pair back together (`raster-gpu/tests/two_rasters.rs`,
    // `doc/QUORRA_FEEDBACK.md` section 17.1).
    let four_components = list.blending().zip(list.black());
    // The one-component form of the same clause — a `CalGray` or `ICCBased` 'GRAY' page group,
    // whose composited component leaves through a curve (`pdf_render::blending::GreyCurve`) — and
    // the three-component form, through a cube (`pdf_render::blending::ColourCube`).
    // §10.8.3 on a page whose group composites in one or three components: the list's result is
    // converted onto the simulated press's four inks in place of its curve or cube, and is from
    // then on the pair a four-component page is drawn as (ADR 3857).
    let onto_press = list
        .press_conversion()
        .filter(|_| four_components.is_none());
    let one_component = list.grey_curve().filter(|_| onto_press.is_none());
    let three_components = list.colour_cube().filter(|_| onto_press.is_none());
    // Without a press, such a page's list is its own process separation and the spot planes are
    // multiplied into it after the curve or the cube (ADR 1329).
    let beside = list
        .separation()
        .filter(|_| four_components.is_none() && onto_press.is_none());
    // §14.11.2.1's clip, which belongs to the page rather than to the medium. `None` for a
    // page-sized target, where the raster's own edge is already the boundary.
    let crop = pdf_render::crop_area(list, target);

    // Both passes are needed premultiplied: `blending::resolve` divides each channel by the
    // pixel's own alpha to recover the ink, and `impose_on_medium` composites the medium in the
    // space where that composite is exact. The device hands back straight alpha, so the round
    // trip is here — and only where something below needs it, because it is lossy at a partly
    // transparent pixel.
    if !imposed && (converts(list) || medium.marks_anything() || crop.is_some()) {
        premultiply(data);
        let pair = match (four_components, onto_press) {
            (Some((space, black)), _) => {
                let mut ink = draw(black, Plane::Ink)?;
                premultiply(&mut ink);
                Some((space, ink))
            }
            // §11.7.4.2: the group's result "converted to the device colour space", which is
            // the press's pair from here on (`pdf_render::press_conversion::split`).
            (None, Some(conversion)) => Some((
                conversion.space(),
                pdf_render::split_onto_press(data, conversion.inks()),
            )),
            (None, None) => None,
        };
        if let Some((space, mut ink)) = pair {
            // §10.8.3's spot planes: every one is one more whole render against the same device.
            let mut spots = match list.separation() {
                Some(separation) => spot_planes(separation, &mut draw)?,
                None => Vec::new(),
            };
            // ISO 32000-2 §10.5 on §10.8.3's simulated press, a CMYK device: "Transfer functions
            // shall always operate in the native colour space of the output device", so each
            // plane takes its own function here, before steps b) to d) compose the planes, and
            // not on the composed colour below (`pdf_render::resolve_press_transfers`, ADR 3769).
            if let Some(channel) = list
                .transfers()
                .filter(|channel| channel.device() == pdf_render::TransferDevice::SimulatedPress)
            {
                let mut planes: Vec<&mut [u8]> = spots.iter_mut().map(Vec::as_mut_slice).collect();
                pdf_render::resolve_press_transfers(
                    channel,
                    list,
                    target,
                    data,
                    &mut ink,
                    &mut planes,
                    |shapes| draw(shapes, Plane::Shapes),
                )?;
            }
            match list.separation() {
                // §10.8.3's simulated press on a page naming a spot colourant: steps b) to d)
                // multiply the spot planes in over the read-back (`pdf_render::separation`,
                // ADR 1317).
                Some(separation) => {
                    let spots: Vec<&[u8]> = spots.iter().map(Vec::as_slice).collect();
                    pdf_render::resolve_separation(data, &ink, &spots, space, separation);
                }
                None => pdf_render::resolve_blending(data, &ink, space),
            }
        }
        // Step b)'s matte under the list in its own components, before the curve or the cube
        // takes it out of them (`pdf_render::separation::matte`).
        if beside.is_some() {
            pdf_render::separation_matte(data);
        }
        if let Some(curve) = one_component {
            pdf_render::resolve_grey(data, curve);
        }
        if let Some(cube) = three_components {
            pdf_render::resolve_cube(data, cube);
        }
        if let Some(separation) = beside {
            let spots = spot_planes(separation, &mut draw)?;
            let spots: Vec<&[u8]> = spots.iter().map(Vec::as_slice).collect();
            pdf_render::resolve_separation_over_device(data, &spots, separation);
        }
        // The page's own ink is cut where §14.11.2.1 says it stops, before anything is put under
        // it: the crop is about what the page may show, and a pass that ran after the composite
        // would cut the ground a window puts beside the page instead.
        if let Some(crop) = crop {
            pdf_render::crop_to_page(data, target.width, 0, crop);
        }
        // §11.4.7's page group is isolated, so the medium composites with the *result* — and
        // after the conversion above, which is where the clause puts it. Where 𝑊 stops is
        // §14.11.2.1's page boundary rather than the target's edge.
        if medium.marks_anything() {
            pdf_render::impose_within(
                data,
                target.width,
                0,
                pdf_render::page_area(list, target),
                medium,
            );
        }
        demultiply(data);
    }

    // ISO 32000-2 §11.7.5.2's transfer function, last of all because §11.7.5.3's NOTE puts it
    // there: "the current halftone and transfer function, whose values are used only when all
    // colour compositing has been completed and rasterization is being performed". The clause
    // chooses the function by the topmost elementary object covering a point rather than by the
    // colour under it, so `pdf_model` leaves it off the colours and hands the marks' shapes over
    // on the list instead; a shape list is the page's own marks with their paint replaced, and
    // the alpha that comes back *is* §11.7.5.2's object shape.
    //
    // On §10.8.3's simulated press the functions were applied to the planes above instead.
    if let Some(channel) = list
        .transfers()
        .filter(|channel| channel.device() == pdf_render::TransferDevice::Screen)
    {
        pdf_render::resolve_transfers(channel, list, target, data, |shapes| {
            draw(shapes, Plane::Shapes)
        })?;
    }
    Ok(())
}

/// §10.8.3's spot planes, each drawn as one more whole list at the page's target and read back
/// premultiplied for steps b) to d) to multiply in (ADRs 1317 and 1329).
fn spot_planes(
    separation: &pdf_render::SpotSeparation,
    draw: &mut impl FnMut(&DisplayList, Plane) -> Result<Vec<u8>, QuorraRasterError>,
) -> Result<Vec<Vec<u8>>, QuorraRasterError> {
    let mut spots = Vec::with_capacity(separation.planes().len());
    for plane in separation.planes() {
        let mut raster = draw(plane, Plane::Ink)?;
        premultiply(&mut raster);
        spots.push(raster);
    }
    Ok(spots)
}

/// The parts of a device a list is drawn through in one frame of its own.
pub(crate) struct Drawing<'a> {
    pub(crate) device: &'a mut raster_gpu::Device,
    pub(crate) caches: &'a mut cache::ResourceCaches,
    pub(crate) functions: &'a mut FunctionPaints,
    pub(crate) phases: &'a mut Vec<(&'static str, std::time::Duration)>,
}

/// One display list through the device in **one** frame, as straight-alpha RGBA8 with no medium
/// under it — or, given a `backdrop`, as the opaque bytes of the page on it (ADR 2150) — and the
/// clip residue pair the frame reported (`QuorraRasterizer::last_clip_residue`).
///
/// A frame of its own: the caches begin and settle around it and its transient resources go back
/// whether it drew or refused, so a caller holding a retained scene on the same caches discards or
/// replaces that scene first.
///
/// `cost` is filled as the call goes, so that a frame that refuses still reports what it uploaded
/// and settled (ADR 0402).
///
/// # Errors
///
/// As [`pdf_render::Rasterizer::rasterize`].
pub(crate) fn draw_once(
    parts: &mut Drawing<'_>,
    list: &DisplayList,
    target: TargetSpec,
    cost: &mut FrameCost,
    backdrop: Option<raster_gpu::Backdrop>,
) -> Result<(Vec<u8>, (u32, u32)), QuorraRasterError> {
    // Before the device is touched at all: a bound asked in one funnel and not the other is a
    // bound one lane holds and the other does not (`pdf_render::group_cost`).
    pdf_render::check_group_blit(list, target)?;

    let began = std::time::Instant::now();
    // The marks that reach the target (ADR 1841), as the window's scene takes them: §10.7.4 paints
    // only a pixel a shape's area meets, so a command left out marked no pixel here. Every list a
    // page is drawn from — its plane of black, its spot planes, its shapes for §11.7.5.2 — has the
    // page's geometry and loses the same commands, so the passes stay in step (ADR 1857).
    let culled = list.reaching(target);
    let drawn = Selection::drawn(list, culled.as_ref());
    parts.caches.begin_frame();
    let mut builder = raster_scene::SceneBuilder::new();
    let mut transient: Vec<ResourceId> = Vec::new();
    parts.functions.clear();
    let built = crate::scene::Encoder::new(
        parts.device,
        list,
        target,
        parts.caches,
        &mut transient,
        parts.functions,
    )
    .commands(&mut builder, drawn.commands());
    cost.uploads = parts
        .caches
        .stored()
        .saturating_add(u32::try_from(transient.len()).unwrap_or(u32::MAX));
    cost.scene = began.elapsed();
    // The offscreen lane's half of ADR 0423: a subdivision only the window could see would be one
    // no gate ever prints.
    cost.handover = parts.caches.handed();
    cost.outline_segments = parts.caches.segments();

    let submitted = std::time::Instant::now();
    let rendered = built.and_then(|()| {
        let scene = builder.finish();
        // The target transform is baked into every command at translation (`Encoder::placed`),
        // so the viewport itself is identity.
        let viewport =
            raster_gpu::Viewport::full(target.width, target.height, raster_scene::Affine::IDENTITY);
        let into = backdrop.map_or(
            raster_gpu::Target::Readback,
            raster_gpu::Target::ReadbackOnto,
        );
        Ok(parts.device.render(&scene, &viewport, into)?)
    });
    cost.device = submitted.elapsed();

    // Per-frame resources (clips, dashed strokes, meshes, sampled grids) go back to the budget
    // whether the frame drew or refused — every one of them, even after a failure — and the
    // frame's own error, if any, outranks a release problem.
    let settling = std::time::Instant::now();
    let mut release_error: Option<raster_gpu::DeviceError> = None;
    for id in transient.drain(..) {
        if let Err(error) = parts.device.release(id) {
            release_error.get_or_insert(error);
        }
    }
    // Frames drawn and frames refused both settle the caches: a long session must stay healthy
    // through its refusals (QUORRA_FEEDBACK.md section 2).
    let evicted = parts.caches.evict_settled(parts.device);
    cost.settle = settling.elapsed();
    evicted?;
    let frame = rendered?;
    if let Some(error) = release_error {
        return Err(error.into());
    }
    // Read after the refusals above, because these are a *drawn* frame's own numbers.
    cost.read(&frame, parts.phases);
    let counters = frame.counters();
    let residue = (counters.clip_residue_regions, counters.clip_residue_tiles);
    Ok((frame.into_raster()?.into_pixels(), residue))
}

/// One display list through the device at `target` — in one frame, or in tiles where the target
/// is wider or taller than the adapter renders (`crate::tiles`, ADR 1472).
///
/// # Errors
///
/// As [`draw_once`], and [`QuorraRasterError::Allocation`] where the target's whole raster cannot
/// be held to stitch the tiles into.
pub(crate) fn draw(
    parts: &mut Drawing<'_>,
    list: &DisplayList,
    target: TargetSpec,
    cost: &mut FrameCost,
) -> Result<(Vec<u8>, (u32, u32)), QuorraRasterError> {
    let limits = parts.device.limits();
    if target.width <= limits.max_target_size && target.height <= limits.max_target_size {
        return draw_once(parts, list, target, cost, None);
    }
    let mut residue = (0, 0);
    let stitched = crate::tiles::render_in_tiles(limits, target, cost, |tile, tile_cost| {
        let (data, tile_residue) = draw_once(parts, list, tile, tile_cost, None)?;
        residue = tile_residue;
        Ok(data)
    })?;
    Ok((stitched, residue))
}

/// A page finished for the window's scene: the pixels of its own rectangle of the window,
/// `width` by `height` at (`left`, `top`).
pub(crate) struct Finished {
    pub(crate) left: u32,
    pub(crate) top: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Vec<u8>,
}

/// The page `list` finished at `target` for the window's scene: drawn, then [`finish`]ed onto
/// `medium`'s 𝑊 within the page and transparency outside it, so that the picture goes into a
/// window's frame in the page's place over the surround the scene already holds (ADR 3701) —
/// or `None` where no pixel of the page is in the target.
///
/// **Over the page's own rectangle of the target, not the whole target.** Outside §14.11.2.1's
/// page boundary a finished page is transparency — its ink is cropped there and 𝑊 stops there —
/// so the pixels beyond it are work with no answer: a device frame and a read-back per list, the
/// passes, and the upload. The rectangle is the target under a whole-pixel translation, which
/// moves no edge, sample or glyph phase against the pixel grid (`crate::tiles`'s argument), so
/// its pixels are the target's. On the turn gate's 1600 by 1000 window `personwithdog.pdf` covers
/// under half the window, and its turn was paying for all of it (ADR 3701).
///
/// # Errors
///
/// Whatever a draw refused.
pub(crate) fn finished_page(
    parts: &mut Drawing<'_>,
    list: &DisplayList,
    target: TargetSpec,
    medium: Medium,
) -> Result<Option<Finished>, QuorraRasterError> {
    let Some((left, top, width, height)) = page_pixels(list, target) else {
        return Ok(None);
    };
    let own = TargetSpec {
        width,
        height,
        transform: target
            .transform
            .then(Transform::translate(-pixels(left), -pixels(top))),
    };
    let mut cost = FrameCost::default();
    let (mut data, _) = draw(parts, list, own, &mut cost)?;
    let within = Medium {
        page: medium.page,
        surround: pdf_render::Color::TRANSPARENT,
    };
    finish(list, own, within, &mut data, false, |plane, _| {
        let mut cost = FrameCost::default();
        draw(parts, plane, own, &mut cost).map(|(data, _)| data)
    })?;
    Ok(Some(Finished {
        left,
        top,
        width,
        height,
        pixels: data,
    }))
}

/// The whole pixels of `target` that §14.11.2.1's page boundary touches, as `(left, top, width,
/// height)`, or `None` where it touches none.
fn page_pixels(list: &DisplayList, target: TargetSpec) -> Option<(u32, u32, u32, u32)> {
    let area = pdf_render::page_area(list, target);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a coordinate clamped into 0..=limit, which a target's side keeps exact in f32"
    )]
    let within = |value: f32, limit: u32| -> u32 {
        if value.is_nan() || value <= 0.0 {
            0
        } else if value >= limit as f32 {
            limit
        } else {
            value as u32
        }
    };
    let left = within(area.min.x.min(area.max.x).floor(), target.width);
    let right = within(area.min.x.max(area.max.x).ceil(), target.width);
    let top = within(area.min.y.min(area.max.y).floor(), target.height);
    let bottom = within(area.min.y.max(area.max.y).ceil(), target.height);
    (right > left && bottom > top).then(|| {
        (
            left,
            top,
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        )
    })
}

/// A whole number of device pixels as a placement offset.
fn pixels(count: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a target's side is far below f32's exact integer range"
    )]
    let offset = count as f32;
    offset
}

/// The image transform that puts a `width` by `height` raster at (`left`, `top`) of the frame:
/// the unit square carries the image with its top row at unit y = 1 (§8.9.5), so placing the top
/// row at `top` takes a flip.
pub(crate) fn unit_square_at(left: u32, top: u32, width: u32, height: u32) -> raster_scene::Affine {
    raster_scene::Affine {
        a: pixels(width),
        b: 0.0,
        c: 0.0,
        d: -pixels(height),
        e: pixels(left),
        f: pixels(top) + pixels(height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page with no page-level pass is drawn by the window's scene as it always was: the
    /// predicate is the whole of what the window's turn path pays for ADR 3701.
    #[test]
    fn a_plain_page_needs_no_finishing() {
        let list = DisplayList::new(pdf_render::Size {
            width: 10.0,
            height: 10.0,
        });
        assert!(!needs_finishing(&list));
        assert!(!converts(&list));
    }
}
