//! Which pixels need more than their integral, where the fill as a whole does (ADR 1389).
//!
//! [`fill_mask`](super::fill_mask) integrates the winding over each pixel and applies
//! §8.5.3.3's rule to the average. That is the area of the inside set exactly when the
//! winding inside the pixel takes at most two neighbouring values: the rule is linear
//! between them, and the average's image is the inside area. [`topology`](super::topology)
//! answers that for the whole fill where it can. Where it cannot — a subpath crosses itself
//! or another, or two nest wound the same way — this module walks the fill's edges once
//! more, records the pixels each passes through, and asks the same rule of the subpaths
//! through each pixel: a pixel only one subpath passes through, where that subpath does not
//! cross itself, winds two neighbouring values whatever the rest of the fill does, and so
//! does one whose subpaths nest in alternation. What is left is **complex**, and
//! [`exact`](super::exact) recomputes it.
//!
//! The test is conservative in one direction only. A pixel called simple is one where the
//! integral is exact; a pixel called complex may still be one, and costs a recomputation
//! that agrees with the integral, nothing more.

use super::super::flatten::Polyline;
use super::topology::Topology;
use super::{Edge, local_edge};

/// The most column spans recorded for one fill. Past it the marks stop, and no pixel of
/// that fill is recomputed: the integral stands, which is the answer every pixel had before
/// ADR 1389 — too much ink where same-wound portions overlap, too little where opposed
/// ones do. At 20 bytes a span this is 20 MiB, beside an accumulation grid that for a
/// region this busy is larger; no page the corpus holds comes near it.
const MAX_SPANS: usize = 1 << 20;

/// The pixels of one row one edge passes through, columns `from..=to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    row: u32,
    from: u32,
    to: u32,
    subpath: u32,
    edge: u32,
}

/// The pixels each edge of a fill passes through, recorded as the fill walks its edges.
#[derive(Debug, Default)]
pub(super) struct Marks {
    spans: Vec<Span>,
    overflowed: bool,
}

/// A run of complex pixels, `row`, columns `from..=to`, and the edges that pass through
/// every one of them: `edges` indexes [`Complex::edges`]'s pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Run {
    pub row: u32,
    pub from: u32,
    pub to: u32,
    pub edges: std::ops::Range<usize>,
}

/// The complex runs of a fill, and the pool of `(subpath, edge)` pairs their ranges index.
#[derive(Debug, Default)]
pub(super) struct Complex {
    pub runs: Vec<Run>,
    pub edges: Vec<(u32, u32)>,
}

impl Marks {
    /// The pixels every edge of the fill passes through, over the region at `origin` of
    /// `size` pixels: each edge's rows as the accumulation cuts them, and the one row a
    /// flat edge lies in — it deposits nothing, but the winding differs on its two sides.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // rows in range
    #[expect(clippy::cast_precision_loss)] // region dims are bounded by target limits
    pub(super) fn walk(polylines: &[Polyline], origin: (i32, i32), size: (usize, usize)) -> Self {
        let (w, h) = size;
        let fh = h as f32;
        let mut marks = Self::default();
        for (index, polyline) in polylines.iter().enumerate() {
            for i in 0..polyline.points.len() {
                let (x0, y0, x1, y1) = local_edge(polyline, i, origin.0, origin.1);
                if let Some(edge) = Edge::cut(x0, y0, x1, y1, fh) {
                    let mut y = edge.top_y.floor().max(0.0);
                    while y < edge.bot_y {
                        let (entry, exit) = (edge.top_y.max(y), edge.bot_y.min(y + 1.0));
                        marks.record(
                            (index, i),
                            y as usize,
                            (edge.x_at(entry), edge.x_at(exit)),
                            w,
                        );
                        y += 1.0;
                    }
                } else if y0.to_bits() == y1.to_bits() && y0 >= 0.0 && y0 < fh {
                    marks.record((index, i), y0 as usize, (x0, x1), w);
                }
            }
        }
        marks
    }

