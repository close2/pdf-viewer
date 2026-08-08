//! The quorra backend, verified headlessly against the CPU oracle.
//!
//! The same harness discipline as `render-gpu/tests/headless_gpu.rs`: both backends
//! consume the *same* `test_scenes` display list, so a difference cannot be blamed
//! on differing input — it is a backend defect. The thresholds are the measured
//! gates that suite established (an order of magnitude above observed noise, tight
//! enough that a missing shape cannot pass), reused unchanged so the two GPU
//! backends are held to one bar.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: an explanatory panic is the intended failure mode, and the \
              expects live in helpers the allow-expect-in-tests config cannot see"
)]

use pdf_render::{Rasterizer, TargetSpec};
use raster_compare::Comparison;
use render_cpu::CpuRasterizer;
use render_quorra::QuorraRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Broad-difference gate. Catches gamma, colour-space and inversion errors.
const MAX_MEAN_ERROR: f64 = 0.5;
/// Localised-difference gate. Catches missing or misplaced geometry.
const MAX_WORST_TILE_ERROR: f64 = 5.0;
/// At most this fraction of channels may differ noticeably.
const MAX_DIFFERING_FRACTION: f64 = 0.01;

/// Builds the quorra backend, or explains why the whole suite should fail.
/// Deliberately does not skip (ADR 0004): CI installs a software Vulkan driver.
fn quorra() -> QuorraRasterizer {
    match QuorraRasterizer::new_headless() {
        Ok(rasterizer) => rasterizer,
        Err(e) => panic!(
            "no adapter available for quorra: {e}\n\
             Install a Vulkan driver (vulkan-radeon, or mesa-vulkan-drivers for a \
             software one). These tests do not skip, because a skipped suite \
             reports success while verifying nothing."
        ),
    }
}

fn compare(what: &str, list: &pdf_render::DisplayList) -> Comparison {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("target fits the budget");
    let cpu = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the CPU oracle draws every fixture");
    let ours = quorra()
        .rasterize(list, target)
        .unwrap_or_else(|e| panic!("{what}: quorra refused: {e}"));
    raster_compare::compare(&cpu, &ours).expect("same dimensions")
}

fn assert_within_tolerance(what: &str, c: Comparison) {
    assert_within(what, c, MAX_DIFFERING_FRACTION);
}

/// The same gate as the render-gpu suite, with only the boundary-heavy scenes
/// allowed a wider differing-channel fraction (that bound counts channels that
/// differ at all, so a scene that is mostly edge trips it on antialiasing alone).
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
    let raster = quorra()
        .rasterize(&list, target)
        .expect("basic scene is supported");
    assert_eq!((raster.width, raster.height), (595, 842));
    assert_eq!(raster.data.len(), 595 * 842 * 4);
}

#[test]
fn cpu_and_quorra_agree_on_the_basic_scene() {
    assert_within_tolerance("basic", compare("basic", &test_scenes::basic()));
}

#[test]
fn cpu_and_quorra_agree_on_transparency_groups() {
    assert_within_tolerance(
        "transparency group",
        compare("transparency group", &test_scenes::transparency_group()),
    );
}

#[test]
fn cpu_and_quorra_agree_on_knockout_groups() {
    assert_within_tolerance(
        "knockout group",
        compare("knockout group", &test_scenes::knockout_group()),
    );
}

/// §11.4.6's element whose shape is stated apart from its alpha is refused **by name**.
///
/// `quorra_scene::Compose` offers source-over and coverage-modulated source, and the second
/// is precisely the assumption this element exists to contradict: it reads the shape off the
/// coverage. Writing `(1 − shape) × backdrop + object` needs Porter-Duff Destination-Out and
/// Plus, which the scene vocabulary does not have — so the backend says so and draws nothing,
/// rather than drawing the page the coverage-modulated form would give.
///
/// This is a *test of the refusal*, not of a defect: it fails if the refusal ever becomes
/// silent, and it is what `doc/QUORRA_FEEDBACK.md`'s entry is measured against.
#[test]
fn quorra_refuses_a_knockout_element_that_states_its_shape() {
    let list = test_scenes::knockout_stated_shape();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits the budget");
    let refusal = quorra()
        .rasterize(&list, target)
        .expect_err("quorra has no Destination-Out and no Plus")
        .to_string();
    assert!(
        refusal.contains("§11.4.6") && refusal.contains("shape"),
        "the refusal names the clause and what it needs: {refusal}"
    );
}

