//! GPU rasteriser backend built on Vello and wgpu.
//!
//! Implements [`pdf_render::Rasterizer`] against the GPU, where continuous zoom and
//! pan, large vector artwork, high-DPI output and thumbnail grids are decisively
//! faster than on the CPU.
//!
//! # Headless by construction
//!
//! This crate never creates a window or a surface. It renders into an offscreen
//! texture and reads the pixels back, so the entire GPU path is testable with no
//! display server — under `lavapipe` in CI, and on a real driver locally. Presenting
//! to a window is `viewer-ui`'s job, and it reuses the same [`GpuContext`].
//!
//! That split is deliberate: the part that can be verified automatically is kept
//! separate from the part that needs a human looking at a screen.
//!
//! # Safety posture
//!
//! Unlike the parsing and model crates, this crate is *permitted* `unsafe`, because
//! creating a surface from a raw window handle eventually requires it. That is
//! acceptable only because no untrusted document bytes reach this code — it consumes
//! a [`pdf_render::DisplayList`], which is data we produced ourselves. Every `unsafe`
//! block must carry a comment establishing that invariant, enforced by the
//! `undocumented_unsafe_blocks` lint.
//!
//! At present the crate contains no `unsafe` at all, since the offscreen path needs
//! none, so `forbid` is set below and will be relaxed only when presentation lands.
//!
//! GPU drivers are large bodies of unsafe C and are themselves an attack surface, so
//! this crate is a candidate for its own sandboxed process. See `crates/pdf-sandbox`.

#![forbid(unsafe_code)]

mod scene;
mod shading;
mod soft_mask;

use pdf_render::{
    ClipId, DisplayList, Medium, Raster, RasterFormat, Rasterizer, SoftMaskId, TargetSpec,
};
use vello::wgpu;

pub use soft_mask::SoftMaskRasters;

/// Translates a display list into a Vello scene.
///
/// `to_device` maps page space to device space.
///
/// Exposed because window presentation needs the *same* translation as offscreen
/// rasterisation, but renders into a surface texture this crate did not allocate. A
/// caller that reimplemented the translation would silently drift — clips, blend modes
/// and stroke handling would diverge from what the tests cover, so the window would
/// show something the test suite never checks.
///
/// This does leak `vello::Scene` into the public API. That is acceptable for a crate
/// whose entire purpose is the Vello backend, and the display list remains the
/// portable contract.
///
/// # Errors
///
/// As [`GpuRasterizer::rasterize`]: unsupported commands or paints, and dangling or
/// cyclic clip chains.
pub fn build_scene(
    list: &DisplayList,
    target: TargetSpec,
    masks: &SoftMaskRasters,
) -> Result<vello::Scene, GpuRasterError> {
    scene::build(list, target, masks)
}

/// Evaluates every soft mask a display list carries, at one target (§11.5).
///
/// A mask is a transparency group rendered at device resolution, so it cannot be part of the
/// scene that uses it: each one is rendered to its own texture, read back, and converted into
/// mask values by `pdf_render::SoftMask`, which is the function the CPU backend calls on its
/// own pixels. The result is handed to [`build_scene`], which draws it as the alpha of a
/// `DestIn` layer around each object the mask applies to.
///
/// Exposed for the same reason [`build_scene`] is: window presentation needs the same masks
/// as offscreen rasterisation, and a second implementation would drift. A list with no soft
/// mask costs nothing here — the loop does not run — so a caller may always call it.
///
/// # Errors
///
/// As [`build_scene`], plus [`GpuRasterError::Render`] and [`GpuRasterError::Readback`] for
/// a mask that could not be rendered or read back.
pub fn evaluate_soft_masks(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut vello::Renderer,
    list: &DisplayList,
    target: TargetSpec,
) -> Result<SoftMaskRasters, GpuRasterError> {
    let mut bands = Bands::default();
    soft_mask::evaluate(list, target, &mut |scene| {
        let texture = render_to_texture(device, queue, renderer, scene, target, &mut bands)?;
        read_pixels(device, queue, &texture, target)
    })
}

