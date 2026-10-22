//! §8.7.3: a tiling pattern read from its stream: the cell, its steps and bounding box, and the
//! resources its content runs under (ADR 2081).

use std::rc::Rc;

use pdf_render::Transform;
use pdf_syntax::{Dictionary, Object, ObjectId};

use crate::colour::ColourSpace;

use crate::content::report::Unsupported;
use crate::content::run::narrow;
use crate::content::{GraphicsState, Interpreter};

use super::lattice::{cell_extent, non_zero};

/// A tiling pattern: a cell of content, and how to repeat it.
#[derive(Debug)]
pub(in crate::content) struct Tiling {
    /// The cell's content stream, read **once** for the whole tiling (ADR 0430).
    ///
    /// Routed like the other three of §7.8.2's nested streams since the cell stopped being
    /// re-interpreted per site: a decode the memo declines is windowed, and a window is read
    /// once here rather than four thousand times. ADR 0427's exception ends with the loop that
    /// caused it.
    pub(super) content: crate::content::reader::NestedContent,
    /// The resources its operators name.
    pub(super) resources: Dictionary,
    /// The pattern stream's own object, where the `/Pattern` entry named it by reference.
    ///
    /// Carried for the ledger alone: the cell is run where the pattern is painted, not where
    /// it is selected, and the run is what has to say which stream it is.
    pub(super) source: Option<ObjectId>,
    /// Spacing between cells, in pattern space. Never zero.
    pub(super) step: (f32, f32),
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
    pub(super) bbox: Option<[f32; 4]>,
    /// Maps pattern space to the page's default space.
    pub(super) to_page: Transform,
    /// The colour an uncoloured pattern is poured through, if it is uncoloured.
    ///
    /// `/PaintType 2` cells carry no colour of their own; the colour comes from `scn`.
    pub(super) tint: Option<Rc<TintSource>>,
}

/// Immutable source selected alongside an uncoloured tiling pattern.
#[derive(Debug)]
pub(in crate::content) struct TintSource {
    /// The resolved selection space, independent of later resource dictionaries.
    pub(in crate::content) space: ColourSpace,
    /// Only components the selected space can consume are retained.
    pub(in crate::content) values: Box<[f32]>,
}

impl Interpreter<'_> {
    /// Reads a tiling pattern's cell and how it repeats.
    #[expect(
        clippy::too_many_arguments,
        reason = "the pattern's entry arrives in three shapes — the object, its reference and \
                  its dictionary — and each is read by a different part of the cell's construction"
    )]
    pub(super) fn tiling(
        &mut self,
        name: &str,
        object: &Object,
        source: Option<ObjectId>,
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
        // A cell whose stream cannot be decoded at all is **reported**, never dropped with the
        // page coming back complete and the pattern silently unpainted. A form says
        // `undecodable form /Fx` in the same circumstance, and this is the same sentence for
        // the other of §7.8.2's five.
        let Some(content) =
            self.content_stream(stream, &format!("a tiling pattern /{name} (§8.7.3.1)"))
        else {
            self.note(Unsupported::Operator {
                operator: format!("undecodable tiling pattern /{name}"),
            });
            return None;
        };

        // Table 74's `/TilingType`, "[a] code that controls adjustments to the spacing of tiles
        // relative to the device pixel grid". `pdf_render::lattice` carries the three codes and
        // the arithmetic each one asks for; [`Interpreter::lattice`] below is where the device
        // pixel grid comes from, and ADR 1080 is why the answer lives in those two places.
        //
        // It is read and not **reported**, which is a decision and not an omission (ADR 1031):
        // an `Unsupported` here would name a departure on almost every patterned page in the
        // world, and take each of them off the oracle's judged list to say it.
        let tiling_type = self
            .document
            .get_key(dict, "TilingType")
            .as_integer()
            .and_then(pdf_render::TilingType::from_code);
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

        // Table 74 makes a tiling pattern's `/Resources` "( Required )", and §7.8.3 requires it
        // of a pattern in the same breath as of a form: "This shall apply to content streams that
        // define form XObjects, patterns, and annotation appearances." What no sentence does is
        // give a pattern the *fallback* it gives the other two — the fourth bullet Errata
        // Collection 3 retired named form XObjects and Type 3 fonts, its NOTE 3 adds annotation
        // appearance streams, and neither names a pattern. So a pattern stating none has a
        // current resource dictionary that defines nothing: every name its cell uses is reported
        // by `draw_xobject`, `font` and their neighbours as a resource the file never defined,
        // and what the cell draws without a name is still drawn (trap 5). Reading the invoking
        // stream's dictionary instead would be a fallback the clause does not state, and it is
        // the reading `pdf-archive`'s survey had until ADR 1059 (ADR 1055 section 5).
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
                    ColourSpace::Pattern { base: Some(base) } => (**base).clone(),
                    ColourSpace::Pattern { base: None } => match tint.len() {
                        3 => ColourSpace::Rgb,
                        4 => ColourSpace::Cmyk,
                        _ => ColourSpace::Gray,
                    },
                    other => other.clone(),
                };
                let count = space.components();
                Some(Rc::new(TintSource {
                    space,
                    values: tint.iter().take(count).copied().collect(),
                }))
            }
            _ => None,
        };

        // §8.7.2: the pattern matrix "maps the pattern's internal coordinate system to the
        // default coordinate system of the pattern's parent content stream", which `base` holds.
        let stated = crate::shading::matrix_of(self.document, dict, "Matrix").then(self.base);

        Some(Rc::new(Tiling {
            content,
            resources,
            source,
            step,
            bbox: cell_box,
            to_page: self.lattice(tiling_type, stated, step, cell_box),
            tint,
        }))
    }
}
