//! A frame's three payloads: marks, window, raster, and the route.

use super::*;

/// One frame carrying ADR 0607's list arm, written the way [`encode_answer`] writes it.
///
/// Hand-built rather than taken off a [`Marks`], because [`viewer_core::RenderToken`] is
/// opaque and nothing outside its own crate can mint one — which is correct and is why the
/// *choosing* half is tested end to end, against the real worker, in `tests/confined.rs`.
/// What is under test here is the half a hostile worker controls: the bytes.
fn a_list_frame(width: u32, height: u32) -> Vec<u8> {
    use pdf_render::{
        BlendMode, Color, Command as Mark, DisplayList, FillRule, Paint, Path, Transform,
    };

    let mut list = DisplayList::new(Size::new(10.0, 20.0));
    list.push(Mark::Fill {
        path: std::sync::Arc::new(Path::new()),
        paint: Paint::Solid(Color::BLACK),
        fill_rule: FillRule::NonZero,
        transform: Transform::IDENTITY,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    let mut writer = Writer::new();
    writer.u8(answer_kind::FRAME).usize(1);
    writer.usize(0).u8(PAYLOAD_LIST).u32(width).u32(height);
    display_list::write_transform(&mut writer, Transform::IDENTITY);
    writer.bytes(&display_list::encode(&list).expect("a one-command page encodes"));
    writer.point((1.5, -2.5)).bool(false);
    writer.finish()
}

/// The list arm decodes, and it decodes into the target the message named.
///
/// The control for the three refusals below: a bound that refused everything would pass all
/// of them and carry no page at all.
#[test]
fn a_frame_crossing_as_marks_decodes_into_a_target_a_host_can_draw() {
    let Reply::Frame(frames) = decode_answer(&a_list_frame(100, 200)).expect("a frame") else {
        panic!("a frame answer decodes as one");
    };
    let [shown] = frames.as_slice() else {
        panic!("one page was written")
    };
    assert_eq!(shown.origin, (1.5, -2.5));
    let crate::Payload::List { list, target } = &shown.payload else {
        panic!("the marks were written, so the marks are what crossed")
    };
    assert_eq!((target.width, target.height), (100, 200));
    assert_eq!(list.page_size, Size::new(10.0, 20.0));
    assert_eq!(list.commands().len(), 1);
}

/// The list of one page in a frame, for the reuse tests below.
fn the_list_of(reply: &Reply) -> std::sync::Arc<pdf_render::DisplayList> {
    let Reply::Frame(frames) = reply else {
        panic!("a frame answer decodes as one");
    };
    let [shown] = frames.as_slice() else {
        panic!("one page was written")
    };
    let crate::Payload::List { list, .. } = &shown.payload else {
        panic!("the marks were written, so the marks are what crossed")
    };
    std::sync::Arc::clone(list)
}

/// A page whose bytes did not change comes back as the same `Arc` — across two frames, which
/// is where every real scroll and zoom lives (ADR 0725).
///
/// The worker re-sends a page's encoded marks unchanged until a re-render replaces them
/// ([`Marks`] holds them encoded), so byte-identical re-crossings are the ordinary case. The
/// contrast decode at the end is the defect this exists to pin: two stateless decodes are
/// exactly what [`crate::Confined::query`] did for the whole of this crate's life before,
/// and they hand back two identities for one unchanged page — which is why the screen's
/// same-drawing-moved arm and every `Arc`-keyed cache behind a host missed on every frame.
#[test]
fn an_unchanged_list_recrosses_as_the_same_arc() {
    let mut held = HeldLists::default();
    let first = decode_answer_reusing(&a_list_frame(100, 200), &mut held).expect("a frame");
    let second = decode_answer_reusing(&a_list_frame(100, 200), &mut held).expect("a frame");
    assert!(
        std::sync::Arc::ptr_eq(&the_list_of(&first), &the_list_of(&second)),
        "the same bytes are the same list"
    );

    // A zoom re-renders the page: the *same* marks re-encode under a new target, so the
    // identity survives while the target moves — which is what makes a zoom a new
    // `TargetSpec` over the same list, as `Payload::List` promises.
    let zoomed = decode_answer_reusing(&a_list_frame(200, 400), &mut held).expect("a frame");
    assert!(
        std::sync::Arc::ptr_eq(&the_list_of(&second), &the_list_of(&zoomed)),
        "a new target over unchanged bytes keeps the list's identity"
    );

    // The contrast: stateless decodes — what the host did before — give two identities.
    let one = decode_answer(&a_list_frame(100, 200)).expect("a frame");
    let two = decode_answer(&a_list_frame(100, 200)).expect("a frame");
    assert!(
        !std::sync::Arc::ptr_eq(&the_list_of(&one), &the_list_of(&two)),
        "a throwaway state cannot carry an identity, which is why Confined keeps one"
    );
}

/// What is held is bounded by the frame on hand: a page that stops crossing as marks is
/// forgotten, and the same bytes arriving later are a new list rather than a stale one.
///
/// The bound is the point — the host-side store must not grow by one decoded list per page a
/// reader ever saw — and forgetting on the pixels arm is what keeps "the same bytes" a
/// statement about the frame on hand rather than about history.
#[test]
fn a_page_that_stops_crossing_as_marks_is_forgotten() {
    let mut held = HeldLists::default();
    let first = decode_answer_reusing(&a_list_frame(100, 200), &mut held).expect("a frame");

    // The same page crosses as pixels: one 2×2 raster, page 0, any origin.
    let mut writer = Writer::new();
    writer.u8(answer_kind::FRAME).usize(1);
    writer.usize(0).u8(PAYLOAD_RASTER).u32(2).u32(2).u8(0);
    writer.bytes(&[0u8; 16]);
    writer.point((0.0, 0.0)).bool(false);
    let pixels = decode_answer_reusing(&writer.finish(), &mut held).expect("a frame");
    assert!(matches!(
        &pixels,
        Reply::Frame(frames) if matches!(frames[0].payload, crate::Payload::Raster(_))
    ));

    let third = decode_answer_reusing(&a_list_frame(100, 200), &mut held).expect("a frame");
    assert!(
        !std::sync::Arc::ptr_eq(&the_list_of(&first), &the_list_of(&third)),
        "the pixels arm evicted the held list, so the same bytes decode afresh"
    );
}

/// A page the worker drew as pixels says whether its draw declined a mark, after its origin, so
/// the host can name it in its title; the payload's bytes are the payload's alone (ADR 2316).
#[test]
fn a_page_drawn_as_pixels_says_its_draw_declined_a_mark() {
    for declined in [true, false] {
        let mut writer = Writer::new();
        writer.u8(answer_kind::FRAME).usize(1);
        writer.usize(3).u8(PAYLOAD_RASTER).u32(2).u32(2).u8(0);
        writer.bytes(&[0u8; 16]);
        writer.point((4.0, 5.0)).bool(declined);
        let Reply::Frame(frames) = decode_answer(&writer.finish()).expect("a frame") else {
            panic!("a frame answer decodes as one");
        };
        let [shown] = frames.as_slice() else {
            panic!("one page was written")
        };
        assert_eq!(
            (shown.page, shown.origin, shown.declined),
            (3, (4.0, 5.0), declined)
        );
    }
}

/// **A frame answer holding no pixels at all still carries the page**, out of this store.
///
/// That is the whole of what ADR 0640 moved on this side. A page whose marks are what
/// crosses now answers [`viewer_core::Rendered::Listed`], so the worker draws nothing for it,
/// the viewer holds nothing of it and [`Answer::Frame`] is silent about it — and the reply
/// has to carry it anyway, at the origin the viewer places it at. Before the change the page
/// was in that answer because the worker had drawn a raster it then threw away, so an empty
/// `Answer::Frame` produced an empty reply.
///
/// Driven by a real [`viewer_core::Viewer`] because [`viewer_core::RenderToken`] is opaque
/// and nothing outside that crate can mint one. The document is the committed note, which
/// every checkout has, and its first page is sparse — which is what makes [`Carries::Marks`]
/// the answer rather than an assumption.
#[test]
fn a_page_the_viewer_holds_no_pixels_of_still_crosses_as_marks() {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
    )
    .expect("the committed note is readable");
    let mut viewer = viewer_core::Viewer::new(900, 1200, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = events
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .expect("opening a document asks for its first page");
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(request.page)) else {
        panic!("the page a render was asked for is a page the arrangement places");
    };

    let mut marks = Marks::default();
    assert_eq!(
        marks.decide(&request, Some(geometry.origin)),
        Carries::Marks,
        "a page of text is a small fraction of its own pixels"
    );
    marks.place(|page| (page == request.page).then_some(geometry.origin));

    // An empty list, which is exactly what the viewer answers once the only page on the
    // screen has been taken as marks.
    let encoded = encode_answer(&Answer::Frame(Vec::new()), &marks).expect("a frame crosses");
    let Reply::Frame(frames) = decode_answer(&encoded).expect("what was written reads back") else {
        panic!("a frame answer decodes as one");
    };
    let [only] = frames.as_slice() else {
        panic!("the page this store holds is the page that crossed: {frames:?}");
    };
    assert_eq!(only.page, request.page);
    assert_eq!(only.origin, geometry.origin);
    let crate::Payload::List { target, .. } = &only.payload else {
        panic!("the marks are what this store holds: {only:?}");
    };
    assert_eq!(
        (target.width, target.height),
        (request.target.width, request.target.height)
    );

    // And the eviction rule: a page the viewer will not place is one this store forgets, so
    // the reply goes back to carrying nothing rather than a page that has scrolled away.
    marks.place(|_| None);
    let encoded = encode_answer(&Answer::Frame(Vec::new()), &marks).expect("a frame crosses");
    let Reply::Frame(frames) = decode_answer(&encoded).expect("what was written reads back") else {
        panic!("a frame answer decodes as one");
    };
    assert!(frames.is_empty(), "{frames:?}");
}

/// A page past the bound whose marks cannot cross is drawn here as the window shows it, and
/// crosses as that window, to a host that said it draws such a page and to no other; the
/// window is drawn again only where a scroll leaves it (ADR 1986).
#[test]
fn a_window_drawn_here_crosses_where_the_page_could_not() {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
    )
    .expect("the committed note is readable");
    let mut viewer = viewer_core::Viewer::new(800, 1000, 1.0);
    let mut request = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("opening a document asks for its first page");
    // The page at a target past the bound, its corner a thousand pixels up and left of the
    // viewport's: the viewport shows columns 1 000 to 1 799 and rows 1 000 to 1 999.
    request.target.width = 40_000;
    request.target.height = 50_000;
    let viewport = (800, 1000);
    let mut marks = Marks::default();
    marks.hold_window(&request, (-1000.0, -1000.0));

    // Nothing crosses until a window is drawn.
    let empty = encode_answer(&Answer::Frame(Vec::new()), &marks).expect("a frame crosses");
    let mut told = HeldLists::default();
    told.told(&Command::Oversize(viewer_core::Oversize::Listed));
    let Ok(Reply::Frame(frames)) = decode_answer_reusing(&empty, &mut told) else {
        panic!("a frame answer decodes as one");
    };
    assert!(frames.is_empty(), "{frames:?}");

    let owed = marks.windows_owed(viewport);
    let [(page, _, target, window)] = owed.as_slice() else {
        panic!("the one page held owes one window: {owed:?}");
    };
    assert_eq!((*page, *target), (request.page, request.target));
    assert_eq!(
        (window.x, window.y, window.width, window.height),
        (800, 750, 1200, 1500)
    );
    let raster = Raster {
        width: window.width,
        height: window.height,
        format: RasterFormat::Rgba8,
        data: vec![7; 1200 * 1500 * 4],
    };
    marks.drew(*page, *window, raster.clone());

    let encoded = encode_answer(&Answer::Frame(Vec::new()), &marks).expect("a frame crosses");
    let Ok(Reply::Frame(frames)) = decode_answer_reusing(&encoded, &mut told) else {
        panic!("a host that draws past the bound reads the window");
    };
    let [only] = frames.as_slice() else {
        panic!("the window is the page that crossed: {frames:?}");
    };
    assert_eq!((only.page, only.origin), (request.page, (-1000.0, -1000.0)));
    assert_eq!(
        only.payload,
        crate::Payload::Window {
            target: request.target,
            corner: (800, 750),
            raster,
        }
    );
    // A host that never said so is handed nothing it did not ask for.
    assert!(decode_answer(&encoded).is_err());

    // A scroll inside the margin owes nothing; one past it owes the next window.
    marks.place(|_| Some((-1150.0, -1100.0)));
    assert!(marks.windows_owed(viewport).is_empty());
    marks.place(|_| Some((-1300.0, -1000.0)));
    assert_eq!(marks.windows_owed(viewport).len(), 1);
    // A refused window lets the page go, naming its document for the sentence.
    assert_eq!(marks.refused_window(request.page), Some(DocumentId(1)));
    assert!(marks.windows_owed(viewport).is_empty());
}

