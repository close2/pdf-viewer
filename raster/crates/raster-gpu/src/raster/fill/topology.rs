//! How a fill's subpaths lie against one another: which cross themselves, which cross each
//! other, and how the rest nest (ADR 1389).
//!
//! §8.5.3.3's winding number is a sum over subpaths, and a subpath that does not cross
//! itself adds `0` outside and its orientation, `±1`, inside. So where no subpath crosses
//! itself or another, the insides are nested or apart, and the winding at a point is the
//! sum of the orientations of the subpaths holding it. **When each subpath's orientation
//! alternates with its depth** — each nested one wound against the one holding it, and
//! those held by none all wound one way — that sum is only ever `0` or one value, the whole
//! fill winds two neighbouring values, and [`fill_mask`](super::fill_mask)'s integral is the
//! set's area in every pixel. A glyph's contours and a stroke's abutting pieces are that
//! shape, which is why this module is asked first and why its answer is usually the last
//! word.
//!
//! Where it is not, [`overlap`](super::overlap) asks the same rule pixel by pixel of the
//! subpaths that pass through each one, with what this module found.

use raster_scene::Point;

use super::super::flatten::{Polyline, convex, polyline_bounds};

/// Box comparisons per unit of the fill's own work — an edge, or a pixel of its region —
/// past which the sweep stops: nothing about the fill is then vouched for, and it keeps its
/// integral in every pixel, the answer every pixel had before ADR 1389. The sweep is
/// quadratic where edges crowd together, and a fill of thousands of pixel-sized pieces
/// packed into a few pixels would otherwise cost a hundred times its own rasterisation;
/// bounded so, the question never costs more than a small multiple of the answer.
const TESTS_PER_UNIT: usize = 8;

/// Distinct subpaths the nesting rule is asked about at once, past which it answers no:
/// the rule is quadratic in them. The whole-fill question asks only among subpaths whose
/// boxes meet, so this bounds it per subpath rather than overall.
const MAX_GROUP: usize = 8;

/// What the nesting rule needs of one subpath that does not cross itself.
#[derive(Debug, Clone, Copy)]
struct Shape {
    /// `+1` or `−1` by the sign of its area, `0` for a subpath that bounds none.
    orientation: f32,
    /// A point strictly inside it, where it has an inside, in `f64`: see [`shape_of`].
    inside: (f64, f64),
}

/// What a fill's sweep found, and the nesting it has asked about since.
pub(super) struct Topology<'a> {
    subpaths: &'a [Polyline],
    /// The subpaths whose boxes meet the region: every other one adds a constant across it.
    near: Vec<u32>,
    /// Per subpath: whether it crosses itself.
    untrusted: Vec<bool>,
    /// The pairs of subpaths, lower index first, whose edges cross one another, sorted.
    crossing: Vec<(u32, u32)>,
    /// Per subpath, once asked: its orientation, and a point strictly inside it.
    shape: Vec<Option<Shape>>,
    /// Pairs `(a, b)` once asked, sorted: whether `a` holds `b`'s inside point.
    holds: Vec<((u32, u32), bool)>,
}

/// The buffers one thread's sweeps reuse.
#[derive(Debug, Default)]
struct Sweep {
    edges: Vec<Swept>,
    boxes: Vec<Box2>,
    order: Vec<u64>,
    active: Vec<usize>,
}

thread_local! {
    static SWEEP: std::cell::RefCell<Sweep> = std::cell::RefCell::default();
}

/// One edge of the fill: its subpath, its index, and its ends.
#[derive(Debug, Clone, Copy)]
struct Swept {
    subpath: u32,
    edge: u32,
    ends: (Point, Point),
}

/// A box, `[left, top, right, bottom]`.
pub(super) type Box2 = [f32; 4];

