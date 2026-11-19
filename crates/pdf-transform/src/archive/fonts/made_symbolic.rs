//! A font the file does not embed, whose own encoding sends a shown code to `.notdef`, embedded
//! with a `glyf` face as the symbolic font ISO 32000-2 §9.6.5.4 describes, the code reaching a
//! glyph that draws nothing (`doc/adr/3125`).
//!
//! # Why the font is made symbolic
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator to
//! reference `.notdef`, and the code is in the producer's content stream, which the conversion does
//! not edit. Where the file embeds no program, the conversion chooses the program, and
//! `doc/questions/A48` allows the code a real glyph that draws nothing, on the condition that no
//! character is stated for it (`super::empty`). The faces this program ships for the fonts a
//! descriptor names are `glyf` ones, which go under a `TrueType` dictionary (§9.9's Table 124),
//! and under a *non-symbolic* one a code reaches a glyph only through a name and the character
//! the Adobe Glyph List gives that name — the route on which every way to an empty glyph states a
//! character. §9.6.5.4's other route states none:
//!
//! > When the font has no Encoding entry, or the font descriptor's Symbolic flag is set (in which
//! > case the Encoding entry is ignored), this shall occur:
//!
//! a (3, 0) subtable read with 0xF000 prepended to each code. So the descriptor states the
//! Symbolic flag and not the Nonsymbolic one, the dictionary states no `/Encoding` (ISO 19005-2
//! section 6.2.11.6 asks that of a symbolic font), and the program carries the (1, 0) and (3, 0)
//! subtables ADR 2520's restatement writes, mapping each code to the glyph this reader drew for it
//! under the file's encoding and each code on `.notdef` to a glyph appended with no outline at the
//! width the dictionary states. The page draws exactly what this reader drew.
//!
//! # What the encoding said is kept as text
//!
//! Without its `/Encoding` the font's codes no longer name a character by the glyph-name method of
//! §9.10.2, so where the file states no `/ToUnicode` one is written from the file's own encoding:
//! each code whose name the Adobe Glyph List resolves, exactly as `super::super::to_unicode`
//! derives one for Level U. A code on `.notdef` has no name and is given no entry, so the empty
//! glyph states no character — A48's condition — and the text a reader extracts is the text it
//! extracted from the source.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::SelectedFont;
use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object};

use super::super::decision::Because;
use super::super::prepare::Spare;
use super::{Substitutes, Table124, truetype_cmap};

/// Table 121's Symbolic flag, bit position 3.
const SYMBOLIC: i64 = 1 << 2;

/// Table 121's Nonsymbolic flag, bit position 6.
const NONSYMBOLIC: i64 = 1 << 5;

/// The program made symbolic, and the glyph every code reaches in it.
pub(super) struct MadeSymbolic {
    /// The face with one empty glyph per distinct width and its `cmap` restated.
    pub(super) program: Vec<u8>,
    /// Every code the face reached a glyph for under the file's encoding, and its glyph.
    pub(super) reached: BTreeMap<u8, u16>,
    /// Every code on `.notdef`, and the empty glyph it now reaches.
    pub(super) given: BTreeMap<u8, u16>,
}

/// The face `program`, which `glyph_of` reads under the file's encoding, made symbolic: every code
/// reaching a glyph keeps it, and each of `empty` reaches a glyph drawing nothing at the advance
/// `font` states for it.
pub(super) fn restated(
    font: &LoadedFont,
    program: &[u8],
    empty: &BTreeSet<u8>,
    glyph_of: &dyn Fn(u8) -> Option<u16>,
) -> Result<MadeSymbolic, Because> {
    let reached: BTreeMap<u8, u16> = (0..=u8::MAX)
        .filter(|byte| !empty.contains(byte))
        .filter_map(|byte| {
            glyph_of(byte)
                .filter(|glyph| *glyph != pdf_font::NOTDEF_GLYPH)
                .map(|glyph| (byte, glyph))
        })
        .collect();
    let codes: Vec<u8> = empty.iter().copied().collect();
    let (program, given) = truetype_cmap::program_restated(font, program, &reached, (&[], &codes))
        .ok_or(Because::NotBuiltYet(NOT_WRITTEN))?;
    Ok(MadeSymbolic {
        program,
        reached,
        given,
    })
}

/// The `/Flags` a descriptor stating `stated` states once the font is made symbolic: the Symbolic
/// flag set and the Nonsymbolic one cleared, which Table 121 makes exclusive, and every other flag
/// as it was.
pub(super) const fn flags(stated: i64) -> Object {
    Object::Integer((stated & !NONSYMBOLIC) | SYMBOLIC)
}

