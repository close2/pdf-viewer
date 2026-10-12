//! The page as it is being built, handed to whoever asked for a look.
//!
//! # What this is for
//!
//! A host shows a page when its draw ends. For nearly every page that is the right moment — the
//! median first page of `doc/pdf.js` draws in two milliseconds — and for a page that draws for
//! seconds it is the wrong one: the person looks at nothing, or at the previous size of the page,
//! for the whole of the draw. The project owner, of a valid drawing that draws for six seconds in a
//! window: a page shown "step by step … better than waiting; but not a requirement".
//!
//! [`Glance`] is a flag another thread raises to ask for the picture so far, and [`Glimpse`] is
//! what comes back: a run of the target's rows, finished exactly as the page will be finished —
//! the medium under it, straight alpha — but holding only the marks drawn so far.
//!
//! # Why in painting order rather than in tiles
//!
//! The obvious shape is spatial: cut the target into tiles, draw the visible ones first and show
//! each as it lands, which is how a viewer whose tiles are independent does it. **A tile here is a
//! strip, and a strip may only be cut where the cut draws the same bytes** (ADR 0139): the page
//! this exists for strokes a frame round its whole sheet, and until ADR 1819 that forbade every
//! cut and it was one strip; a stroke drawn a row at a time now forbids none, so the witness is cut
//! into strips and each lands as it finishes. A tile that is not cut at a granted row draws a
//! different picture from the page, and showing that picture and then replacing it is showing a
//! person something the page is not.
//!
//! **Painting order is exact on every page, whatever the cuts.** A display list's commands are
//! independent and drawn in order (`pdf_render::Command`), so the picture after the first *k* of
//! them is a prefix of the page's own drawing, and the page at the end is the same draw it would
//! have been unobserved: the observation copies rows out and writes nothing back. Where the page
//! *does* grant cuts, each strip answers for its own rows, and a strip that has finished answers
//! once more with its final rows — so on such a page the strips land one by one as well. It is the
//! shape a renderer that pauses and resumes inside one page's drawing already has; ADR 1825 is the
//! argument and the measurement.
//!
//! # What a look costs, and who pays
//!
//! Nothing at all where nobody asks: the check is one relaxed load per command at the top of the
//! page, beside the interrupt's. A look that is asked for copies the strip's rows and runs the
//! per-pixel passes the page ends with over the copy, on the drawing thread — so the host decides
//! how often to ask, and the cost of each is stated in ADR 1825 rather than assumed.
//!
//! # What is never glimpsed
//!
//! A page whose finish needs a second plane — §11.4.7's blending space of four components,
//! §10.8.3's separation simulation — or §11.7.5.2's transfer functions, whose shapes are drawn as a
//! list of their own after the page. A copy of the first plane alone would show colours the page will not
//! have (trap 5), so such a page is drawn exactly as before and shown when it ends.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use pdf_render::{DisplayList, MarkBounds, Raster, RasterFormat, TargetSpec};

/// A flag another thread raises to ask a draw in progress for the picture so far.
///
/// **A count rather than a flag**, because a page drawn in strips answers once per strip: each
/// strip remembers the last request it answered, and a request no strip has seen is newer than
/// all of them. Both accesses are `Relaxed` for [`pdf_render::Interrupt`]'s reason — the pixels
/// travel through whatever channel [`Glimpse`]s are sent on, not through this — so a draw nobody
/// is looking at pays one load per command.
#[derive(Debug, Clone, Default)]
pub struct Glance(Arc<AtomicU64>);

impl Glance {
    /// A glance nobody has asked for yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks the draw for its picture at its next command, once per strip.
    pub fn ask(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// How many times it has been asked.
    pub(crate) fn asked(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// A run of the target's rows as they stand part-way through the draw.
#[derive(Debug)]
pub struct Glimpse {
    /// The first row of the target these pixels are.
    pub top: u32,
    /// The rows, the target's width across, finished as the page is finished: the medium under
    /// the marks drawn so far, in [`RasterFormat::Rgba8`]'s straight alpha.
    pub rows: Raster,
    /// How many of the page's commands these rows have been drawn through.
    pub drawn: usize,
    /// How many commands the page has — `drawn == of` is a strip that has finished.
    pub of: usize,
}

/// Where a draw sends what it is asked for, and the flag it is asked through.
#[derive(Clone)]
pub(crate) struct Showing {
    pub(crate) glance: Glance,
    to: Arc<dyn Fn(Glimpse) + Send + Sync>,
}

impl std::fmt::Debug for Showing {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Showing")
            .field("glance", &self.glance)
            .finish_non_exhaustive()
    }
}

impl Showing {
    pub(crate) fn new(glance: Glance, to: impl Fn(Glimpse) + Send + Sync + 'static) -> Self {
        Self {
            glance,
            to: Arc::new(to),
        }
    }

    /// Whether `list` may be glimpsed at all — see the module's last section.
    pub(crate) fn admits(list: &DisplayList) -> bool {
        list.blending().is_none() && list.separation().is_none() && list.transfers().is_none()
    }
}

/// One strip's side of the arrangement: which request it last answered, and the rows it owns.
pub(crate) struct Watch<'a> {
    showing: &'a Showing,
    answered: u64,
    /// The page's own target, which the per-pixel passes measure rows against.
    page: TargetSpec,
    /// The first page row the strip holds.
    top: u32,
    list: &'a DisplayList,
    medium: pdf_render::Medium,
}

impl<'a> Watch<'a> {
    pub(crate) fn new(
        showing: &'a Showing,
        list: &'a DisplayList,
        page: TargetSpec,
        top: u32,
        medium: pdf_render::Medium,
    ) -> Self {
        Self {
            // A request made before the strip began is answered by its first command, which shows
            // the rows the medium alone gives — the page's paper, at the size it will be.
            answered: 0,
            showing,
            page,
            top,
            list,
            medium,
        }
    }

