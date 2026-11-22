//! Table 50's text object, text state, text positioning and text showing operators, sent to the
//! method for the group the table puts each in.

use super::machine::Machine;
use super::operands::{FlatArray, integer_at, matrix_from, name_at, narrow, number_at, string_at};

use pdf_render::Transform;
use pdf_syntax::{Dictionary, Object};

use crate::content::font::Font;

use crate::content::reader::Operator;
use crate::content::report::{
    ELEMENT_NEITHER_STRING_NOR_NUMBER, OUTSIDE_A_TEXT_OBJECT, OUTSIDE_ITS_ARRAY,
    POSITIONING_OPERAND_NOT_A_NUMBER, SPACING_OPERAND_NOT_A_NUMBER, TEXT_OBJECT_NESTED,
    TEXT_STATE_OPERAND_TYPE, Unsupported,
};
use crate::content::text::TextObject;
use crate::content::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// The text object, text state and text positioning operators of Table 50 (ISO 32000-2 §9.3
    /// and §9.4): `BT`, `ET`, `Tc`, `Tw`, `Tz`, `TL`, `Tf`, `Tr`, `Ts`, `Td`, `TD`, `Tm` and `T*`,
    /// sent to the method for the group Table 50 puts each in.
    pub(super) fn run_text_op(
        &mut self,
        operator: Operator,
        operands: &[Object],
        resources: &Dictionary,
        machine: &mut Machine,
    ) {
        match operator {
            Operator::BeginText | Operator::EndText => self.run_text_object_op(operator, machine),
            Operator::Font
            | Operator::CharSpacing
            | Operator::WordSpacing
            | Operator::HorizontalScaling
            | Operator::Leading
            | Operator::Rise
            | Operator::RenderMode => {
                self.run_text_state_op(operator, operands, resources, &mut machine.state);
            }
            Operator::MoveText
            | Operator::MoveTextSetLeading
            | Operator::TextMatrix
            | Operator::NextLine => self.run_text_positioning_op(operator, operands, machine),
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// The text object operators of Table 50 (ISO 32000-2 §9.4.1): `BT` and `ET`.
    fn run_text_object_op(&mut self, operator: Operator, machine: &mut Machine) {
        let Machine {
            state,
            text_object,
            in_text,
            ..
        } = machine;
        match operator {
            Operator::BeginText => {
                // Table 105: "Text objects shall not be nested; a second BT shall not appear
                // before an ET." The clause states no reading of one, and `BT`'s own sentence
                // decides it: it begins a text object "initializing the text matrix, Tm, and the
                // text line matrix, Tlm, to the identity matrix", which is only possible once the
                // open object has ended. So the open object ends here as `ET` would end it —
                // §9.3.6's clip from the outlines it accumulated, and §11.7.4.4's and §9.3.8's
                // groups for the glyphs it showed — and the stream's fault is said (ADR 3353).
                if *in_text {
                    self.note(Unsupported::Operator {
                        operator: TEXT_OBJECT_NESTED.to_owned(),
                    });
                    self.end_text_object(text_object, state);
                }
                *in_text = true;
                // Resetting the whole structure is Table 105's two matrices and §9.3.6's empty
                // accumulator in one line.
                *text_object = TextObject {
                    start: self.list.command_count(),
                    enclosing_reading: Some(if state.text.knockout {
                        self.open_knockout_reading_scope(state.alpha_is_shape)
                    } else {
                        self.open_reading_scope(state.alpha_is_shape)
                    }),
                    ..TextObject::default()
                };
            }
            Operator::EndText => {
                if !*in_text {
                    // Nothing is open to end, and nothing is owed: the stray keyword is said and
                    // acts on nothing.
                    self.note(Unsupported::Operator {
                        operator: format!("ET{OUTSIDE_A_TEXT_OBJECT}"),
                    });
                    return;
                }
                *in_text = false;
                self.end_text_object(text_object, state);
                // Table 105: `ET` ends the object "discarding the text matrix", and §9.4.1 has
                // Tm and Tlm "not persist from one text object to the next". Discarded is the
                // identity a `BT` would set, which is all a positioning or showing operator the
                // stream wrongly puts after `ET` can be given (ADR 3353).
                *text_object = TextObject::default();
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// The text state operators of Table 50 (ISO 32000-2 §9.3): `Tc`, `Tw`, `Tz`, `TL`, `Tf`, `Tr`
    /// and `Ts`.
    fn run_text_state_op(
        &mut self,
        operator: Operator,
        operands: &[Object],
        resources: &Dictionary,
        state: &mut GraphicsState,
    ) {
        match operator {
            Operator::Font => {
                // Table 103: "There is no initial value for either font or size; they shall be
                // specified explicitly by using Tf before any text is shown". A `Tf` states both
                // or leaves what it does not state unset — never the previous font's size under
                // the new font, which nothing in the file states (ADR 0303, ADR 3355).
                if let Some(name) = name_at(operands, 0) {
                    (state.text.font, state.text.owed_font_report) = self.font(resources, &name);
                    // The trace's label rather than the lookup's key: §7.3.5's "the bytes
                    // making up the name are never treated as text" binds the second, and
                    // `font` above has them.
                    state.text.font_name = String::from_utf8_lossy(name.as_bytes()).into_owned();
                } else {
                    state.text.font = None;
                    state.text.owed_font_report = None;
                    state.text.font_name.clear();
                }
                // Unset is a size of nought: Table 103's "[z]ero sized text shall not mark or
                // clip any pixels", which is what a show with no size stated can do.
                state.text.size = number_at(operands, 1).unwrap_or(0.0);
                // Judged only when both operands are there: short of one, the loop has said so,
                // and which of the two the one present was meant to be is not in the file.
                if let [name, size] = operands
                    && (name.as_name().is_none() || size.as_number().is_none())
                {
                    self.note_operand_type(operator);
                }
            }
            Operator::CharSpacing => {
                if let Some(value) = self.text_state_number(operator, operands) {
                    state.text.char_spacing = value;
                }
            }
            Operator::WordSpacing => {
                if let Some(value) = self.text_state_number(operator, operands) {
                    state.text.word_spacing = value;
                }
            }
            Operator::HorizontalScaling => {
                if let Some(percent) = self.text_state_number(operator, operands) {
                    state.text.horizontal_scale = percent / 100.0;
                }
            }
            Operator::Leading => {
                if let Some(value) = self.text_state_number(operator, operands) {
                    state.text.leading = value;
                }
            }
            Operator::Rise => {
                if let Some(value) = self.text_state_number(operator, operands) {
                    state.text.rise = value;
                }
            }
            Operator::RenderMode => {
                // Table 103: "render ... shall be an integer". An operand that is none — `1.5`,
                // a name — states no mode, and is said as an integer outside the table is.
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
                } else if !operands.is_empty() {
                    self.note_operand_type(operator);
                }
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// The one number operand of `Tc`, `Tw`, `Tz`, `TL` or `Ts`, or `None` with the operand
    /// reported where it is present and not a number.
    ///
    /// Table 103 gives each of the five a number. One of another type states no value, so the
    /// parameter keeps the one in force — the answer `Tr` gives a mode outside its table — and the
    /// page says so (ADR 3355). An absent operand was said by the loop as a shortfall.
    fn text_state_number(&mut self, operator: Operator, operands: &[Object]) -> Option<f32> {
        if operands
            .first()
            .is_some_and(|operand| operand.as_number().is_none())
        {
            self.note_operand_type(operator);
        }
        number_at(operands, 0)
    }

    /// Reports a text state operator given an operand of a type Table 103 does not give it.
    fn note_operand_type(&mut self, operator: Operator) {
        self.note(Unsupported::Operator {
            operator: format!(
                "{}{TEXT_STATE_OPERAND_TYPE}",
                String::from_utf8_lossy(operator.name())
            ),
        });
    }

    /// The text positioning operators of Table 50 (ISO 32000-2 §9.4.2): `Td`, `TD`, `Tm` and `T*`.
    ///
    /// §9.4.2: "The text-positioning operators shall only appear within text objects." One outside
    /// is said, and moves the matrices `ET` discarded to the identity — the ones `BT` would set —
    /// so a stream that never wrote `BT` still places its text where its `Td` says (ADR 3353).
    fn run_text_positioning_op(
        &mut self,
        operator: Operator,
        operands: &[Object],
        machine: &mut Machine,
    ) {
        let Machine {
            state,
            text_object,
            in_text,
            ..
        } = machine;
        if !*in_text {
            self.note_outside_a_text_object(operator);
        }
        // Table 106 gives `Td` and `TD` two numbers and `Tm` six ("[t]he operands shall all be
        // numbers"). One present and of another type states no position, so the matrices stay
        // where they are and the page says so (ADR 3379); one absent the loop said as a shortfall.
        let stated = match operator {
            Operator::MoveText | Operator::MoveTextSetLeading => 2,
            Operator::TextMatrix => 6,
            _ => 0,
        };
        if any_not_a_number(operands, stated) {
            self.note_operand_not_a_number(operator, POSITIONING_OPERAND_NOT_A_NUMBER);
        }
        match operator {
            Operator::MoveText => {
                if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                    text_object.line = Transform::translate(x, y).then(text_object.line);
                    text_object.matrix = text_object.line;
                }
            }
            Operator::MoveTextSetLeading => {
                // Table 106: `TD` "shall have the same effect as this code: - t y TL t x t y Td",
                // so a numeric `ty` sets the leading whatever `tx` is, and only the move needs both.
                let y = number_at(operands, 1);
                if let Some(y) = y {
                    state.text.leading = -y;
                }
                if let (Some(x), Some(y)) = (number_at(operands, 0), y) {
                    text_object.line = Transform::translate(x, y).then(text_object.line);
                    text_object.matrix = text_object.line;
                }
            }
            Operator::TextMatrix => {
                if let Some(matrix) = matrix_from(operands) {
                    text_object.line = matrix;
                    text_object.matrix = matrix;
                }
            }
            Operator::NextLine => {
                text_object.line =
                    Transform::translate(0.0, -state.text.leading).then(text_object.line);
                text_object.matrix = text_object.line;
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// The text showing operators of Table 50 (ISO 32000-2 §9.4.3): `Tj`, `TJ`, `'` and `"`.
    pub(super) fn run_text_showing_op(
        &mut self,
        operator: Operator,
        operands: &[Object],
        machine: &mut Machine,
    ) {
        let Machine {
            state,
            text_object,
            in_text,
            ..
        } = machine;
        // §9.4.3: "The text-showing operators shall only appear within text objects." One outside
        // is said, and drawn at the identity matrices `ET` left, never at the discarded object's
        // position (ADR 3353).
        if !*in_text {
            self.note_outside_a_text_object(operator);
        }
        match operator {
            Operator::ShowText => {
                if let Some(bytes) = string_at(operands, 0) {
                    self.show_text(&bytes, state, text_object);
                }
            }
            Operator::ShowTextAdjusted => {
                // Table 107 gives `TJ` one operand, an array, and "[e]ach element of array shall
                // be either a string or a number". The content lexer flattens it, so the array is
                // read out of its brackets by the reading `d` shares (ADR 3354): whatever stands
                // outside it is left over and not read, and an element of another type is not
                // shown.
                let Some(FlatArray {
                    before,
                    elements,
                    after,
                }) = FlatArray::last_of(operands)
                else {
                    if !operands.is_empty() {
                        self.note(Unsupported::Operator {
                            operator: format!("TJ{OUTSIDE_ITS_ARRAY}"),
                        });
                    }
                    return;
                };
                if !before.is_empty() || !after.is_empty() {
                    self.note(Unsupported::Operator {
                        operator: format!("TJ{OUTSIDE_ITS_ARRAY}"),
                    });
                }
                for element in elements {
                    if let Object::String(bytes) = element {
                        self.show_text(bytes, state, text_object);
                    } else if let Some(adjust) = element.as_number() {
                        // §9.4.3: the amount "shall be subtracted from the current horizontal
                        // or vertical coordinate, depending on the writing mode", in thousandths
                        // of an em scaled by the size — and by the horizontal scaling only in
                        // the horizontal mode, which is where §9.4.4's `Th` sits.
                        let shift = -narrow(adjust) / 1000.0 * state.text.size;
                        let vertical = state.text.font.as_ref().is_some_and(Font::is_vertical);
                        let step = if vertical {
                            Transform::translate(0.0, shift)
                        } else {
                            Transform::translate(shift * state.text.horizontal_scale, 0.0)
                        };
                        text_object.matrix = step.then(text_object.matrix);
                    } else {
                        self.note(Unsupported::Operator {
                            operator: format!("TJ{ELEMENT_NEITHER_STRING_NOR_NUMBER}"),
                        });
                    }
                }
            }
            Operator::NextLineShowText => {
                text_object.line =
                    Transform::translate(0.0, -state.text.leading).then(text_object.line);
                text_object.matrix = text_object.line;
                if let Some(bytes) = string_at(operands, 0) {
                    self.show_text(&bytes, state, text_object);
                }
            }
            Operator::NextLineShowTextSpaced => {
                // Table 107: `"` "shall have the same effect as this code: a w Tw a c Tc string '",
                // and `aw` and `ac` "shall be numbers". One of another type is `Tw` or `Tc` given
                // one, so that spacing keeps its value as ADR 3355 has it, and is said (ADR 3379).
                if any_not_a_number(operands, 2) {
                    self.note_operand_not_a_number(operator, SPACING_OPERAND_NOT_A_NUMBER);
                }
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
                    self.show_text(&bytes, state, text_object);
                }
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// Reports a text-positioning operator, or `"`, given an operand its table states as a number
    /// and the file gave as something else; `sentence` names the table.
    fn note_operand_not_a_number(&mut self, operator: Operator, sentence: &str) {
        self.note(Unsupported::Operator {
            operator: format!("{}{sentence}", String::from_utf8_lossy(operator.name())),
        });
    }

    /// Reports a text-positioning or text-showing operator met with no text object open.
    fn note_outside_a_text_object(&mut self, operator: Operator) {
        self.note(Unsupported::Operator {
            operator: format!(
                "{}{OUTSIDE_A_TEXT_OBJECT}",
                String::from_utf8_lossy(operator.name())
            ),
        });
    }
}

/// Whether any of the first `stated` operands is present and not a number. An operand short of
/// `stated` is not judged here: the loop reports a shortfall itself.
fn any_not_a_number(operands: &[Object], stated: usize) -> bool {
    operands
        .iter()
        .take(stated)
        .any(|operand| operand.as_number().is_none())
}
