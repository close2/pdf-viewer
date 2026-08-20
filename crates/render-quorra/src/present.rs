//! Tier 3: the window's frame drawn into textures the host owns, no readback.
//!
//! This is the tier `RENDER_LIBRARY.md` section 6.1 measurements pointed at from the start: the
//! readback that dominates a `rasterize` call simply does not exist here — quorra
//! renders the scene into a texture, and the pixels never cross back to the CPU.
//! Each list is drawn at its own placement (which is why the adapter bakes placements
//! into commands rather than into the viewport): the page under its target transform, or a
//! CPU-rendered raster standing in for it, and the window-pixel overlay lists —
//! selection, sidebar, modal.
//!
//! # Two textures rather than one, and why the split is here
//!
//! [`QuorraWindowRenderer::render`] draws a window's frame into **two** host textures: the page
//! with the medium under it, and the chrome on transparency. A host that put both in one raster
//! could only ever move both together, and moving the page is exactly what a host does while the
//! next frame is still being drawn — so the seam is a texture boundary rather than a compositing
//! rule somebody has to remember. What is done with the two afterwards is entirely the host's;
//! this crate hands back rasters and knows nothing about where they go.
//!
//! **Nothing here presents.** Since quorra's ADR 0056 the surface leaves the device with
//! [`quorra_gpu::Presenter`], and [`QuorraWindowRenderer::detach_presenter`] is the whole of what
//! this crate has to say about it: the presenter is `Send`, the host moves it to whichever thread
//! owns its window, and the device stays here drawing pages. Before that split this type owned
//! the surface and one call did both, which meant a frame of a heavy page held the only path to
//! the screen for as long as it took.

use std::sync::Arc;

use pdf_render::{Color, DisplayList, Medium, Raster, TargetSpec, Transform};
use quorra_scene::ResourceId;

use crate::QuorraRasterError;
use crate::scene::{Encoder, FunctionPaints};

/// One frame of the window, everything included.
#[derive(Debug, Clone, Copy)]
pub struct PresentFrame<'a> {
    /// The surface size in device pixels.
    pub width: u32,
    /// See [`PresentFrame::width`].
    pub height: u32,
    /// The pages on the screen and where each goes — empty when a raster stands in for them.
    ///
    /// **A list since ISO 32000-2 Table 29's `/PageLayout` reached a tier-2 host** (ADR 0442).
    /// `OneColumn` puts several pages in one window and each carries its own [`TargetSpec`], so
    /// the frame is a sequence of placed lists exactly as [`Self::overlays`] already was; they
    /// are drawn in the order given. One entry is `SinglePage`, which is Table 29's own default.
    ///
    /// **How the pages compose is not decided here and could not be**: each list's placement is
    /// stated by its own `TargetSpec`, so a backend executes an arrangement rather than choosing
    /// one — which is what keeps `doc/traps/pixels-and-rasterisers.md` trap 2 satisfied with two
    /// backends drawing the same frame.
    ///
    /// **The `Arc` is the identity a frame is reused by, and it is why these are not plain
    /// references** (ADR 0351). [`FrameSlot`] keeps the pages' scene across frames and decides
    /// whether to rebuild it by the address of these samples, so it must be able to *pin* those
    /// addresses — the same ABA argument [`crate::cache`] makes for every other key in this
    /// crate. A borrowed `&DisplayList` could be dropped between two frames and the allocator
    /// could hand the same address to the next page, which is a stale page drawn with no
    /// report; holding the `Arc` makes the address unique for as long as the slot keeps it.
    pub pages: &'a [(&'a Arc<DisplayList>, TargetSpec)],
    /// A pre-rendered page, drawn 1:1 from the window's top-left corner: the CPU
    /// fallback path, which must stay presentable even when the page itself is
    /// something the device refused.
    pub raster: Option<&'a Raster>,
    /// Window-pixel display lists drawn over the page, in order (selection
    /// highlights, sidebar, modal card — chrome crosses as geometry, not pixels).
    pub overlays: &'a [&'a DisplayList],
}

