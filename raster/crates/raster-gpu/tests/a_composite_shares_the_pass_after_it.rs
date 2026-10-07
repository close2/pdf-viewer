//! A child's composite is drawn first in the pass of the marks after it, and a plan whose
//! first op is a child is cleared by that composite's pass (ADR 1618).
//!
//! Both are pass merges that must leave every byte where two passes put it. What each test
//! holds is the part of the merge that could go wrong without the digests of a corpus to
//! say so: the order of the two draws in one pass, the scissor between them, and the
//! clear of an accumulator handed back by the pool with another plan's pixels in it
//! (`layers.rs`'s reuse argument). Every expected value is a closed form of ISO 32000-2
//! §11.3.6 for opaque marks, which composite to the mark painted last.

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

fn group() -> GroupSpec {
    GroupSpec {
        alpha: 1.0,
        blend: BlendMode::Normal,
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
const GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
const BLUE: Color = Color::new(0.0, 0.0, 1.0, 1.0);

fn render(device: &mut Device, scene: &Scene) -> Vec<u8> {
    device
        .render(
            scene,
            &Viewport::full(SIZE, SIZE, Affine::IDENTITY),
            Target::Readback,
        )
        .expect("renders")
        .into_raster()
        .unwrap()
        .into_pixels()
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    pixels[at..at + 4].try_into().unwrap()
}

/// A group of one red square, then a blue square over half of it and past it.
fn group_then_marks() -> Scene {
    let mut builder = SceneBuilder::new();
    builder
        .group(group(), |body| {
            body.rect(
                square(8.0, 8.0, 24.0, 24.0),
                Affine::IDENTITY,
                RED,
                None,
                None,
            )
        })
        .unwrap();
    builder
        .rect(
            square(16.0, 16.0, 40.0, 40.0),
            Affine::IDENTITY,
            BLUE,
            None,
            None,
        )
        .unwrap();
    builder.finish()
}

/// **The composite is drawn before the marks after it, and the marks are not held to the
/// composite's scissor.** The blue square is painted after the group, so where they overlap
/// §11.3.6 gives blue; a pass that drew the marks first would leave red there, and one that
/// kept the composite's rectangle as the marks' scissor would leave the blue square's
/// outer part transparent.
#[test]
fn the_composite_is_drawn_first_and_the_marks_after_it_keep_their_scissor() {
    let mut device = device();
    let pixels = render(&mut device, &group_then_marks());
    assert_eq!(pixel(&pixels, 10, 10), [255, 0, 0, 255], "the group alone");
    assert_eq!(
        pixel(&pixels, 20, 20),
        [0, 0, 255, 255],
        "painted after the group"
    );
    assert_eq!(
        pixel(&pixels, 36, 36),
        [0, 0, 255, 255],
        "past the group's rectangle"
    );
    assert_eq!(pixel(&pixels, 50, 50), [0, 0, 0, 0], "marked by nothing");
}

/// A caller-owned target, which is the one kind a frame patches rather than redraws.
fn target_texture(device: &Device) -> wgpu::Texture {
    let (gpu, _) = device.wgpu();
    gpu.create_texture(&wgpu::TextureDescriptor {
        label: Some("merged pass target"),
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
        label: Some("merged pass readback"),
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

/// **Under a damage patch the merged pass draws the same closed forms** inside the damage,
/// and nothing outside it: the scissor the marks take after the composite is the damage
/// box's, not the composite's own. The target holds white first, so a pixel the patch
/// left alone reads white.
#[test]
fn a_patched_frame_draws_the_merged_pass_inside_the_damage() {
    let mut device = device();
    let target = target_texture(&device);
    let mut white = SceneBuilder::new();
    let whole = square(0.0, 0.0, SIZE as f32, SIZE as f32);
    white
        .rect(
            whole,
            Affine::IDENTITY,
            Color::new(1.0, 1.0, 1.0, 1.0),
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
    let damage = [square(12.0, 12.0, 38.0, 38.0)];
    let viewport = Viewport {
        width: SIZE,
        height: SIZE,
        transform: Affine::IDENTITY,
        damage: &damage,
    };
    device
        .render(&group_then_marks(), &viewport, Target::Texture(&target))
        .expect("the patch");
    let pixels = read_texture(&device, &target);
    assert_eq!(
        pixel(&pixels, 14, 14),
        [255, 0, 0, 255],
        "the group, inside the damage"
    );
    assert_eq!(
        pixel(&pixels, 20, 20),
        [0, 0, 255, 255],
        "painted after the group"
    );
    assert_eq!(
        pixel(&pixels, 36, 36),
        [0, 0, 255, 255],
        "past the group's rectangle"
    );
    assert_eq!(
        pixel(&pixels, 10, 10),
        [255, 255, 255, 255],
        "outside the damage"
    );
    assert_eq!(
        pixel(&pixels, 39, 39),
        [255, 255, 255, 255],
        "outside the damage"
    );
}

/// **A plan whose first op is a child is cleared by that child's composite pass**, so a
/// texture the pool hands it with a sibling's pixels in it shows none of them. The first
/// sibling is a green group at the top left; the second is a group of the same size
/// elsewhere, whose first op is a red group and whose corners are two blue dots — so its
/// accumulator is the first sibling's texture, handed back with green in it, and every
/// pixel of it neither group marks must come out transparent (§11.4.5).
#[test]
fn a_plan_that_opens_with_a_child_is_cleared_by_the_composites_pass() {
    let mut device = device();
    let mut builder = SceneBuilder::new();
    builder
        .group(group(), |body| {
            body.rect(
                square(0.0, 0.0, 20.0, 20.0),
                Affine::IDENTITY,
                GREEN,
                None,
                None,
            )
        })
        .unwrap();
    builder
        .group(group(), |body| {
            body.group(group(), |inner| {
                inner.rect(
                    square(45.0, 45.0, 50.0, 50.0),
                    Affine::IDENTITY,
                    RED,
                    None,
                    None,
                )
            })?;
            body.rect(
                square(40.0, 40.0, 41.0, 41.0),
                Affine::IDENTITY,
                BLUE,
                None,
                None,
            )?;
            body.rect(
                square(59.0, 59.0, 60.0, 60.0),
                Affine::IDENTITY,
                BLUE,
                None,
                None,
            )
        })
        .unwrap();
    let pixels = render(&mut device, &builder.finish());
    assert_eq!(
        pixel(&pixels, 10, 10),
        [0, 255, 0, 255],
        "the first sibling"
    );
    assert_eq!(pixel(&pixels, 47, 47), [255, 0, 0, 255], "the inner group");
    assert_eq!(pixel(&pixels, 40, 40), [0, 0, 255, 255], "a corner dot");
    for (x, y) in [(42, 42), (55, 44), (44, 55), (55, 55)] {
        assert_eq!(
            pixel(&pixels, x, y),
            [0, 0, 0, 0],
            "inside the second sibling, marked by nothing, at ({x}, {y})"
        );
    }
}
