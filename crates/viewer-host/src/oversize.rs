//! A page whose whole raster would be past `viewer_core::MAX_PIXELS`, drawn as the window shows it.
//!
//! A toolkit window holds a page as one raster of the whole page, and the viewer refuses a page
//! past [`viewer_core::MAX_PIXELS`] because that raster is an allocation. Zoomed far enough into a
//! large page — an A1 drawing at a few times its fit on a large screen — every notch was refused,
//! and the window showed the last frame it had, enlarged and blurred, with the refusal said again
//! at every command that settled. `quorra` never met the bound: it draws a window-sized frame from
//! the marks that reach it (ADR 1841). This module is the same answer for the two toolkit windows
//! (ADR 1892).
//!
//! **The bound is kept and moved to where the allocation now is.** The window tells the viewer it
//! draws such a page from its list (`viewer_core::Command::Oversize`), so the request is made at
//! the page's whole target; [`Oversized`] holds it, the drawing draws the part of the target the
//! viewport shows with a margin round it ([`crate::Drawing::ask_window`]), and the viewer is told
//! the host took the list (`viewer_core::Rendered::Listed`) once the first window of it has
//! landed. What is allocated is the window, a few times the viewport, and never the page. A scroll
//! that leaves what was drawn asks for the window again. **Every page under the bound is asked for
//! and drawn exactly as before**, so no frame a gate digests moves.
//!
//! Until a window lands the page stands in as any page does: the viewer keeps its last frame of
//! the page while the request is outstanding (ADR 1843), and the retained picture shows under the
//! window once it has (ADR 1866).

use pdf_render::{Raster, TargetSpec, Transform};
use viewer_core::{DocumentId, FrameView, PageGeometry, RenderRequest, RenderToken};

use crate::drawing::{Finished, Partial};
use crate::stand_in::Placed;

/// A rectangle of a page's target, in its pixels: what a window of it is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// The first column.
    pub x: u32,
    /// The first row.
    pub y: u32,
    /// How many columns.
    pub width: u32,
    /// How many rows.
    pub height: u32,
}

impl Window {
    /// This part of `target`, as a target of its own: the page's transform with the window's
    /// corner moved to the origin — `drawing::band_target`'s construction in two directions.
    #[must_use]
    pub fn target_of(self, target: TargetSpec) -> TargetSpec {
        #[expect(
            clippy::cast_precision_loss,
            reason = "`rasterize` refuses a target beyond MAX_EXTENT = 2^24, which f32 holds exactly"
        )]
        let (x, y) = (self.x as f32, self.y as f32);
        TargetSpec {
            width: self.width,
            height: self.height,
            transform: target.transform.then(Transform::translate(-x, -y)),
        }
    }

    /// Whether `other` lies inside this.
    fn holds(self, other: Self) -> bool {
        let end = |first: u32, length: u32| u64::from(first).saturating_add(u64::from(length));
        other.x >= self.x
            && other.y >= self.y
            && end(other.x, other.width) <= end(self.x, self.width)
            && end(other.y, other.height) <= end(self.y, self.height)
    }
}

/// Whether a request's whole raster would be past [`viewer_core::MAX_PIXELS`] — the pages this
/// module draws as the window shows them, and the only ones.
#[must_use]
pub fn oversized(request: &RenderRequest) -> bool {
    u64::from(request.target.width).saturating_mul(u64::from(request.target.height))
        > viewer_core::MAX_PIXELS
}

/// The part of `target` a viewport `viewport` device pixels across shows, where the arrangement
/// places the page at `geometry` — `None` where it places the page at another size, so the request
/// is not for the page the window shows, or where none of it is on the screen.
#[must_use]
pub fn shown(target: TargetSpec, geometry: PageGeometry, viewport: (u32, u32)) -> Option<Window> {
    if (geometry.width, geometry.height) != (target.width, target.height) {
        return None;
    }
    let (x, width) = span(geometry.origin.0, target.width, viewport.0)?;
    let (y, height) = span(geometry.origin.1, target.height, viewport.1)?;
    Some(Window {
        x,
        y,
        width,
        height,
    })
}

/// The pixels of a line `extent` long, placed at `origin` in a viewport `viewport` long, that the
/// viewport shows: the first and last any part of which is inside it, so no sliver of the window is
/// left undrawn.
fn span(origin: f32, extent: u32, viewport: u32) -> Option<(u32, u32)> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "the clamp to the page's pixels comes before the cast, and a page is under \
                  MAX_EXTENT = 2^24 pixels, which f32 holds exactly"
    )]
    let pixel = |at: f32| at.clamp(0.0, extent as f32) as u32;
    let first = pixel((-origin).floor());
    #[expect(
        clippy::cast_precision_loss,
        reason = "a viewport is a few thousand pixels"
    )]
    let end = pixel((viewport as f32 - origin).ceil());
    (first < end).then(|| (first, end.saturating_sub(first)))
}

