//! §10.7.4's "no shape ever disappears", on **both** backends.
//!
//! > This ensures that no shape ever disappears as a result of unfavourable placement relative
//! > to the device pixel grid, as might happen with other possible scan conversion rules.
//!
//! `pdf_render::collapsed` gives a subpath with *no* area the thinnest mark the device has (ADR
//! 0154), and that is gated in `render-cpu`. What was measured and ungated until the
//! three-hundred-and-eighty-ninth session is the case one step along: a shape that **has** an
//! area and is thinner than the rasteriser's coverage quantum. `tiny-skia` supersamples four
//! times per pixel row and takes each sub-row's sample at its centre, so a sliver under an eighth
//! of a pixel crossed no sample line and vanished — 0.05 and 0.1 user units of an 80-unit rule
//! gave zero ink at scale 1 — and a stroke under a pixel wide was drawn as a hairline smeared
//! about the path, so one within half a pixel of the raster's edge lost the half of its smear
//! that fell outside.
//!
//! **This file asserted nothing about the processor for that whole time, deliberately**: a gate
//! on the behaviour above would have ratcheted a defect rather than a requirement. Since ADR 0226
//! it asserts the same thing of both backends, which is what makes it a gate on the *clause*
//! rather than on one library — and the number a backend is held to is the shape's own area, not
//! the other backend's answer.
//!
//! `render-quorra/examples/sub_pixel_marks` prints both ladders side by side and is what to run
//! when this fails.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: an index over a raster this file sized itself cannot overflow, and the \
              page's own coordinates are the small integers above"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw one rectangle onto a 100 x 320 page is the \
              failure this file exists to report, and it reports it by name"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer,
    Size, Stroke, TargetSpec, Transform,
};
use render_quorra::QuorraRasterizer;

/// The page each mark is drawn on.
const PAGE: Size = Size {
    width: 100.0,
    height: 320.0,
};

/// Where a mark starts and stops horizontally, which is where the ink is counted.
const LEFT: f32 = 10.0;
/// See [`LEFT`].
const RIGHT: f32 = 90.0;

/// How far a backend's answer may sit from the shape's own area, as a fraction of it.
///
/// Measured rather than chosen. The five sliver thicknesses below come back 0.0510, 0.1020,
/// 0.2000, 0.5020 and 1.0000 on both backends against 0.05, 0.1, 0.2, 0.5 and 1.0, so the worst
/// is 2% — which is one level of 255 on the thinnest of them, and there is nowhere finer for an
/// eight-bit raster to put it. This is 8%: wide enough that a different adapter's rounding cannot
/// fail it, narrow enough that a mark promoted to a whole pixel (twenty times its area, at the
/// thinnest) or lost to a quantum would.
const TOLERANCE: f32 = 0.08;

/// The same, for a rule that is not axis-aligned, and it is looser for a reason that is measured.
///
/// The turned ladder's worst residual after ADR 0268 is **11.3%**, at exactly 45°, and it is not
/// this construction's: `tiny-skia` draws the plain fill of a *one-device-pixel* band at 45° at
/// 177.44 of its own 200 — its scan converter quantises that band's per-row run to quarter pixels
/// — so the substitute inherits it whole and no rule written here could be held tighter. Away from
/// that knife edge the worst is 9.5%, at 0.05 of a pixel where an eight-bit raster has one level
/// to spend.
///
/// 14% is therefore what the measurement allows, and it still catches the defect the test exists
/// for by a factor of two: the hairline it replaced carried `cos θ` of the rule's area, **29.3%
/// short at 45°** and short at every thickness under a pixel rather than only near the quantum.
const TURNED_TOLERANCE: f32 = 0.14;

/// A rule of the given thickness, filled.
fn sliver(thickness: f32) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(LEFT, 160.0)));
    path.push(PathCommand::LineTo(Point::new(RIGHT, 160.0)));
    path.push(PathCommand::LineTo(Point::new(RIGHT, 160.0 + thickness)));
    path.push(PathCommand::LineTo(Point::new(LEFT, 160.0 + thickness)));
    path.push(PathCommand::Close);

    let mut list = DisplayList::new(PAGE);
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The square page a turned rule is drawn on, so that one length fits at every angle.
const TURNED: Size = Size {
    width: 320.0,
    height: 320.0,
};

