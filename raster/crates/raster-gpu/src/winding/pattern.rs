//! Where the [`Coverage::Gpu`](crate::Coverage::Gpu) lane puts its samples, and the two widths
//! and the edge error that follow from where it puts them, for any pattern of the lane's shape
//! (the viewer's ADR 2312).
//!
//! # The shape every pattern has
//!
//! `n` samples, the i-th at `(i + ½)/n` across a pixel and `(rows[i] + ½)/n` down it, where
//! `rows` is a permutation of `0..n`: every sample has a column and a row of its own, so an
//! axis-aligned edge crosses them one at a time and reads one of `n + 1` levels, and an
//! axis-aligned band `1/n` wide holds a sample in every pixel it spans. The pattern repeats in
//! every pixel. A rank-1 lattice ([`SampleLattice`]) is the permutation `rows[i] = g·i mod n`;
//! it is one pattern of this shape among `n!`.
//!
//! # The two widths, for any such pattern
//!
//! ADR 0070 keeps a mark on the processor lane when it could miss every sample. The viewer's
//! ADR 2296 split that question in two for the lattice; both halves are asked here of the
//! pattern itself, by a finite computation that needs no lattice structure:
//!
//! - [`SamplePattern::widest_empty_band`]: the widest infinitely long band, at any angle, that
//!   holds no sample. A band along a direction with an irrational slope meets the pattern's
//!   copies densely and is never empty for any width, so only the directions of integer
//!   vectors `(p, q)` count. Along one of them, the plane's copies of the pattern project onto
//!   the band's normal with period `1/|(p, q)|`, so the widest empty band in that direction is
//!   the widest gap between the `n` samples' projections taken modulo the period — at most
//!   `1/|(p, q)|`. The horizontal direction already gives `1/n`, so no vector longer than `n`
//!   can beat it and the search over directions is finite.
//! - [`SamplePattern::widest_empty_square`]: the widest axis-aligned square, open on every
//!   side, that holds none. Slid left and then down until it touches a sample on each of those
//!   two sides, a largest one has its corner at one sample's column and another's row — `n²`
//!   corners — and from each the square grows until it meets the first sample above and to the
//!   right of it.
//!
//! [`SamplePattern::vanishing_width`] is the larger of the two, and ADR 0070's argument is the
//! same sentence for every pattern: a mark whose thin axis reaches it holds a sample under
//! either reading, a box whose two sides both reach it and a band of that width at any angle.
//!
//! # The error the pattern gives a straight edge
//!
//! [`SamplePattern::straight_edge_error`] is how far a pixel crossed by one straight edge can
//! read from its area: over every direction and every placement of the edge across the pixel,
//! the gap between the share of samples behind the edge and the share of the pixel behind it.
//! For one direction this is exact — between two samples' projections the count is constant and
//! the area rises, so the gap is largest at a projection, from one side or the other — and the
//! directions are taken at a stated count of equal angles.

use crate::winding::SampleLattice;

/// The most samples a pattern holds: the device's own clamp (`device/construct.rs`).
pub(super) const LARGEST_COUNT: u32 = 64;

