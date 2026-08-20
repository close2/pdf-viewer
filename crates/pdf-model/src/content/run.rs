//! The operator loop: one flat dispatch over ISO 32000-2 Annex A's operator set.
//!
//! [`Interpreter::run`] is the machine's core — a content stream is a bytecode, and this is
//! its interpreter — together with the helpers that turn the content lexer's tokens into
//! operands. Operator families with behaviour of their own (text, colour, transparency,
//! patterns…) live in the sibling modules; what stays here is the table itself.

use std::sync::Arc;

use pdf_render::{FillRule, Path, PathCommand, Point, Transform};
use pdf_syntax::{Dictionary, Name, Object};

use super::colour::{Intent, assign_colour};
use super::font::Font;
use super::marked::Marked;
use super::path::{begin_subpath, close_subpath};
use super::reader::{ContentReader, NestedContent, Word};
use super::report::{ArtifactSpan, DamagedStream, MarkedSpan, Unsupported};
use super::text::TextObject;
use super::{
    GraphicsState, Interpreter, MAX_OPERANDS, MAX_OPERATIONS, MAX_STATE_DEPTH, line_cap, line_join,
    miter_limit, set_dash,
};

impl Interpreter<'_> {
    pub(super) fn note(&mut self, item: Unsupported) {
        self.unsupported.insert(item.clone(), item);
    }

    /// Decodes one of §7.8.2's self-contained content streams, saying so where it is damaged.
    ///
    /// > Content streams shall also be used to package sequences of instructions as
    /// > self-contained graphical elements, such as forms (see 8.10, "Form XObjects"), patterns
    /// > (8.7, "Patterns"), certain fonts (9.6.4, "Type 3 fonts"), and annotation appearances
    /// > (12.5.5, "Appearance streams").
    ///
    /// Every one of those is "a sequence of instructions" by the same clause's first sentence,
    /// so §7.4.1's two halves are owed here exactly as they are for a page's `/Contents`: the
    /// prefix a damaged filter produced goes on the page, and that it is only a prefix goes in
    /// the report. `what` says which kind and which resource, because each of the five costs a
    /// different mark and a report that did not distinguish them could not be acted on.
    ///
    /// `None` where nothing decoded at all. The caller words that one itself: there the whole
    /// element is missing rather than the end of it, and the five callers already had five
    /// different sentences for it.
    ///
    /// **What comes back is a source rather than a buffer** (ADR 0427): each of the four is
    /// read more than once, so a reader is made per run, and whether the bytes are held whole
    /// or inflated through a window each time is
    /// [`pdf_syntax::Document::nested_content_source`]'s decision. Where they are held whole
    /// the damage is known now and is reported here; where they are windowed it is met during
    /// the run and [`Interpreter::run`] reports it then, in the same words.
    pub(super) fn content_stream(
        &mut self,
        stream: &pdf_syntax::Stream,
        what: &str,
    ) -> Option<NestedContent> {
        let content = NestedContent::of(self.document, stream, what.to_owned()).ok()?;
        if let Some((damage, kept)) = content.stated_damage() {
            self.note(Unsupported::DamagedContentStream {
                stream: DamagedStream {
                    detail: what.to_owned(),
                    damage,
                    kept,
                },
            });
        }
        Some(content)
    }

    /// Reads the inline image whose `BI` the reader has just consumed, ISO 32000-2 §8.9.7.
    ///
    /// An inline image is the one construction in a content stream that is not a token: `ID`
    /// is followed by data whose length the dictionary need not state, so where it ends is
    /// found by reading it. `examples/token_window_census` measured what that costs a bounded
    /// reader — of **93 930** inline images in 39 976 documents, **90 304 state or imply their
    /// length before their data is read** (336 by §8.9.7's `/L`, 89 968 by §8.9.3's sample
    /// arithmetic), and of the **3 455** that need the forward `EI` search the largest is
    /// **2.99 KiB**.
    ///
    /// So the lookahead starts at the window's own size and doubles only while the answer may
    /// have been cut by it, up to [`crate::content::reader::LOOKAHEAD`]. Past that the image
    /// is refused *by name* rather than read short: the data goes the resource route, whole,
    /// as an image always has, and a bounded reader that stopped looking has said something
    /// different from a file that states no `EI`.
    fn inline_image(
        &mut self,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
    ) -> crate::inline_image::Scan {
        let bound = crate::content::reader::LOOKAHEAD.min(self.document.limits().max_stream_len);
        let mut want = crate::content::reader::WINDOW;
        loop {
            let (ahead, complete) = reader.lookahead(want);
            let scanned = crate::inline_image::scan(self.document, ahead, 0, resources);
            if complete || scanned.image.is_ok() {
                return scanned;
            }
            if want >= bound {
                return crate::inline_image::Scan {
                    resume: scanned.resume,
                    image: Err(crate::inline_image::InlineImageError::Unbuffered { bound }),
                };
            }
            want = want.saturating_mul(2).min(bound);
        }
    }

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
        form_depth: usize,
    ) {
        let mut reader = content.reader();
        self.run_reader(&mut reader, resources, initial, form_depth);
        for issue in reader.take_issues() {
            self.note_nested(issue, content.detail());
        }
    }

    /// Says what reading one of §7.8.2's other four content streams through a window found.
    ///
    /// [`ContentIssue`] is Table 31's noun and every one of its indexed variants is about a
    /// part of a *page's* `/Contents`, so what the window raises here is translated rather
    /// than passed on: a form reached through `/XObject` has no part index, and putting a
    /// zero there would say something about `/Contents` that is not true. ADR 0359 made the
    /// same distinction for the damage report and this is the rest of it.
    fn note_nested(&mut self, issue: crate::page::ContentIssue, what: &str) {
        match issue {
            // ADR 0343's sentence, in the same words the whole-decode route uses — see
            // [`Interpreter::content_stream`].
            crate::page::ContentIssue::Damaged { damage, kept, .. } => {
                self.note(Unsupported::DamagedContentStream {
                    stream: DamagedStream {
                        detail: what.to_owned(),
                        damage,
                        kept,
                    },
                });
            }
            // Neither of these is indexed by a part, so both carry across as themselves: a
            // token no buffer of `CEILING` bytes can hold, and a stream that reached
            // `max_stream_len`. The second is a bound of ours and the first is this reader's
            // buffer, and both are refusals rather than damage — ADR 0365's distinction.
            issue @ crate::page::ContentIssue::TokenTooLong { .. } => {
                self.note(Unsupported::Content { issue });
            }
            crate::page::ContentIssue::TooLarge { limit, .. } => {
                self.note(Unsupported::Content {
                    issue: crate::page::ContentIssue::TooLarge { part: None, limit },
                });
            }
            // A stream the window could not decode at all reaches the caller's own sentence
            // instead, because `nested_content_source` only windows a stream whose decode has
            // already produced bytes. It is written rather than left out because a silence
            // here would be exactly the failure trap 5 is about.
            crate::page::ContentIssue::Undecodable { .. }
            | crate::page::ContentIssue::NotAStream { .. }
            | crate::page::ContentIssue::Unreachable { .. } => {
                self.note(Unsupported::Operator {
                    operator: format!("undecodable {what}"),
                });
            }
        }
    }

    /// The same, over a stream that need not be resident all at once.
    ///
    /// A page's `/Contents` is read this way — see [`ContentReader`] for why that one and not
    /// the nested content streams above.
    #[expect(
        clippy::too_many_lines,
        reason = "a bytecode dispatch table reads better whole than split; see above"
    )]
    pub(super) fn run_reader(
        &mut self,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
        initial: &GraphicsState,
        form_depth: usize,
    ) {
        // What the stream has stated since the last operator. §7.8.2 makes an operator's own
        // operands the ones that *immediately precede* it, which is a distinction only a
        // malformed stream can show: `operands_before` is what turns this into that slice.
        let mut pending: Vec<Object> = Vec::new();
        let mut state = initial.clone();
        // §8.4.2's stack, carrying §9.4.2's two text matrices beside the graphics state.
        //
        // ISO 32000-2 §9.4.2, as Errata Collection 3 adds it (issue #368, `/State` `Review`
        // `Completed`): within a text object the graphics state stack operators q and Q "shall
        // additionally push and pop Tm and Tlm as part of the graphics state stack". Quoted in
        // prose rather than as a blockquote because the sentence is an addition and `doc/md/`
        // is a conversion of the base text — see `text_state.rs`'s test for the whole reading.
        //
        // So the entry is a triple rather than a `GraphicsState`: the two matrices are saved
        // and restored *with* it, which is what "as part of the graphics state stack" says, and
        // holding them in a stack of their own would let the two go out of step on a stream
        // whose `q` is inside a text object and whose `Q` is outside it. ADR 0421.
        let mut stack: Vec<(GraphicsState, Transform, Transform)> = Vec::new();

        // The path being built, and the pending clip requested by `W`/`W*`.
        let mut path = Path::new();
        let mut start = Point::new(0.0, 0.0);
        let mut current = Point::new(0.0, 0.0);
        let mut pending_clip: Option<FillRule> = None;
        let mut in_text = false;
        // The text object's own parameters. `BT` resets them, and `q`/`Q` save and restore
        // them with the graphics state — §9.4.2 as Errata Collection 3 amends it, above.
        let mut text_object = TextObject::default();
        // One entry per open marked-content section, saying whether it hid what follows.
        // Every `BMC` and `BDC` pushes, so an `EMC` closes the section it actually belongs
        // to rather than the last optional one — which is why this is not just a counter.
        //
        // It carries no bound of its own because it already has one: a section costs an
        // operator, and `MAX_OPERATIONS` bounds those at four million. A stream that nests
        // that deep has spent its whole budget doing so.
        let mut marked: Vec<Marked> = Vec::new();
        // §7.8.2's compatibility section, `BX` … `EX`, which is nesting depth rather than a
        // flag because the clause says the pair "may be nested".
        //
        // > Ordinarily, when a PDF reader encounters an operator in a content stream that it
        // > does not recognise, an error shall occur.
        //
        // — and inside the section, "unrecognised operators (along with their operands) shall
        // be ignored without error". So this is the one place in the interpreter where an
        // unsupported *input* is deliberately silent, and it is silent because the file said
        // in advance that ignoring it is the appropriate thing to do.
        let mut compatibility = 0usize;
        // How many `[` are open. An array is *one* operand, so nothing inside it is an
        // operator; see the keyword arm below.
        let mut array_depth = 0usize;
        // Where the last `/ActualText` replacement ended in the readback, for §14.9.4's rule
        // that two consecutive ones have no word break between them.
        let mut replaced_ends_at: Option<usize> = None;

        loop {
            // **The token is read inside the closure and nothing borrowed leaves it**, which
            // is what lets the stream be read through a moving window: see
            // [`ContentReader::with_token`]. An *operand* is dealt with in there and never
            // named again — the closure holds `pending` and the array depth, so the object
            // goes straight into the list the operator will read it from, which is the same
            // work this loop did when it held the lexer itself. Only what the loop has to act
            // on afterwards comes out, and [`Step`] owns all of it.
            let step = reader.with_token(|token| match token {
                None => Step::End,
                Some(pdf_syntax::Token::Keyword(word)) if array_depth == 0 => {
                    Step::Operator(Word::new(word))
                }
                // §7.8.2's grammar puts an operator *after* its operands, and §7.3.6 makes an
                // array "a one-dimensional collection of objects arranged sequentially" — so a
                // keyword between two elements of an array is neither an element (it is not an
                // object) nor an operator (an array is one operand, and an operator cannot be
                // inside one). `operator-in-TJ-array.pdf` writes exactly that:
                // `[(Grandes) 0.0 Tc -250.0 (Clientèles,) 0.0 Tc … ] TJ`, and dispatching those
                // `Tc`s consumed the runs before them — the page drew one word of five.
                //
                // The recovery is to skip the keyword and keep the array, *and say so*: the
                // file is malformed and the standard states no reading for it, so drawing the
                // text without a word would be the silent-fallback failure this project
                // forbids, and refusing the text would lose what the file plainly states.
                Some(pdf_syntax::Token::Keyword(word)) => Step::InsideAnArray(Word::new(word)),
                // An inline dictionary is one operand, assembled by the caller because it
                // needs the reader again. §14.6.2: "[i]f all of the values in a property list
                // dictionary are direct objects, the dictionary may be written inline in the
                // content stream as a direct object" — the form real documents use for
                // §14.9.4's `/ActualText`, and the form that reached the operator dispatch one
                // token at a time until the fifty-fifth session.
                Some(pdf_syntax::Token::DictOpen) => Step::Dictionary,
                Some(other) => {
                    if matches!(other, pdf_syntax::Token::ArrayOpen) {
                        array_depth = array_depth.saturating_add(1);
                    } else if matches!(other, pdf_syntax::Token::ArrayClose) {
                        array_depth = array_depth.saturating_sub(1);
                    }
                    // Arrays are deliberately left flattened: `TJ` and `d` read their elements
                    // as separate operands and have since the beginning.
                    if pending.len() < MAX_OPERANDS {
                        pending.push(token_to_object(other));
                        Step::Operand
                    } else {
                        // An unclosed `[` would otherwise suppress every operator for the
                        // rest of the stream, which on a fuzzed file means a blank page. One
                        // operand cap's worth of tokens is as far as an array is believed.
                        array_depth = 0;
                        Step::TooManyOperands
                    }
                }
            });
            // Operands accumulate until an operator consumes them.
            let word = match step {
                Step::End => break,
                Step::Operand => continue,
                Step::InsideAnArray(word) => {
                    self.note(Unsupported::Operator {
                        operator: format!(
                            "{} inside an array, which §7.3.6 admits only objects into",
                            String::from_utf8_lossy(word.as_slice())
                        ),
                    });
                    continue;
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
                    continue;
                }
                Step::Dictionary => {
                    let dictionary = Object::Dictionary(inline_dictionary(reader, 0));
                    if pending.len() < MAX_OPERANDS {
                        pending.push(dictionary);
                    } else {
                        self.note(Unsupported::LimitReached {
                            limit: "MAX_OPERANDS",
                        });
                        array_depth = 0;
                    }
                    continue;
                }
                Step::Operator(word) => word,
            };

            // **Here rather than at the top of the loop, and that is the whole of ADR 0306.**
            // `MAX_OPERATIONS` names operators and this is the only place the interpreter knows
            // it has one: §7.8.2's grammar puts the operator after its operands, so a `c` is
            // seven tokens and one operator, and counting the loop's turns charged a curve seven
            // times over. Everything above this line is an operand, an array bracket or a
            // keyword inside an array, and none of those is an operator.
            //
            // What bounds the *token* loop is the stream's own length — every token consumes at
            // least one byte — and `Limits::max_stream_len` bounds that.
            //
            // **Under a window that bound stops being spent in advance, which is ADR 0362's
            // consequence and ADR 0365's arithmetic.** A bomb's gibibyte used to be allocated
            // before the first operator was counted, so this bound guarded nothing it was
            // reached by; the same bomb read through a window reaches four million operators a
            // few megabytes in, and stops there.
            let operator = word.as_slice();
            self.operations = self.operations.saturating_add(1);
            if self.operations > MAX_OPERATIONS {
                self.note(Unsupported::LimitReached {
                    limit: "MAX_OPERATIONS",
                });
                return;
            }

            // §8.6.8: inside a `d1` glyph description or an uncoloured tiling pattern —
            // and inside everything either of them invokes — "all of the following operators
            // shall be ignored", the list being `is_colour_operator`. Dropping them here
            // rather than in each arm keeps the rule where the clause puts it, in one place
            // for both circumstances, and it is what lets the colour the figure is *used*
            // with reach the marks inside it.
            if self.uncoloured && is_colour_operator(operator) {
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
            let operands: &[Object] = operands_before(&pending, operator);

            match operator {
                // --- graphics state ---
                b"q" => {
                    if stack.len() < MAX_STATE_DEPTH {
                        stack.push((state.clone(), text_object.matrix, text_object.line));
                    } else {
                        self.note(Unsupported::LimitReached {
                            limit: "MAX_STATE_DEPTH",
                        });
                    }
                }
                b"Q" => {
                    if let Some((previous, matrix, line)) = stack.pop() {
                        state = previous;
                        // The two matrices come back only *inside* a text object, which is
                        // where §9.4.2's addition places them: outside one they are the
                        // parameters of an object that has ended, and Table 105's `BT` sets
                        // both to the identity before the next glyph is shown anyway.
                        if in_text {
                            text_object.matrix = matrix;
                            text_object.line = line;
                        }
                    }
                    // An unmatched `Q` is ignored: the alternative is to abandon the page,
                    // and files with one extra `Q` render correctly everywhere else.
                }
                b"cm" => {
                    if let Some(matrix) = matrix_from(operands) {
                        state.transform = matrix.then(state.transform);
                    }
                }
                b"gs" => self.apply_ext_gstate(operands, resources, &mut state, in_text),

                // --- line parameters ---
                b"w" => {
                    if let Some(width) = number_at(operands, 0) {
                        // ISO 32000-2 §8.4.3.2: the line width "shall be a non-negative
                        // number expressed in user space units". A negative one is outside
                        // the parameter's stated domain and the clause states no recovery,
                        // so clamping it into the domain is a **documented choice** and not a
                        // derivation — see `Stroke::device_width` for what 0 then means, and
                        // `oracle.rs`'s `CONTRADICTED_NEGATIVE_LINE_WIDTH` for the page that
                        // shows the three answers apart.
                        state.stroke.width = width.max(0.0);
                    }
                }
                b"J" => {
                    if let Some(code) = integer_at(operands, 0) {
                        state.stroke.cap = line_cap(code);
                    }
                }
                b"j" => {
                    if let Some(code) = integer_at(operands, 0) {
                        state.stroke.join = line_join(code);
                    }
                }
                b"M" => {
                    if let Some(limit) = number_at(operands, 0) {
                        state.stroke.miter_limit = miter_limit(limit);
                    }
                }
                b"d" => set_dash(operands, &mut state.stroke),
                // Rendering intent and flatness affect nothing this renderer does.
                // --- path construction ---
                b"m" => {
                    if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                        current = Point::new(x, y);
                        start = current;
                        begin_subpath(&mut path, current);
                    }
                }
                b"l" => {
                    if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                        current = Point::new(x, y);
                        path.push(PathCommand::LineTo(current));
                    }
                }
                b"c" => {
                    if let Some(points) = points_from::<3>(operands) {
                        path.push(PathCommand::CurveTo(points[0], points[1], points[2]));
                        current = points[2];
                    }
                }
                b"v" => {
                    // The first control point is the current point.
                    if let Some(points) = points_from::<2>(operands) {
                        path.push(PathCommand::CurveTo(current, points[0], points[1]));
                        current = points[1];
                    }
                }
                b"y" => {
                    // The second control point is the endpoint.
                    if let Some(points) = points_from::<2>(operands) {
                        path.push(PathCommand::CurveTo(points[0], points[1], points[1]));
                        current = points[1];
                    }
                }
                b"h" => {
                    close_subpath(&mut path);
                    current = start;
                }
                b"re" => {
                    if let Some(values) = numbers_from::<4>(operands) {
                        let (x, y, w, h) = (values[0], values[1], values[2], values[3]);
                        // Table 58 states `re` as `x y m` and three `l`s and an `h`, so the
                        // `m` it begins with overrides a preceding one exactly as a written
                        // `m` would: 60 paths on `issue12810.pdf`'s first page pair the two.
                        begin_subpath(&mut path, Point::new(x, y));
                        path.push(PathCommand::LineTo(Point::new(x + w, y)));
                        path.push(PathCommand::LineTo(Point::new(x + w, y + h)));
                        path.push(PathCommand::LineTo(Point::new(x, y + h)));
                        path.push(PathCommand::Close);
                        start = Point::new(x, y);
                        current = start;
                    }
                }

                // --- path painting ---
                b"n" => self.end_path(&mut path, &mut pending_clip, &mut state, None, None),
                b"f" | b"F" => {
                    self.end_path(
                        &mut path,
                        &mut pending_clip,
                        &mut state,
                        Some(FillRule::NonZero),
                        None,
                    );
                }
                b"f*" => {
                    self.end_path(
                        &mut path,
                        &mut pending_clip,
                        &mut state,
                        Some(FillRule::EvenOdd),
                        None,
                    );
                }
                b"S" => self.end_path(&mut path, &mut pending_clip, &mut state, None, Some(false)),
                b"s" => {
                    close_subpath(&mut path);
                    self.end_path(&mut path, &mut pending_clip, &mut state, None, Some(true));
                }
                b"B" => {
                    self.end_path(
                        &mut path,
                        &mut pending_clip,
                        &mut state,
                        Some(FillRule::NonZero),
                        Some(false),
                    );
                }
                b"B*" => {
                    self.end_path(
                        &mut path,
                        &mut pending_clip,
                        &mut state,
                        Some(FillRule::EvenOdd),
                        Some(false),
                    );
                }
                b"b" | b"b*" => {
                    close_subpath(&mut path);
                    let rule = if operator == b"b*" {
                        FillRule::EvenOdd
                    } else {
                        FillRule::NonZero
                    };
                    self.end_path(
                        &mut path,
                        &mut pending_clip,
                        &mut state,
                        Some(rule),
                        Some(true),
                    );
                }
                b"W" => pending_clip = Some(FillRule::NonZero),
                b"W*" => pending_clip = Some(FillRule::EvenOdd),

                // --- colour ---
                // `g`, `rg` and `k` set a device space and a colour together — or the
                // matching `Default` space, where the resources name one, which is why
                // these resolve the space rather than naming it directly.
                b"g" | b"G" => {
                    if let Some(grey) = number_at(operands, 0) {
                        let space = self.device_space("DeviceGray", resources);
                        let colour = self.colour(&space, &[grey], state.black_point());
                        assign_colour(&mut state, operator == b"g", colour, space);
                    }
                }
                b"rg" | b"RG" => {
                    if let Some(values) = numbers_from::<3>(operands) {
                        let space = self.device_space("DeviceRGB", resources);
                        let colour = self.colour(&space, &values, state.black_point());
                        assign_colour(&mut state, operator == b"rg", colour, space);
                    }
                }
                b"k" | b"K" => {
                    if let Some(values) = numbers_from::<4>(operands) {
                        let space = self.device_space("DeviceCMYK", resources);
                        let colour = self.colour(&space, &values, state.black_point());
                        assign_colour(&mut state, operator == b"k", colour, space);
                    }
                }
                b"cs" | b"CS" => {
                    let fill = operator == b"cs";
                    self.set_colour_space(operands, resources, &mut state, fill);
                }
                b"sc" | b"scn" | b"SC" | b"SCN" => {
                    let fill = matches!(operator, b"sc" | b"scn");
                    self.set_colour(operands, resources, &mut state, fill);
                }

                // --- text ---
                b"BT" => {
                    in_text = true;
                    // Table 105: `BT` initialises both matrices. Resetting the whole
                    // structure also discards any glyph outlines a malformed stream left
                    // unconsumed by an `ET`, which is the only state a second `BT` could
                    // otherwise carry into the text object it starts.
                    text_object = TextObject {
                        start: self.list.command_count(),
                        ..TextObject::default()
                    };
                }
                b"ET" => {
                    in_text = false;
                    self.end_text_object(&mut text_object, &mut state);
                }
                b"Tf" => {
                    if let Some(name) = name_at(operands, 0) {
                        state.text.font = self.font(resources, &name);
                        // The trace's label rather than the lookup's key: §7.3.5's "the bytes
                        // making up the name are never treated as text" binds the second, and
                        // `font` above has them.
                        state.text.font_name =
                            String::from_utf8_lossy(name.as_bytes()).into_owned();
                    }
                    if let Some(size) = number_at(operands, 1) {
                        state.text.size = size;
                    }
                }
                b"Tc" => {
                    if let Some(value) = number_at(operands, 0) {
                        state.text.char_spacing = value;
                    }
                }
                b"Tw" => {
                    if let Some(value) = number_at(operands, 0) {
                        state.text.word_spacing = value;
                    }
                }
                b"Tz" => {
                    if let Some(percent) = number_at(operands, 0) {
                        state.text.horizontal_scale = percent / 100.0;
                    }
                }
                b"TL" => {
                    if let Some(value) = number_at(operands, 0) {
                        state.text.leading = value;
                    }
                }
                b"Ts" => {
                    if let Some(value) = number_at(operands, 0) {
                        state.text.rise = value;
                    }
                }
                b"Tr" => {
                    if let Some(mode) = integer_at(operands, 0) {
                        // Table 104 defines eight modes and no default for anything else.
                        // Silently keeping an out-of-range value would draw nothing at all,
                        // since none of the three operations would match it — a whole text
                        // object missing, with no report. The mode is left as it was and the
                        // operand is named instead.
                        if (0..=7).contains(&mode) {
                            state.text.render_mode = mode;
                        } else {
                            self.note(Unsupported::Operator {
                                operator: format!("Tr with mode {mode}"),
                            });
                        }
                    }
                }
                b"Td" => {
                    if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                        text_object.line = Transform::translate(x, y).then(text_object.line);
                        text_object.matrix = text_object.line;
                    }
                }
                b"TD" => {
                    if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                        // `TD` is `Td` with the side effect of setting the leading.
                        state.text.leading = -y;
                        text_object.line = Transform::translate(x, y).then(text_object.line);
                        text_object.matrix = text_object.line;
                    }
                }
                b"Tm" => {
                    if let Some(matrix) = matrix_from(operands) {
                        text_object.line = matrix;
                        text_object.matrix = matrix;
                    }
                }
                b"T*" => {
                    text_object.line =
                        Transform::translate(0.0, -state.text.leading).then(text_object.line);
                    text_object.matrix = text_object.line;
                }
                b"Tj" => {
                    if let Some(bytes) = string_at(operands, 0) {
                        self.show_text(&bytes, &state, &mut text_object, resources, form_depth);
                    }
                }
                b"TJ" => {
                    // The array operand is not reconstructed by the content lexer, so the
                    // strings and the numeric adjustments between them arrive as separate
                    // operands in order — which is enough to render them correctly.
                    for operand in operands {
                        match operand {
                            Object::String(bytes) => {
                                self.show_text(
                                    bytes,
                                    &state,
                                    &mut text_object,
                                    resources,
                                    form_depth,
                                );
                            }
                            other => {
                                if let Some(adjust) = other.as_number() {
                                    // §9.4.3: the amount "shall be subtracted from the current
                                    // horizontal or vertical coordinate, depending on the
                                    // writing mode", in thousandths of an em scaled by the
                                    // size — and by the horizontal scaling only in the
                                    // horizontal mode, which is where §9.4.4's `Th` sits.
                                    let shift = -narrow(adjust) / 1000.0 * state.text.size;
                                    let vertical =
                                        state.text.font.as_ref().is_some_and(Font::is_vertical);
                                    let step = if vertical {
                                        Transform::translate(0.0, shift)
                                    } else {
                                        Transform::translate(
                                            shift * state.text.horizontal_scale,
                                            0.0,
                                        )
                                    };
                                    text_object.matrix = step.then(text_object.matrix);
                                }
                            }
                        }
                    }
                }
                b"'" => {
                    text_object.line =
                        Transform::translate(0.0, -state.text.leading).then(text_object.line);
                    text_object.matrix = text_object.line;
                    if let Some(bytes) = string_at(operands, 0) {
                        self.show_text(&bytes, &state, &mut text_object, resources, form_depth);
                    }
                }
                b"\"" => {
                    // `aw ac string "` sets word and character spacing, then shows.
                    if let Some(word) = number_at(operands, 0) {
                        state.text.word_spacing = word;
                    }
                    if let Some(character) = number_at(operands, 1) {
                        state.text.char_spacing = character;
                    }
                    text_object.line =
                        Transform::translate(0.0, -state.text.leading).then(text_object.line);
                    text_object.matrix = text_object.line;
                    if let Some(bytes) = string_at(operands, 2) {
                        self.show_text(&bytes, &state, &mut text_object, resources, form_depth);
                    }
                }

                // --- XObjects ---
                b"Do" => self.draw_xobject(operands, resources, &state, form_depth),

                // --- shadings and inline images ---
                b"sh" => {
                    let name = name_at(operands, 0).unwrap_or_else(|| Name::new(Vec::new()));
                    self.paint_shading(&name, resources, &state);
                }
                // §8.9.7: an image written into the content stream rather than as an
                // `XObject`. `crate::inline_image` turns it into the stream the same image
                // would have been as an `XObject`, so from here on it is an ordinary image —
                // including §8.6.8's rule about an uncoloured figure, which `draw_image`
                // owns and which is what a Type 3 glyph drawn as an inline mask needs.
                //
                // The lexer is moved past the data on every path, error included: the bytes
                // between `ID` and `EI` are not a program, and tokenising them would emit
                // drawing commands from image samples.
                b"BI" => {
                    let scanned = self.inline_image(reader, resources);
                    reader.skip(scanned.resume);
                    // A hidden layer suppresses the drawing and the report both: an image
                    // the document turns off is not one we failed to draw (§8.11.3.1).
                    if !self.is_hidden() {
                        match scanned.image {
                            Ok(stream) => {
                                // The stream is new at every `BI`, so `image::RasterCache` is
                                // told to name it by its content: a hatching states the same
                                // few samples once per tiling cell, and an entry named by this
                                // allocation's address could answer none of them (ADR 0399).
                                let stream = Arc::new(stream);
                                self.draw_image(
                                    crate::image::NamedStream::inline(&stream),
                                    "<inline>",
                                    resources,
                                    &state,
                                );
                            }
                            Err(error) => self.note(Unsupported::Image {
                                name: format!("<inline>: {error}"),
                            }),
                        }
                    }
                }

                // Operators that affect no geometry this renderer produces: marked
                // content and compatibility sections carry structure rather than drawing;
                // rendering intent needs colour management; and flatness tolerance is a
                // hint about curve subdivision that the rasteriser decides for itself.
                b"ri" => {
                    // §8.6.5.8's first route to the intent: "Rendering intents shall be
                    // specified with the ri operator". What the intent then does to black point
                    // compensation is §8.6.5.9's and is asked of the state when an object is
                    // painted — setting it here would make the *order* of `ri` and `gs` decide
                    // an answer neither clause makes conditional on one.
                    if let Some(name) = name_at(operands, 0) {
                        state.intent = Intent::read(name.as_bytes());
                    }
                }
                // §8.11.3.2: a marked-content section is optional content when its tag is
                // `OC` and its property list names a group or a membership dictionary.
                // Because a group is an indirect object, the operand is a *name* into the
                // resource dictionary's `/Properties`; an inline dictionary cannot carry
                // one, so it governs nothing.
                b"BDC" => {
                    // A *tag* is one of the standard's own names rather than a resource name, so
                    // it is compared against an ASCII literal — as bytes, which §7.3.5 makes the
                    // comparison a name has ("an exact binary match") and which costs nothing.
                    let tag = name_at(operands, 0);
                    let tag = tag.as_ref().map_or(&[][..], Name::as_bytes);
                    let hides = tag == b"OC"
                        && name_at(operands, 1).is_some_and(|name| {
                            self.unresolved_resource(resources, "Properties", &name)
                                .is_some_and(|oc| !self.shows_optional_content(&oc))
                        });
                    // §14.9's four entries, all from the one property list this section
                    // names, so that a tagged page reads it once rather than four times.
                    let (actual_text, described) = self.accessibility(resources, operands.get(1));
                    // §14.8.2.2.2's two forms of an artifact are `/Artifact BMC` and
                    // `/Artifact <<propertyList>> BDC`; this is the second, and the property
                    // list is Table 363's.
                    let artifact = (tag == b"Artifact").then(|| {
                        self.property_list(resources, operands.get(1))
                            .map(|list| crate::structure::Artifact::read(self.document, &list))
                            .unwrap_or_default()
                    });
                    let reversed = tag == b"ReversedChars";
                    // §14.13.5: "One or more files may be associated with sections of content in
                    // a content stream by enclosing those sections between the marked-content
                    // operators BDC and EMC … with a marked-content tag of AF." NOTE 2 is why
                    // this is on `BDC` alone: "[t]he BMC operator does not take properties and
                    // therefore cannot be used with the AF key." The *tag* is `AF`; the key
                    // inside the property list is `/MCAF` since Errata Collection 3, and the
                    // two are not the same word by accident — see
                    // `attachment::associated_in_property_list`.
                    let associated = if tag == b"AF" {
                        self.property_list(resources, operands.get(1))
                            .map(|list| {
                                crate::attachment::associated_in_property_list(self.document, &list)
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    let mcid = self
                        .property_list(resources, operands.get(1))
                        .and_then(|list| self.document.get_key(&list, "MCID").as_integer());
                    marked.push(Marked {
                        hides,
                        starts_at: self.text.len(),
                        mcid,
                        // §14.9.4's replacement text, which belongs to *extraction* and not to
                        // drawing: the marks are unchanged and what a reader copies is not.
                        actual_text,
                        described,
                        artifact,
                        reversed,
                        associated,
                    });
                    if hides {
                        self.hidden = self.hidden.saturating_add(1);
                    }
                    if reversed {
                        self.reversed_chars = self.reversed_chars.saturating_add(1);
                    }
                }
                b"BMC" => {
                    let tag = name_at(operands, 0);
                    let tag = tag.as_ref().map_or(&[][..], Name::as_bytes);
                    // The generic forms: `/Artifact BMC` states an artifact with no property
                    // list, and `/ReversedChars BMC` is the form §14.8.2.5.3's own EXAMPLE uses.
                    let reversed = tag == b"ReversedChars";
                    marked.push(Marked {
                        starts_at: self.text.len(),
                        artifact: (tag == b"Artifact").then(crate::structure::Artifact::default),
                        reversed,
                        ..Marked::default()
                    });
                    if reversed {
                        self.reversed_chars = self.reversed_chars.saturating_add(1);
                    }
                }
                b"EMC" => {
                    if let Some(section) = marked.pop() {
                        if section.hides {
                            self.hidden = self.hidden.saturating_sub(1);
                        }
                        if section.reversed {
                            self.reversed_chars = self.reversed_chars.saturating_sub(1);
                        }
                        // "The ActualText value shall be used as a replacement, not a
                        // description, for the content" — so whatever the enclosed operators
                        // read back is discarded and the stated text stands in its place.
                        let replaced_here = section.actual_text.is_some();
                        if let Some(replacement) = section.actual_text
                            && section.starts_at <= self.text.len()
                        {
                            self.text.truncate(section.starts_at);
                            // "If each of two (or more) consecutive structure or marked-content
                            // sequences has an ActualText entry, they shall be treated as if no
                            // word break is present between them." The space between them is not
                            // in either sequence — it is what the *placement* pass inferred from
                            // the gap the glyphs left — so the clause is asking for it to go.
                            // Only whitespace is removed, and only where the previous section's
                            // replacement ended where this one's text began: a real character
                            // between the two means they are not consecutive.
                            if let Some(end) = replaced_ends_at
                                && self.text.get(end..).is_some_and(|between| {
                                    !between.is_empty() && between.chars().all(char::is_whitespace)
                                })
                            {
                                self.text.truncate(end);
                            }
                            self.text.push_str(&replacement);
                            replaced_ends_at = Some(self.text.len());
                        } else if !replaced_here && self.text.len() > section.starts_at {
                            // Marks were made that no `/ActualText` replaced, so whatever came
                            // before is no longer adjacent to whatever comes next.
                            replaced_ends_at = None;
                        }
                        // §14.9.3's and §14.9.5's substitutions are recorded over the range
                        // rather than applied to it: they are what the page is *spoken* as,
                        // and the text a person copies is unchanged by either.
                        if let Some(described) = section.described {
                            self.described.push(crate::accessibility::Described {
                                range: section.starts_at..self.text.len(),
                                alt: described.alt,
                                expansion: described.expansion,
                                language: described.language,
                            });
                        }
                        // Last, because the range is over the readback *as it now stands*:
                        // an artifact whose section also states an `/ActualText` has had its
                        // text replaced by the block above, and a range taken before that
                        // would name the text the replacement removed.
                        if let Some(artifact) = section.artifact {
                            self.artifacts.push(ArtifactSpan {
                                range: section.starts_at..self.text.len(),
                                artifact,
                            });
                        }
                        // §14.7.5.2's identifier over the same range, for the same reason and
                        // after the same replacements: this is where a structure element's
                        // content actually is in the readback.
                        if let Some(mcid) = section.mcid {
                            self.marked.push(MarkedSpan {
                                mcid,
                                range: section.starts_at..self.text.len(),
                            });
                        }
                        // §14.13.5's files belong to the graphics objects the section enclosed,
                        // so like an artifact they are recorded over the range rather than
                        // changing anything drawn.
                        for attachment in section.associated {
                            self.associated
                                .push((section.starts_at..self.text.len(), attachment));
                        }
                    }
                }
                b"BX" => compatibility = compatibility.saturating_add(1),
                b"EX" => compatibility = compatibility.saturating_sub(1),
                b"MP" | b"DP" | b"i" => {}

                // --- Type 3 glyph metrics (§9.6.4 Table 111) ---
                //
                // Both operators state the glyph's horizontal displacement, and the width
                // used is the font dictionary's `/Widths` entry instead: Table 111 requires
                // the two to agree ("it shall be consistent with the corresponding width in
                // the font's Widths array"), and Table 110 makes `/Widths` required, so the
                // font dictionary is the one statement present for every glyph — including
                // the ones whose `/CharProcs` entry is missing and which are never run.
                //
                // `d1` additionally declares the glyph uncoloured, which is the half that
                // changes what is drawn; see the intercept above. Its bounding box is
                // deliberately not used as a clip: Table 111 requires it to enclose the
                // glyph ("the declared bounding box shall be correct"), so clipping to it
                // can only ever remove marks a correct file does not have, and on an
                // incorrect one it hides the defect rather than reporting it.
                b"d0" | b"d1" => {
                    if operator == b"d1" && self.glyph_depth > 0 {
                        // `d1` forbids the *description* from setting a colour, and nothing
                        // more. What the description paints with is the graphics state
                        // §9.6.4 says it inherited — both colour parameters, each used by
                        // the operation that selects it, so a description that strokes
                        // strokes in the stroking colour.
                        //
                        // **This branch collapsed the two into one until the
                        // five-hundred-and-fifty-eighth session**, on Table 111's singular
                        // "[i]ts colour shall be determined by the graphics state in effect
                        // each time this glyph is painted by a text-showing operator". The
                        // clause refutes that reading three times over. §9.6.4 NOTE 2 is
                        // plural — "it is unnecessary and undesirable to initialise the
                        // current colour parameters because the text-showing operators are
                        // designed to paint glyphs with the current colours" — the sentence
                        // above it lists what a stroking description must set for itself
                        // ("the line width, line join, line cap, and dash pattern") and puts
                        // no colour in that list, and §9.6.4's own EXAMPLE sets a distinct
                        // `RG` before each `Tj` for a `d1` `square` glyph whose body is
                        // `72 w 0 0 750 750 re B`, which the collapsed reading makes dead
                        // syntax. `Type3Test.pdf` in `pdf-differences` is the corpus witness
                        // and ADR 0393 has the argument; the image-mask sentence the old
                        // comment leaned on is §8.9.6.2's rule about a *stencil*, which
                        // paints with the non-stroking colour because that is what an image
                        // mask does, not because a `d1` glyph has one colour.
                        self.uncoloured = true;
                    }
                }

                other => {
                    if compatibility == 0 {
                        self.note(Unsupported::Operator {
                            operator: String::from_utf8_lossy(other).into_owned(),
                        });
                    }
                }
            }

            pending.clear();
        }

        // An unclosed `BT` is malformed but harmless here; noted so it is not invisible.
        // Any glyph outlines a clipping render mode accumulated are discarded with it —
        // §9.3.6 makes `ET` the moment they become a clip, and a text object that never
        // ended never reached it.
        if in_text {
            self.note(Unsupported::Operator {
                operator: "BT without ET".to_owned(),
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
    }
}

/// The operators ISO 32000-2 §8.6.8 says an uncoloured figure's content stream may not use.
///
/// The clause's own list, in full: the twelve colour operators of Table 73, plus `ri` and
/// `sh`. The last two are worth noticing rather than copying — `ri` sets a rendering intent,
/// which is colour-related without being a colour, and `sh` paints a *shading*, which carries
/// its own colours and so cannot belong to a figure whose colour comes from outside it.
///
/// `gs` is not here, because an `/ExtGState` sets much more than colour — including the line
/// width and dash pattern §9.6.4 tells a glyph description to set explicitly. The clause
/// lists the entries *within* it that are ignored, and `apply_ext_gstate` drops those.
fn is_colour_operator(operator: &[u8]) -> bool {
    matches!(
        operator,
        b"CS"
            | b"cs"
            | b"SC"
            | b"SCN"
            | b"sc"
            | b"scn"
            | b"G"
            | b"g"
            | b"RG"
            | b"rg"
            | b"K"
            | b"k"
            | b"ri"
            | b"sh"
    )
}

/// What the operator loop must do about a token it has just read.
///
/// The [`ContentReader::with_token`] closure returns one of these, which is what carries a
/// token's meaning *out* of the borrow it was read under. An operand carries nothing: the
/// closure has already pushed it. What is left needs either the reader again, or a report, or
/// the dispatch table — and an operator's bytes come out on the stack rather than borrowed
/// (see [`Word`]), because the dispatch calls the reader again for §8.9.7's inline images.
#[derive(Debug)]
enum Step {
    /// A bare keyword where an operator belongs: the dispatch table's business.
    Operator(Word),
    /// An operand, already in the pending list.
    Operand,
    /// A keyword inside an array, which §7.3.6 admits nothing but objects into.
    InsideAnArray(Word),
    /// `<<` — an inline dictionary, which the caller assembles because it needs the reader.
    Dictionary,
    /// One more operand than `MAX_OPERANDS` allows.
    TooManyOperands,
    /// The stream has no more tokens.
    End,
}

/// What one value of an inline dictionary or array is, out of the token's borrow.
///
/// [`Step`]'s counterpart for the two constructions §14.6.2 writes inside a content stream.
/// The two that need the reader again are named rather than built here, for the reason
/// [`ContentReader::with_token`] gives: the token is lent, and the reader cannot be asked for
/// the next one while it is still alive.
#[derive(Debug)]
enum Value {
    /// A direct object, already built.
    Object(Object),
    /// `<<` — a dictionary nested inside this one.
    Dictionary,
    /// `[` — an array.
    Array,
    /// The construction's own closer, or the end of the stream.
    End,
}

/// Converts a content-stream token into an operand.
fn token_to_object(token: pdf_syntax::Token<'_>) -> Object {
    match token {
        pdf_syntax::Token::Integer(value) => Object::Integer(value),
        pdf_syntax::Token::Real(value) => Object::Real(value),
        pdf_syntax::Token::Name(bytes) => Object::Name(Name::new(bytes)),
        pdf_syntax::Token::String(bytes) => Object::String(bytes.into()),
        // Arrays and dictionaries appear as operands to `d`, `TJ` and `BDC`. Recognising
        // the brackets is enough for the operators this interpreter implements; a full
        // re-parse would duplicate the object parser for no present gain.
        _ => Object::Null,
    }
}

/// Assembles an inline dictionary from a content stream's tokens, after its `<<`.
///
/// The content lexer yields tokens and not objects, so a dictionary written inside a content
/// stream — which only `BDC` and the inline-image operators use — has to be put together here.
///
/// Array values were read as far as their brackets and discarded until the eighty-third
/// session, on the reasoning that "no property list entry this tree reads is an array". That
/// stopped being true the moment §14.8.2.2's artifacts were read: Table 363's `/BBox` and
/// `/Attached` are both arrays, and both came back empty from a parser that was recognising
/// the brackets without reading between them — which is this project's own trap 8 in
/// `doc/HANDOVER.md`, met from the inside.
///
/// An unterminated dictionary ends with the stream, which is what a truncated content stream
/// leaves behind; the entries read before it are still the ones the file stated.
fn inline_dictionary(reader: &mut ContentReader<'_>, depth: usize) -> Dictionary {
    /// How deep a dictionary may nest inside a content stream.
    ///
    /// A property list is one level in every use the standard defines; this bounds a hostile
    /// stream that opens dictionaries and never closes them.
    const MAX_DEPTH: usize = 8;

    let mut dict = Dictionary::new();
    if depth > MAX_DEPTH {
        return dict;
    }
    loop {
        let key = reader.with_token(|token| match token {
            Some(pdf_syntax::Token::DictClose) | None => None,
            Some(pdf_syntax::Token::Name(bytes)) => Some(Some(Name::new(bytes))),
            // Anything that is not a name where a key belongs is a malformed dictionary;
            // skipping the token keeps the rest of the entries readable.
            Some(_) => Some(None),
        });
        let Some(key) = key else { break };
        let Some(key) = key else { continue };

        let value = reader.with_token(|token| match token {
            Some(pdf_syntax::Token::DictOpen) => Value::Dictionary,
            Some(pdf_syntax::Token::ArrayOpen) => Value::Array,
            Some(pdf_syntax::Token::DictClose) | None => Value::End,
            // `true`, `false` and `null` lex as keywords in a content stream, which is why
            // two corpus documents used to report them as unknown *operators*: an inline
            // property list's booleans were reaching the operator dispatch one token at a
            // time. §7.3.2 makes them objects wherever an object belongs.
            Some(pdf_syntax::Token::Keyword(word)) => Value::Object(match word {
                b"true" => Object::Boolean(true),
                b"false" => Object::Boolean(false),
                _ => Object::Null,
            }),
            Some(other) => Value::Object(token_to_object(other)),
        });
        let value = match value {
            Value::End => break,
            Value::Dictionary => {
                Object::Dictionary(inline_dictionary(reader, depth.saturating_add(1)))
            }
            Value::Array => Object::Array(inline_array(reader, 0)),
            Value::Object(object) => object,
        };
        dict.insert(key, value);
    }
    dict
}

/// Assembles an array from a content stream's tokens, after its `[`.
///
/// Bounded in both directions a hostile stream can grow: the nesting, by the same constant
/// [`inline_dictionary`] uses, and the number of elements — a property list is a handful of
/// numbers or names, and an array of a million of them is a file making a reader work.
fn inline_array(reader: &mut ContentReader<'_>, depth: usize) -> Vec<Object> {
    /// The same bound as [`inline_dictionary`]'s, and for the same reason.
    const MAX_DEPTH: usize = 8;
    /// Most elements read from one array written inside a content stream.
    const MAX_ELEMENTS: usize = 65_536;

    let mut out = Vec::new();
    if depth > MAX_DEPTH {
        // Consumed rather than left, so the caller resumes at the right token: an array this
        // deep is nothing this reader will use, and the stream after it still has to parse.
        while reader.with_token(|token| match token {
            None | Some(pdf_syntax::Token::ArrayClose) => false,
            Some(_) => true,
        }) {}
        return out;
    }
    loop {
        let step = reader.with_token(|token| match token {
            None | Some(pdf_syntax::Token::ArrayClose) => Value::End,
            Some(pdf_syntax::Token::ArrayOpen) => Value::Array,
            Some(pdf_syntax::Token::DictOpen) => Value::Dictionary,
            // As in a dictionary's values: §7.3.2's booleans and §7.3.9's null lex as
            // keywords inside a content stream.
            Some(pdf_syntax::Token::Keyword(word)) => Value::Object(match word {
                b"true" => Object::Boolean(true),
                b"false" => Object::Boolean(false),
                _ => Object::Null,
            }),
            Some(other) => Value::Object(token_to_object(other)),
        });
        let value = match step {
            Value::End => break,
            Value::Array => Object::Array(inline_array(reader, depth.saturating_add(1))),
            Value::Dictionary => {
                Object::Dictionary(inline_dictionary(reader, depth.saturating_add(1)))
            }
            Value::Object(object) => object,
        };
        if out.len() < MAX_ELEMENTS {
            out.push(value);
        }
    }
    out
}

/// The operands one operator is given, out of everything the stream has stated since the
/// last one.
///
/// ISO 32000-2 §7.8.2 states the rule this implements:
///
/// > In PDF, all of the operands needed by an operator shall immediately precede that
/// > operator. Operators do not return results, and operands shall not be left over when an
/// > operator finishes execution.
///
/// So an operator's operands are the *last* `n` of what precedes it, not the first. On a
/// conforming stream nothing is ever left over and the two readings are the same slice; the
/// sentence decides only what a malformed one means, and there the difference is total —
/// reading from the front shifts every operand by however many the file left behind, which
/// silently mis-draws a mark or, where the shifted operand has the wrong type, drops it with
/// nothing reported.
///
/// `count_of` returns [`None`] for the operators whose own table states no fixed number, and
/// those are given everything: `TJ` and `d` take an array the content lexer flattens into one
/// operand per element, and `sc`, `scn`, `SC` and `SCN` take as many components as the current
/// colour space has (§8.6.8, Table 73). A leftover operand in front of one of those is
/// indistinguishable from one of its own, so nothing here can improve on reading them whole.
fn operands_before<'a>(pending: &'a [Object], operator: &[u8]) -> &'a [Object] {
    let Some(count) = count_of(operator) else {
        return pending;
    };
    let start = pending.len().saturating_sub(count);
    pending.get(start..).unwrap_or(pending)
}

