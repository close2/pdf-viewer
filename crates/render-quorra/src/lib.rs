//! The quorra backend: [`Rasterizer`] over the document renderer this viewer's
//! brief commissioned (`doc/RENDER_LIBRARY.md`, built at `../render-lib`).
//!
//! This crate is the bridge the brief's integration notes said would live on this
//! side: the display list's vocabulary maps onto quorra's scene almost one-to-one —
//! both were shaped by the same contract — and what differs is stated per method
//! rather than absorbed silently. The three real seams:
//!
//! - **Shadings.** Axial and radial map directly (quorra anchors a shading through
//!   its own transform, exactly as [`pdf_render::Shading`] does). A *sampled*
//!   shading — a grid, standing in for a function — becomes an image clipped to the
//!   filled path; the `render-gpu` backend refuses these outright, so drawing the
//!   domain is a strict improvement, and the one divergence (no pad-extension
//!   beyond the domain rectangle) is documented at the conversion.
//! - **Strokes.** quorra strokes but does not dash — dashing is settled on this
//!   side by `RENDER_LIBRARY.md` section 4.5 — so dashes run through the same `kurbo::dash` the
//!   `render-gpu` backend uses, and the §8.5.3.2 zero-length rules through the same
//!   `pdf-render` helpers, keeping the backends' answers identical by construction.
//! - **Alpha and the medium.** quorra renders onto transparency and hands back
//!   straight alpha (`RENDER_LIBRARY.md` section 3); the medium is imposed here through
//!   [`pdf_render::impose_on_medium`], the same function every backend uses.

#![forbid(unsafe_code)]

use pdf_render::{
    ClipId, Color, DisplayList, Raster, RasterFormat, Rasterizer, SoftMaskId, TargetSpec,
};
use quorra_scene::ResourceId;

mod cache;
mod present;
mod scene;
mod stroke;

pub use present::{PresentFrame, QuorraPresenter};

