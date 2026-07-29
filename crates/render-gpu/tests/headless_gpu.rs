//! Spike B: the GPU backend, verified headlessly.
//!
//! Everything here runs with no window and no display server, which is what lets it
//! run in CI on a software Vulkan implementation. Presenting to a window is the one
//! part that cannot be checked this way and is left to manual verification.
//!
//! # The cross-backend oracle
//!
//! The central test is [`cpu_and_gpu_agree_on_the_basic_scene`]. Both backends consume
//! the *same* [`test_scenes`] display list, so a difference cannot be blamed on
//! differing input — it is a backend defect. That is a far tighter check than
//! comparing against an external PDF viewer, where antialiasing, gamma and page-box
//! choices differ for entirely legitimate reasons.
//!
//! # Where the thresholds come from
//!
//! They were measured, not guessed. On this workstation (RADV, Radeon 890M) against
//! `tiny-skia`, the basic scene gives:
//!
//! ```text
//! mean error 0.0136/255   worst tile 0.44   differing channels 0.08%   max 28
//! ```
//!
//! The gates below sit an order of magnitude above those figures — loose enough not to
//! fail on driver or antialiasing noise, tight enough that a real defect cannot pass.
//! For scale, a single missing shape pushes the worst tile above 150.

#![expect(
    clippy::panic,
    reason = "test code: an explanatory panic is the intended failure mode when no GPU \
              adapter is present, since skipping would report success while verifying \
              nothing"
)]

use pdf_render::{Rasterizer, TargetSpec};
use raster_compare::Comparison;
use render_cpu::CpuRasterizer;
use render_gpu::{GpuContext, GpuRasterizer};

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Broad-difference gate. Catches gamma, colour-space and inversion errors.
const MAX_MEAN_ERROR: f64 = 0.5;
/// Localised-difference gate. Catches missing or misplaced geometry.
const MAX_WORST_TILE_ERROR: f64 = 5.0;
/// At most this fraction of channels may differ noticeably.
const MAX_DIFFERING_FRACTION: f64 = 0.01;

/// Builds a GPU rasteriser, or explains why the whole suite should fail.
///
/// Deliberately does **not** skip when no adapter is present. A silently skipped GPU
/// suite is worse than a failing one: it reports success while testing nothing, and
/// this is exactly the environment-dependent failure that would go unnoticed. CI
/// installs `mesa-vulkan-drivers` so that a software adapter always exists.
fn gpu() -> GpuRasterizer {
    match GpuRasterizer::new_headless() {
        Ok(rasterizer) => rasterizer,
        Err(e) => panic!(
            "no GPU adapter available: {e}\n\
             Install a Vulkan driver (vulkan-radeon, or mesa-vulkan-drivers for a \
             software one). These tests do not skip, because a skipped GPU suite \
             reports success while verifying nothing."
        ),
    }
}

fn assert_within_tolerance(what: &str, c: Comparison) {
    assert!(
        c.mean_error < MAX_MEAN_ERROR,
        "{what}: mean error {:.4} exceeds {MAX_MEAN_ERROR}",
        c.mean_error
    );
    assert!(
        c.worst_tile_error < MAX_WORST_TILE_ERROR,
        "{what}: worst tile error {:.4} at {:?} exceeds {MAX_WORST_TILE_ERROR}",
        c.worst_tile_error,
        c.worst_tile_at
    );
    assert!(
        c.differing_fraction < MAX_DIFFERING_FRACTION,
        "{what}: {:.4}% of channels differ, exceeding {:.4}%",
        c.differing_fraction * 100.0,
        MAX_DIFFERING_FRACTION * 100.0
    );
}

#[test]
fn renders_without_a_window_or_display_server() {
    let list = test_scenes::basic();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("A4 target is valid");

    let raster = gpu()
        .rasterize(&list, target)
        .expect("basic scene is supported");

    assert_eq!((raster.width, raster.height), (595, 842));
    assert_eq!(
        raster.data.len(),
        595 * 842 * 4,
        "four bytes per pixel, no row padding"
    );
}

