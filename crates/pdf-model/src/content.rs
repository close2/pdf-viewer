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

use pdf_font::Code;
use pdf_render::Shading;
use pdf_render::display_list::Clip;
use pdf_render::{
    BlendMode, ClipId, Color, Command, DisplayList, FillRule, LineCap, LineJoin, Paint, Path,
    PathCommand, Point, Rect, Size, SoftMaskId, Stroke, Transform,
};
use pdf_syntax::{Dictionary, Document, Name, Object};

use crate::colour::ColourSpace;
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

/// Deepest nesting of soft-mask groups.
///
/// A mask's group is a content stream like any other, so it may set a soft mask of its own
/// — including, in a file that is broken or hostile, one whose `/G` is the group being
/// evaluated. That is a cycle the document controls, and this is what makes it terminate.
/// Four levels is far past anything a producer writes and cheap to allow: each level costs
/// a whole group's commands.
const MAX_SOFT_MASK_DEPTH: usize = 4;

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
    /// A page's content stream could not be decoded, so its drawing is missing.
    ///
    /// The page still renders — just without whatever that stream described. Without this
    /// report a page compressed with a filter we do not implement is indistinguishable from
    /// a page the producer meant to leave sparse.
    Content {
        /// What went wrong with which part of `/Contents`.
        issue: crate::page::ContentIssue,
    },
    /// An annotation carried something that could not be drawn.
    ///
    /// Almost always an annotation with no appearance stream, which would have to be
    /// synthesised from its type-specific entries — see `crate::annotation`.
    Annotation {
        /// The subtype and what was wrong with it.
        detail: String,
    },
    /// A bound was reached and interpretation stopped early.
    LimitReached {
        /// Which bound.
        limit: &'static str,
    },
    /// A text object whose glyphs should have knocked one another out (§9.3.8).
    ///
    /// `Tk`'s initial value is true, which makes a text object a non-isolated knockout
    /// transparency group, so a later glyph overwrites an earlier one where they overlap.
    /// This renderer composites each glyph against what is already there — the `Tk` false
    /// model — which differs only where two glyphs of one object overlap under a constant
    /// alpha below one or a blend mode other than Normal. Reported for exactly those
    /// objects; see [`Interpreter::end_text_object`].
    TextKnockout {
        /// How many glyphs the object marked the page with under such a paint.
        glyphs: usize,
    },
    /// One object painted in more than one part, which §11.6.2 says is one element.
    ///
    /// > Portions of an object shall not be composited with one another, even if they are
    /// > described in a way that would seem to cause overlaps (such as a self-intersecting
    /// > path, combined fill and stroke of a path, or a shading pattern containing an overlap
    /// > or fold-over).
    ///
    /// `B` and its three relatives fill *and* stroke one path, and a centred stroke always
    /// covers the edge of what the fill painted. This renderer emits two commands, so under a
    /// paint that composites the overlap is painted twice where the clause asks for once.
    /// Reported for exactly those paths; see [`Interpreter::end_path`]. §9.3.8's text knockout
    /// is the same sentence's exception for a text object, reported separately because its
    /// condition is different.
    CompositedInParts {
        /// What painted the object in parts.
        detail: &'static str,
    },
    /// A graphics state's `/SMask` that could not be evaluated (§11.6.5.1).
    ///
    /// A soft mask *is* implemented — a transparency group evaluated for its alpha (§11.5.2)
    /// or its luminosity (§11.5.3) — so what reaches this is a dictionary the clause does
    /// not describe: no `/G`, a subtype that is neither of the two Table 142 names, or a
    /// `/TR` that is not a readable function. The object is then painted *unmasked*, which
    /// is more ink than the document asked for and never less, and saying so is what keeps
    /// that from passing as a page we drew.
    SoftMask {
        /// What was wrong, and with which resource.
        detail: String,
    },
    /// A transparency group asking for something §11.4 defines and this does not do.
    ///
    /// A `/Group` is composited as §11.4.5's isolated, non-knockout group: its elements are
    /// drawn onto a transparent backdrop and the result painted once, under the constant
    /// alpha and blend mode in force at `Do`. Table 145's other two answers — non-isolated
    /// and knockout — and a group blending colour space that is not the device's are drawn
    /// the same way, and reported where they can change a pixel. See
    /// [`Interpreter::note_group_departures`] for each condition and the clause it is from.
    TransparencyGroup {
        /// What the group asked for.
        detail: String,
    },
    /// Optional content whose visibility could not be decided, so it was drawn.
    ///
    /// ISO 32000-2 §8.11. Only a visibility expression nested past the interpreter's bound
    /// reaches this: everything else the clause defines has an answer. Drawing is the
    /// deliberate choice of the two ways to be wrong — content that should be hidden is
    /// visible on the page, where content that should be visible would be missing without a
    /// trace — and saying so is what keeps it from being the second kind of failure.
    OptionalContent {
        /// What could not be decided.
        detail: String,
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

/// Whether black point compensation applies, per ISO 32000-2 §8.6.5.9.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlackPoint {
    /// `/UseBlackPtComp ON`, or the processor's own default.
    On,
    /// `/UseBlackPtComp OFF`, or any rendering intent of `AbsColorimetric` — for which
    /// the specification says the entry "shall be treated as OFF" whatever it holds.
    Off,
    /// `/UseBlackPtComp Default`, which the specification leaves to the processor.
    Default,
}

impl BlackPoint {
    /// Whether to compensate. `Default` does, which is this processor's determination.
    fn applies(self) -> bool {
        self != Self::Off
    }
}

/// What a `/Pattern` colour space's `scn` selected.
///
/// The two kinds are drawn in completely different ways. A shading pattern is a paint and
/// travels into the display list as one. A tiling pattern is a *content stream*, replayed
/// once per tile inside a clip shaped like the path being filled — so it never becomes a
/// paint and is expanded here instead.
#[derive(Debug, Clone)]
enum PatternPaint {
    /// A shading pattern (`/PatternType 2`), with Table 77's `/BBox` where it states one.
    ///
    /// The box travels as a rectangle and a transform rather than as a `ClipId`, because
    /// the clause makes it a clip "in addition to the current clipping path … in effect at
    /// that time" — the time being when the *pattern is painted*, which may be several `q`
    /// levels away from the `scn` that selected it.
    Shading(Arc<Shading>, Option<([f32; 4], Transform)>),
    /// A tiling pattern (`/PatternType 1`).
    Tiling(Rc<Tiling>),
}

/// A tiling pattern: a cell of content, and how to repeat it.
#[derive(Debug)]
struct Tiling {
    /// The cell's content stream.
    content: Arc<[u8]>,
    /// The resources its operators name.
    resources: Dictionary,
    /// Spacing between cells, in pattern space. Never zero.
    step: (f32, f32),
    /// Maps pattern space to the page's default space.
    to_page: Transform,
    /// The colour an uncoloured pattern is poured through, if it is uncoloured.
    ///
    /// `/PaintType 2` cells carry no colour of their own; the colour comes from `scn`.
    tint: Option<Color>,
}

/// What a form `XObject`'s `/Group` asks for (ISO 32000-2 §11.6.6 Table 145).
///
/// Only the three entries that change what is drawn. `/S` is not carried because a
/// dictionary whose subtype is not `/Transparency` never becomes one of these.
#[derive(Debug, Clone)]
struct TransparencyGroup {
    /// `/I`: whether the elements are composited onto a transparent initial backdrop
    /// (§11.4.5) rather than onto the group's backdrop.
    isolated: bool,
    /// `/K`: whether each element is composited with the initial backdrop rather than with
    /// the elements below it (§11.4.6).
    knockout: bool,
    /// `/CS`: the group's blending colour space, unresolved, or `Null` where absent.
    colour_space: Object,
}

/// One level of PDF graphics state.
#[derive(Debug, Clone)]
struct GraphicsState {
    transform: Transform,
    clip: Option<ClipId>,
    /// The current soft mask, or `None` for §11.6.4.3's implicit 1.0 everywhere.
    ///
    /// Set by `gs` and, like every other parameter here, saved and restored by `q`/`Q`.
    /// It is one identifier rather than the mask itself because a mask's group is a whole
    /// command list and the same mask commonly applies to every object on a page.
    soft_mask: Option<SoftMaskId>,
    fill: Color,
    /// The pattern set as the fill colour, if the fill space is `/Pattern`.
    fill_pattern: Option<PatternPaint>,
    /// As above, for stroking.
    stroke_pattern: Option<PatternPaint>,
    stroke_colour: Color,
    stroke: Stroke,
    blend: BlendMode,
    fill_alpha: f32,
    stroke_alpha: f32,
    /// Whether black point compensation applies to CIE-based conversions.
    ///
    /// ISO 32000-2 §8.6.5.9. `Default` is the initial value and leaves the choice to the
    /// processor; this one compensates, which is what makes blacks black.
    black_point: BlackPoint,
    /// The current fill colour space, which decides how `sc`/`scn` operands are read.
    fill_space: ColourSpace,
    /// As above, for stroking.
    stroke_space: ColourSpace,
    /// Text state, which `q`/`Q` saves and restores along with everything else.
    text: TextState,
}

/// The current font, which is one of the two kinds PDF has.
///
/// They differ in what a glyph *is*. Every font with a program hands out an outline, and the
/// interpreter fills it. A Type 3 font hands out a content stream, and the interpreter runs
/// it — see `crate::type3` for why that puts the two kinds in different crates.
#[derive(Debug, Clone)]
enum Font {
    /// A font with a glyph program, read by `pdf-font`.
    Program(Rc<pdf_font::LoadedFont>),
    /// A Type 3 font, whose glyphs are content streams (§9.6.4).
    Type3(Rc<crate::type3::Type3Font>),
}

impl Font {
    /// Splits a PDF string into character codes.
    ///
    /// A Type 3 font is a simple font — Table 110 gives it `/FirstChar` and `/LastChar`,
    /// which are byte codes — so one byte is one code, always.
    fn decode(&self, bytes: &[u8]) -> Vec<Code> {
        match self {
            Self::Program(font) => font.decode(bytes),
            Self::Type3(_) => bytes.iter().copied().map(Code::single_byte).collect(),
        }
    }

    /// A code's advance in text-space units, where one em is 1.0.
    fn advance(&self, code: Code) -> f32 {
        match self {
            Self::Program(font) => font.advance(code),
            Self::Type3(font) => font.advance(code.value()),
        }
    }

    /// Appends what a code means to the page's extracted text.
    fn text(&self, code: Code, out: &mut String) -> bool {
        match self {
            Self::Program(font) => font.text(code, out),
            Self::Type3(font) => font.text(code.value(), out),
        }
    }
}

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
struct TextObject {
    /// `Tm`, the text matrix.
    matrix: Transform,
    /// `Tlm`, the text line matrix: `Tm` as it was at the start of the current line.
    line: Transform,
    /// Glyph outlines accumulated by rendering modes 4 to 7, already in page space.
    ///
    /// Empty means no clipping mode has shown a glyph with an outline, which §9.3.6 makes a
    /// meaningful state of its own rather than an empty clip — see
    /// [`Interpreter::end_text_object`].
    clip: Path,
    /// Where this object's glyphs have marked the page under a paint that composites.
    ///
    /// `None` for a Type 3 glyph, whose ink is a content stream this does not run twice to
    /// find out. Accumulated rather than reported per glyph because knockout is a property
    /// of the *text object*: one glyph cannot overlap itself, so the difference §9.3.8
    /// describes needs two — see [`Interpreter::end_text_object`].
    composited: Vec<Option<Rect>>,
    /// Whether two of those glyphs were found to overlap, which is what `Tk` would change.
    knockout_owed: bool,
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

/// A glyph outline's bounding box in page space, for §9.3.8's overlap test.
///
/// Built from the control points rather than from the curves' extremes, so it contains the
/// outline rather than hugging it. Both approximations run the same way — the box is a
/// superset of the ink — which is what the caller needs.
fn outline_bounds(outline: &Path, transform: Transform) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    let mut add = |point: Point| {
        let mapped = transform.apply(point);
        bounds = Some(match bounds {
            Some(rect) => rect.union(Rect::from_corners(mapped, mapped)),
            None => Rect::from_corners(mapped, mapped),
        });
    };
    for command in outline.commands() {
        match *command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => add(point),
            PathCommand::CurveTo(first, second, end) => {
                add(first);
                add(second);
                add(end);
            }
            PathCommand::Close => {}
        }
    }
    bounds
}

/// Whether any command in a group, at any depth, satisfies `wanted`.
///
/// Recursive because a group's elements may themselves be groups: §11.4.3 calls both an
/// *element*, and a question about what a group contains is a question about its tree.
fn any_command(commands: &[Command], wanted: &dyn Fn(&Command) -> bool) -> bool {
    commands.iter().any(|command| {
        wanted(command)
            || match command {
                Command::Group { commands, .. } => any_command(commands, wanted),
                _ => false,
            }
    })
}

/// Whether a command asks to be blended with what is under it, rather than painted over it.
fn command_blends(command: &Command) -> bool {
    match command {
        Command::Fill { blend, .. }
        | Command::Stroke { blend, .. }
        | Command::Image { blend, .. }
        | Command::Group { blend, .. } => *blend != BlendMode::Normal,
        // `Command` is non-exhaustive. A command whose blending is unknown counts as
        // blending, because both callers decide whether to *report*: an unnecessary report
        // is recoverable, a missed one is a page drawn wrong in silence.
        _ => true,
    }
}

/// Whether a command's result can let what is under it show through.
///
/// The union of blending and of any transparency at all — a constant alpha below one, a
/// colour or shading carrying alpha, an image with a soft mask or a stencil, and a soft mask
/// in the graphics state. A group is transparent if its own alpha is, or if anything inside
/// it is: a group of half-opaque objects produces a half-opaque result.
///
/// Scanning an image's samples is linear in them, which is why the one caller asks this only
/// for a knockout group.
fn command_composites(command: &Command) -> bool {
    if command_blends(command) {
        return true;
    }
    // A soft mask is §11.6.4.1's third source of opacity, so an object painted through one
    // composites however opaque its colour is. `knockout_smask.pdf` is why this line is
    // here: its knockout group paints an opaque blue over an opaque red *under a mask*, and
    // without this the §11.4.6 report saw two opaque fills and stayed quiet about a page
    // three references draw the other way.
    if command.mask().is_some() {
        return true;
    }
    match command {
        Command::Fill { paint, .. } | Command::Stroke { paint, .. } => match paint {
            Paint::Solid(colour) => colour.a < 1.0,
            Paint::Shading(shading) => !shading.is_opaque(),
            // `Paint` is non-exhaustive, and a paint whose opacity is unknown is treated as
            // compositing: this decides whether to *report*, and an unnecessary report is
            // recoverable where a missed one is a page drawn wrong in silence.
            _ => true,
        },
        Command::Image { image, alpha, .. } => *alpha < 1.0 || !image.is_opaque(),
        Command::Group {
            commands, alpha, ..
        } => *alpha < 1.0 || any_command(commands, &command_composites),
        _ => true,
    }
}

