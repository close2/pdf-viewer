//! The embedded `CMap` a composite font's encoding is restated as, and the character collection
//! it states.
//!
//! ISO 32000-2 §9.7.5.3 has a `CMap` that is not predefined embedded as a stream whose data
//! follows Adobe Technical Note #5014 and whose dictionary carries Table 118's entries, the
//! `/CMapName`, `/CIDSystemInfo` and `/WMode` each the same as the file states. Two constructions
//! write one: a `CIDFontType2` over a CFF-flavoured program re-keyed to its glyph indices
//! (`doc/adr/2342`), and a `Type0` font whose shown CID 0 is sent to a fresh CID
//! (`doc/adr/2622`). Both restate an identity encoding, so the one codespace written is the
//! two-byte range `Identity-H` and `Identity-V` state (Table 116).

use std::fmt::Write as _;

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, Stream};
use pdf_syntax::serialize::flate_encode;

use super::super::COMPRESSION_LEVEL;

/// Adobe Technical Note #5014 caps one `begincidrange` block at a hundred entries.
const RANGES_PER_BLOCK: usize = 100;

/// A character collection's `Registry`, `Ordering` and `Supplement` (§9.7.3).
pub(in crate::archive) type SystemInfo = (Vec<u8>, Vec<u8>, i64);

/// The descendant's `/CIDSystemInfo`, which the `CMap` states as its own: ISO 19005-2 section
/// 6.2.11.3.1 holds a `CMap` that is no identity to its font's `Registry` and `Ordering`.
///
/// Each string is taken where it is printable ASCII with no parenthesis or backslash, so that the
/// `CMap` file can write it as a PostScript string unchanged.
pub(in crate::archive) fn system_info(
    document: &Document,
    descendant: &Dictionary,
) -> Option<SystemInfo> {
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

/// The embedded `CMap` §9.7.5.3 asks for, over the one two-byte codespace `<0000>` to `<FFFF>`,
/// sending each code of a `ranges` entry `(low, high, first)` to `first` plus its distance from
/// `low`, and every other code to CID 0 by the notdef mapping §9.7.5.4 item d asks for, under the
/// character collection `info` and the writing mode `mode`, which Table 118 has the stream's
/// dictionary state as the file does.
///
/// The entries are written in `begincidrange` blocks of at most [`RANGES_PER_BLOCK`].
pub(in crate::archive) fn cid_cmap_stream(
    name: &str,
    (registry, ordering, supplement): &SystemInfo,
    mode: i64,
    ranges: &[(u16, u16, u16)],
) -> Option<Object> {
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
