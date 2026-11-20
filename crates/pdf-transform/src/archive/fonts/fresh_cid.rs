//! A `Type0` font whose shown code selects CID 0, with that code sent to a fresh CID through an
//! embedded `CMap`.
//!
//! ISO 32000-2 §9.7.4.2:
//!
//! > Every CIDFont shall contain a glyph description for CID 0, which is analogous to the .notdef
//! > character name in simple fonts
//!
//! so a code whose CID is 0 references the `CIDFont`'s `.notdef` whatever glyph draws it, which
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator
//! to do. The code is the content stream's and the content stream is the producer's, so what
//! changes is the map §9.7.6.2 reads the code through, the `Type0` font's `/Encoding`: the
//! predefined `Identity-H` is restated as an embedded `CMap` (§9.7.5.3) that sends every code to
//! the CID `Identity-H` gave it (Table 116: the two-byte code is the CID) except code `<0000>`,
//! which goes to a CID past every CID the font uses. The descendant gives that CID the glyph CID 0
//! drew — the program's own, or a copy of `.notdef`'s outline where CID 0 drew that — and `/W`
//! gives it CID 0's width (§9.7.4.3), so every mark stays where it was (`doc/adr/2622`).
//!
//! **Only `Identity-H`.** Another predefined `CMap` would be restated with its own codespace and
//! every mapping it holds, and `Identity-V` with the vertical metrics `/W2` keys by CID; neither
//! is written here, and such a font keeps the refusal.
//!
//! **What text extraction reads stays.** §9.10.2 maps a code to Unicode first through the font's
//! `ToUnicode` `CMap`, which is keyed by code and so does not move; where there is none, its third
//! method maps the *CID* through the collection's own Unicode `CMap` for a font whose descendant
//! uses one of the Adobe CJK collections it names, and there a fresh CID would read as a character
//! CID 0 never was. Such a font without a `ToUnicode` keeps the refusal.
//!
//! **An invalid code under an embedded `CMap`.** A code no codespace range of the font's own
//! embedded `CMap` matches is what §9.7.6.3 calls invalid, and it is drawn as CID 0 by the
//! clause's fallback; no map keyed by CID can give it another glyph. Its own `CMap` is restated
//! instead with a codespace range holding that one code (one range per code, each the length the
//! fallback consumed, so no string of the file is split differently — the proof reads every
//! string again) and a `cidchar` sending it to the fresh CID ([`planned_for_invalid`],
//! `doc/adr/3237`). That `CMap` is no longer the one its name registers, so its `/CMapName` is
//! restated with it. For a collection §9.10.2's third method maps by CID, the fresh CID is placed
//! past every CID the collection's own Unicode `CMap` maps, so that method reads nothing for it,
//! as it read nothing for the invalid code.
//!
//! The embedded `CMap` states the descendant's own `/CIDSystemInfo`, which ISO 19005-2 section
//! 6.2.11.3.1 asks a `CMap` other than an identity to share with its `CIDFont`, and writing mode
//! 0 in both its dictionary and its file, which section 6.2.11.3.3 asks to agree.

use std::fmt::Write as _;

use pdf_font::Code;
use pdf_font::cmap::CMap;
use pdf_font::tounicode::Mapping;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::embedded_cmap::{self, RANGES_PER_BLOCK, SystemInfo};

/// The collections whose CIDs §9.10.2's third method maps to Unicode by the collection's own
/// `CMap`, under the `Adobe` registry.
const MAPPED_ORDERINGS: [&[u8]; 5] = [b"GB1", b"CNS1", b"Japan1", b"Korea1", b"KR"];

/// The most `/W` entries read for the width of CID 0 and the highest CID it states: a `/W` that
/// states every 16-bit CID one at a time holds 131 072 of them.
const MOST_WIDTH_ENTRIES: usize = 1 << 17;

/// The width §9.7.4.3 gives a CID that neither `/W` nor `/DW` states.
const DEFAULT_WIDTH: i64 = 1000;

/// What a font's shown CID 0 is sent to, ready to write.
#[derive(Debug)]
pub(in crate::archive) struct FreshCid {
    /// The CID code `<0000>` selects from now on.
    pub(in crate::archive) cid: u16,
    /// The embedded `CMap` the `Type0` fonts that show CID 0 name as their `/Encoding`.
    pub(in crate::archive) cmap: Object,
    /// The descendant's `/W`, every entry it stated and then the fresh CID at CID 0's width.
    pub(in crate::archive) widths: Object,
}

