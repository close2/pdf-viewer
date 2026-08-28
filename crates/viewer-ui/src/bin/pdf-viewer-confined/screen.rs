//! The pages a confined frame becomes, and the bookkeeping between the two.
//!
//! [`viewer_confined::Reply::Frame`] arrives with one [`Framed`] per page of Table 29's
//! arrangement, and each crossed the pipe as one of two payloads (ADR 0607): the pixels the
//! worker drew, or the marks for this side to draw. What becomes of them depends on which
//! screen this is (ADR 0725). **A device screen** — the ordinary one — holds every page as
//! something the graphics device draws: the marks as they crossed, the pixels wrapped as a
//! one-image list, each handed whole to [`crate::device`] per frame. **A processor screen**
//! (`--cpu`, or a machine whose device would not come up) places pixels as they arrive and
//! sends the marks to [`viewer_host::Drawing`]'s thread, because nothing bounds what a display
//! list costs to draw (ADR 0650) and a window may not be taken hostage by one — the whole of
//! `viewer_host::drawing`'s module comment applies here unchanged, which is why the arrangement
//! is that one and not a copy. The device screen reaches that same thread through
//! [`Screen::fall_back`], for the frames the device refuses.
//!
//! What this module owns is the gap between arrival and pixels: which page is ready, which is
//! still being drawn, which refused — and the one identity check that keeps a slow draw from
//! landing on a view that has moved on (trap 12a's neighbour: pixels drawn *for* one target may
//! not be placed as though they were another's).

use std::sync::Arc;

use pdf_render::{
    BlendMode, Command as Mark, DisplayList, Image, Raster, RasterFormat, Size, TargetSpec,
    Transform,
};
use viewer_confined::{Framed, Payload};
use viewer_core::Rendered;
use viewer_host::drawing::{DrawRequest, Drawing, Finished};

/// One page's marks on their way to the drawing thread.
///
/// The confined counterpart of [`viewer_core::RenderRequest`], and the reason
/// [`viewer_host::DrawRequest`] exists: the three facts a drawing needs are here, and the
/// `RenderToken` a tier-1 request carries cannot be, because the viewer that mints tokens is on
/// the far side of the pipe. Nothing is owed to that viewer for these pixels either — the worker
/// recorded the page as [`viewer_core::Rendered::Listed`] when it shipped the marks (ADR 0640) —
/// so what this request answers to is the screen alone.
#[derive(Debug, Clone)]
pub(crate) struct Draw {
    /// Which page of the document, zero-based.
    pub page: usize,
    /// The marks, exactly as they crossed.
    pub list: Arc<DisplayList>,
    /// What the confined process would have drawn into, carried rather than rebuilt
    /// ([`Payload::List`] says why).
    pub target: TargetSpec,
}

impl DrawRequest for Draw {
    fn page(&self) -> usize {
        self.page
    }

    fn list(&self) -> &Arc<DisplayList> {
        &self.list
    }

    fn target(&self) -> TargetSpec {
        self.target
    }
}

/// What one page of the arrangement has come to on this side of the pipe.
#[derive(Debug)]
enum Content {
    /// The marks are on the drawing thread, or queued behind another page's.
    ///
    /// The pair is the drawing's *identity*: a [`Finished`] whose list is not this `Arc` or whose
    /// target is not this one answers a question this screen no longer asks — a zoom or a resize
    /// replaced it — and its pixels are dropped rather than placed. `Arc` identity rather than
    /// comparing commands, for the same reason the wire format interns by it: it is exact and it
    /// costs a pointer.
    Drawing(Arc<DisplayList>, TargetSpec),
    /// The pixels, from either arm: crossed as [`Payload::Raster`], or drawn here from the marks.
    ///
    /// The pair they were drawn from is kept beside them so that the next frame can tell "the
    /// same drawing, moved" — a scroll, which changes only where the page sits — from "a new
    /// drawing" — a zoom, which changes the target. The first keeps these pixels; the second
    /// queues a draw. A page that crossed as pixels keeps `None` and is replaced whenever the
    /// worker sends new ones, because the worker only re-sends what changed enough to re-draw.
    Pixels(Raster, Option<(Arc<DisplayList>, TargetSpec)>),
    /// A device screen's page, awaiting no thread: the marks exactly as they crossed, with the
    /// page-sized target beside them (ADR 0725).
    ///
    /// The device draws these at present time — that is what the marks crossed *for* (ADR 0607)
    /// — so unlike [`Self::Drawing`] this is an answered page: nothing gates presentation on it,
    /// and nothing but a device refusal ([`Screen::fall_back`]) moves it to the thread.
    Marks(Arc<DisplayList>, TargetSpec),
    /// A device screen's pixels, wrapped as the one-command list the device draws them by.
    ///
    /// The wrapper's `Arc` is its identity for the device's retained scene, so it is built once
    /// and kept: a raster payload recrossing with the same bytes keeps this wrapper, which is
    /// what makes a scroll of a photo page a placement change rather than a re-upload. `drawn`
    /// carries what the pixels were drawn from where they came off the fallback thread — the
    /// same identity [`Self::Pixels`] keeps, for the same scroll-keeps-them reason — and `None`
    /// where they crossed as pixels.
    Wrapped {
        /// One [`Mark::Image`] drawing the page's pixels 1:1 at the page's own size.
        list: Arc<DisplayList>,
        /// What the pixels were drawn from, `None` where they crossed as pixels.
        drawn: Option<(Arc<DisplayList>, TargetSpec)>,
    },
    /// The rasteriser refused the page, in its own words.
    ///
    /// Kept rather than retried: a rasteriser that refused this list at this target will refuse
    /// it again (the same reasoning as `viewer_core::Rendered::Failed`), and trap 5 wants the
    /// sentence on the screen's status rather than a quiet gap.
    Refused(String),
}

