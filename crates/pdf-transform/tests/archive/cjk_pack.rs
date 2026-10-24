//! A composite font of an Adobe character collection the file does not embed, and the CJK pack.
//!
//! The expected values come from ISO 32000-2: §9.7.2 (a collection's order numbers its glyphs, so
//! a CID of `Adobe-Japan1` names one glyph in every font of it), §9.7.5.2's Table 116 (`UniJIS-
//! UCS2-H` maps U+3042 to CID 843 of `Adobe-Japan1`, by Adobe's published file the tree carries),
//! Table 124 (a bare CID-keyed CFF program goes under `/FontFile3` with `/Subtype
//! /CIDFontType0C`, beneath a `CIDFontType0`) and Table 117 (that subtype's `/BaseFont` is the
//! program's own name); and from ADR 2162's reading that the pack's face standing in for the
//! producer's is the loss `machine-face`, which `as-if-printed` discards and `as-shown` does not.
use std::sync::Arc;

use super::{
    Conforming, Decision, Level, Target, a_supplied_font, conversion, convert_with_plan, decision,
    font_resource,
};
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{Authorisations, CjkPack, Loss, PackFiles};

/// The fixture's one font: `UniJIS-UCS2-H` over an `Adobe-Japan1` descendant with no program,
/// showing U+3042 (CID 843) and U+3044 (CID 845).
fn a_japanese_font_not_embedded() -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <30423044> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /HeiseiKakuGo-W5-UniJIS-UCS2-H \
             /Encoding /UniJIS-UCS2-H /DescendantFonts [8 0 R] >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /HeiseiKakuGo-W5 /Flags 4 \
             /FontBBox [-92 -250 1010 922] /ItalicAngle 0 /Ascent 752 /Descent -221 \
             /CapHeight 737 /StemV 114 >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /HeiseiKakuGo-W5 \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> \
             /FontDescriptor 7 0 R /DW 1000 >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// A reader of the pack directory `QUORRA_CJK_PACK` names, or of nothing.
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
        Some(b"OTTO, but not the program this tree's builder writes".to_vec())
    }
}

fn refusal_of(pack: Option<CjkPack>) -> String {
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    plan.cjk_pack = pack;
    let (report, output) = convert_with_plan(&a_japanese_font_not_embedded(), &plan);
    assert!(output.is_none(), "no file is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("a collection's font with no pack program is refused by name");
    };
    because.sentence().to_owned()
}

#[test]
fn a_collections_font_without_the_pack_names_the_pack_as_what_is_missing() {
    // The pack is a second download, and a person who has not made it is told so in the refusal
    // rather than told that no program could ever answer the font.
    for pack in [None, Some(CjkPack::new(Arc::new(Directory(None))))] {
        let sentence = refusal_of(pack);
        assert!(
            sentence.contains("CJK pack") && sentence.contains("not installed"),
            "{sentence}"
        );
    }
}

#[test]
fn a_pack_file_that_is_not_this_trees_program_is_refused_rather_than_embedded() {
    let sentence = refusal_of(Some(CjkPack::new(Arc::new(NotThePack))));
    assert!(sentence.contains("SHA-256"), "{sentence}");
}

#[test]
#[ignore = "needs the CJK pack, a second download that is not part of a checkout: \
            QUORRA_CJK_PACK=<dir> names it (doc/cjk-pack.md)"]
fn the_pack_answers_a_collections_font_as_the_named_loss_machine_face() {
    let directory = std::env::var_os("QUORRA_CJK_PACK")
        .expect("QUORRA_CJK_PACK names the directory the pack was unpacked into");
    let pack = CjkPack::new(Arc::new(Directory(Some(directory.into()))));
    let source = a_japanese_font_not_embedded();

    // `as-shown`'s yardstick: the window drew another face, so the loss is not admitted.
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    plan.cjk_pack = Some(pack.clone());
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(
        output.is_none(),
        "the loss nobody authorised stops the conversion"
    );
    let Decision::Unauthorised { losses, .. } = decision(&report, "fonts/font-programs-embedded")
    else {
        panic!("the pack's face is a loss to authorise, not a refusal");
    };
    assert!(losses.iter().any(|loss| loss == Loss::MachineFace));

    // `as-if-printed`'s: the loss is discarded, and the font is given the pack's program.
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    plan.authorised = authorised;
    let (report, output) = convert_with_plan(&source, &plan);
    let held = Document::open_with_limits(output.expect("it converts"), Limits::DEFAULT)
        .expect("it opens");
    let descendant = held
        .resolve(
            held.get_key(&font_resource(&held), "DescendantFonts")
                .as_array()
                .and_then(|array| array.first())
                .expect("§9.7.6.2's one-element array"),
        )
        .as_dict()
        .cloned()
        .expect("the descendant CIDFont");
    let name = |dict: &pdf_syntax::object::Dictionary, key: &str| {
        held.get_key(dict, key)
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
    };
    assert_eq!(
        name(&descendant, "Subtype").as_deref(),
        Some("CIDFontType0")
    );
    let base_font = name(&descendant, "BaseFont").expect("a BaseFont");
    assert!(
        base_font.len() == 6 + 1 + "QuorraCJK-Japan1".len()
            && base_font.ends_with("+QuorraCJK-Japan1"),
        "§9.9.2's tag, a plus sign and the program's name, which Table 117 makes the BaseFont: \
         {base_font}"
    );
    let descriptor = held
        .get_key(&descendant, "FontDescriptor")
        .as_dict()
        .cloned()
        .expect("the descriptor");
    assert_eq!(name(&descriptor, "FontName"), Some(base_font));
    let program = held
        .get_key(&descriptor, "FontFile3")
        .as_stream()
        .cloned()
        .expect("Table 124's /FontFile3");
    assert_eq!(
        name(&program.dict, "Subtype").as_deref(),
        Some("CIDFontType0C")
    );
    let metadata = held
        .get_key(&program.dict, "Metadata")
        .as_stream()
        .cloned()
        .expect("the licence's packet");
    let packet = held.decoded_stream_data(&metadata).expect("it decodes");
    assert!(
        String::from_utf8_lossy(&packet).contains("SIL OPEN FONT LICENSE Version 1.1"),
        "the OFL's condition 2: each copy carries the notice and the licence"
    );
    let font = conversion(&report)
        .substituted
        .first()
        .expect("the one font");
    assert!(
        font.losses.iter().any(|loss| loss == Loss::MachineFace),
        "{font:?}"
    );
}
