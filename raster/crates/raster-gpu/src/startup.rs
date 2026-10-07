//! How a device comes to exist: what configures construction, how the adapter is
//! chosen, and what each step of it cost.
//!
//! Section 7 of the brief and CLAUDE.md's "startup is a first-class requirement" are this
//! module's whole subject. The caller's decision that page one goes to the graphics
//! device (their ADR 0179) put bring-up on their time-to-first-page, so the two
//! obligations here are sharper than they were at M1:
//!
//! - **Every reported number names exactly one step.** [`StartupTimings`] has a field
//!   per thing that can regress independently — the driver loader, surface creation,
//!   physical-device enumeration, `request_device` — because a host watching one
//!   number that measures three cannot say which moved. That is not hypothetical: the
//!   single `adapter_enumeration` this replaces measured all three of the first ones.
//! - **What needs no window may be started before there is one.** [`create_instance`]
//!   is the instance raster would have made for itself, exposed so a host can make it
//!   on a thread at `main`'s first line and hand it to
//!   [`Device::for_surface_with_instance`](crate::device::Device::for_surface_with_instance).
//!   `wgpu::Instance` is `Send + Sync`; the caller measured ~20 ms of a 145 ms launch
//!   in the overlap.
//!
//! - **Which driver stack talks to the hardware is the host's to say.**
//!   [`create_instance_with`] takes the backend set; [`create_instance`] is it with
//!   `Backends::all()` and is unchanged. This is an escape hatch from a driver, not a
//!   speed knob — restricting the instance to Vulkan halves `Instance::new` and gives
//!   every millisecond back in `request_adapter`, which the caller measured (their
//!   feedback section 8.3), so the total is the invariant. What it *is* for is the machine in
//!   their section 12, where wgpu reached an Intel Vulkan driver that crashed and nothing
//!   could ask for the DX12 one. ADR 0017 records the shape and the two silences that
//!   go with it: no backend field in [`Options`], and no `WGPU_BACKEND`.

use std::time::{Duration, Instant};

use crate::error::{DeviceError, PipelineProblem};

/// Where the background pipeline warm-up has got to.
///
/// One running state and three ways of being finished, for the reason brief section 5 gives about
/// frames: a caller that waits for the warm set — by polling [`Device::warm_up`] or by
/// blocking in [`Device::wait_until_warm`] — must be able to learn that it is never
/// coming, and only one of the three finished states means it arrived.
/// [`Device::is_warm`] is this question narrowed to that one.
///
/// [`Device::warm_up`]: crate::device::Device::warm_up
/// [`Device::wait_until_warm`]: crate::device::Device::wait_until_warm
/// [`Device::is_warm`]: crate::device::Device::is_warm
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarmUp {
    /// Still compiling. The device renders correctly meanwhile, compiling what a frame
    /// needs on the spot.
    Running,
    /// The warm set exists; the duration is what compiling it cost, and it is what
    /// [`StartupTimings::pipeline_compilation`] reports.
    Warm(Duration),
    /// This adapter refused a shader or a pipeline of the warm set, by name. Nothing
    /// retries it, and a frame needing that pipeline is refused with the same reason.
    Refused(PipelineProblem),
    /// The warm-up thread ended without an answer, because it panicked.
    ///
    /// Reachable, and recorded rather than left as a wait nobody will end: this crate
    /// pops a `wgpu` error scope for validation, but an out-of-memory or an internal
    /// error is neither, and those still reach `wgpu`'s uncaptured-error handler —
    /// which panics. A device in this state has no warm set and no reason to give for
    /// it; the frame that next needs one of those pipelines compiles it itself and
    /// reports whatever happens then.
    Abandoned,
}

/// The default per-frame budget for scene-derived allocations, in bytes.
///
/// A stated number rather than an implicit one, because a GPU buffer sized from
/// document-derived arithmetic is a decompression bomb with a different name
/// (CLAUDE.md principle 3). 256 MiB of instance data is roughly eight million
/// rectangle commands — beyond any real page by orders of magnitude, while still
/// refusing runaway input long before an allocator does.
pub const DEFAULT_MAX_FRAME_BYTES: u64 = 256 * 1024 * 1024;

/// The default budget for resident resources — outlines, images, ramps, meshes — in
/// bytes.
///
/// The same principle as [`DEFAULT_MAX_FRAME_BYTES`], at resource scope: brief section 4.7's
/// 60 000×60 000 image arrives from real files by way of a correct interpreter, and
/// its 14.4 GB of RGBA8 must be a refusal naming this number, never an allocation
/// attempt. 512 MiB holds every page of every corpus document the brief quotes with
/// room to spare, and the caller can raise it deliberately.
pub const DEFAULT_MAX_RESOURCE_BYTES: u64 = 512 * 1024 * 1024;

