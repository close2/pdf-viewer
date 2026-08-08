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
    pub page: Option<(&'a DisplayList, TargetSpec)>,
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
    /// Releasing the frame's transient resources and evicting settled cache entries.
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

/// The window-owning form of the backend: quorra's device holds the surface, and
/// [`QuorraPresenter::present`] draws and presents one frame.
#[derive(Debug)]
pub struct QuorraPresenter {
    device: quorra_gpu::Device,
    background: Color,
    caches: crate::cache::ResourceCaches,
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
        self.caches.begin_frame();
        let mut builder = quorra_scene::SceneBuilder::new();
        let mut transient: Vec<ResourceId> = Vec::new();
        let built = build(
            &mut self.device,
            &mut self.caches,
            self.background,
            &mut builder,
            &frame,
            &mut transient,
        );
        self.last.scene = began.elapsed();
        self.last.uploads = self
            .caches
            .stored()
            .saturating_add(u32::try_from(transient.len()).unwrap_or(u32::MAX));

        let submitted = std::time::Instant::now();
        let rendered = built.and_then(|()| {
            let scene = builder.finish();
            let viewport = quorra_gpu::Viewport::full(
                frame.width,
                frame.height,
                quorra_scene::Affine::IDENTITY,
            );
            Ok(self
                .device
                .render(&scene, &viewport, quorra_gpu::Target::Surface)?)
        });
        self.last.device = submitted.elapsed();
        if let Ok(drawn) = &rendered {
            let timings = drawn.timings();
            self.last.encode = timings.encode;
            self.last.upload = timings.upload;
            self.last.execute = timings.execute;
            self.last.execute_measured = matches!(
                timings.execute_provenance,
                quorra_gpu::TimingProvenance::TimestampQueries
            );
            let counters = drawn.counters();
            self.last.commands = counters.commands;
            self.last.commands_culled = counters.commands_culled;
            self.last.bytes_uploaded = counters.bytes_uploaded;
        }

        let settling = std::time::Instant::now();
        // Per-frame resources go back to the budget on both paths; the frame's
        // own error outranks a release problem (as in `rasterize`).
        let mut release_error: Option<quorra_gpu::DeviceError> = None;
        for id in transient.drain(..) {
            if let Err(error) = self.device.release(id) {
                release_error.get_or_insert(error);
            }
        }
        // Frames drawn and frames refused both settle the caches: a viewer with
        // documents open all afternoon is the long-lived instance
        // QUORRA_FEEDBACK.md section 2 described.
        let evicted = self.caches.evict_settled(&mut self.device);
        self.last.settle = settling.elapsed();
        self.last.total = began.elapsed();
        evicted?;
        rendered?;
        if let Some(error) = release_error {
            return Err(error.into());
        }
        Ok(())
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
