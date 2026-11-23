//! Specification audit of ISO 32000-2 §12.3, §12.4 and §12.6 (review 2197), through the viewer.
//!
//! Two findings, each with a control that passes on the tree it was written against:
//!
//! - 2197-B02: a remote go-to's structure destination (Table 203's `/SD`) decides the page, and
//!   §12.3.2.3 says that, "using that page, the structure destination shall behave identically to
//!   a destination" — so its own Table 149 view is applied there, and an element that identifies
//!   no page takes the first page rather than handing the decision back to `/D`.
//! - 2197-B03: §12.6.2's `/Next` actions "shall be performed in order", so a second page-naming
//!   action in a chain moves the view from where the first left it.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "small fixed fixtures whose failure must be loud, and exact xref offsets"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, ObjectId};
use viewer_core::{Answer, Command, DocumentId, Event, Purpose, Query, Viewer};

const SOURCE: DocumentId = DocumentId(1);

/// A file assembled from object bodies, object 1 the catalog, with exact xref offsets.
fn assembled(objects: &[&str]) -> Vec<u8> {
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
    let bytes = out.into_bytes();
    let parsed = Document::open(bytes.clone()).expect("a complete fixture opens");
    assert!(!parsed.was_recovered(), "the fixture's offsets are exact");
    bytes
}

/// Opens `bytes` in a 400 × 400 window at scale 1.
fn opened(bytes: Vec<u8>) -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: SOURCE,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "the fixture opens"
    );
    viewer
}

fn current_page(viewer: &Viewer) -> usize {
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("an opened viewer has a current page");
    };
    index
}

/// A one-page document whose link (object 4) performs `action`.
fn linking(action: &str) -> Vec<u8> {
    let link = format!("<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A {action} >>");
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>",
        &link,
    ])
}

/// Three 600 × 800 pages, tagged: `Chap3` is a structure element whose one content item is on
/// the third page, and `Empty` one with no content at all.
fn tagged_target() -> Vec<u8> {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>",
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>",
        "<< /Type /StructTreeRoot /K [7 0 R 10 0 R] /IDTree 9 0 R >>",
        "<< /Type /StructElem /S /Sect /P 6 0 R /ID (Chap3) /K [8 0 R] >>",
        "<< /Type /MCR /Pg 5 0 R /MCID 0 >>",
        "<< /Names [(Chap3) 7 0 R (Empty) 10 0 R] >>",
        "<< /Type /StructElem /S /Sect /P 6 0 R /ID (Empty) >>",
    ])
}

/// Activates the source's link and supplies the tagged target as the file it asks for.
fn followed_into_the_target(action: &str) -> Viewer {
    let mut viewer = opened(linking(action));
    let asked: Vec<Event> = viewer
        .handle(Command::Activate(ObjectId::new(4, 0)))
        .collect();
    assert!(
        asked.iter().any(|event| matches!(
            event,
            Event::NeedsFile { purpose: Purpose::RemoteDocument, name, .. } if name == "target.pdf"
        )),
        "the remote go-to asks for its file"
    );
    let supplied: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(tagged_target()),
        })
        .collect();
    assert!(
        supplied
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 3, .. })),
        "the target opens"
    );
    viewer
}

/// `/XYZ 40 700 2` on a 600 × 800 page in a 400 × 400 window: magnification 2, and the point
/// (40, 700) at the window's top-left corner — 80 pixels in and 200 down from the page's own.
fn shows_xyz_40_700_2(viewer: &Viewer) -> bool {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(2)) else {
        panic!("the third page has geometry");
    };
    (geometry.scale - 2.0).abs() < 1e-3
        && (geometry.origin.0 + 80.0).abs() < 2.0
        && (geometry.origin.1 + 200.0).abs() < 2.0
}

