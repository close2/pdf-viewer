//! Bare CFF font programs.
//!
//! `/FontFile3` may hold a bare CFF font program rather than a complete `OpenType` file.
//! A CFF carries no `cmap` table, so a character code reaches a glyph by a different route
//! than in a `TrueType` font, and that route is what this module supplies.
//!
//! # The two routes, and why the distinction matters
//!
//! A *name-keyed* CFF — what a PDF simple font embeds — names every glyph in its charset.
//! A character code becomes a glyph name (from the PDF `/Encoding`, or from the encoding
//! the font itself carries) and the name becomes a glyph index. A *CID-keyed* CFF — what a
//! composite font embeds — has no glyph names at all: its charset assigns a CID per glyph,
//! and a code reaches a glyph by inverting that.
//!
//! Confusing the two is not a harmless error. Treating a character code as a glyph index,
//! which is what happens when neither route is taken, loads without complaint and draws
//! the wrong glyphs — text that looks like text and says something else. That failure is
//! why [`crate::LoadedFont::glyph_for`] refuses to guess.
//!
//! # Why this module is thin
//!
//! Every byte-level structure involved — the INDEX and DICT encodings, charset formats 0,
//! 1 and 2, encoding formats 0 and 1 with their supplements, the String INDEX and Adobe's
//! 391 standard strings — is parsed by `read-fonts`, which `skrifa` already brings in and
//! which is fuzzed and memory-safe. Hand-rolling that parsing would mean writing exactly
//! the untrusted-input byte handling this project chose `skrifa` to avoid, and would mean
//! transcribing a 391-entry table by hand. See `doc/adr/0006-cff-through-read-fonts.md`.
//!
//! What is left here is the PDF-specific part: deciding *which* route applies, and putting
//! the results in a form the loader can use.

use std::collections::BTreeMap;

use skrifa::GlyphId;
use skrifa::outline::OutlinePen;
use skrifa::raw::ps::cff::CffFontRef;

/// Why a bare CFF font program could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CffError {
    /// The program could not be parsed.
    #[error("the CFF font program could not be read: {detail}")]
    Malformed {
        /// What `read-fonts` reported.
        detail: String,
    },
    /// The program parsed, but carries no charset.
    ///
    /// Without one there is no way to reach a glyph from a name or a CID, and the only
    /// remaining option would be to assume the character code is a glyph index — the
    /// silent-wrong-glyph failure this module exists to prevent.
    #[error("the CFF font program has no charset, so its glyphs cannot be identified")]
    NoCharset,
}

/// The character-code mapping a bare CFF carries in place of a `cmap` table.
///
/// Built once when the font is loaded, because resolving it per glyph would mean
/// re-parsing the font program on every character drawn.
#[derive(Debug)]
pub enum CodeToGlyph {
    /// A name-keyed font, which is what a PDF *simple* font embeds.
    Named {
        /// Glyph index by glyph name, taken from the font's charset.
        by_name: BTreeMap<Box<str>, u16>,
        /// Glyph index by character code, taken from the encoding the font itself carries.
        ///
        /// ISO 32000-2 §9.6.5.1 Table 112 makes this the *base* encoding whenever the font
        /// program is embedded and the `/Encoding` dictionary names no `/BaseEncoding`, so
        /// it is what a `/Differences` array describes differences from rather than only a
        /// fallback for a code nothing else reached.
        builtin: Box<[Option<u16>; 256]>,
        /// The glyph name the built-in encoding gives each character code.
        ///
        /// The same mapping as `builtin`, carried through the charset instead of stopping
        /// at the glyph index. Nothing about *drawing* needs it — `builtin` selects the
        /// glyph directly — but a code's glyph name is what a document with no
        /// `/ToUnicode` means by that code, so text extraction and
        /// [`crate::LoadedFont::code_for`] would otherwise lose every code the PDF
        /// encoding left to the font.
        builtin_names: Box<[Option<Box<str>>; 256]>,
    },
    /// A CID-keyed font, which is what a PDF *composite* font embeds.
    Keyed {
        /// Glyph index by CID, inverting the charset.
        by_cid: BTreeMap<u16, u16>,
    },
}

