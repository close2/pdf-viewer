//! The one copy of a sound the helper holds: its samples as they arrive, read by each pass from
//! where the pass starts (ADR 3457).
//!
//! The window sends a sound a run at a time, as the media worker decodes it, and the helper starts
//! the first pass at the first run; the device's thread therefore reads a sound that may still be
//! growing. A repeat, a resumption and the first pass all read this one copy, so a sound is held
//! once however many passes it is played as.
//!
//! The samples are a list of shared runs behind a lock that is held only to add a run or to copy
//! the list's new entries: the device's thread never waits on a run being read from the pipe, only
//! on the push of a pointer, and it keeps its own copy of the list so that it takes the lock only
//! when it has played every run it already knew of.

use std::sync::{Arc, Mutex, PoisonError};

/// A sound's samples, interleaved at the device's format, shared by the helper's order loop, which
/// adds to them, and each pass's reader.
#[derive(Clone, Debug, Default)]
pub struct Held(Arc<Mutex<Runs>>);

/// The runs that have arrived, and whether the last has.
#[derive(Debug, Default)]
struct Runs {
    list: Vec<Arc<[f32]>>,
    len: usize,
    complete: bool,
}

impl Held {
    /// An empty sound, its samples still to come.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `samples` added at the sound's end.
    pub fn append(&self, samples: Vec<f32>) {
        if samples.is_empty() {
            return;
        }
        let mut runs = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        runs.len = runs.len.saturating_add(samples.len());
        runs.list.push(samples.into());
    }

    /// Every sample has arrived.
    pub fn complete(&self) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .complete = true;
    }

    /// How many samples have arrived.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).len
    }

    /// Whether no sample has arrived.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether every sample has arrived.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .complete
    }

    /// The samples from `from` up to, not including, `end` that have arrived, copied: what a
    /// device that plays a pass at once — a test's recording double — takes.
    #[must_use]
    pub fn copied(&self, from: usize, end: usize) -> Vec<f32> {
        let mut reader = self.reader(from, end);
        let mut out = vec![0.0; end.min(self.len()).saturating_sub(from)];
        let read = reader.read(&mut out);
        out.truncate(read.copied);
        out
    }

    /// A reader of the samples from `from` up to, not including, `end` — or to the sound's end,
    /// where that comes first.
    #[must_use]
    pub fn reader(&self, from: usize, end: usize) -> Reader {
        Reader {
            held: self.clone(),
            known: Vec::new(),
            run: 0,
            offset: 0,
            skip: from,
            at: from,
            end: end.max(from),
            complete: false,
        }
    }
}

/// One pass's place in a [`Held`] sound.
#[derive(Debug)]
pub struct Reader {
    held: Held,
    /// The runs this reader has seen, a copy of the list's pointers.
    known: Vec<Arc<[f32]>>,
    /// The run being read, an index into `known`.
    run: usize,
    /// The next sample's place in that run.
    offset: usize,
    /// Samples still to pass over to reach where the pass starts.
    skip: usize,
    /// The sample of the sound the next copy starts at.
    at: usize,
    /// The sample the pass ends before.
    end: usize,
    /// Whether the list was complete when it was last copied.
    complete: bool,
}

/// What one [`Reader::read`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Read {
    /// How many samples were copied, from the start of the slice handed over.
    pub copied: usize,
    /// Whether the pass is over: its end, or the sound's last sample, has been read.
    pub over: bool,
}

impl Reader {
    /// Copies the pass's next samples into `out`, as many as have arrived and the pass holds.
    pub fn read(&mut self, out: &mut [f32]) -> Read {
        let mut copied = 0;
        while copied < out.len() && self.at < self.end {
            let Some(run) = self.next_run() else {
                break;
            };
            let available = run.len().saturating_sub(self.offset);
            let wanted = out
                .len()
                .saturating_sub(copied)
                .min(self.end.saturating_sub(self.at))
                .min(available);
            if let (Some(to), Some(from)) = (
                out.get_mut(copied..copied.saturating_add(wanted)),
                run.get(self.offset..self.offset.saturating_add(wanted)),
            ) {
                to.copy_from_slice(from);
            }
            copied = copied.saturating_add(wanted);
            self.offset = self.offset.saturating_add(wanted);
            self.at = self.at.saturating_add(wanted);
        }
        let over = self.at >= self.end || (self.complete && self.exhausted());
        Read { copied, over }
    }

    /// The sample of the sound the next copy starts at.
    #[must_use]
    pub fn position(&self) -> usize {
        self.at
    }

    /// The run holding the next sample, with the pass's start passed over; `None` where no
    /// sample past those copied has arrived yet.
    fn next_run(&mut self) -> Option<Arc<[f32]>> {
        loop {
            if self.exhausted() {
                self.refresh();
                if self.exhausted() {
                    return None;
                }
            }
            let run = self.known.get(self.run).map(Arc::clone)?;
            if self.offset >= run.len() {
                self.run = self.run.saturating_add(1);
                self.offset = 0;
                continue;
            }
            if self.skip > 0 {
                let passed = self.skip.min(run.len().saturating_sub(self.offset));
                self.skip = self.skip.saturating_sub(passed);
                self.offset = self.offset.saturating_add(passed);
                continue;
            }
            return Some(run);
        }
    }

    /// Whether every run this reader knows of has been read.
    fn exhausted(&self) -> bool {
        match self.known.get(self.run) {
            None => true,
            Some(run) => self.offset >= run.len() && self.run.saturating_add(1) >= self.known.len(),
        }
    }

    /// The runs that arrived since the last look, and whether the sound is complete.
    fn refresh(&mut self) {
        let runs = self.held.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(new) = runs.list.get(self.known.len()..) {
            self.known.extend(new.iter().map(Arc::clone));
        }
        self.complete = runs.complete;
    }
}

#[cfg(test)]
mod tests {
    use super::{Held, Read};

    /// A pass reads what has arrived, waits at the edge without ending, and ends only once the
    /// sound is complete; a second pass over the same copy reads from its own start.
    #[test]
    fn a_pass_reads_a_growing_sound_and_ends_at_its_end() {
        let held = Held::new();
        held.append(vec![1.0, 2.0, 3.0]);
        let mut reader = held.reader(1, usize::MAX);
        let mut out = [0.0; 4];
        assert_eq!(
            reader.read(&mut out),
            Read {
                copied: 2,
                over: false
            }
        );
        assert_eq!(&out[..2], &[2.0, 3.0]);
        held.append(vec![4.0]);
        held.complete();
        assert_eq!(
            reader.read(&mut out),
            Read {
                copied: 1,
                over: true
            }
        );
        assert_eq!(out[0], 4.0);
        let mut again = held.reader(0, 2);
        assert_eq!(
            again.read(&mut out),
            Read {
                copied: 2,
                over: true
            }
        );
        assert_eq!(&out[..2], &[1.0, 2.0]);
        assert_eq!(held.len(), 4);
    }

    /// A start past several runs passes over them, and an empty run is never added.
    #[test]
    fn a_start_inside_a_later_run_is_found() {
        let held = Held::new();
        held.append(vec![0.0, 1.0]);
        held.append(Vec::new());
        held.append(vec![2.0, 3.0]);
        held.append(vec![4.0, 5.0]);
        held.complete();
        let mut reader = held.reader(3, 5);
        let mut out = [0.0; 8];
        assert_eq!(
            reader.read(&mut out),
            Read {
                copied: 2,
                over: true
            }
        );
        assert_eq!(&out[..2], &[3.0, 4.0]);
        assert_eq!(reader.position(), 5);
    }
}
