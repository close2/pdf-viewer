//! §8.7.4.3 and §11.6.7: the colour of a shading pattern: the half the `scn` fixes, the half that
//! belongs to the mark painting it, and the paint a shading becomes (ADR 2081).

use std::sync::Arc;

use pdf_render::{Color, Paint, Shading};
use pdf_syntax::Object;

use crate::content::Interpreter;
use crate::content::report::Unsupported;

use super::{ShadingDefinition, ShadingPattern};

/// The half of a shading pattern's colours that belongs to the mark painting it.
///
/// One quantity, put here by a clause of its own: **§11.7.2's compositing target**, inside
/// [`crate::colour::Conversion`]. §11.6.7 makes the pattern's definition a non-isolated group and
/// §11.7.2 says "\[n]on-isolated groups shall inherit their colour space from the nearest ancestor
/// isolated parent group" — which for a pattern painted inside a group is *that* group, not
/// whichever one the `scn` stood in.
///
/// §10.5's transfer function was the second until ADR 1266 and is not one: §11.7.5.3's NOTE says
/// its values "are used only when all colour compositing has been completed and rasterization is
/// being performed", so it rides on the mark into §11.7.5.2's channel and never reaches a colour.
///
/// The black point [`crate::colour::Conversion`] also carries is the *definition's* and never the
/// mark's — [`Interpreter::mark_colouring`] is the only constructor, and it reads it from
/// [`ShadingDefinition`].
#[derive(Debug, Clone)]
pub(super) struct MarkColouring {
    /// §11.7.2's target, with §11.6.7's black point decision folded in.
    conversion: crate::colour::Conversion,
}

impl MarkColouring {
    /// Whether two marks ask a pattern's definition for the same colours.
    fn same_as(&self, other: &Self) -> bool {
        self.conversion == other.conversion
    }
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

impl Interpreter<'_> {
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
    pub(super) fn note_unpainted_background(
        &mut self,
        label: &str,
        object: &Object,
        resolved: Option<Color>,
    ) {
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
    /// [`super::initial::PatternInitial`]. A signature that could reach a `&GraphicsState` would make that a rule
    /// somebody has to keep; this way the wrong version does not compile.
    ///
    /// The one quantity that *does* come from the mark is §11.7.2's compositing target, through
    /// [`Interpreter::conversion_under`], which reads the target this run is compositing into.
    /// §10.5's transfer function is not a second: §11.7.5.3's NOTE says its values "are used only
    /// when all colour compositing has been completed and rasterization is being performed", so
    /// it rides on the mark and is applied at the device pixel (ADR 1266).
    pub(super) fn mark_colouring(&self, definition: &ShadingDefinition) -> MarkColouring {
        MarkColouring {
            conversion: self
                .conversion_under(definition.initial.rendering())
                .under_black_generation(definition.initial.black_generation()),
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
    pub(super) fn build_shading(
        &mut self,
        definition: &ShadingDefinition,
        colouring: &MarkColouring,
    ) -> Result<(Shading, bool), crate::shading::ShadingError> {
        let built = self.shadings.build(
            self.document,
            &definition.object,
            &definition.resources,
            definition.transform,
            // §11.7.5.3's NOTE: §10.5's values "are used only when all colour compositing has
            // been completed and rasterization is being performed", so no colour built here
            // carries one and §11.7.5.2's channel maps the device pixel (ADR 1266).
            crate::shading::Colouring::new(definition.initial.smoothness, &colouring.conversion),
        )?;
        // Raised here rather than at one of the two callers because both of them paint: the
        // `scn` builds the colours and every later mark rebuilds them, and a mesh the bound cut
        // short is the same mesh at both. `note` is a set, so saying it once a mark says it
        // once.
        if let Some(limit) = built.truncated {
            self.note(Unsupported::LimitReached { limit });
        }
        if built.coarse {
            self.note(Unsupported::LimitReached {
                limit: "max_mesh_refinement",
            });
        }
        let shading = built.shading;
        if definition.paints_background {
            return Ok((shading, built.black_generation_bypassed));
        }
        Ok((
            Shading {
                background: None,
                ..shading
            },
            built.black_generation_bypassed,
        ))
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
    pub(super) fn shading_paint(&mut self, pattern: &ShadingPattern, alpha: f32) -> Paint {
        let wanted = self.mark_colouring(&pattern.definition);
        if wanted.same_as(&pattern.built) {
            self.black_generation_painted |= pattern.black_generation_bypassed;
            return Paint::Shading(shading_with_alpha(&pattern.shading, alpha));
        }
        match self.build_shading(&pattern.definition, &wanted) {
            Ok((shading, bypassed)) => {
                self.black_generation_painted |= bypassed;
                Paint::Shading(shading_with_alpha(&Arc::new(shading), alpha))
            }
            Err(error) => {
                self.black_generation_painted |= pattern.black_generation_bypassed;
                self.note(Unsupported::Shading {
                    name: format!("a shading pattern's colours could not be rebuilt: {error}"),
                });
                Paint::Shading(shading_with_alpha(&pattern.shading, alpha))
            }
        }
    }
}