/// Why a frame could not be produced. Every variant names what refused (the same
/// contract as the other backends: unsupported input is an error, never a skipped
/// command — a silently omitted draw hands the comparison harness a
/// plausible-looking wrong image instead of a failure).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QuorraRasterError {
    /// quorra refused a resource upload (validation or budget, named inside).
    #[error("resource upload refused: {0}")]
    Device(#[from] quorra_gpu::DeviceError),
    /// quorra refused the frame (budget, limits, or a dangling resource id).
    #[error("frame refused: {0}")]
    Render(#[from] quorra_gpu::RenderError),
    /// quorra's scene boundary refused a value the display list carried.
    #[error("scene refused: {0}")]
    Scene(#[from] quorra_scene::SceneError),
    /// The display list references a clip it does not define.
    #[error("display list references unknown clip {0:?}")]
    UnknownClip(ClipId),
    /// A clip chain loops back on itself.
    #[error("clip chain cycles at {0:?}")]
    CyclicClip(ClipId),
    /// The display list references a soft mask it does not define.
    #[error("display list references unknown soft mask {0:?}")]
    UnknownSoftMask(SoftMaskId),
    /// A command or paint this backend cannot draw as asked.
    #[error("this backend cannot draw {0}")]
    Unsupported(String),
}

/// The quorra backend: a persistent device, resource caches keyed by the display
/// list's own `Arc` identities (pinned and evicted by [`cache::ResourceCaches`],
/// whose docs carry both the ABA argument for pinning and the eviction policy
/// `QUORRA_FEEDBACK.md` section 2 asked for), and the medium colour to impose.
#[derive(Debug)]
pub struct QuorraRasterizer {
    device: quorra_gpu::Device,
    background: Color,
    caches: cache::ResourceCaches,
}

impl QuorraRasterizer {
    /// A backend on the best adapter quorra can find.
    ///
    /// Returns as soon as the device exists; quorra compiles its warm pipeline set
    /// on a background thread, so construction does not wait for shaders.
    ///
    /// # Errors
    ///
    /// [`QuorraRasterError::Device`] when no adapter yields a device.
    pub fn new_headless() -> Result<Self, QuorraRasterError> {
        Self::with_options(&quorra_gpu::Options::default())
    }

    /// A backend pinned to a software adapter (llvmpipe), for CI and comparison
    /// runs that must not depend on hardware.
    ///
    /// # Errors
    ///
    /// As [`QuorraRasterizer::new_headless`].
    pub fn new_headless_software() -> Result<Self, QuorraRasterError> {
        Self::with_options(&quorra_gpu::Options {
            adapter: Some("llvmpipe".into()),
            ..quorra_gpu::Options::default()
        })
    }

    /// A backend with explicit quorra options — `RENDER_LIBRARY.md` section 4.5 makes the glyph
    /// cache's sub-pixel quantum the caller's decision to take, and this is where
    /// a caller takes it.
    ///
    /// # Errors
    ///
    /// As [`QuorraRasterizer::new_headless`].
    pub fn with_options(options: &quorra_gpu::Options) -> Result<Self, QuorraRasterError> {
        Ok(Self {
            device: quorra_gpu::Device::headless(options)?,
            background: Color::WHITE,
            caches: cache::ResourceCaches::new(),
        })
    }

    /// Sets the medium colour the finished page is imposed on (white by default,
    /// as on the other backends).
    #[must_use]
    pub fn with_background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    /// Which lane draws coverage from now on, as [`QuorraPresenter::set_coverage`] does it.
    ///
    /// Here as well as on the presenter because the *offscreen* path is where a lane can be
    /// compared against the CPU oracle: `viewer-ui` switches lanes at a magnification
    /// (`GPU_COVERAGE_MAGNIFICATION`), so a headless ladder that never switches is not
    /// measuring what a person sees past 1000%. `examples/zoom_ladder` is the caller.
    ///
    /// [`QuorraPresenter::set_coverage`]: crate::present::QuorraPresenter::set_coverage
    pub fn set_coverage(&mut self, coverage: quorra_gpu::Coverage) {
        self.device.set_coverage(coverage);
    }

    /// The adapter quorra selected, for reports and golden-file metadata.
    #[must_use]
    pub fn adapter_description(&self) -> &str {
        self.device.description()
    }

    /// A whole *window's* frame — page, raster stand-in and overlays — drawn offscreen.
    ///
    /// [`QuorraPresenter::present`] is the same scene onto a swapchain, and this is the only way
    /// to look at one without a window. It exists because a defect lived where no instrument
    /// reached: `viewer-ui`'s sidebar stops being drawn above about 2000% magnification on the
    /// graphics device and not on the processor (`doc/todo/12`), and every gate in this tree
    /// rasterises **one** display list — the corpus and the oracle a page, `tests/corpus.rs` a
    /// page at 1×, 2× and 4×. None of them puts chrome over a magnified page, which is exactly
    /// the combination that breaks.
    ///
    /// The medium is the bottom of the scene rather than imposed afterwards, as it is on the
    /// surface path: a window has an opaque background, and this is a window's frame.
    ///
    /// # Errors
    ///
    /// As [`Rasterizer::rasterize`], plus whatever the overlays' own commands refuse.
    ///
    /// [`QuorraPresenter::present`]: crate::present::QuorraPresenter::present
    pub fn rasterize_frame(
        &mut self,
        frame: &PresentFrame<'_>,
    ) -> Result<Raster, QuorraRasterError> {
        self.caches.begin_frame();
        let mut builder = quorra_scene::SceneBuilder::new();
        let mut transient: Vec<ResourceId> = Vec::new();
        let built = present::build(
            &mut self.device,
            &mut self.caches,
            self.background,
            &mut builder,
            frame,
            &mut transient,
        );

        let rendered = built.and_then(|()| {
            let scene = builder.finish();
            let viewport = quorra_gpu::Viewport::full(
                frame.width,
                frame.height,
                quorra_scene::Affine::IDENTITY,
            );
            Ok(self
                .device
                .render(&scene, &viewport, quorra_gpu::Target::Readback)?)
        });

        let mut release_error: Option<quorra_gpu::DeviceError> = None;
        for id in transient.drain(..) {
            if let Err(error) = self.device.release(id) {
                release_error.get_or_insert(error);
            }
        }
        self.caches.evict_settled(&mut self.device)?;
        let readback = rendered?;
        if let Some(error) = release_error {
            return Err(error.into());
        }
        Ok(Raster {
            width: frame.width,
            height: frame.height,
            // Opaque throughout: the medium is the bottom of this scene, so straight and
            // premultiplied alpha are the same bytes and no imposition is owed.
            format: RasterFormat::Rgba8,
            data: readback.into_raster()?.into_pixels(),
        })
    }
}

impl Rasterizer for QuorraRasterizer {
    type Error = QuorraRasterError;

    fn name(&self) -> &'static str {
        "quorra"
    }

    fn rasterize(&mut self, list: &DisplayList, target: TargetSpec) -> Result<Raster, Self::Error> {
        self.caches.begin_frame();
        let mut builder = quorra_scene::SceneBuilder::new();
        let mut transient: Vec<ResourceId> = Vec::new();
        let built = scene::Encoder::new(
            &mut self.device,
            list,
            target,
            &mut self.caches,
            &mut transient,
        )
        .commands(&mut builder, list.commands());

        let rendered = built.and_then(|()| {
            let scene = builder.finish();
            // The target transform is baked into every command at translation
            // (`Encoder::placed`), so the viewport itself is identity.
            let viewport = quorra_gpu::Viewport::full(
                target.width,
                target.height,
                quorra_scene::Affine::IDENTITY,
            );
            Ok(self
                .device
                .render(&scene, &viewport, quorra_gpu::Target::Readback)?)
        });

        // Per-frame resources (clips, dashed strokes, meshes, sampled grids) go
        // back to the budget whether the frame drew or refused — every one of
        // them, even after a failure — and the frame's own error, if any,
        // outranks a release problem.
        let mut release_error: Option<quorra_gpu::DeviceError> = None;
        for id in transient.drain(..) {
            if let Err(error) = self.device.release(id) {
                release_error.get_or_insert(error);
            }
        }
        // Frames drawn and frames refused both settle the caches: a long session
        // must stay healthy through its refusals (QUORRA_FEEDBACK.md section 2).
        self.caches.evict_settled(&mut self.device)?;
        let frame = rendered?;
        if let Some(error) = release_error {
            return Err(error.into());
        }

        // Transparent render, then the medium (§11.4.7: the page group is
        // isolated, so the medium composites with the *result*) — premultiplied
        // for the imposition, straight alpha for the Raster, as on every backend.
        let mut data = frame.into_raster()?.into_pixels();
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

/// Converts straight-alpha RGBA to premultiplied, in place ([`demultiply`]'s
/// inverse; the same rounding as the other backends).
fn premultiply(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha == u8::MAX {
            continue;
        }
        for channel in &mut pixel[..3] {
            // u8 × u8 peaks at 65 025, inside u16; the quotient is at most 255.
            let scaled = u16::from(*channel)
                .saturating_mul(u16::from(alpha))
                .saturating_add(127);
            *channel = u8::try_from(scaled / 255).unwrap_or(u8::MAX);
        }
    }
}

/// Converts premultiplied RGBA to straight alpha, in place.
fn demultiply(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha == 0 || alpha == u8::MAX {
            continue;
        }
        for channel in &mut pixel[..3] {
            let scaled = u16::from(*channel).saturating_mul(255);
            let value = scaled
                .checked_div(u16::from(alpha))
                .unwrap_or(u16::from(u8::MAX));
            *channel = u8::try_from(value).unwrap_or(u8::MAX);
        }
    }
}
