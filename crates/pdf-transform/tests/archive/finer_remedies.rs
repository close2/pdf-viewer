//! Three of R03's fenced rows answered by a finer remedy than a refusal (`doc/pdfa-blocking-issues.md`
//! section 5): a media box past ISO 19005-2 section 6.1.13's range restated in larger units
//! (`doc/adr/2888`), an `/ActualText` holding a Private Use character removed where an object states
//! it (`doc/adr/2889`), and a transfer function's page photographed under `as-if-printed`
//! (`doc/adr/2889`).

use super::*;
use pdf_transform::profiles::{Contribution, named};

const BOUNDARIES: &str = "implementation-limits/page-boundary-sizes";
const ACTUAL_TEXT: &str = "fonts/actual-text-states-no-private-use";
const TRANSFER: &str = "graphics/no-transfer-function-in-a-graphics-state";

/// What a shipped profile contributes at `target`, as the command line and a window fold it in.
fn authorised_by(profile: &str, target: Target) -> Authorisations {
    Contribution::read(named(profile).expect("a shipped profile").text, target)
        .expect("a shipped profile reads")
        .authorised
}

/// A part 2 page whose media box is `media`, written in the fixture's own thirteen bytes so that
/// no cross-reference offset moves, with `page` added to its dictionary.
fn a_page_with_a_media_box(media: &str, page: &str) -> Vec<u8> {
    assert_eq!(
        media.len(),
        "[0 0 200 200]".len(),
        "the replacement keeps every offset"
    );
    let built = Conforming {
        page: page.to_owned(),
        contents: Some((String::new(), b"q 0 0 1 rg 10 1 100 2 re f Q".to_vec())),
        ..Conforming::part_two()
    }
    .build();
    let at = built
        .windows(13)
        .position(|window| window == b"[0 0 200 200]")
        .expect("the fixture's media box");
    let mut out = built;
    out[at..at.saturating_add(13)].copy_from_slice(media.as_bytes());
    out
}

/// The page object of a converted file.
fn first_page(bytes: &[u8]) -> (Document, pdf_model::Page) {
    let document =
        Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("the output opens");
    let page = pdf_model::Pages::new(&document)
        .every()
        .into_iter()
        .next()
        .flatten()
        .expect("a first page");
    (document, page)
}

fn numbers(document: &Document, page: &pdf_model::Page, key: &str) -> Vec<f64> {
    document
        .get_key(&page.dict, key)
        .as_array()
        .expect("an array")
        .iter()
        .map(|item| item.as_number().expect("a number"))
        .collect()
}

#[test]
fn a_media_box_past_the_range_is_restated_in_larger_units() {
    // ISO 19005-2 section 6.1.13 holds every page boundary to at most 14 400 units, and ISO
    // 32000-2 §7.7.3.3's Table 31 makes `/UserUnit` the size of a unit. A box 16 000 units wide is
    // stated in units twice as large: 8 000 by 4.5, `/UserUnit` 2, and the producer's content
    // drawn under `0.5 0 0 0.5 0 0 cm`, so every mark keeps its place measured in inches.
    let source = a_page_with_a_media_box("[0 0 16000 9]", "");
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert_eq!(
        decision(&report, BOUNDARIES),
        Decision::Mechanical(Rewrite::PageBoundaryRemoved)
    );
    let output = written(&report, output);
    let held = holds(&output, Target::Two(Level::B));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (document, page) = first_page(&output);
    assert_eq!(
        numbers(&document, &page, "MediaBox"),
        [0.0, 0.0, 8000.0, 4.5]
    );
    assert_eq!(
        document.get_key(&page.dict, "UserUnit").as_number(),
        Some(2.0)
    );
    let (content, issues) = page.content_with_report(&document);
    assert!(issues.is_empty(), "{issues:?}");
    let content = String::from_utf8_lossy(&content);
    assert!(
        content.starts_with("q\n0.5 0 0 0.5 0 0 cm\n"),
        "the bracket opens the content: {content}"
    );
    assert!(
        content.contains("q 0 0 1 rg 10 1 100 2 re f Q"),
        "the producer's operators stay"
    );
    assert!(
        content.trim_end().ends_with('Q'),
        "and it closes: {content}"
    );
    let restated = &conversion(&report).restated_pages;
    assert_eq!(restated.len(), 1, "{restated:?}");
    assert_eq!(
        (
            restated[0].page,
            restated[0].factor,
            restated[0].user_unit_now
        ),
        (1, 2.0, 2.0)
    );
}

#[test]
fn a_media_box_under_the_range_is_restated_in_smaller_units() {
    // The same limit's lower bound, 3 units: a 2.5-unit box is counted in half units, 5 by 5,
    // `/UserUnit` 0.5, and the content drawn under `2 0 0 2 0 0 cm`.
    let source = a_page_with_a_media_box("[0 0 2.5 2.5]", "");
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let output = written(&report, output);
    let held = holds(&output, Target::Two(Level::B));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (document, page) = first_page(&output);
    assert_eq!(numbers(&document, &page, "MediaBox"), [0.0, 0.0, 5.0, 5.0]);
    assert_eq!(
        document.get_key(&page.dict, "UserUnit").as_number(),
        Some(0.5)
    );
    let (content, _) = page.content_with_report(&document);
    assert!(String::from_utf8_lossy(&content).starts_with("q\n2 0 0 2 0 0 cm\n"));
}

