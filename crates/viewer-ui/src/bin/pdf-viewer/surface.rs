//! How this window gets pixels: which surface is behind it, and what one frame does.
//!
//! Two paths and never both — a graphics device or the processor's own surface — and the whole of
//! the difference is in [`Surface`]. Bringing one up is the launch path's last step and is
//! therefore measured; drawing on it is the frame, and the frame is where every other module's
//! work arrives, the page from `viewer-core` and the overlays from this host.
//!
//! **A frame is two things happening at once on both of them**, which is ADR 0391 for the window
//! with a device and ADR 0461 for the one without: [`crate::renderer`] and [`crate::composer`] draw
//! pages on a thread of their own and this module presents, on the clock, whatever that thread has
//! finished — moved to where the view now is where it is not of the view now being asked for. What
//! one call to [`App::present`] does is therefore *adopt, ask, place* rather than *draw and wait*,
//! and the whole of the difference to how long a person waits for a window to answer them is in
//! that.
//!
//! So the two paths differ in their rasteriser and in **one** thing about the policy: what a
//! stand-in costs the thread that presents ([`crate::stale::Standing`]). Everything else —
//! `doc/todo/37`'s five rules, the retained pages, the clock — is one implementation asked twice.

use std::sync::Arc;

use pdf_render::{TargetSpec, Transform};
use render_quorra::QuorraWindowRenderer;
use viewer_core::{Answer, Command, Query, Rendered};
use viewer_ui::software::SoftwareSurface;
use winit::window::Window;

use crate::app::App;
use crate::arguments::backend_names;
use crate::overlays::Overlays;
use crate::timing::Stages;
use crate::trace::Topic;

/// The magnification past which quorra's GPU coverage lane is the cheaper one.
///
/// **Derived, not tuned.** quorra keeps a glyph's rasterised coverage in an atlas until
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

/// Which coverage lane the next frame should be drawn with.
///
/// Per frame, because the crossover is a magnification and a person zooming crosses it;
/// decided *here*, because this is the only crate that knows what magnification the
/// frame is at. The transform's determinant is the magnification squared — the page
/// transform is a scale, a y flip and a translation, and §7.7.3.3's page rotation puts
/// the same factor into `b` and `c` instead of `a` and `d` — so its square root is the
/// number to compare, and it is right for a rotated page as well.
/// Which lane `auto` picks for one frame, from what the window is already showing.
///
/// Three cases, each a measurement (quorra's ADR 0080/0081, this tree's ADR 0700):
///
/// - **The view moved** — any coefficient of the arrangement's transform differs from
///   the shown frame's — so every cached tile is cold, which is the regime the compute
///   lane wins on both page shapes measured (a 58k-fill page: ~150 ms against ~270; a
///   dense text page's cold sweep: 0.93 ms of encode against 8.84).
/// - **The view is the one being shown** — a chrome-only ask — so the frame replays
///   its retained encode, *if* the lane does not move: quorra keys a retained encode on
///   the lane, so the choice is **sticky** here, and flipping it would turn a selection
///   change into a full re-encode.
/// - **There is no shown frame** — the launch path — which keeps the lane the
///   time-to-first-page gates were measured on ([`coverage_for`]'s magnification rule);
///   the compute lane's first frame pays its pipeline compile and its segment
///   residency, and the launch path pays for nothing it can defer.
///
/// The sampled [`Gpu`](quorra_gpu::Coverage::Gpu) lane is deliberately absent from the
/// moved-view case: the compute lane beats it on the cold sweep (0.93 against 9.8 ms),
/// matches it held at 100×, and is exact where §10.7.4 records it non-conformant. It
/// stays reachable by `--coverage gpu`, and in the launch rule until a first-frame
/// measurement moves it.
pub(crate) fn lane_for(
    asked: Transform,
    shown: Option<Transform>,
    last: Option<quorra_gpu::Coverage>,
    seen_by_the_atlas: &[[u32; 4]],
    software: bool,
    choice: crate::arguments::CoverageChoice,
) -> quorra_gpu::Coverage {
    if let crate::arguments::CoverageChoice::Fixed(lane) = choice {
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
        // holds: quorra's tiles are keyed by the linear part and evicted only by a
        // repack, so a revisit — zooming back to the fit, the other window size of a
        // pair — hits, and the hit is worth 69 against the compute lane's 130 ms on
        // the worst page (the loop measurement in ADR 0700). A repack in between costs
        // one cold CPU frame, which is the bounded downside of remembering.
        (Some(_), _) if seen_by_the_atlas.contains(&linear_bits_of(asked)) => {
            quorra_gpu::Coverage::Cpu
        }
        (Some(_), _) => quorra_gpu::Coverage::Compute,
        _ => coverage_for(asked, choice),
    }
}

/// The linear part as the bits the atlas keys tiles by — the same reading quorra
/// makes, so "seen" here and "resident" there mean the same magnification.
pub(crate) fn linear_bits_of(transform: Transform) -> [u32; 4] {
    [
        transform.a.to_bits(),
        transform.b.to_bits(),
        transform.c.to_bits(),
        transform.d.to_bits(),
    ]
}

/// Whether this adapter is a software rasteriser, read from the description quorra
/// formats as `"{name} ({device_type:?}, {backend:?})"` (their `construct.rs`) — a
/// string test with a named source, to be replaced by a typed accessor when quorra
/// grows one.
pub(crate) fn software_adapter(description: &str) -> bool {
    description.contains("(Cpu,")
}

/// Bit equality of the six coefficients — the same reading quorra's retained-frame key
/// makes, so "the view moved" here and "the encode survives" there cannot disagree.
fn same_transform(a: Transform, b: Transform) -> bool {
    [a.a, a.b, a.c, a.d, a.e, a.f]
        .iter()
        .zip([b.a, b.b, b.c, b.d, b.e, b.f])
        .all(|(a, b)| a.to_bits() == b.to_bits())
}

pub(crate) fn coverage_for(
    transform: Transform,
    choice: crate::arguments::CoverageChoice,
) -> quorra_gpu::Coverage {
    if let crate::arguments::CoverageChoice::Fixed(lane) = choice {
        return lane;
    }
    let magnification = transform
        .a
        .mul_add(transform.d, -(transform.b * transform.c))
        .abs()
        .sqrt();
    if magnification >= GPU_COVERAGE_MAGNIFICATION {
        quorra_gpu::Coverage::Gpu
    } else {
        quorra_gpu::Coverage::Cpu
    }
}

