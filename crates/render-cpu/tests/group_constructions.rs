//! The two group constructions the four-hundred-and-ninety-second session taught the
//! oracle, each against the clause's own arithmetic (ADR 0327).
//!
//! `CLAUDE.md` keeps `render-cpu` as the correctness oracle, and neither construction is
//! drawn by the other two backends — both refuse each by name, and the refusals are tested
//! against these same scenes in their own suites — so the numbers here are established
//! against ISO 32000-2's formulas rather than against any renderer:
//!
//! - **§11.4.6 on the group's own backdrop.** "In a knockout group, each individual element
//!   shall be composited with the group's initial backdrop rather than with the stack of
//!   preceding elements in the group", and for a non-isolated group that backdrop is the
//!   page — "[a] nonisolated knockout group composites its topmost enclosing element with
//!   the group's backdrop."
//! - **§11.6.6's group blending colour space.** "all blending and compositing computations
//!   shall be done in that space" (§11.7.2), with the pair of element lists resolved
//!   through the space's grid before the group is painted.
//!
//! Each fixture keeps a **fractional-coverage** pixel beside the full-coverage ones,
//! because that is where a wrong construction hides (HANDOVER trap 2), and both tests were
//! confirmed to fail with the construction replaced: the knockout scene draws its first
//! element blue instead of black on the transparent-backdrop substitute, and the blending
//! scene draws 127 instead of 76 with the pair collapsed to one RGB list.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: explanatory panics are the intended failure mode, and the \
              arithmetic is on a 100-unit page and 0..=255 levels, where nothing overflows \
              and the truncating casts are the rounding"
)]

use pdf_render::{Command, DisplayList, Raster, Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above the 100×100 page.
const GENEROUS: u64 = 1 << 24;

fn render(list: &DisplayList) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("a 100x100 target");
    CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the oracle draws both constructions")
}

