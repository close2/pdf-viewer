//! Table 87's `/Width` and `/Height`, ISO 32000-2 §8.9.5.1, and what the refusal says when
//! one of them is not there to read. The table types both as `integer`, marks both
//! `(Required)`, and counts both "in samples".
//!
//! An image with no grid draws nothing, and that was always right. What this file pins is the
//! *sentence*: a refusal reading "missing or invalid /Width" was true of `issue4575.pdf` twice
//! over — its dictionary writes `/Width /Height` and no `/Height` at all — without saying either
//! thing, and a sentence that names one of two faults is one a reader has to go back to the
//! file to resolve. Each fault Table 87's two words can take now has its own sentence, and the
//! witness is read out of the file so the pair cannot pass for a new reason.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the one length computed \
              here is a fixed string's"
)]

use std::fmt::Write as _;
use std::path::Path;

use pdf_model::Unsupported;
use pdf_syntax::{Document, Name, Object, ObjectId};

/// A one-page fixture drawing `/I1`, an 8-bit `DeviceGray` image whose dictionary states
/// `entries` for its dimensions and one byte of sample data.
fn fixture(entries: &str) -> Vec<u8> {
    let content = "q 100 0 0 100 0 0 cm /I1 Do Q";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /I1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Image /ColorSpace /DeviceGray \
         /BitsPerComponent 8 {entries} /Length 1 >>\nstream\n\u{0}\nendstream\nendobj\n",
        content.len()
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = writeln!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
    );
    out.into_bytes()
}

/// What page one reports for an image whose dictionary states `entries`.
fn reports(entries: &str) -> Vec<Unsupported> {
    let document = Document::open(fixture(entries)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page).unsupported
}

/// The one report an image refusal is, with the fixture's resource name in front of it.
fn refused(detail: &str) -> Vec<Unsupported> {
    vec![Unsupported::Image {
        name: format!("I1: malformed image: {detail}"),
    }]
}

#[test]
fn a_grid_the_dictionary_states_is_drawn_without_a_report() {
    assert_eq!(reports("/Width 1 /Height 1"), vec![]);
}

#[test]
fn an_absent_dimension_is_named_as_absent() {
    assert_eq!(
        reports("/Height 1"),
        refused("no /Width, which Table 87 requires")
    );
    assert_eq!(
        reports("/Width 1"),
        refused("no /Height, which Table 87 requires")
    );
}

#[test]
fn a_dimension_of_another_type_is_named_with_its_type() {
    assert_eq!(
        reports("/Width /Height"),
        refused("/Width is the name /Height, where Table 87 requires an integer")
    );
    assert_eq!(
        reports("/Width [1] /Height 1"),
        refused("/Width is an array, where Table 87 requires an integer")
    );
    assert_eq!(
        reports("/Width 1 /Height (1)"),
        refused("/Height is a string, where Table 87 requires an integer")
    );
}

#[test]
fn a_number_that_counts_no_samples_is_named_with_its_value() {
    assert_eq!(
        reports("/Width 0 /Height 1"),
        refused("/Width is 0, which counts no samples")
    );
    assert_eq!(
        reports("/Width 1 /Height -2.5"),
        refused("/Height is -2.5, which counts no samples")
    );
}

/// `issue4575.pdf`'s `/I1` states `/Width /Height` and no `/Height`; the first is what the
/// refusal names, because `/Width` is read first and one sentence is the report. The
/// dictionary is read out of the file beside the report so that a document silently replaced
/// cannot leave this passing for a new reason.
#[test]
fn the_witness_writes_a_name_for_its_width_and_no_height() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/issue4575.pdf");
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let document = Document::open(bytes).expect("issue4575.pdf opens");
    let image = document.get(ObjectId {
        number: 4,
        generation: 0,
    });
    let dict = &image.as_stream().expect("object 4 is the image /I1").dict;
    assert_eq!(
        dict.get("Width")
            .and_then(Object::as_name)
            .map(Name::as_bytes),
        Some(&b"Height"[..]),
        "the file writes `/Width /Height`"
    );
    assert!(dict.get("Height").is_none(), "and no /Height at all");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("issue4575.pdf has a page");
    let reported = pdf_model::interpret(&document, &page).unsupported;
    assert_eq!(
        reported,
        refused("/Width is the name /Height, where Table 87 requires an integer"),
        "the sentence says what the entry is, and only that"
    );
}
