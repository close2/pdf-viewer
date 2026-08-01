//! A viewer with no display: commands in, events out, pixels through a worker.
//!
//! This is consumer #2 of `viewer-core`, and it exists to prove the thing the crate's first
//! sentence claims — that the application logic runs without a windowing toolkit. `viewer-ui`
//! is consumer #1 and cannot prove it: a state machine that only ever runs inside a winit
//! event loop is toolkit-free by assertion.
//!
//! The worker here is `render-cpu`, called synchronously. That is the whole of what a host
//! owes: take a [`RenderRequest`], produce pixels, hand them back with the token. A real host
//! does it on another thread, which changes nothing in the protocol — which is the point.

#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::path::{Path, PathBuf};

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Edit, Event, PageTarget, PointerAction, Query, Rendered,
    Selection, Viewer, Zoom,
};

/// A document committed in `doc/`, which every checkout has.
///
/// Not a corpus file: the corpus is an optional submodule, and a test that skipped itself
/// silently would be worse than no test. The PDF Association's note on black point compensation
/// is five pages of A4 and draws text on every one of them.
fn specification_bytes() -> Vec<u8> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    std::fs::read(&path).unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()))
}

/// A corpus document's bytes, or `None` when the submodule is not checked out.
fn corpus_bytes(name: &str) -> Option<Vec<u8>> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// The document every test here opens, unless it says otherwise.
const DOCUMENT: DocumentId = DocumentId(1);

/// How many pages that document has.
const PAGES: usize = 5;

/// `basicapi.pdf`'s second first-page link, `[60.62, 697.08, 141.95, 709.88]` in user space.
///
/// Written as the document states it rather than as a device point, so that
/// [`device_point`] has to do the mapping and the test cannot be satisfied by a mirror of it.
const LINK_RECT: [f32; 4] = [60.62, 697.08, 141.95, 709.88];

/// The centre of a user-space rectangle, in device pixels, from the geometry the viewer reports.
///
/// This is the arithmetic a host does to draw anything over a page, so a test that does it
/// independently is a test of the whole mapping: the magnification, the centring, and the y axis
/// PDF measures up and a raster measures down.
fn device_point(viewer: &Viewer, rect: [f32; 4], page_height: f32) -> (f32, f32) {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    let (x, y) = (
        f32::midpoint(rect[0], rect[2]),
        f32::midpoint(rect[1], rect[3]),
    );
    (
        geometry.origin.0 + x * geometry.scale,
        geometry.origin.1 + (page_height - y) * geometry.scale,
    )
}

/// `basicapi.pdf`'s page height, which the y flip is measured about.
const PAGE_HEIGHT: f32 = 841.89;

/// Opens the specification note into a viewport of the given size, draining the events.
fn opened(width: u32, height: u32) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(width, height, 1.0);
    let events = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: specification_bytes(),
            password: None,
        })
        .collect();
    (viewer, events)
}

/// The render request in these events, where there is exactly one.
fn request(events: &[Event]) -> &viewer_core::RenderRequest {
    let mut found = events.iter().filter_map(|event| match event {
        Event::NeedsRender(request) => Some(request),
        _ => None,
    });
    let request = found.next().expect("a render was asked for");
    assert!(found.next().is_none(), "one render per settled state");
    request
}

/// Plays a host: rasterises the request on the CPU and hands the pixels back.
fn serve(viewer: &mut Viewer, request: &viewer_core::RenderRequest) -> Vec<Event> {
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the CPU backend draws this page");
    viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .collect()
}

#[test]
fn opening_a_document_names_its_pages_and_asks_for_the_first() {
    let (_viewer, events) = opened(800, 1000);
    let opened = events.iter().find_map(|event| match event {
        Event::Opened { document, pages } => Some((*document, *pages)),
        _ => None,
    });
    assert_eq!(opened, Some((DOCUMENT, PAGES)));

    let changed = events.iter().find_map(|event| match event {
        Event::PageChanged { index, of, .. } => Some((*index, *of)),
        _ => None,
    });
    assert_eq!(
        changed,
        Some((0, PAGES)),
        "the first page, and how many there are"
    );
    assert_eq!(request(&events).page, 0);
}

