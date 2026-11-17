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
    a_serif_font_of("/Type1 /BaseFont /Times-Roman", "/Times-Roman")
}

/// The same font as a `TrueType` dictionary, `TimesNewRoman`: §9.6.5.4 takes a non-symbolic
/// `TrueType` font's code through its encoding's glyph name to the character the Adobe Glyph List
/// gives it, which is the same three characters.
fn a_truetype_serif_font_showing_greek_polish_and_cyrillic() -> Vec<u8> {
    a_serif_font_of("/TrueType /BaseFont /TimesNewRoman", "/TimesNewRoman")
}

/// The fixture with `subtype_and_name` after `/Subtype` and `font_name` as the descriptor's.
fn a_serif_font_of(subtype_and_name: &str, font_name: &str) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (ABC) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype {subtype_and_name} /FirstChar 65 /LastChar 67 \
                 /Widths [500 444 900] /Encoding << /Type /Encoding /BaseEncoding \
                 /WinAnsiEncoding /Differences [65 /beta /aogonek /afii10024] >> \
                 /FontDescriptor 7 0 R >>"
            ),
            format!(
                "<< /Type /FontDescriptor /FontName {font_name} /Flags 34 \
                 /FontBBox [-168 -218 1000 898] /ItalicAngle 0 /Ascent 683 /Descent -217 \
                 /CapHeight 662 /StemV 84 >>"
            ),
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
    the_pack_draws_the_fixture(&a_serif_font_showing_greek_polish_and_cyrillic());
}

/// ADR 2938: a `TrueType` dictionary is re-filed as `Type1`, since Table 124 admits a bare CFF
/// program under no key of a `TrueType` one, and the pack face draws it as for `Times-Roman`.
#[test]
#[ignore = "needs the Latin-Greek-Cyrillic pack, a download that is not part of a checkout: \
            QUORRA_LGC_PACK=<dir> names it (doc/lgc-pack.md)"]
fn a_truetype_dictionary_is_refiled_as_type1_for_the_packs_bare_cff_face() {
    the_pack_draws_the_fixture(&a_truetype_serif_font_showing_greek_polish_and_cyrillic());
}

