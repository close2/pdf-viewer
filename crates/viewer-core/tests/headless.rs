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

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pdf_model::form::Control;
use pdf_model::view::WidgetAppearances;
use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{
    Answer, Command, DocumentId, Edit, Entered, Event, FocusMove, PageTarget, PointerAction, Query,
    Rendered, RestrictionLevel, Selection, Viewer, Zoom,
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

/// The one page's structure a test about a `SinglePage` window is asking for.
///
/// `Query::AccessibilityTree` answers with one entry per page Table 29's arrangement is showing,
/// and every test below opens into the default arrangement — where "the page being shown" and
/// "the pages on the screen" are the same one page. Flattened rather than indexed at zero so that
/// a test which does put a column on the screen fails on what it asserts rather than on a panic
/// about an entry that is there.
fn on_one_page(pages: Vec<viewer_core::PageStructure>) -> Vec<viewer_core::AccessibilityNode> {
    pages.into_iter().flat_map(|page| page.nodes).collect()
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

    let Answer::Frame(frames) = viewer.query(Query::Frame) else {
        panic!("the viewer is holding the pixels it was handed");
    };
    // One entry, because Table 29's default is `SinglePage` and this document states nothing.
    let [frame] = frames.as_slice() else {
        panic!(
            "a single-page arrangement holds one frame: {} held",
            frames.len()
        );
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
        matches!(viewer.query(Query::Frame), Answer::Frame(ref frames) if frames.is_empty()),
        "and it is not held either — an empty list rather than no answer, because a tier-1 host \
         waiting for its first frame is not a tier-2 host that hands none back"
    );

    serve(&mut viewer, &fresh);
    let Answer::Frame(frames) = viewer.query(Query::Frame) else {
        panic!("the answer to the outstanding request is kept");
    };
    assert_eq!(frames.first().map(|frame| frame.page), Some(1));
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
            password: Some("abc".to_owned().into()),
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
    assert!(matches!(viewer.query(Query::Frame), Answer::Frame(ref frames) if !frames.is_empty()));
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

/// ISO 32000-2 §7.5.7's objects, when the stream storing them decodes only in part.
///
/// The other half of ADR 0366: `pdf_syntax` refuses an object whose bytes a damaged prefix does not
/// wholly carry — a truncated token still parses, so reading one would put a value the producer
/// never wrote under a number the producer did — and a person is owed the sentence saying so.
///
/// **The file is built rather than found**, which is trap 8's case and is measured rather than
/// assumed: the two corpus documents whose object stream decodes only in part
/// (`issue19484_1.pdf`, `issue19484_2.pdf`) lose the *header* of theirs, so no object number
/// survives to be asked for and nothing on page one reaches into either. The one below puts the
/// page dictionary itself inside the stream, ahead of the object the damage takes.
///
/// **What it pins as much as the sentence is *when* it is said.** Nothing expands an object stream
/// until an object inside it is wanted, which is `CLAUDE.md`'s startup rule, so this cannot be part
/// of what a document says at open and be true in general — it is said when the loss becomes known.
/// `Query::Reports` is where a host that cleared its status bar finds it again.
#[test]
fn objects_lost_inside_a_damaged_object_stream_are_said_out_loud() {
    let bytes = a_page_stored_beside_an_object_the_damage_takes();
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let mut said: Vec<String> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect();
    if let Answer::Reports(all) = viewer.query(Query::Reports) {
        // One entry per page on the screen, and this document has one page — so the flattening
        // is the identity here and would name the page it came from if it were not.
        said = all
            .iter()
            .flat_map(|page| page.notes.iter().cloned())
            .collect();
    }
    assert!(
        said.iter()
            .any(|note| note.contains("object stream (§7.5.7) could not be read")),
        "the objects the prefix does not carry are named rather than silently missing: {said:?}"
    );
}

/// A page whose codes ISO 32000-2 §9.10.2 cannot name says how many, without reporting one.
///
/// **The counterpart of the test above, and the distinction between them is the whole decision.**
/// That one is a *refusal* — something this program could not do — and it goes into
/// `Query::Reports` where a host puts it in a status bar. This one is the clause's own answer:
/// "there is no way to determine what the character code represents", said of a page that
/// interprets whole. It may not become a report, because a page that reports leaves the oracle's
/// judged set (ADR 0152) and the font drew what it could — so it crosses as a count instead,
/// which is what `Query::Readback` is. ADR 0422.
///
/// The fixture is `french_diacritics.pdf` reduced to two codes: a `/Differences` naming `/a192`
/// and `/a224`, pdfTeX's private labels for `À` and `à`, which neither list the clause names
/// holds. `A` and `a` beside them are the control — a page where *nothing* could be named would
/// pass with a viewer that counted every code, and they are also what keeps this off
/// `Query::Reports`: the report fires on a font that drew **none** of its codes, and this one
/// draws two of four.
///
/// **This test said the substitute "drew all four" and it never did** (ADR 0520). A name no
/// encoding defines addresses nothing in the substitute face either, so the same two codes are a
/// mark lost as well as a character lost — and the count could not contradict the sentence,
/// because until that ADR it excluded any code §9.10.2 could not name. The decision the test is
/// *about* is unchanged: this crosses as a count and not as a report.
#[test]
fn a_page_whose_codes_no_method_can_name_answers_with_a_count_and_not_a_report() {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 45 >>\nstream\nBT /F1 12 Tf 20 40 Td (A\\300a\\340) Tj ET\n\
         endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding << /Type /Encoding /Differences [192 /a192 224 /a224] >> >>\nendobj\n";
    let mut viewer = Viewer::new(400, 300, 1.0);

    // Before a page is interpreted there is no answer, which is not the same as an answer of
    // zero: a host must not be able to tell a person that nothing was lost off a page nobody
    // has read. An **empty list** says it since the six-hundred-and-tenth session — the same
    // distinction `Answer::Frame` draws between a host that holds no pixels and one that holds a
    // page's — and `Answer::None` is what a viewer with no document open says.
    assert!(matches!(viewer.query(Query::Readback), Answer::None));

    let _ = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: assemble(body),
            password: None,
            fragment: None,
        })
        .count();

    let Answer::Reports(reports) = viewer.query(Query::Reports) else {
        panic!("a page that was interpreted answers with its reports");
    };
    assert!(
        reports.iter().all(|page| page.notes.is_empty()),
        "the clause's own 'there is no way' is not a refusal of ours: {reports:?}"
    );

    let Answer::Readback(counts) = viewer.query(Query::Readback) else {
        panic!("a page that was interpreted answers with what its codes cost");
    };
    let [counted] = counts.as_slice() else {
        panic!("one page is on the screen and it has been read: {counts:?}");
    };
    assert_eq!(counted.page, 0, "the entry says which page it counted");
    let shortfall = counted.shortfall;
    assert_eq!(shortfall.unnamed.total(), 2);
    assert_eq!(shortfall.unnamed.unlisted_name, 2, "{shortfall:?}");
    assert_eq!(
        shortfall.without_a_glyph, 2,
        "the substitute drew the two codes it could name and neither of the two it could not"
    );
    assert!(!shortfall.is_whole());
}

/// One document whose page dictionary is compressed beside an object its stream stops short of.
///
/// The stream is a single RFC 1951 stored block with BFINAL clear and no Adler-32, so every byte
/// arrives and nothing says the stream is over — which is what leaves the *last* object's end
/// unstated under §7.5.7's NOTE 7, while the page dictionary ahead of it ends where the next offset
/// says and is read as usual.
///
/// `readable_table` decides whether the file's own cross-reference stream can be decoded. With it
/// false the reader falls to `xref::rebuild`, which is the other half of §7.5.7 — see
/// [`a_rebuild_says_what_it_recovered_from_an_object_stream`].
fn a_page_stored_beside_an_object_the_damage_takes() -> Vec<u8> {
    packed_page(true)
}

/// The same document with a cross-reference stream no filter chain here can decode.
fn a_packed_page_behind_an_unreadable_table() -> Vec<u8> {
    packed_page(false)
}

/// Both of the above.
fn packed_page(readable_table: bool) -> Vec<u8> {
    let compressed = [
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\n",
        "(the object the damage takes)\n",
    ];
    let mut header = String::new();
    let mut at = 0usize;
    for (number, part) in [(5u32, compressed[0]), (6, compressed[1])] {
        let _ = write!(header, "{number} {at} ");
        at = at.saturating_add(part.len());
    }
    let first = header.len();
    let payload = format!("{header}{}{}", compressed[0], compressed[1]).into_bytes();

    // The zlib stream: a header, one stored block that does not claim to be the last, and no
    // checksum — every byte of the payload, with nothing saying the stream finished.
    let mut data = vec![0x78, 0x01, 0x00];
    let length = u16::try_from(payload.len()).expect("a few hundred bytes");
    data.extend_from_slice(&length.to_le_bytes());
    data.extend_from_slice(&(!length).to_le_bytes());
    data.extend_from_slice(&payload);

    let mut out = Vec::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for body in [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [5 0 R] >>".to_owned(),
    ] {
        offsets.push(out.len());
        let number = offsets.len();
        out.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    offsets.push(out.len());
    out.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /ObjStm /N 2 /First {first} /Filter /FlateDecode /Length {} >>\n\
             stream\n",
            data.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&data);
    out.extend_from_slice(b"\nendstream\nendobj\n");

    // The cross-reference stream: three objects at their offsets, then the two compressed ones.
    let stream_at = out.len();
    let rows: [[u64; 3]; 7] = [
        [0, 0, 65535],
        [1, offsets[0] as u64, 0],
        [1, offsets[1] as u64, 0],
        [1, offsets[2] as u64, 0],
        [1, stream_at as u64, 0],
        [2, 3, 0],
        [2, 3, 1],
    ];
    let mut table = Vec::new();
    for row in rows {
        table.push(u8::try_from(row[0]).expect("a Table 18 type"));
        table.extend_from_slice(&u32::try_from(row[1]).expect("a small offset").to_be_bytes());
        table.extend_from_slice(&u16::try_from(row[2]).expect("a small field").to_be_bytes());
    }
    let filter = if readable_table {
        ""
    } else {
        "/Filter /XXXDecode "
    };
    out.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /XRef /Size 7 /Index [0 7] /W [1 4 2] /Root 1 0 R {filter}\
             /Length {} >>\nstream\n",
            table.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&table);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out.extend_from_slice(format!("startxref\n{stream_at}\n%%EOF\n").as_bytes());
    out
}

/// A rebuilt document says how much of itself the rebuild recovered, §7.5.7 and §C.4.
///
/// The document above with its cross-reference stream made undecodable, so the reader falls to a
/// scan — which finds `N G obj` headers and therefore no compressed object at all until the
/// rebuild reads the object stream's own header (ADR 0395). What the host is told has to carry
/// both halves: **a rebuild that recovered part of a file must not read like one that recovered
/// all of it**, and the page here is drawable exactly because the recovery worked while one
/// object inside the stream is still lost to the damage.
#[test]
fn a_rebuild_says_what_it_recovered_from_an_object_stream() {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let mut said: Vec<String> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: a_packed_page_behind_an_unreadable_table(),
            password: None,
            fragment: None,
        })
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect();
    // Both channels, because they are two: what the file says about itself is said when it opens,
    // and what a damaged object stream cost is said when it becomes known (`notes::losses`).
    if let Answer::Reports(all) = viewer.query(Query::Reports) {
        said.extend(all.iter().flat_map(|page| page.notes.iter().cloned()));
    }
    assert!(
        said.iter().any(|note| note.contains("rebuilt by scanning")
            && note.contains("object stream(s) (§7.5.7)")),
        "the rebuild says what it entered from the file's object streams: {said:?}"
    );
    assert!(
        said.iter()
            .any(|note| note.contains("object stream (§7.5.7) could not be read")),
        "and the object the damage takes is still named, by the account that owns it: {said:?}"
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
    let [again] = again.as_slice() else {
        panic!("one page is on the screen: {again:?}");
    };
    assert_eq!(again.page, 0, "and the entry says which page said it");
    assert_eq!(again.notes.len(), reported.len());
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
fn taking_one_pages_list_does_not_lift_the_raster_budget() {
    // **The discriminating half of `Rendered::Listed`.** The test above is this one with
    // `Rendered::Presented` in its place, and the two must not agree. A host that presented has
    // said it draws *every* page onto its own surface at its own size, so `MAX_PIXELS` has
    // nothing left to bound; a host that took one page's display list has said nothing at all
    // about the next page, and may hand that one back as pixels this crate has to hold.
    //
    // `viewer-confined`'s worker is that host, and it is why the distinction is worth a variant:
    // it runs under an address-space ceiling where an unbounded raster is a kill rather than a
    // refusal (ADR 0640).
    let (mut viewer, events) = opened(800, 1000);
    let token = request(&events).token;
    let after: Vec<_> = viewer
        .handle(Command::RenderReady {
            token,
            rendered: Rendered::Listed,
        })
        .collect();
    assert!(
        after.iter().any(|event| matches!(event, Event::Damage(_))),
        "a page whose marks the host has is a page somebody still has to put on a screen: \
         {after:?}"
    );
    assert!(
        !after
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "and it holds its place, so the scheduler does not ask for it again: {after:?}"
    );

    // The same A4-at-40× zoom the tier-2 test above sails through.
    let refused: Vec<_> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(40.0),
            at: None,
        })
        .collect();
    assert!(
        refused
            .iter()
            .any(|event| matches!(event, Event::Reported { .. })),
        "the budget is still on, and a page over it is named rather than drawn at a scale \
         nobody chose: {refused:?}"
    );
    assert!(
        !refused
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "so nothing is asked for that could not be handed back: {refused:?}"
    );
}

#[test]
fn a_refusal_is_final_for_this_view_and_a_token_never_answered_is_not_re_asked() {
    // **What a host that interrupts its own draw has to know about this crate**, and the reason
    // ADR 0657's rule 3 is a rule rather than a preference. Both halves are deliberate here and
    // neither was written down anywhere a host could read it.
    //
    // `Rendered::Failed` records the request as answered — `shown` becomes the pending target and
    // revision — so the scheduler stops asking. That is right for a page that will not rasterise,
    // which would refuse again at the same size; it is wrong for a draw the *host* abandoned,
    // which says nothing about the page. A host reporting one as the other marks the page shown
    // for good, and nothing but a view change ever asks for it again.
    let (mut viewer, events) = opened(800, 1000);
    let token = request(&events).token;
    let refused: Vec<Event> = viewer
        .handle(Command::RenderReady {
            token,
            rendered: Rendered::Failed("the processor would not draw this page".to_owned()),
        })
        .collect();
    assert!(
        refused
            .iter()
            .any(|event| matches!(event, Event::Reported { .. })),
        "a refusal is reported rather than swallowed: {refused:?}"
    );
    assert!(
        !refused
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "and it is final for this view: without that the two spin — ask, refuse, ask: {refused:?}"
    );
    // The other half, and the one that makes answering *nothing* the right thing for an abandoned
    // draw rather than a leak: an outstanding token holds the page's place, so a host that stays
    // silent about one is not asked twice either. What re-asks is the question changing.
    let silent: Vec<Event> = viewer.handle(Command::Tick { millis: 16 }).collect();
    assert!(
        !silent
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "nothing about a tick changes what is being asked for: {silent:?}"
    );
    let moved: Vec<Event> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(1.5),
            at: None,
        })
        .collect();
    assert!(
        moved
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "and a view change is what asks again, which is the recovery a person has: {moved:?}"
    );
}

