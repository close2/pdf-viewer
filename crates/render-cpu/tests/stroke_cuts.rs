//! A stroke this backend draws a row at a time may be cut anywhere, and the cut draws the page.
//!
//! `pdf_render::unsplittable_rows` counts every stroke's whole extent as rows no strip may begin
//! at, because under `tiny-skia`'s scan converter a cut re-states a round cap's curve or an
//! oblique edge (ADR 0139). This backend's own converter, `render_cpu::area`, evaluates every row
//! from the outline's device coordinates, so it asks `pdf_render::unsplittable_rows_where` instead
//! and forbids no row for a stroke it draws that way (ADR 1819). These scenes are what that claim
//! is held to: the shapes the owner's drawing is made of — a frame drawn round the whole sheet as
//! `re S`, a grid of thin round-capped rules, hatching — drawn whole and in strips, **byte for
//! byte**.
//!
//! **Each scene has to be one the old rule could not cut**, or equality would say nothing about
//! the cut: [`every_scene_was_one_strip_before`] asserts it, and that the new rule grants it cuts.
//!
//! The coordinates are dyadic fractions on purpose, and that is trap 12b's second half read the
//! other way: ADR 0219's shifted origin moves a pixel only where subtracting a strip's first row
//! from a coordinate rounds, which it never does from a dyadic fraction of this size. So what these
//! scenes measure is the cut alone, and the shifted origin is `pdf-model`'s division test's to hold
//! over real pages (ADR 1758).
//!
//! The frame's target is taller than one `area` accumulator holds, so the frame's region is
//! measured in runs of rows on the whole page and in fewer in a strip. That is the case which drew
//! one mark through two converters before a region's height stopped deciding the converter, and it
//! is why the scene is that large.

#![expect(
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects,
    reason = "test code over scenes of known size: the casts are small grid indices and the \
              arithmetic counts and indexes pixels of one raster"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these scenes request.
const GENEROUS: u64 = 1 << 30;

