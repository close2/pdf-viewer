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
    Answer, Command, DocumentId, Edit, Event, FocusMove, PageTarget, PointerAction, Query,
    Rendered, Selection, Viewer, Zoom,
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
            fragment: None,
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

    let zoomed: Vec<_> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::In,
            at: None,
        })
        .collect();
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
            fragment: None,
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
            fragment: None,
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
            fragment: None,
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
            fragment: None,
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
fn a_host_that_draws_its_own_frames_may_zoom_past_the_raster_budget() {
    // `MAX_PIXELS` bounds a raster *this crate hands back*, and a tier-2 host is handed none:
    // it draws the page onto its own surface at window size and keeps nothing of ours. Holding
    // its render request to that budget refused pages that nothing was going to allocate, which
    // is what a person zooming in saw — the viewer said "this page cannot be drawn at this
    // size" about a size no raster of that page was ever going to have.
    //
    // A4 at 40× is 5.7 × 10⁸ pixels: well over the 2²⁸ budget, and 33 676 on its longest side,
    // well under `pdf_render::MAX_EXTENT`. So the case separates the budget on an allocation
    // from the bound on a dimension, and only the first should have moved.
    let (mut viewer, events) = opened(800, 1000);
    let token = request(&events).token;
    viewer
        .handle(Command::RenderReady {
            token,
            rendered: Rendered::Presented,
        })
        .for_each(drop);

    let zoomed: Vec<_> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(40.0),
            at: None,
        })
        .collect();
    let asked = request(&zoomed);
    assert!(
        u64::from(asked.target.width) * u64::from(asked.target.height) > 1 << 28,
        "the case has to be over the budget to be the case: {} x {}",
        asked.target.width,
        asked.target.height
    );
    assert!(
        !zoomed
            .iter()
            .any(|event| matches!(event, Event::Reported { .. })),
        "a host that allocates nothing is told nothing went wrong: {zoomed:?}"
    );

    // A host that takes the pixels is still held to it, because it is still the one allocating
    // them — and is told, rather than handed a page drawn at a scale nobody chose.
    let (mut holding, _) = opened(800, 1000);
    let refused: Vec<_> = holding
        .handle(Command::Zoom {
            zoom: Zoom::Scale(40.0),
            at: None,
        })
        .collect();
    assert!(
        refused
            .iter()
            .any(|event| matches!(event, Event::Reported { .. })),
        "{refused:?}"
    );
    assert!(
        !refused
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "and nothing is asked for that could not be handed back: {refused:?}"
    );
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
            fragment: None,
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
            fragment: None,
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
            fragment: None,
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
            fragment: None,
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
fn a_file_newer_than_this_program_says_so_before_a_page_is_drawn() {
    // Annex I: "[i]f a PDF processor opens a PDF file with a version number newer than the
    // version that it supports … it should warn the user that it is unlikely to be able to read
    // the document successfully". No corpus document can reach this — the newest of the 974
    // states 2.0, which is what this program implements — so the witness is built here, and the
    // pair is the point: the same document one version lower says nothing.
    let about = |header: &str| -> Vec<String> {
        let bytes = format!(
            "{header}\n\
             1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>\nendobj\n\
             trailer\n<< /Root 1 0 R /Size 4 >>\n"
        )
        .into_bytes();
        let mut viewer = Viewer::new(800, 1000, 1.0);
        viewer
            .handle(Command::Open {
                id: DOCUMENT,
                bytes,
                password: None,
                fragment: None,
            })
            .filter_map(|event| match event {
                Event::Reported {
                    page: None, notes, ..
                } => Some(notes),
                _ => None,
            })
            .flatten()
            .collect()
    };

    let newer = about("%PDF-2.1");
    assert!(
        newer.iter().any(|note| note.contains("newer than the 2.0")),
        "a 2.1 file is newer than what this program implements: {newer:?}"
    );
    let current = about("%PDF-2.0");
    assert!(
        !current.iter().any(|note| note.contains("newer than")),
        "2.0 is the version this program implements: {current:?}"
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

    // §14.8.2.5's other order, for the same selection. `Query::Selection` answers in the order
    // the *stream* showed the glyphs, which is what the shapes above are in and the wrong answer
    // to give a person pressing copy on a page whose producer wrote its columns out of order.
    //
    // The assertion is the invariant rather than a string: what comes back is a rearrangement of
    // exactly the characters `Query::Selection` gave, and on this document — whose two orders
    // coincide, which §14.8.2.5.1 says they *should* — it is the same string in the same order.
    // A page where it is not is `pdf-model`'s `the_logical_order_reorders_what_the_stream_showed`,
    // because five corpus pages disagree about order and none of them on purpose.
    //
    // Asserted rather than tolerated: this fixture *is* tagged and its tree does reach the drag,
    // so an `Answer::None` here would be §14.7 having stopped being read rather than a document
    // that states no order — and a match arm accepting both is where that regression would go to
    // be ignored.
    let Answer::LogicalSelection(logical) = viewer.query(Query::LogicalSelection) else {
        panic!("this document is tagged and the tree reaches the whole drag");
    };
    let mut ours: Vec<char> = logical.chars().collect();
    let mut theirs: Vec<char> = selection.text.chars().collect();
    ours.sort_unstable();
    theirs.sort_unstable();
    assert_eq!(ours, theirs, "the same characters, whatever the order");
}

/// §12.5.1's tab key: the focus walks the page's annotations and wraps.
///
/// > Interactive PDF processors may permit the user to navigate through the annotations on a page
/// > by using the keyboard (in particular, the tab key).
///
/// The clause names a key this crate does not have, so what crosses is the *direction*; the order
/// is the document's (Table 31's `/Tabs`, `pdf_model::tab_order`) and the wrap is this crate's
/// choice. What is asserted here is the part `pdf-model`'s own fixture cannot see: that the focus
/// is state the viewer keeps, that moving it is a change to the *viewport* and not to the page —
/// a focus ring is chrome the host draws — and that a document with no annotations is unmoved.
#[test]
fn the_tab_key_walks_the_pages_annotations() {
    let Some(bytes) = corpus_bytes("160F-2019.pdf") else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .collect();
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    // A tab with nothing focused takes the first annotation, and it is a viewport change rather
    // than a render: nothing about the page's own marks moved.
    let events: Vec<_> = viewer.handle(Command::Focused(FocusMove::Next)).collect();
    assert!(
        events.iter().any(|event| matches!(event, Event::Damage(_))),
        "a focus ring is chrome, so the viewport changed: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "and the page did not have to be drawn again: {events:?}"
    );

    // Walking forward and back returns to where it started, whatever the order is — the property
    // that holds for all five of Table 31's values and needs none of them named.
    let steps = 5;
    for _ in 0..steps {
        viewer
            .handle(Command::Focused(FocusMove::Next))
            .for_each(drop);
    }
    for _ in 0..steps {
        viewer
            .handle(Command::Focused(FocusMove::Previous))
            .for_each(drop);
    }
    let after = viewer.handle(Command::Focused(FocusMove::Next)).count();
    assert!(
        after > 0,
        "the walk came back to a position the next tab can move off"
    );

    // And where it is on the screen, which is what a host draws a ring from. Device pixels of
    // the viewport, like every other shape this crate answers with — a host that computed them
    // from a `/Rect` itself would be re-deriving the origin, the magnification and the y flip,
    // which is the arithmetic ADR 0118 found wrong for seventy-five sessions.
    let Answer::Focus { quad, .. } = viewer.query(Query::Focus) else {
        panic!("something is focused, and it has a /Rect");
    };
    for corner in quad.chunks_exact(2) {
        assert!(
            (0.0..=800.0).contains(&corner[0]) && (0.0..=1000.0).contains(&corner[1]),
            "{quad:?} is off an 800x1000 viewport"
        );
    }
    assert!(quad[1] < quad[7], "y grows downward on a screen: {quad:?}");

    // And clearing it is a move like any other, which a press outside every annotation already
    // does — so a host binding Escape to it needs no second message.
    viewer
        .handle(Command::Focused(FocusMove::None))
        .for_each(drop);
    assert_eq!(
        viewer.handle(Command::Focused(FocusMove::None)).count(),
        0,
        "clearing a cleared focus changed nothing, so it said nothing"
    );
    assert!(
        matches!(viewer.query(Query::Focus), Answer::None),
        "and nothing focused is nothing to draw a ring round"
    );
}

/// §14.8.2.5 answers nothing when there is nothing selected, on any document.
#[test]
fn the_logical_order_of_no_selection_is_no_answer() {
    let (mut viewer, events) = opened(800, 1000);
    let request = request(&events).clone();
    serve(&mut viewer, &request);
    assert!(matches!(
        viewer.query(Query::LogicalSelection),
        Answer::None
    ));

    // And a viewer with no document at all, which is the answer every other query gives.
    let empty = Viewer::new(800, 1000, 1.0);
    assert!(matches!(empty.query(Query::LogicalSelection), Answer::None));
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
            fragment: None,
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

/// Where the caret goes, in the pixels a host draws it in.
///
/// **The standard states no caret** — §12.5.6.11's caret *annotation* is a different object, and
/// nothing in ISO 32000-2 describes a text cursor at all — so what this pins is the relation that
/// makes one mean anything to a person: it stands where the next character will be drawn, which
/// §12.7.4.3's layout is what knows. The host keeps the point it clicked and an offset into the
/// value, exactly as ADR 0201 has it keep the point and not the text; everything else is derived
/// here, because a host deriving it would be re-deriving the magnification, the centring and the
/// y flip. ADR 0211.
#[test]
fn a_caret_says_where_the_next_character_goes() {
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
            fragment: None,
        })
        .for_each(drop);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page has a geometry");
    };
    // This form's first widget, `[48.54, 727.93, 198.54, 749.93]` in default user space — the
    // same one `a_click_finds_the_field_it_landed_on` presses, written from the document.
    let rect = [48.54_f32, 727.93, 198.54, 749.93];
    let at = (
        geometry.origin.0 + 120.0 * geometry.scale,
        geometry.origin.1 + (geometry.page.height - 738.0) * geometry.scale,
    );
    let caret = |viewer: &Viewer, offset: usize| match viewer.query(Query::Caret { at, offset }) {
        Answer::Caret { from, to } => Some((from, to)),
        _ => None,
    };

    // An empty field still has a caret: it is where the first character will land, and a person
    // clicking into an untouched field is shown exactly that.
    let (from, to) = caret(&viewer, 0).expect("an empty text field has somewhere to type");
    assert!(
        (from.0 - to.0).abs() < 0.001,
        "a caret is a vertical segment on an unturned page: {from:?} {to:?}"
    );
    assert!(
        from.1 > to.1,
        "the descent end is below the ascent end in a raster's downward y: {from:?} {to:?}"
    );
    // Inside the widget it belongs to, mapped through the same geometry the host draws with.
    let left = geometry.origin.0 + rect[0] * geometry.scale;
    let right = geometry.origin.0 + rect[2] * geometry.scale;
    let top = geometry.origin.1 + (geometry.page.height - rect[3]) * geometry.scale;
    let bottom = geometry.origin.1 + (geometry.page.height - rect[1]) * geometry.scale;
    assert!(
        (left..=right).contains(&from.0) && (top..=bottom).contains(&from.1),
        "the caret is inside the widget: {from:?} in {left}..{right} by {top}..{bottom}"
    );
    let empty = from;

    // Typed into, the caret after the value has moved along the line and not off it.
    viewer
        .handle(Command::Edit(Edit::SetField {
            field: "Text1".to_owned(),
            value: Some("Ada".to_owned()),
        }))
        .for_each(drop);
    let (start, _) = caret(&viewer, 0).expect("the field still has a caret");
    let (end, _) = caret(&viewer, 3).expect("and one at the end of what was typed");
    assert!(
        (start.0 - empty.0).abs() < 0.001 && (start.1 - empty.1).abs() < 0.001,
        "the caret before the first character has not moved: {start:?} against {empty:?}"
    );
    assert!(
        end.0 > start.0 && end.0 < right,
        "three characters along the line and still in the box: {end:?}"
    );
    assert!(
        (end.1 - start.1).abs() < 0.001,
        "and on the same line: {start:?} {end:?}"
    );

    // An offset past the end of the value is the end of the value rather than an error: a host
    // whose caret outlived §12.7.5.3's truncation of what it typed asks a question with no answer
    // otherwise.
    let (clamped, _) = caret(&viewer, 4096).expect("an offset past the end still answers");
    assert!(
        (clamped.0 - end.0).abs() < 0.001,
        "clamped to the end of the value: {clamped:?} against {end:?}"
    );

    // And a point on no field has no caret, which is the answer a host uses to decide the
    // keyboard is back on the page.
    assert!(
        matches!(
            viewer.query(Query::Caret {
                at: (2.0, 2.0),
                offset: 0
            }),
            Answer::None
        ),
        "nothing to type into at the corner of the page"
    );
}

