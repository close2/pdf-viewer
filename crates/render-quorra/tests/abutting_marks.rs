//! What two marks that abut leave of the backdrop between them, on **both** backends.
//!
//! A page that states one region as several opaque fills — a map, a cross-section, anything a
//! drawing program exports as filled polygons — asks a rasteriser to paint a boundary pixel from
//! two marks at once. Each covers part of it; neither covers all of it. §10.7.4's scan conversion
//! is aliased, so under it *both* marks paint the whole pixel and the pair covers it with nothing
//! of the backdrop left. Both backends here anti-alias instead — departure (1) of
//! `doc/todo/_scan-conversion.md` — and an anti-aliased mark composites at its own coverage, so the
//! pair leaves a seam.
//!
//! **The seam is what §11.3.7.3 states, not a defect either backend can be accused of.** Result
//! shape is the *union* of the backdrop's and the source's,
//!
//! > This is a generalization of the conventional concept of union for opaque shapes, and it can
//! > be thought of as an "inverted multiplication" -a multiplication with the inputs and outputs
//! > complemented. The result tends toward 1.0: if either input is 1.0, the result is 1.0.
//!
//! and the union of two halves is three quarters, not one. So this file gates the two things the
//! standard does state and declines to gate the artefact itself:
//!
//! - **one statement of a region covers it**, which is §10.7.4's "[t]he area covered by painted
//!   pixels shall always be at least as large as the area of the original shape";
//! - **two statements of the same region leave no more than the union does**, which is
//!   §11.3.7.3's formula read as an upper bound — a backend that composed coverage any other way
//!   would leave more, and this catches it;
//! - and both backends answer alike.
//!
//! A rasteriser that resolved the two marks against one another before compositing would leave
//! **nothing**, and would pass every assertion here unchanged. That is deliberate: the bound is an
//! upper one, so the day this project builds that rasteriser this file does not have to move.
//! `doc/todo/11-shapes-that-still-disappear.md` carries what such a rasteriser costs.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: the page's own coordinates are the small integers below, and an index \
              over a raster this file sized itself cannot overflow"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw three rectangles onto a 40 x 40 page is the \
              failure this file exists to report, and it reports it by name"
)]
#![expect(
    clippy::print_stdout,
    reason = "test code: the seam's size is the measurement, and `--nocapture` is how it is read"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer,
    Size, TargetSpec, Transform,
};
use render_quorra::QuorraRasterizer;

/// The page every scene is drawn on, at one device pixel per user unit.
const PAGE: Size = Size {
    width: 40.0,
    height: 40.0,
};

/// The square the marks are meant to fill, and the backdrop under it.
const LOW: f32 = 10.0;
/// See [`LOW`].
const HIGH: f32 = 30.0;

/// Where the two marks meet, half way across device column 20.
const SEAM: f32 = 20.5;

/// How far a backend's seam may sit from the arithmetic, as a share of the backdrop.
///
/// Measured rather than chosen: the processor reads 0.2510 and the device 0.2471 against the
/// union's 0.25 — one level of 255 either side, which is all an eight-bit raster has to spend —
/// and four quarter-covers read 0.3137 on both against `0.75^4 = 0.3164`. 0.02 is wide enough that
/// neither backend can fail on rounding and narrow enough that a second composition of the same
/// coverage — `1 - 0.75 * 0.75 = 0.4375` left — could not pass.
const TOLERANCE: f32 = 0.02;

/// A rectangle, as a closed subpath.
fn rectangle(x0: f32, x1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, LOW)));
    path.push(PathCommand::LineTo(Point::new(x1, LOW)));
    path.push(PathCommand::LineTo(Point::new(x1, HIGH)));
    path.push(PathCommand::LineTo(Point::new(x0, HIGH)));
    path.push(PathCommand::Close);
    path
}

/// Adds one opaque fill to `list`.
fn fill(list: &mut DisplayList, path: Path, colour: Color) {
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
}

/// A black square with `cuts` interior boundaries, painted white over it.
///
/// `cuts` empty states the square once, which is the control: one mark covers what it covers.
fn scene(cuts: &[f32]) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    fill(&mut list, rectangle(LOW, HIGH), Color::BLACK);
    let mut left = LOW;
    for &cut in cuts {
        fill(&mut list, rectangle(left, cut), Color::WHITE);
        left = cut;
    }
    fill(&mut list, rectangle(left, HIGH), Color::WHITE);
    list
}

/// The share of the backdrop still showing in device column `column`, averaged over the rows the
/// square covers.
///
/// The marks are white and the backdrop black, so a pixel's distance from white *is* the share of
/// the backdrop the marks between them failed to cover.
fn backdrop_showing(raster: &pdf_render::Raster, column: usize) -> f32 {
    let width = raster.width as usize;
    let (first, last) = (LOW as usize + 1, HIGH as usize - 1);
    let mut total = 0.0_f32;
    for row in first..last {
        let at = (row * width + column) * 4;
        total += f32::from(255 - raster.data[at]) / 255.0;
    }
    total / (last - first) as f32
}