/// Renders a scene into a fresh target-sized texture, onto transparency.
///
/// Transparent rather than the medium's colour for the reason `rasterize` gives: §11.4.7's
/// page group is isolated, and a soft mask's group has no medium at all.
///
/// # Errors
///
/// [`GpuRasterError::Render`] if Vello rejects the scene.
fn render_to_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut vello::Renderer,
    scene: &mut vello::Scene,
    target: TargetSpec,
    bands: &mut Bands,
) -> Result<wgpu::Texture, GpuRasterError> {
    // Vello writes its result through a storage binding, and the pixels are then copied out, so
    // both usages are required. `COPY_DST` is for banding, which composes the result from
    // band-sized renders.
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("render target"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    render_checked(
        device,
        queue,
        renderer,
        scene,
        &texture,
        &vello::RenderParams {
            base_color: vello::peniko::Color::TRANSPARENT,
            width: target.width,
            height: target.height,
            antialiasing_method: vello::AaConfig::Area,
        },
        bands,
    )?;
    Ok(texture)
}

/// How many horizontal bands a target had to be split into for the device to draw it.
///
/// Carried by the caller from one frame to the next so that a page which needed splitting is not
/// re-discovered — and one wasted render not repaid — on every scroll.
///
/// **The count is remembered against the size it was learnt at**, and a render at any other size
/// starts again from a single pass. Without that it only ever ratchets upward: one dense page at
/// a large zoom would band every page after it, at every size, for the life of the program, and
/// each unnecessary band costs a scene re-encoded, a render submitted and a copy. Scrolling and
/// turning pages keep the size, so the common case still pays the discovery once.
#[derive(Debug, Clone, Copy, Default)]
pub struct Bands {
    count: u32,
    learnt_at: (u32, u32),
}

impl Bands {
    /// Forgets what the last render needed, so the next starts from a single pass.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// How many bands the last successful render used; zero before the first.
    #[must_use]
    pub fn count(self) -> u32 {
        self.count
    }

    /// What to start from for a target of this size.
    fn start(self, size: (u32, u32)) -> u32 {
        if self.learnt_at == size {
            self.count.max(1)
        } else {
            1
        }
    }
}

/// The shortest band worth trying, in device pixels.
///
/// Splitting further stops helping long before this: what a band costs is fixed — a scene
/// re-encoded, a render submitted, a copy — while what it saves falls with its height. A scene
/// that will not fit in a 32-pixel band will not fit in a 16-pixel one either, and the honest
/// answer at that point is the refusal, which the processor then draws.
const MIN_BAND_HEIGHT: u32 = 32;

/// Renders a scene, **checks that it was rendered**, and splits the target until it is.
///
/// Vello sizes its GPU-side working buffers by a table of constants, and its own comment on them
/// is the whole of this function's reason: `vello_encoding::BufferSizes::new` says they are
/// "hand picked to accommodate the vello test scenes as well as paris-30k" and "should instead
/// get derived from the scene layout using reasonable heuristics". (Quoted inline rather than as
/// a blockquote: in this tree a blockquote is a clause of ISO 32000-2 and the conformance checker
/// verifies it against `doc/md/`. This is a dependency's words, not the standard's.)
///
/// A scene that needs more — many small paths over many tiles, which is a page of text at a high
/// resolution — overflows one of them *on the device*. The shaders set a `failed` flag in the
/// bump allocators and stop filling; `Renderer::render_to_texture` cannot see that flag, returns
/// `Ok`, and the target texture is left **blank**. Page 6 of ISO 32000-2 at 1132×1600 is such a
/// scene: it needs 2 183 025 tile records against the 2 097 152 the buffer holds, four per cent
/// more, and every pixel comes back empty (ADR 0127).
///
/// So this asks, and then it *answers*. `render_to_texture_async` returns the allocators, which
/// is the only route vello 0.9 offers to the flag; a set flag means the target is halved into
/// horizontal bands and each band rendered and copied into place. A band holds fewer paths, so it
/// needs fewer tiles, and the halving repeats until the device draws it or a band would be
/// shorter than [`MIN_BAND_HEIGHT`]. **This is vello's own remedy** — its issue 366 on robust
/// dynamic memory proposes subdividing the viewport and resubmitting — implemented by the caller
/// because vello 0.9 does not implement it and exposes no way to enlarge the buffers instead.
///
/// **Public because it is the only call a host should make.** `Renderer::render_to_texture` is
/// right there and reports success on a blank page; a viewer drawing to its own surface — which
/// is the tier-2 path, and the one a person actually looks at — has to come through here or it
/// inherits exactly the defect this exists to catch. The texture it draws into therefore needs
/// `COPY_DST` as well as `STORAGE_BINDING`, which vello's own surface target does not have.
///
/// # Errors
///
/// [`GpuRasterError::Render`] if vello rejects the scene, [`GpuRasterError::SceneTooLarge`] if a
/// band as short as [`MIN_BAND_HEIGHT`] still overflows, [`GpuRasterError::Readback`] if the
/// device does not answer within the timeout.
///
/// The cost of asking is one device synchronisation per render, because the flag lives in a
/// buffer that has to be mapped. That is real and it is affordable *here*: this program renders a
/// frame when a person turns a page, not sixty times a second, and the alternative is a page that
/// is blank with nothing said.
pub fn render_checked(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut vello::Renderer,
    scene: &mut vello::Scene,
    texture: &wgpu::Texture,
    params: &vello::RenderParams,
    bands: &mut Bands,
) -> Result<(), GpuRasterError> {
    keep_the_line_soup_non_empty(scene);
    let size = (params.width, params.height);
    let mut count = bands.start(size);
    loop {
        match render_in_bands(device, queue, renderer, scene, texture, params, count) {
            Ok(()) => {
                *bands = Bands {
                    count,
                    learnt_at: size,
                };
                return Ok(());
            }
            Err(error @ GpuRasterError::SceneTooLarge { .. }) => {
                let doubled = count.saturating_mul(2);
                if params.height.checked_div(doubled).unwrap_or(0) < MIN_BAND_HEIGHT {
                    bands.reset();
                    return Err(error);
                }
                count = doubled;
            }
            Err(other) => return Err(other),
        }
    }
}

/// Renders the scene as `count` horizontal bands, copying each into the target.
///
/// One band is the ordinary case and costs nothing extra — no second texture, no copy, the same
/// call vello would have received. Beyond one, each band is the same scene translated up to the
/// band's top and rendered at the band's height, so the paths outside it fall away in binning and
/// take their tile records with them.
fn render_in_bands(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut vello::Renderer,
    scene: &vello::Scene,
    texture: &wgpu::Texture,
    params: &vello::RenderParams,
    count: u32,
) -> Result<(), GpuRasterError> {
    if count <= 1 {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        return render_once(device, queue, renderer, scene, &view, params);
    }

    let height = params.height.div_ceil(count);
    let strip = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("band"),
        size: wgpu::Extent3d {
            width: params.width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = strip.create_view(&wgpu::TextureViewDescriptor::default());

    let mut top = 0;
    while top < params.height {
        let mut band = vello::Scene::new();
        band.append(
            scene,
            Some(vello::kurbo::Affine::translate((0.0, -f64::from(top)))),
        );
        keep_the_line_soup_non_empty(&mut band);
        render_once(
            device,
            queue,
            renderer,
            &band,
            &view,
            &vello::RenderParams {
                base_color: params.base_color,
                width: params.width,
                height,
                antialiasing_method: params.antialiasing_method,
            },
        )?;

        // The last band is shorter than the rest where the height does not divide, and copying
        // its full height would run off the end of the target.
        let rows = height.min(params.height.saturating_sub(top));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("band copy"),
        });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &strip,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: top, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: params.width,
                height: rows,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        top = top.saturating_add(height);
    }
    Ok(())
}

