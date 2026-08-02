//! The viewer: opens a PDF and shows it.
//!
//! ``text
//! cargo run --release -p viewer-ui --bin pdf-viewer -- document.pdf
//! ``
//!
//! `--page N` opens at a page. Arrows, Page Up and Down or Space turn pages, Home and End jump, `+` and `-` zoom, `a`
//! selects the whole page and dragging selects part of it, `o` shows §12.3.3's outline, `s`
//! saves what was changed beside the document, Escape quits. The window title shows
//! the page's own label where the document states one (§12.4.2), the page number, and how many
//! things on the page could not be drawn; the things themselves are printed.
//!
//! # What is here and what is not
//!
//! This is **consumer #1 of `viewer-core`** and a **tier-2 host**: everything about documents,
//! pages, zoom, links and actions is in that crate, and what is left here is a window, a
//! keyboard, a GPU and the two decisions a host owns — which files a document may name, and what
//! to do when it asks for a password.
//!
//! And, since the hundred-and-sixty-sixth session, **chrome**: `viewer_ui::chrome` draws
//! §12.3.3's outline in a panel of this program's own, because winit is a window and an event
//! loop and there is no toolkit here to ask for a tree view. A native host would use its
//! platform's, from the same [`viewer_core::Query::Outline`]; what is host-specific is the
//! drawing, not the data.
//!
//! Tier 2 means the pixels never cross the boundary: `viewer-core` hands over a display list and
//! a target, this draws it onto the surface with `render-gpu`, and answers `Rendered::Presented`.
//! A tier-1 host would take the raster back instead; the protocol is otherwise identical, which
//! is the property that makes the interface worth having.
//!
//! # Reporting incomplete pages
//!
//! When the interpreter cannot draw something — an unsupported font, a shading — it is said out
//! loud. A viewer that renders a page missing half its content and looks confident about it is
//! worse than one that admits the gap, and the person looking at it is the only one in a
//! position to judge whether what is missing matters.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line application: stdout is a reporting channel and a panic on a \
              missing display is the intended failure"
)]

use std::collections::VecDeque;
use std::io::Write as _;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

use pdf_render::{Rasterizer as _, TargetSpec, Transform};
use render_cpu::CpuRasterizer;
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu::CurrentSurfaceTexture;
use vello::{AaConfig, AaSupport, Renderer, RendererOptions, wgpu};
use viewer_core::{
    Answer, Command, DocumentId, Event, PageTarget, PointerAction, Purpose, Query, RenderRequest,
    Rendered, Selection, Viewer, Zoom,
};
use viewer_ui::chrome::{Chrome, Hit, Panel};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

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

fn main() {
    // Parsed before anything opens a document, because it decides where that document's images
    // are decoded and a policy applied halfway through is not a policy.
    let mut path = None;
    let mut sandbox = true;
    let mut trace = false;
    let mut processor = false;
    let mut opens_at = None;
    let mut arguments = std::env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--licences" || argument == "--licenses" {
            print!("{NOTICE}");
            return;
        } else if argument == "--no-sandbox" {
            sandbox = false;
        } else if argument == "--trace" {
            trace = true;
            // The graphics stack's own voice, which is silent until something receives it.
            speak_up();
        } else if argument == "--cpu" {
            processor = true;
        } else if argument == "--page" {
            // A page number as the title bar shows it, which is one-based. §12.3.2.1's
            // `/OpenAction` is the document's own answer to the same question and wins where
            // this is absent; where both are stated, the person asking now wins.
            opens_at = arguments
                .next()
                .and_then(|value| value.to_string_lossy().parse::<usize>().ok())
                .filter(|page| *page > 0);
            if opens_at.is_none() {
                eprintln!("--page wants a page number, counting from 1");
                std::process::exit(2);
            }
        } else if path.is_none() {
            path = Some(argument);
        } else {
            eprintln!("unexpected argument: {}", argument.to_string_lossy());
            std::process::exit(2);
        }
    }
    let Some(path) = path else {
        usage();
        std::process::exit(2);
    };

    if !sandbox {
        pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
        // Said out loud, once, on the way past. Turning the sandbox off is a reasonable choice
        // for documents you produced yourself and a bad one for documents that arrived by
        // email, and the difference is not visible from inside the program.
        println!(
            "note: --no-sandbox — JBIG2 and JPEG 2000 will be decoded in this process, with no \
             memory ceiling, and a decoder failure will take the viewer down with it"
        );
    }

    // Rule 2: the host owns the filesystem, and this is the only place a path becomes bytes.
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.to_string_lossy());
            std::process::exit(1);
        }
    };

    let mut app = App {
        // No viewport until the window exists. The core renders nothing into one with no
        // extent, which is exactly right: there is nothing to render into yet.
        viewer: Viewer::new(0, 0, 1.0),
        title: path.to_string_lossy().into_owned(),
        path: PathBuf::from(&path),
        // §12.7.6.4's import-data action names a file, and this is the only place that name is
        // allowed to mean anything: a *sibling of the document being shown*. See `supply`.
        directory: std::path::Path::new(&path)
            .parent()
            .map(std::path::Path::to_path_buf),
        caption: String::new(),
        request: None,
        acknowledged: true,
        trace,
        processor,
        cursor: (0.0, 0.0),
        dragging: false,
        dirty: false,
        attempts: 0,
        chrome: match Chrome::new() {
            Ok(chrome) => Some(chrome),
            Err(problem) => {
                eprintln!("note: no panel: {problem}");
                None
            }
        },
        panel: Panel::default(),
        outline: pdf_model::outline::Outline::default(),
        context: RenderContext::new(),
        state: None,
    };
    app.dispatch(Command::Open {
        id: DOCUMENT,
        bytes,
        password: None,
    });
    if let Some(page) = opens_at {
        app.dispatch(Command::GoTo(PageTarget::Index(page.saturating_sub(1))));
    }

    let event_loop = EventLoop::new().expect("an event loop requires a display server");
    // Redraw on request rather than continuously: a document viewer is idle almost all the time,
    // and a spinning loop would drain a battery for nothing.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app).expect("event loop failed");
}