/// The default glyph-atlas budget, in bytes (an R8 texel is one byte).
///
/// The brief's section 6.3: the atlas is sized from a budget the caller sets, never from a constant
/// of ours alone; this is only the default. It stays an afterthought next to a single 1191×1684
/// target.
///
/// **The justification was an argument about the brief's dense page at 1×** — 8 MiB holds
/// roughly two thousand 64×64 tiles, far beyond 107 outlines at several phases each — and
/// a scale-1 argument says nothing about the magnifications brief section 6 cares about. ADR 0063
/// replaced it with the measurement: over page one of 974 corpus documents at **4×**, the
/// largest single page's [`Counters::atlas_working_set_bytes`] is **4 298 422 bytes**, the
/// p99 is 1.4 MiB and the median is 11 KiB. So the default is right by a factor of two at
/// the worst page in the corpus, and the marks that page shapes still lose to the packer
/// are lost to an atlas holding **other pages'** tiles, which no budget reaches.
///
/// Raising it is a real lever on how *often* a shared atlas is exhausted and not on
/// whether it is, and it stops working above `2048 × Limits::max_target_size` — see
/// [`Options::atlas_budget`] and [`Limits::atlas_bytes`].
///
/// [`Counters::atlas_working_set_bytes`]: crate::frame::Counters::atlas_working_set_bytes
/// [`Limits::atlas_bytes`]: crate::device::Limits::atlas_bytes
pub const DEFAULT_ATLAS_BUDGET: u64 = 8 * 1024 * 1024;

/// The default sub-pixel quantum of the glyph cache: 1/16 of a pixel.
///
/// brief section 4.5's fifth decision, measured by the caller (its ADR 0131): 1/16 reused 5.0× on
/// a dense page and left its oracle's verdicts unmoved; 1/8 contradicted pages.
pub const DEFAULT_GLYPH_QUANTUM: u16 = 16;

/// Which producer makes coverage bytes for a **solid** fill or stroke.
///
/// The two lanes hand the same artefact — an R8 tile in the frame's scratch sheet — to
/// the same quad lane, so nothing downstream of coverage can tell them apart. What
/// differs is where the cost falls, and the curves are opposite (ADR 0016).
///
/// **What this setting does not reach**, stated here because a caller cannot see it from
/// the outside: a fill or stroke whose paint is *not* a solid colour — a shading, an
/// image, a mesh, or §7.10.5's `Paint::Function` — rasterises its coverage on the CPU
/// whichever value this holds. The lane is chosen in the solid arm alone, and a rare
/// paint reaches the scratch sheet by a path that never asks. So a page of large
/// shading-filled paths pays the CPU rasteriser under `Gpu`, and a zoom of one does not
/// get the magnification independence the `Gpu` variant describes below.
/// `tests/function_coverage.rs` and `tests/rare_lane_coverage.rs` hold that equality in
/// the pixels rather than leaving it to be inferred from the code.
///
/// **That is a decision now, and it is ADR 0064's.** It was an open question until
/// 2026-08-17, when it was priced over the caller's 974-document corpus: the marks it
/// would move are **0.11 % of a frame's rasterised coverage at a page's own scale and
/// 0.63 % at 4×** — a rare paint under a non-rectangular clip takes the processor lane
/// under either setting anyway, and two thirds of what is left is glyph-sized, which is
/// the shape class this lane is the accurate one for. `raster/doc/notes-rare-lane.md` has the
/// numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Coverage {
    /// The CPU scanline rasteriser of ADR 0008, with the glyph atlas in front of it.
    ///
    /// Coverage is the **exact** area of the pixel a shape covers, 256 levels, and a
    /// page of text at reading size costs almost nothing because the atlas answers
    /// most of it. Its weakness is magnification: past `MAX_GLYPH_DIM` a glyph never
    /// enters the atlas and is rasterised again on every frame.
    #[default]
    Cpu,
    /// Outline triangles rasterised by the device, winding accumulated in a float
    /// target (ADR 0016 — Wallace's method).
    ///
    /// **Nothing in it depends on the magnification**: cubics become quadratics once —
    /// on the first frame that takes this lane, never at upload (ADR 0075) — so a frame
    /// at 100× re-uses what the frame that converted built, and a zoom gesture, where
    /// every cached tile is cold on every frame, costs the same as standing still.
    /// Setting this is therefore what makes a host pay for that conversion at all, and
    /// the frame it is first set on is the one that pays. Its weakness is the other end: coverage is sampled rather than
    /// exact ([`Options::coverage_samples`] levels, not 256), and there is no atlas in
    /// front of it, so a dense page of small text pays per glyph per frame.
    ///
    /// Commands under a non-rectangular clip still take the CPU lane, because the
    /// residue meet is CPU-side; both kinds of tile share one sheet.
    ///
    /// # What sampled coverage costs, and the clause it does not meet (ADR 0076)
    ///
    /// "Sampled rather than exact" is a stronger statement than a level count, and it is
    /// stated here because this variant is where it is chosen. The samples lie on one
    /// lattice of period `p = 1/√`[`coverage_samples`] across the whole device — **0.25 of
    /// a device pixel at the default sixteen** — so for an axis-aligned mark:
    ///
    /// - its **coverage** is a count of lattice rows times `p`, and so within one `p` of
    ///   the area ISO 32000-2 §8.5.3.3 gives it, in either direction. A rule 0.878 pixels
    ///   thick draws 0.75 or 1.0 depending only on where it lands.
    /// - its **placement** — the centroid of its coverage — moves by up to `p`, and by up
    ///   to `p/2` where its width is a multiple of `p`, which a one-pixel hairline is.
    /// - **a pixel the mark intersects may receive nothing at all.** ISO 32000-2 §10.7.4
    ///   requires "painting any pixel whose half-open square region intersects the shape,
    ///   no matter how small the intersection is", and its NOTE 1 puts a pixel the mark's
    ///   *boundary* crosses inside that requirement. This lane leaves such a pixel at zero
    ///   whenever no sample row falls in it. **That is a non-conformance and it is
    ///   recorded as one**; the mark itself cannot disappear, which is the part ADR 0070
    ///   guarantees by keeping anything narrower than `p` on the [`Cpu`](Coverage::Cpu)
    ///   lane.
    ///
    /// Raising [`coverage_samples`] narrows `p` and costs one pass pair per four samples;
    /// it does not remove the last point, because a finer lattice still has gaps. Only
    /// [`Cpu`](Coverage::Cpu) computes exact area (ADR 0005), and a caller that needs it —
    /// a page whose subject is hairlines — should ask for it.
    ///
    /// [`coverage_samples`]: Options::coverage_samples
    Gpu,
    /// The CPU rasteriser's exact arithmetic, run by the device (ADR 0080).
    ///
    /// Every solid fill's coverage is computed by a compute dispatch — one invocation
    /// per tile row, exact signed trapezoid areas, 256 levels — from polylines the
    /// encode flattened, with **no atlas in front of it**: a page of repeated glyphs
    /// pays per glyph per frame, which is the price of a lane whose cost does not grow
    /// with what the atlas holds. Its bytes are the [`Cpu`](Coverage::Cpu) lane's
    /// bytes — the port's determinism is measured rather than assumed
    /// (`tests/compute_coverage_determinism.rs`, ADR 0079) — so unlike
    /// [`Gpu`](Coverage::Gpu) it meets §10.7.4's no-disappearance requirement exactly
    /// as the CPU lane does, and needs no thin-mark guard.
    ///
    /// Strokes, rare paints and anything under a non-rectangular clip still take the
    /// CPU lane; both kinds of tile share one sheet.
    Compute,
}

