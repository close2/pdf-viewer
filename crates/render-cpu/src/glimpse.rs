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

use pdf_render::{DisplayList, Raster, RasterFormat, TargetSpec};

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
    /// # Errors
    ///
    /// As `encode`.
    pub(crate) fn encode_page(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        surface: crate::Surface,
        masks: &mut crate::MaskCache,
        showing: Option<&Showing>,
    ) -> Result<(), crate::CpuRasterError> {
        let commands = list.commands();
        let Some(showing) = showing else {
            return self.encode(
                pixmap,
                list,
                commands,
                surface,
                masks,
                0,
                crate::Compose::Over,
            );
        };
        let mut watch = Watch::new(showing, list, surface.page, surface.rows.top, self.medium);
        for (drawn, command) in commands.iter().enumerate() {
            if watch.due() {
                watch.answer(pixmap.as_ref().data(), drawn, commands.len());
            }
            self.encode(
                pixmap,
                list,
                std::slice::from_ref(command),
                surface,
                masks,
                0,
                crate::Compose::Over,
            )?;
        }
        // The strip has landed; somebody looking at the page is shown its final rows now rather
        // than when the slowest strip ends.
        if showing.glance.asked() > 0 {
            watch.answer(pixmap.as_ref().data(), commands.len(), commands.len());
        }
        Ok(())
    }
}
