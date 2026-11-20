//! A fault of a content stream's own bytes is reported as the stream's, by name.
//!
//! The archive's photograph (`pdf-transform`'s `archive/fonts/photographed.rs`, ADRs 2516 and 2618)
//! bakes in what the window drew of a page whose stream is malformed, and keeps refused a page
//! the window left short of what a well-formed document asked for. That line is drawn on these
//! reports, so each one below is asserted as the kind the clause makes it: a fault the file wrote,
//! never "an image or a shading this reader could not build". The pages that came to this file are
//! govdocs pages whose `/Contents` fail their own Flate check value and so decode to bytes no
//! producer wrote (ADR 2759).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the one length computed \
              here is a fixed string's"
)]

use std::fmt::Write as _;

use pdf_model::Unsupported;
use pdf_model::content::StreamSyntaxFault;
use pdf_syntax::Document;

/// A one-page document whose page states `resources` and draws `content`.
fn fixture(content: &str, resources: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// What interpreting the content stream reported.
fn reports(content: &str, resources: &str) -> Vec<Unsupported> {
    let document = Document::open(fixture(content, resources)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page).unsupported
}

/// ISO 32000-2 Table 73: `sc`'s operand count "depends on the current … colour space", three for
/// `DeviceRGB`. Two operands match no device family either, so the colour in force stays and the
/// page says the operands were the stream's fault.
#[test]
fn a_colour_given_operands_its_space_does_not_take_is_the_stream_s_fault() {
    assert_eq!(
        reports("/DeviceRGB cs 0.5 0.5 sc 0 0 10 10 re f", ""),
        vec![Unsupported::ColourOperands {
            stroking: false,
            given: 2,
            takes: 3,
        }]
    );
    assert_eq!(
        reports("/DeviceRGB CS 1 1 1 1 1 SC", ""),
        vec![Unsupported::ColourOperands {
            stroking: true,
            given: 5,
            takes: 3,
        }]
    );
}

/// §7.8.3: "A content stream's named resources shall be defined by a resource dictionary". `sh`
/// names its shading as `Do` names its form, and a name the dictionary does not define is the
/// same fault.
#[test]
fn a_shading_name_the_resources_do_not_define_is_a_missing_resource() {
    assert_eq!(
        reports("/Sh0 sh", ""),
        vec![Unsupported::MissingResource {
            category: "Shading",
            detail: String::from("/Sh0 is not in /Shading"),
        }]
    );
}

/// §8.9.7: "BI and ID shall bracket a series of key-value pairs". A number where a key belongs is
/// the stream's syntax; the image is not drawn and nothing else is lost.
#[test]
fn an_inline_image_whose_pairs_are_not_pairs_is_malformed_not_unbuilt() {
    let found = reports("BI /W 1 /H 1 /BPC 8 /CS /G 7 ID \x00 EI 0 0 10 10 re f", "");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        matches!(&found[0], Unsupported::MalformedInlineImage { detail }
            if detail == "Integer(7) where a key was expected"),
        "{found:?}"
    );
    // Table 91 marks `/W` required; the pairs that would state it are the stream's.
    assert_eq!(
        reports("BI /H 1 /BPC 8 /CS /G ID \x00 EI", ""),
        vec![Unsupported::MalformedInlineImage {
            detail: String::from("no /W, which Table 91 requires"),
        }]
    );
}

