//! What the embedded font program itself states, as opposed to what the document says of it:
//! its size, its `cmap` subtables, its glyph names and character identifiers, and the glyph a
//! code reaches in it.

use super::{CodeMapping, LoadedFont, NOTDEF_GLYPH};
use crate::cff::CodeToGlyph;
use crate::cmap::Code;
use crate::name_keyed::NameKeyed;
use crate::program::Program;
use crate::truetype;
use skrifa::FontRef;
use skrifa::raw::TableProvider;

impl LoadedFont {
    /// How many bytes the glyph program this font draws from occupies.
    ///
    /// What a caller keeping fonts across pages charges itself for — `pdf_model::FontCache` is
    /// the one, and its `FONT_BUDGET` is what it bounds. The *decoded*
    /// program rather than the stream in the file, because that is what is held; and for a
    /// substituted font it is the face this crate stood in with, which is held just the same.
    ///
    /// **The program is not the whole of what a loaded font retains** — the widths, the
    /// `CMap`s, the glyph names and the outlines built on demand sit beside it — and this
    /// deliberately does not guess at those. It is the term that dominates and the only one
    /// that is exact; what the rest add is measured rather than estimated, as peak resident
    /// memory over a sweep (ADR 0710).
    #[must_use]
    pub fn program_bytes(&self) -> usize {
        self.data.len()
    }

    /// The `cmap` subtables the embedded program carries, as platform and encoding ID pairs.
    ///
    /// ISO 32000-2 §9.6.5.4 names the subtables it uses by exactly those two numbers, and so
    /// do the requirements written on top of it — ISO 19005-2 section 6.2.11.6 and ISO 19005-4
    /// Section 6.2.10.6 among them. This crate's own reader keeps only the three the subclause's
    /// algorithm needs; a caller asking what a font *contains* rather than what it can draw
    /// needs the whole list, in the table's order.
    ///
    /// `None` for a font whose program is not an sfnt — a bare CFF or Type 1 program has
    /// nowhere to put a `cmap` — and for a substituted one, whose tables are this machine's.
    /// An sfnt carrying no `cmap` at all is an empty list, which is a statement about the file.
    #[must_use]
    pub fn program_cmap_subtables(&self) -> Option<Vec<(u16, u16)>> {
        if self.substituted {
            return None;
        }
        truetype::cmap_subtables(&self.data)
    }

    /// Every glyph name the embedded **name-keyed** program defines, in the program's own order.
    ///
    /// The set a Type 1 or Type 1C program's charset assigns, which is what a font descriptor's
    /// `/CharSet` string (ISO 32000-2 §9.8.1, Table 122) claims to list. Asked by a caller
    /// checking that claim, so the answer has to be the *program's* set rather than the set the
    /// document's encoding can reach — a name in the program that no code selects is exactly the
    /// case such a claim gets wrong.
    ///
    /// `.notdef` is included: it is a glyph the program defines like any other, and whether the
    /// clause counting them means to count it is the caller's reading rather than this crate's.
    ///
    /// `None` where the question does not apply or cannot be answered honestly: a substituted
    /// font, whose program is this machine's; an sfnt, which keys its glyphs by index; and a
    /// CID-keyed CFF, whose charset assigns CIDs rather than names (see
    /// [`Self::program_character_identifiers`]).
    #[must_use]
    pub fn program_glyph_names(&self) -> Option<Vec<String>> {
        if self.substituted {
            return None;
        }
        let names = |keyed: &NameKeyed| {
            keyed
                .by_name
                .keys()
                .map(|name| name.as_ref().to_owned())
                .collect()
        };
        match self.program {
            Program::Type1 => Some(names(&self.type1.as_ref()?.code_to_glyph().ok()?)),
            Program::BareCff => match CodeToGlyph::read(&self.data).ok()? {
                CodeToGlyph::Named(keyed) => Some(names(&keyed)),
                CodeToGlyph::Keyed { .. } => None,
            },
            Program::Sfnt => None,
        }
    }

    /// Every character identifier the embedded **CID-keyed** program defines.
    ///
    /// The counterpart of [`Self::program_glyph_names`] for a composite font, and what a font
    /// descriptor's `/CIDSet` stream (§9.8.3.1, Table 124) claims to mark. The two formats state
    /// it differently and both are read:
    ///
    /// - A **CID-keyed CFF** — a `CIDFontType0`'s `/FontFile3` — assigns a CID to each glyph in
    ///   its charset, so the set is that charset's values.
    /// - An **sfnt** — a `CIDFontType2`'s `/FontFile2` — has no CIDs of its own. §9.7.4.2 says a
    ///   CID reaches a glyph through the `CIDFont`'s `/CIDToGIDMap`, and the identity map is
    ///   both the default and what all but a handful of files state, so the identifiers are the
    ///   glyph indices the program holds. A caller for which that assumption is wrong has the
    ///   document's `/CIDToGIDMap` in front of it and this crate does not.
    ///
    /// `None` for a substituted font, for a name-keyed program, and where the program cannot be
    /// read at all.
    #[must_use]
    pub fn program_character_identifiers(&self) -> Option<Vec<u16>> {
        if self.substituted {
            return None;
        }
        match self.program {
            Program::BareCff => match CodeToGlyph::read(&self.data).ok()? {
                CodeToGlyph::Keyed { by_cid } => Some(by_cid.into_keys().collect()),
                CodeToGlyph::Named(_) => None,
            },
            Program::Sfnt => {
                let font = FontRef::new(&self.data).ok()?;
                let count = font.maxp().ok()?.num_glyphs();
                Some((0..count).collect())
            }
            Program::Type1 => None,
        }
    }

    /// The glyph a code selects **in the producer's own program**, or nothing.
    ///
    /// Deliberately stricter than [`Self::glyph_for`], which answers what gets *drawn* and
    /// therefore applies §9.7.6.3's fallbacks to CID 0. The two advance methods above compare a
    /// number the program states against a number the document states, and a fallback glyph's
    /// metrics are a statement about a code the font does not have. See
    /// [`Self::program_advance`] for the whole list of what this declines and why.
    pub(super) fn program_glyph(&self, code: Code) -> Option<u16> {
        if self.substituted || self.font_dicts.is_some() {
            return None;
        }
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
