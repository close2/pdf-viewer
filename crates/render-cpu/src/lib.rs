//! CPU rasteriser backend; the correctness oracle for the GPU backend.
//!
//! Implements [`pdf_render::Rasterizer`] on the CPU using `tiny-skia`. This backend
//! has three jobs, and the second and third are why it exists first:
//!
//! 1. It renders when no usable GPU is present.
//! 2. It is the **reference** against which `render-gpu` is validated. Both backends
//!    consume the same [`pdf_render::DisplayList`], so any difference between their
//!    outputs is a backend defect rather than a difference in how the document was
//!    interpreted — a far tighter test than comparing against another PDF viewer,
//!    where antialiasing and colour handling differ for legitimate reasons.
//! 3. It is the **startup path**. Creating a GPU device and compiling pipelines costs
//!    tens to hundreds of milliseconds, so page one renders here while the GPU
//!    initialises on another thread.
//!
//! Correctness therefore outranks speed in this crate. Where the two conflict, this
//! backend takes the clearer construction and leaves optimisation to `render-gpu`.
//! See `doc/adr/0002-cpu-rasteriser-first.md`.

#![forbid(unsafe_code)]

mod blend;
mod convert;
mod images;
mod scan;
mod shading;

use rayon::iter::{IndexedParallelIterator as _, IntoParallelIterator as _, ParallelIterator as _};
use rayon::slice::ParallelSliceMut as _;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use pdf_render::{
    BackendError, ClipId, Command, DisplayList, Interrupt, MAX_EXTENT, MAX_GROUP_DEPTH, Medium,
    Paint, Path, Raster, RasterFormat, Rasterizer, SoftMaskId, Stroke, TargetSpec, Transform,
};

/// Every paint this backend hands `tiny-skia` asks for its high-precision pipeline, ISO 32000-2
/// §11.3.6.
///
/// `tiny-skia` compiles a raster pipeline twice over: a *lowp* one that carries a pixel as four
/// `u16`s in 0..=255, and a *highp* one that carries it as `f32`. It picks the first whenever
/// every stage of the pipeline has a lowp implementation, which for a solid colour drawn through
/// a mask is always — and the lowp arithmetic is not this clause's:
///
/// > the compositing formula collapses to a simple weighted average of the backdrop and source
/// > colours, controlled by the backdrop and source alpha values
///
/// A weighted average by `α` needs a division by 255 to get back from two byte factors to one,
/// and lowp's is `div255(v) = (v + 255) >> 8`. That is an *upper* bound on `v ÷ 255` rather than
/// its rounding — `255·(v + 255) ≥ 256·v` for every `v ≤ 255²` — and this path spends two of
/// them per pixel, one scaling the source by the mask and one scaling the destination by
/// `1 − α`. Both biases point the same way, so a masked mark comes out **up to two levels of 255
/// too close to the backdrop**, always in that direction.
///
/// Measured, on the whole range a mask value can take: `smask_luminosity_oob_transfer.pdf`'s
/// page composites `0.85 0.2 0.1 rg` over `0.95 0.95 0.95 rg` at a mask of `191`, whose closed
/// form is `(223, 99, 80)`; the lowp pipeline gives `(223, 100, 81)` and the highp pipeline gives
/// the closed form. Swept over all 256 mask values the highp pipeline reproduces the closed form
/// **exactly at every one of them** and the lowp pipeline departs by up to two levels. The
/// eight-bit mask is not the cause and was blamed for it for five hundred sessions; ADR 0418.
///
/// # What it costs, because a correctness fix still has to be priced
///
/// Nothing measurable, and on two pages of three it is cheaper —
/// `examples/callgrind_rasterise`, A/B in one sitting: ISO 32000-2 page 101 **5568.7 M → 5454.1 M**
/// instructions (−2.1%), `alphatrans.pdf` **1977.3 M → 1949.7 M** (−1.4%), `firefox_logo.pdf`
/// **855.2 M → 860.0 M** (+0.6%). The lowp pipeline processes sixteen pixels a stage against the
/// highp one's eight, and pays for it in the `u16` packing and in `div255` itself.
const HIGH_PRECISION_PIPELINE: bool = true;

/// Renders display lists on the CPU.
#[derive(Debug, Clone)]
pub struct CpuRasterizer {
    medium: Medium,
    anti_alias: bool,
    strips: Option<u32>,
    interrupt: Option<Interrupt>,
    /// Deeply reduced image samples, so that the strips of one draw — and the draws of one
    /// rasteriser — pay for a reduction once between them. See [`images`] for what is shared
    /// and why an address is a sound key for it.
    ///
    /// **Held here rather than made per draw, and that is the liveness rule**: a rasteriser
    /// kept across frames keeps its reductions, which is what `viewer-confined`'s worker does,
    /// and one made per job keeps nothing, which is what `viewer_host::drawing` does
    /// deliberately. Behind an `Arc` because `Clone` on this type means another handle on the
    /// same rasteriser rather than a second one that has to warm up again.
    reduced_images: Arc<images::ReducedImages>,
}

impl CpuRasterizer {
    /// Creates a rasteriser whose whole target is the page, painted on §11.4.7's white 𝑊.
    ///
    /// White rather than transparent because a PDF page is conceptually opaque white
    /// unless the document paints otherwise, and because the reference renderers used
    /// by the comparison harness (`pdftoppm`, `mutool draw`) do the same. Matching
    /// them removes an entire class of spurious differences.
    ///
    /// [`Medium::PAGE_ONLY`] rather than [`Medium::WINDOW`] because a rasteriser is handed one
    /// display list and a target for it: every target this tree builds for a single page is the
    /// page's own extent, and a caller drawing into something *larger* than the page says so
    /// with [`Self::with_medium`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            medium: Medium::PAGE_ONLY,
            anti_alias: true,
            strips: None,
            interrupt: None,
            reduced_images: Arc::default(),
        }
    }

    /// Hands the rasteriser a flag another thread can raise to abandon the draw.
    ///
    /// **This is what a host taking display lists across the confinement owns.** Since ADR 0633 a
    /// page usually crosses `viewer-confined` as marks and the host draws them, so the drawing of
    /// a document written to be expensive happens in the *unconfined* process — where the
    /// worker's cancel, which is a kill, reaches nothing (`doc/todo/34` §3). Without this there
    /// is no way to get that thread back at all.
    ///
    /// # What it is worth, and what it costs
    ///
    /// The flag is read once per command in [`CpuRasterizer::encode`], which is the loop nothing
    /// bounds: 1567 bytes of PDF amplify to ten thousand page-covering fills, 990 kB of marks and
    /// **27.6 s** of drawing at a 900x1165 window, and raising it returns the thread in 1.3 to
    /// 2.1 ms. What it cannot interrupt is one command's own scan conversion and the per-pixel
    /// pass after the drawing, both bounded by the target's area — see [`Interrupt`].
    ///
    /// The load is `Relaxed`, and what it costs was counted rather than assumed. Under callgrind,
    /// ISO 32000-2 page 101 — the densest page in this tree, 3007 commands — drawn twenty times:
    /// **5 441 579 467** instruction references before this method existed, **5 441 596 808** with
    /// no interrupt handed over, and **5 451 627 652** with one handed over and never raised. So
    /// the path every gate in this tree runs is unchanged and a caller that asks to be able to
    /// stop pays **0.18%**. ADR 0650 section 5.
    #[must_use]
    pub fn interruptible(mut self, interrupt: Interrupt) -> Self {
        self.interrupt = Some(interrupt);
        self
    }

    /// Asks for a fixed number of horizontal strips instead of one per available core.
    ///
    /// **This changes how long a page takes and almost nothing about what it looks like**, which
    /// is [`Surface`]'s claim and is the reason this method exists: the property is only
    /// checkable by rendering one page several ways and comparing the bytes, which is what
    /// `strip_parallelism.rs` does — over six scenes here, and over three real pages in
    /// `pdf-model`, which is where the exception the six scenes could not hold was found. What
    /// "almost" covers is `tiny-skia`'s own arithmetic at a shifted origin, worth one supersample
    /// on an edge that lands on one: ADR 0219 measures it and says why it cannot be removed.
    ///
    /// **One** is the value a caller with no filesystem must pass, because asking the machine
    /// reads `/proc` (ADR 0218). A page still gets fewer strips than asked for where its curves
    /// forbid the cuts.
    #[must_use]
    pub fn with_strips(mut self, strips: u32) -> Self {
        self.strips = Some(strips);
        self
    }

    /// Sets what the page is imposed on: §11.4.7's 𝑊, and what lies outside the page.
    ///
    /// [`Medium::NONE`] is the right choice when compositing a page over something else — an
    /// overlay, or another page of the same arrangement — and [`Medium::WINDOW`] when the target
    /// is a window rather than a page. See [`pdf_render::medium`] for which of the two colours
    /// the standard states and which is this program's.
    #[must_use]
    pub fn with_medium(mut self, medium: Medium) -> Self {
        self.medium = medium;
        self
    }

    /// Enables or disables antialiasing.
    ///
    /// Disabling it is useful when diffing against a reference renderer that was
    /// itself run without antialiasing, since it makes exact pixel comparison
    /// meaningful rather than merely approximate.
    #[must_use]
    pub fn with_anti_alias(mut self, anti_alias: bool) -> Self {
        self.anti_alias = anti_alias;
        self
    }

    /// Whether a caller has asked for this draw to be abandoned.
    ///
    /// `None` where no interrupt was handed over, which is every gate in this tree — and where the
    /// question costs 867 instructions a page rather than one check per command, which is the
    /// middle figure in [`CpuRasterizer::interruptible`]'s costing.
    fn interrupted(&self) -> bool {
        self.interrupt.as_ref().is_some_and(Interrupt::raised)
    }

    /// Builds the `tiny-skia` paint for a resolved paint and blend mode.
    ///
    /// `page_to_path` maps page space into the space the path is stated in — the
    /// inverse of the command's own transform. See [`shading::shader`] for why a paint
    /// is positioned in the path's space rather than the device's; getting this wrong
    /// draws a gradient in the right shape and the wrong place, which no metric
    /// short of looking at the page detects.
    ///
    /// `page_to_device` is the other direction and answers the other question: how many
    /// device pixels a sampled shading's domain covers, which is where its colours are
    /// produced (`Shading::sampled_at`). `target` is the extent of the pixmap being drawn
    /// into — the *band's*, where a page is banded, which clips a shading's grid tighter
    /// still and stays exact because a band's transform differs from the page's by a
    /// translation and its lattice is therefore the same one. Only the sampled kind reads
    /// either.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError::UnsupportedPaint`] for a paint variant this backend
    /// does not implement yet. [`Paint`] is `#[non_exhaustive]`, so shadings and
    /// patterns will be added to it before this backend handles them; failing loudly
    /// in the interim is deliberate, because a silent fallback colour would give the
    /// comparison harness a plausible-looking wrong image.
    fn paint<'a>(
        &self,
        paint: &Paint,
        blend: tiny_skia::BlendMode,
        page_to_path: Transform,
        page_to_device: Transform,
        target: (u32, u32),
        scratch: &'a mut Option<tiny_skia::Pixmap>,
    ) -> Result<tiny_skia::Paint<'a>, CpuRasterError> {
        let shader = match paint {
            Paint::Solid(colour) => tiny_skia::Shader::SolidColor(convert::color(*colour)),
            Paint::Shading(shading) => {
                shading::shader(shading, page_to_path, page_to_device, target, scratch)
                    .ok_or_else(|| CpuRasterError::UnsupportedPaint(format!("{shading:?}")))?
            }
            other => return Err(CpuRasterError::UnsupportedPaint(format!("{other:?}"))),
        };
        Ok(tiny_skia::Paint {
            shader,
            blend_mode: blend,
            anti_alias: self.anti_alias,
            force_hq_pipeline: HIGH_PRECISION_PIPELINE,
            ..tiny_skia::Paint::default()
        })
    }
}

impl Default for CpuRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer for CpuRasterizer {
    type Error = CpuRasterError;

    fn name(&self) -> &'static str {
        "cpu"
    }

    fn rasterize(&mut self, list: &DisplayList, target: TargetSpec) -> Result<Raster, Self::Error> {
        // Before the pixmap, because a target may be a gibibyte and an interrupt already raised
        // is an allocation nobody wants the answer to.
        if self.interrupted() {
            return Err(BackendError::Interrupted.into());
        }

        // Checked here rather than assumed, because [`Band`] converts a row index to
        // `f32` and that is lossless only below 2^24. `TargetSpec::for_page` already
        // enforces this, but the struct's fields are public, so a hand-built spec can
        // violate it and would misplace every banded command if it did.
        for extent in [target.width, target.height] {
            if extent > MAX_EXTENT {
                return Err(BackendError::ExtentTooLarge {
                    extent: u64::from(extent),
                    limit: MAX_EXTENT,
                }
                .into());
            }
        }

        let mut pixmap = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
            CpuRasterError::Allocation {
                width: target.width,
                height: target.height,
            },
        )?;
        // Not filled with the background: §11.4.7 makes the page group *isolated*, so the
        // page's elements composite onto transparency and the medium's colour is applied to
        // the result — see `impose_on_medium`, which both backends end with.

        self.encode_in_strips(&mut pixmap, list, target)?;

        // §11.4.7 puts a colour space under the whole page — "[a]ll page-level compositing
        // shall be done in the default blending colour space of the page, and the entire
        // result shall then ... be converted to the native colour space of the output device
        // before being composited with the context-dependent backdrop". Where that space has
        // four components the interpreter hands over a second page whose colours carry the
        // fourth, drawn on the same geometry under the same shapes and opacities, and the two
        // rasters are put back together here — *before* `impose_on_medium`, which is where
        // the clause puts the conversion. See `pdf_render::blending`.
        if let (Some(space), Some(black)) = (list.blending(), list.black()) {
            let mut ink = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
                CpuRasterError::Allocation {
                    width: target.width,
                    height: target.height,
                },
            )?;
            self.encode_in_strips(&mut ink, black, target)?;
            pdf_render::resolve_blending(pixmap.data_mut(), ink.data(), space);
        }

        // §11.4.7's page group is isolated, so the medium's colour is composited with the
        // finished page rather than being the backdrop its blend modes saw. Before the
        // conversion below, because `tiny-skia`'s pixels are premultiplied here and that is
        // where the composite is exact.
        //
        // **Where 𝑊 stops is §14.11.2.1's page boundary and not the target's edge**, which is
        // the whole of `page_area`: a target that *is* its page takes one colour everywhere and
        // is unchanged by the distinction, and a window takes the surround wherever no page
        // lies. See `pdf_render::medium` for which of the two colours the standard states.
        //
        // Both this and the conversion below are per-pixel and independent, so they are run
        // across the same rows the strips used. **They are not an afterthought**: on a
        // 1192×1684 page they were together a third of what was left after the drawing was
        // divided, and a serial third is a 3× ceiling however many strips the page grants.
        // Splitting a per-pixel pass changes no byte, which is why it needs no rule of its
        // own.
        // Per-pixel and independent, so it is split across the same rows the strips used.
        // Splitting a per-pixel pass changes no byte, which is why it needs no rule of its
        // own; it is here because after the drawing is divided a serial pass over every pixel
        // is what is left to bound the speed-up.
        let stride = (target.width as usize).saturating_mul(4).max(4);
        let area = pdf_render::page_area(list, target);
        // §14.11.2.1's clip, where the target is larger than the region the page's contents
        // stop at — a window, which shows ground beside the page and the next page of a column
        // where a page-sized raster showed neither. `None` for every page-sized target, which
        // is what leaves every gate in this tree byte for byte where it was.
        let crop = pdf_render::crop_area(list, target);
        let medium = self.medium;
        pixmap
            .data_mut()
            .par_chunks_mut(stride.saturating_mul(PIXEL_PASS_ROWS))
            .enumerate()
            .for_each(|(chunk, rows)| {
                // Which row of the target this chunk starts at — the one thing a split pass
                // has to know that a whole-target one does not.
                let first =
                    u32::try_from(chunk.saturating_mul(PIXEL_PASS_ROWS)).unwrap_or(u32::MAX);
                // The page's own ink first and the medium under it second: the crop is about
                // what the *page* may show, and running it after the composite would cut the
                // colours a window puts beside the page instead.
                if let Some(crop) = crop {
                    pdf_render::crop_to_page(rows, target.width, first, crop);
                }
                pdf_render::impose_within(rows, target.width, first, area, medium);
            });

        Ok(Raster {
            width: target.width,
            height: target.height,
            format: RasterFormat::Rgba8,
            // `tiny-skia` stores premultiplied alpha internally; `Raster` is documented
            // as straight alpha, so the conversion happens here at the backend boundary
            // rather than being left to every consumer to remember.
            data: pixmap.take_demultiplied(),
        })
    }
}

/// Most horizontal strips one page is divided into, whatever the machine offers.
///
/// A strip carries its own mask cache, its own group buffers and its own soft masks, so the
/// per-strip constants are what bound this rather than the thread count: sixteen strips of a
/// 842-row page are 52 rows apiece, which is shorter than many clip bands. The cores beyond it
/// are not idle in a viewer — pages are rendered on their own threads above this one.
const MAX_STRIPS: u32 = 16;

/// Most work a split may replay, as a multiple of the command list itself.
///
/// See [`pdf_render::replay_ratio`]: a command reaching two strips is built, bounded and
/// pipelined twice, and on a page of a few page-wide commands that duplication is the whole
/// render. A quarter more is what a page of small marks costs and a page of page-wide ones
/// cannot reach.
const MAX_REPLAY: f64 = 1.25;

/// Rows a per-pixel pass over the finished page hands one thread at a time.
///
/// Large enough that a page under this many rows is done on one thread, which is where the
/// scheduling would cost more than the work.
const PIXEL_PASS_ROWS: usize = 64;

/// Fewest rows a strip may have, below which the target is drawn serially.
///
/// A strip pays for a mask cache and a replay of the command list, and a page thumbnail is not
/// worth either.
const MIN_STRIP_ROWS: u32 = 64;

impl CpuRasterizer {
    /// Draws the whole list into `pixmap`, in parallel where the page permits it exactly.
    ///
    /// # Why a strip is not simply a band
    ///
    /// [`Band`] already restricts a command to the rows its clip admits, and cutting the page
    /// into runs of rows and replaying the list into each is the same geometry one level up
    /// (ADR 0137). What makes it a decision rather than a refactor is that **it is only exact
    /// at some rows**: rasterising a path into a surface that does not contain it chops the
    /// path against the surface's edge, and a *curve* chopped at an edge is re-parameterised,
    /// so its coverage differs from the unclipped curve's by up to a quarter of a channel.
    /// ADR 0138 shipped nothing for that reason — four oracle pages stopped agreeing with the
    /// reference consensus — and ADR 0139 is the probe that says exactly where the difference
    /// is: a cut at a row no curve crosses is **bit-identical** to the serial render, and one
    /// at a row a curve crosses is not.
    ///
    /// So [`pdf_render::unsplittable_rows`] names the rows a curve crosses and
    /// [`pdf_render::strip_boundaries_avoiding`] cuts only at the others. A page that grants
    /// none — one wide gradient under one curved clip, `bug1721218_reduced.pdf` — is drawn
    /// serially, which is not a fallback but the only division of it that draws the same page.
    ///
    /// That rule is about *geometry*, and it was not the whole of it: the *arithmetic* had to
    /// be the page's too, which is [`Surface`] and ADR 0219.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`]. A strip that fails takes the render with it: half a page
    /// is not a result.
    fn encode_in_strips(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        list: &DisplayList,
        target: TargetSpec,
    ) -> Result<(), CpuRasterError> {
        let boundaries = plan_strips(list, target, self.strips);
        let strips = boundaries.len().saturating_sub(1);
        if strips < 2 {
            let surface = Surface::whole(target);
            let mut masks = MaskCache::new(surface, self.anti_alias, MASK_BUDGET);
            return self.encode(
                &mut pixmap.as_mut(),
                list,
                list.commands(),
                surface,
                &mut masks,
                0,
                Compose::Over,
            );
        }

        // A strip is a replay of the whole list, so a deeply reduced image is reduced once per
        // strip — work that is per *source* sample and that `pdf_render::replay_ratio` cannot
        // see, because it bounds a replay by the rows a command covers. Reduced here, on this
        // thread, before any strip is queued: the strips then all hit, and this is the one place
        // the reduction may use rayon without being re-entered by a strip (ADR 0731).
        self.reduced_images
            .warm(list.commands(), target.transform, whole_target(target));
        // The budget is divided rather than multiplied: the masks of a strip are a strip tall,
        // so the same total memory buys the same coverage of the page it did serially.
        let budget = MASK_BUDGET.checked_div(strips).unwrap_or(MASK_BUDGET);
        let width = pixmap.width();
        let stride = (width as usize).saturating_mul(4);
        let mut rest = pixmap.data_mut();
        let mut pieces = Vec::with_capacity(strips);
        for pair in boundaries.windows(2) {
            let (top, bottom) = (pair.first().copied().unwrap_or(0), pair.get(1).copied());
            let Some(bottom) = bottom else { break };
            let rows = bottom.saturating_sub(top);
            let (piece, tail) =
                rest.split_at_mut((rows as usize).saturating_mul(stride).min(rest.len()));
            rest = tail;
            pieces.push((top, rows, piece));
        }

        pieces
            .into_par_iter()
            .try_for_each(|(top, rows, piece)| -> Result<(), CpuRasterError> {
                // The page's own target travels into the strip, not a shifted copy of it:
                // where the strip starts is [`Surface`]'s other field, and it reaches the
                // geometry once, last. ADR 0219 is what a shifted copy cost.
                let surface = Surface {
                    page: target,
                    rows: Band { top, height: rows },
                };
                let mut piece = tiny_skia::PixmapMut::from_bytes(piece, width, rows).ok_or(
                    CpuRasterError::Allocation {
                        width,
                        height: rows,
                    },
                )?;
                let mut masks = MaskCache::new(surface, self.anti_alias, budget);
                self.encode(
                    &mut piece,
                    list,
                    list.commands(),
                    surface,
                    &mut masks,
                    0,
                    Compose::Over,
                )
            })
    }

