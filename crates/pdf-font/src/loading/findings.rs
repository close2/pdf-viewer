//! What loading found out about a font, for a caller deciding how far to trust it: whether it
//! carries only the document's metrics, what had to be repaired, which bound discarded a
//! mapping, and the descriptor's flags.

use super::{CodeMapping, LoadedFont};
use crate::cmap::{CMap, Code};
use crate::tounicode;

impl LoadedFont {
    /// Whether this font carries the document's metrics and no glyphs at all.
    ///
    /// True only for a font [`Self::metrics_only`] built. A caller that shows text through one
    /// advances the text matrix and draws nothing; see that constructor for why the two are
    /// separable and ADR 1094 for why they are separated.
    #[must_use]
    pub fn is_metrics_only(&self) -> bool {
        matches!(self.mapping, CodeMapping::MetricsOnly { .. })
    }

    /// Table 120's `/Ascent` and `/Descent`, in ems, for a caller measuring a line's height.
    ///
    /// See [`crate::metrics::vertical_extent`], including what a font that states neither gets
    /// and why.
    #[must_use]
    pub fn extent(&self) -> (f32, f32) {
        self.extent
    }

    /// What was done to read the program at all, where its own Font DICTs could not be.
    ///
    /// A CID-keyed CFF some of whose Font DICTs could not be read draws the glyphs under them
    /// against an empty Private DICT ([`cff::FontDictRepair`]), and the sentence says so and how
    /// many of those glyphs call a local subroutine that DICT cannot hold. It is a fact about the
    /// program; whether the page shows one of those glyphs is [`Self::glyph_lost_to_repair`]'s
    /// question, asked per code, and a page that shows none has lost nothing and reports nothing.
    #[must_use]
    pub fn repair_shortfall(&self) -> Option<String> {
        let repair = self.font_dicts.as_ref()?;
        Some(format!(
            "{} of the {} Font DICTs its CID-keyed CFF selects cannot be read (FDArray at offset \
             {}), so the glyphs under them are drawn against an empty Private DICT, and the {} \
             of its {} glyphs that call a local subroutine that DICT cannot hold are not drawn",
            repair.replaced,
            repair.font_dicts,
            repair.fd_array_at,
            repair.lost.len(),
            repair.glyphs
        ))
    }

    /// Whether `code` reaches a glyph the repaired program cannot draw — one that calls a local
    /// subroutine the replacement Private DICT does not hold. `false` for a program that needed
    /// no repair.
    #[must_use]
    pub fn glyph_lost_to_repair(&self, code: Code) -> bool {
        self.font_dicts.as_ref().is_some_and(|repair| {
            self.glyph_for(code)
                .is_some_and(|glyph| repair.lost.contains(&glyph))
        })
    }

    /// Which of `pdf-font`'s `CMap` bounds discarded a mapping this font's `CMap` stated.
    ///
    /// `None` for every simple font, which has no `CMap`, and for every composite font whose
    /// `CMap` was read whole. `Some` names the bound in [`crate::cmap::CUT_BY_RANGES`]'s
    /// vocabulary and means §9.7.6.2's lookup — "[t]he code extracted from the string shall be
    /// looked up in the character code mappings for codes of that length" — will fail for codes
    /// the producer mapped, so §9.7.6.3's CID 0 is drawn in their place. It is a fact about the
    /// font rather than about a code, which is why it is asked once where the font is loaded.
    #[must_use]
    pub fn cmap_truncated(&self) -> Option<&'static str> {
        self.cmap().and_then(CMap::truncated)
    }

    /// Which of `pdf-font`'s `/ToUnicode` bounds discarded a mapping one of this font's
    /// Unicode tables stated.
    ///
    /// `None` where every table was read whole, which is every font that states none. `Some`
    /// names the bound in [`crate::tounicode::CUT_BY_SINGLES`]'s vocabulary.
    ///
    /// **Two tables are asked, because a font can hold two** and they answer different
    /// questions: the producer's own `/ToUnicode` (§9.10.2's first method) and the character
    /// collection's `registry-ordering-UCS2` table (its third). For a composite font with no
    /// usable program the same two are what [`CodeMapping::Substituted`] selects glyphs through,
    /// which is why this is not only about extracted text: §9.7.4.2 says that with the program
    /// absent "CIDs shall not participate in glyph selection", so the substitute's glyph is
    /// reached *through* the character, and a mapping the bound discarded is a glyph the page
    /// does not draw. The substituted variant holds a copy of neither (ADR 1002).
    #[must_use]
    pub fn to_unicode_truncated(&self) -> Option<&'static str> {
        self.to_unicode.truncated().or_else(|| {
            self.collection
                .as_ref()
                .and_then(tounicode::ToUnicode::truncated)
        })
    }

    /// Whether the font's descriptor sets both Table 121's Symbolic and Nonsymbolic flags, which
    /// §9.8.2 says "shall not both be set"; see the field of the same name.
    #[must_use]
    pub fn states_both_symbolic_flags(&self) -> bool {
        self.symbolic_flags_conflict
    }

    /// The horizontal scale this font's outlines are drawn at, one being the face as it is.
    ///
    /// A substituted face's glyphs are as wide as *its* designer drew them, inside advances the
    /// document states for the face it meant, so a condensed font substituted by a normal one
    /// collides where the file says there is a gap. This is the number that closes that, derived
    /// from the file's own `/Widths` against the chosen face's own advances — see
    /// [`crate::metrics::substitute_stretch`], which holds the clauses and the argument.
    ///
    /// Always one for a font whose program the document embedded: those outlines are the
    /// producer's own and nothing here may reshape them.
    #[must_use]
    pub fn stretch(&self) -> f32 {
        self.stretch
    }
}