/// What a swapchain state means for the tick after it, which is the whole of what a host decides.
///
/// A free function over an enum rather than a `match` inside [`App::put_up`], for one reason: the
/// decision is now made from **two** inputs — the state quorra reports and whatever the device
/// said to its uncaptured-error handler on the way — and a decision with two inputs is one worth
/// being able to test without a graphics device. Its tests are at the foot of this file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Swapchain {
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
/// **`Validation` is the one that changed in the six-hundred-and-twenty-eighth session.** It used
/// to be `swapchain validation failed`, four words that name no cause and suggest no action —
/// and by construction there *is* a cause to name: quorra reaches this state by asking wgpu for a
/// texture and being refused, and the refusal that reaches a surface which has been configured
/// once is very nearly always a *re*configure that failed. `Surface::configure` returns `()`, so
/// the only account of it is what the device told the handler, which is why `said` is here.
pub(crate) fn swapchain(
    reason: quorra_gpu::SurfaceProblem,
    said: Option<&render_quorra::Uncaptured>,
) -> Swapchain {
    match reason {
        // The window system has moved under the swapchain — a resize, a monitor change, a
        // compositor restart. quorra has already marked the surface for reconfiguration, so the
        // frame that follows this one will replace it.
        quorra_gpu::SurfaceProblem::Outdated | quorra_gpu::SurfaceProblem::Lost => {
            Swapchain::AskAgain
        }
        // Both are ordinary and neither is this program's to fix: an occluded window is one
        // nobody can see, and a timeout is a swapchain whose images are all still in flight.
        quorra_gpu::SurfaceProblem::Timeout | quorra_gpu::SurfaceProblem::Occluded => {
            Swapchain::Wait
        }
        quorra_gpu::SurfaceProblem::Validation => Swapchain::Refused(match said {
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

/// How this window's pixels reach it: with a graphics device, or without one.
///
/// **Never both, and that is the point.** A process holding [`Surface::Processor`] has created no
/// `wgpu::Instance`, selected no adapter and made no device, so a driver that faults while it
/// loads cannot reach it. Before the three-hundred-and-eighty-fourth session there was one
/// variant and `--cpu` chose only which rasteriser drew into it (ADR 0221).
///
/// **Both variants are now the same arrangement over two rasterisers** (ADR 0461): a thread that
/// draws pages, a store of finished pictures on this thread, and a present on the clock's tick.
/// What they do not share is the price of a stand-in, which is [`crate::stale::Standing`].
pub(crate) enum Surface {
    /// The window's surface, held apart from the device that made it (quorra's ADR 0056): this
    /// thread presents finished rasters and [`crate::renderer`]'s thread draws them.
    Device(Box<crate::renderer::Window>),
    /// The processor's raster copied onto the window, with the overlays composited into it
    /// first. `--cpu`, and a device that would not come up.
    ///
    /// **It carries the last frame's pixels and the retained pages beside the surface**, which the
    /// device path's variant keeps in `crate::renderer` for the same reason: the base is the
    /// window's own picture, chrome excluded, held wherever it already exists.
    Processor(Box<crate::composer::Composer>),
}

impl std::fmt::Debug for Surface {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(window) => formatter.debug_tuple("Device").field(window).finish(),
            Self::Processor(composer) => {
                formatter.debug_tuple("Processor").field(composer).finish()
            }
        }
    }
}

/// The window, and whatever puts pixels on it.
pub(crate) struct State {
    pub(crate) window: Arc<Window>,
    /// The graphics device's surface, or the software one. See [`Surface`].
    pub(crate) surface: Surface,
    /// The surface size in device pixels, updated on `WindowEvent::Resized`.
    pub(crate) size: (u32, u32),
}

impl App {
    /// The presenting half of this window, where there is one.
    ///
    /// A method rather than a `let` at each site because every one of them has to end the borrow
    /// before touching another field of `App`: the clock and the stand-in policy are fields
    /// beside `state`, and a window held across a call to either is a borrow the compiler is
    /// right to refuse.
    fn device_window(&mut self) -> Option<&mut crate::renderer::Window> {
        match self.state.as_mut()?.surface {
            Surface::Device(ref mut window) => Some(window),
            Surface::Processor(_) => None,
        }
    }

    /// The presenting half of this window where the processor is what draws on it.
    ///
    /// The same shape as [`Self::device_window`] and for the same borrow reason: the clock and the
    /// stand-in policy are fields beside `state`, and a composer held across a call to either is a
    /// borrow the compiler is right to refuse.
    fn composer(&mut self) -> Option<&mut crate::composer::Composer> {
        match self.state.as_mut()?.surface {
            Surface::Processor(ref mut composer) => Some(composer),
            Surface::Device(_) => None,
        }
    }

    /// Everything a tick does with a window that has a graphics device: adopt, ask, place.
    ///
    /// **Three steps in this order, and the order is the design** (ADR 0391).
    ///
    /// 1. **Adopt** whatever the render thread has finished, so that the placement below is
    ///    computed against the newest rendering there is. A frame that landed a microsecond ago
    ///    must not be stood in for as though it had not.
    /// 2. **Ask** for the frame this view needs, where the rendering on hand is not of it and the
    ///    thread is idle. This is where a render *starts*, and nothing waits for it.
    /// 3. **Place** the rendering on hand under whatever transform makes it depict this view —
    ///    the identity where it already does, [`crate::stale`]'s composition where it does not,
    ///    and nothing at all where a refusal says there is nothing true to draw.
    ///
    /// `stand_in` is false for §12.4.4's transition, whose frames are pictures of *two* pages
    /// moving: no transform of one is any view of either, so the newest is put up at the identity
    /// and nothing about it is approximated.
    fn on_the_device(
        &mut self,
        pages: &[crate::stale::Placed],
        stand_in: bool,
        chrome: &Overlays,
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let now = std::time::Instant::now();
        self.adopt(now, stages);
        let overlays = chrome.owned();
        let choice = self.coverage;
        let last_lane = self.lane;
        let self_seen = self.atlas_saw.clone();
        let drawing = {
            let window = self.device_window()?;
            // The magnification is the arrangement's rather than one page's: every page of a
            // column is placed at the same magnification by `viewer_core::layout`, so the first
            // states it — and the lane follows what the window already shows (ADR 0700).
            let coverage = lane_for(
                pages.first()?.target.transform,
                window
                    .shown()
                    .and_then(|shown| Some(shown.pages.first()?.target.transform)),
                last_lane,
                &self_seen,
                software_adapter(window.description()),
                choice,
            );
            // **Read before the ask below, and that is not an ordering accident.** Rule 5's
            // observation is "a render asked for at an *earlier* tick is out past a refresh",
            // and a render dispatched two lines further down has missed nothing yet. Reading it
            // afterwards would make every view change observe its own dispatch as a miss, and
            // the prediction — the half that decides whether a quick frame is waited for —
            // would never be asked.
            let was_out = window.out_for();
            // A rendering of exactly these lists at exactly these targets needs no successor. Each
            // page is compared by the `Arc` that makes its address mean something, because a page
            // turn at an unchanged magnification is a different picture at the same placement;
            // the targets by value, because a resize is a different frame at the same transform.
            // The *count* is compared with them: a scroll that brings a further row of a column
            // onto the screen leaves every page already up exactly where it was.
            // **And the chrome with them, since ADR 0526.** A frame is the pages *and* the
            // overlays drawn over them, and this test compared only the pages — so a window whose
            // page had not moved put up the frame it already had, and the find bar, the panel, the
            // notices card and §12.5.1's ring never reached the screen at all. Compared by value
            // because that is what a display list is: a few hundred commands against a page's tens
            // of thousands, and the comparison is what decides whether a whole frame is skipped.
            let of_this_view = window.shown().is_some_and(|shown| {
                shown.pages.len() == pages.len()
                    && shown.pages.iter().zip(pages).all(|(drawn, asked)| {
                        Arc::ptr_eq(&drawn.list, &asked.list) && drawn.target == asked.target
                    })
            }) && window.chrome_asked() == overlays;
            if !of_this_view {
                window.ask(pages.to_vec(), overlays, coverage, now);
            }
            (was_out, coverage)
        };
        // What the sticky half of `lane_for` reads next tick: the lane this view was
        // (or already had been) asked in — and, for the revisit rule, which
        // magnifications the atlas has drawn (a short ring; older entries age out as
        // the atlas's own tiles do, by being forgotten).
        self.lane = Some(drawing.1);
        if drawing.1 == quorra_gpu::Coverage::Cpu
            && let Some(first) = pages.first()
        {
            let bits = linear_bits_of(first.target.transform);
            if !self.atlas_saw.contains(&bits) {
                if self.atlas_saw.len() >= 8 {
                    self.atlas_saw.remove(0);
                }
                self.atlas_saw.push(bits);
            }
        }
        let out_for = drawing.0;
        // A transition is a picture of two pages moving and no transform of it is any view of
        // either, so the newest is put up at the identity and nothing about it is approximated.
        // A window that has never drawn is the other case with no stand-in to consider: it is not
        // a view change, so the policy is not asked and the tick presents whatever there is —
        // which for the ticks before the first frame lands is nothing at all.
        if !stand_in || !self.stale.has_rendering() {
            // The clock stays armed, so page one arrives on the tick after it is drawn rather
            // than waiting for an event that is not coming.
            self.cadence.owed(now);
            return self.put_up(Some(Transform::IDENTITY), &[], stages);
        }
        // Which retained pages have pixels for this view — asked before the plan, because whether
        // a page turn is a refusal or a blurred picture depends on it (ADR 0443).
        let under = self.device_window()?.underlay(pages);
        // Every rule that makes an approximation defensible is in `crate::stale` rather than
        // here. The period is the one number rule 5 is measured against (ADR 0384), and how
        // long the in-flight render has been out is its second way of knowing that a frame
        // has missed (ADR 0391).
        let planned = self.stale.plan(
            pages,
            under.len(),
            self.cadence.period(),
            out_for,
            // Three textured quads, issued while `crate::renderer` draws on a thread of its own:
            // they take nothing from the frame they stand in for (ADR 0391).
            crate::stale::Standing::Quads,
        );
        let stand = match planned {
            crate::stale::Plan::Render => {
                // **A frame asked for on this tick has to be collected on a later one, and this
                // is the only branch where nothing else arms the clock for it** (ADR 0526). A
                // view that has not moved plans no approximation, so `about_to_wait` rests on
                // `Wait`: the frame carrying the chrome that was just asked for would land in the
                // channel and stay there. It cannot spin — the tick that collects it finds the
                // chrome unchanged, asks for nothing, and stops arming.
                if self.device_window().is_some_and(|window| window.drawing()) {
                    self.cadence.owed(now);
                }
                return self.put_up(Some(Transform::IDENTITY), &[], stages);
            }
            crate::stale::Plan::Approximate(stand) => stand,
            crate::stale::Plan::Refused(why) => {
                let trace = self.trace;
                self.stale.declined(&why, trace);
                // Nothing is put up: the window keeps the picture it has, and the clock stays
                // armed below, so the tick that follows carries the rendering this one waited
                // for. Rule 5's whole point is that waiting one refresh beats showing a
                // resampling of a frame that was about to arrive anyway.
                self.cadence.owed(now);
                return None;
            }
        };
        // The sharp layer, where the plan says there is one. Asked through `Stale::reproject` and
        // nowhere else, so that no caller can compose against anything but the last rendering.
        let base = if stand.from.has_base() {
            match self.stale.reproject(pages) {
                Ok(carried) => {
                    // Rule 3 over the bound the placements were accepted under: a column says how
                    // far its pages disagreed, so the number the bound is set against is one a
                    // run prints rather than one this program asserts.
                    crate::stale::absorbed(carried, pages.len(), self.trace);
                    Some(carried.placement)
                }
                Err(why) => {
                    let trace = self.trace;
                    self.stale.declined(&why, trace);
                    return None;
                }
            }
        } else {
            // Rule 3: the refusal of the sharp layer is said and counted even though the window
            // is about to move, because "a low-resolution page was shown" and "nothing was shown"
            // are two answers and a person reading a page turn's trace needs to know which.
            if let Some(why) = &stand.instead_of_the_base {
                let trace = self.trace;
                self.stale.without_base(why, under.len(), trace);
            }
            None
        };
        stages.approximated = Some(stand.from);
        stages.missed = Some(stand.missed);
        self.put_up(base, &under, stages)
    }

    /// Takes whatever the render thread has finished, and records what it is.
    ///
    /// Everything a landed frame says about itself is said here, once: the launch timeline's
    /// scene mark, the note a device refusal earns, the §8.7.4.5.2 programs it drew from the
    /// grid, and the view it settled — which is what the next reprojection composes against.
    fn adopt(&mut self, now: std::time::Instant, stages: &mut Stages) {
        let Some(window) = self.device_window() else {
            return;
        };
        let landed = window.collect();
        let sharpened = window.sharpened();
        if let Some(took) = sharpened {
            // Rule 3's sentence for ADR 0699: the page on the window just became the 2×
            // rendering shown box-filtered down, and a trace that did not say so would
            // show a colour shift with no cause.
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "sharpened: the settled view at 2x, {:.1} ms on the render thread",
                    took.as_secs_f64() * 1e3
                ),
            );
        }
        let Some(landed) = landed else {
            return;
        };
        stages.gpu = landed.cost;
        // The first frame's scene translation is a launch milestone — the other half, with
        // interpretation, of what used to sit unnamed between `document joined` and `first
        // present` (ADR 0332). Marked when the frame *lands* rather than when it was asked for,
        // because the render is somebody else's thread now and this is the moment it is known: the
        // frame began `total` before now, and the scene was the first `scene` of that.
        //
        // A clock that will not go back that far is a machine whose monotonic clock started inside
        // this frame, which is not a state — but a subtraction that could saturate silently is,
        // so `now` stands in and the mark reads as an instant rather than as a wrong one.
        self.launch.scene_built(
            now.checked_sub(landed.cost.total).unwrap_or(now),
            landed.cost.scene,
        );
        // ADR 0376: a §8.7.4.5.2 program the device declined draws from the grid instead — the
        // right picture, four orders of magnitude slower — so the ground is said out loud rather
        // than left to a timing to imply.
        for ground in &landed.function_refusals {
            self.trace.say(
                Topic::Frames,
                format_args!("function shading drawn on the processor's grid: {ground}"),
            );
        }
        // A page drawn by the slower of two backends is a fact about this build worth saying out
        // loud, and saying it is what would have made the hundred-and-forty-second session's
        // report a sentence rather than a mystery.
        if let Some(problem) = &landed.fell_back {
            println!(
                "note: the graphics device {problem}, so the page was drawn on the processor \
                 instead"
            );
        }
        if let Some(problem) = &landed.refused {
            eprintln!("note: this page could not be drawn: {problem}");
        }
        // What the window is now able to draw, and what producing it cost. The next view change
        // reads both: the placement to carry the pixels *from*, and the cost to predict whether
        // the frame after it will miss its refresh (`doc/todo/37` rule 5, ADR 0384).
        //
        // **Whether the frame *built* its picture is part of that**, and it is quorra's own
        // observable rather than an inference from a small duration: a frame that replayed a
        // retained encode (ADR 0351) says what a replay costs and nothing about what the next
        // render will, and a view change never replays.
        if !landed.pages.is_empty() {
            let built = !matches!(
                landed.cost.encode_source,
                Some(quorra_gpu::EncodeSource::Replayed)
            );
            self.stale.settled(&landed.pages, landed.waited, built);
        }
    }

    /// Puts the frame on hand on the window under `placement`, and says what it was.
    ///
    /// The one place a swapchain state is answered, and it is answered exactly as it was when the
    /// device owned the surface: quorra's presenter reconfigures itself on a timeout or an
    /// outdated surface, so these are events to try again on rather than failures to report.
    ///
    /// `None` where nothing was put up — there is no frame yet, or the swapchain said to try
    /// again — and `None` for a reprojection as well, deliberately: the core is told what became
    /// of its request by the frame that *answers* it, and a stand-in answers nothing.
    fn put_up(
        &mut self,
        placement: Option<Transform>,
        under: &[(usize, Transform)],
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let began = std::time::Instant::now();
        let expected = self.stale.expected();
        let period = self.cadence.period();
        let (outcome, cost, retained, said) = {
            let window = self.device_window()?;
            (
                window.present(placement, under),
                window.last_present(),
                window.retained(),
                // **The one place a device's uncaptured complaint is taken**, and it is here
                // rather than once a frame because this is the call that can provoke one:
                // `Surface::configure` returns `()` and says what it thought only through the
                // handler `render-quorra` installs. Taken whether the present refused or not, so
                // that a failure under a present that then succeeded is not left in the record to
                // be attributed to some later frame.
                window.device_said(),
            )
        };
        if let Some(said) = &said {
            self.device_reported(said, outcome.is_err());
        }
        match outcome {
            Ok(true) => {}
            // Nothing has been drawn yet, which is every tick between the window appearing and
            // the first frame landing. Not a refusal and not a failure.
            Ok(false) => return None,
            // Swapchain states are events, not failures: nothing was presented, nothing is stale,
            // and the processor cannot help a window that is not presentable.
            Err(quorra_gpu::RenderError::SurfaceUnavailable { reason }) => {
                return match swapchain(reason, said.as_ref()) {
                    Swapchain::AskAgain => {
                        self.redraw();
                        None
                    }
                    Swapchain::Wait => None,
                    Swapchain::Refused(why) => {
                        self.swapchain_refused(&why);
                        Some(Rendered::Failed(why))
                    }
                };
            }
            Err(problem) => {
                let why = crate::stale::Refusal::DeviceRefused(problem.to_string());
                let trace = self.trace;
                self.stale.declined(&why, trace);
                return None;
            }
        }
        if let Some(cost) = cost {
            stages.present = cost
                .acquire_wall
                .saturating_add(cost.record_wall)
                .saturating_add(cost.present_wall);
        }
        let Some(from) = stages.approximated else {
            // A rendering reached the window. **Whether it landed at *this* tick is deliberately
            // not asked**: a tick that put the same rendering up again has still presented one,
            // which is what the core's acknowledgement and the launch timeline both read, and
            // what the summary counts as a correct frame.
            return Some(Rendered::Presented);
        };
        // Rule 3 twice over: the frame line carries [`crate::stale::Source::word`], and this says
        // what the picture is an approximation *of* — which frame it stands in for, which layers
        // filled it, and what putting it up cost. There is no readback in it and no upload behind
        // it any more, which is why the third number this line used to carry is gone with them
        // (ADR 0391).
        self.trace.say(
            Topic::Frames,
            format_args!(
                "{}: {} against a {:.1} ms refresh, so it misses, and {} of {retained} \
                 retained low-resolution page(s) stand in under composed placements \
                 (present {:.2} ms); the real frame is being drawn",
                from.word(),
                stages
                    .missed
                    .unwrap_or(crate::stale::Missed::Predicted { frame: expected }),
                period.as_secs_f64() * 1e3,
                under.len(),
                began.elapsed().as_secs_f64() * 1e3,
            ),
        );
        // Rule 1, as a value that cannot be dropped: the frame that replaces this one is asked
        // for here, in the same expression that records it — of the clock rather than of the
        // window, so that a view still moving is answered again on the next tick.
        self.stale.drawn(from).follow(&mut self.cadence, began);
        None
    }

    /// Says what the program did about something the device reported that no call returned.
    ///
    /// **The half that was missing when the project owner's viewer aborted.** The device's own
    /// sentence was printed by the handler, and then nothing said what became of it — so a person
    /// reading the output could not tell whether the program had noticed. Two sentences, two
    /// authors: `render-quorra`'s handler says what the device said, and this says what was done.
    ///
    /// `refused` is whether the present that provoked it went on to refuse, because those are two
    /// different reports: an error under a present that succeeded is a fact about the device that
    /// cost this frame nothing, and one under a present that refused is this frame's cause.
    fn device_reported(&mut self, said: &render_quorra::Uncaptured, refused: bool) {
        if refused {
            // The refusal itself is reported by the arm that classifies it, which has the state
            // quorra named and can therefore say something this cannot; a second sentence here
            // would be the same event twice.
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "{} uncaptured device error(s) arrived with this frame's refusal",
                    said.since
                ),
            );
            return;
        }
        // Loud rather than traced: nothing refused, so no other line will mention this at all,
        // and an error the device raised on its own is exactly the thing that goes unnoticed
        // until a window stops updating.
        println!(
            "note: the graphics device reported {} error(s) that no call returned; the frame was \
             presented anyway, so this is being carried on past deliberately",
            said.since
        );
    }

    /// Says, once, that this window's swapchain is not coming back — and what a person can do.
    ///
    /// **A sentence rather than a crash, and a sentence rather than a silent fallback.** The
    /// processor path exists and could draw here, but taking it without being asked would make
    /// this a different program from the one `CLAUDE.md` describes: page one goes to the graphics
    /// device by the project owner's decision, and a run that quietly stopped using it would hide
    /// exactly the fault a person needs to see. So the two things that *are* offered are named.
    fn swapchain_refused(&mut self, why: &str) {
        if self.frames.refused_swapchain {
            return;
        }
        self.frames.refused_swapchain = true;
        eprintln!("{why}");
        eprintln!(
            "This window cannot present again until it is restarted. --cpu opens no graphics \
             driver at all and draws every page on the processor."
        );
    }

    /// `stages` is filled in as the frame goes: see [`Stages`] for why one number was not enough.
    fn present(&mut self, stages: &mut Stages) -> Option<Rendered> {
        let began = std::time::Instant::now();
        // §12.3.4's list is built here and nowhere else: this is the one place that holds
        // `&mut self` and runs before the panel is drawn.
        self.fill_visible_pages();
        let (width, height) = {
            let state = self.state.as_ref()?;
            state.size
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let pages = self.arrangement(edge, width, height);
        let Some(first) = pages.first() else {
            // **A window with no page is a window that can still have something on it**, and until
            // the six-hundred-and-ninety-fifth session this line was a `?` that ended the frame.
            // §7.6.4.1's card is drawn over a document that has not authenticated: there is no
            // page behind it because the document is not open, which is the whole reason the card
            // is there. 687's lesson one round on — a piece of chrome added is a piece of chrome
            // to check on *both* surfaces — and this is the third path it has to reach.
            let chrome = Overlays::of(self, edge, width, height);
            stages.host = began.elapsed();
            return self.without_a_page(&chrome, (width, height), stages);
        };
        stages.page = first.of.page().saturating_add(1);
        stages.pages = pages.len();
        stages.commands = pages
            .iter()
            .map(|placed| placed.list.commands().len())
            .sum();

        // §12.4.4: a transition in flight substitutes its own picture of *two* pages for the
        // arrangement, and it is a display list, so everything below this line is unchanged by it
        // — which is the point of shaping a frame in `viewer_core::transition` rather than
        // compositing here. A transition runs in §12.4.4's presentation mode, which shows one
        // page, so it stands in for the whole arrangement rather than for one entry of it.
        let (playing, drawn) = self.frame_to_draw(&first.list, first.target, width, height);
        let transitioning = playing.is_some();
        let identity = first.of;
        let for_the_frame = match playing {
            // The transition's own picture, under the identity of the page it is a transition
            // *to*: it is never a stand-in for anything — `Stale::forget` sees to that below —
            // so the identity here is what keeps the type honest rather than something read.
            Some(frame) => vec![crate::stale::Placed {
                of: identity,
                list: frame,
                target: drawn,
            }],
            None => pages,
        };

        let chrome = Overlays::of(self, edge, width, height);
        stages.host = began.elapsed();

        // A transition is already a picture of two pages moving, drawn from rasters this host
        // took for it: there is no stall to cover and the pixels on the screen are not a page.
        if transitioning {
            self.stale.forget();
        }

        if self.device_window().is_some() {
            return self.on_the_device(&for_the_frame, !transitioning, &chrome, stages);
        }
        self.on_the_processor(&for_the_frame, !transitioning, &chrome, stages)
    }

    /// The chrome alone, on a window with no page under it.
    ///
    /// One method for both surfaces because there is nothing to decide between them here: no page
    /// means no magnification to pick a coverage lane for, no retained pixels to stand in with and
    /// no view change to approximate, so all the machinery `on_the_device` and `on_the_processor`
    /// exist for has no subject. What is left is *draw these lists over the surround and present*.
    ///
    /// `None` where there is nothing to draw either, which is a window whose document has not
    /// arrived yet — every tick between the window appearing and the first frame landing, exactly
    /// as before.
    fn without_a_page(
        &mut self,
        chrome: &Overlays,
        extent: (u32, u32),
        stages: &mut Stages,
    ) -> Option<Rendered> {
        // **Nothing to draw and nothing ever drawn is the launch path**, and it stays exactly what
        // it was: the ticks between the window appearing and page one landing present nothing, so
        // no blank frame goes up in front of the first page. Once this method *has* put chrome up,
        // an empty list stops meaning "nothing to show" and starts meaning "take it away" — which
        // is what a card whose attempts have run out needs, and what the screen said otherwise: the
        // prompt stayed on the window after the program had said it was done asking.
        if chrome.lists().is_empty() && !self.drawn_without_a_page {
            return None;
        }
        self.drawn_without_a_page = true;
        let began = std::time::Instant::now();
        if self.device_window().is_some() {
            let owned = chrome.owned();
            let now = std::time::Instant::now();
            // **The frame drawn for the *previous* tick is taken here, and leaving this line out
            // is what a first run of this method looked like**: the job went to the render thread,
            // the thread drew the card, and nothing ever collected it — so every tick asked again
            // and reported *nothing to show*, for ever, at the tick rate. `on_the_device` opens
            // with the same call for the same reason.
            self.adopt(now, stages);
            let asked = {
                let chrome_coverage = coverage_for(Transform::IDENTITY, self.coverage);
                let window = self.device_window()?;
                // The same test `on_the_device` makes, with the page half of it answered by there
                // being no page: a frame of exactly these overlays over nothing needs no successor.
                let of_this_view = window.shown().is_some_and(|shown| shown.pages.is_empty())
                    && window.chrome_asked() == owned;
                if of_this_view {
                    false
                } else {
                    window.ask(Vec::new(), owned, chrome_coverage, now);
                    true
                }
            };
            // **Armed only where a frame was asked for**, which is `doc/todo/36`'s fourth rule and
            // not a detail: a frame is collected on a later tick than the one that asked for it, so
            // the clock has to stay on until it lands — and arming it unconditionally makes a
            // window with an unchanging card present at the tick rate for ever. Measured doing
            // exactly that before this line distinguished the two cases.
            if asked {
                self.cadence.owed(now);
            }
            return self.put_up(Some(Transform::IDENTITY), &[], stages);
        }
        // The composing thread's counterpart, and the same reason: a refusal it reported has to be
        // taken before anything is asked for again, or a window that cannot draw spins.
        if let Some(problem) = self.adopt_composed(stages) {
            return Some(Rendered::Failed(problem));
        }
        let overlays = chrome.lists();
        let outcome = self.composer()?.put_up_without_a_page(extent, &overlays);
        stages.present = began.elapsed();
        match outcome {
            Ok(()) => Some(Rendered::Presented),
            // Trap 5: a card a person is meant to type into that did not reach the window is
            // exactly the failure this whole item is about, so it is said rather than counted as a
            // tick with nothing on it.
            Err(problem) => Some(Rendered::Failed(format!(
                "presenting the chrome over an empty window: {problem}"
            ))),
        }
    }

    /// Everything a tick does with a window the processor draws on: adopt, ask, place.
    ///
    /// **The same three steps in the same order as [`Self::on_the_device`]** (ADR 0461), which is
    /// the whole of what giving this surface a composing thread bought: the two windows had two
    /// policies — one drew its stand-in beside the frame and the other in front of it, one had the
    /// retained pages and the other could not — and they now differ in one number, which is what a
    /// stand-in costs the thread that presents ([`crate::stale::Standing`]).
    ///
    /// `stand_in` is false for §12.4.4's transition, whose frames are pictures of *two* pages
    /// moving: no transform of one is any view of either.
    fn on_the_processor(
        &mut self,
        pages: &[crate::stale::Placed],
        stand_in: bool,
        chrome: &Overlays,
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let now = std::time::Instant::now();
        // A page that would not draw is reported before anything is asked for again, so that a
        // document this program cannot rasterise refuses once per redraw rather than spinning.
        if let Some(problem) = self.adopt_composed(stages) {
            return Some(Rendered::Failed(problem));
        }
        let overlays = chrome.lists();
        let (out_for, superseded) = {
            let composer = self.composer()?;
            // **Read before the ask below**, and that is not an ordering accident — see
            // [`Self::on_the_device`], where the same line has the same reason.
            let was_out = composer.out_for();
            let mut superseded = false;
            if !composer.depicts(pages) {
                // **ADR 0657's rule 1, and it is asked before the ask rather than instead of
                // it.** A raise does not free the thread this instant — the command in progress
                // finishes first, measured at 1.3 to 2.1 ms against a 2.76 ms command (ADR 0650)
                // — so the job it clears the way for is sent by the tick that collects it, one
                // period later. Asking here is what makes that one period rather than the
                // remainder of a frame.
                superseded = composer.superseded(pages);
                composer.ask(pages.to_vec(), now);
            }
            (was_out, superseded)
        };
        if superseded {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "the frame being drawn is of a view this one could not stand in with, so the \
                     composing thread was interrupted and will draw this view instead"
                ),
            );
        }
        if !stand_in || !self.stale.has_rendering() {
            // The clock stays armed, so page one arrives on the tick after it is drawn rather than
            // waiting for an event that is not coming.
            self.cadence.owed(now);
            return self.put_up_composed(&overlays, stages);
        }
        // Which retained pages have pixels for this view — asked before the plan, because whether
        // a page turn is a refusal or a blurred picture depends on it (ADR 0443).
        let under = self.composer()?.underlay(pages);
        let planned = self.stale.plan(
            pages,
            under.len(),
            self.cadence.period(),
            out_for,
            // A resample of a window of pixels, on the thread that presents: beside the frame it
            // stands in for, as on the other surface, but not free (ADR 0461).
            crate::stale::Standing::Resample,
        );
        let stand = match planned {
            crate::stale::Plan::Render => return self.put_up_composed(&overlays, stages),
            crate::stale::Plan::Approximate(stand) => stand,
            crate::stale::Plan::Refused(why) => {
                let trace = self.trace;
                self.stale.declined(&why, trace);
                // The clock stays armed, so the tick that follows carries the frame this one
                // waited for.
                self.cadence.owed(now);
                return self.put_up_unshown(&overlays, stages);
            }
        };
        let base = if stand.from.has_base() {
            match self.stale.reproject(pages) {
                Ok(carried) => {
                    crate::stale::absorbed(carried, pages.len(), self.trace);
                    Some(carried.placement)
                }
                Err(why) => {
                    let trace = self.trace;
                    self.stale.declined(&why, trace);
                    return self.put_up_unshown(&overlays, stages);
                }
            }
        } else {
            // Rule 3: the refusal of the sharp layer is said and counted even though the window is
            // about to move, because "a low-resolution page was shown" and "nothing was shown" are
            // two answers and a person reading a page turn's trace needs to know which.
            if let Some(why) = &stand.instead_of_the_base {
                let trace = self.trace;
                self.stale.without_base(why, under.len(), trace);
            }
            None
        };
        stages.approximated = Some(stand.from);
        stages.missed = Some(stand.missed);
        self.stand_in_composed(pages, base, &under, &overlays, stages)
    }

    /// Takes whatever the composing thread has finished, and records what it is.
    ///
    /// Answers the sentence a page that would not draw produced, and `None` for every other tick —
    /// including the ones where nothing has finished, which is most of them.
    fn adopt_composed(&mut self, stages: &mut Stages) -> Option<String> {
        let landed = self
            .composer()
            .and_then(crate::composer::Composer::collect)?;
        // **ADR 0657's rule 3, and trap 20: an abandoned draw is answered to nobody.** It is not a
        // refusal — `Rendered::Failed` would set the core's `shown` for this page and stop the
        // scheduler asking again, which is right for a page that will not rasterise and would
        // freeze one this host merely chose not to finish. It is not a settled view either: no
        // pixels of it exist, so recording it as the base would have `Stale` reproject from a
        // picture that was never drawn and feed rule 5's prediction a frame that did not land.
        //
        // **And `stages.composed` stays unset**, which is a measurement decision rather than a
        // tidy-up: it is a row of the summary's percentiles (`crate::timing::SUMMARY_ROWS`) and
        // what those describe is what a *frame* costs the composing thread. A draw stopped part
        // way through is a sample of nothing, and letting it into the distribution would pull that
        // number down by however far through the page the interrupt landed. What it cost is said
        // in a line of its own instead.
        if landed.abandoned {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "the interrupted frame came back after {:.1} ms, drawn and dropped; nothing \
                     is reported and the view it was of is not recorded as settled",
                    landed.cost.as_secs_f64() * 1e3
                ),
            );
            return None;
        }
        stages.composed = landed.cost;
        if landed.refused.is_some() {
            return landed.refused;
        }
        // What the window is now able to draw, and what waiting for it cost. The next view change
        // reads both: the placement to carry the pixels *from*, and the cost to predict whether the
        // frame after it will miss its refresh (`doc/todo/37` rule 5, ADR 0384). Every frame on
        // this surface builds its picture — there is no encode to replay — so the prediction is
        // updated by all of them.
        if !landed.pages.is_empty() {
            self.stale.settled(&landed.pages, landed.waited, true);
        }
        None
    }

    /// Puts up a rendering the window is holding and has never shown, where a stand-in was refused.
    ///
    /// **What a refusal costs is not the same on the two surfaces, and this is the difference**
    /// (ADR 0461). On the device the only *judged* refusal is [`crate::stale::Refusal::InsideTheRefresh`],
    /// which says the true frame is expected within one refresh — so waiting costs one refresh and
    /// bounds itself. Here rule 4 refuses precisely when the frame is **slow**, and a frame that
    /// landed while the view was still moving is then never put up at all: the window would go on
    /// showing a picture older than the one it is holding, for as long as the gesture lasts. It
    /// was measured doing exactly that — three renderings finished and discarded inside one scroll.
    ///
    /// So the refusal keeps its meaning — *do not stand in* — and stops meaning *show nothing*. A
    /// rendering nobody has seen is the truest picture this window has, at the placement it was
    /// drawn for, which is what every viewer showed before `doc/todo/37` existed. It is a
    /// rendering rather than an approximation and is counted as one: the pixels are right, and
    /// what is old is the view they are of.
    fn put_up_unshown(
        &mut self,
        overlays: &[&pdf_render::DisplayList],
        stages: &mut Stages,
    ) -> Option<Rendered> {
        if !self.composer()?.unshown() {
            return None;
        }
        self.trace.say(
            Topic::Frames,
            format_args!(
                "the rendering on hand is of the view before this one and has not been on the \
                 window at all, so it goes up unmoved rather than being held back"
            ),
        );
        self.put_up_composed(overlays, stages)
    }

    /// Puts the frame on hand on the window as it is, under the chrome.
    ///
    /// `None` where nothing has been drawn yet, which is every tick between the window appearing
    /// and the first frame landing, and `None` where the surface refused — which is reported by
    /// name rather than counted as a frame.
    fn put_up_composed(
        &mut self,
        overlays: &[&pdf_render::DisplayList],
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let began = std::time::Instant::now();
        let outcome = self.composer()?.put_up(overlays);
        stages.present = began.elapsed();
        match outcome {
            Ok(true) => Some(Rendered::Presented),
            Ok(false) => None,
            Err(problem) => Some(Rendered::Failed(format!(
                "presenting the processor's page {problem}"
            ))),
        }
    }

    /// Resamples the last frame's own pixels onto this view, over whatever retained pages have
    /// pixels for it, and puts that on the window.
    ///
    /// **Every rule that makes this defensible is `crate::stale`'s and none of them is new.** Rule
    /// 5 decided that the frame being drawn will miss the refresh; rule 4 decided that the resample
    /// finishes a refresh before it, which is a question on this surface and only on this one; rule
    /// 3 says what was shown, in the trace and in the summary's count; and rule 1 is
    /// [`crate::stale::MustFollow`], discharged of the clock exactly as the other surface
    /// discharges it.
    fn stand_in_composed(
        &mut self,
        pages: &[crate::stale::Placed],
        base: Option<Transform>,
        under: &[(usize, Transform)],
        overlays: &[&pdf_render::DisplayList],
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let extent = pages
            .first()
            .map(|placed| (placed.target.width, placed.target.height))?;
        let began = std::time::Instant::now();
        let (retained, outcome) = {
            let composer = self.composer()?;
            (
                composer.retained(),
                composer.stand_in(extent, base, under, overlays),
            )
        };
        let costs = match outcome {
            Ok(costs) => costs,
            Err(problem) => {
                let why = crate::stale::Refusal::DeviceRefused(problem.to_string());
                let trace = self.trace;
                self.stale.declined(&why, trace);
                stages.approximated = None;
                return None;
            }
        };
        // `None` is a picture made of no layers at all, which the plan has already ruled out: it
        // answers `Approximate` only where the base carries or a retained page has pixels.
        let Some((resample, cost)) = costs else {
            stages.approximated = None;
            return None;
        };
        // Rule 4's only possible sample, taken from the thing itself, on the machine it ran on.
        self.stale.resampled(cost);
        stages.present = cost.saturating_sub(resample);
        let from = stages.approximated?;
        self.trace.say(
            Topic::Frames,
            format_args!(
                "{}: {} against a {:.1} ms refresh, so it misses, and {} of {retained} \
                 retained low-resolution page(s) stand in under composed placements \
                 (resample {:.2} ms, present {:.2} ms — rule 4 judges their sum); the real \
                 frame is being drawn",
                from.word(),
                stages.missed.unwrap_or(crate::stale::Missed::Predicted {
                    frame: self.stale.expected(),
                }),
                self.cadence.period().as_secs_f64() * 1e3,
                under.len(),
                resample.as_secs_f64() * 1e3,
                cost.saturating_sub(resample).as_secs_f64() * 1e3,
            ),
        );
        // Rule 1, as a value that cannot be dropped: the frame that replaces this one is asked for
        // here, of the clock rather than of the window, so that a view still moving is answered
        // again on the next tick.
        self.stale.drawn(from).follow(&mut self.cadence, began);
        None
    }

    /// Table 29's arrangement as this window is about to draw it: every page, placed.
    ///
    /// **The whole of what a tier-2 host needs in order to obey `/PageLayout`, and it needed no
    /// message.** `viewer-core` hands one [`viewer_core::RenderRequest`] per page on the screen
    /// and answers `Query::PageGeometry` for each of them; a page the arrangement no longer shows
    /// has no geometry — that question's own documentation says so — which is what says a request
    /// this host is still holding has scrolled off. So the requests kept are exactly the pages
    /// placed, and the list comes back in page order because that is the order they arrived in and
    /// the order `viewer_core::layout` sorts its placements into.
    ///
    /// Each target is the *window's* extent with the page's placement composed into its transform,
    /// which is the one thing this host adds to what the core said: the core centres and scrolls
    /// the arrangement, and the panel's edge is the host's own.
    fn arrangement(&mut self, edge: f32, width: u32, height: u32) -> Vec<crate::stale::Placed> {
        let mut placed = Vec::with_capacity(self.requests.len());
        for request in &self.requests {
            let Answer::Geometry(geometry) = self.viewer.query(Query::PageGeometry(request.page))
            else {
                continue;
            };
            placed.push(crate::stale::Placed {
                // What a picture of this page is a picture *of*, carried from the request that
                // asked for it: the document, the page, and the state of the ink it was
                // interpreted against (ADR 0457).
                of: crate::stale::Picture::new(request.document, request.page, request.ink),
                list: Arc::clone(&request.list),
                target: TargetSpec {
                    width,
                    height,
                    transform: request.target.transform.then(Transform::translate(
                        geometry.origin.0 + edge,
                        geometry.origin.1,
                    )),
                },
            });
        }
        // What the arrangement dropped is dropped here, so that a thousand-page document scrolled
        // from end to end holds one request per page on the screen rather than one per page read.
        self.requests
            .retain(|request| placed.iter().any(|placed| placed.of.page() == request.page));
        placed
    }

    /// Brings up whatever will put pixels on this window, or says why nothing can.
    ///
    /// **Two paths, and `--cpu` chooses between them.** With the flag, no `wgpu::Instance` is
    /// created, no adapter is enumerated and no device is made — the driver is never loaded, which
    /// is what makes the flag an answer to a driver that faults while loading. Without it, the
    /// device is brought up as it always was, and the processor's path is what a machine falls to
    /// when the device will not come at all.
    ///
    /// `None` means neither worked, which is the one launch failure this program cannot show a
    /// page past.
    pub(crate) fn bring_up(&mut self, window: &Arc<Window>) -> Option<Surface> {
        if self.processor {
            let surface = self.software(window);
            self.launch.mark("software surface");
            return surface;
        }

        // Shaders compile on a background thread and nothing here waits for them —
        // `CLAUDE.md`'s rule, since page one goes to the graphics device: what bringing the
        // device up costs is part of time-to-first-page, so it is measured rather than
        // assumed. The presenter reports uncaptured device errors itself, for the same
        // silent-window reason the Vello host did.
        let mut instance = self
            .instancing
            .take()
            .map(|thread| thread.join().expect("the thread creating the instance"));
        self.launch.mark("graphics instance");
        let began = std::time::Instant::now();
        let mut attempt = match instance.as_ref() {
            Some(instance) => QuorraWindowRenderer::with_instance(instance, window.clone()),
            None => QuorraWindowRenderer::new(window.clone()),
        };

        // **A default gives way; a flag does not.** This arm is only reachable where this build
        // restricted the backends by itself — today that is Windows and DX12 — and the machine
        // turned out to have no adapter behind it. Refusing there would be this project's guess
        // deciding that somebody's machine cannot start, so it is a note and a second attempt
        // with everything. `QuorraWindowRenderer::new` makes its own all-backends instance, which is
        // why the old one is dropped rather than reused.
        if attempt.is_err()
            && !self.backend_asked_for
            && let Some(named) = self.backend.take()
        {
            println!(
                "note: no graphics adapter behind the {} backend, which is the one this build \
                 asks for first on this platform, so every backend was offered instead",
                named.name()
            );
            instance = None;
            attempt = QuorraWindowRenderer::new(window.clone());
        }

        let renderer = match attempt {
            Ok(renderer) => renderer,
            Err(problem) => return self.no_device(window, instance.as_ref(), &problem),
        };
        let brought_up = began.elapsed();
        self.launch.mark("graphics device");
        // **The surface leaves the device here, on the launch path, and that is deliberate.**
        // quorra's `detach_presenter` clones four handles and moves the surface state; it asks the
        // pipeline store nothing, so it cannot compile, cannot wait for warmth and cannot block
        // (quorra's ADR 0056). The *thread* is not started here — that is the first job's, which
        // is `CLAUDE.md`'s rule about scheduler decisions in front of a launch milestone.
        let size = window.inner_size();
        let ungrounded = match crate::renderer::Window::split(
            renderer,
            (size.width.max(1), size.height.max(1)),
            self.proxy_pages,
            self.supersample,
            self.coverage,
            self.waker.clone(),
        ) {
            Ok(ungrounded) => ungrounded,
            Err(renderer) => {
                // A device built with `for_surface` always has one to hand over, so this is a
                // proof about quorra's constructors rather than a state anybody has seen — and it
                // is still a sentence rather than a panic, because the alternative to a window
                // that cannot present is a window nobody can read.
                eprintln!(
                    "the graphics device came up with no surface to present through ({}), so the \
                     page is drawn on the processor instead",
                    renderer.adapter_description()
                );
                return self.software(window);
            }
        };
        // **The surface is configured here and it is a launch milestone.** `Ungrounded` carries
        // the whole argument; what it comes to is that the *first* configure of a process is the
        // one wgpu answers with a panic rather than a status, and this is the only moment where
        // no other thread of this program can be submitting while it happens. A device that
        // cannot put its own background on its window cannot show a page on it either, so the
        // refusal is a launch decision — the processor draws instead, out loud, exactly as it
        // does for a device that would not come up at all.
        let grounded = std::time::Instant::now();
        let presenter = match ungrounded.ground() {
            Ok(presenter) => presenter,
            Err(why) => {
                eprintln!("the graphics device could not put anything on this window: {why}");
                eprintln!("  asked for: {}", self.backend_description());
                eprintln!(
                    "Drawing on the processor instead, which opens no graphics driver. Pages \
                     will be slower."
                );
                return self.software(window);
            }
        };
        let grounding = grounded.elapsed();
        self.launch.mark("surface configured");
        if self.trace.on(Topic::Launch) {
            let startup = presenter.startup();
            // Two lines about one choice, and they answer different questions. The first is what
            // was *asked for* — which is a fact about this command line — and the second ends in
            // the backend that was actually chosen, because quorra's adapter description carries
            // it: `llvmpipe (LLVM 22.1.8, 256 bits) (Cpu, Vulkan)`. A person diagnosing a driver
            // crash needs both, and before the three-hundred-and-eighty-fourth session there was
            // no way to ask for the first at all.
            self.trace.say(
                Topic::Launch,
                format_args!("backend asked for: {}", self.backend_description()),
            );
            self.trace.say(
                Topic::Launch,
                format_args!("rendering with {}", presenter.description()),
            );
            self.trace.say(
                Topic::Launch,
                format_args!(
                    "device up in {brought_up:?} — instance {:?}, surface {:?}, adapter {:?}, \
                     device {:?}, pipelines {}",
                    startup.instance_creation,
                    startup.surface_creation,
                    startup.adapter_selection,
                    startup.device_creation,
                    startup
                        .pipeline_compilation
                        .map_or_else(|| "still compiling".to_owned(), |d| format!("{d:?}"))
                ),
            );
            // What grounding cost, said on its own line because it is a *number to keep small*
            // rather than a step to take on trust: it is the swapchain's creation and the
            // window's first acquire, moved off page one's present onto a moment where the
            // configure cannot race a submission (`crate::renderer::Ungrounded`).
            self.trace.say(
                Topic::Launch,
                format_args!(
                    "surface configured and the window's ground put up in {grounding:?} — the \
                     first configure of this process, with no render thread to submit beside it"
                ),
            );
        }
        Some(Surface::Device(Box::new(presenter)))
    }

    /// What this run asked the graphics stack for, in words, for `--trace` and for a refusal.
    fn backend_description(&self) -> String {
        match (self.backend, self.backend_asked_for) {
            (Some(named), true) => format!("{} (--backend)", named.name()),
            (Some(named), false) => format!("{} (this platform's default)", named.name()),
            (None, _) => "every backend this build has".to_owned(),
        }
    }

    /// The window written to by the processor, or a sentence saying why there is not one.
    ///
    /// **A method rather than an associated function since ADR 0461**, because this surface has a
    /// setting of its own now: `--proxy-pages` reaches the composing thread exactly as it reaches
    /// the render thread, and the host is where it lives (`doc/todo/37` rule 2).
    fn software(&self, window: &Arc<Window>) -> Option<Surface> {
        match SoftwareSurface::new(Arc::clone(window)) {
            Ok(surface) => Some(Surface::Processor(Box::new(
                crate::composer::Composer::new(surface, self.proxy_pages),
            ))),
            Err(problem) => {
                eprintln!(
                    "this window cannot be drawn on without a graphics device: {problem}\n\
                     The software path is compiled for X11 and Wayland here; a session that is \
                     neither has no way to show a page without a device."
                );
                None
            }
        }
    }

    /// What to say — and what to do — when the graphics device will not come up.
    ///
    /// **This was `.expect("presenter creation")` until the three-hundred-and-eighty-fourth
    /// session**, which is `CLAUDE.md` principle 1's rule about panics in the one place a person
    /// most needs a sentence: a device that will not come up is a fact about the machine, not a
    /// defect in this program, and the shape to use is `Confinement::shortfall`'s — name the
    /// stage, name what was seen, name what to try.
    ///
    /// The two outcomes are deliberately different. A backend a **person** named is honoured or
    /// refused, because "this machine has no DX12 adapter" is an answer to the question they
    /// asked and starting on the stack they were avoiding is not. A device that failed with no
    /// backend named is a broken machine rather than a mistaken command line, so the page is
    /// drawn on the processor and the note says so.
    fn no_device(
        &self,
        window: &Arc<Window>,
        instance: Option<&quorra_gpu::wgpu::Instance>,
        problem: &render_quorra::QuorraRasterError,
    ) -> Option<Surface> {
        eprintln!("the graphics device could not be brought up: {problem}");
        eprintln!("  asked for: {}", self.backend_description());
        if let Some(instance) = instance {
            // What *this* instance could see, which is the number that distinguishes a backend
            // this machine does not have (none) from a driver that failed later (some).
            let visible = QuorraWindowRenderer::adapters_on(instance);
            eprintln!(
                "  adapters behind it: {}",
                if visible.is_empty() {
                    "none — this machine has no adapter for that backend".to_owned()
                } else {
                    visible.join(", ")
                }
            );
        }
        // And what the machine has by every route, which is the list a person picks their next
        // `--backend` out of. Its own all-backends instance, so it costs a driver load — which is
        // acceptable here and nowhere else on this path: this run has already failed.
        let every = quorra_gpu::Device::adapter_names();
        eprintln!(
            "  adapters on this machine: {}",
            if every.is_empty() {
                "none".to_owned()
            } else {
                every.join(", ")
            }
        );
        if self.backend_asked_for {
            eprintln!(
                "Refused rather than started on another backend: --backend named one, and a flag \
                 that silently did something else would be worse than no flag. Try --backend with \
                 one of {}, or --cpu, which opens no graphics driver at all.",
                backend_names()
            );
            return None;
        }
        eprintln!(
            "Drawing on the processor instead, which opens no graphics driver. Pages will be slower."
        );
        self.software(window)
    }

    /// Asks the surface once more what it refreshes at, until it answers.
    ///
    /// **A Wayland surface enters no output until it has been drawn to**, and the cadence is read
    /// in `resumed`, which is strictly before that — so every Wayland session took
    /// `doc/todo/36`'s floor and 120 Hz was out of reach in principle. Called after a present
    /// rather than before one for exactly that reason, and it stops asking the moment the window's
    /// own output answers ([`crate::cadence::Cadence::ask`], ADR 0384).
    fn settle_cadence(&mut self) {
        if self.cadence.settled() {
            return;
        }
        // Cloned rather than borrowed because the clock and the window are two fields of the same
        // value; one `Arc` bump per frame, and only until the surface has answered once.
        let Some(window) = self.state.as_ref().map(|state| Arc::clone(&state.window)) else {
            return;
        };
        if self.cadence.ask(&window) {
            self.trace.say(
                Topic::Launch,
                format_args!(
                    "presenting on a cadence of {} — re-asked now that the window has been drawn to",
                    self.cadence.described()
                ),
            );
        }
    }

    /// Closes the one line the launch timeline never had: when the pipelines finished compiling.
    ///
    /// **ADR 0227.** Bring-up prints `pipelines still compiling` because
    /// `CLAUDE.md` forbids waiting for warmth on the launch path, and nothing ever said when the
    /// wait nobody did would have ended — so a first frame that absorbed a shader compilation
    /// and one that did not read identically. Polled once a frame, which behind the topic check
    /// is one `Option` read; *noticed* rather than *finished*, because the compilation ends on
    /// quorra's own thread and this is only the first frame to look.
    ///
    /// **Read off the last finished frame since ADR 0391**, because the device is no longer on
    /// this thread to ask: [`crate::renderer::Window`] keeps whatever the render thread saw when
    /// it last drew, which is the same answer one tick later.
    fn pipelines_compiled(&mut self) {
        if self.frames.pipelines || !self.trace.on(Topic::Launch) {
            return;
        }
        let Some(State {
            surface: Surface::Device(window),
            ..
        }) = self.state.as_ref()
        else {
            return;
        };
        let Some(compiling) = window.pipelines() else {
            return;
        };
        self.frames.pipelines = true;
        self.trace.say(
            Topic::Launch,
            format_args!(
                "pipelines compiled in {compiling:?}, noticed at this frame — nothing on the \
                 launch path waited for them, so every frame before this one drew with whatever \
                 was ready"
            ),
        );
    }

    /// Draws the frame the window asked for, and tells the core what became of it.
    ///
    /// **The frame's number is the frame's, since the three-hundred-and-ninetieth session.**
    /// This used to start a timer, present, close the launch timeline, publish the accessibility
    /// tree, and *then* read the timer — so `present -> presented in T` was the frame plus the
    /// bridge plus the timeline's own printing. ADR 0227 called that a measurement
    /// defect rather than a design choice, which is exactly what it was: on a page turn the tree
    /// is rebuilt and published, and that work was being attributed to the graphics device.
    pub(crate) fn redraw_requested(&mut self) {
        // **The clock's gate, and it is on every frame rather than only on the ones that follow
        // a reprojection** (`doc/todo/36`). A redraw that arrives before the surface has
        // refreshed would be overdrawn before anybody saw it, so it is deferred to the tick
        // instead — which is what stops the window running at whatever rate the *input* device
        // happens to deliver. Nothing is lost: `about_to_wait` sees the obligation and asks
        // again, and a window that has been still is due at once, so the first frame after an
        // input is not held back by so much as one refresh.
        let now = std::time::Instant::now();
        if !self.cadence.due(now) {
            self.cadence.owed(now);
            return;
        }
        self.cadence.serviced();
        // Before the frame rather than after it: a tick can advance the page, and a page that
        // advanced after its frame was drawn would be one frame late for the whole slide show.
        self.drive_the_clock();
        let started = std::time::Instant::now();
        let mut stages = Stages::default();
        let outcome = self.present(&mut stages);
        stages.total = started.elapsed();
        if matches!(outcome, Some(Rendered::Presented | Rendered::Raster(_))) {
            self.launch.arrived(self.trace, &stages);
            // **After the timeline is closed, never before it.** Everything the accessibility
            // bridge does is off the launch path by construction, and this line is where that is
            // enforced rather than merely intended.
            let attending = std::time::Instant::now();
            self.attend();
            stages.attend = attending.elapsed();
        }
        // Rule 1's other half: *every* frame that is not a reprojection clears the flag, a frame
        // that drew nothing included. A flag that could outlive the redraw answering it would
        // leave `about_to_wait` asking for a frame that never comes, which is a spinning loop.
        if stages.approximated.is_none() {
            self.stale.real();
        }
        // What the window actually put up, which is what `doc/todo/36`'s rule 6 counts: a
        // rendering and a reprojection are both presents and a frame that drew nothing is not
        // one. The clock is moved on by exactly these, so an idle window — which produces none —
        // never advances it and never wakes for it.
        stages.presented = stages.approximated.is_some()
            || matches!(outcome, Some(Rendered::Presented | Rendered::Raster(_)));
        if stages.presented {
            self.cadence.presented(std::time::Instant::now());
            self.settle_cadence();
        }
        let outcome_said = match (&outcome, stages.approximated) {
            (None, Some(from)) => from.word().to_owned(),
            (None, None) => "nothing to show".to_owned(),
            (Some(Rendered::Presented), _) => "presented".to_owned(),
            (Some(Rendered::Failed(why)), _) => format!("failed: {why}"),
            (Some(Rendered::Raster(_)), _) => "a raster".to_owned(),
            // Neither of the last two is this host's, and both are written out rather than
            // swept into a catch-all, which is what nothing in `viewer-core` being
            // `#[non_exhaustive]` is for: this window presents on its own device, so it never
            // hands back pixels and never keeps a page's list for itself.
            (Some(Rendered::Listed), _) => "a list the host kept".to_owned(),
        };
        let trace = self.trace;
        self.frames.frame(trace, &stages, &outcome_said);
        self.pipelines_compiled();
        let Some(rendered) = outcome else {
            return;
        };
        // **One acknowledgement per page of the arrangement**, because the core holds one
        // outstanding request per page and a page never answered for is a page it goes on
        // believing is not yet on the screen. A token the core has since superseded drops itself,
        // which is the whole reason a token exists.
        //
        // `Rendered` is deliberately not `Clone` — tier 1's variant carries a whole page of
        // pixels — so what is repeated is the *answer* rather than the value. This host is tier 2
        // and the only two it can produce are a present and a refusal; `Rendered::Raster` is tier
        // 1's and `Rendered::Listed` belongs to a host that keeps a page's marks rather than
        // drawing them, and nothing here builds either.
        let refused = match &rendered {
            Rendered::Failed(why) => Some(why.clone()),
            Rendered::Presented | Rendered::Raster(_) | Rendered::Listed => None,
        };
        for token in std::mem::take(&mut self.unacknowledged) {
            let rendered = refused
                .clone()
                .map_or(Rendered::Presented, Rendered::Failed);
            self.dispatch(Command::RenderReady { token, rendered });
        }
    }
}