/// What the program does when it is given nothing to open.
fn usage() {
    eprintln!("usage: pdf-viewer [--no-sandbox] <document.pdf>");
    eprintln!("       pdf-viewer --licences");
    eprintln!();
    eprintln!("Arrows, Page Up/Down or Space turn pages; Home and End jump; + and - zoom;");
    eprintln!("o shows the document's outline; drag to select text, a selects the page,");
    eprintln!("s saves, Escape quits.");
    eprintln!();
    eprintln!("  --no-sandbox  decode JBIG2 and JPEG 2000 images in this process rather than");
    eprintln!("                in a confined worker. Faster by a process spawn and a pipe");
    eprintln!("                round trip; appropriate only for documents you trust.");
    eprintln!("  --page N      open at page N, counting from 1 as the title bar does.");
    eprintln!("  --cpu         draw with the processor rather than the graphics device. Slower,");
    eprintln!("                and the same rasteriser the reference comparison is built on: a");
    eprintln!("                page that appears with this and not without it is the device's.");
    eprintln!("  --trace       print every command, every event and every frame, with the time");
    eprintln!("                each took, and whatever the graphics stack has to say. What to");
    eprintln!("                run when a page will not appear: the last line printed is the");
    eprintln!("                step that did not finish. PDFVIEWER_LOG=error|warn|info|debug");
    eprintln!("                sets how much of the graphics stack's own logging comes with it,");
    eprintln!("                and defaults to warn.");
    eprintln!("  --licences    print the third-party notices this binary carries, and exit.");
}

/// How many passwords a person is asked for before the program gives up.
///
/// §7.6.4.1 states no limit — it says a processor tries the empty password and then prompts —
/// so this is a choice about a terminal rather than about the clause, and an empty line cancels
/// before it is reached.
const PASSWORD_ATTEMPTS: usize = 3;

#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent facts about a window, each read in one place: whether a button is \
              down, whether the core has been told about the last frame, whether anything is \
              unsaved, and whether to say what is happening"
)]
struct App {
    /// Everything about documents, pages and clicks.
    viewer: Viewer,
    /// The file's name, for the title bar.
    title: String,
    /// The file itself, kept because §7.6.4.1's prompt has to open it again with a password.
    ///
    /// The path rather than the bytes: a host that held a copy of every document it had failed
    /// to open would be holding a copy of every document.
    path: PathBuf,
    /// The directory the open document is in, where one can be named.
    ///
    /// The whole of this program's answer to "which files may a document ask for". §12.7.6.4's
    /// import-data action carries a file specification the *document* wrote, so honouring it
    /// unrestricted would let a PDF read any path this process can — and the clause states no
    /// policy, because a policy is a property of the processor. See [`App::supply`].
    directory: Option<PathBuf>,
    /// What the title bar says about the page, from the last `PageChanged`.
    caption: String,
    /// The render `viewer-core` last asked for, kept so an expose can redraw it.
    request: Option<RenderRequest>,
    /// Whether the core has been told that request was drawn.
    acknowledged: bool,
    /// Where the pointer last was, in device pixels.
    ///
    /// `winit` reports movement and clicks as separate events, so a click needs the position
    /// remembered from the last `CursorMoved` — the click itself carries none.
    cursor: (f64, f64),
    /// Whether to say what is happening, from `--trace`.
    ///
    /// A viewer that will not draw a page has to be able to say how far it got, and the four
    /// steps between a key press and a frame — command, interpretation, draw, present — are
    /// invisible from outside the process. This makes them visible, in order, with a duration
    /// apiece; the *last line printed* is the step that did not finish.
    trace: bool,
    /// Whether to draw with `render-cpu` rather than the graphics device, from `--cpu`.
    ///
    /// The same rasteriser the reference oracle is built on, and the same one that draws a page
    /// the device refuses. As a *flag* it is a diagnostic a person can pull without a debugger:
    /// if a page appears under `--cpu` and not without it, the difference is the device.
    processor: bool,
    /// Whether the button is down, which is what separates a move from a drag.
    dragging: bool,
    /// Whether anything a person did is unsaved.
    dirty: bool,
    /// How many passwords have been asked for.
    attempts: usize,
    /// The fonts this program draws its own text with, or why it cannot.
    ///
    /// An `Option` because a build whose compiled-in faces will not parse must still show the
    /// document: the panel is chrome and the page is the point. The refusal is printed once.
    chrome: Option<Chrome>,
    /// §12.3.3's outline, as this program draws it — the first panel this project has had.
    panel: Panel,
    /// The outline itself, taken once when the document opened.
    ///
    /// Copied out of `Query::Outline` rather than asked for per frame, and not for speed:
    /// `Answer::Outline` borrows the viewer, and a panel that is about to send it a command
    /// cannot be holding a borrow of it. The document is immutable and no edit reaches
    /// §12.3.3, so a copy taken at open cannot go stale.
    outline: pdf_model::outline::Outline,
    context: RenderContext,
    state: Option<State>,
}

/// The window, and the GPU objects that belong to it.
struct State {
    window: Arc<Window>,
    surface: RenderSurface<'static>,
    renderer: Renderer,
    /// What the page is drawn into, before it is blitted onto the frame.
    ///
    /// Vello's own surface target would do, but it is created with `STORAGE_BINDING` and
    /// `TEXTURE_BINDING` only. Banding composes the result from band-sized renders copied into
    /// place, which needs `COPY_DST`, so this program owns the texture it draws into.
    target: Target,
    /// How many bands the last render of this window needed; reset when the window resizes.
    bands: render_gpu::Bands,
}

