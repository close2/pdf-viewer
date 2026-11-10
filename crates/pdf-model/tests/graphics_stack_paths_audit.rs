//! Regression checks for the boundary between the graphics-state stack and the current path,
//! and for when a pending clipping rule becomes part of the graphics state.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are deliberately small; malformed fixtures should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Command, FillRule, PathCommand, Point, Rasterizer, TargetSpec, Transform};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn list(content: &str) -> pdf_render::DisplayList {
    let document = Document::open(fixture(content)).expect("fixture opens");
    assert!(!document.was_recovered(), "fixture parses without recovery");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture interpretation is complete: {:?}",
        interpretation.unsupported
    );
    interpretation.display_list
}

#[test]
fn q_and_q_operators_leave_the_current_path_outside_the_stack() {
    let display_list = list("10 10 m q 20 20 l Q 30 30 l S");
    let strokes: Vec<_> = display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Stroke { path, .. } => Some(path.commands().to_vec()),
            _ => None,
        })
        .collect();
    assert_eq!(
        strokes,
        [vec![
            PathCommand::MoveTo(Point::new(10.0, 10.0)),
            PathCommand::LineTo(Point::new(20.0, 20.0)),
            PathCommand::LineTo(Point::new(30.0, 30.0)),
        ]],
        "the path and its current point continue across graphics-state save and restore"
    );
}

#[test]
fn nested_restores_pop_the_most_recent_graphics_state_first() {
    let display_list = list(
        "q 10 10 60 60 re W n q 20 20 30 30 re W n 0 0 100 100 re f Q 0 0 100 100 re f Q 0 0 100 100 re f",
    );
    let clips: Vec<_> = display_list
        .commands()
        .iter()
        .filter_map(|command| {
            if let Command::Fill { clip, .. } = command {
                Some(*clip)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(clips.len(), 3);
    let inner = display_list
        .clip(clips[0].expect("inner scope clips"))
        .expect("inner clip");
    assert_eq!(
        inner.parent, clips[1],
        "the first restore returns to the enclosing scope"
    );
    assert!(clips[1].is_some(), "the enclosing scope is still clipped");
    assert_eq!(
        clips[2], None,
        "the second restore returns to the initial scope"
    );
}

#[test]
fn construction_without_painting_emits_no_marks() {
    let display_list =
        list("10 10 m 20 20 l 30 40 50 60 70 70 c 80 80 90 90 v 50 60 20 30 y h 10 10 20 20 re n");
    assert!(
        display_list.commands().is_empty(),
        "construction and n produce no painting commands"
    );
}

#[test]
fn an_explicit_cubic_uses_the_current_point_and_leaves_its_endpoint_current() {
    let display_list = list("10 10 m 20 30 40 50 60 70 c 80 90 l S");
    let path = display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Stroke { path, .. } => Some(path),
            _ => None,
        })
        .expect("the fixture strokes one path");
    assert_eq!(
        path.commands(),
        [
            PathCommand::MoveTo(Point::new(10.0, 10.0)),
            PathCommand::CurveTo(
                Point::new(20.0, 30.0),
                Point::new(40.0, 50.0),
                Point::new(60.0, 70.0),
            ),
            PathCommand::LineTo(Point::new(80.0, 90.0)),
        ],
        "c supplies the two controls and endpoint, and the following l starts at that endpoint"
    );
}

#[test]
fn clipping_is_deferred_until_paint_then_restore_returns_the_prior_clip() {
    let display_list = list("q 20 20 30 30 re W f 0 0 100 100 re f Q 0 0 100 100 re f");
    let fills: Vec<_> = display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                clip, fill_rule, ..
            } => Some((*clip, *fill_rule)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills.len(),
        3,
        "all three path fills reach the display list"
    );
    assert_eq!(fills[0], (None, FillRule::NonZero));
    let committed = fills[1]
        .0
        .expect("the first fill commits the pending W clip");
    let clip = display_list
        .clip(committed)
        .expect("clip is in the display list");
    assert_eq!(clip.fill_rule, FillRule::NonZero);
    assert_eq!(
        clip.path.commands(),
        [
            PathCommand::MoveTo(Point::new(20.0, 20.0)),
            PathCommand::LineTo(Point::new(50.0, 20.0)),
            PathCommand::LineTo(Point::new(50.0, 50.0)),
            PathCommand::LineTo(Point::new(20.0, 50.0)),
            PathCommand::Close,
        ],
        "the committed clip is exactly the rectangle named before W"
    );
    assert_eq!(fills[1].0, Some(committed));
    assert_eq!(
        fills[2].0, None,
        "Q restores the clip that was current at q"
    );
}

