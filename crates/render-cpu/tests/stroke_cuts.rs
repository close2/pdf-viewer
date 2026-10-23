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
//!
//! **A fill is asked the same question** (ADR 1837): ADR 0139 forbade a fill's curves and oblique
//! edges for the same reason it forbade a stroke's extent, and the same converter draws both. So
//! two scenes of fills are held the same way — discs and diamonds stacked so that every row of the
//! page crosses one, which the old rule left as one strip — and a page of a few large discs is held
//! to the planner's other half: a cut the freed rows permit but that replays too much gives way to
//! the cuts between the fills, never to one strip.

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

/// A filled path through `points`, closed.
fn polygon(points: &[(f32, f32)]) -> Command {
    let mut path = Path::new();
    for (index, (x, y)) in points.iter().enumerate() {
        let point = Point::new(*x, *y);
        path.push(if index == 0 {
            PathCommand::MoveTo(point)
        } else {
            PathCommand::LineTo(point)
        });
    }
    path.push(PathCommand::Close);
    filled(path)
}

/// A filled disc of radius `r` about `(x, y)`, as four cubics — the curves a glyph is made of.
fn disc(x: f32, y: f32, r: f32) -> Command {
    // The usual four-arc approximation's control distance, 4/3 · (√2 − 1).
    let k = 0.552_284_8 * r;
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x + r, y)));
    path.push(PathCommand::CurveTo(
        Point::new(x + r, y + k),
        Point::new(x + k, y + r),
        Point::new(x, y + r),
    ));
    path.push(PathCommand::CurveTo(
        Point::new(x - k, y + r),
        Point::new(x - r, y + k),
        Point::new(x - r, y),
    ));
    path.push(PathCommand::CurveTo(
        Point::new(x - r, y - k),
        Point::new(x - k, y - r),
        Point::new(x, y - r),
    ));
    path.push(PathCommand::CurveTo(
        Point::new(x + k, y - r),
        Point::new(x + r, y - k),
        Point::new(x + r, y),
    ));
    path.push(PathCommand::Close);
    filled(path)
}