/// Half the length of every turned rule, in user units.
const REACH: f32 = 100.0;

/// A rule of the given width at `degrees` from the x axis, centred on [`TURNED`].
///
/// Butt caps are `Stroke::default()`'s, so the mark is exactly the parallelogram its width and
/// length state and its area is `2 * REACH * width` — no cap or join adds to it.
fn turned_rule(degrees: f32, width: f32) -> DisplayList {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (cx, cy) = (TURNED.width / 2.0, TURNED.height / 2.0);
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(
        cx - REACH * cos,
        cy - REACH * sin,
    )));
    path.push(PathCommand::LineTo(Point::new(
        cx + REACH * cos,
        cy + REACH * sin,
    )));

    let mut list = DisplayList::new(TURNED);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// A rule of the given width, stroked along `y`.
fn rule(y: f32, width: f32) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(LEFT, y)));
    path.push(PathCommand::LineTo(Point::new(RIGHT, y)));

    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Total ink over the mark's own columns, in units of one fully covered row.
fn ink(raster: &pdf_render::Raster) -> f32 {
    let w = raster.width as usize;
    let (from, to) = (LEFT as usize, RIGHT as usize);
    let mut total = 0.0_f32;
    for row in 0..raster.height as usize {
        for x in from..to {
            let at = (row * w + x) * 4;
            total += f32::from(255 - raster.data[at]) / 255.0;
        }
    }
    total / (to - from) as f32
}

/// Total ink over the whole raster, in units of one fully covered device pixel.
///
/// What a turned rule needs: it lies in no run of rows, so [`ink`]'s normalisation by the mark's
/// own columns says nothing about it, and the quantity to compare with its area is the sum over
/// every pixel.
fn total_ink(raster: &pdf_render::Raster) -> f32 {
    raster
        .data
        .chunks_exact(4)
        .map(|pixel| f32::from(255 - pixel[0]) / 255.0)
        .sum()
}