/// The fixture converted with the pack: `/FontFile3 /Type1C` under a `Type1` dictionary, the
/// licence in the program's packet, a subset of the three glyphs and `.notdef` named by
/// §9.9.2's tag rule in `/BaseFont` and `/FontName` alike, and each code reaching its glyph at the
/// file's width under the named loss `machine-face`.
fn the_pack_draws_the_fixture(fixture: &[u8]) {
    let directory = std::env::var_os("QUORRA_LGC_PACK")
        .expect("QUORRA_LGC_PACK names the directory the pack was unpacked into");
    let pack = LgcPack::new(Arc::new(Directory(Some(directory.into()))));
    let (report, output) = convert_with_plan(fixture, &plan_with(Some(pack)));
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
    assert_eq!(
        held.get_key(&font, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Type1".to_vec()),
        "Table 124 admits the bare CFF under a Type1 dictionary only"
    );
    // ISO 32000-2 §9.9.2: a subset's /BaseFont and /FontName are one tag, a plus sign and the
    // PostScript name of the font it was cut from; the program holds the three glyphs and .notdef.
    let name_of = |dict: &pdf_syntax::object::Dictionary, key: &str| {
        held.get_key(dict, key)
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
    };
    let base_font = name_of(&font, "BaseFont").expect("a /BaseFont");
    assert_eq!(name_of(&descriptor, "FontName").as_ref(), Some(&base_font));
    let (tag, face) = base_font.split_once('+').expect("a subset's tag");
    assert!(tag.len() == 6 && tag.bytes().all(|byte| byte.is_ascii_uppercase()));
    assert_eq!(face, "NotoSerif-Regular");
    let bytes = held
        .decoded_stream_data(&program)
        .expect("the program decodes");
    assert_eq!(pdf_font::cff::glyph_count(&bytes).ok(), Some(4));
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

/// `Times-Roman` stated by name alone — no descriptor, no `/Widths`, as Table 109 lets one of the
/// fourteen — its codes 65 to 67 naming a-ogonek, n-acute and z-dot-above, which the standard's
/// Times-Roman metrics carry and the Foxit serif face does not.
fn a_standard_font_by_name_alone_showing_latin_extended_a() -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (ABC) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding << /Type /Encoding \
             /BaseEncoding /WinAnsiEncoding /Differences [65 /aogonek /nacute /zdotaccent] >> >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// ADR 2938: a standard font stated by name alone, whose shown codes only the pack draws, is given
/// the pack face under a descriptor measured off it, with the widths Table 109 then requires — the
/// ones the page is laid out by — where the shipped face's route had to photograph the page.
#[test]
#[ignore = "needs the Latin-Greek-Cyrillic pack, a download that is not part of a checkout: \
            QUORRA_LGC_PACK=<dir> names it (doc/lgc-pack.md)"]
fn a_standard_font_stated_by_name_alone_takes_the_packs_face_and_a_descriptor_of_its_own() {
    let directory = std::env::var_os("QUORRA_LGC_PACK")
        .expect("QUORRA_LGC_PACK names the directory the pack was unpacked into");
    let pack = LgcPack::new(Arc::new(Directory(Some(directory.into()))));
    let fixture = a_standard_font_by_name_alone_showing_latin_extended_a();
    let (report, output) = convert_with_plan(&fixture, &plan_with(Some(pack)));
    let held = Document::open_with_limits(output.expect("it converts"), Limits::DEFAULT)
        .expect("it opens");
    let font = font_resource(&held);
    let descriptor = held
        .get_key(&font, "FontDescriptor")
        .as_dict()
        .cloned()
        .expect("a descriptor is written");
    let program = held
        .get_key(&descriptor, "FontFile3")
        .as_stream()
        .cloned()
        .expect("Table 124's /FontFile3");
    let bytes = held
        .decoded_stream_data(&program)
        .expect("the program decodes");
    assert_eq!(pdf_font::cff::glyph_count(&bytes).ok(), Some(4));
    let base_font = held
        .get_key(&font, "BaseFont")
        .as_name()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
        .expect("a /BaseFont");
    assert!(base_font.ends_with("+NotoSerif-Regular"), "{base_font}");
    // The page is laid out as before: each code advances by the source's own width, and reaches a
    // glyph of the embedded program.
    let source = Document::open_with_limits(fixture, Limits::DEFAULT).expect("it opens");
    let before = pdf_font::LoadedFont::load(&source, &font_resource(&source), "F1")
        .expect("the source loads");
    let after = pdf_font::LoadedFont::load(&held, &font, "F1").expect("it loads");
    assert!(!after.is_substituted(), "the embedded program is drawn");
    for code in [65_u8, 66, 67].map(pdf_font::Code::single_byte) {
        assert!(
            after
                .glyph_index(code)
                .is_some_and(|glyph| glyph != pdf_font::NOTDEF_GLYPH),
            "{code:?}"
        );
        assert_eq!(
            after.advance(code).to_bits(),
            before.advance(code).to_bits(),
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

/// `Times-Roman` stated by name alone, its code 65 naming a-ogonek, which the standard's metrics
/// carry, and 66 naming `beta`, which they do not and the Foxit serif face does not hold either.
fn a_standard_font_by_name_alone_showing_a_beta() -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (ABA) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding << /Type /Encoding \
             /BaseEncoding /WinAnsiEncoding /Differences [65 /aogonek /beta] >> >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// ADR 2942: the window advances the `beta` by its face's `.notdef`, which ISO 32000-2 §9.6.5.2
/// substitutes for a name the face does not hold; the converted file draws the pack face's own
/// `beta`, and states that glyph's own advance for it, because §9.2.4 makes the width the painted
/// glyph's. The a-ogonek the metrics name keeps the published width the window laid it out at.
#[test]
#[ignore = "needs the Latin-Greek-Cyrillic pack, a download that is not part of a checkout: \
            QUORRA_LGC_PACK=<dir> names it (doc/lgc-pack.md)"]
fn a_name_the_standard_font_lacks_is_stated_at_the_pack_glyphs_own_advance() {
    let directory = std::env::var_os("QUORRA_LGC_PACK")
        .expect("QUORRA_LGC_PACK names the directory the pack was unpacked into");
    let pack = LgcPack::new(Arc::new(Directory(Some(directory.into()))));
    let fixture = a_standard_font_by_name_alone_showing_a_beta();
    let (_, output) = convert_with_plan(&fixture, &plan_with(Some(pack)));
    let held = Document::open_with_limits(output.expect("it converts"), Limits::DEFAULT)
        .expect("it opens");
    let font = font_resource(&held);
    let source = Document::open_with_limits(fixture, Limits::DEFAULT).expect("it opens");
    let before = pdf_font::LoadedFont::load(&source, &font_resource(&source), "F1")
        .expect("the source loads");
    let after = pdf_font::LoadedFont::load(&held, &font, "F1").expect("it loads");
    let (aogonek, beta) = (
        pdf_font::Code::single_byte(65),
        pdf_font::Code::single_byte(66),
    );
    assert!(before.advanced_by_notdef(beta));
    assert!((before.advance(beta) - 0.25).abs() < 1e-6, "Times' .notdef");
    assert_eq!(
        after.advance(aogonek).to_bits(),
        before.advance(aogonek).to_bits()
    );
    let own = after
        .program_advance(beta)
        .expect("the embedded program states the beta's advance");
    assert!(
        after
            .glyph_index(beta)
            .is_some_and(|glyph| glyph != pdf_font::NOTDEF_GLYPH),
        "the pack face's own beta is drawn"
    );
    assert!(own > 0.25, "{own}");
    assert!(
        (after.advance(beta) - own).abs() < 1e-6,
        "{} against {own}",
        after.advance(beta)
    );
}
