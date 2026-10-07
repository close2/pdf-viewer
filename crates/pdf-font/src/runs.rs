//! Metrics keyed by a code or a CID, held as the runs of consecutive keys they are stated in.
//!
//! ISO 32000-2 §9.7.4.3 states a composite font's widths in two forms, and the second is one
//! statement about a whole range:
//!
//! > The second format shall define the same width, w , as a number, for all CIDs in the range
//! > c first to c last.
//!
//! A range is twelve bytes of the file and may name four billion CIDs, so a table with one entry
//! per CID has a size the producer chooses rather than the size of what the producer wrote —
//! `CLAUDE.md` principle 3's resource exhaustion, and a cost the parser's own budgets cannot
//! see because the array that asks for it is small. Held as runs, a table costs at most two runs
//! per statement and one value per number the array holds, so it is bounded by the array the
//! parser already read and needs no budget of its own. ADR 1596.
//!
//! A lookup is a binary search over the runs, which are disjoint and sorted by their first key.

use std::collections::BTreeMap;

/// One stretch of consecutive keys, all answered from `values`.
#[derive(Debug, Clone, Copy)]
struct Run {
    /// The first key the run answers.
    first: u32,
    /// The last key the run answers, inclusive.
    last: u32,
    /// Where in `values` the first key's value is.
    at: usize,
    /// Whether each key has its own value (`c [w1 w2 …]`, read from `at` onwards) or every
    /// key shares the one at `at` (`cfirst clast w`).
    consecutive: bool,
}

/// A key's value, for every key a statement covers — answered without expanding a range.
#[derive(Debug, Clone)]
pub(crate) struct Runs<T> {
    /// Disjoint, sorted by `first`.
    runs: Vec<Run>,
    values: Vec<T>,
}

impl<T> Default for Runs<T> {
    fn default() -> Self {
        Self {
            runs: Vec::new(),
            values: Vec::new(),
        }
    }
}

impl<T: Copy> Runs<T> {
    /// The value stated for `key`, or `None` where no statement covers it.
    pub(crate) fn get(&self, key: u32) -> Option<T> {
        let after = self.runs.partition_point(|run| run.first <= key);
        let run = self.runs.get(after.checked_sub(1)?)?;
        if key > run.last {
            return None;
        }
        let index = if run.consecutive {
            run.at
                .checked_add(usize::try_from(key.checked_sub(run.first)?).ok()?)?
        } else {
            run.at
        };
        self.values.get(index).copied()
    }

    /// Whether no key has a value.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// How many runs hold the table: what it costs, where [`Self::iter`] is what it says.
    #[cfg(test)]
    pub(crate) fn run_count(&self) -> usize {
        self.runs.len()
    }

    /// Every key with a value, in ascending order — one item per key, produced as it is asked
    /// for, so the caller pays for a range's length only by walking it.
    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (u32, T)> + '_ {
        self.runs
            .iter()
            .flat_map(|run| (run.first..=run.last).filter_map(|key| Some((key, self.get(key)?))))
    }

    /// The table a map states, one run for each stretch of consecutive keys in it.
    ///
    /// For the tables that are built one key at a time — a simple font's `/Widths` and what
    /// §9.6.2.2's metrics and a substitute add to it — whose size is already the number of keys.
    pub(crate) fn from_map(map: &BTreeMap<u32, T>) -> Self {
        let mut runs: Vec<Run> = Vec::new();
        let mut values = Vec::with_capacity(map.len());
        for (&key, &value) in map {
            let at = values.len();
            values.push(value);
            match runs.last_mut() {
                Some(run) if run.last.checked_add(1) == Some(key) => run.last = key,
                _ => runs.push(Run {
                    first: key,
                    last: key,
                    at,
                    consecutive: true,
                }),
            }
        }
        Self { runs, values }
    }
}

/// Collects statements in the order an array makes them, and keeps the first for any key two of
/// them name.
///
/// ISO 32000-2 §9.7.4.3: "Specifying a given CID value more than once should not be done. In the
/// case where it is done, the first specification is the one that shall be used." So each
/// statement claims only the keys no earlier statement covered, and `covered` — the union of
/// everything claimed so far, as merged disjoint intervals — is what says which those are. A
/// statement's walk over `covered` merges every interval it meets into one, so the work over a
/// whole array is its statement count times a logarithm, whatever the statements overlap.
#[derive(Debug)]
pub(crate) struct RunsBuilder<T> {
    /// Merged, disjoint, non-adjacent intervals of claimed keys: first key → last key.
    covered: BTreeMap<u32, u32>,
    runs: Vec<Run>,
    values: Vec<T>,
}

