//! A page drawn ahead in strips on the pool, part by part, and finished from its list.
//!
//! `render_cpu::Underway::in_strips` cuts the target evenly and draws every part it is handed into
//! each strip on the pool (ADR 1896). Its page is defined as the page drawn whole at the same
//! cuts, `CpuRasterizer::with_cuts`, and the three properties below are what that definition
//! rests on, in the order a mistake in each would cost:
//!
//! 1. **The page finished from its parts is the page drawn whole at its cuts, byte for byte**, on
//!    a page of thousands of marks across every strip and on the scenes the other drawing tests
//!    use.
//! 2. **A mark across a cut that only one strip draws exactly ends the drawing ahead**, with a
//!    sentence, rather than drawing another picture: with anti-aliasing withdrawn every mark's
//!    oblique edge's rows are forbidden (`CpuRasterizer::draws_row_by_row`), so a page-tall slanted
//!    band forbids every cut.
//! 3. **A target too short for two strips is one surface**, which is `Underway::new`'s page.

#![expect(
    clippy::expect_used,
    reason = "test code: a scene the rasteriser refuses should fail loudly, and the helpers that \
              draw them are outside the `#[test]`s `clippy.toml` already permits"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point,
    Rasterizer as _, Size, Stated, Stroke, TargetSpec, Transform,
};
use render_cpu::{CpuRasterizer, Underway};

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// A page of `marks` small oblique strokes and squares spread down and across it, every eighth
/// one half transparent, so that every strip of any division draws some and a cut crosses many.
fn many_marks(marks: u16) -> DisplayList {
    let mut list = DisplayList::new(Size {
        width: 300.0,
        height: 400.0,
    });
    for mark in 0..marks {
        let (column, row) = (f32::from(mark % 37), f32::from(mark / 37));
        let (x, y) = (5.0 + column * 7.9, 3.0 + (row * 2.7) % 394.0);
        let colour = Color {
            r: f32::from(mark % 5) / 4.0,
            g: f32::from(mark % 7) / 6.0,
            b: 0.4,
            a: if mark % 8 == 0 { 0.5 } else { 1.0 },
        };
        if mark % 3 == 0 {
            list.push(Command::Fill {
                path: Arc::new(test_scenes::rect(x, y, x + 2.3, y + 2.9)),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(colour),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        } else {
            let mut line = Path::new();
            line.push(PathCommand::MoveTo(Point::new(x, y)));
            line.push(PathCommand::LineTo(Point::new(x + 4.1, y + 3.3)));
            list.push(Command::Stroke {
                path: Arc::new(line),
                transform: Transform::IDENTITY,
                stroke: Stroke {
                    width: 0.7,
                    ..Stroke::default()
                },
                paint: Paint::Solid(colour),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
    }
    list
}

/// `list` handed to an `Underway` in `parts` parts and finished from it.
fn drawn_ahead(
    underway: Option<Underway>,
    list: &DisplayList,
    parts: usize,
) -> (Vec<u32>, Result<Vec<u8>, &'static str>) {
    let mut underway = underway.expect("a target this backend draws");
    let stated = list.stated();
    let per = stated.commands.div_ceil(parts.max(1)).max(1);
    let mut from = Stated::default();
    while from.commands < stated.commands {
        let mut part = list.part_since(from).expect("a prefix of the list");
        part.commands.truncate(per);
        from = part.until();
        if !underway.take(part) {
            break;
        }
    }
    let cuts = underway.cuts().to_vec();
    (
        cuts,
        underway.finish(list).map(|finished| finished.raster.data),
    )
}

/// The page drawn whole at `cuts`.
fn whole_at(
    rasterizer: CpuRasterizer,
    list: &DisplayList,
    target: TargetSpec,
    cuts: &[u32],
) -> Vec<u8> {
    rasterizer
        .with_cuts(cuts)
        .rasterize(list, target)
        .expect("the page draws whole")
        .data
}

#[test]
fn a_page_finished_from_its_parts_in_strips_is_the_page_drawn_whole_at_its_cuts() {
    let list = many_marks(6_000);
    let target = TargetSpec::for_page(&list, 2.0, GENEROUS).expect("a target");
    for strips in [2, 4, 7] {
        let rasterizer = CpuRasterizer::new().with_strips(strips);
        let (cuts, finished) = drawn_ahead(Underway::in_strips(&rasterizer, target), &list, 5);
        assert_eq!(
            cuts.len(),
            strips as usize + 1,
            "{strips} strips asked of {} rows",
            target.height
        );
        let finished = finished.expect("a page of marks drawn a row at a time is finished");
        assert!(
            finished == whole_at(CpuRasterizer::new(), &list, target, &cuts),
            "the page drawn ahead in {strips} strips is the page drawn whole at {cuts:?}"
        );
    }
}

#[test]
fn every_scene_drawn_ahead_in_strips_is_the_scene_drawn_whole_at_its_cuts_or_says_why_not() {
    let scenes = [
        ("basic", test_scenes::basic()),
        ("curves", test_scenes::curves()),
        ("diagonal_stroke", test_scenes::diagonal_stroke()),
        ("transparency_group", test_scenes::transparency_group()),
        ("knockout_group", test_scenes::knockout_group()),
        ("blend_modes", test_scenes::blend_modes()),
        ("soft_mask", test_scenes::soft_mask()),
    ];
    let mut finished_in_strips = 0;
    for (name, list) in &scenes {
        let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("a target");
        let rasterizer = CpuRasterizer::new().with_strips(4);
        let (cuts, finished) = drawn_ahead(Underway::in_strips(&rasterizer, target), list, 2);
        match finished {
            Ok(finished) => {
                assert!(
                    finished == whole_at(CpuRasterizer::new(), list, target, &cuts),
                    "{name} drawn ahead at {cuts:?} is {name} drawn whole there"
                );
                finished_in_strips += usize::from(cuts.len() > 2);
            }
            Err(reason) => assert!(!reason.is_empty(), "{name} says why it was not finished"),
        }
    }
    assert!(
        finished_in_strips >= 3,
        "most scenes are finished in strips, and {finished_in_strips} were"
    );
}

#[test]
fn a_mark_across_a_cut_that_one_strip_draws_ends_the_drawing_ahead() {
    // A slanted band down the whole page: an oblique edge, which ADR 0139 measured a clipping
    // converter drawing differently on either side of a cut.
    let mut band = Path::new();
    band.push(PathCommand::MoveTo(Point::new(140.0, 1.0)));
    band.push(PathCommand::LineTo(Point::new(160.0, 1.0)));
    band.push(PathCommand::LineTo(Point::new(190.0, 399.0)));
    band.push(PathCommand::LineTo(Point::new(170.0, 399.0)));
    band.push(PathCommand::Close);
    let mut list = many_marks(300);
    list.push(Command::Fill {
        path: Arc::new(band),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        }),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, 2.0, GENEROUS).expect("a target");
    let aliased = CpuRasterizer::new().with_anti_alias(false).with_strips(4);
    let (cuts, finished) = drawn_ahead(Underway::in_strips(&aliased, target), &list, 3);
    assert_eq!(cuts.len(), 5, "the target was cut");
    assert_eq!(
        finished.err(),
        Some("a mark crosses a row the page was cut at, where only one strip draws it exactly"),
    );
}

#[test]
fn a_target_too_short_for_two_strips_is_one_surface() {
    let list = many_marks(500);
    let target = TargetSpec::for_page(&list, 0.25, GENEROUS).expect("a target");
    assert!(target.height < 128, "{} rows", target.height);
    let rasterizer = CpuRasterizer::new().with_strips(8);
    let (cuts, finished) = drawn_ahead(Underway::in_strips(&rasterizer, target), &list, 3);
    assert_eq!(cuts, vec![0, target.height]);
    let (_, one) = drawn_ahead(Underway::new(&rasterizer, target), &list, 3);
    assert!(
        finished.expect("finished") == one.expect("finished"),
        "one strip drawn ahead is one surface drawn ahead"
    );
}
