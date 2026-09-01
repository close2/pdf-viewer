//! Interpreting a content stream into a resolved display list.
//!
//! This is where PDF's graphics state machine is executed, and executed *once*. Every
//! command that comes out carries its absolute transform and an explicit clip, so the
//! backends contain no PDF semantics at all — which is what lets the CPU backend serve as
//! an oracle for the GPU one. See `pdf-render`.
//!
//! # Unsupported content is reported, never silently dropped
//!
//! Ignoring what cannot be drawn would produce a page that looks plausible and is wrong, which
//! is the single most dangerous failure mode for a viewer — and it would make the comparison
//! harness report a pass on a page missing half its content. (This said "[t]ext and images are
//! not yet drawn" until the two-hundred-and-twenty-first session, having been true of the sixth.)
//!
//! So [`Interpretation`] carries a list of what it could not draw. A caller can render the
//! partial page *and* know it is partial: the viewer can say so, and the harness can
//! exclude the page from comparison rather than reporting a false difference.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_render::{
    BlendMode, ClipId, Color, DisplayList, LineCap, LineJoin, Paint, Point, Rect, Size, SoftMaskId,
    Stroke, Transform,
};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::colour::{ColourSpace, Compositing};
use crate::page::Page;

use colour::{BlackPoint, Intent, output_intent_space};
pub use ext_gstate::Transfer;
use ext_gstate::TransferState;
use font::{Font, FontKey};
use pattern::{PatternInitial, PatternPaint};
use run::narrow;
use text::Coverage;
use transparency::{AlphaSourcesSeen, PagePress, page_blending_space, page_press};

mod annotations;
mod colour;
mod ext_gstate;
mod font;
mod image;
mod marked;
mod path;
mod pattern;
pub mod reader;
mod report;
mod resources;
mod run;
mod text;
mod transparency;
mod xobject;

pub use font::{FONT_BUDGET, FontCache, FontCacheReport};
pub use report::{
    ArtifactSpan, ContentStream, DamagedStream, Interpretation, MarkedSpan, Placed, Shortfall,
    UnnamedCodes, Unsupported, named_sequences,
};

/// Deepest nesting of `q`/`Q` that will be tracked.
///
/// Legitimate content nests a few levels. A stream with thousands of unmatched `q`
/// operators is either broken or hostile, and each level costs a saved state — the whole
/// `GraphicsState`, including `Stroke::dash_array`, which a content stream can make large.
///
/// **ISO 32000-2 §C.2's Table C.1 is the only place the standard prints a figure for this**,
/// and it is the reason 256 rather than an argument about what looks generous. The *Nested
/// objects* row anticipates the bound —
///
/// > However PDF processors may implement recursive algorithms which may cause issues for
/// > excessively nested constructs.
///
/// — and its NOTE says how deep a writer could rely on:
///
/// > In previous versions of PDF, a maximum depth of graphics state nesting by q and Q
/// > operators was 28.
///
/// Annex C is informative, so neither sentence binds; what they settle is that 256 is nine
/// times the standard's own figure. **One document of the 65 944 crawled ones surveyed in the
/// four-hundred-and-thirty-third session reaches this bound and it wants 337** — twelve times
/// that figure — which is why the four-hundred-and-thirty-fifth left the number alone rather
/// than moving it to admit one file. ADR 0271, `tests/hostile_budgets.rs`.
const MAX_STATE_DEPTH: usize = 256;

/// Most operators executed for one page.
///
/// A content stream is a program, and this bounds how long it may run. Without it a
/// compressed stream of a few kilobytes can expand into tens of millions of operations —
/// a decompression bomb aimed at the renderer rather than at memory.
///
/// **It said "operators" and counted lexer tokens for its whole life, and the value has not
/// moved: the unit has.** §7.8.2 puts an operator after its operands — "all of the operands
/// needed by an operator shall immediately precede that operator" — so `x1 y1 x2 y2 x3 y3 c` is
/// seven tokens and one operator, and the single increment site was the token loop. For a
/// hand-traced drawing that is a budget about seven times tighter than this constant advertises,
/// and the project owner's witness was truncated at 19% of its artwork while stating 814 705
/// *fewer* operators than the bound. The counter now increments where the interpreter knows it
/// has an operator, which is after the keyword arm and before the dispatch.
///
/// **What that costs, measured rather than argued.** Over **926 680 pages of 65 967 crawled
/// documents** (`cargo run --release -p pdf-model --example content_budget_census`), 48 pages
/// state more than four million lexer tokens and **8** state more than four million operators;
/// the corpus-wide ratio is 3.76 tokens per operator and is not a constant — it is about 2 for
/// text and about 7 for cubic Béziers. So correcting the unit hands back forty pages of a
/// million and refuses the eight that really are programs of that length.
///
/// **It is a bound on slowness rather than on exhaustion, and the four-hundred-and-thirty-fifth
/// session opened the documents that reach it to find that out.** All 31 of 65 944 that did
/// *terminate* when it was lifted a hundredfold — they are maps, plans and charts rather than
/// bombs. The bound stays at four million for the reason a raised one would not help: **a count
/// is not a cost.** One `sh` can paint the whole page, so no number here bounds the time, and
/// what actually bounds it is the confined worker's cancel — a kill, at 0.83–1.97 ms (ADR
/// 0241). ADRs 0271 and 0306.
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
///
/// **Every document that reaches this bound is such a cycle**, which the
/// four-hundred-and-thirty-fifth session established by lifting it sixteenfold to 256 in a
/// scratch build and running the four of the 65 944-document `SafeDocs` crawl that reported
/// it: all four reached 256 as well. **The eight-hundred-and-fifty-seventh ran the same
/// experiment over a population that did not exist then** — the Tika issue-tracker corpus's
/// 6835-document Mozilla directory, where **seven** documents report it — and all seven
/// reach 256 too. Two corpora, eleven documents, no legitimate one among them. So this is
/// the one of the four bounds whose population is entirely the attack it
/// exists for, and it is also the one nothing else could catch — unbounded recursion exhausts
/// the *stack*, which the confined worker's address-space ceiling does not see and which Rust
/// turns into an abort rather than into a report. ISO 32000-2 §C.2's Table C.1 lists
/// `XObject`s beside `q`/`Q` in its *Nested objects* row and leaves the depth to the
/// processor. ADR 0271.
const MAX_FORM_DEPTH: usize = 16;

/// Deepest nesting of soft-mask groups.
///
/// A mask's group is a content stream like any other, so it may set a soft mask of its own
/// — including, in a file that is broken or hostile, one whose `/G` is the group being
/// evaluated. That is a cycle the document controls, and this is what makes it terminate.
/// Four levels is far past anything a producer writes and cheap to allow: each level costs
/// a whole group's commands.
const MAX_SOFT_MASK_DEPTH: usize = 4;

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
    /// Table 57's `/UseBlackPtComp`, which says whether black point compensation applies to
    /// CIE-based conversions.
    ///
    /// ISO 32000-2 §8.6.5.9. `Default` is the initial value and leaves the choice to the
    /// processor; this one compensates, which is what makes blacks black. Read through
    /// [`GraphicsState::black_point`] rather than directly, because the rendering intent can
    /// override it.
    use_black_pt_comp: BlackPoint,
    /// The rendering intent parameter, set by `ri` and by `/RI` (§8.6.5.8).
    ///
    /// §8.4.1 Table 52 states the initial value, and it is not the absent answer a `bool` would
    /// have given: an object painted before any `ri` has an intent, and it is a named one.
    ///
    /// > Initial value: RelativeColorimetric .
    intent: Intent,
    /// The current fill colour space, which decides how `sc`/`scn` operands are read.
    fill_space: ColourSpace,
    /// As above, for stroking.
    stroke_space: ColourSpace,
    /// §10.5's transfer function, where an `/ExtGState` sets one — by either of the clause's
    /// two routes, which is what [`TransferState`] holds apart and composes.
    ///
    /// Empty is the initial value and what `/Identity`, `/TR2`'s `/Default` and `/HT`'s `/Default`
    /// restore between them. Saved and restored by `q`/`Q` like every other parameter here, and
    /// inherited by a form `XObject` and by a tiling pattern's replay, which is what §8.4's
    /// "graphics state" means.
    ///
    /// **This is what the file states, not what a mark is painted with.** §11.7.5.2 chooses
    /// between it and the page's default per point, and [`Interpreter::transfer_for_mark`] is the
    /// single place that asks — every other reader of this field takes the answer from there.
    transfer: TransferState,
    /// Table 57's `/SM`, §10.7.3's smoothness tolerance, if the file states one.
    ///
    /// `None` is the initial value in the sense that matters: no document has asked for
    /// anything, so this device's own resolution stands. See `Ramp::resolution_for`.
    smoothness: Option<f32>,
    /// §11.6.4.3's `/AIS`, Table 57's alpha source flag: whether the soft mask and the two
    /// alpha constants state *shape* rather than opacity.
    ///
    /// A graphics state parameter like any other in this struct — set by `gs`, saved and
    /// restored by `q`/`Q` — and carried here so that [`Interpreter::alpha_sources`] can be
    /// seeded with the value actually in force when a group's content starts, rather than
    /// with the whole page's history. Initially `false` (§8.4.1 Table 52).
    alpha_is_shape: bool,
    /// Text state, which `q`/`Q` saves and restores along with everything else.
    text: TextState,
}

/// The text-related part of the graphics state.
///
/// Separate from [`TextObject`], which the specification resets at every `BT` and which
/// therefore does not survive `q`/`Q`.
#[derive(Debug, Clone)]
struct TextState {
    /// The resource name of the current font, and the font itself once loaded.
    font: Option<Font>,
    /// The `/Font` resource name `Tf` last selected, for a report that has to name it.
    font_name: String,
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
            font_name: String::new(),
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
    /// The initial state defined by ISO 32000-2 §8.4.
    fn initial(base: Transform) -> Self {
        Self {
            transform: base,
            clip: None,
            soft_mask: None,
            transfer: TransferState::default(),
            smoothness: None,
            alpha_is_shape: false,
            fill: Color::BLACK,
            fill_pattern: None,
            stroke_pattern: None,
            stroke_colour: Color::BLACK,
            stroke: Stroke::default(),
            blend: BlendMode::Normal,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            use_black_pt_comp: BlackPoint::Default,
            intent: Intent::Relative,
            fill_space: ColourSpace::Gray,
            stroke_space: ColourSpace::Gray,
            text: TextState::default(),
        }
    }

