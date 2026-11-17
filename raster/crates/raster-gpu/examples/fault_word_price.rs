//! What a fault word read back after each frame would cost a page that draws a type 4 shading.
//!
//! The caller's ADR 2849 names two remedies for a device that records none of ISO 32000-2
//! §7.10.5.3's run-time errors. The first is a storage word the shading's shader would set and the
//! host would map after the frame; this prices that map, which is the part every frame with a
//! type 4 shading would pay whether or not a fault occurred. The shader's own `atomicOr` is not
//! here: it runs only on a faulting pixel, and a binding no frame writes changes no timing this
//! example could separate from its noise.
//!
//! The page is `function_based_shading.pdf`'s fifth program (`{ 0.5 sub exch 0.5 sub dup mul exch
//! dup mul add sqrt 2 sqrt mul }`, a gray radial distance) over a window of `turn_path`'s size, and
//! four arms are timed, interleaved, minimum and median of `ROUNDS` each:
//!
//! - **texture**: the frame into a texture, as a window draws one — `Device::render` submits and
//!   returns without waiting for the device;
//! - **texture + word**: the same, then a four-byte copy into a mappable buffer, submitted, mapped
//!   and waited for — the frame's own work now stands between the host and its next step;
//! - **readback**: the frame read back, as `turn_path`'s arm draws one, so the device is idle after;
//! - **readback + word**: the same and the word, which prices the map alone.
//!
//! ```text
//! cargo run --release -p raster-gpu --example fault_word_price [-- <rounds> | --check]
//! ```
//!
//! `--check` is one round of every arm and its assertion, printing no figure: a single round of a
//! wall clock is not a measurement (ADR 0060).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "a measurement whose output is its purpose; a panic is the measurement failing"
)]

#[path = "../../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::time::{Duration, Instant};

use raster_gpu::{Device, Options, Target, Viewport, wgpu};
use raster_scene::{
    Affine, BlendMode, Compose, FillRule, FnOp, FnRange, Paint, Point, Rect, Scene, SceneBuilder,
    Segment,
};

/// `turn_path`'s window (`frame_cost::WINDOW`).
const WINDOW: (u32, u32) = (1600, 1000);

/// Rounds per arm unless the command line says otherwise.
const ROUNDS: usize = 40;

/// `{ 0.5 sub exch 0.5 sub dup mul exch dup mul add sqrt 2 sqrt mul }`, `2 sqrt` folded.
const PROGRAM: [FnOp; 14] = [
    FnOp::PushReal(0.5),
    FnOp::Sub,
    FnOp::Exch,
    FnOp::PushReal(0.5),
    FnOp::Sub,
    FnOp::Dup,
    FnOp::Mul,
    FnOp::Exch,
    FnOp::Dup,
    FnOp::Mul,
    FnOp::Add,
    FnOp::Sqrt,
    FnOp::PushReal(std::f32::consts::SQRT_2),
    FnOp::Mul,
];

/// The page: the window filled with the shading, its unit domain mapped onto the window.
fn page(device: &mut Device) -> Scene {
    let (width, height) = (WINDOW.0 as f32, WINDOW.1 as f32);
    let program = device.upload_function(&PROGRAM).expect("admitted");
    let outline = device
        .upload_outline(&[
            Segment::MoveTo(Point::new(0.0, 0.0)),
            Segment::LineTo(Point::new(width, 0.0)),
            Segment::LineTo(Point::new(width, height)),
            Segment::LineTo(Point::new(0.0, height)),
            Segment::Close,
        ])
        .expect("upload");
    let mut builder = SceneBuilder::new();
    builder
        .fill(
            outline,
            Affine::IDENTITY,
            FillRule::NonZero,
            Paint::Function {
                program,
                domain: Rect::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0)),
                matrix: Affine::scale(width, height),
                range: FnRange::Gray([0.0, 1.0]),
                background: None,
            },
            None,
            BlendMode::Normal,
            Compose::SrcOver,
            None,
        )
        .expect("a valid function fill");
    builder.finish()
}

/// The word the shader would set, and the buffer the host maps it through.
struct FaultWord {
    word: wgpu::Buffer,
    staging: wgpu::Buffer,
}

impl FaultWord {
    fn new(gpu: &wgpu::Device) -> Self {
        let word = gpu.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fault word"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = gpu.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fault word, mapped"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { word, staging }
    }

    /// The copy, the submission, the map and the wait — what the host would do after a frame.
    fn read(&self, gpu: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        let mut encoder = gpu.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("fault word copy"),
        });
        encoder.copy_buffer_to_buffer(&self.word, 0, &self.staging, 0, 4);
        let submission = queue.submit([encoder.finish()]);
        let slice = self.staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| result.expect("mapped"));
        gpu.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .expect("the device answered");
        let bytes = slice.get_mapped_range().expect("mapped");
        let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        drop(bytes);
        self.staging.unmap();
        word
    }
}

fn milliseconds(span: Duration) -> f64 {
    span.as_secs_f64() * 1000.0
}

fn main() {
    let argument = std::env::args().nth(1);
    let check = argument.as_deref() == Some("--check");
    let rounds = if check {
        1
    } else {
        argument
            .and_then(|argument| argument.parse().ok())
            .unwrap_or(ROUNDS)
    };
    gpu_lock_hook::install();
    let mut device = Device::headless(&Options::default()).expect("an adapter");
    device.wait_until_warm();
    println!("adapter: {}", device.description());
    let scene = page(&mut device);
    let viewport = Viewport::full(WINDOW.0, WINDOW.1, Affine::IDENTITY);
    let (gpu, queue) = device.wgpu();
    let (gpu, queue) = (gpu.clone(), queue.clone());
    let texture = gpu.create_texture(&wgpu::TextureDescriptor {
        label: Some("fault word price target"),
        size: wgpu::Extent3d {
            width: WINDOW.0,
            height: WINDOW.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let fault = FaultWord::new(&gpu);

    let arms = ["texture", "texture + word", "readback", "readback + word"];
    let mut spans: [Vec<f64>; 4] = Default::default();
    // One unmeasured pass of every arm, so the shader and the staging buffers exist.
    for round in 0..=rounds {
        for (arm, samples) in spans.iter_mut().enumerate() {
            let target = if arm < 2 {
                Target::Texture(&texture)
            } else {
                Target::Readback
            };
            let began = Instant::now();
            let frame = device
                .render(&scene, &viewport, target)
                .expect("the page draws");
            if arm % 2 == 1 {
                assert_eq!(fault.read(&gpu, &queue), 0, "no program wrote the word");
            }
            let span = began.elapsed();
            drop(frame);
            if round > 0 {
                samples.push(milliseconds(span));
            }
            // Each arm starts on an idle device, so no arm pays for the one before it.
            gpu.poll(wgpu::PollType::wait_indefinitely())
                .expect("the device answered");
        }
    }
    if check {
        println!("check: every arm drew and the word read back unwritten");
        return;
    }
    println!(
        "{rounds} rounds per arm, interleaved, window {}×{}:",
        WINDOW.0, WINDOW.1
    );
    for (name, samples) in arms.iter().zip(spans.iter_mut()) {
        samples.sort_by(f64::total_cmp);
        let minimum = samples.first().copied().unwrap_or(f64::NAN);
        let median = samples.get(samples.len() / 2).copied().unwrap_or(f64::NAN);
        println!("  {name:<16} minimum {minimum:7.3} ms   median {median:7.3} ms");
    }
}
