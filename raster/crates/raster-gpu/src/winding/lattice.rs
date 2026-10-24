//! The rank-1 lattice of each sample count, and the two widths that follow from it in closed
//! form (the viewer's ADR 2296).
//!
//! **The lattice is the reference the lane's pattern is derived against, and its bound**
//! (the viewer's ADR 2313): [`SamplePattern::of`] draws with the pattern a seeded search
//! finds from this lattice and holds to no width wider than the lattice's, and with the lattice
//! itself at a count no search was run for. What follows is the lattice's own argument, which
//! stands.
//!
//! A **rank-1 lattice**: `n` samples, the i-th at `(i + ½)/n` across a pixel
//! and `((g·i mod n) + ½)/n` down it, for one generator `g` coprime to `n`. Every sample has a
//! column and a row of its own, so an axis-aligned edge crosses them one at a time and reads one
//! of `n + 1` levels. The ordered `√n × √n` grid this replaced put `√n` samples in each column,
//! so the same edge read one of `√n + 1` — five at the default sixteen, which is the
//! non-conformance raster's ADR 0076 recorded and the viewer's ADR 2286 measured on glyph
//! pages at 4x.
//!
//! # What the lattice costs ADR 0070's condition
//!
//! ADR 0070 keeps a mark on the processor lane when it could miss every sample. On the ordered
//! grid one number answered that for every shape the condition reads, `1/√n`, because the grid
//! is a product of a column lattice and a row lattice: a box at least that wide and that tall
//! holds a column and a row, and so the sample where they cross. A rank-1 lattice is not a
//! product, and the question splits in two, each answered here from the lattice itself:
//!
//! - [`SampleLattice::widest_empty_band`]: the widest infinitely long band, at any angle, that
//!   holds no sample — what a stroke's own width is compared with. For a lattice it is the
//!   spacing of the lattice lines along its shortest vector, `det / λ₁`, which is `1/√m` where
//!   `m` is that vector's squared length in units of `1/n`.
//! - [`SampleLattice::widest_empty_square`]: the widest axis-aligned square that holds none —
//!   what a mark's device box is compared with.
//!
//! [`SampleLattice::vanishing_width`] is the larger of the two, and it is wider than the
//! ordered grid's `1/√n` at every admitted count: `1/√10 ≈ 0.316` at sixteen samples against
//! `0.25`. No lattice with `n` distinct columns and rows can reach `1/√n` for squares: squares of
//! that side would have to tile the plane, one to a sample, and a lattice of squares tiles only
//! with a vector along a side — two samples in one row or one column. That is the price of the
//! `n + 1` levels, and ADR 2296 counts what it costs in marks.
//!
//! What gets better is the axis-aligned band, which is most of what a document draws thin:
//! with `n` distinct columns a band `1/n` wide holds one in every pixel it spans, so every pixel
//! along it receives a sample — the "every column" guarantee at a quarter of the width the
//! ordered grid needed.

use super::pattern::{LARGEST_COUNT, SamplePattern};

/// The rank-1 lattice of one sample count: `n` samples on the lattice of generator `g`.
///
/// Built by [`SampleLattice::of`], which chooses the generator, so a count names exactly one
/// pattern and two adapters place their samples identically (ADR 0006's promise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleLattice {
    /// How many samples a pixel takes.
    count: u32,
    /// The lattice's generator: sample `i` sits in row `g·i mod n`.
    generator: u32,
}

impl SampleLattice {
    /// The lattice of `count` samples, clamped to `1..=64` as the device clamps it.
    ///
    /// **The generator is derived, not chosen**: of the `g` coprime to `n` — the ones that give
    /// every sample a row of its own — the one whose lattice can hide the narrowest mark, its
    /// [`vanishing_width`](Self::vanishing_width), because that width is what ADR 0070's
    /// condition sends to the processor lane; on a tie the smallest `g`. At sixteen samples `g`
    /// is 3, whose shortest vector is `(1, 3)/16`; 5, 11 and 13 tie with it and are its mirror
    /// images.
    ///
    /// Choosing it costs `O(n³)` steps — a quarter of a million at the largest count the device
    /// admits — so each count's generator is found once in the process's life and kept, and a
    /// frame asks for it twice.
    #[must_use]
    pub fn of(count: u32) -> Self {
        let count = count.clamp(1, LARGEST_COUNT);
        let generator = match GENERATORS.get(count as usize) {
            Some(kept) => *kept.get_or_init(|| best_generator(count)),
            None => best_generator(count),
        };
        Self { count, generator }
    }