/// A stroke of `points` joined by straight segments, closed if asked.
fn polyline(points: &[(f32, f32)], close: bool, stroke: Stroke) -> Command {
    let mut path = Path::new();
    for (index, (x, y)) in points.iter().enumerate() {
        let point = Point::new(*x, *y);
        path.push(if index == 0 {
            PathCommand::MoveTo(point)
        } else {
            PathCommand::LineTo(point)
        });
    }
    if close {
        path.push(PathCommand::Close);
    }
    Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// The owner's drawing's own stroke style: round caps, mitre joins at the default limit.
fn drawing_pen(width: f32) -> Stroke {
    Stroke {
        width,
        cap: LineCap::Round,
        join: LineJoin::Miter,
        ..Stroke::default()
    }
}

/// A pen whose outline on a horizontal or vertical path is nothing but horizontal and vertical
/// edges at the path's own dyadic offsets: butt caps, mitre joins.
fn square_pen(width: f32) -> Stroke {
    Stroke {
        width,
        cap: LineCap::Butt,
        join: LineJoin::Miter,
        ..Stroke::default()
    }
}

/// A frame round the whole sheet, `re S` at fractional coordinates — the stroke that made the
/// owner's page one strip — round a sheet of short ticks.
///
/// The ticks are what a drawing's frame surrounds, and they are here for the planner: a page of
/// one page-wide command replays it in every strip, which `pdf_render::replay_ratio` refuses
/// however the rows are granted.
fn frame() -> DisplayList {
    let mut list = DisplayList::new(Size::new(2400.0, 2400.0));
    list.push(polyline(
        &[
            (20.25, 20.75),
            (2380.5, 20.75),
            (2380.5, 2379.25),
            (20.25, 2379.25),
        ],
        true,
        drawing_pen(3.5),
    ));
    for row in 0..40 {
        for column in 0..40 {
            let (x, y) = (60.5 + 57.25 * column as f32, 60.25 + 57.5 * row as f32);
            list.push(polyline(&[(x, y), (x, y + 20.75)], false, square_pen(1.5)));
        }
    }
    list
}

/// A grid of cells as a drawing states one: horizontal rules across the page, and vertical rules
/// a cell long between them, at widths either side of a device pixel, drawn with `pen`.
fn grid(pen: fn(f32) -> Stroke) -> DisplayList {
    let mut list = DisplayList::new(Size::new(600.0, 900.0));
    for row in 0..60 {
        let y = 10.75 + 14.5 * row as f32;
        list.push(polyline(&[(4.5, y), (595.25, y)], false, pen(0.75)));
        for column in 0..24 {
            let x = 12.25 + 24.5 * column as f32;
            let width = if column % 2 == 0 { 0.375 } else { 1.25 };
            list.push(polyline(&[(x, y), (x, y + 14.5)], false, pen(width)));
        }
    }
    list
}

/// Hatching: oblique rules a band of rows long, and a polyline of right angles with round joins
/// down the whole page.
fn hatching() -> DisplayList {
    let mut list = DisplayList::new(Size::new(600.0, 900.0));
    for row in 0..30 {
        for line in 0..12 {
            let (x, y) = (15.75 + 48.5 * line as f32, 10.25 + 29.5 * row as f32);
            list.push(polyline(
                &[(x, y), (x + 25.5, y + 30.25)],
                false,
                drawing_pen(0.875),
            ));
        }
    }
    let mut stairs = Vec::new();
    for step in 0..30 {
        let at = 15.5 + 28.75 * step as f32;
        stairs.push((at * 0.5, at));
        stairs.push((at * 0.5 + 17.25, at));
    }
    list.push(polyline(
        &stairs,
        false,
        Stroke {
            width: 4.5,
            cap: LineCap::Round,
            join: LineJoin::Round,
            ..Stroke::default()
        },
    ));
    list
}

/// Scenes whose outlines are horizontal and vertical edges at dyadic coordinates only, so a strip's
/// shifted origin cannot move a pixel and the cut is the only thing compared.
fn exact_scenes() -> [(&'static str, DisplayList, f32); 2] {
    [
        ("frame", frame(), 1.0),
        ("square grid", grid(square_pen), 2.0),
    ]
}

/// Scenes with round caps, round joins and oblique edges, whose outlines are not dyadic: here a
/// strip's shifted origin moves pixels whether or not a cut crosses the mark (ADR 0219), and what
/// is held is ADR 1758's bound for one coverage.
fn bounded_scenes() -> [(&'static str, DisplayList, f32); 2] {
    [
        ("round grid", grid(drawing_pen), 2.0),
        ("hatching", hatching(), 2.0),
    ]
}

/// Renders `list` at `strips`, the planner's own cuts.
fn drawn(list: &DisplayList, target: TargetSpec, strips: u32) -> Vec<u8> {
    CpuRasterizer::new()
        .with_strips(strips)
        .rasterize(list, target)
        .expect("the scene is supported")
        .data
}

/// Pixels that differ between two rasters, the worst byte, and the first differing pixel.
fn difference(one: &[u8], other: &[u8], width: u32) -> (usize, u8, Option<(usize, usize)>) {
    let mut moved = 0;
    let mut worst = 0;
    let mut first = None;
    for (at, (a, b)) in one.chunks_exact(4).zip(other.chunks_exact(4)).enumerate() {
        let most = a
            .iter()
            .zip(b)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        if most > 0 {
            moved += 1;
            worst = worst.max(most);
            first.get_or_insert((at % width as usize, at / width as usize));
        }
    }
    (moved, worst, first)
}

/// The premise of the comparisons below: the old rule left every scene one strip, and the new one
/// grants each of them cuts, so the bytes compared were drawn either side of one.
#[test]
fn every_scene_was_one_strip_before() {
    for (name, list, scale) in exact_scenes().into_iter().chain(bounded_scenes()) {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("a valid target");
        let extents = pdf_render::command_extents(&list, target);
        let costs = pdf_render::row_costs(&extents, target);
        let before = pdf_render::unsplittable_rows(&list, target);
        assert_eq!(
            pdf_render::strip_boundaries_avoiding(&costs, &before, 16, 64).len(),
            2,
            "{name} could already be cut, so it says nothing about a stroke's cut"
        );
        let plan = CpuRasterizer::new()
            .with_strips(16)
            .strip_plan(&list, target);
        assert!(
            plan.len() > 2,
            "{name} is granted no cut, so the comparison below draws it whole twice"
        );
    }
}

/// The claim, where nothing else can move a pixel: a stroke cut by a strip draws the page, byte
/// for byte — the frame's region measured in runs of rows on both sides of every cut included.
#[test]
fn a_stroke_cut_by_a_strip_draws_the_page() {
    for (name, list, scale) in exact_scenes() {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("a valid target");
        let whole = drawn(&list, target, 1);
        for strips in [2_u32, 3, 5, 8, 16] {
            let (moved, worst, first) =
                difference(&whole, &drawn(&list, target, strips), target.width);
            assert_eq!(
                moved, 0,
                "{name} at {strips} strips: {moved} pixels differ by up to {worst}, the first at \
                 {first:?}"
            );
        }
    }
}

/// And where a strip's shifted origin also reaches the outline: no pixel more than one level, and
/// fewer than one in a thousand at all — ADR 1758's bound for one coverage, which a curve or an
/// oblique edge re-stated by a cut would break (ADR 0138 measured up to 64 levels).
#[test]
fn a_round_or_oblique_stroke_cut_by_a_strip_stays_inside_the_division_bound() {
    for (name, list, scale) in bounded_scenes() {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("a valid target");
        let whole = drawn(&list, target, 1);
        let pixels = (target.width as usize) * (target.height as usize);
        for strips in [2_u32, 3, 5, 8, 16] {
            let (moved, worst, first) =
                difference(&whole, &drawn(&list, target, strips), target.width);
            assert!(
                worst <= 1 && moved * 1000 < pixels,
                "{name} at {strips} strips: {moved} of {pixels} pixels differ by up to {worst}, \
                 the first at {first:?}"
            );
        }
    }
}
