//! The viewer confined, with a window on it: pages arrive from `pdf-view-worker` over a pipe.
//!
//! ```text
//! cargo run --release -p viewer-ui --bin quorra-confined -- document.pdf
//! ```
//!
//! **This is the first host on `viewer-confined`'s boundary** (ADR 0713), which is the boundary
//! `CLAUDE.md` principle 3 asks for: the document, the content interpreter and the rasteriser —
//! where a hostile file's bytes actually go — run in a separate process under seccomp-BPF,
//! Landlock and an address-space ceiling, with no filesystem and no network. This program is
//! what is left on the outside: a window, a keyboard, and pixels. It never parses a byte of the
//! document; what it draws is either a raster the worker produced or a display list the wire
//! decoder bounds-checked field by field, chosen per page by size (ADR 0607).
//!
//! Arrows, Page Up and Down or Space turn pages, Home and End jump, `+` and `-` zoom, the wheel
//! scrolls, `q` quits, Tab walks a form's fields, `m` measures. **Escape is the abort**: it ends the worker with a kill it cannot decline
//! (ADR 0241) and takes back this side's own drawing thread (ADR 0650) — the reader's answer to
//! a document written to take for ever, and it does not block. The window title carries the page
//! and §12.4.2's label; what a page could not draw is printed, as everywhere else in this tree.
//!
//! # What this window is, and is not
//!
//! It is deliberately the *smallest complete* host on this boundary, and its scope is a decision
//! with its cost written down (ADR 0713) rather than a promise of more: no panels, no form
//! controls drawn over the page, no selection, no find bar — each of those is chrome the three
//! established windows already have in-process, and moving *them* onto this boundary is the
//! remainder `doc/todo/15` names. What it does take from a form is the keyboard's share — Tab
//! through §12.5.1's order, characters into a single-line text field, Enter to commit and Control
//! and S to save — because §12.7.4.3's commit runs where the scripts are, inside the confinement,
//! and a boundary nobody can type across has not been shown to carry it (ADR 1592). What is complete is the part no other window has at all: every page on the screen came
//! out of the sandboxed process, on both of ADR 0607's arms, with the drawing of the marks arm
//! interruptible and the worker killable from the keyboard. A link to another program, a file the
//! document names, a form sent to a server and form data fetched from one are put to the reader at
//! the level `--links=`, `--remote-documents=` and `--submissions=` set, as in the other three
//! windows, on a card answered by Enter and Escape (`asking`, ADR 2040); the form is composed in
//! the worker, sent from this side, and the server's answer handed back to the worker to import
//! (`sending`, ADR 2076). What this window still cannot serve — a second document beside the one
//! it holds — is refused **by name**, on standard error, never quietly.
//!
//! # When the worker dies without being asked to
//!
//! A ceiling breach, the seccomp filter firing, an allocation deep inside the interpreter that
//! `RLIMIT_AS` refuses: each ends the worker, and each is one page's breach rather than the
//! document's end. **This window starts another worker, opens the file again and goes back to the
//! view the reader was looking at**, without sending the command that killed the last one —
//! [`viewer_confined::Resuming`] decides whether a refusal is worth that and how many in a row are
//! enough, so that a second host on this boundary cannot answer it differently. **The
//! magnification and the position on the page come back too, exactly** (ADR 0737), which they did
//! not until [`Query::View`] existed: this window asks that question per frame and hands the
//! answer back as [`Command::View`], so what is restored is the reader's own view rather than the
//! document's opening one. A worker the **reader** ended (Escape, below) is never started again;
//! that is what pressing it meant.
//!
//! # §7.6.4.1's prompt, and which way the password goes
//!
//! An encrypted document is not a refusal here (ADR 0718; it was under ADR 0713's first scope):
//! for such a document *open* — the first verb of this window's charter — is unreachable without
//! the prompt, so the prompt completes the scope rather than extending it. Everything about it is
//! the shared machinery: [`viewer_host::password`] counts the attempts and decides what an empty
//! entry means, [`viewer_ui::chrome::PasswordCard`] draws the card, and what this file adds is
//! wiring. The password crosses **into** the confinement inside [`Command::Open`], as a
//! [`viewer_core::Secret`], and that direction is the design rather than an accident: §7.6.4's
//! decryption happens where the document's bytes are, and the confinement is precisely what
//! bounds where the password can go from there — no filesystem, no network, nothing but the pipe
//! back to this process. Decrypting on this side so that the password never crossed would put the
//! hostile bytes through the cryptography in the *unconfined* process, which is the boundary
//! defeated by courtesy.
//!
//! # Where the drawing happens
//!
//! **The graphics device draws this window's pages** (ADR 0725), which is what the marks cross
//! the pipe *for*: the device is this side's by necessity — a confined process holding one dies
//! on its first `ioctl` (ADR 0607) — so a list payload goes to `render-raster` on
//! [`crate::device`]'s render thread and its pixels never touch the processor, while a raster
//! payload is wrapped as the one-image list the same device places. The processor keeps exactly
//! the two jobs `CLAUDE.md` leaves it: `--cpu` is the window with no device — a raster payload
//! placed as it arrives, a list payload drawn by `render-cpu` on [`viewer_host::drawing`]'s
//! thread, presented through [`viewer_ui::software`] — and that same interruptible thread is
//! what draws a frame the device *refuses*, out loud, because nothing bounds what a display
//! list costs to draw (ADR 0650) and a hostile page must be drawn where an interrupt can reach
//! it. A machine whose device will not come up falls to the processor's path by itself, saying
//! so.

#![expect(
    clippy::print_stderr,
    clippy::expect_used,
    reason = "a command-line application: standard error is a reporting channel, and a machine \
              that cannot create a window or an event loop should stop loudly"
)]

#[path = "quorra-confined/asking.rs"]
mod asking;
#[path = "quorra-confined/device.rs"]
mod device;
#[path = "quorra-confined/exchange.rs"]
mod exchange;
#[path = "quorra-confined/pricing.rs"]
mod pricing;
#[path = "quorra-confined/screen.rs"]
mod screen;
#[path = "quorra-confined/sending.rs"]
mod sending;
#[path = "quorra-confined/typing.rs"]
mod typing;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use render_raster::QuorraWindowRenderer;
use viewer_confined::{
    Canceller, Confined, ConfinedError, Payload, Reopen, Reply, Resume, Resuming,
};
use viewer_core::{Command, DocumentId, Event, PageTarget, Query, Viewing, Zoom};
use viewer_host::drawing::Drawing;
use viewer_host::trace::{Topic, Trace};
use viewer_host::{Ask, Asking, Supplied};
use viewer_ui::chrome::{Chrome, PasswordCard, QuestionCard};
use viewer_ui::software::SoftwareSurface;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::device::{Device, Presented};
use crate::exchange::{Exchanging, Held, Waiting};
use crate::screen::{Draw, Screen};

/// The one document this window opens; the identity is the host's to choose.
const DOCUMENT: DocumentId = DocumentId(0);

/// What this window does with a page whose whole raster would be past `viewer_core::MAX_PIXELS`:
/// it draws the page from its marks at the size of the window, on the device or, on the processor,
/// as the window shows it (`screen`'s windows), so the worker sends the marks rather than refusing
/// the page (ADR 1944).
const OVERSIZE: Command = Command::Oversize(viewer_core::Oversize::Listed);

/// How far one wheel notch moves the page, in device pixels.
///
/// No clause states a wheel distance and neither toolkit host chooses one — each takes what GTK
/// or Qt hands it — so this window's is a documented choice: three text lines' worth, the common
/// desktop convention.
const WHEEL_NOTCH: f32 = 48.0;

/// The command line: a document, optionally `--trace[=topics]`, `--cpu` and two levels.
struct Arguments {
    /// The file to open.
    path: PathBuf,
    /// Which trace topics to print.
    topics: u8,
    /// `--cpu`: no graphics device — the flagship's flag with the flagship's meaning (ADR
    /// 0221): no instance is created, so no driver is loaded, and the window presents through
    /// the processor.
    processor: bool,
    /// `--machine-fonts`: `doc/todo/59`'s resource port, off unless it is asked for.
    ///
    /// It gives the worker no capability at all — the descriptor is opened *here* — and what it
    /// buys is a page that names an uninstalled face drawn in that face rather than in a
    /// compiled-in Latin one (ADRs 0870, 0880).
    faces: bool,
    /// §12.8.1's anchors, §8.10.4's target documents and §8.11.4.4's reader, in the words every
    /// window takes (ADRs 1580, 1581).
    reader: viewer_host::ReaderWords,
    /// §12.6.4.8: what this window does when a link asks for a URI, per [`viewer_host::LINKS`].
    links: viewer_host::Links,
    /// §12.6.4.3: what it does when a document names another PDF file, per
    /// [`viewer_host::REMOTE_DOCUMENTS`].
    remote_documents: viewer_host::RemoteDocuments,
    /// §12.7.6.2's form and an import named by an absolute URI, per [`viewer_host::SUBMISSIONS`]
    /// (ADR 2076).
    submissions: viewer_host::Submissions,
    /// §12.6.4.6's launched document that is not a PDF, per [`viewer_host::LAUNCHED_DOCUMENTS`]
    /// (ADR 2171).
    launched_documents: viewer_host::LaunchedDocuments,
}

/// What the usage line names, after the program's name.
const USAGE: &str = "[--trace[=topics]] [--cpu] [--links=L] [--remote-documents=R] \
                     [--submissions=S] [--launched-documents=D]";

/// A message's `Debug` form cut to a line, for the trace.
///
/// The cut is what the other hosts apply, and it was load-bearing here before it was cosmetic:
/// `Command::Open` used to carry the whole document as a `Vec<u8>` whose `Debug` is five
/// characters a byte, and the first run of this program wrote three quarters of a megabyte of
/// trace for one open. `pdf_syntax::FileBytes` prints its length and how it is held now, and a
/// `Secret` prints no password whatever the length, so what the cut protects is a long event.
fn brief(message: &impl std::fmt::Debug) -> String {
    let whole = format!("{message:?}");
    if whole.chars().count() > 120 {
        let mut cut: String = whole.chars().take(120).collect();
        cut.push('…');
        return cut;
    }
    whole
}

/// A file's own name, or the whole path where it has none.
fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.to_string_lossy().into_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The four of the settings words this window carries out: a link, a file the document names, a
/// form sent to a server and a launched document handed to this machine's own handler, each put to
/// the reader on a card (ADRs 2040, 2076, 2171).
const CARRIED: [&str; 4] = [
    viewer_host::LINKS,
    viewer_host::REMOTE_DOCUMENTS,
    viewer_host::SUBMISSIONS,
    viewer_host::LAUNCHED_DOCUMENTS,
];

/// Why a settings word this window does not carry is refused: what the window does instead.
fn not_carried(word: &str) -> &'static str {
    if word == viewer_host::EMBEDDED_DOCUMENTS {
        "this window holds one document and opens none beside it, so an embedded document is \
         refused at every level"
    } else if word == viewer_host::SEPARATIONS {
        "this window asks for no separation simulation: its pages are drawn in the worker as \
         the screen shows them"
    } else {
        "this window sets no level for a document's restrictions: each operation stands at \
         off, where none is refused"
    }
}

