//! Table 253 `/F` is a file specification, so exercise its dictionary form and `/UF` precedence.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "fixed exact-xref fixtures"
)]

use std::fmt::Write as _;

use pdf_model::{forms_data::FormsData, view::ViewState};
use pdf_syntax::{Document, FileBytes};

fn assembled(header: &str, objects: &[&str]) -> Document {
    let mut bytes = String::from(header);
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(FileBytes::from(bytes.into_bytes())).expect("valid fixture");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    document
}

fn fdf() -> Document {
    assembled(
        "%FDF-1.3\n",
        &[
            "<< /Type /Catalog /FDF 2 0 R >>",
            "<< /Pages [<< /Templates [<< /TRef << /Name (stamp) /F << /Type /Filespec /UF (library-unicode.pdf) /F (legacy-library.pdf) >> >> >>] >>] >>",
        ],
    )
}

fn target() -> Document {
    assembled(
        "%PDF-1.7\n",
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>",
        ],
    )
}

#[test]
fn named_template_file_spec_uses_the_dictionarys_unicode_filename() {
    let fdf = fdf();
    let data = FormsData::read(&fdf).expect("FDF data");
    assert_eq!(data.pages.len(), 1);
    assert_eq!(data.pages[0].templates.len(), 1);
    assert_eq!(
        data.pages[0].templates[0].reference.file.as_deref(),
        Some("library-unicode.pdf")
    );

    // A host-facing model callback sees this exact file key; no filesystem or network is used.
    let target = target();
    let mut view = ViewState::of(&target);
    let outcome = view.import(&target, &data);
    assert_eq!(outcome.pages, 0);
    assert_eq!(view.file_awaited(), Some("library-unicode.pdf"));
    assert_eq!(outcome.awaiting.len(), 1);
    assert_eq!(outcome.refused, [] as [String; 0]);
}
