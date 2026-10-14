//! A page zoomed past its window, drawn as the window shows it.
//!
//! A toolkit window holds a page as one raster of the whole page, copied into the toolkit's own
//! image at every update. At the fit that is the page on the screen and nothing else; zoomed in,
//! it is the page and every row of it nobody can see, drawn at every notch and copied at every
//! update — 65 to 250 ms a pump for a page of half a gigabyte to one, and past
//! [`viewer_core::MAX_PIXELS`] an allocation the viewer refuses. `quorra` never met either: it
//! draws a window-sized frame from the marks that reach it (ADR 1841). This module is the same
//! answer for the two toolkit windows (ADRs 1892, 1930).
//!
//! **Which pages: those whose window is at most half of them.** [`oversized`] holds a page drawn
//! this way where [`largest_window`] — the viewport and a quarter of it on every side, inside the
//! page — is at most half the page's area, or where its whole raster would be past
//! [`viewer_core::MAX_PIXELS`]. A window is drawn from a list of its own, of the marks that reach
//! it (ADR 1841), and measured on the witness one that is most of the page lands later than the
//! page drawn whole while one under half of it lands sooner, by more the deeper the zoom (ADR
//! 1930 §3); a page drawn whole is also drawn once for every scroll, where a window is drawn again
//! at each scroll past its margin. So every page at the fit and the first notches past it is
//! asked for, drawn and answered exactly as before, and no frame of it moves.
//!
//! The window tells the viewer it draws a page from its list (`viewer_core::Command::Oversize`),
//! so a page past the bound is asked for at its whole target rather than refused; [`Oversized`]
//! holds every request [`oversized`] names, the drawing draws the part of the target the viewport
//! shows with a margin round it ([`crate::Drawing::ask_window`]), and the viewer is told the host
//! took the list (`viewer_core::Rendered::Listed`) once the first window of it has landed. What
//! is allocated and copied is the window, and never the page. A scroll that leaves what was drawn
//! asks for the window again.
//!
//! Until a window lands the page stands in as any page does: the viewer keeps its last frame of
//! the page while the request is outstanding (ADR 1843), the retained picture shows under the
//! window once it has (ADR 1866), and the last window drawn of the page stands in, scaled, for the
//! next one at another magnification — and for the page drawn whole, after a notch back to a size
//! drawn whole, until that frame lands (ADR 1930).

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

/// Whether a request is for a page drawn as the window shows it — the pages this module holds,
/// and the only ones: the largest window a viewport `viewport` device pixels across draws of it is
/// at most half its target's area, or its whole raster is past [`viewer_core::MAX_PIXELS`]
/// (ADRs 1892, 1930).
///
/// **Half, because a window that is most of the page costs more than the page.** The window is
/// drawn from the marks that reach it, which are found for every window drawn, and on the
/// witness a window of nine tenths of the page landed 60–130 ms after the page drawn whole and one
/// of two fifths 180–220 ms before it (ADR 1930 §3). It is the same half for the same reason as
/// the band's (`drawing::band_of`, ADR 1868): the fixed part of a draw is about half of it.
#[must_use]
pub fn oversized(request: &RenderRequest, viewport: (u32, u32)) -> bool {
    let target = request.target;
    let (width, height) = largest_window(viewport);
    let area = |width: u32, height: u32| u64::from(width).saturating_mul(u64::from(height));
    let page = area(target.width, target.height);
    let window = area(width.min(target.width), height.min(target.height));
    window.saturating_mul(2) <= page || page > viewer_core::MAX_PIXELS
}

