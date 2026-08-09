//! Turning points into text positions, and text positions back into shapes.
//!
//! Selection is not in ISO 32000-2. The standard says where a glyph is drawn and what character
//! it stands for; what a person means by dragging across a page is a question about a user
//! interface, and everything here is therefore a *choice*. Each one is written down.
//!
//! What it is built on is not a choice: `Interpretation::text_layer` is one entry per character
//! code with the range of the readback it produced and the quadrilateral it occupies, both
//! derived from §9.4.4's text rendering matrix and Table 120's font metrics (ADR 0118).

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

/// Where a string occurs in the page's readback, as ranges of it.
///
/// **Case-insensitively, and that is the only judgement in it.** A person searching for "the"
/// means "The" as well, which every search interface in existence agrees about; anything beyond
/// that — accent folding, ligature equivalence, the Unicode collation algorithm's tailorings — is
/// a decision about a language rather than about a page, and the readback is what §9.10.2's three
/// methods produced rather than normalised text. `char::to_lowercase` is Unicode's own simple
/// mapping and is what "the same letter" means here.
///
/// Overlapping matches are not reported: after a match the scan continues past it, so "aa" in
/// "aaa" is one match rather than two. That is what a person pressing *next* expects.
///
/// **A space in the needle matches whatever separated the words on the page**, which is the
/// second judgement and the one with a derivation rather than a convention behind it. Neither
/// space nor line break is in the file: `content.rs`'s `separate_text` *infers* both from where
/// §9.4.4's text rendering matrix put the next glyph — "[a] content stream has no notion of words
/// or lines; it has positions" — so a phrase matched against a literal `' '` would be matched
/// against this crate's own inference, and "transparency group" broken across a line would not be
/// found on a page that plainly says it. One or more whitespace characters therefore satisfy one
/// or more spaces in the needle. A single word is unaffected, which is why the case-folding test
/// below still reads the same.
///
/// **Whole-word matching is deliberately not done**, and the reason is the same sentence from the
/// other side: `tests/text_extraction.rs` compares words with all whitespace *removed* because
/// "[w]ord boundaries are deliberately not compared, because a content stream does not record
/// them". A reader that required a boundary would be requiring one this tree reconstructs by
/// heuristic, so "the" finds the one inside "theme" — which is also what every find bar does.
///
/// The ranges index [`pdf_model::Interpretation::text`], so [`quads_for`] turns each into the
/// shapes to draw over it — which is why search cost nothing beyond this function.
pub(crate) fn find(text: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }
    // Lowered once, and the *byte offsets* of the lowered text are not the original's: one
    // character may lower to several, and a naive search over the lowered string would report
    // ranges that do not exist in the readback. So the scan walks the original's character
    // boundaries and compares from each.
    let needle: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    let mut out = Vec::new();
    let mut from = 0;
    while from < text.len() {
        let Some(rest) = text.get(from..) else {
            // Not a character boundary: step to the next one.
            from = from.saturating_add(1);
            continue;
        };
        match matches_at(rest, &needle) {
            Some(length) => {
                out.push((from, from.saturating_add(length)));
                from = from.saturating_add(length.max(1));
            }
            None => from = from.saturating_add(rest.chars().next().map_or(1, char::len_utf8)),
        }
    }
    out
}

