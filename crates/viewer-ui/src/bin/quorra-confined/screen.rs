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
//! **A page past `viewer_core::MAX_PIXELS` is drawn on the processor as the window shows it**
//! ([`Content::Windowed`], ADR 1944): the worker sends such a page's marks because this window
//! said it draws them (`Command::Oversize`), the device draws them at the window's size as it
//! draws every page, and the processor — `--cpu`, or a frame the device refused — draws the part
//! of the page the window shows with `viewer_host::oversize`'s margin round it, so no raster of the
//! whole page is allocated on this side of the pipe either. **So is a page under the bound whose
//! window the toolkit windows would draw in its place** — `viewer_host::oversize::worth_a_window`,
//! asked here of the marks that crossed, because this side holds them and only this side knows
//! that the processor is what draws them (ADR 2026).
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
use viewer_host::oversize::{self, Window};

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
        /// One [`pdf_render::Command::Image`] drawing the page's pixels 1:1 at the page's own size.
        list: Arc<DisplayList>,
        /// What the pixels were drawn from, `None` where they crossed as pixels.
        drawn: Option<(Arc<DisplayList>, TargetSpec)>,
    },
    /// A page the processor draws as the window shows it: one past `viewer_core::MAX_PIXELS`
    /// (ADR 1944), or one under it whose window the toolkit windows' rule draws in its place
    /// ([`drawn_as_windows`], ADR 2026).
    Windowed(Windowed),
    /// A window the worker drew of a page past the bound whose marks cannot cross, placed where
    /// the frame says it is (`Payload::Window`, ADR 1986). The worker draws the next one when a
    /// scroll leaves it, so nothing here asks.
    Sent(Window, WindowPixels),
    /// The rasteriser refused the page, in its own words, and what it refused.
    ///
    /// Kept rather than retried **for that drawing and no other**: a rasteriser that refused this
    /// list at this target will refuse it again (the same reasoning as
    /// `viewer_core::Rendered::Failed`), and trap 5 wants the sentence on the screen's status
    /// rather than a quiet gap. The pair is what makes "that drawing" checkable, and it is the
    /// same identity every other variant here keeps.
    ///
    /// **A refusal is about a drawing, not about a page.** A target whose pixels exceed
    /// `viewer_core::MAX_PIXELS` is refused, so the commonest refusal this window can show is
    /// one a zoom *out* lifts and a re-interpretation replaces outright — and a refusal held
    /// against the page would have made the first magnification that overshot permanent for the
    /// rest of the document's life on the screen. It was, from ADR 0713 until this pair existed.
    Refused {
        /// The rasteriser's own sentence.
        words: String,
        /// The list and target that were refused, by the identity [`Self::Drawing`] keeps.
        of: (Arc<DisplayList>, TargetSpec),
    },
}

/// A page drawn on the processor as the window shows it, and what is drawn of it.
#[derive(Debug)]
struct Windowed {
    /// The marks and the whole target the window is a part of — the identity every other variant
    /// keeps.
    of: (Arc<DisplayList>, TargetSpec),
    /// The window on the drawing thread, until it lands.
    asked: Option<Window>,
    /// The last window that landed, and its pixels.
    drawn: Option<(Window, WindowPixels)>,
}

impl Windowed {
    /// Whether this is a window of that list at that target.
    fn is_of(&self, list: &Arc<DisplayList>, target: TargetSpec) -> bool {
        Arc::ptr_eq(&self.of.0, list) && self.of.1 == target
    }
}

/// A window's pixels: placed as they are on a processor screen, wrapped as the one-image list the
/// device places on a device screen whose frame the device refused.
#[derive(Debug)]
enum WindowPixels {
    /// The raster [`Screen::compose`] blits.
    Raster(Raster),
    /// [`wrap`] of the raster, for [`Screen::device_pages`].
    Wrapped(Arc<DisplayList>),
}

/// Whether a target's whole raster would be past `viewer_core::MAX_PIXELS` — a page the worker
/// sends as its marks because this window draws it at the window's size (ADR 1944).
fn past_the_bound(target: TargetSpec) -> bool {
    u64::from(target.width).saturating_mul(u64::from(target.height)) > viewer_core::MAX_PIXELS
}