/// One page of the arrangement, where it sits, and what it has come to.
#[derive(Debug)]
struct Slot {
    /// Which page of the document, zero-based.
    page: usize,
    /// Where the page's top-left corner sits in the window, in device pixels — [`Framed::origin`].
    origin: (f32, f32),
    content: Content,
}

/// Every page the current frame shows, in the arrangement's own order.
#[derive(Debug, Default)]
pub(crate) struct Screen {
    /// The window's extent in device pixels, which is what [`Self::compose`] fills.
    extent: (u32, u32),
    slots: Vec<Slot>,
    /// Whether a graphics device draws this screen's pages (ADR 0725).
    ///
    /// A device screen keeps a list payload as [`Content::Marks`] for the device instead of
    /// queueing it on the drawing thread, and wraps a raster payload as the one-command list the
    /// device draws it by. The thread stays what `CLAUDE.md` keeps the CPU backend for — the
    /// frame the device refuses — reached through [`Self::fall_back`].
    device: bool,
}

impl Screen {
    /// A screen with no frame yet, whose pages the drawing thread rasterises.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A screen with no frame yet, whose pages the graphics device draws (ADR 0725).
    #[must_use]
    pub(crate) fn for_device() -> Self {
        Self {
            device: true,
            ..Self::default()
        }
    }

    /// Whether this screen's pages are the graphics device's to draw.
    #[must_use]
    pub(crate) fn draws_on_the_device(&self) -> bool {
        self.device
    }