/// The GPU lane's default sample count: sixteen, on a 4×4 grid.
///
/// Seventeen coverage levels. Wallace's article reached eight samples by packing them
/// into the bits of one byte; a float target has no such ceiling, and sixteen is where
/// a half-covered pixel lands on exactly 128 while the pass still runs four times.
pub const DEFAULT_COVERAGE_SAMPLES: u32 = 16;

/// Construction options.
///
/// A plain value: budgets, an adapter filter, the glyph quantum. No `wgpu` handle
/// lives here on purpose — a hoisted [`wgpu::Instance`] is an argument to the
/// constructor that uses it ([`Device::headless_with_instance`] and
/// [`Device::for_surface_with_instance`]), so that a host cloning an `Options`
/// around is never accidentally sharing a device-adjacent resource.
///
/// [`Device::headless_with_instance`]: crate::device::Device::headless_with_instance
/// [`Device::for_surface_with_instance`]: crate::device::Device::for_surface_with_instance
#[derive(Debug, Clone)]
pub struct Options {
    /// Select the adapter by case-insensitive substring of its name — `"llvmpipe"`
    /// picks the software rasteriser, `"radv"` the Radeon Vulkan driver. `None` asks
    /// wgpu for the highest-performance adapter it can find. Ties among matches are
    /// broken by name order, so the same request on the same machine picks the same
    /// adapter (brief section 4.6's spirit applied to setup).
    pub adapter: Option<String>,
    /// The per-frame budget for scene-derived allocations, in bytes. Exceeding it is
    /// a [`RenderError::FrameBudgetExceeded`] naming both numbers, before anything is
    /// allocated — including, for a [`Target::Surface`] frame, before the swapchain
    /// texture is acquired, so a refusal costs the surface nothing.
    ///
    /// [`RenderError::FrameBudgetExceeded`]: crate::error::RenderError::FrameBudgetExceeded
    /// [`Target::Surface`]: crate::target::Target::Surface
    pub max_frame_bytes: u64,
    /// The budget for resident resources (outlines, images, ramps, meshes), in bytes.
    /// Exceeding it is a [`DeviceError::ResourceBudgetExceeded`] naming all three
    /// numbers, before anything is stored.
    pub max_resource_bytes: u64,
    /// The glyph atlas budget, in bytes ([`DEFAULT_ATLAS_BUDGET`]) — a **request**.
    ///
    /// The atlas is one near-square R8 texture whose width is capped at 2048 and whose
    /// sides are clamped to the adapter's texture limit, so anything above
    /// `2048 × Limits::max_target_size` is not granted and nothing fails: read
    /// [`Limits::atlas_bytes`] for what this became (ADR 0063).
    ///
    /// [`Limits::atlas_bytes`]: crate::device::Limits::atlas_bytes
    pub atlas_budget: u64,
    /// The sub-pixel quantum of the glyph cache, as a denominator: `Some(16)` keys
    /// glyph tiles at 1/16-pixel phases; `None` switches quantisation **off** (exact
    /// phase keying — correct everywhere, and it almost never hits, which is the
    /// caller's own measurement). Quantising moves rendered text by at most half a
    /// quantum, which is why this is exposed rather than chosen silently (brief section 4.5).
    pub glyph_quantum: Option<u16>,
    /// Which lane produces coverage ([`Coverage`]); [`Coverage::Cpu`] by default,
    /// which is the lane whose bytes are exact and whose output the caller's CPU
    /// oracle agrees with.
    pub coverage: Coverage,
    /// Under [`Coverage::Cpu`], flatten the tiles the atlas will not hold on the
    /// device instead of the processor (ADR 0090) — the per-frame hybrid: glyphs keep
    /// the atlas, everything else takes the compute lane's flattening, and the two
    /// lanes are held to zero pixels against each other so the reroute is invisible
    /// except in time.
    ///
    /// `None` decides by adapter: on for a real device, off where the "device" is a
    /// software rasteriser (a compute dispatch on llvmpipe loses to the scanline it
    /// replaces — the caller measured 600 ms against 229). `Some` overrides either
    /// way, which is what lets the pixel gates run the hybrid on the CI's own
    /// software adapter.
    pub compute_assist: Option<bool>,
    /// Subdivide [`Timings::encode`] into geometry, staging and recording, reported
    /// through [`Timings::phases`] (the caller's feedback section 13; ADR 0023).
    ///
    /// **Off by default, because the measurement is not free.** Encode's parts
    /// interleave per command, so the subdivision reads the clock at each seam: about
    /// 0.2 ms over a page of 5 933 commands, which is three times the whole encode of a
    /// page of rectangles and about 1% of a page of paths. A host that traces frames
    /// turns it on for the trace; a host that does not pays an `Option` check.
    ///
    /// [`Timings::encode`]: crate::frame::Timings::encode
    /// [`Timings::phases`]: crate::frame::Timings::phases
    pub instrument_encode: bool,
    /// How many threads one frame's coverage rasterisation may use — **1 by default,
    /// which is the walk this library has always run**.
    ///
    /// The caller measured a page where 59 % of the frame was the scanline rasteriser
    /// turning three million path segments into fifty-eight thousand coverage tiles on
    /// one thread (`pdf-viewer/doc/QUORRA_ENCODE_THREADS.md`). Coverage is a pure
    /// function of one mark's own geometry, so it divides exactly; everything the
    /// frame's order depends on — the budget, the sheet's shelves, the atlas, the
    /// instance stream — stays on the calling thread, which is what makes the result
    /// **byte-identical at any value of this field** (brief section 4.6, and
    /// `tests/encode_threads.rs` holds it to that).
    ///
    /// **Why the host names the number, rather than this library asking the machine.**
    /// ADR 0023 recorded three reasons a renderer must not size a pool for itself, all
    /// of them the caller's: their own `rayon` is already sized to the machine and a
    /// second pool oversubscribes a page turn; their confined worker cannot spawn at all,
    /// because the `/sys` read `glibc` sizes its arenas from is killed by their seccomp
    /// filter; and a pool built at construction lands on their time-to-first-page. So
    /// this is a permission, not a preference — a host that says nothing gets no threads,
    /// and a host that says `n` gets at most `n`, held to
    /// [`std::thread::available_parallelism`].
    ///
    /// **Nothing is built at construction and nothing outlives a frame.** The threads are
    /// a [`std::thread::scope`] entered inside `Device::render` and left before it
    /// returns, and a frame whose geometry is below a measured floor does not enter one
    /// at all — so a small page pays nothing for a large page's lane
    /// (`raster/doc/notes-encode-threads.md` carries the floor and its measurement).
    pub encode_threads: usize,
    /// How many samples the GPU lane takes per pixel, rounded down to a square and
    /// clamped to 4..=64 ([`DEFAULT_COVERAGE_SAMPLES`]).
    ///
    /// Coverage has `samples + 1` levels rather than 256, so this is the quality knob
    /// and it costs **time, not memory**: four samples fit one `rgba16float` texel, and
    /// a frame runs the pass once per group of four. Ignored by [`Coverage::Cpu`],
    /// whose coverage is analytic.
    pub coverage_samples: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            adapter: None,
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
            max_resource_bytes: DEFAULT_MAX_RESOURCE_BYTES,
            atlas_budget: DEFAULT_ATLAS_BUDGET,
            glyph_quantum: Some(DEFAULT_GLYPH_QUANTUM),
            coverage: Coverage::Cpu,
            compute_assist: None,
            coverage_samples: DEFAULT_COVERAGE_SAMPLES,
            instrument_encode: false,
            encode_threads: 1,
        }
    }
}

