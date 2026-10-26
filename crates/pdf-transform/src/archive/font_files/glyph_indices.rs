//! A `CIDFontType2` whose `/CIDToGIDMap` is a stream, over a name-keyed CFF, re-keyed so that its
//! CIDs are the glyph indices they selected (ADR 2342).
//!
//! Table 124 admits a CFF-flavoured OpenType program only under `CIDFontType0`, and there §9.7.4.2
//! uses a CID of a program whose Top DICT has no `CIDFont` operators as the glyph index outright:
//!
//! > The CIDs shall be used directly as GID values, and the glyph procedure shall be retrieved
//! > using the CharStrings INDEX
//!
//! Under the `CIDFontType2` the producer wrote, Table 115's stream map sends CID *c* to the glyph
//! index stored in bytes 2*c* and 2*c* + 1. So refiling the font keeps every glyph only where the
//! CIDs themselves change to the glyph indices, and three statements are keyed by CID: the `Type0`
//! font's encoding, which §9.7.6.2 makes the map from codes to CIDs; the descendant's `/W`, which
//! §9.7.4.3 keys by CID; and `/W2`, keyed the same way. The first two are restated here and the
//! third refuses:
//!
//! - **The encoding.** Each `Type0` font naming the descendant states `Identity-H` or `Identity-V`,
//!   under which code *c* is CID *c* (Table 116); it is given an embedded `CMap` (§9.7.5.3) whose
//!   one codespace is the same two bytes and whose `cidrange` entries send code *c* to the glyph
//!   index this reader drew it with. A code that drew glyph 0 is left to the `notdefrange` that
//!   sends every code to CID 0, since §9.7.6.2 looks a code no character mapping covers up in the
//!   notdef mappings.
//! - **The widths.** `/W` states, for each glyph index a code reaches, the advance this reader
//!   gave that code; two codes reaching one glyph at two advances have no re-keying that keeps
//!   both, and refuse.
//!
//! **The glyphs and advances are this reader's**, read through `pdf_font::LoadedFont` for every
//! code of the two-byte codespace: the re-keyed font is built from what the page was drawn with,
//! not from a second reading of the map and the widths. `pdf-transform`'s archive tests load the
//! output again and compare the two for every code.
//!
//! What refuses, each by a sentence: an encoding other than the two identities; a `/W2`; a
//! descriptor's `/CIDSet`, whose bits are CIDs; a `Type0` font written inside another object,
//! whose `/Encoding` this rewrite cannot reach; a code reaching no glyph, or a glyph past the
//! program's last; a font this reader substituted or did not load.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use pdf_font::LoadedFont;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use super::super::COMPRESSION_LEVEL;
use super::super::decision::Because;
use super::super::prepare::Spare;

/// Why an encoding other than the two identity `CMap`s keeps the refusal.
const NOT_AN_IDENTITY: &str = "a CIDFontType2 over a CFF-flavoured program, whose CIDToGIDMap \
     is a stream, is the descendant of a Type0 font whose Encoding is not Identity-H or \
     Identity-V; re-keying a CMap that maps codes to other CIDs is not built";

/// Why a `/W2` keeps the refusal.
const VERTICAL_METRICS: &str = "a CIDFontType2 over a CFF-flavoured program, whose CIDToGIDMap \
     is a stream, states W2, whose vertical metrics are keyed by CID (section 9.7.4.3); \
     re-keying them to glyph indices is not built";

/// Why a `/CIDSet` keeps the refusal.
const CID_SET: &str = "a CIDFontType2 over a CFF-flavoured program, whose CIDToGIDMap is a \
     stream, has a font descriptor stating CIDSet, whose bits are the CIDs that would change; \
     re-keying it to glyph indices is not built";