/// Whether the processor draws `list` at `target` as the window shows it rather than whole, at a
/// viewport `viewport` device pixels across: a page past the bound always (ADR 1944), and one under
/// it where `viewer_host::oversize::worth_a_window` holds — the rule the toolkit windows draw every
/// page by, the very function and not a statement of it again (ADRs 1994, 2026).
///
/// **Asked once for a drawing, as the toolkits ask it once for a request**: a drawing this screen
/// already holds, as windows or whole, keeps the answer it was given, so a scroll asks the list
/// nothing and a page is never drawn whole and then as windows at one target. A new list or a new
/// target is a new drawing and is asked. A window with no extent shows no window, so a page under
/// the bound is drawn whole there, as the worker draws it behind no viewport.
///
/// **The question costs one stride of the list on the window's thread**
/// ([`pdf_render::DisplayList::share_reaching`], at most 4 096 of its commands, and none where the
/// window is half its page or less): a pass bounded by the bytes that crossed, which decoding the
/// frame has already paid once.
fn drawn_as_windows(
    before: Option<&Content>,
    list: &Arc<DisplayList>,
    target: TargetSpec,
    viewport: (u32, u32),
) -> bool {
    let same = |held: &Arc<DisplayList>, at: TargetSpec| Arc::ptr_eq(held, list) && at == target;
    match before {
        _ if past_the_bound(target) => true,
        Some(Content::Windowed(held)) if held.is_of(list, target) => true,
        Some(
            Content::Pixels(_, Some((held, at)))
            | Content::Drawing(held, at)
            | Content::Refused { of: (held, at), .. },
        ) if same(held, *at) => false,
        _ => viewport.0 > 0 && viewport.1 > 0 && oversize::worth_a_window(list, target, viewport),
    }
}