#[test]
fn a_viewport_with_no_extent_renders_nothing_until_it_has_one() {
    // A window before its first layout. Interpreting a page for a zero-pixel raster would be
    // work thrown away, and asking a host to draw into one is a request it cannot satisfy.
    let (mut viewer, events) = opened(0, 0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "nothing to draw into"
    );

    let events: Vec<_> = viewer
        .handle(Command::Resize {
            width: 640,
            height: 480,
            scale: 1.0,
        })
        .collect();
    assert_eq!(
        request(&events).page,
        0,
        "the first layout asks for page one"
    );
}

#[test]
fn a_frame_comes_back_and_the_viewer_hands_it_out_again() {
    let (mut viewer, events) = opened(800, 1000);
    let request = request(&events).clone();
    let (width, height) = (request.target.width, request.target.height);
    let events = serve(&mut viewer, &request);
    assert!(
        events.iter().any(|event| matches!(event, Event::Damage(_))),
        "a frame that arrived is a viewport that changed"
    );

    let Answer::Frame(frame) = viewer.query(Query::Frame) else {
        panic!("the viewer is holding the pixels it was handed");
    };
    assert_eq!(frame.page, 0);
    assert_eq!((frame.raster.width, frame.raster.height), (width, height));
    // Centred horizontally, flush against the top: this page is taller than it is wide, so
    // fitting it leaves slack on one axis only.
    assert!(
        frame.origin.0 > 0.0 && frame.origin.1 == 0.0,
        "{:?}",
        frame.origin
    );

    // And nothing more is asked for, because what is on the screen is what should be.
    let quiet: Vec<_> = viewer
        .handle(Command::Scroll { dx: 0.0, dy: 0.0 })
        .collect();
    assert!(
        !quiet
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "a settled viewer asks for nothing"
    );
}

#[test]
fn a_tier_two_host_is_not_asked_to_draw_the_same_frame_twice() {
    // A host that draws onto its own surface hands back no pixels, so the viewer holds nothing
    // — and if that were also taken to mean "nothing is on the screen", the scheduler would ask
    // for the same frame again the moment it was told the last one was drawn, for ever. What
    // `Rendered::Presented` says is *what is on the screen*, which is a different fact from
    // *what the viewer is holding*.
    let (mut viewer, events) = opened(800, 1000);
    let token = request(&events).token;
    let after: Vec<_> = viewer
        .handle(Command::RenderReady {
            token,
            rendered: Rendered::Presented,
        })
        .collect();
    assert!(
        !after
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "the page that was just drawn is the page that should be drawn: {after:?}"
    );
    assert!(
        matches!(viewer.query(Query::Frame), Answer::None),
        "and the viewer holds no pixels of its own"
    );

    // It is asked again as soon as what should be on the screen changes.
    let turned: Vec<_> = viewer.handle(Command::GoTo(PageTarget::Next)).collect();
    assert_eq!(request(&turned).page, 1);
}

#[test]
fn a_render_answered_after_the_page_turned_is_dropped() {
    // The reason `RenderToken` exists. A page turned while a render was in flight must not be
    // overwritten by the frame the previous page produced.
    let (mut viewer, events) = opened(800, 1000);
    let stale = request(&events).clone();

    let turned: Vec<_> = viewer.handle(Command::GoTo(PageTarget::Next)).collect();
    let fresh = request(&turned).clone();
    assert_eq!(fresh.page, 1);
    assert_ne!(fresh.token, stale.token);

    let late = serve(&mut viewer, &stale);
    assert!(
        late.is_empty(),
        "an answer about page one is not news: {late:?}"
    );
    assert!(
        matches!(viewer.query(Query::Frame), Answer::None),
        "and it is not held either"
    );

    serve(&mut viewer, &fresh);
    let Answer::Frame(frame) = viewer.query(Query::Frame) else {
        panic!("the answer to the outstanding request is kept");
    };
    assert_eq!(frame.page, 1);
}