/// The counts the device admits: a square, clamped to `4..=64` (`device/construct.rs`), each
/// with the pattern `examples/sample_pattern.rs` derives for it.
///
/// **Data a program re-derives, not a table chosen by hand** (the viewer's ADR 2313). Each row
/// is what that example's seeded search finds, starting from the count's [`SampleLattice`] and
/// held to the lattice's two widths: the pattern with the smallest
/// [`straight_edge_error`](SamplePattern::straight_edge_error) the search reaches, whose
/// [`widest_empty_band`](SamplePattern::widest_empty_band) and
/// [`widest_empty_square`](SamplePattern::widest_empty_square) are each no wider than the
/// lattice's. `sample_pattern --check` runs the search again and fails on any row that differs,
/// and this module's tests re-derive both widths and the constraint by brute force.
const SEARCHED: [(u32, &[u8]); 7] = [
    (4, &[2, 0, 3, 1]),
    (9, &[2, 5, 0, 7, 4, 1, 6, 3, 8]),
    (16, &[5, 15, 10, 2, 7, 12, 1, 4, 11, 8, 14, 3, 6, 13, 9, 0]),
    (
        25,
        &[
            0, 4, 9, 15, 13, 19, 24, 3, 7, 10, 16, 22, 20, 1, 5, 11, 14, 18, 23, 2, 8, 6, 12, 17,
            21,
        ],
    ),
    (
        36,
        &[
            1, 4, 11, 16, 21, 25, 30, 34, 5, 9, 14, 19, 24, 28, 35, 2, 8, 13, 18, 23, 29, 32, 3, 7,
            12, 17, 22, 26, 33, 0, 6, 10, 15, 20, 27, 31,
        ],
    ),
    (
        49,
        &[
            0, 5, 13, 19, 25, 31, 36, 43, 47, 6, 11, 17, 23, 29, 35, 40, 48, 4, 10, 16, 21, 28, 34,
            41, 46, 2, 9, 15, 22, 27, 33, 39, 45, 3, 8, 14, 20, 26, 32, 38, 44, 1, 7, 12, 18, 24,
            30, 37, 42,
        ],
    ),
    (
        64,
        &[
            1, 7, 14, 22, 27, 36, 43, 49, 56, 62, 5, 13, 20, 28, 34, 41, 48, 55, 63, 6, 12, 19, 25,
            32, 39, 47, 54, 61, 4, 11, 18, 26, 33, 40, 46, 53, 60, 2, 10, 17, 24, 31, 37, 45, 52,
            58, 3, 8, 15, 23, 30, 38, 44, 51, 59, 0, 9, 16, 21, 29, 35, 42, 50, 57,
        ],
    ),
];

/// One sample pattern: `count` samples, sample `i` in column `i` and row `rows[i]`.
///
/// Built by [`SamplePattern::of`] for the pattern the lane draws with, or by
/// [`SamplePattern::from_rows`] for any other of the same shape — which is how the search and
/// the tests measure the ones it did not choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplePattern {
    /// How many samples a pixel takes, `1..=64`.
    count: u32,
    /// Sample `i`'s row, for `i < count`; the rest is zero.
    rows: [u8; LARGEST_COUNT as usize],
}

/// The largest gap between a pixel's area behind a straight edge and its samples' share
/// behind it, over the directions [`SamplePattern::straight_edge_error`] was asked at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeError {
    /// The largest, over every direction and every placement: the error the pattern promises.
    pub worst: f64,
    /// Each direction's largest, averaged over the directions: what the search breaks a tie in
    /// `worst` by.
    pub mean: f64,
}

impl SamplePattern {
    /// The pattern the device lane draws `count` samples with, `count` clamped to `1..=64` as
    /// the device clamps it.
    ///
    /// At a count the device admits, the searched pattern of `SEARCHED`; at any other —
    /// which only a caller of this type, never the device, can ask for — the count's
    /// [`SampleLattice`], which is the search's starting point and the bound it is held to.
    #[must_use]
    pub fn of(count: u32) -> Self {
        let count = count.clamp(1, LARGEST_COUNT);
        SEARCHED
            .iter()
            .find(|(searched, rows)| *searched == count && !rows.is_empty())
            .and_then(|(_, rows)| Self::from_rows(rows))
            .unwrap_or_else(|| SampleLattice::of(count).pattern())
    }

    /// The pattern whose sample `i` sits in row `rows[i]`, if `rows` is a permutation of
    /// `0..rows.len()` and holds `1..=64` of them — the lane's shape, every sample in a column
    /// and a row of its own.
    #[must_use]
    pub fn from_rows(rows: &[u8]) -> Option<Self> {
        let count = u32::try_from(rows.len()).ok()?;
        if !(1..=LARGEST_COUNT).contains(&count) {
            return None;
        }
        let mut seen = [false; LARGEST_COUNT as usize];
        let mut stored = [0_u8; LARGEST_COUNT as usize];
        for (slot, &row) in stored.iter_mut().zip(rows) {
            let taken = seen
                .get_mut(usize::from(row))
                .filter(|_| u32::from(row) < count)?;
            if *taken {
                return None;
            }
            *taken = true;
            *slot = row;
        }
        Some(Self {
            count,
            rows: stored,
        })
    }

