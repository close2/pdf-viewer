//! The viewer: opens a PDF and shows it.
//!
//! ``text
//! cargo run --release -p viewer-ui --bin pdf-viewer -- document.pdf
//! ``
//!
//! `--page N` opens at a page, and so does ISO 32000-2 Annex O's fragment identifier —
//! `document.pdf#page=5`, `#nameddest=Chapter3`, `#zoom=150,0,792` — which this program splits off
//! the argument and `viewer-core` carries out, saying by name what it cannot.
//! Arrows, Page Up and Down or Space turn pages, Home and End jump,
//! `+` and `-` zoom and **Ctrl + the wheel zooms about the pointer**, the wheel alone scrolls
//! whatever is under it, `a` selects the whole page and dragging selects part of it, `o` shows
//! the sidebar — §12.3.3's outline, §8.11's layers and §7.11.4's embedded files — `s` saves what
//! was changed beside the document, `?` shows the third-party notices this binary carries,
//! Escape quits. The window title shows the page's own label where the document states one
//! (§12.4.2), the page number, and how many things on the page could not be drawn; the things
//! themselves are printed.
//!
//! # What is here and what is not
//!
//! This is **consumer #1 of `viewer-core`** and a **tier-2 host**: everything about documents,
//! pages, zoom, links and actions is in that crate, and what is left here is a window, a
//! keyboard, a GPU and the two decisions a host owns — which files a document may name, and what
//! to do when it asks for a password.
//!
//! And, since the hundred-and-sixty-sixth session, **chrome**: `viewer_ui::chrome` draws a
//! sidebar of this program's own — §12.3.3's outline, §8.11.4.3's layers with their switches,
//! and §7.11.4's embedded files — because winit is a window and an event loop and there is no
//! toolkit here to ask for a tree view. A native host would use its platform's, from the same
//! three queries; what is host-specific is the drawing, not the data.
//!
//! Tier 2 means the pixels never cross the boundary: `viewer-core` hands over a display list and
//! a target, this draws it onto the surface with `render-quorra`, and answers `Rendered::Presented`.
//! A tier-1 host would take the raster back instead; the protocol is otherwise identical, which
//! is the property that makes the interface worth having.
//!
//! # Reporting incomplete pages
//!
//! When the interpreter cannot draw something — an unsupported font, a shading — it is said out
//! loud. A viewer that renders a page missing half its content and looks confident about it is
//! worse than one that admits the gap, and the person looking at it is the only one in a
//! position to judge whether what is missing matters.
//!
//! # Where the rest of it is
//!
//! This file is the launch path and nothing else: the arguments, the two threads it starts, and
//! the event loop it hands them to. The window's state is [`app::App`] and every other module is
//! an `impl` block on it, one per thing a host does — because a host is a list of decisions
//! rather than a layer, and a reader looking for one of them should not have to read the others.
//!
//! | module | what it owns |
//! |---|---|
//! | [`arguments`] | the command line, and the settings that precede a document |
//! | [`trace`] | `--trace`'s topics, and the sentence each line says |
//! | [`timing`] | the launch timeline and what every frame cost |
//! | [`app`] | the window's state, its title, and the lists a document states once |
//! | [`dispatch`] | commands into `viewer-core` and what this window does about each event |
//! | [`files`] | the filesystem, which rule 2 leaves entirely to a host |
//! | [`sidebar`] | the panel this program draws, and where the pointer is with respect to it |
//! | [`typing`] | the keyboard inside a field's value, and the clipboard |
//! | [`find`] | Annex O's `search`: the bar, its steps, and the colour over what it found |
//! | [`overlays`] | the geometry drawn over the page, in colours no clause states |
//! | [`presentation`] | §12.4.4's clock and the transitions it draws |
//! | [`surface`] | the graphics device or the processor's window, and one frame on it |
//! | [`renderer`] | the graphics device on a thread of its own, and the frames that cross back |
//! | [`composer`] | the processor on a thread of its own, for the window that has no device |
//! | [`stale`] | the reprojection a slow view change shows until the real frame lands |
//! | [`access`] | §14.7's tree handed to AccessKit |
//! | [`window`] | winit's callbacks and the key table |

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line application: stdout is a reporting channel and a panic on a \
              missing display is the intended failure"
)]

