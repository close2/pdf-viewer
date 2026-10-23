//! What one glyph is to have done to it, decided once per show string: Table 104's three
//! operations and §11.7.4's two knockout questions.

use pdf_render::BlendMode;

use crate::content::GraphicsState;
use crate::content::overprint::{FirstBullet, implicit_group_owed};
use crate::content::pattern::PatternPaint;

/// What one glyph is to have done to it, decided once per show string rather than per glyph.
///
/// §9.3.6's Table 104 is three independent operations — fill, stroke, add to the clipping path
/// — rather than eight cases, and the two knockout questions are answers about the *paint*
/// rather than about the glyph, so all five are constant across a `Tj`.
#[derive(Debug, Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent yes-or-no answers about one glyph, three of them Table 104's \
              own decomposition of the rendering mode; a state machine would have to \
              enumerate the product of five bits that the clause deliberately keeps separate"
)]
pub(super) struct GlyphPainting {
    /// Modes 0, 2, 4 and 6.
    pub(super) fills: bool,
    /// Modes 1, 2, 5 and 6.
    pub(super) strokes: bool,
    /// Modes 4 to 7.
    pub(super) clipping: bool,
    /// Whether §9.3.8's text knockout could change a pixel of this object.
    pub(super) knockout_can_show: bool,
    /// Whether it could by a tiling cell's own marks alone, which only painting the glyph
    /// answers, as [`Self::tiled_pair`] is for §11.7.4.4's.
    pub(super) knockout_by_cells: bool,
    /// Whether §11.7.4.4's implicit group could change a pixel of this glyph.
    pub(super) combining: bool,
    /// Whether it could by a tiling cell's own marks alone, which only painting the glyph
    /// answers ([`Interpreter::tile`](crate::content::Interpreter::tile)).
    pub(super) tiled_pair: bool,
    /// Which of §11.7.4's implicit groups this glyph's parts are wrapped in once painted.
    pub(super) implicit: Implicit,
    /// The blend mode the fill is painted under, §11.7.4.3's special one included.
    pub(super) fill_blend: BlendMode,
    /// The blend mode the stroke is painted under.
    pub(super) stroke_blend: BlendMode,
}

/// Which of ISO 32000-2 §11.7.4's two implicit non-isolated, non-knockout groups a glyph's
/// parts are wrapped in once they are painted.
///
/// The two differ in where §11.6.4.4's alpha constants sit, which is why they are two:
/// §11.7.4.3's last paragraph moves the blend mode alone, and §11.7.4.4's first bullet paints
/// the parts "with an alpha value of 1.0" and composites the group at the stated one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Implicit {
    /// Neither clause asks for one.
    None,
    /// §11.7.4.3's last paragraph, around an object §11.7.4.4 does not address.
    Special,
    /// §11.7.4.4's first bullet, around a glyph filled and stroked by mode 2 or 6.
    FirstBullet,
}

