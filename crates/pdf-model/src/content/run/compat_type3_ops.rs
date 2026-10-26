//! Table 50's compatibility operators, §7.8.2, and a Type 3 glyph description's `d0` and `d1`.

use super::machine::Machine;

use crate::content::reader::Operator;

use crate::content::Interpreter;

impl Interpreter<'_> {
    /// The compatibility operators of Table 50 (ISO 32000-2 §7.8.2): `BX` and `EX`.
    pub(super) fn run_compatibility_op(operator: Operator, machine: &mut Machine) {
        let Machine { compatibility, .. } = machine;
        match operator {
            Operator::BeginCompatibility => *compatibility = compatibility.saturating_add(1),
            Operator::EndCompatibility => *compatibility = compatibility.saturating_sub(1),
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }

    /// The Type 3 glyph description operators of Table 50 (ISO 32000-2 §9.6.4, Table 111): `d0` and
    /// `d1`.
    pub(super) fn run_type3_op(&mut self, operator: Operator) {
        match operator {
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
            Operator::GlyphWidth | Operator::GlyphWidthAndBox
                if operator == Operator::GlyphWidthAndBox && self.glyph_depth > 0 =>
            {
                // `d1` forbids the *description* from setting a colour, and nothing
                // more. What the description paints with is the graphics state
                // §9.6.4 says it inherited — both colour parameters, each used by
                // the operation that selects it, so a description that strokes
                // strokes in the stroking colour.
                //
                // **The two are not one**, whatever Table 111's singular "[i]ts
                // colour shall be determined by the graphics state in effect each time
                // this glyph is painted by a text-showing operator" suggests. The
                // clause refutes that reading three times over. §9.6.4 NOTE 2 is
                // plural — "it is unnecessary and undesirable to initialise the
                // current colour parameters because the text-showing operators are
                // designed to paint glyphs with the current colours" — the sentence
                // above it lists what a stroking description must set for itself
                // ("the line width, line join, line cap, and dash pattern") and puts
                // no colour in that list, and §9.6.4's own EXAMPLE sets a distinct
                // `RG` before each `Tj` for a `d1` `square` glyph whose body is
                // `72 w 0 0 750 750 re B`, which the collapsed reading makes dead
                // syntax. `Type3Test.pdf` in `pdf-differences` is the corpus witness
                // and ADR 0393 has the argument; the image-mask sentence that seems to
                // say otherwise is §8.9.6.2's rule about a *stencil*, which
                // paints with the non-stroking colour because that is what an image
                // mask does, not because a `d1` glyph has one colour.
                self.uncoloured = true;
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }
}