    /// How many samples a pixel takes.
    #[must_use]
    pub fn count(self) -> u32 {
        self.count
    }

    /// The lattice's generator: sample `i` sits in row `g·i mod n`.
    #[must_use]
    pub fn generator(self) -> u32 {
        self.generator
    }

    /// The row sample `index` sits in, `g·index mod n`.
    fn row(self, index: u32) -> u32 {
        lattice_row(self.generator, index, self.count)
    }

    /// The lattice as a [`SamplePattern`]: sample `i` in column `i` and row `g·i mod n`.
    ///
    /// What the lane draws with at a count no search has been run for, and where
    /// `examples/sample_pattern.rs`'s search starts (the viewer's ADR 2313).
    #[must_use]
    pub fn pattern(self) -> SamplePattern {
        SamplePattern::of_rank_1_lattice(self.count, |index| self.row(index))
    }

    /// The distance between neighbouring sample columns, and between neighbouring rows: `1/n`.
    ///
    /// An axis-aligned edge's coverage is a multiple of this, and an axis-aligned band at least
    /// this wide holds a column — and so a sample — in every pixel it spans.
    #[must_use]
    pub fn edge_pitch(self) -> f32 {
        #[expect(clippy::cast_precision_loss)] // a sample count clamped to 4..=64 by the device
        let count = self.count as f32;
        1.0 / count
    }

    /// The widest band, infinitely long and at any angle, that can lie between the samples.
    ///
    /// The lattice's points lie on lines parallel to any of its vectors, spaced `det/|v|`
    /// apart, and the widest gap is along the shortest vector `λ₁`. With `n` points to a unit
    /// pixel `det` is `1/n`, and with `λ₁ = √m / n` the gap is `1/√m`. A band wider than this
    /// holds a sample wherever it lies and at whatever angle.
    #[must_use]
    pub fn widest_empty_band(self) -> f32 {
        #[expect(clippy::cast_precision_loss)] // at most 2·(n/2)², far below 2^24
        let length = shortest_vector_squared(self.count, self.generator) as f32;
        1.0 / length.sqrt()
    }

    /// The widest axis-aligned square that can lie between the samples, open on every side.
    ///
    /// A largest empty square touches a sample on its left side and one on its bottom; moved by
    /// the lattice vector that takes the first to the origin, its lower-left corner is `(0, y₀)`
    /// for a lattice row `y₀`, so `n` corners decide it. From each, the square grows until it
    /// meets the first sample above and to the right of the corner. Counted in units of `1/n`
    /// on the lattice's integer points; the half-sample offset of [`SamplePattern::offsets`]
    /// moves every point alike and changes no distance.
    #[must_use]
    pub fn widest_empty_square(self) -> f32 {
        let n = self.count;
        let widest = (0..n)
            .map(|bottom| {
                // The samples with x in 1..=n and y in bottom+1..=bottom+n: the copies one pixel
                // up are what a square near the pixel's top edge meets.
                (1..=n)
                    .flat_map(|column| {
                        let row = self.row(column);
                        [(column, row), (column, row.saturating_add(n))]
                    })
                    .filter(|&(_, row)| row > bottom)
                    .map(|(column, row)| column.max(row.saturating_sub(bottom)))
                    .min()
                    .unwrap_or(n)
            })
            .max()
            .unwrap_or(n);
        #[expect(clippy::cast_precision_loss)] // both at most 2n, far below 2^24
        let (widest, n) = (widest as f32, n as f32);
        widest / n
    }

    /// The narrowest mark the lane can promise not to lose wherever it lands: the larger of
    /// [`widest_empty_band`](Self::widest_empty_band) and
    /// [`widest_empty_square`](Self::widest_empty_square).
    ///
    /// ADR 0070's condition compares a mark's thin axis with this — the narrower side of its
    /// device box, or a stroke's own width where that is narrower — so it must hold for both
    /// readings: a box with both sides at least this wide holds a sample, and so does a band of
    /// this width at any angle.
    #[must_use]
    pub fn vanishing_width(self) -> f32 {
        self.widest_empty_band().max(self.widest_empty_square())
    }
}

