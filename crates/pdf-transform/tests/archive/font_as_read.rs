//! A font this reader read past, written as it read it (ADRs 3569, 3570, 3793 and 3794): a program
//! whose table directory names a table past its end, a `/ToUnicode` and an `/Encoding` that name
//! nothing ISO 32000-2 defines, a Type 3 font's encoding, and a font written inside another object.

use super::*;
use pdf_syntax::Dictionary;
use pdf_transform::profiles::{Contribution, named};

const TARGET: Target = Target::Two(Level::B);

/// Liberation Sans with its `post` record pointed past the program's end, which `pdf_font` reads
/// as no `post` table and logs as `font/sfnt-table-past-the-program`.
fn a_program_past_its_end() -> Vec<u8> {
    let mut cut = LIBERATION_SANS.to_vec();
    let count = usize::from(u16::from_be_bytes([cut[4], cut[5]]));
    let post = (0..count)
        .filter_map(|index| index.checked_mul(16)?.checked_add(12))
        .find(|at| {
            cut.get(*at..)
                .is_some_and(|record| record.starts_with(b"post"))
        })
        .expect("the face states a post table");
    let offset = post.checked_add(8).expect("a record's offset field");
    cut.splice(offset..offset.saturating_add(4), u32::MAX.to_be_bytes());
    cut
}

/// One page showing `A` in a TrueType font embedding `program`, its dictionary stating `entries`.
fn a_page_in(program: &[u8], entries: &str) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 65 \
                 /LastChar 65 /Widths [667] /FontDescriptor 7 0 R {entries} >>"
            ),
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 8 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!("/Length {} /Length1 {}", program.len(), program.len()),
            program,
        )],
        ..Conforming::part_two()
    }
    .build()
}

fn authorised_by(profile: &str) -> Authorisations {
    Contribution::read(named(profile).expect("a shipped profile").text, TARGET)
        .expect("a shipped profile reads")
        .authorised
}

fn fails(bytes: &[u8], row: &str) -> bool {
    holds(bytes, TARGET)
        .failures()
        .any(|judgement| judgement.id == row)
}

/// The converted font object 6, and its program's bytes.
fn font_and_program(bytes: &[u8]) -> (Dictionary, Vec<u8>) {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let font = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font");
    let descriptor = held.resolve(&held.get_key(&font, "FontDescriptor"));
    let descriptor = descriptor.as_dict().expect("a descriptor");
    let program = held.resolve(&held.get_key(descriptor, "FontFile2"));
    let program = held
        .decoded_stream_data(program.as_stream().expect("a program stream"))
        .expect("it decodes")
        .to_vec();
    (font, program)
}

#[test]
fn a_program_read_past_its_end_fails_the_embedding_row_and_is_written_as_read() {
    let source = a_page_in(&a_program_past_its_end(), "/Encoding /WinAnsiEncoding");
    assert!(fails(&source, "fonts/font-programs-embedded"));
    let (report, output) = convert(&source, TARGET, authorised_by("as-shown"));
    assert_eq!(
        decision(&report, "fonts/font-programs-embedded"),
        Decision::Mechanical(Rewrite::FontProgramAsRead)
    );
    let output = written(&report, output);
    let held = holds(&output, TARGET);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (_, program) = font_and_program(&output);
    assert!(
        program.len() < LIBERATION_SANS.len(),
        "the post table is left out"
    );
    assert!(!program.windows(4).take(400).any(|tag| tag == b"post"));
}

#[test]
fn both_shipped_profiles_authorise_the_maps_as_read() {
    for profile in ["as-if-printed", "as-shown"] {
        assert!(
            authorised_by(profile).grants(Loss::FontMapsAsRead),
            "{profile}"
        );
    }
}

#[test]
fn a_to_unicode_name_is_removed_as_read() {
    let source = a_page_in(
        LIBERATION_SANS,
        "/ToUnicode /Identity-H /Encoding /WinAnsiEncoding",
    );
    assert!(fails(&source, "fonts/font-maps-read-as-written"));
    let (report, output) = convert(&source, TARGET, authorised_by("as-if-printed"));
    assert_eq!(
        decision(&report, "fonts/font-maps-read-as-written"),
        Decision::Authorised {
            losses: Losses::one(Loss::FontMapsAsRead),
            rewrite: Rewrite::FontMapsAsRead,
        }
    );
    let output = written(&report, output);
    let held = holds(&output, TARGET);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (font, _) = font_and_program(&output);
    assert!(font.get("ToUnicode").is_none());
    assert!(font.get("Encoding").is_some());
}