    /// Turns a dash pattern's zero-length dashes into marks, ISO 32000-2 §8.5.3.2.
    ///
    /// > This rule shall apply only to zero-length subpaths of the path being stroked, and
    /// > not to zero-length dashes in a dash pattern of a non-degenerate subpath. In the
    /// > latter case, the line caps shall always be painted, since their orientation is
    /// > determined by the direction of the underlying path
    ///
    /// Errata Collection 3 qualifies that sentence — "In the opaque imaging model, this"
    /// for "This", Issue #103 — and `pdf_render::degenerate`'s module comment carries the
    /// reading; the quotation above is `doc/md/`'s, which is what the quotation gate reads.
    ///
    /// `tiny-skia` paints such a cap, so a dotted line is *drawn* here without this — but it
    /// faces a square cap upright, because Skia's dasher loses the direction and its stroker
    /// says so: "since the zero length segment has no direction, set the orientation to
    /// upright as the default orientation". The clause says the direction is the path's, and
    /// on a diagonal dotted line the two answers cover different pixels. Doing the dashing
    /// here keeps the direction (see [`pdf_render::ZERO_DASH`]) and puts the answer in the
    /// crate both backends share.
    ///
    /// Returns `None` when the pattern holds no zero-length dash whose cap would show, which
    /// leaves the ordinary path untouched: `tiny-skia` does its own dashing and this
    /// allocates nothing.
    fn zero_length_dashes(
        geometry: &Path,
        stroke: &Stroke,
        (width, substitute): (f32, Option<Transform>),
        dots: &mut Path,
    ) -> Option<(Path, f32)> {
        let pattern = pdf_render::dashes_showing_direction(&stroke.dash_array, stroke.cap)?;
        let source = convert::path(geometry)?;
        let dash = tiny_skia::StrokeDash::new(pattern, stroke.dash_phase)?;
        // The resolution scale bounds how finely a curve is measured for arc length.
        // `tiny-skia` derives it from the transform for stroking; one is what it uses for a
        // path already in device-sized units, and a dash's *position* is what this needs
        // rather than the smoothness of its ends.
        let dashed = source.dash(&dash, 1.0)?;
        let split = pdf_render::split_dash_marks(
            &convert::from_skia_path(&dashed),
            stroke.cap,
            width,
            substitute,
        );
        dots.extend(split.dots.commands());
        Some((split.stroked, split.coverage))
    }

