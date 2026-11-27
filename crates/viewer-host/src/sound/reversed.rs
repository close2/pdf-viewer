//! A movie's sound played backward: ISO 32000-2 §13.4, Table 307's `/Rate` and `/Mode`
//! `Palindrome` (ADRs 3841, 3842).
//!
//! > If the value of this entry is negative, the movie shall be played backward with respect to
//! > Start and Duration.
//!
//! A sound can only be decoded forward — an AAC frame's samples overlap the frame before it — so
//! its last sample exists only once the whole section has been decoded. The player therefore holds
//! the section's converted samples, as [`Turning`], until its last run, and only then hands the
//! sink a sound whose frames run last first: the order of the runs and the samples within each run
//! reversed, each frame's channels kept in their order. The sink then holds what a pass reads, as
//! it does going forward (ADR 3457), and the pass, its clock and the pictures following it are
//! timed exactly as a forward pass is, from the section's end.
//!
//! `Palindrome` — "Play continuously forward and backward until stopped" — has the sink hold the
//! section twice, in the first pass's direction and then in the other, and each second pass
//! starts at the second copy ([`Direction::pass_from`]). Nothing is decoded again: one decode of
//! the file serves every pass in both directions.
//!
//! **The bound** is [`MAX_REVERSED_SAMPLES`], half of [`MAX_PLAYED_SAMPLES`], so that a palindrome's
//! two copies are no more than a forward sound's one. A section past it is refused by name, before
//! any of it sounds where its length is known at the first run and as it arrives otherwise; it is
//! never cut short.

use pdf_model::media::Playing;

use super::MAX_PLAYED_SAMPLES;

/// The most samples, every channel counted, of a movie's section this player holds to play it
/// backward: half of [`MAX_PLAYED_SAMPLES`], so a palindrome's two copies fit in what a forward
/// sound may take, which is 87 seconds of a 48 kHz stereo section (ADR 3841).
pub const MAX_REVERSED_SAMPLES: usize = MAX_PLAYED_SAMPLES / 2;

/// Which way a sound's passes run (Table 307's `/Rate` and `/Mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Every pass forward, the sound streamed as it arrives.
    Forward,
    /// Every pass backward: a negative `/Rate`.
    Backward,
    /// `Palindrome`: the passes alternate, the first backward where `/Rate` is negative.
    Palindrome {
        /// Whether the first pass runs backward.
        first_backward: bool,
    },
}

impl Direction {
    /// The direction `playing` asks for.
    pub(crate) fn of(playing: &Playing) -> Self {
        match (playing.palindrome, playing.backward) {
            (true, first_backward) => Self::Palindrome { first_backward },
            (false, true) => Self::Backward,
            (false, false) => Self::Forward,
        }
    }

    /// Whether the sound is held whole before any of it sounds: every direction but forward.
    pub(crate) fn holds(self) -> bool {
        self != Self::Forward
    }

    /// The frame of the sink's sound where pass `number` (from zero) reads frame `from` of its
    /// own: the second copy for every second pass of a palindrome, `intrinsic` frames in.
    pub(crate) fn pass_from(self, number: u64, from: u64, intrinsic: u64) -> u64 {
        match self {
            Self::Palindrome { .. } if number % 2 == 1 => from.saturating_add(intrinsic),
            _ => from,
        }
    }
}

/// The sentence a section past [`MAX_REVERSED_SAMPLES`] is refused with.
pub(crate) fn too_long(samples: u64) -> String {
    format!(
        "its section of {samples} samples is past the {MAX_REVERSED_SAMPLES} this player holds to \
         play a movie's sound backward (ISO 32000-2 §13.4, Table 307's negative /Rate or /Mode \
         /Palindrome)"
    )
}

/// A section's converted samples, held until its last run so that it can be turned.
#[derive(Debug)]
pub(crate) struct Turning {
    samples: Vec<f32>,
    channels: usize,
}

impl Turning {
    /// Nothing held yet, of interleaved frames of `channels` samples.
    pub(crate) fn new(channels: usize) -> Self {
        Self {
            samples: Vec::new(),
            channels: channels.max(1),
        }
    }