/// §12.3.2.3: "using that page, the structure destination shall behave identically to a
/// destination" — the `/SD` array's own `/XYZ` is the view of the page its element identified.
///
/// 2197-B02. The control is the same view stated by `/D`, which ADR 2964 already applies; the
/// failing half keeps the page `/SD` named and drops the rest of `/SD`, because
/// `RemoteGoTo::structure_destination` carries the element's identifier and nothing else.
#[test]
fn a_remote_structure_destination_applies_its_own_view_spec_audit() {
    let control = followed_into_the_target("<< /S /GoToR /F (target.pdf) /D [2 /XYZ 40 700 2] >>");
    assert_eq!(
        current_page(&control),
        2,
        "control: /D names the third page"
    );
    assert!(
        shows_xyz_40_700_2(&control),
        "control: /D's /XYZ is applied in the target"
    );

    let viewer = followed_into_the_target(
        "<< /S /GoToR /F (target.pdf) /D [2 /Fit] /SD [(Chap3) /XYZ 40 700 2] >>",
    );
    assert_eq!(
        current_page(&viewer),
        2,
        "Table 203: /SD's element is on the third page"
    );
    assert!(
        shows_xyz_40_700_2(&viewer),
        "§12.3.2.3: the structure destination behaves identically to a destination, so its \
         /XYZ 40 700 2 is the view"
    );
}

/// §12.3.2.3: "In the case where no page content is identified, then the page reference shall be
/// assumed to be the first page in the document."
///
/// 2197-B02. The control is the element with content, whose page `/SD` decides over `/D`'s; the
/// failing half is an element that identifies no page, where `RemoteGoTo::page_in` falls through
/// to `/D` rather than taking the first page as the local reading (`Destination::resolve`) does.
#[test]
fn a_remote_structure_destination_without_content_takes_the_first_page_spec_audit() {
    let control =
        followed_into_the_target("<< /S /GoToR /F (target.pdf) /D [1 /Fit] /SD [(Chap3) /Fit] >>");
    assert_eq!(
        current_page(&control),
        2,
        "control: /SD's element decides over /D"
    );

    let viewer =
        followed_into_the_target("<< /S /GoToR /F (target.pdf) /D [1 /Fit] /SD [(Empty) /Fit] >>");
    assert_eq!(
        current_page(&viewer),
        0,
        "§12.3.2.3: an element under which no page content is identified names the first page"
    );
}

/// Three pages; the link on the first (object 6) performs `action`.
fn three_pages_linking(action: &str) -> Vec<u8> {
    let link = format!("<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /P 3 0 R /A {action} >>");
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [6 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        &link,
    ])
}

/// The page shown after the first page's link performs `action`.
fn page_after(action: &str) -> usize {
    let mut viewer = opened(three_pages_linking(action));
    assert_eq!(
        current_page(&viewer),
        0,
        "the document opens on its first page"
    );
    viewer
        .handle(Command::Activate(ObjectId::new(6, 0)))
        .for_each(drop);
    current_page(&viewer)
}

/// §12.6.2's `/Next`: "an array of action dictionaries that shall be performed in order". A
/// `NextPage` performed after another is performed from the page the first one reached.
///
/// 2197-B03. The controls are one `NextPage` and one go-to; the failing halves are chains of two,
/// where `interact::resolve`'s `Jump` keeps the first page any request names and sets
/// Table 215's `NextPage` against the page the chain started from.
#[test]
fn a_chain_of_page_actions_is_performed_in_order_spec_audit() {
    assert_eq!(
        page_after("<< /S /Named /N /NextPage >>"),
        1,
        "control: one NextPage"
    );
    assert_eq!(
        page_after("<< /S /GoTo /D [4 0 R /Fit] >>"),
        1,
        "control: one go-to"
    );

    let performed = [
        (
            "two NextPage actions",
            page_after("<< /S /Named /N /NextPage /Next << /S /Named /N /NextPage >> >>"),
        ),
        (
            "a go-to to the second page, then NextPage",
            page_after("<< /S /GoTo /D [4 0 R /Fit] /Next [<< /S /Named /N /NextPage >>] >>"),
        ),
        (
            "a go-to to the second page, then one to the third",
            page_after("<< /S /GoTo /D [4 0 R /Fit] /Next << /S /GoTo /D [5 0 R /Fit] >> >>"),
        ),
    ];
    assert!(
        performed.iter().all(|(_, page)| *page == 2),
        "§12.6.2: each chain ends on the third page (index 2): {performed:?}"
    );
}