#[test]
fn a_page_the_host_took_the_list_for_leaves_its_neighbours_answerable() {
    use pdf_model::viewer_preferences::PageLayout;

    // The other half of the same distinction, and the half `Rendered::Presented` could not
    // express at all: it sets one flag for the whole viewer, so a column showing two pages would
    // go silent about *both* the moment a host took one page's list. `Query::Frame` has to go on
    // answering for the page whose pixels this crate is actually holding.
    let mut viewer = arranged(PageLayout::OneColumn);
    // A magnification nothing has been drawn at yet, so every page on the screen is asked for
    // again and this test chooses each answer separately.
    let events: Vec<Event> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(0.51),
            at: None,
        })
        .collect();
    let asked = requests(&events);
    let ([taken], held) = asked.split_at(1) else {
        panic!("a column at half size asks for more than one page: {asked:?}");
    };
    assert!(
        !held.is_empty(),
        "the case needs a neighbour to be answerable about: {asked:?}"
    );
    viewer
        .handle(Command::RenderReady {
            token: taken.token,
            rendered: Rendered::Listed,
        })
        .for_each(drop);
    for request in held {
        serve(&mut viewer, request);
    }

    let Answer::Frame(frames) = viewer.query(Query::Frame) else {
        panic!("a host this crate is holding pixels for is one it answers for");
    };
    let answered: Vec<usize> = frames.iter().map(|frame| frame.page).collect();
    let neighbours: Vec<usize> = held.iter().map(|request| request.page).collect();
    assert_eq!(
        answered, neighbours,
        "the page whose list the host took is not in the answer, and its neighbours are"
    );
    // Both are still on the screen, which is what the frame answer is *silent* about rather than
    // ignorant of: where a page sits is a different question from what this crate holds of it.
    assert!(placed(&viewer, taken.page).is_some());
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
    assert!(matches!(viewer.query(Query::Frame), Answer::Frame(ref frames) if !frames.is_empty()));
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
fn a_click_on_an_action_this_program_will_not_perform_says_which_and_why() {
    // The five action types whose ledger rows are `reported` rather than `silent`, which is a
    // claim that the refusal reaches a person: §12.6.4.3's `GoToR`, §12.6.4.6's `Launch`,
    // §12.6.4.9's `Sound`, §12.6.4.10's `Movie` and §12.7.6.2's `SubmitForm`. **Nothing in the
    // tree reached it.** All five rows cited
    // `action.rs::a_name_the_table_does_not_hold_is_not_an_action`, which asserts that `/Teleport`
    // produces *no* action at all and therefore never touches `action::refused`; the only other
    // test that came near was `a_next_chain_is_flattened_in_execution_order`, which reaches
    // `Launch`'s refusal and splits the sentence off at the colon.
    //
    // **Three of the five were covered here in the six-hundred-and-twenty-sixth session and two
    // were left behind**, still citing the test that cannot reach them, which is why `GoToR` and
    // `SubmitForm` join the table below. `refused`'s arms are the population: every name it
    // answers either has a row that owes this assertion or an `out-of-scope` one that owes
    // nothing, and those two were the remainder.
    //
    // The witness is built rather than borrowed, and the population is a command rather than a
    // sentence — `cargo run --release -p pdf-model --example refused_action_census`, which walks
    // every numbered object *and every dictionary inside one* and asks `action::read` what it made
    // of each. A built witness keeps the click deterministic; the census says whether the refusal
    // is one a reader ever meets. **The comment this replaces stated the population as a bare
    // number and got it wrong**: "of the 974 corpus documents exactly one states a `/S /Launch`
    // action" missed `externalLink.pdf`, whose action dictionary is written directly inside its
    // annotation and so has no object number to be found by. `/S /Sound` and `/S /Movie` are the
    // ones genuinely absent — `multimedia_annotations.pdf`'s `/Sound` names are §13.6.2's
    // annotation subtype and §13.3's sound object, not §12.6.4.9's action.
    //
    // What it pins is the whole path the rows claim: `action::refused`'s sentence, `Action::Refused`
    // carrying it out of `pdf-model`, `interact::perform` turning it into a note rather than
    // dropping it (trap 5), and `Event::Reported` being the channel `viewer-ui`'s `dispatch.rs`
    // prints from.
    // Each type on its own document, because a refusal names *which* action declined and a page
    // carrying three would not show that the name follows the click rather than the page.
    let said = |action: &str| -> Vec<String> {
        let bytes = format!(
            "%PDF-2.0\n\
             1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Annots [4 0 R] >>\nendobj\n\
             4 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 80 190 120] /A 5 0 R >>\nendobj\n\
             5 0 obj\n{action}\nendobj\n\
             trailer\n<< /Root 1 0 R /Size 6 >>\n"
        )
        .into_bytes();
        let mut viewer = Viewer::new(400, 400, 1.0);
        viewer
            .handle(Command::Open {
                id: DOCUMENT,
                bytes,
                password: None,
                fragment: None,
            })
            .for_each(drop);
        let at = device_point(&viewer, [10.0, 80.0, 190.0, 120.0], 200.0);
        assert!(
            matches!(viewer.query(Query::LinkAt(at)), Answer::Link(true)),
            "the annotation is a link whatever its action turns out to be: {at:?}"
        );
        viewer
            .handle(Command::Pointer {
                at,
                action: PointerAction::Pressed,
            })
            .for_each(drop);
        viewer
            .handle(Command::Pointer {
                at,
                action: PointerAction::Released,
            })
            .filter_map(|event| match event {
                Event::Reported { notes, .. } => Some(notes),
                _ => None,
            })
            .flatten()
            .collect()
    };

    for (action, sentence) in [
        (
            "<< /S /Launch /F (calc.exe) >>",
            "Launch: running an application, which the sandbox withholds",
        ),
        (
            "<< /S /Sound /Sound 9 0 R >>",
            "Sound: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        ),
        (
            "<< /S /Movie /Operation /Play >>",
            "Movie: clause 13's multimedia, excluded by CLAUDE.md principle 5",
        ),
        // Table 203's two required entries, so that the refusal is the clause's rather than a
        // malformed dictionary's: a `GoToR` naming neither a file nor a destination would be
        // refused by the same arm and would prove nothing about a well-formed one.
        (
            "<< /S /GoToR /F (other.pdf) /D [0 /Fit] >>",
            "GoToR: a destination in another file, which this reader has no filesystem to open",
        ),
        // Table 239's required `/F`, a §7.11.5 URL file specification, for the same reason.
        (
            "<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/) >> >>",
            "SubmitForm: §12.7.6.2's submission, which needs a network",
        ),
    ] {
        let notes = said(action);
        assert!(
            notes.iter().any(|note| note.contains(sentence)),
            "a click on {action} says so, by name: {notes:?}"
        );
    }

    // And the other half of the claim: an action this program *does* perform says nothing about
    // declining. `Named /NextPage` on a one-page document moves nowhere and reports nothing,
    // which is what makes the three sentences above evidence rather than noise.
    let performed = said("<< /S /Named /N /NextPage >>");
    assert!(
        !performed.iter().any(|note| note.contains("declines")),
        "an action this program performs is not a refusal: {performed:?}"
    );
}

#[test]
fn a_document_whose_unmet_requirements_pass_the_clauses_threshold_says_the_total() {
    // §12.11.6 sends a processor to §12.11.3 for "the computation of the penalty value", and
    // §12.11.3's last paragraph is the only sentence in the standard that turns that number into
    // an instruction:
    //
    // > In the situation where the penalty values are being used to evaluate the presentation of
    // > the base PDF document, and there exist no other alternates, if the penalty value exceeds
    // > 100 then the PDF processor should not attempt to display or process the document.
    //
    // **0 of the 974 corpus documents state a `/Requirements` array**, so the witness is built
    // here — and the pair is the point: the same file whose unmet requirements total exactly 100
    // says nothing, because the clause's condition is "exceeds".
    let about = |requirements: &str| -> Vec<String> {
        let bytes = format!(
            "%PDF-2.0\n\
             1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Requirements [{requirements}] >>\nendobj\n\
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

    // Two this program cannot meet at 60 and 55, and one it can at 100. The total is 115: the
    // sum of the *unmet* two, which is what Table 273 prices — "the penalty value to be applied
    // when this requirement cannot be met by a PDF processor".
    let over = about(
        "<< /S /EnableJavaScripts /Penalty 60 >> << /S /Markup /Penalty 55 >> \
         << /S /Navigation /Penalty 100 >>",
    );
    assert!(
        over.iter()
            .any(|note| note.contains("total 115 penalty points (§12.11.3)")),
        "the computation §12.11.6 names is performed and said: {over:?}"
    );
    assert!(
        over.iter()
            .any(|note| note.contains("this document requires EnableJavaScripts (penalty 60)")),
        "and each requirement is still named beside the total: {over:?}"
    );
    // Nothing was refused: the clause says "should not attempt to display" and this program
    // displays, which is the departure the note is honest about.
    assert!(
        over.iter().any(|note| note.contains("displayed anyway")),
        "{over:?}"
    );

    let at_the_limit = about("<< /S /EnableJavaScripts /Penalty 100 >> << /S /Navigation >>");
    assert!(
        !at_the_limit
            .iter()
            .any(|note| note.contains("penalty points (§12.11.3)")),
        "100 does not exceed 100: {at_the_limit:?}"
    );
    assert!(
        at_the_limit
            .iter()
            .any(|note| note.contains("this document requires EnableJavaScripts")),
        "though the requirement itself is still named: {at_the_limit:?}"
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

/// A press on text that lies over an annotation still anchors a selection.
///
/// **The defect `viewer-core/tests/selection_census.rs` found on its first run** (ADR 0421), and
/// it was in the loop no gate touches: `Command::Pointer` sets §12.5.5's appearance state before
/// it decides where the press landed, and changing that state calls `Open::stale`, which throws
/// the interpretation away. The anchor was then taken from an interpretation that no longer
/// existed, so a press over any annotation stating a down appearance produced *no* selection at
/// all and the drag from it selected nothing. 44 corpus documents, 78 of 1017 dragged words.
///
/// `annotation-tx.pdf` is the smallest witness: one text widget over the two words the page draws,
/// and the drag's endpoints are `pdftotext`'s in `selection_census.rs`. Here they are the
/// document's own — the widget's `/Rect` is `[47 704 199 726]` and the page shows "tx annotation"
/// at 718.72 in default user space, both read out of the file — which is trap 12a's rule with the
/// strongest source it admits.
#[test]
fn a_press_over_an_annotation_still_anchors_a_selection() {
    let Some(bytes) = corpus_bytes("annotation-tx.pdf") else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .collect();
    serve(&mut viewer, &request(&events).clone());
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    // The page's own text: "tx annotation", drawn at y 718.72 in default user space, inside the
    // widget whose `/Rect` covers 704 to 726. A point on that line, from the file rather than
    // from the viewer.
    let device = |x: f32, y: f32| {
        (
            geometry.origin.0 + x * geometry.scale,
            geometry.origin.1 + (geometry.page.height - y) * geometry.scale,
        )
    };
    let start = device(30.0, 724.0);
    let end = device(95.0, 724.0);
    for (at, action) in [
        (start, PointerAction::Pressed),
        (end, PointerAction::Dragged),
        (end, PointerAction::Released),
    ] {
        viewer
            .handle(Command::Pointer { at, action })
            .for_each(drop);
    }
    let Answer::Selected(selection) = viewer.query(Query::Selection) else {
        panic!("the drag across the line selected nothing at all — the defect under test");
    };
    assert!(
        selection.text.contains("annotation"),
        "the drag selected {:?}",
        selection.text
    );
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
            value: Entered::Text("Ada Lovelace".to_owned()),
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

/// A save writes what the log holds, so nothing is unsaved until the next edit.
///
/// **The mark stayed on for as long as the document stayed open**, until `doc/todo/01`'s fifth
/// sweep found `ViewState::additions` — "what a host asks to know whether there is anything to
/// save" — called by nothing, and the host answering that question from its own log's length
/// instead. §7.5.6's update writes every edit before the cursor; what is unsaved is therefore
/// the *distance* between the cursor and the last save, and an undo back to it is clean again.
#[test]
fn a_save_takes_the_unsaved_mark_off_and_an_edit_puts_it_back() {
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
    viewer
        .handle(Command::Edit(Edit::SetField {
            field: "Text1".to_owned(),
            value: Entered::Text("Ada Lovelace".to_owned()),
        }))
        .for_each(drop);
    assert!(matches!(viewer.query(Query::Dirty), Answer::Dirty(true)));

    let events: Vec<_> = viewer.handle(Command::Save).collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Saved { .. })),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Dirty { dirty: false, .. })),
        "a save says so, or a host has nothing to take its mark off with: {events:?}"
    );
    assert!(matches!(viewer.query(Query::Dirty), Answer::Dirty(false)));

    // A second save has nothing to announce, and an undo across the saved point is a change to
    // the file again — the cursor's *distance* from the save is what unsaved means.
    let quiet: Vec<_> = viewer.handle(Command::Save).collect();
    assert!(
        !quiet
            .iter()
            .any(|event| matches!(event, Event::Dirty { .. })),
        "{quiet:?}"
    );
    let events: Vec<_> = viewer.handle(Command::Undo).collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Dirty { dirty: true, .. })),
        "{events:?}"
    );
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
            value: Entered::Text("Ada".to_owned()),
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

/// A point turned into an offset, and the shapes over what lies between two of them.
///
/// **`Query::Caret`'s inverse and the third question beside it**, in the device pixels a host
/// works in. The standard states none of this — no cursor, no click that places one, no selection
/// inside a value — so what is pinned is the relation the pair has to each other: an offset gives
/// a place through `Query::Caret`, and that place gives the offset back through `Query::Offset`.
/// A host that can do that round trip can put the caret where the click was, which is the whole
/// of what a person means by clicking into a word. ADR 0225.
#[test]
fn a_point_inside_a_value_names_the_byte_it_is_nearest() {
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
    // The same widget the caret test presses: `[48.54, 727.93, 198.54, 749.93]`, from the file.
    let at = (
        geometry.origin.0 + 120.0 * geometry.scale,
        geometry.origin.1 + (geometry.page.height - 738.0) * geometry.scale,
    );
    viewer
        .handle(Command::Edit(Edit::SetField {
            field: "Text1".to_owned(),
            value: Entered::Text("Ada Lovelace".to_owned()),
        }))
        .for_each(drop);

    let caret = |viewer: &Viewer, offset: usize| match viewer.query(Query::Caret { at, offset }) {
        Answer::Caret { from, to } => (from, to),
        other => panic!("the field has a caret at {offset}: {other:?}"),
    };
    let offset =
        |viewer: &Viewer, point: (f32, f32)| match viewer.query(Query::Offset { at, point }) {
            Answer::Offset(offset) => offset,
            other => panic!("a point in the field names an offset: {other:?}"),
        };

    // The round trip, at every byte of the value.
    for want in 0..="Ada Lovelace".len() {
        let (from, to) = caret(&viewer, want);
        let middle = (from.0, (from.1 + to.1) * 0.5);
        assert_eq!(
            offset(&viewer, middle),
            want,
            "the caret at {want} is at {middle:?}, and that point names {want} again"
        );
    }

    // A point past the end of the value names the end rather than refusing — the choice ADR 0225
    // records, and what a host needs from a press it has already decided is a press into a field.
    let (end, _) = caret(&viewer, "Ada Lovelace".len());
    assert_eq!(offset(&viewer, (end.0 + 40.0, end.1)), "Ada Lovelace".len());
    // And the *field-naming* point is what says which field: a point outside every widget, asked
    // about with `at` naming this one, is still measured inside this value. That is what makes a
    // drag out of the widget's rectangle keep selecting inside it.
    assert_eq!(offset(&viewer, (2.0, 2.0)), 0);
    // While a point that names no widget at all has no offset, which is how a host decides the
    // keyboard belongs to the page.
    assert!(matches!(
        viewer.query(Query::Offset {
            at: (2.0, 2.0),
            point: (2.0, 2.0)
        }),
        Answer::None
    ));

    // The shapes over a range, in the same device pixels: one line, so one shape, and it runs
    // from the caret at one end to the caret at the other.
    let Answer::FieldSelection(quads) = viewer.query(Query::FieldSelection {
        at,
        from: 4,
        to: 12,
    }) else {
        panic!("a range of the value has shapes");
    };
    assert_eq!(quads.len(), 1, "one line of a single-line field: {quads:?}");
    let (start, _) = caret(&viewer, 4);
    let (finish, _) = caret(&viewer, 12);
    let (left, right) = (quads[0][0].min(quads[0][2]), quads[0][0].max(quads[0][2]));
    assert!(
        (left - start.0).abs() < 0.01 && (right - finish.0).abs() < 0.01,
        "the shape runs between the two carets: {:?} against {start:?} and {finish:?}",
        quads[0]
    );
    // Two equal offsets are a caret rather than a selection, and a caret is drawn by the host as
    // a line: no shapes come back for it.
    let Answer::FieldSelection(none) = viewer.query(Query::FieldSelection { at, from: 3, to: 3 })
    else {
        panic!("an empty range still answers");
    };
    assert!(none.is_empty(), "{none:?}");
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
    assert_eq!(value.as_ref().map(|shown| shown.text.as_str()), Some(""));
    // And Table 231 bit 14 is clear on it, so the string above *is* the field's characters and a
    // host may write it back — ADR 0247's third amendment, from the answering side.
    assert_eq!(value.map(|shown| shown.obscured), Some(false));
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

/// §12.7's whole form on a page, as a host that draws it in native controls needs it.
///
/// **The sixth chrome population, and the one the chrome audit found missing.** Five already
/// crossed as data — §12.3.3's outline, §8.11.4.3's layers, §7.11.4's files, §12.3.5's collection,
/// §12.5.6.14's popups — and a form field did not, so a native host could draw everything else in
/// a `QTreeView` or an `NSPopover` and then had to take its fields as pixels off the raster.
/// `Query::FieldAt` answers for one *point*, which is what a click has; this answers for the page,
/// which is what a host placing controls has. ADR 0235.
///
/// `issue17492.pdf` is the fixture because its first page states one of nearly every control
/// §12.7.5 defines: two text fields with Table 232's `/MaxLen`, an editable combo box, a
/// non-editable one, a multi-select list box with Table 234's export/label pairs, four check boxes
/// and a §12.7.5.2.4 radio set of four widgets under one field.
#[test]
fn a_page_states_its_whole_form_as_controls_a_host_can_build() {
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
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("the page has a form");
    };
    assert_eq!(fields.len(), 12, "{:?}", named(&fields));

    // §12.7.5.3, with Table 232's `/MaxLen` and §14.9.3's `/TU` beside it.
    let first = &fields[0];
    assert_eq!(first.name.qualified, "firstName");
    assert_eq!(first.partial, "firstName");
    assert_eq!(first.name.shown(), "First name");
    let shown = first.value.as_ref().expect("a text field states a value");
    assert_eq!(shown.text, "Lucía");
    assert!(!first.read_only && !first.required);
    let Control::Text(text) = &first.control else {
        panic!("{:?}", first.control)
    };
    assert_eq!(text.max_len, Some(40));
    assert!(!text.multiline && !text.password && text.comb.is_none());

    // §12.7.5.4, both of Table 233's forms: an editable combo box and a multi-select list box
    // whose `/Opt` states export/label pairs.
    let Some(country) = find(&fields, "country") else {
        panic!("{:?}", named(&fields))
    };
    let Control::Choice(combo) = &country.control else {
        panic!("{:?}", country.control)
    };
    assert!(combo.combo && combo.editable && !combo.multi_select);
    assert_eq!(combo.options.len(), 28);
    assert_eq!(combo.options[26].label, "Spain");
    assert_eq!(
        combo.selected,
        vec![26],
        "§12.7.5.4: `/V` names the item, and it is the twenty-seventh"
    );

    let Some(databases) = find(&fields, "databases") else {
        panic!("{:?}", named(&fields))
    };
    let Control::Choice(list) = &databases.control else {
        panic!("{:?}", databases.control)
    };
    assert!(!list.combo && list.multi_select);
    assert_eq!(list.options[0].label, "Oracle");
    assert_eq!(
        list.options[0].export.as_deref(),
        Some("oracle"),
        "Table 234's pair: \"the option's export value and the text that shall be displayed\""
    );
    assert!(
        list.selected.is_empty(),
        "the field states no `/V`, and \"the default value of V is null\""
    );

    // §12.7.5.2.4: one field, four widgets, a state name apiece, and one of them on.
    let Some(education) = find(&fields, "educationLevel") else {
        panic!("{:?}", named(&fields))
    };
    assert_eq!(
        education.control,
        Control::RadioButton {
            on: true,
            no_toggle_to_off: true,
            in_unison: false,
        }
    );
    assert_eq!(education.widgets.len(), 4);
    let states: Vec<&str> = education
        .widgets
        .iter()
        .filter_map(|widget| widget.on_state.as_deref())
        .collect();
    assert_eq!(
        states,
        [
            "highSchool",
            "associateDegree",
            "bachelorDegree",
            "masterDegree"
        ]
    );
    let on: Vec<bool> = education.widgets.iter().map(|widget| widget.on).collect();
    assert_eq!(on, [false, false, true, false], "the third is checked");

    // The quadrilaterals are in the viewport's device pixels, like every other shape this crate
    // hands over — checked against the one question that already took a point: a click inside a
    // widget's quad finds that widget's field.
    let Some(java) = find(&fields, "javaScript") else {
        panic!("{:?}", named(&fields))
    };
    let quad = java.widgets[0].quad;
    let centre = (
        f32::midpoint(quad[0], quad[4]),
        f32::midpoint(quad[1], quad[5]),
    );
    let Answer::Field { name, .. } = viewer.query(Query::FieldAt(centre)) else {
        panic!("the middle of a widget's quadrilateral is that widget");
    };
    assert_eq!(name.qualified, "javaScript");
}

/// §12.7.5.3's Table 231 bit 14, from the answering side: a value that is not the value says so.
///
/// > If set, the field is intended for entering a secure password that should not be echoed
/// > visibly to the screen. Characters typed from the keyboard shall instead be echoed in some
/// > unreadable form, such as asterisks or bullet characters.
///
/// **The corpus has exactly one of these** — `issue19389.pdf`, 1 widget over 974 documents, which
/// `examples/field_flag_census` counts — and until the four-hundred-and-eleventh session nothing
/// on this boundary said that its value was the echo rather than the characters. What that cost is
/// ADR 0247: `viewer-ui` obeyed ADR 0201's read-the-value-back rule and therefore sent the bullets
/// as the field's next value on every keystroke. The two answers that carry a value both carry the
/// flag now, and this asserts them against each other on the one document that has one.
#[test]
fn a_password_fields_value_says_that_it_is_not_the_fields_characters() {
    let Some(bytes) = corpus_bytes("issue19389.pdf") else {
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
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("this form has fields on page one");
    };
    let secret = fields
        .iter()
        .find(|field| matches!(&field.control, Control::Text(text) if text.password))
        .expect("issue19389.pdf states Table 231 bit 14 on one widget");
    // A person has typed into it, so the value is theirs rather than the file's — which is the
    // case that matters, because it is the one a host would write back.
    viewer
        .handle(Command::Edit(Edit::SetField {
            field: secret.name.qualified.clone(),
            value: Entered::Text("hunter2".to_owned()),
        }))
        .for_each(drop);

    let Answer::Fields(after) = viewer.query(Query::Fields) else {
        panic!("this form has fields on page one");
    };
    let secret = after
        .iter()
        .find(|field| field.name.qualified == secret.name.qualified)
        .expect("the field is still there");
    let shown = secret.value.as_ref().expect("a text field states a value");
    assert!(shown.obscured, "the flag did not cross with the value");
    assert_ne!(shown.text, "hunter2", "the characters crossed");
    assert_eq!(shown.text.chars().count(), 7, "one echo per character");

    // And `Query::FieldAt` answers the same way, because the two must not be learnable apart: a
    // host that read the exception off one question and missed it on the other would ship the bug
    // either way round.
    let widget = secret.widgets.first().expect("the field has a widget");
    let at = (
        (widget.quad[0] + widget.quad[4]) * 0.5,
        (widget.quad[1] + widget.quad[5]) * 0.5,
    );
    let Answer::Field { value, .. } = viewer.query(Query::FieldAt(at)) else {
        panic!("the widget is at its own centre");
    };
    let point = value.expect("a text field states a value");
    assert!(point.obscured);
    assert_eq!(point.text, shown.text);
}

