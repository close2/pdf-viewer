//! Tier 2: the page presented straight onto a window's surface, no readback.
//!
//! This is the tier `RENDER_LIBRARY.md` section 6.1 measurements pointed at from the start: the
//! readback that dominates a `rasterize` call simply does not exist here — quorra
//! renders the scene and presents the swapchain texture, and the pixels never
//! cross back to the CPU. One frame carries everything the window shows, each at
//! its own placement (which is why the adapter bakes placements into commands
//! rather than into the viewport): the page under its target transform, or a
//! CPU-rendered raster standing in for it, and the window-pixel overlay lists —
//! selection, sidebar, modal — on top, in order.

use std::sync::Arc;

use pdf_render::{Color, DisplayList, Raster, TargetSpec, Transform};
use quorra_scene::ResourceId;

use crate::QuorraRasterError;
use crate::scene::Encoder;

/// One frame of the window, everything included.
#[derive(Debug, Clone, Copy)]
pub struct PresentFrame<'a> {
    /// The surface size in device pixels.
    pub width: u32,
    /// See [`PresentFrame::width`].
    pub height: u32,
    /// The page and its placement — or `None` when a raster stands in for it.
    ///
    /// **The `Arc` is the identity a frame is reused by, and it is why this is not a plain
    /// reference** (ADR 0351). [`FrameSlot`] keeps the page's scene across frames and decides
    /// whether to rebuild it by the address of these samples, so it must be able to *pin* that
    /// address — the same ABA argument [`crate::cache`] makes for every other key in this
    /// crate. A borrowed `&DisplayList` could be dropped between two frames and the allocator
    /// could hand the same address to the next page, which is a stale page drawn with no
    /// report; holding the `Arc` makes the address unique for as long as the slot keeps it.
    pub page: Option<(&'a Arc<DisplayList>, TargetSpec)>,
    /// A pre-rendered page, drawn 1:1 from the window's top-left corner: the CPU
    /// fallback path, which must stay presentable even when the page itself is
    /// something the device refused.
    pub raster: Option<&'a Raster>,
    /// Window-pixel display lists drawn over the page, in order (selection
    /// highlights, sidebar, modal card — chrome crosses as geometry, not pixels).
    pub overlays: &'a [&'a DisplayList],
}

