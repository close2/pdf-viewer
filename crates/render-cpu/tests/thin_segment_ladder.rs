//! The ladder for a straight stroke whose ink lies in two by two device pixels (ADR 1829): what
//! the processor paints for one, against the area the clauses state.
//!
//! # Where the expected values come from
//!
//! §8.4.3.2 defines the stroke as a set — "stroking a path shall entail painting all points whose
//! perpendicular distance from the path in user space is less than or equal to half the line
//! width" — and §8.4.3.3's Table 53 states its ends: a butt cap squared off at the endpoint, a
//! round cap a filled semicircle of the line's width, a projecting square cap the stroke continued
//! for half the width. Under a placement that scales the axes by `sx` and `sy`, the half-width's disc
//! is an ellipse of semi-axes `h·sx` and `h·sy`. §10.7.4 names the pixel `[i, i+1) × [j, j+1)`, and
//! this tree's anti-aliasing departure paints it at the area of the set inside it.
//!
//! So each expected coverage below is that area, **integrated numerically from the set's own chord
//! lengths** — at each of twenty thousand positions along the segment's axis, the length of the
//! set's cross-section inside the pixel — and not from any formula the code under test uses: the
//! code takes the round cap's area from the unit disc's antiderivative, the ladder from the
//! ellipse's equation sampled. A pixel black on white is then `255 − level`, the level being the
//! area rounded to nearest, since §11.3.6's `Normal` compositing of an opaque black at coverage `c`
//! leaves `1 − c` of the white.
//!
//! Every placement is a multiple of 1/64 of a pixel under scales that are powers of two, so the
//! device coordinates the ladder states are the ones the rasteriser computes, to the bit.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "test code: a rasteriser that refuses a scene should fail loudly, and the arithmetic \
              is over a small page and a raster of known size"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, Color, Command, DisplayList, FillRule, LineCap, Paint, Path, PathCommand,
    Point, Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything this test requests.
const GENEROUS: u64 = 1 << 30;

/// The page's side in units, and the raster's at scale 1.
const PAGE: f32 = 64.0;

/// Steps of the numerical integral across one pixel.
const STEPS: u32 = 20_000;

/// One mark of the ladder, stated in device pixels.
#[derive(Clone, Copy, Debug)]
struct Mark {
    /// Whether the segment runs along device y.
    vertical: bool,
    /// Where it starts and ends along its own axis.
    start: f64,
    /// See [`Mark::start`].
    end: f64,
    /// Where it lies across its axis.
    middle: f64,
    /// The pen's semi-axis along the segment.
    along: f64,
    /// The pen's semi-axis across it.
    across: f64,
    /// Table 53's end.
    cap: LineCap,
}

impl Mark {
    /// The half-length of the set's cross-section at `u`, from §8.4.3.2's set and Table 53's ends.
    fn half_chord(&self, u: f64) -> f64 {
        let inside = u >= self.start && u <= self.end;
        match self.cap {
            LineCap::Butt => {
                if inside {
                    self.across
                } else {
                    0.0
                }
            }
            LineCap::Square => {
                if u >= self.start - self.along && u <= self.end + self.along {
                    self.across
                } else {
                    0.0
                }
            }
            LineCap::Round => {
                if inside {
                    return self.across;
                }
                let past = if u < self.start {
                    self.start - u
                } else {
                    u - self.end
                };
                let t = past / self.along;
                if t >= 1.0 {
                    0.0
                } else {
                    self.across * (1.0 - t * t).sqrt()
                }
            }
        }
    }

    /// The set's area in device pixel `(column, row)`, integrated along the segment's axis.
    fn expected(&self, column: f64, row: f64) -> f64 {
        let (u, v) = if self.vertical {
            (row, column)
        } else {
            (column, row)
        };
        let step = 1.0 / f64::from(STEPS);
        let mut area = 0.0;
        for k in 0..STEPS {
            let at = u + (f64::from(k) + 0.5) * step;
            let half = self.half_chord(at);
            if half > 0.0 {
                let low = (self.middle - half).max(v);
                let high = (self.middle + half).min(v + 1.0);
                area += (high - low).max(0.0) * step;
            }
        }
        area
    }
}

