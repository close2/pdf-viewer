//! A document interpreted and drawn in a confined process, checked against one drawn here.
//!
//! Two questions, and they are different. **Does the confined path draw the same page?** — which
//! a comparison against `render-cpu` in this process answers to the byte, and which is the only
//! way to know that confining the interpreter did not quietly change what a person sees. **Is it
//! actually confined?** — which no amount of reading the source establishes, because it is the
//! kernel that decides, and which the two probe tests ask by trying the thing the confinement
//! exists to prevent.
//!
//! The probes are `cfg`-gated to Linux rather than skipped, for ADR 0194's reason: a test that
//! silently passed by doing nothing on a platform with no seccomp-BPF would be worse than one
//! that is not there.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pdf_render::Rasterizer as _;
use render_cpu::CpuRasterizer;
use viewer_confined::{Canceller, Confined, ConfinedError, Reply};
use viewer_core::{Answer, Command, DocumentId, Event, PageTarget, Query, Rendered, Viewer, Zoom};

/// A document committed in `doc/`, which every checkout has.
///
/// Not a corpus file: the corpus is an optional submodule, and a test that skipped itself
/// silently would be worse than no test.
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

/// The viewport these tests look at the page through, in device pixels.
const VIEWPORT: (u32, u32) = (500, 700);

/// Opens the committed note in a confined process, at [`VIEWPORT`].
fn opened() -> (Confined, Vec<Event>) {
    let mut confined = Confined::start().expect("a confined viewer starts");
    confined
        .handle(&Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        })
        .expect("a resize crosses");
    let events = confined
        .handle(&Command::Open {
            id: DOCUMENT,
            bytes: specification_bytes(),
            password: None,
            fragment: None,
        })
        .expect("an open crosses");
    (confined, events)
}

/// The same document, opened and drawn in *this* process by the same two crates.
///
/// This is the comparison's other side and it is deliberately the whole loop rather than a
/// remembered image: a fixture would only say that the confined path matched a picture, and what
/// is worth knowing is that it matches the unconfined path *today*.
fn drawn_here(commands: &[Command]) -> Viewer {
    let mut viewer = Viewer::new(VIEWPORT.0, VIEWPORT.1, 1.0);
    // **One strip, because the confined side draws in one** — and pinning it is what makes this a
    // comparison of the confinement rather than of the strip planner. It is not a formality: with
    // it removed, page 2 of this document at 200% differs in **15 pixels**, each by one of
    // `tiny-skia`'s sixteen supersamples. ADR 0218 found that departure (and blamed the wrong
    // arithmetic for it); ADR 0219 fixed this crate's half — a strip's row offset is composed
    // last now — and measured the half that is the dependency's and cannot be closed.
    let mut rasterizer = CpuRasterizer::new().with_strips(1);
    let mut pending: Vec<Command> = Vec::new();
    for command in commands {
        pending.push(clone_command(command));
        while let Some(command) = pending.pop() {
            for event in viewer.handle(command) {
                if let Event::NeedsRender(request) = event {
                    let raster = rasterizer
                        .rasterize(&request.list, request.target)
                        .expect("the CPU backend draws this page");
                    pending.push(Command::RenderReady {
                        token: request.token,
                        rendered: Rendered::Raster(raster),
                    });
                }
            }
        }
    }
    viewer
}

/// `Command` is not `Clone` — it carries a whole document — so the few this test replays are
/// rebuilt by hand rather than the enum being given a derive it does not otherwise need.
fn clone_command(command: &Command) -> Command {
    match command {
        Command::Resize {
            width,
            height,
            scale,
        } => Command::Resize {
            width: *width,
            height: *height,
            scale: *scale,
        },
        Command::Open { id, .. } => Command::Open {
            id: *id,
            bytes: specification_bytes(),
            password: None,
            fragment: None,
        },
        Command::GoTo(target) => Command::GoTo(*target),
        Command::Zoom { zoom, at } => Command::Zoom {
            zoom: *zoom,
            at: *at,
        },
        other => panic!("this helper replays only what these tests send, not {other:?}"),
    }
}

/// How many bytes two rasters differ in.
///
/// A count rather than `assert_eq!` on the two vectors: a failure there prints eight megabytes of
/// pixels, and the number of them that moved is what a person reading the failure needs.
fn differing_bytes(ours: &[u8], theirs: &[u8]) -> usize {
    if ours.len() != theirs.len() {
        return ours.len().abs_diff(theirs.len());
    }
    ours.iter().zip(theirs).filter(|(a, b)| a != b).count()
}

/// The pixels the viewer in this process is holding.
fn frame_here(viewer: &Viewer) -> (usize, pdf_render::Raster, (f32, f32)) {
    let Answer::Frame(frames) = viewer.query(Query::Frame) else {
        panic!("a tier-1 viewer holds the frame it was handed");
    };
    let [frame] = frames.as_slice() else {
        panic!(
            "a single-page arrangement holds one frame: {} held",
            frames.len()
        );
    };
    (frame.page, frame.raster.clone(), frame.origin)
}

#[test]
fn a_document_opens_in_the_confined_process_and_says_how_many_pages_it_has() {
    let (_confined, events) = opened();
    let pages = events.iter().find_map(|event| match event {
        Event::Opened { document, pages } if *document == DOCUMENT => Some(*pages),
        _ => None,
    });
    assert_eq!(pages, Some(5), "{events:?}");
}

