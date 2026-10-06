//! The sweep's arithmetic against the clauses that define it.
//!
//! Every expected number below is derived from ISO 32000-2 and the comment above it says
//! from which clause — never from what this function returns, which would make the test
//! a record of the implementation rather than a check on it.
#![allow(clippy::arithmetic_side_effects)] // test indices are tiny and literal

use raster_scene::{Color, Stop};

use super::{
    MAX_SEGMENTS, RAMP_RESOLUTION, RAMP_ROWS, lay_out, ramp_color_at, sample_ramp, sample_ramps,
    segments, texel_byte, texel_for,
};

const RED: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
const BLUE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

fn stop(offset: f32, color: Color) -> Stop {
    Stop { offset, color }
}

/// The last index `sample_ramp` divides by, and the grid every claim below is on.
#[expect(clippy::cast_precision_loss)] // 4095, exact in f32
const LAST: f32 = (RAMP_RESOLUTION - 1) as f32;

/// Two stops are §7.10.3's type 2 exponential with `N` of 1, whose value is
/// `C0 + x^N × (C1 − C0)` — linear, so a quarter of the way along is a quarter of
/// the way between the colours, exactly.
#[test]
fn two_stops_are_the_type_2_interpolation_between_them() {
    let stops = [stop(0.0, RED), stop(1.0, BLUE)];
    assert_eq!(ramp_color_at(&stops, 0.0), RED);
    assert_eq!(ramp_color_at(&stops, 1.0), BLUE);
    assert_eq!(
        ramp_color_at(&stops, 0.25),
        Color::new(0.75, 0.0, 0.25, 1.0)
    );
    assert_eq!(ramp_color_at(&stops, 0.5), Color::new(0.5, 0.0, 0.5, 1.0));
}

/// §7.10.1 clips an input outside a function's declared domain to the nearest
/// boundary value, so a ramp whose stops span only the middle of `0..=1` holds its
/// end colours over the rest rather than fading out of them.
#[test]
fn outside_the_stops_a_ramp_holds_its_end_colours() {
    let stops = [stop(0.25, RED), stop(0.75, BLUE)];
    assert_eq!(ramp_color_at(&stops, 0.0), RED);
    assert_eq!(ramp_color_at(&stops, 0.25), RED);
    assert_eq!(ramp_color_at(&stops, 0.75), BLUE);
    assert_eq!(ramp_color_at(&stops, 1.0), BLUE);
}

/// A coincident pair of offsets is §7.10.4's stitching boundary: two subfunctions
/// meet at a bound and neither interpolates across it, so the two sides are the two
/// colours and there is nothing between them.
///
/// **The bound itself is the later subfunction's**, because §7.10.4's subdomains are
/// "closed on the left and open on the right" away from the two ends — so `t == 0.5`
/// is in `[0.5, 1.0]` and not in `[0.0, 0.5)`, and the colour there is `BLUE`.
#[test]
fn a_coincident_pair_is_a_step_and_not_a_ramp() {
    let stops = [
        stop(0.0, RED),
        stop(0.5, RED),
        stop(0.5, BLUE),
        stop(1.0, BLUE),
    ];
    // One grid step either side of the boundary: the two subfunctions' own values,
    // not a blend of them.
    assert_eq!(ramp_color_at(&stops, 0.5 - 1.0 / LAST), RED);
    assert_eq!(ramp_color_at(&stops, 0.5 + 1.0 / LAST), BLUE);
    // And the bound belongs to the interval that starts there.
    assert_eq!(ramp_color_at(&stops, 0.5), BLUE);
    // And each subfunction is constant over its own subdomain, which is what makes
    // the step a step: §7.10.3's `C0 == C1` is a type 2 that does not move.
    assert_eq!(ramp_color_at(&stops, 0.1), RED);
    assert_eq!(ramp_color_at(&stops, 0.9), BLUE);
}