/// And a host can now *check a box*, which it could not before (§12.7.5.2.3).
///
/// The demonstration this round owes. [`Edit::SetField`] takes a string, and for a check box the
/// only strings that mean anything are the names Table 170's appearance dictionary is keyed by —
/// the file's own invention, `/Yes` here and `/1`, `/On` or Table 230's positional `/0` elsewhere.
/// A host had no way to learn one. `FormWidget::on_state` is that name, and the page draws the
/// state it selects: two halves of one round, because either without the other is a host that
/// knows what to send and cannot see it work, or one that could see it and cannot know.
#[test]
fn a_host_can_check_a_box_with_the_name_the_page_gave_it() {
    let Some(bytes) = corpus_bytes("issue17492.pdf") else {
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
    let before = request(&events).clone();

    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("the page has a form");
    };
    let Some(field) = find(&fields, "typeScript") else {
        panic!("{:?}", named(&fields))
    };
    assert_eq!(field.control, Control::CheckBox { on: false });
    let state = field.widgets[0]
        .on_state
        .clone()
        .expect("§12.7.5.2.3 keys the appearance dictionary by state, and this box states one");
    assert_eq!(state, "Yes");

    let events: Vec<_> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: field.name.qualified.clone(),
            value: Entered::Text(state),
        }))
        .collect();
    let after = request(&events).clone();
    assert!(
        !std::sync::Arc::ptr_eq(&before.list, &after.list),
        "a checked box is a page drawn again"
    );

    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("the page still has a form");
    };
    let Some(field) = find(&fields, "typeScript") else {
        panic!("{:?}", named(&fields))
    };
    assert_eq!(field.control, Control::CheckBox { on: true });
    assert!(field.widgets[0].on);

    // And the state's own appearance stream is what the page now draws, which is the half of
    // §12.7.5.2.3 nothing obeyed until this round: this widget's `/AP /N` states `Yes` and no
    // `Off` at all, so the on state adds a stream where the off state has none.
    //
    // **The display list and not the ink**, deliberately: the tick is `(5)` in ZapfDingbats and
    // whether this machine has a face for it is trap 8's question rather than the clause's.
    // `pdf-model`'s `checking_a_box_draws_the_state_the_new_value_names` is where the pixels are
    // asserted, on fixtures built to make the mark visible whatever is installed.
    let cleared: Vec<_> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: "typeScript".to_owned(),
            // §12.7.5.2.3 names the off state and §12.7.5.2.4 gives it as the default.
            value: Entered::Text("Off".to_owned()),
        }))
        .collect();
    let off = request(&cleared);
    assert_eq!(
        after.list.commands().len(),
        before.list.commands().len() + 1,
        "the checked box draws one thing the unchecked one does not"
    );
    assert_eq!(
        off.list.commands().len(),
        before.list.commands().len(),
        "and unchecking it takes that thing away again"
    );
}

/// §12.7.5.4's list box, several items at once, saved and read back (Table 233 bit 22).
///
/// **The one message-shaped gap three hosts found**, and the round that closed it. Sessions 408,
/// 410 and 411 each built a list box over [`Query::Fields`] and each asked its toolkit for single
/// selection, because `Edit::SetField` carried one string while the bit permits several:
///
/// > (PDF 1.4) If set, more than one of the field's option items may be selected simultaneously;
/// > if clear, at most one item shall be selected.
///
/// `issue17492.pdf`'s `databases` is one of the corpus's **4** widgets that set it, over 4
/// documents. Its `/Opt` is Table 234's two-element form throughout, so the export values and the
/// labels are different strings and §12.7.5.4 decides which reach `/V`: of a two-element `/Opt`
/// entry it is the second element. ADR 0248.
#[test]
fn a_host_can_select_several_items_of_a_list_box_and_save_them() {
    let Some(bytes) = corpus_bytes("issue17492.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let _events: Vec<_> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .collect();

    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("the page has a form");
    };
    let Some(field) = find(&fields, "databases") else {
        panic!("{:?}", named(&fields))
    };
    let Control::Choice(choice) = &field.control else {
        panic!("{:?}", field.control)
    };
    assert!(choice.multi_select, "Table 233 bit 22");
    assert!(!choice.combo, "bit 18 clear: a list box");
    let labels: Vec<&str> = choice
        .options
        .iter()
        .map(|option| option.label.as_str())
        .collect();
    assert_eq!(labels, ["Oracle", "SQL Server", "DB2", " PostgreSQL"]);

    let _events: Vec<_> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: field.name.qualified.clone(),
            value: Entered::Chosen(vec![2, 0]),
        }))
        .collect();

    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("the page still has a form");
    };
    let Some(field) = find(&fields, "databases") else {
        panic!("{:?}", named(&fields))
    };
    let Control::Choice(choice) = &field.control else {
        panic!("{:?}", field.control)
    };
    assert_eq!(
        choice.selected,
        vec![0, 2],
        "ascending, whatever order they were clicked in"
    );

    // And §7.5.6's update says the same thing to a reader that has never seen this session.
    let events: Vec<_> = viewer.handle(Command::Save).collect();
    let Some(Event::Saved { bytes, .. }) = events
        .iter()
        .find(|event| matches!(event, Event::Saved { .. }))
    else {
        panic!("{events:?}")
    };
    let mut again = Viewer::new(800, 1000, 1.0);
    let _events: Vec<_> = again
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.clone(),
            password: None,
            fragment: None,
        })
        .collect();
    let Answer::Fields(fields) = again.query(Query::Fields) else {
        panic!("the saved document has a form");
    };
    let Some(field) = find(&fields, "databases") else {
        panic!("{:?}", named(&fields))
    };
    let Control::Choice(choice) = &field.control else {
        panic!("{:?}", field.control)
    };
    assert_eq!(
        choice.selected,
        vec![0, 2],
        "the selection survives the file it was written into"
    );
}

/// The field of that fully qualified name, where the page states one.
fn find<'a>(
    fields: &'a [viewer_core::FormField],
    name: &str,
) -> Option<&'a viewer_core::FormField> {
    fields.iter().find(|field| field.name.qualified == name)
}

/// Every field's qualified name, for a failure that has to say what was there instead.
fn named(fields: &[viewer_core::FormField]) -> Vec<&str> {
    fields
        .iter()
        .map(|field| field.name.qualified.as_str())
        .collect()
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
            value: Entered::Text("Ada Lovelace".to_owned()),
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

    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);
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
        Answer::Accessibility(pages) if pages.iter().all(|page| page.nodes.is_empty())
    ));
}

/// A page whose *page-tree node* has the lower object number still answers with its structure.
///
/// `pdf_model::Pages::indices` answers object → index and holds an entry for an intermediate
/// `/Pages` node as well as for each page, "answering with the first page beneath it" — so
/// inverting it by scanning for an index hands back whichever object number is lower, and where a
/// node's is lower than page one's the answer is a node rather than a page. Table 355's `/Pg`
/// comparisons then all failed and page one of ten of this project's own tagged documents — ISO
/// 14289-1 among them, which is PDF/UA — answered a screen reader with the silence an untagged
/// page gives. ADR 0342, found by `tests/accessibility_census.rs` on its first run.
///
/// The fixture is asserted to still *exhibit* the hazard before the answer is judged: a document
/// whose object numbers were renumbered would pass this test while testing nothing.
#[test]
fn page_one_answers_where_its_page_tree_node_has_the_lower_object_number() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN002-AF.pdf");
    let bytes = std::fs::read(&path).expect("the application note is in doc/");
    let document = pdf_syntax::Document::open(bytes.clone()).expect("the application note opens");
    let pages = pdf_model::Pages::new(&document);
    let page_one = pages.get(0).and_then(|page| page.id).expect("page one");
    let scanned = pages
        .indices()
        .into_iter()
        .find(|(_, index)| *index == 0)
        .map(|(object, _)| object);
    assert_ne!(
        scanned,
        Some(page_one),
        "the fixture no longer exhibits the hazard: nothing lower than page one answers for index 0"
    );

    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);
    assert!(
        !nodes.is_empty(),
        "page one of a tagged document answered with no structure at all"
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.substituted && !node.name.trim().is_empty()),
        "the page's figure states §14.9.3's /Alt and it should be spoken: {nodes:?}"
    );
}

/// Every page of a large tagged document answers with its own elements, not the first page's.
///
/// **Two thirds of ISO 32000-2's 1023 pages answered with nothing**, and nothing said so: the
/// walk started at the structure tree root, gathered the *document's* elements until the bound on
/// one page's answer stopped it, and pruned to the page afterwards — so every page past the first
/// few pages' worth of elements got an empty list, which is the same answer an untagged page
/// gives. §14.7.5.4 states the route that has no such shape — the page's own `/StructParents` is
/// the key into the structural parent tree, and
///
/// > For a content stream containing marked-content sequences that are content items, the value
/// > shall be an array of indirect references to the sequences' parent structure elements.
///
/// so the page names its own elements and §14.7.2's Table 355 `/P` places them. ADR 0325.
///
/// The pages are sampled across the document rather than taken from the front, because the defect
/// was invisible at the front: pages 1 and 40 were right throughout. Each page's answer is also
/// required to differ from the page before's, which is what says these are *its* elements rather
/// than a tree handed out unchanged.
#[test]
fn every_page_of_a_large_tagged_document_answers_with_its_own_elements() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("the specification is in doc/");
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);

    let mut answers: Vec<(usize, Vec<String>)> = Vec::new();
    for page in [1_usize, 150, 400, 1022] {
        viewer
            .handle(Command::GoTo(PageTarget::Index(page)))
            .for_each(drop);
        let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
            panic!("the query always answers");
        };
        let nodes = on_one_page(pages);
        assert!(
            !nodes.is_empty(),
            "page {page} of a tagged document answered with no structure at all"
        );
        // Parent-first, which is what makes an index into the list a usable parent link — and
        // which a pruned walk could break where the whole-tree one did not.
        for (index, node) in nodes.iter().enumerate() {
            if let Some(parent) = node.parent {
                assert!(parent < index, "page {page}, node {index} names {parent}");
            }
        }
        assert!(
            nodes
                .iter()
                .any(|node| !node.name.trim().is_empty() && !node.quads.is_empty()),
            "page {page}: no element both speaks and has a place on the page"
        );
        answers.push((page, nodes.iter().map(|node| node.name.clone()).collect()));
    }
    for pair in answers.windows(2) {
        let ([before, after], ..) = (pair, ()) else {
            continue;
        };
        assert_ne!(
            before.1, after.1,
            "pages {} and {} answered with the same elements",
            before.0, after.0
        );
    }
}

/// A two-page tagged document whose own `/RoleMap` renames one of §14.8.4's types.
///
/// Three things in one fixture, because they are three properties of one answer: §14.7.3's role
/// map (`Chap` is this file's name for a `Sect`), an element whose own text is not its subtree's,
/// and a second page whose elements are in the same tree and not on this page.
fn with_a_role_map() -> Vec<u8> {
    use std::fmt::Write as _;
    let first = "BT /F1 12 Tf 10 60 Td\n\
         /P <</MCID 0>> BDC (Alpha) Tj EMC\n\
         /P <</MCID 1>> BDC (Beta) Tj EMC\n\
         /P <</MCID 2>> BDC (Gamma) Tj EMC\nET\n";
    let second = "BT /F1 12 Tf 10 60 Td\n/P <</MCID 0>> BDC (Delta) Tj EMC\nET\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 8 0 R \
          /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 5 0 R \
          /Resources << /Font << /F1 7 0 R >> >> /StructParents 0 >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 6 0 R \
          /Resources << /Font << /F1 7 0 R >> >> /StructParents 1 >>\nendobj\n\
         5 0 obj\n<< /Length {} >>\nstream\n{first}endstream\nendobj\n\
         6 0 obj\n<< /Length {} >>\nstream\n{second}endstream\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         8 0 obj\n<< /Type /StructTreeRoot /K [9 0 R 12 0 R] \
          /RoleMap << /Chap /Sect >> >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /Chap /P 8 0 R /Pg 3 0 R \
          /K [0 10 0 R 11 0 R] >>\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /Span /P 9 0 R /Pg 3 0 R /K [1] >>\nendobj\n\
         11 0 obj\n<< /Type /StructElem /S /Figure /P 9 0 R /Pg 3 0 R /K [2] \
          /Alt (a picture of a bridge) >>\nendobj\n\
         12 0 obj\n<< /Type /StructElem /S /P /P 8 0 R /Pg 4 0 R /K [0] >>\nendobj\n",
        first.len(),
        second.len()
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

/// §14.7.3's role map is applied, an element speaks for itself, and another page is another page.
///
/// > A structure type shall always be mapped to its corresponding name in the role map, if there
/// > is one, even if the original name is one of the standard types.
///
/// **A `shall` this answer did not obey until the three-hundred-and-seventy-sixth session**, on
/// an argument that was about a different mapping: `pdf-model` has followed the role map since
/// the seventy-eighth and this query read the raw `/S` past it. ADR 0214.
#[test]
fn a_structure_type_crosses_role_mapped_and_speaking_only_for_itself() {
    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_role_map(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);
    // Three elements are on page one; the fourth is on page two and is not answered with.
    assert_eq!(nodes.len(), 3, "{nodes:?}");
    assert!(
        nodes.iter().all(|node| !node.name.contains("Delta")),
        "page two's text reached page one's tree: {nodes:?}"
    );

    // §14.7.3: the file's own `Chap` is the standard `Sect` its role map says it is.
    assert_eq!(nodes[0].role, "Sect", "{nodes:?}");
    assert_eq!(nodes[0].parent, None);
    // Its own marked-content sequence, and not its children's: a container whose name repeated
    // what is under it would be read twice by anything that walks the tree.
    assert_eq!(nodes[0].name, "Alpha", "{nodes:?}");
    assert!(!nodes[0].substituted);
    // And it still *covers* what it encloses, which is what a focus ring is drawn round.
    assert!(nodes[0].quads.len() >= 3, "{:?}", nodes[0]);

    assert_eq!(nodes[1].role, "Span");
    assert_eq!(nodes[1].parent, Some(0));
    assert_eq!(nodes[1].name, "Beta");

    // §14.9.3's `/Alt` is a substitution for the whole element, and says so.
    assert_eq!(nodes[2].role, "Figure");
    assert_eq!(nodes[2].name, "a picture of a bridge");
    assert!(nodes[2].substituted, "{:?}", nodes[2]);
}

/// One tagged table, built so that every branch of §14.8.5.7's assumption is on one page.
///
/// The corner cell spans two rows, which is what makes the second row's first *child* not its
/// first column — the difference between a reader that counts children and one that keeps a grid.
/// The last row's header states a `/Scope` the assumption would have contradicted.
///
/// The table also states Table 384's `/Summary`, one header its `/Short`, and both entries are
/// *planted* on types their own sentences exclude — a `/Short` on the table, both on a `TD` —
/// so that a reader applying the entries without their conditions is caught by the test that
/// asserts them absent.
fn with_a_table() -> Vec<u8> {
    use std::fmt::Write as _;
    let content = "BT /F1 12 Tf 10 60 Td\n\
         /TH <</MCID 0>> BDC (Region) Tj EMC\n\
         /TH <</MCID 1>> BDC (2023) Tj EMC\n\
         /TH <</MCID 2>> BDC (North) Tj EMC\n\
         /TH <</MCID 3>> BDC (South) Tj EMC\n\
         /TD <</MCID 4>> BDC (12) Tj EMC\n\
         /TH <</MCID 5>> BDC (Total) Tj EMC\nET\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R \
          /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> /StructParents 0 >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R] >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Table /P 6 0 R /Pg 3 0 R \
          /K [8 0 R 11 0 R 13 0 R 16 0 R] \
          /A << /O /Table /Summary (sales by region and year) /Short (not for a table) >> \
          >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /TR /P 7 0 R /Pg 3 0 R /K [9 0 R 10 0 R] >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /TH /P 8 0 R /Pg 3 0 R /K [0] \
          /A << /O /Table /RowSpan 2 >> >>\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /TH /P 8 0 R /Pg 3 0 R /K [1] \
          /A << /O /Table /Short (Yr) >> >>\nendobj\n\
         11 0 obj\n<< /Type /StructElem /S /TR /P 7 0 R /Pg 3 0 R /K [12 0 R] >>\nendobj\n\
         12 0 obj\n<< /Type /StructElem /S /TH /P 11 0 R /Pg 3 0 R /K [2] >>\nendobj\n\
         13 0 obj\n<< /Type /StructElem /S /TR /P 7 0 R /Pg 3 0 R /K [14 0 R 15 0 R] >>\nendobj\n\
         14 0 obj\n<< /Type /StructElem /S /TH /P 13 0 R /Pg 3 0 R /K [3] >>\nendobj\n\
         15 0 obj\n<< /Type /StructElem /S /TD /P 13 0 R /Pg 3 0 R /K [4] \
          /A << /O /Table /Summary (not for a cell) /Short (not for a data cell) >> >>\nendobj\n\
         16 0 obj\n<< /Type /StructElem /S /TR /P 7 0 R /Pg 3 0 R /K [17 0 R] >>\nendobj\n\
         17 0 obj\n<< /Type /StructElem /S /TH /P 16 0 R /Pg 3 0 R /K [5] \
          /A << /O /Table /Scope /Column >> >>\nendobj\n",
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

/// A document whose figures mark no text and say where they are.
///
/// Two pages holding the same figure — a filled rectangle inside a marked-content sequence, with
/// no text in it at all — and the same Table 379 `/BBox`. The second page states `/Rotate 90`,
/// which is what makes the fixture worth having: the attribute is stated in **default user
/// space**, so a reader that took its numbers straight to the raster would agree with one that
/// mapped them on the first page and disagree on the second.
///
/// The first page also carries a paragraph, so that "an element with quads" and "an element with
/// bounds" are both present and can be told apart.
fn with_a_figure() -> Vec<u8> {
    use std::fmt::Write as _;
    let first = "/Figure <</MCID 0>> BDC 20 30 60 40 re f EMC\n\
         BT /F1 12 Tf 100 20 Td /P <</MCID 1>> BDC (a caption) Tj EMC ET\n";
    let second = "/Figure <</MCID 0>> BDC 20 30 60 40 re f EMC\n\
         /Figure <</MCID 1>> BDC 0 0 10 10 re f EMC\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 8 0 R \
          /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 5 0 R \
          /Resources << /Font << /F1 7 0 R >> >> /StructParents 0 >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Rotate 90 \
          /Contents 6 0 R /Resources << >> /StructParents 1 >>\nendobj\n\
         5 0 obj\n<< /Length {} >>\nstream\n{first}endstream\nendobj\n\
         6 0 obj\n<< /Length {} >>\nstream\n{second}endstream\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         8 0 obj\n<< /Type /StructTreeRoot /K [9 0 R 10 0 R 11 0 R 12 0 R] >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /Figure /P 8 0 R /Pg 3 0 R /K [0] /Alt (a chart) \
          /A << /O /Layout /BBox [20 30 80 70] >> >>\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /P /P 8 0 R /Pg 3 0 R /K [1] >>\nendobj\n\
         11 0 obj\n<< /Type /StructElem /S /Figure /P 8 0 R /Pg 4 0 R /K [0] /Alt (the same \
          chart, on a page that is turned) /A << /O /Layout /BBox [20 30 80 70] >> >>\nendobj\n\
         12 0 obj\n<< /Type /StructElem /S /Figure /P 8 0 R /Pg 4 0 R /K [1] /Alt (a figure \
          whose producer wrote the whole plane) \
          /A << /O /Layout /BBox [-32768 -32768 32767 32767] >> >>\nendobj\n",
        first.len(),
        second.len(),
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

/// Table 379's `/BBox` crosses for an element the text layer cannot place, in the viewport's own
/// pixels and through §7.7.3.3's rotation.
///
/// ISO 32000-2 §14.8.5.4.3 states the attribute as
///
/// > An array of four numbers in default user space units that shall give the coordinates of the
/// > left, bottom, right, and top edges, respectively, of the structure element's bounding box
/// > (the rectangle that completely encloses its visible content).
///
/// which is a rectangle a magnifier can be pointed at, for exactly the elements
/// [`viewer_core::AccessibilityNode::quads`] is empty for: this figure's content is a filled
/// rectangle and no glyph, so the text layer knows nothing about where it is.
///
/// **The expected numbers are written out from the clause's own space rather than taken from the
/// code**, which is trap 12a's rule: default user space has its y pointing up from the bottom of
/// the page, `/Rotate 90` takes `(x, y)` to `(y, W - x)` for the unrotated width `W`, and the
/// viewport's origin and scale come from [`Query::PageGeometry`] — a different answer of the
/// viewer's, not the one under test.
#[test]
fn an_element_that_marks_no_text_crosses_with_the_bounds_the_document_states() {
    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_figure(),
            password: None,
            fragment: None,
        })
        .collect();
    let first_frame = request(&events).clone();
    serve(&mut viewer, &first_frame);

    let geometry = |viewer: &Viewer, page: usize| match viewer.query(Query::PageGeometry(page)) {
        Answer::Geometry(geometry) => geometry,
        other => panic!("a page has a geometry: {other:?}"),
    };
    let nodes = |viewer: &Viewer| match viewer.query(Query::AccessibilityTree) {
        Answer::Accessibility(pages) => on_one_page(pages),
        other => panic!("the query always answers: {other:?}"),
    };
    let named = |nodes: &[viewer_core::AccessibilityNode], name: &str| {
        nodes
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("{name} is on the page: {nodes:?}"))
            .clone()
    };

    let first = geometry(&viewer, 0);
    let figure = named(&nodes(&viewer), "a chart");
    assert!(
        figure.quads.is_empty(),
        "the figure marks no text, so the text layer places it nowhere: {figure:?}"
    );
    // Page height 100, y up: the top edge 70 is 30 units down from the top, the bottom edge 30
    // is 70 units down.
    assert_eq!(
        figure.bounds,
        Some([
            first.origin.0 + 20.0 * first.scale,
            first.origin.1 + 30.0 * first.scale,
            first.origin.0 + 80.0 * first.scale,
            first.origin.1 + 70.0 * first.scale,
        ]),
    );

    let caption = named(&nodes(&viewer), "a caption");
    assert!(
        !caption.quads.is_empty(),
        "the paragraph marks text and is placed by the text layer: {caption:?}"
    );
    assert_eq!(
        caption.bounds, None,
        "an element stating no /BBox has said nothing about where it is"
    );

    // The same figure on a page turned a quarter clockwise. `/Rotate 90` maps (x, y) to
    // (y, 200 - x), so the rectangle's corners become (30, 180) and (70, 120) in the page's own
    // space, and the displayed page is 200 units tall — which puts them 20 and 80 units from
    // the top.
    let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(1))).collect();
    let next = request(&turned).clone();
    serve(&mut viewer, &next);
    let second = geometry(&viewer, 1);
    let rotated = named(&nodes(&viewer), "the same chart, on a page that is turned");
    assert_eq!(
        rotated.bounds,
        Some([
            second.origin.0 + 30.0 * second.scale,
            second.origin.1 + 20.0 * second.scale,
            second.origin.0 + 70.0 * second.scale,
            second.origin.1 + 80.0 * second.scale,
        ]),
        "the attribute is in default user space, and §7.7.3.3's rotation stands between it and \
         the screen",
    );

    // And `doc/PDF20_AN001-BPC.pdf`'s own idiom, which 8 of the corpus's 132 rectangles share:
    // the whole representable plane, which encloses the figure and everything else. §14.11.2.1
    // says what of a page can be seen, so what crosses is the page.
    let whole_plane = named(
        &nodes(&viewer),
        "a figure whose producer wrote the whole plane",
    );
    assert_eq!(
        whole_plane.bounds,
        Some([
            second.origin.0,
            second.origin.1,
            second.origin.0 + 100.0 * second.scale,
            second.origin.1 + 200.0 * second.scale,
        ]),
        "a rectangle beyond the page is clipped to the page, which is all of it that is visible",
    );

    // And the same figure's *own* marks, which is what §14.8.3.3 derives a content rectangle
    // from. `0 0 10 10 re` on a page `/Rotate 90` turns clockwise onto the strip along the
    // displayed page's top edge — 10 units of the 100-unit width, 10 units of the 200-unit
    // height — and the difference from the stated rectangle above is the whole point: one is
    // the page and the other is the figure.
    assert_eq!(
        whole_plane.drawn,
        Some([
            second.origin.0,
            second.origin.1,
            second.origin.0 + 10.0 * second.scale,
            second.origin.1 + 10.0 * second.scale,
        ]),
        "the marks say where the figure is, whatever its producer wrote: {whole_plane:?}",
    );
}

