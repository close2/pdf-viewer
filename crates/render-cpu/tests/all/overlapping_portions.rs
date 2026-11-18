//! What this backend paints where portions of one path overlap — ISO 32000-2 §10.7.4 with
//! §8.5.3.3's two rules and §11.6.2.
//!
//! # What the expected values come from
//!
//! §10.7.4 applies its rules to a shape whose inside has already been decided:
//!
//! > At this level, curves have been flattened to sequences of straight lines, and all
//! > "insideness" computations have been performed.
//!
//! and §11.6.2 forbids reading two portions of one path as two marks:
//!
//! > Portions of an object shall not be composited with one another, even if they are described in
//! > a way that would seem to cause overlaps (such as a self-intersecting path, combined fill and
//! > stroke of a path, or a shading pattern containing an overlap or fold-over).
//!
//! So a pixel is covered by the area of the **set** the rule declares inside. For two axis-aligned
//! squares wound the same way, §8.5.3.3.2's non-zero rule makes that set their union and
//! §8.5.3.3.3's even-odd rule their union less the overlap, and each is a closed form by
//! inclusion–exclusion over §10.7.4's half-open pixel: `|A ∩ p| + |B ∩ p| − k·|A ∩ B ∩ p|`, with
//! `k` one for the union and two for the difference, and every term the product of two
//! one-dimensional overlaps. No renderer enters an expected value.
//!
//! Under this tree's anti-aliasing departure — §10.7.1's NOTE — that area is the coverage the pixel
//! is painted at. What the placements are chosen to separate is the winding **integral**, which
//! reads `|A ∩ p| + |B ∩ p|` under both rules and would paint the corner pixels heavy.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code: a rasteriser that refuses one of these scenes should fail loudly, and the \
              arithmetic is over a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything here.
const GENEROUS: u64 = 1 << 30;

/// The page every scene is drawn on, at one device pixel per user unit.
const PAGE: Size = Size {
    width: 8.0,
    height: 8.0,
};

/// One level of an eight-bit raster, plus the arithmetic's residue: the coverage is rounded once
/// and composited once.
const TOLERANCE: f32 = 1.01 / 255.0;

/// A device-space rectangle `[left, right) × [top, bottom)`.
#[derive(Clone, Copy)]
struct Square {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Square {
    /// The rectangle as a closed subpath in page space, wound clockwise on the device.
    fn subpath(self, path: &mut Path) {
        // The page's y runs up and the device's down, over a page of the raster's own height.
        let page_y = |device: f32| PAGE.height - device;
        path.push(PathCommand::MoveTo(Point::new(self.left, page_y(self.top))));
        path.push(PathCommand::LineTo(Point::new(
            self.right,
            page_y(self.top),
        )));
        path.push(PathCommand::LineTo(Point::new(
            self.right,
            page_y(self.bottom),
        )));
        path.push(PathCommand::LineTo(Point::new(
            self.left,
            page_y(self.bottom),
        )));
        path.push(PathCommand::Close);
    }

    /// The area of this rectangle inside device pixel `(x, y)`: the product of two overlaps.
    fn area_in(self, x: f32, y: f32) -> f32 {
        let along = |low: f32, high: f32, at: f32| (high.min(at + 1.0) - low.max(at)).max(0.0);
        along(self.left, self.right, x) * along(self.top, self.bottom, y)
    }

