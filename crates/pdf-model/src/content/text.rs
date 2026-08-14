//! Showing text: §9.4's positioning, Table 104's rendering modes, and the readback.
//!
//! One pass places the glyphs and extracts the page's text from the same code-to-glyph
//! decisions, which is what makes the readback evidence about the rendering. §9.3.8's text
//! knockout and §11.7.4.4's implicit per-glyph group are judged at `ET`, where the finished
//! object can be seen whole.

use std::rc::Rc;
use std::sync::Arc;

use pdf_font::Code;
use pdf_render::display_list::Clip;
use pdf_render::{BlendMode, ClipId, Command, FillRule, Path, Point, Rect, Transform};
use pdf_syntax::Dictionary;

use super::font::Font;
use super::path::marks;
use super::pattern::PatternPaint;
use super::report::{Placed, Unsupported};
use super::transparency::{knockout_is_drawable, outline_bounds};
use super::{GraphicsState, Interpreter, MAX_FORM_DEPTH};

/// What a text object owns, as against what the graphics state does.
///
/// ISO 32000-2 §9.4.1 draws the line:
///
/// > In addition, three parameters may be specified only within a text object and shall not
/// > persist from one text object to the next
///
/// Two of the three are fields here. The third, `Trm`, "is actually just an intermediate
/// result" and is recomputed for each glyph in [`Interpreter::show_text`] rather than
/// stored. The accumulated clipping path joins them because it has exactly the same scope —
/// §9.3.6 starts it at `BT` and consumes it at `ET` — and because keeping it out of
/// [`GraphicsState`] is what stops `q`/`Q` from saving and restoring something the
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
    /// [`Interpreter::end_text_object`].
    pub(super) clip: Path,
    /// Where this object's glyphs have marked the page under a paint that composites.
    ///
    /// `None` for a Type 3 glyph, whose ink is a content stream this does not run twice to
    /// find out. Accumulated rather than reported per glyph because knockout is a property
    /// of the *text object*: one glyph cannot overlap itself, so the difference §9.3.8
    /// describes needs two — see [`Interpreter::end_text_object`].
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
    /// decision, taken at `ET` in [`Interpreter::end_text_object`].
    pub(super) combined: Vec<(usize, usize)>,
    /// How many commands the display list held at this object's `BT`.
    ///
    /// §9.3.8 makes a text object with `Tk` true "equivalent to treating the entire text
    /// object as if it were a non-isolated knockout transparency group", so what the group
    /// contains is everything drawn between `BT` and `ET` — which is this mark to the end.
    pub(super) start: usize,
}

/// What one glyph is to have done to it, decided once per show string rather than per glyph.
///
/// §9.3.6's Table 104 is three independent operations — fill, stroke, add to the clipping path
/// — rather than eight cases, and the two knockout questions are answers about the *paint*
/// rather than about the glyph, so all five are constant across a `Tj`.
#[derive(Debug, Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent yes-or-no answers about one glyph, three of them Table 104's \
              own decomposition of the rendering mode; a state machine would have to \
              enumerate the product of five bits that the clause deliberately keeps separate"
)]
struct GlyphPainting {
    /// Modes 0, 2, 4 and 6.
    fills: bool,
    /// Modes 1, 2, 5 and 6.
    strokes: bool,
    /// Modes 4 to 7.
    clipping: bool,
    /// Whether §9.3.8's text knockout could change a pixel of this object.
    knockout_can_show: bool,
    /// Whether §11.7.4.4's implicit group could change a pixel of this glyph.
    combining: bool,
}

impl GlyphPainting {
    /// Reads Table 104's mode and the two clauses that ask about the paint behind it.
    ///
    /// A hidden optional-content layer suppresses the two operations that mark the page and
    /// *not* the clip: §8.11.3.1 lists clipping among the "graphics state operations" that
    /// "shall still be applied", and requires that "graphics state parameters that persist
    /// past the end of a marked-content section shall be the same whether the optional content
    /// is visible or not". The clip a text object leaves behind is one of those, since it
    /// outlives the `ET` that built it.
    fn read(mode: i64, hidden: bool, state: &GraphicsState) -> Self {
        let fills = matches!(mode, 0 | 2 | 4 | 6) && !hidden;
        let strokes = matches!(mode, 1 | 2 | 5 | 6) && !hidden;
        Self {
            fills,
            strokes,
            clipping: matches!(mode, 4..=7),
            // §9.3.8: with `Tk` true — its initial value — the whole text object behaves as a
            // non-isolated knockout group, so "later glyphs shall overwrite ('knock out')
            // earlier ones in the area of overlap". We composite each glyph against what is
            // already on the page, which is exactly the `Tk` false behaviour. Two conditions
            // have to hold before the models can differ, and both are checked rather than
            // assumed: the paint has to composite at all — an opaque glyph under the Normal
            // blend mode overwrites what it covers either way — and two glyphs of the object
            // have to overlap, which most text never does and which only `ET` can know.
            knockout_can_show: (fills || strokes)
                && state.text.knockout
                && state.paint_composites(),
            // §11.7.4.4 applies to "the painting of glyphs with text rendering mode 2 or 6",
            // which is `fills && strokes`, and its NOTE 1 says the rule "is independent of the
            // text knockout parameter in the graphics state" — so this is a different
            // condition from the one above, not a special case of it. The other two halves are
            // §11.6.2's, for the same reason they are there: the paint has to composite at
            // all, and both parts have to mark the page.
            combining: fills
                && strokes
                && state.paint_composites()
                && (matches!(state.fill_pattern, Some(PatternPaint::Tiling(_)))
                    || marks(&state.fill_paint()))
                && marks(&state.stroke_paint()),
        }
    }
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
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Coverage {
    /// Codes that reached an outline.
    pub(super) drawn: u32,
    /// Codes that did not.
    pub(super) empty: u32,
    /// How many of `empty` were §9.10.2's uncovered characters, which decides which of the
    /// two reports a silent font gets.
    pub(super) uncovered: u32,
}

/// What one code contributed to the page's readback.
///
/// Three states rather than a string, because the difference between the last two decides
/// whether a code that reached no outline is a mark the reader lost. A code that reads back as
/// a space is *meant* to have no outline; a code §9.10.2 could not name says nothing either
/// way, and taking the second for the first is a wrong answer that reports nothing.
///
/// **They were the same state until the four-hundred-and-seventy-sixth session**, because the
/// test in front of the tally was `self.text[start..].chars().all(char::is_whitespace)` and an
/// empty slice satisfies that vacuously — so a font that named none of its codes was read as a
/// page of spaces. It was blind twice over: inside §14.8.2.5.3's reversal the readback is
/// collected per code and appended after the string, so *every* code's slice was empty there.
/// Asking the font what it said, rather than asking the buffer what arrived, answers both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Readback {
    /// Text, at least one character of which is not whitespace.
    Characters,
    /// Text, all of it whitespace.
    Whitespace,
    /// Nothing at all: every one of §9.10.2's methods, its closing permission and §9.3.3's
    /// naming of code 32 declined, or the producer's own mapping is the empty string.
    Nothing,
}

