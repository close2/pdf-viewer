//! Turning points into text positions, and text positions back into shapes.
//!
//! Selection is not in ISO 32000-2. The standard says where a glyph is drawn and what character
//! it stands for; what a person means by dragging across a page is a question about a user
//! interface, and everything here is therefore a *choice*. Each one is written down.
//!
//! What it is built on is not a choice: `Interpretation::text_layer` is one entry per character
//! code with the range of the readback it produced and the quadrilateral it occupies, both
//! derived from §9.4.4's text rendering matrix and Table 120's font metrics (ADR 0118).

use std::ops::Range;

use pdf_font::shaping;
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

/// The same range of the readback, **unmerged** and broken into the lines it was drawn on.
///
/// [`quads_for`]'s population, kept one entry per character code instead of one per run, and
/// grouped by [`continues`] rather than merged by [`joins`]. A highlight wants the merged shapes;
/// a caret wants the opposite — where each character begins, how wide it is, where one line ends —
/// which is what AT-SPI's `org.a11y.atspi.Text` and every other platform's text interface ask
/// for, and what §14.8.2.5's logical order is worth reading in the first place.
///
/// **A code that reads back as nothing is left out**, which is where this and [`quads_for`]
/// deliberately differ. There it is ink a person dragged across and a hole in the highlight would
/// be a lie; here it would be a character of zero bytes, and a caret cannot stand on one — a
/// platform's arrays are indexed by character and a zero-length entry is a position that can be
/// reached and never left. The glyph is still drawn and is still in the selection; what it is not
/// is somewhere the caret can be.
///
/// Each entry is the code's range of the readback and its quadrilateral in the display list's own
/// space, in the order the page drew them — which for a line of text is reading order and for a
/// producer that wrote its glyphs out of order is not. Ordering by position is deliberately not
/// done here: the readback's own order is what §14.8.2.5 makes the logical one.
///
/// **All of the caller's ranges at once, rather than one call each**, and that is not an
/// optimisation. A structure element's own content items are one range per §14.7.5.2 sequence, and
/// a producer that opened a new sequence in the middle of a line — which is what a `Span` inside a
/// paragraph is — would otherwise have that line broken at the sequence boundary, into as many
/// lines as it has pieces. What decides a line is where the glyphs landed, and that is a question
/// about the whole element.
pub(crate) fn lines_for(
    placed: &[Placed],
    ranges: &[(usize, usize)],
) -> Vec<Vec<(Range<usize>, [f32; 8])>> {
    let mut lines: Vec<Vec<(Range<usize>, [f32; 8])>> = Vec::new();
    let mut last: Option<[f32; 8]> = None;
    for entry in placed {
        let within = ranges.iter().any(|(from, to)| {
            let (low, high) = (from.min(to), from.max(to));
            entry.span.start < *high && entry.span.end > *low
        });
        if entry.span.is_empty() || !within {
            continue;
        }
        match (last, lines.last_mut()) {
            (Some(previous), Some(line)) if continues(previous, entry.quad) => {
                line.push((entry.span.clone(), entry.quad));
            }
            _ => lines.push(vec![(entry.span.clone(), entry.quad)]),
        }
        last = Some(entry.quad);
    }
    lines
}