    /// Record that edge `edge` of subpath `subpath` passes through row `row` between `xa`
    /// and `xb`, in a region `w` pixels wide.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // floors in range
    #[expect(clippy::cast_precision_loss)] // region dims are bounded by target limits
    pub(super) fn record(
        &mut self,
        (subpath, edge): (usize, usize),
        row: usize,
        (xa, xb): (f32, f32),
        w: usize,
    ) {
        let (lo, hi) = (xa.min(xb).floor(), xa.max(xb).floor());
        if self.overflowed || hi < 0.0 || lo >= w as f32 {
            return;
        }
        let (Ok(row), Ok(subpath), Ok(edge)) = (
            u32::try_from(row),
            u32::try_from(subpath),
            u32::try_from(edge),
        ) else {
            self.overflowed = true;
            return;
        };
        if self.spans.len() >= MAX_SPANS {
            self.overflowed = true;
            return;
        }
        self.spans.push(Span {
            row,
            from: lo.max(0.0) as u32,
            to: hi.min(w as f32 - 1.0) as u32,
            subpath,
            edge,
        });
    }

    /// The complex pixels, as runs in row order. Empty when the marks overflowed, which
    /// leaves the integral standing everywhere.
    ///
    /// The spans are grouped by row with a counting pass rather than sorted whole: a row
    /// holds a handful of spans, and a row where no two share a pixel is settled by one
    /// look at it.
    #[expect(clippy::arithmetic_side_effects)] // counts below the span count
    pub(super) fn complex_pixels(self, topology: &mut Topology<'_>, h: usize) -> Complex {
        if self.overflowed || self.spans.len() < 2 {
            return Complex::default();
        }
        let mut starts = vec![0_usize; h + 1];
        for span in &self.spans {
            if let Some(count) = starts.get_mut(span.row as usize + 1) {
                *count += 1;
            }
        }
        for row in 0..h {
            starts[row + 1] += starts[row];
        }
        let mut placed = starts.clone();
        let mut by_row = vec![self.spans[0]; self.spans.len()];
        for span in &self.spans {
            if let Some(at) = placed.get_mut(span.row as usize) {
                by_row[*at] = *span;
                *at += 1;
            }
        }
        let mut sweep = Stretches::default();
        for row in 0..h {
            let spans = &mut by_row[starts[row]..starts[row + 1]];
            if spans.len() < 2 {
                continue;
            }
            spans.sort_unstable_by_key(|s| s.from);
            let mut reach = spans[0].to;
            let meet = spans[1..].iter().any(|s| {
                let meets = s.from <= reach;
                reach = reach.max(s.to);
                meets
            });
            if meet {
                sweep.row(spans, topology);
            }
        }
        sweep.complex
    }
}

/// One fill's walk over its rows' stretches, with the buffers the rows reuse.
#[derive(Debug, Default)]
struct Stretches {
    complex: Complex,
    cuts: Vec<u32>,
    active: Vec<Span>,
    group: Vec<u32>,
}

impl Stretches {
    /// One row, given its spans sorted by first column: the columns where the set of
    /// spans covering them changes, the set asked once per stretch.
    #[expect(clippy::arithmetic_side_effects)] // columns below the region's width
    fn row(&mut self, spans: &[Span], topology: &mut Topology<'_>) {
        self.cuts.clear();
        self.cuts
            .extend(spans.iter().flat_map(|s| [s.from, s.to.saturating_add(1)]));
        self.cuts.sort_unstable();
        self.cuts.dedup();
        self.active.clear();
        let mut next = 0;
        for index in 1..self.cuts.len() {
            let (from, to) = (self.cuts[index - 1], self.cuts[index] - 1);
            self.active.retain(|s| s.to >= from);
            while next < spans.len() && spans[next].from <= from {
                self.active.push(spans[next]);
                next += 1;
            }
            if self.active.len() < 2 {
                continue;
            }
            self.group.clear();
            self.group.extend(self.active.iter().map(|s| s.subpath));
            self.group.sort_unstable();
            self.group.dedup();
            if topology.alternates(&self.group) {
                continue;
            }
            let edges = &mut self.complex.edges;
            let begin = edges.len();
            edges.extend(self.active.iter().map(|s| (s.subpath, s.edge)));
            self.complex.runs.push(Run {
                row: spans[0].row,
                from,
                to,
                edges: begin..edges.len(),
            });
        }
    }
}
