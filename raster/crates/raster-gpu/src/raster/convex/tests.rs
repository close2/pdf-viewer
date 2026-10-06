//! A convex meet's areas against the general construction they stand in for (ADR 1582): the
//! same bytes, on the shapes `bug1721218_reduced.pdf` meets three thousand times a frame, and
//! the shapes the shortcut must decline.
#![allow(clippy::arithmetic_side_effects)] // test indices and literal coordinates

use super::{Convex, ConvexMeet, Work};
use crate::raster::fill::Rule;
use crate::raster::flatten::{DeviceTransform, Polyline, flatten};
use crate::raster::meet::{RowEdges, Work as MeetWork, area_in_pixel};
use raster_scene::{Point, Segment};

fn polygon(points: &[(f32, f32)]) -> Polyline {
    Polyline {
        points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        closed: true,
        tangents: Vec::new(),
    }
}

/// A step of a small linear congruential sequence, in `0 .. 1`: enough to place shapes
/// everywhere in a pixel without a dependency.
fn next(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    #[expect(clippy::cast_precision_loss)] // 24 bits, exactly an `f32`'s
    let unit = (*state >> 40) as f32 / 16_777_216.0;
    unit
}

/// One of the page's dots as its content stream states it — four cubics and four lines of no
/// length, in page space — under the 1× page transform (`y` flipped at 792), moved by
/// `(dx, dy)`.
fn page_dot(dx: f32, dy: f32) -> Vec<Polyline> {
    let p = |x: f32, y: f32| Point::new(x + dx, y + dy);
    let segments = [
        Segment::MoveTo(p(401.587, 299.47)),
        Segment::CubicTo {
            c1: p(400.954, 298.48),
            c2: p(400.945, 297.498),
            to: p(401.474, 297.114),
        },
        Segment::LineTo(p(401.474, 297.114)),
        Segment::CubicTo {
            c1: p(401.889, 296.813),
            c2: p(402.728, 297.159),
            to: p(403.508, 298.152),
        },
        Segment::LineTo(p(403.508, 298.152)),
        Segment::CubicTo {
            c1: p(404.129, 298.943),
            c2: p(404.167, 300.091),
            to: p(403.665, 300.496),
        },
        Segment::LineTo(p(403.665, 300.496)),
        Segment::CubicTo {
            c1: p(403.538, 300.598),
            c2: p(403.383, 300.648),
            to: p(403.215, 300.648),
        },
        Segment::LineTo(p(403.215, 300.648)),
        Segment::CubicTo {
            c1: p(402.708, 300.648),
            c2: p(402.06, 300.21),
            to: p(401.587, 299.47),
        },
        Segment::Close,
    ];
    let page = DeviceTransform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: -1.0,
        e: 0.0,
        f: 792.0,
    };
    flatten(&segments, page)
}

/// A `count`-gon inscribed in an ellipse of radii `(rx, ry)` turned by `angle`, centred at
/// `(cx, cy)`, wound the other way where `reversed`.
fn ellipse(
    centre: (f32, f32),
    radii: (f32, f32),
    angle: f32,
    count: u16,
    reversed: bool,
) -> Polyline {
    let mut points: Vec<(f32, f32)> = (0..count)
        .map(|k| {
            let a = std::f32::consts::TAU * f32::from(k) / f32::from(count);
            let (x, y) = (radii.0 * a.cos(), radii.1 * a.sin());
            (
                centre.0 + x * angle.cos() - y * angle.sin(),
                centre.1 + x * angle.sin() + y * angle.cos(),
            )
        })
        .collect();
    if reversed {
        points.reverse();
    }
    polygon(&points)
}