    /// Draws a stroked path, including the marks its own geometry has no length to make.
    ///
    /// Split out of [`CpuRasterizer::draw`] because ISO 32000-2 §8.5.3.2 turns one command
    /// into two draws — the subpaths that span a distance are stroked, and the ones that do
    /// not are *filled*, as circles `pdf-render` states rather than as whatever `tiny-skia`'s
    /// caps would have produced.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects or a paint this backend
    /// does not implement.
    #[expect(
        clippy::too_many_arguments,
        reason = "the fields of one display list command, passed through rather than \
                  regrouped: a struct here would be a second spelling of `Command::Stroke`"
    )]
    fn draw_stroke(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        path: &Path,
        transform: Transform,
        stroke: &Stroke,
        paint: &Paint,
        blend: tiny_skia::BlendMode,
        to_device: ToDevice,
        clip: scan::Clip<'_>,
    ) -> Result<(), CpuRasterError> {
        // §8.3.4 NOTE 3: a matrix with no inverse carries this path onto a line or a point, so
        // the *mark* is refused and the rest of the page is drawn. Asked before any geometry is
        // built, because none of it would be used, and asked through `pdf-render` so that all
        // three backends refuse the same marks (trap 2). `pdf_model` reports the count.
        let Some(page_to_path) = pdf_render::paint_space(transform) else {
            return Ok(());
        };
        let at = to_device.of(transform);
        let width = stroke.device_width(at);
        // §8.5.3.2's marks are circles and squares of the line's width, so a width under the
        // device's coverage quantum makes each of them a shape whose *area* is under the square
        // of it — a 0.2-unit dot is 0.03 of a pixel at scale 1 and this rasteriser drew nothing
        // at all for it. §10.7.4 forbids that by name, and `pdf_render::point_mark` is where the
        // substitution is decided, for all three backends at once (trap 2). Handing it the
        // transform is what asks for it; it is withheld on the same question as every other use
        // of the coverage-as-alpha identity.
        let substitute = carries_coverage_as_alpha(self.anti_alias, blend).then_some(at);
        // ISO 32000-2 §8.5.3.2's two rules about a stroke with no length. Neither is
        // Skia's answer: it paints a projecting square cap where the clause asks for
        // no output, it refuses a path that is only a `m` rather than drawing
        // nothing, and it faces a zero-length dash's square cap upright rather than
        // along the path. Both are decided in `pdf-render` so that the two backends
        // cannot answer them differently.
        let split = pdf_render::split_degenerate(path, stroke.cap, width, substitute);
        let geometry = split.as_ref().map_or(path, |s| &s.stroked);
        // §8.4.3.6's "the end cap is painted before the corner" at the vertex a closed subpath's
        // own close makes, decided in `pdf-render` for all three backends (trap 2). Skia's dasher
        // merges the first and last dash of a closed contour whenever both are on, which is
        // §8.4.3.4's answer for a dash the close cuts short and the wrong one for a dash that
        // finishes there.
        let opened = pdf_render::opened_where_a_dash_ends_at_the_close(
            geometry,
            &stroke.dash_array,
            stroke.dash_phase,
        );
        let geometry = opened.as_ref().unwrap_or(geometry);
        let mut dots = split.as_ref().map_or_else(Path::new, |s| s.dots.clone());
        // The two rules make marks of one width under one cap, so they are stated at one
        // coverage; whichever produced marks answers for both.
        let mut coverage = split.as_ref().map_or(1.0, |s| s.coverage);
        let (geometry, dashed) =
            match Self::zero_length_dashes(geometry, stroke, (width, substitute), &mut dots) {
                Some((remainder, dashed_coverage)) => {
                    coverage = dashed_coverage;
                    (remainder, true)
                }
                None => (geometry.clone(), false),
            };

        let mut scratch = None;
        if !geometry.is_empty() {
            let converted = convert::path(&geometry).ok_or(CpuRasterError::InvalidPath)?;
            let mut style = convert::stroke(stroke, at);
            // The dashes have already been dispensed, so what is left is solid.
            if dashed {
                style.dash = None;
            }
            let brush = self.paint(
                paint,
                blend,
                page_to_path,
                to_device.of(Transform::IDENTITY),
                (pixmap.width(), pixmap.height()),
                &mut scratch,
            )?;
            if !draw_sub_pixel_rule(
                pixmap,
                (&geometry, &converted),
                (&style, stroke.cap),
                at,
                &brush,
                clip,
            ) && !draw_long_mitres(
                pixmap,
                (&geometry, &converted),
                (stroke, &style),
                at,
                &brush,
                clip,
            ) && !draw_stroked_outline(pixmap, &converted, &style, at, &brush, clip)
            {
                scan::stroke(
                    pixmap,
                    &converted,
                    &brush,
                    &style,
                    convert::transform(at),
                    clip,
                );
            }
        }
        // The marks are *filled*, not stroked: §8.5.3.2 asks for "a filled circle
        // centred at the single point", and the stroking paint is what fills it.
        if !dots.is_empty()
            && let Some(converted) = convert::path(&dots)
        {
            let mut scratch = None;
            let mut brush = self.paint(
                paint,
                blend,
                page_to_path,
                to_device.of(Transform::IDENTITY),
                (pixmap.width(), pixmap.height()),
                &mut scratch,
            )?;
            // The marks were stated by `pdf-render` above, so this is the area they gave up.
            brush.shader.apply_opacity(coverage);
            scan::fill(
                pixmap,
                &converted,
                &brush,
                tiny_skia::FillRule::Winding,
                convert::transform(at),
                clip,
            );
        }
        Ok(())
    }

    /// Draws a sequence of commands onto a target-sized pixmap.
    ///
    /// Split out of [`CpuRasterizer::rasterize`] because a transparency group is the same
    /// loop over a nested list and a fresh surface, and a second copy of the band and clip
    /// handling would be a place for the two to diverge.
    ///
    /// `depth` counts enclosing groups; see [`MAX_GROUP_DEPTH`] for why it is bounded.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for anything [`CpuRasterizer::draw`] rejects, for a clip
    /// chain that is dangling or cyclic, and for groups nested past the bound.
    #[expect(
        clippy::too_many_arguments,
        reason = "a display list, where it is drawn, and the two things a recursion carries \
                  — how deep it is and how its elements combine. Grouping them would put \
                  the target and the mask cache in a struct that exists once per call"
    )]
    fn encode(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        commands: &[Command],
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        for command in commands {
            // **The one place a draw already in progress can be interrupted**, and it is per
            // command rather than per strip because a strip is not a unit of time: a page whose
            // curves forbid every cut is one strip, and the amplification fixture's ten thousand
            // page-covering fills are one command each. Every recursion — a group, a shaped pair,
            // a soft mask's own list — comes back through this loop, so one check covers all of
            // them. See [`CpuRasterizer::interruptible`] for what it buys and what it cannot
            // reach.
            if self.interrupted() {
                return Err(BackendError::Interrupted.into());
            }

            // A command whose extent misses this surface marks nothing, and saying so here is
            // what makes a strip cost what its own rows cost: without it every strip would
            // build every command's path and compile every command's pipeline, which session
            // 154 measured at 19% of a dense page's rasterisation. A row of margin, for the
            // same reason `Band::covering` takes one — the extent comes from control points
            // and the mask from the path.
            if misses_surface(command, surface) {
                continue;
            }

            // A soft mask is evaluated before anything borrows the cache, because building
            // it renders a whole command list of its own and so needs the cache mutably.
            // Idempotent: the second command under the same mask finds it already there.
            if let Some(id) = command.mask() {
                self.build_soft_mask(list, id, surface, masks, depth)?;
            }

            if let Command::Shaped { object, shape } = command {
                self.encode_shaped(
                    pixmap,
                    list,
                    (object, shape),
                    surface,
                    masks,
                    depth,
                    compose,
                )?;
                continue;
            }

            // A group is the one command that needs the mask cache mutably *while* it
            // draws, so it cannot hold a clip mask borrowed from it across the recursion.
            // It therefore resolves its own clip, twice, either side of its elements.
            if matches!(command, Command::Group { .. }) {
                self.encode_group_command(pixmap, list, command, surface, masks, depth, compose)?;
                continue;
            }

            // Resolved before the match so that every arm shares one code path for
            // clip handling; a per-arm lookup would be a place for them to diverge.
            // The clip admits no row of the target, so nothing this command draws can
            // survive it.
            let Some(Admitted {
                band,
                mask: clip,
                admits,
            }) = masks.effective(list, command.clip(), command.mask())?
            else {
                continue;
            };

            // Everything below draws into the band rather than the page, which is what
            // keeps a command's cost proportional to the pixels its clip can admit.
            // The device map carries the band's first row so that geometry, paints
            // and images all move together; missing one would tear the page apart in a
            // way no metric would notice, so there is exactly one of these.
            let to_device = surface.to_device(band);

            // ISO 32000-2 §11.3.5.3's four modes are computed by this backend rather than
            // by `tiny-skia`, whose three of them are wrong (ADR 0047), so such a command
            // is drawn onto transparency first and composited by `blend::composite`.
            // Drawing onto transparency loses nothing: with αb = 0 the compositing formula
            // collapses to the source, whatever the blend mode.
            if let Some(mode) = compose.non_separable(command.blend()) {
                let mut layer = tiny_skia::Pixmap::new(surface.width(), band.height).ok_or(
                    CpuRasterError::Allocation {
                        width: surface.width(),
                        height: band.height,
                    },
                )?;
                self.draw(
                    &mut layer.as_mut(),
                    command,
                    to_device,
                    (clip, admits),
                    compose,
                )?;
                let mut rows = band
                    .rows(pixmap, surface)
                    .ok_or(CpuRasterError::Allocation {
                        width: surface.width(),
                        height: band.height,
                    })?;
                blend::composite(&mut rows, &layer, mode);
                continue;
            }

            let mut rows = band
                .rows(pixmap, surface)
                .ok_or(CpuRasterError::Allocation {
                    width: surface.width(),
                    height: band.height,
                })?;

            self.draw(&mut rows, command, to_device, (clip, admits), compose)?;
        }
        Ok(())
    }

    /// Unpacks one [`Command::Group`] and hands it to [`CpuRasterizer::draw_group`].
    ///
    /// [`CpuRasterizer::encode`]'s group arm, split out so that the recursion stays
    /// readable at a glance; the argument list is the recursion's own.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`]. A *bare* group under knockout would have to be
    /// composited by its shape and has none stated — a group whose shape is needed arrives
    /// as one half of a [`Command::Shaped`] — so it is refused rather than approximated:
    /// `pdf-model` does not build that display list, and erroring is what keeps the
    /// assumption from becoming a silence if it ever does.
    #[expect(
        clippy::too_many_arguments,
        reason = "one arm of `encode`, carrying exactly what the recursion carries"
    )]
    fn encode_group_command(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        command: &Command,
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        let Command::Group {
            commands,
            alpha,
            blend,
            isolated,
            knockout,
            alpha_is_shape,
            blending,
            ..
        } = command
        else {
            // The one caller matches `Command::Group` first; anything else arriving here
            // is a programming error, and a loud one beats a skipped command.
            return Err(CpuRasterError::UnsupportedCommand(format!(
                "encode_group_command was handed {command:?}"
            )));
        };
        if compose == Compose::Knockout {
            return Err(CpuRasterError::UnsupportedCommand(
                "a group inside a knockout group has a shape this backend cannot \
                 separate from its opacity (ISO 32000-2 §11.4.6)"
                    .to_owned(),
            ));
        }
        self.draw_group(
            pixmap,
            list,
            Group {
                commands,
                alpha: *alpha,
                blend: *blend,
                clip: command.clip(),
                mask: command.mask(),
                isolated: *isolated,
                alpha_is_shape: *alpha_is_shape,
                compose: if *knockout {
                    Compose::Knockout
                } else {
                    Compose::Over
                },
                into: compose,
                blending: blending.as_deref(),
            },
            surface,
            masks,
            depth,
        )
    }

    /// Draws §11.4.6's two stages for an element that states its shape (`Command::Shaped`).
    ///
    /// Empty `1 − shape` of what is under it, then add the object: `P' = (1 − f) × P + S`,
    /// which is the clause's weighted average on the transparent initial backdrop a group is
    /// built on. Outside a knockout group the shape is unused — §11.4.4 reaches it only
    /// through shape × opacity — so the object is drawn alone, which is what
    /// `Command::Shaped` guarantees a backend may do.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`].
    #[expect(
        clippy::too_many_arguments,
        reason = "the two halves of one element, and everything `encode` carries; this is \
                  that recursion split at one arm rather than a call with its own state"
    )]
    fn encode_shaped(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        (object, shape): (&Command, &Command),
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        if compose == Compose::Knockout {
            let shape = std::slice::from_ref(shape);
            self.encode(pixmap, list, shape, surface, masks, depth, Compose::Erase)?;
        }
        let object = std::slice::from_ref(object);
        let after = if compose == Compose::Knockout {
            Compose::Add
        } else {
            compose
        };
        self.encode(pixmap, list, object, surface, masks, depth, after)
    }

    /// Composites one transparency group (ISO 32000-2 §11.4.1).
    ///
    /// The elements are drawn onto a buffer the size of this surface and the result is then
    /// painted onto the page once, under the group's own constant alpha and blend mode.
    /// Compositing the elements one at a time onto the page instead is what §11.6.6's
    /// initialisation of the alpha constants exists to prevent, and is visibly different
    /// wherever two elements overlap.
    ///
    /// The buffer covers the whole surface rather than the group's band because the elements'
    /// clips are resolved against the surface, so their bands are its rows; a band-sized
    /// buffer would need every one of them shifted, and one coordinate system that is right
    /// beats two that have to agree.
    ///
    /// # The two backdrops, and the two ways the buffer comes back
    ///
    /// An **isolated** group starts on transparency — §11.4.5's initial backdrop — and comes
    /// back with source-over. A **non-isolated** one starts as a *copy of the page*, which is
    /// §11.4.4's own model, and comes back as the interpolation `Command::Group`'s `isolated`
    /// derives: `(1 − w) × page + w × buffer`, with `w` the group's constant alpha times its
    /// soft mask at the pixel. That is two draws — Destination-Out by `w` over the band, then
    /// Plus of the buffer at `w` — and the pair is bounded by 1 everywhere, because the two
    /// weights sum to 1 and each operand is a valid premultiplied sample, so `Plus` never
    /// saturates.
    ///
    /// Outside the group's marks the buffer still *is* the page, and the interpolation of a
    /// value with itself is that value, so the copy costs the region nothing.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError::UnsupportedCommand`] for a non-isolated group carrying
    /// anything the collapse above does not hold for; `pdf-model` guarantees it does not
    /// build one, and refusing keeps that from becoming a silence.
    fn draw_group(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        group: Group<'_>,
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<(), CpuRasterError> {
        let depth = depth.saturating_add(1);
        if depth > MAX_GROUP_DEPTH {
            return Err(BackendError::GroupsTooDeep {
                depth,
                limit: MAX_GROUP_DEPTH,
            }
            .into());
        }

        // The band is taken first so that a group whose clip admits no row costs nothing at
        // all — not even the buffer.
        let Some(Admitted { band, .. }) = masks.effective(list, group.clip, group.mask)? else {
            return Ok(());
        };

        // §11.6.6, §11.7.2: a group with a blending colour space of its own is composited
        // in it and converted out at the end — two passes over the same geometry, resolved
        // per pixel, exactly the page-level construction in `rasterize` one scope down.
        // §11.4.6 with `isolated: false` is the other construction with a buffer discipline
        // of its own; everything else is one buffer and one pass.
        let mut buffer = if let Some(pair) = group.blending {
            self.composite_in_own_space(list, pair, &group, surface, masks, depth)?
        } else if !group.isolated && group.compose == Compose::Knockout {
            self.knockout_on_backdrop(pixmap, list, &group, surface, band, masks, depth)?
        } else {
            let mut buffer = initial_backdrop(pixmap, surface, band, &group)?;
            self.encode(
                &mut buffer.as_mut(),
                list,
                group.commands,
                surface,
                masks,
                depth,
                group.compose,
            )?;
            buffer
        };

        // The elements may have evicted the soft mask this group is painted through, since
        // they share the cache; rebuilding it is what makes eviction safe here as it is for
        // a clip.
        if let Some(id) = group.mask {
            self.build_soft_mask(list, id, surface, masks, depth)?;
        }

        // Resolved again rather than held across the recursion: the elements' own clips
        // share this cache and may have evicted the entry, and a rebuilt mask is the mask
        // that was dropped — see `a_rebuilt_mask_is_the_mask_that_was_evicted`.
        let Some(Admitted { mask: clip, .. }) = masks.effective(list, group.clip, group.mask)?
        else {
            return Ok(());
        };

        let paint = tiny_skia::PixmapPaint {
            opacity: group.alpha.clamp(0.0, 1.0),
            // `Compose::mode` reads the group's own blend where the group composites
            // ordinarily, and §11.4.6's operator where it is half of a shaped element —
            // whose blend mode the clause leaves nothing to blend against.
            blend_mode: group.into.mode(group.blend),
            // The buffer is drawn at 1:1 with no transform, so no sample is ever
            // interpolated and the quality setting cannot change a pixel.
            quality: tiny_skia::FilterQuality::Nearest,
        };
        // Negative, because the buffer covers the whole surface and the rows drawn into
        // start at the band's first row. Both are page rows, so the difference is what the
        // buffer has to be shifted by.
        let top = i32::try_from(band.top.saturating_sub(surface.rows.top)).map_err(|_| {
            CpuRasterError::Allocation {
                width: surface.width(),
                height: band.height,
            }
        })?;

        // The mask the blit still owes, which is none where §8.5.4's intersection has already
        // folded it into the buffer — see `group_blit_mask`.
        let blit_mask = group_blit_mask(&mut buffer, &group, (surface, band, top), clip);

        // §11.4.4's own model, for a group whose elements were drawn onto a copy of the
        // page: `(1 − w) × page + w × buffer`, with `w` the group's constant alpha times its
        // soft mask at the pixel. One pass over the band rather than two Porter-Duff draws,
        // for the rounding reason `blend::interpolate` records. The whole band is walked
        // because outside the group's marks the buffer *is* the page and the interpolation
        // has to put it back unchanged — which it does, exactly, and cheaply.
        if !group.isolated {
            let from_row = band.top.saturating_sub(surface.rows.top);
            let mut rows = band
                .rows(pixmap, surface)
                .ok_or(CpuRasterError::Allocation {
                    width: surface.width(),
                    height: band.height,
                })?;
            // A **non-isolated** group's raster is still composited through its clip as a
            // product, and here that is the buffer's own construction rather than a reading
            // of §10.7.4: §8.5.4 does give a group a shape for the clip to intersect — "the
            // shape of a transparency group … shall be influenced … by the [clipping path] in
            // effect at the time the group's results are painted onto its backdrop" — and this
            // buffer started as a copy of the page, so what its alpha holds is the backdrop's
            // unioned with the group's and not that shape. The interpolation buries the
            // group's own shape inside `E(B)` besides, where no factor can reach it.
            // `doc/todo/11` item 4 carries it; ADR 0492 pays the isolated case above.
            blend::interpolate(&mut rows, &buffer, from_row, paint.opacity, clip.mask());
            return Ok(());
        }

        // §11.3.5.3's four modes are this backend's own (ADR 0047), and a group reaches
        // them by the same two steps a command does: the group's clip, alpha and offset
        // are applied onto transparency, and the result is composited by `blend`. The
        // extra buffer is the price of the mode, not of every group.
        if let Some(mode) = group.into.non_separable(group.blend) {
            let mut layer = tiny_skia::Pixmap::new(surface.width(), band.height).ok_or(
                CpuRasterError::Allocation {
                    width: surface.width(),
                    height: band.height,
                },
            )?;
            layer.as_mut().draw_pixmap(
                0,
                top.saturating_neg(),
                buffer.as_ref(),
                &tiny_skia::PixmapPaint {
                    blend_mode: tiny_skia::BlendMode::SourceOver,
                    ..paint
                },
                tiny_skia::Transform::identity(),
                blit_mask,
            );
            let mut rows = band
                .rows(pixmap, surface)
                .ok_or(CpuRasterError::Allocation {
                    width: surface.width(),
                    height: band.height,
                })?;
            blend::composite(&mut rows, &layer, mode);
            return Ok(());
        }

        let mut rows = band
            .rows(pixmap, surface)
            .ok_or(CpuRasterError::Allocation {
                width: surface.width(),
                height: band.height,
            })?;
        rows.draw_pixmap(
            0,
            top.saturating_neg(),
            buffer.as_ref(),
            &paint,
            tiny_skia::Transform::identity(),
            blit_mask,
        );
        Ok(())
    }

    /// Composites a group's elements in the four-component space the group states, and
    /// converts the result out (ISO 32000-2 §11.6.6, §11.7.2).
    ///
    /// §11.7.2: "all blending and compositing computations shall be done in that space",
    /// and "[t]he resulting colours shall then be interpreted in the group's colour space
    /// when the group is subsequently composited with its backdrop". The first sentence is
    /// the two `encode` passes — §11.3.4 composites per component, so four components are
    /// two passes of three, the same construction `rasterize` applies to §11.4.7's page —
    /// and the second is `pdf_render::blending::resolve`, run before the caller paints the
    /// buffer onto the parent. The group is isolated by `pdf_render::GroupBlending`'s own
    /// guarantee, so both passes start on transparency.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::UnsupportedCommand`] for a
    /// non-isolated group carrying a pair, which `pdf-model` never builds.
    fn composite_in_own_space(
        &self,
        list: &DisplayList,
        pair: &pdf_render::GroupBlending,
        group: &Group<'_>,
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<tiny_skia::Pixmap, CpuRasterError> {
        if !group.isolated {
            return Err(CpuRasterError::UnsupportedCommand(
                "a non-isolated group cannot composite in a blending colour space of its \
                 own: §11.6.6 inherits its space from the parent (ISO 32000-2 §11.6.6)"
                    .to_owned(),
            ));
        }
        let allocation = || CpuRasterError::Allocation {
            width: surface.width(),
            height: surface.rows.height,
        };
        let mut chromatic =
            tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or_else(allocation)?;
        self.encode(
            &mut chromatic.as_mut(),
            list,
            group.commands,
            surface,
            masks,
            depth,
            group.compose,
        )?;
        let mut black =
            tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or_else(allocation)?;
        self.encode(
            &mut black.as_mut(),
            list,
            &pair.black,
            surface,
            masks,
            depth,
            group.compose,
        )?;
        pdf_render::resolve_blending(chromatic.data_mut(), black.data(), &pair.space);
        Ok(chromatic)
    }

    /// Accumulates §11.4.6's two stages for a non-isolated knockout group, whose initial
    /// backdrop is the group's own.
    ///
    /// > In a knockout group, each individual element shall be composited with the group's
    /// > initial backdrop rather than with the stack of preceding elements in the group.
    ///
    /// The clause's two stages, per element: a) "[c]omposite the source object with the
    /// group's initial backdrop, disregarding the object's shape and using a source shape
    /// value of 1.0 everywhere", then b) "[c]ompute a weighted average of this result with
    /// the object's immediate backdrop, using the source shape as the weighting factor".
    /// Stage a) is the element drawn onto a scratch copy of the backdrop `B` — its blend
    /// mode, opacity and soft mask all applied, against `B` — and stage b) is
    /// [`blend::knockout_average`], with the element's shape read per pixel off its
    /// [`Command::Shaped`] shape half, which `pdf-model` guarantees every element carries.
    ///
    /// Drawing the object bakes its shape into the scratch where stage a) asks for a shape
    /// of 1.0 everywhere, and the identity `f × E = scratch − (1 − f) × B` (see
    /// [`blend::knockout_average`]) is what takes it back out, so the accumulation is the
    /// clause's arithmetic and not an approximation of it. The buffer starts as `B` — the
    /// accumulation's own initial value — and the caller's non-isolated completion then
    /// interpolates it with the page by the group's alpha, which is the collapse of
    /// §11.4.4's backdrop removal against §11.3.3's recompositing under the Normal blend
    /// function (ADR 0307's formula; ADR 0237's cancellation, unchanged by knockout).
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::UnsupportedCommand`] where a
    /// guarantee `pdf-model` states for the combination does not hold: every element a
    /// [`Command::Shaped`], the group's own blend Normal, composited by Over.
    #[expect(
        clippy::too_many_arguments,
        reason = "one arm of `draw_group`'s buffer construction, carrying what the \
                  recursion carries plus the band the backdrop is copied to"
    )]
    fn knockout_on_backdrop(
        &self,
        pixmap: &tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        group: &Group<'_>,
        surface: Surface,
        band: Band,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<tiny_skia::Pixmap, CpuRasterError> {
        if group.into != Compose::Over || group.blend != pdf_render::BlendMode::Normal {
            return Err(CpuRasterError::UnsupportedCommand(
                "a non-isolated knockout group composited by anything but §11.3.3's Normal \
                 blend function needs Table 140's group alpha kept apart from the composite \
                 alpha (ISO 32000-2 §11.4.4 NOTE 4, §11.4.6)"
                    .to_owned(),
            ));
        }
        let allocation = || CpuRasterError::Allocation {
            width: surface.width(),
            height: surface.rows.height,
        };
        // The group's initial backdrop, retained beside the accumulation: every element
        // composites against *it*, not against what earlier elements left. Copied to the
        // group's band by ADR 0328's own argument, which carries over unchanged: the only
        // reader of this construction's result is the caller's band-rows interpolation, the
        // per-element scratches below are clones of this buffer and so hold the same rows,
        // and no operation moves a value between rows — so a row outside the band, which
        // here is transparency rather than the page, can never reach the page.
        let mut backdrop =
            tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or_else(allocation)?;
        let stride = (surface.width() as usize).saturating_mul(4);
        let start = (band.top.saturating_sub(surface.rows.top) as usize).saturating_mul(stride);
        let end = start.saturating_add((band.height as usize).saturating_mul(stride));
        let source = pixmap
            .as_ref()
            .data()
            .get(start..end)
            .ok_or_else(allocation)?;
        backdrop
            .data_mut()
            .get_mut(start..end)
            .ok_or_else(allocation)?
            .copy_from_slice(source);
        let mut accumulated = backdrop.clone();
        for element in group.commands {
            let Command::Shaped { object, shape } = element else {
                return Err(CpuRasterError::UnsupportedCommand(
                    "an element of a non-isolated knockout group must state its shape \
                     apart from its alpha (ISO 32000-2 §11.4.6)"
                        .to_owned(),
                ));
            };
            // Stage a): the element against the initial backdrop, in a scratch of its own.
            let mut composed = backdrop.clone();
            self.encode(
                &mut composed.as_mut(),
                list,
                std::slice::from_ref(object),
                surface,
                masks,
                depth,
                Compose::Over,
            )?;
            // §11.6.4.2's shape, drawn onto transparency so its alpha *is* the shape.
            let mut stated = tiny_skia::Pixmap::new(surface.width(), surface.rows.height)
                .ok_or_else(allocation)?;
            self.encode(
                &mut stated.as_mut(),
                list,
                std::slice::from_ref(shape),
                surface,
                masks,
                depth,
                Compose::Over,
            )?;
            blend::knockout_average(&mut accumulated, &backdrop, &composed, &stated);
        }
        Ok(accumulated)
    }

    /// Evaluates a soft mask into the cache, if it is not there already (§11.5).
    ///
    /// The mask's group is drawn onto a fully transparent buffer covering this surface — the same
    /// isolated backdrop [`CpuRasterizer::draw_group`] uses, and what both §11.5.2 and
    /// §11.5.3 ask for — and each pixel is then turned into a mask value by
    /// [`pdf_render::SoftMask::value`], which is the function the GPU backend calls on its
    /// own readback. That shared derivation is the whole reason the display list carries a
    /// mask's *commands* rather than a raster: the group is evaluated at device resolution,
    /// which only a backend knows, while what the pixels mean is decided once for both.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::UnknownSoftMask`] for an
    /// identifier this display list does not hold.
    fn build_soft_mask(
        &self,
        list: &DisplayList,
        id: SoftMaskId,
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<(), CpuRasterError> {
        if masks.holds_soft_mask(id) {
            return Ok(());
        }
        let mask = list
            .soft_mask(id)
            .ok_or(CpuRasterError::UnknownSoftMask(id))?;

        let mut buffer = tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or(
            CpuRasterError::Allocation {
                width: surface.width(),
                height: surface.rows.height,
            },
        )?;
        // A soft mask's group is evaluated as §11.4.5's ordinary group: `SoftMask` carries
        // no knockout flag, and `pdf-model` reports a mask group that asks for one.
        self.encode(
            &mut buffer.as_mut(),
            list,
            &mask.commands,
            surface,
            masks,
            depth,
            Compose::Over,
        )?;

        // Straight alpha, which is what `SoftMask::value` is defined over and what the GPU
        // backend reads back; `tiny-skia` stores premultiplied, so the conversion happens
        // here at the same boundary as every other one in this backend.
        //
        // **Per pixel rather than through `Pixmap::take_demultiplied`, and only for a pixel
        // the group marked.** The buffer covers the whole target while a mask group covers
        // its own `/BBox`, so most of it is the transparency it was allocated as — and both
        // halves of the old line ran over all of it: `take_demultiplied` divides three
        // channels by the alpha, and `SoftMask::values` derives a luminosity in floating
        // point. For the transparent pixel both answers are constants: the division gives
        // back the zero it started from, and the derivation gives [`SoftMask::outside`].
        //
        // This is exact, not an approximation — the branch's two arms call the same function
        // on the same pixel — and it is worth stating what it buys, because `CLAUDE.md`
        // requires an optimisation to carry its number: the two slowest documents of a
        // 65 944-document sample of the web spend 22.4 s of 25.4 and 51.0 s of 52.6 on this
        // one line, over rasters that are 98.5% and 99.96% transparent. ADR 0271.
        //
        // **And the conversion runs over the rows the group's marks could reach, not the
        // surface** (`doc/todo/40`, ADR 0328): a row outside `marked_rows`' answer is a row
        // of the buffer nobody wrote, its pixels are the transparency the buffer was
        // allocated as, and the value the pass would derive from every one of them is the
        // `outside` constant the entry now carries instead. The drawing buffer itself
        // deliberately stays surface-sized — the group's elements draw under the very
        // transforms they draw under on the page, which is what keeps this byte-exact where
        // a banded *drawing* would move an edge by a supersample (ADR 0219). Its untouched
        // rows are never read, and an untouched page of a zeroed allocation costs no work.
        // `6081357.pdf` — 912 distinct masks on a 4.3-megapixel page, 99.96% of every raster
        // transparent — went from 81.90 G instructions through `open_one` to **17.02 G**
        // when the pass and the storage stopped being surface-sized (ADR 0328's A/B, two
        // binaries built in one sitting).
        let outside = mask.outside();
        let reach = marked_rows(&mask.commands, surface);
        let width = surface.width() as usize;
        let start = (reach.top.saturating_sub(surface.rows.top) as usize).saturating_mul(width);
        let end = start.saturating_add((reach.height as usize).saturating_mul(width));
        let values: Vec<u8> = buffer
            .pixels()
            .get(start..end)
            .ok_or(CpuRasterError::Allocation {
                width: surface.width(),
                height: reach.height,
            })?
            .iter()
            .map(|pixel| {
                if pixel.alpha() == 0 && pixel.red() == 0 && pixel.green() == 0 && pixel.blue() == 0
                {
                    outside
                } else {
                    let straight = pixel.demultiply();
                    mask.value([
                        straight.red(),
                        straight.green(),
                        straight.blue(),
                        straight.alpha(),
                    ])
                }
            })
            .collect();
        let built = tiny_skia::Mask::from_vec(
            values,
            tiny_skia::IntSize::from_wh(surface.width(), reach.height).ok_or(
                CpuRasterError::Allocation {
                    width: surface.width(),
                    height: reach.height,
                },
            )?,
        )
        .ok_or(CpuRasterError::Allocation {
            width: surface.width(),
            height: reach.height,
        })?;
        masks.admit_soft_mask(id, built, reach, outside);
        Ok(())
    }

    /// Draws `path` with a paint no shader can express, and says whether it did.
    ///
    /// Three of them, and each is a *paint* that becomes pixels rather than a gradient:
    ///
    /// - **A shading stating ISO 32000-2 §8.7.4.3 Table 77's `/Background`** answers a colour
    ///   outside its own bounds, which no spread mode expresses. §11.6.7 makes the wash and the
    ///   shading one painting operation — the pattern's implicit group is "filled with the
    ///   specified background colour before the sh operator is invoked" — so
    ///   [`pdf_render::ShadingRaster`] answers both per device pixel and the shape's own edge
    ///   does the compositing once. It is asked first because it holds for every kind.
    ///
    /// - **A mesh** carries a colour — or §8.7.4.5.5's parametric value — per triangle corner,
    ///   so it is rasterised by [`pdf_render::MeshRaster`] and drawn inside the shape rather
    ///   than as a paint over it.
    /// - **A radial whose two circles do not contain one another** is ISO 32000-2 §8.7.4.5.4's
    ///   cone, where a point can lie on two blend circles and the clause's "greatest value of
    ///   s" decides between them — including the case where the greater root is one `/Extend`
    ///   refuses. No two-point conical gradient expresses that, so the cone is evaluated
    ///   exactly and drawn as a raster; every other radial keeps the native gradient, which
    ///   [`shading::is_a_cone`] proves is enough there.
    ///
    /// Split out of [`CpuRasterizer::draw_fill`] because it is one question — is this paint a
    /// raster? — asked twice, and neither answer shares a line with what follows it there.
    fn filled_as_a_raster(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        (path, fill_rule): (&tiny_skia::Path, pdf_render::FillRule),
        (paint, blend): (&Paint, tiny_skia::BlendMode),
        to_device: ToDevice,
        (transform, clip): (Transform, scan::Clip<'_>),
    ) -> bool {
        let Paint::Shading(shading) = paint else {
            return false;
        };
        let at = convert::transform(to_device.of(transform));
        if shading.background.is_some() {
            shading::fill_with_background(
                pixmap,
                path,
                shading,
                to_device.of(Transform::IDENTITY),
                convert::fill_rule(fill_rule),
                at,
                clip,
                blend,
                self.anti_alias,
            );
            // Answered whether or not a raster came back: an empty one is a shape covering no
            // device pixel, and falling through to a shader that has refused this paint (see
            // `shading::shader`) would turn that into an error on the page.
            return true;
        }
        match shading.kind.as_ref() {
            pdf_render::ShadingKind::Mesh { triangles, ramp } => {
                shading::fill_mesh(
                    pixmap,
                    path,
                    triangles,
                    ramp.as_ref(),
                    to_device.of(shading.transform),
                    convert::fill_rule(fill_rule),
                    at,
                    clip,
                    blend,
                    self.anti_alias,
                );
                true
            }
            pdf_render::ShadingKind::Radial {
                start,
                start_radius,
                end,
                end_radius,
                ramp,
                extend,
            } => {
                shading::is_a_cone(*start, *start_radius, *end, *end_radius)
                    && shading::fill_radial(
                        pixmap,
                        path,
                        pdf_render::Radial {
                            start: *start,
                            start_radius: *start_radius,
                            end: *end,
                            end_radius: *end_radius,
                            ramp,
                            extend: *extend,
                        },
                        to_device.of(shading.transform),
                        convert::fill_rule(fill_rule),
                        at,
                        clip,
                        blend,
                        self.anti_alias,
                    )
            }
            _ => false,
        }
    }

    /// Draws a filled path, including the marks a shape with no area cannot make itself.
    ///
    /// Split out of [`CpuRasterizer::draw`] because ISO 32000-2 §10.7.4 turns one command into
    /// two draws: a subpath with no extent along one axis has zero area, so this rasteriser
    /// computes zero coverage for it at every placement and every scale, and the clause says
    /// no shape may disappear. What it marks instead is `pdf-render`'s geometry rather than
    /// this backend's hairline, so that the two backends cannot answer it differently — and
    /// the marks are filled under the non-zero rule whatever the command's own rule is,
    /// because a mark is a shape in its own right rather than part of the path's winding.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects or a paint this backend
    /// does not implement.
    #[expect(
        clippy::too_many_arguments,
        reason = "the fields of one display list command, passed through rather than \
                  regrouped: a struct here would be a second spelling of `Command::Fill`"
    )]
    fn draw_fill(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        (source, fill_rule): (&Path, pdf_render::FillRule),
        transform: Transform,
        paint: &Paint,
        blend: tiny_skia::BlendMode,
        to_device: ToDevice,
        (clip, admits): (scan::Clip<'_>, Option<tiny_skia::Rect>),
    ) -> Result<(), CpuRasterError> {
        // §8.3.4 NOTE 3: a matrix with no inverse carries this path onto a line or a point, so
        // the *mark* is refused and the rest of the page is drawn — see `pdf_render::paint_space`,
        // which states that for all three backends (trap 2). Before any geometry, because none of
        // it would be used and because the raster route below would otherwise answer separately.
        let Some(page_to_path) = pdf_render::paint_space(transform) else {
            return Ok(());
        };
        let at = to_device.of(transform);
        let cropped = crop_to_mask(source, transform, to_device, admits);
        let source = cropped.as_ref().unwrap_or(source);
        let path = convert::path(source).ok_or(CpuRasterError::InvalidPath)?;

        if self.filled_as_a_raster(
            pixmap,
            (&path, fill_rule),
            (paint, blend),
            to_device,
            (transform, clip),
        ) {
            return Ok(());
        }

        // A sampled shading's pixels are borrowed by its shader, so they need somewhere to
        // live for exactly as long as this call.
        let mut scratch = None;
        let brush = self.paint(
            paint,
            blend,
            page_to_path,
            to_device.of(Transform::IDENTITY),
            (pixmap.width(), pixmap.height()),
            &mut scratch,
        )?;
        let split = pdf_render::split_collapsed_fill(source, at);
        if let Some(split) = &split
            && !split.marks.is_empty()
            && let Some(marks) = convert::path(&split.marks)
        {
            scan::fill(
                pixmap,
                &marks,
                &brush,
                tiny_skia::FillRule::Winding,
                convert::transform(at),
                clip,
            );
        }
        let remaining = match &split {
            // Every subpath collapsed, so the marks are the whole of the drawing.
            Some(split) if split.filled.is_empty() => return Ok(()),
            Some(split) => &split.filled,
            None => source,
        };
        // §10.7.4 again, for a shape this rasteriser's coverage quantum would round to nothing.
        if carries_coverage_as_alpha(brush.anti_alias, brush.blend_mode)
            && let Some(bands) = pdf_render::sub_pixel_bands(remaining, at, fill_rule)
        {
            for band in &bands {
                let shape = convert::path(&band.shape).ok_or(CpuRasterError::InvalidPath)?;
                let mut faint = brush.clone();
                faint.shader.apply_opacity(band.coverage);
                scan::fill(
                    pixmap,
                    &shape,
                    &faint,
                    tiny_skia::FillRule::Winding,
                    convert::transform(at),
                    clip,
                );
            }
            return Ok(());
        }
        let path = match &split {
            Some(split) => convert::path(&split.filled).ok_or(CpuRasterError::InvalidPath)?,
            None => path,
        };
        // §10.7.4 a third time, for the *edge* of a shape thicker than a pixel: an axis-aligned
        // rectangle's coverage is the product of its two overlaps, which `pdf_render::edge`
        // derives from the clause's own definition of a pixel and `scan::fill_rectangles` draws
        // exactly where this converter would round it to a quarter. §11.6.2 is what admits a path
        // stating *several* — they are portions of one object, so they may be drawn one at a time
        // only while no device pixel receives two of them (ADR 0583).
        let exact = rectangular_mark(remaining, at);
        if exact.is_some() {
            scan::fill_rectangles(
                pixmap,
                (&path, &exact),
                &brush,
                convert::transform(at),
                clip,
            );
            return Ok(());
        }
        scan::fill(
            pixmap,
            &path,
            &brush,
            convert::fill_rule(fill_rule),
            convert::transform(at),
            clip,
        );
        Ok(())
    }

    /// Draws one command onto `pixmap`, which is the band its clip admits.
    ///
    /// `to_device` maps page space onto that band, carrying its first row as a whole number
    /// subtracted at the end of every composition; every transform below goes through it and
    /// none reaches the device directly. See [`ToDevice`] for why that order is the property
    /// rather than a detail.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects, for a paint or a
    /// command variant this backend does not implement, or for an inconsistent image.
    fn draw(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        command: &Command,
        to_device: ToDevice,
        (clip, admits): (scan::Clip<'_>, Option<tiny_skia::Rect>),
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        match command {
            Command::Fill {
                path: source,
                transform,
                fill_rule,
                paint,
                blend,
                ..
            } => {
                self.draw_fill(
                    pixmap,
                    (source, *fill_rule),
                    *transform,
                    paint,
                    compose.mode(*blend),
                    to_device,
                    (clip, admits),
                )?;
            }
            Command::Stroke {
                path,
                transform,
                stroke,
                paint,
                blend,
                ..
            } => {
                self.draw_stroke(
                    pixmap,
                    path,
                    *transform,
                    stroke,
                    paint,
                    compose.mode(*blend),
                    to_device,
                    clip,
                )?;
            }
            Command::Image {
                image,
                transform,
                alpha,
                blend,
                ..
            } => {
                let placement = ImagePlacement {
                    transform: *transform,
                    alpha: *alpha,
                    blend: compose.mode(*blend),
                    to_device,
                };
                self.draw_image(pixmap, image, placement, clip)?;
            }
            // `Command` is `#[non_exhaustive]`, so new commands will appear here before
            // this backend implements them. Erroring keeps an unimplemented command
            // visible instead of rendering a page that looks almost right.
            other => {
                return Err(CpuRasterError::UnsupportedCommand(format!("{other:?}")));
            }
        }
        Ok(())
    }
}

/// How an element of a group combines with the elements drawn before it.
///
/// ISO 32000-2 §11.4.6's two answers. The ordinary one is over, and a knockout group's is
/// that "each individual element shall be composited with the group's initial backdrop
/// rather than with the stack of preceding elements in the group" — which, onto the
/// transparent backdrop [`CpuRasterizer::draw_group`] builds, is the element itself
/// replacing what is under it within its own coverage. That is Porter-Duff Source, which
/// `tiny-skia` applies through the same coverage and clip mask as any other mode, so a
/// knockout element needs no second raster and no shape channel.
///
/// An element's *own* blend mode disappears under knockout, and that is the clause rather
/// than a shortcut: its backdrop is transparent, and §11.3.6's formula with αb = 0 is the
/// source colour whatever the blend function is.
///
/// # The two halves of a [`Command::Shaped`] element
///
/// Where the shape is *not* the coverage the display list states it separately, and the
/// clause's two stages become two draws: [`Self::Erase`] scales the accumulated result by
/// `1 − shape`, then [`Self::Add`] adds the element. In premultiplied form that is
/// `P' = (1 − f) × P + S` exactly — see [`Command::Shaped`] — where the one-step
/// [`Self::Knockout`] form is the same line with `f` read off the coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compose {
    /// §11.4.5: an element paints over the group's accumulated result.
    Over,
    /// §11.4.6: an element replaces it within its own coverage.
    Knockout,
    /// §11.4.6's second stage, weighting the immediate backdrop by `1 − shape`.
    Erase,
    /// §11.4.6's second stage, adding the object the first stage composited.
    ///
    /// Addition rather than source-over, and the difference is the whole point of the pair:
    /// source-over would weight the backdrop by `1 − shape × opacity` a second time, which
    /// is right only where the object is opaque or its shape is 0 or 1. What the two draws
    /// leave behind is disjoint — `1 − f` of the backdrop and `f × opacity` of the object —
    /// so the sum cannot exceed 1 and `Plus`'s saturation never engages.
    Add,
}

impl Compose {
    /// The `tiny-skia` mode an element painted under `blend` is drawn with.
    fn mode(self, blend: pdf_render::BlendMode) -> tiny_skia::BlendMode {
        match self {
            Self::Over => convert::blend_mode(blend),
            Self::Knockout => tiny_skia::BlendMode::Source,
            Self::Erase => tiny_skia::BlendMode::DestinationOut,
            Self::Add => tiny_skia::BlendMode::Plus,
        }
    }

    /// Whether §11.3.5.3's four modes still need this backend's own compositing.
    ///
    /// Never under knockout: the element has no backdrop to blend with.
    fn non_separable(self, blend: pdf_render::BlendMode) -> Option<blend::NonSeparable> {
        match self {
            Self::Over => blend::NonSeparable::of(blend),
            Self::Knockout | Self::Erase | Self::Add => None,
        }
    }
}

/// One transparency group, unpacked from its command.
///
/// Grouped for the same reason as [`ImagePlacement`]: five values that always travel
/// together, and a call site that would otherwise be a row of unlabelled arguments.
#[derive(Debug, Clone, Copy)]
struct Group<'a> {
    commands: &'a [Command],
    alpha: f32,
    blend: pdf_render::BlendMode,
    clip: Option<ClipId>,
    mask: Option<SoftMaskId>,
    /// What the elements are composited onto — see `Command::Group`'s `isolated`.
    isolated: bool,
    /// Whether the raster the elements accumulate carries Table 139's shape as well as its
    /// alpha — see `Command::Group`'s `alpha_is_shape`, which is where the argument is.
    alpha_is_shape: bool,
    /// How this group's own elements combine with each other (§11.4.6).
    compose: Compose,
    /// How the finished group combines with what it is drawn onto.
    ///
    /// [`Compose::Over`] everywhere except the two halves of a [`Command::Shaped`], where a
    /// group is one element of a knockout group and §11.4.6's two stages are two draws.
    into: Compose,
    /// The four-component blending colour space the elements composite in, with the same
    /// elements drawn in its black component — see `pdf_render::GroupBlending`.
    blending: Option<&'a pdf_render::GroupBlending>,
}

