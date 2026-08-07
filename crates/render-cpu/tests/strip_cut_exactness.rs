//! Which shapes survive being cut, which is the measurement the strip planner rests on.
//!
//! `pdf_render::unsplittable_rows` forbids a cut wherever a curve or an oblique edge spans the
//! row, and permits one where only axis-aligned edges do. That is not a deduction from the
//! specification — it is a fact about how `tiny-skia` clips a path against a target that does
//! not contain it, and ADR 0139 established it by filling one shape into a whole pixmap and
//! into two pieces of it and counting the bytes:
//!
//! | shape crossing the cut | bytes differing of 2.9 M | worst |
//! |---|---|---|
//! | axis-aligned rectangle | **0** | 0 |
//! | oblique polygon | 292–528 | 32 |
//! | cubic | 2480–2744 | 64 |
//!
//! **The first row is what makes the parallel rasteriser correct** and is asserted as such.
//! The other two are asserted the other way round: they say the conservative half of the rule
//! is still buying something. If either stops differing — a `tiny-skia` release that clips
//! exactly — the assertion fails, and what it should provoke is a *relaxation* of
//! `Path::oblique_spans`, not a repair here.
//!
//! **A third question was missing until session 382, and the answer to it retired a claim this
//! project made about itself.** The three probes above cut a shape at a row and ask whether the
//! pieces join; none of them asks whether drawing the *same* shape under the *same* matrix into
//! a surface whose first row is elsewhere gives the same pixels. It does not — `tiny-skia` maps
//! a point as `y·sy + ty` in `f32`, and subtracting a whole number of rows from `ty` changes
//! the magnitude that sum rounds at. See `a_surface_that_starts_elsewhere_is_not_invariant`,
//! and ADR 0219 for what follows: a page drawn in strips is the page drawn whole *up to this*,
//! and no arrangement of this crate's arithmetic can close it.
//!
//! Note what the three probes above have in common, because it is why they never asked: every
//! coordinate in them is a dyadic fraction — 111.75, 903.25, 37.25 — and subtracting an integer
//! from one of those is exact. A suite of shapes is a suite of shapes, and so is a suite of
//! *coordinates*.
//!
//! Why these are tests rather than paragraphs: the rules are claims about a dependency's
//! internals, and a dependency's internals change without asking.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "test code over rasters of known size; the casts are a split row index, which \
              is a small integer, in the two forms the two APIs want it"
)]

use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Transform};

/// Target width for every case.
const WIDTH: u32 = 600;

/// Target height for every case, big enough that a cut has room either side of it.
const HEIGHT: u32 = 1200;

/// One closed cubic spanning most of the target's height.
fn cubic() -> tiny_skia::Path {
    let mut path = PathBuilder::new();
    path.move_to(100.0, 100.0);
    path.cubic_to(400.0, 150.0, 500.0, 900.0, 200.0, 950.0);
    path.cubic_to(50.0, 970.0, 30.0, 400.0, 100.0, 100.0);
    path.close();
    path.finish().expect("a closed cubic")
}

/// A quadrilateral with no horizontal or vertical edge.
fn oblique() -> tiny_skia::Path {
    let mut path = PathBuilder::new();
    path.move_to(30.0, 10.0);
    path.line_to(560.0, 90.0);
    path.line_to(540.0, 1190.0);
    path.line_to(10.0, 1100.0);
    path.close();
    path.finish().expect("a quadrilateral")
}

/// An axis-aligned rectangle at fractional coordinates, so its edges are partly covered.
fn axis_aligned() -> tiny_skia::Path {
    let mut path = PathBuilder::new();
    path.move_to(37.25, 111.75);
    path.line_to(480.5, 111.75);
    path.line_to(480.5, 903.25);
    path.line_to(37.25, 903.25);
    path.close();
    path.finish().expect("a rectangle")
}

/// Fills `shape` into a pixmap of `height` rows whose top row is device row `top`.
fn piece(height: u32, top: f32, shape: &tiny_skia::Path) -> Pixmap {
    let mut pixmap = Pixmap::new(WIDTH, height).expect("a target");
    let mut paint = Paint::default();
    paint.set_color_rgba8(20, 60, 200, 255);
    paint.anti_alias = true;
    pixmap.fill_path(
        shape,
        &paint,
        FillRule::Winding,
        Transform::from_translate(0.0, -top),
        None,
    );
    pixmap
}

