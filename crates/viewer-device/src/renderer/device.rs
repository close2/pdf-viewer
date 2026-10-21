//! One window frame drawn — on the device, unless it refused these pages before or the processor
//! was predicted the faster lane — and the device's scene built while nobody waits (ADR 1898).

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;

use pdf_render::{DisplayList, TargetSpec};
use render_raster::{FrameCost, PresentFrame, QuorraWindowRenderer, WindowTextures};

use super::lanes::{Kept, remembered, upload_lane};
use super::processor::processor_half;
use super::{Done, Finished, Job, Pair, Prepared};

/// One job, drawn — on the device, or on the processor where the device refused.
#[expect(
    clippy::too_many_lines,
    reason = "one frame's two lanes in the order they are tried, each with what it reports; the \
              sequence is the function, and splitting it would hand each half the other's locals"
)]
pub(super) fn draw(
    renderer: &mut QuorraWindowRenderer,
    job: Job,
    done: &Sender<Finished>,
    waker: Option<&crate::Wake>,
) -> Done {
    let started = Instant::now();
    let Job {
        width,
        height,
        pages,
        overlays,
        coverage,
        reuse,
        known_refusal,
        cheaper,
        interrupt,
        on_the_processor,
        ahead,
        look: _,
        build_ahead: _,
    } = job;
    let drawn_ahead = ahead.is_some();
    let textures = reuse
        .filter(|pair| pair.fits(width, height))
        .unwrap_or_else(|| Pair::new(renderer, width, height));
    // Which lane draws this frame's coverage, decided from this frame's magnification by the
    // thread that knows it (`crate::lane::coverage_for`) and carried here.
    renderer.set_coverage(coverage);
    let borrowed: Vec<&DisplayList> = overlays.iter().collect();
    let placed: Vec<(&Arc<DisplayList>, TargetSpec)> = pages
        .iter()
        .map(|placed| (&placed.list, placed.target))
        .collect();
    let into = WindowTextures {
        page: &textures.page,
        chrome: &textures.chrome,
    };
    let frame = PresentFrame {
        width,
        height,
        pages: &placed,
        raster: None,
        overlays: &borrowed,
    };
    let mut refused = None;
    let mut device_tried = None;
    let mut device_drew = None;
    let mut processor_drew = None;
    let mut finished_ahead = false;
    let mut abandoned = false;
    // **A refusal already known is not asked again** (ADR 1821): the device's attempt is the
    // whole scene built and priced before it says no, and it would say the same thing — of these
    // pages at this magnification, or of every frame of a lane the adapter refused (ADR 1968).
    // **Nor is a frame the processor was predicted to draw faster** (ADR 1831).
    let known = known_refusal.as_ref().map(|known| known.of);
    let (fell_back, refused_now) = if let Some(known) = known_refusal {
        (Some(known.said), None)
    } else if cheaper.is_some() || drawn_ahead {
        (None, None)
    } else {
        let asked = Instant::now();
        match renderer.render(frame, into) {
            Ok(()) => {
                if !matches!(
                    renderer.last_frame().encode_source,
                    Some(raster_gpu::EncodeSource::Replayed)
                ) {
                    device_drew = Some(asked.elapsed());
                }
                (None, None)
            }
            Err(problem) => {
                device_tried = Some(asked.elapsed());
                (Some(problem.to_string()), remembered(&problem, coverage))
            }
        }
    };
    // **The processor's picture goes back through the device under a lane the adapter did not
    // refuse** (ADR 1968). It is one image and the window's chrome, and under the refused lane the
    // chrome is refused with it: the window's chrome missing, and the page said to be undrawable
    // although it was drawn, at every frame.
    if let Some(Kept::Lane(refused)) = known.or(refused_now) {
        renderer.set_coverage(upload_lane(refused));
    }
    if fell_back.is_some() || cheaper.is_some() || drawn_ahead {
        let drawn = processor_half(
            renderer,
            (width, height),
            (&pages, ahead),
            &borrowed,
            (&interrupt, &on_the_processor),
            (done, waker),
            into,
        );
        // A frame finished from the page drawn while it was read measures nothing about what the
        // processor charges for the page: its cost is the last part's (ADR 0657's rule).
        finished_ahead = drawn.finished_ahead;
        processor_drew = drawn.drew.filter(|_| !finished_ahead);
        abandoned = drawn.abandoned;
        refused = drawn.refused;
    }
    Done {
        coverage,
        extent: (width, height),
        textures,
        pages,
        // An abandoned frame drew nothing, and the device's last frame is somebody else's.
        cost: if abandoned {
            FrameCost::default()
        } else {
            renderer.last_frame()
        },
        fell_back,
        refused_now,
        known,
        device_tried,
        processor_drew,
        device_drew,
        chose: cheaper,
        finished_ahead,
        abandoned,
        refused,
        function_refusals: renderer.last_function_paints().refusals().to_vec(),
        pipelines: renderer.startup().pipeline_compilation,
        started,
        finished: Instant::now(),
    }
}

