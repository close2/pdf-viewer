//! §11.7.5.2: which of a mark's transfer functions apply to the object being painted, and which of the
//! clause's conditions it fails. Part of `Interpreter`'s transparency block (ADR 2080).

use std::sync::Arc;

use pdf_render::BlendMode;

use crate::content::{GraphicsState, Interpreter, Transfer};

/// Which elementary graphics object is being painted, for §11.7.5.2's conditions.
///
/// The clause's six conditions are not all about the graphics state: the fourth is about the
/// image dictionary and the first depends on which of Table 51's two alpha constants the painting
/// operation reads. Both are the caller's to know, so they arrive as this rather than as two
/// booleans nobody could read at a call site. See [`Interpreter::transfer_for_mark`].
#[derive(Debug, Clone, Copy)]
pub(in crate::content) enum Painted {
    /// A non-stroking mark whose paint carries a colour: a fill, a glyph fill, a stencil mask.
    Filled,
    /// A stroking mark, which reads the *stroking* alpha constant instead.
    Stroked,
    /// An image `XObject`, with whether its own dictionary states the clause's `/SMask`.
    Image {
        /// The fourth condition, which only the caller can answer.
        soft_mask: bool,
    },
    /// A shading, painted by `sh` or as a pattern.
    Shading {
        /// Whether it was the stroke colour, for the first condition.
        stroking: bool,
    },
}

impl Painted {
    /// Which kind a fill or a stroke is, given the state it is painted under.
    ///
    /// §8.7.2 makes a pattern a colour — "[a]ll patterns shall be treated as colours" — and the
    /// three answers differ only in which of §11.7.5.2's six conditions they can fail. A tiling
    /// pattern is not a paint at all, so it never reaches a call site of this.
    ///
    /// **This used to carry a `stale` flag** saying that a shading pattern's colours had been
    /// built under a transfer function the mark no longer states. There is no such state now:
    /// `Interpreter::shading_paint` rebuilds them, so §10.5's function at a mark is the mark's
    /// own whichever kind of paint carries it.
    pub(in crate::content) fn of(state: &GraphicsState, stroking: bool) -> Self {
        let pattern = if stroking {
            state.stroke_pattern.as_ref()
        } else {
            state.fill_pattern.as_ref()
        };
        match pattern {
            Some(crate::content::pattern::PatternPaint::Shading(_)) => Self::Shading { stroking },
            _ if stroking => Self::Stroked,
            _ => Self::Filled,
        }
    }
}

