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

use pdf_render::{TargetSpec, Transform};
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu::CurrentSurfaceTexture;
use vello::{AaConfig, AaSupport, Renderer, RendererOptions, Scene, wgpu};
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

    // Fit the page to the window rather than assuming a scale, so resizing is visibly
    // correct rather than merely not crashing.
    let scale = f64::from(width) / f64::from(list.page_size.width);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a window dimension divided by a page dimension is a small ratio"
    )]
    let target =
        TargetSpec::for_page(&list, scale as f32, GENEROUS).expect("window-sized target is valid");

    // The device transform from `for_page` assumes a page-sized raster; here the raster
    // is the window, so the surface dimensions are used directly.
    let scene = build_scene(&list, target.transform);

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

/// Builds a Vello scene directly, mirroring `render-gpu`'s translation.
///
/// `render-gpu`'s translation is deliberately private, since the display list is the
/// public contract rather than the Vello scene. This spike re-implements the small
/// subset it needs; it is not a second renderer, and nothing depends on it.
fn build_scene(list: &pdf_render::DisplayList, to_device: Transform) -> Scene {
    use pdf_render::{Command, Paint};
    use vello::kurbo;

    let mut scene = Scene::new();

    for command in list.commands() {
        // Clips are exercised by the headless tests; this spike draws unclipped so that
        // it stays a windowing check rather than a second rasteriser to maintain.
        let (Command::Fill {
            path,
            transform,
            paint,
            ..
        }
        | Command::Stroke {
            path,
            transform,
            paint,
            ..
        }) = command
        else {
            continue;
        };

        let mut bez = kurbo::BezPath::new();
        for step in path.commands() {
            use pdf_render::PathCommand as P;
            match *step {
                P::MoveTo(p) => bez.move_to((f64::from(p.x), f64::from(p.y))),
                P::LineTo(p) => bez.line_to((f64::from(p.x), f64::from(p.y))),
                P::CurveTo(a, b, c) => bez.curve_to(
                    (f64::from(a.x), f64::from(a.y)),
                    (f64::from(b.x), f64::from(b.y)),
                    (f64::from(c.x), f64::from(c.y)),
                ),
                P::Close => bez.close_path(),
            }
        }

        let combined = transform.then(to_device);
        let affine = kurbo::Affine::new([
            f64::from(combined.a),
            f64::from(combined.b),
            f64::from(combined.c),
            f64::from(combined.d),
            f64::from(combined.e),
            f64::from(combined.f),
        ]);

        let Paint::Solid(colour) = *paint else {
            continue;
        };
        let brush = vello::peniko::Color::new([colour.r, colour.g, colour.b, colour.a]);

        match command {
            Command::Stroke { stroke, .. } => {
                scene.stroke(
                    &kurbo::Stroke::new(f64::from(stroke.width)),
                    affine,
                    brush,
                    None,
                    &bez,
                );
            }
            _ => scene.fill(vello::peniko::Fill::NonZero, affine, brush, None, &bez),
        }
    }

    scene
}