    /// Whether §8.6.5.9's black point compensation applies to an object painted now.
    ///
    /// The clause states the override over the object rather than over the entry:
    ///
    /// > If the current render intent of an object is AbsColorimetric then the value of
    /// > UseBlackPtComp shall be treated as OFF .
    ///
    /// So it is asked here, where an object's colour is converted, rather than performed as an
    /// assignment when either parameter is set. **The difference is two orderings, and both
    /// were wrong until the six-hundred-and-seventh session**, when the two parameters shared
    /// one field: a `ri` naming any other intent used to reset an explicit `/UseBlackPtComp
    /// OFF` back to compensating, and a `/UseBlackPtComp ON` set *after* an absolute intent
    /// used to compensate although the intent still in force says it shall not.
    fn black_point(&self) -> BlackPoint {
        self.black_point_under(self.intent)
    }

    /// As [`GraphicsState::black_point`], for an object that states an intent of its own.
    ///
    /// The clause says *the current render intent of an object*, and §8.6.5.8 gives an object
    /// three routes to one: the `ri` operator, an `/ExtGState`'s `/RI`, and §8.9.5.1 Table 87's
    /// `/Intent`, which is an image's own. Only the third can differ from the state's, which is
    /// why this takes the intent as an argument rather than reading it.
    fn black_point_under(&self, intent: Intent) -> BlackPoint {
        if intent == Intent::Absolute {
            // Absolute colorimetry reproduces the source's measured colours, including its own
            // paper white and black; compensating for the black point would defeat that.
            return BlackPoint::Off;
        }
        self.use_black_pt_comp
    }

    /// Returns the fill colour with the constant alpha applied.
    ///
    /// A *solid* colour only. A shading pattern replaces the colour entirely and PDF has no
    /// notion of tinting one, but its colours are the mark's to resolve — see
    /// [`Interpreter::fill_paint`], which is what every painting operator calls and which comes
    /// back here for the colours a graphics state can answer on its own. A tiling pattern is not
    /// a paint at all: it is drawn by replaying its content stream, so it leaves the colour
    /// alone.
    /// `transfer` is §11.7.5.2's answer for the mark being painted rather than this state's own
    /// parameter, which is why it arrives as an argument: see [`Interpreter::transfer_for_mark`],
    /// the one place that decides which of the two a mark gets.
    fn solid_fill(&self, transfer: Option<&Transfer>) -> Paint {
        // §10.5's transfer function, applied here because here is where a colour becomes the
        // value a device receives: the clause puts it "after performing any needed conversions
        // between colour spaces", and by this point `fill` is already RGB.
        Paint::Solid(transferred(
            Color {
                a: self.fill.a * self.fill_alpha,
                ..self.fill
            },
            transfer,
        ))
    }

    /// Whether a non-stroking mark under this state puts anything on the page.
    ///
    /// §11.6.2's and §11.7.4.4's reports both need this and neither needs a *paint*, which is
    /// why it is asked here rather than through [`Interpreter::fill_paint`]: building one now
    /// would resolve a shading pattern's colours for a question whose answer is yes for every
    /// pattern there is. A shading marks where its own colours say — where it does not is the
    /// rasteriser's question, not a report's — and a tiling pattern is a cell replayed across
    /// the area, which marks whatever the cell marks.
    /// `None` for the transfer function: §10.5 gives "[e]ach colour component … its own separate
    /// transfer function" and touches no alpha, and [`path::marks`] asks about alpha alone — so
    /// the answer is the same whichever function §11.7.5.2 turns out to choose for the mark, and
    /// resolving that here would ask the question twice.
    fn fill_marks(&self) -> bool {
        self.fill_pattern.is_some() || path::marks(&self.solid_fill(None))
    }

    /// As [`GraphicsState::fill_marks`], for a stroking mark.
    fn stroke_marks(&self) -> bool {
        self.stroke_pattern.is_some() || path::marks(&self.solid_stroke(None))
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

    /// Returns the stroke colour with the constant alpha applied, as [`GraphicsState::solid_fill`].
    fn solid_stroke(&self, transfer: Option<&Transfer>) -> Paint {
        Paint::Solid(transferred(
            Color {
                a: self.stroke_colour.a * self.stroke_alpha,
                ..self.stroke_colour
            },
            transfer,
        ))
    }
}

/// One colour through §10.5's transfer function, or unchanged where the mark is handed none.
fn transferred(colour: Color, transfer: Option<&Transfer>) -> Color {
    transfer.map_or(colour, |transfer| transfer.apply(colour))
}

/// Interprets a page's content into a display list.
///
/// The returned list is in PDF user space with the page's crop box at the origin, so a
/// backend applies only the device transform. Page rotation is folded in here, because it
/// is a property of the page rather than of the device.
#[must_use]
pub fn interpret(document: &Document, page: &Page) -> Interpretation {
    interpret_with(document, page, &crate::view::ViewState::of(document))
}

/// Interprets a page against a viewer state §12.6.4's actions have moved.
///
/// The same as [`interpret`] except that the optional content groups' states and the
/// annotations' Hidden flags come from `state` rather than from the file alone — which is
/// what §12.6.4.13 and §12.6.4.11 change, and what §8.11.4.5 says a manual change does:
/// "Manual changes shall override the states that were set automatically."
///
/// [`interpret`] is this function with the state the document opens in, so the two cannot
/// diverge and a caller that never performs an action pays nothing but one struct.
#[must_use]
pub fn interpret_with(
    document: &Document,
    page: &Page,
    state: &crate::view::ViewState,
) -> Interpretation {
    interpret_with_fonts(document, page, state, &FontCache::new())
}

/// Interprets a page, reusing the fonts an earlier interpretation of the same document loaded.
///
/// The same as [`interpret_with`] except that §9.6's font loading may be answered out of
/// `fonts` and adds to it. **The `Interpretation` is the same one either way**, and that is a
/// property to state rather than a hope: a loaded font is a pure function of the document and
/// the font dictionary, an [`ObjectId`](pdf_syntax::ObjectId) names one dictionary in one file,
/// and [`FontCache`] refuses to answer for a document other than the one it was filled from.
/// `content::tests::a_kept_font_changes_what_a_page_costs_and_not_what_it_says` is what checks
/// it, over the corpus's multi-page documents, against the planted defect it would take to
/// break it.
///
/// [`interpret_with`] is this function with a cache of its own, which is why §11.4.7's
/// subtractive pair below shares one: the two runs of one page load one set of fonts between
/// them rather than two. ADR 0710 has the measurements.
#[must_use]
pub fn interpret_with_fonts(
    document: &Document,
    page: &Page,
    state: &crate::view::ViewState,
    fonts: &FontCache,
) -> Interpretation {
    interpreted(document, page, state, fonts, Keep::Nothing).0
}

/// Interprets a page and keeps what re-running §12.5.3's annotation pass would need.
///
/// The same [`Interpretation`] [`interpret_with_fonts`] produces, plus — for a page that carries
/// an annotation whose placement §12.5.3 makes a function of the magnification — a
/// [`Replacement`] that [`replace`] turns into the same page at another magnification without
/// reading a byte of its content stream again. `None` is a page the clause has nothing to say
/// about, and a caller that gets one re-interprets or does nothing at all.
///
/// **The replacement is against this `state`**, and that is a precondition rather than a
/// courtesy: everything but the magnification — §8.11's optional content, §12.7.5's field values,
/// §12.5.5's appearance under the pointer, §6.3.2.2's delegated widgets — was resolved while the
/// content half ran and is baked into what this carries. A caller that changes any of those
/// throws the replacement away and interprets again; `viewer_core::Open::stale` is that one
/// place, and `viewer_core::Open::reinterpret` is the one that keeps it.
///
/// [`interpret_with_fonts`] is this function with the replacement dropped, so the two cannot
/// diverge.
#[must_use]
pub fn interpret_replaceable(
    document: &Document,
    page: &Page,
    state: &crate::view::ViewState,
    fonts: &FontCache,
) -> (Interpretation, Option<Replacement>) {
    interpreted(document, page, state, fonts, Keep::Replacement)
}

/// Runs §12.5.3's annotation pass again, over the content half a [`Replacement`] kept.
///
/// The page's content stream is not read: `draw_annotations` runs last, so everything it
/// contributes is a tail on every one of the interpreter's accumulators, and this restores the
/// state at that seam and runs the tail against `state`'s magnification. The result is the
/// [`Interpretation`] a whole re-interpretation would have produced —
/// `a_replaced_page_is_the_page_it_would_have_been_interpreted_as` checks that field by field
/// over every corpus page the clause is about.
///
/// `state` must be the one the replacement was made against but for
/// [`ViewState::set_magnification`](crate::view::ViewState::set_magnification); see
/// [`interpret_replaceable`].
#[must_use]
pub fn replace(
    document: &Document,
    page: &Page,
    state: &crate::view::ViewState,
    fonts: &FontCache,
    replacement: &Replacement,
) -> Interpretation {
    let mut interpreter = Interpreter::for_page(
        document,
        page,
        state,
        replacement.checkpoint.compositing.clone(),
        &replacement.presses,
        fonts,
    );
    interpreter.restore(replacement.checkpoint.clone());
    complete(document, page, base_transform(page), interpreter).0
}

/// Whether an interpretation is asked to keep what [`replace`] would need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Keep {
    /// Nothing: the caller will interpret the page again if it needs it again.
    Nothing,
    /// The content half, where §12.5.3 makes this page's placement depend on the magnification.
    Replacement,
}

/// What [`replace`] needs to put §12.5.3's annotations somewhere else on an already-interpreted
/// page.
///
/// Opaque on purpose: what it holds is decided by the compiler rather than by an interface, and
/// naming a field here would be a second place for that decision to live. `doc/todo/46` has the
/// two constructions this is the cheaper of, and ADR 0777 the measurement that chose it.
#[derive(Debug, Clone)]
pub struct Replacement {
    /// The interpreter's own accumulated state at the seam.
    checkpoint: Checkpoint,
    /// §11.4.7's press table, shared with the run that made this so that a press this page named
    /// is sampled once and counted once against [`crate::colour::MAX_PRESSES`] however many times
    /// the annotations are re-placed. ADR 0417 is why that budget is per interpretation.
    presses: Arc<crate::colour::Presses>,
}

