//! §8.7's patterns and shadings: the `sh` operator, shading paints and tiling cells.
//!
//! A shading pattern travels into the display list as a paint; a tiling pattern is a content
//! stream read **once** and its marks copied to every site the lattice reaches, which is why
//! most of this module is about placing, clipping and folding cells rather than about colour.

use std::rc::Rc;
use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{BlendMode, FillRule, Paint, Path, Shading, Stroke, Transform, stroked_bounds};
use pdf_syntax::{Dictionary, Name, Object};

use super::report::Unsupported;
use super::transparency::Painted;
use super::{GraphicsState, Interpreter};

mod cell;
mod colouring;
mod initial;
mod lattice;
mod reach;
mod repeat;
mod shading;
mod tiling;
mod wrapping;

use reach::Reach;

use colouring::MarkColouring;
pub(super) use initial::PatternInitial;
use lattice::{as_f32, bounds_of, spans};
use tiling::Tiling;

/// The selected tint and the parameters of the mark using it, separate from cell state.
///
/// Binding the retained tint to each mark's parameters is what ADR 2212 accepted and priced:
/// paired whole interpretations of 20 000 cell marks read median ratios of 1.017 for one cell,
/// 1.011 for many, 1.030 for alternating tints and 1.044 for changed pairs — about 60–408 ns a
/// mark. `black_generation_order --pattern-cost` runs the four populations.
#[derive(Debug)]
pub(super) struct TintBinding {
    source: Rc<tiling::TintSource>,
    rendering: crate::icc::Rendering,
    generation: Option<Arc<crate::black_generation::BlackGeneration>>,
}

impl TintBinding {
    /// The mark's conversion parameters combined with the current target.
    fn conversion(&self, interpreter: &Interpreter<'_>) -> crate::colour::Conversion {
        interpreter
            .conversion_under(self.rendering)
            .under_black_generation(self.generation.clone())
    }

    /// Whether this same effective conversion bypasses its stated pair.
    pub(super) fn bypasses_black_generation(&self, interpreter: &Interpreter<'_>) -> bool {
        interpreter
            .paint_context(self.rendering, self.generation.as_ref())
            .bypasses_black_generation(&self.source.space)
    }