/// The generator each sample count up to [`LARGEST_COUNT`] takes, found on first use.
static GENERATORS: [std::sync::OnceLock<u32>; LARGEST_COUNT as usize + 1] =
    [const { std::sync::OnceLock::new() }; LARGEST_COUNT as usize + 1];

/// The generator of [`SampleLattice::of`]: the `g` coprime to `n` whose lattice has the
/// smallest [`vanishing_width`](SampleLattice::vanishing_width), the smallest such `g` on a tie.
fn best_generator(count: u32) -> u32 {
    (1..count.max(2))
        .filter(|&generator| greatest_common_divisor(generator, count) == 1)
        .map(|generator| SampleLattice { count, generator })
        .fold(None, |best: Option<(SampleLattice, f32)>, lattice| {
            let width = lattice.vanishing_width();
            match best {
                Some((_, narrowest)) if narrowest <= width => best,
                _ => Some((lattice, width)),
            }
        })
        .map_or(1, |(lattice, _)| lattice.generator)
}

/// The squared length, in units of `1/n`, of the shortest non-zero vector of the rank-1
/// lattice of generator `g` with `n` points: the least `x² + y²` over the vectors `i·(1, g)/n`
/// reduced to the pixel around the origin.
fn shortest_vector_squared(n: u32, g: u32) -> u32 {
    (1..n)
        .map(|index| {
            let row = lattice_row(g, index, n);
            // Each coordinate reduced to the copy nearest the origin, which is at most n/2.
            let x = index.min(n.saturating_sub(index));
            let y = row.min(n.saturating_sub(row));
            x.saturating_mul(x).saturating_add(y.saturating_mul(y))
        })
        .min()
        // One sample: the lattice is the integer grid itself, whose shortest vector is a pixel.
        .unwrap_or_else(|| n.saturating_mul(n))
}

/// Euclid's algorithm, for the generators that give every sample a row of its own.
fn greatest_common_divisor(mut a: u32, mut b: u32) -> u32 {
    while let Some(remainder) = a.checked_rem(b) {
        (a, b) = (b, remainder);
    }
    a
}

/// The row of sample `index` on the lattice of `generator` with `count` samples:
/// `generator · index mod count`.
fn lattice_row(generator: u32, index: u32, count: u32) -> u32 {
    // The product of two `u32`s fits a `u64` and a remainder by `count` fits back in a `u32`,
    // so neither step can fail; `count` is at least one wherever a lattice is built.
    u64::from(generator)
        .saturating_mul(u64::from(index))
        .checked_rem(u64::from(count))
        .and_then(|row| u32::try_from(row).ok())
        .unwrap_or(0)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)] // test-file policy as in `winding.rs`: integer fixtures far below every type's range
mod tests {
    use super::SampleLattice;

    /// The counts the device admits: a square, clamped to `4..=64` (`device/construct.rs`).
    const ADMITTED: [u32; 7] = [4, 9, 16, 25, 36, 49, 64];

    /// The pattern's samples in units of `1/(2n)`, so that every coordinate is an odd integer.
    fn doubled(lattice: SampleLattice) -> Vec<(i64, i64)> {
        let n = f64::from(lattice.count());
        lattice
            .pattern()
            .offsets()
            .iter()
            .map(|&[x, y]| {
                let scale = |v: f32| ((f64::from(v) + 0.5) * 2.0 * n).round() as i64;
                (scale(x), scale(y))
            })
            .collect()
    }

    /// Every sample has a column and a row of its own, which is what gives an axis-aligned
    /// edge `n + 1` levels.
    #[test]
    fn every_sample_has_a_column_and_a_row_of_its_own() {
        for count in ADMITTED {
            let samples = doubled(SampleLattice::of(count));
            let mut columns: Vec<_> = samples.iter().map(|s| s.0).collect();
            let mut rows: Vec<_> = samples.iter().map(|s| s.1).collect();
            columns.sort_unstable();
            rows.sort_unstable();
            let expected: Vec<i64> = (0..i64::from(count)).map(|k| 2 * k + 1).collect();
            assert_eq!(columns, expected, "{count} samples, one to a column");
            assert_eq!(rows, expected, "{count} samples, one to a row");
        }
    }