/// One render, with the device asked afterwards whether it happened.
///
/// # Errors
///
/// [`GpuRasterError::SceneTooLarge`] where the bump allocators come back with their `failed` flag
/// set, naming the stage that ran out of room.
fn render_once(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut vello::Renderer,
    scene: &vello::Scene,
    view: &wgpu::TextureView,
    params: &vello::RenderParams,
) -> Result<(), GpuRasterError> {
    // Deprecated in favour of the synchronous call, whose return type is stable — and which
    // cannot answer the question. The deprecation note says the *shape* of this is unstable, not
    // that the information is unavailable elsewhere; there is nowhere else in vello 0.9. Revisit
    // when vello stabilises a statistics API, which its own note promises.
    #[expect(
        deprecated,
        reason = "the only route in vello 0.9 to whether the render actually happened; see above"
    )]
    let rendering = renderer.render_to_texture_async(
        device,
        queue,
        scene,
        view,
        params,
        vello::low_level::DebugLayers::none(),
    );
    let allocators =
        drive(device, rendering)?.map_err(|error| GpuRasterError::Render(error.to_string()))?;

    if let Some(bump) = allocators
        && bump.failed != 0
    {
        return Err(GpuRasterError::SceneTooLarge {
            width: params.width,
            height: params.height,
            wanted: stage_that_ran_out(bump.failed),
        });
    }
    Ok(())
}