/// A texture the size of the window, with the usages banding needs.
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    /// Creates a target for a window of this size.
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("page"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            // `STORAGE_BINDING` is where Vello writes, `TEXTURE_BINDING` is what the blitter
            // reads, and `COPY_DST` is where a band lands.
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Target").finish_non_exhaustive()
    }
}

impl App {
    /// The window's extent in device pixels and its scale factor, once there is a window.
    fn window(&self) -> Option<(u32, u32, f32)> {
        let state = self.state.as_ref()?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = state.window.scale_factor() as f32;
        Some((
            state.surface.config.width,
            state.surface.config.height,
            scale,
        ))
    }

    /// How many device pixels down the left edge the panel occupies.
    ///
    /// **The page's viewport is the window less this.** A panel drawn *over* the page would
    /// hide part of it and leave the core centring the page behind the panel; telling the core
    /// about the smaller viewport instead is what makes a fitted page fit what is visible. It
    /// also means every coordinate crossing the boundary — a pointer going in, a selection quad
    /// coming out — is offset by exactly this and nothing else.
    fn inset(&self) -> u32 {
        self.window()
            .map_or(0, |(_, _, scale)| self.panel.inset(scale))
    }

    /// Tells the core how much of the window is the page's, after the panel appeared or went.
    fn resize_page(&mut self) {
        let Some((width, height, scale)) = self.window() else {
            return;
        };
        self.dispatch(Command::Resize {
            width: width.saturating_sub(self.inset()).max(1),
            height,
            scale,
        });
        self.redraw();
    }

    /// The panel's own display list for this frame, or `None` when there is nothing to draw.
    ///
    /// Rebuilt per frame rather than kept: it is a few hundred glyph fills against the page's
    /// tens of thousands, and a cache would be one more thing that can disagree with the scroll
    /// position.
    fn panel_list(&self, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        if !self.panel.shown {
            return None;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        Some(self.panel.draw(chrome, &self.outline, height, scale))
    }

    /// What the pointer moving does: the panel's highlight, or the page's §12.5.5 appearance.
    ///
    /// Only one of the two, and never both: a hover highlight in the panel and a rollover
    /// appearance on the page are both answers to "what is under the pointer", and answering
    /// both would leave an annotation lit up behind a panel.
    fn pointer_moved(&mut self) {
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        if self.panel.hover(at(self.cursor), &self.outline, scale) {
            self.redraw();
        }
        if self.over_panel() {
            if let Some(state) = self.state.as_ref() {
                state.window.set_cursor(winit::window::CursorIcon::Default);
            }
            return;
        }
        let point = self.on_page(self.cursor);
        self.dispatch(Command::Pointer {
            at: point,
            action: if self.dragging {
                PointerAction::Dragged
            } else {
                PointerAction::Moved
            },
        });
        // §12.5.6.5's activation region, asked at pointer speed — which is why it is a query
        // rather than a command with an event coming back.
        if let (Answer::Link(over), Some(state)) =
            (self.viewer.query(Query::LinkAt(point)), self.state.as_ref())
        {
            state.window.set_cursor(if over {
                winit::window::CursorIcon::Pointer
            } else {
                winit::window::CursorIcon::Default
            });
        }
    }

    /// Whether the pointer is over the panel rather than over the page.
    fn over_panel(&self) -> bool {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        edge > 0.0 && at(self.cursor).0 < edge
    }

    /// A window point in the page's own viewport, which begins where the panel ends.
    fn on_page(&self, cursor: (f64, f64)) -> (f32, f32) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let (x, y) = at(cursor);
        (x - edge, y)
    }

    /// What a click inside the panel does.
    fn click_panel(&mut self) {
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        // The outline is a field rather than a query for exactly this: `Panel::click` produces a
        // command for the viewer, and `Answer::Outline` would still be borrowing it.
        let hit = self.panel.click(at(self.cursor), &self.outline, scale);
        match hit {
            Some(Hit::Follow(target)) => self.dispatch(Command::GoTo(target)),
            Some(Hit::Toggle) => self.redraw(),
            Some(Hit::Nothing) | None => {}
        }
    }

