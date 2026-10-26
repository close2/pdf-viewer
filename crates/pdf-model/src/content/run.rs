//! The operator loop: one dispatch over ISO 32000-2 Annex A's operator set, to one method per
//! family of Table 50.
//!
//! [`Interpreter::run`] is the machine's core — a content stream is a bytecode, and this is
//! its interpreter — together with the helpers that turn the content lexer's tokens into
//! operands. Operator families with behaviour of their own (text, colour, transparency,
//! patterns…) live in the sibling modules; what stays here is the table itself.
//!
//! The loop's own concerns are continued in `run/` as further `impl Interpreter` blocks, one
//! module per concern (ADR 2406):
//!
//! | module | holds |
//! |---|---|
//! | `steps` | one token read into the step the loop acts on |
//! | `operands` | operand counts, the pending operands an operator is given, the typed readers |
//! | `machine` | what a run holds between operators, and what it leaves at the stream's end |
//! | `graphics` | graphics-state, path and colour operators |
//! | `text_ops` | the text object, text state, positioning and showing operators |
//! | `xobject_ops` | `Do`, `sh` and the inline image operators |
//! | `marked_ops` | marked-content operators and the sections they open |
//! | `compat_type3_ops` | `BX`/`EX` and a Type 3 glyph's `d0`/`d1` |
//! | `nested` | the content of forms, patterns and glyphs, and §8.9.7's inline image |
//! | `notes` | the report entries the loop raises |

mod compat_type3_ops;
mod graphics;
mod machine;
mod marked_ops;
mod nested;
mod notes;
mod operands;
mod steps;
mod text_ops;
mod xobject_ops;

pub(super) use machine::{Composed, Leftover};
pub(super) use operands::{name_at, narrow, number_at};

use std::sync::Arc;

use machine::Machine;
use operands::operands_before;
use steps::{Keyword, Step, next_step};

use pdf_render::FillRule;
use pdf_syntax::{Dictionary, Object};

use super::reader::{ContentReader, NestedContent, Operator, Strings, inline_dictionary};
use super::report::{KeywordNote, Unsupported};

use super::{GraphicsState, Interpreter, MAX_FORM_DEPTH, MAX_OPERANDS};

