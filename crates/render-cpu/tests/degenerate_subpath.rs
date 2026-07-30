//! A stroke with no length: ISO 32000-2 §8.5.3.2, measured in pixels.
//!
//! # What the expected values come from
//!
//! §8.5.3.2, on a subpath that spans no distance:
//!
//! > the S operator shall paint it only if round line caps have been specified, producing a
//! > filled circle centred at the single point. If butt or projecting square line caps have
//! > been specified, S shall produce no output
//!
//! and, of a path that is only a `m`:
//!
//! > A single-point open subpath (specified by a trailing m operator) shall produce no
//! > output.
//!
//! and, of a dash pattern rather than a subpath:
//!
//! > This rule shall apply only to zero-length subpaths of the path being stroked, and not
//! > to zero-length dashes … In the latter case, the line caps shall always be painted
//!
//! So there are three different answers for three shapes that look alike, and every
//! assertion below is one of the clause's sentences turned into ink: a filled circle of
//! diameter `w` deposits `πw²/4` pixels' worth, and "no output" deposits none.
//!
//! # Why the scale varies
//!
//! Trap 2's rule. A dot's diameter is a *width*, and a width is stated in the path's space
//! and resolved against the device (§8.4.3.2), so a test at one scale cannot tell a
//! constant from something that tracks the transform. The circle's area grows with the
//! square of the scale, which is a sharper signal than a line's ink and is why these
//! measure area rather than a pixel.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, \
              and the arithmetic is over a hundred-unit page and a raster of known size"
)]

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use std::sync::Arc;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units.
const PAGE: f32 = 100.0;

/// The stroke width these scenes use, in PDF units.
const WIDTH: f32 = 10.0;

/// Builds a page holding one stroked path.
fn scene(commands: &[PathCommand], stroke: Stroke) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    for command in commands {
        path.push(*command);
    }
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Total darkness on the page, in units of one fully black pixel.
fn ink(raster: &Raster) -> f64 {
    let sum: u64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| u64::from(255 - pixel[0]))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "the sum is bounded by four bytes per pixel of a raster under a megapixel, \
                  far inside f64's exact integer range"
    )]
    let sum = sum as f64;
    sum / 255.0
}

/// Renders `list` at `scale` and returns the ink it deposited.
fn ink_at(list: &DisplayList, scale: f32) -> f64 {
    let target = TargetSpec::for_page(list, scale, GENEROUS).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a stroke is supported");
    ink(&raster)
}

/// The area of a circle of diameter `WIDTH` at `scale`, in device pixels.
fn disc(scale: f32) -> f64 {
    let radius = f64::from(WIDTH * scale) / 2.0;
    core::f64::consts::PI * radius * radius
}

/// A degenerate subpath under round caps is a filled circle — at every scale.
#[test]
fn a_single_point_closed_path_is_a_disc_under_round_caps() {
    let list = scene(
        &[
            PathCommand::MoveTo(Point::new(50.0, 50.0)),
            PathCommand::Close,
        ],
        Stroke {
            width: WIDTH,
            cap: LineCap::Round,
            ..Stroke::default()
        },
    );
    for scale in [1.0_f32, 2.0, 3.5] {
        let expected = disc(scale);
        let got = ink_at(&list, scale);
        // The tolerance is the circle's circumference times half a pixel, which is what
        // anti-aliasing can move an edge by, expressed generously.
        let edge = core::f64::consts::PI * f64::from(WIDTH * scale);
        assert!(
            (got - expected).abs() < edge,
            "at scale {scale}: {got:.1} pixels of ink, expected about {expected:.1}"
        );
    }
}

/// The same shape under the other two caps paints nothing at all.
///
/// This is the half that a rasteriser gets wrong on its own: Skia's stroker paints a
/// projecting square cap on a zero-length segment — it says so in a comment — where the
/// clause says "S shall produce no output, because the orientation of the caps would be
/// indeterminate". Deleting `split_degenerate`'s call in `render-cpu` makes this fail at
/// 100 pixels of ink against 0.
#[test]
fn a_single_point_closed_path_paints_nothing_under_butt_or_square_caps() {
    for cap in [LineCap::Butt, LineCap::Square] {
        let list = scene(
            &[
                PathCommand::MoveTo(Point::new(50.0, 50.0)),
                PathCommand::Close,
            ],
            Stroke {
                width: WIDTH,
                cap,
                ..Stroke::default()
            },
        );
        for scale in [1.0_f32, 2.0, 3.5] {
            assert!(
                ink_at(&list, scale) < 0.5,
                "{cap:?} at scale {scale} painted {:.1} pixels where the clause asks for none",
                ink_at(&list, scale)
            );
        }
    }
}

/// "two or more points at the same coordinates" is the same rule, stated for an open path.
#[test]
fn two_points_at_one_place_are_the_same_degenerate_subpath() {
    let list = scene(
        &[
            PathCommand::MoveTo(Point::new(50.0, 50.0)),
            PathCommand::LineTo(Point::new(50.0, 50.0)),
        ],
        Stroke {
            width: WIDTH,
            cap: LineCap::Round,
            ..Stroke::default()
        },
    );
    let expected = disc(1.0);
    assert!((ink_at(&list, 1.0) - expected).abs() < core::f64::consts::PI * f64::from(WIDTH));
}

