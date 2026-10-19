//! A page whose marks cannot cross, drawn in this process as the window shows it (ADRs 1986,
//! 2016).
//!
//! A page crosses as its marks where its list is no larger than its raster, and two kinds of page
//! cannot: one the list format refuses ([`Uncodable`](super::display_list::Uncodable) — a producer
//! deferred to the device's scale, §11.7.5.2's transfer channel, a nest past what a backend
//! composites) and one whose list is larger than its pixels. Such a page crosses as a raster, and
//! two of them cross as a window instead: the part of the page the viewport shows and a quarter of
//! the viewport round it, drawn here from the list this process already holds.
//!
//! **Which pages: those the toolkit windows draw as a window.** A page past
//! `viewer_core::MAX_PIXELS`, whose raster is the allocation the bound exists to refuse (ADR 1986);
//! and a page under it whose largest window, centred, is part of it and either at most half its
//! area or keeps at most seven eighths of its commands ([`worth_a_window`]) — ADR 1994's rule, which
//! the toolkit windows apply to every page they draw, and which this process can apply because the
//! list is here and the host holds none of a page that crosses as pixels (ADR 2016).
//!
//! **The window is this process's statement and not a request the host makes.** The frame says
//! which part of the target it is, and the host places it there, so the host and this module need
//! not agree on the margin for the page to land where it belongs. The rule is
//! `viewer_host::oversize`'s all the same — the same quarter, for the reason ADR 1930 gives, and the
//! same bound — and it is written again here because this process links no host crate:
//! `viewer_host` brings the host's network client with it (ADR 1871). The tests below hold the two
//! statements equal.

use std::sync::Arc;

use pdf_render::{DisplayList, Raster, TargetSpec, Transform};
use viewer_core::{DocumentId, RenderToken};

/// A rectangle of a page's target, in its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Window {
    /// The first column.
    pub(crate) x: u32,
    /// The first row.
    pub(crate) y: u32,
    /// How many columns.
    pub(crate) width: u32,
    /// How many rows.
    pub(crate) height: u32,
}