/// §7.10.4's two exceptions to *closed on the left*, which point opposite ways and
/// are the only two places a ramp's boundary rule reverses:
///
/// > - the last interval, shall always be closed on the right,
/// > - if Domain0 = Bounds0 then the first interval shall be closed on both the left
/// >   and right and the second (next) interval shall be open on the left.
///
/// So a pair coincident at the ramp's **first** offset gives that point to the
/// earlier stop — the degenerate first interval `[0, 0]` — and a pair coincident at
/// its **last** offset gives it to the later one, whose interval `[1, 1]` is closed
/// on the right while the one below it is open there. Both are grid positions
/// `sample_ramp` visits exactly, so both are texels of every such ramp.
#[test]
fn the_ramps_two_ends_take_a_coincident_pair_opposite_ways() {
    let at_the_start = [stop(0.0, RED), stop(0.0, BLUE), stop(1.0, BLUE)];
    assert_eq!(ramp_color_at(&at_the_start, 0.0), RED);
    let at_the_end = [stop(0.0, RED), stop(1.0, RED), stop(1.0, BLUE)];
    assert_eq!(ramp_color_at(&at_the_end, 1.0), BLUE);
    // Each is one texel of the table and not a region of it: the neighbour on the
    // inside is the other subfunction's colour in both cases.
    assert_eq!(ramp_color_at(&at_the_start, 1.0 / LAST), BLUE);
    assert_eq!(ramp_color_at(&at_the_end, 1.0 - 1.0 / LAST), RED);
}

/// The colour row's texel `t` reads, as four bytes.
fn colour_at(table: &[u8], t: f32) -> [u8; 4] {
    let at = texel_for(table, t) * 4;
    [table[at], table[at + 1], table[at + 2], table[at + 3]]
}

/// A smooth ramp is one segment: row 0 is one texel per grid step, ending on the last
/// stop, with each component the 8-bit level nearest the colour — the arithmetic ADR
/// 0011 keeps on the CPU so that every adapter reads the same texel — and `t` reads
/// texel `round(t · 4095)`.
#[test]
fn a_smooth_ramp_is_one_texel_per_grid_step() {
    let stops = [stop(0.0, RED), stop(1.0, BLUE)];
    let bytes = sample_ramp(&stops);
    assert_eq!(
        bytes.len(),
        RAMP_RESOLUTION as usize * RAMP_ROWS as usize * 4
    );
    assert_eq!(&bytes[0..4], &[255, 0, 0, 255]);
    let row = RAMP_RESOLUTION as usize * 4;
    assert_eq!(&bytes[row - 4..row], &[0, 0, 255, 255]);
    // Texel `i` is the ramp at `i / (N − 1)`: a quarter of the way along the grid is
    // §7.10.3's quarter, rounded to a byte — `round(0.75 × 255)` is 191.
    let quarter = (RAMP_RESOLUTION as usize - 1) / 4;
    assert_eq!(&bytes[quarter * 4..quarter * 4 + 4], &[191, 0, 64, 255]);
    for i in [0_usize, 1, 1000, 2047, 2048, 4094, 4095] {
        #[expect(clippy::cast_precision_loss)] // an index below 4096
        let t = i as f32 / LAST;
        assert_eq!(texel_for(&bytes, t), i, "t = {t}");
    }
}

/// §8.7.4.5.3 gives a point the function's value at its own `t`, and §7.10.4 gives a
/// bound to the interval that starts there: so the step is **at** its offset, on both
/// sides, however the offset sits against any grid (ADR 1389). The offsets below are
/// deliberately off the grid, one on it, and one a hair from a grid position.
#[test]
fn a_hard_step_is_at_its_own_offset_on_both_sides() {
    for boundary in [
        0.313_79_f32,
        2048.0 / LAST,
        0.5 + 0.5 / LAST,
        0.000_1,
        0.999_9,
    ] {
        let stops = [
            stop(0.0, RED),
            stop(boundary, RED),
            stop(boundary, BLUE),
            stop(1.0, BLUE),
        ];
        let bytes = sample_ramp(&stops);
        let below = f32::from_bits(boundary.to_bits() - 1);
        assert_eq!(
            colour_at(&bytes, below),
            [255, 0, 0, 255],
            "below {boundary}"
        );
        assert_eq!(
            colour_at(&bytes, boundary),
            [0, 0, 255, 255],
            "at {boundary}"
        );
        assert_eq!(colour_at(&bytes, 0.0), [255, 0, 0, 255]);
        assert_eq!(colour_at(&bytes, 1.0), [0, 0, 255, 255]);
    }
}

