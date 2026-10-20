//! Widths, vertical displacements and pair kerning: what a code advances the pen by, in the
//! document's terms (§9.7.4) and in the program's.

use super::{CodeMapping, LoadedFont, NOTDEF_GLYPH};
use crate::cmap::Code;
use crate::metrics;
use crate::program::Program;
use std::sync::Arc;

impl LoadedFont {
    /// Returns a code's advance width in text-space units, where one em is 1.0.
    ///
    /// A simple font's `/Widths` is indexed by character code; a composite font's `/W` is
    /// indexed by CID (§9.7.4.3), so the code goes through the `CMap` first. A code the `CMap`
    /// does not define takes CID 0's width, because CID 0's glyph is what §9.7.6.3 says is
    /// drawn.
    #[must_use]
    pub fn advance(&self, code: Code) -> f32 {
        self.widths
            .get(self.selector(code))
            .unwrap_or(self.default_width)
            / 1000.0
    }

    /// The advance the *embedded font program* states for a code, in the units
    /// [`Self::advance`] answers in: one em is 1.0.
    ///
    /// The two are the two halves of ISO 32000-2 §9.6.2.1's Table 109 —
    ///
    /// > These widths shall be consistent with the actual widths given in the font program.
    ///
    /// — and [`Self::advance`] is the *document's* half: it resolves `/Widths` or `/W`, because
    /// that is what a page is laid out by and what a renderer must obey. This is the other
    /// half, read out of the program's `hmtx` record or its charstring's leading width operand,
    /// so that a caller comparing the two is comparing two statements rather than one with
    /// itself.
    ///
    /// It exists for the callers that have to make exactly that comparison: ISO 19005-2 section
    /// 6.2.11.5 and ISO 19005-4 section 6.2.10.5 require the two to agree to within a thousandth of
    /// a text-space unit, and `pdf-model/tests/composite_fonts.rs` asks the same question of the
    /// corpus by reading `hmtx` by hand. A conformance check reading it a second time would have
    /// been a second font reader in a crate whose whole design says it has none.
    ///
    /// # `None` is an answer about this reader, never about the file
    ///
    /// Every refusal below exists so that a caller reporting a disagreement is reporting the
    /// producer's, and each one is a way this could otherwise accuse a sound file:
    ///
    /// - **A substituted font.** The advances would be the stand-in face's, which is a fact
    ///   about this machine ([`Self::is_substituted`]).
    /// - **A repaired CID-keyed CFF.** [`Self::repair_shortfall`] says what was done: the Font
    ///   DICTs that could not be read were replaced by empty ones, and a Type 2 charstring's
    ///   advance is decided by its Private DICT's `defaultWidthX` and `nominalWidthX` — so the
    ///   number that came back would be this crate's rather than the program's.
    /// - **A bare Type 1 program**, whose `hsbw` width this crate does not read.
    /// - **A code that reaches no glyph, or reaches `.notdef`.** §9.7.6.3's CID-0 fallback and
    ///   §9.6.5.2's `.notdef` substitution are what happens when the font is *drawn*; here they
    ///   are not applied, and a code the tables send to glyph 0 outright is declined for the
    ///   same reason [`crate::metrics`]'s substitute scale declines it — that glyph is what a
    ///   character the program does not have resolves to, so its advance is a statement about
    ///   absence rather than about this code.
    ///
    /// A Type 3 font never reaches this method at all — [`Self::load`] refuses one with
    /// [`FontError::Type3`], its glyphs being content streams rather than a program.
    #[must_use]
    pub fn program_advance(&self, code: Code) -> Option<f32> {
        let glyph = self.program_glyph(code)?;
        let advance = metrics::program_advance(self.program, &self.data, glyph)?;
        self.in_ems(advance)
    }

    /// The vertical displacement the *embedded font program* states for a code, in the units
    /// [`Self::vertical_metrics`] answers in and with its sign convention: downward is negative.
    ///
    /// The program-side counterpart of the first number [`Self::vertical_metrics`] returns. That
    /// method resolves §9.7.4.3's `/DW2` and `/W2`, which are the *`CIDFont` dictionary's*
    /// statement of `w1`; this reads OpenType's `vmtx`, which is the program's. ISO 19005-4 section
    /// 6.2.10.5 requires the two to agree where the program states them at all, and there is no way
    /// to ask that question of one number.
    ///
    /// `None` where the program states nothing — which is the common case, since a face never
    /// meant to be set vertically carries no `vmtx` — and for every refusal
    /// [`Self::program_advance`] lists, for the same reasons. It is deliberately *not* gated on
    /// [`Self::is_vertical`]: whether the file shows the font in writing mode 1 is the caller's
    /// question, and this answers only what the program says.
    #[must_use]
    pub fn program_vertical_advance(&self, code: Code) -> Option<f32> {
        let glyph = self.program_glyph(code)?;
        let height = metrics::program_advance_height(self.program, &self.data, glyph)?;
        // `vmtx` states a distance and §9.7.4.3 states a displacement, which for writing mode 1
        // runs down the page; the sign is PDF's convention rather than the table's.
        self.in_ems(-height)
    }