fn pixel(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let bytes = &raster.data[index..index + 4];
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

fn assert_close(what: &str, got: [u8; 4], want: [u8; 4], tolerance: i32) {
    assert!(
        got.iter()
            .zip(want)
            .all(|(&g, w)| (i32::from(g) - i32::from(w)).abs() <= tolerance),
        "{what}: drew {got:?}, and the clause's arithmetic gives {want:?}"
    );
}

/// §11.4.6's two stages per element, against the group's own backdrop.
///
/// # The arithmetic, from the clause
///
/// [`test_scenes::knockout_group_on_its_own_backdrop`] carries the geometry. The page is
/// opaque red, `B = (1, 0, 0)`. Element 1 is an opaque **blue** fill under `Multiply`;
/// §11.3.5.2's Multiply is the componentwise product, so its stage-a) composite is
/// `E₁ = B × blue = (0, 0, 0)` — black, which only exists because the element blends with
/// the *page* and not with §11.4.5's transparency. Element 2 is **green at opacity 0.3**
/// under Normal, so `E₂ = 0.7 × B + 0.3 × green = (0.7, 0.3, 0)` — `(178.5, 76.5, 0)` in
/// eight bits. Stage b) weights by each element's shape:
///
/// ```text
/// P₁ = (1 − f₁) × B  + f₁ × E₁      →  black inside element 1, red outside
/// P₂ = (1 − f₂) × P₁ + f₂ × E₂      →  E₂ wherever element 2's shape is 1,
///                                       *including* over element 1's black
/// ```
///
/// The knockout is the second line: within its shape, element 2 replaces what element 1
/// left — even though element 2 is translucent, its stage-a) composite against `B` is what
/// lands, not a blend with the accumulation.
///
/// # The half-covered pixel, where wrong constructions agree with none of this
///
/// Element 2's left edge sits at x = 30.5, so device column 30 has shape `f₂ = ½` over
/// element 1's black: `P' = ½ × (0,0,0) + ½ × (178.5, 76.5, 0) = (89, 38, 0)`. Ordinary
/// source-over of the object onto the accumulation gives `(0, 38, 0)` there — no red at
/// all, 89 of 255 away — and the transparent-backdrop staged pair (`DestOut` then `Plus`,
/// right for an *isolated* knockout group) has no `B` term anywhere, so element 1 would
/// come out blue rather than black. Both failures were confirmed by substituting each
/// construction in turn.
#[test]
fn a_knockout_groups_element_composites_with_the_groups_own_backdrop() {
    let raster = render(&test_scenes::knockout_group_on_its_own_backdrop());

    // Device y = 100 − page y. Element 1 covers device rows 40..90, element 2 rows 20..70.
    assert_close(
        "element 1 alone: Multiply against the page, not against transparency",
        pixel(&raster, 15, 85),
        [0, 0, 0, 255],
        1,
    );
    assert_close(
        "the overlap: element 2's composite with B replaces element 1's",
        pixel(&raster, 45, 55),
        [178, 76, 0, 255],
        1,
    );
    assert_close(
        "element 2 alone: the same stage-a) composite",
        pixel(&raster, 70, 30),
        [178, 76, 0, 255],
        1,
    );
    assert_close(
        "outside both, the page is untouched",
        pixel(&raster, 5, 5),
        [255, 0, 0, 255],
        0,
    );
    assert_close(
        "half a shape keeps half the accumulation and takes half of E₂",
        pixel(&raster, 30, 55),
        [89, 38, 0, 255],
        2,
    );
}

/// §11.6.6's group blending colour space: the pair composites in ink and resolves once.
///
/// # The arithmetic
///
/// [`test_scenes::group_in_its_own_blending_space`] carries the geometry: paper, and
/// registration black at constant alpha ½ over it, inside a group whose space is the
/// sixteen-corner process-ink cube, on a page that states no space at all. Per §11.3.4 the
/// composite happens per component **in the group's space**, so the covered pixels hold
/// half of each of the four inks, and the conversion out — the multilinear cube at
/// `(½, ½, ½, ½)` — is the mean of its sixteen corners: `(76.0, 66.1, 63.9)` of 255.
/// Converting each colour first and compositing on the device gives `127.5` instead, which
/// is ADR 0251's 51-of-255 gap and what this pixel fails with when the pair is drawn as
/// one RGB list.
///
/// # The half-covered pixel
///
/// The inner fill's right edge sits at x = 80.5, so device column 80 has coverage ½ and
/// carries a quarter of each ink; the expectation is the same grid evaluated at
/// `(¼, ¼, ¼, ¼)`, computed here from the scene's own space rather than copied from any
/// output.
#[test]
fn a_group_with_its_own_blending_space_composites_in_it() {
    let list = test_scenes::group_in_its_own_blending_space();
    // The space the scene itself states, read back off the display list so the expected
    // values below are the same grid the backend resolves through.
    let space = list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Group {
                blending: Some(pair),
                ..
            } => match pair.as_ref() {
                pdf_render::GroupBlending::FourComponents { space, .. } => Some(space.clone()),
                pdf_render::GroupBlending::OneComponent { .. }
                | pdf_render::GroupBlending::ThreeComponents { .. } => None,
            },
            _ => None,
        })
        .expect("the scene states a group blending space");
    let raster = render(&list);

    let level = |value: f32| (value * 255.0 + 0.5) as u8;
    let half = space.convert(0.5, 0.5, 0.5, 0.5);
    assert_close(
        "half of registration black over paper is the cube's mean, not 127.5",
        pixel(&raster, 50, 50),
        [level(half[0]), level(half[1]), level(half[2]), 255],
        1,
    );
    assert!(
        (75..=77).contains(&pixel(&raster, 50, 50)[0]),
        "and that mean is ADR 0251's 76.0 of 255: {:?}",
        pixel(&raster, 50, 50)
    );
    assert_close(
        "paper alone converts to white",
        pixel(&raster, 15, 50),
        [255, 255, 255, 255],
        1,
    );
    let quarter = space.convert(0.25, 0.25, 0.25, 0.25);
    assert_close(
        "a half-covered pixel carries a quarter of each ink through the same grid",
        pixel(&raster, 80, 50),
        [level(quarter[0]), level(quarter[1]), level(quarter[2]), 255],
        2,
    );
}