/// §8.11.4.3's Table 99 `/ListMode`, which is the one entry of that table whose answer depends
/// on the window rather than on the file.
///
/// > A name specifying which optional content groups in the Order array shall be displayed to
/// > the user. Valid values shall be: AllPages Display all groups in the Order array.
/// > VisiblePages Display only those groups in the Order array that are referenced by one or
/// > more visible pages.
///
/// This window shows one page at a time, so "one or more visible pages" is the page it is
/// showing — the same derivation §12.6.3's `/PV` and `/PO` took in the two-hundred-and-fourth
/// session. The ledger's reason for leaving it unapplied was "which pages are visible is a
/// question about a window this crate does not have", and a window arrived in the
/// hundred-and-thirty-second session.
///
/// `visibility_expressions.pdf` is the only corpus document that states the entry, on a scan of
/// every uncompressed `/ListMode` in all 974. It states `VisiblePages`, and its one page reaches
/// all three of its groups through the `/VE` of five membership dictionaries — so what this
/// pins is the direction that costs a person something: **a filter that is applied must not
/// empty a panel the document meant to fill.**
#[test]
fn a_visible_pages_list_mode_keeps_the_groups_the_page_reaches() {
    let Some(bytes) = corpus_bytes("visibility_expressions.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Layers(layers) = viewer.query(Query::Layers) else {
        panic!("this document states an /Order");
    };
    let names: Vec<&str> = layers
        .iter()
        .filter_map(|layer| match layer {
            viewer_core::Layer::Group { name, .. } => name.as_deref(),
            viewer_core::Layer::Collection { .. } => None,
        })
        .collect();
    assert_eq!(names, vec!["A", "B", "C"]);
}

/// §12.5.5's appearances belong to every annotation, not only to a link.
///
/// > An annotation may define as many as three separate appearances:
/// >
/// > - The normal appearance shall be used when the annotation is not interacting with the user.
/// >   This appearance is also used for printing the annotation.
/// > - The rollover appearance shall be used when the user moves the cursor into the annotation's
/// >   active area without pressing the mouse button.
/// > - The down appearance shall be used when the mouse button is pressed or held down within the
/// >   annotation's active area.
///
/// `pdf_model` has answered that for every subtype since the hundred-and-thirty-eighth session,
/// including §12.5.6.19's `/H` highlighting mode whose default is `I` — and until the
/// two-hundred-and-fifty-third this crate took the annotation under the pointer from
/// `link::at`, which returns a `/Subtype /Link` and nothing else. So the one entry that is a
/// *widget's* could not be reached by any host, and neither could a rollover on anything else.
///
/// The observable is the display list: an appearance that changed is a page interpreted again.
#[test]
fn a_press_on_a_widget_draws_the_page_again() {
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
            fragment: None,
        })
        .collect();
    let first = request(&events).clone();
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page has a geometry");
    };
    // This form's first widget, `[48.54, 727.93, 198.54, 749.93]` in default user space — the
    // same one `a_click_finds_the_field_it_landed_on` addresses, and no link is anywhere near it.
    let on_widget = (
        geometry.origin.0 + 120.0 * geometry.scale,
        geometry.origin.1 + (geometry.page.height - 738.0) * geometry.scale,
    );
    assert!(
        matches!(viewer.query(Query::LinkAt(on_widget)), Answer::Link(false)),
        "the point is a widget and not a link, which is the whole of the case"
    );
    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at: on_widget,
            action: PointerAction::Pressed,
        })
        .collect();
    let pressed = request(&events).clone();
    assert!(
        !std::sync::Arc::ptr_eq(&first.list, &pressed.list),
        "a press inside a widget's active area shows Table 170's down appearance"
    );

    // And a release puts it back: the button is up, so §12.5.5's rollover applies, and this
    // widget states no `/R` — so the picture is the normal one again.
    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at: on_widget,
            action: PointerAction::Released,
        })
        .collect();
    let released = request(&events).clone();
    assert!(!std::sync::Arc::ptr_eq(&pressed.list, &released.list));

    // Nothing is under the corner of the page, and a pointer over nothing draws nothing again.
    let quiet: Vec<_> = viewer
        .handle(Command::Pointer {
            at: (2.0, 2.0),
            action: PointerAction::Moved,
        })
        .collect();
    assert!(
        !quiet
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "{quiet:?}"
    );
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
            fragment: None,
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
    let Answer::Field { name, value } = viewer.query(Query::FieldAt(at)) else {
        panic!("a field is there");
    };
    assert_eq!(name.qualified, "Text1");
    // An empty text field answers with an empty string rather than with nothing: `None` is
    // reserved for a field whose value is not text at all, and a host deciding where to send the
    // keyboard needs those to be two answers. 147 of the corpus's first-page widgets are this.
    assert_eq!(value.as_deref(), Some(""));
    // This form states no `/TU`, so §14.9.3's alternative is absent and the name a user
    // interface shows is the field's own — which is the case the clause's "if present" covers.
    assert_eq!(name.alternative, None);
    assert_eq!(name.shown(), "Text1");
    // And nothing is at the very corner of the page.
    assert!(matches!(
        viewer.query(Query::FieldAt((2.0, 2.0))),
        Answer::None
    ));
}