#[cfg(test)]
mod tests {

    /// **The `auto` policy in its three cases** (ADR 0700): a moved view takes the
    /// compute lane, an unchanged view keeps the lane it was drawn in — quorra keys a
    /// retained encode on the lane, so a flip would cost a selection change a full
    /// re-encode — and a window with nothing shown keeps the launch rule.
    #[test]
    fn the_lane_follows_the_view_and_sticks_where_it_stands() {
        use crate::arguments::CoverageChoice;
        let auto = CoverageChoice::Auto;
        let at = |scale: f32| Transform::scale(scale, -scale);
        assert_eq!(
            super::lane_for(
                at(1.5),
                Some(at(1.0)),
                Some(quorra_gpu::Coverage::Cpu),
                &[],
                false,
                auto
            ),
            quorra_gpu::Coverage::Compute,
            "a zoom is cold tiles everywhere, which is the compute lane's regime"
        );
        assert_eq!(
            super::lane_for(
                at(1.0),
                Some(at(1.0)),
                Some(quorra_gpu::Coverage::Compute),
                &[],
                false,
                auto
            ),
            quorra_gpu::Coverage::Compute,
            "an unchanged view keeps its lane, or the replay dies with the flip"
        );
        assert_eq!(
            super::lane_for(
                at(1.0),
                Some(at(1.0)),
                Some(quorra_gpu::Coverage::Cpu),
                &[],
                false,
                auto
            ),
            quorra_gpu::Coverage::Cpu,
            "sticky in both directions: the lane is the shown frame's, not a favourite"
        );
        assert_eq!(
            super::lane_for(at(1.0), None, None, &[], false, auto),
            quorra_gpu::Coverage::Cpu,
            "the launch path keeps the rule its gates were measured on"
        );
        assert_eq!(
            super::lane_for(
                at(1.5),
                Some(at(1.0)),
                Some(quorra_gpu::Coverage::Cpu),
                &[],
                true,
                auto
            ),
            quorra_gpu::Coverage::Cpu,
            "a software adapter loses on the dispatch and keeps the processor's lanes"
        );
        assert!(
            super::software_adapter("llvmpipe (LLVM 22.1.8, 256 bits) (Cpu, Vulkan)"),
            "the format quorra's construct.rs states"
        );
        assert!(!super::software_adapter(
            "AMD Radeon 890M Graphics (RADV STRIX1) (IntegratedGpu, Vulkan)"
        ));
        assert_eq!(
            super::lane_for(
                at(1.0),
                Some(at(1.5)),
                Some(quorra_gpu::Coverage::Compute),
                &[super::linear_bits_of(at(1.0))],
                false,
                auto
            ),
            quorra_gpu::Coverage::Cpu,
            "a magnification the atlas has drawn is a revisit, and the atlas holds it"
        );
        assert_eq!(
            super::lane_for(
                at(1.5),
                Some(at(1.0)),
                Some(quorra_gpu::Coverage::Cpu),
                &[],
                false,
                CoverageChoice::Fixed(quorra_gpu::Coverage::Gpu)
            ),
            quorra_gpu::Coverage::Gpu,
            "a pinned lane is pinned"
        );
    }
    use pdf_render::Transform;
    use quorra_gpu::SurfaceProblem;
    use render_quorra::Uncaptured;

    use super::{GPU_COVERAGE_MAGNIFICATION, Swapchain, coverage_for, swapchain};

    /// The device's words after a `Surface::configure` that failed, as wgpu formats them — the
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
            coverage_for(page(8.0), crate::arguments::CoverageChoice::Auto),
            quorra_gpu::Coverage::Cpu,
            "below the atlas cliff the cached lane is cheaper"
        );
        assert_eq!(
            coverage_for(page(12.0), crate::arguments::CoverageChoice::Auto),
            quorra_gpu::Coverage::Gpu,
            "above it the CPU lane rasterises every glyph on every frame"
        );
        assert_eq!(
            coverage_for(
                page(GPU_COVERAGE_MAGNIFICATION),
                crate::arguments::CoverageChoice::Auto
            ),
            quorra_gpu::Coverage::Gpu,
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
        let auto = crate::arguments::CoverageChoice::Auto;
        assert_eq!(coverage_for(upright, auto), coverage_for(turned, auto));
        assert_eq!(coverage_for(turned, auto), quorra_gpu::Coverage::Gpu);
    }
}