#[test]
fn a_media_box_with_no_width_and_a_page_with_annotations_stay_refused() {
    // A side of no length is three units in no unit at all, and an annotation's `/Rect` (§12.5.2)
    // counts the page's default units, which the bracket's `cm` does not reach.
    for (media, page) in [
        ("[0 0 0   200]", ""),
        (
            "[0 0 16000 9]",
            "/Annots [<< /Type /Annot /Subtype /Square /Rect [0 0 4 4] /F 4 >>]",
        ),
    ] {
        let source = a_page_with_a_media_box(media, page);
        let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
        assert!(output.is_none(), "{media} {page}");
        assert!(
            matches!(decision(&report, BOUNDARIES), Decision::Refused(_)),
            "{media} {page}: {:?}",
            decision(&report, BOUNDARIES)
        );
    }
}

/// A part 4 file whose structure element states `/ActualText <FEFFF0B7>`, U+F0B7 being in the
/// Private Use Area.
fn a_private_use_actual_text() -> Vec<u8> {
    Conforming {
        catalog: "/StructTreeRoot 6 0 R".to_owned(),
        objects: vec![
            "<< /Type /StructTreeRoot /K 7 0 R >>".to_owned(),
            "<< /Type /StructElem /S /Span /P 6 0 R /ActualText <FEFFF0B7> >>".to_owned(),
        ],
        contents: Some((String::new(), b"q 0 0 1 rg 10 10 50 50 re f Q".to_vec())),
        ..Conforming::default()
    }
    .build()
}

#[test]
fn a_private_use_actual_text_is_a_loss_nobody_authorised() {
    // ISO 19005-4 section 6.2.10.8: the entry shall contain no Private Use value. Removing it takes
    // the producer's value away, so a caller who authorised nothing is refused.
    let (report, output) = to_part_four(&a_private_use_actual_text());
    assert!(output.is_none());
    assert!(matches!(
        decision(&report, ACTUAL_TEXT),
        Decision::Unauthorised { losses, rewrite: Rewrite::ActualTextRemoved }
            if losses == Losses::one(Loss::ActualTextPrivateUse)
    ));
}

#[test]
fn a_private_use_actual_text_is_removed_under_both_shipped_profiles() {
    // ISO 32000-2 §14.9.4 makes the entry optional, so the structure element keeps its place and
    // loses the value; the report names the object and the value it held.
    for profile in ["as-shown", "as-if-printed"] {
        let target = Target::Four(Flavour::Plain);
        let (report, output) = convert(
            &a_private_use_actual_text(),
            target,
            authorised_by(profile, target),
        );
        assert_eq!(
            decision(&report, ACTUAL_TEXT),
            Decision::Authorised {
                losses: Losses::one(Loss::ActualTextPrivateUse),
                rewrite: Rewrite::ActualTextRemoved,
            },
            "{profile}"
        );
        let output = written(&report, output);
        let held = holds(&output, target);
        assert_eq!(
            held.verdict(),
            Verdict::Conforms,
            "{profile}: {}",
            held.render()
        );
        assert!(
            !String::from_utf8_lossy(&output).contains("ActualText"),
            "{profile}"
        );
        let removed = &conversion(&report).removed_actual_texts;
        assert_eq!(removed.len(), 1, "{profile}");
        assert_eq!(
            (removed[0].object, removed[0].value.as_str()),
            (7, "U+F0B7")
        );
    }
}

#[test]
fn an_actual_text_inside_a_content_stream_is_not_removed_here() {
    // An entry in a `BDC` operand is inside the producer's content stream (ADR 0816's fence), so
    // `as-shown`, which takes no photograph of it, keeps the refusal.
    let source = Conforming {
        contents: Some((
            String::new(),
            b"/Span << /ActualText <FEFFF0B7> >> BDC 0 0 1 rg 10 10 50 50 re f EMC".to_vec(),
        )),
        ..Conforming::default()
    }
    .build();
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert(&source, target, authorised_by("as-shown", target));
    assert!(output.is_none());
    assert!(matches!(
        decision(&report, ACTUAL_TEXT),
        Decision::Refused(_)
    ));
}

/// A page whose one graphics state inverts its grey through `/TR`.
fn an_inverting_transfer_function() -> Vec<u8> {
    Conforming {
        resources: "/ExtGState << /G1 6 0 R >>".to_owned(),
        objects: vec![
            "<< /Type /ExtGState /TR 7 0 R >>".to_owned(),
            "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>".to_owned(),
        ],
        contents: Some((String::new(), b"/G1 gs 0 0 1 rg 10 10 50 50 re f".to_vec())),
        ..Conforming::default()
    }
    .build()
}

#[test]
fn a_transfer_functions_page_is_photographed_as_if_printed_and_stays_refused_as_shown() {
    // ISO 32000-2 §10.5: "the PDF processor shall apply the transfer function", so a print applies
    // it and a print scanned back in shows its colours; the photograph is this reader's drawing
    // with the function applied, and the state that stated it leaves with the page's resources.
    // `as-shown` keeps ADR 2394's gate (A694 item 1).
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert(
        &an_inverting_transfer_function(),
        target,
        authorised_by("as-if-printed", target),
    );
    let Decision::Authorised { losses, .. } = decision(&report, TRANSFER) else {
        panic!("{:?}", decision(&report, TRANSFER));
    };
    assert!(
        losses.iter().eq([Loss::TextAsImage, Loss::PageAsImage]),
        "{}",
        losses.words()
    );
    let output = written(&report, output);
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let document =
        Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("the output opens");
    let states_tr = document.xref().object_numbers().any(|number| {
        document
            .get(document.defined_id(number))
            .as_dict()
            .is_some_and(|dict| dict.get("TR").is_some())
    });
    assert!(!states_tr, "no graphics state of the output states /TR");
    assert_eq!(conversion(&report).photographed.len(), 1);

    let (report, output) = convert(
        &an_inverting_transfer_function(),
        target,
        authorised_by("as-shown", target),
    );
    assert!(output.is_none());
    assert!(!matches!(
        decision(&report, TRANSFER),
        Decision::Authorised { .. }
    ));
}