/// The two textures one window frame is drawn into.
///
/// **Both are the host's**, created with [`QuorraWindowRenderer::layer_texture`] so that they
/// carry the two usages a texture needs to be rendered into *and* sampled afterwards — quorra's
/// `LayerProblem::NotSampleable` is the refusal a texture that has only ever been a render target
/// earns, and it is a trap worth naming here rather than at the point it fires.
#[derive(Debug, Clone, Copy)]
pub struct WindowTextures<'a> {
    /// The page, with the window's medium under it: opaque everywhere, so a host may move it
    /// about without a hole appearing in the middle of it.
    pub page: &'a quorra_gpu::wgpu::Texture,
    /// The chrome, on transparency, in window pixels — so that it composites over the page
    /// wherever the host puts that page, and shows the page through everywhere it draws nothing.
    pub chrome: &'a quorra_gpu::wgpu::Texture,
}

/// What one frame of [`QuorraWindowRenderer::render`] spent, part by part.
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
    /// Inside `scene`: the part of it spent *inside* quorra's own `upload_*` calls.
    ///
    /// **The phase `doc/todo/45` §5 said nothing could see into**, and the launch path is where
    /// it matters: a first frame builds every outline the page states, so on the project owner's
    /// own drawing `scene` is a fifth of the whole launch while a *second* frame of the same page
    /// spends almost nothing there. One number could not say whether that fifth is this crate's
    /// walk of the display list or the boundary it hands each resource across, and the two have
    /// different owners. ADR 0423; `crate::cache::ResourceCaches::handed` is where it accumulates
    /// and what it costs to take.
    ///
    /// Zero on a frame that reused its scene, exactly as [`Self::scene`] is.
    pub handover: std::time::Duration,
    /// [`Self::handover`]'s denominator: path segments handed over in this frame's outlines.
    ///
    /// A count beside a duration is read as that duration's denominator whether or not it is one
    /// (ADR 0387), so the one that *is* one is here rather than left to be inferred from
    /// [`Self::uploads`] — an outline's upload costs by the segment, and a page of glyphs and a
    /// draughtsman's line work differ by two orders of magnitude in segments per resource.
    pub outline_segments: u64,
    /// [`quorra_gpu::Device::render`] end to end: the three below, plus acquiring the swapchain
    /// texture, presenting it and reading the instrumentation back.
    pub device: std::time::Duration,
    /// Inside `device`: turning the scene into device commands.
    pub encode: std::time::Duration,
    /// Inside `device`: preparing and scheduling this frame's CPU→GPU transfers.
    pub upload: std::time::Duration,
    /// Inside `device`: the drawing passes themselves.
    pub execute: std::time::Duration,
    /// Inside `device`: copying the finished frame back off the device, mapping it, and
    /// converting premultiplied alpha to straight.
    ///
    /// **Zero for every frame that goes to a window**, which is why this field was missing for
    /// twenty-six sessions and why leaving it missing was wrong. A window frame is drawn into a
    /// texture and never read, so [`QuorraWindowRenderer::render`] reports zero here and nothing
    /// about the viewer's own trace changes. What *does* read back is
    /// [`crate::QuorraRasterizer::rasterize_frame`] — every corpus and oracle page, and the
    /// zoom-step instrument — and there the copy of a multi-megabyte frame was landing in the
    /// remainder a caller computes by subtracting the other three. A phase quorra measures and a
    /// host drops is a phase a host will attribute to something else (ADR 0387).
    pub readback: std::time::Duration,
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
    /// Whether the device repacked its glyph atlas after this frame, throwing every tile's
    /// placement away.
    ///
    /// **The one event outside this crate that can make [`Self::encode_source`] read `Encoded`
    /// on a frame [`FrameSlot`] meant to replay** (quorra's ADR 0050). ADR 0351 enumerated every
    /// input on *this* side of the boundary and gave each its own test, and left the device's own
    /// invalidation unobservable: a window whose retained encode dies every frame looked exactly
    /// like a key that keeps missing. It is not a rate and not a duration — a page that changes
    /// settles after at most one repack, so `true` frame after frame is the pathology and one
    /// `true` is the ordinary cost of a page turn.
    pub atlas_repacked: bool,
    /// Atlas bytes this frame's distinct glyph tiles asked for, cache hits included.
    ///
    /// The number [`Self::atlas_repacked`] is only actionable against, which is why it is here
    /// and its three companions are not: it is what holding *all* of this page's tiles would
    /// cost, so it is what says whether a repeated repack means the atlas is holding another
    /// page's tiles or that this page has never fitted the budget the device was built with.
    pub atlas_working_set_bytes: u64,
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
    /// The medium under everything, or `None` for a scene drawn straight onto transparency.
    ///
    /// **`None` is the chrome's**, and it is part of the key rather than a fixed property of a
    /// slot because it decides the bottom of the scene: a frame drawn over a medium and the same
    /// frame drawn over nothing are two pictures, and a key that could not tell them apart would
    /// let one replay for the other.
    ///
    /// **Both of its colours are in the key**, because both reach the pixels: §11.4.7's 𝑊 under
    /// each page and the surround outside every page are two rectangles of the scene, so a frame
    /// that changed either is a frame that has to be built again.
    medium: Option<Medium>,
    /// Where each page of the arrangement was placed, in the order they are drawn.
    ///
    /// *Which* pages these are is not here: identity lives in [`Retained::pages`], which are the
    /// `Arc`s that make the addresses mean something, and [`Retained::draws`] asks them. One
    /// representation of one fact, rather than an address here and the pin that keeps it unique
    /// somewhere else. The **count** is part of the key by construction, which is what makes a
    /// scroll that puts a further page on the screen rebuild rather than replay.
    pages: Vec<TargetSpec>,
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
    /// The pages this scene was built from — **the identities, and the pins that make them ones**.
    ///
    /// Their addresses are what [`Self::draws`] compares, and holding the `Arc`s is what stops the
    /// allocator handing one of those addresses to the next page ([`crate::cache`]'s argument, and
    /// [`PresentFrame::pages`]'s). The outlines the lists carry stay reachable for the same
    /// reason [`Self::overlays`] gives. It costs the arrangement's display lists held past the
    /// page turn that replaced them — the posture the host's own `presentation.rs` already has,
    /// for the transition that may want to draw from one.
    pages: Vec<Arc<DisplayList>>,
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
    /// pages by identity ([`Self::pages`]), the chrome by value ([`Self::overlays`]).
    ///
    /// The pages need no length check of their own: [`SceneKey::pages`] holds one placement per
    /// page, so a key that compared equal has already said the two frames show as many.
    fn draws(&self, frame: &PresentFrame<'_>, key: &SceneKey) -> bool {
        self.key == *key
            && self
                .pages
                .iter()
                .zip(frame.pages)
                .all(|(kept, (asked, _))| identity(kept) == identity(asked))
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
        medium: Option<Medium>,
        frame: &PresentFrame<'_>,
        into: quorra_gpu::Target<'_>,
        reported: &mut Reported<'_>,
    ) -> Result<quorra_gpu::Frame, QuorraRasterError> {
        let Reported {
            cost,
            functions,
            phases,
        } = reported;
        let began = std::time::Instant::now();
        let key = SceneKey::of(frame, medium, &mut self.rasters);
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
                // Cleared here rather than per frame, because what it describes is the *scene*:
                // a replayed frame draws the paints this build chose, and forgetting them on the
                // frame after would make a still page look as though it had stopped using them.
                functions.clear();
                let built = build(
                    device,
                    caches,
                    medium,
                    &mut builder,
                    frame,
                    &mut transient,
                    functions,
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
                    pages: frame
                        .pages
                        .iter()
                        .map(|(list, _)| Arc::clone(list))
                        .collect(),
                    overlays: frame.overlays.iter().map(|list| (*list).clone()).collect(),
                    scene: quorra_gpu::RetainedScene::new(builder.finish()),
                    transient,
                })
            }
        };
        // The two are disjoint by construction — a rebuild does one and then the other, a reuse
        // does neither — so this reports what the trace's columns have always reported.
        cost.scene = began.elapsed().saturating_sub(cost.settle);
        // A subdivision of the line above rather than a phase beside it: `begin_frame` zeroes it,
        // and a frame that reused its scene never reached one, so a reuse reports zero for both.
        cost.handover = caches.handed();
        cost.outline_segments = caches.segments();

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
        cost.read(&drawn, phases);
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
    fn of(frame: &PresentFrame<'_>, medium: Option<Medium>, rasters: &mut u64) -> Self {
        Self {
            width: frame.width,
            height: frame.height,
            medium,
            pages: frame.pages.iter().map(|(_, target)| *target).collect(),
            raster: frame.raster.map(|_| {
                *rasters = rasters.saturating_add(1);
                *rasters
            }),
        }
    }
}