    /// A wheel notch: the panel's list where the pointer is over it, the page otherwise.
    fn wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        // A line is not a pixel and winit reports whichever the device produced. Sixteen logical
        // pixels a line is about one row of this program's own text, which is what a line means
        // on a list; a touchpad reports pixels and needs no conversion.
        let by = match delta {
            winit::event::MouseScrollDelta::LineDelta(_, lines) => -lines * 16.0,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a scroll delta in pixels, which is tens"
            )]
            winit::event::MouseScrollDelta::PixelDelta(position) => -(position.y as f32),
        };
        if self.over_panel() {
            let Some((_, height, scale)) = self.window() else {
                return;
            };
            self.panel.scroll(by / scale, &self.outline, height, scale);
            self.redraw();
        } else {
            self.dispatch(Command::Scroll { dx: 0.0, dy: by });
        }
    }

    /// Hands a command to the core and deals with everything that comes back.
    ///
    /// A queue rather than recursion, because reacting to an event may produce a command — a
    /// password supplied, a file read — and a chain of those is a loop rather than a stack.
    fn dispatch(&mut self, command: Command) {
        let mut queue = VecDeque::from([command]);
        while let Some(command) = queue.pop_front() {
            let started = std::time::Instant::now();
            let described = self.trace.then(|| describe_command(&command));
            let events: Vec<Event> = self.viewer.handle(command).collect();
            if let Some(described) = described {
                println!(
                    "trace: {described} -> {} event(s) in {:?}",
                    events.len(),
                    started.elapsed()
                );
                for event in &events {
                    println!("trace:     {}", describe_event(event));
                }
            }
            for event in events {
                self.react(event, &mut queue);
            }
        }
    }

    /// Does what one event asks.
    fn react(&mut self, event: Event, queue: &mut VecDeque<Command>) {
        match event {
            Event::Opened { pages, .. } => {
                println!("{}: {pages} page(s)", self.title);
                if pages == 0 {
                    eprintln!("the document has no pages");
                    std::process::exit(1);
                }
                self.attempts = 0;
                // §12.3.3, taken once. `Query::Outline` borrows the viewer, so what the panel
                // holds is a copy — see the field's own note.
                if let Answer::Outline(outline) = self.viewer.query(Query::Outline) {
                    self.outline = outline.clone();
                }
                if !self.outline.items.is_empty() {
                    println!(
                        "{}: an outline of {} visible item(s) — press o to show it",
                        self.title,
                        self.outline.visible_count()
                    );
                }
            }
            Event::OpenFailed { reason, .. } => {
                eprintln!("cannot open {}: {reason}", self.title);
                std::process::exit(1);
            }
            // §7.6.4.1: a processor tries the default user password and then prompts. This is the
            // prompt, and it is the whole of what this program owed the clause.
            Event::PasswordRequired { document } => {
                self.attempts = self.attempts.saturating_add(1);
                if self.attempts > PASSWORD_ATTEMPTS {
                    eprintln!("{}: too many attempts", self.title);
                    std::process::exit(1);
                }
                let Some(password) = ask_password(&self.title) else {
                    eprintln!("{}: needs a password", self.title);
                    std::process::exit(1);
                };
                let Ok(bytes) = std::fs::read(&self.path) else {
                    eprintln!("cannot re-read {}", self.title);
                    std::process::exit(1);
                };
                queue.push_back(Command::Open {
                    id: document,
                    bytes,
                    password: Some(password),
                });
            }
            Event::Closed(_) => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => {
                // ISO 32000-2 §12.4.2: "Page labels and page indices need not coincide". Where
                // the document states a label it is what a reader is meant to see — a page of
                // front matter is *iv*, not page four — so the index is shown beside it rather
                // than instead of it, because a title saying only `iv` cannot say `of 320`.
                let page = match label {
                    Some(label) => format!("{label} — page {} of {of}", index.saturating_add(1)),
                    None => format!("page {} of {of}", index.saturating_add(1)),
                };
                // §12.3.3's outline is a table of contents, so the item covering this page is
                // the section a reader is in. After the page number rather than before it,
                // because it is context for a position rather than the position itself.
                self.caption = match section {
                    Some(section) if !section.is_empty() => format!("{page} — {section}"),
                    _ => page,
                };
                self.retitle();
            }
            Event::NeedsRender(request) => {
                self.request = Some(request);
                self.acknowledged = false;
                self.redraw();
            }
            Event::Damage(_) => self.redraw(),
            // §12.6.4.8: printed rather than opened. What this program will not do is hand a
            // string a document controls to a browser, because that is a decision about this
            // machine and not about the document.
            Event::OpenUri { uri, .. } => println!("link: {uri}"),
            Event::NeedsFile { purpose, name, .. } => {
                let bytes = self.supply(purpose, &name);
                queue.push_back(Command::Supply { purpose, bytes });
            }
            // §12.6.4.15: this window redraws whole pages and animates nothing, so what it can
            // honestly do with a transition is name it.
            Event::Transition { transition, .. } => println!(
                "link: a transition action asks for {:?} over {} s; this program draws the page \
                 without animating it",
                transition.style, transition.duration
            ),
            // Rule 2 in one arm: the core produced the bytes and the host owns the filesystem.
            // Written beside the document with `.edited.pdf` appended rather than over it,
            // because overwriting somebody's file is a decision this program has not been given.
            Event::Saved { bytes, .. } => {
                let path = self.path.with_extension("edited.pdf");
                match std::fs::write(&path, &bytes) {
                    Ok(()) => println!("saved {} bytes to {}", bytes.len(), path.display()),
                    Err(error) => println!("note: cannot write {}: {error}", path.display()),
                }
            }
            // What a host does with this is mark its window and ask before closing. This one
            // has no dialogue to ask with, so it marks the title and says so on the way past.
            Event::Dirty { dirty, .. } => {
                self.dirty = dirty;
                self.retitle();
                if dirty {
                    println!("note: this document has unsaved changes");
                }
            }
            Event::Reported { page, notes, .. } => {
                for note in &notes {
                    println!("note: {note}");
                }
                if page.is_some() {
                    self.retitle_incomplete(notes.len());
                }
            }
        }
    }

    /// §12.7.6.4's file, under the narrowest policy that still performs the action.
    ///
    /// The clause says a processor "shall import data … from a specified file" and specifies
    /// nothing about *which* files a document may name, because that is a property of the
    /// processor. So this states the policy, and it is a host's to state:
    ///
    /// - the name must be a single path component, so `../…` and any absolute path are refused;
    /// - it is resolved against the directory the open document is in, and nowhere else.
    ///
    /// Every refusal is printed, which is trap 5 on the one path where a click can decline.
    fn supply(&self, purpose: Purpose, name: &str) -> Option<Vec<u8>> {
        let Purpose::ImportData = purpose;
        let directory = self.directory.as_ref().or_else(|| {
            println!("import-data: declined — the document is not in a known directory");
            None
        })?;
        // One component, checked as a path rather than as a string, so that a separator this
        // platform recognises and this program does not cannot slip through.
        let named = std::path::Path::new(name);
        let mut components = named.components();
        let (Some(std::path::Component::Normal(single)), None) =
            (components.next(), components.next())
        else {
            println!("import-data: declined — {name} is not a plain file name beside the document");
            return None;
        };
        let path = directory.join(single);
        match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                println!("import-data: cannot read {}: {error}", path.display());
                None
            }
        }
    }

    /// Asks the window to draw again, where there is one.
    fn redraw(&self) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }

    /// Puts the caption in the title bar.
    fn retitle(&self) {
        if let Some(state) = self.state.as_ref() {
            let mark = if self.dirty { "• " } else { "" };
            state
                .window
                .set_title(&format!("{mark}{} — {}", self.title, self.caption));
        }
    }

    /// Adds what the page could not draw to the title bar.
    ///
    /// A count rather than the list: a page may report dozens of items and a title bar that
    /// scrolls off the screen tells a person less than a number does. The items themselves are
    /// printed, in the core's own words.
    fn retitle_incomplete(&self, items: usize) {
        if let Some(state) = self.state.as_ref() {
            state.window.set_title(&format!(
                "{} — {} — incomplete: {items} item(s) not drawn",
                self.title, self.caption
            ));
        }
    }

    /// Draws the outstanding request onto the surface and presents it.
    ///
    /// Returns what to tell the core, or `None` where there is nothing to tell it: a redraw the
    /// swapchain gave back never reached a screen, and saying it did would leave the window
    /// showing the last page until something else happened to change.
    ///
    /// **That last sentence is the whole reason this returns an outcome rather than a `bool`.**
    /// A draw that *fails* used to print a line to stdout and answer `Presented`, so the core
    /// recorded the page as shown, never asked again, and the window kept the previous page under
    /// a title bar naming the new one. A person looking at the window saw a page that would not
    /// change and no reason why. Trap 5, on a path a person reaches with an arrow key.
    fn present(&mut self) -> Option<Rendered> {
        let request = self.request.clone()?;
        // Where the page sits in the window: the core centres it and scrolls it, and the host
        // draws it there by composing that offset into the target's own transform.
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        let (width, height) = {
            let state = self.state.as_ref()?;
            (state.surface.config.width, state.surface.config.height)
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let target = TargetSpec {
            width,
            height,
            transform: request
                .target
                .transform
                .then(Transform::translate(origin.0 + edge, origin.1)),
        };

        // Interactive chrome crosses as geometry, not pixels: the core hands over the shapes and
        // this host draws them in its own colour. A native one would use macOS's selection
        // colour, KDE's accent or the Windows highlight brush; this one has no theme to ask, so
        // it picks a blue and says so.
        let mut highlight = match self.viewer.query(Query::Selection) {
            Answer::Selected(selection) => selection.quads,
            _ => Vec::new(),
        };
        // The quads are device pixels of the *page's* viewport, which begins where the panel
        // ends. One addition here rather than a second coordinate space in the core.
        for quad in &mut highlight {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        let panel = self.panel_list(height);
        let drawn = if self.processor {
            Err("was not asked, because --cpu".to_owned())
        } else {
            let state = self.state.as_mut()?;
            draw(
                &self.context,
                state,
                &request,
                target,
                (width, height),
                &highlight,
                panel.as_ref(),
            )
        };
        if let Err(problem) = drawn {
            // **The CPU backend draws it instead**, and this is what `CLAUDE.md` keeps that
            // backend for: it is the correctness oracle *and* the startup path, so a page the
            // GPU refuses is a page this program can still show — more slowly, which is a cost
            // a person can see past, where a page that never appears is not.
            //
            // Reported either way. A page drawn by the slower of two backends is a fact about
            // this build worth saying out loud, and saying it is what would have made the
            // hundred-and-forty-second session's report a sentence rather than a mystery.
            let fallback = self.on_the_processor(
                &request,
                target,
                (width, height),
                &highlight,
                panel.as_ref(),
            );
            match fallback {
                Ok(()) if self.processor => {}
                Ok(()) => println!(
                    "note: page {}: the graphics device {problem}, so it was drawn on the \
                     processor instead",
                    request.page.saturating_add(1)
                ),
                Err(second) => {
                    return Some(Rendered::Failed(format!(
                        "the graphics device {problem}, and the processor {second}"
                    )));
                }
            }
        }
        let state = self.state.as_mut()?;
        let handle = &self.context.devices[state.surface.dev_id];

        // `get_current_texture` reports swapchain state rather than returning a Result, and the
        // non-success cases are ordinary events: a resize race leaves the surface outdated,
        // minimising occludes it.
        let frame = match state.surface.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.context.configure_surface(&state.surface);
                state.window.request_redraw();
                return None;
            }
            CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => return None,
            CurrentSurfaceTexture::Validation => {
                return Some(Rendered::Failed("swapchain validation failed".to_owned()));
            }
        };

        let mut encoder = handle
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("present"),
            });
        state.surface.blitter.copy(
            &handle.device,
            &mut encoder,
            &state.target.view,
            &frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        handle.queue.submit(Some(encoder.finish()));
        frame.present();
        Some(Rendered::Presented)
    }

    /// Draws the frame the window asked for, and tells the core what became of it.
    fn redraw_requested(&mut self) {
        let started = std::time::Instant::now();
        if self.trace {
            println!(
                "trace: redraw requested, page {:?}",
                self.request
                    .as_ref()
                    .map(|request| request.page.saturating_add(1))
            );
        }
        let outcome = self.present();
        if self.trace {
            println!(
                "trace: present -> {} in {:?}",
                match &outcome {
                    None => "nothing to show".to_owned(),
                    Some(Rendered::Presented) => "presented".to_owned(),
                    Some(Rendered::Failed(why)) => format!("failed: {why}"),
                    Some(Rendered::Raster(_)) => "a raster".to_owned(),
                },
                started.elapsed()
            );
        }
        let Some(rendered) = outcome else {
            return;
        };
        if !self.acknowledged
            && let Some(token) = self.request.as_ref().map(|request| request.token)
        {
            self.acknowledged = true;
            self.dispatch(Command::RenderReady { token, rendered });
        }
    }

    /// Draws a page with `render-cpu` and puts the pixels on the surface.
    ///
    /// The tier-1 rasteriser inside a tier-2 host: the raster crosses as an image in a scene of
    /// its own, because what this program has to hand pixels to is a Vello surface and Vello
    /// draws an image as readily as a path. One copy per frame, which is what tier 1 costs
    /// everywhere and is paid here only on a page the device refused.
    fn on_the_processor(
        &mut self,
        request: &RenderRequest,
        target: TargetSpec,
        viewport: (u32, u32),
        highlight: &[[f32; 8]],
        panel: Option<&pdf_render::DisplayList>,
    ) -> Result<(), String> {
        let raster = CpuRasterizer::new()
            .rasterize(&request.list, target)
            .map_err(|error| error.to_string())?;
        let image = vello::peniko::ImageBrush::new(vello::peniko::ImageData {
            data: vello::peniko::Blob::from(raster.data),
            format: vello::peniko::ImageFormat::Rgba8,
            // `Raster` is straight alpha, which its own documentation states and which is what
            // the comparison harness and PNG both want.
            alpha_type: vello::peniko::ImageAlphaType::Alpha,
            width: raster.width,
            height: raster.height,
        });
        let mut scene = vello::Scene::new();
        scene.draw_image(&image, vello::kurbo::Affine::IDENTITY);
        draw_selection(&mut scene, highlight);
        draw_panel(&mut scene, panel, viewport)?;

        let Some(state) = self.state.as_mut() else {
            return Err("has no window".to_owned());
        };
        let handle = &self.context.devices[state.surface.dev_id];
        // Checked like every other render in this tree, though this scene is one image over one
        // rectangle and is the least likely thing on the device to run out of room. It is also
        // the *last* resort: were it to come back blank, the note above would say the processor
        // drew the page while the window showed black, which is precisely the failure this
        // session exists to remove.
        let mut bands = render_gpu::Bands::default();
        render_gpu::render_checked(
            &handle.device,
            &handle.queue,
            &mut state.renderer,
            &mut scene,
            &state.target.texture,
            &vello::RenderParams {
                base_color: vello::peniko::Color::WHITE,
                width: viewport.0,
                height: viewport.1,
                antialiasing_method: AaConfig::Area,
            },
            &mut bands,
        )
        .map_err(|error| error.to_string())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 1000.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("window creation"),
        );

        let size = window.inner_size();
        let surface = pollster::block_on(self.context.create_surface(
            window.clone(),
            size.width.max(1),
            size.height.max(1),
            wgpu::PresentMode::AutoVsync,
        ))
        .expect("surface creation");

        // **wgpu reports a device error to a handler, and the default one is silent here.**
        // Everything else in this program says what went wrong; a validation failure or a lost
        // device was the one thing that could stop the window updating without a word.
        self.context.devices[surface.dev_id]
            .device
            .on_uncaptured_error(Arc::new(|error| {
                eprintln!("note: the graphics device reported: {error}");
            }));

        let renderer = Renderer::new(
            &self.context.devices[surface.dev_id].device,
            RendererOptions {
                antialiasing_support: AaSupport {
                    area: true,
                    msaa8: false,
                    msaa16: false,
                },
                // One thread: shader compilation is on the startup path, and the default
                // heuristic spawns threads whose benefit here is unmeasured.
                num_init_threads: NonZeroUsize::new(1),
                ..Default::default()
            },
        )
        .expect("renderer creation");

        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        let target = Target::new(
            &self.context.devices[surface.dev_id].device,
            size.width.max(1),
            size.height.max(1),
        );
        self.state = Some(State {
            window,
            surface,
            renderer,
            target,
            bands: render_gpu::Bands::default(),
        });
        self.retitle();
        // The window's size is the first thing the core has been told about the viewport, and
        // it is what makes page one render.
        self.dispatch(Command::Resize {
            width: size.width.max(1),
            height: size.height.max(1),
            scale,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.state.is_none() {
            return;
        }
        // Every window event but the pointer's, which arrives faster than a person can read.
        // What this answers is the question a stuck window raises first: *is the program being
        // told anything at all?*
        if self.trace && !matches!(event, WindowEvent::CursorMoved { .. }) {
            println!("trace: window event {}", describe_window_event(&event));
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        ..
                    },
                ..
            } => {
                if matches!(logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    event_loop.exit();
                    return;
                }
                // The one key this program answers itself rather than by sending a command:
                // whether a panel is shown is chrome, and `viewer-core` has no opinion about
                // chrome by construction (rule 5).
                if matches!(logical_key.as_ref(), Key::Character("o")) {
                    self.panel.toggle();
                    self.resize_page();
                    return;
                }
                let Some(command) = key_command(&logical_key.as_ref()) else {
                    return;
                };
                self.dispatch(command);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                self.pointer_moved();
            }

            WindowEvent::MouseInput {
                state: element,
                button: MouseButton::Left,
                ..
            } => {
                if self.over_panel() {
                    // Answered once, on the press: a panel that acted on both ends of a click
                    // would follow a destination twice.
                    if element == ElementState::Pressed {
                        self.click_panel();
                    }
                    return;
                }
                self.dragging = element == ElementState::Pressed;
                self.dispatch(Command::Pointer {
                    at: self.on_page(self.cursor),
                    action: match element {
                        ElementState::Pressed => PointerAction::Pressed,
                        ElementState::Released => PointerAction::Released,
                    },
                });
            }

            WindowEvent::Resized(size) => {
                let scale = self.state.as_ref().map_or(1.0, |state| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a display's scale factor is a small ratio"
                    )]
                    let scale = state.window.scale_factor() as f32;
                    scale
                });
                if let Some(state) = self.state.as_mut() {
                    self.context.resize_surface(
                        &mut state.surface,
                        size.width.max(1),
                        size.height.max(1),
                    );
                    // A band count is an answer about a scene at a size, and the size just
                    // changed; keeping it would band a window that no longer needs it.
                    state.target = Target::new(
                        &self.context.devices[state.surface.dev_id].device,
                        size.width.max(1),
                        size.height.max(1),
                    );
                    state.bands.reset();
                }
                self.dispatch(Command::Resize {
                    width: size.width.saturating_sub(self.panel.inset(scale)).max(1),
                    height: size.height.max(1),
                    scale,
                });
            }

            WindowEvent::MouseWheel { delta, .. } => self.wheel(delta),

            WindowEvent::RedrawRequested => self.redraw_requested(),

            _ => {}
        }
    }
}

