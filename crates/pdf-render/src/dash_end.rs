//! What a dashed *open* subpath draws at its own end when a zero-length dash falls there:
//! ISO 32000-2 §8.4.3.6 and §8.5.3.2.
//!
//! §8.4.3.6 dispenses the pattern along the whole subpath:
//!
//! > A stroking operation shall take no measures to coordinate the dash pattern with features of
//! > the path; it simply shall dispense dashes and gaps along the path in the pattern defined by
//! > the dash array.
//!
//! and §8.5.3.2 says what a dash of no length leaves:
//!
//! > In the latter case, the line caps shall always be painted, since their orientation is
//! > determined by the direction of the underlying path except in the case of a degenerate
//! > subpath.
//!
//! A subpath of length `L` is the arc lengths from 0 to `L`, both ends included, so a zero-length
//! dash whose position is `L` lies on it exactly as one whose position is 0 does, and its round or
//! projecting cap is painted at the subpath's last point. Nothing in either clause sets the end
//! apart from the start: `[0 2] 0 d` on a rule 100 units long places 51 dots, and the same rule
//! drawn from its other end places the same 51.
//!
//! # Why this is not left to the dashers
//!
//! Because the three this tree draws through give two answers. `tiny-skia`'s dasher dispenses
//! while the distance it has walked is *less than* the contour's length, so a dash that begins at
//! the end is never dispensed and the last dot of a dotted rule is missing; `kurbo`'s, which
//! raster and Vello use, dispenses it. On a rule of 0.6 user units at scale 1 the two lanes
//! differed by a whole dot in the rule's last device columns, and which of them a given rule got
//! depended on each library's arithmetic rather than on the clause.
//!
//! So the rule is stated here, in the crate all three consume, for the reason
//! [`crate::degenerate`]'s comment gives: a decision a backend can make alone is a decision nobody
//! has made.
//!
//! # How the end's dash is dispensed
//!
//! By extending each subpath the rule reaches with a straight segment of twice
//! [`ZERO_DASH`], pointing the way the subpath's last segment points. That is the length
//! [`crate::dashes_showing_direction`] gives a zero-length dash plus as much again of the gap that
//! follows it, which is longer than that, so each dasher now meets the dash strictly inside the
//! path, dispenses it as a segment facing along the path, and stops inside the gap: the extension
//! carries one mark and paints nothing else. A dasher whose own arithmetic placed the dash a hair
//! before the end dispenses it there instead, and still once.
//!
//! # What "exactly" can be decided about
//!
//! As in [`crate::closing`]: a subpath of straight segments has an exactly computable length and a
//! subpath holding a cubic does not, so no coincidence is claimed for the second and its end is
//! whatever its dasher makes of it.

use crate::closing::period;
use crate::degenerate::ZERO_DASH;
use crate::geom::{Path, PathCommand, Point};

/// Extends every open subpath whose dash pattern places a zero-length dash exactly at its end,
/// so that the dasher dispenses that dash, ISO 32000-2 §8.4.3.6 and §8.5.3.2.
///
/// `dash_array` and `dash_phase` are the stroke's own — the document's lengths, not the ones
/// [`crate::dashes_showing_direction`] substitutes — in the path's space, and the path is the
/// geometry about to be dashed. Returns `None`, leaving the caller's path untouched and
/// allocating nothing beyond one walk, for a pattern with no zero-length dash and for every path
/// none of whose open subpaths ends on one, which is nearly all of them.
///
/// A subpath that ends where it began is left alone when its start already carries a zero-length
/// dash: the two positions are one point and the dash at the start is that mark.
#[must_use]
pub fn extended_to_a_zero_dash_at_the_end(
    path: &Path,
    dash_array: &[f32],
    dash_phase: f32,
) -> Option<Path> {
    let period = period(dash_array)?;
    if !dash_phase.is_finite() {
        return None;
    }
    let starts = zero_dash_starts(dash_array, period);
    if starts.is_empty() {
        return None;
    }
    let begins_on_one = on_a_zero_dash(dash_phase, period, &starts);
    let reaches: Vec<(usize, Point)> = open_subpaths(path)
        .into_iter()
        .filter(|subpath| {
            subpath.length.is_some_and(|length| {
                length > 0.0
                    && on_a_zero_dash(dash_phase + length, period, &starts)
                    && !(begins_on_one && subpath.end == subpath.start)
            })
        })
        .filter_map(|subpath| subpath.reach().map(|reach| (subpath.after, reach)))
        .collect();
    if reaches.is_empty() {
        return None;
    }
    let mut out = Path::new();
    let mut next = reaches.iter().peekable();
    for (index, command) in path.commands().iter().enumerate() {
        if let Some((_, reach)) = next.next_if(|(after, _)| *after == index) {
            out.push(PathCommand::LineTo(*reach));
        }
        out.push(*command);
    }
    // The last subpath ends with the path, so its extension follows every command.
    for (_, reach) in next {
        out.push(PathCommand::LineTo(*reach));
    }
    Some(out)
}