/// Where a window of a page whose top-left corner is at `origin` is placed: the page's corner,
/// rounded as [`blit`] rounds it, moved by the window's own corner.
fn window_origin(origin: (f32, f32), window: Window) -> (f32, f32) {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a window's corner is inside a target under pdf_render::MAX_EXTENT (2^24), which \
                  f32 holds exactly"
    )]
    let corner = (window.x as f32, window.y as f32);
    (origin.0.round() + corner.0, origin.1.round() + corner.1)
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

    /// The window's extent in device pixels.
    #[must_use]
    pub(crate) fn extent(&self) -> (u32, u32) {
        self.extent
    }

    /// How many commands of marks the device draws for this screen: the price a device frame is
    /// divided by (ADR 2017).
    #[must_use]
    pub(crate) fn device_commands(&self) -> usize {
        self.slots
            .iter()
            .map(|slot| match &slot.content {
                Content::Marks(list, _) => list.commands().len(),
                _ => 0,
            })
            .sum()
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
    /// **On both, a refusal is kept only for the drawing that earned it** — the same identity,
    /// applied to the one variant that used to ignore it ([`Content::Refused`] says what that
    /// cost). A new list or a new target is a question the rasteriser has not been asked, so it
    /// is asked.
    ///
    /// A page the new arrangement no longer shows takes the drawing thread back
    /// ([`Drawing::superseded`]'s rule, decided here because only this side knows the new
    /// arrangement): pixels drawn for it could never reach the window.
    pub(crate) fn take(&mut self, frames: Vec<Framed>, drawing: &mut Drawing<Draw>) {
        let previous = std::mem::take(&mut self.slots);
        let mut kept: Vec<Option<Slot>> = previous.into_iter().map(Some).collect();
        for framed in frames {
            // The bytes that crossed price the crossing (`pricing`) and place nothing.
            let Framed {
                page,
                payload,
                origin,
                bytes: _,
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
                Payload::Window {
                    target: _,
                    corner,
                    raster,
                } => sent(before.map(|slot| slot.content), corner, raster, self.device),
                Payload::List { list, target } if self.device => on_the_device(
                    before.map(|slot| slot.content),
                    (page, origin, self.extent),
                    list,
                    target,
                    drawing,
                ),
                Payload::List { list, target }
                    if drawn_as_windows(
                        before.as_ref().map(|slot| &slot.content),
                        &list,
                        target,
                        self.extent,
                    ) =>
                {
                    windowed(
                        before.map(|slot| slot.content),
                        (page, origin, self.extent),
                        list,
                        target,
                        drawing,
                    )
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
                        // The same drawing, already refused: retrying a refusal is a loop. A
                        // different one is a question the rasteriser has not been asked.
                        Some(Content::Refused { words, of })
                            if Arc::ptr_eq(&of.0, &list) && of.1 == target =>
                        {
                            Content::Refused { words, of }
                        }
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
                slot.content = Content::Refused {
                    words,
                    of: (request.list, request.target),
                };
                true
            }
            // `Presented` and `Listed` are statements a host makes *to* a viewer, and this thread
            // draws rasters; `None` is a draw taken back, and nothing is owed for it.
            Some(Rendered::Presented | Rendered::Listed) | None => false,
        }
    }

    /// A finished window ([`Content::Windowed`]), placed if this screen still asks for a window of
    /// that drawing. Answers whether a pixel of the composition changed, as [`Self::landed`] does.
    pub(crate) fn landed_window(&mut self, finished: Finished<Draw>, window: Window) -> bool {
        let Finished {
            request, outcome, ..
        } = finished;
        let device = self.device;
        let Some(slot) = self.slots.iter_mut().find(|slot| slot.page == request.page) else {
            return false;
        };
        let Content::Windowed(held) = &mut slot.content else {
            return false;
        };
        if !held.is_of(&request.list, request.target) {
            return false;
        }
        if held.asked == Some(window) {
            held.asked = None;
        }
        match outcome {
            Some(Rendered::Raster(raster)) => {
                let pixels = if device {
                    WindowPixels::Wrapped(wrap(raster))
                } else {
                    WindowPixels::Raster(raster)
                };
                held.drawn = Some((window, pixels));
                true
            }
            Some(Rendered::Failed(words)) => {
                slot.content = Content::Refused {
                    words,
                    of: (request.list, request.target),
                };
                true
            }
            // A window taken back is owed to nobody, as a page's draw is (trap 20); the next frame
            // asks for the part the window shows again.
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
        self.slots.iter().all(|slot| !waiting(&slot.content))
    }

    /// Whether any page of the arrangement has an answer to show — pixels, marks, or a refusal's
    /// sentence — rather than the surround alone.
    #[must_use]
    pub(crate) fn shows_a_page(&self) -> bool {
        self.slots.iter().any(|slot| !waiting(&slot.content))
    }

    /// The sentences of every page the rasteriser refused, for a status line.
    pub(crate) fn refusals(&self) -> impl Iterator<Item = (usize, &str)> {
        self.slots.iter().filter_map(|slot| match &slot.content {
            Content::Refused { words, .. } => Some((slot.page, words.as_str())),
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
            match &slot.content {
                Content::Pixels(raster, _) => blit(&mut window, raster, slot.origin),
                Content::Windowed(Windowed {
                    drawn: Some((drawn, WindowPixels::Raster(raster))),
                    ..
                })
                | Content::Sent(drawn, WindowPixels::Raster(raster)) => {
                    blit(&mut window, raster, window_origin(slot.origin, *drawn));
                }
                _ => {}
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
                Content::Windowed(Windowed {
                    drawn: Some((drawn, WindowPixels::Wrapped(list))),
                    ..
                })
                | Content::Sent(drawn, WindowPixels::Wrapped(list)) => Some((
                    Arc::clone(list),
                    placed(
                        image_target(list),
                        window_origin(slot.origin, *drawn),
                        width,
                        height,
                    ),
                )),
                Content::Drawing(..)
                | Content::Pixels(..)
                | Content::Windowed(..)
                | Content::Sent(..)
                | Content::Refused { .. } => None,
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
        let extent = self.extent;
        for slot in &mut self.slots {
            if let Content::Marks(list, target) = &slot.content
                && drawn_as_windows(None, list, *target, extent)
            {
                slot.content = windowed(
                    None,
                    (slot.page, slot.origin, extent),
                    Arc::clone(list),
                    *target,
                    drawing,
                );
                asked = asked.saturating_add(1);
            } else if let Content::Marks(list, target) = &slot.content {
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

/// A list payload on a device screen: what was held of the same drawing kept — the device's
/// page, a fallback on the thread or landed, the windows of a page past the bound, a refusal —
/// and any other drawing offered to the device first.
fn on_the_device(
    before: Option<Content>,
    (page, origin, viewport): (usize, (f32, f32), (u32, u32)),
    list: Arc<DisplayList>,
    target: TargetSpec,
    drawing: &mut Drawing<Draw>,
) -> Content {
    match before {
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
        }) if Arc::ptr_eq(&drawn_list, &list) && drawn_target == target => Content::Wrapped {
            list: wrapper,
            drawn: Some((drawn_list, drawn_target)),
        },
        // The same page past the bound, fallen back to the processor's windows:
        // a scroll asks for the part the window now shows, where it was not drawn.
        Some(Content::Windowed(held)) if held.is_of(&list, target) => windowed(
            Some(Content::Windowed(held)),
            (page, origin, viewport),
            list,
            target,
            drawing,
        ),
        // The same drawing, already refused by both: retrying a refusal is a loop.
        // A *different* drawing is not that loop and falls through to the device,
        // which is what lets a zoom out lift a refusal the zoom in earned.
        Some(Content::Refused { words, of }) if Arc::ptr_eq(&of.0, &list) && of.1 == target => {
            Content::Refused { words, of }
        }
        _ => Content::Marks(list, target),
    }
}

/// Whether a page's content is still owed by the drawing thread: a draw in flight, or a window
/// asked for with none of the page drawn yet.
fn waiting(content: &Content) -> bool {
    match content {
        Content::Drawing(..) => true,
        Content::Windowed(held) => held.asked.is_some() && held.drawn.is_none(),
        _ => false,
    }
}

/// A page drawn as its windows on the processor ([`drawn_as_windows`], ADRs 1944, 2026): what was
/// held of that drawing kept — a refusal of it, or the windows of it — and the part the window now
/// shows, with `viewer_host::oversize`'s margin, asked of the drawing thread where neither the
/// window drawn nor the one asked for holds it.
fn windowed(
    before: Option<Content>,
    (page, origin, viewport): (usize, (f32, f32), (u32, u32)),
    list: Arc<DisplayList>,
    target: TargetSpec,
    drawing: &mut Drawing<Draw>,
) -> Content {
    let mut held = match before {
        Some(Content::Refused { words, of }) if Arc::ptr_eq(&of.0, &list) && of.1 == target => {
            return Content::Refused { words, of };
        }
        Some(Content::Windowed(held)) if held.is_of(&list, target) => held,
        _ => Windowed {
            of: (list, target),
            asked: None,
            drawn: None,
        },
    };
    let Some(shown) = oversize::shown_at(target, origin, viewport) else {
        return Content::Windowed(held);
    };
    let covered = |window: Option<Window>| window.is_some_and(|window| window.holds(shown));
    if !covered(held.asked) && !covered(held.drawn.as_ref().map(|(window, _)| *window)) {
        let window = oversize::with_margin(shown, target, viewport);
        drawing.ask_window(
            Draw {
                page,
                list: Arc::clone(&held.of.0),
                target,
            },
            window,
        );
        held.asked = Some(window);
    }
    Content::Windowed(held)
}

/// A window the worker drew (ADR 1986): blitted as it is on a processor screen, wrapped as the
/// one-image list the device places on a device screen — the held wrapper kept where the same
/// window recrossed with the same pixels, for the reason a raster payload's is kept.
fn sent(before: Option<Content>, corner: (u32, u32), raster: Raster, device: bool) -> Content {
    let window = Window {
        x: corner.0,
        y: corner.1,
        width: raster.width,
        height: raster.height,
    };
    if !device {
        return Content::Sent(window, WindowPixels::Raster(raster));
    }
    match before {
        Some(Content::Sent(held, WindowPixels::Wrapped(list)))
            if held == window && wraps_exactly(&list, &raster) =>
        {
            Content::Sent(held, WindowPixels::Wrapped(list))
        }
        _ => Content::Sent(window, WindowPixels::Wrapped(wrap(raster))),
    }
}

/// The page's pixels as the one-command list the device draws them by.
///
/// The image occupies the unit square with its top row at y = 1 ([`pdf_render::Command::Image`]'s
/// convention), so the transform scales it to the page — one page unit per pixel — and
/// [`image_target`]'s y flip puts the top row at the top, exactly as [`TargetSpec::for_page`]
/// constructs a page's.
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
            // A screen of the viewer's own pixels, not a document's image: nothing in §11
            // composites it, so its alpha states the rectangle it covers (ADR 1022).
            sample_alpha: pdf_render::SampleAlpha::Shape,
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
            for (finished, window) in drawing.windows() {
                screen.landed_window(finished, window);
            }
            assert!(
                began.elapsed() < std::time::Duration::from_mins(2),
                "the drawing thread never answered"
            );
            std::thread::sleep(viewer_host::drawing::POLL);
        }
    }

    /// A page-covering fill at a magnification whose whole raster is past `MAX_PIXELS`: 8 × 4 096
    /// pixels on a side, 2³⁰ in all.
    fn past_the_bound() -> (Arc<DisplayList>, TargetSpec) {
        let list = a_list(8.0, 8.0);
        let target = TargetSpec::for_page(&list, 4096.0, u64::MAX).expect("a target");
        assert!(
            u64::from(target.width) * u64::from(target.height) > viewer_core::MAX_PIXELS,
            "the test is about a page past the bound"
        );
        (list, target)
    }

    /// A page past the bound on the processor is drawn as the window shows it: the window and its
    /// margin, placed at its part of the page, and nothing the size of the page (ADR 1944).
    #[test]
    fn a_page_past_the_bound_is_drawn_as_the_window_shows_it() {
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.resize(40, 40);
        let (list, target) = past_the_bound();
        let framed = |origin| {
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                bytes: 0,
                origin,
            }]
        };
        // The page's corner 1 000 pixels up and to the left of the window's.
        screen.take(framed((-1000.0, -1000.0)), &mut drawing);
        assert!(!screen.settled(), "the window is on the drawing thread");
        drain(&mut screen, &mut drawing);
        let Some(Content::Windowed(held)) = screen.slots.first().map(|slot| &slot.content) else {
            panic!("the page is held as a window");
        };
        let Some((window, super::WindowPixels::Raster(raster))) = held.drawn.as_ref() else {
            panic!("its window landed as pixels");
        };
        // The viewport, and a quarter of it on every side.
        assert_eq!(
            (window.x, window.y, window.width, window.height),
            (990, 990, 60, 60)
        );
        assert_eq!((raster.width, raster.height), (60, 60), "a window's pixels");
        let composed = screen.compose().expect("a window with an extent composes");
        assert_eq!(pixel(&composed, 0, 0), [255, 0, 0, 255]);
        assert_eq!(pixel(&composed, 39, 39), [255, 0, 0, 255]);

        // A scroll inside the margin repaints from the window and asks for nothing.
        screen.take(framed((-1005.0, -1005.0)), &mut drawing);
        assert!(screen.settled(), "inside the margin nothing is drawn");
        assert!(drawing.interval().is_none(), "and nothing was asked");

        // One past it asks for the part the window shows now.
        screen.take(framed((-1500.0, -1000.0)), &mut drawing);
        let Some(Content::Windowed(held)) = screen.slots.first().map(|slot| &slot.content) else {
            panic!("the page is still held as a window");
        };
        assert_eq!(
            held.asked.map(|window| (window.x, window.y)),
            Some((1490, 990)),
            "the window the view moved to"
        );
        assert!(
            screen.settled(),
            "the last window stands in while the next is drawn"
        );
        drain_windows(&mut screen, &mut drawing);
        let composed = screen.compose().expect("a window with an extent composes");
        assert_eq!(pixel(&composed, 20, 20), [255, 0, 0, 255]);
    }

    /// Waits for every window asked for to land.
    fn drain_windows(screen: &mut Screen, drawing: &mut Drawing<Draw>) {
        let began = std::time::Instant::now();
        while drawing.interval().is_some() {
            drawing.collect();
            for (finished, window) in drawing.windows() {
                screen.landed_window(finished, window);
            }
            assert!(
                began.elapsed() < std::time::Duration::from_mins(2),
                "the drawing thread never answered"
            );
            std::thread::sleep(viewer_host::drawing::POLL);
        }
        for (finished, window) in drawing.windows() {
            screen.landed_window(finished, window);
        }
    }

    /// A device screen draws a page past the bound on the device, as every marks page, and a
    /// frame the device refused falls back to the processor's windows and comes back to the
    /// device as a window's pixels at its part of the page (ADR 1944).
    #[test]
    fn a_refused_page_past_the_bound_falls_back_to_a_window() {
        let mut screen = Screen::for_device();
        let mut drawing = Drawing::new();
        screen.resize(40, 40);
        let (list, target) = past_the_bound();
        screen.take(
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                bytes: 0,
                origin: (-1000.0, -1000.0),
            }],
            &mut drawing,
        );
        assert_eq!(screen.device_pages(40, 40).len(), 1, "the device draws it");
        assert_eq!(screen.fall_back(&mut drawing), 1, "the device refused it");
        drain(&mut screen, &mut drawing);
        let pages = screen.device_pages(40, 40);
        let [(wrapper, placed)] = pages.as_slice() else {
            panic!("the window's pixels are the device's page");
        };
        assert_eq!(
            wrapper.page_size,
            Size::new(60.0, 60.0),
            "a window's pixels"
        );
        assert_eq!(
            placed.transform.apply(Point::new(0.0, 60.0)),
            Point::new(-10.0, -10.0),
            "placed at the window's corner, a quarter of the viewport up and to the left"
        );
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
                bytes: 0,
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
                    bytes: 0,
                    origin: (-5.0, -5.0),
                },
                Framed {
                    page: 1,
                    payload: Payload::Raster(a_raster(10, 10, [7, 7, 7, 255])),
                    bytes: 0,
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
                bytes: 0,
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
                bytes: 0,
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
            declined: Vec::new(),
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
                bytes: 0,
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
        // A window the page fits, so that it is drawn whole: at a smaller one the toolkit
        // windows' rule draws it as its window (ADR 2026).
        screen.resize(600, 800);
        let list = an_expensive_list(600.0, 800.0, 20_000);
        let target = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX)).expect("a target");
        screen.take(
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                bytes: 0,
                origin: (0.0, 0.0),
            }],
            &mut drawing,
        );
        // The next frame shows page 1 only; page 0's draw is answering nobody.
        screen.take(
            vec![Framed {
                page: 1,
                payload: Payload::Raster(a_raster(4, 4, [8, 8, 8, 255])),
                bytes: 0,
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

    /// A refusal is kept for the drawing that earned it and for nothing else: the same list at
    /// the same target keeps the sentence, and a new target asks again.
    ///
    /// The zoom is the case that matters. A target whose pixels exceed `viewer_core::MAX_PIXELS`
    /// is refused, so a magnification that overshoots is refused and the next one down is not —
    /// and a refusal keyed on the page rather than the drawing would keep the first one for good,
    /// whatever the reader zoomed to afterwards.
    #[test]
    fn a_refusal_is_kept_for_its_own_drawing_and_not_for_the_page() {
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let list = a_list(8.0, 8.0);
        let refused = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX)).expect("a target");
        let after_a_zoom = TargetSpec::for_page(&list, 2.0, u64::from(u32::MAX)).expect("a target");
        let framed = |target| {
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                bytes: 0,
                origin: (0.0, 0.0),
            }]
        };
        screen.take(framed(refused), &mut drawing);
        // The rasteriser's refusal, in the shape `landed` takes it.
        assert!(screen.landed(Finished {
            request: Draw {
                page: 0,
                list: Arc::clone(&list),
                target: refused,
            },
            outcome: Some(viewer_core::Rendered::Failed("too many pixels".to_owned())),
            declined: Vec::new(),
            cost: std::time::Duration::ZERO,
            waited: std::time::Duration::ZERO,
        }));
        assert_eq!(
            screen.refusals().collect::<Vec<_>>(),
            vec![(0, "too many pixels")],
            "the sentence is on the status"
        );
        // A scroll: the same drawing, moved. The refusal stands, and nothing is re-asked.
        screen.take(framed(refused), &mut drawing);
        assert!(screen.settled(), "a refused drawing is not asked again");
        assert_eq!(screen.refusals().count(), 1, "and it still says why");
        // A zoom: a target the rasteriser has never been asked about.
        screen.take(framed(after_a_zoom), &mut drawing);
        assert_eq!(
            screen.refusals().count(),
            0,
            "the refusal did not outlive the target it was about"
        );
        drain(&mut screen, &mut drawing);
        let window = screen.compose().expect("a window with an extent composes");
        assert_eq!(
            pixel(&window, 1, 1),
            [255, 0, 0, 255],
            "and the page drew at the new target"
        );
    }

    /// The same, on a device screen: a refused fallback goes back to the device at a new target
    /// rather than staying refused for ever (ADR 0725's arm inherited the defect).
    #[test]
    fn a_device_screens_refusal_is_kept_for_its_own_drawing_too() {
        let mut screen = Screen::for_device();
        let mut drawing = Drawing::new();
        screen.resize(30, 30);
        let list = a_list(8.0, 8.0);
        let refused = TargetSpec::for_page(&list, 1.0, u64::from(u32::MAX)).expect("a target");
        let after_a_zoom = TargetSpec::for_page(&list, 2.0, u64::from(u32::MAX)).expect("a target");
        let framed = |target| {
            vec![Framed {
                page: 0,
                payload: Payload::List {
                    list: Arc::clone(&list),
                    target,
                },
                bytes: 0,
                origin: (0.0, 0.0),
            }]
        };
        screen.take(framed(refused), &mut drawing);
        assert_eq!(screen.fall_back(&mut drawing), 1, "the device refused it");
        assert!(screen.landed(Finished {
            request: Draw {
                page: 0,
                list: Arc::clone(&list),
                target: refused,
            },
            outcome: Some(viewer_core::Rendered::Failed("too many pixels".to_owned())),
            declined: Vec::new(),
            cost: std::time::Duration::ZERO,
            waited: std::time::Duration::ZERO,
        }));
        screen.take(framed(refused), &mut drawing);
        assert_eq!(screen.refusals().count(), 1, "the same drawing, refused");
        assert!(
            screen.device_pages(30, 30).is_empty(),
            "and not offered to the device that refused it"
        );
        screen.take(framed(after_a_zoom), &mut drawing);
        assert_eq!(
            screen.refusals().count(),
            0,
            "a new target is a new question"
        );
        assert_eq!(
            screen.device_pages(30, 30).len(),
            1,
            "asked of the device first, as every marks page is"
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
                bytes: 0,
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
                bytes: 0,
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
                bytes: 0,
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

    /// A page of four marks with edges that fall between pixels: a diagonal, a slanted
    /// quadrilateral and two rectangles at fractional coordinates, so a window drawn of it and the
    /// page drawn whole could differ wherever a window's corner moved an edge's coverage.
    fn a_page_of_edges(side: f32) -> Arc<DisplayList> {
        // Each shape's corners in page units, and its colour.
        type Shape<'a> = (&'a [(f32, f32)], [f32; 3]);
        let mut list = DisplayList::new(Size {
            width: side,
            height: side,
        });
        let shapes: [Shape<'_>; 4] = [
            (&[(0.0, 0.0), (side, 0.0), (0.0, side)], [1.0, 0.0, 0.0]),
            (
                &[(10.3, 20.7), (60.1, 15.2), (75.9, 70.4), (30.6, 88.8)],
                [0.0, 0.5, 1.0],
            ),
            (
                &[
                    (41.25, 41.25),
                    (58.75, 41.25),
                    (58.75, 58.75),
                    (41.25, 58.75),
                ],
                [0.1, 0.8, 0.2],
            ),
            (
                &[(5.5, 90.1), (95.3, 90.1), (95.3, 92.6), (5.5, 92.6)],
                [0.0, 0.0, 0.0],
            ),
        ];
        for (points, [red, green, blue]) in shapes {
            let mut path = MarkPath::new();
            let mut corners = points.iter();
            if let Some(&(x, y)) = corners.next() {
                path.push(PathCommand::MoveTo(Point::new(x, y)));
            }
            for &(x, y) in corners {
                path.push(PathCommand::LineTo(Point::new(x, y)));
            }
            path.push(PathCommand::Close);
            list.push(Mark::Fill {
                path: Arc::new(path),
                transform: Transform::IDENTITY,
                paint: Paint::Solid(Color::rgb(red, green, blue)),
                fill_rule: FillRule::NonZero,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        Arc::new(list)
    }

    /// One list payload of page 0.
    fn one_page(list: &Arc<DisplayList>, target: TargetSpec, origin: (f32, f32)) -> Vec<Framed> {
        vec![Framed {
            page: 0,
            payload: Payload::List {
                list: Arc::clone(list),
                target,
            },
            bytes: 0,
            origin,
        }]
    }

    /// A page under the bound whose largest window is at most half of it is drawn on the processor
    /// as the window shows it, as the toolkit windows draw it (ADR 2026) — and the window it
    /// composes is, byte for byte, the window the page drawn whole composes at the same place.
    #[test]
    fn a_page_under_the_bound_the_toolkits_window_is_drawn_as_its_window_and_composes_alike() {
        let list = a_page_of_edges(100.0);
        let target = TargetSpec::for_page(&list, 2.13, u64::from(u32::MAX)).expect("a target");
        assert!(
            !super::past_the_bound(target),
            "the test is about a page under the bound"
        );
        let origin = (-37.4, -61.6);

        let mut windowed = Screen::new();
        let mut drawing = Drawing::new();
        windowed.resize(40, 40);
        windowed.take(one_page(&list, target, origin), &mut drawing);
        let Some(Content::Windowed(held)) = windowed.slots.first().map(|slot| &slot.content) else {
            panic!("the page is held as a window");
        };
        let asked = held
            .asked
            .expect("a window was asked of the drawing thread");
        assert!(
            u64::from(asked.width) * u64::from(asked.height)
                < u64::from(target.width) * u64::from(target.height),
            "the window is part of its page"
        );
        drain(&mut windowed, &mut drawing);

        // The control: the same drawing taken first where its window would be the whole page, so
        // it is drawn whole, then shown at the small window; the drawing keeps its answer.
        let mut whole = Screen::new();
        let mut drawing = Drawing::new();
        whole.resize(400, 400);
        whole.take(one_page(&list, target, origin), &mut drawing);
        drain(&mut whole, &mut drawing);
        whole.resize(40, 40);
        whole.take(one_page(&list, target, origin), &mut drawing);
        assert!(
            matches!(
                whole.slots.first().map(|slot| &slot.content),
                Some(Content::Pixels(..))
            ),
            "a drawing held whole keeps its answer: the rule is asked once for a drawing"
        );
        assert!(whole.settled(), "and nothing was asked again");

        let (by_window, by_page) = (
            windowed
                .compose()
                .expect("a window with an extent composes"),
            whole.compose().expect("a window with an extent composes"),
        );
        assert!(
            by_window.data == by_page.data,
            "the window composes what the page drawn whole composes"
        );
        assert_ne!(
            pixel(&by_window, 20, 20),
            surround_pixel(),
            "and the page is on it"
        );
    }

    /// A window with no extent shows no window, so a page under the bound is drawn whole there.
    #[test]
    fn a_page_under_the_bound_behind_no_window_is_drawn_whole() {
        let list = a_page_of_edges(100.0);
        let target = TargetSpec::for_page(&list, 2.13, u64::from(u32::MAX)).expect("a target");
        let mut screen = Screen::new();
        let mut drawing = Drawing::new();
        screen.take(one_page(&list, target, (0.0, 0.0)), &mut drawing);
        assert!(matches!(
            screen.slots.first().map(|slot| &slot.content),
            Some(Content::Drawing(..))
        ));
    }

    /// A device screen's page under the bound whose frame the device refused falls back to the
    /// processor's windows where the toolkit windows' rule holds (ADR 2026), and to a whole page
    /// where it does not.
    #[test]
    fn a_refused_page_under_the_bound_falls_back_to_a_window_where_the_rule_holds() {
        let list = a_page_of_edges(100.0);
        let target = TargetSpec::for_page(&list, 2.13, u64::from(u32::MAX)).expect("a target");
        for (extent, as_windows) in [((40, 40), true), ((400, 400), false)] {
            let mut screen = Screen::for_device();
            let mut drawing = Drawing::new();
            screen.resize(extent.0, extent.1);
            screen.take(one_page(&list, target, (-37.4, -61.6)), &mut drawing);
            assert_eq!(screen.fall_back(&mut drawing), 1, "the device refused it");
            assert_eq!(
                matches!(
                    screen.slots.first().map(|slot| &slot.content),
                    Some(Content::Windowed(..))
                ),
                as_windows,
                "at {extent:?}"
            );
            drain(&mut screen, &mut drawing);
            assert_eq!(screen.device_pages(extent.0, extent.1).len(), 1, "it lands");
        }
    }
}
