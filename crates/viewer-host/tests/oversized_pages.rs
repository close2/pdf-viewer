//! A page zoomed past its window — twice the largest window or more, a window that leaves out an
//! eighth of its commands, or past `viewer_core::MAX_PIXELS` — asked for, drawn as the window
//! shows it and answered, through the pieces both toolkit windows use (ADRs 1892, 1930, 1994).

use std::time::{Duration, Instant};

use viewer_core::{Command, DocumentId, Event, Oversize, Query, Rendered, Viewer, Zoom};
use viewer_host::Drawing;
use viewer_host::drawing::Finished;
use viewer_host::oversize::{Oversized, largest_window, oversized};

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
    a1_of("1 0 0 rg 0 0 1684 2384 re f")
}

/// One A1 page drawn by `content`.
fn a1_of(content: &str) -> Vec<u8> {
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
    assert!(opened.iter().all(|request| !oversized(request, viewport)));
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
        oversized(request, viewport),
        "the case has to be past the bound to be the case"
    );

    let mut held = Oversized::default();
    assert!(held.asked(request, viewport));
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

/// The one request a zoom to `scale` makes.
#[expect(
    clippy::panic,
    reason = "a test's helper stops the test where its case is not the case"
)]
fn zoomed(viewer: &mut Viewer, scale: f32) -> viewer_core::RenderRequest {
    let asked = requests(
        viewer
            .handle(Command::Zoom {
                zoom: Zoom::Scale(scale),
                at: None,
            })
            .collect(),
    );
    let [request] = asked.as_slice() else {
        panic!("one page, one request: {asked:?}");
    };
    request.clone()
}

/// A viewer of `viewport` device pixels that draws a page past the bound from its list, with the
/// A1 page open.
fn opened_a1(viewport: (u32, u32)) -> Viewer {
    opened(viewport, a1_in_red())
}

