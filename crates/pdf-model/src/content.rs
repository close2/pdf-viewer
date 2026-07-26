//! Interpreting a content stream into a resolved display list.
//!
//! This is where PDF's graphics state machine is executed, and executed *once*. Every
//! command that comes out carries its absolute transform and an explicit clip, so the
//! backends contain no PDF semantics at all — which is what lets the CPU backend serve as
//! an oracle for the GPU one. See `pdf-render`.
//!
//! # Unsupported content is reported, never silently dropped
//!
//! Text and images are not yet drawn: text needs the font machinery and images need the
//! codecs. Ignoring them silently would produce a page that looks plausible and is wrong,
//! which is the single most dangerous failure mode for a viewer — and it would make the
//! comparison harness report a pass on a page missing half its content.
//!
//! So [`Interpretation`] carries a list of what it could not draw. A caller can render the
//! partial page *and* know it is partial: the viewer can say so, and the harness can
//! exclude the page from comparison rather than reporting a false difference.

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{
    BlendMode, ClipId, Color, Command, DisplayList, FillRule, LineCap, LineJoin, Paint, Path,
    PathCommand, Point, Size, Stroke, Transform,
};
use pdf_syntax::{Dictionary, Document, Object};

use crate::page::Page;

/// Deepest nesting of `q`/`Q` that will be tracked.
///
/// Legitimate content nests a few levels. A stream with thousands of unmatched `q`
/// operators is either broken or hostile, and each level costs a saved state.
const MAX_STATE_DEPTH: usize = 256;

/// Most operators executed for one page.
///
/// A content stream is a program, and this bounds how long it may run. Without it a
/// compressed stream of a few kilobytes can expand into tens of millions of operations —
/// a decompression bomb aimed at the renderer rather than at memory.
const MAX_OPERATIONS: usize = 4_000_000;

/// Most operands one operator may take before the rest are refused.
///
/// Every operator in the specification takes at most six operands except `TJ` and `d`,
/// which take arrays. A `TJ` array holds one entry per text run and one per kerning
/// adjustment between them, so a single justified line of text routinely runs to several
/// hundred entries — a bound of 64 silently cut real sentences in half. This is set well
/// above any legitimate line while still bounding what one operator can allocate.
const MAX_OPERANDS: usize = 8192;

/// Deepest nesting of form `XObject`s.
///
/// A form may draw another form, and a form that draws itself is a cycle. The
/// specification forbids it; files do it anyway.
const MAX_FORM_DEPTH: usize = 16;

/// Something the interpreter met but could not draw.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unsupported {
    /// Text-showing operators were present.
    Text {
        /// How many show operations were skipped.
        operations: usize,
    },
    /// An image `XObject` was drawn.
    Image {
        /// The resource name, for diagnosis.
        name: String,
    },
    /// A shading or pattern was used as paint.
    Shading {
        /// The resource name.
        name: String,
    },
    /// An operator this interpreter does not implement.
    Operator {
        /// The operator, as written.
        operator: String,
    },
    /// A font could not be loaded, so its text was not drawn.
    Font {
        /// Why, from `pdf-font`.
        detail: String,
    },
    /// A bound was reached and interpretation stopped early.
    LimitReached {
        /// Which bound.
        limit: &'static str,
    },
}

/// The result of interpreting a page.
#[derive(Debug, Clone)]
pub struct Interpretation {
    /// The drawing commands, ready for any backend.
    pub display_list: DisplayList,
    /// What could not be drawn. Empty means the page is complete.
    pub unsupported: Vec<Unsupported>,
    /// The page's text, in the order the content stream showed it.
    ///
    /// Produced by the same pass that draws the glyphs, and from the same code-to-glyph
    /// decisions, which is what makes it worth comparing against another extractor: a
    /// difference is evidence about the *rendering*, not about a separate text pipeline
    /// that might be wrong in its own way.
    ///
    /// This is reading order as the producer wrote it, which is not always visual order.
    /// It carries no layout analysis and does not try to reconstruct columns.
    pub text: String,
}

impl Interpretation {
    /// Returns `true` if everything on the page was drawn.
    ///
    /// The harness uses this to decide whether a page may be compared against a reference
    /// renderer at all: comparing a page we knowingly rendered incompletely would report a
    /// difference that is not a defect.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unsupported.is_empty()
    }
}

/// One level of PDF graphics state.
#[derive(Debug, Clone)]
struct GraphicsState {
    transform: Transform,
    clip: Option<ClipId>,
    fill: Color,
    stroke_colour: Color,
    stroke: Stroke,
    blend: BlendMode,
    fill_alpha: f32,
    stroke_alpha: f32,
    /// How many components the current fill colour space expects, used to read `sc`/`scn`.
    fill_components: usize,
    /// As above, for stroking.
    stroke_components: usize,
    /// Text state, which `q`/`Q` saves and restores along with everything else.
    text: TextState,
}

/// The text-related part of the graphics state.
///
/// Separate from the text *object* state (`Tm` and `Tlm`), which the specification resets
/// at every `BT` and which therefore does not survive `q`/`Q`.
#[derive(Debug, Clone)]
struct TextState {
    /// The resource name of the current font, and the font itself once loaded.
    font: Option<Rc<pdf_font::LoadedFont>>,
    /// Font size, in unscaled text-space units.
    size: f32,
    /// Character spacing, added to every glyph's advance.
    char_spacing: f32,
    /// Word spacing, added to the advance of a single-byte code 32.
    word_spacing: f32,
    /// Horizontal scaling, as a factor rather than the percentage the operator takes.
    horizontal_scale: f32,
    /// Leading, the vertical distance `T*` moves.
    leading: f32,
    /// Rise, which lifts the baseline for superscripts.
    rise: f32,
    /// Rendering mode: whether glyphs are filled, stroked, both, or invisible.
    render_mode: i64,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            font: None,
            size: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scale: 1.0,
            leading: 0.0,
            rise: 0.0,
            render_mode: 0,
        }
    }
}

