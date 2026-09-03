//! Which sites of a tiling can mark the page: the lattice cells a fill's interior reaches.
//!
//! ISO 32000-2 §8.7.3.1 has the processor "paint the cell on the current page as many times
//! as necessary to fill an area", and the area is the path's interior — every cell is clipped
//! to it (`Interpreter::tile` makes the path the tiles' clip). A site whose cell box lies wholly
//! outside that interior therefore paints nothing, and since ADR 0430 made a site a *copy* of the
//! cell's commands, it still costs one copy of every command the cell holds. On a floor plan whose
//! walls are hatched — `batch5/PDFIUM/PDFIUM-1497-2.pdf`, thirty-four polygons of an A3 sheet at an
//! 8-unit cell — the walls are a few per cent of their own hull, and nine sites in ten were copies
//! the clip then threw away, against the same budget the rest of the page draws from (ADR 0810).
//!
//! So the lattice is cut to the sites the interior reaches, **conservatively**: a site is kept
//! wherever this cannot prove its box misses the interior, and never dropped where the box
//! touches it. The proof is a scan conversion of the path onto the lattice, one row band at a
//! time, in the pattern's own space:
//!
//! - **every cell an edge passes through is kept.** An edge clipped to the band spans a range of
//!   x, and every column whose box meets that range is kept — a superset of the cells the edge
//!   crosses, which is the direction that is safe;
//! - **every cell the interior covers at the band's centre line is kept**, by the fill rule
//!   §8.5.3.3 names: the edges crossing the centre line are sorted by x and walked, non-zero
//!   winding or even–odd parity says which gaps are inside, and every column whose box meets an
//!   inside gap is kept;
//! - **a curve is kept whole.** A cubic lies inside the hull of its four control points, so every
//!   cell under that hull's box is kept, and the curve's *crossings* are taken from its control
//!   polygon — a point outside the hull sees the same winding from the polygon as from the curve,
//!   because the loop they form between them lies inside the hull.
//!
//! Why the two rules together miss nothing: take a point of the interior inside a site's box. If
//! the band's centre line meets the interior anywhere inside the box, the second rule keeps the
//! site. If it does not, the vertical segment from the point to the centre line stays inside the
//! box — a box is convex — and leaves the interior on the way, so an edge crosses that segment,
//! inside the box, inside the band, and the first rule keeps the site.
//!
//! A row is answered when the loop reaches it rather than for the whole lattice at once, because
//! the lattice a file may state is `span`'s clamp squared — two million rows — and the budget in
//! `Interpreter::repeat_cell` is what decides how many rows are ever asked for.
//!
//! **And asking is itself work, which is why the page has a budget for it.** A row costs
//! [`Reach::cost`] edge tests whether or not it keeps a site, and a row that keeps *none* spends
//! no copies — so a file stating a fill of a hundred thousand edges clustered at the foot of a
//! lattice of a million rows would have had the rest of those rows scanned for nothing, at no
//! charge to any budget, which is hours. `Interpreter::MAX_REACH_SCAN` bounds the page's whole
//! scan; past it the caller stops asking and keeps every site, which is what a stroke does and
//! what this module did not exist to do. Giving up is safe in the direction that matters —
//! more sites, never fewer — and the copies are still bounded by the two budgets that bound
//! them (ADR 0810).

use pdf_render::{FillRule, Path, PathCommand, Point, Transform};

/// One straight edge of the flattened outline, in pattern space.
#[derive(Clone, Copy, Debug)]
struct Edge {
    from: Point,
    to: Point,
}

/// The sites a fill's interior reaches, answered a row at a time.
#[derive(Debug)]
pub(super) struct Reach {
    /// Every straight edge, with each subpath closed the way §8.5.3.3 closes it for filling and
    /// each curve replaced by its control polygon.
    edges: Vec<Edge>,
    /// The control-point box of every curve, `(min_x, min_y, max_x, max_y)`, kept whole.
    curves: Vec<(f32, f32, f32, f32)>,
    /// §8.5.3.3's rule for which gaps between crossings are inside.
    rule: FillRule,
    /// `/XStep` and `/YStep`, and the cell box's extent along each axis as `(low, high)`.
    step: (f32, f32),
    cell_x: (f32, f32),
    cell_y: (f32, f32),
}