/// One interpretation of a page, and the replacement it was asked to keep.
fn interpreted(
    document: &Document,
    page: &Page,
    state: &crate::view::ViewState,
    fonts: &FontCache,
    keep: Keep,
) -> (Interpretation, Option<Replacement>) {
    // ISO 32000-2 §11.4.7 puts a colour space under the whole page — "[a]ll page-level
    // compositing shall be done in the default blending colour space of the page" — and where
    // that space is `DeviceCMYK` this tree draws the page in it rather than on the device's
    // three components. §11.3.4 makes the compositing formula per component, so four
    // components are three plus one: the page is interpreted twice, once carrying cyan,
    // magenta and yellow and once carrying black, and `pdf_render::blending` puts the two
    // rasters back together where the clause puts the conversion. ADR 0262.
    // One table for the whole interpretation, so that the two runs of the pair below name the
    // same press once between them and §11.7.2's refusal is a fact about this page rather than
    // about what the process opened before it. ADR 0417.
    let presses = Arc::new(crate::colour::Presses::default());
    // **§11.4.7's pair is interpreted whole, every time, and that is the documented fallback.**
    // A page drawn in its own subtractive space is two interpretations of one content stream
    // merged by geometry digest, so the seam below is two seams and the merge is a third thing to
    // get right — and `examples/press_census` says how rare such a page is. So a page in this
    // branch keeps no replacement and a zoom of it re-interprets, at the price this branch
    // already pays twice over. `doc/todo/46` asked for this decision and for its cost in writing.
    if let PagePress::In(press) = page_press(document, page, &presses) {
        let (chromatic, drawable, _) = interpret_into(
            document,
            page,
            state,
            Compositing::Subtractive(crate::colour::Half::Chromatic, Arc::clone(&press)),
            &presses,
            fonts,
            Keep::Nothing,
        );
        if drawable {
            let (black, _, _) = interpret_into(
                document,
                page,
                state,
                Compositing::Subtractive(crate::colour::Half::Black, Arc::clone(&press)),
                &presses,
                fonts,
                Keep::Nothing,
            );
            // The two runs differ only in what a colour resolves to, so their geometry is
            // identical by construction — and this is what checks it, because the halves are
            // put together per pixel and a command in one and not the other would be
            // composited against a shape that never drew it. A mismatch falls through to the
            // device's components and the report, which is the answer that was right before
            // this round and is still right.
            let mut chromatic = chromatic;
            if chromatic.display_list.geometry_digest() == black.display_list.geometry_digest() {
                chromatic
                    .display_list
                    .set_blending(press.blending_space(), black.display_list);
                return (chromatic, None);
            }
        }
    }
    let (interpretation, _, checkpoint) = interpret_into(
        document,
        page,
        state,
        Compositing::Device,
        &presses,
        fonts,
        keep,
    );
    // A checkpoint the pass then found nothing to use is dropped rather than kept: the seam's
    // condition is Table 167's bit read off the file, and `draw_annotations` may decline to draw
    // the annotation that set it (§12.5.3's Hidden, §8.11.3.3's `/OC`, §12.6.4.11's action,
    // §6.3.2.2's delegation), in which case the page is not view-dependent after all and a
    // replacement of it would be a second copy of a list nothing will ask for.
    let replacement = checkpoint
        .filter(|_| interpretation.view_dependent)
        .map(|checkpoint| Replacement {
            checkpoint,
            presses,
        });
    (interpretation, replacement)
}

impl<'a> Interpreter<'a> {
    /// The interpreter one page starts with, before a byte of its content stream is read.
    ///
    /// Separate from [`interpret_into`] so that the page's own preparation — what a host has
    /// instructed about §12.7's widgets, what §11.4.7 gives the page to composite in — reads
    /// as the one list of answers it is, rather than as the opening third of the function
    /// that then runs the page.
    fn for_page(
        document: &'a Document,
        page: &Page,
        state: &'a crate::view::ViewState,
        compositing: Compositing,
        presses: &'a crate::colour::Presses,
        across: &'a FontCache,
    ) -> Self {
        let size = displayed_size(page);
        // §6.3.2.2's "unless otherwise instructed", asked once per page and only where a host
        // has instructed: a document nobody has said this about pays one enum comparison, and
        // one that has pays a walk of §12.7.4.1's field tree — the same walk `Query::Fields`
        // already makes for the same page, which is what keeps the two sets identical rather
        // than similar.
        let beyond = match page_press(document, page, presses) {
            PagePress::Beyond(beyond) => Some(beyond),
            PagePress::Device | PagePress::In(_) => None,
        };
        let delegated = match state.widget_appearances() {
            crate::view::WidgetAppearances::Drawn => BTreeSet::new(),
            crate::view::WidgetAppearances::Delegated => {
                crate::form::delegated_widgets(document, page, state)
            }
        };
        Self {
            document,
            list: DisplayList::new(size),
            unsupported: BTreeMap::new(),
            notes_raised: 0,
            text_operations: 0,
            segments_without_a_current_point: 0,
            glyph_coverage: BTreeMap::new(),
            glyphs: 0,
            codes_without_a_glyph: 0,
            codes_reaching_a_blank_glyph: 0,
            codes_without_a_vertical_form: 0,
            codes_without_a_character: UnnamedCodes::default(),
            operations: 0,
            fonts: BTreeMap::new(),
            across,
            text: String::new(),
            described: Vec::new(),
            artifacts: Vec::new(),
            marked: Vec::new(),
            stream: ContentStream::Page,
            marking: Vec::new(),
            clip_extents: Vec::new(),
            inferred_separators: 0,
            text_layer: Vec::new(),
            associated: Vec::new(),
            reversed_chars: 0,
            view_dependent: false,
            text_cursor: None,
            base: base_transform(page),
            // §11.6.7's companion to `base`, and initialised from the same place: nothing has run
            // before a page's content stream, so the parameters it begins with are §8.4.1
            // Table 52's initial values.
            pattern_initial: PatternInitial::of(&GraphicsState::initial(base_transform(page))),
            page: size,
            shadings: crate::shading::Cache::default(),
            resource_tables: std::cell::RefCell::default(),
            icc_spaces: BTreeMap::new(),
            image_masks: crate::image::MaskCache::default(),
            image_rasters: crate::image::RasterCache::default(),
            structure: Arc::new(crate::structure::ParentTree::for_page(document, &page.dict)),
            stream_structures: BTreeMap::new(),
            output_intent: output_intent_space(document),
            optional_content: state.optional_content().cloned(),
            view: state,
            delegated,
            hidden: 0,
            glyph_depth: 0,
            soft_mask_depth: 0,
            uncoloured: false,
            inside_knockout: false,
            transparent_initial_backdrop: false,
            // §8.4.1 Table 52 gives the alpha source parameter an initial value of `false`,
            // so a page that never states `gs` paints entirely under §11.6.4.3's opacity
            // reading.
            alpha_sources: AlphaSourcesSeen::Opacity,
            alpha_sources_mark: 0,
            compositing,
            blending: page_blending_space(document, page),
            blending_changed: false,
            black_generation_stated: false,
            // Nothing encloses the page's own content stream, so §11.7.5.2's fifth and sixth
            // conditions hold vacuously until a `Do` or a pattern fill narrows them.
            opaque_ancestry: true,
            transfer_painted_opaquely: false,
            nested_space_departed: false,
            presses,
            blending_beyond: beyond,
        }
    }

    /// Everything this interpretation has accumulated, so that a later run can carry on from here.
    ///
    /// **The destructure below is exhaustive on purpose and `..` may not be added to it.** A field
    /// added to [`Interpreter`] stops this function compiling until somebody has decided which of
    /// the three kinds it is — accumulated, memoised or derived from the page — and a `..` would
    /// make that decision silently. `doc/todo/46` states the cost of getting it wrong: a field
    /// forgotten is a report lost, on a page that still looks right.
    #[expect(
        clippy::too_many_lines,
        reason = "one line per field of an interpreter that has fifty-nine of them, twice over. \
                  The length is the guard rather than an accident: shortening it means a `..`, \
                  which is the one thing this function may not have"
    )]
    fn checkpoint(&self) -> Checkpoint {
        let Self {
            // Borrowed, or derived from the page and the view state, so [`Interpreter::for_page`]
            // produces them again from the same inputs.
            document: _,
            across: _,
            view: _,
            presses: _,
            base: _,
            pattern_initial: _,
            page: _,
            structure: _,
            output_intent: _,
            optional_content: _,
            delegated: _,
            blending: _,
            blending_beyond: _,
            // Memos of pure functions. A resumed run starts with empty ones and computes the same
            // answers — the cost is that an annotation's appearance loads its own fonts, shadings
            // and images again, which `Self::across` already answers for the fonts. Carrying them
            // would be carrying a decoded image per notch to save a decode the annotation half
            // rarely makes.
            fonts: _,
            shadings: _,
            resource_tables: _,
            icc_spaces: _,
            image_masks: _,
            image_rasters: _,
            stream_structures: _,
            clip_extents: _,
            // Accumulated: the tail of every one of these is what the annotation pass appends.
            list,
            unsupported,
            notes_raised,
            text_operations,
            segments_without_a_current_point,
            glyph_coverage,
            glyphs,
            codes_without_a_glyph,
            codes_reaching_a_blank_glyph,
            codes_without_a_vertical_form,
            codes_without_a_character,
            operations,
            text,
            described,
            artifacts,
            marked,
            marking,
            associated,
            reversed_chars,
            view_dependent,
            text_cursor,
            text_layer,
            inferred_separators,
            stream,
            hidden,
            glyph_depth,
            soft_mask_depth,
            uncoloured,
            inside_knockout,
            transparent_initial_backdrop,
            alpha_sources,
            alpha_sources_mark,
            compositing,
            blending_changed,
            black_generation_stated,
            opaque_ancestry,
            transfer_painted_opaquely,
            nested_space_departed,
        } = self;
        Checkpoint {
            list: list.clone(),
            unsupported: unsupported.clone(),
            notes_raised: *notes_raised,
            text_operations: *text_operations,
            segments_without_a_current_point: *segments_without_a_current_point,
            glyph_coverage: glyph_coverage.clone(),
            glyphs: *glyphs,
            codes_without_a_glyph: *codes_without_a_glyph,
            codes_reaching_a_blank_glyph: *codes_reaching_a_blank_glyph,
            codes_without_a_vertical_form: *codes_without_a_vertical_form,
            codes_without_a_character: *codes_without_a_character,
            operations: *operations,
            text: text.clone(),
            described: described.clone(),
            artifacts: artifacts.clone(),
            marked: marked.clone(),
            marking: marking.clone(),
            associated: associated.clone(),
            reversed_chars: *reversed_chars,
            view_dependent: *view_dependent,
            text_cursor: *text_cursor,
            text_layer: text_layer.clone(),
            inferred_separators: *inferred_separators,
            stream: *stream,
            hidden: *hidden,
            glyph_depth: *glyph_depth,
            soft_mask_depth: *soft_mask_depth,
            uncoloured: *uncoloured,
            inside_knockout: *inside_knockout,
            transparent_initial_backdrop: *transparent_initial_backdrop,
            alpha_sources: *alpha_sources,
            alpha_sources_mark: *alpha_sources_mark,
            compositing: compositing.clone(),
            blending_changed: *blending_changed,
            black_generation_stated: *black_generation_stated,
            opaque_ancestry: *opaque_ancestry,
            transfer_painted_opaquely: *transfer_painted_opaquely,
            nested_space_departed: *nested_space_departed,
        }
    }

    /// Puts a [`Checkpoint`] back into an interpreter [`Interpreter::for_page`] has just built.
    ///
    /// The destructure is exhaustive for [`Interpreter::checkpoint`]'s reason, and every binding
    /// is used: a field this stopped installing would be a warning, and warnings are errors here.
    fn restore(&mut self, checkpoint: Checkpoint) {
        let Checkpoint {
            list,
            unsupported,
            notes_raised,
            text_operations,
            segments_without_a_current_point,
            glyph_coverage,
            glyphs,
            codes_without_a_glyph,
            codes_reaching_a_blank_glyph,
            codes_without_a_vertical_form,
            codes_without_a_character,
            operations,
            text,
            described,
            artifacts,
            marked,
            marking,
            associated,
            reversed_chars,
            view_dependent,
            text_cursor,
            text_layer,
            inferred_separators,
            stream,
            hidden,
            glyph_depth,
            soft_mask_depth,
            uncoloured,
            inside_knockout,
            transparent_initial_backdrop,
            alpha_sources,
            alpha_sources_mark,
            compositing,
            blending_changed,
            black_generation_stated,
            opaque_ancestry,
            transfer_painted_opaquely,
            nested_space_departed,
        } = checkpoint;
        self.list = list;
        self.unsupported = unsupported;
        self.notes_raised = notes_raised;
        self.text_operations = text_operations;
        self.segments_without_a_current_point = segments_without_a_current_point;
        self.glyph_coverage = glyph_coverage;
        self.glyphs = glyphs;
        self.codes_without_a_glyph = codes_without_a_glyph;
        self.codes_reaching_a_blank_glyph = codes_reaching_a_blank_glyph;
        self.codes_without_a_vertical_form = codes_without_a_vertical_form;
        self.codes_without_a_character = codes_without_a_character;
        self.operations = operations;
        self.text = text;
        self.described = described;
        self.artifacts = artifacts;
        self.marked = marked;
        self.marking = marking;
        self.associated = associated;
        self.reversed_chars = reversed_chars;
        self.view_dependent = view_dependent;
        self.text_cursor = text_cursor;
        self.text_layer = text_layer;
        self.inferred_separators = inferred_separators;
        self.stream = stream;
        self.hidden = hidden;
        self.glyph_depth = glyph_depth;
        self.soft_mask_depth = soft_mask_depth;
        self.uncoloured = uncoloured;
        self.inside_knockout = inside_knockout;
        self.transparent_initial_backdrop = transparent_initial_backdrop;
        self.alpha_sources = alpha_sources;
        self.alpha_sources_mark = alpha_sources_mark;
        self.compositing = compositing;
        self.blending_changed = blending_changed;
        self.black_generation_stated = black_generation_stated;
        self.opaque_ancestry = opaque_ancestry;
        self.transfer_painted_opaquely = transfer_painted_opaquely;
        self.nested_space_departed = nested_space_departed;
    }
}