/// `shown` grown by a quarter of the viewport on every side, inside the page.
///
/// **A quarter, because it is paid on every draw and bought back on every small scroll.** The
/// window is drawn once and a scroll inside the margin repaints from it with nothing drawn; the
/// margin costs its area on each draw, and a quarter all round is two and a quarter times the
/// viewport — a wheel notch's scroll, several times over, for about twice the marks.
fn with_margin(shown: Window, target: TargetSpec, viewport: (u32, u32)) -> Window {
    let grow = |first: u32, length: u32, extent: u32, by: u32| {
        let start = first.saturating_sub(by);
        let end = first.saturating_add(length).saturating_add(by).min(extent);
        (start, end.saturating_sub(start))
    };
    let (x, width) = grow(shown.x, shown.width, target.width, viewport.0 / 4);
    let (y, height) = grow(shown.y, shown.height, target.height, viewport.1 / 4);
    Window {
        x,
        y,
        width,
        height,
    }
}

/// Most pages held at once: Table 29's arrangements show a few pages, and a page that leaves the
/// screen is asked for again when it comes back.
const HELD: usize = 4;

/// One page past the bound: the viewer's request for it, and the window drawn of it.
#[derive(Debug)]
struct Held {
    request: RenderRequest,
    /// Whether the viewer has been told the host took this request's list.
    answered: bool,
    /// The window handed to the drawing and not yet landed.
    asked: Option<Window>,
    /// A window taken back before it landed — a person's Escape, or the page leaving the
    /// arrangement — which is not asked again while the view stays inside it: a page drawn whole
    /// is drawn again when the view changes and not before, and so is a window.
    declined: Option<Window>,
    /// The last window drawn of this page, its pixels, and the target it is a window of — this
    /// request's, or the one before it, which stands in scaled until this one's lands.
    drawn: Option<(Window, Raster, TargetSpec)>,
}

/// The pages a window holds past the whole-page bound, and what it has drawn of each (ADR 1892).
#[derive(Debug, Default)]
pub struct Oversized {
    held: Vec<Held>,
}

impl Oversized {
    /// The viewer asked for a page: an oversized one is held and answers `true` — the window then
    /// draws it through [`Self::due`] — and any other request for that page lets go of what was
    /// held for it, because the viewer's own frame is the page now.
    pub fn asked(&mut self, request: &RenderRequest) -> bool {
        let same = |held: &Held| {
            held.request.document == request.document && held.request.page == request.page
        };
        if !oversized(request) {
            self.held.retain(|held| !same(held));
            return false;
        }
        // The last window drawn of the page stays, as a stand-in, until the new request's lands:
        // at the same target it is the same place under the old ink, and at another it is the
        // page at the old magnification, scaled to the new one as a toolkit stands in (ADR 1843).
        let drawn = self
            .held
            .iter()
            .position(same)
            .and_then(|at| self.held.remove(at).drawn);
        self.held.retain(|held| !same(held));
        self.held.insert(
            0,
            Held {
                request: request.clone(),
                answered: false,
                asked: None,
                declined: None,
                drawn,
            },
        );
        self.held.truncate(HELD);
        true
    }

    /// The windows to draw now: for each page held of `focused` that the arrangement places at its
    /// target's size, the part the viewport shows, with its margin, where neither the window drawn
    /// nor the one asked for holds it.
    pub fn due(
        &mut self,
        focused: DocumentId,
        mut geometry: impl FnMut(usize) -> Option<PageGeometry>,
        viewport: (u32, u32),
    ) -> Vec<(RenderRequest, Window)> {
        let mut due = Vec::new();
        for held in self
            .held
            .iter_mut()
            .filter(|held| held.request.document == focused)
        {
            let target = held.request.target;
            let Some(shown) =
                geometry(held.request.page).and_then(|placed| shown(target, placed, viewport))
            else {
                continue;
            };
            let covered = |window: Option<Window>| window.is_some_and(|window| window.holds(shown));
            let drawn = held
                .drawn
                .as_ref()
                .filter(|(_, _, of)| *of == target)
                .map(|(window, _, _)| *window);
            if covered(held.asked) || covered(drawn) || covered(held.declined) {
                continue;
            }
            held.declined = None;
            let window = with_margin(shown, target, viewport);
            held.asked = Some(window);
            due.push((held.request.clone(), window));
        }
        due
    }

