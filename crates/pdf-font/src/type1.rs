//! Bare Type 1 font programs — ISO 32000-2 §9.9's `/FontFile`.
//!
//! A Type 1 font program is, in §9.6.2.1's words, "a stylised PostScript language program
//! that describes glyph shapes": a cleartext header holding the font's dictionaries and its
//! built-in `/Encoding` array, then an `eexec`-encrypted section holding the charstrings,
//! each of which is encrypted again under its own key. §9.9.1's Table 123 wraps that in
//! three lengths, `/Length1`, `/Length2` and `/Length3`, one per section.
//!
//! # Why this module is thin, and why it exists at all
//!
//! Every byte-level structure — the PFB segment headers, the `eexec` decryption, the
//! charstring index, the Type 1 charstring interpreter with its `seac` and `hsbw` operators
//! — is parsed by `read_fonts::ps::type1`, on the same argument ADR 0006 makes for CFF: this
//! is exactly the untrusted-input byte handling the project chose `skrifa` to avoid writing.
//! What is left here is the PDF-specific part, and it is the same part `cff.rs` supplies,
//! because §9.6.5.2's encoding algorithm does not distinguish the two formats — §9.6.2.1's
//! NOTE 1 calls a CFF "an alternative, more compact but functionally equivalent
//! representation of a Type 1 font program". So both readers produce a
//! [`crate::name_keyed::NameKeyed`] and nothing downstream asks which one it came from.
//!
//! It exists because the format is not historical: the corpus embeds `/FontFile` on pages
//! that a substitute could not draw at all, and §9.9's Table 124 lists it as the way a
//! `Type1` or `MMType1` font's program is embedded.

use std::collections::BTreeMap;

use skrifa::GlyphId;
use skrifa::outline::OutlinePen;
use skrifa::raw::ps::type1::Type1Font;

use crate::name_keyed::NameKeyed;

/// Why a bare Type 1 font program could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Type1Error {
    /// The program could not be parsed.
    #[error("the Type 1 font program could not be read: {detail}")]
    Malformed {
        /// What `read-fonts` reported.
        detail: String,
    },
    /// The program parsed and names none of its glyphs.
    ///
    /// A Type 1 font keys its charstrings by name, so a program with no names has nothing
    /// an encoding can address. Refusing beats the only alternative, which is assuming the
    /// character code is a glyph index — the silent-wrong-glyph failure `cff.rs` refuses
    /// for the same reason.
    #[error("the Type 1 font program names none of its glyphs")]
    NoNames,
}

/// A parsed Type 1 font program.
///
/// Kept rather than re-derived, which is the one place this module *differs* from `cff.rs`
/// and it is a measured difference. A `CffFontRef` borrows its bytes and costs nothing to
/// make; a `Type1Font` decrypts the whole `eexec` section and indexes every charstring, and
/// on `tracemonkey.pdf`'s twelve embedded programs that is 10 to 86 µs each. `build_outline`
/// runs once per *distinct* glyph on a page, so re-parsing per glyph put about 6 ms on that
/// page's interpretation — against 7.7 ms for the whole of it before this landed.
pub struct Program(Type1Font);

impl std::fmt::Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Program")
            .field("name", &self.0.name())
            .field("glyphs", &self.0.num_glyphs())
            .finish()
    }
}

impl Program {
    /// Parses a bare Type 1 font program.
    ///
    /// # Errors
    ///
    /// See [`Type1Error`].
    pub fn parse(data: &[u8]) -> Result<Self, Type1Error> {
        Ok(Self(Type1Font::new(data).map_err(|e| malformed(&e))?))
    }

    /// Reads the program's units per em, which every outline is scaled by.
    ///
    /// A Type 1 font states its scale in its `/FontMatrix`, as a CFF does; the conventional
    /// value is one thousandth, meaning 1000 units per em.
    ///
    /// # Errors
    ///
    /// See [`Type1Error`].
    pub fn units_per_em(&self) -> Result<f32, Type1Error> {
        let upem = self.0.upem();
        if upem <= 0 {
            return Err(Type1Error::Malformed {
                detail: format!("units per em is {upem}"),
            });
        }
        // A units-per-em beyond f32's exact integer range is not a font scale.
        #[expect(
            clippy::cast_precision_loss,
            reason = "checked non-zero and, in any real font, far below f32's exact integer \
                      limit"
        )]
        Ok(upem as f32)
    }

    /// Reads the code and name mapping out of the program.
    ///
    /// # Errors
    ///
    /// See [`Type1Error`].
    pub fn code_to_glyph(&self) -> Result<NameKeyed, Type1Error> {
        let mut by_glyph = BTreeMap::new();
        for (glyph, name) in self.0.glyph_names() {
            let Ok(glyph) = u16::try_from(glyph.to_u32()) else {
                continue;
            };
            by_glyph
                .entry(glyph)
                .or_insert_with(|| name.to_owned().into_boxed_str());
        }
        if by_glyph.is_empty() {
            return Err(Type1Error::NoNames);
        }

        // §9.6.5.2: "A Type 1 font's built -in encoding shall be defined by an 'encoding'
        // array that is part of the font program, not to be confused with the Encoding entry
        // in the PDF font dictionary." A program stating none leaves every code to the PDF
        // encoding, which is what an empty table means here.
        let mut builtin = Box::new([None; 256]);
        if let Some(encoding) = self.0.encoding() {
            for (code, slot) in builtin.iter_mut().enumerate() {
                let Ok(code) = u8::try_from(code) else {
                    continue;
                };
                *slot = encoding
                    .map(code)
                    .and_then(|glyph| u16::try_from(glyph.to_u32()).ok());
            }
        }

        Ok(NameKeyed::new(&by_glyph, builtin))
    }

    /// Draws one glyph.
    ///
    /// # Errors
    ///
    /// See [`Type1Error`].
    pub fn draw(&self, glyph: u16, pen: &mut impl OutlinePen) -> Result<(), Type1Error> {
        // No size in pixels per em: the outline stays in font units and the caller
        // normalises it, because a PDF text matrix scales it afterwards anyway.
        self.0
            .draw(GlyphId::from(glyph), None, pen)
            .map_err(|e| malformed(&e))?;
        Ok(())
    }
}

fn malformed(error: &skrifa::raw::ps::error::Error) -> Type1Error {
    Type1Error::Malformed {
        detail: format!("{error:?}"),
    }
}
