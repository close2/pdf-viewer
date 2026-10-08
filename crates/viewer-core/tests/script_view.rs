//! `this.zoom`, `this.zoomType`, `this.layout` and `this.scroll`, carried out by the viewer as the
//! person's own zoom, layout and scroll, and the window's view told to the runner with every event
//! (ADR 1736).
//!
//! The runner here is the test's own, answering the open action with the edits it is given and
//! keeping the view each event handed it, so what is judged is the viewer's half — what it tells and
//! what it carries out — and never what a script engine does with Adobe's members.

#![expect(
    clippy::panic,
    reason = "test code: a viewer that answers no view must fail loudly rather than pass by doing \
              nothing"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use pdf_model::view::{
    ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewChange, WindowView,
    ZoomType,
};
use pdf_model::viewer_preferences::PageLayout;
use viewer_core::{Answer, Command, DocumentId, Query, ScriptRunners, Scripting, Viewer, Zoom};

const DOCUMENT: DocumentId = DocumentId(1);

/// Two pages 400 units square, an open action whose script the runner below answers, and Table
/// 200's `/WC`, so that closing the document hands the runner a second event.
fn two_pages() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (view\\(\\);) >> \
         /AA << /WC << /S /JavaScript /JS (closing\\(\\);) >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 400 400] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R >>".to_owned(),
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
    out.into_bytes()
}

/// A runner that answers the open action with `changes`, and keeps every view it was handed.
#[derive(Debug, Default)]
struct ViewRunner {
    changes: Vec<ViewChange>,
    told: Arc<Mutex<Vec<WindowView>>>,
}

impl ScriptRunner for ViewRunner {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.told
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event.view);
        let edits = if event.site == ScriptSite::OpenAction {
            self.changes
                .iter()
                .map(|change| ScriptEdit::View { change: *change })
                .collect()
        } else {
            Vec::new()
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct Viewers(Vec<ViewChange>, Arc<Mutex<Vec<WindowView>>>);

impl ScriptRunners for Viewers {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(ViewRunner {
            changes: self.0.clone(),
            told: Arc::clone(&self.1),
        })
    }
}

/// A viewer 800 by 1000 device pixels at one device pixel per logical one, with the document open
/// and its open action run, and where every view its runner is handed is kept.
fn opened(changes: Vec<ViewChange>) -> (Viewer, Arc<Mutex<Vec<WindowView>>>) {
    let told = Arc::new(Mutex::new(Vec::new()));
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Viewers(
            changes,
            Arc::clone(&told),
        )))))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: two_pages().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    (viewer, told)
}

/// Every view the runner has been handed, in order.
fn told(kept: &Mutex<Vec<WindowView>>) -> Vec<WindowView> {
    kept.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// Where the viewer has put the reader.
fn view(viewer: &Viewer) -> viewer_core::Viewing {
    match viewer.query(Query::View) {
        Answer::View(viewing) => viewing,
        other => panic!("the document is open and has a view: {other:?}"),
    }
}

/// The open action is handed the window as it opened: the whole page fitted, which on a page 400
/// units square in a window 800 wide and 1000 high is two logical pixels a unit — 200 per cent —
/// and Table 29's default arrangement.
#[test]
fn the_window_s_view_is_told_with_the_open_action() {
    let (_, kept) = opened(Vec::new());
    let told = told(&kept);
    assert_eq!(
        told.first(),
        Some(&WindowView {
            zoom: Some(200.0),
            zoom_type: ZoomType::FitPage,
            layout: PageLayout::SinglePage,
        }),
        "{told:?}"
    );
}

/// A zoom of 300 per cent is three logical pixels a unit, and a layout is the person's own; the
/// next event a script is handed is told the window as the script left it.
#[test]
fn a_script_s_zoom_and_layout_are_the_window_s_after_the_command_that_ran_it() {
    let (mut viewer, kept) = opened(vec![
        ViewChange::Zoom(300.0),
        ViewChange::Layout(PageLayout::OneColumn),
    ]);
    assert_eq!(view(&viewer).zoom, Zoom::Scale(3.0));
    viewer.handle(Command::Close(DOCUMENT)).for_each(drop);
    let told = told(&kept);
    assert_eq!(
        told.last(),
        Some(&WindowView {
            zoom: Some(300.0),
            zoom_type: ZoomType::NoVary,
            layout: PageLayout::OneColumn,
        }),
        "{told:?}"
    );
}

/// `zoomType` names a fitting mode, and `NoVary` holds the magnification the mode resolved to.
#[test]
fn a_zoom_type_is_the_fitting_mode_of_its_name_and_no_vary_holds_the_fit() {
    let (viewer, _) = opened(vec![ViewChange::ZoomType(ZoomType::FitWidth)]);
    assert_eq!(view(&viewer).zoom, Zoom::FitWidth);
    let (viewer, _) = opened(vec![ViewChange::ZoomType(ZoomType::NoVary)]);
    // The window opened fitted at two logical pixels a unit.
    assert_eq!(view(&viewer).zoom, Zoom::Scale(2.0));
}

/// `scroll` brings the point to the middle of the window: at four pixels a unit the point
/// (250, 200) is 1000 pixels from the page's left and 800 from its top, so the scroll is the
/// window's half — 400 and 500 — short of each.
#[test]
fn a_scroll_brings_its_point_to_the_middle_of_the_window() {
    let (viewer, _) = opened(vec![
        ViewChange::Zoom(400.0),
        ViewChange::Scroll {
            page: 0,
            x: 250.0,
            y: 200.0,
        },
    ]);
    let viewing = view(&viewer);
    assert_eq!(viewing.page, 0);
    assert_eq!(viewing.scroll, (600.0, 300.0));
}

/// A scroll on a page other than the one showing turns to it first.
#[test]
fn a_scroll_on_another_page_turns_to_it() {
    let (viewer, _) = opened(vec![
        ViewChange::Zoom(400.0),
        ViewChange::Scroll {
            page: 1,
            x: 250.0,
            y: 200.0,
        },
    ]);
    let viewing = view(&viewer);
    assert_eq!((viewing.page, viewing.scroll), (1, (600.0, 300.0)));
}

/// `gotoNamedDest`'s destination is shown as a link's is (ADR 1751): Table 149's `/XYZ` puts
/// `left` at the window's left edge and `top` at its top, at its own magnification — at four pixels
/// a unit, the point (100, 300) is 400 pixels from page two's left and 400 from its top.
#[test]
fn a_named_destination_s_view_is_shown_on_its_page() {
    let (viewer, _) = opened(vec![ViewChange::Destination {
        page: 1,
        view: pdf_model::destination::View::Xyz {
            left: Some(100.0),
            top: Some(300.0),
            zoom: Some(4.0),
        },
    }]);
    let viewing = view(&viewer);
    assert_eq!(
        (viewing.page, viewing.zoom, viewing.scroll),
        (1, Zoom::Scale(4.0), (400.0, 400.0))
    );
}

/// Table 149's `/Fit` on the page already showing fits it and turns nothing: the whole page 400
/// units square fits an 800 by 1000 window at two pixels a unit, so the 400 per cent set before it
/// is undone, as a link's `/Fit` undoes it.
#[test]
fn a_named_destination_on_the_page_showing_changes_only_the_view() {
    let (viewer, _) = opened(vec![
        ViewChange::Zoom(400.0),
        ViewChange::Destination {
            page: 0,
            view: pdf_model::destination::View::Fit,
        },
    ]);
    let viewing = view(&viewer);
    assert_eq!((viewing.page, viewing.zoom), (0, Zoom::Scale(2.0)));
}
