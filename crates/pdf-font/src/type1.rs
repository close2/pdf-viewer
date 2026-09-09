//! Bare Type 1 font programs — ISO 32000-2 §9.9's `/FontFile`.
//!
//! A Type 1 font program is, in §9.6.2.1's words, "a stylised PostScript language program
//! that describes glyph shapes": a cleartext header holding the font's dictionaries and its
//! built-in `/Encoding` array, then an `eexec`-encrypted section holding the charstrings,
//! each of which is encrypted again under its own key. §9.9.1's Table 125 wraps that in
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
pub struct Program {
    /// What `read-fonts` parsed out of the program.
    font: Type1Font,
    /// Which of the 256 codes the program's own `/Encoding` array assigns a name to.
    ///
    /// # Why this is read here rather than asked of `read-fonts`
    ///
    /// `read_fonts::ps::type1` builds a custom encoding as a 256-entry vector **pre-filled with
    /// `GlyphId::NOTDEF`** and fills in the entries the array names, so `Encoding::map` answers
    /// `Some(0)` for a code the array never mentions — indistinguishable, through that API, from
    /// a code the array deliberately points at `.notdef`. `Encoding::glyph_name` resolves the
    /// same slot and so cannot tell them apart either. The CFF reader beside this one produces a
    /// genuine `None`, so the two producers of [`NameKeyed`] disagreed about what *unencoded*
    /// means, and everything downstream read the difference: an unassigned code selected glyph 0
    /// and drew the designer's `.notdef` — commonly a filled box — where the program says
    /// nothing should be drawn at all.
    ///
    /// The distinction is not recoverable from the resolved map, so it is taken from the place
    /// the program states it: the `/Encoding` array in the **cleartext** header, before `eexec`.
    /// [`assigned_codes`] reads only which codes the array mentions, never what they mean; the
    /// name and the glyph stay `read-fonts`' answer.
    ///
    /// **A code the array assigns to a name the program does not contain stays assigned**, and
    /// that is ISO 32000-2 §9.6.5.2's own instruction rather than a convenience:
    ///
    /// > If an encoding maps to a character name that does not exist in the Type 1 font program,
    /// > the .notdef glyph shall be substituted.
    ///
    /// So `.notdef` is the right answer for that code, and the wrong answer for one the array
    /// never mentioned.
    assigned: Box<[bool; 256]>,
}

