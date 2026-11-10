//! Regressions for Table 197's distinction between a page being current/open and visible.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref test fixtures"
)]

use std::io::Write as _;

use pdf_model::viewer_preferences::PageLayout;
use pdf_syntax::Document;
use viewer_core::{Answer, Command, DocumentId, Event, PageTarget, Query, Viewer, Zoom};

const DOCUMENT: DocumentId = DocumentId(1);

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        writeln!(&mut bytes, "{offset:010} 00000 n ").expect("Vec writes cannot fail");
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

fn uri_action(uri: &str) -> String {
    format!("<< /S /URI /URI ({uri}) >>")
}

fn three_pages() -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 5 0 R 7 0 R] /Count 3 >>".to_vec(),
    ];
    for page in 0..3 {
        let page_object = 3 + page * 2;
        let annotation_object = page_object + 1;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
                 /Resources << >> /Annots [{annotation_object} 0 R] >>"
            )
            .into_bytes(),
        );
        objects.push(
            format!(
                "<< /Type /Annot /Subtype /Square /P {page_object} 0 R /Rect [10 10 30 30] \
                 /F 0 /AA << /PV {} /PI {} >> >>",
                uri_action(&format!("page-{page}-visible")),
                uri_action(&format!("page-{page}-invisible")),
            )
            .into_bytes(),
        );
    }
    assemble(&objects)
}

fn opened() -> (Viewer, Vec<Event>) {
    let bytes = three_pages();
    let parsed = Document::open(bytes.clone()).expect("complete valid fixture");
    assert!(!parsed.was_recovered(), "fixture has exact xref offsets");
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    (viewer, events)
}

fn uris(events: &[Event]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::OpenUri { uri, .. } => Some(uri.as_str()),
            _ => None,
        })
        .collect()
}

fn place_two_page_spread(viewer: &mut Viewer) -> Vec<Event> {
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(0.5),
            at: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Layout(PageLayout::TwoPageLeft))
        .collect()
}

fn page_is_shown(viewer: &Viewer, page: usize) -> bool {
    matches!(viewer.query(Query::PageGeometry(page)), Answer::Geometry(_))
}

#[test]
fn newly_visible_neighbor_gets_pv_before_it_becomes_current() {
    let (mut viewer, opened) = opened();
    assert_eq!(uris(&opened), ["page-0-visible"], "page zero opens visible");
    let layout_events = place_two_page_spread(&mut viewer);

    assert!(page_is_shown(&viewer, 0));
    assert!(
        page_is_shown(&viewer, 1),
        "Table 29's two-page spread places page one beside the current page"
    );
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 0, .. }
    ));
    assert!(
        uris(&layout_events).contains(&"page-1-visible"),
        "Table 197 PV is due when the neighbor becomes visible, not only when it later becomes current: {layout_events:?}"
    );
}

#[test]
fn page_remaining_visible_does_not_get_pi_when_current_page_changes() {
    let (mut viewer, opened) = opened();
    assert_eq!(uris(&opened), ["page-0-visible"]);
    place_two_page_spread(&mut viewer);
    assert!(page_is_shown(&viewer, 0));
    assert!(page_is_shown(&viewer, 1));

    let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 1, .. }
    ));
    assert!(
        page_is_shown(&viewer, 0),
        "the two-page spread still places page zero after page one becomes current"
    );
    assert!(page_is_shown(&viewer, 1));
    assert!(
        !uris(&turned).contains(&"page-0-invisible"),
        "page zero remains visible in this spread and must not receive PI: {turned:?}"
    );
}
