//! Which coverage lane a frame is drawn with, and what a swapchain state means for the tick after.
//!
//! Decisions a window with a device makes about its device, from what it is already showing and
//! what the device said, and testable without one (ADR 2052): the render thread asks them for the
//! frames it draws, and the host asks them for what it presents.

use pdf_render::Transform;

/// The magnification past which raster's GPU coverage lane is the cheaper one.
///
/// **Derived, not tuned.** raster keeps a glyph's rasterised coverage in an atlas until
/// the glyph exceeds 128 device pixels; past that it rasterises the glyph again on
/// every frame, which is where its cost stops being flat (its ADR 0016). The
/// magnification that happens at is `128 ÷ the height of the text`, so body text of 10
/// to 12 points crosses it between 10.7× and 13×. Ten is the low end of that band,
/// chosen because being early costs a fraction of a millisecond and being late costs
/// ten — measured on this machine at 0.44 ms per frame at 8× against 4.4 ms at 12×
/// (`doc/quorra-gpu-coverage.md`).
///
/// A page whose text is much larger or much smaller than a book's crosses it somewhere
/// else, and the honest way to do better would be to ask the display list what size its
/// text is rather than to move this number.
const GPU_COVERAGE_MAGNIFICATION: f32 = 10.0;

/// Which coverage lane `--coverage` pinned, or the magnification policy where it
/// did not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageChoice {
    /// The magnification policy ([`coverage_for`]'s own).
    Auto,
    /// One lane, held for the whole session.
    Fixed(raster_gpu::Coverage),
}

/// Which lane `auto` picks for one frame, from what the window is already showing.
///
/// Three cases, each a measurement (raster's ADR 0080/0081, this tree's ADR 0700):
///
/// - **The view moved** — any coefficient of the arrangement's transform differs from
///   the shown frame's — so every cached tile is cold, which is the regime the compute
///   lane wins on both page shapes measured (a 58k-fill page: ~150 ms against ~270; a
///   dense text page's cold sweep: 0.93 ms of encode against 8.84).
/// - **The view is the one being shown** — a chrome-only ask — so the frame replays
///   its retained encode, *if* the lane does not move: raster keys a retained encode on
///   the lane, so the choice is **sticky** here, and flipping it would turn a selection
///   change into a full re-encode.
/// - **There is no shown frame** — the launch path — which keeps the lane the
///   time-to-first-page gates were measured on ([`coverage_for`]'s magnification rule);
///   the compute lane's first frame pays its pipeline compile and its segment
///   residency, and the launch path pays for nothing it can defer.
///
/// The sampled [`Gpu`](raster_gpu::Coverage::Gpu) lane is deliberately absent from the
/// moved-view case: the compute lane beats it on the cold sweep (0.93 against 9.8 ms),
/// matches it held at 100×, and is exact where §10.7.4 records it non-conformant. It
/// stays reachable by `--coverage gpu`, and in the launch rule until a first-frame
/// measurement moves it.
#[must_use]
pub fn lane_for(
    asked: Transform,
    shown: Option<Transform>,
    last: Option<raster_gpu::Coverage>,
    seen_by_the_atlas: &[[u32; 4]],
    software: bool,
    choice: CoverageChoice,
) -> raster_gpu::Coverage {
    if let CoverageChoice::Fixed(lane) = choice {
        return lane;
    }
    // A software adapter runs the dispatch on the processor without the scanline
    // rasteriser's shape, and loses: 600 against 229 ms on the worst page's zoom step
    // under llvmpipe. The compute lane is for machines with a device worth the name.
    if software {
        return coverage_for(asked, choice);
    }
    match (shown, last) {
        (Some(drawn), Some(lane)) if same_transform(drawn, asked) => lane,
        // A magnification the atlas has drawn before is a magnification it still
        // holds: raster's tiles are keyed by the linear part and evicted only by a
        // repack, so a revisit — zooming back to the fit, the other window size of a
        // pair — hits, and the hit is worth 69 against the compute lane's 130 ms on
        // the worst page (the loop measurement in ADR 0700). A repack in between costs
        // one cold CPU frame, which is the bounded downside of remembering.
        (Some(_), _) if seen_by_the_atlas.contains(&linear_bits_of(asked)) => {
            raster_gpu::Coverage::Cpu
        }
        (Some(_), _) => raster_gpu::Coverage::Compute,
        _ => coverage_for(asked, choice),
    }
}