/// §11.6.6's group blending colour space of one component: the component composites and
/// the curve resolves once (ISO 32000-2 §11.3.4, §11.7.2, §8.6.5.2).
///
/// [`test_scenes::group_in_a_one_component_blending_space`] carries the geometry and the
/// curve; the expectation is the scene's own curve evaluated at the composited component,
/// which for half of black over paper is ½ — sRGB's 188 of 255 rather than device grey's
/// 128 — and for the half-covered column ¾.
#[test]
fn a_group_in_a_one_component_blending_space_composites_its_component() {
    let list = test_scenes::group_in_a_one_component_blending_space();
    let curve = list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Group {
                blending: Some(pair),
                ..
            } => match pair.as_ref() {
                pdf_render::GroupBlending::OneComponent { curve } => Some(curve.clone()),
                pdf_render::GroupBlending::FourComponents { .. }
                | pdf_render::GroupBlending::ThreeComponents { .. } => None,
            },
            _ => None,
        })
        .expect("the scene states a one-component group blending space");
    let raster = render(&list);

    let level = |value: f32| (value * 255.0 + 0.5) as u8;
    let half = curve.convert(0.5);
    assert_close(
        "half of black over paper composites to ½ and leaves by the curve",
        pixel(&raster, 50, 50),
        [level(half[0]), level(half[1]), level(half[2]), 255],
        1,
    );
    assert!(
        (187..=189).contains(&pixel(&raster, 50, 50)[0]),
        "and that is sRGB's 188 of 255, not device grey's 128: {:?}",
        pixel(&raster, 50, 50)
    );
    assert_close(
        "paper alone leaves as white",
        pixel(&raster, 15, 50),
        [255, 255, 255, 255],
        1,
    );
    let three_quarters = curve.convert(0.75);
    assert_close(
        "a half-covered pixel holds three quarters and leaves through the same curve",
        pixel(&raster, 80, 50),
        [
            level(three_quarters[0]),
            level(three_quarters[1]),
            level(three_quarters[2]),
            255,
        ],
        2,
    );
}

/// §11.6.6's group blending colour space of three CIE-based components: the components
/// composite and the cube resolves once (ISO 32000-2 §11.3.4, §11.7.2, §8.6.5.3).
///
/// [`test_scenes::group_in_a_three_component_blending_space`] carries the geometry and the
/// cube; the expectation is the scene's own cube evaluated at the composited components,
/// which for half of black over paper is ½ on each — sRGB's 188 of 255 rather than the
/// device's 128 — and for the half-covered column ¾.
#[test]
fn a_group_in_a_three_component_blending_space_composites_its_components() {
    let list = test_scenes::group_in_a_three_component_blending_space();
    let cube = list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Group {
                blending: Some(pair),
                ..
            } => match pair.as_ref() {
                pdf_render::GroupBlending::ThreeComponents { cube } => Some(cube.clone()),
                pdf_render::GroupBlending::FourComponents { .. }
                | pdf_render::GroupBlending::OneComponent { .. } => None,
            },
            _ => None,
        })
        .expect("the scene states a three-component group blending space");
    let raster = render(&list);

    let level = |value: f32| (value * 255.0 + 0.5) as u8;
    let half = cube.convert([0.5; 3]);
    assert_close(
        "half of black over paper composites to ½ on each component and leaves by the cube",
        pixel(&raster, 50, 50),
        [level(half[0]), level(half[1]), level(half[2]), 255],
        1,
    );
    assert!(
        (187..=189).contains(&pixel(&raster, 50, 50)[0]),
        "and that is sRGB's 188 of 255, not the device's 128: {:?}",
        pixel(&raster, 50, 50)
    );
    assert_close(
        "paper alone leaves as white",
        pixel(&raster, 15, 50),
        [255, 255, 255, 255],
        1,
    );
    let quarter = cube.convert([0.75; 3]);
    assert_close(
        "a half-covered pixel holds ¾ and leaves through the same cube",
        pixel(&raster, 80, 50),
        [level(quarter[0]), level(quarter[1]), level(quarter[2]), 255],
        2,
    );
}
