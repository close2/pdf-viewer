//! The graphics device's clock level through a timed frame, read off the driver's own table of
//! levels (ADR 2402, trap 274).
//!
//! **A GPU-bound figure is a claim about a clock as well as a tree.** amdgpu publishes the levels
//! the graphics engine's clock may run at, one line each, in `pp_dpm_sclk`, and stars the one it
//! runs at now. This machine's Radeon 890M draws a child's frames at its lowest level, 600 MHz, or at
//! a raised clock, mostly 1 100 MHz and above, for minutes at a time either way and at a busy share
//! the device check admits, and a zoom step that is the device's own work costs three times as much
//! at the lowest (ADRs 2388, 2402). A figure that does not say which it ran at cannot be told from a
//! regression.
//!
//! [`Sampler`] reads the starred line every [`CADENCE`] on a thread of its own, from before a
//! round's first frame to after its last; [`Residency::during`] keeps the readings inside one timed
//! frame's span and counts them by level. An integrated device's table is three lines — the lowest
//! level, the clock it runs at now where that lies between the ends, and the highest — so a level
//! is counted with the lowest and highest megahertz it was read at, which is a range only for that
//! middle line.
//!
//! Where no device offers the file (another driver, a machine with no graphics device, CI) the
//! sampler does not start, every residency is empty, and the line that prints it says so.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How often [`Sampler`] reads the table: within a few readings of a two-millisecond step, and one
/// read costs about 16 µs of one core, under one percent of it.
pub(crate) const CADENCE: Duration = Duration::from_millis(2);

/// The levels a [`Residency`] keeps apart: amdgpu's tables run to eight lines on the older discrete
/// devices and to three on the newer ones and on an integrated one. A line past the last is counted
/// in [`Residency::beyond`], so it is reported rather than lost.
pub(crate) const LEVELS: usize = 8;

/// One reading of the table: which line was starred, at how many megahertz, and when.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Reading {
    at: Instant,
    level: usize,
    mhz: u32,
}

/// The first device in name order whose driver publishes `pp_dpm_sclk`, or `None`.
///
/// One device is read: this machine has one, and the gate draws on the adapter wgpu picks, which
/// on a machine of several is not named by the card's number. Where several offer the file the
/// device line of `turn_path` says how many it saw.
pub(crate) fn table() -> Option<PathBuf> {
    tables().into_iter().next()
}

/// Every device's `pp_dpm_sclk`, in name order.
pub(crate) fn tables() -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("device/pp_dpm_sclk"))
        // A card and its render node are two entries over one device: the table is named by the
        // device's own path, so it is counted once.
        .filter_map(|path| std::fs::canonicalize(path).ok())
        .filter(|path| path.is_file())
        .collect();
    found.sort();
    found.dedup();
    found
}

/// The starred line of one read of `table`: its level and its megahertz, or `None` where the
/// table does not read, and the sampler keeps no reading for it.
fn starred(table: &std::path::Path) -> Option<(usize, u32)> {
    starred_in(&std::fs::read_to_string(table).ok()?)
}

/// The starred line of a table's text: a line is `<level>: <mhz>Mhz` with a trailing `*` on the
/// one the clock runs at, and a table with no starred line gives `None`.
fn starred_in(text: &str) -> Option<(usize, u32)> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_suffix('*')?;
        let (level, clock) = rest.split_once(':')?;
        let mhz = clock.trim().trim_end_matches("Mhz").trim_end_matches("MHz");
        Some((level.trim().parse().ok()?, mhz.trim().parse().ok()?))
    })
}

/// A thread reading the table every [`CADENCE`] until [`Sampler::stop`].
pub(crate) struct Sampler {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<Vec<Reading>>,
}

impl Sampler {
    /// Starts reading [`table`]'s device, or returns `None` where there is none to read.
    pub(crate) fn start() -> Option<Self> {
        let table = table()?;
        starred(&table)?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut readings = Vec::new();
            while !stopping.load(Ordering::Relaxed) {
                if let Some((level, mhz)) = starred(&table) {
                    readings.push(Reading {
                        at: Instant::now(),
                        level,
                        mhz,
                    });
                }
                // A cadence, not a wait for an event: the clock is a level the driver changes on
                // its own, and the gap is what spreads the readings across a frame (ADR 1800).
                std::thread::sleep(CADENCE);
            }
            readings
        });
        Some(Self { stop, thread })
    }

    /// Stops the thread and hands back every reading it took, in order.
    pub(crate) fn stop(self) -> Vec<Reading> {
        self.stop.store(true, Ordering::Relaxed);
        // A sampler that panicked took no readings anyone can trust, and an empty residency
        // prints as unread rather than as a level.
        self.thread.join().unwrap_or_default()
    }
}

/// The readings one figure's frames hold, counted by level, with the megahertz each level was read
/// at.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Residency {
    /// Readings at each level, by the table's own line number.
    pub(crate) samples: [u32; LEVELS],
    /// The lowest megahertz each level was read at; meaningful where `samples` is not zero.
    pub(crate) low_mhz: [u32; LEVELS],
    /// The highest.
    pub(crate) high_mhz: [u32; LEVELS],
    /// Readings at a line past [`LEVELS`].
    pub(crate) beyond: u32,
}