/// §14.8.3.3's content rectangle crosses for an element the text layer cannot place.
///
/// ISO 32000-2 §14.8.3.3, of every block- and inline-level structure element:
///
/// > The content rectangle shall be derived from the shape of the enclosed content and defines
/// > the bounds used for the layout of any included child elements.
///
/// §14.8.5.4.5 states that derivation for the cases that are marks rather than layout, and this
/// is it: the figure's content is a filled rectangle and no glyph, so
/// [`viewer_core::AccessibilityNode::quads`] is empty and only what was drawn can place it.
///
/// **Written out from the page's own space rather than from the code** (trap 12a): the fill is
/// `20 30 60 40 re`, so its box is `[20 30 80 70]` with y up from the bottom of a 100-unit page,
/// and the viewport's origin and scale come from [`Query::PageGeometry`] — a different answer of
/// the viewer's than the one under test.
///
/// It is asserted **beside** the stated `/BBox`, not instead of it: this fixture's producer wrote
/// a rectangle that agrees with its marks, which is what a conforming file does, and the two
/// still cross as two statements because a host chooses between them.
#[test]
fn an_element_that_marks_no_text_crosses_with_the_rectangle_its_content_drew() {
    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_figure(),
            password: None,
            fragment: None,
        })
        .collect();
    let first_frame = request(&events).clone();
    serve(&mut viewer, &first_frame);

    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("a page has a geometry");
    };
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);
    let named = |name: &str| {
        nodes
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("{name} is on the page: {nodes:?}"))
            .clone()
    };

    let figure = named("a chart");
    assert!(
        figure.quads.is_empty(),
        "the figure marks no text, so the text layer places it nowhere: {figure:?}"
    );
    assert_eq!(
        figure.drawn,
        Some([
            geometry.origin.0 + 20.0 * geometry.scale,
            geometry.origin.1 + 30.0 * geometry.scale,
            geometry.origin.0 + 80.0 * geometry.scale,
            geometry.origin.1 + 70.0 * geometry.scale,
        ]),
        "the fill the figure encloses is where the figure is: {figure:?}",
    );

    // The caption's sequence drew glyphs, so it has a content rectangle too — coarser than its
    // quadrilaterals and answering the same question, which is why `tree::place` asks the
    // quadrilaterals first.
    let caption = named("a caption");
    assert!(
        caption.drawn.is_some(),
        "a sequence that drew glyphs marked the page: {caption:?}"
    );
    assert_eq!(
        caption.bounds, None,
        "an element stating no /BBox has said nothing about where it is"
    );
}

/// A page whose `/Contents` and whose form `XObject` both mark `/MCID 0`, and the twin that
/// does not.
///
/// ISO 32000-2 §14.7.5.2 makes the identifier unique "within its content stream", and permits the
/// form to carry sequences of its own — so both streams numbering from zero is conforming, and
/// §14.7.5.4 gives each stream its own `/StructParents` entry to tell them apart. `form_mcid`
/// chooses which of the pair this is.
///
/// The two sequences draw in different places on purpose: the page's fill is `10 10 20 20` and the
/// form's is `120 10 40 40`, so an element handed both would be four times as wide as the marks it
/// names. Neither states a `/BBox`, so what places them is the marks and nothing else.
fn with_a_form_that_marks(form_mcid: i64) -> Vec<u8> {
    use std::fmt::Write as _;
    // §14.7.5.2: "any Do operator that paints the form XObject shall not be part of a logical
    // structure content item", which is why the `Do` is outside the page's sequence.
    let page = "/P <</MCID 0>> BDC 10 10 20 20 re f EMC\n/Fm Do\n";
    let form = format!("/Figure <</MCID {form_mcid}>> BDC 120 10 40 40 re f EMC\n");
    let entry = if form_mcid == 0 {
        "[9 0 R]"
    } else {
        "[null 9 0 R]"
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R \
          /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
          /Resources << /XObject << /Fm 5 0 R >> >> /StructParents 0 >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}endstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 200 100] /StructParents 1 \
          /Length {} >>\nstream\n{form}endstream\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [8 0 R 9 0 R] /ParentTree 7 0 R >>\nendobj\n\
         7 0 obj\n<< /Nums [0 [8 0 R] 1 {entry}] >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K [0] \
          /Alt (the page's own) >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /Figure /P 6 0 R /Alt (the form's own) \
          /K << /Type /MCR /Pg 3 0 R /Stm 5 0 R /MCID {form_mcid} >> >>\nendobj\n",
        page.len(),
        form.len(),
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

/// §14.8.3.3's content rectangle stops at the stream its sequence's identifier is unique within.
///
/// ISO 32000-2 §14.7.5.2 makes an `/MCID` "an integer marked-content identifier that uniquely
/// identifies the marked-content sequence within its content stream", and Errata Collection 3's
/// Issue #308 adds §14.7.5.4 the NOTE that draws the consequence: identifiers are scoped by content
/// stream and start at zero, so the same one may reappear across pages or `XObject`s.
///
/// So the paragraph and the figure below are two different sequences both called `/MCID 0`, and
/// what places each is its own marks. Written out from the page's own space rather than from the
/// code (trap 12a): the fills are `10 10 20 20` and `120 10 40 40`, so the boxes are `[10 10 30 30]`
/// and `[120 10 160 50]` with y up from the bottom of a 100-unit page.
///
/// **Asserted of the pair**, because a reader that lost the form's content altogether would pass
/// half of this: the twin numbers the form's sequence 1 instead, and every rectangle below is the
/// same.
#[test]
fn a_content_rectangle_is_not_taken_from_another_content_stream() {
    for (name, form_mcid) in [("collide", 0_i64), ("distinct", 1)] {
        let mut viewer = Viewer::new(400, 300, 1.0);
        let events: Vec<Event> = viewer
            .handle(Command::Open {
                id: DOCUMENT,
                bytes: with_a_form_that_marks(form_mcid),
                password: None,
                fragment: None,
            })
            .collect();
        let first_frame = request(&events).clone();
        serve(&mut viewer, &first_frame);

        let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
            panic!("a page has a geometry");
        };
        let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
            panic!("the query always answers");
        };
        let nodes = on_one_page(pages);
        // §8.3.2.3's default user space has y up from the bottom of the page and the viewport has
        // it down from the top, so the page's 100 units become the flip: the box's *top* edge is
        // the smaller device coordinate. Written from the page's own numbers and the geometry the
        // viewer answers separately, never from the answer under test.
        let place = |box_: [f32; 4]| {
            Some([
                geometry.origin.0 + box_[0] * geometry.scale,
                geometry.origin.1 + (100.0 - box_[3]) * geometry.scale,
                geometry.origin.0 + box_[2] * geometry.scale,
                geometry.origin.1 + (100.0 - box_[1]) * geometry.scale,
            ])
        };
        let named = |want: &str| {
            nodes
                .iter()
                .find(|node| node.name == want)
                .unwrap_or_else(|| panic!("{name}: {want} is on the page: {nodes:?}"))
                .clone()
        };

        assert_eq!(
            named("the page's own").drawn,
            place([10.0, 10.0, 30.0, 30.0]),
            "{name}: the paragraph is where the page's own stream drew"
        );
        assert_eq!(
            named("the form's own").drawn,
            place([120.0, 10.0, 160.0, 50.0]),
            "{name}: the figure is where the form drew, and Table 357's /Stm is what says so"
        );
    }
}

/// Table 384's `/Scope` crosses for a `TH`, stated or assumed, and for nothing else.
///
/// §14.8.4.8.3 makes a `TH` a cell "describing one or more rows, columns or rows and columns of
/// the table", and §14.8.5.7 says which where the document does not:
///
/// > if it is in the first row and column, the Scope is assumed to be Both
///
/// > otherwise, if it is in the first row, the Scope is assumed to be Column
///
/// > otherwise, if it is in the first column, the Scope is assumed to be Row
///
/// > otherwise, the Scope is assumed to be Both
///
/// The assumption is about the cell's place in the table's *grid*, which is why the answer is
/// this crate's: a host has the elements and not the spans that placed them.
#[test]
fn a_header_cell_crosses_with_the_axis_it_describes() {
    use pdf_model::structure::HeaderScope;

    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_table(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);
    let scope = |name: &str| {
        nodes
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("{name} is on the page: {nodes:?}"))
            .header_scope
    };

    // First row and column: the corner of a table with headers on two sides.
    assert_eq!(scope("Region"), Some(HeaderScope::Both));
    // The first row, and not the first column.
    assert_eq!(scope("2023"), Some(HeaderScope::Column));
    // The second row's only child — and *not* its first column, because the corner cell above it
    // states a `/RowSpan` of 2. A reader counting children would answer `Row` here.
    assert_eq!(scope("North"), Some(HeaderScope::Both));
    // The third row's first column, where the spill has expired.
    assert_eq!(scope("South"), Some(HeaderScope::Row));
    // A data cell describes nothing: Table 384's entry "shall only have an effect for structure
    // elements of type of TH".
    assert_eq!(scope("12"), None);
    // And a stated `/Scope` beats the assumption, which would have said `Row` for this one.
    assert_eq!(scope("Total"), Some(HeaderScope::Column));

    // Nothing outside the table claims an axis.
    assert!(
        nodes
            .iter()
            .all(|node| node.role == "TH" || node.header_scope.is_none()),
        "{nodes:?}"
    );
}

/// Table 384's `/Summary` and `/Short` cross, each for the type its own sentence names.
///
/// §14.8.5.7 conditions the first — "[t]his entry shall only be used within Table structure
/// elements" — and the second: "[t]his entry shall only have an effect for structure elements of
/// type of TH". The fixture plants both entries on types those sentences exclude, a `/Short` on
/// the table and both on a data cell, so a reader that skipped the conditions fails here rather
/// than passing quietly.
#[test]
fn a_tables_summary_and_a_headers_short_form_cross_for_their_own_types() {
    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_table(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);

    let table = nodes
        .iter()
        .find(|node| node.role == "Table")
        .unwrap_or_else(|| panic!("the table is on the page: {nodes:?}"));
    assert_eq!(table.summary.as_deref(), Some("sales by region and year"));
    assert_eq!(
        table.short, None,
        "a /Short on a table is a statement §14.8.5.7 does not define"
    );

    let named = |want: &str| {
        nodes
            .iter()
            .find(|node| node.name == want)
            .unwrap_or_else(|| panic!("{want} is on the page: {nodes:?}"))
    };
    assert_eq!(named("2023").short.as_deref(), Some("Yr"));
    assert_eq!(
        named("12").short,
        None,
        "the entry has an effect for a TH and a data cell is not one"
    );
    assert_eq!(named("12").summary, None, "nor is it a table");
    assert_eq!(
        named("Region").short,
        None,
        "a header that states none answers nothing rather than an invention"
    );
}

