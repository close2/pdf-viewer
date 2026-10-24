//! The sweep's arithmetic against the clauses that define it.
//!
//! Every expected number below is derived from ISO 32000-2 and the comment above it says
//! from which clause — never from what this function returns, which would make the test
//! a record of the implementation rather than a check on it.
#![allow(clippy::arithmetic_side_effects, clippy::cast_precision_loss)] // test indices and counts are tiny and literal

use raster_scene::{Color, Stop};

use super::{
    MAX_RAMP_STOPS, RAMP_ROWS, RAMP_WIDTH, RampTable, colour_for, ramp_color_at, ramp_table,
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

/// A small step of the parameter, `1/4095`, for the claims below about either side of a
/// point: far coarser than an ulp, far finer than any interval the tests build.
const LAST: f32 = 4095.0;

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
/// on the right while the one below it is open there.
#[test]
fn the_ramps_two_ends_take_a_coincident_pair_opposite_ways() {
    let at_the_start = [stop(0.0, RED), stop(0.0, BLUE), stop(1.0, BLUE)];
    assert_eq!(ramp_color_at(&at_the_start, 0.0), RED);
    let at_the_end = [stop(0.0, RED), stop(1.0, RED), stop(1.0, BLUE)];
    assert_eq!(ramp_color_at(&at_the_end, 1.0), BLUE);
    // Each is one point and not a region: the neighbour on the inside is the other
    // subfunction's colour in both cases.
    assert_eq!(ramp_color_at(&at_the_start, 1.0 / LAST), BLUE);
    assert_eq!(ramp_color_at(&at_the_end, 1.0 - 1.0 / LAST), RED);
}

/// The table's colour at `t` as the bits of its four components, so that a comparison is
/// exact and a NaN would differ rather than compare unequal to itself unnoticed.
fn bits(color: Color) -> [u32; 4] {
    [
        color.r.to_bits(),
        color.g.to_bits(),
        color.b.to_bits(),
        color.a.to_bits(),
    ]
}

/// The word of row `row` for stop `index` in a one-block table, read back from the bytes.
fn word(table: &RampTable, index: usize, row: usize) -> f32 {
    let at = (row * table.width as usize + index) * 4;
    f32::from_le_bytes([
        table.bytes[at],
        table.bytes[at + 1],
        table.bytes[at + 2],
        table.bytes[at + 3],
    ])
}

/// A ramp's table is its stops: one column a stop, its offset and its four components each
/// the `f32` itself (ADR 2310), so a two-stop ramp is two texels by five rows.
#[test]
fn a_ramps_table_is_its_own_stops() {
    let stops = [stop(0.25, RED), stop(0.75, BLUE)];
    let table = ramp_table(&stops);
    assert_eq!((table.width, table.height), (2, RAMP_ROWS));
    assert_eq!(table.bytes.len(), 2 * RAMP_ROWS as usize * 4);
    for (index, expected) in stops.iter().enumerate() {
        assert_eq!(word(&table, index, 0).to_bits(), expected.offset.to_bits());
        let color = expected.color;
        for (row, component) in [color.r, color.g, color.b, color.a].into_iter().enumerate() {
            assert_eq!(word(&table, index, row + 1).to_bits(), component.to_bits());
        }
    }
}

/// §8.7.4.5.3 gives a point the function's value at its own `t`, and the ramp's statement of
/// that is [`ramp_color_at`]: the table's lookup is that function, bit for bit, at every `t` a
/// pixel can bring — on each stop, an ulp either side of it, between, and outside the stops —
/// for every ramp shape the table meets: two stops, many, coincident pairs and runs of them,
/// and stops that leave the ends uncovered.
#[test]
fn the_table_is_the_ramps_own_colour_at_every_t() {
    let mut draws = Draws(0x2310);
    for case in 0..600 {
        let count = 1 + case % 23;
        let mut offsets: Vec<f32> = (0..count).map(|_| draws.unit()).collect();
        offsets.sort_by(f32::total_cmp);
        if case % 3 == 0 && count > 2 {
            offsets[count / 2] = offsets[count / 2 - 1];
        }
        if case % 7 == 0 && count > 3 {
            offsets[2] = offsets[1];
            offsets[3] = offsets[1];
        }
        if case % 5 == 0 {
            offsets[0] = 0.0;
            offsets[count - 1] = 1.0;
        }
        let stops: Vec<Stop> = offsets
            .iter()
            .map(|&offset| {
                stop(
                    offset,
                    Color::new(draws.unit(), draws.unit(), draws.unit(), draws.unit()),
                )
            })
            .collect();
        let table = ramp_table(&stops);
        let mut ts: Vec<f32> = vec![0.0, 1.0, -0.5, 1.5];
        for offset in &offsets {
            ts.extend([
                *offset,
                f32::from_bits(offset.to_bits().saturating_sub(1)),
                f32::from_bits(offset.to_bits().saturating_add(1)),
            ]);
        }
        ts.extend((0..64).map(|_| draws.unit()));
        for t in ts {
            assert_eq!(
                bits(colour_for(&table, t)),
                bits(ramp_color_at(&stops, t)),
                "case {case} at {t}: {stops:?}"
            );
        }
    }
}

/// §8.7.4.5.3 gives a point the function's value at its own `t`, and §7.10.4 gives a
/// bound to the interval that starts there: so the step is **at** its offset, on both
/// sides, however the offset sits against any grid. The offsets below are deliberately
/// arbitrary, and two of them a hair from either end.
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
        let table = ramp_table(&stops);
        let below = f32::from_bits(boundary.to_bits() - 1);
        assert_eq!(colour_for(&table, below), RED, "below {boundary}");
        assert_eq!(colour_for(&table, boundary), BLUE, "at {boundary}");
        assert_eq!(colour_for(&table, 0.0), RED);
        assert_eq!(colour_for(&table, 1.0), BLUE);
    }
}

