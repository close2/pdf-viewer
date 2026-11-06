//! A code Annex D encodes twice, named for the character ISO 32000-1 says it means.
//!
//! # The two codes, and why one glyph cannot serve them
//!
//! `WinAnsiEncoding` names the glyph `hyphen` at codes 45 and 173 and the glyph `space` at codes
//! 32 and 160, and `MacRomanEncoding` names `space` at 32 and 202 (ISO 32000-2 Annex D, Table
//! D.2). A font dictionary may still state different widths for the two codes of a pair — the
//! corpora's producers state 0 for a soft hyphen beside 333 for the hyphen — and ISO 19005-2
//! section 6.2.11.5 and ISO 19005-4 section 6.2.10.5 then ask one glyph's advance to agree with
//! two numbers. A program holds one advance per glyph, so no rewrite of it answers both while
//! both codes reach that glyph.
//!
//! # What the second code means, and the name that says it
//!
//! ISO 32000-1's Annex D, the edition ISO 19005-2 section 5.1 builds on and the one describing a
//! file labelled PDF 1.7 or earlier (ISO 32000-2 §I.2), states each duplicate's meaning in its
//! notes 5 and 6. Of code 173: "The meaning of this duplicate code shall be “soft hyphen,” but it
//! shall be typographically the same as hyphen." Of the second space code, that it signifies a
//! nonbreaking space and is typographically the same as the space. ISO 32000-2's Annex D keeps the glyph and gives
//! the producer the means of saying so, a `/Differences` entry naming `softhyphen` or
//! `nonbreakingspace`, in its own example. Both names are rows of the Adobe Glyph List, which
//! ISO 19005-2 section 6.2.11.6 requires of a non-symbolic `TrueType` font's differences.
//!
//! So the encoding is restated with the name of the character the code already meant, and the
//! code then reaches the glyph the program gives that character, whose advance the existing
//! route restates to the dictionary's width. Nothing is invented: the name is the one ISO 32000-1
//! assigns the code, and the glyph is the program's own. This is done only where that edition
//! describes the file, as `/CIDToGIDMap`'s default is (`doc/adr/2246`): at a part 2 target, and at
//! a part 4 target where the file is labelled below PDF 2.0. A file labelled 2.0 meant `hyphen`
//! by the code unless it said otherwise, and is refused as before (`doc/adr/2723`).

use std::collections::BTreeMap;

use pdf_archive::{Part, Target};
use pdf_syntax::object::{Dictionary, Name, Object};
use pdf_syntax::{Document, Version};

/// The pairs Annex D encodes twice: the base encoding, the duplicate code, the code it
/// duplicates, and the Adobe Glyph List name ISO 32000-1's notes 5 and 6 give it.
const TWINS: [(&[u8], u8, u8, &str); 3] = [
    (b"WinAnsiEncoding", 173, 45, "softhyphen"),
    (b"WinAnsiEncoding", 160, 32, "nonbreakingspace"),
    (b"MacRomanEncoding", 202, 32, "nonbreakingspace"),
];

/// The first version ISO 32000-2 describes; every earlier one is ISO 32000-1's (§I.2).
const FIRST_VERSION_ISO_32000_2_DESCRIBES: Version = Version { major: 2, minor: 0 };

/// A simple font's dictionary with its duplicate codes named, and which codes those were.
#[derive(Debug)]
pub(super) struct Renamed {
    /// The font dictionary with `/Encoding` restated.
    pub(super) dict: Dictionary,
    /// The restated `/Encoding` value, an encoding dictionary.
    pub(super) encoding: Object,
    /// The codes now named for the character ISO 32000-1 says they mean.
    pub(super) codes: Vec<u8>,
}

