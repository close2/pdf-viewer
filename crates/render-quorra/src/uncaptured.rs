//! What the graphics device reports that no call returned — kept as a value, so that a host can
//! *decide* rather than read a line of output going past.
//!
//! # Why wgpu has a channel like this at all
//!
//! Most of `wgpu`'s calls answer for themselves. Two of the ones on a window's path do not:
//! `Surface::configure` returns `()`, and so does `Queue::write_buffer`. When one of those fails,
//! the only place it is said is the device's *uncaptured error handler*, whose default is
//! silence — and a host that installs a handler which prints has made the failure visible and
//! nothing more. The call after it proceeds as though the device had agreed.
//!
//! # The failure this type was written for
//!
//! The project owner's viewer aborted on launch. `Surface::configure` failed with wgpu's
//! `GpuWaitTimeout`; this program printed the handler's line and carried on; the acquire that
//! followed found a surface that had never been configured, and `wgpu` **panics** there rather
//! than returning a status — which under `panic = "abort"` is a core dump. The second launch, on
//! the same document seconds later, worked.
//!
//! One line of that sequence is this crate's: the note. A note is not a decision, and
//! `CLAUDE.md` principle 1 asks for the second. So the handler records what it was told, and
//! the host takes it — `crate::QuorraWindowRenderer::uncaptured` — at the point where it knows
//! what to do about it. The handler still *says* it at once, because an error that ends the
//! process must have been said before it does; what is new is that it also survives the sentence.
//!
//! # Threading
//!
//! The handler is called on whatever thread made the failing call, which in this viewer is two
//! of them: the event thread configures the surface, the render thread uploads. So the record is
//! shared behind an `Arc` and interior-locked, and a host may take from it on either.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Errors the graphics device reported to its uncaptured-error handler, waiting to be acted on.
///
/// Held behind an `Arc` by [`crate::QuorraWindowRenderer`] and handed out by
/// [`crate::QuorraWindowRenderer::uncaptured`]; the renderer keeps recording into it after it has
/// moved to a thread of its own, which is exactly why the host needs a handle rather than a
/// borrow.
#[derive(Debug, Default)]
pub struct UncapturedErrors {
    /// Every one since this device was built, never reset.
    ///
    /// A *total* beside the drainable record below, because the two answer different questions: a
    /// host asks "what has happened that I have not dealt with" every frame, and a report at the
    /// end of a run asks "did this device complain at all".
    seen: AtomicU64,
    /// The ones no host has taken yet, folded into one.
    ///
    /// Folded rather than queued deliberately. A device that has begun to fail produces these by
    /// the frame, and a host that acted on each in turn would print a screen of the same sentence;
    /// what a decision needs is *how many* and *the most recent words*, which is
    /// [`Uncaptured`].
    pending: Mutex<Option<Uncaptured>>,
}

/// What the device has said since a host last asked — never nothing.
///
/// `Option<Uncaptured>` rather than an `Uncaptured` with a zero count, for the reason
/// `quorra_gpu::Presenter::last` gives one layer down: "no errors" and "one error with no words"
/// are different facts, and a type that cannot tell them apart says something untrue about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uncaptured {
    /// How many the device reported since the last [`UncapturedErrors::take`]. At least one.
    pub since: u64,
    /// The most recent one's words, as wgpu formatted them — several lines, typically, because
    /// wgpu prints an error's whole `source` chain.
    pub last: String,
}

impl UncapturedErrors {
    /// Records one, and says it at once.
    ///
    /// Saying it here rather than leaving it to the host is not a duplicate of what the host
    /// prints: this is the *device's* sentence and the host's is what it did about it. An error
    /// that kills the process before the next frame — which is exactly the launch abort this
    /// module exists for — must still have been said by somebody.
    pub(crate) fn record(&self, said: String) {
        eprintln!("note: the graphics device reported: {said}");
        self.seen.fetch_add(1, Ordering::Relaxed);
        let mut pending = lock(&self.pending);
        match pending.as_mut() {
            Some(held) => {
                held.since = held.since.saturating_add(1);
                held.last = said;
            }
            None => {
                *pending = Some(Uncaptured {
                    since: 1,
                    last: said,
                });
            }
        }
    }

    /// Takes whatever the device has said since this was last asked, leaving nothing behind.
    ///
    /// `None` is the ordinary answer and means the device has reported nothing — which is what a
    /// caller checks *after* a call that can only fail this way.
    #[must_use]
    pub fn take(&self) -> Option<Uncaptured> {
        lock(&self.pending).take()
    }

    /// How many the device has reported since it was built, taken or not.
    #[must_use]
    pub fn seen(&self) -> u64 {
        self.seen.load(Ordering::Relaxed)
    }
}

/// The record, whether or not a thread died holding it.
///
/// A poisoned lock here means some thread panicked between the two lines of [`Self::record`],
/// which leaves an `Option` that is either the old value or the new one — both of them true
/// statements about what the device said. There is no invariant to have broken, so recovering is
/// right and panicking would turn a device's complaint into a second crash.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::UncapturedErrors;

    /// Nothing said, nothing to take — the answer on every frame of a healthy run.
    #[test]
    fn a_quiet_device_hands_back_nothing() {
        let errors = UncapturedErrors::default();
        assert!(errors.take().is_none());
        assert_eq!(errors.seen(), 0);
    }

    /// The words survive the sentence, which is the whole point: a host that reads this after a
    /// `Surface::configure` that returned `()` learns that it failed.
    #[test]
    fn what_the_device_said_survives_to_be_taken() {
        let errors = UncapturedErrors::default();
        errors.record("Validation Error: In Surface::configure".to_owned());
        let taken = errors.take().expect("one was recorded");
        assert_eq!(taken.since, 1);
        assert!(taken.last.contains("Surface::configure"));
        // Drained: a decision is made once per occurrence, not once per frame for ever.
        assert!(errors.take().is_none());
    }

    /// Several between two frames fold into one decision with a count — see
    /// [`UncapturedErrors::pending`] for why a queue would be the wrong shape.
    #[test]
    fn several_fold_into_one_with_a_count() {
        let errors = UncapturedErrors::default();
        errors.record("first".to_owned());
        errors.record("second".to_owned());
        errors.record("third".to_owned());
        let taken = errors.take().expect("three were recorded");
        assert_eq!(taken.since, 3);
        assert_eq!(taken.last, "third", "the most recent words, not the oldest");
        assert_eq!(errors.seen(), 3, "the total is not reset by taking");
    }
}