/// **A convex meet's areas are the general meet's, byte for byte**: the page's dot met with
/// an ellipse a pixel or two across — the page's own pair, a shading's disc under a dot clip —
/// at five hundred placements, every pixel of the pair's rows asked, and a third convex set
/// added on every other placement, which is what a chain of two links is.
#[test]
fn a_convex_meet_is_the_general_meet_byte_for_byte() {
    let mut state = 11_u64;
    let (mut convex_work, mut general_work) = (Work::default(), MeetWork::default());
    let (mut asked, mut fractional) = (0_usize, 0_usize);
    for k in 0..500 {
        let dot = page_dot(next(&mut state) * 2.0, next(&mut state) * 2.0);
        let centre = (
            402.3 + next(&mut state) * 1.5,
            492.7 + next(&mut state) * 1.5,
        );
        let disc = ellipse(
            centre,
            (0.4 + next(&mut state) * 1.2, 0.3 + next(&mut state)),
            next(&mut state) * 3.0,
            32,
            k % 2 == 1,
        );
        let wedge = polygon(&[
            (390.0, 480.0),
            (420.0, 480.0),
            (402.0 + next(&mut state), 510.0),
        ]);
        let mut sets: Vec<(&Polyline, Convex)> = vec![
            (
                &disc,
                Convex::of(std::slice::from_ref(&disc)).expect("convex"),
            ),
            (&dot[0], Convex::of(&dot).expect("convex")),
        ];
        if k % 2 == 0 {
            sets.push((
                &wedge,
                Convex::of(std::slice::from_ref(&wedge)).expect("convex"),
            ));
        }
        let meet = ConvexMeet::of(&sets);
        let edges: Vec<RowEdges> = sets
            .iter()
            .map(|(polyline, _)| {
                RowEdges::of(
                    std::slice::from_ref(*polyline),
                    Rule::NonZero,
                    485,
                    15,
                    usize::MAX,
                )
                .expect("within the limit")
            })
            .collect();
        let refs: Vec<&RowEdges> = edges.iter().collect();
        for y in 488..498 {
            for x in 398..408 {
                let general = area_in_pixel(x, y, &refs, &mut general_work);
                let measured = meet.area_in_pixel(x, y, &mut convex_work);
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0 ..= 255
                let byte = |area: f64| (area * 255.0).round() as u8;
                assert_eq!(
                    byte(general),
                    byte(measured),
                    "pixel ({x}, {y}) of meet {k}"
                );
                assert!(
                    (general - measured).abs() < 1e-9,
                    "{general} against {measured}"
                );
                asked += 1;
                if general > 0.0 && general < 1.0 {
                    fractional += 1;
                }
            }
        }
    }
    assert_eq!(asked, 50_000);
    assert!(fractional > 2_000, "{fractional} pixels both sets cut");
}

/// **Every shape the shortcut cannot measure as one convex polygon is declined**: two subpaths,
/// a five-pointed star (one way round, twice), a reflex corner, a spike back along an edge, a
/// polygon wound twice, every point on one line, and fewer than three distinct points.
#[test]
fn what_is_not_one_convex_polygon_is_declined() {
    let square = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
    assert!(Convex::of(&[polygon(&square)]).is_some());
    assert!(
        Convex::of(&[polygon(&square), polygon(&square)]).is_none(),
        "two subpaths"
    );
    let star: Vec<(f32, f32)> = (0_u8..5)
        .map(|k| {
            let a = std::f32::consts::TAU * f32::from(k * 2) / 5.0;
            (10.0 * a.cos(), 10.0 * a.sin())
        })
        .collect();
    assert!(Convex::of(&[polygon(&star)]).is_none(), "a star");
    let reflex = [(0.0, 0.0), (4.0, 0.0), (2.0, 1.0), (4.0, 4.0), (0.0, 4.0)];
    assert!(Convex::of(&[polygon(&reflex)]).is_none(), "a reflex corner");
    let spike = [
        (0.0, 0.0),
        (4.0, 0.0),
        (6.0, 0.0),
        (4.0, 0.0),
        (4.0, 4.0),
        (0.0, 4.0),
    ];
    assert!(
        Convex::of(&[polygon(&spike)]).is_none(),
        "back along an edge"
    );
    let twice: Vec<(f32, f32)> = square.iter().chain(square.iter()).copied().collect();
    assert!(Convex::of(&[polygon(&twice)]).is_none(), "round twice");
    assert!(
        Convex::of(&[polygon(&[(0.0, 0.0), (1.0, 1.0), (3.0, 3.0)])]).is_none(),
        "a line"
    );
    assert!(
        Convex::of(&[polygon(&[(0.0, 0.0), (1.0, 1.0), (1.0, 1.0)])]).is_none(),
        "two points"
    );
    let repeated = [
        (0.0, 0.0),
        (4.0, 0.0),
        (4.0, 0.0),
        (4.0, 4.0),
        (0.0, 4.0),
        (0.0, 0.0),
    ];
    assert!(
        Convex::of(&[polygon(&repeated)]).is_some(),
        "repeats are edges of no length"
    );
}

/// **A square's area in a pixel is the closed form**, on the cases that are ties for the byte:
/// a half pixel, a quarter, and a pixel whole and empty.
#[test]
fn a_square_reads_its_closed_form() {
    let square = polygon(&[(0.5, 0.0), (2.0, 0.0), (2.0, 1.5), (0.5, 1.5)]);
    let convex = Convex::of(std::slice::from_ref(&square)).expect("convex");
    let meet = ConvexMeet::of(&[(&square, convex)]);
    let mut work = Work::default();
    let areas: Vec<f64> = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]
        .iter()
        .map(|&(x, y)| meet.area_in_pixel(x, y, &mut work))
        .collect();
    assert_eq!(areas, vec![0.5, 1.0, 0.0, 0.25, 0.5, 0.0]);
}