/// Reads the arguments or says why not; the usage line is the refusal.
fn arguments() -> Arguments {
    let mut path = None;
    let mut topics = 0u8;
    let mut processor = false;
    let mut faces = false;
    let mut typed = viewer_host::policy::Words::default();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        // Read as every window reads them (ADRs 1581, 2120, 2156): the reader's words on this
        // side, which has the filesystem, carried across the wire; a settings word this window
        // has no level for refused by name rather than taken and not obeyed.
        let taken = typed
            .take(std::ffi::OsStr::new(&argument), &mut arguments)
            .unwrap_or_else(|complaint| {
                eprintln!("{complaint}");
                std::process::exit(2);
            });
        if let Some(viewer_host::policy::Taken::Setting(word)) = taken
            && !CARRIED.contains(&word)
        {
            eprintln!("{argument}: {}", not_carried(word));
            std::process::exit(2);
        }
        if taken.is_some() {
            // Taken, and in force.
        } else if argument == "--trace" {
            topics = u8::MAX;
        } else if argument == "--cpu" {
            processor = true;
        } else if argument == viewer_host::MACHINE_FONTS {
            faces = true;
        } else if let Some(list) = argument.strip_prefix("--trace=") {
            topics = match viewer_host::parse_topics(list) {
                Ok(topics) => topics,
                Err(said) => {
                    eprintln!("{said}");
                    std::process::exit(2);
                }
            };
        } else if path.is_none() {
            path = Some(PathBuf::from(argument));
        } else {
            eprintln!(
                "usage: quorra-confined {USAGE} [{}] {} document.pdf",
                viewer_host::MACHINE_FONTS,
                viewer_host::ReaderWords::USAGE
            );
            std::process::exit(2);
        }
    }
    let Some(path) = path else {
        eprintln!(
            "usage: quorra-confined {USAGE} [{}] {} document.pdf",
            viewer_host::MACHINE_FONTS,
            viewer_host::ReaderWords::USAGE
        );
        std::process::exit(2);
    };
    // RFC 0008 section 6.3's level is pinned here, and a person who asked for another is told so
    // rather than given a word this window would not obey (`doc/questions/A193`, ADR 1616).
    let asked = typed.reader().scripts;
    if asked != viewer_host::Scripts::Off {
        eprintln!(
            "note: {} {}: {}",
            viewer_host::SCRIPTS,
            asked.as_str(),
            viewer_host::SCRIPTS_PINNED
        );
        typed.scripts(viewer_host::Scripts::Off);
    }
    let (settings, reader) = typed.into_parts();
    Arguments {
        path,
        topics,
        processor,
        // The word, or the environment for a window nobody typed a command line at.
        faces: viewer_host::offers_machine_fonts(faces),
        reader,
        links: settings.links,
        remote_documents: settings.remote_documents,
        submissions: settings.submissions,
        launched_documents: settings.launched_documents,
    }
}

/// The window, the confined viewer behind it, and what stands between the two.
struct Host {
    /// The document's path — rule 2's: the filesystem is this side's, never the worker's.
    path: PathBuf,
    trace: Trace,
    /// Made before the worker so that an abort during the greeting has something to signal.
    canceller: Canceller,
    /// The confined viewer, from the first `resumed` until it dies or is cancelled.
    confined: Option<Confined>,
    /// Whether this host offers its worker the machine's faces (`doc/todo/59`'s port).
    faces: bool,
    /// The sentence after the worker is gone — cancelled, dead, or refused — shown in the title.
    ///
    /// Once set, every later command is declined here rather than sent to a process that cannot
    /// answer: the worker's document died with it, so there is nothing to resume (ADR 0241).
    stopped: Option<String>,
    /// What the title says while the pages on the screen were drawn on the processor because the
    /// graphics device refused their frame, or nothing — principle 2's "reported out loud", in
    /// the window as well as on standard error (ADR 1466).
    on_the_processor: Option<String>,
    /// Whether the device has refused a frame of the pages the screen took last. A device frame
    /// that lands after a refusal draws the pages *beside* the ones handed to the processor, so it
    /// withdraws the sentence only when nothing since the last take was refused.
    refused_since_take: bool,
    window: Option<Arc<Window>>,
    /// What puts pixels on the window — the device, or the processor (`--cpu`, or a device
    /// that would not come up). Chosen once in `resumed` and never switched mid-life.
    presentation: Option<Presentation>,
    /// The thread creating the graphics instance — roughly 80% of what device bring-up blocks
    /// for, started before the window exists (the flagship's arrangement, raster's ADR 0014).
    /// `None` under `--cpu`, where no driver may be loaded at all.
    instancing: Option<std::thread::JoinHandle<raster_gpu::wgpu::Instance>>,
    /// The thread starting the worker and opening the document in it while the window and the
    /// device come up (ADR 1539), until `resumed` joins it — or, for a page still being read in the
    /// worker past `viewer_host::drawing::WARN`, until the tick that finds it ready (ADR 1811).
    opening: Option<viewer_host::Loading<Result<Opened, String>>>,
    /// What the title says about a document still opening past `WARN`, or nothing.
    loading: Option<String>,
    /// A command still out with the worker past `WARN`, on a thread of its own, until the tick
    /// that finds it answered (ADR 1827).
    exchanging: Option<Exchanging>,
    /// What the reader did while the worker was busy, to be done in order once it answers.
    held: VecDeque<Held>,
    /// What the title says about the command still out, or nothing.
    waiting: Option<Waiting>,
    /// How long the act being done now was held, for the trace line of the command it sends.
    held_for: Option<Duration>,
    /// When the last command went to the worker, which times the frame it causes (ADR 2017).
    sent: Option<Instant>,
    /// What this window has measured of a page past the bound's two routes (ADR 2017).
    pricing: pricing::Pricing,
    /// The drawing thread — `viewer_host::drawing`'s arrangement, on this side's own request
    /// shape. Under `--cpu` it draws every list payload; behind a device it draws the frames
    /// the device refused, which is `CLAUDE.md`'s second job for the CPU backend.
    drawing: Drawing<Draw>,
    screen: Screen,
    /// The extent the last presented frame was composed for, `None` before the first.
    ///
    /// What gates presentation: once something is on the screen, a frame with pages still on the
    /// drawing thread is held back — `doc/todo/37`'s show-what-it-had — unless the window has
    /// changed size, in which case the stale picture is the wrong shape and a partial frame is
    /// the honest one.
    presented: Option<(u32, u32)>,
    /// The title's document half: §12.4.2's label and the page count, updated per event.
    heading: String,
    /// §7.6.4.1's attempts, counted by [`viewer_host::Asking`] so that every host counts alike.
    asking: Asking,
    /// Whether the open document still owes what it says about *itself*.
    ///
    /// Asked for once the first frame is on the window rather than as part of opening, because
    /// §12.8's answer digests the signed part of the file and `CLAUDE.md` principle 2 keeps that
    /// off a launch. `viewer_host::report::Due` holds the rule for every host. ADR 1044.
    report_due: viewer_host::report::Due,
    /// §12.6.4.8's and §12.6.4.3's levels, read off the command line as the other three windows
    /// read them (ADR 2040).
    links: viewer_host::Links,
    remote_documents: viewer_host::RemoteDocuments,
    /// §12.7.6.2's level, read off the command line as the other three windows read it, and the
    /// requests sent under it that have not been answered yet (ADR 2076).
    submissions: viewer_host::Submissions,
    submitter: viewer_host::submit::Submitter,
    /// §12.6.4.6's level, read off the command line as the other three windows read it (ADR 2171).
    launched_documents: viewer_host::LaunchedDocuments,
    /// The *ask* level's card — the flagship's, reused whole — and what it is asking about.
    ///
    /// While it is shown it has the keyboard and the pointer, as the password card does.
    question: QuestionCard,
    asked: Option<asking::Asked>,
    /// A file a document named, supplied to the worker and not yet answered for (`asking`).
    offered: Option<asking::Offered>,
    /// A file a document named whose §7.6.4.1 password the card is asking for.
    prompting: Option<asking::Offered>,
    /// §7.6.4.1's prompt — the card the flagship draws for itself, reused whole.
    ///
    /// While it is shown it has the keyboard: the document behind it is not open, so a key that
    /// turned a page would be turning a page that does not exist (the flagship's own rule).
    password: PasswordCard,
    /// The interface's own faces, loaded the first time the card needs them and never on the
    /// launch path: a document that is not encrypted costs this window no chrome at all.
    chrome: Option<Chrome>,
    /// `q` was pressed; the loop exits at its next turn, where it holds the `ActiveEventLoop`.
    leaving: bool,
    /// Which page of the document is in front of the reader, zero-based.
    ///
    /// Kept because the title says it, and because only this side ever learns it: it arrives as
    /// [`Event::PageChanged`] and the worker that stated it may be gone by the time it is wanted.
    /// **Where a resume goes back to is the whole view rather than this**, which is
    /// [`Query::View`]'s answer and [`Resuming`]'s to hold.
    page: usize,
    /// What has already been tried for this document — [`viewer_confined::Resuming`]'s count.
    resuming: Resuming,
    /// A worker to start at the next turn of the event loop, set by [`Host::died`].
    ///
    /// **Deferred rather than done where the death was seen**, because a resume issues commands
    /// and a command can see the *next* death: recovering inside a recovery would nest one
    /// restart inside another and make the depth of this program's stack a document's to choose.
    /// At the loop's turn there is nothing of the failed exchange left on the stack.
    resume: Option<Reopen>,
    /// The reader's three policy words, kept because every worker is owed them: a worker that
    /// replaces a dead one is a new viewer, and one that lost the anchors would judge a signature
    /// differently after a crash (ADR 1580).
    reader: viewer_host::ReaderWords,
    /// §12.7.4.2's qualified name of the text field the keyboard is in, or nothing (ADR 1592).
    typing: Option<String>,
    /// The modifiers held: Shift turns §12.5.1's Tab round, and Control makes S the save.
    modifiers: winit::keyboard::ModifiersState,
    /// §12.9's measuring mode and its points, shared with the other windows' rule for when a
    /// press is a point (ADR 1191).
    measuring: viewer_host::Measuring,
    /// Where the pointer is, in the window's device pixels.
    cursor: (f32, f32),
}

/// A worker started, confined, and holding the document with page one interpreted: what the
/// opening thread hands `resumed` (ADR 1539).
struct Opened {
    /// The worker.
    confined: Confined,
    /// What the open said, to be answered on the window's thread.
    events: Vec<Event>,
}

/// Starts a confined worker, and offers it the machine's faces where this host was asked to.
///
/// One function rather than two call sites, because the first worker and the one that replaces a
/// dead one must be given the same thing: a window whose *second* worker lost the port would draw
/// a page differently after a crash, which is the shape nobody would look for.
fn start_worker(canceller: &Canceller, faces: bool) -> Result<Confined, ConfinedError> {
    let mut confined = Confined::start_with(canceller)?;
    if faces {
        confined.offer_machine_faces();
    }
    Ok(confined)
}

/// What puts this window's pixels up (ADR 0725): the graphics device, or the processor.
enum Presentation {
    /// A `render-raster` device on its own thread, presented through a `raster_gpu::Presenter`.
    ///
    /// Boxed because a `Device` holds the renderer until the first job moves it to its thread,
    /// and an enum is as wide as its widest variant.
    Device(Box<Device>),
    /// The processor's surface — `--cpu`, or the path a machine with no usable device falls to.
    Software(SoftwareSurface),
}

impl Host {
    fn new(path: PathBuf, topics: u8, processor: bool, faces: bool, began: Instant) -> Self {
        Self {
            heading: path.file_name().map_or_else(
                || path.to_string_lossy().into_owned(),
                |name| name.to_string_lossy().into_owned(),
            ),
            path,
            trace: Trace::of(topics, began),
            canceller: Canceller::new(),
            confined: None,
            stopped: None,
            on_the_processor: None,
            refused_since_take: false,
            window: None,
            presentation: None,
            // Creating the instance *is* loading the driver, so `--cpu` must not spawn this
            // thread — the flagship's rule (ADR 0221), kept here for the same crash.
            instancing: (!processor).then(|| std::thread::spawn(QuorraWindowRenderer::instance)),
            opening: None,
            loading: None,
            exchanging: None,
            held: VecDeque::new(),
            waiting: None,
            held_for: None,
            sent: None,
            pricing: pricing::Pricing::default(),
            drawing: Drawing::new(),
            screen: if processor {
                Screen::new()
            } else {
                Screen::for_device()
            },
            presented: None,
            asking: Asking::new(),
            report_due: viewer_host::report::Due::default(),
            links: viewer_host::Links::default(),
            remote_documents: viewer_host::RemoteDocuments::default(),
            submissions: viewer_host::Submissions::default(),
            submitter: viewer_host::submit::Submitter::new(),
            launched_documents: viewer_host::LaunchedDocuments::default(),
            question: QuestionCard::default(),
            asked: None,
            offered: None,
            prompting: None,
            password: PasswordCard::default(),
            chrome: None,
            leaving: false,
            page: 0,
            resuming: Resuming::new(),
            resume: None,
            faces,
            reader: viewer_host::ReaderWords::default(),
            typing: None,
            modifiers: winit::keyboard::ModifiersState::empty(),
            measuring: viewer_host::Measuring::default(),
            cursor: (0.0, 0.0),
        }
    }