/// The whole point, in one assertion: **the confined process draws the same page**.
///
/// Byte-identical rather than similar. Both sides run `render-cpu` over a display list built by
/// the same interpreter, so anything but equality would mean the confinement changed what the
/// page says — a font the confined side could not reach, an image it could not decode, a
/// difference in how many threads drew it. Each of those is a defect, and none of them is
/// visible to a comparison that allows a tolerance.
#[test]
fn the_confined_process_draws_the_page_this_one_would_have() {
    let (mut confined, _events) = opened();
    let Reply::Frame(frames) = confined.query(Query::Frame).expect("a frame crosses") else {
        panic!("the confined viewer holds the frame it drew");
    };
    let [
        viewer_confined::Framed {
            page,
            raster,
            origin,
        },
    ] = frames.as_slice()
    else {
        panic!(
            "a single-page arrangement crosses as one frame: {}",
            frames.len()
        );
    };
    let (page, origin) = (*page, *origin);

    let here = drawn_here(&[
        Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        },
        Command::Open {
            id: DOCUMENT,
            bytes: Vec::new(),
            password: None,
            fragment: None,
        },
    ]);
    let (page_here, raster_here, origin_here) = frame_here(&here);

    assert_eq!(page, page_here);
    assert_eq!(origin, origin_here);
    assert_eq!(
        (raster.width, raster.height),
        (raster_here.width, raster_here.height)
    );
    assert_eq!(raster.format, raster_here.format);
    assert_eq!(
        differing_bytes(&raster.data, &raster_here.data),
        0,
        "the confined process drew a different page"
    );

    // A blank raster would satisfy every equality above if both sides were blank, so this is the
    // assertion that says a page was actually drawn.
    let ink = raster.data.chunks_exact(4).filter(|p| p[0] < 200).count();
    assert!(
        ink > 1000,
        "the page came back nearly blank: {ink} dark pixels"
    );
}

/// A page turn and a magnification, both drawn behind the filter.
///
/// Two commands rather than one because they exercise the two halves of the render scheduler
/// that the confined process now owns: `GoTo` re-interprets and `Zoom` re-rasterises the list it
/// already has.
#[test]
fn turning_a_page_and_magnifying_it_both_draw_behind_the_filter() {
    let (mut confined, _events) = opened();
    let events = confined
        .handle(&Command::GoTo(PageTarget::Next))
        .expect("a page turn crosses");
    let index = events.iter().find_map(|event| match event {
        Event::PageChanged { index, .. } => Some(*index),
        _ => None,
    });
    assert_eq!(index, Some(1), "{events:?}");

    confined
        .handle(&Command::Zoom {
            zoom: Zoom::Scale(2.0),
            at: None,
        })
        .expect("a zoom crosses");

    let commands = [
        Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        },
        Command::Open {
            id: DOCUMENT,
            bytes: Vec::new(),
            password: None,
            fragment: None,
        },
        Command::GoTo(PageTarget::Next),
        Command::Zoom {
            zoom: Zoom::Scale(2.0),
            at: None,
        },
    ];
    let here = drawn_here(&commands);
    let (page_here, raster_here, _) = frame_here(&here);

    let Reply::Frame(frames) = confined.query(Query::Frame).expect("a frame crosses") else {
        panic!("the confined viewer holds the frame it drew");
    };
    let [frame] = frames.as_slice() else {
        panic!(
            "a single-page arrangement crosses as one frame: {}",
            frames.len()
        );
    };
    let (page, raster) = (frame.page, &frame.raster);
    assert_eq!(page, page_here);
    assert_eq!(
        (raster.width, raster.height),
        (raster_here.width, raster_here.height),
        "a magnified page is a larger raster"
    );
    assert_eq!(
        differing_bytes(&raster.data, &raster_here.data),
        0,
        "the confined process drew a different page 2 at 200%"
    );
}

/// The confinement itself, as the worker reports it.
///
/// A property of the machine as much as of the code, and deliberately so: this kernel supports
/// every Landlock right the ruleset asks for and has seccomp-BPF, so anything less than full
/// enforcement here means the confinement stopped being applied — which nothing else would
/// notice, because an unconfined viewer draws exactly the same pages.
#[test]
#[cfg(target_os = "linux")]
fn the_confined_viewer_reports_a_confinement_it_actually_has() {
    let confined = Confined::start().expect("a confined viewer starts");
    let confinement = confined.confinement();
    assert!(
        confinement.is_enforced(),
        "the system-call filter is not in force: {confinement:?}"
    );
    assert_eq!(
        confinement.landlock,
        pdf_sandbox::lockdown::LandlockLevel::Enforced,
        "Landlock is not fully enforced on a kernel that supports it"
    );
    assert_eq!(
        confinement.address_space_limit,
        4 << 30,
        "an interpreter's ceiling is four times a decoder's"
    );
    assert_eq!(confinement.shortfall(), None);
}

