//! What a host does about a confined viewer that stopped answering.
//!
//! # Why this exists, and why it is a policy rather than a mechanism
//!
//! A worker dies for reasons the document chose. The address-space ceiling refuses an allocation
//! and libstd aborts; the seccomp filter fires; a decode deep inside the interpreter asks for more
//! than `RLIMIT_AS` will give. Every one of those arrives at a host as
//! [`ConfinedError::WorkerDied`], and until this module existed the only host on the boundary
//! answered all of them the same way: the window said the sentence and the document was over.
//!
//! **That treats one page's breach as the document's end, and it is not.** `doc/todo/15` records
//! the breach as owed "as a refusal", on the reasoning that a refusal the worker itself makes
//! needs a fallible allocation on a path this crate does not own — which is true, and which is
//! about the *worker*. What the reader needs is a refusal of the **page**, and that costs nothing
//! inside the confinement: the worker is a process, another one starts in milliseconds, the
//! document is on this side's filesystem by rule 2, and the command that killed it is simply not
//! sent again. The confinement is what makes this safe rather than hopeful — a worker's death
//! leaves nothing of the document behind it.
//!
//! # Why the counting is here and the starting is not
//!
//! Starting a worker needs a host's own things — the file, the window's extent, the page the
//! reader was on — and this crate deliberately reads no file and knows of no window. What it can
//! own is the part two confined hosts must not disagree about: **which errors are worth another
//! worker, how many are enough, and what a resume goes back to.** [`Resuming`] is that, it is
//! pure, and its tests need no pipe.

use crate::ConfinedError;

/// How many workers a host starts in a row before it stops trying.
///
/// **Three, and the number matters less than the word *in a row***: [`Resuming::showing`] puts it
/// back the moment a frame reaches the screen, so what this bounds is a *failing* recovery rather
/// than a long read. A document that kills a worker once every hundred pages is read to its end;
/// one whose every open kills a worker costs three starts and then says so.
///
/// The value is the smallest that distinguishes the three cases a host has to tell apart: one
/// start proves the death was not the open's, a second proves it was not the page's, and a third
/// leaves room for a machine that was momentarily unable to spawn. A larger number buys nothing —
/// a document that has failed three consecutive starts is not going to open — and a smaller one
/// cannot tell the second case from the first.
pub const RESTARTS: usize = 3;

/// Where a host goes back to, and how far into its budget it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reopen {
    /// The last page a frame arrived for, zero-based — where the reader was looking.
    ///
    /// The *page*, and not the view: magnification and scroll are the confined viewer's own
    /// state, and nothing on this boundary asks for them, so a host that replayed them would be
    /// guessing. What a host can restore exactly it restores; the rest returns to the document's
    /// opening view, and a host owes the reader that sentence.
    pub page: usize,
    /// Which start this is, counting from one.
    pub attempt: usize,
    /// How many there are in all — [`RESTARTS`], carried so that a host's sentence needs no
    /// arithmetic of its own.
    pub of: usize,
}

/// What to do about a viewer that stopped answering.
///
/// Two arms and no third: either another worker is worth starting or the host says why not. A
/// host matches this exhaustively, which is `doc/ui-boundary.md`'s rule for every closed
/// vocabulary that crosses into a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resume {
    /// Start another worker, open the document again, and go back to this page.
    Reopen(Reopen),
    /// Nothing left to start one for: the host reports the error and stops.
    Stop,
}

/// A host's count of what it has already tried for the document in front of it.
#[derive(Debug, Default)]
pub struct Resuming {
    /// Starts spent since the last frame reached the screen.
    spent: usize,
    /// The last page a frame arrived for.
    page: usize,
}

impl Resuming {
    /// Nothing tried yet, and page one.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A frame arrived for this page and the reader is looking at it.
    ///
    /// Two things at once, and they are the same fact: this is where a resume goes back to, and
    /// the budget is spent again from here. **Consecutive rather than cumulative**, because the
    /// two describe different worlds — a cumulative budget makes a document that recovers
    /// perfectly well fail on its fourth incident an hour into a read, which is a rule about the
    /// length of the reading rather than about the document.
    pub fn showing(&mut self, page: usize) {
        self.spent = 0;
        self.page = page;
    }