#[test]
fn zooming_rasterises_again_without_interpreting_again() {
    // The reason the protocol carries a display list and a target separately rather than a
    // finished page: `TargetSpec`'s own documentation calls this the zoom and pan case, and
    // pointer equality of the shared list is what proves the content stream was read once.
    let (mut viewer, events) = opened(800, 1000);
    let first = request(&events).clone();
    serve(&mut viewer, &first);

    let zoomed: Vec<_> = viewer.handle(Command::Zoom(Zoom::In)).collect();
    let second = request(&zoomed).clone();
    assert!(
        std::sync::Arc::ptr_eq(&first.list, &second.list),
        "the same display list, at a new resolution"
    );
    assert!(
        second.target.width > first.target.width,
        "{} > {}",
        second.target.width,
        first.target.width
    );

    // A page turn, on the other hand, has to read a different content stream.
    let turned: Vec<_> = viewer.handle(Command::GoTo(PageTarget::Next)).collect();
    assert!(!std::sync::Arc::ptr_eq(&first.list, &request(&turned).list));
}

#[test]
fn a_page_target_is_clamped_to_the_document() {
    let (mut viewer, _) = opened(800, 1000);
    for (target, expected) in [
        (PageTarget::Last, PAGES - 1),
        (PageTarget::Next, PAGES - 1),
        (PageTarget::Index(9999), PAGES - 1),
        (PageTarget::First, 0),
        (PageTarget::Previous, 0),
        (PageTarget::Relative(-100), 0),
        (PageTarget::Relative(3), 3),
    ] {
        viewer.handle(Command::GoTo(target)).for_each(drop);
        let Answer::Page { index, of, .. } = viewer.query(Query::CurrentPage) else {
            panic!("a document is open");
        };
        assert_eq!((index, of), (expected, PAGES), "{target:?}");
    }
}

#[test]
fn a_document_that_is_not_a_pdf_is_refused_by_name() {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: b"this is not a PDF".to_vec(),
            password: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::OpenFailed { .. })),
        "{events:?}"
    );
    assert!(matches!(viewer.query(Query::PageCount), Answer::None));
}

#[test]
fn an_encrypted_document_asks_for_a_password_and_opens_with_it() {
    // ISO 32000-2 §7.6.4.1: a processor tries the empty user password and then prompts. The
    // prompt is what this program has owed since the twenty-second session, and it is an event
    // rather than a failure — a document that wants a password is not one we cannot read.
    let Some(bytes) = corpus_bytes("issue6010_1.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.clone(),
            password: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::PasswordRequired { .. })),
        "{events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::OpenFailed { .. })),
        "a locked document is not a broken one: {events:?}"
    );

    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: Some("abc".to_owned()),
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "{events:?}"
    );
    let request = request(&events).clone();
    serve(&mut viewer, &request);
    assert!(matches!(viewer.query(Query::Frame), Answer::Frame(_)));
}

#[test]
fn the_page_geometry_maps_the_page_onto_the_screen() {
    let (mut viewer, events) = opened(800, 1000);
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    // A4 at 72 dpi. The scale is what fits it into the viewport's height, and the raster is the
    // page's own extent at that scale.
    assert!(
        (geometry.page.width - 595.0).abs() < 1.0,
        "{:?}",
        geometry.page
    );
    assert!((geometry.scale - 1000.0 / geometry.page.height).abs() < 0.001);
    assert_eq!(geometry.height, 1000);
    assert!(geometry.origin.0 > 0.0, "centred: {:?}", geometry.origin);

    assert!(
        matches!(viewer.query(Query::PageGeometry(1)), Answer::None),
        "a page that is not on the screen has no place on it"
    );
}

#[test]
fn a_page_that_could_not_be_drawn_whole_says_so() {
    // Trap 5's channel, end to end. `issue1155.pdf` is one of the corpus documents whose first
    // page this program cannot draw entirely, and the point of the test is that the host is
    // *told* rather than handed a page that looks finished.
    let Some(bytes) = corpus_bytes("issue1155.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
        })
        .collect();
    let reported: Vec<&String> = events
        .iter()
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(!reported.is_empty(), "{events:?}");
    // And the same sentences are still available to a host that cleared its status bar.
    let Answer::Reports(again) = viewer.query(Query::Reports) else {
        panic!("a page that reported something remembers it");
    };
    assert_eq!(again.len(), reported.len());
}

#[test]
fn closing_the_last_document_leaves_nothing_to_answer_with() {
    let (mut viewer, _) = opened(800, 1000);
    let events: Vec<_> = viewer.handle(Command::Close(DOCUMENT)).collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Closed(DOCUMENT))),
        "{events:?}"
    );
    assert!(matches!(viewer.query(Query::PageCount), Answer::None));
    assert!(matches!(viewer.query(Query::Frame), Answer::None));
}

