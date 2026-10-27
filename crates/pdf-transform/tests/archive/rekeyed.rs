//! An `Adobe-Identity` composite font the file does not embed, re-keyed from its `/ToUnicode`
//! (owner answer A570, ADR 2378).
//!
//! The expected values come from ISO 32000-2: Table 116 (under `Identity-H` a two-byte code is
//! its CID), §9.10.3 (the `/ToUnicode` `CMap` states the character each code stands for), Table
//! 115 (a Type 2 `CIDFont`'s `/CIDToGIDMap` stream stores the glyph index for CID *c* in bytes
//! 2*c* and 2*c* + 1) and Table 124 (a TrueType program goes under `/FontFile2`); and from A570's
//! reading that the shipped face's glyphs chosen by those characters are two losses,
//! `machine-face` and `glyphs-from-to-unicode`, which `as-if-printed` discards and `as-shown`
//! does not.
use super::{Conforming, Decision, Level, Target, a_supplied_font, convert_with_plan, decision};
use super::{font_resource, stream};
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{Authorisations, Loss};
use skrifa::MetadataProvider as _;

/// The fixture's one font: `Identity-H` over an `Adobe-Identity` `CIDFontType2` named `Arial`
/// with no program, showing CIDs 36, 37 and 48, whose `/ToUnicode` names `A`, `B` and `A` again
/// — or, with `stated` false, leaves CID 48 out.
fn an_identity_font_not_embedded(stated: bool) -> Vec<u8> {
    if stated {
        an_identity_font(
            "3 beginbfchar\n<0024> <0041>\n<0025> <0042>\n<0030> <0041>\nendbfchar\n",
            b"<002400250030>",
            "48 [667]",
        )
    } else {
        an_identity_font(
            "2 beginbfchar\n<0024> <0041>\n<0025> <0042>\nendbfchar\n",
            b"<002400250030>",
            "48 [667]",
        )
    }
}

/// The same font with its `/ToUnicode` entries, the string it shows and a third `/W` run given.
fn an_identity_font(entries: &str, shown: &[u8], third: &str) -> Vec<u8> {
    an_identity_font_beside(entries, shown, third, false, "Arial")
}

/// [`an_identity_font`], with, where `sibling` is true, a second `CIDFont` that the page
/// never shows, states the producer's own `/BaseFont /{name}` and names the same descriptor object.
fn an_identity_font_beside(
    entries: &str,
    shown: &[u8],
    third: &str,
    sibling: bool,
    name: &str,
) -> Vec<u8> {
    let cmap = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CMapName /Test def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n{entries}\
         endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
    );
    Conforming {
        resources: format!(
            "/Font << /F1 6 0 R{} >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
             [0.9505 1.0 1.089] >>] >>",
            if sibling { " /F2 10 0 R" } else { "" }
        ),
        contents: Some((
            String::new(),
            [
                &b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td "[..],
                shown,
                &b" Tj ET"[..],
            ]
            .concat(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H \
                 /DescendantFonts [7 0 R] /ToUnicode 9 0 R >>"
            ),
            format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{name} /CIDSystemInfo \
             << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R \
             /DW 1000 /W [36 [667 667] {third}] >>"
            ),
            format!(
                "<< /Type /FontDescriptor /FontName /{name} /Flags 32 \
                 /FontBBox [-665 -325 2000 1006] /ItalicAngle 0 /Ascent 905 /Descent -212 \
                 /CapHeight 716 /StemV 80 >>"
            ),
        ],
        binary_objects: [stream(&format!("/Length {}", cmap.len()), cmap.as_bytes())]
            .into_iter()
            .chain(sibling.then(|| {
                format!(
                    "<< /Type /Font /Subtype /Type0 /BaseFont /{name}-Sibling /Encoding \
                     /Identity-H /DescendantFonts [11 0 R] >>"
                )
                .into_bytes()
            }))
            .chain(sibling.then(|| {
                format!(
                    "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{name} /CIDSystemInfo \
                     << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor \
                     8 0 R /DW 1000 >>"
                )
                .into_bytes()
            }))
            .collect(),
        ..Conforming::part_two()
    }
    .build()
}

