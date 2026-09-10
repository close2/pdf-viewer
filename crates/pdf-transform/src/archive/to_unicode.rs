//! The `/ToUnicode` `CMap` a font's own encoding derives, for the fonts a Level U file needs it on.
//!
//! # What may be derived, and what may not
//!
//! ISO 19005-2 section 6.2.11.7.2 requires a `/ToUnicode` `CMap` on every font a Level A or Level U
//! file uses, with four exemptions, and requires the values it states to be usable ones.
//! `doc/pdf-a-conversion-limits.md` section 4.3 draws the line this file is built on: what can be
//! *derived* is a `CMap` written from a font's own encoding where the glyph names are the standard
//! ones; what cannot is a guess, and a code whose meaning is not derivable is a refusal rather
//! than an invention. `doc/questions/A48`'s line governs — state an interpretation the standard
//! defines, never fill in an absence.
//!
//! So the derivation is exactly **§9.10.2's second method**, applied to the codes a content stream
//! actually showed:
//!
//! > If the font is a simple font and the glyph selection algorithm (see 9.6.5, "Character
//! > encoding") uses a glyph name, that name can be looked up in the Adobe Glyph List and Adobe
//! > Glyph List for New Fonts to obtain the corresponding Unicode value.
//!
//! The name comes from `pdf_font`'s own reading of that mapping, which is the same table the
//! renderer selected the glyph with, and it is turned into characters by the Adobe Glyph List
//! Specification's algorithm — so a ligature name and a variant suffix resolve, and a private
//! name resolves to nothing at all. A font with one code this cannot answer for produces no `CMap`:
//! a half-written table would say that the codes it omits mean nothing, which is worse than the
//! absent entry the clause is complaining about.
//!
//! # Simple fonts only, and that is the clause rather than a limit
//!
//! §9.10.2's second method turns on a *glyph name*, and only a simple font selects its glyph by
//! one. A Type 0 font's codes reach a CID, which §9.7.4.2 makes an index into the glyphs of the
//! font that defined it and which therefore says nothing about any character; the clause's third
//! method is for the registered collections, and ISO 19005-2 section 6.2.11.7.2 exempts four of
//! those outright. So a composite font outside the exemption is a refusal here, by the standard's
//! own account of what its codes are.
//!
//! # What the producer already said is kept
//!
//! Where a font's own `/ToUnicode` states a usable value for a code, that value is carried into
//! the derived `CMap` unchanged. It is the producer's statement about their own file and this
//! converter has no better evidence than the person who made the document; what is replaced is
//! only the codes the clause calls placeholders — zero, U+FEFF and U+FFFE — and the codes the
//! table never held.

use core::fmt::Write as _;
use std::collections::BTreeMap;

use pdf_archive::survey::Survey;
use pdf_font::LoadedFont;
use pdf_font::encoding::{self, SymbolicEncoding};
use pdf_font::tounicode::ToUnicode;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::{Document, serialize::flate_encode};

use super::COMPRESSION_LEVEL;
use super::prepare::Spare;

/// The three values ISO 19005-2 section 6.2.11.7.2 names as placeholders rather than mappings.
///
/// Zero because the clause asks for values greater than it, and the other two because a
/// `/ToUnicode` destination is UTF-16BE (§9.10.3) — so either of them is a producer having
/// written the byte-order mark where the character belonged.
const UNUSABLE: [char; 3] = ['\u{0}', '\u{FEFF}', '\u{FFFE}'];

/// The `/ToUnicode` `CMaps` this conversion is in a position to write.
pub(super) struct DerivedMaps {
    /// The stream each font's `/ToUnicode` is to name, keyed by the font object.
    pub(super) at: BTreeMap<ObjectId, ObjectId>,
    /// The streams themselves, in the source's numbering, for the walk to add.
    pub(super) written: BTreeMap<ObjectId, Object>,
}

/// Derives a `/ToUnicode` `CMap` for each of the fonts the validator named.
///
/// `fonts` is the set of font objects the two Unicode requirements failed at, taken from the
/// validator's own findings — **not** from a walk of this converter's own. The clause has four
/// exemptions and reading them is `pdf_archive`'s job; a converter that decided for itself which
/// fonts need a `CMap` would be re-deriving the reading, and would write one into a font the
/// standard excuses.
///
/// # Errors
///
/// The reason no `CMap` could be derived, which becomes the reason the requirements it would have
/// answered are refused with. **All or nothing across the document**: a file whose second font is
/// underivable is refused rather than half-converted, because the requirement is stated of every
/// font and a file meeting it for one of two has not met it.
pub(super) fn derive(
    document: &Document,
    fonts: &[ObjectId],
    spare: &mut Spare,
) -> Result<DerivedMaps, &'static str> {
    if fonts.is_empty() {
        return Err(NO_FONT_NAMED);
    }
    let survey = Survey::of(document);
    let mut maps = DerivedMaps {
        at: BTreeMap::new(),
        written: BTreeMap::new(),
    };
    for id in fonts {
        let Some(used) = survey
            .fonts()
            .find(|used| used.id == Some(*id) && used.shown_complete)
        else {
            // Either no content stream selected this font — in which case there is no set of
            // referenced codes to write a CMap over — or the survey's own budget stopped before
            // its strings were read, in which case the set is a prefix and a CMap built on it
            // would be silently short.
            return Err(CODES_NOT_KNOWN);
        };
        let mapping = derive_one(document, used)?;
        let at = spare.take(document).ok_or(NO_SPARE_OBJECT)?;
        maps.written.insert(at, cmap_stream(&mapping));
        maps.at.insert(*id, at);
    }
    Ok(maps)
}

