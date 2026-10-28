//! Table 50's text object, text state, text positioning and text showing operators, sent to the
//! method for the group the table puts each in.

use super::machine::Machine;
use super::operands::{integer_at, matrix_from, name_at, narrow, number_at, string_at};

use pdf_render::Transform;
use pdf_syntax::{Dictionary, Object};

use crate::content::font::Font;

use crate::content::reader::Operator;
use crate::content::report::Unsupported;
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
            | Operator::NextLine => Self::run_text_positioning_op(operator, operands, machine),
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
                *in_text = true;
                // Table 105: `BT` initialises both matrices. Resetting the whole
                // structure also discards any glyph outlines a malformed stream left
                // unconsumed by an `ET`, which is the only state a second `BT` could
                // otherwise carry into the text object it starts.
                // A `BT` inside a text object that never ended closes that object's record
                // of §11.6.4.3's readings before this one opens its own.
                if let Some(outer) = text_object.enclosing_reading.take() {
                    let (reading, in_force) = self.close_reading_scope(outer);
                    self.absorb_reading(reading, text_object.start);
                    self.note_alpha_source(in_force);
                }
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
                *in_text = false;
                self.end_text_object(text_object, state);
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
                if let Some(name) = name_at(operands, 0) {
                    (state.text.font, state.text.owed_font_report) = self.font(resources, &name);
                    // The trace's label rather than the lookup's key: §7.3.5's "the bytes
                    // making up the name are never treated as text" binds the second, and
                    // `font` above has them.
                    state.text.font_name = String::from_utf8_lossy(name.as_bytes()).into_owned();
                }
                if let Some(size) = number_at(operands, 1) {
                    state.text.size = size;
                }
            }
            Operator::CharSpacing => {
                if let Some(value) = number_at(operands, 0) {
                    state.text.char_spacing = value;
                }
            }
            Operator::WordSpacing => {
                if let Some(value) = number_at(operands, 0) {
                    state.text.word_spacing = value;
                }
            }
            Operator::HorizontalScaling => {
                if let Some(percent) = number_at(operands, 0) {
                    state.text.horizontal_scale = percent / 100.0;
                }
            }
            Operator::Leading => {
                if let Some(value) = number_at(operands, 0) {
                    state.text.leading = value;
                }
            }
            Operator::Rise => {
                if let Some(value) = number_at(operands, 0) {
                    state.text.rise = value;
                }
            }
            Operator::RenderMode => {
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
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// The text positioning operators of Table 50 (ISO 32000-2 §9.4.2): `Td`, `TD`, `Tm` and `T*`.
    fn run_text_positioning_op(operator: Operator, operands: &[Object], machine: &mut Machine) {
        let Machine {
            state, text_object, ..
        } = machine;
        match operator {
            Operator::MoveText => {
                if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                    text_object.line = Transform::translate(x, y).then(text_object.line);
                    text_object.matrix = text_object.line;
                }
            }
            Operator::MoveTextSetLeading => {
                if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                    // `TD` is `Td` with the side effect of setting the leading.
                    state.text.leading = -y;
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
            state, text_object, ..
        } = machine;
        match operator {
            Operator::ShowText => {
                if let Some(bytes) = string_at(operands, 0) {
                    self.show_text(&bytes, state, text_object);
                }
            }
            Operator::ShowTextAdjusted => {
                // The array operand is not reconstructed by the content lexer, so the
                // strings and the numeric adjustments between them arrive as separate
                // operands in order — which is enough to render them correctly.
                for operand in operands {
                    match operand {
                        Object::String(bytes) => {
                            self.show_text(bytes, state, text_object);
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
                                    Transform::translate(shift * state.text.horizontal_scale, 0.0)
                                };
                                text_object.matrix = step.then(text_object.matrix);
                            }
                        }
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
                    self.show_text(&bytes, state, text_object);
                }
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }
}
