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
use viewer_core::{Answer, Command, DocumentId, Event, PageTarget, Query, Rendered, Viewer, Zoom};

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