/// Names the pipeline stage whose allocation failed, from the flag the shaders set.
///
/// The bits are `vello_shaders`' own, in `shader/shared/bump.wgsl`, and there is no Rust constant
/// for them to import. More than one can be set — a stage that runs out often starves the next —
/// so the earliest is named, because it is the one that caused the rest.
fn stage_that_ran_out(failed: u32) -> &'static str {
    match failed {
        _ if failed & 0x1 != 0 => "binning",
        _ if failed & 0x2 != 0 => "tile",
        _ if failed & 0x4 != 0 => "line",
        _ if failed & 0x8 != 0 => "segment-count",
        _ if failed & 0x10 != 0 => "per-tile command list",
        _ => "an unnamed",
    }
}

/// Adds a path that draws nothing, because vello 0.9 panics on a scene that has none.
///
/// The `debug_layers` feature is what makes [`render_checked`] possible at all, and it has a
/// second effect: after the render, vello slices its captured line buffer to `bump.lines` entries
/// and hands that slice to wgpu, which rejects an empty one by panicking — "buffer slices can not
/// be empty". A scene with no lines is not an exotic case. **A blank page has none**, and so does
/// a zero-length stroke or a clip that admits nothing, both of which are cross-backend fixtures
/// here. With `panic = "abort"` in the release profile that is not a failed render, it is a dead
/// viewer, which would be a far worse defect than the one being fixed.
///
/// So every scene gets one rectangle in a fully transparent paint. It contributes four lines to
/// the soup and, being alpha zero over the destination, composites as the identity — which the
/// fourteen cross-backend fixtures check, since they compare every pixel against the processor's.
///
/// Reported upstream is the right long-term answer; until vello guards that slice, this is the
/// cost of being able to tell a drawn page from a blank one, and it is one rectangle.
fn keep_the_line_soup_non_empty(scene: &mut vello::Scene) {
    scene.fill(
        vello::peniko::Fill::NonZero,
        vello::kurbo::Affine::IDENTITY,
        vello::peniko::Color::TRANSPARENT,
        None,
        &vello::kurbo::Rect::new(0.0, 0.0, 1.0, 1.0),
    );
}

/// Runs a future that only makes progress when the device is polled.
///
/// Vello's asynchronous render maps a buffer and waits for the callback, and **that callback
/// fires when somebody polls the device**. `pollster::block_on` parks the thread instead, so the
/// poll never happens and the wait never ends — which is a deadlock rather than a slow render,
/// and it is what the first version of [`render_checked`] did.
///
/// So this is the executor that future needs and no more: poll the future, and where it is not
/// ready, poll the device. The wait is bounded for [`read_pixels`]'s reason — a wedged driver
/// must surface as an error rather than hang the viewer.
///
/// **The device is polled in slices, and a slice expiring is not a failure.** Each `poll` waits
/// up to [`POLL_SLICE`] so that the deadline above can be checked between waits;
/// [`wgpu::PollError::Timeout`] from one of them means "still working", and the only thing that
/// gives up is [`GPU_WAIT_TIMEOUT_SECS`]. Treating the slice's expiry as an error instead made
/// the effective bound one second rather than sixty, which is a bound no software rasteriser
/// meets on a large page: CI failed here on `lavapipe` with the timeout's own words, in a test
/// the machine that wrote it passes, and the sixty-second constant had never once applied.
fn drive<F: Future>(device: &wgpu::Device, future: F) -> Result<F::Output, GpuRasterError> {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(GPU_WAIT_TIMEOUT_SECS));
    loop {
        if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return Ok(value);
        }
        if deadline.is_some_and(|deadline| std::time::Instant::now() > deadline) {
            return Err(GpuRasterError::Readback(format!(
                "the device did not finish within {GPU_WAIT_TIMEOUT_SECS} s"
            )));
        }
        match device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(POLL_SLICE),
        }) {
            Ok(_) | Err(wgpu::PollError::Timeout) => {}
            Err(error) => return Err(GpuRasterError::Readback(error.to_string())),
        }
    }
}

/// How long one `poll` waits before the loop above looks at its deadline again.
///
/// Short enough that a wedged device is noticed promptly and long enough that the loop is not
/// a spin. Nothing about it bounds the render: [`GPU_WAIT_TIMEOUT_SECS`] does.
const POLL_SLICE: std::time::Duration = std::time::Duration::from_secs(1);

/// How long to wait for the GPU before giving up on a readback.
///
/// Generous enough for `lavapipe` rendering a large page in CI, where software
/// rasterisation of a full A4 at high resolution is genuinely slow.
const GPU_WAIT_TIMEOUT_SECS: u64 = 60;