impl<T> Default for RunsBuilder<T> {
    fn default() -> Self {
        Self {
            covered: BTreeMap::new(),
            runs: Vec::new(),
            values: Vec::new(),
        }
    }
}

impl<T: Copy> RunsBuilder<T> {
    /// `cfirst clast w`: one value for every key from `first` to `last`. A range whose last key
    /// is below its first names no key.
    pub(crate) fn range(&mut self, first: u32, last: u32, value: T) {
        if last < first {
            return;
        }
        let at = self.values.len();
        self.values.push(value);
        self.claim(first, last, at, false);
    }

    /// `c [w1 w2 …]`: one value each for consecutive keys from `first`. A `None` is an element
    /// that states no value; its key is left to a later statement, and the keys after it keep
    /// their places. Values that would fall past the last representable key name nothing.
    pub(crate) fn list(&mut self, first: u32, values: impl IntoIterator<Item = Option<T>>) {
        let mut segment: Option<(u32, usize)> = None;
        let mut key = Some(first);
        for value in values {
            let Some(current) = key else {
                break;
            };
            match (value, segment) {
                (Some(value), None) => {
                    segment = Some((current, self.values.len()));
                    self.values.push(value);
                }
                (Some(value), Some(_)) => self.values.push(value),
                (None, Some((start, at))) => {
                    // The segment ends at the key before this one, and `current > start`
                    // because `start` was claimed by a value.
                    if let Some(previous) = current.checked_sub(1) {
                        self.claim(start, previous, at, true);
                    }
                    segment = None;
                }
                (None, None) => {}
            }
            key = current.checked_add(1);
        }
        if let Some((start, at)) = segment {
            let last = key.map_or(u32::MAX, |next| next.saturating_sub(1));
            self.claim(start, last, at, true);
        }
    }

    /// The table, its runs sorted for the search.
    pub(crate) fn finish(mut self) -> Runs<T> {
        self.runs.sort_unstable_by_key(|run| run.first);
        Runs {
            runs: self.runs,
            values: self.values,
        }
    }

    /// Gives the keys in `first..=last` that no earlier statement covered to the run starting at
    /// `at`, and marks the whole span covered.
    fn claim(&mut self, first: u32, last: u32, at: usize, consecutive: bool) {
        // Every claimed interval this span overlaps or touches. Intervals are disjoint and
        // sorted, so walking down from the last one that starts at or before `last + 1` meets
        // them in descending order and the first one ending before `first - 1` ends the walk.
        let reach_high = u64::from(last).saturating_add(1);
        let reach_low = u64::from(first).saturating_sub(1);
        let met: Vec<(u32, u32)> = self
            .covered
            .range(..=u32::try_from(reach_high).unwrap_or(u32::MAX))
            .rev()
            .take_while(|&(_, &end)| u64::from(end) >= reach_low)
            .map(|(&start, &end)| (start, end))
            .collect();

        // The gaps between them, inside the span, in ascending order. `u64` so that a claimed
        // interval ending at `u32::MAX` moves the cursor past every key rather than wrapping.
        let mut cursor = u64::from(first);
        let end = u64::from(last);
        for &(start, stop) in met.iter().rev() {
            let start = u64::from(start);
            if start > cursor && cursor <= end {
                self.push_run(
                    first,
                    cursor,
                    start.saturating_sub(1).min(end),
                    at,
                    consecutive,
                );
            }
            cursor = cursor.max(u64::from(stop).saturating_add(1));
        }
        if cursor <= end {
            self.push_run(first, cursor, end, at, consecutive);
        }

        let mut merged = (first, last);
        for (start, stop) in met {
            self.covered.remove(&start);
            merged = (merged.0.min(start), merged.1.max(stop));
        }
        self.covered.insert(merged.0, merged.1);
    }