impl GraphicsState {
    /// The initial state defined by ISO 32000-2 8.4.
    fn initial(base: Transform) -> Self {
        Self {
            transform: base,
            clip: None,
            fill: Color::BLACK,
            stroke_colour: Color::BLACK,
            stroke: Stroke::default(),
            blend: BlendMode::Normal,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            fill_components: 1,
            stroke_components: 1,
            text: TextState::default(),
        }
    }

    /// Returns the fill colour with the constant alpha applied.
    fn fill_paint(&self) -> Paint {
        Paint::Solid(Color {
            a: self.fill.a * self.fill_alpha,
            ..self.fill
        })
    }

    /// Returns the stroke colour with the constant alpha applied.
    fn stroke_paint(&self) -> Paint {
        Paint::Solid(Color {
            a: self.stroke_colour.a * self.stroke_alpha,
            ..self.stroke_colour
        })
    }
}

/// Interprets a page's content into a display list.
///
/// The returned list is in PDF user space with the page's crop box at the origin, so a
/// backend applies only the device transform. Page rotation is folded in here, because it
/// is a property of the page rather than of the device.
#[must_use]
pub fn interpret(document: &Document, page: &Page) -> Interpretation {
    let content = page.content(document);
    let size = rotated_size(page);

    let mut interpreter = Interpreter {
        document,
        list: DisplayList::new(size),
        unsupported: BTreeMap::new(),
        text_operations: 0,
        operations: 0,
        fonts: BTreeMap::new(),
        text: String::new(),
        text_cursor: None,
    };

    let base = base_transform(page);
    interpreter.run(&content, &page.resources, &GraphicsState::initial(base), 0);

    let mut unsupported: Vec<Unsupported> = interpreter.unsupported.into_values().collect();
    if interpreter.text_operations > 0 {
        unsupported.push(Unsupported::Text {
            operations: interpreter.text_operations,
        });
    }
    unsupported.sort_unstable();

    Interpretation {
        display_list: interpreter.list,
        unsupported,
        text: interpreter.text,
    }
}

/// Returns the page size after rotation, since a rotated page swaps its extents.
fn rotated_size(page: &Page) -> Size {
    let (width, height) = (page.width(), page.height());
    if page.rotate == 90 || page.rotate == 270 {
        Size::new(height, width)
    } else {
        Size::new(width, height)
    }
}

/// Builds the transform from PDF user space to the page's own space.
///
/// Two things fold in here. The crop box may not start at the origin, so content is
/// translated by its lower-left corner; and `/Rotate` turns the page clockwise, which is
/// a rotation plus a translation to bring the result back into positive coordinates.
fn base_transform(page: &Page) -> Transform {
    let shift = Transform::translate(-page.crop_box[0], -page.crop_box[1]);
    let (width, height) = (page.width(), page.height());

    let rotation = match page.rotate {
        // Clockwise in device terms; in PDF's y-up space that is this matrix.
        90 => Transform::new(0.0, 1.0, -1.0, 0.0, height, 0.0),
        180 => Transform::new(-1.0, 0.0, 0.0, -1.0, width, height),
        270 => Transform::new(0.0, -1.0, 1.0, 0.0, 0.0, width),
        // 0, and anything that normalised to it.
        _ => Transform::IDENTITY,
    };

    shift.then(rotation)
}

/// Interpreter state for one page.
struct Interpreter<'a> {
    document: &'a Document,
    list: DisplayList,
    /// Keyed so that a page drawing the same unsupported image a thousand times reports it
    /// once rather than flooding the diagnostics.
    unsupported: BTreeMap<Unsupported, Unsupported>,
    text_operations: usize,
    operations: usize,
    /// Fonts already loaded, keyed by resource name.
    ///
    /// A page names the same font on every `Tf`, and parsing a font program is expensive,
    /// so this is what keeps text rendering from being dominated by font loading.
    fonts: BTreeMap<String, Option<Rc<pdf_font::LoadedFont>>>,
    /// The page's text, accumulated as the glyphs are placed.
    text: String,
    /// Where the last glyph ended, used to decide where a space belongs.
    text_cursor: Option<(f32, f32)>,
}

