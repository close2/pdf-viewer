//! A page drawn more than 8191 pixels wide or tall is drawn as the window of it is — ISO 32000-2
//! §10.7.4, ADR 2058.
//!
//! # What the expected values come from
//!
//! §10.7.4 fixes a pixel as the half-open square `[i, i+1) × [j, j+1)` on integer device
//! coordinates, and this backend paints a partly covered pixel at the area the shape covers there
//! (the ledger row's departure (1)). That area is a function of the shape's position against the
//! grid and of nothing else, so the same shape moved by a whole number of pixels covers the same
//! areas of the moved pixels — and subtracting a whole number from these coordinates is exact in
//! `f32`, so the moved shape is the same shape. A window of a page is that move: the page's marks
//! under its transform with the window's corner taken off. The page drawn whole and its window
//! therefore owe the same pixels, and no renderer enters the expected value.
//!
//! # What broke it, and why both halves are here
//!
//! `tiny-skia`'s fixed-point arithmetic stops at 8191, and two of its consequences reached a page
//! past that size while sparing the window of it, which sits nearer the origin: a mark whose device
//! extent passed 8191 was drawn without anti-aliasing at all, and every rectangle on a surface
//! past 8191 lost its closed form to the library's supersampled converter. On the owner's witness
//! at 9.3 times the fit the second put one glyph's bar edge at 0 on the page and 58 in the window.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is over rasters this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// The width of the page every scene is drawn on: past 8191, as the witness's is.
const WIDE: u32 = 11_176;

/// The height of every surface here.
const TALL: u32 = 64;

/// How far the window's corner lies from the page's, in whole pixels.
const CORNER: f32 = 8_000.0;

/// A triangle with oblique edges and its corner at `(x, y)`, every vertex stated as `x` or `y`
/// plus a constant, so that moving `x` by a whole number moves every vertex by it exactly.
fn triangle(x: f32, y: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x, y)));
    path.push(PathCommand::LineTo(Point::new(x + 5.3, y + 0.7)));
    path.push(PathCommand::LineTo(Point::new(x + 1.9, y + 4.6)));
    path.push(PathCommand::Close);
    path
}

/// A closed axis-aligned rectangle.
fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// One black mark on a white surface `width` pixels wide, at one device pixel per unit, drawn in
/// one strip.
fn drawn(mark: Command, width: u32) -> Raster {
    let mut list = DisplayList::new(Size::new(width as f32, TALL as f32));
    list.push(mark);
    let target = TargetSpec {
        width,
        height: TALL,
        transform: Transform::IDENTITY,
    };
    CpuRasterizer::new()
        .with_strips(1)
        .rasterize(&list, target)
        .expect("a solid mark draws")
}

