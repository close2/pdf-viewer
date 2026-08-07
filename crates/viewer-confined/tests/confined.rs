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

use pdf_render::Rasterizer as _;
use render_cpu::CpuRasterizer;
use viewer_confined::{Confined, ConfinedError, Reply};
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
    // comparison of the confinement rather than of the strip planner. It is not a formality: on
    // this very document, one strip and any number above one differ by one pixel, 127 against 111
    // at (117, 636), which is a departure `render-cpu` has on its own and which ADR 0218 records
    // as a finding of the round that first drew a page in one strip on purpose.
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
    let Answer::Frame(frame) = viewer.query(Query::Frame) else {
        panic!("a tier-1 viewer holds the frame it was handed");
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
    let Reply::Frame {
        page,
        raster,
        origin,
    } = confined.query(Query::Frame).expect("a frame crosses")
    else {
        panic!("the confined viewer holds the frame it drew");
    };

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

    let Reply::Frame { page, raster, .. } = confined.query(Query::Frame).expect("a frame crosses")
    else {
        panic!("the confined viewer holds the frame it drew");
    };
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

/// A question whose answer this transport does not carry is refused **by name**.
///
/// The property that keeps the boundary honest: eleven of `viewer-core`'s questions answer with
/// document-model types nothing here encodes yet, and a host asking for one is told which. A
/// boundary that answered `Reply::None` instead would be indistinguishable from a document with
/// no outline.
#[test]
fn a_question_that_does_not_cross_is_refused_by_name() {
    let (mut confined, _events) = opened();
    let error = confined
        .query(Query::Outline)
        .expect_err("an outline does not cross yet");
    let ConfinedError::Uncarried(uncarried) = error else {
        panic!("a question that does not cross is refused as one");
    };
    assert_eq!(uncarried.message, "Query::Outline");

    // And the worker is still there afterwards, which is what makes a refusal a response rather
    // than a transport failure.
    assert!(matches!(
        confined.query(Query::PageCount),
        Ok(Reply::Count(5))
    ));
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
    let Reply::Frame { raster, .. } = confined.query(Query::Frame).expect("a frame crosses") else {
        panic!("the confined viewer holds the frame it drew");
    };
    let ink = raster.data.chunks_exact(4).filter(|p| p[0] < 200).count();
    assert!(
        ink > 100,
        "the page came back nearly blank: {ink} dark pixels"
    );
}

/// Names the probe a re-executed test binary should run.
#[cfg(target_os = "linux")]
const PROBE_VARIABLE: &str = "PDF_CONFINED_TEST_PROBE";

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