/// Where along one period of the pattern each zero-length dash begins.
///
/// Even-numbered entries are dashes (§8.4.3.6). An odd-length array alternates on and off across
/// its own end — `[0 2 3]` is a dot, two off, three on, then no gap, two on, three off — so it is
/// read twice, which is the period [`period`] gives it.
fn zero_dash_starts(dash_array: &[f32], period: f32) -> Vec<f32> {
    let copies = if dash_array.len().is_multiple_of(2) {
        1
    } else {
        2
    };
    let mut starts = Vec::new();
    let mut at = 0.0_f32;
    for (index, entry) in dash_array
        .iter()
        .cycle()
        .take(dash_array.len().saturating_mul(copies))
        .enumerate()
    {
        if index.is_multiple_of(2) && *entry <= 0.0 {
            starts.push(at.rem_euclid(period));
        }
        at += *entry;
    }
    starts
}

/// Whether the pattern position `distance` into the pattern is where a zero-length dash begins.
///
/// §8.4.3.6's phase is reduced by the period whichever its sign: the clause's "incremented by
/// twice the sum of all lengths in the dash array until it is positive" adds a multiple of the
/// period, which `rem_euclid` does in one step.
fn on_a_zero_dash(distance: f32, period: f32, starts: &[f32]) -> bool {
    let at = distance.rem_euclid(period);
    // Exact, because the dash and the subpath's end are the same point only when the pattern
    // position there is the dash's own. A margin would be a tolerance nobody derived: a position a
    // hair short of the dash is one §8.4.3.6 leaves in the gap before it, with the dash past the
    // end of the path.
    starts.contains(&at)
}

/// One open subpath: where it starts and ends, the way its last segment points, and its length.
struct OpenSubpath {
    /// The point it begins at.
    start: Point,
    /// Its last point.
    end: Point,
    /// The last segment that spans a distance, from its first point to its last; zero where none
    /// does.
    direction: Point,
    /// Its length in the path's own space, or `None` where a cubic makes that length inexact.
    length: Option<f32>,
    /// The index of the command after its last one, which is where its extension goes.
    after: usize,
}

impl OpenSubpath {
    /// The point twice [`ZERO_DASH`] past its end along its last segment, or `None` where no
    /// segment gives it a direction.
    fn reach(&self) -> Option<Point> {
        let length = self.direction.x.hypot(self.direction.y);
        if length.is_nan() || length <= 0.0 {
            return None;
        }
        let step = ZERO_DASH * 2.0 / length;
        Some(Point::new(
            self.end.x + self.direction.x * step,
            self.end.y + self.direction.y * step,
        ))
    }
}

