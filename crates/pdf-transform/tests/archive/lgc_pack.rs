//! A simple serif font the file does not embed, showing characters the shipped serif face lacks,
//! and the Latin-Greek-Cyrillic pack (ADR 2934).
//!
//! The expected values come from ISO 32000-2 and the two parts: §9.6.5's route from a code to a
//! glyph of a Type 1 font's program is its glyph name, so a `/Differences` array naming `beta`,
//! `aogonek` and `afii10024` asks a program holding those names, which the Foxit serif face does
//! not (ISO 19005-2 section 6.2.11.8 forbids the `.notdef` the shipped face would show); Table
//! 124 puts a bare CFF program under `/FontFile3` with `/Subtype /Type1C` beneath a `Type1`
//! dictionary; and ADR 2162's reading makes a face that is neither the producer's nor the
//! window's the loss `machine-face`, which `as-if-printed` discards.
use std::sync::Arc;

use super::{
    Conforming, Decision, Level, Target, a_supplied_font, conversion, convert_with_plan, decision,
    font_resource,
};
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{Authorisations, LgcPack, Loss, PackFiles};

/// The fixture's one font: `Times-Roman` with a descriptor and no program, its codes 65 to 67
/// naming Greek small beta, Latin small a with ogonek and Cyrillic capital Zhe by the Adobe Glyph
/// List's names.
fn a_serif_font_showing_greek_polish_and_cyrillic() -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (ABC) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /FirstChar 65 /LastChar 67 \
             /Widths [500 444 900] /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding \
             /Differences [65 /beta /aogonek /afii10024] >> /FontDescriptor 7 0 R >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /Times-Roman /Flags 34 \
             /FontBBox [-168 -218 1000 898] /ItalicAngle 0 /Ascent 683 /Descent -217 \
             /CapHeight 662 /StemV 84 >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// A reader of the pack directory `QUORRA_LGC_PACK` names, or of nothing.
#[derive(Debug)]
struct Directory(Option<std::path::PathBuf>);

impl PackFiles for Directory {
    fn read(&self, file: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.as_ref()?.join(file)).ok()
    }
}

/// A reader whose every file holds the same bytes, none of them a pack program.
#[derive(Debug)]
struct NotThePack;

impl PackFiles for NotThePack {
    fn read(&self, _: &str) -> Option<Vec<u8>> {
        Some(b"a bare CFF, but not the program this tree's builder writes".to_vec())
    }
}

/// The plan `as-if-printed` would make of the fixture's font: `machine-face` authorised, so that
/// no face of this machine's is embedded ahead of the shipped face and the pack (`doc/adr/2774`).
fn plan_with(pack: Option<LgcPack>) -> pdf_transform::archive::ArchivePlan {
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    plan.lgc_pack = pack;
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    plan.authorised = authorised;
    plan
}

/// The fixture's refusal under `plan`.
fn refusal_of(pack: Option<LgcPack>) -> String {
    let (report, output) = convert_with_plan(
        &a_serif_font_showing_greek_polish_and_cyrillic(),
        &plan_with(pack),
    );
    assert!(output.is_none(), "no file is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("a code no installed face draws is refused by name");
    };
    because.sentence().to_owned()
}

#[test]
fn without_the_pack_the_shipped_faces_refusal_names_the_pack() {
    for pack in [None, Some(LgcPack::new(Arc::new(Directory(None))))] {
        let sentence = refusal_of(pack);
        assert!(
            sentence.contains("Latin-Greek-Cyrillic pack") && sentence.contains(".notdef"),
            "{sentence}"
        );
    }
}

#[test]
fn a_pack_file_that_is_not_this_trees_program_is_refused_rather_than_embedded() {
    let sentence = refusal_of(Some(LgcPack::new(Arc::new(NotThePack))));
    assert!(sentence.contains("SHA-256"), "{sentence}");
}

#[test]
#[ignore = "needs the Latin-Greek-Cyrillic pack, a download that is not part of a checkout: \
            QUORRA_LGC_PACK=<dir> names it (doc/lgc-pack.md)"]
fn the_pack_draws_what_the_shipped_serif_face_cannot_as_the_named_loss_machine_face() {
    let directory = std::env::var_os("QUORRA_LGC_PACK")
        .expect("QUORRA_LGC_PACK names the directory the pack was unpacked into");
    let pack = LgcPack::new(Arc::new(Directory(Some(directory.into()))));
    let (report, output) = convert_with_plan(
        &a_serif_font_showing_greek_polish_and_cyrillic(),
        &plan_with(Some(pack)),
    );
    let held = Document::open_with_limits(output.expect("it converts"), Limits::DEFAULT)
        .expect("it opens");
    let font = font_resource(&held);
    let descriptor = held
        .get_key(&font, "FontDescriptor")
        .as_dict()
        .cloned()
        .expect("the descriptor");
    let program = held
        .get_key(&descriptor, "FontFile3")
        .as_stream()
        .cloned()
        .expect("Table 124's /FontFile3");
    assert_eq!(
        held.get_key(&program.dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Type1C".to_vec())
    );
    let metadata = held
        .get_key(&program.dict, "Metadata")
        .as_stream()
        .cloned()
        .expect("the licence's packet");
    let packet = held.decoded_stream_data(&metadata).expect("it decodes");
    assert!(
        String::from_utf8_lossy(&packet).contains("The Noto Project Authors"),
        "the OFL's condition 2: each copy carries the notice and the licence"
    );
    // Read back by this reader, each of the three codes reaches a glyph that is not `.notdef`, at
    // the width the file states.
    let loaded = pdf_font::LoadedFont::load(&held, &font, "F1").expect("it loads");
    assert!(!loaded.is_substituted(), "the embedded program is drawn");
    for (code, width) in [(65_u8, 500.0_f32), (66, 444.0), (67, 900.0)] {
        let code = pdf_font::Code::single_byte(code);
        assert!(
            loaded
                .glyph_index(code)
                .is_some_and(|glyph| glyph != pdf_font::NOTDEF_GLYPH),
            "{code:?}"
        );
        assert!(
            (loaded.advance(code) * 1000.0 - width).abs() < 0.01,
            "{code:?}"
        );
    }
    let substituted = conversion(&report)
        .substituted
        .first()
        .expect("the one font");
    assert!(
        substituted.face.starts_with("Noto Serif Regular")
            && substituted
                .losses
                .iter()
                .any(|loss| loss == Loss::MachineFace),
        "{substituted:?}"
    );
}
