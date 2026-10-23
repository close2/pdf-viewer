//! CPU rasteriser backend; the correctness oracle for the GPU backend.
//!
//! Implements [`pdf_render::Rasterizer`] on the CPU, over its own scan converter with
//! `tiny-skia` beneath it. Page one goes to the graphics device by default (`CLAUDE.md`
//! principle 2), and this backend has the jobs that principle leaves it:
//!
//! 1. It is the **reference** against which the device backends, `render-raster` and
//!    `render-gpu`, are validated. Every backend consumes the same
//!    [`pdf_render::DisplayList`], so any difference between their outputs is a backend
//!    defect rather than a difference in how the document was interpreted — a far tighter
//!    test than comparing against another PDF viewer, where antialiasing and colour handling
//!    differ for legitimate reasons.
//! 2. It draws a frame the device refuses — a coverage or budget refusal, not a swapchain
//!    state — said out loud.
//! 3. It draws a page past what a frame was budgeted to be where it is predicted the faster
//!    lane, page one included (ADR 1831); never a frame drawn only to cover the device's
//!    bring-up.
//!
//! A window started with `--cpu` makes no device and draws every frame here, which is a
//! person's diagnostic and the answer to a driver that will not load (ADR 0221).
//!
//! Correctness outranks speed in this crate: where the two conflict, this backend takes the
//! clearer construction, because every cross-backend comparison rests on it. See
//! `doc/adr/0002-cpu-rasteriser-first.md`.
//!
//! # Where each part is
//!
//! [`CpuRasterizer`] and its [`Rasterizer`] implementation are this file. A target is cut into
//! strips by `strips`, each strip walks the list in `encode`, and each command is drawn by a
//! kernel: `fill`, `stroke` (with `stroke::folding` and `stroke::sub_pixel_rule`), `thin_segment`
//! and `image`, scan-converted by `scan` and `area`, shaded by `shading`. Groups are `groups`,
//! soft masks `soft_mask`, and the masks a strip keeps `mask_cache`; how an element composites is
//! `compose` and `blend`, and the rows a draw may mark are `surface`. `glimpse` shows a page as it
//! is built and hands a page drawn ahead over to the window, and `reduced_images` shares an
//! image's reduction across strips and redraws. ADR 1914 records the division.

#![forbid(unsafe_code)]

mod area;
mod blend;
mod compose;
mod convert;
mod encode;
mod fill;
mod glimpse;
mod groups;
mod image;
mod mask_cache;
mod reduced_images;
mod scan;
mod shading;
mod soft_mask;
mod strips;
mod stroke;
mod surface;
mod thin_segment;

use rayon::iter::{IndexedParallelIterator as _, ParallelIterator as _};
use rayon::slice::ParallelSliceMut as _;
use std::sync::Arc;

pub use area::{UnionCondition, UnionDecline};
pub use glimpse::{Glance, Glimpse, Underway};
pub use strips::MAX_STRIPS;
use strips::{PIXEL_PASS_ROWS, drawn_row_by_row, plan_strips};
pub use surface::Shown;
use surface::Surface;