/// Call `visit` with every pair of `boxes` that meet, each pair once, sweeping along the
/// longer side of their union so that a line of text or a long stroke keeps few boxes in
/// the sweep at once. `false` where the comparisons ran past `budget`, or `visit` asked to
/// stop.
#[expect(clippy::arithmetic_side_effects)] // axis indices and counts
fn meeting_pairs(
    boxes: &[Box2],
    budget: usize,
    (order, active): (&mut Vec<u64>, &mut Vec<usize>),
    mut visit: impl FnMut(usize, usize) -> bool,
) -> bool {
    if u32::try_from(boxes.len()).is_err() {
        return false;
    }
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for b in boxes {
        for axis in 0..2 {
            lo[axis] = lo[axis].min(b[axis]);
            hi[axis] = hi[axis].max(b[axis + 2]);
        }
    }
    let (along, across) = if hi[0] - lo[0] >= hi[1] - lo[1] {
        (0, 1)
    } else {
        (1, 0)
    };
    // Each box's key and index packed into one integer, sorted as integers: the sweep's
    // order, without a comparison that looks each box up (ADR 1397). The answer does not
    // depend on how boxes with one start are ordered among themselves — every pair that
    // meets is visited whichever comes first, and the comparisons are counted the same.
    order.clear();
    order.extend(
        boxes
            .iter()
            .zip(0_u64..)
            .map(|(b, index)| (u64::from(ordered_bits(b[along])) << 32) | index),
    );
    order.sort_unstable();
    active.clear();
    let mut tests = 0_usize;
    #[expect(clippy::cast_possible_truncation)] // the low half is an index below `u32::MAX`
    for k in order.iter().map(|&entry| entry as u32 as usize) {
        let bk = boxes[k];
        active.retain(|&m| boxes[m][along + 2] >= bk[along]);
        tests += active.len();
        if tests > budget {
            return false;
        }
        for &m in active.iter() {
            let bm = boxes[m];
            if bm[across] <= bk[across + 2] && bk[across] <= bm[across + 2] && !visit(m, k) {
                return false;
            }
        }
        active.push(k);
    }
    true
}

/// `x`'s bits, mapped so that unsigned order is [`f32::total_cmp`]'s: a negative number's
/// bits all turned over, a positive one's sign bit set.
fn ordered_bits(x: f32) -> u32 {
    let bits = x.to_bits();
    if bits & 0x8000_0000 == 0 {
        bits | 0x8000_0000
    } else {
        !bits
    }
}

/// Subpaths [`plainly_two_values`] looks at before leaving the question to the sweep.
const PLAIN: usize = 4;

/// Whether the fill is, as far as `region` sees it, a few convex subpaths wound one way
/// whose boxes do not meet — the rectangle, the triangle, the dotted `i` — which cross
/// nothing and hold nothing: each winds `0` outside and the same value inside, and no point
/// is inside two. Such a fill needs no sweep, and most fills are one. (Wound against each
/// other, two apart in one pixel would wind it `+1`, `0` and `−1`.)
#[expect(clippy::float_cmp)] // orientations are exactly `−1`, `0` or `+1`
pub(super) fn plainly_two_values(subpaths: &[Polyline], region: Box2) -> bool {
    let mut boxes = [[0.0_f32; 4]; PLAIN];
    let mut count = 0;
    let mut wound = 0.0_f32;
    for polyline in subpaths {
        let Some((x0, y0, x1, y1)) = polyline_bounds(std::slice::from_ref(polyline)) else {
            continue;
        };
        if x1 < region[0] || y1 < region[1] || x0 > region[2] || y0 > region[3] {
            continue;
        }
        let Some(slot) = boxes.get_mut(count) else {
            return false;
        };
        if !convex(&polyline.points) {
            return false;
        }
        let sign = orientation(polyline);
        if sign != 0.0 {
            if wound != 0.0 && sign != wound {
                return false;
            }
            wound = sign;
        }
        *slot = [x0, y0, x1, y1];
        count = count.saturating_add(1);
    }
    let boxes = &boxes[..count];
    boxes.iter().enumerate().all(|(i, a)| {
        boxes
            .iter()
            .skip(i.saturating_add(1))
            .all(|b| a[2] < b[0] || b[2] < a[0] || a[3] < b[1] || b[3] < a[1])
    })
}

impl<'a> Topology<'a> {
    /// Test every pair of edges whose boxes meet, of the subpaths whose boxes meet
    /// `region` (in device space, `[left, top, right, bottom]`, `area` pixels); `None` where
    /// that ran past [`TESTS_PER_UNIT`] comparisons per edge and pixel.
    ///
    /// A subpath that stays outside the region adds the same winding to every point of it,
    /// whatever it is, so it neither moves a pixel's values apart nor needs asking about.
    pub(super) fn of(subpaths: &'a [Polyline], region: Box2, area: usize) -> Option<Self> {
        // The sweep's buffers are the thread's, reused fill after fill: most fills are a
        // few dozen edges, and four allocations each were a measurable part of asking.
        SWEEP.with(|cell| {
            let mut sweep = cell.take();
            let topology = Self::swept(subpaths, region, area, &mut sweep);
            cell.replace(sweep);
            topology
        })
    }

