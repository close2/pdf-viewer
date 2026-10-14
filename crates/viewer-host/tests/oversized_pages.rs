//! A page whose whole raster would be past `viewer_core::MAX_PIXELS`, asked for, drawn as the
//! window shows it and answered, through the pieces both toolkit windows use (ADR 1892).

use std::time::{Duration, Instant};

use viewer_core::{Command, DocumentId, Event, Oversize, Query, Rendered, Viewer, Zoom};
use viewer_host::Drawing;
use viewer_host::oversize::{Oversized, oversized};

/// The identity the one document is opened under.
const DOCUMENT: DocumentId = DocumentId(1);

/// Writes object bodies numbered from 1 as a file with §7.5.4's table and a trailer.
fn assembled(objects: &[&str]) -> Vec<u8> {
    use std::fmt::Write as _;
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
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

/// One A1 page (ISO 216's 594 × 841 mm, 1684 × 2384 points) filled red.
fn a1_in_red() -> Vec<u8> {
    let content = "1 0 0 rg 0 0 1684 2384 re f";
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1684 2384] /Contents 4 0 R >>",
        &format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
    ])
}

/// Where the arrangement places `page`.
fn placed(viewer: &Viewer, page: usize) -> Option<viewer_core::PageGeometry> {
    match viewer.query(Query::PageGeometry(page)) {
        viewer_core::Answer::Geometry(geometry) => Some(geometry),
        _ => None,
    }
}

/// The render requests in `events`.
fn requests(events: Vec<Event>) -> Vec<viewer_core::RenderRequest> {
    events
        .into_iter()
        .filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .collect()
}

#[test]
fn a_page_past_the_bound_is_drawn_as_the_window_shows_it_and_answered_listed() {
    let viewport = (800, 600);
    let mut viewer = Viewer::new(viewport.0, viewport.1, 1.0);
    viewer
        .handle(Command::Oversize(Oversize::Listed))
        .for_each(drop);
    let opened = requests(
        viewer
            .handle(Command::Open {
                id: DOCUMENT,
                bytes: a1_in_red().into(),
                password: None,
                fragment: None,
            })
            .collect(),
    );
    assert!(opened.iter().all(|request| !oversized(request)));
    let zoomed: Vec<Event> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(20.0),
            at: None,
        })
        .collect();
    assert!(
        !zoomed
            .iter()
            .any(|event| matches!(event, Event::Reported { .. })),
        "a host that draws such a page is not refused it: {zoomed:?}"
    );
    let asked = requests(zoomed);
    let [request] = asked.as_slice() else {
        panic!("one page, one request: {asked:?}");
    };
    assert!(
        oversized(request),
        "the case has to be past the bound to be the case"
    );

    let mut held = Oversized::default();
    assert!(held.asked(request));
    let due = held.due(DOCUMENT, |page| placed(&viewer, page), viewport);
    let [(window_of, window)] = due.as_slice() else {
        panic!("the page on the screen is due once: {due:?}");
    };
    // The viewport and a quarter of it on every side, inside a page far larger than both — a
    // pixel more where the page's origin is at a fraction and a part of one more column shows.
    assert!(
        (1200..=1202).contains(&window.width) && (900..=902).contains(&window.height),
        "{window:?}"
    );
    assert!(
        held.due(DOCUMENT, |page| placed(&viewer, page), viewport)
            .is_empty(),
        "a window asked for is not asked again"
    );

    let mut drawing: Drawing = Drawing::new();
    drawing.ask_window(window_of.clone(), *window);
    let began = Instant::now();
    let landed = loop {
        drawing.collect();
        let landed = drawing.windows();
        if !landed.is_empty() {
            break landed;
        }
        assert!(
            began.elapsed() < Duration::from_mins(1),
            "the window never landed"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    let [(finished, drawn)] = landed.as_slice() else {
        panic!("one window: {landed:?}");
    };
    let Some(Rendered::Raster(raster)) = finished.outcome.as_ref() else {
        panic!("the window was drawn: {:?}", finished.outcome);
    };
    assert_eq!((raster.width, raster.height), (drawn.width, drawn.height));
    // Every pixel of a window inside a page filled red is red: the window is of the page, at
    // the page's own transform moved to the window's corner.
    assert!(
        raster
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 0, 0, 255]),
        "a pixel of the window is not the page's red"
    );

    let mut landed = landed;
    let (finished, drawn) = landed.remove(0);
    let Some((token, rendered)) = held.landed(finished, drawn) else {
        panic!("the first window of a request answers it");
    };
    assert!(matches!(rendered, Rendered::Listed));
    let after: Vec<Event> = viewer
        .handle(Command::RenderReady { token, rendered })
        .collect();
    assert!(
        requests(after).is_empty(),
        "the page holds its place and is not asked for again"
    );
    assert!(
        held.due(DOCUMENT, |page| placed(&viewer, page), viewport)
            .is_empty(),
        "and what the window drew holds what it shows"
    );
}
