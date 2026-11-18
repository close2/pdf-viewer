//! A Type 3 glyph procedure whose `d0` width disagrees with its font's `/Widths` is written as the
//! font states it, under the loss `type3-width-as-drawn` (`doc/adr/2907`, `doc/questions/A865`).
//!
//! ISO 19005-4 section 6.2.10.5 holds the `d0` and `d1` operands of each glyph procedure to the
//! glyph's width, and ISO 32000-2 §9.6.4's Table 111 says which: "w x ... shall be consistent
//! with the corresponding width in the font's Widths array". §9.2.4 makes `/Widths` what advances
//! the text position, so the operand written is the number the page is already laid out by.

use super::statement::{holds_fact, stated};
use super::*;
use pdf_model::{Pages, interpret};
use pdf_syntax::object::Object;
use pdf_transform::profiles::{Contribution, named};

const TYPE3_WIDTHS: &str = "fonts/type3-glyph-procedures-state-their-width";

/// Part 4, the one part that states the row.
const PART_FOUR: Target = Target::Four(Flavour::Plain);

/// What a shipped profile contributes at part 4.
fn authorised_by(profile: &str) -> Authorisations {
    Contribution::read(named(profile).expect("a shipped profile").text, PART_FOUR)
        .expect("a shipped profile reads")
        .authorised
}

/// A page showing each code `differences` names in `/F1` and then `A` in `/F2`, two Type 3 fonts sharing one `/CharProcs`
/// and one glyph procedure, whose `d0` states 500: `/F1`'s `/Widths` states `widths_one` and `/F2`'s
/// `widths_two`, each over `differences` (one or two codes from 65).
fn two_fonts_over_one_procedure(widths_one: &str, widths_two: &str, differences: &str) -> Vec<u8> {
    let glyph = b"500 0 d0 0 0 500 500 re f";
    let (last, shown) = if differences.split_whitespace().count() > 2 {
        (66, "AB")
    } else {
        (65, "A")
    };
    let font = |widths: &str| {
        format!(
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
             /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 8 0 R /Encoding 9 0 R /FirstChar 65 \
             /LastChar {last} /Widths [{widths}] /Resources << >> >>"
        )
    };
    Conforming {
        resources: "/Font << /F1 6 0 R /F2 7 0 R >> /ColorSpace << /CS0 [/CalGray << \
                    /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            format!("/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td ({shown}) Tj /F2 12 Tf (A) Tj ET")
                .into_bytes(),
        )),
        objects: vec![
            font(widths_one),
            font(widths_two),
            "<< /A 10 0 R >>".to_owned(),
            format!("<< /Type /Encoding /Differences [{differences}] >>"),
        ],
        binary_objects: vec![stream(&format!("/Length {}", glyph.len()), glyph)],
        ..Conforming::part_two()
    }
    .build()
}