#[test]
fn an_identity_font_is_two_losses_as_shown_refuses() {
    let plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    let (report, output) = convert_with_plan(&an_identity_font_not_embedded(true), &plan);
    assert!(output.is_none(), "the losses nobody authorised stop it");
    let Decision::Unauthorised { losses, .. } = decision(&report, "fonts/font-programs-embedded")
    else {
        panic!("a re-keyed font is a loss to authorise, not a refusal");
    };
    assert!(losses.iter().any(|loss| loss == Loss::MachineFace));
    assert!(losses.iter().any(|loss| loss == Loss::GlyphsFromToUnicode));
}

#[test]
fn an_identity_font_is_given_the_glyphs_its_to_unicode_values_select_at_its_own_cids() {
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    authorised.authorise(Loss::GlyphsFromToUnicode);
    plan.authorised = authorised;
    let (_, output) = convert_with_plan(&an_identity_font_not_embedded(true), &plan);
    let held = Document::open_with_limits(output.expect("it converts"), Limits::DEFAULT)
        .expect("it opens");
    let font = font_resource(&held);
    assert_eq!(
        held.get_key(&font, "Encoding")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Identity-H".to_vec()),
        "the codes, and so the CIDs, are the document's"
    );
    let descendant = held
        .resolve(
            held.get_key(&font, "DescendantFonts")
                .as_array()
                .and_then(|array| array.first())
                .expect("§9.7.6.2's one-element array"),
        )
        .as_dict()
        .cloned()
        .expect("the descendant CIDFont");
    let pdf_syntax::object::Object::Stream(map) = held.get_key(&descendant, "CIDToGIDMap") else {
        panic!("Table 115's stream map");
    };
    let map = held.decoded_stream_data(&map).expect("the map decodes");
    let descriptor = held.get_key(&descendant, "FontDescriptor");
    let descriptor = held
        .resolve(&descriptor)
        .as_dict()
        .cloned()
        .expect("a descriptor");
    let pdf_syntax::object::Object::Stream(program) = held.get_key(&descriptor, "FontFile2") else {
        panic!("Table 124's FontFile2");
    };
    let program = held
        .decoded_stream_data(&program)
        .expect("the program decodes");
    let face = skrifa::FontRef::new(&program).expect("an sfnt");
    let glyph_of = |character: char| {
        u16::try_from(
            face.charmap()
                .map(character)
                .expect("the face has it")
                .to_u32(),
        )
        .expect("a 16-bit glyph")
    };
    let mapped = |cid: usize| u16::from_be_bytes([map[2 * cid], map[2 * cid + 1]]);
    assert_eq!(mapped(36), glyph_of('A'));
    assert_eq!(mapped(37), glyph_of('B'));
    assert_eq!(
        mapped(48),
        glyph_of('A'),
        "two CIDs stating one character show one glyph"
    );
    assert_eq!(mapped(40), 0, "a CID nothing shows reaches .notdef");
}

// ISO 32000-2 Table 120's `/FontName`: "This name shall be the same as the value of BaseFont in
// the font or CIDFont dictionary that refers to this font descriptor." The re-keyed font's
// descriptor is renamed for the face it now carries, so a font the page never shows that names
// the same object must be pointed at a descriptor stating its own name instead.
#[test]
fn a_font_that_shares_a_rekeyed_descriptor_keeps_the_name_it_states() {
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    authorised.authorise(Loss::GlyphsFromToUnicode);
    plan.authorised = authorised;
    let source = an_identity_font_beside(
        "3 beginbfchar\n<0024> <0041>\n<0025> <0042>\n<0030> <0041>\nendbfchar\n",
        b"<002400250030>",
        "48 [667]",
        true,
        "TimesNewRoman",
    );
    let (_, output) = convert_with_plan(&source, &plan);
    let held = Document::open_with_limits(output.expect("it converts"), Limits::DEFAULT)
        .expect("it opens");
    let mut named = Vec::new();
    for number in 1..held.xref().next_object_number() {
        let object = held.get(pdf_syntax::object::ObjectId::new(number, 0));
        let Some(font) = object.as_dict() else {
            continue;
        };
        let Some(base_font) = held
            .get_key(font, "BaseFont")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
        else {
            continue;
        };
        let descriptor = held.get_key(font, "FontDescriptor");
        let Some(descriptor) = held.resolve(&descriptor).as_dict().cloned() else {
            continue;
        };
        let font_name = held
            .get_key(&descriptor, "FontName")
            .as_name()
            .map(|name| name.as_bytes().to_vec());
        assert_eq!(
            font_name.as_deref(),
            Some(base_font.as_slice()),
            "Table 120: object {number}'s descriptor names the font its BaseFont names"
        );
        let embedded = ["FontFile", "FontFile2", "FontFile3"]
            .iter()
            .any(|key| !held.get_key(&descriptor, key).is_null());
        named.push((base_font, embedded));
    }
    assert!(
        named
            .iter()
            .any(|(name, embedded)| name == b"TimesNewRoman" && !embedded),
        "the font nothing shows keeps a descriptor with no program: {named:?}"
    );
    assert!(
        named.iter().any(|(_, embedded)| *embedded),
        "the font the page shows carries the face: {named:?}"
    );
}

