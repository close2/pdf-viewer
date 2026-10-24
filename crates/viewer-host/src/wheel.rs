//! Control and the wheel: how far a wheel or a touchpad has to travel for one zoom step.
//!
//! The standard says nothing about a wheel, so everything here is a choice — and it is made once
//! for the three windows because what a notch of magnification is worth is not a fact about a
//! toolkit. Each window converts its own toolkit's report into *lines* (a notch is one) or into
//! *pixels* (a touchpad), and [`ZoomWheel`] turns that travel into whole steps of
//! `viewer_core::Zoom`. Where the step is anchored is each window's to say, by one rule: the
//! pointer's point of the page stays still, and over a panel there is no such point and the
//! anchor is `None` (ADR 1118).
//!
//! **A wheel counts in whole notches and a touchpad in pixels, in every window** (ADRs 2274,
//! 2291). GTK's scroll controller is built `DISCRETE` and hands its window a wheel's whole notches,
//! a finger's surface pixels going to its smooth controller instead; both windows spend either by
//! [`WheelTravel::distance`]. Qt from
//! 6.12 on reports every Wayland wheel event with a pixel delta beside its angle, and a window that
//! preferred the pixels would move 15 logical pixels a notch where GTK moves 48, and need three and
//! a third notches for a zoom step where GTK needs one. [`WheelTravel::of`] is the rule that picks
//! the unit, and [`DiscreteWheel`] counts a wheel's fractions into notches exactly as GTK's
//! controller counts them, so the two windows scroll alike for every wheel.
//!
//! **A zoom is a step a click and in proportion between** ([`ZoomTravel`]): a whole notch is one
//! step of `viewer_core::ZOOM_STEP`, exactly as a key press is, and a part of one — a
//! high-resolution wheel's eighth, a touchpad's pixels — magnifies by that part of a step at once,
//! so such a device zooms as smoothly as it reports and a notched one by the click.

/// How far a touchpad must be dragged under Ctrl for one zoom step, in pixels.
///
/// A choice, not a derivation: a notch of a mouse wheel is one step by construction and a
/// touchpad reports a stream of pixels instead, so something has to say how many of them a notch
/// is worth. Fifty is about a finger's width on a laptop's touchpad and gives roughly the same
/// number of steps per gesture as the wheel does per flick.
pub const ZOOM_PIXELS: f32 = 50.0;

/// How far one notch of a wheel moves the page, in logical pixels.
///
/// A choice, and written down as one: the standard says nothing about a wheel. Three lines of a
/// document's body text is what every reader the project owner uses has converged on, and this is
/// about that. One number for every window, because what a notch is worth is not a fact about a
/// toolkit.
pub const SCROLL_NOTCH: f32 = 48.0;

/// What one wheel event travelled, in the unit [`Self::of`] says to count it in.
///
/// The signs are the toolkit's: this type decides the unit and nothing about the direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelTravel {
    /// A wheel's travel, in notches; a high-resolution wheel sends fractions of one.
    Notches {
        /// Across.
        x: f32,
        /// Up and down.
        y: f32,
    },
    /// A touchpad's travel, in logical pixels.
    Pixels {
        /// Across.
        x: f32,
        /// Up and down.
        y: f32,
    },
}

impl WheelTravel {
    /// The unit an event is counted in, from what the toolkit reported: `notches` (zero where the
    /// event stated no angle), `pixels` (zero where it stated none), and whether the event belongs
    /// to a `gesture` — a touchpad's, which a toolkit marks by a scroll phase or by naming the
    /// system as the event's source.
    ///
    /// **A wheel counts in notches whatever else the event carries.** A wheel's pixel delta is the
    /// compositor's notion of a notch's distance (`KWin`'s is fifteen), not a distance the person
    /// chose, and counting it would make a notch worth whatever that compositor's number is. A
    /// gesture counts in pixels, because a finger's travel *is* a distance. Each falls back to the
    /// other unit only where it stated nothing in its own (ADR 2274).
    #[must_use]
    pub fn of(notches: (f32, f32), pixels: (f32, f32), gesture: bool) -> Self {
        let stated = |(x, y): (f32, f32)| x != 0.0 || y != 0.0;
        let in_pixels = if gesture {
            stated(pixels) || !stated(notches)
        } else {
            !stated(notches) && stated(pixels)
        };
        if in_pixels {
            Self::Pixels {
                x: pixels.0,
                y: pixels.1,
            }
        } else {
            Self::Notches {
                x: notches.0,
                y: notches.1,
            }
        }
    }
}