/// §14.9.3's alternative field name, which a user interface is told to show in place of the real
/// one.
///
/// > An alternative name may be specified for an interactive form field (see 12.7, "Forms")
/// > which, if present, shall be used in place of the actual field name when an interactive PDF
/// > processor identifies the field in a user-interface. This alternative name, if provided,
/// > shall be specified using the TU entry of the field dictionary.
///
/// A `shall` addressed to a processor with a user interface, and this became one in the
/// hundred-and-thirty-second session. The ledger's row said `/TU` "names a field in a user
/// interface this program does not have" — and what made the clause unreachable was not the
/// window but the *answer*: one string cannot carry both a field's identity and its label, so a
/// host had nothing to obey the clause with. ADR 0167.
///
/// `issue17492.pdf`'s first widget is §12.5.6.19's merged dictionary — field and annotation in
/// one — stating `/T (firstName)` and a `/TU` in UTF-16BE, which is also §7.9.2.2's other
/// encoding taken through the same path.
#[test]
fn a_field_states_the_name_a_user_interface_is_to_show() {
    let Some(bytes) = corpus_bytes("issue17492.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page has a geometry");
    };
    // The middle of `/Rect [165.7 673.9 315.7 688.1]`, taken from the document rather than from
    // the code under test — trap 12a's rule.
    let at = (
        geometry.origin.0 + 240.7 * geometry.scale,
        geometry.origin.1 + (geometry.page.height - 681.0) * geometry.scale,
    );
    let Answer::Field { name, .. } = viewer.query(Query::FieldAt(at)) else {
        panic!("a field is there");
    };
    assert_eq!(
        name.qualified, "firstName",
        "the name that addresses the field is §12.7.4.2's, unchanged"
    );
    assert_eq!(
        name.shown(),
        "First name",
        "and the name shown to a person is Table 226's /TU"
    );
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
            fragment: None,
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

    // §12.7.4.3's appearance stream is *written*, and this is what says so. Table 224's
    // `/NeedAppearances` is the alternative — a flag asking the next reader to do the work — and
    // a saved file that does not set it has nothing but the widget's own stream to show the
    // value with. Without this assertion the readback below passes either way, because this
    // program honours the flag it would have set.
    let written = pdf_syntax::Document::open(saved.clone()).expect("the saved file opens");
    let catalog = written.catalog().expect("a /Root");
    let form = written.get_key(&catalog, "AcroForm");
    let flag = form
        .as_dict()
        .map(|form| written.get_key(form, "NeedAppearances"));
    assert!(
        !matches!(flag, Some(pdf_syntax::Object::Boolean(true))),
        "every changed widget got its own appearance stream, so nothing is owed to the reader"
    );

    // A second viewer, which knows nothing of the edit, opens the saved bytes and draws the
    // value — which, with the flag unset above, it can only be reading out of the stream.
    let mut reader = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = reader
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: saved,
            password: None,
            fragment: None,
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
            fragment: None,
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

#[test]
fn a_tagged_page_answers_with_its_structure_and_an_untagged_one_says_so() {
    // §14.7's structure tree, reaching a consumer for the first time. `pdf-model` has read it
    // since the seventy-eighth session and §14.9's entries since the sixtieth; until the
    // hundred-and-forty-ninth nothing in this program handed either to anybody.
    let (mut viewer, events) = opened(800, 1000);
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    let Answer::Accessibility(nodes) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    assert!(
        !nodes.is_empty(),
        "the specification's own PDF is tagged; §14.7 should have something to say about page one"
    );

    // Parent-first, which is what makes an index into this list a usable parent link: a node's
    // parent is always already there when a host reaches it.
    for (index, node) in nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            assert!(parent < index, "node {index} names a later parent {parent}");
        }
    }

    // Something on the page has both a role and text, and the shapes for it are in the same
    // device pixels a selection is answered in — which is the property that lets a host draw a
    // focus ring without a second mapping.
    let named = nodes
        .iter()
        .find(|node| !node.name.trim().is_empty() && !node.quads.is_empty())
        .unwrap_or_else(|| panic!("no node carries text and a place: {nodes:?}"));
    assert!(!named.role.is_empty(), "{named:?}");
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page showing has a geometry");
    };
    for quad in &named.quads {
        for corner in quad.chunks_exact(2) {
            assert!(
                corner[0] >= geometry.origin.0 - 1.0
                    && corner[0] <= geometry.origin.0 + geometry.page.width * geometry.scale + 1.0,
                "a node's quad is off the page: {quad:?} against {geometry:?}"
            );
        }
    }

    // And an untagged document answers with an empty list rather than failing. 885 of the
    // corpus's 974 are untagged, and "this page says nothing about its own structure" is an
    // answer §14.7 leaves a producer free to give.
    let Some(bytes) = corpus_bytes("basicapi.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut plain = Viewer::new(800, 1000, 1.0);
    plain
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    assert!(matches!(
        plain.query(Query::AccessibilityTree),
        Answer::Accessibility(nodes) if nodes.is_empty()
    ));
}

#[test]
fn a_page_stating_a_duration_advances_when_it_is_told_the_time() {
    // §12.4.4.1's `/Dur`, and rule 3: this crate has no clock, so the only way it learns that a
    // second went by is `Command::Tick`. A host reading a document sends none and nothing
    // advances, which is why "is a presentation running" is not a state this crate keeps.
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_durations(),
            password: None,
            fragment: None,
        })
        .for_each(drop);

    // Short of the duration, nothing happens — a maximum, not a schedule.
    let quiet: Vec<_> = viewer.handle(Command::Tick { millis: 900 }).collect();
    assert!(
        !quiet
            .iter()
            .any(|event| matches!(event, Event::PageChanged { .. })),
        "{quiet:?}"
    );

    let advanced: Vec<_> = viewer.handle(Command::Tick { millis: 200 }).collect();
    let index = advanced.iter().find_map(|event| match event {
        Event::PageChanged { index, .. } => Some(*index),
        _ => None,
    });
    assert_eq!(index, Some(1), "{advanced:?}");
    // §12.4.4.1 makes `/Trans` "the transition style that shall be used when moving to *this*
    // page from another", so what is named is the page arrived at — page two's `Wipe` and not
    // page one's `Split`. Read from the page tree rather than from `Open::current`, which is
    // filled during interpretation and so still holds the page just left.
    let named = advanced.iter().find_map(|event| match event {
        Event::Transition { transition, .. } => Some(transition.style.clone()),
        _ => None,
    });
    assert_eq!(
        named,
        Some(pdf_model::navigation::Style::Wipe),
        "{advanced:?}"
    );

    // And the clock restarted with the page: the same 900 ms that did not advance page one does
    // not advance page two either, which is the clause making the duration a property of the
    // page rather than of the presentation.
    let quiet: Vec<_> = viewer.handle(Command::Tick { millis: 900 }).collect();
    assert!(
        !quiet
            .iter()
            .any(|event| matches!(event, Event::PageChanged { .. })),
        "{quiet:?}"
    );

    // The last page states a `/Dur` too and does not advance: §12.4.4 says nothing about what
    // follows the end, and looping is a decision a host can make with a `GoTo` and this crate
    // cannot unmake.
    viewer
        .handle(Command::GoTo(PageTarget::Last))
        .for_each(drop);
    let ended: Vec<_> = viewer.handle(Command::Tick { millis: 5000 }).collect();
    assert!(
        !ended
            .iter()
            .any(|event| matches!(event, Event::PageChanged { .. })),
        "{ended:?}"
    );
}