fn filled(path: Path) -> Command {
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

fn stroked(path: Path) -> Command {
    Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 1.7,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// The red channel of pixel `(x, y)`, which on black over white is the complement of coverage.
fn level(raster: &Raster, x: u32, y: u32) -> u8 {
    raster.data[((y * raster.width + x) * 4) as usize]
}

/// The pixels of `page` from column `from` on that differ from `window`'s, which starts there.
fn differing(page: &Raster, window: &Raster, from: u32) -> Vec<(u32, u32, u8, u8)> {
    let mut found = Vec::new();
    for y in 0..TALL {
        for x in 0..window.width {
            let (a, b) = (level(page, from + x, y), level(window, x, y));
            if a != b {
                found.push((x, y, a, b));
            }
        }
    }
    found
}

/// A mark past 8191 device pixels is measured where it lies, as the same mark moved near the
/// origin is: before ADR 2058 the page drew it without anti-aliasing, every boundary pixel 0 or
/// 255.
#[test]
fn a_mark_past_the_range_is_the_same_mark_moved_into_it() {
    let (x, y) = (9_031.3_f32, 20.27_f32);
    for (what, page, window) in [
        (
            "a fill",
            filled(triangle(x, y)),
            filled(triangle(x - CORNER, y)),
        ),
        (
            "a stroke",
            stroked(triangle(x, y)),
            stroked(triangle(x - CORNER, y)),
        ),
    ] {
        let page = drawn(page, WIDE);
        let window = drawn(window, WIDE - CORNER as u32);
        let found = differing(&page, &window, CORNER as u32);
        assert!(
            found.is_empty(),
            "{what} past the range differs from itself moved into it at {found:?}"
        );
    }
}

/// A rectangle on a surface past 8191 pixels is its closed form, §10.7.4's product of two
/// overlaps, to the rounding of one level — where the library's rectangle call would have handed
/// it to the supersampled converter and rounded each edge to a quarter of a pixel.
#[test]
fn a_rectangle_on_a_surface_past_the_tile_is_its_closed_form() {
    let corners = (6_231.3_f32, 20.227_f32, 6_236.8_f32, 23.4_f32);
    let page = drawn(
        filled(rectangle(corners.0, corners.1, corners.2, corners.3)),
        WIDE,
    );
    let area = pdf_render::Rect::from_corners(
        Point::new(corners.0, corners.1),
        Point::new(corners.2, corners.3),
    );
    for y in 0..TALL {
        for x in 6_225..6_245_u32 {
            let wanted = pdf_render::rectangle_coverage(area, x as f32, y as f32);
            let painted = f32::from(255 - level(&page, x, y)) / 255.0;
            assert!(
                (painted - wanted).abs() <= 1.0 / 255.0,
                "pixel ({x}, {y}) is painted at {painted}, its area is {wanted}"
            );
        }
    }
}

/// And the window of it, drawn on a surface the library's rectangle converter reaches, is the
/// same closed form in that converter's 8.8 fixed point: the two agree to a level of 255, which is
/// what remains between a page and its window here (ADR 2058).
#[test]
fn a_rectangle_and_its_window_agree_to_a_level() {
    let (x0, y0, x1, y1) = (6_231.3_f32, 20.227_f32, 6_236.8_f32, 23.4_f32);
    let page = drawn(filled(rectangle(x0, y0, x1, y1)), WIDE);
    let window = drawn(
        filled(rectangle(x0 - CORNER / 2.0, y0, x1 - CORNER / 2.0, y1)),
        WIDE - CORNER as u32,
    );
    let found = differing(&page, &window, (CORNER / 2.0) as u32);
    assert!(
        found.iter().all(|&(_, _, a, b)| a.abs_diff(b) <= 1),
        "the page and its window differ by more than a level at {found:?}"
    );
}

/// A spike from the page out to `reach` device pixels, which is what a damaged content stream
/// states: ADR 0269's two witnesses reach 10²⁵ on a 368 × 542 page.
fn spike(reach: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(reach, reach * 200.0)));
    path.push(PathCommand::LineTo(Point::new(30.0, 10.0)));
    path.push(PathCommand::Close);
    path
}

/// A mark whose coordinates are past what `f32` states to a measured coverage is drawn as the
/// library's aliased converter draws it, as it was before ADR 2058: `render_cpu::area` measures a
/// mark past 8191 only while its coordinates keep the precision its integral needs. Drawn with
/// anti-aliasing asked for and with it withdrawn, the two are the same page. Before the bound,
/// `4605705.pdf` lost a third of its ink to the integral of a spike at 10²⁵.
#[test]
fn a_mark_past_what_f32_measures_is_drawn_as_the_library_draws_it() {
    for reach in [1.0e7_f32, 1.0e25] {
        let mut list = DisplayList::new(Size::new(368.0, TALL as f32));
        list.push(filled(spike(reach)));
        let target = TargetSpec {
            width: 368,
            height: TALL,
            transform: Transform::IDENTITY,
        };
        let asked = CpuRasterizer::new()
            .with_strips(1)
            .rasterize(&list, target)
            .expect("the spike draws");
        let aliased = CpuRasterizer::new()
            .with_strips(1)
            .with_anti_alias(false)
            .rasterize(&list, target)
            .expect("the spike draws aliased");
        let differing = asked
            .data
            .chunks_exact(4)
            .zip(aliased.data.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            differing, 0,
            "the spike to {reach} differs from the library's aliased drawing at {differing} pixels"
        );
    }
}

