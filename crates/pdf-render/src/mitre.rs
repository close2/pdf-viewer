//! The mitre a file asks for and a rasteriser will not draw: ISO 32000-2 §8.4.3.5.
//!
//! # The clause, and its closed form
//!
//! §8.4.3.4 says what a mitre join *is* — "[t]he outer edges of the strokes for the two segments
//! shall be extended until they meet at an angle, as in a picture frame" — and §8.4.3.5 says how
//! long that is allowed to be:
//!
//! > The miter limit shall impose a maximum on the ratio of the miter length to the line width
//! > (see "Figure 15 -Miter length"). When the limit is exceeded, the join is converted from a
//! > miter to a bevel.
//!
//! The ratio is stated by the clause itself, as a formula the markdown conversion cannot carry
//! (`doc/md/`'s line reads `formula-not-decoded`; the PDF prints it): for the angle φ between the
//! segments in user space,
//!
//! ```text
//! miterLength / lineWidth = 1 / sin(φ / 2)
//! ```
//!
//! and the clause's own EXAMPLE is what checks a reading of it — "[a] miter limit of 1.414
//! converts miters to bevels for [φ] less than 90 degrees, a limit of 2.0 converts them for [φ]
//! less than 60 degrees, and a limit of 10.0 converts them for [φ] less than approximately 11.5
//! degrees" — since `1/sin(45°) = 1.41421`, `1/sin(30°) = 2` and `1/sin(5.75°) = 9.98`.
//!
//! Two consequences, and neither is a matter of taste. **A limit is a maximum rather than a
//! length**: a join at or under it is drawn to the length its own angle implies, and one over it
//! is a bevel — not a mitre truncated at the limit, which is what `tiny-skia`'s
//! `LineJoin::MiterClip` and SVG's `stroke-linejoin: miter-clip` are and what PDF has no
//! spelling for. And **the standard is explicit that the result may be enormous**: the NOTE
//! under the formula is one line, "Very large miter lengths are allowed."
//!
//! The miter *length* is the distance across the whole join — from the point where the two inner
//! edges meet to the point where the two outer edges do — so the tip sits half of it from the
//! vertex, at `(w/2) / sin(φ/2)`. That is the quantity a page shows and the one every assertion
//! about this module is written against.
//!
//! # Why this is a construction of ours rather than each rasteriser's own join code
//!
//! `doc/todo/11` §6 found `pdf-differences`' `LargeMitreLimit.pdf` drawn with a bevel where its
//! own `333 M` admits a mitre 166.676 line widths long. The cause is one shortcut in one library:
//! `tiny-skia`'s stroker classifies a join by the dot product of the two segments' normals
//! *before* it consults the limit, and treats anything within `SCALAR_NEARLY_ZERO` — 1/4096 — of
//! −1 as `Nearly180`, which goes straight to a blunt join. Since `sin(φ/2) = sqrt((1 + dot) / 2)`,
//! that angle test is a **ratio** cutoff in disguise, at `1 / sqrt(1/8192)` = 90.51: a join sharper
//! than that is bevelled whatever the file says. The graphics device's two strokers have no such
//! cutoff and draw the spike where `mutool` and `ghostscript` put it, which is the measurement
//! `render-quorra/examples/mitre_ladder` prints.
//!
//! So the *decision* — is this mitre admitted, and where does its tip go — is stated here, once,
//! from the clause's own formula, and a backend whose library will not draw it asks for the
//! geometry rather than working the angle out again. That is [`crate::degenerate`]'s rule and
//! trap 2's: a decision either backend can make alone is a decision neither has made.
//!
//! # What is handed back, and why it is a triangle
//!
//! A mitre join is a bevel join plus one triangle. §8.4.3.4's bevel finishes both segments with
//! butt caps and fills "the resulting notch beyond the ends of the segments … with a triangle",
//! whose two far corners are the outer ends of the two butt caps; the mitre extends the same two
//! outer edges until they cross. The region between the two is therefore the triangle
//! `(A, tip, B)` — `A` and `B` being those same two corners, which is what makes the pair exact
//! rather than approximate: the base of the triangle *is* the bevel's outer edge, so the union of
//! the two shapes is the mitred outline with no overlap and no gap.
//!
//! [`mitre_wedges`] returns those triangles for **every** join on the path whose mitre the limit
//! admits, and only when at least one of them is sharper than the caller says it can draw. A
//! caller therefore has one thing to do and one to avoid getting wrong: stroke with
//! [`LineJoin::Bevel`] and fill the triangles *with the outline in one path*, under the non-zero
//! rule. In one path because two draws of two shapes sharing an edge composite by §11.3.7.3's
//! union function and leave a seam along it, where coverage inside one scan conversion adds
//! (`doc/todo/_scan-conversion.md`, item 5); under the non-zero rule because a triangle outside
//! the outline is filled whichever way it winds, while the even-odd rule would punch it out.
//!
//! # What is declined, and what each decline costs
//!
//! - **A join style that is not §8.4.3.4's mitre.** Nothing is owed for a round or bevel join.
//! - **A line width of zero.** §8.4.3.5: "When the line width is zero, the miter length is zero."
//! - **A dash pattern.** A dash decides where a stroke has ends and where it still has joins, and
//!   this walks the *undashed* path, so a wedge could be added at a vertex the dash has cut away.
//!   The caller dashes after asking, so the honest answer here is to decline; `long_mitre_census`
//!   is what says how many documents that costs, and the answer at the time of writing is none.
//! - **A join that doubles back exactly.** `sin(φ/2)` is zero, the ratio is unbounded, and every
//!   finite limit is therefore exceeded — the clause's own answer is a bevel.
//! - **A vertex whose two segments are collinear and continue in the same direction.** §8.4.3.4:
//!   "[j]oin styles shall be significant only at points where consecutive segments of a path
//!   connect at an angle", and the triangle there has no area anyway.