/// A path that is only a `m` paints nothing, and is not an error.
///
/// Two clauses meet here and they agree: §8.5.3.2's last sentence says such a subpath
/// "shall produce no output", and §8.5.3.3.1 says a trailing one "shall be disregarded and
/// not considered to be part of the path". `tiny-skia` *refuses* such a path — it has no
/// segments — so before the rule was written down this scene was an `InvalidPath` error and
/// the whole page failed to render.
#[test]
fn a_lone_move_is_neither_a_mark_nor_an_error() {
    for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
        let list = scene(
            &[PathCommand::MoveTo(Point::new(50.0, 50.0))],
            Stroke {
                width: WIDTH,
                cap,
                ..Stroke::default()
            },
        );
        let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a valid target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("a path with nothing to paint is not a failure to paint it");
        assert!(ink(&raster) < 0.5, "{cap:?}: a lone m painted something");
    }
}

/// A dot beside a line keeps the line, which is what makes the split a split.
#[test]
fn a_dot_beside_a_line_leaves_the_line_alone() {
    let with_dot = scene(
        &[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
            PathCommand::MoveTo(Point::new(50.0, 70.0)),
            PathCommand::Close,
        ],
        Stroke {
            width: WIDTH,
            cap: LineCap::Round,
            ..Stroke::default()
        },
    );
    let without_dot = scene(
        &[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
        ],
        Stroke {
            width: WIDTH,
            cap: LineCap::Round,
            ..Stroke::default()
        },
    );
    let difference = ink_at(&with_dot, 1.0) - ink_at(&without_dot, 1.0);
    let expected = disc(1.0);
    assert!(
        (difference - expected).abs() < core::f64::consts::PI * f64::from(WIDTH),
        "the dot added {difference:.1} pixels of ink, expected about {expected:.1}"
    );
}

/// A dot's diameter obeys §8.4.3.2's zero-width minimum, exactly as a line's thickness does.
///
/// `0 w` is "the thinnest line that can be rendered at device resolution: 1 device pixel
/// wide", so a `0 w 1 J` dot is one device pixel across however far the page is zoomed —
/// which is a *reciprocal* of the scale in path space, and the reason `split_degenerate`
/// takes the resolved width rather than the field.
#[test]
fn a_zero_width_dot_is_one_device_pixel_across_at_every_scale() {
    let list = scene(
        &[
            PathCommand::MoveTo(Point::new(50.0, 50.0)),
            PathCommand::Close,
        ],
        Stroke {
            width: 0.0,
            cap: LineCap::Round,
            ..Stroke::default()
        },
    );
    for scale in [1.0_f32, 2.0, 3.5] {
        let got = ink_at(&list, scale);
        // A disc one device pixel across covers π/4 ≈ 0.785 of a pixel, and anti-aliasing
        // spreads it over four; either way it is under one pixel's worth and above half.
        assert!(
            (0.4..1.2).contains(&got),
            "at scale {scale}: a zero-width dot deposited {got:.2} pixels of ink"
        );
    }
}

/// A dash of no length is the clause's *other* answer: every cap is painted.
///
/// `[0 6] 0 d` on a 60-unit line dispenses a dash at 0, 6, … 54 — ten of them — and
/// §8.5.3.2 says their caps "shall always be painted, since their orientation is determined
/// by the direction of the underlying path". Under round caps that is ten discs; under
/// projecting square caps, ten squares, where the *subpath* rule above would have painted
/// nothing at all.
#[test]
fn a_zero_length_dash_paints_a_cap_where_a_degenerate_subpath_paints_nothing() {
    let dashes = 10.0;
    let dotted = |cap| {
        scene(
            &[
                PathCommand::MoveTo(Point::new(20.0, 50.0)),
                PathCommand::LineTo(Point::new(80.0, 50.0)),
            ],
            Stroke {
                width: WIDTH,
                cap,
                dash_array: vec![0.0, 6.0],
                ..Stroke::default()
            },
        )
    };

    assert!(
        ink_at(&dotted(LineCap::Butt), 1.0) < 0.5,
        "a butt cap has no extent, so painting it deposits nothing"
    );

    let round = ink_at(&dotted(LineCap::Round), 1.0);
    assert!(
        (round - dashes * disc(1.0)).abs() < dashes * core::f64::consts::PI * f64::from(WIDTH),
        "a dotted line deposited {round:.1} pixels, expected about {:.1}",
        dashes * disc(1.0)
    );

    let square = ink_at(&dotted(LineCap::Square), 1.0);
    let squares = dashes * f64::from(WIDTH * WIDTH);
    assert!(
        (square - squares).abs() < dashes * 4.0 * f64::from(WIDTH),
        "square caps on zero-length dashes deposited {square:.1}, expected about {squares:.1}"
    );
}