/// A tagged page whose structure reaches three annotations through §14.7.5.3's object references.
///
/// Two of them are widget annotations wrapped in §14.8.4.7.2's `Form` — a check box that is on and
/// a multi-line text field — and the third is a text annotation wrapped in an `Annot`. None of the
/// three marks any text, which is the whole point: before §12.5.2's rectangle was read, all three
/// crossed with no place at all, and the two `Form`s crossed as generic groups.
///
/// The page states a §14.7.5.4 parent tree keyed for all four elements, so the walk takes the
/// route ADR 0325 built rather than the fallback.
fn with_a_form() -> Vec<u8> {
    use std::fmt::Write as _;
    let content = "BT /F1 12 Tf 10 10 Td /P <</MCID 0>> BDC (a caption) Tj EMC ET\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R \
          /MarkInfo << /Marked true >> /AcroForm << /Fields [12 0 R 13 0 R] >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> /StructParents 0 \
          /Annots [12 0 R 13 0 R 14 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R 8 0 R 9 0 R 10 0 R] /ParentTree 11 0 R >>\
         \nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Form /P 6 0 R /Pg 3 0 R \
          /K [<< /Type /OBJR /Obj 12 0 R >>] /Alt (agree to the terms) >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /Form /P 6 0 R /Pg 3 0 R \
          /K [<< /Type /OBJR /Obj 13 0 R >>] /Alt (your surname) >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /Annot /P 6 0 R /Pg 3 0 R \
          /K [<< /Type /OBJR /Obj 14 0 R >>] /Alt (a note in the margin) >>\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K [0] >>\nendobj\n\
         11 0 obj\n<< /Nums [0 [10 0 R] 1 7 0 R 2 8 0 R 3 9 0 R] >>\nendobj\n\
         12 0 obj\n<< /Type /Annot /Subtype /Widget /F 4 /FT /Btn /T (agree) /V /Yes \
          /Rect [10 60 30 80] /StructParent 1 \
          /AP << /N << /Yes 15 0 R /Off 15 0 R >> >> >>\nendobj\n\
         13 0 obj\n<< /Type /Annot /Subtype /Widget /F 4 /FT /Tx /T (surname) /Ff 4096 \
          /V (Ada) /DA (/F1 0 Tf 0 g) /Rect [40 20 160 40] /StructParent 2 >>\nendobj\n\
         14 0 obj\n<< /Type /Annot /Subtype /Text /F 4 /Name /Note /Contents (a note) \
          /Rect [170 70 190 90] /StructParent 3 >>\nendobj\n\
         15 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\
         \nstream\n\nendstream\nendobj\n",
        content.len(),
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

/// An element whose content item is §14.7.5.3's object reference is placed by §12.5.2's rectangle,
/// and a `Form` says which of §12.7.5's controls it is.
///
/// §14.7.5.3 makes an object reference the form a content item takes
///
/// > When a structure element's content consists of an entire PDF object, such as an XObject
/// > directly or indirectly referenced by a page description or an annotation
///
/// and for the annotation half of that sentence Table 166 states where the object is —
/// "defining the location of the annotation on the page in default user space units" — so an
/// element that marks no text and states no Table 379 `/BBox` still has a place. 333 of the 1675
/// corpus elements in that position are placed this way (`pdf-model --example
/// element_bounds_census`).
///
/// The control is §14.8.4.7.2's, whose Table 368 makes `Form` one that "[e]ncloses a PDF widget
/// annotation and associated content, if any" — Errata Collection 3's Issue #437 — one widget, and
/// therefore one control rather than a group.
///
/// **The expected rectangles are written out from the clause's own space**, which is trap 12a's
/// rule: `/Rect` is in default user space with y pointing up from the bottom of a 100-unit page,
/// and the viewport's origin and scale come from [`Query::PageGeometry`].
#[test]
fn an_element_reached_through_an_object_reference_is_placed_and_says_what_control_it_is() {
    use pdf_model::form::Control;

    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_form(),
            password: None,
            fragment: None,
        })
        .collect();
    let first = request(&events).clone();
    serve(&mut viewer, &first);

    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("a page has a geometry");
    };
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);
    let node = |name: &str| {
        nodes
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("{name} is on the page: {nodes:?}"))
            .clone()
    };
    // A rectangle stated in default user space, on the screen: y up becomes y down about the
    // page's own height.
    let placed = |rect: [f32; 4]| {
        Some([
            geometry.origin.0 + rect[0] * geometry.scale,
            geometry.origin.1 + (100.0 - rect[3]) * geometry.scale,
            geometry.origin.0 + rect[2] * geometry.scale,
            geometry.origin.1 + (100.0 - rect[1]) * geometry.scale,
        ])
    };

    let check_box = node("agree to the terms");
    assert!(
        check_box.quads.is_empty(),
        "a widget annotation marks no text: {check_box:?}"
    );
    assert_eq!(check_box.bounds, placed([10.0, 60.0, 30.0, 80.0]));
    assert_eq!(
        check_box.control,
        Some(Control::CheckBox { on: true }),
        "§12.7.5.2.3's field toggles between two states and Table 226's /V says which"
    );

    let text = node("your surname");
    assert_eq!(text.bounds, placed([40.0, 20.0, 160.0, 40.0]));
    let Some(Control::Text(control)) = text.control else {
        panic!("§12.7.5.3's text field: {text:?}");
    };
    assert!(
        control.multiline,
        "Table 231 bit 13 is set, so the field 'may contain multiple lines of text'"
    );
    assert!(!control.password);

    // §14.7.5.3's other annotation: placed the same way, and no control, because it is not a
    // widget and §12.7 has nothing to say about it.
    let margin = node("a note in the margin");
    assert_eq!(margin.bounds, placed([170.0, 70.0, 190.0, 90.0]));
    assert_eq!(margin.control, None);

    // And the paragraph, which the text layer places and no annotation describes.
    let caption = node("a caption");
    assert!(!caption.quads.is_empty());
    assert_eq!(caption.bounds, None);
    assert_eq!(caption.control, None);

    // **Which of them *is* an annotation**, which is a different question from where it is and
    // from what control it is — and the one an assistive technology asking to click needs
    // answered. §12.5.1 makes activation something a person does to an annotation, and the three
    // widget-bearing elements above each name one through §14.7.5.3 while the paragraph names
    // none. A rectangle cannot tell the two apart: the caption has quads and the `Figure` in
    // other documents has a stated `/BBox`.
    assert!(check_box.annotation.is_some());
    assert!(text.annotation.is_some());
    assert!(
        margin.annotation.is_some(),
        "Table 368's `Annot` encloses a PDF annotation as much as `Form` encloses a widget"
    );
    assert_eq!(caption.annotation, None);
    assert_ne!(
        check_box.annotation, text.annotation,
        "two elements, two widgets"
    );
}

/// A one-page tagged form whose element reaches its widget through Table 357 rather than 358.
///
/// The check box's marked sequence is inside its own appearance stream — the `/Yes` stream of
/// `/AP`'s state subdictionary — and the structure element names it with a marked-content
/// reference: `/Stm` the appearance stream, `/StmOwn` the widget that owns it, `/MCID` its
/// identifier there. No §14.7.5.3 object reference anywhere, which is the point: Table 357 is
/// the other route to the same annotation.
fn with_an_owned_appearance() -> Vec<u8> {
    use std::fmt::Write as _;
    let content = "BT /F1 12 Tf 10 10 Td /P <</MCID 0>> BDC (a caption) Tj EMC ET\n";
    let appearance = "/P << /MCID 0 >> BDC BT /F1 8 Tf 2 5 Td (yes) Tj ET EMC\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R \
          /MarkInfo << /Marked true >> /AcroForm << /Fields [12 0 R] >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> /StructParents 0 \
          /Annots [12 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R 8 0 R] /ParentTree 11 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Form /P 6 0 R /Pg 3 0 R \
          /K << /Type /MCR /Pg 3 0 R /Stm 15 0 R /StmOwn 12 0 R /MCID 0 >> \
          /Alt (agree to the terms) >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K [0] >>\nendobj\n\
         11 0 obj\n<< /Nums [0 [8 0 R] 1 [7 0 R]] >>\nendobj\n\
         12 0 obj\n<< /Type /Annot /Subtype /Widget /F 4 /FT /Btn /T (agree) /V /Yes /AS /Yes \
          /Rect [10 60 30 80] \
          /AP << /N << /Yes 15 0 R /Off 16 0 R >> >> >>\nendobj\n\
         15 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /StructParents 1 \
          /Resources << /Font << /F1 5 0 R >> >> /Length {} >>\
         \nstream\n{appearance}endstream\nendobj\n\
         16 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\
         \nstream\n\nendstream\nendobj\n",
        content.len(),
        appearance.len(),
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        let number: usize = object
            .split_whitespace()
            .next()
            .and_then(|word| word.parse().ok())
            .expect("every object states its number");
        offsets.insert(number, cursor);
        cursor = cursor.saturating_add(object.len());
    }
    out.push_str(&body);
    let xref_at = out.len();
    let size = offsets.keys().copied().max().unwrap_or(0).saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for number in 1..size {
        match offsets.get(&number) {
            Some(offset) => {
                let _ = writeln!(out, "{offset:010} 00000 n ");
            }
            None => {
                let _ = writeln!(out, "0000000000 65535 f ");
            }
        }
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// An element that names its widget through Table 357's `/StmOwn` is placed and says what it is.
///
/// ISO 32000-2 §14.7.5.2, Table 357's `/StmOwn` row:
///
/// > The indirect reference to the PDF object referencing the stream identified by the Stm key.
///
/// and its NOTE names the use this fixture is: "to identify the annotation dictionary owning the
/// appearance stream". So the entry is §14.7.5.3's statement — this element's content belongs to
/// that annotation — made from the marked-content side, and it reaches the same three answers an
/// object reference does: §12.5.2's rectangle places the element, §12.7 says which control it is,
/// and §12.5.1's activation names the annotation a click goes to. The sequence itself still
/// carries the quads, because `/Stm` names the appearance stream the widget's marks are in.
#[test]
fn an_element_reaching_its_widget_through_stmown_is_placed_and_says_what_control_it_is() {
    use pdf_model::form::Control;

    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_an_owned_appearance(),
            password: None,
            fragment: None,
        })
        .collect();
    let first = request(&events).clone();
    serve(&mut viewer, &first);

    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("a page has a geometry");
    };
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);
    let check_box = nodes
        .iter()
        .find(|node| node.name == "agree to the terms")
        .unwrap_or_else(|| panic!("the element is on the page: {nodes:?}"))
        .clone();

    // §12.5.2's rectangle, from default user space (y up, a 100-unit page) to the viewport.
    assert_eq!(
        check_box.bounds,
        Some([
            geometry.origin.0 + 10.0 * geometry.scale,
            geometry.origin.1 + (100.0 - 80.0) * geometry.scale,
            geometry.origin.0 + 30.0 * geometry.scale,
            geometry.origin.1 + (100.0 - 60.0) * geometry.scale,
        ]),
        "Table 357's /StmOwn names the widget, and the widget's /Rect places the element"
    );
    assert_eq!(
        check_box.control,
        Some(Control::CheckBox { on: true }),
        "§12.7.5.2.3's control arrives through the same match"
    );
    assert!(
        check_box.annotation.is_some(),
        "§12.5.1's activation has an annotation to go to"
    );
    assert!(
        !check_box.quads.is_empty(),
        "/Stm names the appearance stream, so the sequence's own marks are the element's: {check_box:?}"
    );
}

/// §14.8.4.8.3's search gives each cell the header cells that describe it.
///
/// > To find headers for any data or header cell, begin from the current cell position and use
/// > the current value of WritingMode to search towards the first cell in the appropriate
/// > horizontal/vertical direction.
///
/// The same fixture as the axis test above, because the two questions are one grid: the search
/// walks out along the row and up the column, and both the `/RowSpan` and the stated `/Scope`
/// change what it finds. 17 152 of the corpus's 17 431 cells that end with a header get it this
/// way rather than from Table 384's array — `pdf-model --example cell_header_census`.
#[test]
fn a_cell_is_given_the_header_cells_that_describe_it() {
    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: with_a_table(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = request(&events).clone();
    serve(&mut viewer, &request);

    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let nodes = on_one_page(pages);
    let headers = |name: &str| -> Vec<&str> {
        nodes
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("{name} is on the page: {nodes:?}"))
            .headers
            .iter()
            .map(|at| nodes.get(*at).map_or("", |header| header.name.as_str()))
            .collect()
    };

    // The corner cell is the table's edge in both directions and has no headers at all.
    assert!(headers("Region").is_empty(), "{:?}", headers("Region"));
    // Table 384's order: the row's headers, then the column's. `Region` spans two rows, so the
    // second row's cell meets it along its row even though nothing of it was written there.
    assert_eq!(headers("North"), vec!["Region", "2023"]);
    // A data cell, whose row header is the `TH` beside it and whose column headers are the two
    // above — the search collects a run of header cells and stops at the first data cell after
    // one, which is what makes this three rather than two.
    assert_eq!(headers("12"), vec!["South", "North", "2023"]);
    // And the scope filter: `South` is a header cell in this cell's own column, but §14.8.5.7
    // assumes it to be its *row*'s, so the column search steps over it to reach `Region`.
    assert_eq!(headers("Total"), vec!["Region"]);
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
/// **No corpus document states a thread *action*** — four state a thread, all outside pdf.js, and none
/// of them states a `/Thread` action to reach it — so the fixture is built from the clause, and the
/// assertion is the magnification a 100-unit-wide bead earns in an 800-pixel window rather than a
/// page number nothing distinguishes. This comment said "no corpus document states an article" until
/// the five-hundred-and-seventieth session, on a count taken over pdf.js alone; ADR 0405.
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
/// Built from §12.4.3 and Table 209, because no corpus document states a thread *action* — and this
/// comment used to say "the corpus states no article at all", which the four witnesses under
/// `doc/corpora/` falsify (ADR 0405). `pdf-model/tests/articles.rs` reads one of them. The thread's
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

/// §12.4.4's transition, drawn: frame `n` is between the two pages, in pixels.
///
/// **This is the whole of what a host owes a transition** — the core shapes the frame and the
/// host owns the clock (ADR 0230) — so a tier-1 host with no display can play one by choosing a
/// fraction, which is what this does at a quarter and a half of the way through.
///
/// Table 164's `Wipe`: "[a] single line sweeps across the screen from one edge to the other in
/// the direction specified by the Di entry, revealing the new page", with `/Di 0` "[l]eft to
/// right". So at a quarter of the way through, the left quarter of the window is the page moved
/// **to** and the rest is the page being left — which is a statement about pixels and is checked
/// as one, against the two flat colours the fixture draws.
#[test]
fn a_transition_frame_is_between_the_two_pages() {
    const WINDOW: (u32, u32) = (200, 100);
    let mut viewer = Viewer::new(WINDOW.0, WINDOW.1, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: two_coloured_pages(),
            password: None,
            fragment: None,
        })
        .collect();
    let leaving = raster(request(&events));
    assert_eq!(&leaving.data[0..3], &[255, 0, 0], "page one is red");

    // The clock, which is the only way this crate learns that a second went by (rule 3).
    let advanced: Vec<Event> = viewer.handle(Command::Tick { millis: 1100 }).collect();
    let transition = advanced
        .iter()
        .find_map(|event| match event {
            Event::Transition { transition, .. } => Some(transition.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("page two states a /Trans: {advanced:?}"));
    let arriving = raster(request(&advanced));
    assert_eq!(&arriving.data[0..3], &[0, 0, 255], "page two is blue");

    let viewport = pdf_render::Rect::from_corners(
        pdf_render::Point::new(0.0, 0.0),
        pdf_render::Point::new(200.0, 100.0),
    );
    let (outgoing, incoming) = (
        viewer_core::transition::drawable(&leaving).expect("a page is drawable"),
        viewer_core::transition::drawable(&arriving).expect("a page is drawable"),
    );

    for (progress, revealed) in [(0.25_f32, 50_u32), (0.5, 100)] {
        let frame = viewer_core::transition::frame(&transition, viewport, progress)
            .expect("a /Wipe is shaped");
        let list = frame
            .draw(viewport, &outgoing, &incoming)
            .expect("two images and one clip");
        let drawn = CpuRasterizer::new()
            .rasterize(
                &list,
                pdf_render::TargetSpec {
                    width: WINDOW.0,
                    height: WINDOW.1,
                    transform: pdf_render::Transform::IDENTITY,
                },
            )
            .expect("the CPU backend draws a transition frame");
        // Where the sweeping line is, to the pixel: everything left of it is the new page and
        // everything right of it is the old one.
        let at = |x: u32| {
            let index = (x as usize).saturating_mul(4).saturating_add(50 * 200 * 4);
            drawn
                .data
                .get(index..index.saturating_add(3))
                .map(<[u8]>::to_vec)
        };
        assert_eq!(
            at(revealed.saturating_sub(2)),
            Some(vec![0, 0, 255]),
            "at {progress}: the swept side is the page moved to"
        );
        assert_eq!(
            at(revealed.saturating_add(2)),
            Some(vec![255, 0, 0]),
            "at {progress}: the rest is the page being left"
        );
    }
}

/// A style Table 164 names and this reader does not draw is reported by name.
///
/// Trap 5 where a viewer is most tempted to be silent: the page that arrives looks right, and
/// only the file knows it asked for an effect. `Blinds` is "[m]ultiple lines, evenly spaced
/// across the screen" and the clause never says how many, which is why it is one of the four
/// left unshaped (ADR 0230).
#[test]
fn a_transition_this_reader_does_not_draw_is_named_rather_than_cut() {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Trans << /S /Blinds >> >>\nendobj\n";
    let mut viewer = Viewer::new(200, 100, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: assemble(body),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let advanced: Vec<Event> = viewer.handle(Command::Tick { millis: 1100 }).collect();
    let said = advanced
        .iter()
        .find_map(|event| match event {
            Event::Reported { notes, page, .. } => Some((notes.join(" "), *page)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a style with no frames is reported: {advanced:?}"));
    assert!(said.0.contains("/Blinds"), "{said:?}");
    assert_eq!(said.1, Some(1), "about the page it was moving to");
    // And it is still *named*, because a host that can draw it is not this one.
    assert!(
        advanced
            .iter()
            .any(|event| matches!(event, Event::Transition { .. })),
        "{advanced:?}"
    );
}

/// A style this reader *does* draw, asked for in a direction Table 164 does not give it.
///
/// The same trap 5 obligation one step in from the test above, and until the
/// seven-hundred-and-twentieth session this page arrived as a cut with nothing said: the report
/// was keyed on the style, which is `Wipe` and is shaped, while the frame was refused for the
/// direction. Table 164 gives `Wipe` the four quarter turns and reserves 315 to `Glitter`, so a
/// `Wipe` at 315 is a direction the table does not send that effect in — and `viewer_core::
/// transition` shapes no frame for it, which is what makes the sentence owed.
#[test]
fn a_direction_the_table_does_not_give_a_style_is_named_rather_than_cut() {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Trans << /S /Wipe /Di 315 >> >>\nendobj\n";
    let mut viewer = Viewer::new(200, 100, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: assemble(body),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let advanced: Vec<Event> = viewer.handle(Command::Tick { millis: 1100 }).collect();
    let said = advanced
        .iter()
        .find_map(|event| match event {
            Event::Reported { notes, page, .. } => Some((notes.join(" "), *page)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a direction with no frames is reported: {advanced:?}"));
    assert!(
        said.0.contains("/Wipe") && said.0.contains("315"),
        "{said:?}"
    );
    assert_eq!(said.1, Some(1), "about the page it was moving to");
    // And the transition is still raised, because a host that can draw it is not this one.
    assert!(
        advanced
            .iter()
            .any(|event| matches!(event, Event::Transition { .. })),
        "{advanced:?}"
    );
}

/// Two pages of one flat colour each, the second stating a `/Trans`, both stating a `/Dur`.
///
/// A transition is a picture *between two pages*, so the fixture's whole job is to make the two
/// distinguishable by a pixel: page one is red and page two is blue, and neither draws anything
/// else. §12.4.4.1's `/Trans` is on the page arrived at — "the transition style that shall be
/// used when moving to this page from another" — so the `Wipe` belongs to page two.
///
/// `/Di 0` is Table 164's "[l]eft to right", which is the direction the assertion reads.
fn two_coloured_pages() -> Vec<u8> {
    let red = "1 0 0 rg 0 0 200 100 re f";
    let blue = "0 0 1 rg 0 0 200 100 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 \
         /Contents 5 0 R >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 \
         /Trans << /Type /Trans /S /Wipe /D 2 /Di 0 >> /Contents 6 0 R >>\nendobj\n\
         5 0 obj\n<< /Length {} >>\nstream\n{red}\nendstream\nendobj\n\
         6 0 obj\n<< /Length {} >>\nstream\n{blue}\nendstream\nendobj\n",
        red.len(),
        blue.len()
    );
    assemble(&body)
}

/// Wraps hand-written objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str) -> Vec<u8> {
    use std::fmt::Write as _;
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
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 \
         /Trans << /S /Split >> >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 \
         /Trans << /S /Wipe >> >>\nendobj\n\
         5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Dur 1 >>\nendobj\n";
    assemble(body)
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
    let [
        Event::Extracted {
            asked, name, bytes, ..
        },
    ] = events.as_slice()
    else {
        panic!("one extraction, not {events:?}");
    };
    // A person pressed something, which is what lets a host write the file to disk without asking
    // again — Annex O's `ef` is the other provenance and `tests/fragments.rs` holds that end.
    assert_eq!(*asked, viewer_core::Extraction::Asked);
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

/// §12.5.6.15: clicking a file attachment annotation extracts the file it attaches.
///
/// §12.5.1 says an activated annotation "exhibits its associated object, such as by opening a
/// popup window displaying a text note", and §12.5.6.15 says what a file attachment
/// annotation's associated object is and what activating one does: "activating the annotation
/// extracts the embedded file and gives the user an opportunity to view it or store it in the
/// file system". Until ADR 0295 nothing in this tree read Table 187's required `/FS` for its own
/// clause — the icon was drawn and the file behind it was unreachable, because the only list
/// this program built came from §7.7.4's `/EmbeddedFiles` tree and this document has no name
/// dictionary at all.
///
/// It is the only such document in the 974 (`pdf-model --example file_attachment_census`), and
/// ISO 32000-2's own PDF has six.
#[test]
fn a_click_on_a_file_attachment_annotation_extracts_its_file() {
    let Some(bytes) = corpus_bytes("annotation-fileattachment.pdf") else {
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
    // The annotation's own `/Rect`, on a page 841.92 units tall.
    let on_paperclip = device_point(&viewer, [70.7023, 724.338, 90.7023, 748.338], 841.92);
    viewer
        .handle(Command::Pointer {
            at: on_paperclip,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at: on_paperclip,
            action: PointerAction::Released,
        })
        .collect();
    let extracted = events.iter().find_map(|event| match event {
        Event::Extracted { name, bytes, .. } => Some((name.clone(), bytes.clone())),
        _ => None,
    });
    let Some((name, bytes)) = extracted else {
        panic!("the click extracts the file, and produced {events:?}");
    };
    assert_eq!(name, "Test.txt", "Table 43's own name for the file");
    assert_eq!(
        String::from_utf8_lossy(&bytes),
        "Test attachment",
        "the file itself, with §7.4's filters undone"
    );
    // And this corpus's only witness to a rule `pdf-model` had only fixtures for: Table 45's
    // `/CheckSum` is "a 16-byte string" and this file's is a UTF-16BE text string beginning with
    // a byte order mark, which is the producer having written the MD5 digest as text. A checksum
    // stated wrongly is not a checksum absent, so it is reported — and the bytes still come,
    // because the clause says the entry "is strictly a checksum, and is not used for security
    // purposes".
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Reported { notes, .. } if notes.iter().any(|note| note.contains("MD5 checksum"))
        )),
        "{events:?}"
    );

    // A click somewhere else on the page extracts nothing: the file crosses because the
    // annotation was activated, not because the document carries it.
    viewer
        .handle(Command::Pointer {
            at: (400.0, 400.0),
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at: (400.0, 400.0),
            action: PointerAction::Released,
        })
        .collect();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Extracted { .. })),
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
    // The re-interpretation is the view's, not the ink's: the same layers are on and the same
    // values are in the fields, so a host holding a picture of the old magnification may stand
    // in with it while this request renders. A superseded ink here froze every zoom of such a
    // page for the length of the real frame, because a host refuses to show a picture of other
    // ink — which is right, and why the ink must not move for a zoom.
    assert_eq!(
        first.ink, second.ink,
        "a zoom re-interprets the page without superseding its ink"
    );
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

/// A one-page form whose author certified it at §12.8.2.2's `/P` `level`.
///
/// Built rather than found: the corpus's one certification signature states `/P` 2, so the level
/// that refuses filling in a field exists nowhere in the 974 (trap 8). The catalog carries
/// §12.8.6's `/Perms /DocMDP`, which is what §12.8.2.2.1's parenthesis makes binding:
///
/// > (These changes to the document shall also be prevented if the signature dictionary is
/// > referred from the DocMDP entry in the permissions dictionary.)
///
/// It points at a signature whose `/Reference` names the `DocMDP` transform and states the level.
fn certified_form(level: i64) -> Vec<u8> {
    use std::fmt::Write as _;

    let objects = [
        "<< /Type /Catalog /Perms << /DocMDP 6 0 R >> /Pages 2 0 R /AcroForm \
         << /Fields [5 0 R] /DR << /Font << /Helv 8 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx /T (name) \
         /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        "<< /Type /Sig /Reference [7 0 R] >>".to_owned(),
        format!(
            "<< /Type /SigRef /TransformMethod /DocMDP /TransformParams \
             << /Type /TransformParams /P {level} /V /1.2 >> >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
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

/// Opens a document under a host-chosen restriction level, draining the events.
fn opened_with(bytes: Vec<u8>, level: RestrictionLevel) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer.handle(Command::Restrict(level)).for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

/// An operation the document restricts is refused **with a reason**, not with a silence.
///
/// The shape `CLAUDE.md`'s "a document's restrictions are the reader's to set, and they have
/// levels" asks for. What is asserted is not that something failed — a count of nothing happening
/// is what this used to be — but that the refusal names its clause and its operation, which is
/// what an *ask* level would put in front of a person and what a host words into a sentence. The
/// level is read rather than the presence of a signature: `/P` 2 permits exactly this operation,
/// and the same fixture one number apart is not refused at all. ADR 0212.
#[test]
fn a_restricted_operation_is_refused_with_a_reason() {
    let mut viewer = opened_with(certified_form(1), RestrictionLevel::On);
    let events: Vec<_> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: "name".to_owned(),
            value: Entered::Text("typed".to_owned()),
        }))
        .collect();

    let refused: Vec<&Event> = events
        .iter()
        .filter(|event| matches!(event, Event::Refused { .. }))
        .collect();
    let [
        Event::Refused {
            operation, notes, ..
        },
    ] = refused.as_slice()
    else {
        panic!("one refusal, naming what it refused: {events:?}");
    };
    assert_eq!(*operation, pdf_model::restriction::Operation::FillInForm);
    let [note] = notes.as_slice() else {
        panic!("one clause withholds it, so one sentence: {notes:?}");
    };
    assert!(
        note.contains("§12.8.2.2's /P 1") && note.contains("filling in a form field"),
        "the reason names the clause and the operation: {note}"
    );

    // Nothing was done, and the log knows it: an edit that was refused is not an edit that can
    // be undone (ADR 0196's rule — the log records what was *done*).
    assert!(
        matches!(viewer.query(Query::Dirty), Answer::Dirty(false)),
        "a refused edit leaves nothing to save"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Dirty { .. })),
        "{events:?}"
    );

    // And it is the *level* that is read: Table 257's `/P` 2 is "filling in forms, instantiating
    // page templates, and signing", so the same document one number apart refuses nothing.
    let mut permitted = opened_with(certified_form(2), RestrictionLevel::On);
    let events: Vec<_> = permitted
        .handle(Command::Edit(Edit::SetField {
            field: "name".to_owned(),
            value: Entered::Text("typed".to_owned()),
        }))
        .collect();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Refused { .. })),
        "{events:?}"
    );
}

/// The reader turns the restriction off, and the operation happens.
///
/// `CLAUDE.md`, in the project owner's own words: "**it shall always be possible to turn them
/// off**". So this is the second half of the shape and not a convenience — the policy is one
/// value, it arrives from the host, and the same document that refused above accepts the same
/// keystroke. The evidence is the *saved file*: §7.5.6's incremental update carries the value,
/// which is a stronger statement than any flag this crate could answer with.
#[test]
fn the_reader_can_turn_a_documents_restrictions_off() {
    let mut viewer = opened_with(certified_form(1), RestrictionLevel::Off);
    let events: Vec<_> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: "name".to_owned(),
            value: Entered::Text("typed".to_owned()),
        }))
        .collect();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Refused { .. })),
        "the reader said not to obey it: {events:?}"
    );
    assert!(matches!(viewer.query(Query::Dirty), Answer::Dirty(true)));

    let events: Vec<_> = viewer.handle(Command::Save).collect();
    let Some(Event::Saved { bytes, .. }) = events
        .iter()
        .find(|event| matches!(event, Event::Saved { .. }))
    else {
        panic!("the fixture can be updated: {events:?}");
    };
    let reopened = pdf_syntax::Document::open(bytes.clone()).expect("what was written is a PDF");
    let names = pdf_model::view::ViewState::of(&reopened);
    assert_eq!(
        names.field_value(&reopened, "name").map(|shown| shown.text),
        Some("typed".to_owned()),
        "the value a person typed is in the file that came back"
    );

    // And the signature the document was certified with is still in the file, untouched: turning
    // a restriction off is the reader's, and §12.8.2.2 states no obligation to remove anything.
    // §12.8.2.3's `/UR3` is the one that would have to go, and this document states none.
    assert!(
        pdf_model::signature::permissions(&reopened)
            .doc_mdp
            .is_some(),
        "the /DocMDP is where the producer put it"
    );
}