/// §12.4.3's article thread, followed to a bead rather than to the page a bead is on.
///
/// Table 163's `/R` is
///
/// > A rectangle specifying the location of this bead on the page in default user space
///
/// and §12.4.3 says why a reader would want it: the beads "are connected in sequence" so that a
/// reader "can follow a thread from one bead to the next". A jump that turned to the bead's page
/// and stopped there has not followed anything — a bead is a column of a magazine layout, and the
/// page it sits on may hold four of them.
///
/// So the jump composes Table 149's `/FitR`, which states the same thing about a window:
/// "[d]isplay the page … with its contents magnified just enough to fit the rectangle specified
/// by the coordinates left, bottom, right, and top entirely within the window". The ledger's
/// reason for leaving this undone was "`viewer-ui` fits whole pages", which stopped being true in
/// the hundred-and-thirty-second session and stopped being true of destinations in the
/// two-hundred-and-first (ADR 0162).
///
/// **No corpus document states an article** — `pdf-model/tests/articles.rs` is a ratchet on that
/// number — so the fixture is built from the clause, and the assertion is the magnification a
/// 100-unit-wide bead earns in an 800-pixel window rather than a page number nothing distinguishes.
#[test]
fn a_thread_action_shows_the_bead_and_not_merely_its_page() {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_thread(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Geometry(before) = viewer.query(Query::PageGeometry(0)) else {
        panic!("page one has a geometry");
    };
    // A 400 x 500 page in an 800 x 1000 window fits at 2.0, which is where a document with no
    // destination starts.
    assert!((before.scale - 2.0).abs() < 0.01, "{before:?}");

    // The link on page one, whose action is `/S /Thread`. Its `/Rect` is [10 10 90 90].
    let on_link = device_point(&viewer, [10.0, 10.0, 90.0, 90.0], 500.0);
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
        Event::PageChanged { index, .. } => Some(*index),
        _ => None,
    });
    assert_eq!(changed, Some(1), "the thread's first bead is on page two");

    // And the window is on the bead: `/R [100 100 200 300]` is 100 x 200 units, which fits an
    // 800 x 1000 window at 5.0 rather than at the page's own 2.0.
    let Answer::Geometry(after) = viewer.query(Query::PageGeometry(1)) else {
        panic!("page two has a geometry");
    };
    assert!(
        (after.scale - 5.0).abs() < 0.01,
        "the bead is magnified to fit, not the page: {after:?}"
    );
}

/// Two pages, one thread of one bead on the second, and a link on the first that jumps to it.
///
/// Built from §12.4.3 and Table 209, because the corpus states no article at all. The thread's
/// `/F` and the bead's `/N` are the same object: "[i]n the last bead … shall refer to the first
/// bead", and a thread of one bead is its own successor.
fn with_a_thread() -> Vec<u8> {
    use std::fmt::Write as _;
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Threads [6 0 R] >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 500] /Annots [5 0 R] \
         >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 500] /B [7 0 R] >>\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 10 90 90] /F 4 \
         /A << /S /Thread /D 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /Thread /F 7 0 R /I << /Title (one column) >> >>\nendobj\n\
         7 0 obj\n<< /Type /Bead /T 6 0 R /N 7 0 R /V 7 0 R /P 4 0 R \
         /R [100 100 200 300] >>\nendobj\n"
        .to_owned();

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A three-page document whose every page states §12.4.4.1's `/Dur 1`.
fn with_durations() -> Vec<u8> {
    use std::fmt::Write as _;
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 \
         /Trans << /S /Split >> >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 \
         /Trans << /S /Wipe >> >>\nendobj\n\
         5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 >>\nendobj\n"
        .to_owned();

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// §12.3.3's outline activates, and the index it lands on is the page tree's.
///
/// `Command::Activate` exists because a host cannot do this itself twice over: §12.3.2.2's
/// target is a page *object*, and turning one into an index is a walk of the page tree; and
/// Table 151's `/A` may be any of §12.6's types and a whole `/Next` chain of them. So the check
/// walks the tree **here**, by `/Kids`, rather than asking `pdf_model::Pages` — a test that
/// resolved the destination through the same function the command uses would agree with itself.
#[test]
fn an_outline_item_goes_to_the_page_its_destination_names() {
    let document = pdf_syntax::Document::open(specification_bytes()).expect("a committed PDF");
    let pages = page_objects(&document);
    assert_eq!(pages.len(), PAGES, "the tree walk found every page");

    let (mut viewer, _) = opened(800, 1000);
    let Answer::Outline(outline) = viewer.query(Query::Outline) else {
        panic!("the note has a §12.3.3 outline");
    };
    // Flattened, because what a panel shows is the whole tree and what a click sends is one
    // item's object whatever its depth.
    let mut wanted: Vec<(String, pdf_syntax::ObjectId, usize)> = Vec::new();
    let mut stack: Vec<&pdf_model::outline::Item> = outline.items.iter().rev().collect();
    while let Some(item) = stack.pop() {
        if let Some(destination) = item.destination
            && let pdf_model::destination::Target::Object(page) = destination.target
            && let Some(index) = pages.iter().position(|candidate| *candidate == page)
        {
            wanted.push((item.title.clone(), item.id, index));
        }
        stack.extend(item.children.iter().rev());
    }
    assert!(
        wanted.iter().any(|(_, _, index)| *index > 0),
        "every item points at page one, so nothing here would move"
    );

    for (title, id, index) in wanted {
        viewer
            .handle(Command::GoTo(PageTarget::First))
            .for_each(drop);
        viewer.handle(Command::Activate(id)).for_each(drop);
        let Answer::Page { index: at, .. } = viewer.query(Query::CurrentPage) else {
            panic!("a document is open");
        };
        assert_eq!(at, index, "{title:?}");
    }

    // An object that is not an outline item activates nothing and says nothing: it is the host
    // that named the wrong thing, and `Event::Reported` is for what the *document* could not do.
    viewer
        .handle(Command::GoTo(PageTarget::Last))
        .for_each(drop);
    let events: Vec<_> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(999_999, 0)))
        .collect();
    assert!(events.is_empty(), "{events:?}");
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("a document is open");
    };
    assert_eq!(index, PAGES - 1, "nothing moved");
}

/// §12.3.3's other half: an item whose `/A` is a URI hands the URI over.
///
/// The sentence is one sentence — "jump to a destination **or trigger an action** associated
/// with the item" — and until the hundred-and-sixty-eighth session only the jump happened. Seven
/// corpus outline items over three documents carry a `/URI` action; this is one of them, and
/// what it proves is that the action path a *link* takes is the one an outline item takes.
#[test]
fn an_outline_item_whose_action_is_a_uri_hands_it_over() {
    let Some(bytes) = corpus_bytes("issue3214.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Outline(outline) = viewer.query(Query::Outline) else {
        panic!("the fixture has an outline");
    };
    let mut items: Vec<(String, pdf_syntax::ObjectId)> = Vec::new();
    let mut stack: Vec<&pdf_model::outline::Item> = outline.items.iter().rev().collect();
    while let Some(item) = stack.pop() {
        items.push((item.title.clone(), item.id));
        stack.extend(item.children.iter().rev());
    }
    assert!(!items.is_empty(), "the fixture states items");

    let mut opened: Vec<String> = Vec::new();
    for (_, id) in items {
        for event in viewer.handle(Command::Activate(id)) {
            if let Event::OpenUri { uri, .. } = event {
                opened.push(uri);
            }
        }
    }
    assert!(
        !opened.is_empty(),
        "no outline item in this document handed over a URI"
    );
}

/// Every page object of a document, in order, by walking `/Kids` from the catalog.
///
/// Deliberately not `pdf_model::Pages`: see the caller.
fn page_objects(document: &pdf_syntax::Document) -> Vec<pdf_syntax::ObjectId> {
    fn descend(
        document: &pdf_syntax::Document,
        node: &pdf_syntax::Dictionary,
        depth: usize,
        out: &mut Vec<pdf_syntax::ObjectId>,
    ) {
        if depth > 32 {
            return;
        }
        let kids = document.get_key(node, "Kids");
        let Some(kids) = kids.as_array() else {
            return;
        };
        for kid in kids {
            let Some(id) = kid.as_reference() else {
                continue;
            };
            let child = document.get(id);
            let Some(child) = child.as_dict() else {
                continue;
            };
            if child.get("Kids").is_some() {
                descend(document, child, depth.saturating_add(1), out);
            } else {
                out.push(id);
            }
        }
    }
    let catalog = document.catalog().expect("a catalog");
    let root = document.get_key(&catalog, "Pages");
    let mut out = Vec::new();
    if let Some(root) = root.as_dict() {
        descend(document, root, 0, &mut out);
    }
    out
}

