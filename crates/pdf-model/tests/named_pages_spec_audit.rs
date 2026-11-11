//! Draft only: narrow coverage of the public named-page conformance diagnostic.
#![expect(clippy::expect_used, reason = "complete fixture construction")]

use std::fmt::Write as _;

use pdf_model::{Pages, named_page::NamedPages};
use pdf_syntax::{Document, FileBytes};

fn complete_pdf(body: &str) -> Document {
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
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID \
         [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\n\
         startxref\n{xref_at}\n%%EOF\n"
    );
    let document = Document::open(FileBytes::from(out.into_bytes())).expect("valid PDF syntax");
    assert!(!document.was_recovered(), "exact xref");
    document
}

fn templates(names: &str, objects: &str) -> Document {
    complete_pdf(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Names << /Templates << /Names [{names}] >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> >>\nendobj\n\
         {objects}"
    ))
}

fn disagreement_names(document: &Document) -> Vec<String> {
    let pages = Pages::new(document);
    NamedPages::read(document)
        .disagreements(document, &pages)
        .into_iter()
        .map(|disagreement| disagreement.name)
        .collect()
}

/// Control pair: a correctly typed /Template is accepted, while an explicit /Page is reported.
/// This mirrors the neighboring invariant checks in ADR 0091 rather than asserting reader refusal.
#[test]
fn template_type_diagnostic_has_valid_and_wrong_type_controls() {
    let document = templates(
        "(good) 4 0 R (wrong) 5 0 R",
        "4 0 obj\n<< /Type /Template /MediaBox [0 0 200 100] /Resources << >> >>\nendobj\n\
         5 0 obj\n<< /Type /Page /MediaBox [0 0 200 100] /Resources << >> >>\nendobj\n",
    );
    assert_eq!(disagreement_names(&document), ["wrong"]);
}

/// §12.7.7 says an invisible named page shall have object type Template. An absent /Type is no
/// more /Template than the tested wrong /Type /Page, so the public diagnostic should name it.
/// This is a document-conformance diagnostic gap, not a normative reader-rejection requirement.
#[test]
fn a_template_without_type_is_reported_as_not_a_template() {
    let document = templates(
        "(missing) 4 0 R",
        "4 0 obj\n<< /MediaBox [0 0 200 100] /Resources << >> >>\nendobj\n",
    );
    assert_eq!(disagreement_names(&document), ["missing"]);
}

/// A non-name /Type likewise does not identify a Template object. Syntax remains complete; the
/// intentionally nonconforming named object is the condition under test.
#[test]
fn a_template_with_non_name_type_is_reported_as_not_a_template() {
    let document = templates(
        "(nonname) 4 0 R",
        "4 0 obj\n<< /Type 7 /MediaBox [0 0 200 100] /Resources << >> >>\nendobj\n",
    );
    assert_eq!(disagreement_names(&document), ["nonname"]);
}