    /// The rank-1 lattice whose sample `i` sits in row `row(i)`, for [`SampleLattice::pattern`].
    ///
    /// Infallible where `from_rows` checks, because the lattice's generator is coprime to the
    /// count (`SampleLattice::of` chooses it among those), so `g·i mod n` is a permutation of
    /// `0..n`; a row that did not fit a byte would read as row 0, and none can, being below 64.
    pub(super) fn of_rank_1_lattice(count: u32, row: impl Fn(u32) -> u32) -> Self {
        let count = count.clamp(1, LARGEST_COUNT);
        let mut rows = [0_u8; LARGEST_COUNT as usize];
        for (slot, index) in rows.iter_mut().zip(0..count) {
            *slot = u8::try_from(row(index)).unwrap_or(0);
        }
        Self { count, rows }
    }

    /// How many samples a pixel takes.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Each sample's row, sample `i`'s at index `i`.
    #[must_use]
    pub fn rows(&self) -> &[u8] {
        self.rows.get(..self.count as usize).unwrap_or(&[])
    }

    /// Every sample, in pixels relative to the pixel's centre: the i-th at `(i + ½)/n − ½`
    /// across and `(rows[i] + ½)/n − ½` down.
    #[must_use]
    pub fn offsets(&self) -> Vec<[f32; 2]> {
        #[expect(clippy::cast_precision_loss)] // a sample count of at most 64
        let step = 1.0 / self.count as f32;
        self.rows()
            .iter()
            .zip(0_u8..)
            .map(|(&row, column)| {
                let (x, y) = (f32::from(column), f32::from(row));
                [(x + 0.5).mul_add(step, -0.5), (y + 0.5).mul_add(step, -0.5)]
            })
            .collect()
    }

    /// The distance between neighbouring sample columns, and between neighbouring rows: `1/n`.
    ///
    /// An axis-aligned edge's coverage is a multiple of this, and an axis-aligned band at least
    /// this wide holds a column — and so a sample — in every pixel it spans.
    #[must_use]
    pub fn edge_pitch(&self) -> f32 {
        #[expect(clippy::cast_precision_loss)] // a sample count of at most 64
        let count = self.count as f32;
        1.0 / count
    }

    /// The widest band, infinitely long and at any angle, that can lie between the samples, in
    /// pixels.
    ///
    /// Over the directions of the primitive integer vectors `(p, q)` no longer than `n`: the
    /// samples' projections on the normal, `p·row − q·column` in units of `1/n`, are taken
    /// modulo `n` — the period the pattern's copies in the other pixels repeat them at, since
    /// `p` and `q` share no factor — and the widest gap between them, divided by `n·|(p, q)|`,
    /// is the widest empty band along that direction. A direction is skipped once
    /// `1/|(p, q)|`, the most it could give, is no more than the widest already found.
    #[must_use]
    #[expect(clippy::arithmetic_side_effects)] // every operand is below 2·64², far from i64's range
    pub fn widest_empty_band(&self) -> f32 {
        let n = i64::from(self.count);
        let mut widest = 1.0 / f64::from(self.count);
        let mut projections: Vec<i64> = Vec::with_capacity(self.count as usize);
        for p in 0..=n {
            for q in -n..=n {
                if (p == 0 && q != 1) || greatest_common_divisor(p, q.abs()) != 1 {
                    continue;
                }
                #[expect(clippy::cast_precision_loss)] // below 2·64², exact in an f64
                let length = ((p * p + q * q) as f64).sqrt();
                if 1.0 / length <= widest {
                    continue;
                }
                projections.clear();
                projections.extend(
                    self.rows()
                        .iter()
                        .zip(0_i64..)
                        .map(|(&row, column)| (p * i64::from(row) - q * column).rem_euclid(n)),
                );
                projections.sort_unstable();
                let wrap = match (projections.first(), projections.last()) {
                    (Some(first), Some(last)) => first + n - last,
                    _ => n,
                };
                let gap = projections
                    .windows(2)
                    .map(|pair| pair[1] - pair[0])
                    .fold(wrap, i64::max);
                #[expect(clippy::cast_precision_loss)] // both below 64, exact in an f64
                let width = gap as f64 / (n as f64 * length);
                widest = widest.max(width);
            }
        }
        #[expect(clippy::cast_possible_truncation)] // a width in 0..=1, kept to f32 as the lane's
        let widest = widest as f32;
        widest
    }

