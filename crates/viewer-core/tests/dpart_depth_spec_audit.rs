//! A `DPart` enumeration ceiling must not silently make a valid attachment disappear.
//!
//! This is a truthful-result product regression, not a demand for unlimited ISO support.
//! A partial-result report is accepted instead of requiring traversal past the resource ceiling.
//! Source: EC3 Tables408–409 physical851/printed836, #612; section14.13.8 physical855/840.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    reason = "small synthetic PDF object numbers and exact offsets"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

fn dpart_pdf(depth: usize) -> Vec<u8> {
    // Objects 1..4 are Catalog, Pages, Page, and DPartRoot. The root DPart is object 5.
    let root_part = 5usize;
    let node_count = depth + 1;
    let filespec = root_part + node_count;
    let embedded_stream = filespec + 1;
    let leaf = root_part + depth;
    let catalog_file = embedded_stream + 1;
    let catalog_stream = catalog_file + 1;
    let mut objects = Vec::<String>::with_capacity(6 + node_count);
    objects.push(format!(
        "<< /Type /Catalog /Pages 2 0 R /DPartRoot 4 0 R /AF [{catalog_file} 0 R] >>"
    ));
    objects.push("<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned());
    objects.push(format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /DPart {leaf} 0 R >>"
    ));
    objects.push("<< /Type /DPartRoot /DPartRootNode 5 0 R >>".to_owned());

    for level in 0..=depth {
        let object_id = root_part + level;
        let parent = if level == 0 { 4 } else { object_id - 1 };
        if level < depth {
            let child = object_id + 1;
            // Effective EC3 #612 shape: an array of indirect refs to immediate descendants.
            objects.push(format!(
                "<< /Type /DPart /Parent {parent} 0 R /DParts [{child} 0 R] >>"
            ));
        } else {
            objects.push(format!(
                "<< /Type /DPart /Parent {parent} 0 R /Start 3 0 R /AF [{filespec} 0 R] >>"
            ));
        }
    }
    objects.push(format!(
        "<< /Type /Filespec /F (deep-part.txt) /UF (deep-part.txt) /AFRelationship /Data /EF << /F {embedded_stream} 0 R /UF {embedded_stream} 0 R >> >>"
    ));
    objects.push(
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Params << /ModDate (D:20261116000000Z) /Size 4 >> /Length 4 >>\nstream\ndeep\nendstream"
            .to_owned(),
    );

    objects.push(format!("<< /Type /Filespec /F (catalog-control.txt) /UF (catalog-control.txt) /AFRelationship /Data /EF << /F {catalog_stream} 0 R /UF {catalog_stream} 0 R >> >>"));
    objects.push("<< /Type /EmbeddedFile /Subtype /text#2Fplain /Params << /ModDate (D:20261116000000Z) /Size 7 >> /Length 7 >>\nstream\ncontrol\nendstream".to_owned());
    let mut pdf = String::from("%PDF-2.0\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = writeln!(pdf, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref_at = pdf.len();
    let size = objects.len() + 1;
    let _ = writeln!(pdf, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899aabbccddeeff> <ffeeddccbbaa99887766554433221100>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    pdf.into_bytes()
}

fn observed(depth: usize) -> (Vec<String>, Vec<String>) {
    let bytes = dpart_pdf(depth);
    let syntax = Document::open(bytes.clone()).expect("exact-xref PDF opens");
    assert!(!syntax.was_recovered(), "fixture offsets are exact");
    let mut viewer = Viewer::new(300, 300, 1.0);
    let opening: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opening
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 1, .. }))
    );
    let names: Vec<String> = match viewer.query(Query::Attachments) {
        Answer::Attachments(files) => files.into_iter().map(|file| file.name).collect(),
        other => panic!("attachments query is answered: {other:?}"),
    };
    assert!(
        names.iter().any(|name| name == "catalog-control.txt"),
        "catalog attachment remains visible"
    );
    let notes = viewer
        .handle(Command::Report)
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect();
    (names, notes)
}

#[test]
fn dpart_af_on_a_leaf_at_the_depth_limit_is_listed() {
    let (names, _) = observed(64);
    assert!(names.iter().any(|name| name == "deep-part.txt"));
}

#[test]
fn dpart_af_beyond_the_limit_is_listed_or_disclosed_as_incomplete() {
    let (names, notes) = observed(65);
    let listed = names.iter().any(|name| name == "deep-part.txt");
    let disclosed = notes.iter().any(|note| {
        let lower = note.to_ascii_lowercase();
        (lower.contains("dpart") || lower.contains("document part"))
            && ["limit", "truncat", "incomplete", "partial"]
                .iter()
                .any(|word| lower.contains(word))
    });
    assert!(
        listed || disclosed,
        "valid deep attachment is silently absent: names={names:?}, notes={notes:?}"
    );
}
