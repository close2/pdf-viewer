//! The viewer confined, with a window on it: pages arrive from `pdf-view-worker` over a pipe.
//!
//! ```text
//! cargo run --release -p viewer-ui --bin pdf-viewer-confined -- document.pdf
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
//! scrolls, `q` quits. **Escape is the abort**: it ends the worker with a kill it cannot decline
//! (ADR 0241) and takes back this side's own drawing thread (ADR 0650) — the reader's answer to
//! a document written to take for ever, and it does not block. The window title carries the page
//! and §12.4.2's label; what a page could not draw is printed, as everywhere else in this tree.
//!
//! # What this window is, and is not
//!
//! It is deliberately the *smallest complete* host on this boundary, and its scope is a decision
//! with its cost written down (ADR 0713) rather than a promise of more: no panels, no form
//! controls, no selection, no find bar — each of those is chrome the three established windows
//! already have in-process, and moving *them* onto this boundary is the remainder `doc/todo/15`
//! names. What is complete is the part no other window has at all: every page on the screen came
//! out of the sandboxed process, on both of ADR 0607's arms, with the drawing of the marks arm
//! interruptible and the worker killable from the keyboard. A document this window cannot serve —
//! a file the document asks for, a URI it wants resolved — is refused **by name**, on the screen
//! and on standard error, never quietly.
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
//! on its first `ioctl` (ADR 0607) — so a list payload goes to `render-quorra` on
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

#[path = "pdf-viewer-confined/device.rs"]
mod device;
#[path = "pdf-viewer-confined/screen.rs"]
mod screen;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use render_quorra::QuorraWindowRenderer;
use viewer_confined::{
    Canceller, Confined, ConfinedError, Payload, Reopen, Reply, Resume, Resuming,
};
use viewer_core::{Command, DocumentId, Event, PageTarget, Query, Viewing, Zoom};
use viewer_host::drawing::Drawing;
use viewer_host::trace::{Topic, Trace};
use viewer_host::{Ask, Asking, Supplied};
use viewer_ui::chrome::{Chrome, PasswordCard};
use viewer_ui::software::SoftwareSurface;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::device::{Device, Presented};
use crate::screen::{Draw, Screen};

/// The one document this window opens; the identity is the host's to choose.
const DOCUMENT: DocumentId = DocumentId(0);

/// How far one wheel notch moves the page, in device pixels.
///
/// No clause states a wheel distance and neither toolkit host chooses one — each takes what GTK
/// or Qt hands it — so this window's is a documented choice: three text lines' worth, the common
/// desktop convention.
const WHEEL_NOTCH: f32 = 48.0;

/// The command line: a document, optionally `--trace[=topics]`, and `--cpu`.
struct Arguments {
    /// The file to open.
    path: PathBuf,
    /// Which trace topics to print.
    topics: u8,
    /// `--cpu`: no graphics device — the flagship's flag with the flagship's meaning (ADR
    /// 0221): no instance is created, so no driver is loaded, and the window presents through
    /// the processor.
    processor: bool,
}

/// A message's `Debug` form cut to a line, for the trace.
///
/// The cut is what the other hosts apply and is load-bearing here, not cosmetic: `Command::Open`
/// carries the whole document and a `Debug` of its bytes is five characters a byte — the first
/// run of this program wrote three quarters of a megabyte of trace for one open. A `Secret`
/// already prints no password whatever the length, so what the cut protects is the terminal.
fn brief(message: &impl std::fmt::Debug) -> String {
    let whole = format!("{message:?}");
    if whole.chars().count() > 120 {
        let mut cut: String = whole.chars().take(120).collect();
        cut.push('…');
        return cut;
    }
    whole
}