/// How many operands an operator takes, where its own table states a fixed number.
///
/// Each count is the operand list in the table Annex A's summary points at for that operator
/// — Table 56 and 57 for the graphics state, 58 and 59 for paths, 60 for clipping, 73 for
/// colour, 74 for shading, 87 for `XObject`s, 103, 105, 106 and 107 for text, 111 for a Type 3
/// glyph's metrics, 33 for compatibility and 352 for marked content.
///
/// An operator this interpreter does not implement reaches here too, and gets [`None`]: it is
/// reported rather than run, so how many operands it wanted is not a question anything asks.
const fn count_of(operator: &[u8]) -> Option<usize> {
    Some(match operator {
        b"q" | b"Q" | b"h" | b"n" | b"f" | b"F" | b"f*" | b"S" | b"s" | b"B" | b"B*" | b"b"
        | b"b*" | b"W" | b"W*" | b"BT" | b"ET" | b"T*" | b"EMC" | b"BX" | b"EX" | b"BI" => 0,
        b"w" | b"J" | b"j" | b"M" | b"i" | b"ri" | b"gs" | b"Tc" | b"Tw" | b"Tz" | b"TL"
        | b"Ts" | b"Tr" | b"Tj" | b"'" | b"cs" | b"CS" | b"g" | b"G" | b"sh" | b"Do" | b"MP"
        | b"BMC" => 1,
        b"m" | b"l" | b"Td" | b"TD" | b"Tf" | b"d0" | b"DP" | b"BDC" => 2,
        b"rg" | b"RG" | b"\"" => 3,
        b"v" | b"y" | b"re" | b"k" | b"K" => 4,
        b"cm" | b"c" | b"Tm" | b"d1" => 6,
        _ => return None,
    })
}