impl Reach {
    /// The interior of `path` under `rule`, seen through `to_pattern`, against a lattice of
    /// `step` whose cell box spans `cell_x` and `cell_y` about each site.
    ///
    /// `None` where the path has a coordinate that is not finite: nothing can be proved about
    /// where such a path is, and the caller then keeps every site, which is what it did before.
    pub(super) fn of(
        path: &Path,
        to_pattern: Transform,
        rule: FillRule,
        step: (f32, f32),
        cell_x: (f32, f32),
        cell_y: (f32, f32),
    ) -> Option<Self> {
        let mut edges = Vec::new();
        let mut curves = Vec::new();
        let mut start: Option<Point> = None;
        let mut current: Option<Point> = None;
        let mut finite = true;
        let mut at = |point: Point| {
            let mapped = to_pattern.apply(point);
            if !mapped.x.is_finite() || !mapped.y.is_finite() {
                finite = false;
            }
            mapped
        };
        // §8.5.3.3: a subpath left open is closed for filling "as if by a closing operator", so
        // every subpath contributes its closing edge whether or not `h` was written.
        let close = |edges: &mut Vec<Edge>, current: Option<Point>, start: Option<Point>| {
            if let (Some(from), Some(to)) = (current, start) {
                edges.push(Edge { from, to });
            }
        };
        for command in path.commands() {
            match command {
                PathCommand::MoveTo(point) => {
                    close(&mut edges, current, start);
                    let point = at(*point);
                    start = Some(point);
                    current = Some(point);
                }
                PathCommand::LineTo(point) => {
                    let point = at(*point);
                    if let Some(from) = current {
                        edges.push(Edge { from, to: point });
                    } else {
                        start = Some(point);
                    }
                    current = Some(point);
                }
                PathCommand::CurveTo(a, b, c) => {
                    let (a, b, c) = (at(*a), at(*b), at(*c));
                    if let Some(from) = current {
                        curves.push((
                            from.x.min(a.x).min(b.x).min(c.x),
                            from.y.min(a.y).min(b.y).min(c.y),
                            from.x.max(a.x).max(b.x).max(c.x),
                            from.y.max(a.y).max(b.y).max(c.y),
                        ));
                        edges.push(Edge { from, to: a });
                        edges.push(Edge { from: a, to: b });
                        edges.push(Edge { from: b, to: c });
                    } else {
                        start = Some(c);
                    }
                    current = Some(c);
                }
                PathCommand::Close => {
                    close(&mut edges, current, start);
                    current = start;
                }
            }
        }
        close(&mut edges, current, start);
        if !finite || step.0 == 0.0 || step.1 == 0.0 {
            return None;
        }
        Some(Self {
            edges,
            curves,
            rule,
            step,
            cell_x: (cell_x.0.min(cell_x.1), cell_x.0.max(cell_x.1)),
            cell_y: (cell_y.0.min(cell_y.1), cell_y.0.max(cell_y.1)),
        })
    }

    /// What one call to [`Self::row`] costs, in edge tests, and never less than one.
    ///
    /// Every edge is asked twice — once for the band it may cross and once for the centre line —
    /// and every curve box once, so the row's work is linear in this and the caller charges it
    /// against [`super::Interpreter::MAX_REACH_SCAN`] before asking.
    pub(super) fn cost(&self) -> usize {
        self.edges.len().saturating_add(self.curves.len()).max(1)
    }

    /// The columns of `row`, within `first..=last`, whose cell box the interior may reach — as
    /// sorted, disjoint, inclusive intervals.
    pub(super) fn row(&self, row: i32, (first, last): (i32, i32)) -> Vec<(i32, i32)> {
        let offset = self.step.1 * as_f32(row);
        let band = (offset + self.cell_y.0, offset + self.cell_y.1);
        let centre = f32::midpoint(band.0, band.1);
        let mut kept: Vec<(i32, i32)> = Vec::new();
        let mut keep = |x_low: f32, x_high: f32| {
            if let Some(columns) = self.columns_meeting(x_low, x_high, (first, last)) {
                kept.push(columns);
            }
        };

        // Every cell an edge passes through, and every curve's whole box.
        for edge in &self.edges {
            if let Some((x_low, x_high)) = edge.within(band) {
                keep(x_low, x_high);
            }
        }
        for &(x0, y0, x1, y1) in &self.curves {
            if y1 >= band.0 && y0 <= band.1 {
                keep(x0, x1);
            }
        }

        // Every cell the interior covers on the band's centre line.
        let mut crossings: Vec<(f32, i32)> = self
            .edges
            .iter()
            .filter_map(|edge| edge.crossing(centre))
            .collect();
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = 0i32;
        for pair in crossings.windows(2) {
            winding = winding.saturating_add(pair[0].1);
            let inside = match self.rule {
                FillRule::NonZero => winding != 0,
                FillRule::EvenOdd => winding % 2 != 0,
            };
            if inside {
                keep(pair[0].0, pair[1].0);
            }
        }

        merged(kept)
    }