impl Interpreter<'_> {
    fn note(&mut self, item: Unsupported) {
        self.unsupported.insert(item.clone(), item);
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
    #[expect(
        clippy::too_many_lines,
        reason = "a bytecode dispatch table reads better whole than split; see above"
    )]
    fn run(
        &mut self,
        content: &[u8],
        resources: &Dictionary,
        initial: &GraphicsState,
        form_depth: usize,
    ) {
        let mut lexer = pdf_syntax::Lexer::new(content);
        let mut operands: Vec<Object> = Vec::new();
        let mut state = initial.clone();
        let mut stack: Vec<GraphicsState> = Vec::new();

        // The path being built, and the pending clip requested by `W`/`W*`.
        let mut path = Path::new();
        let mut start = Point::new(0.0, 0.0);
        let mut current = Point::new(0.0, 0.0);
        let mut pending_clip: Option<FillRule> = None;
        let mut in_text = false;
        // The text object's own matrices, which `BT` resets and `q`/`Q` do not touch.
        let mut text_matrix = Transform::IDENTITY;
        let mut line_matrix = Transform::IDENTITY;

        while let Some(token) = lexer.next_token() {
            self.operations = self.operations.saturating_add(1);
            if self.operations > MAX_OPERATIONS {
                self.note(Unsupported::LimitReached {
                    limit: "MAX_OPERATIONS",
                });
                return;
            }

            // Operands accumulate until an operator consumes them.
            let operator = match token {
                pdf_syntax::Token::Keyword(word) => word,
                other => {
                    if operands.len() < MAX_OPERANDS {
                        operands.push(token_to_object(other));
                    } else {
                        // Dropping operands silently truncates the page: a `TJ` array is
                        // one operand per run *and* per kerning adjustment, so a single
                        // justified line can be hundreds, and the text simply stopped
                        // mid-sentence with nothing reported. The bound stays, because a
                        // hostile stream can otherwise make one operator allocate without
                        // limit — but reaching it is now a reported defect.
                        self.note(Unsupported::LimitReached {
                            limit: "MAX_OPERANDS",
                        });
                    }
                    continue;
                }
            };

            match operator.as_slice() {
                // --- graphics state ---
                b"q" => {
                    if stack.len() < MAX_STATE_DEPTH {
                        stack.push(state.clone());
                    } else {
                        self.note(Unsupported::LimitReached {
                            limit: "MAX_STATE_DEPTH",
                        });
                    }
                }
                b"Q" => {
                    if let Some(previous) = stack.pop() {
                        state = previous;
                    }
                    // An unmatched `Q` is ignored: the alternative is to abandon the page,
                    // and files with one extra `Q` render correctly everywhere else.
                }
                b"cm" => {
                    if let Some(matrix) = matrix_from(&operands) {
                        state.transform = matrix.then(state.transform);
                    }
                }
                b"gs" => self.apply_ext_gstate(&operands, resources, &mut state),

                // --- line parameters ---
                b"w" => {
                    if let Some(width) = number_at(&operands, 0) {
                        state.stroke.width = width.max(0.0);
                    }
                }
                b"J" => {
                    if let Some(code) = integer_at(&operands, 0) {
                        state.stroke.cap = match code {
                            1 => LineCap::Round,
                            2 => LineCap::Square,
                            // The specification defines 0, 1 and 2; anything else is
                            // malformed and butt caps are the initial value.
                            _ => LineCap::Butt,
                        };
                    }
                }
                b"j" => {
                    if let Some(code) = integer_at(&operands, 0) {
                        state.stroke.join = match code {
                            1 => LineJoin::Round,
                            2 => LineJoin::Bevel,
                            _ => LineJoin::Miter,
                        };
                    }
                }
                b"M" => {
                    if let Some(limit) = number_at(&operands, 0) {
                        state.stroke.miter_limit = limit.max(1.0);
                    }
                }
                b"d" => set_dash(&operands, &mut state.stroke),
                // Rendering intent and flatness affect nothing this renderer does.
                // --- path construction ---
                b"m" => {
                    if let (Some(x), Some(y)) = (number_at(&operands, 0), number_at(&operands, 1)) {
                        current = Point::new(x, y);
                        start = current;
                        path.push(PathCommand::MoveTo(current));
                    }
                }
                b"l" => {
                    if let (Some(x), Some(y)) = (number_at(&operands, 0), number_at(&operands, 1)) {
                        current = Point::new(x, y);
                        path.push(PathCommand::LineTo(current));
                    }
                }
                b"c" => {
                    if let Some(points) = points_from(&operands, 3) {
                        path.push(PathCommand::CurveTo(points[0], points[1], points[2]));
                        current = points[2];
                    }
                }
                b"v" => {
                    // The first control point is the current point.
                    if let Some(points) = points_from(&operands, 2) {
                        path.push(PathCommand::CurveTo(current, points[0], points[1]));
                        current = points[1];
                    }
                }
                b"y" => {
                    // The second control point is the endpoint.
                    if let Some(points) = points_from(&operands, 2) {
                        path.push(PathCommand::CurveTo(points[0], points[1], points[1]));
                        current = points[1];
                    }
                }
                b"h" => {
                    path.push(PathCommand::Close);
                    current = start;
                }
                b"re" => {
                    if let Some(values) = numbers_from(&operands, 4) {
                        let (x, y, w, h) = (values[0], values[1], values[2], values[3]);
                        path.push(PathCommand::MoveTo(Point::new(x, y)));
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
                    path.push(PathCommand::Close);
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
                    path.push(PathCommand::Close);
                    let rule = if operator.as_slice() == b"b*" {
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
                b"g" | b"G" => {
                    if let Some(grey) = number_at(&operands, 0) {
                        let colour = Color::rgb(grey, grey, grey);
                        assign_colour(&mut state, operator.as_slice() == b"g", colour, 1);
                    }
                }
                b"rg" | b"RG" => {
                    if let Some(values) = numbers_from(&operands, 3) {
                        let colour = Color::rgb(values[0], values[1], values[2]);
                        assign_colour(&mut state, operator.as_slice() == b"rg", colour, 3);
                    }
                }
                b"k" | b"K" => {
                    if let Some(values) = numbers_from(&operands, 4) {
                        let colour = cmyk(values[0], values[1], values[2], values[3]);
                        assign_colour(&mut state, operator.as_slice() == b"k", colour, 4);
                    }
                }
                b"cs" | b"CS" => {
                    let fill = operator.as_slice() == b"cs";
                    self.set_colour_space(&operands, resources, &mut state, fill);
                }
                b"sc" | b"scn" | b"SC" | b"SCN" => {
                    let fill = matches!(operator.as_slice(), b"sc" | b"scn");
                    self.set_colour(&operands, &mut state, fill);
                }

                // --- text ---
                b"BT" => {
                    in_text = true;
                    // Both matrices reset at the start of every text object.
                    text_matrix = Transform::IDENTITY;
                    line_matrix = Transform::IDENTITY;
                }
                b"ET" => in_text = false,
                b"Tf" => {
                    if let Some(name) = name_at(&operands, 0) {
                        state.text.font = self.font(resources, &name);
                    }
                    if let Some(size) = number_at(&operands, 1) {
                        state.text.size = size;
                    }
                }
                b"Tc" => {
                    if let Some(value) = number_at(&operands, 0) {
                        state.text.char_spacing = value;
                    }
                }
                b"Tw" => {
                    if let Some(value) = number_at(&operands, 0) {
                        state.text.word_spacing = value;
                    }
                }
                b"Tz" => {
                    if let Some(percent) = number_at(&operands, 0) {
                        state.text.horizontal_scale = percent / 100.0;
                    }
                }
                b"TL" => {
                    if let Some(value) = number_at(&operands, 0) {
                        state.text.leading = value;
                    }
                }
                b"Ts" => {
                    if let Some(value) = number_at(&operands, 0) {
                        state.text.rise = value;
                    }
                }
                b"Tr" => {
                    if let Some(mode) = integer_at(&operands, 0) {
                        state.text.render_mode = mode;
                    }
                }
                b"Td" => {
                    if let (Some(x), Some(y)) = (number_at(&operands, 0), number_at(&operands, 1)) {
                        line_matrix = Transform::translate(x, y).then(line_matrix);
                        text_matrix = line_matrix;
                    }
                }
                b"TD" => {
                    if let (Some(x), Some(y)) = (number_at(&operands, 0), number_at(&operands, 1)) {
                        // `TD` is `Td` with the side effect of setting the leading.
                        state.text.leading = -y;
                        line_matrix = Transform::translate(x, y).then(line_matrix);
                        text_matrix = line_matrix;
                    }
                }
                b"Tm" => {
                    if let Some(matrix) = matrix_from(&operands) {
                        line_matrix = matrix;
                        text_matrix = matrix;
                    }
                }
                b"T*" => {
                    line_matrix = Transform::translate(0.0, -state.text.leading).then(line_matrix);
                    text_matrix = line_matrix;
                }
                b"Tj" => {
                    if let Some(bytes) = string_at(&operands, 0) {
                        self.show_text(&bytes, &state, &mut text_matrix);
                    }
                }
                b"TJ" => {
                    // The array operand is not reconstructed by the content lexer, so the
                    // strings and the numeric adjustments between them arrive as separate
                    // operands in order — which is enough to render them correctly.
                    for operand in &operands {
                        match operand {
                            Object::String(bytes) => {
                                self.show_text(bytes, &state, &mut text_matrix);
                            }
                            other => {
                                if let Some(adjust) = other.as_number() {
                                    // A positive adjustment moves *left*: it is subtracted,
                                    // in thousandths of an em, scaled by size and horizontal
                                    // scaling.
                                    let shift = -narrow(adjust) / 1000.0
                                        * state.text.size
                                        * state.text.horizontal_scale;
                                    text_matrix =
                                        Transform::translate(shift, 0.0).then(text_matrix);
                                }
                            }
                        }
                    }
                }
                b"'" => {
                    line_matrix = Transform::translate(0.0, -state.text.leading).then(line_matrix);
                    text_matrix = line_matrix;
                    if let Some(bytes) = string_at(&operands, 0) {
                        self.show_text(&bytes, &state, &mut text_matrix);
                    }
                }
                b"\"" => {
                    // `aw ac string "` sets word and character spacing, then shows.
                    if let Some(word) = number_at(&operands, 0) {
                        state.text.word_spacing = word;
                    }
                    if let Some(character) = number_at(&operands, 1) {
                        state.text.char_spacing = character;
                    }
                    line_matrix = Transform::translate(0.0, -state.text.leading).then(line_matrix);
                    text_matrix = line_matrix;
                    if let Some(bytes) = string_at(&operands, 2) {
                        self.show_text(&bytes, &state, &mut text_matrix);
                    }
                }

                // --- XObjects ---
                b"Do" => self.draw_xobject(&operands, resources, &state, form_depth),

                // --- shadings and inline images ---
                b"sh" => {
                    let name = name_at(&operands, 0).unwrap_or_default();
                    self.note(Unsupported::Shading { name });
                }
                b"BI" => {
                    // An inline image runs to `EI` and its data is not PDF syntax, so the
                    // lexer must be skipped past it or it will tokenise binary as operators.
                    self.note(Unsupported::Image {
                        name: "<inline>".to_owned(),
                    });
                    skip_inline_image(&mut lexer);
                }

                // Operators that affect no geometry this renderer produces: marked
                // content and compatibility sections carry structure rather than drawing;
                // rendering intent needs colour management; and flatness tolerance is a
                // hint about curve subdivision that the rasteriser decides for itself.
                b"BMC" | b"BDC" | b"EMC" | b"MP" | b"DP" | b"BX" | b"EX" | b"ri" | b"i" => {}

                other => {
                    self.note(Unsupported::Operator {
                        operator: String::from_utf8_lossy(other).into_owned(),
                    });
                }
            }

            operands.clear();
        }

        // An unclosed `BT` is malformed but harmless here; noted so it is not invisible.
        if in_text {
            self.note(Unsupported::Operator {
                operator: "BT without ET".to_owned(),
            });
        }
    }

    /// Emits the drawing for a completed path and resets it.
    ///
    /// `fill` and `stroke` say what to paint; `close_before_stroke` is already applied by
    /// the caller. A pending `W` takes effect here, which is what the specification
    /// requires: the clip changes *after* the current path is painted.
    fn end_path(
        &mut self,
        path: &mut Path,
        pending_clip: &mut Option<FillRule>,
        state: &mut GraphicsState,
        fill: Option<FillRule>,
        stroke: Option<bool>,
    ) {
        if !path.is_empty() && (fill.is_some() || stroke.is_some()) {
            // `B` fills *and* strokes one path, and both commands then describe the same
            // geometry; sharing it means the copy happens once rather than twice.
            let shared = Arc::new(path.clone());
            if let Some(rule) = fill {
                self.list.push(Command::Fill {
                    path: Arc::clone(&shared),
                    transform: state.transform,
                    fill_rule: rule,
                    paint: state.fill_paint(),
                    clip: state.clip,
                    blend: state.blend,
                });
            }
            if stroke.is_some() {
                self.list.push(Command::Stroke {
                    path: Arc::clone(&shared),
                    transform: state.transform,
                    stroke: state.stroke.clone(),
                    paint: state.stroke_paint(),
                    clip: state.clip,
                    blend: state.blend,
                });
            }
        }

        // A pending `W` takes effect now: the specification says the clip changes *after*
        // the current path is painted, so the fill and stroke above used the old clip and
        // everything following uses the new one. The new clip becomes a child of the
        // current one, since clipping intersects rather than replaces.
        if let Some(rule) = pending_clip.take()
            && !path.is_empty()
        {
            let clip = Clip {
                path: path.clone(),
                transform: state.transform,
                fill_rule: rule,
                parent: state.clip,
            };
            match self.list.add_clip(clip) {
                Ok(id) => state.clip = Some(id),
                Err(_) => self.note(Unsupported::LimitReached { limit: "max_clips" }),
            }
        }

        *path = Path::new();
    }
}

/// Applies an `/ExtGState` resource.
impl Interpreter<'_> {
    fn apply_ext_gstate(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &mut GraphicsState,
    ) {
        let Some(name) = name_at(operands, 0) else {
            return;
        };
        let Some(dict) = self.resource(resources, "ExtGState", &name) else {
            return;
        };
        let Some(dict) = dict.as_dict() else { return };

        if let Some(alpha) = self.document.get_key(dict, "ca").as_number() {
            state.fill_alpha = clamp_unit(alpha);
        }
        if let Some(alpha) = self.document.get_key(dict, "CA").as_number() {
            state.stroke_alpha = clamp_unit(alpha);
        }
        if let Some(width) = self.document.get_key(dict, "LW").as_number() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a line width outside f32's range is not a line width"
            )]
            {
                state.stroke.width = (width as f32).max(0.0);
            }
        }
        match self.document.get_key(dict, "BM") {
            Object::Name(name) => state.blend = blend_mode(name.as_bytes()),
            Object::Array(items) => {
                // An array offers alternatives in preference order; take the first known.
                if let Some(name) = items
                    .iter()
                    .map(|item| self.document.resolve(item))
                    .find_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                {
                    state.blend = blend_mode(&name);
                }
            }
            _ => {}
        }

        // A soft mask other than /None changes compositing in a way this renderer cannot
        // yet reproduce, so it must be reported rather than ignored.
        if let Object::Dictionary(_) = self.document.get_key(dict, "SMask") {
            self.note(Unsupported::Shading {
                name: format!("SMask in /{name}"),
            });
        }
    }

    /// Sets a colour space, recording how many components its colours will have.
    fn set_colour_space(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &mut GraphicsState,
        fill: bool,
    ) {
        let Some(name) = name_at(operands, 0) else {
            return;
        };

        let components = match name.as_str() {
            "DeviceGray" | "G" | "CalGray" => 1,
            "DeviceRGB" | "RGB" | "CalRGB" | "Lab" => 3,
            "DeviceCMYK" | "CMYK" => 4,
            "Pattern" => {
                self.note(Unsupported::Shading {
                    name: "Pattern".to_owned(),
                });
                1
            }
            _ => self.resolved_space_components(resources, &name),
        };

        // Setting a colour space resets the colour to its initial value: black for the
        // device spaces. Omitting this leaves the previous space's colour in place, which
        // shows up as content painted in the wrong colour.
        let colour = Color::BLACK;
        if fill {
            state.fill_components = components;
            state.fill = colour;
        } else {
            state.stroke_components = components;
            state.stroke_colour = colour;
        }
    }

    /// Determines the component count of a named colour space resource.
    fn resolved_space_components(&mut self, resources: &Dictionary, name: &str) -> usize {
        let Some(space) = self.resource(resources, "ColorSpace", name) else {
            return 1;
        };

        match &space {
            Object::Name(family) => match family.as_bytes() {
                b"DeviceRGB" | b"CalRGB" => 3,
                b"DeviceCMYK" => 4,
                _ => 1,
            },
            Object::Array(items) => {
                let family = items
                    .first()
                    .map(|item| self.document.resolve(item))
                    .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                    .unwrap_or_default();
                match family.as_slice() {
                    b"CalRGB" | b"Lab" => 3,
                    b"ICCBased" => self.icc_components(items),
                    // One component each: a grey level, an index into a lookup table, or a
                    // single ink tint.
                    b"CalGray" | b"Indexed" | b"I" | b"Separation" => 1,
                    b"DeviceN" => items
                        .get(1)
                        .map(|item| self.document.resolve(item))
                        .and_then(|item| item.as_array().map(<[Object]>::len))
                        .unwrap_or(1),
                    _ => {
                        self.note(Unsupported::Shading {
                            name: format!(
                                "colour space {}",
                                String::from_utf8_lossy(family.as_slice())
                            ),
                        });
                        1
                    }
                }
            }
            _ => 1,
        }
    }

    /// Reads `/N` from an `ICCBased` stream, which gives its component count.
    fn icc_components(&self, items: &[Object]) -> usize {
        items
            .get(1)
            .map(|item| self.document.resolve(item))
            .and_then(|item| item.as_dict().cloned())
            .and_then(|dict| self.document.get_key(&dict, "N").as_integer())
            .and_then(|value| usize::try_from(value).ok())
            .filter(|count| matches!(count, 1 | 3 | 4))
            .unwrap_or(3)
    }

    /// Sets a colour from `sc`/`scn` operands, interpreting them by component count.
    fn set_colour(&mut self, operands: &[Object], state: &mut GraphicsState, fill: bool) {
        // A trailing name means a pattern rather than a colour.
        if operands.iter().any(|operand| operand.as_name().is_some()) {
            let name = operands
                .iter()
                .filter_map(|operand| operand.as_name())
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                .next()
                .unwrap_or_default();
            self.note(Unsupported::Shading {
                name: format!("pattern /{name}"),
            });
            return;
        }

        let expected = if fill {
            state.fill_components
        } else {
            state.stroke_components
        };
        let values: Vec<f32> = (0..operands.len())
            .filter_map(|index| number_at(operands, index))
            .collect();

        // Take the operand count as authoritative where it disagrees with the declared
        // space: producers get `/CS` wrong more often than they get the operand count wrong.
        let colour = match values.len() {
            1 => Color::rgb(values[0], values[0], values[0]),
            3 => Color::rgb(values[0], values[1], values[2]),
            4 => cmyk(values[0], values[1], values[2], values[3]),
            _ => {
                self.note(Unsupported::Shading {
                    name: format!("{} colour components (expected {expected})", values.len()),
                });
                return;
            }
        };

        if fill {
            state.fill = colour;
        } else {
            state.stroke_colour = colour;
        }
    }

    /// Draws an `XObject`: a form is interpreted inline, an image is reported.
    fn draw_xobject(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &GraphicsState,
        form_depth: usize,
    ) {
        let Some(name) = name_at(operands, 0) else {
            return;
        };
        let Some(object) = self.resource(resources, "XObject", &name) else {
            return;
        };
        let Some(stream) = object.as_stream().cloned() else {
            return;
        };

        let subtype = self.document.get_key(&stream.dict, "Subtype");
        let subtype = subtype
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();

        if subtype == b"Image" {
            // A PDF image occupies the unit square in user space, so the command's
            // transform is the current transform and nothing else.
            match crate::image::decode(self.document, &stream, state.fill) {
                Ok(image) => self.list.push(Command::Image {
                    image,
                    transform: state.transform,
                    alpha: state.fill_alpha,
                    clip: state.clip,
                    blend: state.blend,
                }),
                Err(error) => self.note(Unsupported::Image {
                    name: format!("{name}: {error}"),
                }),
            }
            return;
        }
        if subtype != b"Form" {
            self.note(Unsupported::Operator {
                operator: format!("Do on /{name}"),
            });
            return;
        }

        if form_depth >= MAX_FORM_DEPTH {
            self.note(Unsupported::LimitReached {
                limit: "MAX_FORM_DEPTH",
            });
            return;
        }

        let Some(data) = self.document.decoded_stream_data(&stream) else {
            self.note(Unsupported::Operator {
                operator: format!("undecodable form /{name}"),
            });
            return;
        };

        // A form carries its own matrix and its own resources, falling back to the page's.
        let mut inner = state.clone();
        if let Some(matrix) = self
            .document
            .get_key(&stream.dict, "Matrix")
            .as_array()
            .and_then(|items| {
                let values: Vec<f32> = items
                    .iter()
                    .map(|item| self.document.resolve(item))
                    .filter_map(|item| item.as_number())
                    .map(|value| {
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "a matrix component outside f32's range is not usable \
                                      as one"
                        )]
                        {
                            value as f32
                        }
                    })
                    .collect();
                (values.len() >= 6).then(|| {
                    Transform::new(
                        values[0], values[1], values[2], values[3], values[4], values[5],
                    )
                })
            })
        {
            inner.transform = matrix.then(inner.transform);
        }

        let form_resources = self
            .document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_else(|| resources.clone());

        self.run(&data, &form_resources, &inner, form_depth.saturating_add(1));
    }

    /// Loads a font by resource name, caching the result including failures.
    ///
    /// A failure is cached too: a page that names an unloadable font on every `Tf` should
    /// pay for the attempt once, and should report it once.
    fn font(&mut self, resources: &Dictionary, name: &str) -> Option<Rc<pdf_font::LoadedFont>> {
        if let Some(cached) = self.fonts.get(name) {
            return cached.clone();
        }

        let loaded = self
            .resource(resources, "Font", name)
            .and_then(|object| object.as_dict().cloned())
            .map(|dict| pdf_font::LoadedFont::load(self.document, &dict, name));

        let result = match loaded {
            Some(Ok(font)) => Some(Rc::new(font)),
            Some(Err(error)) => {
                self.note(Unsupported::Font {
                    detail: error.to_string(),
                });
                None
            }
            None => {
                self.note(Unsupported::Font {
                    detail: format!("no /Font resource named /{name}"),
                });
                None
            }
        };

        self.fonts.insert(name.to_owned(), result.clone());
        result
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
    fn show_text(&mut self, bytes: &[u8], state: &GraphicsState, text_matrix: &mut Transform) {
        let Some(font) = state.text.font.clone() else {
            self.text_operations = self.text_operations.saturating_add(1);
            return;
        };

        // Mode 3 is invisible text, and mode 7 adds to the clip without painting. Both are
        // used for the OCR layer under a scanned image, where drawing them would be wrong.
        let invisible = matches!(state.text.render_mode, 3 | 7);
        let size = state.text.size;
        let scale = state.text.horizontal_scale;

        // How wide a gap has to be before it means a word break rather than kerning.
        //
        // Measured against the font's own space, because that is what a word break is made
        // of. A fixed fraction of the font size cannot work: a title set with loose
        // tracking moves each glyph further than a body-text space, and judging it by size
        // alone spells "Clarification" as "Clar if ic at ion".
        let space_em = font.advance(32);
        let word_gap = if space_em > 0.0 {
            space_em * size * 0.6
        } else {
            size * 0.25
        };

        for code in font.decode(bytes) {
            let advance_em = font.advance(code);

            // A content stream has no notion of words or lines; it has positions. A glyph
            // placed left of, or well below, where the last one ended began a new line,
            // and one placed a noticeable gap to the right of it began a new word. These
            // are the only two separators reconstructed, because anything more is layout
            // analysis and belongs to a consumer of this text rather than to the drawing
            // pass. `pdftotext` does do that analysis, which is why the comparison
            // normalises whitespace away.
            // The text-space origin under the matrix is simply its translation.
            let here = (text_matrix.e, text_matrix.f);
            if let Some((last_x, last_y)) = self.text_cursor {
                let gap = here.0 - last_x;
                if (here.1 - last_y).abs() > size * 0.5 {
                    self.text.push('\n');
                } else if gap > word_gap {
                    self.text.push(' ');
                }
            }
            font.text(code, &mut self.text);

            if !invisible
                && size != 0.0
                && let Some(outline) = font.outline(code)
            {
                {
                    // Glyph space to text space: scale by the font size, apply horizontal
                    // scaling and rise, then the text matrix and the current transform.
                    let glyph_to_text =
                        Transform::new(size * scale, 0.0, 0.0, size, 0.0, state.text.rise);
                    let transform = glyph_to_text.then(*text_matrix).then(state.transform);

                    self.list.push(Command::Fill {
                        // The font hands out shared outlines and the display list keeps
                        // them shared: a page of text is the same few dozen glyphs over
                        // and over, so this is a refcount rather than a copy of the
                        // segments.
                        path: Arc::clone(&outline),
                        transform,
                        // Glyph outlines are non-zero filled; even-odd would hollow out
                        // counters that overlap, such as in a bold 'B'.
                        fill_rule: FillRule::NonZero,
                        // Mode 1 strokes rather than fills; approximated as a fill, which
                        // is closer than drawing nothing, and noted so it is not silent.
                        paint: state.fill_paint(),
                        clip: state.clip,
                        blend: state.blend,
                    });
                }
            }

            // Word spacing applies only to the single-byte code 32.
            let word = if code == 32 {
                state.text.word_spacing
            } else {
                0.0
            };
            let shift = (advance_em * size + state.text.char_spacing + word) * scale;
            *text_matrix = Transform::translate(shift, 0.0).then(*text_matrix);
            self.text_cursor = Some((text_matrix.e, text_matrix.f));
        }

        if matches!(state.text.render_mode, 1 | 2 | 5 | 6) {
            self.note(Unsupported::Operator {
                operator: format!("text render mode {}", state.text.render_mode),
            });
        }
    }

    /// Looks up a named resource of a given category.
    fn resource(&self, resources: &Dictionary, category: &str, name: &str) -> Option<Object> {
        let table = self.document.get_key(resources, category);
        let table = table.as_dict()?;
        let value = table.get(name)?;
        Some(self.document.resolve(value))
    }
}