/// Reads operand `index` as an integer code.
///
/// Accepts a real that happens to be integral, since producers write `1.0` where `1` is
/// meant.
fn integer_at(operands: &[Object], index: usize) -> Option<i64> {
    let value = operands.get(index)?;
    value.as_integer().or_else(|| {
        let number = value.as_number()?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "guarded to an integral value below 1000 by the condition"
        )]
        let code = number as i64;
        (number.is_finite() && number.fract() == 0.0 && number.abs() < 1000.0).then_some(code)
    })
}

/// Reads operand `index` as a number.
pub(super) fn number_at(operands: &[Object], index: usize) -> Option<f32> {
    let value = operands.get(index)?.as_number()?;
    if !value.is_finite() {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "content-stream coordinates are page-scale; a value outside f32's range \
                  cannot describe a position on a page"
    )]
    Some(value as f32)
}

/// Reads the first `N` operands as numbers, requiring all of them.
///
/// **`N` is a constant rather than an argument, and the array is the whole reason.** Annex A
/// gives every operator that reaches here a fixed operand count — `count_of` is that table —
/// so the answer's size is known where it is asked for, and returning it on the stack costs
/// no allocation at all. The `Vec` this used to return did: with a `filter_map`'s lower size
/// hint of zero, `collect` began at capacity nought and grew, so six numbers were a `malloc`
/// and two `realloc`s. On the witness of `doc/todo/44` — one page, 3.19 M operators, most of
/// them `c` — that was **12.00% of the whole interpretation** in the collect and its
/// reallocation, and `points_from`'s second `Vec` another 2.83% (ADR 0370's table).
///
/// What it costs in readability is one type parameter at six call sites; what it buys is
/// measured in ADR 0370.
fn numbers_from<const N: usize>(operands: &[Object]) -> Option<[f32; N]> {
    let mut values = [0.0_f32; N];
    for (index, slot) in values.iter_mut().enumerate() {
        *slot = number_at(operands, index)?;
    }
    Some(values)
}