/// The placement: the path's space scaled by `(sx, sy)` into the page, which scale 1 flips into
/// the raster.
fn placement(scale: (f32, f32)) -> Transform {
    Transform::new(scale.0, 0.0, 0.0, scale.1, 0.0, 0.0)
}

/// The path that draws `mark` under `placement(scale)`, in the path's own space.
fn path_of(mark: &Mark, scale: (f32, f32)) -> Path {
    let device = |u: f64| -> Point {
        let (x, y) = if mark.vertical {
            (mark.middle, u)
        } else {
            (u, mark.middle)
        };
        // Device `(x, y)` is page `(x, PAGE − y)`, and the page point is `(sx·px, sy·py)`.
        Point::new(x as f32 / scale.0, (f64::from(PAGE) - y) as f32 / scale.1)
    };
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(device(mark.start)));
    path.push(PathCommand::LineTo(device(mark.end)));
    path
}

fn stroke(width: f32, cap: LineCap) -> Stroke {
    Stroke {
        width,
        cap,
        ..Stroke::default()
    }
}

fn command(
    path: Path,
    scale: (f32, f32),
    stroke: Stroke,
    clip: Option<pdf_render::ClipId>,
) -> Command {
    Command::Stroke {
        path: Arc::new(path),
        transform: placement(scale),
        stroke,
        paint: Paint::Solid(Color::BLACK),
        clip,
        mask: None,
        blend: BlendMode::Normal,
    }
}

fn raster(list: &DisplayList) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("a valid target");
    CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a stroke is supported")
}

/// The red channel of pixel `(x, y)`.
fn red(raster: &Raster, x: u32, y: u32) -> u8 {
    raster.data[((y * raster.width + x) * 4) as usize]
}

/// The level a coverage is painted at, and whether it lies so close to a rounding boundary that
/// the integral's own error could put it on either side.
fn level(coverage: f64) -> (u8, bool) {
    let scaled = coverage.clamp(0.0, 1.0) * 255.0;
    let near_a_tie = (scaled - scaled.floor() - 0.5).abs() < 0.01;
    (scaled.round() as u8, near_a_tie)
}