/// Where and how an image is placed.
///
/// Grouped because these four always travel together, and passing them separately made
/// the call site a row of unlabelled arguments.
#[derive(Debug, Clone, Copy)]
struct ImagePlacement {
    transform: Transform,
    alpha: f32,
    blend: tiny_skia::BlendMode,
    to_device: ToDevice,
}

impl CpuRasterizer {
    /// Draws an image mapped onto the unit square.
    ///
    /// `tiny-skia` has no image primitive, so this fills the unit square with a pattern
    /// shader whose own transform maps image pixels onto it. The composed transform is:
    /// scale by `1/width` and `1/height` to reach the unit square, flip vertically because
    /// PDF's y-up space puts the image's *first* row at the top, then the command's
    /// transform, then the device transform.
    ///
    /// Whether the samples are filtered is [`pdf_render::Image::is_smoothed`]'s decision,
    /// from §8.9.5.3's `/Interpolate` and how large the image is being drawn: bilinear where
    /// a page is viewed at less than full size, which is the common case and where
    /// nearest-neighbour would alias badly, and nearest where the document has magnified a
    /// few samples across an area and asked for no smoothing.
    ///
    /// Deep reductions are handled before that, by [`pdf_render::Image::area_averaged`],
    /// which leaves this filter under a two-fold shrink where four taps see every sample.
    fn draw_image(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        source: &pdf_render::ImageSource,
        placement: ImagePlacement,
        clip: scan::Clip<'_>,
    ) -> Result<(), CpuRasterError> {
        let ImagePlacement {
            transform,
            alpha,
            blend,
            to_device,
        } = placement;
        // ISO 32000-2 §8.3.4 NOTE 3, the same refusal a fill and a stroke take: a matrix with no
        // inverse carries the unit square this image is drawn on onto a line or a point, so the
        // mark is refused and the page is drawn (ADR 0482). Before the samples are resolved,
        // because their grid is derived from a placement that states no scale.
        if pdf_render::paint_space(transform).is_none() {
            return Ok(());
        }
        let placement = to_device.of(transform);
        // Where a command's samples do not exist until the device scale does — §11.6.5.2's
        // soft mask on a grid of its own is the case — this is where they are produced, and
        // `pdf_render` decides the grid so that the three backends cannot ask for different
        // ones. An ordinary image borrows and costs nothing.
        let resolved = source.at(placement);
        let image: &pdf_render::Image = &resolved;
        if !image.is_consistent() {
            return Err(CpuRasterError::InvalidImage {
                width: image.width,
                height: image.height,
                bytes: image.data.len(),
            });
        }

        // Blocks of samples that would share one device pixel are averaged before
        // `tiny-skia` sees them, because its bilinear filter reads four neighbours whatever
        // the reduction and an eleven-fold shrink never looks at most of the source. The
        // decision is `pdf_render`'s so that every backend makes it identically; ADR 0025 has
        // why it is a departure from §10.7.4 rather than a reading of it.
        //
        // Through the memo rather than straight to `area_averaged`, because the work is per
        // *source* sample and this function is called once per strip and once per redraw:
        // `images` has the measurement and the key.
        let reduced = self.reduced_images.reduced(image, placement);
        let image = reduced.as_ref().unwrap_or(image);

        // `tiny-skia` pixmaps are premultiplied; `Image` is documented as straight alpha,
        // so the conversion happens here at the boundary.
        let mut samples = tiny_skia::Pixmap::new(image.width, image.height).ok_or(
            CpuRasterError::Allocation {
                width: image.width,
                height: image.height,
            },
        )?;
        for (target, source) in samples
            .pixels_mut()
            .iter_mut()
            .zip(image.data.chunks_exact(4))
        {
            let a = source[3];
            // `from_rgba` rejects a channel exceeding its alpha, which `premultiply`
            // cannot produce; a fully transparent pixel is the safe fallback if it ever
            // does, since it changes nothing on the page.
            *target = tiny_skia::PremultipliedColorU8::from_rgba(
                premultiply(source[0], a),
                premultiply(source[1], a),
                premultiply(source[2], a),
                a,
            )
            .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
        }

        let width = f32::from(u16::try_from(image.width).unwrap_or(u16::MAX));
        let height = f32::from(u16::try_from(image.height).unwrap_or(u16::MAX));
        // Image space (pixels, y down) to the unit square (y up) — and no further. The
        // pattern is a paint, so `tiny-skia` reads its transform in the space of the path
        // being filled and applies the drawing transform itself; see
        // [`shading::shader`]. That path *is* the unit square, so this is the whole of it,
        // and composing the device transform in here as well would apply it twice.
        let to_unit = Transform::new(1.0 / width, 0.0, 0.0, -1.0 / height, 0.0, 1.0);

        let filter = if image.is_smoothed(placement) {
            tiny_skia::FilterQuality::Bilinear
        } else {
            tiny_skia::FilterQuality::Nearest
        };

        let paint = tiny_skia::Paint {
            shader: tiny_skia::Pattern::new(
                samples.as_ref(),
                tiny_skia::SpreadMode::Pad,
                filter,
                alpha.clamp(0.0, 1.0),
                convert::transform(to_unit),
            ),
            blend_mode: blend,
            anti_alias: self.anti_alias,
            force_hq_pipeline: HIGH_PRECISION_PIPELINE,
            ..tiny_skia::Paint::default()
        };

        // The unit square, which the command's transform carries onto the page.
        let mut builder = tiny_skia::PathBuilder::new();
        builder.push_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).ok_or(
            CpuRasterError::InvalidImage {
                width: 1,
                height: 1,
                bytes: 0,
            },
        )?);
        let square = builder.finish().ok_or(CpuRasterError::InvalidPath)?;

        scan::fill(
            pixmap,
            &square,
            &paint,
            tiny_skia::FillRule::Winding,
            convert::transform(placement),
            clip,
        );
        Ok(())
    }
}

/// Draws a stroke thinner than a device pixel as a shape this rasteriser can measure, ISO 32000-2
/// §10.7.4, and reports whether it did.
///
/// The clause's "no shape ever disappears" applies to strokes as well as fills — "[t]his rule
/// applies both to fill operations and to strokes with non-zero width" — and `tiny-skia` loses a
/// thin one in two ways of its own. Its painter draws a stroke under a pixel wide as a **hairline**
/// with the paint's opacity scaled by the width, which
///
/// - smears the mark symmetrically about the path whatever fraction of a pixel the path lies at,
///   so a rule within half a pixel of the raster's edge loses the half that falls outside — 0.0549
///   of its own 0.1 at the top edge of a 320-unit page, where the graphics device carried 0.0980;
/// - and lays that mark down **one pixel per step along the line's longer device axis**, so a rule
///   at `θ` from the nearer axis carries `cos θ` of its area. Measured by
///   `render-quorra/examples/sub_pixel_marks`: 3.4% short at 15°, 13.4% at 30° and **29.3% at
///   45°**, at every thickness rather than only near the coverage quantum.
///
/// The second reads directly against the sentence three along from the one above — "[t]he area
/// covered by painted pixels shall always be at least as large as the area of the original shape"
/// — so both are answered here rather than left to the library.
///
/// # Two constructions, exact first
///
/// [`draw_rule_as_bands`] is ADR 0226's and is exact: the outline of a straight axis-aligned rule
/// is the rectangle its width and length state, and [`pdf_render::sub_pixel_bands`] draws that at
/// the coverage its own area implies, including the part that is off the raster, which is then
/// clipped away rather than folded back in.
///
/// [`draw_rule_at_one_pixel`] is the general answer and is ADR 0268's: **the same path stroked one
/// device pixel wide, with the width it gave up carried in the paint's alpha.** That conserves the
/// ink exactly at every angle, because widening by a factor and dividing the alpha by it cancel,
/// and it needs no scan converter of our own — which is what the residual `doc/todo/11` carried
/// since ADR 0226 was priced at. It is tried second because it is the blunter of the two: a band
/// one device pixel wide spreads its ink over a pixel where a band of the true width spreads it
/// over `w`, and at the raster's edge it still loses what falls outside.
///
/// Nothing is snapped by either. A 0.1-unit rule draws 0.1 of a row at the fractional position the
/// document put it, which is the sentence `doc/todo/_scan-conversion.md` demands of anything that
/// touches the pixel grid; what §10.7.5 would do under `/SA` is multiply the *ink* by
/// `one_pixel / w` rather than divide the alpha by it.
///
/// # What it still declines
///
/// The whole rule is conditioned on [`carries_coverage_as_alpha`], which is where its warrant
/// lives. Beyond that, only a stroke the stroker itself refuses — and the exact construction
/// declines a great deal more, each case argued in `pdf_render::sub_pixel`'s module comment.
/// A dashed rule is now taken: the dashes are dispensed by `tiny_skia::Path::dash`, which is the
/// same function `stroke_path` would have called, rather than by a second implementation of
/// §8.4.3.6 in this file.
fn draw_sub_pixel_rule(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    geometry: (&Path, &tiny_skia::Path),
    (style, cap): (&tiny_skia::Stroke, pdf_render::LineCap),
    at: Transform,
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let Some(one_pixel) = pdf_render::thinnest_line(at) else {
        return false;
    };
    if !at_or_under_the_quantum(style.width, one_pixel)
        || !carries_coverage_as_alpha(brush.anti_alias, brush.blend_mode)
    {
        return false;
    }
    // The resolution scale bounds how finely a curve is measured while it is offset, and is what
    // `stroke_path` would have used had the hairline test not fired.
    let scale = tiny_skia::PathStroker::compute_resolution_scale(&convert::transform(at));
    // The cheap questions come first: a path of straight axis-aligned rules is one walk over the
    // commands, and only such a path can reach the exact construction at all.
    // **The exact construction stays strictly under the quantum.** A rule *thinner* than a device
    // pixel lies inside one pixel line and `pdf_render::sub_pixel_bands` draws that line at the
    // coverage its area implies; a rule that is exactly one pixel wide has no such line — it
    // spans two of them at a fractional offset — and snapping it onto one would be §10.7.5's
    // automatic stroke adjustment performed without `/SA`, which ADR 0208 forbids and
    // `render-cpu/tests/zero_area_fill.rs` pins by making a `0 w` stroke and a zero-height fill
    // land in *different* places. The general construction below moves nothing.
    if style.width < one_pixel
        && style.dash.is_none()
        && at.preserves_axes()
        && pdf_render::only_flat_subpaths(geometry.0)
        && draw_rule_as_bands(pixmap, geometry.1, style, at, scale, brush, clip)
    {
        return true;
    }
    draw_rule_at_one_pixel(pixmap, (geometry, cap), style, at, scale, brush, clip)
}

/// Whether a mark this wide is one §10.7.4 owes its substitute, at a quantum of `one_pixel`.
///
/// **The boundary is inclusive, and that is a decision rather than a reading.** §10.7.4 states
/// the rule and one exemption:
///
/// > The area covered by painted pixels shall always be at least as large as the area of the
/// > original shape. This rule applies both to fill operations and to strokes with non-zero
/// > width. Zero-width strokes may be done in an implementation-defined manner that may include
/// > fewer pixels than the rule implies.
///
/// For a stroke the document gave a width the `shall` is plain, and `tiny-skia`'s hairline —
/// which it chooses for every width up to and including one device pixel — carries `cos θ` of
/// the rule's area, 29.3% short at 45°. That is the whole reason this boundary moved from `<` to
/// `<=`: at exactly one device pixel a `1 w` rule was being drawn short of its own area on every
/// technical drawing in the corpus, and the discontinuity was `tiny-skia`'s `<=` rather than
/// anything derived.
///
/// **A `0 w` rule follows it, and the clause permits either.** §10.7.4's exemption is a `may`, so
/// the hairline is allowed; §8.4.3.2 says "[a] line width of 0 shall denote the thinnest line
/// that can be rendered at device resolution: 1 device pixel wide", and a staircase one pixel
/// wide *along an axis* is thinner than that measured across the line. Neither reading is forced.
/// What decides it here is this project's own rule about two backends: `pdf_render::Stroke::
/// device_width` resolves a zero width to one device pixel **in the shared crate**, so that both
/// backends draw one mark, and quorra strokes exactly that. Leaving the hairline in place would
/// be `render-cpu` privately re-deciding what `pdf-render` had decided, and the two backends
/// would disagree by 29% on every turned `0 w` line with no clause to arbitrate. No corpus
/// document ranks the choice: the whole gate is identical either way, measured.
///
/// `<=` rather than an equality test on purpose: a page transform of 1.0037 leaves the two sides
/// near rather than equal, and a rule that only fired on the exact float would be a rule about
/// one scale.
fn at_or_under_the_quantum(width: f32, one_pixel: f32) -> bool {
    width <= one_pixel
}

/// ADR 0226's exact construction: the rule's own outline, at the coverage its area implies.
///
/// See [`draw_sub_pixel_rule`], whose first choice this is. Returns `false` — leaving the caller
/// to the construction below — for an outline [`pdf_render::sub_pixel_bands`] cannot measure
/// exactly, which is every case its module comment argues: a round cap's arc, two rules meeting in
/// one pixel row, a subpath that is not a rectangle.
fn draw_rule_as_bands(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    style: &tiny_skia::Stroke,
    at: Transform,
    scale: f32,
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let Some(outline) = path.stroke(style, scale) else {
        return false;
    };
    let Some(bands) = pdf_render::sub_pixel_bands(
        &convert::from_skia_path(&outline),
        at,
        pdf_render::FillRule::NonZero,
    ) else {
        return false;
    };
    // Converted before anything is drawn, so a shape the rasteriser rejects leaves the whole rule
    // to the construction below rather than drawing half of it twice.
    let Some(shapes) = bands
        .iter()
        .map(|band| Some((convert::path(&band.shape)?, band.coverage)))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    for (shape, coverage) in &shapes {
        let mut faint = brush.clone();
        faint.shader.apply_opacity(*coverage);
        scan::fill(
            pixmap,
            shape,
            &faint,
            tiny_skia::FillRule::Winding,
            convert::transform(at),
            clip,
        );
    }
    true
}

/// ADR 0268's general construction: the rule stroked one device pixel wide, at the alpha its own
/// width implies, with §8.4.3.3's cap as a second mark at the alpha *its* own area implies.
///
/// See [`draw_sub_pixel_rule`] for why this is owed and [`pdf_render::substitute_width`] for why
/// the width is stated from the transform's *smaller* stretch. The two moves are one arithmetic
/// identity: the substitute's device area is `width / style.width` times the rule's, and the alpha
/// is `style.width / width`, so the ink is the rule's own area whatever the transform and whatever
/// the angle.
///
/// The dashes are dispensed first, by the same `tiny_skia::Path::dash` that `stroke_path` would
/// have called, because widening a dashed stroke must not widen its dashes: §8.4.3.6's pattern is
/// measured along the path and has nothing to do with the width. The caps are taken from the
/// dashed path for the same clause's sake: §8.4.3.3 states the cap style "shall be used at both
/// ends of open subpaths (and dashes 8.4.3.6, "Line dash pattern") when they are stroked".
fn draw_rule_at_one_pixel(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    (geometry, cap): ((&Path, &tiny_skia::Path), pdf_render::LineCap),
    style: &tiny_skia::Stroke,
    at: Transform,
    scale: f32,
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let Some(width) = pdf_render::substitute_width(at) else {
        return false;
    };
    // A width above the substitute's is not this rule's business: `draw_sub_pixel_rule` has
    // already established that the stroke is at or under one device pixel by §8.4.3.2's reading,
    // and under an anisotropic transform that leaves the two readings a factor apart.
    if !at_or_under_the_quantum(style.width, width) {
        return false;
    }
    let dashed;
    // The caps are stated from the *path's* own commands, and where the dasher has cut new ends
    // they have to be read back from what it produced — which is the one case that pays for a
    // conversion. An undashed rule already has its geometry in both forms.
    let mut cut_ends = None;
    let path = match style.dash.as_ref() {
        None => geometry.1,
        Some(dash) => {
            let Some(cut) = geometry.1.dash(dash, scale) else {
                return false;
            };
            dashed = cut;
            if cap != pdf_render::LineCap::Butt {
                cut_ends = Some(convert::from_skia_path(&dashed));
            }
            &dashed
        }
    };
    let mut widened = style.clone();
    widened.width = width;
    widened.dash = None;
    // The identity above is the *body's*: a cap's area goes as the square of the width, so
    // widening multiplies it by `(width / style.width)²` where the alpha divides it back only
    // once, and the end would be overstated by that factor — which is why the widened stroke is
    // butt-capped and the cap is a mark of its own below, at the alpha its own square implies.
    // **Except where nothing was widened.** The factor above is `width / style.width`, and at
    // the quantum itself it is exactly 1: a rule already one device pixel wide overstates its own
    // cap by nothing, so the stroker's own cap is exact and there is nothing to state twice.
    let widening = style.width < width;
    if widening {
        widened.line_cap = tiny_skia::LineCap::Butt;
    }
    let Some(outline) = path.stroke(&widened, scale) else {
        return false;
    };
    let mut faint = brush.clone();
    // `width` is a reciprocal of a positive stretch, so it is finite and above zero. The floor is
    // §10.7.4's "no shape ever disappears" reaching the *alpha* the substitute rides in, which is
    // eight bits and runs out below 1/255 of a device pixel of thickness.
    faint
        .shader
        .apply_opacity(pdf_render::expressible_coverage(style.width / width));
    scan::fill(
        pixmap,
        &outline,
        &faint,
        tiny_skia::FillRule::Winding,
        convert::transform(at),
        clip,
    );
    // A butt cap projects nothing, which is the overwhelmingly common case and the one that must
    // not pay for a path conversion to be told so.
    if widening
        && cap != pdf_render::LineCap::Butt
        && let Some(mark) = pdf_render::enlarged_mark(style.width, at)
        && let Some(caps) =
            pdf_render::sub_pixel_caps(cut_ends.as_ref().unwrap_or(geometry.0), cap, mark)
        && let Some(converted) = convert::path(&caps)
    {
        let mut faint = brush.clone();
        faint.shader.apply_opacity(mark.coverage);
        scan::fill(
            pixmap,
            &converted,
            &faint,
            tiny_skia::FillRule::Winding,
            convert::transform(at),
            clip,
        );
    }
    true
}

/// Which device rectangles a fill's path is, if any — ISO 32000-2 §10.7.4 and §11.6.2.
///
/// One walk answers both questions: `pdf_render::device_rectangles` returns its one-rectangle
/// variant without allocating, which is the great majority of every corpus here
/// (`pdf-model/examples/rectangular_path_census`), and a `Vec` only for the paths that state
/// several. Asking two functions instead cost +0.18% of the rasteriser on a page of text, which is
/// the second walk over every fill the first one declined.
///
/// **The second half is `DeviceRectangles::share_a_device_pixel`**: §11.6.2 makes a path's subpaths
/// portions of one object and forbids compositing portions with one another, so a pixel reached by
/// two of them may not receive two compositing steps. That question decides which of the two
/// multi-rectangle variants the mark is rather than whether it is one at all — the portions'
/// coverages are summed into one buffer and blitted once (ADR 0590) where they may not be drawn
/// separately. Both questions live in the shared crate because both are decisions about the device,
/// which is trap 2's rule and [`scan::Exact`]'s own comment. ADR 0583.
fn rectangular_mark(path: &Path, at: Transform) -> scan::Exact {
    let Some(rectangles) = pdf_render::device_rectangles(path, at) else {
        return scan::Exact::Unknown;
    };
    let shared = rectangles.share_a_device_pixel();
    match rectangles {
        pdf_render::DeviceRectangles::One(rect) => {
            convert::to_skia_rect(rect).map_or(scan::Exact::Unknown, scan::Exact::One)
        }
        pdf_render::DeviceRectangles::Several(rects) => rects
            .into_iter()
            .map(convert::to_skia_rect)
            .collect::<Option<Vec<_>>>()
            .map_or(scan::Exact::Unknown, |rects| {
                if shared {
                    scan::Exact::Shared(rects)
                } else {
                    scan::Exact::Several(rects)
                }
            }),
    }
}

/// The verbs an axis-aligned rectangle's outline can take, above which [`draw_stroked_outline`]
/// stops asking whether it is one.
///
/// A **cost guard and not a condition**: `pdf_render::device_rectangle` decides whether a shape is
/// a rectangle, and this only decides whether asking is worth a path conversion. `tiny-skia`'s
/// stroker closes a butt-capped straight rule with a move, three lines and a close — five verbs —
/// and `the_outline_of_a_straight_rule_is_a_rectangle` pins that count, so the margin here is for a
/// stroker that spells the same rectangle a verb or two differently rather than for a shape with
/// more corners.
const RECTANGULAR_OUTLINE_VERBS: usize = 8;

