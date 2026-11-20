//! How long the parent waits for one answer from the worker: the worker's processor time, and a
//! wall-clock ceiling behind it (ADR 3133).
//!
//! **A decode's outcome is the page's, so the budget that ends it is counted in work.** The same
//! image decoded on an idle machine and on one whose load is two hundred costs the worker the same
//! processor time and a very different wall time: `issue19517.pdf`'s JPEG 2000 scan spends under
//! three seconds of processor on either, and at `nice 19` beside a loaded machine it took more
//! than thirty seconds of wall clock, so a wall-clock budget drew the page without its image on
//! one run in several — the page was a fact about the machine. ADR 1507 declined a clock inside
//! `interpret` for this reason; a decode whose output is all-or-nothing is the same case.
//!
//! The processor time is the worker's, read by the parent from `/proc/<pid>/stat` (`utime` and
//! `stime`), so the worker is not trusted to time itself, and it is the time spent **on this
//! request**, the reading when it was sent subtracted, so a long-lived worker that has decoded a
//! great deal is never charged for its past.
//!
//! **The wall clock stays, as a ceiling and as its own refusal.** A worker that neither answers
//! nor computes — stopped, starved of the processor, or subverted into waiting — spends no
//! processor time, and principle 3 wants every wait bounded. So a request also ends at
//! [`STALL_FACTOR`] times its processor budget of wall clock, as [`SandboxError::Stalled`]: a
//! refusal that says the machine decided it, which a caller that must be a function of the bytes —
//! the golden, the oracle — can tell from the image's own refusal ([`crate::wall_clock_refusals`]).
//! Where the worker's processor time cannot be read — a platform with no `/proc`, or a parent not
//! allowed to read it — the wall clock is the whole budget, the processor budget's length as
//! before, and its refusal is the same `Stalled`, because that is what it is.
//!
//! **The in-process path is held to the same two bounds** ([`crate::in_process`]): its decode runs
//! on one thread of this process, whose processor time `/proc/self/task/<tid>/stat` reads the same
//! way, and the thread names itself through the `/proc/thread-self` link, so no system call is
//! made that the standard library does not make safely (ADR 3150). There the refusals are
//! [`SandboxError::Exhausted`] and [`SandboxError::Overran`], because a thread is abandoned where a
//! worker is killed; [`Ended`] is what both paths share.

use std::time::{Duration, Instant};

use crate::SandboxError;

/// How many times its processor budget of wall clock a request is waited for before the parent
/// gives up on a worker that has not spent that budget.
///
/// Ten is a statement about the machine, and the sentence it makes is the refusal's: a worker that
/// has had less than a tenth of one processor for the whole of its wait has been stalled, and
/// what it was decoding is not known to be at fault. `issue19517.pdf`'s worker was measured under a
/// load of 155 to 258 at less than a tenth of a processor for thirty seconds (ADR 3133), so the
/// ceiling sits where the ordinary worker's three hundred seconds no longer turn on one bad minute
/// of a shared machine.
pub(crate) const STALL_FACTOR: u32 = 10;

/// Whose processor time an allowance reads.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Whose {
    /// A worker process, by its pid.
    Process(u32),
    /// A thread of this process, by its thread id.
    Thread(u32),
}

/// Why a wait ended without an answer: the work's own budget, or the machine's ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// The processor budget was spent: the image's refusal.
    Spent,
    /// The wall-clock ceiling passed first, after this long: the machine's refusal.
    Ceiling {
        /// The wall-clock length of the wait.
        after: Duration,
    },
}

/// A worker's request ends as the worker's refusals say it: killed for its budget, or stalled.
impl From<Ended> for SandboxError {
    fn from(ended: Ended) -> Self {
        match ended {
            Ended::Spent => Self::TimedOut,
            Ended::Ceiling { after } => Self::Stalled { after },
        }
    }
}