impl Window {
    /// The part of `target` a viewport `viewport` device pixels across shows, where the page's
    /// top-left corner sits at `origin`; `None` where none of it is on the screen.
    fn shown(target: TargetSpec, origin: (f32, f32), viewport: (u32, u32)) -> Option<Self> {
        let (x, width) = span(origin.0, target.width, viewport.0)?;
        let (y, height) = span(origin.1, target.height, viewport.1)?;
        Some(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// This window grown by a quarter of the viewport on every side, inside the page.
    fn with_margin(self, target: TargetSpec, viewport: (u32, u32)) -> Self {
        let grow = |first: u32, length: u32, extent: u32, by: u32| {
            let start = first.saturating_sub(by);
            let end = first.saturating_add(length).saturating_add(by).min(extent);
            (start, end.saturating_sub(start))
        };
        let (x, width) = grow(self.x, self.width, target.width, viewport.0 / 4);
        let (y, height) = grow(self.y, self.height, target.height, viewport.1 / 4);
        Self {
            x,
            y,
            width,
            height,
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

    /// The largest window a viewport `viewport` device pixels across is drawn as, centred on
    /// `target` and inside it: the viewport and a quarter of it on every side, which
    /// [`Self::with_margin`] grows a part shown to where the page runs past the viewport all round.
    fn largest_centred(target: TargetSpec, viewport: (u32, u32)) -> Self {
        let grown = |length: u32| length.saturating_add((length / 4).saturating_mul(2));
        let (across, down) = (
            grown(viewport.0).min(target.width),
            grown(viewport.1).min(target.height),
        );
        Self {
            x: target.width.saturating_sub(across) / 2,
            y: target.height.saturating_sub(down) / 2,
            width: across,
            height: down,
        }
    }

    /// This part of `target` as a target of its own: the page's transform with the window's
    /// corner moved to the origin.
    pub(crate) fn target_of(self, target: TargetSpec) -> TargetSpec {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a target is under pdf_render::MAX_EXTENT = 2^24, which f32 holds exactly"
        )]
        let (x, y) = (self.x as f32, self.y as f32);
        TargetSpec {
            width: self.width,
            height: self.height,
            transform: target.transform.then(Transform::translate(-x, -y)),
        }
    }
}

/// The pixels of a line `extent` long, placed at `origin` in a viewport `viewport` long, that the
/// viewport shows: the first and last any part of which is inside it.
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

/// The most of a page's commands a window may keep and be drawn in the page's place: seven
/// eighths, `viewer_host::oversize`'s bound, measured where a window and its page land together
/// (ADR 1994).
const KEPT: f64 = 0.875;

/// Whether a page under the bound that crosses as pixels is drawn here as the window shows it,
/// at a viewport `viewport` device pixels across: its largest window, centred, is part of it and
/// either at most half its target's area or keeps at most [`KEPT`] of its commands —
/// `viewer_host::oversize::oversized`'s rule for a page under the bound (ADRs 1994, 2016).
///
/// **Commands, because a window costs what it keeps, and half the pixels, because a page of few
/// commands is drawn per pixel**: ADR 1994's two measurements, made on the processor this process
/// draws with. The share is [`DisplayList::share_reaching`]'s stride of the list, asked on this
/// thread; the area is asked first, so a window of half its page or less asks the list nothing.
pub(crate) fn worth_a_window(list: &DisplayList, target: TargetSpec, viewport: (u32, u32)) -> bool {
    let area = |width: u32, height: u32| u64::from(width).saturating_mul(u64::from(height));
    let page = area(target.width, target.height);
    let window = Window::largest_centred(target, viewport);
    let kept = area(window.width, window.height);
    kept < page
        && (kept.saturating_mul(2) <= page || list.share_reaching(window.target_of(target)) <= KEPT)
}

/// One page drawn here as the window shows it.
#[derive(Debug)]
pub(crate) struct Windowed {
    /// Which request it answers, for `Marks::record`'s rule.
    pub(crate) token: RenderToken,
    /// Which document, for a refusal said about it.
    pub(crate) document: DocumentId,
    /// The marks the windows are drawn from.
    pub(crate) list: Arc<DisplayList>,
    /// The whole target a window is a part of.
    pub(crate) target: TargetSpec,
    /// Where the viewer places the page in the viewport, kept current as `Crossed::origin` is.
    pub(crate) origin: (f32, f32),
    /// The window last drawn and its pixels; `None` until the viewport shows some of the page.
    pub(crate) drawn: Option<(Window, Raster)>,
}

impl Windowed {
    /// The window to draw where the viewport `viewport` now shows a part of the page the last
    /// window does not hold: the part shown and its margin. `None` where nothing new is shown.
    pub(crate) fn owed(&self, viewport: (u32, u32)) -> Option<Window> {
        let shown = Window::shown(self.target, self.origin, viewport)?;
        if self
            .drawn
            .as_ref()
            .is_some_and(|(window, _)| window.holds(shown))
        {
            return None;
        }
        Some(shown.with_margin(self.target, viewport))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Window, span, worth_a_window};
    use pdf_render::{
        BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Size,
        TargetSpec, Transform,
    };
    use viewer_host::oversize;

    fn target(width: u32, height: u32) -> TargetSpec {
        TargetSpec {
            width,
            height,
            transform: Transform::IDENTITY,
        }
    }

    #[test]
    fn a_window_is_the_part_shown_and_a_quarter_of_the_viewport_round_it_inside_the_page() {
        // The page's corner 1 000 px up and left of the viewport's: the viewport shows columns
        // 1 000 to 1 799 and rows 1 000 to 1 999, and a quarter is 200 across and 250 down.
        let page = target(40_000, 50_000);
        let shown = Window::shown(page, (-1000.0, -1000.0), (800, 1000)).expect("on the screen");
        assert_eq!(
            (shown.x, shown.y, shown.width, shown.height),
            (1000, 1000, 800, 1000)
        );
        let grown = shown.with_margin(page, (800, 1000));
        assert_eq!(
            (grown.x, grown.y, grown.width, grown.height),
            (800, 750, 1200, 1500)
        );
        // At the page's corner the margin stops at the page.
        let corner = Window::shown(page, (0.0, 0.0), (800, 1000)).expect("on the screen");
        let grown = corner.with_margin(page, (800, 1000));
        assert_eq!(
            (grown.x, grown.y, grown.width, grown.height),
            (0, 0, 1000, 1250)
        );
    }

    /// A page 1 000 units across and 100 down, holding a unit square every ten units along its
    /// middle row, so the share of its commands a centred window keeps is the share of its width.
    fn a_row_of_squares() -> DisplayList {
        let mut list = DisplayList::new(Size::new(1000.0, 100.0));
        for at in 0..100_u16 {
            let x = f32::from(at) * 10.0 + 4.5;
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(x, 50.0)));
            path.push(PathCommand::LineTo(Point::new(x + 1.0, 50.0)));
            path.push(PathCommand::LineTo(Point::new(x + 1.0, 51.0)));
            path.push(PathCommand::LineTo(Point::new(x, 51.0)));
            path.push(PathCommand::Close);
            list.push(Command::Fill {
                path: Arc::new(path),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(Color::BLACK),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        list
    }

    /// **The window's shape is stated twice, and the two statements are one rule** (ADRs 1986,
    /// 2016): the part shown, its margin and the largest window, at every corner and viewport
    /// asked, are `viewer_host::oversize`'s to the pixel.
    #[test]
    fn the_window_this_process_draws_is_the_one_the_toolkit_windows_draw() {
        let tuple = |w: oversize::Window| (w.x, w.y, w.width, w.height);
        let ours = |w: Window| (w.x, w.y, w.width, w.height);
        for (width, height) in [(612, 792), (5_000, 3_000), (40_000, 50_000), (1, 1)] {
            let page = target(width, height);
            for viewport in [(800, 1000), (1200, 900), (1, 1), (3, 7)] {
                assert_eq!(
                    ours(Window::largest_centred(page, viewport)),
                    tuple(oversize::largest_centred(page, viewport)),
                    "the largest window of {width} x {height} at {viewport:?}"
                );
                for origin in [(0.0, 0.0), (-1000.5, -250.25), (300.0, -10.0), (-0.5, 0.75)] {
                    let shown = Window::shown(page, origin, viewport);
                    let theirs = oversize::shown_at(page, origin, viewport);
                    assert_eq!(shown.map(ours), theirs.map(tuple), "shown at {origin:?}");
                    if let (Some(shown), Some(theirs)) = (shown, theirs) {
                        assert_eq!(
                            ours(shown.with_margin(page, viewport)),
                            tuple(oversize::with_margin(theirs, page, viewport)),
                            "the margin at {origin:?}"
                        );
                    }
                }
            }
        }
    }

    /// **And the bound is one rule too**: on a page whose centred window keeps the share of its
    /// commands that it keeps of the page's width, this process windows exactly the magnifications
    /// the toolkit windows do — among them one whose window is more than half the page and is
    /// windowed for the commands it leaves out, and one past seven eighths that is drawn whole
    /// (ADRs 1994, 2016).
    #[test]
    fn this_process_draws_as_a_window_the_pages_the_toolkit_windows_do() {
        let list = a_row_of_squares();
        let viewport = (400, 300);
        let (mut by_commands, mut whole) = (false, false);
        for hundredth in 40..=400_u16 {
            let scale = f32::from(hundredth) / 100.0;
            let page = TargetSpec::for_page(&list, scale, u64::MAX).expect("a target");
            let ours = worth_a_window(&list, page, viewport);
            assert_eq!(
                ours,
                oversize::worth_a_window(&list, page, viewport),
                "at {scale} times the page's size"
            );
            let window = Window::largest_centred(page, viewport);
            let area = |width: u32, height: u32| u64::from(width) * u64::from(height);
            let more_than_half =
                area(window.width, window.height) * 2 > area(page.width, page.height);
            by_commands |= ours && more_than_half;
            whole |= !ours && area(window.width, window.height) < area(page.width, page.height);
        }
        assert!(
            by_commands,
            "no magnification was windowed for its commands alone"
        );
        assert!(whole, "no window smaller than its page was drawn whole");
    }

    #[test]
    fn a_page_off_the_screen_shows_no_window() {
        assert_eq!(span(900.0, 100, 800), None);
        assert_eq!(span(-200.0, 100, 800), None);
        assert!(Window::shown(target(100, 100), (0.0, 2000.0), (800, 1000)).is_none());
    }

    #[test]
    fn a_window_holds_what_lies_inside_it_and_not_what_crosses_its_edge() {
        let outer = Window {
            x: 800,
            y: 750,
            width: 1200,
            height: 1500,
        };
        let inside = Window {
            x: 900,
            y: 800,
            width: 800,
            height: 1000,
        };
        let across = Window { x: 1300, ..inside };
        assert!(outer.holds(inside));
        assert!(!outer.holds(across));
    }
}