    /// [`Topology::of`] with the thread's buffers in hand.
    #[expect(clippy::arithmetic_side_effects)] // `i + 1` below the subpath's length
    fn swept(
        subpaths: &'a [Polyline],
        region: Box2,
        area: usize,
        sweep: &mut Sweep,
    ) -> Option<Self> {
        let Sweep {
            edges,
            boxes,
            order,
            active,
        } = sweep;
        edges.clear();
        boxes.clear();
        let mut near = Vec::new();
        for (s, polyline) in subpaths.iter().enumerate() {
            let Some((x0, y0, x1, y1)) = polyline_bounds(std::slice::from_ref(polyline)) else {
                continue;
            };
            if x1 < region[0] || y1 < region[1] || x0 > region[2] || y0 > region[3] {
                continue;
            }
            near.push(u32::try_from(s).ok()?);
            let n = polyline.points.len();
            for i in 0..n {
                let (p, q) = (polyline.points[i], polyline.points[(i + 1) % n]);
                edges.push(Swept {
                    subpath: u32::try_from(s).ok()?,
                    edge: u32::try_from(i).ok()?,
                    ends: (p, q),
                });
                boxes.push([p.x.min(q.x), p.y.min(q.y), p.x.max(q.x), p.y.max(q.y)]);
            }
        }
        let mut topology = Self {
            subpaths,
            near,
            untrusted: vec![false; subpaths.len()],
            crossing: Vec::new(),
            shape: vec![None; subpaths.len()],
            holds: Vec::new(),
        };
        let budget = TESTS_PER_UNIT.saturating_mul(edges.len().saturating_add(area));
        let complete = meeting_pairs(boxes, budget, (order, active), |a, b| {
            topology.pair(edges[a], edges[b]);
            true
        });
        if !complete {
            return None;
        }
        topology.crossing.sort_unstable();
        topology.crossing.dedup();
        Some(topology)
    }

    /// One pair of edges whose boxes meet: two of one subpath that meet without being
    /// neighbours along it make it cross itself; two of different subpaths that cross make
    /// the pair cross.
    fn pair(&mut self, a: Swept, b: Swept) {
        if a.subpath == b.subpath {
            let s = a.subpath as usize;
            if !self.untrusted[s]
                && !neighbours(&self.subpaths[s], a.edge as usize, b.edge as usize)
                && meet(a.ends, b.ends, true)
            {
                self.untrusted[s] = true;
            }
        } else if meet(a.ends, b.ends, false) {
            self.crossing
                .push((a.subpath.min(b.subpath), a.subpath.max(b.subpath)));
        }
    }

    /// Whether the whole fill winds two neighbouring values, so that no pixel needs more
    /// than its integral: no subpath crosses itself or another, and every subpath's
    /// orientation, turned over once per subpath holding it, is the same.
    ///
    /// Only a subpath whose box meets `k`'s can hold `k` or be held by it, so `k`'s depth is
    /// asked among those alone.
    #[expect(clippy::float_cmp)] // orientations are exactly `−1`, `0` or `+1`
    pub(super) fn two_values(&mut self) -> bool {
        if !self.crossing.is_empty() || self.untrusted.iter().any(|&u| u) {
            return false;
        }
        // Each subpath's neighbours: the subpaths whose boxes meet its own.
        let mut boxes = Vec::with_capacity(self.subpaths.len());
        let mut which = Vec::with_capacity(self.subpaths.len());
        for &k in &self.near {
            if let Some((x0, y0, x1, y1)) =
                polyline_bounds(std::slice::from_ref(&self.subpaths[k as usize]))
            {
                boxes.push([x0, y0, x1, y1]);
                which.push(k);
            }
        }
        let mut near: Vec<Vec<u32>> = vec![Vec::new(); boxes.len()];
        let budget = TESTS_PER_UNIT.saturating_mul(boxes.len().saturating_mul(MAX_GROUP));
        let (mut order, mut active) = (Vec::new(), Vec::new());
        let bounded = meeting_pairs(&boxes, budget, (&mut order, &mut active), |a, b| {
            near[a].push(which[b]);
            near[b].push(which[a]);
            near[a].len() <= MAX_GROUP && near[b].len() <= MAX_GROUP
        });
        if !bounded {
            return false;
        }
        let mut group = Vec::new();
        let mut parity: Option<f32> = None;
        for (at, &k) in which.iter().enumerate() {
            group.clear();
            group.push(k);
            group.extend(&near[at]);
            let Some(signed) = self.signed_by_depth(k, &group) else {
                return false;
            };
            if signed == 0.0 {
                continue;
            }
            match parity {
                None => parity = Some(signed),
                Some(p) if p == signed => {}
                Some(_) => return false,
            }
        }
        true
    }