/// The oracle: two independent rasterisers, identical input.
#[test]
fn cpu_and_gpu_agree_on_the_basic_scene() {
    let list = test_scenes::basic();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");

    assert_within_tolerance(
        "basic scene",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// A diagonal edge is where the two rasterisers compute coverage differently, so this
/// sets the realistic floor for the tolerances above.
#[test]
fn cpu_and_gpu_agree_on_a_diagonal_stroke() {
    let list = test_scenes::diagonal_stroke();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");

    assert_within_tolerance(
        "diagonal stroke",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// Curves are flattened independently by each backend, making this the scene most
/// likely to expose a genuine geometric disagreement rather than edge antialiasing.
#[test]
fn cpu_and_gpu_agree_on_curves() {
    let list = test_scenes::curves();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");

    assert_within_tolerance(
        "curves",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// A transparency group is one object to both backends, or neither is the other's oracle.
///
/// ISO 32000-2 §11.4.1. `tiny-skia` composites the group into a buffer of its own and draws
/// that buffer once; Vello pushes a layer, which is the same construction expressed as a
/// stack. Those are different enough mechanisms that agreement is evidence rather than
/// tautology — and the failure mode this catches is the quiet one, a backend that applies the
/// group's alpha to each element and doubles the overlap.
#[test]
fn cpu_and_gpu_agree_on_a_transparency_group() {
    let list = test_scenes::transparency_group();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");

    assert_within_tolerance(
        "transparency group",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// A soft mask is the same mask on both backends, values and all (§11.5).
///
/// The two mechanisms could hardly be less alike: `tiny-skia` builds an eight-bit coverage
/// mask and multiplies it into the clip, while Vello renders the mask's group to a texture
/// of its own and composites it back with `Compose::DestIn`. What they *share* is
/// `pdf_render::SoftMask::value`, which turns rendered pixels into mask values — and that is
/// the point of the scene: the derivation is one function, so a difference here is a
/// difference in how a mask is applied rather than in what it says.
///
/// `test_scenes::soft_mask` is coloured rather than grey on purpose. Both rasterisers offer
/// a luminance mask of their own — `tiny_skia::MaskType::Luminance` and Vello's
/// `push_luminance_mask_layer` — and both use coefficients that are not §11.5.3's. On grey
/// artwork every formula agrees; on the green square in this scene they are a fifth of the
/// mask's range apart, so reaching for either library's version fails this test.
#[test]
fn cpu_and_gpu_agree_on_a_soft_mask() {
    let list = test_scenes::soft_mask();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");

    assert_within_tolerance(
        "soft mask",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// Vello hands back straight alpha, and this backend used to convert it as if it were
/// premultiplied.
///
/// The defect was invisible for fifteen sessions because the page was rendered onto an opaque
/// background: every pixel came back with an alpha of 255, and the conversion is the identity
/// there. §11.4.7's page group is what made it visible — the page is now drawn onto
/// transparency and imposed on the medium afterwards, so a partly covered pixel reaches the
/// conversion with an alpha of its own.
///
/// A half-covered pixel of a 50% grey is the whole test: straight alpha is `[128, 0, 0, 128]`,
/// and dividing a colour by its own coverage gives `[255, 0, 0, 128]`. The CPU backend is the
/// oracle for the expected value, as it is for everything else here.
#[test]
fn vello_hands_back_straight_alpha() {
    use pdf_render::{
        BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Size,
        Transform,
    };

    let mut list = DisplayList::new(Size::new(20.0, 20.0));
    let mut path = Path::new();
    // The right edge falls at x = 10.5, so column 10 is covered exactly half.
    path.push(PathCommand::MoveTo(Point::new(2.0, 2.0)));
    path.push(PathCommand::LineTo(Point::new(10.5, 2.0)));
    path.push(PathCommand::LineTo(Point::new(10.5, 18.0)));
    path.push(PathCommand::LineTo(Point::new(2.0, 18.0)));
    path.push(PathCommand::Close);
    list.push(Command::Fill {
        path: std::sync::Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::rgb(0.5, 0.0, 0.0)),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    // A transparent medium, because an opaque one hides exactly this: it takes every alpha
    // back to 255 before the raster leaves the backend.
    let raster = gpu()
        .with_background(Color::TRANSPARENT)
        .rasterize(&list, target)
        .expect("supported");
    let cpu = CpuRasterizer::new()
        .with_background(Color::TRANSPARENT)
        .rasterize(&list, target)
        .expect("supported");

    let at = ((10 * 20 + 10) * 4) as usize;
    let edge = &raster.data[at..at + 4];
    assert_eq!(
        edge,
        &cpu.data[at..at + 4],
        "half-covered pixel: GPU {edge:?} against the CPU oracle"
    );
    assert_eq!(
        edge[3], 128,
        "the pixel must be half covered, or the test proves nothing"
    );
    assert_eq!(
        edge[0], 128,
        "the colour is the colour, not the colour over its own coverage"
    );
}

/// Row padding in the GPU readback, which is invisible in ordinary sizes.
///
/// A width of 101 pixels is 404 bytes per row, padded to 512. A backend that fails to
/// strip the padding produces a progressively sheared image, so a uniform fill makes
/// the defect unmissable: every pixel must be exactly the fill colour.
#[test]
fn readback_strips_row_padding_at_an_unaligned_width() {
    let list = test_scenes::unaligned_full_bleed();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    assert_eq!(
        target.width, 101,
        "the scene must exercise an unaligned row length"
    );

    let raster = gpu().rasterize(&list, target).expect("supported");

    assert_eq!(
        raster.data.len(),
        (101 * 37 * 4) as usize,
        "padding must not survive"
    );

    // Interior pixels only: page-edge pixels are antialiased against the boundary.
    for y in 1..36u32 {
        for x in 1..100u32 {
            let index = ((y * 101 + x) * 4) as usize;
            let pixel = &raster.data[index..index + 4];
            assert_eq!(
                pixel,
                [255, 0, 0, 255],
                "pixel ({x},{y}) is {pixel:?}, not the fill colour — rows are misaligned"
            );
        }
    }
}

/// Hardware and software Vulkan produced byte-identical output when this was written,
/// because Vello's compute pipeline has no driver-dependent fixed-function
/// rasterisation. That is worth pinning: it means a CI diff on `lavapipe` is
/// reproducible on a workstation GPU, and that goldens need not be per-backend.
///
/// If this ever fails, the conclusion is not that the code is broken but that the
/// assumption no longer holds and goldens must become per-adapter.
#[test]
fn hardware_and_software_adapters_agree_exactly() {
    let software = match GpuContext::new_headless_software() {
        Ok(context) => context,
        Err(e) => panic!(
            "no software Vulkan adapter: {e}\n\
             Install vulkan-swrast (Arch) or mesa-vulkan-drivers (Debian/Ubuntu)."
        ),
    };

    let hardware = gpu();
    if hardware.context().is_software() {
        // Both adapters are the same software implementation, so this would compare a
        // render against itself and prove nothing. Say so rather than passing quietly.
        println!("only a software adapter is present; comparison skipped as vacuous");
        return;
    }

    let list = test_scenes::basic();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let mut software = GpuRasterizer::with_context(software);
    let mut hardware = hardware;
    let sw = software.rasterize(&list, target).expect("supported");
    let hw = hardware.rasterize(&list, target).expect("supported");

    let comparison = raster_compare::compare(&sw, &hw).expect("same size");
    assert_eq!(
        comparison.max_error,
        0,
        "hardware ({}) and software ({}) diverged by up to {} levels; \
         goldens can no longer be shared between adapters",
        hardware.context().adapter_description(),
        software.context().adapter_description(),
        comparison.max_error
    );
}

/// Builds a page-sized display list whose only content is one shading.
fn shaded_page(kind: pdf_render::ShadingKind) -> pdf_render::DisplayList {
    use pdf_render::{
        BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Shading,
        Size, Transform,
    };

    let size = Size::new(200.0, 200.0);
    let mut list = DisplayList::new(size);

    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(190.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(190.0, 190.0)));
    path.push(PathCommand::LineTo(Point::new(10.0, 190.0)));
    path.push(PathCommand::Close);

    let _ = Color::BLACK;
    list.push(Command::Fill {
        path: std::sync::Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Shading(std::sync::Arc::new(Shading {
            kind,
            transform: Transform::IDENTITY,
        })),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// A red-to-blue ramp, shared by the gradient scenes below.
fn ramp() -> pdf_render::Ramp {
    pdf_render::Ramp::sample(|t| pdf_render::Color::rgb(1.0 - t, 0.0, t))
}

/// Both backends implement axial shadings natively, so they must agree on one.
///
/// This is what makes the GPU shading work checkable at all: the CPU backend's colours
/// have already been pinned against values derived from the specification, so agreement here
/// carries that verification across.
#[test]
fn cpu_and_gpu_agree_on_an_axial_shading() {
    let list = shaded_page(pdf_render::ShadingKind::Axial {
        start: pdf_render::Point::new(10.0, 0.0),
        end: pdf_render::Point::new(190.0, 0.0),
        ramp: ramp(),
        extend: (true, true),
    });
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");
    assert_within_tolerance(
        "axial shading",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// An image, which no other scene here draws.
///
/// The gap that let the CPU backend compose the device transform into an image's pattern
/// as well as into the path it fills — drawing a whole photograph as one flat colour —
/// while this suite stayed green. Vello takes a single transform for an image and had it
/// right, so the two backends could have been compared at any point and would have
/// disagreed immediately.
#[test]
fn cpu_and_gpu_agree_on_an_image() {
    use pdf_render::{BlendMode, Command, DisplayList, Image, Size, Transform};

    // A 4×4 checkerboard of distinguishable colours: a placement error that survives a
    // flat image does not survive this one.
    let mut data = Vec::with_capacity(4 * 4 * 4);
    for row in 0..4_u8 {
        for column in 0..4_u8 {
            data.extend_from_slice(&[row * 60, column * 60, 255 - row * 40, 255]);
        }
    }

    let mut list = DisplayList::new(Size::new(200.0, 200.0));
    list.push(Command::Image {
        image: Image {
            width: 4,
            height: 4,
            data: data.into(),
            // The default of §8.9.5.3, so this scene also holds the two backends to the
            // same *sampler*: sixteen samples over 120x80 pixels is magnification, where
            // one backend filtering and the other not would be visible everywhere.
            interpolate: false,
        },
        // Deliberately not the whole page, and not square, so that an inverted or
        // transposed mapping moves colours rather than merely permuting a symmetry.
        transform: Transform::scale(120.0, 80.0).then(Transform::translate(40.0, 60.0)),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    for scale in [1.0, 2.0] {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
        let cpu = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        let gpu = gpu().rasterize(&list, target).expect("supported");
        assert_within_tolerance(
            &format!("image at scale {scale}"),
            raster_compare::compare(&cpu, &gpu).expect("same size"),
        );
    }
}

/// The same axial shading, running up the page instead of across it, at two scales.
///
/// Every shading scene above runs its gradient along x, and both backends once applied
/// the device transform to a paint twice — which at a scale of 1.0 is exactly a mirror
/// about the page's horizontal centre and therefore invisible to a gradient that does
/// not vary in y. Both were wrong, so both agreed, and this suite passed throughout.
///
/// Agreement is evidence only where the two can fail independently. This scene varies in
/// the axis the shared defect moved, at a scale where the double application does not
/// cancel; `render-cpu`'s `shading_placement.rs` pins the same geometry against the
/// clause, so the pair together says both *agree* and are *right*.
#[test]
fn cpu_and_gpu_agree_on_a_vertical_axial_shading() {
    let list = shaded_page(pdf_render::ShadingKind::Axial {
        start: pdf_render::Point::new(0.0, 10.0),
        end: pdf_render::Point::new(0.0, 190.0),
        ramp: ramp(),
        extend: (true, true),
    });

    for scale in [1.0, 2.0] {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
        let cpu = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        let gpu = gpu().rasterize(&list, target).expect("supported");
        assert_within_tolerance(
            &format!("vertical axial shading at scale {scale}"),
            raster_compare::compare(&cpu, &gpu).expect("same size"),
        );
    }
}

/// A two-circle radial, which is PDF's general case and which both rasterisers take
/// directly rather than as a single-circle approximation.
#[test]
fn cpu_and_gpu_agree_on_a_radial_shading() {
    let list = shaded_page(pdf_render::ShadingKind::Radial {
        start: pdf_render::Point::new(100.0, 100.0),
        start_radius: 10.0,
        end: pdf_render::Point::new(100.0, 100.0),
        end_radius: 85.0,
        ramp: ramp(),
        extend: (true, true),
    });
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");
    assert_within_tolerance(
        "radial shading",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// A mesh, which neither rasteriser can shade natively and which both therefore
/// subdivide into flat triangles.
///
/// The two do that with the same thresholds but not the same rasteriser, so this is the
/// scene most likely to expose a difference in how the subdivision was applied.
#[test]
fn cpu_and_gpu_agree_on_a_mesh_shading() {
    use pdf_render::{Color, Point, Triangle};

    let triangles = vec![
        Triangle {
            points: [
                Point::new(20.0, 20.0),
                Point::new(180.0, 20.0),
                Point::new(20.0, 180.0),
            ],
            colours: [
                Color::rgb(1.0, 0.0, 0.0),
                Color::rgb(0.0, 1.0, 0.0),
                Color::rgb(0.0, 0.0, 1.0),
            ],
        },
        Triangle {
            points: [
                Point::new(180.0, 20.0),
                Point::new(180.0, 180.0),
                Point::new(20.0, 180.0),
            ],
            colours: [
                Color::rgb(0.0, 1.0, 0.0),
                Color::rgb(1.0, 1.0, 0.0),
                Color::rgb(0.0, 0.0, 1.0),
            ],
        },
    ];

    let list = shaded_page(pdf_render::ShadingKind::Mesh {
        triangles: triangles.into(),
    });
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");
    assert_within_tolerance(
        "mesh shading",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// A deeply reduced image, where the two backends' own filters read different taps.
///
/// The scene the ninth image test could not be: `cpu_and_gpu_agree_on_an_image`
/// *magnifies* sixteen samples, and magnification is the one direction where
/// `tiny-skia`'s and Vello's samplers are held to the same answer by
/// `Image::is_smoothed` alone. Under reduction they are not — a four-tap bilinear and
/// whatever Vello's `Medium` does read different neighbourhoods of a grid eight times
/// finer than the pixels — so an area average applied in only one of them, or applied
/// with different block boundaries, separates them here and nowhere else. That is trap
/// 2's other half: a scene that cannot fail in the direction the change moves is not a
/// test of it. ADR 0025.
#[test]
fn cpu_and_gpu_agree_on_a_deeply_reduced_image() {
    use pdf_render::{BlendMode, Command, DisplayList, Image, Size, Transform};

    // Fine detail at the sample grid — a pattern no reduction can resolve, so any
    // disagreement about *which* samples are read shows as a different average.
    //
    // The image has to be large and cover most of the page, not merely be reduced: the
    // first draft of this scene shrank 64x64 into an 8x4 corner, which differs on 32
    // pixels of 40 000 and passes `MAX_DIFFERING_FRACTION` with the GPU filter removed
    // altogether. A test of a filter has to put the filtered pixels where the tolerance
    // can see them.
    let (width, height) = (800u32, 800u32);
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        for column in 0..width {
            let on = (row % 2 == 0) ^ (column % 3 == 0);
            let value = if on { 255 } else { 0 };
            data.extend_from_slice(&[value, u8::try_from(column % 256).unwrap_or(255), 128, 255]);
        }
    }

    let mut list = DisplayList::new(Size::new(200.0, 200.0));
    list.push(Command::Image {
        image: Image {
            width,
            height,
            data: data.into(),
            interpolate: false,
        },
        // Five samples per pixel across and ten down, so the two axes reduce by different
        // factors and a filter that used one factor for both would show.
        transform: Transform::scale(160.0, 80.0).then(Transform::translate(20.0, 60.0)),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });

    for scale in [1.0, 2.0] {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
        let cpu = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        let gpu = gpu().rasterize(&list, target).expect("supported");
        assert_within_tolerance(
            &format!("reduced image at scale {scale}"),
            raster_compare::compare(&cpu, &gpu).expect("same size"),
        );
    }
}

/// The two backends agree on the thinnest line the device can draw.
///
/// The scene the fifteenth session did not have. ISO 32000-2 §8.4.3.2 makes a zero width
/// "the thinnest line that can be rendered at device resolution: 1 device pixel wide", and
/// `tiny-skia` implements that as its hairline mode while `kurbo` expands a zero-width stroke
/// into an **empty outline** — so the GPU drew nothing at all, on every `0 w` line in every
/// document, and the eleven scenes before this one all stroked a width the document stated.
/// A backend-specific convention that one of two backends happens to share with PDF is not a
/// reading of the clause; `Stroke::device_width` is where the rule lives now.
///
/// §10.7.5's stroke adjustment is the same substitution on a different condition, so it is in
/// the same scene: a 0.2-unit line under `/SA` is a fifth of a pixel at scale 1.0 and must
/// come out as a whole one on both backends.
///
/// Two scales, because the substituted width is a *reciprocal* of the scale and one scale
/// cannot tell a reciprocal from a constant.
#[test]
fn cpu_and_gpu_agree_on_the_thinnest_line_the_device_draws() {
    use pdf_render::{
        BlendMode, Color, Command, DisplayList, Paint, Path, PathCommand, Point, Size, Stroke,
        Transform,
    };
    use std::sync::Arc;

    // Both a horizontal and a diagonal line: a horizontal one at a half-integer y covers
    // exactly one row, which is where a wrong width shows as a wrong ink level, and a
    // diagonal one is where the two rasterisers' coverage differs and sets the floor for
    // these tolerances.
    let scene = |stroke: Stroke| {
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        for (from, to) in [
            (Point::new(20.0, 100.5), Point::new(180.0, 100.5)),
            (Point::new(20.0, 20.0), Point::new(180.0, 80.0)),
        ] {
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(from));
            path.push(PathCommand::LineTo(to));
            list.push(Command::Stroke {
                path: Arc::new(path),
                transform: Transform::IDENTITY,
                stroke: stroke.clone(),
                paint: Paint::Solid(Color::BLACK),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        list
    };

    let cases = [
        (
            "a zero-width stroke",
            Stroke {
                width: 0.0,
                ..Stroke::default()
            },
        ),
        (
            "a stroke-adjusted hairline",
            Stroke {
                width: 0.2,
                adjust: true,
                ..Stroke::default()
            },
        ),
    ];
    for (what, stroke) in cases {
        let list = scene(stroke);
        for scale in [1.0, 2.5] {
            let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
            let cpu = CpuRasterizer::new()
                .rasterize(&list, target)
                .expect("supported");
            let gpu = gpu().rasterize(&list, target).expect("supported");
            assert_within_tolerance(
                &format!("{what} at scale {scale}"),
                raster_compare::compare(&cpu, &gpu).expect("same size"),
            );
        }
    }
}