/// What one frame of [`QuorraPresenter::present`] spent, part by part.
///
/// **The question this exists to answer is "which of the stages was the slow one".** A host's
/// own timer around `present` gives one number over four different things — translating display
/// lists into a scene, handing resources to the device, encoding and submitting, and the device's
/// own execution — so a frame that is slow because it re-uploads a 30 MB image every time and one
/// that is slow because the page has four thousand commands look identical. ADR 0227 is the
/// argument; this is the split.
///
/// **Gathered on every frame, whether anything asks for it or not.** That is a deliberate cost:
/// three `Instant::now()` calls, two `u32` additions and a copy of this struct, tens of
/// nanoseconds against a frame measured in milliseconds. The alternative — a flag threaded down
/// from the host — would mean the instrument's presence changes the program being measured, which
/// is what ADR 0227 forbids outright.
///
/// **No boundary here is fabricated, and that is worth stating because it easily could have
/// been.** [`quorra_gpu::Device::render`] measures its own three phases and already blocks on the
/// device — `poll` with an indefinite wait — before it returns, so [`Self::execute`] is a number
/// this crate *reports* rather than a wait it introduced. [`Self::execute_measured`] says whether
/// the adapter's timestamp queries produced it or a host-side wall clock stood in, because
/// quorra's own rule is that a wall clock is context and a timestamp query is evidence.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameCost {
    /// The whole call, which is what a host's own timer would have seen alone.
    pub total: std::time::Duration,
    /// Translating this frame's display lists into a [`quorra_scene::Scene`] — the walk in
    /// [`build`], including every resource this crate uploaded on the way through.
    pub scene: std::time::Duration,
    /// [`quorra_gpu::Device::render`] end to end: the three below, plus acquiring the swapchain
    /// texture, presenting it and reading the instrumentation back.
    pub device: std::time::Duration,
    /// Inside `device`: turning the scene into device commands.
    pub encode: std::time::Duration,
    /// Inside `device`: preparing and scheduling this frame's CPU→GPU transfers.
    pub upload: std::time::Duration,
    /// Inside `device`: the drawing passes themselves.
    pub execute: std::time::Duration,
    /// Whether [`Self::execute`] came from the adapter's timestamp queries rather than from a
    /// wall clock over the submit-and-wait.
    pub execute_measured: bool,
    /// Whether this frame's device commands were encoded by it or replayed from an earlier
    /// frame of the same scene — `None` when the frame never reached the device.
    ///
    /// **An observable rather than an inference from a small [`Self::encode`]** (quorra's ADR
    /// 0048). A frame loop that expects to reuse its scene and does not is the failure mode
    /// [`FrameSlot`] can have, and "encode was small" is not a state a test or a gate can
    /// assert on. `None` rather than a third variant because a refusal that never reached
    /// [`quorra_gpu::Device::render_retained`] has no encode source to report, which is the
    /// same answer [`Self::device`]'s zero gives.
    pub encode_source: Option<quorra_gpu::EncodeSource>,
    /// Heap bytes the retained encode holds, or zero when none is retained.
    ///
    /// The price of the reuse above, reported every frame because it is the number a host
    /// budgets a retained page against and nothing else in this tree can compute it: a page of
    /// dense text retains about a third of a megabyte, and a page placing a quarter of a
    /// gigabyte of coverage tiles at 4× retains that.
    pub retained_bytes: u64,
    /// Releasing the previous scene's transient resources and evicting settled cache entries.
    ///
    /// Zero on a frame that reused its scene, and that is the whole of what moved in ADR 0351:
    /// both are the *rebuild's* work, because the scene being replaced named those resources
    /// until it was replaced.
    pub settle: std::time::Duration,
    /// Resources this frame handed the device: cache misses plus its own transients.
    ///
    /// A count rather than a duration, and the reason is in [`crate::cache`]: the caches are
    /// keyed by `Arc` identity, so a display list rebuilt from scratch every frame re-uploads
    /// everything, and a number that stays high on a page that has not changed is what says so.
    pub uploads: u32,
    /// Scene commands the device encoded, as quorra counted them.
    pub commands: u32,
    /// Of those, how many reached no pixel of the target and were dropped before any geometry
    /// was built (quorra's ADR 0015) — the share of a frame the window did not need.
    pub commands_culled: u32,
    /// Bytes quorra scheduled for transfer to the device this frame.
    pub bytes_uploaded: u64,
}

/// Everything about a frame that decides its scene, apart from the chrome.
///
/// Two frames with equal keys **and** equal overlays ([`Retained::draws`]) were built from the
/// same inputs, so the scene one of them produced is the scene the other would.
///
/// The floating-point fields compare by `PartialEq` rather than by bits, unlike the key quorra
/// keeps one layer down, and the difference costs nothing here: a `NaN` compares unequal and so
/// rebuilds, which is the safe direction, and `-0.0 == 0.0` is the one case where equality is
/// wider than identity — a placement that changed sign of zero draws the page in the same
/// pixels, because every coordinate it produces differs only in the sign of a zero.
#[derive(Debug, PartialEq)]
struct SceneKey {
    width: u32,
    height: u32,
    background: Color,
    /// Where the page was placed, and whether there was one.
    ///
    /// *Which* page is not here: identity lives in [`Retained::page`], which is the `Arc` that
    /// makes the address mean something, and [`Retained::draws`] asks it. One representation of
    /// one fact, rather than an address here and the pin that keeps it unique somewhere else.
    page: Option<TargetSpec>,
    /// Which CPU raster stood in for the page, by a serial rather than by identity.
    ///
    /// **A raster frame never reuses a scene, and the serial is how that is stated rather than
    /// remembered.** [`PresentFrame::raster`] is bytes the processor produced for *this* frame
    /// and handed over by reference: there is no allocation this slot can pin and no identity
    /// to key on, so a key that recorded only "a raster was here" would let the next raster's
    /// frame replay the previous one's pixels. What that costs is one upload and one encode per
    /// fallback frame, against a full CPU rasterisation of the page on the same frame — which
    /// is what the fallback path is, and what `doc/todo/45` would have to cache first for this
    /// to be worth an identity.
    raster: Option<u64>,
}

