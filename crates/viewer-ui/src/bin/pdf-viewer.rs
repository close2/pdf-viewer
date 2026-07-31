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
use pdf_render::TargetSpec;
use pdf_syntax::Document;
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu::CurrentSurfaceTexture;
use vello::{AaConfig, AaSupport, Renderer, RendererOptions, wgpu};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Pixel budget for one rendered page.
///
/// Page dimensions come from the document and the scale from the window, so the product
/// needs a bound: a page claiming absurd dimensions must fail to render rather than ask
/// for all available memory.
const MAX_PIXELS: u64 = 1 << 28;

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

    let mut app = App::new(document, path.to_string_lossy().into_owned(), page_count);
    event_loop.run_app(&mut app).expect("event loop failed");
}

struct App {
    context: RenderContext,
    document: Document,
    title: String,
    /// §12.4.2's labelling ranges, read once when the document opens.
    ///
    /// Once rather than per page turn: the tree is a handful of ranges and reading it costs
    /// one walk, where doing it per page would put a number-tree walk on every arrow key.
    labels: pdf_model::page_label::PageLabels,
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
    fn new(document: Document, title: String, page_count: usize) -> Self {
        let labels = pdf_model::page_label::PageLabels::read(&document);
        Self {
            context: RenderContext::new(),
            document,
            title,
            labels,
            page_count,
            page_index: 0,
            state: None,
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
            .with_title(format!("{} — page 1 of {}", self.title, self.page_count))
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
                let Some(state) = self.state.as_mut() else {
                    return;
                };
                let report = render(
                    &self.context,
                    state,
                    &self.document,
                    index,
                    count,
                    &self.labels,
                );
                state
                    .window
                    .set_title(&format!("{} — {report}", self.title));
            }

            _ => {}
        }
    }
}

/// Renders the current page and presents it, returning a short status for the title bar.
fn render(
    context: &RenderContext,
    state: &mut State,
    document: &Document,
    page_index: usize,
    page_count: usize,
    labels: &pdf_model::page_label::PageLabels,
) -> String {
    let width = state.surface.config.width;
    let height = state.surface.config.height;

    let Some(page) = Pages::new(document).get(page_index) else {
        return format!("page {} could not be read", page_index.saturating_add(1));
    };
    let interpretation = pdf_model::interpret(document, &page);
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
        return format!(
            "page {} is too large to render",
            page_index.saturating_add(1)
        );
    };

    let handle = &context.devices[state.surface.dev_id];

    // §11.5's soft masks are rendered first, each into a texture of its own: a mask is a
    // transparency group evaluated at device resolution, so it cannot be part of the scene
    // that uses it. Costs nothing on a page with no mask.
    let Ok(masks) = render_gpu::evaluate_soft_masks(
        &handle.device,
        &handle.queue,
        &mut state.renderer,
        list,
        target,
    ) else {
        return format!(
            "page {} has a soft mask this build cannot evaluate",
            page_index.saturating_add(1)
        );
    };

    // The same translation the headless tests exercise, so what the window shows cannot
    // drift from what CI checks.
    let Ok(scene) = render_gpu::build_scene(list, target, &masks) else {
        return format!(
            "page {} contains content this build cannot draw",
            page_index.saturating_add(1)
        );
    };

    if state
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
        .is_err()
    {
        return "rendering failed".to_owned();
    }

    // `get_current_texture` reports swapchain state rather than returning a Result, and the
    // non-success cases are ordinary events: a resize race leaves the surface outdated,
    // minimising occludes it.
    let frame = match state.surface.surface.get_current_texture() {
        CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => frame,
        CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
            context.configure_surface(&state.surface);
            state.window.request_redraw();
            return describe(page_index, page_count, labels, &interpretation);
        }
        CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => {
            return describe(page_index, page_count, labels, &interpretation);
        }
        CurrentSurfaceTexture::Validation => return "swapchain validation failed".to_owned(),
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

    describe(page_index, page_count, labels, &interpretation)
}

/// Builds the title-bar status, naming what could not be drawn.
fn describe(
    page_index: usize,
    page_count: usize,
    labels: &pdf_model::page_label::PageLabels,
    interpretation: &pdf_model::Interpretation,
) -> String {
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
