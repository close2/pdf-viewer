//! The rectangle each page's pixels are shown in, which after a zoom is not the pixels' own size.
//!
//! A zoom changes where the arrangement places a page and how big it is at once, and the viewer
//! answers both for the new magnification the moment the command is taken
//! (`viewer_core::Query::PageGeometry`), while the pixels it holds for the page are the last frame
//! drawn, at the old magnification, until the new one lands. A window that put those pixels at the
//! new origin at their own size showed the page at the old size anchored at the new corner — the
//! owner's "incorrect frames during zooming" in both toolkit windows. [`placed`] says where the old
//! pixels belong instead: **scaled to the extent the page now occupies, at the origin it now has**,
//! which is the picture `quorra` stands in with until its frame lands (`stale.rs`), made by the
//! toolkit's own scaled draw rather than by a resample of ours (ADR 1843).
//!
//! **It is shown only where the zoomed frame has not landed by the next refresh** ([`Hold`], A657,
//! ADR 2510): for one period of the display after a zoom command a window keeps painting the
//! pictures it had, at the geometry they had, and a frame that lands inside that refresh replaces
//! a sharp picture and not a blurred one. `quorra` waits the same refresh by prediction and
//! observation (`doc/todo/37`'s rule 5); a toolkit window has neither, so it waits by the clock.
//!
//! It is a wrong picture shown on purpose — the old pixels enlarged are blurred, and what the new
//! frame adds is absent — so it is held to `doc/todo/37`'s rules as far as a toolkit window can
//! hold them: the frame that lands replaces it (rule 1, `RenderReady` sets the pixels and the
//! extent agrees again), a window says so on its `frames` topic (rule 3, [`Placed::stands_in`]),
//! and the scale is drawn by the toolkit beside the render on another thread, clipped to the
//! window (rule 4).
//!
//! **And a page that arrives on the screen with no pixels at all** — a notch out under a column,
//! a page scrolled back to after it left — is shown from [`Retained`]: a whole page at
//! [`RETAINED_EDGE`] pixels along its longer side, kept from the last finished frame of that
//! page at the document's current ink, scaled by the toolkit into the extent the page has now
//! until its own frame lands. `quorra` draws its retained pages again on the render thread
//! (ADR 0443) because its frame is a picture of the window; a toolkit window's frame is the
//! whole page already, so the picture is a reduction of a frame it was handed and not a draw
//! (ADR 1866).

use std::time::{Duration, Instant};

use pdf_render::{Raster, RasterFormat};
use viewer_core::{DocumentId, FrameView, PageGeometry, RenderRequest};

/// One page's pixels and the rectangle a window shows them in, in device pixels of the viewport.
#[derive(Debug)]
pub struct Placed<'a> {
    /// What the viewer, or `drawing::over_frames`, handed over for the page.
    pub frame: FrameView<'a>,
    /// The width and height the arrangement gives the page now, which the pixels are drawn to fill.
    pub extent: (u32, u32),
    /// Whether the pixels are a [`Retained`] page rather than a frame the viewer holds — the
    /// second amount of wrong, which a window says apart from the first (`doc/todo/37`'s rule 3).
    pub retained: bool,
    /// Whether the pixels are a frame kept where it was for one refresh after a zoom command
    /// ([`Hold`], A657): the whole old picture at the geometry it had, which is not a stand-in
    /// and is said apart from one.
    pub held: bool,
}

impl Placed<'_> {
    /// Whether the pixels are shown at a size other than their own: the page's last frame
    /// standing in, scaled, for the one being drawn at the new magnification.
    #[must_use]
    pub fn stands_in(&self) -> bool {
        self.extent != (self.frame.raster.width, self.frame.raster.height)
    }

    /// Whether the pixels are shown while the frame for the page's size is still coming: held
    /// where they were, or scaled to the extent the page has now. A window says so on its
    /// `frames` topic (`doc/todo/37`'s rule 3).
    #[must_use]
    pub fn waits_for_a_frame(&self) -> bool {
        self.held || self.stands_in()
    }

    /// Where the pixels' top-left corner goes: [`whole_pixel`] of the origin for pixels at their
    /// own size, and the origin itself for a stand-in, which the toolkit filters as it scales.
    #[must_use]
    pub fn shown_at(&self) -> (f32, f32) {
        if self.stands_in() {
            self.frame.origin
        } else {
            whole_pixel(self.frame.origin)
        }
    }
}

/// The whole device pixel nearest `origin`, which is where a window shows a page's own pixels.
///
/// `viewer_core::Query::PageGeometry` places a page at the fraction its centring and scroll give,
/// and its raster is drawn on its own grid. Shown at a fraction, a toolkit either rounds — Qt's
/// raster engine, for an image at its own size — or filters, which is what GSK does: every row of
/// the page comes out a blend of two, and the edge row a blend of the page and the ground. A
/// transition's faces are put at this same pixel (`crate::clock::face_as_shown`), so the effect's
/// last frame and the page that replaces it agree pixel for pixel (ADR 1865).
#[must_use]
pub fn whole_pixel(origin: (f32, f32)) -> (f32, f32) {
    (origin.0.round(), origin.1.round())
}

/// The window's sentence for a stand-in on its `frames` topic, `doc/todo/37`'s rule 3: which page,
/// at what size, scaled from what — so that a photograph of a blurred page is read as the stand-in
/// it is, and a frame line with no such sentence after a zoom says the frame landed first.
impl std::fmt::Display for Placed<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.held {
            return write!(
                f,
                "page {} holds its {}x{} frame where it was until the frame for this size lands or a refresh passes",
                self.frame.page.saturating_add(1),
                self.frame.raster.width,
                self.frame.raster.height
            );
        }
        if self.retained {
            return write!(
                f,
                "page {} stands in at {}x{}, its retained {}x{} page scaled until its frame lands",
                self.frame.page.saturating_add(1),
                self.extent.0,
                self.extent.1,
                self.frame.raster.width,
                self.frame.raster.height
            );
        }
        write!(
            f,
            "page {} stands in at {}x{}, its {}x{} frame scaled until the frame for this size lands",
            self.frame.page.saturating_add(1),
            self.extent.0,
            self.extent.1,
            self.frame.raster.width,
            self.frame.raster.height
        )
    }
}