/// A panel's worth of a document, asked across the boundary and asked here, on real content.
///
/// **The property the three-hundred-and-eighty-sixth session added, checked where it matters.**
/// `protocol`'s unit tests populate every field of every type and round trip it in memory; this
/// one takes what a *document* actually says, sends it through a pipe and a process that cannot
/// open a file, and compares it with what the same document says in this process. Three files
/// rather than one, because no single committed document has an outline, a layer order, an
/// attachment list, a thumbnail, a metadata packet and a structure tree at once — and an
/// encoding is only exercised by an answer that is not empty.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "eleven answers, each compared against the same answer read in this process; the \
              length is the vocabulary's, and splitting it would hide which of the eleven a \
              failure is in"
)]
fn every_panel_answer_crosses_a_real_document_unchanged() {
    for name in [
        // An outline of 37 visible items, a thumbnail, and an XMP packet.
        "PDF20_AN002-AF.pdf",
        // Two embedded files, §7.11.4's own subject.
        "PDF-Declarations.pdf",
        // An optional content group, and a tagged page: 988 outline items and §14.7's structure.
        "ISO_32000-2_sponsored_EC3.pdf",
    ] {
        let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc")
            .join(name);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()));

        let mut confined = Confined::start().expect("a confined viewer starts");
        confined
            .handle(&Command::Resize {
                width: VIEWPORT.0,
                height: VIEWPORT.1,
                scale: 1.0,
            })
            .expect("a resize crosses");
        confined
            .handle(&Command::Open {
                id: DOCUMENT,
                bytes: bytes.clone(),
                password: None,
                fragment: None,
            })
            .expect("an open crosses");

        let mut here = Viewer::new(VIEWPORT.0, VIEWPORT.1, 1.0);
        for _ in here.handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        }) {}

        let ask = |confined: &mut Confined, query| {
            confined
                .query(query)
                .unwrap_or_else(|error| panic!("{name}: {query:?} crosses: {error}"))
        };

        // §12.3.3.
        match (
            ask(&mut confined, Query::Outline),
            here.query(Query::Outline),
        ) {
            (Reply::Outline(crossed), Answer::Outline(ours)) => {
                assert_eq!(crossed, ours, "{name}: an outline changed on the way over");
            }
            (Reply::None, Answer::None) => {}
            (crossed, ours) => panic!("{name}: an outline came back as {crossed:?} for {ours:?}"),
        }

        // §8.11.4.3.
        match (ask(&mut confined, Query::Layers), here.query(Query::Layers)) {
            (Reply::Layers(crossed), Answer::Layers(ours)) => {
                assert_eq!(crossed, ours, "{name}: a layer order changed");
            }
            (Reply::None, Answer::None) => {}
            (crossed, ours) => panic!("{name}: layers came back as {crossed:?} for {ours:?}"),
        }

        // §7.11.4, whose stream deliberately stays where it is.
        match (
            ask(&mut confined, Query::Attachments),
            here.query(Query::Attachments),
        ) {
            (Reply::Attachments(crossed), Answer::Attachments(ours)) => {
                assert_eq!(
                    crossed.len(),
                    ours.len(),
                    "{name}: an attachment went missing"
                );
                for (crossed, ours) in crossed.iter().zip(&ours) {
                    assert_eq!(crossed.name, ours.name);
                    assert_eq!(crossed.file_name, ours.file_name);
                    assert_eq!(crossed.description, ours.description);
                    assert_eq!(crossed.media_type, ours.media_type);
                    assert_eq!(crossed.size, ours.size);
                    assert_eq!(crossed.created, ours.created);
                    assert_eq!(crossed.modified, ours.modified);
                    assert_eq!(crossed.checksum, ours.checksum);
                    assert_eq!(crossed.relationship, ours.relationship);
                }
            }
            (Reply::None, Answer::None) => {}
            (crossed, ours) => panic!("{name}: attachments came back as {crossed:?} for {ours:?}"),
        }

        // §12.4.3.
        match (
            ask(&mut confined, Query::Articles),
            here.query(Query::Articles),
        ) {
            (Reply::Articles(crossed), Answer::Articles(ours)) => {
                assert_eq!(crossed, ours, "{name}: an article thread changed");
            }
            (Reply::None, Answer::None) => {}
            (crossed, ours) => panic!("{name}: articles came back as {crossed:?} for {ours:?}"),
        }

        // §12.3.4.
        match (
            ask(&mut confined, Query::Thumbnail(0)),
            here.query(Query::Thumbnail(0)),
        ) {
            (Reply::Thumbnail(crossed), Answer::Thumbnail(ours)) => {
                assert_eq!(crossed, ours, "{name}: a thumbnail changed");
            }
            (Reply::None, Answer::None) => {}
            (crossed, ours) => panic!("{name}: a thumbnail came back as {crossed:?} for {ours:?}"),
        }

        // §14.3.3 and §14.3.2.
        match (
            ask(&mut confined, Query::Properties),
            here.query(Query::Properties),
        ) {
            (
                Reply::Properties {
                    information: crossed,
                    metadata: crossed_metadata,
                },
                Answer::Properties {
                    information: ours,
                    metadata: our_metadata,
                },
            ) => {
                assert_eq!(*crossed, ours, "{name}: the information dictionary changed");
                assert_eq!(
                    crossed_metadata, our_metadata,
                    "{name}: the metadata changed"
                );
            }
            (Reply::None, Answer::None) => {}
            (crossed, ours) => panic!("{name}: properties came back as {crossed:?} for {ours:?}"),
        }

        // Table 29 and Table 147.
        match (
            ask(&mut confined, Query::Opening),
            here.query(Query::Opening),
        ) {
            (Reply::Opening(crossed), Answer::Opening(ours)) => assert_eq!(crossed, ours),
            (crossed, ours) => {
                panic!("{name}: an opening pair came back as {crossed:?} for {ours:?}")
            }
        }
        match (
            ask(&mut confined, Query::Preferences),
            here.query(Query::Preferences),
        ) {
            (Reply::Preferences(crossed), Answer::Preferences(ours)) => assert_eq!(*crossed, ours),
            (crossed, ours) => panic!("{name}: preferences came back as {crossed:?} for {ours:?}"),
        }

        // §12.5.6.14 and §14.7.
        match (ask(&mut confined, Query::Popups), here.query(Query::Popups)) {
            (Reply::Popups(crossed), Answer::Popups(ours)) => assert_eq!(crossed, ours),
            (Reply::None, Answer::None) => {}
            (crossed, ours) => panic!("{name}: popups came back as {crossed:?} for {ours:?}"),
        }
        match (
            ask(&mut confined, Query::AccessibilityTree),
            here.query(Query::AccessibilityTree),
        ) {
            (Reply::Accessibility(crossed), Answer::Accessibility(ours)) => {
                // Page by page, because the two types are the same answer with one side's
                // borrows owned: `Structured` is `PageStructure` across a pipe.
                assert_eq!(crossed.len(), ours.len(), "{name}: a page went missing");
                for (crossed, ours) in crossed.iter().zip(&ours) {
                    assert_eq!(crossed.page, ours.page, "{name}: a page changed");
                    assert_eq!(
                        crossed.nodes, ours.nodes,
                        "{name}: a structure tree changed"
                    );
                }
            }
            (Reply::None, Answer::None) => {}
            (crossed, ours) => {
                panic!("{name}: a structure tree came back as {crossed:?} for {ours:?}")
            }
        }

        // §12.3.5: no committed document states one, and `Answer::None` is the answer rather than
        // an absence — so this checks that the *pair* agrees rather than that a collection crossed.
        match (
            ask(&mut confined, Query::Collection),
            here.query(Query::Collection),
        ) {
            (
                Reply::Collection {
                    collection: crossed,
                    initial: crossed_initial,
                },
                Answer::Collection {
                    collection: ours,
                    initial: ours_initial,
                },
            ) => {
                assert_eq!(*crossed, ours);
                assert_eq!(crossed_initial, ours_initial);
            }
            (Reply::None, Answer::None) => {}
            (crossed, ours) => panic!("{name}: a collection came back as {crossed:?} for {ours:?}"),
        }

        // And the worker is still there afterwards.
        assert!(matches!(
            confined.query(Query::PageCount),
            Ok(Reply::Count(_))
        ));
    }
}