/// What one interpretation of a page has accumulated when its content stream has finished and
/// §12.5.3's annotation pass has not started.
///
/// `draw_annotations` runs **last** in [`interpret_into`], so everything the clause's pass
/// contributes is a tail on every one of the interpreter's accumulators — and re-placing an
/// annotation at a new magnification means keeping the content half and running that tail again.
/// This is that content half, and it is what makes a wheel notch on a page carrying a `NoZoom`
/// annotation cost the annotations rather than the page (`doc/todo/46`, ADR 0777).
///
/// **Which fields it holds is decided by the compiler**, not by this comment: see
/// [`Interpreter::checkpoint`].
#[expect(
    clippy::struct_excessive_bools,
    reason = "the same independent facts about the run in progress that `Interpreter` carries \
              and for the same reason — this is those fields and not a configuration, so \
              grouping them here would put a shape in front of a mirror"
)]
#[derive(Debug, Clone)]
struct Checkpoint {
    /// The marks the content stream made.
    list: DisplayList,
    /// What it could not draw, keyed as [`Interpreter::unsupported`] keys it.
    unsupported: BTreeMap<Unsupported, Unsupported>,
    /// [`Interpreter::notes_raised`], which a marked-content sequence brackets against.
    notes_raised: usize,
    /// The four counters `finished` turns into reports of their own.
    text_operations: usize,
    /// See [`Interpreter::segments_without_a_current_point`].
    segments_without_a_current_point: usize,
    /// Per-font coverage, **undrained**: the "this font drew nothing" report is computed over
    /// the whole page, and a page whose content half drew a font's glyphs and whose annotation
    /// half draws none of them must not report the font as blank. That report is why this is a
    /// checkpoint of the interpreter rather than a merge of two interpretations.
    glyph_coverage: BTreeMap<String, Coverage>,
    /// See [`Interpretation::glyphs`].
    glyphs: usize,
    /// See [`Interpretation::codes_without_a_glyph`].
    codes_without_a_glyph: usize,
    /// See [`Interpretation::codes_reaching_a_blank_glyph`].
    codes_reaching_a_blank_glyph: usize,
    /// See [`Interpretation::codes_without_a_vertical_form`].
    codes_without_a_vertical_form: usize,
    /// See [`Interpretation::codes_without_a_character`].
    codes_without_a_character: UnnamedCodes,
    /// See [`Interpreter::operations`].
    operations: usize,
    /// The readback so far, which every span below is an offset into.
    text: String,
    /// See [`Interpretation::described`].
    described: Vec<crate::accessibility::Described>,
    /// See [`Interpretation::artifacts`].
    artifacts: Vec<ArtifactSpan>,
    /// See [`Interpretation::marked`].
    marked: Vec<MarkedSpan>,
    /// Marked-content sequences a malformed content stream left open, which the annotation pass
    /// would otherwise close on a stack that had not been opened.
    marking: Vec<marked::OpenSequence>,
    /// See [`Interpretation::associated_files`].
    associated: Vec<(std::ops::Range<usize>, crate::attachment::Attachment)>,
    /// See [`Interpreter::reversed_chars`].
    reversed_chars: usize,
    /// False at every seam — nothing but §12.5.3's own pass sets it — and carried rather than
    /// assumed, because assuming it is what would make this list say something the code does not.
    view_dependent: bool,
    /// See [`Interpreter::text_cursor`].
    text_cursor: Option<(f32, f32)>,
    /// See [`Interpretation::text_layer`].
    text_layer: Vec<Placed>,
    /// See [`Interpreter::inferred_separators`].
    inferred_separators: usize,
    /// Which content stream was running, which is [`ContentStream::Page`] at every seam.
    stream: ContentStream,
    /// The graphics-state facts §11.6, §11.7 and §9.6.4 carry across a stream boundary, each of
    /// which the annotation pass reads or narrows in turn.
    hidden: usize,
    /// See [`Interpreter::glyph_depth`].
    glyph_depth: usize,
    /// See [`Interpreter::soft_mask_depth`].
    soft_mask_depth: usize,
    /// See [`Interpreter::uncoloured`].
    uncoloured: bool,
    /// See [`Interpreter::inside_knockout`].
    inside_knockout: bool,
    /// See [`Interpreter::transparent_initial_backdrop`].
    transparent_initial_backdrop: bool,
    /// See [`Interpreter::alpha_sources`].
    alpha_sources: AlphaSourcesSeen,
    /// An index into [`Self::list`], which is why the two are carried together.
    alpha_sources_mark: usize,
    /// What the page is painting into, which decides what a colour becomes.
    compositing: Compositing,
    /// See [`Interpreter::blending_changed`].
    blending_changed: bool,
    /// See [`Interpreter::black_generation_stated`].
    black_generation_stated: bool,
    /// See [`Interpreter::opaque_ancestry`].
    opaque_ancestry: bool,
    /// See [`Interpreter::transfer_painted_opaquely`].
    transfer_painted_opaquely: bool,
    /// See [`Interpreter::nested_space_departed`].
    nested_space_departed: bool,
}

