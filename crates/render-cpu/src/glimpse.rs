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

use rayon::iter::{
    IndexedParallelIterator as _, IntoParallelRefMutIterator as _, ParallelIterator as _,
};
use rayon::slice::ParallelSliceMut as _;

use pdf_render::{DisplayList, MarkBounds, Raster, RasterFormat, Selection, TargetSpec};

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
    /// The commands drawn are the selection's from position `from` on, which is how a page drawn
    /// ahead from its parts is finished from the list once the read has ended
    /// ([`Underway::finish`]). A window's selection is asked of `bounds` by each command's index
    /// into the list and watched by its position in the window (ADR 1980).
    ///
    /// # Errors
    ///
    /// As `encode`.
    pub(crate) fn encode_page(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        (drawn, bounds, from): (Selection<'_>, &MarkBounds, usize),
        surface: crate::surface::Surface,
        masks: &mut crate::mask_cache::MaskCache,
        showing: Option<&Showing>,
    ) -> Result<(), crate::CpuRasterError> {
        let list = drawn.list();
        let length = drawn.len();
        #[expect(
            clippy::cast_precision_loss,
            reason = "a target height below MAX_EXTENT = 2^24 is exact in f32"
        )]
        let (top, bottom) = (
            surface.rows.top as f32,
            surface.rows.top.saturating_add(surface.rows.height) as f32,
        );
        // The per-pixel passes measure rows of what the rasterisation shows, so a window's strip
        // is watched as rows of the window's own target (ADR 2132).
        let mut watch = showing.map(|showing| {
            Watch::new(
                showing,
                list,
                surface.shown.target(surface.page),
                surface.rows.top.saturating_sub(surface.shown.top),
                self.medium,
            )
        });
        for (position, (index, command)) in drawn.indexed().enumerate().skip(from) {
            if let Some(watch) = watch.as_mut()
                && watch.due()
            {
                watch.answer(pixmap.as_ref().data(), position, length);
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
                crate::compose::Compose::Over,
            )?;
        }
        // The strip has landed; somebody looking at the page is shown its final rows now rather
        // than when the slowest strip ends.
        if let (Some(showing), Some(watch)) = (showing, watch.as_mut())
            && showing.glance.asked() > 0
        {
            watch.answer(pixmap.as_ref().data(), length, length);
        }
        Ok(())
    }
}

/// A page drawn from a display list handed over in parts while it is still being read
/// (ADR 1855): each part painted into the target as it arrives, the picture so far shown on
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
/// # One surface, or strips on the pool
///
/// [`Underway::new`] paints one surface on the calling thread, and its page is the page drawn
/// whole as one strip. [`Underway::in_strips`] cuts the target into strips as wide as the pool,
/// draws each part into every strip on the pool and finishes the rest of the list there too, and
/// its page is the page drawn whole at those cuts ([`crate::CpuRasterizer::with_cuts`]) — the
/// strips of one draw, where the whole draw would choose its own. A cut is a row no mark of the
/// page forbids, asked of every part as the planner asks it of the whole list (ADRs 0139, 1819,
/// 1837): a part that forbids one ends the drawing ahead, because a strip cut across such a mark
/// draws another picture, and the page is then drawn whole. The cuts are even rather than balanced
/// by the marks' costs, because the cost of a part not yet read is not known; the whole list is
/// balanced by the planner when nothing was drawn ahead. ADR 1896 is the measurement: finishing
/// the owner's drawing on one thread took longer than drawing it whole on the pool after the read.
pub struct Underway {
    rasterizer: crate::CpuRasterizer,
    target: TargetSpec,
    /// The rows the target is cut at: the first `0`, the last its height.
    cuts: Vec<u32>,
    /// One per pair of cuts, top to bottom.
    strips: Vec<Strip>,
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

/// A page drawn ahead and finished: [`Underway::finish`]'s answer.
#[derive(Debug)]
pub struct Finished {
    /// The page.
    pub raster: Raster,
    /// The marks its draw left to the library's converter, each once (ADR 2280).
    pub declined: Vec<crate::UnionDecline>,
}

/// One strip of a page drawn ahead: its rows, their pixels and the masks built for them.
struct Strip {
    rows: crate::surface::Band,
    pixmap: tiny_skia::Pixmap,
    masks: crate::mask_cache::MaskCache,
}

/// Written by hand because the mask cache is built pixels and has nothing worth printing.
impl std::fmt::Debug for Underway {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Underway")
            .field("target", &self.target)
            .field("cuts", &self.cuts)
            .field("drawn", &self.drawn)
            .field("stopped", &self.stopped)
            .finish_non_exhaustive()
    }
}