impl Readback {
    /// Classifies what [`Font::text`] appended for one code.
    fn of(named: bool, text: &str) -> Self {
        if !named || text.is_empty() {
            Self::Nothing
        } else if text.chars().all(char::is_whitespace) {
            Self::Whitespace
        } else {
            Self::Characters
        }
    }

    /// Whether this readback says a mark was owed.
    ///
    /// Only characters do. A space is *meant* to have no outline, and a code §9.10.2 could not
    /// name says nothing about what the page owed — the clause's own words are "there is no way
    /// to determine what the character code represents", which is not evidence in either
    /// direction and must not be read as either.
    fn names_a_mark(self) -> bool {
        self == Self::Characters
    }
}

/// A glyph's box, mapped by §9.4.4's text rendering matrix.
///
/// The corners in glyph space are (0, descent), (advance, descent), (advance, ascent) and
/// (0, ascent) — the advance along the baseline and the font's own reach above and below it —
/// and they go round the quadrilateral in that order so that a consumer can draw it as a
/// polygon without sorting.
fn glyph_quad(advance: f32, extent: (f32, f32), transform: Transform) -> [f32; 8] {
    let (ascent, descent) = extent;
    let corner = |x: f32, y: f32| {
        let point = transform.apply(Point::new(x, y));
        (point.x, point.y)
    };
    let (a, b, c, d) = (
        corner(0.0, descent),
        corner(advance, descent),
        corner(advance, ascent),
        corner(0.0, ascent),
    );
    [a.0, a.1, b.0, b.1, c.0, c.1, d.0, d.1]
}