/// How far a two-piece render is from the whole one: bytes differing, and the worst byte.
fn cut_at(split: u32, shape: &tiny_skia::Path) -> (usize, u8) {
    let whole = piece(HEIGHT, 0.0, shape);
    let above = piece(split, 0.0, shape);
    let below = piece(HEIGHT - split, split as f32, shape);

    let mut joined = Pixmap::new(WIDTH, HEIGHT).expect("a target");
    for (at, part) in [(0_i32, &above), (split as i32, &below)] {
        joined.draw_pixmap(
            0,
            at,
            part.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }

    let pairs = || whole.data().iter().zip(joined.data());
    (
        pairs().filter(|(one, other)| one != other).count(),
        pairs()
            .map(|(one, other)| one.abs_diff(*other))
            .max()
            .unwrap_or(0),
    )
}

/// The rule the planner is allowed to rely on: an axis-aligned edge crossing a cut is exact.
#[test]
fn an_axis_aligned_edge_survives_a_cut() {
    for split in [300, 500, 700] {
        assert_eq!(
            cut_at(split, &axis_aligned()),
            (0, 0),
            "a rectangle cut at row {split} did not come back byte-identical, so \
             `unsplittable_rows` is permitting cuts it must forbid"
        );
    }
}

/// A shape wholly on one side of a cut is exact whatever it is made of, which is the other
/// half of the rule: what costs is *crossing* the cut, not being on the page.
///
/// The cubic's control hull spans device rows 100 to 970, and 100 and 970 themselves are
/// exact — a boundary at row `r` cuts along `y = r`, so the shape is chopped only when
/// `top < r < bottom`. That is where `strips::mark`'s two roundings come from.
#[test]
fn a_shape_on_one_side_of_a_cut_survives_it() {
    for split in [60, 100, 970, 1000] {
        assert_eq!(
            cut_at(split, &cubic()),
            (0, 0),
            "a cubic lying wholly on one side of row {split} was not drawn identically"
        );
    }
}

/// The conservative half, asserted so that it cannot quietly stop being necessary.
#[test]
fn an_oblique_edge_and_a_curve_do_not_survive_a_cut() {
    for split in [300, 500, 700] {
        let (oblique_bytes, oblique_worst) = cut_at(split, &oblique());
        let (curve_bytes, curve_worst) = cut_at(split, &cubic());
        assert!(
            oblique_bytes > 0 && curve_bytes > 0,
            "at row {split} an oblique edge differed in {oblique_bytes} bytes (worst \
             {oblique_worst}) and a curve in {curve_bytes} (worst {curve_worst}); a zero \
             means `tiny-skia` now clips exactly and `Path::oblique_spans` may be relaxed \
             — see ADR 0139"
        );
    }
}

/// A path drawn into a surface whose first row is elsewhere, under the matrix that puts it in
/// the same place, is **not** the same drawing.
///
/// This is the residual ADR 0219 could not remove. `CpuRasterizer` now hands a strip exactly
/// the matrix it hands the whole page with a whole number of rows subtracted from `f` — that
/// much is this crate's own arithmetic and is exact (see `the_offset_is_composed_last` in the
/// crate's unit tests). What is left is the dependency's: a point becomes `y·sy + ty`, and
/// `fl(p + ty)` minus an integer is not `fl(p + ty − integer)` when the two sums fall in
/// different binades. One `ulp` of a device coordinate is one of `tiny-skia`'s sixteen
/// supersamples wherever an edge lands within an `ulp` of a sample row, which is 16 of 255.
///
/// Asserted in the direction it is true — that a difference exists — for the same reason
/// `an_oblique_edge_and_a_curve_do_not_survive_a_cut` is: if a `tiny-skia` release ever makes
/// this invariant, the failure here is the notice that the narrowed claim can be widened again.
#[test]
fn a_surface_that_starts_elsewhere_is_not_invariant() {
    // Coordinates that are *not* dyadic fractions, and a page transform of the shape
    // `TargetSpec::for_page` produces: a scale and a flip about the page's height.
    let (scale, f) = (2.0197_f32, 1683.7834_f32);
    let mut differing = 0_usize;
    let mut worst = 0_u8;

    for shape in 0..40_u32 {
        let y = 3.13 * shape as f32 + 11.7;
        let mut builder = PathBuilder::new();
        builder.move_to(10.3, y);
        builder.line_to(240.7, y + 7.31);
        builder.quad_to(300.1, y + 20.9, 120.4, y + 33.77);
        builder.line_to(11.9, y + 12.5);
        builder.close();
        let path = builder
            .finish()
            .expect("a quadrilateral with one curved side");

        let mut paint = Paint::default();
        paint.set_color_rgba8(20, 60, 200, 255);
        paint.anti_alias = true;
        let tall = 1684;
        let mut whole = Pixmap::new(WIDTH, tall).expect("a target");
        whole.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::from_row(scale, 0.0, 0.0, -scale, 0.0, f),
            None,
        );

        for rows in [1_u32, 8, 64, 236, 512, 887, 1024] {
            let mut strip = Pixmap::new(WIDTH, tall - rows).expect("a target");
            strip.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::from_row(scale, 0.0, 0.0, -scale, 0.0, f - rows as f32),
                None,
            );
            let from = (rows as usize) * (WIDTH as usize) * 4;
            let pairs = || whole.data()[from..].iter().zip(strip.data());
            differing += pairs().filter(|(one, other)| one != other).count();
            worst = worst.max(
                pairs()
                    .map(|(one, other)| one.abs_diff(*other))
                    .max()
                    .unwrap_or(0),
            );
        }
    }

    assert!(
        differing > 0,
        "`tiny-skia` drew 40 shapes at 7 offsets identically at every one of them, so it is \
         now invariant under a whole-row shift of the origin and ADR 0219's narrowing of \
         ADR 0139 can be revisited"
    );
    assert!(
        worst <= 16,
        "a whole-row shift moved a byte by {worst}, which is more than the one supersample \
         (16 of 255) an `ulp` of position can cost — that is a chopped path rather than a \
         rounded one, and it means something other than the origin has moved"
    );
}