/// Both backends' reading of one scene, or nothing where this machine has no adapter.
fn both(list: &DisplayList, column: usize) -> Option<[(&'static str, f32); 2]> {
    let target = TargetSpec::for_page(list, 1.0, 1 << 30).expect("a page of a stated size");
    let mut device = QuorraRasterizer::new_headless().ok()?;
    let processor = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a scene of three rectangles");
    let device = device
        .rasterize(list, target)
        .expect("a scene of three rectangles");
    Some([
        ("processor", backdrop_showing(&processor, column)),
        ("device", backdrop_showing(&device, column)),
    ])
}

/// §10.7.4: one statement of a region covers the region.
///
/// The control the seam is measured against. Without it, a backend that painted nothing at all
/// would pass every bound below.
#[test]
fn one_mark_leaves_none_of_the_backdrop() {
    let Some(drawn) = both(&scene(&[]), SEAM as usize) else {
        println!("skipped: no adapter on this machine");
        return;
    };
    for (backend, showing) in drawn {
        println!("one mark, {backend}: {showing:.4} of the backdrop showing");
        assert!(
            showing < TOLERANCE,
            "§10.7.4: the area covered by painted pixels shall always be at least as large as the \
             area of the original shape, and one rectangle left {showing:.4} of the backdrop \
             showing on the {backend}"
        );
    }
}

/// §11.3.7.3: two marks that abut leave the union of their shapes, and no less.
///
/// Half and half unite to three quarters, so a quarter of the backdrop survives between them.
/// **That is the bound rather than the target**: a backend that leaves less is nearer the clause
/// and passes.
#[test]
fn two_abutting_marks_leave_no_more_than_the_union() {
    let Some(drawn) = both(&scene(&[SEAM]), SEAM as usize) else {
        println!("skipped: no adapter on this machine");
        return;
    };
    // Union(0.5, 0.5) = 0.5 + 0.5 - 0.25, so a quarter of the pixel is never painted.
    let union_leaves = 0.25_f32;
    let mut readings = Vec::new();
    for (backend, showing) in drawn {
        println!("two marks, {backend}: {showing:.4} of the backdrop showing");
        assert!(
            showing < union_leaves + TOLERANCE,
            "§11.3.7.3's union of two half-covered shapes leaves {union_leaves}, and the \
             {backend} left {showing:.4} — coverage was composed some other way"
        );
        readings.push(showing);
    }
    let apart = (readings[0] - readings[1]).abs();
    assert!(
        apart < TOLERANCE,
        "the two backends disagree about the seam by {apart:.4}, {:.4} against {:.4}",
        readings[0],
        readings[1]
    );
}

/// However many marks meet in one pixel, the union never leaves more than `1/e` of it.
///
/// `Union` is an inverted multiplication, so *n* marks whose coverages sum to the whole pixel
/// leave the product of their complements, which is largest when they are equal: `(1 - 1/n)^n`,
/// rising with *n* towards `1/e`. Four quarters therefore leave `0.75^4 = 0.3164`, which is the
/// number this scene draws and is **more** than the pair above leaves — the seam gets worse as a
/// page states its region in more pieces, which is why a drawing of tens of thousands of small
/// polygons is where it is seen. A cross-section of 58 003 filled polygons reads 0.22 to 0.30 at
/// page scale, which is this bound and not a separate fault.
#[test]
fn many_abutting_marks_stay_under_the_union_s_own_limit() {
    // Three cuts inside device column 20, so that four marks each cover a quarter of it.
    let cuts = [20.25, SEAM, 20.75];
    let Some(drawn) = both(&scene(&cuts), SEAM as usize) else {
        println!("skipped: no adapter on this machine");
        return;
    };
    let limit = 1.0_f32 / std::f32::consts::E;
    for (backend, showing) in drawn {
        println!("four marks, {backend}: {showing:.4} of the backdrop showing");
        assert!(
            showing < limit,
            "§11.3.7.3's union cannot leave as much as 1/e = {limit:.4} of a pixel however many \
             marks meet in it, and the {backend} left {showing:.4}"
        );
    }
}

/// §11.6.2: the same two rectangles stated as **one path** leave nothing, on both backends.
///
/// The discriminator between this file's subject and `doc/todo/11` item 7's, and the reason the two
/// are different clauses. Everything above states the region as two `f` operators, which is two
/// graphics objects and is what §11.3.7.3 composites; this states it as two subpaths of one `f`,
/// which is one object — and §11.6.2 is a `shall` about that:
///
/// > Portions of an object shall not be composited with one another, even if they are described in
/// > a way that would seem to cause overlaps (such as a self-intersecting path, combined fill and
/// > stroke of a path, or a shading pattern containing an overlap or fold-over).
///
/// So one scan conversion accumulates the two portions and the seam does not exist here at all.
/// Both backends already did this; what the test pins is that nothing added to make a
/// multi-rectangle path *measurable* starts drawing its portions separately, which would trade
/// item 7's quantum for this file's seam. ADR 0583.
#[test]
fn two_portions_of_one_path_leave_none_of_the_backdrop() {
    let mut list = DisplayList::new(PAGE);
    fill(&mut list, rectangle(LOW, HIGH), Color::BLACK);
    let mut both_halves = rectangle(LOW, SEAM);
    both_halves.extend(rectangle(SEAM, HIGH).commands());
    fill(&mut list, both_halves, Color::WHITE);
    let Some(drawn) = both(&list, SEAM as usize) else {
        println!("skipped: no adapter on this machine");
        return;
    };
    for (backend, showing) in drawn {
        println!("one path, two portions, {backend}: {showing:.4} of the backdrop showing");
        assert!(
            showing < TOLERANCE,
            "§11.6.2: portions of an object shall not be composited with one another, and the \
             {backend} left {showing:.4} of the backdrop showing between two subpaths of one fill"
        );
    }
}