/// **A page the host asked for as windows is not encoded again** (ADR 2017): its next render
/// is held as windows without the list's encoding, which on the owner's witness is 0.6 s a
/// notch (ADR 1986 §1); and asked back, its next render crosses as its marks.
#[test]
fn a_page_asked_for_as_windows_is_held_as_windows_at_its_next_render() {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
    )
    .expect("the committed note is readable");
    let mut viewer = viewer_core::Viewer::new(800, 1000, 1.0);
    let mut request = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("opening a document asks for its first page");
    request.target.width = 40_000;
    request.target.height = 50_000;
    let mut marks = Marks::default();
    let origin = Some((-1000.0, -1000.0));
    assert_eq!(marks.decide(&request, origin), Carries::Marks);
    marks.route(request.page, crate::Route::Window);
    assert_eq!(marks.decide(&request, origin), Carries::Window);
    assert!(marks.by_page.is_empty() && marks.windows.contains_key(&request.page));
    marks.route(request.page, crate::Route::Marks);
    assert_eq!(marks.decide(&request, origin), Carries::Marks);
    assert!(marks.windows.is_empty());
    // A page that leaves the arrangement is forgotten with the asking.
    marks.route(request.page, crate::Route::Window);
    marks.place(|_| None);
    assert!(marks.routed.is_empty());
}