/// The `d0` width the procedure font `font` names for `A` states, in `bytes`.
fn d0_of(bytes: &[u8], font: u32) -> String {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let dict = held
        .get(ObjectId::new(font, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    let procedures = held.get_key(&dict, "CharProcs");
    let procedures = procedures.as_dict().expect("CharProcs");
    let Some(Object::Reference(procedure)) = procedures.get("A") else {
        panic!("/A is a procedure: {procedures:?}");
    };
    let Object::Stream(stream) = held.get(*procedure) else {
        panic!("a stream");
    };
    let data = held.decoded_stream_data(&stream).expect("it decodes");
    String::from_utf8_lossy(&data)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// Where each character of page one begins, as this reader lays it out.
fn laid_out(bytes: &[u8]) -> Vec<f32> {
    let document = Document::open(bytes.to_vec()).expect("it opens");
    let page = Pages::new(&document).get(0).expect("a page");
    interpret(&document, &page)
        .text_layer
        .iter()
        .map(|placed| placed.quad[0])
        .collect()
}

#[test]
fn both_shipped_profiles_write_the_width_the_page_is_laid_out_by() {
    // `/F1` states 1000 where its procedure states 500, and `/F2` 500, which the procedure already
    // states: so `/F1` is given a copy stating 1000 and `/F2` keeps the producer's procedure. The
    // page is laid out by `/Widths` before and after (§9.2.4), and nothing is photographed.
    let source = two_fonts_over_one_procedure("1000", "500", "65 /A");
    for profile in ["as-shown", "as-if-printed"] {
        let (report, output) = convert(&source, PART_FOUR, authorised_by(profile));
        let Decision::Authorised { losses, .. } = decision(&report, TYPE3_WIDTHS) else {
            panic!("{profile}: {:?}", decision(&report, TYPE3_WIDTHS));
        };
        assert!(
            losses.iter().eq([Loss::Type3WidthAsDrawn]),
            "{profile}: {}",
            losses.words()
        );
        let output = written(&report, output);
        assert_eq!(
            holds(&output, PART_FOUR).verdict(),
            Verdict::Conforms,
            "{profile}"
        );
        assert!(conversion(&report).photographed.is_empty(), "{profile}");
        assert_eq!(d0_of(&output, 6), "1000", "{profile}: /F1's copy");
        assert_eq!(d0_of(&output, 7), "500", "{profile}: /F2's own");
        assert_eq!(d0_of(&source, 6), "500", "the source is untouched");
        assert_eq!(
            laid_out(&output),
            laid_out(&source),
            "{profile}: no glyph moves"
        );
        let restated = &conversion(&report).restated_type3_widths;
        assert_eq!(restated.len(), 1, "{profile}: {restated:?}");
        assert_eq!(restated[0].font, Some(ObjectId::new(6, 0)));
        assert_eq!(restated[0].was, "500");
        assert!((restated[0].width - 1000.0).abs() < f64::EPSILON);
        assert_ne!(restated[0].written_at, restated[0].procedure, "a copy");
        assert!(
            output
                .windows(b"type3-width-as-drawn".len())
                .any(|window| window == b"type3-width-as-drawn"),
            "{profile}: the file's own history names the loss"
        );
    }
}

#[test]
fn a_procedure_every_font_needs_one_width_for_is_written_in_place() {
    // Both fonts state 1000, so one operand answers both and no copy is made.
    let source = two_fonts_over_one_procedure("1000", "1000", "65 /A");
    let (report, output) = convert(&source, PART_FOUR, authorised_by("as-shown"));
    let output = written(&report, output);
    assert_eq!(holds(&output, PART_FOUR).verdict(), Verdict::Conforms);
    assert_eq!(d0_of(&output, 6), "1000");
    assert_eq!(d0_of(&output, 7), "1000");
    let restated = &conversion(&report).restated_type3_widths;
    assert_eq!(restated.len(), 2, "one per font: {restated:?}");
    assert!(
        restated
            .iter()
            .all(|written| written.written_at == written.procedure),
        "in place: {restated:?}"
    );
}

#[test]
fn refuse_any_loss_stops_on_it() {
    let source = two_fonts_over_one_procedure("1000", "500", "65 /A");
    let (report, output) = convert(&source, PART_FOUR, authorised_by("refuse-any-loss"));
    assert!(output.is_none(), "nothing is written");
    let Decision::Unauthorised { losses, .. } = decision(&report, TYPE3_WIDTHS) else {
        panic!("{:?}", decision(&report, TYPE3_WIDTHS));
    };
    assert!(
        losses.contains(Loss::Type3WidthAsDrawn),
        "{}",
        losses.words()
    );
}

#[test]
fn one_font_reaching_one_procedure_at_two_widths_is_given_a_name_for_the_second() {
    // Codes 65 and 66 both name `/A` in `/F1`, at 1000 and 700, and `/F2` reaches it at 500, which
    // the procedure states: `/F2` keeps the producer's procedure, `/F1`'s `/A` names a copy at 1000,
    // and code 66 is given `/A.1`, a copy at 700 — a name the Adobe Glyph List's rule reads as `A`
    // — its `/Differences` entry written with it. No glyph moves (§9.2.4).
    let source = two_fonts_over_one_procedure("1000 700", "500 500", "65 /A /A");
    let (report, output) = convert(&source, PART_FOUR, authorised_by("as-shown"));
    let output = written(&report, output);
    assert_eq!(holds(&output, PART_FOUR).verdict(), Verdict::Conforms);
    assert_eq!(d0_of(&output, 6), "1000");
    assert_eq!(d0_of(&output, 7), "500");
    assert_eq!(laid_out(&output), laid_out(&source), "no glyph moves");
    let held = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
    let font = held.get(ObjectId::new(6, 0));
    let font = font.as_dict().expect("the font");
    assert_eq!(
        pdf_archive::type3_encoding(&held, font),
        [(65, "A".to_owned()), (66, "A.1".to_owned())]
    );
    let restated = &conversion(&report).restated_type3_widths;
    assert_eq!(restated.len(), 2, "{restated:?}");
}

#[test]
fn a_text_layer_reads_the_renamed_code_as_it_did() {
    // The readback is the same before and after: the renamed `/A.1` is read as `A`.
    let source = two_fonts_over_one_procedure("1000 700", "500 500", "65 /A /A");
    let (report, output) = convert(&source, PART_FOUR, authorised_by("as-shown"));
    let output = written(&report, output);
    let text = |bytes: &[u8]| {
        let document = Document::open(bytes.to_vec()).expect("it opens");
        let page = Pages::new(&document).get(0).expect("a page");
        interpret(&document, &page).text
    };
    assert!(text(&source).contains('A'), "{:?}", text(&source));
    assert_eq!(text(&output), text(&source));
}

/// The object number of the font page one's resources name `name`.
fn font_on_page_one(bytes: &[u8], name: &str) -> u32 {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = Pages::new(&document).get(0).expect("a page");
    let resources = document.get_key(&page.dict, "Resources");
    let fonts = document.get_key(resources.as_dict().expect("resources"), "Font");
    match fonts.as_dict().and_then(|fonts| fonts.get(name)) {
        Some(Object::Reference(font)) => font.number,
        other => panic!("/{name} is a font object: {other:?}"),
    }
}

/// A plan converting to part 4 under a shipped profile, its statement steps included.
fn under(profile: &str) -> ArchivePlan {
    let text = named(profile).expect("a shipped profile").text;
    let config = pdf_transform::archive::Configuration::read(text, PART_FOUR).expect("it reads");
    ArchivePlan {
        authorised: config.authorisations(PART_FOUR),
        ..plan_from(text, PART_FOUR)
    }
}

#[test]
fn state_every_loss_writes_each_width_and_states_it_on_its_appended_page() {
    // `state-every-loss` takes every loss it states (its profile's first line), and this one is
    // stated in facts read from the file: the font, the glyph, the procedure the producer wrote,
    // the operand it held and the width written (ADR 2917, CLAUDE.md's first near-side
    // construction). So the width is written as under the other two profiles, the page keeps its
    // text, and the appended page lists the width.
    let source = two_fonts_over_one_procedure("1000", "500", "65 /A");
    let (report, output) = convert_with_plan(&source, &under("state-every-loss"));
    let Decision::Authorised { losses, .. } = decision(&report, TYPE3_WIDTHS) else {
        panic!("{:?}", decision(&report, TYPE3_WIDTHS));
    };
    assert!(
        losses.iter().eq([Loss::Type3WidthAsDrawn]),
        "{}",
        losses.words()
    );
    let output = written(&report, output);
    assert_eq!(holds(&output, PART_FOUR).verdict(), Verdict::Conforms);
    let conversion = conversion(&report);
    assert_eq!(conversion.photographed, []);
    // The statement page's own objects are numbered among the source's, so `/F1` is found by
    // its resource name rather than by its number in the source.
    assert_eq!(
        d0_of(&output, font_on_page_one(&output, "F1")),
        "1000",
        "/F1's copy"
    );
    let statement = conversion.statement.as_ref().expect("a statement page");
    assert!(
        statement.sites.contains(&TYPE3_WIDTHS),
        "{:?}",
        statement.sites
    );
    let page = stated(&output, statement);
    // Each fact read from the source here, not from the conversion's record.
    let procedure = {
        let held = Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("it opens");
        let procedures = held.get(ObjectId::new(8, 0));
        let Some(Object::Reference(procedure)) = procedures
            .as_dict()
            .and_then(|procedures| procedures.get("A"))
            .cloned()
        else {
            panic!("/A names a procedure");
        };
        procedure
    };
    for fact in [
        format!("Requirement: {TYPE3_WIDTHS} (ISO 19005-4 section 6.2.10.5)"),
        "Loss: type3-width-as-drawn".to_owned(),
        "Font: dictionary 6 0".to_owned(),
        "Glyph: /A".to_owned(),
        format!(
            "Procedure: stream {} {}",
            procedure.number, procedure.generation
        ),
        format!("Width as the producer wrote it: {} d0", d0_of(&source, 6)),
        "Width written: 1000".to_owned(),
    ] {
        assert!(holds_fact(&page, &fact), "{fact}:\n{page}");
    }
    assert!(
        !holds_fact(&page, "Font: dictionary 7 0"),
        "/F2 keeps the producer's procedure and loses nothing:\n{page}"
    );
}
