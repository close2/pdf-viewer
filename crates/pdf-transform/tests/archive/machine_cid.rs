//! A composite font the window drew from a face of this machine's, embedded with that face where
//! the profile withholds the loss `machine-face` (`doc/adr/2807`, `doc/adr/2774`).
//!
//! The expected values come from ISO 32000-2: Table 116 (`UniJIS-UCS2-H` maps U+3042 and U+3044
//! to CIDs 843 and 845 of `Adobe-Japan1`), Table 124 (a TrueType program goes under `/FontFile2`)
//! and Table 117 (beneath a `CIDFontType2`), §9.7.4.2 and Table 115 (a Type 2 `CIDFont`'s
//! `/CIDToGIDMap` stream holds the glyph index for CID *c* in bytes 2*c* and 2*c* + 1, high byte
//! first, and is what maps its CIDs to the embedded program's glyphs); from ISO 19005-2 section
//! 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 (the program of every rendered font is
//! embedded); and from ADR 2773's reading that the program which keeps the page as the window
//! showed it is the face the window drew.
//!
//! **Which faces a machine has is a fact of the machine**, so the test asks the viewer first,
//! through `pdf_font::LoadedFont` and not through the conversion, whether it draws the fixture's
//! font from a face of this machine's, and says so where it does not.

use super::cjk_pack::a_japanese_font_not_embedded;
use super::*;

/// The fixture's font `F1` (object 6) as the viewer loads it, with the codes the page shows.
fn loaded(document: &Document) -> (pdf_font::LoadedFont, Vec<pdf_font::Code>) {
    let font = document.get(ObjectId::new(6, 0));
    let font = font.as_dict().expect("object 6 is the font");
    let loaded = pdf_font::LoadedFont::load(document, font, "F1").expect("the font loads");
    let codes = loaded.decode(&[0x30, 0x42, 0x30, 0x44]);
    (loaded, codes)
}

/// Whether the viewer draws the fixture's composite font from a face of this machine's: a
/// substitute that is none of the faces this program ships.
pub(super) fn drawn_from_a_machine_face(source: &[u8]) -> bool {
    let document = Document::open_with_limits(source.to_vec(), Limits::DEFAULT).expect("it opens");
    let (font, _) = loaded(&document);
    let Some(program) = font.substitute_program() else {
        return false;
    };
    let base = pdf_font::substitute::Request::derive(
        &document,
        &pdf_syntax::object::Dictionary::new(),
        None,
    );
    let families = [
        pdf_font::substitute::Family::Serif,
        pdf_font::substitute::Family::SansSerif,
        pdf_font::substitute::Family::Monospace,
        pdf_font::substitute::Family::Symbol,
        pdf_font::substitute::Family::ZapfDingbats,
    ];
    !families.into_iter().any(|family| {
        [(false, false), (true, false), (false, true), (true, true)]
            .into_iter()
            .any(|(bold, italic)| {
                let asked = pdf_font::substitute::Request {
                    family,
                    bold,
                    italic,
                    ..base
                };
                pdf_font::standard::face(asked).0 == program
            })
    })
}

/// One shown code as the viewer draws it: its outline, wound one way, and the bits of its advance.
type Drawn = (Option<Vec<pdf_render::PathCommand>>, u32);

/// The outline and advance the viewer draws each shown code with, from `bytes`, and whether the
/// font is a substitute.
fn drawn(bytes: &[u8]) -> (Vec<Drawn>, bool) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let (font, codes) = loaded(&document);
    let drawn = codes
        .iter()
        .map(|code| {
            (
                // Wound one way: the viewer winds a substitute's glyphs counter-clockwise and
                // leaves an embedded program's as drawn (ADR 0396).
                font.outline(*code).map(|path| {
                    if path.signed_area() < 0.0 {
                        path.reversed().commands().to_vec()
                    } else {
                        path.commands().to_vec()
                    }
                }),
                font.advance(*code).to_bits(),
            )
        })
        .collect();
    (drawn, font.is_substituted())
}