    /// The window grew or shrank; the next [`Self::compose`] fills the new extent.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.extent = (width, height);
    }

    /// Takes one frame as it crossed the confinement, queueing what still needs drawing.
    ///
    /// On a processor screen, three cases per page, and the middle one is the point: pixels are
    /// ready as they arrive; a list identical to the one already drawn keeps its pixels at the
    /// new origin, which is what makes a scroll a re-placement rather than a re-rasterisation;
    /// and any other list is handed to `drawing`, whose own rule replaces a queued or in-flight
    /// draw for the same page.
    ///
    /// On a device screen the same identity checks keep the same things — a held wrapper, a
    /// fallback draw in flight or finished — and what they keep it *for* is the device's
    /// retained scene, which is keyed by these very `Arc`s: an unchanged page recrossing must
    /// reach the device as the same page (the identity `viewer_confined::Confined` preserves
    /// across frames, ADR 0725).
    ///
    /// A page the new arrangement no longer shows takes the drawing thread back
    /// ([`Drawing::superseded`]'s rule, decided here because only this side knows the new
    /// arrangement): pixels drawn for it could never reach the window.
    pub(crate) fn take(&mut self, frames: Vec<Framed>, drawing: &mut Drawing<Draw>) {
        let previous = std::mem::take(&mut self.slots);
        let mut kept: Vec<Option<Slot>> = previous.into_iter().map(Some).collect();
        for framed in frames {
            let Framed {
                page,
                payload,
                origin,
            } = framed;
            let before = kept
                .iter_mut()
                .find(|slot| slot.as_ref().is_some_and(|slot| slot.page == page))
                .and_then(Option::take);
            let content = match payload {
                Payload::Raster(raster) if self.device => {
                    match before.map(|slot| slot.content) {
                        // The same pixels, recrossed: the wrapper's `Arc` is the identity the
                        // device's retained scene keys by, so keeping it is what makes a scroll
                        // of a photo page a placement change rather than a re-upload. Byte
                        // equality, because the worker is the untrusted side and nothing weaker
                        // may stand in for "the same pixels".
                        Some(Content::Wrapped { list, drawn: None })
                            if wraps_exactly(&list, &raster) =>
                        {
                            Content::Wrapped { list, drawn: None }
                        }
                        _ => Content::Wrapped {
                            list: wrap(raster),
                            drawn: None,
                        },
                    }
                }
                Payload::Raster(raster) => Content::Pixels(raster, None),
                Payload::List { list, target } if self.device => {
                    match before.map(|slot| slot.content) {
                        // The same marks at the same target: the device holds a retained scene
                        // keyed by this `Arc`, and a scroll must reach it as the same page.
                        Some(Content::Marks(held_list, held_target))
                            if Arc::ptr_eq(&held_list, &list) && held_target == target =>
                        {
                            Content::Marks(held_list, held_target)
                        }
                        // The same drawing, still on the fallback thread after a device refusal:
                        // asking again would interrupt the very draw whose answer is wanted.
                        Some(Content::Drawing(asked_list, asked_target))
                            if Arc::ptr_eq(&asked_list, &list) && asked_target == target =>
                        {
                            Content::Drawing(asked_list, asked_target)
                        }
                        // The same drawing, fallen back and finished: a scroll keeps the pixels
                        // the thread drew — going back to the device would re-refuse.
                        Some(Content::Wrapped {
                            list: wrapper,
                            drawn: Some((drawn_list, drawn_target)),
                        }) if Arc::ptr_eq(&drawn_list, &list) && drawn_target == target => {
                            Content::Wrapped {
                                list: wrapper,
                                drawn: Some((drawn_list, drawn_target)),
                            }
                        }
                        // The same drawing, already refused by both: retrying a refusal is a loop.
                        Some(Content::Refused(words)) => Content::Refused(words),
                        _ => Content::Marks(list, target),
                    }
                }
                Payload::List { list, target } => {
                    match before.map(|slot| slot.content) {
                        // The same drawing, moved: a scroll changes where the page sits and
                        // nothing about its pixels.
                        Some(Content::Pixels(pixels, Some((drawn_list, drawn_target))))
                            if Arc::ptr_eq(&drawn_list, &list) && drawn_target == target =>
                        {
                            Content::Pixels(pixels, Some((drawn_list, drawn_target)))
                        }
                        // The same drawing, still on the thread: asking again would interrupt
                        // the very draw whose answer is wanted.
                        Some(Content::Drawing(asked_list, asked_target))
                            if Arc::ptr_eq(&asked_list, &list) && asked_target == target =>
                        {
                            Content::Drawing(asked_list, asked_target)
                        }
                        // The same drawing, already refused: retrying a refusal is a loop.
                        Some(Content::Refused(words)) => Content::Refused(words),
                        _ => {
                            drawing.ask(Draw {
                                page,
                                list: Arc::clone(&list),
                                target,
                            });
                            Content::Drawing(list, target)
                        }
                    }
                }
            };
            self.slots.push(Slot {
                page,
                origin,
                content,
            });
        }
        // The thread may be inside a page that just left the arrangement.
        if let Some(inside) = drawing.inside() {
            let shown = self.slots.iter().any(|slot| slot.page == inside);
            drawing.superseded(shown);
        }
    }

    /// A finished draw, placed if this screen still asks the question it answers.
    ///
    /// Answers whether a pixel of the composition changed, so the caller knows whether a redraw
    /// is owed. An abandoned draw (`outcome: None`) is owed to nobody — trap 20's rule, which
    /// holds here even without a viewer to freeze: the request that replaced it is already queued
    /// or the page is gone.
    pub(crate) fn landed(&mut self, finished: Finished<Draw>) -> bool {
        let Finished {
            request, outcome, ..
        } = finished;
        let Some(slot) = self.slots.iter_mut().find(|slot| slot.page == request.page) else {
            return false;
        };
        let Content::Drawing(asked_list, asked_target) = &slot.content else {
            return false;
        };
        if !Arc::ptr_eq(asked_list, &request.list) || *asked_target != request.target {
            return false;
        }
        match outcome {
            Some(Rendered::Raster(raster)) => {
                slot.content = if self.device {
                    // The fallback pixels go back through the device as the page they are —
                    // wrapped once, keyed by the wrapper's `Arc` like every other page it draws.
                    Content::Wrapped {
                        list: wrap(raster),
                        drawn: Some((request.list, request.target)),
                    }
                } else {
                    Content::Pixels(raster, Some((request.list, request.target)))
                };
                true
            }
            Some(Rendered::Failed(words)) => {
                slot.content = Content::Refused(words);
                true
            }
            // `Presented` and `Listed` are statements a host makes *to* a viewer, and this thread
            // draws rasters; `None` is a draw taken back, and nothing is owed for it.
            Some(Rendered::Presented | Rendered::Listed) | None => false,
        }
    }

    /// Whether every page of the frame has been answered — with pixels or with a refusal.
    ///
    /// What the caller gates presentation on once something is on the screen: a half-drawn
    /// arrangement replacing a whole one would be a window blinking its pages in and out on
    /// every zoom. `doc/todo/37`'s rule — show what it had — with nothing reprojected yet.
    #[must_use]
    pub(crate) fn settled(&self) -> bool {
        self.slots
            .iter()
            .all(|slot| !matches!(slot.content, Content::Drawing(..)))
    }

    /// The sentences of every page the rasteriser refused, for a status line.
    pub(crate) fn refusals(&self) -> impl Iterator<Item = (usize, &str)> {
        self.slots.iter().filter_map(|slot| match &slot.content {
            Content::Refused(words) => Some((slot.page, words.as_str())),
            _ => None,
        })
    }

    /// The window's pixels: [`pdf_render::SURROUND`] with every ready page placed on it.
    ///
    /// `None` for a window with no extent — minimised, or between a resize and its first frame —
    /// which is not a failure: there is nothing to compose *for*.
    ///
    /// The surround rather than a toolkit background, for the reason all three windows share
    /// (`pdf_render::medium`): what lies outside every page is no clause's subject, and one
    /// documented choice in one place is what keeps the hosts saying one thing. A page still
    /// being drawn leaves the surround showing, which is exactly what the gate in
    /// [`Self::settled`] keeps off the screen after the first frame.
    #[must_use]
    pub(crate) fn compose(&self) -> Option<Raster> {
        let (width, height) = self.extent;
        if width == 0 || height == 0 {
            return None;
        }
        let mut window = Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: surround_pixel().repeat((width as usize).saturating_mul(height as usize)),
        };
        for slot in &self.slots {
            if let Content::Pixels(raster, _) = &slot.content {
                blit(&mut window, raster, slot.origin);
            }
        }
        Some(window)
    }

    /// Every page the device is to draw, placed into a window of this extent (ADR 0725).
    ///
    /// One entry per [`Content::Marks`] and [`Content::Wrapped`] slot, in the arrangement's own
    /// order: the marks under the page-sized target they crossed with, the wrapped pixels under
    /// a 1:1 target of their own construction, each composed with the slot's origin exactly as
    /// [`blit`] would have placed the pixels — the same rounding, so the two paths put a page's
    /// top-left corner on the same device pixel. A page still on the fallback thread contributes
    /// nothing, which is the surround showing through until it lands.
    pub(crate) fn device_pages(
        &self,
        width: u32,
        height: u32,
    ) -> Vec<(Arc<DisplayList>, TargetSpec)> {
        self.slots
            .iter()
            .filter_map(|slot| match &slot.content {
                Content::Marks(list, target) => Some((
                    Arc::clone(list),
                    placed(*target, slot.origin, width, height),
                )),
                Content::Wrapped { list, .. } => Some((
                    Arc::clone(list),
                    placed(image_target(list), slot.origin, width, height),
                )),
                Content::Drawing(..) | Content::Pixels(..) | Content::Refused(_) => None,
            })
            .collect()
    }

    /// The device refused the frame: every page it was to draw goes to the drawing thread.
    ///
    /// This is `CLAUDE.md`'s second job for the CPU backend reached from the device screen — a
    /// frame the graphics device refuses is drawn on the processor, out loud — and it goes
    /// through [`viewer_host::Drawing`] rather than being composed where the refusal was seen,
    /// because that thread is the one with an interrupt (ADR 0650): on this boundary the marks
    /// are a document's, and a hostile page must not hold whichever thread draws it. Answers how
    /// many pages were handed over, so the caller can say so.
    pub(crate) fn fall_back(&mut self, drawing: &mut Drawing<Draw>) -> usize {
        let mut asked = 0usize;
        for slot in &mut self.slots {
            if let Content::Marks(list, target) = &slot.content {
                drawing.ask(Draw {
                    page: slot.page,
                    list: Arc::clone(list),
                    target: *target,
                });
                asked = asked.saturating_add(1);
                slot.content = Content::Drawing(Arc::clone(list), *target);
            }
        }
        asked
    }
}