/// The fresh CID the `Type0` fonts `type0s` over the descendant `descendant` (at
/// `descendant_at`) send their code `<0000>` to, past `highest_used` — the highest CID the caller
/// found shown or held by the program — and past every CID `/W` states.
///
/// `None` where a font's `/Encoding` is not the name `Identity-H`, where a font has no `ToUnicode`
/// and the descendant's collection is one §9.10.2 maps by CID, where the descendant's
/// `/CIDSystemInfo` cannot be written into a `CMap` file, where `/W` or `/DW` is malformed or `/W`
/// passes [`MOST_WIDTH_ENTRIES`], or where no 16-bit CID is left.
pub(in crate::archive) fn planned(
    document: &Document,
    type0s: &[&Dictionary],
    (descendant_at, descendant): (ObjectId, &Dictionary),
    highest_used: u32,
) -> Option<FreshCid> {
    let info = embedded_cmap::system_info(document, descendant)?;
    if type0s.is_empty() || !type0s.iter().all(|font| admits(document, font, &info)) {
        return None;
    }
    let (entries, zero_width, highest_stated) = widths_read(document, descendant)?;
    let cid = u16::try_from(highest_used.max(highest_stated).checked_add(1)?).ok()?;
    let name = format!(
        "FreshCid-{}-{}-H",
        descendant_at.number, descendant_at.generation
    );
    let cmap = embedded_cmap::cid_cmap_stream(&name, &info, 0, &identity_but_zero(cid))?;
    let mut widths = entries;
    widths.push(Object::Integer(i64::from(cid)));
    widths.push(Object::Array(vec![zero_width]));
    Some(FreshCid {
        cid,
        cmap,
        widths: Object::Array(widths),
    })
}

/// The fresh CID the `Type0` font `type0` over the descendant `descendant` (at `descendant_at`)
/// sends each code of `invalid` to, every one a code §9.7.6.3 calls invalid under the font's own
/// embedded `CMap`, past `highest_used`, every CID `/W` states and the `CMap` selects, and every
/// CID the collection's Unicode `CMap` maps where §9.10.2's third method reads the collection
/// (`doc/adr/3237`).
///
/// `None` where `invalid` is empty or holds a code that matched a range, where the `/Encoding` is
/// not a stream of its own, names another `CMap` to build on, states a writing mode other than 0,
/// or does not define its `/CMapName` as its dictionary names it, where the descendant's
/// `/CIDSystemInfo` cannot be written, where the collection's Unicode `CMap` is one this binary
/// does not carry, where `/W` or `/DW` is malformed, or where no 16-bit CID is left.
pub(in crate::archive) fn planned_for_invalid(
    document: &Document,
    type0: &Dictionary,
    (descendant_at, descendant): (ObjectId, &Dictionary),
    highest_used: u32,
    invalid: &[Code],
) -> Option<FreshCid> {
    if invalid.is_empty() || invalid.iter().any(|code| code.in_codespace()) {
        return None;
    }
    let (registry, ordering, _) = embedded_cmap::system_info(document, descendant)?;
    let Object::Stream(stream) = document.get(type0.get("Encoding")?.as_reference()?) else {
        return None;
    };
    if stream.dict.get("UseCMap").is_some() {
        return None;
    }
    let file = document.decoded_stream_data(&stream)?;
    let parsed = CMap::parse(&file, None);
    if parsed.references_another() || parsed.wmode() != 0 || !parsed.has_codespace() {
        return None;
    }
    let collection_highest =
        if registry.as_slice() == b"Adobe" && MAPPED_ORDERINGS.contains(&ordering.as_slice()) {
            let unicode = pdf_font::predefined::cid_to_unicode(
                &String::from_utf8_lossy(&registry),
                &String::from_utf8_lossy(&ordering),
            )?;
            unicode
                .mappings()
                .map(|mapping| match mapping {
                    Mapping::Single { code, .. } => code,
                    Mapping::Span { high, .. } => high,
                })
                .max()
                .unwrap_or(0)
        } else {
            0
        };
    let (entries, zero_width, highest_stated) = widths_read(document, descendant)?;
    let highest = highest_used
        .max(highest_stated)
        .max(parsed.greatest_selector().unwrap_or(0))
        .max(collection_highest);
    let cid = u16::try_from(highest.checked_add(1)?).ok()?;
    let stated_name = document.get_key(&stream.dict, "CMapName");
    let stated_name = stated_name.as_name()?.as_bytes().to_vec();
    let name = [
        format!(
            "FreshCid-{}-{}-",
            descendant_at.number, descendant_at.generation
        )
        .as_bytes(),
        &stated_name,
    ]
    .concat();
    let restated = invalid_codes_sent(&file, (&stated_name, &name), invalid, cid)?;
    let mut dict = stream.dict.clone();
    dict.insert(
        Name::new(&b"CMapName"[..]),
        Object::Name(Name::new(&name[..])),
    );
    let cmap = super::program_stream(Some(&dict), &restated, false)?;
    let mut widths = entries;
    widths.push(Object::Integer(i64::from(cid)));
    widths.push(Object::Array(vec![zero_width]));
    Some(FreshCid {
        cid,
        cmap,
        widths: Object::Array(widths),
    })
}

