//! Bounds are conservative relative to individual shader admission, including fused mapping.
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]
use super::{Bounds, LineBlocks, LineView};

fn line(ends: [f32; 4], half: [f32; 2], resolution: u32) -> Bounds {
    let mut bytes = [0_u8; 60];
    let fields = [
        ends[0],
        ends[1],
        ends[2],
        ends[3],
        half[0],
        half[1],
        (resolution * 3) as f32,
    ];
    for (value, destination) in fields.iter().zip(bytes.chunks_exact_mut(4)) {
        destination.copy_from_slice(&value.to_le_bytes());
    }
    Bounds::of(&bytes)
}

#[test]
fn spatial_blocks_and_adjustment() {
    let view = [1., 0., 0., 1., 0., 0., 100., 100.];
    let mut far = line([200., 200., 201., 201.], [0.1; 2], 0);
    assert!(!far.admitted(LineView(view)));
    far = far.union(line([99., 50., 101., 50.], [0.1; 2], 0));
    assert!(far.admitted(LineView(view)));
    let adjusted = line([-0.2, 10., -0.2, 20.], [0.01; 2], 1);
    assert!(adjusted.admitted(LineView(view)));
    let ordinary = line([-0.2, 10., -0.2, 20.], [0.01; 2], 0);
    assert!(!ordinary.admitted(LineView(view)));
    let reflected = [-1., 0., 0., 1., 100., 0., 100., 100.];
    assert!(adjusted.admitted(LineView(reflected)));
    let turned = [0., 1., -1., 0., 100., 0., 100., 100.];
    assert!(adjusted.admitted(LineView(turned)));
}
#[test]
fn conservative_against_individual_shader_admission() {
    let mut state = 73_u64;
    let mut random = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (state >> 32) as u32
    };
    for i in 0..200_000 {
        let scale = [0.00001, 0.1, 1., 3.6, 1000., 1e8][i % 6];
        let mut number = || (random() as f32 / u32::MAX as f32 - 0.5) * 400. * scale;
        let ends = [number(), number(), number(), number()];
        let half = [number().abs() * 0.01, number().abs() * 0.01];
        let resolution = (i % 3) as u32;
        let (a, b, c, d) = if i % 2 == 0 {
            (1. / scale, 0., 0., -1. / scale)
        } else {
            (0., 1. / scale, -1. / scale, 0.)
        };
        let view = [a, b, c, d, 50., 50., 100., 100.];
        let bound = line(ends, half, resolution);
        for fused in [false, true] {
            let map = |x: f32, y: f32, axis: usize| {
                if fused {
                    view[axis].mul_add(x, view[axis + 2].mul_add(y, view[axis + 4]))
                } else {
                    view[axis] * x + view[axis + 2] * y + view[axis + 4]
                }
            };
            let mut pen = [
                half[0] * a.abs() + half[1] * c.abs(),
                half[0] * b.abs() + half[1] * d.abs(),
            ];
            if resolution == 2 {
                pen = half;
            } else if resolution == 1 && 2. * pen[0].max(pen[1]) < 0.5 {
                pen = [0.5; 2];
            }
            let visible = (0..2).all(|axis| {
                let start = map(ends[0], ends[1], axis);
                let end = map(ends[2], ends[3], axis);
                (start.max(end) + pen[axis] * 1.4143).min(100.)
                    > (start.min(end) - pen[axis] * 1.4143).max(0.)
            });
            assert!(
                !visible || bound.admitted(LineView(view)),
                "{i}: {bound:?} {view:?}"
            );
        }
    }
}

#[test]
fn uncertain_arithmetic_keeps_the_block() {
    let view = LineView([1., 0., 0., 1., 0., 0., 100., 100.]);
    assert!(Bounds::UNKNOWN.admitted(view));
    assert!(line([f32::NAN, 200., 201., 201.], [0.1; 2], 0).admitted(view));
    let overflowing = line([200., f32::MAX, 201., f32::MAX], [0.1; 2], 0);
    assert!(overflowing.admitted(LineView([1., 0., 0., 2., 0., 0., 100., 100.])));
    assert!(
        Bounds::UNKNOWN
            .union(line([200.; 4], [0.1; 2], 0))
            .admitted(view)
    );
}

#[test]
fn table_storage_matches_its_prior_charge() {
    for instances in [1, 255, 256, 257, 1024] {
        let stream = vec![0_u8; instances * 60];
        let table = LineBlocks::of(&stream);
        assert_eq!(table.bytes(), LineBlocks::allocation_bytes(stream.len()));
    }
}

#[test]
fn fixed_pens_do_not_rotate_and_cancellation_stays_conservative() {
    let quarter_turn = LineView([0.0, 1.0, -1.0, 0.0, 100.0, 0.0, 100.0, 100.0]);
    let fixed = line([50.0, 110.0, 60.0, 110.0], [20.0, 0.1], 2);
    assert!(fixed.admitted(quarter_turn));
    let scaled = line([50.0, 110.0, 60.0, 110.0], [20.0, 0.1], 0);
    assert!(!scaled.admitted(quarter_turn));
    let cancelled = line([1e20, 50.0, 1e20, 60.0], [0.1; 2], 0);
    assert!(cancelled.admitted(LineView([1.0, 0.0, 0.0, 1.0, -1e20, 0.0, 100.0, 100.0])));
}
