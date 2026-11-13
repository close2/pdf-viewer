//! ISO 32000-2 §9.9.2's subset tag, moved inside the font program that states it.
//!
//! > For a font subset, the PostScript name of the font, that is, the value of the font's
//! > BaseFont entry and the font descriptor's FontName entry, shall begin with a tag followed by
//! > a plus sign (+) followed by the PostScript name of the font from which the subset was
//! > created. The tag shall consist of exactly six uppercase letters; the choice of letters is
//! > arbitrary, but different subsets of the same font in the same PDF file shall have different
//! > tags.
//!
//! A writer that finds two different subsets under one tag gives one of them another, and the
//! name lives in the program as well as in the dictionaries wherever the format states it there:
//! Table 117's `/BaseFont` for a Type 0 `CIDFont` "shall be the value of the `CIDFontName` entry in
//! the `CIDFont` program", which a compact font states in its Name INDEX (Adobe Technical Note
//! #5176, section 7); a Type 1 program states its `/FontName` in its clear-text portion (Adobe
//! Type 1 Font Format, section 2.2); and §9.6.3 derives a `TrueType` font's `/BaseFont` from the
//! program's `name` table, whose PostScript-name records a producer may have tagged as well.
//!
//! **The new tag is written over the old one in place.** Both are six letters, so no offset,
//! length or `/Length1` the program or its stream states changes; an sfnt's checksums are the
//! one thing restated, because they sum the bytes that moved. A name that does not begin with the
//! old tag and a plus sign is left as it is, so a program that never stated the tag comes back
//! unchanged and a face genuinely named with a plus sign keeps its name.

use crate::sfnt;

/// The program formats Table 124 embeds, as far as where they state their name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramFormat {
    /// `/FontFile`: a Type 1 program, its name in the clear-text `/FontName`.
    Type1,
    /// `/FontFile2`: a `TrueType` program, its names in the `name` table.
    TrueType,
    /// `/FontFile3` `/Type1C` or `/CIDFontType0C`: a bare compact font, its name in the Name INDEX.
    Compact,
    /// `/FontFile3` `/OpenType`: an sfnt, its names in the `name` table and, where it has one, in
    /// its `CFF ` table's Name INDEX.
    OpenType,
}

/// `program` with every name it states under the tag `from` restated under the tag `to`, or
/// `None` where it states none (or cannot be read far enough to say).
///
/// `from` and `to` are §9.9.2's six uppercase letters; a name the program states is retagged
/// where it begins with `from` and a plus sign.
#[must_use]
pub fn retagged(
    program: &[u8],
    format: ProgramFormat,
    from: [u8; 6],
    to: [u8; 6],
) -> Option<Vec<u8>> {
    let mut out = program.to_vec();
    let changed = match format {
        ProgramFormat::Type1 => in_type1(&mut out, from, to),
        ProgramFormat::Compact => in_name_index(&mut out, from, to),
        ProgramFormat::TrueType | ProgramFormat::OpenType => {
            let tables = sfnt::sfnt_tables(program)?;
            let mut changed = false;
            for (tag, &(at, length)) in &tables {
                let table = out.get_mut(at..at.checked_add(length)?)?;
                changed |= match tag.as_slice() {
                    b"name" => in_name_table(table, from, to),
                    b"CFF " => in_name_index(table, from, to),
                    _ => false,
                };
            }
            if changed {
                out = sfnt::checksummed(out)?;
            }
            changed
        }
    };
    changed.then_some(out)
}

/// Overwrites `from` with `to` where `bytes` begins with `from` and a plus sign.
fn retag_at(bytes: &mut [u8], from: [u8; 6], to: [u8; 6]) -> bool {
    match bytes.get_mut(..7) {
        Some(head) if head.starts_with(&from) && head.get(6) == Some(&b'+') => head
            .get_mut(..6)
            .map(|tag| tag.copy_from_slice(&to))
            .is_some(),
        _ => false,
    }
}

