//! A knockout element's shape is the area its mark covers — ISO 32000-2 §11.4.6 and §10.7.4,
//! ADR 2286.
//!
//! # What the expected values come from
//!
//! §11.4.6 generalises knockout to a fractional shape:
//!
//! > The concept of knockout is generalized to accommodate fractional shape values. In that case,
//! > the immediate backdrop shall be only partially knocked out and shall be replaced by only a
//! > fraction of the result of compositing the object with the initial backdrop.
//!
//! and §11.3.7.2's NOTE 1 says that an anti-aliased boundary's fractional coverage *is* that shape.
//! So an opaque element over an opaque immediate backdrop `P` leaves `(1 − f) × P + f × S` at a
//! pixel it covers by `f`, element after element. No renderer enters the expected value but this
//! one drawing each element **alone**, under §11.3's ordinary compositing: that draw is the
//! coverage `crate::area` measures for every mark outside a knockout group (ADR 1082), and the
//! clause asks the group for the same `f`.
//!
//! The marks are those of `knockout_groups_test.pdf`'s worst pixel, scaled to a pixel grid: a
//! rectangle's two outlines, 0.66 of a pixel wide, whose top edges coincide, and a triangle whose
//! edges are no multiple of a sixteenth (trap 225). The library's converter states their coverage
//! on its lattice of sixteenths, a quarter for a band 0.13 of a pixel deep.

#![expect(
    clippy::expect_used,
    clippy::cast_precision_loss,
    reason = "test code: a panic with a message is the intended failure mode, and the casts are \
              of a page side this file chose, far below f32's exact range"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, LineCap, Paint, Path, PathCommand, Point,
    Raster, Rasterizer as _, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// The page's side, in pixels and in user units.
const SIDE: u32 = 24;

/// The immediate backdrop every element knocks into: opaque, and a colour whose three channels
/// differ so that a channel taken for another would show.
const BACKDROP: Color = Color::rgb(0.25, 0.5, 1.0);

/// A device point, with the page flipped as `TargetSpec::for_page` flips one.
fn at(x: f32, y: f32) -> Point {
    Point::new(x, SIDE as f32 - y)
}

/// One pixel a unit, flipped.
fn target() -> TargetSpec {
    TargetSpec {
        width: SIDE,
        height: SIDE,
        transform: Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, SIDE as f32),
    }
}

/// The closed polygon through `points`.
fn polygon(points: &[(f32, f32)]) -> Path {
    let mut path = Path::new();
    for (index, &(x, y)) in points.iter().enumerate() {
        path.push(if index == 0 {
            PathCommand::MoveTo(at(x, y))
        } else {
            PathCommand::LineTo(at(x, y))
        });
    }
    path.push(PathCommand::Close);
    path
}

/// `path` filled with `color` under the non-zero rule.
fn filled(path: Path, color: Color) -> Command {
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(color),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// `path` stroked solid black at `width`.
fn stroked(path: Path, width: f32) -> Command {
    Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            cap: LineCap::Butt,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// `commands` as the elements of an isolated §11.4.6 knockout group.
fn knockout(commands: Vec<Command>) -> Command {
    Command::Group {
        commands,
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
        isolated: true,
        knockout: true,
        alpha_is_shape: false,
        blending: None,
    }
}

/// `commands` drawn on the page.
fn drawn(commands: Vec<Command>) -> Raster {
    let mut list = DisplayList::new(Size::new(SIDE as f32, SIDE as f32));
    for command in commands {
        list.push(command);
    }
    CpuRasterizer::new()
        .rasterize(&list, target())
        .expect("the page draws")
}

/// The black marks the group knocks over its backdrop, in paint order: the two outlines'
/// top edges coincide at a depth of 0.13 of a pixel into row 6, and the triangle crosses both.
fn marks() -> Vec<Command> {
    let outline = |left: f32, right: f32| {
        polygon(&[
            (left, 5.47 + 0.33),
            (right, 5.47 + 0.33),
            (right, 19.3),
            (left, 19.3),
        ])
    };
    vec![
        stroked(outline(2.4, 13.7), 0.66),
        stroked(outline(8.47, 21.6), 0.66),
        filled(
            polygon(&[(3.37, 2.21), (20.83, 9.71), (6.91, 21.13)]),
            Color::BLACK,
        ),
    ]
}

/// Each element's coverage at every pixel, read off its own draw over white: a black mark
/// leaves `255 × (1 − f)` in every channel.
fn coverages() -> Vec<Vec<f64>> {
    marks()
        .into_iter()
        .map(|mark| {
            drawn(vec![mark])
                .data
                .chunks_exact(4)
                .map(|pixel| 1.0 - f64::from(pixel[0]) / 255.0)
                .collect()
        })
        .collect()
}

/// The group: an opaque backdrop the size of the page, then each mark knocking into it.
#[test]
fn a_knockout_element_knocks_out_the_area_it_covers() {
    let page = polygon(&[
        (0.0, 0.0),
        (SIDE as f32, 0.0),
        (SIDE as f32, SIDE as f32),
        (0.0, SIDE as f32),
    ]);
    let mut elements = vec![filled(page, BACKDROP)];
    elements.extend(marks());
    let group = drawn(vec![knockout(elements)]);
    let shapes = coverages();
    let backdrop = [BACKDROP.r, BACKDROP.g, BACKDROP.b].map(|c| f64::from(c) * 255.0);
    let mut partial = 0;
    let mut wrong = Vec::new();
    for (index, pixel) in group.data.chunks_exact(4).enumerate() {
        // §11.4.6 element after element: the black object replaces `f` of what is there.
        let kept: f64 = shapes.iter().map(|shape| 1.0 - shape[index]).product();
        if kept > 0.0 && kept < 1.0 {
            partial += 1;
        }
        for (channel, level) in backdrop.iter().enumerate() {
            let expected = level * kept;
            if (f64::from(pixel[channel]) - expected).abs() > 1.5 {
                wrong.push((
                    index % SIDE as usize,
                    index / SIDE as usize,
                    channel,
                    pixel[channel],
                ));
            }
        }
    }
    assert!(
        partial > 40,
        "only {partial} partly covered pixels: the scene no longer tests a fractional shape"
    );
    assert!(
        wrong.is_empty(),
        "{} channel(s) of {partial} partly covered pixels are not §11.4.6's weighted average of \
         the marks' own coverage (column, row, channel, level): {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(12)]
    );
}