/// What startup cost, one field per step that can regress on its own (brief section 7), so a
/// regression can be attributed rather than argued about. Gated in CI from the commit
/// that first produced it.
///
/// The four blocking steps happen in field order and are summed by
/// [`StartupTimings::blocking_total`]; `pipeline_compilation` is not among them
/// because no constructor waits for it.
#[derive(Debug, Clone, Copy)]
pub struct StartupTimings {
    /// `wgpu::Instance::new`: the driver loader, and on this machine the larger half
    /// of what a single "adapter enumeration" number used to hide.
    ///
    /// `None` when the caller supplied the instance
    /// ([`Device::headless_with_instance`], [`Device::for_surface_with_instance`]) —
    /// the step happened, but not here, and reporting zero for work someone else
    /// timed would be a number that lies about what it measured.
    ///
    /// [`Device::headless_with_instance`]: crate::device::Device::headless_with_instance
    /// [`Device::for_surface_with_instance`]: crate::device::Device::for_surface_with_instance
    pub instance_creation: Option<Duration>,
    /// Turning the window handle into a `wgpu::Surface`. Genuinely zero for a
    /// headless device, where the step does not happen at all — which is why this is
    /// a `Duration` and `instance_creation` is an `Option`.
    pub surface_creation: Duration,
    /// Physical-device enumeration and the choice among the results: `request_adapter`,
    /// or `enumerate_adapters` plus the [`Options::adapter`] filter. A different cause
    /// from instance creation — the loader versus the devices — and it moves for
    /// different reasons.
    pub adapter_selection: Duration,
    /// `request_device`: getting a queue on the chosen adapter.
    pub device_creation: Duration,
    /// Compiling the warm pipeline set, on the background thread. `None` while that is
    /// still in flight — poll [`Device::warm_up`], or read this again later — and
    /// `None` for good if that warm-up was refused, which is why the state to poll is
    /// [`WarmUp`] rather than a boolean. Not part of
    /// [`StartupTimings::blocking_total`]: nothing blocks on it.
    ///
    /// [`Device::warm_up`]: crate::device::Device::warm_up
    pub pipeline_compilation: Option<Duration>,
}

