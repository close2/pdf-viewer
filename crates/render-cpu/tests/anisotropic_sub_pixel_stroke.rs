//! Where a turned rule too thin to measure lands, when the placement is anisotropic.
//!
//! # What the expected values come from
//!
//! ISO 32000-2 §10.7.4 states both halves of what a scan conversion owes a shape:
//!
//! > A shape shall be scan-converted by painting any pixel whose half-open square region
//! > intersects the shape, no matter how small the intersection is.
//!
//! and
//!
//! > The area covered by painted pixels shall always be at least as large as the area of the
//! > original shape.
//!
//! `render_cpu`'s substitution for a stroke the raster cannot measure (ADR 0268) answers the
//! second by restating the rule at a width the raster *can* hold and carrying the width it gave up
//! in the paint's alpha. The first is what bounds how wide that substitute may be: a band of the
//! document's own width lies inside the pixel line the substitute stretches it into, so the
//! substitute may not reach past it.
//!
//! §8.4.3.2 says which placements make that a question at all:
//!
//! > If the CTM specifies scaling by different factors in the horizontal and vertical dimensions,
//! > the thickness of stroked lines in device space shall vary according to their orientation.
//!
//! So under `diag(1/8, 1/200)` a band of one path unit is 1/200 of a device pixel thick where it
//! runs along `x` and 1/8 of one where it runs along `y`, and no single path-space width is one
//! device pixel for both. The scene below is a turned rule under exactly that placement, and the
//! numbers are its own geometry:
//!
//! ```text
//!   direction u        (320, 10000) path units
//!   |u|                10005.12
//!   |T u|              hypot(320/8, 10000/200) = hypot(40, 50) = 64.0312 device pixels
//!   |det T|            1/1600
//!   band thickness     |u| · |det T| / |T u| = 6.25320 / 64.0312 = 0.097659 device pixels
//!   its ink            64.0312 × 0.097659 = 6.2532 device pixels of coverage
//! ```
//!
//! ADR 0945. The witness the corpus states is `issue12295.pdf`, whose 65 859 sub-pixel strokes
//! sit under a placement whose two stretches are a factor of 25 apart.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a rasteriser that refuses this scene should fail loudly, and the \
              arithmetic is over a hundred-unit page and a raster of known size"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything this test requests.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units, and the raster's side at scale 1.
const PAGE: f32 = 100.0;

/// The placement's two stretches, a factor of 25 apart.
const SX: f32 = 0.125;
/// The second of them; `1/200`, so that the reciprocals are whole numbers.
const SY: f32 = 0.005;

/// The turned rule's direction, in the path's own space.
const RUN: (f32, f32) = (320.0, 10000.0);

/// The ink the rule's own geometry states, in device pixels of coverage — the module comment's
/// last line.
const GEOMETRY: f64 = 6.2532;

/// One stroke of unit width along [`RUN`], placed by the anisotropic transform above.
///
/// The rule starts where the transform puts path-space `(0, 0)` and the page is large enough that
/// a substitute twenty-five device pixels wide would still be inside the raster: a mark clipped
/// away at the edge would make the ink read correctly for the wrong reason.
fn turned_rule() -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(RUN.0, RUN.1)));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::new(SX, 0.0, 0.0, SY, 30.0, 25.0),
        stroke: Stroke {
            width: 1.0,
            cap: LineCap::Butt,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Renders the scene at scale 1, where the placement above is the whole device transform.
fn raster() -> Raster {
    let list = turned_rule();
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a stroke is supported")
}

/// Total darkness, in units of one fully black pixel.
fn ink(raster: &Raster) -> f64 {
    let sum: u64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| u64::from(255 - pixel[0]))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "the sum is bounded by four bytes per pixel of a hundred-square raster"
    )]
    let sum = sum as f64;
    sum / 255.0
}

/// The widest run of marked columns any one raster row carries.
fn widest_marked_row(raster: &Raster) -> usize {
    let width = raster.width as usize;
    raster
        .data
        .chunks_exact(4 * width)
        .map(|row| row.chunks_exact(4).filter(|pixel| pixel[0] < 255).count())
        .max()
        .unwrap_or(0)
}

/// The substitute carries the rule's own area, under a placement that stretches the axes unevenly.
///
/// §10.7.4's area sentence, measured. The substitution widens the band and divides the alpha by
/// the same factor, so the ink is the geometry's whatever width is chosen — **except** where the
/// factor drives the alpha under the eight bits it rides in, which is what a substitute stated
/// twenty-five device pixels wide does: 1/200 of an alpha is 1.275 levels and an eight-bit raster
/// holds one, a fifth of the ink gone before any pixel is touched.
#[test]
fn a_turned_rule_under_an_anisotropic_placement_carries_its_own_area() {
    let got = ink(&raster());
    assert!(
        (got - GEOMETRY).abs() < 0.4,
        "{got} device pixels of ink, where the rule's own area is {GEOMETRY}"
    );
}

/// And it lands in the pixels the shape reaches, rather than across the placement's whole stretch.
///
/// §10.7.4's first sentence is about *which* pixels, and it is the half an ink measurement cannot
/// see: widening a band by twenty-five and dividing its alpha by twenty-five conserves the ink
/// exactly while moving most of it a dozen pixels away from the mark. The rule is 0.0977 of a
/// device pixel thick and advances 0.8 of a column per row, so the shape meets at most two columns
/// in any row and a substitute one device pixel wide meets at most three; four is the margin for
/// where the band's edges fall on the grid.
#[test]
fn a_turned_rule_does_not_spread_across_the_placements_wider_axis() {
    let got = widest_marked_row(&raster());
    assert!(
        got <= 4,
        "one raster row carries ink in {got} columns, where the rule crosses at most two"
    );
}