#[test]
fn a_shown_code_the_to_unicode_does_not_state_keeps_the_refusal() {
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    authorised.authorise(Loss::GlyphsFromToUnicode);
    plan.authorised = authorised;
    let (report, output) = convert_with_plan(&an_identity_font_not_embedded(false), &plan);
    assert!(output.is_none(), "no file is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("a code with no character is refused by name");
    };
    assert!(
        because.sentence().contains("no single usable character"),
        "{}",
        because.sentence()
    );
}

/// A reader of the pack directory `QUORRA_CJK_PACK` names.
#[derive(Debug)]
struct Directory(std::path::PathBuf);

impl pdf_transform::archive::PackFiles for Directory {
    fn read(&self, file: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(file)).ok()
    }
}

#[test]
#[ignore = "needs the CJK pack, a second download that is not part of a checkout: \
            QUORRA_CJK_PACK=<dir> names it (doc/cjk-pack.md)"]
fn a_character_the_shipped_face_lacks_is_drawn_from_the_pack_at_the_documents_cid() {
    let directory = std::env::var_os("QUORRA_CJK_PACK")
        .expect("QUORRA_CJK_PACK names the directory the pack was unpacked into");
    // CID 256 states あ, which no shipped Latin face draws and kana sends to Adobe-Japan1, whose
    // UniJIS-UTF32-H maps U+3042 to CID 843.
    let source = an_identity_font(
        "3 beginbfchar\n<0024> <0041>\n<0025> <0042>\n<0100> <3042>\nendbfchar\n",
        b"<002401000025>",
        "256 [1000]",
    );
    let mut plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    plan.cjk_pack = Some(pdf_transform::archive::CjkPack::new(std::sync::Arc::new(
        Directory(directory.into()),
    )));
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::MachineFace);
    authorised.authorise(Loss::GlyphsFromToUnicode);
    plan.authorised = authorised;
    let (report, output) = convert_with_plan(&source, &plan);
    let Some(output) = output else {
        panic!(
            "it converts: {:?}",
            decision(&report, "fonts/font-programs-embedded")
        );
    };
    let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let font = font_resource(&held);
    let descendant = held
        .resolve(
            held.get_key(&font, "DescendantFonts")
                .as_array()
                .and_then(|array| array.first())
                .expect("§9.7.6.2's one-element array"),
        )
        .as_dict()
        .cloned()
        .expect("the descendant CIDFont");
    assert_eq!(
        held.get_key(&descendant, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"CIDFontType0".to_vec()),
        "Table 124 puts a CFF program under a CIDFontType0"
    );
    let descriptor = held.get_key(&descendant, "FontDescriptor");
    let descriptor = held
        .resolve(&descriptor)
        .as_dict()
        .cloned()
        .expect("a descriptor");
    let pdf_syntax::object::Object::Stream(program) = held.get_key(&descriptor, "FontFile3") else {
        panic!("Table 124's FontFile3");
    };
    let program = held
        .decoded_stream_data(&program)
        .expect("the program decodes");
    let Ok(pdf_font::cff::CodeToGlyph::Keyed { by_cid }) =
        pdf_font::cff::CodeToGlyph::read(&program)
    else {
        panic!("a CID-keyed subset");
    };
    assert_eq!(
        by_cid.keys().copied().collect::<Vec<_>>(),
        vec![0, 36, 37, 256],
        "the charset numbers every glyph by the document's own CID"
    );
}
