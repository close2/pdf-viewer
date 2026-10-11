//! The window's pixels, written by the processor, with no graphics device anywhere.
//!
//! **This exists so that `--cpu` can mean what it says.** A flag that chose only the rasteriser
//! would still create a `wgpu::Instance`, select an adapter and make a device, and hand the
//! processor's raster *back* to that device as one image, because a working surface would be the
//! only path pixels took to the screen. A driver that faults while it is being loaded would then
//! take the process down under `--cpu` exactly as without it, which is what the project owner hit
//! on a Windows machine with Intel graphics (`doc/QUORRA_FEEDBACK.md` section 12, and ADR 0221 for
//! this half of it).
//!
//! So there are two ways a frame reaches the window, and this is the second. It is deliberately
//! the smaller of the two in every way — one full-window buffer, one source-over per overlay, one
//! copy to the window — because the page it draws was already rasterised on the processor and the
//! difference in cost is not where this path is slow.
//!
//! # What it is not
//!
//! It is not a second renderer. Everything it draws comes from [`CpuRasterizer`], which is this
//! project's correctness oracle, through the same display lists the graphics device is handed.
//! What is here is compositing — §11.3.6's simple case, straight alpha and no blend mode, over
//! lists that are already in the window's own pixels — and a copy.

use std::num::NonZeroU32;
use std::sync::Arc;

use pdf_render::{
    DisplayList, Interrupt, Medium, Raster, RasterFormat, Rasterizer as _, TargetSpec, Transform,
};
use render_cpu::{CpuRasterError, CpuRasterizer};
use winit::window::Window;

/// Why a frame could not reach the window without a graphics device.
#[derive(Debug, thiserror::Error)]
pub enum SoftwareError {
    /// This platform, or this build, has no software path onto a window.
    ///
    /// The X11 and Wayland backends are compiled in here and the DRM/KMS one is not, so on Linux
    /// this is a session neither protocol answers for. See the manifest's note on the feature set.
    #[error("no software surface for this window: {0}")]
    Surface(#[from] softbuffer::SoftBufferError),
    /// An overlay could not be drawn on the processor.
    #[error("an overlay could not be drawn: {0}")]
    Overlay(#[from] CpuRasterError),
    /// A page of Table 29's arrangement could not be drawn on the processor.
    ///
    /// Named apart from [`Self::Overlay`] although both are the same rasteriser's refusal, because
    /// the two are different sentences to a person: a page that will not draw is a fact about the
    /// document, and an overlay that will not draw is a defect in this host's own chrome.
    #[error("page {page} could not be drawn: {problem}")]
    Page {
        /// Which page of the arrangement, counting from one as a reader does.
        page: usize,
        /// What the processor said.
        problem: CpuRasterError,
    },
    /// The window has no extent — minimised, or between a resize and its first frame.
    ///
    /// Not an error the caller reports: there is nothing to present *to*, which is different
    /// from failing to present.
    #[error("the window has no extent")]
    NoExtent,
    /// The raster offered is not the size the surface was configured for.
    ///
    /// A defect in this host rather than a fact about the machine — the page is rasterised at
    /// the window's own size — so it says both numbers rather than silently drawing part of a
    /// frame.
    #[error("a {given_width}x{given_height} raster for a {width}x{height} window")]
    WrongSize {
        /// The raster's width.
        given_width: u32,
        /// The raster's height.
        given_height: u32,
        /// The window's width.
        width: u32,
        /// The window's height.
        height: u32,
    },
}

/// A window that is written to by the processor.
///
/// Held instead of a `QuorraWindowRenderer`, never beside one: the whole point is that a run
/// holding one of these has opened no driver.
pub struct SoftwareSurface {
    /// Kept for its lifetime rather than read again. The backends take what they need from it
    /// when the surface is made, and dropping it here would be relying on that.
    _context: softbuffer::Context<Arc<Window>>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
}

impl std::fmt::Debug for SoftwareSurface {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SoftwareSurface").finish()
    }
}

impl SoftwareSurface {
    /// Takes a software surface on `window`.
    ///
    /// # Errors
    ///
    /// [`SoftwareError::Surface`] where the platform has no software path onto a window, which
    /// is a fact about the session rather than about this document.
    pub fn new(window: Arc<Window>) -> Result<Self, SoftwareError> {
        let context = softbuffer::Context::new(Arc::clone(&window))?;
        let surface = softbuffer::Surface::new(&context, window)?;
        Ok(Self {
            _context: context,
            surface,
        })
    }