/// A page of `width` × `height` pixels: a frame stroked round its edge, as the owner's witness
/// states one, and a grid of small marks over the whole page — triangles with oblique edges,
/// filled and stroked by turns, eighteen rows tall every sixteen so that every row is crossed by
/// one, and a rectangle with fractional edges beside each. Small, so that a strip replays a
/// fraction of the list rather than all of it, as a page of drawing does. Every coordinate is a
/// dyadic fraction, so a strip's whole-row shift moves each exactly.
fn page_of_marks(width: u32, height: u32) -> DisplayList {
    let (w, h) = (width as f32, height as f32);
    let mut list = DisplayList::new(Size::new(w, h));
    list.push(stroked(rectangle(4.5, 4.5, w - 4.5, h - 4.5)));
    let mut y = 8.25_f32;
    let mut turn = 0_u32;
    while y + 20.0 < h {
        let mut x = 10.375_f32;
        while x + 30.0 < w {
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(x, y)));
            path.push(PathCommand::LineTo(Point::new(x + 13.75, y + 6.5)));
            path.push(PathCommand::LineTo(Point::new(x + 2.125, y + 18.0)));
            path.push(PathCommand::Close);
            list.push(if turn.is_multiple_of(2) {
                filled(path)
            } else {
                stroked(path)
            });
            list.push(filled(rectangle(
                x + 17.25,
                y + 3.375,
                x + 24.625,
                y + 9.875,
            )));
            turn += 1;
            x += 40.0;
        }
        y += 16.0;
    }
    list
}

/// `list` drawn on `target` in the strips `strips` asks for, and the plan that drew it.
fn drawn_in(list: &DisplayList, target: TargetSpec, strips: u32) -> (Vec<u32>, Raster) {
    let mut backend = CpuRasterizer::new().with_strips(strips);
    let plan = backend.strip_plan(list, target);
    let raster = backend.rasterize(list, target).expect("the page draws");
    (plan, raster)
}

/// A pixel two drawings differ at: its column, its row, and the red level of each.
type Difference = (u32, u32, u8, u8);

/// Every pixel at which `one` and `cut` differ.
fn differences(one: &Raster, cut: &Raster) -> Vec<Difference> {
    one.data
        .chunks_exact(4)
        .zip(cut.data.chunks_exact(4))
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, (a, b))| {
            let index = index as u32;
            (index % one.width, index / one.width, a[0], b[0])
        })
        .collect()
}

/// [`page_of_marks`] drawn in one strip and then in each count of `asked`, every count's plan
/// asserted to cut the page: the plans and the differences, count by count.
fn cut_against_one(width: u32, height: u32, asked: &[u32]) -> Vec<(Vec<u32>, Vec<Difference>)> {
    let list = page_of_marks(width, height);
    let target = TargetSpec {
        width,
        height,
        transform: Transform::IDENTITY,
    };
    let (one_plan, one) = drawn_in(&list, target, 1);
    assert_eq!(one_plan, vec![0, height], "one strip asked, one made");
    asked
        .iter()
        .map(|&strips| {
            let (plan, cut) = drawn_in(&list, target, strips);
            assert!(
                plan.len() > 2,
                "{width} x {height}: {strips} strips asked and the page is not cut, cuts {plan:?}"
            );
            (plan, differences(&one, &cut))
        })
        .collect()
}

