//! A meet measured from convex pieces against the general meet over the same inputs (ADR
//! 1582): `bug1721218_reduced.pdf`'s own configuration — a shading's disc under one clip of many
//! dots — and the links a convex meet must hand back to the general one.
#![allow(clippy::arithmetic_side_effects)] // test indices and literal coordinates

use std::sync::Arc;

use super::super::{Mark, both_cut, exact_areas};
use super::{ChainLink, areas};
use crate::raster::{self, CoverageMask, Polyline, RowEdges, RowIndex, Rule};
use raster_scene::Point;

/// A `count`-gon inscribed in the ellipse of radii `radii` centred at `centre`.
fn ellipse(centre: (f32, f32), radii: (f32, f32), count: u16) -> Polyline {
    Polyline::polygon(
        (0..count)
            .map(|k| {
                let a = std::f32::consts::TAU * f32::from(k) / f32::from(count);
                Point::new(centre.0 + radii.0 * a.cos(), centre.1 + radii.1 * a.sin())
            })
            .collect(),
    )
}

/// The device pixels `polyline`'s points reach, rounded out.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // small test coordinates
fn tile_of(polyline: &Polyline) -> (i32, i32, u32, u32) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in &polyline.points {
        (x0, y0, x1, y1) = (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y));
    }
    let (left, top) = (x0.floor() as i32, y0.floor() as i32);
    let (right, bottom) = (x1.ceil() as i32, y1.ceil() as i32);
    (left, top, (right - left) as u32, (bottom - top) as u32)
}

/// What the walk hands both constructions for a disc met with `dots` as one clip: the disc's
/// tile, the pixels both sets cut, the chain's edges over the cut rows, and the chain's
/// flattening with its row index.
fn meet_of(disc: &Polyline, dots: &[Polyline]) -> (CoverageMask, Vec<usize>, RowEdges, ChainLink) {
    let (left, top, width, height) = tile_of(disc);
    let tile = raster::fill_mask(
        std::slice::from_ref(disc),
        Rule::NonZero,
        left,
        top,
        width,
        height,
    );
    let clip = raster::clip_mask(dots, None, Rule::NonZero, (left, top, width, height));
    let cut = both_cut(&tile, &clip);
    let edges = RowEdges::of(dots, Rule::NonZero, top, height, usize::MAX).expect("unbounded");
    let link = ChainLink {
        polylines: Arc::from(dots.to_vec()),
        index: RowIndex::of(dots, usize::MAX).map(Arc::new),
    };
    (tile, cut, edges, link)
}

/// **A disc under a clip of many dots is measured as the general meet measures it**: a grid
/// of 400 dots of 37 points two to three pixels apart, as the page's clip is, and a disc of 33
/// points placed at 300 places among them — every pixel both sets cut answered by both
/// constructions to the same byte.
#[test]
fn a_disc_under_many_dots_meets_as_the_general_meet() {
    let dots: Vec<Polyline> = (0..400_u16)
        .map(|k| {
            let (i, j) = (f32::from(k % 20), f32::from(k / 20));
            ellipse((10.0 + 3.1 * i, 10.0 + 2.7 * j), (1.2, 0.9), 37)
        })
        .collect();
    let mut answered = 0_usize;
    for k in 0..300_u16 {
        let (i, j) = (f32::from(k % 17), f32::from(k / 17));
        let disc = ellipse((10.4 + 3.3 * i, 10.2 + 2.9 * j), (0.8, 0.5), 33);
        let (tile, cut, edges, link) = meet_of(&disc, &dots);
        if cut.is_empty() {
            continue;
        }
        let mark = Mark {
            polylines: std::slice::from_ref(&disc),
            rule: Rule::NonZero,
            edges: None,
        };
        let general = exact_areas(&tile, &cut, std::slice::from_ref(&edges), mark);
        let measured = areas(&tile, &cut, mark, std::slice::from_ref(&link));
        assert!(measured.is_some(), "disc {k} is a convex meet");
        assert_eq!(general, measured, "disc {k}");
        answered += cut.len();
    }
    assert!(answered > 300, "{answered} pixels both sets cut");
}

/// **What is not convex pieces apart is handed back**: two dots whose boxes meet inside the
/// tile, a link of many subpaths with no row index, and a mark that is not convex.
#[test]
fn a_meet_of_other_sets_is_the_general_meets() {
    let disc = ellipse((10.0, 10.0), (0.8, 0.5), 33);
    let mark = Mark {
        polylines: std::slice::from_ref(&disc),
        rule: Rule::NonZero,
        edges: None,
    };
    let touching = [
        ellipse((9.6, 10.0), (0.7, 0.7), 37),
        ellipse((10.6, 10.0), (0.7, 0.7), 37),
    ];
    let (tile, cut, _, link) = meet_of(&disc, &touching);
    assert!(!cut.is_empty());
    assert_eq!(
        areas(&tile, &cut, mark, std::slice::from_ref(&link)),
        None,
        "boxes meet"
    );
    let apart = [
        ellipse((9.6, 10.0), (0.3, 0.3), 37),
        ellipse((10.6, 10.0), (0.3, 0.3), 37),
    ];
    let (tile, cut, _, link) = meet_of(&disc, &apart);
    let unindexed = ChainLink {
        index: None,
        ..link.clone()
    };
    assert!(areas(&tile, &cut, mark, std::slice::from_ref(&link)).is_some());
    assert_eq!(areas(&tile, &cut, mark, &[unindexed]), None, "no row index");
    let notch = Polyline::polygon(vec![
        Point::new(9.2, 9.5),
        Point::new(10.8, 9.5),
        Point::new(10.0, 10.0),
        Point::new(10.8, 10.5),
        Point::new(9.2, 10.5),
    ]);
    let reflex = Mark {
        polylines: std::slice::from_ref(&notch),
        ..mark
    };
    assert_eq!(
        areas(&tile, &cut, reflex, std::slice::from_ref(&link)),
        None,
        "a notch"
    );
}
