//! Showing text: §9.4's positioning, Table 104's rendering modes, and the readback.
//!
//! One pass places the glyphs and extracts the page's text from the same code-to-glyph
//! decisions, which is what makes the readback evidence about the rendering. §9.3.8's text
//! knockout and §11.7.4.4's implicit per-glyph group are judged at `ET`, where the finished
//! object can be seen whole.

use pdf_render::{Path, Rect, Transform};

use super::transparency::AlphaSourcesSeen;

mod end_object;
mod glyph_painting;
mod paint;
mod placement;
mod readback;
mod show;
mod type3;
mod word_gaps;

pub(super) use word_gaps::WordGaps;

/// What a text object owns, as against what the graphics state does.
///
/// ISO 32000-2 §9.4.1 draws the line:
///
/// > In addition, three parameters may be specified only within a text object and shall not
/// > persist from one text object to the next
///
/// Two of the three are fields here. The third, `Trm`, "is actually just an intermediate
/// result" and is recomputed for each glyph in [`Interpreter::show_text`](crate::content::Interpreter::show_text) rather than
/// stored. The accumulated clipping path joins them because it has exactly the same scope —
/// §9.3.6 starts it at `BT` and consumes it at `ET` — and because keeping it out of
/// [`GraphicsState`](super::GraphicsState) is what stops `q`/`Q` from saving and restoring something the
/// specification never puts in the graphics state.
///
/// A `BT` resets the whole struct, which is Table 105's requirement for the two matrices
/// and §9.3.6's for the third field, in one line that cannot get one of them wrong.
#[derive(Debug, Default)]
pub(super) struct TextObject {
    /// `Tm`, the text matrix.
    pub(super) matrix: Transform,
    /// `Tlm`, the text line matrix: `Tm` as it was at the start of the current line.
    pub(super) line: Transform,
    /// Glyph outlines accumulated by rendering modes 4 to 7, already in page space.
    ///
    /// Empty means no clipping mode has shown a glyph with an outline, which §9.3.6 makes a
    /// meaningful state of its own rather than an empty clip — see
    /// [`Interpreter::end_text_object`](crate::content::Interpreter::end_text_object).
    pub(super) clip: Path,
    /// Where this object's glyphs have marked the page under a paint that composites.
    ///
    /// `None` for a Type 3 glyph, whose ink is a content stream this does not run twice to
    /// find out. Accumulated rather than reported per glyph because knockout is a property
    /// of the *text object*: one glyph cannot overlap itself, so the difference §9.3.8
    /// describes needs two — see [`Interpreter::end_text_object`](crate::content::Interpreter::end_text_object).
    pub(super) composited: Vec<Option<Rect>>,
    /// Whether two of those glyphs were found to overlap, which is what `Tk` would change.
    pub(super) knockout_owed: bool,
    /// Command ranges holding one glyph's fill and stroke, for §11.7.4.4's implicit group.
    ///
    /// A glyph shown in rendering mode 2 or 6 is filled *and* stroked, and the clause makes
    /// that pair one object rather than two — the same requirement §11.6.2 places on `B`. The
    /// ranges are collected rather than wrapped as they are drawn because §9.3.8's own group
    /// may turn out to enclose the whole object, and a knockout group inside a knockout group
    /// is not something either backend can state; which of the two is built is therefore one
    /// decision, taken at `ET` in [`Interpreter::end_text_object`](crate::content::Interpreter::end_text_object).
    ///
    /// Each range carries the readings of §11.6.4.3's `/AIS` its two parts were painted under:
    /// the one in force at the showing operator, and whatever a tiling cell among them ran
    /// under — the pair's own, as a `B`'s is (ADR 1306).
    pub(super) combined: Vec<(usize, usize, AlphaSourcesSeen)>,
    /// How many commands the display list held at this object's `BT`.
    ///
    /// §9.3.8 makes a text object with `Tk` true "equivalent to treating the entire text
    /// object as if it were a non-isolated knockout transparency group", so what the group
    /// contains is everything drawn between `BT` and `ET` — which is this mark to the end.
    pub(super) start: usize,
    /// The enclosing content's record of §11.6.4.3's readings, held while this object keeps
    /// its own: §9.3.8's group is the object's glyphs, so the readings it asks about are the
    /// ones they were shown under, from the reading in force at `BT` on
    /// (`Interpreter::open_reading_scope`, ADR 1306). `None` outside a text object.
    pub(super) enclosing_reading: Option<super::transparency::Readings>,
}

