//! The pixels a confined frame becomes, and the bookkeeping between the two.
//!
//! [`viewer_confined::Reply::Frame`] arrives with one [`Framed`] per page of Table 29's
//! arrangement, and each crossed the pipe as one of two payloads (ADR 0607): the pixels the
//! worker drew, or the marks for this side to draw. The first kind is ready the moment it
//! arrives; the second goes to [`viewer_host::Drawing`]'s thread, because nothing bounds what a
//! display list costs to draw (ADR 0650) and a window may not be taken hostage by one — the
//! whole of `viewer_host::drawing`'s module comment applies here unchanged, which is why the
//! arrangement is that one and not a copy.
//!
//! What this module owns is the gap between those two moments: which page is ready, which is
//! still being drawn, which refused — and the one identity check that keeps a slow draw from
//! landing on a view that has moved on (trap 12a's neighbour: pixels drawn *for* one target may
//! not be placed as though they were another's).

use std::sync::Arc;

use pdf_render::{DisplayList, Raster, RasterFormat, TargetSpec};
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
}

impl Screen {
    /// A screen with no frame yet.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The window grew or shrank; the next [`Self::compose`] fills the new extent.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.extent = (width, height);
    }

    /// Takes one frame as it crossed the confinement, queueing what still needs drawing.
    ///
    /// Three cases per page, and the middle one is the point: pixels are ready as they arrive; a
    /// list identical to the one already drawn keeps its pixels at the new origin, which is what
    /// makes a scroll a re-placement rather than a re-rasterisation; and any other list is handed
    /// to `drawing`, whose own rule replaces a queued or in-flight draw for the same page.
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
                Payload::Raster(raster) => Content::Pixels(raster, None),
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
                slot.content = Content::Pixels(raster, Some((request.list, request.target)));
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
}
