//! What a dashed *closed* subpath draws at the vertex its own close makes: ISO 32000-2
//! §8.4.3.4 and §8.4.3.6.
//!
//! Every other corner of a dashed path is decided by the dasher: a dash that spans a corner
//! carries the corner inside one open contour and the stroker joins it, and a dash that stops
//! short of one ends as its own contour and the stroker caps it. The vertex where a subpath
//! closes is the exception, because there the dasher has to decide whether the *last* dash and
//! the *first* dash are one mark that happens to wrap round. Two sentences of the standard
//! decide it, and both were added in ISO 32000-2.
//!
//! §8.4.3.4, below Table 54:
//!
//! > In a closed subpath that is dashed, if the first segment starts with an on-dash and the
//! > last segment ends within an on-dash, then they shall be joined.
//!
//! §8.4.3.6:
//!
//! > If the end of a dashed segment coincides exactly with a join point, then the end cap is
//! > painted before the corner.
//!
//! *Within* is the load-bearing word in the first, and *exactly* in the second. A pattern whose
//! last on-dash is cut short by the close is one mark wrapping the corner, and the join is
//! painted; a pattern whose on-dash **finishes** at the close is two marks meeting there, and
//! what is painted is two end caps.
//!
//! # Why this is not left to the rasterisers
//!
//! Because a dasher that merges the first and last dash of a closed contour cannot tell the two
//! cases apart: it sees "on at the start, on at the end" and merges. **All three dashers this tree
//! draws through did**, which is what makes this trap 2's shape rather than one library's bug —
//! measured with this rule turned off, on the scene `render-quorra/tests/dashed_close.rs` states:
//! the processor put 3.133 square units into a quadrant the clause leaves empty, quorra 3.086 and
//! vello 2.753.
//!
//! `doc/corpora/pdf-differences`'s `DegenerateDashing.pdf` is the witness and it states both cases
//! in one file. Its 200 × 45 rectangle under `[ 10 10 ] 0 d` and `5 w` has a perimeter of exactly
//! 490, so its last on-dash finishes at the lower-left corner: that one drew the round join the
//! document sets where the clause asks for two caps, and under butt caps it filled a corner the
//! clause leaves blank. The 200 × 44 rectangle beside it, perimeter 488, stops 8 units *into* its
//! last on-dash and is the case §8.4.3.4 joins.
//!
//! So the rule is stated here, in the crate all three rasterisers consume, for the reason
//! [`crate::degenerate`]'s comment gives: a decision a backend can make alone is a decision nobody
//! has made.
//!
//! # How a subpath is opened
//!
//! By replacing its [`PathCommand::Close`] with the straight segment that command stands for.
//! The geometry is unchanged — Table 58 defines `h` as "a straight line segment from the current
//! point to the starting point of the subpath" — and what changes is that the subpath now has
//! two ends, so the stroker caps them instead of joining them. Nothing else about the path moves,
//! and a subpath the rule does not reach is not copied at all.
//!
//! # What "exactly" can be decided about
//!
//! A subpath made of straight segments has an exactly computable length, so the clause's
//! coincidence is a question this module can answer. A subpath containing a cubic does not: a
//! Bézier's arc length has no closed form, so no position along the pattern can be established
//! exactly and no coincidence is claimed. Such a subpath is left as the file wrote it, which is
//! the same answer the standard's own word gives — a coincidence nobody can establish is not one.

use crate::geom::{Path, PathCommand, Point};

/// Opens every closed subpath whose dash pattern finishes a dash exactly at the closing vertex,
/// ISO 32000-2 §8.4.3.4 and §8.4.3.6.
///
/// `dash_array` and `dash_phase` are the stroke's own, in the path's space, and the path is the
/// geometry about to be dashed. Returns `None` — leaving the caller's path untouched and
/// allocating nothing — for a solid stroke, for a path with no closed subpath, and for the
/// overwhelmingly common case where no subpath's length lands on a dash boundary.
///
/// The dash array is expected to have an even length, which is what `pdf_model`'s `apply_dash`
/// leaves in the graphics state; an odd one alternates across its own end, so its period is twice
/// its sum and that is what is used here.
#[must_use]
pub fn opened_where_a_dash_ends_at_the_close(
    path: &Path,
    dash_array: &[f32],
    dash_phase: f32,
) -> Option<Path> {
    let period = period(dash_array)?;
    let to_open: Vec<(usize, Point)> = closed_subpaths(path)
        .into_iter()
        .filter(|subpath| ends_a_dash(subpath.length, dash_phase, period, dash_array))
        .map(|subpath| (subpath.close, subpath.start))
        .collect();
    if to_open.is_empty() {
        return None;
    }
    let mut out = Path::new();
    let mut next = to_open.iter().peekable();
    for (index, command) in path.commands().iter().enumerate() {
        match next.peek() {
            Some((at, start)) if *at == index => {
                out.push(PathCommand::LineTo(*start));
                next.next();
            }
            _ => out.push(*command),
        }
    }
    Some(out)
}