/// What a key press asks for, where it asks for anything.
///
/// One place rather than an arm apiece inside the event handler, because this is the whole of
/// this program's key bindings and a reader looking for them should find them together.
fn key_command(key: &Key<&str>) -> Option<Command> {
    Some(match *key {
        Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space) => {
            Command::GoTo(PageTarget::Next)
        }
        Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => Command::GoTo(PageTarget::Previous),
        Key::Named(NamedKey::Home) => Command::GoTo(PageTarget::First),
        Key::Named(NamedKey::End) => Command::GoTo(PageTarget::Last),
        Key::Character("+" | "=") => Command::Zoom(Zoom::In),
        Key::Character("-") => Command::Zoom(Zoom::Out),
        Key::Character("0") => Command::Zoom(Zoom::FitPage),
        Key::Character("a") => Command::Select(Selection::All),
        Key::Character("s") => Command::Save,
        // A page taller than the window: the scroll is in device pixels, so this is about a
        // fifteenth of a fitted A4 page and the same on any display.
        Key::Named(NamedKey::ArrowDown) => Command::Scroll { dx: 0.0, dy: 60.0 },
        Key::Named(NamedKey::ArrowUp) => Command::Scroll { dx: 0.0, dy: -60.0 },
        _ => return None,
    })
}

/// Receives what `wgpu`, `vello` and `naga` say about themselves.
///
/// Those three write to the `log` facade, and a facade with nothing behind it drops every record
/// — which is why a page that would not draw produced no output at all (ADR 0126). Twenty lines
/// rather than a logging framework: there is one destination, one format, and one filter, and a
/// configuration language for those would be longer than this.
struct Speak {
    /// The most detailed level to print, from `PDFVIEWER_LOG`.
    level: log::LevelFilter,
}

