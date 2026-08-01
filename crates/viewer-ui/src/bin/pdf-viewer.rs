//! The viewer: opens a PDF and shows it.
//!
//! ```text
//! cargo run --release -p viewer-ui --bin pdf-viewer -- document.pdf
//! ```
//!
//! Left and right arrows or Page Up and Down change page, Escape quits. The window title
//! shows the page's own label where the document states one (§12.4.2), the page number, and
//! anything on the page that could not be drawn.
//!
//! # Reporting incomplete pages
//!
//! When the interpreter cannot draw something — an unsupported font, a shading — the title
//! says so. A viewer that renders a page missing half its content and looks confident about
//! it is worse than one that admits the gap, and the user is the only one in a position to
//! judge whether what is missing matters.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line application: stdout is a reporting channel and a panic on a \
              missing display is the intended failure"
)]

use std::num::NonZeroUsize;
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::view::Request;
use pdf_render::TargetSpec;
use pdf_syntax::Document;
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu::CurrentSurfaceTexture;
use vello::{AaConfig, AaSupport, Renderer, RendererOptions, wgpu};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Pixel budget for one rendered page.
///
/// Page dimensions come from the document and the scale from the window, so the product
/// needs a bound: a page claiming absurd dimensions must fail to render rather than ask
/// for all available memory.
const MAX_PIXELS: u64 = 1 << 28;

#[expect(
    clippy::too_many_lines,
    reason = "opening a document is a sequence of things said out loud about it, and each is \
              one clause's — splitting them would separate a note from the reason it exists"
)]
fn main() {
    // Parsed before anything opens a document, because it decides where that document's
    // images are decoded and a policy applied halfway through is not a policy.
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
        eprintln!("usage: pdf-viewer [--no-sandbox] <document.pdf>");
        eprintln!();
        eprintln!("Arrow keys or Page Up/Down change page; Escape quits.");
        eprintln!();
        eprintln!("  --no-sandbox  decode JBIG2 and JPEG 2000 images in this process rather than");
        eprintln!("                in a confined worker. Faster by a process spawn and a pipe");
        eprintln!("                round trip; appropriate only for documents you trust.");
        std::process::exit(2);
    };

    if !sandbox {
        pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
        // Said out loud, once, on the way past. Turning the sandbox off is a reasonable
        // choice for documents you produced yourself and a bad one for documents that
        // arrived by email, and the difference is not visible from inside the program.
        println!(
            "note: --no-sandbox — JBIG2 and JPEG 2000 will be decoded in this process, with \
             no memory ceiling, and a decoder failure will take the viewer down with it"
        );
    }

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.to_string_lossy());
            std::process::exit(1);
        }
    };

    let document = match Document::open(bytes) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("cannot open {}: {error}", path.to_string_lossy());
            std::process::exit(1);
        }
    };

    if document.was_recovered() {
        // Worth saying: the file's own cross-reference table was unusable and the document
        // was reconstructed by scanning. It may still be missing content.
        println!("note: this file's cross-reference table was broken and was rebuilt by scanning");
    }

    // §12.11's document requirements, said once and out loud. The clause makes this a
    // statement about the *document* rather than about any page — "there is no formal
    // connection between the requirement type and the operation of the associated feature(s)"
    // — so it belongs here rather than in a page's report, and it is the one thing a page
    // report cannot do: tell a person before they trust what they are looking at. §12.11.6
    // asks a processor that cannot meet the requirements to stop; this one draws the document
    // and names what it could not promise, because refusing to open a file somebody asked for
    // is a worse failure. No corpus document states any of this.
    for (requirement, reason) in pdf_model::requirements::unmet(&document) {
        println!(
            "note: this document requires {} (penalty {}) — {reason}",
            requirement.kind.as_str(),
            requirement.penalty
        );
    }

    // §7.11.4's embedded files, listed and not extracted: the bytes are inside the document,
    // and writing one out is a person's decision taken in a program that can ask. Saying they
    // exist is the half a viewer with no attachment panel can still do honestly.
    let attachments = pdf_model::attachment::attachments(&document);
    for attachment in &attachments {
        let size = attachment
            .size
            .map_or_else(String::new, |size| format!(", {size} bytes"));
        println!(
            "note: this document carries an embedded file: {}{size}{}",
            attachment
                .file_name
                .as_deref()
                .unwrap_or(attachment.name.as_str()),
            attachment
                .media_type
                .as_deref()
                .map_or_else(String::new, |media| format!(" ({media})"))
        );
    }

    // §12.8's signatures, said once and out loud, because a signature is a claim about the
    // *file* and a person deciding whether to trust what they are looking at has no other way to
    // hear it. Three things are said and a fourth is refused: who signed, why, whether the range
    // they signed runs to the end of the file (§12.8.1) — and that this program does not verify
    // anything, because verification needs a certificate chain and a trust store, which is
    // §7.6.5's refusal one clause over (ADR 0031).
    let signatures = pdf_model::signature::signatures(&document);
    if !signatures.is_empty() {
        let length = document.bytes().len() as u64;
        for signature in &signatures {
            let who = signature.name.as_deref().unwrap_or("an unnamed signer");
            let why = signature
                .reason
                .as_deref()
                .map_or_else(String::new, |reason| format!(", reason: {reason}"));
            println!(
                "note: this document is signed by {who}{why}{}",
                if signature.certification {
                    " (a certification signature)"
                } else {
                    ""
                }
            );
            match signature.coverage(length) {
                pdf_model::signature::Coverage::WholeFile => {}
                pdf_model::signature::Coverage::Unsigned { tail } => println!(
                    "note: {tail} bytes were appended after that signature and are not covered \
                     by it"
                ),
                pdf_model::signature::Coverage::Malformed => {
                    println!("note: that signature's /ByteRange does not describe this file");
                }
            }
        }
        println!(
            "note: signatures are not verified — this program has no certificate store, so it \
             says what a signature claims and never whether it is valid"
        );
    }

    let page_count = Pages::new(&document).len();
    println!("{}: {page_count} page(s)", path.to_string_lossy());
    if page_count == 0 {
        eprintln!("the document has no pages");
        std::process::exit(1);
    }

    let event_loop = EventLoop::new().expect("an event loop requires a display server");
    // Redraw on request rather than continuously: a document viewer is idle almost all the
    // time, and a spinning loop would drain a battery for nothing.
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new(
        document,
        path.to_string_lossy().into_owned(),
        // §12.7.6.4's import-data action names a file, and this is the only place that name is
        // allowed to mean anything: a *sibling of the document being shown*. See `import_data`.
        std::path::Path::new(&path)
            .parent()
            .map(std::path::Path::to_path_buf),
        page_count,
    );
    event_loop.run_app(&mut app).expect("event loop failed");
}