/// Why a `Type0` font written inside another object keeps the refusal.
const PARENT_NOT_AN_OBJECT: &str = "a CIDFontType2 over a CFF-flavoured program, whose \
     CIDToGIDMap is a stream, is the descendant of a Type0 font written inside another object, \
     whose Encoding this rewrite does not reach, so that font would show other glyphs";

/// Why a font this reader did not load from its own program keeps the refusal.
const NOT_LOADED: &str = "a CIDFontType2 over a CFF-flavoured program, whose CIDToGIDMap is a \
     stream, did not load from its own program in this reader, so there are no glyphs it drew \
     to re-key its CIDs to";

/// Why a code reaching no glyph keeps the refusal.
const NO_GLYPH: &str = "a CIDFontType2 over a CFF-flavoured program, whose CIDToGIDMap is a \
     stream, has a code that reaches no glyph of the program, or a glyph past its last, and no \
     CID under CIDFontType0 selects that";

/// Why two codes reaching one glyph at two advances keep the refusal.
const TWO_WIDTHS: &str = "a CIDFontType2 over a CFF-flavoured program, whose CIDToGIDMap is a \
     stream, gives two codes that reach one glyph two different widths, and under CIDFontType0 \
     one glyph index is one CID with one width (section 9.7.4.3)";

/// Why an advance the widths array cannot state exactly keeps the refusal.
const WIDTH_NOT_WRITTEN: &str = "a CIDFontType2 over a CFF-flavoured program, whose \
     CIDToGIDMap is a stream, has an advance this conversion cannot write as a number that reads \
     back to the same width";

/// Why a descendant whose character collection cannot be restated keeps the refusal.
const NO_SYSTEM_INFO: &str = "a CIDFontType2 over a CFF-flavoured program, whose CIDToGIDMap \
     is a stream, states no CIDSystemInfo whose Registry and Ordering this conversion can write \
     into the CMap that would re-key its encoding, and ISO 19005-2 section 6.2.11.3.1 holds that \
     CMap to the font's collection";

/// Why no object number was free.
const NO_NUMBER: &str = "no object number was free for the CMap a re-keyed composite font's \
     encoding names";

/// Why the `CMap` could not be compressed.
const NOT_ENCODED: &str = "the CMap a re-keyed composite font's encoding names could not be \
     compressed into a stream";

/// TN 5014 caps one `begincidrange` block at a hundred entries.
const RANGES_PER_BLOCK: usize = 100;

/// How deep the search for a `Type0` font written inside another object looks.
const MAX_DEPTH: usize = 32;

/// What re-keying one descendant writes.
#[derive(Debug, Default)]
pub(in crate::archive) struct Rekeyed {
    /// The descendant's `/W`, keyed by glyph index.
    pub(in crate::archive) widths: Vec<Object>,
    /// For each `Type0` font naming the descendant, the `CMap` stream its `/Encoding` names now.
    pub(in crate::archive) encodings: BTreeMap<ObjectId, ObjectId>,
    /// The `CMap` streams, one per writing mode.
    pub(in crate::archive) written: BTreeMap<ObjectId, Object>,
}