/// The `CMap` file `file` with each code of `codes` given a codespace range of its own and a
/// `cidchar` to `fresh`, written just after its first `endcodespacerange`, and its `/CMapName`
/// definition restated from `stated` to `name`.
///
/// `None` where the file has no `endcodespacerange` or no `/CMapName` definition naming `stated`.
fn invalid_codes_sent(
    file: &[u8],
    (stated, name): (&[u8], &[u8]),
    codes: &[Code],
    fresh: u16,
) -> Option<Vec<u8>> {
    let mut spelled: Vec<String> = codes
        .iter()
        .map(|code| {
            let digits = usize::from(code.length()) * 2;
            format!("<{:0digits$x}>", code.value())
        })
        .collect();
    spelled.sort();
    spelled.dedup();
    let mut inserted = String::new();
    for block in spelled.chunks(RANGES_PER_BLOCK) {
        let _ = writeln!(inserted, "\n{} begincodespacerange", block.len());
        for code in block {
            let _ = writeln!(inserted, "{code} {code}");
        }
        inserted.push_str("endcodespacerange\n");
        let _ = writeln!(inserted, "{} begincidchar", block.len());
        for code in block {
            let _ = writeln!(inserted, "{code} {fresh}");
        }
        inserted.push_str("endcidchar");
    }
    let close = b"endcodespacerange";
    let at = file
        .windows(close.len())
        .position(|window| window == close)?
        .checked_add(close.len())?;
    let (head, tail) = file.split_at(at);
    let mut restated = [head, inserted.as_bytes(), tail].concat();
    renamed(&mut restated, stated, name)?;
    Some(restated)
}

/// `file` with the name its `/CMapName` key defines restated from `stated` to `name`, the
/// definition being the key, any white space, and the name (§7.3.5's solidus and the bytes);
/// `None` where no such definition names `stated`.
fn renamed(file: &mut Vec<u8>, stated: &[u8], name: &[u8]) -> Option<()> {
    let key = b"/CMapName";
    let mut from = 0;
    while let Some(found) = file
        .get(from..)?
        .windows(key.len())
        .position(|window| window == key)
    {
        let after_key = from.checked_add(found)?.checked_add(key.len())?;
        let spaces = file
            .get(after_key..)?
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
        let solidus = after_key.checked_add(spaces)?;
        let first = solidus.checked_add(1)?;
        let end = first.checked_add(stated.len())?;
        let delimited = file
            .get(end)
            .is_none_or(|byte| byte.is_ascii_whitespace() || b"/[]<>(){}%".contains(byte));
        if file.get(solidus) == Some(&b'/') && file.get(first..end) == Some(stated) && delimited {
            file.splice(first..end, name.iter().copied());
            return Some(());
        }
        from = after_key;
    }
    None
}

/// Whether one `Type0` font's encoding can be restated: it names `Identity-H`, and §9.10.2 reads
/// its codes through a `ToUnicode` or reads nothing by CID for its collection.
fn admits(document: &Document, font: &Dictionary, (registry, ordering, _): &SystemInfo) -> bool {
    let identity = document
        .get_key(font, "Encoding")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"Identity-H");
    let to_unicode = matches!(document.get_key(font, "ToUnicode"), Object::Stream(_));
    let mapped_by_cid =
        registry.as_slice() == b"Adobe" && MAPPED_ORDERINGS.contains(&ordering.as_slice());
    identity && (to_unicode || !mapped_by_cid)
}