impl std::fmt::Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Program")
            .field("name", &self.font.name())
            .field("glyphs", &self.font.num_glyphs())
            .field(
                "codes the built-in encoding assigns",
                &self.assigned.iter().filter(|slot| **slot).count(),
            )
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
        Ok(Self {
            font: Type1Font::new(data).map_err(|e| malformed(&e))?,
            assigned: assigned_codes(data),
        })
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
        let upem = self.font.upem();
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
        for (glyph, name) in self.font.glyph_names() {
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
        if let Some(encoding) = self.font.encoding() {
            // A predefined encoding resolves each code through the charstring names, so an
            // unmapped code already comes back as `None` and the array below has nothing to say
            // about it. Only the custom form is pre-filled with `.notdef`; see `Self::assigned`.
            let custom = encoding.predefined().is_none();
            for (code, slot) in builtin.iter_mut().enumerate() {
                let Ok(code) = u8::try_from(code) else {
                    continue;
                };
                if custom && !self.assigned.get(usize::from(code)).copied().unwrap_or(false) {
                    continue;
                }
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
        self.font
            .draw(GlyphId::from(glyph), None, pen)
            .map_err(|e| malformed(&e))?;
        Ok(())
    }
}

/// Which of the 256 codes a program's own `/Encoding` array assigns a name to.
///
/// ISO 32000-2 §9.6.2.1 calls a Type 1 program "a stylised PostScript language program", and the
/// `/Encoding` array is in the **cleartext** part of it, before `eexec` — which is why this can
/// be read without decrypting anything. The array is written one assignment per code, in the
/// shape PostScript gives it:
///
/// ```text
/// dup 32 /space put
/// ```
///
/// so the codes it mentions are the codes it assigns. A program that names a predefined encoding
/// instead — `/Encoding StandardEncoding def` — mentions no code here, and its caller does not
/// ask: [`Program::assigned`] says why.
///
/// # What this deliberately does not do
///
/// It does not parse PostScript. It reads `dup <integer> /<name> put` and nothing else, because
/// the only question it answers is *which codes are mentioned* — the name, the glyph and every
/// other property stay `read-fonts`' answer. A program that builds its encoding by some other
/// PostScript construction is read as mentioning no codes, which restores exactly the behaviour
/// this file had before: the resolved map, `.notdef` and all.
///
/// The scan stops at `eexec` because everything after it is encrypted and cannot contain the
/// array. On the load path this is one pass over a header that is a few kilobytes in a real
/// font, against a parse of the whole program that has already happened.
fn assigned_codes(data: &[u8]) -> Box<[bool; 256]> {
    let mut assigned = Box::new([false; 256]);
    let cleartext = match find(data, b"eexec") {
        Some(at) => data.get(..at).unwrap_or(data),
        None => data,
    };
    let mut rest = cleartext;
    while let Some(at) = find(rest, b"dup ") {
        rest = rest.get(at.saturating_add(4)..).unwrap_or_default();
        let mut fields = rest.splitn(3, u8::is_ascii_whitespace);
        let (Some(code), Some(name), Some(tail)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        // `dup` appears in a Type 1 program for things other than an encoding assignment — a
        // `Subrs` entry is `dup <index> <length> RD <binary> NP`. Requiring the `/name put`
        // shape is what makes this one an assignment, so anything else is passed over rather
        // than guessed at.
        if !name.starts_with(b"/") || !tail.starts_with(b"put") {
            continue;
        }
        let Ok(code) = std::str::from_utf8(code).unwrap_or_default().parse::<usize>() else {
            continue;
        };
        if let Some(slot) = assigned.get_mut(code) {
            *slot = true;
        }
    }
    assigned
}

/// The first offset at which `needle` occurs in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn malformed(error: &skrifa::raw::ps::error::Error) -> Type1Error {
    Type1Error::Malformed {
        detail: format!("{error:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::assigned_codes;

    /// The shape a Type 1 program writes an encoding assignment in.
    ///
    /// One `dup <code> /<name> put` per code, in the cleartext header. The array is normally
    /// preceded by `/Encoding 256 array` and `0 1 255 {1 index exch /.notdef put} for`, and the
    /// second of those is why the resolved map is full of `.notdef` — it is the program filling
    /// the array before assigning the codes it actually uses.
    const SPARSE: &[u8] = b"/Encoding 256 array\n\
        0 1 255 {1 index exch /.notdef put} for\n\
        dup 32 /space put\n\
        dup 65 /A put\n\
        readonly def\n\
        currentfile eexec\n\
        dup 99 /neverreached put\n";

    #[test]
    fn only_the_codes_the_array_mentions_are_assigned() {
        let assigned = assigned_codes(SPARSE);
        assert!(assigned[32], "the array assigns code 32");
        assert!(assigned[65], "and code 65");
        assert!(!assigned[33], "and mentions no other, whatever the map resolves to");
        assert_eq!(assigned.iter().filter(|slot| **slot).count(), 2);
    }

    #[test]
    fn nothing_after_eexec_is_read() {
        // Everything past `eexec` is encrypted, so a byte sequence there that happens to spell an
        // assignment is a coincidence rather than one. Scanning it would let ciphertext decide
        // which codes a font encodes.
        assert!(!assigned_codes(SPARSE)[99]);
    }

    #[test]
    fn a_subrs_entry_is_not_an_assignment() {
        // `dup <index> <length> RD <binary> NP` is how a Type 1 program writes a subroutine, and
        // it opens with the same keyword. The `/name put` shape is what tells the two apart.
        let subrs = b"/Subrs 2 array\ndup 0 15 RD ................ NP\ndup 1 9 RD ......... NP\n";
        assert_eq!(assigned_codes(subrs).iter().filter(|slot| **slot).count(), 0);
    }

    #[test]
    fn a_code_outside_the_array_is_ignored_rather_than_wrapping() {
        // A program may write a code no 256-entry array can hold. It names no code this reader
        // reports, and it must not fold onto one that it does.
        let wide = b"dup 300 /A put\ndup 65 /B put\n";
        let assigned = assigned_codes(wide);
        assert!(assigned[65]);
        assert_eq!(assigned.iter().filter(|slot| **slot).count(), 1);
    }

    #[test]
    fn a_program_that_names_a_predefined_encoding_mentions_no_code() {
        // `Program::assigned`'s caller does not consult this table for a predefined encoding,
        // because that form resolves through the charstring names and already answers `None`
        // for a code it does not map. The table is empty either way.
        let standard = b"/Encoding StandardEncoding def\ncurrentfile eexec\n";
        assert_eq!(
            assigned_codes(standard).iter().filter(|slot| **slot).count(),
            0
        );
    }
}