/// The three sentences an `Operator` report carries for a stream's own syntax, each named by
/// [`Unsupported::stream_syntax_fault`]: §8.9.7 admits `EI` "only … between a BI and its
/// balancing EI" (of `ID`, and `EI` closes what `BI` opens); Table 86 gives `Do` one operand, a
/// name; §8.5.4 has `W` modify "the succeeding painting operator", and the stream ended first.
#[test]
fn a_stray_image_operator_a_nameless_do_and_a_pending_clip_are_faults_of_the_stream() {
    let faults = |content: &str| -> Vec<Option<StreamSyntaxFault>> {
        reports(content, "")
            .iter()
            .map(Unsupported::stream_syntax_fault)
            .collect()
    };
    assert_eq!(
        faults("EI"),
        vec![Some(StreamSyntaxFault::ImageOperatorOutsideAnImage)]
    );
    assert_eq!(
        faults("1 Do"),
        vec![Some(StreamSyntaxFault::DoWithoutAName)]
    );
    assert_eq!(
        faults("0 0 10 10 re W"),
        vec![Some(StreamSyntaxFault::ClipLeftPending)]
    );
    // An operator nobody defines is not one of them: the fence decides that one.
    assert_eq!(faults("Frobnicate"), vec![None]);
}

/// §8.4.3.6: "The dash array's elements shall be numbers"; Table 57 gives `/D` the same pattern.
/// `[3 (x) 1]` is no dash array, so neither route sets a pattern and each says so (ADR 3335); a
/// pattern of numbers is said nothing of.
#[test]
fn a_dash_array_of_something_other_than_numbers_is_a_fault_of_the_stream() {
    let faults = |content: &str, resources: &str| -> Vec<Option<StreamSyntaxFault>> {
        reports(content, resources)
            .iter()
            .map(Unsupported::stream_syntax_fault)
            .collect()
    };
    assert_eq!(
        faults("[3 (x) 1] 0 d 0 0 m 10 0 l S", ""),
        vec![Some(StreamSyntaxFault::DashNotNumbers)]
    );
    assert_eq!(
        faults("[3 1] /x d 0 0 m 10 0 l S", ""),
        vec![Some(StreamSyntaxFault::DashNotNumbers)]
    );
    assert_eq!(
        faults(
            "/G gs 0 0 m 10 0 l S",
            "/ExtGState << /G << /D [[3 (x) 1] 0] >> >>"
        ),
        vec![Some(StreamSyntaxFault::DashNotNumbers)]
    );
    assert_eq!(faults("[3 1] 0 d 0 0 m 10 0 l S", ""), vec![]);
}

/// §7.3.2 makes `true` and `false` boolean objects and §7.3.9 `null` the null object, and §7.8.2
/// makes a content stream's objects operands, so none of the three is an operator nobody lists.
/// Read from the back as §7.8.2's "immediately precede" has it, `re` still finds its four numbers.
#[test]
fn a_boolean_or_null_is_an_operand_not_an_operator() {
    assert_eq!(reports("true null false 0 0 10 10 re f", ""), vec![]);
}

/// A real past f32's range (Annex C's Table C.1 bounds a conforming one inside it) is no position
/// on a page, so the `l` it stands in is declined and the rest of the path is drawn; an infinity in
/// the display list would have the backend refuse the whole page (`150277`'s page 57, ADR 2759).
#[test]
fn a_coordinate_past_the_range_of_a_real_is_declined_and_the_page_still_draws() {
    let huge = format!("1{}", "0".repeat(40));
    let content = format!("0 0 m 10 {huge} l 10 10 l 0 10 l f");
    let document = Document::open(fixture(&content, "")).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let text = format!("{:?}", list.commands());
    assert!(text.contains("Fill"), "{text}");
    assert!(!text.contains("inf"), "{text}");
}

/// A dash pattern past `pdf_render::MAX_DASHES` is drawn as its coverage rather than cut (ADR
/// 3334), so the page names the bound it reached: §8.4.3.6's `[0.00001 0.00001]` along 80 units is
/// four million dashes, and `[1 1]` forty.
#[test]
fn a_dash_pattern_past_the_bound_names_it() {
    assert_eq!(
        reports("[0.00001 0.00001] 0 d 10 50 m 90 50 l S", ""),
        vec![Unsupported::LimitReached {
            limit: "MAX_DASHES"
        }]
    );
    assert_eq!(reports("[1 1] 0 d 10 50 m 90 50 l S", ""), vec![]);
}