/// The re-keying of the `CIDFontType2` `descendant`, whose program holds `glyphs` glyphs, or why
/// there is none.
pub(super) fn rekey(
    document: &Document,
    descendant: ObjectId,
    glyphs: u16,
    spare: &mut Spare,
) -> Result<Rekeyed, Because> {
    let Object::Dictionary(dict) = document.get(descendant) else {
        return Err(Because::NotBuiltYet(NOT_LOADED));
    };
    if dict.get("W2").is_some() {
        return Err(Because::NotBuiltYet(VERTICAL_METRICS));
    }
    let descriptor = document.get_key(&dict, "FontDescriptor");
    if descriptor
        .as_dict()
        .is_some_and(|descriptor| descriptor.get("CIDSet").is_some())
    {
        return Err(Because::NotBuiltYet(CID_SET));
    }
    let parents = parents(document, descendant)?;
    let Some((first, _)) = parents.first() else {
        return Err(Because::NotBuiltYet(PARENT_NOT_AN_OBJECT));
    };
    let Object::Dictionary(type0) = document.get(*first) else {
        return Err(Because::NotBuiltYet(PARENT_NOT_AN_OBJECT));
    };
    // Under either identity code *c* is CID *c*, so one parent's glyphs are every parent's.
    let drawn = drawn(document, &type0, glyphs)?;
    let widths = widths(&drawn)?;
    let info = system_info(document, &dict).ok_or(Because::NotBuiltYet(NO_SYSTEM_INFO))?;
    let mut out = Rekeyed {
        widths,
        ..Rekeyed::default()
    };
    let mut by_mode: BTreeMap<i64, ObjectId> = BTreeMap::new();
    for (parent, mode) in parents {
        let at = if let Some(at) = by_mode.get(&mode) {
            *at
        } else {
            let at = spare
                .take(document)
                .ok_or(Because::NotBuiltYet(NO_NUMBER))?;
            let name = format!(
                "GlyphIndices-{}-{}-{}",
                descendant.number,
                descendant.generation,
                if mode == 0 { "H" } else { "V" }
            );
            let stream =
                cmap_stream(&name, &info, mode, &drawn).ok_or(Because::NotBuiltYet(NOT_ENCODED))?;
            out.written.insert(at, stream);
            by_mode.insert(mode, at);
            at
        };
        out.encodings.insert(parent, at);
    }
    Ok(out)
}

/// Every `Type0` font naming `descendant`, with the writing mode its identity encoding states.
///
/// Every object is searched, and a `Type0` font found anywhere but as an object of its own
/// refuses: leaving one font's encoding as it was would show other glyphs through it.
fn parents(document: &Document, descendant: ObjectId) -> Result<Vec<(ObjectId, i64)>, Because> {
    let mut out = Vec::new();
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let object = document.get(id);
        if let Object::Dictionary(dict) = &object
            && names_descendant(document, dict, descendant)
        {
            let mode = match document.get_key(dict, "Encoding").as_name() {
                Some(name) if name.as_bytes() == b"Identity-H" => 0,
                Some(name) if name.as_bytes() == b"Identity-V" => 1,
                _ => return Err(Because::NotBuiltYet(NOT_AN_IDENTITY)),
            };
            out.push((id, mode));
            for (_, value) in dict.iter() {
                if nested_parent(document, value, descendant, 0) {
                    return Err(Because::NotBuiltYet(PARENT_NOT_AN_OBJECT));
                }
            }
        } else if nested_parent(document, &object, descendant, 0) {
            return Err(Because::NotBuiltYet(PARENT_NOT_AN_OBJECT));
        }
    }
    Ok(out)
}

/// Whether `dict` is a font whose `/DescendantFonts` names `descendant`.
fn names_descendant(document: &Document, dict: &Dictionary, descendant: ObjectId) -> bool {
    document
        .get_key(dict, "DescendantFonts")
        .as_array()
        .is_some_and(|fonts| {
            fonts
                .iter()
                .any(|font| font.as_reference() == Some(descendant))
        })
}

/// Whether a dictionary written inside `object` names `descendant` as a descendant font.
fn nested_parent(document: &Document, object: &Object, descendant: ObjectId, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let deeper = depth.saturating_add(1);
    match object {
        Object::Dictionary(dict) => {
            names_descendant(document, dict, descendant)
                || dict
                    .iter()
                    .any(|(_, value)| nested_parent(document, value, descendant, deeper))
        }
        Object::Stream(stream) => stream
            .dict
            .iter()
            .any(|(_, value)| nested_parent(document, value, descendant, deeper)),
        Object::Array(items) => items
            .iter()
            .any(|item| nested_parent(document, item, descendant, deeper)),
        _ => false,
    }
}

