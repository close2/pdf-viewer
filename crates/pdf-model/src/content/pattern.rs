//! §8.7's patterns and shadings: the `sh` operator, shading paints and tiling cells.
//!
//! A shading pattern travels into the display list as a paint; a tiling pattern is a content
//! stream read **once** and its marks copied to every site the lattice reaches, which is why
//! most of this module is about placing, clipping and folding cells rather than about colour.

use std::rc::Rc;
use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{
    BlendMode, ClipId, Color, Command, DisplayListError, FillRule, Paint, Path, PathCommand, Point,
    Rect, Shading, ShadingKind, SoftMask, SoftMaskId, SoftMaskKind, Stroke, Transform,
    stroked_bounds,
};
use pdf_syntax::{Dictionary, Name, Object};

use crate::colour::ColourSpace;

use super::colour::{BlackPoint, Intent, convert};
use super::report::Unsupported;
use super::run::narrow;
use super::transparency::{Painted, any_command, command_blends, group_alpha_is_shape};
use super::{GraphicsState, Interpreter, MAX_OPERATIONS};

mod reach;

use reach::Reach;

/// The graphics state a shading pattern's *definition* is evaluated under (ISO 32000-2 §11.6.7).
///
/// A pattern is a colour (§8.7.2) and `scn` is where a colour is set, so the obvious moment to
/// resolve a shading pattern's colours is the `scn` — and the obvious repair, when that turns out
/// to be several graphics states before the mark, is to move it to the mark. **The clause names
/// neither**, twice and in two places. §11.6.7:
///
/// > The definition shall not inherit the current values of the graphics state parameters at the
/// > time it is evaluated; those parameters shall take effect only when the resulting pattern is
/// > later used to paint an object.
///
/// > Any parameters that are not so specified shall be inherited from the graphics state that was
/// > in effect at the beginning of the content stream in which the shading pattern is set to be
/// > the current colour in the graphics state or in which the sh operator is used.
///
/// and Table 75's `/ExtGState` entry says the same of the same parameters — "inherited from the
/// graphics state that was in effect at the beginning of the pattern's parent content stream, and
/// as modified by clause 11.6.7".
///
/// This tree already obeyed that rule for **one** parameter and had never noticed: §8.7.2's
/// pattern matrix maps to "the default coordinate system of the pattern's parent content stream",
/// which is `Interpreter::base`, swapped at each of the four ways of becoming a parent. The
/// transformation matrix is the first of the three §11.6.7 names; the other two are this struct.
///
/// # Which parameters, and why not the transfer function
///
/// §11.6.7's third bullet names them: "Only those parameters that affect the sh operator, such as
/// the current transformation matrix, black point compensation and rendering intent, shall be
/// used." §10.7.3's smoothness is a fourth by the same test — it is the tolerance a shading's
/// colour function is sampled to, and nothing else reads it.
///
/// §10.5's transfer function is deliberately **not** here, and the reason is §11.7.5.3's NOTE
/// about the intent it has just placed at the painting operation:
///
/// > This differs from the current halftone and transfer function, whose values are used only when
/// > all colour compositing has been completed and rasterization is being performed.
///
/// So a transfer is not part of a pattern's evaluation at all; §11.7.5.2 puts it at the topmost
/// painting object, which is the mark — and [`ShadingDefinition`] is what lets the mark have it
/// without any of these three moving with it.
#[derive(Debug, Clone, Copy)]
pub(super) struct PatternInitial {
    /// Table 57's `/UseBlackPtComp` as the beginning of this content stream had it.
    black_point: BlackPoint,
    /// §8.6.5.8's rendering intent, which §8.6.5.9 lets override the entry above.
    intent: Intent,
    /// Table 57's `/SM`, §10.7.3's smoothness tolerance.
    smoothness: Option<f32>,
}

impl PatternInitial {
    /// The parameters as one graphics state has them.
    ///
    /// Read off the state a content stream *begins* with, which is what makes it the clause's
    /// quantity — see [`Interpreter::run_reader`], the one place this is called.
    pub(super) fn of(state: &GraphicsState) -> Self {
        Self {
            black_point: state.use_black_pt_comp,
            intent: state.intent,
            smoothness: state.smoothness,
        }
    }

    /// The same parameters as Table 75's `/ExtGState` augments them, per §11.6.7's third bullet.
    ///
    /// > In the case of a shading pattern, the parameter values may be augmented by the contents
    /// > of the ExtGState entry in the pattern dictionary (see 8.7.4, "Shading patterns"). Only
    /// > those parameters that affect the sh operator, such as the current transformation matrix,
    /// > black point compensation and rendering intent, shall be used. Parameters that affect
    /// > path-painting operators shall not be used, since the execution of sh does not entail
    /// > painting a path.
    ///
    /// Three entries are read because three are what a shading's colours depend on here.
    /// Everything else Table 57 can carry is either a path-painting parameter the sentence
    /// excludes outright, one §11.6.7's first bullet has already initialised (the blend mode, the
    /// two alpha constants, the soft mask), or one this device does not perform at all —
    /// §10.6's halftone screen, §8.6.7's overprint, §10.4's black generation and undercolour
    /// removal.
    /// **A `/TR`, a `/TR2` or an `/HT` here is none of those and is deliberately not read**:
    /// §11.7.5.3's NOTE takes the transfer function out of the group evaluation, so a pattern's
    /// `/ExtGState` cannot state one for its own colours by either of §10.5's two routes. All three
    /// are silent for the same reason and none of them is reported;
    /// `Interpreter::note_black_generation` is what reports the entries that are skipped and
    /// could have marked, which is `/BG` and `/UCR`. **This sentence named a
    /// `note_pattern_ext_gstate` that is in no crate of this tree** until the
    /// six-hundred-and-seventy-seventh session found it while adding `/HT` to the list above.
    fn augmented(self, document: &pdf_syntax::Document, dict: &Dictionary) -> Self {
        let Some(state) = document.get_key(dict, "ExtGState").as_dict().cloned() else {
            return self;
        };
        let mut augmented = self;
        if let Object::Name(value) = document.get_key(&state, "UseBlackPtComp") {
            augmented.black_point = match value.as_bytes() {
                b"ON" => BlackPoint::On,
                b"OFF" => BlackPoint::Off,
                _ => BlackPoint::Default,
            };
        }
        if let Object::Name(intent) = document.get_key(&state, "RI") {
            augmented.intent = Intent::read(intent.as_bytes());
        }
        if let Some(tolerance) = document.get_key(&state, "SM").as_number() {
            augmented.smoothness = Some(narrow(tolerance));
        }
        augmented
    }

    /// Whether §8.6.5.9's black point compensation applies to the colours built under this.
    ///
    /// The same combination [`GraphicsState::black_point`] makes, for the same reason: the clause
    /// states the override over an object's intent rather than over the entry.
    fn black_point(self) -> BlackPoint {
        if self.intent == Intent::Absolute {
            return BlackPoint::Off;
        }
        self.black_point
    }
}

// Which of Table 57's entries §11.6.7 lets a shading pattern's `/ExtGState` state, and what
// becomes of each here. The clause admits "those parameters that affect the sh operator" and
// excludes those that "affect path-painting operators", and every entry falls into one of five
// buckets. Written down because a reader of `PatternInitial::augmented` will otherwise ask why
// three entries are read and twenty are not, and because the answer is the clause's for most of
// them rather than this tree's:
//
// - **Read there**: `/UseBlackPtComp`, `/RI`, `/SM` — the three §11.6.7's own sentence names, less
//   the transformation matrix, which Table 57 cannot state and which `Interpreter::base` already
//   carries.
// - **Excluded by §11.6.7's first bullet**, which initialises them: `/BM`, `/CA`, `/ca`,
//   `/SMask`, `/AIS`. A pattern's `/ExtGState` may not put a blend mode or an alpha constant
//   back, because "as always for transparency groups" they are the group's own, applied once
//   where the pattern is used.
// - **Excluded because they affect path painting** and `sh` "does not entail painting a path":
//   `/LW`, `/LC`, `/LJ`, `/ML`, `/D`, `/SA`, `/FL`, and the whole of Table 102's text state.
// - **Left to a standing decision recorded elsewhere**, because this device does not perform them
//   at all: `/OP`, `/op`, `/OPM` (§8.6.7's own permission), and `/BG`, `/BG2`, `/UCR`, `/UCR2` —
//   which are §11.7.5.3's conversion parameters and are noted by
//   `Interpreter::note_black_generation` below, since a pattern dictionary is a second route to a
//   statement `gs` already makes.
// - **`/TR`, `/TR2` and `/HT`**, which are none of those: §11.7.5.3's NOTE takes the transfer
//   function out of the group evaluation entirely, so one stated here says nothing about the
//   pattern's own colours. **This list put `/HT` in the bullet above until the
//   six-hundred-and-seventy-seventh session**, under "§10.6, inapplicable on the standard's own
//   condition" — and the condition covers a halftone *screen*, not the `TransferFunction` §10.5's
//   second bullet reads out of a halftone dictionary (ADR 0505). It is skipped here for the
//   transfer function's reason rather than the screen's, and skipping it is still right.
//   See `PatternInitial`.
impl Interpreter<'_> {
    /// Records Table 57's black generation and undercolour removal where a *pattern dictionary*
    /// states them (ISO 32000-2 §11.6.7's third bullet, Table 75's `/ExtGState`).
    ///
    /// `Interpreter::apply_ext_gstate` records the same statement for the `gs` operator and says
    /// what it costs; this is the second of the two routes §8.4.5's parameters have to the same
    /// page, and reading one of them is the failure mode that reports nothing. The flag is
    /// monotone for the page there and here for the same reason: the parameters apply wherever a
    /// §10.4.2.4 conversion happens, not only where they were set.
    fn note_black_generation(&mut self, dict: &Dictionary) {
        let Some(state) = self.document.get_key(dict, "ExtGState").as_dict().cloned() else {
            return;
        };
        if ["BG", "BG2", "UCR", "UCR2"]
            .iter()
            .any(|key| !matches!(self.document.get_key(&state, key), Object::Null))
        {
            self.black_generation_stated = true;
        }
    }
}

/// What a `/Pattern` colour space's `scn` selected.
///
/// The two kinds are drawn in completely different ways. A shading pattern is a paint and
/// travels into the display list as one. A tiling pattern is a *content stream*, drawn once
/// inside a clip shaped like the path being filled and copied to every other tile position —
/// so it never becomes a paint and is expanded here instead.
///
/// Both arms are one reference count wide because this rides in [`GraphicsState`], which `q`
/// copies: a page that selects a pattern and then saves and restores around every mark pays a
/// pointer per level rather than a shading, a box and a transfer apiece.
#[derive(Debug, Clone)]
pub(super) enum PatternPaint {
    /// A shading pattern (`/PatternType 2`).
    Shading(Rc<ShadingPattern>),
    /// A tiling pattern (`/PatternType 1`).
    Tiling(Rc<Tiling>),
}