/// A route crosses as the page and the route, and nothing else does (ADR 2017).
#[test]
fn a_route_crosses_as_its_page_and_its_route_and_nothing_else_does() {
    for route in [crate::Route::Marks, crate::Route::Window] {
        assert_eq!(
            decode_route(&encode_route(7, route)).expect("a route decodes"),
            (7, route)
        );
    }
    let mut unknown = encode_route(7, crate::Route::Window);
    if let Some(last) = unknown.last_mut() {
        *last = 2;
    }
    assert!(matches!(
        decode_route(&unknown),
        Err(ProtocolError::Unrecognised { value: 2, .. })
    ));
    let mut long = encode_route(7, crate::Route::Marks);
    long.push(0);
    assert!(matches!(
        decode_route(&long),
        Err(ProtocolError::Trailing { .. })
    ));
    let header = confined_transport::frame::header(FRAME_ROUTE, 9);
    assert_eq!(parse_frame_header(header), Some((FRAME_ROUTE, 9)));
}

/// A window is refused where no honest worker could have sent it: to a host that places none,
/// or not a part of its page (ADRs 1986, 2016).
#[test]
fn a_window_no_worker_could_have_drawn_is_refused() {
    let frame = |width: u32, height: u32, corner: (u32, u32), size: (u32, u32)| {
        let mut writer = Writer::new();
        writer.u8(answer_kind::FRAME).usize(1);
        writer.usize(0).u8(PAYLOAD_WINDOW).u32(width).u32(height);
        display_list::write_transform(&mut writer, pdf_render::Transform::IDENTITY);
        let pixels = vec![0; (size.0 * size.1 * 4) as usize];
        writer
            .u32(corner.0)
            .u32(corner.1)
            .u32(size.0)
            .u32(size.1)
            .u8(0)
            .bytes(&pixels);
        writer.point((0.0, 0.0)).bool(false);
        writer.finish()
    };
    // A host that never said it places a window refuses one of a page under the bound, and
    // reads it once it has (ADR 2016).
    let mut told = HeldLists::default();
    let refused = decode_answer_reusing(&frame(100, 100, (0, 0), (4, 4)), &mut told)
        .expect_err("a window to a host that places none");
    assert!(
        matches!(refused, ProtocolError::Unbuildable { what, .. } if what == "a window of a page"),
        "a window to a host that places none was refused as {refused}"
    );
    told.told(&Command::Oversize(viewer_core::Oversize::Listed));
    let past = 1 << 20;
    assert!(decode_answer_reusing(&frame(past, past, (5, 5), (4, 4)), &mut told).is_ok());
    assert!(decode_answer_reusing(&frame(100, 100, (0, 0), (4, 4)), &mut told).is_ok());
    for (width, height, corner, size, why) in [
        (
            100,
            100,
            (98, 0),
            (4, 4),
            "a window across a small page's edge",
        ),
        (
            past,
            past,
            (past - 2, 0),
            (4, 4),
            "a window across the page's edge",
        ),
        (past, past, (0, 0), (0, 4), "a window of no pixels"),
    ] {
        let refused =
            decode_answer_reusing(&frame(width, height, corner, size), &mut told).expect_err(why);
        assert!(
            matches!(refused, ProtocolError::Unbuildable { what, .. } if what == "a window of a page"),
            "{why} was refused as {refused}"
        );
    }
}