struct App {
    context: RenderContext,
    document: Document,
    title: String,
    /// The directory the open document is in, where one can be named.
    ///
    /// The whole of this program's answer to "which files may a document ask for". §12.7.6.4's
    /// import-data action carries a file specification the *document* wrote, so honouring it
    /// unrestricted would let a PDF read any path this process can — and the clause states no
    /// policy, because a policy is a property of the processor. See `import_data`.
    directory: Option<std::path::PathBuf>,
    /// What §12.6.4's actions have changed since the document opened.
    ///
    /// Held here rather than rebuilt per frame because that is what it *is*: a layer a click
    /// switched off is not in the file, and a state rebuilt from the file every frame would
    /// switch it back on. See `pdf_model::view`.
    view: pdf_model::view::ViewState,
    /// §12.4.2's labelling ranges, read once when the document opens.
    ///
    /// Once rather than per page turn: the tree is a handful of ranges and reading it costs
    /// one walk, where doing it per page would put a number-tree walk on every arrow key.
    labels: pdf_model::page_label::PageLabels,
    /// Where the pointer last was, in window pixels.
    ///
    /// `winit` reports movement and clicks as separate events, so following a link needs the
    /// position remembered from the last `CursorMoved` — the click itself carries none.
    cursor: (f64, f64),
    /// The scale the last render used, which is what maps a window pixel back to the page.
    ///
    /// Recomputed per frame from the window size, so it is read from the frame that is on the
    /// screen rather than from one this handler computes for itself.
    scale: f32,
    /// §12.3.3's outline, read once for the same reason.
    ///
    /// There is no panel to show it in. What it is used for is the one question a viewer
    /// without a panel can still answer — which section the current page is in — and that is
    /// worth more in a title bar than a page number alone.
    outline: pdf_model::outline::Outline,
    page_count: usize,
    page_index: usize,
    state: Option<State>,
}

struct State {
    window: Arc<Window>,
    surface: RenderSurface<'static>,
    renderer: Renderer,
}