/// Where each frame is shown: at the extent `geometry` says the page occupies, where the pixels
/// are the same shape at another size, and at their own size otherwise.
///
/// `geometry` is `viewer_core::Query::PageGeometry`'s answer, which this crate holds no viewer to
/// ask; a page it does not answer for keeps its pixels' own size.
///
/// **Only a uniform scale stands in.** A zoom multiplies both sides of a page by one factor, and
/// each side is rounded up to a whole pixel (`viewer_core`'s `raster_extent`), so the old pixels
/// and the new extent agree in shape to within that rounding: `old_h × new_w` and
/// `new_h × old_w` then differ by less than the sum of the four sides. A change of shape — §7.7.3.3's
/// `/Rotate` turned by the view, a quarter turn of anything but a square — is not a zoom, and
/// stretching the old pixels to it would show the page distorted; those keep their own size, as
/// every frame did before this module, until their frame lands.
#[must_use]
pub fn placed(
    frames: Vec<FrameView<'_>>,
    mut geometry: impl FnMut(usize) -> Option<PageGeometry>,
) -> Vec<Placed<'_>> {
    frames
        .into_iter()
        .map(|frame| {
            let now = geometry(frame.page);
            scaled(frame, now)
        })
        .collect()
}

/// `frame` with the extent [`placed`] gives it, `now` being what the arrangement says of its
/// page: the page's extent now where the pixels are the same shape at another size, and their
/// own size otherwise.
fn scaled(frame: FrameView<'_>, now: Option<PageGeometry>) -> Placed<'_> {
    let own = (frame.raster.width, frame.raster.height);
    let extent = now
        .map(|placed| (placed.width, placed.height))
        .filter(|&now| now == own || same_shape(own, now))
        .unwrap_or(own);
    Placed {
        frame,
        extent,
        retained: false,
        held: false,
    }
}

/// Whether `now` is `was` scaled by one factor on both axes, to within each side's rounding up.
fn same_shape(was: (u32, u32), now: (u32, u32)) -> bool {
    if was.0 == 0 || was.1 == 0 || now.0 == 0 || now.1 == 0 {
        return false;
    }
    // Two `u32` multiplied and four added are inside a `u64`, so neither saturation is reachable.
    let crossed = u64::from(was.1).saturating_mul(u64::from(now.0));
    let other = u64::from(now.1).saturating_mul(u64::from(was.0));
    let slack = [was.0, was.1, now.0, now.1]
        .into_iter()
        .map(u64::from)
        .fold(0_u64, u64::saturating_add);
    crossed.abs_diff(other) <= slack
}

/// The refresh period of a display that states none: a sixtieth of a second, the slowest rate
/// the project owner's "at least 60Hz but 120Hz should be the target" admits (ADR 0383), so that
/// an unknown display is held for no shorter than a known one would be.
pub const FLOOR: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// Thousandths of a hertz in a hertz: the unit a toolkit reports a refresh rate in.
const MILLIHERTZ: u64 = 1_000;

/// One page's frame as it was shown when a zoom command was taken: the geometry it had.
#[derive(Debug, Clone, Copy)]
struct Before {
    document: DocumentId,
    page: usize,
    /// The pixels' own width and height, which are the extent the page had.
    size: (u32, u32),
    /// Where the pixels' top-left corner was in the viewport.
    origin: (f32, f32),
}

/// The pictures a window keeps where they were for one refresh after a zoom command, so that a
/// zoomed frame which lands inside the refresh replaces a sharp picture rather than a blurred one
/// (A657, ADR 2510).
///
/// **The rule**: for one display period after a zoom command, a page whose frame has not landed
/// is painted as it was — its own pixels, at its own size, at the origin it had when the command
/// was taken, which is the whole old picture and not old pixels at the new geometry (that is ADR
/// 1843's defect) — and at the first look after the period it is painted as [`placed`] scales it.
/// `quorra` stands in only where a frame has missed the refresh (`doc/todo/37`'s rule 5, ADR
/// 0384); this is the same wait, kept here because a toolkit window has no prediction to make it
/// with.
///
/// **The period runs from the first command a frame has not answered.** A second notch while the
/// first is unanswered does not restart it: a gesture whose frames never land inside a refresh
/// stands in after one refresh and stays there, as it does in `quorra`, rather than showing the
/// picture from before the gesture for as long as it lasts.
///
/// **A page holds only the frame it was recorded with.** A frame that landed for an intermediate
/// magnification has other pixels, and the origin kept is not theirs; that page is scaled.
#[derive(Debug)]
pub struct Hold {
    period: Duration,
    before: Vec<Before>,
    /// When the command that started the running wait was taken.
    since: Option<Instant>,
    /// Whether the end of the period has not yet been told to the window, which owes a look then.
    owed: bool,
}

impl Default for Hold {
    fn default() -> Self {
        Self {
            period: FLOOR,
            before: Vec::new(),
            since: None,
            owed: false,
        }
    }
}

impl Hold {
    /// The period of a display refreshing at `millihertz`, or [`FLOOR`] where it states none.
    ///
    /// A rate of zero is the same answer written differently, as it is for `quorra`'s cadence.
    pub fn refresh_is(&mut self, millihertz: Option<u32>) {
        self.period = millihertz
            .filter(|rate| *rate > 0)
            .and_then(|rate| {
                1_000_000_000_u64
                    .saturating_mul(MILLIHERTZ)
                    .checked_div(u64::from(rate))
            })
            .map(Duration::from_nanos)
            .filter(|period| !period.is_zero())
            .unwrap_or(FLOOR);
    }

    /// One refresh of the display, which is how long a picture is held.
    #[must_use]
    pub fn period(&self) -> Duration {
        self.period
    }

    /// A zoom command is about to be handled: keeps where each of `document`'s frames is shown,
    /// for the pages whose frame is the size the page has, and starts the period unless one is
    /// running for a command no frame has answered yet.
    ///
    /// Asked *before* the viewer handles the command, because afterwards the geometry it answers
    /// is the new one and the old one is gone. A page that was already waiting keeps the record it
    /// has, so that a second notch holds the first notch's picture and not a blurred one.
    pub fn zoomed(
        &mut self,
        now: Instant,
        document: DocumentId,
        frames: &[FrameView<'_>],
        mut geometry: impl FnMut(usize) -> Option<PageGeometry>,
    ) {
        let mut kept = Vec::with_capacity(frames.len());
        let mut waiting = false;
        for frame in frames {
            let size = (frame.raster.width, frame.raster.height);
            if is_current(geometry(frame.page), size) {
                kept.push(Before {
                    document,
                    page: frame.page,
                    size,
                    origin: frame.origin,
                });
            } else if let Some(earlier) = self.recorded(document, frame.page, size) {
                waiting = true;
                kept.push(earlier);
            }
        }
        if !waiting {
            self.since = Some(now);
            self.owed = !kept.is_empty();
        }
        kept.sort_unstable_by_key(|before| (before.document, before.page));
        self.before = kept;
    }

    /// `frames` placed as [`placed`] places them, except that within the period a page whose frame
    /// has not landed is held where it was (`Placed::held`).
    #[must_use]
    pub fn placed<'a>(
        &self,
        now: Instant,
        document: DocumentId,
        frames: Vec<FrameView<'a>>,
        mut geometry: impl FnMut(usize) -> Option<PageGeometry>,
    ) -> Vec<Placed<'a>> {
        let running = self.running(now);
        frames
            .into_iter()
            .map(|frame| {
                let size = (frame.raster.width, frame.raster.height);
                let now_geometry = geometry(frame.page);
                if running
                    && !is_current(now_geometry, size)
                    && let Some(kept) = self.recorded(document, frame.page, size)
                {
                    return Placed {
                        frame: FrameView {
                            page: frame.page,
                            raster: frame.raster,
                            origin: kept.origin,
                        },
                        extent: size,
                        retained: false,
                        held: true,
                    };
                }
                scaled(frame, now_geometry)
            })
            .collect()
    }

    /// Whether the period ended since the window was last told, and a frame is still not there:
    /// true once, at the first look after it, which is when the window repaints to stand in.
    pub fn lapsed(
        &mut self,
        now: Instant,
        document: DocumentId,
        frames: &[FrameView<'_>],
        mut geometry: impl FnMut(usize) -> Option<PageGeometry>,
    ) -> bool {
        if !self.owed {
            return false;
        }
        let waiting = frames.iter().any(|frame| {
            let size = (frame.raster.width, frame.raster.height);
            !is_current(geometry(frame.page), size)
                && self.recorded(document, frame.page, size).is_some()
        });
        if !waiting {
            self.owed = false;
            return false;
        }
        if self.running(now) {
            return false;
        }
        self.owed = false;
        true
    }

    /// How long until the window must look again for [`Self::lapsed`], while a look is owed.
    #[must_use]
    pub fn wait(&self, now: Instant) -> Option<Duration> {
        let since = self.since.filter(|_| self.owed)?;
        Some(
            self.period
                .saturating_sub(now.saturating_duration_since(since)),
        )
    }

    /// Whether the period of the running wait has not ended.
    fn running(&self, now: Instant) -> bool {
        self.since
            .is_some_and(|since| now.saturating_duration_since(since) < self.period)
    }

    /// The record of `page`'s frame if it is the pixels of that `size`, found by bisection so that
    /// a paint of a column of thousands of pages stays linear in the pages.
    fn recorded(&self, document: DocumentId, page: usize, size: (u32, u32)) -> Option<Before> {
        let at = self
            .before
            .binary_search_by_key(&(document, page), |kept| (kept.document, kept.page))
            .ok()?;
        self.before
            .get(at)
            .copied()
            .filter(|kept| kept.size == size)
    }
}

/// Whether the page's frame is the size the arrangement gives the page now: the zoomed frame has
/// landed, or the page was never zoomed.
fn is_current(now: Option<PageGeometry>, size: (u32, u32)) -> bool {
    now.is_some_and(|now| (now.width, now.height) == size)
}

/// The longer side of a retained page, in pixels.
///
/// `quorra`'s `PROXY_EDGE`, for the reason ADR 0443 measured it by: the binding cost of a retained
/// page is memory, and 512 is 741 376 bytes for an A4 page, half the linear resolution of a page
/// filling an 800 × 1000 window — soft, unmistakably a stand-in, and unmistakably the page.
pub const RETAINED_EDGE: u32 = 512;

/// How many pages a window retains, when nobody says otherwise — `quorra`'s `PROXY_PAGES`, whose
/// whole set, 5.9 MB for A4, is smaller than one window-sized frame (ADR 0443).
pub const RETAINED_PAGES: usize = 8;

/// The word that sets how many pages a toolkit window retains, `quorra`'s own spelling.
///
/// The extent is the host's setting and nothing below a binary's command line — the project owner
/// asked on 2026-08-20 for it to be configurable (`doc/todo/37`) — and all three windows read it
/// under one name, so that a person measuring it against its own absence types the same thing.
pub const RETAINED_PAGES_WORD: &str = "--proxy-pages";

/// The count after [`RETAINED_PAGES_WORD`], or the sentence that says it is not one.
///
/// **Refused rather than defaulted**, for `quorra`'s reason: a launch that ignored a number it
/// could not read would leave a person measuring the default while believing they had changed it.
///
/// # Errors
///
/// The sentence, where `word` is absent or not a count of pages.
pub fn retained_pages(word: Option<&str>) -> Result<usize, String> {
    word.and_then(|count| count.parse::<usize>().ok())
        .ok_or_else(|| {
            format!(
                "{RETAINED_PAGES_WORD} wants a count of pages, 0 or more (default {RETAINED_PAGES})"
            )
        })
}

/// Source samples taken along each axis of one retained pixel's footprint.
///
/// A box filter over the whole footprint would cost the whole frame — up to `viewer_core`'s
/// `MAX_PIXELS` read on the toolkit's thread for a page zoomed far in — where this costs at most
/// `RETAINED_EDGE`² × 16 samples whatever the frame's size (ADR 1866). Four is the footprint of a
/// page at its usual fit, two to three frame pixels to one retained pixel, sampled at least once
/// per pixel; further out the reduction aliases, which a picture that is a stand-in may.
const TAPS: u32 = 4;

/// One retained page and which page of which document it is.
#[derive(Debug)]
struct Kept {
    document: DocumentId,
    page: usize,
    pixels: Raster,
}

/// The whole pages a window keeps a small picture of, newest first, so that a page with no frame
/// has something true to show (ADR 1866).
///
/// **What a picture is of is `(document, page, ink)`**, `viewer_core::RenderRequest::ink`'s
/// statement: two lists of a page under one ink are the same picture. Inks are per document, so
/// the store keeps the newest ink it has been told of for each, and a request under another ink
/// drops every picture of that document, because a layer switched off, a field typed into or an
/// appearance under the pointer is a page this store no longer holds a picture of — shown, it
/// would assert ink the page no longer has, which blur does not excuse.
///
/// **Bounded by [`RETAINED_PAGES`]**, oldest dropped: a picture is kept only from a frame that
/// has just landed, so the pages on the screen are always the newest entries.
#[derive(Debug)]
pub struct Retained {
    held: Vec<Kept>,
    inks: Vec<(DocumentId, u64)>,
    extent: usize,
}

impl Default for Retained {
    fn default() -> Self {
        Self::new(RETAINED_PAGES)
    }
}

impl Retained {
    /// A store that keeps up to `extent` pages; zero turns the whole of it off.
    #[must_use]
    pub fn new(extent: usize) -> Self {
        Self {
            held: Vec::new(),
            inks: Vec::new(),
            extent,
        }
    }

    /// The viewer asked for a page: the request's ink is the document's ink now.
    pub fn asked(&mut self, request: &RenderRequest) {
        self.ink_is(request.document, request.ink);
    }

    /// A frame of `request` finished as `raster`: keeps a reduction of it where none of that page
    /// at this ink is held, and answers whether it did.
    ///
    /// A frame drawn under an ink the store has since been told is gone keeps nothing.
    pub fn landed(&mut self, request: &RenderRequest, raster: &Raster) -> bool {
        self.keep(request.document, request.page, request.ink, raster)
    }

    fn ink_is(&mut self, document: DocumentId, ink: u64) {
        match self.inks.iter_mut().find(|(known, _)| *known == document) {
            Some((_, known)) if *known == ink => {}
            Some((_, known)) => {
                *known = ink;
                self.held.retain(|kept| kept.document != document);
            }
            None => self.inks.push((document, ink)),
        }
    }

    fn keep(&mut self, document: DocumentId, page: usize, ink: u64, raster: &Raster) -> bool {
        if self.extent == 0 {
            return false;
        }
        let current = self
            .inks
            .iter()
            .find(|(known, _)| *known == document)
            .is_none_or(|&(_, known)| known == ink);
        if !current {
            return false;
        }
        self.ink_is(document, ink);
        if self
            .held
            .iter()
            .any(|kept| kept.document == document && kept.page == page)
        {
            return false;
        }
        let Some(pixels) = reduced(raster) else {
            return false;
        };
        self.held.insert(
            0,
            Kept {
                document,
                page,
                pixels,
            },
        );
        self.held.truncate(self.extent);
        true
    }

    /// `placed` with a retained page added for every page of `focused` the arrangement places and
    /// no pixels are shown for, scaled to the extent `geometry` gives it.
    ///
    /// A page whose retained picture is not its shape now is left out rather than stretched, by
    /// [`placed`]'s rule.
    #[must_use]
    pub fn fill<'a>(
        &'a self,
        mut placed: Vec<Placed<'a>>,
        focused: DocumentId,
        mut geometry: impl FnMut(usize) -> Option<PageGeometry>,
    ) -> Vec<Placed<'a>> {
        // A held picture is the whole old arrangement; a retained page at the new geometry
        // beside it would be two arrangements on one screen (ADR 2510).
        if placed.iter().any(|shown| shown.held) {
            return placed;
        }
        for kept in self.held.iter().filter(|kept| kept.document == focused) {
            if placed.iter().any(|shown| shown.frame.page == kept.page) {
                continue;
            }
            let Some(now) = geometry(kept.page) else {
                continue;
            };
            let extent = (now.width, now.height);
            if !same_shape((kept.pixels.width, kept.pixels.height), extent) {
                continue;
            }
            placed.push(Placed {
                frame: FrameView {
                    page: kept.page,
                    raster: &kept.pixels,
                    origin: now.origin,
                },
                extent,
                retained: true,
                held: false,
            });
        }
        placed.sort_by_key(|shown| shown.frame.page);
        placed
    }

    /// How many pages are held, for a window's trace.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether nothing is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
}