/// **A target is the one length on this boundary with no bytes behind it.**
///
/// ADR 0597's rule in a new place: a raster's dimensions are checked against samples the sender
/// had to write, so a claim costs the sender what it costs the reader — and a render target is
/// eight bytes that become however many pixels the *host* asks its allocator for. The last of
/// these four frames names a terabyte of pixels in a message the assertion below holds to a
/// couple of hundred bytes.
#[test]
fn a_target_no_render_request_could_have_asked_for_is_refused() {
    for (width, height, why) in [
        (0, 200, "a dimension of zero"),
        (100, 0, "a dimension of zero"),
        (
            pdf_render::MAX_EXTENT + 1,
            1,
            "a dimension past what an f32 resolves",
        ),
        (1 << 20, 1 << 20, "more pixels than a render request"),
    ] {
        let frame = a_list_frame(width, height);
        assert!(
            frame.len() < 256,
            "the point of this test is that the message is small: {} bytes",
            frame.len()
        );
        let refused = decode_answer(&frame).expect_err("a target this size is refused");
        // A host that draws a page past the bound from its marks admits the last, whose
        // pixels it never allocates, and still refuses the three no target can be (ADR 1944).
        let mut listing = HeldLists::default();
        listing.told(&Command::Oversize(viewer_core::Oversize::Listed));
        assert_eq!(
            decode_answer_reusing(&frame, &mut listing).is_ok(),
            why == "more pixels than a render request",
            "{width}x{height} ({why}) to a host that draws it from its marks"
        );
        assert!(
            matches!(refused, ProtocolError::Unbuildable { what, .. } if what == "a render target"),
            "{width}x{height} ({why}) was refused as {refused}"
        );
    }
}

/// A payload byte this build does not define is a refusal, not a raster read as a list.
#[test]
fn a_frame_payload_this_build_does_not_define_is_refused() {
    let mut writer = Writer::new();
    writer.u8(answer_kind::FRAME).usize(1);
    writer.usize(0).u8(200);
    let refused = decode_answer(&writer.finish()).expect_err("200 is not a payload");
    assert_eq!(
        refused,
        ProtocolError::Unrecognised {
            what: "a frame payload",
            value: 200,
        }
    );
}
