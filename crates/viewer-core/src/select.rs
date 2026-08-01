//! Turning points into text positions, and text positions back into shapes.
//!
//! Selection is not in ISO 32000-2. The standard says where a glyph is drawn and what character
//! it stands for; what a person means by dragging across a page is a question about a user
//! interface, and everything here is therefore a *choice*. Each one is written down.
//!
//! What it is built on is not a choice: `Interpretation::text_layer` is one entry per character
//! code with the range of the readback it produced and the quadrilateral it occupies, both
//! derived from §9.4.4's text rendering matrix and Table 122's font metrics (ADR 0118).

use pdf_model::content::Placed;

/// The text position a point selects.
///
/// The nearest character code, and then its near edge: a point in the left half of a glyph means
/// the position *before* it and one in the right half means *after*. That is what makes a drag
/// across a word select the whole word rather than all but its last letter, and it is what every
/// text interface does.
///
/// `None` for a page with no text at all. A point that is nowhere near any glyph still answers —
/// dragging below the last line selects to the end of it, which is what a person dragging off the
/// bottom of a paragraph means.
pub(crate) fn position_at(placed: &[Placed], point: (f32, f32)) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for entry in placed {
        let distance = distance_to(entry, point);
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, position_in(entry, point)));
        }
    }
    best.map(|(_, position)| position)
}

/// Which end of this glyph the point is nearer, as a byte offset into the readback.
///
/// Measured along the glyph's own advance rather than along x, so that rotated and mirrored text
/// behave the same way: the point is projected onto the baseline vector, and the halfway mark
/// along it is what decides.
fn position_in(entry: &Placed, point: (f32, f32)) -> usize {
    let (ox, oy) = (entry.quad[0], entry.quad[1]);
    let (dx, dy) = (entry.quad[2] - ox, entry.quad[3] - oy);
    let length = dx.hypot(dy);
    if length <= 0.0 {
        return entry.span.start;
    }
    let along = ((point.0 - ox) * dx + (point.1 - oy) * dy) / length;
    if along * 2.0 > length {
        entry.span.end
    } else {
        entry.span.start
    }
}

/// How far a point is from a glyph's box, zero inside it.
///
/// The box is a quadrilateral and this measures against its *bounding* rectangle, which is the
/// same thing for every unrotated page and a slight over-reach for a rotated one. A point is
/// being matched to the nearest glyph, so the over-reach costs nothing a person can see.
fn distance_to(entry: &Placed, point: (f32, f32)) -> f32 {
    let quad = entry.quad;
    let (mut min_x, mut min_y) = (quad[0], quad[1]);
    let (mut max_x, mut max_y) = (quad[0], quad[1]);
    for corner in [(quad[2], quad[3]), (quad[4], quad[5]), (quad[6], quad[7])] {
        min_x = min_x.min(corner.0);
        min_y = min_y.min(corner.1);
        max_x = max_x.max(corner.0);
        max_y = max_y.max(corner.1);
    }
    let dx = (min_x - point.0).max(0.0).max(point.0 - max_x);
    let dy = (min_y - point.1).max(0.0).max(point.1 - max_y);
    dx.hypot(dy)
}

/// The shapes covering a range of the readback, merged along each line.
///
/// One quadrilateral per *run* rather than per glyph, and the merge is the reason: a highlight
/// drawn as three hundred abutting rectangles under one alpha shows a seam at every edge, and a
/// host would have to do the merge itself to avoid it. Two glyphs join when their boxes share
/// both baseline corners' y — which is what "on the same line, at the same size" means — and the
/// second begins no further along than the first ends.
pub(crate) fn quads_for(placed: &[Placed], range: (usize, usize)) -> Vec<[f32; 8]> {
    let (from, to) = (range.0.min(range.1), range.0.max(range.1));
    let mut runs: Vec<[f32; 8]> = Vec::new();
    for entry in placed {
        // A code whose readback is entirely outside the selection contributes nothing. A code
        // that reads back as *nothing* — a glyph no `/ToUnicode`, glyph name or `cmap` could
        // name — has an empty span, and it is included when the selection runs across its
        // position: it is ink a person dragged over, and leaving a hole in the highlight where
        // it sits would be saying something false about what is selected.
        let overlaps = entry.span.start < to && entry.span.end > from;
        let unnameable = entry.span.is_empty() && entry.span.start > from && entry.span.start < to;
        if !(overlaps || unnameable) {
            continue;
        }
        match runs.last_mut() {
            Some(last) if joins(*last, entry.quad) => {
                last[2] = entry.quad[2];
                last[3] = entry.quad[3];
                last[4] = entry.quad[4];
                last[5] = entry.quad[5];
            }
            _ => runs.push(entry.quad),
        }
    }
    runs
}

/// Whether a glyph's box continues the run that ends with `run`.
fn joins(run: [f32; 8], quad: [f32; 8]) -> bool {
    let same_line = (run[1] - quad[1]).abs() < 0.01 && (run[7] - quad[7]).abs() < 0.01;
    // No further along than the run already reaches, plus the width of one space: a run that
    // skips a gap is one a person dragged across, and breaking it there would leave the space
    // between two words unhighlighted.
    let gap = quad[0] - run[2];
    let line = (run[7] - run[1]).abs();
    same_line && gap >= -0.01 && gap < line
}
