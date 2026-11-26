//! ISO 32000-2 §10.7.5's first requirement: a stroke's width and coordinates fitted to the
//! device grid when the graphics state's stroke adjustment parameter enables it.
//!
//! > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
//! > automatically be adjusted as necessary to produce lines of uniform thickness. The
//! > thickness shall be as near as possible to the requested line width -no more than half a
//! > pixel different.
//!
//! The non-uniformity the clause removes is the one its first paragraph names: "\[a] line of a
//! given width can intersect with different numbers of device pixels, depending on where it is
//! positioned". On an anti-aliasing device that shows as a rule drawn across a partly covered row
//! either side of its whole ones in one place and as whole rows alone in another. The fit removes
//! it for the one shape it is defined on, a stroke whose every segment lies along a device axis:
//! the width becomes the whole number of device pixels nearest the requested one, and never fewer
//! than one ([`fitted_width`]), and each vertex moves, across the segments it ends, to the
//! nearest place where both edges of a stroke that wide lie on pixel boundaries
//! ([`fitted_centre`]) — a pixel centre for an odd width, a pixel boundary for an even one — so
//! the line covers the same number of whole rows or columns wherever the document placed it. A
//! free end keeps where the document ended it, because moving a rule along itself changes nothing
//! the clause names.
//!
//! **What is fitted is a construction of this tree's, and its bounds are the clause's.**
//!
//! - *Every width*, because the clause's first sentence binds every stroke the parameter
//!   enables, and the nearest whole number of pixels is never more than half a pixel from the
//!   requested width — the clause's own bound. A requested width halfway between two whole
//!   numbers takes the smaller, which is as near as the larger and moves less ink (ADR 3729).
//! - *Every segment along a device axis*, because a diagonal or a curve has no pair of edges to
//!   put on pixel boundaries; such a stroke keeps its own coordinates and its coverage-weighted
//!   thickness, which is the requested width at every placement (ADR 0848).
//! - *A transform that stretches both axes alike*, because one stroke width is one device width
//!   only then; §8.4.3.2 says a stroke's device thickness otherwise varies with its orientation.
//! - *A move of at most half a device pixel per coordinate*: the places a fitted stroke's centre
//!   may stand are a whole pixel apart, so the nearest is never further away.
//!
//! ADR 2530 is the construction and its measurement over the pdf.js documents that enable the
//! parameter; ADR 2553 draws it by default in this backend and every lane of `raster` at once,
//! as the owner chose (`doc/questions/A714`); ADR 3729 carries it to every width.

use crate::geom::{Path, PathCommand, Point, Transform};
use crate::paint::{Stroke, thinnest_line};

/// The whole number of device pixels a stroke `device` pixels wide is fitted to: the nearest,
/// the smaller of two equally near, and never fewer than one.
///
/// Within the clause's half a pixel of every requested width of half a pixel or more; a width
/// under half a pixel is one pixel by the clause's second requirement already
/// ([`Stroke::device_width`]).
#[must_use]
pub fn fitted_width(device: f32) -> f32 {
    (device - 0.5).ceil().max(1.0)
}

/// The device coordinate across a segment at which a fitted stroke `pixels` wide, whose centre
/// the document put at `v`, has both edges on pixel boundaries: the nearest such place.
///
/// For one pixel that is the pixel centre `⌊v⌋ + 0.5`; for two, the nearest pixel boundary.
/// `pixels` is a whole number ([`fitted_width`]), so the places are a whole pixel apart and the
/// move is at most half a pixel.
#[must_use]
pub fn fitted_centre(v: f32, pixels: f32) -> f32 {
    let half = 0.5 * pixels;
    (v - half + 0.5).floor() + half
}

/// How far, in device pixels, a segment's two ends may differ across an axis and still count
/// as lying along it.
///
/// A sixty-fourth of a pixel is far under anything a reader could see and far over an `f32`
/// page transform's rounding, so a rule the document drew along an axis counts as one and a
/// line the document drew at an angle does not.
const ALONG_AN_AXIS: f32 = 1.0 / 64.0;

/// How far apart a transform's two singular values may be, as a fraction of the larger, for one
/// stroke width to be one device width.
const ALIKE: f32 = 1.0 / 1024.0;

