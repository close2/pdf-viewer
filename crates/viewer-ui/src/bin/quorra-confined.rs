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
//! scrolls, `q` quits, `r` opens the launched-document permissions menu, Tab walks a form's fields, `m` measures. **Escape is the abort**: it ends the worker with a kill it cannot decline
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
//! not until [`viewer_core::Query::View`] existed: this window asks that question per frame and hands the
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

#[path = "quorra-confined/arguments.rs"]
mod arguments;
#[path = "quorra-confined/asking.rs"]
mod asking;
#[path = "quorra-confined/device.rs"]
mod device;
#[path = "quorra-confined/events.rs"]
mod events;
#[path = "quorra-confined/exchange.rs"]
mod exchange;
#[path = "quorra-confined/frames.rs"]
mod frames;
#[path = "quorra-confined/input.rs"]
mod input;
#[path = "quorra-confined/permissions.rs"]
mod permissions;
#[path = "quorra-confined/pricing.rs"]
mod pricing;
#[path = "quorra-confined/screen.rs"]
mod screen;
#[path = "quorra-confined/sending.rs"]
mod sending;
#[path = "quorra-confined/typing.rs"]
mod typing;
#[path = "quorra-confined/window.rs"]
mod window;
#[path = "quorra-confined/worker.rs"]
mod worker;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use render_raster::QuorraWindowRenderer;
use viewer_confined::{Canceller, Confined, ConfinedError, Reopen, Resuming};
use viewer_core::{Command, DocumentId, Event};
use viewer_host::Asking;
use viewer_host::drawing::Drawing;
use viewer_host::trace::{Topic, Trace};
use viewer_ui::chrome::{Chrome, PasswordCard, QuestionCard, RestrictionsCard};
use viewer_ui::software::SoftwareSurface;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::arguments::{Arguments, arguments};
use crate::device::Device;
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
    /// The pages a draw of which left marks to the library's converter — here, or in the worker
    /// whose frames say so — which the title names (ADRs 2290, 2316); the sentences are the
    /// confined viewer's, said once per page and condition.
    declined: std::collections::BTreeSet<usize>,
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
    instancing: Option<std::thread::JoinHandle<raster_gpu::LaunchInstance>>,
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
    permissions: RestrictionsCard,
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
    /// [`viewer_core::Query::View`]'s answer and [`Resuming`]'s to hold.
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
            declined: std::collections::BTreeSet::new(),
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
            permissions: RestrictionsCard::default(),
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
            WindowEvent::MouseInput { state, .. } if self.permissions.shown => {
                if state == ElementState::Pressed && !self.password.shown && !self.question.shown {
                    self.permissions_pointer();
                }
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
            WindowEvent::MouseWheel { .. } if self.permissions.shown => {}
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
        for mut finished in self.drawing.collect() {
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
            self.declined_in(
                finished.request.page,
                std::mem::take(&mut finished.declined),
            );
            changed |= self.screen.landed(finished);
        }
        for (mut finished, window) in self.drawing.windows() {
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
            self.declined_in(
                finished.request.page,
                std::mem::take(&mut finished.declined),
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
#[path = "quorra-confined/tests.rs"]
mod tests;