    /// Draws `page` under `overlays` and puts the result on the window.
    ///
    /// `page` must already be the window's size: the caller rasterises it through a
    /// [`TargetSpec`] whose extent is the window's and whose transform carries the page's
    /// placement, which is the same spec the graphics device is given.
    ///
    /// # Errors
    ///
    /// [`SoftwareError::NoExtent`] for a window with no pixels, [`SoftwareError::WrongSize`] for
    /// a raster that is not the window's, [`SoftwareError::Overlay`] where an overlay would not
    /// draw, and [`SoftwareError::Surface`] where the platform refused the buffer or the copy.
    pub fn present(
        &mut self,
        page: &Raster,
        overlays: &[&DisplayList],
    ) -> Result<(), SoftwareError> {
        let (Some(width), Some(height)) =
            (NonZeroU32::new(page.width), NonZeroU32::new(page.height))
        else {
            return Err(SoftwareError::NoExtent);
        };
        self.surface.resize(width, height)?;
        let mut buffer = self.surface.buffer_mut()?;
        if buffer.len() != pixels(page) {
            // `resize` succeeded, so this is the platform's buffer disagreeing with the size it
            // was just given — reported rather than trusted, because the alternative is writing
            // a partial frame and calling it a present.
            return Err(SoftwareError::WrongSize {
                given_width: page.width,
                given_height: page.height,
                width: width.get(),
                height: height.get(),
            });
        }

        let composed = compose(page, overlays)?;
        // softbuffer's pixel is `00000000RRRRRRRRGGGGGGGGBBBBBBBB` on every platform it
        // supports, so the window is opaque by construction and the page's alpha — which
        // `impose_on_medium` has already resolved against the medium — is dropped here.
        for (pixel, rgba) in buffer.iter_mut().zip(composed.data.chunks_exact(4)) {
            *pixel = u32::from_be_bytes([0, rgba[0], rgba[1], rgba[2]]);
        }
        buffer.present()?;
        Ok(())
    }
}

/// How many pixels a raster holds: the count of whole RGBA groups in its bytes.
///
/// Counted through `chunks_exact` rather than divided, so there is no arithmetic to be wrong
/// about and a truncated buffer answers with what it actually holds.
fn pixels(raster: &Raster) -> usize {
    raster.data.chunks_exact(4).len()
}

/// Draws every page of Table 29's arrangement into one window-sized raster, in the order given.
///
/// ISO 32000-2 §7.7.2's `OneColumn` and the four two-page values put several pages in one window,
/// and each arrives with its own [`TargetSpec`] whose extent is the window's and whose transform
/// carries that page's placement — the same specs the graphics device is handed. So the first
/// page is drawn as it always was, onto the medium, and every page after it onto transparency and
/// composited over what is there. The pages do not overlap, so the order is the arrangement's
/// rather than a compositing decision.
///
/// **The same shape as [`compose`], and for the same reason**: [`CpuRasterizer`] answers with a
/// new raster rather than drawing into one, so a page costs a window's worth of pixels and a
/// source-over. There are at most `viewer_core`'s bound of them, and this is the path that is
/// already the slow one.
///
/// **The medium is [`Medium::WINDOW`] and the pages after the first carry only their own 𝑊.**
/// A window is not a page: ISO 32000-2 §11.4.7's 𝑊 is the initial colour of the page and stops
/// at §14.11.2.1's crop box, and what a window shows between two pages is this program's own
/// choice — [`pdf_render::medium`] has the reading and the argument. The first page's raster
/// paints the surround across the whole window because it is the bottom of the stack; every page
/// after it is [`Medium::on_transparency`], so it carries its paper and not the ground beside
/// its neighbour.
///
/// # Errors
///
/// [`SoftwareError::NoExtent`] where the arrangement places no page at all, and
/// [`SoftwareError::Page`] where one would not rasterise.
pub fn compose_pages(pages: &[(&DisplayList, TargetSpec)]) -> Result<Raster, SoftwareError> {
    compose_pages_interruptibly(pages, None)
}

/// [`compose_pages`], with a flag another thread can raise to take the drawing thread back.
///
/// **What the interrupt is for is the *host's* decision and not this function's** — ADR 0657 has
/// the policy and `pdf_render::Interrupt` the mechanism. What is decided here is only that the
/// flag reaches every page of Table 29's arrangement rather than the first: a two-page spread is
/// two rasterisations and a `None` on the second would make a raise wait for a whole page.
///
/// The arrangement's pages are drawn in order, so an interrupt raised part way through leaves the
/// pages before it drawn and the rest not. Nothing here composes that partial picture: the whole
/// call refuses, because a window frame that is right about one page of two is a window with a
/// hole in it, and the caller's answer to a refusal is to show what it had (`doc/todo/37`).
///
/// # Errors
///
/// As [`compose_pages`], plus [`SoftwareError::Page`] carrying
/// `pdf_render::BackendError::Interrupted` where the flag was raised.
pub fn compose_pages_interruptibly(
    pages: &[(&DisplayList, TargetSpec)],
    interrupt: Option<&Interrupt>,
) -> Result<Raster, SoftwareError> {
    let Some((first, rest)) = pages.split_first() else {
        return Err(SoftwareError::NoExtent);
    };
    compose_pages_showing(*first, rest, interrupt, None).map_err(PageRefused::into_error)
}

/// What a draw of Table 29's arrangement says about itself while somebody is looking at it.
///
/// **Two kinds of news, because the arrangement is drawn a page at a time**: a run of one page's
/// rows as they stand ([`render_cpu::Glimpse`], ADR 1825), and the arrangement composed through a
/// page that has finished — which is what the next page's rows are drawn over, since every page
/// after the first is drawn onto transparency and composited ([`compose_pages`]).
#[derive(Debug)]
pub enum Seen {
    /// Rows of the arrangement's page `page`, counting from zero, as they stand part-way.
    Rows {
        /// Which page of the arrangement.
        page: usize,
        /// The rows.
        glimpse: render_cpu::Glimpse,
    },
    /// The arrangement composed through page `page`, which has finished.
    Through {
        /// Which page of the arrangement finished.
        page: usize,
        /// Every page up to and including it, composed as the finished frame will have them.
        composed: Raster,
    },
}

/// Which page of the arrangement refused, counting from zero, and what the processor said.
///
/// [`SoftwareError::Page`] without the variants that cannot cross a thread — a software surface's
/// error holds the platform's handles — so that the draw can run on one ([`compose_pages_watched`]).
#[derive(Debug)]
struct PageRefused {
    index: usize,
    problem: CpuRasterError,
}

impl PageRefused {
    /// The refusal a caller reports, counting pages from one as a reader does.
    fn into_error(self) -> SoftwareError {
        SoftwareError::Page {
            page: self.index.saturating_add(1),
            problem: self.problem,
        }
    }
}

/// Where a draw sends what it is asked for, the flag it is asked through, and whether it has been.
struct Sink {
    glance: render_cpu::Glance,
    /// Raised by the watcher the first time it asks, so that a page finishing before anybody
    /// looked does not copy the arrangement for nobody.
    looked: std::sync::atomic::AtomicBool,
    to: Arc<dyn Fn(Seen) + Send + Sync>,
}

/// [`compose_pages_interruptibly`], with each page answering `showing`'s glances as it is drawn.
///
/// A page whose draw the rasteriser does not glimpse — its finish needs a second plane
/// (`render_cpu`'s glimpse module says which) — answers nothing until it finishes, and then its
/// [`Seen::Through`] arrives as every page's does. The first page is taken apart from the rest
/// because an arrangement of none is no frame, which each caller has already said.
fn compose_pages_showing(
    (first, target): (&DisplayList, TargetSpec),
    rest: &[(&DisplayList, TargetSpec)],
    interrupt: Option<&Interrupt>,
    showing: Option<&Sink>,
) -> Result<Raster, PageRefused> {
    /// Which page of the arrangement refused, counting from zero.
    fn refused(index: usize) -> impl Fn(CpuRasterError) -> PageRefused {
        move |problem| PageRefused { index, problem }
    }
    /// A rasteriser that honours `interrupt`, or the plain one where there is none to honour.
    ///
    /// Built per page rather than cloned, which is what the existing shape already did, and the
    /// `Option` is what keeps a caller that hands none over on the path every gate in this tree
    /// runs — 867 instructions a draw against 0.18%, ADR 0650 section 5.
    fn drawing(
        interrupt: Option<&Interrupt>,
        medium: Medium,
        showing: Option<&Sink>,
        page: usize,
    ) -> CpuRasterizer {
        let rasterizer = CpuRasterizer::new().with_medium(medium);
        let rasterizer = match interrupt {
            Some(interrupt) => rasterizer.interruptible(interrupt.clone()),
            None => rasterizer,
        };
        match showing {
            Some(sink) => {
                let to = Arc::clone(&sink.to);
                rasterizer.showing(sink.glance.clone(), move |glimpse| {
                    to(Seen::Rows { page, glimpse });
                })
            }
            None => rasterizer,
        }
    }
    /// The arrangement through `page`, where somebody is looking — a window's worth of bytes
    /// copied once a page, and only for a draw that has been asked for a look.
    fn through(showing: Option<&Sink>, page: usize, composed: &Raster) {
        if let Some(sink) = showing
            && sink.looked.load(std::sync::atomic::Ordering::Relaxed)
        {
            (sink.to)(Seen::Through {
                page,
                composed: composed.clone(),
            });
        }
    }
    let medium = Medium::WINDOW;
    let mut composed = drawing(interrupt, medium, showing, 0)
        .rasterize(first, target)
        .map_err(refused(0))?;
    match composed.format {
        RasterFormat::Rgba8 => {}
    }
    through(showing, 0, &composed);
    for (index, (list, placed)) in rest.iter().enumerate() {
        let page = index.saturating_add(1);
        // Its own 𝑊 and no surround: this is a layer over what is already drawn, and the
        // ground between the pages was painted once by the page below.
        let over = drawing(interrupt, medium.on_transparency(), showing, page)
            .rasterize(list, *placed)
            .map_err(refused(page))?;
        match over.format {
            RasterFormat::Rgba8 => {}
        }
        source_over(&mut composed, &over);
        through(showing, page, &composed);
    }
    Ok(composed)
}

/// When a draw is first looked at, and how often after that.
///
/// `viewer_host::drawing`'s two waits, which the toolkit windows look at a page in flight by
/// (ADR 1825): no page of `doc/pdf.js`'s 957 first pages is looked at, and a page that draws for
/// seconds is shown four times a second.
#[derive(Debug, Clone, Copy)]
pub struct Looks {
    /// How long the draw runs before it is first looked at.
    pub after: std::time::Duration,
    /// How long between looks after that.
    pub every: std::time::Duration,
}

impl Looks {
    /// The waits the toolkit windows look by.
    pub const SHIPPED: Self = Self {
        after: viewer_host::drawing::GLIMPSE_AFTER,
        every: viewer_host::drawing::GLIMPSE_EVERY,
    };
}

/// [`compose_pages_interruptibly`] on a thread of its own, with the arrangement as it stands
/// handed to `shown` on the calling thread at every look (ADR 1831).
///
/// **The calling thread is the one that keeps time and answers**, because that is the thread a
/// window's pixels go out through: `quorra`'s render thread owns the graphics device a picture is
/// uploaded by, and the composing thread of the window with none owns the channel back. So the
/// draw itself runs on a scoped thread — rayon's strips under it as always — and this thread asks
/// [`render_cpu::Glance`] for a look at [`Looks::after`] and every [`Looks::every`] after, folds
/// what comes back into an [`Assembly`], and calls `shown` once per batch of news. The raster at
/// the end is the unobserved one, byte for byte: a look copies rows out and writes nothing back.
///
/// # Errors
///
/// As [`compose_pages_interruptibly`].
pub fn compose_pages_watched(
    pages: &[(&DisplayList, TargetSpec)],
    interrupt: Option<&Interrupt>,
    looks: Looks,
    mut shown: impl FnMut(&Assembly),
) -> Result<Raster, SoftwareError> {
    use std::sync::mpsc::{RecvTimeoutError, channel};
    use std::time::Instant;

    let Some((&(list, target), rest)) = pages.split_first() else {
        return Err(SoftwareError::NoExtent);
    };
    // `None` is the draw saying it has ended, so that the end is noticed when it happens rather
    // than at the next look.
    let (sender, news) = channel::<Option<Seen>>();
    let ended = sender.clone();
    let glance = render_cpu::Glance::new();
    let to: Arc<dyn Fn(Seen) + Send + Sync> = Arc::new(move |seen| {
        // A send that fails is a watcher that has stopped reading, which is a draw that has
        // ended; the rows are a copy, so dropping them loses nothing of the page.
        let _ = sender.send(Some(seen));
    });
    let sink = Sink {
        glance: glance.clone(),
        looked: std::sync::atomic::AtomicBool::new(false),
        to,
    };
    let began = Instant::now();
    std::thread::scope(|scope| {
        let drawing = scope.spawn(|| {
            let drawn = compose_pages_showing((list, target), rest, interrupt, Some(&sink));
            let _ = ended.send(None);
            drawn
        });
        let mut assembly = Assembly::new(target.width, target.height, pages.len());
        let mut next_look = began.checked_add(looks.after);
        loop {
            let wait = next_look.map_or(looks.every, |at| {
                at.saturating_duration_since(Instant::now())
            });
            match news.recv_timeout(wait.min(looks.every)) {
                Ok(Some(seen)) => {
                    assembly.take(seen);
                    let mut over = false;
                    while let Ok(more) = news.try_recv() {
                        match more {
                            Some(more) => assembly.take(more),
                            None => over = true,
                        }
                    }
                    if over {
                        break;
                    }
                    shown(&assembly);
                }
                Ok(None) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
            if next_look.is_some_and(|at| Instant::now() >= at) {
                sink.looked
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                glance.ask();
                next_look = Instant::now().checked_add(looks.every);
            }
        }
        match drawing.join() {
            Ok(drawn) => drawn.map_err(PageRefused::into_error),
            // The scoped thread is this function's own and `compose_pages_showing` returns its
            // refusals as values, so a panic there is a defect, and it is carried on as one.
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}

/// The arrangement part-way through its draw, put together from what [`compose_pages_watched`]'s
/// looks brought back — what a window shows while a page draws for seconds (ADR 1831).
///
/// Rows of the page being drawn that no strip has answered for yet are transparent, so a window
/// shows its own ground there rather than a colour the page has not been given — the toolkit
/// windows' `viewer_host::drawing::Partial`, for an arrangement of several pages.
#[derive(Debug)]
pub struct Assembly {
    /// The picture as it stands.
    picture: Raster,
    /// The arrangement composed through the last finished page, which the page being drawn is
    /// drawn over; `None` while the first page is still being drawn.
    below: Option<Raster>,
    /// How many pages the arrangement has.
    pages: usize,
    /// How many of them have finished.
    finished: usize,
    /// Per run of rows of the page being drawn: its first row, its height, and how far through
    /// that page's commands it had got.
    bands: Vec<(u32, u32, usize, usize)>,
}

impl Assembly {
    /// Nothing drawn yet, for an arrangement of `pages` pages in a `width` × `height` window.
    #[must_use]
    pub fn new(width: u32, height: u32, pages: usize) -> Self {
        let bytes = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        Self {
            picture: Raster {
                width,
                height,
                format: RasterFormat::Rgba8,
                data: vec![0; bytes],
            },
            below: None,
            pages,
            finished: 0,
            bands: Vec::new(),
        }
    }

    /// The pixels, at the window's size.
    #[must_use]
    pub fn picture(&self) -> &Raster {
        &self.picture
    }

    /// How much of the arrangement's drawing the picture holds: whole pages for the pages that
    /// have finished, and the page being drawn by its commands, weighted by the rows each strip
    /// owns — what a window's bar shows.
    #[must_use]
    pub fn fraction(&self) -> f64 {
        let rows = f64::from(self.picture.height.max(1));
        let current = self
            .bands
            .iter()
            .map(|&(_, height, drawn, of)| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a share of a command count, for a bar"
                )]
                let share = if of == 0 {
                    1.0
                } else {
                    drawn as f64 / of as f64
                };
                share * f64::from(height)
            })
            .sum::<f64>()
            / rows;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of an arrangement's pages, which is a handful"
        )]
        let (finished, pages) = (self.finished as f64, self.pages.max(1) as f64);
        ((finished + current.min(1.0)) / pages).min(1.0)
    }