/// Whether a glyph continues the *line* the one before it is on.
///
/// [`joins`]'s question with the tolerances a **line** needs rather than the ones a highlight
/// needs, and the difference was measured rather than assumed. Under `joins`, ISO 32000-2's cover
/// answers `In`, `terna`, `tiona`, `l `, `Sta`, `nda`, `rd ` where the page says *International
/// Standard*: its display face is tracked, so glyph boxes overlap their neighbours by a fraction
/// of an em, and `joins` ends a run at the first overlap over a hundredth of a unit. A highlight
/// does not care — the two rectangles abut and a person sees one band — and a caret does: those
/// are seven lines to move through where a person sees one.
///
/// So the two conditions are stated against the glyph's own height, which is the only length
/// available that scales with the text:
///
/// - **the same baseline** within a twentieth of the height, rather than exactly. Two glyphs of
///   one line set in faces of slightly different metrics do not share a corner to the last bit.
/// - **no further apart than one height, and no further overlapped than half of one.** A gap of a
///   height is the widest inter-word space that is still a space; an overlap of half a glyph is
///   more than tracking and more than kerning, and is text drawn over text.
///
/// It is deliberately not [`joins`] with looser numbers: a selection's merge is a statement about
/// what a person dragged over, and this is a statement about where a caret may stop. Changing the
/// first to suit the second would move a feature nobody was measuring.
fn continues(previous: [f32; 8], quad: [f32; 8]) -> bool {
    let height = (previous[7] - previous[1]).abs().max(f32::EPSILON);
    let baseline = (previous[1] - quad[1]).abs() <= height / 20.0;
    let gap = quad[0] - previous[2];
    baseline && gap > -height / 2.0 && gap < height
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
/// **Case-insensitively, and with presentation forms read as their letters.** A person searching
/// for "the" means "The" as well, which every search interface in existence agrees about.
/// `char::to_lowercase` is Unicode's own simple mapping and is what "the same letter" means here.
/// A presentation form — an Arabic letter's initial, medial, final or isolated form, a lam-alef
/// or Latin `ﬁ` ligature — is compared as the characters `UnicodeData.txt` decomposes it to
/// ([`pdf_font::shaping::fold`]), because §9.10.2 hands a `/ToUnicode` that names the forms
/// through as stated and nobody types them (ADR 1465). Every character is compared as its
/// canonical decomposition ([`pdf_font::shaping::decompose`]), so `é` and `e` with U+0301 are the
/// one text the Unicode Standard says they are.
///
/// **A mark the needle leaves off is not asked for; a mark it states is**, which is the fifth
/// judgement and one rule for every script: "كتب" finds "كَتَبَ", "שלום" finds "שָׁלוֹם" and "cafe"
/// finds "café", while "كَتَبَ" does not find "كتب" and "café" does not find "cafe". A mark is a
/// nonspacing mark with a canonical combining class ([`pdf_font::shaping::mark_class`]); harakat
/// and points are what a writer of Arabic and Hebrew leaves off, and a search that required them
/// would find nothing a person types. Beyond that — the Unicode collation algorithm's tailorings,
/// a language's own equivalences — is a decision about a language rather than about a page, and
/// the readback is what §9.10.2's methods produced rather than normalised text (ADR 1477).
///
/// **A right-to-left word is looked for in the order the page stored it**, which is the fourth
/// judgement and the subject of [`Order`] and [`spellings`]: the needle is typed in reading order,
/// and a page that shows Arabic with show strings "whose character codes are given in reverse
/// order" (§14.8.2.5.3 NOTE 1) stored the word in display order. So the needle is also spelled
/// as Unicode Standard Annex #9 displays it, and each spelling is admitted only over the runs the
/// glyphs' positions say were stored that way.
///
/// Overlapping matches are not reported: after a match the scan continues past it, so "aa" in
/// "aaa" is one match rather than two. That is what a person pressing *next* expects.
///
/// **A space in the needle matches whatever separated the words on the page**, which is the
/// second judgement and the one with a derivation rather than a convention behind it. A word
/// break reaches the readback by one of two quite different routes, and only one of them is in
/// the file: §9.3.3's single-byte code 32, which the clause names "the ASCII SPACE character
/// (20h)" and which `content.rs`'s `Font::text` reads as one; or `separate_text`'s *inference*
/// from where §9.4.4's text rendering matrix put the next glyph — "[a] content stream has no
/// notion of words or lines; it has positions". A line break is always the second. So a phrase
/// matched against a literal `' '` would sometimes be matched against this crate's own inference,
/// and "transparency group" broken across a line would not be found on a page that plainly says
/// it. One or more whitespace characters therefore satisfy one or more spaces in the needle. A single word is unaffected, which is why the case-folding test
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
pub(crate) fn find(text: &str, needle: &str, order: &Order) -> Vec<(usize, usize)> {
    // Folded once, and the *byte offsets* of the folded text are not the original's: one
    // character may lower or decompose to several, and a naive search over a folded string would
    // report ranges that do not exist in the readback. So the scan walks the original's character
    // boundaries and compares from each.
    let mut out = Vec::new();
    let mut scratch = Scratch::default();
    for spelling in spellings(needle) {
        let mut from = 0;
        while from < text.len() {
            let Some(rest) = text.get(from..) else {
                // Not a character boundary: step to the next one.
                from = from.saturating_add(1);
                continue;
            };
            let found = matches_at(rest, from, &spelling.characters, order, &mut scratch).filter(
                |length| spelling.admitted(text, from..from.saturating_add(*length), order),
            );
            match found {
                Some(length) => {
                    out.push((from, from.saturating_add(length)));
                    from = from.saturating_add(length.max(1));
                }
                None => from = from.saturating_add(rest.chars().next().map_or(1, char::len_utf8)),
            }
        }
    }
    // Two spellings may find the same characters — a word of one letter reads the same both ways
    // — and a person pressing *next* expects each place once. Earliest first, and of two that
    // start together the longer.
    out.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    let mut kept: Vec<(usize, usize)> = Vec::with_capacity(out.len());
    for found in out {
        if kept.last().is_none_or(|last| found.0 >= last.1) {
            kept.push(found);
        }
    }
    kept
}

/// One order of a needle's characters, and which stored orders it may be matched against.
#[derive(Debug, PartialEq, Eq)]
struct Spelling {
    /// The needle, folded and lowered, in this order.
    characters: Vec<char>,
    /// Whether a right-to-left run the page stored in reading order may hold it.
    reading: bool,
    /// Whether a run the page stored in display order, left to right, may hold it.
    display: bool,
}

impl Spelling {
    /// Whether a match of this spelling over `range` sits where the page stored its order.
    ///
    /// Only the right-to-left characters are asked: a digit or a Latin word inside an Arabic line
    /// is stored left to right in both orders, which is Unicode Standard Annex #9's rule L2 and
    /// why the display spelling leaves them as they were typed. A run the positions could not
    /// decide — one glyph, or glyphs on top of each other — admits either.
    fn admitted(&self, text: &str, range: Range<usize>, order: &Order) -> bool {
        let start = range.start;
        text.get(range)
            .unwrap_or_default()
            .char_indices()
            .filter(|(_, character)| shaping::right_to_left(*character))
            .all(|(at, _)| match order.stored(start.saturating_add(at)) {
                Some(Stored::Display) => self.display,
                Some(Stored::Reading) => self.reading,
                None => true,
            })
    }
}

/// The orders a needle is looked for in: as typed, and as Unicode Standard Annex #9 displays it.
///
/// A needle is typed in reading order. A page whose producer laid Arabic out itself stored it in
/// the order it is *displayed*, left to right, and that is the order its readback is in — so the
/// needle is spelled the same way, by the annex's rule L2 over the levels its resolution gives,
/// which reverses the right-to-left letters and leaves an embedded number or Latin word reading
/// left to right inside them. **Under both paragraph directions**, because the direction of the
/// line a word sat in is a fact about the page the needle cannot know: "عربي PDF" displays as
/// `PDF يبرع` in a right-to-left paragraph and as `يبرع PDF` in a left-to-right one, and an
/// untagged page states neither. A needle with no right-to-left character has one spelling.
///
/// Folded first — a presentation form typed into the bar is its letters, as on the page, and a
/// precomposed letter its canonical decomposition — and lowered, which is [`find`]'s first
/// judgement.
fn spellings(needle: &str) -> Vec<Spelling> {
    let mut letters = Vec::new();
    let mut spare = Vec::new();
    for character in needle.chars() {
        compared_as(character, false, &mut spare, &mut letters);
    }
    let folded: String = letters.into_iter().collect();
    if folded.is_empty() {
        return Vec::new();
    }
    let typed: Vec<char> = folded.chars().collect();
    let right_to_left = typed.iter().copied().any(shaping::right_to_left);
    let mut out = vec![Spelling {
        characters: typed.clone(),
        reading: true,
        display: !right_to_left,
    }];
    if !right_to_left {
        return out;
    }
    for paragraph in [false, true] {
        let Some(paragraphs) = shaping::Paragraphs::with_direction(&folded, Some(paragraph)) else {
            continue;
        };
        let levels = paragraphs.line_levels(0..folded.len());
        let by_character: Vec<u8> = folded
            .char_indices()
            .map(|(at, _)| levels.get(at).copied().unwrap_or(0))
            .collect();
        let characters: Vec<char> = shaping::visual_order(&by_character)
            .into_iter()
            .filter_map(|index| typed.get(index).copied())
            .collect();
        match out
            .iter_mut()
            .find(|spelling| spelling.characters == characters)
        {
            Some(same) => same.display = true,
            None => out.push(Spelling {
                characters,
                reading: false,
                display: true,
            }),
        }
    }
    out
}

/// How a page stored one run of right-to-left characters, as the positions of its glyphs say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stored {
    /// In reading order: each glyph placed to the left of the one before it, or §14.8.2.5.3's
    /// `ReversedChars` read back the right way round.
    Reading,
    /// In display order: each glyph to the right of the one before it, which is the show strings
    /// "whose character codes are given in reverse order" of §14.8.2.5.3 NOTE 1.
    Display,
}

