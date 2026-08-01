//! The viewer: opens a PDF and shows it.
//!
//! ``text
//! cargo run --release -p viewer-ui --bin pdf-viewer -- document.pdf
//! ``
//!
//! Arrows, Page Up and Down or Space turn pages, Home and End jump, `+` and `-` zoom, `a`
//! selects the whole page and dragging selects part of it, Escape quits. The window title shows
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

use pdf_render::{TargetSpec, Transform};
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu::CurrentSurfaceTexture;
use vello::{AaConfig, AaSupport, Renderer, RendererOptions, wgpu};
use viewer_core::{
    Answer, Command, DocumentId, Event, PageTarget, PointerAction, Purpose, Query, RenderRequest,
    Rendered, Selection, Viewer, Zoom,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

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
    for argument in std::env::args_os().skip(1) {
        if argument == "--no-sandbox" {
            sandbox = false;
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
        cursor: (0.0, 0.0),
        dragging: false,
        dirty: false,
        attempts: 0,
        context: RenderContext::new(),
        state: None,
    };
    app.dispatch(Command::Open {
        id: DOCUMENT,
        bytes,
        password: None,
    });

    let event_loop = EventLoop::new().expect("an event loop requires a display server");
    // Redraw on request rather than continuously: a document viewer is idle almost all the time,
    // and a spinning loop would drain a battery for nothing.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app).expect("event loop failed");
}

/// What the program does when it is given nothing to open.
fn usage() {
    eprintln!("usage: pdf-viewer [--no-sandbox] <document.pdf>");
    eprintln!();
    eprintln!("Arrows, Page Up/Down or Space turn pages; Home and End jump; + and - zoom;");
    eprintln!("drag to select text, a selects the page, Escape quits.");
    eprintln!();
    eprintln!("  --no-sandbox  decode JBIG2 and JPEG 2000 images in this process rather than");
    eprintln!("                in a confined worker. Faster by a process spawn and a pipe");
    eprintln!("                round trip; appropriate only for documents you trust.");
}

/// How many passwords a person is asked for before the program gives up.
///
/// §7.6.4.1 states no limit — it says a processor tries the empty password and then prompts —
/// so this is a choice about a terminal rather than about the clause, and an empty line cancels
/// before it is reached.
const PASSWORD_ATTEMPTS: usize = 3;

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
    /// Whether the button is down, which is what separates a move from a drag.
    dragging: bool,
    /// Whether anything a person did is unsaved.
    dirty: bool,
    /// How many passwords have been asked for.
    attempts: usize,
    context: RenderContext,
    state: Option<State>,
}

/// The window, and the GPU objects that belong to it.
struct State {
    window: Arc<Window>,
    surface: RenderSurface<'static>,
    renderer: Renderer,
}

impl App {
    /// Hands a command to the core and deals with everything that comes back.
    ///
    /// A queue rather than recursion, because reacting to an event may produce a command — a
    /// password supplied, a file read — and a chain of those is a loop rather than a stack.
    fn dispatch(&mut self, command: Command) {
        let mut queue = VecDeque::from([command]);
        while let Some(command) = queue.pop_front() {
            let events: Vec<Event> = self.viewer.handle(command).collect();
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
    /// Returns whether the core should be told, which is false only where the swapchain gave
    /// this frame back: a redraw that never reached a screen is not a frame that was presented,
    /// and telling the core otherwise would leave the window blank until something else changed.
    fn present(&mut self) -> bool {
        let Some(request) = self.request.clone() else {
            return false;
        };
        // Where the page sits in the window: the core centres it and scrolls it, and the host
        // draws it there by composing that offset into the target's own transform.
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        let Some(state) = self.state.as_mut() else {
            return false;
        };
        let (width, height) = (state.surface.config.width, state.surface.config.height);
        let target = TargetSpec {
            width,
            height,
            transform: request
                .target
                .transform
                .then(Transform::translate(origin.0, origin.1)),
        };

        // Interactive chrome crosses as geometry, not pixels: the core hands over the shapes and
        // this host draws them in its own colour. A native one would use macOS's selection
        // colour, KDE's accent or the Windows highlight brush; this one has no theme to ask, so
        // it picks a blue and says so.
        let highlight = match self.viewer.query(Query::Selection) {
            Answer::Selected(selection) => selection.quads,
            _ => Vec::new(),
        };
        let handle = &self.context.devices[state.surface.dev_id];
        if let Err(problem) = draw(
            &self.context,
            state,
            &request,
            target,
            (width, height),
            &highlight,
        ) {
            println!("note: page {}: {problem}", request.page.saturating_add(1));
            return true;
        }

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
                return false;
            }
            CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => return false,
            CurrentSurfaceTexture::Validation => {
                println!("note: swapchain validation failed");
                return false;
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
            &state.surface.target_view,
            &frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        handle.queue.submit(Some(encoder.finish()));
        frame.present();
        true
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
        self.state = Some(State {
            window,
            surface,
            renderer,
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
                let Some(command) = key_command(&logical_key.as_ref()) else {
                    return;
                };
                self.dispatch(command);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                self.dispatch(Command::Pointer {
                    at: at(self.cursor),
                    action: if self.dragging {
                        PointerAction::Dragged
                    } else {
                        PointerAction::Moved
                    },
                });
                // §12.5.6.5's activation region, asked at pointer speed — which is why it is a
                // query rather than a command with an event coming back.
                if let (Answer::Link(over), Some(state)) = (
                    self.viewer.query(Query::LinkAt(at(self.cursor))),
                    self.state.as_ref(),
                ) {
                    state.window.set_cursor(if over {
                        winit::window::CursorIcon::Pointer
                    } else {
                        winit::window::CursorIcon::Default
                    });
                }
            }

            WindowEvent::MouseInput {
                state: element,
                button: MouseButton::Left,
                ..
            } => {
                self.dragging = element == ElementState::Pressed;
                self.dispatch(Command::Pointer {
                    at: at(self.cursor),
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
                }
                self.dispatch(Command::Resize {
                    width: size.width.max(1),
                    height: size.height.max(1),
                    scale,
                });
            }

            WindowEvent::RedrawRequested => {
                if !self.present() {
                    return;
                }
                if !self.acknowledged
                    && let Some(token) = self.request.as_ref().map(|request| request.token)
                {
                    self.acknowledged = true;
                    self.dispatch(Command::RenderReady {
                        token,
                        rendered: Rendered::Presented,
                    });
                }
            }

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
        // A page taller than the window: the scroll is in device pixels, so this is about a
        // fifteenth of a fitted A4 page and the same on any display.
        Key::Named(NamedKey::ArrowDown) => Command::Scroll { dx: 0.0, dy: 60.0 },
        Key::Named(NamedKey::ArrowUp) => Command::Scroll { dx: 0.0, dy: -60.0 },
        _ => return None,
    })
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
) -> Result<(), &'static str> {
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
    .map_err(|_| "has a soft mask this build cannot evaluate")?;

    // The same translation the headless tests exercise, so what the window shows cannot drift
    // from what CI checks.
    let mut scene = render_gpu::build_scene(list, target, &masks)
        .map_err(|_| "contains content this build cannot draw")?;
    draw_selection(&mut scene, highlight);

    state
        .renderer
        .render_to_texture(
            &handle.device,
            &handle.queue,
            &scene,
            &state.surface.target_view,
            &vello::RenderParams {
                base_color: vello::peniko::Color::WHITE,
                width,
                height,
                antialiasing_method: AaConfig::Area,
            },
        )
        .map_err(|_| "could not be rendered")
}