    /// Folds one piece of news in; rows that do not fit the window are dropped.
    pub fn take(&mut self, seen: Seen) {
        match seen {
            Seen::Through { page, composed } => {
                if composed.width != self.picture.width || composed.height != self.picture.height {
                    return;
                }
                self.picture.data.copy_from_slice(&composed.data);
                self.below = Some(composed);
                self.finished = page.saturating_add(1);
                self.bands.clear();
            }
            Seen::Rows { page, glimpse } => {
                if page != self.finished || glimpse.rows.width != self.picture.width {
                    return;
                }
                let stride = (self.picture.width as usize).saturating_mul(4);
                let start = (glimpse.top as usize).saturating_mul(stride);
                let end = start.saturating_add(glimpse.rows.data.len());
                let Some(rows) = self.picture.data.get_mut(start..end) else {
                    return;
                };
                match self
                    .below
                    .as_ref()
                    .and_then(|below| below.data.get(start..end))
                {
                    // A later page is a layer over the pages before it, as in the finished frame.
                    Some(under) => {
                        rows.copy_from_slice(under);
                        over_rows(rows, &glimpse.rows.data);
                    }
                    None => rows.copy_from_slice(&glimpse.rows.data),
                }
                let band = (glimpse.top, glimpse.rows.height, glimpse.drawn, glimpse.of);
                match self.bands.iter_mut().find(|held| held.0 == glimpse.top) {
                    Some(held) => *held = band,
                    None => self.bands.push(band),
                }
            }
        }
    }
}