/// One frame's scene, kept for the frames that ask for it again.
#[derive(Debug)]
struct Retained {
    key: SceneKey,
    /// The page this scene was built from — **the identity, and the pin that makes it one**.
    ///
    /// Its address is what [`Self::draws`] compares, and holding the `Arc` is what stops the
    /// allocator handing that address to the next page ([`crate::cache`]'s argument, and
    /// [`PresentFrame::page`]'s). The outlines the list carries stay reachable for the same
    /// reason [`Self::overlays`] gives. It costs one display list held past the page turn that
    /// replaced it — the posture the host's own `presentation.rs` already has, for the transition
    /// that may want to draw from it.
    page: Option<Arc<DisplayList>>,
    /// The chrome this scene was built from, by value.
    ///
    /// Held rather than keyed by address for two reasons, and the second is the one that makes
    /// the whole scheme work. The chrome is rebuilt from the host's own state on every frame
    /// (`Overlays::of`), so its allocations are this frame's and an address would either miss
    /// every time or match a different list that landed on the same one. And holding it keeps
    /// the outlines it uploaded *reachable*: [`crate::cache`] releases an entry whose pin is the
    /// only reference left to that allocation, so overlays dropped after their frame would take
    /// their cache entries with them, bump quorra's resource generation, and cost every frame an
    /// encode — the reuse defeating itself through the cache it shares with the page.
    overlays: Vec<DisplayList>,
    scene: quorra_gpu::RetainedScene,
    /// This scene's own per-frame resources: split clips, dashed strokes, soft masks, images.
    ///
    /// **Released when the scene is replaced, not after the frame that built them.** The
    /// retained scene names these ids until something else takes its place, so releasing them
    /// at the end of their frame would leave the handle holding a scene whose resources are
    /// gone — and the next re-encode of it would be refused by name rather than drawn.
    transient: Vec<ResourceId>,
}

impl Retained {
    /// Whether this is the scene `frame` asks for.
    ///
    /// Three questions, in the order that rejects soonest: [`SceneKey`]'s derived comparison, the
    /// page by identity ([`Self::page`]), the chrome by value ([`Self::overlays`]).
    fn draws(&self, frame: &PresentFrame<'_>, key: &SceneKey) -> bool {
        self.key == *key
            && self.page.as_ref().map(identity) == frame.page.map(|(list, _)| identity(list))
            && self.overlays.len() == frame.overlays.len()
            && self
                .overlays
                .iter()
                .zip(frame.overlays)
                .all(|(kept, asked)| kept == *asked)
    }
}

/// A display list's address, which is a page's identity for exactly as long as something pins it.
///
/// The same construction, and the same one-line justification, as `cache::key`.
fn identity(list: &Arc<DisplayList>) -> usize {
    Arc::as_ptr(list) as usize
}

/// The frame's scene, built when it has to be and reused when it does not (ADR 0351).
///
/// **What this exists to stop paying for.** quorra's `encode` walks the scene's commands,
/// rasterises their coverage and lays out their instances on every `Device::render` call, and on
/// the project owner's own document that is a median 233.8 ms of a 393.1 ms frame for an answer
/// that cannot differ — 28 frames of one page at one view, the display list unchanged after the
/// first (`doc/todo/44` §3). `quorra_gpu::RetainedScene` replays that encode when nothing an
/// encode reads has moved, and this is the half of the bargain that lives here: the scene it
/// holds must stop being rebuilt, because a handle handed a freshly built scene has nothing to
/// replay.
///
/// **A rebuild is the only frame that translates, uploads, releases or evicts.** That is not
/// tidiness; each of the other three would defeat the reuse or break it:
///
/// - `Device::release` bumps a generation quorra keys every retained encode on, so a frame that
///   released its transients would invalidate the encode it had just stored — and the re-encode
///   after it would name resources that no longer exist. [`Retained::transient`] is where they
///   wait instead.
/// - The caches' own eviction is keyed on a frame clock, and an entry this frame did not *look
///   up* is evictable. A reused frame looks nothing up, so advancing that clock on one would
///   make the live page's outlines evictable — and evicting them would leave the retained scene
///   naming released ids. So the clock counts *scenes* rather than frames: it advances on a
///   rebuild and on nothing else, which is exactly what it meant when every frame was one.
/// - A rebuild on every frame is the same thing as not adopting any of this, which is what
///   [`FrameCost::encode_source`] is in the trace to say out loud.
#[derive(Debug, Default)]
pub(crate) struct FrameSlot {
    held: Option<Retained>,
    /// How many frames have carried a CPU raster: see [`SceneKey::raster`].
    rasters: u64,
}