/// A compact font's Name INDEX, Adobe Technical Note #5176 sections 5 and 7: after the header,
/// whose third byte is its own size, a two-byte count, a one-byte offset size and `count + 1`
/// offsets, each one past the index's data start.
fn in_name_index(cff: &mut [u8], from: [u8; 6], to: [u8; 6]) -> bool {
    let entries = || -> Option<Vec<(usize, usize)>> {
        let header = usize::from(*cff.get(2)?);
        let count = usize::from(sfnt::be16(cff, header)?);
        if count == 0 {
            return Some(Vec::new());
        }
        let size = usize::from(*cff.get(header.checked_add(2)?)?);
        let offsets = header.checked_add(3)?;
        let offset = |index: usize| -> Option<usize> {
            let at = offsets.checked_add(index.checked_mul(size)?)?;
            cff.get(at..at.checked_add(size)?)?
                .iter()
                .try_fold(0usize, |value, byte| {
                    value.checked_mul(256)?.checked_add(usize::from(*byte))
                })
        };
        // The offsets count from the byte before the data, so the first is 1.
        let data = offsets
            .checked_add(count.checked_add(1)?.checked_mul(size)?)?
            .checked_sub(1)?;
        (0..count)
            .map(|index| {
                Some((
                    data.checked_add(offset(index)?)?,
                    data.checked_add(offset(index.checked_add(1)?)?)?,
                ))
            })
            .collect()
    };
    let mut changed = false;
    for (start, end) in entries().unwrap_or_default() {
        if let Some(name) = cff.get_mut(start..end) {
            changed |= retag_at(name, from, to);
        }
    }
    changed
}

/// A Type 1 program's `/FontName /<name> def`, the first the clear text states.
fn in_type1(program: &mut [u8], from: [u8; 6], to: [u8; 6]) -> bool {
    const KEY: &[u8] = b"/FontName";
    let Some(at) = program.windows(KEY.len()).position(|window| window == KEY) else {
        return false;
    };
    let after = at.saturating_add(KEY.len());
    let Some(rest) = program.get_mut(after..) else {
        return false;
    };
    // PostScript's white space is the six characters §7.2.3 names for PDF as well.
    let skipped = rest
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'\0'))
        .count();
    match rest.get_mut(skipped..) {
        Some([b'/', name @ ..]) => retag_at(name, from, to),
        _ => false,
    }
}