/// ADR 2807: under every loss withheld, as `as-shown` withholds `machine-face`, the font goes in
/// with the face the window drew — a `CIDFontType2` over a `/FontFile2`, its `/CIDToGIDMap` a
/// stream naming a glyph for CIDs 843 and 845 — loses nothing, is not settled by the second pass,
/// conforms, and draws each shown code's outline at its advance as the window drew it. The plant:
/// a map that sends either CID to another glyph draws another outline, and the assertion on the
/// outlines fails.
#[test]
fn a_collections_font_the_window_drew_from_a_machines_face_is_embedded_with_that_face() {
    let source = a_japanese_font_not_embedded();
    if !drawn_from_a_machine_face(&source) {
        eprintln!("this machine has no face drawing the fixture's characters; nothing to embed");
        return;
    }
    let (before, substituted_before) = drawn(&source);
    assert!(substituted_before, "the window draws a substitute");
    assert!(
        before.iter().all(|(outline, _)| outline.is_some()),
        "the window drew a glyph for each code"
    );
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert(&source, target, Authorisations::default());
        let output = written(&report, output);
        assert!(
            conversion(&report).settled.is_empty(),
            "{target}: the first pass embeds it"
        );
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}: what was written is held to the target again"
        );
        let font = conversion(&report).substituted.first().expect("one font");
        assert_eq!(
            (font.authority, font.losses),
            (
                pdf_transform::archive::FaceAuthority::Machine,
                Losses::default()
            ),
            "{target}: the machine's face, and nothing lost"
        );
        let held = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
        let descendant = held
            .resolve(
                held.get_key(&font_resource(&held), "DescendantFonts")
                    .as_array()
                    .and_then(|array| array.first())
                    .expect("§9.7.6.2's one-element array"),
            )
            .as_dict()
            .cloned()
            .expect("the descendant");
        assert_eq!(
            held.get_key(&descendant, "Subtype")
                .as_name()
                .map(|name| name.as_bytes().to_vec()),
            Some(b"CIDFontType2".to_vec()),
            "{target}: Table 124 puts a TrueType program beneath a CIDFontType2"
        );
        let descriptor = held
            .get_key(&descendant, "FontDescriptor")
            .as_dict()
            .cloned()
            .expect("its descriptor");
        assert!(
            matches!(
                held.get_key(&descriptor, "FontFile2"),
                pdf_syntax::Object::Stream(_)
            ),
            "{target}: the program is under /FontFile2"
        );
        let pdf_syntax::Object::Stream(map) = held.get_key(&descendant, "CIDToGIDMap") else {
            panic!("{target}: Table 115's /CIDToGIDMap is a stream");
        };
        let map = held.decoded_stream_data(&map).expect("the map decodes");
        for cid in [843_usize, 845] {
            let glyph = u16::from_be_bytes([map[2 * cid], map[2 * cid + 1]]);
            assert_ne!(
                glyph, 0,
                "{target}: CID {cid} reaches a glyph of the program"
            );
        }
        let (after, substituted_after) = drawn(&output);
        assert!(!substituted_after, "{target}: drawn from its own program");
        assert_eq!(
            before, after,
            "{target}: the output draws what the window drew, code by code"
        );
    }
}

/// ADR 2774: where the profile authorises `machine-face`, as `as-if-printed` does, this route is
/// not taken: the font keeps the route it took before, which is never the machine's face. The
/// plant: computing the route's switch as `true` under every profile makes the authority
/// `Machine` here.
#[test]
fn where_the_profile_authorises_the_loss_a_composite_font_keeps_its_route() {
    let source = a_japanese_font_not_embedded();
    if !drawn_from_a_machine_face(&source) {
        eprintln!("this machine has no face drawing the fixture's characters");
        return;
    }
    let (report, _) = convert(&source, Target::Two(Level::B), the_face_losses_authorised());
    assert!(
        conversion(&report)
            .substituted
            .iter()
            .all(|font| font.authority != pdf_transform::archive::FaceAuthority::Machine),
        "as-if-printed keeps the face every machine would write (doc/adr/2774)"
    );
}