/// Reads `N` coordinate pairs, requiring all of them.
fn points_from<const N: usize>(operands: &[Object]) -> Option<[Point; N]> {
    let mut points = [Point::new(0.0, 0.0); N];
    // The pairs are read directly rather than through `numbers_from`, because a const
    // parameter cannot be doubled in a type without `generic_const_exprs`. Two `number_at`s
    // per point is what the old two-step did anyway, minus the intermediate array.
    let mut index = 0_usize;
    for slot in &mut points {
        let x = number_at(operands, index)?;
        let y = number_at(operands, index.saturating_add(1))?;
        *slot = Point::new(x, y);
        index = index.saturating_add(2);
    }
    Some(points)
}

/// Reads six operands as a matrix.
fn matrix_from(operands: &[Object]) -> Option<Transform> {
    let values = numbers_from::<6>(operands)?;
    Some(Transform::new(
        values[0], values[1], values[2], values[3], values[4], values[5],
    ))
}

/// Reads operand `index` as a string.
fn string_at(operands: &[Object], index: usize) -> Option<Vec<u8>> {
    operands.get(index)?.as_string().map(<[u8]>::to_vec)
}

/// Narrows a PDF number to `f32`.
pub(super) fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a text adjustment outside f32's range is not a position on a page"
    )]
    {
        value as f32
    }
}

/// Reads operand `index` as a name, keeping the bytes §7.3.5 says a name is.
///
/// > Uniquely defined means that any two name objects that, after all escaping is expanded (see
/// > below), and the resulting sequences of bytes are not an exact binary match denote different
/// > objects.
///
/// So the operand travels to `resources.rs` as a [`Name`]: a `String` built with
/// `from_utf8_lossy` on the way — which is what this returned until the
/// six-hundred-and-third session — turns every byte outside UTF-8 into U+FFFD and makes the
/// resource it names unfindable (ADR 0438). A *tag* is compared against one of the standard's
/// own ASCII names and may be read as bytes here too, which is what `== b"OC"` is.
pub(super) fn name_at(operands: &[Object], index: usize) -> Option<Name> {
    operands.get(index)?.as_name().cloned()
}