/// Whether a [`SoftwareError`] is the caller's own interrupt coming back rather than a refusal.
///
/// **A refusal is a fact about the document and this is a decision the host made about its own
/// thread**, which is why the two may not be reported alike: `viewer_core::Rendered::Failed`
/// records a page as answered for and stops the scheduler asking again (ADR 0657), and a host
/// that said it about a draw it abandoned itself would mark the page shown for good.
///
/// **Both variants that can carry the rasteriser's refusal are named**, although only
/// [`compose_pages_interruptibly`] hands an interrupt over today and an overlay is drawn without
/// one. It classifies an *error* rather than a call site: a later round that made the chrome
/// interruptible would otherwise get `false` here and report the abandonment as a failure, which
/// is precisely the confusion this exists to prevent.
#[must_use]
pub fn was_interrupted(problem: &SoftwareError) -> bool {
    matches!(
        problem,
        SoftwareError::Page {
            problem: CpuRasterError::Target(pdf_render::BackendError::Interrupted),
            ..
        } | SoftwareError::Overlay(CpuRasterError::Target(
            pdf_render::BackendError::Interrupted
        ))
    )
}

/// A window with no page on it at all, in [`pdf_render::SURROUND`].
///
/// **The window before any document is open, which is a window that still has something on it**
/// (ADR 0545): §7.6.4.1's card is drawn over an encrypted document that has not authenticated, and
/// there is no page behind it to compose onto.
///
/// The colour is [`Medium::WINDOW`]'s ground rather than a value chosen here, which is
/// [`compose_pages`]'s own reading applied to the degenerate case: §11.4.7's 𝑊 is *the page's*
/// initial colour and stops at §14.11.2.1's crop box, so what a window shows where there is no
/// page is this program's choice and is stated once, in [`pdf_render::medium`] (ADR 0446).
///
/// Built by rasterising an empty display list rather than by filling a buffer, so that the one
/// place the surround is decided stays the one place it is applied.
///
/// # Errors
///
/// [`SoftwareError::NoExtent`] for a window with no pixels, and [`SoftwareError::Page`] where the
/// rasteriser refused an empty list — which would be a defect in this host rather than anything
/// about a document.
pub fn surround(width: u32, height: u32) -> Result<Raster, SoftwareError> {
    if width == 0 || height == 0 {
        return Err(SoftwareError::NoExtent);
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a window's extent in pixels, which is thousands"
    )]
    let size = pdf_render::Size {
        width: width as f32,
        height: height as f32,
    };
    let empty = DisplayList::new(size);
    CpuRasterizer::new()
        .with_medium(Medium::WINDOW)
        .rasterize(
            &empty,
            TargetSpec {
                width,
                height,
                transform: Transform::IDENTITY,
            },
        )
        .map_err(|problem| SoftwareError::Page { page: 0, problem })
}