/// Every record of an sfnt `name` table whose string begins with the tag, in either of the two
/// encodings ISO/IEC 14496-22 gives the records a PostScript name is stated in: one byte a
/// character (the Macintosh platform) or UTF-16BE (Unicode and Windows).
fn in_name_table(table: &mut [u8], from: [u8; 6], to: [u8; 6]) -> bool {
    let records = || -> Option<Vec<(usize, usize)>> {
        let count = usize::from(sfnt::be16(table, 2)?);
        let storage = usize::from(sfnt::be16(table, 4)?);
        (0..count)
            .map(|index| {
                let at = 6usize.checked_add(index.checked_mul(12)?)?;
                let length = usize::from(sfnt::be16(table, at.checked_add(8)?)?);
                let offset = usize::from(sfnt::be16(table, at.checked_add(10)?)?);
                let start = storage.checked_add(offset)?;
                Some((start, start.checked_add(length)?))
            })
            .collect()
    };
    let wide = |tag: [u8; 6]| -> [u8; 12] {
        let mut out = [0u8; 12];
        for (pair, letter) in out.chunks_exact_mut(2).zip(tag) {
            if let Some(low) = pair.get_mut(1) {
                *low = letter;
            }
        }
        out
    };
    let (wide_from, wide_to) = (wide(from), wide(to));
    let mut changed = false;
    for (start, end) in records().unwrap_or_default() {
        let Some(string) = table.get_mut(start..end) else {
            continue;
        };
        if retag_at(string, from, to) {
            changed = true;
        } else if string.starts_with(&wide_from)
            && string.get(12..14) == Some(&[0, b'+'])
            && let Some(head) = string.get_mut(..12)
        {
            head.copy_from_slice(&wide_to);
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::{ProgramFormat, retagged};

    /// A compact font as far as its Name INDEX: Adobe Technical Note #5176's header of four
    /// bytes, then a count of one, a one-byte offset size and two offsets.
    fn compact(name: &[u8]) -> Vec<u8> {
        let mut out = vec![1, 0, 4, 1, 0, 1, 1, 1];
        out.push(u8::try_from(name.len().saturating_add(1)).expect("short name"));
        out.extend_from_slice(name);
        out.extend_from_slice(b"rest of the program");
        out
    }

    /// §9.9.2's tag in the Name INDEX moves with the dictionaries, which Table 117 requires of a
    /// Type 0 `CIDFont`'s `/BaseFont` ("the value of the `CIDFontName` entry in the `CIDFont`
    /// program"); every other byte stays.
    #[test]
    fn a_compact_fonts_name_index_takes_the_new_tag() {
        let source = compact(b"EOODIA+Poetica");
        let moved =
            retagged(&source, ProgramFormat::Compact, *b"EOODIA", *b"EOODIB").expect("tagged");
        assert_eq!(moved, compact(b"EOODIB+Poetica"));
        assert_eq!(
            retagged(&source, ProgramFormat::Compact, *b"ABCDEF", *b"ABCDEG"),
            None,
            "a name under another tag is not this subset's"
        );
        assert_eq!(
            retagged(
                &compact(b"EOODIA-Poetica"),
                ProgramFormat::Compact,
                *b"EOODIA",
                *b"EOODIB"
            ),
            None,
            "§9.9.2's tag is followed by a plus sign"
        );
    }

    /// §9.9.2's own EXAMPLE, "EOODIA+Poetica is the name of a subset of Poetica®, a Type 1
    /// font", as the clear text of a Type 1 program states it.
    #[test]
    fn a_type1_programs_font_name_takes_the_new_tag() {
        let source =
            b"%!PS-AdobeFont-1.0: Poetica\n/FontName /EOODIA+Poetica def\n/Encoding".to_vec();
        let moved =
            retagged(&source, ProgramFormat::Type1, *b"EOODIA", *b"EOODIB").expect("tagged");
        assert_eq!(
            moved,
            b"%!PS-AdobeFont-1.0: Poetica\n/FontName /EOODIB+Poetica def\n/Encoding".to_vec()
        );
    }

    /// §9.6.3 derives a `TrueType` `/BaseFont` from the `name` table's PostScript name, so a
    /// tagged one moves with the dictionaries, in both the one-byte and the UTF-16BE records,
    /// and the program's checksums hold over the new bytes.
    #[test]
    fn a_truetype_programs_tagged_names_take_the_new_tag() {
        let (program, _) = crate::standard::face(crate::substitute::Request {
            family: crate::substitute::Family::SansSerif,
            bold: false,
            italic: false,
            standard: true,
        });
        let wide: Vec<u8> = "EOODIA+Poetica"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect();
        // Two records, name ID 6 on the Macintosh platform and on Windows.
        let mut names = vec![0, 0, 0, 2, 0, 30];
        for (platform, encoding, language, length, offset) in
            [(1u16, 0u16, 0u16, 14u16, 0u16), (3, 1, 0x409, 28, 14)]
        {
            for value in [platform, encoding, language, 6, length, offset] {
                names.extend_from_slice(&value.to_be_bytes());
            }
        }
        names.extend_from_slice(b"EOODIA+Poetica");
        names.extend_from_slice(&wide);
        let source = crate::sfnt::with_name_table(program, names).expect("fixture");
        assert_eq!(crate::embed::postscript_name(&source), "EOODIA+Poetica");
        let moved =
            retagged(&source, ProgramFormat::TrueType, *b"EOODIA", *b"EOODIB").expect("tagged");
        assert_eq!(crate::embed::postscript_name(&moved), "EOODIB+Poetica");
        assert!(crate::sfnt::checksums_hold(&moved));
        assert_eq!(moved.len(), source.len());
        assert_eq!(
            retagged(program, ProgramFormat::TrueType, *b"EOODIA", *b"EOODIB"),
            None,
            "the face states no tag, so it is not rewritten"
        );
    }
}