/// The `cidrange` entries of `Identity-H` with code `<0000>` sent to `fresh`: one range for each
/// first byte, as `Identity-H` states them, the first starting at `<0001>`.
fn identity_but_zero(fresh: u16) -> Vec<(u16, u16, u16)> {
    let mut ranges = vec![(0, 0, fresh), (1, 0x00FF, 1)];
    ranges.extend((1..=0xFF_u16).map(|high| (high << 8, (high << 8) | 0xFF, high << 8)));
    ranges
}

/// The descendant's `/W` entries as stated, the width it gives CID 0 (from `/W`, else `/DW`, else
/// §9.7.4.3's default of 1000), and the highest CID `/W` states, 0 where it states none.
///
/// `None` where `/W` is not an array or holds an entry §9.7.4.3's two forms do not, a CID is
/// negative, or the entries pass [`MOST_WIDTH_ENTRIES`].
fn widths_read(document: &Document, descendant: &Dictionary) -> Option<(Vec<Object>, Object, u32)> {
    let entries: Vec<Object> = match descendant.get("W") {
        None => Vec::new(),
        Some(value) => document.resolve(value).as_array()?.to_vec(),
    };
    let (zero_width, highest) = widths_of(&entries, descendant.get("DW"), |value| {
        document.resolve(value)
    })?;
    Some((entries, zero_width, highest))
}

/// [`widths_read`] over the entries of `/W` and the `/DW` stated, each value read through
/// `resolve`.
fn widths_of(
    entries: &[Object],
    default_width: Option<&Object>,
    resolve: impl Fn(&Object) -> Object,
) -> Option<(Object, u32)> {
    if entries.len() > MOST_WIDTH_ENTRIES {
        return None;
    }
    let mut zero_width: Option<Object> = None;
    let mut highest = 0_u32;
    let mut at = 0_usize;
    while let Some(first) = entries.get(at) {
        let first = u32::try_from(resolve(first).as_integer()?).ok()?;
        match entries.get(at.checked_add(1)?).map(&resolve)? {
            Object::Array(each) => {
                if each.len() > MOST_WIDTH_ENTRIES {
                    return None;
                }
                let count = u32::try_from(each.len()).ok()?;
                if let Some(span) = count.checked_sub(1) {
                    highest = highest.max(first.checked_add(span)?);
                }
                if first == 0
                    && zero_width.is_none()
                    && let Some(width) = each.first()
                {
                    zero_width = Some(number(&resolve, width)?);
                }
                at = at.checked_add(2)?;
            }
            last => {
                let last = u32::try_from(last.as_integer()?).ok()?;
                let width = number(&resolve, entries.get(at.checked_add(2)?)?)?;
                highest = highest.max(last);
                if first == 0 && zero_width.is_none() {
                    zero_width = Some(width);
                }
                at = at.checked_add(3)?;
            }
        }
    }
    let zero_width = match zero_width {
        Some(width) => width,
        None => match default_width {
            Some(value) => number(&resolve, value)?,
            None => Object::Integer(DEFAULT_WIDTH),
        },
    };
    Some((zero_width, highest))
}