/// Draws every overlay over `page`, in order, and answers the result.
///
/// The overlays are window-pixel display lists — selection quads, the sidebar, the modal card —
/// so each is rasterised at the identity transform into the page's own extent and composited.
/// Separate from [`SoftwareSurface::present`] and public for one reason: it is the half of the
/// software path that can be tested without a window, and it is the half with the arithmetic in
/// it.
///
/// **A copy per overlay, deliberately.** [`CpuRasterizer`] answers with a new raster rather than
/// drawing into one, so the alternative is a second entry point into the oracle for the sake of a
/// path that is already the slow one. There are between zero and six of these lists on a frame.
///
/// # Errors
///
/// [`SoftwareError::Overlay`] where an overlay would not rasterise, which is a defect in this
/// host's own chrome rather than anything about the document.
pub fn compose(page: &Raster, overlays: &[&DisplayList]) -> Result<Raster, SoftwareError> {
    // Exhaustive, and that is what ADR 0247 bought: this function's arithmetic is written for
    // four straight-alpha bytes per pixel, and a second `RasterFormat` now fails to compile here
    // rather than being refused at runtime. Both operands are checked because both are rasters —
    // the page from the worker and each overlay from `CpuRasterizer` below.
    match page.format {
        RasterFormat::Rgba8 => {}
    }
    let mut composed = page.clone();
    for list in overlays {
        let spec = TargetSpec {
            width: page.width,
            height: page.height,
            transform: Transform::IDENTITY,
        };
        // Nothing under it at all: this is a layer over the page, and any medium would erase
        // it. `Medium::NONE` is what `CpuRasterizer::with_medium`'s own documentation names for
        // exactly this.
        let over = CpuRasterizer::new()
            .with_medium(Medium::NONE)
            .rasterize(list, spec)?;
        match over.format {
            RasterFormat::Rgba8 => {}
        }
        source_over(&mut composed, &over);
    }
    Ok(composed)
}