    /// The rectangle both share, or an empty one.
    fn meet(self, other: Self) -> Self {
        Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }
}

/// Two squares overlapping at a corner: the first reaches `a` into column 3 and row 3, the
/// second starts `b` into column 2 and row 2.
fn corner_pair(a: f32, b: f32) -> [Square; 2] {
    [
        Square {
            left: 1.0,
            top: 1.0,
            right: 3.0 + a,
            bottom: 3.0 + a,
        },
        Square {
            left: 2.0 + b,
            top: 2.0 + b,
            right: 6.0,
            bottom: 6.0,
        },
    ]
}

/// Both squares as one path.
fn path_of(squares: [Square; 2]) -> Path {
    let mut path = Path::new();
    for square in squares {
        square.subpath(&mut path);
    }
    path
}

/// The clause's area at pixel `(x, y)` under `rule`.
fn clauses_area(squares: [Square; 2], rule: FillRule, x: u32, y: u32) -> f32 {
    let (x, y) = (x as f32, y as f32);
    let [first, second] = squares;
    let shared = first.meet(second).area_in(x, y);
    let overlap_counts = match rule {
        FillRule::NonZero => 1.0,
        FillRule::EvenOdd => 2.0,
    };
    first.area_in(x, y) + second.area_in(x, y) - overlap_counts * shared
}

/// The coverage a pixel carries, on a black-on-white page where darkness is coverage.
fn coverage(raster: &Raster, x: u32, y: u32) -> f32 {
    let at = ((y * raster.width + x) as usize) * 4;
    f32::from(255 - raster.data[at]) / 255.0
}

/// Draws `list` at one device pixel per unit and checks every pixel against the clause's area.
fn assert_every_pixel(what: &str, list: &DisplayList, squares: [Square; 2], rule: FillRule) {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the scene is supported");
    for y in 0..raster.height {
        for x in 0..raster.width {
            let expected = clauses_area(squares, rule, x, y);
            let got = coverage(&raster, x, y);
            assert!(
                (got - expected).abs() <= TOLERANCE,
                "{what}, {rule:?}: pixel ({x}, {y}) carries {got}, the set's area is {expected}"
            );
        }
    }
}

/// A black fill of both squares as one path.
fn filled(squares: [Square; 2], rule: FillRule) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    list.push(Command::Fill {
        path: Arc::new(path_of(squares)),
        transform: Transform::IDENTITY,
        fill_rule: rule,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The whole page filled black through both squares as a clipping path.
fn clipped(squares: [Square; 2], rule: FillRule) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    let clip = list
        .add_clip(Clip {
            path: path_of(squares),
            transform: Transform::IDENTITY,
            fill_rule: rule,
            parent: None,
        })
        .expect("a clip");
    let mut page = Path::new();
    Square {
        left: -4.0,
        top: -4.0,
        right: 12.0,
        bottom: 12.0,
    }
    .subpath(&mut page);
    list.push(Command::Fill {
        path: Arc::new(page),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: Some(clip),
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The placements: the brief's half pixel, and two off the quarter lattice a supersampled
/// converter measures to — where only the set's own area lands within a level.
const PLACEMENTS: [(f32, f32); 3] = [(0.5, 0.5), (0.3, 0.6), (0.7, 0.15)];

/// Filled under the non-zero rule, the two squares are their union.
#[test]
fn two_overlapping_squares_fill_their_union_under_the_non_zero_rule() {
    for (a, b) in PLACEMENTS {
        let squares = corner_pair(a, b);
        assert_every_pixel(
            &format!("filled at a = {a}, b = {b}"),
            &filled(squares, FillRule::NonZero),
            squares,
            FillRule::NonZero,
        );
    }
}

/// Filled under the even-odd rule, the overlap is outside.
#[test]
fn two_overlapping_squares_fill_their_union_less_the_overlap_under_the_even_odd_rule() {
    for (a, b) in PLACEMENTS {
        let squares = corner_pair(a, b);
        assert_every_pixel(
            &format!("filled at a = {a}, b = {b}"),
            &filled(squares, FillRule::EvenOdd),
            squares,
            FillRule::EvenOdd,
        );
    }
}

/// §10.7.4's clipping paragraph — "the clipping region consists of the set of pixels that would
/// be included by a fill operation" — so the region is measured by the same set, under each rule.
#[test]
fn the_same_path_as_a_clipping_region_admits_the_same_set() {
    for (a, b) in PLACEMENTS {
        let squares = corner_pair(a, b);
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            assert_every_pixel(
                &format!("clipped at a = {a}, b = {b}"),
                &clipped(squares, rule),
                squares,
                rule,
            );
        }
    }
}