#[test]
fn a_request_can_be_sent_to_another_thread() {
    // Rule 4 says the core takes no threads of its own, which is only worth anything if the
    // work it hands out can be given to one. Checked by *doing* it rather than by asserting a
    // trait bound: a `Send` bound that compiles proves the type, and this proves the protocol.
    let (mut viewer, events) = opened(800, 1000);
    let sent = request(&events).clone();
    let token = sent.token;
    let raster = std::thread::spawn(move || {
        CpuRasterizer::new()
            .rasterize(&sent.list, sent.target)
            .expect("the CPU backend draws this page")
    })
    .join()
    .expect("the worker thread finished");
    viewer
        .handle(Command::RenderReady {
            token,
            rendered: Rendered::Raster(raster),
        })
        .for_each(drop);
    assert!(matches!(viewer.query(Query::Frame), Answer::Frame(_)));
}

#[test]
fn a_click_on_a_link_shows_the_page_it_names() {
    // §12.5.6.5's link, §12.5.2's coordinate space and §12.3.2's destination, end to end from a
    // point in a window. `basicapi.pdf`'s first page carries a link into its third.
    let Some(bytes) = corpus_bytes("basicapi.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
        })
        .for_each(drop);
    let on_link = device_point(&viewer, LINK_RECT, PAGE_HEIGHT);
    assert!(
        matches!(viewer.query(Query::LinkAt(on_link)), Answer::Link(true)),
        "a host asks this on every pointer move, to choose a cursor: {on_link:?}"
    );
    assert!(matches!(
        viewer.query(Query::LinkAt((5.0, 5.0))),
        Answer::Link(false)
    ));
    // The same point mirrored about the middle of the page, which is where a click landed for
    // the seventy-five sessions this mapping had the y axis upside down. Nothing is there.
    let mirrored = (on_link.0, 1000.0 - on_link.1);
    assert!(
        matches!(viewer.query(Query::LinkAt(mirrored)), Answer::Link(false)),
        "the mirror of a link is not a link: {mirrored:?}"
    );

    viewer
        .handle(Command::Pointer {
            at: on_link,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at: on_link,
            action: PointerAction::Released,
        })
        .collect();
    let changed = events.iter().find_map(|event| match event {
        Event::PageChanged { index, section, .. } => Some((*index, section.clone())),
        _ => None,
    });
    assert_eq!(changed, Some((2, Some("Paragraph 1.1".to_owned()))));
    assert_eq!(request(&events).page, 2, "and the page it named is drawn");
}

#[test]
fn a_press_dragged_off_a_link_does_not_activate_it() {
    // The clause states no rule for this — §12.5.5 describes appearances, not activation — so it
    // is a choice, and it is the one every pointing interface makes.
    let Some(bytes) = corpus_bytes("basicapi.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
        })
        .for_each(drop);
    let on_link = device_point(&viewer, LINK_RECT, PAGE_HEIGHT);
    viewer
        .handle(Command::Pointer {
            at: on_link,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at: (5.0, 5.0),
            action: PointerAction::Released,
        })
        .collect();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::PageChanged { .. })),
        "{events:?}"
    );
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("a document is open");
    };
    assert_eq!(index, 0);
}

#[test]
fn a_uri_is_handed_over_rather_than_opened() {
    // §12.6.4.8. The string is one the *document* controls, and handing it to a browser is a
    // decision about this machine — so it leaves as an event and the host decides.
    let Some(bytes) = corpus_bytes("TAMReview.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
        })
        .for_each(drop);
    // §12.6.4.8's link on `TAMReview.pdf`'s first page, as the document states it.
    let at = device_point(&viewer, [134.0, 331.0, 449.2, 343.0], 842.0);
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Released,
        })
        .collect();
    let uri = events.iter().find_map(|event| match event {
        Event::OpenUri { uri, .. } => Some(uri.clone()),
        _ => None,
    });
    assert_eq!(
        uri.as_deref(),
        Some("http://creativecommons.org/licenses/by-nc-nd/3.0/")
    );
}