/// The byte length of a case-insensitive match at the start of `text`, if there is one.
///
/// Whitespace is the one place the two sides are not compared character for character: a run of
/// it in the needle stands for a run of it in the text, for the reason [`find`] states.
fn matches_at(text: &str, needle: &[char]) -> Option<usize> {
    let mut wanted = needle.iter().copied().peekable();
    let mut characters = text.chars().peekable();
    let mut length = 0_usize;
    loop {
        if wanted.peek().is_some_and(|want| want.is_whitespace()) {
            let mut separated = false;
            while let Some(character) = characters.peek().copied().filter(|c| c.is_whitespace()) {
                characters.next();
                length = length.saturating_add(character.len_utf8());
                separated = true;
            }
            if !separated {
                return None;
            }
            while wanted.peek().is_some_and(|want| want.is_whitespace()) {
                wanted.next();
            }
            if wanted.peek().is_none() {
                return Some(length);
            }
            continue;
        }
        let character = characters.next()?;
        for have in character.to_lowercase() {
            match wanted.peek() {
                Some(want) if *want == have => {
                    wanted.next();
                }
                _ => return None,
            }
        }
        length = length.saturating_add(character.len_utf8());
        if wanted.peek().is_none() {
            return Some(length);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{find, quads_for};
    use pdf_model::content::Placed;

    /// One line of an OCR layer: six glyph boxes, each `width` wide and `height` tall, abutting.
    fn line(width: f32, height: f32) -> Vec<Placed> {
        (0..6_usize)
            .map(|index| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "test code: six boxes, and the index is exact in f32"
                )]
                let left = index as f32 * width;
                Placed {
                    span: index..index.saturating_add(1),
                    quad: [
                        left,
                        0.0,
                        left + width,
                        0.0,
                        left + width,
                        height,
                        left,
                        height,
                    ],
                }
            })
            .collect()
    }

    /// A run of glyphs on one line becomes one shape, and it needs the boxes to have a height.
    ///
    /// The merge in [`super::joins`] measures the gap it will step over against the *line's own
    /// height*, so a layer whose boxes had no height would come back as one shape per glyph — a
    /// highlight of six abutting rectangles under one alpha, with a seam at every edge, which is
    /// the thing the merge exists to prevent. That makes this a consumer-side witness for
    /// `pdf_font::measured_extent`: the band that keeps a font descriptor from stating a
    /// zero-height line is what keeps this merge working (ADR 0216).
    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "test code: the fixture's coordinates are whole numbers, exactly representable, \
                  and the merge is expected to carry them across unchanged"
    )]
    fn one_line_of_an_ocr_layer_is_one_shape() {
        let merged = quads_for(&line(10.0, 12.0), (0, 6));
        assert_eq!(merged.len(), 1, "one run: {merged:?}");
        assert_eq!(merged[0][0], 0.0, "from the first glyph's left edge");
        assert_eq!(merged[0][2], 60.0, "to the last one's right");
        assert_eq!(merged[0][5], 12.0, "with the line's height");

        let slivers = quads_for(&line(10.0, 0.0), (0, 6));
        assert_eq!(
            slivers.len(),
            6,
            "a zero-height line cannot merge, which is why the extent has a band"
        );
    }

    /// Case folding, overlap and the character-boundary trap in one place.
    ///
    /// The last is the one worth a test: one character may lower to *several*, so a search that
    /// lowered the whole string and reported the lowered string's offsets would hand back ranges
    /// that do not exist in the readback. `İ` (U+0130) lowers to two code points, and a range
    /// taken from the lowered text would be off by one byte for everything after it.
    #[test]
    fn a_match_is_a_range_of_the_text_that_was_searched() {
        assert_eq!(find("The theme", "the"), vec![(0, 3), (4, 7)]);
        assert_eq!(find("aaa", "aa"), vec![(0, 2)], "no overlapping matches");
        assert_eq!(find("abc", ""), vec![]);
        assert_eq!(find("", "a"), vec![]);

        let text = "\u{130}stanbul, then";
        let found = find(text, "then");
        assert_eq!(found.len(), 1);
        let (from, to) = found[0];
        assert_eq!(&text[from..to], "then", "the range indexes the original");
    }

    /// A phrase whose words the page put on two lines is still that phrase.
    ///
    /// The readback's separators are `content.rs`'s *inference* from where §9.4.4's matrix put
    /// the next glyph — a newline across the line and a space along it — so a needle's space has
    /// to stand for either, or a search would be querying this crate's line-breaking rather than
    /// the page. The last case is the guard on the other side: a space in the needle still
    /// requires a separator, so two words run together are not two words.
    #[test]
    fn a_needles_space_matches_whatever_separated_the_words_on_the_page() {
        let text = "the transparency\ngroup and the transparency group";
        let found = find(text, "transparency group");
        assert_eq!(found.len(), 2, "both, one of them across a line: {found:?}");
        assert_eq!(&text[found[0].0..found[0].1], "transparency\ngroup");
        assert_eq!(&text[found[1].0..found[1].1], "transparency group");

        assert_eq!(
            find("a  b", "a b").len(),
            1,
            "a run of spaces is one separator"
        );
        assert_eq!(
            find("ab", "a b"),
            vec![],
            "and a separator is still required"
        );
    }
}
