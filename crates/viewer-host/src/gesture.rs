//! A touchpad's two magnifying gestures: the pinch and the two-finger double tap.
//!
//! ISO 32000-2 says nothing about a touchpad, so everything here is a choice, made once for every
//! window because what a pinch is worth is not a fact about a toolkit (ADR 2848). Each window
//! reads its toolkit's gesture and hands the number here; where the zoom is anchored is the
//! window's to say, by [`crate::wheel`]'s rule — the pointer's point of the page stays still, and
//! over a panel there is no such point and the anchor is `None`.
//!
//! **Whether a gesture reaches the window at all is the operating system's.** macOS sends the
//! double tap only while the person has *Smart zoom* turned on in the trackpad settings, and the
//! pinch only while *Zoom in or out* is, so a window that answers both obeys those settings
//! without reading them.

use viewer_core::Zoom;

use crate::wheel::ZoomTravel;

/// The zoom one pinch event asks for, from the toolkit's `delta`: the change in magnification
/// since the last event, positive apart (winit's `PinchGesture`, `AppKit`'s `magnification`).
///
/// **A factor and never a step**: a finger's travel is continuous and the page should follow it,
/// which is [`ZoomTravel::Factor`]'s proportion rather than a notch. `1 + delta` is the
/// convention both toolkits state for the number, and a `delta` of −1 or less — a pinch
/// that would take the page to nothing or past it — or one that is not a number is refused
/// as no zoom, as [`ZoomTravel::of`] refuses a wheel reporting nonsense.
#[must_use]
pub fn pinch(delta: f64) -> ZoomTravel {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a pinch event's delta is a small fraction, well inside f32"
    )]
    let factor = (1.0 + delta) as f32;
    // A phase's first and last events carry no travel, and a zoom by nothing would still be a
    // command and a frame.
    let moved = delta.abs() > f64::EPSILON;
    if moved && factor.is_finite() && factor > 0.0 {
        ZoomTravel::Factor(factor)
    } else {
        ZoomTravel::Steps(0)
    }
}

/// How far two magnifications may differ and still be the same one: the reading of a magnification
/// back from a page's geometry goes through a division in `f32`.
const SAME: f32 = 1e-3;

fn same(a: f32, b: f32) -> bool {
    (a - b).abs() <= SAME * a.abs().max(b.abs())
}

/// The two-finger double tap — macOS's *Smart zoom* — as a toggle between the page's width and
/// where the reader was.
///
/// **The choice, written down** (ADR 2848): a first tap fits the page's width about the pointer,
/// and a second tap with nothing moved in between puts back the magnification the first one left.
/// A page already at its width has nothing to fit, so the first tap doubles the magnification
/// instead — a tap is always *in* and the one after it always *back*. Anything else that changes
/// the magnification between the two taps — a pinch, a wheel, a key — makes the next tap a first
/// tap again, because the reader is no longer where the first tap put them and "back" would be a
/// jump from somewhere they chose.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SmartZoom {
    back: Option<Back>,
}

/// What a first tap left: where to go back to, and where the tap arrived.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Back {
    to: f32,
    arrived: f32,
}