/// The device's scene of the pages on hand at the view they were shown at, built and not drawn
/// (ADR 1898) — `None` where the device refused, and silent for `retained::draw_whole_page`'s
/// reason: nobody asked for it, and the next frame of these pages asks the device itself and hears
/// the refusal then, where [`Refusals`] keeps it.
///
/// [`Refusals`]: super::lanes::Refusals
pub(super) fn prepare(
    renderer: &mut QuorraWindowRenderer,
    showing: &[crate::stale::Placed],
    (width, height): (u32, u32),
) -> Option<Prepared> {
    if showing.is_empty() {
        return None;
    }
    let placed: Vec<(&Arc<DisplayList>, TargetSpec)> = showing
        .iter()
        .map(|placed| (&placed.list, placed.target))
        .collect();
    let cost = renderer
        .prepare(PresentFrame {
            width,
            height,
            pages: &placed,
            raster: None,
            overlays: &[],
        })
        .ok()?;
    Some(Prepared {
        pages: showing.to_vec(),
        cost,
    })
}

/// The lane a step from the frame just drawn takes, which the idle build is drawn on (ADR 2134):
/// `crate::lane::lane_for`'s for a moved view — the session's lane where it is fixed, the compute
/// lane on a device worth the name, and on a software adapter the magnification rule's, which the
/// frame just drawn was drawn on (`drawn`). A step to a magnification the atlas has drawn before
/// takes the processor's lane instead, and walks whole, as it does today.
pub(super) fn next_step_lane(
    renderer: &QuorraWindowRenderer,
    choice: crate::lane::CoverageChoice,
    drawn: raster_gpu::Coverage,
) -> raster_gpu::Coverage {
    match choice {
        crate::lane::CoverageChoice::Fixed(lane) => lane,
        crate::lane::CoverageChoice::Auto => {
            if crate::lane::software_adapter(renderer.adapter_description()) {
                drawn
            } else {
                raster_gpu::Coverage::Compute
            }
        }
    }
}

/// The idle build (ADR 2134): the page on hand walked again at the view it was shown at with no
/// line culled, drawn into a texture nobody presents, so that the next view of it takes its lines
/// in runs from a stream already on the device (ADR 2056). `target` keeps that texture for the
/// window's size. Answers whether it built, and is silent for [`prepare`]'s reason: nobody asked
/// for it, and a refusal here is the device's answer the next frame hears for itself.
pub(super) fn build_whole(
    renderer: &mut QuorraWindowRenderer,
    (showing, lane): (&[crate::stale::Placed], raster_gpu::Coverage),
    (width, height): (u32, u32),
    target: &mut Option<(u32, u32, raster_gpu::wgpu::Texture)>,
) -> bool {
    if showing.is_empty() || width == 0 || height == 0 {
        return false;
    }
    // Under the lane the step it is for will take (`next_step_lane`): an encode is kept for the
    // view after it only on the same lane (raster's `EncodeKey::replay_compatible`).
    renderer.set_coverage(lane);
    let placed: Vec<(&Arc<DisplayList>, TargetSpec)> = showing
        .iter()
        .map(|placed| (&placed.list, placed.target))
        .collect();
    if target
        .as_ref()
        .is_none_or(|(w, h, _)| (*w, *h) != (width, height))
    {
        *target = Some((
            width,
            height,
            renderer.layer_texture("the idle build", width, height),
        ));
    }
    let Some((_, _, texture)) = target.as_ref() else {
        return false;
    };
    matches!(
        renderer.build_whole(
            PresentFrame {
                width,
                height,
                pages: &placed,
                raster: None,
                overlays: &[],
            },
            texture,
        ),
        Ok(Some(_))
    )
}
