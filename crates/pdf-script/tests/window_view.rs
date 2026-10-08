//! ADR 1736 driven through `pdf_model::view::ViewState` with the in-process engine: `this.zoom`,
//! `this.zoomType`, `this.layout` read as a host told the view state, written as requests a host
//! takes, and `this.scroll` handing the host a point of default user space.
//!
//! The members' meanings are Adobe's "Doc properties" and "Doc methods", each a documented choice
//! under principle 5; the expected layouts are Table 29's six names, and the rotated user space a
//! scroll is stated in is ADR 1724's reading, undone here by ISO 32000-2 Table 31's `/Rotate`.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{ViewChange, ViewState, WindowView, ZoomType};
use pdf_model::viewer_preferences::PageLayout;
use pdf_script::{Budget, Engine};
use pdf_syntax::Document;

/// A one-page document — Letter, turned a quarter — whose open action is `open`, whose catalog
/// also states `catalog`, and whose text field `Out` the script writes what it read into.
fn document(open: &str, catalog: &str) -> Document {
    let bodies = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> {catalog} \
             /OpenAction << /S /JavaScript /JS ({open}) >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 90 /Annots [4 0 R] >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Out) /Rect [10 10 210 40] /F 4 /P 3 0 R \
         /DA (/Helv 10 Tf 0 g) >>"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// A view state of `document` with the engine supplied, told `window` where one is given, and the
/// document opened.
fn opened(document: &Document, window: Option<WindowView>) -> ViewState {
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::new(Engine::new(Budget::FIELD_EVENT))));
    if let Some(window) = window {
        view.set_window_view(window);
    }
    view.run_open_scripts(document, 0);
    view
}

/// What `Out` shows.
fn out(view: &ViewState, document: &Document) -> String {
    view.field_value(document, "Out")
        .map(|shown| shown.text)
        .unwrap_or_default()
}

/// What the script reads is what the host told.
#[test]
fn the_window_s_view_is_read_as_the_host_told_it() {
    let document = document(
        "getField\\('Out'\\).value = [zoom, zoomType, layout].join\\('|'\\);",
        "",
    );
    let view = opened(
        &document,
        Some(WindowView {
            zoom: Some(150.0),
            zoom_type: ZoomType::FitWidth,
            layout: PageLayout::TwoColumnLeft,
        }),
    );
    assert_eq!(out(&view, &document), "150|FitWidth|TwoColumnLeft");
}

/// With no window told, the layout is the one Table 29's `/PageLayout` asks the window to open
/// with, and no magnification is drawn, so `zoom` is `undefined`.
#[test]
fn with_no_window_told_the_layout_is_the_catalog_s_and_the_zoom_undefined() {
    let document = document(
        "getField\\('Out'\\).value = [typeof zoom, zoomType, layout].join\\('|'\\);",
        "/PageLayout /TwoPageRight",
    );
    let view = opened(&document, None);
    assert_eq!(out(&view, &document), "undefined|NoVary|TwoPageRight");
}

/// Each write is a request a host takes, in the order asked, the latest of a kind standing in the
/// first one's place; the script reads back what it wrote, and a fixed zoom is `NoVary`.
#[test]
fn a_script_s_writes_are_held_for_the_host_and_read_back() {
    let document = document(
        "zoom = 200; layout = 'OneColumn'; zoomType = zoomtype.fitP; zoom = 300; \
         getField\\('Out'\\).value = [zoom, zoomType, layout, zoomtype.fitV].join\\('|'\\);",
        "",
    );
    let mut view = opened(&document, None);
    assert_eq!(
        out(&view, &document),
        "300|NoVary|OneColumn|FitVisibleWidth"
    );
    assert_eq!(
        view.take_view_requests(),
        vec![
            ViewChange::Zoom(300.0),
            ViewChange::Layout(PageLayout::OneColumn),
            ViewChange::ZoomType(ZoomType::FitPage),
        ]
    );
    assert!(view.take_view_requests().is_empty(), "taken once");
}

/// A zoom outside the reference's 8.33 to 6400 per cent, a magnification this program does not
/// have, and names neither list holds change nothing, and each is said.
#[test]
fn a_value_that_names_nothing_drawable_changes_nothing_and_is_said() {
    let document = document(
        "zoom = 10000; zoomType = 'ReflowWidth'; zoomType = 'Sideways'; layout = 'Spiral'; \
         getField\\('Out'\\).value = [typeof zoom, zoomType, layout].join\\('|'\\);",
        "",
    );
    let mut view = opened(&document, None);
    assert_eq!(out(&view, &document), "undefined|NoVary|SinglePage");
    assert!(view.take_view_requests().is_empty());
    let said = view
        .script_reports()
        .iter()
        .filter(|sentence| sentence.contains("the view does not change (ADR 1736)"))
        .count();
    assert_eq!(said, 4, "{:?}", view.script_reports());
}

/// `scroll` reads its point in rotated user space and hands the host default user space: on a
/// Letter page turned a quarter, ADR 1724's rotated space is 792 wide and 612 high, and its point
/// (100, 50) is the point (562, 100) of the page as the file states it — Table 31 turns the page
/// clockwise, so what was 100 up from the bottom is 100 in from the left.
#[test]
fn a_scroll_hands_the_host_a_point_of_default_user_space() {
    let document = document("scroll\\(100, 50\\);", "");
    let mut view = opened(&document, None);
    assert_eq!(
        view.take_view_requests(),
        vec![ViewChange::Scroll {
            page: 0,
            x: 562.0,
            y: 100.0,
        }]
    );
}