/// `dict` with each shown duplicate code whose stated width differs from its twin's named for
/// its own character, where ISO 32000-1 describes the file and the code's name is the base
/// encoding's; `None` where no code is.
///
/// `width` is the width the dictionary states for a code, and `shown` the codes the document
/// shows. A code a `/Differences` entry already names is the producer's and is left alone.
pub(super) fn renamed(
    document: &Document,
    dict: &Dictionary,
    shown: &[u8],
    width: impl Fn(u8) -> f32,
    target: Target,
) -> Option<Renamed> {
    let described_by_iso_32000_1 = match target.part() {
        Part::Two => true,
        Part::Four => document
            .version()
            .is_some_and(|version| version < FIRST_VERSION_ISO_32000_2_DESCRIBES),
    };
    if !described_by_iso_32000_1 {
        return None;
    }
    let stated = document.resolve(dict.get("Encoding")?);
    let (base, mut encoding) = match &stated {
        Object::Name(name) => {
            let mut encoding = Dictionary::new();
            encoding.insert(Name::new(&b"BaseEncoding"[..]), Object::Name(name.clone()));
            (name.as_bytes().to_vec(), encoding)
        }
        Object::Dictionary(encoding) => (
            document
                .get_key(encoding, "BaseEncoding")
                .as_name()?
                .as_bytes()
                .to_vec(),
            encoding.clone(),
        ),
        _ => return None,
    };
    let differences = match encoding.get("Differences") {
        Some(differences) => document.resolve(differences).as_array()?.to_vec(),
        None => Vec::new(),
    };
    let differed = differed_codes(&differences);
    let codes: Vec<u8> = TWINS
        .iter()
        .filter(|(encoding, code, twin, _)| {
            *encoding == base.as_slice()
                && shown.contains(code)
                && !differed.contains(code)
                && !differed.contains(twin)
                && (width(*code) - width(*twin)).abs() > f32::EPSILON
        })
        .map(|(_, code, _, _)| *code)
        .collect();
    if codes.is_empty() {
        return None;
    }
    let mut written = differences;
    for (_, code, _, name) in TWINS.iter().filter(|(_, code, ..)| codes.contains(code)) {
        written.push(Object::Integer(i64::from(*code)));
        written.push(Object::Name(Name::new(name.as_bytes())));
    }
    encoding.insert(Name::new(&b"Differences"[..]), Object::Array(written));
    let encoding = Object::Dictionary(encoding);
    let mut dict = dict.clone();
    dict.insert(Name::new(&b"Encoding"[..]), encoding.clone());
    Some(Renamed {
        dict,
        encoding,
        codes,
    })
}

/// Every code a `/Differences` array names (ISO 32000-2 §9.6.5.1's Table 112): each integer
/// starts a run, and each name after it takes the next code.
fn differed_codes(differences: &[Object]) -> Vec<u8> {
    let mut codes = Vec::new();
    let mut next: Option<i64> = None;
    for item in differences {
        match item {
            Object::Integer(code) => next = Some(*code),
            Object::Name(_) => {
                if let Some(code) = next {
                    if let Ok(byte) = u8::try_from(code) {
                        codes.push(byte);
                    }
                    next = code.checked_add(1);
                }
            }
            _ => {}
        }
    }
    codes
}

/// The character each Annex D name stands for, by the Adobe Glyph List.
fn character_of(name: &str) -> Option<u16> {
    match name {
        "softhyphen" => Some(0x00AD),
        "nonbreakingspace" => Some(0x00A0),
        _ => None,
    }
}

/// The name [`TWINS`] gives a duplicate code.
pub(super) fn name_of(code: u8) -> Option<&'static str> {
    TWINS
        .iter()
        .find(|(_, duplicate, ..)| *duplicate == code)
        .map(|(.., name)| *name)
}