impl TextObject {
    /// Records where a glyph marked the page, and whether §9.3.8 could show on this object.
    ///
    /// `bounds` is `None` where the ink is not known — a Type 3 glyph — and an unknown box is
    /// taken to overlap everything, which is the safe direction for a *report*: it may say a
    /// text object could differ where it does not, and never the reverse.
    fn note_knockout(&mut self, bounds: Option<Rect>) {
        let overlaps = self.composited.iter().any(|other| match (other, bounds) {
            (Some(first), Some(second)) => {
                first.min.x < second.max.x
                    && second.min.x < first.max.x
                    && first.min.y < second.max.y
                    && second.min.y < first.max.y
            }
            _ => true,
        });
        self.knockout_owed |= overlaps;
        self.composited.push(bounds);
    }
}

/// What a page's codes got out of one font, tallied while they are shown.
#[derive(Debug, Clone, Default)]
pub(super) struct Coverage {
    /// Codes that reached an outline.
    pub(super) drawn: u32,
    /// Codes that did not.
    pub(super) empty: u32,
    /// How many of `empty` were §9.10.2's uncovered characters, which decides which of the
    /// two reports a silent font gets.
    pub(super) uncovered: u32,
    /// How many of `empty` reached a glyph the program's repair could not draw — a CID-keyed
    /// CFF read against an empty Private DICT, whose glyph calls a local subroutine (ADR 0808).
    pub(super) lost: u32,
    /// The repair's own sentence, taken from the font the first time `lost` is raised, so that
    /// the page's report can say what was done to the program and not only what it cost.
    pub(super) shortfall: Option<String>,
    /// How many of `empty` reached a glyph whose program the evaluator refused — a charstring
    /// past the operation budget (ADR 2572) or any other error running it — which is a mark the
    /// file may describe and this page did not draw (ADR 2586).
    pub(super) refused: u32,
    /// The first such refusal, with the glyph and the evaluator's reason, for the page's report.
    pub(super) refusal: Option<pdf_font::OutlineRefused>,
    /// A `glyf` composite one of the empty codes reached that includes itself, which describes no
    /// outline at all: the program's fault, named in the report (ADR 1411).
    pub(super) cycle: Option<u16>,
    /// Whether the font's descriptor sets both Table 121's Symbolic and Nonsymbolic flags, which
    /// §9.8.2 says "shall not both be set": the reading that chose the empty glyphs is the one the
    /// clause's "should always check the Symbolic flag" gives such a file (ADR 1411).
    pub(super) both_flags: bool,
    /// Whether the program describes no outline for any glyph it holds, asked once at the first
    /// empty code. Such a font draws nothing by §9.7.6.3's own route, and the page does not report
    /// it (ADR 1411).
    pub(super) outline_free: Option<bool>,
}

#[cfg(test)]
mod tests {
    //! `separate_text`'s gap, read in §9.4.4's text space (ADR 1490).
    //!
    //! Each page is one `TJ` in Helvetica at 20 units, whose `a` and `b` are 556 thousandths wide
    //! and whose space is 278, so — with fewer steps than a font's own threshold is read off (ADR
    //! 1515) — the threshold is 278 × 20 / 1000 × 0.5 = 2.78 text-space units at `20 Tf` and 0.139
    //! at `1 Tf`. Every expected readback follows from §9.4.4's `tx` and the `Tm` written beside it.