impl Residency {
    /// The readings that fall inside `span`, counted.
    pub(crate) fn during(readings: &[Reading], (began, ended): (Instant, Instant)) -> Self {
        let mut residency = Self::default();
        for reading in readings
            .iter()
            .filter(|reading| reading.at >= began && reading.at <= ended)
        {
            residency.count(reading.level, reading.mhz);
        }
        residency
    }

    /// Counts one reading at `level`, read at `mhz`.
    fn count(&mut self, level: usize, mhz: u32) {
        if level >= LEVELS {
            self.beyond = self.beyond.saturating_add(1);
            return;
        }
        if self.samples[level] == 0 {
            (self.low_mhz[level], self.high_mhz[level]) = (mhz, mhz);
        } else {
            self.low_mhz[level] = self.low_mhz[level].min(mhz);
            self.high_mhz[level] = self.high_mhz[level].max(mhz);
        }
        self.samples[level] = self.samples[level].saturating_add(1);
    }

    /// Both residencies' readings together: the figure of several rounds is read over all of their
    /// frames, because one frame of a millisecond or two holds a reading or none.
    pub(crate) fn and(self, other: Self) -> Self {
        let mut both = self;
        for level in 0..LEVELS {
            if other.samples[level] == 0 {
                continue;
            }
            if both.samples[level] == 0 {
                (both.low_mhz[level], both.high_mhz[level]) =
                    (other.low_mhz[level], other.high_mhz[level]);
            } else {
                both.low_mhz[level] = both.low_mhz[level].min(other.low_mhz[level]);
                both.high_mhz[level] = both.high_mhz[level].max(other.high_mhz[level]);
            }
            both.samples[level] = both.samples[level].saturating_add(other.samples[level]);
        }
        both.beyond = both.beyond.saturating_add(other.beyond);
        both
    }

    /// Every reading this residency holds.
    pub(crate) fn readings(&self) -> u32 {
        self.samples
            .iter()
            .fold(self.beyond, |sum, samples| sum.saturating_add(*samples))
    }
}

/// `600 MHz 83%, 1005..1364 MHz 17% (18 readings)`, or `unread` where nothing was read.
impl std::fmt::Display for Residency {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total = self.readings();
        if total == 0 {
            return write!(out, "sclk unread");
        }
        let share = |samples: u32| f64::from(samples) * 100.0 / f64::from(total);
        let mut first = true;
        write!(out, "sclk ")?;
        for level in 0..LEVELS {
            let (samples, low, high) = (
                self.samples[level],
                self.low_mhz[level],
                self.high_mhz[level],
            );
            if samples == 0 {
                continue;
            }
            if !first {
                write!(out, ", ")?;
            }
            first = false;
            if low == high {
                write!(out, "{low} MHz {:.0}%", share(samples))?;
            } else {
                write!(out, "{low}..{high} MHz {:.0}%", share(samples))?;
            }
        }
        if self.beyond > 0 {
            write!(out, ", past line {LEVELS} {:.0}%", share(self.beyond))?;
        }
        write!(out, " ({total} readings)")
    }
}

#[cfg(test)]
mod tests {
    use super::{LEVELS, Reading, Residency, starred_in};
    use std::time::{Duration, Instant};

    /// The two shapes the 890M's table was read in on 2026-10-26: at its lowest level, with the
    /// middle line a fixed placeholder, and between its ends, with the middle line the clock now.
    #[test]
    fn the_starred_line_is_the_level_and_its_megahertz() {
        assert_eq!(
            starred_in("0: 600Mhz *\n1: 1100Mhz \n2: 2900Mhz \n"),
            Some((0, 600))
        );
        assert_eq!(
            starred_in("0: 600Mhz \n1: 1005Mhz *\n2: 2900Mhz \n"),
            Some((1, 1005))
        );
        assert_eq!(starred_in("0: 600Mhz \n1: 1100Mhz \n"), None);
    }

    /// A frame keeps the readings inside its span, and two frames' residencies add up level by
    /// level with the megahertz range of each.
    #[test]
    fn a_frame_keeps_its_own_readings_and_rounds_add_up() {
        let began = Instant::now();
        let at = |ms: u64, level: usize, mhz: u32| Reading {
            at: began + Duration::from_millis(ms),
            level,
            mhz,
        };
        let readings = [at(0, 0, 600), at(2, 1, 1005), at(4, 1, 1364), at(6, 0, 600)];
        let frame = (
            began + Duration::from_millis(1),
            began + Duration::from_millis(5),
        );
        let one = Residency::during(&readings, frame);
        assert_eq!(one.samples[..2], [0, 2]);
        assert_eq!((one.low_mhz[1], one.high_mhz[1]), (1005, 1364));
        let both = one.and(Residency::during(&readings, (began, began)));
        assert_eq!(both.samples[..2], [1, 2]);
        assert_eq!(both.readings(), 3);
        assert_eq!(
            both.to_string(),
            "sclk 600 MHz 33%, 1005..1364 MHz 67% (3 readings)"
        );
        let mut past = Residency::default();
        past.count(LEVELS, 3000);
        assert_eq!(past.beyond, 1);
    }
}