    /// The generators the rule derives, written out because ADR 2296 quotes the default's.
    #[test]
    fn the_default_lattice_is_generated_by_three() {
        let lattice = SampleLattice::of(16);
        assert_eq!(lattice.generator(), 3);
        assert!((lattice.widest_empty_band() - 1.0 / 10.0_f32.sqrt()).abs() <= f32::EPSILON);
        assert!((lattice.widest_empty_square() - 5.0 / 16.0).abs() <= f32::EPSILON);
        assert!((lattice.edge_pitch() - 1.0 / 16.0).abs() <= f32::EPSILON);
    }

    /// The widest empty band, re-derived by brute force over the directions of every integer
    /// vector up to the pixel's width: the largest gap between the samples' projections on
    /// the direction's normal, periodic because the pixel lattice projects to multiples of
    /// `1/|v|`. Held against the closed form `1/√m` at every admitted count.
    #[test]
    fn the_widest_empty_band_is_the_lattice_line_spacing_by_brute_force() {
        for count in ADMITTED {
            let lattice = SampleLattice::of(count);
            let samples = doubled(lattice);
            let period = 2 * i64::from(count);
            let mut widest = 0.0_f64;
            let limit = i64::from(count);
            for p in 0..=limit {
                for q in -limit..=limit {
                    if (p, q) == (0, 0) || num_gcd(p, q.abs()) != 1 || (p == 0 && q < 0) {
                        continue;
                    }
                    let mut projections: Vec<i64> = samples
                        .iter()
                        .map(|&(x, y)| (-q * x + p * y).rem_euclid(period))
                        .collect();
                    projections.sort_unstable();
                    projections.dedup();
                    let wrap = projections[0] + period - projections[projections.len() - 1];
                    let gap = projections
                        .windows(2)
                        .map(|w| w[1] - w[0])
                        .fold(wrap, i64::max);
                    let width = gap as f64 / period as f64 / ((p * p + q * q) as f64).sqrt();
                    widest = widest.max(width);
                }
            }
            let closed = f64::from(lattice.widest_empty_band());
            assert!(
                (widest - closed).abs() < 1e-6,
                "{count} samples: brute force {widest}, closed form {closed}"
            );
        }
    }

    /// The widest empty square, re-derived by brute force over every corner a sample's column
    /// and another sample's row define, against the samples of the pixel and its neighbours.
    #[test]
    fn the_widest_empty_square_by_brute_force_over_every_corner() {
        for count in ADMITTED {
            let lattice = SampleLattice::of(count);
            let samples = doubled(lattice);
            let period = 2 * i64::from(count);
            let tiled: Vec<(i64, i64)> = samples
                .iter()
                .flat_map(|&(x, y)| {
                    (0..3).flat_map(move |a| (0..3).map(move |b| (x + a * period, y + b * period)))
                })
                .collect();
            let widest = samples
                .iter()
                .flat_map(|&(x0, _)| samples.iter().map(move |&(_, y0)| (x0, y0)))
                .map(|(x0, y0)| {
                    tiled
                        .iter()
                        .filter(|&&(x, y)| x > x0 && y > y0)
                        .map(|&(x, y)| (x - x0).max(y - y0))
                        .min()
                        .expect("the neighbouring pixels hold samples above and right")
                })
                .max()
                .expect("a lattice has samples");
            let brute = widest as f64 / period as f64;
            let closed = f64::from(lattice.widest_empty_square());
            assert!(
                (brute - closed).abs() < 1e-6,
                "{count} samples: brute force {brute}, closed form {closed}"
            );
        }
    }

    /// The price, stated: at every admitted count the lattice can hide a wider mark than the
    /// ordered grid's `1/√n` could, and never more than half again as wide.
    #[test]
    fn the_vanishing_width_is_wider_than_the_ordered_grids_at_every_count() {
        for count in ADMITTED {
            let ordered = 1.0 / f64::from(count).sqrt();
            let width = f64::from(SampleLattice::of(count).vanishing_width());
            assert!(
                width > ordered && width <= 1.5 * ordered,
                "{count} samples: {width} against the ordered grid's {ordered}"
            );
        }
    }

    fn num_gcd(a: i64, b: i64) -> i64 {
        if b == 0 { a } else { num_gcd(b, a % b) }
    }
}