    /// Starts a confined worker, and offers it the machine's faces where this host was asked to.
    ///
    /// One function rather than two call sites, because the first worker and the one that replaces
    /// a dead one must be given the same thing: a window whose *second* worker lost the port would
    /// draw a page differently after a crash, which is the shape nobody would look for.
    fn start_confined(&self, canceller: &Canceller) -> Result<Confined, ConfinedError> {
        start_worker(canceller, self.faces)
    }

    /// Starts the worker and opens the document in it on a thread of its own, while the window
    /// and the graphics device come up on this one (ADR 1539).
    ///
    /// **The launch path's overlap, in this window's shape.** The worker is a process, so what
    /// the thread holds is a pipe: it starts the worker, tells it there is no viewport yet, and
    /// sends `Command::Open`, and the worker interprets page one with nothing to render it into
    /// (`viewer_confined::worker`'s `perform` calls `Viewer::anticipate` after an open). Decoding
    /// stays where it was, inside the confined process; this thread is in the unconfined host and
    /// reads only the file's descriptor and the worker's frames (trap 83 is about the other side).
    /// The first `Resize` after the join then goes straight to the page's marks.
    fn anticipate(&mut self) {
        let (path, faces, canceller, trace, reader) = (
            self.path.clone(),
            self.faces,
            self.canceller.clone(),
            self.trace,
            self.reader.clone(),
        );
        self.opening = Some(viewer_host::Loading::start_reporting(move |progress| {
            let starting = Instant::now();
            let mut confined = start_worker(&canceller, faces).map_err(|e| e.to_string())?;
            // The worker relays how far its page has got inside the exchange this thread waits on,
            // and the window reads the figure this stores it in (ADR 1811).
            confined.report_progress_to(progress);
            trace.say(
                Topic::Launch,
                format_args!(
                    "worker started and confined in {:.1} ms",
                    starting.elapsed().as_secs_f64() * 1e3
                ),
            );
            // Opened rather than read: the file crosses to the worker as its descriptor beside
            // `Command::Open`, and this side holds no byte of it (ADR 0812).
            let bytes = pdf_syntax::FileBytes::on_disk(&path)
                .map_err(|problem| format!("cannot open {}: {problem}", path.display()))?;
            // No viewport until the window has one: the worker's own starting size would have
            // page one drawn at a size nobody asked for.
            let mut events = confined
                .handle(&Command::Resize {
                    width: 0,
                    height: 0,
                    scale: 1.0,
                })
                .map_err(|e| e.to_string())?;
            // The reader's policy before the document, for `Command::Restrict`'s reason: a policy
            // applied halfway through is not a policy. Read here, on this side's filesystem, and
            // carried across the wire as bytes (ADR 1580).
            let supply = reader.commands();
            for refusal in &supply.refused {
                eprintln!("note: {refusal}");
            }
            for command in supply.commands.iter().chain([&OVERSIZE]) {
                events.extend(confined.handle(command).map_err(|e| e.to_string())?);
            }
            events.extend(
                confined
                    .handle(&Command::Open {
                        id: DOCUMENT,
                        bytes,
                        password: None,
                        fragment: None,
                    })
                    .map_err(|e| e.to_string())?,
            );
            trace.say(Topic::Launch, format_args!("document opened in the worker"));
            Ok(Opened { confined, events })
        }));
    }

    /// The worker is gone or unusable; every later command is declined with this sentence.
    ///
    /// The handle is dropped here rather than kept: dropping a `Confined` kills and **reaps** its
    /// worker, and a host that only cancelled would sit beside a zombie for the rest of the
    /// window's life — observed in this program's own first abort, `pdf-view-worker` `<defunct>`
    /// until exit.
    fn stop(&mut self, said: String) {
        eprintln!("{said}");
        self.stopped = Some(said);
        self.confined = None;
        // Nothing is left to do what the reader did meanwhile, and nothing is being waited for.
        self.held.clear();
        self.waiting = None;
        self.retitle();
    }

    /// The confined viewer answered with a refusal instead of a message: another worker, or not.
    ///
    /// **A dead worker is one page's breach, not the document's end**, and this is where that is
    /// decided. `viewer_confined::Resuming` owns the decision — which refusals are worth another
    /// worker, and how many in a row — because it is the part two confined hosts must not disagree
    /// about; what is here is what only a host has: the file, the window, and the page.
    ///
    /// The dead handle is dropped now rather than at the resume, for the reason [`Host::stop`]
    /// drops it: a `Confined` reaps its worker where a `Canceller` only kills, and a window with
    /// a `<defunct>` process beside it is what the first abort in this program's life left behind.
    fn died(&mut self, problem: &ConfinedError) {
        match self.resuming.after(problem) {
            Resume::Stop => self.stop(problem.to_string()),
            Resume::Reopen(reopen) => {
                self.confined = None;
                eprintln!(
                    "{problem}; starting another and returning to page {} at the magnification \
                     and position the reader was at — attempt {} of {}.",
                    reopen.view.page.saturating_add(1),
                    reopen.attempt,
                    reopen.of
                );
                self.heading = format!(
                    "{} — restarting the confined viewer ({} of {})",
                    self.name(),
                    reopen.attempt,
                    reopen.of
                );
                self.retitle();
                self.resume = Some(reopen);
            }
        }
    }