/// Draws every mark of one rung on a grid of its own cells, and compares every pixel of the raster
/// with the expected area of the one mark whose cell holds it. Returns the number of marks the
/// classification admitted, which is what the ladder exists to exercise.
fn rung(marks: &[Mark], scale: (f32, f32), width: f32, cap: LineCap) -> usize {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let page = placement(scale).then(Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, PAGE));
    let mut admitted = Vec::new();
    for mark in marks {
        let path = path_of(mark, scale);
        let stroke = stroke(width, cap);
        if pdf_render::thin_segment(
            &path,
            &stroke,
            &Paint::Solid(Color::BLACK),
            BlendMode::Normal,
            page,
        )
        .is_none()
        {
            continue;
        }
        admitted.push(*mark);
        list.push(command(path, scale, stroke, None));
    }
    let raster = raster(&list);
    // Each mark's pixels and their areas: its ink box and a pixel beyond it on every side, which
    // must come out empty. A mark none of whose pixels reaches one level is painted at one level
    // in each pixel it has area in — §10.7.4's "no shape ever disappears", taken per shape as ADR
    // 1374 takes it for a rectangle.
    let mut expected = vec![0.0_f64; (raster.width * raster.height) as usize];
    let mut lifted = vec![false; expected.len()];
    for mark in &admitted {
        let reach = mark.along.max(mark.across) + 1.0;
        let (u0, u1) = (mark.start - reach, mark.end + reach);
        let (v0, v1) = (mark.middle - reach, mark.middle + reach);
        let ((x0, x1), (y0, y1)) = if mark.vertical {
            ((v0, v1), (u0, u1))
        } else {
            ((u0, u1), (v0, v1))
        };
        let mut cells = Vec::new();
        for y in (y0.floor().max(0.0) as u32)..(y1.ceil() as u32).min(raster.height) {
            for x in (x0.floor().max(0.0) as u32)..(x1.ceil() as u32).min(raster.width) {
                cells.push((
                    (y * raster.width + x) as usize,
                    mark.expected(f64::from(x), f64::from(y)),
                ));
            }
        }
        // The whole shape's pixels decide the floor, including any off the raster.
        let largest = whole_largest(mark);
        for (at, area) in cells {
            expected[at] += area;
            if largest < 1.0 / 255.0 && area > 0.0 {
                lifted[at] = true;
            }
        }
    }
    for y in 0..raster.height {
        for x in 0..raster.width {
            let at = (y * raster.width + x) as usize;
            let coverage = expected[at];
            let (want, near_a_tie) = if lifted[at] {
                (level(coverage).0.max(1), false)
            } else {
                level(coverage)
            };
            let got = 255 - red(&raster, x, y);
            assert!(
                got == want || (near_a_tie && got.abs_diff(want) <= 1),
                "{cap:?} width {width} under {scale:?}: pixel ({x}, {y}) painted {got} where the set's \
                 area {coverage:.6} is level {want}"
            );
        }
    }
    admitted.len()
}

/// The largest area `mark` puts in any one pixel, over every pixel its ink can meet.
fn whole_largest(mark: &Mark) -> f64 {
    let reach = mark.along.max(mark.across) + 1.0;
    let (u0, u1) = (mark.start - reach, mark.end + reach);
    let (v0, v1) = (mark.middle - reach, mark.middle + reach);
    let mut largest = 0.0_f64;
    for u in (u0.floor() as i64)..(u1.ceil() as i64) {
        for v in (v0.floor() as i64)..(v1.ceil() as i64) {
            let (column, row) = if mark.vertical { (v, u) } else { (u, v) };
            largest = largest.max(mark.expected(column as f64, row as f64));
        }
    }
    largest
}

/// The marks of one rung: a length along the axis, a half-width across it, every placement in a
/// eighth of a pixel's steps on both axes, one per six-pixel cell so that no two share a pixel.
fn marks(vertical: bool, length: f64, along: f64, across: f64, cap: LineCap) -> Vec<Mark> {
    let mut marks = Vec::new();
    for i in 0..9_u8 {
        for j in 0..9_u8 {
            let cell = (4.0 + 6.0 * f64::from(i), 4.0 + 6.0 * f64::from(j));
            let (u0, v) = if vertical {
                (cell.1, cell.0)
            } else {
                (cell.0, cell.1)
            };
            let start = u0 + f64::from(j) / 8.0;
            marks.push(Mark {
                vertical,
                start,
                end: start + length,
                middle: v + f64::from(i) / 8.0 + 1.0 / 64.0,
                along,
                across,
                cap,
            });
        }
    }
    marks
}

#[test]
fn every_pixel_of_a_thin_segment_is_the_area_of_its_set_at_every_size_cap_and_placement() {
    let mut admitted = 0;
    for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
        for vertical in [false, true] {
            // Device widths 0.05 to 1 pixel; the ladder's longest marks and widest pens leave the
            // block, which the classification declines and the count below does not need.
            for width in [0.046_875_f32, 0.093_75, 0.1875, 0.375, 0.5, 0.75, 1.0] {
                for length in [1.0 / 64.0, 0.125, 0.5, 0.875] {
                    let half = f64::from(width) / 2.0;
                    admitted += rung(
                        &marks(vertical, length, half, half, cap),
                        (1.0, 1.0),
                        width,
                        cap,
                    );
                }
            }
        }
    }
    assert!(
        admitted > 9_000,
        "only {admitted} marks were the population"
    );
}

