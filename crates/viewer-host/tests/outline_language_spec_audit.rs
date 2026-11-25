//! Specification audit of ISO 32000-2 §14.9.2.1 (review 2229): the document's language reaches
//! the outline rows a host draws.
//!
//! §14.9.2.1 makes the catalog's `/Lang` the language of text strings outside the structure
//! hierarchy as well as of page content:
//!
//! > This applies to both content within content streams and any text strings, including text
//! > strings not included in the structure hierarchy such as, for example, entries in metadata,
//! > outline entries and names for optional content groups.
//!
//! `viewer_host::outline_rows` is where every host's outline panel is built, and a row carries the
//! item's title and nothing that says what language the title is in, so a screen reader reading the
//! panel speaks a German document's outline in whatever language the desktop is in. The control is
//! that the rows do follow what they carry: a different title is a different row.

#![expect(clippy::expect_used, reason = "exact-xref test fixture")]

use std::fmt::Write as _;

use pdf_model::Pages;
use pdf_model::outline::Outline;
use pdf_syntax::Document;
use viewer_host::{PanelRow, outline_rows};

/// A one-page document whose catalog states `language` and whose one outline item is `title`.
fn document(language: &str, title: &str) -> Vec<u8> {
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R /Outlines 5 0 R /Lang ({language}) >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Outlines /First 6 0 R /Last 6 0 R /Count 1 >>".to_owned(),
        format!("<< /Title ({title}) /Parent 5 0 R /Dest [3 0 R /Fit] >>"),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
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

fn rows(language: &str, title: &str) -> Vec<PanelRow> {
    let document = Document::open(document(language, title)).expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let pages = Pages::new(&document);
    let outline = Outline::read(&document, &pages);
    assert_eq!(outline.items.len(), 1, "the one outline item is read");
    outline_rows(&outline)
}

/// 2229-B04. The control: the row carries the item's title. The failing half: a document in
/// German and one in English give the same row, so nothing a host builds from it can say which
/// language the title is in.
#[test]
fn an_outline_row_carries_the_documents_language_spec_audit() {
    assert_ne!(
        rows("de-DE", "Kapitel"),
        rows("de-DE", "Anhang"),
        "control: the row is built from the item's title"
    );
    assert_ne!(
        rows("de-DE", "Kapitel"),
        rows("en-GB", "Kapitel"),
        "§14.9.2.1: the catalog's /Lang is the language of outline entries, and the row a host \
         builds its outline panel from does not carry it"
    );
}
