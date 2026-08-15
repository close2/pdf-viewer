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

pub use present::{FrameCost, PresentFrame, QuorraPresenter};
pub use scene::FunctionPaints;

/// Why a frame could not be produced. Every variant names what refused (the same
/// contract as the other backends: unsupported input is an error, never a skipped
/// command — a silently omitted draw hands the comparison harness a
/// plausible-looking wrong image instead of a failure).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QuorraRasterError {
    /// quorra's device said no: bringing it up, or a resource upload (named inside).
    ///
    /// **The prefix is gone, and it was wrong.** This read `resource upload refused: {0}` until
    /// the three-hundred-and-eighty-fourth session, which is true of three of `DeviceError`'s
    /// seven variants and false of the four that are about *construction* — so a machine with no
    /// adapter for the backend it was given reported "resource upload refused: surface creation
    /// failed", which is two claims and one of them invented. Every variant already names what
    /// happened, so there is nothing for a prefix to add.
    #[error("{0}")]
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

/// quorra's options as *this host* asks for them: [`quorra_gpu::Options::default`] with the one
/// field this tree decides rather than accepts.
///
/// **`Options::encode_threads` is a permission, not a preference** — upstream's own word for it
/// (their ADR 0054, `doc/QUORRA_ENCODE_THREADS_ANSWER.md` section 4). Its default is 1, because
/// only a host knows whether it has a pool of its own, a seccomp policy that forbids a thread, or
/// a launch path a pool would land on. So the number is ours to choose, and this is the one place
/// that chooses it: every constructor below and every caller that wants quorra's defaults plus
/// this decision spreads `..render_quorra::options()` rather than repeating the reasoning.
///
/// **The answer is what the machine says it has**, and it is measured rather than assumed —
/// `examples/encode_threads.rs` is the instrument and ADR 0377 has every row. On the project
/// owner's 58 009-command drawing at its fit view, a cold device per sample and the minimum of
/// five round-robin rounds, quorra's `encode` phase falls from 467 ms on one thread to 221 on
/// eight and 151 on twenty-four with the machine quiet; with it deliberately loaded past its own
/// core count the twenty-four-thread column is still the fastest of the six. Upstream reported a
/// round of *theirs* where twenty-four threads read worse than eight at load 25–33 and declined
/// to publish any crossover as a constant, which is exactly why this asks the machine at run time
/// instead of naming a number here: [`std::thread::available_parallelism`] answers the question
/// this machine's cgroup and affinity mask were asked, and a build moved to a smaller machine
/// gets that machine's answer.
///
/// **Nothing here is built at startup.** quorra clamps this number when the device is
/// constructed and spawns nothing until a frame's geometry passes its own floor, so the
/// cold-start gate is untouched — checked in ADR 0377 rather than repeated from upstream.
#[must_use]
pub fn options() -> quorra_gpu::Options {
    quorra_gpu::Options {
        encode_threads: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        ..quorra_gpu::Options::default()
    }
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
    /// The window frame [`QuorraRasterizer::rasterize_frame`] last drew, kept for the next one.
    slot: present::FrameSlot,
    last: FrameCost,
    functions: FunctionPaints,
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
        Self::with_options(&options())
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
            ..options()
        })
    }

    /// A backend with explicit quorra options — `RENDER_LIBRARY.md` section 4.5 makes the glyph
    /// cache's sub-pixel quantum the caller's decision to take, and this is where
    /// a caller takes it.
    ///
    /// **Spread [`options`] rather than [`quorra_gpu::Options::default`]** unless the point is to
    /// have quorra's own defaults: the difference is `encode_threads`, whose default is 1 and
    /// whose value is this host's decision to make once (see [`options`]).
    ///
    /// # Errors
    ///
    /// As [`QuorraRasterizer::new_headless`].
    pub fn with_options(options: &quorra_gpu::Options) -> Result<Self, QuorraRasterError> {
        Ok(Self {
            device: quorra_gpu::Device::headless(options)?,
            background: Color::WHITE,
            caches: cache::ResourceCaches::new(),
            slot: present::FrameSlot::default(),
            last: FrameCost::default(),
            functions: FunctionPaints::default(),
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
    /// graphics device and not on the processor (ADR 0198), and every gate in this tree
    /// rasterises **one** display list — the corpus and the oracle a page, `tests/corpus.rs` a
    /// page at 1×, 2× and 4×. None of them puts chrome over a magnified page, which is exactly
    /// the combination that breaks.
    ///
    /// The medium is the bottom of the scene rather than imposed afterwards, as it is on the
    /// surface path: a window has an opaque background, and this is a window's frame.
    ///
    /// **It reuses its scene exactly as the surface path does** (ADR 0351) — the same
    /// [`present::FrameSlot`], the same key — which is what lets a test look at a replayed frame
    /// at all: `Device::render_retained` replays into a `Readback` target as readily as onto a
    /// swapchain, and quorra's own survival table says the target is not part of the question.
    /// [`QuorraRasterizer::last_frame`] says which of the two this call was.
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
        let began = std::time::Instant::now();
        self.last = FrameCost::default();
        let drawn = self.slot.render(
            &mut self.device,
            &mut self.caches,
            self.background,
            frame,
            quorra_gpu::Target::Readback,
            &mut present::Reported {
                cost: &mut self.last,
                functions: &mut self.functions,
            },
        );
        self.last.total = began.elapsed();
        Ok(Raster {
            width: frame.width,
            height: frame.height,
            // Opaque throughout: the medium is the bottom of this scene, so straight and
            // premultiplied alpha are the same bytes and no imposition is owed.
            format: RasterFormat::Rgba8,
            data: drawn?.into_raster()?.into_pixels(),
        })
    }

    /// What the last [`Self::rasterize_frame`] cost, stage by stage — including whether it
    /// encoded its scene or replayed one, which is the observable this whole path is tested by.
    #[must_use]
    pub fn last_frame(&self) -> FrameCost {
        self.last
    }

    /// What the scene last built did with §8.7.4.5.2's type 1 shadings: how many the device
    /// evaluated, and the ground on which it declined each of the rest (ADR 0376).
    #[must_use]
    pub fn last_function_paints(&self) -> &FunctionPaints {
        &self.functions
    }
}

impl Rasterizer for QuorraRasterizer {
    type Error = QuorraRasterError;

    fn name(&self) -> &'static str {
        "quorra"
    }

    fn rasterize(&mut self, list: &DisplayList, target: TargetSpec) -> Result<Raster, Self::Error> {
        let mut data = self.render(list, target)?;

        // ISO 32000-2 §11.4.7 puts a colour space under the whole page — "[a]ll page-level
        // compositing shall be done in the default blending colour space of the page, and the
        // entire result shall then ... be converted to the native colour space of the output
        // device before being composited with the context-dependent backdrop". Where that
        // space has four components §11.3.4 makes the compositing formula per component, so
        // the page arrives as *two* lists on the same geometry — the additive complements of
        // cyan, magenta and yellow, and the complement of black — and this draws the second
        // one against the same device before `pdf_render::blending` puts the pair back
        // together. That the second render is whole, shares the first's uploaded resources
        // and costs no geometry at all is `quorra-gpu/tests/two_rasters.rs`'s claim, and
        // `doc/QUORRA_FEEDBACK.md` section 17.1 is where it was measured.
        let four_components = list.blending().zip(list.black());
        // Both passes are needed premultiplied: `blending::resolve` divides each channel by
        // the pixel's own alpha to recover the ink, and `impose_on_medium` composites the
        // medium in the space where that composite is exact. quorra hands back straight
        // alpha, so the round trip is here — and only where something below needs it, because
        // it is lossy at a partly transparent pixel.
        if four_components.is_some() || self.background.a > 0.0 {
            premultiply(&mut data);
            if let Some((space, black)) = four_components {
                let mut ink = self.render(black, target)?;
                premultiply(&mut ink);
                pdf_render::resolve_blending(&mut data, &ink, space);
            }
            // §11.4.7's page group is isolated, so the medium composites with the *result* —
            // and after the conversion above, which is where the clause puts it.
            if self.background.a > 0.0 {
                pdf_render::impose_on_medium(&mut data, self.background);
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

impl QuorraRasterizer {
    /// One display list through the device, as straight-alpha RGBA8 with no medium under it.
    ///
    /// Separate from [`Rasterizer::rasterize`] because §11.4.7's four-component page is two
    /// lists over one page and both take exactly this path — the same device, the same
    /// caches, the same per-frame releases — and nothing about drawing one of them may
    /// depend on which of the two it is.
    ///
    /// # Errors
    ///
    /// As [`Rasterizer::rasterize`].
    fn render(
        &mut self,
        list: &DisplayList,
        target: TargetSpec,
    ) -> Result<Vec<u8>, QuorraRasterError> {
        // A single-list rasterisation is a different scene, and this path evicts: an entry the
        // retained window frame's scene still names could go, leaving that scene holding
        // released ids. Discarding it first is three lines against a class of reasoning about
        // an instance used both ways, which nothing in this tree does but nothing forbids.
        self.slot.discard(&mut self.device)?;
        self.caches.begin_frame();
        let mut builder = quorra_scene::SceneBuilder::new();
        let mut transient: Vec<ResourceId> = Vec::new();
        self.functions.clear();
        let built = scene::Encoder::new(
            &mut self.device,
            list,
            target,
            &mut self.caches,
            &mut transient,
            &mut self.functions,
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

        Ok(frame.into_raster()?.into_pixels())
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