/// Every open subpath of `path` holding at least one segment, in construction order.
fn open_subpaths(path: &Path) -> Vec<OpenSubpath> {
    let origin = Point::new(0.0, 0.0);
    let fresh = |at: Point| OpenSubpath {
        start: at,
        end: at,
        direction: origin,
        length: Some(0.0),
        after: 0,
    };
    let mut out = Vec::new();
    let mut building = fresh(origin);
    // Whether the subpath under construction holds a segment: a lone `m` has no end to dash at.
    let mut spans = false;
    for (index, command) in path.commands().iter().enumerate() {
        match *command {
            PathCommand::MoveTo(p) => {
                if spans {
                    building.after = index;
                    out.push(building);
                }
                building = fresh(p);
                spans = false;
            }
            PathCommand::LineTo(p) => {
                let (dx, dy) = (p.x - building.end.x, p.y - building.end.y);
                if dx != 0.0 || dy != 0.0 {
                    building.direction = Point::new(dx, dy);
                }
                building.length = building.length.map(|so_far| so_far + dx.hypot(dy));
                building.end = p;
                spans = true;
            }
            PathCommand::CurveTo(_, c2, p) => {
                building.direction = Point::new(p.x - c2.x, p.y - c2.y);
                building.length = None;
                building.end = p;
                spans = true;
            }
            PathCommand::Close => {
                // A closed subpath has no end; `crate::closing` decides its closing vertex.
                // Table 58: after `h` the current point is the subpath's starting point, and a
                // further segment with no `m` before it begins a new subpath there.
                building = fresh(building.start);
                spans = false;
            }
        }
    }
    if spans {
        building.after = path.commands().len();
        out.push(building);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::extended_to_a_zero_dash_at_the_end;
    use crate::degenerate::ZERO_DASH;
    use crate::geom::{Path, PathCommand, Point};

    /// `x0 y0 m x1 y1 l`.
    fn rule(from: (f32, f32), to: (f32, f32)) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(from.0, from.1)));
        path.push(PathCommand::LineTo(Point::new(to.0, to.1)));
        path
    }

    /// `[0 2] 0 d` on a rule of 100 places a dot at 100, so the rule is carried twice
    /// [`ZERO_DASH`] further along its own direction and nothing else changes.
    #[test]
    fn a_dot_at_the_end_extends_the_rule() {
        let extended = extended_to_a_zero_dash_at_the_end(
            &rule((20.0, 10.0), (120.0, 10.0)),
            &[0.0, 2.0],
            0.0,
        )
        .expect("100 is a whole number of periods of 2");
        assert_eq!(
            extended.commands(),
            [
                PathCommand::MoveTo(Point::new(20.0, 10.0)),
                PathCommand::LineTo(Point::new(120.0, 10.0)),
                PathCommand::LineTo(Point::new(120.0 + ZERO_DASH * 2.0, 10.0)),
            ]
        );
    }

    /// A rule of 101 ends a unit into the gap after its last dot, so nothing is added.
    #[test]
    fn a_rule_ending_in_a_gap_is_untouched() {
        let path = rule((20.0, 10.0), (121.0, 10.0));
        assert!(extended_to_a_zero_dash_at_the_end(&path, &[0.0, 2.0], 0.0).is_none());
    }

    /// The direction is the last segment's, drawn from the far end as well.
    #[test]
    fn the_extension_follows_the_last_segment() {
        let extended = extended_to_a_zero_dash_at_the_end(
            &rule((120.0, 10.0), (20.0, 10.0)),
            &[0.0, 2.0],
            0.0,
        )
        .expect("the same rule from its other end");
        assert_eq!(
            extended.commands().last(),
            Some(&PathCommand::LineTo(Point::new(
                20.0 - ZERO_DASH * 2.0,
                10.0
            )))
        );
    }

    /// The phase moves where the pattern stands at the end: `[0 2] 1` on 101 units ends on a dot,
    /// and a negative phase is the same pattern shifted by whole periods (§8.4.3.6).
    #[test]
    fn the_phase_moves_the_dot() {
        let path = rule((0.0, 0.0), (101.0, 0.0));
        assert!(extended_to_a_zero_dash_at_the_end(&path, &[0.0, 2.0], 1.0).is_some());
        assert!(extended_to_a_zero_dash_at_the_end(&path, &[0.0, 2.0], -3.0).is_some());
        assert!(extended_to_a_zero_dash_at_the_end(&path, &[0.0, 2.0], 0.0).is_none());
    }

    /// A zero-length dash after a positive one: `[3 1 0 2]` has its dot 4 into the period of 6.
    #[test]
    fn a_dot_inside_the_pattern_is_found() {
        assert!(
            extended_to_a_zero_dash_at_the_end(
                &rule((0.0, 0.0), (10.0, 0.0)),
                &[3.0, 1.0, 0.0, 2.0],
                0.0
            )
            .is_some()
        );
        assert!(
            extended_to_a_zero_dash_at_the_end(
                &rule((0.0, 0.0), (9.0, 0.0)),
                &[3.0, 1.0, 0.0, 2.0],
                0.0
            )
            .is_none()
        );
    }

    /// An odd-length array alternates across its own end: `[0 2 3]` is a dot, 2 off, 3 on, then
    /// on the second pass 0 off, 2 on, 3 off, so its one zero-length dash begins at 0 of a period
    /// of 10 and the 0 at 5 is a gap.
    #[test]
    fn an_odd_array_is_read_twice() {
        let path = |length| rule((0.0, 0.0), (length, 0.0));
        assert!(extended_to_a_zero_dash_at_the_end(&path(10.0), &[0.0, 2.0, 3.0], 0.0).is_some());
        assert!(extended_to_a_zero_dash_at_the_end(&path(5.0), &[0.0, 2.0, 3.0], 0.0).is_none());
    }

    /// A pattern with no zero-length dash, and a solid stroke, are not this rule's.
    #[test]
    fn a_pattern_without_a_zero_dash_is_untouched() {
        let path = rule((0.0, 0.0), (100.0, 0.0));
        assert!(extended_to_a_zero_dash_at_the_end(&path, &[1.0, 1.0], 0.0).is_none());
        assert!(extended_to_a_zero_dash_at_the_end(&path, &[], 0.0).is_none());
    }

    /// A closed subpath has no end; a subpath holding a cubic has no exact length.
    #[test]
    fn closed_and_curved_subpaths_are_untouched() {
        let mut closed = rule((0.0, 0.0), (100.0, 0.0));
        closed.push(PathCommand::Close);
        assert!(extended_to_a_zero_dash_at_the_end(&closed, &[0.0, 2.0], 0.0).is_none());
        let mut curved = rule((0.0, 0.0), (50.0, 0.0));
        curved.push(PathCommand::CurveTo(
            Point::new(60.0, 0.0),
            Point::new(70.0, 0.0),
            Point::new(100.0, 0.0),
        ));
        assert!(extended_to_a_zero_dash_at_the_end(&curved, &[0.0, 2.0], 0.0).is_none());
    }

    /// A subpath drawn back to its start with `l` ends on the dot its start already carries.
    #[test]
    fn an_end_on_the_starting_dot_is_that_dot() {
        let mut square = rule((0.0, 0.0), (2.0, 0.0));
        square.push(PathCommand::LineTo(Point::new(2.0, 2.0)));
        square.push(PathCommand::LineTo(Point::new(0.0, 2.0)));
        square.push(PathCommand::LineTo(Point::new(0.0, 0.0)));
        assert!(extended_to_a_zero_dash_at_the_end(&square, &[0.0, 2.0], 0.0).is_none());
        // With the phase in the gap, the start carries no dot and the end's is its own.
        assert!(extended_to_a_zero_dash_at_the_end(&square, &[0.0, 3.0], 1.0).is_some());
    }

    /// Each open subpath is extended where it ends, before the next one's `m`.
    #[test]
    fn each_subpath_answers_for_itself() {
        let mut path = rule((0.0, 0.0), (4.0, 0.0));
        path.push(PathCommand::MoveTo(Point::new(0.0, 5.0)));
        path.push(PathCommand::LineTo(Point::new(3.0, 5.0)));
        path.push(PathCommand::MoveTo(Point::new(0.0, 9.0)));
        path.push(PathCommand::LineTo(Point::new(0.0, 15.0)));
        let extended = extended_to_a_zero_dash_at_the_end(&path, &[0.0, 2.0], 0.0)
            .expect("the first and last subpaths end on a dot");
        let reach = ZERO_DASH * 2.0;
        assert_eq!(
            extended.commands(),
            [
                PathCommand::MoveTo(Point::new(0.0, 0.0)),
                PathCommand::LineTo(Point::new(4.0, 0.0)),
                PathCommand::LineTo(Point::new(4.0 + reach, 0.0)),
                PathCommand::MoveTo(Point::new(0.0, 5.0)),
                PathCommand::LineTo(Point::new(3.0, 5.0)),
                PathCommand::MoveTo(Point::new(0.0, 9.0)),
                PathCommand::LineTo(Point::new(0.0, 15.0)),
                PathCommand::LineTo(Point::new(0.0, 15.0 + reach)),
            ]
        );
    }
}