/// One font's code-to-text table, or the reason it has none.
fn derive_one(
    document: &Document,
    used: &pdf_archive::survey::SelectedFont,
) -> Result<BTreeMap<u8, String>, &'static str> {
    let font = LoadedFont::load(document, &used.dict, &used.name).map_err(|_| FONT_NOT_READ)?;
    if font.is_substituted() {
        // A substituted font answers for the substitute's glyph names rather than for the
        // file's, so what it would derive is a statement about a font this document does not
        // carry — the same guard `pdf_archive` applies when it rules the exemption out.
        return Err(FONT_NOT_READ);
    }
    let stated = producer_map(document, &used.dict);
    let mut out = BTreeMap::new();
    for text in used.shown.keys() {
        for code in font.decode(text) {
            let Ok(byte) = u8::try_from(code.value()) else {
                // §9.10.3 writes a simple font's codes as one byte, and a code outside that
                // range is a composite font's — whose codes carry no glyph name to derive from.
                return Err(NOT_A_SIMPLE_FONT);
            };
            if let Some(kept) = stated
                .as_ref()
                .and_then(|stated| usable(stated, code.value()))
            {
                out.insert(byte, kept);
                continue;
            }
            let name = font.selected_glyph_name(code).ok_or(NO_GLYPH_NAME)?;
            out.insert(byte, characters_for(name).ok_or(NAME_NOT_LISTED)?);
        }
    }
    if out.is_empty() {
        return Err(CODES_NOT_KNOWN);
    }
    Ok(out)
}

/// The `/ToUnicode` `CMap` the producer wrote, where the font states one this reader can parse.
fn producer_map(document: &Document, font: &Dictionary) -> Option<ToUnicode> {
    let Object::Stream(stream) = document.get_key(font, "ToUnicode") else {
        return None;
    };
    let data = document.decoded_stream_data(&stream)?;
    Some(ToUnicode::parse(&data))
}

/// What the producer's own `CMap` says one code means, where that is a usable value.
///
/// `None` for a code the table does not hold, for one it maps to nothing, and for one it maps to
/// a value ISO 19005-2 section 6.2.11.7.2 names as a placeholder. Those three are exactly the
/// cases the derivation is for; every other value is the producer's and is kept.
fn usable(stated: &ToUnicode, code: u32) -> Option<String> {
    let mut text = String::new();
    if !stated.append(code, &mut text) || text.is_empty() {
        return None;
    }
    text.chars()
        .all(|character| !UNUSABLE.contains(&character))
        .then_some(text)
}

/// The characters a glyph name stands for, by the two lists the exemption's second bullet names.
///
/// The Adobe Glyph List first, by its own specification's algorithm — so `f_f_i` and `oacute.sc`
/// resolve — and then Annex D.6's `ZapfDingbats` set, whose `a1` and `a192` that list does not
/// hold. `Symbol`'s own names are Adobe Glyph List names, which is why only one of the two
/// symbolic sets appears here.
fn characters_for(name: &str) -> Option<String> {
    encoding::text_for(name).or_else(|| {
        SymbolicEncoding::ZapfDingbats
            .character_for(name)
            .map(|character| character.to_string())
    })
}

/// One `/ToUnicode` `CMap` stream, in the shape §9.10.3 requires of one.
///
/// The clause states three differences from an ordinary `CMap` and this writes all three: a
/// codespace range consistent with the font's encoding, which for a simple font "shall be one
/// byte long"; `beginbfchar`/`endbfchar` mappings "to Unicode character sequences expressed in
/// UTF-16BE encoding"; and nothing in the stream dictionary but what §9.7.5's Table 118 admits.
/// The surrounding `/CIDInit` procedure, the `/CMapType 2` and the `Adobe-Identity-UCS` system
/// information are the shape §9.10.3's own EXAMPLE 2 prints.
///
/// Written uncompressed only where it is small enough that a reader with a text editor can see
/// what was asserted about their file; the same judgement §14.3.2's metadata packet gets, and for
/// the same reason.
fn cmap_stream(mapping: &BTreeMap<u8, String>) -> Object {
    let table = mapping
        .iter()
        .collect::<Vec<_>>()
        .chunks(BFCHAR_LIMIT)
        .fold(String::new(), |mut table, chunk| {
            let entries = chunk
                .iter()
                .fold(String::new(), |mut entries, (code, text)| {
                    appended(
                        &mut entries,
                        format_args!("<{code:02X}> <{}>\n", utf16be(text)),
                    );
                    entries
                });
            appended(
                &mut table,
                format_args!("{} beginbfchar\n{entries}endbfchar\n", chunk.len()),
            );
            table
        });
    let body = format!(
        "/CIDInit /ProcSet findresource begin\n\
         12 dict begin\n\
         begincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n\
         /CMapType 2 def\n\
         1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
         {table}\
         endcmap\n\
         CMapName currentdict /CMap defineresource pop\n\
         end\nend\n"
    );
    stream_object(body.into_bytes())
}