    /// The columns whose cell box meets `x_low..=x_high`, clipped to `first..=last`.
    fn columns_meeting(
        &self,
        x_low: f32,
        x_high: f32,
        (first, last): (i32, i32),
    ) -> Option<(i32, i32)> {
        // Site `c`'s box is `c * step + cell_x`, so it meets the range where
        // `c * step + cell.1 >= x_low` and `c * step + cell.0 <= x_high`; a negative step
        // turns both inequalities round, which is what taking the two bounds in either order
        // does.
        let a = (x_low - self.cell_x.1) / self.step.0;
        let b = (x_high - self.cell_x.0) / self.step.0;
        let (low, high) = (a.min(b).ceil(), a.max(b).floor());
        if !(low.is_finite() && high.is_finite()) {
            return Some((first, last));
        }
        let low = low.max(as_f32(first));
        let high = high.min(as_f32(last));
        if low > high {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "clamped between two i32 tile indices, which span() keeps within a million"
        )]
        Some((low as i32, high as i32))
    }
}

impl Edge {
    /// The x-range this edge covers inside the row band `(low, high)`, if it enters it.
    fn within(self, (low, high): (f32, f32)) -> Option<(f32, f32)> {
        let (y0, y1) = (self.from.y.min(self.to.y), self.from.y.max(self.to.y));
        if y1 < low || y0 > high {
            return None;
        }
        // Clip to the band along y; a horizontal edge is its own x-range.
        let dy = self.to.y - self.from.y;
        if dy.abs() <= f32::EPSILON {
            return Some((self.from.x.min(self.to.x), self.from.x.max(self.to.x)));
        }
        let at = |y: f32| self.from.x + (y - self.from.y) / dy * (self.to.x - self.from.x);
        let t_low = at(low.clamp(y0, y1));
        let t_high = at(high.clamp(y0, y1));
        Some((t_low.min(t_high), t_high.max(t_low)))
    }

    /// Where this edge crosses the horizontal line at `y`, with its direction, under the
    /// half-open rule that counts an endpoint on the line once: `from.y <= y < to.y` is +1 and
    /// `to.y <= y < from.y` is −1.
    fn crossing(self, y: f32) -> Option<(f32, i32)> {
        let upward = self.from.y <= y && y < self.to.y;
        let downward = self.to.y <= y && y < self.from.y;
        if !upward && !downward {
            return None;
        }
        let t = (y - self.from.y) / (self.to.y - self.from.y);
        let x = self.from.x + t * (self.to.x - self.from.x);
        Some((x, if upward { 1 } else { -1 }))
    }
}