/// Which order the page stored each right-to-left run of its readback in.
///
/// **This is the page's own evidence and nothing else.** §14.8.2.5.1 defines page content order
/// as "the sequencing of graphics objects within a page's content stream", and the readback is
/// in that order; logical content order is the structure tree's, so an untagged page states none.
/// What the page does state is where each glyph landed, and a run of right-to-left letters whose
/// glyphs advance rightwards in the order they were shown was shown in display order. The
/// direction is read off each glyph's own box ([`axes`]) — its ascent side is *up*, and *right*
/// is the quarter turn from it on the side the glyph's own horizontal axis points — so a rotated
/// page, a flipped `cm` and a mirroring text matrix all read as the producer wrote them.
///
/// A run is the right-to-left glyphs of one line, with whatever sits between them; each pair of
/// neighbours votes and the majority decides. A run with no vote is not recorded, and every
/// spelling is admitted over it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Order {
    /// Ranges of the readback, sorted by where they start, and how each was stored.
    runs: Vec<(Range<usize>, Stored)>,
}

impl Order {
    /// The stored order of every right-to-left run `placed` shows of `text`.
    pub(crate) fn of(text: &str, placed: &[Placed]) -> Self {
        /// One run being gathered: its range, its last glyph's box and its running vote.
        struct Run {
            span: Range<usize>,
            last: [f32; 8],
            votes: i64,
        }
        // Every page is asked this as it is drawn, and almost every page is all Latin: a strongly
        // right-to-left character is at or above U+0590, so its UTF-8 lead byte is at or above
        // 0xD6, and a readback with no such byte has no run to find. One pass over the bytes,
        // which the compiler vectorises, keeps the walk below off the launch path (ADR 1465).
        if !text.bytes().any(|byte| byte >= 0xd6) {
            return Self::default();
        }
        let mut runs = Vec::new();
        let finish = |run: Run, runs: &mut Vec<(Range<usize>, Stored)>| match run.votes.signum() {
            1 => runs.push((run.span, Stored::Display)),
            -1 => runs.push((run.span, Stored::Reading)),
            _ => {}
        };
        let mut current: Option<Run> = None;
        for entry in placed {
            let right_to_left = text
                .get(entry.span.clone())
                .is_some_and(|piece| piece.chars().any(shaping::right_to_left));
            if !right_to_left {
                continue;
            }
            match current.as_mut() {
                Some(run) if on_one_line(run.last, entry.quad) => {
                    run.votes = run.votes.saturating_add(vote(run.last, entry.quad));
                    run.span.start = run.span.start.min(entry.span.start);
                    run.span.end = run.span.end.max(entry.span.end);
                    run.last = entry.quad;
                }
                _ => {
                    if let Some(run) = current.take() {
                        finish(run, &mut runs);
                    }
                    current = Some(Run {
                        span: entry.span.clone(),
                        last: entry.quad,
                        votes: 0,
                    });
                }
            }
        }
        if let Some(run) = current {
            finish(run, &mut runs);
        }
        runs.sort_by_key(|(span, _)| span.start);
        Self { runs }
    }