    /// Starts another worker for the same document and puts the reader back on their page.
    ///
    /// The pages already on the screen are left exactly where they are — `doc/todo/37`'s
    /// show-what-it-had, and the reason nothing here forgets them: they are pixels and marks this
    /// side already holds, and a window that blanked while it recovered would tell the reader
    /// something worse than the truth. The next frame replaces them, page by page, when it lands.
    ///
    /// The command that killed the last worker is *not* among these: the resume sends the extent,
    /// the document and the view, and stops. If the document's own open is what kills a worker,
    /// this fails again and `Resuming`'s budget ends it.
    ///
    /// **The view is one command and not three** (ADR 0737): [`Command::View`] states the page,
    /// the magnification and the offset together, in the values [`Query::View`] answered with,
    /// which is what makes the restore exact. Replaying a `GoTo`, a `Zoom` and a `Scroll` of the
    /// difference would land near it and would need this window to know which of the three resets
    /// the others.
    fn reopen(&mut self, reopen: Reopen) {
        if self.stopped.is_some() {
            return;
        }
        self.forget_the_offers();
        let bytes = match pdf_syntax::FileBytes::on_disk(&self.path) {
            Ok(bytes) => bytes,
            Err(problem) => {
                self.stop(format!("cannot open {}: {problem}", self.path.display()));
                return;
            }
        };
        // A canceller of its own, because a cancellation holds the worker it was made for: the
        // old one names a process that has already been reaped, and Escape must reach the new one.
        let canceller = Canceller::new();
        let starting = Instant::now();
        let confined = match self.start_confined(&canceller) {
            Ok(confined) => confined,
            Err(problem) => {
                self.stop(problem.to_string());
                return;
            }
        };
        self.trace.say(
            Topic::Launch,
            format_args!(
                "worker started again and confined in {:.1} ms",
                starting.elapsed().as_secs_f64() * 1e3
            ),
        );
        // Asked of the new worker rather than assumed from the old one: a kernel's answer is
        // about the process that asked, and a person relying on the sandbox is owed each one.
        if let Some(short) = confined.confinement().shortfall() {
            eprintln!("confinement shortfall: {short}");
        }
        self.canceller = canceller;
        self.confined = Some(confined);

        // The restart goes in front of whatever the reader did while the last worker was dying:
        // those acts are for the view this puts back.
        self.ahead_of_the_held(|host| {
            if let Some(window) = host.window.as_ref() {
                let extent = window.inner_size();
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a display's scale factor is a small ratio"
                )]
                let scale = window.scale_factor() as f32;
                host.dispatch(Command::Resize {
                    width: extent.width,
                    height: extent.height,
                    scale,
                });
            }
            // The new worker is a new viewer, so it is told again what the first one was told.
            for command in host
                .reader
                .commands()
                .commands
                .into_iter()
                .chain([OVERSIZE])
            {
                host.dispatch(command);
            }
            host.dispatch(Command::Open {
                id: DOCUMENT,
                bytes,
                // The password is deliberately not kept on this side (`password_answered` says
                // why), so an encrypted document asks for it again — §7.6.4.1's card, exactly as
                // at launch.
                password: None,
                fragment: None,
            });
            host.dispatch(Command::View(reopen.view));
        });
    }

    /// The file's own name, for the title.
    fn name(&self) -> String {
        file_name(&self.path)
    }

    /// Sends one command, hands its events on, and pulls the frame the view now shows — or, while
    /// the worker is busy, holds it to be sent once the worker answers (ADR 1827).
    fn dispatch(&mut self, command: Command) {
        if self.stopped.is_some() {
            return;
        }
        if self.must_hold() {
            self.hold(Held::Command {
                command,
                at: Instant::now(),
            });
            return;
        }
        self.send(command);
    }

    /// §7.6.4.1's prompt, which is the one arm of [`Self::event`] long enough to be its own.
    ///
    /// "an interactive PDF processor *should* prompt", and this window is one (ADR 0718; the
    /// refusal it replaces was ADR 0713's scope decision). The policy — how many attempts, what
    /// exhaustion means — is `viewer_host::password`'s, shared with the three established
    /// windows.
    fn password_required(&mut self) {
        if self.chrome.is_none() {
            match Chrome::new() {
                Ok(chrome) => self.chrome = Some(chrome),
                Err(said) => {
                    // A build whose compiled-in faces will not parse cannot draw the card;
                    // refusing by name is what is left (trap 5).
                    self.stop(format!(
                        "{} is encrypted and this build cannot draw the prompt ({said}); open \
                         it in quorra, quorra-gtk or quorra-qt",
                        self.path.display()
                    ));
                    return;
                }
            }
        }
        match self.asking.required() {
            Ask::Prompt { attempt, of } => {
                self.password.ask(viewer_host::password::prompt(
                    &file_name(self.prompted_for()),
                    attempt,
                    of,
                ));
                self.card_changed();
            }
            Ask::Exhausted => {
                // Stop asking and leave the window open — `viewer_host::password`'s rule: no
                // host may make `Exhausted` mean the window closes. A file a document named is
                // declined to the worker that holds it, and the window's own stays.
                eprintln!("note: {}", viewer_host::password::EXHAUSTED);
                if self.decline_the_prompted() {
                    return;
                }
                self.heading = format!("{} — {}", self.name(), viewer_host::password::EXHAUSTED);
                self.retitle();
            }
        }
    }

    /// What this window does about each event the confined viewer sent back.
    ///
    /// Exhaustive on purpose: a message added to the boundary must fail to compile here rather
    /// than fall into a catch-all arm (`doc/ui-boundary.md`'s rule).
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per variant of `viewer_core::Event`, and the count is that enum's. \
                  Splitting it would lose the property the match rests on: a message added to \
                  the core is named here rather than swallowed by a catch-all"
    )]
    fn event(&mut self, event: Event) {
        if self.trace.on(Topic::Events) {
            self.trace
                .say(Topic::Events, format_args!("<- {}", brief(&event)));
        }
        match event {
            Event::Opened { pages, .. } => {
                // A file a document named and this window supplied is the window's document now.
                self.opened_an_offer();
                // The next document's attempts start from nothing (`Asking::opened`'s rule).
                self.asking.opened();
                self.report_due.opened();
                // A document opens where its own view says, which the viewer states as a
                // `PageChanged` when it is not the first page. Until it does, this is page one.
                self.page = 0;
                self.heading = format!("{} — {pages} page(s)", self.name());
                self.retitle();
            }
            // This window sends no `Command::Print`, so the grant never arrives — and it is
            // answered all the same, in the direction that says so, for the reason the two
            // levels below are: an operation added to this window later may not behave like
            // another one in silence. What the arm would mean if it did arrive is RFC 0004 §6's
            // preview: the worker interprets for paper and this window shows what it sends
            // (ADR 1180).
            Event::Printing { pages, .. } => {
                eprintln!(
                    "note: the confined viewer granted a print operation over {pages} page(s), \
                     which this window never asked for"
                );
            }
            // The same for a conversion's grant: `Command::Archive` never crosses (ADR 1954).
            Event::Archiving { .. } => {
                eprintln!(
                    "note: the confined viewer granted a conversion this window never asked for"
                );
            }
            Event::OpenFailed { reason, .. } => {
                // The worker survives an open it refused; only this document is over.
                self.stop(format!("cannot open {}: {reason}", self.path.display()));
            }
            Event::PasswordRequired { .. } => {
                // The file a document named, where it is the one that asks (ADR 1332).
                self.locked_an_offer();
                self.password_required();
            }
            // Five arms with nothing to do, and each for its own reason, kept in one place so
            // that a message added to the boundary still breaks this build: a `Closed` follows
            // this window's own `Close` and nothing else; a `Transition` never fires because this
            // window sends no `Command::Tick` and starts no presentation; a `Searched` answers a
            // `Find` this window never sends; a `Dirty` marks a title this window does not mark,
            // because its title is the confinement's sentence; and an `AttachmentsChanged` names a
            // list this window shows no panel for (ADR 0713).
            Event::Closed(_)
            | Event::Transition { .. }
            | Event::Dirty { .. }
            | Event::Searched { .. }
            | Event::AttachmentsChanged { .. } => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => {
                self.page = index;
                let name = self.name();
                let page = match label.as_deref().map(viewer_host::documents::one_line) {
                    Some(label) => format!("page {label} ({} of {of})", index.saturating_add(1)),
                    None => format!("page {} of {of}", index.saturating_add(1)),
                };
                // §12.3.3's section, which the worker reads beside page one and says again once
                // it has (ADR 1553): the caption the other three windows show, on one line as
                // theirs is (ADR 1973).
                self.heading = match section.as_deref().map(viewer_host::documents::one_line) {
                    Some(section) if !section.is_empty() => format!("{name} — {page} — {section}"),
                    _ => format!("{name} — {page}"),
                };
                self.retitle();
            }
            // The render round trip is the worker's own; this event never crosses the boundary
            // (`viewer_confined`'s module documentation). Arriving here would mean the protocol
            // changed under this window, which is worth a sentence and not a crash.
            Event::NeedsRender(_) => {
                eprintln!("note: a render request crossed the confined boundary; ignored");
            }
            Event::Damage(_) => {
                self.ask_frame();
                self.redraw();
            }
            // §12.6.4.8: the same policy the other three windows ask, at the level this window was
            // started at, and the *ask* level's question on the card (ADRs 1079, 1155, 2040).
            Event::OpenUri { uri, .. } => self.open_uri(&uri),
            // §12.7.6.2: composed by the confined process and sent from this one, at the level
            // `--submissions=` set and on the card at `ask` (ADRs 1291, 2040, 2076).
            Event::Submit {
                document,
                submission,
            } => self.submit(document, *submission),
            // A file the document asks for is a question about *this* machine's filesystem
            // (rule 2), and this window supplies none: said, not swallowed — and **answered**,
            // which is the half that was missing. `Command::Supply` with no bytes is what tells
            // the confined viewer that nobody will supply it, so the action it is holding is
            // dropped and the person is told the link declined; without it the worker waits for
            // an answer that never comes and the click reports nothing (trap 5).
            //
            // §12.6.4.3's file and the three purposes beside it are asked at the level
            // `--remote-documents=` set, as in the other three windows (ADRs 1227, 1239, 2040);
            // §12.7.6.4's own file is the rest of this arm.
            //
            // An import named by an absolute URI is fetched from this side, under the level a
            // submission is sent at, as in the other three windows (ADRs 1527, 2076).
            Event::NeedsFile { purpose, name, .. }
                if viewer_host::under_remote_documents(purpose) =>
            {
                self.needs_remote(purpose, &name);
            }
            Event::NeedsFile {
                document,
                purpose: viewer_core::Purpose::ImportData,
                name,
                ..
            } if viewer_host::import_is_fetched(&name) => self.fetch_import(document, name),
            Event::NeedsFile { purpose, name, .. } => {
                let why = format!(
                    "this window supplies no file a document names, and has no dialogue to ask \
                     you about one. The document asked for {name}"
                );
                eprintln!("{}", viewer_host::policy::supply_note(purpose, &why));
                self.dispatch(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
            // §12.8.3.4.4's policy, whose published copy is a server's: declined at every level
            // here, because what a fetched copy is for is a document opened beside the one that
            // named it, and this window holds one (ADRs 1738, 2076).
            Event::SignaturePoliciesPublished { policies, .. } => {
                for policy in &policies {
                    eprintln!(
                        "{}",
                        viewer_host::policy::signature_policy_declined(
                            policy,
                            "this window holds one document and opens no copy beside it, so it \
                             fetches none, at any level"
                        )
                    );
                }
            }
            // §O.2.1's `ef` at `EmbeddedDocuments::Refuse`, because this window holds one document
            // and opens none beside it (ADR 1331). It passes no fragment, so the arm is the policy's answer said
            // out loud should one ever arrive, rather than a window that opens it unasked.
            Event::Extracted {
                asked, name, bytes, ..
            } if viewer_host::opens_as_document(asked, &bytes) => {
                let why = match viewer_host::may_open_extracted(
                    asked,
                    &name,
                    viewer_host::EmbeddedDocuments::Refuse,
                ) {
                    viewer_host::Unpacking::Refuse(why) => why,
                    // Unreachable at this level and said rather than ignored (trap 5).
                    viewer_host::Unpacking::Ask(words) => {
                        viewer_host::unanswerable(&[words.reasons])
                    }
                    viewer_host::Unpacking::Open | viewer_host::Unpacking::Warn(_) => {
                        viewer_host::embedded_note(
                            &name,
                            Some("this window holds one document and opens none beside it"),
                        )
                    }
                };
                eprintln!("note: {why}");
            }
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            Event::Extracted { .. } => {
                eprintln!("note: the confined viewer sent bytes this window never asked for");
            }
            Event::Refused { notes, .. } => {
                for note in notes {
                    eprintln!("refused by the document's restrictions: {note}");
                }
            }
            // The other two of `CLAUDE.md`'s four levels (ADR 0814). This window sets no level,
            // so each operation stands at the default, `off`, and neither arrives; both are
            // answered all the same, in the direction that obeys, so that a level added to this
            // window later cannot behave like another one in silence.
            Event::Warned { notes, .. } => {
                eprintln!("note: {}", viewer_host::warned(&notes));
            }
            // This window has no copy gesture, so nothing here sends `Command::Copy` and this
            // cannot arrive; the arm exists because `doc/ui-boundary.md`'s rule is that a message
            // added to the core is named here rather than swallowed by a catch-all.
            Event::Copied {
                logical,
                page_order,
                ..
            } => {
                if let Some(copied) = viewer_host::copied(logical, &page_order) {
                    eprintln!(
                        "note: copied {} characters in {}",
                        copied.text.chars().count(),
                        copied.order
                    );
                }
            }
            // And the third, for the reason the arms above give. Two of this window's operations
            // can be restricted — a click that toggles a check box fills a field, and one on a
            // rendition hands its clip over (ADR 1987) — but it sets no level, so both stand at
            // `off` and no question arrives. The other three windows put the question on a
            // dialogue of their own (ADR 1145); what this one owes is that a level never behaves
            // like another in silence, which `viewer_host::unanswerable` is the sentence for.
            Event::Asking {
                document, notes, ..
            } => {
                eprintln!("note: {}", viewer_host::unanswerable(&notes));
                self.dispatch(Command::Answer {
                    document,
                    proceed: false,
                });
            }
            // RFC 0008 section 6.3's question cannot arrive: this window is pinned to `off` and
            // hands its worker no runner. Answered `no` where it does, for the arm above's reason
            // (ADR 1616).
            Event::AskingToRunScripts { document, .. } => {
                eprintln!(
                    "note: {SCRIPTS_PINNED}",
                    SCRIPTS_PINNED = viewer_host::SCRIPTS_PINNED
                );
                self.dispatch(Command::AnswerScripts {
                    document,
                    proceed: false,
                });
            }
            // RFC 0008 section 4.2's `app.alert` and `app.response` cannot arrive either, for the
            // arm above's reason; this window has no dialogue for a script, so it refuses by name
            // and answers as a closed dialogue does, rather than hold the script until its wait
            // runs out (ADR 1628).
            Event::ScriptAsking { document, .. } => {
                eprintln!("note: {}", viewer_host::script_asks::UNANSWERABLE);
                self.dispatch(Command::AnswerScript {
                    document,
                    answer: viewer_core::ScriptAnswer::Unanswerable,
                });
            }
            // A question is answered the moment it arrives, so none stands for a withdrawal to
            // take down; the arm says so rather than receiving a newer worker's event in silence
            // (ADR 1643).
            // Pinned to `off`, this window's worker runs no script that could ask for a sound; the
            // arm says so rather than receiving a newer worker's event in silence (ADR 1702).
            Event::Beep { document, sound } => {
                eprintln!(
                    "note: {}",
                    viewer_host::script_timers::unplayed(
                        &format!("document {}", document.0),
                        sound,
                        "this window has no sound for a document's script"
                    )
                );
            }
            // §13.2.4.2's clip, which the worker judged and this window was handed: no player is
            // started yet, said (ADR 1961).
            Event::Media {
                document,
                content_type,
                media,
            } => {
                eprintln!(
                    "note: {}",
                    viewer_host::handed_media(
                        &format!("document {}", document.0),
                        content_type.as_deref(),
                        &media
                    )
                );
            }
            // Pinned to `off` as for a sound; the arm says so rather than receiving a newer
            // worker's event in silence (ADR 1771).
            Event::Console { document, request } => {
                eprintln!(
                    "note: {}",
                    viewer_host::script_timers::console(
                        &format!("document {}", document.0),
                        request
                    )
                );
            }
            Event::ScriptQuestionWithdrawn { .. } => {
                eprintln!(
                    "note: a script's question was withdrawn, and this window had answered it \
                     already: it has no dialogue for a document's script"
                );
            }
            Event::Reported { page, notes, .. } => {
                for note in notes {
                    match page {
                        Some(page) => {
                            eprintln!("page {}: {note}", page.saturating_add(1));
                        }
                        None => eprintln!("document: {note}"),
                    }
                }
            }
        }
    }

    /// Asks the confined viewer for the frame and hands it to the screen.
    fn pull_frame(&mut self) {
        if self.stopped.is_some() {
            return;
        }
        let began = self.sent.take().unwrap_or_else(Instant::now);
        let asked = match self.confined.as_mut() {
            Some(confined) => confined.query(Query::Frame),
            None => return,
        };
        match asked {
            Ok(Reply::Frame(frames)) => {
                self.trace.say(
                    Topic::Frames,
                    format_args!(
                        "frame: {} page(s), {} as marks",
                        frames.len(),
                        frames
                            .iter()
                            .filter(|framed| matches!(framed.payload, Payload::List { .. }))
                            .count()
                    ),
                );
                // A window the worker drew, said in the toolkit windows' words for the one they
                // ask for, so a drive judges the three alike (ADR 2016).
                for framed in &frames {
                    if let Payload::Window {
                        target,
                        corner,
                        raster,
                    } = &framed.payload
                    {
                        self.trace.say(
                            Topic::Frames,
                            format_args!(
                                "page {} crossed as the window shows it, {}x{} at {},{} of {}x{}",
                                framed.page.saturating_add(1),
                                raster.width,
                                raster.height,
                                corner.0,
                                corner.1,
                                target.width,
                                target.height
                            ),
                        );
                    }
                }
                if self.screen.draws_on_the_device() {
                    let forecasts =
                        self.pricing
                            .frame(&frames, began.elapsed(), self.screen.extent());
                    self.route(&forecasts);
                }
                self.screen.take(frames, &mut self.drawing);
                self.refused_since_take = false;
                // A frame crossed, so where the reader is now is where a resume goes back to —
                // and the restart budget starts again from here, because what it bounds is a
                // recovery that is not working rather than the length of the reading. The view is
                // *asked for* rather than assembled from what this window sent: `Zoom` and
                // `Scroll` are relative and the viewer clamps them, so only the viewer knows.
                if let Some(view) = self.view() {
                    self.resuming.showing(view);
                }
                self.ask_frame();
                self.redraw();
            }
            // No document is focused — before the open, or after a failed one.
            Ok(_) => {}
            Err(problem) => self.died(&problem),
        }
    }

    /// Says each priced page's route, and asks the worker for the one that changed (ADR 2017):
    /// `quorra`'s `device chosen` and `processor chosen` lines, with the worker's window as the
    /// processor (ADR 1886).
    fn route(&mut self, forecasts: &[pricing::Forecast]) {
        for forecast in forecasts {
            let (chosen, past) = match forecast.chosen {
                viewer_confined::Route::Window => ("window", "the device's past the warning"),
                viewer_confined::Route::Marks => {
                    ("device", "the device's inside the warning or the smaller")
                }
            };
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} {chosen} chosen: device {:.1} ms with the crossing (measured), window \
                     {:.1} ms ({}); {past}",
                    forecast.page.saturating_add(1),
                    forecast.device.as_secs_f64() * 1e3,
                    forecast.window.as_secs_f64() * 1e3,
                    if forecast.window_measured {
                        "measured"
                    } else {
                        "seed"
                    },
                ),
            );
            if !forecast.changed {
                continue;
            }
            let Some(confined) = self.confined.as_mut() else {
                return;
            };
            match confined.route(forecast.page, forecast.chosen) {
                Ok(()) => self.pricing.asked(forecast.page, forecast.chosen),
                // The page keeps the route it had, which draws it; the sentence says why the
                // quicker one was not taken.
                Err(problem) => eprintln!(
                    "page {}: the worker did not take the {chosen} route: {problem}",
                    forecast.page.saturating_add(1)
                ),
            }
        }
    }

    /// Where the confined viewer says the reader is looking, or nothing if it cannot say.
    ///
    /// A refusal here is deliberately *not* a death: this question is asked beside a frame that
    /// has already crossed, so a worker that will not answer it will be found by the next command
    /// — and treating it as a death here would start a worker from inside the frame pull, which is
    /// exactly the nesting [`Host::resume`] exists to prevent.
    fn view(&mut self) -> Option<Viewing> {
        match self.confined.as_mut()?.query(Query::View) {
            Ok(Reply::View(view)) => Some(view),
            _ => None,
        }
    }

    /// The reader's abort: ends the worker and takes back the drawing thread, blocking on
    /// neither.
    ///
    /// `doc/todo/15` asked for exactly this pair — the `Canceller` is the kill the document
    /// cannot decline (ADR 0241), and the interrupt is what reaches the draw a kill does not,
    /// because on the marks arm the drawing is this side's (ADR 0650).
    fn abort(&mut self) {
        self.drawing.superseded(false);
        self.canceller.cancel();
        self.stop(format!(
            "aborted: the confined viewer for {} was ended by the reader",
            self.path.display()
        ));
    }

    /// Ends a worker that is still performing something as this window leaves.
    ///
    /// The command's thread holds the worker's handle, and a process that leaves does not run that
    /// thread's drop, which is what would otherwise have ended it: the worker would draw on into a
    /// pipe nobody reads. The kill is the abort's own (ADR 0241).
    fn end_a_busy_worker(&self) {
        if self.busy() {
            self.canceller.cancel();
        }
    }

    /// Brings up whatever will put pixels on this window (ADR 0725).
    ///
    /// The device path first, unless `--cpu` kept the driver unloaded: the instance thread is
    /// joined, the renderer built on it, the surface detached and configured (`Ungrounded`
    /// carries that argument). Every refusal on the way falls to the processor's path, out
    /// loud — the second of the two jobs `CLAUDE.md` keeps the CPU backend for.
    fn bring_up(&mut self, window: &Arc<Window>) -> Presentation {
        if let Some(instancing) = self.instancing.take() {
            let Ok(instance) = instancing.join() else {
                eprintln!(
                    "the thread creating the graphics instance panicked; drawing on the \
                     processor instead"
                );
                return self.software(window);
            };
            self.trace
                .say(Topic::Launch, format_args!("graphics instance"));
            let began = Instant::now();
            match QuorraWindowRenderer::with_instance(&instance, Arc::clone(window)) {
                Ok(renderer) => {
                    let size = window.inner_size();
                    match Device::split(renderer, (size.width.max(1), size.height.max(1))) {
                        Ok(ungrounded) => match ungrounded.ground() {
                            Ok(device) => {
                                let device = Box::new(device);
                                self.trace.say(
                                    Topic::Launch,
                                    format_args!(
                                        "graphics device in {:.1} ms, surface configured; \
                                         rendering with {}",
                                        began.elapsed().as_secs_f64() * 1e3,
                                        device.description()
                                    ),
                                );
                                self.trace.say(
                                    Topic::Launch,
                                    format_args!("bring-up parts: {:?}", device.startup()),
                                );
                                return Presentation::Device(device);
                            }
                            Err(why) => eprintln!(
                                "the graphics device could not put anything on this window \
                                 ({why}); drawing on the processor instead"
                            ),
                        },
                        Err(renderer) => eprintln!(
                            "the graphics device came up with no surface to present through \
                             ({}); drawing on the processor instead",
                            renderer.adapter_description()
                        ),
                    }
                }
                Err(problem) => eprintln!(
                    "no graphics device for this window ({problem}); drawing on the processor \
                     instead"
                ),
            }
        }
        self.software(window)
    }

    /// The processor's surface, which is the one launch failure this window cannot show past.
    fn software(&self, window: &Arc<Window>) -> Presentation {
        match SoftwareSurface::new(Arc::clone(window)) {
            Ok(surface) => {
                self.trace
                    .say(Topic::Launch, format_args!("software surface"));
                Presentation::Software(surface)
            }
            Err(problem) => {
                // Nothing can put pixels on this window; a blank window for ever is the worse
                // answer.
                eprintln!("no software surface for this window: {problem}");
                std::process::exit(1);
            }
        }
    }

    /// The card changed: the picture is the card over the screen, so a device frame carries it
    /// (`--cpu` composes at present time and needs only the redraw).
    fn card_changed(&mut self) {
        self.ask_frame();
        self.redraw();
    }

    /// One device frame came home: adopted, or refused and handed to the fallback thread.
    fn frame_landed(&mut self, landed: device::Landed) {
        if let Some(why) = landed.refused {
            let asked = self.screen.fall_back(&mut self.drawing);
            eprintln!(
                "the graphics device refused the frame ({why}); {asked} page(s) fall back to \
                 the processor"
            );
            self.refused_since_take = true;
            // **A title is one line**, and `wgpu` writes a refused pipeline as a chain of causes,
            // one per line: the title takes the first, which names what was refused, and the
            // terminal above keeps all of it (ADR 1962).
            let said = format!(
                "drawn on the processor: the graphics device refused ({})",
                why.lines().next().unwrap_or(&why)
            );
            if self.on_the_processor.as_ref() != Some(&said) {
                self.on_the_processor = Some(said);
                self.retitle();
            }
            if asked > 0 {
                // The card and the pixel pages still want a frame, now without the refused
                // marks; the fallback pages join it as each lands off the thread.
                self.ask_frame();
            }
            return;
        }
        if !self.refused_since_take && self.on_the_processor.take().is_some() {
            self.retitle();
        }
        self.pricing
            .device_drew(landed.cost.total, self.screen.device_commands());
        self.trace.say(
            Topic::Frames,
            format_args!(
                "device frame in {:.1} ms, asked-to-finished {:.1} ms",
                landed.cost.total.as_secs_f64() * 1e3,
                landed.waited.as_secs_f64() * 1e3
            ),
        );
        self.redraw();
    }

    /// The cards as display lists — §7.6.4.1's and the *ask* level's — over whatever the screen
    /// has, which for an encrypted document is the surround, exactly the flagship's arrangement of
    /// the same cards. The device and the processor both draw what this answers (ADR 2040).
    fn overlays(&self) -> Vec<pdf_render::DisplayList> {
        let (Some(window), Some(chrome)) = (self.window.as_ref(), self.chrome.as_ref()) else {
            return Vec::new();
        };
        let extent = window.inner_size();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        self.password
            .draw(chrome, extent.width, extent.height, scale)
            .into_iter()
            .chain(
                self.question
                    .draw(chrome, extent.width, extent.height, scale),
            )
            .collect()
    }

    /// Asks the device for a frame of what the screen now holds, where a device is what draws.
    ///
    /// Called wherever the picture changed — a frame pulled, the card typed into, a resize, a
    /// fallback draw landing. Under `--cpu` there is no device and presentation composes from
    /// the screen directly, so this is nothing.
    fn ask_frame(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        self.screen.resize(extent.width, extent.height);
        let overlays = self.overlays();
        if let Some(Presentation::Device(device)) = self.presentation.as_mut() {
            device.ask(
                self.screen.device_pages(extent.width, extent.height),
                overlays,
            );
        }
    }

    /// Puts what the screen has onto the window, under the presentation gate.
    fn present(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        self.screen.resize(extent.width, extent.height);
        match self.presentation.as_mut() {
            Some(Presentation::Device(device)) => match device.present() {
                Presented::Shown => self.shown(extent),
                // Before the first frame lands the grounded surround stays up, which is the
                // window's honest picture of having nothing yet.
                Presented::Nothing | Presented::Waited => {}
                Presented::AskAgain => self.redraw(),
                Presented::Refused(why) => {
                    eprintln!("the frame could not be presented: {why}");
                }
            },
            Some(Presentation::Software(_)) => self.present_software(),
            None => return,
        }
        for (page, words) in self.screen.refusals() {
            eprintln!(
                "page {} could not be drawn: {words}",
                page.saturating_add(1)
            );
        }
        // The reader has their page, so the document may now be asked what it says about itself
        // — §12.8's signatures above all, whose answer reads and digests the signed part of the
        // file behind the filter. Once per opened document; `viewer_host::report::Due` is the
        // rule all four hosts follow. ADR 1044.
        if self.presented.is_some() && self.report_due.after_a_frame() {
            self.dispatch(Command::Report);
            // And its open sequence, which runs in the worker beside the scripts' view, after the
            // first present and never in front of it (ADR 1602).
            self.dispatch(Command::Presented);
        }
    }

    /// The processor's present: the screen composed, the card drawn over it.
    fn present_software(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        let resized = self.presented != Some((extent.width, extent.height));
        if !(self.screen.settled() || self.presented.is_none() || resized) {
            // Something newer is still on the drawing thread and the window already shows a
            // whole frame of the right size: keep it (doc/todo/37's rule) rather than blink
            // pages in and out.
            return;
        }
        let Some(composed) = self.screen.compose() else {
            return;
        };
        let cards = self.overlays();
        let Some(Presentation::Software(surface)) = self.presentation.as_mut() else {
            return;
        };
        let overlays: Vec<&pdf_render::DisplayList> = cards.iter().collect();
        match surface.present(&composed, &overlays) {
            Ok(()) => self.shown(extent),
            Err(problem) => eprintln!("the frame could not be presented: {problem}"),
        }
    }

    /// A frame reached the window. It is the *first* frame only once a page is in it: the window
    /// presents the bare surround while its worker draws (ADR 1827), and the launch stamp, the
    /// presentation gate and the document's report all mean the reader's page.
    fn shown(&mut self, extent: winit::dpi::PhysicalSize<u32>) {
        if !self.screen.shows_a_page() {
            return;
        }
        if self.presented.is_none() {
            self.trace
                .say(Topic::Launch, format_args!("first frame presented"));
        }
        self.presented = Some((extent.width, extent.height));
    }

    fn redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn retitle(&self) {
        if let Some(window) = self.window.as_ref() {
            let drawn = self
                .on_the_processor
                .as_ref()
                .map_or_else(String::new, |said| format!(" — {said}"));
            let loading = self
                .loading
                .as_deref()
                .or_else(|| self.waiting_said())
                .map_or_else(String::new, |said| format!(" — {said}"));
            let title = match &self.stopped {
                Some(said) => format!("{} — {said}{drawn} — confined", self.heading),
                None => format!("{}{loading}{drawn} — confined", self.heading),
            };
            window.set_title(&title);
        }
    }

    /// A key press while §7.6.4.1's card has the keyboard.
    ///
    /// Every key is taken, which is what *modal* means here (the flagship's `password_key`, whose
    /// rule this copies): the document behind the card is **not open**, so a key that turned a
    /// page would be turning a page that does not exist — and Escape is the prompt's decline
    /// rather than [`Self::abort`], because what the person is declining is the question, not the
    /// worker. Escape and an empty Enter reach [`viewer_host::password::supplied`] by the same
    /// route, and that one place decides what a decline means.
    fn password_key(&mut self, key: &Key<&str>) {
        match key {
            Key::Named(NamedKey::Escape) => {
                self.password.clear();
                self.password_answered();
            }
            Key::Named(NamedKey::Enter) => self.password_answered(),
            Key::Named(NamedKey::Backspace) => {
                self.password.backspace();
                self.card_changed();
            }
            Key::Named(NamedKey::Space) => {
                self.password.typed(" ");
                self.card_changed();
            }
            Key::Character(text) if !text.is_empty() => {
                self.password.typed(text);
                self.card_changed();
            }
            // A key with no character and no meaning here. Taken anyway, for the reason above.
            _ => {}
        }
    }

    /// The card was answered: open again with what was typed, or say why nothing opened.
    ///
    /// The [`viewer_core::Secret`] moves from the card into [`Command::Open`] and is dropped
    /// with it — no copy of it stays on this side, and the trace's [`brief`] never prints one.
    /// The file is opened again rather than kept: rule 2 makes the filesystem this side's, and a
    /// file that has gone away between the attempts is a fact about this machine, said out loud.
    fn password_answered(&mut self) {
        let typed = self.password.take();
        self.card_changed();
        let secret = match viewer_host::password::supplied(typed) {
            Supplied::Open(secret) => secret,
            Supplied::Cancelled => {
                eprintln!("note: {}", viewer_host::password::CANCELLED);
                if self.decline_the_prompted() {
                    return;
                }
                self.heading = format!("{} — {}", self.name(), viewer_host::password::CANCELLED);
                self.retitle();
                return;
            }
        };
        let bytes = match pdf_syntax::FileBytes::on_disk(self.prompted_for()) {
            Ok(bytes) => bytes,
            Err(problem) => {
                eprintln!(
                    "note: cannot reopen {}: {problem}",
                    self.prompted_for().display()
                );
                if self.decline_the_prompted() {
                    return;
                }
                self.heading = format!("{} — cannot reopen the file", self.name());
                self.retitle();
                return;
            }
        };
        self.offer_the_prompted();
        // The worker survives an open it could not finish (ADR 0597), so the retry goes to the
        // same process: the document's descriptor crosses once more, which since ADR 0812 costs
        // an `open` and a `sendmsg` rather than the file's whole length down a pipe, however
        // many of `ATTEMPTS` it is paid.
        self.dispatch(Command::Open {
            id: DOCUMENT,
            bytes,
            password: Some(secret),
            fragment: None,
        });
    }

    /// A press or a release on the page, as `Command::Pointer` (ADR 1987).
    ///
    /// Held behind a busy worker as any command is, so the two ends arrive in order. **After the
    /// release the keyboard follows the focus**, as after a Tab: a press inside a text field is
    /// how a person says "type here", and the worker's viewer has moved §12.6.3's focus there.
    fn click(&mut self, state: ElementState) {
        let action = match state {
            ElementState::Pressed => viewer_core::PointerAction::Pressed,
            ElementState::Released => viewer_core::PointerAction::Released,
        };
        self.dispatch(Command::Pointer {
            at: self.cursor,
            action,
        });
        if action == viewer_core::PointerAction::Released {
            if self.must_hold() {
                self.hold(Held::Focus);
            } else {
                self.ask_the_focus();
            }
        }
    }

    /// One key, one meaning; anything else is nothing.
    fn key(&mut self, event: &KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        self.press(&event.logical_key);
    }

    /// A pressed key: held while the worker is busy, unless it is one about the wait itself
    /// (ADR 1827), and otherwise what it means now.
    fn press(&mut self, key: &Key) {
        if self.must_hold() && !self.acts_at_once(key) {
            self.hold(Held::Key {
                key: key.clone(),
                modifiers: self.modifiers,
                at: Instant::now(),
            });
            return;
        }
        if self.password.shown {
            self.password_key(&key.as_ref());
            return;
        }
        if self.question.shown {
            self.question_key(&key.as_ref());
            return;
        }
        if let typing::Typed::Taken = self.field_key(&key.as_ref()) {
            return;
        }
        match key {
            Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space) => {
                self.dispatch(Command::GoTo(PageTarget::Next));
            }
            Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => {
                self.dispatch(Command::GoTo(PageTarget::Previous));
            }
            Key::Named(NamedKey::ArrowDown) => self.dispatch(Command::Scroll {
                dx: 0.0,
                dy: WHEEL_NOTCH,
            }),
            Key::Named(NamedKey::ArrowUp) => self.dispatch(Command::Scroll {
                dx: 0.0,
                dy: -WHEEL_NOTCH,
            }),
            Key::Named(NamedKey::Home) => self.dispatch(Command::GoTo(PageTarget::First)),
            Key::Named(NamedKey::End) => self.dispatch(Command::GoTo(PageTarget::Last)),
            Key::Named(NamedKey::Escape) => self.abort(),
            Key::Character(text) => match text.as_str() {
                "+" | "=" => self.dispatch(Command::Zoom {
                    zoom: Zoom::In,
                    at: None,
                }),
                "-" => self.dispatch(Command::Zoom {
                    zoom: Zoom::Out,
                    at: None,
                }),
                "q" => self.leaving = true,
                // The archive key, said rather than ignored: `Command::Archive` does not cross
                // this window's pipe (ADR 1954).
                "A" => eprintln!("note: {}", viewer_host::archiving::NOT_IN_A_CONFINED_WINDOW),
                "m" => {
                    let on = self.measuring.toggle();
                    eprintln!("note: {}", viewer_host::measuring::switched(on));
                }
                _ => {}
            },
            _ => {}
        }
    }
}