/// A `glyf` program in which each of `splits` — a renamed code, the glyph it reached together
/// with its twin, and its own advance in font units — reaches a copy of that glyph of its own,
/// and the copy each code reaches; `None` where the program is not one this rewrites.
///
/// **A copy rather than another glyph**, because ISO 32000-1's Annex D makes each duplicate
/// typographically the same as the code it duplicates: the copy is the twin's outline byte for
/// byte, its bearing the twin's, its advance the width the dictionary states for the code
/// ([`super::notdef_glyph::with_glyph_copied`]). The `cmap` then sends the code's character to
/// the copy in every Unicode subtable, which is where ISO 32000-2 §9.6.5.4 reaches a
/// non-symbolic `TrueType` font's glyph by name; a Unicode subtable in a format other than 4 is
/// not rewritten, and the program is then not one this rewrites.
pub(super) fn split(program: &[u8], splits: &[(u8, u16, u16)]) -> Option<Split> {
    let mut out = program.to_vec();
    let mut reached = Vec::with_capacity(splits.len());
    let mut characters = BTreeMap::new();
    for (code, source, advance) in splits {
        let name = name_of(*code)?;
        let given = super::notdef_glyph::with_glyph_copied(&out, *source, &[*advance], name)?;
        let copy = *given.glyphs.first()?;
        out = pdf_font::embedding::with_tables(&out, &given.tables)?;
        characters.insert(character_of(name)?, copy);
        reached.push((*code, copy));
    }
    let cmap = cmap_with(table(&out, *b"cmap")?, &characters)?;
    out = pdf_font::embedding::with_tables(&out, &[(*b"cmap", cmap)])?;
    Some(Split {
        program: out,
        reached,
    })
}

/// What [`split`] writes: the program, and the copy each split code reaches.
pub(super) struct Split {
    /// The program with the copies and the rewritten `cmap`.
    pub(super) program: Vec<u8>,
    /// Each split code and the copy of its glyph it now reaches.
    pub(super) reached: Vec<(u8, u16)>,
}

/// One table of an sfnt, by tag, read off its directory.
fn table(program: &[u8], tag: [u8; 4]) -> Option<&[u8]> {
    let count = usize::from(u16_at(program, 4)?);
    (0..count).find_map(|index| {
        let record = index.checked_mul(16)?.checked_add(12)?;
        if program.get(record..record.checked_add(4)?)? != tag {
            return None;
        }
        let offset = usize::try_from(u32_at(program, record.checked_add(8)?)?).ok()?;
        let length = usize::try_from(u32_at(program, record.checked_add(12)?)?).ok()?;
        program.get(offset..offset.checked_add(length)?)
    })
}

/// A `cmap` table with each of `characters` sent to its glyph in every Unicode subtable — the
/// Unicode platform's and the Microsoft platform's encodings 1 and 10 — and every other subtable
/// kept byte for byte.
fn cmap_with(cmap: &[u8], characters: &BTreeMap<u16, u16>) -> Option<Vec<u8>> {
    let count = u16_at(cmap, 2)?;
    let mut records = Vec::with_capacity(usize::from(count));
    let mut bodies: Vec<Vec<u8>> = Vec::new();
    for index in 0..usize::from(count) {
        let record = index.checked_mul(8)?.checked_add(4)?;
        let platform = u16_at(cmap, record)?;
        let encoding = u16_at(cmap, record.checked_add(2)?)?;
        let offset = usize::try_from(u32_at(cmap, record.checked_add(4)?)?).ok()?;
        let subtable = cmap.get(offset..)?;
        let unicode = platform == 0 || (platform == 3 && matches!(encoding, 1 | 10));
        let body = if unicode {
            let mut entries = format_4_entries(subtable)?;
            entries.extend(characters);
            super::truetype_cmap::format_4(&entries)?
        } else {
            subtable.get(..subtable_length(subtable)?)?.to_vec()
        };
        records.push((platform, encoding, bodies.len()));
        bodies.push(body);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&count.to_be_bytes());
    let mut at = usize::from(count).checked_mul(8)?.checked_add(4)?;
    let mut offsets = Vec::with_capacity(bodies.len());
    for body in &bodies {
        offsets.push(at);
        at = at.checked_add(body.len())?;
    }
    for (platform, encoding, body) in records {
        out.extend_from_slice(&platform.to_be_bytes());
        out.extend_from_slice(&encoding.to_be_bytes());
        out.extend_from_slice(&u32::try_from(*offsets.get(body)?).ok()?.to_be_bytes());
    }
    for body in bodies {
        out.extend_from_slice(&body);
    }
    Some(out)
}

