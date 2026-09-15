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
//! `render-raster/examples/sub_pixel_marks` prints both ladders side by side and is what to run
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
    BlendMode, Color, Command, DisplayList, FillRule, LineCap, Paint, Path, PathCommand, Point,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

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
/// **The two thinnest rungs of that ladder were being flattered by a library bias, and the
/// five-hundred-and-eighty-third session removed it** (ADR 0418). `tiny-skia` compiled the
/// low-precision raster pipeline for these draws, whose division by 255 rounds *up* twice per
/// pixel, and the substitute of ADR 0268 carries a rule's given-up width in the paint's **alpha**
/// — so the thinner the rule, the larger a share of its whole ink that upward bias was. It very
/// nearly cancelled the quantum, and at the thinnest rung it cancelled it exactly:
///
/// ```text
///   45°, cpu       0.05     0.10     0.20     0.50     1.00     2.00
///   low precision  -0.2%    -8.5%    -9.9%   -11.3%   -11.3%    -2.7%
///   this clause   -16.8%   -11.3%    -9.9%   -11.3%   -11.3%    -2.7%
/// ```
///
/// Nothing about the substitute moved; what moved is that the ladder now measures it. **A
/// measurement taken through an approximation measures the approximation too** — and a residual
/// that is flat in the width, as the right column now is, is what a scan converter's quantum
/// looks like, where one that shrinks towards zero as the mark thins is not.
///
/// 20% is therefore what the measurement allows — 16.8% worst with room for another adapter's
/// rounding — and it still catches the defect the test exists for: the hairline it replaced
/// carried `cos θ` of the rule's area, **29.3% short at 45°** and short at every thickness under
/// a pixel rather than only near the quantum. That margin is 1.7× rather than the 2× this comment
/// used to claim, and the narrowing is the honest half of the correction.
const TURNED_TOLERANCE: f32 = 0.20;

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

/// The same rule stated twice, in one display list, exactly where it was.
///
/// ISO 32000-2 §8.5.3.3 makes the painted region a set of points, so a region painted twice covers
/// what it covers once and the two draws owe one rule's ink between them.
fn rule_twice(y: f32, width: f32) -> DisplayList {
    let mut list = rule(y, width);
    let repeat = list.commands()[0].clone();
    list.push(repeat);
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
             — run `cargo run --release -p render-raster --example sub_pixel_marks` for both \
             backends' ladders",
            error * 100.0
        );
    }
}

/// A rule of `length` at `degrees` from the x axis, centred on [`TURNED`], with `cap` at each end.
fn capped_rule(degrees: f32, length: f32, width: f32, cap: LineCap) -> DisplayList {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (cx, cy) = (TURNED.width / 2.0, TURNED.height / 2.0);
    let half = length / 2.0;
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(
        cx - half * cos,
        cy - half * sin,
    )));
    path.push(PathCommand::LineTo(Point::new(
        cx + half * cos,
        cy + half * sin,
    )));

    let mut list = DisplayList::new(TURNED);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            cap,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The area a capped rule sweeps, from ISO 32000-2 §8.4.3.3's Table 53.
