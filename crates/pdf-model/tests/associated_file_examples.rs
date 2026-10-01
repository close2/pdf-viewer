//! ISO 32000-2 §14.13.10's three examples, run as documents (habit 39).
//!
//! The clause is examples and nothing else; each one is an association some other subclause
//! states in the abstract — §14.13.3's catalog, §14.13.5's marked-content section and §14.13.7's
//! form `XObject` — written out with the objects a producer would write. So each is a fixture here,
//! at the clause's own object numbers, with the embedded streams' elided bodies replaced by a few
//! bytes and their `/Filter` and `/Length` written to match.
//!
//! EXAMPLE 2 is the one that found something. Its page maps `/NamedAF` to the array
//! `[12 0 R]` and its stream writes `/AF /NamedAF BDC`: the named resource *is* the array, which is
//! the plainest reading of §14.13.5's "[t]he named resource in the Property List … shall specify an
//! array of file specification dictionaries". This tree read only a dictionary holding the array
//! under `/AF` or `/MCAF`, so the clause's own example associated nothing (ADR 1461).
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and these files are a few \
              hundred bytes long"
)]

use std::fmt::Write as _;

use pdf_model::attachment::{self, Relationship};
use pdf_syntax::{Document, ObjectId};

/// Numbered objects wrapped in §7.5's header, cross-reference table and trailer.
///
/// The objects keep the clause's own numbers, so the table is written sparse: one subsection per
/// object, and the trailer's `/Size` one past the largest.
fn assemble(objects: &[(u32, String)]) -> Vec<u8> {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (number, body) in objects {
        offsets.push((*number, out.len()));
        let _ = write!(out, "{number} 0 obj\n{body}\nendobj\n");
    }
    let xref_at = out.len();
    out.push_str("xref\n0 1\n0000000000 65535 f \n");
    for (number, offset) in &offsets {
        let _ = write!(out, "{number} 1\n{offset:010} 00000 n \n");
    }
    let size = offsets
        .iter()
        .map(|(number, _)| number + 1)
        .max()
        .unwrap_or(1);
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 19 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// An embedded file stream of the clause's shape, holding `data` uncompressed.
fn embedded(subtype: &str, data: &str) -> String {
    format!(
        "<< /Length {} /Type /EmbeddedFile /Subtype {subtype} /Params << \
         /CheckSum <ad032d7a6ea930489df4bfd6acb585b9> /Size {} \
         /CreationDate (D:20010727133719) /ModDate (D:20010727133720) >> >>\n\
         stream\n{data}\nendstream",
        data.len(),
        data.len()
    )
}

/// The three examples in one file: the catalog of EXAMPLE 1 (object 19), the page and file of
/// EXAMPLE 2 (objects 10, 11, 12 and 5), and EXAMPLE 3's form `XObject`, renumbered 40 to 42
/// because the clause reuses 19 to 21 for it.
fn examples() -> Document {
    let content = "/AF /NamedAF BDC BT /F1 1 Tf 12 0 0 12 100 600 Tm (Hello) Tj ET EMC /Fx Do";
    let objects = vec![
        (
            19,
            "<< /Type /Catalog /Pages 6 0 R /AF [20 0 R] >>".to_owned(),
        ),
        (
            20,
            "<< /Type /Filespec /F (My Presentation.ppt) /UF (My Presentation.ppt) \
             /AFRelationship /Source /EF <</F 21 0 R>> >>"
                .to_owned(),
        ),
        (21, embedded("/application#2Fvnd.ms-powerpoint", "ppt")),
        (6, "<< /Type /Pages /Kids [10 0 R] /Count 1 >>".to_owned()),
        (
            10,
            "<< /Type /Page /Parent 6 0 R /MediaBox [0 0 612 792] /Resources << \
             /Properties <</NamedAF [12 0 R]>> \
             /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> \
             /XObject << /Fx 40 0 R >> >> /Contents 11 0 R >>"
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
            "<< /Type /Filespec /F (datatable.doc) /UF (datatable.doc) /AFRelationship /Data \
             /EF <</F 5 0 R>> >>"
                .to_owned(),
        ),
        (5, embedded("/application#2Fvnd.ms-word", "doc")),
        (
            40,
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /AF [41 0 R] /Length 0 >>\n\
             stream\n\nendstream"
                .to_owned(),
        ),
        (
            41,
            "<< /Type /Filespec /F (equation.mathml) /UF (equation.mathml) \
             /AFRelationship /Supplement /EF <</F 42 0 R>> >>"
                .to_owned(),
        ),
        (42, embedded("/application#2Fxhtml+xml", "<math/>")),
    ];
    Document::open(assemble(&objects)).expect("the examples make a valid file")
}

/// EXAMPLE 1: a file associated with the document as a whole, through the catalog's `/AF`.
#[test]
fn example_1s_presentation_is_associated_with_the_document() {
    let document = examples();
    let catalog = document.catalog().expect("a catalog");
    let files = attachment::associated(&document, &catalog);
    let [presentation] = files.as_slice() else {
        panic!("one file associated with the catalog, got {files:?}");
    };
    assert_eq!(presentation.relationship, Relationship::Source);
    assert_eq!(
        presentation.file_name.as_deref(),
        Some("My Presentation.ppt")
    );
    assert_eq!(
        presentation.media_type.as_deref(),
        Some("application/vnd.ms-powerpoint")
    );
    assert!(
        attachment::attachments(&document)
            .iter()
            .any(|listed| listed.file_name.as_deref() == Some("My Presentation.ppt")),
        "a document-level association is one a panel lists"
    );
}

/// EXAMPLE 2: a file associated with a marked-content section, through a named resource that is
/// the array of file specifications itself.
#[test]
fn example_2s_data_table_is_associated_with_the_section_it_encloses() {
    let document = examples();
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let drawn = pdf_model::interpret(&document, &page);
    let [(range, file)] = drawn.associated_files.as_slice() else {
        panic!(
            "one file associated with the section, got {:?}",
            drawn.associated_files
        );
    };
    assert_eq!(file.relationship, Relationship::Data);
    assert_eq!(file.file_name.as_deref(), Some("datatable.doc"));
    assert_eq!(file.media_type.as_deref(), Some("application/vnd.ms-word"));
    if drawn.glyphs > 0 {
        assert_eq!(
            drawn.text.get(range.clone()).map(str::trim),
            Some("Hello"),
            "the range is the section's own content: {:?}",
            drawn.text
        );
    }
}

/// EXAMPLE 3: a file associated with a form `XObject`, through the form dictionary's `/AF`.
#[test]
fn example_3s_equation_is_associated_with_its_form() {
    let document = examples();
    let form = document.get(ObjectId::new(40, 0));
    let form = form.as_stream().expect("object 40 is the form");
    let files = attachment::associated(&document, &form.dict);
    let [equation] = files.as_slice() else {
        panic!("one file associated with the form, got {files:?}");
    };
    assert_eq!(equation.relationship, Relationship::Supplement);
    assert_eq!(equation.file_name.as_deref(), Some("equation.mathml"));
}
