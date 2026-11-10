//! §12.3.3 Count signs and linked order reach the pure outline panel.
//!
//! The source is a valid PDF 1.7 with a complete classic xref, Catalog/Pages/Page, `MediaBox` and
//! page Resources. It follows the parsed outline all the way into viewer-ui's pure Sidebar API.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "the self-contained fixture must fail loudly if it stops parsing"
)]

use std::fmt::Write as _;

use pdf_model::outline::Outline;
use pdf_syntax::{Document, ObjectId};
use viewer_ui::chrome::{Content, Hit, Sidebar};

fn pdf_with_nested_open_and_closed_outline() -> Vec<u8> {
    let objects: [&[u8]; 10] = [
        b"<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        // Five visible entries: Chapter A + three visible descendants + Chapter B.
        b"<< /Type /Outlines /First 5 0 R /Last 8 0 R /Count 5 >>",
        b"<< /Title (Chapter A) /Parent 4 0 R /First 6 0 R /Last 7 0 R /Count 3 /Next 8 0 R >>",
        b"<< /Title (Open child) /Parent 5 0 R /First 9 0 R /Last 9 0 R /Count 1 /Next 7 0 R >>",
        b"<< /Title (Closed child) /Parent 5 0 R /Prev 6 0 R /First 10 0 R /Last 10 0 R /Count -1 >>",
        b"<< /Title (Chapter B) /Parent 4 0 R /Prev 5 0 R >>",
        b"<< /Title (Visible grandchild) /Parent 6 0 R >>",
        b"<< /Title (Initially hidden grandchild) /Parent 7 0 R >>",
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(body);
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

static NOTHING: std::sync::LazyLock<pdf_model::metadata::Information> =
    std::sync::LazyLock::new(pdf_model::metadata::Information::default);

fn content(outline: &Outline) -> Content<'_> {
    Content {
        outline,
        layers: &[],
        attachments: &[],
        articles: &[],
        collection: None,
        information: &NOTHING,
        metadata: None,
        page_count: 0,
        pages: None,
    }
}

#[test]
fn pdf_outline_count_controls_initial_panel_visibility_and_user_toggle() {
    let document = Document::open(pdf_with_nested_open_and_closed_outline())
        .expect("the fixture has a complete xref and matching references");
    assert!(
        !document.was_recovered(),
        "the fixture uses its valid classic xref"
    );
    let pages = pdf_model::page::Pages::new(&document);
    let outline = Outline::read(&document, &pages);

    assert_eq!(outline.stated_count, Some(5));
    assert_eq!(outline.visible_count(), 5);
    assert_eq!(outline.items.len(), 2);
    let chapter_a = &outline.items[0];
    assert!(chapter_a.open, "positive Count opens Chapter A");
    assert_eq!(chapter_a.children.len(), 2);
    assert_eq!(chapter_a.children[0].title, "Open child");
    assert!(
        chapter_a.children[0].open,
        "positive Count opens the nested child"
    );
    assert_eq!(chapter_a.children[1].title, "Closed child");
    assert!(
        !chapter_a.children[1].open,
        "negative Count closes this child"
    );
    assert_eq!(
        chapter_a.children[1].children[0].title,
        "Initially hidden grandchild"
    );

    let panel_content = content(&outline);
    let mut panel = Sidebar::default();
    panel.toggle();

    // Visible order is depth-first linked-list order: Chapter A, Open child, its grandchild,
    // Closed child, Chapter B. The closed child's grandchild is hidden, so Chapter B retains row 5.
    for (y, object, title) in [
        (36.0, 5, "Chapter A"),
        (56.0, 6, "Open child"),
        (76.0, 9, "Visible grandchild"),
        (96.0, 7, "Closed child"),
        (116.0, 8, "Chapter B"),
    ] {
        assert_eq!(
            panel.click((80.0, y), panel_content, 1.0),
            Some(Hit::Activate(ObjectId::new(object, 0))),
            "visible row {title} follows linked-list order and activates its own item"
        );
    }

    assert_eq!(
        panel.click((6.0, 96.0), panel_content, 1.0),
        Some(Hit::Redraw)
    );
    assert_eq!(
        panel.click((80.0, 116.0), panel_content, 1.0),
        Some(Hit::Activate(ObjectId::new(10, 0))),
        "opening the negative-Count child reveals its grandchild"
    );

    assert_eq!(
        panel.click((6.0, 96.0), panel_content, 1.0),
        Some(Hit::Redraw)
    );
    assert_eq!(
        panel.click((80.0, 116.0), panel_content, 1.0),
        Some(Hit::Activate(ObjectId::new(8, 0))),
        "closing it hides its descendant and restores Chapter B's row position"
    );
}