/// Where a command marks the page, as a box containing its ink.
///
/// A superset rather than a tight fit — control points rather than curve extremes, and a
/// stroke widened by its whole line width rather than by half of it — because the one caller
/// is an overlap test for a *report*, where saying two elements might overlap when they do
/// not is recoverable and the reverse is a missed gap.
fn command_bounds(command: &Command) -> Option<Rect> {
    match command {
        Command::Fill {
            path, transform, ..
        } => outline_bounds(path, *transform),
        Command::Stroke {
            path,
            transform,
            stroke,
            ..
        } => {
            let bounds = outline_bounds(path, *transform)?;
            // The width is in the path's space, so it reaches the page scaled by the
            // transform — by the *largest* factor the transform stretches a length, since the
            // margin has to hold in every direction. This used to be the determinant's square
            // root, described as "an over-estimate for a sheared one", which is the wrong way
            // round: a shear can leave the determinant at 1 while tripling a length, so the
            // margin was too small and an overlap could be missed. `Transform::max_stretch`
            // is the bound the comment claimed.
            let margin = stroke.width * transform.max_stretch();
            Some(Rect::from_corners(
                Point::new(bounds.min.x - margin, bounds.min.y - margin),
                Point::new(bounds.max.x + margin, bounds.max.y + margin),
            ))
        }
        Command::Image { transform, .. } => {
            // An image occupies the unit square, which the command's transform places.
            let mut square = Path::new();
            square.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
            square.push(PathCommand::LineTo(Point::new(1.0, 0.0)));
            square.push(PathCommand::LineTo(Point::new(1.0, 1.0)));
            square.push(PathCommand::LineTo(Point::new(0.0, 1.0)));
            outline_bounds(&square, *transform)
        }
        Command::Group { commands, .. } => commands
            .iter()
            .filter_map(command_bounds)
            .reduce(Rect::union),
        // Unbounded, which the overlap test reads as covering everything.
        _ => None,
    }
}

/// Whether §11.4.6's knockout could change a pixel of this group.
///
/// True when an element that composites overlaps an element painted before it. Where the
/// upper element is opaque and blends Normal it overwrites the lower one under either model,
/// and where two elements do not overlap there is nothing to knock out.
///
/// An element whose ink cannot be bounded is taken to overlap everything, which is the same
/// direction [`TextObject::note_knockout`] errs in and for the same reason.
fn knockout_can_show(commands: &[Command]) -> bool {
    let mut painted: Vec<Option<Rect>> = Vec::with_capacity(commands.len());
    for command in commands {
        let bounds = command_bounds(command);
        if command_composites(command)
            && painted.iter().any(|below| match (below, bounds) {
                (Some(first), Some(second)) => {
                    first.min.x < second.max.x
                        && second.min.x < first.max.x
                        && first.min.y < second.max.y
                        && second.min.y < first.max.y
                }
                _ => true,
            })
        {
            return true;
        }
        painted.push(bounds);
    }
    false
}

/// The text-related part of the graphics state.
///
/// Separate from [`TextObject`], which the specification resets at every `BT` and which
/// therefore does not survive `q`/`Q`.
#[derive(Debug, Clone)]
struct TextState {
    /// The resource name of the current font, and the font itself once loaded.
    font: Option<Font>,
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
    /// `Tk`, text knockout: whether a text object composites as one knockout group.
    ///
    /// ISO 32000-2 §9.3.8, and Table 102's ninth text state parameter. It is the only one
    /// with no operator — "it may be set only through the TK entry in a graphics state
    /// parameter dictionary by using the gs operator" — and the only one this tree does not
    /// implement. It is carried anyway, because its *value* decides whether the gap can be
    /// seen: `false` asks for exactly what we do, and `true`, which is the initial value,
    /// asks for §11.4.6's knockout compositing, which we do not have.
    knockout: bool,
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
            // §9.3.8: "Its initial value shall be true."
            knockout: true,
        }
    }
}

impl GraphicsState {
    /// The initial state defined by ISO 32000-2 8.4.
    fn initial(base: Transform) -> Self {
        Self {
            transform: base,
            clip: None,
            soft_mask: None,
            fill: Color::BLACK,
            fill_pattern: None,
            stroke_pattern: None,
            stroke_colour: Color::BLACK,
            stroke: Stroke::default(),
            blend: BlendMode::Normal,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            black_point: BlackPoint::Default,
            fill_space: ColourSpace::Gray,
            stroke_space: ColourSpace::Gray,
            text: TextState::default(),
        }
    }

    /// Returns the fill colour with the constant alpha applied.
    fn fill_paint(&self) -> Paint {
        // A shading pattern replaces the colour entirely; PDF has no notion of tinting
        // one. A tiling pattern is not a paint at all — it is drawn by replaying its
        // content stream — so it leaves the colour alone here.
        //
        // The constant alpha still applies, and reaching every colour the shading carries is
        // the only way to apply it (§11.6.4.4, `Shading::with_alpha`). Until the fifteenth
        // session this line dropped it: `alphatrans.pdf` states `Gradient: .5` on the page
        // and draws its gradient over three other objects, and we painted it opaque while
        // three references showed what was behind it.
        if let Some(PatternPaint::Shading(shading, _)) = &self.fill_pattern {
            return Paint::Shading(shading_with_alpha(shading, self.fill_alpha));
        }
        Paint::Solid(Color {
            a: self.fill.a * self.fill_alpha,
            ..self.fill
        })
    }

    /// Whether painting under this state composites with what is already on the page.
    ///
    /// Opaque paint under the Normal blend mode overwrites what it covers, so every model of
    /// how overlapping parts combine gives the same pixels and a report about them would name
    /// pages that cannot differ. Both §9.3.8's text knockout and §11.6.2's one-object rule
    /// hang off this question — see [`Unsupported::TextKnockout`] and
    /// [`Unsupported::CompositedInParts`] — and asking it in one place keeps the two reports
    /// from drifting into different definitions of the same word.
    fn paint_composites(&self) -> bool {
        self.fill_alpha < 1.0 || self.stroke_alpha < 1.0 || self.blend != BlendMode::Normal
    }

    /// Returns the stroke colour with the constant alpha applied.
    fn stroke_paint(&self) -> Paint {
        if let Some(PatternPaint::Shading(shading, _)) = &self.stroke_pattern {
            return Paint::Shading(shading_with_alpha(shading, self.stroke_alpha));
        }
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
    let (content, issues) = page.content_with_report(document);
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
        base: base_transform(page),
        page: size,
        output_intent: output_intent_space(document),
        optional_content: crate::optional_content::OptionalContent::read(document),
        hidden: 0,
        glyph_depth: 0,
        soft_mask_depth: 0,
        uncoloured: false,
    };

    for issue in issues {
        interpreter.note(Unsupported::Content { issue });
    }

    let base = base_transform(page);
    interpreter.run(&content, &page.resources, &GraphicsState::initial(base), 0);
    // §12.5: an annotation is drawn *over* the page content, and in `/Annots` order, so
    // this pass follows the content stream rather than being folded into it.
    interpreter.draw_annotations(page, base);

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
    // §7.7.3.3 Table 31's `/UserUnit` is "the size of default user space units, in multiples
    // of 1/72 inch", so a page's extent *in the units a device resolution is stated in* is
    // its crop box scaled by it. Applying it here and in `base_transform` — rather than
    // asking every caller to multiply the scale it passes `TargetSpec::for_page` — keeps it
    // where the page's geometry already lives, and keeps the display list's own coordinates
    // in seventy-seconds of an inch whatever the file says a unit is.
    let unit = page.user_unit;
    let (width, height) = (page.width() * unit, page.height() * unit);
    if page.rotate == 90 || page.rotate == 270 {
        Size::new(height, width)
    } else {
        Size::new(width, height)
    }
}

/// Builds the transform from PDF user space to the page's own space.
///
/// Two things fold in here. The crop box may not start at the origin, so content is
/// translated by its lower-left corner; and `/Rotate` turns the page, which is a rotation
/// plus a translation to bring the result back into positive coordinates.
///
/// # Which way `/Rotate` turns, and the sign that has to be got right
///
/// ISO 32000-2 §7.7.3.3 Table 31 defines the entry as
///
/// > The number of degrees by which the page shall be rotated clockwise when displayed or
/// > printed.
///
/// **Clockwise as displayed.** This space is not the display's: page space keeps PDF's y-up
/// axis and [`pdf_render::TargetSpec::for_page`] does the flip to a raster's y-down one. A
/// turn that is clockwise on the screen is therefore a *negative* rotation here, and each
/// matrix below is that rotation composed with the translation that puts the page back in
/// the positive quadrant. Writing them out for `/Rotate 90`, where `H` is the unrotated
/// height: the rotation takes `(x, y)` to `(y, -x)`, and adding `H'` — the rotated page's own
/// height, which is the unrotated *width* — gives `(y, W - x)`.
///
/// Getting the sign wrong is invisible to everything except a picture, which is how it
/// survived from the first page tree until the twelfth session: 90 and 270 were exchanged, so
/// every rotated page in the corpus came out turned by 180° from what four other renderers
/// draw. Six pages were contradicted by it — five of `hello_world_rotated.pdf`, filed under
/// substituted fonts because they carry one — and a page that is upside down is one that
/// still has the right ink in the right *quantity*, so no metric in this tree could see it.
/// `rotation_turns_the_page_clockwise_as_displayed` pins all four angles.
fn base_transform(page: &Page) -> Transform {
    let shift = Transform::translate(-page.crop_box[0], -page.crop_box[1]);
    let (width, height) = (page.width(), page.height());

    let rotation = match page.rotate {
        // (x, y) -> (y, W - x). The rotated page is `height` wide and `width` tall.
        90 => Transform::new(0.0, -1.0, 1.0, 0.0, 0.0, width),
        // (x, y) -> (W - x, H - y).
        180 => Transform::new(-1.0, 0.0, 0.0, -1.0, width, height),
        // (x, y) -> (H - y, x).
        270 => Transform::new(0.0, 1.0, -1.0, 0.0, height, 0.0),
        // 0, and anything that normalised to it.
        _ => Transform::IDENTITY,
    };

    // `/UserUnit` last, because the rotation's translations are stated in the page's own
    // units and scaling before them would move the page off its own origin.
    shift
        .then(rotation)
        .then(Transform::scale(page.user_unit, page.user_unit))
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
    fonts: BTreeMap<String, Option<Font>>,
    /// Maps PDF user space to page space.
    ///
    /// Pattern space is defined relative to the page's default coordinates rather than to
    /// the transform in force when a pattern is used, so this is kept for patterns and
    /// must not be confused with the current transform.
    base: Transform,
    /// The page's extent, used to bound a shading painted by `sh`.
    page: Size,
    /// The colour space the document's output intent describes, if it has one.
    ///
    /// ISO 32000-2 §14.11.5: an output intent's `/DestOutputProfile` is "an ICC profile
    /// stream defining the transformation from the PDF document's source colours to
    /// output device colourants". §8.6.5.7 NOTE 3 names it as the one thing in a PDF that
    /// can say how its device colours are calibrated, so it is what a device space means
    /// when nothing nearer to hand says otherwise.
    output_intent: Option<ColourSpace>,
    /// The page's text, accumulated as the glyphs are placed.
    text: String,
    /// Where the last glyph ended, used to decide where a space belongs.
    text_cursor: Option<(f32, f32)>,
    /// The document's optional content configuration, if it has one (§8.11).
    optional_content: Option<crate::optional_content::OptionalContent>,
    /// How many enclosing `BDC /OC` sections are hidden.
    ///
    /// A counter rather than a flag because marked content nests, and the outermost hidden
    /// section wins: §8.11.2.1 says that if an outer level indicates content is to be
    /// hidden, "all inner levels shall be hidden regardless of their individual visibility
    /// states".
    hidden: usize,
    /// How many Type 3 glyph descriptions are being run, one per level of nesting.
    ///
    /// `d0` and `d1` are meaningful only inside one — §9.6.4 Table 111 says each "shall be
    /// used only in a content stream appearing in a Type 3 font's `CharProcs` dictionary"
    /// and this is what tells a stray one in a page's own content stream from a real one.
    glyph_depth: usize,
    /// How many soft-mask groups are being evaluated, one per level of nesting.
    ///
    /// See [`MAX_SOFT_MASK_DEPTH`]: a mask's group may set a mask of its own, and a
    /// document decides how deep that goes.
    soft_mask_depth: usize,
    /// Whether the content being run is a figure whose colour is supplied from outside it.
    ///
    /// ISO 32000-2 §8.6.8 names two such circumstances and gives them one rule: "in any glyph
    /// description that uses the d1 operator (see 9.6.4, "Type 3 fonts") and to all other
    /// content streams invoked from within the same glyph description", and "in the content
    /// stream of an uncoloured tiling pattern (see 8.7.3.3, "Uncoloured tiling patterns") and
    /// to all other content streams invoked from within the uncoloured tiling pattern
    /// stream". In both, a listed set of operators "shall be ignored" — which is what makes
    /// the colour the figure is *used* with survive to the marks inside it.
    ///
    /// A flag rather than a counter, and saved and restored by whoever set it, because the
    /// clause extends the restriction to everything such a stream invokes: an inner figure
    /// finishing must not re-enable colour for the rest of an outer one.
    uncoloured: bool,
}