///
/// The body is `width * length`; both caps that project lie outside it and outside each other, so
/// their areas add. A round cap is "[a] semicircular arc with a diameter equal to the line width",
/// which is `pi w^2 / 8` apiece, and a projecting square cap continues "for a distance equal to
/// half the line width", which is `w^2 / 2` apiece.
fn capped_area(length: f32, width: f32, cap: LineCap) -> f32 {
    let caps = match cap {
        LineCap::Butt => 0.0,
        LineCap::Round => core::f32::consts::PI * width * width / 4.0,
        LineCap::Square => width * width,
    };
    width * length + caps
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

/// A sub-pixel rule's **cap** is drawn, and carries the area §8.4.3.3 states for it.
///
/// The substitute for a rule under the quantum is the same rule one device pixel wide with the
/// width it gave up in the paint's alpha (ADR 0268), and that identity is the swept *body's*: a
/// cap's area goes as the square of the width, so a widened cap at the body's alpha is overstated
/// by the same factor the body's is corrected by. ADR 0268 therefore butt-capped the substitute
/// and left the cap undrawn, which cost the whole of it: a rule as long as it is wide lost 44% of
/// its own area under round caps and 50% under projecting square ones, and one shorter than its
/// own width lost all of it — `0.15` long and `0.5` wide drew **nothing at all**, which is the
/// disappearance §10.7.4 forbids by name.
///
/// The cap is now a mark of its own at the alpha *its* own square implies, so both are exact. The
/// rungs below are measured: the worst residual on them is 3.8%, on a rule one unit long where the
/// substitute's own body is quantised by `tiny-skia`'s quarter-pixel runs, and the shortfall this
/// gate exists to catch is 28% to 33% on the same rungs.
///
/// The last rung is five units wide — far above the quantum — and is the control: the construction
/// must leave an ordinary stroke exactly where the stroker put it.
///
/// **The round-cap rows gate both backends since the five-hundred-and-twelfth session**, which is
/// the row this file had been holding against the processor only. The device used to draw no round
/// cap at all — the near cap was the *inward* half-disc wound against the body, cancelling the far
/// one under the non-zero rule — and raster's `d594566` (taken at `87898c69`) builds the cap fan
/// from the outward direction the stroker already has. Measured before flipping:
/// `sub_pixel_marks` reads the device at −0.1% on the 40 × 5 rule that used to be −8.9%.
#[test]
fn a_sub_pixel_rules_cap_carries_its_own_area() {
    for degrees in [0.0_f32, 30.0] {
        for (length, width) in [(1.0_f32, 0.5_f32), (4.0, 0.5), (40.0, 0.5), (40.0, 5.0)] {
            for cap in [LineCap::Round, LineCap::Square] {
                let what =
                    format!("a {width}-unit rule {length} long at {degrees} degrees, {cap:?}");
                let list = capped_rule(degrees, length, width, cap);
                let area = capped_area(length, width, cap);
                agrees_with_the_area(&list, total_ink, TOLERANCE, area, &what);
            }
        }
    }
}

/// §8.5.3.2's dot is the same mark with no rule under it, and the same quantum swallowed it.
///
/// > If a subpath is degenerate (consists of a single-point closed path or of two or more points
/// > at the same coordinates), the S operator shall paint it only if round line caps have been
/// > specified, producing a filled circle centred at the single point.
///
/// A circle of the line's width is a shape whose area is the *square* of a width the device
/// measures in pixels, so at 0.1 and 0.2 units it was 0.008 and 0.03 of a pixel and the processor
/// drew nothing whatever — while at 0.5 it drew 28% *more* than the circle's own area, because
/// `tiny-skia` rounds a shape that crosses one of its sample lines up to a quarter of a row. Both
/// are answered by stating the mark at one device pixel and carrying its area in the alpha.
///
/// 1.0 is the boundary, where nothing is widened and the rasteriser draws the document's own
/// circle; 2.0 is above it and must be untouched. The rungs **below** 0.2 are
/// [`a_dot_lands_in_one_pixel_at_every_width_and_every_placement`]'s, because down there the
/// quantity a raster can be held to is a level rather than a fraction of the area.
///
/// **Gates both backends since the five-hundred-and-twelfth session.** The device used to flatten
/// a small circle into a polygon inscribed in it — the one-pixel dot read 0.5020 against `pi/4`,
/// the inscribed *square* exactly — and raster's ADR 0044 (taken at `87898c69`) bounds a cubic's
/// flattening by 1/32 of its own device extent, flooring a full turn at 16 chords, on §10.7.2's
/// own NOTE 2. Measured before flipping: the device reads −0.1% to −4.1% on these rungs.
#[test]
fn a_degenerate_subpaths_dot_carries_its_own_area() {
    for width in [0.2_f32, 0.5, 1.0, 2.0] {
        let list = capped_rule(0.0, 0.0, width, LineCap::Round);
        agrees_with_the_area(
            &list,
            total_ink,
            TOLERANCE,
            capped_area(0.0, width, LineCap::Round),
            &format!("a {width}-unit dot"),
        );
    }
}

/// §8.5.3.2's dot lands, at every sub-pixel width and wherever the grid falls under it.
///
/// The rungs above stop at 0.2 of a device pixel, and the defect was under them: both backends
/// drew **nothing at all** at 0.1, 0.05, 0.02 and 0.01, and ADR 0290's own ladder had printed the
/// 0.1 row as `-100.0%` from the day it landed. What the placement column says is which of the two
/// suspects it was — measured, by drawing the same mark half a pixel over:
///
/// ```text
///   width   placed at         backend   total ink   its own area
///    0.10   a pixel's corner  cpu          0.0000         0.0079
///    0.10   a pixel's centre  cpu          0.0078         0.0079
/// ```
///
/// One mark, one width, one alpha, two answers. The widened circle covers `π/4` of a pixel when
/// its centre is at one and `π/16` when its centre is on a corner, so the alpha the raster could
/// hold arrived and was divided by four before it was rounded — which is
/// "unfavourable placement relative to the device pixel grid" in the clause's own words.
/// `pdf_render::point_mark` answers it with the one shape whose coverage is not a fraction: the
/// device pixel §10.7.4 identifies by flooring the mark's own centre.
///
/// **What the mark is held to down here is a level, not a percentage.** The pixel is painted at
/// the mark's own area and an eight-bit raster quantises that, so a mark of 4.5 levels lands as 5;
/// and below a width of `0.0707` the area itself is under one level, where
/// `pdf_render::expressible_coverage` states the least the raster can hold. Both are bounded by
/// one level of the closed form, which is the finest statement there is about an eight-bit raster.
#[test]
fn a_dot_lands_in_one_pixel_at_every_width_and_every_placement() {
    /// One level of an eight-bit raster: see [`pdf_render::expressible_coverage`].
    const ONE_LEVEL: f32 = 1.0 / 255.0;

    for width in [0.2_f32, 0.15, 0.1, 0.05, 0.02, 0.01] {
        // A whole page coordinate at scale 1 is a device pixel's corner; half a unit over is its
        // centre. The clause's guarantee is about the difference between them.
        for (where_, offset) in [("a pixel's corner", 0.0_f32), ("a pixel's centre", 0.5)] {
            let mut list = capped_rule(0.0, 0.0, width, LineCap::Round);
            list = shifted(&list, offset);
            let Some(drawn) = both(&list, total_ink) else {
                println!("skipped: no adapter on this machine");
                return;
            };
            let area = capped_area(0.0, width, LineCap::Round);
            let expected = area.max(ONE_LEVEL);
            for (backend, ink) in drawn {
                assert!(
                    ink > 0.0,
                    "§10.7.4: no shape ever disappears, and a {width}-unit dot at {where_} did \
                     on the {backend}"
                );
                assert!(
                    (ink - expected).abs() <= ONE_LEVEL,
                    "a {width}-unit dot at {where_} drew {ink:.5} of ink on the {backend} where \
                     its own area is {area:.5} — more than one level of 255 from the \
                     {expected:.5} an eight-bit raster can state. Run `cargo run --release -p \
                     render-raster --example sub_pixel_marks` for both backends' ladders"
                );
            }
        }
    }
}

/// The same scene with every stroked path moved `offset` user units along both axes.
///
/// Rebuilding it rather than parameterising [`capped_rule`]: the placement is this test's
/// question alone, and the other scenes are pinned where they are by measurements in their own
/// comments.
fn shifted(list: &DisplayList, offset: f32) -> DisplayList {
    let mut out = DisplayList::new(TURNED);
    for command in list.commands() {
        let Command::Stroke {
            path,
            stroke,
            paint,
            blend,
            ..
        } = command
        else {
            continue;
        };
        let mut moved = Path::new();
        for at in path.commands() {
            let shift = |p: Point| Point::new(p.x + offset, p.y + offset);
            moved.push(match *at {
                PathCommand::MoveTo(p) => PathCommand::MoveTo(shift(p)),
                PathCommand::LineTo(p) => PathCommand::LineTo(shift(p)),
                PathCommand::CurveTo(a, b, p) => PathCommand::CurveTo(shift(a), shift(b), shift(p)),
                PathCommand::Close => PathCommand::Close,
            });
        }
        out.push(Command::Stroke {
            path: Arc::new(moved),
            transform: Transform::IDENTITY,
            stroke: stroke.clone(),
            paint: paint.clone(),
            clip: None,
            mask: None,
            blend: *blend,
        });
    }
    out
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

/// A rule too thin for the *alpha* the substitute rides in still marks the raster.
///
/// Every construction under `pdf_render::sub_pixel` carries a mark's given-up area in the paint's
/// alpha, and an alpha is eight bits. So below `1/255` of a device pixel of thickness the mark was
/// gone again — not through the rasteriser's coverage quantum, which ADR 0226 answered, and not
/// through the hairline's projection, which ADR 0268 answered, but through the substitute's own
/// arithmetic. §10.7.4 forbids the outcome however it is reached:
///
/// > This ensures that no shape ever disappears as a result of unfavourable placement relative to
/// > the device pixel grid, as might happen with other possible scan conversion rules.
///
/// `pdf_render::expressible_coverage` states a positive coverage under one level *at* one level,
/// so what lands is heavier than the geometry — which is the direction the same paragraph asks
/// for, "[t]he area covered by painted pixels shall always be at least as large as the area of the
/// original shape". This therefore checks that ink exists rather than what it comes to; the ladder
/// is `sub_pixel_marks`' seventh section.
///
/// **The processor only, and the device's answer is the reason.** raster takes no substitution
/// here — it strokes the document's own width and its rasteriser's coverage is what runs out — so
/// at 0.002 and 0.001 of a device pixel it draws nothing, measured on that ladder. That is
/// `doc/todo/11`'s, not this test's, and holding the device to a rule it has not been given would
/// only say so twice.
#[test]
fn a_rule_under_one_level_of_alpha_still_marks_the_processors_raster() {
    for degrees in [0.0_f32, 30.0] {
        for width in [0.002_f32, 0.001] {
            let list = turned_rule(degrees, width);
            let Some(drawn) = both(&list, total_ink) else {
                println!("skipped: no adapter on this machine");
                return;
            };
            let (backend, ink) = drawn[0];
            assert_eq!(
                backend, "processor",
                "the processor is the first of the two"
            );
            assert!(
                ink > 0.0,
                "§10.7.4: no shape ever disappears, and a {width}-unit rule at {degrees} \
                 degrees did on the {backend} — run `cargo run --release -p render-raster \
                 --example sub_pixel_marks` for both backends' ladders"
            );
        }
    }
}

/// Where a rule is placed so that it lies **inside one device row** and a band one device pixel
/// wide about it would not.
///
/// The page is 320 units tall and its raster 320 rows, and `TargetSpec::for_page` flips about the
/// page's height (`render_cpu` and the display list do not share a y axis), so a mark at page
/// `y` sits at device `320 - y`. At 160.25 that is device 159.75, a quarter of a pixel from the
/// boundary: a rule up to half a pixel wide lies wholly inside row 159, and the band ADR 0268
/// widened it to would reach a quarter of a pixel into row 160.
const INSIDE_ONE_ROW: f32 = 160.25;

/// A sub-pixel rule the document draws **twice** lands where the document put it, on both
/// backends — the gate on ADR 1102's boundary.
///
/// ISO 32000-2 §11.3.6 composites two objects painted over one another, and over an opaque white
/// backdrop with Normal blend and no constant alpha the result is `1 − (1 − a)(1 − b)` per pixel.
/// So a rule of device width `w` lying inside one device row, stated twice, owes that row
/// `1 − (1 − w)²` — a number that comes from the geometry the document states and from the
/// compositing formula, and from neither backend.
///
/// **This is what a substitution costs above the width at which it is owed, and it is why ADR 1102
/// moved that width.** Restating a mark as a band one device pixel wide with its area in the
/// paint's alpha conserves the ink of *one* mark exactly — widening by a factor and dividing the
/// alpha by it cancel — and that identity holds only while the mark meets nothing. Move the ink to
/// pixels the shape does not cover and every later composite sees a different pair of coverages:
/// at [`INSIDE_ONE_ROW`] a 0.5-unit rule owes 0.7500 and the widened band puts 0.8438 across two
/// rows, 12.5% more ink than the document asked for. `standard_fonts.pdf` draws every table rule
/// twice at 0.57 of a device pixel, and that was 7.8% of the whole page: the marks alone sum to
/// 48 418.38 where the page composites to 31 937.84, and the page now composites to 29 674.33 with
/// the marks' own sum unmoved at 48 530.71.
///
/// Sub-pixel widths only, and that is the test's subject rather than a limitation: above one device
/// pixel there is no substitution to make and the rule no longer lies inside one row.
#[test]
fn a_sub_pixel_rule_stated_twice_composites_where_the_document_put_it() {
    for width in [0.1_f32, 0.2, 0.3, 0.5] {
        let composited = 1.0 - (1.0 - width) * (1.0 - width);
        agrees_with_the_area(
            &rule_twice(INSIDE_ONE_ROW, width),
            ink,
            TOLERANCE,
            composited,
            &format!("a {width}-unit rule stated twice"),
        );
    }
}

/// A sub-pixel rule paints only the pixels its own band meets, on both backends — ISO 32000-2
/// §10.7.4's first sentence read for what it excludes.
///
/// > A shape shall be scan-converted by painting any pixel whose half-open square region
/// > intersects the shape, no matter how small the intersection is.
///
/// The sentence names the pixels a shape affects, and a pixel the shape does not intersect is not
/// among them. That is the half of §10.7.4 a substitution can fail while conserving the ink
/// perfectly: ADR 0268's band is the rule widened to a whole device pixel with the width it gave
/// up carried in the paint's alpha, so its total is the rule's own at every angle and its
/// *footprint* is up to a pixel wider than the rule. That costs nothing while the mark meets
/// nothing and costs whatever it meets otherwise, which is why ADR 1102 stops the substitution at
/// [`pdf_render::unmeasurable_width`] — one level of a device pixel — and draws the shape the
/// document states above it.
///
/// **Calibrated, and the calibration is the old boundary** (trap 13). A 200-unit rule at 5° marks
/// 248 pixels at 0.2 wide and 308 at 0.5, on both backends, and every one of them is a pixel the
/// band meets; with the substitution taken at one whole device pixel instead, the processor marked
/// 400 and 404, of which **144** and **96** lie further from the rule than half its width. At 45°
/// the widened band happens to meet the same 424 pixels the rule does, which is why this is
/// measured at more than one angle.
#[test]
fn a_sub_pixel_rule_marks_only_the_pixels_its_own_band_meets() {
    for degrees in [5.0_f32, 15.0, 45.0] {
        for width in [0.2_f32, 0.5] {
            let list = turned_rule(degrees, width);
            let target =
                TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a page of a stated size");
            let Ok(mut device) = QuorraRasterizer::new_headless() else {
                println!("skipped: no adapter on this machine");
                return;
            };
            let drawn = [
                (
                    "processor",
                    render_cpu::CpuRasterizer::new()
                        .rasterize(&list, target)
                        .expect("a scene of one mark"),
                ),
                (
                    "device",
                    device
                        .rasterize(&list, target)
                        .expect("a scene of one mark"),
                ),
            ];
            for (backend, raster) in drawn {
                let stray = pixels_the_band_does_not_meet(&raster, degrees, width);
                assert_eq!(
                    stray, 0,
                    "a {width}-unit rule at {degrees} degrees painted {stray} pixels its own band                      does not intersect, on the {backend}"
                );
            }
        }
    }
}

/// How many inked pixels of `raster` lie further from [`turned_rule`]'s segment than half its
/// width.
///
/// A pixel `(i, j)` is the square `[i, i+1) × [j, j+1)` (§10.7.4), so the shape meets it when the
/// segment passes within `width / 2` of some point of that square. The raster's y axis is the
/// device's and the rule's is the page's, so the rule is reflected about the page height before
/// the comparison (trap 12a).
fn pixels_the_band_does_not_meet(raster: &pdf_render::Raster, degrees: f32, width: f32) -> usize {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (cx, cy) = (TURNED.width / 2.0, TURNED.height / 2.0);
    // Device space: y grows downward, so the rule's two ends swap sides of the centre line.
    let from = (cx - REACH * cos, TURNED.height - (cy - REACH * sin));
    let to = (cx + REACH * cos, TURNED.height - (cy + REACH * sin));
    let mut stray = 0;
    let stride = raster.width as usize;
    for (index, pixel) in raster.data.chunks_exact(4).enumerate() {
        if pixel[0] == 255 {
            continue;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a raster index under 2^24 on a page this test states the size of"
        )]
        let (i, j) = ((index % stride) as f32, (index / stride) as f32);
        let corners = [(i, j), (i + 1.0, j), (i + 1.0, j + 1.0), (i, j + 1.0)];
        let nearest = (0..4)
            .map(|k| segment_distance((from, to), (corners[k], corners[(k + 1) % 4])))
            .fold(f32::INFINITY, f32::min);
        if nearest > width / 2.0 {
            stray += 1;
        }
    }
    stray
}

/// The distance between two segments: zero where they cross, and otherwise the nearest of the
/// four endpoint-to-segment distances, which is where the minimum of a convex distance over two
/// convex sets is attained.
fn segment_distance(a: ((f32, f32), (f32, f32)), b: ((f32, f32), (f32, f32))) -> f32 {
    let cross = |o: (f32, f32), p: (f32, f32), q: (f32, f32)| {
        (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0)
    };
    let (d1, d2) = (cross(a.0, a.1, b.0), cross(a.0, a.1, b.1));
    let (d3, d4) = (cross(b.0, b.1, a.0), cross(b.0, b.1, a.1));
    if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
        return 0.0;
    }
    [
        distance_to_segment(a.0, b.0, b.1),
        distance_to_segment(a.1, b.0, b.1),
        distance_to_segment(b.0, a.0, a.1),
        distance_to_segment(b.1, a.0, a.1),
    ]
    .into_iter()
    .fold(f32::INFINITY, f32::min)
}

/// The distance from `point` to the segment `from`–`to`.
fn distance_to_segment(point: (f32, f32), from: (f32, f32), to: (f32, f32)) -> f32 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared > 0.0 {
        (((point.0 - from.0) * dx + (point.1 - from.1) * dy) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (point.0 - (from.0 + t * dx)).hypot(point.1 - (from.1 + t * dy))
}