#[test]
fn a_document_says_what_it_carries_before_a_page_is_drawn() {
    // §12.11's requirements, §12.8's signatures and §7.11.4's embedded files are claims about the
    // *file*, and a person deciding whether to trust what they are looking at needs them before
    // any page is drawn. That is why they arrive with no page number.
    let Some(bytes) = corpus_bytes("attachment.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
        })
        .collect();
    let about: Vec<&String> = events
        .iter()
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(
        about
            .iter()
            .any(|note| note.contains("carries an embedded file")),
        "this document carries one: {about:?}"
    );
}

#[test]
fn a_drag_across_a_line_selects_what_it_crossed() {
    // Selection is not in ISO 32000-2 — the standard says where a glyph is and what character it
    // stands for, and the rest is a choice. What can be asserted is that the choice is coherent:
    // dragging across text selects that text, the shapes come back in device pixels over it, and
    // dragging further selects more.
    let (mut viewer, events) = opened(800, 1000);
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    // The first line of the specification note's first page, found through the geometry the
    // viewer reports rather than by scanning for it.
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    let line = |from: f32, to: f32, y: f32| {
        (
            (
                geometry.origin.0 + from * geometry.scale,
                geometry.origin.1 + y * geometry.scale,
            ),
            (
                geometry.origin.0 + to * geometry.scale,
                geometry.origin.1 + y * geometry.scale,
            ),
        )
    };
    // A band a fifth of the way down the page, from a quarter to three quarters across.
    let (start, end) = line(
        geometry.page.width * 0.25,
        geometry.page.width * 0.75,
        geometry.page.height * 0.2,
    );

    viewer
        .handle(Command::Pointer {
            at: start,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    assert!(
        matches!(viewer.query(Query::Selection), Answer::Selected(selection) if selection.text.is_empty()),
        "a press selects nothing until it is dragged"
    );

    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at: end,
            action: PointerAction::Dragged,
        })
        .collect();
    assert!(
        events.iter().any(|event| matches!(event, Event::Damage(_))),
        "a selection that changed is a viewport that changed"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "and *not* a page that has to be drawn again: {events:?}"
    );

    let Answer::Selected(selection) = viewer.query(Query::Selection) else {
        panic!("something is selected");
    };
    assert!(
        selection.text.len() > 4,
        "half a line of text: {:?}",
        selection.text
    );
    assert!(!selection.quads.is_empty());
    // The shapes are in device pixels, over the text they cover, between the two points.
    for quad in &selection.quads {
        for corner in quad.chunks_exact(2) {
            assert!(
                (0.0..=800.0).contains(&corner[0]) && (0.0..=1000.0).contains(&corner[1]),
                "{quad:?} is off an 800x1000 viewport"
            );
        }
        assert!(quad[1] > quad[7], "y grows downward on a screen: {quad:?}");
    }
    let widest = selection
        .quads
        .iter()
        .map(|quad| quad[2] - quad[0])
        .fold(0.0_f32, f32::max);
    assert!(
        widest <= end.0 - start.0 + 1.0,
        "no wider than the drag: {widest}"
    );
}

#[test]
fn selecting_everything_is_the_readback_and_clearing_it_is_nothing() {
    let (mut viewer, events) = opened(800, 1000);
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    let Answer::Selected(selection) = viewer.query(Query::Selection) else {
        panic!("everything is selected");
    };
    let Answer::Reports(_) = viewer.query(Query::Reports) else {
        panic!("this page draws completely");
    };
    // The document's title, as this page reads back: §9.3's spacing heuristics put a space
    // inside "Black" on this page, which is the text gate's own subject and not this test's.
    assert!(
        selection.text.contains("Compensation"),
        "{:?}",
        &selection.text[..120.min(selection.text.len())]
    );
    assert!(
        selection.quads.len() > 10,
        "one shape per run of a line, not one per glyph: {}",
        selection.quads.len()
    );

    viewer
        .handle(Command::Select(Selection::None))
        .for_each(drop);
    assert!(matches!(viewer.query(Query::Selection), Answer::None));
}