/// The page's pixels as the one-command list the device draws them by.
///
/// The image occupies the unit square with its top row at y = 1 ([`Mark::Image`]'s convention),
/// so the transform scales it to the page — one page unit per pixel — and [`image_target`]'s
/// y flip puts the top row at the top, exactly as [`TargetSpec::for_page`] constructs a page's.
fn wrap(raster: Raster) -> Arc<DisplayList> {
    let Raster {
        width,
        height,
        format: RasterFormat::Rgba8,
        data,
    } = raster;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster dimension is bounded by pdf_render::MAX_EXTENT (2^24), inside f32's \
                  exact integer range"
    )]
    let (page_width, page_height) = (width as f32, height as f32);
    let mut list = DisplayList::new(Size {
        width: page_width,
        height: page_height,
    });
    list.push(Mark::Image {
        image: Image {
            width,
            height,
            data: data.into(),
            interpolate: false,
        }
        .into(),
        transform: Transform::scale(page_width, page_height),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    Arc::new(list)
}

/// Whether `list` is [`wrap`] of exactly these pixels — dimensions and every byte.
fn wraps_exactly(list: &DisplayList, raster: &Raster) -> bool {
    let [Mark::Image { image, .. }] = list.commands() else {
        return false;
    };
    let pdf_render::ImageSource::Decoded(image) = image else {
        return false;
    };
    image.width == raster.width && image.height == raster.height && *image.data == *raster.data
}

/// The 1:1 target a wrapped page's own pixels want: no scaling, and the same y flip about the
/// page's top edge as [`TargetSpec::for_page`] builds (its comment carries the argument).
fn image_target(list: &DisplayList) -> TargetSpec {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a wrapped page's size is its raster's dimensions, whole and bounded by \
                  pdf_render::MAX_EXTENT"
    )]
    TargetSpec {
        width: list.page_size.width as u32,
        height: list.page_size.height as u32,
        transform: Transform::scale(1.0, -1.0)
            .then(Transform::translate(0.0, list.page_size.height)),
    }
}

