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
    loaded_showing(document, &[0x30, 0x42, 0x30, 0x44])
}

/// The fixture's font `F1` (object 6) as the viewer loads it, with the codes of `shown`.
fn loaded_showing(
    document: &Document,
    shown: &[u8],
) -> (pdf_font::LoadedFont, Vec<pdf_font::Code>) {
    let font = document.get(ObjectId::new(6, 0));
    let font = font.as_dict().expect("object 6 is the font");
    let loaded = pdf_font::LoadedFont::load(document, font, "F1").expect("the font loads");
    let codes = loaded.decode(shown);
    (loaded, codes)
}

/// The string [`a_kana_and_a_latin_capital`] shows: U+3042, then U+0042, which `UniJIS-UCS2-H`
/// maps to CIDs 843 and 35 of `Adobe-Japan1` (Table 116).
const KANA_AND_CAPITAL: [u8; 4] = [0x30, 0x42, 0x00, 0x42];

/// [`a_japanese_font_not_embedded`] showing [`KANA_AND_CAPITAL`], its descendant naming the
/// descriptor object 7 where `described`, and stating no descriptor at all where not.
fn a_kana_and_a_latin_capital(described: bool) -> Vec<u8> {
    let descriptor = if described {
        "/FontDescriptor 7 0 R "
    } else {
        ""
    };
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <30420042> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /HeiseiKakuGo-W5-UniJIS-UCS2-H \
             /Encoding /UniJIS-UCS2-H /DescendantFonts [8 0 R] >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /HeiseiKakuGo-W5 /Flags 4 \
             /FontBBox [-92 -250 1010 922] /ItalicAngle 0 /Ascent 752 /Descent -221 \
             /CapHeight 737 /StemV 114 >>"
                .to_owned(),
            format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /HeiseiKakuGo-W5 \
                 /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> \
                 {descriptor}/DW 1000 /W [35 [500]] >>"
            ),
        ],
        ..Conforming::part_two()
    }
    .build()
}

/// What the window draws for each code of [`KANA_AND_CAPITAL`] from `bytes`, as [`drawn`] does,
/// and whether the font is a substitute.
fn drawn_showing(bytes: &[u8]) -> (Vec<Drawn>, bool) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let (font, codes) = loaded_showing(&document, &KANA_AND_CAPITAL);
    (
        codes
            .iter()
            .map(|code| (wound(font.outline(*code)), font.advance(*code).to_bits()))
            .collect(),
        font.is_substituted(),
    )
}

/// An outline wound one way: the viewer winds a substitute's glyphs counter-clockwise and leaves
/// an embedded program's as drawn (ADR 0396).
fn wound(
    outline: Option<std::sync::Arc<pdf_render::Path>>,
) -> Option<Vec<pdf_render::PathCommand>> {
    outline.map(|path| {
        if path.signed_area() < 0.0 {
            path.reversed().commands().to_vec()
        } else {
            path.commands().to_vec()
        }
    })
}

/// The written output's descendant `CIDFont` of the font the page shows.
fn descendant_of(held: &Document) -> pdf_syntax::object::Dictionary {
    held.resolve(
        held.get_key(&font_resource(held), "DescendantFonts")
            .as_array()
            .and_then(|array| array.first())
            .expect("§9.7.6.2's one-element array"),
    )
    .as_dict()
    .cloned()
    .expect("the descendant")
}

/// The glyph index Table 115's `/CIDToGIDMap` stream gives `cid`: bytes 2*c* and 2*c* + 1, high
/// byte first.
fn mapped(held: &Document, descendant: &pdf_syntax::object::Dictionary, cid: usize) -> u16 {
    let pdf_syntax::Object::Stream(map) = held.get_key(descendant, "CIDToGIDMap") else {
        panic!("Table 115's /CIDToGIDMap is a stream");
    };
    let map = held.decoded_stream_data(&map).expect("the map decodes");
    let at = cid.saturating_mul(2);
    let entry = map
        .get(at..at.saturating_add(2))
        .expect("the map reaches the CID");
    u16::from_be_bytes([entry[0], entry[1]])
}

/// The fixture showing [`KANA_AND_CAPITAL`] where the window drew nothing for the capital, or
/// `None`, said out loud, where this machine draws it otherwise.
fn where_the_capital_draws_nothing(described: bool) -> Option<Vec<u8>> {
    let source = a_kana_and_a_latin_capital(described);
    if !drawn_from_a_machine_face(&source) {
        eprintln!("this machine has no face drawing the fixture's characters; nothing to embed");
        return None;
    }
    let (before, substituted) = drawn_showing(&source);
    assert!(substituted, "the window draws a substitute");
    if before[0].0.is_none() || before[1].0.is_some() {
        eprintln!("the face this machine draws the fixture from has a glyph for U+0042");
        return None;
    }
    Some(source)
}