/// Reads the arguments or says why not; the usage line is the refusal.
fn arguments() -> Arguments {
    let mut path = None;
    let mut topics = 0u8;
    let mut processor = false;
    for argument in std::env::args().skip(1) {
        if argument == "--trace" {
            topics = u8::MAX;
        } else if argument == "--cpu" {
            processor = true;
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
            eprintln!("usage: pdf-viewer-confined [--trace[=topics]] [--cpu] document.pdf");
            std::process::exit(2);
        }
    }
    let Some(path) = path else {
        eprintln!("usage: pdf-viewer-confined [--trace[=topics]] [--cpu] document.pdf");
        std::process::exit(2);
    };
    Arguments {
        path,
        topics,
        processor,
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
    /// The sentence after the worker is gone — cancelled, dead, or refused — shown in the title.
    ///
    /// Once set, every later command is declined here rather than sent to a process that cannot
    /// answer: the worker's document died with it, so there is nothing to resume (ADR 0241).
    stopped: Option<String>,
    window: Option<Arc<Window>>,
    /// What puts pixels on the window — the device, or the processor (`--cpu`, or a device
    /// that would not come up). Chosen once in `resumed` and never switched mid-life.
    presentation: Option<Presentation>,
    /// The thread creating the graphics instance — roughly 80% of what device bring-up blocks
    /// for, started before the window exists (the flagship's arrangement, quorra's ADR 0014).
    /// `None` under `--cpu`, where no driver may be loaded at all.
    instancing: Option<std::thread::JoinHandle<quorra_gpu::wgpu::Instance>>,
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
}

/// What puts this window's pixels up (ADR 0725): the graphics device, or the processor.
enum Presentation {
    /// A `render-quorra` device on its own thread, presented through a `quorra_gpu::Presenter`.
    ///
    /// Boxed because a `Device` holds the renderer until the first job moves it to its thread,
    /// and an enum is as wide as its widest variant.
    Device(Box<Device>),
    /// The processor's surface — `--cpu`, or the path a machine with no usable device falls to.
    Software(SoftwareSurface),
}

impl Host {
    fn new(path: PathBuf, topics: u8, processor: bool, began: Instant) -> Self {
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
            window: None,
            presentation: None,
            // Creating the instance *is* loading the driver, so `--cpu` must not spawn this
            // thread — the flagship's rule (ADR 0221), kept here for the same crash.
            instancing: (!processor).then(|| std::thread::spawn(QuorraWindowRenderer::instance)),
            drawing: Drawing::new(),
            screen: if processor {
                Screen::new()
            } else {
                Screen::for_device()
            },
            presented: None,
            asking: Asking::new(),
            password: PasswordCard::default(),
            chrome: None,
            leaving: false,
            page: 0,
            resuming: Resuming::new(),
            resume: None,
        }
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
        let bytes = match pdf_syntax::read_file(&self.path) {
            Ok(bytes) => bytes,
            Err(problem) => {
                self.stop(format!("cannot read {}: {problem}", self.path.display()));
                return;
            }
        };
        // A canceller of its own, because a cancellation holds the worker it was made for: the
        // old one names a process that has already been reaped, and Escape must reach the new one.
        let canceller = Canceller::new();
        let starting = Instant::now();
        let confined = match Confined::start_with(&canceller) {
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

        if let Some(window) = self.window.as_ref() {
            let extent = window.inner_size();
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a display's scale factor is a small ratio"
            )]
            let scale = window.scale_factor() as f32;
            self.dispatch(&Command::Resize {
                width: extent.width,
                height: extent.height,
                scale,
            });
        }
        self.dispatch(&Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            // The password is deliberately not kept on this side (`password_answered` says why),
            // so an encrypted document asks for it again — §7.6.4.1's card, exactly as at launch.
            password: None,
            fragment: None,
        });
        self.dispatch(&Command::View(reopen.view));
    }

    /// The file's own name, for the title.
    fn name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.to_string_lossy().into_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
    }

    /// Sends one command, hands its events on, and pulls the frame the view now shows.
    fn dispatch(&mut self, command: &Command) {
        if self.stopped.is_some() {
            return;
        }
        if self.trace.on(Topic::Events) {
            self.trace
                .say(Topic::Events, format_args!("-> {}", brief(command)));
        }
        let outcome = match self.confined.as_mut() {
            Some(confined) => confined.handle(command),
            None => return,
        };
        match outcome {
            Ok(events) => {
                for event in events {
                    self.event(event);
                }
                self.pull_frame();
            }
            Err(problem) => self.died(&problem),
        }
    }

    /// What this window does about each event the confined viewer sent back.
    ///
    /// Exhaustive on purpose: a message added to the boundary must fail to compile here rather
    /// than fall into a catch-all arm (`doc/ui-boundary.md`'s rule).
    fn event(&mut self, event: Event) {
        if self.trace.on(Topic::Events) {
            self.trace
                .say(Topic::Events, format_args!("<- {}", brief(&event)));
        }
        match event {
            Event::Opened { pages, .. } => {
                // The next document's attempts start from nothing (`Asking::opened`'s rule).
                self.asking.opened();
                // A document opens where its own view says, which the viewer states as a
                // `PageChanged` when it is not the first page. Until it does, this is page one.
                self.page = 0;
                self.heading = format!("{} — {pages} page(s)", self.name());
                self.retitle();
            }
            Event::OpenFailed { reason, .. } => {
                // The worker survives an open it refused; only this document is over.
                self.stop(format!("cannot open {}: {reason}", self.path.display()));
            }
            Event::PasswordRequired { .. } => {
                // §7.6.4.1: an interactive processor *should* prompt, and this window is one
                // (ADR 0718; the refusal it replaces was ADR 0713's scope decision). The policy
                // — how many attempts, what exhaustion means — is `viewer_host::password`'s,
                // shared with the three established windows.
                if self.chrome.is_none() {
                    match Chrome::new() {
                        Ok(chrome) => self.chrome = Some(chrome),
                        Err(said) => {
                            // A build whose compiled-in faces will not parse cannot draw the
                            // card; refusing by name is what is left (trap 5).
                            self.stop(format!(
                                "{} is encrypted and this build cannot draw the prompt \
                                 ({said}); open it in pdf-viewer, pdf-viewer-gtk or \
                                 pdf-viewer-qt",
                                self.path.display()
                            ));
                            return;
                        }
                    }
                }
                match self.asking.required() {
                    Ask::Prompt { attempt, of } => {
                        self.password
                            .ask(viewer_host::password::prompt(&self.name(), attempt, of));
                        self.card_changed();
                    }
                    Ask::Exhausted => {
                        // Stop asking and leave the window open — `viewer_host::password`'s
                        // rule: no host may make `Exhausted` mean the window closes.
                        eprintln!("note: {}", viewer_host::password::EXHAUSTED);
                        self.heading =
                            format!("{} — {}", self.name(), viewer_host::password::EXHAUSTED);
                        self.retitle();
                    }
                }
            }
            // Four arms with nothing to do, and each for its own reason, kept in one place so
            // that a message added to the boundary still breaks this build: a `Closed` follows
            // this window's own `Close` and nothing else; a `Transition` never fires because this
            // window sends no `Command::Tick` and starts no presentation; a `Searched` answers a
            // `Find` this window never sends; and a `Dirty` reports an edit no key here can make.
            Event::Closed(_)
            | Event::Transition { .. }
            | Event::Dirty { .. }
            | Event::Searched { .. } => {}
            Event::PageChanged {
                index, label, of, ..
            } => {
                self.page = index;
                let name = self.name();
                self.heading = match label {
                    Some(label) => {
                        format!(
                            "{name} — page {label} ({} of {of})",
                            index.saturating_add(1)
                        )
                    }
                    None => format!("{name} — page {} of {of}", index.saturating_add(1)),
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
            // §12.6.4.8: resolving a URI reaches outside the program, and this window declines
            // by name rather than quietly. The policy a fuller host applies is `viewer_host`'s.
            Event::OpenUri { uri, .. } => {
                eprintln!("this window does not open links; the document asked for: {uri}");
            }
            // A file the document asks for is a question about *this* machine's filesystem
            // (rule 2), and this window supplies none: said, not swallowed.
            Event::NeedsFile { .. } => {
                eprintln!("this window does not supply files a document asks for");
            }
            Event::Saved { .. } | Event::Extracted { .. } => {
                eprintln!("note: the confined viewer sent bytes this window never asked for");
            }
            Event::Refused { notes, .. } => {
                for note in notes {
                    eprintln!("refused by the document's restrictions: {note}");
                }
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
                self.screen.take(frames, &mut self.drawing);
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
            if asked > 0 {
                // The card and the pixel pages still want a frame, now without the refused
                // marks; the fallback pages join it as each lands off the thread.
                self.ask_frame();
            }
            return;
        }
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

    /// §7.6.4.1's card as display lists, over whatever the screen has — which for an encrypted
    /// document is the surround, exactly the flagship's arrangement of the same card.
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
                Presented::Shown => {
                    if self.presented.is_none() {
                        self.trace
                            .say(Topic::Launch, format_args!("first frame presented"));
                    }
                    self.presented = Some((extent.width, extent.height));
                }
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
    }

    /// The processor's present: the screen composed, the card drawn over it.
    fn present_software(&mut self) {
        let (Some(window), Some(Presentation::Software(surface))) =
            (self.window.as_ref(), self.presentation.as_mut())
        else {
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
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        let card = self.chrome.as_ref().and_then(|chrome| {
            self.password
                .draw(chrome, extent.width, extent.height, scale)
        });
        let overlays: Vec<&pdf_render::DisplayList> = card.iter().collect();
        match surface.present(&composed, &overlays) {
            Ok(()) => {
                if self.presented.is_none() {
                    self.trace
                        .say(Topic::Launch, format_args!("first frame presented"));
                }
                self.presented = Some((extent.width, extent.height));
            }
            Err(problem) => eprintln!("the frame could not be presented: {problem}"),
        }
    }

    fn redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn retitle(&self) {
        if let Some(window) = self.window.as_ref() {
            let title = match &self.stopped {
                Some(said) => format!("{} — {said} — confined", self.heading),
                None => format!("{} — confined", self.heading),
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
    /// The file is read again rather than kept: rule 2 makes the filesystem this side's, and a
    /// file that has gone away between the attempts is a fact about this machine, said out loud.
    fn password_answered(&mut self) {
        let typed = self.password.take();
        self.card_changed();
        let secret = match viewer_host::password::supplied(typed) {
            Supplied::Open(secret) => secret,
            Supplied::Cancelled => {
                eprintln!("note: {}", viewer_host::password::CANCELLED);
                self.heading = format!("{} — {}", self.name(), viewer_host::password::CANCELLED);
                self.retitle();
                return;
            }
        };
        let bytes = match pdf_syntax::read_file(&self.path) {
            Ok(bytes) => bytes,
            Err(problem) => {
                eprintln!("note: cannot re-read {}: {problem}", self.path.display());
                self.heading = format!("{} — cannot re-read the file", self.name());
                self.retitle();
                return;
            }
        };
        // The worker survives an open it could not finish (ADR 0597), so the retry goes to the
        // same process: the document's bytes cross the pipe once more, which is the cost of the
        // host keeping no copy — priced in `doc/todo/15` §5 and paid at most `ATTEMPTS` times.
        self.dispatch(&Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: Some(secret),
            fragment: None,
        });
    }

    /// One key, one meaning; anything else is nothing.
    fn key(&mut self, event: &KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        if self.password.shown {
            let key = event.logical_key.as_ref();
            self.password_key(&key);
            return;
        }
        match &event.logical_key {
            Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space) => {
                self.dispatch(&Command::GoTo(PageTarget::Next));
            }
            Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => {
                self.dispatch(&Command::GoTo(PageTarget::Previous));
            }
            Key::Named(NamedKey::ArrowDown) => self.dispatch(&Command::Scroll {
                dx: 0.0,
                dy: WHEEL_NOTCH,
            }),
            Key::Named(NamedKey::ArrowUp) => self.dispatch(&Command::Scroll {
                dx: 0.0,
                dy: -WHEEL_NOTCH,
            }),
            Key::Named(NamedKey::Home) => self.dispatch(&Command::GoTo(PageTarget::First)),
            Key::Named(NamedKey::End) => self.dispatch(&Command::GoTo(PageTarget::Last)),
            Key::Named(NamedKey::Escape) => self.abort(),
            Key::Character(text) => match text.as_str() {
                "+" | "=" => self.dispatch(&Command::Zoom {
                    zoom: Zoom::In,
                    at: None,
                }),
                "-" => self.dispatch(&Command::Zoom {
                    zoom: Zoom::Out,
                    at: None,
                }),
                "q" => self.leaving = true,
                _ => {}
            },
            _ => {}
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
            // has crossed yet — the worker starts below — so the replacement costs nothing.
            self.screen = Screen::new();
        }
        let extent = window.inner_size();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        self.window = Some(window);

        // The worker starts when there is a document for it, not before: CLAUDE.md's launch
        // rule, and the spawn-to-confined cost is the first thing the trace says.
        let starting = Instant::now();
        let confined = match Confined::start_with(&self.canceller) {
            Ok(confined) => confined,
            Err(problem) => {
                self.stop(problem.to_string());
                return;
            }
        };
        self.trace.say(
            Topic::Launch,
            format_args!(
                "worker started and confined in {:.1} ms",
                starting.elapsed().as_secs_f64() * 1e3
            ),
        );
        // A kernel can refuse what a build offers, and a person relying on the sandbox is owed
        // the difference out loud (`Confined::confinement` is a report, not a promise).
        if let Some(short) = confined.confinement().shortfall() {
            eprintln!("confinement shortfall: {short}");
        }
        self.confined = Some(confined);

        let bytes = match pdf_syntax::read_file(&self.path) {
            Ok(bytes) => bytes,
            Err(problem) => {
                self.stop(format!("cannot read {}: {problem}", self.path.display()));
                return;
            }
        };
        self.dispatch(&Command::Resize {
            width: extent.width,
            height: extent.height,
            scale,
        });
        self.dispatch(&Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                // The drawing thread is joined on the way out; a hostile page still on it would
                // hold the join for as long as it liked, so it is taken back first.
                self.drawing.superseded(false);
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
                self.dispatch(&Command::Resize {
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
                self.dispatch(&Command::Scroll { dx, dy });
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
            event_loop.exit();
            return;
        }
        // The one place a worker is started again, and it is here rather than where the death
        // was seen so that no restart runs inside another one ([`Host::resume`]).
        if let Some(reopen) = self.resume.take() {
            self.reopen(reopen);
        }
        let mut changed = false;
        for finished in self.drawing.collect() {
            changed |= self.screen.landed(finished);
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
        let waiting = self.drawing.interval().is_some()
            || matches!(
                self.presentation.as_ref(),
                Some(Presentation::Device(device)) if device.busy()
            );
        if waiting {
            let at = Instant::now()
                .checked_add(viewer_host::drawing::POLL)
                .unwrap_or_else(Instant::now);
            event_loop.set_control_flow(ControlFlow::WaitUntil(at));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn main() {
    let began = Instant::now();
    let Arguments {
        path,
        topics,
        processor,
    } = arguments();
    let mut host = Host::new(path, topics, processor, began);
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
        Host::new(PathBuf::from("locked.pdf"), 0, true, Instant::now())
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
            host.heading.contains("cannot re-read"),
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