impl Host {
    /// Takes in the worker and the document the opening thread started, and tells it the
    /// window's size as it is now.
    fn take_the_opening(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        if self.loading.take().is_some() {
            println!("note: {}", viewer_host::loading::READ);
        }
        // A reader who pressed Escape while the bar was up has ended this worker already, and the
        // window says so; what the thread then answers is the cancel it was told.
        if self.stopped.is_some() {
            drop(self.opening.take().map(viewer_host::Loading::join));
            self.retitle();
            return;
        }
        let opened = match self.opening.take().map(viewer_host::Loading::join) {
            Some(Ok(Ok(opened))) => opened,
            Some(Ok(Err(problem))) => {
                self.stop(problem);
                return;
            }
            Some(Err(_)) => {
                self.stop("the thread opening the document panicked".to_owned());
                return;
            }
            None => {
                self.stop("no document was opened for this window".to_owned());
                return;
            }
        };
        self.trace
            .say(Topic::Launch, format_args!("document joined"));
        // A kernel can refuse what a build offers, and a person relying on the sandbox is owed
        // the difference out loud (`Confined::confinement` is a report, not a promise).
        if let Some(short) = opened.confined.confinement().shortfall() {
            eprintln!("confinement shortfall: {short}");
        }
        self.confined = Some(opened.confined);
        // The size sent below is the window's now, so a size taken while the open was out is owed
        // to nobody; the keys taken then go after what the open itself caused (ADR 1827).
        self.held.retain(|held| {
            !matches!(
                held,
                Held::Command {
                    command: Command::Resize { .. },
                    ..
                }
            )
        });
        self.ahead_of_the_held(|host| {
            for event in opened.events {
                host.event(event);
            }
            host.dispatch(Command::Resize {
                width: extent.width,
                height: extent.height,
                scale,
            });
        });
        // A window with nothing on the screen yet waits for page one instead of polling for it,
        // out of the launch's one-refresh budget (ADR 0678) — same rule, same numbers, third
        // window. Behind a device that wait is for the first frame the render thread finishes,
        // asked for by the frame pull inside the open's dispatch; under `--cpu`, and for a
        // device-refused fallback, it is the drawing thread's.
        if let Some(Presentation::Device(device)) = self.presentation.as_mut()
            && let Some(landed) = device.settle(viewer_host::drawing::SETTLE)
        {
            self.frame_landed(landed);
        }
        for finished in self.drawing.settle(viewer_host::drawing::SETTLE) {
            self.screen.landed(finished);
        }
        self.redraw();
    }

