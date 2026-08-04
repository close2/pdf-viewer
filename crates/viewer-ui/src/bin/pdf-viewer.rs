//! The viewer: opens a PDF and shows it.
//!
//! ``text
//! cargo run --release -p viewer-ui --bin pdf-viewer -- document.pdf
//! ``
//!
//! `--page N` opens at a page. Arrows, Page Up and Down or Space turn pages, Home and End jump,
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

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line application: stdout is a reporting channel and a panic on a \
              missing display is the intended failure"
)]

use std::collections::VecDeque;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command as DrawCommand, FillRule, Paint, Path, PathCommand, Point,
    Rasterizer as _, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use render_quorra::{PresentFrame, QuorraPresenter};
use viewer_core::{
    Answer, Command, DocumentId, Event, PageTarget, PointerAction, Purpose, Query, RenderRequest,
    Rendered, Selection, Viewer, Zoom,
};
use viewer_ui::chrome::{About, Chrome, Content, Hit, Sidebar, Tab};
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

/// How far a touchpad must be dragged under Ctrl for one zoom step.
///
/// A choice, not a derivation: a notch of a mouse wheel is one step by construction and a
/// touchpad reports a stream of pixels instead, so something has to say how many of them a notch
/// is worth. Fifty is about a finger's width on this machine's touchpad and gives roughly the
/// same number of steps per gesture as the wheel does per flick.
const WHEEL_ZOOM_PIXELS: f32 = 50.0;

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
        control: false,
        pinch: 0.0,
        dirty: false,
        attempts: 0,
        chrome: match Chrome::new() {
            Ok(chrome) => Some(chrome),
            Err(problem) => {
                eprintln!("note: no panel: {problem}");
                None
            }
        },
        panel: Sidebar::default(),
        about: About::default(),
        outline: pdf_model::outline::Outline::default(),
        attachments: Vec::new(),
        pages: Vec::new(),
        information: pdf_model::metadata::Information::default(),
        metadata_stream: false,
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
    eprintln!("o shows the sidebar — the outline, the layers and the embedded files;");
    eprintln!("? shows the third-party notices; drag to select text, a selects the page,");
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
    /// Whether Ctrl is held, which is what separates a wheel scroll from a wheel zoom.
    ///
    /// `winit` reports a modifier change as its own event and puts nothing in the wheel's, so a
    /// host that wants to know has to remember. Ctrl + wheel is a convention rather than a
    /// clause, and it is the one every desktop viewer has converged on. ADR 0166.
    control: bool,
    /// A touchpad's accumulated pixels, spent one zoom step at a time.
    ///
    /// A wheel notch arrives as a line and a touchpad's pinch as a stream of pixels; sixteen
    /// pixels is one of this program's own text rows and means nothing to a magnification, so
    /// the pixels are counted up and a step taken per `WHEEL_ZOOM_PIXELS` rather than per event.
    pinch: f32,
    /// Whether anything a person did is unsaved.
    dirty: bool,
    /// How many passwords have been asked for.
    attempts: usize,
    /// The fonts this program draws its own text with, or why it cannot.
    ///
    /// An `Option` because a build whose compiled-in faces will not parse must still show the
    /// document: the panel is chrome and the page is the point. The refusal is printed once.
    chrome: Option<Chrome>,
    /// The three lists a document keeps about itself, as this program draws them.
    panel: Sidebar,
    /// `/NOTICE`, over the page, which is the About panel the owner asked for.
    about: About,
    /// §12.3.3's outline and §7.11.4's embedded files, taken once when the document opened.
    ///
    /// Copied out of the queries rather than asked for per frame, and not for speed:
    /// `Answer::Outline` borrows the viewer, and a panel that is about to send it a command
    /// cannot be holding a borrow of it. Both are properties of an immutable document that no
    /// edit reaches, so a copy taken at open cannot go stale — which is exactly not true of
    /// §8.11's layers, whose whole point is that a click changes them, so those are asked for
    /// every time.
    outline: pdf_model::outline::Outline,
    /// §7.11.4's embedded files, likewise.
    attachments: Vec<pdf_model::attachment::Attachment>,
    /// §14.3.3's Table 349 and whether §14.3.2's stream exists, likewise.
    information: pdf_model::metadata::Information,
    /// Whether the catalog names a `/Metadata` stream.
    metadata_stream: bool,
    /// §12.3.4's tab: one entry per page, with its label and its decoded thumbnail.
    ///
    /// **Empty until that tab is first shown**, which is principle 2 with a clause behind it:
    /// §12.3.4's NOTE says thumbnails "are not required, and can be included for some pages and
    /// not for others", so building this list means decoding every miniature a document carries,
    /// and a document opens at a page rather than at a contact sheet. Filled once and kept,
    /// because a thumbnail is a property of an immutable document — the same argument the
    /// outline and the attachments are cached under, and exactly not the layers'.
    pages: Vec<viewer_ui::chrome::Page>,
    state: Option<State>,
}