impl StartupTimings {
    /// What the constructor actually blocked for: instance creation when it was ours,
    /// plus surface creation, adapter selection and device creation.
    ///
    /// Pipeline compilation is excluded because no constructor waits for it — a
    /// caller that chooses to wait ([`Device::wait_until_warm`]) is adding a cost, not
    /// discovering one.
    ///
    /// [`Device::wait_until_warm`]: crate::device::Device::wait_until_warm
    #[must_use]
    pub fn blocking_total(&self) -> Duration {
        self.instance_creation
            .unwrap_or(Duration::ZERO)
            .saturating_add(self.surface_creation)
            .saturating_add(self.adapter_selection)
            .saturating_add(self.device_creation)
    }
}

/// The two blocking steps a constructor performs before adapter selection, carried
/// into the shared build path so each one keeps its own number.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PreSteps {
    /// `None` when the instance was the caller's.
    pub instance_creation: Option<Duration>,
    pub surface_creation: Duration,
}

impl PreSteps {
    /// A device built on an instance the caller made, with no surface yet.
    pub(crate) const BORROWED_INSTANCE: Self = Self {
        instance_creation: None,
        surface_creation: Duration::ZERO,
    };
}

/// The `wgpu::Instance` raster creates for itself, exposed so a host can create it
/// early.
///
/// Instance creation needs no window, no surface and no event loop, so it can run on
/// a thread started at `main`'s first line while the document is read and the window
/// opened; `wgpu::Instance` is `Send + Sync`, so that thread can hand it back. The
/// caller measured the overlap at roughly 20 ms of a 145 ms launch (their feedback
/// section 8.2). Hand the result to
/// [`Device::for_surface_with_instance`](crate::device::Device::for_surface_with_instance).
///
/// Use this rather than constructing a `wgpu::Instance` by hand: the descriptor is
/// the one raster's own constructors use, and surface creation is only guaranteed
/// against an instance made the same way.
///
/// **One instance per process when measuring.** A second instance in the same process
/// finds the driver loader warm and reports a fraction of the true cost — the caller
/// spent a first version of its bring-up harness on exactly that mistake.
///
/// Every backend `wgpu` was built with is loaded. To name a subset — because a machine
/// has a driver that must be avoided — use [`create_instance_with`].
#[must_use]
pub fn create_instance() -> wgpu::Instance {
    create_instance_with(wgpu::Backends::all())
}

