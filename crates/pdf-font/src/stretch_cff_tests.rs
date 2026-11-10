//! The fitting written into whole CFF faces (`doc/adr/2789`): the compiled-in bare CFF faces, and
//! the OpenType CFF faces of this machine's URW collection where it has one.

use super::{StretchError, cff_outlines, drawn, drawn_alike, proved};

/// ADR 0358's scale for a face fitted to narrower widths, as the `glyf` tests use it.
const NARROW: f32 = 0.8201;

/// The shipped bare CFF faces (each `.pfb` here is a CFF program; `PROVENANCE.md`).
const SHIPPED: [(&str, &[u8]); 10] = [
    (
        "FoxitSerif",
        include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb"),
    ),
    (
        "FoxitSerifBold",
        include_bytes!("../../../data/standard-fonts/FoxitSerifBold.pfb"),
    ),
    (
        "FoxitSerifItalic",
        include_bytes!("../../../data/standard-fonts/FoxitSerifItalic.pfb"),
    ),
    (
        "FoxitSerifBoldItalic",
        include_bytes!("../../../data/standard-fonts/FoxitSerifBoldItalic.pfb"),
    ),
    (
        "FoxitFixed",
        include_bytes!("../../../data/standard-fonts/FoxitFixed.pfb"),
    ),
    (
        "FoxitFixedBold",
        include_bytes!("../../../data/standard-fonts/FoxitFixedBold.pfb"),
    ),
    (
        "FoxitFixedItalic",
        include_bytes!("../../../data/standard-fonts/FoxitFixedItalic.pfb"),
    ),
    (
        "FoxitFixedBoldItalic",
        include_bytes!("../../../data/standard-fonts/FoxitFixedBoldItalic.pfb"),
    ),
    (
        "FoxitSymbol",
        include_bytes!("../../../data/standard-fonts/FoxitSymbol.pfb"),
    ),
    (
        "FoxitDingbats",
        include_bytes!("../../../data/standard-fonts/FoxitDingbats.pfb"),
    ),
];

/// `cff`, a bare CFF program of 1000 units to the em, inside the smallest OpenType wrapper the
/// reader opens: `CFF `, `head`, `hhea`, `hmtx` (an advance of 1000 and a bearing of 0 for every
/// glyph: the writer copies advances, and nothing here reads them) and
/// `maxp` (ISO/IEC 14496-22's version 0.5, which a CFF face uses).
fn opentype_around(cff: &[u8]) -> Vec<u8> {
    let glyphs = u16::try_from(crate::cff::glyph_count(cff).expect("a count")).expect("u16");
    let mut head = vec![0u8; 54];
    head[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    let mut hhea = vec![0u8; 36];
    hhea[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    hhea[34..36].copy_from_slice(&glyphs.to_be_bytes());
    let mut hmtx = Vec::new();
    for _ in 0..glyphs {
        hmtx.extend_from_slice(&1000u16.to_be_bytes());
        hmtx.extend_from_slice(&0i16.to_be_bytes());
    }
    let mut maxp = 0x0000_5000u32.to_be_bytes().to_vec();
    maxp.extend_from_slice(&glyphs.to_be_bytes());
    let tables: [(&[u8; 4], Vec<u8>); 5] = [
        (b"CFF ", cff.to_vec()),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"maxp", maxp),
    ];
    let mut out = b"OTTO".to_vec();
    out.extend_from_slice(&5u16.to_be_bytes());
    out.extend_from_slice(&[0, 64, 0, 2, 0, 16]);
    let directory = tables.len().saturating_mul(16).saturating_add(12);
    let mut at = directory;
    let mut data = Vec::new();
    for (tag, table) in &tables {
        out.extend_from_slice(*tag);
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&u32::try_from(at).expect("offset").to_be_bytes());
        out.extend_from_slice(&u32::try_from(table.len()).expect("length").to_be_bytes());
        data.extend_from_slice(table);
        while !data.len().is_multiple_of(4) {
            data.push(0);
        }
        at = directory.saturating_add(data.len());
    }
    out.extend_from_slice(&data);
    out
}

/// Every glyph of every shipped CFF face, in an OpenType wrapper, is stated again and read back:
/// the same commands, each y the same, each x within half a unit of the window's `x × scale` (the
/// rounding of one point; these faces place no accent), a line the rounding collapses aside.
#[test]
fn every_shipped_cff_face_draws_its_outlines_at_the_windows_scale() {
    for (name, face) in SHIPPED {
        let face = opentype_around(face);
        let scaled = cff_outlines(&face, NARROW).unwrap_or_else(|why| panic!("{name}: {why}"));
        let glyphs = crate::cff::glyph_count(shipped_cff(name)).expect("a glyph count");
        for glyph in 0..u16::try_from(glyphs).expect("a u16 count") {
            let (before, after) = match (drawn(&face, glyph), drawn(&scaled, glyph)) {
                (Some(before), Some(after)) => (before, after),
                (None, None) => continue,
                _ => panic!("{name} glyph {glyph}: drawn on one side only"),
            };
            // Points that stand alike, command for command, are each within half a unit; a
            // line the writer did not state is the only other difference `drawn_alike` admits.
            let lined_up = before.0.len() == after.0.len();
            assert!(drawn_alike(&before, &after, NARROW), "{name} glyph {glyph}");
            if lined_up {
                for (was, now) in before.0.iter().zip(&after.0) {
                    assert!(
                        (was.1 * NARROW - now.1).abs() <= 0.5 + 1.0 / 1024.0,
                        "{name} glyph {glyph}: {} became {}",
                        was.1,
                        now.1
                    );
                }
            }
        }
    }
}