#[test]
fn an_anisotropic_placement_paints_the_ellipse_its_pen_becomes() {
    // §8.4.3.2: "If the CTM specifies scaling by different factors in the horizontal and vertical
    // dimensions, the thickness of stroked lines in device space shall vary according to their
    // orientation." Under (½, ¼) a pen of width 2 is an ellipse of semi-axes ½ and ¼.
    let scale = (0.5, 0.25);
    let mut admitted = 0;
    for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
        for vertical in [false, true] {
            let (along, across) = if vertical { (0.25, 0.5) } else { (0.5, 0.25) };
            for length in [1.0 / 64.0, 0.25] {
                admitted += rung(
                    &marks(vertical, length, along, across, cap),
                    scale,
                    2.0,
                    cap,
                );
            }
        }
    }
    assert!(admitted > 300, "only {admitted} marks were the population");
}

#[test]
fn a_mark_on_the_rasters_edge_paints_the_part_that_is_on_it() {
    let mark = Mark {
        vertical: true,
        start: 10.25,
        end: 10.5,
        middle: 0.125,
        along: 0.375,
        across: 0.375,
        cap: LineCap::Round,
    };
    // Half of it lies left of column 0, and the part inside column 0 is what is painted.
    assert_eq!(rung(&[mark], (1.0, 1.0), 0.75, LineCap::Round), 1);
}

/// A turned segment, in device pixels: its endpoints, and the pen's two semi-axes.
#[derive(Clone, Copy, Debug)]
struct Turned {
    /// The first endpoint.
    from: (f64, f64),
    /// The second endpoint.
    to: (f64, f64),
    /// The pen's semi-axes along device x and y.
    pen: (f64, f64),
    /// Table 53's end.
    cap: LineCap,
}

impl Turned {
    /// The set's boundary as a convex polygon, in device space: §8.4.3.2's band about the segment
    /// in the frame where the pen is the unit disc, a round end as 4 096 points of its semicircle
    /// — whose area falls short of the half disc's by under a millionth of it — and the frame's
    /// two axes multiplied back out.
    fn outline(&self) -> Vec<(f64, f64)> {
        let p0 = (self.from.0 / self.pen.0, self.from.1 / self.pen.1);
        let p1 = (self.to.0 / self.pen.0, self.to.1 / self.pen.1);
        let length = (p1.0 - p0.0).hypot(p1.1 - p0.1);
        let d = ((p1.0 - p0.0) / length, (p1.1 - p0.1) / length);
        let n = (-d.1, d.0);
        let mut points = Vec::new();
        let reach = if self.cap == LineCap::Square {
            1.0
        } else {
            0.0
        };
        let corner = |p: (f64, f64), along: f64, across: f64| {
            (
                p.0 + d.0 * along + n.0 * across,
                p.1 + d.1 * along + n.1 * across,
            )
        };
        if self.cap == LineCap::Round {
            const SIDES: u32 = 4096;
            let base = n.1.atan2(n.0);
            // Around the far end from +n to −n through +d, then the near end from −n to +n.
            for k in 0..=SIDES {
                let angle = base - std::f64::consts::PI * f64::from(k) / f64::from(SIDES);
                points.push((p1.0 + angle.cos(), p1.1 + angle.sin()));
            }
            for k in 0..=SIDES {
                let angle = base + std::f64::consts::PI
                    - std::f64::consts::PI * f64::from(k) / f64::from(SIDES);
                points.push((p0.0 + angle.cos(), p0.1 + angle.sin()));
            }
        } else {
            points.push(corner(p0, -reach, 1.0));
            points.push(corner(p1, reach, 1.0));
            points.push(corner(p1, reach, -1.0));
            points.push(corner(p0, -reach, -1.0));
        }
        points
            .into_iter()
            .map(|(x, y)| (x * self.pen.0, y * self.pen.1))
            .collect()
    }
}