/// For every two-byte code, the glyph index and the advance this reader drew it with.
fn drawn(document: &Document, type0: &Dictionary, glyphs: u16) -> Result<Vec<(u16, f32)>, Because> {
    let font = LoadedFont::load(document, type0, "re-keyed")
        .map_err(|_| Because::NotBuiltYet(NOT_LOADED))?;
    if font.is_substituted() || font.is_metrics_only() {
        return Err(Because::NotBuiltYet(NOT_LOADED));
    }
    let mut out = Vec::with_capacity(0x1_0000);
    for value in 0..=u16::MAX {
        let codes = font.decode(&value.to_be_bytes());
        let [code] = codes.as_slice() else {
            return Err(Because::NotBuiltYet(NO_GLYPH));
        };
        let glyph = font
            .glyph_index(*code)
            .filter(|glyph| *glyph < glyphs)
            .ok_or(Because::NotBuiltYet(NO_GLYPH))?;
        out.push((glyph, font.advance(*code)));
    }
    Ok(out)
}

/// The `/W` array keyed by glyph index: one width per glyph a code reaches, in runs of
/// consecutive glyphs.
fn widths(drawn: &[(u16, f32)]) -> Result<Vec<Object>, Because> {
    let mut by_glyph: BTreeMap<u16, f32> = BTreeMap::new();
    for &(glyph, advance) in drawn {
        match by_glyph.get(&glyph) {
            Some(held) if held.to_bits() != advance.to_bits() => {
                return Err(Because::NotBuiltYet(TWO_WIDTHS));
            }
            Some(_) => {}
            None => {
                by_glyph.insert(glyph, advance);
            }
        }
    }
    let mut out = Vec::new();
    let mut run: Option<(u16, Vec<Object>)> = None;
    let mut previous: Option<u16> = None;
    for (&glyph, &advance) in &by_glyph {
        let width = width(advance).ok_or(Because::NotBuiltYet(WIDTH_NOT_WRITTEN))?;
        let follows = previous.and_then(|previous| previous.checked_add(1)) == Some(glyph);
        match run.as_mut() {
            Some((_, list)) if follows => list.push(width),
            _ => {
                if let Some((first, list)) = run.take() {
                    out.push(Object::Integer(i64::from(first)));
                    out.push(Object::Array(list));
                }
                run = Some((glyph, vec![width]));
            }
        }
        previous = Some(glyph);
    }
    if let Some((first, list)) = run {
        out.push(Object::Integer(i64::from(first)));
        out.push(Object::Array(list));
    }
    Ok(out)
}

/// An advance in text space as the glyph-space number `/W` states, where one reads back to it.
///
/// The reader takes a width as an `f32` and divides it by a thousand (§9.7.4.3's glyph space), so
/// the number written is the one that gives the same `f32` back: an integer where one does, and
/// otherwise the shortest decimal of up to six places that does.
fn width(advance: f32) -> Option<Object> {
    let reads_back = |number: f64| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the reader narrows a width to f32 the same way, which is what is compared"
        )]
        let narrowed = number as f32;
        (narrowed / 1000.0).to_bits() == advance.to_bits()
    };
    let exact = f64::from(advance) * 1000.0;
    let whole = exact.round();
    if reads_back(whole) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a whole number that reads back as an f32 width is far inside i64"
        )]
        return Some(Object::Integer(whole as i64));
    }
    (1..=6).find_map(|places| {
        let scale = 10_f64.powi(places);
        let number = (exact * scale).round() / scale;
        reads_back(number).then_some(Object::Real(number))
    })
}