/// A page's target placed into the window: the page-space transform carried onto the window's
/// pixels at the slot's origin, rounded exactly as [`blit`] rounds — so the device and the
/// processor put a page's top-left corner on the same device pixel.
fn placed(target: TargetSpec, origin: (f32, f32), width: u32, height: u32) -> TargetSpec {
    TargetSpec {
        width,
        height,
        transform: target
            .transform
            .then(Transform::translate(origin.0.round(), origin.1.round())),
    }
}

/// One RGBA pixel of [`pdf_render::SURROUND`].
fn surround_pixel() -> [u8; 4] {
    let level = |component: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a colour component clamped to 0..=1 scaled by 255"
        )]
        {
            (component.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
        }
    };
    let colour = pdf_render::SURROUND;
    [level(colour.r), level(colour.g), level(colour.b), 255]
}

/// Places one page's pixels on the window at `origin`, clipped to both.
///
/// Row copies rather than a compositing loop: a page's raster is opaque — `render-cpu` has
/// already imposed it on its medium — so placing it is a `memcpy` per row, which is the same
/// arithmetic the two native hosts hand their toolkits with a texture and an offset.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every operand is clipped to the two rasters' extents first, and an extent is \
              bounded by pdf_render::MAX_EXTENT (2^24), so the widest value here — a byte \
              offset in a window — fits an i64 with twenty bits to spare"
)]
fn blit(window: &mut Raster, page: &Raster, origin: (f32, f32)) {
    /// A device coordinate as an integer, exactly for every value a window can hold.
    fn rounded(value: f32) -> i64 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "clamped to i32's range first, every value of which i64 holds exactly"
        )]
        {
            value.round().clamp(-2_147_483_648.0, 2_147_483_647.0) as i64
        }
    }

    let (ox, oy) = (rounded(origin.0), rounded(origin.1));
    let (window_w, window_h) = (i64::from(window.width), i64::from(window.height));
    let (page_w, page_h) = (i64::from(page.width), i64::from(page.height));
    let x0 = ox.max(0);
    let y0 = oy.max(0);
    let x1 = (ox + page_w).min(window_w);
    let y1 = (oy + page_h).min(window_h);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let row_bytes = usize::try_from((x1 - x0) * 4).unwrap_or(0);
    for y in y0..y1 {
        let Ok(into) = usize::try_from((y * window_w + x0) * 4) else {
            return;
        };
        let Ok(from) = usize::try_from(((y - oy) * page_w + (x0 - ox)) * 4) else {
            return;
        };
        // `get` rather than indexing: the page's length is the worker's claim about its own
        // extent, and a claim short of `width * height * 4` must cost the copy, not the process.
        let (Some(into), Some(from)) = (
            window.data.get_mut(into..into + row_bytes),
            page.data.get(from..from + row_bytes),
        ) else {
            return;
        };
        into.copy_from_slice(from);
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test fixtures whose dimensions are single digits; an overflow here is a wrong test"
)]
mod tests {
    use std::sync::Arc;