impl App {
    fn new(
        document: Document,
        title: String,
        directory: Option<std::path::PathBuf>,
        page_count: usize,
    ) -> Self {
        let labels = pdf_model::page_label::PageLabels::read(&document);
        let outline = pdf_model::outline::Outline::read(&document, &Pages::new(&document));
        // §12.3.2.1: "the optional OpenAction entry in a document's catalog dictionary may
        // specify a destination that shall be displayed when the document is opened." Table 29
        // states the other half — an absent or unresolvable entry means "the top of the first
        // page" — which is what `unwrap_or(0)` is, and why nothing is reported here.
        //
        // Only the *page* of the destination is honoured. Its location and magnification are
        // properties of a window with scrolling and zoom, and this one fits a page to its
        // surface; `Destination::view` carries them for whoever builds that.
        let page_index = pdf_model::destination::Destination::open_action(&document)
            .and_then(|destination| destination.page_index(&document, &Pages::new(&document)))
            .filter(|index| *index < page_count)
            .unwrap_or(0);
        let view = pdf_model::view::ViewState::of(&document);
        Self {
            context: RenderContext::new(),
            document,
            title,
            directory,
            view,
            labels,
            outline,
            cursor: (0.0, 0.0),
            scale: 1.0,
            page_count,
            page_index,
            state: None,
        }
    }

    /// Activates the §12.5.6.5 link under the pointer, if there is one.
    ///
    /// Returns whether the page changed. A click on nothing, on a link whose action this
    /// program will not perform — §12.6.4.6's launch action is absent for the reason
    /// principle 3 gives — or on a link to the page already shown, all leave the view where
    /// it is.
    ///
    /// A §12.6.4.8 URI is printed rather than opened. The URI is resolved by `pdf_model`,
    /// including `/IsMap`'s cursor coordinates; what this program will not do is hand a
    /// string a document controls to a browser, because that is a decision about this machine
    /// and not about the document.
    fn follow_link(&mut self) -> bool {
        let pages = Pages::new(&self.document);
        let Some(page) = page_at(&self.document, &self.view, self.page_index) else {
            return false;
        };
        // The window pixel to the page's own space, then to default user space, which is
        // where §12.5.2 puts an annotation's rectangle. The scale is the one the frame on
        // screen was drawn with.
        if self.scale <= 0.0 {
            return false;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a window coordinate is a small number of pixels"
        )]
        let (x, y) = (
            self.cursor.0 as f32 / self.scale,
            self.cursor.1 as f32 / self.scale,
        );
        let Some((x, y)) = pdf_model::content::user_space_at(&page, x, y) else {
            return false;
        };

        let links = pdf_model::link::links(&self.document, &page);
        let Some(link) = pdf_model::link::at(&links, x, y) else {
            return false;
        };

        // §12.6.4's actions first, because two of the five this program performs change what
        // the *current* page draws — a layer's state (§12.6.4.13) and an annotation's Hidden
        // flag (§12.6.4.11) — and a link may do both and then jump. `ViewState` keeps them;
        // redrawing is this function's answer either way.
        let before = self.view.clone();
        // Trap 5, on the one path where an action can be declined: every type Table 201 lists
        // and this program does not perform arrives as `Action::Refused` carrying its own
        // reason, and dropping it silently would make a click that does nothing
        // indistinguishable from a click on nothing.
        for action in &link.actions {
            if let pdf_model::action::Action::Refused(why) = action {
                println!("link: declined — {why}");
            }
        }
        let requests = self.view.perform_all(&self.document, &link.actions);

