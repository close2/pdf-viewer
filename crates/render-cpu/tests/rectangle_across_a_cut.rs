//! A rectangle a strip cut crosses draws the bytes the page drawn whole does — ISO 32000-2
//! §10.7.4, ADR 2084.
//!
//! # What the expected values come from
//!
//! §10.7.4 fixes a pixel as the half-open square `[i, i+1) × [j, j+1)` on integer device
//! coordinates, and a strip is a run of whole rows of the page: the pixels of a strip are pixels of
//! the page, at the same coordinates once the strip's first row is added back. Every coordinate
//! below is a dyadic fraction, so the strip's whole-row shift moves each exactly (ADR 0219's residue
//! does not arise). So the page drawn in strips owes the page drawn in one strip byte for byte, and
//! no renderer enters the expected value.
//!
//! # What it rests on, and why the first test is about the library
//!
//! A rectangle is drawn by `tiny-skia`'s rectangle converter (ADR 0476), which clips it to the
//! surface and then walks it in 8.8 fixed point. Where a cut leaves one row of it inside a strip,
//! that row goes down the converter's one-row branch and comes out a level from the same row of the
//! rectangle unclipped; `render_cpu::scan` therefore hands the converter those rows in a surface
//! with a row beyond the cut, where the rectangle is not clipped at them (ADR 2084). The first test
//! asserts the library's half of that, on the library alone, as `strip_cut_exactness.rs` asserts
//! ADR 0139's: if a release stops differing there, the construction is no longer needed and may
//! go. The rest hold this backend to the page at stated cuts across every row of a rectangle and
//! of a rule, under a clip, at the page's own edge, and at the cuts its planner makes.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is over rasters this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, ClipId, Color, Command, DisplayList, FillRule, LineCap, Paint, Path,
    PathCommand, Point, Raster, Rasterizer, Rect, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// The side of the small scenes' square surface.
const SIDE: u32 = 64;

/// The rectangle the stated cuts are put across: its top edge in row 19, its bottom in row 30.
const PROBED: (f32, f32, f32, f32) = (5.375, 19.25, 40.5, 30.625);

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

/// One straight segment.
fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path
}

/// A black fill of `path`.
fn filled(path: Path) -> Command {
    fill_under(path, None)
}

/// A black fill of `path` under `clip`.
fn fill_under(path: Path, clip: Option<ClipId>) -> Command {
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip,
        mask: None,
        blend: BlendMode::Normal,
    }
}