/// §7.11.4: an embedded file's bytes come out of the document, decoded.
///
/// The clause is the one part of §7.11 that needs no filesystem — "the bytes are inside the
/// document" — and until the hundred-and-sixty-ninth session this crate listed them and could
/// not hand one over. What the *host* does with them is rule 2's business and not this test's.
///
/// The check is against the file's own content rather than against a length: an extraction that
/// handed back the still-deflated stream would be the right number of nothing.
#[test]
fn an_embedded_file_comes_out_of_the_document() {
    let Some(bytes) = corpus_bytes("attachment.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Attachments(files) = viewer.query(Query::Attachments) else {
        panic!("the fixture embeds a file");
    };
    let [file] = files.as_slice() else {
        panic!("one embedded file, not {}", files.len());
    };
    let (key, claimed) = (file.name.clone(), file.size);

    let events: Vec<_> = viewer.handle(Command::Extract { name: key }).collect();
    let [Event::Extracted { name, bytes, .. }] = events.as_slice() else {
        panic!("one extraction, not {events:?}");
    };
    assert_eq!(name, "foo.txt", "Table 43's own name for the file");
    assert_eq!(
        String::from_utf8_lossy(bytes),
        "bar baz \n",
        "the file itself, with §7.4's filters undone"
    );
    // Table 45's `/Size` is "the size of the uncompressed embedded file, in bytes" — the
    // document's claim, and here is the one place this tree can check one against a measurement.
    assert_eq!(
        claimed.map(usize::try_from),
        Some(Ok(bytes.len())),
        "the document's stated size and the bytes disagree"
    );

    // A name the tree does not hold is reported rather than swallowed: a click that produced
    // nothing and said nothing would be indistinguishable from one that worked.
    let events: Vec<_> = viewer
        .handle(Command::Extract {
            name: "nothing.txt".to_owned(),
        })
        .collect();
    assert!(
        matches!(events.as_slice(), [Event::Reported { .. }]),
        "{events:?}"
    );
}

/// Table 29's `/PageMode` reaches a host, which is what makes three of its six values mean
/// anything.
///
/// The entry names *panels* — `UseOutlines` shows the document outline, `UseOC` the optional
/// content group panel, `UseAttachments` the attachments panel — so until the sidebar of
/// sessions 166 and 167 existed there was nothing for a host to do with it. This checks the
/// answer rather than the panel, because the panel is `viewer-ui`'s and the query is the
/// boundary.
#[test]
fn the_catalog_says_which_panel_a_host_should_open() {
    use pdf_model::viewer_preferences::{PageLayout, PageMode};

    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: specification_bytes(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    // The committed note asks for both: its outline panel, and one continuous column of pages.
    // The second is a layout this window does not have and says so once — a document asking for
    // something and getting silence is trap 5 in an interface.
    let Answer::Opening(opening) = viewer.query(Query::Opening) else {
        panic!("a document is open");
    };
    assert_eq!(
        opening.mode,
        PageMode::UseOutlines,
        "this document asks for the outline panel"
    );
    assert_eq!(opening.layout, PageLayout::OneColumn);

    // A document stating neither gets Table 29's own defaults rather than an absence.
    let mut plain = Viewer::new(800, 1000, 1.0);
    plain
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_durations(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Opening(opening) = plain.query(Query::Opening) else {
        panic!("a document is open");
    };
    assert_eq!(
        (opening.mode, opening.layout),
        (PageMode::UseNone, PageLayout::SinglePage)
    );

    // And nothing is answered for a viewer with no document, which is the same answer every
    // other query gives.
    let empty = Viewer::new(800, 1000, 1.0);
    assert!(matches!(empty.query(Query::Opening), Answer::None));
}

/// §14.3.3's `/Info` reaches a host, and since the two-hundred-and-ninety-fourth so does
/// §14.3.2's XMP.
///
/// The second half is the point. Table 349's every text entry carries a NOTE naming an XMP
/// counterpart and §12.2's `/DisplayDocTitle` names `dc:title` outright, so a host titling a
/// window needs the *stream's* answer and not the dictionary's — which is what this query now
/// carries. **The variant changed shape rather than gaining a message** (`metadata_stream: bool`
/// became the packet itself), and nothing in this vocabulary being `#[non_exhaustive]` is what
/// made every consumer fail to compile until it was read. ADR 0186.
#[test]
fn a_document_hands_over_what_it_says_about_itself() {
    let (viewer, _) = opened(800, 1000);
    let Answer::Properties {
        information,
        metadata,
    } = viewer.query(Query::Properties)
    else {
        panic!("a document is open");
    };
    assert_eq!(
        information.producer.as_deref(),
        Some("Adobe PDF Library 15.0"),
        "Table 349's /Producer, as the committed note states it"
    );
    assert_eq!(
        information.created_date().map(|date| date.year),
        Some(2018),
        "§7.9.4's date parses out of what the file wrote"
    );
    let xmp = metadata
        .expect("this document carries §14.3.2's stream")
        .expect("and it is well-formed XMP");
    assert_eq!(
        xmp.producer(),
        information.producer.as_deref(),
        "Table 349's NOTE pairs /Producer with pdf:Producer, and this file states both"
    );
    assert!(
        xmp.title().is_some(),
        "and a dc:title, which is what §12.2's /DisplayDocTitle names"
    );

    // A document that states nothing says nothing, which is 454 of the 964 corpus documents that
    // open — an absent `/Info` is what §14.3.3 calls optional, not a failure.
    let mut plain = Viewer::new(800, 1000, 1.0);
    plain
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_durations(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Properties {
        information,
        metadata,
    } = plain.query(Query::Properties)
    else {
        panic!("a document is open");
    };
    assert!(information.is_empty());
    assert!(
        metadata.is_none(),
        "no /Metadata is `None`, which is not the same answer as a stream that would not read"
    );
}

/// §12.6.3: the pointer raises Table 197's events, which is the half this crate could not do.
///
/// The clause's data and its execution have been here since the seventy-seventh session —
/// `action::for_annotation` reads the table and `ViewState::perform_all` performs it — and the
/// row said `partial` because "[n]othing raises an event: entering, pressing and focusing are a
/// window's business and this crate has no events". `Command::Pointer` arrived in the
/// hundred-and-thirty-second session and nobody re-read the row for forty-one.
///
/// The fixture is a widget whose `/AA` switches an optional content group: `/E` on, `/X` off,
/// `/D` on again. That makes the *page* the assertion — a layer's state decides what is drawn —
/// rather than anything about the event plumbing, which is what would still pass if the actions
/// were read and dropped.
#[test]
fn the_pointer_raises_table_197s_events() {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_triggers(),
            password: None,
            fragment: None,
        })
        .collect();
    assert_eq!(
        marks(&opened),
        Some(0),
        "the layer opens off, so the page draws nothing"
    );

    // The page is 100 × 100 in a 400 × 400 viewport, and the widget is its bottom-left quarter.
    // The points are taken from the *document* through the geometry the viewer reports, which is
    // trap 12a's rule: a point taken from the code under test would follow it into its own
    // mirror.
    let inside = device_point(&viewer, [10.0, 10.0, 30.0, 30.0], 100.0);
    let outside = device_point(&viewer, [70.0, 70.0, 90.0, 90.0], 100.0);

    let moved = |viewer: &mut Viewer, at| {
        let events: Vec<Event> = viewer
            .handle(Command::Pointer {
                at,
                action: PointerAction::Moved,
            })
            .collect();
        marks(&events)
    };

    assert_eq!(
        moved(&mut viewer, inside),
        Some(1),
        "`/E` switched the layer on, so the page has a mark"
    );
    assert_eq!(
        moved(&mut viewer, inside),
        None,
        "the cursor is still inside, so nothing is entered again"
    );
    assert_eq!(
        moved(&mut viewer, outside),
        Some(0),
        "`/X` switched it off again"
    );

    let pressed: Vec<Event> = viewer
        .handle(Command::Pointer {
            at: inside,
            action: PointerAction::Pressed,
        })
        .collect();
    assert_eq!(
        marks(&pressed),
        Some(1),
        "`/E` and then `/D`, in the order the cursor arrived and pressed"
    );
}

/// §12.6.3's `/Fo` and `/Bl`, the last two of Table 197's ten this program did not raise.
///
/// Both entries are
///
/// > (Optional; PDF 1.2; widget annotations only)
///
/// and the standard says what happens when an annotation "receives the input focus" and nothing
/// whatever about how it comes to. So **a press inside a widget's active area gives it the
/// focus, and a press anywhere else takes it away** — a choice, and the one every pointing
/// interface makes, recorded the same way "a press dragged off a link does not activate it" is.
///
/// `doc/todo/25` recorded these two as wanting "keyboard focus, which `viewer-core` does not
/// have — there is no focus model in `Command` at all", which reads as a vocabulary problem and
/// is not one: focus arrives through the pointer this program already has, and what a keyboard
/// would add is Table 31's `/Tabs` *order*, which is a different clause.
///
/// The fixture is two annotations — a widget whose `/Fo` switches a layer on and whose `/Bl`
/// switches it off, and a link beside it that is not a widget — so the assertion is the page.
#[test]
fn a_press_gives_a_widget_the_focus_and_a_press_elsewhere_takes_it_away() {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_focus_triggers(),
            password: None,
            fragment: None,
        })
        .collect();
    assert_eq!(marks(&opened), Some(0), "the layer opens off");

    let widget = device_point(&viewer, [10.0, 10.0, 30.0, 30.0], 100.0);
    let link = device_point(&viewer, [60.0, 60.0, 90.0, 90.0], 100.0);
    let press = |viewer: &mut Viewer, at| {
        let events: Vec<Event> = viewer
            .handle(Command::Pointer {
                at,
                action: PointerAction::Pressed,
            })
            .collect();
        marks(&events)
    };

    assert_eq!(press(&mut viewer, widget), Some(1), "`/Fo` switched it on");
    assert_eq!(
        press(&mut viewer, widget),
        None,
        "the widget already has the focus, so nothing is received again"
    );
    assert_eq!(
        press(&mut viewer, link),
        Some(0),
        "a press on something that is not a widget blurs it, and `/Bl` switched it off"
    );

    // And a page turned takes the focus with it, wherever the pointer is.
    assert_eq!(press(&mut viewer, widget), Some(1), "focused again");
    let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Next)).collect();
    assert!(
        turned
            .iter()
            .any(|event| matches!(event, Event::PageChanged { index: 1, .. })),
        "{turned:?}"
    );
    let back: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Previous)).collect();
    assert_eq!(
        marks(&back),
        Some(0),
        "the widget lost the focus when its page did, so `/Bl` switched the layer off"
    );
}