#[test]
fn turning_the_page_forgets_what_was_selected() {
    // The selection is a range of *this page's* readback, so carrying it across a page turn
    // would leave it pointing into text that is no longer there.
    let (mut viewer, events) = opened(800, 1000);
    let request = request(&events).clone();
    serve(&mut viewer, &request);
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    assert!(matches!(
        viewer.query(Query::Selection),
        Answer::Selected(_)
    ));
    viewer
        .handle(Command::GoTo(PageTarget::Next))
        .for_each(drop);
    assert!(matches!(viewer.query(Query::Selection), Answer::None));
}

#[test]
fn a_field_is_typed_into_undone_and_redone() {
    // §12.7.4's value, changed by a person rather than by the file or by an action. The document
    // is never touched — `CLAUDE.md`'s rule 1 — so what happens is an entry in a log beside it,
    // and undo is that log's cursor moving rather than an inverse being applied.
    let Some(bytes) = corpus_bytes("form_two_pages.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
        })
        .collect();
    let first = request(&events).clone();
    assert!(matches!(viewer.query(Query::Dirty), Answer::Dirty(false)));

    // A field of this form, by the fully qualified name §12.7.4.2 gives it.
    let field = "Text1";
    let events: Vec<_> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: field.to_owned(),
            value: Some("Ada Lovelace".to_owned()),
        }))
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Dirty { dirty: true, .. })),
        "{events:?}"
    );
    // The page has to be interpreted again, because a value is ink — **and asked for again**,
    // even though the page and the resolution have not changed. A scheduler that compared only
    // those two would have left the old picture on the screen; `Open::revision` is what says the
    // display list is a different one.
    let after = request(&events).clone();
    assert!(
        !std::sync::Arc::ptr_eq(&first.list, &after.list),
        "an edited value is a page that has to be drawn again"
    );
    assert_eq!((after.page, after.target), (first.page, first.target));
    assert_ne!(after.token, first.token);
    assert!(matches!(viewer.query(Query::Dirty), Answer::Dirty(true)));

    let events: Vec<_> = viewer.handle(Command::Undo).collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Dirty { dirty: false, .. })),
        "{events:?}"
    );
    assert!(matches!(viewer.query(Query::Dirty), Answer::Dirty(false)));

    let events: Vec<_> = viewer.handle(Command::Redo).collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Dirty { dirty: true, .. })),
        "{events:?}"
    );

    // And an undo past the beginning, or a redo past the end, changes nothing.
    viewer.handle(Command::Undo).for_each(drop);
    let quiet: Vec<_> = viewer.handle(Command::Undo).collect();
    assert!(quiet.is_empty(), "{quiet:?}");
    viewer.handle(Command::Redo).for_each(drop);
    let quiet: Vec<_> = viewer.handle(Command::Redo).collect();
    assert!(quiet.is_empty(), "{quiet:?}");
}

#[test]
fn a_click_finds_the_field_it_landed_on() {
    // What a host asks before it can send an edit: §12.5.2 puts a widget's rectangle in default
    // user space and §12.7.4.2 gives its field a name, and this is the two together from a point
    // in a window.
    let Some(bytes) = corpus_bytes("form_two_pages.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
        })
        .for_each(drop);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page has a geometry");
    };
    // This form's first widget, `[48.54, 727.93, 198.54, 749.93]` in default user space.
    let at = (
        geometry.origin.0 + 120.0 * geometry.scale,
        geometry.origin.1 + (geometry.page.height - 738.0) * geometry.scale,
    );
    let Answer::Field(name) = viewer.query(Query::FieldAt(at)) else {
        panic!("a field is there");
    };
    assert_eq!(name, "Text1");
    // And nothing is at the very corner of the page.
    assert!(matches!(
        viewer.query(Query::FieldAt((2.0, 2.0))),
        Answer::None
    ));
}

