//! Three of R03's fenced rows answered by a finer remedy than a refusal (`doc/pdfa-blocking-issues.md`
//! section 5): a media box past ISO 19005-2 section 6.1.13's range restated in larger units
//! (`doc/adr/2888`) or, with a side of no length, as the box this viewer draws (`doc/adr/3142`), an `/ActualText` holding a Private Use character removed where an object states
//! it (`doc/adr/2889`), and a transfer function's page photographed under `as-if-printed`
//! (`doc/adr/2889`).

use super::*;
use pdf_syntax::object::Object;
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
fn a_media_box_with_no_width_is_written_as_the_box_this_viewer_draws() {
    // A side of no length is three units in no unit at all, so no `/UserUnit` answers it. ISO
    // 32000-2 §7.9.5 admits the rectangle and §7.7.3.3's Table 31 requires the entry with no
    // default, and this viewer draws such a page on `Page::DEFAULT_MEDIA_BOX` (ADR 0389). The
    // output states that box, and a crop box of no area, read as absent, is written as
    // §14.11.2.1's default, the media box (`doc/adr/3142`). The unit is the page's own, so the
    // content takes no bracket.
    let drawn = pdf_model::Page::DEFAULT_MEDIA_BOX.map(f64::from);
    for page in ["", "/CropBox [0 0 0 0]"] {
        let source = a_page_with_a_media_box("[0 0 0   200]", page);
        let (_, before) = first_page(&source);
        assert_eq!(
            before.media_box,
            pdf_model::Page::DEFAULT_MEDIA_BOX,
            "{page}"
        );
        let target = Target::Two(Level::B);
        let (report, output) = convert(&source, target, Authorisations::default());
        assert_eq!(
            decision(&report, BOUNDARIES),
            Decision::Mechanical(Rewrite::PageBoundaryRemoved),
            "{page}"
        );
        let output = written(&report, output);
        let held = holds(&output, target);
        assert_eq!(
            held.verdict(),
            Verdict::Conforms,
            "{page}: {}",
            held.render()
        );
        let (document, after) = first_page(&output);
        assert_eq!(numbers(&document, &after, "MediaBox"), drawn, "{page}");
        assert_eq!(after.media_box, before.media_box, "{page}");
        assert_eq!(after.crop_box, before.crop_box, "{page}");
        assert_eq!(
            after.substituted_media_box, None,
            "{page}: the box is stated now"
        );
        if page.starts_with("/CropBox") {
            assert_eq!(numbers(&document, &after, "CropBox"), drawn, "{page}");
        }
        assert!(
            document
                .get_key(&after.dict, "UserUnit")
                .as_number()
                .is_none(),
            "{page}: the unit is the page's own"
        );
        let (content, _) = after.content_with_report(&document);
        assert_eq!(
            String::from_utf8_lossy(&content).trim(),
            "q 0 0 1 rg 10 1 100 2 re f Q",
            "{page}: the producer's content, unbracketed"
        );
        let restated = &conversion(&report).restated_pages;
        assert_eq!(restated.len(), 1, "{page}: {restated:?}");
        assert_eq!(
            (
                restated[0].page,
                restated[0].factor,
                restated[0].user_unit_now
            ),
            (1, 1.0, 1.0),
            "{page}"
        );
        assert_eq!(restated[0].boxes[0].0, "MediaBox", "{page}");
        assert_eq!(restated[0].boxes[0].2, drawn, "{page}");
    }
}