/// `raster` reduced to [`RETAINED_EDGE`] along its longer side, each pixel the average of up to
/// [`TAPS`]² samples spread over its footprint and weighted by their alpha, or `None` for a raster
/// with no pixels. A raster already that small is kept as it is.
fn reduced(raster: &Raster) -> Option<Raster> {
    let RasterFormat::Rgba8 = raster.format;
    let (width, height) = (raster.width, raster.height);
    let expected = u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(4);
    if width == 0 || height == 0 || u64::try_from(raster.data.len()).ok() != Some(expected) {
        return None;
    }
    let longer = width.max(height);
    if longer <= RETAINED_EDGE {
        return Some(Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: raster.data.clone(),
        });
    }
    // Each side scaled by one factor and rounded up, as `viewer_core` rounds a page's extent, so
    // that the reduction is the page's shape to within the slack `placed` allows. Every product
    // below is of numbers under 2^32 and fits a `u64`, so no saturation is reachable.
    let side = |length: u32| {
        let scaled = u64::from(length)
            .saturating_mul(u64::from(RETAINED_EDGE))
            .div_ceil(u64::from(longer));
        u32::try_from(scaled).unwrap_or(RETAINED_EDGE).max(1)
    };
    let (out_width, out_height) = (side(width), side(height));
    let stride = usize::try_from(width).ok()?.checked_mul(4)?;
    let (across, columns) = taps(out_width, width, 4)?;
    let (down, rows) = taps(out_height, height, stride)?;
    let samples = u32::try_from(across.saturating_mul(down)).ok()?;
    let mut data = Vec::with_capacity(
        usize::try_from(
            u64::from(out_width)
                .saturating_mul(u64::from(out_height))
                .saturating_mul(4),
        )
        .ok()?,
    );
    for row in rows.chunks_exact(down) {
        for column in columns.chunks_exact(across) {
            // Premultiplied sums, so that a pixel half outside the page is the page's colour at
            // half its alpha rather than that colour darkened by the transparent black beside it.
            // A channel times an alpha is under 2^16 and there are at most `TAPS`² of them, so a
            // `u32` holds the sum and no saturation is reachable.
            let mut sum = [0_u32; 4];
            for &start in row {
                let Some(line) = raster.data.get(start..start.saturating_add(stride)) else {
                    continue;
                };
                for &offset in column {
                    let Some(&[r, g, b, a]) = line.get(offset..offset.saturating_add(4)) else {
                        continue;
                    };
                    let alpha = u32::from(a);
                    sum[0] = sum[0].saturating_add(u32::from(r).saturating_mul(alpha));
                    sum[1] = sum[1].saturating_add(u32::from(g).saturating_mul(alpha));
                    sum[2] = sum[2].saturating_add(u32::from(b).saturating_mul(alpha));
                    sum[3] = sum[3].saturating_add(alpha);
                }
            }
            let colour = |channel: u32| {
                let rounded = channel
                    .saturating_add(sum[3] / 2)
                    .checked_div(sum[3])
                    .unwrap_or(0);
                u8::try_from(rounded).unwrap_or(u8::MAX)
            };
            let alpha = sum[3]
                .saturating_add(samples / 2)
                .checked_div(samples)
                .unwrap_or(0);
            data.extend_from_slice(&[
                colour(sum[0]),
                colour(sum[1]),
                colour(sum[2]),
                u8::try_from(alpha).unwrap_or(u8::MAX),
            ]);
        }
    }
    Some(Raster {
        width: out_width,
        height: out_height,
        format: RasterFormat::Rgba8,
        data,
    })
}