#[test]
fn a_saved_document_carries_the_edit_and_the_file_under_it() {
    // §7.5.6's incremental update, end to end from a keystroke: what a person typed comes back
    // out of the *saved* bytes, read by a viewer that has never seen the edit — and the original
    // file is still there underneath, which is the clause's whole point.
    let Some(bytes) = corpus_bytes("form_two_pages.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.clone(),
            password: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Edit(Edit::SetField {
            field: "Text1".to_owned(),
            value: Some("Ada Lovelace".to_owned()),
        }))
        .for_each(drop);

    let events: Vec<_> = viewer.handle(Command::Save).collect();
    let saved = events
        .iter()
        .find_map(|event| match event {
            Event::Saved { bytes, .. } => Some(bytes.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{events:?}"));
    assert!(
        saved.starts_with(&bytes),
        "§7.5.6 appends, leaving the original contents intact"
    );

    // A second viewer, which knows nothing of the edit, opens the saved bytes and draws the
    // value — which is the only statement about a save worth making.
    let mut reader = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = reader
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: saved,
            password: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "{events:?}"
    );
    let request = request(&events).clone();
    serve(&mut reader, &request);
    reader
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    let Answer::Selected(selection) = reader.query(Query::Selection) else {
        panic!("the reopened page has text on it");
    };
    assert!(
        selection.text.contains("Ada Lovelace"),
        "the saved value is drawn: {:?}",
        selection.text
    );
}

#[test]
fn a_search_finds_every_occurrence_and_hands_back_its_shapes() {
    // The text layer's third consumer, and the cheapest: selection built the geometry, and a
    // search is a range of the same readback turned into the same shapes.
    let (mut viewer, events) = opened(800, 1000);
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    let Answer::Found(found) = viewer.query(Query::Find("compensation")) else {
        panic!("a search always answers");
    };
    assert!(
        !found.is_empty(),
        "the document is a note about black point compensation"
    );
    // Case-insensitively: the page states it capitalised.
    let Answer::Found(exact) = viewer.query(Query::Find("Compensation")) else {
        panic!("a search always answers");
    };
    assert_eq!(exact.len(), found.len());

    for occurrence in &found {
        assert!(!occurrence.is_empty(), "each match has shapes");
        for quad in occurrence {
            for corner in quad.chunks_exact(2) {
                assert!(
                    (0.0..=800.0).contains(&corner[0]) && (0.0..=1000.0).contains(&corner[1]),
                    "{quad:?} is off an 800x1000 viewport"
                );
            }
        }
    }

    // A string the page does not have, and the empty needle, both answer with nothing rather
    // than with everything.
    let Answer::Found(none) = viewer.query(Query::Find("zzzzz")) else {
        panic!("a search always answers");
    };
    assert!(none.is_empty());
    let Answer::Found(none) = viewer.query(Query::Find("")) else {
        panic!("a search always answers");
    };
    assert!(none.is_empty());
}

/// A question about the page on the screen does not walk the page tree.
///
/// `Pages::get` is a walk from the root, and on ISO 32000-2's thousandth page it is milliseconds.
/// A host asks for the geometry on every frame and hit-tests a link on every pointer move, so a
/// walk inside either is milliseconds *per mouse move* — which is what this program did on a
/// large document until the hundred-and-forty-first session (ADR 0124).
///
/// Stated as a ratio against a walk this test performs itself, for the reason
/// `an_outline_resolves_against_the_page_tree_once` gives: the absolute number is the machine's
/// and the shape is the code's.
#[test]
fn a_query_about_the_page_on_the_screen_costs_less_than_finding_it() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("the specification is committed in doc/");
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.clone(),
            password: None,
        })
        .for_each(drop);
    let page = 900;
    viewer
        .handle(Command::GoTo(PageTarget::Index(page)))
        .for_each(drop);

    let document = pdf_syntax::Document::open(bytes).expect("it opens");
    let start = std::time::Instant::now();
    let found = pdf_model::Pages::new(&document).get(page);
    let one_walk = start.elapsed();
    assert!(found.is_some(), "the page is there to be found");

    let start = std::time::Instant::now();
    for _ in 0..10 {
        assert!(matches!(
            viewer.query(Query::PageGeometry(page)),
            Answer::Geometry(_)
        ));
        assert!(matches!(
            viewer.query(Query::LinkAt((400.0, 400.0))),
            Answer::Link(_)
        ));
    }
    let twenty_queries = start.elapsed();

    // **Twenty queries, against *one* walk.** The margin is what makes the assertion mean
    // something rather than merely hold: with the page kept, twenty queries are about an eighth
    // of a walk; with it looked up each time they are twenty walks, and the two are two orders of
    // magnitude apart. A bound anywhere between them says the same thing, and this one is the
    // easiest to state.
    assert!(
        twenty_queries < one_walk,
        "twenty queries took {twenty_queries:?} against {one_walk:?} for one walk of the tree"
    );
}