/// The descriptor the proof reads: `descriptor` with no program key, its `/Flags` made symbolic,
/// and `gains` over it.
pub(super) fn proof_descriptor(
    document: &Document,
    descriptor: &Dictionary,
    gains: &Dictionary,
) -> Dictionary {
    let mut written = descriptor.clone();
    for key in ["FontFile", "FontFile2", "FontFile3"] {
        written.remove(key);
    }
    for (key, value) in gains.iter() {
        written.insert(key.clone(), value.clone());
    }
    let stated = document
        .get_key(&written, "Flags")
        .as_integer()
        .unwrap_or(0);
    written.insert(Name::new(&b"Flags"[..]), flags(stated));
    written
}

/// The `/ToUnicode` `CMap` the file's own encoding derives, where the file states none and some
/// code's name resolves to a character; `None` otherwise.
pub(super) fn to_unicode(
    document: &Document,
    used: &SelectedFont,
    font: &LoadedFont,
) -> Option<Object> {
    if !document.get_key(&used.dict, "ToUnicode").is_null() {
        return None;
    }
    let mapping: BTreeMap<u8, String> = (0..=u8::MAX)
        .filter_map(|byte| {
            let name = font.selected_glyph_name(Code::single_byte(byte))?;
            if name.is_empty() || name == ".notdef" {
                return None;
            }
            super::super::to_unicode::characters_for(name).map(|text| (byte, text))
        })
        .collect();
    (!mapping.is_empty()).then(|| super::super::to_unicode::cmap_stream(&mapping))
}

/// The font dictionary's entries for the symbolic font: `/Encoding` removed (a null value, which
/// §7.3.9 makes no entry) and, where the file states none, the `/ToUnicode` its encoding derives,
/// written once [`proved`] holds the font as written to the source.
///
/// # Errors
///
/// [`proved`]'s refusal, or no object number free for the `CMap`.
pub(super) fn written(
    document: &Document,
    (used, font): (&SelectedFont, &LoadedFont),
    made: &MadeSymbolic,
    (descriptor, row): (&Dictionary, Table124),
    entries: &mut Dictionary,
    (spare, substitutes): (&mut Spare, &mut Substitutes),
) -> Result<(), Because> {
    entries.insert(Name::new(&b"Encoding"[..]), Object::Null);
    proved(document, used, font, made, (descriptor, row, entries))?;
    if let Some(cmap) = to_unicode(document, used, font) {
        let at = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
        substitutes.written.insert(at, cmap);
        entries.insert(Name::new(&b"ToUnicode"[..]), Object::Reference(at));
    }
    Ok(())
}

/// Writes the font as it will be written — no `/Encoding`, the symbolic descriptor, the restated
/// program — and holds it to the source: every code on the glyph it reached or was given, none by
/// a mapping of the reader's choosing, each shown code at the advance it had, each code given an
/// empty glyph drawing nothing, and the font drawn from its own program.
pub(super) fn proved(
    document: &Document,
    used: &SelectedFont,
    font: &LoadedFont,
    made: &MadeSymbolic,
    (written_descriptor, row, entries): (&Dictionary, Table124, &Dictionary),
) -> Result<(), Because> {
    let mut stated = used.clone();
    stated.dict.remove("Encoding");
    let mut written_entries = Dictionary::new();
    for (key, value) in entries.iter() {
        if !matches!(value, Object::Null) {
            written_entries.insert(key.clone(), value.clone());
        }
    }
    let converted = super::described::loaded_as_written(
        document,
        &stated,
        &written_entries,
        written_descriptor,
        row,
        &made.program,
    )
    .map_err(|_| Because::NotBuiltYet(NOT_PROVED))?;
    if converted.substitute_program().is_some()
        || !truetype_cmap::on_the_same_glyphs(&made.reached, &made.given, &converted)
    {
        return Err(Because::NotBuiltYet(NOT_PROVED));
    }
    for text in used.shown.keys() {
        for code in font.decode(text) {
            if super::disagrees(font.advance(code), converted.advance(code)) {
                return Err(Because::NotBuiltYet(NOT_PROVED));
            }
        }
    }
    let draws_nothing = made.given.keys().all(|byte| {
        converted
            .outline(Code::single_byte(*byte))
            .is_none_or(|outline| outline.commands().is_empty())
    });
    if draws_nothing {
        Ok(())
    } else {
        Err(Because::NotBuiltYet(NOT_PROVED))
    }
}

/// The face could not be given its empty glyphs and its symbolic `cmap`.
const NOT_WRITTEN: &str = "this document renders a font it does not embed and shows a code its own \
     encoding names no glyph for, or names .notdef; the face this program ships for it could not be \
     given a glyph that draws nothing at the width the font dictionary states and the cmap of the \
     symbolic font ISO 32000-2 \u{a7}9.6.5.4 describes, so the code would still reach .notdef, which \
     ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid";

/// The font as written did not read back as the source drew it.
const NOT_PROVED: &str = "this document renders a font it does not embed and shows a code its own \
     encoding names no glyph for, or names .notdef; made the symbolic font ISO 32000-2 \
     \u{a7}9.6.5.4 describes with a glyph that draws nothing for that code, the font read back by \
     this program did not reach every code's glyph at its width as the source drew it, so it is \
     not written";