use crate::geom::{Path, PathCommand, Point};
use crate::paint::{LineJoin, Stroke};

/// The mitres §8.4.3.5 admits on `path`, as triangles to fill beside a bevelled outline.
///
/// `half` is half the line width, in the path's own space — where §8.4.3.2 states a width — and
/// `beyond` is the mitre-length ratio above which the caller cannot draw a mitre itself. The
/// answer is `Some` only when some join on the path needs one that sharp: a path whose every
/// mitre the caller's own stroker draws is left entirely alone, which is what keeps this off the
/// ordinary stroking path.
///
/// The triangles are in the path's space too, so a caller composes them with the stroke's outline
/// and draws the two under one transform.
///
/// # Panics
///
/// Never: every branch below either returns or pushes three points onto a path.
#[must_use]
pub fn mitre_wedges(path: &Path, stroke: &Stroke, half: f32, beyond: f32) -> Option<Path> {
    if stroke.join != LineJoin::Miter || !stroke.dash_array.is_empty() || !half.is_finite() {
        return None;
    }
    // "When the line width is zero, the miter length is zero", and a width that is not a number is
    // not one the standard describes either.
    if half <= 0.0 {
        return None;
    }
    // **The path is not walked at all for an ordinary stroke, and this is why.** A join this is
    // owed for has a ratio over `beyond` *and* at or under the file's own limit, so a limit at or
    // under `beyond` cannot have one — whatever the geometry. Table 51's initial limit is 10 and
    // `beyond` is the ratio a stroker refuses, near 90, so the test below is what keeps every
    // stroke in the corpus on the ordinary path for the price of one comparison.
    if stroke.miter_limit <= beyond {
        return None;
    }
    // §8.4.3.5's ratio is at least 1 by construction, so a malformed smaller limit behaves as the
    // smallest one the formula can produce — the same clamp `Stroke`'s consumers apply.
    let limit = f64::from(stroke.miter_limit.max(1.0));
    let half = f64::from(half);

    let mut wedges = Path::new();
    let mut owed = false;
    let mut wedge = |from: Point, vertex: Point, to: Point| {
        let Some(mitre) = Mitre::at(from, vertex, to, half) else {
            return;
        };
        if mitre.ratio > limit {
            // "When the limit is exceeded, the join is converted from a miter to a bevel", which
            // is what the caller's stroker is about to draw.
            return;
        }
        owed |= mitre.ratio > f64::from(beyond);
        wedges.push(PathCommand::MoveTo(mitre.corners[0]));
        wedges.push(PathCommand::LineTo(mitre.corners[1]));
        wedges.push(PathCommand::LineTo(mitre.corners[2]));
        wedges.push(PathCommand::Close);
    };

    for_each_join(path, &mut wedge);

    (owed && !wedges.is_empty()).then_some(wedges)
}