/// The distance along the pattern after which it repeats, or `None` where there is no pattern.
///
/// An odd-length array alternates on and off across its own end — `[3]` is three on, three off —
/// so it states a pattern of twice its sum. `pdf_model`'s `apply_dash` already doubles such an
/// array before it reaches a display list; this says the same thing for a caller that did not.
fn period(dash_array: &[f32]) -> Option<f32> {
    let total: f32 = dash_array.iter().sum();
    if dash_array.is_empty() || !total.is_finite() || total <= 0.0 {
        return None;
    }
    Some(if dash_array.len().is_multiple_of(2) {
        total
    } else {
        total * 2.0
    })
}

/// One closed subpath of a path: where it starts, where its `Close` is, and how long it is.
struct ClosedSubpath {
    /// The point its `MoveTo` named, which its `Close` returns to.
    start: Point,
    /// The index of its [`PathCommand::Close`] among the path's commands.
    close: usize,
    /// Its length in the path's own space, including the closing segment, or `None` where a
    /// cubic makes that length inexact.
    length: Option<f32>,
}

/// Every closed subpath of `path`, in construction order.
fn closed_subpaths(path: &Path) -> Vec<ClosedSubpath> {
    let mut out = Vec::new();
    let mut start = Point::new(0.0, 0.0);
    let mut current = start;
    // `None` once a cubic has been seen in the subpath under construction: see the module
    // comment on what "exactly" can be decided about.
    let mut length = Some(0.0_f32);
    for (index, command) in path.commands().iter().enumerate() {
        match *command {
            PathCommand::MoveTo(p) => {
                start = p;
                current = p;
                length = Some(0.0);
            }
            PathCommand::LineTo(p) => {
                length = length.map(|so_far| so_far + distance(current, p));
                current = p;
            }
            PathCommand::CurveTo(_, _, p) => {
                length = None;
                current = p;
            }
            PathCommand::Close => {
                out.push(ClosedSubpath {
                    start,
                    close: index,
                    length: length.map(|so_far| so_far + distance(current, start)),
                });
                // Table 58: after `h` the current point is the subpath's starting point, and a
                // further segment with no `m` before it begins a new subpath there.
                current = start;
                length = Some(0.0);
            }
        }
    }
    out
}

/// The straight-line distance between two points.
fn distance(from: Point, to: Point) -> f32 {
    (to.x - from.x).hypot(to.y - from.y)
}

