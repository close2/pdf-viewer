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
//!   side by the brief's §4.5 — so dashes run through the same `kurbo::dash` the
//!   `render-gpu` backend uses, and the §8.5.3.2 zero-length rules through the same
//!   `pdf-render` helpers, keeping the backends' answers identical by construction.
//! - **Alpha and the medium.** quorra renders onto transparency and hands back
//!   straight alpha (its §3); the medium is imposed here through
//!   [`pdf_render::impose_on_medium`], the same function every backend uses.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use pdf_render::{
    ClipId, Color, DisplayList, Raster, RasterFormat, Rasterizer, SoftMaskId, TargetSpec,
};
use quorra_scene::ResourceId;

mod scene;
mod stroke;

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
/// list's own `Arc` identities, and the medium colour to impose.
///
/// Each cache entry **holds a clone of the `Arc` it is keyed by**. That is not a
/// convenience: a pointer key alone is an ABA bug — drop a display list, let the
/// allocator hand the same address to a different path, and the cache serves the
/// old outline for the new geometry, sporadically, by allocator mood. Pinning the
/// allocation makes the address unique for as long as the entry lives, and is
/// what lets the cache span rasterize calls (a zoom re-renders the same `Arc`s
/// and re-uploads nothing, which is the §2.2 economy).
#[derive(Debug)]
pub struct QuorraRasterizer {
    device: quorra_gpu::Device,
    background: Color,
    /// Uploaded outlines by (pinned) path identity — 5 933 fills of 107 distinct
    /// outlines upload 107 outlines, once.
    outlines: HashMap<usize, (std::sync::Arc<pdf_render::Path>, quorra_scene::OutlineId)>,
    /// Uploaded images by (pinned) sample-data identity.
    images: HashMap<usize, (std::sync::Arc<[u8]>, quorra_scene::ImageId)>,
    /// Uploaded colour ramps by (pinned) shading-kind identity.
    ramps: HashMap<usize, (std::sync::Arc<pdf_render::ShadingKind>, quorra_scene::RampId)>,
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

    /// A backend with explicit quorra options — the brief's §4.5 makes the glyph
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
            outlines: HashMap::new(),
            images: HashMap::new(),
            ramps: HashMap::new(),
        })
    }

    /// Sets the medium colour the finished page is imposed on (white by default,
    /// as on the other backends).
    #[must_use]
    pub fn with_background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    /// The adapter quorra selected, for reports and golden-file metadata.
    #[must_use]
    pub fn adapter_description(&self) -> &str {
        self.device.description()
    }
}

impl Rasterizer for QuorraRasterizer {
    type Error = QuorraRasterError;

    fn name(&self) -> &'static str {
        "quorra"
    }

    fn rasterize(&mut self, list: &DisplayList, target: TargetSpec) -> Result<Raster, Self::Error> {
        let mut builder = quorra_scene::SceneBuilder::new();
        let mut transient: Vec<ResourceId> = Vec::new();
        let built = scene::Encoder::new(
            &mut self.device,
            list,
            target,
            &mut self.outlines,
            &mut self.images,
            &mut self.ramps,
            &mut transient,
        )
        .commands(&mut builder, list.commands());

        let rendered = built.and_then(|()| {
            let scene = builder.finish();
            let viewport = quorra_gpu::Viewport::full(
                target.width,
                target.height,
                scene::affine(target.transform),
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