/// The allowance one request is waited for under.
#[derive(Debug)]
pub struct Allowance {
    /// The worker or the thread whose processor time is read, and its reading when the request
    /// began; `None` where it cannot be read, and the wall clock is then the whole bound.
    reading: Option<(Whose, Duration)>,
    /// The processor time the request may spend.
    processor: Duration,
    /// The wall-clock length of the wait, for the refusal's sentence.
    wall: Duration,
    /// When the wait ends whatever the worker has spent.
    wall_deadline: Instant,
}

impl Allowance {
    /// Starts the allowance of a request sent now to the worker `pid`, which may spend
    /// `processor` of processor time on it.
    #[must_use]
    pub fn start(pid: u32, processor: Duration) -> Self {
        let whose = Whose::Process(pid);
        let reading = processor_time(whose).map(|at_start| (whose, at_start));
        Self::since(reading, processor, Instant::now())
    }

    /// The allowance of a request that `began` at that instant, whose decoder and its processor
    /// time then are `reading` — `None` where it could not be read, and the wall clock is then the
    /// whole bound, `processor` long.
    pub(crate) fn since(
        reading: Option<(Whose, Duration)>,
        processor: Duration,
        began: Instant,
    ) -> Self {
        let wall = if reading.is_some() {
            processor.saturating_mul(STALL_FACTOR)
        } else {
            processor
        };
        Self {
            reading,
            processor,
            wall,
            // `checked_add` because an `Instant` that cannot represent the ceiling is not
            // reachable on a real clock; if it were, the safe reading is "already out of time",
            // not "never".
            wall_deadline: began.checked_add(wall).unwrap_or(began),
        }
    }

    /// How long the parent may wait for the worker's next bytes before it looks again, or the
    /// refusal that ends the request.
    ///
    /// **The wait is the processor time left, not a polling interval.** The worker is one thread
    /// — its filter refuses `clone` (`lockdown_linux.rs`) — so it cannot spend more processor time
    /// than the wall time that passes, and a wait as long as what is left of its budget cannot
    /// overshoot it; an idle parent wakes once per budget it could have been spent in, not on a
    /// timer. The floor is one tick of the clock `/proc` counts in, below which a reading cannot
    /// have moved and a shorter wait would only spin.
    ///
    /// # Errors
    ///
    /// [`Ended`], saying which bound ended the request.
    pub fn next_wait(&self) -> Result<Duration, Ended> {
        let wall_left = self.wall_deadline.saturating_duration_since(Instant::now());
        if wall_left.is_zero() {
            return Err(Ended::Ceiling { after: self.wall });
        }
        let Some((whose, at_start)) = self.reading else {
            return Ok(wall_left);
        };
        // A worker whose reading is gone mid-request has ended; the pipe says how, so the wait
        // goes on to the ceiling and the read that follows it reports the death.
        let Some(now) = processor_time(whose) else {
            return Ok(wall_left);
        };
        let spent = now.saturating_sub(at_start);
        if spent >= self.processor {
            return Err(Ended::Spent);
        }
        let processor_left = self.processor.saturating_sub(spent).max(TICK);
        Ok(processor_left.min(wall_left))
    }
}

/// One tick of the clock `/proc/<pid>/stat` counts processor time in: `USER_HZ`, which Linux
/// fixes at a hundred a second for every architecture this tree builds for.
const TICK: Duration = Duration::from_millis(10);

/// Whether this platform's `/proc` is the one `proc(5)` describes, which every reading below
/// assumes; elsewhere nothing is read, and the wall clock is the bound.
///
/// A constant rather than `#[cfg]` on each reader, so that the readers and every variant of
/// [`Whose`] are built and read on every target, and checked there, while no other platform's
/// `/proc`, whose `stat` has another layout, is ever parsed as Linux's.
const PROC_IS_LINUX: bool = cfg!(target_os = "linux");

