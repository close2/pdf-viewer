//! Strengthened ISO 32000-2 §14.13.10 example checks with EC3 corrections.
//!
//! This keeps the examples' relevant association shapes, but uses actual replacement payload bytes,
//! matching checksums, an embedded deterministic Type 3 font, and corrected `MathML` MIME from Issue #637.
//! The text-range assertion is unconditional so an empty interpretation fails.

#![expect(
    clippy::expect_used,
    reason = "small fixture offsets; malformed fixtures and missing example behavior must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::attachment::{self, Relationship};
use pdf_syntax::{Document, ObjectId};

fn assemble(objects: &[(u32, String)]) -> Vec<u8> {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (number, body) in objects {
        offsets.push((*number, out.len()));
        let _ = write!(out, "{number} 0 obj\n{body}\nendobj\n");
    }
    let xref_at = out.len();
    let size = objects
        .iter()
        .map(|(number, _)| number.saturating_add(1))
        .max()
        .unwrap_or(1);
    let free: Vec<u32> = (1..size)
        .filter(|number| !offsets.iter().any(|(used, _)| used == number))
        .collect();
    writeln!(out, "xref\n0 {size}").expect("String write");
    writeln!(out, "{:010} 65535 f ", free.first().copied().unwrap_or(0)).expect("String write");
    for number in 1..size {
        if let Some((_, offset)) = offsets.iter().find(|(used, _)| *used == number) {
            writeln!(out, "{offset:010} 00000 n ").expect("String write");
        } else {
            let next = free
                .iter()
                .copied()
                .find(|unused| *unused > number)
                .unwrap_or(0);
            writeln!(out, "{next:010} 00000 f ").expect("String write");
        }
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 19 0 R /ID [<0123456789abcdef0123456789abcdef> <fedcba9876543210fedcba9876543210>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn embedded(subtype: &str, data: &str, checksum: &str) -> String {
    format!(
        "<< /Length {} /Type /EmbeddedFile /Subtype {subtype} /Params << \
         /CheckSum <{checksum}> /Size {} /CreationDate (D:20010727133719) \
         /ModDate (D:20010727133720) >> >>\nstream\n{data}\nendstream",
        data.len(),
        data.len()
    )
}

fn examples() -> Document {
    let content = "BT /F1 12 Tf 10 700 Td (H) Tj ET /AF /NamedAF BDC BT /F1 12 Tf 100 600 Td (Hello) Tj ET EMC BT /F1 12 Tf 10 500 Td (o) Tj ET";
    let glyph = "500 0 0 0 500 700 d1 0 0 250 500 re f";
    let objects = vec![
        (
            19,
            "<< /Type /Catalog /Pages 6 0 R /AF [20 0 R] >>".to_owned(),
        ),
        (
            20,
            "<< /Type /Filespec /F (presentation.ppt) /UF (presentation.ppt) \
             /AFRelationship /Source /EF << /F 21 0 R >> >>"
                .to_owned(),
        ),
        (
            21,
            embedded(
                "/application#2Fvnd.ms-powerpoint",
                "ppt",
                "8a267223a8635f5e7348cd0d01c3bb3c",
            ),
        ),
        (6, "<< /Type /Pages /Kids [10 0 R] /Count 1 >>".to_owned()),
        (
            10,
            "<< /Type /Page /Parent 6 0 R /MediaBox [0 0 612 792] \
             /Resources << /Properties << /NamedAF << /MCAF [12 0 R] >> >> \
             /Font << /F1 50 0 R >> /XObject << /Fx 40 0 R >> >> \
             /Contents 11 0 R >>"
                .to_owned(),
        ),
        (
            11,
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
        ),
        (
            12,
            "<< /Type /Filespec /F (datatable.doc) /UF (datatable.doc) \
             /AFRelationship /Data /EF << /F 5 0 R >> >>"
                .to_owned(),
        ),
        (
            5,
            embedded(
                "/application#2Fvnd.ms-word",
                "doc",
                "9a09b4dfda82e3e665e31092d1c3ec8d",
            ),
        ),
        (
            50,
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] \
             /FontMatrix [0.001 0 0 0.001 0 0] \
             /CharProcs << /H 51 0 R /e 51 0 R /l 51 0 R /o 51 0 R >> \
             /Encoding << /Type /Encoding /Differences [72 /H 101 /e 108 /l 111 /o] >> \
             /FirstChar 72 /LastChar 111 \
             /Widths [500 500 500 500 500 500 500 500 500 500 \
                      500 500 500 500 500 500 500 500 500 500 \
                      500 500 500 500 500 500 500 500 500 500 \
                      500 500 500 500 500 500 500 500 500 500] \
             /Resources << >> >>"
                .to_owned(),
        ),
        (
            51,
            format!("<< /Length {} >>\nstream\n{glyph}\nendstream", glyph.len()),
        ),
        (
            40,
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Resources << >> /AF [41 0 R] \
             /Length 0 >>\nstream\n\nendstream"
                .to_owned(),
        ),
        (
            41,
            "<< /Type /Filespec /F (equation.mathml) /UF (equation.mathml) \
             /AFRelationship /Supplement /EF << /F 42 0 R >> >>"
                .to_owned(),
        ),
        (
            42,
            embedded(
                "/application#2Fmathml+xml",
                "<math/>",
                "1e1ee6904e7dd1e843c2c9bbbfaabe80",
            ),
        ),
    ];
    let document = Document::open(assemble(&objects)).expect("complete PDF 2.0 example fixture");
    assert!(!document.was_recovered());
    document
}

#[test]
fn marked_content_example_interprets_nonempty_text_inside_the_associated_range() {
    let document = examples();
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let drawn = pdf_model::interpret(&document, &page);
    assert_eq!(
        drawn.glyphs, 7,
        "Hello inside the section, plus one glyph before and after"
    );
    let [(range, file)] = drawn.associated_files.as_slice() else {
        panic!(
            "one associated range and file, got {:?}",
            drawn.associated_files
        );
    };
    assert_eq!(file.relationship, Relationship::Data);
    assert_eq!(file.file_name.as_deref(), Some("datatable.doc"));
    assert_eq!(file.media_type.as_deref(), Some("application/vnd.ms-word"));
    assert_eq!(
        drawn.text.get(range.clone()).map(str::trim),
        Some("Hello"),
        "the associated range contains the example's emitted text: {:?}",
        drawn.text
    );
    assert_eq!(file.checksum_matches(b"doc"), Some(true));
}

#[test]
fn corrected_mathml_example_uses_the_erratum_media_type() {
    let document = examples();
    let form = document.get(ObjectId::new(40, 0));
    let form = form.as_stream().expect("form XObject");
    let files = attachment::associated(&document, &form.dict);
    let [equation] = files.as_slice() else {
        panic!("the form carries its equation association");
    };
    assert_eq!(equation.relationship, Relationship::Supplement);
    assert_eq!(equation.file_name.as_deref(), Some("equation.mathml"));
    assert_eq!(
        equation.media_type.as_deref(),
        Some("application/mathml+xml"),
        "EC3 Issue #637 corrects XHTML to MathML"
    );
    assert_eq!(equation.checksum_matches(b"<math/>"), Some(true));
}