/// A solid black fill of `path` under the non-zero rule.
fn filled(path: Path) -> Command {
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: pdf_render::FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// Discs in staggered columns, each column's discs overlapping the next column's rows, so every
/// row of the page crosses a curve — a page of glyphs with no gap between its lines.
fn discs() -> DisplayList {
    let mut list = DisplayList::new(Size::new(600.0, 900.0));
    for column in 0..12 {
        for row in 0..45 {
            let stagger = if column % 2 == 0 { 0.0 } else { 10.125 };
            let (x, y) = (
                25.5 + 48.25 * column as f32,
                10.75 + 19.5 * row as f32 + stagger,
            );
            list.push(disc(x, y, 6.0 + 0.25 * (row % 5) as f32));
        }
    }
    list
}

/// Diamonds at dyadic coordinates, stacked so that every row crosses an oblique edge — their side
/// vertices off the device's whole rows, where ADR 0139's rule would leave a cut legal.
fn diamonds() -> DisplayList {
    let mut list = DisplayList::new(Size::new(600.0, 900.0));
    for column in 0..10 {
        for row in 0..36 {
            let (x, y) = (30.5 + 56.0 * column as f32, 4.25 + 24.75 * row as f32);
            list.push(polygon(&[
                (x, y),
                (x + 12.5, y + 13.125),
                (x, y + 26.5),
                (x - 6.25, y + 13.125),
            ]));
        }
    }
    list
}

/// A sheet of the owner's drawing's dots: one-segment round-capped strokes a fifth of a pixel long,
/// every one inside two by two pixels, at coordinates that are not dyadic, stacked so that every row
/// of the page crosses several — and every other one thinner than one level of a pixel, which
/// `tiny-skia`'s hairline drew before `pdf_render::thin_segment` took it (ADR 1847).
///
/// What these scenes hold is that such a mark is measured on the page's own grid in every strip: a
/// strip's shifted origin would round its endpoints at another magnitude (ADR 0219), so the
/// non-dyadic coordinates are the point rather than an accident.
fn dots() -> DisplayList {
    let mut list = DisplayList::new(Size::new(400.0, 600.0));
    for index in 0..6000_u32 {
        let y = 0.31 + 0.0999 * index as f32;
        let x = 3.7 + ((index * 37) % 390) as f32 + 0.137 * (index % 7) as f32;
        let width = if index % 2 == 0 { 0.0013 } else { 0.23 };
        list.push(polyline(
            &[(x, y), (x + 0.21, y + 0.17)],
            false,
            drawing_pen(width),
        ));
    }
    list
}

/// A page of thin segments is cut, and the cut draws the page byte for byte: each mark's areas are
/// the page's in every strip, and a strip only moves the rows they land in (ADR 1847, trap 146).
#[test]
fn a_thin_segment_cut_by_a_strip_draws_the_page() {
    let list = dots();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a valid target");
    let extents = pdf_render::command_extents(&list, target);
    let costs = pdf_render::row_costs(&extents, target);
    let before = pdf_render::unsplittable_rows(&list, target);
    assert_eq!(
        pdf_render::strip_boundaries_avoiding(&costs, &before, 16, 64).len(),
        2,
        "the dots could already be cut under ADR 0139's rule"
    );
    let backend = CpuRasterizer::new().with_strips(16);
    let asked = std::cell::Cell::new(0_u32);
    let freed = pdf_render::unsplittable_rows_where(&list, target, &|mark| {
        let answer = backend.draws_row_by_row(mark, target);
        if let pdf_render::PlannedMark::Stroke(stroke) = mark
            && stroke.stroke.width < 0.002
        {
            asked.set(asked.get() + 1);
            assert!(answer, "a dot under one level of a pixel forbids its rows");
        }
        answer
    });
    assert_eq!(asked.get(), 3000, "every thinner dot was asked");
    assert!(
        freed.iter().all(|row| !*row),
        "a row is still forbidden: the dots are all thin segments"
    );
    let plan = backend.strip_plan(&list, target);
    assert!(
        plan.len() > 8,
        "the dots are granted too few cuts: {plan:?}"
    );
    let whole = drawn(&list, target, 1);
    for strips in [2_u32, 3, 5, 8, 16] {
        let (moved, worst, first) = difference(&whole, &drawn(&list, target, strips), target.width);
        assert_eq!(
            moved, 0,
            "the dots at {strips} strips: {moved} pixels differ by up to {worst}, the first at \
             {first:?}"
        );
    }
}

/// And the other side of [`rings`]: a ring whose pen is wider than its radius bends more tightly
/// than its half-width, which ADR 1348 strokes as convex pieces, measured from their union's
/// boundary unless that declines the mark to `tiny-skia`'s converter (ADR 2122), so it keeps its
/// rows — the backend's answer is the draw's own question, not "a curve is fine".
#[test]
fn a_curve_that_may_fold_keeps_its_rows() {
    let list = rings(14.0);
    let target = TargetSpec::for_page(&list, 2.0, GENEROUS).expect("a valid target");
    let backend = CpuRasterizer::new().with_strips(16);
    let rows = pdf_render::unsplittable_rows_where(&list, target, &|mark| {
        backend.draws_row_by_row(mark, target)
    });
    assert!(
        rows.iter().all(|row| *row),
        "a row of the folding rings is free"
    );
}

/// A fill taller than one accumulator holds at the page's width and far narrower than the page — a
/// slanted rule of a drawing's border, say — is measured in one accumulator of its own columns, so
/// its suspicion of overlap is the same in every division and it forbids no row (ADR 1847). Ten of
/// them side by side, each with an oblique edge in every row, which ADR 0139's rule left as one
/// strip, among a sheet of small squares; and the width they would have been asked at before, against `area`'s budget of 2^22 cells,
/// asserted to refuse them, so the scene is one the page's width could not free.
#[test]
fn a_tall_narrow_fill_is_measured_in_its_own_columns_and_forbids_no_cut() {
    const CELL_BUDGET: f32 = 4_194_304.0;
    let (width, height) = (600.0_f32, 7600.0_f32);
    let mut list = DisplayList::new(Size::new(width, height));
    for column in 0..10 {
        let x = 20.5 + 57.25 * column as f32;
        list.push(polygon(&[
            (x, 12.25),
            (x + 6.5, 12.25),
            (x + 30.75, height - 12.5),
            (x + 24.25, height - 12.5),
        ]));
    }
    // What the rules border: small squares, which forbid no row, so that the tall fills are a
    // small share of what every strip replays (`pdf_render::replay_ratio`).
    for row in 0..400 {
        for column in 0..20 {
            let (x, y) = (14.5 + 29.0 * column as f32, 20.25 + 18.75 * row as f32);
            list.push(polygon(&[
                (x, y),
                (x + 6.5, y),
                (x + 6.5, y + 6.5),
                (x, y + 6.5),
            ]));
        }
    }
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a valid target");
    assert!(
        (height - 24.75 + 4.0) * (width + 1.0) > CELL_BUDGET,
        "a fill this tall fits one accumulator at the page's width, so it says nothing"
    );
    let extents = pdf_render::command_extents(&list, target);
    let costs = pdf_render::row_costs(&extents, target);
    let before = pdf_render::unsplittable_rows(&list, target);
    assert_eq!(
        pdf_render::strip_boundaries_avoiding(&costs, &before, 16, 64).len(),
        2,
        "the fills could already be cut under ADR 0139's rule"
    );
    let plan = CpuRasterizer::new()
        .with_strips(16)
        .strip_plan(&list, target);
    assert!(
        plan.len() > 8,
        "the fills are granted too few cuts: {plan:?}"
    );
    let whole = drawn(&list, target, 1);
    let pixels = (target.width as usize) * (target.height as usize);
    for strips in [2_u32, 5, 16] {
        let (moved, worst, first) = difference(&whole, &drawn(&list, target, strips), target.width);
        assert!(
            worst <= 1 && moved * 1000 < pixels,
            "the fills at {strips} strips: {moved} of {pixels} pixels differ by up to {worst}, \
             the first at {first:?}"
        );
    }
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
fn bounded_scenes() -> [(&'static str, DisplayList, f32); 5] {
    [
        ("round grid", grid(drawing_pen), 2.0),
        ("hatching", hatching(), 2.0),
        ("discs", discs(), 2.0),
        ("diamonds", diamonds(), 2.0),
        ("rings", rings(1.5), 2.0),
    ]
}

/// The disc of [`disc`] stroked rather than filled, under `pen`.
fn ring(x: f32, y: f32, r: f32, pen: Stroke) -> Command {
    let Command::Fill { path, .. } = disc(x, y, r) else {
        unreachable!("`disc` states a fill")
    };
    Command::Stroke {
        path,
        transform: Transform::IDENTITY,
        stroke: pen,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// The owner's drawing's curved strokes: rings of round-capped pen `width` wide, staggered as
/// [`discs`] are so that every row of the page crosses a curve, each bending far less tightly than
/// its half-width — so the stroker's outline draws it and `area` measures that, which ADR 1348's
/// pieces would not (ADR 1847).
fn rings(width: f32) -> DisplayList {
    let mut list = DisplayList::new(Size::new(600.0, 900.0));
    for column in 0..12 {
        for row in 0..45 {
            let stagger = if column % 2 == 0 { 0.0 } else { 10.125 };
            let (x, y) = (
                25.5 + 48.25 * column as f32,
                10.75 + 19.5 * row as f32 + stagger,
            );
            list.push(ring(
                x,
                y,
                6.0 + 0.25 * (row % 5) as f32,
                drawing_pen(width),
            ));
        }
    }
    list
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

/// A page of a few large discs one above the other: every cut the freed rows permit runs through a
/// disc and replays it, so the planner's replay bound refuses each count those cuts make, and the
/// page is cut in the gaps between the discs instead — the division ADR 0139's rule for fills made
/// — rather than drawn as one strip (ADR 1837).
#[test]
fn a_page_of_a_few_large_fills_is_cut_between_them_rather_than_not_at_all() {
    let mut list = DisplayList::new(Size::new(300.0, 1500.0));
    for row in 0..6 {
        list.push(disc(150.0, 125.0 + 250.0 * row as f32, 100.0));
    }
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a valid target");
    let extents = pdf_render::command_extents(&list, target);
    let costs = pdf_render::row_costs(&extents, target);
    let freed = pdf_render::unsplittable_rows_where(&list, target, &|_| true);
    let through = pdf_render::strip_boundaries_avoiding(&costs, &freed, 16, 64);
    assert!(
        pdf_render::replay_ratio(&extents, &through) > 1.25,
        "the freed cuts replay little, so the page does not exercise the fallback"
    );
    let plan = CpuRasterizer::new()
        .with_strips(16)
        .strip_plan(&list, target);
    assert_eq!(plan.len(), 7, "six discs, five gaps between them: {plan:?}");
    assert!(pdf_render::replay_ratio(&extents, &plan) <= 1.25);
    // The cuts cross no mark, so what may move is ADR 0219's shifted origin on the discs' chords
    // alone, and ADR 1758's bound for one coverage holds it.
    let whole = drawn(&list, target, 1);
    let pixels = (target.width as usize) * (target.height as usize);
    let (moved, worst, first) = difference(&whole, &drawn(&list, target, 16), target.width);
    assert!(
        worst <= 1 && moved * 1000 < pixels,
        "{moved} of {pixels} pixels differ by up to {worst}, the first at {first:?}"
    );
}