/// Two pages; the first holds a widget stating §12.6.3's `/Fo` and `/Bl`, and a link beside it.
fn with_focus_triggers() -> Vec<u8> {
    use std::fmt::Write as _;
    let content = "/OC /L1 BDC 20 20 10 10 re f EMC";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R \
         /OCProperties << /OCGs [6 0 R] /D << /OFF [6 0 R] >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Count 2 /Kids [3 0 R 9 0 R] >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Annots [5 0 R 10 0 R] /Resources << /Properties << /L1 6 0 R >> >> >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [0 0 50 50] /F 4 \
         /AA << /Fo 7 0 R /Bl 8 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /OCG /Name (layer) >>\nendobj\n\
         7 0 obj\n<< /S /SetOCGState /State [/ON 6 0 R] >>\nendobj\n\
         8 0 obj\n<< /S /SetOCGState /State [/OFF 6 0 R] >>\nendobj\n\
         9 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>\nendobj\n\
         10 0 obj\n<< /Type /Annot /Subtype /Link /Rect [55 55 95 95] /F 4 >>\nendobj\n",
        content.len().saturating_add(1),
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// How many commands the render these events asked for holds, if they asked for one.
///
/// The *page* is the assertion — a layer's state decides what is drawn — rather than anything
/// about the event plumbing, which is what would still pass if the actions were read and
/// dropped.
fn marks(events: &[Event]) -> Option<usize> {
    events.iter().rev().find_map(|event| match event {
        Event::NeedsRender(request) => Some(request.list.commands().len()),
        _ => None,
    })
}

/// A page whose one widget states §12.6.3's `/E`, `/X` and `/D`, each switching a layer.
fn with_triggers() -> Vec<u8> {
    use std::fmt::Write as _;
    let content = "/OC /L1 BDC 20 20 10 10 re f EMC";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R \
         /OCProperties << /OCGs [6 0 R] /D << /OFF [6 0 R] >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Annots [5 0 R] /Resources << /Properties << /L1 6 0 R >> >> >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [0 0 50 50] /F 4 \
         /AA << /E 7 0 R /X 8 0 R /D 7 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /OCG /Name (layer) >>\nendobj\n\
         7 0 obj\n<< /S /SetOCGState /State [/ON 6 0 R] >>\nendobj\n\
         8 0 obj\n<< /S /SetOCGState /State [/OFF 6 0 R] >>\nendobj\n",
        content.len().saturating_add(1),
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A one-page fixture whose catalog `/OpenAction` states the destination given.
///
/// Written out by hand so that the destination array is legible as PDF: what is under test is
/// this crate's reading of Table 149, and a fixture built by our own code would share any
/// misreading of it. The page is 600×800 with content in one corner, so a `/FitB` has something
/// smaller than the page to fit.
fn with_open_action(destination: &str) -> Vec<u8> {
    use std::fmt::Write as _;

    let content = "0 0 1 rg 100 600 200 100 re f\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OpenAction {destination} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n",
        content.len()
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Opens that fixture into a 300×400 viewport and answers with the page's geometry.
fn geometry_for(destination: &str) -> viewer_core::PageGeometry {
    geometry_in(destination, 300, 400)
}

/// The same, into a viewport of the given size.
fn geometry_in(destination: &str, width: u32, height: u32) -> viewer_core::PageGeometry {
    let mut viewer = Viewer::new(width, height, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_open_action(destination),
            password: None,
            fragment: None,
        })
        .count();
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    geometry
}

/// ISO 32000-2 §12.3.2.1's other two items, which this crate answered with nothing until now.
///
/// > A destination defines a particular view of a document, consisting of the following items:
/// >
/// > - The page of the document that shall be displayed
/// > - The location of the document window on that page
/// > - The magnification (zoom) factor
///
/// The page has always been computed. The other two need a window, and the reason the ledger
/// gave for not applying them — "properties of a window with scrolling and zoom, which this
/// program does not have" — stopped being true in the hundred-and-thirty-second session and
/// went on being written down for sixty-nine more. ADR 0162.
///
/// The fixture is a 600×800 page in a 300×400 viewport, so a fit is 0.5 and every number below
/// is checkable by hand.
#[test]
fn an_open_action_states_where_to_look_at_the_page_and_how_large() {
    // `/Fit`: the whole page, both directions. 300/600 and 400/800 both give 0.5.
    let fit = geometry_for("[3 0 R /Fit]");
    assert!(
        (fit.scale - 0.5).abs() < 1e-3,
        "the whole page fits at half size, got {}",
        fit.scale
    );

    // `/FitH` needs a window the page does not already fit vertically, or there is nothing to
    // scroll: 300x200, where fitting the 600-wide page gives 0.5 and a 400-tall raster.
    //
    // `/FitH 800` puts user-space y 800 — the page's top — at the window's top, which is an
    // origin of zero rather than the centring a smaller page would get.
    let fit_h = geometry_in("[3 0 R /FitH 800]", 300, 200);
    assert!((fit_h.scale - 0.5).abs() < 1e-3, "{}", fit_h.scale);
    assert!(
        fit_h.origin.1.abs() < 1.0,
        "the top of the page is at the top of the window, got {:?}",
        fit_h.origin
    );

    // `/FitH 400`: the same magnification, with the middle of the page at the window's top —
    // 400 user units down from the top, at half size, is 200 device pixels of scroll.
    let middle = geometry_in("[3 0 R /FitH 400]", 300, 200);
    assert!(
        (middle.origin.1 + 200.0).abs() < 2.0,
        "half way down the page, got {:?}",
        middle.origin
    );

    // `/XYZ` states its own magnification, and a zoom of 1 is 72 dpi — one device pixel per
    // user space unit, whatever the window is.
    let xyz = geometry_for("[3 0 R /XYZ 0 800 1]");
    assert!(
        (xyz.scale - 1.0).abs() < 1e-3,
        "a zoom of 1 is actual size, got {}",
        xyz.scale
    );
    assert_eq!(
        (xyz.width, xyz.height),
        (600, 800),
        "and the raster is the page's own size"
    );

    // `/FitR`: a 200x100 rectangle in a 300x400 window fits at 1.5 — the width decides, because
    // 400/100 would be 4 — and the rectangle's top-left corner goes to the window's.
    let fit_r = geometry_for("[3 0 R /FitR 100 600 300 700]");
    assert!(
        (fit_r.scale - 1.5).abs() < 1e-2,
        "the narrower fit decides, got {}",
        fit_r.scale
    );
    assert!(
        (fit_r.origin.0 + 150.0).abs() < 2.0 && (fit_r.origin.1 + 150.0).abs() < 2.0,
        "the rectangle's corner is the window's corner, got {:?}",
        fit_r.origin
    );

    // `/FitB`: the same again for "the smallest rectangle enclosing all of its contents", which
    // no page dictionary states. The fixture's one rectangle is 200x100 at (100, 600), so this
    // must land where `/FitR` over the same box did.
    let fit_b = geometry_for("[3 0 R /FitB]");
    assert!(
        (fit_b.scale - fit_r.scale).abs() < 1e-2,
        "the content box is the rectangle, got {} against {}",
        fit_b.scale,
        fit_r.scale
    );
}

/// A destination with no magnification leaves the one in force, which is Table 149's own rule.
///
/// ISO 32000-2 §12.3.2.2, Table 149:
///
/// > A null value for any of the parameters left , top , or zoom specifies that the current
/// > value of that parameter shall be retained unchanged.
///
/// So the two `null`s and the omitted zoom below must each change nothing about the scale, while
/// still moving the window — which is the distinction that makes this worth its own test: a
/// reading that treated an absent parameter as a zero would fit the page instead.
#[test]
fn a_destinations_null_parameters_leave_what_they_do_not_state() {
    let stated = geometry_for("[3 0 R /XYZ 0 800 2]");
    assert!((stated.scale - 2.0).abs() < 1e-3, "{}", stated.scale);

    let no_zoom = geometry_for("[3 0 R /XYZ 0 800 null]");
    let fitted = geometry_for("[3 0 R /Fit]");
    assert!(
        (no_zoom.scale - fitted.scale).abs() < 1e-3,
        "a null zoom keeps the one the document opened at, got {} against {}",
        no_zoom.scale,
        fitted.scale
    );

    // "A zoom value of 0 has the same meaning as a null value."
    let zero = geometry_for("[3 0 R /XYZ 0 800 0]");
    assert!((zero.scale - fitted.scale).abs() < 1e-3, "{}", zero.scale);
}

/// A three-page fixture whose annotations and pages carry §12.6.3's page-scoped triggers.
///
/// Each event performs a `/URI` action naming itself, because a URI is the one action this crate
/// hands *out* rather than acting on — so the events arrive in `Event::OpenUri` in the order they
/// were raised, which is what this test is about.
fn with_page_triggers() -> Vec<u8> {
    use std::fmt::Write as _;

    let page = |number: usize, object: usize, annot: usize| {
        format!(
            "{object} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << >> /Annots [{annot} 0 R] \
             /AA << /O << /S /URI /URI (page{number}-O) >> \
             /C << /S /URI /URI (page{number}-C) >> >> >>\nendobj\n\
             {annot} 0 obj\n<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] \
             /AA << /PO << /S /URI /URI (page{number}-PO) >> \
             /PC << /S /URI /URI (page{number}-PC) >> \
             /PV << /S /URI /URI (page{number}-PV) >> \
             /PI << /S /URI /URI (page{number}-PI) >> >> >>\nendobj\n"
        )
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 5 0 R 7 0 R] /Count 3 >>\nendobj\n\
         {}{}{}",
        page(1, 3, 4),
        page(2, 5, 6),
        page(3, 7, 8)
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The URIs these events handed out, in order.
fn uris(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::OpenUri { uri, .. } => Some(uri.clone()),
            _ => None,
        })
        .collect()
}

/// ISO 32000-2 §12.6.3's four page-scoped trigger events, and Table 198's two, in its order.
///
/// > The action shall be executed after the O action in the page's additional - actions
/// > dictionary (see "Table 198 - Entries in a page object's additional - actions dictionary")
/// > and the OpenAction entry in the document Catalog (see "Table 29 - Entries in the catalog
/// > dictionary"), if such actions are present.
///
/// and of `/PC`, that it shall be executed before the page's own `/C`. So a turn is: the
/// leaving page's annotations, then the leaving page; then the
/// arriving page, then its annotations. Four of Table 197's ten events and both of Table 198's,
/// none of which anything raised until the two-hundred-and-fourth session. ADR 0164.
///
/// `/PV` and `/PI` land beside `/PO` and `/PC` because §12.6.3 says what separates them —
/// "[t]he PV and PI entries allow a distinction between pages that are open and pages that are
/// visible. At any one time, while more than one page may be visible, depending on the page
/// layout" — and this viewer shows one page at a time.
#[test]
fn a_page_turn_raises_the_events_the_clause_orders() {
    let mut viewer = Viewer::new(200, 200, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_page_triggers(),
            password: None,
            fragment: None,
        })
        .collect();
    assert_eq!(
        uris(&opened),
        ["page1-O", "page1-PO", "page1-PV"],
        "opening a document opens its first page: the page's own action, then its annotations'"
    );

    let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
    assert_eq!(
        uris(&turned),
        [
            "page1-PC", "page1-PI", "page1-C", "page2-O", "page2-PO", "page2-PV"
        ],
        "the leaving page's annotations before its /C, and the arriving page's /O before its"
    );

    // A page the document does not have raises nothing, and neither does going where we are.
    let same: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
    assert!(uris(&same).is_empty(), "{:?}", uris(&same));
}

