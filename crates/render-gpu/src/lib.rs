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
    ClipId, Color, DisplayList, Raster, RasterFormat, Rasterizer, SoftMaskId, TargetSpec,
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
    soft_mask::evaluate(list, target, &mut |scene| {
        let texture = render_to_texture(device, queue, renderer, scene, target)?;
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
    scene: &vello::Scene,
    target: TargetSpec,
) -> Result<wgpu::Texture, GpuRasterError> {
    // Vello writes its result through a storage binding, and the pixels are then
    // copied out, so both usages are required.
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
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_to_texture(
            device,
            queue,
            scene,
            &view,
            &vello::RenderParams {
                base_color: vello::peniko::Color::TRANSPARENT,
                width: target.width,
                height: target.height,
                antialiasing_method: vello::AaConfig::Area,
            },
        )
        .map_err(|e| GpuRasterError::Render(e.to_string()))?;
    Ok(texture)
}

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
    background: Color,
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
            background: Color::WHITE,
        }
    }

    /// Sets the background colour.
    ///
    /// Defaults to opaque white, matching `render_cpu::CpuRasterizer`, so that the two
    /// backends are directly comparable without configuring either.
    #[must_use]
    pub fn with_background(mut self, background: Color) -> Self {
        self.background = background;
        self
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
        texture: &wgpu::Texture,
        target: TargetSpec,
    ) -> Result<Raster, GpuRasterError> {
        let mut data = read_pixels(&self.context.device, &self.context.queue, texture, target)?;

        if self.background.a > 0.0 {
            premultiply(&mut data);
            pdf_render::impose_on_medium(&mut data, self.background);
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
        // Before the scene, because every mask a command names has to exist by the time that
        // command is encoded — and because a mask is a render of its own, at this target.
        let masks = evaluate_soft_masks(
            &self.context.device,
            &self.context.queue,
            &mut self.context.renderer,
            list,
            target,
        )?;
        let scene = scene::build(list, target, &masks)?;

        // Transparent, not the background: §11.4.7's page group is isolated, so the medium's
        // colour is composited with the *result* rather than being the backdrop the page's
        // own blend modes see. `read_back` ends with `impose_on_medium`, which is the same
        // function the CPU backend uses.
        let texture = render_to_texture(
            &self.context.device,
            &self.context.queue,
            &mut self.context.renderer,
            &scene,
            target,
        )?;

        self.read_back(&texture, target)
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
