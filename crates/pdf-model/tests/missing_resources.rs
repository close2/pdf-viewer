//! A name a content stream uses that ISO 32000-2 §7.8.3's resource dictionary does not define.
//!
//! §7.8.3 makes the file responsible for this and states it twice:
//!
//! > A content stream's named resources shall be defined by a resource dictionary, which shall
//! > enumerate the named resources needed by the operators in the content stream and the names
//! > by which they can be referred to.
//!
//! and, of a form `XObject`, a pattern or an appearance stream, "a PDF writer shall include a
//! Resources entry in the stream's dictionary specifying the resource dictionary which contains
//! all the resources used by that content stream". So a `Do`, a `gs` or an `scn` whose operand
//! finds nothing is a *malformed file*, and what this reader owes it is trap 5's rule: draw what
//! can be drawn and say what could not.
//!
//! # Why these are synthetic
//!
//! Trap 8, and measured rather than assumed. The 974-document corpus reaches these paths on
//! **three** first pages between them — `issue6541.pdf` (a tiling pattern naming an `/XObject`
//! only the page defines), `issue8702.pdf` (a form `XObject` written inside an object stream,
//! which §7.5.7 forbids from holding a stream, so it is a dictionary with no content) and
//! `operator_list_cycle.pdf` (a `gs` in a form whose `/Resources` states only a `/Pattern`).
//! None of the three exercises the condition that decides whether the report is honest — a
//! `Do` inside optional content the configuration hides, where nothing was going to be drawn
//! and a report would cost a judged page for nothing (trap 11). That one has no witness in 974
//! files and is the reason this module exists.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// Assembles a one-page PDF.
///
/// Object numbering is fixed so that a fixture can refer to its own objects: 1 catalog,
/// 2 pages, 3 page, 4 contents, and 5 onwards whatever `extra` defines.
fn pdf(catalog_extra: &str, resources: &str, content: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog_extra} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{extra}",
        content.len().saturating_add(1)
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
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

/// What the interpreter reported about a fixture.
fn reports(catalog_extra: &str, resources: &str, content: &str, extra: &str) -> Vec<String> {
    let bytes = pdf(catalog_extra, resources, content, extra);
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
        .unsupported
        .iter()
        .map(|report| format!("{report:?}"))
        .collect()
}

/// A form `XObject` drawing a square, so that a fixture can tell "drawn" from "not there".
fn square_form(number: usize) -> String {
    let content = "0 g 20 20 60 60 re f";
    format!(
        "{number} 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\n\
         stream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
    )
}

/// Table 86: "The operand name shall appear as a key in the `XObject` subdictionary".
#[test]
fn a_do_naming_no_xobject_at_all_is_reported() {
    let reported = reports("", "", "/Fm0 Do", "");

    assert_eq!(
        reported,
        vec![
            r#"MissingResource { category: "XObject", detail: "/Fm0 is not in /XObject" }"#
                .to_owned()
        ],
        "a `Do` on a page with no /XObject subdictionary draws nothing and must say so"
    );
}

/// Table 86's second requirement: "The associated value shall be a stream".
///
/// `issue8702.pdf`'s shape, and the reason a file can be written this way at all is §7.5.7 —
/// an object stream holds no streams, so a producer that puts a form `XObject` in one writes a
/// dictionary with `/Subtype /Form`, a `/BBox` and a `/Matrix` and no content anywhere.
#[test]
fn a_do_whose_value_is_not_a_stream_is_reported() {
    let reported = reports(
        "",
        "/XObject << /Fm0 5 0 R >>",
        "/Fm0 Do",
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] >>\nendobj\n",
    );

    assert_eq!(
        reported,
        vec![
            r#"MissingResource { category: "XObject", detail: "/Fm0 is not a stream" }"#.to_owned()
        ],
        "a form XObject with no stream body carries no content to draw"
    );
}

/// A form that omits `/Resources` is looked up in the page's, and reports nothing.
///
/// §7.8.3's NOTE 3 since Errata Collection 3 (Issue #128), which replaced the `shall` this
/// tree used to quote: "PDF files written obeying earlier versions of PDF may have omitted the
/// Resources entry in form `XObject`s, Type 3 glyph descriptions or annotation appearance streams
/// used on a page. Those earlier versions state that resources that were referenced from those
/// content streams can be inherited from the resource dictionary of the page on which they are
/// used."
#[test]
fn a_form_that_states_no_resources_uses_the_pages() {
    let outer = "q /Fm1 Do Q";
    let reported = reports(
        "",
        "/XObject << /Fm0 5 0 R /Fm1 6 0 R >>",
        "/Fm0 Do",
        &format!(
            "{}6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\n\
             stream\n{outer}\nendstream\nendobj\n",
            square_form(5),
            outer.len().saturating_add(1)
        ),
    );

    assert_eq!(
        reported,
        Vec::<String>::new(),
        "the inner form names /Fm0, which only the page defines, and states no /Resources"
    );
}

/// A form that *states* `/Resources` has said which names it uses, and a name it omits is
/// reported rather than looked up in the page's dictionary a second time.
///
/// `issue6541.pdf`'s shape one construction over. The standard defines nothing for this case —
/// §7.8.3's inheritance is stated for an *omitted* entry — so falling back would be a choice,
/// and it is the choice session 127 had to undo for fonts: a page's `/Fm0` and a form's `/Fm0`
/// are two objects as often as they are one.
#[test]
fn a_form_with_its_own_resources_does_not_reach_past_them() {
    let inner = "q /Fm0 Do Q";
    let reported = reports(
        "",
        "/XObject << /Fm0 5 0 R /Fm1 6 0 R >>",
        "/Fm1 Do",
        &format!(
            "{}6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Resources << /Font << >> >> /Length {} >>\nstream\n{inner}\nendstream\nendobj\n",
            square_form(5),
            inner.len().saturating_add(1)
        ),
    );

    assert_eq!(
        reported,
        vec![
            r#"MissingResource { category: "XObject", detail: "/Fm0 is not in /XObject" }"#
                .to_owned()
        ],
        "the form's own /Resources states no /XObject, so /Fm0 resolves to nothing there"
    );
}