impl Underway {
    /// A target nothing has been drawn into yet, drawn with `rasterizer`'s settings as one
    /// surface on the calling thread — `None` where the target is larger than this backend draws
    /// or cannot be allocated.
    #[must_use]
    pub fn new(rasterizer: &crate::CpuRasterizer, target: TargetSpec) -> Option<Self> {
        Self::cut_at(rasterizer, target, vec![0, target.height])
    }

    /// [`Self::new`], cut into as many even strips as the pool this is called on is wide — or as
    /// `rasterizer` states with `with_strips` — within [`crate::strips::MAX_STRIPS`] and the fewest
    /// rows a strip may have, each part drawn into them on the pool.
    ///
    /// For a window's first frame of a page that reads for seconds, which a thread of its own
    /// would finish later than the pool draws the page whole after the read (ADR 1896).
    #[must_use]
    pub fn in_strips(rasterizer: &crate::CpuRasterizer, target: TargetSpec) -> Option<Self> {
        let asked = rasterizer.strips.unwrap_or_else(|| {
            rayon::current_num_threads()
                .try_into()
                .unwrap_or(crate::strips::MAX_STRIPS)
        });
        let count = asked
            .min(crate::strips::MAX_STRIPS)
            .min(target.height / crate::strips::MIN_STRIP_ROWS)
            .max(1);
        let cuts = (0..=count)
            .map(|strip| {
                let row = u64::from(target.height)
                    .saturating_mul(u64::from(strip))
                    .checked_div(u64::from(count))
                    .unwrap_or(0);
                u32::try_from(row).unwrap_or(target.height)
            })
            .collect();
        Self::cut_at(rasterizer, target, cuts)
    }