#[test]
fn a_page_with_annotations_past_the_range_stays_refused() {
    // An annotation's `/Rect` (§12.5.2) counts the page's default units, which the bracket's `cm`
    // does not reach.
    let page = "/Annots [<< /Type /Annot /Subtype /Square /Rect [0 0 4 4] /F 4 >>]";
    let source = a_page_with_a_media_box("[0 0 16000 9]", page);
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert!(output.is_none());
    assert!(
        matches!(decision(&report, BOUNDARIES), Decision::Refused(_)),
        "{:?}",
        decision(&report, BOUNDARIES)
    );
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

const SECOND_TRANSFER: &str = "graphics/second-transfer-function-is-default";
const HALFTONE_TRANSFER: &str = "graphics/halftone-transfer-function-only-where-required";

/// A page whose one graphics state states `/TR2` as `value`, with an inverting function as object
/// 7, at `target`'s part.
fn a_second_transfer_function(value: &str, part_two: bool) -> Vec<u8> {
    let base = if part_two {
        Conforming::part_two()
    } else {
        Conforming::default()
    };
    Conforming {
        resources: "/ExtGState << /G1 6 0 R >>".to_owned(),
        objects: vec![
            format!("<< /Type /ExtGState /TR2 {value} >>"),
            "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>".to_owned(),
        ],
        contents: Some((String::new(), b"/G1 gs 0 0 1 rg 10 10 50 50 re f".to_vec())),
        ..base
    }
    .build()
}

/// A part 2 page whose graphics state's `HT` is a type 5 halftone whose `Cyan` component states
/// `TransferFunction` as `value`; object 10 is an inverting function.
///
/// ISO 32000-2 §10.6.5.6 lists cyan among `DeviceCMYK`'s primary components, for which the
/// function is not required, so ISO 19005-2 section 6.2.5 forbids it there.
fn a_halftone_transfer_function(value: &str) -> Vec<u8> {
    Conforming {
        resources: "/ExtGState << /G1 6 0 R >>".to_owned(),
        objects: vec![
            "<< /Type /ExtGState /HT 7 0 R >>".to_owned(),
            "<< /Type /Halftone /HalftoneType 5 /Cyan 8 0 R /Default 9 0 R >>".to_owned(),
            format!(
                "<< /Type /Halftone /HalftoneType 1 /Frequency 60 /Angle 15 /SpotFunction \
                 /Round /TransferFunction {value} >>"
            ),
            "<< /Type /Halftone /HalftoneType 1 /Frequency 60 /Angle 45 /SpotFunction /Round >>"
                .to_owned(),
            "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>".to_owned(),
        ],
        contents: Some((String::new(), b"/G1 gs 0 0 1 rg 10 10 50 50 re f".to_vec())),
        ..Conforming::part_two()
    }
    .build()
}

/// Whether any object of `output` states `key`.
fn any_object_states(output: &[u8], key: &str) -> bool {
    let document =
        Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("the output opens");
    document.xref().object_numbers().any(|number| {
        let object = document.get(document.defined_id(number));
        match &object {
            Object::Dictionary(dict) => dict.get(key).is_some(),
            Object::Stream(stream) => stream.dict.get(key).is_some(),
            _ => false,
        }
    })
}

/// Converts `source` under both shipped profiles, and checks that `as-if-printed` photographs its
/// one page for `row` and leaves no object stating `key`, and that `as-shown` refuses it.
fn photographed_as_if_printed_and_refused_as_shown(
    source: &[u8],
    target: Target,
    row: &str,
    key: &str,
) {
    let (report, output) = convert(source, target, authorised_by("as-if-printed", target));
    let Decision::Authorised { losses, .. } = decision(&report, row) else {
        panic!("{target}: {:?}", decision(&report, row));
    };
    assert!(
        losses.iter().eq([Loss::TextAsImage, Loss::PageAsImage]),
        "{target}: {}",
        losses.words()
    );
    assert_eq!(conversion(&report).photographed.len(), 1, "{target}");
    let output = written(&report, output);
    let held = holds(&output, target);
    assert_eq!(
        held.verdict(),
        Verdict::Conforms,
        "{target}: {}",
        held.render()
    );
    assert!(
        !any_object_states(&output, key),
        "{target}: no object states /{key}"
    );

    let (report, output) = convert(source, target, authorised_by("as-shown", target));
    assert!(output.is_none(), "{target}");
    assert!(
        !matches!(decision(&report, row), Decision::Authorised { .. }),
        "{target}: {:?}",
        decision(&report, row)
    );
}

#[test]
fn a_second_transfer_functions_page_is_photographed_as_if_printed_and_stays_refused_as_shown() {
    // ISO 32000-2 §8.4.5's Table 57: TR2 is "[s]ame as TR except that the value may also be the
    // name Default", and ISO 19005-2 and -4 section 6.2.5 admit only Default. A function there is
    // one §10.5 has "the PDF processor shall apply", so a print carries it, and the photograph is
    // this reader's drawing with it applied (ADR 2893).
    for (target, part_two) in [
        (Target::Two(Level::B), true),
        (Target::Four(Flavour::Plain), false),
    ] {
        photographed_as_if_printed_and_refused_as_shown(
            &a_second_transfer_function("7 0 R", part_two),
            target,
            SECOND_TRANSFER,
            "TR2",
        );
    }
}

#[test]
fn a_halftones_transfer_functions_page_is_photographed_as_if_printed_and_stays_refused_as_shown() {
    // ISO 32000-2 §10.5: "[a] transfer function specified in a halftone dictionary shall override
    // the corresponding one specified by the current transfer function parameter in the graphics
    // state", so it is applied as a state's is. ISO 19005-2 section 6.2.5 admits it only where
    // §10.6.5.6 requires it, and cyan is a primary component, so it is forbidden here; the type 5
    // halftone holding it is reached through the state's HT (ADR 2893).
    photographed_as_if_printed_and_refused_as_shown(
        &a_halftone_transfer_function("10 0 R"),
        Target::Two(Level::B),
        HALFTONE_TRANSFER,
        "TransferFunction",
    );
}

#[test]
fn an_identity_second_or_halftone_transfer_function_is_removed_and_nothing_is_photographed() {
    // Table 57: "[t]he name Identity may be used to represent the Identity function", the
    // function in force on this reader's device at the start of the page (ADR 1964), so writing a
    // TR2 Default or removing a halftone's Identity moves no colour: the row is a statement under
    // either profile, and no page is photographed for it.
    for (source, row, key) in [
        (
            a_second_transfer_function("/Identity", true),
            SECOND_TRANSFER,
            "TR2",
        ),
        (
            a_halftone_transfer_function("/Identity"),
            HALFTONE_TRANSFER,
            "TransferFunction",
        ),
    ] {
        for profile in ["as-shown", "as-if-printed"] {
            let target = Target::Two(Level::B);
            let (report, output) = convert(&source, target, authorised_by(profile, target));
            assert!(
                matches!(decision(&report, row), Decision::Stated { .. }),
                "{profile} {row}: {:?}",
                decision(&report, row)
            );
            assert!(
                conversion(&report).photographed.is_empty(),
                "{profile} {row}"
            );
            let output = written(&report, output);
            let held = holds(&output, target);
            assert_eq!(
                held.verdict(),
                Verdict::Conforms,
                "{profile}: {}",
                held.render()
            );
            if key == "TransferFunction" {
                assert!(!any_object_states(&output, key), "{profile}");
            }
        }
    }
}
