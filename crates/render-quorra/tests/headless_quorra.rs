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
/// Plus — so the backend says so and draws nothing, rather than drawing the page the
/// coverage-modulated form would give.
///
/// **The two operators exist since `89d7dd77` (quorra's ADR 0025) and neither can be asked
/// for where this element occurs**, which the four-hundred-and-thirty-ninth session found by
/// writing the translation rather than by reading about it —
/// `quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over` below is the position
/// itself, stated in the scene vocabulary and refused. `doc/QUORRA_FEEDBACK.md` section 14.2
/// is the ask that follows.
///
/// This is a *test of the refusal*, not of a defect: it fails if the refusal ever becomes
/// silent.
#[test]
fn quorra_refuses_a_knockout_element_that_states_its_shape() {
    let list = test_scenes::knockout_stated_shape();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits the budget");
    let refusal = quorra()
        .rasterize(&list, target)
        .expect_err("the two marks §11.4.6's second stage needs cannot be asked for")
        .to_string();
    assert!(
        refusal.contains("§11.4.6") && refusal.contains("shape"),
        "the refusal names the clause and what it needs: {refusal}"
    );
}

/// The scene vocabulary refuses §11.4.6's pair in the one position this tree would emit it
/// from — which is what the refusal above is *about*, held here rather than described.
///
/// `quorra_scene::Compose::DestOut` and `Plus` arrived at `89d7dd77` with two positions
/// refused, `BlendNotNormal` and `InsideKnockoutGroup`, recorded on both sides of the
/// boundary as positions this tree does not emit. The second is the only position it emits
/// them from: `pdf_render::Command::Shaped` "appears only as a direct element of a
/// [`Command::Group`] whose `knockout` is set", because outside one §11.4.4's formulas reach
/// the shape only through shape × opacity and the object may be drawn alone. So the mark that
/// would carry `DestOut` is by construction inside a knockout group.
///
/// This asserts the refusal directly against the builder, with no display list in the way, so
/// that it is a statement about the vocabulary rather than about this backend. **It fails
/// when quorra lifts the restriction** — which is the notification the next round wants, and
/// the reason it is a test rather than a paragraph.
///
/// The second half of the obstacle cannot be a test at all and is recorded here instead:
/// `SceneBuilder::group`, `stroke` and `image` take no `Compose`, and three of the four corpus
/// pages behind this refusal state a `Shaped` whose two halves are groups. That one is a
/// missing parameter, so a round that tried it would not compile.
#[test]
fn quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over() {
    let mut builder = quorra_scene::SceneBuilder::new();
    let spec = quorra_scene::GroupSpec {
        alpha: 1.0,
        blend: quorra_scene::BlendMode::Normal,
        clip: None,
        knockout: true,
        mask: None,
        isolated: true,
    };
    let mut staged = Ok(());
    builder
        .group(spec, |body| {
            // The shape half of a `Command::Shaped`: the object with every source of opacity
            // removed, so the alpha it is drawn with is §11.6.4.2's shape. The outline id is
            // never resolved — the builder refuses before a device would see it.
            staged = body.fill(
                quorra_scene::OutlineId(0),
                quorra_scene::Affine::IDENTITY,
                quorra_scene::FillRule::NonZero,
                quorra_scene::Paint::Solid(quorra_scene::Color::new(1.0, 1.0, 1.0, 1.0)),
                None,
                quorra_scene::BlendMode::Normal,
                quorra_scene::Compose::DestOut,
                None,
            );
            Ok(())
        })
        .expect("the knockout group itself is accepted");

    let refusal = staged.expect_err(
        "Destination-Out inside a knockout group is what §11.4.6's first stage needs here",
    );
    assert!(
        matches!(
            refusal,
            quorra_scene::SceneError::StagedComposeUnsupported {
                compose: quorra_scene::Compose::DestOut,
                ..
            }
        ),
        "the vocabulary names what it will not take and where: {refusal:?}"
    );
}