    /// The widest axis-aligned square, open on every side, that can lie between the samples, in
    /// pixels.
    ///
    /// Over the `n²` corners a sample's column and another sample's row define: from each, the
    /// samples `d = 1, 2, …` columns to its right (in this pixel or the next) are met in turn,
    /// each `dy` rows above the corner's row (in this pixel or the next one up, and a whole pixel
    /// where the rows are equal), and the square grows to the least `max(d, dy)`. The walk stops
    /// once `d` reaches that least value, since no later sample can be nearer.
    #[must_use]
    #[expect(clippy::arithmetic_side_effects)] // every operand is below 2·64
    pub fn widest_empty_square(&self) -> f32 {
        let n = self.count as usize;
        let rows = self.rows();
        let mut widest = 0_usize;
        for corner_column in 0..n {
            for &corner_row in rows {
                let mut side = n;
                let mut d = 1;
                while d < side {
                    let row = rows
                        .get((corner_column + d) % n)
                        .copied()
                        .unwrap_or(corner_row);
                    let dy = match (usize::from(row) + n - usize::from(corner_row)) % n {
                        0 => n,
                        dy => dy,
                    };
                    side = side.min(d.max(dy));
                    d += 1;
                }
                widest = widest.max(side);
            }
        }
        #[expect(clippy::cast_precision_loss)] // both at most 64
        let widest = widest as f32 / n as f32;
        widest
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
    pub fn vanishing_width(&self) -> f32 {
        self.widest_empty_band().max(self.widest_empty_square())
    }

    /// How far the pattern's answer to one straight edge across a pixel can be from the area
    /// behind it, over `directions` equal angles of the edge's normal in `[0, π)` (its opposite
    /// gives the complementary answer and the same gap).
    ///
    /// For each direction the samples are sorted by their projection on the normal; at each
    /// projection the area behind an edge through it is set against the share of samples
    /// strictly behind and the share up to and including those on the edge, and the larger gap
    /// is that placement's. Samples within `1e-12` of a pixel are one projection.
    #[must_use]
    #[expect(clippy::arithmetic_side_effects)] // indices below 64
    pub fn straight_edge_error(&self, directions: u32) -> EdgeError {
        let n = f64::from(self.count);
        let samples: Vec<(f64, f64)> = self
            .rows()
            .iter()
            .zip(0_u8..)
            .map(|(&row, column)| ((f64::from(column) + 0.5) / n, (f64::from(row) + 0.5) / n))
            .collect();
        let mut projections = vec![0.0_f64; samples.len()];
        let (mut worst, mut sum) = (0.0_f64, 0.0_f64);
        for step in 0..directions.max(1) {
            let angle = std::f64::consts::PI * f64::from(step) / f64::from(directions.max(1));
            let (nx, ny) = (angle.cos(), angle.sin());
            for (projection, &(x, y)) in projections.iter_mut().zip(&samples) {
                *projection = nx.mul_add(x, ny * y);
            }
            projections.sort_unstable_by(f64::total_cmp);
            let mut largest = 0.0_f64;
            let mut first = 0;
            while let Some(&at) = projections.get(first) {
                let last = projections
                    .iter()
                    .skip(first)
                    .take_while(|&&other| other - at < 1e-12)
                    .count()
                    + first;
                let area = area_behind(nx, ny, at);
                #[expect(clippy::cast_precision_loss)] // counts of at most 64
                let (behind, through) = (first as f64 / n, last as f64 / n);
                largest = largest
                    .max((behind - area).abs())
                    .max((through - area).abs());
                first = last;
            }
            worst = worst.max(largest);
            sum += largest;
        }
        EdgeError {
            worst,
            mean: sum / f64::from(directions.max(1)),
        }
    }
}

/// The share of the unit pixel `[0, 1]²` on the side `nx·x + ny·y < c` of a line whose normal
/// `(nx, ny)` has unit length.
fn area_behind(nx: f64, ny: f64, c: f64) -> f64 {
    // Mirrored so both components are non-negative: `x → 1 − x` turns `nx·x` into
    // `|nx|·x − |nx|`, which moves the line and not the area.
    let (a, b) = (nx.abs(), ny.abs());
    let c = c - nx.min(0.0) - ny.min(0.0);
    // A normal within this of an axis is the axis: the corner formula divides by `a·b`.
    if a < 1e-12 || b < 1e-12 {
        return (c / (a + b)).clamp(0.0, 1.0);
    }
    let corner = |t: f64| t.max(0.0).powi(2) / 2.0;
    (corner(c) - corner(c - a) - corner(c - b) + corner(c - a - b)) / (a * b)
}

/// Euclid's algorithm, on the components of a direction.
fn greatest_common_divisor(mut a: i64, mut b: i64) -> i64 {
    while let Some(remainder) = a.checked_rem(b) {
        (a, b) = (b, remainder);
    }
    a
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)] // test-file policy as in `winding.rs`: integer fixtures far below every type's range
mod tests {
    use super::{SampleLattice, SamplePattern};