/// The largest window a viewport `viewport` device pixels across is drawn as: the viewport and a
/// quarter of it on every side, [`with_margin`]'s growth of a page that runs past the viewport on
/// every side.
#[must_use]
pub fn largest_window(viewport: (u32, u32)) -> (u32, u32) {
    let grown = |length: u32| length.saturating_add((length / 4).saturating_mul(2));
    (grown(viewport.0), grown(viewport.1))
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

/// One page drawn as the window shows it: the viewer's request for it, and the window drawn of it.
#[derive(Debug)]
struct Held {
    request: RenderRequest,
    /// Whether the request is for a page drawn as the window shows it. `false` once the viewer
    /// has asked for the page at a size drawn whole: the last window drawn then only stands in,
    /// scaled, until that frame lands ([`Oversized::answered_whole`]), and nothing is asked.
    windowed: bool,
    /// Whether the viewer has been told the host took this request's list.
    answered: bool,
    /// The window handed to the drawing and not yet landed.
    asked: Option<Window>,
    /// A window taken back before it landed — a person's Escape, or the page leaving the
    /// arrangement — which is not asked again while the view stays inside it: a page drawn whole
    /// is drawn again when the view changes and not before, and so is a window.
    declined: Option<Window>,
    /// The last window drawn of this page — this request's, or one before it, which stands in
    /// until this one's lands.
    drawn: Option<Drawn>,
}

/// A window that landed, and what it is a window of.
#[derive(Debug)]
struct Drawn {
    window: Window,
    raster: Raster,
    /// The target it is a window of, which places it on a page drawn at another magnification.
    of: TargetSpec,
    /// The request it was drawn for. **Only that request's window holds what the view shows**:
    /// one drawn for the request before it at the same target is the page under another ink — a
    /// field typed into, an appearance under the pointer — and stands in until the new ink's
    /// window lands, as a whole page's last frame does (ADR 1843).
    token: RenderToken,
}

/// The pages a window holds drawn as it shows them, and what it has drawn of each (ADRs 1892,
/// 1930).
#[derive(Debug, Default)]
pub struct Oversized {
    held: Vec<Held>,
}

impl Oversized {
    /// The viewer asked for a page, in a viewport `viewport` device pixels across: one
    /// [`oversized`] is held and answers `true` — the window then draws it through [`Self::due`] —
    /// and any other request for that page answers `false` and is drawn whole, with the last
    /// window drawn of the page standing in until that frame lands.
    pub fn asked(&mut self, request: &RenderRequest, viewport: (u32, u32)) -> bool {
        let same = |held: &Held| {
            held.request.document == request.document && held.request.page == request.page
        };
        let windowed = oversized(request, viewport);
        // The last window drawn of the page stays, as a stand-in, until the new request's frame
        // lands: at the same target it is the same place under the old ink, and at another it is
        // the page at the old magnification, scaled to the new one as a toolkit stands in (ADR
        // 1843).
        let drawn = self
            .held
            .iter()
            .position(same)
            .and_then(|at| self.held.remove(at).drawn);
        self.held.retain(|held| !same(held));
        if windowed || drawn.is_some() {
            self.held.insert(
                0,
                Held {
                    request: request.clone(),
                    windowed,
                    answered: false,
                    asked: None,
                    declined: None,
                    drawn,
                },
            );
            self.held.truncate(HELD);
        }
        windowed
    }

    /// The page's whole frame for `request` finished — drawn, or refused, either of which is the
    /// page's answer now: the window standing in for it is let go.
    pub fn answered_whole(&mut self, request: &RenderRequest) {
        self.held
            .retain(|held| held.windowed || held.request.token != request.token);
    }

    /// The windows to draw now: for each page held of `focused` that the arrangement places at its
    /// target's size, the part the viewport shows, with its margin, where neither the window drawn
    /// for the request nor the one asked for holds it.
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
            .filter(|held| held.windowed && held.request.document == focused)
        {
            let target = held.request.target;
            let Some(shown) =
                geometry(held.request.page).and_then(|placed| shown(target, placed, viewport))
            else {
                continue;
            };
            let covered = |window: Option<Window>| window.is_some_and(|window| window.holds(shown));
            let token = held.request.token;
            let drawn = held
                .drawn
                .as_ref()
                .filter(|drawn| drawn.token == token)
                .map(|drawn| drawn.window);
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

    /// Whether a window drawn for the request held is on the screen for a page of `focused` — a
    /// frame, for a window deciding whether it has shown its first.
    #[must_use]
    pub fn shows(&self, focused: DocumentId) -> bool {
        self.held.iter().any(|held| {
            held.windowed
                && held.request.document == focused
                && held
                    .drawn
                    .as_ref()
                    .is_some_and(|drawn| drawn.token == held.request.token)
        })
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
            .find(|held| held.windowed && held.request.token == finished.request.token)?;
        if held.asked == Some(window) {
            held.asked = None;
        }
        let answer = match finished.outcome? {
            viewer_core::Rendered::Raster(raster) => {
                held.drawn = Some(Drawn {
                    window,
                    raster,
                    of: held.request.target,
                    token: held.request.token,
                });
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
    /// arrangement places the page at its target's size. A window standing in for a page asked
    /// for whole is placed the same way, scaled, over the retained picture: the viewer holds no
    /// frame of a page it was answered `Listed` for.
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
            if let Some(drawn) = held.drawn.as_ref()
                && let Some(shown) = at(held.request.page, target, drawn.of, drawn.window)
            {
                placed.push(window_placed(held.request.page, &drawn.raster, shown));
            }
            if let Some(partial) = partial.filter(|partial| {
                held.windowed && self.holds_partial(partial) && partial.page() == held.request.page
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