/// Whether `t` stretches every direction alike to within [`ALIKE`]: a similarity, under which
/// one width stated in a path's own space is one width in the device.
///
/// Shared with [`Stroke::strokes_in_device_space`], which asks the same question of the thinnest
/// line.
pub(crate) fn stretches_alike(t: Transform) -> bool {
    let (widest, narrowest) = (t.max_stretch(), t.min_stretch());
    widest - narrowest <= ALIKE * widest
}

/// A stroke fitted to the device grid: the path to stroke in place of the command's and the
/// stroke parameters to stroke it with.
#[derive(Debug, Clone)]
pub struct GridFit {
    /// The command's path with every vertex moved across the segments it ends to the nearest
    /// place where the fitted stroke's edges lie on pixel boundaries, in the path's own space.
    pub path: Path,
    /// The command's stroke with its width a whole number of device pixels ([`fitted_width`]),
    /// in the path's own space.
    pub stroke: Stroke,
}

/// The stroke §10.7.5's first requirement asks for in place of `path` under `stroke`, or `None`
/// where it asks for none or the stroke is already on the grid.
///
/// `to_device` maps the path to device pixels. `None` for a stroke whose `/SA` is off, one whose
/// device width is not a finite number, a path with a curve or a segment off a device axis, a
/// transform that stretches the axes differently or has no inverse, and a stroke the fit would
/// leave exactly where it is — so a caller strokes the command's own path whenever this declines.
#[must_use]
pub fn grid_fit(path: &Path, stroke: &Stroke, to_device: Transform) -> Option<GridFit> {
    if !stroke.adjust {
        return None;
    }
    let one_pixel = thinnest_line(to_device)?;
    if !stretches_alike(to_device) {
        return None;
    }
    // `device_width` already holds §8.4.3.2's zero width and §10.7.5's half-pixel promotion,
    // so a stroke it makes one pixel is fitted for its position too.
    let device = stroke.device_width(to_device) / one_pixel;
    // A width that is not a finite number has no whole number of pixels nearest it.
    if !device.is_finite() {
        return None;
    }
    let pixels = fitted_width(device);
    let back = to_device.invert()?;
    let fitted = snapped(path, pixels, to_device, back)?;
    let width = pixels * one_pixel;
    // A fitted centre is a whole or a half-integer, which holds exactly in `f32` at every page
    // size, so a vertex already there comes back from the round trip within this of where it was.
    let unchanged = fitted
        .commands()
        .iter()
        .zip(path.commands())
        .all(|(a, b)| same_command(*a, *b, one_pixel / 1024.0));
    if unchanged && (stroke.width - width).abs() <= one_pixel / 1024.0 {
        return None;
    }
    Some(GridFit {
        path: fitted,
        stroke: Stroke {
            width,
            ..stroke.clone()
        },
    })
}

/// `path` with each vertex moved across the segments it ends to the nearest place a stroke
/// `pixels` wide has both edges on pixel boundaries ([`fitted_centre`]), or `None` where a
/// segment of it, closing segments included, lies off the device axes.
///
/// Only the coordinate *across* a segment moves: a vertex that ends a horizontal segment has its
/// device `y` moved and one that ends a vertical segment its `x`, so a corner
/// moves in both and the free end of a rule keeps where the document ended it. The device point
/// is mapped there by `to_device` and back by `back`.
fn snapped(path: &Path, pixels: f32, to_device: Transform, back: Transform) -> Option<Path> {
    let commands = path.commands();
    let device: Vec<Option<Point>> = commands
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some(to_device.apply(p)),
            PathCommand::CurveTo(..) | PathCommand::Close => None,
        })
        .collect();
    // Which of each vertex's two device coordinates a segment ending at it lies across.
    let mut across = vec![(false, false); commands.len()];
    let (mut from, mut start) = (None::<usize>, None::<usize>);
    for (index, command) in commands.iter().enumerate() {
        let (a, b) = match command {
            PathCommand::MoveTo(_) => {
                (from, start) = (Some(index), Some(index));
                continue;
            }
            PathCommand::LineTo(_) => (from.replace(index)?, index),
            PathCommand::Close => {
                let (a, b) = (from?, start?);
                from = Some(b);
                (a, b)
            }
            PathCommand::CurveTo(..) => return None,
        };
        let (p, q) = (device[a]?, device[b]?);
        let (horizontal, vertical) = (
            (q.y - p.y).abs() <= ALONG_AN_AXIS,
            (q.x - p.x).abs() <= ALONG_AN_AXIS,
        );
        match (horizontal, vertical) {
            // A segment of no length states no direction and moves nothing.
            (true, true) => {}
            (true, false) => (across[a].1, across[b].1) = (true, true),
            (false, true) => (across[a].0, across[b].0) = (true, true),
            (false, false) => return None,
        }
    }
    let centre = |v: f32| fitted_centre(v, pixels);
    let mut fitted = Path::new();
    for (index, command) in commands.iter().enumerate() {
        let moved = |p: Point| {
            let d = device[index].unwrap_or(p);
            let (x, y) = across[index];
            back.apply(Point {
                x: if x { centre(d.x) } else { d.x },
                y: if y { centre(d.y) } else { d.y },
            })
        };
        fitted.push(match *command {
            PathCommand::MoveTo(p) => PathCommand::MoveTo(moved(p)),
            PathCommand::LineTo(p) => PathCommand::LineTo(moved(p)),
            curve @ PathCommand::CurveTo(..) => curve,
            PathCommand::Close => PathCommand::Close,
        });
    }
    Some(fitted)
}