/// A zoom holds the point it is given, which is what makes a wheel feel like magnification.
///
/// No clause decides this — §12.3.2.1's magnification is a *document's* opinion about where to
/// look and this is a reader's — so what is checked is the invariant the choice was made for:
/// **the page point under a viewport point is the same page point afterwards**, which is
/// `(at - origin) / scale` before and after. ADR 0166.
///
/// The fixture is the 600×800 page again, so every number below is checkable by hand: at a
/// magnification of 0.5 the raster is exactly the 300×400 viewport, and one step is 1.25.
#[test]
fn a_zoom_holds_the_point_it_is_given() {
    let held = |geometry: &viewer_core::PageGeometry, at: (f32, f32)| {
        (
            (at.0 - geometry.origin.0) / geometry.scale,
            (at.1 - geometry.origin.1) / geometry.scale,
        )
    };

    let mut viewer = Viewer::new(300, 400, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_open_action("[3 0 R /Fit]"),
            password: None,
            fragment: None,
        })
        .count();
    let geometry = |viewer: &Viewer| {
        let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
            panic!("the page on the screen has a geometry");
        };
        geometry
    };

    // Exactly fitted: a 300×400 raster in a 300×400 viewport, so nothing is centred and nothing
    // is scrolled, and a step in about (60, 100) is the plainest case there is.
    let at = (60.0, 100.0);
    let before = geometry(&viewer);
    assert!((before.scale - 0.5).abs() < 1e-3, "{}", before.scale);
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::In,
            at: Some(at),
        })
        .for_each(drop);
    let after = geometry(&viewer);
    assert!((after.scale - 0.625).abs() < 1e-3, "{}", after.scale);
    assert!(
        (after.origin.0 + 15.0).abs() < 1e-3 && (after.origin.1 + 25.0).abs() < 1e-3,
        "60 and 100 grow by a quarter, so the raster moves 15 and 25 up and left: {:?}",
        after.origin
    );
    let (bx, by) = held(&before, at);
    let (ax, ay) = held(&after, at);
    assert!(
        (ax - bx).abs() < 0.01 && (ay - by).abs() < 0.01,
        "the same page point under the pointer: {bx},{by} then {ax},{ay}"
    );

    // A page *smaller* than the viewport is centred, and the scroll cannot express an anchor at
    // all — `Open::origin` returns the slack and `clamp_scroll` puts the scroll back to zero. So
    // the requirement here is that anchoring is a no-op rather than a jitter.
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(0.25),
            at: Some((0.0, 0.0)),
        })
        .for_each(drop);
    let small = geometry(&viewer);
    assert_eq!((small.width, small.height), (150, 200));
    assert!(
        (small.origin.0 - 75.0).abs() < 1e-3 && (small.origin.1 - 100.0).abs() < 1e-3,
        "centred in the slack, not pulled to the corner it was zoomed at: {:?}",
        small.origin
    );

    // And out of that centring into a page larger than the viewport, which is the case the
    // arithmetic that reads the scroll alone gets wrong: the point under (100, 150) is 100 and
    // 200 user units in, and at a magnification of 1 that is a scroll of 0 and 50.
    let at = (100.0, 150.0);
    let (bx, by) = held(&small, at);
    assert!(
        (bx - 100.0).abs() < 0.01 && (by - 200.0).abs() < 0.01,
        "{bx},{by}"
    );
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(1.0),
            at: Some(at),
        })
        .for_each(drop);
    let large = geometry(&viewer);
    assert_eq!((large.width, large.height), (600, 800));
    let (ax, ay) = held(&large, at);
    assert!(
        (ax - bx).abs() < 0.01 && (ay - by).abs() < 0.01,
        "the same page point again, out of a centred page: {bx},{by} then {ax},{ay}"
    );

    // A step that changes nothing must move the scroll by nothing rather than by the ratio it
    // did not get, which is what `Open::stepped`'s clamp produces at either end of `ZOOM_RANGE`.
    // Asked here as a ratio of exactly one, because the top of that range — 64 — puts this page
    // past the pixel budget and there would be no geometry to compare.
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(1.0),
            at: Some((0.0, 0.0)),
        })
        .for_each(drop);
    let again = geometry(&viewer);
    assert!(
        (again.scale - large.scale).abs() < 1e-3 && again.origin == large.origin,
        "a zoom to the magnification already showing is not a scroll: {large:?} then {again:?}"
    );
}

