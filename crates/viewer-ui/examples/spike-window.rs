//! Spike B, interactive half: present a Vello scene in a real window.
//!
//! Run with:
//!
//! ```text
//! cargo run --release --example spike-window -p viewer-ui
//! ```
//!
//! Escape or closing the window exits. Frame times are printed to stdout once a
//! second, so this doubles as the first crude latency measurement.
//!
//! # Why this is an example and not a test
//!
//! Everything that *can* be checked without a display already is — see
//! `render-gpu/tests/headless_gpu.rs`, which covers scene translation, clip
//! re-nesting, rasterisation and readback under a software adapter in CI. What remains
//! is whether a window appears and presents, which no headless check can answer. So
//! this is the one piece deliberately left for a human to look at.
//!
//! Resizing is handled because a surface that is not reconfigured on resize either
//! stretches or panics, and that is precisely the sort of defect only interactive use
//! reveals.

// An example binary: printing is the interface, and a panic is an acceptable failure
// mode for a spike whose purpose is to be watched.
#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "interactive spike: stdout is the reporting channel and panics are visible"
)]

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;

use pdf_render::TargetSpec;
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu::CurrentSurfaceTexture;
use vello::{AaConfig, AaSupport, Renderer, RendererOptions, wgpu};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Pixel budget for a target; far above anything this spike requests.
const GENEROUS: u64 = 1 << 30;

fn main() {
    let event_loop = EventLoop::new().expect("an event loop requires a display server");
    // The renderer redraws only on request rather than spinning: a viewer is idle
    // almost all the time, and a busy loop would burn battery for nothing.
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("event loop failed");
}

struct App {
    context: RenderContext,
    state: Option<State>,
    frames: u32,
    last_report: Option<Instant>,
}

impl App {
    fn new() -> Self {
        // `RenderContext` is not `Default`: it allocates a wgpu instance.
        Self {
            context: RenderContext::new(),
            state: None,
            frames: 0,
            last_report: None,
        }
    }
}

struct State {
    window: Arc<Window>,
    surface: RenderSurface<'static>,
    renderer: Renderer,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("pdf-viewer — Spike B")
            .with_inner_size(winit::dpi::LogicalSize::new(595.0, 842.0));
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

        let device = &self.context.devices[surface.dev_id].device;
        let renderer = Renderer::new(
            device,
            RendererOptions {
                antialiasing_support: AaSupport {
                    area: true,
                    msaa8: false,
                    msaa16: false,
                },
                // One thread: shader compilation is on the startup path, and the
                // heuristic default spawns threads whose benefit is unmeasured here.
                num_init_threads: NonZeroUsize::new(1),
                ..Default::default()
            },
        )
        .expect("renderer creation");

        println!(
            "adapter: {:?}",
            self.context.devices[surface.dev_id].adapter().get_info()
        );
        self.state = Some(State {
            window,
            surface,
            renderer,
        });
        self.last_report = Some(Instant::now());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),

            WindowEvent::Resized(size) => {
                // Without this the surface either stretches or the swapchain panics on
                // the next acquire.
                self.context.resize_surface(
                    &mut state.surface,
                    size.width.max(1),
                    size.height.max(1),
                );
                state.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let started = Instant::now();
                render(&self.context, state);

                self.frames = self.frames.saturating_add(1);
                if self
                    .last_report
                    .is_some_and(|last| last.elapsed().as_secs() >= 1)
                {
                    println!(
                        "{} frames in the last second, last frame {:.2} ms",
                        self.frames,
                        started.elapsed().as_secs_f64() * 1000.0
                    );
                    self.frames = 0;
                    self.last_report = Some(Instant::now());
                }
            }

            _ => {}
        }
    }
}

/// Draws the shared test scene, scaled to fill the window, and presents it.
fn render(context: &RenderContext, state: &mut State) {
    let list = test_scenes::basic();
    let width = state.surface.config.width;
    let height = state.surface.config.height;

    // Fit the whole page inside the window, taking the smaller of the two ratios so that
    // neither dimension overflows. Fitting on width alone crops the bottom whenever the
    // window is proportionally taller than the page.
    let scale = (f64::from(width) / f64::from(list.page_size.width))
        .min(f64::from(height) / f64::from(list.page_size.height));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a window dimension divided by a page dimension is a small ratio"
    )]
    let target =
        TargetSpec::for_page(&list, scale as f32, GENEROUS).expect("window-sized target is valid");

    // Deliberately the same translation the headless tests exercise, rather than a
    // second one that could drift from it. An earlier version of this spike had its own
    // simplified builder that ignored clips, so the window showed a scene the test suite
    // never checked — exactly the divergence this avoids.
    let scene = render_gpu::build_scene(&list, target.transform).expect("scene is supported");

    let handle = &context.devices[state.surface.dev_id];
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
        .expect("scene renders");

    // `get_current_texture` reports swapchain state rather than returning a Result, and
    // the non-success cases are all ordinary events in a real window: a resize race
    // leaves the surface outdated, minimising occludes it. Panicking on those would make
    // the viewer fragile in exactly the situations a user creates by accident.
    let frame = match state.surface.surface.get_current_texture() {
        CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => frame,
        CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
            // Reconfigure and let the next redraw succeed.
            context.configure_surface(&state.surface);
            state.window.request_redraw();
            return;
        }
        // Nothing is visible, so there is nothing to present.
        CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => return,
        CurrentSurfaceTexture::Validation => {
            println!("swapchain acquire failed validation; skipping frame");
            return;
        }
    };
    let mut encoder = handle
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("present"),
        });
    // Vello renders into its own target texture; the blitter copies that into the
    // swapchain image, which may have a different format.
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
}