/// ISO 32000-2 §8.4.3's stroked outline, filled — which is the shape §10.7.4's clipping paragraph
/// can meet as a *set* where a hairline and a library-internal fill cannot.
///
/// Returns `false` for a stroke at or under the coverage quantum, which is
/// [`draw_sub_pixel_rule`]'s, and for a path the stroker or the dasher refuses.
///
/// # Why a stroke needs this
///
/// §10.7.4 states the clip as a set of pixels intersected with "the set of pixels for the region
/// to be painted", and §8.5.4 as the intersection of the clipping path with "the object's
/// intrinsic shape". Neither sentence knows which operator painted the region. This backend did:
/// a *fill* has met its clip by `min` since ADR 0355, and a stroke went through
/// `tiny_skia::PixmapMut::stroke_path`, which hands the finished mask to its own `fill_path` and
/// multiplies. A boundary pixel covered `c` by the mark and `c` by a coincident clip was painted
/// `c²` — `crates/pdf-model/examples/coincident_edge_probe` is the ladder and
/// `render-cpu/tests/clip_intersection.rs` is the identity.
///
/// # Why it is not a second stroker
///
/// `scan::stroke`'s comment priced this as "choosing between duplicating the library's stroker and
/// contradicting its hairline", and the first half was wrong in ADR 0476's direction: **the
/// library already contains the pieces**. `tiny_skia::Path::stroke` is the same
/// `PathStroker` `stroke_path` calls, `tiny_skia::Path::dash` the same dasher, and `stroke_path`'s
/// own non-hairline branch is exactly the two of them followed by a `fill_path` under the non-zero
/// rule. So the mark drawn here is the mark the library drew, reached one call earlier so that
/// `scan::fill` can compose it — the same move `draw_long_mitres` already makes for the paths
/// §8.4.3.5 takes out of the stroker's hands.
///
/// # The second half: the hairline boundary was the *library's*
///
/// The remaining half of that sentence — contradicting the hairline — is answered by moving the
/// boundary rather than by crossing it. `tiny-skia` decides between a hairline and an outline with
/// `treat_as_hairline`, which maps the width along each of the transform's two basis vectors and
/// compares an approximate length against 1; `pdf_render::thinnest_line` is a singular value and is
/// exact, and the two agree for every similarity transform and part by up to a factor of `√2`
/// under a shear. A boundary either backend can be given by its own library is a boundary neither
/// backend chose (trap 2), so it is `pdf-render`'s here: at or under one device pixel §10.7.4's
/// substitutions own the mark, and above it the stroke's own outline does. The library's hairline
/// is then reached only where [`carries_coverage_as_alpha`] has already withdrawn every
/// substitution this module makes.
///
/// # And what falls out of it, because a stroke's mark is now a fill's
///
/// A butt-capped straight rule along a device axis has an outline that is *one axis-aligned
/// rectangle*, so ADR 0476's exact coverage — the product of a pixel's two overlaps, from
/// §10.7.4's own definition of a pixel — applies to it and did not before. That is the same rule
/// the clip region beside it is already measured by, which is what the subclause asks for in so
/// many words: the region "consists of the set of pixels that would be included by a fill
/// operation". `a_stroke_and_the_fill_of_its_outline_are_one_mark` is the scene, and it read three
/// levels apart before this.
fn draw_stroked_outline(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    style: &tiny_skia::Stroke,
    at: Transform,
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let Some(one_pixel) = pdf_render::thinnest_line(at) else {
        return false;
    };
    if at_or_under_the_quantum(style.width, one_pixel) {
        return false;
    }
    let scale = tiny_skia::PathStroker::compute_resolution_scale(&convert::transform(at));
    // §8.4.3.6's pattern is measured along the path and the stroker knows nothing about it, which
    // is the order `stroke_path` dispenses them in too.
    let dashed;
    let path = match style.dash.as_ref() {
        None => path,
        Some(dash) => {
            let Some(cut) = path.dash(dash, scale) else {
                return false;
            };
            dashed = cut;
            &dashed
        }
    };
    let mut solid = style.clone();
    solid.dash = None;
    let Some(outline) = path.stroke(&solid, scale) else {
        return false;
    };
    let at_device = convert::transform(at);
    if outline.len() <= RECTANGULAR_OUTLINE_VERBS
        && let Some(rect) = pdf_render::device_rectangle(&convert::from_skia_path(&outline), at)
        && let Some(rect) = convert::to_skia_rect(rect)
    {
        scan::fill_rectangles(
            pixmap,
            (&outline, &scan::Exact::One(rect)),
            brush,
            at_device,
            clip,
        );
        return true;
    }
    // The non-zero rule, because a stroked outline's inner contours are wound against its outer
    // ones and the even-odd rule would hollow a self-overlapping stroke out. It is what
    // `stroke_path` fills the same outline with.
    scan::fill(
        pixmap,
        &outline,
        brush,
        tiny_skia::FillRule::Winding,
        at_device,
        clip,
    );
    true
}

/// The mitre-length ratio above which this library's stroker draws a bevel whatever `M` says.
///
/// `tiny-skia`'s stroker classifies a join before it consults the limit: `dot_to_angle_type` calls
/// a normals' dot product within `SCALAR_NEARLY_ZERO` — `1/4096` — of −1 `Nearly180` and sends it
/// to `do_blunt_or_clipped`. ISO 32000-2 §8.4.3.5's ratio is `1 / sin(φ/2)` and the library's own
/// comment gives `sin(φ/2) = sqrt((1 + dot) / 2)`, so that angle test is the ratio
/// `1 / sqrt((1/4096) / 2)` = 90.51 in disguise, and every sharper join is bevelled with the file's
/// limit unread.
///
/// **Measured as well as derived**, by `render-quorra/examples/mitre_ladder`: this backend draws
/// the mitre at a ratio of 90.23 and nothing at all at 95.50, where the two graphics-device
/// backends draw the tip within a pixel of the clause's arithmetic at every rung.
const BEVELLED_BY_THE_STROKER: f32 = 90.51;

/// ISO 32000-2 §8.4.3.5's mitre, where [`BEVELLED_BY_THE_STROKER`] says the library will not draw
/// one: the outline bevelled by the stroker with `pdf-render`'s own mitres filled into it.
///
/// Returns `false` — leaving the caller's ordinary stroke to run — for every path that has no such
/// join, which is every path in the corpus bar the ones `long_mitre_census` counts. The condition
/// costs one comparison for an ordinary stroke: a join sharper than the ratio above is only drawn
/// where the file's own limit admits it, so a stroke whose `M` is under that ratio cannot have one
/// and `pdf_render::mitre_wedges` says so without walking the path.
///
/// # Why the two shapes are drawn as one path
///
/// A mitre join is a bevel join plus one triangle per join, and the triangle's base is the bevel's
/// own outer edge (`pdf_render::mitre`). Two draws of two shapes sharing an edge composite by
/// §11.3.7.3's union function and leave a seam along it — `doc/todo/11` item 5 — where coverage
/// accumulated inside one scan conversion adds. So the stroker's outline and the wedges go into one
/// `tiny_skia::Path` and are filled once, under the non-zero rule, which is what
/// `tiny_skia::PixmapMut::stroke_path` does with an outline anyway.
///
/// # What it declines
///
/// A stroke at or under one device pixel, where the library draws a hairline that has no joins at
/// all and where §10.7.4's own substitutions ([`draw_sub_pixel_rule`]) own the geometry — a wedge
/// under the coverage quantum is the case those two ADRs argue, not this one. And a path the
/// stroker itself refuses, or one whose wedges do not convert, both of which leave the ordinary
/// draw to report in its own way.
fn draw_long_mitres(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    geometry: (&Path, &tiny_skia::Path),
    (stroke, style): (&Stroke, &tiny_skia::Stroke),
    at: Transform,
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    // Asked first, and it answers a stroke whose stated limit cannot admit such a join without
    // walking the path — which is every stroke in the corpus bar eight. The width is the stroke's
    // in the path's own space, which is where §8.4.3.2 states it and where the stroker offsets;
    // `convert::stroke` puts the same number in `style.width`.
    let Some(wedges) = pdf_render::mitre_wedges(
        geometry.0,
        stroke,
        style.width / 2.0,
        BEVELLED_BY_THE_STROKER,
    ) else {
        return false;
    };
    if pdf_render::thinnest_line(at).is_some_and(|one_pixel| style.width <= one_pixel) {
        return false;
    }
    let Some(wedges) = convert::path(&wedges) else {
        return false;
    };
    let mut bevelled = style.clone();
    bevelled.line_join = tiny_skia::LineJoin::Bevel;
    let scale = tiny_skia::PathStroker::compute_resolution_scale(&convert::transform(at));
    let Some(outline) = geometry.1.stroke(&bevelled, scale) else {
        return false;
    };
    let mut builder = tiny_skia::PathBuilder::new();
    builder.push_path(&outline);
    builder.push_path(&wedges);
    let Some(combined) = builder.finish() else {
        return false;
    };
    scan::fill(
        pixmap,
        &combined,
        brush,
        tiny_skia::FillRule::Winding,
        convert::transform(at),
        clip,
    );
    true
}

/// Whether painting a shape at alpha `c` says the same thing as painting it at coverage `c`.
///
/// **The first condition is that this rasteriser is anti-aliasing at all.** The substitution's
/// whole warrant is the departure §10.7.4's row records — the clause's "paint the pixel" replaced
/// by coverage proportional to area — so where [`CpuRasterizer::with_anti_alias`] has turned that
/// departure off, the clause's own answer for a sub-pixel shape is a *whole* covered pixel and a
/// fractional alpha is neither that nor what the rasteriser was asked for. The aliased mode is
/// therefore left exactly as it was. It is one test's knob today and this costs it nothing; what
/// it buys is that the rule below is true as stated rather than true of one configuration.
///
/// [`pdf_render::sub_pixel_bands`] hands back a coverage this rasteriser cannot express as a
/// shape, and the only place left to put it is the paint's alpha. The standard says when the two
/// are one quantity, ISO 32000-2 §11.3.7.1:
///
/// > As stated earlier, the alpha values that control the compositing process shall be defined as
/// > the product of shape and opacity
///
/// and §11.3.7.2's NOTE 1 says that anti-aliased coverage is the first of the two factors:
///
/// > Mathematically, elementary objects have "hard" edges, with a shape value of either 0.0 or
/// > 1.0 at every point. However, when such objects are rasterized to device pixels, the shape
/// > values along the boundaries can be anti-aliased, taking on fractional values representing
/// > fractional coverage of those pixels. When such anti-aliasing is performed, it is important
/// > to treat the fractional coverage as shape rather than opacity.
///
/// A raster that carries one alpha channel per pixel carries exactly that product, so folding a
/// band's coverage into it is the clause's own arithmetic rather than a trick — every blend
/// function reads `αs` and none of them can tell which factor it came from.
///
/// **The NOTE's warning is the exception, and it is why this returns `false` for one mode.**
/// Where shape has to stay distinguishable from opacity the substitution is wrong, and this
/// backend has one such place: §11.4.6's knockout group, where an element replaces its backdrop
/// *within its own shape* and `tiny-skia` states that as Porter-Duff Source. There a scaled alpha
/// leaves a partly transparent pixel where a partly covered one is meant. A sub-pixel shape inside
/// a knockout group therefore keeps whatever the rasteriser already gave it. ADR 0226.
///
/// Asked of the two fields rather than of a paint, because §8.5.3.2's marks are decided before
/// there is a paint to ask: `draw_stroke` states a dot's *diameter* while building the geometry,
/// and the alpha it will be filled at is the same question one step later.
pub(crate) fn carries_coverage_as_alpha(anti_alias: bool, blend: tiny_skia::BlendMode) -> bool {
    anti_alias && blend != tiny_skia::BlendMode::Source
}

/// Multiplies a straight-alpha channel by its alpha.
fn premultiply(value: u8, alpha: u8) -> u8 {
    // Rounded rather than truncated, so a fully opaque pixel round-trips exactly.
    let scaled = u16::from(value)
        .saturating_mul(u16::from(alpha))
        .saturating_add(127);
    u8::try_from(scaled / 255).unwrap_or(u8::MAX)
}

/// A contiguous run of **page** rows: the only rows a command is allowed to mark.
///
/// A command can only change pixels its clip admits, and a clip usually admits very
/// little of the page. Restricting the drawing surface to those rows is what keeps a
/// command's cost proportional to what it can actually change, and it is not a micro
/// optimisation: on `bug1721218_reduced.pdf` (3576 distinct clips, each covering a
/// mean 1.2% of the page's height) `callgrind` attributed **80% of the whole render**
/// to the raster pipeline's gradient stage, evaluating shadings across the full page
/// only for the clip to discard almost all of it. Bounding the surface removes that
/// work rather than masking it away afterwards.
///
/// Rows and not a rectangle, because a pixmap's rows are contiguous in memory and its
/// columns are not: `tiny_skia::PixmapMut::from_bytes` can borrow a band of a pixmap,
/// and `tiny-skia` exposes no sub-rectangle view. The same constraint applies to the
/// clip mask, which must share the pixmap's row stride, so it is band-tall and
/// page-wide too.
///
/// **Page rows, and a [`Surface`]'s own rows are one of these too.** A strip of a page and
/// the band a clip admits are the same idea at two scales, and counting both from the page's
/// first row is what lets the offset from page space reach the geometry once (ADR 0219).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Band {
    /// First page row this band covers.
    top: u32,
    /// Number of rows.
    height: u32,
}

impl Band {
    /// The band covering `bounds`, clipped to the rows `surface` holds.
    ///
    /// Returns `None` when `bounds` covers no row of the surface.
    ///
    /// `bounds` is widened by a row before rounding outward. Clip bounds are computed
    /// from a path's control points and then transformed, while the mask is built by
    /// transforming the path itself; the two agree to within floating-point rounding
    /// rather than exactly, and a band one row short would erase a row of a shading
    /// that the clip admits. A spare row costs a fraction of a percent of the band and
    /// removes that class of defect entirely.
    fn covering(bounds: tiny_skia::Rect, surface: Surface) -> Option<Self> {
        let rows = bounds.outset(0.0, 1.0)?.round_out()?;
        let first = i32::try_from(surface.rows.top).ok()?;
        let limit = first.checked_add(i32::try_from(surface.rows.height).ok()?)?;
        let top = rows.top().clamp(first, limit);
        let bottom = rows.bottom().clamp(first, limit);
        let rows = u32::try_from(bottom.checked_sub(top)?).ok()?;
        (rows > 0).then_some(Self {
            top: u32::try_from(top).ok()?,
            height: rows,
        })
    }

    /// Borrows this band's rows of `pixmap`, which covers `surface`, as a pixmap of its own.
    ///
    /// `None` only if the band does not lie within the surface, which [`Band::covering`]
    /// does not produce.
    fn rows<'a>(
        self,
        pixmap: &'a mut tiny_skia::PixmapMut<'_>,
        surface: Surface,
    ) -> Option<tiny_skia::PixmapMut<'a>> {
        let width = pixmap.width();
        let stride = (width as usize).checked_mul(4)?;
        let start = (self.top.checked_sub(surface.rows.top)? as usize).checked_mul(stride)?;
        let end = start.checked_add((self.height as usize).checked_mul(stride)?)?;
        let rows = pixmap.data_mut().get_mut(start..end)?;
        tiny_skia::PixmapMut::from_bytes(rows, width, self.height)
    }

    /// Bytes a mask covering this band of a `width`-wide target occupies.
    fn mask_bytes(self, width: u32) -> usize {
        (self.height as usize).saturating_mul(width as usize)
    }
}

/// The rows of a page one call draws into, and the page they belong to.
///
/// # Why the page's transform travels and a strip's does not
///
/// A strip is a run of the page's rows, and the obvious way to hand one to a rasteriser is to
/// give it a target whose transform has been shifted up by the rows above it. That is what
/// this backend did from ADR 0139 until session 382, and **it made the picture depend on how
/// the page was divided**: `Transform::then` folds the shift into the page transform's `f`, so
/// a mark's own translation is then added to a number of a different magnitude and the sum
/// rounds elsewhere. One `ulp` of the composed matrix is one supersample of a glyph's edge,
/// which `tiny-skia` quantises to 16 of 255 — the pixel ADR 0219 was written for.
///
/// So the page's own target is what travels, and where the surface starts is a separate whole
/// number of rows, applied once and last by [`ToDevice`]. Every band, clip mask and group
/// buffer below is measured in page rows for the same reason.
#[derive(Debug, Clone, Copy)]
struct Surface {
    /// The whole page's target: `transform` maps page space onto the *page's* device grid,
    /// however few of its rows this surface holds.
    page: TargetSpec,
    /// The page rows it holds.
    rows: Band,
}

impl Surface {
    /// The surface covering a whole page.
    fn whole(page: TargetSpec) -> Self {
        Self {
            page,
            rows: Band {
                top: 0,
                height: page.height,
            },
        }
    }

    /// Pixels across, which is the page's width: a surface is a run of whole rows.
    fn width(self) -> u32 {
        self.page.width
    }

    /// The map onto the device grid of `band`, which must be rows this surface holds.
    fn to_device(self, band: Band) -> ToDevice {
        ToDevice {
            page: self.page.transform,
            top: band.top,
        }
    }
}

/// Page space onto the device grid of a surface starting at page row `top`.
///
/// Two values rather than one matrix, and that is the whole of ADR 0219: the row offset is
/// composed **last**, onto the fully composed matrix, where subtracting a whole number of rows
/// from a device coordinate is exact. Folded in first — as `target.transform.then(offset)` —
/// it changes the magnitude every later composition rounds at, and a page drawn in strips stops
/// being the page drawn whole.
#[derive(Debug, Clone, Copy)]
struct ToDevice {
    /// Page space to the whole page's device grid.
    page: Transform,
    /// First page row of the surface being drawn into.
    top: u32,
}

impl ToDevice {
    /// The matrix geometry stated in `transform`'s space is drawn under.
    fn of(self, transform: Transform) -> Transform {
        #[expect(
            clippy::cast_precision_loss,
            reason = "rasterize rejects a target taller than MAX_EXTENT = 2^24, and every \
                      integer below that is exactly representable as f32"
        )]
        let rows = self.top as f32;
        transform
            .then(self.page)
            .then(Transform::translate(0.0, -rows))
    }
}

/// The rectangle a fill should be drawn as instead, where its mask leaves room for less.
///
/// A rectangle wider than its mask can mark is drawn as the part that can be marked. `sh`
/// states one such fill per shading — ISO 32000-2 §8.7.4.2's Table 76 bounds the operator by
/// the current clipping path and by nothing else, so a display list carries the whole page —
/// and a rasteriser evaluates its shader over the *path's* spans and multiplies the mask in
/// afterwards, so the columns the mask rejects cost full price. The same table says the cost
/// out loud — an unbounded shading paints "across the entire clipping region, which may be
/// time-consuming" — which is the sentence this answers.
///
/// **The benchmark that justifies it**: `bug1721218_reduced.pdf`, the corpus's worst page,
/// states 3490 of those fills, and this is the difference between 10.4 M shaded pixels per
/// render and 85 608. Twenty renders through `examples/callgrind_rasterise` go **38.45 G
/// instructions to 20.03 G**, of which `tiny_skia::pipeline::lowp::gradient` alone is 15.78 G
/// to 0.58 G, and the page's own ink is unchanged to the byte. ADR 0236.
///
/// **What it costs in readability** is this function and one indirection at the call site. What
/// it costs a page it cannot help is one rectangle-containment test per clipped fill, which is
/// [`pdf_render::cropped_rectangle`]'s first line and is why the memoised hull is what it asks
/// for; the specification's own pages measure unchanged.
///
/// The exactness argument, and what a caller owes, are on [`pdf_render::cropped_rectangle`].
/// What is supplied here is the rectangle [`MaskCache::build`] measured, with the pixel of
/// margin [`Band::covering`] takes for the same reason — and `admits` is `None` wherever no
/// rectangle bounds the mask, which is every unclipped command and every bare soft mask.
fn crop_to_mask(
    source: &Path,
    transform: Transform,
    to_device: ToDevice,
    admits: Option<tiny_skia::Rect>,
) -> Option<Path> {
    pdf_render::cropped_rectangle(
        source,
        // The page's own device grid, which is the one `admits` is stated on: the band's row
        // offset is `ToDevice`'s to apply and belongs to neither of them.
        transform.then(to_device.page),
        convert::from_skia_rect(admits?),
    )
}

/// The buffer a transparency group's elements are drawn onto (ISO 32000-2 §11.4.4, §11.4.5).
///
/// Transparent for §11.4.5's isolated group, and a **copy of `pixmap`** for a non-isolated
/// one, whose elements composite onto the group's own backdrop. Copied rather than aliased:
/// the elements read it while they blend, and [`blend::interpolate`] reads the original
/// again to put the two together.
///
/// A non-isolated group carries three guarantees from `pdf-model` — `Command::Group`'s
/// `isolated` states them, and each is what makes §11.4.4's backdrop removal cancel against
/// §11.3.3's re-compositing — and this is where they are checked rather than assumed.
///
/// # Only `band`'s rows are copied, and that is exact rather than approximate
///
/// The buffer still covers the whole surface, so the elements draw under the very transforms
/// they draw under on the page — a buffer starting at another row would shift `tiny-skia`'s
/// `y·sy + ty` into another binade, which is the departure ADR 0219 measured and this backend,
/// being the oracle, does not take. But only the rows of the group's band — the rows its clip
/// and mask admit — are copied into it, because no other row of the buffer can reach the page:
/// every way [`CpuRasterizer::draw_group`] brings the buffer back reads exactly the band's
/// rows, and every operation an element performs on a buffer pixel writes that pixel and no
/// other, so a value outside the band cannot travel into it. A nested group repeats the same
/// argument one level down, over its own band.
///
/// **The benchmark that justifies the crop**, per `CLAUDE.md`'s rule: `0423548.pdf`, the
/// second-slowest document of a 65 944-document crawl of the web, states 132 non-isolated
/// groups on one 1843 × 5103 page and paid **4.3 GB** of whole-surface copies for **82 MB**
/// of band — 2.85 s of the 6.6 that remained after ADR 0271. With this crop, and the
/// soft-mask banding that shipped beside it, `open_one` on that document — open, interpret,
/// rasterise — went from **149.48 G instructions to 76.48 G**, an A/B of two binaries built
/// in one sitting (ADR 0328). What it costs in readability is this paragraph and one block
/// of slice arithmetic below.
///
/// The mask a group's blit still owes, after §8.5.4's intersection has been taken over
/// `buffer`'s band where it could be.
///
/// `None` says the intersection is in the buffer already and the blit carries no mask; the clip's
/// own mask says it was not taken and the blit owes it, which is what this backend did for every
/// group before ADR 0492.
///
/// The arithmetic and its exactness are [`scan::intersect_group`]'s; this is the two conditions
/// that decide whether the buffer is one it may be asked of.
///
/// - **`alpha_is_shape`**, which is `pdf_render::Command::Group`'s own field and the whole
///   argument: Table 139 returns a group's shape beside its alpha and this buffer holds one
///   number per pixel, so the composition is expressible only for a group whose opacity is 1.0
///   everywhere. `pdf-model` answers it, because the display list holds a translucent colour
///   and not the reason it is translucent.
/// - **Isolated only.** A non-isolated group's buffer starts as a copy of the page
///   ([`initial_backdrop`]), so the alpha in it is the backdrop's unioned with the group's and
///   is not that shape at all — the sentence `pdf-model`'s `element_alpha_is_shape` writes one
///   layer up, and the reason the caller's interpolation is left alone.
///
/// A group composited in a blending colour space of its own is *not* excluded: `resolve_blending`
/// rewrites three channels of every pixel and leaves the fourth, so the alpha that reaches here
/// is the one the elements accumulated.
fn group_blit_mask<'a>(
    buffer: &mut tiny_skia::Pixmap,
    group: &Group<'_>,
    (surface, band, top): (Surface, Band, i32),
    clip: scan::Clip<'a>,
) -> Option<&'a tiny_skia::Mask> {
    if !group.isolated || !group.alpha_is_shape {
        return clip.mask();
    }
    let stride = (surface.width() as usize).saturating_mul(4);
    let start = (top.unsigned_abs() as usize).saturating_mul(stride);
    let end = start.saturating_add((band.height as usize).saturating_mul(stride));
    let composed = buffer
        .data_mut()
        .get_mut(start..end)
        .is_some_and(|rows| scan::intersect_group(rows, clip));
    if composed { None } else { clip.mask() }
}