/// Whether a subpath of this length finishes a dash exactly at its closing vertex.
///
/// The pattern position there is `dash_phase + length` reduced by the period, and the clause's
/// coincidence is that position falling on a boundary between two entries of the array — or on
/// the pattern's own start, which `rem_euclid` reports as zero. The comparison is exact because
/// §8.4.3.6's is: a producer whose arithmetic does not land on the boundary has not written the
/// coincidence the sentence is about.
#[expect(
    clippy::float_cmp,
    reason = "§8.4.3.6's condition is that the dash's end and the join point 'coincide \
              exactly', so the comparison is the clause's own. A margin here would be a \
              tolerance nobody derived, and it would take the neighbouring case with it: a \
              close that lands a hair inside its last on-dash is the case §8.4.3.4 joins."
)]
fn ends_a_dash(length: Option<f32>, dash_phase: f32, period: f32, dash_array: &[f32]) -> bool {
    let Some(length) = length else {
        return false;
    };
    if length <= 0.0 || !dash_phase.is_finite() {
        return false;
    }
    let at = (dash_phase + length).rem_euclid(period);
    if at == 0.0 {
        return true;
    }
    let mut boundary = 0.0_f32;
    for entry in dash_array {
        boundary += *entry;
        if boundary == at {
            return true;
        }
    }
    // An odd-length array's second half repeats the first, so its boundaries are the ones above
    // shifted by the array's own sum.
    if !dash_array.len().is_multiple_of(2) {
        for entry in dash_array {
            boundary += *entry;
            if boundary == at {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::opened_where_a_dash_ends_at_the_close;
    use crate::geom::{Path, PathCommand, Point};

    /// The rectangle `x y w h re` states, as Table 58 defines it.
    fn rectangle(width: f32, height: f32) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(width, 0.0)));
        path.push(PathCommand::LineTo(Point::new(width, height)));
        path.push(PathCommand::LineTo(Point::new(0.0, height)));
        path.push(PathCommand::Close);
        path
    }

    /// `DegenerateDashing.pdf`'s left column: perimeter 490, `[ 10 10 ] 0 d`, so the last
    /// on-dash ends exactly at the lower-left corner. §8.4.3.6 paints the end cap there, which
    /// is what an opened subpath asks the stroker for.
    #[test]
    fn a_dash_finishing_at_the_close_opens_the_subpath() {
        let opened =
            opened_where_a_dash_ends_at_the_close(&rectangle(200.0, 45.0), &[10.0, 10.0], 0.0)
                .expect("the perimeter is 490, an exact multiple of the pattern's half-period");
        assert_eq!(
            opened.commands().last(),
            Some(&PathCommand::LineTo(Point::new(0.0, 0.0))),
            "the close is the straight segment Table 58 defines it as, and the subpath is open"
        );
        assert_eq!(opened.commands().len(), 5, "no command is added or dropped");
    }

    /// The same file's right column: perimeter 488, so the close falls two units *into* the last
    /// on-dash. §8.4.3.4 joins that one, which is what leaving the `Close` in place asks for.
    #[test]
    fn a_dash_still_running_at_the_close_leaves_the_subpath_closed() {
        assert!(
            opened_where_a_dash_ends_at_the_close(&rectangle(200.0, 44.0), &[10.0, 10.0], 0.0)
                .is_none()
        );
    }

    /// The phase moves the boundary with it: at 488 the close lands on one once the pattern
    /// starts 2 units in.
    #[test]
    fn the_phase_moves_the_boundary() {
        assert!(
            opened_where_a_dash_ends_at_the_close(&rectangle(200.0, 44.0), &[10.0, 10.0], 2.0)
                .is_some()
        );
    }

    /// A solid stroke has no pattern to finish, and neither has one whose array is all zero.
    #[test]
    fn a_solid_stroke_is_untouched() {
        assert!(opened_where_a_dash_ends_at_the_close(&rectangle(200.0, 45.0), &[], 0.0).is_none());
        assert!(
            opened_where_a_dash_ends_at_the_close(&rectangle(200.0, 45.0), &[0.0, 0.0], 0.0)
                .is_none()
        );
    }

    /// An open subpath has no closing vertex, so there is nothing for either clause to decide.
    #[test]
    fn an_open_subpath_is_untouched() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(490.0, 0.0)));
        assert!(opened_where_a_dash_ends_at_the_close(&path, &[10.0, 10.0], 0.0).is_none());
    }

    /// A subpath holding a cubic has no exactly computable length, so no coincidence is claimed.
    #[test]
    fn a_curve_leaves_the_subpath_closed() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(200.0, 0.0)));
        path.push(PathCommand::CurveTo(
            Point::new(210.0, 0.0),
            Point::new(210.0, 45.0),
            Point::new(200.0, 45.0),
        ));
        path.push(PathCommand::LineTo(Point::new(0.0, 45.0)));
        path.push(PathCommand::Close);
        assert!(opened_where_a_dash_ends_at_the_close(&path, &[10.0, 10.0], 0.0).is_none());
    }

    /// Two closed subpaths in one path are decided one at a time.
    #[test]
    fn each_subpath_answers_for_itself() {
        let mut path = rectangle(200.0, 45.0);
        path.extend(rectangle(200.0, 44.0).commands());
        let opened = opened_where_a_dash_ends_at_the_close(&path, &[10.0, 10.0], 0.0)
            .expect("the first subpath's perimeter is 490");
        assert_eq!(
            opened.commands()[4],
            PathCommand::LineTo(Point::new(0.0, 0.0)),
            "the 490 subpath is opened"
        );
        assert_eq!(
            opened.commands()[9],
            PathCommand::Close,
            "the 488 subpath is left closed"
        );
    }

    /// An odd-length array states a pattern of twice its sum, so its boundaries are every entry
    /// of it twice over: `[ 10 ]` is ten on, ten off, and a perimeter of 490 finishes a dash.
    #[test]
    fn an_odd_array_alternates_across_its_own_end() {
        assert!(
            opened_where_a_dash_ends_at_the_close(&rectangle(200.0, 45.0), &[10.0], 0.0).is_some()
        );
        assert!(
            opened_where_a_dash_ends_at_the_close(&rectangle(200.0, 44.0), &[10.0], 0.0).is_none()
        );
    }
}