impl WheelTravel {
    /// How far this travel moves the page, in logical pixels with the travel's own signs: a notch
    /// moves [`SCROLL_NOTCH`] and a touchpad's pixel moves a pixel (ADRs 2274, 2291).
    ///
    /// **The one distance both windows spend**, so a finger moves the page as far in each and a
    /// notch moves it as far in each. A wheel's notches are counted whole before they get here —
    /// by GTK's `DISCRETE` controller in one window and by [`DiscreteWheel`] in the other — and a
    /// touchpad's pixels are spent as they come, never counted into notches: ten pixels of a finger
    /// counted as a notch would move the page forty-eight.
    #[must_use]
    pub fn distance(self) -> (f32, f32) {
        match self {
            Self::Notches { x, y } => (x * SCROLL_NOTCH, y * SCROLL_NOTCH),
            Self::Pixels { x, y } => (x, y),
        }
    }
}

/// What one Control + wheel event does to the magnification, turned away from the person positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoomTravel {
    /// Whole steps of `viewer_core::ZOOM_STEP`, positive in: what a notched wheel's click is.
    Steps(i32),
    /// A factor the magnification showing is multiplied by: part of a step, or more than one that
    /// is not a whole number of them.
    Factor(f32),
}

impl ZoomTravel {
    /// The zoom one event's `travel` asks for (ADR 2274).
    ///
    /// **A whole notch is a step and anything else is a proportion.** Counting a fraction into
    /// whole steps would hold a high-resolution wheel's first seven eighths back and spend them on
    /// the eighth, which is the click a person with such a wheel did not ask for; spending it as
    /// `ZOOM_STEP` to the power of the fraction gives eight eighths the magnification of one click,
    /// no carry to lose and none to drift. A touchpad's pixels are a proportion at
    /// [`ZOOM_PIXELS`] a step. A device reporting nonsense is refused, as [`ZoomWheel::lines`]
    /// refuses it.
    #[must_use]
    pub fn of(travel: WheelTravel) -> Self {
        let (steps, whole_notches) = match travel {
            WheelTravel::Notches { y, .. } => (y, y.fract() == 0.0),
            WheelTravel::Pixels { y, .. } => (y / ZOOM_PIXELS, false),
        };
        if !steps.is_finite() || steps == 0.0 {
            return Self::Steps(0);
        }
        let steps = steps.clamp(-64.0, 64.0);
        if whole_notches {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a whole number clamped to ±64 on the line above"
            )]
            let whole = steps as i32;
            Self::Steps(whole)
        } else {
            Self::Factor(viewer_core::ZOOM_STEP.powf(steps))
        }
    }

    /// The commands this is for `viewer`, about `at` in device pixels of the viewport, at the
    /// window's `scale`: a step is [`viewer_core::Zoom::In`] or [`viewer_core::Zoom::Out`], a
    /// factor a [`viewer_core::Zoom::Scale`] of the magnification the page shows now — the one
    /// place both windows turn a wheel into a zoom, so they cannot turn it differently. Nothing
    /// where no page is showing.
    #[must_use]
    pub fn commands(
        self,
        viewer: &viewer_core::Viewer,
        scale: f32,
        at: Option<(f32, f32)>,
    ) -> Vec<viewer_core::Command> {
        use viewer_core::{Answer, Command, Query, Zoom};
        match self {
            Self::Steps(steps) => {
                let zoom = if steps > 0 { Zoom::In } else { Zoom::Out };
                (0..steps.unsigned_abs())
                    .map(|_| Command::Zoom { zoom, at })
                    .collect()
            }
            Self::Factor(factor) => {
                let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
                    return Vec::new();
                };
                let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(index)) else {
                    return Vec::new();
                };
                // A scale that is not a positive number has no magnification to multiply.
                if !scale.is_finite() || scale <= 0.0 {
                    return Vec::new();
                }
                vec![Command::Zoom {
                    zoom: Zoom::Scale(geometry.scale / scale * factor),
                    at,
                }]
            }
        }
    }
}

