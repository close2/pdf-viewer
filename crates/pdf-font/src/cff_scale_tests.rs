//! Technical Note 5176 Table 9's matrix applies to both outlines and advances.

use super::{advances, draw, int5, nominal_width, units_per_em};
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
fn font(matrix: &[u8]) -> Vec<u8> {
    font_drawing(matrix, &[28, 2, 137, 139, 139, 21, 239, 6, 14])
}

/// A one-glyph CFF under `matrix` whose one charstring is `charstring`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the one-glyph fixture is under a hundred bytes"
)]
fn font_drawing(matrix: &[u8], charstring: &[u8]) -> Vec<u8> {
    let mut top = matrix.to_vec();
    // A five-byte offset keeps the Top DICT length independent of the offset's value.
    let charstrings_at = 4 + index(b"F").len() + 5 + top.len() + 6 + 4;
    top.extend_from_slice(&int5(i64::try_from(charstrings_at).expect("small font")));
    top.push(17);
    let mut bytes = vec![1, 0, 4, 4];
    bytes.extend_from_slice(&index(b"F"));
    bytes.extend_from_slice(&index(&top));
    bytes.extend_from_slice(&[0, 0, 0, 0]); // empty strings and global subroutines
    bytes.extend_from_slice(&index(charstring));
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

#[test]
fn a_fractional_width_is_read_as_the_charstring_states_it() {
    // Type 2 operand 255 is a 16.16 fixed number: 0x0308_E3D7 is 776.89 to within 2^-16. A
    // reader that rounds it reads 777, which ISO 19005-2 6.2.11.5 would compare as agreeing
    // with a /Widths entry of 777 that the program contradicts.
    let data = font_drawing(&[], &[255, 3, 8, 227, 215, 139, 139, 21, 239, 6, 14]);
    let advance = advances(&data, &[0]).expect("font reads")[0].expect("width stated");
    assert!((advance - 776.89).abs() < 1e-3, "read {advance}");
    assert!((advance - 777.0).abs() > 0.1, "read {advance}");
}

#[test]
fn every_shipped_substitute_states_its_widths_as_the_rounding_reader_reads_them() {
    // The viewer lays a substituted font out by these widths (`metrics::program_widths`), so
    // reading fractions where the outline reader rounded moves nothing it draws only if every
    // width the shipped programs state is whole: each glyph of each is compared here.
    struct NoOutline;
    impl OutlinePen for NoOutline {
        fn move_to(&mut self, _: f32, _: f32) {}
        fn line_to(&mut self, _: f32, _: f32) {}
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {}
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
        fn close(&mut self) {}
    }
    let shipped: [&[u8]; 10] = [
        include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitSerifBold.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitSerifItalic.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitSerifBoldItalic.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitFixed.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitFixedBold.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitFixedItalic.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitFixedBoldItalic.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitSymbol.pfb"),
        include_bytes!("../../../data/standard-fonts/FoxitDingbats.pfb"),
    ];
    for data in shipped {
        let font = super::open(data).expect("a shipped program opens");
        let glyphs: Vec<u16> = (0..font.num_glyphs())
            .map(|glyph| u16::try_from(glyph).expect("a CFF holds at most 65535 glyphs"))
            .collect();
        let read = advances(data, &glyphs).expect("a shipped program reads");
        for (glyph, advance) in glyphs.iter().zip(read) {
            let id = skrifa::GlyphId::from(*glyph);
            let subfont = font
                .subfont(font.subfont_index(id).expect("a subfont"), &[])
                .expect("the subfont reads");
            let rounded = font
                .draw(&subfont, id, &[], None, &mut NoOutline)
                .expect("the glyph draws");
            assert_eq!(
                advance.map(f32::to_bits),
                rounded.map(f32::to_bits),
                "glyph {glyph}"
            );
        }
    }
}

#[test]
fn a_real_nominal_width_is_floored_as_the_evaluator_adds_it() {
    // Technical Note 5176 Table 23: nominalWidthX (operator 21) is a number. 495.5 as a real
    // (operand 30, nibbles 4 9 5 . 5 end) and 495 as an integer both read 495, the floor the
    // charstring evaluator adds to a width.
    assert_eq!(nominal_width(&[30, 0x49, 0x5a, 0x5f, 21]), 495);
    assert_eq!(nominal_width(&[28, 0x01, 0xef, 21]), 495);
    assert_eq!(nominal_width(&[]), 0);
}
