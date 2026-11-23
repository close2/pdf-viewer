//! A `CIDFontType0`'s name-keyed CFF program, selected through a `/CIDToGIDMap`, written again as
//! the CID-keyed program whose charset selects the same glyphs (ADR 3446).
//!
//! ISO 32000-2 §9.7.4.2 uses a name-keyed CFF's CIDs "directly as GID values", and Table 115 gives
//! `/CIDToGIDMap` to a Type 2 `CIDFont`; this reader draws such a font through the map
//! (`pdf_archive`'s `fonts/cidfonttype0-glyphs-selected-by-cid` records it). The program is written
//! by [`pdf_font::cff::cid_keyed_by_map`], which gives each CID the map sent to a glyph that glyph's
//! charstring at a glyph index of its own and the CID in its charset, and proves every CID draws
//! the outline and the width it drew. The new program is filed as §9.9's Table 124 files a
//! CID-keyed CFF under a `CIDFontType0` — `/FontFile3` of `/Subtype /CIDFontType0C` — rather than
//! back inside the `OpenType` wrapper a source may have held it in, because that wrapper's
//! `maxp`, `hmtx` and `cmap` count and name the old glyphs; and the map leaves the `CIDFont`,
//! since the charset now states what it stated.
//!
//! Refused by name: a program shared by two `CIDFont`s or not an object of its own, a map or a
//! program this reader does not decode, a `/CIDSystemInfo` stating no registry or ordering, and a
//! program the writer refuses.

use std::collections::{BTreeMap, BTreeSet};

use pdf_font::embed::SystemInfo;
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId, text_string};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const ROW: &str = "fonts/cidfonttype0-glyphs-selected-by-cid";

/// Why a program is not written again.
const NOT_REKEYED: &str = "a CIDFontType0 over a name-keyed CFF program states a CIDToGIDMap, \
     which ISO 32000-2 \u{a7}9.7.4.2 does not use, and this conversion writes such a program again \
     as a CID-keyed one whose charset selects what the map selected only where the program and \
     the map decode, the program serves that one CIDFont, its CIDSystemInfo names a registry and \
     an ordering, and every CID draws the outline and the width it drew; this font is not that";

/// The programs written again and the `CIDFont`s whose map goes.
#[derive(Debug, Default)]
pub(super) struct Rekeyed {
    /// Each new `/FontFile3` stream, whole, keyed by the object it goes into.
    pub(super) programs: BTreeMap<ObjectId, Object>,
    /// The `CIDFont` dictionaries that lose `/CIDToGIDMap`.
    pub(super) cidfonts: BTreeSet<ObjectId>,
}

/// Every `CIDFont` [`ROW`] failed at, its program written again, or why one cannot be.
///
/// # Errors
///
/// [`NOT_REKEYED`] where any failing `CIDFont` is outside what the module comment lists; a
/// requirement half answered is a requirement refused.
pub(super) fn rekeyed(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Rekeyed, Because> {
    let refused = Because::NotBuiltYet(NOT_REKEYED);
    let (named, elsewhere) = super::sites::objects_named(input, &[ROW])?;
    if elsewhere {
        return Err(refused);
    }
    let mut rekeyed = Rekeyed::default();
    for id in named {
        let Object::Dictionary(cidfont) = document.get(id) else {
            return Err(refused);
        };
        let (at, stream) = program_of(document, &cidfont).ok_or(refused)?;
        if rekeyed.programs.contains_key(&at) {
            return Err(refused);
        }
        let Object::Stream(map) = document.get_key(&cidfont, "CIDToGIDMap") else {
            return Err(refused);
        };
        let map = document.decoded_stream_data(&map).ok_or(refused)?;
        let map: Vec<u16> = map
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect();
        let system = system_of(document, &cidfont).ok_or(refused)?;
        let data = document.decoded_stream_data(&stream).ok_or(refused)?;
        let cff = pdf_font::cff::name_keyed_cff(&data).ok_or(refused)?;
        let program = pdf_font::cff::cid_keyed_by_map(&cff, &map, &system).map_err(|_| refused)?;
        let mut dict = stream.dict.clone();
        dict.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"CIDFontType0C"[..])),
        );
        let written = super::remedies::restated_stream(&dict, &program).ok_or(refused)?;
        rekeyed.programs.insert(at, written);
        rekeyed.cidfonts.insert(id);
    }
    // A program two `CIDFont`s share would be written for one map and drawn through the other.
    let mut shared = BTreeMap::<ObjectId, usize>::new();
    for number in document.xref().object_numbers() {
        if let Object::Dictionary(font) = document.get(document.defined_id(number))
            && let Some((at, _)) = program_of(document, &font)
            && rekeyed.programs.contains_key(&at)
        {
            let count = shared.entry(at).or_default();
            *count = count.saturating_add(1);
        }
    }
    if shared.values().any(|count| *count > 1) {
        return Err(refused);
    }
    Ok(rekeyed)
}

/// The `/FontFile3` stream a `CIDFont`'s descriptor names, with its object number.
fn program_of(
    document: &Document,
    cidfont: &Dictionary,
) -> Option<(ObjectId, std::sync::Arc<pdf_syntax::Stream>)> {
    let Object::Dictionary(descriptor) = document.get_key(cidfont, "FontDescriptor") else {
        return None;
    };
    let at = descriptor.get("FontFile3")?.as_reference()?;
    let Object::Stream(stream) = document.get(at) else {
        return None;
    };
    Some((at, stream))
}

/// The `CIDFont`'s `/CIDSystemInfo` as the `ROS` a CID-keyed CFF states: the registry and the
/// ordering as their text (§7.9.2.2), which is what a CFF string holds.
fn system_of(document: &Document, cidfont: &Dictionary) -> Option<SystemInfo> {
    let Object::Dictionary(info) = document.get_key(cidfont, "CIDSystemInfo") else {
        return None;
    };
    let text = |key: &str| match document.get_key(&info, key) {
        Object::String(bytes) => Some(text_string(&bytes).into_bytes()),
        _ => None,
    };
    Some(SystemInfo {
        registry: text("Registry")?,
        ordering: text("Ordering")?,
        supplement: document.get_key(&info, "Supplement").as_integer()?,
    })
}
