//! ISO 32000-2 §10.7.5's first requirement, drawn alike by the oracle and every lane of raster.
//!
//! > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
//! > automatically be adjusted as necessary to produce lines of uniform thickness.
//!
//! The oracle fits a thin rule along the device axes under `/SA` to one whole pixel through pixel
//! centres (`pdf_render::grid_fit`, ADR 2530), and raster fits it at its own encode — the stroke
//! arm and the line drawn as a stroke through `raster::grid_fit`, the line lane's closed form in
//! `line.wgsl`'s vertex stage (ADR 2553). This file holds the two to one picture at eight
//! placements an eighth of a pixel apart, in each of raster's three coverage lanes, for a lone
//! rule — which the translation hands raster as a line — and for a rectangle's outline, which it
//! hands raster as a stroke with corners. The expected value is the clause's: the rule is one
//! whole pixel row wherever it was put, which the oracle is asserted to draw before the lanes are
//! compared with it.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "test code: indices over a raster this file sized itself"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw one stroke onto a 100 x 100 page is the \
              failure this file exists to report, and it reports it by name"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, Paint, Path, PathCommand, Point, Raster, Rasterizer,
    Size, Stroke, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

/// The page each stroke is drawn on.
const PAGE: Size = Size {
    width: 100.0,
    height: 100.0,
};

/// The stated width: 0.6 of a device pixel at scale 1, under [`pdf_render::FIT_BELOW`] and over
/// the half pixel §10.7.5's second requirement promotes, so only the first requirement acts.
const WIDTH: f32 = 0.6;

/// How far a lane's channel may sit from the oracle's, in levels of 255: the lanes' coverage is
/// exact area and the oracle's is analytic, and a pixel centre is a half-integer either way, so a
/// fitted rule's edges fall on pixel boundaries and the two agree to rounding.
const LEVELS: u8 = 2;

/// A stroke of `path` at [`WIDTH`], under `/SA` where `adjust` says so.
fn stroked(path: Path, adjust: bool) -> DisplayList {
    stroked_under(path, adjust, WIDTH, Transform::IDENTITY)
}

/// A stroke of `path` at `width` under `transform`, under `/SA` where `adjust` says so.
fn stroked_under(path: Path, adjust: bool, width: f32, transform: Transform) -> DisplayList {
    let stroke = Stroke {
        width,
        adjust,
        ..Stroke::default()
    };
    stroked_with(path, stroke, transform)
}

/// A stroke of `path` under `stroke` and `transform`, in black.
fn stroked_with(path: Path, stroke: Stroke, transform: Transform) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform,
        stroke,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// A horizontal rule at `y`, from 10 to 90, under `/SA` where `adjust` says so.
fn rule(y: f32, adjust: bool) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, y)));
    path.push(PathCommand::LineTo(Point::new(90.0, y)));
    stroked(path, adjust)
}

/// A horizontal rule at `y` under `/SA`, a sixth of a pixel wide under a transform whose two
/// stretches differ by an eighth of a percent: the promotion makes it one pixel, and the fit
/// declines it on the stretches, as `pdf_render::grid_fit` does, so every lane keeps its
/// coordinates as the oracle does.
fn promoted_under_unequal_stretches(y: f32) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, y / 1.00125)));
    path.push(PathCommand::LineTo(Point::new(90.0, y / 1.00125)));
    stroked_under(path, true, 1.0 / 6.0, Transform::scale(1.0, 1.00125))
}

/// A closed rectangle's outline with its lower left corner at `(at, at)`.
fn outline(at: f32) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(at, at)));
    path.push(PathCommand::LineTo(Point::new(at + 60.0, at)));
    path.push(PathCommand::LineTo(Point::new(at + 60.0, at + 40.0)));
    path.push(PathCommand::LineTo(Point::new(at, at + 40.0)));
    path.push(PathCommand::Close);
    stroked(path, true)
}

/// The rows of columns 20 to 80 that hold any ink, each with its darkest red level.
fn inked_rows(raster: &Raster) -> Vec<(usize, u8)> {
    let width = raster.width as usize;
    (0..raster.height as usize)
        .filter_map(|row| {
            let darkest = (20..80)
                .map(|column| raster.data[(row * width + column) * 4])
                .min()
                .expect("sixty columns");
            (darkest < 255).then_some((row, darkest))
        })
        .collect()
}

/// The largest channel difference between two rasters of one size.
fn worst(a: &Raster, b: &Raster) -> u8 {
    assert_eq!(
        (a.width, a.height),
        (b.width, b.height),
        "one target, one size"
    );
    a.data
        .iter()
        .zip(&b.data)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0)
}

/// Raster's three coverage lanes, or `None` where this machine has no adapter to bring one up.
fn lanes() -> Option<Vec<(&'static str, QuorraRasterizer)>> {
    [
        ("cpu", raster_gpu::Coverage::Cpu),
        ("gpu", raster_gpu::Coverage::Gpu),
        ("compute", raster_gpu::Coverage::Compute),
    ]
    .into_iter()
    .map(|(name, coverage)| {
        QuorraRasterizer::with_options(&raster_gpu::Options {
            coverage,
            ..render_raster::options()
        })
        .ok()
        .map(|lane| (name, lane))
    })
    .collect()
}

/// Every placement the oracle and the lanes are held to: an eighth of a pixel further each time.
fn placements() -> impl Iterator<Item = f32> {
    (0..8u8).map(|eighth| f32::from(eighth) / 8.0)
}

/// The oracle draws the clause: a 0.6-pixel rule under `/SA` is one whole pixel row, fully
/// covered, at each of eight placements — and the rule drawn without `/SA` is not.
#[test]
fn the_oracle_draws_a_fitted_rule_as_one_whole_row_at_every_placement() {
    for offset in placements() {
        let levels = |adjust: bool| {
            let list = rule(50.0 + offset, adjust);
            let target =
                TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a page of a stated size");
            let drawn = render_cpu::CpuRasterizer::new()
                .rasterize(&list, target)
                .expect("one stroke");
            inked_rows(&drawn)
                .iter()
                .map(|&(_, level)| level)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            levels(true),
            [0],
            "a rule placed {offset} past a pixel boundary is one fully covered row"
        );
        assert_ne!(
            levels(false),
            [0],
            "a rule placed {offset} past a pixel boundary without /SA keeps its coverage-weighted \
             width"
        );
    }
}

/// Every lane draws what the oracle draws, a fitted rule and a fitted outline alike, at each of
/// eight placements.
#[test]
fn every_lane_draws_the_oracles_fitted_strokes_at_every_placement() {
    let Some(mut lanes) = lanes() else {
        eprintln!("no adapter on this machine: raster's lanes are not compared");
        return;
    };
    for offset in placements() {
        for (shape, list) in [
            ("rule", rule(50.0 + offset, true)),
            ("outline", outline(20.0 + offset)),
            (
                "promoted rule under unequal stretches",
                promoted_under_unequal_stretches(50.0 + offset),
            ),
        ] {
            let target =
                TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a page of a stated size");
            let oracle = render_cpu::CpuRasterizer::new()
                .rasterize(&list, target)
                .expect("one stroke");
            for (name, lane) in &mut lanes {
                let drawn = lane.rasterize(&list, target).expect("one stroke");
                let off = worst(&oracle, &drawn);
                assert!(
                    off <= LEVELS,
                    "the {name} lane's {shape} placed {offset} past a pixel boundary is {off} \
                     levels from the oracle's"
                );
            }
        }
    }
}