impl FrameCost {
    /// Reads back what quorra measured and counted, whichever way the frame was drawn.
    ///
    /// `phases` is filled rather than replaced, which is the whole reason it is an argument
    /// instead of a field: [`quorra_gpu::Timings::phases`] is a `Vec` quorra allocates every
    /// frame, and a host that cloned it would allocate every frame too. Cloning *into* a buffer
    /// the caller keeps reuses its capacity after the first frame, so what a still window pays
    /// for the finest instrument this boundary carries is a memcpy of a handful of entries.
    pub(crate) fn read(
        &mut self,
        drawn: &quorra_gpu::Frame,
        phases: &mut Vec<(&'static str, std::time::Duration)>,
    ) {
        let timings = drawn.timings();
        phases.clone_from(&timings.phases);
        self.encode = timings.encode;
        self.upload = timings.upload;
        self.execute = timings.execute;
        self.readback = timings.readback;
        self.execute_measured = matches!(
            timings.execute_provenance,
            quorra_gpu::TimingProvenance::TimestampQueries
        );
        self.encode_source = Some(drawn.encode_source());
        let counters = drawn.counters();
        self.commands = counters.commands;
        self.commands_culled = counters.commands_culled;
        self.bytes_uploaded = counters.bytes_uploaded;
        self.atlas_repacked = counters.atlas_repacked;
        self.atlas_working_set_bytes = counters.atlas_working_set_bytes;
    }

    /// Folds a second lane's accounting into this one, for a call that drew two.
    ///
    /// Every duration and every count adds, because the two lanes happened one after the other on
    /// one thread and their sum is what the call took. Three fields are not sums and say why
    /// where they are set: [`Self::total`] is the caller's own timer over both,
    /// [`Self::encode_source`] stays the *page's* — the lane whose reuse is worth reporting —
    /// and [`Self::execute_measured`] holds only where **both** lanes were measured, because a
    /// figure that is half a timestamp query and half a wall clock is neither.
    pub(crate) fn add(&mut self, other: Self) {
        self.scene = self.scene.saturating_add(other.scene);
        self.handover = self.handover.saturating_add(other.handover);
        self.outline_segments = self.outline_segments.saturating_add(other.outline_segments);
        self.device = self.device.saturating_add(other.device);
        self.encode = self.encode.saturating_add(other.encode);
        self.upload = self.upload.saturating_add(other.upload);
        self.execute = self.execute.saturating_add(other.execute);
        self.readback = self.readback.saturating_add(other.readback);
        self.settle = self.settle.saturating_add(other.settle);
        self.execute_measured = self.execute_measured && other.execute_measured;
        self.retained_bytes = self.retained_bytes.saturating_add(other.retained_bytes);
        self.uploads = self.uploads.saturating_add(other.uploads);
        self.commands = self.commands.saturating_add(other.commands);
        self.commands_culled = self.commands_culled.saturating_add(other.commands_culled);
        self.bytes_uploaded = self.bytes_uploaded.saturating_add(other.bytes_uploaded);
        self.atlas_repacked |= other.atlas_repacked;
        self.atlas_working_set_bytes = self
            .atlas_working_set_bytes
            .saturating_add(other.atlas_working_set_bytes);
    }
}

/// One scene the device keeps between frames, and the caches that keep its resources alive.
///
/// **Two of these rather than one, and it is a correctness requirement rather than tidiness.**
/// [`crate::cache`] evicts an entry the *current* scene did not look up, and its clock advances
/// on a rebuild; two scenes sharing one cache would therefore evict each other's outlines every
/// time either of them was rebuilt, leaving the other's retained encode naming released ids. The
/// page and the chrome are rebuilt on completely different occasions — a zoom rebuilds one, a
/// caret the other — so they get a lane each.
#[derive(Debug)]
struct Lane {
    caches: crate::cache::ResourceCaches,
    slot: FrameSlot,
}

impl Lane {
    /// An empty lane: no scene held and nothing cached.
    fn new() -> Self {
        Self {
            caches: crate::cache::ResourceCaches::new(),
            slot: FrameSlot::default(),
        }
    }
}

/// The window-drawing form of the backend: quorra's device draws a window's frame into two
/// textures the host owns, and the host puts them on its window itself.
///
/// **It no longer presents, and the name says so since the five-hundred-and-fifty-sixth
/// session.** quorra's ADR 0056 split the surface off the device into a `Send`
/// [`quorra_gpu::Presenter`]; [`Self::detach_presenter`] hands that over, and from then on this
/// type is a renderer and nothing else. Before the split one call drew *and* presented, so a
/// frame of a heavy page held the only path to the screen for as long as it took to draw.
#[derive(Debug)]
pub struct QuorraWindowRenderer {
    device: quorra_gpu::Device,
    /// §11.4.7's 𝑊 under each page, and what the window shows outside every page.
    ///
    /// **[`Medium::WINDOW`] rather than a rasteriser's [`Medium::PAGE_ONLY`]**, and the type is
    /// what makes that a decision rather than a habit: this renderer's whole subject is a
    /// surface larger than a page, so it is the one place in the tree where the two colours are
    /// different by default. A `Rasterizer` is handed one list and a target for it and takes one
    /// colour for the whole of it, which is what a page-sized raster wants.
    medium: Medium,
    /// The page, over the medium.
    page: Lane,
    /// The chrome, over transparency. See [`Lane`] for why it is not the page's.
    chrome: Lane,
    last: FrameCost,
    functions: FunctionPaints,
    phases: Vec<(&'static str, std::time::Duration)>,
}

impl QuorraWindowRenderer {
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
        let device =
            quorra_gpu::Device::for_surface_with_instance(instance, window, &crate::options())?;
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
        let device = quorra_gpu::Device::for_surface(window, &crate::options())?;
        Ok(Self::around(device))
    }