    /// Puts the worker's figure for the page in the title, and says so once on the way in.
    fn mind_the_opening(&mut self) {
        let Some(reading) = self
            .opening
            .as_ref()
            .and_then(viewer_host::Loading::showing)
        else {
            return;
        };
        let sentence = viewer_host::loading::reading(reading);
        let said = match viewer_host::loading::bar(reading, 20) {
            Some(bar) => format!("{bar} {sentence}"),
            None => sentence.clone(),
        };
        if self.loading.as_deref() == Some(said.as_str()) {
            return;
        }
        if self.loading.is_none() {
            println!("note: {sentence}");
        }
        self.loading = Some(said);
        self.retitle();
    }
}

impl ApplicationHandler for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(format!("{} — confined", self.heading))
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 1000.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("window creation"),
        );
        self.trace.say(Topic::Launch, format_args!("window up"));
        self.presentation = Some(self.bring_up(&window));
        if matches!(self.presentation, Some(Presentation::Software(_)))
            && self.screen.draws_on_the_device()
        {
            // The device did not come up, so the screen must not hold pages for one: nothing
            // has crossed yet — the worker has no viewport until the `Resize` below — so the
            // replacement costs nothing.
            self.screen = Screen::new();
        }
        self.window = Some(window);

        // The worker and the document, from the thread `anticipate` started before the window:
        // the worker starts when there is a document for it, which is at launch (ADR 1539). Waited
        // for no longer than `WARN`: a page still being read in the worker then is a bar in the
        // title, the window answers its loop — Escape included, which ends the worker — and
        // `about_to_wait` takes the document in on the tick that finds it ready (ADR 1811).
        if self.opening.as_ref().is_some_and(|opening| !opening.wait()) {
            self.trace
                .say(Topic::Launch, format_args!("document still opening"));
            self.mind_the_opening();
            return;
        }
        self.take_the_opening();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                // The drawing thread is joined on the way out; a hostile page still on it would
                // hold the join for as long as it liked, so it is taken back first.
                self.drawing.superseded(false);
                self.end_a_busy_worker();
                event_loop.exit();
            }
            WindowEvent::Resized(extent) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a display's scale factor is a small ratio"
                )]
                let scale = self
                    .window
                    .as_ref()
                    .map_or(1.0, |window| window.scale_factor() as f32);
                if let Some(Presentation::Device(device)) = self.presentation.as_mut() {
                    device.resize(extent.width, extent.height);
                }
                self.dispatch(Command::Resize {
                    width: extent.width,
                    height: extent.height,
                    scale,
                });
                // The frame pull inside the dispatch asked for the resized frame already; this
                // covers a resize arriving before any document frame exists — the card alone.
                self.ask_frame();
                self.redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => self.key(&event),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a pointer position in device pixels, which is thousands"
            )]
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } if self.measuring.is_on() => {
                if self.must_hold() {
                    self.hold(Held::Point {
                        cursor: self.cursor,
                        at: Instant::now(),
                    });
                } else {
                    self.measure_at();
                }
            }
            // §12.5.1's activation: both ends of the press go to the worker, whose viewer finds
            // the annotation under them and performs its actions — a link, a field, §12.6.4.14's
            // rendition — and answers in the events every other command's arrive in (ADR 1987).
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } if !self.password.shown && !self.question.shown && !self.measuring.is_on() => {
                self.click(state);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (-x * WHEEL_NOTCH, -y * WHEEL_NOTCH),
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a scroll delta in device pixels, which is tens"
                    )]
                    MouseScrollDelta::PixelDelta(position) => {
                        (-position.x as f32, -position.y as f32)
                    }
                };
                self.dispatch(Command::Scroll { dx, dy });
            }
            WindowEvent::RedrawRequested => self.present(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.leaving {
            // The drawing thread is joined on the way out; a hostile page still on it would hold
            // the join for as long as it liked, so it is taken back first.
            self.drawing.superseded(false);
            self.end_a_busy_worker();
            event_loop.exit();
            return;
        }
        // A document still opening past `WARN`: its bar, until the tick that finds it ready
        // (ADR 1811). Nothing below has a worker to talk to yet.
        if self.window.is_some() && self.opening.is_some() {
            if self
                .opening
                .as_ref()
                .is_some_and(viewer_host::Loading::ended)
            {
                self.take_the_opening();
            } else {
                self.mind_the_opening();
                let at = Instant::now()
                    .checked_add(viewer_host::loading::TICK)
                    .unwrap_or_else(Instant::now);
                event_loop.set_control_flow(ControlFlow::WaitUntil(at));
                return;
            }
        }
        // A command still out past `WARN`: its sentence, or its answer if it has come (ADR 1827).
        self.mind_the_exchange();
        // The one place a worker is started again, and it is here rather than where the death
        // was seen so that no restart runs inside another one ([`Host::resume`]).
        if let Some(reopen) = self.resume.take() {
            self.reopen(reopen);
        }
        // A server's answer, handed to the worker — held behind a busy one like any command.
        self.take_the_answers();
        // What the reader did while the worker was busy, now that it is not.
        if !self.busy() && !self.held.is_empty() {
            self.release_held();
        }
        let mut changed = false;
        // A page the processor drew whole, and a window of one it draws as the window shows it
        // (ADRs 1944, 2026), each said in the toolkit windows' words, so a drive tells the two
        // apart as it does theirs; a window's line ends with the page it is of, which is what says
        // whether the page was under the bound.
        for finished in self.drawing.collect() {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} {} {}x{} in {:?}, waited {:?}",
                    finished.request.page.saturating_add(1),
                    if finished.outcome.is_some() {
                        "rasterised"
                    } else {
                        "abandoned after"
                    },
                    finished.request.target.width,
                    finished.request.target.height,
                    finished.cost,
                    finished.waited
                ),
            );
            changed |= self.screen.landed(finished);
        }
        for (finished, window) in self.drawing.windows() {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} window {}x{} at {},{} {} in {:?}, waited {:?}, of {}x{}",
                    finished.request.page.saturating_add(1),
                    window.width,
                    window.height,
                    window.x,
                    window.y,
                    if finished.outcome.is_some() {
                        "rasterised"
                    } else {
                        "abandoned"
                    },
                    finished.cost,
                    finished.waited,
                    finished.request.target.width,
                    finished.request.target.height
                ),
            );
            changed |= self.screen.landed_window(finished, window);
        }
        if changed {
            // Fallback pixels landed: behind a device they reach the window inside the next
            // device frame, so one is asked for; under `--cpu` the redraw composes them.
            self.ask_frame();
            self.redraw();
        }
        if let Some(Presentation::Device(device)) = self.presentation.as_mut()
            && let Some(landed) = device.collect()
        {
            self.frame_landed(landed);
        }
        // The password card's chrome may have drawn a box for a character whose machine face was
        // still being searched for (ADR 1406); this window has no waker, so it looks on the same
        // poll a drawing thread gets and draws the card again when the face is in.
        if self.chrome.as_ref().is_some_and(Chrome::take_arrivals) {
            self.redraw();
        }
        match self
            .next_look()
            .and_then(|after| Instant::now().checked_add(after))
        {
            Some(at) => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}