/// Which source pixels each of `out` retained columns (or rows) is the average of, as byte offsets
/// `step` apart, and how many there are to each: the taps of retained pixel `index` sit at
/// `(index + (tap + ½) / taps) / out` of the way across `source`, in integers and never past the
/// last pixel, and there are as many as whole source pixels fit the footprint — one for a frame a
/// little over the edge, [`TAPS`] at most. Worked out once per reduction rather than per pixel.
fn taps(out: u32, source: u32, step: usize) -> Option<(usize, Vec<usize>)> {
    let taps = source.checked_div(out).unwrap_or(1).clamp(1, TAPS);
    let mut at = Vec::with_capacity(usize::try_from(out.saturating_mul(taps)).ok()?);
    let across = u64::from(taps)
        .saturating_mul(2)
        .saturating_mul(u64::from(out));
    for index in 0..out {
        for tap in 0..taps {
            let numerator = u64::from(index)
                .saturating_mul(u64::from(taps))
                .saturating_add(u64::from(tap))
                .saturating_mul(2)
                .saturating_add(1)
                .saturating_mul(u64::from(source));
            let pixel = numerator
                .checked_div(across)
                .unwrap_or(0)
                .min(u64::from(source.saturating_sub(1)));
            at.push(usize::try_from(pixel).ok()?.checked_mul(step)?);
        }
    }
    Some((usize::try_from(taps).ok()?, at))
}

