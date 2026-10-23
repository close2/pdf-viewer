//! §9.4.3's showing operators: one string's codes placed, drawn and read back in a single pass.

use super::glyph_painting::{GlyphPainting, Parts};
use super::readback::{Readback, glyph_quad};
use super::{Coverage, TextObject};
use crate::content::font::Font;
use crate::content::overprint::FirstBullet;
use crate::content::report::Placed;
use crate::content::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// Adds one show string's worth of coverage to a font's tally.
    ///
    /// Per *string* rather than per glyph, which is not a style choice: the map is keyed by the
    /// resource name and a lookup per glyph costs **2%** of interpretation on the specification's
    /// own page, measured by `callgrind_interpret`. The font cannot
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
            entry.lost = entry.lost.saturating_add(counted.lost);
            if entry.shortfall.is_none() {
                entry.shortfall = counted.shortfall;
            }
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
    pub(in crate::content) fn show_text(
        &mut self,
        bytes: &[u8],
        state: &GraphicsState,
        text: &mut TextObject,
    ) {
        let Some(font) = state.text.font.clone() else {
            // Text we cannot draw is counted so the page says it is incomplete — unless the
            // layer it belongs to is off, in which case not drawing it is correct.
            if !self.is_hidden() {
                self.text_operations = self.text_operations.saturating_add(1);
            }
            return;
        };

        if font.is_metrics_only() {
            self.advance_through_a_refused_font(&font, bytes, state, text);
            return;
        }

        // The three operations of Table 104, and the two clauses that ask about the paint
        // behind them; see `GlyphPainting::read`. Mode 3 does none of the three and mode 7
        // only the last, which is what an OCR layer under a scanned image uses; either way
        // the text matrix still advances and the extracted text still accumulates, because
        // §9.3.6 requires it — "The e and f components of Tm shall be updated for each glyph
        // drawn when using text rendering mode 3 or 7 in exactly the same way as would be
        // done for other text rendering modes."
        // ISO 32000-2 §11.7.4.3's special overprinting blend mode, once per show-text operator
        // rather than once per glyph: it is a function of the graphics state, which no glyph of
        // one operator changes. `state.blend` on every page that does not overprint inside a
        // four-component blending colour space.
        let mut parts = Parts {
            fill: self.overprint_blend(state, false),
            stroke: self.overprint_blend(state, true),
            first_bullet: FirstBullet::No,
        };
        if matches!(state.text.render_mode, 2 | 6) && !self.is_hidden() {
            parts.first_bullet = Interpreter::combined_overprint(state, [parts.fill, parts.stroke]);
        }
        let painting = GlyphPainting::read(state.text.render_mode, self.is_hidden(), state, parts);
        // Once per show-text operator, for the parts its glyphs paint (ADR 1311).
        if painting.fills {
            self.note_colourants_without_a_plane(&state.fill_space);
        }
        if painting.strokes {
            self.note_colourants_without_a_plane(&state.stroke_space);
        }
        // Inside §11.7.4.4's first-bullet group "the fill and stroke shall be performed with an
        // alpha value of 1.0", so the two constants are lifted off the parts and on to the group
        // `show_program_glyph` wraps them in. Once per show-text operator rather than once per
        // glyph, because it is a function of the graphics state and no glyph of one operator
        // changes that; only the *paint* is built from it, since §11.7.5.2's opacity conditions
        // are about the object as it composites and `state` is what says that.
        let lifted =
            (parts.first_bullet == FirstBullet::Group).then(|| state.with_opaque_constants());
        let paints = lifted.as_ref().unwrap_or(state);
        let GlyphPainting {
            fills,
            strokes,
            clipping,
            ..
        } = painting;
        let size = state.text.size;
        let scale = state.text.horizontal_scale;

        let space = Self::space(&font, size);
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
            self.separate_text(&font, text.matrix, size, scale, space, codes.len());
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
                                (state, paints, glyph_fill_clip),
                                text,
                                painting,
                            );
                            coverage.drawn = coverage.drawn.saturating_add(1);
                            // A mark was made, and the question this asks is whether it is the
                            // *shape* the producer chose. §9.7.5.1's NOTE: "the horizontal and
                            // vertical variants of a CMap specify different CIDs for a given
                            // character code", so a vertical CID names a form — and a
                            // substituted face, reached by character alone (§9.7.4.2), draws
                            // whatever it has unless it states that form. Counted where it does
                            // not, and counted here rather than beside the two arms below
                            // because this is the arm where something was drawn: the other two
                            // are a mark missed and a mark the font meant not to make, and this
                            // is a mark made in the wrong shape. A report would cost the oracle
                            // a judged page for a statement about a face (ADRs 0152, 0763), so
                            // this is a number and `Shortfall` is where it goes (ADR 0764).
                            if let Some(character) = program.unsupplied_vertical_form(code) {
                                self.codes_without_a_vertical_form =
                                    self.codes_without_a_vertical_form.saturating_add(1);
                                // The same idiom as the two traces below, and it prints the
                                // character because that is what says which form was wanted.
                                if std::env::var_os("PDFVIEWER_TRACE_VERTICAL_FORM").is_some() {
                                    eprintln!(
                                        "UPRIGHT font=/{} code={} char={character:?}",
                                        state.text.font_name,
                                        code.value()
                                    );
                                }
                            }
                        } else if program.uncovered_character(code).is_some() {
                            // §9.10.2 gave this code a character and the substitute face has
                            // no glyph for it, so a mark the document states is not made.
                            // Tallied rather than reported here: see `glyph_coverage`.
                            coverage.empty = coverage.empty.saturating_add(1);
                            coverage.uncovered = coverage.uncovered.saturating_add(1);
                        } else if read.is_some_and(|read| read != Readback::Whitespace) {
                            // The program made no mark, and the code did not read back as a
                            // **space** — which is the one readback that answers this by
                            // itself, because a space is *meant* to have no outline. Measured
                            // rather than assumed: counting one took the corpus's incomplete
                            // documents from 79 to 109, and twenty-two of the thirty new
                            // reports named a single code (trap 11 — print what a condition
                            // matched before trusting it).
                            //
                            // Everything else divides on the *glyph* rather than on the
                            // reading (ADR 0520). §9.6.5.4 and §9.7.4.2 state the routes
                            // from a code to a glyph, and this asks which of two things
                            // happened at the end of one. A code that reached a glyph the
                            // program contains has been answered: what that glyph draws is the
                            // program's own statement, and a glyph with no contours states a
                            // mark of nothing — which is how every sfnt in existence stores a
                            // space. A code that reached no glyph, or reached `.notdef`, was
                            // not answered: §9.6.5.2 makes `.notdef` what is shown when "an
                            // encoding maps to a character name that does not exist in the
                            // Type 1 font program", and §9.7.6.3 makes CID 0 what is
                            // substituted when "no glyph exists for that CID", so glyph 0 is
                            // the program saying it has none.
                            let blank = program
                                .glyph_index(code)
                                .is_some_and(|glyph| glyph != pdf_font::NOTDEF_GLYPH);
                            // What the report at the end of the page says about *why* nothing
                            // was drawn, asked here where the code and the program are both at
                            // hand, and each asked once per font (ADR 1411).
                            if coverage.cycle.is_none() {
                                coverage.cycle = program.composite_cycle(code);
                            }
                            coverage.both_flags |= program.states_both_symbolic_flags();
                            if coverage.outline_free.is_none() {
                                coverage.outline_free = Some(program.holds_no_outline());
                            }
                            // Asked before the two arms below, because a glyph the program's
                            // repair could not supply has an index and no contours *drawn* —
                            // which the first arm would read as the program describing it
                            // empty. It is the one empty outline here with a known reason, and
                            // the page says it at the end (`Interpreter::glyph_coverage`);
                            // a well-formed program never reaches this.
                            if program.glyph_lost_to_repair(code) {
                                coverage.empty = coverage.empty.saturating_add(1);
                                coverage.lost = coverage.lost.saturating_add(1);
                                if coverage.shortfall.is_none() {
                                    coverage.shortfall = program.repair_shortfall();
                                }
                            } else if blank {
                                // A glyph the program contains and describes as empty is the
                                // program answering, so nothing was missed here. What a
                                // *reader* lost is the readback's question, and §9.10.2's own
                                // answer for a code it cannot name is "there is no way to
                                // determine what the character code represents" — not evidence
                                // in either direction. So this half stays gated on a named
                                // character; the rest is in `codes_without_a_character`.
                                if read.is_some_and(Readback::names_a_mark) {
                                    coverage.empty = coverage.empty.saturating_add(1);
                                    self.codes_reaching_a_blank_glyph =
                                        self.codes_reaching_a_blank_glyph.saturating_add(1);
                                }
                            } else {
                                // The route ended without an answer, which is a fact about the
                                // *program* and needs no character to establish. Gating it on
                                // the readback made `issue17333.pdf` — one `Tj`, one code, an
                                // embedded subset, and §9.6.5.4's algorithm terminating with
                                // nothing — draw an entirely blank page while every counter
                                // that measures the picture read zero and the report below
                                // could not fire, because its `coverage.empty` was never
                                // raised. One of these is still not news on its own — a
                                // producer's deliberate `.notdef` is one — but a font every
                                // one of whose codes comes back empty has drawn nothing the
                                // document asked for, which is exactly what that report says.
                                coverage.empty = coverage.empty.saturating_add(1);
                                self.codes_without_a_glyph =
                                    self.codes_without_a_glyph.saturating_add(1);
                            }
                            // `PDFVIEWER_TRACE_MISSING_GLYPH=1` names each one on stderr, the
                            // same idiom `tests/corpus.rs` uses for a document that never
                            // returns. It fires for the whole arm rather than for what was
                            // counted, because the codes the two branches divide are what a
                            // person reading it is trying to tell apart. The readback is there
                            // because the count alone cannot tell a mark that is missing from
                            // a *space* whose font reads it back as something else, and the
                            // glyph index because that is what the two arms above are decided
                            // by.
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
                            // which [`Readback`] tells apart (ADR 0311).
                            //
                            // A code that reads back as a **space** is *meant* to have no
                            // outline, and the arm above says what that cost when it was
                            // measured.
                            //
                            // `None` — a code inside a Type 3 glyph description — is here for a
                            // second reason: what such a code is, and whether it drew, are
                            // §9.6.4's questions about the glyph rather than this page's. The
                            // test used to be `self.text[start..]` all whitespace, which an
                            // empty slice satisfies vacuously, so a code with no readback at
                            // all sat here too *and was blind inside §14.8.2.5.3's reversal*,
                            // where a code's readback never lands in that slice at all.
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
                            self.draw_type3_glyph(type3, code.value(), state, transform);
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

    /// Moves the pen across a string whose font has no program, drawing and reading nothing.
    ///
    /// ISO 32000-2 §9.4.4 makes the update owed by every code a show string states, whatever
    /// became of the glyph — "\[a]fter the glyph is painted, the text matrix shall be updated
    /// according to the glyph displacement and any spacing parameters that apply" — and §9.2.4
    /// puts the displacement in the *font dictionary* as well as in the program, so a font
    /// `pdf-font` refused still states it. What the refusal costs is the mark; what it does not
    /// cost is the position of everything after it, which on a line that continues in a second
    /// font is that font's glyphs. `issue6127.pdf` page 1 is the witness and ADR 1094 the
    /// argument: `/C2_14 1 Tf 5.737 0 Td <0003>Tj /C2_2 1 Tf [<0003>187<000b>…]TJ`, where
    /// `/C2_14` is an `/Identity-H` over a `CIDFont` with no program (§9.7.5.2 forbids the
    /// combination outright) whose descendant's `/W` states 250 for CID 3, and `/C2_2` is a font
    /// this tree loads — so the `TJ` began 3.0158 pt short of where both references put it.
    ///
    /// Three things the ordinary path does are deliberately not done here, and they are one
    /// sentence: this font marked no part of the page. There is no glyph to paint, no
    /// quadrilateral for the text layer to place over blank paper, and no §9.10.2 readback — a
    /// code named here would be selectable text with nothing under it, while the page's own
    /// `Unsupported::Font` already says the program was refused. The string is still counted as
    /// text the page could not draw, exactly as it was before the metrics were kept.
    fn advance_through_a_refused_font(
        &mut self,
        font: &Font,
        bytes: &[u8],
        state: &GraphicsState,
        text: &mut TextObject,
    ) {
        if !self.is_hidden() {
            self.text_operations = self.text_operations.saturating_add(1);
        }
        let vertical = font.is_vertical();
        for code in font.decode(bytes) {
            // §9.3.3's word spacing, on the same test the drawing path uses: the rule is about
            // the code's encoded length rather than its value.
            let word = if code.takes_word_spacing() {
                state.text.word_spacing
            } else {
                0.0
            };
            let displacement = match font {
                // §9.7.4.3's `w1`, whose horizontal component is 0 and whose vertical component
                // `/W2` and `/DW2` state — read from the dictionary like every other metric here.
                Font::Program(program) if vertical => program.vertical_metrics(code).0[1],
                Font::Program(_) | Font::Type3(_) => font.advance(code),
            };
            text.matrix = Self::advance_step(
                displacement,
                state.text.size,
                state.text.char_spacing + word,
                state.text.horizontal_scale,
                vertical,
            )
            .then(text.matrix);
            self.text_cursor = Some((text.matrix.e, text.matrix.f));
        }
    }
}