    /// Whether a look has been asked for that this strip has not answered.
    pub(crate) fn due(&self) -> bool {
        self.showing.glance.asked() != self.answered
    }

    /// Answers every request so far with the strip's rows as they stand.
    ///
    /// `premultiplied` is the strip's pixmap, `tiny-skia`'s premultiplied bytes; the passes run
    /// over a copy in the order [`crate::CpuRasterizer`]'s `rasterize` runs them over the page,
    /// so a strip that has finished sends exactly the rows the page will be.
    pub(crate) fn answer(&mut self, premultiplied: &[u8], drawn: usize, of: usize) {
        self.answered = self.showing.glance.asked();
        let width = self.page.width;
        let stride = (width as usize).saturating_mul(4).max(4);
        let Some(height) = u32::try_from(premultiplied.len().div_euclid(stride)).ok() else {
            return;
        };
        let mut copy = premultiplied.to_vec();
        if let Some(curve) = self.list.grey_curve() {
            pdf_render::resolve_grey(&mut copy, curve);
        }
        if let Some(cube) = self.list.colour_cube() {
            pdf_render::resolve_cube(&mut copy, cube);
        }
        if let Some(crop) = pdf_render::crop_area(self.list, self.page) {
            pdf_render::crop_to_page(&mut copy, width, self.top, crop);
        }
        let area = pdf_render::page_area(self.list, self.page);
        pdf_render::impose_within(&mut copy, width, self.top, area, self.medium);
        // The page's own conversion, by way of the same type, so that a strip's last glimpse and
        // the page's rows cannot be two arithmetics.
        let Some(data) = tiny_skia::IntSize::from_wh(width, height)
            .and_then(|size| tiny_skia::Pixmap::from_vec(copy, size))
            .map(tiny_skia::Pixmap::take_demultiplied)
        else {
            return;
        };
        (self.showing.to)(Glimpse {
            top: self.top,
            rows: Raster {
                width,
                height,
                format: RasterFormat::Rgba8,
                data,
            },
            drawn,
            of,
        });
    }
}

impl crate::CpuRasterizer {
    /// Draws the page's own commands onto a strip of it, answering `showing`'s glances between
    /// them.
    ///
    /// **One command at a time through the same loop**, which is exact rather than close:
    /// [`crate::CpuRasterizer`]'s `encode` carries nothing from one command to the next but the
    /// mask cache it is handed, so the list drawn a command at a time and the list drawn whole are
    /// the same calls in the same order. Only the page's top level is observed — a group's
    /// elements are drawn onto a buffer of their own and composited at its end, so a picture taken
    /// inside one would be a picture of no state the page passes through.
    ///
    /// **Whether a command reaches the strip is asked of `bounds`**, measured once for the page's
    /// target rather than once per strip: the test is the one `encode` asks of a list met in a
    /// recursion, and every strip of the owner's witness replayed it for each of 4.4 M commands,
    /// 9.9 % of the processor's drawing (ADR 1841).
    ///
    /// # Errors
    ///
    /// As `encode`.
    pub(crate) fn encode_page(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        (list, bounds): (&DisplayList, &MarkBounds),
        surface: crate::Surface,
        masks: &mut crate::MaskCache,
        showing: Option<&Showing>,
    ) -> Result<(), crate::CpuRasterError> {
        let commands = list.commands();
        #[expect(
            clippy::cast_precision_loss,
            reason = "a target height below MAX_EXTENT = 2^24 is exact in f32"
        )]
        let (top, bottom) = (
            surface.rows.top as f32,
            surface.rows.top.saturating_add(surface.rows.height) as f32,
        );
        let mut watch = showing
            .map(|showing| Watch::new(showing, list, surface.page, surface.rows.top, self.medium));
        for (index, command) in commands.iter().enumerate() {
            if let Some(watch) = watch.as_mut()
                && watch.due()
            {
                watch.answer(pixmap.as_ref().data(), index, commands.len());
            }
            // `encode`'s first two questions, in its order: whether anybody has asked for the
            // thread back, and whether this command can mark any row of the strip.
            if self.interrupted() {
                return Err(pdf_render::BackendError::Interrupted.into());
            }
            if bounds.misses_rows(index, surface.page.transform, top, bottom) {
                continue;
            }
            self.encode_command(
                pixmap,
                list,
                command,
                surface,
                masks,
                0,
                crate::Compose::Over,
            )?;
        }
        // The strip has landed; somebody looking at the page is shown its final rows now rather
        // than when the slowest strip ends.
        if let (Some(showing), Some(watch)) = (showing, watch.as_mut())
            && showing.glance.asked() > 0
        {
            watch.answer(pixmap.as_ref().data(), commands.len(), commands.len());
        }
        Ok(())
    }
}