    /// Holds `samples`, the next the section has.
    ///
    /// # Errors
    ///
    /// [`too_long`]'s sentence where the section would pass [`MAX_REVERSED_SAMPLES`].
    pub(crate) fn hold(&mut self, samples: &[f32]) -> Result<(), String> {
        let total = self.samples.len().saturating_add(samples.len());
        if total > MAX_REVERSED_SAMPLES {
            return Err(too_long(u64::try_from(total).unwrap_or(u64::MAX)));
        }
        self.samples.extend_from_slice(samples);
        Ok(())
    }

    /// Hands the sound `direction` reads to `append`: the section last frame first; or, for a
    /// palindrome, the section in the first pass's direction and then in the other.
    ///
    /// # Errors
    ///
    /// What `append` answers.
    pub(crate) fn into_sink(
        mut self,
        direction: Direction,
        mut append: impl FnMut(&[f32]) -> Result<(), String>,
    ) -> Result<(), String> {
        match direction {
            Direction::Forward => append(&self.samples),
            Direction::Backward => {
                self.turn();
                append(&self.samples)
            }
            Direction::Palindrome { first_backward } => {
                if first_backward {
                    self.turn();
                }
                append(&self.samples)?;
                self.turn();
                append(&self.samples)
            }
        }
    }

    /// The frames in reverse order, each frame's channels in their own.
    fn turn(&mut self) {
        self.samples.reverse();
        for frame in self.samples.chunks_exact_mut(self.channels) {
            frame.reverse();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, MAX_REVERSED_SAMPLES, Turning};

    /// The sink's sound, collected.
    fn sunk(turning: Turning, direction: Direction) -> Vec<f32> {
        let mut sink = Vec::new();
        turning
            .into_sink(direction, |samples| {
                sink.extend_from_slice(samples);
                Ok(())
            })
            .expect("collected");
        sink
    }

    /// Two runs of stereo frames whose left sample is the frame's place and whose right is its
    /// negative come back last frame first, each frame still left then right: block order and
    /// order within a block both reversed, the channels not swapped.
    #[test]
    fn a_section_turns_frame_by_frame_across_its_runs() {
        let mut turning = Turning::new(2);
        turning.hold(&[0.0, -0.0, 1.0, -1.0]).expect("held");
        turning.hold(&[2.0, -2.0]).expect("held");
        assert_eq!(
            sunk(turning, Direction::Backward),
            vec![2.0, -2.0, 1.0, -1.0, 0.0, -0.0]
        );
    }

    /// A palindrome's sink holds the section in its first direction and then the other, and a
    /// second pass reads the second copy; a first pass that runs backward turns the order.
    #[test]
    fn a_palindrome_holds_both_directions_and_every_second_pass_reads_the_second() {
        let mut turning = Turning::new(1);
        turning.hold(&[0.0, 1.0, 2.0]).expect("held");
        let first_forward = Direction::Palindrome {
            first_backward: false,
        };
        assert_eq!(
            sunk(turning, first_forward),
            vec![0.0, 1.0, 2.0, 2.0, 1.0, 0.0]
        );
        assert_eq!(first_forward.pass_from(0, 1, 3), 1);
        assert_eq!(first_forward.pass_from(1, 1, 3), 4);
        assert_eq!(first_forward.pass_from(2, 0, 3), 0);
        assert_eq!(Direction::Backward.pass_from(1, 0, 3), 0);
        let mut turning = Turning::new(1);
        turning.hold(&[0.0, 1.0, 2.0]).expect("held");
        assert_eq!(
            sunk(
                turning,
                Direction::Palindrome {
                    first_backward: true
                }
            ),
            vec![2.0, 1.0, 0.0, 0.0, 1.0, 2.0]
        );
    }

    /// A section one sample past the bound is refused by its sentence, and one at it is held.
    #[test]
    fn a_section_past_the_bound_is_refused_by_name() {
        let mut turning = Turning::new(1);
        turning
            .hold(&vec![0.0; MAX_REVERSED_SAMPLES])
            .expect("at the bound");
        let refused = turning.hold(&[0.0]).expect_err("past the bound");
        assert!(refused.contains("backward"), "{refused}");
    }
}
