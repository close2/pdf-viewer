//! A simple `TrueType` dictionary over a CFF-flavoured OpenType program, written as the `Type1`
//! font that selects the same glyphs by name (ADR 3078).
//!
//! Table 124 admits such a program under `Type1` and `CIDFontType0` and under no `TrueType` row,
//! and §9.9.1 changes how a code reaches a glyph with the dictionary:
//!
//! > For Type 1 fonts using "CFF " tables, the process shall be as described in 9.6.5.2,
//! > "Encodings for Type 1 fonts".
//!
//! Under `TrueType` the producer's file was read by §9.6.5.4's `cmap` route; under `Type1` a code is
//! its glyph name, looked up in the CFF charset. So the retyped font keeps every glyph only where
//! each code's name is the name of the glyph it drew, and §9.6.5.1's Table 112 makes a
//! `/Differences` array the place a name is given to a code. Every code that drew a glyph is given
//! that glyph's charset name, `.notdef` for glyph 0 among them; a code that drew none is left to
//! the base encoding, and the comparison below decides whether it still reaches nothing.
//!
//! **The base is the first of four that keeps every such code on nothing**: none, so that Table
//! 112 makes the program's built-in encoding the base, then `WinAnsiEncoding`,
//! `MacRomanEncoding` and `MacExpertEncoding`, the three Table 109 permits. In this reader an
//! empty slot of a named base reaches no glyph, and a slot naming a glyph the charset lacks
//! reaches none either, so a named base often holds where a program's own encoding fills the
//! codes the producer left unmapped. The standard encoding, which leaves 0 to 31 and 127 to 160
//! empty, is not among them, because Table D.1 says of it (ADR 3753), in ISO 32000-2 §D.1:
//!
//! > PDF processors shall not have a predefined encoding named StandardEncoding.
//!
//! Nothing else in the dictionary changes: `/Widths`, `/FirstChar`, `/LastChar` and the
//! descriptor's `/MissingWidth` mean the same under both subtypes (Table 109), and `/ToUnicode` is
//! the producer's.
//!
//! **The glyphs and advances are this reader's**, read through `pdf_font::LoadedFont` for each of
//! the 256 codes, and the retyped dictionary is loaded again over the same program and compared
//! code by code: a glyph or an advance that differs refuses.
//!
//! What refuses, each by a sentence: a font this reader substituted or did not load, or loaded
//! with metrics only; a glyph the charset gives no name of its own; a retyped font that would
//! draw a code otherwise under every base, a code that drew nothing and that each base names
//! among them.

use pdf_font::LoadedFont;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::decision::Because;

/// Why a font this reader did not load from its own program keeps the refusal.
const NOT_LOADED: &str = "a simple TrueType font over a CFF-flavoured OpenType program did not \
     load from its own program in this reader, so there are no glyphs it drew to name in a Type1 \
     font's encoding";

/// Why a glyph with no name of its own keeps the refusal.
const NO_NAME: &str = "a simple TrueType font over a CFF-flavoured OpenType program draws a code \
     with a glyph the program's CFF charset gives no name of its own, so no Type1 encoding names \
     the glyph it drew (ISO 32000-2 section 9.6.5.2)";

/// Why a retyped font that draws a code otherwise keeps the refusal.
const DRAWN_OTHERWISE: &str = "a simple TrueType font over a CFF-flavoured OpenType program, \
     written as the Type1 font its program is admitted under, would draw a code with another \
     glyph or advance";

/// The `/Encoding` the `TrueType` font `font` is written with as a `Type1` font over `program`, an
/// sfnt whose `CFF ` table is keyed by name, or why there is none.
pub(super) fn encoding(
    document: &Document,
    font: ObjectId,
    program: &[u8],
) -> Result<Object, Because> {
    let Object::Dictionary(dict) = document.get(font) else {
        return Err(Because::NotBuiltYet(NOT_LOADED));
    };
    let source = loaded(document, &dict)?;
    let names = pdf_font::embedding::wrapped_cff_glyph_names(program)
        .ok_or(Because::NotBuiltYet(NO_NAME))?;
    // A run of `/Differences` opens at each code that follows one drawing no glyph.
    let mut differences = Vec::with_capacity(257);
    let mut in_run = false;
    for byte in 0..=u8::MAX {
        let Some(glyph) = glyph_of(&source, byte) else {
            in_run = false;
            continue;
        };
        let name = names.get(&glyph).ok_or(Because::NotBuiltYet(NO_NAME))?;
        if !in_run {
            differences.push(Object::Integer(i64::from(byte)));
            in_run = true;
        }
        differences.push(Object::Name(Name::new(name.as_bytes())));
    }
    for base in BASES {
        let mut encoding = Dictionary::new();
        encoding.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"Encoding"[..])),
        );
        if let Some(base) = base {
            encoding.insert(
                Name::new(&b"BaseEncoding"[..]),
                Object::Name(Name::new(base)),
            );
        }
        encoding.insert(
            Name::new(&b"Differences"[..]),
            Object::Array(differences.clone()),
        );
        let encoding = Object::Dictionary(encoding);
        let mut retyped = dict.clone();
        as_type1(&mut retyped, encoding.clone());
        let written = loaded(document, &retyped)?;
        let drawn_alike = (0..=u8::MAX).all(|byte| {
            glyph_of(&source, byte) == glyph_of(&written, byte)
                && advance_of(&source, byte).to_bits() == advance_of(&written, byte).to_bits()
        });
        if drawn_alike {
            return Ok(encoding);
        }
    }
    Err(Because::NotBuiltYet(DRAWN_OTHERWISE))
}

/// The base encodings a retyped font is tried under, in order: none, the program's built-in
/// encoding standing, then the three names Table 109 permits.
const BASES: [Option<&[u8]>; 4] = [
    None,
    Some(b"WinAnsiEncoding"),
    Some(b"MacRomanEncoding"),
    Some(b"MacExpertEncoding"),
];

/// Writes `dict` as the `Type1` font over the same program, its codes named by `encoding`.
pub(in crate::archive) fn as_type1(dict: &mut Dictionary, encoding: Object) {
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"Type1"[..])),
    );
    dict.insert(Name::new(&b"Encoding"[..]), encoding);
}

/// The font `dict` loaded from its own program, or the refusal.
fn loaded(document: &Document, dict: &Dictionary) -> Result<LoadedFont, Because> {
    let font = LoadedFont::load(document, dict, "retyped")
        .map_err(|_| Because::NotBuiltYet(NOT_LOADED))?;
    if font.is_substituted() || font.is_metrics_only() {
        return Err(Because::NotBuiltYet(NOT_LOADED));
    }
    Ok(font)
}

/// The glyph `font` draws the one-byte code `byte` with.
fn glyph_of(font: &LoadedFont, byte: u8) -> Option<u16> {
    match font.decode(&[byte]).as_slice() {
        [code] => font.glyph_index(*code),
        _ => None,
    }
}

/// The advance `font` gives the one-byte code `byte`.
fn advance_of(font: &LoadedFont, byte: u8) -> f32 {
    match font.decode(&[byte]).as_slice() {
        [code] => font.advance(*code),
        _ => f32::NAN,
    }
}