/// §11.4.4's non-isolated group, drawn — the two backends agree on the *other* initial
/// backdrop.
///
/// This test read `quorra_refuses_a_non_isolated_group` until the
/// four-hundred-and-thirty-eighth session. `GroupSpec` gained Table 145's `/I` (quorra's
/// ADR 0019, `doc/QUORRA_NON_ISOLATED_GROUPS.md`), so the group's buffer can begin as a copy
/// of what is under it and the composite back is the interpolation ADR 0237 derived —
/// `result = (1 − w) × B + w × E(B)`. Two independent transcriptions of the clause, one per
/// backend, and this scene is where they meet: the element blends `Multiply` against a green
/// page, which is the only thing §11.4.4's NOTE 2 says the two kinds of group differ about.
///
/// The refusal it replaces has not gone away — `quorra_refuses_a_non_isolated_group_that_blends`
/// below holds the set that is still refused.
///
/// # The pixel, not the tolerance
///
/// A tolerance says the two backends agree; it does not say they agree on the *clause*, and
/// two backends substituting §11.4.5's transparent backdrop agreed for four hundred sessions.
/// So the colour inside the group is asserted against the arithmetic. Opaque green page,
/// opaque blue element under `Multiply`, group alpha ½:
///
/// - non-isolated, which is this scene — the element composites onto the backdrop, so
///   §11.3.5.2's Table 134 gives Multiply as `B(cb, cs) = cb × cs`, here
///   `(0,1,0) × (0,0,1) = (0,0,0)`, and `result = (1 − ½) × green + ½ × black` is
///   **`(0, 128, 0)`**;
/// - isolated, the picture this used to be refused rather than draw — the element blends
///   against §11.4.5's transparent initial backdrop, and §11.3.6 says what that does to a
///   blend mode: "[a]n alpha value of αs = 0.0 or αb = 0.0 results in no blend mode effect".
///   The blue survives whole and the composite is `(0, 128, 128)`.
///
/// A byte on one channel apart, and it is the whole of §11.4.4.
#[test]
fn cpu_and_quorra_agree_on_a_non_isolated_group() {
    let list = test_scenes::non_isolated_group();
    assert_within_tolerance("non-isolated group", compare("non-isolated group", &list));

    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits the budget");
    let ours = quorra()
        .rasterize(&list, target)
        .expect("quorra draws a non-isolated group whose composite is Normal");
    // (300, 400) is inside the group's rectangle, well away from every edge.
    let at = ((400 * ours.width + 300) * 4) as usize;
    assert_eq!(
        &ours.data[at..at + 4],
        &[0, 128, 0, 255],
        "the element blended against the page, not against transparency"
    );
}

/// A non-isolated group whose *own* blend is not Normal is still refused **by name**.
///
/// The cancellation ADR 0237 derives is the composite back under §11.3.3's **Normal** blend
/// function: the group alpha §11.4.4's Result step divides out is multiplied straight back in,
/// and nothing else in the pipeline observes it. Under any other blend the group's own colour
/// is needed, and with it Table 140's group alpha — which a premultiplied raster does not hold
/// (NOTE 4: "For shape and alpha, backdrop removal can be accomplished by maintaining two sets
/// of variables to hold the accumulated values."). quorra measures the identity 0.91 of full
/// scale wrong there and refuses at `SceneBuilder::group`.
///
/// `pdf-model` never emits this list — `Command::Group`'s `isolated` states the three
/// conditions as a guarantee, and `render-cpu` refuses it too — so the display list is built
/// here by hand. A *test of the refusal*, as
/// `quorra_refuses_a_knockout_element_that_states_its_shape` is: it fails if the refusal ever
/// becomes a silently wrong picture.
#[test]
fn quorra_refuses_a_non_isolated_group_that_blends() {
    let list = a_non_isolated_group_composited_with(pdf_render::BlendMode::Multiply);
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits the budget");
    let refusal = quorra()
        .rasterize(&list, target)
        .expect_err("the cancellation is the Normal composite, and this is not one")
        .to_string();
    assert!(
        refusal.contains("non-isolated") && refusal.contains("Normal"),
        "the refusal names what it cannot do and why: {refusal}"
    );
}