/// The largest mitre-length ratio §8.4.3.5 admits anywhere on `path`, or `None` where it admits
/// none.
///
/// The measurement behind [`mitre_wedges`], and the same walk over the same joins, so that a census
/// of what documents state and the construction that draws it cannot disagree about where a path's
/// joins are or which of them the limit converts to a bevel.
/// `pdf-model/examples/long_mitre_census` is the caller.
#[must_use]
pub fn sharpest_admitted_mitre(path: &Path, stroke: &Stroke) -> Option<f32> {
    if stroke.join != LineJoin::Miter {
        return None;
    }
    let limit = f64::from(stroke.miter_limit.max(1.0));
    let mut sharpest: Option<f64> = None;
    for_each_join(path, &mut |from, vertex, to| {
        // The ratio is what a census wants and `Mitre::at` computes the triangle beside it; the
        // few multiplications that costs buy the guarantee this function exists for, which is that
        // the count and the construction agree about every join.
        if let Some(mitre) = Mitre::at(from, vertex, to, f64::from(stroke.width).max(0.0) / 2.0)
            && mitre.ratio <= limit
        {
            sharpest = Some(sharpest.map_or(mitre.ratio, |s: f64| s.max(mitre.ratio)));
        }
    });
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a ratio is bounded by the file's own limit, which is an f32 already"
    )]
    sharpest.map(|ratio| ratio as f32)
}

/// One admitted mitre: the ratio §8.4.3.5 bounds, and the triangle it adds to a bevel.
struct Mitre {
    /// `1 / sin(φ / 2)` — the ratio of mitre length to line width.
    ratio: f64,
    /// The bevel's two outer corners with the mitre's tip between them.
    corners: [Point; 3],
}

impl Mitre {
    /// The mitre at `vertex`, where a segment arrives from `from` and one leaves towards `to`.
    ///
    /// `None` where the clause states no mitre: a direction that cannot be taken, a join that
    /// doubles back exactly, one that does not turn at all, or arithmetic that left the range a
    /// coordinate can hold.
    fn at(from: Point, vertex: Point, to: Point, half: f64) -> Option<Self> {
        let into = unit(from, vertex)?;
        let out = unit(vertex, to)?;
        let dot = into.0 * out.0 + into.1 * out.1;
        // sin(φ/2) from the two directions: the turn is `π − φ`, so `dot = −cos φ` and
        // `sin(φ/2) = sqrt((1 − cos φ) / 2) = sqrt((1 + dot) / 2)`.
        let sin_half = f64::midpoint(1.0, dot).max(0.0).sqrt();
        if sin_half <= 0.0 {
            return None;
        }
        let ratio = 1.0 / sin_half;
        // Which side the mitre is on: the outer side of the turn. A left turn (a positive cross
        // product) puts it on the right, and the right normal of a direction `d` is `(dy, −dx)`.
        let cross = into.0 * out.1 - into.1 * out.0;
        if cross == 0.0 && dot > 0.0 {
            return None; // straight through: no join mark of any style
        }
        let outward = |d: (f64, f64)| {
            if cross > 0.0 {
                (d.1, -d.0)
            } else {
                (-d.1, d.0)
            }
        };
        let (n_into, n_out) = (outward(into), outward(out));
        // The bisector: `|n_into + n_out| = 2 sin(φ/2)`, since the angle between the two outward
        // normals is the turn. Normalising by its own length rather than by that identity keeps
        // the arithmetic honest where `1 + dot` has lost digits to cancellation.
        let (bx, by) = (n_into.0 + n_out.0, n_into.1 + n_out.1);
        let bisector = bx.hypot(by);
        if bisector <= 0.0 {
            return None;
        }
        let reach = half * ratio;
        let (vx, vy) = wide(vertex);
        let tip = (vx + bx / bisector * reach, vy + by / bisector * reach);
        let corner = |n: (f64, f64)| point(vx + n.0 * half, vy + n.1 * half);
        Some(Self {
            ratio,
            corners: [corner(n_into), point(tip.0, tip.1), corner(n_out)],
        })
    }
}