/// A shading pattern as the `scn` left it, and everything the *mark* needs to finish it.
///
/// ISO 32000-2 §11.6.7 splits a shading pattern in two, and the split is this struct. One half
/// is fixed before the `scn` and no later operator may move it:
///
/// > The definition shall not inherit the current values of the graphics state parameters at the
/// > time it is evaluated; those parameters shall take effect only when the resulting pattern is
/// > later used to paint an object.
///
/// The other half belongs to whichever mark paints it, and the same clause says so in as many
/// words:
///
/// > This painting operation is subject to the values of the graphics state parameters in effect
/// > at the time, just as in painting an object with a constant colour.
///
/// [`ShadingDefinition`] is the first half; [`MarkColouring`] is the second. `shading` is the
/// colours one particular [`MarkColouring`] produced — recorded in `built`, so that a mark asking
/// for a different one can tell, and rebuild through `shading::Cache` rather than paint colours
/// the clause does not put there. See [`Interpreter::shading_paint`].
#[derive(Debug)]
pub(super) struct ShadingPattern {
    /// The colours and the geometry, as built under `built`.
    shading: Arc<Shading>,
    /// Table 77's `/BBox` where the pattern's shading states one, and the transform that
    /// places it.
    ///
    /// The box travels as a rectangle and a transform rather than as a `ClipId`, because
    /// the clause makes it a clip "in addition to the current clipping path … in effect at
    /// that time" — the time being when the *pattern is painted*, which may be several `q`
    /// levels away from the `scn` that selected it.
    bbox: Option<([f32; 4], Transform)>,
    /// What §11.6.7 fixed before the `scn`, which is all a rebuild is allowed to read.
    definition: ShadingDefinition,
    /// The mark-time colouring `shading` carries, which one mark later may no longer be current.
    built: MarkColouring,
}

/// The half of a shading pattern's colours ISO 32000-2 §11.6.7 fixes before the `scn`.
///
/// Held whole rather than resolved once because the colours are built again at the mark, and the
/// rebuild has to reach the same answer for every parameter but the two [`MarkColouring`] carries.
/// Keeping the definition's own [`PatternInitial`] *in* this struct is what makes that structural:
/// [`Interpreter::mark_colouring`] and [`Interpreter::build_shading`] take a `&ShadingDefinition`
/// and no `&GraphicsState`, so there is no argument through which the state at the mark could
/// supply the black point, the intent or the smoothness the clause has already decided.
#[derive(Debug)]
pub(super) struct ShadingDefinition {
    /// Table 75's `/Shading`, **unresolved**: `shading::Cache` is keyed by the reference, so a
    /// rebuild of a pattern painted a thousand times costs one build.
    object: Object,
    /// The resource dictionary §8.6.5.1 resolves the shading's `/ColorSpace` name through.
    resources: Dictionary,
    /// §8.7.2's pattern matrix composed with the parent content stream's default space.
    ///
    /// A property of the definition for the same reason as the three `initial` carries: §8.7.2 maps it to
    /// "the default coordinate system of the pattern's parent content stream", which is
    /// §11.6.7's first named parameter and which [`Interpreter::base`] has scoped since the
    /// fifty-second session.
    transform: Transform,
    /// §11.6.7's black point compensation, rendering intent and smoothness.
    initial: PatternInitial,
    /// Whether marks under this definition paint Table 77's `/Background`.
    ///
    /// True for a *non-stroking* selection and false for a stroking one, which is a statement
    /// about this tree rather than about the clause: Table 77's "the area to be painted" is the
    /// area of any painting operation, and all three backends draw the wash through the
    /// device-resolution raster lane a **fill** takes ([`pdf_render::ShadingRaster`]). A stroke
    /// therefore paints its shading and not its wash, which is a shortfall and is named
    /// ([`Unsupported::ShadingBackground`]) rather than left silent.
    ///
    /// It lives on the definition because a shading pattern's colours are rebuilt at the mark
    /// (§11.6.7, [`Interpreter::shading_paint`]), so a flag the rebuild could not see would be
    /// a rule somebody has to remember instead of one the type carries.
    paints_background: bool,
}

/// The half of a shading pattern's colours that belongs to the mark painting it.
///
/// Two quantities, each put here by a clause of its own rather than by symmetry with the other:
///
/// - **§11.7.2's compositing target**, inside [`crate::colour::Conversion`]. §11.6.7 makes the
///   pattern's definition a non-isolated group and §11.7.2 says "[n]on-isolated groups shall
///   inherit their colour space from the nearest ancestor isolated parent group" — which for a
///   pattern painted inside a group is *that* group, not whichever one the `scn` stood in.
/// - **§10.5's transfer function**, which §11.7.5.2 puts at "the last (topmost) elementary
///   graphics object enclosing that point" and §11.7.5.3's NOTE takes out of the group evaluation
///   altogether.
///
/// The black point [`crate::colour::Conversion`] also carries is the *definition's* and never the
/// mark's — [`Interpreter::mark_colouring`] is the only constructor, and it reads it from
/// [`ShadingDefinition`].
#[derive(Debug, Clone)]
pub(super) struct MarkColouring {
    /// §11.7.2's target, with §11.6.7's black point decision folded in.
    conversion: crate::colour::Conversion,
    /// §10.5's function as the mark's graphics state states it.
    transfer: Option<Arc<crate::content::Transfer>>,
}

impl MarkColouring {
    /// Whether two marks ask a pattern's definition for the same colours.
    ///
    /// The transfer functions are compared by `Arc::ptr_eq` rather than by value, which
    /// over-approximates in the direction that costs a rebuild rather than a wrong colour: two
    /// `gs` operators naming one `/ExtGState` parse two `Transfer`s that are equal and not
    /// identical, so a stream re-stating the same function between the `scn` and the mark builds
    /// colours that were already right. Equality of parsed §7.10 functions is a relation this
    /// tree does not have, and inventing one for a population
    /// `examples/pattern_state_census` measures would buy nothing.
    fn same_as(&self, other: &Self) -> bool {
        self.conversion == other.conversion
            && match (self.transfer.as_ref(), other.transfer.as_ref()) {
                (None, None) => true,
                (Some(one), Some(two)) => Arc::ptr_eq(one, two),
                _ => false,
            }
    }
}

/// What one pattern cell's commands fold, by position within the cell.
///
/// Each entry names a command's offset from the cell's first and which of its subpaths are the
/// second statement of a mark another cell also makes (§8.7.3.1, §11.6.2 — ADR 0213). It is
/// decided on the one cell that is interpreted and every site inherits it by being a copy of
/// that cell, for the reason [`Interpreter::fold_repeated_marks`] gives.
type CellFold = Vec<(usize, pdf_render::Repeats)>;

/// A tiling pattern: a cell of content, and how to repeat it.
#[derive(Debug)]
pub(super) struct Tiling {
    /// The cell's content stream, read **once** for the whole tiling (ADR 0430).
    ///
    /// Routed like the other three of §7.8.2's nested streams since the cell stopped being
    /// re-interpreted per site: a decode the memo declines is windowed, and a window is read
    /// once here rather than four thousand times. ADR 0427's exception ends with the loop that
    /// caused it.
    content: super::reader::NestedContent,
    /// The resources its operators name.
    resources: Dictionary,
    /// Spacing between cells, in pattern space. Never zero.
    step: (f32, f32),
    /// ISO 32000-2 §8.7.3.1 Table 74's `/BBox`, the pattern cell's bounding box, in pattern
    /// space.
    ///
    /// > These boundaries shall be used to clip the pattern cell.
    ///
    /// Carried per cell rather than applied once, because the clause clips *each* cell: a
    /// cell whose content runs past its own box would otherwise spill into the next cell's,
    /// and where `/XStep` exceeds the box — which is how a pattern tiles with gaps — it would
    /// spill into the gap between them. `/BBox` is required, so a pattern without one is
    /// malformed; it is then not clipped, which is the only reading that draws anything.
    bbox: Option<[f32; 4]>,
    /// Maps pattern space to the page's default space.
    to_page: Transform,
    /// The colour an uncoloured pattern is poured through, if it is uncoloured.
    ///
    /// `/PaintType 2` cells carry no colour of their own; the colour comes from `scn`.
    tint: Option<Color>,
}

/// Which mark a tiling pattern is the colour of (ISO 32000-2 §8.7.2).
///
/// > All patterns shall be treated as colours; ...
///
/// A colour is a colour whichever operator paints with it, so the same cell covers a path's
/// interior for `f` and a stroke's outline for `S`. The two differ in one thing only — the
/// region the tiles are cut to — and that is what this type carries.
///
/// # Why a stroke does not name a path here
///
/// The obvious construction for the stroking case is the outline as a path, tiled the way a
/// fill's path is; that is what [`Interpreter::tile`] refused to do until the
/// eight-hundred-and-second session, on the reason ADR 0028 gives — no crate that builds a
/// display list expands a stroke, all three backends do it themselves, and computing an
/// outline here would be a fourth expander in the one crate whose whole point is that it has
/// none. That reason is about *one* construction. The region a stroke covers is equally the
/// alpha of a group whose single element is that stroke (§11.5.2), and a soft mask is
/// already a command list every backend rasterises with the machinery it has — so the shape
/// travels as a `Command::Stroke` and each backend expands it with its own expander, exactly
/// once, exactly as it expands the strokes it already draws. ADR 0735.
#[derive(Debug, Clone, Copy)]
pub(super) enum Tiled<'a> {
    /// §8.5.3.3's fill: the region is the path's interior under this rule.
    Fill(FillRule),
    /// §8.4.3's stroke: the region is the outline these parameters deposit around the path.
    Stroke(&'a Stroke),
}