/// The descendant's `/CIDSystemInfo`, which the `CMap` states as its own: ISO 19005-2 section
/// 6.2.11.3.1 holds a `CMap` that is no identity to its font's `Registry` and `Ordering`.
///
/// Each string is taken where it is printable ASCII with no parenthesis or backslash, so that the
/// `CMap` file can write it as a PostScript string unchanged.
fn system_info(document: &Document, descendant: &Dictionary) -> Option<(Vec<u8>, Vec<u8>, i64)> {
    let info = document.get_key(descendant, "CIDSystemInfo");
    let info = info.as_dict()?;
    let text = |key: &str| {
        document
            .get_key(info, key)
            .as_string()
            .filter(|text| {
                text.iter()
                    .all(|byte| byte.is_ascii_graphic() && !b"()\\".contains(byte))
            })
            .map(<[u8]>::to_vec)
    };
    Some((
        text("Registry")?,
        text("Ordering")?,
        document.get_key(info, "Supplement").as_integer()?,
    ))
}

/// The embedded `CMap` §9.7.5.3 asks for, mapping every code to the glyph it drew.
fn cmap_stream(
    name: &str,
    (registry, ordering, supplement): &(Vec<u8>, Vec<u8>, i64),
    mode: i64,
    drawn: &[(u16, f32)],
) -> Option<Object> {
    let ranges = ranges(drawn);
    let registry_text = String::from_utf8_lossy(registry);
    let ordering_text = String::from_utf8_lossy(ordering);
    let mut program = String::new();
    let _ = write!(
        program,
        "%!PS-Adobe-3.0 Resource-CMap\n%%DocumentNeededResources: ProcSet (CIDInit)\n\
         %%IncludeResource: ProcSet (CIDInit)\n%%BeginResource: CMap ({name})\n%%EndComments\n\
         /CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo 3 dict dup begin\n/Registry ({registry_text}) def\n\
         /Ordering ({ordering_text}) def\n/Supplement {supplement} def\nend def\n\
         /CMapName /{name} def\n/CMapVersion 1.000 def\n/CMapType 1 def\n/WMode {mode} def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
    );
    for block in ranges.chunks(RANGES_PER_BLOCK) {
        let _ = writeln!(program, "{} begincidrange", block.len());
        for (low, high, glyph) in block {
            let _ = writeln!(program, "<{low:04X}> <{high:04X}> {glyph}");
        }
        program.push_str("endcidrange\n");
    }
    program.push_str(
        "1 beginnotdefrange\n<0000> <FFFF> 0\nendnotdefrange\nendcmap\n\
         CMapName currentdict /CMap defineresource pop\nend\nend\n%%EndResource\n%%EOF\n",
    );

    let mut info = Dictionary::new();
    info.insert(
        Name::new(&b"Registry"[..]),
        Object::String(registry.clone().into()),
    );
    info.insert(
        Name::new(&b"Ordering"[..]),
        Object::String(ordering.clone().into()),
    );
    info.insert(Name::new(&b"Supplement"[..]), Object::Integer(*supplement));
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"CMap"[..])),
    );
    dict.insert(
        Name::new(&b"CMapName"[..]),
        Object::Name(Name::new(name.as_bytes())),
    );
    dict.insert(Name::new(&b"CIDSystemInfo"[..]), Object::Dictionary(info));
    dict.insert(Name::new(&b"WMode"[..]), Object::Integer(mode));
    let encoded = flate_encode(program.as_bytes(), COMPRESSION_LEVEL)?;
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}

/// The codes that drew a glyph other than 0, as `cidrange` entries: runs of consecutive codes
/// drawing consecutive glyphs.
fn ranges(drawn: &[(u16, f32)]) -> Vec<(u16, u16, u16)> {
    let mut out: Vec<(u16, u16, u16)> = Vec::new();
    for (code, &(glyph, _)) in (0..=u16::MAX).zip(drawn) {
        if glyph == 0 {
            continue;
        }
        if let Some((low, high, first)) = out.last_mut()
            && high.checked_add(1) == Some(code)
            && code
                .checked_sub(*low)
                .and_then(|step| first.checked_add(step))
                == Some(glyph)
        {
            *high = code;
            continue;
        }
        out.push((code, code, glyph));
    }
    out
}