        // The first request that names a page wins, because a chain that jumps twice has
        // shown the second page either way and §12.6.2 states no rule for the pair.
        let mut target = link
            .destination
            .and_then(|destination| destination.page_index(&self.document, &pages));
        let mut imports = Vec::new();
        for request in &requests {
            match request {
                Request::Display(destination) => {
                    target = target.or_else(|| destination.page_index(&self.document, &pages));
                }
                Request::Page(named) => {
                    target = target.or_else(|| named.page_from(self.page_index, self.page_count));
                }
                Request::Resolve(uri) => {
                    println!("link: {}", uri.at_position((x, y), link.rect));
                }
                // Deferred rather than performed here: reading the file needs `&mut self`
                // and `pages` still borrows the document. Nothing is lost by the wait —
                // §12.6.2 makes a chain a sequence, and an import changes no page number.
                Request::Import(import) => imports.push(import.clone()),
                Request::Thread(jump) => {
                    // §12.4.3's threads are read *here* rather than when the document opens:
                    // an article is a list nothing else in this program consults, and
                    // `CLAUDE.md` principle 2's "nothing eager" applies to the two documents
                    // in a thousand that would pay for it at launch.
                    let articles = pdf_model::article::Articles::read(&self.document);
                    target = target.or_else(|| {
                        jump.bead_in(&articles)
                            .and_then(|bead| bead.page_index(&pages))
                    });
                }
            }
        }
        let target =
            target.filter(|target| *target != self.page_index && *target < self.page_count);
        for import in &imports {
            self.import_data(import);
        }
        // After the imports, not before: §12.7.8's values are what §12.7.4.3 lays out, so a
        // successful import changes this page's ink and the page has to be redrawn.
        let changed_here = self.view != before;
        if let Some(target) = target {
            self.page_index = target;
            return true;
        }
        changed_here
    }

    /// Performs §12.7.6.4's import-data action, which is the half `pdf_model` cannot.
    ///
    /// The clause says a processor "shall import data … from a specified file", and specifies
    /// nothing about *which* files a document may name — because that is a property of the
    /// processor rather than of the document. So this states the policy, and it is the narrowest
    /// one that still performs the action:
    ///
    /// - the name must be a single path component, so `../…` and any absolute path are refused;
    /// - it is resolved against the directory the open document is in, and nowhere else;
    /// - only §12.7.8's FDF is read. ISO 19444-1's XFDF is the same data in XML and would need
    ///   an XML parser, which is a dependency and a decision rather than a clause.
    ///
    /// Every refusal is printed, which is trap 5 on the one path where a click can decline.
    fn import_data(&mut self, import: &pdf_model::action::ImportData) {
        use pdf_model::action::DataFormat;
        use pdf_model::forms_data::FormsData;

        if import.format != DataFormat::Fdf {
            println!(
                "import-data: declined — {} is not §12.7.8's FDF, and no other data format is read",
                import.file
            );
            return;
        }
        let Some(directory) = self.directory.as_ref() else {
            println!("import-data: declined — the document is not in a known directory");
            return;
        };
        // One component, checked as a path rather than as a string, so that a separator this
        // platform recognises and this program does not cannot slip through.
        let named = std::path::Path::new(&import.file);
        let mut components = named.components();
        let (Some(std::path::Component::Normal(single)), None) =
            (components.next(), components.next())
        else {
            println!(
                "import-data: declined — {} is not a plain file name beside the document",
                import.file
            );
            return;
        };
        let path = directory.join(single);

        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("import-data: cannot read {}: {error}", path.display());
                return;
            }
        };
        let opened = match Document::open(bytes) {
            Ok(opened) => opened,
            Err(error) => {
                println!("import-data: cannot open {}: {error}", path.display());
                return;
            }
        };
        let data = match FormsData::read(&opened) {
            Ok(data) => data,
            Err(error) => {
                println!("import-data: {}: {error}", path.display());
                return;
            }
        };

        // §14.4's file identifier, where both files state one: an FDF exported from a different
        // document is imported anyway — the clause states no rule against it and a form's fields
        // may legitimately be shared — but a person deserves to be told.
        if data.belongs_to(&self.document) == Some(false) {
            println!("import-data: this FDF file's /ID names a different document");
        }
        // Table 246's `/Status` is "a status string that shall be displayed"; this window has no
        // place to display one, and stdout is where every other such answer goes.
        if let Some(status) = &data.status {
            println!("import-data: status — {status}");
        }
        for owed in &data.owed {
            println!("import-data: not applied — {owed}");
        }
        let outcome = self.view.import(&self.document, &data);
        println!(
            "import-data: {} field(s) from {}, into {} widget(s)",
            data.fields.len(),
            path.display(),
            outcome.widgets
        );
        for name in &outcome.unmatched {
            println!("import-data: this document has no field named {name}");
        }
        for refusal in &outcome.refused {
            println!("import-data: declined — {refusal}");
        }
        if outcome.pages > 0 {
            // §12.7.7's template pages become part of the document being shown, so the page
            // count moves — which is the one thing an action in this program has ever changed
            // about how many pages there are.
            self.page_count = Pages::new(&self.document)
                .len()
                .saturating_add(self.view.appended_pages().len());
            println!(
                "import-data: {} template page(s) added; the document now has {}",
                outcome.pages, self.page_count
            );
        }
    }

    /// Moves by `delta` pages, clamped to the document.
    ///
    /// Clamped rather than wrapping: paging past the end and landing back at page one is
    /// disorienting, and the end of a document is information worth feeling.
    fn turn_page(&mut self, delta: isize) -> bool {
        let last = self.page_count.saturating_sub(1);
        let target = self.page_index.saturating_add_signed(delta).min(last);
        if target == self.page_index {
            return false;
        }
        self.page_index = target;
        true
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(format!(
                "{} — page {} of {}",
                self.title,
                self.page_index.saturating_add(1),
                self.page_count
            ))
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

        self.state = Some(State {
            window,
            surface,
            renderer,
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
                let delta = match logical_key {
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space) => 1,
                    Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => -1,
                    // Home and End clamp inside `turn_page`, so a delta larger than the
                    // document is the simplest way to express "as far as it goes".
                    Key::Named(NamedKey::Home) => isize::MIN,
                    Key::Named(NamedKey::End) => isize::MAX,
                    _ => return,
                };
                if self.turn_page(delta)
                    && let Some(state) = self.state.as_ref()
                {
                    state.window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if self.follow_link()
                    && let Some(state) = self.state.as_ref()
                {
                    state.window.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(state) = self.state.as_mut() {
                    self.context.resize_surface(
                        &mut state.surface,
                        size.width.max(1),
                        size.height.max(1),
                    );
                    state.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                let (index, count) = (self.page_index, self.page_count);
                let section = self
                    .outline
                    .section_at(&self.document, &Pages::new(&self.document), index)
                    .map(ToOwned::to_owned);
                let (view, labels) = (&self.view, &self.labels);
                let Some(state) = self.state.as_mut() else {
                    return;
                };
                let caption = Caption {
                    page_index: index,
                    page_count: count,
                    labels,
                    section: section.as_deref(),
                };
                let (report, scale) = render(&self.context, state, &self.document, view, &caption);
                state
                    .window
                    .set_title(&format!("{} — {report}", self.title));
                self.scale = scale;
            }

            _ => {}
        }
    }
}

/// The page at `index`, counting §12.7.8.3.3's imported template pages after the document's own.
///
/// §12.7.7's template pages are objects of this document that the page *tree* does not reach —
/// the clause puts them in a name tree precisely so that they are not displayed until something
/// asks — so showing one means building it without the inheritance a tree would have given it,
/// which `Pages::detached` is. Their position is this program's choice: §12.7.8.3.3 says a
/// template page is added to the document and states no place, and after the document's own
/// pages is the only order that leaves every existing page index meaning what it meant.
fn page_at(
    document: &Document,
    view: &pdf_model::view::ViewState,
    index: usize,
) -> Option<pdf_model::Page> {
    let pages = Pages::new(document);
    if let Some(page) = pages.get(index) {
        return Some(page);
    }
    let appended = index.checked_sub(pages.len())?;
    let object = document.get(*view.appended_pages().get(appended)?);
    Some(pages.detached(object.as_dict()?))
}

/// Renders the current page and presents it.
///
/// Returns a short status for the title bar and the scale it drew at — the second because a
/// What the title bar needs to name the page being shown.
///
/// One struct rather than four parameters because they travel together everywhere and none of
/// them is about *drawing*: §12.4.2's label, the index behind it, how many pages there are, and
/// §12.3.3's section. [`render`] passes it straight through to [`describe`].
struct Caption<'a> {
    /// Zero-based, which is the page tree's numbering and not a reader's.
    page_index: usize,
    /// How many pages the document has.
    page_count: usize,
    /// §12.4.2's labelling ranges.
    labels: &'a pdf_model::page_label::PageLabels,
    /// §12.3.3's section title for this page, where the outline names one.
    section: Option<&'a str>,
}

