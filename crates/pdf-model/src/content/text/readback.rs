//! The page's readback: what each code reads back as, the glyph boxes, and where word gaps fall
//! (ADR 1502, 1515).

use pdf_font::Code;
use pdf_render::{Point, Transform};

use super::word_gaps::{
    HYPHENS, LEAST_WORD_GAP_SHARE, NOMINAL_SPACE_EM, Provisional, Step, WORD_GAP_SHARE, WordGaps,
};
use crate::content::Interpreter;
use crate::content::font::Font;
use crate::content::report::Placed;

/// What one code contributed to the page's readback.
///
/// Three states rather than a string, because the difference between the last two decides
/// whether a code that reached no outline is a mark the reader lost. A code that reads back as
/// a space is *meant* to have no outline; a code §9.10.2 could not name says nothing either
/// way, and taking the second for the first is a wrong answer that reports nothing.
///
/// **They are separate states because a test on the buffer cannot tell them apart** (ADR
/// 0311): `self.text[start..].chars().all(char::is_whitespace)` is satisfied vacuously by an
/// empty slice, so a font that named none of its codes would read as a page of spaces, and
/// inside §14.8.2.5.3's reversal the readback is collected per code and appended after the
/// string, so *every* code's slice is empty there. Asking the font what it said, rather than
/// asking the buffer what arrived, answers both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Readback {
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
    ///
    /// **It is a question about the reader and not about the picture** (ADR 0520).
    /// Whether the *program* answered a code is decided by the glyph the code reached, and
    /// needs no character: a route that ends at no glyph at all ended without an answer
    /// whatever §9.10.2 could or could not say about the code. So this decides which readback
    /// lost something and never whether a mark did.
    pub(super) fn names_a_mark(self) -> bool {
        self == Self::Characters
    }
}