/// # Errors
///
/// [`CpuRasterError::Allocation`] if the buffer does not fit or `band` lies outside it, and
/// [`CpuRasterError::UnsupportedCommand`] for a non-isolated group carrying anything the
/// collapse does not hold for.
fn initial_backdrop(
    pixmap: &tiny_skia::PixmapMut<'_>,
    surface: Surface,
    band: Band,
    group: &Group<'_>,
) -> Result<tiny_skia::Pixmap, CpuRasterError> {
    if !group.isolated
        && (group.compose != Compose::Over
            || group.into != Compose::Over
            || group.blend != pdf_render::BlendMode::Normal)
    {
        return Err(CpuRasterError::UnsupportedCommand(
            "a non-isolated group whose result is not composited by §11.3.3's Normal blend \
             function needs Table 140's group alpha kept apart from the composite alpha \
             (ISO 32000-2 §11.4.4 NOTE 4)"
                .to_owned(),
        ));
    }
    let mut buffer = tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or(
        CpuRasterError::Allocation {
            width: surface.width(),
            height: surface.rows.height,
        },
    )?;
    if group.isolated {
        return Ok(buffer);
    }
    // A byte copy rather than a `Source` draw: both move the band's bytes unchanged, and the
    // slice arithmetic is `Band::rows`'s, over the same rows of both buffers.
    let stride = (surface.width() as usize).saturating_mul(4);
    let start = (band.top.saturating_sub(surface.rows.top) as usize).saturating_mul(stride);
    let end = start.saturating_add((band.height as usize).saturating_mul(stride));
    let source = pixmap
        .as_ref()
        .data()
        .get(start..end)
        .ok_or(CpuRasterError::Allocation {
            width: surface.width(),
            height: band.height,
        })?;
    buffer
        .data_mut()
        .get_mut(start..end)
        .ok_or(CpuRasterError::Allocation {
            width: surface.width(),
            height: band.height,
        })?
        .copy_from_slice(source);
    Ok(buffer)
}

/// Whether a command's own extent lies wholly above or below `surface`'s rows.
///
/// A group answers `false`: its elements carry the extents and each is asked in turn.
///
/// Measured on the *page's* grid, so a strip asks the same question of the same numbers the
/// whole page asks — only over a different run of rows.
fn misses_surface(command: &Command, surface: Surface) -> bool {
    let Some(bounds) = command.device_bounds(surface.page.transform) else {
        return false;
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "a target height below MAX_EXTENT = 2^24 is exact in f32"
    )]
    let (top, bottom) = (
        surface.rows.top as f32,
        surface.rows.top.saturating_add(surface.rows.height) as f32,
    );
    bounds.max.y < top - 1.0 || bounds.min.y > bottom + 1.0
}

/// The rows of `surface` a command list's own marks can reach — a bound, never a coverage.
///
/// The extents are the ones [`misses_surface`] culls by — [`Command::device_bounds`] per
/// leaf, a group answering through its elements — measured on the page's grid and given
/// [`Band::covering`]'s row of margin, so a row outside the answer is a row no command in
/// the list can mark. A leaf whose extent cannot be measured widens the answer to the whole
/// surface, which is the safe direction and the same reading `misses_surface` takes.
///
/// Asked by [`CpuRasterizer::build_soft_mask`], which needs a superset of the rows its
/// buffer was *written* in: outside them the buffer is still the transparency it was
/// allocated as. A list that can mark no row at all answers one row rather than none — that
/// row's pixels are untouched, so every value derived from them is the same
/// [`pdf_render::SoftMask::outside`] constant any other unmarked pixel yields, and a
/// one-row mask keeps [`Built`] free of an empty case no other entry has.
fn marked_rows(commands: &[Command], surface: Surface) -> Band {
    let mut extent = None;
    if !vertical_extent(commands, surface.page.transform, &mut extent, 0) {
        return surface.rows;
    }
    let one_row = Band {
        top: surface.rows.top,
        height: 1,
    };
    let Some((low, high)) = extent else {
        return one_row;
    };
    tiny_skia::Rect::from_ltrb(0.0, low, 1.0, high)
        .and_then(|bounds| Band::covering(bounds, surface))
        // Off the surface entirely, so nothing was marked; the degenerate answer above.
        .unwrap_or(one_row)
}

/// Accumulates the least and greatest device row the leaves of `commands` can mark.
///
/// `true` when every leaf answered; `false` the moment one cannot — an extent
/// [`Command::device_bounds`] does not state, one that overflowed past `f32`, or nesting
/// past [`MAX_GROUP_DEPTH`] — at which point the caller must read the answer as
/// "anywhere". The depth bound mirrors [`CpuRasterizer::encode`]'s rather than trusting
/// it: a group whose clip admits nothing is skipped there without its elements ever being
/// walked, so this walk can meet nesting the draw never did.
fn vertical_extent(
    commands: &[Command],
    to_device: Transform,
    extent: &mut Option<(f32, f32)>,
    depth: usize,
) -> bool {
    if depth > MAX_GROUP_DEPTH {
        return false;
    }
    for command in commands {
        // A group's own extent is its elements' — the documented `None` of
        // `device_bounds` — so the elements are what get asked.
        if let Command::Group { commands, .. } = command {
            if !vertical_extent(commands, to_device, extent, depth.saturating_add(1)) {
                return false;
            }
            continue;
        }
        let Some(bounds) = command.device_bounds(to_device) else {
            return false;
        };
        if !bounds.min.y.is_finite() || !bounds.max.y.is_finite() {
            return false;
        }
        *extent = Some(match *extent {
            None => (bounds.min.y, bounds.max.y),
            Some((low, high)) => (low.min(bounds.min.y), high.max(bounds.max.y)),
        });
    }
    true
}

/// The rows at which to cut this target, or one strip's worth if it may not be cut.
///
/// The strip count asked for is what this machine offers, bounded by [`MAX_STRIPS`] and by
/// [`MIN_STRIP_ROWS`]; what comes back may be fewer, because a cut is only made at a row no
/// curve crosses. **The picture is very nearly independent of the answer**, which is the property
/// ADR 0139 exists to establish and `strip_parallelism.rs` asserts: a machine with four cores and
/// one with thirty-two draw the same bytes but for a handful in a million.
///
/// **"Very nearly" is a correction sessions 381 and 382 made together**, and both halves are worth
/// carrying. The 381st found the claim false — `doc/PDF20_AN001-BPC.pdf` page 1 differed by one
/// pixel between one strip and every division above one — by drawing a page in one strip on
/// purpose, which a confined process must because it may not ask how many cores it has (ADR 0218).
/// The 382nd found the cause not to be the one written down: a strip's offset was folded into the
/// page transform *before* a mark's own transform was composed with it, so the sum rounded at
/// another magnitude. It is applied last now, and that page is exact — see [`ToDevice`].
///
/// What survives is a dependency's. `tiny-skia` maps a point as `y·sy + ty`, and shifting `ty` by
/// a whole number of rows moves the sum into another binade; ADR 0219 measures what is left —
/// fewer than one pixel in ten thousand, none by more than one supersample — and says why no
/// arrangement of this crate's arithmetic closes it.
/// The whole target, as a rectangle in its own pixel space.
///
/// One caller — [`images::ReducedImages::warm`], which asks whether a command marks the raster at
/// all before paying for its reduction, and which is not the strip loop's own test because it runs
/// before there are strips.
#[expect(
    clippy::cast_precision_loss,
    reason = "`rasterize` refuses a target beyond MAX_EXTENT = 2^24, which is the largest \
              integer f32 represents exactly"
)]
fn whole_target(target: TargetSpec) -> pdf_render::Rect {
    pdf_render::Rect::from_corners(
        pdf_render::Point::new(0.0, 0.0),
        pdf_render::Point::new(target.width as f32, target.height as f32),
    )
}

fn plan_strips(list: &DisplayList, target: TargetSpec, asked: Option<u32>) -> Vec<u32> {
    // Asked of the machine only where the caller did not say, and that is not tidiness:
    // `available_parallelism` reads `/proc/self/cgroup` on Linux, and a caller drawing inside a
    // confinement with no filesystem is *killed* for it rather than told no. Such a caller states
    // the number with `with_strips` (ADR 0218).
    let wanted = asked
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map_or(1, std::num::NonZero::get)
                .try_into()
                .unwrap_or(MAX_STRIPS)
        })
        .min(MAX_STRIPS)
        .min(target.height.checked_div(MIN_STRIP_ROWS).unwrap_or(0));
    if wanted < 2 {
        return vec![0, target.height];
    }
    let extents = pdf_render::command_extents(list, target);
    let costs = pdf_render::row_costs(&extents, target);
    let unsplittable = pdf_render::unsplittable_rows(list, target);

    // Asked for from the most strips downwards, taking the first division whose replay is
    // affordable. A strip pays again for every command that reaches it — see
    // `pdf_render::replay_ratio` — and on a page of a few page-wide commands that is the whole
    // cost: `issue12841_reduced.pdf` is two of them and was 105 ms serially against 166 ms in
    // sixteen strips. The bound is a quarter more work than the list itself, which every text
    // page measured in ADR 0137 sits far inside (1.01 to 1.13 at eight strips) and a page of
    // page-wide commands cannot reach at any count above one.
    let mut count = wanted;
    while count > 1 {
        let boundaries =
            pdf_render::strip_boundaries_avoiding(&costs, &unsplittable, count, MIN_STRIP_ROWS);
        if pdf_render::replay_ratio(&extents, &boundaries) <= MAX_REPLAY {
            return boundaries;
        }
        count = count.checked_div(2).unwrap_or(0);
    }
    vec![0, target.height]
}

/// One clip of a chain, ready to be drawn into a mask.
struct Shape<'a> {
    path: tiny_skia::Path,
    /// The same shape before the rasteriser's library saw it, kept so that §10.7.4's
    /// rectangle question can be asked once the band transform is known.
    ///
    /// The clipping region "consists of the set of pixels that would be included by a fill
    /// operation", so a rectangular clip is scan-converted by the rule a rectangular fill is —
    /// and [`pdf_render::device_rectangle`] is the one place that rule decides what a rectangle
    /// is (ADR 0476). It cannot be asked in the loop that builds this, because the band the mask
    /// covers is not known until every clip in the chain has been measured.
    source: &'a Path,
    /// The clip's own transform; the target and band transforms are applied later,
    /// because the band is not known until every clip in the chain has been measured.
    transform: Transform,
    fill_rule: tiny_skia::FillRule,
    /// This shape's transform composed with the band's, filled in once the band is known.
    at: Transform,
    /// What [`rectangular_mark`] says this shape is under `at`, computed once: the answer decides
    /// both whether the shape states anything at all and how it is scan-converted if it does.
    mark: scan::Exact,
}

/// A built clip mask, the band it covers, and the device rectangle it can mark within.
struct Built {
    mask: tiny_skia::Mask,
    band: Band,
    /// Device rectangle outside which this mask is zero, or `None` where nothing bounds it.
    ///
    /// The band already says which *rows* the mask can mark, and a surface is a run of whole
    /// rows, so the columns had nowhere to be recorded until a caller wanted them. One does:
    /// [`pdf_render::cropped_rectangle`] shrinks a rectangular fill to the part of it that can
    /// survive, and this is what tells it where that part is. Outset and rounded outward
    /// exactly as [`Band::covering`] treats the rows, and for the same reason — a clip's bounds
    /// are composed one way and its mask drawn through another.
    admits: Option<tiny_skia::Rect>,
    /// The mask's value at every pixel outside `band`.
    ///
    /// Zero for a clip and for a clip × soft-mask product: ISO 32000-2 §8.5.4's clipping path
    /// admits nothing outside itself, so a band-sized mask and "nothing elsewhere" are the
    /// same statement. For a soft mask it is [`pdf_render::SoftMask::outside`] — §11.6.5.1's
    /// one value for the area the group's marks never reached — which need not be zero: a
    /// white `/BC` puts it at 255. Carrying the constant beside the band is what lets a soft
    /// mask be *stored* over the rows its group could mark instead of over the whole surface
    /// (`doc/todo/40`); every reader substitutes it for the rows the raster does not hold.
    outside: u8,
    /// For a clip × soft-mask product, the soft mask's own values over `band`; `None` for the
    /// other two kinds of entry, which are not products of anything.
    ///
    /// **Kept because a product is not a set and the two factors are not the same kind of
    /// thing.** ISO 32000-2 §8.5.4 intersects the clipping path with the object's shape and
    /// §11.3.7.2 multiplies the mask shape into the result, so a mark meeting a clip and a
    /// soft mask at once needs both factors and not only their product — [`scan::Clip::Both`]
    /// and ADR 0363. Storing it doubles what such an entry costs, which [`Built::held`]
    /// charges to the same budget the product is charged to.
    value: Option<Vec<u8>>,
}

impl Built {
    /// The bytes this entry holds on a surface `width` pixels across.
    fn held(&self, width: u32) -> usize {
        let mask = self.band.mask_bytes(width);
        match self.value {
            Some(_) => mask.saturating_mul(2),
            None => mask,
        }
    }
}

/// What a command's clip and soft mask together let it mark.
///
/// Three answers to one question, which is why they travel together: which rows of the surface
/// are worth drawing into, what coverage multiplies the command there, and — since the surface
/// is a run of whole rows and the band cannot say it — which columns.
#[derive(Debug, Clone, Copy)]
struct Admitted<'a> {
    /// Rows of the surface the command may mark.
    band: Band,
    /// The coverage it is drawn through, and what that coverage *is*: §10.7.4's set of pixels
    /// or §11.6.5's value, which is what decides whether it composes with a mark by `min` or by
    /// a product. See [`scan::Clip`].
    mask: scan::Clip<'a>,
    /// Device rectangle outside which `mask` is zero, or `None` where nothing bounds it.
    admits: Option<tiny_skia::Rect>,
}

/// What a cached mask was built from.
///
/// Three kinds share one cache because they share one budget and one eviction order, and
/// because what a command asks for is the *effective* mask: a clip, a soft mask, or their
/// product. Keeping the product under its own key is what stops a page of masked text from
/// multiplying the two per glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    /// The intersection of a clip chain.
    Clip(ClipId),
    /// A soft mask's values, over the whole target (§11.5).
    Soft(SoftMaskId),
    /// A clip and a soft mask multiplied together.
    ///
    /// §11.5.1's NOTE 2 makes that the right arithmetic rather than a shortcut: a hard clip
    /// "can be represented as a soft clip having shape values of 1.0 inside and 0.0 outside
    /// the clipping path", so intersecting the two is multiplying them.
    Both(ClipId, SoftMaskId),
}

/// Builds and memoises clip masks, within a memory budget.
///
/// A clip commonly applies to thousands of consecutive commands, so rasterising its
/// mask once per command would dominate the render; masks are therefore built on first
/// use and kept.
///
/// The cache is bounded, which the memoisation alone is not. A document names as many
/// distinct clips as it likes — the corpus's worst holds 3554 on one page — so keeping
/// every mask is a memory-exhaustion vector: before this bound and before [`Band`],
/// that page held 1.7 GB of page-sized masks. Dropping an entry costs a rebuild and
/// nothing else, so eviction can be crude, and it is: entries go in build order,
/// oldest first, and the most recently built is never dropped. Clips are used in runs,
/// so build order tracks use order closely enough that an active clip is rebuilt at
/// most once per run.
struct MaskCache {
    surface: Surface,
    anti_alias: bool,
    /// Masks by what they were built from. `None` records a clip that admits no row of the
    /// target, which is worth remembering rather than rediscovering: every command it clips
    /// draws nothing. These entries hold no pixels, and there is one at most per clip in the
    /// display list, so they are bounded by the list itself.
    built: HashMap<Key, Option<Built>>,
    /// Keys holding a mask, in build order, for eviction. Soft masks are not among them —
    /// see [`MaskCache::admit_soft_mask`] for why they are evicted on their own terms.
    order: VecDeque<Key>,
    /// Soft masks holding a raster, in build order, for eviction.
    soft_order: VecDeque<SoftMaskId>,
    /// Bytes held by the soft masks in `built`.
    soft_bytes: usize,
    /// Bytes held by the masks in `built`.
    bytes: usize,
    /// Largest total the masks may reach before the oldest are dropped.
    budget: usize,
    /// Where a mark's own coverage is built before §10.7.4's intersection composes the two.
    /// Here because it is one buffer per band, which is what this cache already is.
    scratch: scan::Scratch,
}

/// Largest total size of the masks a [`MaskCache`] holds, in bytes.
///
/// Sized against the corpus rather than guessed. `bug1721218_reduced.pdf` is the heaviest
/// first page in the 974-document pdf.js corpus, and it never evicts: **3554 clip masks built
/// per render of one 612×792 page, peaking at 12.31 MB against this 32 MB** — 10.85 MB of clip
/// masks, 1.45 MB of soft masks, and three clip × soft-mask products alive at the peak.
///
/// **That figure said 27.9 MB and a margin of 13% until the three-hundred-and-ninety-ninth
/// session, which measured it again and found 12.31.** The 27.9 was taken in the
/// hundred-and-thirteenth and was true then; ADR 0132 arrived in the hundred-and-forty-seventh
/// and made [`DisplayList::add_clip`] return an existing identifier for an identical region,
/// which is a change to how many masks this cache is asked for. **A margin recorded as thin is
/// a claim that decays like any other**, and the reason to re-take it rather than inherit it is
/// [`doc/todo/40`](../../../doc/todo/40-mask-chain-crop.md), whose stated blocker was that this
/// page had no room for its intermediate clips. It has 19.7 MB of room, and they cost 9.4.
///
/// The number is not lowered on that evidence, because nothing has been measured to be paying
/// for it; it is recorded so that a session which finds a document evicting knows what the
/// margin was.
///
/// [`DisplayList::add_clip`]: pdf_render::DisplayList::add_clip
const MASK_BUDGET: usize = 32 << 20;

impl MaskCache {
    /// A cache for one target, bounded by `budget` bytes.
    ///
    /// The budget is a parameter rather than [`MASK_BUDGET`] itself because a page drawn in
    /// strips has one cache per strip and they are alive at once: dividing the constant keeps
    /// a parallel render's mask memory equal to a serial one's.
    fn new(surface: Surface, anti_alias: bool, budget: usize) -> Self {
        Self {
            surface,
            anti_alias,
            built: HashMap::new(),
            order: VecDeque::new(),
            soft_order: VecDeque::new(),
            bytes: 0,
            soft_bytes: 0,
            budget,
            scratch: scan::Scratch::default(),
        }
    }