impl log::Log for Speak {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{}: {}: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Installs [`Speak`], at `PDFVIEWER_LOG`'s level or `warn`.
///
/// `warn` by default because that is the level at which a graphics driver says something is
/// wrong; `PDFVIEWER_LOG=debug` is what to set when *nothing* is wrong and the question is what
/// the device is doing.
fn speak_up() {
    let level = match std::env::var("PDFVIEWER_LOG").unwrap_or_default().as_str() {
        "error" => log::LevelFilter::Error,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Warn,
    };
    // A failure here means a logger is already installed, which nothing else in this program
    // does; there is no second thing to try and nothing is lost by carrying on quietly.
    if log::set_boxed_logger(Box::new(Speak { level })).is_ok() {
        log::set_max_level(level);
    }
}

/// One line naming a window event, for `--trace`.
fn describe_window_event(event: &WindowEvent) -> String {
    match event {
        WindowEvent::RedrawRequested => "redraw requested".to_owned(),
        WindowEvent::Resized(size) => format!("resized to {}x{}", size.width, size.height),
        WindowEvent::KeyboardInput { event, .. } => {
            format!("key {:?} {:?}", event.logical_key, event.state)
        }
        WindowEvent::MouseInput { state, button, .. } => format!("mouse {button:?} {state:?}"),
        WindowEvent::CloseRequested => "close requested".to_owned(),
        other => format!("{other:?}"),
    }
}

/// One line naming a command, for `--trace`.
///
/// The command's own `Debug` would print a document's bytes and a raster's pixels, which is not a
/// line. This is what a person following a page turn needs to see.
fn describe_command(command: &Command) -> String {
    match command {
        Command::Open { id, bytes, .. } => format!("open {:?}, {} bytes", id, bytes.len()),
        Command::Close(id) => format!("close {id:?}"),
        Command::Tick { millis } => format!("tick {millis} ms"),
        Command::Focus(id) => format!("focus {id:?}"),
        Command::Resize {
            width,
            height,
            scale,
        } => format!("resize {width}x{height} at {scale}"),
        Command::GoTo(target) => format!("go to {target:?}"),
        Command::Zoom(zoom) => format!("zoom {zoom:?}"),
        Command::Scroll { dx, dy } => format!("scroll {dx} {dy}"),
        Command::SetGroup { group, on } => format!("layer {group:?} {on}"),
        Command::Pointer { at, action } => format!("pointer {action:?} at {at:?}"),
        Command::Select(what) => format!("select {what:?}"),
        Command::Edit(edit) => format!("edit {edit:?}"),
        Command::Undo => "undo".to_owned(),
        Command::Redo => "redo".to_owned(),
        Command::Save => "save".to_owned(),
        Command::Supply { purpose, bytes } => format!(
            "supply {purpose:?}, {}",
            bytes
                .as_ref()
                .map_or_else(|| "declined".to_owned(), |b| format!("{} bytes", b.len()))
        ),
        Command::RenderReady { token, .. } => format!("render ready {token:?}"),
    }
}

/// One line naming an event, for `--trace`.
fn describe_event(event: &Event) -> String {
    match event {
        Event::Opened { pages, .. } => format!("opened, {pages} page(s)"),
        Event::OpenFailed { reason, .. } => format!("open failed: {reason}"),
        Event::PasswordRequired { .. } => "a password is required".to_owned(),
        Event::Closed(_) => "closed".to_owned(),
        Event::PageChanged { index, of, .. } => format!("page {} of {of}", index.saturating_add(1)),
        Event::NeedsRender(request) => format!(
            "needs render: page {}, {}x{}, {} command(s), {:?}",
            request.page.saturating_add(1),
            request.target.width,
            request.target.height,
            request.list.command_count(),
            request.token
        ),
        Event::Damage(_) => "damage".to_owned(),
        Event::OpenUri { uri, .. } => format!("open uri {uri}"),
        Event::NeedsFile { name, .. } => format!("needs file {name}"),
        Event::Transition { .. } => "a transition".to_owned(),
        Event::Dirty { dirty, .. } => format!("dirty {dirty}"),
        Event::Saved { bytes, .. } => format!("saved, {} bytes", bytes.len()),
        Event::Reported { page, notes, .. } => {
            format!("reported about page {page:?}: {}", notes.join("; "))
        }
    }
}

/// A window position as the device pixels `viewer-core` speaks in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a window coordinate is a small number of pixels"
)]
fn at(cursor: (f64, f64)) -> (f32, f32) {
    (cursor.0 as f32, cursor.1 as f32)
}

