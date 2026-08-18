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
    BlendMode, ClipId, Color, DisplayList, LineCap, LineJoin, Paint, Point, Size, SoftMaskId,
    Stroke, Transform,
};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::colour::{ColourSpace, Compositing};
use crate::page::Page;

use colour::{BlackPoint, output_intent_space};
use ext_gstate::Transfer;
use font::{Font, FontKey};
use pattern::{PatternPaint, shading_with_alpha};
use run::narrow;
use text::Coverage;
use transparency::{PagePress, page_blending_space, page_press};

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

pub use report::{
    ArtifactSpan, DamagedStream, Interpretation, MarkedSpan, Placed, UnnamedCodes, Unsupported,
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
/// **Every document on the web that reaches this bound is such a cycle**, which the
/// four-hundred-and-thirty-fifth session established by lifting it sixteenfold to 256 in a
/// scratch build and running the four of 65 944 that reported it: all four reached 256 as
/// well. So this is the one of the four bounds whose population is entirely the attack it
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
    /// Whether black point compensation applies to CIE-based conversions.
    ///
    /// ISO 32000-2 §8.6.5.9. `Default` is the initial value and leaves the choice to the
    /// processor; this one compensates, which is what makes blacks black.
    black_point: BlackPoint,
    /// The current fill colour space, which decides how `sc`/`scn` operands are read.
    fill_space: ColourSpace,
    /// As above, for stroking.
    stroke_space: ColourSpace,
    /// §10.5's transfer function, where an `/ExtGState` sets one.
    ///
    /// `None` is the initial value and what `/Identity` or `/TR2`'s `/Default` restores. Saved and
    /// restored by `q`/`Q` like every other parameter here, and inherited by a form `XObject` and by
    /// a tiling pattern's replay, which is what §8.4's "graphics state" means.
    transfer: Option<Arc<Transfer>>,
    /// Table 57's `/SM`, §10.7.3's smoothness tolerance, if the file states one.
    ///
    /// `None` is the initial value in the sense that matters: no document has asked for
    /// anything, so this device's own resolution stands. See `Ramp::resolution_for`.
    smoothness: Option<f32>,
    /// §11.6.4.3's `/AIS`, Table 57's alpha source flag: whether the soft mask and the two
    /// alpha constants state *shape* rather than opacity.
    ///
    /// A graphics state parameter like any other in this struct — set by `gs`, saved and
    /// restored by `q`/`Q` — and carried here so that [`Interpreter::alpha_is_shape`] can be
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
            transfer: None,
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
        // §10.5's transfer function, applied here because here is where a colour becomes the
        // value a device receives: the clause puts it "after performing any needed conversions
        // between colour spaces", and by this point `fill` is already RGB.
        Paint::Solid(self.transferred(Color {
            a: self.fill.a * self.fill_alpha,
            ..self.fill
        }))
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
        Paint::Solid(self.transferred(Color {
            a: self.stroke_colour.a * self.stroke_alpha,
            ..self.stroke_colour
        }))
    }

    /// One colour through §10.5's transfer function, or unchanged where none is in effect.
    fn transferred(&self, colour: Color) -> Color {
        self.transfer
            .as_ref()
            .map_or(colour, |transfer| transfer.apply(colour))
    }
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
    // ISO 32000-2 §11.4.7 puts a colour space under the whole page — "[a]ll page-level
    // compositing shall be done in the default blending colour space of the page" — and where
    // that space is `DeviceCMYK` this tree draws the page in it rather than on the device's
    // three components. §11.3.4 makes the compositing formula per component, so four
    // components are three plus one: the page is interpreted twice, once carrying cyan,
    // magenta and yellow and once carrying black, and `pdf_render::blending` puts the two
    // rasters back together where the clause puts the conversion. ADR 0262.
    if let PagePress::In(press) = page_press(document, page) {
        let (chromatic, drawable) = interpret_into(
            document,
            page,
            state,
            Compositing::Subtractive(crate::colour::Half::Chromatic, press),
        );
        if drawable {
            let (black, _) = interpret_into(
                document,
                page,
                state,
                Compositing::Subtractive(crate::colour::Half::Black, press),
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
                    .set_blending(crate::colour::blending_space_of(press), black.display_list);
                return chromatic;
            }
        }
    }
    interpret_into(document, page, state, Compositing::Device).0
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
    ) -> Self {
        let size = displayed_size(page);
        // §6.3.2.2's "unless otherwise instructed", asked once per page and only where a host
        // has instructed: a document nobody has said this about pays one enum comparison, and
        // one that has pays a walk of §12.7.4.1's field tree — the same walk `Query::Fields`
        // already makes for the same page, which is what keeps the two sets identical rather
        // than similar.
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
            text_operations: 0,
            glyph_coverage: BTreeMap::new(),
            glyphs: 0,
            codes_without_a_glyph: 0,
            codes_reaching_a_blank_glyph: 0,
            codes_without_a_character: UnnamedCodes::default(),
            operations: 0,
            fonts: BTreeMap::new(),
            text: String::new(),
            described: Vec::new(),
            artifacts: Vec::new(),
            marked: Vec::new(),
            inferred_separators: 0,
            text_layer: Vec::new(),
            associated: Vec::new(),
            reversed_chars: 0,
            view_dependent: false,
            text_cursor: None,
            base: base_transform(page),
            page: size,
            shadings: crate::shading::Cache::default(),
            resource_tables: std::cell::RefCell::default(),
            icc_spaces: BTreeMap::new(),
            image_masks: crate::image::MaskCache::default(),
            image_rasters: crate::image::RasterCache::default(),
            structure: crate::structure::ParentTree::for_page(document, &page.dict),
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
            alpha_is_shape: false,
            compositing,
            blending: page_blending_space(document, page),
            blending_changed: false,
            black_generation_stated: false,
            nested_space_departed: false,
            blending_beyond: match page_press(document, page) {
                PagePress::Beyond(why) => Some(why),
                PagePress::Device | PagePress::In(_) => None,
            },
        }
    }
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
) -> (Interpretation, bool) {
    // **The page's `/Contents` is read through a window and never assembled into one buffer**,
    // which is road D of `doc/todo/10` §5 and ADR 0365. What it buys, measured: a
    // decompression bomb costs 8.4 MB of resident memory instead of a gibibyte, and the
    // largest honest content stream this project has met — 141 MiB in one part — is
    // interpreted from 194 MB instead of 381 MB. What it costs is 5.74% of the instructions
    // to interpret an ordinary page, and one report arriving late: a part damaged half way
    // through is met half way through, so the reader is asked twice, here and after the run.
    let mut reader = reader::ContentReader::for_page(document, page);
    let mut interpreter = Interpreter::for_page(document, page, state, compositing);

    for issue in reader.take_issues() {
        interpreter.note(Unsupported::Content { issue });
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
    // §12.2's `/ViewClip`: "the page boundary to which the contents of a page shall be
    // clipped when viewing the document on the screen". Where it names the same region the
    // page is displayed at — which it does for every document that states no preference, and
    // for all 974 corpus documents — there is nothing to clip and no clip is built.
    let view_clip = interpreter.view_clip(page, base);
    let mut initial = GraphicsState::initial(base);
    initial.clip = view_clip;
    interpreter.run_reader(&mut reader, &page.resources, &initial, 0);
    // §7.4.1's second half, for a part whose damage the pump met while the page was being
    // drawn: the bytes are on the page and the shortfall is in the report (ADR 0343). The
    // order the two loops find issues in does not matter — `note` collects them into a map
    // and `finished` sorts it.
    for issue in reader.take_issues() {
        interpreter.note(Unsupported::Content { issue });
    }
    // §12.5: an annotation is drawn *over* the page content, and in `/Annots` order, so
    // this pass follows the content stream rather than being folded into it.
    interpreter.draw_annotations(page, base, view_clip);
    // Asked once the page is complete, because the condition is about the whole of it: an
    // annotation composites into the page group exactly as the content stream's marks do.
    interpreter.note_page_blending_space();

    // A font that drew *nothing* of what it was asked to show. Two ways to get there and one
    // condition: §9.10.2 gave the codes characters and the substitute face has none of them,
    // or the program — embedded or not — answers every code with no outline. Reported per font
    // and only where the count of glyphs drawn through it is zero, which is what keeps a space
    // and a deliberate `.notdef` from being news; see `Interpreter::glyph_coverage`.
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
    text_operations: usize,
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
    /// Codes shown that §9.10.2 could not name; see
    /// `Interpretation::codes_without_a_character`.
    codes_without_a_character: UnnamedCodes,
    operations: usize,
    /// Fonts already loaded, keyed by resource name.
    ///
    /// A page names the same font on every `Tf`, and parsing a font program is expensive,
    /// so this is what keeps text rendering from being dominated by font loading.
    fonts: BTreeMap<FontKey, Option<Font>>,
    /// Maps PDF user space to page space.
    ///
    /// Pattern space is defined relative to the page's default coordinates rather than to
    /// the transform in force when a pattern is used, so this is kept for patterns and
    /// must not be confused with the current transform.
    base: Transform,
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
    structure: crate::structure::ParentTree,
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
    /// Whether §11.6.4.3's `/AIS` has been in force anywhere in the content being run.
    ///
    /// While it is, a mask and the alpha constants are *shape* rather than opacity, and
    /// `stated_shape` — which builds a knockout element's shape by removing exactly those
    /// two — states the wrong quantity. A knockout group any of whose elements may have
    /// been painted under it is therefore refused by name.
    ///
    /// Monotone **within one group's content** rather than across the page, which is the
    /// four-hundred-and-ninety-second session's narrowing: the entry is a graphics state
    /// parameter, so `q`/`Q` bound it, and a `gs` inside one form said nothing about a
    /// sibling form's group — yet the page-wide flag refused every knockout group after it
    /// (`issue18032.pdf` states it inside a form whose group draws nothing at all, two
    /// forms before the knockout group it cost). [`Interpreter::run_transparency_group`]
    /// seeds this from [`GraphicsState::alpha_is_shape`] — the value actually in force at
    /// the `Do` — runs the content, reads what the run left, and restores the enclosing
    /// value OR-ed with it, so an enclosing group still sees a nested `gs`. Within one
    /// scope it stays an over-approximation ("stated while the content ran", not "a mark
    /// was painted under it"), which errs toward the report.
    alpha_is_shape: bool,
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
    /// profile and a profile arriving after this process has sampled its last.
    blending_beyond: Option<&'static str>,
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

    use super::{base_transform, displayed_size};
    use crate::page::Page;

    /// A page 400 wide and 200 tall, with no crop offset, at `rotate` degrees.
    fn landscape(rotate: u16) -> Page {
        Page {
            id: None,
            dict: pdf_syntax::Dictionary::default(),
            resources: pdf_syntax::Dictionary::default(),
            media_box: [0.0, 0.0, 400.0, 200.0],
            substituted_media_box: None,
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
