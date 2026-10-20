//! From a character code to a glyph index: the selector a code resolves to, the face the glyph
//! belongs to, and the lookup by selector or by character.

use super::{CharacterGlyph, CodeMapping, Face, LoadedFont, NOTDEF_GLYPH, Placed};
use crate::cmap::Code;
use crate::vertical::Form;
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};

impl LoadedFont {
    /// The character selector a code resolves to, which for a composite font is a CID.
    ///
    /// Only the two lookups §9.7.6.2 names, in its order: the character mappings, then the
    /// notdef mappings. Failing both, CID 0, which §9.7.6.3's NOTE states:
    ///
    /// > If the `CMap` does not contain either a character mapping or a notdef mapping for the
    /// > code, descendant 0 shall be selected and the glyph for CID 0 shall be substituted from
    /// > the associated `CIDFont`.
    ///
    /// A simple font has no CID and its code indexes both its glyph table and its `/Widths`
    /// directly, so the code is its own selector.
    pub(super) fn selector(&self, code: Code) -> u32 {
        let Some(cmap) = self.cmap() else {
            return code.value();
        };
        cmap.cid(code)
            .or_else(|| cmap.notdef_cid(code))
            .unwrap_or(0)
    }

    /// Resolves a character code to a glyph index.
    ///
    /// Deliberately not memoised. Two of the mappings build a `FontRef` here, which looks
    /// like a per-character cost worth caching — but measuring it on a dense specification
    /// page (3587 lookups, 211 distinct codes, two thirds of them through the character
    /// map) moved the interpretation pass by less than the run-to-run noise. `FontRef` is
    /// a zero-copy view over the table directory, not a parse. A cache here would be
    /// unmeasured cleverness, and `CLAUDE.md` forbids that.
    pub(super) fn glyph_for(&self, code: Code) -> Option<u16> {
        self.placed_glyph(code).map(|placed| placed.glyph)
    }

    /// The same glyph, together with the face it is an index into.
    ///
    /// Separated from [`Self::glyph_for`] because only the drawing route needs the pair:
    /// everything that counts codes, compares a glyph with [`NOTDEF_GLYPH`] or asks whether a
    /// code reached anything is asking about the *font*, and a font answers with one glyph
    /// however many faces ISO 32000-2 §9.8.3.3's `/FD` gave it.
    pub(super) fn placed_glyph(&self, code: Code) -> Option<Placed> {
        let own = |glyph| {
            Some(Placed {
                face: Face::Own,
                glyph,
            })
        };
        match &self.mapping {
            CodeMapping::Composite { cmap, glyphs } => {
                // §9.7.6.3's two fallbacks, in its order. "If a code maps to a CID for which
                // no such glyph exists in the descendant CIDFont, the notdef mappings in the
                // CMap shall be consulted … If no glyph exists for that CID, the glyph for
                // CID 0 (which shall be present) shall be substituted." The second is also
                // the sentence about a `/CIDToGIDMap` stream too short for a CID: "if a
                // (character) code does not have a corresponding GID in the CIDtoGIDMap
                // stream, the glyph for CID 0 shall be substituted".
                if let Some(glyph) = cmap.cid(code).and_then(|cid| glyphs.glyph(cid)) {
                    return own(glyph);
                }
                if let Some(glyph) = cmap.notdef_cid(code).and_then(|cid| glyphs.glyph(cid)) {
                    return own(glyph);
                }
                glyphs.glyph(0).and_then(own)
            }
            // The substitute has no notion of this document's CIDs, so the code is taken
            // to the character it stands for and that character is looked up.
            //
            // **And then the writing mode is asked, because the character is not the whole of
            // what the producer chose.** §9.7.5.1's NOTE says a vertical `CMap` names a
            // different CID where the shape differs, and the CID-to-Unicode table this arm
            // reaches the substitute through is keyed to the *character* — Adobe-Japan1's 7911
            // and 686 are both U+300C. So where the collection says this CID is a vertical form
            // and the face states one for the glyph, that is the glyph the file asked for.
            // [`crate::vertical`] holds the argument and names what neither half is derived
            // from.
            CodeMapping::Substituted { .. } => {
                let (_, placed, form) = self.substituted_glyph(code)?;
                Some(match form {
                    // A rotated form is another glyph of the *same* face, so the face the class
                    // chose travels with it.
                    Form::Rotated(rotated) => Placed {
                        glyph: rotated,
                        ..placed
                    },
                    Form::Upright | Form::Unsupplied => placed,
                })
            }
            // Resolved when the font was loaded. A code with no entry has no glyph, and
            // that is final: falling back to the code as a glyph index here is exactly
            // how a font draws plausible, wrong text.
            CodeMapping::Named(table) => own((*table.get(usize::try_from(code.value()).ok()?)?)?),
            // No program was read, so there is no glyph of any kind: what this font states is
            // where the *next* one goes ([`Self::metrics_only`]).
            CodeMapping::MetricsOnly { .. } => None,
        }
    }

