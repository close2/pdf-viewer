//! §12.4.2 page-label ranges reach viewer queries and page-panel entries.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "complete bounded fixtures must open and answer queries"
)]
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn opened(bytes: Vec<u8>) -> Viewer {
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF fixture");
    assert!(!parsed.was_recovered(), "exact xref offsets");
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(93),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

fn nine_labeled_pages(prefix_entry: &str) -> Vec<u8> {
    use std::fmt::Write as _;

    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R /PageLabels 12 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 9 /Kids [3 0 R 4 0 R 5 0 R 6 0 R 7 0 R 8 0 R 9 0 R 10 0 R 11 0 R] >>".to_vec(),
    ];
    for _ in 0..9 {
        objects.push(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
        );
    }
    objects.push(format!("<< /Nums [0 << /S /r >> 4 << {prefix_entry} >> 6 << /Type /PageLabel /S /D /P (A-) /St 8 >>] >>").into_bytes());

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = pdf.len();
    let size = objects.len() + 1;
    let mut xref = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    pdf.extend_from_slice(xref.as_bytes());
    pdf
}

#[test]
fn page_label_lookup_reaches_the_page_panel_row_and_keeps_the_index_fallback() {
    let viewer = opened(nine_labeled_pages("/P (Appendix)"));
    let labels: Vec<_> = (0..9)
        .map(|index| match viewer.query(Query::PageLabel(index)) {
            Answer::Label(label) => label,
            other => panic!("the file labels page {index}, got {other:?}"),
        })
        .collect();
    assert_eq!(
        labels,
        [
            "i", "ii", "iii", "iv", "Appendix", "Appendix", "A-8", "A-9", "A-10"
        ]
    );

    // The host's PageEntry is the actual panel-facing consumer; a missing label remains the
    // host's explicit fallback rather than a synthesized normative label.
    assert_eq!(viewer_host::page_entry(&viewer, 0).label, "i");
    assert_eq!(viewer_host::page_entry(&viewer, 4).label, "Appendix");
    assert_eq!(viewer_host::page_entry(&viewer, 8).label, "A-10");

    let unlabelled = opened(minimal_one_page_pdf_without_page_labels());
    assert!(matches!(
        unlabelled.query(Query::PageLabel(0)),
        Answer::None
    ));
    assert_eq!(viewer_host::page_entry(&unlabelled, 0).label, "Page 1");
}

fn minimal_one_page_pdf_without_page_labels() -> Vec<u8> {
    use std::fmt::Write as _;

    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".as_slice(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".as_slice(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".as_slice(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = pdf.len();
    let size = objects.len() + 1;
    let mut xref = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    pdf.extend_from_slice(xref.as_bytes());
    pdf
}

#[test]
fn an_explicit_empty_page_label_is_not_replaced_by_the_unlabelled_fallback() {
    for prefix_entry in ["", "/P ()"] {
        let bytes = nine_labeled_pages(prefix_entry);
        let document = pdf_syntax::Document::open(bytes.clone()).expect("valid fixture");
        let labels = pdf_model::page_label::PageLabels::read(&document);
        assert_eq!(labels.label(4).as_deref(), Some(""));
        assert_eq!(labels.label(5).as_deref(), Some(""));
        let viewer = opened(bytes);
        assert_eq!(
            viewer_host::page_entry(&viewer, 6).label,
            "A-8",
            "the next range remains distinct"
        );
        for index in [4, 5] {
            let answer = viewer.query(Query::PageLabel(index));
            assert!(
                matches!(&answer, Answer::Label(label) if label.is_empty()),
                "missing S and missing/empty P define an empty label: {answer:?}"
            );
            assert_eq!(viewer_host::page_entry(&viewer, index).label, "");
        }
    }
}