// **`#[path]` on every one of them, and cargo is why.** A binary's crate root resolves `mod x`
// against its *own* directory, which here is `src/bin` — where cargo also discovers one binary
// target per file. So the modules of this program would each become a program, and naming the
// directory explicitly is what keeps fourteen modules from becoming fourteen binaries.
#[path = "pdf-viewer/access.rs"]
mod access;
#[path = "pdf-viewer/app.rs"]
mod app;
#[path = "pdf-viewer/arguments.rs"]
mod arguments;
#[path = "pdf-viewer/cadence.rs"]
mod cadence;
#[path = "pdf-viewer/composer.rs"]
mod composer;
#[path = "pdf-viewer/dispatch.rs"]
mod dispatch;
#[path = "pdf-viewer/files.rs"]
mod files;
#[path = "pdf-viewer/find.rs"]
mod find;
#[path = "pdf-viewer/overlays.rs"]
mod overlays;
#[path = "pdf-viewer/presentation.rs"]
mod presentation;
#[path = "pdf-viewer/renderer.rs"]
mod renderer;
#[path = "pdf-viewer/sidebar.rs"]
mod sidebar;
#[path = "pdf-viewer/stale.rs"]
mod stale;
#[path = "pdf-viewer/surface.rs"]
mod surface;
#[path = "pdf-viewer/timing.rs"]
mod timing;
#[path = "pdf-viewer/trace.rs"]
mod trace;
#[path = "pdf-viewer/typing.rs"]
mod typing;
#[path = "pdf-viewer/window.rs"]
mod window;

use std::path::PathBuf;

use viewer_core::{Command, DocumentId, Event, PageTarget, RestrictionLevel, Viewer};
use viewer_ui::chrome::{About, Chrome, FindBar, Sidebar};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::App;
use crate::arguments::{Arguments, arguments, spawn_instancing};
use crate::timing::{FrameLog, Launch};

/// The third-party notices this binary is obliged to carry, printed by `--licences`.
///
/// Both licences covering the compiled-in standard 14 fonts require a *binary* distribution to
/// reproduce their notices "in the documentation and/or other materials provided with the
/// distribution", and until the hundred-and-forty-eighth session this program had nowhere to put
/// them. `include_str!` rather than a path, for the same reason the fonts themselves are
/// `include_bytes!`d: a notice that can go missing between the binary and the file system is not
/// carried by the binary.
///
/// `--licenses` is accepted too. The project spells it the other way and a person typing the
/// other spelling wants the same thing.
const NOTICE: &str = include_str!("../../../../NOTICE");

/// The one document this program opens.
///
/// `viewer-core` keeps a set of them because §12.6.4.4's embedded go-to and a tabbed host both
/// need one; this window shows one at a time, so it names one.
const DOCUMENT: DocumentId = DocumentId(0);

/// Reads the file and opens it, wherever this is called from.
///
/// Split out of `main` because it is called on a thread of its own — see the comment at the call
/// site — so it returns the viewer it made and the events it produced rather than touching an
/// `App` that is being built on another thread at the same time.
///
/// **Rule 2 lives here**: the host owns the filesystem, and this is the only place a path becomes
/// bytes.
fn open_document(
    path: &std::path::Path,
    opens_at: Option<usize>,
    fragment: Option<&str>,
    restrictions: RestrictionLevel,
) -> (Viewer, Vec<Event>) {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.to_string_lossy());
            std::process::exit(1);
        }
    };
    // No viewport yet: the window does not exist and may not for another 50 ms. The core renders
    // nothing into one with no extent, which is exactly right — there is nothing to render into.
    let mut viewer = Viewer::new(0, 0, 1.0);
    // Before the document rather than after it, for the same reason the sandbox decision is made
    // before anything opens: a policy applied halfway through is not a policy. Nothing an open
    // does is restricted, so this is about where the value *belongs* rather than about an
    // operation it would otherwise miss.
    drop(viewer.handle(Command::Restrict(restrictions)));
    let mut events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: fragment.map(str::to_owned),
        })
        .collect();
    // After the fragment rather than before it: Annex O's parameters are what the *URI* asked
    // for and `--page` is what the person at the keyboard asked for a moment later, so where
    // both name a page the second one wins.
    if let Some(page) = opens_at {
        let turned: Vec<Event> = viewer
            .handle(Command::GoTo(PageTarget::Index(page.saturating_sub(1))))
            .collect();
        events.extend(turned);
    }
    (viewer, events)
}