/// A wheel's travel counted into whole notches, across and up and down, the way GTK's `DISCRETE`
/// scroll controller counts it.
///
/// **The rule is GTK 4's, taken whole so that the other window matches it rather than resembling
/// it** (`gtkeventcontrollerscroll.c`, the wheel-unit branch): the travel is added to what is
/// carried; once it reaches half a notch either way the whole notches in it are spent — one at
/// least — and the rest is carried on; and a turn the other way throws the carry away first. So a
/// high-resolution wheel sending eighths moves a notch on its fourth eighth and every eighth eighth
/// after, and a whole notch moves a notch at once. The standard says nothing about a wheel; this
/// is a choice to agree with the toolkit the other window is built on (ADR 2274).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DiscreteWheel {
    /// Across.
    x: Discrete,
    /// Up and down.
    y: Discrete,
}

impl DiscreteWheel {
    /// How many whole notches `(x, y)` notches of travel complete across and up and down, given
    /// what earlier events left; the signs are the caller's.
    pub fn notches(&mut self, x: f32, y: f32) -> (i32, i32) {
        (self.x.spend(x), self.y.spend(y))
    }
}

/// One axis of [`DiscreteWheel`].
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Discrete {
    /// The travel not yet spent on a notch, in notches.
    carry: f32,
    /// The last travel this axis was given that was not zero, whose sign a reversal is read from.
    last: f32,
}

impl Discrete {
    fn spend(&mut self, travel: f32) -> i32 {
        // As in `ZoomWheel::lines`: one non-finite value would poison the carry for good.
        if !travel.is_finite() {
            return 0;
        }
        if travel != 0.0 {
            if self.last == 0.0 || (travel < 0.0) != (self.last < 0.0) {
                self.carry = 0.0;
            }
            self.last = travel;
        }
        self.carry += travel;
        if self.carry.abs() < 0.5 {
            return 0;
        }
        let whole = self.carry.trunc();
        let spent = if whole == 0.0 {
            self.carry.signum()
        } else {
            whole
        };
        // The bound `ZoomWheel::lines` sets, for the reason it gives: a saturating cast of a
        // device's nonsense would otherwise be two billion notches.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "clamped to ±64 on the same line"
        )]
        let notches = spent.clamp(-64.0, 64.0) as i32;
        self.carry -= spent;
        notches
    }
}

/// Where the core put the page showing, as the fields of a JSON receipt: its origin in the
/// viewport and its device pixels per unit, read after a zoom step.
///
/// Diagnostic, for `PDFVIEWER_ZOOM_PROBE` in both windows alike: it is what lets
/// `tools/zoom-probe-wayland.py --drift` tell the core's anchor from a window's placement of the
/// frame, since the probe knows where the page point it watches is and the output only shows where
/// it was drawn (ADR 2275). `None` where no page is showing.
#[must_use]
pub fn placement_receipt(viewer: &viewer_core::Viewer) -> Option<String> {
    use viewer_core::{Answer, Query};
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        return None;
    };
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(index)) else {
        return None;
    };
    Some(format!(
        "\"page\":{index},\"origin_x\":{},\"origin_y\":{},\"scale\":{},\"page_height\":{}",
        geometry.origin.0, geometry.origin.1, geometry.scale, geometry.page.height
    ))
}

/// The travel a Ctrl + wheel gesture has made and not yet spent on a step.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ZoomWheel {
    /// The fraction of a step travelled and not yet spent, in lines.
    carry: f32,
}

impl ZoomWheel {
    /// How many whole zoom steps `pixels` of touchpad travel completes; positive is a zoom in.
    ///
    /// The pixels are converted at [`ZOOM_PIXELS`] and spent by [`Self::lines`].
    pub fn pixels(&mut self, pixels: f32) -> i32 {
        self.lines(pixels / ZOOM_PIXELS)
    }