/// The processor time `whose` has spent, user and system together, from its `stat` file under
/// `/proc`; `None` where it cannot be read.
///
/// Fields 14 and 15 (`utime`, `stime`) in `proc(5)`'s numbering, counted after the closing
/// parenthesis of field 2, the command name, which may itself hold spaces and parentheses. A
/// thread's `/proc/self/task/<tid>/stat` has the same fields, counted for that thread alone.
fn processor_time(whose: Whose) -> Option<Duration> {
    if !PROC_IS_LINUX {
        return None;
    }
    let path = match whose {
        Whose::Process(pid) => format!("/proc/{pid}/stat"),
        Whose::Thread(tid) => format!("/proc/self/task/{tid}/stat"),
    };
    stat_processor_time(&std::fs::read_to_string(path).ok()?)
}

/// `utime + stime` of one `stat` file's text.
fn stat_processor_time(stat: &str) -> Option<Duration> {
    let (_, after_name) = stat.rsplit_once(')')?;
    // `after_name` begins with field 3, the state, so field 14 is the twelfth after it.
    let mut fields = after_name.split_ascii_whitespace().skip(11);
    let user: u64 = fields.next()?.parse().ok()?;
    let system: u64 = fields.next()?.parse().ok()?;
    let ticks = user.checked_add(system)?;
    Some(TICK.saturating_mul(u32::try_from(ticks).unwrap_or(u32::MAX)))
}

/// The calling thread's id, from the `/proc/thread-self` link (`<pid>/task/<tid>`): the name its
/// processor time is read under from another thread, found without the `gettid` call a
/// `#![forbid(unsafe_code)]` crate cannot make (ADR 3150). `None` where `/proc` is not Linux's,
/// not there, or not readable.
pub(crate) fn this_thread() -> Option<Whose> {
    if !PROC_IS_LINUX {
        return None;
    }
    let link = std::fs::read_link("/proc/thread-self").ok()?;
    let tid = link.file_name()?.to_str()?.parse().ok()?;
    Some(Whose::Thread(tid))
}

/// The calling thread's own processor time, read as [`processor_time`] reads it from elsewhere.
pub(crate) fn this_threads_time() -> Option<Duration> {
    if !PROC_IS_LINUX {
        return None;
    }
    stat_processor_time(&std::fs::read_to_string("/proc/thread-self/stat").ok()?)
}

#[cfg(test)]
mod tests {
    use super::{Allowance, Ended, Whose, processor_time};
    use std::time::{Duration, Instant};

    /// Spins until this process's reading reaches `at_least`, and returns it; a wall-clock
    /// minute is a ceiling only on a reading that never moves, which is the failure.
    #[cfg(target_os = "linux")]
    fn spin_until(at_least: Duration) -> Duration {
        let pid = std::process::id();
        let started = Instant::now();
        let mut sum = 0_u64;
        loop {
            let now = processor_time(Whose::Process(pid))
                .expect("this process's own /proc/<pid>/stat is readable");
            if now >= at_least {
                return now;
            }
            assert!(
                started.elapsed() < Duration::from_mins(1),
                "a minute of busy work did not move the reading from {now:?}"
            );
            for _ in 0..1_000_000 {
                sum = std::hint::black_box(sum.wrapping_add(1));
            }
        }
    }