impl Interpreter<'_> {
    /// Adds one show string's worth of coverage to a font's tally.
    ///
    /// Per *string* rather than per glyph, which is not a style choice: the map is keyed by the
    /// resource name and a lookup per glyph cost **2%** of interpretation on the specification's
    /// own page, measured by `callgrind_interpret` in the session that added it. The font cannot
    /// change inside a show string — only `Tf` changes it — so the counts are accumulated in
    /// three integers and applied once.
    fn tally_glyph(&mut self, name: &str, counted: Coverage) {
        // `entry` would take the resource name by value, which is an allocation per show
        // string whether or not the font is already in the map — **2.2% of interpretation**
        // on the specification's own page, measured by stubbing this function out. A page
        // names two or three fonts and shows thousands of strings through them, so the
        // lookup that allocates is the one that almost never has to.
        if let Some(entry) = self.glyph_coverage.get_mut(name) {
            entry.drawn = entry.drawn.saturating_add(counted.drawn);
            entry.empty = entry.empty.saturating_add(counted.empty);
            entry.uncovered = entry.uncovered.saturating_add(counted.uncovered);
        } else {
            self.glyph_coverage.insert(name.to_owned(), counted);
        }
    }

    /// Draws a string, advancing the text matrix.
    ///
    /// # The positioning arithmetic
    ///
    /// Each glyph is placed by the text rendering matrix, which is the font size and
    /// horizontal scaling, times the text matrix, times the current transform. The advance
    /// after each glyph is `(w0 * size + char_spacing + word_spacing) * horizontal_scale`,
    /// where `w0` is the glyph's width in em units and word spacing applies only to a
    /// single-byte code 32.
    ///
    /// Getting the order wrong produces text that is present but misplaced, which looks
    /// like a font bug and is really an arithmetic one.
    ///
    /// # The rendering mode
    ///
    /// §9.3.6 Table 104's eight modes are three independent operations — fill, stroke, add
    /// to the clipping path — rather than eight cases, and they are read that way below.
    /// The clause makes each behave as it would for a path: "Stroking, filling, and clipping
    /// shall have the same effects for a text object as they do for a path object … although
    /// they are specified in an entirely different way."
    #[expect(
        clippy::too_many_lines,
        reason = "one pass over a show string's codes, and every step of it is a clause: the \
                  readback, the text layer's geometry, the two kinds of glyph and §9.3's four \
                  spacing parameters. Splitting it would need nine parameters to carry the \
                  loop's state into the piece that was moved"
    )]
    pub(super) fn show_text(
        &mut self,
        bytes: &[u8],
        state: &GraphicsState,
        text: &mut TextObject,
        resources: &Dictionary,
        form_depth: usize,
    ) {
        let Some(font) = state.text.font.clone() else {
            // Text we cannot draw is counted so the page says it is incomplete — unless the
            // layer it belongs to is off, in which case not drawing it is correct.
            if !self.is_hidden() {
                self.text_operations = self.text_operations.saturating_add(1);
            }
            return;
        };

        // The three operations of Table 104, and the two clauses that ask about the paint
        // behind them; see `GlyphPainting::read`. Mode 3 does none of the three and mode 7
        // only the last, which is what an OCR layer under a scanned image uses; either way
        // the text matrix still advances and the extracted text still accumulates, because
        // §9.3.6 requires it — "The e and f components of Tm shall be updated for each glyph
        // drawn when using text rendering mode 3 or 7 in exactly the same way as would be
        // done for other text rendering modes."
        let painting = GlyphPainting::read(state.text.render_mode, self.is_hidden(), state);
        let GlyphPainting {
            fills,
            strokes,
            clipping,
            ..
        } = painting;
        let size = state.text.size;
        let scale = state.text.horizontal_scale;

        let word_gap = Self::word_gap(&font, size);
        let vertical = font.is_vertical();

        // §14.8.2.5.3: inside a `ReversedChars` sequence, "the sequence of the characters as
        // found in the show string operator shall be reversed before using them. If the
        // sequence encompasses multiple show strings, only the individual characters within
        // each string shall be reversed." So the readback of *this* string is collected per
        // code and appended backwards, and the reversal is per code rather than per `char`:
        // what the clause reverses are the characters the show string states, and one code
        // may map to several — a ligature's `/ToUnicode` says `fi`, which reversing by `char`
        // would spell `if`.
        //
        // The inferred word breaks `separate_text` adds are suppressed inside the string for
        // the same clause: such a block "may have a SPACE (U+0020) character or other
        // whitespace characters at the beginning or end to indicate a word break … but shall
        // not contain interior SPACE characters", so a break is something the file states
        // rather than something a gap implies — and the glyphs of a reversed string run
        // against the writing direction, where a gap means nothing.
        let reversing = self.reversed_chars > 0;
        let mut pieces: Vec<String> = Vec::new();
        // The quadrilaterals of a reversed string, in the order the glyphs were *placed*, so
        // that they can be paired with their pieces when those are appended backwards.
        let mut reversed_quads: Vec<[f32; 8]> = Vec::new();
        // One show string's worth of glyph coverage, applied to the font's tally once at the
        // end: see `tally_glyph` for why it is not applied per code.
        let mut coverage = Coverage::default();

        // Table 120's `/Ascent` and `/Descent`, which say how tall this font's line is. Read
        // once per show operation: they are a property of the font. Table 120 requires neither
        // of a Type 3 font, so its box is the em box.
        let extent = match &font {
            Font::Program(program) => program.extent(),
            Font::Type3(_) => (1.0, 0.0),
        };

        // One separation decision per show string, taken before its first glyph, because
        // §9.4.4 leaves nothing inside one to infer from. The clause's combined displacement
        // is `tx = ((w0 − Tj/1000) × Tfs + Tc + Tw) × Th`, and between two codes of one
        // string the `Tj` term is absent: what separates them is the first glyph's own width
        // plus `Tc`, which applies to every pair alike and is tracking rather than a word
        // break, plus `Tw`, which §9.3.3 applies to the single-byte code 32 alone. So the only
        // word gap a show string can state is that code, and `Font::text` reads it as the
        // character §9.3.3 names rather than as a distance. The separation *between* show
        // operations still has a position to read, which is where `Tj`'s adjustment and every
        // `Td`, `T*` and `Tm` land.
        let codes = font.decode(bytes);
        if !codes.is_empty() {
            self.separate_text(text.matrix, size, word_gap, vertical);
        }

        for code in codes {
            let advance_em = font.advance(code);
            // §9.7.4.3's second set of metrics, which decide where the glyph is drawn
            // relative to the current text position and where that position goes next.
            let program_metrics = match &font {
                Font::Program(program) => program.vertical_metrics(code),
                Font::Type3(_) => ([0.0, 0.0], [0.0, 0.0]),
            };

            let start = self.text.len();
            let read = self.read_back(&font, code, reversing.then_some(&mut pieces));
            if read == Some(Readback::Nothing)
                && let Some(gap) = font.naming_gap(code)
            {
                // §9.10.2 exhausted on a code the page *showed*. Counted rather than reported,
                // for ADR 0152's reason one column over — a report would cost the oracle a
                // judged page (trap 11) for a shortfall in the readback and not in the picture
                // — but counted rather than nothing at all, because a refusal that says nothing
                // is indistinguishable from a page with no text on it.
                //
                // Counted *by cause*, because the total cannot say whether the clause has no
                // answer or this program did not take one it states; `UnnamedCodes` has the
                // argument. `PDFVIEWER_TRACE_UNNAMED_CODE=1` names each one on stderr, the same
                // idiom the missing-glyph trace below uses, and it is what shows the glyph name
                // behind an `UnlistedName` — the one variant where what the name *is* decides
                // whose gap it is.
                if std::env::var_os("PDFVIEWER_TRACE_UNNAMED_CODE").is_some() {
                    eprintln!(
                        "UNNAMED font=/{} code={} gap={gap:?}",
                        state.text.font_name,
                        code.value()
                    );
                }
                self.codes_without_a_character.count(&gap);
            }

            // Glyph space to text space: scale by the font size, apply horizontal scaling and
            // rise, then the text matrix and the current transform. §9.4.4 calls this the text
            // rendering matrix, and both kinds of glyph are placed by it — the difference is
            // only what is placed.
            //
            // Computed here rather than inside the branch below because the *text layer* wants
            // it for every code, including the ones rendering modes 3 and 7 draw nothing for:
            // an OCR layer under a scanned page is invisible text that a person still selects.
            let glyph_to_text =
                Self::glyph_to_text(size, scale, state.text.rise, program_metrics.1);
            let glyph_to_user = glyph_to_text.then(text.matrix);
            let transform = glyph_to_user.then(state.transform);
            let quad = glyph_quad(advance_em, extent, transform);
            if reversing {
                reversed_quads.push(quad);
            } else {
                let span = start..self.text.len();
                self.text_layer.push(Placed { span, quad });
            }

            if (fills || strokes || clipping) && size != 0.0 {
                let glyph_fill_clip = self.paint_clip(state, true);
                match &font {
                    Font::Program(program) => {
                        if let Some(outline) = program.outline(code) {
                            self.show_program_glyph(
                                &outline,
                                [transform, glyph_to_user],
                                (state, glyph_fill_clip),
                                text,
                                painting,
                            );
                            coverage.drawn = coverage.drawn.saturating_add(1);
                        } else if program.uncovered_character(code).is_some() {
                            // §9.10.2 gave this code a character and the substitute face has
                            // no glyph for it, so a mark the document states is not made.
                            // Tallied rather than reported here: see `glyph_coverage`.
                            coverage.empty = coverage.empty.saturating_add(1);
                            coverage.uncovered = coverage.uncovered.saturating_add(1);
                        } else if read.is_some_and(Readback::names_a_mark) {
                            // The program answered with no outline for a code §9.10.2 *did*
                            // name, so a character the document states did not reach the page.
                            // One of these is not news — a producer's deliberate `.notdef` is
                            // one — but a font every one of whose codes comes back empty has
                            // drawn nothing the document asked for, which is the condition the
                            // report below applies. So the tally is the same either way, and
                            // what the two arms separate is the *measurement*: whether a mark
                            // was missed at all.
                            coverage.empty = coverage.empty.saturating_add(1);
                            // §9.6.5.4 and §9.7.4.2 state the routes from a code to a glyph,
                            // and this asks which of two things happened at the end of one.
                            // A code that reached a glyph the program contains has been
                            // answered: what that glyph draws is the program's own statement,
                            // and a glyph with no contours states a mark of nothing — which is
                            // how every sfnt in existence stores a space. A code that reached
                            // no glyph, or reached `.notdef`, was not answered: §9.6.5.2 makes
                            // `.notdef` what is shown when "an encoding maps to a character
                            // name that does not exist in the Type 1 font program", and
                            // §9.7.6.3 makes CID 0 what is substituted when "no glyph exists
                            // for that CID", so glyph 0 is the program saying it has none.
                            let blank = program
                                .glyph_index(code)
                                .is_some_and(|glyph| glyph != pdf_font::NOTDEF_GLYPH);
                            if blank {
                                self.codes_reaching_a_blank_glyph =
                                    self.codes_reaching_a_blank_glyph.saturating_add(1);
                            } else {
                                self.codes_without_a_glyph =
                                    self.codes_without_a_glyph.saturating_add(1);
                            }
                            // `PDFVIEWER_TRACE_MISSING_GLYPH=1` names each one on stderr, the
                            // same idiom `tests/corpus.rs` uses for a document that never
                            // returns. The readback is there because the count alone cannot
                            // tell a mark that is missing from a *space* whose font reads it
                            // back as something else, and the glyph index because that is what
                            // the two arms above are decided by.
                            if std::env::var_os("PDFVIEWER_TRACE_MISSING_GLYPH").is_some() {
                                eprintln!(
                                    "MISSING {} font=/{} code={} glyph={:?} read={:?}",
                                    if blank { "blank" } else { "absent" },
                                    state.text.font_name,
                                    code.value(),
                                    program.glyph_index(code),
                                    self.text.get(start..)
                                );
                            }
                        } else {
                            // Neither a mark made nor a mark missed, for one of two reasons,
                            // and the code could not tell them apart until the
                            // four-hundred-and-seventy-sixth session.
                            //
                            // A code that reads back as a **space** is *meant* to have no
                            // outline. Measured rather than assumed: counting one took the
                            // corpus's incomplete documents from 79 to 109, and twenty-two of
                            // the thirty new reports named a single code (trap 11 — print what
                            // a condition matched before trusting it).
                            //
                            // A code §9.10.2 could **not name** is a different thing wearing
                            // the same clothes: the clause's own answer is "there is no way to
                            // determine what the character code represents", so nothing here
                            // knows whether a mark was owed, and reporting a font on that
                            // evidence would be a guess that costs the oracle a judged page.
                            // It is counted where it belongs instead — `codes_without_a_character`
                            // above. The test used to be `self.text[start..]` all whitespace,
                            // which an empty slice satisfies vacuously, so the two were one
                            // branch *and were blind inside §14.8.2.5.3's reversal*, where a
                            // code's readback never lands in that slice at all.
                            //
                            // `None` — a code inside a Type 3 glyph description — is here for a
                            // third reason: what such a code is, and whether it drew, are
                            // §9.6.4's questions about the glyph rather than this page's.
                        }
                    }
                    Font::Type3(type3) => {
                        // §9.3.6 on a Type 3 font: the glyph description is run for every
                        // mode but 3 and 7 — which is exactly `fills || strokes`, since the
                        // description does its own painting and the mode's choice between
                        // filling and stroking has nothing to apply to — and "If text
                        // rendering mode is set to a value of 4, 5, 6 or 7, nothing shall be
                        // added to the clipping path."
                        if fills || strokes {
                            self.glyphs = self.glyphs.saturating_add(1);
                            self.draw_type3_glyph(
                                type3,
                                code.value(),
                                state,
                                transform,
                                resources,
                                form_depth,
                            );
                            if painting.knockout_can_show {
                                // A Type 3 glyph's ink is whatever its description painted,
                                // which is not knowable without running it again.
                                text.note_knockout(None);
                            }
                        }
                    }
                }
            }

            // Word spacing applies only to the single-byte code 32 (§9.3.3), which is a rule
            // about the code's encoded length rather than its value — see
            // `pdf_font::Code::takes_word_spacing`. A Type 3 font's codes are all one byte,
            // Table 110 giving it `/FirstChar` and `/LastChar`, so the same test serves both
            // kinds of font.
            let word = if code.takes_word_spacing() {
                state.text.word_spacing
            } else {
                0.0
            };
            let displacement = if vertical {
                program_metrics.0[1]
            } else {
                advance_em
            };
            text.matrix = Self::advance_step(
                displacement,
                size,
                state.text.char_spacing + word,
                scale,
                vertical,
            )
            .then(text.matrix);
            self.text_cursor = Some((text.matrix.e, text.matrix.f));
        }

        if coverage.drawn > 0 || coverage.empty > 0 {
            self.tally_glyph(&state.text.font_name, coverage);
        }
        self.append_reversed(&pieces, reversed_quads);
    }

    /// §14.8.2.5.3's reversal: one show string's readback, appended backwards.
    ///
    /// Nothing about the *drawing* changed — the glyphs were placed where their positions put
    /// them, and what the clause reverses is what a reader extracts or hears — so each piece
    /// keeps the quadrilateral of the glyph that produced it and only their order changes.
    fn append_reversed(&mut self, pieces: &[String], quads: Vec<[f32; 8]>) {
        for (piece, quad) in pieces.iter().zip(quads).rev() {
            let start = self.text.len();
            self.text.push_str(piece);
            self.text_layer.push(Placed {
                span: start..self.text.len(),
                quad,
            });
        }
    }

    /// Appends one code's text to the readback, or to the string being reversed.
    ///
    /// The two destinations are §14.8.2.5.3's whole difference, and the reversal is per *code*
    /// rather than per `char` because what the clause reverses are the characters "as found in
    /// the show string operator" — one code may map to several, and a ligature's `/ToUnicode`
    /// saying `fi` would come back as `if` from a reversal that worked on characters.
    ///
    /// Returns what the code contributed, or `None` where it contributed nothing *because it is
    /// not the page's text* — a code inside a Type 3 glyph description, below. That is a
    /// different thing from [`Readback::Nothing`], which is a code the page showed and §9.10.2
    /// could not name, and the caller counts only the second.
    fn read_back(
        &mut self,
        font: &Font,
        code: Code,
        reversed: Option<&mut Vec<String>>,
    ) -> Option<Readback> {
        // **Not from inside a Type 3 glyph description.** §9.6.4 makes a glyph description a
        // way of *painting* one glyph — "a glyph in a Type 3 font shall be defined by a
        // content stream that contains the operators that paint the glyph" — so the text
        // operators inside it are the glyph's implementation and not text of the page. What
        // the page showed is the code that invoked it, and §9.10.2 is what says what *that*
        // means.
        //
        // `pr4922.pdf` is the case, and it is why this is here: its Type 3 glyphs are drawn
        // by showing a character of another font, so before this line the page read back
        // "pp2200--4400::" — every character twice, once from the outer code and once from
        // the description that draws it.
        if self.glyph_depth > 0 {
            return None;
        }
        Some(if let Some(pieces) = reversed {
            let mut piece = String::new();
            let named = font.text(code, &mut piece);
            let read = Readback::of(named, &piece);
            pieces.push(piece);
            read
        } else {
            let start = self.text.len();
            let named = font.text(code, &mut self.text);
            Readback::of(named, self.text.get(start..).unwrap_or_default())
        })
    }

    /// How wide a gap has to be before it means a word break rather than kerning.
    ///
    /// Measured against the font's own space, because that is what a word break is made of.
    /// A fixed fraction of the font size cannot work: a title set with loose tracking moves
    /// each glyph further than a body-text space, and judging it by size alone spells
    /// "Clarification" as "Clar if ic at ion".
    ///
    /// Taken from the magnitude of the size because §9.3.1's NOTE says "Negative text font
    /// size is permitted", and a negative threshold is below every gap there is — which would
    /// have put a space between every pair of glyphs in the extracted text.
    fn word_gap(font: &Font, size: f32) -> f32 {
        let space_em = font.advance(Code::single_byte(32));
        if space_em > 0.0 {
            space_em * size.abs() * 0.6
        } else {
            size.abs() * 0.25
        }
    }

    /// Adds a space or a newline to the readback where the glyphs' positions imply one.
    ///
    /// A content stream has no notion of words or lines; it has positions. A glyph placed
    /// against the writing direction, or well off the line, began a new line, and one placed
    /// a noticeable gap along it began a new word. These are the only two separators
    /// reconstructed, because anything more is layout analysis and belongs to a consumer of
    /// this text rather than to the drawing pass. `pdftotext` does do that analysis, which is
    /// why the comparison normalises whitespace away.
    ///
    /// The two axes swap in writing mode 1, where a column advances downward and a new column
    /// is a new line.
    ///
    /// **A heuristic the standard names as one.** §14.8.2.6.2 requires a *tagged* producer to
    /// state its word breaks — "any white-space characters that would be present to separate
    /// words in a pure text representation shall be present in the tagged PDF representation
    /// of the text" — and says what that spares a reader: "the PDF processor can determine
    /// word breaks without having to rely on heuristics based on information such as glyph
    /// positioning on the page, font changes, or glyph sizes". An untagged page leaves exactly
    /// that reliance, so what is below is a **choice** rather than a clause obeyed, and the
    /// standard's own sentence is what says which kind of thing it is.
    ///
    /// **It is called once per show operation and not once per code**, because §9.4.4 leaves
    /// nothing inside one show string to read: see the comment at the call site for the
    /// decomposition, and `Font::text` for the one gap a show string *can* state.
    fn separate_text(&mut self, matrix: Transform, size: f32, word_gap: f32, vertical: bool) {
        // The text-space origin under the matrix is simply its translation.
        let here = (matrix.e, matrix.f);
        let Some((last_x, last_y)) = self.text_cursor else {
            return;
        };
        let (along, across) = if vertical {
            (last_y - here.1, here.0 - last_x)
        } else {
            (here.0 - last_x, here.1 - last_y)
        };
        if across.abs() > size.abs() * 0.5 {
            self.text.push('\n');
            self.inferred_separators = self.inferred_separators.saturating_add(1);
        } else if along > word_gap {
            self.text.push(' ');
            self.inferred_separators = self.inferred_separators.saturating_add(1);
        }
    }

    /// Glyph space to text space: the font size, the horizontal scaling, and the rise.
    ///
    /// §9.2.4 adds one term in writing mode 1: "the glyph position shall be described by a
    /// position vector from the origin used for horizontal writing (origin 0) to the origin
    /// used for vertical writing (origin 1)". The outline is stated relative to origin 0 and
    /// the text position *is* origin 1, so the glyph moves back by `v`, which is zero for
    /// every font in writing mode 0.
    fn glyph_to_text(size: f32, scale: f32, rise: f32, position: [f32; 2]) -> Transform {
        Transform::new(
            size * scale,
            0.0,
            0.0,
            size,
            -position[0] * size * scale,
            (-position[1]).mul_add(size, rise),
        )
    }

    /// §9.4.4's combined displacement, as the translation it applies to the text matrix.
    ///
    /// The clause computes `tx` in horizontal writing mode and `ty` in vertical, "the
    /// variable corresponding to the other writing mode shall be set to 0", and the two
    /// differ in one term: the horizontal scaling multiplies `tx` alone, because `Th` scales
    /// the *width* of a line rather than the advance along it. Character and word spacing are
    /// added to whichever component applies.
    fn advance_step(
        displacement: f32,
        size: f32,
        spacing: f32,
        scale: f32,
        vertical: bool,
    ) -> Transform {
        if vertical {
            Transform::translate(0.0, displacement.mul_add(size, spacing))
        } else {
            Transform::translate(displacement.mul_add(size, spacing) * scale, 0.0)
        }
    }

    /// Fills one glyph outline, which a pattern makes more than a `Fill` command.
    ///
    /// §9.2.3 lets a glyph be painted "in any colour", and §8.7.2 makes a pattern one: "All
    /// patterns shall be treated as colours". A *tiling* pattern is not a paint, though — it
    /// is a cell replayed across an area — so a glyph filled with one is its outline tiled,
    /// exactly as a path is. The transform is the *glyph's* rather than the text object's,
    /// because the outline is in glyph space.
    fn fill_glyph(
        &mut self,
        outline: &Arc<Path>,
        transform: Transform,
        state: &GraphicsState,
        clip: Option<ClipId>,
    ) {
        // Borrowed rather than cloned: this runs once per glyph, and cloning the whole
        // `Option<PatternPaint>` would bump a shading's refcount on every glyph of a page whose
        // text is painted with one.
        if let Some(PatternPaint::Tiling(tiling)) = &state.fill_pattern {
            let tiling = Rc::clone(tiling);
            self.tile(outline, transform, FillRule::NonZero, &tiling, state);
            return;
        }
        self.list.push(Command::Fill {
            // The font hands out shared outlines and the display list keeps them shared: a
            // page of text is the same few dozen glyphs over and over, so this is a refcount
            // rather than a copy of the segments.
            path: Arc::clone(outline),
            transform,
            // Glyph outlines are non-zero filled; even-odd would hollow out counters that
            // overlap, such as in a bold 'B'.
            fill_rule: FillRule::NonZero,
            paint: state.fill_paint(),
            clip,
            mask: state.soft_mask,
            blend: state.blend,
        });
    }

    /// Strokes one glyph outline, ISO 32000-2 §9.3.6 rendering modes 1, 2, 5 and 6.
    ///
    /// `glyph_to_user` maps the outline from glyph space to the *user* space in effect,
    /// which is the whole reason this is not two lines beside the fill. The clause puts the
    /// stroke's parameters in that space:
    ///
    /// > The graphics state parameters affecting those operations, such as line width, shall
    /// > be interpreted in user space rather than in text space.
    ///
    /// A [`Command::Stroke`]'s width and dash lengths are in its path's own space, so
    /// leaving the outline in em units would have divided the width by the font size and
    /// stretched it by the horizontal scaling — an 11-point glyph would have been outlined
    /// about eleven times too thickly, and a horizontally scaled one anisotropically. Moving
    /// the geometry instead is exact for any text matrix, including one that shears; the
    /// cost is a copy of the outline per stroked glyph, which is paid only by the modes that
    /// stroke and never on the ordinary fill path.
    fn stroke_glyph(
        &mut self,
        outline: &Arc<Path>,
        glyph_to_user: Transform,
        state: &GraphicsState,
    ) {
        let mut in_user_space = Path::new();
        in_user_space.extend_transformed(outline, glyph_to_user);
        let glyph_stroke_clip = self.paint_clip(state, false);
        self.list.push(Command::Stroke {
            path: Arc::new(in_user_space),
            transform: state.transform,
            stroke: state.stroke.clone(),
            paint: state.stroke_paint(),
            clip: glyph_stroke_clip,
            mask: state.soft_mask,
            blend: state.blend,
        });
    }

    /// Turns the glyph outlines a text object accumulated into a clip, at its `ET`.
    ///
    /// ISO 32000-2 §9.3.6:
    ///
    /// > At the end of the text object identified by the ET operator the accumulated glyph
    /// > outlines, if any, shall be combined into a single path, treating the individual
    /// > outlines as subpaths of that path and applying the non-zero winding number rule
    /// > (see 8.5.3.3.2, "Non-zero winding number rule"). The current clipping path in the
    /// > graphics state shall be set to the intersection of this path with the previous
    /// > clipping path.
    ///
    /// Intersection is what the display list's `parent` chain already means, so the new clip
    /// is a child of the one in effect. It is set on the live graphics state rather than on a
    /// saved copy because the clause continues: "It remains in effect until a previous
    /// clipping path is restored by an invocation of the Q operator" — so it outlives the
    /// text object, and `Q` is the only thing that ends it.
    ///
    /// # An empty accumulator is not an empty clip
    ///
    /// > If no glyphs are shown or if the only glyphs shown have no outlines (for example,
    /// > if they are ASCII SPACE characters (20h)), no clipping shall occur.
    ///
    /// Clipping to an empty path would hide everything drawn after the text object, which is
    /// the opposite of what the clause says and would be invisible to every metric this tree
    /// owns except pixels somebody else produced. A text object in mode 7 showing one space
    /// is not a hypothetical: it is what a producer emits when a line of OCR text happens to
    /// be blank.
    pub(super) fn end_text_object(&mut self, text: &mut TextObject, state: &mut GraphicsState) {
        // §9.3.8's knockout is a property of the finished object, so this is where it can be
        // judged: two or more glyphs marked the page under a paint that composites, and `Tk`
        // asked for them to knock one another out instead.
        //
        // The condition is deliberately narrow. Treating every text object drawn while `Tk`
        // is true as a group would wrap almost every page in the world, since true is the
        // initial value, and would say nothing: with opaque glyphs and the Normal blend mode
        // the two models produce identical pixels.
        //
        // The clause states the construction exactly, and it is the one §11.4.6 built in the
        // seventy-first session: "the behaviour shall be equivalent to treating the entire
        // text object as if it were a non-isolated knockout transparency group … where each
        // glyph is an individual element in that group's transparency stack", after which
        // "the group results shall be composited with the backdrop, using the Normal blend
        // mode and alpha and soft mask values of 1.0" — which is this command's four other
        // fields. The graphics state is *not* reset for the elements, unlike §11.6.6's group
        // XObject, and it is not: each glyph command already carries the alpha, mask and
        // blend mode in force when it was shown.
        //
        // §11.7.4.4's implicit group is decided here too, and it has to be: a glyph shown in
        // mode 2 or 6 owes a knockout group of its own fill and stroke, and where the object
        // above is built that group is *inside* it. One knockout group inside another is not
        // something either backend can state — `knockout_is_drawable` rejects an element that
        // is a group — and it does not have to be stated, because it computes the same
        // picture flat: in a knockout group every element composites with the initial
        // backdrop, so at each point the topmost element wins, and nesting cannot change
        // which element that is. So the whole-object group subsumes every glyph's, and the
        // per-glyph groups are built only where there is no whole-object group to be inside.
        let knockout_owed = text.knockout_owed;
        if knockout_owed || !text.combined.is_empty() {
            let glyphs = text.composited.len();
            let elements = self.list.split_off_commands(text.start);
            if knockout_owed && knockout_is_drawable(&elements) && !self.alpha_is_shape {
                self.list.push(Command::Group {
                    commands: elements,
                    alpha: 1.0,
                    clip: None,
                    mask: None,
                    blend: BlendMode::Normal,
                    isolated: true,
                    knockout: true,
                    blending: None,
                });
            } else {
                if knockout_owed {
                    self.note(Unsupported::TextKnockout { glyphs });
                }
                self.push_combined_glyphs(elements, text);
            }
        }
        text.knockout_owed = false;
        text.combined.clear();
        text.composited.clear();

        let path = std::mem::take(&mut text.clip);
        if path.is_empty() {
            return;
        }
        let clip = Clip {
            path,
            // The outlines were mapped into page space as they were collected, because one
            // path cannot carry one transform per glyph.
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: state.clip,
        };
        match self.list.add_clip(clip) {
            Ok(id) => state.clip = Some(id),
            Err(_) => self.note(Unsupported::LimitReached { limit: "max_clips" }),
        }
    }

    /// Draws one glyph of an outline font, in whichever of §9.3.6's three operations apply.
    ///
    /// `places` is the glyph's two transforms: into page space, and into user space — the
    /// second is what a stroke needs, since §9.3.6 makes the stroke's width a user-space
    /// quantity like any other path's.
    fn show_program_glyph(
        &mut self,
        outline: &Arc<Path>,
        places: [Transform; 2],
        painted: (&GraphicsState, Option<ClipId>),
        text: &mut TextObject,
        painting: GlyphPainting,
    ) {
        let [transform, glyph_to_user] = places;
        let (state, fill_clip) = painted;
        if painting.fills || painting.strokes {
            // Marked the page; see `Interpretation::glyphs`. An empty outline — a space in a
            // font that has one — is a glyph the font drew and is counted, because the
            // question this answers is what *kind* of page this is.
            self.glyphs = self.glyphs.saturating_add(1);
        }
        let parts_at = self.list.command_count();
        if painting.fills {
            self.fill_glyph(outline, transform, state, fill_clip);
        }
        if painting.strokes {
            self.stroke_glyph(outline, glyph_to_user, state);
        }
        // §11.7.4.4 makes this glyph's fill and stroke one object; the range is recorded and
        // `ET` decides what to build from it. Fewer than two commands is a glyph that marked
        // the page once — an empty outline, or a fill a tiling pattern drew nothing for — and
        // there is nothing for it to composite with.
        if painting.combining && self.list.command_count() > parts_at.saturating_add(1) {
            text.combined.push((parts_at, self.list.command_count()));
        }
        if painting.clipping {
            // §9.3.6 wants "a single path, treating the individual outlines as subpaths of
            // that path", and the glyphs of one text object have as many transforms as there
            // are glyphs — so the transform is baked in here and the clip carries none. Note
            // that a hidden layer still reaches this line.
            text.clip.extend_transformed(outline, transform);
        }
        if painting.knockout_can_show {
            text.note_knockout(outline_bounds(outline, transform));
        }
    }

    /// Pushes a text object's commands back, wrapping §11.7.4.4's fill-and-stroke pairs.
    ///
    /// ISO 32000-2 §11.7.4.4, of a combined fill and stroke — which "include the B , B\* , b ,
    /// and b\* operators … and the painting of glyphs with text rendering mode 2 or 6":
    ///
    /// > In all other cases, a non-isolated knockout group shall be established. Within the
    /// > group, the fill and stroke shall be performed with their respective prevailing alpha
    /// > constants and the prevailing blend mode. The group results shall then be composited
    /// > with the backdrop, using an alpha value of 1.0 and the Normal blend mode.
    ///
    /// "All other cases" is every case here: the first bullet needs overprinting enabled, and
    /// §8.6.7 is why this device never enables it (ADR 0028). The construction is therefore
    /// identical to the one the `B` operator gets in [`Interpreter::paint_path`], and NOTE 2
    /// says what it is for — "to avoid having a non-opaque stroke composite with the result of
    /// the fill in the region of overlap, which would produce a double border effect".
    ///
    /// A pair the backends cannot draw as a knockout — one carrying a soft mask, or a fill a
    /// tiling pattern turned into a group — is pushed flat and named once for the whole text
    /// object, because a report per glyph would name the same gap a hundred times on one line.
    fn push_combined_glyphs(&mut self, elements: Vec<Command>, text: &TextObject) {
        let mut owed = false;
        let mut pairs = text.combined.iter().peekable();
        let mut index = text.start;
        let mut rest = elements.into_iter();
        while let Some(command) = rest.next() {
            let pair = pairs.next_if(|(from, _)| *from == index).copied();
            let Some((from, to)) = pair else {
                self.list.push(command);
                index = index.saturating_add(1);
                continue;
            };
            let mut parts = vec![command];
            parts.extend(
                rest.by_ref()
                    .take(to.saturating_sub(from).saturating_sub(1)),
            );
            index = to;
            if knockout_is_drawable(&parts) && !self.alpha_is_shape {
                self.list.push(Command::Group {
                    commands: parts,
                    alpha: 1.0,
                    clip: None,
                    mask: None,
                    blend: BlendMode::Normal,
                    isolated: true,
                    knockout: true,
                    blending: None,
                });
            } else {
                owed = true;
                for part in parts {
                    self.list.push(part);
                }
            }
        }
        if owed {
            self.note(Unsupported::CompositedInParts {
                detail: "a glyph filled and stroked by text rendering mode 2 or 6",
            });
        }
    }

    /// Runs one Type 3 glyph description, ISO 32000-2 §9.6.4.
    ///
    /// `text_rendering` is §9.4.4's text rendering matrix — everything the glyph is placed by
    /// except the font's own matrix, which is applied here because it is the font's business
    /// rather than the text object's.
    ///
    /// The steps §9.6.4 lays out for each character code are all here or in
    /// [`crate::type3::Type3Font`]: the encoding and `/CharProcs` lookups are the font's, and
    /// this does the rest — save the state, set the CTM, run the description, restore.
    fn draw_type3_glyph(
        &mut self,
        font: &crate::type3::Type3Font,
        code: u32,
        state: &GraphicsState,
        text_rendering: Transform,
        resources: &Dictionary,
        form_depth: usize,
    ) {
        // §9.6.4 b): "If the name is not present as a key in CharProcs, no glyph shall be
        // painted." Neither that nor a code the encoding does not name is a failure — both
        // are defined outcomes — so neither is reported, and both still advance the text
        // position, which the caller does whatever happens here.
        let Some(glyph) = font.glyph(self.document, code) else {
            return;
        };

        // A glyph description may show text in another Type 3 font, which is a recursion a
        // file can build a cycle out of — `ContentStreamCycleType3insideType3.pdf` in the
        // corpus is exactly that. It shares the bound with form XObjects because it is the
        // same danger and the same cost: a nested content stream.
        if form_depth >= MAX_FORM_DEPTH {
            self.note(Unsupported::LimitReached {
                limit: "MAX_FORM_DEPTH",
            });
            return;
        }

        // Table 110's `/CharProcs`: each value "shall be a content stream that constructs and
        // paints the glyph for that character. The stream shall include as its first operator
        // either d0 or d1 , followed by operators describing one or more graphics objects." So
        // §7.8.2's prefix rule reaches a glyph description, and this clause makes the prefix
        // *faithful* in a way the general argument does not: `d0`/`d1` is required to be first,
        // so any prefix carrying a mark carries the glyph's own declaration ahead of it, and
        // Table 110's `/Widths` — not the description — supplies the advance, so what the
        // damage costs is marks inside this glyph and never the position of the next one.
        // Named by the glyph rather than by the code, because §9.6.4 step b) keys `/CharProcs`
        // that way and two codes may reach one description. `glyph` above returned `Some`, so
        // the encoding does name this code; the fallback is unreachable and is written rather
        // than unwrapped because nothing in the type system says so.
        let name = font.glyph_name(code).unwrap_or("?").to_owned();
        let Some(data) = self.content_stream(
            &glyph,
            &format!("a Type 3 glyph description /{name} (§9.6.4)"),
        ) else {
            self.note(Unsupported::Font {
                detail: format!("Type 3 glyph for code {code} could not be decoded"),
            });
            return;
        };

        // §9.6.4: "When the glyph description begins execution, the current transformation
        // matrix (CTM) shall be the concatenation of the font matrix (FontMatrix in the
        // current font dictionary) and the text space that was in effect at the time the
        // text-showing operator was invoked". Everything else is inherited: "Aside from the
        // CTM, the graphics state shall be inherited from the graphics state at the point of
        // invocation of the text-showing operator" — which is what cloning it does, and the
        // clone is also step c)'s save and restore, since nothing the description changes can
        // reach the caller's copy.
        let mut inner = state.clone();
        inner.transform = font.font_matrix().then(text_rendering);

        // §7.8.3's first step for a glyph description, which Errata Collection 3 put in front
        // of §9.6.4's own rule (Issue #128): "the stream dictionary of that glyph description
        // content stream". Resolved here rather than in `Type3Font` because the font holds the
        // `/CharProcs` dictionary and not the decoded streams — a glyph is read when it is
        // drawn — and cloned only where the stream states one, which is the rare case.
        let stated = self
            .document
            .get_key(&glyph.dict, "Resources")
            .as_dict()
            .cloned();

        let saved_uncoloured = self.uncoloured;
        self.glyph_depth = self.glyph_depth.saturating_add(1);
        self.run(
            &data,
            font.resources(stated.as_ref(), resources),
            &inner,
            form_depth.saturating_add(1),
        );
        self.glyph_depth = self.glyph_depth.saturating_sub(1);
        // `d1` inside the description raised this; the description is over. Restoring rather
        // than clearing is what lets an uncoloured glyph invoke another one without the
        // inner one's end re-enabling colour for the rest of the outer.
        self.uncoloured = saved_uncoloured;
    }
}