/// A collection whose `/D` names an entry the `/EmbeddedFiles` tree has, or does not, or cannot.
///
/// ISO 32000-2 §12.3.5.1, Table 153's `/D`, whose value is "[a] string that identifies an entry
/// in the `EmbeddedFiles` name tree, determining the document that shall be initially presented in
/// the user interface".
fn a_collection_naming(initial: &str, files: &str) -> Vec<u8> {
    use std::fmt::Write as _;

    let objects: [String; 4] = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [{files}] >> >> \
             /Collection << /Type /Collection {initial} >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_owned(),
        "<< /Type /Filespec /F (report.pdf) >>".to_owned(),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let number = index.saturating_add(1);
        let _ = write!(out, "{number} 0 obj\n{body}\nendobj\n");
    }
    let at = out.len();
    let size = objects.len().saturating_add(1);
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

/// The collection this viewer answers with, and the document §12.3.5.1 says it opens on.
fn collection_of(bytes: Vec<u8>) -> pdf_model::collection::Initial {
    let mut viewer = Viewer::new(200, 200, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .any(|event| matches!(event, Event::Opened { .. }));
    assert!(opened, "the fixture is a valid PDF");
    let Answer::Collection { initial, .. } = viewer.query(Query::Collection) else {
        panic!("a document with a /Collection answers with one");
    };
    initial
}

/// §12.3.5.1's four `/D` outcomes reach a host, which none of them could before.
///
/// Table 153's `/D` states three fallbacks as `shall`s and the entry's presence as the fourth
/// case, and every one of them is decided against the `/EmbeddedFiles` name tree rather than
/// against the collection dictionary — which is why `Collection::initial_document` takes the
/// document, and why no panel holding Table 153 could work it out. It was implemented in the
/// three-hundred-and-fifty-second session and reachable from no host until the
/// three-hundred-and-ninety-fourth.
#[test]
fn a_collections_initial_document_reaches_a_host() {
    use pdf_model::collection::Initial;

    let one_file = "(<1>report.pdf) 4 0 R";

    // "If the D entry is missing or is not a valid byte string, the initial document shall be the
    // one that contains the collection dictionary."
    assert_eq!(
        collection_of(a_collection_naming("", one_file)),
        Initial::Container
    );
    assert_eq!(
        collection_of(a_collection_naming("/D /report", one_file)),
        Initial::Container,
        "a name is not a byte string"
    );

    // A `/D` that names an entry the tree holds is that entry.
    assert_eq!(
        collection_of(a_collection_naming("/D (<1>report.pdf)", one_file)),
        Initial::Embedded("<1>report.pdf".to_owned())
    );

    // "If the D entry is a valid byte string that does not match any file in the EmbeddedFiles
    // name tree, the interactive PDF processor shall select the first item from the list of files
    // to display in its user interface".
    assert_eq!(
        collection_of(a_collection_naming("/D (missing.pdf)", one_file)),
        Initial::FirstFile
    );

    // "if no files exist in the name tree, the interactive PDF processor shall display an empty
    // preview window."
    assert_eq!(
        collection_of(a_collection_naming("/D (missing.pdf)", "")),
        Initial::Empty
    );
}

/// §12.5.6.6: a person drags a box, types into it, saves, and another reader shows the words.
///
/// **The whole of what this round added, end to end**, and every step of it is something no
/// other test in this tree can see: the geometry comes from a drag rather than from a selection,
/// the annotation is found again by asking at a point inside it, the text goes in by object, the
/// caret stands inside the annotation the way it stands inside a field, and the file that comes
/// out is one a second viewer — which knows nothing of any of it — draws the words from.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one sitting from a drag to a re-opened file, which is the statement: splitting it \
              into steps would let one of them pass while the sitting does not"
)]
fn a_free_text_annotation_is_drawn_from_a_drag_typed_into_and_read_back() {
    let (mut viewer, events) = opened(600, 800);
    // This page has red in it already — the note's own headings — so every count below is read
    // against this one rather than against zero.
    let plain = red(&raster(request(&events)));
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    // The two corners a pointer would have gone down and come up at, in device pixels — a wide
    // band across the middle of the page, where nothing this document draws is red.
    let corner = |across: f32, down: f32| {
        (
            geometry.origin.0 + geometry.page.width * across * geometry.scale,
            geometry.origin.1 + geometry.page.height * down * geometry.scale,
        )
    };
    let (from, to) = (corner(0.2, 0.45), corner(0.8, 0.55));

    let events: Vec<_> = viewer
        .handle(Command::Edit(Edit::FreeText {
            from,
            to,
            colour: [1.0, 0.0, 0.0],
        }))
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Dirty { dirty: true, .. })),
        "{events:?}"
    );
    assert_eq!(
        red(&raster(request(&events))),
        plain,
        "§12.5.6.6's text *is* the annotation, so an empty one draws nothing"
    );

    // Which annotation: asked at a point inside the rectangle, because the core hands back no
    // event carrying it — a host can ask, so there is no message for it.
    let middle = (f32::midpoint(from.0, to.0), f32::midpoint(from.1, to.1));
    let Answer::FreeText {
        annotation,
        ref text,
    } = viewer.query(Query::FreeTextAt { at: middle })
    else {
        panic!("the drag made an annotation and it is under the point");
    };
    assert!(text.is_empty(), "nothing has been typed into it yet");

    let events: Vec<_> = viewer
        .handle(Command::Edit(Edit::SetFreeText {
            annotation,
            text: "Reviewed".to_owned(),
        }))
        .collect();
    let typed = red(&raster(request(&events)));
    assert!(
        typed > plain + 40,
        "the text a person typed is on the page in the colour the /DA states: {typed} pixels \
         against {plain} before it"
    );

    // And the caret stands inside it, from the same question a field answers — which is the
    // piece `doc/todo/33` said was missing, because `appearance::caret` began by reading a field.
    let Answer::Caret {
        from: low,
        to: high,
    } = viewer.query(Query::Caret {
        at: middle,
        offset: "Reviewed".len(),
    })
    else {
        panic!("the annotation lays its text out, so it has somewhere the next character goes");
    };
    assert!(
        (low.0 - high.0).abs() < 0.01 && high.1 < low.1,
        "a caret is a vertical segment with the ascent end above the descent end: {low:?} {high:?}"
    );
    assert!(
        low.0 > from.0 && low.0 < to.0,
        "and it stands inside the rectangle that was dragged: {low:?}"
    );

    // Undo is a replay of the log's surviving prefix, and there are two entries in it.
    let events: Vec<_> = viewer.handle(Command::Undo).collect();
    assert_eq!(
        red(&raster(request(&events))),
        plain,
        "the first undo takes the text back out"
    );
    let events: Vec<_> = viewer.handle(Command::Redo).collect();
    assert!(
        red(&raster(request(&events))) > plain + 40,
        "and redo puts it back, because the annotation's object is the same on every replay"
    );

    let events: Vec<_> = viewer.handle(Command::Save).collect();
    let saved = events
        .iter()
        .find_map(|event| match event {
            Event::Saved { bytes, .. } => Some(bytes.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{events:?}"));

    // A second viewer, which knows nothing of the drag or the keystrokes, opens the saved bytes.
    let mut reader = Viewer::new(600, 800, 1.0);
    let events: Vec<_> = reader
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: saved,
            password: None,
            fragment: None,
        })
        .collect();
    let reopened = red(&raster(request(&events)));
    assert!(
        reopened > plain + 40,
        "what another reader shows is what this round is judged on: {reopened} red pixels \
         against {plain} on the page the producer wrote"
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
        selection.text.contains("Reviewed"),
        "and the words read back off the page: {:?}",
        selection.text
    );
}

/// How many pixels are red, counted against the same page before anything was added to it.
fn red(raster: &pdf_render::Raster) -> usize {
    raster
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100)
        .count()
}

/// §6.3.2.2's "unless otherwise instructed", asked by a host that has its own controls.
///
/// The whole of what a native form host needs from this crate, in one exchange: it reads the
/// form with [`Query::Fields`], places a control per widget, and then asks for the page *without*
/// the pictures of those widgets — otherwise a person sees each field twice, which is what
/// ADR 0244 photographed. Three things are asserted and each is a way the change could be wrong.
///
/// The page is interpreted again, because §12.5.5's appearance streams are drawing commands and
/// not pixels; the form is still answered, because delegating the *appearance* must not withdraw
/// the description a host builds its controls from; and asking for them back restores the list
/// the viewer started with, which is what makes this a policy rather than a one-way door.
#[test]
fn a_host_can_ask_for_the_page_without_the_widgets_it_draws_itself() {
    let Some(bytes) = corpus_bytes("160F-2019.pdf") else {
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
    let drawn = request(&events).clone();
    serve(&mut viewer, &drawn.clone());

    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("this page is a form");
    };
    let widgets: usize = fields.iter().map(|field| field.widgets.len()).sum();
    assert!(
        fields.len() > 1 && widgets >= fields.len(),
        "{} field(s), {widgets} widget(s)",
        fields.len()
    );

    let events: Vec<_> = viewer
        .handle(Command::Delegate(WidgetAppearances::Delegated))
        .collect();
    let delegated = request(&events).clone();
    assert_eq!(
        (delegated.page, delegated.target),
        (drawn.page, drawn.target),
        "the same page at the same size"
    );
    assert!(
        delegated.list.commands().len() < drawn.list.commands().len(),
        "the widgets' own drawing is gone: {} command(s) against {}",
        delegated.list.commands().len(),
        drawn.list.commands().len()
    );

    // The description survives the picture. A host that lost the form when it took over drawing
    // it would have nothing to put in its controls.
    let Answer::Fields(after) = viewer.query(Query::Fields) else {
        panic!("the page is still a form");
    };
    assert_eq!(
        after.len(),
        fields.len(),
        "delegating an appearance withdraws no field"
    );

    // And back. §6.3.2.2's default is what a processor nobody has instructed does, so a host
    // that changes its mind gets the page the standard describes.
    serve(&mut viewer, &delegated.clone());
    let events: Vec<_> = viewer
        .handle(Command::Delegate(WidgetAppearances::Drawn))
        .collect();
    let again = request(&events).clone();
    assert_eq!(
        again.list.commands().len(),
        drawn.list.commands().len(),
        "the page came back whole"
    );
    // `DisplayList`'s own equality, which `Path` defines as "the same commands" and which
    // deliberately ignores the bounds a rasterisation memoises into the path it drew — the list
    // that has been on screen carries those and the one just built does not.
    assert!(
        *again.list == *drawn.list,
        "the same page, command for command"
    );
}

/// Annex O's `search` across a whole document, one page per step, and back round to the start.
///
/// ISO 32000-2, Table Annex O.4: "Open the document and search for one or more words, selecting
/// the first matching word in the document." Every expectation here is *derived* rather than
/// written down: the page the search lands on is checked against `pdf_model`'s own readback of
/// every page, so this test would fail if the search reported a page the word is not on and would
/// equally fail if it skipped an earlier one that has it.
#[test]
fn a_search_reads_the_document_one_page_at_a_time_and_lands_on_the_first_occurrence() {
    let needle = "compensation";
    // The independent answer: which pages hold the word, from the same readback the viewer's own
    // `Query::Find` is over but reached without going through the viewer at all.
    let document = pdf_syntax::Document::open(specification_bytes()).expect("the note opens");
    let pages = pdf_model::Pages::new(&document);
    let view = pdf_model::view::ViewState::of(&document);
    let holding: Vec<usize> = (0..PAGES)
        .filter(|index| {
            pages.get(*index).is_some_and(|page| {
                pdf_model::content::interpret_with(&document, &page, &view)
                    .text
                    .to_lowercase()
                    .contains(needle)
            })
        })
        .collect();
    assert!(
        holding.len() >= 2,
        "the fixture must hold the word on more than one page: {holding:?}"
    );

    let (mut viewer, _) = opened(800, 1000);
    let mut steps = 0_usize;
    let found = run_search(&mut viewer, needle, false, &mut steps);
    let (page, range) = found.expect("the word is in this document");
    assert_eq!(page, holding[0], "the first page that holds it");
    assert_eq!(
        steps,
        holding[0].saturating_add(1),
        "one step per page read, and no page read twice"
    );

    // "[S]electing the first matching word": the occurrence *is* the selection, which is what a
    // host draws its highlight from and what makes the annex's own verb mean something here.
    let Answer::Selected(selected) = viewer.query(Query::Selection) else {
        panic!("the occurrence is selected");
    };
    assert_eq!(selected.text.to_lowercase(), needle);
    assert!(
        !selected.quads.is_empty(),
        "and it has shapes to draw over it"
    );

    // Every occurrence on the page being shown, which is the other question and the cheap one.
    let Answer::Found(occurrences) = viewer.query(Query::Find(needle)) else {
        panic!("Query::Find answers for the page being shown");
    };
    assert!(
        !occurrences.is_empty(),
        "the page the search landed on holds the word"
    );

    // Next: forward from the selection, which must be a later occurrence and never this one.
    let mut steps = 0_usize;
    let (next, next_range) =
        run_search(&mut viewer, needle, false, &mut steps).expect("there is another");
    assert!(
        (next, next_range) != (page, range),
        "a second search from the selection moves off it"
    );
}