/// A page with a `NoZoom` annotation is interpreted again on a zoom, and no other page is.
///
/// §12.5.3 makes that one annotation's placement a function of the magnification, which is the
/// one thing in the standard that breaks the display list's promise — and the point of
/// `Interpretation::view_dependent` is that it breaks it *only* there.
/// `zooming_rasterises_again_without_interpreting_again` above is the other half of this pair
/// and holds for every page that has no such annotation.
#[test]
fn a_no_zoom_annotation_is_the_one_thing_a_zoom_re_interprets() {
    let mut viewer = Viewer::new(300, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_no_zoom_annotation(),
            password: None,
            fragment: None,
        })
        .collect();
    let first = request(&opened).clone();
    serve(&mut viewer, &first);

    let zoomed: Vec<_> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::In,
            at: None,
        })
        .collect();
    let second = request(&zoomed).clone();
    assert!(
        !std::sync::Arc::ptr_eq(&first.list, &second.list),
        "the annotation's size is a function of the zoom, so the page is read again"
    );
    assert!(second.target.width > first.target.width);
}

/// The same fixture as `pdf-model`'s: a 100×100 page and one 30×30 `Square` at `/F 12`.
fn with_no_zoom_annotation() -> Vec<u8> {
    use std::fmt::Write as _;

    let appearance = "0 0 0 rg 0 0 30 30 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [40 40 70 70] /F 12 \
         /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 30 30] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        appearance.len().saturating_add(1)
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A 100×100 page carrying one `Text` annotation and the popup window it opens.
///
/// §12.5.6.14's shape exactly: the markup annotation states `/Popup`, the popup states `/Parent`,
/// and the text a window would show is the parent's `/Contents` rather than the popup's.
fn with_a_popup(open: bool) -> Vec<u8> {
    use std::fmt::Write as _;

    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R 6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Text /Rect [10 80 30 100] /Popup 6 0 R \
         /T (the author) /Contents (a sticky note) /C [1 1 0] >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Popup /Rect [30 20 90 70] /Parent 5 0 R \
         /Open {open} >>\nendobj\n"
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Opens `with_a_popup` into a 200×200 viewport and settles the first frame.
fn popup_viewer(open: bool) -> Viewer {
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_popup(open),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

#[test]
fn a_document_that_opens_a_popup_window_says_so_before_anybody_clicks() {
    // Table 186's `/Open`: "A flag specifying whether the popup annotation shall initially be
    // displayed open." Seven popups in the corpus state it; this is the same shape.
    let viewer = popup_viewer(true);
    let Answer::Popups(windows) = viewer.query(Query::Popups) else {
        panic!("a popup query answers with popups");
    };
    assert_eq!(windows.len(), 1);
    let window = &windows[0];
    // Table 186: the parent's `Contents`, `M`, `C` and `T` "shall override those of the popup
    // annotation itself" — and the popup here states none of the four.
    assert_eq!(window.text.as_deref(), Some("a sticky note"));
    assert_eq!(window.title.as_deref(), Some("the author"));
    assert_eq!(
        window.colour.map(|c| (c.r, c.g, c.b)),
        Some((1.0, 1.0, 0.0))
    );
    assert_eq!(window.parent, Some(pdf_syntax::ObjectId::new(5, 0)));

    // The window is 60 × 50 user units on a page drawn at some magnification, and the quad is
    // the same mapping `Query::Focus` and `Query::Selection` answer in — clockwise from the
    // top-left, y downwards.
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    let width = window.quad[2] - window.quad[0];
    let height = window.quad[5] - window.quad[3];
    assert!((width - 60.0 * geometry.scale).abs() < 0.01, "{width}");
    assert!((height - 50.0 * geometry.scale).abs() < 0.01, "{height}");
}

#[test]
fn a_closed_popup_is_no_window_until_the_annotation_is_clicked() {
    // §12.5.1: "When the user activates the annotation by clicking it, it exhibits its
    // associated object, such as by opening a popup window displaying a text note."
    let mut viewer = popup_viewer(false);
    assert!(
        matches!(viewer.query(Query::Popups), Answer::Popups(windows) if windows.is_empty()),
        "the file says closed, so nothing is open"
    );
    // The `Text` annotation is `/Rect [10 80 30 100]`, a 20-unit square at the page's top left.
    let on_note = device_point(&viewer, [10.0, 80.0, 30.0, 100.0], 100.0);
    viewer
        .handle(Command::Pointer {
            at: on_note,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at: on_note,
            action: PointerAction::Released,
        })
        .for_each(drop);
    assert!(
        matches!(viewer.query(Query::Popups), Answer::Popups(windows) if windows.len() == 1),
        "the click exhibited the annotation's object"
    );
    // A second click closes it, which the clause does not state and this crate chooses.
    viewer
        .handle(Command::Pointer {
            at: on_note,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at: on_note,
            action: PointerAction::Released,
        })
        .for_each(drop);
    assert!(
        matches!(viewer.query(Query::Popups), Answer::Popups(windows) if windows.is_empty()),
        "and a second click puts it away"
    );
}

#[test]
fn a_host_may_open_a_popup_without_a_pointer() {
    // `Command::Activate` is what a panel row sends and what a keyboard would: the object, and
    // the document decides what activating it means. Here it means §12.5.1's exhibition.
    let mut viewer = popup_viewer(false);
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .for_each(drop);
    assert!(
        matches!(viewer.query(Query::Popups), Answer::Popups(windows) if windows.len() == 1),
        "activating the parent opens its window"
    );
}

/// §12.5.6.10: a person marks up what they selected, and undo takes it away again.
///
/// The first edit that *adds* an object to a document rather than changing one it holds, which
/// `CLAUDE.md`'s amended exclusion permits: what a user does to an open document is not
/// authoring. Three things are checked because three things could go wrong in silence — the
/// annotation is drawn at all, it is drawn in the colour asked for, and the log's replay removes
/// it rather than leaving it behind.
#[test]
fn a_markup_over_a_selection_is_drawn_and_undone() {
    let (mut viewer, events) = opened(600, 800);
    let before = yellow(&raster(request(&events)));
    assert_eq!(before, 0, "the note is black on white before anything");

    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    let events: Vec<_> = viewer
        .handle(Command::Edit(Edit::Markup {
            kind: pdf_model::view::Markup::Highlight,
            colour: [1.0, 1.0, 0.0],
        }))
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Dirty { dirty: true, .. })),
        "{events:?}"
    );
    let marked = yellow(&raster(request(&events)));
    assert!(
        marked > 5000,
        "§12.5.6.10's wash is not on the page: {marked} yellow pixels"
    );

    let events: Vec<_> = viewer.handle(Command::Undo).collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Dirty { dirty: false, .. })),
        "{events:?}"
    );
    assert_eq!(
        yellow(&raster(request(&events))),
        0,
        "undo replays the log's surviving prefix, so the annotation is gone"
    );
}

/// Rasterises a render request with the CPU backend, which is what a tier-1 host does.
fn raster(request: &viewer_core::RenderRequest) -> pdf_render::Raster {
    CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the CPU backend draws this page")
}

/// How many pixels are yellow: a full red and green with no blue, which is what a `Multiply`
/// wash of `[1 1 0]` over white leaves and what nothing else on this page draws.
fn yellow(raster: &pdf_render::Raster) -> usize {
    raster
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 200 && pixel[1] > 200 && pixel[2] < 60)
        .count()
}

/// A selection survives the page being drawn again, and ends when the page is turned.
///
/// The two are different events and one line treated them alike: every re-interpretation cleared
/// the selection, so a field edited, a layer switched or §12.5.6.10's markup added took a
/// person's selection away from them. A page's readback is a function of the document and the
/// view state, and after any of those three it is still *this* page's — so the range still names
/// what it named. A page **turn** is the case the line was written for.
#[test]
fn a_selection_survives_a_redraw_of_the_same_page_and_not_a_page_turn() {
    let (mut viewer, events) = opened(600, 800);
    serve(&mut viewer, &request(&events).clone());
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    let Answer::Selected(selected) = viewer.query(Query::Selection) else {
        panic!("everything on the page is selected");
    };
    let before = selected.text.len();
    assert!(before > 0, "the cover page reads back as something");

    // §12.5.6.10's markup rebuilds the display list, which is the case that took the selection.
    let events: Vec<_> = viewer
        .handle(Command::Edit(Edit::Markup {
            kind: pdf_model::view::Markup::Highlight,
            colour: [1.0, 1.0, 0.0],
        }))
        .collect();
    serve(&mut viewer, &request(&events).clone());
    let Answer::Selected(after) = viewer.query(Query::Selection) else {
        panic!("the selection is still there after the page is drawn again");
    };
    assert_eq!(after.text.len(), before);

    // A page turn is a different page's readback, and there the range means nothing.
    viewer
        .handle(Command::GoTo(PageTarget::Next))
        .for_each(drop);
    assert!(
        matches!(viewer.query(Query::Selection), Answer::None),
        "a turned page ends the selection"
    );
}