impl Interpreter<'_> {
    /// Executes a content stream with the given initial state.
    ///
    /// The operator dispatch is deliberately one flat `match` rather than several
    /// functions. A content stream is a bytecode, and this is its interpreter loop: the
    /// operators are a single table in the specification, and splitting the table across
    /// functions would mean a reader checking "what does `B*` do" has to find which piece
    /// owns it. The state it threads — current path, current point, pending clip, the `q`
    /// stack — is genuinely shared by most arms, so extracting them would replace local
    /// variables with a struct that exists only to be passed back and forth.
    pub(super) fn run(
        &mut self,
        content: &NestedContent,
        resources: &Dictionary,
        initial: &GraphicsState,
    ) {
        // The one place the bound is asked, so that no kind of nested stream can be run without
        // it — see [`MAX_FORM_DEPTH`] for the kind that was.
        if self.nesting >= MAX_FORM_DEPTH {
            // Which of the two it is, said rather than refused: a stream already on the chain
            // is a chain that re-enters itself, and ADR 0793 is why that is not refused sooner —
            // except a glyph description, which `draw_type3_glyph` refuses at its re-entry (ADR
            // 1792).
            let reentered = content
                .identity()
                .is_some_and(|identity| self.chain.contains(&Some(identity)));
            self.note(if reentered {
                Unsupported::NestingCycle {
                    stream: content.detail().to_owned(),
                }
            } else {
                Unsupported::LimitReached {
                    limit: "MAX_FORM_DEPTH",
                }
            });
            return;
        }
        self.nesting = self.nesting.saturating_add(1);
        self.chain.push(content.identity());
        let mut reader = content.reader();
        // What the run has to have produced *nothing* against, for the one fact below to travel.
        let operations = self.operations;
        // ISO 32000-2 §11.6.7 gives a shading pattern's definition "the graphics state that was in
        // effect at the beginning of the content stream in which the shading pattern is set to be
        // the current colour", so a *nested* stream is a new beginning and the parameters it starts
        // with are the ones any pattern it selects inherits. Saved and restored around the run
        // rather than assigned, because a content stream is a scope: what a pattern named after the
        // `Do` inherits is what this stream's caller inherited. The companion of
        // `Interpreter::base`, which the same four sites swap for the same sentence. See
        // [`super::pattern::PatternInitial`].
        let outer = std::mem::replace(
            &mut self.pattern_initial,
            super::pattern::PatternInitial::of(initial),
        );
        self.run_reader(&mut reader, resources, initial);
        self.pattern_initial = outer;
        self.nesting = self.nesting.saturating_sub(1);
        self.chain.pop();
        let mut reached = None;
        for issue in reader.take_issues() {
            if let crate::page::ContentIssue::TooLarge { limit, .. } = issue {
                reached = Some(limit);
            }
            self.note_nested(issue, content.detail());
        }
        // **The one hop `doc/todo/41` was left owing** (ADR 0646). A window makes no
        // `FilterRefusal::TooLarge` of its own — it has nothing allocated to bound — so a bomb
        // the memo would keep beside its encoded bytes was re-inflated to `max_stream_len` on
        // every one of §7.8.2's re-reads, while the buffered route had remembered the same
        // refusal since ADR 0437. This is where both halves of the fact are visible at once:
        // the reader knows it reached the bound, and the interpreter knows the run added not
        // one operation. **Both halves are required.** A window delivers everything up to the
        // bound before it says it stopped, so a run that had drawn something would owe those
        // marks to its second read as well, and a memory that silenced them would make the page
        // a function of the cache — which is the failure `Outcome::Decoded`'s `damage` field
        // already exists to prevent for the other half of this memo.
        if let (Some(limit), Some(decoding)) = (reached, content.decoding())
            && self.operations == operations
        {
            self.document.window_found_nothing(decoding, limit);
        }
    }

    /// The same, over a stream that need not be resident all at once.
    ///
    /// A page's `/Contents` is read this way — see [`ContentReader`] for why that one and not
    /// the nested content streams above.
    pub(super) fn run_reader(
        &mut self,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
        initial: &GraphicsState,
    ) {
        if let Some(left) = self.run_from(reader, resources, Composed::fresh(initial)) {
            self.end_of_stream(left, initial);
        }
    }

    /// Runs a stream's operators from `from`, and hands back what the run left open where the
    /// stream ran out — `None` where the list's budget stopped it.
    ///
    /// **The stream's end is not run here**, which is what lets a piece of a page's stream end
    /// where the next piece begins (ADR 1880): [`Self::run_reader`] runs [`Self::end_of_stream`]
    /// after it, and a divided read runs it once, after the last piece.
    pub(super) fn run_from(
        &mut self,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
        from: Composed,
    ) -> Option<Leftover> {
        // What the stream has stated since the last operator. §7.8.2 makes an operator's own
        // operands the ones that *immediately precede* it, which is a distinction only a
        // malformed stream can show: `operands_before` is what turns this into that slice.
        let mut pending: Vec<Object> = Vec::new();
        let mut machine = Machine::begin(from);
        // How many `[` are open. An array is *one* operand, so nothing inside it is an
        // operator; see the keyword arm below.
        let mut array_depth = 0usize;
        let empty_string: Arc<[u8]> = Arc::from(&[][..]);

        loop {
            // **The token is read inside the closure and nothing borrowed leaves it**, which
            // is what lets the stream be read through a moving window: see
            // [`ContentReader::with_token`]. An *operand* is dealt with in there and never
            // named again — the closure holds `pending` and the array depth, so the object
            // goes straight into the list the operator will read it from, which is the same
            // work this loop did when it held the lexer itself. Only what the loop has to act
            // on afterwards comes out, and [`Step`] owns all of it.
            //
            // §9.3.1's Table 103: with no font in force a show draws nothing and reads none of its
            // string, so no string operand is read until a `Tf` (or a `gs` naming a font) sets
            // one. Exact rather than a guess, because §7.8.2 makes an operator's operands the
            // ones since the last operator and every operator clears them below: a string read
            // now can only reach an operator dispatched under the state in force now (ADR 1521).
            let strings = if machine.state.text.font.is_none() {
                Strings::StepOver
            } else {
                Strings::Read
            };
            let step = next_step(
                reader,
                strings,
                &mut pending,
                &mut array_depth,
                &empty_string,
            );
            if matches!(step, Step::End) {
                break;
            }
            let Some(keyword) = self.settle_step(step, reader, &mut pending, &mut array_depth)
            else {
                continue;
            };

            // **Counted here rather than at the top of the loop, and that is the whole of ADR
            // 0306**: this is the only place the interpreter knows it has an operator, because
            // §7.8.2's grammar puts the operator after its operands — a `c` is seven tokens and
            // one operator. Everything above this line is an operand, an array bracket or a
            // keyword inside an array.
            //
            // **The count is a figure and refuses nothing** (ADR 1809): it is how far the run has
            // got, for the window's progress and for a census, and a page that states more
            // operators than any figure is drawn whole. What ends the token loop is the stream's
            // own length — every token consumes at least one byte, and Table 31's
            // `Limits::max_stream_len` bounds the concatenation — and what ends a run that
            // *re-reads* streams, a form or a glyph or a cell invoked again and again, is the
            // memory its marks take (`MAX_LIST_BYTES`) or the person's stop, which in the
            // confined worker is a kill (ADR 0241).
            self.operations = self.operations.saturating_add(1);
            // The list's budget is asked every few dozen operators, in this one branch (ADR
            // 1507), and a charge past it stops the run: no operator after it is dispatched, here
            // or in any stream enclosing this one.
            if self.operations >= self.list_budget.next_check() {
                if self.list_spent(machine.state.stroke.dash_array.len()) {
                    return None;
                }
                // The window's figure rides the same check, so it costs the loop nothing between
                // checks and a thread nobody is watching one thread-local look (ADR 1811).
                self.publish_progress(reader, !machine.in_text);
            }
            if let Some(ledger) = self.ledger {
                ledger.borrow_mut().operator(keyword.bytes());
            }
            // A keyword Table 50 does not list is an error §7.8.2 states — "an error shall occur" —
            // outside a compatibility section, and ignored with its operands inside one.
            let operator = match keyword {
                Keyword::Operator(operator) => operator,
                Keyword::Unknown(word) => {
                    if machine.compatibility == 0 {
                        self.note_unknown_operator(word.as_slice());
                    }
                    pending.clear();
                    continue;
                }
            };

            // §8.6.8: inside a `d1` glyph description or an uncoloured tiling pattern —
            // and inside everything either of them invokes — "all of the following operators
            // shall be ignored", the list being `Operator::is_colour`. Dropping them here
            // rather than in each arm keeps the rule where the clause puts it, in one place
            // for both circumstances, and it is what lets the colour the figure is *used*
            // with reach the marks inside it.
            if self.uncoloured && operator.is_colour() {
                pending.clear();
                continue;
            }

            // §7.8.2: "In PDF, all of the operands needed by an operator shall immediately
            // precede that operator. Operators do not return results, and operands shall not
            // be left over when an operator finishes execution." A conforming stream leaves
            // nothing over, so on one of those this slice is everything `pending` holds and
            // the sentence costs nothing. What it decides is the malformed stream, where the
            // operands an operator is given are the *last* of them rather than the first —
            // `T02-05-01_008_Font-set-operator-missing.pdf` writes `/F0 36. (Hello
            // PDF-world!) Tj`, whose `Tj` was reading the name and drawing nothing.
            let operands: &[Object] = operands_before(&pending, operator.operand_count());
            // The same sentence's other half — "all of the operands needed" — read as a count:
            // an operator short of them is dispatched with what is there, and the page says so.
            if let Some(takes) = operator.operand_count()
                && operands.len() < takes
            {
                let given = operands.len();
                self.note_keyword(
                    KeywordNote::Shortfall { given },
                    operator.name(),
                    |keyword| Unsupported::OperandShortfall {
                        operator: String::from_utf8_lossy(keyword).into_owned(),
                        given,
                        takes,
                    },
                );
            }

            // One method per family of Table 50: the loop knows which family an operator is in
            // and nothing of what the family does with it.
            self.run_operator(operator, operands, reader, resources, &mut machine);

            pending.clear();
        }

        Some(machine.into_leftover())
    }

    /// Settles a token that is not an operand or the stream's end: what a keyword inside an array,
    /// an inline dictionary or an operand past the bound each come to, and the keyword where there
    /// is one for the loop to dispatch. `None` is "read the next token".
    fn settle_step(
        &mut self,
        step: Step,
        reader: &mut ContentReader<'_>,
        pending: &mut Vec<Object>,
        array_depth: &mut usize,
    ) -> Option<Keyword> {
        // Operands accumulate until an operator consumes them.
        match step {
            // The loop has already stopped at the stream's end, and an operand is in `pending`.
            Step::End | Step::Operand => None,
            Step::InsideAnArray(word) => {
                self.note_keyword(KeywordNote::InsideAnArray, word.as_slice(), |keyword| {
                    Unsupported::Operator {
                        operator: format!(
                            "{} inside an array, which §7.3.6 admits only objects into",
                            String::from_utf8_lossy(keyword)
                        ),
                    }
                });
                None
            }
            Step::TooManyOperands => {
                // Dropping operands silently truncates the page: a `TJ` array is
                // one operand per run *and* per kerning adjustment, so a single
                // justified line can be hundreds, and the text simply stopped
                // mid-sentence with nothing reported. The bound stays, because a
                // hostile stream can otherwise make one operator allocate without
                // limit — but reaching it is now a reported defect.
                self.note(Unsupported::LimitReached {
                    limit: "MAX_OPERANDS",
                });
                None
            }
            Step::Dictionary => {
                let dictionary = Object::Dictionary(inline_dictionary(reader));
                if pending.len() < MAX_OPERANDS {
                    pending.push(dictionary);
                } else {
                    self.note(Unsupported::LimitReached {
                        limit: "MAX_OPERANDS",
                    });
                    *array_depth = 0;
                }
                None
            }
            Step::Operator(operator) => Some(Keyword::Operator(operator)),
            Step::Unknown(word) => Some(Keyword::Unknown(word)),
        }
    }

    /// Sends one operator to the method for its family in Table 50 (ISO 32000-2 §8.2), each of
    /// which owns what its operators do.
    fn run_operator(
        &mut self,
        operator: Operator,
        operands: &[Object],
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
        machine: &mut Machine,
    ) {
        // One method per family of Table 50: the loop knows which family an operator is in
        // and nothing of what the family does with it.
        match operator {
            Operator::Save
            | Operator::Restore
            | Operator::Concatenate
            | Operator::ExtGState
            | Operator::LineWidth
            | Operator::LineCap
            | Operator::LineJoin
            | Operator::MiterLimit
            | Operator::Dash
            | Operator::RenderingIntent
            | Operator::Flatness => {
                self.run_graphics_state_op(operator, operands, resources, machine);
            }
            Operator::MoveTo
            | Operator::LineTo
            | Operator::CurveTo
            | Operator::CurveInitialReplicated
            | Operator::CurveFinalReplicated
            | Operator::ClosePath
            | Operator::Rectangle => {
                self.run_path_construction_op(operator, operands, machine);
            }
            Operator::EndPath
            | Operator::Fill
            | Operator::FillObsolete
            | Operator::FillEvenOdd
            | Operator::Stroke
            | Operator::CloseStroke
            | Operator::FillStroke
            | Operator::FillStrokeEvenOdd
            | Operator::CloseFillStroke
            | Operator::CloseFillStrokeEvenOdd
            | Operator::Clip
            | Operator::ClipEvenOdd => self.run_path_painting_op(operator, machine),
            Operator::FillGray
            | Operator::StrokeGray
            | Operator::FillRgb
            | Operator::StrokeRgb
            | Operator::FillCmyk
            | Operator::StrokeCmyk
            | Operator::FillColourSpace
            | Operator::StrokeColourSpace
            | Operator::FillColour
            | Operator::FillColourN
            | Operator::StrokeColour
            | Operator::StrokeColourN => {
                self.run_colour_op(operator, operands, resources, machine);
            }
            Operator::BeginText
            | Operator::EndText
            | Operator::Font
            | Operator::CharSpacing
            | Operator::WordSpacing
            | Operator::HorizontalScaling
            | Operator::Leading
            | Operator::Rise
            | Operator::RenderMode
            | Operator::MoveText
            | Operator::MoveTextSetLeading
            | Operator::TextMatrix
            | Operator::NextLine => {
                self.run_text_op(operator, operands, resources, machine);
            }
            Operator::ShowText
            | Operator::ShowTextAdjusted
            | Operator::NextLineShowText
            | Operator::NextLineShowTextSpaced => {
                self.run_text_showing_op(operator, operands, machine);
            }
            Operator::GlyphWidth | Operator::GlyphWidthAndBox => self.run_type3_op(operator),
            Operator::XObject => self.run_xobject_op(operands, resources, machine),
            Operator::Shade => self.run_shading_op(operands, resources, machine),
            Operator::BeginImage | Operator::ImageData | Operator::EndImage => {
                self.run_inline_image_op(operator, reader, resources, machine);
            }
            Operator::MarkPoint
            | Operator::MarkPointProperties
            | Operator::BeginMarked
            | Operator::BeginMarkedProperties
            | Operator::EndMarked => {
                self.run_marked_content_op(operator, operands, resources, machine);
            }
            Operator::BeginCompatibility | Operator::EndCompatibility => {
                Self::run_compatibility_op(operator, machine);
            }
        }
    }

    /// What a stream's end says about what it left open, and the `/AIS` reading it began under
    /// in force again.
    pub(super) fn end_of_stream(&mut self, left: Leftover, initial: &GraphicsState) {
        self.report_leftover(left);
        // §8.10.1's `Do` ends with "[r]estores the saved graphics state, as if by invoking the Q
        // operator", and a Type 3 glyph's description and an appearance stream are run the same
        // way: whatever reading of §11.6.4.3's `/AIS` this stream left in force, the one it began
        // under is in force again for whatever follows it. Stated whether or not it changed,
        // because the statement is also what joins this stream's last elements to the record
        // before a caller takes them off the list (ADR 1319).
        self.note_alpha_source(initial.alpha_is_shape);
    }

    /// What a stream's end says about what it left open: a text object, a pending clip, a
    /// marked-content sequence.
    pub(super) fn report_leftover(&mut self, left: Leftover) {
        let Leftover {
            in_text,
            mut text_object,
            pending_clip,
            marked,
            ..
        } = left;
        // An unclosed `BT` is malformed but harmless here; noted so it is not invisible.
        // Any glyph outlines a clipping render mode accumulated are discarded with it —
        // §9.3.6 makes `ET` the moment they become a clip, and a text object that never
        // ended never reached it.
        if in_text {
            self.note(Unsupported::Operator {
                operator: "BT without ET".to_owned(),
            });
        }
        if let Some(outer) = text_object.enclosing_reading.take() {
            let (reading, in_force) = self.close_reading_scope(outer);
            self.absorb_reading(reading, text_object.start);
            self.note_alpha_source(in_force);
        }
        // A `W` the stream ended on modified nothing: §8.5.4 has it "modify the effect of the
        // succeeding painting operator", and none succeeded it in this stream. It does not
        // reach the stream that invoked this one either — §8.10.1 makes a form "a self-contained
        // description", and the only content a path object may straddle is a page's own
        // `/Contents` array (§7.8.2), which arrives here as one stream. `issue6413.pdf`'s form
        // ends `Q W`, and `mupdf` carries that `W` out into the page, where the next fill becomes
        // the clip and the rest of the page vanishes under it.
        if let Some(rule) = pending_clip {
            self.note(Unsupported::Operator {
                operator: format!(
                    "{} without a path-painting operator after it",
                    clip_operator(rule)
                ),
            });
        }

        // A marked-content section left open by a malformed stream must not leave this
        // stream's hidden layers hiding the next one. The annotation pass runs after the
        // page's content, and a leaked counter would silently blank every annotation.
        let unclosed = marked.iter().filter(|section| section.hides).count();
        if unclosed > 0 {
            self.hidden = self.hidden.saturating_sub(unclosed);
            self.note(Unsupported::Operator {
                operator: "BDC without EMC".to_owned(),
            });
        }
        // And it must not leave the *extent* of an unclosed sequence collecting the marks of
        // whatever runs next. A form `XObject` may hold sequences of its own (§14.7.5.2), so
        // this stream's leaked entries are dropped rather than the whole stack: what the
        // sequence enclosed is unknowable, and the span it would have produced is never
        // recorded because no `EMC` reached it. Reported through the same operator note above,
        // which is the same defect said once.
        for section in &marked {
            self.close_marking(section.mcid);
        }
    }
}

/// Which of Table 60's two operators set a pending clip, from the rule it set.
const fn clip_operator(rule: FillRule) -> &'static str {
    match rule {
        FillRule::NonZero => "W",
        FillRule::EvenOdd => "W*",
    }
}