/// Composites `top` onto `under` in place: source-over, straight alpha, no blend mode.
///
/// §11.3.6's formula with the group machinery removed, which is what is left when both operands
/// are already device pixels at the same extent and neither is in a transparency group. Both
/// rasters are [`RasterFormat::Rgba8`], which [`compose`] has checked — this function is private
/// so that the check has exactly one caller to be true of.
fn source_over(under: &mut Raster, top: &Raster) {
    over_rows(&mut under.data, &top.data);
}

/// [`source_over`] on two runs of straight-alpha pixels of the same extent.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "each channel is a ratio of two bytes scaled back to 0..=255 and clamped there \
              before the cast, so neither truncation nor a negative value is reachable"
)]
fn over_rows(under: &mut [u8], top: &[u8]) {
    for (dst, src) in under.chunks_exact_mut(4).zip(top.chunks_exact(4)) {
        let source_alpha = f32::from(src[3]) / 255.0;
        if source_alpha <= 0.0 {
            continue;
        }
        if source_alpha >= 1.0 {
            dst.copy_from_slice(src);
            continue;
        }
        let under_alpha = f32::from(dst[3]) / 255.0;
        let alpha = source_alpha + under_alpha * (1.0 - source_alpha);
        if alpha <= 0.0 {
            dst.copy_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        for channel in 0..3 {
            let source = f32::from(src[channel]) / 255.0;
            let below = f32::from(dst[channel]) / 255.0;
            let value =
                (source * source_alpha + below * under_alpha * (1.0 - source_alpha)) / alpha;
            dst[channel] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        dst[3] = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pdf_render::{
        BlendMode, Color, Command, DisplayList, Interrupt, Paint, Path, PathCommand, Point, Raster,
        RasterFormat, Size, TargetSpec,
    };

    use super::{compose, compose_pages, compose_pages_interruptibly, was_interrupted};

    /// A window-sized opaque page, one colour throughout.
    fn page(width: u32, height: u32, colour: [u8; 4]) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: colour.repeat((width as usize).saturating_mul(height as usize)),
        }
    }

    /// An overlay that fills the left half of the window with `paint`.
    fn left_half(width: u32, height: u32, paint: Color) -> DisplayList {
        #[expect(
            clippy::cast_precision_loss,
            reason = "test extents are tens of pixels"
        )]
        let mut list = DisplayList::new(Size::new(width as f32, height as f32));
        let mut path = Path::new();
        #[expect(
            clippy::cast_precision_loss,
            reason = "test extents are tens of pixels"
        )]
        let (half, tall) = ((width / 2) as f32, height as f32);
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(half, 0.0)));
        path.push(PathCommand::LineTo(Point::new(half, tall)));
        path.push(PathCommand::LineTo(Point::new(0.0, tall)));
        path.push(PathCommand::Close);
        list.push(Command::Fill {
            path: Arc::new(path),
            transform: pdf_render::Transform::IDENTITY,
            paint: Paint::Solid(paint),
            fill_rule: pdf_render::FillRule::NonZero,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        list
    }

    /// A raised interrupt reaches the arrangement, and it is legible as itself afterwards.
    ///
    /// **The second half is the one that matters** (ADR 0657): the host's answer to the core turns
    /// on telling an interrupt apart from a refusal, so `was_interrupted` is asserted rather than
    /// the error merely being an error. And a second arm with no interrupt at all, because the
    /// same call is what every frame of a `--cpu` run makes.
    #[test]
    fn a_raised_interrupt_refuses_the_arrangement_and_says_which_refusal_it_is() {
        let list = left_half(16, 8, Color::rgb(0.0, 0.0, 0.0));
        let target = TargetSpec {
            width: 16,
            height: 8,
            transform: pdf_render::Transform::IDENTITY,
        };
        let pages = [(&list, target)];

        let interrupt = Interrupt::new();
        interrupt.raise();
        let problem = compose_pages_interruptibly(&pages, Some(&interrupt))
            .expect_err("a raised interrupt is honoured before the first command");
        assert!(
            was_interrupted(&problem),
            "a host that read this as a refusal would tell the core the page cannot be drawn: \
             {problem}"
        );

        let drawn = compose_pages_interruptibly(&pages, Some(&Interrupt::new()))
            .expect("an unraised interrupt draws the arrangement");
        assert_eq!(
            drawn,
            compose_pages(&pages).expect("and so does no interrupt at all"),
            "handing one over changes nothing about the pixels"
        );
    }

    /// **A draw looked at is the draw unobserved, byte for byte**, and it is looked at: the
    /// watcher's thread asks, the rows come back folded into the picture, and the share is a share
    /// (ADR 1831). Two pages, so that the second's rows are composed over the first's.
    #[test]
    fn a_watched_arrangement_is_the_arrangement_and_is_shown_on_the_way() {
        let (width, height) = (32_u32, 16_u32);
        let mut list = left_half(width, height, Color::rgb(0.2, 0.4, 0.6));
        // Enough commands that the draw outlasts a look asked for at once.
        let one: Vec<Command> = list.commands().to_vec();
        for _ in 0..20_000 {
            for command in &one {
                list.push(command.clone());
            }
        }
        let target = TargetSpec {
            width,
            height,
            transform: pdf_render::Transform::IDENTITY,
        };
        let moved = TargetSpec {
            transform: pdf_render::Transform::translate(16.0, 0.0),
            ..target
        };
        let pages = [(&list, target), (&list, moved)];
        let mut looks = 0_usize;
        let looked = super::compose_pages_watched(
            &pages,
            None,
            super::Looks {
                after: std::time::Duration::ZERO,
                every: std::time::Duration::from_millis(1),
            },
            |assembly| {
                looks += 1;
                assert!((0.0..=1.0).contains(&assembly.fraction()));
                assert_eq!(assembly.picture().data.len(), 32 * 16 * 4);
            },
        )
        .expect("drawn");
        assert_eq!(
            looked,
            compose_pages(&pages).expect("drawn unobserved"),
            "a look copies rows out and writes nothing back"
        );
        assert!(
            looks > 0,
            "a draw of forty thousand commands was never looked at"
        );
    }

    /// The picture part-way: a first page's rows replace, a later page's are composed over what
    /// the pages before it left, and the share counts the finished pages whole.
    #[test]
    fn an_assembly_composes_a_later_page_over_the_earlier_ones() {
        let rows = |colour: [u8; 4]| Raster {
            width: 2,
            height: 1,
            format: RasterFormat::Rgba8,
            data: colour.repeat(2),
        };
        let glimpse = |top, colour, drawn, of| super::Seen::Rows {
            page: 0,
            glimpse: render_cpu::Glimpse {
                top,
                rows: rows(colour),
                drawn,
                of,
            },
        };
        let mut assembly = super::Assembly::new(2, 2, 2);
        assembly.take(glimpse(0, [255, 0, 0, 255], 5, 10));
        assert_eq!(&assembly.picture().data[0..4], &[255, 0, 0, 255]);
        assert_eq!(
            &assembly.picture().data[8..12],
            &[0, 0, 0, 0],
            "a row nobody answered"
        );
        assert!(
            (assembly.fraction() - 0.125).abs() < 1e-9,
            "half of half of one page of two"
        );
        assembly.take(super::Seen::Through {
            page: 0,
            composed: page(2, 2, [255, 255, 255, 255]),
        });
        assert!((assembly.fraction() - 0.5).abs() < 1e-9);
        let super::Seen::Rows { glimpse: later, .. } = glimpse(1, [0, 0, 0, 0], 1, 1) else {
            unreachable!("built as rows a line above");
        };
        assembly.take(super::Seen::Rows {
            page: 1,
            glimpse: later,
        });
        assert_eq!(
            &assembly.picture().data[8..12],
            &[255, 255, 255, 255],
            "a later page's transparent rows leave the page below as it was"
        );
        assembly.take(glimpse(0, [0, 255, 0, 255], 10, 10));
        assert_eq!(
            &assembly.picture().data[0..4],
            &[255, 255, 255, 255],
            "rows of a page that has finished are news of nothing"
        );
    }

    /// No overlays is the page itself: the identity of the composition, which is what a
    /// `--cpu` run with no sidebar and no selection presents.
    #[test]
    fn no_overlays_leaves_the_page_alone() {
        let under = page(8, 4, [200, 100, 50, 255]);
        let composed = compose(&under, &[]).expect("compose");
        assert_eq!(composed, under);
    }

    /// An opaque overlay replaces what it covers and leaves the rest.
    #[test]
    fn an_opaque_overlay_covers_the_page() {
        let under = page(8, 4, [255, 255, 255, 255]);
        let list = left_half(8, 4, Color::BLACK);
        let composed = compose(&under, &[&list]).expect("compose");
        assert_eq!(&composed.data[0..4], &[0, 0, 0, 255], "the covered half");
        assert_eq!(
            &composed.data[7 * 4..8 * 4],
            &[255, 255, 255, 255],
            "the half nothing was drawn on"
        );
    }

    /// A half-transparent overlay is a half-way colour, and the result stays opaque —
    /// the property that makes the 0RGB copy to the window lossless.
    #[test]
    fn a_translucent_overlay_mixes_and_stays_opaque() {
        let under = page(8, 4, [255, 255, 255, 255]);
        let list = left_half(
            8,
            4,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            },
        );
        let composed = compose(&under, &[&list]).expect("compose");
        assert_eq!(composed.data[3], 255, "an opaque page stays opaque");
        assert!(
            (100..=160).contains(&composed.data[0]),
            "half of white over black is a middle grey, not {}",
            composed.data[0]
        );
    }

    /// Table 29's column on the processor: two pages, each under its own placement, in one
    /// window-sized raster.
    ///
    /// The property that matters is that the **second page reaches the window at all** — a
    /// composition that drew the first and stopped would be the tier-2 host drawing one page of a
    /// column, which is exactly what ADR 0442 was for. Each list here fills the whole target under
    /// its own transform, so the second covers the first and the result is the second's colour.
    #[test]
    fn every_page_of_the_arrangement_reaches_the_raster() {
        let first = left_half(8, 4, Color::BLACK);
        let second = left_half(
            8,
            4,
            Color {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        );
        // The second page placed four pixels to the right of the first, which is what a column's
        // second entry is: the same list at a different origin.
        let whole = TargetSpec {
            width: 8,
            height: 4,
            transform: pdf_render::Transform::IDENTITY,
        };
        let moved = TargetSpec {
            transform: pdf_render::Transform::translate(4.0, 0.0),
            ..whole
        };
        let composed = compose_pages(&[(&first, whole), (&second, moved)]).expect("two pages");
        assert_eq!(&composed.data[0..4], &[0, 0, 0, 255], "the first page");
        assert_eq!(
            &composed.data[5 * 4..6 * 4],
            &[0, 0, 255, 255],
            "the second page, at the origin the arrangement gave it"
        );
    }

    /// An arrangement that places no page has nothing to compose, and says so rather than
    /// answering with an empty raster a caller would present.
    #[test]
    fn no_pages_is_not_a_frame() {
        assert!(matches!(
            compose_pages(&[]),
            Err(super::SoftwareError::NoExtent)
        ));
    }

    /// Overlays are drawn in the order they are given: the modal card is over the sidebar
    /// because it is later in the list, and nothing else decides that.
    #[test]
    fn later_overlays_are_drawn_over_earlier_ones() {
        let under = page(8, 4, [255, 255, 255, 255]);
        let first = left_half(8, 4, Color::BLACK);
        let second = left_half(
            8,
            4,
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        let composed = compose(&under, &[&first, &second]).expect("compose");
        assert_eq!(&composed.data[0..4], &[255, 0, 0, 255]);
    }
}