    /// What to do about `problem`, and the spending of a start if that is the answer.
    ///
    /// **Only [`ConfinedError::WorkerDied`] is worth another worker**, and every other arm is a
    /// [`Resume::Stop`] for a reason of its own rather than by default:
    ///
    /// - [`ConfinedError::Cancelled`] is the *host's* own kill (ADR 0241). A reader who pressed
    ///   the abort and got another worker would have pressed it for nothing.
    /// - [`ConfinedError::WorkerMissing`] and [`ConfinedError::Spawn`] are about starting one, so
    ///   starting one is exactly what will not help.
    /// - [`ConfinedError::Connection`] is a pipe failure with the worker still alive
    ///   (`Confined::explain` returns `WorkerDied` when it is not), so it is this side's channel
    ///   and a second worker would inherit the same one.
    /// - [`ConfinedError::Malformed`], [`ConfinedError::UnrecognisedFrame`],
    ///   [`ConfinedError::Uncarried`], [`ConfinedError::Refused`] and [`ConfinedError::NoRoom`]
    ///   all leave the worker **alive and answering** — each is a message refused, not a process
    ///   lost — so there is nothing to restart and restarting would throw away a working viewer.
    ///
    /// **Every arm is written out and there is no wildcard**, which is what makes the list above
    /// a claim rather than a memory: [`ConfinedError`] is `#[non_exhaustive]` to its callers but
    /// not to this module, so a variant added to it stops compiling here until somebody has said
    /// which of the two answers it is. That is `doc/ui-boundary.md`'s rule for a closed
    /// vocabulary, applied to the one enum a confined host has to reason about.
    pub fn after(&mut self, problem: &ConfinedError) -> Resume {
        match problem {
            ConfinedError::WorkerDied { .. } => {
                if self.spent >= RESTARTS {
                    return Resume::Stop;
                }
                self.spent = self.spent.saturating_add(1);
                Resume::Reopen(Reopen {
                    page: self.page,
                    attempt: self.spent,
                    of: RESTARTS,
                })
            }
            ConfinedError::WorkerMissing { .. }
            | ConfinedError::Spawn(_)
            | ConfinedError::Connection(_)
            | ConfinedError::Malformed(_)
            | ConfinedError::UnrecognisedFrame
            | ConfinedError::Uncarried(_)
            | ConfinedError::Refused { .. }
            | ConfinedError::NoRoom { .. }
            | ConfinedError::Cancelled => Resume::Stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RESTARTS, Reopen, Resume, Resuming};
    use crate::ConfinedError;

    /// A worker that stopped without answering.
    fn a_death() -> ConfinedError {
        ConfinedError::WorkerDied {
            detail: "killed by signal 6".to_owned(),
        }
    }

    /// The budget is spent one start at a time and then the host is told to stop.
    #[test]
    fn a_dead_worker_is_started_again_until_the_budget_is_gone() {
        let mut resuming = Resuming::new();
        for attempt in 1..=RESTARTS {
            assert_eq!(
                resuming.after(&a_death()),
                Resume::Reopen(Reopen {
                    page: 0,
                    attempt,
                    of: RESTARTS,
                }),
                "start {attempt} is owed"
            );
        }
        assert_eq!(
            resuming.after(&a_death()),
            Resume::Stop,
            "and the budget is not endless"
        );
    }

    /// A frame on the screen puts the budget back: what it bounds is a failing recovery, not the
    /// length of the reading.
    #[test]
    fn a_frame_that_reached_the_screen_gives_the_budget_back() {
        let mut resuming = Resuming::new();
        for _ in 0..RESTARTS {
            let _ = resuming.after(&a_death());
        }
        assert_eq!(resuming.after(&a_death()), Resume::Stop, "spent");
        resuming.showing(4);
        assert_eq!(
            resuming.after(&a_death()),
            Resume::Reopen(Reopen {
                page: 4,
                attempt: 1,
                of: RESTARTS,
            }),
            "the reader got somewhere, so the count starts again from there"
        );
    }

    /// A resume goes back to the last page a frame arrived for, not to the one that killed the
    /// worker.
    #[test]
    fn a_resume_returns_to_the_last_page_that_answered() {
        let mut resuming = Resuming::new();
        resuming.showing(3);
        resuming.showing(4);
        let Resume::Reopen(reopen) = resuming.after(&a_death()) else {
            panic!("a death is worth another worker");
        };
        assert_eq!(reopen.page, 4, "where the reader was when it last worked");
    }

    /// Every other refusal on this boundary stops, each for its own reason, and this walks all of
    /// them rather than trusting the reading — the shape ADR 0729's `Key::ALL` test has.
    ///
    /// This list is every variant the enum has, and it is a list rather than a match, so what
    /// keeps it complete is `Resuming::after`'s own wildcard-free match: a variant added there
    /// stops the build, and the round that adds an arm adds the value here beside it.
    #[test]
    fn only_a_dead_worker_is_worth_another_one() {
        let others = [
            ConfinedError::WorkerMissing {
                executable: std::path::PathBuf::from("pdf-viewer-confined"),
            },
            ConfinedError::Spawn(std::io::Error::other("no")),
            ConfinedError::Connection(std::io::Error::other("no")),
            ConfinedError::UnrecognisedFrame,
            ConfinedError::Refused {
                detail: "a raster in a second pixel layout".to_owned(),
            },
            ConfinedError::NoRoom { bytes: 1 << 40 },
            ConfinedError::Uncarried(crate::Uncarried {
                message: "Command::RenderReady",
                reason: "the confined process answers it itself",
            }),
            ConfinedError::Cancelled,
        ];
        for problem in &others {
            let mut resuming = Resuming::new();
            assert_eq!(
                resuming.after(problem),
                Resume::Stop,
                "{problem} is not a dead worker"
            );
        }
        // The one whose source has to be built rather than named, kept beside the others so
        // that every arm of the enum is accounted for rather than silently skipped.
        let mut resuming = Resuming::new();
        assert_eq!(
            resuming.after(&ConfinedError::Malformed(crate::ProtocolError::Truncated {
                what: "a frame header",
            })),
            Resume::Stop
        );
        assert_eq!(
            resuming.after(&a_death()),
            Resume::Reopen(Reopen {
                page: 0,
                attempt: 1,
                of: RESTARTS,
            }),
            "and none of them spent a start"
        );
    }
}