/// click has to be mapped back through exactly the transform the frame on screen was drawn
/// with, and that scale is computed here from the window's own size.
fn render(
    context: &RenderContext,
    state: &mut State,
    document: &Document,
    view: &pdf_model::view::ViewState,
    caption: &Caption<'_>,
) -> (String, f32) {
    let page_index = caption.page_index;
    let width = state.surface.config.width;
    let height = state.surface.config.height;

    let Some(page) = page_at(document, view, page_index) else {
        return (
            format!("page {} could not be read", page_index.saturating_add(1)),
            1.0,
        );
    };
    let interpretation = pdf_model::content::interpret_with(document, &page, view);
    let list = &interpretation.display_list;

    // Fit the whole page in the window, taking the smaller ratio so neither dimension
    // overflows.
    let scale = (f64::from(width) / f64::from(list.page_size.width))
        .min(f64::from(height) / f64::from(list.page_size.height));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a window dimension divided by a page dimension is a small ratio"
    )]
    let scale = scale as f32;

    let Ok(target) = TargetSpec::for_page(list, scale, MAX_PIXELS) else {
        return (
            format!(
                "page {} is too large to render",
                page_index.saturating_add(1)
            ),
            scale,
        );
    };

    let handle = &context.devices[state.surface.dev_id];
    if let Err(problem) = draw(context, state, list, target, width, height) {
        return (
            format!("page {}: {problem}", page_index.saturating_add(1)),
            scale,
        );
    }

    // `get_current_texture` reports swapchain state rather than returning a Result, and the
    // non-success cases are ordinary events: a resize race leaves the surface outdated,
    // minimising occludes it.
    let frame = match state.surface.surface.get_current_texture() {
        CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => frame,
        CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
            context.configure_surface(&state.surface);
            state.window.request_redraw();
            return (describe(caption, &interpretation), scale);
        }
        CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => {
            return (describe(caption, &interpretation), scale);
        }
        CurrentSurfaceTexture::Validation => {
            return ("swapchain validation failed".to_owned(), scale);
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

    (describe(caption, &interpretation), scale)
}