    /// Resolves the source in the current nested target without replacing definition state.
    pub(super) fn paint(&self, interpreter: &mut Interpreter<'_>, fill: bool) -> pdf_render::Color {
        if matches!(self.source.space, crate::colour::ColourSpace::Rgb)
            && let Ok(values) = self.source.values.as_ref().try_into()
        {
            return interpreter.rgb_paint(values, self.rendering, self.generation.as_ref(), fill);
        }
        let conversion = self.conversion(interpreter);
        let read = super::colour::read_where_painted(
            &conversion,
            &self.source.space,
            self.source.device_cmyk.as_ref(),
        );
        conversion.paint(
            read.as_ref().unwrap_or(&self.source.space),
            &self.source.values,
        )
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
    /// The colours bypassed a stated pair; consumed only when this pattern paints.
    black_generation_bypassed: bool,
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
    /// A property of the definition for the same reason as the three `initial` carries: §8.7.2 maps
    /// it to "the default coordinate system of the pattern's parent content stream", which is
    /// §11.6.7's first named parameter and which [`Interpreter::base`] scopes.
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
/// The obvious construction for the stroking case is the outline as a path, tiled the way a fill's
/// path is; [`Interpreter::tile`] does not do that, on the reason ADR 0028 gives — no crate that
/// builds a display list expands a stroke, all three backends do it themselves, and computing an
/// outline here would be a fourth expander in the one crate whose whole point is that it has none.
/// That reason is about *one* construction. The region a stroke covers is equally the alpha of a
/// group whose single element is that stroke (§11.5.2), and a soft mask is already a command list
/// every backend rasterises with the machinery it has — so the shape travels as a `Command::Stroke`
/// and each backend expands it with its own expander, exactly once, exactly as it expands the
/// strokes it already draws. ADR 0735.
#[derive(Debug, Clone, Copy)]
pub(super) enum Tiled<'a> {
    /// §8.5.3.3's fill: the region is the path's interior under this rule.
    Fill(FillRule),
    /// §8.4.3's stroke: the region is the outline these parameters deposit around the path.
    Stroke(&'a Stroke),
}

impl Interpreter<'_> {
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
    ///
    /// Answers whether the cell's own marks composite, which a combined fill and stroke asks:
    /// §11.6.7 makes what the cell evaluates to the object's own opacity, so a part painted
    /// through a translucent cell composites with the other part however opaque the painting
    /// mark's state is (§11.6.2, §11.7.4.4).
    pub(super) fn tile(
        &mut self,
        path: &Arc<Path>,
        transform: Transform,
        region: Tiled<'_>,
        tiling: &Tiling,
        state: &GraphicsState,
    ) -> bool {
        // The pattern is anchored to the page, so the question "which cells does this path
        // touch" has to be asked in the pattern's own coordinates.
        let Some(to_pattern) = tiling.to_page.invert() else {
            self.note(Unsupported::Shading {
                name: "a tiling pattern's matrix is degenerate".to_owned(),
            });
            return false;
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
            return false;
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
                    return false;
                };
                (Some(clip), None)
            }
            Tiled::Stroke(stroke) => {
                let Some(shape) = self.stroke_shape(path, transform, stroke) else {
                    self.note(Unsupported::LimitReached {
                        limit: "max_soft_masks",
                    });
                    return false;
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
        // `Interpreter::cell_state` gives it — and the state's own blend mode, alpha constant
        // and soft mask are applied *once*, to the finished tiling, by the group pushed after
        // the loop. NOTE 2 asks for exactly that shape: "[i]n a raster-based implementation of
        // tiling, it is advisable to treat all tiles as a single transparency group. This
        // avoids artifacts due to multiple marking of pixels along the boundaries between
        // adjacent tiles."
        //
        // Were each cell to inherit them instead, an `0.5 ca` under a pattern would be applied
        // per tile rather than to the pattern, and the graphics state's soft mask would reach
        // nothing at all (ADR 0107).
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
        // the two the same picture, and ADR 0430 for what it saves: a content stream run once
        // per site would make a bomb inside a cell cost its decode four thousand times over.
        let at = pdf_render::Mark::of(&self.list);
        let offset = Transform::translate(
            tiling.step.0 * as_f32(first_column),
            tiling.step.1 * as_f32(first_row),
        );
        // §11.7.5.2's sixth condition: "[i]f the current colour is a tiling pattern, all objects
        // in the definition of its pattern cell also satisfy the foregoing conditions." The cell
        // runs from `Interpreter::cell_state` for §11.6.7's reason, so a mark inside it cannot see
        // the mark that invoked it; the four conditions are read off `state` here and carried
        // down, with `alpha` above naming which of §11.6.4.4's two constants the invoking mark
        // is under. **A stroke's shape mask is a fifth condition and it fails**: the tiles are
        // multiplied by a coverage that is below 1.0 wherever the outline is anti-aliased, so
        // the marks inside such a cell are not opaque and §11.7.5.2's sixth condition cannot be
        // met through them.
        let (inside, transfer) = self.cell_paint_parameters(state, region, shape.is_none(), alpha);
        let (box_clip, transfer, reading) = self.run_cell_scoped(
            tiling,
            state,
            offset.then(tiling.to_page),
            clip,
            inside,
            transfer,
        );
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
        // Asked of the one cell before it is copied: the copies are the same marks displaced.
        let cell_composites = self.marks_composite(mark);
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
        self.compose_tiling(mark, (alpha, shape), state, reading);
        self.record_tiling(mark, transfer);
        cell_composites
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
        // Unresolved first, for the same reason `draw_xobject` reads its entry that way: the
        // reference is the one thing that says which stream a tiling cell is.
        let Some(entry) = self.resource_entry(resources, "Pattern", name) else {
            self.note_missing_resource("Pattern", name, "is not in /Pattern");
            return None;
        };
        let source = entry.as_reference();
        let object = self.document.resolve(&entry);
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
                    .tiling(&label, &object, source, &dict, tint, state, fill)
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
            initial: self.pattern_initial.clone().augmented(self.document, &dict),
            // Table 77's `/Background` "shall be applied only when the shading is used as part
            // of a shading pattern, not when painted directly with the sh operator", so this is
            // the one place in the interpreter where the entry means anything at all.
            paints_background: fill,
        };
        let built = self.mark_colouring(&definition);
        match self.build_shading(&definition, &built) {
            Ok((shading, black_generation_bypassed)) => {
                self.note_unpainted_background(&label, &definition.object, shading.background);
                Some(PatternPaint::Shading(Rc::new(ShadingPattern {
                    black_generation_bypassed,
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

    /// The paint a non-stroking mark under `state` puts on the page.
    ///
    /// On the interpreter rather than on [`GraphicsState`] because a shading pattern's colours
    /// are the *mark's* (§11.6.7, [`Interpreter::shading_paint`]) and building them needs the
    /// document, the resource dictionary and `shading::Cache` — none of which a graphics state
    /// has. Every other paint is the state's own and comes straight back from
    /// [`GraphicsState::solid_fill`].
    ///
    /// §11.6.4.4's constant alpha is applied here, and reaching every colour a shading carries is
    /// the only way to apply it to one (`Shading::with_alpha`): `alphatrans.pdf` states
    /// `Gradient: .5` on the page and draws its gradient over three other objects (ADR 0024).
    ///
    /// Called **once per mark**: a rebuild costs a build, so asking twice for one command would
    /// pay twice.
    ///
    /// §10.5's transfer function reaches no colour built here, and that is §11.7.5.3's NOTE
    /// rather than an omission: its values "are used only when all colour compositing has been
    /// completed and rasterization is being performed", which is where §11.7.5.2's channel
    /// applies them ([`Interpreter::mark_transfer`], ADR 1266).
    pub(super) fn fill_paint(&mut self, state: &GraphicsState) -> Paint {
        let Some(PatternPaint::Shading(pattern)) = &state.fill_pattern else {
            self.report_solid_black_generation(state, true);
            let colour = self.solid_colour(state, true);
            return Paint::Solid(pdf_render::Color {
                a: colour.a * state.fill_alpha,
                ..colour
            });
        };
        let pattern = Rc::clone(pattern);
        self.shading_paint(&pattern, state.fill_alpha)
    }

    /// §11.7.5.2's function for one mark, split into the half that goes inside the colour and
    /// the half that rides on the mark.
    ///
    /// §11.7.5.2 chooses the function by the topmost object covering a point and §11.7.5.3's NOTE
    /// applies it "only when all colour compositing has been completed", so the function belongs
    /// on the mark — [`Interpreter::draw_mark`] — and never in the colour. **Every paint answers
    /// the same way**, a shading pattern's ramp included: applying the function to a *simplified*
    /// ramp's stops would draw the chord between the transferred ends rather than the curve
    /// (ADR 0479), but the channel does not apply it to the ramp at all — it applies it to the
    /// finished device pixel, where the clause puts it, so the ramp is sampled raw and the curve
    /// is evaluated once per pixel instead of once per stop. ADR 1266.
    pub(super) fn mark_transfer(
        &mut self,
        state: &GraphicsState,
        painted: Painted,
    ) -> Option<Arc<crate::content::Transfer>> {
        self.transfer_for_mark(state, painted).cloned()
    }

    /// Reads the invoking mark's opacity and transfer before definition-state evaluation.
    fn cell_paint_parameters(
        &mut self,
        state: &GraphicsState,
        region: Tiled<'_>,
        no_shape_mask: bool,
        alpha: f32,
    ) -> (bool, Option<Arc<crate::content::Transfer>>) {
        let inside = self.opaque_ancestry
            && no_shape_mask
            && alpha >= 1.0
            && state.blend == BlendMode::Normal
            && state.soft_mask.is_none();
        let transfer = self.mark_transfer(
            state,
            Painted::of(state, matches!(region, Tiled::Stroke(_))),
        );
        (inside, transfer)
    }

    /// As [`Interpreter::fill_paint`], for a stroking mark and §11.6.4.4's stroking constant.
    pub(super) fn stroke_paint(&mut self, state: &GraphicsState) -> Paint {
        let Some(PatternPaint::Shading(pattern)) = &state.stroke_pattern else {
            self.report_solid_black_generation(state, false);
            let colour = self.solid_colour(state, false);
            return Paint::Solid(pdf_render::Color {
                a: colour.a * state.stroke_alpha,
                ..colour
            });
        };
        let pattern = Rc::clone(pattern);
        self.shading_paint(&pattern, state.stroke_alpha)
    }
}