    use pdf_render::{
        BlendMode, Color, Command as Mark, DisplayList, FillRule, Paint, Path as MarkPath,
        PathCommand, Point, Raster, RasterFormat, Size, TargetSpec, Transform,
    };
    use viewer_confined::{Framed, Payload};
    use viewer_host::drawing::{Drawing, Finished};

    use super::{Content, Draw, Screen, surround_pixel};

    /// A page-covering fill repeated `fills` times, so a drawn raster is distinguishable from
    /// the surround — and, with enough repeats, slow enough to be interrupted.
    fn an_expensive_list(width: f32, height: f32, fills: usize) -> Arc<DisplayList> {
        let mut list = DisplayList::new(Size { width, height });
        let mut path = MarkPath::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(width, 0.0)));
        path.push(PathCommand::LineTo(Point::new(width, height)));
        path.push(PathCommand::LineTo(Point::new(0.0, height)));
        path.push(PathCommand::Close);
        let path = Arc::new(path);
        for _ in 0..fills {
            list.push(Mark::Fill {
                path: Arc::clone(&path),
                transform: Transform::IDENTITY,
                paint: Paint::Solid(Color::rgb(1.0, 0.0, 0.0)),
                fill_rule: FillRule::NonZero,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        Arc::new(list)
    }

    /// One page-covering fill.
    fn a_list(width: f32, height: f32) -> Arc<DisplayList> {
        an_expensive_list(width, height, 1)
    }

    /// A solid raster whose every pixel is `colour`.
    fn a_raster(width: u32, height: u32, colour: [u8; 4]) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: colour.repeat((width as usize) * (height as usize)),
        }
    }

    /// The pixel at `(x, y)` of a composition.
    fn pixel(window: &Raster, x: usize, y: usize) -> [u8; 4] {
        let at = (y * window.width as usize + x) * 4;
        [
            window.data[at],
            window.data[at + 1],
            window.data[at + 2],
            window.data[at + 3],
        ]
    }

    /// Waits for the drawing thread, then hands everything it finished to the screen.
    fn drain(screen: &mut Screen, drawing: &mut Drawing<Draw>) {
        let began = std::time::Instant::now();
        while !screen.settled() {
            for finished in drawing.collect() {
                screen.landed(finished);
            }
            assert!(
                began.elapsed() < std::time::Duration::from_mins(2),
                "the drawing thread never answered"
            );
            std::thread::sleep(viewer_host::drawing::POLL);
        }
    }

    /// A raster payload is ready the moment it arrives, exactly where its origin says.
    #[test]
    fn a_raster_payload_is_placed_at_its_origin() {
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.resize(40, 40);
        screen.take(
            vec![Framed {
                page: 0,
                payload: Payload::Raster(a_raster(10, 10, [1, 2, 3, 255])),
                origin: (5.0, 7.0),
            }],
            &mut drawing,
        );
        assert!(screen.settled(), "pixels need no drawing thread");
        let window = screen.compose().expect("a window with an extent composes");
        assert_eq!(pixel(&window, 5, 7), [1, 2, 3, 255]);
        assert_eq!(pixel(&window, 14, 16), [1, 2, 3, 255]);
        assert_eq!(pixel(&window, 4, 7), surround_pixel(), "left of the page");
        assert_eq!(pixel(&window, 15, 7), surround_pixel(), "right of the page");
    }

    /// A page placed partly off the window is clipped, on all four sides, rather than anything
    /// panicking or wrapping.
    #[test]
    fn a_page_off_the_window_edge_is_clipped() {
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.resize(20, 20);
        screen.take(
            vec![
                Framed {
                    page: 0,
                    payload: Payload::Raster(a_raster(10, 10, [9, 9, 9, 255])),
                    origin: (-5.0, -5.0),
                },
                Framed {
                    page: 1,
                    payload: Payload::Raster(a_raster(10, 10, [7, 7, 7, 255])),
                    origin: (15.0, 15.0),
                },
            ],
            &mut drawing,
        );
        let window = screen.compose().expect("a window with an extent composes");
        assert_eq!(pixel(&window, 0, 0), [9, 9, 9, 255], "clipped top-left");
        assert_eq!(pixel(&window, 4, 4), [9, 9, 9, 255]);
        assert_eq!(pixel(&window, 5, 5), surround_pixel());
        assert_eq!(
            pixel(&window, 19, 19),
            [7, 7, 7, 255],
            "clipped bottom-right"
        );
        assert_eq!(pixel(&window, 14, 14), surround_pixel());
    }

    /// A list payload is drawn on the drawing thread and lands as pixels.
    #[test]
    fn a_list_payload_is_drawn_and_then_composes() {
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let list = a_list(8.0, 8.0);
        let target = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX))
            .expect("an 8x8 page has a target");
        screen.take(
            vec![Framed {
                page: 0,
                payload: Payload::List { list, target },
                origin: (2.0, 2.0),
            }],
            &mut drawing,
        );
        assert!(!screen.settled(), "the marks are on the thread");
        drain(&mut screen, &mut drawing);
        let window = screen.compose().expect("a window with an extent composes");
        assert_eq!(pixel(&window, 5, 5), [255, 0, 0, 255], "the fill was drawn");
        assert_eq!(pixel(&window, 1, 1), surround_pixel());
    }

    /// The identity check: a finished draw for a target this screen no longer asks about is
    /// dropped, not placed — a zoom's new frame must never show the old magnification's pixels.
    #[test]
    fn a_stale_draw_is_dropped_rather_than_placed() {
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let list = a_list(8.0, 8.0);
        let old = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX)).expect("a target");
        let new = TargetSpec::for_page(&list, 2.0, u64::from(u32::MAX)).expect("a target");
        screen.take(
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target: new,
                },
                origin: (0.0, 0.0),
            }],
            &mut drawing,
        );
        let stale = Finished {
            request: Draw {
                page: 0,
                list: Arc::clone(&list),
                target: old,
            },
            outcome: Some(viewer_core::Rendered::Raster(a_raster(
                8,
                8,
                [5, 5, 5, 255],
            ))),
            cost: std::time::Duration::ZERO,
            waited: std::time::Duration::ZERO,
        };
        assert!(
            !screen.landed(stale),
            "an old target's pixels changed nothing"
        );
        assert!(!screen.settled(), "the real draw is still owed");
        drain(&mut screen, &mut drawing);
        let window = screen.compose().expect("a window with an extent composes");
        assert_ne!(
            pixel(&window, 1, 1),
            [5, 5, 5, 255],
            "the stale pixels never landed"
        );
    }

    /// A scroll re-places pixels rather than re-drawing them: the same list at the same target
    /// arriving with a new origin keeps what was drawn.
    #[test]
    fn a_moved_page_keeps_its_pixels() {
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let list = a_list(8.0, 8.0);
        let target = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX)).expect("a target");
        let framed = |origin| {
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                origin,
            }]
        };
        screen.take(framed((0.0, 0.0)), &mut drawing);
        drain(&mut screen, &mut drawing);
        screen.take(framed((10.0, 10.0)), &mut drawing);
        assert!(
            screen.settled(),
            "the same drawing at a new origin queued nothing"
        );
        let window = screen.compose().expect("a window with an extent composes");
        assert_eq!(
            pixel(&window, 12, 12),
            [255, 0, 0, 255],
            "moved, not redrawn"
        );
        assert_eq!(
            pixel(&window, 2, 2),
            surround_pixel(),
            "and gone from where it was"
        );
    }

    /// A page that leaves the arrangement takes the drawing thread back: the draw comes back
    /// abandoned, and nothing of it reaches the screen.
    ///
    /// The list is expensive — the construction `viewer_host::drawing`'s own tests use — so that
    /// the interrupt has something to interrupt; a cheap page would finish first and the test
    /// would pass whether or not `take` raised anything.
    #[test]
    fn a_page_that_leaves_the_arrangement_interrupts_its_draw() {
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let list = an_expensive_list(600.0, 800.0, 20_000);
        let target = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX)).expect("a target");
        screen.take(
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                origin: (0.0, 0.0),
            }],
            &mut drawing,
        );
        // The next frame shows page 1 only; page 0's draw is answering nobody.
        screen.take(
            vec![Framed {
                page: 1,
                payload: Payload::Raster(a_raster(4, 4, [8, 8, 8, 255])),
                origin: (0.0, 0.0),
            }],
            &mut drawing,
        );
        let began = std::time::Instant::now();
        let outcome = loop {
            if let Some(finished) = drawing.collect().pop() {
                break finished;
            }
            assert!(
                began.elapsed() < std::time::Duration::from_mins(2),
                "the drawing thread never answered"
            );
            std::thread::sleep(viewer_host::drawing::POLL);
        };
        assert_eq!(outcome.request.page, 0);
        assert!(
            outcome.outcome.is_none(),
            "the draw was not taken back when its page left the arrangement"
        );
        assert!(!screen.landed(outcome), "a departed page changed a pixel");
        assert!(screen.settled());
        assert!(
            matches!(
                screen.slots.first().map(|slot| (&slot.content, slot.page)),
                Some((&Content::Pixels(..), 1))
            ),
            "what remains is page 1's pixels"
        );
    }

    /// A device screen keeps a list payload for the device: nothing reaches the drawing thread,
    /// the page counts as answered, and its placement is the page's own target carried onto the
    /// window at the slot's origin (ADR 0725).
    #[test]
    fn a_device_screen_keeps_marks_for_the_device_not_the_thread() {
        let mut screen = Screen::for_device();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let list = a_list(8.0, 8.0);
        let target = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX)).expect("a target");
        screen.take(
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                origin: (2.4, 3.6),
            }],
            &mut drawing,
        );
        assert!(screen.settled(), "the device answers at present time");
        assert!(
            drawing.interval().is_none(),
            "nothing was queued on the drawing thread"
        );
        let pages = screen.device_pages(30, 30);
        let [(device_list, placed)] = pages.as_slice() else {
            panic!("one page for the device");
        };
        assert!(Arc::ptr_eq(device_list, &list), "the marks as they crossed");
        assert_eq!(
            (placed.width, placed.height),
            (30, 30),
            "the window's extent"
        );
        // The placement is the page target's transform translated by the rounded origin —
        // the same rounding `blit` applies, so both paths agree on the device pixel.
        let expected = target.transform.then(Transform::translate(2.0, 4.0));
        assert_eq!(placed.transform, expected);
    }

    /// A raster payload wraps once, and identical bytes recrossing keep the wrapper's `Arc` —
    /// the identity the device's retained scene is keyed by. Different bytes replace it.
    #[test]
    fn a_raster_page_wraps_once_and_keeps_its_identity() {
        let mut screen = Screen::for_device();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let framed = |colour: [u8; 4]| {
            vec![Framed {
                page: 0,
                payload: Payload::Raster(a_raster(4, 4, colour)),
                origin: (0.0, 0.0),
            }]
        };
        screen.take(framed([1, 2, 3, 255]), &mut drawing);
        let first = screen.device_pages(30, 30);
        screen.take(framed([1, 2, 3, 255]), &mut drawing);
        let second = screen.device_pages(30, 30);
        assert!(
            Arc::ptr_eq(&first[0].0, &second[0].0),
            "the same pixels keep their wrapper"
        );
        screen.take(framed([9, 9, 9, 255]), &mut drawing);
        let third = screen.device_pages(30, 30);
        assert!(
            !Arc::ptr_eq(&second[0].0, &third[0].0),
            "new pixels are a new page"
        );
        // The wrapper draws the pixels 1:1: page size and target are the raster's dimensions.
        let (wrapper, placed) = &third[0];
        assert_eq!(wrapper.page_size, Size::new(4.0, 4.0));
        assert_eq!((placed.width, placed.height), (30, 30));
    }

    /// A device refusal hands every marks page to the drawing thread — `CLAUDE.md`'s second job
    /// for the CPU backend — and the landed pixels come back as a wrapped page whose identity a
    /// scroll then keeps, rather than going back to the device that refused them.
    #[test]
    fn a_refused_frame_falls_back_to_the_thread_and_a_scroll_keeps_the_result() {
        let mut screen = Screen::for_device();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let list = a_list(8.0, 8.0);
        let target = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX)).expect("a target");
        let framed = |origin| {
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                origin,
            }]
        };
        screen.take(framed((0.0, 0.0)), &mut drawing);
        assert_eq!(screen.fall_back(&mut drawing), 1, "one page fell back");
        assert!(
            screen.device_pages(30, 30).is_empty(),
            "a page on the thread is not the device's to draw"
        );
        assert!(!screen.settled(), "the fallback draw is owed");
        drain(&mut screen, &mut drawing);
        let landed = screen.device_pages(30, 30);
        assert_eq!(landed.len(), 1, "the fallback pixels are the device's page");
        // A scroll re-crosses the same list at the same target with a new origin: the wrapped
        // fallback pixels are kept — going back to the device that refused would loop.
        screen.take(framed((10.0, 10.0)), &mut drawing);
        assert!(screen.settled(), "nothing was re-queued");
        let moved = screen.device_pages(30, 30);
        assert!(
            Arc::ptr_eq(&landed[0].0, &moved[0].0),
            "moved, not redrawn: the wrapper's identity survives the scroll"
        );
        assert_ne!(
            landed[0].1.transform, moved[0].1.transform,
            "and what changed is the placement"
        );
    }
}