    /// How many whole zoom steps `lines` of wheel travel completes, given what earlier events
    /// left unspent; positive is a zoom in, which is the wheel turned away from the person.
    ///
    /// **A line is not a notch.** A high-resolution wheel or a touchpad in line mode reports a
    /// *fraction* of a line per event, and truncating each event on its own spends nothing at all:
    /// in the trace of 2026-09-15 the device's quantum was about a thirty-seventh of a line, and
    /// 579 Ctrl + wheel events carrying 146.4 lines of travel over seven gestures produced **one**
    /// zoom step. So the fraction is carried and spent when it completes a step (ADR 1118).
    pub fn lines(&mut self, lines: f32) -> i32 {
        // An accumulator is poisoned permanently by one bad value, which a per-event truncation
        // could not be, so a device reporting a NaN or an infinity is ignored rather than added.
        if !lines.is_finite() {
            return 0;
        }
        self.carry += lines;
        let whole = self.carry.trunc();
        // `ZOOM_RANGE` spans 0.02 to 64, which is thirty-six steps of 1.25 end to end, so a bound
        // of sixty-four cannot hide a magnification anybody could have reached — it is there
        // because a `f32` cast saturates and a device reporting nonsense would otherwise be a loop
        // of two billion commands.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "clamped to ±64 on the same line"
        )]
        let steps = whole.clamp(-64.0, 64.0) as i32;
        // The whole of what `whole` claimed leaves the carry, clamped or not: a nonsense delta is
        // refused a magnification, not banked for the next event to spend.
        self.carry -= whole;
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::{DiscreteWheel, WheelTravel, ZOOM_PIXELS, ZoomTravel, ZoomWheel};

    /// The quantum the mouse in the trace of 2026-09-15 reported, in lines.
    const TRACE_QUANTUM: f32 = 0.026_981_818;

    /// Four lines of travel arriving as 149 events of a thirty-seventh of a line each are four
    /// steps, and a whole notch is still one step on the event that carries it.
    #[test]
    fn every_line_travelled_is_spent_and_a_notch_is_a_step() {
        let mut wheel = ZoomWheel::default();
        let carried: i32 = (0..149).map(|_| wheel.lines(TRACE_QUANTUM)).sum();
        assert_eq!(carried, 4);
        let mut wheel = ZoomWheel::default();
        assert_eq!(wheel.lines(1.0), 1);
        assert_eq!(wheel.lines(-3.0), -3);
    }

    /// A finger's ten pixels move the page ten pixels, in both windows, and not the notch GTK's
    /// `DISCRETE` controller would have counted them into; a notch moves [`super::SCROLL_NOTCH`]
    /// whichever window counted it (ADR 2291).
    #[test]
    fn a_finger_moves_the_page_as_far_as_it_went_and_a_notch_a_notch() {
        let finger = WheelTravel::of((0.0, 0.0), (0.0, 10.0), true);
        assert_eq!(finger, WheelTravel::Pixels { x: 0.0, y: 10.0 });
        assert_eq!(finger.distance(), (0.0, 10.0));
        let ten_fingers: f32 = (0..10)
            .map(|_| WheelTravel::Pixels { x: 0.0, y: -1.5 }.distance().1)
            .sum();
        assert!((ten_fingers + 15.0).abs() < 1e-4, "{ten_fingers}");
        let mut wheel = DiscreteWheel::default();
        let (_, counted) = wheel.notches(0.0, 1.0);
        #[expect(clippy::cast_precision_loss, reason = "one notch")]
        let notch = WheelTravel::Notches {
            x: 0.0,
            y: counted as f32,
        };
        assert_eq!(notch.distance(), (0.0, super::SCROLL_NOTCH));
    }

    /// [`ZOOM_PIXELS`] of touchpad travel is one step.
    #[test]
    fn a_touchpad_takes_fifty_pixels_a_step() {
        let mut wheel = ZoomWheel::default();
        let steps: i32 = (0..25).map(|_| wheel.pixels(ZOOM_PIXELS / 10.0)).sum();
        assert_eq!(steps, 2, "two and a half notches of travel is two steps");
    }

    /// Reversing cancels rather than banking, and nonsense neither runs away nor poisons the
    /// carry.
    #[test]
    fn a_reversal_cancels_and_nonsense_does_not_poison_the_carry() {
        let mut wheel = ZoomWheel::default();
        assert_eq!(wheel.lines(0.9), 0);
        assert_eq!(wheel.lines(-0.9), 0);
        assert_eq!(wheel.lines(1.0), 1);
        assert_eq!(wheel.lines(1e9), 64);
        assert_eq!(wheel.lines(f32::NAN), 0);
        assert_eq!(wheel.lines(f32::INFINITY), 0);
        assert_eq!(wheel.lines(1.0), 1, "the carry survived the nonsense");
    }

    /// GTK's count, read off its own window on a nested `KWin` (ADR 2274): eighths of a notch move
    /// a notch at the fourth and the twelfth, a whole notch moves one at once, and a reversal
    /// starts from nothing.
    #[test]
    fn a_wheel_counts_whole_notches_as_gtk_does() {
        let mut wheel = DiscreteWheel::default();
        let moved: Vec<i32> = (0..16).map(|_| wheel.notches(0.0, 0.125).1).collect();
        let at: Vec<usize> = (0..16).filter(|&i| moved[i] != 0).collect();
        assert_eq!(at, [3, 11], "the fourth and the twelfth eighth");
        assert_eq!(moved.iter().sum::<i32>(), 2);
        assert_eq!(
            wheel.notches(0.0, -1.0),
            (0, -1),
            "a reversal is a notch at once"
        );
        assert_eq!(
            wheel.notches(0.0, 0.25),
            (0, 0),
            "and the carry went with it"
        );
        assert_eq!(wheel.notches(2.0, 0.0), (2, 0));
        assert_eq!(wheel.notches(1e9, f32::NAN), (64, 0));
        assert_eq!(
            wheel.notches(0.0, 0.25),
            (0, 1),
            "nonsense did not poison the carry"
        );
    }

    /// A click is a step and anything less a proportion: eight eighths are one click's
    /// magnification, and a touchpad's fifty pixels are too.
    #[test]
    fn a_click_is_a_step_and_an_eighth_is_an_eighth_of_one() {
        let notches = |y| ZoomTravel::of(WheelTravel::Notches { x: 0.0, y });
        assert_eq!(notches(1.0), ZoomTravel::Steps(1));
        assert_eq!(notches(-2.0), ZoomTravel::Steps(-2));
        let ZoomTravel::Factor(eighth) = notches(0.125) else {
            panic!("an eighth is a proportion");
        };
        assert!(
            (eighth.powi(8) - viewer_core::ZOOM_STEP).abs() < 1e-5,
            "{eighth}"
        );
        let ZoomTravel::Factor(pad) = ZoomTravel::of(WheelTravel::Pixels { x: 0.0, y: 25.0 })
        else {
            panic!("a touchpad's pixels are a proportion");
        };
        assert!((pad * pad - viewer_core::ZOOM_STEP).abs() < 1e-5, "{pad}");
        assert_eq!(notches(f32::NAN), ZoomTravel::Steps(0));
        assert_eq!(notches(0.0), ZoomTravel::Steps(0));
        assert_eq!(notches(1e9), ZoomTravel::Steps(64));
    }

    /// The rule: a wheel's pixels are ignored, a gesture's are counted, and either falls back to
    /// what it did state.
    #[test]
    fn a_wheel_counts_notches_and_a_gesture_counts_pixels() {
        // Qt 6.12 on KWin: one notch is an angle of 120 eighths and a pixel delta of 15.
        assert_eq!(
            WheelTravel::of((0.0, 1.0), (0.0, 15.0), false),
            WheelTravel::Notches { x: 0.0, y: 1.0 }
        );
        assert_eq!(
            WheelTravel::of((0.0, 1.0), (0.0, 15.0), true),
            WheelTravel::Pixels { x: 0.0, y: 15.0 }
        );
        assert_eq!(
            WheelTravel::of((0.0, 1.0), (0.0, 0.0), true),
            WheelTravel::Notches { x: 0.0, y: 1.0 }
        );
        assert_eq!(
            WheelTravel::of((0.0, 0.0), (3.0, 0.0), false),
            WheelTravel::Pixels { x: 3.0, y: 0.0 }
        );
    }
}