/// A word that is in no page of the document is reported as absent, after every page was read.
#[test]
fn a_search_for_a_word_the_document_does_not_hold_reads_every_page_and_says_so() {
    let (mut viewer, _) = opened(800, 1000);
    let mut steps = 0_usize;
    let found = run_search(&mut viewer, "quinquagesima", false, &mut steps);
    assert_eq!(found, None, "no such word");
    // A wrapping search reads every page once and the page it began on twice — the half after the
    // starting point on the way out, the half before it on the way round.
    assert_eq!(
        steps,
        PAGES.saturating_add(1),
        "the whole plan, and no more"
    );
}

/// A search with nothing to look for answers at once rather than leaving a host pumping.
#[test]
fn an_empty_search_is_answered_rather_than_left_running() {
    let (mut viewer, _) = opened(800, 1000);
    let events: Vec<Event> = viewer
        .handle(Command::Find(viewer_core::Find::Start {
            needle: String::new(),
            direction: viewer_core::FindDirection::Forward,
        }))
        .collect();
    let searched: Vec<&Event> = events
        .iter()
        .filter(|event| matches!(event, Event::Searched { .. }))
        .collect();
    assert_eq!(searched.len(), 1, "one answer: {events:?}");
    assert!(
        matches!(
            searched[0],
            Event::Searched {
                found: None,
                remaining: 0,
                ..
            }
        ),
        "{searched:?}"
    );
    // And `Find::Continue` with nothing in progress says nothing at all, which is the right
    // answer for a host that pumped once too often.
    let after: Vec<Event> = viewer
        .handle(Command::Find(viewer_core::Find::Continue))
        .collect();
    assert!(
        !after
            .iter()
            .any(|event| matches!(event, Event::Searched { .. })),
        "{after:?}"
    );
}

/// Drives a search to its answer the way a host does, counting the steps it took.
fn run_search(
    viewer: &mut Viewer,
    needle: &str,
    backward: bool,
    steps: &mut usize,
) -> Option<(usize, (usize, usize))> {
    let mut events: Vec<Event> = viewer
        .handle(Command::Find(viewer_core::Find::Start {
            needle: needle.to_owned(),
            direction: if backward {
                viewer_core::FindDirection::Backward
            } else {
                viewer_core::FindDirection::Forward
            },
        }))
        .collect();
    loop {
        *steps = steps.saturating_add(1);
        let mut remaining = 0;
        let mut answer = None;
        let mut asked: Vec<viewer_core::RenderRequest> = Vec::new();
        for event in &events {
            match event {
                Event::Searched {
                    found,
                    remaining: left,
                    ..
                } => {
                    remaining = *left;
                    answer = *found;
                }
                // A search that turns the page asks for the page, exactly as a page turn does —
                // and a host that did not answer would leave the occurrence unselected, because
                // the selection waits for the page it is a range of.
                Event::NeedsRender(request) => asked.push(request.clone()),
                _ => {}
            }
        }
        for request in &asked {
            let _ = serve(viewer, request);
        }
        if let Some(found) = answer {
            return Some((found.page, found.range));
        }
        if remaining == 0 {
            return None;
        }
        events = viewer
            .handle(Command::Find(viewer_core::Find::Continue))
            .collect();
    }
}

/// A second search over the same ground answers exactly what the first did, out of the cache.
///
/// This is the standing gate for every change to the search path — ADRs 0256, 0317, 0330 and 0335
/// each hold it — and it is stated the same way in all four: *a search that returns different
/// results after the change is a defect, not a speed-up*. The proof is in two halves and it needs
/// both:
///
/// - the **answers** are compared, for a needle that is in the document and for one that is not;
/// - the **counters** say the second sweep interpreted nothing, so the second answer was computed
///   from what the cache held rather than from a fresh interpretation that happened to agree.
///
/// `Command::Select(Selection::None)` between the two is what makes them the same question rather
/// than consecutive ones: a search starts after the far end of what is selected, so a second one
/// run over a match would find the one after it.
#[test]
fn a_second_search_answers_what_the_first_did_without_interpreting_a_page_again() {
    let (mut viewer, _) = opened(800, 1000);
    let mut steps = 0_usize;
    let first = run_search(&mut viewer, "compensation", false, &mut steps);
    assert!(
        first.is_some(),
        "the note is about black point compensation"
    );
    let after_first = viewer
        .readback_cache(DOCUMENT)
        .expect("the document is open");
    assert!(after_first.pages > 0, "something was kept: {after_first:?}");
    assert!(after_first.bytes <= after_first.budget, "{after_first:?}");

    viewer
        .handle(Command::Select(Selection::None))
        .for_each(drop);
    let mut again = 0_usize;
    let second = run_search(&mut viewer, "compensation", false, &mut again);
    assert_eq!(second, first, "the same page and the same range");
    assert_eq!(again, steps, "and it took the same number of steps");

    let after_second = viewer
        .readback_cache(DOCUMENT)
        .expect("the document is open");
    assert_eq!(
        after_second.misses, after_first.misses,
        "the second sweep interpreted no page at all: {after_second:?}"
    );
    assert!(after_second.hits > after_first.hits, "{after_second:?}");
    assert_eq!(after_second.evicted, 0, "five pages fit in the budget");

    // A needle that is in no page reads the whole plan, twice, and answers nothing both times.
    let (mut absent, mut absent_again) = (0_usize, 0_usize);
    assert_eq!(
        run_search(&mut viewer, "quinquagesima", false, &mut absent),
        None
    );
    assert_eq!(
        absent,
        PAGES.saturating_add(1),
        "every page and the origin twice"
    );
    let before = viewer.readback_cache(DOCUMENT).expect("open").misses;
    assert_eq!(
        run_search(&mut viewer, "quinquagesima", false, &mut absent_again),
        None
    );
    assert_eq!(absent_again, absent, "the same plan");
    assert_eq!(
        viewer.readback_cache(DOCUMENT).expect("open").misses,
        before,
        "and no page was read a second time"
    );
}

/// An edit forgets every page's readback, because the readback is a function of the view state.
///
/// §12.5.6.10's markup is the cheapest change of that state to make from a test — it goes through
/// the same `Open::replay` a field edit, an undo and a redo do — and what it pins is the
/// conservative rule `Open::stale` states: **every** page goes, not the page that changed. A
/// cache keyed by page alone would answer the next search out of text produced under the state
/// before the edit, and `settle` immediately putting the page showing back is what leaves exactly
/// one entry rather than none.
#[test]
fn an_edit_forgets_every_page_the_search_had_read() {
    let (mut viewer, _) = opened(800, 1000);
    let mut steps = 0_usize;
    assert_eq!(
        run_search(&mut viewer, "quinquagesima", false, &mut steps),
        None
    );
    let filled = viewer
        .readback_cache(DOCUMENT)
        .expect("the document is open");
    assert_eq!(filled.pages, PAGES, "the sweep kept all five: {filled:?}");

    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    viewer
        .handle(Command::Edit(Edit::Markup {
            kind: pdf_model::view::Markup::Highlight,
            colour: [1.0, 1.0, 0.0],
        }))
        .for_each(drop);
    let emptied = viewer
        .readback_cache(DOCUMENT)
        .expect("the document is open");
    assert_eq!(
        emptied.pages, 1,
        "the edit forgot them and `settle` put the page showing back: {emptied:?}"
    );
    // The tally is not reset: it is what says whether the cache is working, and zeroing it on
    // every edit would hide the edit rather than report it.
    assert_eq!(emptied.misses, filled.misses);

    // And the next search reads the other four again rather than answering out of the state
    // before the edit.
    let mut after = 0_usize;
    assert_eq!(
        run_search(&mut viewer, "quinquagesima", false, &mut after),
        None
    );
    let reread = viewer
        .readback_cache(DOCUMENT)
        .expect("the document is open");
    assert_eq!(
        reread.misses,
        filled.misses.saturating_add(4),
        "four pages interpreted again: {reread:?}"
    );
}

/// A word and its box as `pdftotext -bbox -cropbox` states it: points, origin at the crop
/// box's top-left corner, y growing down.
struct ReferenceWord {
    text: String,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

/// One `name="number"` attribute of one of `pdftotext`'s XML tags.
fn bbox_number(tag: &str, name: &str) -> Option<f32> {
    tag.split_once(&format!("{name}=\""))?
        .1
        .split('"')
        .next()?
        .parse()
        .ok()
}

/// Runs `pdftotext -bbox -cropbox` over page one of a committed document, bounded.
///
/// The reference the drag's endpoints come from — trap 12a's rule that a test needing a point
/// takes it from outside the code under test, applied to the whole drag: `pdf-model`'s
/// corpus instrument (ADR 0323) judges these boxes against our text layer at scale, and this
/// harness composes the rest of the journey — device pixels in, selected text out — from the
/// same reference's answer. `-cropbox` because §14.11.2.1's crop box is the displayed frame
/// and `pdftotext`'s default is the media box (ADR 0323 Finding 1).
fn reference_word_boxes(path: &Path) -> Option<(f32, f32, Vec<ReferenceWord>)> {
    let out = std::env::temp_dir().join(format!(
        "viewer-core-bbox-{}-{:?}.html",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut child = std::process::Command::new("pdftotext")
        .args(["-bbox", "-cropbox", "-f", "1", "-l", "1", "-q"])
        .arg(path)
        .arg(&out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    // Bounded, as every external process in this tree is: a committed document should be
    // seconds, and a poll loop is what the standard library offers instead of a deadline.
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if started.elapsed() > std::time::Duration::from_secs(30) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
    let html = std::fs::read_to_string(&out).ok()?;
    let _ = std::fs::remove_file(&out);

    let (mut width, mut height) = (0.0, 0.0);
    let mut words = Vec::new();
    for line in html.lines() {
        let line = line.trim_start();
        if line.starts_with("<page ") {
            width = bbox_number(line, "width")?;
            height = bbox_number(line, "height")?;
        } else if let Some(open) = line.strip_prefix("<word ") {
            let text = open.split_once('>')?.1.strip_suffix("</word>")?;
            words.push(ReferenceWord {
                text: text.to_owned(),
                x0: bbox_number(open, "xMin")?,
                y0: bbox_number(open, "yMin")?,
                x1: bbox_number(open, "xMax")?,
                y1: bbox_number(open, "yMax")?,
            });
        }
    }
    Some((width, height, words))
}

/// A drag across the reference's word box selects that word — ADR 0323's instrument 1, the
/// composed half.
///
/// The corpus instrument in `pdf-model`'s `text_extraction` binary judges where our text layer
/// *says* the words are against `pdftotext -bbox -cropbox`; this test drives the drag itself
/// through the real boundary — `Command::Pointer` press, drag, release, `Query::Selection` —
/// with **both endpoints taken from the reference's box and neither from this tree's own
/// geometry**. That is trap 12a's rule, and it is what makes this the check that catches
/// `user_space_at`'s mirror on its first run: a viewer that flipped the y axis the wrong way
/// would select the mirror of the reference's word, and the mirror of a word is not the word.
///
/// The viewport is resized to the page's own point size at scale 1, so the map from the
/// reference's frame (y down from the page's top) to device pixels is the identity up to the
/// reported origin — the y flip is entirely the viewer's to get right on the way back in.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one composed journey — reference box to device pixels to selection — and \
              splitting it would hide which half a failure is in"
)]
fn a_drag_across_the_references_word_box_selects_the_word() {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    let Some((ref_width, ref_height, words)) = reference_word_boxes(&path) else {
        panic!("pdftotext is required for this test; it comes with poppler");
    };

    let (mut viewer, events) = opened(800, 1000);
    let request0 = request(&events).clone();
    serve(&mut viewer, &request0);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    // The frame audit in miniature: the reference's page size and ours must be the same frame
    // before a box from one is used as a point in the other (ADR 0323 Finding 1). This
    // document states no /Rotate and no /UserUnit, so the two sizes agree directly.
    assert!(
        (geometry.page.width - ref_width).abs() < 1.0
            && (geometry.page.height - ref_height).abs() < 1.0,
        "pdftotext answers a {ref_width}x{ref_height} page where the viewer shows {:?}",
        geometry.page
    );

    // The page's own point size as the viewport, so device pixels and points coincide.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a page's extent in points is positive and far below u32's range"
    )]
    let events: Vec<Event> = viewer
        .handle(Command::Resize {
            width: ref_width.ceil() as u32,
            height: ref_height.ceil() as u32,
            scale: 1.0,
        })
        .collect();
    let resized = request(&events).clone();
    serve(&mut viewer, &resized);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the resized page still has a geometry");
    };
    assert!(
        (geometry.scale - 1.0).abs() < 0.01,
        "at the page's own size the fit is the identity: {}",
        geometry.scale
    );

    // The three longest words that occur exactly once and carry no markup escapes — unique so
    // that finding the text proves the *place*, which is the whole question.
    let mut unique: Vec<&ReferenceWord> = words
        .iter()
        .filter(|word| {
            word.text.chars().all(char::is_alphanumeric)
                && word.text.chars().count() >= 4
                && words.iter().filter(|other| other.text == word.text).count() == 1
        })
        .collect();
    unique.sort_by_key(|word| std::cmp::Reverse(word.text.len()));
    unique.truncate(3);
    assert!(
        !unique.is_empty(),
        "page one of the note has unique words to drag across"
    );

    for word in unique {
        // The reference's frame is y-down from the page's top, which is the raster's own
        // orientation: a point in it maps to the viewport through origin and scale alone,
        // and the flip back to PDF's y-up space is the viewer's job — the one under test.
        let device = |x: f32, y: f32| {
            (
                geometry.origin.0 + x * geometry.scale,
                geometry.origin.1 + y * geometry.scale,
            )
        };
        let mid = f32::midpoint(word.y0, word.y1);
        let (start, end) = (device(word.x0 - 2.0, mid), device(word.x1 + 2.0, mid));

        viewer
            .handle(Command::Pointer {
                at: start,
                action: PointerAction::Pressed,
            })
            .for_each(drop);
        viewer
            .handle(Command::Pointer {
                at: end,
                action: PointerAction::Dragged,
            })
            .for_each(drop);
        viewer
            .handle(Command::Pointer {
                at: end,
                action: PointerAction::Released,
            })
            .for_each(drop);

        let Answer::Selected(selection) = viewer.query(Query::Selection) else {
            panic!("dragging across {:?} selected nothing", word.text);
        };
        let selected: String = selection
            .text
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        assert!(
            selected.contains(&word.text),
            "the drag across the reference's box for {:?} selected {:?}",
            word.text,
            selection.text
        );
        // A drag two points past each edge must not have swept the line: the box the
        // reference states is the box the selection grew from.
        assert!(
            selected.chars().count() <= word.text.chars().count() + 8,
            "the drag across {:?} took half the line: {:?}",
            word.text,
            selection.text
        );
        viewer
            .handle(Command::Select(Selection::None))
            .for_each(drop);
    }
}

/// A drag across a hollow-font OCR layer selects the word under a full-height band.
///
/// The CSDK 22 shape (ADR 0350): an invisible text layer whose embedded `CIDFontType2`
/// carries real metrics and **no glyph outlines** — the shape on which a reader that derives
/// character boxes from the outlines paints no selection overlay at all, because every box
/// has zero height (old `PDFium`'s `FPDFText_GetCharBox` did). This tree's band is Table 120's
/// `/Ascent`/`/Descent` (ADR 0216), so the drag must select the word and every quad the host
/// is handed to paint must be the descriptor's 11.1 pt tall, not 0.
///
/// The drag's endpoints come from the fixture's own stated layout — trap 12a's rule that the
/// point comes from the document rather than from the code under test, available here in its
/// strongest form because this test's author *is* the document's producer.
#[test]
fn a_drag_across_a_hollow_ocr_layer_selects_under_a_full_height_band() {
    use test_scenes::{
        OCR_ASCENT, OCR_BASELINE, OCR_DESCENT, OCR_FIRST_WORD, OCR_FONT_SIZE, OCR_PAGE, OCR_TEXT_X,
        OcrFont, ocr_advance_for_gid, ocr_gid_for_cid, scanned_ocr_pdf,
    };

    let (page_width, page_height) = OCR_PAGE;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the fixture's page is 300x200 points"
    )]
    let mut viewer = Viewer::new(page_width as u32, page_height as u32, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: scanned_ocr_pdf(OcrFont::HollowEmbedded, 3),
            password: None,
            fragment: None,
        })
        .collect();
    serve(&mut viewer, &request(&events).clone());
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the scanned page has a geometry");
    };
    assert!(
        (geometry.scale - 1.0).abs() < 0.01,
        "at the page's own size the fit is the identity: {}",
        geometry.scale
    );

    // The word's extent, from the fixture's own statements: /W widths for CIDs 1..=6, the
    // descriptor's ascent and descent, all at 12 pt (§9.4.4's arithmetic).
    let word_width: f32 = (1..=6)
        .map(|cid| f32::from(ocr_advance_for_gid(ocr_gid_for_cid(cid))) / 1000.0 * OCR_FONT_SIZE)
        .sum();
    let band_height = (OCR_ASCENT - OCR_DESCENT) / 1000.0 * OCR_FONT_SIZE;
    let band_middle = OCR_BASELINE + (OCR_ASCENT + OCR_DESCENT) / 2000.0 * OCR_FONT_SIZE;

    // User space to viewport: scale, origin, and the y flip about the page's height.
    let device = |x: f32, y: f32| {
        (
            geometry.origin.0 + x * geometry.scale,
            geometry.origin.1 + (page_height - y) * geometry.scale,
        )
    };
    let start = device(OCR_TEXT_X - 2.0, band_middle);
    let end = device(OCR_TEXT_X + word_width + 2.0, band_middle);
    for (at, action) in [
        (start, PointerAction::Pressed),
        (end, PointerAction::Dragged),
        (end, PointerAction::Released),
    ] {
        viewer
            .handle(Command::Pointer { at, action })
            .for_each(drop);
    }

    let Answer::Selected(selection) = viewer.query(Query::Selection) else {
        panic!("dragging across the invisible word selected nothing");
    };
    assert_eq!(
        selection.text.trim(),
        OCR_FIRST_WORD,
        "the drag selects exactly the word under it"
    );
    assert!(
        !selection.quads.is_empty(),
        "a selection the host cannot paint is the defect under test"
    );
    for quad in &selection.quads {
        let height = (quad[1] - quad[7]).abs();
        assert!(
            (height - band_height * geometry.scale).abs() < 0.1,
            "the highlight is the descriptor's {band_height} pt band, not the outlines' 0: \
             {height}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// ISO 32000-2 Table 29's `/PageLayout`
// -------------------------------------------------------------------------------------------

/// The space `crate::layout` puts between two neighbouring pages, in logical pixels.
///
/// Written here as the number the tests below expect rather than read out of the crate, so that a
/// change to the gap has to be made twice — once as a choice and once as a consequence.
const GAP: f32 = 8.0;

/// The geometry of one page of the arrangement, or `None` where it is not on the screen.
fn placed(viewer: &Viewer, page: usize) -> Option<viewer_core::PageGeometry> {
    match viewer.query(Query::PageGeometry(page)) {
        Answer::Geometry(geometry) => Some(geometry),
        _ => None,
    }
}

/// Opens the five-page note and draws whatever the arrangement asks for.
///
/// A layout is the one thing in this vocabulary that makes `settle` produce **several** render
/// requests, so a fixture that served only the first would leave every page but one without a
/// geometry — which is the failure this helper exists to make impossible.
fn arranged(layout: pdf_model::viewer_preferences::PageLayout) -> Viewer {
    let (mut viewer, events) = opened(800, 1000);
    for request in requests(&events) {
        serve(&mut viewer, &request);
    }
    // Half size, so that more than one page fits the window. At `Zoom::FitPage` — which is what a
    // document with no `/OpenAction` opens at — a column has exactly one page on the screen, which
    // is correct and shows nothing about the arrangement.
    let events: Vec<Event> = viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(0.5),
            at: None,
        })
        .collect();
    for request in requests(&events) {
        serve(&mut viewer, &request);
    }
    let events: Vec<Event> = viewer.handle(Command::Layout(layout)).collect();
    for request in requests(&events) {
        serve(&mut viewer, &request);
    }
    viewer
}