/// One interpretation of a page, into the components `compositing` names.
///
/// The second half of the answer is whether the page may be drawn in the blending space it
/// states — see [`Interpreter::blending_undrawable`], which is what decides it.
fn interpret_into(
    document: &Document,
    page: &Page,
    state: &crate::view::ViewState,
    compositing: Compositing,
    presses: &crate::colour::Presses,
    fonts: &FontCache,
    keep: Keep,
) -> (Interpretation, bool, Option<Checkpoint>) {
    // **The page's `/Contents` is read through a window and never assembled into one buffer**,
    // which is road D of `doc/todo/10` §5 and ADR 0365. What it buys, measured: a
    // decompression bomb costs 8.4 MB of resident memory instead of a gibibyte, and the
    // largest honest content stream this project has met — 141 MiB in one part — is
    // interpreted from 194 MB instead of 381 MB. What it costs is 5.74% of the instructions
    // to interpret an ordinary page, and one report arriving late: a part damaged half way
    // through is met half way through, so the reader is asked twice, here and after the run.
    let mut reader = reader::ContentReader::for_page(document, page);
    let mut interpreter = Interpreter::for_page(document, page, state, compositing, presses, fonts);

    for issue in reader.take_issues() {
        interpreter.note(Unsupported::Content { issue });
    }
    // §7.3.7's dictionary, where this page was assembled from the part of it the file states
    // readably. Reported before the media box because it is the reason the media box may be
    // missing: the entry could be one of the ones after the damage.
    if let Some(damage) = page.damaged_dictionary.as_ref() {
        // How the object came to be read as a page is part of the same statement about the file,
        // because the two answers are two different claims: one is Table 31's `/Type` off the
        // producer's own bytes, the other is this reader's inference from §7.7.3.2. A reader owes
        // the difference out loud rather than presenting the second as though it were the first.
        let identification = match damage.identification {
            crate::page::PageIdentification::ItsOwnDeclaration => {
                "it is a page because those entries state Table 31's /Type /Page".to_owned()
            }
            crate::page::PageIdentification::TheTreeAndAPageOnlyEntry(entry) => format!(
                "it is a page because the page tree's /Kids names it — §7.7.3.2 makes a child a \
                 page object or a page tree node — and it states /{entry}, which only a page \
                 object may carry"
            ),
        };
        interpreter.note(Unsupported::PageDictionary {
            detail: format!(
                "this page's object states {} entr(ies) and then stops being readable at byte {} \
                 ({}), so the page is drawn from those entries alone and whatever its producer \
                 wrote after them is not here; {identification}",
                damage.entries, damage.stopped_at, damage.reason
            ),
        });
    }
    // §7.7.3.3's `/MediaBox`, which §7.7.3.4 requires of the page or of an ancestor. Reported
    // before anything is drawn because it is the frame the drawing happens in rather than one of
    // its marks: the page below is the producer's, and the rectangle it is placed on is ours.
    if let Some(substitution) = page.substituted_media_box {
        let [x0, y0, x1, y1] = page.media_box;
        interpreter.note(Unsupported::MediaBox {
            detail: format!(
                "{} — the page is drawn on {} × {} at [{x0} {y0} {x1} {y1}], \
                 which is this reader's default and not the producer's",
                match substitution {
                    crate::page::MediaBoxSubstitution::Absent =>
                        "no /MediaBox anywhere in the page's ancestry (§7.7.3.4)",
                    crate::page::MediaBoxSubstitution::NotARectangle =>
                        "a /MediaBox that is not §7.9.5's four finite numbers",
                    crate::page::MediaBoxSubstitution::Empty =>
                        "a /MediaBox enclosing no area, which §7.9.5 admits as a rectangle \
                         and Table 31 does not admit as a medium",
                },
                x1 - x0,
                y1 - y0,
            ),
        });
    }
    // §8.11.4.4's automatic states, for the two categories that ask about this machine rather
    // than about the document. Reported once per page rather than per group, because what a
    // reader can do about it is the same either way.
    let unresolved: Vec<&'static str> = interpreter
        .optional_content
        .as_ref()
        .map(crate::optional_content::OptionalContent::unresolved_usage)
        .unwrap_or_default()
        .to_vec();
    for category in unresolved {
        interpreter.note(Unsupported::OptionalContent {
            detail: format!(
                "a /AS usage application dictionary asks for the {category} category, which is \
                 a question about this processor rather than about the document"
            ),
        });
    }

    let base = base_transform(page);
    // §14.11.2.1: "[t]he crop box defines the region to which the contents of the page shall be
    // clipped (cropped) when displayed or printed", and §12.2's `/ViewClip` may name a different
    // one of §14.11.2's five boxes — "the page boundary to which the contents of a page shall be
    // clipped when viewing the document on the screen", defaulting to `CropBox`. So the boundary
    // stated here is `clip_box` rather than the crop box by name, and the two are one rectangle
    // for every document that states no preference.
    //
    // **Stated on the list rather than built as a clipping path**, and that is a measurement
    // rather than a preference: a `Clip` would cost every page a page-sized coverage mask and a
    // masked composite per command, and the population that marks outside its own boundary at
    // all is a few percent (`examples/crop_box_census`). `pdf_render::crop_to_page` is where it
    // is applied, once per target, by all three rasterisers.
    interpreter.list.set_content_clip(content_clip(page, base));
    let initial = GraphicsState::initial(base);
    interpreter.run_reader(&mut reader, &page.resources, &initial, 0);
    // §7.4.1's second half, for a part whose damage the pump met while the page was being
    // drawn: the bytes are on the page and the shortfall is in the report (ADR 0343). The
    // order the two loops find issues in does not matter — `note` collects them into a map
    // and `finished` sorts it.
    for issue in reader.take_issues() {
        interpreter.note(Unsupported::Content { issue });
    }
    // **The seam, and it is here because §12.5.3's pass is the only thing left.** Taken before
    // the annotations rather than after them, and only where Table 167's bit says the page has
    // one whose placement the magnification decides: on every other page the clone below would
    // be a second copy of a list nothing will ever ask to move. `doc/todo/46`, ADR 0777.
    let checkpoint = (keep == Keep::Replacement && interpreter.any_no_zoom(page))
        .then(|| interpreter.checkpoint());
    let (interpretation, drawable) = complete(document, page, base, interpreter);
    (interpretation, drawable, checkpoint)
}

/// §12.5.3's annotation pass and everything the page owes after it.
///
/// Split from [`interpret_into`] because it is exactly what [`replace`] runs a second time: the
/// annotations, and the three answers that are about the *whole* page and therefore cannot be
/// taken before them.
fn complete(
    document: &Document,
    page: &Page,
    base: Transform,
    mut interpreter: Interpreter<'_>,
) -> (Interpretation, bool) {
    // §12.5: an annotation is drawn *over* the page content, and in `/Annots` order, so
    // this pass follows the content stream rather than being folded into it. It is not exempt
    // from the boundary set above — an annotation is displayed content of the page — and it
    // does not have to be told, because the boundary belongs to the list rather than to a mark.
    interpreter.draw_annotations(page, base);
    // Asked once the page is complete, because the condition is about the whole of it: an
    // annotation composites into the page group exactly as the content stream's marks do.
    interpreter.note_page_blending_space();

    // A font that drew *nothing* of what it was asked to show. Two ways to get there and one
    // condition: §9.10.2 gave the codes characters and the substitute face has none of them,
    // or the program — embedded or not — answers every code with no outline. Reported per font
    // and only where the count of glyphs drawn through it is zero, which is what keeps a space
    // and a deliberate `.notdef` from being news; see `Interpreter::glyph_coverage`.
    //
    // **The second of those two was narrower than this sentence until the
    // six-hundred-and-eighty-fifth session** (ADR 0520), and a report is only as wide as the
    // count it fires on (trap 11): `Coverage::empty` was raised only for a code §9.10.2 could
    // also *name*, which is a question about the reader, so a font whose codes reached no glyph
    // *and* no character drew a blank page with nothing to report it. `text.rs` divides that
    // branch on the glyph now.
    for (name, coverage) in std::mem::take(&mut interpreter.glyph_coverage) {
        if coverage.drawn > 0 || coverage.empty == 0 {
            continue;
        }
        let detail = if coverage.uncovered > 0 {
            format!(
                "font /{name} is substituted and the face this machine offers draws none of \
                 the {} character(s) it is asked for (§9.10.2)",
                coverage.empty
            )
        } else {
            format!(
                "font /{name}'s program has no outline for any of the {} code(s) the page \
                 shows through it, so the text it states is not drawn",
                coverage.empty
            )
        };
        interpreter.note(Unsupported::Font { detail });
    }

    let drawable = interpreter.blending_undrawable().is_none();
    (finished(document, interpreter), drawable)
}

/// Turns the interpreter's accumulated state into what a caller reads.
///
/// Split out because it is bookkeeping rather than interpretation, and because `interpret_with`
/// is held to a hundred lines.
fn finished(document: &Document, interpreter: Interpreter<'_>) -> Interpretation {
    let mut unsupported: Vec<Unsupported> = interpreter.unsupported.into_values().collect();
    if interpreter.text_operations > 0 {
        unsupported.push(Unsupported::Text {
            operations: interpreter.text_operations,
        });
    }
    // ISO 32000-2 §8.5.2.1's "an error shall be generated", raised once per page with the number
    // of segments it covers rather than once per operator: the condition repeats along a run, and
    // what a reader can act on is that the page is short of geometry, not which `c` it was.
    if interpreter.segments_without_a_current_point > 0 {
        unsupported.push(Unsupported::UndefinedCurrentPoint {
            segments: interpreter.segments_without_a_current_point,
        });
    }
    // ISO 32000-2 §8.3.4 NOTE 3's noninvertible matrix, asked of the finished list rather than
    // at each of the six places a mark is pushed: the condition is a property of the command
    // and one walk cannot miss a route. See [`Unsupported::NoninvertibleMatrix`] for why this
    // is a report and not a refusal, and `pdf_render::DisplayList::noninvertible_marks` for
    // what makes such a mark absent whatever any backend does.
    let noninvertible = interpreter.list.noninvertible_marks();
    if noninvertible > 0 {
        unsupported.push(Unsupported::NoninvertibleMatrix {
            commands: noninvertible,
        });
    }
    unsupported.sort_unstable();

    // §14.9.2.3's default for everything in the file, and the only one of §14.9's entries with
    // a document-wide statement. Read once per page rather than per section — and not at all
    // for a page with nothing to say, which is the whole reason a document's language is
    // wanted: it is the language of the text, and there is none.
    let has_text = !interpreter.text.is_empty() || !interpreter.described.is_empty();
    let language = has_text
        .then(|| crate::structure::document_language(document))
        .flatten();

    Interpretation {
        display_list: interpreter.list,
        view_dependent: interpreter.view_dependent,
        unsupported,
        text: interpreter.text,
        glyphs: interpreter.glyphs,
        codes_without_a_glyph: interpreter.codes_without_a_glyph,
        codes_reaching_a_blank_glyph: interpreter.codes_reaching_a_blank_glyph,
        codes_without_a_vertical_form: interpreter.codes_without_a_vertical_form,
        codes_without_a_character: interpreter.codes_without_a_character,
        described: interpreter.described,
        artifacts: interpreter.artifacts,
        marked: interpreter.marked,
        inferred_separators: interpreter.inferred_separators,
        associated_files: interpreter.associated,
        language,
        text_layer: interpreter.text_layer,
    }
}

