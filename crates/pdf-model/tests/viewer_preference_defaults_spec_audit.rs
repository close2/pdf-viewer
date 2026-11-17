//! Table 147's stated defaults, independently of the implementation's Default values.
#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "bounded exact-xref fixture"
)]

use std::fmt::Write as _;

use pdf_model::{
    page::Boundary,
    viewer_preferences::{
        Direction, Opening, PageLayout, PageMode, PrintScaling, ViewerPreferences,
    },
};
use pdf_syntax::Document;

fn document() -> Document {
    // A present empty dictionary invokes the entry defaults. With no dictionary, §12.2
    // instead recommends the reader's current user preferences.
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = write!(bytes, "{} 0 obj\n{object}\nendobj\n", index + 1);
    }
    let xref = bytes.len();
    let _ = write!(bytes, "xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(bytes.into_bytes()).expect("valid one-page fixture");
    assert!(!document.was_recovered());
    document
}

#[test]
fn a_present_empty_dictionary_uses_literal_source_defaults() {
    let document = document();
    let preferences = ViewerPreferences::read(&document);
    assert!(!preferences.hide_toolbar);
    assert!(!preferences.hide_menubar);
    assert!(!preferences.hide_window_ui);
    assert!(!preferences.fit_window);
    assert!(!preferences.center_window);
    assert!(!preferences.display_doc_title);
    assert_eq!(preferences.non_full_screen_page_mode, PageMode::UseNone);
    assert_eq!(preferences.direction, Direction::LeftToRight);
    assert_eq!(preferences.view_area, Boundary::Crop);
    assert_eq!(preferences.view_clip, Boundary::Crop);
    assert_eq!(preferences.print_area, Boundary::Crop);
    assert_eq!(preferences.print_clip, Boundary::Crop);
    assert_eq!(preferences.print_scaling, PrintScaling::AppDefault);
    // Duplex, tray choice, print range and copy count have implementation-dependent
    // defaults. This source-default test deliberately asserts no fixed choice for them.
    let opening = Opening::read(&document);
    assert_eq!(opening.mode, PageMode::UseNone);
    assert_eq!(opening.layout, PageLayout::SinglePage);
}