/// A nonsymbolic TrueType font's unknown `/Encoding` name is not read as an absent entry: this
/// reader draws `A` through another route without the name, so the proof refuses the
/// restatement and the font is refused by name rather than drawn differently.
#[test]
fn an_unknown_encoding_the_proof_does_not_hold_for_is_refused_by_name() {
    let source = a_page_in(LIBERATION_SANS, "/Encoding /NULL");
    assert!(fails(&source, "fonts/font-maps-read-as-written"));
    let (report, output) = convert(&source, TARGET, authorised_by("as-if-printed"));
    assert!(matches!(
        decision(&report, "fonts/font-maps-read-as-written"),
        Decision::Refused(_)
    ));
    assert!(output.is_none());
}

/// ISO 32000-2 §D.1, Table D.1:
///
/// > PDF processors shall not have a predefined encoding named StandardEncoding.
///
/// On a nonsymbolic TrueType font the name is the one ISO 19005-2 section 6.2.11.6 asks to be
/// `MacRomanEncoding` or `WinAnsiEncoding`, so it is that row's and not this one's (ADR 3753):
/// the font-maps row passes, and the TrueType rewrite restates the name.
#[test]
fn a_nonsymbolic_truetype_fonts_standard_encoding_is_the_truetype_rows() {
    let source = a_page_in(LIBERATION_SANS, "/Encoding /StandardEncoding");
    assert!(!fails(&source, "fonts/font-maps-read-as-written"));
    assert!(fails(
        &source,
        "fonts/non-symbolic-truetype-uses-a-standard-encoding"
    ));
    let (report, output) = convert(&source, TARGET, authorised_by("as-if-printed"));
    assert_eq!(
        decision(
            &report,
            "fonts/non-symbolic-truetype-uses-a-standard-encoding"
        ),
        Decision::Mechanical(Rewrite::StandardTrueTypeEncoding)
    );
    let output = written(&report, output);
    let held = holds(&output, TARGET);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
}