    /// Returns what a command with this clip and this soft mask may mark.
    ///
    /// `None` means nothing this command draws can survive, which is not the same as
    /// drawing unmasked; an [`Admitted`] whose `mask` is `None` is the unmasked case, where
    /// the band is the whole target.
    ///
    /// A soft mask must already have been evaluated by
    /// [`CpuRasterizer::build_soft_mask`] — building one means rendering a command list,
    /// which a cache cannot do.
    ///
    /// # Errors
    ///
    /// As [`MaskCache::get`], plus [`CpuRasterError::UnknownSoftMask`] for a soft mask that
    /// was not evaluated first.
    fn effective(
        &mut self,
        list: &DisplayList,
        clip: Option<ClipId>,
        mask: Option<SoftMaskId>,
    ) -> Result<Option<Admitted<'_>>, CpuRasterError> {
        match (clip, mask) {
            // Unmasked, and still carrying the scratch buffer: a mark whose own portions share a
            // device pixel is composed in one whether or not anything clips it (§11.6.2, ADR 0590).
            (None, None) => Ok(Some(Admitted {
                band: self.surface.rows,
                mask: scan::Clip::Unclipped {
                    scratch: &self.scratch,
                },
                admits: None,
            })),
            // A clip on its own is §10.7.4's set of pixels, which is the one case a mark's
            // own coverage may be composed with by `min`. Built first and looked up after, the
            // way the `(Some, Some)` arm below does, because the composition needs the cache's
            // scratch buffer beside the mask and the two are different fields.
            (Some(clip), None) => {
                if self.get(list, clip)?.is_none() {
                    return Ok(None);
                }
                let Self { built, scratch, .. } = self;
                Ok(built
                    .get(&Key::Clip(clip))
                    .and_then(Option::as_ref)
                    .map(|built| Admitted {
                        band: built.band,
                        mask: scan::Clip::Region {
                            mask: &built.mask,
                            scratch,
                        },
                        admits: built.admits,
                    }))
            }
            (None, Some(mask)) => {
                // Handed the whole surface rather than the stored band: with no clip to
                // band the draw, the command draws over every row — §11.6.5.1 gives a soft
                // mask a value everywhere — and `tiny-skia` applies a mask to a pixmap of
                // exactly its own size. Returning the band instead would also move the
                // rows the command is drawn from, which is the arithmetic ADR 0219 pins.
                self.expand_soft_mask(mask)?;
                let entry = self.soft_mask(mask)?;
                Ok(Some(Admitted {
                    band: entry.band,
                    mask: scan::Clip::Value {
                        value: &entry.mask,
                        scratch: &self.scratch,
                    },
                    admits: entry.admits,
                }))
            }
            (Some(clip), Some(mask)) => {
                self.combine(list, clip, mask)?;
                let Self { built, scratch, .. } = self;
                let entry = built
                    .get(&Key::Both(clip, mask))
                    .ok_or(CpuRasterError::UnknownClip(clip))?
                    .as_ref();
                // The product is what every draw but a fill takes. A fill takes it beside the
                // soft mask it was made from, because §8.5.4 intersects the clip with the
                // object's own shape *before* §11.3.7.2 multiplies the mask shape in — so the
                // two may not be one buffer at the moment the mark's coverage arrives.
                Ok(entry.map(|built| Admitted {
                    band: built.band,
                    mask: match built.value.as_deref() {
                        Some(value) => scan::Clip::Both {
                            product: &built.mask,
                            value,
                            scratch,
                        },
                        // Only `combine` builds this key and it always stores the soft mask's
                        // rows, so this is unreachable; taking the product alone is what this
                        // backend did before ADR 0363 and is coarser rather than wrong.
                        None => scan::Clip::Value {
                            value: &built.mask,
                            scratch,
                        },
                    },
                    admits: built.admits,
                }))
            }
        }
    }

    /// Builds the product of a clip and a soft mask, if it is not cached already.
    fn combine(
        &mut self,
        list: &DisplayList,
        clip: ClipId,
        mask: SoftMaskId,
    ) -> Result<(), CpuRasterError> {
        if self.built.contains_key(&Key::Both(clip, mask)) {
            return Ok(());
        }
        // The clip is built first and the soft mask read after it, which is safe in that
        // order only because building a clip cannot evict a soft mask — see
        // `admit_soft_mask`.
        self.get(list, clip)?;

        let Some(Some(clipped)) = self.built.get(&Key::Clip(clip)) else {
            // The clip admits no row, so their product admits none either.
            self.built.insert(Key::Both(clip, mask), None);
            return Ok(());
        };
        let Some(Some(soft)) = self.built.get(&Key::Soft(mask)) else {
            return Err(CpuRasterError::UnknownSoftMask(mask));
        };

        let band = clipped.band;
        // The product is zero wherever the clip is, so the clip's rectangle bounds it too; a
        // soft mask has a value everywhere (§11.6.5.1) and bounds nothing on its own.
        let admits = clipped.admits;
        let mut product = clipped.mask.clone();
        let width = self.surface.width() as usize;
        // The soft mask is stored over the rows its group could mark and is `outside`
        // everywhere else (§11.6.5.1), so the clip's rows are assembled here: the constant
        // first, then the stored rows laid over it where the two bands overlap. Both bands
        // count page rows, so the three offsets below are row differences and nothing else.
        let mut soft_rows = vec![soft.outside; product.data().len()];
        let from = band.top.max(soft.band.top);
        let until = band
            .top
            .saturating_add(band.height)
            .min(soft.band.top.saturating_add(soft.band.height));
        if from < until {
            let length = (until.saturating_sub(from) as usize).saturating_mul(width);
            let into = (from.saturating_sub(band.top) as usize).saturating_mul(width);
            let out_of = (from.saturating_sub(soft.band.top) as usize).saturating_mul(width);
            let source = soft
                .mask
                .data()
                .get(out_of..out_of.saturating_add(length))
                .ok_or(CpuRasterError::UnknownSoftMask(mask))?;
            soft_rows
                .get_mut(into..into.saturating_add(length))
                .ok_or(CpuRasterError::UnknownSoftMask(mask))?
                .copy_from_slice(source);
        }
        for (value, &soft) in product.data_mut().iter_mut().zip(soft_rows.iter()) {
            // Two coverages multiply, through the one rounding `scan::intersected` also scales
            // the mark by: the minimum it then takes is only exact while both sides round the
            // same way. ADR 0363.
            *value = scan::scaled(*value, soft);
        }

        self.admit(
            Key::Both(clip, mask),
            Some(Built {
                mask: product,
                band,
                admits,
                // The clip's own zero: outside its band their product admits nothing.
                outside: 0,
                // The soft mask's rows are kept beside the product rather than only inside
                // it: §8.5.4's intersection happens before §11.3.7.2's multiplication, so a
                // fill's own coverage needs the two factors apart. ADR 0363.
                value: Some(soft_rows),
            }),
        );
        Ok(())
    }

    /// Returns an evaluated soft mask's entry.
    fn soft_mask(&self, id: SoftMaskId) -> Result<&Built, CpuRasterError> {
        self.built
            .get(&Key::Soft(id))
            .and_then(Option::as_ref)
            .ok_or(CpuRasterError::UnknownSoftMask(id))
    }

    /// Whether a soft mask's values are already cached.
    fn holds_soft_mask(&self, id: SoftMaskId) -> bool {
        self.built.contains_key(&Key::Soft(id))
    }

    /// Stores an evaluated soft mask, covering the rows its group's marks could reach.
    ///
    /// `band` is those rows — [`marked_rows`]' answer — and `outside` is the mask's value
    /// everywhere else: §11.6.5.1 gives a soft mask one value for the whole area its group's
    /// marks never reached, so a band loses nothing as long as [`Built::outside`] carries
    /// the constant, which is exactly what `doc/todo/40` said an entry would have to do.
    /// [`MaskCache::combine`] substitutes it row by row; the one reader that needs the whole
    /// surface in one raster goes through [`MaskCache::expand_soft_mask`].
    ///
    /// # Why a soft mask evicts only another soft mask
    ///
    /// Rebuilding an evicted *clip* costs a fill; rebuilding an evicted soft mask means
    /// rendering a whole command list, which only [`CpuRasterizer`] can do — so a cache that
    /// dropped one while a caller was in the middle of combining it with a clip would have
    /// nothing to hand back. Soft masks therefore sit outside the clip eviction order
    /// entirely and are dropped only here, when a *new* one arrives, which is a moment when
    /// no combination is in flight: [`CpuRasterizer::encode`] evaluates the mask a command
    /// needs before it asks for anything else. Their budget is the same one, counted
    /// separately, and the newest is never dropped for the same reason a clip's is not.
    fn admit_soft_mask(&mut self, id: SoftMaskId, mask: tiny_skia::Mask, band: Band, outside: u8) {
        self.soft_bytes = self
            .soft_bytes
            .saturating_add(band.mask_bytes(self.surface.width()));
        self.soft_order.push_back(id);
        self.built.insert(
            Key::Soft(id),
            Some(Built {
                mask,
                band,
                // A mask has a value everywhere — §11.6.5.1 gives one outside its group's
                // bounding box — so no rectangle bounds where it is non-zero.
                admits: None,
                outside,
                // A soft mask is not a product of anything, so there is nothing beside it.
                value: None,
            }),
        );

        while self.soft_bytes > self.budget && self.soft_order.len() > 1 {
            let Some(oldest) = self.soft_order.pop_front() else {
                break;
            };
            if let Some(Some(entry)) = self.built.remove(&Key::Soft(oldest)) {
                self.soft_bytes = self
                    .soft_bytes
                    .saturating_sub(entry.band.mask_bytes(self.surface.width()));
            }
        }
    }

    /// Rebuilds a soft mask's entry to cover the whole surface, if it does not already.
    ///
    /// The reader this serves is a command masked by a soft mask *alone*: with no clip to
    /// band the draw it draws over every row of the surface, the mask has a value on all of
    /// them (§11.6.5.1), and `tiny-skia` applies a mask to a pixmap of exactly its own size
    /// — so the banded entry cannot serve it the way it serves [`MaskCache::combine`]. The
    /// expansion writes `outside` everywhere and lays the stored band over it, which is byte
    /// for byte what the whole-surface conversion produced before the entries were banded:
    /// a row outside the band is a row the mask's group never marked, and the value derived
    /// from an unmarked pixel *is* the constant ([`pdf_render::SoftMask::outside`] is
    /// `value([0, 0, 0, 0])`).
    ///
    /// Memoised by replacement — the entry becomes the expanded one — so a mask read this
    /// way twice is expanded once, and a document whose masks are all read this way pays
    /// what storing every mask whole used to cost and no more. The growth is charged to the
    /// same budget, evicting other soft masks but never this one, for [`MaskCache::admit`]'s
    /// reason: the caller is about to draw with it.
    ///
    /// # Errors
    ///
    /// [`CpuRasterError::UnknownSoftMask`] for a mask never evaluated, and
    /// [`CpuRasterError::Allocation`] where the expanded raster cannot be built.
    fn expand_soft_mask(&mut self, id: SoftMaskId) -> Result<(), CpuRasterError> {
        let rows = self.surface.rows;
        let width = self.surface.width();
        let entry = self
            .built
            .get(&Key::Soft(id))
            .and_then(Option::as_ref)
            .ok_or(CpuRasterError::UnknownSoftMask(id))?;
        if entry.band == rows {
            return Ok(());
        }
        let mut values = vec![entry.outside; rows.mask_bytes(width)];
        let start =
            (entry.band.top.saturating_sub(rows.top) as usize).saturating_mul(width as usize);
        let end = start.saturating_add(entry.band.mask_bytes(width));
        values
            .get_mut(start..end)
            .ok_or(CpuRasterError::UnknownSoftMask(id))?
            .copy_from_slice(entry.mask.data());
        let held = entry.band.mask_bytes(width);
        let outside = entry.outside;
        let mask = tiny_skia::Mask::from_vec(
            values,
            tiny_skia::IntSize::from_wh(width, rows.height).ok_or(CpuRasterError::Allocation {
                width,
                height: rows.height,
            })?,
        )
        .ok_or(CpuRasterError::Allocation {
            width,
            height: rows.height,
        })?;
        self.soft_bytes = self
            .soft_bytes
            .saturating_sub(held)
            .saturating_add(rows.mask_bytes(width));
        // Replacement, not admission: the entry keeps its place in `soft_order`.
        self.built.insert(
            Key::Soft(id),
            Some(Built {
                mask,
                band: rows,
                admits: None,
                outside,
                value: None,
            }),
        );

        while self.soft_bytes > self.budget && self.soft_order.len() > 1 {
            let Some(oldest) = self.soft_order.pop_front() else {
                break;
            };
            if oldest == id {
                // Never the entry in hand. Parking it at the back keeps the loop finite:
                // everything in front of it is popped before it comes round again, and a
                // queue of one ends the loop.
                self.soft_order.push_back(oldest);
                continue;
            }
            if let Some(Some(entry)) = self.built.remove(&Key::Soft(oldest)) {
                self.soft_bytes = self
                    .soft_bytes
                    .saturating_sub(entry.band.mask_bytes(self.surface.width()));
            }
        }
        Ok(())
    }

    /// Returns the mask for `id`, the band it covers and the rectangle it marks within,
    /// building it if needed.
    ///
    /// `None` means the clip admits no row of the target: the caller must draw
    /// nothing at all, which is not the same as drawing unclipped.
    fn get(&mut self, list: &DisplayList, id: ClipId) -> Result<Option<&Built>, CpuRasterError> {
        if !self.built.contains_key(&Key::Clip(id)) {
            let chain = Self::resolve_chain(list, id)?;
            let built = self.build(list, id, &chain)?;
            self.admit(Key::Clip(id), built);
        }

        // Absence here would mean `admit` dropped the entry it had just stored, which
        // it does not do; reporting it beats drawing an unclipped page if it ever did.
        Ok(self
            .built
            .get(&Key::Clip(id))
            .ok_or(CpuRasterError::UnknownClip(id))?
            .as_ref())
    }

    /// Walks parent links from `id` to the root, returning the chain root-first.
    ///
    /// Iterative rather than recursive, and bounded by a seen-set: a deep or cyclic
    /// chain is reachable from a malformed document, and recursion there would exhaust
    /// the stack. Both a cycle and a dangling reference are reported as errors.
    fn resolve_chain(list: &DisplayList, id: ClipId) -> Result<Vec<ClipId>, CpuRasterError> {
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(id);

        while let Some(this) = current {
            if !seen.insert(this) {
                return Err(CpuRasterError::CyclicClip(this));
            }
            let clip = list.clip(this).ok_or(CpuRasterError::UnknownClip(this))?;
            chain.push(this);
            current = clip.parent;
        }

        chain.reverse();
        Ok(chain)
    }

    /// Builds the effective mask for a root-first chain, in the band it covers.
    ///
    /// The whole chain is drawn into one mask rather than each clip being cached and
    /// intersected with its parent's. Building the chain costs its depth in band-sized fills,
    /// which is less than one page-sized copy.
    ///
    /// **The reason this comment used to give for that is wrong, and the correction is worth
    /// keeping because it names a real optimisation nobody has taken.** It said "a parent
    /// covers a different band from its child, so a parent's mask cannot be reused as a
    /// starting point" — but the band comes from the running *intersection* of the chain's
    /// bounds, so a child's band is always contained in its parent's. A parent's rows for the
    /// child's band would then be the prefix's contribution, and a chain could be one crop plus
    /// one `intersect_path` instead of a fill plus depth-minus-one intersects.
    ///
    /// **Three things about that were re-derived in the three-hundred-and-ninety-ninth session
    /// and all three moved**, which is why the item is still open and why it is
    /// [`doc/todo/40`](../../../doc/todo/40-mask-chain-crop.md) rather than a line here:
    ///
    /// - **It is worth about 17% of the page and not "most of `MaskCache::get`'s 24.3%".**
    ///   Intermediates are barely shared: 3551 leaf clips on `bug1721218_reduced.pdf` reach
    ///   through **7066 distinct nodes**, so building each node once replaces 3551 fills and
    ///   10 702 intersects with 7065 intersects and as many crops — 42% of this function.
    /// - **It is not blocked on memory.** That was the stated blocker, on a peak of 27.9 MB
    ///   against [`MASK_BUDGET`]; the peak is **12.31 MB** and the intermediates cost 9.4.
    /// - **It is not obviously pixel-exact, and the sentence above is where that hides.** A
    ///   mask value at a given device row *does* depend on which band holds it, because
    ///   [`ToDevice`] composes the band's first row into the translation and ADR 0219 measures
    ///   what shifting a whole number of rows does to `y·sy + ty` — fewer than one pixel in ten
    ///   thousand, none by more than one supersample, but not nothing. A parent's mask rows are
    ///   therefore *nearly* the prefix's contribution for the child's band, and this backend is
    ///   the oracle. Taking the item means either building intermediates in the child's band or
    ///   proving the difference away.
    ///
    /// **A fourth thing was measured in the seven-hundred-and-forty-seventh, and it is why the
    /// `retain_mut` below is here rather than any of the three above** (ADR 0656). The exactness
    /// question has a *price*, and it is nearly the whole item: restricted to the prefixes a
    /// parent shares a band with — the ones reusable byte for byte, which is half the corpus's
    /// worst page's nodes — the proposal saves **5.6%** of that page's scanned mask rows where
    /// taking the departure saves **51.1%** (`pdf-model/examples/clip_chain_census`, which prints
    /// both arms). So there was never a cheap exact version of *reuse*.
    ///
    /// What the same census found instead is that reuse was the wrong question on that page:
    /// **three chain steps in four state a rectangle that admits every pixel of the band they are
    /// converted into**, and a step that admits everything can be *declined* rather than reused.
    /// That costs no departure of any kind — nothing is shifted between bands, so ADR 0219's
    /// arithmetic never enters — and it is what [`scan::admits_every_pixel`] answers.
    fn build(
        &self,
        list: &DisplayList,
        id: ClipId,
        chain: &[ClipId],
    ) -> Result<Option<Built>, CpuRasterError> {
        let mut shapes = Vec::with_capacity(chain.len());
        // The effective clip is the intersection of the chain, so it lies within the
        // intersection of the chain's bounds. Path bounds are control-point bounds,
        // which overstate a curve — the safe direction, since too large a band only
        // costs work while too small a one would erase pixels the clip admits.
        let mut bounds: Option<tiny_skia::Rect> = None;

        for &id in chain {
            let clip = list.clip(id).ok_or(CpuRasterError::UnknownClip(id))?;
            // §8.5.4 with §8.5.3.3.1: an empty clipping path encloses nothing, so the
            // chain admits no row. `tiny-skia` would refuse the path instead, which is why
            // `Clip::admits_nothing` states the rule for both backends rather than either
            // rasteriser's treatment of an empty path standing in for it.
            if clip.admits_nothing() {
                return Ok(None);
            }
            let path = convert::path(&clip.path).ok_or(CpuRasterError::InvalidPath)?;
            // A bound that overflows to infinity is left out of the measurement rather
            // than reported. The band is an optimisation over which rows may be marked,
            // so leaving a clip out of it only widens the band; what is drawn is decided
            // by the mask, which is built from the paths themselves either way.
            if let Some(device) = path.bounds().transform(convert::transform(
                clip.transform.then(self.surface.page.transform),
            )) {
                bounds = match bounds {
                    None => Some(device),
                    // An empty intersection is an empty clip, not a failure.
                    Some(outer) => match outer.intersect(&device) {
                        Some(both) => Some(both),
                        None => return Ok(None),
                    },
                };
            }
            shapes.push(Shape {
                path,
                source: &clip.path,
                transform: clip.transform,
                fill_rule: convert::fill_rule(clip.fill_rule),
                at: Transform::IDENTITY,
                mark: scan::Exact::default(),
            });
        }

        // A chain always holds at least the clip it was resolved from, so this is
        // unreachable; an empty clip chain would mean "clipped by nothing", which the
        // caller expresses by having no clip at all, and silently drawing unclipped here
        // would be exactly the plausible-looking wrong page this backend refuses to
        // produce.
        if shapes.is_empty() {
            return Err(CpuRasterError::UnknownClip(id));
        }
        let band = match bounds {
            Some(bounds) => match Band::covering(bounds, self.surface) {
                Some(band) => band,
                // The clip admits no row of this surface at all.
                None => return Ok(None),
            },
            // Nothing in the chain could be measured, so nothing bounds the band.
            None => self.surface.rows,
        };
        // The columns, kept for the same reason the rows are and treated the same way: a whole
        // pixel of margin before rounding outward, because `bounds` composes the clip's
        // transform with the page's while the mask below composes the band's in as well, and
        // the two agree to within rounding rather than exactly. See [`Band::covering`].
        //
        // A rectangle covering the whole surface is recorded as `None`, which is not tidiness:
        // it is what keeps [`crop_to_mask`] off a page that has nothing to gain. The commonest
        // clip in the corpus is a page-wide rectangle — the specification's own page 6 wraps
        // 303 text runs in one, which `DisplayList::add_clip` folds into a single identifier
        // (ADR 0132) — and answering here costs one containment test per chain *built* where
        // answering in the fill would cost one per fill drawn. Measured: page 6's rasterisation
        // is 4 004.7 M without the crop and 4 021.7 M with it asked per fill, which is the
        // whole of the +0.42%.
        let admits = bounds
            .and_then(|bounds| bounds.outset(1.0, 1.0))
            .and_then(|bounds| bounds.round_out())
            .map(|rounded| rounded.to_rect())
            .filter(|admits| !self.covers_the_surface(*admits));

        let to_band = self.surface.to_device(band);
        let extent = (self.surface.width(), band.height);
        // §10.7.4 says a clipping region "consists of the set of pixels that would be included by
        // a fill operation", so a region is measured by the rule a mark is: the same
        // [`rectangular_mark`], for the same reason `clip_intersection.rs` exists — a mark painted
        // at its exact area under a region measured to a quarter breaks `S ∩ C = S`.
        //
        // **And a step whose fill would include every pixel of this band states nothing**, so it
        // is dropped here rather than scan-converted and then composed with `min`. That is the one
        // saving in this function that costs no departure at all — see [`scan::admits_every_pixel`]
        // — and on the corpus's worst page it is three chain steps in four
        // (`pdf-model/examples/clip_chain_census`). ADR 0656.
        shapes.retain_mut(|shape| {
            shape.at = to_band.of(shape.transform);
            shape.mark = rectangular_mark(shape.source, shape.at);
            !scan::admits_every_pixel(&shape.mark, extent)
        });
        let mut mask = tiny_skia::Mask::new(self.surface.width(), band.height).ok_or(
            CpuRasterError::Allocation {
                width: self.surface.width(),
                height: band.height,
            },
        )?;
        // A fresh mask blocks everything, so filling the root path is what opens it.
        let Some((root, nested)) = shapes.split_first() else {
            // Every step admitted the whole band, so the chain's region is the whole band: what
            // the fills above would have written, pixel for pixel, is the level a whole pixel's
            // coverage takes.
            mask.data_mut().fill(u8::MAX);
            return Ok(Some(Built {
                mask,
                band,
                admits,
                outside: 0,
                value: None,
            }));
        };
        scan::mask_fill(
            &mut mask,
            &root.path,
            root.fill_rule,
            self.anti_alias,
            (convert::transform(root.at), &root.mark),
        );
        if !nested.is_empty() {
            // One scratch mask for the whole chain, allocated from the same width and height as
            // the mask above so that the two are the same size by construction. `tiny-skia`
            // allocates one per `intersect_path` call; the chain needs only one, and the
            // difference is 3554 allocations rather than 7108 on the corpus's worst page.
            let mut scratch = tiny_skia::Mask::new(self.surface.width(), band.height).ok_or(
                CpuRasterError::Allocation {
                    width: self.surface.width(),
                    height: band.height,
                },
            )?;
            for shape in nested {
                scan::mask_intersect(
                    &mut mask,
                    &mut scratch,
                    &shape.path,
                    shape.fill_rule,
                    self.anti_alias,
                    (convert::transform(shape.at), &shape.mark),
                );
            }
        }

        Ok(Some(Built {
            mask,
            band,
            admits,
            // §8.5.4: outside the clipping path nothing is admitted, so outside its band
            // there is nothing to state.
            outside: 0,
            // A clip on its own is already the set §10.7.4 states; nothing multiplies it.
            value: None,
        }))
    }

    /// Whether a device rectangle reaches every pixel of this surface.
    ///
    /// Measured on the *page's* grid, which is where a clip's bounds are computed and where
    /// this surface's own rows are counted, so a strip asks the question of the same numbers
    /// the whole page asks.
    fn covers_the_surface(&self, rect: tiny_skia::Rect) -> bool {
        #[expect(
            clippy::cast_precision_loss,
            reason = "rasterize rejects a target larger than MAX_EXTENT = 2^24, and every \
                      integer below that is exact in f32"
        )]
        let (top, bottom) = (
            self.surface.rows.top as f32,
            self.surface
                .rows
                .top
                .saturating_add(self.surface.rows.height) as f32,
        );
        #[expect(
            clippy::cast_precision_loss,
            reason = "as above, for the width this surface spans"
        )]
        let right = self.surface.width() as f32;
        rect.left() <= 0.0 && rect.top() <= top && rect.right() >= right && rect.bottom() >= bottom
    }

    /// Stores an entry and evicts oldest-first until the budget is met.
    fn admit(&mut self, id: Key, built: Option<Built>) {
        if let Some(entry) = &built {
            self.bytes = self.bytes.saturating_add(entry.held(self.surface.width()));
            self.order.push_back(id);
        }
        self.built.insert(id, built);

        // The entry just built is the last in `order` and the caller is about to draw
        // with it, so eviction stops before reaching it even if one mask alone exceeds
        // the budget. A budget that cannot hold the mask in hand is not a reason to
        // fail the page.
        while self.bytes > self.budget && self.order.len() > 1 {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(Some(entry)) = self.built.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(entry.held(self.surface.width()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pdf_render::{
        Clip, ClipId, DisplayList, FillRule, Path, PathCommand, Point, Size, SoftMaskId,
        TargetSpec, Transform,
    };

    use super::{Band, MASK_BUDGET, MaskCache, RECTANGULAR_OUTLINE_VERBS, Surface};

    /// What [`RECTANGULAR_OUTLINE_VERBS`] is a margin above, read off the library rather than
    /// assumed — and that the shape it spells really is the rectangle §10.7.4's closed form is
    /// about.
    ///
    /// A stroker that started spelling this outline with more verbs than the constant admits
    /// would take every axis-aligned rule back to the supersampled converter's quarter with no
    /// gate failing anywhere else, because the mark would still be the right shape.
    #[test]
    fn the_outline_of_a_straight_rule_is_a_rectangle() {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(10.0, 20.0);
        builder.line_to(30.0, 20.0);
        let rule = builder.finish().expect("a two-point path");
        let stroke = tiny_skia::Stroke {
            width: 4.0,
            line_cap: tiny_skia::LineCap::Butt,
            ..tiny_skia::Stroke::default()
        };
        let outline = rule.stroke(&stroke, 1.0).expect("a stroked rule");
        assert!(
            outline.len() <= RECTANGULAR_OUTLINE_VERBS,
            "the outline takes {} verbs, above the constant's {RECTANGULAR_OUTLINE_VERBS}",
            outline.len()
        );
        let rect = pdf_render::device_rectangle(
            &super::convert::from_skia_path(&outline),
            Transform::IDENTITY,
        )
        .expect("a butt-capped axis-aligned rule outlines to one rectangle");
        assert_eq!(
            (rect.min.x, rect.min.y, rect.max.x, rect.max.y),
            (10.0, 18.0, 30.0, 22.0)
        );
    }

    /// A page carrying `count` clips, each a thin horizontal bar at its own height.
    ///
    /// Thin so that one mask is small and a budget of a few of them is easy to state.
    fn stacked_clips(count: u16) -> (DisplayList, Vec<ClipId>, TargetSpec) {
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        let mut ids = Vec::with_capacity(usize::from(count));
        for index in 0..count {
            let y = f32::from(index) * 2.0;
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(0.0, y)));
            path.push(PathCommand::LineTo(Point::new(200.0, y)));
            path.push(PathCommand::LineTo(Point::new(200.0, y + 1.0)));
            path.push(PathCommand::LineTo(Point::new(0.0, y + 1.0)));
            path.push(PathCommand::Close);
            ids.push(
                list.add_clip(Clip {
                    path,
                    transform: Transform::IDENTITY,
                    fill_rule: FillRule::NonZero,
                    parent: None,
                })
                .expect("under the clip limit"),
            );
        }
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        (list, ids, target)
    }

    /// A clip chain whose steps admit the whole band builds the mask the remaining steps alone
    /// build — bytes, band and rectangle alike.
    ///
    /// The saving `MaskCache::build`'s `retain_mut` takes is byte-identical by arithmetic, so no
    /// page can discriminate it and this is what stands in for one: a bar wrapped in two
    /// page-covering rectangles against the same bar wrapped in nothing. What it protects against
    /// is not the arithmetic — `scan`'s own tests pin that — but a future condition that drops a
    /// step which does *not* admit everything, which would erase pixels the clip rejects with
    /// every gate in this tree still green on the pages that have no such chain. ADR 0656.
    #[test]
    fn a_chain_step_admitting_the_whole_band_changes_no_byte_of_the_mask() {
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        let rectangle = |ltrb: (f32, f32, f32, f32)| {
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(ltrb.0, ltrb.1)));
            path.push(PathCommand::LineTo(Point::new(ltrb.2, ltrb.1)));
            path.push(PathCommand::LineTo(Point::new(ltrb.2, ltrb.3)));
            path.push(PathCommand::LineTo(Point::new(ltrb.0, ltrb.3)));
            path.push(PathCommand::Close);
            path
        };
        let mut add = |path: Path, parent: Option<ClipId>| {
            list.add_clip(Clip {
                path,
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .expect("under the clip limit")
        };
        // The bar's edge is fractional on purpose: a whole-pixel one would be 0 or 255 everywhere
        // and could not tell a mask that was composed from one that was not.
        let bar = rectangle((0.0, 40.25, 200.0, 44.75));
        let alone = add(bar.clone(), None);
        let outer = add(rectangle((-5.0, -5.0, 205.0, 205.0)), None);
        let inner = add(rectangle((-1.0, -1.0, 201.0, 201.0)), Some(outer));
        let wrapped = add(bar, Some(inner));

        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        let mut cache = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
        let read = |cache: &mut MaskCache, id| {
            cache
                .get(&list, id)
                .expect("a rectangular chain builds")
                .map(|built| (built.band, built.admits, built.mask.data().to_vec()))
                .expect("the chain admits rows")
        };
        let (band, admits, bytes) = read(&mut cache, alone);
        assert!(
            bytes.iter().any(|&level| level > 0 && level < u8::MAX),
            "the bar's edge must be partly covered for this to discriminate"
        );
        assert_eq!(read(&mut cache, wrapped), (band, admits, bytes));
    }

    /// The bound is the whole point of the cache, so it is checked directly rather than
    /// inferred from a render: a page whose masks happen to fit says nothing about one
    /// whose do not, and the documents that do not fit are the hostile ones.
    #[test]
    fn the_mask_cache_stays_inside_its_budget() {
        let (list, ids, target) = stacked_clips(40);
        let mut cache = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
        // Room for two of these masks and no more.
        let one = Band { top: 0, height: 4 }.mask_bytes(target.width);
        cache.budget = one * 2;

        for (drawn, &id) in ids.iter().enumerate() {
            cache.get(&list, id).expect("a rectangular clip builds");
            assert!(
                cache.bytes <= cache.budget.max(one),
                "after {} clips the cache holds {} bytes, over a budget of {}",
                drawn + 1,
                cache.bytes,
                cache.budget
            );
        }
        assert!(
            cache.built.len() < ids.len(),
            "nothing was evicted, so the budget was never exercised"
        );
    }

    /// Rebuilding an evicted mask must produce the mask that was dropped.
    ///
    /// Eviction is only safe because it costs a rebuild and nothing else. If a rebuilt
    /// mask differed, how much memory happened to be available would decide what the
    /// page looks like.
    #[test]
    fn a_rebuilt_mask_is_the_mask_that_was_evicted() {
        let (list, ids, target) = stacked_clips(8);
        let first = *ids.first().expect("eight clips");

        let mut generous = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
        let before = generous
            .get(&list, first)
            .expect("builds")
            .map(|built| (built.band, built.admits, built.mask.data().to_vec()));

        let mut tight = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
        tight.budget = 1;
        for &id in &ids {
            tight.get(&list, id).expect("builds");
        }
        let after = tight
            .get(&list, first)
            .expect("rebuilds")
            .map(|built| (built.band, built.admits, built.mask.data().to_vec()));

        assert!(before.is_some(), "the clip covers rows of the target");
        assert_eq!(before, after, "a rebuilt mask differs from the original");
    }

    /// A clip that admits no row is remembered as such, not rebuilt on every command.
    #[test]
    fn a_clip_off_the_target_is_cached_as_admitting_nothing() {
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 400.0)));
        path.push(PathCommand::LineTo(Point::new(200.0, 400.0)));
        path.push(PathCommand::LineTo(Point::new(200.0, 500.0)));
        path.push(PathCommand::LineTo(Point::new(0.0, 500.0)));
        path.push(PathCommand::Close);
        let id = list
            .add_clip(Clip {
                path,
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("first clip");
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");

        let mut cache = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
        assert!(
            cache.get(&list, id).expect("resolves").is_none(),
            "a clip entirely above the page admits no row"
        );
        assert_eq!(cache.bytes, 0, "an empty clip holds no pixels");
        assert!(
            cache.built.contains_key(&super::Key::Clip(id)),
            "the emptiness is remembered rather than rediscovered per command"
        );
    }

    /// A clip must never evict a soft mask, because only the rasteriser can rebuild one.
    ///
    /// `MaskCache::combine` reads the soft mask *after* building the clip it multiplies into,
    /// and that order is only safe under this rule. A cache that dropped the mask there would
    /// error out on a page that has many clips and one mask — which is `bug1721218_reduced.pdf`
    /// with a `gs` added, not a hypothetical.
    #[test]
    fn a_clip_never_evicts_a_soft_mask() {
        let (list, ids, target) = stacked_clips(40);
        let surface = Surface::whole(target);
        let mut cache = MaskCache::new(surface, true, MASK_BUDGET);
        cache.budget = Band { top: 0, height: 4 }.mask_bytes(target.width) * 2;

        let mask = SoftMaskId::new(0);
        cache.admit_soft_mask(
            mask,
            tiny_skia::Mask::new(target.width, target.height).expect("a target-sized mask"),
            surface.rows,
            0,
        );
        for &id in &ids {
            cache.get(&list, id).expect("a rectangular clip builds");
        }

        assert!(
            cache.built.len() < ids.len(),
            "nothing was evicted, so the rule was never exercised"
        );
        assert!(
            cache.soft_mask(mask).is_ok(),
            "the soft mask was dropped by clips it has nothing to do with"
        );
    }

    /// The shipped budget is stated in the type, so a careless edit to it fails here.
    #[test]
    fn the_shipped_budget_is_thirty_two_mebibytes() {
        assert_eq!(MASK_BUDGET, 32 * 1024 * 1024);
    }

    /// A soft mask stored over its band combines with a clip exactly as one stored whole.
    ///
    /// This is the exactness claim `doc/todo/40` said had to be settled before a soft mask
    /// could be banded: outside its group's marks the mask is one constant (§11.6.5.1), so
    /// an entry that carries the constant beside the band is the same *function* of a page
    /// row as the whole-surface raster was — and [`MaskCache::combine`] is the reader that
    /// has to substitute it. The clip here spans rows above, inside and below the soft
    /// band, so both substitution edges are exercised.
    #[test]
    fn a_banded_soft_mask_combines_as_a_whole_one_does() {
        let (list, ids, target) = stacked_clips(40);
        let clip = *ids.get(20).expect("forty clips"); // rows around y = 40..41
        let surface = Surface::whole(target);
        let width = target.width as usize;

        // Clip 20 sits at page y 40..41, which the flipped page transform puts at device
        // rows 159..160, so its band is a few rows around them. A two-row soft band inside
        // it leaves clip rows above and below to take the substituted constant — and the
        // constant is non-zero, which is the white-`/BC` case a zero fill would get wrong.
        let band = Band {
            top: 159,
            height: 2,
        };
        let outside = 200u8;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "row and column indices in a 200-pixel test target fit u8 modulo 256"
        )]
        let pattern: Vec<u8> = (0..band.height as usize * width)
            .map(|index| (index % 251) as u8)
            .collect();

        let mask = SoftMaskId::new(0);
        let banded_mask = tiny_skia::Mask::from_vec(
            pattern.clone(),
            tiny_skia::IntSize::from_wh(target.width, band.height).expect("a non-zero size"),
        )
        .expect("data matches the size");
        let mut banded = MaskCache::new(surface, true, MASK_BUDGET);
        banded.admit_soft_mask(mask, banded_mask, band, outside);
        banded.combine(&list, clip, mask).expect("combines");

        // The same mask stored the way every entry used to be: the constant everywhere,
        // the band's rows laid in.
        let mut whole_values = vec![outside; surface.rows.mask_bytes(target.width)];
        let start = band.top as usize * width;
        whole_values[start..start + pattern.len()].copy_from_slice(&pattern);
        let whole_mask = tiny_skia::Mask::from_vec(
            whole_values,
            tiny_skia::IntSize::from_wh(target.width, target.height).expect("a non-zero size"),
        )
        .expect("data matches the size");
        let mut whole = MaskCache::new(surface, true, MASK_BUDGET);
        whole.admit_soft_mask(mask, whole_mask, surface.rows, outside);
        whole.combine(&list, clip, mask).expect("combines");

        let read = |cache: &MaskCache| {
            cache
                .built
                .get(&super::Key::Both(clip, mask))
                .and_then(Option::as_ref)
                .map(|built| (built.band, built.mask.data().to_vec()))
                .expect("the product was built")
        };
        assert_eq!(
            read(&banded),
            read(&whole),
            "the banded entry and the whole one produced different products"
        );
    }

    /// Expansion writes the outside constant everywhere the band was not, and nothing else.
    ///
    /// This is the other reader of a banded entry: a command masked by a soft mask alone
    /// needs one raster covering the surface, and what it must receive is byte for byte
    /// what the whole-surface conversion used to produce — the band's rows unchanged, the
    /// constant on every other row (§11.6.5.1).
    #[test]
    fn an_expanded_soft_mask_is_the_constant_with_the_band_laid_in() {
        let (_, _, target) = stacked_clips(4);
        let surface = Surface::whole(target);
        let width = target.width as usize;

        let band = Band { top: 3, height: 2 };
        let outside = 55u8;
        let pattern: Vec<u8> = vec![7; band.height as usize * width];
        let mask = SoftMaskId::new(9);
        let banded_mask = tiny_skia::Mask::from_vec(
            pattern.clone(),
            tiny_skia::IntSize::from_wh(target.width, band.height).expect("a non-zero size"),
        )
        .expect("data matches the size");

        let mut cache = MaskCache::new(surface, true, MASK_BUDGET);
        cache.admit_soft_mask(mask, banded_mask, band, outside);
        cache.expand_soft_mask(mask).expect("expands");

        let entry = cache.soft_mask(mask).expect("still cached");
        assert_eq!(entry.band, surface.rows, "the entry now covers the surface");
        let mut expected = vec![outside; surface.rows.mask_bytes(target.width)];
        let start = band.top as usize * width;
        expected[start..start + pattern.len()].copy_from_slice(&pattern);
        assert_eq!(
            entry.mask.data(),
            expected.as_slice(),
            "expansion changed a value the conversion produced"
        );
        assert_eq!(
            cache.soft_bytes,
            surface.rows.mask_bytes(target.width),
            "the growth was not charged to the budget"
        );
        cache.expand_soft_mask(mask).expect("idempotent");
        assert_eq!(
            cache.soft_bytes,
            surface.rows.mask_bytes(target.width),
            "a second expansion charged the budget again"
        );
    }

    /// The backdrop buffer holds the band's rows of the page and transparency elsewhere.
    ///
    /// [`initial_backdrop`]'s crop rests on one claim — no buffer row outside the group's
    /// band can reach the page — and this pins what the crop must still guarantee: inside
    /// the band the buffer *is* the page, byte for byte, because [`blend::interpolate`]
    /// reads those rows as the group's backdrop and any difference there would be a
    /// different picture, not a faster one.
    #[test]
    fn the_backdrop_copy_is_the_bands_rows() {
        let list = DisplayList::new(Size::new(8.0, 8.0));
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        let surface = Surface::whole(target);

        let mut page = tiny_skia::Pixmap::new(target.width, target.height).expect("a pixmap");
        for (index, pixel) in page.pixels_mut().iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "an 8x8 fixture has 64 pixels, all of which fit u8"
            )]
            let level = index as u8;
            *pixel = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, level)
                .expect("black under any alpha is premultiplied-valid");
        }

        let band = Band { top: 2, height: 3 };
        let group = super::Group {
            commands: &[],
            alpha: 1.0,
            blend: pdf_render::BlendMode::Normal,
            clip: None,
            mask: None,
            isolated: false,
            alpha_is_shape: false,
            compose: super::Compose::Over,
            into: super::Compose::Over,
            blending: None,
        };
        let buffer = super::initial_backdrop(&page.as_mut(), surface, band, &group)
            .expect("a small buffer allocates");

        let stride = target.width as usize * 4;
        let (start, end) = (band.top as usize * stride, 5 * stride);
        assert_eq!(
            &buffer.data()[start..end],
            &page.data()[start..end],
            "inside the band the buffer must be the page"
        );
        assert!(
            buffer.data()[..start].iter().all(|&byte| byte == 0)
                && buffer.data()[end..].iter().all(|&byte| byte == 0),
            "outside the band the buffer holds rows nothing can read, and copying them \
             was 2.85 s of a 6.6 s page (ADRs 0271, 0328)"
        );
    }

    /// `marked_rows` bounds every leaf, recurses into groups, and widens where it cannot say.
    #[test]
    fn marked_rows_bounds_every_leaf_and_widens_where_it_cannot_say() {
        use pdf_render::{BlendMode, Paint};

        let list = DisplayList::new(Size::new(100.0, 100.0));
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        let surface = Surface::whole(target);

        let bar = |top: f32, bottom: f32| {
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(10.0, top)));
            path.push(PathCommand::LineTo(Point::new(90.0, top)));
            path.push(PathCommand::LineTo(Point::new(90.0, bottom)));
            path.push(PathCommand::LineTo(Point::new(10.0, bottom)));
            path.push(PathCommand::Close);
            pdf_render::Command::Fill {
                path: std::sync::Arc::new(path),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(pdf_render::Color::BLACK),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }
        };

        // The page transform flips y, so page y 30..40 is device rows 60..70 of 100.
        let alone = super::marked_rows(&[bar(30.0, 40.0)], surface);
        assert!(
            alone.top <= 60 && alone.top >= 58 && alone.height >= 10 && alone.height <= 14,
            "one bar at device rows 60..70 answered {alone:?}"
        );

        // A group's elements carry the extents, so nesting must not widen the answer.
        let grouped = super::marked_rows(
            &[pdf_render::Command::Group {
                commands: vec![bar(30.0, 40.0)],
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
                isolated: true,
                knockout: false,
                alpha_is_shape: true,
                blending: None,
            }],
            surface,
        );
        assert_eq!(alone, grouped, "a group widened its elements' own extent");

        // An empty list marks nothing: one degenerate row, not the surface.
        let nothing = super::marked_rows(&[], surface);
        assert_eq!(nothing, Band { top: 0, height: 1 });

        // A leaf whose extent cannot be measured — an empty path has no bounds — widens
        // the answer to the whole surface, which is `device_bounds`'s documented reading.
        let unbounded = super::marked_rows(
            &[pdf_render::Command::Fill {
                path: std::sync::Arc::new(Path::new()),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(pdf_render::Color::BLACK),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }],
            surface,
        );
        assert_eq!(unbounded, surface.rows, "an unmeasurable leaf must widen");
    }

    /// A strip's matrix is the page's matrix with a whole number of rows taken off, exactly.
    ///
    /// This is the whole of ADR 0219 in one assertion, and it is stated in `f64` so that it is
    /// about the arithmetic rather than about itself: recomputing the subtraction in `f32`
    /// would round the same way twice and prove nothing. Every component but `f` must be the
    /// page's own bits, and `f` must be the page's `f` minus the offset with no rounding at all
    /// — which is what makes composing the offset *last* different from folding it into the
    /// page transform first, where the mark's own translation is then added at another
    /// magnitude and rounds elsewhere.
    #[test]
    fn the_offset_is_composed_last_and_costs_nothing() {
        // A page transform of the shape `TargetSpec::for_page` builds, and mark transforms of
        // the shapes a content stream states: a glyph at 8 and at 20 points, and a form's.
        let pages = [
            Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, 841.89),
            Transform::new(0.838_926_2, 0.0, 0.0, -0.838_926_2, 0.0, 706.283_57),
            Transform::new(2.019_7, 0.0, 0.0, -2.019_7, 0.0, 1_683.783_4),
        ];
        let marks = [
            Transform::IDENTITY,
            Transform::new(8.0, 0.0, 0.0, 8.0, 488.387_54, 51.023_6),
            Transform::new(20.0, 0.0, 0.0, 20.0, 51.023_6, 51.023_6),
            Transform::new(0.5, 0.25, -0.25, 0.5, 133.777, 642.101),
        ];

        for page in pages {
            for mark in marks {
                let whole = super::ToDevice { page, top: 0 }.of(mark);
                for top in [1_u32, 8, 64, 236, 512, 887, 1024] {
                    let strip = super::ToDevice { page, top }.of(mark);
                    assert_eq!(
                        (
                            strip.a.to_bits(),
                            strip.b.to_bits(),
                            strip.c.to_bits(),
                            strip.d.to_bits(),
                            strip.e.to_bits()
                        ),
                        (
                            whole.a.to_bits(),
                            whole.b.to_bits(),
                            whole.c.to_bits(),
                            whole.d.to_bits(),
                            whole.e.to_bits()
                        ),
                        "a row offset moved something other than the vertical translation"
                    );
                    #[expect(
                        clippy::float_cmp,
                        reason = "exactly what is asserted: the subtraction of a whole number \
                                  of rows from a device coordinate rounds not at all, so an \
                                  epsilon here would assert nothing"
                    )]
                    let exact = (f64::from(whole.f) - f64::from(top)) == f64::from(strip.f);
                    assert!(
                        exact,
                        "{top} rows off {whole:?} gave {strip:?}, which is not the page's \
                         translation minus {top}: the subtraction rounded"
                    );
                }
            }
        }
    }

    /// `build_soft_mask`'s two arms give the same answer for every pixel a buffer can hold.
    ///
    /// The function reads a premultiplied buffer and takes [`pdf_render::SoftMask::outside`]
    /// for the wholly transparent pixel instead of demultiplying it and deriving a value. That
    /// is only sound if `tiny-skia`'s own `demultiply` sends `[0, 0, 0, 0]` to `[0, 0, 0, 0]`
    /// — it divides by the alpha, so the answer is a `NaN` that Rust's saturating cast lands on
    /// zero, which is a fact about the dependency and about the cast rather than about this
    /// tree. Asserted rather than reasoned about, over every alpha and both mask kinds, because
    /// what rests on it is 51 s of a 52.6 s page. ADR 0271.
    #[test]
    fn the_transparent_pixels_shortcut_is_the_derivation() {
        use pdf_render::{Color, SoftMask, SoftMaskKind};

        let transparent = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0)
            .expect("all-zero is a valid premultiplied pixel");
        let straight = transparent.demultiply();
        assert_eq!(
            [
                straight.red(),
                straight.green(),
                straight.blue(),
                straight.alpha()
            ],
            [0, 0, 0, 0],
            "demultiplying transparency must give transparency, or the shortcut changes pixels"
        );

        for kind in [
            SoftMaskKind::Alpha,
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(0.0, 0.0, 0.0),
            },
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(1.0, 0.5, 0.0),
            },
        ] {
            let mask = SoftMask {
                commands: Vec::new(),
                kind,
                transfer: None,
            };
            assert_eq!(
                mask.outside(),
                mask.value([
                    straight.red(),
                    straight.green(),
                    straight.blue(),
                    straight.alpha()
                ])
            );
        }
    }

    /// Every premultiplied pixel a buffer can hold reaches the same value either way.
    ///
    /// The test above pins the transparent pixel, which is the one the shortcut *replaces*;
    /// this one walks the other arm over a spread of alphas and colours to say that nothing
    /// else moved when `Pixmap::take_demultiplied` stopped being the route. ADR 0271.
    #[test]
    fn the_per_pixel_route_agrees_with_the_whole_buffers() {
        use pdf_render::{Color, SoftMask, SoftMaskKind};

        let mut buffer = tiny_skia::Pixmap::new(16, 16).expect("a 16x16 pixmap");
        for (index, pixel) in buffer.pixels_mut().iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "index is bounded by 16*16 = 256, so every cast below is exact"
            )]
            let alpha = index as u8;
            // Premultiplied, so no channel may exceed the alpha; `from_rgba` refuses one that
            // does, which is what keeps this fixture a buffer the backend could have produced.
            *pixel =
                tiny_skia::PremultipliedColorU8::from_rgba(alpha / 2, alpha / 3, alpha / 4, alpha)
                    .expect("no channel exceeds the alpha");
        }

        for kind in [
            SoftMaskKind::Alpha,
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(0.2, 0.7, 0.9),
            },
        ] {
            let mask = SoftMask {
                commands: Vec::new(),
                kind,
                transfer: None,
            };
            let outside = mask.outside();
            let per_pixel: Vec<u8> = buffer
                .pixels()
                .iter()
                .map(|pixel| {
                    if pixel.alpha() == 0
                        && pixel.red() == 0
                        && pixel.green() == 0
                        && pixel.blue() == 0
                    {
                        outside
                    } else {
                        let straight = pixel.demultiply();
                        mask.value([
                            straight.red(),
                            straight.green(),
                            straight.blue(),
                            straight.alpha(),
                        ])
                    }
                })
                .collect();
            let whole_buffer = mask.values(&buffer.clone().take_demultiplied());
            assert_eq!(
                per_pixel, whole_buffer,
                "the per-pixel conversion and `take_demultiplied` must agree pixel for pixel"
            );
        }
    }
}