use pdf_render::{
    BackendError, ClipId, DisplayList, Interrupt, MAX_EXTENT, Medium, Paint, Raster, RasterFormat,
    Rasterizer, Selection, SoftMaskId, TargetSpec, Transform,
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
/// eight-bit mask is not the cause; ADR 0418.
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
    /// The rows a caller stated the target is cut at, in place of the planner's — [`Self::with_cuts`].
    cuts: Option<Arc<[u32]>>,
    interrupt: Option<Interrupt>,
    /// Where the page as it is being built goes when somebody asks for a look — [`Self::showing`].
    showing: Option<glimpse::Showing>,
    /// Deeply reduced image samples, so that the strips of one draw — and the draws of one
    /// rasteriser — pay for a reduction once between them. See [`reduced_images`] for what is
    /// shared and why an address is a sound key for it.
    ///
    /// **Held here rather than made per draw, and that is the liveness rule**: a rasteriser
    /// kept across frames keeps its reductions, which is what `viewer-confined`'s worker does,
    /// and one made per job keeps nothing, which is what `viewer_host::drawing` does
    /// deliberately. Behind an `Arc` because `Clone` on this type means another handle on the
    /// same rasteriser rather than a second one that has to warm up again.
    reduced_images: Arc<reduced_images::ReducedImages>,
    /// What the last draw declined to the library's converter — [`Self::declined`].
    declined: Vec<UnionDecline>,
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
            cuts: None,
            interrupt: None,
            showing: None,
            reduced_images: Arc::default(),
            declined: Vec::new(),
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
    /// The flag is read once per command in `CpuRasterizer::encode`, which is the loop nothing
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

    /// Hands the rasteriser a flag another thread raises to see the page as it is being built, and
    /// where to send what it sees.
    ///
    /// Each time `glance` is asked, every strip of the page answers at its next command with its
    /// rows as they stand, finished the way the page will be, and a strip that ends while anybody
    /// has asked answers once more with its final rows. The draw itself is the same draw: a look
    /// copies rows out and writes nothing back. A page this cannot show truthfully is drawn
    /// without answering — the `glimpse` module's comment says which, and ADR 1825 why the unit is
    /// painting order rather than a tile.
    #[must_use]
    pub fn showing(mut self, glance: Glance, to: impl Fn(Glimpse) + Send + Sync + 'static) -> Self {
        self.showing = Some(glimpse::Showing::new(glance, to));
        self
    }

    /// Asks for a fixed number of horizontal strips instead of one per thread of the pool.
    ///
    /// **This changes how long a page takes and almost nothing about what it looks like**, which
    /// is `Surface`'s claim and is the reason this method exists: the property is only
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

    /// The rows this rasteriser cuts `target` at for `list`: the first `0`, the last the target's
    /// height, and a single pair where it draws the page as one strip.
    ///
    /// The plan [`Rasterizer::rasterize`] draws by, for an instrument or a test that has to know
    /// whether a page it compares was cut at all — a comparison of a page drawn whole and in
    /// strips that were never made says nothing (ADR 1819).
    #[must_use]
    pub fn strip_plan(&self, list: &DisplayList, target: TargetSpec) -> Vec<u32> {
        self.selection_plan(Selection::whole(list), target)
    }

    /// [`Self::strip_plan`] of a selection of a list's commands — a window's, which the planner
    /// reads as a list of those commands alone (ADR 1980).
    pub(crate) fn selection_plan(&self, drawn: Selection<'_>, target: TargetSpec) -> Vec<u32> {
        match self.stated_cuts(target) {
            Some(cuts) => cuts.to_vec(),
            None => plan_strips(drawn, target, self.strips, self.anti_alias),
        }
    }

    /// Cuts every target of `cuts`'s height at exactly these rows instead of the rows the planner
    /// would choose: the first `0`, the last the target's height, each above the one before.
    ///
    /// **For a drawing that has to be compared with one made at the same rows**: a page drawn
    /// ahead in strips while it is read ([`Underway::in_strips`]) is the page drawn whole at its
    /// cuts byte for byte, and this is how the whole page is drawn at them. The rows are taken as
    /// stated, whether or not a mark forbids one — the planner's question is the caller's to have
    /// asked. A target of another height, or rows that are not such a list, are cut by the
    /// planner as though nothing had been stated.
    #[must_use]
    pub fn with_cuts(mut self, cuts: &[u32]) -> Self {
        self.cuts = Some(cuts.into());
        self
    }

    /// The rows [`Self::with_cuts`] stated, where they are a division of `target`.
    fn stated_cuts(&self, target: TargetSpec) -> Option<&[u32]> {
        self.cuts.as_deref().filter(|cuts| {
            cuts.first() == Some(&0)
                && cuts.last() == Some(&target.height)
                && cuts.len() >= 2
                && cuts.windows(2).all(|pair| pair.first() < pair.get(1))
        })
    }

    /// Whether this rasteriser draws `mark` a row at a time on `target`, so that it forbids no
    /// cut — its answer to [`pdf_render::unsplittable_rows_where`]'s question, which
    /// [`CpuRasterizer::strip_plan`] asks of every mark.
    ///
    /// For an instrument that has to say which marks forbid a page's rows, and why (ADR 1837).
    #[must_use]
    pub fn draws_row_by_row(&self, mark: pdf_render::PlannedMark<'_>, target: TargetSpec) -> bool {
        drawn_row_by_row(mark, self.anti_alias, target)
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

    /// The marks the last draw — [`Rasterizer::rasterize`] or [`Self::rasterize_window`] — left to
    /// the library's converter because the exact measure of their area declined them: which
    /// condition, how many pieces and where, each mark once (ADR 2280).
    ///
    /// **Drawn, and said**: the library's converter draws such a mark at its own sixteenths, so the
    /// page looks finished, and nothing but this says it is not measured as §10.7.4's area is —
    /// the silent fallback `CLAUDE.md`'s first principle forbids. Empty after a draw that declined
    /// nothing and after one that failed. [`UnionDecline`]'s `Display` is the sentence a host
    /// says it with; [`UnionCondition::is_defect`] names the two that no input should reach.
    #[must_use]
    pub fn declined(&self) -> &[UnionDecline] {
        &self.declined
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

/// Refuses what is shown where its far edge lies past `MAX_EXTENT`.
///
/// Checked rather than assumed, because [`surface::Band`] and [`surface::ToDevice`] convert a row
/// or a column index to `f32` and that is lossless only below 2^24. `TargetSpec::for_page` already
/// enforces this for a page, but the struct's fields are public, so a hand-built spec can violate
/// it and would misplace every banded command if it did; and a window's far edge is its corner
/// and its extent together (ADR 2132). For a page, whose corner is the origin, that is its extent.
fn within_extent(shown: Shown) -> Result<(), CpuRasterError> {
    for (first, extent) in [(shown.left, shown.width), (shown.top, shown.height)] {
        let far = u64::from(first).saturating_add(u64::from(extent));
        if far > u64::from(MAX_EXTENT) {
            return Err(BackendError::ExtentTooLarge {
                extent: far,
                limit: MAX_EXTENT,
            }
            .into());
        }
    }
    Ok(())
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
        self.draw_shown(list, Surface::whole(target))
    }
}

impl CpuRasterizer {
    /// The part `window` of the page `page` is the target of — a window of a page drawn zoomed
    /// past what a toolkit copies whole (ADR 1930) — drawn on the page's own device grid: the
    /// same pixels as `page` drawn whole has there, as [`Rasterizer::rasterize`] of
    /// `window`'s own target would draw them only up to the arithmetic of another origin.
    ///
    /// §10.7.4 identifies a pixel by the device coordinates its corner has, and a window's corner
    /// is a whole number of the page's pixels; so the window's first column and row are taken off
    /// each mark's device geometry last, where that subtraction is exact, as a strip's first row
    /// is (ADRs 0219, 2132), and every converter is the one the page chooses (ADR 2116).
    ///
    /// # Errors
    ///
    /// As [`Rasterizer::rasterize`], and [`BackendError::ExtentTooLarge`] for a window whose far
    /// edge lies past `MAX_EXTENT`.
    pub fn rasterize_window(
        &mut self,
        list: &DisplayList,
        page: TargetSpec,
        window: Shown,
    ) -> Result<Raster, CpuRasterError> {
        self.draw_shown(list, Surface::showing(page, window))
    }

    /// [`Rasterizer::rasterize`] of what `whole` shows: the page, or a window of it. `target` below
    /// is what that is to everything but a mark's geometry — the page's own target, for a page.
    fn draw_shown(&mut self, list: &DisplayList, whole: Surface) -> Result<Raster, CpuRasterError> {
        let target = whole.shown.target(whole.page);
        self.declined.clear();
        // Before the pixmap, because a target may be a gibibyte and an interrupt already raised
        // is an allocation nobody wants the answer to.
        if self.interrupted() {
            return Err(BackendError::Interrupted.into());
        }

        within_extent(whole.shown)?;

        // ISO 32000-2 §10.7.4 paints a pixel a shape's area meets, and a mark whose bound misses
        // every pixel of the target meets none: a target that shows part of a page — a window
        // at a zoom — is drawn from the commands that reach it, and is the same pixels as the
        // page's whole list drawn there. What changes is what the strip planner below is told,
        // which no longer counts the rows a mark outside the window would have forbidden
        // (ADR 1841). `None` where every command reaches, which is every page-sized target. The
        // window names the list's own commands, so every pass below reads the list (ADR 1980).
        let window = list.reaching(target);
        let drawn = Selection::drawn(list, window.as_ref());

        // Before the pixmap for the same reason the interrupt is: refusing a page that would
        // spend minutes inside `draw_group` is worth nothing if the refusal is paid for after
        // the allocation. `pdf_render::check_group_blit` reads the list and draws nothing —
        // see `pdf_render::group_cost` for the measure, the census that sized it and the page
        // that made it necessary.
        pdf_render::check_selected_blit(drawn, target)?;

        let mut pixmap = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
            CpuRasterError::Allocation {
                width: target.width,
                height: target.height,
            },
        )?;
        // Not filled with the background: §11.4.7 makes the page group *isolated*, so the
        // page's elements composite onto transparency and the medium's colour is applied to
        // the result — see `impose_on_medium`, which both backends end with.

        let showing = self
            .showing
            .as_ref()
            .filter(|_| glimpse::Showing::admits(list));
        let mut declined = self.encode_in_strips_watched(&mut pixmap, drawn, whole, showing)?;
        // One mark crosses one strip's rows at most where it is a folding stroke's (ADR 2253),
        // but a soft mask's group is drawn once for each surface that reads it.
        declined.sort_unstable();
        declined.dedup();
        self.declined = declined;

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
            self.encode_in_strips(&mut ink, black, whole)?;
            match list.separation() {
                // ISO 32000-2 §10.8.3, under a reader's request for the simulation on a page naming
                // a spot colourant: the pair above is the simulated press's process separation,
                // and every spot plane is drawn beside it and multiplied in by steps b) to d),
                // before the medium as the pair's own conversion is (`pdf_render::separation`,
                // ADR 1317). Each plane is a page of its own for `check_group_blit`'s budget.
                Some(separation) => {
                    let spots = self.spot_planes(separation, whole)?;
                    let spots: Vec<&[u8]> = spots.iter().map(tiny_skia::Pixmap::data).collect();
                    pdf_render::resolve_separation(
                        pixmap.data_mut(),
                        ink.data(),
                        &spots,
                        space,
                        separation,
                    );
                }
                None => pdf_render::resolve_blending(pixmap.data_mut(), ink.data(), space),
            }
        }
        // §10.8.3 on a page whose group composites in one or three components: the list is its
        // own process separation, so step b)'s matte goes under it in its own components here,
        // before the curve or the cube below takes it out of them (`pdf_render::separation`,
        // ADR 1329).
        let beside = list.separation().filter(|_| list.blending().is_none());
        if beside.is_some() {
            pdf_render::separation_matte(pixmap.data_mut());
        }
        // The same clause for a space of one component that reaches the device through a
        // curve (`CalGray`, an `ICCBased` 'GRAY' profile): the page composited its component
        // in every channel, and the curve is applied here, at the same point — before the
        // medium. See `pdf_render::blending::GreyCurve`.
        if let Some(curve) = list.grey_curve() {
            pdf_render::resolve_grey(pixmap.data_mut(), curve);
        }
        // And for a space of three CIE-based components (`CalRGB`, an `ICCBased` 'RGB '
        // profile): the page composited the space's components, and the cube they leave by
        // is applied at the same point. See `pdf_render::blending::ColourCube`.
        if let Some(cube) = list.colour_cube() {
            pdf_render::resolve_cube(pixmap.data_mut(), cube);
        }
        // And the spot planes multiplied into the device colour the process separation became —
        // steps b) to d), before the medium as the pair's are.
        if let Some(separation) = beside {
            let spots = self.spot_planes(separation, whole)?;
            let spots: Vec<&[u8]> = spots.iter().map(tiny_skia::Pixmap::data).collect();
            pdf_render::resolve_separation_over_device(pixmap.data_mut(), &spots, separation);
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

        let mut data = pixmap.take_demultiplied();

        // ISO 32000-2 §11.7.5.2's transfer function, last of all because §11.7.5.3's NOTE puts it
        // there: "the current halftone and transfer function, whose values are used only when all
        // colour compositing has been completed and rasterization is being performed". The clause
        // chooses the function by the topmost elementary object covering a point rather than by
        // the colour under it, so `pdf_model` leaves it off the colours and hands the marks'
        // shapes over on the list instead; `pdf_render::resolve_transfers` is the one statement of
        // what that means, which is what keeps the two backends from answering it apart (trap 2).
        //
        // Nothing runs here on a list that states no transfer, which is all but one of the corpus
        // (`pdf-model`'s `examples/transfer_function_census`).
        if let Some(channel) = list.transfers() {
            let coverage_of = |shapes: &DisplayList| -> Result<Vec<u8>, CpuRasterError> {
                // Onto transparency and without the medium, so the alpha read back *is*
                // §11.7.5.2's object shape — the same path the page itself took above, minus the
                // passes that put a colour under it.
                let mut coverage = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
                    CpuRasterError::Allocation {
                        width: target.width,
                        height: target.height,
                    },
                )?;
                self.encode_in_strips(&mut coverage, shapes, whole)?;
                Ok(coverage.take())
            };
            pdf_render::resolve_transfers(channel, list, target, &mut data, coverage_of)?;
        }

        Ok(Raster {
            width: target.width,
            height: target.height,
            format: RasterFormat::Rgba8,
            // `tiny-skia` stores premultiplied alpha internally; `Raster` is documented
            // as straight alpha, so the conversion happens here at the backend boundary
            // rather than being left to every consumer to remember.
            data,
        })
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
