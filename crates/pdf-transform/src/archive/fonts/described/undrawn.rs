//! A simple font drawn only in text rendering mode 3, given the descriptor and widths ISO 32000-2
//! Table 109 requires of it, with no program (`doc/adr/2688`).
//!
//! ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 do not require the program
//! of a font used only in mode 3, which §9.3.6's Table 104 calls "Neither fill nor stroke text
//! (invisible).", so the embedding route that writes all four of Table 109's entries with a face
//! is never taken for it. The four are still required of the dictionary: in a PDF 2.0 file by
//! Table 109 of every simple font, and by ISO 32000-1 Table 111 of every one but the fourteen.
//!
//! **Every value written is the one this reader already uses.** `/FirstChar`, `/LastChar` and
//! `/Widths` are the advances the glyphs are laid out by ([`super::state_laid_out_widths`]): for
//! one of the fourteen the published metrics §9.6.2.1 makes a processor provide, and for any other
//! name the metrics of the face its name resembles. The descriptor's Table 120 entries are those
//! of the face this program ships for the font, the one the reader substitutes for it, measured
//! as the embedding route measures it (`Measured`): `/FontBBox`, `/ItalicAngle`, `/Ascent`,
//! `/Descent`, `/CapHeight` and `/StemV` from its program, and `/Flags` from Table 121 read off the
//! same face. §9.8.1 says what a descriptor is for when the program is not there: its metrics
//! "enable a PDF processor to synthesise a substitute font or select a similar font when the font
//! program is unavailable", and the shipped face is the substitute this reader selects, the same
//! on every machine. `/FontName` is the dictionary's own `/BaseFont`, which Table 120 makes it.
//!
//! **That is proved, not trusted**: the dictionary as it will be written is loaded beside the
//! source, and every one of the 256 codes must advance by the same amount, bit for bit, and give
//! the same text, since an invisible layer is there for its text and its positions.

use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object};

use super::super::super::decision::Because;
use super::Measured;

/// The three widths entries and the descriptor a font drawn only in mode 3 is given.
#[derive(Debug)]
pub(in crate::archive) struct Undrawn {
    /// `/FirstChar`, `/LastChar` and `/Widths`, written into the font dictionary.
    pub(in crate::archive) entries: Dictionary,
    /// The Table 120 descriptor, written as an object of its own (Table 109 makes the entry an
    /// indirect reference) and named by the dictionary's `/FontDescriptor`.
    pub(in crate::archive) descriptor: Dictionary,
}

/// Describes the simple font `dict`, used only in rendering mode 3 and stating no descriptor, by
/// the face and the advances this reader lays it out by, or says why not.
pub(in crate::archive) fn undrawn(
    document: &Document,
    dict: &Dictionary,
    name: &str,
) -> Result<Undrawn, Because> {
    let subtype = document.get_key(dict, "Subtype");
    if !subtype
        .as_name()
        .is_some_and(|subtype| matches!(subtype.as_bytes(), b"Type1" | b"MMType1" | b"TrueType"))
    {
        return Err(Because::NotBuiltYet(NOT_A_SIMPLE_FONT));
    }
    let font_name = document
        .get_key(dict, "BaseFont")
        .as_name()
        .map(|name| name.as_bytes().to_vec())
        .ok_or(Because::NotBuiltYet(NO_FONT_NAME))?;
    let face = pdf_font::standard::shipped_face(document, dict, name)
        .map_err(|_| Because::NotBuiltYet(NO_SHIPPED_FACE))?;
    let measured = Measured::of(&face).ok_or(Because::NotBuiltYet(FACE_NOT_MEASURED))?;
    let source =
        LoadedFont::load(document, dict, name).map_err(|_| Because::NotBuiltYet(NOT_READ))?;
    let mut entries = Dictionary::new();
    super::state_laid_out_widths(&mut entries, &source);
    let descriptor = measured.descriptor(&font_name, &face, None);

    // The output names the descriptor by reference; the loader resolves a reference and a direct
    // dictionary alike, so the copy holds it directly and is read against the source's objects.
    let mut written = dict.clone();
    for (key, value) in entries.iter() {
        written.insert(key.clone(), value.clone());
    }
    written.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Dictionary(descriptor.clone()),
    );
    let converted =
        LoadedFont::load(document, &written, name).map_err(|_| Because::NotBuiltYet(NOT_READ))?;
    for code in (0..=u8::MAX).map(Code::single_byte) {
        // Bit for bit: each number is this reader's reading of one statement, and an extracted
        // word's position is made of them.
        if converted.advance(code).to_bits() != source.advance(code).to_bits() {
            return Err(Because::NotBuiltYet(LAID_OUT_DIFFERENTLY));
        }
        let (mut before, mut after) = (String::new(), String::new());
        if source.text(code, &mut before) != converted.text(code, &mut after) || before != after {
            return Err(Because::NotBuiltYet(READ_DIFFERENTLY));
        }
    }
    Ok(Undrawn {
        entries,
        descriptor,
    })
}

/// Why a font of another kind is not described here.
const NOT_A_SIMPLE_FONT: &str = "a font used only in text rendering mode 3 states no \
     FontDescriptor and its Subtype is none of Type1, MMType1 and TrueType, the simple fonts ISO \
     32000-2 Table 109 and \u{a7}9.6.3 give a descriptor of Table 120's entries";

/// Why a font with no `/BaseFont` name is not described.
const NO_FONT_NAME: &str = "a font used only in text rendering mode 3 states no FontDescriptor \
     and no BaseFont name, and ISO 32000-2 Table 120 makes the descriptor's FontName the font \
     dictionary's BaseFont, so there is no name to state; with no program embedded there is no \
     program's name to supply either";

/// Why a font no shipped face answers is not described.
const NO_SHIPPED_FACE: &str = "a font used only in text rendering mode 3 states no \
     FontDescriptor, and no face this program ships answers it, so there is no substitute whose \
     metrics ISO 32000-2 \u{a7}9.8.1's descriptor could state";

/// Why a face whose program cannot be measured is not described.
const FACE_NOT_MEASURED: &str = "a font used only in text rendering mode 3 states no \
     FontDescriptor, and the face this program ships for it could not be read for the bounding \
     box, italic angle and extents ISO 32000-2 \u{a7}9.8.1's Table 120 requires of a descriptor";

/// Why a font this reader cannot load is not described.
const NOT_READ: &str = "a font used only in text rendering mode 3 states no FontDescriptor, and \
     this reader cannot load it, or cannot load it with the descriptor this conversion would \
     write, so there are no advances to state";

/// Why a description that would move an invisible glyph is not written.
const LAID_OUT_DIFFERENTLY: &str = "a font used only in text rendering mode 3 states no \
     FontDescriptor, and with the descriptor and widths this conversion would write this reader \
     advances at least one code differently, which would move the text an invisible layer is \
     there to place";

/// Why a description that would change the text read is not written.
const READ_DIFFERENTLY: &str = "a font used only in text rendering mode 3 states no \
     FontDescriptor, and with the descriptor and widths this conversion would write this reader \
     reads at least one code as different text, which would change the text an invisible layer \
     is there to carry";
