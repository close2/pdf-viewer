//! The program that derives the device lane's sample patterns (the viewer's ADR 2313).
//!
//! For each count the device admits it searches the permutations of `n` rows — the patterns
//! that give every sample a column and a row of its own — for the one whose worst straight edge
//! is nearest the area behind it ([`SamplePattern::straight_edge_error`]), under one
//! constraint: its widest empty band and its widest empty square are each no wider than the
//! count's rank-1 lattice's ([`SampleLattice`]), so the pattern loses no mark under ADR 0070's
//! two readings that the lattice would not.
//!
//! The search is simulated annealing over transpositions, and it is **reproducible** rather
//! than random: every chain starts from the lattice and draws from `SplitMix64` seeded by
//! [`SEED`], the count and the chain's index, so the same program finds the same rows on every
//! machine. A move that breaks the constraint is refused; one that is no worse — by the worst
//! error, then by the mean over directions — is taken, and a worse one with the Metropolis
//! probability at a temperature that falls linearly to zero. The best pattern any chain met is
//! the count's, the earlier chain on a tie.
//!
//! The search compares patterns at [`SEARCH_DIRECTIONS`] directions; the figures it prints are
//! at [`REPORT_DIRECTIONS`], which is what the ADR quotes.
//!
//! Run: `cargo run --release -p raster-gpu --example sample_pattern` prints the rows, each
//! count's widths and errors against the lattice's, and the table `winding/pattern.rs` holds.
//! `--check` runs the same search and fails on any count whose rows differ from that table, so
//! the table stays what this program derives (CI runs it, ADR 0060).

// The counts and indices here are at most 64 and the iteration counts far below 2^53; a
// panic in a chain or an empty search is a broken program, and says so.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects,
    clippy::expect_used
)]

use raster_gpu::{EdgeError, SampleLattice, SamplePattern};

/// The counts the device admits: a square, clamped to `4..=64` (`device/construct.rs`).
const ADMITTED: [u32; 7] = [4, 9, 16, 25, 36, 49, 64];

/// The seed every chain's generator is derived from: the session that ran the search first.
const SEED: u64 = 1739;

/// Independent chains per count, each from the lattice with a generator of its own.
const CHAINS: u64 = 8;

/// Moves each chain proposes.
const ITERATIONS: u32 = 40_000;

/// The starting temperature, as a share of the lattice's worst edge error.
const TEMPERATURE: f64 = 0.03;

/// Directions the search compares two patterns at.
const SEARCH_DIRECTIONS: u32 = 512;

/// Directions the printed figures are taken at.
const REPORT_DIRECTIONS: u32 = 16_384;

/// `SplitMix64` (Steele, Lea and Flood, 2014): a generator whose whole state is one counter, so a
/// seed is all a chain needs to be repeated.
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform draw from `[0, 1)`, from the top 53 bits.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }

    /// A uniform draw from `0..bound`.
    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

/// Whether `a` beats or ties `b`: by the worst error, then by the mean.
fn no_worse(a: EdgeError, b: EdgeError) -> bool {
    (a.worst, a.mean) <= (b.worst, b.mean)
}

/// One chain's best pattern and its error.
fn chain(lattice: SampleLattice, chain: u64) -> (Vec<u8>, EdgeError) {
    let start = lattice.pattern();
    let (band, square) = (start.widest_empty_band(), start.widest_empty_square());
    let n = start.count() as usize;
    let mut random =
        SplitMix(SEED ^ u64::from(start.count()) ^ chain.wrapping_mul(0xD1B5_4A32_D192_ED03));
    let mut rows = start.rows().to_vec();
    let mut error = start.straight_edge_error(SEARCH_DIRECTIONS);
    let temperature = TEMPERATURE * error.worst;
    let (mut best, mut best_error) = (rows.clone(), error);
    for iteration in 0..ITERATIONS {
        let i = random.below(n);
        let j = (i + 1 + random.below(n.saturating_sub(1).max(1))) % n;
        let draw = random.unit();
        rows.swap(i, j);
        let taken = SamplePattern::from_rows(&rows)
            .filter(|p| p.widest_empty_band() <= band && p.widest_empty_square() <= square)
            .map(|p| p.straight_edge_error(SEARCH_DIRECTIONS))
            .filter(|candidate| {
                let delta = if candidate.worst.total_cmp(&error.worst).is_eq() {
                    candidate.mean - error.mean
                } else {
                    candidate.worst - error.worst
                };
                let t = temperature * (1.0 - f64::from(iteration) / f64::from(ITERATIONS));
                delta <= 0.0 || (t > 0.0 && draw < (-delta / t).exp())
            });
        match taken {
            Some(candidate) => {
                error = candidate;
                if !no_worse(best_error, error) {
                    (best, best_error) = (rows.clone(), error);
                }
            }
            None => rows.swap(i, j),
        }
    }
    (best, best_error)
}

/// The count's pattern: the best of [`CHAINS`] chains, run side by side.
fn search(count: u32) -> Vec<u8> {
    let lattice = SampleLattice::of(count);
    let results: Vec<(Vec<u8>, EdgeError)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..CHAINS)
            .map(|index| scope.spawn(move || chain(lattice, index)))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a chain does not panic"))
            .collect()
    });
    results
        .into_iter()
        .reduce(|kept, next| if no_worse(kept.1, next.1) { kept } else { next })
        .map(|(rows, _)| rows)
        .expect("CHAINS is not zero")
}

fn main() {
    let check = std::env::args().any(|arg| arg == "--check");
    let mut table = Vec::new();
    for count in ADMITTED {
        let started = std::time::Instant::now();
        let rows = search(count);
        let found = SamplePattern::from_rows(&rows).expect("a search keeps a permutation");
        let lattice = SampleLattice::of(count).pattern();
        println!(
            "{count:>2} samples, {:.1} s: band {:.4} (lattice {:.4}), \
             square {:.4} (lattice {:.4}), \
             worst edge {:.4} (lattice {:.4}), mean {:.4} (lattice {:.4})",
            started.elapsed().as_secs_f64(),
            found.widest_empty_band(),
            lattice.widest_empty_band(),
            found.widest_empty_square(),
            lattice.widest_empty_square(),
            found.straight_edge_error(REPORT_DIRECTIONS).worst,
            lattice.straight_edge_error(REPORT_DIRECTIONS).worst,
            found.straight_edge_error(REPORT_DIRECTIONS).mean,
            lattice.straight_edge_error(REPORT_DIRECTIONS).mean,
        );
        if check {
            assert_eq!(
                SamplePattern::of(count).rows(),
                found.rows(),
                "{count} samples: `winding/pattern.rs`'s table is not what this search derives"
            );
        }
        table.push(format!("    ({count}, &{rows:?}),"));
    }
    println!(
        "const SEARCHED: [(u32, &[u8]); 7] = [\n{}\n];",
        table.join("\n")
    );
}