    /// How the run holding byte `at` was stored, where a run holds it and its glyphs decided.
    fn stored(&self, at: usize) -> Option<Stored> {
        let after = self.runs.partition_point(|(span, _)| span.start <= at);
        let (span, stored) = self.runs.get(after.checked_sub(1)?)?;
        span.contains(&at).then_some(*stored)
    }

    /// The bytes this holds beyond its own size, for a cache that counts what it keeps.
    pub(crate) fn heap_bytes(&self) -> usize {
        self.runs
            .len()
            .saturating_mul(size_of::<(Range<usize>, Stored)>())
    }
}

/// A glyph's *up* and *right*, from its box: the ascent side, and the quarter turn of it that
/// text space's horizontal axis points along.
///
/// §9.4.4's text rendering matrix carries the glyph's box from text space, where a glyph extends
/// from its origin along +x and its ascent is along +y, into the display list's space. Under a
/// matrix of positive determinant the picture keeps its handedness and +x is the same quarter turn
/// from +y it is on an upright page; under a negative one — `Tm`, `Tz` or `cm` with one axis
/// negated — the picture is a mirror image, the glyphs advance the other way across the display,
/// and +x is the opposite quarter turn. The box's own corners say which: *right* is the quarter
/// turn of *up* on the side its base (origin to advance corner) points to, which is the sign of
/// the matrix's determinant read off the box, and holds whatever the handedness of the space the
/// box is in. A box of no width says nothing, and is read as unmirrored.
fn axes(quad: [f32; 8]) -> ((f32, f32), (f32, f32)) {
    let up = (quad[6] - quad[0], quad[7] - quad[1]);
    let base = (quad[2] - quad[0], quad[3] - quad[1]);
    let clockwise = (up.1, -up.0);
    if base.0 * clockwise.0 + base.1 * clockwise.1 < 0.0 {
        (up, (-up.1, up.0))
    } else {
        (up, clockwise)
    }
}

/// Whether `quad` stands on the line `previous` stands on: its foot no further off that baseline
/// than a twentieth of the line's height, which is [`continues`]'s tolerance.
fn on_one_line(previous: [f32; 8], quad: [f32; 8]) -> bool {
    let (up, _) = axes(previous);
    let height = up.0.hypot(up.1).max(f32::EPSILON);
    let off = ((quad[0] - previous[0]) * up.0 + (quad[1] - previous[1]) * up.1) / height;
    off.abs() <= height / 20.0
}

/// `1` where `quad` was placed to the right of `previous`, `-1` to its left, `0` on top of it.
///
/// A glyph within a hundredth of the line's height of the one before casts no vote: a mark
/// placed over its letter is neither order.
fn vote(previous: [f32; 8], quad: [f32; 8]) -> i64 {
    let (up, right) = axes(previous);
    let height = up.0.hypot(up.1).max(f32::EPSILON);
    let along = ((quad[0] - previous[0]) * right.0 + (quad[1] - previous[1]) * right.1) / height;
    if along > height / 100.0 {
        1
    } else if along < -height / 100.0 {
        -1
    } else {
        0
    }
}

/// ISO 32000-2 §14.8.2.3's soft hyphen, U+00AD.
///
/// > In tagged PDF, the visible hyphen that is introduced through the incidental division of a
/// > word at the end of a line but which would not be present otherwise, may be represented as a
/// > soft hyphen , mapped to the Unicode value U+00AD
const SOFT_HYPHEN: char = '\u{00ad}';