/// The bare program of a shipped face, by name.
fn shipped_cff(name: &str) -> &'static [u8] {
    SHIPPED
        .iter()
        .find(|(shipped, _)| *shipped == name)
        .map(|(_, face)| *face)
        .expect("a shipped face")
}

/// The plant for the read-back: a face whose outlines were not scaled is not proved to be the
/// scaled one. A scale the window never draws at is refused before anything is written, and so is
/// a bare CFF program, whose reader here draws a coordinate's whole part only.
#[test]
fn the_read_back_refuses_a_cff_outline_that_was_not_scaled() {
    let (_, serif) = SHIPPED[0];
    let face = opentype_around(serif);
    let glyphs = u16::try_from(crate::cff::glyph_count(serif).expect("a count")).expect("u16");
    assert!(matches!(
        proved(&face, &face, NARROW, glyphs),
        Err(StretchError::NotProved { .. })
    ));
    for scale in [1.0, 0.49, f32::NAN] {
        assert!(matches!(
            cff_outlines(&face, scale),
            Err(StretchError::Scale(_))
        ));
    }
    assert!(matches!(
        cff_outlines(serif, NARROW),
        Err(StretchError::Unreadable(_))
    ));
}

/// The faces of this machine's URW collection, OpenType with a `CFF ` table, which the viewer
/// draws a font the file names and does not embed from (ADR 0270): every face is written at the
/// scale, its `CFF ` table proved glyph by glyph inside [`cff_outlines`], and its advances read
/// back unchanged through the OpenType reader. A machine without the collection says so.
#[test]
fn every_urw_face_on_this_machine_takes_its_fitting() {
    use skrifa::MetadataProvider as _;
    let directory = std::path::Path::new("/usr/share/fonts/opentype/urw-base35");
    let Ok(entries) = std::fs::read_dir(directory) else {
        eprintln!(
            "{} is not on this machine; nothing to write",
            directory.display()
        );
        return;
    };
    let mut faces = 0;
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|extension| extension != "otf") {
            continue;
        }
        let face = std::fs::read(&path).expect("the face reads");
        let scaled =
            cff_outlines(&face, NARROW).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
        let advances = |program: &[u8]| -> Vec<Option<f32>> {
            let font = skrifa::FontRef::new(program).expect("it opens");
            let metrics = font.glyph_metrics(
                skrifa::instance::Size::unscaled(),
                skrifa::instance::LocationRef::default(),
            );
            let count = skrifa::raw::TableProvider::maxp(&font)
                .expect("maxp")
                .num_glyphs();
            (0..count)
                .map(|glyph| metrics.advance_width(skrifa::GlyphId::from(glyph)))
                .collect()
        };
        assert_eq!(advances(&face), advances(&scaled), "{}", path.display());
        faces += 1;
    }
    eprintln!("{faces} URW faces written at the scale {NARROW}");
}

/// The one difference the read-back admits: a line whose two ends round to one point, which the
/// writer does not state. A move to 11 and a line to 12, at a scale of one half, are 5.5 and 6,
/// both 6 once rounded, so the line is missing and the outline is the same; a line that moves y,
/// or a curve, is not admitted missing (the plant would let any command go).
#[test]
fn only_a_line_that_rounds_to_no_length_may_be_missing() {
    use super::{CURVE, Commands, LINE, MOVE};
    let before = |y: f32| Commands(vec![(MOVE, 11.0, 0.0), (LINE, 12.0, y), (LINE, 100.0, y)]);
    let after = |y: f32| Commands(vec![(MOVE, 6.0, 0.0), (LINE, 50.0, y)]);
    assert!(drawn_alike(&before(0.0), &after(0.0), 0.5));
    assert!(!drawn_alike(&before(1.0), &after(1.0), 0.5));
    let curved = Commands(vec![
        (MOVE, 11.0, 0.0),
        (CURVE, 11.0, 0.0),
        (CURVE, 12.0, 0.0),
        (CURVE, 12.0, 0.0),
        (LINE, 100.0, 0.0),
    ]);
    assert!(!drawn_alike(&curved, &after(0.0), 0.5));
}