/// A page drawn from a display list handed over in parts while it is still being read
/// (ADR 1855): each part painted into one target as it arrives, the picture so far shown on
/// request, and the page finished from the list once the read has ended.
///
/// # Why its pixels are the page's
///
/// **The same commands, in the same order, into the same target, through the same loop.** A
/// display list's top-level commands are drawn one after the other and carry nothing from one to
/// the next but the mask cache (this module's head), so drawing the first *k* now and the rest
/// later is the whole draw with a pause in it. What the whole draw does that a part cannot see is
/// stated and refused rather than approximated: a soft mask is built against every consumer's
/// reach on the whole list (`mask_consumer_reach`), so a part bringing one ends the drawing ahead;
/// a page this module does not glimpse (§11.4.7's second plane, §10.8.3, §11.7.5.2) is finished
/// whole; and a part whose commands the reader took back is never handed over at all
/// (`pdf_model::progress`). [`Underway::finish`] answers `Err` for each of those, and the caller
/// draws the finished list whole, as it would have without any of this.
///
/// **One surface, not strips.** The whole draw cuts a page into strips only at rows where a cut
/// draws the same bytes (ADR 0139), so the page drawn as one surface is the page drawn in strips;
/// a part is too small a unit of work to divide, and one thread keeps up with the reader on the
/// witness (ADR 1845 §4).
pub struct Underway {
    rasterizer: crate::CpuRasterizer,
    target: TargetSpec,
    pixmap: tiny_skia::Pixmap,
    masks: crate::MaskCache,
    /// The clips and soft masks of the list so far, and none of its commands: what the parts'
    /// identifiers name.
    tables: Option<DisplayList>,
    /// How much of the list has been drawn.
    drawn: pdf_render::Stated,
    /// What the groups drawn so far blitted, against `pdf_render::MAX_GROUP_BLIT_PIXELS`.
    blitted: u64,
    /// Why this can no longer become the page, once it cannot.
    stopped: Option<&'static str>,
}

/// Written by hand because the mask cache is built pixels and has nothing worth printing.
impl std::fmt::Debug for Underway {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Underway")
            .field("target", &self.target)
            .field("drawn", &self.drawn)
            .field("stopped", &self.stopped)
            .finish_non_exhaustive()
    }
}

impl Underway {
    /// A target nothing has been drawn into yet, drawn with `rasterizer`'s settings — `None`
    /// where the target is larger than this backend draws or cannot be allocated.
    #[must_use]
    pub fn new(rasterizer: &crate::CpuRasterizer, target: TargetSpec) -> Option<Self> {
        if target.width > crate::MAX_EXTENT || target.height > crate::MAX_EXTENT {
            return None;
        }
        let surface = crate::Surface::whole(target);
        Some(Self {
            rasterizer: rasterizer.clone(),
            target,
            pixmap: tiny_skia::Pixmap::new(target.width, target.height)?,
            masks: crate::MaskCache::new(surface, rasterizer.anti_alias, crate::MASK_BUDGET),
            tables: None,
            drawn: pdf_render::Stated::default(),
            blitted: 0,
            stopped: None,
        })
    }

    /// The target the page is being drawn at.
    #[must_use]
    pub fn target(&self) -> TargetSpec {
        self.target
    }

    /// How much of the list has been drawn.
    #[must_use]
    pub fn drawn(&self) -> pdf_render::Stated {
        self.drawn
    }