/// The page's extent as it is displayed: after §7.7.3.3's `/Rotate`, and in `/UserUnit`s.
///
/// A rotated page swaps its extents, so this is not [`Page::width`] and [`Page::height`].
///
/// Public because a viewer needs it *before* there is a display list to read it from: fitting a
/// page to a window is what decides the scale to interpret it at, and asking the other way round
/// would interpret every page twice.
#[must_use]
pub fn displayed_size(page: &Page) -> Size {
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

/// Maps a point in the **display list's** space back to default user space.
///
/// The inverse of the transform every page is drawn under, which is what a caller needs to turn
/// a click into a place in the document: §12.5.2 states an annotation's `/Rect` "in default user
/// space units", and §7.7.3.3's `/Rotate` and `/CropBox` are exactly what stand between that and
/// a pixel. Returns `None` for a page whose transform is degenerate, which a zero-sized crop box
/// would produce.
///
/// **The display list's space is not the raster's**, and this doc comment said it was for
/// seventy-five sessions. PDF's y axis points up and a raster's points down, and the flip
/// between them belongs to [`pdf_render::TargetSpec::for_page`] rather than to the page — see
/// [`base_transform`]. A caller holding a pixel position therefore subtracts it from the page's
/// height *in the same units* before calling this, which is what `viewer-core` does; one that
/// did not was mirroring every click about the middle of the page. ADR 0118.
#[must_use]
pub fn user_space_at(page: &Page, x: f32, y: f32) -> Option<(f32, f32)> {
    let point = base_transform(page).invert()?.apply(Point::new(x, y));
    Some((point.x, point.y))
}

/// Maps a point in default user space into the page's own space.
///
/// [`user_space_at`]'s forward direction, and the same caution applies in reverse: what comes
/// back is the display list's space, whose y still points **up** from the bottom of the page.
/// §12.3.2.2's destinations are the caller this exists for — Table 149 states its coordinates
/// "in the default user space" and a viewer has to put them somewhere on a raster.
#[must_use]
pub fn page_space_at(page: &Page, x: f32, y: f32) -> (f32, f32) {
    let point = base_transform(page).apply(Point::new(x, y));
    (point.x, point.y)
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
/// The map from a page's default user space to the display list's own coordinates.
///
/// [`base_transform`] under a name a caller outside this module can say, and it is public for
/// one reason: **an edit has to go the other way**. Every geometry this crate answers with —
/// [`Placed::quad`], a selection's shapes — is in the display list's space, and §12.5.6.10's
/// `/QuadPoints` is defined in default user space, so a host authoring an annotation from a drag
/// composes this transform's inverse. `pdf_render::Transform::invert` is the other half.
#[must_use]
pub fn page_transform(page: &Page) -> Transform {
    base_transform(page)
}

/// §14.11.2.1's boundary for this page, in the display list's own space.
///
/// ISO 32000-2 §14.11.2.1, and it is a `shall`:
///
/// > The crop box defines the region to which the contents of the page shall be clipped
/// > (cropped) when displayed or printed. Unlike the other boxes, the crop box has no defined
/// > meaning in terms of physical page geometry or intended use; it merely imposes clipping on
/// > the page contents.
///
/// [`Page::clip_box`] and not [`Page::crop_box`], because §12.2's `/ViewClip` names which of
/// §14.11.2's five boxes a screen clips to and Table 147 defaults it to `CropBox`: the two are
/// the same rectangle for every document that states no preference, and where they differ the
/// preference is what the clause defers to.
///
/// `base` maps default user space into the list's, which for every page is a translation, a
/// quarter turn and a scale — so a rectangle stays a rectangle and two opposite corners carry
/// it. [`Rect::from_corners`] orders them, which is what §7.9.5 requires anyway: a rectangle's
/// corners "can be given in any order".
fn content_clip(page: &Page, base: Transform) -> Rect {
    let [x0, y0, x1, y1] = page.clip_box;
    Rect::from_corners(
        base.apply(Point::new(x0, y0)),
        base.apply(Point::new(x1, y1)),
    )
}

fn base_transform(page: &Page) -> Transform {
    let shift = Transform::translate(-page.display_box[0], -page.display_box[1]);
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent facts about the run in progress — whether the page is being \
              interpreted for a view, whether an uncoloured pattern's cell is running, whether \
              a knockout group encloses the content, and whether §11.6.4.3's /AIS has been \
              seen. They are not a configuration a caller passes and grouping them would put \
              four unrelated questions behind one name"
)]
struct Interpreter<'a> {
    document: &'a Document,
    list: DisplayList,
    /// Keyed so that a page drawing the same unsupported image a thousand times reports it
    /// once rather than flooding the diagnostics.
    unsupported: BTreeMap<Unsupported, Unsupported>,
    /// How many times [`Interpreter::note`] has been called, **before** the map deduplicates.
    ///
    /// Not a count of reports and never printed as one: [`Self::unsupported`] is keyed so that a
    /// page refusing the same image a thousand times reports it once, and a *position* in that
    /// stream is what [`marked::OpenSequence`] needs — a page whose second refusal of `/Im0`
    /// falls inside a different structure element than its first must attribute one to each, and
    /// the map's length would have moved only for the first. Monotonic, so a sequence's opening
    /// value and the value at its `EMC` bracket exactly what it enclosed.
    notes_raised: usize,
    text_operations: usize,
    /// Segment operators refused for having no current point; see
    /// [`Unsupported::UndefinedCurrentPoint`]. Counted rather than noted per occurrence for the
    /// same reason `text_operations` is: `issue6342.pdf` states thirteen of them on one page.
    segments_without_a_current_point: usize,
    /// Per font resource name, what its codes got out of its program. Drained into one report
    /// per font at the end of the page.
    ///
    /// **The condition is "drew none", and it was narrowed to that by measurement.** Reporting
    /// every code a substitute cannot draw named 13 corpus documents, most of which draw
    /// nearly all of their text — `noembed-eucjp.pdf` draws あいうえお and misses one
    /// character — and each report costs the oracle a judged page (trap 11). A substitute that
    /// draws *nothing* is the case where a page is blank and nobody is told, which is
    /// `issue8372.pdf`, and it is the same condition the simple-font path applies at load time:
    /// "the face draws none of the codes the document declares".
    glyph_coverage: BTreeMap<String, Coverage>,
    /// Glyphs that marked the page; see [`Interpretation::glyphs`].
    glyphs: usize,
    /// Codes shown that reached no glyph; see `Interpretation::codes_without_a_glyph`.
    codes_without_a_glyph: usize,
    /// Codes shown that reached an empty glyph; see
    /// `Interpretation::codes_reaching_a_blank_glyph`.
    codes_reaching_a_blank_glyph: usize,
    /// Codes drawn whose vertical form the substituted face lacked; see
    /// `Interpretation::codes_without_a_vertical_form`.
    codes_without_a_vertical_form: usize,
    /// Codes shown that §9.10.2 could not name; see
    /// `Interpretation::codes_without_a_character`.
    codes_without_a_character: UnnamedCodes,
    operations: usize,
    /// Fonts already loaded during *this* interpretation, by the object each dictionary is.
    ///
    /// A page names the same font on every `Tf`, and parsing a font program is expensive,
    /// so this is what keeps text rendering from being dominated by font loading.
    ///
    /// **This comment said "keyed by resource name" for six hundred and forty rounds after that
    /// stopped being true**, which is the shape `doc/habits.md`'s ledger section is about one
    /// directory over: the hundred-and-twenty-seventh session moved the key to the object's
    /// identity precisely because a name conflated a page's `/F1` with a form's, and the line
    /// describing it did not move. See [`FontKey`].
    ///
    /// It holds a *failure* as well as a font, which [`Self::across`] deliberately does not.
    fonts: BTreeMap<FontKey, Option<Font>>,
    /// Fonts loaded out of this document by an *earlier* interpretation, where the caller kept
    /// them (§9.6, ADR 0710).
    ///
    /// Empty and unshared for every caller that does not, which is [`interpret`] and
    /// [`interpret_with`] — those build one per call, so the two runs of §11.4.7's subtractive
    /// pair below share a cache and nothing else does.
    across: &'a FontCache,
    /// Maps PDF user space to page space.
    ///
    /// Pattern space is defined relative to the page's default coordinates rather than to
    /// the transform in force when a pattern is used, so this is kept for patterns and
    /// must not be confused with the current transform.
    base: Transform,
    /// The parameters ISO 32000-2 §11.6.7 hands a shading pattern selected in the content stream
    /// now running — the graphics state it *began* with, not the one at the `scn`.
    ///
    /// Scoped exactly like [`Self::base`] and for the same sentence: the pattern matrix maps to
    /// "the default coordinate system of the pattern's parent content stream" (§8.7.2), and
    /// §11.6.7 says the same of black point compensation, the rendering intent and §10.7.3's
    /// smoothness. See [`PatternInitial`].
    pattern_initial: PatternInitial,
    /// The page's extent, used to bound a shading painted by `sh`.
    page: Size,
    /// Shadings already built, by the object that states them (§8.7, ADR 0069).
    ///
    /// A page paints one shading object many times — a pattern under every cell of a chart,
    /// an `sh` inside a form invoked per data point — and the colours it carries are the
    /// same every time. This is what keeps `Function::parse` from running once per painting
    /// operation; `shading::Cache` has the measurement and the one case it refuses.
    shadings: crate::shading::Cache,
    /// A resource **category table** — `/ExtGState`, `/Font`, `/XObject` — that a resource
    /// dictionary states as an indirect reference, resolved once per object.
    ///
    /// # Why this exists, and it is quadratic without it
    ///
    /// `Document::get` hands back an *owned* object, so resolving `/Resources /ExtGState`
    /// copies the whole category table out of the document's cache — every time. A page that
    /// states one `/ExtGState` entry per `gs` operator therefore copies an *n*-entry
    /// `BTreeMap` *n* times. `1284722.pdf` from the `SafeDocs` corpus is exactly that page:
    /// **26 414 entries and 26 414 `gs` operators**, and 57% of its 108 G interpretation
    /// instructions were cloning and dropping that map (`doc/todo/03` named it as the
    /// population's next candidate at 11.1 s for 94 596 commands).
    ///
    /// Keyed by [`ObjectId`] because that is what identifies the table: two resource
    /// dictionaries naming the same object name the same table, and a reference is the only
    /// thing that says so. A *direct* table needs no entry here at all — it is already in
    /// hand, and [`Interpreter::resource_entry`] reads it in place.
    ///
    /// **What it costs is a second copy of each table** beside the document's own cache,
    /// bounded by the number of distinct resource tables the page's forms reach. That is the
    /// trade, and on the witness it is one copy of one map against 26 414 of them.
    resource_tables: std::cell::RefCell<BTreeMap<ObjectId, Dictionary>>,
    /// An `ICCBased` colour space a `cs` or `CS` operator names, parsed once per stream.
    ///
    /// # Why only this one shape
    ///
    /// `ColourSpace::parse` is a pure function of the object *and* the resource dictionary in
    /// force — §8.6.5.1 resolves a name through it, and an `Indexed` space's base may be one —
    /// so a space cannot in general be remembered by the object alone. `[/ICCBased <stream>]`
    /// can: its whole content is the stream, §8.6.5.5 states nothing about it that a resource
    /// dictionary could change, and the stream's [`ObjectId`] identifies it exactly.
    ///
    /// # What it is worth
    ///
    /// The parse **inflates the profile and reads its tables**, and a page that sets the same
    /// space per mark pays that per mark. `3129278.pdf` from the `SafeDocs` corpus is 1052
    /// shading fills each preceded by its own `cs`, and 95% of its 380 G interpretation
    /// instructions were inside `ColourSpace::parse_at` — 78% of the page in `zlib` and 17%
    /// in `icc::Profile::parse` — for one profile it read 1053 times.
    icc_spaces: BTreeMap<ObjectId, ColourSpace>,
    /// §11.6.5.2's soft masks already read for the device to place (§10.7.4).
    ///
    /// The same argument as [`Self::shadings`], and the same shape: a page draws one
    /// `XObject` many times and its mask's samples do not depend on where.
    /// `crate::image::MaskCache` carries the measurement.
    image_masks: crate::image::MaskCache,
    /// §8.9.5's base rasters already decoded, so that one `XObject` drawn many times is
    /// decoded once.
    ///
    /// The same shape as [`Self::image_masks`] and a harder key: a mask's every input is the
    /// mask object's own, while a base image's raster depends on the resource dictionary in
    /// force, the fill colour and what the samples are composited into as well as on the
    /// stream. `crate::image::RasterCache` states what the key claims and carries the
    /// measurement.
    image_rasters: crate::image::RasterCache,
    /// §14.7.5.4's structural parent tree for this page, empty for most documents.
    ///
    /// Read once when the page is interpreted, because the lookup it answers — a
    /// marked-content identifier to its structure element — happens per `BDC` and the tree is
    /// a number tree walk. 87 of the 974 corpus documents have a structure tree at all.
    structure: Arc<crate::structure::ParentTree>,
    /// The same tree for each *other* content stream that stated a `/StructParents` of its own.
    ///
    /// §14.7.5.4 gives every content stream holding structure content items its own entry, so a
    /// form `XObject` has a parent tree of its own and a page drawing it many times reads that
    /// entry many times. Keyed by the object the stream is, which is what makes two `Do`s of one
    /// form the same stream. Empty for every document with no structure inside a form, which is
    /// almost all of them.
    stream_structures: BTreeMap<ObjectId, Arc<crate::structure::ParentTree>>,
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
    /// §14.9's accessibility spans over [`Self::text`], pushed as each section closes.
    described: Vec<crate::accessibility::Described>,
    /// §14.8.2.2's artifact spans, in the order their sections closed.
    artifacts: Vec<ArtifactSpan>,
    /// §14.7.5.2's marked-content spans, in the order their sections closed.
    marked: Vec<MarkedSpan>,
    /// Which content stream the operators now being read came out of (§14.7.5.2).
    ///
    /// [`ContentStream::Page`] while the page's own `/Contents` is running, and replaced for the
    /// span of a form `XObject`'s or an appearance's stream by [`Interpreter::draw_xobject`] and
    /// [`Interpreter::draw_appearance`] — because a `/MCID` "uniquely identifies the
    /// marked-content sequence within its content stream" and not beyond it, so the two together
    /// are what names a sequence.
    ///
    /// **A Type 3 glyph description does not replace it, deliberately.** §9.6.4 makes a
    /// `/CharProc` how one *glyph* of the enclosing stream's text is painted; its marks belong to
    /// the show operation that asked for them, and therefore to whatever sequence encloses that.
    stream: ContentStream,
    /// One entry per open marked-content sequence that stated an `/MCID`, innermost last.
    ///
    /// What each entry accumulates is §14.8.3.3's content rectangle for that sequence — "derived
    /// from the shape of the enclosed content" — as [`Interpreter::draw`] unions in each command's
    /// own bound. `None` inside an entry is a sequence that has drawn nothing yet, which is a
    /// different fact from a sequence that drew something empty and is why the entry is an
    /// `Option` rather than a degenerate rectangle.
    ///
    /// **Empty for every untagged page**, which is 885 of the corpus's 974, so what those pay for
    /// this is one `Vec::is_empty` per command.
    ///
    /// A stack rather than one accumulator because §14.7.5.1.1 forbids the nesting — "[a]
    /// marked-content sequence corresponding to a structure content item shall not have another
    /// marked-content sequence for a structure content item nested within it" — and a file that
    /// does it anyway must not have the inner sequence's marks silently become the outer one's
    /// only extent. A stack costs one allocation on a tagged page and answers both.
    marking: Vec<marked::OpenSequence>,
    /// What each clip region of [`Self::list`] admits, indexed by its [`pdf_render::ClipId`].
    ///
    /// [`Interpreter::clip_extent`]'s memo, and empty for every page that never opens a
    /// marked-content sequence with an `/MCID` — the answers are only asked for while one is
    /// open.
    clip_extents: Vec<marked::ClipExtent>,
    /// §14.13.5's associated files, with the range of the readback their section covered.
    associated: Vec<(std::ops::Range<usize>, crate::attachment::Attachment)>,
    /// How many §14.8.2.5.3 `ReversedChars` sections are open.
    ///
    /// A counter because marked content nests and the clause states no limit on it; a show
    /// string is reversed whenever at least one is open.
    reversed_chars: usize,
    /// Whether any annotation on this page sets §12.5.3's `NoZoom`.
    view_dependent: bool,
    /// Where the last glyph ended, used to decide where a space belongs.
    text_cursor: Option<(f32, f32)>,
    /// Where each shown code's readback sits on the page; see [`Interpretation::text_layer`].
    text_layer: Vec<Placed>,
    /// How many separators [`Interpreter::separate_text`] inferred from position.
    inferred_separators: usize,
    /// The document's optional content configuration, if it has one (§8.11).
    ///
    /// Cloned from the viewer state rather than borrowed, because §12.6.4.13's action may
    /// have moved it and the interpreter reads it thousands of times per page.
    optional_content: Option<crate::optional_content::OptionalContent>,
    /// The viewer state, for the half of it the interpreter asks per annotation (§12.6.4.11).
    view: &'a crate::view::ViewState,
    /// §12.7's widgets on this page whose appearance the host draws instead (§6.3.2.2).
    ///
    /// Empty for every caller that has not asked, which is every caller in this workspace but a
    /// native form host — see [`crate::view::WidgetAppearances`].
    delegated: BTreeSet<ObjectId>,
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
    /// Whether the group being built is, or is inside, §11.4.6's knockout group.
    ///
    /// One flag rather than a depth, for `uncoloured`'s reason: what it guards is a property
    /// every enclosing group shares. It exists for §11.4.4's NOTE 5, whose first condition is
    /// that a group "has the same knockout attribute as its parent group" — a child flattened
    /// into a knockout parent would stop being *one* element of that parent and become several,
    /// which is precisely what §11.4.6 makes different.
    inside_knockout: bool,
    /// Whether a group opened here composites its elements onto a **transparent** initial
    /// backdrop whatever Table 145's `/I` says (§11.4.6 NOTE 6).
    ///
    /// > When a non-isolated group is nested within a knockout group, the initial backdrop of
    /// > the inner group is the same as that of the outer group; it is not the immediate
    /// > backdrop of the inner group.
    ///
    /// So a *direct element* of a knockout group takes that group's initial backdrop, and
    /// where the knockout group's own is transparent — §11.4.5's, which an isolated one has
    /// and which a knockout group that is itself such an element inherits in turn — the inner
    /// group is §11.4.5's isolated group by that clause's own definition, and drawing it on
    /// transparency is the clause rather than a substitution.
    ///
    /// Set for a knockout group's own content and cleared for everything else, because NOTE 6
    /// reaches a direct element and not a descendant: a group two levels down composites onto
    /// its parent's *accumulated* content, which is what "it is not the immediate backdrop"
    /// distinguishes.
    transparent_initial_backdrop: bool,
    /// Which readings of §11.6.4.3's `/AIS` the content being run painted under.
    ///
    /// The entry decides whether a soft mask and the alpha constants are *shape* or
    /// *opacity*, and §11.4.6's weighted average is taken with the shape — so a knockout
    /// group's elements are built one way under each reading (`stated_shape`), and a group
    /// whose content stated both is refused by name because no single reading describes it.
    ///
    /// Scoped **to one group's content** rather than to the page, which is the
    /// four-hundred-and-ninety-second session's narrowing: the entry is a graphics state
    /// parameter, so `q`/`Q` bound it, and a `gs` inside one form said nothing about a
    /// sibling form's group — yet the page-wide flag refused every knockout group after it
    /// (`issue18032.pdf` states it inside a form whose group draws nothing at all, two
    /// forms before the knockout group it cost). [`Interpreter::run_transparency_group`]
    /// seeds this from [`GraphicsState::alpha_is_shape`] — the value actually in force at
    /// the `Do` — runs the content, reads what the run left, and folds it into the enclosing
    /// value, so an enclosing group still sees a nested `gs`.
    ///
    /// **It propagates outward on purpose**, and that is what makes the `Shape` reading safe
    /// for a group that contains other groups: a nested group's own marks are the enclosing
    /// group's marks too, so a `Shape` answer here means every mark inside, at every depth,
    /// was painted under `/AIS true`. A soft mask's group is the one exception and is
    /// restored exactly, because its marks become one alpha per pixel rather than elements.
    ///
    /// Within one scope it is an over-approximation in one direction only, and
    /// [`Interpreter::alpha_sources_mark`] is what keeps it from being one in the other: a
    /// reading that was in force while *nothing was painted* is replaced rather than mixed in.
    alpha_sources: AlphaSourcesSeen,
    /// The display list's length when [`Interpreter::alpha_sources`] last changed.
    ///
    /// A reading nothing was painted under says nothing about the group being built, and the
    /// commonest shape in a real file is a form whose content opens with the `gs` that states
    /// `/AIS` — where the value inherited from the `Do` reached no mark at all. So a statement
    /// arriving while the list is still this long **replaces** the record instead of mixing
    /// into it.
    ///
    /// The invariant it rests on is that a command is only ever taken off the list to be
    /// folded into a replacement, or by a group that painted nothing — so the list being this
    /// long again means nothing has been painted since. Where that ever stopped holding the
    /// comparison would simply fail and the record would say `Mixed`, which is the direction
    /// that costs a report rather than a wrong pixel.
    alpha_sources_mark: usize,
    /// What the content being run is painting into, which decides what a colour becomes.
    compositing: Compositing,
    /// The blending colour space in force here, named where this tree does not composite in it.
    ///
    /// §11.3.4 makes the space part of the model rather than a property of the output — "[t]he
    /// result of the computation thus depends on the colour space in which the colours are
    /// represented" — and two clauses decide which one is in force at any point. §11.4.7 roots
    /// it at the page group and §11.6.6 inherits it down the group stack, taking a group's own
    /// `/CS` only where that group is isolated. `None` is a space whose components are the
    /// three the device raster already holds, which is what this tree composites in; `Some`
    /// names one that is not, and is what gets reported where it is introduced.
    blending: Option<String>,
    /// Whether the space in force changed anywhere below the page group, on the page itself.
    ///
    /// §11.4.7's page group is drawn in its own space by running the page twice, once per half
    /// of its four components (`crate::colour::Half`), and that answers the *page*: a group
    /// inside it that introduces a different space would need its own pair of rasters and a
    /// conversion between the two spaces at its `Do`. Where one does, the page is drawn on the
    /// device's components and reported instead — narrowing the page's own condition until it
    /// stopped firing is the failure this flag exists to avoid.
    ///
    /// **"On the page itself" is the whole of the four-hundred-and-fortieth session's finding.**
    /// A *soft mask's* group is not painted onto the page: §11.5.3 composites it against its own
    /// backdrop and takes one luminosity from the result, which becomes an alpha. So a space
    /// declared inside one says nothing about the space the page composites in, and
    /// [`Interpreter::build_soft_mask`] scopes this flag the way it already scoped
    /// [`Interpreter::blending`]. ADR 0276.
    blending_changed: bool,
    /// Whether any `/ExtGState` on this page states Table 57's `/BG`, `/BG2`, `/UCR` or
    /// `/UCR2`, which §11.7.5.3 puts inside §10.4.2.4's conversion into a `DeviceCMYK` group.
    black_generation_stated: bool,
    /// Whether §11.7.5.2's opacity conditions held wherever the content being run was invoked.
    ///
    /// The clause's fifth and sixth conditions are about *ancestry* rather than about the mark:
    ///
    /// > The foregoing four conditions were also true at the time the `Do` operator was invoked
    /// > for the group containing the object, as well as for any direct ancestor groups.
    ///
    /// > If the current colour is a tiling pattern, all objects in the definition of its pattern
    /// > cell also satisfy the foregoing conditions.
    ///
    /// A mark inside a transparency group cannot see either from its own graphics state, because
    /// §11.6.6 resets the blend mode, both alpha constants and the soft mask before the group's
    /// content runs — and a tiling pattern's cell starts from [`GraphicsState::initial`] for
    /// §11.6.7's reason. So the answer is carried down instead: one flag rather than a stack, for
    /// [`Self::inside_knockout`]'s reason, since what it guards is a property every enclosing
    /// scope shares. Saved and restored by whoever narrows it.
    opaque_ancestry: bool,
    /// Whether a **fully opaque** mark on this page has carried §10.5's transfer function.
    ///
    /// §11.7.5.2 is a statement about a *point*, and the colour at a point has as many
    /// contributors as there are objects covering it — so the question "was a transfer function
    /// applied to something composited here" outlives the object that applied it. Monotone over
    /// the page for that reason.
    ///
    /// **Fully opaque is the whole of the condition since the seven-hundred-and-sixth session**,
    /// and it narrowed because the code under it did: a mark the clause does not call fully opaque
    /// is now handed the page's default function, so its colour is one no transfer has touched and
    /// it can no longer put a wrong colour under anything. See
    /// [`Interpreter::transfer_for_mark`].
    transfer_painted_opaquely: bool,
    /// Whether a group changed the blending space in force, with something compositing in
    /// it, while colours were being resolved for a space that is not the device's.
    ///
    /// The record a *group-scoped* pair run reads where the page-scoped construction reads
    /// [`Interpreter::blending_changed`]: §11.6.6's departure reports fire only where the
    /// device's components are what is composited on, so a departure met during a pair's
    /// subtractive runs would otherwise be drawn approximately in silence. The pair run
    /// that finds this set discards its pair and re-runs on the device, where the same
    /// group reports ordinarily. Scoped by [`Interpreter::group_commands`] around a pair
    /// attempt and left alone everywhere else, so that a departure inside a nested
    /// ordinary group still reaches the pair enclosing it; a soft mask's run restores it
    /// exactly, since a space inside a mask is the mask's own (ADR 0276).
    nested_space_departed: bool,
    /// Why the four components §11.4.7 names cannot be sampled into a press, if they cannot.
    ///
    /// [`PagePress::Beyond`]'s reason, carried into the report. Since the
    /// four-hundred-and-thirty-sixth session a press a *document* names is drawn rather than
    /// reported (ADR 0272), so what is left here is a four-component space that is not an ICC
    /// profile, and a page naming more distinct presses than [`crate::colour::MAX_PRESSES`].
    blending_beyond: Option<transparency::BeyondPress>,
    /// The distinct presses this interpretation has named, and the budget it spends on them.
    ///
    /// Shared by every run of the page — §11.4.7's pair is one content stream interpreted
    /// twice — so that a press is sampled once and counted once. ADR 0417.
    presses: &'a crate::colour::Presses,
}