impl GlyphPainting {
    /// Reads Table 104's mode and the two clauses that ask about the paint behind it.
    ///
    /// A hidden optional-content layer suppresses the two operations that mark the page and
    /// *not* the clip: §8.11.3.1 lists clipping among the "graphics state operations" that
    /// "shall still be applied", and requires that "graphics state parameters that persist
    /// past the end of a marked-content section shall be the same whether the optional content
    /// is visible or not". The clip a text object leaves behind is one of those, since it
    /// outlives the `ET` that built it.
    pub(super) fn read(mode: i64, hidden: bool, state: &GraphicsState, parts: Parts) -> Self {
        let fills = matches!(mode, 0 | 2 | 4 | 6) && !hidden;
        let strokes = matches!(mode, 1 | 2 | 5 | 6) && !hidden;
        // §11.7.4.4 applies to "the painting of glyphs with text rendering mode 2 or 6",
        // which is `fills && strokes`, and its NOTE 1 says the rule "is independent of the
        // text knockout parameter in the graphics state" — so this is a different condition
        // from `knockout_can_show` below, not a special case of it. The other two halves are
        // §11.6.2's, for the same reason they are there: the paint has to composite at all,
        // and both parts have to mark the page.
        // A part that keeps a component of the backdrop composites with what is under it
        // however opaque it is, which is one more way for the portions of an object to be
        // composited with one another; and §11.7.4.4's *first* bullet, where it applies, is a
        // group of its own rather than §11.4.6's, so a pair under it is not recorded here.
        // `Interpreter::combined_overprint` decides that.
        let pair = fills
            && strokes
            && parts.first_bullet == FirstBullet::No
            && state.fill_marks()
            && state.stroke_marks();
        let combining = pair && (state.paint_composites() || parts.overprints());
        // §11.6.7 makes what a tiling cell evaluates to the part's own opacity, so a part
        // painted through one may composite where the state does not (ADR 1306).
        let tiled_pair = pair
            && !combining
            && [&state.fill_pattern, &state.stroke_pattern]
                .into_iter()
                .any(|pattern| matches!(pattern, Some(PatternPaint::Tiling(_))));
        Self {
            fill_blend: parts.fill,
            stroke_blend: parts.stroke,
            fills,
            strokes,
            clipping: matches!(mode, 4..=7),
            // §9.3.8: with `Tk` true — its initial value — the whole text object behaves as a
            // non-isolated knockout group, so "later glyphs shall overwrite ('knock out')
            // earlier ones in the area of overlap". We composite each glyph against what is
            // already on the page, which is exactly the `Tk` false behaviour. Two conditions
            // have to hold before the models can differ, and both are checked rather than
            // assumed: the paint has to composite at all — an opaque glyph under the Normal
            // blend mode overwrites what it covers either way — and two glyphs of the object
            // have to overlap, which most text never does and which only `ET` can know.
            knockout_can_show: (fills || strokes)
                && state.text.knockout
                && state.paint_composites(),
            // §11.6.7 makes what a cell evaluates to the glyph's own opacity (ADR 1306), so a
            // glyph whose paint is a tiling pattern may composite where the state does not.
            knockout_by_cells: (fills || strokes)
                && state.text.knockout
                && !state.paint_composites()
                && [
                    fills.then_some(&state.fill_pattern),
                    strokes.then_some(&state.stroke_pattern),
                ]
                .into_iter()
                .flatten()
                .any(|pattern| matches!(pattern, Some(PatternPaint::Tiling(_)))),
            combining,
            tiled_pair,
            // §11.7.4.3's last paragraph asks for a group around "the object being painted"
            // wherever the special mode is invoked under a mode other than Normal, and
            // §11.7.4.4's first bullet asks for one around a pair. A pair the second bullet
            // takes is neither: that bullet states the construction for such an object
            // itself, down to where the prevailing blend mode goes, so there is no third
            // group around it (ADR 1170).
            //
            // A glyph wrapped this way cannot then be an element of §9.3.8's own group for
            // the text object, because a non-isolated group may not be one (§11.4.6 NOTE 6,
            // `implicit_knockout_group`). Where both are owed the glyph's group is built and
            // the text object's is named instead — §11.7.4.4's NOTE 1 makes this rule
            // independent of the text knockout parameter, and §9.3.8's needs two glyphs to
            // overlap before it can change a pixel at all.
            implicit: match parts.first_bullet {
                FirstBullet::Group => Implicit::FirstBullet,
                FirstBullet::No | FirstBullet::AsPainted
                    if !combining
                        && implicit_group_owed(
                            state,
                            [
                                if fills { parts.fill } else { BlendMode::Normal },
                                if strokes {
                                    parts.stroke
                                } else {
                                    BlendMode::Normal
                                },
                            ],
                        ) =>
                {
                    Implicit::Special
                }
                FirstBullet::No | FirstBullet::AsPainted => Implicit::None,
            },
        }
    }
}

/// What §11.7.4's reading of the graphics state answered for one glyph's two parts.
///
/// Three values that always travel together, for the reason `ImagePlacement` is a struct: a
/// call site that would otherwise be a row of unlabelled booleans.
#[derive(Debug, Clone, Copy)]
pub(super) struct Parts {
    /// The blend mode the fill paints under (`Interpreter::overprint_blend`).
    pub(super) fill: BlendMode,
    /// The blend mode the stroke paints under.
    pub(super) stroke: BlendMode,
    /// Which of §11.7.4.4's first bullet's three shapes the pair takes.
    pub(super) first_bullet: FirstBullet,
}

impl Parts {
    /// Whether either part carries §11.7.4.3's special overprinting blend mode.
    fn overprints(self) -> bool {
        matches!(self.fill, BlendMode::Overprint(_))
            || matches!(self.stroke, BlendMode::Overprint(_))
    }
}