/// ADR 2824: a CID the window drew nothing for is sent by the `/CIDToGIDMap` to a glyph of the
/// program that draws nothing, never to glyph 0, the `.notdef` (§9.9.2) a text-showing operator
/// may not reference under ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9. The
/// output conforms, the first pass writes it, the CID is reported, and each code draws as the
/// window drew it: the kana's outline, and nothing at the capital's 500 (§9.7.4.3's `/W`). The
/// plant: sending the CID to glyph 0 draws `.notdef`'s box, the route's proof refuses it, and
/// the document does not convert.
#[test]
fn a_cid_the_window_drew_nothing_for_reaches_a_glyph_that_draws_nothing() {
    let Some(source) = where_the_capital_draws_nothing(true) else {
        return;
    };
    let (before, _) = drawn_showing(&source);
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
            (font.authority, font.empty_cids.as_slice()),
            (pdf_transform::archive::FaceAuthority::Machine, &[35][..]),
            "{target}: the machine's face, and CID 35 reported"
        );
        let held = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
        let descendant = descendant_of(&held);
        assert_ne!(
            mapped(&held, &descendant, 35),
            0,
            "{target}: CID 35 is not sent to .notdef"
        );
        assert_ne!(mapped(&held, &descendant, 843), 0, "{target}: nor CID 843");
        let (after, substituted_after) = drawn_showing(&output);
        assert!(!substituted_after, "{target}: drawn from its own program");
        assert_eq!(
            before, after,
            "{target}: the output draws what the window drew, code by code"
        );
        assert_eq!(
            after[1],
            (None, 0.5_f32.to_bits()),
            "{target}: nothing, at 500"
        );
    }
}

/// ADR 2825: a descendant stating no descriptor is given one of its own, an indirect object as
/// ISO 32000-2 Table 117 requires, holding every entry Table 120 requires — `/Type`, `/FontName`
/// (the subset's, as `/BaseFont` is, §9.9.2), `/Flags`, `/FontBBox`, `/ItalicAngle`, `/Ascent`,
/// a negative `/Descent`, `/CapHeight` and `/StemV` — and the program under `/FontFile2` (Table
/// 124). The box holds the ascent and the descent, both being points of the glyphs it encloses.
/// The plant: leaving `/Descent` out of the built descriptor fails the entry's assertion.
#[test]
fn a_descendant_stating_no_descriptor_is_given_one_measured_on_the_program() {
    let Some(source) = where_the_capital_draws_nothing(false) else {
        return;
    };
    let (before, _) = drawn_showing(&source);
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
        let held = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
        let descendant = descendant_of(&held);
        assert!(
            matches!(
                descendant.get("FontDescriptor"),
                Some(pdf_syntax::Object::Reference(_))
            ),
            "{target}: Table 117's descriptor is an indirect reference"
        );
        let descriptor = held
            .get_key(&descendant, "FontDescriptor")
            .as_dict()
            .cloned()
            .expect("its descriptor");
        let number = |key: &str| {
            held.get_key(&descriptor, key)
                .as_number()
                .unwrap_or_else(|| panic!("{target}: Table 120's /{key}"))
        };
        assert_eq!(
            held.get_key(&descriptor, "FontName").as_name(),
            held.get_key(&descendant, "BaseFont").as_name(),
            "{target}: /FontName is the subset's name"
        );
        assert_eq!(
            held.get_key(&descriptor, "Type")
                .as_name()
                .map(pdf_syntax::Name::as_bytes),
            Some(b"FontDescriptor".as_slice()),
            "{target}: Table 120's /Type"
        );
        let flags = number("Flags");
        let (italic, stem, capital) = (number("ItalicAngle"), number("StemV"), number("CapHeight"));
        assert!(flags >= 0.0 && italic.is_finite() && stem >= 0.0 && capital > 0.0);
        let (ascent, descent) = (number("Ascent"), number("Descent"));
        assert!(descent < 0.0, "{target}: Table 120's /Descent is negative");
        let bbox: Vec<f64> = held
            .get_key(&descriptor, "FontBBox")
            .as_array()
            .expect("Table 120's /FontBBox")
            .iter()
            .map(|side| held.resolve(side).as_number().expect("a number"))
            .collect();
        assert!(
            bbox.len() == 4 && bbox[1] <= descent && ascent <= bbox[3],
            "{target}: the box {bbox:?} encloses {descent}..{ascent}"
        );
        assert!(
            matches!(
                held.get_key(&descriptor, "FontFile2"),
                pdf_syntax::Object::Stream(_)
            ),
            "{target}: the program is under /FontFile2"
        );
        let (after, _) = drawn_showing(&output);
        assert_eq!(
            before, after,
            "{target}: the output draws what the window drew"
        );
    }
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

