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

/// The pack's reader, for the tests that need a built pack.
fn the_pack() -> CjkPack {
    let directory = std::env::var_os("QUORRA_CJK_PACK")
        .expect("QUORRA_CJK_PACK names the directory the pack was unpacked into");
    CjkPack::new(Arc::new(Directory(Some(directory.into()))))
}

/// `source` converted to PDF/A-2b with the pack and the loss `machine-face` authorised, opened.
fn converted_with_the_pack(source: &[u8]) -> Document {
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    plan.cjk_pack = Some(the_pack());
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    plan.authorised = authorised;
    let (report, output) = convert_with_plan(source, &plan);
    let Some(output) = output else {
        panic!(
            "it converts: {:?}",
            decision(&report, "fonts/font-programs-embedded")
        );
    };
    Document::open_with_limits(output, Limits::DEFAULT).expect("it opens")
}

/// Each descendant `CIDFont` the converted file holds, by object number, with its descriptor
/// reference as the dictionary states it and the descriptor it resolves to.
fn descendants_and_descriptors(
    held: &Document,
) -> Vec<(
    pdf_syntax::object::Dictionary,
    Option<pdf_syntax::object::ObjectId>,
    pdf_syntax::object::Dictionary,
)> {
    held.xref()
        .object_numbers()
        .filter_map(|number| {
            let object = held.resolve(&pdf_syntax::Object::Reference(
                pdf_syntax::object::ObjectId::new(number, 0),
            ));
            let dict = object.as_dict()?.clone();
            let subtype = held.get_key(&dict, "Subtype");
            (subtype.as_name()?.as_bytes() == b"CIDFontType0").then_some(dict)
        })
        .map(|descendant| {
            let stated = descendant
                .get("FontDescriptor")
                .and_then(pdf_syntax::Object::as_reference);
            let descriptor = held
                .get_key(&descendant, "FontDescriptor")
                .as_dict()
                .cloned()
                .expect("a descriptor");
            (descendant, stated, descriptor)
        })
        .collect()
}

#[test]
#[ignore = "needs the CJK pack, a second download that is not part of a checkout: \
            QUORRA_CJK_PACK=<dir> names it (doc/cjk-pack.md)"]
fn a_descendant_stating_no_descriptor_is_given_one_measured_off_the_pack_program() {
    // Table 117 makes a CIDFont's /FontDescriptor required and an indirect reference; this one
    // states none, so the converter writes one, every Table 120 entry the table requires present.
    let source = Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <30423044> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /HeiseiMin-W3-UniJIS-UCS2-H \
             /Encoding /UniJIS-UCS2-H /DescendantFonts [7 0 R] >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /HeiseiMin-W3 \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> /DW 1000 >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build();
    let held = converted_with_the_pack(&source);
    let [(descendant, stated, descriptor)] = &descendants_and_descriptors(&held)[..] else {
        panic!("one descendant");
    };
    assert!(stated.is_some(), "Table 117's indirect reference");
    assert_eq!(
        held.get_key(descendant, "BaseFont").as_name(),
        held.get_key(descriptor, "FontName").as_name(),
        "Table 120: the FontName is the BaseFont of the CIDFont that refers to it"
    );
    for key in [
        "Type",
        "FontName",
        "Flags",
        "FontBBox",
        "ItalicAngle",
        "Ascent",
        "Descent",
        "CapHeight",
        "StemV",
        "FontFile3",
    ] {
        assert!(descriptor.get(key).is_some(), "{key}: {descriptor:?}");
    }
    let number = |key: &str| {
        held.get_key(descriptor, key)
            .as_integer()
            .expect("a whole number")
    };
    // Table 121's Symbolic alone; Table 120's Descent "shall be a negative number".
    assert_eq!(number("Flags"), 4);
    assert!(number("Descent") < 0 && number("Ascent") > 0);
    assert!(number("CapHeight") > 0 && number("CapHeight") <= number("Ascent"));
}

#[test]
#[ignore = "needs the CJK pack, a second download that is not part of a checkout: \
            QUORRA_CJK_PACK=<dir> names it (doc/cjk-pack.md)"]
fn descendants_sharing_a_descriptor_at_two_widths_are_each_given_their_own() {
    // §9.7.4.3: /DW is the advance of every CID /W does not list, so CID 843 is 1000 units wide
    // in one descendant and 500 in the other. A program states one advance per glyph (§9.2.4),
    // so the two cannot share one; each is given a descriptor and a subset of its own.
    let source = Conforming {
        resources: "/Font << /F1 6 0 R /F2 7 0 R >> /ColorSpace << /CS0 [/CalGray << \
                    /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <3042> Tj /F2 12 Tf <3042> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /Ryumin-Light-UniJIS-UCS2-H \
             /Encoding /UniJIS-UCS2-H /DescendantFonts [9 0 R] >>"
                .to_owned(),
            "<< /Type /Font /Subtype /Type0 /BaseFont /Ryumin-Light-UniJIS-UCS2-H \
             /Encoding /UniJIS-UCS2-H /DescendantFonts [10 0 R] >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /Ryumin-Light /Flags 4 \
             /FontBBox [-170 -331 1024 903] /ItalicAngle 0 /Ascent 723 /Descent -241 \
             /CapHeight 709 /StemV 69 >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /Ryumin-Light \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> \
             /FontDescriptor 8 0 R /DW 1000 >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /Ryumin-Light \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> \
             /FontDescriptor 8 0 R /DW 500 >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build();
    let held = converted_with_the_pack(&source);
    let found = descendants_and_descriptors(&held);
    let [(one, at_one, first), (other, at_other, second)] = &found[..] else {
        panic!("two descendants: {}", found.len());
    };
    assert_ne!(
        at_one, at_other,
        "each descendant names a descriptor of its own"
    );
    assert_ne!(
        held.get_key(first, "FontFile3")
            .as_stream()
            .map(|it| it.data.clone()),
        held.get_key(second, "FontFile3")
            .as_stream()
            .map(|it| it.data.clone()),
        "each descriptor holds a subset of its own, at its own advances"
    );
    for (descendant, descriptor) in [(one, first), (other, second)] {
        assert_eq!(
            held.get_key(descendant, "BaseFont").as_name(),
            held.get_key(descriptor, "FontName").as_name()
        );
        // The copy carries the producer's entries.
        assert_eq!(held.get_key(descriptor, "StemV").as_integer(), Some(69));
    }
}