    /// One run for the keys `low..=high` of a statement that began at `first`.
    fn push_run(&mut self, first: u32, low: u64, high: u64, at: usize, consecutive: bool) {
        // Both are keys inside `first..=last`, which are `u32`s, so neither conversion fails.
        let (Ok(low), Ok(high)) = (u32::try_from(low), u32::try_from(high)) else {
            return;
        };
        let at = if consecutive {
            let offset = low.checked_sub(first).and_then(|o| usize::try_from(o).ok());
            match offset.and_then(|offset| at.checked_add(offset)) {
                Some(at) => at,
                None => return,
            }
        } else {
            at
        };
        self.runs.push(Run {
            first: low,
            last: high,
            at,
            consecutive,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{Runs, RunsBuilder};

    fn built(statements: impl FnOnce(&mut RunsBuilder<u32>)) -> Runs<u32> {
        let mut builder = RunsBuilder::default();
        statements(&mut builder);
        builder.finish()
    }

    /// §9.7.4.3's EXAMPLE 1: `/W [120 [400 325 500] 7080 8032 1000]`.
    #[test]
    fn the_clauses_example_is_answered_cid_by_cid() {
        let runs = built(|b| {
            b.list(120, [Some(400), Some(325), Some(500)]);
            b.range(7080, 8032, 1000);
        });
        assert_eq!(runs.get(119), None);
        assert_eq!(runs.get(120), Some(400));
        assert_eq!(runs.get(121), Some(325));
        assert_eq!(runs.get(122), Some(500));
        assert_eq!(runs.get(123), None);
        assert_eq!(runs.get(7079), None);
        assert_eq!(runs.get(7080), Some(1000));
        assert_eq!(runs.get(8032), Some(1000));
        assert_eq!(runs.get(8033), None);
        assert_eq!(runs.run_count(), 2);
    }

    /// "the first specification is the one that shall be used", whichever form either is in.
    #[test]
    fn the_first_statement_of_a_cid_wins() {
        let runs = built(|b| {
            b.range(10, 20, 1);
            b.list(5, (0..20).map(|n| Some(100 + n)));
            b.range(0, 100, 2);
        });
        assert_eq!(runs.get(4), Some(2));
        assert_eq!(runs.get(5), Some(100));
        assert_eq!(runs.get(9), Some(104));
        assert_eq!(runs.get(10), Some(1));
        assert_eq!(runs.get(20), Some(1));
        assert_eq!(runs.get(21), Some(116));
        assert_eq!(runs.get(24), Some(119));
        assert_eq!(runs.get(25), Some(2));
        assert_eq!(runs.get(100), Some(2));
        assert_eq!(runs.get(101), None);
    }

    /// An element that is not a number states nothing for its CID and moves the rest along.
    #[test]
    fn a_hole_in_a_list_leaves_its_cid_to_a_later_statement() {
        let runs = built(|b| {
            b.list(0, [Some(1), None, Some(3)]);
            b.range(0, 5, 9);
        });
        assert_eq!(runs.get(0), Some(1));
        assert_eq!(runs.get(1), Some(9));
        assert_eq!(runs.get(2), Some(3));
        assert_eq!(runs.get(3), Some(9));
    }

    #[test]
    fn a_backward_range_names_no_cid() {
        let runs = built(|b| b.range(10, 5, 1));
        assert!(runs.is_empty());
    }

    /// The whole key space in one statement, and statements at its top edge, cost a run each.
    #[test]
    fn a_range_over_every_cid_is_one_run() {
        let runs = built(|b| {
            b.range(u32::MAX - 1, u32::MAX, 7);
            b.range(0, u32::MAX, 1);
            b.list(u32::MAX, [Some(5), Some(6)]);
        });
        assert_eq!(runs.get(0), Some(1));
        assert_eq!(runs.get(u32::MAX - 2), Some(1));
        assert_eq!(runs.get(u32::MAX), Some(7));
        assert_eq!(runs.run_count(), 2);
    }

    /// Many overlapping statements stay bounded by twice their count.
    #[test]
    fn interleaved_statements_cost_at_most_two_runs_each() {
        let runs = built(|b| {
            for n in 0..1000u32 {
                b.range(n * 10, n * 10 + 4, n);
            }
            for n in 0..1000u32 {
                b.range(0, 65_535, n);
            }
        });
        assert!(runs.run_count() <= 4000);
        assert_eq!(runs.get(3), Some(0));
        assert_eq!(runs.get(7), Some(0));
        assert_eq!(runs.get(9_994), Some(999));
        assert_eq!(runs.get(65_535), Some(0));
    }

    #[test]
    fn a_map_becomes_one_run_per_stretch() {
        let map = [(1, 10), (2, 20), (3, 30), (7, 70)].into_iter().collect();
        let runs = Runs::from_map(&map);
        assert_eq!(runs.run_count(), 2);
        assert_eq!(
            runs.iter().collect::<Vec<_>>(),
            vec![(1, 10), (2, 20), (3, 30), (7, 70)]
        );
        assert_eq!(runs.get(4), None);
    }
}
