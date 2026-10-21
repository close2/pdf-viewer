//! Control and the wheel through `quorra_window`: the travel a caller's toolkit reported, in the
//! unit it reported it in (ADR 2152).
//!
//! The three toolkit windows each turn their toolkit's report into lines or into pixels and hand
//! it to `viewer_host::ZoomWheel`, which carries the fraction of a step a high-resolution wheel
//! leaves (ADR 1118). A C caller gets the same arrangement: it says which of the two units its
//! platform's number is in, and the carry is the window's, so a fourth window cannot count a
//! gesture a fourth way.

/// The unit of the travel `quorra_window_zoom_wheel` is handed.
///
/// Taken, so a number outside the two is refused rather than read as one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum WheelKind {
    /// Lines, a notch of a mouse wheel being one: what a wheel and a touchpad in line mode report.
    Lines = 0,
    /// Pixels of a touchpad's drag, of which `viewer_host::ZOOM_PIXELS` make a step.
    Pixels = 1,
}

impl WheelKind {
    /// The kind for a number, or `None` for one this build does not define.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Lines,
            1 => Self::Pixels,
            _ => return None,
        })
    }

    /// How many whole zoom steps `travel` completes on `wheel`, which carries what is left over;
    /// positive is a zoom in.
    pub(crate) fn steps(self, wheel: &mut viewer_host::ZoomWheel, travel: f32) -> i32 {
        match self {
            Self::Lines => wheel.lines(travel),
            Self::Pixels => wheel.pixels(travel),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WheelKind;

    /// The two codes are the two units, and the next number is refused.
    #[test]
    fn the_two_units_are_taken_and_a_third_is_refused() {
        assert_eq!(WheelKind::from_code(0), Some(WheelKind::Lines));
        assert_eq!(WheelKind::from_code(1), Some(WheelKind::Pixels));
        assert_eq!(WheelKind::from_code(2), None);
    }

    /// A touchpad's pixels and a wheel's lines are spent by the one carry the toolkits use.
    #[test]
    fn pixels_and_lines_are_spent_by_the_toolkits_carry() {
        let mut wheel = viewer_host::ZoomWheel::default();
        assert_eq!(WheelKind::Lines.steps(&mut wheel, 0.5), 0);
        assert_eq!(WheelKind::Lines.steps(&mut wheel, 0.5), 1);
        assert_eq!(
            WheelKind::Pixels.steps(&mut wheel, -viewer_host::ZOOM_PIXELS),
            -1
        );
    }
}