/// A GPU device and queue, plus a Vello renderer bound to them.
///
/// Held separately from [`GpuRasterizer`] so that `viewer-ui` can share one device
/// between offscreen rasterisation and window presentation. Creating a device and
/// compiling pipelines costs tens to hundreds of milliseconds, so it happens once.
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: vello::Renderer,
    adapter_info: wgpu::AdapterInfo,
}

/// Written by hand because `vello::Renderer` does not implement `Debug`. Reports the
/// adapter, which is the part worth seeing in a log or a test failure.
impl std::fmt::Debug for GpuContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuContext")
            .field("adapter", &self.adapter_description())
            .finish()
    }
}

impl GpuContext {
    /// Creates a headless GPU context, choosing any available adapter.
    ///
    /// No surface is requested, so this works with no display server. Prefers a real
    /// device, and a software implementation such as `lavapipe` serves when that is
    /// all there is.
    ///
    /// # Errors
    ///
    /// [`GpuRasterError::NoAdapter`] when no adapter can be found at all,
    /// [`GpuRasterError::NoDevice`] when an adapter exists but yields no device, and
    /// [`GpuRasterError::Renderer`] when Vello's shader compilation fails.
    pub fn new_headless() -> Result<Self, GpuRasterError> {
        Self::new_headless_inner(false)
    }

    /// Creates a headless context on a *software* adapter, such as `lavapipe`.
    ///
    /// Two uses. CI has no GPU, so this is the path that runs there — and being able
    /// to select it explicitly locally means a CI-only rendering difference can be
    /// reproduced on a developer machine, instead of being debugged through pushes.
    ///
    /// # Errors
    ///
    /// As [`Self::new_headless`], and additionally [`GpuRasterError::NoAdapter`] if no
    /// software implementation is installed.
    pub fn new_headless_software() -> Result<Self, GpuRasterError> {
        Self::new_headless_inner(true)
    }

    fn new_headless_inner(force_software: bool) -> Result<Self, GpuRasterError> {
        // `new_without_display_handle` is the headless constructor: it declines to
        // acquire a windowing-system display handle at all, so no display server is
        // needed and none is looked for.
        //
        // Environment overrides such as `WGPU_BACKEND` are deliberately not consulted:
        // backend choice affects rendered output, and a comparison suite whose results
        // depend on ambient environment is not reproducible. Selection is explicit,
        // via this argument.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: if force_software {
                wgpu::PowerPreference::LowPower
            } else {
                wgpu::PowerPreference::HighPerformance
            },
            // The defining choice for headless operation: requiring no compatible
            // surface means adapter selection never touches windowing.
            compatible_surface: None,
            force_fallback_adapter: force_software,
        }))
        .map_err(|e| GpuRasterError::NoAdapter(e.to_string()))?;

        let adapter_info = adapter.get_info();

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("pdf-viewer render-gpu"),
            // Vello needs generous limits for its compute pipeline; taking the
            // adapter's own limits avoids failing on a device that is in fact capable.
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(|e| GpuRasterError::NoDevice(e.to_string()))?;

        let renderer = vello::Renderer::new(
            &device,
            vello::RendererOptions {
                // Compile only the antialiasing mode actually used. Each permutation
                // costs shader-compilation time at startup, which the startup rules in
                // CLAUDE.md make a cost worth minimising.
                antialiasing_support: vello::AaSupport {
                    area: true,
                    msaa8: false,
                    msaa16: false,
                },
                ..Default::default()
            },
        )
        .map_err(|e| GpuRasterError::Renderer(e.to_string()))?;

        Ok(Self {
            device,
            queue,
            renderer,
            adapter_info,
        })
    }

    /// Returns a human-readable description of the adapter in use.
    ///
    /// Recorded in comparison reports, because output can legitimately differ between
    /// drivers and a diff is not interpretable without knowing which one produced it.
    #[must_use]
    pub fn adapter_description(&self) -> String {
        format!(
            "{} ({:?}, {:?})",
            self.adapter_info.name, self.adapter_info.device_type, self.adapter_info.backend
        )
    }

    /// Returns `true` when rendering happens on a software implementation.
    ///
    /// CI runs on `lavapipe`, where a test asserting real-GPU timing would be
    /// meaningless, so tests can relax or skip accordingly.
    #[must_use]
    pub fn is_software(&self) -> bool {
        self.adapter_info.device_type == wgpu::DeviceType::Cpu
    }
}

/// Renders display lists on the GPU.
#[derive(Debug)]
pub struct GpuRasterizer {
    context: GpuContext,
    medium: Medium,
    /// What the last page needed, so a document of dense pages pays the discovery once.
    bands: Bands,
}