/// A steep ramp is read at its own `t`. `issue14165.pdf`'s function crosses the whole range
/// many times along its axis, and between two stops §7.10.3's interpolation is exact
/// arithmetic: so a quarter of the way between a black stop and a white one is a quarter
/// level of white however steep the ramp is — the answer a grid read at its nearest texel
/// moved by half a texel's worth of slope (ADR 2310).
#[test]
fn a_steep_ramp_is_read_at_its_own_t() {
    let black = Color::new(0.0, 0.0, 0.0, 1.0);
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    let stops: Vec<Stop> = (0..=32)
        .map(|k| stop(k as f32 / 32.0, if k % 2 == 0 { black } else { white }))
        .collect();
    let table = ramp_table(&stops);
    for k in 0..32 {
        let rising = k % 2 == 0;
        for quarter in 1..4 {
            let t = (k as f32 + quarter as f32 / 4.0) / 32.0;
            let grey = colour_for(&table, t).r;
            let expected = if rising {
                quarter as f32 / 4.0
            } else {
                1.0 - quarter as f32 / 4.0
            };
            assert!(
                (grey - expected).abs() < 1e-5,
                "at {t}: {grey}, the clause's {expected}"
            );
        }
    }
}

/// A ramp longer than a row wraps into a second block, and the block's tail is the last stop
/// again: the stop count the shader takes from the texture's size then changes no colour,
/// below the last offset or on it.
#[test]
fn a_ramp_past_a_row_wraps_into_blocks_and_ends_on_its_last_stop() {
    let count = RAMP_WIDTH as usize + 3;
    let stops: Vec<Stop> = (0..count)
        .map(|k| {
            let at = k as f32 / (count - 1) as f32;
            let colour = if k + 1 == count {
                BLUE
            } else {
                Color::new(at, 0.0, 1.0 - at, 1.0)
            };
            stop(at, colour)
        })
        .collect();
    let table = ramp_table(&stops);
    assert_eq!((table.width, table.height), (RAMP_WIDTH, 2 * RAMP_ROWS));
    for t in [0.0, 0.1, 0.5, 0.999, 0.999_9, 1.0, 2.0] {
        assert_eq!(
            bits(colour_for(&table, t)),
            bits(ramp_color_at(&stops, t)),
            "at {t}"
        );
    }
    assert!(MAX_RAMP_STOPS >= RAMP_WIDTH as usize * (RAMP_WIDTH / RAMP_ROWS) as usize);
}

/// An empty ramp is refused at upload (`ResourceProblem::RampEmpty`), and the table still
/// answers rather than describing a texture of no texels: one transparent stop.
#[test]
fn an_empty_ramp_is_one_transparent_stop() {
    let table = ramp_table(&[]);
    assert_eq!((table.width, table.height), (1, RAMP_ROWS));
    assert_eq!(colour_for(&table, 0.5), Color::new(0.0, 0.0, 0.0, 0.0));
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