/// Lays the selection's shapes over the page.
///
/// The quadrilaterals arrive from `viewer-core` in device pixels of this window, so nothing here
/// composes a transform: that is the whole point of chrome crossing as geometry rather than as
/// pixels. Drawn with `Multiply`, which darkens what is under it and leaves the glyphs readable —
/// the behaviour a person expects of a highlighter and the one a plain alpha blend does not give.
fn draw_selection(scene: &mut vello::Scene, quads: &[[f32; 8]]) {
    if quads.is_empty() {
        return;
    }
    // A blue with no theme behind it. A native host asks its platform for this colour; this one
    // has nobody to ask, and a hard-coded value that says so is better than one that pretends.
    let colour = vello::peniko::Color::from_rgba8(140, 180, 255, 255);
    let brush = vello::peniko::Brush::Solid(colour);
    for quad in quads {
        let mut path = vello::kurbo::BezPath::new();
        for (index, corner) in quad.chunks_exact(2).enumerate() {
            let point = vello::kurbo::Point::new(f64::from(corner[0]), f64::from(corner[1]));
            if index == 0 {
                path.move_to(point);
            } else {
                path.line_to(point);
            }
        }
        path.close_path();
        scene.fill(
            vello::peniko::Fill::NonZero,
            vello::kurbo::Affine::IDENTITY,
            &brush,
            None,
            &path,
        );
    }
}