/// An embedded file listed on one side of this boundary and extracted through the other.
///
/// **The gap `doc/todo/01`'s fifth sweep found in this round's own work.** §7.11.4's stream does
/// not cross with the list — a panel drawing five rows would otherwise pull five payloads — so a
/// host holds Table 45's `/CheckSum` from one message and the bytes from another, and the clause's
/// own rule about the two of them together had no way to be asked. It has one now, and this is it
/// end to end: list, extract, compare.
#[test]
fn an_attachment_is_listed_here_and_checked_against_what_the_worker_extracted() {
    let path: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF-Declarations.pdf");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()));

    let mut confined = Confined::start().expect("a confined viewer starts");
    confined
        .handle(&Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .expect("an open crosses");

    let Reply::Attachments(attachments) = confined
        .query(Query::Attachments)
        .expect("an attachment list crosses")
    else {
        panic!("this document states two embedded files");
    };
    assert_eq!(attachments.len(), 2, "§7.11.4's two files");

    for attachment in &attachments {
        let events = confined
            .handle(&Command::Extract {
                name: attachment.name.clone(),
            })
            .expect("an extraction crosses");
        // `Event::Extracted` names the file by Table 43's `/UF` where it states one — "that is
        // what a person would call it" — and by the tree's key otherwise, so the two names are
        // not the same string and the event is found by being the only one.
        let extracted = events
            .iter()
            .find_map(|event| match event {
                Event::Extracted { name, bytes, .. } => {
                    assert!(
                        *name == attachment.name || Some(name) == attachment.file_name.as_ref(),
                        "an extraction named {name:?}, which is neither of this file's names"
                    );
                    Some(bytes)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{} comes back: {events:?}", attachment.name));

        // Table 45's `/Size` is "the size of the uncompressed embedded file, in bytes", which is
        // the document's claim; the bytes are the measurement, and here they agree.
        assert_eq!(
            attachment.size,
            Some(i64::try_from(extracted.len()).expect("a file this machine can hold")),
            "{}: /Size against what was decoded",
            attachment.name
        );
        // `None` for a file that states no checksum, which the clause permits and both of these
        // do — so what is under test is that the question can be *asked* from this side at all.
        assert_ne!(
            attachment.checksum_matches(extracted),
            Some(false),
            "{}: the bytes do not match the checksum the file states",
            attachment.name
        );
    }
}

/// §12.7's form, read across the boundary and filled in through it.
///
/// **The twelfth answer, and the one that decides whether a confined host can be a form at all.**
/// The eleven panel answers crossed in the three-hundred-and-eighty-sixth session; a form field
/// did not, so a host on this side could place a native tree, a native popover and a native
/// attachment list and then had to take its text fields, its check boxes and its combo boxes as
/// pixels off the raster. ADR 0235.
///
/// Two properties, and they are separate. **The description crosses unchanged** — every control,
/// every flag, every option and every quadrilateral, compared against the same document read in
/// this process, which is what `every_panel_answer_crosses_a_real_document_unchanged` does for the
/// other eleven. **And an edit built from it works**: the on-state name the confined process
/// answered with is sent back through the same pipe, and the page it draws afterwards is a
/// different page.
#[test]
fn a_form_crosses_the_boundary_and_can_be_filled_in_through_it() {
    let Some(bytes) = corpus_bytes("issue17492.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };

    let mut confined = Confined::start().expect("a confined viewer starts");
    confined
        .handle(&Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        })
        .expect("a resize crosses");
    confined
        .handle(&Command::Open {
            id: DOCUMENT,
            bytes: bytes.clone(),
            password: None,
            fragment: None,
        })
        .expect("an open crosses");

    let mut here = Viewer::new(VIEWPORT.0, VIEWPORT.1, 1.0);
    for _ in here.handle(Command::Open {
        id: DOCUMENT,
        bytes,
        password: None,
        fragment: None,
    }) {}

    let Reply::Fields(crossed) = confined.query(Query::Fields).expect("a form crosses") else {
        panic!("this document's first page states twelve fields");
    };
    let Answer::Fields(ours) = here.query(Query::Fields) else {
        panic!("and it states them here too");
    };
    assert_eq!(crossed.len(), 12, "§12.7.5's controls, all but a signature");
    assert_eq!(
        crossed, ours,
        "a form changed on the way over: every control, flag, option and quadrilateral"
    );

    // And the edit a host builds out of it. §12.7.5.2.3 keys the appearance dictionary by state
    // and the name is the file's own invention, so this string exists on this side of the pipe
    // only because the answer above carried it.
    let field = crossed
        .iter()
        .find(|field| field.name.qualified == "typeScript")
        .expect("a check box");
    let state = field.widgets[0]
        .on_state
        .clone()
        .expect("§12.7.5.2.3's on state");
    confined
        .handle(&Command::Edit(viewer_core::Edit::SetField {
            field: field.name.qualified.clone(),
            value: pdf_model::view::Entered::Text(state),
        }))
        .expect("an edit crosses");

    let Reply::Fields(after) = confined
        .query(Query::Fields)
        .expect("the form crosses again")
    else {
        panic!("the page still states a form");
    };
    let checked = after
        .iter()
        .find(|field| field.name.qualified == "typeScript")
        .expect("the same check box");
    assert_eq!(
        checked.control,
        pdf_model::form::Control::CheckBox { on: true },
        "the box the host checked is checked"
    );
    assert!(checked.widgets[0].on);
    assert!(
        matches!(confined.query(Query::Dirty), Ok(Reply::Dirty(true))),
        "and the document knows it was edited"
    );
}

/// §12.7.5.4: a confined host selects several items of a list box, and the file says so.
///
/// The transport's half of ADR 0248. `Edit::SetField` carries a *set* of Table 234 `/Opt` indices
/// now, so the wire format carries all three of `Entered`'s shapes — and a confined host that could
/// select one item where an unconfined one selects three would be ADR 0178's failure with a
/// clause's name on it.
#[test]
fn a_confined_host_selects_several_items_of_a_list_box() {
    let Some(bytes) = corpus_bytes("issue17492.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut confined = Confined::start().expect("the worker starts");
    confined
        .handle(&Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .expect("an open crosses");

    let Reply::Fields(crossed) = confined.query(Query::Fields).expect("a form crosses") else {
        panic!("this document's first page states a form");
    };
    let field = crossed
        .iter()
        .find(|field| field.name.qualified == "databases")
        .expect("Table 233 bit 22's list box");
    let pdf_model::form::Control::Choice(choice) = &field.control else {
        panic!("{:?}", field.control)
    };
    assert!(choice.multi_select, "Table 233 bit 22 crossed");

    confined
        .handle(&Command::Edit(viewer_core::Edit::SetField {
            field: field.name.qualified.clone(),
            value: pdf_model::view::Entered::Chosen(vec![0, 2]),
        }))
        .expect("a selection crosses");

    let Reply::Fields(after) = confined
        .query(Query::Fields)
        .expect("the form crosses again")
    else {
        panic!("the page still states a form");
    };
    let chosen = after
        .iter()
        .find(|field| field.name.qualified == "databases")
        .expect("the same list box");
    let pdf_model::form::Control::Choice(choice) = &chosen.control else {
        panic!("{:?}", chosen.control)
    };
    assert_eq!(choice.selected, vec![0, 2], "both, ascending");
}

/// §12.5.6.6: a confined host draws a text box, types in it, and saves the file.
///
/// **The third consumer, and the one that would silently have less.** A vocabulary that reached
/// `viewer-ui` and the headless harness and not this transport would be a confined host that can
/// read a document and not annotate one, which is ADR 0178's failure exactly. Every message this
/// round added crosses here: the drag, the question that names what it made, the text, and the
/// bytes coming back — and the *pixels* are the proof, because the confined process is what drew
/// them and this process never saw a display list.
#[test]
fn a_free_text_annotation_is_drawn_typed_and_saved_behind_the_filter() {
    let (mut confined, _) = opened();
    let Ok(Reply::Geometry(geometry)) = confined.query(Query::PageGeometry(0)) else {
        panic!("the page in the confined process has a geometry");
    };
    let corner = |across: f32, down: f32| {
        (
            geometry.origin.0 + geometry.page.width * across * geometry.scale,
            geometry.origin.1 + geometry.page.height * down * geometry.scale,
        )
    };
    let (from, to) = (corner(0.2, 0.45), corner(0.8, 0.55));
    confined
        .handle(&Command::Edit(viewer_core::Edit::FreeText {
            from,
            to,
            colour: [1.0, 0.0, 0.0],
        }))
        .expect("a drag crosses");

    let middle = (f32::midpoint(from.0, to.0), f32::midpoint(from.1, to.1));
    let Ok(Reply::FreeText { annotation, text }) = confined.query(Query::FreeTextAt { at: middle })
    else {
        panic!("the annotation the drag made is under the point");
    };
    assert!(text.is_empty(), "nothing has been typed into it yet");

    confined
        .handle(&Command::Edit(viewer_core::Edit::SetFreeText {
            annotation,
            text: "Reviewed".to_owned(),
        }))
        .expect("the text crosses");
    let Ok(Reply::FreeText { text, .. }) = confined.query(Query::FreeTextAt { at: middle }) else {
        panic!("and it is still there");
    };
    assert_eq!(text, "Reviewed", "read back through the pipe");

    // The pixels, which is what this boundary exists to carry: the confined process interpreted
    // and rasterised the page, and the red is a `/DA` this side never laid out.
    let Ok(Reply::Frame(frames)) = confined.query(Query::Frame) else {
        panic!("the confined process has drawn a frame");
    };
    let [frame] = frames.as_slice() else {
        panic!(
            "a single-page arrangement crosses as one frame: {}",
            frames.len()
        );
    };
    let raster = &frame.raster;
    let red = raster
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100)
        .count();
    assert!(red > 40, "the text is on the confined frame: {red} pixels");

    // And §7.5.6's update, produced by a process with no filesystem to write it to — which is
    // exactly why the bytes come back rather than being written there.
    let events = confined.handle(&Command::Save).expect("a save crosses");
    let saved = events
        .iter()
        .find_map(|event| match event {
            Event::Saved { bytes, .. } => Some(bytes.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{events:?}"));
    assert!(
        saved.starts_with(&specification_bytes()),
        "§7.5.6 appends, leaving the producer's bytes underneath"
    );
}

/// A document whose images need a codec `pdf-sandbox` would ordinarily confine separately.
///
/// The confined viewer cannot spawn anything — the filter has no `execve` — so it decodes its
/// own images in process, which is ADR 0218's one real trade. This is the test that says the
/// arrangement works rather than dying at the first JBIG2 image.
#[test]
fn a_document_with_a_sandboxed_codec_draws_inside_the_confinement() {
    let Some(bytes) = corpus_bytes("issue12963.pdf") else {
        // The corpus is an optional submodule. Saying so is the rule; skipping in silence is not.
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut confined = Confined::start().expect("a confined viewer starts");
    confined
        .handle(&Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        })
        .expect("a resize crosses");
    let events = confined
        .handle(&Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .expect("an open crosses");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "{events:?}"
    );
    let Reply::Frame(frames) = confined.query(Query::Frame).expect("a frame crosses") else {
        panic!("the confined viewer holds the frame it drew");
    };
    let [frame] = frames.as_slice() else {
        panic!(
            "a single-page arrangement crosses as one frame: {}",
            frames.len()
        );
    };
    let raster = &frame.raster;
    let ink = raster.data.chunks_exact(4).filter(|p| p[0] < 200).count();
    assert!(
        ink > 100,
        "the page came back nearly blank: {ink} dark pixels"
    );
}

/// The hostile document, shared with `examples/confined_cancel`.
#[path = "support/amplification.rs"]
mod amplification;

/// How long the hostile document is given to *not* finish before it is cancelled.
///
/// The assertion is one-sided on purpose: this says the work had not finished, which is what
/// makes the cancel that follows a cancel of something. It is two seconds against a page that
/// takes tens, so a machine under load moves it in the safe direction.
const UNFINISHED: Duration = Duration::from_secs(2);

/// How long the host waits for its thread back after cancelling.
///
/// Generous — the cancel is a signal and the read unblocks with the pipe, which is microseconds —
/// because what this test is about is the difference between *bounded* and *unbounded*, and a
/// tight bound here would make it a test of the scheduler.
const AFTER_CANCEL: Duration = Duration::from_secs(30);

/// What the thread that opened the hostile document has to say when it comes back.
///
/// Strings and flags rather than the values themselves, because this crosses a channel and what
/// the assertions need is what happened rather than the events that did not arrive.
#[derive(Debug)]
struct Outcome {
    /// Whether the open ended in [`ConfinedError::Cancelled`].
    open_cancelled: bool,
    /// What it ended in, in its own words, for a failure to print.
    open: String,
    /// What [`Confined::is_cancelled`] said afterwards.
    flag: bool,
    /// Whether a question asked after the cancel was refused the same way, without a pipe error.
    later_cancelled: bool,
}

/// **The cancel, end to end: a document that will not finish, and a host that takes control
/// back.**
///
/// Three things are asserted and they are separate claims. The work **had not finished** after
/// [`UNFINISHED`], which is what makes this a cancel of something rather than a race with a fast
/// page. The host thread **came back**, with [`ConfinedError::Cancelled`] rather than a worker
/// that died of something — the distinction a host prints. And the viewer is **finished**: the
/// flag says so and a later question is refused without going near the pipe, because the worker
/// and the document it held are gone. ADR 0241 argues why that last one is the price of a cancel
/// a hostile document cannot decline.
#[test]
fn a_document_that_will_not_finish_is_cancelled_and_the_host_gets_its_thread_back() {
    let bytes = amplification::document(amplification::LEVELS, amplification::BRANCH);
    assert!(
        bytes.len() < 2048,
        "the point is the amplification: {} bytes",
        bytes.len()
    );

    let canceller = Canceller::new();
    let mut confined = Confined::start_with(&canceller).expect("a confined viewer starts");
    confined
        .handle(&Command::Resize {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            scale: 1.0,
        })
        .expect("a resize crosses");

    let (sender, receiver) = std::sync::mpsc::channel();
    let opening = std::thread::spawn(move || {
        let opened = confined.handle(&Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        });
        let outcome = Outcome {
            open_cancelled: matches!(opened, Err(ConfinedError::Cancelled)),
            open: match opened {
                Ok(events) => format!("{events:?}"),
                Err(error) => error.to_string(),
            },
            flag: confined.is_cancelled(),
            later_cancelled: matches!(
                confined.query(Query::PageCount),
                Err(ConfinedError::Cancelled)
            ),
        };
        // The receiver outlives this thread — it is on the stack of the test that spawned it —
        // so a send that fails means the test has already failed for another reason.
        let _ = sender.send(outcome);
    });

    assert!(
        receiver.recv_timeout(UNFINISHED).is_err(),
        "the hostile document finished in {UNFINISHED:?}, so this test cancels nothing"
    );

    let at = Instant::now();
    canceller.cancel();
    let outcome = receiver
        .recv_timeout(AFTER_CANCEL)
        .expect("the host gets its thread back after a cancel");
    let took = at.elapsed();
    opening.join().expect("the opening thread ends");

    assert!(
        outcome.open_cancelled,
        "the open came back as {:?} rather than a cancel",
        outcome.open
    );
    assert!(outcome.flag, "and the viewer says it was cancelled");
    assert!(
        outcome.later_cancelled,
        "a question asked afterwards is refused the same way"
    );
    assert!(
        took < AFTER_CANCEL,
        "the cancel took {took:?}, which is not taking control back"
    );
}

/// A canceller that fires before there is a worker starts nothing at all.
///
/// The hole [`Confined::canceller`] cannot close by itself: `Confined::start` blocks reading the
/// worker's greeting, and a handle taken from the value it returns does not exist while it is
/// blocked. So a canceller can be made first, and this is what it does when it is used first.
#[test]
fn a_cancel_before_the_worker_starts_starts_no_worker() {
    let canceller = Canceller::new();
    canceller.cancel();
    assert!(canceller.is_cancelled());
    assert!(
        matches!(
            Confined::start_with(&canceller),
            Err(ConfinedError::Cancelled)
        ),
        "a cancelled canceller spawns nothing"
    );
}

/// A cancel between two commands is the same answer as one during a page.
///
/// Not a special case in the caller's code, which is the property worth pinning: a host that
/// cancels while nothing is happening gets [`ConfinedError::Cancelled`] from the next call rather
/// than a broken pipe about a process that is gone.
#[test]
fn a_cancelled_viewer_refuses_the_next_command_without_touching_the_pipe() {
    let (mut confined, _events) = opened();
    let canceller = confined.canceller();
    assert!(!canceller.is_cancelled());
    assert!(matches!(
        confined.query(Query::PageCount),
        Ok(Reply::Count(5))
    ));

    canceller.cancel();
    // Twice, because a cancel that has already happened must not become a different answer.
    canceller.cancel();

    assert!(confined.is_cancelled());
    assert!(matches!(
        confined.handle(&Command::GoTo(PageTarget::Next)),
        Err(ConfinedError::Cancelled)
    ));
    assert!(matches!(
        confined.query(Query::PageCount),
        Err(ConfinedError::Cancelled)
    ));
}

/// Names the probe a re-executed test binary should run.
#[cfg(target_os = "linux")]
const PROBE_VARIABLE: &str = "PDF_CONFINED_TEST_PROBE";

/// The probe that asks whether the allocator's arena question can be answered in advance.
#[cfg(target_os = "linux")]
const WARM: &str = "warm";

/// Exit code from a probe whose forbidden operation was refused.
#[cfg(target_os = "linux")]
const REFUSED: i32 = 17;
/// Exit code from a probe whose forbidden operation *succeeded*, which is the failure.
#[cfg(target_os = "linux")]
const ALLOWED: i32 = 18;
/// Exit code from a probe that drew a page, which is what says the profile permits the work.
#[cfg(target_os = "linux")]
const DREW: i32 = 19;

#[test]
#[cfg(target_os = "linux")]
fn a_confined_interpreter_cannot_open_a_file() {
    let status = run_probe("open");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined interpreter read a file it should not have been able to name"
    );
    assert!(
        refused(status),
        "expected the open to be refused or the process killed, got {status:?}"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn a_confined_interpreter_cannot_reach_the_network() {
    let status = run_probe("socket");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined interpreter opened a socket"
    );
    assert!(
        refused(status),
        "expected the socket to be refused or the process killed, got {status:?}"
    );
}

/// A confined interpreter cannot start another program.
///
/// The one that makes the interpreter profile's extra system calls defensible: it permits a
/// *thread* and this says it does not permit a *program*. Without it, "we allowed `clone`" would
/// be a claim about a list rather than about the kernel.
#[test]
#[cfg(target_os = "linux")]
fn a_confined_interpreter_cannot_start_a_program() {
    let status = run_probe("spawn");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined interpreter started another program"
    );
    assert!(
        refused(status),
        "expected the spawn to be refused or the process killed, got {status:?}"
    );
}

/// And it *can* still interpret and draw a page.
///
/// The other half of the previous three: a filter that refused everything would pass them all and
/// be useless. This one confines the process and then does the work, which is what the profile's
/// four extra system calls are for — the one rasterising thread needs every one of them.
#[test]
#[cfg(target_os = "linux")]
fn a_confined_interpreter_can_still_draw_a_page() {
    let status = run_probe("draw");
    assert_eq!(
        status.code(),
        Some(DREW),
        "a confined interpreter could not draw a page: {status:?}"
    );
}

/// **The measurement `doc/todo/34`'s item 4 asked for, kept runnable.**
///
/// The confined worker rasterises on one thread because `glibc`'s allocator sizes its arena count
/// from `__get_nprocs`, which reads `/sys/devices/system/cpu/online` — an `openat` the filter
/// kills for, on a thread's first allocation (ADR 0218 section 2). One of the two candidate answers is to
/// build the pool *before* seccomp with a `start_handler` that allocates, so that the question is
/// asked while it can still be answered; and `doc/todo/34` says of it that "it rests on the
/// allocator not asking again later, which is a claim about `glibc` internals — write it down as
/// one, or measure it".
///
/// **This measures it**, and it is a *precondition* rather than the arrangement this crate ships:
/// [`viewer_confined::confine`] still builds a one-thread pool after the confinement, because a
/// pool warmed first has the seccomp filter (it is installed with `TSYNC`) and **not** the
/// Landlock domain, which `landlock_restrict_self` gives only to the calling thread and its
/// future children. Closing that needs an entry point `pdf-sandbox` does not have. ADR 0241.
///
/// What the probe does: 24 threads warmed before the filter, the filter, a real page drawn on 24
/// strips, and then twenty rounds of four-mebibyte allocations broadcast to every one of them —
/// because the question is asked on a *later* allocation, not the first. `strace` counts 25
/// `clone3` before the filter, none after, and no `openat` at all after it.
#[test]
#[cfg(target_os = "linux")]
fn an_allocator_warmed_before_the_filter_does_not_ask_the_kernel_again() {
    let status = run_probe(WARM);
    assert_eq!(
        status.code(),
        Some(DREW),
        "a pool warmed before the confinement could not draw behind it: {status:?} — if this is \
         SIGSYS, the allocator asked again and `doc/todo/34`'s first candidate is dead"
    );
}

/// The `warm` probe's body: a thread pool warmed *before* the confinement, then the work.
///
/// Separate from [`confined_probe`]'s match because it has to run before
/// [`viewer_confined::confine`] rather than after it, and the order is the whole subject. Never
/// returns.
#[cfg(target_os = "linux")]
fn warm_then_confine(bytes: Vec<u8>) -> ! {
    // The confined process cannot spawn a decoder, exactly as `worker::confine` arranges.
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);

    // Asked here for the same reason `worker::confine` asks it here: it reads `/proc/self/cgroup`.
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .start_handler(|_| allocate())
        .build_global()
        .expect("a thread pool");
    // `build_global` is not enough on its own to be sure every thread has run its handler, so
    // every thread is made to do something before the filter goes on.
    rayon::broadcast(|_| allocate());

    let confinement = pdf_sandbox::lockdown::apply_for(pdf_sandbox::lockdown::Profile::Interpreter)
        .expect("a probe that cannot confine itself proves nothing");
    assert!(confinement.is_enforced(), "{confinement:?}");

    let mut viewer = Viewer::new(VIEWPORT.0, VIEWPORT.1, 1.0);
    let mut rasterizer =
        CpuRasterizer::new().with_strips(u32::try_from(threads).unwrap_or(u32::MAX));
    let mut drew = false;
    for event in viewer.handle(Command::Open {
        id: DOCUMENT,
        bytes,
        password: None,
        fragment: None,
    }) {
        if let Event::NeedsRender(request) = event
            && rasterizer.rasterize(&request.list, request.target).is_ok()
        {
            drew = true;
        }
    }

    // And then far more allocation than the page needed, on every thread, because `arena_get2`
    // asks `__get_nprocs` only once `narenas > mp_.arena_test` — so a page that happened not to
    // cross that threshold would prove nothing.
    for _ in 0..20 {
        rayon::broadcast(|_| {
            let more: Vec<u8> = vec![3u8; 4 << 20];
            std::hint::black_box(more.len());
        });
    }

    std::process::exit(if drew { DREW } else { REFUSED });
}

/// One allocation large enough that the allocator has to go to the operating system for it.
#[cfg(target_os = "linux")]
fn allocate() {
    let warm: Vec<u8> = vec![7u8; 1 << 20];
    std::hint::black_box(warm.len());
}

/// Whether a probe was stopped rather than served.
///
/// Two outcomes count. `SIGSYS` is the seccomp filter firing, which is what happens when the
/// system call is not on the allow-list at all. A clean `REFUSED` exit is the operation having
/// failed for a reason the confinement caused before the call could be made. Both are the
/// confinement working; only `ALLOWED` is not.
#[cfg(target_os = "linux")]
fn refused(status: std::process::ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal() == Some(libc::SIGSYS) || status.code() == Some(REFUSED)
}

/// Runs one probe in a fresh child and waits for it.
///
/// The child is this same test binary, re-executed with a filter that selects the one test
/// below. Confinement cannot be tested in the process doing the testing: it is irreversible and
/// process-wide, so the first thing it would break is the test harness.
#[cfg(target_os = "linux")]
fn run_probe(probe: &str) -> std::process::ExitStatus {
    std::process::Command::new(std::env::current_exe().expect("a test binary knows where it is"))
        .args(["--exact", "confined_probe", "--test-threads=1"])
        .env(PROBE_VARIABLE, probe)
        .output()
        .expect("the probe runs")
        .status
}

/// The child half of the four confinement tests, and not a test of anything by itself.
///
/// Run normally it does nothing, because the variable is unset. Run by [`run_probe`] it confines
/// itself with the *interpreter* profile and then attempts the thing in question, reporting the
/// answer as an exit code — the only channel it has left.
#[test]
#[cfg(target_os = "linux")]
fn confined_probe() {
    let Ok(probe) = std::env::var(PROBE_VARIABLE) else {
        return;
    };

    // Read before the confinement, because a confined process has no filesystem — which is the
    // whole reason a document reaches the real worker as bytes in a command.
    let bytes = specification_bytes();

    // The one probe that must run *before* `confine()`, because what it is about is the order.
    if probe == WARM {
        warm_then_confine(bytes);
    }

    let (_confinement, strips) =
        viewer_confined::confine().expect("a probe that cannot confine itself proves nothing");

    let permitted = match probe.as_str() {
        // `/proc/self/maps` rather than anything under `/etc`, because it is guaranteed to exist
        // and to be readable by this user: a failure here has to be the confinement.
        "open" => std::fs::File::open("/proc/self/maps").is_ok(),
        "socket" => std::net::UdpSocket::bind("127.0.0.1:0").is_ok(),
        "spawn" => std::process::Command::new("/bin/true").status().is_ok(),
        "draw" => {
            let mut viewer = Viewer::new(VIEWPORT.0, VIEWPORT.1, 1.0);
            let mut rasterizer = CpuRasterizer::new().with_strips(strips);
            let mut drew = false;
            for event in viewer.handle(Command::Open {
                id: DOCUMENT,
                bytes,
                password: None,
                fragment: None,
            }) {
                if let Event::NeedsRender(request) = event
                    && rasterizer.rasterize(&request.list, request.target).is_ok()
                {
                    drew = true;
                }
            }
            std::process::exit(if drew { DREW } else { REFUSED });
        }
        other => panic!("no probe named {other}"),
    };

    std::process::exit(if permitted { ALLOWED } else { REFUSED });
}