impl FrameSlot {
    /// Draws one frame into `into`, building its scene only if the last one will not do.
    ///
    /// Fills the parts of `cost` that are this call's; the caller owns [`FrameCost::total`],
    /// which is its own timer.
    ///
    /// # Errors
    ///
    /// Whatever the translation, the eviction or the device refused. As everywhere else in this
    /// crate, the frame's own error outranks a release problem.
    pub(crate) fn render(
        &mut self,
        device: &mut quorra_gpu::Device,
        caches: &mut crate::cache::ResourceCaches,
        background: Color,
        frame: &PresentFrame<'_>,
        into: quorra_gpu::Target<'_>,
        cost: &mut FrameCost,
    ) -> Result<quorra_gpu::Frame, QuorraRasterError> {
        let began = std::time::Instant::now();
        let key = SceneKey::of(frame, background, &mut self.rasters);
        let mut release_error: Option<quorra_gpu::DeviceError> = None;
        let held = match &mut self.held {
            Some(held) if held.draws(frame, &key) => held,
            slot => {
                // Settling first, and with the cache's clock still on the scene being replaced:
                // that keeps `evict_settled`'s rule exactly what it was — an entry the *current*
                // scene used is never evicted — while moving it off the frames that reuse one.
                let settling = std::time::Instant::now();
                if let Some(previous) = slot.take() {
                    for id in previous.transient {
                        if let Err(error) = device.release(id) {
                            release_error.get_or_insert(error);
                        }
                    }
                }
                let evicted = caches.evict_settled(device);
                cost.settle = settling.elapsed();
                evicted?;

                caches.begin_frame();
                let mut builder = quorra_scene::SceneBuilder::new();
                let mut transient: Vec<ResourceId> = Vec::new();
                let built = build(
                    device,
                    caches,
                    background,
                    &mut builder,
                    frame,
                    &mut transient,
                );
                cost.uploads = caches
                    .stored()
                    .saturating_add(u32::try_from(transient.len()).unwrap_or(u32::MAX));
                if let Err(problem) = built {
                    // Nothing will draw this scene, so its resources go back now rather than
                    // waiting for a replacement that will never be asked for.
                    for id in transient {
                        if let Err(error) = device.release(id) {
                            release_error.get_or_insert(error);
                        }
                    }
                    return Err(problem);
                }
                slot.insert(Retained {
                    key,
                    page: frame.page.map(|(list, _)| Arc::clone(list)),
                    overlays: frame.overlays.iter().map(|list| (*list).clone()).collect(),
                    scene: quorra_gpu::RetainedScene::new(builder.finish()),
                    transient,
                })
            }
        };
        // The two are disjoint by construction — a rebuild does one and then the other, a reuse
        // does neither — so this reports what the trace's columns have always reported.
        cost.scene = began.elapsed().saturating_sub(cost.settle);

        let submitted = std::time::Instant::now();
        // The target transform is baked into every command at translation (`Encoder::placed`),
        // so the viewport itself is identity — which is also why a scroll or a zoom reaches this
        // slot's key rather than quorra's: quorra sees one unchanging viewport.
        let viewport =
            quorra_gpu::Viewport::full(frame.width, frame.height, quorra_scene::Affine::IDENTITY);
        let rendered = device.render_retained(&mut held.scene, &viewport, into);
        cost.device = submitted.elapsed();
        cost.retained_bytes = held.scene.retained_bytes();

        let drawn = rendered?;
        cost.read(&drawn);
        if let Some(error) = release_error {
            return Err(error.into());
        }
        Ok(drawn)
    }