    /// The counts the device admits: a square, clamped to `4..=64` (`device/construct.rs`).
    const ADMITTED: [u32; 7] = [4, 9, 16, 25, 36, 49, 64];

    /// Every pattern the tests measure: the lane's at each admitted count, and every rank-1
    /// lattice of each, generator by generator — the second population has widths in closed
    /// form ([`SampleLattice`]'s own tests hold those against brute force), so it checks the
    /// general computation from outside.
    fn populations() -> Vec<SamplePattern> {
        let mut patterns: Vec<SamplePattern> =
            ADMITTED.iter().map(|&n| SamplePattern::of(n)).collect();
        for n in ADMITTED {
            for g in 1..n {
                if gcd(i64::from(g), i64::from(n)) == 1 {
                    let rows: Vec<u8> = (0..n).map(|i| ((g * i) % n) as u8).collect();
                    patterns.extend(SamplePattern::from_rows(&rows));
                }
            }
        }
        patterns
    }

    /// The samples in units of `1/(2n)`, so that every coordinate is an odd integer.
    fn doubled(pattern: &SamplePattern) -> Vec<(i64, i64)> {
        pattern
            .rows()
            .iter()
            .zip(0_i64..)
            .map(|(&r, i)| (2 * i + 1, 2 * i64::from(r) + 1))
            .collect()
    }

    /// The lane's pattern at each admitted count gives every sample a column and a row of its
    /// own, which is what gives an axis-aligned edge `n + 1` levels; at a count the device
    /// never asks for, the pattern is the lattice's.
    #[test]
    fn every_pattern_has_a_column_and_a_row_per_sample() {
        for n in ADMITTED {
            let pattern = SamplePattern::of(n);
            assert_eq!(pattern.count(), n);
            let mut rows = pattern.rows().to_vec();
            rows.sort_unstable();
            assert_eq!(rows, (0..n as u8).collect::<Vec<_>>(), "{n} samples");
        }
        for n in [1, 2, 3, 5, 15, 17, 63] {
            assert_eq!(
                SamplePattern::of(n),
                SampleLattice::of(n).pattern(),
                "{n} samples"
            );
        }
    }

    /// `from_rows` refuses what is not of the lane's shape: a repeated row, a row past the
    /// count, no samples, more than the device takes.
    #[test]
    fn a_pattern_that_is_not_a_permutation_is_refused() {
        assert!(SamplePattern::from_rows(&[0, 1, 1, 3]).is_none());
        assert!(SamplePattern::from_rows(&[0, 1, 2, 4]).is_none());
        assert!(SamplePattern::from_rows(&[]).is_none());
        assert!(SamplePattern::from_rows(&(0..65).collect::<Vec<u8>>()).is_none());
        assert!(SamplePattern::from_rows(&[3, 1, 0, 2]).is_some());
    }