fn main() {
    let mut launch = Launch::new();
    let Arguments {
        path,
        trace,
        processor,
        backend,
        backend_asked_for,
        opens_at,
        fragment,
        restrictions,
        proxy_pages,
    } = arguments(launch.began);
    launch.mark("arguments");

    // **The document opens on a thread of its own, and this is the launch path's one lever that
    // is ours.** What the window needs is a chain of three steps that must happen in order and on
    // this thread — an event loop, a window, a graphics device — and on this machine that chain
    // costs 50 to 90 ms (ADR 0179, ADR 0182). Reading a document depends on none of it, and costs
    // 21 ms on ISO 32000-2's 101 318 objects. Done one after the other those are 110 ms; done side
    // by side they are the longer of the two.
    //
    // `viewer-core`'s rule 4 is "no threads the core was not handed", and this keeps it: the core
    // is *made* on that thread and moved back, so it is still single-threaded and still owns none
    // of its own scheduling. What crosses is a `Viewer` and a `Vec<Event>`, both `Send`.
    let opening = std::thread::spawn({
        let path = path.clone();
        let fragment = fragment.clone();
        move || open_document(&path, opens_at, fragment.as_deref(), restrictions)
    });

    // **And the graphics instance on a second thread**, since the two-hundred-and-eighty-eighth:
    // a `wgpu::Instance` is the driver loader, it needs no window either, and quorra measured it
    // at roughly 80% of what bringing a device up blocks for (their ADR 0014, answering
    // `doc/QUORRA_FEEDBACK.md` section 8.2). Its own thread rather than the document's, and the
    // difference is not style: the device *needs* the instance and does not need the document, so
    // the instance is joined before the presenter is built and the document after it — one thread
    // for both would make the first join wait for the second's work.
    //
    // **Not under `--cpu`, and that is the whole of the flag's new meaning.** Creating the
    // instance *is* loading the driver, so a run that will not draw on the device must not make
    // one: the thread is not spawned, `resumed` builds no presenter, and nothing in the process
    // opens an ICD. Before the three-hundred-and-eighty-fourth session this line ran regardless
    // and a driver that faulted while loading took `--cpu` down with it (ADR 0221).
    let instancing = spawn_instancing(processor, backend);

    let chrome = match Chrome::new() {
        Ok(chrome) => Some(chrome),
        Err(problem) => {
            eprintln!("note: no panel: {problem}");
            None
        }
    };
    launch.mark("chrome fonts");

    let mut app = App {
        // No viewport until the window exists. The core renders nothing into one with no
        // extent, which is exactly right: there is nothing to render into yet.
        viewer: Viewer::new(0, 0, 1.0),
        title: path.to_string_lossy().into_owned(),
        path: PathBuf::from(&path),
        embedded: None,
        fragment,
        // §12.7.6.4's import-data action names a file, and this is the only place that name is
        // allowed to mean anything: a *sibling of the document being shown*. See `supply`.
        directory: std::path::Path::new(&path)
            .parent()
            .map(std::path::Path::to_path_buf),
        caption: String::new(),
        requests: Vec::new(),
        unacknowledged: Vec::new(),
        presented: None,
        presentation: None,
        // Table 29's own default, replaced by whatever the catalog states the moment the document
        // opens (`App::obey_page_mode`).
        layout: pdf_model::viewer_preferences::PageLayout::SinglePage,
        arming: None,
        trace,
        processor,
        proxy_pages,
        backend,
        backend_asked_for,
        cursor: (0.0, 0.0),
        dragging: false,
        control: false,
        shift: false,
        pinch: 0.0,
        dirty: false,
        attempts: 0,
        chrome,
        panel: Sidebar::default(),
        about: About::default(),
        find: FindBar::default(),
        pages_left: 0,
        searched_at: None,
        outline: pdf_model::outline::Outline::default(),
        attachments: Vec::new(),
        articles: Vec::new(),
        collection: None,
        typing: None,
        clipboard: String::new(),
        drawing: None,
        pages: Vec::new(),
        information: pdf_model::metadata::Information::default(),
        metadata: None,
        state: None,
        opening: Some(opening),
        instancing,
        launch,
        frames: FrameLog::default(),
        stale: stale::Stale::default(),
        // Replaced with the surface's own rate the moment there is a window; `doc/todo/36`'s
        // floor until then, which is what a program with no display to ask presents at.
        cadence: cadence::Cadence::default(),
        accessibility: None,
        spoken: None,
        waker: None,
    };

    let event_loop = EventLoop::new().expect("an event loop requires a display server");
    // The proxy is the only way into this loop from another thread, and the accessibility bridge's
    // is the only such thread. Taken here rather than where the bridge comes up, because
    // `ActiveEventLoop` — which is all a running loop hands its callbacks — cannot make one.
    app.waker = Some(event_loop.create_proxy());
    app.launch.mark("event loop");
    // Redraw on request rather than continuously: a document viewer is idle almost all the time,
    // and a spinning loop would drain a battery for nothing.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app).expect("event loop failed");
}
