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
    assert_within(what, c, MAX_DIFFERING_FRACTION);
}

/// The same gate with the differing-channel bound named by the caller.
///
/// Only the third of the three thresholds is ever loosened, and only for a scene that is
/// mostly *boundary*: it counts channels that differ at all, so a page of large circles —
/// where every mark is edge — trips it on antialiasing alone, while the two thresholds that
/// catch a missing or misplaced mark stay where they are.
fn assert_within(what: &str, c: Comparison, max_differing: f64) {
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
        c.differing_fraction < max_differing,
        "{what}: {:.4}% of channels differ, exceeding {:.4}%",
        c.differing_fraction * 100.0,
        max_differing * 100.0
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

/// §11.4.6's knockout is the same rule on both backends.
///
/// The clause's rule is one sentence and each backend reaches it through its own library's
/// spelling of Porter-Duff Source: `tiny-skia` sets a per-draw blend mode, Vello has no such
/// parameter and composites a layer clipped to the element's shape with `Compose::Copy`.
/// Trap 2's question — what does every scene leave at its default — is what this answers for
/// the group flag that was `false` in every scene until the seventy-first session.
#[test]
fn cpu_and_gpu_agree_on_a_knockout_group() {
    let list = test_scenes::knockout_group();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");

    assert_within_tolerance(
        "knockout group",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// §11.4.6's *stated* shape is the same pair of operators on both backends.
///
/// The scene above leaves every element's shape equal to its coverage, which is the half of
/// the clause a single Porter-Duff Source expresses. This one states the shape separately —
/// `Command::Shaped` — so each backend draws the clause's two stages as two marks:
/// Destination-Out with the shape, then addition of the object. `tiny-skia` sets both as
/// per-draw modes; Vello has neither as a parameter and reaches them through two layers. Trap
/// 2 again: where one backend states a rule directly and the other builds it, the built one
/// needs a scene at the fractional coverage where the two constructions differ, which is what
/// the scene's wedge is for.
#[test]
fn cpu_and_gpu_agree_on_a_knockout_group_that_states_its_shape() {
    let list = test_scenes::knockout_stated_shape();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");

    assert_within_tolerance(
        "knockout group with a stated shape",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );
}

/// §11.4.4's non-isolated group is refused **by name** on this backend.
///
/// A Vello layer begins fully transparent — §11.4.5's initial backdrop — and a scene cannot
/// read what it has drawn so far, so there is no way to seed one with the page a
/// non-isolated group's elements have to composite onto. The frame goes to the CPU backend,
/// which is what `CLAUDE.md` keeps that backend for; drawing the isolated group it is not
/// would be a plausible wrong picture, and this fails if that ever becomes the answer.
#[test]
fn the_gpu_refuses_a_non_isolated_group() {
    let list = test_scenes::non_isolated_group();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let refusal = gpu()
        .rasterize(&list, target)
        .expect_err("Vello has no layer seeded from its backdrop")
        .to_string();
    assert!(
        refusal.contains("§11.4.4") && refusal.contains("non-isolated"),
        "the refusal names the clause and what it needs: {refusal}"
    );
    // And the CPU backend draws it: §11.3.5.2's Multiply of blue over green leaves nothing
    // of the blue, so the group's half-alpha result over the green page is a darker green
    // rather than the teal an isolated group would give.
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the correctness oracle draws what the device refuses");
}

/// §11.4.7's four-component page is refused **by name** on this backend.
///
/// §11.3.4 applies the compositing formula per component, so a page whose blending colour
/// space has four of them is two rasters over one geometry — and a Vello scene renders one,
/// with no place in this backend to hold the second. Drawing the chromatic list alone would
/// paint the page in the complements of cyan, magenta and yellow with no black in it at all,
/// which is a plausible wrong picture rather than an obvious one, so the frame goes to the
/// CPU backend instead. `render-quorra` draws it since the four-hundred-and-thirty-ninth
/// session, on two `Target::Readback` renders against one device; this fails if the refusal
/// here ever becomes silent.
#[test]
fn the_gpu_refuses_a_four_component_page() {
    let list = test_scenes::four_component_page();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let refusal = gpu()
        .rasterize(&list, target)
        .expect_err("a Vello scene renders one raster and this page is two")
        .to_string();
    assert!(
        refusal.contains("§11.4.7") && refusal.contains("four-component"),
        "the refusal names the clause and what it needs: {refusal}"
    );
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the correctness oracle draws what the device refuses");
}

/// §8.7.4.5.2's sampled shading is refused **by name** on this backend.
///
/// A function-based shading's colours resolve to a grid at the device's own resolution
/// (`Shading::sampled_at`), and a grid is not a brush any Vello gradient can express. The
/// sibling backends draw it their own way — a pattern on the CPU, an image clipped to the
/// path on quorra, which is what the window presents with — and this backend reports, which
/// keeps the two honestly different: the comparison harness excludes a page a backend says
/// it cannot draw instead of blaming the difference on the GPU. This fails if the refusal
/// ever becomes silent, and the CPU draw beside it fails if the oracle stops covering what
/// the device refuses.
#[test]
fn the_gpu_refuses_a_sampled_shading_by_name() {
    let list = test_scenes::sampled_shading();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let refusal = gpu()
        .rasterize(&list, target)
        .expect_err("a device-resolved grid is not a brush")
        .to_string();
    assert!(
        refusal.contains("Sampled"),
        "the refusal names the kind it cannot draw: {refusal}"
    );
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the correctness oracle draws what the device refuses");
}

/// §11.4.6's non-isolated knockout group is refused **by name** on this backend.
///
/// Each element composites with the group's *initial* backdrop — here the group's own,
/// since the group is non-isolated — which needs that backdrop retained beside the
/// accumulation and a scratch per element. A Vello layer begins transparent and a scene
/// cannot read what it has drawn so far, so the refusal is the same one
/// [`the_gpu_refuses_a_non_isolated_group`] pins, reached through the knockout shape of the
/// command; this fails if either backdrop is ever silently substituted for the other.
#[test]
fn the_gpu_refuses_a_knockout_group_on_its_own_backdrop() {
    let list = test_scenes::knockout_group_on_its_own_backdrop();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let refusal = gpu()
        .rasterize(&list, target)
        .expect_err("Vello has no layer seeded from its backdrop")
        .to_string();
    assert!(
        refusal.contains("non-isolated"),
        "the refusal names what it needs: {refusal}"
    );
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the correctness oracle draws what the device refuses");
}

/// §11.6.6's group compositing in a four-component space is refused **by name** here.
///
/// The pair's colours are ink complements, resolved per pixel *after* the group
/// composites, and a scene under composition cannot be read back — the page-level
/// refusal of [`the_gpu_refuses_a_four_component_page`], one scope down. Drawing the
/// chromatic list alone would paint the group in the complements of cyan, magenta and
/// yellow with no black at all, which is a plausible wrong picture rather than an obvious
/// one, so this fails if the refusal ever becomes silent.
#[test]
fn the_gpu_refuses_a_group_in_its_own_blending_space() {
    let list = test_scenes::group_in_its_own_blending_space();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let refusal = gpu()
        .rasterize(&list, target)
        .expect_err("a scene under composition cannot resolve a pair per pixel")
        .to_string();
    assert!(
        refusal.contains("§11.6.6") && refusal.contains("four components"),
        "the refusal names the clause and what it needs: {refusal}"
    );
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the correctness oracle draws what the device refuses");
}

/// Every one of §11.3.5's sixteen blend modes is the same function on both backends, to
/// the channel.
///
/// # The scene that was missing
///
/// Reading clause 11 as a family in the thirty-seventh session found that not one
/// cross-backend scene selected a blend mode at all: every `Command` in every other scene
/// here carries `BlendMode::Normal`. So the two backends' sixteen blend functions had never
/// been compared with each other, and trap 2 says what that means — a decision either backend
/// can make alone is a decision neither has made.
///
/// Table 135's four are why it matters. Hue, Saturation, Color and Luminosity are
/// *non-separable*: each is defined over all three components at once through the clause's
/// `Lum`, `ClipColor`, `SetLum` and `SetSat` functions, so no per-component formula produces
/// them and one of them being wrong still draws a plausible picture.
///
/// # What it found, and why the closed form settled it
///
/// Twelve separable modes and `Saturation` agreed exactly at once. `Hue`, `Color` and
/// `Luminosity` differed by 113 of 255, and that was not a tie: §11.3.5.3's arithmetic says
/// which is right.
///
/// Take red painted over blue in `Hue`, which the clause defines as
/// `SetLum(SetSat(Cs, Sat(Cb)), Lum(Cb))`. `Sat(blue)` is 1 and `SetSat(red, 1)` is red;
/// `Lum(blue)` is 0.11 and `Lum(red)` is 0.30, so `SetLum` adds −0.19 to each component and
/// gives (0.81, −0.19, −0.19). `ClipColor` then applies, because a component fell below 0:
/// with `L` = 0.11 and `n` = −0.19 the red component becomes
/// `L + (C − L) × L ÷ (L − n)` = 0.11 + 0.70 × 0.11 ÷ 0.30 = **0.367**, which is 94 in eight
/// bits. Vello produced 94. `tiny-skia` produced 207, which is 0.81 — the value *before*
/// `ClipColor`.
///
/// The three are now `render-cpu`'s own arithmetic (ADR 0047) and the list of disagreements
/// is empty. It stays here as a list rather than a tolerance, ratcheted in both directions,
/// because that is what makes a fourth mode joining it a failure rather than a footnote —
/// and because two independent implementations of a clause's formula agreeing to the channel
/// is the strongest evidence this project can produce about arithmetic.
#[test]
fn cpu_and_gpu_agree_on_every_blend_mode() {
    /// The modes whose two implementations are known to differ; empty since ADR 0047.
    const DISAGREE: [pdf_render::BlendMode; 0] = [];
    /// Four by four tiles, in the order `test_scenes::ALL_BLEND_MODES` lists them.
    const ACROSS: u32 = 4;

    // Still a release-build test, and the reason is not the one this comment used to give.
    // `tiny-skia`'s `u16x16` lanes are *meant* to wrap — that is what the SIMD instruction
    // they stand in for does — so Rust's debug-build overflow check fires inside the library
    // on modes that are perfectly correct: with the three non-separable ones no longer
    // reaching it at all, `lowp::overlay` still panics in `wide/u16x16_t.rs`, and `Overlay`
    // agrees with Vello to the channel in release. So an overflow panic in a dependency
    // whose arithmetic is modular is not evidence of anything, and ADR 0046 leaned on it —
    // see ADR 0047. What settled these three was the clause's closed form.
    if cfg!(debug_assertions) {
        return;
    }

    let list = test_scenes::blend_modes();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");
    assert_eq!((cpu.width, cpu.height), (gpu.width, gpu.height));

    let (tile_width, tile_height) = (cpu.width / ACROSS, cpu.height / ACROSS);
    let mut differing = Vec::new();
    for (index, mode) in test_scenes::ALL_BLEND_MODES.into_iter().enumerate() {
        let index = u32::try_from(index).expect("sixteen modes");
        let (column, row) = (index % ACROSS, index / ACROSS);
        let mut worst = 0u8;
        for y in row * tile_height..(row + 1) * tile_height {
            for x in column * tile_width..(column + 1) * tile_width {
                let at = ((y * cpu.width + x) * 4) as usize;
                for channel in 0..4 {
                    let (a, b) = (cpu.data[at + channel], gpu.data[at + channel]);
                    worst = worst.max(a.abs_diff(b));
                }
            }
        }
        if worst > 0 {
            differing.push((mode, worst));
        }
    }

    let named: Vec<pdf_render::BlendMode> = differing.iter().map(|(mode, _)| *mode).collect();
    assert_eq!(
        named,
        DISAGREE.to_vec(),
        "the set of blend modes the two backends disagree about has changed: {differing:?}"
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
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    let cpu = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
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
            kind: std::sync::Arc::new(kind),
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
        }
        .into(),
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

/// §8.7.4.5.4's cone, where the clause and every two-point conical gradient part company.
///
/// `test_scenes::radial_cone` has the geometry and why it is the one radial worth a scene of
/// its own: a point on it lies on two blend circles, the greater root is one `/Extend` refuses,
/// and the clause's "greatest value of s" is therefore the *lesser* one. Both backends leave
/// their gradient and draw `pdf_render::RadialRaster`'s bytes, so agreement here is agreement
/// about an evaluation rather than about two libraries' shaders.
#[test]
fn cpu_and_gpu_agree_on_a_radial_cone() {
    let list = test_scenes::radial_cone();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");
    assert_within_tolerance(
        "radial cone",
        raster_compare::compare(&cpu, &gpu).expect("same size"),
    );

    // And agreement about a blank page would be agreement about nothing, so the CPU raster is
    // checked against the clause's own answer at the point the arithmetic was done for. Device
    // (70, 100) is the ending circle's centre; the admissible root there is s = 5775/12075 =
    // 0.478261 and the ramp is `rgb(1 - t, 0, t)`, so the pixel is 133, 0, 122 — the *lesser*
    // root's colour, which a gradient never reaches.
    let at = (100 * cpu.width as usize + 70) * 4;
    let pixel = &cpu.data[at..at + 4];
    assert_eq!(
        pixel[3], 255,
        "the cone's interior is painted, not left clear"
    );
    assert!(
        pixel[0].abs_diff(133) <= 2 && pixel[1] == 0 && pixel[2].abs_diff(122) <= 2,
        "§8.7.4.5.4's colour at s = 0.478 is 133, 0, 122; the raster says {pixel:?}"
    );
}

/// A mesh, which neither rasteriser can shade natively and which both therefore draw as the
/// one raster `pdf_render::MeshRaster` builds.
///
/// The colours are shared and the *placement* is not, so what this scene can still catch is
/// the raster being put in the wrong place or sampled with a filter on one backend.
#[test]
fn cpu_and_gpu_agree_on_a_mesh_shading() {
    use pdf_render::{Color, Corners, Point, Triangle};

    let triangles = vec![
        Triangle {
            points: [
                Point::new(20.0, 20.0),
                Point::new(180.0, 20.0),
                Point::new(20.0, 180.0),
            ],
            corners: Corners::Colours([
                Color::rgb(1.0, 0.0, 0.0),
                Color::rgb(0.0, 1.0, 0.0),
                Color::rgb(0.0, 0.0, 1.0),
            ]),
        },
        Triangle {
            points: [
                Point::new(180.0, 20.0),
                Point::new(180.0, 180.0),
                Point::new(20.0, 180.0),
            ],
            corners: Corners::Colours([
                Color::rgb(0.0, 1.0, 0.0),
                Color::rgb(1.0, 1.0, 0.0),
                Color::rgb(0.0, 0.0, 1.0),
            ]),
        },
    ];

    let list = shaded_page(pdf_render::ShadingKind::Mesh {
        triangles: triangles.into(),
        ramp: None,
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

/// The same mesh with ISO 32000-2 §8.7.4.5.5's *parametric* corners, which the scene above
/// leaves at their default.
///
/// A mesh with a `/Function` carries one value per corner and a ramp beside the triangles, so
/// the colours a backend draws depend on something the coloured scene never hands it. Trap 2's
/// rule about a suite's defaults, one field over.
#[test]
fn cpu_and_gpu_agree_on_a_parametric_mesh_shading() {
    use pdf_render::{Color, Corners, Point, Ramp, Triangle};

    let triangles = vec![Triangle {
        points: [
            Point::new(20.0, 20.0),
            Point::new(180.0, 20.0),
            Point::new(20.0, 180.0),
        ],
        corners: Corners::Parameters([0.0, 1.0, 0.5]),
    }];
    // A square law, so that a backend interpolating the colours rather than the parameter
    // would differ by a quarter of full scale in the middle of an edge.
    let ramp = Ramp::sample(|t| Color::rgb(t * t, 0.25, 1.0 - t * t));

    let list = shaded_page(pdf_render::ShadingKind::Mesh {
        triangles: triangles.into(),
        ramp: Some(ramp),
    });
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");

    let cpu = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let gpu = gpu().rasterize(&list, target).expect("supported");
    assert_within_tolerance(
        "parametric mesh shading",
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
        }
        .into(),
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
            // Three times the usual differing-channel bound, and nothing else loosened.
            // Six discs 20 units across on a 200-unit page are almost entirely antialiased
            // edge, where the two rasterisers legitimately differ by a level or two: this
            // scene measures 1.36% where the basic scene measures 0.08%. The mean and the
            // worst tile are what catch a mark that is missing or in the wrong place, and
            // deleting either backend's handling of §8.5.3.2 fails this on the *mean*: 17.91
            // against 0.5 for the dotted line, and 1.49 for a single dot.
            assert_within(
                &format!("{what} at scale {scale}"),
                raster_compare::compare(&cpu, &gpu).expect("same size"),
                MAX_DIFFERING_FRACTION * 3.0,
            );
        }
    }
}

/// A stroke with no length draws the same on both backends: ISO 32000-2 §8.5.3.2.
///
/// The second instance of the defect the previous scene guards against, and the reason both
/// live here. `tiny-skia` painted a projecting square cap where the clause asks for no
/// output; `kurbo` dropped a contour that expanded to nothing, so a dot drew nothing at all
/// and `[0 6] 0 d 1 J` — a dotted line — drew nothing at all either. Two rasterisers, two
/// different wrong answers, and a cross-backend comparison that could not see either until a
/// scene stroked a subpath with no length.
///
/// The four shapes are the four sentences: a single-point closed path is a disc under round
/// caps and nothing under the others, a lone `m` is nothing under all three, and a
/// zero-length *dash* is the opposite rule — every cap painted, including the square one.
#[test]
fn cpu_and_gpu_agree_on_a_stroke_with_no_length() {
    use pdf_render::{
        BlendMode, Color, Command, DisplayList, LineCap, Paint, Path, PathCommand, Point, Size,
        Stroke, Transform,
    };
    use std::sync::Arc;

    let scene = |commands: &[PathCommand], stroke: Stroke| {
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        let mut path = Path::new();
        for command in commands {
            path.push(*command);
        }
        list.push(Command::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        list
    };

    let dot = [
        PathCommand::MoveTo(Point::new(100.0, 100.0)),
        PathCommand::Close,
    ];
    let stray = [PathCommand::MoveTo(Point::new(100.0, 100.0))];
    // Diagonal, so that a square cap's orientation is visible: an axis-aligned square and a
    // square turned to face the path cover different pixels, and a horizontal dotted line
    // could not tell them apart.
    let dashed_line = [
        PathCommand::MoveTo(Point::new(20.0, 20.0)),
        PathCommand::LineTo(Point::new(180.0, 180.0)),
    ];

    let mut cases = Vec::new();
    for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
        let wide = Stroke {
            width: 20.0,
            cap,
            ..Stroke::default()
        };
        cases.push((
            format!("a single-point closed path, {cap:?}"),
            &dot[..],
            wide.clone(),
        ));
        cases.push((format!("a lone m, {cap:?}"), &stray[..], wide.clone()));
        cases.push((
            format!("a dotted line, {cap:?}"),
            &dashed_line[..],
            Stroke {
                dash_array: vec![0.0, 20.0],
                ..wide
            },
        ));
    }

    for (what, commands, stroke) in cases {
        let list = scene(commands, stroke);
        // Two scales: a dot's diameter is a width, and a width is resolved against the
        // device, so one scale cannot tell a constant from something that tracks it.
        for scale in [1.0, 2.5] {
            let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
            let cpu = CpuRasterizer::new()
                .rasterize(&list, target)
                .expect("supported");
            let gpu = gpu().rasterize(&list, target).expect("supported");
            // Three times the usual differing-channel bound, and nothing else loosened.
            // Six discs 20 units across on a 200-unit page are almost entirely antialiased
            // edge, where the two rasterisers legitimately differ by a level or two: this
            // scene measures 1.36% where the basic scene measures 0.08%. The mean and the
            // worst tile are what catch a mark that is missing or in the wrong place, and
            // deleting either backend's handling of §8.5.3.2 fails this on the *mean*: 17.91
            // against 0.5 for the dotted line, and 1.49 for a single dot.
            assert_within(
                &format!("{what} at scale {scale}"),
                raster_compare::compare(&cpu, &gpu).expect("same size"),
                MAX_DIFFERING_FRACTION * 3.0,
            );
        }
    }
}

/// §8.5.3.2's dot marks this rasteriser too, at a width no rasteriser can measure.
///
/// The scene above draws its discs 20 units across, where every backend can see them. What
/// none of them could see is the same mark under a device pixel: its area goes as the
/// *square* of the width, so at a tenth of a pixel it is eight thousandths of one, and
/// §10.7.4 forbids the outcome by name.
///
/// > This ensures that no shape ever disappears as a result of unfavourable placement relative
/// > to the device pixel grid, as might happen with other possible scan conversion rules.
///
/// **A cross-backend comparison cannot gate this and that is why the assertion is absolute.**
/// One lost pixel out of forty thousand moves no differing-channel fraction, so the quantity
/// checked here is the raster's own ink against the mark's own area — the same reading
/// `render-quorra/tests/sub_pixel_coverage.rs` takes of the other two rasterisers.
///
/// The mark lands in one device pixel (`pdf_render::point_mark`), so what an eight-bit raster
/// can hold is the mark's area quantised to a level, and the least it can hold at all is one
/// level: below a width of `0.0707` the area is under that and the ink stops tracking it.
#[test]
fn a_sub_pixel_dot_marks_the_devices_raster() {
    use pdf_render::{
        BlendMode, Color, Command, DisplayList, LineCap, Paint, Path, PathCommand, Point, Size,
        Stroke, Transform,
    };
    use std::sync::Arc;

    /// One level of an eight-bit raster, which is what a mark stated in one pixel is held to.
    const ONE_LEVEL: f32 = 1.0 / 255.0;

    for width in [0.2_f32, 0.1, 0.05, 0.01] {
        let mut path = Path::new();
        // A whole number at scale 1 is a device pixel's *corner*, which is the placement the
        // clause names and the one this mark used to be lost at.
        path.push(PathCommand::MoveTo(Point::new(100.0, 100.0)));
        path.push(PathCommand::Close);
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        list.push(Command::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: Stroke {
                width,
                cap: LineCap::Round,
                ..Stroke::default()
            },
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
        let raster = gpu().rasterize(&list, target).expect("supported");
        let ink: f32 = raster
            .data
            .chunks_exact(4)
            .map(|pixel| f32::from(255 - pixel[0]) / 255.0)
            .sum();
        assert!(
            ink > 0.0,
            "§10.7.4: no shape ever disappears, and a {width}-unit dot did on the device"
        );
        let area = core::f32::consts::PI * width * width / 4.0;
        let expected = area.max(ONE_LEVEL);
        assert!(
            (ink - expected).abs() <= ONE_LEVEL,
            "a {width}-unit dot drew {ink:.5} of ink where its own area is {area:.5}, which is \
             more than one level of 255 from the {expected:.5} a raster can state"
        );
    }
}

/// A clip that admits nothing admits nothing on both backends: ISO 32000-2 §8.5.4.
///
/// The empty clipping path is the third shape in this file that each rasteriser answers for
/// itself and answers differently: `tiny-skia` refuses an empty path outright, where `kurbo`
/// clips to an empty region. Both are conventions, and only one of them is §8.5.4's answer —
/// the region a clip admits is "the same area that would be filled by the f operator", which
/// for a path §8.5.3.3.1 has disregarded down to nothing is no area at all.
///
/// The scene pairs the empty clip with an ordinary one covering a quarter of the page, so
/// that a backend which dropped *every* clipped command would pass the first half and fail
/// the second. `render-cpu/tests/empty_clip.rs` pins the absolute answer, ink 0 against 2500;
/// this pins the two backends to the same one.
#[test]
fn cpu_and_gpu_agree_on_a_clip_that_admits_nothing() {
    use pdf_render::{
        BlendMode, Clip, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point,
        Size, Transform,
    };
    use std::sync::Arc;

    let page = 200.0_f32;
    let rectangle = |from: Point, to: Point| {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(from));
        path.push(PathCommand::LineTo(Point::new(to.x, from.y)));
        path.push(PathCommand::LineTo(to));
        path.push(PathCommand::LineTo(Point::new(from.x, to.y)));
        path.push(PathCommand::Close);
        path
    };

    let scene = |clip_path: Path| {
        let mut list = DisplayList::new(Size::new(page, page));
        let clip = list
            .add_clip(Clip {
                path: clip_path,
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("one clip is within the limit");
        list.push(Command::Fill {
            path: Arc::new(rectangle(Point::new(0.0, 0.0), Point::new(page, page))),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: Some(clip),
            mask: None,
            blend: BlendMode::Normal,
        });
        list
    };

    let quarter = rectangle(Point::new(0.0, 0.0), Point::new(page / 2.0, page / 2.0));
    for (what, list) in [
        ("an empty clip", scene(Path::new())),
        ("a clip that admits a quarter of the page", scene(quarter)),
    ] {
        let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
        let cpu = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        let gpu = gpu().rasterize(&list, target).expect("supported");
        assert_within_tolerance(
            what,
            raster_compare::compare(&cpu, &gpu).expect("same size"),
        );
    }
}

/// A fill with no area draws the same on both backends: ISO 32000-2 §10.7.4.
///
/// The third instance of the shape the two scenes above guard, and the reason it is here
/// rather than only in `render-cpu`'s pixel test. "A shape shall be scan-converted by painting
/// any pixel whose half-open square region intersects the shape … This ensures that no shape
/// ever disappears", and a subpath with no extent along one axis has zero area, so *both*
/// rasterisers compute zero coverage for it and both drew nothing — a page of ruling lines
/// written as `848 1085 10159 0 re f` came out blank on either backend, and a cross-backend
/// comparison agreeing about nothing is what trap 2 is for.
///
/// `pdf_render::collapsed` builds the mark, so what this checks is that both backends *ask*:
/// the geometry is shared and the width is `thinnest_line`'s, so a difference here can only be
/// one backend skipping the rule. Two scales, because the mark's width is a reciprocal of the
/// scale and one scale cannot tell a reciprocal from a constant.
#[test]
fn cpu_and_gpu_agree_on_a_fill_with_no_area() {
    use pdf_render::{
        BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Size,
        Transform,
    };
    use std::sync::Arc;

    let page = 200.0_f32;
    let mut list = DisplayList::new(Size::new(page, page));
    // A grid: horizontal rules written as zero-height rectangles and vertical ones as
    // zero-width, so that both axes of the rule are drawn and a backend implementing one of
    // them shows up as a half-drawn page.
    for step in 1..8 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "test code: the loop counter is under ten"
        )]
        let at = step as f32 * 25.0;
        let mut row = Path::new();
        row.push(PathCommand::MoveTo(Point::new(20.0, at)));
        row.push(PathCommand::LineTo(Point::new(180.0, at)));
        row.push(PathCommand::Close);
        let mut column = Path::new();
        column.push(PathCommand::MoveTo(Point::new(at, 20.0)));
        column.push(PathCommand::LineTo(Point::new(at, 180.0)));
        column.push(PathCommand::Close);
        for path in [row, column] {
            list.push(Command::Fill {
                path: Arc::new(path),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(Color::BLACK),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
    }

    for scale in [1.0, 2.5] {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
        let cpu = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        let gpu = gpu().rasterize(&list, target).expect("supported");
        // A page of nothing but one-pixel marks is a page of nothing but antialiased edge,
        // which is what the loosened differing-channel bound is for — and it is the same
        // three-fold loosening `cpu_and_gpu_agree_on_the_thinnest_line_the_device_draws`
        // takes for the same reason. Deleting either backend's call to the rule fails this
        // on the *mean*, which is the threshold that catches a mark that is not there.
        assert_within(
            &format!("a grid of flat fills at scale {scale}"),
            raster_compare::compare(&cpu, &gpu).expect("same size"),
            MAX_DIFFERING_FRACTION * 3.0,
        );
    }
}