impl GpuRasterizer {
    /// Creates a rasteriser with its own headless context.
    ///
    /// # Errors
    ///
    /// As [`GpuContext::new_headless`].
    pub fn new_headless() -> Result<Self, GpuRasterError> {
        Ok(Self::with_context(GpuContext::new_headless()?))
    }

    /// Creates a rasteriser over an existing context.
    #[must_use]
    pub fn with_context(context: GpuContext) -> Self {
        Self {
            context,
            medium: Medium::PAGE_ONLY,
            bands: Bands::default(),
        }
    }

    /// Sets what the page is imposed on: §11.4.7's 𝑊, and what lies outside the page.
    ///
    /// Defaults to [`Medium::PAGE_ONLY`], matching `render_cpu::CpuRasterizer`, so that the
    /// backends are directly comparable without configuring any of them — and so that a target
    /// larger than its page has to *say* it is a window rather than inheriting one backend's
    /// habit, which is `doc/traps/pixels-and-rasterisers.md` trap 2's rule.
    #[must_use]
    pub fn with_medium(mut self, medium: Medium) -> Self {
        self.medium = medium;
        self
    }

    /// How many bands the last render needed.
    ///
    /// A caller has no decision to make with this — the split is automatic — but a *test* does:
    /// it is how `real_pages.rs` asserts that page 6 is drawn *because* it was banded rather than
    /// because something else changed underneath.
    #[must_use]
    pub fn bands(&self) -> Bands {
        self.bands
    }

    /// Returns the underlying context.
    #[must_use]
    pub fn context(&self) -> &GpuContext {
        &self.context
    }

    /// Copies a rendered texture into a [`Raster`], imposing the medium's colour.
    ///
    /// §11.4.7: the page group is isolated, so the medium's colour is composited with the
    /// finished page rather than being the backdrop its blend modes saw. Shared with the CPU
    /// backend so the two cannot differ about it, and done in premultiplied form, which is
    /// where a source-over composite is exact.
    ///
    /// Vello hands back **straight** alpha, hence the conversion around it rather than after
    /// it — see the note above [`demultiply`]. Skipped entirely for a transparent medium,
    /// where the composite is the identity and the round trip would cost a level on every
    /// partly covered pixel for nothing.
    fn read_back(
        &self,
        list: &DisplayList,
        texture: &wgpu::Texture,
        target: TargetSpec,
    ) -> Result<Raster, GpuRasterError> {
        let mut data = read_pixels(&self.context.device, &self.context.queue, texture, target)?;

        // §14.11.2.1's clip, which is the page's own and not the medium's: a caller asking for
        // `Medium::NONE` wants the page's alpha back, not the marks the standard says shall not
        // be shown. `None` for a page-sized target, which is what leaves every gate unmoved.
        let crop = pdf_render::crop_area(list, target);
        if self.medium.marks_anything() || crop.is_some() {
            premultiply(&mut data);
            if let Some(crop) = crop {
                pdf_render::crop_to_page(&mut data, target.width, 0, crop);
            }
            // Where 𝑊 stops is §14.11.2.1's page boundary rather than the target's edge, which
            // for a page-sized target is the same rectangle and for a window is not. The same
            // `pdf_render` functions all three backends end with, in the same order, so none of
            // them can decide either alone.
            if self.medium.marks_anything() {
                pdf_render::impose_within(
                    &mut data,
                    target.width,
                    0,
                    pdf_render::page_area(list, target),
                    self.medium,
                );
            }
            demultiply(&mut data);
        }

        Ok(Raster {
            width: target.width,
            height: target.height,
            format: RasterFormat::Rgba8,
            data,
        })
    }
}

