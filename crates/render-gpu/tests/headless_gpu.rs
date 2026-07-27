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