    /// The glyph index a character code reaches, or `None` where it reaches none.
    ///
    /// Public for one reason: the strongest check in this tree is that the document's stated
    /// width for a code and the font program's own advance for the glyph that code reaches
    /// agree, and those two statements travel through completely separate structures — so they
    /// agree only if the whole chain (§9.7.6.2's `CMap`, §9.7.4.2's `CIDToGIDMap` or charset)
    /// landed on the glyph the producer meant. That check needs the glyph index, and it
    /// verifies the mapping without consulting the mapping. See
    /// `pdf-model/tests/composite_fonts.rs`.
    #[must_use]
    pub fn glyph_index(&self, code: Code) -> Option<u16> {
        self.glyph_for(code)
    }

    /// The glyph a *character selector* reaches, skipping the code that selected it.
    ///
    /// A simple font's `/Widths` is indexed by character code and a composite font's `/W` by
    /// CID (§9.7.4.3), so this is the key both tables use — which is what the
    /// widths-against-charstrings cross-check needs, since it walks those tables rather than a
    /// string. Drawing always starts from a code; the two callers that do not are that check
    /// and §9.10.2's last resort in [`Self::text_from_program`], which needs the glyph in order
    /// to ask the program what it is called.
    pub(super) fn glyph_for_selector(&self, selector: u32) -> Option<u16> {
        match &self.mapping {
            CodeMapping::Named(table) => *table.get(usize::try_from(selector).ok()?)?,
            CodeMapping::Composite { glyphs, .. } => glyphs.glyph(selector),
            // A substitute is reached through what a code *means*, so a selector alone
            // cannot name a glyph in it; a metrics-only font has no glyphs to name.
            CodeMapping::Substituted { .. } | CodeMapping::MetricsOnly { .. } => None,
        }
    }

    /// What this font's *program* draws for a character, for a caller that has no code.
    ///
    /// **This is not a route a document's text may take, and the distinction is the whole of why
    /// it exists.** A document selects a glyph by character code — §9.6.5's encoding for a simple
    /// font, §9.7.6's `CMap` for a composite one — and drawing a glyph the file did not select
    /// would be inventing what the page says. What has no code at all is the text a *program*
    /// draws for itself: a panel of §12.3.3's outline titles, §8.11.4.3's layer names, §12.4.2's
    /// page labels. There is no font dictionary behind those and therefore no encoding, so the
    /// question they ask is the one this answers — which glyph does this face state for this
    /// character — and [`Self::code_for`] cannot answer it, because a simple font's encoding is
    /// 256 codes wide and a panel's text is not.
    ///
    /// `None` where the program states no glyph for the character, and `None` for every program
    /// with no `cmap` at all: a bare CFF or a Type 1 program is keyed by glyph *name*, which is a
    /// different question, and none of the compiled-in faces so keyed carries anything outside
    /// the standard Latin character set for it to find (ADR 0270). A caller reports the absence —
    /// `viewer_ui::chrome` draws a box for it (ADR 0195).
    ///
    /// The advance is the program's own, from `hmtx`, because there is no `/Widths` array to
    /// disagree with: an interface's text is nothing a document stated a width for.
    ///
    /// **A `cmap` entry naming [`NOTDEF_GLYPH`] answers nothing**, on this crate's own reading of
    /// what glyph 0 is: a statement of absence rather than a glyph. Drawing it would put a
    /// designer's box on the screen while telling the caller the character was set, which is
    /// exactly the confident wrong mark a placeholder exists to avoid.
    #[must_use]
    pub fn character_glyph(&self, character: char) -> Option<CharacterGlyph> {
        let program = FontRef::new(&self.data).ok()?;
        let glyph = program.charmap().map(character)?;
        if glyph.to_u32() == u32::from(NOTDEF_GLYPH) {
            return None;
        }
        let advance = program
            .glyph_metrics(Size::unscaled(), LocationRef::default())
            .advance_width(glyph)?
            / self.units_per_em;
        Some(CharacterGlyph {
            outline: u16::try_from(glyph.to_u32()).ok().and_then(|glyph| {
                self.cached_outline(Placed {
                    face: Face::Own,
                    glyph,
                })
            }),
            advance,
        })
    }

    /// Whether this code reached its glyph only by a mapping ISO 32000-2 §9.6.5.4 leaves open.
    ///
    /// That subclause sets out how a simple `TrueType` or `OpenType` font's codes reach glyphs
    /// and then closes the list with a licence:
    ///
    /// > If a character cannot be mapped in any of the ways described previously, a PDF
    /// > processor may supply a mapping of its choosing.
    ///
    /// `crate::truetype` supplies two such mappings, and this says which codes they answered.
    /// `false` for a code the subclause's own steps reached, for a code that reaches no glyph at
    /// all, and for every font of another kind — a composite font, a Type 1 or bare CFF program,
    /// or a substitute — because none of those is what that sentence is about.
    ///
    /// # Why this is exposed
    ///
    /// Because a *conforming file* is the one thing the licence is not addressed to. ISO 19005-2
    /// Section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 require a rendered TrueType font's codes
    /// to reach glyphs by the base standard's procedure without a mapping the reader chooses — so a
    /// validator has to be able to ask which route drew a code, and the route is knowable only
    /// where the table is built. It costs a viewer thirty-two bytes per loaded font and no work at
    /// all: the bit is set inside a loop that was already running.
    #[must_use]
    pub fn glyph_needed_a_reader_chosen_mapping(&self, code: Code) -> bool {
        self.reader_chosen.contains(code.value())
    }
}