impl Interpreter<'_> {
    /// Which transfer function §11.7.5.2 puts on the mark about to be painted, and what it costs.
    ///
    /// Called **once per elementary graphics object** that marks the page, with the state it is
    /// painted under, and its answer is the only route by which §10.5's function reaches the
    /// page. It reaches no *colour*: every caller passes what comes back to
    /// [`Interpreter::draw_mark`], which puts it on §11.7.5.2's channel, and a backend maps the
    /// finished pixel where §11.7.5.3's NOTE says the values are used (ADR 1148, ADR 1266). That
    /// is the shape trap 2 asks for one layer up: a device decision either backend could make
    /// alone is a decision neither has made, and this one is made here.
    ///
    /// # What §11.7.5.2 chooses between
    ///
    /// > The halftone and transfer function to be used at any given point on the page shall be
    /// > those in effect at the time of painting the last (topmost) elementary graphics object
    /// > enclosing that point, but only if the object is fully opaque.
    ///
    /// > For portions of the page whose topmost object is not fully opaque or that are never
    /// > painted at all, the default halftone and transfer function for the page shall be used
    ///
    /// The parameter belongs to a *point*, decided by one object — the topmost — and by the page's
    /// default where that object is not fully opaque. Table 52 says what the default is ("a PDF
    /// reader shall initialise this to a suitable device dependent value"), and for this device it
    /// is the identity: the table's own subject is a function that "adjusts device gray or colour
    /// component levels to compensate for nonlinear response in a particular output device", and
    /// nothing between a `Command::Fill`'s colour and the raster is such a response. That is a
    /// documented choice rather than a silence, and `doc/todo/13` records it as one.
    ///
    /// # Why a mark that is not fully opaque is handed no function at all
    ///
    /// Take an object *O* that fails any of [`Interpreter::not_fully_opaque`]'s six conditions,
    /// and any point *p* it encloses. Either *O* is the topmost object at *p*, and the first
    /// sentence withholds its function in favour of the second's default; or something else is
    /// topmost at *p*, and that object's function or the default is chosen instead. Neither branch
    /// can choose *O*'s. **So a mark that is not fully opaque has its own function used at no
    /// point on the page**, and the colour it contributes goes to the device unmapped — which is
    /// exact rather than an approximation, and needs nothing per-point to implement.
    ///
    /// A mark inside a *soft mask's* group is the same deduction with a different first step: the
    /// clause is addressed throughout to "any given point on the page", and §11.5.3 makes such a
    /// group's result "the luminosity of the resulting colour" rather than ink at a point. So no
    /// mark inside one is ever "the topmost elementary object in the entire page stack" anywhere,
    /// and none of them is handed a function either. §11.5.3 says as much from the other side, of
    /// the conversion itself: it is done "with no compensation for gamma or other colour
    /// calibration", which is what Table 52 calls a transfer function.
    ///
    /// # A mark inside a tiling pattern's cell
    ///
    /// §11.7.5.2's sixth condition is about a *point* covered by a tiling, and it names the
    /// cell's objects rather than making each of them the topmost object there:
    ///
    /// > If the current colour is a tiling pattern, all objects in the definition of its pattern
    /// > cell also satisfy the foregoing conditions.
    ///
    /// So the elementary object the clause chooses a function for at such a point is the object
    /// *painted with the pattern*, and the cell's marks decide only whether that object is fully
    /// opaque. They are accumulated into [`Interpreter::tiling_cell_opaque`] here, where the
    /// other five conditions are already read, and handed no function of their own —
    /// [`Interpreter::record_tiling`] puts the painting mark's on the finished tiling. A `/TR` a
    /// cell's own `/ExtGState` states therefore decides nothing, which §11.7.5.3's NOTE is what
    /// settles: the value is used "only when all colour compositing has been completed", so it is
    /// not one of the parameters §11.6.7's bullets evaluate a pattern definition under. ADR 1266.
    ///
    /// The population that can reach any of this is measured rather than assumed:
    /// `examples/transfer_function_census` counts how many documents state a Table 57 `/TR` or
    /// `/TR2` and how many state a real one; run it rather than reading a figure here. The
    /// fixtures in `tests/transfer_functions.rs` stand in for the missing corpus witness (trap 8).
    pub(in crate::content) fn transfer_for_mark<'state>(
        &mut self,
        state: &'state GraphicsState,
        painted: Painted,
    ) -> Option<&'state Arc<Transfer>> {
        if self.soft_mask_depth > 0 {
            return None;
        }
        if self.tiling_cell {
            self.tiling_cell_opaque &= self.not_fully_opaque(state, painted).is_none();
            return None;
        }
        if self.not_fully_opaque(state, painted).is_some() {
            return None;
        }
        state.transfer.shared()
    }

    /// Which of §11.7.5.2's six conditions the object being painted fails, if any.
    ///
    /// > An object is fully opaque if all of the following conditions hold at the time the object
    /// > is painted:
    /// >
    /// > - The current alpha constant in the graphics state (stroking or nonstroking, depending
    /// >   on the painting operation) is 1.0.
    /// > - The current blend mode in the graphics state is Normal .
    /// > - The current soft mask in the graphics state is None .
    /// > - If the object is an image XObject and there is not an SMask entry in its image
    /// >   dictionary.
    /// > - The foregoing four conditions were also true at the time the Do operator was invoked
    /// >   for the group containing the object, as well as for any direct ancestor groups.
    /// > - If the current colour is a tiling pattern, all objects in the definition of its
    /// >   pattern cell also satisfy the foregoing conditions.
    ///
    /// The first three are read off the state, the fourth is the caller's since only it knows the
    /// image dictionary, and the last two are [`Interpreter::opaque_ancestry`] — carried down
    /// rather than looked up, because a mark inside a group or a cell cannot see the `Do` or the
    /// fill that invoked it.
    ///
    /// The fourth condition names `/SMask` and nothing else. `/Mask` and `/SMaskInData` also stop
    /// an image obscuring its backdrop, and the clause does not list them; they are left out
    /// rather than folded in, because a condition the standard states is not ours to widen and
    /// [`crate::image::overrides_graphics_state_mask`] is where the wider question already lives.
    ///
    /// Returns the *first* condition that fails, worded for the report. Which one it is is
    /// diagnostic rather than normative — the clause needs all six — but a report that named
    /// none of them could not be acted on. `None` means fully opaque, which is the answer
    /// [`Interpreter::transfer_for_mark`] needs and the one the clause's first sentence turns on.
    fn not_fully_opaque(&self, state: &GraphicsState, painted: Painted) -> Option<&'static str> {
        let stroking = matches!(
            painted,
            Painted::Stroked | Painted::Shading { stroking: true, .. }
        );
        let alpha = if stroking {
            state.stroke_alpha
        } else {
            state.fill_alpha
        };
        if alpha < 1.0 {
            return Some(if stroking {
                "the stroking alpha constant is below 1.0"
            } else {
                "the non-stroking alpha constant is below 1.0"
            });
        }
        if state.blend != BlendMode::Normal {
            return Some("the blend mode in force is not Normal");
        }
        if state.soft_mask.is_some() {
            return Some("a soft mask is in force");
        }
        if matches!(painted, Painted::Image { soft_mask: true }) {
            return Some("the image XObject's own dictionary states an /SMask");
        }
        if !self.opaque_ancestry {
            return Some(
                "an enclosing group's Do, or the fill that painted an enclosing pattern cell, \
                 was itself not fully opaque",
            );
        }
        None
    }
}