/// Both backends' ink for one scene, or nothing where this machine has no adapter.
///
/// `measure` is the scene's own reading of its raster: [`ink`] for a mark that lies in one run of
/// rows, [`total_ink`] for one that does not.
fn both(
    list: &DisplayList,
    measure: fn(&pdf_render::Raster) -> f32,
) -> Option<[(&'static str, f32); 2]> {
    let target = TargetSpec::for_page(list, 1.0, 1 << 30).expect("a page of a stated size");
    let mut gpu = QuorraRasterizer::new_headless().ok()?;
    let processor = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a scene of one mark");
    let device = gpu.rasterize(list, target).expect("a scene of one mark");
    Some([
        ("processor", measure(&processor)),
        ("device", measure(&device)),
    ])
}

/// What both backends owe a mark: some ink, and the amount its own area implies.
fn agrees_with_the_area(
    list: &DisplayList,
    measure: fn(&pdf_render::Raster) -> f32,
    tolerance: f32,
    area: f32,
    what: &str,
) {
    let Some(drawn) = both(list, measure) else {
        println!("skipped: no adapter on this machine");
        return;
    };
    for (backend, drawn) in drawn {
        assert!(
            drawn > 0.0,
            "§10.7.4: no shape ever disappears, and {what} did on the {backend}"
        );
        let error = (drawn - area).abs() / area;
        assert!(
            error < tolerance,
            "{what} drew {drawn:.4} of ink on the {backend}, {:.1}% from its own area of {area} \
             — run `cargo run --release -p render-quorra --example sub_pixel_marks` for both \
             backends' ladders",
            error * 100.0
        );
    }
}

/// A filled sliver thinner than either rasteriser's coverage quantum still carries its own area.
#[test]
fn a_sliver_thinner_than_a_quantum_carries_its_area_on_both_backends() {
    for thickness in [0.05_f32, 0.1, 0.2, 0.5, 1.0] {
        agrees_with_the_area(
            &sliver(thickness),
            ink,
            TOLERANCE,
            thickness,
            &format!("a {thickness}-unit sliver"),
        );
    }
}

/// A stroke a tenth of a unit wide carries a tenth of a row wherever it is placed — including
/// within half a pixel of the raster's edges, where the processor's hairline used to lose 45% of
/// it and the device loses 2%.
///
/// The page is 320 units tall and the raster 320 rows, so there is no spare fraction of a row at
/// either end for a mark to spill into: both edges are the hard case. `TargetSpec::for_page`
/// rounds the raster *up* to contain the page (ADR 0064), and on a page whose height is not a
/// whole number only the top edge would be.
#[test]
fn a_sub_pixel_rule_at_the_rasters_edge_keeps_its_ink_on_both_backends() {
    for (where_it_is, y) in [
        ("at the top edge", PAGE.height - 0.05),
        ("at y 300", 300.0),
        ("at y 160", 160.0),
        ("at y 20", 20.0),
        ("at the bottom edge", 0.05),
    ] {
        agrees_with_the_area(
            &rule(y, 0.1),
            ink,
            TOLERANCE,
            0.1,
            &format!("a 0.1-unit rule {where_it_is}"),
        );
    }
}

/// A sub-pixel rule that is **not** axis-aligned carries its own area too, at every angle.
///
/// The residual ADR 0226 left and `doc/todo/11` carried, and it was never the coverage quantum:
/// `tiny-skia`'s hairline lays one pixel down per step along the line's *longer* device axis, so
/// a rule at `θ` from the nearer axis carried `cos θ` of its area — 29.3% short at 45°, at every
/// thickness under a pixel. ADR 0268 draws such a rule one device pixel wide with the width it
/// gave up in the paint's alpha, which conserves the ink at every angle.
///
/// 0 and 90 degrees are in the list deliberately: they are the axis-aligned case the exact
/// substitution takes, so the ladder crosses from one construction to the other without a step.
///
/// **1.0 is in the width list since ADR 0285 and is the rung that used to fail.** `tiny-skia`
/// takes the hairline for every width up to *and including* one device pixel, so a `1 w` rule —
/// which at the page's own scale is most of the line work in every technical drawing — carried
/// 141.42 of its own 200 at 45°, a 29.3% shortfall on a stroke §10.7.4's `shall` covers by name:
/// "[t]his rule applies both to fill operations and to strokes with non-zero width".
#[test]
fn a_turned_sub_pixel_rule_carries_its_area_on_both_backends() {
    for degrees in [0.0_f32, 5.0, 15.0, 30.0, 45.0, 60.0, 90.0] {
        for width in [0.05_f32, 0.1, 0.2, 0.5, 1.0] {
            agrees_with_the_area(
                &turned_rule(degrees, width),
                total_ink,
                TURNED_TOLERANCE,
                2.0 * REACH * width,
                &format!("a {width}-unit rule at {degrees} degrees"),
            );
        }
    }
}

/// A **zero-width** rule is one device pixel wide on both backends, at every angle.
///
/// §8.4.3.2: "A line width of 0 shall denote the thinnest line that can be rendered at device
/// resolution: 1 device pixel wide", and `pdf_render::Stroke::device_width` resolves it there —
/// in the shared crate, so that neither backend decides it alone. §10.7.4 then *permits* a
/// zero-width stroke to "include fewer pixels than the rule implies", which is what `tiny-skia`'s
/// hairline does and what this tree declines: the permission is a `may`, and taking it in one
/// backend only would have left the two disagreeing by 29% on a turned line with no clause to
/// settle it. ADR 0285 argues the choice; this pins it.
///
/// The area compared against is one device pixel times the rule's length, which is what
/// `device_width` promoted the stroke to — not zero, which has no area to be short of.
#[test]
fn a_zero_width_rule_is_one_device_pixel_wide_on_both_backends() {
    for degrees in [0.0_f32, 15.0, 45.0, 90.0] {
        agrees_with_the_area(
            &turned_rule(degrees, 0.0),
            total_ink,
            TURNED_TOLERANCE,
            2.0 * REACH,
            &format!("a 0-width rule at {degrees} degrees"),
        );
    }
}
