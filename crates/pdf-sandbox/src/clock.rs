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

/// The allowance one request is waited for under.
#[derive(Debug)]
pub(crate) struct Allowance {
    /// The worker, whose processor time is read.
    pid: u32,
    /// The processor time the request may spend.
    processor: Duration,
    /// The worker's processor time when the request was sent; `None` where it cannot be read,
    /// and the wall clock is then the whole bound.
    processor_at_start: Option<Duration>,
    /// The wall-clock length of the wait, for the refusal's sentence.
    wall: Duration,
    /// When the wait ends whatever the worker has spent.
    wall_deadline: Instant,
}

impl Allowance {
    /// Starts the allowance of a request sent now to the worker `pid`, which may spend
    /// `processor` of processor time on it.
    pub(crate) fn start(pid: u32, processor: Duration) -> Self {
        let processor_at_start = processor_time(pid);
        let wall = if processor_at_start.is_some() {
            processor.saturating_mul(STALL_FACTOR)
        } else {
            processor
        };
        let now = Instant::now();
        Self {
            pid,
            processor,
            processor_at_start,
            wall,
            // `checked_add` because an `Instant` that cannot represent the ceiling is not
            // reachable on a real clock; if it were, the safe reading is "already out of time",
            // not "never".
            wall_deadline: now.checked_add(wall).unwrap_or(now),
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
    pub(crate) fn next_wait(&self) -> Result<Duration, SandboxError> {
        let wall_left = self.wall_deadline.saturating_duration_since(Instant::now());
        if wall_left.is_zero() {
            return Err(SandboxError::Stalled { after: self.wall });
        }
        let Some(at_start) = self.processor_at_start else {
            return Ok(wall_left);
        };
        // A worker whose reading is gone mid-request has ended; the pipe says how, so the wait
        // goes on to the ceiling and the read that follows it reports the death.
        let Some(now) = processor_time(self.pid) else {
            return Ok(wall_left);
        };
        let spent = now.saturating_sub(at_start);
        if spent >= self.processor {
            return Err(SandboxError::TimedOut);
        }
        let processor_left = self.processor.saturating_sub(spent).max(TICK);
        Ok(processor_left.min(wall_left))
    }
}

/// One tick of the clock `/proc/<pid>/stat` counts processor time in: `USER_HZ`, which Linux
/// fixes at a hundred a second for every architecture this tree builds for.
const TICK: Duration = Duration::from_millis(10);

/// The processor time process `pid` has spent, user and system together, from
/// `/proc/<pid>/stat`; `None` where it cannot be read.
///
/// Fields 14 and 15 (`utime`, `stime`) in `proc(5)`'s numbering, counted after the closing
/// parenthesis of field 2, the command name, which may itself hold spaces and parentheses.
#[cfg(target_os = "linux")]
fn processor_time(pid: u32) -> Option<Duration> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, after_name) = stat.rsplit_once(')')?;
    // `after_name` begins with field 3, the state, so field 14 is the twelfth after it.
    let mut fields = after_name.split_ascii_whitespace().skip(11);
    let user: u64 = fields.next()?.parse().ok()?;
    let system: u64 = fields.next()?.parse().ok()?;
    let ticks = user.checked_add(system)?;
    Some(TICK.saturating_mul(u32::try_from(ticks).unwrap_or(u32::MAX)))
}

/// No platform but Linux has a `/proc` this reads, so elsewhere the wall clock is the bound.
#[cfg(not(target_os = "linux"))]
fn processor_time(_pid: u32) -> Option<Duration> {
    None
}

#[cfg(test)]
mod tests {
    use super::{Allowance, STALL_FACTOR, processor_time};
    use crate::SandboxError;
    use std::time::{Duration, Instant};

    /// Spins until this process's reading reaches `at_least`, and returns it; a wall-clock
    /// minute is a ceiling only on a reading that never moves, which is the failure.
    #[cfg(target_os = "linux")]
    fn spin_until(at_least: Duration) -> Duration {
        let pid = std::process::id();
        let started = Instant::now();
        let mut sum = 0_u64;
        loop {
            let now = processor_time(pid).expect("this process's own /proc/<pid>/stat is readable");
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
        let before = processor_time(std::process::id()).expect("readable");
        let after = spin_until(before.saturating_add(super::TICK));
        assert!(after > before, "{before:?} to {after:?}");
    }

    /// A process that is gone has no reading, and an allowance for it is the wall clock alone.
    #[test]
    fn a_process_with_no_reading_is_held_to_the_wall_clock_alone() {
        // Past `/proc/sys/kernel/pid_max`'s largest value, so no process has it.
        let pid = 1 << 23;
        assert_eq!(processor_time(pid), None);
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
            Duration::from_millis(1).saturating_mul(STALL_FACTOR)
        );
        allowance.processor_at_start = Some(Duration::ZERO);
        allowance.wall_deadline = Instant::now() + Duration::from_secs(3600);
        let outcome = allowance.next_wait();
        assert!(
            matches!(outcome, Err(SandboxError::TimedOut)),
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
            matches!(outcome, Err(SandboxError::Stalled { .. })),
            "the ceiling has passed, so the refusal is the machine's: {outcome:?}"
        );
    }
}