    /// The renderer around a device, however that device was brought up.
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
            medium: Medium::WINDOW,
            page: Lane::new(),
            chrome: Lane::new(),
            last: FrameCost::default(),
            functions: FunctionPaints::default(),
            phases: Vec::new(),
        }
    }

    /// What the scene last built did with §8.7.4.5.2's type 1 shadings: how many the device
    /// evaluated, and the ground on which it declined each of the rest (ADR 0376).
    ///
    /// The scene's rather than the frame's, which is the honest reading of a replayed frame: it
    /// draws the paints the last *build* chose, so this keeps saying what they were.
    #[must_use]
    pub fn last_function_paints(&self) -> &FunctionPaints {
        &self.functions
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

    /// The named spans quorra measured inside the last frame, in quorra's own words.
    ///
    /// **The finest instrument this boundary carries, and until ADR 0387 nothing on this side
    /// read it.** [`FrameCost`]'s four durations are a partition of one `Instant` span; these are
    /// whatever quorra chose to name inside them, and what is in the list depends on what the
    /// adapter and the options allow:
    ///
    /// - **One entry per drawing pass**, wherever the adapter offers timestamp queries — the
    ///   subdivision of [`FrameCost::execute`], and the reason [`FrameCost::execute_measured`]
    ///   exists to be asked first.
    /// - **`encode: geometry`, `encode: staging` and `encode: recording`**, wherever the device
    ///   was built with `quorra_gpu::Options::instrument_encode`. That is off by default and
    ///   costs a few per cent of `encode` when it is on (ADR 0368 measured 512.8 ms against
    ///   475.9), so a subdivision read from it is *shares* rather than absolutes.
    /// - **A one-off a frame absorbed**, such as a first-use pipeline compilation.
    ///
    /// So an empty slice is an ordinary answer and not a failure: it is what a device with no
    /// timestamp queries and no encode instrument has to say.
    ///
    /// - **`target acquire` and `present`**, the two host-side steps of
    ///   [`FrameCost::device`] that are in none of the three durations, which is what quorra's
    ///   own documentation of [`quorra_gpu::Timings`] directs a caller to read the remainder
    ///   against.
    ///
    /// **And reading them is what says the remainder is still unexplained.** On the project
    /// owner's own adapter the two together are under a *twentieth* of a millisecond on a frame
    /// whose unnamed span is over a hundred, so the span the viewer's trace calls `elsewhere`
    /// has a subdivision that accounts for none of it. Calling it a bound rather than a duration
    /// (ADR 0228) therefore stands, and `doc/QUORRA_FEEDBACK.md` section 29 is what went back with
    /// the numbers.
    #[must_use]
    pub fn last_phases(&self) -> &[(&'static str, std::time::Duration)] {
        &self.phases
    }

    /// Draws one window frame into two textures the host owns, and puts nothing anywhere.
    ///
    /// The page — with the window's medium under it — goes into `into.page`, and the chrome goes
    /// into `into.chrome` **on transparency**, in window pixels. Both must have been made by
    /// [`Self::layer_texture`] on this device and must be exactly `frame.width` by `frame.height`,
    /// which is what `Target::Texture` requires of any target.
    ///
    /// **A frame whose page, placement, size, medium and chrome are all the ones the last
    /// frame had costs neither a translation nor an encode** (ADR 0351): its scene is the scene
    /// [`FrameSlot`] is already holding, and quorra replays the device commands it made from
    /// it. [`FrameCost::encode_source`] says which of the two happened, for the page — the lane
    /// whose reuse is worth thousands of commands.
    ///
    /// [`Self::last_frame`] afterwards is the **sum** of the two lanes, because they are two
    /// halves of one window frame and a caller timing this call would otherwise have to add them
    /// up itself. [`Self::last_phases`] is the page's alone: quorra names its spans with static
    /// strings, so two lanes' worth of them would be one list with every name in it twice.
    ///
    /// # Errors
    ///
    /// Whatever either lane's translation, eviction or device call refused. The page's refusal
    /// outranks the chrome's: a window with no page and correct chrome is not a better answer
    /// than a window with neither, and what a caller needs is why the page is missing.
    pub fn render(
        &mut self,
        frame: PresentFrame<'_>,
        into: WindowTextures<'_>,
    ) -> Result<(), QuorraRasterError> {
        // Three clock reads and a struct copy per frame, spent whether anyone reads them or
        // not — see [`FrameCost`] for why that is the deliberate choice rather than a flag.
        let began = std::time::Instant::now();
        self.last = FrameCost::default();
        if frame.width == 0 || frame.height == 0 {
            return Ok(()); // minimised: nothing to draw into
        }
        let drawn = self.page.slot.render(
            &mut self.device,
            &mut self.page.caches,
            Some(self.medium),
            &PresentFrame {
                overlays: &[],
                ..frame
            },
            quorra_gpu::Target::Texture(into.page),
            &mut Reported {
                cost: &mut self.last,
                functions: &mut self.functions,
                phases: &mut self.phases,
            },
        );
        // The chrome is drawn even where the page refused. Its cost belongs in the accounting
        // either way, and a lane that skipped a frame would hold a scene keyed to a window the
        // page has since moved out of — so the next frame would rebuild it for nothing.
        let mut chrome_cost = FrameCost::default();
        // The chrome states no §8.7.4.5.2 program, and a record shared with the page would be
        // cleared by whichever lane rebuilt last — so this one is written and dropped.
        let mut chrome_functions = FunctionPaints::default();
        let mut chrome_phases = Vec::new();
        let chrome = self.chrome.slot.render(
            &mut self.device,
            &mut self.chrome.caches,
            None,
            &PresentFrame {
                pages: &[],
                raster: None,
                ..frame
            },
            quorra_gpu::Target::Texture(into.chrome),
            &mut Reported {
                cost: &mut chrome_cost,
                functions: &mut chrome_functions,
                phases: &mut chrome_phases,
            },
        );
        self.last.add(chrome_cost);
        self.last.total = began.elapsed();
        drawn?;
        chrome.map(|_| ())
    }

    /// A texture this device can render a window layer into **and** sample afterwards.
    ///
    /// **Both usages, which is the trap quorra's `LayerProblem::NotSampleable` exists to name**:
    /// `Target::Texture` needs `RENDER_ATTACHMENT` and putting the result on a window needs
    /// `TEXTURE_BINDING`, so a texture that has been a render target every frame is still
    /// unpresentable without the second. Asked for here, once, so that no host has to know.
    ///
    /// The format is the one `Target::Texture` renders in and the one a layer must be.
    #[must_use]
    pub fn layer_texture(&self, label: &str, width: u32, height: u32) -> quorra_gpu::wgpu::Texture {
        let (gpu, _) = self.device.wgpu();
        gpu.create_texture(&quorra_gpu::wgpu::TextureDescriptor {
            label: Some(label),
            size: quorra_gpu::wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: quorra_gpu::wgpu::TextureDimension::D2,
            format: quorra_gpu::wgpu::TextureFormat::Rgba8Unorm,
            usage: quorra_gpu::wgpu::TextureUsages::RENDER_ATTACHMENT
                | quorra_gpu::wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    /// One opaque texel of the window's medium, for a host to put under everything else.
    ///
    /// **A window's background is not a page and is not drawn by one.** A host that moves a
    /// finished page about reveals whatever the page does not cover, and what belongs there is
    /// the surround this renderer draws outside every page — never page white, which would assert
    /// that the page is blank there. One texel scaled over the window says exactly that and costs
    /// one textured quad; a window-sized medium texture would spend the bytes of a whole window
    /// to hold one colour.
    ///
    /// **This doc comment described the separation before the code had it.** Until the
    /// six-hundred-and-eleventh session `self.background` was one colour serving as both 𝑊 and
    /// the surround, so "never page white" was exactly what this texel was.
    ///
    /// Premultiplied, as every layer is — which for the opaque medium this renderer draws under
    /// every page is the same four bytes as the straight-alpha form `crate::scene` quantises to.
    #[must_use]
    pub fn medium_texture(&self) -> quorra_gpu::wgpu::Texture {
        let (gpu, queue) = self.device.wgpu();
        let extent = quorra_gpu::wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let texture = gpu.create_texture(&quorra_gpu::wgpu::TextureDescriptor {
            label: Some("the window's medium"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: quorra_gpu::wgpu::TextureDimension::D2,
            format: quorra_gpu::wgpu::TextureFormat::Rgba8Unorm,
            usage: quorra_gpu::wgpu::TextureUsages::TEXTURE_BINDING
                | quorra_gpu::wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &crate::scene::byte_colour(self.medium.surround),
            quorra_gpu::wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            extent,
        );
        texture
    }

    /// Takes the window's surface out of the device, so that it can be presented to from a thread
    /// that is not the one rendering (quorra's ADR 0056).
    ///
    /// `None` for a device with no surface, and `None` on the second call. **It compiles nothing
    /// and waits for nothing** — quorra's own documentation says it clones four handles and moves
    /// the surface state, asking the pipeline store nothing — which is why a host may call it on
    /// its launch path without putting a shader in front of page one (`CLAUDE.md`'s startup
    /// rules).
    #[must_use]
    pub fn detach_presenter(&mut self) -> Option<quorra_gpu::Presenter> {
        self.device.detach_presenter()
    }

    /// Gives the surface back, so that this device could present again.
    ///
    /// # Errors
    ///
    /// [`quorra_gpu::ForeignPresenter`] when the presenter came from another device — and it
    /// carries the presenter back out, because losing a window's surface over a caller's mix-up
    /// is not a cost any error path should impose.
    pub fn attach_presenter(
        &mut self,
        presenter: quorra_gpu::Presenter,
    ) -> Result<(), quorra_gpu::ForeignPresenter> {
        self.device.attach_presenter(presenter)
    }
}

/// What one [`FrameSlot::render`] hands back beside the frame: what it cost, and what it did
/// with §8.7.4.5.2's programs.
///
/// One borrow rather than two arguments because the two are the same thing — the caller's
/// record of the call — and because they are filled at different points inside it: the cost as
/// each stage ends, the paints only where the scene is rebuilt.
pub(crate) struct Reported<'a> {
    pub(crate) cost: &'a mut FrameCost,
    pub(crate) functions: &'a mut FunctionPaints,
    pub(crate) phases: &'a mut Vec<(&'static str, std::time::Duration)>,
}

/// Assembles the frame's scene: medium, page (or its raster stand-in), overlays.
///
/// A free function rather than a method because there are two kinds of device that draw a
/// *window's* frame: [`QuorraWindowRenderer::render`] draws it into textures a host will put on a
/// window, and [`crate::QuorraRasterizer::rasterize_frame`] draws the same scene into a readback
/// so that a test can look at it. A second copy of this would be two scenes that drift.
///
/// `medium` is what lies under everything, or `None` for a scene drawn straight onto
/// transparency — which is what a chrome layer is, and what lets it composite over a page the
/// host places wherever the view now is.
///
/// **A medium is two colours and they go in two places.** [`pdf_render::medium`] has the
/// reading: §11.4.7's 𝑊 is the initial colour of *the page* and covers §14.11.2.1's crop box and
/// no more, while what a window shows where there is no page is this program's own choice. So the
/// surround is one rectangle over the whole frame and 𝑊 is one rectangle per placed page, at that
/// page's own boundary — which is what makes the gap between two pages of a column visible at all.
pub(crate) fn build(
    device: &mut quorra_gpu::Device,
    caches: &mut crate::cache::ResourceCaches,
    medium: Option<Medium>,
    builder: &mut quorra_scene::SceneBuilder,
    frame: &PresentFrame<'_>,
    transient: &mut Vec<ResourceId>,
    functions: &mut FunctionPaints,
) -> Result<(), QuorraRasterError> {
    // The surround first, where there is one: a window frame has no compositor behind it to
    // impose on, so what lies outside every page is the bottom of the scene itself.
    if let Some(medium) = medium {
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let (w, h) = (frame.width as f32, frame.height as f32);
        fill(builder, 0.0, 0.0, w, h, medium.surround)?;
    }

    // One encoder per page, in the order the arrangement placed them. Table 29's `SinglePage` is
    // the one-element case and is what this loop was before ADR 0442; the pages of a column do
    // not overlap, so the order is the arrangement's rather than a compositing decision.
    for (list, target) in frame.pages {
        // §11.4.7's 𝑊 under *this* page and nowhere else. Drawn as part of the scene rather than
        // imposed afterwards for the reason `QuorraRasterizer::rasterize_frame` states: a window
        // has no compositor behind it, so the bottom of the scene is the medium. Its extent is
        // `pdf_render::page_area`, which every backend asks the same question of.
        if let Some(medium) = medium {
            let area = pdf_render::page_area(list, *target);
            fill(
                builder,
                area.min.x,
                area.min.y,
                area.max.x,
                area.max.y,
                medium.page,
            )?;
        }
        Encoder::new(device, list, *target, caches, transient, functions)
            .commands(builder, list.commands())?;
    }
    if let Some(raster) = frame.raster {
        // Through the same clock as every other upload (`Encoder::handing_over`): a fallback
        // frame's whole picture crosses here, and leaving the one upload that is not the
        // encoder's outside the count would make `handover` mean two things.
        let spec = quorra_scene::ImageSpec {
            width: raster.width,
            height: raster.height,
            data: Arc::from(raster.data.as_slice()),
        };
        let began = std::time::Instant::now();
        let uploaded = device.upload_image(&spec);
        caches.hand_over(began.elapsed());
        let image = uploaded?;
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
        Encoder::new(device, list, spec, caches, transient, functions)
            .commands(builder, list.commands())?;
    }
    Ok(())
}

/// One flat rectangle in window pixels, which is what both halves of the medium are.
fn fill(
    builder: &mut quorra_scene::SceneBuilder,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    colour: Color,
) -> Result<(), QuorraRasterError> {
    builder.rect(
        quorra_scene::Rect::new(
            quorra_scene::Point::new(left, top),
            quorra_scene::Point::new(right, bottom),
        ),
        quorra_scene::Affine::IDENTITY,
        crate::scene::colour(colour),
        None,
        None,
    )?;
    Ok(())
}