/// Skips an inline image, whose binary data is not PDF syntax.
///
/// Without this the lexer would tokenise compressed image bytes as operators and could emit
/// arbitrary drawing commands from data that is not a program at all.
fn skip_inline_image(lexer: &mut pdf_syntax::Lexer<'_>) {
    let input = lexer.input();
    let from = lexer.position();

    // `EI` delimited by whitespace marks the end. Searching for the bare bytes would match
    // inside the image data.
    let mut at = from;
    while let Some(found) = input
        .get(at..)
        .and_then(|rest| rest.windows(2).position(|window| window == b"EI"))
    {
        let candidate = at.saturating_add(found);
        let before_is_space = candidate
            .checked_sub(1)
            .and_then(|index| input.get(index))
            .is_some_and(|&byte| pdf_syntax::lexer::is_whitespace(byte));
        let after = input.get(candidate.saturating_add(2)).copied();
        let after_is_boundary = after.is_none_or(pdf_syntax::lexer::is_whitespace);

        if before_is_space && after_is_boundary {
            lexer.seek(candidate.saturating_add(2));
            return;
        }
        at = candidate.saturating_add(2);
    }

    // No terminator: the rest of the stream is image data.
    lexer.seek(input.len());
}

/// Converts a content-stream token into an operand.
fn token_to_object(token: pdf_syntax::Token) -> Object {
    match token {
        pdf_syntax::Token::Integer(value) => Object::Integer(value),
        pdf_syntax::Token::Real(value) => Object::Real(value),
        pdf_syntax::Token::Name(bytes) => Object::Name(pdf_syntax::Name::new(bytes)),
        pdf_syntax::Token::String(bytes) => Object::String(bytes.into()),
        // Arrays and dictionaries appear as operands to `d`, `TJ` and `BDC`. Recognising
        // the brackets is enough for the operators this interpreter implements; a full
        // re-parse would duplicate the object parser for no present gain.
        _ => Object::Null,
    }
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
fn number_at(operands: &[Object], index: usize) -> Option<f32> {
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

/// Reads the first `count` operands as numbers, requiring all of them.
fn numbers_from(operands: &[Object], count: usize) -> Option<Vec<f32>> {
    let values: Vec<f32> = (0..count)
        .filter_map(|index| number_at(operands, index))
        .collect();
    (values.len() == count).then_some(values)
}

/// Reads `count` coordinate pairs.
fn points_from(operands: &[Object], count: usize) -> Option<Vec<Point>> {
    let values = numbers_from(operands, count.saturating_mul(2))?;
    Some(
        values
            .chunks_exact(2)
            .map(|pair| Point::new(pair[0], pair[1]))
            .collect(),
    )
}

/// Reads six operands as a matrix.
fn matrix_from(operands: &[Object]) -> Option<Transform> {
    let values = numbers_from(operands, 6)?;
    Some(Transform::new(
        values[0], values[1], values[2], values[3], values[4], values[5],
    ))
}

/// Reads operand `index` as a string.
fn string_at(operands: &[Object], index: usize) -> Option<Vec<u8>> {
    operands.get(index)?.as_string().map(<[u8]>::to_vec)
}

/// Narrows a PDF number to `f32`.
fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a text adjustment outside f32's range is not a position on a page"
    )]
    {
        value as f32
    }
}