impl CodeToGlyph {
    /// Reads the mapping out of a bare CFF font program.
    ///
    /// # Errors
    ///
    /// See [`CffError`].
    pub fn read(data: &[u8]) -> Result<Self, CffError> {
        let font = open(data)?;
        let charset = font.charset().ok_or(CffError::NoCharset)?;

        // A CID-keyed font's charset assigns CIDs, not names, and its `Encoding` — which
        // `read-fonts` will still report as the default — is meaningless. Reading names
        // out of it would yield whatever standard string happened to share the CID's
        // number, which is how a composite font ends up drawing Latin letters for CJK.
        if font.is_cid() {
            let mut by_cid = BTreeMap::new();
            for (glyph, cid) in charset.iter() {
                if let Ok(glyph) = u16::try_from(glyph.to_u32()) {
                    by_cid.entry(cid.to_u16()).or_insert(glyph);
                }
            }
            return Ok(Self::Keyed { by_cid });
        }

        let mut by_name = BTreeMap::new();
        let mut by_glyph = BTreeMap::new();
        for (glyph, sid) in charset.iter() {
            let Ok(glyph) = u16::try_from(glyph.to_u32()) else {
                continue;
            };
            let Some(name) = font.string(sid) else {
                continue;
            };
            // Glyph names are ASCII by specification; a font that breaks that is
            // malformed, not a reason to give up on the glyphs that are well formed.
            let name = String::from_utf8_lossy(name).into_owned().into_boxed_str();
            by_glyph.entry(glyph).or_insert_with(|| name.clone());
            // A duplicated name keeps the lowest glyph index, matching the order a
            // charset assigns them.
            by_name.entry(name).or_insert(glyph);
        }

        let mut builtin = Box::new([None; 256]);
        if let Some(encoding) = font.encoding() {
            for (code, slot) in builtin.iter_mut().enumerate() {
                let Ok(code) = u8::try_from(code) else {
                    continue;
                };
                *slot = encoding
                    .map(code)
                    .and_then(|glyph| u16::try_from(glyph.to_u32()).ok());
            }
        }
        let builtin_names = Box::new(std::array::from_fn(|code| {
            builtin
                .get(code)
                .copied()
                .flatten()
                .and_then(|glyph| by_glyph.get(&glyph).cloned())
        }));

        Ok(Self::Named {
            by_name,
            builtin,
            builtin_names,
        })
    }
}

/// Reads a font program's units per em.
///
/// A bare CFF states its scale in its `FontMatrix` rather than in a `head` table; the
/// default is one thousandth, meaning 1000 units per em.
///
/// # Errors
///
/// See [`CffError`].
pub fn units_per_em(data: &[u8]) -> Result<f32, CffError> {
    let font = open(data)?;
    let upem = font.upem();
    if upem <= 0 {
        return Err(CffError::Malformed {
            detail: format!("units per em is {upem}"),
        });
    }
    // A units-per-em beyond f32's exact integer range is not a font scale.
    #[expect(
        clippy::cast_precision_loss,
        reason = "checked non-zero and, in any real font, far below f32's exact integer limit"
    )]
    Ok(upem as f32)
}

/// Draws one glyph from a bare CFF font program.
///
/// # Errors
///
/// See [`CffError`].
pub fn draw(data: &[u8], glyph: u16, pen: &mut impl OutlinePen) -> Result<(), CffError> {
    let font = open(data)?;
    let id = GlyphId::from(glyph);
    // A CID-keyed font splits its private dictionaries across subfonts selected per glyph,
    // so the subfont must be resolved before the charstring can be interpreted.
    let index = font.subfont_index(id).ok_or_else(|| CffError::Malformed {
        detail: format!("glyph {glyph} selects no subfont"),
    })?;
    let subfont = font.subfont(index, &[]).map_err(|e| malformed(&e))?;
    // No size in pixels per em: the outline stays in font units and the caller normalises
    // it, because a PDF text matrix scales it afterwards anyway.
    font.draw(&subfont, id, &[], None, pen)
        .map_err(|e| malformed(&e))?;
    Ok(())
}

/// Opens a bare CFF font program.
///
/// The units per em is left unstated so it is taken from the font's own `FontMatrix`;
/// passing one would only be right for a CFF inside an `OpenType` file, which has a `head`
/// table to take it from.
fn open(data: &[u8]) -> Result<CffFontRef<'_>, CffError> {
    CffFontRef::new_cff(data, 0, None).map_err(|e| malformed(&e))
}

fn malformed(error: &skrifa::raw::ps::error::Error) -> CffError {
    CffError::Malformed {
        detail: format!("{error:?}"),
    }
}
