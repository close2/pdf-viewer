//! A composite's backdrop is copied out of its accumulator by a transfer (ADR 1630).
//!
//! The transfer is exact by definition; what it can get wrong is *where*: the rectangle it
//! reads is `child ∩ parent` stated in the accumulator's own texels, which sit at the
//! plan's region rather than at the frame's origin (ADR 0036). Each test composites a
//! child under §11.3.5.2's `Multiply` — a blend that reads the backdrop — onto a parent
//! group whose region starts away from the origin and whose backdrop changes colour inside
//! the child's rectangle, so a copy read one texel off in either axis puts the wrong colour
//! beside the edge. Every expected value is the closed form of §11.3.6 for opaque marks:
//! an opaque source over an opaque backdrop is `B(cb, cs)`, and `Multiply` is `cb × cs`.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]

use raster_gpu::{Device, Options, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, Compose, GroupSpec, Point, Rect, Scene, SceneBuilder,
};

const SIZE: u32 = 64;

fn device() -> Device {
    Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        ..Options::default()
    })
    .expect("llvmpipe is present wherever this suite runs")
}

fn group(blend: BlendMode) -> GroupSpec {
    GroupSpec {
        alpha: 1.0,
        blend,
        clip: None,
        knockout: false,
        mask: None,
        isolated: true,
        compose: Compose::SrcOver,
    }
}

fn square(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::new(Point::new(x0, y0), Point::new(x1, y1))
}

const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
const YELLOW: Color = Color::new(1.0, 1.0, 0.0, 1.0);
const CYAN: Color = Color::new(0.0, 1.0, 1.0, 1.0);
const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

/// A parent group over 8..56 whose left half is red and right half yellow, the halves
/// meeting at x = 32 and, through a second split, at y = 32 — and inside it a cyan child
/// over 20..44 composited with `Multiply`. Red × cyan is black and yellow × cyan is green.
fn parent_with_a_multiplied_child() -> Scene {
    let mut builder = SceneBuilder::new();
    builder
        .group(group(BlendMode::Normal), |parent| {
            parent.rect(
                square(8.0, 8.0, 32.0, 56.0),
                Affine::IDENTITY,
                RED,
                None,
                None,
            )?;
            parent.rect(
                square(32.0, 8.0, 56.0, 32.0),
                Affine::IDENTITY,
                YELLOW,
                None,
                None,
            )?;
            parent.rect(
                square(32.0, 32.0, 56.0, 56.0),
                Affine::IDENTITY,
                RED,
                None,
                None,
            )?;
            parent.group(group(BlendMode::Multiply), |child| {
                child.rect(
                    square(20.0, 20.0, 44.0, 44.0),
                    Affine::IDENTITY,
                    CYAN,
                    None,
                    None,
                )
            })
        })
        .unwrap();
    builder.finish()
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    pixels[at..at + 4].try_into().unwrap()
}

const BLACK_OPAQUE: [u8; 4] = [0, 0, 0, 255];
const GREEN_OPAQUE: [u8; 4] = [0, 255, 0, 255];

/// The pixels on both sides of each colour edge inside the child, with what §11.3.6 puts
/// there: the backdrop's own colour, multiplied by cyan.
const EDGES: [((u32, u32), [u8; 4], &str); 6] = [
    (
        (31, 24),
        BLACK_OPAQUE,
        "red × cyan, left of the vertical edge",
    ),
    (
        (32, 24),
        GREEN_OPAQUE,
        "yellow × cyan, right of the vertical edge",
    ),
    (
        (40, 31),
        GREEN_OPAQUE,
        "yellow × cyan, above the horizontal edge",
    ),
    (
        (40, 32),
        BLACK_OPAQUE,
        "red × cyan, below the horizontal edge",
    ),
    ((20, 20), BLACK_OPAQUE, "the child's first texel"),
    ((43, 43), BLACK_OPAQUE, "the child's last texel"),
];

/// **The copy reads the accumulator at the child's rectangle inside the parent's**, not at
/// the frame's origin: the parent's region starts at (8, 8) and the child's at (20, 20),
/// and the colour edges inside the child land where the backdrop put them.
#[test]
fn the_backdrop_is_read_at_the_childs_rectangle_inside_the_parents() {
    let mut device = device();
    let pixels = device
        .render(
            &parent_with_a_multiplied_child(),
            &Viewport::full(SIZE, SIZE, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("renders")
        .into_raster()
        .unwrap()
        .into_pixels();
    for ((x, y), expected, what) in EDGES {
        assert_eq!(pixel(&pixels, x, y), expected, "{what}, at ({x}, {y})");
    }
    assert_eq!(pixel(&pixels, 10, 10), [255, 0, 0, 255], "the parent alone");
    assert_eq!(
        pixel(&pixels, 50, 10),
        [255, 255, 0, 255],
        "the parent alone"
    );
    assert_eq!(pixel(&pixels, 60, 60), [0, 0, 0, 0], "marked by nothing");
}

/// A caller-owned target, which is the one kind a frame patches rather than redraws.
fn target_texture(device: &Device) -> wgpu::Texture {
    let (gpu, _) = device.wgpu();
    gpu.create_texture(&wgpu::TextureDescriptor {
        label: Some("transfer target"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// The target's pixels, read through the raw handles the device exposes to a host.
fn read_texture(device: &Device, texture: &wgpu::Texture) -> Vec<u8> {
    let (gpu, queue) = device.wgpu();
    let buffer = gpu.create_buffer(&wgpu::BufferDescriptor {
        label: Some("transfer readback"),
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let pixels = slice.get_mapped_range().expect("mapped").to_vec();
    buffer.unmap();
    pixels
}

/// **Under a damage patch the transfer copies the whole of `child ∩ parent`, and the frame
/// still writes only the damage**: the composite that reads the copy is scissored to the
/// damage, so the closed forms hold inside it and the white the target held first is left
/// outside it — including the part of the child's rectangle the damage does not reach.
#[test]
fn a_patched_frame_reads_the_transferred_backdrop_only_inside_the_damage() {
    let mut device = device();
    let target = target_texture(&device);
    let mut white = SceneBuilder::new();
    white
        .rect(
            square(0.0, 0.0, SIZE as f32, SIZE as f32),
            Affine::IDENTITY,
            WHITE,
            None,
            None,
        )
        .unwrap();
    device
        .render(
            &white.finish(),
            &Viewport::full(SIZE, SIZE, Affine::IDENTITY),
            Target::Texture(&target),
        )
        .expect("the white wash");
    let damage = [square(28.0, 16.0, 48.0, 36.0)];
    let viewport = Viewport {
        width: SIZE,
        height: SIZE,
        transform: Affine::IDENTITY,
        damage: &damage,
    };
    device
        .render(
            &parent_with_a_multiplied_child(),
            &viewport,
            Target::Texture(&target),
        )
        .expect("the patch");
    let pixels = read_texture(&device, &target);
    for ((x, y), expected, what) in EDGES.iter().take(4).copied() {
        assert_eq!(pixel(&pixels, x, y), expected, "{what}, at ({x}, {y})");
    }
    assert_eq!(
        pixel(&pixels, 46, 20),
        [255, 255, 0, 255],
        "the parent alone, inside the damage"
    );
    for (x, y) in [(22, 22), (43, 43), (27, 24), (48, 24)] {
        assert_eq!(
            pixel(&pixels, x, y),
            [255, 255, 255, 255],
            "outside the damage, at ({x}, {y})"
        );
    }
}