fn stroked(path: Path, width: f32, cap: LineCap, dash_array: Vec<f32>) -> Command {
    Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            cap,
            dash_array,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

fn target(width: u32, height: u32) -> TargetSpec {
    TargetSpec {
        width,
        height,
        transform: Transform::IDENTITY,
    }
}

/// The coverage a pixel was painted at: black over white, so the complement of its red level.
fn coverage(raster: &Raster, x: u32, y: u32) -> u8 {
    255 - raster.data[((y * raster.width + x) * 4) as usize]
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

/// The library states the one row of a rectangle a surface's edge leaves inside it a level from
/// the same row of the rectangle unclipped, at its interior and at both corners: the probed
/// rectangle on a surface twenty rows tall, which ends in the rectangle's top-edge row, against the
/// same rectangle on the whole square.
#[test]
fn the_library_states_one_row_left_inside_a_surface_a_level_from_the_row_unclipped() {
    let (x0, y0, x1, y1) = PROBED;
    let rect = tiny_skia::Rect::from_ltrb(x0, y0, x1, y1).expect("a rectangle");
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    let drawn = |height: u32| {
        let mut pixmap = tiny_skia::Pixmap::new(SIDE, height).expect("a surface");
        pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
        pixmap
    };
    let (cut, whole) = (drawn(20), drawn(SIDE));
    let row = |pixmap: &tiny_skia::Pixmap| -> Vec<u8> {
        let stride = SIDE as usize * 4;
        pixmap.data()[19 * stride..20 * stride]
            .chunks_exact(4)
            .map(|pixel| pixel[3])
            .collect()
    };
    let (cut, whole) = (row(&cut), row(&whole));
    let apart: Vec<usize> = (0..SIDE as usize).filter(|&x| cut[x] != whole[x]).collect();
    assert!(
        !apart.is_empty() && apart.iter().all(|&x| whole[x].abs_diff(cut[x]) == 1),
        "the library's clipped row is no longer a level from its unclipped row: {apart:?}; if it \
         now agrees, `render_cpu::scan`'s surface_edge construction is no longer needed"
    );
}

/// The probed rectangle, black, under a clip of whole pixels where `clipped`, drawn in one strip
/// and at `cuts`.
fn probed_at(cuts: &[u32], clipped: bool) -> (Raster, Raster) {
    let (x0, y0, x1, y1) = PROBED;
    let mut list = DisplayList::new(Size::new(SIDE as f32, SIDE as f32));
    let clip = clipped.then(|| {
        list.add_clip(Clip {
            path: rectangle(0.0, 0.0, 30.0, SIDE as f32),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("a clip")
    });
    list.push(fill_under(rectangle(x0, y0, x1, y1), clip));
    let whole = CpuRasterizer::new()
        .with_strips(1)
        .rasterize(&list, target(SIDE, SIDE))
        .expect("a rectangle draws");
    let cut = CpuRasterizer::new()
        .with_cuts(cuts)
        .rasterize(&list, target(SIDE, SIDE))
        .expect("a rectangle draws");
    (whole, cut)
}

/// A cut at any row across a rectangle draws the page's bytes, unclipped and under a clip whose
/// mask is a set of whole pixels, which the converter is handed beside the rectangle. Printed at
/// the cut that leaves one row above it, beside §10.7.4's area for each pixel, which is neither
/// drawing's: the converter states 192 for an area of 191.25.
#[test]
fn a_stated_cut_at_any_row_across_a_rectangle_draws_the_page() {
    let (x0, y0, x1, y1) = PROBED;
    let area = Rect::from_corners(Point::new(x0, y0), Point::new(x1, y1));
    for clipped in [false, true] {
        for cut in 16..=34 {
            let (whole, strips) = probed_at(&[0, cut, SIDE], clipped);
            if cut == 20 {
                for x in [5, 6, 20, 29] {
                    println!(
                        "clipped {clipped}, cut at 20, pixel ({x}, 19): whole {}, strips {}, area \
                         {:.3}",
                        coverage(&whole, x, 19),
                        coverage(&strips, x, 19),
                        pdf_render::rectangle_coverage(area, x as f32, 19.0) * 255.0
                    );
                }
            }
            let found = differences(&whole, &strips);
            assert!(
                found.is_empty(),
                "clipped {clipped}, a cut at {cut} differs: {found:?}"
            );
        }
    }
}

/// A cut at any row across a rule whose outline is a rectangle draws the page's bytes: a
/// horizontal and a vertical rule under each cap the stroker leaves a rectangle for.
#[test]
fn a_stated_cut_at_any_row_across_a_rule_draws_the_page() {
    for cap in [LineCap::Butt, LineCap::Square] {
        for (what, path, width) in [
            ("horizontal", line(5.375, 20.375, 40.5, 20.375), 1.75),
            ("vertical", line(20.25, 19.375, 20.25, 31.625), 2.5),
        ] {
            let mut list = DisplayList::new(Size::new(SIDE as f32, SIDE as f32));
            list.push(stroked(path, width, cap, Vec::new()));
            let whole = CpuRasterizer::new()
                .with_strips(1)
                .rasterize(&list, target(SIDE, SIDE))
                .expect("a rule draws");
            for cut in 16..=34 {
                let strips = CpuRasterizer::new()
                    .with_cuts(&[0, cut, SIDE])
                    .rasterize(&list, target(SIDE, SIDE))
                    .expect("a rule draws");
                let found = differences(&whole, &strips);
                assert!(
                    found.is_empty(),
                    "the {what} rule under {cap:?}, a cut at {cut} differs: {found:?}"
                );
            }
        }
    }
}

/// A rectangle with one row inside the page and the rest above it is stated at that row as the
/// same rectangle moved eight rows down, onto a page that holds all of it: §10.7.4's pixel is the
/// same square wherever the page's edge falls, so a page and the strips of another division of it
/// state the row alike.
#[test]
fn a_rectangle_one_row_inside_the_page_is_its_row_unclipped() {
    let drawn = |top: f32| {
        let mut list = DisplayList::new(Size::new(SIDE as f32, SIDE as f32));
        list.push(filled(rectangle(5.375, top - 6.0, 40.5, top + 0.625)));
        CpuRasterizer::new()
            .with_strips(1)
            .rasterize(&list, target(SIDE, SIDE))
            .expect("a rectangle draws")
    };
    let (edge, inside) = (drawn(0.0), drawn(8.0));
    for x in 0..SIDE {
        assert_eq!(
            coverage(&edge, x, 0),
            coverage(&inside, x, 8),
            "column {x}: the page's first row against the same row of the rectangle unclipped"
        );
    }
}

/// A page of rectangles and of strokes whose outline is one, each placed with dyadic fractional
/// edges, in bands sixteen rows apart and offset from one another so that the edges fall in every
/// row's neighbourhood somewhere on the page while a strip still has rows to begin at: filled
/// rectangles a row and a half to six rows tall, horizontal and vertical rules under each cap the
/// converter is handed, and a short dashed vertical rule whose two dashes are one rectangle each.
fn page_of_rectangles(width: u32, height: u32, marks: Marks) -> DisplayList {
    let (w, h) = (width as f32, height as f32);
    let mut list = DisplayList::new(Size::new(w, h));
    let mut y = 2.25_f32;
    let mut turn = 0_u32;
    while y + 12.0 < h {
        let mut x = 3.125_f32;
        while x + 40.0 < w {
            let tall = 1.5 + 0.875 * (turn % 6) as f32;
            if marks == Marks::Fills {
                list.push(filled(rectangle(x, y, x + 9.75, y + tall)));
                list.push(filled(rectangle(x + 12.5, y + 2.625, x + 30.0, y + 9.125)));
                turn += 1;
                x += 44.0;
                continue;
            }
            let cap = if turn.is_multiple_of(2) {
                LineCap::Butt
            } else {
                LineCap::Square
            };
            let down = y + 1.875 * (turn % 5) as f32;
            list.push(stroked(
                line(x + 12.5, down + 0.375, x + 24.0, down + 0.375),
                1.75,
                cap,
                Vec::new(),
            ));
            list.push(stroked(
                line(x + 27.25, down + 0.625, x + 27.25, down + 6.5),
                2.5,
                cap,
                Vec::new(),
            ));
            if turn.is_multiple_of(5) {
                list.push(stroked(
                    line(x + 32.75, down + 0.125, x + 32.75, down + 8.875),
                    1.5,
                    LineCap::Butt,
                    vec![3.25, 1.5],
                ));
            }
            turn += 1;
            x += 44.0;
        }
        y += 16.375;
    }
    list
}

/// Which marks [`page_of_rectangles`] states: filled rectangles, or strokes whose outline is one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Marks {
    Fills,
    Strokes,
}

/// [`page_of_rectangles`] of `marks` in one strip and then in 2, 4, 8 and 16: every division the
/// planner makes cuts the page, and every one is the page drawn in one strip to the byte.
fn assert_its_own_strips(marks: Marks) {
    let (width, height) = (368, 542);
    let list = page_of_rectangles(width, height, marks);
    let target = target(width, height);
    let one = CpuRasterizer::new()
        .with_strips(1)
        .rasterize(&list, target)
        .expect("the page draws");
    for strips in [2, 4, 8, 16] {
        let mut backend = CpuRasterizer::new().with_strips(strips);
        let plan = backend.strip_plan(&list, target);
        let cut = backend.rasterize(&list, target).expect("the page draws");
        let found = differences(&one, &cut);
        println!(
            "{strips} strips asked, cuts {plan:?}, {} pixels differ",
            found.len()
        );
        assert!(
            plan.len() > 2,
            "{strips} strips asked and the page is not cut, cuts {plan:?}"
        );
        assert!(
            found.is_empty(),
            "the page in strips at {plan:?} differs from one strip at {} pixels: {:?}",
            found.len(),
            found.get(..found.len().min(12))
        );
    }
}

/// A page of filled rectangles is its own strips at every count (ADR 2084).
#[test]
fn a_page_of_filled_rectangles_is_its_own_strips_at_every_count() {
    assert_its_own_strips(Marks::Fills);
}

/// A page of rules whose outlines are rectangles is its own strips at every count (ADR 2084).
#[test]
fn a_page_of_rules_is_its_own_strips_at_every_count() {
    assert_its_own_strips(Marks::Strokes);
}