/// A page wider than 8191 pixels is cut into strips as a page inside the range is, and the strips
/// are the page drawn in one to the byte (ADR 2068). Its marks past the library's range are
/// measured by `render_cpu::area` on either side of 8191, which evaluates every row from the mark's
/// own device coordinates, so a cut across one draws the bytes the whole page does — ADR 1837's
/// property; and a strip is a run of whole rows, so a column past the range is past it in every
/// strip and every rectangle there is the closed form in every division.
#[test]
fn a_wide_page_is_cut_and_its_strips_are_the_page() {
    for (plan, found) in cut_against_one(WIDE, 512, &[2, 4, 16]) {
        assert!(
            found.is_empty(),
            "the wide page in strips at {plan:?} differs from one strip at {} pixels: {:?}",
            found.len(),
            found.get(..found.len().min(12))
        );
    }
}

/// Asserts that a division drew the page drawn in one strip to the byte.
///
/// A page inside the library's rectangle range owes that as the wide page does: a rectangle is
/// drawn by `tiny-skia`'s rectangle converter wherever that converter reaches the surface (ADR
/// 0476), and the planner forbids the rows beside its horizontal edges, where a cut would leave the
/// converter one row of it to state a level from the unclipped rectangle's (ADR 2084).
fn assert_the_page(what: &str, plan: &[u32], found: &[Difference]) {
    assert!(
        found.is_empty(),
        "{what} in strips at {plan:?} differs from one strip at {} pixels: {:?}",
        found.len(),
        found.get(..found.len().min(12))
    );
}

/// A page taller than 8191 pixels is cut too, its marks past 8191 rows as exactly as the wide
/// page's past 8191 columns, and its strips are the page drawn in one (ADRs 2068, 2084).
#[test]
fn a_tall_page_is_cut_and_its_strips_are_the_page() {
    for (plan, found) in cut_against_one(640, 8_704, &[2, 4, 16]) {
        assert_the_page("the tall page", &plan, &found);
    }
}

/// A spike from the page to `(x, y)` device pixels, stated with dyadic coordinates.
fn spike_to(x: f32, y: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.25, 10.5)));
    path.push(PathCommand::LineTo(Point::new(x, y)));
    path.push(PathCommand::LineTo(Point::new(30.75, 10.5)));
    path.push(PathCommand::Close);
    path
}

/// What the planner's old bound kept out, asked first (trap 214): a mark whose extent passes 8191
/// but not `render_cpu::area`'s own bound, which ADR 2068 frees and which must then draw the page
/// in strips to the byte, and ADR 0269's hostile geometry past that bound, whose rows stay
/// forbidden — `tiny-skia` draws such a mark aliased and clips it at a strip's edge (ADR 2058 §4).
/// On ADR 0269's 368 × 542 page, a fill and a stroke of each; a freed spike leaves its strips the
/// page drawn in one.
#[test]
fn what_the_old_bound_kept_out_is_cut_inside_the_measured_range_and_kept_past_it() {
    let (width, height) = (368, 542);
    let target = TargetSpec {
        width,
        height,
        transform: Transform::IDENTITY,
    };
    for (reach, freed) in [
        ((20_000.5_f32, 9_000.25_f32), true),
        ((1.0e7, 2.0e9), false),
        ((1.0e25, 2.0e27), false),
    ] {
        for (what, mark) in [
            ("a fill", filled(spike_to(reach.0, reach.1))),
            ("a stroke", stroked(spike_to(reach.0, reach.1))),
        ] {
            let mut list = page_of_marks(width, height);
            list.push(mark);
            let (_, one) = drawn_in(&list, target, 1);
            let (plan, cut) = drawn_in(&list, target, 4);
            if freed {
                assert!(
                    plan.len() > 2,
                    "{what} to {reach:?} is inside the measured range and still forbids every \
                     cut: {plan:?}"
                );
                assert_the_page(what, &plan, &differences(&one, &cut));
            } else {
                assert_eq!(
                    plan,
                    vec![0, height],
                    "{what} to {reach:?} is past the measured range, so its rows stay forbidden"
                );
            }
        }
    }
}