    /// Whether the subpaths `group` — those through one pixel, sorted — wind it two
    /// neighbouring values: each not crossing itself, no two crossing, and their
    /// orientations alternating with their depth among one another.
    #[expect(clippy::arithmetic_side_effects)] // indices below the group's length
    #[expect(clippy::float_cmp)] // orientations are exactly `−1`, `0` or `+1`
    pub(super) fn alternates(&mut self, group: &[u32]) -> bool {
        if group.len() > MAX_GROUP
            || group
                .iter()
                .any(|&s| self.untrusted.get(s as usize).is_none_or(|&u| u))
        {
            return false;
        }
        if group.len() == 1 {
            return true;
        }
        for (i, &a) in group.iter().enumerate() {
            if group[i + 1..]
                .iter()
                .any(|&b| self.crossing.binary_search(&(a, b)).is_ok())
            {
                return false;
            }
        }
        let mut parity: Option<f32> = None;
        for &k in group {
            let Some(signed) = self.signed_by_depth(k, group) else {
                return false;
            };
            if signed == 0.0 {
                continue;
            }
            match parity {
                None => parity = Some(signed),
                Some(p) if p == signed => {}
                Some(_) => return false,
            }
        }
        true
    }

    /// Subpath `k`'s orientation, turned over once for each other subpath of `group`
    /// holding it: `0` for a subpath that bounds no area, `None` where two hold each other.
    fn signed_by_depth(&mut self, k: u32, group: &[u32]) -> Option<f32> {
        let shape = self.shape(k)?;
        if shape.orientation == 0.0 {
            return Some(0.0);
        }
        let mut signed = shape.orientation;
        for &m in group.iter().filter(|&&m| m != k) {
            match self.holds(m, k) {
                // Each holding the other is one region stated twice, whose edges lie on
                // one another where no crossing can be seen: not nested, not apart.
                Some(true) if self.holds(k, m) != Some(false) => return None,
                Some(true) => signed = -signed,
                Some(false) => {}
                None => return None,
            }
        }
        Some(signed)
    }

    /// Subpath `k`'s orientation and inside point, computed once.
    fn shape(&mut self, k: u32) -> Option<Shape> {
        let slot = self.shape.get_mut(k as usize)?;
        if slot.is_none() {
            *slot = Some(shape_of(self.subpaths.get(k as usize)?)?);
        }
        *slot
    }

    /// Whether subpath `m` holds subpath `k`'s inside point, computed once; `None` where
    /// `k` bounds no area to have a point in.
    fn holds(&mut self, m: u32, k: u32) -> Option<bool> {
        let at = match self.holds.binary_search_by_key(&(m, k), |&(pair, _)| pair) {
            Ok(at) => return Some(self.holds[at].1),
            Err(at) => at,
        };
        let point = self.shape(k)?.inside;
        let answer = contains(self.subpaths.get(m as usize)?, point);
        self.holds.insert(at, ((m, k), answer));
        Some(answer)
    }
}