/// A glyph's box, mapped by §9.4.4's text rendering matrix.
///
/// The corners in glyph space are (0, descent), (advance, descent), (advance, ascent) and
/// (0, ascent) — the advance along the baseline and the font's own reach above and below it —
/// and they go round the quadrilateral in that order so that a consumer can draw it as a
/// polygon without sorting.
pub(super) fn glyph_quad(advance: f32, extent: (f32, f32), transform: Transform) -> [f32; 8] {
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
    /// §14.8.2.5.3's reversal: one show string's readback, appended backwards.
    ///
    /// Nothing about the *drawing* changed — the glyphs were placed where their positions put
    /// them, and what the clause reverses is what a reader extracts or hears — so each piece
    /// keeps the quadrilateral of the glyph that produced it and only their order changes.
    pub(super) fn append_reversed(&mut self, pieces: &[String], quads: Vec<[f32; 8]>) {
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
    pub(super) fn read_back(
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

    /// The space a gap is measured against before it means a word break rather than kerning.
    ///
    /// The font's own space, because that is what a word break is made of.
    /// A fixed fraction of the font size cannot work: a title set with loose tracking moves
    /// each glyph further than a body-text space, and judging it by size alone spells
    /// "Clarification" as "Clar if ic at ion".
    ///
    /// Taken from the magnitude of the size because §9.3.1's NOTE says "Negative text font
    /// size is permitted", and a negative threshold is below every gap there is — which would
    /// have put a space between every pair of glyphs in the extracted text.
    ///
    /// **Both numbers are choices** (ADRs 1502, 1515): §9.3 and §9.4.4 state no quantity that
    /// separates words, and §14.8.2.6.2 calls any such reading a heuristic. [`NOMINAL_SPACE_EM`] is
    /// the space a font stating none is read with, and the share of it a gap has to exceed is the
    /// font's own, decided by [`Self::settle_word_gaps`].
    pub(super) fn space(font: &Font, size: f32) -> f32 {
        let stated = font.advance(Code::single_byte(32));
        let space_em = if stated > 0.0 {
            stated
        } else {
            NOMINAL_SPACE_EM
        };
        space_em * size.abs()
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
    ///
    /// **The gap is read in text space, where the advance is stated** (ADR 1490). ISO 32000-2
    /// §9.4.4:
    ///
    /// > Both the glyph's shape and its displacement (horizontal or vertical) shall be
    /// > interpreted in text space.
    ///
    /// so the distance a show string moved from where the last glyph left the pen is compared
    /// with `space` — a displacement — in the space that displacement is in:
    /// [`Self::text_space_step`] takes it back through the text matrix's linear part and out of
    /// `Th`, and the direction a glyph advances there is the sign of `Tfs`. Read along user-space
    /// x instead, a mirroring `Tm` turns every `TJ` adjustment that closes a gap into one that
    /// opens it, and a `Tm` that scales measures a gap in different units from the threshold.
    ///
    /// **A gap along the line is decided when the page is finished** (ADR 1515). Its threshold is
    /// a share of `space` that the font's own steps on the page choose, so here every gap wider
    /// than [`LEAST_WORD_GAP_SHARE`] of a space — the least that share can be — leaves a
    /// provisional space, and every gap is recorded in [`Self::word_gaps`] for
    /// [`Self::settle_word_gaps`] to keep or take back.
    pub(super) fn separate_text(
        &mut self,
        font: &Font,
        matrix: Transform,
        size: f32,
        scale: f32,
        space: f32,
        run: usize,
    ) {
        let vertical = font.is_vertical();
        // Taken whatever this step turns out to be: the replacement that asked for it ended a
        // line only if the very next show operation starts another.
        let joins_line = std::mem::take(&mut self.word_gaps.joins_next_line);
        // The text-space origin under the matrix is simply its translation.
        let here = (matrix.e, matrix.f);
        let Some((last_x, last_y)) = self.text_cursor else {
            return;
        };
        let (x, y) = Self::text_space_step(matrix, scale, (here.0 - last_x, here.1 - last_y));
        // §9.4.4's `tx` and `ty` both carry `Tfs` as a factor, so a negative size — which §9.3.1's
        // NOTE permits — advances the other way, and "along" is measured in that direction. In
        // vertical writing §9.7.4.3's `w1` is negative for a column running down the page, which
        // is the direction the reading already takes as forward.
        let forward = if size < 0.0 { -1.0 } else { 1.0 };
        let (along, across) = if vertical {
            (-y * forward, x)
        } else {
            (x * forward, y)
        };
        if across.abs() > size.abs() * 0.5 {
            if joins_line {
                return;
            }
            self.word_gaps.last_separator = Some(self.text.len());
            self.text.push('\n');
            self.inferred_separators = self.inferred_separators.saturating_add(1);
        } else if along > 0.0 {
            let provisional = if along > space * LEAST_WORD_GAP_SHARE {
                let at = self.text.len();
                self.word_gaps.last_separator = Some(at);
                self.text.push(' ');
                self.inferred_separators = self.inferred_separators.saturating_add(1);
                Provisional::At(at)
            } else {
                Provisional::None
            };
            let font = self.word_gaps.font_index(font);
            self.word_gaps.steps.push(Step {
                font,
                along,
                space,
                run,
                provisional,
            });
        }
    }

    /// Cuts the readback to `len` bytes, and forgets the provisional spaces cut with it.
    ///
    /// The one way the readback is shortened, so that no provisional space of [`WordGaps`] can
    /// name a byte that something else has since been written to.
    pub(in crate::content) fn truncate_readback(&mut self, len: usize) {
        self.text.truncate(len);
        self.word_gaps.discard_from(len);
    }

    /// Cuts away what a section with §14.9.4's `/ActualText` read back, before its replacement is
    /// written.
    ///
    /// > The ActualText value shall be used as a replacement, not a description, for the content,
    /// > providing text that is equivalent to what a person would see when viewing the content.
    ///
    /// What is replaced is **the content**, and two things this reader writes beside the content are
    /// not part of it (ADR 1948):
    ///
    /// - **The separator inferred before the section's first glyph stays.** It stands for the gap
    ///   between what came before and the section — [`Self::separate_text`] writes it when that
    ///   glyph is shown, so it lands inside the section's range — and cutting it with the glyphs
    ///   ran `the` into a replaced `first` and a page's last line into its replaced folio. Where
    ///   the section follows another replacement, §14.9.4's "treated as if no word break is present
    ///   between them" still removes it, because that rule is applied to the readback as it stands
    ///   after this cut.
    /// - **A replacement that takes away the hyphen its content ended on joins its line to the
    ///   next.** The clause's own EXAMPLE is that case — `(Dru) Tj /Span <</ActualText (c)>> BDC
    ///   (k-) Tj EMC (ker) '` shows "the correct character content in a case where hyphenation
    ///   changes the spelling of a word", and the word is "Drucker". The line break before `ker` is
    ///   this reader's inference from position, and the producer's replacement of the line-end
    ///   hyphen is the statement that the word goes on; so the next line break inferred is
    ///   withheld. A replacement that keeps a hyphen keeps the break, as a hard hyphen at a line's
    ///   end does everywhere else in the readback.
    pub(in crate::content) fn cut_replaced(&mut self, starts_at: usize, replacement: &str) {
        let cut = self.text.get(starts_at..).unwrap_or_default();
        let separator =
            self.word_gaps.last_separator == Some(starts_at) && cut.starts_with([' ', '\n']);
        let hyphen = |text: &str| text.trim_end().ends_with(HYPHENS);
        self.word_gaps.joins_next_line = hyphen(cut) && !hyphen(replacement);
        self.truncate_readback(starts_at.saturating_add(usize::from(separator)));
    }

    /// Writes §14.9.4's replacement where the enclosed operators' readback was cut away, and gives
    /// each code they showed the whole of it.
    ///
    /// §14.9.4 makes `/ActualText` a replacement for the content rather than a description of it,
    /// so a glyph inside the sequence reads back as the replacement and nothing narrower: its span
    /// covers the replacement, rather than whatever bytes of it its own readback once occupied —
    /// which would make what a selection or a caret finds there depend on how many spaces the cut
    /// text happened to hold (ADR 1515).
    pub(in crate::content) fn push_replacement(&mut self, replacement: &str) {
        let from = self.text.len();
        self.text.push_str(replacement);
        let to = self.text.len();
        for placed in self.text_layer.iter_mut().rev() {
            if placed.span.start < from {
                break;
            }
            placed.span = from..to;
        }
    }

    /// Keeps or takes back each provisional space by its font's own threshold (ADR 1515).
    ///
    /// Run once, when the page's content and its annotations have all been walked, so that each
    /// font's threshold is read off every step it showed. A space taken back is removed from the
    /// readback and every range over it — the text layer's spans, §14.9's and §14.8.2.2's, the
    /// marked-content sequences, the associated files and the structural annotations — is moved to
    /// match; where every provisional space is kept, nothing is touched.
    pub(in crate::content) fn settle_word_gaps(&mut self) {
        let thresholds = self.word_gaps.thresholds();
        let mut removed: Vec<usize> = Vec::new();
        for step in &self.word_gaps.steps {
            let share = thresholds.get(step.font).copied().unwrap_or(WORD_GAP_SHARE);
            if step.along > step.space * share {
                continue;
            }
            match step.provisional {
                Provisional::None => {}
                Provisional::At(at) => {
                    removed.push(at);
                    self.inferred_separators = self.inferred_separators.saturating_sub(1);
                }
                Provisional::Discarded => {
                    self.inferred_separators = self.inferred_separators.saturating_sub(1);
                }
            }
        }
        self.word_gaps = WordGaps::default();
        if removed.is_empty() {
            return;
        }
        // The provisional spaces' bytes rise strictly through the walk — a cut forgets every one
        // after it — so a position moves back by the number of removed bytes before it.
        let mut kept = String::with_capacity(self.text.len().saturating_sub(removed.len()));
        let mut from = 0;
        for &at in &removed {
            kept.push_str(self.text.get(from..at).unwrap_or_default());
            from = at.saturating_add(1);
        }
        kept.push_str(self.text.get(from..).unwrap_or_default());
        self.text = kept;
        let moved =
            |position: usize| position.saturating_sub(removed.partition_point(|&at| at < position));
        let span = |range: &mut std::ops::Range<usize>| {
            *range = moved(range.start)..moved(range.end);
        };
        for placed in &mut self.text_layer {
            span(&mut placed.span);
        }
        for described in &mut self.described {
            span(&mut described.range);
        }
        for artifact in &mut self.artifacts {
            span(&mut artifact.range);
        }
        for marked in &mut self.marked {
            span(&mut marked.range);
        }
        for (range, _) in &mut self.associated {
            span(range);
        }
        for object in &mut self.structural_annotations {
            span(&mut object.range);
        }
    }
}