/// A subtable's length, which the formats state at one of two places (the `cmap` chapter of
/// Apple's TrueType Reference Manual and of the OpenType specification).
fn subtable_length(subtable: &[u8]) -> Option<usize> {
    match u16_at(subtable, 0)? {
        0 | 2 | 4 | 6 => Some(usize::from(u16_at(subtable, 2)?)),
        8 | 10 | 12 | 13 => usize::try_from(u32_at(subtable, 4)?).ok(),
        14 => usize::try_from(u32_at(subtable, 2)?).ok(),
        _ => None,
    }
}

/// Every character a format 4 subtable maps to a glyph other than 0, and the glyph; `None`
/// where the subtable is in another format or does not read.
///
/// The format's own algorithm: per segment, a character from `startCode` to `endCode` maps
/// through `idDelta` alone where `idRangeOffset` is 0, and otherwise through the glyph index
/// array at the place `idRangeOffset` counts to from its own position, `idDelta` added to a
/// non-zero entry; all arithmetic is modulo 65536.
fn format_4_entries(subtable: &[u8]) -> Option<BTreeMap<u16, u16>> {
    if u16_at(subtable, 0)? != 4 {
        return None;
    }
    let segments = usize::from(u16_at(subtable, 6)? / 2);
    let ends = 14_usize;
    let starts = ends.checked_add(segments.checked_mul(2)?)?.checked_add(2)?;
    let deltas = starts.checked_add(segments.checked_mul(2)?)?;
    let ranges = deltas.checked_add(segments.checked_mul(2)?)?;
    let mut entries = BTreeMap::new();
    for segment in 0..segments {
        let end = u16_at(subtable, ends.checked_add(segment.checked_mul(2)?)?)?;
        let start = u16_at(subtable, starts.checked_add(segment.checked_mul(2)?)?)?;
        let delta = u16_at(subtable, deltas.checked_add(segment.checked_mul(2)?)?)?;
        let range_at = ranges.checked_add(segment.checked_mul(2)?)?;
        let range = usize::from(u16_at(subtable, range_at)?);
        if start > end {
            return None;
        }
        for character in start..=end {
            if character == 0xFFFF {
                continue;
            }
            let glyph = if range == 0 {
                character.wrapping_add(delta)
            } else {
                let at = range_at
                    .checked_add(range)?
                    .checked_add(usize::from(character.checked_sub(start)?).checked_mul(2)?)?;
                match u16_at(subtable, at)? {
                    0 => 0,
                    indexed => indexed.wrapping_add(delta),
                }
            };
            if glyph != 0 {
                entries.insert(character, glyph);
            }
        }
    }
    Some(entries)
}

/// A big-endian `u16` at `at`.
fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