/// Sorted, disjoint intervals from any collection of inclusive ones.
fn merged(mut intervals: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    intervals.sort_unstable();
    let mut out: Vec<(i32, i32)> = Vec::with_capacity(intervals.len());
    for (a, b) in intervals {
        match out.last_mut() {
            Some(last) if a <= last.1.saturating_add(1) => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

/// Widens a tile index for arithmetic in pattern space.
fn as_f32(index: i32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "tile indices are clamped to a million, exact in f32"
    )]
    {
        index as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point { x: x0, y: y0 }));
        path.push(PathCommand::LineTo(Point { x: x1, y: y0 }));
        path.push(PathCommand::LineTo(Point { x: x1, y: y1 }));
        path.push(PathCommand::LineTo(Point { x: x0, y: y1 }));
        path.push(PathCommand::Close);
        path
    }

    fn reach(path: &Path, rule: FillRule) -> Reach {
        Reach::of(
            path,
            Transform::IDENTITY,
            rule,
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 10.0),
        )
        .expect("finite")
    }

    /// An axis-aligned rectangle reaches exactly the columns its edges and interior cover.
    #[test]
    fn a_rectangle_reaches_its_own_columns_and_no_others() {
        let reach = reach(&rect(25.0, 25.0, 55.0, 45.0), FillRule::NonZero);
        // Row 3 spans y 30..40: inside the rectangle. Columns whose box meets x 25..55 are
        // 2 (20..30) to 5 (50..60).
        assert_eq!(reach.row(3, (-10, 20)), vec![(2, 5)]);
        // Row 5 spans y 50..60: the rectangle ends at 45, so nothing.
        assert_eq!(reach.row(5, (-10, 20)), vec![]);
        // Row 2 spans y 20..30: the bottom edge at y = 25 passes through it.
        assert_eq!(reach.row(2, (-10, 20)), vec![(2, 5)]);
    }

    /// A thin diagonal reaches only the cells along it, not the whole of its hull.
    #[test]
    fn a_diagonal_sliver_reaches_the_cells_along_it() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point { x: 0.0, y: 0.0 }));
        path.push(PathCommand::LineTo(Point { x: 200.0, y: 200.0 }));
        path.push(PathCommand::LineTo(Point { x: 200.0, y: 202.0 }));
        path.push(PathCommand::Close);
        let reach = reach(&path, FillRule::NonZero);
        // Row 10 spans y 100..110: the sliver is at x ≈ 100..110 there, reaching columns 9
        // to 11 — and not the twenty columns of its hull.
        let row = reach.row(10, (0, 20));
        assert_eq!(row.len(), 1);
        assert!(row[0].0 >= 9 && row[0].1 <= 11, "{row:?}");
    }

    /// Even–odd leaves the hole of a ring out; non-zero fills it when both loops wind alike.
    #[test]
    fn the_fill_rule_decides_a_rings_hole() {
        let mut path = rect(0.0, 0.0, 100.0, 100.0);
        // The same orientation as the outer loop, so non-zero winding is 2 in the hole.
        path.push(PathCommand::MoveTo(Point { x: 30.0, y: 30.0 }));
        path.push(PathCommand::LineTo(Point { x: 70.0, y: 30.0 }));
        path.push(PathCommand::LineTo(Point { x: 70.0, y: 70.0 }));
        path.push(PathCommand::LineTo(Point { x: 30.0, y: 70.0 }));
        path.push(PathCommand::Close);
        // Row 4 spans y 40..50, the centre line at 45 crosses both loops; columns 4 (40..50)
        // and 5 (50..60) lie wholly inside the hole.
        let even_odd = reach(&path, FillRule::EvenOdd).row(4, (0, 9));
        assert_eq!(even_odd, vec![(0, 3), (6, 9)]);
        let non_zero = reach(&path, FillRule::NonZero).row(4, (0, 9));
        assert_eq!(non_zero, vec![(0, 9)]);
    }

    /// A curve is kept by its control box, which is wider than the curve and never narrower.
    #[test]
    fn a_curve_keeps_every_cell_under_its_control_box() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point { x: 0.0, y: 0.0 }));
        path.push(PathCommand::CurveTo(
            Point { x: 0.0, y: 100.0 },
            Point { x: 100.0, y: 100.0 },
            Point { x: 100.0, y: 0.0 },
        ));
        path.push(PathCommand::Close);
        let reach = reach(&path, FillRule::NonZero);
        // Row 9 spans y 90..100: the control box reaches it, so every column under the box.
        assert_eq!(reach.row(9, (0, 20)), vec![(0, 10)]);
        // Row 12 spans y 120..130: nothing reaches it.
        assert_eq!(reach.row(12, (0, 20)), vec![]);
    }

    /// A coordinate that is not finite proves nothing, and the caller keeps every site.
    #[test]
    fn a_path_with_a_non_finite_point_is_not_reached_about() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point { x: 0.0, y: 0.0 }));
        path.push(PathCommand::LineTo(Point {
            x: f32::NAN,
            y: 10.0,
        }));
        path.push(PathCommand::Close);
        assert!(
            Reach::of(
                &path,
                Transform::IDENTITY,
                FillRule::NonZero,
                (10.0, 10.0),
                (0.0, 10.0),
                (0.0, 10.0)
            )
            .is_none()
        );
    }

    /// Overlapping and touching intervals merge; disjoint ones stay apart and sorted.
    #[test]
    fn intervals_merge_where_they_touch() {
        assert_eq!(
            merged(vec![(5, 6), (0, 2), (3, 4), (9, 9)]),
            vec![(0, 6), (9, 9)]
        );
    }
}
