//! Specification audit of ISO 32000-2 §12.3, §12.4 and §12.6 (review 2197): the destination an
//! outline item's go-to action names.
//!
//! Table 202's `/SD` is read where a link's or an open action's go-to is read
//! (`Destination::of_go_to`), and the outline is a third route to the same action dictionary.
//! The fixture's `/D` names the first page and its `/SD` a structure element whose only content
//! item is on the third, so the two answers differ.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a broken fixture must fail loudly, and the xref offsets are small sums"
)]

use std::fmt::Write as _;

use pdf_model::outline::Outline;
use pdf_model::page::Pages;
use pdf_syntax::Document;

/// A document assembled from object bodies, object 1 the catalog, with exact xref offsets.
fn assembled(objects: &[&str]) -> Document {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let document = Document::open(out.into_bytes()).expect("a complete fixture opens");
    assert!(!document.was_recovered(), "the fixture's offsets are exact");
    document
}

/// Three pages, a structure element (object 7) whose content item is on the third (object 5), a
/// named destination `Chapter` stating the `/D` and `/SD` the action states, and an outline whose
/// one item states `item_entries`.
fn tagged_with_outline(item_entries: &str) -> Document {
    let item = format!("<< /Title (Chapter) /Parent 9 0 R {item_entries} >>");
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R /Outlines 9 0 R \
         /Dests << /Chapter << /D [3 0 R /Fit] /SD [7 0 R /Fit] >> >> >>",
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /Type /StructTreeRoot /K [7 0 R] >>",
        "<< /Type /StructElem /S /Sect /P 6 0 R /K [8 0 R] >>",
        "<< /Type /MCR /Pg 5 0 R /MCID 0 >>",
        "<< /Type /Outlines /First 10 0 R /Last 10 0 R /Count 1 >>",
        &item,
    ])
}

/// The page the outline's one item resolves to.
fn item_page(document: &Document) -> Option<usize> {
    let pages = Pages::new(document);
    let outline = Outline::read(document, &pages);
    let item = outline.items.first().expect("the outline has its one item");
    item.destination
        .and_then(|destination| destination.page_index(document, &pages))
}

/// Table 202: "If present, the structure destination should take precedence over destination in
/// the D entry" — for the outline item's `/A` as for a link's.
///
/// 2197-B01. The control is the same pair of entries reached through the item's `/Dest`, a named
/// destination whose `/Dests` value is §12.3.2.4's dictionary form, which `Destination::read`
/// already answers with the element's page; the failing half is the `/A` route, which
/// `outline::destination` reads by `/D` alone.
#[test]
fn an_outline_items_go_to_action_prefers_its_structure_destination_spec_audit() {
    let control = tagged_with_outline("/Dest /Chapter");
    assert_eq!(
        item_page(&control),
        Some(2),
        "control: a destination dictionary's /SD names the element's page"
    );

    let document = tagged_with_outline("/A << /S /GoTo /D [3 0 R /Fit] /SD [7 0 R /Fit] >>");
    assert_eq!(
        item_page(&document),
        Some(2),
        "Table 202: the go-to action's /SD takes precedence over its /D, and the element's \
         content item is on the third page"
    );
}