/// Puts the panel's display list on top of the page's scene.
///
/// The same translation `viewer-core`'s own render takes — `render_gpu::build_scene` — at an
/// identity transform, because the panel's list is already in the window's device pixels. That
/// is what makes the panel a display list rather than a pile of Vello calls: the CPU backend
/// draws it identically, which is what a `--cpu` run and a page the device refuses both need.
fn draw_panel(
    scene: &mut vello::Scene,
    panel: Option<&pdf_render::DisplayList>,
    viewport: (u32, u32),
) -> Result<(), String> {
    let Some(list) = panel else {
        return Ok(());
    };
    let target = TargetSpec {
        width: viewport.0,
        height: viewport.1,
        transform: Transform::IDENTITY,
    };
    let drawn = render_gpu::build_scene(list, target, &render_gpu::SoftMaskRasters::none())
        .map_err(|error| format!("cannot draw its own panel: {error}"))?;
    scene.append(&drawn, None);
    Ok(())
}

/// Reads a password from the terminal, or `None` if the person cancelled with an empty line.
fn ask_password(name: &str) -> Option<String> {
    eprint!("{name} needs a password (empty line to give up): ");
    std::io::stderr().flush().ok()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok()?;
    let password = line.trim_end_matches(['\r', '\n']).to_owned();
    (!password.is_empty()).then_some(password)
}

/// Builds the scene and renders it into the surface's texture.
fn draw(
    context: &RenderContext,
    state: &mut State,
    request: &RenderRequest,
    target: TargetSpec,
    viewport: (u32, u32),
    highlight: &[[f32; 8]],
    panel: Option<&pdf_render::DisplayList>,
) -> Result<(), String> {
    let (width, height) = viewport;
    let handle = &context.devices[state.surface.dev_id];
    let list = &request.list;

    // §11.5's soft masks are rendered first, each into a texture of its own: a mask is a
    // transparency group evaluated at device resolution, so it cannot be part of the scene that
    // uses it. Costs nothing on a page with no mask.
    let masks = render_gpu::evaluate_soft_masks(
        &handle.device,
        &handle.queue,
        &mut state.renderer,
        list,
        target,
    )
    .map_err(|_| "has a soft mask this build cannot evaluate".to_owned())?;

    // The same translation the headless tests exercise, so what the window shows cannot drift
    // from what CI checks.
    let mut scene = render_gpu::build_scene(list, target, &masks)
        .map_err(|_| "contains content this build cannot draw".to_owned())?;
    draw_selection(&mut scene, highlight);
    draw_panel(&mut scene, panel, viewport)?;

    // `render_gpu::render_checked` rather than `Renderer::render_to_texture`: the latter returns
    // `Ok` over a target the device left blank, which is the black page this session was reported
    // (ADR 0127). The error carries its own sentence, so what reaches the person names the buffer
    // that ran out rather than saying only that something went wrong.
    render_gpu::render_checked(
        &handle.device,
        &handle.queue,
        &mut state.renderer,
        &mut scene,
        &state.target.texture,
        &vello::RenderParams {
            base_color: vello::peniko::Color::WHITE,
            width,
            height,
            antialiasing_method: AaConfig::Area,
        },
        &mut state.bands,
    )
    .map_err(|error| error.to_string())
}
