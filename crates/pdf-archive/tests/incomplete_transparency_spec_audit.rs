//! Valid exact-xref regressions for a truncated Annex Q walk.
//!
//! The expected behavior for the over-limit documents is deliberately only that the affected
//! PDF/A row is not `Met`: the checker may refuse or mark it unchecked, but cannot certify a
//! requirement from the prefix it read. Each test checks one affected PDF/A row, not whole-document conformance.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "standalone generated exact-xref PDF"
)]

use std::fmt::Write as _;

use pdf_archive::{Flavour, Level, Outcome, Target, check};
use pdf_syntax::Document;

fn transparency_row(target: Target) -> &'static str {
    match target {
        Target::Two(_) => "graphics/a-transparent-page-has-a-blending-space",
        Target::Four(_) => "graphics/a-transparent-page-has-a-blending-space-or-an-output-intent",
    }
}

fn exact_xref_pdf(objects: &[String], target: Target) -> Document {
    let mut bytes = String::from(match target {
        Target::Two(_) => "%PDF-1.7\n",
        Target::Four(_) => "%PDF-2.0\n",
    });
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899aabbccddeeff> <ffeeddccbbaa99887766554433221100>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(bytes.into_bytes()).expect("exact-xref PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture xref is authoritative"
    );
    document
}

fn stream(content: &str, extra_dictionary: &str) -> String {
    format!(
        "<< /Length {} {extra_dictionary} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    )
}

fn page_document(
    resources: &str,
    page_content: &str,
    forms: Vec<String>,
    target: Target,
) -> Document {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources {resources} >>"
        ),
        stream(page_content, ""),
    ];
    objects.extend(forms);
    exact_xref_pdf(&objects, target)
}

fn painted_form(content: &str, resources: &str) -> String {
    stream(
        content,
        &format!("/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources {resources}"),
    )
}

fn row_outcome(document: &Document, target: Target) -> Outcome {
    check(document, target)
        .judgements
        .into_iter()
        .find(|judgement| judgement.id == transparency_row(target))
        .expect("the selected transparency requirement applies")
        .outcome
}

fn nested_forms(count: usize, target: Target) -> Document {
    assert!(count > 0);
    let mut forms = Vec::with_capacity(count);
    for index in 0..count {
        if index + 1 < count {
            let content = "/Next Do";
            forms.push(painted_form(
                content,
                &format!("<< /XObject << /Next {} 0 R >> >>", 6 + index),
            ));
        } else {
            let content = "/GS gs 0 0 5 5 re f";
            forms.push(painted_form(
                content,
                "<< /ExtGState << /GS << /Type /ExtGState /ca 0.5 >> >> >>",
            ));
        }
    }
    page_document(
        "<< /XObject << /First 5 0 R >> >>",
        "/First Do",
        forms,
        target,
    )
}

/// Eight nested Forms are within `MAX_FORM_DEPTH`; the ninth is reached at the stopping boundary.
/// The control detects the last form, while the candidate proves that truncation cannot certify
/// the PDF/A row as met.
fn depth_case(target: Target) {
    let within_limit = nested_forms(8, target);
    let survey = pdf_archive::survey::Survey::of(&within_limit);
    assert!(!survey.stopped_short());
    assert!(survey.page_is_transparent(0));
    assert!(matches!(
        row_outcome(&within_limit, target),
        Outcome::Failed { .. }
    ));

    let beyond_limit = nested_forms(9, target);
    let survey = pdf_archive::survey::Survey::of(&beyond_limit);

    assert!(
        !matches!(row_outcome(&beyond_limit, target), Outcome::Met),
        "an incomplete Annex Q walk must not report the requirement Met: stopped={}, observed_transparency={}",
        survey.stopped_short(),
        survey.page_is_transparent(0)
    );
}

fn sibling_forms(count: usize, target: Target) -> Document {
    let mut xobjects = String::new();
    let mut content = String::new();
    let mut forms = Vec::with_capacity(count);
    for index in 0..count {
        let object = 5 + index;
        let _ = write!(xobjects, "/F{index} {object} 0 R ");
        let _ = writeln!(content, "/F{index} Do");
        let body = if index + 1 == count {
            "/GS gs 0 0 5 5 re f"
        } else {
            "q Q"
        };
        let resources = if index + 1 == count {
            "<< /ExtGState << /GS << /Type /ExtGState /ca 0.5 >> >> >>"
        } else {
            "<< >>"
        };
        forms.push(painted_form(body, resources));
    }
    page_document(
        &format!("<< /XObject << {xobjects} >> >>"),
        &content,
        forms,
        target,
    )
}

/// `MAX_STREAMS` counts opened nested streams. This pair is much smaller than the token budget and
/// isolates that cap from nesting: every Form is a sibling at depth one.
fn stream_case(target: Target) {
    let within_limit = sibling_forms(4096, target);
    let survey = pdf_archive::survey::Survey::of(&within_limit);
    assert!(!survey.stopped_short());
    assert!(survey.page_is_transparent(0));
    assert!(matches!(
        row_outcome(&within_limit, target),
        Outcome::Failed { .. }
    ));

    let beyond_limit = sibling_forms(4097, target);
    let survey = pdf_archive::survey::Survey::of(&beyond_limit);

    assert!(
        !matches!(row_outcome(&beyond_limit, target), Outcome::Met),
        "a stream-count cutoff must remain visible: stopped={}, observed_transparency={}",
        survey.stopped_short(),
        survey.page_is_transparent(0)
    );
}

#[test]
fn pdf_a2_stopped_depth_is_not_certified_met() {
    depth_case(Target::Two(Level::B));
}
#[test]
fn pdf_a4_stopped_depth_is_not_certified_met() {
    depth_case(Target::Four(Flavour::Plain));
}
#[test]
fn pdf_a2_stopped_stream_count_is_not_certified_met() {
    stream_case(Target::Two(Level::B));
}
#[test]
fn pdf_a4_stopped_stream_count_is_not_certified_met() {
    stream_case(Target::Four(Flavour::Plain));
}