/// The linear part as the bits the atlas keys tiles by — the same reading raster
/// makes, so "seen" here and "resident" there mean the same magnification.
#[must_use]
pub fn linear_bits_of(transform: Transform) -> [u32; 4] {
    [
        transform.a.to_bits(),
        transform.b.to_bits(),
        transform.c.to_bits(),
        transform.d.to_bits(),
    ]
}

/// Whether this adapter is a software rasteriser, read from the description raster
/// formats as `"{name} ({device_type:?}, {backend:?})"` (their `construct.rs`) — a
/// string test with a named source, to be replaced by a typed accessor when raster
/// grows one.
#[must_use]
pub fn software_adapter(description: &str) -> bool {
    description.contains("(Cpu,")
}

/// Bit equality of the six coefficients — the same reading raster's retained-frame key
/// makes, so "the view moved" here and "the encode survives" there cannot disagree.
pub(crate) fn same_transform(a: Transform, b: Transform) -> bool {
    [a.a, a.b, a.c, a.d, a.e, a.f]
        .iter()
        .zip([b.a, b.b, b.c, b.d, b.e, b.f])
        .all(|(a, b)| a.to_bits() == b.to_bits())
}

/// Which coverage lane the next frame should be drawn with.
///
/// Per frame, because the crossover is a magnification and a person zooming crosses it;
/// decided *here*, because this is the only crate that knows what magnification the
/// frame is at. The transform's determinant is the magnification squared — the page
/// transform is a scale, a y flip and a translation, and §7.7.3.3's page rotation puts
/// the same factor into `b` and `c` instead of `a` and `d` — so its square root is the
/// number to compare, and it is right for a rotated page as well.
#[must_use]
pub fn coverage_for(transform: Transform, choice: CoverageChoice) -> raster_gpu::Coverage {
    if let CoverageChoice::Fixed(lane) = choice {
        return lane;
    }
    let magnification = transform
        .a
        .mul_add(transform.d, -(transform.b * transform.c))
        .abs()
        .sqrt();
    if magnification >= GPU_COVERAGE_MAGNIFICATION {
        raster_gpu::Coverage::Gpu
    } else {
        raster_gpu::Coverage::Cpu
    }
}

/// What a swapchain state means for the tick after it, which is the whole of what a host decides.
///
/// A free function over an enum rather than a `match` inside `quorra`'s `App::put_up`, for one reason: the
/// decision is now made from **two** inputs — the state raster reports and whatever the device
/// said to its uncaptured-error handler on the way — and a decision with two inputs is one worth
/// being able to test without a graphics device. Its tests are at the foot of this file.
#[derive(Debug, PartialEq, Eq)]
pub enum Swapchain {
    /// Nothing was presented, and the window should ask for a frame again at once: the swapchain
    /// has been replaced and the next acquire will find a fresh one.
    AskAgain,
    /// Nothing was presented and nothing is owed. The next tick tries, which is soon enough for a
    /// window that is occluded or whose queue is momentarily full.
    Wait,
    /// The swapchain could not be provided for a reason trying again does not clear, and this is
    /// the sentence to say about it.
    Refused(String),
}