    #![expect(
        clippy::arithmetic_side_effects,
        reason = "test code: the fixture's offsets are computed from strings this module wrote"
    )]

    use std::fmt::Write as _;

    use pdf_render::Transform;

    use super::super::Interpreter;

    /// The readback of a one-page document whose content stream is `content`, in Helvetica.
    fn readback(content: &str) -> String {
        readback_in(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            content,
        )
    }

    /// The readback of a one-page document whose content stream is `content`, in `font`.
    fn readback_in(font: &str, content: &str) -> String {
        interpretation_in(font, content).text
    }

    /// The interpretation of a one-page document whose content stream is `content`, in `font`.
    fn interpretation_in(font: &str, content: &str) -> crate::Interpretation {
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
             5 0 obj\n{font}\nendobj\n",
            content.len() + 1,
        );
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        let mut cursor = out.len();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(cursor);
            cursor += object.len();
        }
        out.push_str(&body);
        let xref_at = out.len();
        let size = offsets.len() + 1;
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        let document =
            pdf_syntax::Document::open(out.into_bytes()).expect("the fixture is a valid file");
        let pages = crate::Pages::new(&document);
        let page = pages.get(0).expect("one page");
        crate::interpret(&document, &page)
    }

    /// A `TJ` of `words` words, each `(ab) -10 (ab)` and then `gap` before the next.
    fn words_set_apart(gap: i32, words: usize) -> String {
        let mut array = String::new();
        for _ in 0..words {
            let _ = write!(array, "(ab) -10 (ab) {gap} ");
        }
        format!("BT /F1 20 Tf 10 100 Td [{array}] TJ ET")
    }

    /// A `TJ` written in reading order under a mirroring `Tm`: each adjustment of 1000 moves the
    /// pen back by 20 in text space, twice the 11.12 the glyph advanced, so there is no gap to
    /// read — and along user-space x the mirror made each one a step of 8.88 rightwards, over the
    /// 2.78 threshold.
    #[test]
    fn a_tj_under_a_mirroring_tm_reads_back_with_no_gaps() {
        let mirrored =
            readback("BT /F1 20 Tf -1 0 0 1 200 0 Tm 90 100 Td [(b) 1000 (a) 1000 (b)] TJ ET");
        assert_eq!(mirrored, "bab");
        // And without the mirror, the same string is the same word.
        let upright = readback("BT /F1 20 Tf 90 100 Td [(b) 1000 (a) 1000 (b)] TJ ET");
        assert_eq!(upright, "bab");
    }

    /// The sign is the advance's, not the page's: a gap that opens in text space is a word gap
    /// whichever way the mirror turns it. `-300` adds 6 to the pen, over 2.78.
    #[test]
    fn a_gap_opened_under_a_mirroring_tm_is_still_a_word_gap() {
        assert_eq!(
            readback("BT /F1 20 Tf -1 0 0 1 200 0 Tm 50 100 Td [(a) -300 (b)] TJ ET"),
            "a b"
        );
        assert_eq!(
            readback("BT /F1 20 Tf 50 100 Td [(a) -300 (b)] TJ ET"),
            "a b"
        );
        // A negative horizontal scaling mirrors the line as `Tm` does, and is undone with it.
        assert_eq!(
            readback("BT /F1 20 Tf -100 Tz 150 100 Td [(a) -300 (b) 1000 (a)] TJ ET"),
            "a ba"
        );
    }

    /// A `Tm` that scales: `1 Tf` under `20 0 0 20 Tm` is the same line as `20 Tf` under the
    /// identity, and its gaps are measured in the units its threshold is. `-100` is 0.1 of a text
    /// unit, under 0.139, so it is tracking; `-300` is 0.3, over it, so it is a word gap.
    #[test]
    fn a_scaling_tm_measures_its_gap_in_text_space() {
        assert_eq!(
            readback("BT /F1 1 Tf 20 0 0 20 50 100 Tm [(a) -100 (b) -300 (a)] TJ ET"),
            "ab a"
        );
        assert_eq!(
            readback("BT /F1 20 Tf 50 100 Td [(a) -100 (b) -300 (a)] TJ ET"),
            "ab a"
        );
    }

    /// The step undone: through the linear part and out of `Th`, and read as it stands where
    /// either collapses text space.
    #[test]
    fn a_step_is_taken_back_into_text_space() {
        let mirror = Transform::new(-1.0, 0.0, 0.0, 1.0, 200.0, 0.0);
        assert_eq!(
            Interpreter::text_space_step(mirror, 1.0, (8.0, 2.0)),
            (-8.0, 2.0)
        );
        let scaled = Transform::new(20.0, 0.0, 0.0, 20.0, 50.0, 100.0);
        assert_eq!(
            Interpreter::text_space_step(scaled, 0.5, (4.0, 10.0)),
            (0.4, 0.5)
        );
        let collapsed = Transform::new(0.0, 0.0, 0.0, 0.0, 50.0, 100.0);
        assert_eq!(
            Interpreter::text_space_step(collapsed, 1.0, (4.0, 10.0)),
            (4.0, 10.0)
        );
    }

    /// A font stating no space is given a quarter em for one, and the same half of it: a `TJ`
    /// adjustment of `-250` — a quarter em, the word gap a producer writes for such a subset —
    /// is a word gap, where a threshold of the whole quarter em read it as none.
    #[test]
    fn a_font_with_no_space_still_has_a_word_gap() {
        let boxes = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] \
                     /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 97 /LastChar 98 \
                     /Widths [500 500] /Encoding << /Differences [97 /a /b] >> \
                     /CharProcs << /a 6 0 R /b 6 0 R >> /Resources << >> >>\nendobj\n\
                     6 0 obj\n<< /Length 25 >>\nstream\n500 0 d0 0 0 450 700 re f\nendstream";
        assert_eq!(
            readback_in(boxes, "BT /F1 20 Tf 50 100 Td [(a) -250 (b) -50 (a)] TJ ET"),
            "a ba"
        );
    }

    /// `issue1453.pdf`'s title in miniature: a display face stating no space whose producer set
    /// its word gaps at `-169` and `-140` and kerned one pair at `-31`. At `20 Tf` the threshold
    /// is half a quarter em, 2.5 units; `-140` moves the pen 2.8 and is a word gap, `-31` moves it
    /// 0.62 and is not (ADR 1502).
    #[test]
    fn a_display_face_with_no_space_breaks_at_an_eighth_of_an_em() {
        let boxes = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] \
                     /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 97 /LastChar 98 \
                     /Widths [500 500] /Encoding << /Differences [97 /a /b] >> \
                     /CharProcs << /a 6 0 R /b 6 0 R >> /Resources << >> >>\nendobj\n\
                     6 0 obj\n<< /Length 25 >>\nstream\n500 0 d0 0 0 450 700 re f\nendstream";
        assert_eq!(
            readback_in(
                boxes,
                "BT /F1 20 Tf 50 100 Td [(a) -169 (b) -140 (a) -31 (b)] TJ ET"
            ),
            "a b ab"
        );
    }

    /// A font whose producer set its word gaps nearer its kerning than its own space: Helvetica,
    /// kerned at 0.01 em (0.036 of its space) and its words set 0.09 em apart (0.32 of it), over
    /// forty steps. Under the constant half a space every gap is kerning and the line reads as one
    /// word; the font's own steps put the empty interval between 0.036 and 0.32, and a threshold
    /// inside it reads each pair of strings as a word (ADR 1515).
    #[test]
    fn a_font_whose_word_gaps_sit_below_half_its_space_breaks_at_its_own_gap() {
        assert_eq!(readback(&words_set_apart(-90, 21)), ["abab"; 21].join(" "));
    }

    /// The same steps with every string a single code are a letter-spaced line rather than words,
    /// and the font's own gap does not decide: the constant reads it as one word (ADR 1515).
    #[test]
    fn letter_spaced_single_codes_keep_the_constant() {
        let mut array = String::new();
        for _ in 0..21 {
            array.push_str("(a) -10 (b) -90 ");
        }
        let content = format!("BT /F1 20 Tf 10 100 Td [{array}] TJ ET");
        assert_eq!(readback(&content), "ab".repeat(21));
    }

    /// A provisional space the constant takes back leaves every range over the readback where its
    /// characters are: 0.1 em in Helvetica is 0.36 of its space, so `[(ab) -100 (cd)]` reads "abcd",
    /// each code's span is its own character, and the `/MCID` sequence around it covers all four.
    #[test]
    fn a_space_taken_back_moves_the_ranges_over_the_text() {
        let read = interpretation_in(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            "/P << /MCID 0 >> BDC BT /F1 20 Tf 10 100 Td [(ab) -100 (cd)] TJ ET EMC",
        );
        assert_eq!(read.text, "abcd");
        let spans: Vec<_> = read
            .text_layer
            .iter()
            .map(|placed| placed.span.clone())
            .collect();
        assert_eq!(spans, [0..1, 1..2, 2..3, 3..4]);
        assert_eq!(read.inferred_separators, 0);
        let marked: Vec<(usize, usize)> = read
            .marked
            .iter()
            .map(|span| (span.range.start, span.range.end))
            .collect();
        assert_eq!(marked, [(0, 4)]);
    }

    /// §14.9.4's replacement stands for every code it encloses: each of the four codes shown
    /// inside the sequence reads back as "xy", whatever space the cut readback held between them.
    #[test]
    fn a_code_inside_a_replacement_reads_back_as_all_of_it() {
        let read = interpretation_in(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            "/Span << /ActualText (xy) >> BDC BT /F1 20 Tf 10 100 Td [(ab) -300 (cd)] TJ ET EMC",
        );
        assert_eq!(read.text, "xy");
        let spans: Vec<_> = read
            .text_layer
            .iter()
            .map(|placed| placed.span.clone())
            .collect();
        assert_eq!(spans, [0..2, 0..2, 0..2, 0..2]);
    }

    /// A gap of 0.15 em in Helvetica is 0.54 of its 278-thousandth space: nearer a space than no
    /// gap, so a word break; 0.12 em is 0.43 of one, and is not.
    #[test]
    fn a_gap_nearer_a_space_than_none_is_a_word_break() {
        assert_eq!(
            readback("BT /F1 20 Tf 50 100 Td [(a) -150 (b) -120 (a)] TJ ET"),
            "a ba"
        );
    }
}