    /// A window the drawing finished: kept where it is of the request still held, and the token
    /// to answer `viewer_core::Rendered::Listed` for where the viewer has not been told yet. A
    /// window that failed answers the failure, which the viewer records as this page's answer (trap
    /// 20 is about an abandoned draw, which answers nothing and is asked again).
    pub fn landed(
        &mut self,
        finished: Finished<RenderRequest>,
        window: Window,
    ) -> Option<(RenderToken, viewer_core::Rendered)> {
        let held = self
            .held
            .iter_mut()
            .find(|held| held.request.token == finished.request.token)?;
        if held.asked == Some(window) {
            held.asked = None;
        }
        let answer = match finished.outcome? {
            viewer_core::Rendered::Raster(raster) => {
                held.drawn = Some((window, raster, held.request.target));
                viewer_core::Rendered::Listed
            }
            other => other,
        };
        (!std::mem::replace(&mut held.answered, true)).then_some((finished.request.token, answer))
    }

    /// A window the drawing took back. One a newer window replaced is nothing to remember; one
    /// that was still the window asked for is declined, and asked again by the first
    /// [`Self::due`] whose view leaves it.
    pub fn abandoned(&mut self, request: &RenderRequest, window: Window) {
        if let Some(held) = self
            .held
            .iter_mut()
            .find(|held| held.request.token == request.token && held.asked == Some(window))
        {
            held.asked = None;
            held.declined = Some(window);
        }
    }

    /// Whether the drawing's `partial` is a window of a page held here, which [`Self::place`]
    /// shows where it belongs.
    fn holds_partial(&self, partial: &Partial) -> bool {
        partial.window().is_some()
            && self.held.iter().any(|held| {
                held.request.page == partial.page()
                    && partial.document() == Some(held.request.document)
            })
    }

    /// `placed` with the windows drawn of `focused`'s held pages put over it, each at the part of
    /// the page it is of — and the window being drawn over those, as it stands — where the
    /// arrangement places the page at its target's size.
    #[must_use]
    pub fn place<'a>(
        &'a self,
        mut placed: Vec<Placed<'a>>,
        partial: Option<&'a Partial>,
        focused: DocumentId,
        mut geometry: impl FnMut(usize) -> Option<PageGeometry>,
    ) -> Vec<Placed<'a>> {
        // Where a window of `of` goes and how big it is shown, on a page the arrangement places at
        // `target`'s size: at its own size where it is a window of that target, and scaled by
        // the ratio of the two where it is a window of the page at another magnification.
        let mut at = |page: usize, target: TargetSpec, of: TargetSpec, window: Window| {
            let placed = geometry(page)
                .filter(|placed| (placed.width, placed.height) == (target.width, target.height))?;
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a page is under MAX_EXTENT = 2^24 pixels, which f32 holds exactly, and \
                          a window's scaled extent is a positive few thousand"
            )]
            let shown = {
                let ratio = target.width as f32 / of.width.max(1) as f32;
                let origin = (
                    placed.origin.0 + window.x as f32 * ratio,
                    placed.origin.1 + window.y as f32 * ratio,
                );
                let extent = (
                    (window.width as f32 * ratio).ceil() as u32,
                    (window.height as f32 * ratio).ceil() as u32,
                );
                (origin, extent)
            };
            Some(shown)
        };
        for held in self
            .held
            .iter()
            .filter(|held| held.request.document == focused)
        {
            let target = held.request.target;
            if let Some((window, raster, of)) = held.drawn.as_ref()
                && let Some(shown) = at(held.request.page, target, *of, *window)
            {
                placed.push(window_placed(held.request.page, raster, shown));
            }
            if let Some(partial) = partial.filter(|partial| {
                self.holds_partial(partial) && partial.page() == held.request.page
            }) && let Some(window) = partial.window()
                && let Some(shown) = at(held.request.page, target, target, window)
            {
                placed.push(window_placed(held.request.page, partial.raster(), shown));
            }
        }
        // Stable, so a page's window stays over the stand-in placed for it before.
        placed.sort_by_key(|shown| shown.frame.page);
        placed
    }
}

/// A window's pixels, shown where they belong at the extent given — their own size, or a window
/// of the page at another magnification standing in, scaled.
fn window_placed(
    page: usize,
    raster: &Raster,
    (origin, extent): ((f32, f32), (u32, u32)),
) -> Placed<'_> {
    Placed {
        frame: FrameView {
            page,
            raster,
            origin,
        },
        extent,
        retained: false,
    }
}