/// An `Identity-H` font over an `Adobe-Identity` descendant named `Arial` with no program,
/// showing CIDs 36, 37 and 48, whose `/ToUnicode` names `A`, `B` and `A` again, at `/W`'s 667
/// each; the descendant states `extra` as well.
fn an_arial_identity_font(extra: &str) -> Vec<u8> {
    let cmap = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
                /CMapName /Test def\n/CMapType 2 def\n\
                1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                3 beginbfchar\n<0024> <0041>\n<0025> <0042>\n<0030> <0041>\nendbfchar\n\
                endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td <002400250030> Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /Arial /Encoding /Identity-H \
             /DescendantFonts [7 0 R] /ToUnicode 9 0 R >>"
                .to_owned(),
            format!(
                "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Arial /CIDSystemInfo \
                 << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor \
                 8 0 R /DW 1000 /W [36 [667 667] 48 [667]]{extra} >>"
            ),
            "<< /Type /FontDescriptor /FontName /Arial /Flags 32 \
             /FontBBox [-665 -325 2000 1006] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(&format!("/Length {}", cmap.len()), cmap.as_bytes())],
        ..Conforming::part_two()
    }
    .build()
}

/// The codes [`an_arial_identity_font`] shows.
const ARIAL_SHOWN: [u8; 6] = [0x00, 0x24, 0x00, 0x25, 0x00, 0x30];

/// What the window draws for each code of [`ARIAL_SHOWN`] from `bytes`, and whether the font is a
/// substitute.
fn drawn_arial(bytes: &[u8]) -> (Vec<Drawn>, bool) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let (font, codes) = loaded_showing(&document, &ARIAL_SHOWN);
    (
        codes
            .iter()
            .map(|code| (wound(font.outline(*code)), font.advance(*code).to_bits()))
            .collect(),
        font.is_substituted(),
    )
}

/// [`an_arial_identity_font`] stating nothing more, where this machine draws it from a `CFF `
/// face of its own; `None`, said out loud, where it does not.
fn where_arial_is_drawn_from_a_machines_cff_face() -> Option<Vec<u8>> {
    let source = an_arial_identity_font("");
    let document = Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("it opens");
    let (font, _) = loaded(&document);
    let cff = font
        .substitute_program()
        .is_some_and(pdf_font::embed::has_cff_outlines);
    if !drawn_from_a_machine_face(&source) || !cff {
        eprintln!("this machine draws the fixture's Arial from no CFF face of its own");
        return None;
    }
    Some(source)
}

/// The losses the conversion names for the fixture's one font.
fn losses_of(report: &Report) -> Losses {
    conversion(report)
        .substituted
        .first()
        .expect("one font")
        .losses
}