/// The same name on a Type 1 font is a map this reader read past (ADR 3753): Table D.1 leaves the
/// font's built-in encoding standing, which is the font-maps row's to restate.
#[test]
fn a_type1_fonts_standard_encoding_is_a_map_read_past() {
    let source = Conforming {
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((String::new(), b"BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec())),
        objects: vec![
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 65 /LastChar 65 \
             /Widths [667] /FontDescriptor 7 0 R /Encoding /StandardEncoding >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 \
             /FontBBox [-166 -225 1000 931] /ItalicAngle 0 /Ascent 718 /Descent -207 \
             /CapHeight 718 /StemV 88 >>"
                .to_owned(),
        ],
        ..Conforming::part_two()
    }
    .build();
    assert!(fails(&source, "fonts/font-maps-read-as-written"));
}

/// ISO 32000-2 §9.10.3 has a `/ToUnicode` map's destinations "expressed in UTF-16BE encoding",
/// and `<D800>` is an unpaired surrogate, no UTF-16BE text: this reader reads that entry as no
/// mapping and logs the map damaged. The map is written again from the mappings that read, at its
/// own object (ADR 3781): `A` still reads as `A`, and code `B` reads as nothing, as it did.
#[test]
fn a_damaged_to_unicode_is_written_as_read() {
    let map = b"1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
        2 beginbfchar\n<41> <0041>\n<42> <D800>\nendbfchar\n";
    let source = Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 65 \
             /LastChar 65 /Widths [667] /FontDescriptor 7 0 R /Encoding /WinAnsiEncoding \
             /ToUnicode 9 0 R >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 8 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![
            stream(
                &format!(
                    "/Length {} /Length1 {}",
                    LIBERATION_SANS.len(),
                    LIBERATION_SANS.len()
                ),
                LIBERATION_SANS,
            ),
            stream(&format!("/Length {}", map.len()), map),
        ],
        ..Conforming::part_two()
    }
    .build();
    assert!(fails(&source, "fonts/font-maps-read-as-written"));
    let (report, output) = convert(&source, TARGET, authorised_by("as-if-printed"));
    assert_eq!(
        decision(&report, "fonts/font-maps-read-as-written"),
        Decision::Authorised {
            losses: Losses::one(Loss::FontMapsAsRead),
            rewrite: Rewrite::FontMapsAsRead,
        }
    );
    let output = written(&report, output);
    let held = holds(&output, TARGET);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (font, _) = font_and_program(&output);
    let reopened = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let read = pdf_font::to_unicode(&reopened, &font);
    assert!(!read.damaged());
    let answer = |code: u32| {
        let mut out = String::new();
        read.append(code, &mut out).then_some(out)
    };
    assert_eq!(answer(0x41).as_deref(), Some("A"));
    assert_eq!(answer(0x42), None);
}

#[test]
fn nothing_is_restated_without_the_loss() {
    let source = a_page_in(LIBERATION_SANS, "/ToUnicode /Identity-H");
    let (report, _) = convert(&source, TARGET, Authorisations::default());
    assert!(matches!(
        decision(&report, "fonts/font-maps-read-as-written"),
        Decision::Unauthorised { .. }
    ));
}

/// pdf.js `bug911034`, whose TrueType program's `hmtx` states all 8 320 of its long metrics and
/// none of the 42 057 side bearings after them, and whose `/Widths` disagree with the program: the
/// advances are restated, each bearing the table omits is taken from the glyph's own `glyf` box
/// (ISO/IEC 14496-22's `hmtx`), and the restated program is proved to draw every glyph as the
/// original did (ADR 3642), so the file converts under `as-shown`.
#[test]
fn a_program_whose_hmtx_omits_its_bearings_is_restated_with_the_glyphs_boxes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source =
        std::fs::read(root.join("doc/pdf.js/test/pdfs/bug911034.pdf")).expect("the witness");
    let text = named("as-shown").expect("a shipped profile").text;
    let plan = ArchivePlan {
        authorised: authorised_by("as-shown"),
        ..plan_from(text, TARGET)
    };
    let (report, output) = convert_with_plan(&source, &plan);
    let output = output.unwrap_or_else(|| {
        panic!(
            "{:?}",
            conversion(&report)
                .decided
                .iter()
                .filter(|decided| matches!(decided.decision, Decision::Refused(_)))
                .map(|decided| format!("{}: {:?}", decided.requirement, decided.decision))
                .collect::<Vec<_>>()
        )
    });
    let held = holds(&output, TARGET);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
}

/// Page one's `/F1`, resolved, from the output document; and the document to resolve it in.
fn page_font(bytes: &[u8]) -> (Document, Dictionary) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("a page");
    let resources = document.get_key(&page.dict, "Resources");
    let fonts = document.get_key(resources.as_dict().expect("resources"), "Font");
    let font = document.get_key(fonts.as_dict().expect("fonts"), "F1");
    let font = font.as_dict().cloned().expect("/F1");
    (document, font)
}

/// One page whose content `shows` in a Type 3 font whose `/CharProcs` describes `/square` alone,
/// its `/Encoding` the value `encoding`.
fn a_type3_page(encoding: &str, shows: &str) -> Vec<u8> {
    let glyph = b"600 0 d0 0 0 500 500 re f";
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            format!("/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td {shows} ET").into_bytes(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
                 /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 7 0 R /Encoding {encoding} \
                 /FirstChar 97 /LastChar 98 /Widths [600 700] /Resources << >> >>"
            ),
            "<< /square 8 0 R >>".to_owned(),
        ],
        binary_objects: vec![stream(&format!("/Length {}", glyph.len()), glyph)],
        ..Conforming::part_two()
    }
    .build()
}