/// **The condition, and the whole reason a report can be worse than a silence.**
///
/// §8.11.3.1: content in an optional content group the configuration turns off "shall be
/// skipped, as if there were no `Do` operator to invoke it" — so a `Do` there was never going
/// to mark the page, and a report would take a page off the oracle's judged set to describe a
/// difference no raster can hold (trap 11).
#[test]
fn a_do_inside_a_hidden_layer_names_nothing() {
    let reported = reports(
        "/OCProperties << /OCGs [5 0 R] /D << /OFF [5 0 R] >> >>",
        "/Properties << /oc 5 0 R >>",
        "/OC /oc BDC /Fm0 Do EMC",
        "5 0 obj\n<< /Type /OCG /Name (Layer) >>\nendobj\n",
    );

    assert_eq!(
        reported,
        Vec::<String>::new(),
        "nothing inside a hidden section marks the page, so nothing there can be lost"
    );
}

/// The same fixture with the layer on, which is what makes the test above about the layer.
#[test]
fn the_same_do_outside_the_hidden_layer_is_reported() {
    let reported = reports(
        "/OCProperties << /OCGs [5 0 R] /D << >> >>",
        "/Properties << /oc 5 0 R >>",
        "/OC /oc BDC /Fm0 Do EMC",
        "5 0 obj\n<< /Type /OCG /Name (Layer) >>\nendobj\n",
    );

    assert_eq!(
        reported,
        vec![
            r#"MissingResource { category: "XObject", detail: "/Fm0 is not in /XObject" }"#
                .to_owned()
        ],
        "with the group on the section draws, so the undefined name costs a mark"
    );
}

/// Table 56's `gs`, whose miss leaves every parameter it would have set as it was.
#[test]
fn a_gs_naming_no_ext_gstate_is_reported() {
    let reported = reports("", "", "/GS0 gs 0 g 20 20 60 60 re f", "");

    assert_eq!(
        reported,
        vec![
            r#"MissingResource { category: "ExtGState", detail: "/GS0 is not in /ExtGState" }"#
                .to_owned()
        ],
        "a graphics state the file names and does not define is a wrong state, not a lost mark"
    );
}

/// §8.7.3.2's `scn`, whose miss leaves §8.6.8's "pattern object that causes nothing to be
/// painted" in force for every fill that follows.
#[test]
fn an_scn_naming_no_pattern_is_reported() {
    let reported = reports("", "", "/Pattern cs /P0 scn 20 20 60 60 re f", "");

    assert_eq!(
        reported,
        vec![
            r#"MissingResource { category: "Pattern", detail: "/P0 is not in /Pattern" }"#
                .to_owned()
        ],
        "the fill that follows paints nothing, which looks exactly like a producer's intent"
    );
}

/// §7.3.5's binary match, over a name that is not text.
///
/// > Beginning with PDF 1.2 a name object is an atomic symbol uniquely defined by a sequence of
/// > any characters (8-bit values) except null (character code 0). Uniquely defined means that
/// > any two name objects that, after all escaping is expanded (see below), and the resulting
/// > sequences of bytes are not an exact binary match denote different objects.
///
/// `/Contr#F4le` decodes to a name whose sixth byte is 0xF4, which is a byte no UTF-8 sequence
/// begins. The interpreter used to carry a resource name as a `String` built with
/// `from_utf8_lossy`, so the probe was `Contr\u{FFFD}le` and the key was the file's bytes: the
/// `Do` found nothing and the page came back blank with a report saying the file was malformed.
/// A crawled document does exactly this — a scanner naming its `XObject` after a Windows path
/// with an "ô" in it — and three reference renderers draw its full-page scan (ADR 0438).
///
/// Synthetic for trap 8's reason and for the promotion rule both: the witness is a crawled
/// document out of the `SafeDocs` set, which this repository records by digest and never commits.
#[test]
fn a_resource_name_that_is_not_utf_8_is_found() {
    let reported = reports(
        "",
        "/XObject << /Contr#F4le 5 0 R >>",
        "/Contr#F4le Do",
        &square_form(5),
    );

    assert_eq!(
        reported,
        Vec::<String>::new(),
        "the name the file defines and the name the stream uses are an exact binary match"
    );
}

/// The other half of the same sentence: two names differing only outside UTF-8 are two names.
///
/// This is the direction that draws the *wrong* thing rather than nothing. `from_utf8_lossy`
/// maps every invalid byte to one replacement character, so `/A#F4` and `/A#F5` both became
/// `A\u{FFFD}` and a `Do` on either found whichever the dictionary defined — a mark on the page
/// from an object the content stream did not name, in silence.
#[test]
fn two_names_differing_only_in_a_byte_outside_utf_8_are_two_names() {
    let reported = reports(
        "",
        "/XObject << /A#F4 5 0 R >>",
        "/A#F5 Do",
        &square_form(5),
    );

    assert_eq!(
        reported,
        vec![format!(
            r#"MissingResource {{ category: "XObject", detail: "/A{} is not in /XObject" }}"#,
            char::REPLACEMENT_CHARACTER
        )],
        "0xF4 and 0xF5 are different names however a text conversion renders them"
    );
}
