//! Control and the wheel: how far a wheel or a touchpad has to travel for one zoom step.
//!
//! The standard says nothing about a wheel, so everything here is a choice — and it is made once
//! for the three windows because what a notch of magnification is worth is not a fact about a
//! toolkit. Each window converts its own toolkit's report into *lines* (a notch is one) or into
//! *pixels* (a touchpad), and [`ZoomWheel`] turns that travel into whole steps of
//! `viewer_core::Zoom`. Where the step is anchored is each window's to say, by one rule: the
//! pointer's point of the page stays still, and over a panel there is no such point and the
//! anchor is `None` (ADR 1118).

/// How far a touchpad must be dragged under Ctrl for one zoom step, in pixels.
///
/// A choice, not a derivation: a notch of a mouse wheel is one step by construction and a
/// touchpad reports a stream of pixels instead, so something has to say how many of them a notch
/// is worth. Fifty is about a finger's width on a laptop's touchpad and gives roughly the same
/// number of steps per gesture as the wheel does per flick.
pub const ZOOM_PIXELS: f32 = 50.0;

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
    use super::{ZOOM_PIXELS, ZoomWheel};

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
}
