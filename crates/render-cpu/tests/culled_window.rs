//! A window onto part of a page is drawn from the marks that reach it, and is the same pixels.
//!
//! ISO 32000-2 §10.7.4 paints a pixel whose square the shape's area meets, so a mark whose
//! conservative bound misses every pixel of a target paints none of them, and
//! `pdf_render::DisplayList::reaching` leaves such marks out of a window's list (ADR 1841). What
//! that must not do is leave out a mark that does reach the window — one whose ink crosses its
//! edge, or reaches into it by a quarter of a pixel.
//!
//! **The oracle is the same window drawn from a list that holds only marks reaching it**, so that
//! nothing is culled from it, against the window drawn from that list with marks all over the rest
//! of the page among them. A window cut out of the whole page drawn at once is not an oracle: a
//! mark of partial coverage crossing a target's edge is converted a level apart from the same mark
//! inside a wider target, culled or not — the seam ADR 1817 §3 measured on tiles, and why a window
//! here is divided into strips of whole rows of its own target and never cut out of another.

#![expect(
    clippy::expect_used,
    reason = "test code: a raster that does not draw is the failure being looked for"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, LineCap, LineJoin, Paint, Path, PathCommand,
    Point, Rasterizer as _, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// The page, in page units.
const PAGE: (f32, f32) = (400.0, 300.0);

/// An opaque colour.
fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color { r, g, b, a: 1.0 }
}

fn polyline(points: &[(f32, f32)], close: bool) -> Arc<Path> {
    let mut path = Path::new();
    for (at, (x, y)) in points.iter().enumerate() {
        let point = Point::new(*x, *y);
        path.push(if at == 0 {
            PathCommand::MoveTo(point)
        } else {
            PathCommand::LineTo(point)
        });
    }
    if close {
        path.push(PathCommand::Close);
    }
    Arc::new(path)
}

fn fill(points: &[(f32, f32)], colour: Color) -> Command {
    Command::Fill {
        path: polyline(points, true),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

fn stroke(points: &[(f32, f32)], width: f32, cap: LineCap, colour: Color) -> Command {
    Command::Stroke {
        path: polyline(points, false),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            cap,
            join: LineJoin::Round,
            // The bound a cull asks reaches the mitre limit times the width past the path, and
            // a round-joined path is drawn the same at any limit: one keeps the bound tight.
            miter_limit: 1.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// The window, at scale two: device x 120 to 420 and y 100 to 300, which is page x 60 to 210
/// and, with y flipped about the page's top, page y 250 down to 150.
fn window() -> TargetSpec {
    let whole = TargetSpec::for_page(&DisplayList::new(Size::new(PAGE.0, PAGE.1)), 2.0, u64::MAX)
        .expect("a page target");
    TargetSpec {
        width: 300,
        height: 200,
        transform: whole.transform.then(Transform::translate(-120.0, -100.0)),
    }
}

/// Whether the grid cell at `column` and `row` has its square and its dot inside the window.
///
/// Worked from the coordinates below: column 5's dot ends 2.75 pixels left of the window's left
/// edge and column 21's square starts 4.5 pixels right of its right edge, row 14's dot ends 3.5
/// pixels below it and row 25's square starts 7 above it — each further than a dot's bound
/// reaches past its ink (half a width, at a mitre limit of one) and the cull's pixel of margin.
fn inside(column: u8, row: u8) -> bool {
    (6..=20).contains(&column) && (15..=24).contains(&row)
}

/// The page, and beside it the same page holding only the marks that reach the window: a grid
/// of small squares and short round-capped dots, two long marks through the window and out of
/// it, and two whose ink reaches a quarter of a pixel into its top row.
fn pages() -> (DisplayList, DisplayList) {
    let mut everywhere = DisplayList::new(Size::new(PAGE.0, PAGE.1));
    let mut reaching = DisplayList::new(Size::new(PAGE.0, PAGE.1));
    let mut both = |command: Command, reaches: bool| {
        if reaches {
            reaching.push(command.clone());
        }
        everywhere.push(command);
    };
    let ink = rgb(0.1, 0.2, 0.6);
    for row in 0..30u8 {
        for column in 0..40u8 {
            let (x, y) = (f32::from(column) * 10.0 + 2.25, f32::from(row) * 10.0 + 3.5);
            let reaches = inside(column, row);
            both(
                fill(
                    &[(x, y), (x + 3.75, y), (x + 3.75, y + 2.5), (x, y + 2.5)],
                    ink,
                ),
                reaches,
            );
            both(
                stroke(
                    &[(x + 5.5, y + 5.25), (x + 5.625, y + 5.25)],
                    0.5,
                    LineCap::Round,
                    Color::BLACK,
                ),
                reaches,
            );
        }
    }
    both(
        stroke(
            &[(10.25, 170.5), (390.75, 170.5)],
            3.0,
            LineCap::Round,
            rgb(0.8, 0.1, 0.1),
        ),
        true,
    );
    both(
        stroke(
            &[(20.5, 20.25), (380.5, 280.75)],
            1.5,
            LineCap::Square,
            rgb(0.1, 0.7, 0.2),
        ),
        true,
    );
    // A square whose ink ends a quarter pixel into the window's top row, and a round cap whose
    // ink reaches as far.
    both(
        fill(
            &[
                (150.0, 260.0),
                (160.0, 260.0),
                (160.0, 249.875),
                (150.0, 249.875),
            ],
            Color::BLACK,
        ),
        true,
    );
    both(
        stroke(
            &[(100.0, 260.0), (100.0, 250.125)],
            0.5,
            LineCap::Round,
            Color::BLACK,
        ),
        true,
    );
    (everywhere, reaching)
}

#[test]
fn a_window_drawn_from_the_marks_that_reach_it_is_the_window_drawn_from_them_all() {
    let (everywhere, reaching) = pages();
    let window = window();
    assert!(
        reaching.reaching(window).is_none(),
        "the oracle's list holds only marks that reach the window, so nothing is culled from it"
    );
    let culled = everywhere
        .reaching(window)
        .expect("a window onto a quarter of the page leaves marks out");
    assert_eq!(
        culled.commands(),
        reaching.commands(),
        "the cull keeps exactly the marks that reach the window, in their order"
    );

    let drawn = |list: &DisplayList| {
        CpuRasterizer::new()
            .with_strips(1)
            .rasterize(list, window)
            .expect("the window draws")
    };
    let (culled_raster, oracle) = (drawn(&everywhere), drawn(&reaching));
    assert_eq!(culled_raster.data, oracle.data);

    // The two marks that reach a quarter of a pixel into the window are in it: what the test
    // would show of a cull that left out too much.
    let ink_at = |x: u32, y: u32| culled_raster.data[(y as usize * 300 + x as usize) * 4];
    assert!(ink_at(185, 0) < 255, "the square above the window");
    assert!(ink_at(80, 0) < 255, "the round cap above the window");
}

#[test]
fn a_window_drawn_in_strips_is_the_window_drawn_whole_where_every_cut_is_exact() {
    // The window under the strip planner as shipped: the culled list may grant cuts the whole
    // list forbade, and each must be one of ADR 0139's exact rows.
    let (everywhere, _) = pages();
    let window = window();
    let one = CpuRasterizer::new()
        .with_strips(1)
        .rasterize(&everywhere, window)
        .expect("the window draws");
    let four = CpuRasterizer::new()
        .with_strips(4)
        .rasterize(&everywhere, window)
        .expect("the window draws");
    assert_eq!(one.data, four.data);
}