    /// This process's own reading is there, and it grows with work done: the order of two
    /// readings with work between them, and not a count of seconds, which a loaded machine
    /// stretches (ADR 3133).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_process_s_own_processor_time_is_read_and_grows_with_its_work() {
        let before = processor_time(Whose::Process(std::process::id())).expect("readable");
        let after = spin_until(before.saturating_add(super::TICK));
        assert!(after > before, "{before:?} to {after:?}");
    }

    /// A thread names itself, and another thread reads its processor time under that name and
    /// sees it grow with the thread's work — the in-process path's reading (ADR 3150).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_thread_is_named_and_its_processor_time_is_read_from_another() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, mpsc};

        let stop = Arc::new(AtomicBool::new(false));
        let (named, name) = mpsc::channel();
        let spinner = std::thread::spawn({
            let stop = Arc::clone(&stop);
            move || {
                named
                    .send(super::this_thread().expect("/proc/thread-self names this thread"))
                    .expect("the test is listening");
                let mut sum = 0_u64;
                while !stop.load(Ordering::Relaxed) {
                    sum = std::hint::black_box(sum.wrapping_add(1));
                }
                super::this_threads_time().expect("a thread reads its own time")
            }
        });
        let whose = name.recv().expect("the thread names itself");
        let Whose::Thread(tid) = whose else {
            panic!("a thread is named as a thread: {whose:?}");
        };
        assert_ne!(
            tid,
            std::process::id(),
            "a spawned thread is not the main thread"
        );
        // Read from this thread until the spinner's reading moves: an order of readings, with a
        // minute of wall clock as a ceiling only on a reading that never moves.
        let started = Instant::now();
        let seen = loop {
            let now = processor_time(whose).expect("a live thread's reading is there");
            if now >= super::TICK {
                break now;
            }
            assert!(
                started.elapsed() < Duration::from_mins(1),
                "a minute of the thread's busy work did not move its reading from {now:?}"
            );
            std::thread::yield_now();
        };
        stop.store(true, Ordering::Relaxed);
        let own = spinner.join().expect("the spinning thread returns");
        assert!(
            own >= seen,
            "its own reading {own:?} is at least what was read of it, {seen:?}"
        );
        // A joined thread's reading goes, but not at the join: the kernel wakes the joiner when it
        // clears the thread's id word, early in the thread's exit, and removes its task directory
        // only when the exit is done, so for a moment after `join` the reading is still there
        // (ADR 3181). An order again — the reading is gone at some point after the join — with a
        // minute of wall clock as a ceiling only on a directory that never goes.
        let joined = Instant::now();
        while let Some(still) = processor_time(whose) {
            assert!(
                joined.elapsed() < Duration::from_mins(1),
                "a minute after the join the ended thread's reading is still {still:?}"
            );
            std::thread::yield_now();
        }
    }

    /// A process that is gone has no reading, and an allowance for it is the wall clock alone.
    #[test]
    fn a_process_with_no_reading_is_held_to_the_wall_clock_alone() {
        // Past `/proc/sys/kernel/pid_max`'s largest value, so no process has it.
        let pid = 1 << 23;
        assert_eq!(processor_time(Whose::Process(pid)), None);
        let allowance = Allowance::start(pid, Duration::from_secs(30));
        assert_eq!(allowance.wall, Duration::from_secs(30));
        let wait = allowance.next_wait().expect("nothing is spent yet");
        assert!(wait <= Duration::from_secs(30), "{wait:?}");
    }

    /// A request whose worker has spent its processor budget is refused as the image's, however
    /// little wall clock has passed: this process is made to spend a tick first.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_request_that_spent_its_processor_budget_is_timed_out() {
        spin_until(super::TICK);
        let pid = std::process::id();
        let mut allowance = Allowance::start(pid, Duration::from_millis(1));
        assert_eq!(
            allowance.wall,
            Duration::from_millis(1).saturating_mul(super::STALL_FACTOR)
        );
        allowance.reading = Some((Whose::Process(pid), Duration::ZERO));
        allowance.wall_deadline = Instant::now() + Duration::from_secs(3600);
        let outcome = allowance.next_wait();
        assert!(
            matches!(outcome, Err(Ended::Spent)),
            "the budget is spent, so the refusal is the image's: {outcome:?}"
        );
    }

    /// A request whose wall ceiling has passed is refused as the machine's, whatever its worker
    /// spent.
    #[test]
    fn a_request_past_its_wall_ceiling_is_stalled() {
        let mut allowance = Allowance::start(std::process::id(), Duration::from_secs(30));
        allowance.wall_deadline = Instant::now();
        let outcome = allowance.next_wait();
        assert!(
            matches!(outcome, Err(Ended::Ceiling { .. })),
            "the ceiling has passed, so the refusal is the machine's: {outcome:?}"
        );
    }
}