    /// A target cut at `cuts`, which are a division of its height.
    fn cut_at(
        rasterizer: &crate::CpuRasterizer,
        target: TargetSpec,
        cuts: Vec<u32>,
    ) -> Option<Self> {
        if target.width > pdf_render::MAX_EXTENT || target.height > pdf_render::MAX_EXTENT {
            return None;
        }
        let pairs = cuts.len().saturating_sub(1).max(1);
        // Divided as the whole draw divides it: a strip's masks are a strip tall.
        let budget = crate::mask_cache::MASK_BUDGET
            .checked_div(pairs)
            .unwrap_or(crate::mask_cache::MASK_BUDGET);
        let strips = cuts
            .windows(2)
            .map(|pair| {
                let (top, bottom) = (pair.first().copied()?, pair.get(1).copied()?);
                let rows = crate::surface::Band {
                    top,
                    height: bottom.checked_sub(top)?,
                };
                let surface = crate::surface::Surface {
                    rows,
                    ..crate::surface::Surface::whole(target)
                };
                Some(Strip {
                    rows,
                    pixmap: tiny_skia::Pixmap::new(target.width, rows.height)?,
                    masks: crate::mask_cache::MaskCache::new(
                        surface,
                        rasterizer.anti_alias,
                        budget,
                    ),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            rasterizer: rasterizer.clone(),
            target,
            cuts,
            strips,
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

    /// The rows the target is cut at: the first `0`, the last its height, and a single pair
    /// where it is drawn as one surface — what [`crate::CpuRasterizer::with_cuts`] is handed to
    /// draw the same page whole.
    #[must_use]
    pub fn cuts(&self) -> &[u32] {
        &self.cuts
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
    pub fn take(&mut self, part: pdf_render::ListPart) -> bool {
        self.take_all(vec![part])
    }

    /// Draws the next parts of the list, in their order, as one — and answers whether this can
    /// still become the page.
    ///
    /// **One pass over the strips for every part that is waiting**, because the strips of a part
    /// are not its marks' strips: a page's marks come in the order they are painted, so a part
    /// holds the marks of a few rows and a pass for it alone keeps one strip busy while the others
    /// wait. Several parts reach more of the page, and the drawing is the same commands in the
    /// same order either way.
    pub fn take_all(&mut self, parts: Vec<pdf_render::ListPart>) -> bool {
        if self.stopped.is_some() {
            return false;
        }
        let mut commands = Vec::new();
        let mut until = self.drawn;
        for mut part in parts {
            if part.from != until {
                return self.stop("a part arrived out of its order");
            }
            if part.soft_mask_count() > 0 {
                return self
                    .stop("the page states a soft mask, which is built against the whole list");
            }
            let tables = self
                .tables
                .get_or_insert_with(|| DisplayList::new(part.page_size));
            if !tables.extend_tables(&part) {
                return self.stop("a part's clips do not line up with the ones drawn before it");
            }
            // What the finished list's own pass restates, restated here command by command, so
            // that a part is drawn as the finished list states it (`pdf_render::ListPart`).
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
            until = part.until();
            if commands.is_empty() {
                commands = part.commands;
            } else {
                commands.append(&mut part.commands);
            }
        }
        let Some(tables) = self.tables.as_ref() else {
            return true;
        };
        // The parts' commands in a list of the tables so far, so that their bounds are measured
        // once for every strip, as the whole draw measures the page's (ADR 1841).
        let list = tables.shape_list(commands);
        if let Err(reason) = self.draw_from(&list, 0) {
            return self.stop(reason);
        }
        self.drawn = until;
        true
    }

    /// Draws `list`'s commands from `from` on into every strip, on the pool where there are
    /// several — or says why they cannot become the page.
    fn draw_from(&mut self, list: &DisplayList, from: usize) -> Result<(), &'static str> {
        let target = self.target;
        let bounds = list.mark_bounds(target.transform);
        if self.strips.len() > 1 {
            let anti_alias = self.rasterizer.anti_alias;
            let inputs =
                pdf_render::strip_inputs(Selection::whole(list), target, &bounds, &|mark| {
                    crate::strips::drawn_row_by_row(mark, anti_alias, target)
                });
            let forbidden = inputs.unsplittable();
            let inner = self.cuts.get(1..self.cuts.len().saturating_sub(1));
            if inner.unwrap_or_default().iter().any(|row| {
                usize::try_from(*row)
                    .ok()
                    .and_then(|row| forbidden.get(row))
                    .copied()
                    .unwrap_or(true)
            }) {
                return Err(
                    "a mark crosses a row the page was cut at, where only one strip draws it exactly",
                );
            }
            // As the whole draw reduces them, on this thread before any strip is queued (ADR 0731).
            let commands = list.commands().get(from..).unwrap_or_default();
            self.rasterizer.reduced_images.warm(
                commands,
                target.transform,
                crate::strips::whole_target(target),
            );
        }
        let rasterizer = &self.rasterizer;
        let draw = |strip: &mut Strip| {
            let surface = crate::surface::Surface {
                rows: strip.rows,
                ..crate::surface::Surface::whole(target)
            };
            rasterizer.encode_page(
                &mut strip.pixmap.as_mut(),
                (Selection::whole(list), &bounds, from),
                surface,
                &mut strip.masks,
                None,
            )
        };
        let drawn = if self.strips.len() > 1 {
            self.strips.par_iter_mut().try_for_each(draw)
        } else {
            self.strips.iter_mut().try_for_each(draw)
        };
        drawn.map_err(|_| "the list did not draw")
    }

    /// The strips put together, premultiplied, as one target.
    fn joined(&self) -> Option<tiny_skia::Pixmap> {
        if let [strip] = self.strips.as_slice() {
            return Some(strip.pixmap.clone());
        }
        let mut whole = tiny_skia::Pixmap::new(self.target.width, self.target.height)?;
        let stride = (self.target.width as usize).saturating_mul(4);
        let mut rest = whole.data_mut();
        for strip in &self.strips {
            let length = (strip.rows.height as usize).saturating_mul(stride);
            let (rows, tail) = rest.split_at_mut(length.min(rest.len()));
            rows.copy_from_slice(strip.pixmap.data().get(..rows.len())?);
            rest = tail;
        }
        Some(whole)
    }

    /// The page as it stands, finished as the page will be finished — or `None` before anything
    /// has been drawn or once this has stopped.
    #[must_use]
    pub fn look(&self) -> Option<Raster> {
        let tables = self.tables.as_ref().filter(|_| self.stopped.is_none())?;
        let mut copy = self.joined()?;
        finish_rows(copy.data_mut(), tables, self.target, self.rasterizer.medium);
        Some(raster_of(copy, self.target))
    }

    /// The page, finished from `list` — the list the parts were handed from, once its read has
    /// ended — with every mark its strips declined to the library's converter, as
    /// [`crate::CpuRasterizer::declined`] gives a whole draw's (ADRs 2280, 2290); or `Err` with
    /// why it cannot be, where the caller draws `list` whole instead.
    ///
    /// # Errors
    ///
    /// Every reason in this type's own documentation, each named.
    pub fn finish(mut self, list: &DisplayList) -> Result<Finished, &'static str> {
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
        let from = self.drawn.commands;
        self.draw_from(list, from)?;
        // What every strip's masks kept since the page began to be drawn ahead, each mark once,
        // as the whole draw sorts its own — taken before a single strip is taken apart.
        let mut declined: Vec<crate::UnionDecline> = self
            .strips
            .iter_mut()
            .flat_map(|strip| strip.masks.take_declined())
            .collect();
        declined.sort_unstable();
        declined.dedup();
        let mut whole = match self.strips.len() {
            1 => self.strips.pop().map(|strip| strip.pixmap),
            _ => self.joined(),
        }
        .ok_or("the finished target could not be allocated")?;
        finish_rows(whole.data_mut(), list, self.target, self.rasterizer.medium);
        Ok(Finished {
            raster: raster_of(whole, self.target),
            declined,
        })
    }

    fn stop(&mut self, reason: &'static str) -> bool {
        self.stopped = Some(reason);
        false
    }
}

/// The per-pixel passes `CpuRasterizer::rasterize` ends a page with on a list this module
/// admits — §14.11.2.1's crop, then §11.4.7's medium under the marks — over the whole target.
///
/// Split by rows on the pool as the whole draw splits them; a per-pixel pass is the same bytes
/// either way, which is the rule that lets the whole draw split it.
fn finish_rows(
    premultiplied: &mut [u8],
    list: &DisplayList,
    target: TargetSpec,
    medium: pdf_render::Medium,
) {
    let crop = pdf_render::crop_area(list, target);
    let area = pdf_render::page_area(list, target);
    let stride = (target.width as usize).saturating_mul(4).max(4);
    premultiplied
        .par_chunks_mut(stride.saturating_mul(crate::strips::PIXEL_PASS_ROWS))
        .enumerate()
        .for_each(|(chunk, rows)| {
            let first = u32::try_from(chunk.saturating_mul(crate::strips::PIXEL_PASS_ROWS))
                .unwrap_or(u32::MAX);
            if let Some(crop) = crop {
                pdf_render::crop_to_page(rows, target.width, first, crop);
            }
            pdf_render::impose_within(rows, target.width, first, area, medium);
        });
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