    /// Four samples on the diagonal, by hand: the band along `(1, 1)` is the gap between the
    /// diagonals `x − y = k`, `1/√2`, and the widest square sits between two neighbouring
    /// samples, three quarters of a pixel.
    #[test]
    fn the_diagonal_of_four_by_hand() {
        let diagonal = SamplePattern::from_rows(&[0, 1, 2, 3]).expect("a permutation");
        assert!((diagonal.widest_empty_band() - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((diagonal.widest_empty_square() - 0.75).abs() < 1e-6);
    }

    /// The widest empty band against a brute force that prunes nothing: every direction of an
    /// integer vector up to the pixel's width, in units of `1/(2n)`, with the plane's copies of
    /// the pattern projecting to multiples of `2n`.
    #[test]
    fn the_widest_empty_band_by_brute_force_over_every_direction() {
        for pattern in populations() {
            let n = i64::from(pattern.count());
            let samples = doubled(&pattern);
            let period = 2 * n;
            let mut widest = 0.0_f64;
            for p in 0..=n {
                for q in -n..=n {
                    if (p, q) == (0, 0) || gcd(p, q.abs()) != 1 || (p == 0 && q < 0) {
                        continue;
                    }
                    let mut projections: Vec<i64> = samples
                        .iter()
                        .map(|&(x, y)| (-q * x + p * y).rem_euclid(period))
                        .collect();
                    projections.sort_unstable();
                    let wrap = projections[0] + period - projections[projections.len() - 1];
                    let gap = projections
                        .windows(2)
                        .map(|w| w[1] - w[0])
                        .fold(wrap, i64::max);
                    let width = gap as f64 / period as f64 / ((p * p + q * q) as f64).sqrt();
                    widest = widest.max(width);
                }
            }
            let computed = f64::from(pattern.widest_empty_band());
            assert!(
                (widest - computed).abs() < 1e-6,
                "{:?}: brute force {widest}, computed {computed}",
                pattern.rows()
            );
        }
    }

    /// The widest empty square against a brute force over every corner a sample's column and
    /// another sample's row define, against the samples of the pixel and its eight neighbours.
    #[test]
    fn the_widest_empty_square_by_brute_force_over_every_corner() {
        for pattern in populations() {
            let samples = doubled(&pattern);
            let period = 2 * i64::from(pattern.count());
            let tiled: Vec<(i64, i64)> = samples
                .iter()
                .flat_map(|&(x, y)| {
                    (-1..2)
                        .flat_map(move |a| (-1..2).map(move |b| (x + a * period, y + b * period)))
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
                .expect("a pattern has samples");
            let brute = widest as f64 / period as f64;
            let computed = f64::from(pattern.widest_empty_square());
            assert!(
                (brute - computed).abs() < 1e-6,
                "{:?}: brute force {brute}, computed {computed}",
                pattern.rows()
            );
        }
    }

    /// On every lattice the general computation is the lattice's closed form.
    #[test]
    fn on_a_lattice_both_widths_are_the_closed_forms() {
        for n in ADMITTED {
            let lattice = SampleLattice::of(n);
            let pattern = lattice.pattern();
            // Within an f32's rounding: the closed form divides by `√m`, the computation by
            // `n·|(p, q)|`.
            let band = pattern.widest_empty_band() - lattice.widest_empty_band();
            let square = pattern.widest_empty_square() - lattice.widest_empty_square();
            assert!(
                band.abs() < 1e-6 && square.abs() < 1e-6,
                "{n}: {band}, {square}"
            );
        }
    }

    /// The constraint the search was held to, re-derived: at every admitted count neither of the
    /// lane's widths is wider than the lattice's, so ADR 0070's threshold is no wider either.
    #[test]
    fn neither_width_is_wider_than_the_lattice_s() {
        for n in ADMITTED {
            let (pattern, lattice) = (SamplePattern::of(n), SampleLattice::of(n).pattern());
            assert!(
                pattern.widest_empty_band() <= lattice.widest_empty_band(),
                "{n}"
            );
            assert!(
                pattern.widest_empty_square() <= lattice.widest_empty_square(),
                "{n}"
            );
            assert!(
                pattern.vanishing_width() <= lattice.vanishing_width(),
                "{n}"
            );
        }
    }

    /// An axis-aligned edge errs by at most half a sample pitch, `1/(2n)`, on every pattern of
    /// the lane's shape: the samples behind it are a count of columns, and the area moves by
    /// one column's width between two of them.
    #[test]
    fn an_axis_aligned_edge_errs_by_half_a_pitch() {
        for pattern in populations() {
            let error = pattern.straight_edge_error(2).worst;
            let half_pitch = 0.5 / f64::from(pattern.count());
            assert!(
                (error - half_pitch).abs() < 1e-9,
                "{:?}: {error}",
                pattern.rows()
            );
        }
    }

    /// What the search bought, re-derived: at every admitted count the lane's worst straight
    /// edge is nearer its area than the lattice's.
    #[test]
    fn the_lane_s_worst_straight_edge_is_nearer_than_the_lattice_s() {
        for n in ADMITTED {
            let pattern = SamplePattern::of(n).straight_edge_error(4096).worst;
            let lattice = SampleLattice::of(n)
                .pattern()
                .straight_edge_error(4096)
                .worst;
            assert!(
                pattern < lattice,
                "{n} samples: {pattern} against the lattice's {lattice}"
            );
        }
    }

    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
}