/// Copies a rendered texture into straight-alpha RGBA8 pixels.
///
/// `copy_texture_to_buffer` requires each row to begin on a
/// [`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`] boundary, so the staging buffer holds padded rows
/// that are stripped on the way out. Getting this wrong yields an image that is
/// progressively sheared rather than obviously broken, which is why a test covers a width
/// whose row length is not already aligned.
///
/// A free function rather than a method because a soft mask is read back the same way and
/// through a device the caller may own — the viewer shares one with its window.
///
/// # Errors
///
/// [`GpuRasterError::TargetTooLarge`] if the row arithmetic overflows, and
/// [`GpuRasterError::Readback`] if the copy or the mapping fails.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "row offsets are bounded by the buffer that was just allocated from \
              these same dimensions, and the two multiplications that could \
              overflow are checked explicitly above"
)]
fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    target: TargetSpec,
) -> Result<Vec<u8>, GpuRasterError> {
    let unpadded_row = target
        .width
        .checked_mul(4)
        .ok_or(GpuRasterError::TargetTooLarge)?;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_row = unpadded_row
        .div_ceil(align)
        .checked_mul(align)
        .ok_or(GpuRasterError::TargetTooLarge)?;
    let buffer_size = u64::from(padded_row) * u64::from(target.height);

    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(target.height),
            },
        },
        wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        // A send failure would mean the receiver is gone, which cannot happen
        // while this function is still on the stack waiting on it.
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            // Bounded rather than indefinite: a wedged driver must surface as an
            // error instead of hanging the viewer or the test suite.
            timeout: Some(std::time::Duration::from_secs(GPU_WAIT_TIMEOUT_SECS)),
        })
        .map_err(|e| GpuRasterError::Readback(e.to_string()))?;
    receiver
        .recv()
        .map_err(|e| GpuRasterError::Readback(e.to_string()))?
        .map_err(|e| GpuRasterError::Readback(e.to_string()))?;

    let mapped = slice.get_mapped_range();
    let row_len = unpadded_row as usize;
    let mut data = Vec::with_capacity(row_len * target.height as usize);
    for row in 0..target.height as usize {
        let start = row * padded_row as usize;
        data.extend_from_slice(&mapped[start..start + row_len]);
    }
    drop(mapped);
    staging.unmap();
    Ok(data)
}

impl Rasterizer for GpuRasterizer {
    type Error = GpuRasterError;

    fn name(&self) -> &'static str {
        "gpu"
    }

    fn rasterize(&mut self, list: &DisplayList, target: TargetSpec) -> Result<Raster, Self::Error> {
        // §11.4.7's page group in a four-component blending space is drawn twice and the two
        // rasters put back together (`pdf_render::blending`); a Vello scene renders one, and
        // this backend has no place to hold the second. Refused by name, which sends the frame
        // to the CPU backend — the job `CLAUDE.md` keeps that backend for — rather than
        // painting the page in the complements of cyan, magenta and yellow with no black.
        if list.blending().is_some() {
            return Err(GpuRasterError::UnsupportedCommand(
                "a page composited in a four-component blending colour space (§11.4.7)".to_owned(),
            ));
        }
        // And its one-component form, a `CalGray` or `ICCBased` 'GRAY' page group whose
        // composited component leaves through a curve (`pdf_render::blending::GreyCurve`):
        // the scene's colours would be components and not light, and a Vello scene has no
        // pass over its own result to put the curve in.
        if list.grey_curve().is_some() {
            return Err(GpuRasterError::UnsupportedCommand(
                "a page composited in a one-component blending colour space through a curve \
                 (§11.4.7)"
                    .to_owned(),
            ));
        }
        // And its three-component form, a `CalRGB` or `ICCBased` 'RGB ' page group whose
        // composited components leave through a cube (`pdf_render::blending::ColourCube`),
        // for the same reason.
        if list.colour_cube().is_some() {
            return Err(GpuRasterError::UnsupportedCommand(
                "a page composited in a three-component CIE-based blending colour space \
                 through a cube (§11.4.7)"
                    .to_owned(),
            ));
        }
        // Before the scene, because every mask a command names has to exist by the time that
        // command is encoded — and because a mask is a render of its own, at this target.
        let masks = evaluate_soft_masks(
            &self.context.device,
            &self.context.queue,
            &mut self.context.renderer,
            list,
            target,
        )?;
        let mut scene = scene::build(list, target, &masks)?;

        // Transparent, not the medium: §11.4.7's page group is isolated, so the medium's
        // colour is composited with the *result* rather than being the backdrop the page's
        // own blend modes see. `read_back` ends with `impose_within`, which is the same
        // function every other backend uses.
        let texture = render_to_texture(
            &self.context.device,
            &self.context.queue,
            &mut self.context.renderer,
            &mut scene,
            target,
            &mut self.bands,
        )?;

        self.read_back(list, &texture, target)
    }
}

/// Converts straight-alpha RGBA to premultiplied, in place: [`demultiply`]'s inverse.
fn premultiply(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha == u8::MAX {
            continue;
        }
        for channel in &mut pixel[..3] {
            // `u8 * u8` peaks at 65 025, inside `u16`, and the quotient is at most 255.
            // Rounded rather than truncated so that a fully opaque channel round-trips.
            let scaled = u16::from(*channel)
                .saturating_mul(u16::from(alpha))
                .saturating_add(127);
            *channel = u8::try_from(scaled / 255).unwrap_or(u8::MAX);
        }
    }
}