/// A point from two `f64` coordinates, which is where this module's arithmetic comes back to the
/// display list's own precision.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the display list is f32 throughout; a tip beyond that range is not a coordinate a \
              raster can hold either, and the infinity it becomes is refused by the caller's own \
              path conversion"
)]
fn point(x: f64, y: f64) -> Point {
    Point::new(x as f32, y as f32)
}

/// A point's two coordinates widened, which is the precision this module works in: a tip 833 units
/// from its vertex is the difference of two numbers a hundred times smaller than that.
fn wide(p: Point) -> (f64, f64) {
    (f64::from(p.x), f64::from(p.y))
}

/// The unit direction from `a` to `b`, or `None` where there is no distance to take one from.
fn unit(a: Point, b: Point) -> Option<(f64, f64)> {
    let ((ax, ay), (bx, by)) = (wide(a), wide(b));
    let (dx, dy) = (bx - ax, by - ay);
    let length = dx.hypot(dy);
    (length > 0.0 && length.is_finite()).then(|| (dx / length, dy / length))
}

/// Calls `join` for every vertex of `path` where two segments connect, with the point the
/// incoming tangent comes from and the point the outgoing one goes to.
///
/// The walk is [`crate::outline`]'s `hull` one concern narrower — a curve's tangent at a vertex
/// is taken from the nearest control point that is not the vertex itself, and a `Close` makes
/// both ends of a subpath joins rather than caps — and the two are held together by property
/// rather than by sharing code: `the_bound_contains_every_tip` asserts that the bound that walk
/// produces contains every tip this one finds, which fails if either walker sees a join the other
/// does not.
fn for_each_join(path: &Path, join: &mut impl FnMut(Point, Point, Point)) {
    let mut cursor: Option<Point> = None;
    let mut start: Option<Point> = None;
    let mut joins_at: Option<Point> = None;
    let mut joins_from: Option<Point> = None;
    let mut leaves_start: Option<Point> = None;
    for command in path.commands() {
        match *command {
            PathCommand::MoveTo(p) => {
                cursor = Some(p);
                start = Some(p);
                joins_at = None;
                joins_from = None;
                leaves_start = None;
            }
            PathCommand::LineTo(p) => {
                let Some(from) = cursor.or(start) else {
                    continue;
                };
                if let (Some(vertex), Some(behind)) = (joins_at, joins_from) {
                    join(behind, vertex, p);
                }
                joins_at = Some(p);
                joins_from = Some(from);
                if leaves_start.is_none() {
                    leaves_start = Some(p);
                }
                cursor = Some(p);
                start.get_or_insert(from);
            }
            PathCommand::CurveTo(c1, c2, p) => {
                let Some(from) = cursor.or(start) else {
                    continue;
                };
                if let (Some(vertex), Some(behind)) = (joins_at, joins_from) {
                    // A cubic's tangent at its start points at the first control point that is
                    // not the start itself.
                    if let Some(ahead) = [c1, c2, p].into_iter().find(|point| *point != vertex) {
                        join(behind, vertex, ahead);
                    }
                }
                joins_at = Some(p);
                joins_from = [c2, c1, from].into_iter().find(|point| *point != p);
                if leaves_start.is_none() {
                    leaves_start = [c1, c2, p].into_iter().find(|point| *point != from);
                }
                cursor = Some(p);
                start.get_or_insert(from);
            }
            PathCommand::Close => {
                if let (Some(from), Some(to)) = (cursor, start) {
                    if let (Some(vertex), Some(behind)) = (joins_at, joins_from) {
                        join(behind, vertex, to);
                    }
                    // Closing makes the start point a join too, with the closing segment coming
                    // in and the subpath's first segment going out. **Where the subpath already
                    // ended at its own start the closing segment has no length**, so what arrives
                    // is the previous segment — and getting that wrong would lose a mitre rather
                    // than misplace one, because the caller has been told to bevel every join on
                    // this path.
                    let behind = if from == to { joins_from } else { Some(from) };
                    if let (Some(behind), Some(ahead)) = (behind, leaves_start) {
                        join(behind, to, ahead);
                    }
                    cursor = Some(to);
                }
                joins_at = None;
                joins_from = None;
                leaves_start = None;
                start = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mitre_wedges;
    use crate::geom::{Path, PathCommand, Point, Transform};
    use crate::outline::stroked_bounds;
    use crate::paint::{LineJoin, Stroke};

    /// The corpus witness, reduced: `doc/todo/11` §6's four lines, one join, `333 M`, `10 w`.
    fn witness() -> (Path, Stroke) {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(75.0, -75.0)));
        path.push(PathCommand::LineTo(Point::new(100.0, -50.0)));
        path.push(PathCommand::LineTo(Point::new(100.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(100.9, -75.0)));
        (
            path,
            Stroke {
                width: 10.0,
                miter_limit: 333.0,
                join: LineJoin::Miter,
                ..Stroke::default()
            },
        )
    }

    /// §8.4.3.5's own arithmetic, on the join the corpus file is about.
    ///
    /// φ = atan(0.9 / 75) = 0.687516°, so the ratio is `1 / sin(φ/2)` = 166.676 — inside the
    /// file's stated 333 — and the tip therefore sits `(w/2) × 166.676` = 833.38 units from the
    /// vertex, in the direction that bisects the two outer edges. Both numbers are the clause's;
    /// `mutool` and `ghostscript` agreeing with them is evidence that this reading is right and
    /// not the reason it is written down.
    #[test]
    fn the_tip_sits_where_the_ratio_puts_it() {
        let (path, stroke) = witness();
        let wedges = mitre_wedges(&path, &stroke, stroke.width / 2.0, 90.51).expect("one mitre");

        let phi = (0.9_f64 / 75.0).atan();
        let ratio = 1.0 / (phi / 2.0).sin();
        assert!((ratio - 166.6757).abs() < 1e-3, "ratio {ratio}");
        let reach = 5.0 * ratio;

        // Two triangles: the file's own sharp join at (100, 0) and the 135° corner below it,
        // whose mitre this module states as well because the caller is about to bevel both.
        let commands = wedges.commands();
        assert_eq!(commands.len(), 8, "two triangles: {commands:?}");
        // Each triangle's second point is its tip; the sharp join's is the one that reaches.
        let tip = commands
            .iter()
            .filter_map(|command| match *command {
                PathCommand::LineTo(p) => Some(p),
                _ => None,
            })
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("a tip");
        // The vertex is (100, 0) and the mitre points up, the spike leaning left by the same
        // half-angle that makes it long.
        let (dx, dy) = (f64::from(tip.x) - 100.0, f64::from(tip.y));
        assert!(
            (dx.hypot(dy) - reach).abs() < 0.05,
            "tip {tip:?} is {} from the vertex, not {reach}",
            dx.hypot(dy)
        );
        assert!(
            (dy - 833.38).abs() < 0.05,
            "the tip should sit 833.38 above the join, not {dy}"
        );
    }

    /// A limit the ratio exceeds is a bevel, which is nothing for this module to add.
    #[test]
    fn a_ratio_over_the_limit_is_left_to_the_bevel() {
        let (path, stroke) = witness();
        let bevelled = Stroke {
            // 166.676 is the join's own ratio; anything under it converts the join.
            miter_limit: 100.0,
            ..stroke
        };
        assert!(mitre_wedges(&path, &bevelled, 5.0, 90.51).is_none());
    }

    /// A mitre the caller's own stroker draws is left to it, whatever the limit permits.
    #[test]
    fn a_mitre_the_caller_can_draw_is_not_taken_over() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(10.0, 10.0)));
        path.push(PathCommand::LineTo(Point::new(50.0, 10.0)));
        path.push(PathCommand::LineTo(Point::new(50.0, 50.0)));
        let stroke = Stroke {
            width: 4.0,
            miter_limit: 333.0,
            join: LineJoin::Miter,
            ..Stroke::default()
        };
        // A right angle's ratio is 1/sin(45°) = 1.414, far under any caller's threshold.
        assert!(mitre_wedges(&path, &stroke, 2.0, 90.51).is_none());
    }

    /// The three cases the clause and the caller between them exclude.
    #[test]
    fn nothing_is_owed_where_the_clause_states_no_mitre() {
        let (path, stroke) = witness();
        for declined in [
            Stroke {
                join: LineJoin::Round,
                ..stroke.clone()
            },
            Stroke {
                join: LineJoin::Bevel,
                ..stroke.clone()
            },
            Stroke {
                dash_array: vec![3.0, 2.0],
                ..stroke.clone()
            },
        ] {
            assert!(
                mitre_wedges(&path, &declined, 5.0, 90.51).is_none(),
                "{declined:?}"
            );
        }
        // "When the line width is zero, the miter length is zero."
        assert!(mitre_wedges(&path, &stroke, 0.0, 90.51).is_none());
    }

    /// A join that doubles back exactly has no finite mitre, and every limit is exceeded.
    #[test]
    fn a_join_that_doubles_back_has_no_mitre() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(10.0, 10.0)));
        path.push(PathCommand::LineTo(Point::new(50.0, 10.0)));
        path.push(PathCommand::LineTo(Point::new(10.0, 10.0)));
        let stroke = Stroke {
            width: 4.0,
            miter_limit: 1.0e6,
            join: LineJoin::Miter,
            ..Stroke::default()
        };
        assert!(mitre_wedges(&path, &stroke, 2.0, 90.51).is_none());
    }

    /// The two walkers over a path's joins have to agree about where the joins are.
    ///
    /// [`stroked_bounds`] walks a path for a bound and reaches `miter_limit × w/2` at a mitre;
    /// this module walks it for the mitre itself. They share no code, so what holds them together
    /// is that the bound contains every tip — which fails if either sees a vertex the other does
    /// not, on a curve's tangent or at the point a `Close` turns into a join.
    #[test]
    fn the_bound_contains_every_tip() {
        let stroke = Stroke {
            width: 6.0,
            miter_limit: 400.0,
            join: LineJoin::Miter,
            ..Stroke::default()
        };
        let mut spiked = Path::new();
        spiked.push(PathCommand::MoveTo(Point::new(20.0, 20.0)));
        spiked.push(PathCommand::LineTo(Point::new(120.0, 20.0)));
        spiked.push(PathCommand::LineTo(Point::new(20.9, 21.0)));
        spiked.push(PathCommand::CurveTo(
            Point::new(21.0, 60.0),
            Point::new(100.0, 61.0),
            Point::new(21.2, 22.0),
        ));
        spiked.push(PathCommand::Close);

        let bounds = stroked_bounds(&spiked, &stroke, Transform::IDENTITY).expect("bounded");
        let wedges = mitre_wedges(&spiked, &stroke, stroke.width / 2.0, 90.51).expect("mitres");
        let mut tips = 0_usize;
        for command in wedges.commands() {
            let (PathCommand::MoveTo(point) | PathCommand::LineTo(point)) = *command else {
                continue;
            };
            tips += 1;
            assert!(
                point.x >= bounds.min.x - 1e-2
                    && point.x <= bounds.max.x + 1e-2
                    && point.y >= bounds.min.y - 1e-2
                    && point.y <= bounds.max.y + 1e-2,
                "{point:?} is outside {bounds:?}"
            );
        }
        assert!(tips >= 6, "the closed path has more than one join: {tips}");
    }
}