/// A width as the number the file states, resolved; `None` where it is not one.
fn number(resolve: &impl Fn(&Object) -> Object, value: &Object) -> Option<Object> {
    let resolved = resolve(value);
    resolved.as_number()?;
    Some(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `CMap` file of two codespace ranges, one and three bytes, sending `<41>` to CID 36.
    const TWO_RANGES: &[u8] = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
        /CMapName /Test-UTF8-H def\n2 begincodespacerange\n<00> <7f>\n<e08080> <efbfbf>\n\
        endcodespacerange\n1 begincidchar\n<41> 36\nendcidchar\nendcmap\n\
        CMapName currentdict /CMap defineresource pop end end\n";

    #[test]
    fn an_invalid_code_is_given_a_range_of_its_own_and_sent_to_the_fresh_cid() {
        // ISO 32000-2 §9.7.6.3: `e0 e0 e0` matches no range, and the best partial match is the
        // three-byte range's first byte, so three bytes are consumed and the code is invalid.
        let before = CMap::parse(TWO_RANGES, None);
        let invalid = before.next_code(&[0xe0, 0xe0, 0xe0]);
        assert!(!invalid.in_codespace());
        assert_eq!(invalid.length(), 3);
        let restated = invalid_codes_sent(
            TWO_RANGES,
            (b"Test-UTF8-H", b"F-Test-UTF8-H"),
            &[invalid],
            700,
        )
        .expect("the file is restated");
        let after = CMap::parse(&restated, None);
        let code = after.next_code(&[0xe0, 0xe0, 0xe0]);
        assert!(code.in_codespace());
        assert_eq!((code.value(), code.length()), (invalid.value(), 3));
        assert_eq!(after.cid(code), Some(700));
        // Every other code is split and mapped as before.
        for bytes in [
            &[0x41][..],
            &[0xe0, 0x80, 0x80],
            &[0xe0, 0xe0, 0xe1],
            &[0xe1, 0xe0],
        ] {
            let (was, is) = (before.next_code(bytes), after.next_code(bytes));
            assert_eq!(
                (
                    was.value(),
                    was.length(),
                    was.in_codespace(),
                    before.cid(was)
                ),
                (is.value(), is.length(), is.in_codespace(), after.cid(is)),
                "{bytes:x?}"
            );
        }
        let text = String::from_utf8_lossy(&restated);
        assert!(text.contains("/CMapName /F-Test-UTF8-H def"), "{text}");
        assert!(!text.contains("/Test-UTF8-H def"), "{text}");
    }

    #[test]
    fn a_file_that_does_not_define_its_stated_name_is_not_restated() {
        let code = CMap::parse(TWO_RANGES, None).next_code(&[0xe0, 0xe0, 0xe0]);
        assert_eq!(
            invalid_codes_sent(TWO_RANGES, (b"Test-UTF8", b"F-Test-UTF8"), &[code], 700),
            None,
            "a name that only begins the defined one is no definition of it"
        );
    }

    #[test]
    fn widths_past_the_bound_are_refused_before_any_is_read() {
        let one = |cid: i64| {
            [
                Object::Integer(cid),
                Object::Array(vec![Object::Integer(500)]),
            ]
        };
        let at_bound: Vec<Object> = (0..MOST_WIDTH_ENTRIES / 2)
            .flat_map(|cid| one(i64::try_from(cid).expect("small")))
            .collect();
        assert_eq!(at_bound.len(), MOST_WIDTH_ENTRIES);
        let (width, highest) = widths_of(&at_bound, None, Clone::clone).expect("at the bound");
        assert_eq!(width, Object::Integer(500));
        assert_eq!(
            highest,
            u32::try_from(MOST_WIDTH_ENTRIES / 2 - 1).expect("small")
        );
        let mut past = at_bound;
        past.push(Object::Integer(0));
        assert!(
            widths_of(&past, None, Clone::clone).is_none(),
            "past the bound"
        );
    }

    #[test]
    fn cid_0_s_width_is_the_w_entry_then_dw_then_1000() {
        let ranged = [Object::Integer(0), Object::Integer(3), Object::Integer(250)];
        assert_eq!(
            widths_of(&ranged, None, Clone::clone),
            Some((Object::Integer(250), 3))
        );
        let later = [Object::Integer(3), Object::Array(vec![Object::Real(1.5)])];
        assert_eq!(
            widths_of(&later, Some(&Object::Integer(1280)), Clone::clone),
            Some((Object::Integer(1280), 3))
        );
        assert_eq!(
            widths_of(&[], None, Clone::clone),
            Some((Object::Integer(1000), 0))
        );
        assert!(
            widths_of(
                &[Object::Integer(-1), Object::Array(Vec::new())],
                None,
                Clone::clone
            )
            .is_none()
        );
    }

    #[test]
    fn code_zero_goes_to_the_fresh_cid_and_every_other_code_to_its_own() {
        let ranges = identity_but_zero(700);
        let cid_of = |code: u16| {
            ranges
                .iter()
                .find(|(low, high, _)| (*low..=*high).contains(&code))
                .map(|(low, _, first)| first + (code - low))
        };
        assert_eq!(cid_of(0), Some(700));
        for code in [1, 0xFF, 0x100, 0x1234, 0xFFFF] {
            assert_eq!(cid_of(code), Some(code));
        }
        // Every code is covered once.
        let covered: usize = ranges
            .iter()
            .map(|(low, high, _)| usize::from(high - low) + 1)
            .sum();
        assert_eq!(covered, 1 << 16);
    }
}