impl Host {
    /// How long the loop may sleep before something is owed a look, or `None` where nothing is:
    /// a drawing, a face being searched for or a device frame at `viewer_host::drawing::POLL`, a
    /// command still out at its own tick, and a request on the wire at `viewer_host::submit::LOOK`
    /// (`sending`).
    fn next_look(&self) -> Option<Duration> {
        let waiting = self.drawing.interval().is_some()
            || self.chrome.as_ref().is_some_and(Chrome::searching)
            || matches!(
                self.presentation.as_ref(),
                Some(Presentation::Device(device)) if device.busy()
            );
        let next = if waiting {
            Some(viewer_host::drawing::POLL)
        } else {
            self.exchange_tick()
        };
        match (next, self.submitter.interval()) {
            (Some(soon), Some(look)) => Some(soon.min(look)),
            (soon, look) => soon.or(look),
        }
    }
}

fn main() {
    let began = Instant::now();
    let Arguments {
        path,
        topics,
        processor,
        faces,
        reader,
        links,
        remote_documents,
        submissions,
        launched_documents,
    } = arguments();
    let mut host = Host::new(path, topics, processor, faces, began);
    host.reader = reader;
    host.links = links;
    host.remote_documents = remote_documents;
    host.submissions = submissions;
    host.launched_documents = launched_documents;
    host.anticipate();
    let event_loop = EventLoop::new().expect("an event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut host).expect("the event loop runs");
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;

    use viewer_confined::ConfinedError;
    use viewer_core::{DocumentId, Event, Viewing, Zoom};
    use winit::keyboard::{Key, NamedKey};

    use super::Host;

    /// A host with no window, no surface and no worker: everything the password path touches
    /// short of a pipe, which is exactly the part this file added.
    fn a_host() -> Host {
        // `--cpu`'s shape: a host with no instance thread, which is what a test wants — the
        // password path under test touches no surface either way.
        Host::new(PathBuf::from("locked.pdf"), 0, true, false, Instant::now())
    }

    /// §7.6.4.1's event puts the card up instead of ending the document.
    ///
    /// The behaviour this replaces was a `stop`: before ADR 0718 the window declined every
    /// encrypted document by name, so the discriminating assertion is `stopped.is_none()`.
    #[test]
    fn an_encrypted_document_is_prompted_for_not_refused() {
        let mut host = a_host();
        host.event(Event::PasswordRequired {
            document: DocumentId(0),
        });
        assert!(host.password.shown, "the card is up");
        assert!(host.stopped.is_none(), "the document was not given up on");
    }

    /// The attempts exhaust into a sentence and an open window — `viewer_host::password`'s rule
    /// that no host may make `Ask::Exhausted` mean the window closes.
    #[test]
    fn exhausted_attempts_leave_the_window_open() {
        let mut host = a_host();
        for _ in 0..=viewer_host::password::ATTEMPTS {
            host.event(Event::PasswordRequired {
                document: DocumentId(0),
            });
            // A new `PasswordRequired` only ever follows an answered card (the retry's failed
            // open), so between events the card is answered — what was typed is not the point.
            if host.password.shown {
                let _ = host.password.take();
            }
        }
        assert!(!host.password.shown, "it stopped asking");
        assert!(host.stopped.is_none(), "and the window is still a window");
        assert!(
            host.heading.contains(viewer_host::password::EXHAUSTED),
            "the sentence is where a person will read it, was {:?}",
            host.heading
        );
    }

    /// Escape while the card is up declines the prompt; it is not [`Host::abort`].
    ///
    /// The distinction is the whole reason `password_key` runs before the page keys: an Escape
    /// that killed the worker would turn "I don't know the password" into "end the program's
    /// document", two different facts about the reader.
    #[test]
    fn escape_declines_the_prompt_without_aborting() {
        let mut host = a_host();
        host.event(Event::PasswordRequired {
            document: DocumentId(0),
        });
        host.password_key(&Key::Named(NamedKey::Escape));
        assert!(!host.password.shown, "the card came down");
        assert!(host.stopped.is_none(), "nothing was aborted");
        assert!(
            host.heading.contains(viewer_host::password::CANCELLED),
            "the decline is said, was {:?}",
            host.heading
        );
    }

    /// A file that has gone away between the first open and the retry is a fact about this
    /// machine: said in the heading, and the window survives it (trap 5, the flagship's rule).
    #[test]
    fn a_file_gone_before_the_retry_is_said_not_fatal() {
        let mut host = Host::new(
            PathBuf::from("tests/does-not-exist-781.pdf"),
            0,
            true,
            false,
            Instant::now(),
        );
        host.event(Event::PasswordRequired {
            document: DocumentId(0),
        });
        host.password_key(&Key::Character("x"));
        host.password_key(&Key::Named(NamedKey::Enter));
        assert!(!host.password.shown, "the card came down");
        assert!(host.stopped.is_none(), "the window is still a window");
        assert!(
            host.heading.contains("cannot reopen"),
            "the missing file is said, was {:?}",
            host.heading
        );
    }

    /// A worker that died leaves the window open with a restart owed, and the restart goes back
    /// to the whole view the reader was looking at.
    ///
    /// The behaviour this replaces is a `stop`: from ADR 0713 until ADR 0734 every death ended the
    /// document, so the discriminating assertions are `stopped.is_none()` and the view. **The
    /// magnification and the offset are in it since ADR 0737**, and they are what the assertion is
    /// really about: until then a restart went back to the page and to the document's opening
    /// view, which is the sentence this window used to print.
    #[test]
    fn a_dead_worker_leaves_a_restart_owed_at_the_readers_view() {
        let mut host = a_host();
        let was = Viewing {
            page: 4,
            zoom: Zoom::Scale(2.75),
            scroll: (13.0, 907.5),
        };
        host.event(Event::PageChanged {
            document: DocumentId(0),
            index: 4,
            label: None,
            of: 9,
            section: None,
        });
        host.resuming.showing(was);
        host.died(&ConfinedError::WorkerDied {
            detail: "killed by signal 6".to_owned(),
        });
        assert!(host.stopped.is_none(), "the document was not given up on");
        assert_eq!(
            host.resume.map(|reopen| reopen.view),
            Some(was),
            "the restart goes back to where the reader was, magnification and offset included"
        );
        assert!(
            host.heading.contains("restarting"),
            "and the window says so, was {:?}",
            host.heading
        );
    }

    /// A directory of its own for one test, holding the document and the file it names.
    fn beside(test: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("quorra-confined-{}-{test}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        std::fs::write(directory.join("other.pdf"), b"%PDF-2.0\n").expect("the named file");
        directory
    }

    /// §12.6.4.3's question for a file beside the document, as the worker raises it.
    fn needs_other() -> Event {
        Event::NeedsFile {
            document: DocumentId(0),
            purpose: viewer_core::Purpose::RemoteDocument,
            name: "other.pdf".to_owned(),
            beside: false,
        }
    }

    /// The *ask* level's question reaches this window as the toolkits' does: a link puts the card
    /// up and Escape takes it down with nothing handed over (ADR 2040). Before it the window was
    /// pinned to `refuse`, so the discriminating assertion is that the card was shown at all.
    #[test]
    fn a_link_at_ask_puts_the_question_and_escape_declines_it() {
        let mut host = a_host();
        host.event(Event::OpenUri {
            document: DocumentId(0),
            uri: "https://example.org/linked".to_owned(),
        });
        assert!(host.question.shown, "the question is up");
        host.press(&Key::Named(NamedKey::Escape));
        assert!(!host.question.shown, "the card came down");
        assert!(host.asked.is_none(), "and nothing is still asked");
        assert!(
            host.stopped.is_none(),
            "Escape on the card is not the abort"
        );
    }

    /// At `refuse` nothing is asked: the level is the reader's, and the card is the *ask* level's.
    #[test]
    fn a_link_at_refuse_is_not_asked() {
        let mut host = a_host();
        host.links = viewer_host::Links::Refuse;
        host.event(Event::OpenUri {
            document: DocumentId(0),
            uri: "https://example.org/linked".to_owned(),
        });
        assert!(!host.question.shown, "no question at refuse");
    }

    /// A file a document named is asked about, read on a `yes`, and becomes the window's document
    /// when the worker says it opened — the title, the save and a resume all follow it.
    #[test]
    fn a_remote_file_answered_yes_becomes_the_windows_document() {
        let directory = beside("yes");
        let mut host = a_host();
        host.path = directory.join("document.pdf");
        host.event(needs_other());
        assert!(host.question.shown, "the question is up");
        host.press(&Key::Named(NamedKey::Enter));
        assert!(!host.question.shown, "the card came down");
        assert!(host.offered.is_some(), "the file was read and offered");
        host.event(Event::Opened {
            document: DocumentId(0),
            pages: 1,
        });
        assert_eq!(host.path, directory.join("other.pdf"));
        assert!(
            host.heading.starts_with("other.pdf"),
            "was {:?}",
            host.heading
        );
        std::fs::remove_dir_all(&directory).expect("the scratch directory goes");
    }

    /// An offer nothing took is spent once its command is answered: a later open of the window's
    /// own document is not mistaken for it.
    #[test]
    fn an_offer_nothing_took_is_spent() {
        let directory = beside("spent");
        let mut host = a_host();
        host.path = directory.join("document.pdf");
        host.remote_documents = viewer_host::RemoteDocuments::Open;
        host.event(needs_other());
        assert!(host.offered.is_some(), "read and offered at open, unasked");
        host.supplied();
        host.event(Event::Opened {
            document: DocumentId(0),
            pages: 1,
        });
        assert_eq!(host.path, directory.join("document.pdf"));
        std::fs::remove_dir_all(&directory).expect("the scratch directory goes");
    }

    /// A file a document named that asks for §7.6.4.1's password is the file the card is about,
    /// and a cancelled prompt declines it rather than the window's own document.
    #[test]
    fn a_locked_remote_file_is_the_prompts_and_a_cancel_declines_it() {
        let directory = beside("locked");
        let mut host = a_host();
        host.path = directory.join("document.pdf");
        host.remote_documents = viewer_host::RemoteDocuments::Open;
        host.event(needs_other());
        host.event(Event::PasswordRequired {
            document: DocumentId(0),
        });
        assert!(host.password.shown, "the prompt is up");
        assert_eq!(host.prompted_for(), directory.join("other.pdf"));
        host.password_key(&Key::Named(NamedKey::Escape));
        assert!(host.prompting.is_none(), "the hold was declined");
        assert_eq!(
            host.path,
            directory.join("document.pdf"),
            "the window's own stays"
        );
        assert!(
            !host.heading.contains(viewer_host::password::CANCELLED),
            "the window's own document was not the one cancelled, was {:?}",
            host.heading
        );
        std::fs::remove_dir_all(&directory).expect("the scratch directory goes");
    }

    /// One question stands at a time: a second that arrives while one is up is declined, and the
    /// first is still the one the card holds.
    #[test]
    fn a_second_question_is_declined_while_one_is_up() {
        let directory = beside("second");
        let mut host = a_host();
        host.path = directory.join("document.pdf");
        host.event(Event::OpenUri {
            document: DocumentId(0),
            uri: "https://example.org/linked".to_owned(),
        });
        host.event(needs_other());
        assert!(
            matches!(host.asked, Some(super::asking::Asked::Link { .. })),
            "the link's question is still the one up"
        );
        std::fs::remove_dir_all(&directory).expect("the scratch directory goes");
    }

    /// §12.7.6.2's request as the worker raises it, to `url`.
    fn submits_to(url: &str) -> Event {
        Event::Submit {
            document: DocumentId(0),
            submission: Box::new(pdf_model::submission::Submission {
                url: url.to_owned(),
                method: pdf_model::submission::Method::Post,
                format: pdf_model::submission::Format::Fdf,
                media_type: "application/vnd.fdf".to_owned(),
                body: b"%FDF-1.2\n".to_vec(),
                fields: 1,
                owed: Vec::new(),
            }),
        }
    }

    /// A listener on this machine that answers one request with §12.7.8.1's media type, and hands
    /// back the request's first line — the transmission under test goes nowhere else.
    fn one_answer() -> (String, std::thread::JoinHandle<String>) {
        use std::io::{BufRead as _, Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("its address").port();
        let served = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the window's request");
            let mut reading = std::io::BufReader::new(stream.try_clone().expect("a second handle"));
            let mut first = String::new();
            reading.read_line(&mut first).expect("the request line");
            let mut length = 0usize;
            loop {
                let mut header = String::new();
                reading.read_line(&mut header).expect("a header");
                if header.trim().is_empty() {
                    break;
                }
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().expect("a length");
                }
            }
            let mut body = vec![0; length];
            reading.read_exact(&mut body).expect("the body");
            let answer = b"%FDF-1.2\n";
            let mut stream = stream;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.fdf\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n",
                answer.len()
            )
            .expect("the answer's head");
            stream.write_all(answer).expect("the answer's body");
            first
        });
        (format!("http://127.0.0.1:{port}/submit"), served)
    }

    /// The answers that come back within ten seconds, collected as the loop collects them.
    fn answers_of(host: &mut Host) -> usize {
        let began = Instant::now();
        let mut arrived = 0usize;
        while host.submitter.interval().is_some() && began.elapsed().as_secs() < 10 {
            std::thread::sleep(viewer_host::submit::LOOK);
            arrived = arrived.saturating_add(host.submitter.collect().len());
        }
        arrived
    }

    /// At the default level, `ask`, a form goes on the card, and Enter sends it from this side:
    /// the listener reads a POST and the answer comes back to be handed to the worker (ADR 2076).
    /// Before it the window was pinned to `refuse`, so the discriminating assertions are the card
    /// and the request the listener read.
    #[test]
    fn a_form_at_ask_is_put_on_the_card_and_sent_on_a_yes() {
        let (url, served) = one_answer();
        let mut host = a_host();
        host.event(submits_to(&url));
        assert!(host.question.shown, "the question is up");
        assert!(
            host.submitter.interval().is_none(),
            "nothing is sent while it stands"
        );
        host.press(&Key::Named(NamedKey::Enter));
        assert!(!host.question.shown, "the card came down");
        assert!(host.submitter.interval().is_some(), "the request is out");
        assert_eq!(answers_of(&mut host), 1, "and its answer came back");
        let first = served.join().expect("the listener answered");
        assert!(first.starts_with("POST /submit "), "was {first:?}");
    }

    /// Escape on the card sends nothing.
    #[test]
    fn a_form_answered_no_is_not_sent() {
        let mut host = a_host();
        host.event(submits_to("http://127.0.0.1:9/submit"));
        assert!(host.question.shown, "the question is up");
        host.press(&Key::Named(NamedKey::Escape));
        assert!(!host.question.shown, "the card came down");
        assert!(host.submitter.interval().is_none(), "nothing went out");
    }

    /// At `refuse` nothing is asked and nothing is sent; at `send` nothing is asked and the
    /// request goes out — the two levels that need no card, the first of them the control.
    #[test]
    fn a_form_at_refuse_is_not_sent_and_at_send_is_not_asked() {
        let mut host = a_host();
        host.submissions = viewer_host::Submissions::Refuse;
        host.event(submits_to("http://127.0.0.1:9/submit"));
        assert!(!host.question.shown, "no question at refuse");
        assert!(host.submitter.interval().is_none(), "and nothing sent");

        let (url, served) = one_answer();
        host.submissions = viewer_host::Submissions::Send;
        host.event(submits_to(&url));
        assert!(!host.question.shown, "no question at send");
        assert_eq!(answers_of(&mut host), 1, "sent, and answered");
        assert!(
            served
                .join()
                .expect("answered")
                .starts_with("POST /submit ")
        );
    }

    /// An import named by an absolute URI is asked at the submissions level and fetched on a
    /// `yes`, as in the other three windows (ADRs 1527, 2076).
    #[test]
    fn an_import_named_by_a_uri_is_asked_and_fetched_on_a_yes() {
        let (url, served) = one_answer();
        let mut host = a_host();
        host.event(Event::NeedsFile {
            document: DocumentId(0),
            purpose: viewer_core::Purpose::ImportData,
            name: url.clone(),
            beside: false,
        });
        assert!(
            matches!(&host.asked, Some(super::asking::Asked::Fetch { url: asked, .. }) if *asked == url),
            "the fetch is the question up"
        );
        host.press(&Key::Named(NamedKey::Enter));
        assert_eq!(answers_of(&mut host), 1, "fetched, and answered");
        assert!(served.join().expect("answered").starts_with("GET /submit "));
    }

    /// A refusal that leaves the worker alive is not a death: nothing is restarted and the window
    /// reports it, which is the old behaviour and must stay the old behaviour.
    #[test]
    fn a_refusal_the_worker_survived_is_not_restarted_from() {
        let mut host = a_host();
        host.died(&ConfinedError::Refused {
            detail: "a raster in a second pixel layout".to_owned(),
        });
        assert!(host.resume.is_none(), "nothing to start again");
        assert!(host.stopped.is_some(), "and it is reported");
    }

    /// The reader's own abort is never restarted from: a `Cancelled` viewer is one somebody
    /// ended on purpose (ADR 0241), and a second worker would undo the key press.
    #[test]
    fn the_readers_abort_is_not_undone_by_a_restart() {
        let mut host = a_host();
        host.died(&ConfinedError::Cancelled);
        assert!(host.resume.is_none(), "Escape meant Escape");
        assert!(host.stopped.is_some());
    }

    /// A restart cannot fire once the window has stopped for another reason — the reader aborted
    /// between the death and the loop's next turn, which is a race a deferred resume can lose.
    #[test]
    fn an_abort_between_the_death_and_the_restart_wins() {
        let mut host = a_host();
        host.died(&ConfinedError::WorkerDied {
            detail: "killed by signal 6".to_owned(),
        });
        let owed = host.resume.take().expect("a restart is owed");
        host.abort();
        host.reopen(owed);
        assert!(
            host.confined.is_none(),
            "no worker was started for a window the reader had ended"
        );
        assert!(
            host.stopped
                .as_deref()
                .is_some_and(|said| said.contains("aborted")),
            "and the abort is still what the window says, was {:?}",
            host.stopped
        );
    }
}