/// The byte length of a case-insensitive match at the start of `text`, if there is one.
///
/// Whitespace is the one place the two sides are not compared unit for unit: a run of it in the
/// needle stands for a run of it in the text, for the reason [`find`] states.
///
/// # §14.8.2.3's rejoining, which is the third judgement
///
/// A soft hyphen in the *text* is skipped, together with the line break after it, so that a word
/// an incidental line division split is found by its own spelling. The clause states the
/// distinction and its purpose — a writer "shall distinguish explicitly between soft and hard
/// hyphens so that a PDF processor can unambiguously determine which type a given character
/// represents" — and states nothing at all about what the processor then does with the
/// determination, which §14.8.2.2.1 NOTE 3 says outright is not tagged PDF's business: its purpose
/// "is not to prescribe what the PDF processor does". So this is a documented choice and not a
/// clause obeyed, and ADR 1100 is the argument. What makes it *this* choice: the character's whole
/// reason for existing is to say the hyphen "would not be present otherwise", and a find bar that
/// cannot find "transparency" on the page that plainly says it has ignored the one thing the
/// producer went to the trouble of stating.
///
/// **The readback keeps the character.** Nothing here rewrites `Interpretation::text`; the fold is
/// the searcher's, so a caller copying the same range still gets what §9.10.2's mapping produced.
///
/// **The needle is not folded**, which is what lets a person who types a soft hyphen still mean
/// one: the skip below is declined whenever the needle wants that character next.
///
/// **A hard hyphen is untouched.** NOTE 1 makes U+002D a different character, and a reader that
/// folded it would join two words the page keeps apart.
///
/// # Marks, which are the fifth judgement
///
/// Wherever either side has a run of marks ([`shaping::mark_class`]), the two runs are sorted by
/// canonical combining class and the needle's must be found, in order, inside the page's: a mark
/// the needle leaves off is not asked for, and one it states must be there. A match never starts
/// on a mark the needle does not state, and it takes in the marks after its last letter, so the
/// highlight covers the whole of what was printed (ADR 1477).
fn matches_at(
    text: &str,
    offset: usize,
    needle: &[char],
    order: &Order,
    scratch: &mut Scratch,
) -> Option<usize> {
    let mut wanted = needle.iter().copied().peekable();
    let mut page = Cursor::new(text, offset, order, scratch);
    let mut started = false;
    loop {
        // §14.8.2.3, before either branch: the character and the break it introduced are not part
        // of the word, so neither side of the comparison should see them.
        if page.peek() == Some(SOFT_HYPHEN) && wanted.peek() != Some(&SOFT_HYPHEN) {
            page.next();
            while page.peek().is_some_and(char::is_whitespace) {
                page.next();
            }
            continue;
        }
        if wanted.peek().is_some_and(|want| want.is_whitespace()) {
            let mut separated = false;
            while page.peek().is_some_and(char::is_whitespace) {
                page.next();
                separated = true;
            }
            if !separated {
                return None;
            }
            while wanted.peek().is_some_and(|want| want.is_whitespace()) {
                wanted.next();
            }
            if wanted.peek().is_none() {
                return page.finished();
            }
            continue;
        }
        let page_mark = page.peek().and_then(shaping::mark_class).is_some();
        let wanted_mark = wanted
            .peek()
            .copied()
            .and_then(shaping::mark_class)
            .is_some();
        if page_mark || wanted_mark {
            if !started && !wanted_mark {
                return None;
            }
            if !page.marks_hold(&mut wanted) {
                return None;
            }
        } else {
            let have = page.next()?;
            wanted.next_if_eq(&have)?;
        }
        started = true;
        if wanted.peek().is_none() {
            return page.finished();
        }
    }
}

/// The buffers one [`find`] lends every comparison it starts, so that comparing a page character
/// allocates nothing: a search over a cached readback is a few milliseconds for a whole document
/// (ADR 0256), and an allocation per character compared would be most of it.
#[derive(Debug, Default)]
struct Scratch {
    /// What the page character being compared is compared as.
    units: Vec<char>,
    /// One character's canonical decomposition, before it is lowered.
    spare: Vec<char>,
    /// A run of the page's marks, with their classes.
    have: Vec<(u8, char)>,
    /// A run of the needle's marks, with their classes.
    want: Vec<(u8, char)>,
}

/// The page's side of one comparison: its characters as the units they are compared as, and how
/// many bytes of it the units taken so far wholly cover.
struct Cursor<'t, 'o, 's> {
    characters: std::str::CharIndices<'t>,
    /// Where `characters` starts in the readback, which is what [`Order`] is indexed by.
    offset: usize,
    order: &'o Order,
    scratch: &'s mut Scratch,
    /// How many of `scratch.units` have been taken.
    taken: usize,
    /// The byte, relative to the start, where the character `scratch.units` came from ends.
    end: usize,
    /// The bytes of the characters whose every unit has been taken.
    consumed: usize,
}

impl<'t, 'o, 's> Cursor<'t, 'o, 's> {
    fn new(text: &'t str, offset: usize, order: &'o Order, scratch: &'s mut Scratch) -> Self {
        scratch.units.clear();
        Self {
            characters: text.char_indices(),
            offset,
            order,
            scratch,
            taken: 0,
            end: 0,
            consumed: 0,
        }
    }

    /// The next unit, reading the next character when this one's are all taken.
    fn peek(&mut self) -> Option<char> {
        if self.taken == self.scratch.units.len() {
            let (at, character) = self.characters.next()?;
            self.scratch.units.clear();
            self.taken = 0;
            self.end = at.saturating_add(character.len_utf8());
            let displayed = shaping::right_to_left(character)
                && self.order.stored(self.offset.saturating_add(at)) == Some(Stored::Display);
            compared_as(
                character,
                displayed,
                &mut self.scratch.spare,
                &mut self.scratch.units,
            );
        }
        self.scratch.units.get(self.taken).copied()
    }

    fn next(&mut self) -> Option<char> {
        let unit = self.peek()?;
        self.taken = self.taken.saturating_add(1);
        if self.taken == self.scratch.units.len() {
            self.consumed = self.end;
        }
        Some(unit)
    }

