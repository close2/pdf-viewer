//! Technical Note 5176 Table 9's matrix applies to both outlines and advances.

use super::{advances, draw, int5, units_per_em};
use skrifa::outline::OutlinePen;

#[expect(
    clippy::arithmetic_side_effects,
    reason = "the fixture has one small item"
)]
fn index(item: &[u8]) -> Vec<u8> {
    let mut bytes = vec![
        0,
        1,
        1,
        1,
        u8::try_from(item.len() + 1).expect("small fixture"),
    ];
    bytes.extend_from_slice(item);
    bytes
}

/// A one-glyph CFF. Its charstring states a 649-unit advance and a 100-unit line.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the one-glyph fixture is under a hundred bytes"
)]
fn font(matrix: &[u8]) -> Vec<u8> {
    let mut top = matrix.to_vec();
    // A five-byte offset keeps the Top DICT length independent of the offset's value.
    let charstrings_at = 4 + index(b"F").len() + 5 + top.len() + 6 + 4;
    top.extend_from_slice(&int5(i64::try_from(charstrings_at).expect("small font")));
    top.push(17);
    let mut bytes = vec![1, 0, 4, 4];
    bytes.extend_from_slice(&index(b"F"));
    bytes.extend_from_slice(&index(&top));
    bytes.extend_from_slice(&[0, 0, 0, 0]); // empty strings and global subroutines
    bytes.extend_from_slice(&index(&[28, 2, 137, 139, 139, 21, 239, 6, 14]));
    bytes
}

#[derive(Default)]
struct Points(Vec<(f32, f32)>);

impl OutlinePen for Points {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push((x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push((x, y));
    }
    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
        panic!("fixture has no curves");
    }
    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
        panic!("fixture has no curves");
    }
    fn close(&mut self) {}
}

#[test]
fn a_fractional_matrix_scales_the_width_and_the_outline_without_integer_upem_rounding() {
    // BCD 0.00049, 0, 0, 0.00049, 0, 0, FontMatrix (12 7).
    let data = font(&[
        30, 0x0a, 0x00, 0x04, 0x9f, 139, 139, 30, 0x0a, 0x00, 0x04, 0x9f, 139, 139, 12, 7,
    ]);
    let em = units_per_em(&data).expect("matrix reads");
    assert!((1.0 / em - 0.00049).abs() < 1e-10);
    let advance = advances(&data, &[0]).expect("font reads")[0].expect("width stated");
    assert!((advance / em - 0.31801).abs() < 1e-7);
    let mut points = Points::default();
    draw(&data, 0, &mut points).expect("glyph draws");
    assert!(points.0.iter().any(|(x, _)| (x / em - 0.049).abs() < 1e-7));
}

#[test]
fn the_absent_matrix_keeps_the_formats_thousandth_default() {
    assert_eq!(
        units_per_em(&font(&[])).expect("font reads").to_bits(),
        1000.0_f32.to_bits()
    );
}

#[test]
fn a_nonuniform_matrix_keeps_its_horizontal_factor() {
    // FontMatrix [0.00098 0 0 0.00049 0 0]: normalization must retain X twice Y.
    let data = font(&[
        30, 0x0a, 0x00, 0x09, 0x8f, 139, 139, 30, 0x0a, 0x00, 0x04, 0x9f, 139, 139, 12, 7,
    ]);
    let em = units_per_em(&data).expect("matrix reads");
    let advance = advances(&data, &[0]).expect("font reads")[0].expect("width stated");
    assert!((advance / em - 0.63602).abs() < 1e-7);
    let mut points = Points::default();
    draw(&data, 0, &mut points).expect("glyph draws");
    assert!(points.0.iter().any(|(x, _)| (x / em - 0.098).abs() < 1e-7));
}