/// `source` converted under `as-if-printed`, the font-maps row answered by its rewrite, and the
/// output passing that row.
fn restated_as_read(source: &[u8]) -> Vec<u8> {
    assert!(fails(source, "fonts/font-maps-read-as-written"));
    let (report, output) = convert(source, TARGET, authorised_by("as-if-printed"));
    assert_eq!(
        decision(&report, "fonts/font-maps-read-as-written"),
        Decision::Authorised {
            losses: Losses::one(Loss::FontMapsAsRead),
            rewrite: Rewrite::FontMapsAsRead,
        }
    );
    let output = written(&report, output);
    assert!(!fails(&output, "fonts/font-maps-read-as-written"));
    output
}

/// ISO 32000-2 Table D.1 has no processor hold an encoding named `StandardEncoding`, so a Type 3
/// font's `/BaseEncoding` naming it names no table, and a `/Differences` code past one byte names
/// no glyph (§9.6.4 selects a Type 3 glyph by a single-byte code). The encoding is written as read
/// (ADR 3793): the base and the code past a byte go, and every code names the glyph it named, so
/// the output conforms.
#[test]
fn a_type3_encoding_is_written_as_read() {
    let source = a_type3_page(
        "<< /Type /Encoding /BaseEncoding /StandardEncoding \
         /Differences [97 /square /square 300 /square] >>",
        "(ab) Tj",
    );
    let output = restated_as_read(&source);
    let held = holds(&output, TARGET);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (document, font) = page_font(&output);
    let encoding = document.get_key(&font, "Encoding");
    let encoding = encoding.as_dict().expect("an encoding dictionary");
    assert!(encoding.get("BaseEncoding").is_none());
    let differences = document.get_key(encoding, "Differences");
    let names: Vec<String> = differences
        .as_array()
        .expect("the codes and names read")
        .iter()
        .map(|item| format!("{item:?}"))
        .collect();
    assert_eq!(names.len(), 3, "{names:?}");
    let read = pdf_model::type3::Type3Font::read(&document, &font, "F1").expect("it reads");
    assert_eq!(
        read.glyph_name(97).map(pdf_syntax::Name::as_bytes),
        Some(&b"square"[..])
    );
    assert_eq!(
        read.glyph_name(98).map(pdf_syntax::Name::as_bytes),
        Some(&b"square"[..])
    );
}

/// Table 110 requires a Type 3 font's `/Encoding` and makes it a dictionary. Named
/// `StandardEncoding`, it names no table here, so no code names a glyph; it is written as the
/// encoding dictionary that assigns no code (ADR 3793), which is what was read. The page shows no
/// code in it: a shown code would name no glyph, which is another row's to answer.
#[test]
fn a_type3_encoding_naming_no_table_is_written_as_the_dictionary_assigning_none() {
    let output = restated_as_read(&a_type3_page("/StandardEncoding", ""));
    let held = holds(&output, TARGET);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (document, font) = page_font(&output);
    let encoding = document.get_key(&font, "Encoding");
    let encoding = encoding.as_dict().expect("an encoding dictionary");
    let differences = document.get_key(encoding, "Differences");
    assert_eq!(differences.as_array().map(<[_]>::len), Some(0));
}

/// A font written directly inside the page's resources is restated where it is written (ADR
/// 3794): its `/ToUnicode` naming no stream is removed in place, and the output conforms.
#[test]
fn a_font_written_inside_another_object_is_restated_in_place() {
    let source = Conforming {
        resources: "/Font << /F1 << /Type /Font /Subtype /TrueType /BaseFont /LiberationSans \
                    /FirstChar 65 /LastChar 65 /Widths [667] /FontDescriptor 6 0 R \
                    /Encoding /WinAnsiEncoding /ToUnicode /Identity-H >> >> /ColorSpace << \
                    /CS0 [/CalGray << /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec(),
        )),
        objects: vec![
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 /FontFile2 7 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!(
                "/Length {} /Length1 {}",
                LIBERATION_SANS.len(),
                LIBERATION_SANS.len()
            ),
            LIBERATION_SANS,
        )],
        ..Conforming::part_two()
    }
    .build();
    let output = restated_as_read(&source);
    let held = holds(&output, TARGET);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (_, font) = page_font(&output);
    assert!(font.get("ToUnicode").is_none());
    assert!(font.get("Encoding").is_some());
}