#[test]
fn w_and_w_star_select_distinct_rules_and_clips_nest() {
    let display_list = list("10 10 60 60 re W n 20 20 30 30 re W* n 0 0 100 100 re f");
    let clip_id = match display_list.commands().last().expect("the fill exists") {
        Command::Fill { clip, .. } => clip.expect("the two clips apply to the fill"),
        other => panic!("expected final fill, got {other:?}"),
    };
    let inner = display_list.clip(clip_id).expect("inner clip exists");
    assert_eq!(inner.fill_rule, FillRule::EvenOdd);
    let parent_id = inner.parent.expect("the new clip intersects its parent");
    let outer = display_list.clip(parent_id).expect("outer clip exists");
    assert_eq!(outer.fill_rule, FillRule::NonZero);
    assert_eq!(
        outer.path.commands(),
        [
            PathCommand::MoveTo(Point::new(10.0, 10.0)),
            PathCommand::LineTo(Point::new(70.0, 10.0)),
            PathCommand::LineTo(Point::new(70.0, 70.0)),
            PathCommand::LineTo(Point::new(10.0, 70.0)),
            PathCommand::Close,
        ]
    );
    assert_eq!(
        inner.path.commands(),
        [
            PathCommand::MoveTo(Point::new(20.0, 20.0)),
            PathCommand::LineTo(Point::new(50.0, 20.0)),
            PathCommand::LineTo(Point::new(50.0, 50.0)),
            PathCommand::LineTo(Point::new(20.0, 50.0)),
            PathCommand::Close,
        ]
    );
    assert_eq!(outer.parent, None);
}

#[test]
fn a_clip_keeps_the_ctm_in_force_when_w_is_applied() {
    let display_list = list("2 0 0 3 5 7 cm 10 10 20 20 re W n 7 0 0 7 0 0 cm 0 0 100 100 re f");
    let clip_id = match display_list
        .commands()
        .last()
        .expect("the final fill exists")
    {
        Command::Fill { clip, .. } => clip.expect("the committed clip remains active"),
        other => panic!("expected final fill, got {other:?}"),
    };
    let clip = display_list.clip(clip_id).expect("clip is in the list");
    assert_eq!(clip.transform, Transform::new(2.0, 0.0, 0.0, 3.0, 5.0, 7.0));
    assert_eq!(
        clip.path.commands(),
        [
            PathCommand::MoveTo(Point::new(10.0, 10.0)),
            PathCommand::LineTo(Point::new(30.0, 10.0)),
            PathCommand::LineTo(Point::new(30.0, 30.0)),
            PathCommand::LineTo(Point::new(10.0, 30.0)),
            PathCommand::Close,
        ],
        "the clip retains its untransformed user-space coordinates alongside its CTM"
    );
}

#[test]
fn a_cubic_arch_places_its_midpoint_at_the_bezier_value() {
    let display_list = list("0 g 10 10 m 10 90 90 90 90 10 c h f");
    let target = TargetSpec::for_page(&display_list, 1.0, 1 << 20).expect("small page target");
    let raster = CpuRasterizer::new()
        .rasterize(&display_list, target)
        .expect("the parsed cubic renders");
    let red = |x: u32, y: u32| raster.data[((y * raster.width + x) * 4) as usize];

    // At t=1/2, (P0 + 3P1 + 3P2 + P3) / 8 = (50, 70). The arch's filled
    // region therefore contains the pixel centred at (50.5, 69.5) and excludes
    // the one centred at (50.5, 70.5), in PDF coordinates.
    assert!(
        red(50, 30) < 128,
        "the point just below the cubic is filled"
    );
    assert!(
        red(50, 29) > 127,
        "the point just above the cubic is unfilled"
    );
}

#[test]
fn the_initial_page_clip_stops_marks_past_the_media_box() {
    let display_list = list("0 g -10 20 120 60 re f");
    let mut target = TargetSpec::for_page(&display_list, 1.0, 1 << 20).expect("small page target");
    target.width = 120;
    let raster = CpuRasterizer::new()
        .rasterize(&display_list, target)
        .expect("the page renders into the wider target");
    let red = |x: u32, y: u32| raster.data[((y * raster.width + x) * 4) as usize];

    assert!(red(99, 50) < 128, "the mark reaches the page's last column");
    assert!(
        red(105, 50) > 127,
        "the target exposes space outside MediaBox, where the mark is clipped"
    );
}