/// The sign of a subpath's area: `+1`, `−1`, or `0` for one that bounds none.
#[expect(clippy::arithmetic_side_effects)] // indices below the length
fn orientation(polyline: &Polyline) -> f32 {
    let n = polyline.points.len();
    let mut twice = 0.0_f64;
    for i in 0..n {
        let (p, q) = (polyline.points[i], polyline.points[(i + 1) % n]);
        twice += f64::from(p.x) * f64::from(q.y) - f64::from(q.x) * f64::from(p.y);
    }
    if twice > 0.0 {
        1.0
    } else if twice < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// A subpath's orientation, by the sign of its area, and a point strictly inside it: a
/// short step inward from the middle of its longest edge. `None` for a subpath of fewer
/// than three points.
///
/// **The point is stated in `f64`, because the step is smaller than an `f32` can resolve
/// where the shape is small and far from the origin.** A hairline's round cap at a page
/// coordinate of a thousand has a longest edge of `0.06` pixels, so the step is `6e-5` —
/// half the `f32` spacing there, `1.2e-4` — and a point rounded to `f32` lands on the edge
/// it steps from, which the stroke's own body shares: the body is then read as holding the
/// cap, the fill as nesting two same-wound subpaths, and every such stroke is walked pixel by
/// pixel for an answer the integral already had (ADR 1397). The vertices are `f32`s, so
/// their `f64` images, the midpoint and the ray test below are exact or within one `f64`
/// rounding of it.
#[expect(clippy::arithmetic_side_effects)]
fn shape_of(polyline: &Polyline) -> Option<Shape> {
    let n = polyline.points.len();
    if n < 3 {
        return None;
    }
    let mut twice = 0.0_f64;
    let (mut longest, mut at) = (0.0_f64, 0);
    for i in 0..n {
        let (p, q) = (polyline.points[i], polyline.points[(i + 1) % n]);
        twice += f64::from(p.x) * f64::from(q.y) - f64::from(q.x) * f64::from(p.y);
        let length = (f64::from(q.x) - f64::from(p.x)).hypot(f64::from(q.y) - f64::from(p.y));
        if length > longest {
            (longest, at) = (length, i);
        }
    }
    let orientation = if twice > 0.0 {
        1.0
    } else if twice < 0.0 {
        -1.0
    } else {
        0.0
    };
    let (p, q) = (polyline.points[at], polyline.points[(at + 1) % n]);
    let (p, q) = (
        (f64::from(p.x), f64::from(p.y)),
        (f64::from(q.x), f64::from(q.y)),
    );
    // The inside is on the left of every edge of a positively oriented subpath (in these
    // coordinates, `(−dy, dx)`), and on the right of a negative one.
    let step = f64::from(orientation) * longest * 1.0e-3;
    let (dx, dy) = ((q.0 - p.0) / longest, (q.1 - p.1) / longest);
    let inside = (0.5 * (p.0 + q.0) - dy * step, 0.5 * (p.1 + q.1) + dx * step);
    Some(Shape {
        orientation,
        inside,
    })
}

/// Whether the simple subpath `polyline` contains `point`, by the parity of a ray's
/// crossings (§8.5.3.3.3's count, which for a subpath that does not cross itself is also
/// §8.5.3.3.2's answer).
#[expect(clippy::arithmetic_side_effects)]
fn contains(polyline: &Polyline, (px, py): (f64, f64)) -> bool {
    let count = polyline.points.len();
    let mut inside = false;
    for i in 0..count {
        let (from, to) = (polyline.points[i], polyline.points[(i + 1) % count]);
        let (x0, y0, x1, y1) = (
            f64::from(from.x),
            f64::from(from.y),
            f64::from(to.x),
            f64::from(to.y),
        );
        if (y0 > py) != (y1 > py) && x0 + (py - y0) * (x1 - x0) / (y1 - y0) > px {
            inside = !inside;
        }
    }
    inside
}

/// Whether edges `a` and `b` of a closed subpath follow one another along it, with at most
/// edges of no length between them — the closing edge back to a start point the subpath
/// already returned to, above all, which every closed curve has.
#[expect(clippy::arithmetic_side_effects)] // indices below the subpath's length
#[expect(clippy::float_cmp)] // exact: an edge of no length, to the bit
fn neighbours(polyline: &Polyline, a: usize, b: usize) -> bool {
    let n = polyline.points.len();
    let point = |i: usize| polyline.points[i % n];
    let empty = |i: usize| {
        let (start, end) = (point(i), point(i + 1));
        start.x == end.x && start.y == end.y
    };
    let follows = |from: usize, to: usize| {
        let mut i = (from + 1) % n;
        for _ in 0..n {
            if i == to {
                return true;
            }
            if !empty(i) {
                return false;
            }
            i = (i + 1) % n;
        }
        false
    };
    a == b || follows(a, b) || follows(b, a)
}

/// Whether two edges meet at a point.
///
/// Two edges on one line bound no area between them and are never a meeting: a slit, or a
/// stem's side stated in two pieces. Otherwise a proper crossing always is. A touch — an
/// end point on the other edge — is one where `touching` asks for it, which is for two
/// edges of one subpath, since a subpath may cross itself through a vertex; between two
/// subpaths only an end point strictly inside the other edge counts, because abutting
/// pieces share their corners to the bit and a shared corner is not a crossing. A
/// crossing between two subpaths at a corner both of them have is therefore not seen, and
/// that is this test's one blind spot.
fn meet((p, q): (Point, Point), (r, s): (Point, Point), touching: bool) -> bool {
    let side = |o: Point, x: Point, y: Point| (x.x - o.x) * (y.y - o.y) - (x.y - o.y) * (y.x - o.x);
    let (d1, d2) = (side(p, q, r), side(p, q, s));
    let (d3, d4) = (side(r, s, p), side(r, s, q));
    if d1 == 0.0 && d2 == 0.0 {
        return false;
    }
    if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
        return true;
    }
    if !(d1 * d2 <= 0.0 && d3 * d4 <= 0.0) {
        return false;
    }
    if touching {
        return true;
    }
    #[expect(clippy::float_cmp)] // exact: abutting pieces share corners to the bit
    let same = |a: Point, b: Point| a.x == b.x && a.y == b.y;
    let shared_corner = same(p, r) || same(p, s) || same(q, r) || same(q, s);
    !shared_corner
}
