//! A repair this reader made inside a content stream, and a page box it could not read, answered
//! under ISO 19005-2 and ISO 19005-4 section 5.1 (`doc/adr/3557`, `doc/adr/3558`,
//! `doc/adr/3559`).
//!
//! The expected values come from ISO 32000-2: §8.4.2 ("Occurrences of the q and Q operators shall
//! be balanced"), so a `Q` with nothing saved restores nothing and the page draws the same without
//! it; §9.4.1, whose text-showing operators belong inside `BT … ET`, so one outside that this reader
//! still draws is a mark the page would lose; and §7.7.3.3's Table 31, which requires `/MediaBox`
//! and gives it no default, so the box a page with none is drawn on is this reader's
//! (`Page::DEFAULT_MEDIA_BOX`, ADR 0389) and is what the output states.
use super::*;
use pdf_archive::Flavour;
use pdf_transform::archive::Loss;

/// ISO 19005-2 and ISO 19005-4 section 5.1's row.
const BASE_STANDARD: &str = "conformance/adheres-to-the-base-standard";

/// A simple font with no program and no descriptor, which the conversion embeds a shipped face
/// for (ISO 19005-2 section 6.2.11.4.1).
const HELVETICA: &str =
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>";

/// A page whose one content stream is `data`, with Helvetica as `/F1`.
fn a_page_whose_stream_is(data: &[u8]) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((String::new(), data.to_vec())),
        objects: vec![HELVETICA.to_owned()],
        ..Conforming::part_two()
    }
    .build()
}

/// The plan a shipped profile makes for `target`, with the authorisations its `discard` steps
/// stand for.
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

/// The sentences of every refusal the conversion reported.
fn refusals(report: &Report) -> Vec<String> {
    conversion(report)
        .decided
        .iter()
        .filter_map(|decided| match decided.decision {
            Decision::Refused(because) => Some(because.sentence().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_restore_with_nothing_saved_is_written_out_where_the_page_draws_the_same() {
    let source = a_page_whose_stream_is(b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET Q");
    for profile in ["as-shown", "as-if-printed"] {
        for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
            let (report, output) = convert_with_plan(&source, &under(profile, target));
            let Decision::Authorised { losses, .. } = decision(&report, BASE_STANDARD) else {
                panic!("{profile}: {:?}", decision(&report, BASE_STANDARD));
            };
            assert!(
                losses.iter().eq([Loss::ContentAsDrawn]),
                "{profile}: {}",
                losses.words()
            );
            let output = output.expect("a file is written");
            assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
            let conversion = conversion(&report);
            assert!(
                conversion.photographed.is_empty(),
                "{profile}: the elegant remedy comes before the photograph"
            );
            let [written] = conversion.runs_written_as_drawn.as_slice() else {
                panic!("one operator: {:?}", conversion.runs_written_as_drawn);
            };
            assert_eq!(written.run, b"Q");
            assert_eq!(written.number, [] as [u8; 0]);
            let contents = decoded_contents(&output);
            let text = String::from_utf8_lossy(&contents);
            assert!(text.contains("(Hello) Tj ET"), "{text}");
            assert!(!text.contains('Q'), "the lone Q is gone: {text}");
        }
    }
}

#[test]
fn a_text_operator_the_page_draws_outside_a_text_object_keeps_its_refusal_or_is_photographed() {
    // `Td` and `Tj` with no `BT` before them: this reader draws the text where its text state
    // leaves it, so writing them out would take a mark off the page, and the proof refuses it.
    let source = a_page_whose_stream_is(b"/F1 12 Tf 10 100 Td (Hello) Tj");
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert_with_plan(&source, &under("as-shown", target));
    assert!(output.is_none(), "as-shown writes no file");
    let refused = refusals(&report);
    assert!(
        refused.iter().any(|sentence| sentence
            .contains("model/text-operator-outside-a-text-object")
            && sentence.contains("ADR 3558")),
        "refused by the class's name: {refused:?}"
    );
    let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
    let output = output.expect("as-if-printed photographs the page");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let Decision::Authorised { losses, .. } = decision(&report, BASE_STANDARD) else {
        panic!("{:?}", decision(&report, BASE_STANDARD));
    };
    assert!(losses.contains(Loss::PageAsImage), "{}", losses.words());
    assert!(!losses.contains(Loss::ContentAsDrawn), "{}", losses.words());
    assert_eq!(conversion(&report).photographed.len(), 1);
}

#[test]
fn a_media_box_that_is_no_rectangle_is_written_as_the_box_this_reader_drew() {
    let built = a_page_whose_stream_is(b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET");
    // Three numbers in place of four, the same length so every offset stands.
    let (was, now) = (
        &b"/MediaBox [0 0 200 200]"[..],
        &b"/MediaBox [0 0 200]    "[..],
    );
    let at = built
        .windows(was.len())
        .position(|window| window == was)
        .expect("the fixture states its media box");
    let mut source = built.clone();
    source[at..at + was.len()].copy_from_slice(now);
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert_with_plan(&source, &under("as-shown", target));
        let output = output.unwrap_or_else(|| panic!("{:?}", refusals(&report)));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let document = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
        let page = pdf_model::Pages::new(&document)
            .get(0)
            .expect("the page is there");
        assert_eq!(page.media_box, pdf_model::Page::DEFAULT_MEDIA_BOX);
        assert!(
            document
                .repairs()
                .iter()
                .all(|logged| logged.class.name != pdf_model::repair::MEDIA_BOX_SUBSTITUTED.name),
            "the output states the box, so reading it substitutes none"
        );
    }
}