/// Whether two path commands are the same step with points within `within` of each other.
fn same_command(a: PathCommand, b: PathCommand, within: f32) -> bool {
    let near = |p: Point, q: Point| (p.x - q.x).abs() <= within && (p.y - q.y).abs() <= within;
    match (a, b) {
        (PathCommand::MoveTo(p), PathCommand::MoveTo(q))
        | (PathCommand::LineTo(p), PathCommand::LineTo(q)) => near(p, q),
        (PathCommand::CurveTo(p1, p2, p3), PathCommand::CurveTo(q1, q2, q3)) => {
            near(p1, q1) && near(p2, q2) && near(p3, q3)
        }
        (PathCommand::Close, PathCommand::Close) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(y: f32) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point { x: 10.0, y }));
        path.push(PathCommand::LineTo(Point { x: 90.0, y }));
        path
    }

    fn adjusted(width: f32) -> Stroke {
        Stroke {
            width,
            adjust: true,
            ..Stroke::default()
        }
    }

    fn device_ys(fit: &GridFit, to_device: Transform) -> Vec<f32> {
        fit.path
            .commands()
            .iter()
            .filter_map(|c| match *c {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some(to_device.apply(p).y),
                _ => None,
            })
            .collect()
    }

    /// The clause's purpose at every placement: a 0.6-pixel rule an eighth of a pixel further
    /// along each time is fitted to one whole pixel centred on the same pixel row each time,
    /// so it covers one whole row wherever it was put, and its width is within the clause's
    /// half a pixel of the requested one.
    #[test]
    fn a_thin_rule_is_one_whole_pixel_row_at_every_placement() {
        for scale in [1.0_f32, 2.0] {
            let to_device = Transform::scale(scale, scale);
            for eighth in 0_u8..8 {
                let y = (50.0 + f32::from(eighth) / 8.0) / scale;
                let fit = grid_fit(&rule(y), &adjusted(0.6 / scale), to_device)
                    .expect("an off-grid rule under one and a half pixels is fitted");
                assert!((fit.stroke.width * scale - 1.0).abs() < 1e-6);
                for device in device_ys(&fit, to_device) {
                    assert!(
                        (device - 50.5).abs() < 1e-4,
                        "scale {scale}, {eighth}/8: the rule's centre is at {device}"
                    );
                }
            }
        }
    }

    /// The parameter's initial value is `false` (Table 57), and the clause's first sentence is
    /// conditional on it being enabled.
    #[test]
    fn nothing_is_fitted_without_stroke_adjustment() {
        let stroke = Stroke {
            width: 0.6,
            ..Stroke::default()
        };
        assert!(grid_fit(&rule(50.3), &stroke, Transform::IDENTITY).is_none());
    }

    /// The clause's first sentence binds every width: a rule of any width is fitted to the
    /// nearest whole number of pixels, its edges on pixel boundaries at every placement — a
    /// two-pixel rule's centre on a boundary, a three-pixel rule's on a pixel centre — and its
    /// width within the clause's half a pixel of the requested one.
    #[test]
    fn a_rule_of_any_width_is_whole_pixel_rows_at_every_placement() {
        for scale in [1.0_f32, 2.0] {
            let to_device = Transform::scale(scale, scale);
            // The third field: whether the rule at the first placement is already on the grid.
            let cases = [
                (1.6_f32, 2.0_f32, false),
                (2.0, 2.0, true),
                (2.5, 2.0, false),
                (3.2, 3.0, false),
            ];
            for (requested, pixels, on_grid) in cases {
                for eighth in 0_u8..8 {
                    let y = (50.0 + f32::from(eighth) / 8.0) / scale;
                    let at = format!("scale {scale}, width {requested}, {eighth}/8");
                    let fit = grid_fit(&rule(y), &adjusted(requested / scale), to_device);
                    let Some(fit) = fit else {
                        // Only a rule the fit would leave where it is is declined.
                        assert!(on_grid && eighth == 0, "{at}");
                        continue;
                    };
                    assert!((fit.stroke.width * scale - pixels).abs() < 1e-5, "{at}");
                    assert!((pixels - requested).abs() <= 0.5, "{at}");
                    for device in device_ys(&fit, to_device) {
                        let edge = device - 0.5 * pixels;
                        assert!((edge - edge.round()).abs() < 1e-4, "{at}: edge at {edge}");
                        assert!(
                            (device - 50.0 - f32::from(eighth) / 8.0).abs() <= 0.5,
                            "{at}"
                        );
                    }
                }
            }
        }
    }

    /// The whole number of pixels is the nearest, the smaller of two equally near, and never
    /// fewer than one; the centre moves at most half a pixel, to where both edges are on
    /// boundaries.
    #[test]
    fn the_fitted_width_and_centre() {
        let widths = [
            (0.2, 1.0),
            (0.5, 1.0),
            (1.49, 1.0),
            (1.5, 1.0),
            (1.51, 2.0),
            (2.5, 2.0),
        ];
        for (device, pixels) in widths {
            assert!(
                (fitted_width(device) - pixels).abs() < f32::EPSILON,
                "{device}"
            );
        }
        assert!((fitted_centre(50.3, 1.0) - 50.5).abs() < f32::EPSILON);
        assert!((fitted_centre(50.25, 2.0) - 50.0).abs() < f32::EPSILON);
        assert!((fitted_centre(50.75, 2.0) - 51.0).abs() < f32::EPSILON);
        assert!((fitted_centre(-3.0, 1.0) - -2.5).abs() < f32::EPSILON);
    }

    /// A diagonal, a curve and a stretched transform are not shapes a fit is defined on.
    #[test]
    fn a_diagonal_a_curve_and_a_stretch_are_left_alone() {
        let mut diagonal = Path::new();
        diagonal.push(PathCommand::MoveTo(Point { x: 10.0, y: 10.3 }));
        diagonal.push(PathCommand::LineTo(Point { x: 90.0, y: 40.3 }));
        assert!(grid_fit(&diagonal, &adjusted(0.6), Transform::IDENTITY).is_none());

        let mut curve = rule(50.3);
        curve.push(PathCommand::CurveTo(
            Point { x: 95.0, y: 50.3 },
            Point { x: 99.0, y: 55.0 },
            Point { x: 99.0, y: 60.0 },
        ));
        assert!(grid_fit(&curve, &adjusted(0.6), Transform::IDENTITY).is_none());

        assert!(grid_fit(&rule(50.3), &adjusted(0.6), Transform::scale(1.0, 2.0)).is_none());
    }

    /// A closed rectangle's four sides are each fitted, the closing one included, and no
    /// vertex moves more than half a device pixel.
    #[test]
    fn a_rectangle_is_fitted_on_all_four_sides() {
        let corners = [(10.2, 20.7), (60.9, 20.7), (60.9, 70.1), (10.2, 70.1)];
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point { x: 10.2, y: 20.7 }));
        for &(x, y) in &corners[1..] {
            path.push(PathCommand::LineTo(Point { x, y }));
        }
        path.push(PathCommand::Close);
        let fit = grid_fit(&path, &adjusted(1.0), Transform::IDENTITY).expect("fitted");
        let moved: Vec<Point> = fit
            .path
            .commands()
            .iter()
            .filter_map(|c| match *c {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some(p),
                _ => None,
            })
            .collect();
        for (p, &(x, y)) in moved.iter().zip(&corners) {
            assert!((p.x - x).abs() <= 0.5 && (p.y - y).abs() <= 0.5);
            assert!((p.x.fract() - 0.5).abs() < 1e-4 && (p.y.fract() - 0.5).abs() < 1e-4);
        }
    }

    /// A one-pixel rule already centred on a pixel row is where the fit would put it.
    #[test]
    fn a_rule_already_on_the_grid_is_declined() {
        assert!(grid_fit(&rule(50.5), &adjusted(1.0), Transform::IDENTITY).is_none());
    }
}