impl Interpreter<'_> {
    fn note(&mut self, item: Unsupported) {
        self.unsupported.insert(item.clone(), item);
    }

    /// Whether the content being interpreted right now belongs to a hidden layer.
    ///
    /// What this suppresses is *marking the page*, and nothing else. §8.11.3.1 is explicit
    /// that hiding changes what is drawn and not what the graphics state becomes: colour,
    /// transformation and clipping "shall still be applied", the text position is updated
    /// "even for text wrapped in optional content", and the state after the section is the
    /// same whether it was drawn or not. Suppressing at the point a command enters the
    /// display list is what makes that true by construction rather than by care.
    fn is_hidden(&self) -> bool {
        self.hidden > 0
    }

    /// Whether content governed by `oc` is drawn, reporting what cannot be decided.
    ///
    /// `oc` is what a `BDC /OC`'s name finds in the page's `/Properties`, or the `/OC` entry
    /// of an `XObject` or an annotation — **as written**, reference and all. An optional
    /// content group is identified by which object it is (§8.11.2.2), so resolving it before
    /// this point loses the only identity it has.
    fn shows_optional_content(&mut self, oc: &Object) -> bool {
        use crate::optional_content::Visibility;

        let Some(optional_content) = &self.optional_content else {
            // §8.11.4.2: with no `/OCProperties`, "a PDF processor shall ignore any optional
            // content structures in the document".
            return true;
        };
        match optional_content.visibility(self.document, oc) {
            Visibility::Visible => true,
            Visibility::Hidden => false,
            Visibility::TooDeep => {
                self.note(Unsupported::OptionalContent {
                    detail: "a /VE visibility expression nested past the interpreter's bound"
                        .to_owned(),
                });
                true
            }
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
        // The text object's own parameters, which `BT` resets and `q`/`Q` do not touch.
        let mut text_object = TextObject::default();
        // One entry per open marked-content section, saying whether it hid what follows.
        // Every `BMC` and `BDC` pushes, so an `EMC` closes the section it actually belongs
        // to rather than the last optional one — which is why this is not just a counter.
        //
        // It carries no bound of its own because it already has one: a section costs an
        // operator, and `MAX_OPERATIONS` bounds those at four million. A stream that nests
        // that deep has spent its whole budget doing so.
        let mut marked: Vec<bool> = Vec::new();

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

            // §8.6.8: inside a `d1` glyph description or an uncoloured tiling pattern —
            // and inside everything either of them invokes — "all of the following operators
            // shall be ignored", the list being `is_colour_operator`. Dropping them here
            // rather than in each arm keeps the rule where the clause puts it, in one place
            // for both circumstances, and it is what lets the colour the figure is *used*
            // with reach the marks inside it.
            if self.uncoloured && is_colour_operator(operator.as_slice()) {
                operands.clear();
                continue;
            }

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
                b"gs" => self.apply_ext_gstate(&operands, resources, &mut state, in_text),

                // --- line parameters ---
                b"w" => {
                    if let Some(width) = number_at(&operands, 0) {
                        state.stroke.width = width.max(0.0);
                    }
                }
                b"J" => {
                    if let Some(code) = integer_at(&operands, 0) {
                        state.stroke.cap = line_cap(code);
                    }
                }
                b"j" => {
                    if let Some(code) = integer_at(&operands, 0) {
                        state.stroke.join = line_join(code);
                    }
                }
                b"M" => {
                    if let Some(limit) = number_at(&operands, 0) {
                        state.stroke.miter_limit = miter_limit(limit);
                    }
                }
                b"d" => set_dash(&operands, &mut state.stroke),
                // Rendering intent and flatness affect nothing this renderer does.
                // --- path construction ---
                b"m" => {
                    if let (Some(x), Some(y)) = (number_at(&operands, 0), number_at(&operands, 1)) {
                        current = Point::new(x, y);
                        start = current;
                        begin_subpath(&mut path, current);
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
                    close_subpath(&mut path);
                    current = start;
                }
                b"re" => {
                    if let Some(values) = numbers_from(&operands, 4) {
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
                // `g`, `rg` and `k` set a device space and a colour together — or the
                // matching `Default` space, where the resources name one, which is why
                // these resolve the space rather than naming it directly.
                b"g" | b"G" => {
                    if let Some(grey) = number_at(&operands, 0) {
                        let space = self.device_space("DeviceGray", resources);
                        let colour = convert(&space, &[grey], state.black_point);
                        assign_colour(&mut state, operator.as_slice() == b"g", colour, space);
                    }
                }
                b"rg" | b"RG" => {
                    if let Some(values) = numbers_from(&operands, 3) {
                        let space = self.device_space("DeviceRGB", resources);
                        let colour = convert(&space, &values, state.black_point);
                        assign_colour(&mut state, operator.as_slice() == b"rg", colour, space);
                    }
                }
                b"k" | b"K" => {
                    if let Some(values) = numbers_from(&operands, 4) {
                        let space = self.device_space("DeviceCMYK", resources);
                        let colour = convert(&space, &values, state.black_point);
                        assign_colour(&mut state, operator.as_slice() == b"k", colour, space);
                    }
                }
                b"cs" | b"CS" => {
                    let fill = operator.as_slice() == b"cs";
                    self.set_colour_space(&operands, resources, &mut state, fill);
                }
                b"sc" | b"scn" | b"SC" | b"SCN" => {
                    let fill = matches!(operator.as_slice(), b"sc" | b"scn");
                    self.set_colour(&operands, resources, &mut state, fill);
                }

                // --- text ---
                b"BT" => {
                    in_text = true;
                    // Table 105: `BT` initialises both matrices. Resetting the whole
                    // structure also discards any glyph outlines a malformed stream left
                    // unconsumed by an `ET`, which is the only state a second `BT` could
                    // otherwise carry into the text object it starts.
                    text_object = TextObject::default();
                }
                b"ET" => {
                    in_text = false;
                    self.end_text_object(&mut text_object, &mut state);
                }
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
                    if let (Some(x), Some(y)) = (number_at(&operands, 0), number_at(&operands, 1)) {
                        text_object.line = Transform::translate(x, y).then(text_object.line);
                        text_object.matrix = text_object.line;
                    }
                }
                b"TD" => {
                    if let (Some(x), Some(y)) = (number_at(&operands, 0), number_at(&operands, 1)) {
                        // `TD` is `Td` with the side effect of setting the leading.
                        state.text.leading = -y;
                        text_object.line = Transform::translate(x, y).then(text_object.line);
                        text_object.matrix = text_object.line;
                    }
                }
                b"Tm" => {
                    if let Some(matrix) = matrix_from(&operands) {
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
                    if let Some(bytes) = string_at(&operands, 0) {
                        self.show_text(&bytes, &state, &mut text_object, resources, form_depth);
                    }
                }
                b"TJ" => {
                    // The array operand is not reconstructed by the content lexer, so the
                    // strings and the numeric adjustments between them arrive as separate
                    // operands in order — which is enough to render them correctly.
                    for operand in &operands {
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
                                    // A positive adjustment moves *left*: it is subtracted,
                                    // in thousandths of an em, scaled by size and horizontal
                                    // scaling.
                                    let shift = -narrow(adjust) / 1000.0
                                        * state.text.size
                                        * state.text.horizontal_scale;
                                    text_object.matrix =
                                        Transform::translate(shift, 0.0).then(text_object.matrix);
                                }
                            }
                        }
                    }
                }
                b"'" => {
                    text_object.line =
                        Transform::translate(0.0, -state.text.leading).then(text_object.line);
                    text_object.matrix = text_object.line;
                    if let Some(bytes) = string_at(&operands, 0) {
                        self.show_text(&bytes, &state, &mut text_object, resources, form_depth);
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
                    text_object.line =
                        Transform::translate(0.0, -state.text.leading).then(text_object.line);
                    text_object.matrix = text_object.line;
                    if let Some(bytes) = string_at(&operands, 2) {
                        self.show_text(&bytes, &state, &mut text_object, resources, form_depth);
                    }
                }

                // --- XObjects ---
                b"Do" => self.draw_xobject(&operands, resources, &state, form_depth),

                // --- shadings and inline images ---
                b"sh" => {
                    let name = name_at(&operands, 0).unwrap_or_default();
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
                    let scanned = crate::inline_image::scan(
                        self.document,
                        lexer.input(),
                        lexer.position(),
                        resources,
                    );
                    lexer.seek(scanned.resume);
                    // A hidden layer suppresses the drawing and the report both: an image
                    // the document turns off is not one we failed to draw (§8.11.3.1).
                    if !self.is_hidden() {
                        match scanned.image {
                            Ok(stream) => {
                                self.draw_image(&Arc::new(stream), "<inline>", resources, &state);
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
                    // Absolute colorimetry reproduces the source's measured colours,
                    // including its own paper white and black; compensating for the black
                    // point would defeat that, so the specification forbids it here.
                    if let Some(name) = name_at(&operands, 0) {
                        state.black_point = if name == "AbsoluteColorimetric" {
                            BlackPoint::Off
                        } else {
                            BlackPoint::Default
                        };
                    }
                }
                // §8.11.3.2: a marked-content section is optional content when its tag is
                // `OC` and its property list names a group or a membership dictionary.
                // Because a group is an indirect object, the operand is a *name* into the
                // resource dictionary's `/Properties`; an inline dictionary cannot carry
                // one, so it governs nothing.
                b"BDC" => {
                    let hides = name_at(&operands, 0).is_some_and(|tag| tag == "OC")
                        && name_at(&operands, 1).is_some_and(|name| {
                            self.unresolved_resource(resources, "Properties", &name)
                                .is_some_and(|oc| !self.shows_optional_content(&oc))
                        });
                    marked.push(hides);
                    if hides {
                        self.hidden = self.hidden.saturating_add(1);
                    }
                }
                b"BMC" => marked.push(false),
                b"EMC" => {
                    if marked.pop() == Some(true) {
                        self.hidden = self.hidden.saturating_sub(1);
                    }
                }
                b"MP" | b"DP" | b"BX" | b"EX" | b"i" => {}

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
                    if operator.as_slice() == b"d1" && self.glyph_depth > 0 {
                        self.uncoloured = true;
                        // One shape, one colour. Table 111 says the description "is executed
                        // solely to determine the glyph's shape. Its colour shall be
                        // determined by the graphics state in effect each time this glyph is
                        // painted" — singular, and the clause's own reason for admitting an
                        // image mask is that a mask "merely defines a region of the page to
                        // be painted with the current colour". A description that strokes is
                        // therefore describing part of the same region, not asking for the
                        // stroking colour, so the two colour parameters become one here.
                        // Which of the two operations runs is the text rendering mode's
                        // business (§9.3.6) and is decided in `show_text`; making them the
                        // same colour is what stops that decision from changing the colour
                        // of an uncoloured glyph, which Table 111 does not allow it to.
                        state.stroke_colour = state.fill;
                        state.stroke_pattern = state.fill_pattern.clone();
                        state.stroke_alpha = state.fill_alpha;
                    }
                }

                other => {
                    self.note(Unsupported::Operator {
                        operator: String::from_utf8_lossy(other).into_owned(),
                    });
                }
            }

            operands.clear();
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
        let unclosed = marked.iter().filter(|hides| **hides).count();
        if unclosed > 0 {
            self.hidden = self.hidden.saturating_sub(unclosed);
            self.note(Unsupported::Operator {
                operator: "BDC without EMC".to_owned(),
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
        // Whether the content stream stated a path at all, which is asked *before* the
        // trailing `m` goes: a path the clause disregards down to nothing is not the same
        // thing as a painting operator with no path in front of it, and the two get
        // different answers below.
        let stated = !path.is_empty();

        // §8.5.3.3.1's trailing `m`, removed before anything reads the path: filling,
        // stroking and clipping all disregard it, and this is the one place all three meet.
        drop_trailing_point(path);

        // Hidden optional content still builds its clip below — §8.11.3.1 puts clipping
        // among the graphics state operations that "shall still be applied" — but marks
        // nothing.
        if !path.is_empty() && (fill.is_some() || stroke.is_some()) && !self.is_hidden() {
            // `B` fills *and* strokes one path, and both commands then describe the same
            // geometry; sharing it means the copy happens once rather than twice.
            let shared = Arc::new(path.clone());

            // A tiling pattern is not a paint: its cell is a content stream, replayed
            // across the area the path covers. Doing that here rather than in the display
            // list keeps the list flat — no backend needs to know what a pattern is.
            let fill_clip = self.paint_clip(state, true);
            let stroke_clip = self.paint_clip(state, false);
            if let (Some(rule), Some(PatternPaint::Tiling(tiling))) =
                (fill, state.fill_pattern.clone())
            {
                self.tile(&shared, rule, &tiling, state);
            } else if let Some(rule) = fill {
                self.list.push(Command::Fill {
                    path: Arc::clone(&shared),
                    transform: state.transform,
                    fill_rule: rule,
                    paint: state.fill_paint(),
                    clip: fill_clip,
                    mask: state.soft_mask,
                    blend: state.blend,
                });
            }
            if stroke.is_some() {
                self.list.push(Command::Stroke {
                    path: Arc::clone(&shared),
                    transform: state.transform,
                    stroke: state.stroke.clone(),
                    paint: state.stroke_paint(),
                    clip: stroke_clip,
                    mask: state.soft_mask,
                    blend: state.blend,
                });
            }
            // §11.6.2: the fill and the stroke are two parts of one object, and "[p]ortions
            // of an object shall not be composited with one another". They are two commands
            // here, so the band the stroke shares with the fill — half its width, for any
            // path with an interior — composites twice.
            //
            // Two conditions narrow that to the pages where it can be seen, and the second
            // one is not obvious: the paint has to composite at all, since opaque Normal
            // painting puts the stroke over the fill either way, and *both* parts have to
            // mark the page. A `B` whose fill or stroke alpha is zero is one object painted
            // once, and three of the six corpus documents that reach this line are exactly
            // that — `issue11045.pdf` fills at alpha 0 and strokes opaque, `issue3458.pdf`
            // strokes at alpha 0 and fills. Reporting them would name pages whose pixels are
            // the same under either model, which costs them their place in the oracle's
            // comparison and buys nothing.
            let fill_marks = fill.is_some()
                && (matches!(state.fill_pattern, Some(PatternPaint::Tiling(_)))
                    || marks(&state.fill_paint()));
            if fill_marks
                && stroke.is_some()
                && marks(&state.stroke_paint())
                && state.paint_composites()
            {
                self.note(Unsupported::CompositedInParts {
                    detail: "a path filled and stroked by one operator",
                });
            }
        }

        // A pending `W` takes effect now: the specification says the clip changes *after*
        // the current path is painted, so the fill and stroke above used the old clip and
        // everything following uses the new one. The new clip becomes a child of the
        // current one, since clipping intersects rather than replaces.
        //
        // A path §8.5.3.3.1 has just disregarded down to nothing still becomes a clip, and
        // that clip admits nothing: §8.5.4 defines the region as "the same area that would
        // be filled by the f operator", and an empty path fills none. `issue9017_reduced.pdf`
        // writes `568.938 673.022 m W n` around a shading, and all three reference renderers
        // leave that shading undrawn. A painting operator with *no* path in front of it is
        // the other case — §8.5.3.1 calls it an error — and leaves the clip alone rather
        // than blanking everything after it, which is the recovery a viewer owes a
        // malformed file.
        if let Some(rule) = pending_clip.take()
            && stated
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
        in_text: bool,
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
        // Table 57 `/D`: the line dash pattern, "expressed as an array of the form
        // [ dashArray dashPhase ]". The same pattern the `d` operator sets, written as a
        // real array rather than as flattened operands.
        if let Some(entry) = self.document.get_key(dict, "D").as_array()
            && let Some(items) = entry.first().map(|item| self.document.resolve(item))
            && let Some(items) = items.as_array()
        {
            let array = items
                .iter()
                .map(|item| self.document.resolve(item))
                .filter_map(|item| item.as_number())
                .map(narrow)
                .collect();
            let phase = entry
                .get(1)
                .map(|item| self.document.resolve(item))
                .and_then(|item| item.as_number())
                .map_or(0.0, narrow);
            apply_dash(array, phase, &mut state.stroke);
        }
        // Table 57's `/LC`, `/LJ` and `/ML`: the same three parameters `J`, `j` and `M` set,
        // through the other of §8.4.1 NOTE 1's two routes. `issue16287.pdf`, `issue7878.pdf`
        // and `extgstate.pdf` set all three this way, and none of them reached the stroke.
        if let Some(code) = self.document.get_key(dict, "LC").as_integer() {
            state.stroke.cap = line_cap(code);
        }
        if let Some(code) = self.document.get_key(dict, "LJ").as_integer() {
            state.stroke.join = line_join(code);
        }
        if let Some(limit) = self.document.get_key(dict, "ML").as_number() {
            state.stroke.miter_limit = miter_limit(narrow(limit));
        }
        // Table 57's `/Font`, which is `[font size]` with the font an *indirect reference*
        // rather than a resource name — the one entry here that cannot go through the same
        // route its operator does, since `Tf` and this crate's font cache are both keyed by
        // the name. One corpus document writes it, `extgstate.pdf`, and what it would change
        // is which glyphs the page draws, so it is named rather than passed over.
        if !matches!(self.document.get_key(dict, "Font"), Object::Null) {
            self.note(Unsupported::Font {
                detail: "a font selected by Table 57's /Font entry".to_owned(),
            });
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
        // Table 57's `/SA`: §10.7.5's automatic stroke adjustment. What it changes here is
        // the one rule of that clause a display can state exactly — a line under half a
        // device pixel wide "shall be rendered as a single-pixel line" — which
        // `Stroke::device_width` applies once for both backends, since only a backend knows
        // the resolution. The clause's other half, adjusting a stroke's *coordinates* to the
        // pixel grid for uniform thickness, is what anti-aliasing already achieves by a
        // different route; ADR 0028 has that argument and the ledger's §10.7.5 row records it.
        if let Object::Boolean(adjust) = self.document.get_key(dict, "SA") {
            state.stroke.adjust = adjust;
        }
        // Table 57's `/OP`, `/op` and `/OPM`: overprint and overprint mode, deliberately not
        // read, which §8.6.7 is explicit about rather than silent on. Overprinting decides
        // what happens to the device colourants a painting operation does *not* name, and
        // this device has three additive process colourants and no separations: "Not all
        // devices support overprinting. … If overprinting is not supported, the value of the
        // overprint parameter shall be ignored" (§8.6.7 NOTE 1), and of the overprint mode,
        // "It also shall not apply if the native colour space of the output device does not
        // include CMYK device colourants; in that case, source colours shall be converted to
        // the device's native colour space, and all components participate in the conversion,
        // whatever their values." §11.7.4's transparent-model reading reaches the same place
        // by a second route — see ADR 0028 and the ledger's §11.7.4 rows.
        // ISO 32000-2 §8.6.5.9 and its table entry: `/UseBlackPtComp` takes ON, OFF or
        // Default, and a rendering intent of AbsColorimetric forces it off regardless.
        //
        // Both are skipped inside an uncoloured figure. §8.6.8 lists the `/ExtGState` entries
        // such a stream may not set, and these two are the only ones on that list this tree
        // reads at all: `/UseBlackPtComp` by name, and `/RI` because the `ri` operator that
        // sets the same parameter is on the operator half of the same list. `/TR`, `/TR2`,
        // `/BG`, `/BG2`, `/UCR`, `/UCR2` and `/HT` describe a marking device and are read
        // nowhere here. The rest of this dictionary is not colour and still applies — the
        // line width §9.6.4 asks a glyph description to set explicitly among it.
        if !self.uncoloured {
            if let Object::Name(value) = self.document.get_key(dict, "UseBlackPtComp") {
                state.black_point = match value.as_bytes() {
                    b"ON" => BlackPoint::On,
                    b"OFF" => BlackPoint::Off,
                    _ => BlackPoint::Default,
                };
            }
            if let Object::Name(intent) = self.document.get_key(dict, "RI")
                && intent.as_bytes() == b"AbsoluteColorimetric"
            {
                state.black_point = BlackPoint::Off;
            }
        }
        match self.document.get_key(dict, "BM") {
            Object::Name(name) => state.blend = blend_mode(name.as_bytes()),
            Object::Array(items) => {
                // §11.6.3, of the deprecated array form: a processor "shall use the first
                // blend mode in the array that it recognizes (or Normal if it recognizes none
                // of them)". The first *name* is not the first recognised one — `[/FooBar
                // /Multiply]` names a mode this reader knows in second place — so the
                // recognition test has to be inside the search rather than after it.
                state.blend = items
                    .iter()
                    .map(|item| self.document.resolve(item))
                    .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                    .find_map(|name| known_blend_mode(&name))
                    .unwrap_or(BlendMode::Normal);
            }
            _ => {}
        }

        // §9.3.8, `/TK`: the ninth text state parameter, and the only one with no operator.
        // "Any TK value in a graphics state parameter dictionary installed using the gs
        // operator shall be ignored between the BT and ET operators delimiting a text
        // object" — so a `gs` inside a text object sets everything else here and not this.
        if !in_text && let Object::Boolean(knockout) = self.document.get_key(dict, "TK") {
            state.text.knockout = knockout;
        }

        // §11.6.4.3's soft mask: an independent source of shape or opacity, defined by a
        // transparency group and applied to every object painted while it is in force.
        // Table 57's `/AIS` decides whether its values are shape or opacity, and is
        // deliberately not read: this renderer carries one alpha per pixel, where §11.3.7.1
        // makes alpha the product `α = f × q` of the two — so a mask value multiplies the
        // same alpha whichever of the two it is called. The distinction is visible only to a
        // model that keeps shape and opacity apart, and the places that would need it —
        // knockout and non-isolated groups — are reported as departures already.
        match crate::soft_mask::entry(self.document, dict) {
            crate::soft_mask::SoftMaskEntry::None => state.soft_mask = None,
            crate::soft_mask::SoftMaskEntry::Mask(request) => {
                state.soft_mask = self.build_soft_mask(&request, state);
            }
            crate::soft_mask::SoftMaskEntry::Unusable(detail) => {
                state.soft_mask = None;
                self.note(Unsupported::SoftMask {
                    detail: format!("/{name}: {detail}"),
                });
            }
        }
    }

    /// Evaluates a soft mask's transparency group and registers it (§11.5, §11.6.5.1).
    ///
    /// Returns `None` when the group draws nothing at all, which §11.5.2's NOTE 2 makes a
    /// mask of zero — but only for the alpha derivation, where an empty group masks
    /// everything away; a luminosity mask over a white backdrop is a mask of *one*. Both
    /// answers are the mask's own, so the group is registered either way and only an
    /// unreadable one gives up here.
    fn build_soft_mask(
        &mut self,
        request: &crate::soft_mask::SoftMaskRequest,
        state: &GraphicsState,
    ) -> Option<SoftMaskId> {
        if self.soft_mask_depth >= MAX_SOFT_MASK_DEPTH {
            self.note(Unsupported::LimitReached {
                limit: "MAX_SOFT_MASK_DEPTH",
            });
            return None;
        }
        let Some(content) = self.document.decoded_stream_data(&request.group) else {
            self.note(Unsupported::SoftMask {
                detail: "/SMask names an undecodable /G".to_owned(),
            });
            return None;
        };

        // §11.6.5.1: "The mask's coordinate system shall be defined by concatenating the
        // transformation matrix specified by the Matrix entry in the transparency group's
        // form dictionary … with the current transformation matrix at the moment the soft
        // mask is established in the graphics state with the gs operator." The mask is
        // therefore fixed here, at the `gs`, and does not move with whatever transform is in
        // force when it is finally used.
        let mut inner = GraphicsState::initial(state.transform);
        if let Some(matrix) = self.matrix(&request.group.dict) {
            inner.transform = matrix.then(inner.transform);
        }
        // The clip chain starts fresh rather than inheriting the caller's. A mask is not
        // painted, so nothing about it is clipped by the path in force; its own `/BBox` is
        // the whole of its extent, and §11.6.5.1 gives the mask a value everywhere outside
        // that box — the transfer function of 0.0, or of the backdrop's luminosity — which
        // an inherited clip would have no way to express.
        if let Some(bbox) = self.rectangle(&request.group.dict, "BBox") {
            inner.clip = self.rect_clip(bbox, inner.transform, None);
            if inner.clip.is_none() {
                self.note(Unsupported::LimitReached { limit: "max_clips" });
                return None;
            }
        }

        let resources = self
            .document
            .get_key(&request.group.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_default();

        if let Some(detail) = &request.colour_space_departure {
            self.note(Unsupported::TransparencyGroup {
                detail: detail.clone(),
            });
        }

        let mark = self.list.command_count();
        self.soft_mask_depth = self.soft_mask_depth.saturating_add(1);
        self.run(&content, &resources, &inner, 0);
        self.soft_mask_depth = self.soft_mask_depth.saturating_sub(1);
        let commands = self.list.split_off_commands(mark);

        // §11.5.3, of the group a mask is derived from: "G may be any kind of group -
        // isolated or not, knockout or not - producing various effects on the C result in
        // each case." So Table 145's two flags mean here what they mean anywhere, and the
        // group is evaluated as the isolated non-knockout one either way — the same
        // departure, reported on the same conditions, rather than a second reading of the
        // same table.
        if let Some(group) = self.transparency_group(&request.group.dict) {
            self.note_group_structure(&group, &commands);
        }

        let evaluated = pdf_render::SoftMask {
            commands,
            kind: request.kind,
            transfer: request.transfer.clone(),
        };
        let Ok(id) = self.list.add_soft_mask(evaluated) else {
            self.note(Unsupported::LimitReached {
                limit: "max_soft_masks",
            });
            return None;
        };
        Some(id)
    }

    /// Sets a colour space, which decides how the operands of `sc`/`scn` are read.
    ///
    /// The space itself is kept rather than only its component count, so that `Separation`
    /// and `DeviceN` colours go through their tint transform and `Indexed` ones through
    /// their table. Reading them by component count alone treats a single ink tint as a
    /// grey level, which is a plausible and wrong colour.
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

        let space = ColourSpace::parse(
            self.document,
            &Object::Name(Name::new(name.as_bytes().to_vec())),
            resources,
        );
        let space = space.unwrap_or_else(|| {
            self.note(Unsupported::Shading {
                name: format!("colour space /{name}"),
            });
            ColourSpace::Gray
        });

        // §8.6.8: `cs` and `CS` "shall also set the current colour to its initial value,
        // which depends on the colour space". Omitting this leaves the previous space's
        // colour in place, which shows up as content painted in the wrong colour — and the
        // initial value is *not* simply black: `ColourSpace::initial_colour` carries the
        // clause's five cases, of which a `Separation`'s full ink and an `Indexed` space's
        // entry 0 are the two that are usually some other colour entirely.
        //
        // A `Pattern` space is the sixth case and has no components: its initial colour "shall
        // be a pattern object that causes nothing to be painted", which is a fully transparent
        // paint here, and the pattern the previous `scn` set has to go with it.
        let initial = space.initial_colour();
        let colour = if initial.is_empty() {
            Color::TRANSPARENT
        } else {
            convert(&space, &initial, state.black_point)
        };
        if fill {
            state.fill_space = space;
            state.fill = colour;
            state.fill_pattern = None;
        } else {
            state.stroke_space = space;
            state.stroke_colour = colour;
            state.stroke_pattern = None;
        }
    }

    /// Sets a colour from `sc`/`scn` operands, interpreting them by component count.
    fn set_colour(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &mut GraphicsState,
        fill: bool,
    ) {
        // A trailing name means a pattern rather than a colour.
        if let Some(name) = operands
            .iter()
            .filter_map(|operand| operand.as_name())
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
            .next()
        {
            // Numeric operands alongside the name are the colour an *uncoloured* tiling
            // pattern is poured through, in the pattern's underlying space.
            let tint: Vec<f32> = (0..operands.len())
                .filter_map(|index| number_at(operands, index))
                .collect();
            let pattern = self.pattern(&name, resources, &tint, state, fill);
            if fill {
                state.fill_pattern = pattern;
            } else {
                state.stroke_pattern = pattern;
            }
            return;
        }

        // Setting an ordinary colour clears any pattern the space had selected.
        if fill {
            state.fill_pattern = None;
        } else {
            state.stroke_pattern = None;
        }

        let space = if fill {
            &state.fill_space
        } else {
            &state.stroke_space
        };
        let values: Vec<f32> = (0..operands.len())
            .filter_map(|index| number_at(operands, index))
            .collect();

        // Where the operand count disagrees with the declared space, the operands win:
        // producers get `/CS` wrong more often than they get the operand count wrong, and
        // a device space with a matching component count is the likeliest intent.
        let colour = match (values.len(), space.components()) {
            (0, _) => return,
            (given, expected) if given == expected => convert(space, &values, state.black_point),
            (1, _) => ColourSpace::Gray.to_rgb(&values),
            (3, _) => ColourSpace::Rgb.to_rgb(&values),
            (4, _) => ColourSpace::Cmyk.to_rgb(&values),
            (given, expected) => {
                self.note(Unsupported::Shading {
                    name: format!("{given} colour components (expected {expected})"),
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

        // §8.11.3.3: a form or image XObject may carry an `/OC` entry naming a group or a
        // membership dictionary, and its visibility is that of the group "along with the
        // current visibility state in the context in which the XObject is invoked" — which
        // is what `is_hidden` already carries. §8.11.3.1 permits skipping such an object
        // entirely, because a form's state changes do not outlive it, and skipping is what
        // keeps an undrawable image inside a hidden layer from being reported as a gap.
        // Read unresolved: a group is identified by *which object* it is (§8.11.2.2).
        if let Some(oc) = stream.dict.get("OC").cloned()
            && !self.shows_optional_content(&oc)
        {
            // §8.9.5.4 step c): where a base image's `/OC` says it is *not* visible, its
            // `/Alternates` are examined in order and the first one visible is drawn in its
            // place. Nothing here selects an alternate, so the page loses a picture the
            // document expected to be there — said out loud rather than left blank, and only
            // in the one case where it can happen. An `/Alternates` array on a *visible* base
            // image changes nothing: step b) draws the base.
            if !matches!(
                self.document.get_key(&stream.dict, "Alternates"),
                Object::Null
            ) {
                self.note(Unsupported::Image {
                    name: format!("{name}: hidden, and its /Alternates are not selected from"),
                });
            }
            return;
        }
        if self.is_hidden() {
            return;
        }

        let subtype = self.document.get_key(&stream.dict, "Subtype");
        let subtype = subtype
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();

        if subtype == b"Image" {
            self.draw_image(&stream, &name, resources, state);
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
        if let Some(matrix) = self.matrix(&stream.dict) {
            inner.transform = matrix.then(inner.transform);
        }

        // §8.10.1 lists what `Do` performs on a form XObject, and step c) is "Clips
        // according to the form dictionary's BBox entry"; Table 93 says of `/BBox` that
        //
        // > These boundaries shall be used to clip the form XObject and to determine its
        // > size for caching.
        //
        // Required of every form, not only of an annotation's appearance — which is the one
        // place this tree had it. §11.6.6 needs it too: a group's shape is the union of its
        // elements "clipped by the group XObject's bounding box".
        //
        // A form with no `/BBox` is malformed, since Table 93 makes it required. It is drawn
        // unclipped rather than refused: there is no box to honour, and the alternative
        // reading — clip to nothing — would delete content the producer plainly meant to
        // draw.
        if let Some(bbox) = self.rectangle(&stream.dict, "BBox") {
            let Some(clip) = self.rect_clip(bbox, inner.transform, inner.clip) else {
                self.note(Unsupported::LimitReached { limit: "max_clips" });
                return;
            };
            inner.clip = Some(clip);
        }

        let form_resources = self
            .document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_else(|| resources.clone());

        let Some(group) = self.transparency_group(&stream.dict) else {
            self.run(&data, &form_resources, &inner, form_depth.saturating_add(1));
            return;
        };
        self.run_transparency_group(&group, &data, &form_resources, &inner, state, form_depth);
    }

    /// Runs a transparency group `XObject`'s content and composites it as one object.
    ///
    /// `inner` is the state its content runs under — the form's matrix and `/BBox` clip
    /// already applied — and `outer` the state at the `Do`, which is what the group as an
    /// object is painted with.
    fn run_transparency_group(
        &mut self,
        group: &TransparencyGroup,
        content: &[u8],
        resources: &Dictionary,
        inner: &GraphicsState,
        outer: &GraphicsState,
        form_depth: usize,
    ) {
        let mut inner = inner.clone();
        // §11.6.6, of what `Do` adds for a transparency group XObject:
        //
        // > Before execution of the transparency group XObject's content stream, the current
        // > blend mode in the graphics state shall be initialised to Normal , the current
        // > stroking and nonstroking alpha constants to 1.0, and the current soft mask to
        // > None .
        //
        // Its NOTE 1 gives the reason: those parameters apply to the *group*, once, when it
        // is composited into its parent, and leaving them in force would apply them a second
        // time to every element inside. All four are reset here, the soft mask included —
        // the group carries it instead, on the command below.
        inner.blend = BlendMode::Normal;
        inner.fill_alpha = 1.0;
        inner.stroke_alpha = 1.0;
        inner.soft_mask = None;

        let mark = self.list.command_count();
        self.run(content, resources, &inner, form_depth.saturating_add(1));
        let commands = self.list.split_off_commands(mark);
        if commands.is_empty() {
            return;
        }

        self.note_group_departures(group, &commands);
        // §11.6.6's final compositing: the group's shape "shall then be painted into the
        // parent group or page, using the group's accumulated colour and opacity at each
        // point" — under the state in force at `Do`, which is where `ca` and `/BM` were left
        // by the caller. `ca` and not `CA`, because painting a form is not a stroking
        // operation and §11.6.4.4 gives `CA` to those alone.
        self.list.push(Command::Group {
            commands,
            alpha: outer.fill_alpha,
            clip: inner.clip,
            // The mask in force at the `Do`, applied to the group as one object — which is
            // §11.6.4.3's NOTE 2 recommending exactly this construction: "[t]o apply a soft
            // mask to multiple objects, it is usually best to define the objects as a
            // transparency group and apply the mask to the group as a whole."
            mask: outer.soft_mask,
            blend: outer.blend,
        });
    }

    /// Reads a form `XObject`'s `/Group`, if it is a transparency group (§8.10.3, §11.6.6).
    ///
    /// `None` for a form with no `/Group` at all and for one whose group subtype is not
    /// `/Transparency`, which §11.6.6 makes the same case:
    ///
    /// > An ordinary form XObject -one having no Group entry -or having a Group entry with a
    /// > subtype other than Transparency -shall not be subject to any grouping behaviour for
    /// > transparency purposes.
    fn transparency_group(&mut self, dict: &Dictionary) -> Option<TransparencyGroup> {
        let group = self.document.get_key(dict, "Group");
        let group = group.as_dict()?;
        // §8.10.3 Table 94: `/S` is required and "identifies the type of group whose
        // attributes this dictionary describes"; `/Transparency` is the only subtype the
        // specification defines.
        if self.document.get_key(group, "S").as_name()?.as_bytes() != b"Transparency" {
            return None;
        }
        // Table 145's `/I` and `/K`, both booleans defaulting to false.
        Some(TransparencyGroup {
            isolated: matches!(self.document.get_key(group, "I"), Object::Boolean(true)),
            knockout: matches!(self.document.get_key(group, "K"), Object::Boolean(true)),
            colour_space: self.document.get_key(group, "CS"),
        })
    }

    /// Reports the parts of §11.4 this group asks for and does not get.
    ///
    /// A group is composited here onto a transparent backdrop, under its own constant alpha
    /// and blend mode: §11.4.5's isolated, non-knockout group. Three of Table 145's answers
    /// ask for something else, and each is reported only where it can change a pixel —
    /// a report that fires where the output is provably identical costs the page its place
    /// in the oracle's comparison and buys nothing.
    fn note_group_departures(&mut self, group: &TransparencyGroup, commands: &[Command]) {
        self.note_group_structure(group, commands);

        // §11.6.6: for an isolated group, a `/CS` means "all painting operators shall
        // convert source colours ... to the group colour space before compositing objects
        // into the group", and the result is interpreted in that space. Compositing here
        // happens on the device's RGB components, so a group asking for any other space is
        // blended with different arithmetic — visible only where something composites at
        // all, since an opaque Normal paint carries its colour through unchanged.
        if let Some(name) = self.group_space_departure(&group.colour_space)
            && any_command(commands, &command_composites)
        {
            self.note(Unsupported::TransparencyGroup {
                detail: format!("blending colour space {name}"),
            });
        }
    }

    /// Reports Table 145's `/I` and `/K`, which mean the same thing wherever a group is used.
    ///
    /// Split from [`Interpreter::note_group_departures`] because a *soft mask's* group asks
    /// the same two questions and a different colour-space question: §11.6.5.1 makes its
    /// `/CS` the space the mask's luminosity is computed in, where §11.6.6 makes a painted
    /// group's the space its elements are composited in. `crate::soft_mask` decides the
    /// first; this decides what the two share.
    fn note_group_structure(&mut self, group: &TransparencyGroup, commands: &[Command]) {
        // §11.4.4 composites a non-isolated group's elements onto the group's backdrop and
        // then removes that backdrop's contribution again (its NOTE 3). Under the Normal
        // blend mode the removal is exact and the backdrop cancels, which is what §11.6.7's
        // NOTE 1 states for the same computation applied to a pattern cell: "in the common
        // case in which the pattern consists entirely of objects painted with the Normal
        // blend mode, this behaviour can be optimised by treating the pattern cell as if it
        // were an isolated group. Since in this case the results depend only on the colour,
        // shape, and opacity of the pattern cell and not on those of the backdrop". So a
        // non-isolated group is drawn as an isolated one, and only a blend mode inside it
        // makes that a departure — which is the same sentence §11.4.4's NOTE 2 gives as the
        // reason the two kinds of group differ at all.
        if !group.isolated && any_command(commands, &|command| command_blends(command)) {
            self.note(Unsupported::TransparencyGroup {
                detail: "non-isolated, and an element blends with the backdrop it excludes"
                    .to_owned(),
            });
        }

        // §11.4.6: "In a knockout group, each individual element shall be composited with
        // the group's initial backdrop rather than with the stack of preceding elements in
        // the group." Where the upper of two overlapping elements is opaque and blends
        // Normal it overwrites either way, so the two models differ only where a later
        // element that composites covers an earlier one — which is the condition below,
        // and the same shape as §9.3.8's for a text object.
        if group.knockout && knockout_can_show(commands) {
            self.note(Unsupported::TransparencyGroup {
                detail: "knockout, and an element composites over another".to_owned(),
            });
        }
    }

    /// Names a group's `/CS` if compositing in it would differ from compositing in RGB.
    ///
    /// Compositing here happens on the three components of the device raster. §11.6.6 makes
    /// an absent `/CS` inherit the parent's space, and §11.4.7 leaves the page group's to the
    /// processor — so the spaces that ask for what already happens are the three-component
    /// RGB ones: `/DeviceRGB`, `CalRGB`, and an ICC profile of three components, each of
    /// which this tree already resolves *to* device RGB one colour at a time. Those are a
    /// colorimetric difference this renderer takes page-wide and records as a choice.
    ///
    /// What is reported is a space whose components are not those: `/DeviceGray`,
    /// `/DeviceCMYK`, `Separation` and `DeviceN` blend a different number of components, and
    /// `Lab` blends three that are not a linear map of these. Honouring any of them means
    /// compositing the group in its own components and converting once at the end, which is a
    /// second raster format rather than a colour conversion — which is why it is reported
    /// rather than approximated.
    fn group_space_departure(&mut self, entry: &Object) -> Option<String> {
        let object = self.document.resolve(entry);
        if matches!(object, Object::Null) {
            return None;
        }
        // Named before it is parsed, because what a report has to say is what the file
        // asked for, and a space this crate cannot read has no other description.
        let described = match &object {
            Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
            _ => "an array-formed space".to_owned(),
        };
        match ColourSpace::parse(self.document, &object, &Dictionary::new()) {
            Some(ColourSpace::Rgb | ColourSpace::CalRgb { .. }) => None,
            Some(ColourSpace::Icc { profile }) if profile.channels() == 3 => None,
            _ => Some(described),
        }
    }

    /// Reads a form dictionary's `/Matrix` (§8.10.2 Table 93).
    ///
    /// > An array of six numbers specifying the form matrix , which maps form space into
    /// > user space (see 8.3.4, "Transformation matrices").
    ///
    /// Shared by the two places that need it, because §11.6.5.1 defines a soft mask's
    /// coordinate system as this matrix concatenated with the transform in force at the
    /// `gs` — the same reading of the same entry that `Do` makes, and one worth making
    /// once.
    fn matrix(&mut self, dict: &Dictionary) -> Option<Transform> {
        let entry = self.document.get_key(dict, "Matrix");
        let items = entry.as_array()?;
        let values: Vec<f32> = items
            .iter()
            .map(|item| self.document.resolve(item))
            .filter_map(|item| item.as_number())
            .map(narrow)
            .collect();
        (values.len() >= 6).then(|| {
            Transform::new(
                values[0], values[1], values[2], values[3], values[4], values[5],
            )
        })
    }

    /// Reads a rectangle entry as four numbers, in the order the file wrote them.
    fn rectangle(&mut self, dict: &Dictionary, key: &str) -> Option<[f32; 4]> {
        let entry = self.document.get_key(dict, key);
        let items = entry.as_array()?;
        let values: Vec<f32> = items
            .iter()
            .map(|item| self.document.resolve(item))
            .filter_map(|item| item.as_number())
            .map(narrow)
            .collect();
        (values.len() >= 4).then(|| [values[0], values[1], values[2], values[3]])
    }

    /// The clip in force for a paint, including a shading pattern's own `/BBox`.
    ///
    /// ISO 32000-2 §8.7.4.3 Table 77 makes `/BBox` "a temporary clipping boundary … in
    /// addition to the current clipping path and any other clipping boundaries in effect at
    /// that time", so it nests *inside* whatever the graphics state already has rather than
    /// replacing it. A pattern with no `/BBox`, or a paint that is not a shading pattern,
    /// gets the state's clip unchanged and costs nothing.
    ///
    /// `None` for the clip is "no clip"; the error case — a display list already holding as
    /// many clips as it can — falls back to the state's own, because losing a bounding box
    /// draws too much and losing the whole command draws nothing.
    fn paint_clip(&mut self, state: &GraphicsState, fill: bool) -> Option<ClipId> {
        let pattern = if fill {
            state.fill_pattern.as_ref()
        } else {
            state.stroke_pattern.as_ref()
        };
        let Some(PatternPaint::Shading(_, Some((corners, transform)))) = pattern else {
            return state.clip;
        };
        self.rect_clip(*corners, *transform, state.clip)
            .or(state.clip)
    }

    /// Registers a clip shaped like a rectangle, nested inside `parent`.
    ///
    /// `None` only when the display list is full of clips, which the caller reports.
    fn rect_clip(
        &mut self,
        corners: [f32; 4],
        transform: Transform,
        parent: Option<ClipId>,
    ) -> Option<ClipId> {
        let [x0, y0, x1, y1] = corners;
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        self.list
            .add_clip(Clip {
                path,
                transform,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .ok()
    }

    /// Draws one image `XObject`.
    fn draw_image(
        &mut self,
        stream: &Arc<pdf_syntax::Stream>,
        name: &str,
        resources: &Dictionary,
        state: &GraphicsState,
    ) {
        // §8.6.8, of a `d1` glyph description or an uncoloured tiling pattern's stream:
        // "unless painting an image mask, all image painting operators shall be ignored".
        // Its NOTE 1 gives the reason, and it is the whole of what those two circumstances
        // are about — a stencil "does not specify colours; instead, it designates places
        // where the current colour is painted".
        if self.uncoloured
            && !matches!(
                self.document.get_key(&stream.dict, "ImageMask"),
                Object::Boolean(true)
            )
        {
            return;
        }

        // A soft mask whose grid is not the image's is mapped onto the same unit square and
        // combined at output resolution (§11.6.5.2 Table 143). We combine two rasters
        // instead, so a mask of a different size is not applied — and saying so is what
        // keeps `issue16263.pdf`'s black bars from passing as a page we drew.
        if let Some(detail) =
            crate::image::unapplied_soft_mask(self.document, &stream.dict, resources)
        {
            self.note(Unsupported::Image {
                name: format!("{name}: {detail}"),
            });
        }
        // `/Mask` makes part of the image transparent, either through an explicit mask — a
        // second image naming the areas to leave unpainted (§8.9.6.3) — or through a
        // colour-key range array (§8.9.6.4). Both are applied as of the fourteenth session;
        // what remains reportable is the cases they refuse, and `image::unapplied_mask` is
        // asked rather than the dictionary so that a report cannot outlive the gap.
        //
        // Not to be confused with §8.9.6.2, *stencil* masking, which is this image's own
        // `/ImageMask` and is implemented — see `tests/image_masks.rs`.
        if let Some(detail) = crate::image::unapplied_mask(self.document, &stream.dict, resources) {
            self.note(Unsupported::Image {
                name: format!("{name}: {detail}"),
            });
        }
        // A PDF image occupies the unit square in user space, so the command's transform is
        // the current transform and nothing else.
        match crate::image::decode(self.document, stream, resources, state.fill) {
            Ok(image) => self.list.push(Command::Image {
                image,
                transform: state.transform,
                alpha: state.fill_alpha,
                clip: state.clip,
                // §11.6.4.3: an image's own `/SMask`, `/SMaskInData` or `/Mask` "shall
                // override, for this image object only, the current soft mask in the
                // graphics state" — so the two are never applied together, and the state's
                // mask survives for whatever is drawn next.
                mask: (!crate::image::overrides_graphics_state_mask(self.document, &stream.dict))
                    .then_some(state.soft_mask)
                    .flatten(),
                blend: state.blend,
            }),
            Err(error) => self.note(Unsupported::Image {
                name: format!("{name}: {error}"),
            }),
        }
    }

    /// Draws the page's annotations over its content, in `/Annots` order.
    ///
    /// ISO 32000-2 §12.5.5: each appearance is a form `XObject`, so this resolves *where* it
    /// goes — `crate::annotation` does that — and then hands it to the same machinery that
    /// runs any other form. The only reason it is a separate pass rather than a `Do` is
    /// that nothing in the content stream refers to it.
    fn draw_annotations(&mut self, page: &Page, base: Transform) {
        let annotations = self.document.get_key(&page.dict, "Annots");
        let Some(entries) = annotations.as_array().map(<[Object]>::to_vec) else {
            return;
        };

        for entry in &entries {
            let resolved = self.document.resolve(entry);
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            // §8.11.3.3: "If an annotation contains an OC entry, it shall be visible for
            // screen or print only if the flags have the appropriate settings and the group
            // or membership dictionary indicates it shall be visible." The flags are
            // `decide`'s business (§12.5.3); this is the other half of the condition, and it
            // is silent because an annotation the document hides is not one we failed to
            // draw.
            if let Some(oc) = dict.get("OC").cloned()
                && !self.shows_optional_content(&oc)
            {
                continue;
            }
            match crate::annotation::decide(self.document, dict) {
                crate::annotation::Decision::Nothing => {}
                crate::annotation::Decision::Unsupported(detail) => {
                    self.note(Unsupported::Annotation { detail });
                }
                crate::annotation::Decision::Draw { appearance, owed } => {
                    // What the subtype's clause asks for and `crate::appearance` could not
                    // construct — a field's value, a bevel's shadow — said out loud beside the
                    // part that *is* drawn, rather than either being lost.
                    if let Some(detail) = owed {
                        self.note(Unsupported::Annotation { detail });
                    }
                    self.draw_appearance(&appearance, base, &page.resources);
                }
            }
        }
    }

    /// Runs one appearance stream, clipped to its `/BBox`.
    fn draw_appearance(
        &mut self,
        appearance: &crate::annotation::Appearance,
        base: Transform,
        page_resources: &Dictionary,
    ) {
        let data = match &appearance.content {
            crate::annotation::Content::Stored(stream) => {
                let Some(data) = self.document.decoded_stream_data(stream) else {
                    self.note(Unsupported::Annotation {
                        detail: "undecodable appearance stream".to_owned(),
                    });
                    return;
                };
                data
            }
            crate::annotation::Content::Constructed { bytes, .. } => Arc::from(bytes.as_slice()),
        };

        let transform = appearance.transform.then(base);
        let mut state = GraphicsState::initial(transform);
        // Table 166: `/ca` and `/CA` are the opacities a *constructed* appearance's nonstroking
        // and stroking operations use. A stored stream carries 1.0 here, because §12.5.2 has a
        // reader ignore both — see `crate::annotation`.
        state.fill_alpha = appearance.fill_alpha;
        state.stroke_alpha = appearance.stroke_alpha;
        if let Some(name) = &appearance.blend {
            state.blend = blend_mode(name.as_bytes());
        }

        // §8.10.2: a form `XObject`'s `/BBox` "shall be" the clip for its content. §12.5.5
        // relies on that — the whole algorithm is about making the box cover `/Rect`, and
        // an appearance drawing outside its own box would spill across the page.
        let bbox = &appearance.bbox;
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(bbox[0], bbox[1])));
        path.push(PathCommand::LineTo(Point::new(bbox[2], bbox[1])));
        path.push(PathCommand::LineTo(Point::new(bbox[2], bbox[3])));
        path.push(PathCommand::LineTo(Point::new(bbox[0], bbox[3])));
        path.push(PathCommand::Close);
        let Ok(clip) = self.list.add_clip(Clip {
            path,
            transform,
            fill_rule: FillRule::NonZero,
            parent: None,
        }) else {
            self.note(Unsupported::LimitReached { limit: "max_clips" });
            return;
        };
        state.clip = Some(clip);

        // §7.8.3: an appearance stream is a form `XObject` (§12.5.5), and a form written
        // before PDF 1.2 may omit `/Resources` — "All resources that are referenced from
        // those forms and fonts shall be inherited from the resource dictionary of the page
        // on which they are used." An empty dictionary instead loses every named font and
        // image the appearance draws with.
        let resources = match &appearance.content {
            crate::annotation::Content::Stored(stream) => self
                .document
                .get_key(&stream.dict, "Resources")
                .as_dict()
                .cloned()
                .unwrap_or_else(|| page_resources.clone()),
            // A constructed stream names at most the fonts §12.7.4.3's `/DA` string reaches,
            // which `crate::appearance` took from Table 224's `/DR`. Everything else it draws
            // is a path in a device colour and names nothing.
            crate::annotation::Content::Constructed { resources, .. } => resources.clone(),
        };

        // Depth 1 rather than 0: an appearance is itself a form, so a chain of forms
        // inside it is bounded the same way one inside the page content is.
        self.run(&data, &resources, &state, 1);
    }

    /// Loads a font by resource name, caching the result including failures.
    ///
    /// A failure is cached too: a page that names an unloadable font on every `Tf` should
    /// pay for the attempt once, and should report it once.
    fn font(&mut self, resources: &Dictionary, name: &str) -> Option<Font> {
        if let Some(cached) = self.fonts.get(name) {
            return cached.clone();
        }

        let dict = self
            .resource(resources, "Font", name)
            .and_then(|object| object.as_dict().cloned());
        let loaded = dict
            .as_ref()
            .map(|dict| pdf_font::LoadedFont::load(self.document, dict, name));

        let result = match loaded {
            Some(Ok(font)) => Some(Font::Program(Rc::new(font))),
            // A Type 3 font has no program for `pdf-font` to read: its glyphs are content
            // streams, so it is this crate that draws them (§9.6.4). The refusal there is
            // the hand-off rather than a failure, which is why this is not a report.
            Some(Err(pdf_font::FontError::Type3 { .. })) => {
                match dict
                    .as_ref()
                    .map(|dict| crate::type3::Type3Font::read(self.document, dict, name))
                {
                    Some(Ok(font)) => Some(Font::Type3(Rc::new(font))),
                    Some(Err(error)) => {
                        self.note(Unsupported::Font {
                            detail: error.to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }
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
    ///
    /// # The rendering mode
    ///
    /// §9.3.6 Table 104's eight modes are three independent operations — fill, stroke, add
    /// to the clipping path — rather than eight cases, and they are read that way below.
    /// The clause makes each behave as it would for a path: "Stroking, filling, and clipping
    /// shall have the same effects for a text object as they do for a path object … although
    /// they are specified in an entirely different way."
    fn show_text(
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

        // The three operations of Table 104. Mode 3 does none of them and mode 7 only the
        // last, which is what an OCR layer under a scanned image uses; either way the text
        // matrix still advances and the extracted text still accumulates, because §9.3.6
        // requires it — "The e and f components of Tm shall be updated for each glyph drawn
        // when using text rendering mode 3 or 7 in exactly the same way as would be done for
        // other text rendering modes."
        //
        // A hidden optional-content layer suppresses the two that mark the page and *not*
        // the clip: §8.11.3.1 lists clipping among the "graphics state operations" that
        // "shall still be applied", and requires that "graphics state parameters that
        // persist past the end of a marked-content section shall be the same whether the
        // optional content is visible or not". The clip a text object leaves behind is one
        // of those, since it outlives the `ET` that built it.
        let mode = state.text.render_mode;
        let fills = matches!(mode, 0 | 2 | 4 | 6) && !self.is_hidden();
        let strokes = matches!(mode, 1 | 2 | 5 | 6) && !self.is_hidden();
        let clipping = matches!(mode, 4..=7);
        let size = state.text.size;
        let scale = state.text.horizontal_scale;

        // How wide a gap has to be before it means a word break rather than kerning.
        //
        // Measured against the font's own space, because that is what a word break is made
        // of. A fixed fraction of the font size cannot work: a title set with loose
        // tracking moves each glyph further than a body-text space, and judging it by size
        // alone spells "Clarification" as "Clar if ic at ion".
        //
        // Taken from the magnitude of the size because §9.3.1's NOTE says "Negative text
        // font size is permitted", and a negative threshold is below every gap there is —
        // which would have put a space between every pair of glyphs in the extracted text.
        let space_em = font.advance(Code::single_byte(32));
        let word_gap = if space_em > 0.0 {
            space_em * size.abs() * 0.6
        } else {
            size.abs() * 0.25
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
            let here = (text.matrix.e, text.matrix.f);
            if let Some((last_x, last_y)) = self.text_cursor {
                let gap = here.0 - last_x;
                if (here.1 - last_y).abs() > size.abs() * 0.5 {
                    self.text.push('\n');
                } else if gap > word_gap {
                    self.text.push(' ');
                }
            }
            font.text(code, &mut self.text);

            // §9.3.8: with `Tk` true — its initial value — the whole text object behaves as
            // a non-isolated knockout group, so "later glyphs shall overwrite ('knock out')
            // earlier ones in the area of overlap". We composite each glyph against what is
            // already on the page, which is exactly the `Tk` false behaviour. Two conditions
            // have to hold before the models can differ, and both are checked rather than
            // assumed: the paint has to composite at all — an opaque glyph under the Normal
            // blend mode overwrites what it covers either way — and two glyphs of the object
            // have to overlap, which most text never does.
            let knockout_can_show =
                (fills || strokes) && state.text.knockout && state.paint_composites();

            if (fills || strokes || clipping) && size != 0.0 {
                // Glyph space to text space: scale by the font size, apply horizontal
                // scaling and rise, then the text matrix and the current transform. §9.4.4
                // calls this the text rendering matrix, and both kinds of glyph are placed
                // by it — the difference is only what is placed.
                let glyph_to_text =
                    Transform::new(size * scale, 0.0, 0.0, size, 0.0, state.text.rise);
                let glyph_to_user = glyph_to_text.then(text.matrix);
                let transform = glyph_to_user.then(state.transform);

                let glyph_fill_clip = self.paint_clip(state, true);
                match &font {
                    Font::Program(program) => {
                        if let Some(outline) = program.outline(code) {
                            if fills {
                                self.list.push(Command::Fill {
                                    // The font hands out shared outlines and the display
                                    // list keeps them shared: a page of text is the same few
                                    // dozen glyphs over and over, so this is a refcount
                                    // rather than a copy of the segments.
                                    path: Arc::clone(&outline),
                                    transform,
                                    // Glyph outlines are non-zero filled; even-odd would
                                    // hollow out counters that overlap, such as in a bold
                                    // 'B'.
                                    fill_rule: FillRule::NonZero,
                                    paint: state.fill_paint(),
                                    clip: glyph_fill_clip,
                                    mask: state.soft_mask,
                                    blend: state.blend,
                                });
                            }
                            if strokes {
                                self.stroke_glyph(&outline, glyph_to_user, state);
                            }
                            if clipping {
                                // §9.3.6 wants "a single path, treating the individual
                                // outlines as subpaths of that path", and the glyphs of one
                                // text object have as many transforms as there are glyphs —
                                // so the transform is baked in here and the clip carries
                                // none. Note that a hidden layer still reaches this line.
                                text.clip.extend_transformed(&outline, transform);
                            }
                            if knockout_can_show {
                                text.note_knockout(outline_bounds(&outline, transform));
                            }
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
                            self.draw_type3_glyph(
                                type3,
                                code.value(),
                                state,
                                transform,
                                resources,
                                form_depth,
                            );
                            if knockout_can_show {
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
            let shift = (advance_em * size + state.text.char_spacing + word) * scale;
            text.matrix = Transform::translate(shift, 0.0).then(text.matrix);
            self.text_cursor = Some((text.matrix.e, text.matrix.f));
        }
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
    fn end_text_object(&mut self, text: &mut TextObject, state: &mut GraphicsState) {
        // §9.3.8's knockout is a property of the finished object, so this is where it can be
        // judged: two or more glyphs marked the page under a paint that composites, and `Tk`
        // asked for them to knock one another out instead. Reported rather than approximated
        // — implementing it means §11.4.6's knockout groups, which is the same gap seen from
        // clause 11 and the largest one left in this renderer.
        //
        // The condition is deliberately narrow. Reporting every text object drawn while `Tk`
        // is true would report almost every page in the world, since true is the initial
        // value, and would say nothing: with opaque glyphs and the Normal blend mode the two
        // models produce identical pixels.
        if text.knockout_owed {
            let glyphs = text.composited.len();
            self.note(Unsupported::TextKnockout { glyphs });
        }
        text.knockout_owed = false;
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

        let Some(data) = self.document.decoded_stream_data(&glyph) else {
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

        let saved_uncoloured = self.uncoloured;
        self.glyph_depth = self.glyph_depth.saturating_add(1);
        self.run(
            &data,
            font.resources(resources),
            &inner,
            form_depth.saturating_add(1),
        );
        self.glyph_depth = self.glyph_depth.saturating_sub(1);
        // `d1` inside the description raised this; the description is over. Restoring rather
        // than clearing is what lets an uncoloured glyph invoke another one without the
        // inner one's end re-enabling colour for the rest of the outer.
        self.uncoloured = saved_uncoloured;
    }

    /// Paints a tiling pattern over the area a path covers.
    ///
    /// The path becomes a clip and the pattern's cell is replayed once per tile position
    /// inside it. Expanding the tiling here rather than inventing a display-list paint for
    /// it keeps the list flat: a backend never learns what a pattern is, and the result is
    /// resolution-independent because the cell is real geometry rather than a rendered
    /// image.
    fn tile(&mut self, path: &Arc<Path>, rule: FillRule, tiling: &Tiling, state: &GraphicsState) {
        /// Most cells one pattern fill may draw.
        ///
        /// A small cell over a large area is an enormous number of tiles, and the content
        /// stream inside each one is unbounded. This is the bound that keeps a pattern
        /// from becoming a decompression bomb with extra steps.
        const MAX_TILES: usize = 4096;

        // The pattern is anchored to the page, so the question "which cells does this path
        // touch" has to be asked in the pattern's own coordinates.
        let Some(to_pattern) = tiling.to_page.invert() else {
            self.note(Unsupported::Shading {
                name: "a tiling pattern's matrix is degenerate".to_owned(),
            });
            return;
        };
        let path_to_pattern = state.transform.then(to_pattern);

        let Some(bounds) = bounds_of(path, path_to_pattern) else {
            return;
        };
        let (first_column, last_column) = span(bounds.0, bounds.2, tiling.step.0);
        let (first_row, last_row) = span(bounds.1, bounds.3, tiling.step.1);

        let columns = last_column.saturating_sub(first_column).saturating_add(1);
        let rows = last_row.saturating_sub(first_row).saturating_add(1);
        let total = usize::try_from(columns)
            .unwrap_or(usize::MAX)
            .saturating_mul(usize::try_from(rows).unwrap_or(usize::MAX));
        if total > MAX_TILES {
            self.note(Unsupported::LimitReached { limit: "MAX_TILES" });
            return;
        }

        // The path clips every cell, so a tile that falls outside it contributes nothing.
        let clip = Clip {
            path: (**path).clone(),
            transform: state.transform,
            fill_rule: rule,
            parent: state.clip,
        };
        let Ok(clip) = self.list.add_clip(clip) else {
            self.note(Unsupported::LimitReached { limit: "max_clips" });
            return;
        };
        let clip = Some(clip);

        for row in first_row..=last_row {
            for column in first_column..=last_column {
                let offset = Transform::translate(
                    tiling.step.0 * as_f32(column),
                    tiling.step.1 * as_f32(row),
                );
                let mut cell = GraphicsState::initial(offset.then(tiling.to_page));
                cell.clip = clip;
                cell.blend = state.blend;
                cell.fill_alpha = state.fill_alpha;
                cell.stroke_alpha = state.stroke_alpha;
                // An uncoloured pattern is a stencil: the colour given alongside the
                // pattern name is what pours through it. §8.6.8 is what makes that true of a
                // cell whose content stream *does* try to set a colour — it is the second of
                // the clause's two circumstances, and the colour operators inside it "shall
                // be ignored" exactly as they are inside a `d1` glyph description.
                let saved_uncoloured = self.uncoloured;
                if let Some(tint) = tiling.tint {
                    cell.fill = tint;
                    cell.stroke_colour = tint;
                    self.uncoloured = true;
                }
                self.run(
                    &tiling.content,
                    &tiling.resources,
                    &cell,
                    MAX_FORM_DEPTH - 1,
                );
                self.uncoloured = saved_uncoloured;
            }
        }
    }

    /// Paints a shading across the current clip, for the `sh` operator.
    ///
    /// `sh` covers the whole clipping region rather than a path, so the geometry drawn is
    /// the page itself and the clip does the shaping. Where the shading does not extend,
    /// it paints nothing, so the covered area is only ever as large as the shading says.
    fn paint_shading(&mut self, name: &str, resources: &Dictionary, state: &GraphicsState) {
        // `sh` marks the page and changes nothing else, so a hidden layer skips it whole —
        // including the report a shading we cannot build would otherwise make about a
        // shading that was never going to be drawn.
        if self.is_hidden() {
            return;
        }
        let Some(object) = self.resource(resources, "Shading", name) else {
            self.note(Unsupported::Shading {
                name: format!("/{name} is not in /Shading"),
            });
            return;
        };

        // `sh` is drawn in the current user space, unlike a pattern.
        //
        // Table 77's `/BBox` is read here rather than inside `shading::build`, because it is
        // stated "in the shading's target coordinate space" — which is the space the caller
        // paints into, not the shading's own — and because the clause makes it a *clip*
        // rather than a property of the gradient: "this bounding box shall be applied as a
        // temporary clipping boundary when the shading is painted, in addition to the
        // current clipping path and any other clipping boundaries in effect at that time".
        let clip = crate::shading::bbox_of(self.document, &object).map_or(state.clip, |corners| {
            self.rect_clip(corners, state.transform, state.clip)
                .or(state.clip)
        });
        match crate::shading::build(self.document, &object, resources, state.transform) {
            Ok(shading) => {
                let mut path = Path::new();
                path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
                path.push(PathCommand::LineTo(Point::new(self.page.width, 0.0)));
                path.push(PathCommand::LineTo(Point::new(
                    self.page.width,
                    self.page.height,
                )));
                path.push(PathCommand::LineTo(Point::new(0.0, self.page.height)));
                path.push(PathCommand::Close);

                self.list.push(Command::Fill {
                    path: Arc::new(path),
                    // The page rectangle is already in page space, so it needs no further
                    // transform; the shading carries its own.
                    transform: Transform::IDENTITY,
                    fill_rule: FillRule::NonZero,
                    // §11.6.4.4's non-stroking constant applies to `sh` as to any other
                    // non-stroking painting operation.
                    paint: Paint::Shading(shading_with_alpha(&Arc::new(shading), state.fill_alpha)),
                    clip,
                    mask: state.soft_mask,
                    blend: state.blend,
                });
            }
            Err(error) => self.note(Unsupported::Shading {
                name: format!("/{name}: {error}"),
            }),
        }
    }

    /// Resolves a pattern name, for `scn` in a `/Pattern` colour space.
    fn pattern(
        &mut self,
        name: &str,
        resources: &Dictionary,
        tint: &[f32],
        state: &GraphicsState,
        fill: bool,
    ) -> Option<PatternPaint> {
        let object = self.resource(resources, "Pattern", name)?;
        let dict = match &object {
            Object::Dictionary(dict) => dict.clone(),
            Object::Stream(stream) => stream.dict.clone(),
            _ => return None,
        };

        match self.document.get_key(&dict, "PatternType").as_integer() {
            Some(1) => {
                return self
                    .tiling(&object, &dict, tint, state, fill)
                    .map(PatternPaint::Tiling);
            }
            Some(2) => {}
            other => {
                self.note(Unsupported::Shading {
                    name: format!("/{name} is pattern type {}", other.unwrap_or(0)),
                });
                return None;
            }
        }

        // A pattern is positioned relative to the page's default space, not to the
        // transform in force where it is used. Getting this wrong moves every gradient on
        // the page by whatever the current transform happened to be.
        let matrix = crate::shading::matrix_of(self.document, &dict, "Matrix");
        let shading_object = self.document.get_key(&dict, "Shading");

        match crate::shading::build(
            self.document,
            &shading_object,
            resources,
            matrix.then(self.base),
        ) {
            Ok(shading) => Some(PatternPaint::Shading(
                Arc::new(shading),
                // Stated "in the shading's target coordinate space", which for a pattern is
                // the pattern space — the shading's own `/Matrix` (type 1 only) is applied
                // inside `build` and comes *after* this.
                crate::shading::bbox_of(self.document, &shading_object)
                    .map(|corners| (corners, matrix.then(self.base))),
            )),
            Err(error) => {
                self.note(Unsupported::Shading {
                    name: format!("/{name}: {error}"),
                });
                None
            }
        }
    }

    /// Reads a tiling pattern's cell and how it repeats.
    fn tiling(
        &mut self,
        object: &Object,
        dict: &Dictionary,
        tint: &[f32],
        state: &GraphicsState,
        fill: bool,
    ) -> Option<Rc<Tiling>> {
        let stream = object.as_stream()?;
        let content = self.document.decoded_stream_data(stream)?;

        // `/XStep` and `/YStep` may differ from the cell's bounding box, which is how a
        // pattern tiles with gaps or with overlap. Zero would mean an infinite number of
        // cells in one place, so the specification forbids it and so does this.
        let step_x = self
            .document
            .get_key(dict, "XStep")
            .as_number()
            .map_or(0.0, narrow);
        let step_y = self
            .document
            .get_key(dict, "YStep")
            .as_number()
            .map_or(0.0, narrow);
        let bbox = self.document.get_key(dict, "BBox");
        let bbox: Vec<f32> = bbox
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| self.document.resolve(item).as_number().map(narrow))
                    .collect()
            })
            .unwrap_or_default();
        // A missing or zero step falls back to the cell's own size, which is what a
        // producer means by it and what other readers assume.
        let step = (
            non_zero(step_x).or_else(|| cell_extent(&bbox, 0))?,
            non_zero(step_y).or_else(|| cell_extent(&bbox, 1))?,
        );

        let resources = self
            .document
            .get_key(dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_default();

        // `/PaintType 2` describes a stencil rather than a picture: the cell carries no
        // colour and the current colour is poured through it.
        let tint = match self.document.get_key(dict, "PaintType").as_integer() {
            Some(2) => {
                let space = if fill {
                    &state.fill_space
                } else {
                    &state.stroke_space
                };
                // A bare `/Pattern` names no underlying space, so the operand count is the
                // only evidence of what the colour is — the same fallback `scn` uses when
                // a declared space and its operands disagree.
                let space = match space {
                    ColourSpace::Pattern { base: None } => match tint.len() {
                        3 => ColourSpace::Rgb,
                        4 => ColourSpace::Cmyk,
                        _ => ColourSpace::Gray,
                    },
                    other => other.clone(),
                };
                Some(space.to_rgb(tint))
            }
            _ => None,
        };

        Some(Rc::new(Tiling {
            content,
            resources,
            step,
            to_page: crate::shading::matrix_of(self.document, dict, "Matrix").then(self.base),
            tint,
        }))
    }

    /// Resolves a device colour space to what the document says it means.
    ///
    /// Three sources, in the order the specification puts them. A `/Default` entry in the
    /// resources §8.6.5.6 says *shall* be used. Failing that, the output intent describes
    /// the device the document's colours were prepared for, which §8.6.5.7 NOTE 3 names as
    /// the only thing in a PDF that can. Failing both, the device space itself — for which
    /// the specification defines no conversion at all, so what happens then is this
    /// processor's own choice and is documented as such in `colour.rs`.
    fn device_space(&self, name: &str, resources: &Dictionary) -> ColourSpace {
        let named = Object::Name(Name::new(name.as_bytes().to_vec()));
        if let Some(space) = ColourSpace::parse(self.document, &named, resources) {
            // `parse` returns the device space itself when no `/Default` entry replaces
            // it, so an output intent gets its turn only when nothing did.
            let replaced = !matches!(
                (&space, name),
                (ColourSpace::Gray, "DeviceGray")
                    | (ColourSpace::Rgb, "DeviceRGB")
                    | (ColourSpace::Cmyk, "DeviceCMYK")
            );
            if replaced {
                return space;
            }
        }

        if let Some(intent) = &self.output_intent
            && intent.components() == expected_components(name)
        {
            return intent.clone();
        }

        match name {
            "DeviceGray" => ColourSpace::Gray,
            "DeviceCMYK" => ColourSpace::Cmyk,
            _ => ColourSpace::Rgb,
        }
    }

    /// Looks up a named resource of a given category.
    fn resource(&self, resources: &Dictionary, category: &str, name: &str) -> Option<Object> {
        let table = self.document.get_key(resources, category);
        let table = table.as_dict()?;
        let value = table.get(name)?;
        Some(self.document.resolve(value))
    }

    /// A resource entry exactly as the file writes it, reference and all.
    ///
    /// Optional content is the one place where a resource's *identity* matters rather than
    /// its value. §8.11.2.2 requires an optional content group to be an indirect object, and
    /// `/OCProperties /OCGs` lists the document's groups by reference, so a group is
    /// recognised by which object it is. Resolving it first throws that away and leaves two
    /// identical-looking dictionaries indistinguishable.
    fn unresolved_resource(
        &self,
        resources: &Dictionary,
        category: &str,
        name: &str,
    ) -> Option<Object> {
        let table = self.document.get_key(resources, category);
        Some(table.as_dict()?.get(name)?.clone())
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

/// Converts a content-stream token into an operand.
fn token_to_object(token: pdf_syntax::Token) -> Object {
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
    // `[ 2 1 ] 0 d` arrives as five operands, the two brackets among them as nulls, because
    // the content lexer does not rebuild arrays. Splitting on them gives what is before the
    // opening bracket, the array itself, and what follows the closing one — the phase.
    let mut parts = operands.split(Object::is_null);
    let (Some(_), Some(inside), Some(after)) = (parts.next(), parts.next(), parts.next()) else {
        return;
    };

    let array: Vec<f32> = inside
        .iter()
        .filter_map(Object::as_number)
        .map(narrow)
        .collect();
    let phase = after
        .first()
        .and_then(Object::as_number)
        .map_or(0.0, narrow);
    apply_dash(array, phase, stroke);
}

/// Table 53's line cap style, ISO 32000-2 §8.4.3.3.
///
/// One function rather than a `match` beside each of the two operators that set it — `J` and
/// Table 57's `/LC` — because §8.4.1's NOTE 1 says a parameter "can be specified either way"
/// and the two ways have to mean the same thing. Three corpus documents set the cap through
/// `/LC` and it reached nothing at all until §8.4.3 was read as a family.
///
/// The clause defines 0, 1 and 2; §8.4.1 requires values "of the correct type or … within a
/// certain range", and the initial value is the answer for anything outside it.
fn line_cap(code: i64) -> LineCap {
    match code {
        1 => LineCap::Round,
        2 => LineCap::Square,
        _ => LineCap::Butt,
    }
}

/// Table 54's line join style, ISO 32000-2 §8.4.3.4. Set by `j` and by Table 57's `/LJ`.
fn line_join(code: i64) -> LineJoin {
    match code {
        1 => LineJoin::Round,
        2 => LineJoin::Bevel,
        _ => LineJoin::Miter,
    }
}

/// The miter limit, ISO 32000-2 §8.4.3.5. Set by `M` and by Table 57's `/ML`.
///
/// Clamped below at 1, which §8.4.1 asks for — "[p]arameters that are numeric values, such
/// as the current colour, line width, and miter limit, shall be clipped into valid range".
/// The valid range starts at 1 because the ratio the limit bounds is a miter length over a
/// line width, and §8.4.3.5's formula makes that ratio `1 / sin(φ/2)`, which is never below
/// one. A smaller limit would convert every join to a bevel, including a straight one.
fn miter_limit(limit: f32) -> f32 {
    limit.max(1.0)
}

/// Begins a subpath at `at`, ISO 32000-2 §8.5.2.1 Table 58's `m`.
///
/// Table 58's `m` overrides an `m` immediately before it, leaving
///
/// > no vestige of the previous m operation remains in the path.
///
/// Six corpus documents write consecutive `m` operators and one of them, `bug1743245.pdf`,
/// writes 205 of them on its first page. Keeping them would leave 205 single-point subpaths
/// in the path, and §8.5.3.2 turns a single-point subpath under round caps into a *dot* — so
/// the sentence above is what stands between that clause and 205 marks the file never asked
/// for. It also makes the only single-point open subpath a path can hold a *trailing* one,
/// which is exactly the shape §8.5.3.3.1 names.
fn begin_subpath(path: &mut Path, at: Point) {
    if matches!(path.commands().last(), Some(PathCommand::MoveTo(_))) {
        path.replace_last(PathCommand::MoveTo(at));
    } else {
        path.push(PathCommand::MoveTo(at));
    }
}

/// Closes the current subpath, ISO 32000-2 §8.5.2.1 Table 58's `h`.
///
/// > If the current subpath is already closed, h shall do nothing.
///
/// A subpath is already closed when the previous command closed it, and there is no current
/// subpath to close before the first `m`. A lone `m` followed by `h` is neither: it is
/// §8.5.3.2's single-point closed path, which is a mark rather than a no-op.
fn close_subpath(path: &mut Path) {
    match path.commands().last() {
        None | Some(PathCommand::Close) => {}
        Some(_) => path.push(PathCommand::Close),
    }
}

/// Drops a path's trailing single-point open subpath, ISO 32000-2 §8.5.3.3.1.
///
/// > Any subpaths that are open shall be implicitly closed before being filled, except that
/// > if the last subpath in the path is a single-point open subpath (specified by a trailing
/// > m operator), it shall be disregarded and not considered to be part of the path.
///
/// §8.5.3.2 says the same of stroking — "a single-point open subpath (specified by a
/// trailing m operator) shall produce no output" — and §8.5.4 defines a clip as the area
/// `f` would fill, so all three painting routes want the same thing and it is done once,
/// here, before any of them looks at the path.
///
/// Eleven corpus documents' first pages end a path this way. It changes no pixel by itself,
/// since a point encloses nothing; what it changes is that the path handed to the backends
/// no longer states a subpath the standard has just said is not part of it — and a path
/// consisting only of `m` becomes an empty path rather than one `tiny-skia` refuses.
fn drop_trailing_point(path: &mut Path) {
    if matches!(path.commands().last(), Some(PathCommand::MoveTo(_))) {
        path.pop();
    }
}

/// Puts a dash array and phase into the graphics state, ISO 32000-2 §8.4.3.6.
///
/// Shared by the `d` operator and an `/ExtGState`'s `/D` entry, which Table 57 defines as
/// the same pattern written as a real array. The two arrive in different shapes and mean the
/// same thing, and this is the one place that decides what a pattern means.
fn apply_dash(array: Vec<f32>, phase: f32, stroke: &mut Stroke) {
    // §8.4.3.6: "If the dash array is empty, the dash phase shall be zero and the path shall
    // be stroked with a solid, unbroken line."
    let total: f32 = array.iter().sum();
    // The same clause requires the elements to be "nonnegative and not all zero". A file
    // breaking that describes no pattern at all, so it is drawn solid — the one rendering
    // both remaining readings agree on — rather than left as whatever the previous `d` set.
    if array.is_empty() || total <= 0.0 || array.iter().any(|length| *length < 0.0) {
        stroke.dash_array.clear();
        stroke.dash_phase = 0.0;
        return;
    }

    // An odd-length array alternates on and off across its own end: `[3]` is three on, three
    // off. Repeating it once states the same pattern with an even length, which is what a
    // rasteriser's dash primitive takes, and does it here so that both backends receive one
    // meaning rather than each deriving it.
    stroke.dash_array = if array.len().is_multiple_of(2) {
        array
    } else {
        array.repeat(2)
    };
    // §8.4.3.6: "If the dash phase is negative, it shall be incremented by twice the sum of
    // all lengths in the dash array until it is positive." The pattern repeats with that
    // period, so one remainder is every increment the sentence asks for.
    stroke.dash_phase = if phase < 0.0 {
        phase.rem_euclid(total * 2.0)
    } else {
        phase
    };
}

/// Assigns a colour to the fill or stroke slot, along with the space that set it.
///
/// `g`, `rg` and `k` set a device space and a colour in one operator, so they replace
/// whatever `cs` had selected — including a pattern.
fn assign_colour(state: &mut GraphicsState, fill: bool, colour: Color, space: ColourSpace) {
    if fill {
        state.fill = colour;
        state.fill_space = space;
        state.fill_pattern = None;
    } else {
        state.stroke_colour = colour;
        state.stroke_space = space;
        state.stroke_pattern = None;
    }
}

/// The bounding box of a path once transformed, as `(min_x, min_y, max_x, max_y)`.
fn bounds_of(path: &Path, transform: Transform) -> Option<(f32, f32, f32, f32)> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    let mut visit = |point: Point| {
        let at = transform.apply(point);
        if !at.x.is_finite() || !at.y.is_finite() {
            return;
        }
        bounds = Some(match bounds {
            None => (at.x, at.y, at.x, at.y),
            Some((x0, y0, x1, y1)) => (x0.min(at.x), y0.min(at.y), x1.max(at.x), y1.max(at.y)),
        });
    };
    for command in path.commands() {
        match command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => visit(*point),
            // A curve stays inside the hull of its control points, so those bound it —
            // loosely, which only ever draws tiles that turn out to be clipped away.
            PathCommand::CurveTo(a, b, c) => {
                visit(*a);
                visit(*b);
                visit(*c);
            }
            PathCommand::Close => {}
        }
    }
    bounds
}

/// The range of tile indices covering an interval, given a step.
fn span(low: f32, high: f32, step: f32) -> (i32, i32) {
    /// Bounds the index range so a huge path or a tiny step cannot overflow.
    const LIMIT: f32 = 1e6;

    let first = (low / step).floor().clamp(-LIMIT, LIMIT);
    let last = (high / step).ceil().clamp(-LIMIT, LIMIT);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "both are clamped to a million, well inside i32"
    )]
    {
        (first as i32, last as i32)
    }
}

/// Widens a tile index for arithmetic in pattern space.
fn as_f32(index: i32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "tile indices are clamped to a million, exact in f32"
    )]
    {
        index as f32
    }
}

/// How many components a device space's colours have.
fn expected_components(name: &str) -> usize {
    match name {
        "DeviceGray" => 1,
        "DeviceCMYK" => 4,
        _ => 3,
    }
}

/// Converts a colour, honouring the graphics state's black point setting.
fn convert(space: &ColourSpace, values: &[f32], black_point: BlackPoint) -> Color {
    if black_point.applies() {
        space.to_rgb(values)
    } else {
        space.to_rgb_without_black_point(values)
    }
}

/// Reads the colour space a document's output intent describes.
///
/// Only a profile whose own space is one a PDF can name is useful here; an output intent
/// for a device with some other colourant model says nothing about `DeviceCMYK`.
fn output_intent_space(document: &Document) -> Option<ColourSpace> {
    let catalog = document.catalog().ok()?;
    let intents = document.get_key(&catalog, "OutputIntents");
    // The specification is explicit that PDF carries no selector for choosing among
    // several, so the first usable one is taken.
    for intent in intents.as_array()? {
        let intent = document.resolve(intent);
        let Some(dict) = intent.as_dict() else {
            continue;
        };
        let profile = document.get_key(dict, "DestOutputProfile");
        let Some(stream) = profile.as_stream() else {
            continue;
        };
        if let Some(data) = document.decoded_stream_data(stream)
            && let Some(parsed) = crate::icc::Profile::parse(&data)
        {
            return Some(ColourSpace::Icc {
                profile: Box::new(parsed),
            });
        }
    }
    None
}

/// Returns a step only if it is usable as one.
///
/// A zero step would place every cell on top of the last, which is an infinite loop rather
/// than a pattern; the specification forbids it. A negative one is legal and tiles in the
/// other direction, so only its magnitude matters here.
fn non_zero(step: f32) -> Option<f32> {
    let step = step.abs();
    (step.is_finite() && step > 0.0).then_some(step)
}

/// The width or height of a pattern cell's bounding box, as a fallback step.
fn cell_extent(bbox: &[f32], axis: usize) -> Option<f32> {
    let low = bbox.get(axis)?;
    let high = bbox.get(axis.checked_add(2)?)?;
    non_zero(high - low)
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

/// A shading with a constant alpha applied, sharing the original where it is opaque.
///
/// The share is the common case and the one worth keeping cheap: a pattern set once paints
/// every path filled until the colour changes again, and copying its 256-sample ramp — or a
/// mesh's triangles — per fill would be a copy per path for nothing.
fn shading_with_alpha(shading: &Arc<Shading>, alpha: f32) -> Arc<Shading> {
    if alpha < 1.0 {
        Arc::new(shading.with_alpha(alpha))
    } else {
        Arc::clone(shading)
    }
}

/// Whether a paint puts anything on the page.
///
/// A shading always does — its own colours decide where, and a shading with no coverage is a
/// question for the rasteriser rather than for a report. A solid colour with zero alpha does
/// not, which is what `1 0 0 rg /GS gs` with a `ca` of 0 amounts to: a part of an object that
/// paints nothing cannot composite with the part that does.
fn marks(paint: &Paint) -> bool {
    match paint {
        Paint::Solid(colour) => colour.a > 0.0,
        // `Paint` is `#[non_exhaustive]`, and a paint this function has not been taught about
        // is one that may well mark the page — which is the safe direction for a report.
        _ => true,
    }
}

/// Maps a PDF blend mode name, taking `Normal` for anything this reader does not know.
///
/// `Normal` and `Compatible` are the two names that mean it deliberately; §11.6.3 asks for the
/// same answer for an unrecognised one, which is why the two cases can share an arm here and
/// have to be told apart by [`known_blend_mode`] when an array is choosing between names.
fn blend_mode(name: &[u8]) -> BlendMode {
    known_blend_mode(name).unwrap_or(BlendMode::Normal)
}

/// Maps a PDF blend mode name, or `None` where the name is not one Table 134 or 135 lists.
fn known_blend_mode(name: &[u8]) -> Option<BlendMode> {
    Some(match name {
        b"Normal" | b"Compatible" => BlendMode::Normal,
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
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use pdf_render::Point;

    use super::{base_transform, rotated_size};
    use crate::page::Page;

    /// A page 400 wide and 200 tall, with no crop offset, at `rotate` degrees.
    fn landscape(rotate: u16) -> Page {
        Page {
            dict: pdf_syntax::Dictionary::default(),
            resources: pdf_syntax::Dictionary::default(),
            media_box: [0.0, 0.0, 400.0, 200.0],
            crop_box: [0.0, 0.0, 400.0, 200.0],
            rotate,
            user_unit: 1.0,
        }
    }

    /// ISO 32000-2 §7.7.3.3 Table 31: `/Rotate` is "the number of degrees by which the page
    /// shall be rotated **clockwise** when displayed".
    ///
    /// Clockwise *as displayed*, and this space is y-up, so the check is written in terms of
    /// where a corner ends up rather than in terms of a matrix — a matrix can be transcribed
    /// wrongly and still look like the right kind of thing, which is exactly what happened
    /// here for eleven sessions.
    ///
    /// The user-space point checked is the page's **top-left** corner, `(0, H)`. Turn a sheet
    /// of paper 90° clockwise and its top-left corner becomes the *top-right* one, which in
    /// this y-up space with the rotated page `H` wide and `W` tall is `(H, W)`. Turn it 270°
    /// clockwise and the same corner becomes the bottom-left, `(0, 0)`.
    ///
    /// This test was confirmed to fail with the 90 and 270 matrices exchanged, which is how
    /// they stood until the twelfth session.
    #[test]
    fn rotation_turns_the_page_clockwise_as_displayed() {
        let (width, height) = (400.0_f32, 200.0_f32);
        let top_left = Point::new(0.0, height);

        let unrotated = base_transform(&landscape(0)).apply(top_left);
        assert_eq!((unrotated.x, unrotated.y), (0.0, height), "0 degrees");

        // Clockwise: the top-left corner becomes the top-right of a page that is now
        // `height` wide and `width` tall.
        let quarter = base_transform(&landscape(90)).apply(top_left);
        assert_eq!((quarter.x, quarter.y), (height, width), "90 degrees");

        let half = base_transform(&landscape(180)).apply(top_left);
        assert_eq!((half.x, half.y), (width, 0.0), "180 degrees");

        // Three quarters clockwise puts it at the origin.
        let three_quarters = base_transform(&landscape(270)).apply(top_left);
        assert_eq!(
            (three_quarters.x, three_quarters.y),
            (0.0, 0.0),
            "270 degrees"
        );
    }

    /// Every corner of the page must land inside the rotated page, at every angle.
    ///
    /// The corner test above pins the direction; this pins that the translation which brings
    /// a rotation back into the positive quadrant is the right one. A sign error in either
    /// would otherwise put content off the page, where a comparison sees a blank sheet and
    /// reports a difference without saying it was a placement.
    #[test]
    fn every_corner_lands_inside_the_rotated_page() {
        for rotate in [0, 90, 180, 270] {
            let page = landscape(rotate);
            let size = rotated_size(&page);
            let transform = base_transform(&page);
            for corner in [
                Point::new(0.0, 0.0),
                Point::new(400.0, 0.0),
                Point::new(0.0, 200.0),
                Point::new(400.0, 200.0),
            ] {
                let mapped = transform.apply(corner);
                assert!(
                    (0.0..=size.width).contains(&mapped.x)
                        && (0.0..=size.height).contains(&mapped.y),
                    "rotate {rotate}: {corner:?} landed at {mapped:?}, outside {size:?}"
                );
            }
        }
    }
}