    /// Forgets the retained scene, releasing what it alone was holding.
    ///
    /// # Errors
    ///
    /// [`QuorraRasterError::Device`] if the device refused a release.
    pub(crate) fn discard(
        &mut self,
        device: &mut quorra_gpu::Device,
    ) -> Result<(), QuorraRasterError> {
        let mut release_error: Option<quorra_gpu::DeviceError> = None;
        if let Some(previous) = self.held.take() {
            for id in previous.transient {
                if let Err(error) = device.release(id) {
                    release_error.get_or_insert(error);
                }
            }
        }
        release_error.map_or(Ok(()), |error| Err(error.into()))
    }
}

impl SceneKey {
    /// The key of the frame about to be drawn.
    fn of(frame: &PresentFrame<'_>, background: Color, rasters: &mut u64) -> Self {
        Self {
            width: frame.width,
            height: frame.height,
            background,
            page: frame.page.map(|(_, target)| target),
            raster: frame.raster.map(|_| {
                *rasters = rasters.saturating_add(1);
                *rasters
            }),
        }
    }
}

impl FrameCost {
    /// Reads back what quorra measured and counted, whichever way the frame was drawn.
    fn read(&mut self, drawn: &quorra_gpu::Frame) {
        let timings = drawn.timings();
        self.encode = timings.encode;
        self.upload = timings.upload;
        self.execute = timings.execute;
        self.execute_measured = matches!(
            timings.execute_provenance,
            quorra_gpu::TimingProvenance::TimestampQueries
        );
        self.encode_source = Some(drawn.encode_source());
        let counters = drawn.counters();
        self.commands = counters.commands;
        self.commands_culled = counters.commands_culled;
        self.bytes_uploaded = counters.bytes_uploaded;
    }
}

/// The window-owning form of the backend: quorra's device holds the surface, and
/// [`QuorraPresenter::present`] draws and presents one frame.
#[derive(Debug)]
pub struct QuorraPresenter {
    device: quorra_gpu::Device,
    background: Color,
    caches: crate::cache::ResourceCaches,
    slot: FrameSlot,
    last: FrameCost,
}

impl QuorraPresenter {
    /// The instance a presenter would make for itself, made early.
    ///
    /// **The launch path's one lever that crosses this boundary.** A `wgpu::Instance` is the
    /// driver loader, it needs no window, no surface and no event loop — and on this machine it
    /// is roughly 80% of what bringing a device up blocks for (quorra's ADR 0014). So a host
    /// creates it on a thread started before the window exists and hands it to
    /// [`Self::with_instance`]; `wgpu::Instance` is `Send + Sync`, so the thread that made it can
    /// give it away.
    ///
    /// It must be *this* function rather than a `wgpu::Instance::new` of the host's own: the
    /// descriptor has to match the one quorra's own constructors use, and a host that guessed it
    /// would find out at `create_surface`.
    #[must_use]
    pub fn instance() -> quorra_gpu::wgpu::Instance {
        quorra_gpu::create_instance()
    }

    /// [`Self::instance`], restricted to the driver stacks the host names.
    ///
    /// **An escape hatch from a driver, not a speed knob**, and the distinction is quorra's ADR
    /// 0017 as much as this project's ADR 0221: restricting the instance to one backend halves
    /// `wgpu::Instance::new` and gives every millisecond of it back in `request_adapter`, so the
    /// total is unchanged. What it is for is a machine with two driver stacks and one of them
    /// broken — the project owner's Windows machine, whose Intel Vulkan driver crashed while
    /// wgpu's hub order was choosing Vulkan over DX12 (`doc/QUORRA_FEEDBACK.md` section 12).
    ///
    /// Naming a set this machine cannot supply is not an error here; it becomes
    /// [`quorra_gpu::DeviceError::NoAdapter`] with an **empty** `available` list at
    /// [`Self::with_instance`], which is the signature a caller should report as "this machine
    /// has no such adapter" rather than as a broken driver.
    #[must_use]
    pub fn instance_with(backends: quorra_gpu::wgpu::Backends) -> quorra_gpu::wgpu::Instance {
        quorra_gpu::create_instance_with(backends)
    }