/// The area of a convex polygon inside the unit square at `(column, row)`: the polygon clipped by
/// the square's four sides, and its shoelace.
fn polygon_in_pixel(polygon: &[(f64, f64)], column: f64, row: f64) -> f64 {
    let mut points = polygon.to_vec();
    let sides: [&dyn Fn((f64, f64)) -> f64; 4] = [
        &|p| p.0 - column,
        &|p| column + 1.0 - p.0,
        &|p| p.1 - row,
        &|p| row + 1.0 - p.1,
    ];
    for side in sides {
        let mut kept = Vec::new();
        for index in 0..points.len() {
            let (a, b) = (points[index], points[(index + 1) % points.len()]);
            let (sa, sb) = (side(a), side(b));
            if sa >= 0.0 {
                kept.push(a);
            }
            if (sa >= 0.0) != (sb >= 0.0) {
                let t = sa / (sa - sb);
                kept.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        points = kept;
        if points.is_empty() {
            return 0.0;
        }
    }
    let mut twice = 0.0;
    for index in 0..points.len() {
        let (a, b) = (points[index], points[(index + 1) % points.len()]);
        twice += a.0 * b.1 - b.0 * a.1;
    }
    (twice / 2.0).abs()
}

#[test]
fn a_turned_segment_paints_the_area_of_its_set_in_every_direction() {
    // Under the identity at scale 1 the page's y is flipped, so the path is stated in page space
    // and the expected areas in device space; every coordinate is a multiple of 1/64.
    let mut admitted = 0;
    for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
        for (scale, width) in [
            ((1.0_f32, 1.0_f32), 0.25_f32),
            ((1.0, 1.0), 0.5),
            ((0.5, 0.25), 1.0),
        ] {
            for run in [(0.25, 0.25), (0.375, -0.125), (-0.0625, 0.5), (0.5, 0.0625)] {
                let mut list = DisplayList::new(Size::new(PAGE, PAGE));
                let page = placement(scale).then(Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, PAGE));
                let mut marks = Vec::new();
                for i in 0..9_u8 {
                    for j in 0..9_u8 {
                        let from = (
                            4.0 + 6.0 * f64::from(i) + f64::from(j) / 8.0,
                            4.0 + 6.0 * f64::from(j) + f64::from(i) / 8.0 + 1.0 / 64.0,
                        );
                        let to = (from.0 + run.0, from.1 + run.1);
                        let to_path = |p: (f64, f64)| {
                            Point::new(
                                p.0 as f32 / scale.0,
                                (f64::from(PAGE) - p.1) as f32 / scale.1,
                            )
                        };
                        let mut path = Path::new();
                        path.push(PathCommand::MoveTo(to_path(from)));
                        path.push(PathCommand::LineTo(to_path(to)));
                        let stroke = stroke(width, cap);
                        if pdf_render::thin_segment(
                            &path,
                            &stroke,
                            &Paint::Solid(Color::BLACK),
                            BlendMode::Normal,
                            page,
                        )
                        .is_none()
                        {
                            continue;
                        }
                        let half = f64::from(width) / 2.0;
                        marks.push(Turned {
                            from,
                            to,
                            pen: (half * f64::from(scale.0), half * f64::from(scale.1)),
                            cap,
                        });
                        list.push(command(path, scale, stroke, None));
                    }
                }
                admitted += marks.len();
                let raster = raster(&list);
                let mut expected = vec![0.0_f64; (raster.width * raster.height) as usize];
                for mark in &marks {
                    let outline = mark.outline();
                    let (x0, x1) = (
                        mark.from.0.min(mark.to.0) - 2.0,
                        mark.from.0.max(mark.to.0) + 2.0,
                    );
                    let (y0, y1) = (
                        mark.from.1.min(mark.to.1) - 2.0,
                        mark.from.1.max(mark.to.1) + 2.0,
                    );
                    for y in (y0.floor() as u32)..(y1.ceil() as u32) {
                        for x in (x0.floor() as u32)..(x1.ceil() as u32) {
                            expected[(y * raster.width + x) as usize] +=
                                polygon_in_pixel(&outline, f64::from(x), f64::from(y));
                        }
                    }
                }
                for y in 0..raster.height {
                    for x in 0..raster.width {
                        let coverage = expected[(y * raster.width + x) as usize];
                        let (want, near_a_tie) = level(coverage);
                        let got = 255 - red(&raster, x, y);
                        assert!(
                            got == want || (near_a_tie && got.abs_diff(want) <= 1),
                            "{cap:?} width {width} along {run:?} under {scale:?}: pixel ({x}, {y}) \
                             painted {got} where the set's area {coverage:.6} is level {want}"
                        );
                    }
                }
            }
        }
    }
    assert!(
        admitted > 2_000,
        "only {admitted} marks were the population"
    );
}