/// Reads operand `index` as a name.
fn name_at(operands: &[Object], index: usize) -> Option<String> {
    operands
        .get(index)?
        .as_name()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
}

/// Applies the `d` dash operator.
///
/// The array operand is not reconstructed by the content lexer, so only the "solid line"
/// case is honoured for now: an empty array means solid, and anything else leaves the
/// existing pattern. Getting this wrong draws a solid line where dashes belong, which is
/// visible but not structurally wrong.
fn set_dash(operands: &[Object], stroke: &mut Stroke) {
    if operands.iter().all(Object::is_null) {
        stroke.dash_array.clear();
        stroke.dash_phase = 0.0;
    }
}

/// Assigns a colour to the fill or stroke slot and records its component count.
fn assign_colour(state: &mut GraphicsState, fill: bool, colour: Color, components: usize) {
    if fill {
        state.fill = colour;
        state.fill_components = components;
    } else {
        state.stroke_colour = colour;
        state.stroke_components = components;
    }
}

/// Converts CMYK to RGB.
///
/// The naive conversion, which is what a viewer without colour management can do. A
/// managed pipeline would run the values through the document's output intent; that belongs
/// with the colour work, and doing it approximately here is honest about being approximate.
fn cmyk(c: f32, m: f32, y: f32, k: f32) -> Color {
    let (c, m, y, k) = (
        clamp_unit(c.into()),
        clamp_unit(m.into()),
        clamp_unit(y.into()),
        clamp_unit(k.into()),
    );
    Color::rgb(
        (1.0 - c) * (1.0 - k),
        (1.0 - m) * (1.0 - k),
        (1.0 - y) * (1.0 - k),
    )
}