/// How many mappings go between one `beginbfchar` and its `endbfchar`.
///
/// Adobe Technical Note #5014, which §9.10.3 sends the `CMap` syntax to, limits the operator to a
/// hundred at a time. This tree does not hold that note (`CLAUDE.md` principle 5), so the number
/// is not read from it: it is the smallest chunking that no reader has been observed to refuse,
/// and writing 256 codes in three runs costs nothing.
const BFCHAR_LIMIT: usize = 100;

/// The stream object a `CMap` goes in, compressed where that is worth doing.
fn stream_object(data: Vec<u8>) -> Object {
    let (dict, data) = match flate_encode(&data, COMPRESSION_LEVEL) {
        Some(encoded) if encoded.len() < data.len() => {
            let mut dict = Dictionary::new();
            dict.insert(
                Name::new(&b"Filter"[..]),
                Object::Name(Name::new(&b"FlateDecode"[..])),
            );
            (dict, encoded)
        }
        _ => (Dictionary::new(), data),
    };
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}

/// One string of characters as §9.10.3's destination value: UTF-16BE, in hexadecimal.
fn utf16be(text: &str) -> String {
    text.encode_utf16().fold(String::new(), |mut out, unit| {
        appended(&mut out, format_args!("{unit:04X}"));
        out
    })
}

/// Appends one formatted piece to a string, discarding a `Result` that cannot be an error.
///
/// `core::fmt::Write for String` returns `Ok` on every call — it grows the buffer, or the
/// allocator aborts the process — so there is nothing to propagate. Discarding it here, once,
/// keeps every call site above free of a `Result` that says nothing (`CLAUDE.md` principle 1's
/// provably-infallible case, named where it is taken).
fn appended(out: &mut String, piece: core::fmt::Arguments<'_>) {
    let _ = out.write_fmt(piece);
}

/// Why no `CMap` was derived because the validator named no font.
///
/// Unreachable through [`super::prepare::Prepared`], which asks for a derivation only where one
/// of the two Unicode requirements failed and every such failure names a font object. It says so
/// rather than borrowing a reason that is not the actual one.
const NO_FONT_NAMED: &str = "no font this document failed a Unicode requirement at could be \
     named, so there is nothing to derive a ToUnicode CMap for";

/// Why a font's referenced codes are not known well enough to write a `CMap` over.
const CODES_NOT_KNOWN: &str = "ISO 19005-2 section 6.2.11.7.2 asks for a CMap over the codes a \
     font's glyphs are referenced by, and this document's content streams did not yield that set \
     for one of its fonts — either nothing selected the font, or reading its strings ran into \
     this program's own budget. A CMap written over a prefix of the codes would say that the \
     rest mean nothing";

/// Why a font this reader cannot load derives nothing.
const FONT_NOT_READ: &str = "deriving a ToUnicode CMap means reading the encoding the font \
     itself selects glyphs by, and this font is one this program could not load or had to \
     substitute for — in which case the names would be the substitute's rather than this file's";

/// Why a composite font derives nothing.
const NOT_A_SIMPLE_FONT: &str = "this font's codes are wider than one byte, so they select a CID \
     rather than a glyph name — and ISO 32000-2 \u{a7}9.7.4.2 makes a CID an index into the \
     glyphs of the font that defined it, which says nothing about any character. ISO 19005-2 \
     section 6.2.11.7.2 exempts the four registered collections outright; outside them there is \
     no evidence in the file to derive from";

/// Why a code with no glyph name derives nothing.
const NO_GLYPH_NAME: &str = "ISO 32000-2 \u{a7}9.10.2's second method maps a code to a glyph name \
     and the name to a Unicode value, and one of this font's referenced codes selects its glyph \
     without a name — a symbolic TrueType font reaching its cmap by code is the usual shape. \
     What that code means is not stated anywhere in this file, and manufacturing it would \
     manufacture the evidence Level U exists to require";

/// Why a code whose glyph name is nobody's derives nothing.
const NAME_NOT_LISTED: &str = "one of this font's referenced codes selects a glyph whose name is \
     in neither the Adobe Glyph List nor Annex D's symbolic sets — a subsetter's private label \
     for a glyph it kept. The name says which glyph was drawn and not which character it stands \
     for, so no ToUnicode entry follows from it. PDF/A-2b asks nothing of text extraction and \
     PDF/A-4 states the same rule as a recommendation";

/// Why a document with no free object number gets no `CMap`.
const NO_SPARE_OBJECT: &str = "this document uses every object number a conversion could give to \
     a ToUnicode CMap stream";
