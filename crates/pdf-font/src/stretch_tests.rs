//! `stretch`: each expected value is the window's own figure, ADR 0358's `x × scale` about the
//! glyph's origin, read through the same outline reader the window uses.

use super::{StretchError, composite, drawn, proved, sfnt_outlines};
use crate::sfnt::{be16, sfnt_tables};

/// The four shipped Liberation Sans faces, the `glyf` programs this reader substitutes.
const FACES: [(&str, &[u8]); 4] = [
    (
        "regular",
        include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf"),
    ),
    (
        "bold",
        include_bytes!("../../../data/standard-fonts/LiberationSans-Bold.ttf"),
    ),
    (
        "italic",
        include_bytes!("../../../data/standard-fonts/LiberationSans-Italic.ttf"),
    ),
    (
        "bold italic",
        include_bytes!("../../../data/standard-fonts/LiberationSans-BoldItalic.ttf"),
    ),
];

/// The scale `bug1671312_ArialNarrow.pdf`'s widths give Liberation Sans (ADR 0358's witness).
const NARROW: f32 = 0.8201;

fn glyph_count(program: &[u8]) -> u16 {
    let tables = sfnt_tables(program).expect("a directory");
    let (maxp, _) = tables[b"maxp".as_slice()];
    be16(program, maxp.checked_add(4).expect("in range")).expect("numGlyphs")
}

/// Every glyph of every shipped face, composites included, draws `x × scale` within a unit of
/// the em and the same y, at the same advance; the function's own read-back is what holds it,
/// so this asks it of all four faces and then checks one letter by hand.
#[test]
fn every_shipped_glyf_face_draws_its_outlines_at_the_windows_scale() {
    for (name, face) in FACES {
        let scaled = sfnt_outlines(face, NARROW).unwrap_or_else(|why| panic!("{name}: {why}"));
        let glyphs = glyph_count(face);
        assert_eq!(glyph_count(&scaled), glyphs, "{name}");
        // `loca` is written long: ISO/IEC 14496-22's `indexToLocFormat` 1.
        let tables = sfnt_tables(&scaled).expect("a directory");
        let (head, _) = tables[b"head".as_slice()];
        assert_eq!(
            be16(&scaled, head.checked_add(50).expect("in range")),
            Some(1),
            "{name}"
        );
    }
    // `H` of the regular face, point by point: the window's `x × 0.8201`, rounded to the
    // program's unit, and its y untouched.
    let (_, regular) = FACES[0];
    let scaled = sfnt_outlines(regular, NARROW).expect("the regular face");
    let glyph = h_glyph(regular);
    let before = drawn(regular, glyph).expect("H draws");
    let after = drawn(&scaled, glyph).expect("H still draws");
    assert_eq!(before.0.len(), after.0.len());
    for ((x0, y0), (x1, y1)) in before.0.iter().zip(&after.0) {
        assert_eq!(y0.to_bits(), y1.to_bits());
        assert!((x0 * NARROW - x1).abs() <= 0.5, "{x0} became {x1}");
    }
}

/// The glyph `H` reaches through the face's own `cmap`.
fn h_glyph(face: &[u8]) -> u16 {
    use skrifa::MetadataProvider as _;
    let font = skrifa::FontRef::new(face).expect("the face opens");
    font.charmap()
        .map('H')
        .expect("H is mapped")
        .to_u32()
        .try_into()
        .expect("a u16 glyph")
}

/// ADR 0358 condenses and never expands, and refuses a ratio below one half: a scale outside
/// that is one this reader never draws a face at, so it is no picture of the window's.
#[test]
fn a_scale_the_window_never_draws_is_refused() {
    let (_, regular) = FACES[0];
    for scale in [1.0, 1.2, 0.49, 0.0, f32::NAN] {
        assert!(
            matches!(sfnt_outlines(regular, scale), Err(StretchError::Scale(_))),
            "{scale}"
        );
    }
}

/// The plant for the read-back: a program whose outlines were not scaled is not proved to be
/// the scaled one, and the first glyph with an outline is named.
#[test]
fn the_read_back_refuses_an_outline_that_was_not_scaled() {
    let (_, regular) = FACES[0];
    let glyphs = glyph_count(regular);
    assert!(matches!(
        proved(regular, regular, NARROW, glyphs),
        Err(StretchError::NotProved { .. })
    ));
    let scaled = sfnt_outlines(regular, NARROW).expect("the regular face");
    assert_eq!(proved(regular, &scaled, NARROW, glyphs), Ok(()));
}

/// A composite's two-by-two is conjugated by the scale, `S·T·S⁻¹`: the term adding y into x is
/// multiplied by it and the term adding x into y divided by it, and the x offset scaled. A
/// component `[1 0.5; 0 1]` (x gains half of y) with offset (100, 7) at a scale of one half
/// becomes `[1 0.25; 0 1]`, offset (50, 7): the window's `S·(T·p + o)` for p = (2, 4) is
/// (2, 4) + (100, 7) → x = (2 + 2 + 100) / 2 = 52, and the written one gives 1 + 0.25·4 + 50 = 52.
#[test]
fn a_composites_offset_and_cross_terms_are_scaled_with_it() {
    // Header, then one component: flags (words, xy, two-by-two), glyph 3, dx 100, dy 7,
    // xscale 1.0, scale01 0.0, scale10 0.5, yscale 1.0 in F2Dot14.
    let mut glyph = vec![0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0];
    glyph.extend_from_slice(&0x0083u16.to_be_bytes());
    glyph.extend_from_slice(&3u16.to_be_bytes());
    glyph.extend_from_slice(&100i16.to_be_bytes());
    glyph.extend_from_slice(&7i16.to_be_bytes());
    for value in [0x4000i16, 0, 0x2000, 0x4000] {
        glyph.extend_from_slice(&value.to_be_bytes());
    }
    let mut out = glyph[..10].to_vec();
    composite(&glyph, &mut out, 0.5, 9).expect("the component reads");
    let field = |at: usize| i16::from_be_bytes(be16(&out, at).expect("a field").to_be_bytes());
    assert_eq!(field(14), 50, "the x offset");
    assert_eq!(field(16), 7, "the y offset");
    assert_eq!(field(18), 0x4000, "xscale");
    assert_eq!(field(20), 0, "x into y, divided");
    assert_eq!(field(22), 0x1000, "y into x, multiplied");
    assert_eq!(field(24), 0x4000, "yscale");
}

/// A narrowing never lengthens an offset, so one stated in bytes stays in bytes, rounded to
/// the nearest unit.
#[test]
fn a_byte_offset_stays_a_byte() {
    let mut glyph = vec![0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0];
    glyph.extend_from_slice(&0x0002u16.to_be_bytes());
    glyph.extend_from_slice(&3u16.to_be_bytes());
    glyph.extend_from_slice(&[0x80, 5]);
    let mut out = glyph[..10].to_vec();
    // -128 at 0.999 is -127.872, which rounds to -128: still a byte.
    composite(&glyph, &mut out, 0.999, 9).expect("the component reads");
    assert_eq!(out[10..], [0, 2, 0, 3, 0x80, 5]);
}