    /// Why this can no longer become the page, once it cannot.
    #[must_use]
    pub fn stopped(&self) -> Option<&'static str> {
        self.stopped
    }

    /// Draws the next part of the list, and answers whether this can still become the page.
    pub fn take(&mut self, mut part: pdf_render::ListPart) -> bool {
        if self.stopped.is_some() {
            return false;
        }
        if part.from != self.drawn {
            return self.stop("a part arrived out of its order");
        }
        if part.soft_mask_count() > 0 {
            return self.stop("the page states a soft mask, which is built against the whole list");
        }
        let tables = self
            .tables
            .get_or_insert_with(|| DisplayList::new(part.page_size));
        if !tables.extend_tables(&part) {
            return self.stop("a part's clips do not line up with the ones drawn before it");
        }
        // What the finished list's own pass restates, restated here command by command, so that
        // a part is drawn as the finished list states it (`pdf_render::ListPart`).
        part.restate_collapsed_marks();
        self.blitted = self
            .blitted
            .saturating_add(pdf_render::commands_blit_demand(
                tables,
                &part.commands,
                self.target,
            ));
        if self.blitted > pdf_render::MAX_GROUP_BLIT_PIXELS {
            return self.stop("the page's groups blit more than a page may");
        }
        let drawn = self.rasterizer.encode(
            &mut self.pixmap.as_mut(),
            tables,
            &part.commands,
            crate::Surface::whole(self.target),
            &mut self.masks,
            0,
            crate::Compose::Over,
        );
        if drawn.is_err() {
            return self.stop("a part did not draw");
        }
        self.drawn = part.until();
        true
    }

    /// The page as it stands, finished as the page will be finished — or `None` before anything
    /// has been drawn or once this has stopped.
    #[must_use]
    pub fn look(&self) -> Option<Raster> {
        let tables = self.tables.as_ref().filter(|_| self.stopped.is_none())?;
        let mut copy = self.pixmap.clone();
        finish_rows(copy.data_mut(), tables, self.target, self.rasterizer.medium);
        Some(raster_of(copy, self.target))
    }

    /// The page, finished from `list` — the list the parts were handed from, once its read has
    /// ended — or `Err` with why it cannot be, where the caller draws `list` whole instead.
    ///
    /// # Errors
    ///
    /// Every reason in this type's own documentation, each named.
    pub fn finish(mut self, list: &DisplayList) -> Result<Raster, &'static str> {
        if let Some(reason) = self.stopped {
            return Err(reason);
        }
        if !Showing::admits(list) || list.grey_curve().is_some() || list.colour_cube().is_some() {
            return Err("the page is finished with a plane, a curve or a channel of its own");
        }
        if list.soft_mask_count() > 0 {
            return Err("the page states a soft mask, which is built against the whole list");
        }
        let stated = list.stated();
        if stated.commands < self.drawn.commands || stated.clips < self.drawn.clips {
            return Err("the list is shorter than what was drawn from it");
        }
        if pdf_render::check_group_blit(list, self.target).is_err() {
            return Err("the page's groups blit more than a page may");
        }
        let rest = list
            .commands()
            .get(self.drawn.commands..)
            .unwrap_or_default();
        self.rasterizer
            .encode(
                &mut self.pixmap.as_mut(),
                list,
                rest,
                crate::Surface::whole(self.target),
                &mut self.masks,
                0,
                crate::Compose::Over,
            )
            .map_err(|_| "the rest of the list did not draw")?;
        finish_rows(
            self.pixmap.data_mut(),
            list,
            self.target,
            self.rasterizer.medium,
        );
        Ok(raster_of(self.pixmap, self.target))
    }

    fn stop(&mut self, reason: &'static str) -> bool {
        self.stopped = Some(reason);
        false
    }
}

/// The per-pixel passes `CpuRasterizer::rasterize` ends a page with on a list this module
/// admits — §14.11.2.1's crop, then §11.4.7's medium under the marks — over the whole target.
///
/// Serial where the whole draw splits them by rows; a per-pixel pass is the same bytes either
/// way, which is the rule that lets the whole draw split it.
fn finish_rows(
    premultiplied: &mut [u8],
    list: &DisplayList,
    target: TargetSpec,
    medium: pdf_render::Medium,
) {
    if let Some(crop) = pdf_render::crop_area(list, target) {
        pdf_render::crop_to_page(premultiplied, target.width, 0, crop);
    }
    let area = pdf_render::page_area(list, target);
    pdf_render::impose_within(premultiplied, target.width, 0, area, medium);
}

/// A finished pixmap as the [`Raster`] every backend hands over: straight alpha, by
/// `tiny-skia`'s own conversion, as `rasterize` converts.
fn raster_of(pixmap: tiny_skia::Pixmap, target: TargetSpec) -> Raster {
    Raster {
        width: target.width,
        height: target.height,
        format: RasterFormat::Rgba8,
        data: pixmap.take_demultiplied(),
    }
}