    /// Takes the page's run of marks here and the needle's, and says whether the needle's sorted
    /// run is found in order inside the page's sorted one.
    fn marks_hold(&mut self, wanted: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> bool {
        self.scratch.have.clear();
        while let Some(unit) = self.peek() {
            let Some(class) = shaping::mark_class(unit) else {
                break;
            };
            self.scratch.have.push((class, unit));
            self.next();
        }
        let want = &mut self.scratch.want;
        want.clear();
        while let Some(class) = wanted.peek().copied().and_then(shaping::mark_class) {
            want.extend(wanted.next().map(|mark| (class, mark)));
        }
        // Stable, so marks of one class keep the order they were stored in: canonical ordering
        // reorders only across classes.
        self.scratch.have.sort_by_key(|(class, _)| *class);
        want.sort_by_key(|(class, _)| *class);
        let mut have = self.scratch.have.iter();
        want.iter().all(|mark| have.any(|held| held == mark))
    }

    /// The match's length, once the needle is spent: the marks after its last letter taken in,
    /// and refused where a character was compared only in part — a ligature whose second letter
    /// the needle did not want.
    fn finished(&mut self) -> Option<usize> {
        while self.peek().and_then(shaping::mark_class).is_some() {
            self.next();
        }
        (self.taken == 0 || self.taken == self.scratch.units.len()).then_some(self.consumed)
    }
}

/// Pushes onto `into` the units one character is compared as: a presentation form's letters
/// (§9.10.2 hands the forms through and nobody types them) in the order its run was stored — in a
/// run stored in display order a lam-alef's alef is on the left, so it comes first — and each
/// letter's canonical decomposition, lowered ([`find`]'s first judgement).
fn compared_as(character: char, displayed: bool, spare: &mut Vec<char>, into: &mut Vec<char>) {
    if character.is_ascii() {
        into.push(character.to_ascii_lowercase());
        return;
    }
    match shaping::fold(character) {
        Some(letters) if displayed => {
            for letter in letters.iter().rev() {
                lowered(*letter, spare, into);
            }
        }
        Some(letters) => {
            for letter in letters {
                lowered(*letter, spare, into);
            }
        }
        None => lowered(character, spare, into),
    }
}

/// `letter`'s canonical decomposition, lowered, and decomposed again where lowering composed.
fn lowered(letter: char, spare: &mut Vec<char>, into: &mut Vec<char>) {
    spare.clear();
    shaping::decompose(letter, spare);
    for part in spare.iter() {
        for lower in part.to_lowercase() {
            shaping::decompose(lower, into);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Order, find as find_ordered, quads_for};
    use pdf_model::content::Placed;

    /// [`super::find`] over a page whose glyph positions decided nothing.
    fn find(text: &str, needle: &str) -> Vec<(usize, usize)> {
        find_ordered(text, needle, &Order::default())
    }

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

    /// ISO 32000-2 §14.8.2.3's soft hyphen: a word a line division split is found by its spelling.
    ///
    /// The clause's own subject is the character — "the visible hyphen that is introduced through
    /// the incidental division of a word at the end of a line but which would not be present
    /// otherwise, may be represented as a soft hyphen , mapped to the Unicode value U+00AD" — and
    /// the rejoining is this crate's decision rather than a requirement it states (ADR 1100).
    ///
    /// Four cases, and the last three are the calibration (`doc/traps/instruments-and-reports.md`
    /// trap 13): a hard hyphen is **not** folded, a needle that states a soft hyphen still wants
    /// one, and a soft hyphen does not turn two separate words into one match.
    #[test]
    fn a_word_a_soft_hyphen_divided_is_found_whole() {
        assert_eq!(
            find("transparen\u{ad}\ncy group", "transparency"),
            vec![(0, 15)],
            "the character and the break it introduced are not part of the word, and the range \
             still covers both halves so that a highlight shows the whole of what matched"
        );
        assert_eq!(
            find("well\u{2d}\nknown", "wellknown"),
            vec![],
            "NOTE 1's hard hyphen is a different character and stays"
        );
        assert_eq!(
            find("soft\u{ad}ly", "soft\u{ad}ly"),
            vec![(0, 8)],
            "a needle that states the character is compared against it"
        );
        assert_eq!(
            find("red\u{ad}\ngreen", "redgreenblue"),
            vec![],
            "folding joins what the division split, and invents nothing"
        );
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

    /// A page's readback with one glyph per character, ten units wide, on one line: placed left
    /// to right in the order shown when `display` is true, right to left otherwise.
    fn page(text: &str, display: bool) -> (String, Order) {
        let count = text.chars().count();
        let placed: Vec<Placed> = text
            .char_indices()
            .enumerate()
            .map(|(index, (at, character))| {
                let slot = if display {
                    index
                } else {
                    count.saturating_sub(1).saturating_sub(index)
                };
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "test code: a few dozen glyphs, exact in f32"
                )]
                let left = slot as f32 * 10.0;
                Placed {
                    span: at..at.saturating_add(character.len_utf8()),
                    quad: [left, 0.0, left + 10.0, 0.0, left + 10.0, 12.0, left, 12.0],
                }
            })
            .collect();
        (text.to_owned(), Order::of(text, &placed))
    }

    /// `ArabicCIDTrueType.pdf`'s own shape, by hand: "العربية" shown left to right as the
    /// presentation forms its `/ToUnicode` names, so the readback is `ﺔﻴﺑﺮﻌﻟا`.
    ///
    /// Typed in reading order, as nominal letters, it is found, and the range is the whole of the
    /// stored word — which is what the highlight is drawn over. The calibration is the reversed
    /// spelling: typed as the stored order's letters, it is a different word, and the page's
    /// positions say it is not there (ADR 1465).
    #[test]
    fn a_word_shown_in_display_order_as_presentation_forms_is_found_as_typed() {
        let (text, order) = page(
            "\u{fe94}\u{fef4}\u{fe91}\u{feae}\u{fecc}\u{fedf}\u{627}",
            true,
        );
        let typed = "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629}";
        assert_eq!(find_ordered(&text, typed, &order), vec![(0, text.len())]);
        let reversed: String = typed.chars().rev().collect();
        assert_eq!(
            find_ordered(&text, &reversed, &order),
            vec![],
            "the stored order typed as though it were the reading order is another word"
        );
        assert_eq!(
            find_ordered(&text, "\u{639}\u{631}\u{628}", &order),
            vec![(6, 15)],
            "part of the word, from inside the stored range"
        );
    }

    /// The same word stored in reading order — glyph by glyph to the left, or §14.8.2.5.3's
    /// `ReversedChars` read back — is found as typed, and its reversal is not.
    #[test]
    fn a_word_stored_in_reading_order_is_found_as_typed_and_not_reversed() {
        let typed = "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629}";
        let (text, order) = page(typed, false);
        assert_eq!(find_ordered(&text, typed, &order), vec![(0, text.len())]);
        let reversed: String = typed.chars().rev().collect();
        assert_eq!(find_ordered(&text, &reversed, &order), vec![]);
        // A page whose positions decided nothing admits both, which is the cost of no evidence.
        assert_eq!(find(&text, &reversed), vec![(0, text.len())]);
    }

    /// A lam-alef ligature stored in display order: its alef is on the left, so it is compared
    /// alef first, and "سلام" shown as `ﻡﻼﺳ` is found.
    #[test]
    fn a_ligature_in_a_display_order_run_is_read_in_that_order() {
        let (text, order) = page("\u{fee1}\u{fefc}\u{feb3}", true);
        assert_eq!(
            find_ordered(&text, "\u{633}\u{644}\u{627}\u{645}", &order),
            vec![(0, text.len())]
        );
        let (text, order) = page("\u{feb3}\u{fefc}\u{fee1}", false);
        assert_eq!(
            find_ordered(&text, "\u{633}\u{644}\u{627}\u{645}", &order),
            vec![(0, text.len())],
            "and in reading order, lam first"
        );
    }

    /// Digits keep reading left to right inside a right-to-left line, which is Unicode Standard
    /// Annex #9's rule L2: "سنة 2024" displays as `2024 ةنس`, and a page that stored that is
    /// found by the reading order typed. A Latin word beside Arabic is found under both paragraph
    /// directions, because an untagged page states neither.
    #[test]
    fn digits_and_latin_inside_a_right_to_left_line_keep_their_own_order() {
        let (text, order) = page("2024 \u{629}\u{646}\u{633}", true);
        assert_eq!(
            find_ordered(&text, "\u{633}\u{646}\u{629} 2024", &order),
            vec![(0, text.len())]
        );
        let (text, order) = page("2024\u{629}\u{646}\u{633}", true);
        assert_eq!(
            find_ordered(&text, "\u{633}\u{646}\u{629}2024", &order),
            vec![(0, text.len())],
            "digits joined to the word are displayed to its left, still left to right"
        );
        let typed = "\u{639}\u{631}\u{628}\u{64a} pdf";
        for shown in [
            "\u{64a}\u{628}\u{631}\u{639} PDF",
            "PDF \u{64a}\u{628}\u{631}\u{639}",
        ] {
            let (text, order) = page(shown, true);
            assert_eq!(
                find_ordered(&text, typed, &order),
                vec![(0, text.len())],
                "{shown}"
            );
        }
    }

    /// A Latin ligature the readback states as one character is found by its two letters, and a
    /// presentation form typed into the bar is its letter.
    #[test]
    fn a_presentation_form_on_either_side_is_its_letters() {
        assert_eq!(find("\u{fb01}nd it", "find"), vec![(0, 5)]);
        let (text, order) = page("\u{629}\u{64a}", true);
        assert_eq!(
            find_ordered(&text, "\u{fef3}\u{fe94}", &order),
            vec![(0, text.len())]
        );
    }

    /// ADR 1477's rule, the same for every script: a mark the needle leaves off is not asked for,
    /// and a mark it states must be on the page.
    ///
    /// "كتب" finds "كَتَبَ" and "שלום" finds "שָׁלוֹם", and the range covers the marks after the last
    /// letter, so a highlight is over everything printed. The calibration is the other direction:
    /// a needle with a fatha does not find the bare word, nor a word whose mark is another.
    #[test]
    fn a_mark_the_needle_leaves_off_is_not_asked_for_and_one_it_states_is() {
        let vowelled = "\u{643}\u{64e}\u{62a}\u{64e}\u{628}\u{64e}";
        let bare = "\u{643}\u{62a}\u{628}";
        assert_eq!(find(vowelled, bare), vec![(0, vowelled.len())]);
        assert_eq!(find(vowelled, vowelled), vec![(0, vowelled.len())]);
        assert_eq!(find(bare, vowelled), vec![], "a stated mark is required");
        assert_eq!(
            find(vowelled, "\u{643}\u{64f}\u{62a}\u{628}"),
            vec![],
            "a damma is not a fatha"
        );
        assert_eq!(
            find(vowelled, "\u{643}\u{64e}\u{62a}\u{628}"),
            vec![(0, vowelled.len())],
            "the marks stated are asked for and the rest are not"
        );

        // Hebrew: shin, shin dot, qamats; lamed; vav, holam; final mem.
        let pointed = "\u{5e9}\u{5c1}\u{5b8}\u{5dc}\u{5d5}\u{5b9}\u{5dd}";
        assert_eq!(
            find(pointed, "\u{5e9}\u{5dc}\u{5d5}\u{5dd}"),
            vec![(0, pointed.len())]
        );

        // Latin by the same rule, and canonical equivalence on either side.
        assert_eq!(find("caf\u{e9}", "cafe"), vec![(0, 5)]);
        assert_eq!(find("cafe", "caf\u{e9}"), vec![]);
        assert_eq!(find("cafe\u{301}", "caf\u{e9}"), vec![(0, 6)]);
        assert_eq!(find("caf\u{e9}", "cafe\u{301}"), vec![(0, 5)]);
        assert_eq!(find("CAF\u{c9} noir", "caf\u{e9}"), vec![(0, 5)]);
    }

    /// Two marks on one letter compare by canonical ordering, not by the order a producer wrote
    /// them: a shadda and a fatha stored either way round are one text, and a needle naming the
    /// fatha alone finds both. A match does not start on a mark the needle does not state.
    #[test]
    fn marks_compare_in_canonical_order_and_a_match_starts_on_a_letter() {
        let fatha_first = "\u{628}\u{64e}\u{651}";
        let shadda_first = "\u{628}\u{651}\u{64e}";
        assert_eq!(find(fatha_first, shadda_first), vec![(0, 6)]);
        assert_eq!(find(shadda_first, fatha_first), vec![(0, 6)]);
        assert_eq!(find(shadda_first, "\u{628}\u{64e}"), vec![(0, 6)]);
        assert_eq!(
            find("\u{301}e", "e"),
            vec![(2, 3)],
            "the stray mark is not part of the match"
        );
    }

    /// A Devanagari vowel sign is a nonspacing mark of combining class zero: it spells the
    /// syllable, so a needle without it is another word (ADR 1477).
    #[test]
    fn a_vowel_sign_that_spells_its_syllable_is_a_letter() {
        // कुल, "total", and कल, "tomorrow".
        assert_eq!(find("\u{915}\u{941}\u{932}", "\u{915}\u{932}"), vec![]);
    }

    /// A line under a text matrix that mirrors it (`-1 0 0 1 x y Tm`): each glyph's box extends
    /// from its origin to the *left*, and a word shown in display order advances leftwards across
    /// the display. Its stored order is the producer's, so it is display order, and the needle is
    /// found as typed; read with a fixed quarter turn it would vote reading order and the reversed
    /// spelling would be found instead.
    #[test]
    fn a_mirrored_line_is_read_in_the_order_its_producer_stored() {
        let stored = "\u{628}\u{631}\u{639}";
        let placed: Vec<Placed> = stored
            .char_indices()
            .enumerate()
            .map(|(index, (at, character))| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "test code: three glyphs, exact in f32"
                )]
                let origin = 100.0 - index as f32 * 10.0;
                Placed {
                    span: at..at.saturating_add(character.len_utf8()),
                    quad: [
                        origin,
                        0.0,
                        origin - 10.0,
                        0.0,
                        origin - 10.0,
                        12.0,
                        origin,
                        12.0,
                    ],
                }
            })
            .collect();
        let order = Order::of(stored, &placed);
        let typed = "\u{639}\u{631}\u{628}";
        assert_eq!(find_ordered(stored, typed, &order), vec![(0, stored.len())]);
        assert_eq!(
            find_ordered(stored, stored, &order),
            vec![],
            "the stored order typed as though it were the reading order is another word"
        );
    }

    /// Two right-to-left lines are two runs, each deciding its own order.
    #[test]
    fn each_line_decides_its_own_order() {
        let text = "\u{628}\u{627}\n\u{628}\u{627}";
        let quad = |left: f32, bottom: f32| {
            [
                left,
                bottom,
                left + 10.0,
                bottom,
                left + 10.0,
                bottom + 12.0,
                left,
                bottom + 12.0,
            ]
        };
        let placed = vec![
            Placed {
                span: 0..2,
                quad: quad(0.0, 100.0),
            },
            Placed {
                span: 2..4,
                quad: quad(10.0, 100.0),
            },
            Placed {
                span: 5..7,
                quad: quad(10.0, 80.0),
            },
            Placed {
                span: 7..9,
                quad: quad(0.0, 80.0),
            },
        ];
        let order = Order::of(text, &placed);
        // Typed "اب": the first line shows it in display order, the second in reading order.
        let typed = "\u{627}\u{628}";
        assert_eq!(find_ordered(text, typed, &order), vec![(0, 4)]);
        let other = "\u{628}\u{627}";
        assert_eq!(find_ordered(text, other, &order), vec![(5, 9)]);
    }
}