/// Applies the `d` dash operator.
///
/// **This comment said "only the 'solid line' case is honoured for now" until the
/// two-hundred-and-twenty-first session**, describing the code of the ninth and not the code
/// below it: the tenth read the array (ADR 0018), and until it did, not one dashed line in 974
/// documents was dashed. That is the handover's archetype — "[t]he archetype is the `d`
/// operator" — and its own doc comment was still the sentence from before the fix.
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

#[cfg(test)]
mod tests {
    use pdf_render::Point;

    use super::{FontCache, base_transform, displayed_size, interpret_with, interpret_with_fonts};
    use crate::page::Page;

    /// Corpus documents with more than one page, and one that §11.4.7 interprets twice.
    ///
    /// The first four each draw twenty-odd fonts across their pages and reuse nearly all of
    /// them, which is what makes the equality below worth asserting; `bug1755507.pdf` sets a
    /// `DeviceCMYK` blending space, so `interpret_with` runs the subtractive pair over one page
    /// and the two halves share one cache — the *other* population a kept font reaches.
    const REUSING_PAGES: [&str; 5] = [
        "tracemonkey_annotation_on_page_8.pdf",
        "comments.pdf",
        "issue12337.pdf",
        "issue19239.pdf",
        "bug1755507.pdf",
    ];