/// [`create_instance`], restricted to the backends the host names: `Backends::DX12` on
/// a Windows machine whose Vulkan driver crashes, `Backends::VULKAN` on a machine
/// whose GL driver does.
///
/// **Why this is a parameter rather than a preference.** One GPU is enumerated once per
/// backend that can drive it, under the *device's* name both times, so
/// [`Options::adapter`] cannot express "this GPU, through DX12" — it selects hardware,
/// and the question here is which driver stack talks to it. With no restriction the
/// choice falls to wgpu's hub order, where Vulkan precedes DX12; that is how the
/// caller's project owner reached a crashing Intel Vulkan driver on Windows with no way
/// to ask for the other one (their feedback section 12). The backend set belongs to the
/// instance, which is made before an [`Options`] exists, so it is an argument here and
/// nowhere else (ADR 0017).
///
/// **A host with no driver to avoid and a device to bring up should call
/// [`create_launch_instance`]**, which makes this choice for it the one way that costs
/// nothing where the choice is wrong.
///
/// **The environment is not consulted**, here or in [`create_instance`] — this argument
/// is the only route, deliberately (ADR 0017). A host that wants `WGPU_BACKEND` honoured
/// can say so in one line, and keep its own command line above it:
///
/// ```no_run
/// # use raster_gpu::{create_instance, create_instance_with, wgpu};
/// let from_flag: Option<wgpu::Backends> = None; // whatever `--backend` parsed to
/// let instance = match from_flag.or_else(wgpu::Backends::from_env) {
///     Some(backends) => create_instance_with(backends),
///     None => create_instance(),
/// };
/// ```
///
/// # Naming a set this machine cannot supply
///
/// Is not an error here — an instance with no usable backend is constructible, and
/// nothing has been asked of it yet. It becomes [`DeviceError::NoAdapter`] at the
/// constructor that uses it ([`Device::headless_with_instance`],
/// [`Device::for_surface_with_instance`]), with an **empty** `available` list; on a
/// machine that visibly has a GPU, that emptiness is the signature of this mistake
/// rather than of a broken driver.
///
/// [`Device::headless_with_instance`]: crate::device::Device::headless_with_instance
/// [`Device::for_surface_with_instance`]: crate::device::Device::for_surface_with_instance
#[must_use]
pub fn create_instance_with(backends: wgpu::Backends) -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        flags: instance_flags(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    })
}

/// The flags every instance raster makes carries: the build's own, less the validation of
/// indirect calls, which raster does not make.
///
/// **`VALIDATION_INDIRECT_CALL` is a pipeline compiled before the device is usable.** With
/// it set, `wgpu` builds two compute pipelines inside `request_device` — one to check an
/// indirect dispatch's arguments and one an indirect draw's — so that a device's construction
/// waits for a shader compilation, which `CLAUDE.md` principle 2 forbids the launch path
/// ("[t]he graphics library must return a usable device before it is warm"). On the Radeon
/// 890M through RADV it was about 0.8 ms of a 2.2 ms `request_device` and 3.7 M of its
/// instructions (ADR 1569).
///
/// **What it guards is a call raster does not record.** The pipelines check the arguments an
/// indirect draw or dispatch reads from a buffer; every draw and dispatch here is direct, with
/// its counts computed on the processor and bounded before they are recorded, and the test
/// below holds the crate to that. A change that records an indirect call takes this flag back
/// with it. Debug builds keep the build's other validation, which is the build's choice and
/// not a bring-up figure.
fn instance_flags() -> wgpu::InstanceFlags {
    wgpu::InstanceFlags::from_build_config()
        .difference(wgpu::InstanceFlags::VALIDATION_INDIRECT_CALL)
}

/// The instance a launch brings its device up on: the platform's primary backends, and
/// every backend only where those offer no adapter on real hardware.
///
/// **The secondary backend is GL, and loading it is what most of [`create_instance`] costs
/// on a Vulkan machine.** `wgpu` initialises every backend an instance holds when the
/// instance is made, and GL's initialisation is EGL's — on this tree's measuring machine
/// Mesa's `libEGL_mesa` bringing up a whole second driver, `radeonsi` through
/// `libgallium`, for an adapter `request_adapter` then passes over for the Vulkan one.
/// Measured on the Radeon 890M through RADV, one process per sample, six each: an
/// instance of every backend 22.98 to 25.89 ms and of Vulkan alone 11.90 to 15.54, with
/// `request_adapter` 3.28 to 8.23 ms against 2.88 to 5.83 — the Vulkan-only device usable
/// in 18.49 ms at best against 28.83 (ADR 1532). The adapter enumeration that once gave
/// that saving back in `request_adapter` costs the same with either set on this driver stack.
///
/// **What it gives up is nothing a machine with a working primary backend uses**: wgpu's
/// own preference ranks a primary adapter first wherever there is one. Where the primary
/// backends offer only a processor-emulated adapter, or none — a machine whose GPU has
/// only a GL driver — the primary instance is dropped and every backend is loaded, so such
/// a machine draws on the adapter it would have had and pays one extra enumeration for it.
#[must_use]
pub fn create_launch_instance() -> wgpu::Instance {
    create_launch_instance_timed().0
}