impl SmartZoom {
    /// One tap, with the magnification `showing` now (logical pixels per user space unit, what
    /// [`Zoom::Scale`] takes and [`magnification`](crate::wheel::magnification) reads).
    ///
    /// `zoom` sends a zoom to the viewer and answers the magnification it left, `None` where no
    /// page is showing; it is called once, or twice when a page already at its width is doubled.
    pub fn tap(&mut self, showing: f32, mut zoom: impl FnMut(Zoom) -> Option<f32>) {
        if let Some(back) = self.back.take()
            && same(showing, back.arrived)
        {
            zoom(Zoom::Scale(back.to));
            return;
        }
        let Some(mut arrived) = zoom(Zoom::FitWidth) else {
            return;
        };
        if same(arrived, showing) {
            let Some(doubled) = zoom(Zoom::Scale(showing * 2.0)) else {
                return;
            };
            arrived = doubled;
        }
        self.back = Some(Back {
            to: showing,
            arrived,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A viewer standing in for the real one: a fit-width magnification, and whatever a scale
    /// asked for, clamped as `viewer-core` clamps it.
    struct Window {
        width: f32,
        showing: f32,
        sent: Vec<Zoom>,
    }

    impl Window {
        fn zoom(&mut self, zoom: Zoom) -> f32 {
            self.sent.push(zoom);
            self.showing = match zoom {
                Zoom::FitWidth => self.width,
                Zoom::Scale(scale) => scale.clamp(0.02, 64.0),
                _ => self.showing,
            };
            self.showing
        }
    }

    /// The magnification showing is the one expected, as [`same`] reads two magnifications.
    #[track_caller]
    fn shows(window: &Window, expected: f32, why: &str) {
        assert!(
            same(window.showing, expected),
            "{why}: showing {}, expected {expected}",
            window.showing
        );
    }

    #[test]
    fn a_pinch_is_a_factor_of_what_is_showing() {
        assert_eq!(pinch(0.25), ZoomTravel::Factor(1.25));
        assert_eq!(pinch(-0.2), ZoomTravel::Factor(0.8));
    }

    #[test]
    fn a_pinch_to_nothing_or_nonsense_is_no_zoom() {
        for delta in [0.0, -1.0, -3.0, f64::NAN, f64::INFINITY] {
            assert_eq!(pinch(delta), ZoomTravel::Steps(0), "{delta}");
        }
    }

    #[test]
    fn a_second_tap_goes_back_to_where_the_first_began() {
        let mut window = Window {
            width: 1.6,
            showing: 1.0,
            sent: Vec::new(),
        };
        let mut smart = SmartZoom::default();
        smart.tap(window.showing, |zoom| Some(window.zoom(zoom)));
        shows(&window, 1.6, "the first tap fits the width");
        smart.tap(window.showing, |zoom| Some(window.zoom(zoom)));
        shows(&window, 1.0, "the second goes back");
        assert_eq!(
            window.sent,
            [Zoom::FitWidth, Zoom::Scale(1.0)],
            "one zoom a tap"
        );
    }

    #[test]
    fn a_page_already_at_its_width_is_doubled_and_then_put_back() {
        let mut window = Window {
            width: 1.6,
            showing: 1.6,
            sent: Vec::new(),
        };
        let mut smart = SmartZoom::default();
        smart.tap(window.showing, |zoom| Some(window.zoom(zoom)));
        shows(&window, 3.2, "a fit that moves nothing is a doubling");
        smart.tap(window.showing, |zoom| Some(window.zoom(zoom)));
        shows(&window, 1.6, "and the second tap comes back");
    }

    #[test]
    fn a_zoom_between_the_taps_makes_the_next_a_first_tap() {
        let mut window = Window {
            width: 1.6,
            showing: 1.0,
            sent: Vec::new(),
        };
        let mut smart = SmartZoom::default();
        smart.tap(window.showing, |zoom| Some(window.zoom(zoom)));
        // A pinch in between: the reader is somewhere they chose.
        window.zoom(Zoom::Scale(2.5));
        smart.tap(window.showing, |zoom| Some(window.zoom(zoom)));
        shows(&window, 1.6, "fitted again, not sent back to 1.0");
    }

    #[test]
    fn a_doubling_the_range_stops_still_comes_back() {
        let mut window = Window {
            width: 64.0,
            showing: 64.0,
            sent: Vec::new(),
        };
        let mut smart = SmartZoom::default();
        smart.tap(window.showing, |zoom| Some(window.zoom(zoom)));
        smart.tap(window.showing, |zoom| Some(window.zoom(zoom)));
        shows(
            &window,
            64.0,
            "the range's end is where the taps come back to",
        );
        assert_eq!(
            window.sent.last(),
            Some(&Zoom::Scale(64.0)),
            "the clamped arrival is where the first tap is recognised from"
        );
    }
}