/// A rectangle of the page, as a clip.
fn clip_rectangle(
    list: &mut DisplayList,
    (x0, y0, x1, y1): (f32, f32, f32, f32),
) -> pdf_render::ClipId {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    list.add_clip(Clip {
        path,
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        parent: None,
    })
    .expect("a clip")
}

#[test]
fn a_thin_segment_meets_a_clip_as_the_general_construction_does() {
    // The control: the same butt-capped band drawn as one segment, which the classification admits,
    // and as two collinear segments, which it declines because a second segment brings a join — so
    // the second is the general construction's, whose outline is the same rectangle measured exactly
    // (ADR 0476). Under a clip whose edge falls inside the band's pixels both compose `min(M, C)`,
    // and the two rasters are the same function of the same areas. The band is wider than a pixel
    // so that the general construction measures it rather than restating it as a pixel line with
    // its width in the alpha (ADR 0226), which meets a fractional clip by another arithmetic.
    for clip_edge in [20.1_f32, 20.3, 20.5, 20.7, 21.0] {
        let draw = |segments: usize| -> Raster {
            let mut list = DisplayList::new(Size::new(PAGE, PAGE));
            let clip = clip_rectangle(&mut list, (0.0, 0.0, clip_edge, PAGE));
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(19.8, 40.3)));
            if segments == 2 {
                path.push(PathCommand::LineTo(Point::new(20.2, 40.3)));
            }
            path.push(PathCommand::LineTo(Point::new(20.6, 40.3)));
            list.push(command(
                path,
                (1.0, 1.0),
                stroke(1.2, LineCap::Butt),
                Some(clip),
            ));
            raster(&list)
        };
        let (one, two) = (draw(1), draw(2));
        let worst = one
            .data
            .iter()
            .zip(&two.data)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        assert!(
            worst <= 1,
            "clip edge {clip_edge}: the two constructions differ by {worst} levels"
        );
        assert!(
            one.data.iter().any(|&byte| byte < 255),
            "clip edge {clip_edge}: nothing was painted"
        );
    }
}

#[test]
fn a_mark_under_one_level_everywhere_is_lifted_rather_than_lost() {
    // §10.7.4: "This ensures that no shape ever disappears as a result of unfavourable placement
    // relative to the device pixel grid". A round dot of half a thousandth of a pixel's area.
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(30.25, 30.25)));
    path.push(PathCommand::LineTo(Point::new(30.25, 30.265_625)));
    list.push(command(
        path,
        (1.0, 1.0),
        stroke(0.015_625, LineCap::Round),
        None,
    ));
    let raster = raster(&list);
    let painted: Vec<u8> = raster
        .data
        .chunks_exact(4)
        .map(|pixel| 255 - pixel[0])
        .filter(|&level| level > 0)
        .collect();
    assert_eq!(painted, vec![1], "the dot is one pixel at one level");
}
