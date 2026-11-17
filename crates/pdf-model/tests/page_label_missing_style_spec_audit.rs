//! ISO 32000-2 Table 161: absent S leaves only the prefix, including an empty label.
#![expect(
    clippy::expect_used,
    reason = "bounded exact-xref fixture and public assertions"
)]

use pdf_model::Pages;
use pdf_model::page_label::PageLabels;
use pdf_syntax::Document;
use std::fmt::Write as _;

fn labels(first: &str, second: &str) -> Vec<String> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /PageLabels 4 0 R >>",
        "<< /Type /Pages /Count 4 /Kids [3 0 R 5 0 R 6 0 R 7 0 R] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Nums [0 8 0 R 2 9 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        first,
        second,
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = writeln!(pdf, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let xref_at = pdf.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(pdf, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let document = Document::open(pdf.into_bytes()).expect("complete PDF fixture");
    assert!(!document.was_recovered(), "xref offsets are exact");
    assert_eq!(Pages::new(&document).len(), 4);
    let labels = PageLabels::read(&document);
    (0..4)
        .map(|index| {
            labels
                .label(index)
                .expect("a defined label, even when empty")
        })
        .collect()
}

#[test]
fn absent_style_uses_only_prefix_and_empty_prefix_is_an_empty_label() {
    assert_eq!(
        labels("<< /S /D /P (Contents) >>", "<< /S /D >>"),
        ["Contents1", "Contents2", "1", "2"],
        "explicit numeric styles and default St control"
    );
    for second in ["<< /P () >>", "<< >>"] {
        assert_eq!(
            labels("<< /P (Contents) >>", second),
            ["Contents", "Contents", "", ""],
            "absent S has no numeric portion; second range {second}"
        );
    }
}
