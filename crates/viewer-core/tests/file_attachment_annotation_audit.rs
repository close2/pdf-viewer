//! Exact file selection through public attachment activation with competing embedded files.
//! Stored appearances keep payload routing independent of fallback icon construction.

#![expect(
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "the fixture is deliberately exact and a missing observable must fail loudly"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

fn file_attachment_pdf() -> Vec<u8> {
    fn stream_object(number: usize, data: &[u8]) -> String {
        format!(
            "{number} 0 obj\n<< /Type /EmbeddedFile /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            data.len(),
            String::from_utf8_lossy(data)
        )
    }

    let payload_a = b"payload A";
    let payload_b = b"payload B";
    let payload_decoy = b"not the clicked annotation";
    let content = b"";
    let body = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 13 0 R >> >>\nendobj\n".to_owned(),
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_owned(),
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R /Annots [5 0 R 6 0 R] >>\nendobj\n".to_owned(),
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            content.len(),
            String::from_utf8_lossy(content)
        ),
        "5 0 obj\n<< /Type /Annot /Subtype /FileAttachment /Rect [20 20 40 40] /F 4 /Name /Paperclip /FS 7 0 R /AP << /N 14 0 R >> >>\nendobj\n".to_owned(),
        "6 0 obj\n<< /Type /Annot /Subtype /FileAttachment /Rect [60 20 80 40] /F 4 /Name /Graph /FS 9 0 R /AP << /N 14 0 R >> >>\nendobj\n".to_owned(),
        "7 0 obj\n<< /Type /Filespec /F (first.txt) /UF (first.txt) /EF << /F 8 0 R >> >>\nendobj\n".to_owned(),
        stream_object(8, payload_a),
        "9 0 obj\n<< /Type /Filespec /F (second.txt) /UF (second.txt) /EF << /F 10 0 R >> >>\nendobj\n".to_owned(),
        stream_object(10, payload_b),
        "11 0 obj\n<< /Type /Filespec /F (decoy.txt) /UF (decoy.txt) /EF << /F 12 0 R >> >>\nendobj\n".to_owned(),
        stream_object(12, payload_decoy),
        "13 0 obj\n<< /Names [(decoy.txt) 11 0 R] >>\nendobj\n".to_owned(),
        { let paint="1 0 0 rg 0 0 20 20 re f";
          format!("14 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >> /Length {} >>\nstream\n{paint}\nendstream\nendobj\n",paint.len()+1)
        },
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(body.len());
    for object in &body {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn device_point(viewer: &Viewer, rect: [f32; 4]) -> (f32, f32) {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the opened page has geometry");
    };
    let x = f32::midpoint(rect[0], rect[2]);
    let y = f32::midpoint(rect[1], rect[3]);
    (
        geometry.origin.0 + x * geometry.scale,
        geometry.origin.1 + (200.0 - y) * geometry.scale,
    )
}

fn extracted_at(viewer: &mut Viewer, rect: [f32; 4]) -> Vec<(String, Vec<u8>)> {
    let at = device_point(viewer, rect);
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Released,
        })
        .filter_map(|event| match event {
            Event::Extracted { name, bytes, .. } => Some((name, bytes)),
            _ => None,
        })
        .collect()
}

#[test]
fn clicking_attachment_activates_its_own_filespec_not_a_name_tree_file() {
    let bytes = file_attachment_pdf();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF fixture");
    assert!(!parsed.was_recovered(), "the xref offsets are exact");
    let page = pdf_model::Pages::new(&parsed).get(0).expect("one page");
    let interpreted = pdf_model::interpret(&parsed, &page);
    assert!(
        interpreted.is_complete(),
        "both stored appearances are supported"
    );
    assert!(
        !interpreted.display_list.commands().is_empty(),
        "visible attachment controls"
    );

    let mut viewer = Viewer::new(500, 500, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect::<Vec<_>>();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );
    assert!(
        !opened
            .iter()
            .any(|event| matches!(event, Event::Extracted { .. })),
        "opening the document must not extract any of its three files"
    );

    // Negative control first: an unrelated point must not enumerate the EmbeddedFiles tree.
    assert!(extracted_at(&mut viewer, [120.0, 120.0, 140.0, 140.0]).is_empty());

    // Each annotation names a different FileSpec; a separate third payload is present in the
    // catalog's EmbeddedFiles tree. Exact names and bytes reject a wrong candidate or a mere
    // assertion that some bytes were extracted.
    assert_eq!(
        extracted_at(&mut viewer, [20.0, 20.0, 40.0, 40.0]),
        vec![("first.txt".to_owned(), b"payload A".to_vec())]
    );
    assert_eq!(
        extracted_at(&mut viewer, [60.0, 20.0, 80.0, 40.0]),
        vec![("second.txt".to_owned(), b"payload B".to_vec())]
    );
}

#[test]
fn a_present_empty_attachment_contents_does_not_reveal_the_filespec_description() {
    // §12.5.6.15 puts annotation Contents ahead of Filespec Desc without a nonempty
    // condition. Empty text is a valid text string; omission and present-empty are distinct.
    // This asserts the public Attachment reader seam, not a rendered desktop tooltip.
    let description = |contents: &str| {
        let bytes = file_attachment_pdf();
        let original = String::from_utf8(bytes).expect("ASCII fixture");
        // Rebuild offsets after changing object bodies, retaining all complete fixture objects.
        let body = original.split("xref\n").next().expect("object prefix");
        let body = body.strip_prefix("%PDF-1.7\n").expect("header");
        let body = body
            .replace(
                "/F (first.txt)",
                "/Desc (conflicting filespec description) /F (first.txt)",
            )
            .replace("/Name /Paperclip", &format!("{contents} /Name /Paperclip"));
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(out.len());
            out.push_str(object);
        }
        let xref = out.len();
        writeln!(out, "xref\n0 {}\n0000000000 65535 f ", offsets.len() + 1).expect("xref");
        for offset in &offsets {
            writeln!(out, "{offset:010} 00000 n ").expect("offset");
        }
        write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .expect("trailer");
        let doc = pdf_syntax::Document::open(out.into_bytes()).expect("complete fixture");
        assert!(!doc.was_recovered());
        let object = doc.get(pdf_syntax::ObjectId::new(5, 0));
        let annotation = object.as_dict().expect("annotation");
        pdf_model::attachment::of_annotation(&doc, annotation)
            .expect("associated file")
            .description
    };
    assert_eq!(
        description("").as_deref(),
        Some("conflicting filespec description"),
        "omitted-key fallback control"
    );
    assert_eq!(
        description("/Contents (annotation text)").as_deref(),
        Some("annotation text"),
        "present nonempty precedence control"
    );
    let empty = description("/Contents ()");
    assert!(
        empty.as_deref().is_none_or(str::is_empty),
        "present empty Contents must not expose Desc: {empty:?}"
    );
}