/// [`Swapchain`]'s decision, with the device's own words where it left any.
///
/// **`Validation` names its cause**, where `swapchain validation failed` would be four words that
/// name no cause and suggest no action — and by construction there *is* a cause to name: raster
/// reaches this state by asking wgpu for a texture and being refused, and the refusal that reaches
/// a surface which has been configured once is very nearly always a *re*configure that failed.
/// `wgpu::Surface::configure` returns `()`, so the only account of it is what the device told the
/// handler, which is why `said` is here.
#[must_use]
pub fn swapchain(
    reason: raster_gpu::SurfaceProblem,
    said: Option<&render_raster::Uncaptured>,
) -> Swapchain {
    match reason {
        // The window system has moved under the swapchain — a resize, a monitor change, a
        // compositor restart. raster has already marked the surface for reconfiguration, so the
        // frame that follows this one will replace it.
        raster_gpu::SurfaceProblem::Outdated | raster_gpu::SurfaceProblem::Lost => {
            Swapchain::AskAgain
        }
        // Both are ordinary and neither is this program's to fix: an occluded window is one
        // nobody can see, and a timeout is a swapchain whose images are all still in flight.
        raster_gpu::SurfaceProblem::Timeout | raster_gpu::SurfaceProblem::Occluded => {
            Swapchain::Wait
        }
        raster_gpu::SurfaceProblem::Validation => Swapchain::Refused(match said {
            Some(said) => format!(
                "the window's swapchain could not be rebuilt, and the graphics device said: {}",
                said.last.trim()
            ),
            None => "the window's swapchain could not be rebuilt, and the graphics device gave \
                     no account of why"
                .to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {

    /// **The `auto` policy in its three cases** (ADR 0700): a moved view takes the
    /// compute lane, an unchanged view keeps the lane it was drawn in — raster keys a
    /// retained encode on the lane, so a flip would cost a selection change a full
    /// re-encode — and a window with nothing shown keeps the launch rule.
    #[test]
    fn the_lane_follows_the_view_and_sticks_where_it_stands() {
        use crate::lane::CoverageChoice;
        let auto = CoverageChoice::Auto;
        let at = |scale: f32| Transform::scale(scale, -scale);
        assert_eq!(
            super::lane_for(
                at(1.5),
                Some(at(1.0)),
                Some(raster_gpu::Coverage::Cpu),
                &[],
                false,
                auto
            ),
            raster_gpu::Coverage::Compute,
            "a zoom is cold tiles everywhere, which is the compute lane's regime"
        );
        assert_eq!(
            super::lane_for(
                at(1.0),
                Some(at(1.0)),
                Some(raster_gpu::Coverage::Compute),
                &[],
                false,
                auto
            ),
            raster_gpu::Coverage::Compute,
            "an unchanged view keeps its lane, or the replay dies with the flip"
        );
        assert_eq!(
            super::lane_for(
                at(1.0),
                Some(at(1.0)),
                Some(raster_gpu::Coverage::Cpu),
                &[],
                false,
                auto
            ),
            raster_gpu::Coverage::Cpu,
            "sticky in both directions: the lane is the shown frame's, not a favourite"
        );
        assert_eq!(
            super::lane_for(at(1.0), None, None, &[], false, auto),
            raster_gpu::Coverage::Cpu,
            "the launch path keeps the rule its gates were measured on"
        );
        assert_eq!(
            super::lane_for(
                at(1.5),
                Some(at(1.0)),
                Some(raster_gpu::Coverage::Cpu),
                &[],
                true,
                auto
            ),
            raster_gpu::Coverage::Cpu,
            "a software adapter loses on the dispatch and keeps the processor's lanes"
        );
        assert!(
            super::software_adapter("llvmpipe (LLVM 22.1.8, 256 bits) (Cpu, Vulkan)"),
            "the format raster's construct.rs states"
        );
        assert!(!super::software_adapter(
            "AMD Radeon 890M Graphics (RADV STRIX1) (IntegratedGpu, Vulkan)"
        ));
        assert_eq!(
            super::lane_for(
                at(1.0),
                Some(at(1.5)),
                Some(raster_gpu::Coverage::Compute),
                &[super::linear_bits_of(at(1.0))],
                false,
                auto
            ),
            raster_gpu::Coverage::Cpu,
            "a magnification the atlas has drawn is a revisit, and the atlas holds it"
        );
        assert_eq!(
            super::lane_for(
                at(1.5),
                Some(at(1.0)),
                Some(raster_gpu::Coverage::Cpu),
                &[],
                false,
                CoverageChoice::Fixed(raster_gpu::Coverage::Gpu)
            ),
            raster_gpu::Coverage::Gpu,
            "a pinned lane is pinned"
        );
    }
    use pdf_render::Transform;
    use raster_gpu::SurfaceProblem;
    use render_raster::Uncaptured;

    use super::{GPU_COVERAGE_MAGNIFICATION, Swapchain, coverage_for, swapchain};

    /// The device's words after a `wgpu::Surface::configure` that failed, as wgpu formats them — the
    /// project owner's own launch, verbatim from their session.
    fn what_the_owner_saw() -> Uncaptured {
        Uncaptured {
            since: 1,
            last: "Validation Error\n\nCaused by:\n  In Surface::configure\n    Failed to wait \
                   for GPU to come idle before reconfiguring the Surface\n"
                .to_owned(),
        }
    }

    /// A surface the window system has moved under is one the next frame replaces, so the window
    /// asks again rather than waiting for an event that may not come.
    #[test]
    fn a_moved_surface_is_asked_for_again() {
        assert_eq!(
            swapchain(SurfaceProblem::Outdated, None),
            Swapchain::AskAgain
        );
        assert_eq!(swapchain(SurfaceProblem::Lost, None), Swapchain::AskAgain);
    }

    /// Neither of these is anybody's fault and neither is cleared by asking harder.
    #[test]
    fn an_occluded_or_busy_swapchain_is_waited_out() {
        assert_eq!(swapchain(SurfaceProblem::Timeout, None), Swapchain::Wait);
        assert_eq!(swapchain(SurfaceProblem::Occluded, None), Swapchain::Wait);
    }

    /// **The refusal the project owner's crash would have produced, had it not aborted first.**
    /// The device's own account of a failed configure is what makes this sentence actionable, and
    /// carrying it is the whole reason `swapchain` takes a second argument.
    #[test]
    fn a_validation_refusal_carries_what_the_device_said() {
        let said = what_the_owner_saw();
        let Swapchain::Refused(why) = swapchain(SurfaceProblem::Validation, Some(&said)) else {
            panic!("a validation state is not something a retry clears");
        };
        assert!(
            why.contains("could not be rebuilt"),
            "it says what happened: {why}"
        );
        assert!(
            why.contains("In Surface::configure"),
            "and it names the call the device refused: {why}"
        );
        assert!(
            !why.ends_with('\n'),
            "wgpu's message ends in a newline and a sentence must not: {why:?}"
        );
    }

    /// A device that said nothing still gets a sentence, and it says that it said nothing —
    /// rather than an empty clause a reader would take for a truncated message.
    #[test]
    fn a_validation_refusal_with_no_account_says_so() {
        let Swapchain::Refused(why) = swapchain(SurfaceProblem::Validation, None) else {
            panic!("a validation state is not something a retry clears");
        };
        assert!(why.contains("gave no account"), "{why}");
    }

    /// The lane follows the magnification, and the page transform a frame is drawn
    /// with is what states it: scale, y flip, translation.
    #[test]
    fn the_lane_follows_the_magnification() {
        let page = |magnification: f32| {
            Transform::scale(magnification, -magnification)
                .then(Transform::translate(0.0, 842.0 * magnification))
        };
        assert_eq!(
            coverage_for(page(8.0), crate::lane::CoverageChoice::Auto),
            raster_gpu::Coverage::Cpu,
            "below the atlas cliff the cached lane is cheaper"
        );
        assert_eq!(
            coverage_for(page(12.0), crate::lane::CoverageChoice::Auto),
            raster_gpu::Coverage::Gpu,
            "above it the CPU lane rasterises every glyph on every frame"
        );
        assert_eq!(
            coverage_for(
                page(GPU_COVERAGE_MAGNIFICATION),
                crate::lane::CoverageChoice::Auto
            ),
            raster_gpu::Coverage::Gpu,
            "the threshold itself belongs to the lane it names"
        );
    }

    /// §7.7.3.3's page rotation puts the magnification in `b` and `c` rather than `a`
    /// and `d`, so a rotated page must land on the same lane as an upright one at the
    /// same zoom. This is the case a `transform.a` test would get wrong — and get wrong
    /// silently, by choosing the slow lane on a quarter of the corpus.
    #[test]
    fn a_rotated_page_reads_the_same_magnification() {
        let upright = Transform::scale(12.0, -12.0);
        // A quarter turn: the scale moves off the diagonal entirely.
        let turned = Transform {
            a: 0.0,
            b: 12.0,
            c: -12.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        };
        let auto = crate::lane::CoverageChoice::Auto;
        assert_eq!(coverage_for(upright, auto), coverage_for(turned, auto));
        assert_eq!(coverage_for(turned, auto), raster_gpu::Coverage::Gpu);
    }
}
