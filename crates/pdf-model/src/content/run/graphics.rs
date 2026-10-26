//! Table 50's graphics-state, path-construction, path-painting and colour operators.

use super::machine::Machine;
use super::operands::{integer_at, matrix_from, name_at, number_at, numbers_from, points_from};

use pdf_render::{FillRule, PathCommand, Point};
use pdf_syntax::{Dictionary, Object};

use crate::content::colour::Intent;

use crate::content::path::{begin_subpath, close_subpath};
use crate::content::reader::Operator;
use crate::content::report::Unsupported;

use crate::content::{Interpreter, MAX_STATE_DEPTH, line_cap, line_join, miter_limit, set_dash};

impl Interpreter<'_> {
    /// The general and special graphics state operators of Table 50 (ISO 32000-2 §8.4.4): `q`, `Q`,
    /// `cm`, `gs`, `w`, `J`, `j`, `M`, `d`, `ri` and `i`.
    ///
    /// `Q` is where §9.4.2's two text matrices come back with the state they were saved with.
    pub(super) fn run_graphics_state_op(
        &mut self,
        operator: Operator,
        operands: &[Object],
        resources: &Dictionary,
        machine: &mut Machine,
    ) {
        let Machine {
            state,
            stack,
            text_object,
            in_text,
            ..
        } = machine;
        match operator {
            // --- graphics state ---
            Operator::Save => {
                if stack.len() < MAX_STATE_DEPTH {
                    stack.push((state.clone(), text_object.matrix, text_object.line));
                } else {
                    self.note(Unsupported::LimitReached {
                        limit: "MAX_STATE_DEPTH",
                    });
                }
            }
            Operator::Restore => {
                if let Some((previous, matrix, line)) = stack.pop() {
                    if previous.alpha_is_shape != state.alpha_is_shape {
                        self.note_alpha_source(previous.alpha_is_shape);
                    }
                    *state = previous;
                    // The two matrices come back only *inside* a text object, which is
                    // where §9.4.2's addition places them: outside one they are the
                    // parameters of an object that has ended, and Table 105's `BT` sets
                    // both to the identity before the next glyph is shown anyway.
                    if *in_text {
                        text_object.matrix = matrix;
                        text_object.line = line;
                    }
                }
                // An unmatched `Q` is ignored: the alternative is to abandon the page,
                // and files with one extra `Q` render correctly everywhere else.
            }
            Operator::Concatenate => {
                if let Some(matrix) = matrix_from(operands) {
                    state.transform = matrix.then(state.transform);
                }
            }
            Operator::ExtGState => {
                self.apply_ext_gstate(operands, resources, state, *in_text);
            }
            // --- line parameters ---
            Operator::LineWidth => {
                if let Some(width) = number_at(operands, 0) {
                    // ISO 32000-2 §8.4.3.2 gives the parameter its range — the line width
                    // "shall be a nonnegative number expressed in user space units" — and
                    // §8.4.1 states what a processor does with a value outside it, naming
                    // this parameter in the sentence: "[p]arameters that are numeric
                    // values, such as the current colour, line width, and miter limit,
                    // shall be clipped into valid range, if necessary."
                    //
                    // So `-0.1 w` is a `shall`, not a choice: it is clipped to 0, and
                    // §8.4.3.2's rule for 0 then makes the mark one device pixel wide.
                    // That second step is a *device* adjustment, which the same bullet
                    // requires to happen at painting time and never to be stored back
                    // into the graphics state — which is why it is in
                    // `Stroke::device_width` and only the clipped value is kept here.
                    // `content.rs`'s `miter_limit` quotes that same sentence for the
                    // parameter the list names third.
                    state.stroke.width = width.max(0.0);
                }
            }
            Operator::LineCap => {
                if let Some(code) = integer_at(operands, 0) {
                    state.stroke.cap = line_cap(code);
                }
            }
            Operator::LineJoin => {
                if let Some(code) = integer_at(operands, 0) {
                    state.stroke.join = line_join(code);
                }
            }
            Operator::MiterLimit => {
                if let Some(limit) = number_at(operands, 0) {
                    state.stroke.miter_limit = miter_limit(limit);
                }
            }
            Operator::Dash => set_dash(operands, &mut state.stroke),
            // Rendering intent and flatness affect nothing this renderer does.
            Operator::RenderingIntent => {
                // §8.6.5.8's first route to the intent: "Rendering intents shall be
                // specified with the ri operator". What the intent then does to black point
                // compensation is §8.6.5.9's and is asked of the state when an object is
                // painted — setting it here would make the *order* of `ri` and `gs` decide
                // an answer neither clause makes conditional on one.
                if let Some(name) = name_at(operands, 0) {
                    state.intent = Intent::read(name.as_bytes());
                }
            }
            // Only this family's operators are sent here, by `run_operator` — `i` among them, which
            // sets a flatness tolerance the rasteriser decides for itself.
            _ => {}
        }
    }

    /// The path construction operators of Table 50 (ISO 32000-2 §8.5.2): `m`, `l`, `c`, `v`, `y`,
    /// `h` and `re`.
    pub(super) fn run_path_construction_op(
        &mut self,
        operator: Operator,
        operands: &[Object],
        machine: &mut Machine,
    ) {
        let Machine {
            path,
            start,
            current,
            ..
        } = machine;
        match operator {
            Operator::MoveTo => {
                if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                    *current = Point::new(x, y);
                    *start = *current;
                    begin_subpath(path, *current);
                }
            }
            // The four operators §8.5.2.1 says "start at the current point": each is appended
            // through `extend_subpath`, which refuses one that has none and says so. The
            // current point moves only where the segment was taken, because the clause defines
            // it as the endpoint of the segment most recently *added*.
            Operator::LineTo => {
                if let (Some(x), Some(y)) = (number_at(operands, 0), number_at(operands, 1)) {
                    let to = Point::new(x, y);
                    if self.extend_subpath(path, PathCommand::LineTo(to)) {
                        *current = to;
                    }
                }
            }
            Operator::CurveTo => {
                if let Some(points) = points_from::<3>(operands) {
                    let segment = PathCommand::CurveTo(points[0], points[1], points[2]);
                    if self.extend_subpath(path, segment) {
                        *current = points[2];
                    }
                }
            }
            Operator::CurveInitialReplicated => {
                // The first control point is the current point — which is also why an empty
                // path cannot state this curve at all: the operand the clause implies does
                // not exist.
                if let Some(points) = points_from::<2>(operands) {
                    let segment = PathCommand::CurveTo(*current, points[0], points[1]);
                    if self.extend_subpath(path, segment) {
                        *current = points[1];
                    }
                }
            }
            Operator::CurveFinalReplicated => {
                // The second control point is the endpoint.
                if let Some(points) = points_from::<2>(operands) {
                    let segment = PathCommand::CurveTo(points[0], points[1], points[1]);
                    if self.extend_subpath(path, segment) {
                        *current = points[1];
                    }
                }
            }
            Operator::ClosePath => {
                close_subpath(path);
                *current = *start;
            }
            Operator::Rectangle => {
                if let Some(values) = numbers_from::<4>(operands) {
                    let (x, y, w, h) = (values[0], values[1], values[2], values[3]);
                    // Table 58 states `re` as `x y m` and three `l`s and an `h`, so the
                    // `m` it begins with overrides a preceding one exactly as a written
                    // `m` would: 60 paths on `issue12810.pdf`'s first page pair the two.
                    begin_subpath(path, Point::new(x, y));
                    path.push(PathCommand::LineTo(Point::new(x + w, y)));
                    path.push(PathCommand::LineTo(Point::new(x + w, y + h)));
                    path.push(PathCommand::LineTo(Point::new(x, y + h)));
                    path.push(PathCommand::Close);
                    *start = Point::new(x, y);
                    *current = *start;
                }
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// The path painting and clipping path operators of Table 50 (ISO 32000-2 §8.5.3, §8.5.4):
    /// `S`, `s`, `f`, `F`, `f*`, `B`, `B*`, `b`, `b*`, `n`, `W` and `W*`.
    pub(super) fn run_path_painting_op(&mut self, operator: Operator, machine: &mut Machine) {
        let Machine {
            path,
            pending_clip,
            state,
            ..
        } = machine;
        match operator {
            // --- path painting ---
            //
            // Every operator below but `n` keeps the path in the list, so each charges its
            // segments to `MAX_LIST_BYTES` first: a path costs its segments once however many
            // commands share it, and a form invoked a million times builds a million (ADR
            // 1810). `n` keeps none, and a clip is charged where the table takes it.
            Operator::EndPath => {
                self.end_path(path, pending_clip, state, None, None);
            }
            Operator::Fill | Operator::FillObsolete => {
                self.charge_painted(path);
                self.end_path(path, pending_clip, state, Some(FillRule::NonZero), None);
            }
            Operator::FillEvenOdd => {
                self.charge_painted(path);
                self.end_path(path, pending_clip, state, Some(FillRule::EvenOdd), None);
            }
            Operator::Stroke => {
                self.charge_painted(path);
                self.end_path(path, pending_clip, state, None, Some(false));
            }
            Operator::CloseStroke => {
                close_subpath(path);
                self.charge_painted(path);
                self.end_path(path, pending_clip, state, None, Some(true));
            }
            Operator::FillStroke => {
                self.charge_painted(path);
                self.end_path(
                    path,
                    pending_clip,
                    state,
                    Some(FillRule::NonZero),
                    Some(false),
                );
            }
            Operator::FillStrokeEvenOdd => {
                self.charge_painted(path);
                self.end_path(
                    path,
                    pending_clip,
                    state,
                    Some(FillRule::EvenOdd),
                    Some(false),
                );
            }
            Operator::CloseFillStroke | Operator::CloseFillStrokeEvenOdd => {
                close_subpath(path);
                let rule = if operator == Operator::CloseFillStrokeEvenOdd {
                    FillRule::EvenOdd
                } else {
                    FillRule::NonZero
                };
                self.charge_painted(path);
                self.end_path(path, pending_clip, state, Some(rule), Some(true));
            }
            Operator::Clip => *pending_clip = Some(FillRule::NonZero),
            Operator::ClipEvenOdd => *pending_clip = Some(FillRule::EvenOdd),
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// The colour operators of Table 50 (ISO 32000-2 §8.6.8): `CS`, `cs`, `SC`, `SCN`, `sc`, `scn`,
    /// `G`, `g`, `RG`, `rg`, `K` and `k`.
    pub(super) fn run_colour_op(
        &mut self,
        operator: Operator,
        operands: &[Object],
        resources: &Dictionary,
        machine: &mut Machine,
    ) {
        let Machine { state, .. } = machine;
        match operator {
            // --- colour ---
            // `g`, `rg` and `k` set a device space and a colour together — or the
            // matching `Default` space, where the resources name one, which is why
            // these resolve the space rather than naming it directly.
            Operator::FillGray | Operator::StrokeGray => {
                if let Some(grey) = number_at(operands, 0) {
                    let space = self.device_space("DeviceGray", resources);
                    let colour = self.colour(&space, &[grey], state);
                    self.assign_colour(
                        state,
                        operator == Operator::FillGray,
                        colour,
                        space,
                        &[grey],
                    );
                }
            }
            Operator::FillRgb | Operator::StrokeRgb => {
                if let Some(values) = numbers_from::<3>(operands) {
                    let space = self.device_space("DeviceRGB", resources);
                    let colour = self.colour(&space, &values, state);
                    self.assign_colour(
                        state,
                        operator == Operator::FillRgb,
                        colour,
                        space,
                        &values,
                    );
                }
            }
            Operator::FillCmyk | Operator::StrokeCmyk => {
                if let Some(values) = numbers_from::<4>(operands) {
                    let space = self.device_space("DeviceCMYK", resources);
                    let colour = self.colour(&space, &values, state);
                    self.assign_colour(
                        state,
                        operator == Operator::FillCmyk,
                        colour,
                        space,
                        &values,
                    );
                }
            }
            Operator::FillColourSpace | Operator::StrokeColourSpace => {
                let fill = operator == Operator::FillColourSpace;
                self.set_colour_space(operands, resources, state, fill);
            }
            Operator::FillColour
            | Operator::FillColourN
            | Operator::StrokeColour
            | Operator::StrokeColourN => {
                let fill = matches!(operator, Operator::FillColour | Operator::FillColourN);
                self.set_colour(operands, resources, state, fill);
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }
}