#[cfg(test)]
mod tests {
    use pdf_render::{Raster, RasterFormat, Size};
    use viewer_core::{DocumentId, FrameView, PageGeometry};

    use std::time::{Duration, Instant};

    use super::{FLOOR, Hold, RETAINED_EDGE, Retained, placed, reduced};

    /// A raster of one colour, every byte of it real.
    fn filled(width: u32, height: u32, pixel: [u8; 4]) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: pixel
                .repeat(usize::try_from(width.saturating_mul(height)).expect("a test's raster")),
        }
    }

    /// The owner's case with no frame at all: a column zoomed out puts page 2 on the screen, the
    /// viewer holds pixels only for page 1, and page 2 was drawn before at the document's ink —
    /// so it is shown from its retained picture, scaled into its extent at its origin, and said
    /// apart from a frame standing in. A page whose own frame is up is not covered by one.
    #[test]
    fn a_page_with_no_frame_is_shown_from_its_retained_picture() {
        let document = DocumentId(1);
        let mut retained = Retained::default();
        assert!(retained.keep(document, 1, 7, &filled(1224, 1584, [0, 0, 255, 255])));
        assert!(retained.keep(document, 0, 7, &filled(1224, 1584, [255, 0, 0, 255])));
        let held = filled(612, 792, [255, 0, 0, 255]);
        let frames = placed(
            vec![FrameView {
                page: 0,
                raster: &held,
                origin: (0.0, 0.0),
            }],
            |_| Some(geometry(612, 792)),
        );
        let shown = retained.fill(frames, document, |_| Some(geometry(490, 634)));
        assert_eq!(
            shown.len(),
            2,
            "page 1's frame and page 2's retained picture"
        );
        assert!(!shown[0].retained, "page 1 is the viewer's own frame");
        assert!(shown[1].retained && shown[1].stands_in());
        assert_eq!(shown[1].frame.page, 1);
        assert_eq!(shown[1].extent, (490, 634));
        assert_eq!(shown[1].frame.origin, (10.0, 20.0));
        assert_eq!(
            (shown[1].frame.raster.width, shown[1].frame.raster.height),
            (396, RETAINED_EDGE)
        );
        assert!(shown[1].to_string().contains("retained 396x512 page"));
        assert!(
            retained
                .fill(Vec::new(), DocumentId(2), |_| Some(geometry(490, 634)))
                .is_empty(),
            "a picture of another document is never shown for this one"
        );
        assert!(
            retained.fill(Vec::new(), document, |_| None).is_empty(),
            "a page the arrangement does not place is not shown"
        );
    }

    /// `viewer_core::RenderRequest::ink`: a page under another ink is another picture, so a
    /// request under a new ink drops every picture of that document, and a frame that lands from
    /// under the old one keeps nothing.
    #[test]
    fn a_new_ink_drops_every_picture_of_its_document() {
        let (document, other) = (DocumentId(1), DocumentId(2));
        let mut retained = Retained::default();
        let page = filled(800, 1000, [0, 128, 0, 255]);
        assert!(retained.keep(document, 0, 3, &page));
        assert!(retained.keep(other, 0, 3, &page));
        assert!(
            !retained.keep(document, 0, 3, &page),
            "the same picture is not reduced twice"
        );
        retained.ink_is(document, 4);
        assert_eq!(
            retained.len(),
            1,
            "only the other document's picture is left"
        );
        assert!(
            !retained.keep(document, 1, 3, &page),
            "a frame of the old ink"
        );
        assert!(retained.keep(document, 1, 4, &page));
        let mut off = Retained::new(0);
        assert!(
            !off.keep(document, 0, 3, &page),
            "an extent of zero keeps nothing"
        );
        assert!(off.is_empty());
    }

    /// `--proxy-pages` takes a count and refuses anything else out loud.
    #[test]
    fn the_extent_is_a_count_or_a_refusal() {
        assert_eq!(super::retained_pages(Some("0")), Ok(0));
        assert_eq!(super::retained_pages(Some("12")), Ok(12));
        assert!(super::retained_pages(Some("eight")).is_err());
        assert!(super::retained_pages(None).is_err());
    }

    /// The store is bounded, oldest out.
    #[test]
    fn the_oldest_picture_goes_first() {
        let document = DocumentId(1);
        let mut retained = Retained::new(2);
        let page = filled(600, 600, [9, 9, 9, 255]);
        for number in 0..3 {
            assert!(retained.keep(document, number, 0, &page));
        }
        let shown = retained.fill(Vec::new(), document, |_| Some(geometry(600, 600)));
        let pages: Vec<usize> = shown.iter().map(|placed| placed.frame.page).collect();
        assert_eq!(pages, vec![1, 2]);
    }

    /// A reduction averages what it covers, weighted by alpha: a page of red and blue halves
    /// keeps both halves, and an edge of transparent pixels lowers the alpha and not the colour.
    #[test]
    fn a_reduction_averages_its_footprint_by_alpha() {
        let mut page = filled(1024, 1024, [255, 0, 0, 255]);
        for row in page.data.chunks_exact_mut(1024 * 4) {
            for pixel in row[512 * 4..].chunks_exact_mut(4) {
                pixel.copy_from_slice(&[0, 0, 255, 255]);
            }
        }
        let small = reduced(&page).expect("a page with pixels");
        assert_eq!((small.width, small.height), (512, 512));
        let row = 512 * 4;
        assert_eq!(&small.data[10 * row + 10 * 4..][..4], &[255, 0, 0, 255]);
        assert_eq!(&small.data[300 * row + 500 * 4..][..4], &[0, 0, 255, 255]);
        let mut edged = filled(1024, 1024, [255, 255, 255, 255]);
        for pixel in edged.data[..1024 * 4].chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0, 0, 0, 0]);
        }
        let small = reduced(&edged).expect("a page with pixels");
        assert_eq!(
            &small.data[..4],
            &[255, 255, 255, 128],
            "one of the two rows under it is clear, and its colour is still white"
        );
        let tiny = filled(300, 400, [1, 2, 3, 255]);
        assert_eq!(reduced(&tiny).expect("pixels").data, tiny.data);
        assert!(reduced(&filled(0, 0, [0; 4])).is_none());
    }

    fn raster(width: u32, height: u32) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            // `placed` reads a raster's extent and never its bytes.
            data: Vec::new(),
        }
    }

    fn geometry(width: u32, height: u32) -> PageGeometry {
        PageGeometry {
            page: Size {
                width: 612.0,
                height: 792.0,
            },
            scale: 1.0,
            width,
            height,
            origin: (10.0, 20.0),
        }
    }

    /// The owner's case, at the sizes the drive photographed: `quorra-qt` held 619 × 802 after a
    /// notch in had placed the page at 774 × 1002 (1.25 times both, each side rounded up).
    #[test]
    fn a_frame_from_before_a_zoom_fills_the_extent_the_page_has_now() {
        let old = raster(619, 802);
        let frames = vec![FrameView {
            page: 0,
            raster: &old,
            origin: (10.0, 20.0),
        }];
        let shown = placed(frames, |_| Some(geometry(774, 1002)));
        assert_eq!(shown[0].extent, (774, 1002));
        assert!(shown[0].stands_in());
        let out = vec![FrameView {
            page: 0,
            raster: &old,
            origin: (10.0, 20.0),
        }];
        let shown = placed(out, |_| Some(geometry(496, 642)));
        assert_eq!(shown[0].extent, (496, 642), "a notch out stands in too");
    }

    #[test]
    fn a_frame_at_its_own_size_is_shown_at_it() {
        let held = raster(619, 802);
        let frames = vec![FrameView {
            page: 0,
            raster: &held,
            origin: (0.0, 0.0),
        }];
        let shown = placed(frames, |_| Some(geometry(619, 802)));
        assert_eq!(shown[0].extent, (619, 802));
        assert!(!shown[0].stands_in());
    }

    /// A quarter turn is not a zoom: 619 × 802 stretched to 802 × 619 would be the page distorted.
    #[test]
    fn a_change_of_shape_is_not_stretched_into() {
        let held = raster(619, 802);
        let frames = vec![FrameView {
            page: 0,
            raster: &held,
            origin: (0.0, 0.0),
        }];
        let shown = placed(frames, |_| Some(geometry(802, 619)));
        assert_eq!(shown[0].extent, (619, 802));
        let frames = vec![FrameView {
            page: 0,
            raster: &held,
            origin: (0.0, 0.0),
        }];
        assert_eq!(placed(frames, |_| None)[0].extent, (619, 802));
    }

    /// The rounding the slack is there for, at its worst over a whole gesture: every step of
    /// `viewer_core`'s 1.25 from a twentieth of the page to sixteen times it, each side rounded up,
    /// is the same shape as every other.
    #[test]
    fn every_zoom_step_of_a_page_is_the_same_shape_as_every_other() {
        let side = |points: f64, magnification: f64| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a positive extent of at most ten thousand pixels"
            )]
            let pixels = (points * magnification).ceil() as u32;
            pixels
        };
        let steps: Vec<(u32, u32)> = (-13..=12)
            .map(|step| {
                let magnification = 1.25_f64.powi(step);
                (side(612.0, magnification), side(792.0, magnification))
            })
            .collect();
        for &was in &steps {
            for &now in &steps {
                assert!(super::same_shape(was, now), "{was:?} against {now:?}");
            }
        }
        assert!(!super::same_shape((612, 792), (792, 612)));
        assert!(!super::same_shape((0, 792), (612, 792)));
    }

    // ---- Hold (A657, ADR 2510) ----

    const DOCUMENT: DocumentId = DocumentId(1);

    /// The arrangement's answer for a page of `width` × `height` whose corner is at `origin`.
    fn geometry_at(width: u32, height: u32, origin: (f32, f32)) -> PageGeometry {
        PageGeometry {
            origin,
            ..geometry(width, height)
        }
    }

    fn frame_of(pixels: &Raster, page: usize, origin: (f32, f32)) -> FrameView<'_> {
        FrameView {
            page,
            raster: pixels,
            origin,
        }
    }

    /// The vertical offset of page `page` in a column of pages ten pixels tall.
    fn below(page: usize) -> f32 {
        f32::from(u16::try_from(page).expect("a test column has fewer than 65 536 pages")) * 10.0
    }

    /// A hold that took a zoom command at `start` while page 0 was shown at 100 x 100 at (10, 20).
    fn zoomed_at(start: Instant, pixels: &Raster) -> Hold {
        let mut hold = Hold::default();
        hold.zoomed(
            start,
            DOCUMENT,
            &[frame_of(pixels, 0, (10.0, 20.0))],
            |_| Some(geometry_at(100, 100, (10.0, 20.0))),
        );
        hold
    }

    /// A657: within a refresh of the command the whole old picture is painted where it was, and
    /// not the old pixels at the new origin (ADR 1843's defect) or scaled.
    #[test]
    fn within_a_refresh_the_old_picture_stays_where_it_was() {
        let start = Instant::now();
        let old = raster(100, 100);
        let hold = zoomed_at(start, &old);
        let shown = hold.placed(
            start + hold.period() / 2,
            DOCUMENT,
            vec![frame_of(&old, 0, (0.0, 0.0))],
            |_| Some(geometry_at(200, 200, (0.0, 0.0))),
        );
        assert!(shown[0].held && !shown[0].stands_in());
        assert_eq!(shown[0].extent, (100, 100));
        assert_eq!(shown[0].shown_at(), (10.0, 20.0));
        assert!(shown[0].waits_for_a_frame());
    }

    /// At the first look after the refresh the frame has missed it, and the page is scaled.
    #[test]
    fn after_a_refresh_the_page_stands_in_scaled() {
        let start = Instant::now();
        let old = raster(100, 100);
        let hold = zoomed_at(start, &old);
        let shown = hold.placed(
            start + hold.period(),
            DOCUMENT,
            vec![frame_of(&old, 0, (0.0, 0.0))],
            |_| Some(geometry_at(200, 200, (0.0, 0.0))),
        );
        assert!(!shown[0].held && shown[0].stands_in());
        assert_eq!(shown[0].extent, (200, 200));
    }

    /// A frame that lands inside the refresh is painted as any frame is, with no sentence.
    #[test]
    fn a_frame_that_landed_inside_the_refresh_is_painted_as_it_is() {
        let start = Instant::now();
        let old = raster(100, 100);
        let hold = zoomed_at(start, &old);
        let landed = raster(200, 200);
        let shown = hold.placed(
            start + Duration::from_millis(1),
            DOCUMENT,
            vec![frame_of(&landed, 0, (0.0, 0.0))],
            |_| Some(geometry_at(200, 200, (0.0, 0.0))),
        );
        assert!(!shown[0].held && !shown[0].waits_for_a_frame());
        assert_eq!(shown[0].shown_at(), (0.0, 0.0));
    }

    /// The window repaints once at the end of the period, and only where a frame is still owed.
    #[test]
    fn the_end_of_the_refresh_is_told_once_and_only_to_a_page_still_waiting() {
        let start = Instant::now();
        let old = raster(100, 100);
        let mut hold = zoomed_at(start, &old);
        let waiting = |size: u32| move |_: usize| Some(geometry_at(size, size, (0.0, 0.0)));
        let frames = [frame_of(&old, 0, (0.0, 0.0))];
        assert_eq!(hold.wait(start), Some(hold.period()));
        assert!(!hold.lapsed(start, DOCUMENT, &frames, waiting(200)));
        assert!(hold.lapsed(start + hold.period(), DOCUMENT, &frames, waiting(200)));
        assert!(!hold.lapsed(start + hold.period(), DOCUMENT, &frames, waiting(200)));
        assert_eq!(hold.wait(start + hold.period()), None);
        // A frame that landed first has nothing left to tell.
        let mut landed = zoomed_at(start, &old);
        assert!(!landed.lapsed(start + landed.period(), DOCUMENT, &frames, waiting(100)));
        assert_eq!(landed.wait(start), None);
    }

    /// A second notch while the first is unanswered does not restart the refresh: a gesture
    /// whose frames never land stands in after one refresh, as `quorra`'s does.
    #[test]
    fn a_second_notch_does_not_restart_the_refresh() {
        let start = Instant::now();
        let old = raster(100, 100);
        let mut hold = zoomed_at(start, &old);
        let second = start + hold.period() / 2;
        hold.zoomed(second, DOCUMENT, &[frame_of(&old, 0, (0.0, 0.0))], |_| {
            Some(geometry_at(200, 200, (0.0, 0.0)))
        });
        let shown = |at| {
            hold.placed(at, DOCUMENT, vec![frame_of(&old, 0, (0.0, 0.0))], |_| {
                Some(geometry_at(300, 300, (0.0, 0.0)))
            })[0]
                .held
        };
        assert!(
            shown(second),
            "the first notch's picture is still the one held"
        );
        assert!(!shown(start + hold.period()));
    }

    /// A frame of another size than the one recorded is other pixels, and the origin kept is not
    /// theirs: that page is scaled.
    #[test]
    fn a_frame_that_is_not_the_recorded_one_is_scaled() {
        let start = Instant::now();
        let hold = zoomed_at(start, &raster(100, 100));
        let between = raster(150, 150);
        let shown = hold.placed(
            start,
            DOCUMENT,
            vec![frame_of(&between, 0, (0.0, 0.0))],
            |_| Some(geometry_at(300, 300, (0.0, 0.0))),
        );
        assert!(!shown[0].held && shown[0].stands_in());
    }

    /// No retained page is put beside a held one: the old arrangement is shown whole. The same
    /// store adds its page beside a frame that is not held.
    #[test]
    fn no_retained_page_is_added_beside_a_held_picture() {
        let start = Instant::now();
        let old = raster(100, 100);
        let hold = zoomed_at(start, &old);
        let mut retained = Retained::default();
        assert!(retained.keep(DOCUMENT, 1, 0, &filled(100, 100, [9, 9, 9, 255])));
        let arrangement = |page: usize| Some(geometry_at(200, 200, (0.0, below(page) * 20.0)));
        let on_hold = hold.placed(
            start,
            DOCUMENT,
            vec![frame_of(&old, 0, (0.0, 0.0))],
            arrangement,
        );
        assert!(on_hold[0].held);
        assert_eq!(retained.fill(on_hold, DOCUMENT, arrangement).len(), 1);
        let scaled = placed(vec![frame_of(&old, 0, (0.0, 0.0))], arrangement);
        assert_eq!(retained.fill(scaled, DOCUMENT, arrangement).len(), 2);
    }

    /// The period is the display's, and the floor where it states none.
    #[test]
    fn the_period_is_the_displays_and_a_sixtieth_where_it_states_none() {
        let mut hold = Hold::default();
        assert_eq!(hold.period(), FLOOR);
        hold.refresh_is(Some(120_000));
        assert_eq!(hold.period(), Duration::from_nanos(8_333_333));
        hold.refresh_is(Some(0));
        assert_eq!(hold.period(), FLOOR);
        hold.refresh_is(Some(144_000));
        hold.refresh_is(None);
        assert_eq!(hold.period(), FLOOR);
        hold.refresh_is(Some(u32::MAX));
        assert!(hold.period() > Duration::ZERO);
    }

    /// The bound: a column of thousands of pages is recorded and painted in one pass each, found
    /// by bisection, and every page is held.
    #[test]
    fn a_column_of_thousands_of_pages_is_held_whole() {
        let start = Instant::now();
        let old = raster(10, 10);
        let pages = 5_000_usize;
        let frames: Vec<FrameView<'_>> = (0..pages)
            .rev()
            .map(|page| frame_of(&old, page, (0.0, below(page))))
            .collect();
        let mut hold = Hold::default();
        hold.zoomed(start, DOCUMENT, &frames, |page| {
            Some(geometry_at(10, 10, (0.0, below(page))))
        });
        let shown = hold.placed(start, DOCUMENT, frames, |_| {
            Some(geometry_at(20, 20, (0.0, 0.0)))
        });
        assert_eq!(shown.len(), pages);
        assert!(shown.iter().all(|page| page.held));
    }
}
