//! The settled view drawn again at twice the window's resolution while nobody waits (ADR 0699), and
//! the bound on what it may be predicted to cost (ADRs 0761, 1289).

use std::sync::Arc;
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, TargetSpec, Transform};
use raster_gpu::wgpu;
use render_raster::{PresentFrame, QuorraWindowRenderer};

/// The settled view drawn again at twice the window's resolution, presented box-filtered
/// down so abutting fills stop leaking backdrop at their seams (ADR 0699).
///
/// It lives **beside** the base rather than replacing it: recording it through
/// [`crate::stale::Stale::settled`] would make the next view change refuse to reproject
/// (`Refusal::Resized` — the settled size would be twice the asked one), so the sharp
/// texture is a presentation substitute and the stale machinery never sees it.
#[derive(Debug)]
pub(super) struct Sharpened {
    /// The pages it depicts, at their **window-resolution** targets — the identity the
    /// presenter matches against [`Shown::pages`] before substituting.
    ///
    /// [`Shown::pages`]: super::Shown::pages
    pub(super) pages: Vec<crate::stale::Placed>,
    /// The window size it was drawn for, in the window's own pixels (the texture is twice
    /// this in each axis).
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) texture: wgpu::Texture,
    /// What the pass cost on the render thread, for the trace.
    pub(super) took: Duration,
}

/// The most the settled view's sharp pass may be *predicted* to cost before it is declined.
///
/// The prediction is four times the last frame this thread had to build — the pass draws
/// the same commands at four times the pixels — and the pass is uninterruptible: raster
/// draws it in one call and one submission, so for its whole length a newly arrived view
/// change waits behind it, and on DirectX 12 so does every present the event thread
/// issues, because a present is a queue operation and executes after whatever was
/// submitted before it. This budget bounds that stall; it is the cost ADR 0699 already
/// priced and accepted (~350 ms of idle-thread time on the worst page), written down as a
/// bound rather than left open-ended.
///
/// **The number is a choice between two measured machines, not a derivation** (ADR 0761).
/// On the owner's Radeon 890M a settled zoom frame of the worst page builds in 53–66 ms
/// (`doc/todo/46`), predicting the pass at ~270 ms, inside the budget — it runs. On the
/// owner's Windows Intel UHD (DX12, the 2026-08-31 trace) the first frame of the same
/// page built in 431.6 ms, and the unbounded pass measured **8 867.6 ms**, during which
/// the window's presents blocked for up to 5.25 s and a zoom the person had already made
/// waited eight seconds for its first real frame. Any bound between those two populations
/// decides both correctly.
pub(super) const SHARP_STALL_BUDGET: Duration = Duration::from_millis(400);

/// Whether the sharp pass is predicted to land inside [`SHARP_STALL_BUDGET`].
///
/// `last_built` is the cost of the last frame this thread built rather than replayed.
/// `None` — no frame built yet — declines, because a pass nobody asked for is not worth
/// starting with no evidence it is affordable; in practice the pass is only reachable
/// after a first job, which always builds.
pub(super) fn sharp_pass_affordable(last_built: Option<Duration>) -> bool {
    sharp_pass_cost(last_built).is_some_and(|pass| pass <= SHARP_STALL_BUDGET)
}

/// What the sharp pass is predicted to cost: four times the last frame this thread built,
/// because it draws the same commands at four times the pixels (ADR 0761). `None` before any
/// frame has been built.
///
/// It is both the number [`SHARP_STALL_BUDGET`] bounds and how long the view must have been
/// still before the pass begins (ADR 1289), and the prediction is an over-estimate on the
/// compute lane — the worst page's pass measured 57–63 ms against frames of 55–62 — which
/// errs towards waiting longer, the side that costs a picture nobody asked for a little later
/// rather than a picture somebody did.
pub(super) fn sharp_pass_cost(last_built: Option<Duration>) -> Option<Duration> {
    last_built.map(|frame| frame.saturating_mul(4))
}

/// The settled view again, at twice the window's resolution, for the presenter to show
/// box-filtered down (ADR 0699).
///
/// `None` where the device refused — most plausibly `2 × window` past the adapter's
/// texture limit — and silent for `draw_whole_page`'s reason: this is a picture nobody
/// asked for, and the caller's `sharpened` flag keeps a refusal from spinning.
pub(super) fn draw_sharp(
    renderer: &mut QuorraWindowRenderer,
    showing: &[crate::stale::Placed],
    size: (u32, u32),
    coverage: crate::arguments::CoverageChoice,
) -> Option<Sharpened> {
    let began = Instant::now();
    let (width, height) = (size.0.saturating_mul(2), size.1.saturating_mul(2));
    if width == 0 || height == 0 {
        return None;
    }
    let doubled: Vec<(&Arc<DisplayList>, TargetSpec)> = showing
        .iter()
        .map(|placed| {
            (
                &placed.list,
                TargetSpec {
                    width,
                    height,
                    transform: placed.target.transform.then(Transform::scale(2.0, 2.0)),
                },
            )
        })
        .collect();
    // The sharp pass is a moved view by construction — twice the shown transform — so
    // `auto` takes the compute lane, which also keeps the 2× tiles out of the glyph
    // atlas (the thrash ADR 0699 priced) because that lane has no atlas at all. On a
    // software adapter the dispatch loses (`lane_for`'s reason) and the pass keeps the
    // processor's lanes.
    renderer.set_coverage(match coverage {
        crate::arguments::CoverageChoice::Fixed(lane) => lane,
        crate::arguments::CoverageChoice::Auto => {
            if crate::surface::software_adapter(renderer.adapter_description()) {
                raster_gpu::Coverage::Cpu
            } else {
                raster_gpu::Coverage::Compute
            }
        }
    });
    // A fresh texture per pass rather than a pooled one: the one it replaces is still on
    // the window until the event thread adopts this, and a sharp pass happens once per
    // settled view rather than once per frame — the pool that keeps `Pair` cheap would
    // here be a second four-window allocation held for ever.
    let texture = renderer.layer_texture("the sharpened page", width, height);
    renderer
        .render_page_only(
            PresentFrame {
                width,
                height,
                pages: &doubled,
                raster: None,
                overlays: &[],
            },
            &texture,
        )
        .ok()?;
    Some(Sharpened {
        pages: showing.to_vec(),
        width: size.0,
        height: size.1,
        texture,
        took: began.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two machines [`SHARP_STALL_BUDGET`] was chosen between (ADR 0761). The 890M's
    /// settled zoom frame of the worst page builds in 53–66 ms, so its pass is predicted
    /// inside the budget and runs.
    #[test]
    fn the_sharp_pass_runs_where_the_frame_it_doubles_was_cheap() {
        assert!(sharp_pass_affordable(Some(Duration::from_millis(66))));
    }

    /// The Windows Intel UHD's first frame of the same page built in 431.6 ms, and its
    /// unbounded sharp pass measured 8 867.6 ms — the pass is declined there.
    #[test]
    fn the_sharp_pass_is_declined_where_the_frame_itself_took_hundreds_of_milliseconds() {
        assert!(!sharp_pass_affordable(Some(Duration::from_micros(431_600))));
    }

    /// No built frame means no prediction, and a pass nobody asked for is not started on
    /// no evidence.
    #[test]
    fn the_sharp_pass_waits_for_a_built_frame_to_predict_from() {
        assert!(!sharp_pass_affordable(None));
    }
}