/// [`create_launch_instance`], and what each of its steps cost.
///
/// The two steps move for different reasons, which is why they are two numbers: making the
/// instance is the Vulkan loader and every installed driver initialising, and is steady from
/// launch to launch; the adapter check is the first physical-device enumeration, where the
/// kernel driver is first asked about the GPU, and is the part of bring-up that varies (ADR
/// 1569). [`StartupTimings::instance_creation`] cannot carry either, because the constructor
/// that takes this instance did not make it.
///
/// Before either, the machine's awake render nodes are opened ([`wake_render_nodes`]); they are
/// closed after the check. The power-up the kernel driver starts on an open then runs beside the
/// loader rather than in front of the adapter check (ADR 1658).
#[must_use]
pub fn create_launch_instance_timed() -> (wgpu::Instance, LaunchSteps) {
    let waking = Instant::now();
    let woken = wake_render_nodes();
    let mut render_node_wake = waking.elapsed();
    let render_nodes_opened = woken.len();
    let started = Instant::now();
    let primary = create_instance_with(wgpu::Backends::PRIMARY);
    let made = started.elapsed();
    let checking = Instant::now();
    let on_hardware = pollster::block_on(primary.enumerate_adapters(wgpu::Backends::PRIMARY))
        .iter()
        .any(|adapter| adapter.get_info().device_type != wgpu::DeviceType::Cpu);
    let adapter_check = checking.elapsed();
    // Closed only now: a close made while the power-up runs waits for it (ADR 1658).
    let closing = Instant::now();
    drop(woken);
    render_node_wake = render_node_wake.saturating_add(closing.elapsed());
    if on_hardware {
        let steps = LaunchSteps {
            render_node_wake,
            render_nodes_opened,
            instance_creation: made,
            adapter_check,
            every_backend: false,
        };
        return (primary, steps);
    }
    drop(primary);
    let again = Instant::now();
    let instance = create_instance();
    let steps = LaunchSteps {
        render_node_wake,
        render_nodes_opened,
        instance_creation: made.saturating_add(again.elapsed()),
        adapter_check,
        every_backend: true,
    };
    (instance, steps)
}

/// What [`create_launch_instance_timed`] spent, one field per step that can regress on its
/// own — this module's rule for [`StartupTimings`], applied to the instance a launch makes
/// before any constructor sees it.
#[derive(Debug, Clone, Copy)]
pub struct LaunchSteps {
    /// Opening the awake render nodes before the instance and closing them after the adapter check
    /// ([`wake_render_nodes`]); zero where the platform has none.
    pub render_node_wake: Duration,
    /// How many render nodes that step opened. Fewer than the machine has is a node that was
    /// asleep, or one this process may not open, and the adapter check meets it either way.
    pub render_nodes_opened: usize,
    /// `wgpu::Instance::new`: the driver loader and the drivers' own initialisation — both
    /// instances' together where the fallback made a second.
    pub instance_creation: Duration,
    /// Enumerating the primary backends' adapters to learn whether one is on hardware.
    pub adapter_check: Duration,
    /// Whether the primary backends offered no adapter on hardware, so that every backend was
    /// loaded after them.
    pub every_backend: bool,
}

/// Opens every DRM render node whose device is awake, and hands the files back to be closed after
/// the adapter check.
///
/// **What it buys.** On an AMD APU (the Radeon 890M through `amdgpu`, ADR 1658), opening a render
/// node after the GPU has had about 50 ms without work starts a power-up in the kernel driver that
/// takes about 5 ms, and a firmware query arriving during it — the first `AMDGPU_INFO` the Vulkan
/// driver asks inside the adapter check — waits for the remainder. The query cannot be moved, but
/// the open can: made here, before `wgpu::Instance::new`'s 10 to 13 ms of loader work, the power-up
/// is over by the time the driver opens the node again. Measured on that machine, the gate's
/// bring-up child with and without this step, interleaved, 22 an arm born after 1.5 s of idle:
/// bring-up 22.6 against 26.3 ms (median), the adapter check 6.2 against 9.8; 52 an arm born back
/// to back read 20.7 against 20.0, the adapter check 6.0 in both. The step itself reads 0.08 to
/// 0.19 ms; where the platform has no render node it is nothing. The files are closed after the
/// adapter check, because closing one while its power-up runs waits for it: closed at once, the
/// step read 1.7 to 2.4 ms in the gate's first-page children.
///
/// **Why only awake ones.** A device the kernel has suspended — a laptop's discrete GPU — is
/// woken by an open, and keeping one awake is a cost a person's battery pays. Such a node is left
/// for the adapter check, which decides whether to wake it. `active` is awake and `unsupported`
/// is a device whose power is never managed at run time; anything else, or no status at all, is
/// left alone.
///
/// **Why an open that fails is not an error.** Nothing here is needed: the adapter check opens
/// the same nodes itself and reports what it cannot, and a node this process may not open is
/// that check's to report. The count is in [`LaunchSteps`], so a wake that did nothing is seen.
#[cfg(target_os = "linux")]
fn wake_render_nodes() -> Vec<std::fs::File> {
    render_nodes_awake(
        std::path::Path::new("/dev/dri"),
        std::path::Path::new("/sys/class/drm"),
    )
    .iter()
    .filter_map(|node| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(node)
            .ok()
    })
    .collect()
}

