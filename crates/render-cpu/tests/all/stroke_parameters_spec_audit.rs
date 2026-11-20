//! Audit 2155 of ISO 32000-2 §8.4.3 against the CPU oracle: a zero line width and a dash
//! pattern, each stated in a display list and read back as ink.
//!
//! Each test asserts a control first — a drawing the clause and this rasteriser agree on, so
//! that the sample box is known to see what it is meant to — and then the audited behaviour.
//! Ink is summed over a box of device pixels rather than read at one pixel, because the marks
//! in question are one device pixel wide or finer and a single sample would read anti-aliasing.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::expect_used,
    reason = "small bounded display lists and rasters; refusal to rasterize these strokes is a test failure"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

const PAGE: f32 = 100.0;
const BUDGET: u64 = 1 << 22;

/// One stroke of the polyline `points` under `transform`, drawn at `scale` device pixels a unit.
fn stroked(points: &[(f32, f32)], transform: Transform, stroke: Stroke, scale: f32) -> Drawn {
    let mut path = Path::new();
    for (index, &(x, y)) in points.iter().enumerate() {
        let at = Point::new(x, y);
        path.push(if index == 0 {
            PathCommand::MoveTo(at)
        } else {
            PathCommand::LineTo(at)
        });
    }
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform,
        stroke,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, scale, BUDGET).expect("bounded valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("a polyline stroke is supported");
    Drawn { raster, scale }
}

struct Drawn {
    raster: Raster,
    scale: f32,
}

impl Drawn {
    /// Device pixels' worth of ink inside the page-space box `[x0, x1] × [y0, y1]`.
    fn ink_in(&self, (x0, x1): (f32, f32), (y0, y1): (f32, f32)) -> f32 {
        let columns = (x0 * self.scale) as u32..(x1 * self.scale) as u32;
        let rows = ((PAGE - y1) * self.scale) as u32..((PAGE - y0) * self.scale) as u32;
        let mut total = 0.0;
        for row in rows {
            for column in columns.clone() {
                let at = ((row * self.raster.width + column) * 4) as usize;
                let red = self
                    .raster
                    .data
                    .get(at)
                    .copied()
                    .expect("sample is inside the page");
                total += f32::from(255 - red) / 255.0;
            }
        }
        total
    }
}

fn line(width: f32, join: LineJoin) -> Stroke {
    Stroke {
        width,
        cap: LineCap::Butt,
        join,
        miter_limit: 10.0,
        ..Stroke::default()
    }
}

/// §8.4.3.5: "When the line width is zero, the miter length is zero."
///
/// The two segments meet at about 14° at (90, 50), so §8.4.3.5's ratio is 1 / sin 7° ≈ 8.2,
/// under the limit of 10: a line of any positive width is mitred there, and its mitre reaches
/// 8.2 half-widths beyond the vertex along the outward bisector, which is the box sampled.
/// A zero-width line is drawn one device pixel wide (§8.4.3.2), but its mitre length is zero,
/// so it puts no more ink beyond the vertex than the same line bevelled.
#[test]
fn a_zero_width_line_has_no_mitre_spec_audit() {
    let corner = [(10.0, 50.0), (90.0, 50.0), (10.0, 70.0)];
    let beyond = ((90.75, 95.0), (46.0, 52.0));
    let ink = |width: f32, join: LineJoin| {
        stroked(&corner, Transform::IDENTITY, line(width, join), 1.0).ink_in(beyond.0, beyond.1)
    };

    // Control: a line one device pixel wide by its own width is mitred, and the box sees it.
    let (mitred, bevelled) = (ink(1.0, LineJoin::Miter), ink(1.0, LineJoin::Bevel));
    assert!(
        mitred > bevelled + 1.0,
        "a one-pixel line's mitre reaches into the box: {mitred} against {bevelled} bevelled"
    );

    let (zero_mitred, zero_bevelled) = (ink(0.0, LineJoin::Miter), ink(0.0, LineJoin::Bevel));
    assert!(
        zero_mitred <= zero_bevelled + 0.3,
        "a zero-width line's mitre length is zero, so it inks no more beyond the vertex than a \
         bevel: {zero_mitred} mitred against {zero_bevelled} bevelled"
    );
}

/// §8.4.3.2: "A line width of 0 shall denote the thinnest line that can be rendered at device
/// resolution: 1 device pixel wide."
///
/// The width is stated at device resolution, so the transform's scale does not enter it: under a
/// CTM that stretches x four times and y not at all, a zero-width horizontal line is one device
/// pixel thick exactly as a vertical one is. (The orientation sentence of the same clause is
/// about a width in user space, which a zero width is not.)
#[test]
fn a_zero_width_line_is_one_device_pixel_wide_in_every_direction_spec_audit() {
    let stretched = Transform::scale(4.0, 1.0);
    let zero = line(0.0, LineJoin::Miter);
    // Page x 10..90 at y 50, and page x 60 from y 10 to y 40.
    let horizontal = stroked(&[(2.5, 50.0), (22.5, 50.0)], stretched, zero.clone(), 1.0);
    let vertical = stroked(&[(15.0, 10.0), (15.0, 40.0)], stretched, zero, 1.0);

    // Control: the vertical line, whose width the transform stretches four times, is one pixel.
    let per_row = vertical.ink_in((55.0, 65.0), (15.0, 35.0)) / 20.0;
    assert!(
        (0.75..=1.5).contains(&per_row),
        "the vertical zero-width line is one device pixel wide: {per_row} per row"
    );

    let per_column = horizontal.ink_in((20.0, 80.0), (45.0, 55.0)) / 60.0;
    assert!(
        (0.75..=1.5).contains(&per_column),
        "the horizontal zero-width line is one device pixel wide too: {per_column} per column"
    );
}

/// §8.4.3.6: the dash array's elements "shall be nonnegative and not all zero", and §8.5.3.2: "The
/// line may be painted either solid or with a dash pattern".
///
/// `[0.00001 0.00001] 0` is such an array. Along an 80-unit line it dispenses four million dashes,
/// each a hundred-thousandth of a device pixel, which a rasteriser may draw as the coverage they
/// imply — half the solid line's ink — but the stroke is painted: no clause lets a valid pattern
/// erase it. The control is the same line under a pattern that is fine but not that fine.
#[test]
fn a_dash_pattern_finer_than_a_pixel_still_paints_its_stroke_spec_audit() {
    let along = [(10.0, 50.0), (90.0, 50.0)];
    let box_ = ((10.0, 90.0), (45.0, 55.0));
    let ink = |dash: f32| {
        let stroke = Stroke {
            width: 2.0,
            cap: LineCap::Butt,
            dash_array: if dash > 0.0 {
                vec![dash, dash]
            } else {
                Vec::new()
            },
            dash_phase: 0.0,
            ..Stroke::default()
        };
        stroked(&along, Transform::IDENTITY, stroke, 1.0).ink_in(box_.0, box_.1)
    };
    let solid = ink(0.0);

    let fine = ink(0.01) / solid;
    assert!(
        (0.25..=0.75).contains(&fine),
        "control: a [0.01 0.01] pattern inks about half the solid line: {fine}"
    );

    let finer = ink(0.000_01) / solid;
    assert!(
        (0.25..=0.75).contains(&finer),
        "a [0.00001 0.00001] pattern paints its stroke too, at about half the solid line's ink: \
         {finer}"
    );
}