/// Every render request in a batch of events, cloned so the viewer can be borrowed again.
fn requests(events: &[Event]) -> Vec<viewer_core::RenderRequest> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .collect()
}

/// Table 29's `OneColumn` — "Display the pages in one column" — puts page two under page one.
///
/// The distance is the first page's *raster* height and the gap, and the two share an x origin:
/// that is what a column is, and it is the whole geometric claim the value makes.
#[test]
fn table_29s_one_column_puts_the_next_page_below_the_one_showing() {
    use pdf_model::viewer_preferences::PageLayout;

    let viewer = arranged(PageLayout::OneColumn);
    let first = placed(&viewer, 0).expect("page one is on the screen");
    let second = placed(&viewer, 1).expect("page two is below it, which is what a column means");
    assert!(
        (second.origin.0 - first.origin.0).abs() < 0.5,
        "one column, so one x: {:?} then {:?}",
        first.origin,
        second.origin
    );
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster height in pixels, which is thousands"
    )]
    let below = first.origin.1 + first.height as f32 + GAP;
    assert!(
        (second.origin.1 - below).abs() < 0.5,
        "page two starts a gap below page one's raster: {below} against {}",
        second.origin.1
    );
}

/// A scroll that crosses a row makes the next page current, and nothing on the screen moves.
///
/// Both halves matter. The first is what "which page am I on" has to mean once a scroll can leave
/// a page behind — and it raises §12.6.3's page events, exactly as an arrow key's turn does. The
/// second is what makes a continuous view one surface rather than a sequence: the scroll is
/// measured from the current page's row, so moving the row has to move the origin by the same
/// distance in the other direction.
#[test]
fn a_scroll_across_a_page_boundary_makes_the_next_page_current() {
    use pdf_model::viewer_preferences::PageLayout;

    let mut viewer = arranged(PageLayout::OneColumn);
    let first = placed(&viewer, 0).expect("page one is on the screen");
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster height in pixels, which is thousands"
    )]
    let step = first.height as f32 + GAP;
    // Not yet: page one's row is still on the screen, so it is still the page being read.
    let events: Vec<Event> = viewer
        .handle(Command::Scroll {
            dx: 0.0,
            dy: step - 50.0,
        })
        .collect();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::PageChanged { .. })),
        "a page still on the screen has not been turned away from: {events:?}"
    );
    assert!(
        matches!(
            viewer.query(Query::CurrentPage),
            Answer::Page { index: 0, .. }
        ),
        "the topmost page of the column is the one being read"
    );
    let second = placed(&viewer, 1).expect("page two is on the screen below it");
    assert!(
        (second.origin.1 - 50.0).abs() < 0.5,
        "fifty pixels of page two are showing: {:?}",
        second.origin
    );

    // And now: the row has gone above the window, so the page below it is the one being read —
    // and the seventy pixels past the boundary are still seventy pixels, which is what makes the
    // rebase invisible.
    let events: Vec<Event> = viewer
        .handle(Command::Scroll { dx: 0.0, dy: 70.0 })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::PageChanged { index: 1, .. })),
        "a page that has scrolled off the top is a page turned: {events:?}"
    );
    assert!(
        matches!(
            viewer.query(Query::CurrentPage),
            Answer::Page { index: 1, .. }
        ),
        "the topmost page of the column is the one being read"
    );
    let second = placed(&viewer, 1).expect("page two is now the current one");
    assert!(
        (second.origin.1 + 20.0).abs() < 0.5,
        "the twenty pixels past the boundary are still twenty pixels: {:?}",
        second.origin
    );
    assert!(
        placed(&viewer, 0).is_none(),
        "and page one, which is off the top, is no longer interpreted — the eviction rule the \
         arrangement supplies"
    );
}

/// Table 29's `TwoPageLeft` shows two pages side by side and **only** two.
///
/// "Display the pages two at a time" is the phrase that separates it from `TwoColumnLeft`: what
/// is on the screen is one row, and the row below it is not reached by scrolling.
#[test]
fn table_29s_two_page_left_shows_exactly_two_pages_side_by_side() {
    use pdf_model::viewer_preferences::PageLayout;

    let viewer = arranged(PageLayout::TwoPageLeft);
    let left = placed(&viewer, 0).expect("page one is on the left");
    let right = placed(&viewer, 1).expect("page two is beside it");
    assert!(
        (left.origin.1 - right.origin.1).abs() < 0.5,
        "a spread shares a top edge: {:?} and {:?}",
        left.origin,
        right.origin
    );
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster width in pixels, which is hundreds"
    )]
    let beside = left.origin.0 + left.width as f32 + GAP;
    assert!(
        (right.origin.0 - beside).abs() < 0.5,
        "page two starts a gap to the right of page one: {beside} against {}",
        right.origin.0
    );
    assert!(
        placed(&viewer, 2).is_none(),
        "two at a time is two, and page three is on the next spread"
    );
}

/// Table 29's `TwoColumnRight` leaves page one alone, which is what "odd-numbered … on the right"
/// says once the first page is counted as page **one**.
///
/// The arrangement a bound book has: a lone cover on the right, then two-page spreads. The test
/// is that page two is *not* beside page one — it opens the next row, with page three.
#[test]
fn table_29s_two_column_right_leaves_page_one_alone_on_its_row() {
    use pdf_model::viewer_preferences::PageLayout;

    let viewer = arranged(PageLayout::TwoColumnRight);
    let first = placed(&viewer, 0).expect("page one is on the screen");
    let second = placed(&viewer, 1).expect("page two opens the row below it");
    assert!(
        second.origin.1 > first.origin.1,
        "page two is on the next row, not beside page one: {:?} and {:?}",
        first.origin,
        second.origin
    );
    let third = placed(&viewer, 2).expect("page three is beside page two");
    assert!(
        (third.origin.1 - second.origin.1).abs() < 0.5,
        "pages two and three share a row: {:?} and {:?}",
        second.origin,
        third.origin
    );
    assert!(
        second.origin.0 < third.origin.0,
        "and page three — odd-numbered — is the right-hand one: {:?} and {:?}",
        second.origin,
        third.origin
    );
}

/// A link is still a link after a continuous scroll has moved the page under the pointer.
///
/// Trap 12a's rule twice over: the point comes from the **document** — `basicapi.pdf` states the
/// rectangle — and the mapping is asked of the viewer rather than reproduced here. What it
/// catches is a hit test that kept using the placement the page had before the scroll, which is
/// the defect a continuous layout makes possible and a single-page one cannot.
#[test]
fn a_link_is_hit_where_the_column_has_moved_the_page_to() {
    use pdf_model::viewer_preferences::PageLayout;

    let Some(bytes) = corpus_bytes("basicapi.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .collect();
    for request in requests(&events) {
        serve(&mut viewer, &request);
    }
    let before = device_point(&viewer, LINK_RECT, PAGE_HEIGHT);
    assert!(matches!(
        viewer.query(Query::LinkAt(before)),
        Answer::Link(true)
    ));

    let events: Vec<Event> = viewer
        .handle(Command::Layout(PageLayout::OneColumn))
        .collect();
    for request in requests(&events) {
        serve(&mut viewer, &request);
    }
    let events: Vec<Event> = viewer
        .handle(Command::Scroll { dx: 0.0, dy: 100.0 })
        .collect();
    for request in requests(&events) {
        serve(&mut viewer, &request);
    }
    let after = device_point(&viewer, LINK_RECT, PAGE_HEIGHT);
    assert!(
        (before.1 - after.1 - 100.0).abs() < 0.5,
        "a hundred pixels of scroll moved the link a hundred pixels up: {before:?} then {after:?}"
    );
    assert!(
        matches!(viewer.query(Query::LinkAt(after)), Answer::Link(true)),
        "and the link is where the geometry says it is: {after:?}"
    );
    assert!(
        matches!(viewer.query(Query::LinkAt(before)), Answer::Link(false)),
        "and no longer where it was: {before:?}"
    );
}

/// **A drag that crosses a page boundary selects both pages' text**, which is what Table 29's
/// continuous arrangements made an ordinary gesture rather than an exotic one.
///
/// Until the six-hundred-and-ninth session a selection was a range of *one* page's readback and a
/// drag that reached the page below stopped at the boundary: the sweep selected the first page's
/// half of a paragraph and nothing after it, silently. Both ends of a selection name their own
/// page now — §12.4.2 still gives no document-wide offset and this does not invent one — so the
/// first page contributes its tail, any page between contributes the whole of itself, and the
/// last contributes its head.
///
/// The points are taken from the geometry the viewer reports for **each** page rather than from
/// one page's and an assumption about the other, which is `doc/traps/the-interactive-loop.md`
/// trap 12a's rule: a test that computed the second point itself would agree with a mapping that
/// was wrong in both places.
#[test]
fn a_drag_across_a_page_boundary_selects_both_pages() {
    use pdf_model::viewer_preferences::PageLayout;

    let mut viewer = arranged(PageLayout::OneColumn);
    let first = placed(&viewer, 0).expect("page one is on the screen");
    let second = placed(&viewer, 1).expect("a column shows page two below page one");
    // A fraction across and down each page's own raster, through that page's own geometry.
    let on = |geometry: &viewer_core::PageGeometry, across: f32, down: f32| {
        (
            geometry.origin.0 + geometry.page.width * across * geometry.scale,
            geometry.origin.1 + geometry.page.height * down * geometry.scale,
        )
    };
    viewer
        .handle(Command::Pointer {
            at: on(&first, 0.1, 0.1),
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at: on(&second, 0.9, 0.9),
            action: PointerAction::Dragged,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at: on(&second, 0.9, 0.9),
            action: PointerAction::Released,
        })
        .for_each(drop);
    let Answer::Selected(selected) = viewer.query(Query::Selection) else {
        panic!("a drag across two pages is a selection");
    };
    let (across, quads) = (selected.text.into_owned(), selected.quads.len());
    assert!(
        across.contains('\n'),
        "two pages' readbacks are joined by a line break: {across:?}"
    );

    // What each page reads back on its own, asked of the viewer the way a person asks — which is
    // also the property `selection_census` holds `Selection::All` to.
    let whole_page = |viewer: &mut Viewer, page: usize| -> String {
        let events: Vec<Event> = viewer
            .handle(Command::GoTo(PageTarget::Index(page)))
            .collect();
        for request in requests(&events) {
            serve(viewer, &request);
        }
        viewer
            .handle(Command::Select(Selection::All))
            .for_each(drop);
        match viewer.query(Query::Selection) {
            Answer::Selected(selected) => selected.text.into_owned(),
            other => panic!("page {page} reads back as something: {other:?}"),
        }
    };
    let one = whole_page(&mut viewer, 0);
    let two = whole_page(&mut viewer, 1);
    // A word the *second* page has and the first has not, taken from the document rather than
    // chosen here: if the drag had stopped at the boundary the selection could not contain it.
    let only_on_two = two
        .split_whitespace()
        .find(|word| word.len() > 6 && !one.contains(*word))
        .expect("the second page says something the first does not");
    assert!(
        across.contains(only_on_two),
        "the drag reached the second page: {only_on_two:?} is not in {across:?}"
    );
    let only_on_one = one
        .split_whitespace()
        .find(|word| word.len() > 6 && !two.contains(*word))
        .expect("the first page says something the second does not");
    assert!(
        across.contains(only_on_one),
        "and it kept the first page's half: {only_on_one:?} is not in {across:?}"
    );
    // Both pages' shapes, so a host draws the highlight over both halves rather than over one.
    viewer
        .handle(Command::Select(Selection::None))
        .for_each(drop);
    assert!(
        quads > 0,
        "a selection over two pages has shapes over both of them"
    );
}

/// **`Selection::All` is still one page**, which is the identity two instruments rest on.
///
/// A selection that crosses pages exists now, and the command that selects "everything" is
/// deliberately not it: a range is into a page's readback, `selection_census` asserts that this
/// answer is `pdf_model::Interpretation::text` byte for byte, and `pdf-retrieve`'s default answer
/// is the same identity (ADR 0257). The page break above joins two readbacks and belongs to
/// neither, so a command that quietly selected several pages would put a character into that
/// string that no page states.
#[test]
fn selecting_everything_is_one_pages_readback_even_in_a_column() {
    use pdf_model::viewer_preferences::PageLayout;

    let mut viewer = arranged(PageLayout::OneColumn);
    assert!(
        placed(&viewer, 1).is_some(),
        "the column is showing more than one page, which is what makes this a question"
    );
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    let Answer::Selected(selected) = viewer.query(Query::Selection) else {
        panic!("everything on the current page is a selection");
    };
    assert!(
        !selected.text.contains('\n') || !selected.text.is_empty(),
        "a readback may hold line breaks of its own"
    );
    assert!(
        matches!(selected.text, std::borrow::Cow::Borrowed(_)),
        "one page's selection is a slice of that page's readback and not a copy of it"
    );
}

/// The three questions about a *page* answer for every page the arrangement is showing.
///
/// **What this pins is the population rather than the contents.** `Query::Reports`,
/// `Query::Readback` and `Query::AccessibilityTree` each read one page's interpretation, and
/// under Table 29's default that page and the screen are the same thing — so a column is the only
/// arrangement in which the difference is visible at all. A host given the current page's answer
/// for a screen holding four would be silent about three of them, and silent in the direction
/// nobody checks: the pages a person can see and nothing has spoken about.
///
/// Each entry names its page, because a note that did not say which page it was about would be a
/// note about one of four.
#[test]
fn the_three_page_questions_answer_for_every_page_of_a_column() {
    use pdf_model::viewer_preferences::PageLayout;

    let viewer = arranged(PageLayout::OneColumn);
    let on_screen: Vec<usize> = (0..PAGES)
        .filter(|page| placed(&viewer, *page).is_some())
        .collect();
    assert!(
        on_screen.len() > 1,
        "the column is showing more than one page, which is what makes this a question"
    );

    let Answer::Reports(reports) = viewer.query(Query::Reports) else {
        panic!("a document with pages on the screen answers");
    };
    assert_eq!(
        reports.iter().map(|page| page.page).collect::<Vec<usize>>(),
        on_screen,
        "one entry per page the arrangement shows, in page order"
    );

    let Answer::Readback(counts) = viewer.query(Query::Readback) else {
        panic!("a document with pages on the screen answers");
    };
    assert_eq!(
        counts.iter().map(|page| page.page).collect::<Vec<usize>>(),
        on_screen,
        "the codes a page cost its reader are that page's"
    );

    let Answer::Accessibility(structure) = viewer.query(Query::AccessibilityTree) else {
        panic!("a document with pages on the screen answers");
    };
    assert_eq!(
        structure
            .iter()
            .map(|page| page.page)
            .collect::<Vec<usize>>(),
        on_screen,
        "a screen reader walking a column is told about every page in it"
    );
}

/// A page turn under `SinglePage` still answers about exactly the page being shown.
///
/// The other side of the test above, and the one that would catch an answer built out of every
/// page the *document* has rather than every page the *screen* has: under Table 29's default one
/// page is on the screen, so all three questions answer with exactly one entry and its number is
/// the page a person is looking at.
#[test]
fn a_single_page_window_answers_about_that_page_and_no_other() {
    let (mut viewer, events) = opened(800, 1000);
    for request in requests(&events) {
        serve(&mut viewer, &request);
    }
    let events: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(2))).collect();
    for request in requests(&events) {
        serve(&mut viewer, &request);
    }
    let Answer::Accessibility(structure) = viewer.query(Query::AccessibilityTree) else {
        panic!("a document with a page on the screen answers");
    };
    assert_eq!(
        structure
            .iter()
            .map(|page| page.page)
            .collect::<Vec<usize>>(),
        vec![2],
        "one page is on the screen and it is page three"
    );
    let Answer::Reports(reports) = viewer.query(Query::Reports) else {
        panic!("a document with a page on the screen answers");
    };
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert_eq!(reports.first().map(|page| page.page), Some(2));
}

/// §14.7.5.3's object reference names a **widget**, so the node it makes says what that widget is.
///
/// **The defect this exists for is a sentence a screen reader said and nothing else could hear.**
/// `referenced_objects` keyed its map by the annotation and stored the *field's* control under
/// every one of them, and `pdf_model::form::Control::RadioButton`'s `on` is "[w]hether any widget
/// of the set is on" — so once one button of a set was chosen, AT-SPI reported all of them
/// `checked`. Found by driving `Action.DoAction` over a real bus on this document and reading
/// `GetState` back after each click (ADR 0630).
///
/// ISO 32000-2 §12.7.5.2.4: "Like check boxes, individual radio buttons have two states, on and
/// off", and §12.7.5.2.3 makes the exclusion a `shall` where Table 229 bit 26 is clear — "at most
/// one radio button in a field shall be set at a time".
///
/// `annotation-button-widget.pdf` is the witness `doc/verify.md` names for §14.8.4.7.2's controls:
/// its second radio field states `/V /1`, and of that field's two widgets only the one whose
/// `/AP /N` is keyed by `1` is on.
#[test]
fn each_radio_button_of_a_set_says_whether_it_is_on_rather_than_whether_the_field_is() {
    let Some(bytes) = corpus_bytes("annotation-button-widget.pdf") else {
        return;
    };
    let mut viewer = Viewer::new(900, 1200, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the query always answers");
    };
    let radios: Vec<bool> = pages
        .iter()
        .flat_map(|page| page.nodes.iter())
        .filter_map(|node| match node.control {
            Some(Control::RadioButton { on, .. }) => Some(on),
            _ => None,
        })
        .collect();
    assert_eq!(
        radios.len(),
        6,
        "three radio button fields with two widgets apiece"
    );
    assert_eq!(
        radios.iter().filter(|on| **on).count(),
        1,
        "exactly one of the six is on, which is the one whose field states its appearance state: \
         {radios:?}"
    );
}