/// A viewer of `viewport` device pixels that draws a page past the bound from its list, with
/// `bytes` open.
fn opened(viewport: (u32, u32), bytes: Vec<u8>) -> Viewer {
    let mut viewer = Viewer::new(viewport.0, viewport.1, 1.0);
    viewer
        .handle(Command::Oversize(Oversize::Listed))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

/// What the drawing hands back for `window` of `request`: the window drawn, in red.
fn drawn_red(
    request: &viewer_core::RenderRequest,
    window: viewer_host::oversize::Window,
) -> Finished {
    let pixels = usize::try_from(u64::from(window.width).saturating_mul(u64::from(window.height)))
        .unwrap_or(usize::MAX);
    Finished {
        request: request.clone(),
        outcome: Some(Rendered::Raster(pdf_render::Raster {
            width: window.width,
            height: window.height,
            format: pdf_render::RasterFormat::Rgba8,
            data: [255, 0, 0, 255].repeat(pixels),
        })),
        declined: Vec::new(),
        cost: Duration::ZERO,
        waited: Duration::ZERO,
    }
}

#[test]
fn a_page_whose_window_is_more_than_half_of_it_is_drawn_whole_and_one_of_half_or_less_is_not() {
    // ADR 1930: the viewport and a quarter of it on every side is the most a window of a page is
    // drawn as, and a page is drawn as its windows where that is at most half of it.
    let viewport = (800, 600);
    assert_eq!(largest_window(viewport), (1200, 900));
    let area = |request: &viewer_core::RenderRequest| {
        u64::from(request.target.width) * u64::from(request.target.height)
    };
    let mut viewer = opened_a1(viewport);
    // The A1 page is 1684 × 2384 points: at 0.72 it is 1213 × 1717 and its window 1200 × 900, a
    // little over half of it; at 0.75, 1263 × 1788, a little under.
    let whole = zoomed(&mut viewer, 0.72);
    assert!(2 * 1200 * 900 > area(&whole), "{:?}", whole.target);
    assert!(!oversized(&whole, viewport), "{:?}", whole.target);
    let mut held = Oversized::default();
    assert!(!held.asked(&whole, viewport));
    assert!(
        held.due(DOCUMENT, |page| placed(&viewer, page), viewport)
            .is_empty(),
        "a page drawn whole asks for no window"
    );
    let past = zoomed(&mut viewer, 0.75);
    assert!(2 * 1200 * 900 <= area(&past), "{:?}", past.target);
    assert!(oversized(&past, viewport), "{:?}", past.target);
    assert!(held.asked(&past, viewport));
    assert_eq!(
        held.due(DOCUMENT, |page| placed(&viewer, page), viewport)
            .len(),
        1,
        "the page twice its window is drawn as the window shows it"
    );
}

#[test]
fn a_page_whose_window_is_more_than_half_of_it_and_leaves_out_most_of_its_commands_is_drawn_as_it()
{
    // ADR 1994: a window that keeps at most seven eighths of its page's commands is drawn in the
    // page's place whatever its share of the pixels. The red A1 page with a hundred specks along
    // its foot, which no window about its middle reaches: such a window keeps one command of 101.
    let viewport = (800, 600);
    let content: String = std::iter::once(String::from("1 0 0 rg 0 0 1684 2384 re f\n0 0 1 rg\n"))
        .chain((0..100).map(|speck: u32| format!("{} 10 4 4 re f\n", 10 + speck * 16)))
        .collect();
    let mut viewer = opened(viewport, a1_of(&content));
    // At 0.72 the page is 1213 × 1717 and its window 1200 × 900, a little over half of it.
    let windowed = zoomed(&mut viewer, 0.72);
    assert!(
        2 * 1200 * 900 > u64::from(windowed.target.width) * u64::from(windowed.target.height),
        "{:?}",
        windowed.target
    );
    assert!(oversized(&windowed, viewport), "{:?}", windowed.target);
    // The red page alone keeps its one command in that window, and is drawn whole.
    let mut red = opened_a1(viewport);
    assert!(!oversized(&zoomed(&mut red, 0.72), viewport));
}

#[test]
fn a_window_drawn_under_the_last_request_stands_in_for_the_next_and_does_not_answer_it() {
    // A new request at the same target is the page under another ink — or interpreted again — and
    // the window drawn for the last one is not its picture: it stands in, and a window is drawn.
    let viewport = (800, 600);
    let mut viewer = opened_a1(viewport);
    let first = zoomed(&mut viewer, 2.0);
    let mut held = Oversized::default();
    assert!(held.asked(&first, viewport));
    let due = held.due(DOCUMENT, |page| placed(&viewer, page), viewport);
    let [(_, window)] = due.as_slice() else {
        panic!("one window: {due:?}");
    };
    assert!(held.landed(drawn_red(&first, *window), *window).is_some());
    assert!(held.shows(DOCUMENT));
    zoomed(&mut viewer, 3.0);
    let again = zoomed(&mut viewer, 2.0);
    assert_eq!(again.target, first.target, "the same target, asked again");
    assert_ne!(again.token, first.token);
    assert!(held.asked(&again, viewport));
    assert!(
        !held.shows(DOCUMENT),
        "the window drawn is not this request's"
    );
    let placed_now = held.place(Vec::new(), None, DOCUMENT, |page| placed(&viewer, page));
    assert_eq!(placed_now.len(), 1, "the last window stands in meanwhile");
    assert_eq!(
        held.due(DOCUMENT, |page| placed(&viewer, page), viewport)
            .len(),
        1,
        "and the new request's window is drawn"
    );
}

#[test]
fn a_notch_back_to_a_page_drawn_whole_keeps_the_last_window_standing_in_until_its_frame_lands() {
    let viewport = (800, 600);
    let mut viewer = opened_a1(viewport);
    let deep = zoomed(&mut viewer, 2.0);
    let mut held = Oversized::default();
    assert!(held.asked(&deep, viewport));
    let due = held.due(DOCUMENT, |page| placed(&viewer, page), viewport);
    let [(_, window)] = due.as_slice() else {
        panic!("one window: {due:?}");
    };
    let Some((token, rendered)) = held.landed(drawn_red(&deep, *window), *window) else {
        panic!("the first window answers the request");
    };
    viewer
        .handle(Command::RenderReady { token, rendered })
        .for_each(drop);
    let whole = zoomed(&mut viewer, 0.3);
    assert!(!held.asked(&whole, viewport), "drawn whole");
    assert!(
        held.due(DOCUMENT, |page| placed(&viewer, page), viewport)
            .is_empty(),
        "a page drawn whole asks for no window"
    );
    let standing = held.place(Vec::new(), None, DOCUMENT, |page| placed(&viewer, page));
    let [shown] = standing.as_slice() else {
        panic!("the last window stands in for the page drawn whole: {standing:?}");
    };
    assert!(shown.stands_in(), "scaled to the page's new extent");
    held.answered_whole(&whole);
    assert!(
        held.place(Vec::new(), None, DOCUMENT, |page| placed(&viewer, page))
            .is_empty(),
        "and is let go when the page's own frame lands"
    );
}
