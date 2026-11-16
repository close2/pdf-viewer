//! Current errata and public consumer assertion for alphabetic page labels.
//! Observable current Table 161/#593 behavior for the repeated second block. The current
//! official errata resolution withdraws #432's proposed AA-to-AZ change; this avoids asserting
//! any third-block behavior and only pins an assertion in the directly clarified AA..ZZ block.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture and valid-file prerequisites"
)]

use std::fmt::Write as _;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn fixture(page_count: usize) -> Vec<u8> {
    let labels_object = page_count + 3;
    let kids = (3..labels_object)
        .map(|n| format!("{n} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        format!("<< /Type /Catalog /Pages 2 0 R /PageLabels {labels_object} 0 R >>"),
        format!("<< /Type /Pages /Count {page_count} /Kids [{kids}] >>"),
    ];
    for _ in 0..page_count {
        objects.push(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".into(),
        );
    }
    objects.push("<< /Nums [0 << /S /A >>] >>".into());
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

#[test]
fn repeated_page_label_reaches_the_public_page_list() {
    let bytes = fixture(28);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF");
    assert!(
        !parsed.was_recovered(),
        "the fixture has exact xref offsets"
    );
    assert_eq!(pdf_model::Pages::new(&parsed).len(), 28);
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DocumentId(95),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .count();

    assert!(
        matches!(viewer.query(Query::PageLabel(26)), Answer::Label(ref label) if label == "AA")
    );
    assert!(
        matches!(viewer.query(Query::PageLabel(27)), Answer::Label(ref label) if label == "BB")
    );
    assert_eq!(viewer_host::page_entry(&viewer, 27).label, "BB");
}