/// A big-endian `u32` at `at`.
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// What naming the codes restated, in one clause for the report and the file's own history.
#[must_use]
pub(super) fn described(codes: &[u8]) -> String {
    let listed: Vec<String> = codes
        .iter()
        .map(|code| {
            let name = name_of(*code).unwrap_or_default();
            format!("{code} as /{name}")
        })
        .collect();
    format!(
        "the font's encoding now names {} — the character ISO 32000-1's Annex D says the code \
         means, typographically the same as the code it duplicates — so that it reaches a glyph \
         of its own whose advance is the width the dictionary states for it",
        listed.join(" and ")
    )
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use pdf_archive::{Flavour, Level};

    use super::*;

    /// A one-object document labelled `version`, which is all [`renamed`] reads of it.
    fn document(version: &str) -> Document {
        let mut out = format!("%PDF-{version}\n");
        let at = out.len();
        out.push_str("1 0 obj\n<< /Type /Catalog >>\nendobj\n");
        let xref = out.len();
        let _ = write!(
            out,
            "xref\n0 2\n0000000000 65535 f \n{at:010} 00000 n \n\
             trailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture opens")
    }

    const TWO_B: Target = Target::Two(Level::B);
    const FOUR: Target = Target::Four(Flavour::Plain);

    fn font(encoding: Object) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert(Name::new(&b"Encoding"[..]), encoding);
        dict
    }

    fn win_ansi() -> Object {
        Object::Name(Name::new(&b"WinAnsiEncoding"[..]))
    }

    /// The corpora's widths: 333 for the hyphen, 0 for the soft hyphen.
    fn widths(code: u8) -> f32 {
        if code == 173 { 0.0 } else { 0.333 }
    }

    /// ISO 32000-1 Annex D note 5 makes code 173 of `WinAnsiEncoding` mean the soft hyphen, and
    /// ISO 32000-2's Annex D example names it `/softhyphen` in `/Differences`.
    #[test]
    fn a_soft_hyphen_stated_narrower_than_the_hyphen_is_named_for_itself() {
        let renamed = renamed(
            &document("1.4"),
            &font(win_ansi()),
            &[45, 173],
            widths,
            TWO_B,
        )
        .expect("renamed");
        assert_eq!(renamed.codes, [173]);
        let Object::Dictionary(encoding) = &renamed.encoding else {
            panic!("an encoding dictionary");
        };
        assert_eq!(
            encoding
                .get("BaseEncoding")
                .and_then(Object::as_name)
                .map(Name::as_bytes),
            Some(&b"WinAnsiEncoding"[..])
        );
        assert_eq!(
            encoding.get("Differences"),
            Some(&Object::Array(vec![
                Object::Integer(173),
                Object::Name(Name::new(&b"softhyphen"[..]))
            ]))
        );
        assert!(pdf_font::encoding::is_listed("softhyphen"));
        assert!(pdf_font::encoding::is_listed("nonbreakingspace"));
    }

    /// Two codes the dictionary states at one width already reach one glyph at one advance.
    #[test]
    fn a_soft_hyphen_at_the_hyphens_width_is_left_alone() {
        assert!(
            renamed(
                &document("1.4"),
                &font(win_ansi()),
                &[45, 173],
                |_| 0.333,
                TWO_B,
            )
            .is_none()
        );
    }

    /// ISO 32000-2 §I.2: a file labelled 2.0 is one ISO 32000-2 describes, whose Annex D names
    /// code 173 `hyphen` unless the producer said otherwise; at part 4 it is not renamed.
    #[test]
    fn a_file_iso_32000_2_describes_is_not_renamed_at_part_four() {
        let target = FOUR;
        assert!(renamed(&document("2.0"), &font(win_ansi()), &[173], widths, target).is_none());
        assert!(renamed(&document("1.7"), &font(win_ansi()), &[173], widths, target).is_some());
    }

    /// A code the producer's own `/Differences` names is the producer's statement.
    #[test]
    fn a_code_the_differences_name_is_the_producers() {
        let mut encoding = Dictionary::new();
        encoding.insert(Name::new(&b"BaseEncoding"[..]), win_ansi());
        encoding.insert(
            Name::new(&b"Differences"[..]),
            Object::Array(vec![
                Object::Integer(172),
                Object::Name(Name::new(&b"logicalnot"[..])),
                Object::Name(Name::new(&b"hyphen"[..])),
            ]),
        );
        assert!(
            renamed(
                &document("1.4"),
                &font(Object::Dictionary(encoding)),
                &[45, 173],
                widths,
                TWO_B,
            )
            .is_none()
        );
    }
}
