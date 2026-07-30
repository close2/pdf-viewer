//! ISO 32000-2 §7.8.2's compatibility section, `BX` … `EX`.
//!
//! The clause states the ordinary rule and then the exception to it:
//!
//! > Ordinarily, when a PDF reader encounters an operator in a content stream that it does
//! > not recognise, an error shall occur.
//!
//! and inside a compatibility section, "unrecognised operators (along with their operands)
//! shall be ignored without error". That makes `BX` … `EX` the **one** place in this
//! interpreter where unsupported input is deliberately silent, which is a departure from
//! principle 3's rule that everything unhandled must be loud — so it is worth a test of its
//! own rather than a line in a match arm. The file has said in advance that ignoring the
//! operator is the appropriate thing to do; nowhere else does.
//!
//! # Why this is synthetic
//!
//! Trap 8. Nine corpus documents report an unrecognised operator and not one of them wraps
//! it in `BX` … `EX` — they are `toString`, `undefined`, `inf`, and the byte soup a fuzzed
//! stream lexes as operator names, none of which any producer meant. A corpus cannot
//! exercise a compatibility mechanism, because the mechanism exists for operators newer than
//! the reader and the corpus is older than this code.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// A one-page PDF whose content stream is `content`.
fn page_with(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
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

/// What the interpreter reported about a content stream.
fn reports(content: &str) -> Vec<String> {
    let document = Document::open(page_with(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
        .unsupported
        .iter()
        .map(|report| format!("{report:?}"))
        .collect()
}

/// Outside a compatibility section, an operator nobody knows is named.
#[test]
fn an_unrecognised_operator_is_reported() {
    let reported = reports("0 g 1 2 3 zz 0 0 10 10 re f");

    assert!(
        reported.iter().any(|report| report.contains("zz")),
        "expected the operator to be named, got {reported:?}"
    );
}

/// Inside one, it is ignored without error — and the drawing around it still happens.
#[test]
fn a_compatibility_section_silences_an_unrecognised_operator() {
    let reported = reports("0 g BX 1 2 3 zz EX 0 0 10 10 re f");

    assert_eq!(
        reported,
        Vec::<String>::new(),
        "§7.8.2 says an unrecognised operator inside BX … EX is ignored without error"
    );
}

/// The section ends where `EX` ends it, and not at the end of the stream.
#[test]
fn the_silence_stops_at_the_matching_ex() {
    let reported = reports("BX zz EX yy");

    assert_eq!(reported.len(), 1, "one of the two is outside: {reported:?}");
    assert!(
        reported[0].contains("yy") && !reported[0].contains("zz"),
        "the wrong one was reported: {reported:?}"
    );
}

/// "These operators shall occur in pairs and **may be nested**."
///
/// So an inner `EX` must not reopen reporting while the outer section is still running,
/// which a boolean flag would get wrong and a depth counter gets right.
#[test]
fn compatibility_sections_nest() {
    let reported = reports("BX BX zz EX yy EX xx");

    assert_eq!(reported.len(), 1, "only `xx` is outside: {reported:?}");
    assert!(
        reported[0].contains("xx"),
        "the wrong one was reported: {reported:?}"
    );
}