/// Clamps a value to `0.0..=1.0` as an `f32`.
fn clamp_unit(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to 0.0..=1.0 before narrowing, so the conversion is exact"
    )]
    {
        value.clamp(0.0, 1.0) as f32
    }
}

/// Maps a PDF blend mode name.
fn blend_mode(name: &[u8]) -> BlendMode {
    match name {
        b"Multiply" => BlendMode::Multiply,
        b"Screen" => BlendMode::Screen,
        b"Overlay" => BlendMode::Overlay,
        b"Darken" => BlendMode::Darken,
        b"Lighten" => BlendMode::Lighten,
        b"ColorDodge" => BlendMode::ColorDodge,
        b"ColorBurn" => BlendMode::ColorBurn,
        b"HardLight" => BlendMode::HardLight,
        b"SoftLight" => BlendMode::SoftLight,
        b"Difference" => BlendMode::Difference,
        b"Exclusion" => BlendMode::Exclusion,
        b"Hue" => BlendMode::Hue,
        b"Saturation" => BlendMode::Saturation,
        b"Color" => BlendMode::Color,
        b"Luminosity" => BlendMode::Luminosity,
        // `Normal`, `Compatible`, and any name this reader does not know: the specification
        // requires an unrecognised blend mode to behave as Normal.
        _ => BlendMode::Normal,
    }
}