/// The window, and the presenter that owns its surface.
struct State {
    window: Arc<Window>,
    /// quorra's device holds the surface; one call draws and presents a frame,
    /// and a refused frame is a typed error naming what refused — the banding
    /// machinery, the owned intermediate texture and the blitter of the Vello
    /// host all fell away with the backend that needed them.
    presenter: QuorraPresenter,
    /// The surface size in device pixels, updated on `WindowEvent::Resized`.
    size: (u32, u32),
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
        Some((state.size.0, state.size.1, scale))
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
        let layers = self.layers();
        Some(
            self.panel
                .draw(chrome, self.content(&layers), height, scale),
        )
    }

    /// Takes the two lists a document cannot change, once, when it opens.
    ///
    /// §12.3.3's outline and §7.11.4's embedded files. `Answer::Outline` borrows the viewer, so
    /// what the panel holds is a copy — see the fields' own note — and both are properties of an
    /// immutable document, so a copy cannot go stale. §8.11's layers are *not* here for exactly
    /// that reason.
    fn gather(&mut self) {
        if let Answer::Outline(outline) = self.viewer.query(Query::Outline) {
            self.outline = outline.clone();
        }
        if let Answer::Attachments(files) = self.viewer.query(Query::Attachments) {
            self.attachments = files;
        }
        if let Answer::Properties {
            information,
            metadata_stream,
        } = self.viewer.query(Query::Properties)
        {
            self.information = information;
            self.metadata_stream = metadata_stream;
        }
        // §12.2 names XMP's `dc:title` and this program reads none; see `named`. Said once,
        // and only where it could matter — a document that asks for its title and carries the
        // stream the clause points at.
        if self.metadata_stream && self.display_doc_title() {
            println!(
                "note: this document asks for its title in the title bar (§12.2's \
                 /DisplayDocTitle), which names XMP's dc:title; this program reads no XMP and \
                 shows §14.3.3's /Info /Title instead"
            );
        }
        self.retitle();
        self.obey_page_mode();
        let layers = self.layers().len();
        if !self.outline.items.is_empty() || !self.attachments.is_empty() || layers > 0 {
            println!(
                "{}: {} outline item(s), {layers} layer entr(ies), {} embedded file(s) — \
                 press o for the panel",
                self.title,
                self.outline.visible_count(),
                self.attachments.len()
            );
        }
    }

    /// Writes an extracted embedded file beside the document.
    ///
    /// **Rule 2 in the other direction**: the core produced the bytes and the host decides where
    /// they go. Beside the open document and nowhere else, which is the mirror of the policy
    /// §12.7.6.4's import takes — and the file's own name is a string *the document wrote*, so
    /// only its last component is used and a name that is a path, is empty, or is `..` is
    /// refused rather than followed. §7.11.4 states no policy at all, because a policy is a
    /// property of the processor.
    fn write_extracted(&self, name: &str, bytes: &[u8]) {
        let stem = std::path::Path::new(name).file_name();
        let Some(stem) = stem.filter(|stem| !stem.is_empty()) else {
            println!("note: the embedded file's name {name:?} is not a file name");
            return;
        };
        let path = self.directory.clone().unwrap_or_default().join(stem);
        match std::fs::write(&path, bytes) {
            Ok(()) => println!("extracted {} bytes to {}", bytes.len(), path.display()),
            Err(error) => println!("note: cannot write {}: {error}", path.display()),
        }
    }

    /// Table 29: opens the panel the document asks for, and says what it cannot do.
    ///
    /// §7.7.2's `/PageMode` is "how the document shall be displayed when opened", and until the
    /// hundred-and-seventieth session this program had no panel for any of its answers to name.
    /// **Four of the six it can now obey** — `UseThumbs` joined the other three in the
    /// two-hundred-and-sixty-sixth session, when §12.3.4's panel arrived — and what is left is
    /// `UseNone`, which asks for nothing, and `FullScreen`, which names chrome that does not
    /// exist here and is said once rather than ignored: a document asking for something and
    /// getting silence is trap 5 in an interface.
    ///
    /// `/PageLayout` likewise. This window shows one page at a time, which is Table 29's own
    /// default, so a document stating `SinglePage` — 24 of the corpus's 43 — is answered exactly
    /// and says nothing.
    fn obey_page_mode(&mut self) {
        use pdf_model::viewer_preferences::{PageLayout, PageMode};
        let Answer::Opening(opening) = self.viewer.query(Query::Opening) else {
            return;
        };
        match opening.mode {
            PageMode::UseNone => {}
            PageMode::UseOutlines => self.panel.show(Tab::Contents),
            PageMode::UseOptionalContent => self.panel.show(Tab::Layers),
            PageMode::UseAttachments => self.panel.show(Tab::Files),
            PageMode::UseThumbs => self.panel.show(Tab::Pages),
            PageMode::FullScreen => println!(
                "note: this document asks to open full screen (§7.7.2), which is chrome this \
                 program does not have"
            ),
        }
        if opening.layout != PageLayout::SinglePage {
            println!(
                "note: this document asks for the {:?} page layout (§7.7.2); this window shows \
                 one page at a time",
                opening.layout
            );
        }
    }

    /// The About card's display list for this frame, or `None` when it is not shown.
    fn about_list(&self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let chrome = self.chrome.as_ref()?;
        if !self.about.shown {
            return None;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        Some(self.about.draw(chrome, NOTICE, width, height, scale))
    }

    /// §8.11.4.3's `/Order`, asked for fresh.
    ///
    /// Unlike the outline and the attachments this is *not* cached: a click on a layer's switch
    /// changes it, so a copy taken when the document opened would be the one thing on the panel
    /// that lies.
    fn layers(&self) -> Vec<viewer_core::Layer> {
        match self.viewer.query(Query::Layers) {
            Answer::Layers(layers) => layers,
            _ => Vec::new(),
        }
    }

    /// The three lists, gathered for one call into the sidebar.
    fn content<'a>(&'a self, layers: &'a [viewer_core::Layer]) -> Content<'a> {
        Content {
            outline: &self.outline,
            layers,
            attachments: &self.attachments,
            information: &self.information,
            metadata_stream: self.metadata_stream,
            pages: &self.pages,
        }
    }

    /// Builds §12.3.4's page list, once, the first time its tab is shown.
    ///
    /// Called from `present`, which is the one place that runs before the panel is drawn and
    /// holds `&mut self`. A document with no thumbnails at all still gets a list — the rows are
    /// its pages, and §12.3.4's NOTE is why a page without one is still a page.
    fn ensure_pages(&mut self) {
        if !self.panel.shows_pages() || !self.pages.is_empty() {
            return;
        }
        let Answer::Count(count) = self.viewer.query(Query::PageCount) else {
            return;
        };
        self.pages = (0..count)
            .map(|index| {
                let label = match self.viewer.query(Query::PageLabel(index)) {
                    Answer::Label(label) => label,
                    _ => format!("Page {}", index.saturating_add(1)),
                };
                let thumbnail = match self.viewer.query(Query::Thumbnail(index)) {
                    Answer::Thumbnail(thumbnail) => Some(thumbnail.image),
                    _ => None,
                };
                viewer_ui::chrome::Page { label, thumbnail }
            })
            .collect();
    }

    /// What the pointer moving does: the panel's highlight, or the page's §12.5.5 appearance.
    ///
    /// Only one of the two, and never both: a hover highlight in the panel and a rollover
    /// appearance on the page are both answers to "what is under the pointer", and answering
    /// both would leave an annotation lit up behind a panel.
    fn pointer_moved(&mut self) {
        if self.about.shown {
            return;
        }
        let scale = self.window().map_or(1.0, |(_, _, scale)| scale);
        let layers = self.layers();
        // The struct is written out here rather than built by `content`: `self.panel` is
        // borrowed mutably, and only a *field* borrow of the other three is disjoint from it.
        let moved = self.panel.hover(
            at(self.cursor),
            Content {
                outline: &self.outline,
                layers: &layers,
                attachments: &self.attachments,
                information: &self.information,
                metadata_stream: self.metadata_stream,
                pages: &self.pages,
            },
            scale,
        );
        drop(layers);
        if moved {
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
        // The outline and the attachments are fields rather than queries for exactly this:
        // `Sidebar::click` produces a command for the viewer, and an `Answer` borrowing it would
        // still be alive. The layers are queried and the answer is *owned*, so the borrow ends
        // before the command goes out.
        let layers = self.layers();
        let hit = self.panel.click(
            at(self.cursor),
            Content {
                outline: &self.outline,
                layers: &layers,
                attachments: &self.attachments,
                information: &self.information,
                metadata_stream: self.metadata_stream,
                pages: &self.pages,
            },
            scale,
        );
        drop(layers);
        match hit {
            Some(Hit::Activate(object)) => self.dispatch(Command::Activate(object)),
            Some(Hit::Extract(name)) => self.dispatch(Command::Extract { name }),
            // §8.11.2.2: switching a group re-decides what the page draws, so this goes to the
            // core and comes back as a render rather than as a repaint of the panel.
            Some(Hit::SetGroup { group, on }) => self.dispatch(Command::SetGroup { group, on }),
            // §12.3.4: a click on a page's miniature shows that page. A page index rather than
            // a destination — the thumbnail *is* the page, so there is nothing to resolve.
            Some(Hit::GoTo(page)) => self.dispatch(Command::GoTo(PageTarget::Index(page))),
            Some(Hit::Redraw) => self.redraw(),
            Some(Hit::Nothing) | None => {}
        }
    }

    /// A wheel notch: the About card, the panel's list, or the page — and under Ctrl, a zoom.
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
        if self.about.shown {
            let Some((_, height, scale)) = self.window() else {
                return;
            };
            self.about.scroll(by / scale, NOTICE, height, scale);
            self.redraw();
            return;
        }
        // Ctrl is a magnification of the *page*, and the sidebar has no scale to change — so a
        // notch over the sidebar still zooms the page, with **no anchor**: there is no point of
        // the page under the pointer to hold, and `None` is the core's word for that. A step per
        // notch, and a step per `WHEEL_ZOOM_PIXELS` of a touchpad — the sixteen-pixels-a-line
        // conversion above is a distance on a list and says nothing about a magnification.
        if self.control {
            let whole = match delta {
                winit::event::MouseScrollDelta::LineDelta(_, lines) => {
                    self.pinch = 0.0;
                    lines.trunc()
                }
                winit::event::MouseScrollDelta::PixelDelta(position) => {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a scroll delta in pixels, which is tens"
                    )]
                    let pixels = position.y as f32;
                    self.pinch += pixels;
                    let whole = (self.pinch / WHEEL_ZOOM_PIXELS).trunc();
                    self.pinch -= whole * WHEEL_ZOOM_PIXELS;
                    whole
                }
            };
            // `ZOOM_RANGE` spans 0.02 to 64, which is thirty-six steps of 1.25 end to end, so a
            // bound of sixty-four cannot hide a magnification anybody could have reached — it is
            // there because a `f32` cast saturates and a device reporting nonsense would
            // otherwise be a loop of two billion commands.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "clamped to ±64 on the line above"
            )]
            let steps = whole.clamp(-64.0, 64.0) as i32;
            let zoom = if steps > 0 { Zoom::In } else { Zoom::Out };
            let at = (!self.over_panel()).then(|| self.on_page(self.cursor));
            for _ in 0..steps.unsigned_abs() {
                self.dispatch(Command::Zoom { zoom, at });
            }
            return;
        }
        if self.over_panel() {
            let Some((_, height, scale)) = self.window() else {
                return;
            };
            let layers = self.layers();
            self.panel.scroll(
                by / scale,
                Content {
                    outline: &self.outline,
                    layers: &layers,
                    attachments: &self.attachments,
                    information: &self.information,
                    metadata_stream: self.metadata_stream,
                    pages: &self.pages,
                },
                height,
                scale,
            );
            drop(layers);
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
                self.gather();
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
            Event::Extracted { name, bytes, .. } => self.write_extracted(&name, &bytes),
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
                .set_title(&format!("{mark}{} — {}", self.named(), self.caption));
        }
    }

    /// What the title bar calls the document — §12.2's `/DisplayDocTitle`.
    ///
    /// Table 147: "[a] flag specifying whether the window's title bar should display the
    /// document title taken from the `dc:title` entry of the XMP metadata stream … If false, the
    /// title bar should instead display the name of the PDF file containing the document."
    ///
    /// **A documented departure, measured.** The entry names XMP and this program reads none, so
    /// where a document asks for its title it gets §14.3.3's `/Info /Title` instead — which is
    /// the same fact in the place PDF kept it before PDF 2.0, and Table 349's own NOTE 1 says as
    /// much: "[t]he `dc:title` entry in the document's metadata stream **can be used to
    /// represent** the document's title." The two can disagree and the standard ranks them
    /// nowhere, so the substitution is a choice and is said out loud once per document that
    /// carries both. Of the 22 corpus documents setting the flag, 8 state an `/Info /Title` and
    /// 18 carry a metadata stream.
    fn named(&self) -> &str {
        let Some(title) = self.information.title.as_deref().filter(|t| !t.is_empty()) else {
            return &self.title;
        };
        if self.display_doc_title() {
            title
        } else {
            &self.title
        }
    }

    /// Table 147's `/DisplayDocTitle`, **default false**.
    fn display_doc_title(&self) -> bool {
        matches!(
            self.viewer.query(Query::Preferences),
            Answer::Preferences(preferences) if preferences.display_doc_title
        )
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
    /// The selection's shapes, in the window's own pixels, or `None` when nothing is selected.
    ///
    /// Interactive chrome crosses as geometry, not pixels: the core hands over the shapes and this
    /// host draws them in its own colour. A native one would use macOS's selection colour, KDE's
    /// accent or the Windows highlight brush; this one has no theme to ask, so it picks a blue and
    /// says so.
    fn selection_list(
        &self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let mut quads = match self.viewer.query(Query::Selection) {
            Answer::Selected(selection) => selection.quads,
            _ => Vec::new(),
        };
        // The quads are device pixels of the *page's* viewport, which begins where the panel
        // ends. One addition here rather than a second coordinate space in the core.
        for quad in &mut quads {
            for x in quad.iter_mut().step_by(2) {
                *x += edge;
            }
        }
        if self.trace {
            // The number every part of `doc/todo/13` turned on: the frame the compositor refused
            // was 63 quads, and a present cost 1.9 ms a quad before it. Kept in the tree so that
            // a selection's cost stays visible rather than being rediscovered.
            eprintln!("trace: SELECTION quads {}", quads.len());
        }
        highlight_list(&quads, width, height)
    }

    fn present(&mut self) -> Option<Rendered> {
        // §12.3.4's list is built here and nowhere else: this is the one place that holds
        // `&mut self` and runs before the panel is drawn.
        self.ensure_pages();
        let request = self.request.clone()?;
        // Where the page sits in the window: the core centres it and scrolls it, and the host
        // draws it there by composing that offset into the target's own transform.
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        let (width, height) = {
            let state = self.state.as_ref()?;
            state.size
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

        let chrome = Overlays {
            panel: self.panel_list(height),
            about: self.about_list(width, height),
        };
        let selection = self.selection_list(edge, width, height);
        // Selection first (it belongs to the page), then the sidebar, then the
        // modal card on top — the same order the Vello host drew them in.
        let mut overlays: Vec<&pdf_render::DisplayList> = Vec::new();
        overlays.extend(selection.as_ref());
        overlays.extend(chrome.panel.as_ref());
        overlays.extend(chrome.about.as_ref());

        let state = self.state.as_mut()?;
        let drawn = if self.processor {
            Err("was not asked, because --cpu".to_owned())
        } else {
            match state.presenter.present(PresentFrame {
                width,
                height,
                page: Some((&request.list, target)),
                raster: None,
                overlays: &overlays,
            }) {
                Ok(()) => Ok(()),
                // Swapchain states are events, not failures: nothing was presented,
                // nothing is stale, and the processor cannot help a window that is
                // not presentable — so these return rather than fall back.
                Err(render_quorra::QuorraRasterError::Render(
                    quorra_gpu::RenderError::SurfaceUnavailable { reason },
                )) => {
                    return match reason {
                        quorra_gpu::SurfaceProblem::Outdated | quorra_gpu::SurfaceProblem::Lost => {
                            state.window.request_redraw();
                            None
                        }
                        quorra_gpu::SurfaceProblem::Timeout
                        | quorra_gpu::SurfaceProblem::Occluded => None,
                        quorra_gpu::SurfaceProblem::Validation => {
                            Some(Rendered::Failed("swapchain validation failed".to_owned()))
                        }
                    };
                }
                Err(error) => Err(error.to_string()),
            }
        };
        if let Err(problem) = drawn {
            // **The CPU backend draws it instead**, and this is what `CLAUDE.md` keeps that
            // backend for: it is the correctness oracle *and* the startup path, so a page the
            // GPU refuses is a page this program can still show — more slowly, which is a cost
            // a person can see past, where a page that never appears is not. The raster is
            // presented through the same quorra surface as one image, so a working window is
            // the only path pixels take to the screen.
            //
            // Reported either way. A page drawn by the slower of two backends is a fact about
            // this build worth saying out loud, and saying it is what would have made the
            // hundred-and-forty-second session's report a sentence rather than a mystery.
            let raster = match CpuRasterizer::new().rasterize(&request.list, target) {
                Ok(raster) => raster,
                Err(second) => {
                    return Some(Rendered::Failed(format!(
                        "the graphics device {problem}, and the processor {second}"
                    )));
                }
            };
            if let Err(second) = state.presenter.present(PresentFrame {
                width,
                height,
                page: None,
                raster: Some(&raster),
                overlays: &overlays,
            }) {
                return Some(Rendered::Failed(format!(
                    "the graphics device {problem}, and presenting the processor's page {second}"
                )));
            }
            if !self.processor {
                println!(
                    "note: page {}: the graphics device {problem}, so it was drawn on the \
                     processor instead",
                    request.page.saturating_add(1)
                );
            }
        }
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
        // ~30 ms to a usable device on this machine, shaders compiling on a
        // background thread — the presenter reports uncaptured device errors
        // itself, for the same silent-window reason the Vello host did.
        let presenter = QuorraPresenter::new(window.clone()).expect("presenter creation");
        if self.trace {
            println!("trace: rendering with {}", presenter.adapter_description());
        }

        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        self.state = Some(State {
            window,
            presenter,
            size: (size.width.max(1), size.height.max(1)),
        });
        self.retitle();
        // The window's size is the first thing the core has been told about the viewport, and
        // it is what makes page one render. **Less the sidebar**, which Table 29's `/PageMode`
        // may already have opened: the document was opened before this window existed, so the
        // first `Resize` is the first chance to say how much of it the page has.
        self.dispatch(Command::Resize {
            width: size.width.saturating_sub(self.panel.inset(scale)).max(1),
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
                // The two keys this program answers itself rather than by sending a command:
                // whether a panel is shown is chrome, and `viewer-core` has no opinion about
                // chrome by construction (rule 5).
                if matches!(logical_key.as_ref(), Key::Character("o")) {
                    self.panel.toggle();
                    self.resize_page();
                    return;
                }
                if matches!(logical_key.as_ref(), Key::Character("?")) {
                    self.about.toggle();
                    self.redraw();
                    return;
                }
                // Everything else goes to the page, and the About card is over it: a key press
                // that turned a page nobody can see would be answering the wrong question.
                if self.about.shown {
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
                if self.about.shown {
                    return;
                }
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
                    // The presenter reconfigures its surface from the viewport on
                    // the next frame; the host only has to remember the size.
                    state.size = (size.width.max(1), size.height.max(1));
                }
                self.dispatch(Command::Resize {
                    width: size.width.saturating_sub(self.panel.inset(scale)).max(1),
                    height: size.height.max(1),
                    scale,
                });
            }

            WindowEvent::MouseWheel { delta, .. } => self.wheel(delta),

            // Remembered rather than read at the wheel, because winit puts no modifier state in
            // the wheel's own event.
            WindowEvent::ModifiersChanged(modifiers) => {
                self.control = modifiers.state().control_key();
            }

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
        // No anchor: a keyboard names no point, so the core holds the viewport's centre.
        Key::Character("+" | "=") => Command::Zoom {
            zoom: Zoom::In,
            at: None,
        },
        Key::Character("-") => Command::Zoom {
            zoom: Zoom::Out,
            at: None,
        },
        Key::Character("0") => Command::Zoom {
            zoom: Zoom::FitPage,
            at: None,
        },
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
        Command::Zoom { zoom, at } => format!("zoom {zoom:?} at {at:?}"),
        Command::Scroll { dx, dy } => format!("scroll {dx} {dy}"),
        Command::SetGroup { group, on } => format!("layer {group:?} {on}"),
        Command::Activate(object) => format!("activate {object:?}"),
        Command::Extract { name } => format!("extract {name:?}"),
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
        Event::Extracted { name, bytes, .. } => {
            format!("extracted {name:?}, {} bytes", bytes.len())
        }
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
/// The selection quads as a display list in the window's own pixels.
///
/// The quadrilaterals arrive from `viewer-core` in device pixels of this window, so nothing here
/// composes a transform: that is the whole point of chrome crossing as geometry rather than as
/// pixels. Drawn with `Multiply`, which darkens what is under it and leaves the glyphs readable —
/// §11.3.5.2 makes it the one mode whose "result colour is always at least as dark as either of
/// the two constituent colours", so the text under the wash survives it. A native host asks its
/// platform for the colour; this one has nobody to ask, and a hard-coded blue that says so is
/// better than one that pretends.
///
/// **One fill, one subpath per quad**, and the count matters rather than the shape: a compositor
/// gives every non-`Over` blend its own layer and prices its internal textures before allocating
/// them, so a fill per quad made a selection cost `(quads + 1) × 2 × width × height × 4` bytes of
/// frame budget — 6.4 MB a quad at 800 × 1000, spending a 256 MiB budget at 63 quads, which is one
/// short paragraph. Under one layer the cost stops depending on what is selected at all. The
/// per-quad blend it replaces was preserving something nobody wants: `Query::Selection` answers
/// one quad per *run*, runs tile rather than overlap, and the two overlapping pairs out of 171
/// measured on three lines of `tracemonkey.pdf` overlap by 0.28 and 0.17 of a device pixel. Under
/// the non-zero rule one path is one shape, so those slivers stop darkening twice as well.
fn highlight_list(quads: &[[f32; 8]], width: u32, height: u32) -> Option<pdf_render::DisplayList> {
    if quads.is_empty() {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
    let colour = Color::rgb(140.0 / 255.0, 180.0 / 255.0, 1.0);
    let mut path = Path::new();
    for quad in quads {
        for (index, corner) in quad.chunks_exact(2).enumerate() {
            let point = Point::new(corner[0], corner[1]);
            path.push(if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            });
        }
        path.push(PathCommand::Close);
    }
    list.push(DrawCommand::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Multiply,
    });
    Some(list)
}

/// The chrome drawn over a page, as display lists in the window's own pixels.
///
/// Gathered once per frame and handed to the presenter beside the page, which is why they are
/// display lists and not backend calls: they draw through the same translation as the page
/// itself, and that is what a `--cpu` run and a page the graphics device refuses both need.
#[derive(Default)]
struct Overlays {
    /// The sidebar, where it is shown.
    panel: Option<pdf_render::DisplayList>,
    /// `/NOTICE`, where it is shown. **Second, so it is on top**: it is a modal card and the
    /// sidebar is behind it.
    about: Option<pdf_render::DisplayList>,
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