    /// The whole of an interpretation, as a string two arms can be compared by.
    ///
    /// The `Debug` rendering rather than [`pdf_render::DisplayList::geometry_digest`], which
    /// hashes the *geometry* and would pass a difference in a paint — and a font decides a
    /// glyph's outline, so a wrong font is exactly a difference this must not miss.
    /// `examples/display_list_digest` compares two revisions the same way.
    fn everything(interpretation: &super::Interpretation) -> String {
        format!(
            "{:?}|{:?}|{}|{}|{}",
            interpretation.display_list,
            interpretation.unsupported,
            interpretation.text,
            interpretation.glyphs,
            interpretation.view_dependent,
        )
    }

    /// ADR 0710. A font kept from an earlier page must change what the next page *costs* and
    /// nothing about what it says — the property `CLAUDE.md` rests the oracle's comparison on.
    ///
    /// **Each assertion was calibrated against the defect it is about** (trap 13), and what that
    /// found is the reason there are three rather than one:
    ///
    /// - Against a `FontCache::get` that answers with *any* held font instead of the keyed one,
    ///   the comparison fails on page 2 of the first document. So the equality catches a key
    ///   confusion **inside** a document, which is what it is for.
    /// - Against a `FontCache::bind` stubbed never to rebind, it **passes**, and only the
    ///   `rebound` assertion fails. Four documents' font dictionaries simply do not land on the
    ///   same object numbers, so a cache that answered across files would not be caught by
    ///   comparing pictures — which is exactly why the binding is asserted directly rather than
    ///   trusted to show up as a wrong glyph.
    /// - Without `hits > 0` the whole comparison could pass with the cache never consulted.
    #[test]
    fn a_kept_font_changes_what_a_page_costs_and_not_what_it_says() {
        let corpus =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
        let shared = FontCache::new();
        let mut compared = 0_usize;
        for name in REUSING_PAGES {
            let Ok(bytes) = std::fs::read(corpus.join(name)) else {
                continue;
            };
            let Ok(document) = pdf_syntax::Document::open(bytes) else {
                continue;
            };
            let state = crate::view::ViewState::of(&document);
            let pages = crate::Pages::new(&document);
            for index in 0..pages.len().min(5) {
                let Some(page) = pages.get(index) else { break };
                // Alone first: a fresh cache per call is what every other caller in this
                // workspace does, and it is the answer the kept one has to reproduce.
                let alone = everything(&interpret_with(&document, &page, &state));
                let kept = everything(&interpret_with_fonts(&document, &page, &state, &shared));
                assert_eq!(
                    alone,
                    kept,
                    "{name} page {} differs when a font is kept",
                    index + 1
                );
                compared = compared.saturating_add(1);
            }
        }
        assert!(compared > 0, "no multi-page corpus document was readable");
        let report = shared.report();
        assert!(
            report.hits > 0,
            "the cache answered nothing, so the comparison above tested nothing: {report:?}"
        );
        assert!(
            report.rebound > 0,
            "one cache walked several documents without rebinding once: {report:?}"
        );
        assert!(
            report.bytes <= report.budget,
            "the budget was exceeded: {report:?}"
        );
    }

    /// A budget smaller than one font holds nothing, and answers exactly as an empty cache does.
    ///
    /// The eviction path, which no document this project owns reaches at [`super::FONT_BUDGET`]
    /// — `examples/font_cache_budget` is where that is measured — and which is therefore
    /// otherwise unexercised. `pdf_syntax::DecodedStreams::with_budget` exists for this reason
    /// and this test is the same shape.
    #[test]
    fn a_budget_too_small_for_one_font_keeps_none_and_changes_no_answer() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/pdf.js/test/pdfs/tracemonkey_annotation_on_page_8.pdf");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let Ok(document) = pdf_syntax::Document::open(bytes) else {
            return;
        };
        let state = crate::view::ViewState::of(&document);
        let pages = crate::Pages::new(&document);
        let starved = FontCache::with_budget(64);
        for index in 0..pages.len().min(3) {
            let Some(page) = pages.get(index) else { break };
            let alone = everything(&interpret_with(&document, &page, &state));
            let kept = everything(&interpret_with_fonts(&document, &page, &state, &starved));
            assert_eq!(
                alone,
                kept,
                "page {} differs under a starved cache",
                index + 1
            );
        }
        let report = starved.report();
        assert_eq!(report.fonts, 0, "a 64-byte budget kept a font: {report:?}");
        assert_eq!(
            report.hits, 0,
            "a 64-byte budget answered a lookup: {report:?}"
        );
    }

    /// A page 400 wide and 200 tall, with no crop offset, at `rotate` degrees.
    fn landscape(rotate: u16) -> Page {
        Page {
            id: None,
            dict: pdf_syntax::Dictionary::default(),
            resources: pdf_syntax::Dictionary::default(),
            media_box: [0.0, 0.0, 400.0, 200.0],
            substituted_media_box: None,
            damaged_dictionary: None,
            crop_box: [0.0, 0.0, 400.0, 200.0],
            bleed_box: [0.0, 0.0, 400.0, 200.0],
            trim_box: [0.0, 0.0, 400.0, 200.0],
            art_box: [0.0, 0.0, 400.0, 200.0],
            display_box: [0.0, 0.0, 400.0, 200.0],
            clip_box: [0.0, 0.0, 400.0, 200.0],
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
            let size = displayed_size(&page);
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
