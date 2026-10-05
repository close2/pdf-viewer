//! Whether a sweep over boxes is sure to pass its budget, counted before it sorts (ADR 1517).

use super::Box2;

/// Boxes fewer than this are swept without being counted first: their sort costs less than
/// the count would.
const COUNT_FIRST: usize = 4_096;

/// Whether [`meeting_pairs`](super::meeting_pairs)' sweep along `along` is sure to pass
/// `budget` comparisons, decided without the sort it begins with (ADR 1517).
///
/// **What the sweep counts.** Taking the boxes in order of where they start along the axis,
/// it compares each with every earlier box that has not ended before it starts, so its count
/// is the number of pairs whose later box starts before the earlier one ends: all
/// `n (n − 1) / 2` pairs, less the pairs where one box ends before the other starts. That
/// second number is bounded above without sorting: cut the axis into `n` equal buckets, and a
/// box that ends before another starts ends in that box's bucket or an earlier one, since
/// rounding never reverses an order. Counting, for every box, the boxes ending in its starting
/// bucket or before overcounts by the pairs that share a bucket and no more. So where all
/// pairs, less that count, already pass `budget`, the sweep would pass it too — and it is
/// answered `false` as the sweep would answer it, without the sort that took a page's clip of
/// a hundred thousand edges 72 M instructions to learn it.
#[expect(clippy::cast_precision_loss)] // a box count as a scale: any positive one keeps order
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a bucket, saturated
#[expect(clippy::arithmetic_side_effects)] // counts of boxes, below `u32::MAX` each
pub(super) fn surely_past(
    boxes: &[Box2],
    along: usize,
    (lo, hi): (f32, f32),
    budget: usize,
) -> bool {
    let count = boxes.len();
    let span = hi - lo;
    if count < COUNT_FIRST || !(span > 0.0 && span.is_finite()) {
        return false;
    }
    let scale = count as f32 / span;
    let bucket = |x: f32| (((x - lo) * scale) as usize).min(count - 1);
    let mut ended = vec![0_u64; count];
    for b in boxes {
        // A box with no position along the axis is one the sweep orders by its bits alone,
        // which this count does not model: the sweep answers it.
        if b[along].is_nan() || b[along + 2].is_nan() {
            return false;
        }
        ended[bucket(b[along + 2])] += 1;
    }
    let mut running = 0_u64;
    for slot in &mut ended {
        running += *slot;
        *slot = running;
    }
    let apart: u64 = boxes.iter().map(|b| ended[bucket(b[along])]).sum();
    let count = count as u64;
    let pairs = count * (count - 1) / 2;
    pairs.saturating_sub(apart) > u64::try_from(budget).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[expect(clippy::arithmetic_side_effects, clippy::cast_precision_loss)] // test arithmetic
mod tests {
    use super::{COUNT_FIRST, surely_past};
    use crate::raster::fill::topology::Box2;

    /// What the sweep counts, the long way: boxes in order of where they start, each against
    /// every earlier box that has not ended before it starts.
    fn swept(boxes: &[Box2], along: usize) -> u64 {
        let mut order: Vec<usize> = (0..boxes.len()).collect();
        order.sort_by(|&a, &b| boxes[a][along].total_cmp(&boxes[b][along]).then(a.cmp(&b)));
        let mut tests = 0_u64;
        for (at, &k) in order.iter().enumerate() {
            let start = boxes[k][along];
            tests += order[..at]
                .iter()
                .filter(|&&m| boxes[m][along + 2] >= start)
                .count() as u64;
        }
        tests
    }

    /// Boxes of a curve flattened into short pieces, wound back and forth `turns` times across
    /// a span of `width` — the shape of the clip this count exists for.
    fn wiggle(count: usize, width: f32, turns: f32) -> Vec<Box2> {
        (0..count)
            .map(|i| {
                let t = i as f32 / count as f32;
                let x = |t: f32| width * (0.5 + 0.5 * (t * turns * std::f32::consts::TAU).sin());
                let (a, b) = (x(t), x(t + 1.0 / count as f32));
                [a.min(b), t * 9.0, a.max(b), t * 9.0 + 0.01]
            })
            .collect()
    }

    /// **Never sure where the sweep would not pass**: over crowded and sparse boxes, at
    /// budgets either side of the sweep's own count, `surely_past` says yes only where that
    /// count passes the budget — and it does say yes where the boxes crowd.
    #[test]
    fn a_sweep_is_counted_past_its_budget_only_where_it_passes_it() {
        let mut said_yes = 0;
        for (count, turns) in [
            (COUNT_FIRST, 3.0),
            (COUNT_FIRST * 2, 40.0),
            (COUNT_FIRST, 0.5),
        ] {
            let boxes = wiggle(count, 200.0, turns);
            let (lo, hi) = boxes.iter().fold((f32::MAX, f32::MIN), |(lo, hi), b| {
                (lo.min(b[0]), hi.max(b[2]))
            });
            let tests = swept(&boxes, 0);
            for budget in [tests / 4, tests / 2, tests - 1, tests, tests * 2] {
                let budget = usize::try_from(budget).unwrap_or(usize::MAX);
                if surely_past(&boxes, 0, (lo, hi), budget) {
                    said_yes += 1;
                    assert!(tests > budget as u64, "{tests} tests against {budget}");
                }
            }
        }
        assert!(said_yes > 0, "a crowded sweep is counted past its budget");
    }

    /// Fewer boxes than [`COUNT_FIRST`] are left to the sweep.
    #[test]
    fn a_few_boxes_are_left_to_the_sweep() {
        let boxes = vec![[0.0, 0.0, 1.0, 1.0]; COUNT_FIRST - 1];
        assert!(!surely_past(&boxes, 0, (0.0, 1.0), 0));
    }
}
