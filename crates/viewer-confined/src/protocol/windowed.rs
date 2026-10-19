//! A page past `viewer_core::MAX_PIXELS` whose marks cannot cross, drawn in this process as the
//! window shows it (ADR 1986).
//!
//! A page past the bound crosses as its marks for a host that draws them at the window's size
//! (ADR 1944), and two kinds of page cannot: one the list format refuses
//! ([`Uncodable`](super::display_list::Uncodable) — a producer deferred to the device's scale,
//! §11.7.5.2's transfer channel, a nest past what a backend composites) and one whose list is past
//! the gibibyte a page's pixels may claim here. The toolkit windows draw both. Neither can cross as
//! a raster of the page, because that raster is the allocation the bound exists to refuse.
//! What can cross is the window: the part of the page the viewport shows and a quarter of the
//! viewport round it, drawn here from the list this process already holds.
//!
//! **The window is this process's statement and not a request the host makes.** The frame says
//! which part of the target it is, and the host places it there, so the host and this module need
//! not agree on the margin for the page to land where it belongs. The rule is
//! `viewer_host::oversize`'s all the same — the same quarter, for the reason ADR 1930 gives — and
//! it is written again here, in the twenty lines below, because this process links no host crate:
//! `viewer_host` brings the host's network client with it (ADR 1871).

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
    use super::{Window, span};
    use pdf_render::{TargetSpec, Transform};

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