/// §11.4.4's non-isolated group is refused **by name**.
///
/// `quorra_scene::GroupSpec` opens its layer on a fully transparent surface — §11.4.5's
/// initial backdrop — and a non-isolated group's elements have to composite onto the page
/// behind it instead. Nothing in the scene vocabulary states the second backdrop, so the
/// backend says so rather than drawing the isolated group it is not.
///
/// A *test of the refusal*, as `quorra_refuses_a_knockout_element_that_states_its_shape` is:
/// it fails if the refusal ever becomes silent, and it is what
/// `doc/QUORRA_FEEDBACK.md`'s entry is measured against.
#[test]
fn quorra_refuses_a_non_isolated_group() {
    let list = test_scenes::non_isolated_group();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits the budget");
    let refusal = quorra()
        .rasterize(&list, target)
        .expect_err("quorra has no group buffer seeded from its backdrop")
        .to_string();
    assert!(
        refusal.contains("§11.4.4") && refusal.contains("non-isolated"),
        "the refusal names the clause and what it needs: {refusal}"
    );
}

#[test]
fn cpu_and_quorra_agree_on_all_sixteen_blend_modes() {
    // Release only, for the reason `cpu_and_gpu_agree_on_every_blend_mode` records:
    // `tiny-skia`'s u16 SIMD lanes wrap on purpose, so debug overflow checks fire
    // *inside the oracle* on modes that are perfectly correct in release.
    if cfg!(debug_assertions) {
        return;
    }
    assert_within_tolerance(
        "blend modes",
        compare("blend modes", &test_scenes::blend_modes()),
    );
}

#[test]
fn cpu_and_quorra_agree_on_soft_masks() {
    assert_within_tolerance("soft mask", compare("soft mask", &test_scenes::soft_mask()));
}

#[test]
fn cpu_and_quorra_agree_on_diagonal_strokes() {
    // A diagonal stroke is almost entirely boundary, so the channel count runs on
    // antialiasing differences; the two structural gates stay at the shared bar.
    assert_within(
        "diagonal stroke",
        compare("diagonal stroke", &test_scenes::diagonal_stroke()),
        0.06,
    );
}

/// §8.7.4.5.4's cone, the one radial where the clause and a gradient disagree.
///
/// `test_scenes::radial_cone` has the geometry. quorra has its own radial primitive, and it is
/// a two-point conical like everybody else's, so this scene is the one that would catch the
/// third backend keeping it: all three upload `pdf_render::RadialRaster`'s bytes instead —
/// quorra through its mesh paint, which is exactly "an RGBA raster already at device
/// resolution, placed at (left, top)".
#[test]
fn cpu_and_quorra_agree_on_a_radial_cone() {
    assert_within_tolerance(
        "radial cone",
        compare("radial cone", &test_scenes::radial_cone()),
    );
}

#[test]
fn cpu_and_quorra_agree_on_curves() {
    // As with strokes: curves are boundary-dominated fixtures.
    assert_within("curves", compare("curves", &test_scenes::curves()), 0.06);
}

#[test]
fn cpu_and_quorra_agree_on_unaligned_full_bleed() {
    assert_within_tolerance(
        "unaligned full bleed",
        compare("unaligned full bleed", &test_scenes::unaligned_full_bleed()),
    );
}

/// The determinism half of the contract (`RENDER_LIBRARY.md` section 4.6): the same
/// list at the same target renders byte-identically on the same adapter.
#[test]
fn quorra_frames_are_deterministic() {
    let list = test_scenes::blend_modes();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let mut backend = quorra();
    let first = backend.rasterize(&list, target).expect("draws");
    let second = backend.rasterize(&list, target).expect("draws");
    assert_eq!(first, second, "same adapter, same list, same bytes");
}

/// A page interpreted from an actual PDF, through `pdf-model`, so the adapter sees
/// display lists as the interpreter really builds them — not only hand-made ones.
#[test]
fn cpu_and_quorra_agree_on_an_interpreted_pdf() {
    let bytes = test_scenes::basic_pdf();
    let document = pdf_syntax::Document::open(bytes).expect("well-formed fixture");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("has a page");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    assert_within_tolerance("interpreted pdf", compare("interpreted pdf", &list));
}