/// A shipped profile's plan at `target`, its authorisations read from the profile.
fn under(profile: &str, target: Target) -> ArchivePlan {
    let text = pdf_transform::profiles::named(profile)
        .expect("a shipped profile")
        .text;
    let config = pdf_transform::archive::Configuration::read(text, target).expect("it reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    }
}

/// ADR 2828: an `Adobe-Identity` font the window drew from a `CFF ` face of this machine's goes in
/// with that face, a bare CID-keyed subset under `/FontFile3` `/Subtype /CIDFontType0C` beneath a
/// `CIDFontType0` (Table 124, Table 117), with no `/CIDToGIDMap` (Table 117 gives it to a Type 2
/// `CIDFont` alone, §9.7.4.2 having a Type 0's CIDs reach glyphs through the charset). It loses
/// `glyphs-from-to-unicode` alone, is written by the first pass, conforms, and draws each shown
/// code's outline at its advance as the window drew it. The plants: writing each CID's glyph one
/// index on in the charset draws other outlines, the proof refuses the route, the font takes the
/// shipped face's route, which names `machine-face`, and the document does not convert; and
/// reading `Identity-H` as a predefined file rather than by Table 116's rule finds no `CMap`, with
/// the same end.
#[test]
fn an_identity_font_drawn_from_a_machines_cff_face_is_embedded_with_that_face() {
    let Some(source) = where_arial_is_drawn_from_a_machines_cff_face() else {
        return;
    };
    let (before, _) = drawn_arial(&source);
    assert!(
        before.iter().all(|(outline, _)| outline.is_some()),
        "the window drew a glyph for each code"
    );
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::GlyphsFromToUnicode);
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert(&source, target, authorised);
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
                Losses::one(Loss::GlyphsFromToUnicode)
            ),
            "{target}: the machine's face, and the glyph choice named"
        );
        let held = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
        let descendant = descendant_of(&held);
        assert_eq!(
            held.get_key(&descendant, "Subtype")
                .as_name()
                .map(|name| name.as_bytes().to_vec()),
            Some(b"CIDFontType0".to_vec()),
            "{target}: Table 117's subtype for a CFF program"
        );
        assert!(
            held.get_key(&descendant, "CIDToGIDMap").is_null(),
            "{target}: a Type 0 CIDFont states no /CIDToGIDMap"
        );
        let descriptor = held
            .get_key(&descendant, "FontDescriptor")
            .as_dict()
            .cloned()
            .expect("its descriptor");
        let pdf_syntax::Object::Stream(program) = held.get_key(&descriptor, "FontFile3") else {
            panic!("{target}: the program is under /FontFile3");
        };
        assert_eq!(
            program
                .dict
                .get("Subtype")
                .and_then(pdf_syntax::Object::as_name)
                .map(|name| name.as_bytes().to_vec()),
            Some(b"CIDFontType0C".to_vec()),
            "{target}: Table 124's row for a bare CID-keyed CFF"
        );
        let (after, substituted_after) = drawn_arial(&output);
        assert!(!substituted_after, "{target}: drawn from its own program");
        assert_eq!(
            before, after,
            "{target}: the output draws what the window drew, code by code"
        );
    }
}

/// A832, ADR 2829: `as-shown` admits `glyphs-from-to-unicode` and still stops on `machine-face`,
/// so the font converts where the machine's face went in proved, and the loss stays named in the
/// file's own history (ISO 19005-2 section 6.7.2's `xmpMM:History`, ADR 2693). Where the machine's
/// route cannot take the font — a descendant stating a `/CIDToGIDMap`, which Table 117 gives no
/// Type 0 `CIDFont` — the shipped face's route names `machine-face` and `as-shown` stops on it;
/// `refuse-any-loss` stops on the glyph choice itself. The plant: deleting the profile's
/// `glyphs-from-to-unicode` line refuses the first case.
#[test]
fn as_shown_admits_the_glyph_choice_only_where_the_machines_face_went_in() {
    let Some(source) = where_arial_is_drawn_from_a_machines_cff_face() else {
        return;
    };
    let mapped = an_arial_identity_font(" /CIDToGIDMap /Identity");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert_with_plan(&source, &under("as-shown", target));
        assert!(output.is_some(), "{target}: as-shown converts it");
        assert!(
            conversion(&report).settled.is_empty(),
            "{target}: by the first pass"
        );
        assert_eq!(losses_of(&report), Losses::one(Loss::GlyphsFromToUnicode));
        let recorded = conversion(&report).recorded.as_deref().expect("a history");
        assert!(
            recorded.contains("glyphs-from-to-unicode"),
            "{target}: {recorded}"
        );

        let (report, output) = convert_with_plan(&mapped, &under("as-shown", target));
        assert!(output.is_none(), "{target}: the shipped face's route stops");
        let Decision::Unauthorised { losses, .. } =
            decision(&report, "fonts/font-programs-embedded")
        else {
            panic!("{target}: a loss to authorise, not a refusal");
        };
        assert!(
            losses.iter().any(|loss| loss == Loss::MachineFace),
            "{target}: machine-face"
        );

        let (_, output) = convert_with_plan(&source, &under("refuse-any-loss", target));
        assert!(output.is_none(), "{target}: refuse-any-loss stops");
    }
}

/// A832: of the profiles this program ships, `as-shown` now authorises `glyphs-from-to-unicode`
/// beside the two that already did, `as-if-printed` and `state-every-loss`; every other withholds
/// it, at both parts. The plant: the line deleted from `as-shown.toml` fails its row.
#[test]
fn which_shipped_profiles_admit_the_glyph_choice() {
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        for shipped in pdf_transform::profiles::SHIPPED {
            let admits = matches!(
                shipped.name,
                "as-shown" | "as-if-printed" | "state-every-loss"
            );
            assert_eq!(
                under(shipped.name, target)
                    .authorised
                    .grants(Loss::GlyphsFromToUnicode),
                admits,
                "{} at {target}",
                shipped.name
            );
        }
    }
}