/// Between steps a segment is still a ramp: a step from a red→blue sweep to a
/// green→white one keeps each side's §7.10.3 interpolation, and the texel just below
/// the step is the lower piece's value there, not the upper's.
#[test]
fn each_segment_keeps_its_own_interpolation() {
    let green = Color::new(0.0, 1.0, 0.0, 1.0);
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    let stops = [
        stop(0.0, RED),
        stop(0.5, BLUE),
        stop(0.5, green),
        stop(1.0, white),
    ];
    let bytes = sample_ramp(&stops);
    // A quarter: halfway between red and blue, and three quarters halfway between
    // green and white — each to within the one level the nearest texel of the
    // segment's own grid may round (half of `1/2047` of the parameter here).
    let near = |got: [u8; 4], want: [u8; 4]| got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 1);
    assert!(near(colour_at(&bytes, 0.25), [128, 0, 128, 255]));
    assert!(near(colour_at(&bytes, 0.75), [128, 255, 128, 255]));
    // A hair below the step is the lower piece near its end: blue.
    assert_eq!(colour_at(&bytes, 0.499_99), [0, 0, 255, 255]);
    assert_eq!(colour_at(&bytes, 0.5), [0, 255, 0, 255]);
}

/// §7.10.4's two ends, through the table: a pair coincident at the first offset gives
/// that point to the earlier stop, and one at the last offset gives it to the later —
/// each a segment of one texel.
#[test]
fn the_tables_two_ends_take_a_coincident_pair_opposite_ways() {
    let at_the_start = sample_ramp(&[stop(0.0, RED), stop(0.0, BLUE), stop(1.0, BLUE)]);
    assert_eq!(colour_at(&at_the_start, 0.0), [255, 0, 0, 255]);
    assert_eq!(colour_at(&at_the_start, 1.0e-6), [0, 0, 255, 255]);
    let at_the_end = sample_ramp(&[stop(0.0, RED), stop(1.0, RED), stop(1.0, BLUE)]);
    assert_eq!(colour_at(&at_the_end, 1.0), [0, 0, 255, 255]);
    assert_eq!(colour_at(&at_the_end, 1.0 - 1.0e-6), [255, 0, 0, 255]);
}

/// Three stops at one offset: the middle one's interval `[s, s)` is empty and holds
/// no `t`, so the offset itself and everything above it is the last stop's.
#[test]
fn a_middle_stop_of_three_coincident_holds_nothing() {
    let green = Color::new(0.0, 1.0, 0.0, 1.0);
    let stops = [
        stop(0.0, RED),
        stop(0.5, RED),
        stop(0.5, green),
        stop(0.5, BLUE),
        stop(1.0, BLUE),
    ];
    let bytes = sample_ramp(&stops);
    assert_eq!(colour_at(&bytes, 0.5), [0, 0, 255, 255]);
    assert_eq!(colour_at(&bytes, 0.499_99), [255, 0, 0, 255]);
}

/// Past [`MAX_SEGMENTS`] a ramp is one segment and its steps round to the nearest
/// texel: the bound is a stated fallback, not a failure. Just under it, every step is
/// still exact.
#[test]
fn a_ramp_past_the_segment_bound_is_one_segment() {
    let ramp = |count: usize| {
        let mut stops = Vec::new();
        for k in 0..count {
            #[expect(clippy::cast_precision_loss)] // counts below 2048
            let (lo, hi) = (k as f32 / count as f32, (k + 1) as f32 / count as f32);
            let color = if k % 2 == 0 { RED } else { BLUE };
            stops.push(stop(lo, color));
            stops.push(stop(hi, color));
        }
        stops
    };
    let under = sample_ramp(&ramp(MAX_SEGMENTS));
    let row = RAMP_RESOLUTION as usize * 4;
    let count = |bytes: &[u8]| {
        u32::from_le_bytes([
            bytes[2 * row],
            bytes[2 * row + 1],
            bytes[2 * row + 2],
            bytes[2 * row + 3],
        ])
    };
    assert_eq!(count(&under) as usize, MAX_SEGMENTS);
    let over = sample_ramp(&ramp(MAX_SEGMENTS + 1));
    assert_eq!(count(&over), 1);
}

/// An empty ramp is refused at upload (`ResourceProblem::RampEmpty`), and this
/// function still answers rather than dividing by a span it does not have:
/// transparent black over the whole table, which is a colour and not a NaN.
#[test]
fn an_empty_ramp_samples_to_transparency() {
    let bytes = sample_ramp(&[]);
    assert_eq!(
        bytes.len(),
        RAMP_RESOLUTION as usize * RAMP_ROWS as usize * 4
    );
    assert!(bytes.iter().all(|byte| *byte == 0));
}