/// Failures specific to the CPU backend.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CpuRasterError {
    /// A pixmap or mask of the requested size could not be allocated.
    #[error("could not allocate a {width}x{height} buffer")]
    Allocation {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// A path was empty or contained non-finite coordinates.
    ///
    /// Reported rather than skipped: silently drawing nothing would hand the
    /// comparison harness a plausible-looking wrong image instead of a failure.
    #[error("path is empty or contains non-finite coordinates")]
    InvalidPath,
    /// A command referenced a clip that is not present in the display list.
    #[error("clip {0:?} is not present in this display list")]
    UnknownClip(ClipId),
    /// A command referenced a soft mask that is not present, or was never evaluated.
    #[error("soft mask {0:?} is not present in this display list")]
    UnknownSoftMask(SoftMaskId),
    /// A clip's parent chain forms a cycle.
    #[error("clip {0:?} is part of a cyclic parent chain")]
    CyclicClip(ClipId),
    /// A command variant this backend does not implement yet.
    #[error("command not supported by the CPU backend: {0}")]
    UnsupportedCommand(String),
    /// A paint variant this backend does not implement yet.
    #[error("paint not supported by the CPU backend: {0}")]
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
    /// A failure originating in the shared backend layer.
    #[error(transparent)]
    Target(#[from] BackendError),
}