/// Builds the scene and renders it into the surface's texture.
///
/// Split out of [`render`] because it is the part with no *document* in it: everything here is
/// this build's GPU path, and the caller's job is deciding what to say when it fails.
fn draw(
    context: &RenderContext,
    state: &mut State,
    list: &pdf_render::DisplayList,
    target: TargetSpec,
    width: u32,
    height: u32,
) -> Result<(), &'static str> {
    let handle = &context.devices[state.surface.dev_id];

    // §11.5's soft masks are rendered first, each into a texture of its own: a mask is a
    // transparency group evaluated at device resolution, so it cannot be part of the scene
    // that uses it. Costs nothing on a page with no mask.
    let masks = render_gpu::evaluate_soft_masks(
        &handle.device,
        &handle.queue,
        &mut state.renderer,
        list,
        target,
    )
    .map_err(|_| "has a soft mask this build cannot evaluate")?;

    // The same translation the headless tests exercise, so what the window shows cannot
    // drift from what CI checks.
    let scene = render_gpu::build_scene(list, target, &masks)
        .map_err(|_| "contains content this build cannot draw")?;

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

/// Builds the title-bar status, naming what could not be drawn.
fn describe(caption: &Caption<'_>, interpretation: &pdf_model::Interpretation) -> String {
    let Caption {
        page_index,
        page_count,
        labels,
        section,
    } = *caption;
    // ISO 32000-2 §12.4.2: "Page labels and page indices need not coincide". Where the
    // document states a label, it is what a reader is meant to see — a page of front matter
    // is *iv*, not page four — so the index is shown beside it rather than instead of it,
    // because a viewer whose title says only `iv` cannot say `of 320`.
    let page = match labels.label(page_index) {
        Some(label) if !label.is_empty() => format!(
            "{label} — page {} of {page_count}",
            page_index.saturating_add(1)
        ),
        _ => format!("page {} of {page_count}", page_index.saturating_add(1)),
    };
    // §12.3.3's outline is a table of contents, so the item covering this page is the section
    // a reader is in. Shown after the page number rather than before it, because it is context
    // for a position rather than the position itself.
    let page = match section {
        Some(section) if !section.is_empty() => format!("{page} — {section}"),
        _ => page,
    };
    if interpretation.is_complete() {
        return page;
    }

    // Summarised by kind rather than listed: a page may report dozens of items, and a title
    // bar that scrolls off the screen tells the user less than a count does.
    let mut fonts = 0usize;
    let mut images = 0usize;
    let mut other = 0usize;
    for item in &interpretation.unsupported {
        match item {
            pdf_model::Unsupported::Font { .. } => fonts = fonts.saturating_add(1),
            pdf_model::Unsupported::Image { .. } => images = images.saturating_add(1),
            _ => other = other.saturating_add(1),
        }
    }

    let mut parts = Vec::new();
    if fonts > 0 {
        parts.push(format!("{fonts} font(s) not drawn"));
    }
    if images > 0 {
        parts.push(format!("{images} image(s) not drawn"));
    }
    if other > 0 {
        parts.push(format!("{other} other item(s) not drawn"));
    }

    format!("{page} — incomplete: {}", parts.join(", "))
}