    /// The adapters *an instance* can see, which is the only list a host that restricted its
    /// backends may offer.
    ///
    /// `quorra_gpu::Device::adapter_names` makes its own all-backends instance and so answers a
    /// different question — what the machine has, rather than what these constructors will
    /// choose among. One GPU appears once per backend that can drive it, under the same device
    /// name each time, so a repeated name is that and not a second card.
    #[must_use]
    pub fn adapters_on(instance: &quorra_gpu::wgpu::Instance) -> Vec<String> {
        quorra_gpu::Device::adapter_names_on(instance)
    }

    /// A presenter owning a surface on `window`, on an instance the caller made earlier.
    ///
    /// See [`Self::instance`] for why a host would. Everything else is [`Self::new`]'s.
    ///
    /// # Errors
    ///
    /// [`QuorraRasterError::Device`] when no adapter can present to the window.
    pub fn with_instance(
        instance: &quorra_gpu::wgpu::Instance,
        window: impl Into<quorra_gpu::wgpu::SurfaceTarget<'static>>,
    ) -> Result<Self, QuorraRasterError> {
        let device = quorra_gpu::Device::for_surface_with_instance(
            instance,
            window,
            &quorra_gpu::Options::default(),
        )?;
        Ok(Self::around(device))
    }

    /// A presenter owning a surface on `window` — anything convertible to the
    /// re-exported [`quorra_gpu::wgpu::SurfaceTarget`], which a winit window is.
    ///
    /// Returns as soon as the device exists; shaders compile in the background.
    ///
    /// # Errors
    ///
    /// [`QuorraRasterError::Device`] when no adapter can present to the window.
    pub fn new(
        window: impl Into<quorra_gpu::wgpu::SurfaceTarget<'static>>,
    ) -> Result<Self, QuorraRasterError> {
        let device = quorra_gpu::Device::for_surface(window, &quorra_gpu::Options::default())?;
        Ok(Self::around(device))
    }

    /// The presenter around a device, however that device was brought up.
    fn around(device: quorra_gpu::Device) -> Self {
        // wgpu reports validation failures and lost devices to a handler whose
        // default is silence — the one way this window could stop updating
        // without a word. Same lesson, same sentence as the Vello host had.
        device
            .wgpu()
            .0
            .on_uncaptured_error(Arc::new(|error: quorra_gpu::wgpu::Error| {
                eprintln!("note: the graphics device reported: {error}");
            }));
        Self {
            device,
            background: Color::WHITE,
            caches: crate::cache::ResourceCaches::new(),
            slot: FrameSlot::default(),
            last: FrameCost::default(),
        }
    }

    /// Chooses the coverage lane for the frames after this call.
    ///
    /// Forwarded to `quorra_gpu::Device::set_coverage`, and the reason it is forwarded
    /// rather than fixed when the presenter is built is that the right answer changes
    /// while a document is open. quorra's two lanes have opposite cost curves and the
    /// crossover is a magnification — see `doc/quorra-gpu-coverage.md` and the caller
    /// in `viewer-ui`, which is the crate that knows what magnification the next frame
    /// is at. Nothing here decides; this only carries the decision.
    pub fn set_coverage(&mut self, coverage: quorra_gpu::Coverage) {
        self.device.set_coverage(coverage);
    }

    /// The adapter quorra selected, for reports.
    #[must_use]
    pub fn adapter_description(&self) -> &str {
        self.device.description()
    }

    /// What bringing the device up cost, in the three parts quorra measures it in.
    ///
    /// **`CLAUDE.md` makes this a first-class number**, since the project owner's decision that
    /// page one goes to the graphics device: with GPU bring-up on the critical path by choice,
    /// what it costs is part of time-to-first-page and may not be left unmeasured. Pipeline
    /// compilation is `None` while the background thread is still at it, which is the normal
    /// answer on the launch path and is itself the point — nothing here waits for warmth.
    #[must_use]
    pub fn startup(&self) -> quorra_gpu::StartupTimings {
        self.device.startup()
    }

    /// What the last [`Self::present`] cost, stage by stage.
    ///
    /// Read after the call rather than returned by it, for the same reason [`Self::startup`] is
    /// a separate question: `present`'s answer is whether the frame reached the window, and a
    /// caller that does not care about the accounting should not have to destructure it. The
    /// figures are those of the most recent call, drawn or refused; a refusal that never reached
    /// the device leaves the device's own three at zero, which is itself the answer.
    #[must_use]
    pub fn last_frame(&self) -> FrameCost {
        self.last
    }