/// One non-isolated group over a page, composited under `blend` — the geometry of
/// [`test_scenes::non_isolated_group`] with the group's own blend mode made a parameter.
fn a_non_isolated_group_composited_with(blend: pdf_render::BlendMode) -> pdf_render::DisplayList {
    use std::sync::Arc;

    use pdf_render::{BlendMode, Command, DisplayList, FillRule, Paint, Transform};
    use test_scenes::{A4, BLUE, GREEN, rect};

    let mut list = DisplayList::new(A4);
    list.push(Command::Fill {
        path: Arc::new(rect(40.0, 100.0, 555.0, 750.0)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(GREEN),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list.push(Command::Group {
        commands: vec![Command::Fill {
            path: Arc::new(rect(100.0, 200.0, 450.0, 600.0)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(BLUE),
            clip: None,
            mask: None,
            blend: BlendMode::Multiply,
        }],
        alpha: 0.5,
        clip: None,
        mask: None,
        blend,
        isolated: false,
        knockout: false,
    });
    list
}

/// §11.4.7's page in a four-component blending colour space, drawn — two rasters of one page
/// against one device.
///
/// > All page-level compositing shall be done in the default blending colour space of the
/// > page, and the entire result shall then, if the colour spaces are not equivalent, be
/// > converted to the native colour space of the output device before being composited with
/// > the context-dependent backdrop.
///
/// §11.3.4 applies the compositing formula per component, so four components are three plus
/// one and the page is drawn twice with a different three loaded. This test read
/// `quorra_refuses_a_page_in_a_four_component_blending_space` until the
/// four-hundred-and-thirty-ninth session: `doc/QUORRA_FEEDBACK.md` section 17 asked whether
/// two `Target::Readback` renders against one device were possible, the answer at `89d7dd77`
/// was that they always had been, and this backend now makes them.
///
/// # The pixels, not the tolerance
///
/// A tolerance says the two backends agree; it does not say they agree with the clause, and
/// two backends could agree by both drawing the chromatic half alone. So two pixels are
/// asserted, and [`test_scenes::four_component_page`] says what each is for:
///
/// - **half of registration black over paper** is ½ of each of the four inks per component,
///   which the cube interpolates to (76, 66, 64) — against the (128, 128, 128) that averaging
///   the two *converted* colours gives, 51 of 255 away;
/// - **the black component alone** is white in the chromatic raster, so (35, 31, 32) is only
///   reachable if the second render happened.
///
/// `render-cpu`'s `four_component_page.rs` derives both from the clause; this asserts the
/// same two on the graphics device.
#[test]
fn cpu_and_quorra_agree_on_a_four_component_page() {
    let list = test_scenes::four_component_page();
    assert_within_tolerance("four-component page", compare("four-component page", &list));

    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits the budget");
    let ours = quorra()
        .rasterize(&list, target)
        .expect("quorra draws a page whose blending space has four components");
    let at = |x: u32, y: u32| {
        let index = ((y * ours.width + x) * 4) as usize;
        let p = &ours.data[index..index + 4];
        [p[0], p[1], p[2], p[3]]
    };
    let close = |got: [u8; 4], want: [u8; 4]| {
        got.iter()
            .zip(want)
            .all(|(&g, w)| i32::from(g).abs_diff(i32::from(w)) <= 1)
    };
    let composited = at(190, 242);
    assert!(
        close(composited, [76, 66, 64, 255]),
        "half registration black over paper composites in ink: {composited:?}"
    );
    let black = at(430, 242);
    assert!(
        close(black, [35, 31, 32, 255]),
        "the fourth component was drawn: {black:?}"
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