/// [`wake_render_nodes`] where there are no DRM render nodes to open.
#[cfg(not(target_os = "linux"))]
fn wake_render_nodes() -> Vec<std::fs::File> {
    Vec::new()
}

/// The render nodes under `dri` whose device `class` (the kernel's `/sys/class/drm`) reports
/// awake, in name order: [`wake_render_nodes`]'s choice, apart from the opening.
#[cfg(any(target_os = "linux", test))]
fn render_nodes_awake(dri: &std::path::Path, class: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(dri) else {
        return Vec::new();
    };
    let mut nodes: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|name| name.to_string_lossy().starts_with("renderD"))
        .filter(|name| {
            let status = class.join(name).join("device/power/runtime_status");
            std::fs::read_to_string(status)
                .is_ok_and(|status| matches!(status.trim(), "active" | "unsupported"))
        })
        .map(|name| dri.join(name))
        .collect();
    nodes.sort();
    nodes
}

/// Choose the adapter: the [`Options::adapter`] filter when there is one, wgpu's own
/// preference when there is not.
///
/// The `Backends::all()` passed to `enumerate_adapters` is not a second backend
/// decision: `wgpu::Instance` only holds the backends it was built with, so the mask
/// means "everything this instance has" and [`create_instance_with`]'s restriction is
/// already inside it.
pub(crate) fn select_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'static>>,
    options: &Options,
) -> Result<wgpu::Adapter, DeviceError> {
    match &options.adapter {
        Some(pattern) => {
            let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
            let available: Vec<String> = adapters.iter().map(|a| a.get_info().name).collect();
            let needle = pattern.to_lowercase();
            let mut matches: Vec<wgpu::Adapter> = adapters
                .into_iter()
                .filter(|a| a.get_info().name.to_lowercase().contains(&needle))
                .collect();
            matches.sort_by_key(|a| a.get_info().name);
            matches
                .into_iter()
                .next()
                .ok_or_else(|| DeviceError::NoAdapter {
                    requested: Some(pattern.clone()),
                    available,
                })
        }
        None => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface,
            ..Default::default()
        }))
        .map_err(|_| {
            let available = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
                .iter()
                .map(|a| a.get_info().name)
                .collect();
            DeviceError::NoAdapter {
                requested: None,
                available,
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// [`super::render_nodes_awake`] opens an awake node and a node whose power is never managed,
    /// and leaves a suspended one, one with no status and anything that is not a render node.
    #[test]
    fn only_a_render_node_whose_device_is_awake_is_woken() {
        let root =
            std::env::temp_dir().join(format!("raster-gpu-render-nodes-{}", std::process::id()));
        let (dri, class) = (root.join("dri"), root.join("class"));
        std::fs::create_dir_all(&dri).unwrap();
        for (node, status) in [
            ("renderD128", Some("active\n")),
            ("renderD129", Some("suspended\n")),
            ("renderD130", Some("unsupported\n")),
            ("renderD131", None),
            ("card1", Some("active\n")),
        ] {
            std::fs::write(dri.join(node), b"").unwrap();
            if let Some(status) = status {
                let power = class.join(node).join("device/power");
                std::fs::create_dir_all(&power).unwrap();
                std::fs::write(power.join("runtime_status"), status).unwrap();
            }
        }
        let woken = super::render_nodes_awake(&dri, &class);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(woken, vec![dri.join("renderD128"), dri.join("renderD130")]);
        assert!(super::render_nodes_awake(&root.join("absent"), &class).is_empty());
    }

    /// Every `.rs` file under `directory`, with its text.
    fn sources(directory: &Path, into: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                into.push((path.display().to_string(), text));
            }
        }
    }

    /// `instance_flags` drops `wgpu`'s validation of indirect calls on the ground that raster
    /// makes none (ADR 1569); this is that ground, read off the crate's own source. The needles
    /// are assembled so that this file does not find itself.
    #[test]
    fn raster_records_no_indirect_call_so_its_validation_is_not_asked_for() {
        let mut files = Vec::new();
        sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        assert!(files.len() > 10, "the crate's sources were found");
        let needles = [
            ["_indirect", "("].concat(),
            ["BufferUsages::", "INDIRECT"].concat(),
        ];
        let found: Vec<&str> = files
            .iter()
            .filter(|(_, text)| needles.iter().any(|needle| text.contains(needle.as_str())))
            .map(|(path, _)| path.as_str())
            .collect();
        assert!(
            found.is_empty(),
            "an indirect call is recorded in {found:?}: `instance_flags` must keep \
             `VALIDATION_INDIRECT_CALL` for it"
        );
    }
}