/// The colour row as [`sample_ramp`] states it: [`ramp_color_at`] asked at every texel's own
/// `t`, and each component rounded by `f32::round` — the statement the cursor and
/// [`texel_byte`] are held to (ADR 1567).
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[expect(clippy::cast_precision_loss)]
fn colour_row_as_stated(stops: &[Stop]) -> Vec<u8> {
    let mut row = vec![0_u8; RAMP_RESOLUTION as usize * 4];
    let mut runs = segments(stops);
    if runs.len() > MAX_SEGMENTS {
        runs = vec![(0, stops.len() - 1)];
    }
    for segment in lay_out(stops, &runs) {
        let own = &stops[segment.first..=segment.last];
        let (lo, hi) = (stops[segment.first].offset, stops[segment.last].offset);
        let last = (segment.texels.saturating_sub(1).max(1)) as f32;
        for j in 0..segment.texels {
            let t = lo + (hi - lo) * (j as f32 / last);
            let color = ramp_color_at(own, t);
            let at = (segment.base + j) as usize * 4;
            for (c, component) in [color.r, color.g, color.b, color.a].into_iter().enumerate() {
                row[at + c] = (component * 255.0).round() as u8;
            }
        }
    }
    row
}

/// A small deterministic generator, so that the ramps below are many and the same every run.
struct Draws(u64);

impl Draws {
    #[expect(clippy::cast_precision_loss)]
    fn unit(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 40) as f32 / (1_u64 << 24) as f32
    }
}

/// Every ramp shape the table meets — two stops, many, coincident pairs and runs of them, stops
/// that leave the ends uncovered, components at and between the levels' half-way points — makes
/// the same colour row through the cursor and the byte rounding as through the per-texel
/// statement (ADR 1567).
#[test]
fn the_table_is_the_per_texel_statement_byte_for_byte() {
    let mut draws = Draws(0x1567);
    for case in 0..600 {
        let count = 1 + case % 23;
        let mut offsets: Vec<f32> = (0..count).map(|_| draws.unit()).collect();
        offsets.sort_by(f32::total_cmp);
        if case % 3 == 0 && count > 2 {
            offsets[count / 2] = offsets[count / 2 - 1];
        }
        if case % 5 == 0 {
            offsets[0] = 0.0;
            offsets[count - 1] = 1.0;
        }
        let stops: Vec<Stop> = offsets
            .iter()
            .map(|&offset| {
                // Every fourth component sits exactly on a half level, where rounding decides.
                let mut component = || {
                    let value = draws.unit();
                    if case % 4 == 0 {
                        ((value * 255.0).floor() + 0.5) / 255.0
                    } else {
                        value
                    }
                };
                stop(
                    offset,
                    Color::new(component(), component(), component(), component()),
                )
            })
            .collect();
        let row = RAMP_RESOLUTION as usize * 4;
        assert_eq!(
            &sample_ramp(&stops)[..row],
            colour_row_as_stated(&stops).as_slice(),
            "case {case}: {stops:?}"
        );
    }
}

/// [`texel_byte`] is `(x · 255).round()` saturated into a byte, at every half level, either
/// side of it by an ulp, and outside `0..=1`.
#[test]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn a_texel_byte_rounds_as_round_does() {
    let mut probes = vec![-1.0_f32, -0.0, 0.0, 1.0, 1.5, f32::NAN, f32::INFINITY];
    for level in 0..=255_u16 {
        let half = (f32::from(level) + 0.5) / 255.0;
        probes.extend([half, half.next_up(), half.next_down()]);
    }
    for component in probes {
        assert_eq!(
            texel_byte(component),
            (component * 255.0).round() as u8,
            "{component}"
        );
    }
}

/// Tables made on threads are the tables made one after another, in the order asked, at every
/// thread count and either side of the floor.
#[test]
fn ramps_made_beside_each_other_are_made_in_order() {
    let ramps: Vec<Vec<Stop>> = (0..19_u8)
        .map(|k| {
            let shade = f32::from(k) / 19.0;
            vec![
                stop(0.0, Color::new(shade, 0.0, 1.0 - shade, 1.0)),
                stop(0.5, RED),
                stop(1.0, Color::new(0.0, shade, 0.0, 1.0)),
            ]
        })
        .collect();
    let one_by_one: Vec<Vec<u8>> = ramps.iter().map(|stops| sample_ramp(stops)).collect();
    for count in [1, 3, 19] {
        let asked: Vec<&[Stop]> = ramps[..count].iter().map(Vec::as_slice).collect();
        for threads in [1, 2, 8, 64] {
            assert_eq!(
                sample_ramps(&asked, threads),
                one_by_one[..count].to_vec(),
                "{count} ramps on {threads} threads"
            );
        }
    }
}