/// Converts premultiplied RGBA to straight alpha, in place.
///
/// [`Raster`] is documented as straight alpha, which is also what Vello hands back — so this
/// runs only over what [`premultiply`] did, to undo it after the page has been imposed on its
/// medium (§11.4.7). With an opaque medium every pixel ends fully opaque and the pair is a
/// no-op; with a transparent one they are exact inverses except for the rounding of a channel
/// that is about to be divided by its own alpha again.
///
/// **This function used to be applied to Vello's output directly, on the belief that Vello
/// wrote premultiplied alpha, and that was wrong.** It went unnoticed for fifteen sessions
/// because the background was opaque and every pixel came back with an alpha of 255, where
/// the conversion is the identity. The first render onto transparency showed it: a pixel half
/// covered by a 50% grey came back `[128, 0, 0, 128]` from `tiny-skia` and `[255, 0, 0, 128]`
/// from here, the colour divided by its own coverage. `vello_hands_back_straight_alpha` pins
/// it.
fn demultiply(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha == 0 || alpha == u8::MAX {
            continue;
        }
        for channel in &mut pixel[..3] {
            // `u16::from(u8) * 255` peaks at 65 025, inside u16, and `alpha` is neither
            // 0 nor 255 here, so neither the multiplication nor the division can fail.
            // Written with checked operations so that holds without needing a lint
            // exception; the branch above makes this the uncommon path anyway.
            let scaled = u16::from(*channel).saturating_mul(255);
            let value = scaled
                .checked_div(u16::from(alpha))
                .unwrap_or(u16::from(u8::MAX));
            *channel = u8::try_from(value).unwrap_or(u8::MAX);
        }
    }
}

/// Failures specific to the GPU backend.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum GpuRasterError {
    /// No GPU adapter could be found, not even a software one.
    #[error("no GPU adapter available: {0}")]
    NoAdapter(String),
    /// An adapter was found but would not provide a device.
    #[error("could not create a GPU device: {0}")]
    NoDevice(String),
    /// Vello's renderer could not be constructed.
    #[error("could not create the Vello renderer: {0}")]
    Renderer(String),
    /// Rendering the scene failed.
    #[error("GPU rendering failed: {0}")]
    Render(String),
    /// Copying pixels back from the GPU failed.
    #[error("reading pixels back from the GPU failed: {0}")]
    Readback(String),
    /// The requested target overflows the readback buffer layout arithmetic.
    #[error("target dimensions overflow the readback buffer layout")]
    TargetTooLarge,
    /// The scene overflowed one of Vello's fixed working buffers, so nothing was drawn.
    ///
    /// Not a defect in the page and not one in this tree: Vello's buffer sizes are constants
    /// chosen for its own test scenes, and a page of text at a high enough resolution exceeds
    /// them. The device draws nothing and says nothing; this is that silence, named. See
    /// `render_checked`.
    #[error(
        "could not draw a {width}x{height} scene: it needs more room than Vello's {wanted} \
         buffer has, so the device drew nothing"
    )]
    SceneTooLarge {
        /// The target's width in pixels.
        width: u32,
        /// The target's height in pixels.
        height: u32,
        /// Which of Vello's buffers wanted the most room.
        wanted: &'static str,
    },
    /// A command variant this backend does not implement yet.
    #[error("command not supported by the GPU backend: {0}")]
    UnsupportedCommand(String),
    /// A paint variant this backend does not implement yet.
    #[error("paint not supported by the GPU backend: {0}")]
    UnsupportedPaint(String),
    /// An image's dimensions and buffer length disagree.
    #[error("image is {width}x{height} but holds {bytes} bytes")]
    InvalidImage {
        /// Declared width.
        width: u32,
        /// Declared height.
        height: u32,
        /// Actual buffer length.
        bytes: usize,
    },
    /// A command referenced a clip that is not present in the display list.
    #[error("clip {0:?} is not present in this display list")]
    UnknownClip(ClipId),
    /// A command referenced a soft mask that was not evaluated for this target.
    #[error("soft mask {0:?} was not evaluated for this target")]
    UnknownSoftMask(SoftMaskId),
    /// A clip's parent chain forms a cycle.
    #[error("clip {0:?} is part of a cyclic parent chain")]
    CyclicClip(ClipId),
    /// A failure originating in the shared backend layer.
    #[error(transparent)]
    Target(#[from] pdf_render::BackendError),
}