impl Interpreter<'_> {
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
    pub(super) fn paint_clip(&mut self, state: &GraphicsState, fill: bool) -> Option<ClipId> {
        let pattern = if fill {
            state.fill_pattern.as_ref()
        } else {
            state.stroke_pattern.as_ref()
        };
        let Some(PatternPaint::Shading(pattern)) = pattern else {
            return state.clip;
        };
        // The geometry, which is the half of a shading no colouring can move: a rebuild at the
        // mark states the same domain and the same matrix, so the clip is the selection's.
        let shading = Arc::clone(&pattern.shading);
        let bbox = pattern.bbox;
        let clip = match bbox {
            Some((corners, transform)) => self
                .rect_clip(corners, transform, state.clip)
                .or(state.clip),
            None => state.clip,
        };
        // §8.7.4.5.2's domain is where a type 1 shading marks *and nothing else* only where
        // the shading has no `/Background`. Where it has one, the same sentence says what
        // happens outside instead — those points "shall be painted with the shading's
        // background colour" — and `pdf_render::ShadingRaster` answers it per pixel, so a clip
        // here would cut away the very wash the entry asks for.
        if pattern.shading.background.is_some() {
            return clip;
        }
        self.domain_clip(&shading, clip)
    }

    /// The parallelogram a function-based shading's domain occupies, as a clip.
    ///
    /// ISO 32000-2 §8.7.4.5.2 states where a type 1 shading marks and where it does not:
    ///
    /// > The transformation matrix ( Matrix ) then maps the domain rectangle into a
    /// > corresponding rectangle or parallelogram in the target coordinate space. Points wi
    /// > thin the shading's bounding box ( BBox ) that fall outside this transformed domain
    /// > rectangle shall be painted with the shading's background colour ( Background ); if
    /// > the shading dictionary has no Background entry, such points shall be left unpainted.
    ///
    /// **"Or parallelogram" is the whole of it**, and it is why this is a clip rather than a
    /// property of the sampled grid. `function_based_shading.pdf` states
    /// `/Matrix [85 85 -85 85 515 382]` — a rotation — so its domain occupies a diamond, and
    /// this reader painted a *square* against four references' diamond for the project's whole
    /// life. The backend's pattern is padded outside its grid, which is right for the
    /// interpolation and says nothing about where the shading ends; where a shading ends is
    /// this clause's answer and so the interpreter's to compose, exactly as Table 77's
    /// `/BBox` is.
    ///
    /// **`Background` is not implemented and is reported**, which is Table 77's own gap rather
    /// than this function's: the ledger's §8.7.4.3 row carries it, and the entry applies only
    /// where a shading is used as a *pattern*, which is where [`Interpreter::pattern`] raises
    /// [`Unsupported::ShadingBackground`]. So this leaves the outside unpainted, which is the
    /// clause's branch for a shading that states *no* background, and a shading that states one
    /// is drawn the same way with the shortfall named. **Two earlier versions of this comment
    /// were wrong about it**: one claimed such a shading was refused before reaching here, and
    /// the sentence that corrected that one went on saying it "gets the same treatment silently"
    /// for as long as it was true.
    ///
    /// Nothing happens for any other shading type: an axial or radial shading says where it
    /// stops through `/Extend`, which its ramp already carries, and a mesh through its
    /// triangles.
    fn domain_clip(&mut self, shading: &Shading, parent: Option<ClipId>) -> Option<ClipId> {
        let ShadingKind::Sampled { domain, .. } = shading.kind.as_ref() else {
            return parent;
        };
        // Table 78's order is [x min x max y min y max], which is not `rect_clip`'s.
        let [x0, x1, y0, y1] = *domain;
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        self.list
            .add_clip(Clip {
                path,
                // The shading's own `/Matrix` is composed into this already, which is what
                // makes the domain rectangle's corners the right four points to send.
                transform: shading.transform,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .ok()
            .or(parent)
    }

    /// Takes a pattern cell's `/BBox` clip back off it where the clip removes no geometry.
    ///
    /// Returns whether it did, in which case the caller stops applying the box to the cells
    /// that follow — every cell is the same figure translated, so the first one decides.
    ///
    /// # Why a clip that removes nothing is not free
    ///
    /// Table 74 says a cell's box "shall be used to clip the pattern cell", and applying it
    /// where the cell draws nothing outside it is correct and costs a picture. A clip mask is
    /// anti-aliased, so a mark lying *on* the boundary keeps a fraction of its coverage in the
    /// boundary pixel and the neighbouring cell keeps the rest, and two fractions composite as
    /// `1 − (1−a)(1−b)` rather than adding. `issue16038.pdf` rules a grid with a line spanning
    /// exactly its own cell and lost **15% of the ink its geometry states** to that — measured
    /// by removing the clip, `AMBIGUOUS_TILING_CELL_CLIP`. The clip is load-bearing on the
    /// same page's *other* pattern, whose rule sits on the cell edge and is meant to be halved,
    /// which is why this is a question rather than a rule.
    ///
    /// # Why it is decided after the cell is drawn rather than before
    ///
    /// The extent of a cell's marks is not known until its content stream has run, and running
    /// it twice is not free of consequence: the readback, the text layer, the artifact spans
    /// and §9.3.8's overlap bookkeeping all accumulate as it goes. So the cell is drawn *with*
    /// the clip and the clip is removed afterwards, which needs no rollback at all — the
    /// commands carry their geometry and name their clip, and only the name changes.
    ///
    /// Conservative in three places, each of which keeps a picture rather than a saving: a
    /// command whose extent cannot be bounded, a command whose clip is a *chain* the cell's own
    /// content built on top of the box, and a box that fails to contain what the cell drew.
    fn unclip_redundant_cell(
        &mut self,
        mark: usize,
        corners: [f32; 4],
        offset: Transform,
        to_pattern: Transform,
        outer: Option<ClipId>,
    ) -> bool {
        let [x0, y0, x1, y1] = corners;
        let box_in_pattern = Rect::from_corners(
            offset.apply(Point::new(x0, y0)),
            offset.apply(Point::new(x1, y1)),
        );
        self.unclip_redundant(mark, box_in_pattern, to_pattern, outer)
    }

    /// Runs the pattern's content stream, once, for the cell `to_page` places.
    ///
    /// **Once for the whole tiling**, since ADR 0430: every other site is that cell's commands
    /// displaced, so this is the one interpretation the pattern gets. See [`pdf_render::Cell`]
    /// for why the two are the same picture.
    ///
    /// Returns the clip Table 74's box produced for it, which is what
    /// [`Interpreter::settle_cell_box`] may take back off the commands afterwards, or `None`
    /// where the pattern states no usable box.
    fn run_cell(
        &mut self,
        tiling: &Tiling,
        to_page: Transform,
        outer: Option<ClipId>,
    ) -> Option<ClipId> {
        let mut cell = GraphicsState::initial(to_page);
        // Table 74: "These boundaries shall be used to clip the pattern cell." The box is in
        // pattern space, so it travels with the cell's own offset, and it sits *inside* the
        // path's clip rather than replacing it — a cell is bounded by both. A file whose box
        // is unusable keeps the path clip alone.
        let box_clip = tiling
            .bbox
            .and_then(|corners| self.rect_clip(corners, to_page, outer));
        cell.clip = box_clip.or(outer);
        // An uncoloured pattern is a stencil: the colour given alongside the pattern name is
        // what pours through it. §8.6.8 is what makes that true of a cell whose content stream
        // *does* try to set a colour — it is the second of the clause's two circumstances, and
        // the colour operators inside it "shall be ignored" exactly as they are inside a `d1`
        // glyph description.
        let saved_uncoloured = self.uncoloured;
        if let Some(tint) = tiling.tint {
            cell.fill = tint;
            cell.stroke_colour = tint;
            self.uncoloured = true;
        }
        // §8.7.2's last sentence about nesting, which this reader did not apply until the
        // five-hundred-and-ninety-fifth session:
        //
        // > A pattern can be used within another pattern
        //
        // — and the rest of that sentence says the inner pattern's matrix defines its
        // relationship to the pattern space of the *outer* pattern. It is paraphrased rather
        // than quoted because the standard sets "relationship" broken across a line.
        //
        // So a pattern named inside a cell is anchored to *this cell's* space, exactly as one
        // named inside a form is anchored to the form's (`Interpreter::form`, which has done
        // this since it was written). Anchoring it to the page instead made the tiles differ
        // from one another, which §8.7.3.1's own picture forbids — "the effect is as if the
        // figure were painted on the surface of a clear glass tile, **identical copies** of
        // which were then laid down in an array" — and `issue8565.pdf` is the corpus document
        // that showed it: one radial gradient, page-anchored, under a cell the size of the
        // page.
        let outer_base = std::mem::replace(&mut self.base, to_page);
        self.run(&tiling.content, &tiling.resources, &cell);
        self.base = outer_base;
        self.uncoloured = saved_uncoloured;
        box_clip
    }

    /// What Table 74's box clip is doing to the cell, answered once for the whole tiling.
    ///
    /// Returns which of the cell's marks are to be folded, or nothing where the box came off
    /// the cell entirely. The two questions are asked in this order because they are the same
    /// question at two strengths: [`Interpreter::unclip_redundant_cell`] removes a box that
    /// cuts nothing at all, and [`Interpreter::fold_repeated_marks`] deals with a box that cuts
    /// a mark the cell states again a step away — a rule drawn on the box's own edge, which is
    /// one mark of the tiling described twice.
    ///
    /// Both answers reach every site, and since ADR 0430 they reach it by being *copied* rather
    /// than by being applied again: the cell is settled here and the sites are copies of what
    /// this left.
    fn settle_cell_box(
        &mut self,
        mark: usize,
        corners: [f32; 4],
        placement: (Transform, Transform),
        step: (f32, f32),
        clips: (Option<ClipId>, Option<ClipId>),
    ) -> CellFold {
        let (offset, to_pattern) = placement;
        if self.unclip_redundant_cell(mark, corners, offset, to_pattern, clips.1) {
            return CellFold::new();
        }
        self.plan_repeated_marks(mark, corners, placement, step, clips.0)
    }

    /// Finds a mark the cell states twice, a lattice step apart (§8.7.3.1, §11.6.2).
    ///
    /// Returns one entry per command that has one, by position within the cell.
    /// [`Interpreter::fold_repeated_marks`] carries the answer out, on this cell and on every
    /// cell after it: they are one figure at translations of each other, so what folds in one
    /// folds in all.
    ///
    /// # The figure this is for, and why the clip is not the answer
    ///
    /// A producer builds a continuous rule out of a repeating cell by drawing it **on** the
    /// box's edge and stating it twice, at the bottom of the cell and at the top. Table 74's
    /// clip is what keeps that from painting the rule twice at full width — each cell keeps the
    /// half inside its own box — and the halves meet exactly, in geometry. They do not meet on
    /// the raster: a clip mask is anti-aliased, so the boundary pixel keeps a fraction of one
    /// half and a fraction of the other, and two fractions painted one after another composite
    /// as `1 − (1−a)(1−b)` rather than adding. `issue16038.pdf`'s second square came out 13%
    /// under the ink its own geometry states, where §10.7.4 asks for at least it.
    ///
    /// §11.6.2 says the two halves may not composite at all:
    ///
    /// > Portions of an object shall not be composited with one another, even if they are
    /// > described in a way that would seem to cause overlaps
    ///
    /// and §11.6.7 is what makes the whole tiling one object's paint rather than many —
    /// "the colour, shape, and opacity values resulting from the evaluation of the pattern
    /// definition shall be used as the object's source colour ( 𝐶𝑠 ), object shape ( f j ), and
    /// object opacity ( qi )". So the tiling is evaluated to one shape first, and two cells'
    /// contributions to one device pixel are two portions of that shape.
    ///
    /// The fix is therefore neither a buffer nor a clip: it is to notice that the cell's two
    /// copies of the rule are **one mark of the tiling**, keep one of them, and draw it whole.
    /// [`pdf_render::repeated_subpaths`] carries the conditions under which that paints the same set
    /// of points, and refuses where it would not.
    fn plan_repeated_marks(
        &self,
        mark: usize,
        corners: [f32; 4],
        placement: (Transform, Transform),
        step: (f32, f32),
        box_clip: Option<ClipId>,
    ) -> CellFold {
        let (offset, to_pattern) = placement;
        // No box clip was built — `rect_clip` refused it — so nothing halves a mark and there is
        // nothing to fold away from.
        let Some(box_clip) = box_clip else {
            return CellFold::new();
        };
        let [x0, y0, x1, y1] = corners;
        let tiles = pdf_render::Tiles {
            step,
            cell: Rect::from_corners(
                offset.apply(Point::new(x0, y0)),
                offset.apply(Point::new(x1, y1)),
            ),
        };
        let mut plan = CellFold::new();
        for (at, command) in self.list.commands().iter().enumerate().skip(mark) {
            // A command the cell's own content clipped further is skipped, for the reason
            // `unclip_redundant` leaves one alone: the box is then only part of what bounds it.
            if command.clip() != Some(box_clip) {
                continue;
            }
            if let Some(repeats) = pdf_render::repeated_subpaths(command, tiles, to_pattern) {
                plan.push((at.saturating_sub(mark), repeats));
            }
        }
        plan
    }

    /// Carries out what [`Interpreter::plan_repeated_marks`] decided, on the cell at `mark`.
    ///
    /// Every cell states the same figure at a translation, so one cell's answer is every cell's.
    /// The guard is that the command at each planned position still draws a path with the number
    /// of subpaths the answer counted, and a cell that does not is reported rather than cut by
    /// index into something else. Nothing in the interpreter can produce one — the content stream
    /// and the graphics state are identical from cell to cell and only the transform differs — so
    /// the report is there to make a surprise audible rather than to describe a known case.
    fn fold_repeated_marks(
        &mut self,
        mark: usize,
        plan: &CellFold,
        clips: (Option<ClipId>, Option<ClipId>),
    ) {
        let (box_clip, outer) = clips;
        let mut commands = self.list.split_off_commands(mark);
        let mut owed = false;
        for (at, repeats) in plan {
            let folded = commands
                .get(*at)
                .filter(|command| command.clip() == box_clip)
                .and_then(Command::path)
                .and_then(|path| pdf_render::without_subpaths(path, repeats));
            match (folded, commands.get_mut(*at)) {
                (Some(path), Some(command)) => {
                    if let Some(slot) = command.path_mut() {
                        *slot = Arc::new(path);
                        command.set_clip(outer);
                    } else {
                        owed = true;
                    }
                }
                _ => owed = true,
            }
        }
        for command in commands {
            self.draw(command);
        }
        if owed {
            self.note(Unsupported::Shading {
                name: "a tiling pattern's cells do not all state the same figure".to_owned(),
            });
        }
    }

    /// Takes a `/BBox` clip back off the commands it enclosed, where it removes no geometry.
    ///
    /// The rule and its whole argument are `unclip_redundant_cell`'s, and the argument is not
    /// about tiling patterns: **a clip mask is anti-aliased, so a mark lying on the boundary
    /// keeps only a fraction of the boundary pixel** — and where the boundary cuts nothing, that
    /// fraction is ink the geometry states and nothing removes.
    ///
    /// `box_` is the clip's rectangle and `to_box` maps a command's own coordinates into the
    /// space that rectangle is stated in.
    ///
    /// Conservative in the same three places: a command whose extent cannot be bounded, a
    /// command whose clip is a *chain* built on top of the box, and a box that fails to contain
    /// what was drawn. Each keeps the picture rather than the saving. ADRs 0155 and 0165.
    pub(super) fn unclip_redundant(
        &mut self,
        mark: usize,
        box_: Rect,
        to_box: Transform,
        outer: Option<ClipId>,
    ) -> bool {
        let inside = self.list.commands().get(mark..).unwrap_or_default();
        if inside.is_empty() {
            return false;
        }
        let Some(bbox_clip) = inside.first().and_then(Command::clip) else {
            // The box produced no clip — `rect_clip` refused it, or there was none — so the
            // commands already carry the outer clip and there is nothing to take off.
            return false;
        };
        if Some(bbox_clip) == outer {
            return false;
        }
        for command in inside {
            let contained = pdf_render::marked_bounds(command, to_box)
                .is_some_and(|marks| box_.contains(marks));
            if !contained || command.clip() != Some(bbox_clip) {
                return false;
            }
        }

        let mut commands = self.list.split_off_commands(mark);
        for command in &mut commands {
            command.set_clip(outer);
        }
        for command in commands {
            self.draw(command);
        }
        true
    }

    /// Most commands one tiling may copy, whatever its cell holds and whatever the fill spans.
    ///
    /// A site is a copy of the cell's commands (ADR 0430), and a copy costs what a command
    /// costs: about 225 bytes of display list and, on this machine, two to three microseconds
    /// of rasterisation — so a tiling's cost is the commands it copies, and that is the unit this
    /// bound is in. `MAX_TILES` bounded the same loop in *sites* until ADR 0810, at 4096
    /// whatever the cell held, and the unit was the whole of what was wrong with it: a fill of
    /// 4480 sites of a two-command cell (`PDFIUM-1122-0.pdf`, 8960 commands) was refused its
    /// top fifth while a cell of forty commands was afforded 163 840.
    ///
    /// **The value is a choice, and it is sixteen times the count at a one-command cell.** What
    /// it admits and refuses was measured before it was chosen (ADR 0810): every tiling of the
    /// crawl's `7803372.pdf` (21 320), `4650000.pdf` (17 384), `7680183.pdf` (7610 at most of
    /// 249) and the head above draws whole, and the two that do not are the two that want a
    /// hundred thousand sites or more — `2760154.pdf`'s 762 930 and `PDFIUM-1497-2.pdf`'s
    /// 448 632 at four commands apiece, 1.8 million commands for one operator, eleven seconds
    /// and nine tenths of a gibibyte for the page. Why not the page's whole budget: one
    /// operator's expansion would then starve every operator after it, which is what
    /// `PDFIUM-1497-2.pdf` did to its own frame and title block when this was tried, and a
    /// page of sixty such fills would cost eleven seconds where the count cost two.
    ///
    /// **What those two are cut of was measured in the eight-hundred-and-ninety-first session,
    /// and it is less than this comment used to imply** (ADR 0828). Both arms in one sitting,
    /// `examples/open_one` at scale 1: `PDFIUM-1497-2.pdf` draws a **byte-identical** raster
    /// with the bound lifted, for 1.87 s against 10.53 and 0.19 GiB against 0.93; `2760154.pdf`
    /// is 0.33 s and 0.02 GiB against 2.08 s and 0.42, and its whole tiling is worth a mean of
    /// 1.087 of 255 — 33.583 of ink against 34.670, the pale wash behind its title. Eighty-one
    /// and ninety-four per cent of that gap is *rasterisation* rather than this list, so a
    /// display-list paint carrying a cell and its lattice for a backend to replicate as
    /// geometry would buy the memory and a fifth of the shorter gap. This comment sent the
    /// reader to a note that does not say so: §8.7.3.1's NOTE 2 is about `/XStep` and `/YStep`
    /// differing from the `/BBox`, and the sentence about a cell "evaluated once and then
    /// replicated" is §11.6.7's NOTE 1, which says it of the *opaque* imaging model (ADR 0827).
    /// `doc/todo/49` carries the closed item and `doc/checks/fixed-documents.toml` the two
    /// pages.
    const MAX_TILE_COPIES: usize = 65_536;

    /// Most edge tests one page may spend on `reach.rs`, proving which sites a fill can reach.
    ///
    /// The scan is a saving rather than a requirement, and a saving has to cost less than what
    /// it saves. A row costs [`reach::Reach::cost`] edge tests to ask about — every edge twice
    /// and every curve box once — and a row that reaches *no* site spends no copies, so nothing
    /// else in `repeat_cell` charges for it: a fill of a hundred thousand edges clustered at the
    /// foot of a lattice of a million rows would scan the other nine hundred thousand for
    /// nothing, at about four hundred billion tests. That is the same shape `MAX_TILES` was
    /// retired for — a loop whose trip count a file states and whose body a budget cannot see —
    /// one level in, so it is answered the same way: in the unit of the work.
    ///
    /// **Page-wide rather than per tiling**, because a page may state as many fills as
    /// [`MAX_OPERATIONS`] affords and a per-tiling allowance would multiply by that. Four
    /// million tests is about forty milliseconds here, and **the value is fifteen times the
    /// heaviest page measured** (ADR 0810): `7680183.pdf`, 249 hatched polygons of a plan and
    /// the page every other figure in that ADR is worst on, spends **under 300 000** across all
    /// of them; `PDFIUM-1497-2.pdf` and `2760154.pdf` spend fewer than 5000 rows' worth apiece.
    ///
    /// Running out is not a refusal and is not reported: the caller stops asking and keeps
    /// every site, which is what it does for a stroke and what it did before `reach.rs` existed.
    /// The direction is the safe one — more sites, never fewer — and those sites are still
    /// bounded by [`Self::MAX_TILE_COPIES`] and [`MAX_OPERATIONS`].
    const MAX_REACH_SCAN: usize = 4_194_304;

    /// Draws every site of a tiling but the one its cell was interpreted at.
    ///
    /// §8.7.3.1: "The pattern cell shall be replicated at fixed horizontal and vertical
    /// intervals to fill the area to be painted" — so what each site needs is the cell's marks
    /// displaced, which is [`pdf_render::Cell::repeat`].
    ///
    /// # What bounds it, which is what bounded it before
    ///
    /// Each copy is charged to [`MAX_OPERATIONS`], the budget the cell's *operators* were
    /// charged to when every site interpreted the content stream again. The trade is exact in
    /// the direction that matters: a command costs at least one operator to state, so no page
    /// that finished its tiling before reaches the bound now, and a cell that copies four
    /// million commands stops at the same place a cell that ran four million operators did.
    ///
    /// **Three things bound the site count since ADR 0810, and none of them is a count of
    /// sites.** Until the eight-hundred-and-eighty-second session a constant, `MAX_TILES`,
    /// capped the sites at 4096 whatever the cell held, and what kept it after ADR 0430 made a
    /// site a copy was the one case a charge per copy cannot see: a cell that drew *nothing*
    /// copies nothing, so its loop ran the trip count `/XStep` and `/YStep` state — 3.6 × 10¹¹
    /// of them for a thousandth of a unit over a 600-unit fill, about four days at 0.89 µs a
    /// trip (ADR 0271). But a cell with no marks replicated any number of times is no marks:
    /// §8.7.3.1's replication has nothing to replicate, so the loop is not entered at all. Every
    /// other cell costs at least one command a site, charged to the page's budget above and to
    /// [`Self::MAX_TILE_COPIES`], the tiling's own — the same cost bounded twice, once for the
    /// page and once so that one operator's expansion cannot take the page from the operators
    /// after it. And a site is only copied where the fill's interior can reach its cell at all
    /// (`reach.rs`), so a hatched wall costs its own area and not its hull's.
    ///
    /// The prefix a budget affords is whole rows, on the same argument ADR 0477 made for the
    /// constant: the sites are laid down row-major from the span's own corner, and a budget
    /// running out mid-row would put a ragged edge where the file states none. And a row is
    /// only the sites the fill's interior reaches — `reach` is that question, answered a row at
    /// a time — so a hatched wall costs its own area in copies and not its hull's (ADR 0810).
    ///
    /// **Asking that question is work too, and the page has a budget for it.** A row that
    /// reaches no site spends no copy, so a lattice of a million rows a fill barely touches
    /// would be scanned for nothing at no charge to either budget above — the same shape the
    /// retired count was retired for, one level in. [`Self::MAX_REACH_SCAN`] bounds the page's
    /// whole scan; past it the rows are taken whole, which loses the saving and nothing else.
    fn repeat_cell(
        &mut self,
        cell: &pdf_render::Cell,
        tiling: &Tiling,
        spans: ((i32, i32), (i32, i32)),
        reach: Option<&Reach>,
    ) {
        // A cell with no marks replicated any number of times is no marks, and the trip count
        // a file may state is the one cost a copy of nothing never charges — so the loop is not
        // entered. This line is what retired `MAX_TILES` (ADR 0810).
        if cell.is_empty() {
            return;
        }
        let ((first_column, last_column), (first_row, last_row)) = spans;
        let columns = (first_column, last_column);
        let mut rows_laid = 0usize;
        let mut spent = 0usize;
        for row in first_row..=last_row {
            // The sites of this row the fill's interior can reach at all (`reach.rs`); a
            // stroke's hull is taken whole, and so is every row once the page has spent its
            // scan (see [`Self::MAX_REACH_SCAN`] for why asking is itself charged).
            let sites = match reach {
                Some(reach) if self.reach_scanned < Self::MAX_REACH_SCAN => {
                    self.reach_scanned = self.reach_scanned.saturating_add(reach.cost());
                    reach.row(row, columns)
                }
                _ => vec![columns],
            };
            let count: usize = sites.iter().map(|&(a, b)| extent(a, b)).sum();

            // §8.7.3.1 puts the requirement on the processor rather than on the file: "[w]hen
            // performing painting operations such as S (stroke) or f (fill), the PDF processor
            // shall paint the cell on the current page as many times as necessary to fill an
            // area." A budget means some pages cannot have as many times as necessary — but
            // painting the cell *no* times is the furthest a processor can get from that
            // sentence, and the sites the budget does afford are the producer's own marks. So
            // the shortfall is reported and the affordable prefix is drawn, which is §7.8.2's
            // rule for a stream that decoded part-way (ADR 0343) applied to the second of the
            // two things a tiling is made of; the first — the cell's own content stream — has
            // drawn its prefix since ADR 0359. The prefix is whole rows, asked for row by row:
            // the sites are laid down row-major from the span's own corner, and a budget that
            // ran out mid-row would put a ragged edge where the file states none (ADR 0477).
            // Where one row alone is over budget the row is cut instead, by the check inside
            // the loop, since a prefix of one row is all there is to keep.
            let (bound, remaining) = tighter_of(
                (
                    "MAX_OPERATIONS",
                    MAX_OPERATIONS.saturating_sub(self.operations),
                ),
                (
                    "MAX_TILE_COPIES",
                    Self::MAX_TILE_COPIES.saturating_sub(spent),
                ),
            );
            let affordable = remaining.checked_div(cell.len()).unwrap_or(0);
            if count > affordable && rows_laid > 0 {
                self.note(Unsupported::LimitReached { limit: bound });
                return;
            }
            for (low, high) in sites {
                for column in low..=high {
                    if (column, row) == (first_column, first_row) {
                        continue;
                    }
                    let by = displacement(
                        tiling,
                        column.saturating_sub(first_column),
                        row.saturating_sub(first_row),
                    );
                    // Asked **before** the copy, not after it, and that is the whole of what
                    // keeps a nested tiling bounded. A cell that holds a tiling holds every one
                    // of its copies, so a chain of patterns each filling with the next is 9ⁿ
                    // commands — the span takes a neighbour on each side even for a fill inside
                    // one cell — and a check after the copy stopped only *this* loop: the parent
                    // tiling then copied a list already past the budget eight more times, and
                    // its parent that, so `ContentStreamCycleType3insideType3.pdf` cost 25 GB
                    // and a minute the day the nesting bound was raised past its cycle (ADR
                    // 0793). Refusing the copy that would cross the budget bounds the whole
                    // list at the budget plus one cell.
                    if self.operations.saturating_add(cell.len()) > MAX_OPERATIONS {
                        self.note(Unsupported::LimitReached {
                            limit: "MAX_OPERATIONS",
                        });
                        return;
                    }
                    if spent.saturating_add(cell.len()) > Self::MAX_TILE_COPIES {
                        self.note(Unsupported::LimitReached {
                            limit: "MAX_TILE_COPIES",
                        });
                        return;
                    }
                    match cell.repeat(&mut self.list, by) {
                        Ok(copied) => {
                            spent = spent.saturating_add(copied);
                            self.operations = self.operations.saturating_add(copied);
                            if self.operations > MAX_OPERATIONS {
                                self.note(Unsupported::LimitReached {
                                    limit: "MAX_OPERATIONS",
                                });
                                return;
                            }
                        }
                        Err(error) => {
                            self.note(Unsupported::LimitReached {
                                limit: match error {
                                    DisplayListError::TooManySoftMasks => "max_soft_masks",
                                    _ => "max_clips",
                                },
                            });
                            return;
                        }
                    }
                }
            }
            rows_laid = rows_laid.saturating_add(1);
        }
    }

    /// A stroke's own region, as the soft mask [`Tiled::Stroke`] cuts its tiles to.
    ///
    /// ISO 32000-2 §11.6.4.2 states an object's shape as 1.0 inside and 0.0 outside the path
    /// for a mark painted in a uniform colour, and then says what a pattern does to it:
    ///
    /// > For objects painted with a tiling pattern (8.7.3, "Tiling patterns") or a shading
    /// > pattern (8.7.4, "Shading patterns"), the shape shall be further constrained by the
    /// > objects that define the pattern (see 11.6.7, "Patterns and transparency").
    ///
    /// The clause's two factors are the two this construction multiplies: the mark's own shape,
    /// which is this mask, and the pattern's objects, which are the tiles it is put on.
    /// §11.5.2 states the first of the two derivations a soft mask has:
    ///
    /// > The mask value at each point shall then be derived from the alpha of the group.
    ///
    /// So a group whose one element is this stroke, taken for its alpha, is this stroke's
    /// shape — including whatever coverage a rasteriser gives its anti-aliased edge, which is
    /// the same quantity it would have given the stroke itself. §11.6.5.1 says the colour is
    /// irrelevant — "[t]he colours of the constituent objects shall be ignored" — so the
    /// element is painted opaque white and nothing reads it.
    ///
    /// `None` where the display list can hold no further mask, which the caller reports.
    fn stroke_shape(
        &mut self,
        path: &Arc<Path>,
        transform: Transform,
        stroke: &Stroke,
    ) -> Option<SoftMaskId> {
        let mask = SoftMask {
            commands: vec![Command::Stroke {
                path: Arc::clone(path),
                transform,
                stroke: stroke.clone(),
                paint: Paint::Solid(Color::WHITE),
                // The clip belongs to the tiles rather than to the shape: intersecting it
                // here as well would resolve one region twice, which is what the group below
                // this mask already declines to do.
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }],
            kind: SoftMaskKind::Alpha,
            transfer: None,
            luminance: None,
        };
        self.list.add_soft_mask(mask).ok()
    }

    /// Paints a tiling pattern over the area a path's fill or stroke covers.
    ///
    /// The region becomes a clip or a shape mask, the pattern's cell is drawn once inside it,
    /// and its marks are copied to every other tile position. Expanding the tiling here rather
    /// than inventing a display-list paint for it keeps the list flat: a backend never learns
    /// what a pattern is, and the result is resolution-independent because the cell is real
    /// geometry rather than a rendered image.
    ///
    /// [`Tiled`] says which of §8.7.2's two regions this is and why a stroke's arrives as a
    /// mask rather than as a clip.
    pub(super) fn tile(
        &mut self,
        path: &Arc<Path>,
        transform: Transform,
        region: Tiled<'_>,
        tiling: &Tiling,
        state: &GraphicsState,
    ) {
        // The pattern is anchored to the page, so the question "which cells does this path
        // touch" has to be asked in the pattern's own coordinates.
        let Some(to_pattern) = tiling.to_page.invert() else {
            self.note(Unsupported::Shading {
                name: "a tiling pattern's matrix is degenerate".to_owned(),
            });
            return;
        };
        let path_to_pattern = transform.then(to_pattern);

        // Which sites the region reaches. A fill's is the path's own hull; a stroke's is the
        // outline's, which `pdf_render::stroked_bounds` answers tightly — and it is asked in
        // *device* space, where §8.4.3.2's width is resolved, and the answer mapped into the
        // pattern's, because a zero width means one device pixel and a pattern unit is not one.
        let bounds = match region {
            Tiled::Fill(_) => bounds_of(path, path_to_pattern),
            Tiled::Stroke(stroke) => stroked_bounds(path, stroke, transform).map(|reach| {
                let reach = reach.mapped(to_pattern);
                (reach.min.x, reach.min.y, reach.max.x, reach.max.y)
            }),
        };
        let Some(bounds) = bounds else {
            return;
        };
        // How many sites that is, is not asked here: the span is cut to what the budget affords
        // in [`Interpreter::repeat_cell`], which is the first place the cell's own size is known
        // and therefore the first place the question has a unit (ADR 0810).
        let ((first_column, last_column), (first_row, last_row)) = spans(tiling, bounds);

        // What cuts the tiles to the region. A fill's interior is a clip, which is exactly
        // what a clip is; a stroke's outline is a shape mask, for [`Tiled`]'s reason — and
        // then the tiles carry the state's own clip instead, unchanged.
        let (clip, shape) = match region {
            Tiled::Fill(rule) => {
                // The path clips every cell, so a tile that falls outside it contributes
                // nothing.
                let clip = Clip {
                    path: (**path).clone(),
                    transform,
                    fill_rule: rule,
                    parent: state.clip,
                };
                let Ok(clip) = self.list.add_clip(clip) else {
                    self.note(Unsupported::LimitReached { limit: "max_clips" });
                    return;
                };
                (Some(clip), None)
            }
            Tiled::Stroke(stroke) => {
                let Some(shape) = self.stroke_shape(path, transform, stroke) else {
                    self.note(Unsupported::LimitReached {
                        limit: "max_soft_masks",
                    });
                    return;
                };
                (state.clip, Some(shape))
            }
        };

        // §11.6.7: "the pattern definition shall be treated as if it were implicitly enclosed
        // in a non-isolated transparency group: a non-knockout group for tiling patterns …
        // The definition shall not inherit the current values of the graphics state
        // parameters at the time it is evaluated; those parameters shall take effect only
        // when the resulting pattern is later used to paint an object." So every cell below
        // runs with the transparency parameters at their defaults — which is what
        // `GraphicsState::initial` gives it — and the state's own blend mode, alpha constant
        // and soft mask are applied *once*, to the finished tiling, by the group pushed after
        // the loop. NOTE 2 asks for exactly that shape: "[i]n a raster-based implementation of
        // tiling, it is advisable to treat all tiles as a single transparency group. This
        // avoids artifacts due to multiple marking of pixels along the boundaries between
        // adjacent tiles."
        //
        // Until the hundred-and-seventeenth session each cell inherited them instead, so an
        // `0.5 ca` under a pattern was applied per tile rather than to the pattern, and the
        // graphics state's soft mask reached nothing at all.
        let mark = self.list.command_count();

        // §11.6.4.4 puts the two alpha constants on different operators — `ca` on a fill and
        // `CA` on a stroke — and a pattern is the colour of one mark rather than of both, so
        // which constant applies to the finished tiling is which operator invoked it.
        let alpha = match region {
            Tiled::Fill(_) => state.fill_alpha,
            Tiled::Stroke(_) => state.stroke_alpha,
        };

        // The one interpretation the whole tiling gets, at the first site the span reaches.
        // §8.7.3.1's cell "shall be replicated at fixed horizontal and vertical intervals", and
        // a replica is this cell's commands displaced: see [`pdf_render::Cell`] for what makes
        // the two the same picture, and ADR 0430 for what it saves. Until the
        // five-hundred-and-ninety-fifth session the content stream was run once per site, which
        // is what made a bomb inside a cell cost its decode four thousand times over.
        let at = pdf_render::Mark::of(&self.list);
        let offset = Transform::translate(
            tiling.step.0 * as_f32(first_column),
            tiling.step.1 * as_f32(first_row),
        );
        // §11.7.5.2's sixth condition: "[i]f the current colour is a tiling pattern, all objects
        // in the definition of its pattern cell also satisfy the foregoing conditions." The cell
        // runs from `GraphicsState::initial` for §11.6.7's reason, so a mark inside it cannot see
        // the mark that invoked it; the four conditions are read off `state` here and carried
        // down, with `alpha` above naming which of §11.6.4.4's two constants the invoking mark
        // is under. **A stroke's shape mask is a fifth condition and it fails**: the tiles are
        // multiplied by a coverage that is below 1.0 wherever the outline is anti-aliased, so
        // the marks inside such a cell are not opaque and §11.7.5.2's sixth condition cannot be
        // met through them.
        let inside = self.opaque_ancestry
            && shape.is_none()
            && alpha >= 1.0
            && state.blend == BlendMode::Normal
            && state.soft_mask.is_none();
        let ancestry = std::mem::replace(&mut self.opaque_ancestry, inside);
        let box_clip = self.run_cell(tiling, offset.then(tiling.to_page), clip);
        self.opaque_ancestry = ancestry;
        // Table 74's box, and the marks it halves: both are settled on the cell itself, so
        // every site is a copy of the settled figure rather than a repetition of the question.
        if let Some(corners) = tiling.bbox {
            let plan = self.settle_cell_box(
                mark,
                corners,
                (offset, to_pattern),
                tiling.step,
                (box_clip, clip),
            );
            if !plan.is_empty() {
                self.fold_repeated_marks(mark, &plan, (box_clip, clip));
            }
        }

        // Taken after the box and the fold are settled, so that what every site copies is the
        // finished cell rather than the question.
        // `clip` is the path's own, which bounds the whole tiling: it is what tells a clip the
        // cell built from one that was already in force. See [`pdf_render::Cell`].
        let cell = pdf_render::Cell::drawn(&self.list, at, clip);
        // Which sites the region reaches at all. A fill's interior is scanned onto the lattice
        // (`reach.rs`), so a site whose cell box the interior never touches is not copied — a
        // hatched wall is a few per cent of its own hull. A stroke's outline is not a region
        // that scan can take, and its hull is kept whole.
        let box_extent = tiling
            .bbox
            .unwrap_or([0.0, 0.0, tiling.step.0, tiling.step.1]);
        let reach = match region {
            Tiled::Fill(rule) => Reach::of(
                path,
                path_to_pattern,
                rule,
                tiling.step,
                (box_extent[0], box_extent[2]),
                (box_extent[1], box_extent[3]),
            ),
            Tiled::Stroke(_) => None,
        };
        self.repeat_cell(
            &cell,
            tiling,
            ((first_column, last_column), (first_row, last_row)),
            reach.as_ref(),
        );

        // The two groups the finished tiling may want, and which of §11.6.4.1's sources of
        // shape and opacity each one carries.
        self.compose_tiling(mark, alpha, shape, state);
    }

    /// Wraps a finished tiling in the groups its region and its graphics state ask for.
    ///
    /// `mark` is where the tiles begin, `alpha` is whichever of §11.6.4.4's two constants the
    /// invoking operator is under, and `shape` is [`Tiled::Stroke`]'s region where there is one.
    /// Split out of [`Interpreter::tile`] because it is the only part of that function that is
    /// about compositing rather than about placing cells.
    fn compose_tiling(
        &mut self,
        mark: usize,
        alpha: f32,
        shape: Option<SoftMaskId>,
        state: &GraphicsState,
    ) {
        // A stroke's region, applied once to the finished tiling. §11.6.4.2 makes an object's
        // shape "1.0 inside and 0.0 outside" the mark it makes, and §11.5.2 derives a mask
        // from "the alpha of the group" — so a group holding the stroke alone, taken for its
        // alpha, *is* that shape, and multiplying the tiles by it is §11.3.7.1's `α = f × q`
        // with the tiling supplying `q`.
        //
        // It is a group of its own rather than the one below because the two carry different
        // quantities and a command has one mask slot: this one is the object's shape, the one
        // below is the state's own opacity source (§11.6.4.1's second and third sources). Where
        // the state sets none of its parameters this is the only group, which is why it is not
        // conditioned on `composites`.
        if let Some(shape) = shape {
            let parts = self.list.split_off_commands(mark);
            if parts.is_empty() {
                return;
            }
            let alpha_is_shape = group_alpha_is_shape(&parts, self.alpha_sources.settled());
            self.draw(Command::Group {
                commands: parts,
                // The state's constant rides the group below; this one only shapes.
                alpha: 1.0,
                clip: None,
                mask: Some(shape),
                blend: BlendMode::Normal,
                // §11.4.6's NOTE 6 again: a mask multiplies what the group produced, so the
                // group has to produce the tiling alone rather than the tiling over whatever
                // is under it. That is what isolation means, and it is the construction
                // rather than a choice.
                isolated: true,
                knockout: false,
                alpha_is_shape,
                blending: None,
            });
        }

        // The state's transparency parameters, applied once to the finished tiling. Where
        // they are all at their defaults there is nothing for a group to do and §11.4.4's
        // NOTE 5 says so in as many words — "the effect of compositing objects as a group is
        // the same as that of compositing them separately (without grouping)" — so the
        // commands stay inline and no page pays a buffer for a pattern that composites
        // trivially, which is almost every patterned page in the corpus.
        let composites =
            alpha < 1.0 || state.blend != BlendMode::Normal || state.soft_mask.is_some();
        if !composites {
            return;
        }
        let parts = self.list.split_off_commands(mark);
        if parts.is_empty() {
            return;
        }
        // §11.6.7 makes the implicit group *non-isolated*, and this one is isolated. Its own
        // NOTE 1 is what makes that exact wherever no element blends — "in the common case in
        // which the pattern consists entirely of objects painted with the Normal blend mode …
        // the results depend only on the colour, shape, and opacity of the pattern cell and
        // not on those of the backdrop" — and a cell that sets a blend mode of its own is the
        // case it is not, which is §11.4.4's report.
        // — and since ADR 0237 the display list can say the cell's own backdrop instead of
        // substituting §11.4.5's, on the three conditions `Command::Group`'s `isolated`
        // states. What is left to report is a cell composited under a blend mode of its own,
        // and a cell inside a knockout group, where the collapse those conditions rest on
        // does not hold.
        let isolated = self.inside_knockout
            || state.blend != BlendMode::Normal
            || !any_command(&parts, &command_blends);
        // §11.4.6's NOTE 6 reaches the implicit group too, because §11.6.7 makes the cell an
        // *element* of whatever paints it: a pattern painted inside a knockout group whose
        // initial backdrop is transparent has that backdrop rather than its immediate one, so
        // the isolated construction is the clause and there is no backdrop being excluded.
        if isolated && !self.transparent_initial_backdrop && any_command(&parts, &command_blends) {
            self.note(Unsupported::TransparencyGroup {
                detail: "non-isolated, and an element blends with the backdrop it excludes"
                    .to_owned(),
            });
        }
        // Asked of the cell's own marks under the `/AIS` reading the content ran under, the
        // way every other group is asked — see `group_alpha_is_shape`. It changes no pixel
        // today, because this group states no clip of its own, and it is stated truthfully
        // rather than as `false` so that the field means one thing everywhere it is written.
        let alpha_is_shape = group_alpha_is_shape(&parts, self.alpha_sources.settled());
        self.draw(Command::Group {
            commands: parts,
            alpha,
            // The tiles carry the path's clip already; a second copy on the group would be
            // the same region resolved twice.
            clip: None,
            mask: state.soft_mask,
            blend: state.blend,
            isolated,
            knockout: false,
            alpha_is_shape,
            // §8.7.3.1: a pattern cell's colours are resolved in the compositing already in
            // force, so the implicit group introduces no space of its own.
            blending: None,
        });
    }

    /// Paints a shading across the current clip, for the `sh` operator.
    ///
    /// `sh` covers the whole clipping region rather than a path, so the geometry drawn is
    /// the page itself and the clip does the shaping. Where the shading does not extend,
    /// it paints nothing, so the covered area is only ever as large as the shading says.
    pub(super) fn paint_shading(
        &mut self,
        name: &Name,
        resources: &Dictionary,
        state: &GraphicsState,
    ) {
        // `sh` marks the page and changes nothing else, so a hidden layer skips it whole —
        // including the report a shading we cannot build would otherwise make about a
        // shading that was never going to be drawn.
        if self.is_hidden() {
            return;
        }
        // The name's bytes do the finding (§7.3.5, `resources.rs`); the text says so afterwards.
        let label = String::from_utf8_lossy(name.as_bytes()).into_owned();
        let Some(object) = self.resource_entry(resources, "Shading", name) else {
            self.note(Unsupported::Shading {
                name: format!("/{label} is not in /Shading"),
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
        // §10.5's transfer function. An `sh` builds its shading at the mark, so the state here is
        // the state the clause asks about and nothing can have moved between them — and §11.6.4.4
        // makes `sh` a non-stroking painting operation, which is which of Table 52's two alpha
        // constants §11.7.5.2's first condition reads.
        let transfer = self.transfer_for_mark(state, Painted::Shading { stroking: false });
        let conversion = self.conversion(state);
        let colouring = crate::shading::Colouring::new(
            state.smoothness,
            &conversion,
            transfer.map(Arc::as_ref),
        );
        match self.shadings.build(
            self.document,
            &object,
            resources,
            state.transform,
            colouring,
        ) {
            Ok(shading) => {
                // Table 77's `/Background` "shall be applied only when the shading is used as
                // part of a shading pattern, not when painted directly with the sh operator";
                // §8.7.4.2 says the same of this operator — "[t]he Background entry, if
                // present, is ignored" — and §11.6.4.2 a third time, of the shape a `sh`
                // contributes: "1.0 inside and 0.0 outside the bounds of the shading's painti
                // ng geometry, disregarding the Background entry". `shading::Cache` is keyed
                // by the object and one object can be both a pattern's shading and an `sh`'s,
                // so the entry is resolved once where the colour space is and dropped here.
                let shading = Shading {
                    background: None,
                    ..shading
                };
                // §8.7.4.5.2's domain, which for a type 1 shading is where it marks at all.
                let clip = self.domain_clip(&shading, clip);
                let (path, transform) = self.shading_surface(&shading);

                self.draw(Command::Fill {
                    path: Arc::new(path),
                    transform,
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
                name: format!("/{label}: {error}"),
            }),
        }
    }

    /// The surface a `sh` fills, and the transform that places it.
    ///
    /// ISO 32000-2 §8.7.4.2 gives the operator no path at all:
    ///
    /// > This operator does not require the creation of a pattern dictionary or a path and
    /// > works without reference to the current colour in the graphics state.
    ///
    /// A display list fills paths, so one has to stand in for "wherever the shading marks".
    /// [`pdf_render::Shading::painting_bounds`] is §11.6.4.2's answer to where that is, and it
    /// is used wherever the shading has one; the page rectangle is the fallback for the
    /// shadings whose geometry is unbounded — axial and radial — where nothing smaller is
    /// true.
    ///
    /// # Why the page is not good enough on its own
    ///
    /// A `sh` inside a tiling pattern's cell is part of a figure the lattice repeats, and
    /// [`pdf_render::Cell::repeat`] displaces each copy's geometry — the shading with it, so
    /// that "every site would show the first site's gradient" does not happen. A page
    /// rectangle is displaced by the same lattice step while being no part of the figure, so
    /// the site whose shading arrives on the page is the site whose rectangle has just left
    /// it, and the tiling draws nothing at all. `0423269.pdf` in the `SafeDocs` crawl is two
    /// mesh-filled backgrounds lost that way, silently, because a shading that paints outside
    /// its own surface is not a refusal anything can report.
    ///
    /// # Why the rectangle is grown
    ///
    /// The mesh's own bounds pass through its outermost vertices, so an outline laid exactly
    /// on them shares an edge with the triangles it is there to admit — and a shared edge is
    /// counted twice, because coverage multiplies: two halves composite as a quarter, which is
    /// the arithmetic [`Interpreter::unclip_redundant`] documents for a cell's box. The margin
    /// only has to be non-zero, since it is the shading's own alpha and the current clip that
    /// bound the mark; a sixteenth of the geometry's own extent is that in whatever units the
    /// shading states, and costs nothing because it paints nothing.
    fn shading_surface(&self, shading: &Shading) -> (Path, Transform) {
        /// The share of a mesh's own extent the outline is grown by on each side.
        const MARGIN: f32 = 1.0 / 16.0;

        let (corners, transform) = match shading.painting_bounds() {
            Some([x0, y0, x1, y1]) => {
                let margin = (x1 - x0).abs().max((y1 - y0).abs()) * MARGIN;
                (
                    [x0 - margin, y0 - margin, x1 + margin, y1 + margin],
                    shading.transform,
                )
            }
            None => (
                [0.0, 0.0, self.page.width, self.page.height],
                Transform::IDENTITY,
            ),
        };
        let [x0, y0, x1, y1] = corners;
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        (path, transform)
    }

    /// Resolves a pattern name, for `scn` in a `/Pattern` colour space.
    ///
    /// §8.7.3.2 makes the operand a name into Table 34's `/Pattern` subdictionary:
    ///
    /// > This name shall be the key of an entry in the Pattern subdictionary of the current
    /// > resource dictionary (see 7.8.3, "Resource dictionaries"), whose value shall be the
    /// > stream object representing the pattern.
    ///
    /// A name that finds nothing there leaves the paint at §8.6.8's initial value for a
    /// `Pattern` space — "a pattern object that causes nothing to be painted" — so every
    /// subsequent fill and stroke in that space marks the page with nothing. That is why the
    /// miss is reported rather than left to look like a producer's transparent figure.
    pub(super) fn pattern(
        &mut self,
        name: &Name,
        resources: &Dictionary,
        tint: &[f32],
        state: &GraphicsState,
        fill: bool,
    ) -> Option<PatternPaint> {
        let label = String::from_utf8_lossy(name.as_bytes()).into_owned();
        let Some(object) = self.resource(resources, "Pattern", name) else {
            self.note_missing_resource("Pattern", name, "is not in /Pattern");
            return None;
        };
        let dict = match &object {
            Object::Dictionary(dict) => dict.clone(),
            Object::Stream(stream) => stream.dict.clone(),
            // §8.7.3.2's "value shall be the stream object representing the pattern" for a
            // tiling one, Table 75's dictionary for a shading one — so a `/Pattern` entry that
            // is neither is a pattern with no definition, which is the same failure as a name
            // the subdictionary omits arriving one step later.
            _ => {
                self.note_missing_resource("Pattern", name, "is not a dictionary or a stream");
                return None;
            }
        };

        match self.document.get_key(&dict, "PatternType").as_integer() {
            Some(1) => {
                return self
                    .tiling(&label, &object, &dict, tint, state, fill)
                    .map(PatternPaint::Tiling);
            }
            Some(2) => {}
            other => {
                self.note(Unsupported::Shading {
                    name: format!("/{label} is pattern type {}", other.unwrap_or(0)),
                });
                return None;
            }
        }

        // A pattern is positioned relative to the page's default space, not to the
        // transform in force where it is used. Getting this wrong moves every gradient on
        // the page by whatever the current transform happened to be.
        let matrix = crate::shading::matrix_of(self.document, &dict, "Matrix");
        // Unresolved on purpose: `shading::Cache` is keyed by the reference, and a pattern
        // painted a thousand times states the same one every time.
        let shading_object = dict.get("Shading").cloned().unwrap_or(Object::Null);

        // §11.6.7 and Table 75 both say which graphics state a shading pattern's definition is
        // evaluated under, and it is the one the *content stream began with*, augmented by the
        // pattern's own `/ExtGState` — neither the `scn` that selects it nor the mark that paints
        // it. §8.6.5.9's black point compensation, §8.6.5.8's intent and §10.7.3's smoothness
        // come from there, and they are carried in the definition so that no mark can reach past
        // them for the state's own.
        //
        // The colours built here are the ones a mark under *this* state would ask for. That is
        // the whole of what the `scn` decides: §11.7.2's compositing target and §10.5's transfer
        // function belong to the mark, so [`Interpreter::shading_paint`] compares and rebuilds
        // where the mark asks for something else. Building now rather than only at the paint is
        // what makes the common page — one state from the `scn` to every mark — one build.
        let definition = ShadingDefinition {
            object: shading_object,
            resources: resources.clone(),
            transform: matrix.then(self.base),
            initial: self.pattern_initial.augmented(self.document, &dict),
            // Table 77's `/Background` "shall be applied only when the shading is used as part
            // of a shading pattern, not when painted directly with the sh operator", so this is
            // the one place in the interpreter where the entry means anything at all.
            paints_background: fill,
        };
        self.note_black_generation(&dict);
        let built = self.mark_colouring(&definition, state.transfer.shared());
        match self.build_shading(&definition, &built) {
            Ok(shading) => {
                self.note_unpainted_background(&label, &definition.object, shading.background);
                Some(PatternPaint::Shading(Rc::new(ShadingPattern {
                    shading: Arc::new(shading),
                    // Stated "in the shading's target coordinate space", which for a pattern is
                    // the pattern space — the shading's own `/Matrix` (type 1 only) is applied
                    // inside `build` and comes *after* this.
                    bbox: crate::shading::bbox_of(self.document, &definition.object)
                        .map(|corners| (corners, definition.transform)),
                    definition,
                    built,
                })))
            }
            Err(error) => {
                self.note(Unsupported::Shading {
                    name: format!("/{label}: {error}"),
                });
                None
            }
        }
    }

    /// Names a `/Background` this mark will not paint, and stays quiet about one it will.
    ///
    /// Two cases reach here, and the report's condition is exactly those two (trap 11):
    ///
    /// - a **stroking** selection, whose wash [`ShadingDefinition::paints_background`] has
    ///   already dropped, so `resolved` is `None` while the dictionary states an array;
    /// - an array Table 77's own sentence cannot use — "an array of colour components
    ///   appropriate to the colour space" is a *count*, and one of any other length states no
    ///   colour that could be painted.
    ///
    /// A fill of a usable array is drawn ([`pdf_render::ShadingRaster`]) and owes nothing.
    fn note_unpainted_background(&mut self, label: &str, object: &Object, resolved: Option<Color>) {
        if resolved.is_some() {
            return;
        }
        if let Some(components) = crate::shading::background_components(self.document, object) {
            self.note(Unsupported::ShadingBackground {
                detail: format!("/{label} states a /Background of {components} component(s)"),
            });
        }
    }

    /// What a mark asks a shading pattern's definition to build its colours under.
    ///
    /// **The graphics state is not an argument here, and that is the design rather than an
    /// omission.** ISO 32000-2 §11.6.7 says a shading pattern's definition "shall not inherit the
    /// current values of the graphics state parameters at the time it is evaluated", and the
    /// three parameters that decide its colours — §8.6.5.9's black point compensation, §8.6.5.8's
    /// intent and §10.7.3's smoothness — are therefore taken from [`ShadingDefinition`]'s own
    /// [`PatternInitial`]. A signature that could reach a `&GraphicsState` would make that a rule
    /// somebody has to keep; this way the wrong version does not compile.
    ///
    /// The two quantities that *do* come from the mark arrive by the two routes the clauses give
    /// them: §11.7.2's compositing target through [`Interpreter::conversion_under`], which reads
    /// the target this run is compositing into, and §10.5's transfer function as the one argument
    /// — passed by a caller that has read `GraphicsState::transfer`, which is exactly what
    /// §11.7.5.2 puts at "the last (topmost) elementary graphics object enclosing that point".
    fn mark_colouring(
        &self,
        definition: &ShadingDefinition,
        transfer: Option<&Arc<crate::content::Transfer>>,
    ) -> MarkColouring {
        MarkColouring {
            conversion: self.conversion_under(definition.initial.black_point()),
            transfer: transfer.cloned(),
        }
    }

    /// Builds a shading pattern's colours, for a definition and one mark's colouring.
    ///
    /// The only place a shading pattern's colours are made, so the `scn` and every later mark
    /// cannot read §11.6.7's parameters differently. `shading::Cache` is keyed by the object, the
    /// resolution and the conversion, so the second mark asking for a colouring the first already
    /// built pays a lookup — the exception being a transfer function in force, which that table
    /// deliberately does not cache and which no document in either corpus states over a shading.
    ///
    /// # Errors
    ///
    /// See [`crate::shading::ShadingError`]. The caller decides what a failure means: at the
    /// `scn` it is a pattern that paints nothing and is named; at a mark it is a rebuild that
    /// falls back to the colours the selection made.
    fn build_shading(
        &mut self,
        definition: &ShadingDefinition,
        colouring: &MarkColouring,
    ) -> Result<Shading, crate::shading::ShadingError> {
        let shading = self.shadings.build(
            self.document,
            &definition.object,
            &definition.resources,
            definition.transform,
            crate::shading::Colouring::new(
                definition.initial.smoothness,
                &colouring.conversion,
                colouring.transfer.as_deref(),
            ),
        )?;
        if definition.paints_background {
            return Ok(shading);
        }
        Ok(Shading {
            background: None,
            ..shading
        })
    }

    /// The paint a shading pattern makes at the mark that is painting it.
    ///
    /// ISO 32000-2 §11.6.7 gives the painting operation its own sentence, and it is the reason
    /// this is not a field read:
    ///
    /// > This painting operation is subject to the values of the graphics state parameters in
    /// > effect at the time, just as in painting an object with a constant colour.
    ///
    /// Two of the parameters in effect at the time reach a shading's colours rather than the
    /// compositing that follows them — §10.5's transfer function, which §11.7.5.2 places at the
    /// topmost object painting the point, and §11.7.2's compositing space, which a non-isolated
    /// group inherits "from the nearest ancestor isolated parent group". Both are
    /// [`MarkColouring`], and where the mark's differs from the one the `scn` built under the
    /// colours are made again.
    ///
    /// # Why the colours cannot simply be mapped afterwards
    ///
    /// The same reason `shading::kind_of` applies §10.5 inside the sampling (ADR 0479): a `Ramp`
    /// reaching the display list has been through §10.7.3's simplifier, which drops every stop
    /// within half an eight-bit level of the line its neighbours draw. A `/FunctionType 2`
    /// interpolation with `/N 1` is two stops by then, and mapping two stops draws the chord
    /// where the clause asks for the curve.
    ///
    /// A rebuild that fails paints the colours the selection made and says so. The build
    /// succeeded once for this object, so a failure here is a resource the document changed under
    /// us rather than a shading this tree cannot read; drawing the earlier colours is closer to
    /// the page than dropping the mark.
    pub(super) fn shading_paint(
        &mut self,
        pattern: &ShadingPattern,
        transfer: Option<&Arc<crate::content::Transfer>>,
        alpha: f32,
    ) -> Paint {
        let wanted = self.mark_colouring(&pattern.definition, transfer);
        if wanted.same_as(&pattern.built) {
            return Paint::Shading(shading_with_alpha(&pattern.shading, alpha));
        }
        match self.build_shading(&pattern.definition, &wanted) {
            Ok(shading) => Paint::Shading(shading_with_alpha(&Arc::new(shading), alpha)),
            Err(error) => {
                self.note(Unsupported::Shading {
                    name: format!("a shading pattern's colours could not be rebuilt: {error}"),
                });
                Paint::Shading(shading_with_alpha(&pattern.shading, alpha))
            }
        }
    }

    /// The paint a non-stroking mark under `state` puts on the page.
    ///
    /// On the interpreter rather than on [`GraphicsState`] because a shading pattern's colours
    /// are the *mark's* (§11.6.7, [`Interpreter::shading_paint`]) and building them needs the
    /// document, the resource dictionary and `shading::Cache` — none of which a graphics state
    /// has. Every other paint is the state's own and comes straight back from
    /// [`GraphicsState::solid_fill`].
    ///
    /// §11.6.4.4's constant alpha is applied here, and reaching every colour a shading carries is
    /// the only way to apply it to one (`Shading::with_alpha`). Until the fifteenth session this
    /// was dropped: `alphatrans.pdf` states `Gradient: .5` on the page and draws its gradient
    /// over three other objects, and we painted it opaque while three references showed what was
    /// behind it.
    ///
    /// Called **once per mark**: a rebuild costs a build, so asking twice for one command would
    /// pay twice.
    ///
    /// `transfer` is §11.7.5.2's answer for this mark, from
    /// [`Interpreter::transfer_for_mark`], rather than the graphics state's own parameter — the
    /// two differ wherever the mark is not fully opaque, and the caller has asked already.
    pub(super) fn fill_paint(
        &mut self,
        state: &GraphicsState,
        transfer: Option<&Arc<crate::content::Transfer>>,
    ) -> Paint {
        let Some(PatternPaint::Shading(pattern)) = &state.fill_pattern else {
            return state.solid_fill(transfer.map(Arc::as_ref));
        };
        let pattern = Rc::clone(pattern);
        self.shading_paint(&pattern, transfer, state.fill_alpha)
    }

    /// As [`Interpreter::fill_paint`], for a stroking mark and §11.6.4.4's stroking constant.
    pub(super) fn stroke_paint(
        &mut self,
        state: &GraphicsState,
        transfer: Option<&Arc<crate::content::Transfer>>,
    ) -> Paint {
        let Some(PatternPaint::Shading(pattern)) = &state.stroke_pattern else {
            return state.solid_stroke(transfer.map(Arc::as_ref));
        };
        let pattern = Rc::clone(pattern);
        self.shading_paint(&pattern, transfer, state.stroke_alpha)
    }

    /// Reads a tiling pattern's cell and how it repeats.
    fn tiling(
        &mut self,
        name: &str,
        object: &Object,
        dict: &Dictionary,
        tint: &[f32],
        state: &GraphicsState,
        fill: bool,
    ) -> Option<Rc<Tiling>> {
        let stream = object.as_stream()?;
        // §8.7.3.1: "The appearance of the pattern cell shall be defined by a content stream
        // containing the painting operators needed to paint one instance of the cell." So the
        // cell is §7.8.2's sequence of instructions, a prefix of it is a cell with fewer marks
        // in the same places, and the tiling replicates that shorter cell at the file's own
        // `/XStep` and `/YStep` — nothing stands in place of the marks the damage took.
        // See [`Interpreter::content_stream`].
        //
        // A cell whose stream cannot be decoded at all is **reported**, which it was not before
        // the five-hundred-and-ninety-fifth session: the refusal was dropped here and the page
        // came back complete with the pattern silently unpainted. A form says
        // `undecodable form /Fx` in the same circumstance and always has, and this is the same
        // sentence for the other of §7.8.2's five.
        let Some(content) =
            self.content_stream(stream, &format!("a tiling pattern /{name} (§8.7.3.1)"))
        else {
            self.note(Unsupported::Operator {
                operator: format!("undecodable tiling pattern /{name}"),
            });
            return None;
        };

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
        // Normalised to (left, bottom, right, top): the clause names the four edges in that
        // order and producers write the corners in any of them, exactly as they do for a page
        // box. A box with no extent in either direction clips everything away, and Table 74's
        // NOTE 1 says otherwise — "[a] BBox of zero height or width will still paint one
        // pixel" — so it is left unclipped rather than emptied.
        let cell_box = match bbox.as_slice() {
            [x0, y0, x1, y1] if (x1 - x0).abs() > 0.0 && (y1 - y0).abs() > 0.0 => {
                Some([x0.min(*x1), y0.min(*y1), x0.max(*x1), y0.max(*y1)])
            }
            _ => None,
        };

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
                // Through `convert`, so that an uncoloured cell painted inside a
                // `/Luminosity` mask group is poured through §11.5.3's luminosity like every
                // other colour there. `BlackPoint::Default`: the tint arrived with `scn` and
                // §8.6.5.9's setting belongs to the state that paints the cell.
                Some(convert(
                    &space,
                    tint,
                    BlackPoint::Default,
                    &self.compositing,
                ))
            }
            _ => None,
        };

        Some(Rc::new(Tiling {
            content,
            resources,
            step,
            bbox: cell_box,
            to_page: crate::shading::matrix_of(self.document, dict, "Matrix").then(self.base),
            tint,
        }))
    }
}

/// Where one site of a tiling sits relative to the site its cell was drawn at, in page space.
///
/// ISO 32000-2 §8.7.3.1 puts site (i, j) at i × `/XStep` and j × `/YStep` in *pattern* space, so
/// the displacement between two sites is that difference carried through the pattern matrix —
/// and through its linear part alone, because the matrix's own translation is common to both
/// sites and cancels. The result is therefore a pure translation of page space whatever the
/// pattern matrix rotates or shears, which is what [`pdf_render::Cell::repeat`] wants.
fn displacement(tiling: &Tiling, columns: i32, rows: i32) -> Transform {
    let to_page = tiling.to_page;
    let x = tiling.step.0 * as_f32(columns);
    let y = tiling.step.1 * as_f32(rows);
    Transform::translate(
        to_page.a.mul_add(x, to_page.c * y),
        to_page.b.mul_add(x, to_page.d * y),
    )
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

/// Which tiles of a pattern the given bounds in pattern space touch, by column and by row.
///
/// Where the cell itself is matters, which is [`span`]'s subject. Table 74 makes `/BBox`
/// required, and a pattern that states none is tiled as though its cell began at the origin —
/// which is what this did for every pattern until the two-hundred-and-eighteenth session.
fn spans(tiling: &Tiling, bounds: (f32, f32, f32, f32)) -> ((i32, i32), (i32, i32)) {
    let cell = tiling
        .bbox
        .unwrap_or([0.0, 0.0, tiling.step.0, tiling.step.1]);
    (
        span(
            bounds.0,
            bounds.2,
            tiling.step.0,
            cell[0].min(cell[2]),
            cell[0].max(cell[2]),
        ),
        span(
            bounds.1,
            bounds.3,
            tiling.step.1,
            cell[1].min(cell[3]),
            cell[1].max(cell[3]),
        ),
    )
}

/// Whichever of two named budgets has less left, and how much that is.
fn tighter_of(a: (&'static str, usize), b: (&'static str, usize)) -> (&'static str, usize) {
    if b.1 < a.1 { b } else { a }
}

/// How many columns or rows an inclusive index range spans, and never fewer than one.
fn extent(first: i32, last: i32) -> usize {
    usize::try_from(last.saturating_sub(first).saturating_add(1))
        .unwrap_or(usize::MAX)
        .max(1)
}

/// The range of tile indices covering an interval, given a step and where the cell itself sits.
///
/// §8.7.3.1 places the pattern cell where its content stream draws it and replicates that at
/// multiples of `/XStep` and `/YStep` — so the offsets needed to cover `low..high` are measured
/// from the **cell's own extent**, not from the pattern space's origin. Tile `k` covers
/// `cell + k × step`, so it is wanted when `cell_low + k × step <= high` and
/// `cell_high + k × step >= low`.
///
/// **This took `cell_low` and `cell_high` from the two-hundred-and-eighteenth session and did
/// not before**, which was invisible for as long as it was because Table 74's `/BBox` is nearly
/// always at the pattern's origin: the ±1 of slack `floor` and `ceil` give covers a cell within
/// one step of it. `issue13561_reduced.pdf` states `/BBox [35.4 396.6 287.4 588]` against a
/// `/YStep` of 191.4 — two steps out — and every tile landed two rows below the page.
fn span(low: f32, high: f32, step: f32, cell_low: f32, cell_high: f32) -> (i32, i32) {
    /// Bounds the index range so a huge path or a tiny step cannot overflow.
    const LIMIT: f32 = 1e6;

    let first = ((low - cell_high) / step).floor().clamp(-LIMIT, LIMIT);
    let last = ((high - cell_low) / step).ceil().clamp(-LIMIT, LIMIT);
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

/// A shading with a constant alpha applied, sharing the original where it is opaque.
///
/// The share is the common case and the one worth keeping cheap: a pattern set once paints
/// every path filled until the colour changes again, and copying its 256-sample ramp — or a
/// mesh's triangles — per fill would be a copy per path for nothing.
pub(super) fn shading_with_alpha(shading: &Arc<Shading>, alpha: f32) -> Arc<Shading> {
    if alpha < 1.0 {
        Arc::new(shading.with_alpha(alpha))
    } else {
        Arc::clone(shading)
    }
}