    /// Draws one frame and presents it.
    ///
    /// **A frame whose page, placement, size, medium and chrome are all the ones the last
    /// frame had costs neither a translation nor an encode** (ADR 0351): its scene is the scene
    /// [`FrameSlot`] is already holding, and quorra replays the device commands it made from
    /// it. [`FrameCost::encode_source`] says which of the two happened.
    ///
    /// # Errors
    ///
    /// A refusal names what could not be drawn or why the surface was not
    /// presentable — [`QuorraRasterError::Render`] wrapping
    /// [`quorra_gpu::RenderError::SurfaceUnavailable`] carries the swapchain
    /// states a host reacts to (outdated, occluded, lost) rather than reports.
    pub fn present(&mut self, frame: PresentFrame<'_>) -> Result<(), QuorraRasterError> {
        // Three clock reads and a struct copy per frame, spent whether anyone reads them or
        // not — see [`FrameCost`] for why that is the deliberate choice rather than a flag.
        let began = std::time::Instant::now();
        self.last = FrameCost::default();
        if frame.width == 0 || frame.height == 0 {
            return Ok(()); // minimised: nothing to present to
        }
        let outcome = self.slot.render(
            &mut self.device,
            &mut self.caches,
            self.background,
            &frame,
            quorra_gpu::Target::Surface,
            &mut self.last,
        );
        self.last.total = began.elapsed();
        outcome.map(|_| ())
    }
}

/// Assembles the frame's scene: medium, page (or its raster stand-in), overlays.
///
/// A free function rather than a method because there are two devices that draw a *window's*
/// frame and only one of them owns a surface: [`QuorraPresenter::present`] renders it to the
/// swapchain, and [`crate::QuorraRasterizer::rasterize_frame`] renders the same scene to a
/// readback so that a test can look at it. A second copy of this would be two scenes that drift.
pub(crate) fn build(
    device: &mut quorra_gpu::Device,
    caches: &mut crate::cache::ResourceCaches,
    background: Color,
    builder: &mut quorra_scene::SceneBuilder,
    frame: &PresentFrame<'_>,
    transient: &mut Vec<ResourceId>,
) -> Result<(), QuorraRasterError> {
    // The medium first: a surface frame has no compositor behind it to impose
    // on, so the background is the bottom of the scene itself.
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    let (w, h) = (frame.width as f32, frame.height as f32);
    builder.rect(
        quorra_scene::Rect::new(
            quorra_scene::Point::new(0.0, 0.0),
            quorra_scene::Point::new(w, h),
        ),
        quorra_scene::Affine::IDENTITY,
        crate::scene::colour(background),
        None,
        None,
    )?;

    if let Some((list, target)) = frame.page {
        Encoder::new(device, list, target, caches, transient).commands(builder, list.commands())?;
    }
    if let Some(raster) = frame.raster {
        let image = device.upload_image(&quorra_scene::ImageSpec {
            width: raster.width,
            height: raster.height,
            data: Arc::from(raster.data.as_slice()),
        })?;
        transient.push(image.into());
        #[expect(
            clippy::cast_precision_loss,
            reason = "raster dimensions are far below f32's exact integer range"
        )]
        let (rw, rh) = (raster.width as f32, raster.height as f32);
        // The unit square carries the image with its top row at unit y = 1
        // (§8.9.5), so placing the top row at the window's y = 0 takes a flip.
        builder.image(
            image,
            quorra_scene::Affine {
                a: rw,
                b: 0.0,
                c: 0.0,
                d: -rh,
                e: 0.0,
                f: rh,
            },
            1.0,
            quorra_scene::ImageFilter::Nearest,
            None,
            quorra_scene::BlendMode::Normal,
            None,
        )?;
    }
    for list in frame.overlays {
        let spec = TargetSpec {
            width: frame.width,
            height: frame.height,
            transform: Transform::IDENTITY,
        };
        Encoder::new(device, list, spec, caches, transient).commands(builder, list.commands())?;
    }
    Ok(())
}