    /// One of the program's own units as a fraction of the em, for the two advance methods above.
    ///
    /// `None` for a program stating no em square, which no reader here produces and which would
    /// otherwise divide by zero.
    fn in_ems(&self, units: f32) -> Option<f32> {
        (self.units_per_em > 0.0).then(|| units / self.units_per_em)
    }

    /// Whether this font is shown in §9.2.4's writing mode 1, one glyph below the next.
    ///
    /// Set by the `CMap`'s `/WMode` (§9.7.5.1) and available only to a composite font, which
    /// is the clause's own restriction: "this feature is available only for composite fonts".
    #[must_use]
    pub fn is_vertical(&self) -> bool {
        self.vertical.is_some()
    }

    /// §9.7.4.3's vertical displacement `w1` and position vector `v`, in text-space units.
    ///
    /// `w1`'s horizontal component is 0 and `v` is the offset from the horizontal origin to
    /// the vertical one — so a glyph drawn in writing mode 1 is placed at `-v` from the
    /// current text position, and the position then moves by `w1`.
    ///
    /// Returns the horizontal metrics' degenerate form — no displacement, no offset — for a
    /// font in writing mode 0, so a caller that asks without checking gets a glyph that does
    /// not move rather than one that moves wrongly.
    #[must_use]
    pub fn vertical_metrics(&self, code: Code) -> ([f32; 2], [f32; 2]) {
        let Some(vertical) = self.vertical.as_ref() else {
            return ([0.0, 0.0], [0.0, 0.0]);
        };
        let cid = self.selector(code);
        let width = self.widths.get(cid).unwrap_or(self.default_width);
        let (displacement, position) = vertical.metrics(cid, width);
        (
            [displacement[0] / 1000.0, displacement[1] / 1000.0],
            [position[0] / 1000.0, position[1] / 1000.0],
        )
    }

    /// The pair kerning the document's own program states, for XFA 3.3 chapter 27's
    /// `kerning-mode:pair` (ADR 1682): `GPOS`'s `kern` feature, or the legacy `kern` table.
    ///
    /// Read on request rather than at load, because only a run asking for pair kerning needs it.
    ///
    /// # Errors
    ///
    /// [`crate::pairs::NoPairs::NotTheDocuments`] for a substituted font, whose pairs would be
    /// the stand-in's, unless it is one of §9.6.2.2's fourteen by name, which is kerned by the
    /// pairs Adobe published for the face (ADR 1916); [`crate::pairs::NoPairs::NoPairData`] for a
    /// bare CFF or Type 1 program, which carries neither table; and the reader's own refusals
    /// otherwise.
    pub fn pairs(&self) -> Result<crate::pairs::Pairs, crate::pairs::NoPairs> {
        if self.substituted {
            return self
                .published_pairs
                .clone()
                .ok_or(crate::pairs::NoPairs::NotTheDocuments);
        }
        if self.program != Program::Sfnt {
            return Err(crate::pairs::NoPairs::NoPairData);
        }
        crate::pairs::Pairs::read(Arc::clone(&self.data), self.units_per_em)
    }

    /// The adjustments [`Self::pairs`] gives a run of this font's codes, in logical order;
    /// `scripts` gives, per code, the script its character resolves to
    /// ([`crate::pairs::scripts`]), which selects the program's script table, `language` the
    /// run's natural language, which selects the table's language system
    /// ([`crate::pairs::Language`]), and `right_to_left` whether each code reads right to left. A
    /// code reaching no glyph of the program, or its `.notdef`, pairs with nothing; for one of
    /// the fourteen, a code is asked about by the glyph name it selects (§9.6.5), which is what a
    /// published pair names.
    #[must_use]
    pub fn pair_adjustments(
        &self,
        pairs: &crate::pairs::Pairs,
        codes: &[Code],
        scripts: &[crate::pairs::Script],
        language: crate::pairs::Language,
        right_to_left: &[bool],
    ) -> crate::pairs::Adjusted {
        let glyphs: Vec<Option<u16>> = if pairs.by_glyph_name() {
            codes
                .iter()
                .map(|code| {
                    self.selected_glyph_name(*code)
                        .and_then(crate::pairs::Pairs::published_glyph)
                })
                .collect()
        } else {
            codes.iter().map(|code| self.pair_glyph(*code)).collect()
        };
        pairs.adjust(&glyphs, scripts, language, right_to_left)
    }

    /// The program glyph a code reaches for pair kerning: [`Self::program_glyph`]'s mapping,
    /// without its refusal of a repaired CID-keyed CFF, whose glyph indices the repair leaves as
    /// they were.
    fn pair_glyph(&self, code: Code) -> Option<u16> {
        let glyph = match &self.mapping {
            CodeMapping::Named(table) => *table.get(usize::try_from(code.value()).ok()?)?,
            CodeMapping::Composite { cmap, glyphs } => {
                cmap.cid(code).and_then(|cid| glyphs.glyph(cid))
            }
            CodeMapping::Substituted { .. } | CodeMapping::MetricsOnly { .. } => None,
        }?;
        (glyph != NOTDEF_GLYPH).then_some(glyph)
    }
}
