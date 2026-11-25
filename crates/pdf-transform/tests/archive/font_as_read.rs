//! A font this reader read past, written as it read it (ADRs 3569 and 3570): a program whose table
//! directory names a table past its end, and a `/ToUnicode` and an `/Encoding` that name nothing
//! ISO 32000-2 defines.

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
