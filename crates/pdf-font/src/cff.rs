//! Wrapping a bare CFF font program in a minimal `OpenType` container.
//!
//! `/FontFile3` may hold a bare CFF font program rather than a complete `OpenType` file,
//! and `skrifa` reads fonts only through an sfnt container. Rather than reimplement CFF
//! charstring interpretation — which is exactly the memory-unsafe font parsing this
//! project chose skrifa to avoid — the CFF is wrapped in the smallest sfnt that skrifa
//! will accept, and skrifa does the interpreting.
//!
//! # What the wrapper contains, and why only that
//!
//! Three tables. `CFF ` is the original program, untouched. `head` supplies units per em,
//! which every outline is scaled by. `maxp` supplies the glyph count, which bounds glyph
//! lookup. Nothing else is needed to extract outlines: character maps, metrics and layout
//! tables serve text *layout*, and PDF has already done the layout.
//!
//! The glyph count is read out of the CFF's `CharStrings` index, so it is the font's own
//! count rather than a guess. Everything else in `head` is the specification's default.

/// The CFF operator introducing the `CharStrings` offset in a top dictionary.
const OP_CHARSTRINGS: u8 = 17;

/// Units per em assumed for the synthesised `head` table.
///
/// A CFF font's scale lives in its `FontMatrix`, whose default is `0.001` — that is, 1000
/// units per em — and which skrifa reads from the CFF itself. This value only has to agree
/// with that default; a font overriding `FontMatrix` is handled by skrifa applying it.
const UNITS_PER_EM: u16 = 1000;

/// Wraps a bare CFF program in a minimal `OpenType` file.
///
/// Returns `None` if the CFF cannot be understood well enough to count its glyphs, since a
/// wrapper claiming the wrong glyph count would make lookups fail in confusing ways.
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the sfnt directory arithmetic is over a three-element table whose sizes are \
              known at the call site; the shift is bounded by that count"
)]
pub fn wrap_in_sfnt(cff: &[u8]) -> Option<Vec<u8>> {
    let glyphs = glyph_count(cff)?;

    let head = build_head();
    let maxp = build_maxp(glyphs);
    // Tag order must be ascending, as the table directory is binary-searched by readers.
    let tables: [(&[u8; 4], &[u8]); 3] = [(b"CFF ", cff), (b"head", &head), (b"maxp", &maxp)];

    let count = u16::try_from(tables.len()).ok()?;
    let mut out = Vec::with_capacity(cff.len().saturating_add(256));

    // `OTTO` marks an sfnt whose outlines are CFF rather than TrueType.
    out.extend_from_slice(b"OTTO");
    out.extend_from_slice(&count.to_be_bytes());
    // searchRange, entrySelector and rangeShift are derived from the table count. Readers
    // that binary-search trust them, so they are computed rather than zeroed.
    let entry_selector = count.ilog2();
    let search_range = (1u16 << entry_selector).saturating_mul(16);
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&u16::try_from(entry_selector).ok()?.to_be_bytes());
    out.extend_from_slice(
        &count
            .saturating_mul(16)
            .saturating_sub(search_range)
            .to_be_bytes(),
    );

    // Each directory entry is 16 bytes, and the data follows the whole directory.
    let mut offset = 12u32.saturating_add(u32::from(count).saturating_mul(16));
    let mut directory = Vec::new();
    let mut body = Vec::new();

    for (tag, data) in tables {
        directory.extend_from_slice(tag);
        // Checksums are not verified by skrifa, and computing them correctly would mean
        // also patching head's checkSumAdjustment; zero is honest about being unused.
        directory.extend_from_slice(&0u32.to_be_bytes());
        directory.extend_from_slice(&offset.to_be_bytes());
        directory.extend_from_slice(&u32::try_from(data.len()).ok()?.to_be_bytes());

        body.extend_from_slice(data);
        // Tables are four-byte aligned.
        let padding = (4 - data.len() % 4) % 4;
        body.extend(std::iter::repeat_n(0u8, padding));
        offset = offset
            .saturating_add(u32::try_from(data.len()).ok()?)
            .saturating_add(u32::try_from(padding).ok()?);
    }

    out.extend_from_slice(&directory);
    out.extend_from_slice(&body);
    Some(out)
}

/// Builds a `head` table carrying the units per em.
fn build_head() -> Vec<u8> {
    let mut head = Vec::with_capacity(54);
    head.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // version 1.0
    head.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // fontRevision
    head.extend_from_slice(&0u32.to_be_bytes()); // checkSumAdjustment
    head.extend_from_slice(&0x5F0F_3CF5u32.to_be_bytes()); // magicNumber, fixed by the spec
    head.extend_from_slice(&0u16.to_be_bytes()); // flags
    head.extend_from_slice(&UNITS_PER_EM.to_be_bytes());
    head.extend_from_slice(&0u64.to_be_bytes()); // created
    head.extend_from_slice(&0u64.to_be_bytes()); // modified
    // The bounding box is advisory for outline extraction; a generous box avoids any
    // reader clipping to it.
    head.extend_from_slice(&(-2000i16).to_be_bytes()); // xMin
    head.extend_from_slice(&(-2000i16).to_be_bytes()); // yMin
    head.extend_from_slice(&2000i16.to_be_bytes()); // xMax
    head.extend_from_slice(&2000i16.to_be_bytes()); // yMax
    head.extend_from_slice(&0u16.to_be_bytes()); // macStyle
    head.extend_from_slice(&8u16.to_be_bytes()); // lowestRecPPEM
    head.extend_from_slice(&2i16.to_be_bytes()); // fontDirectionHint
    head.extend_from_slice(&0i16.to_be_bytes()); // indexToLocFormat
    head.extend_from_slice(&0i16.to_be_bytes()); // glyphDataFormat
    head
}

/// Builds a version 0.5 `maxp` table, which is the CFF form and carries only a count.
fn build_maxp(glyphs: u16) -> Vec<u8> {
    let mut maxp = Vec::with_capacity(6);
    maxp.extend_from_slice(&0x0000_5000u32.to_be_bytes());
    maxp.extend_from_slice(&glyphs.to_be_bytes());
    maxp
}

/// Counts glyphs by reading the CFF's `CharStrings` index.
///
/// The path is: header, then the Name INDEX, then the Top DICT INDEX, whose first
/// dictionary holds the `CharStrings` offset; the INDEX there begins with its own count.
fn glyph_count(cff: &[u8]) -> Option<u16> {
    // The header's fourth byte is its own size, so the first index starts there.
    let header_size = usize::from(*cff.get(2)?);
    let after_names = skip_index(cff, header_size)?;
    let top_dicts_start = after_names;
    let (top_dict, _) = first_index_entry(cff, top_dicts_start)?;

    let charstrings_offset = find_operand(top_dict, OP_CHARSTRINGS)?;
    let count = read_u16(cff, usize::try_from(charstrings_offset).ok()?)?;
    Some(count)
}

/// Returns the offset just past an INDEX structure.
fn skip_index(data: &[u8], at: usize) -> Option<usize> {
    let count = read_u16(data, at)?;
    if count == 0 {
        // An empty index is just its two-byte count.
        return at.checked_add(2);
    }
    let off_size = usize::from(*data.get(at.checked_add(2)?)?);
    if !(1..=4).contains(&off_size) {
        return None;
    }
    let offsets_at = at.checked_add(3)?;
    // There are count + 1 offsets; the last gives the data length.
    let last_offset_at = offsets_at.checked_add(usize::from(count).checked_mul(off_size)?)?;
    let data_len = read_offset(data, last_offset_at, off_size)?;
    let data_start = last_offset_at.checked_add(off_size)?;
    // Offsets are one-based.
    data_start.checked_add(data_len.checked_sub(1)?)
}

/// Returns the first entry of an INDEX, and the offset just past the index.
fn first_index_entry(data: &[u8], at: usize) -> Option<(&[u8], usize)> {
    let count = read_u16(data, at)?;
    if count == 0 {
        return None;
    }
    let off_size = usize::from(*data.get(at.checked_add(2)?)?);
    if !(1..=4).contains(&off_size) {
        return None;
    }
    let offsets_at = at.checked_add(3)?;
    let first = read_offset(data, offsets_at, off_size)?;
    let second = read_offset(data, offsets_at.checked_add(off_size)?, off_size)?;
    let last_offset_at = offsets_at.checked_add(usize::from(count).checked_mul(off_size)?)?;
    let data_start = last_offset_at.checked_add(off_size)?;

    let entry_start = data_start.checked_add(first.checked_sub(1)?)?;
    let entry_end = data_start.checked_add(second.checked_sub(1)?)?;
    let entry = data.get(entry_start..entry_end)?;

    let total = read_offset(data, last_offset_at, off_size)?;
    let end = data_start.checked_add(total.checked_sub(1)?)?;
    Some((entry, end))
}

/// Reads a big-endian offset of `size` bytes.
fn read_offset(data: &[u8], at: usize, size: usize) -> Option<usize> {
    let bytes = data.get(at..at.checked_add(size)?)?;
    let mut value = 0usize;
    for &byte in bytes {
        value = value.checked_mul(256)?.checked_add(usize::from(byte))?;
    }
    Some(value)
}

fn read_u16(data: &[u8], at: usize) -> Option<u16> {
    let bytes = data.get(at..at.checked_add(2)?)?;
    Some(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]))
}

/// Finds an operator's single integer operand in a CFF DICT.
///
/// A DICT is a sequence of operands followed by an operator. Only the integer
/// encodings are decoded, which is all the `CharStrings` offset needs.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the CFF integer encodings are defined by arithmetic on bounded byte ranges: \
              each arm's guard fixes the operand's range, so none can overflow"
)]
fn find_operand(dict: &[u8], operator: u8) -> Option<u32> {
    let mut operands: Vec<i32> = Vec::new();
    let mut at = 0usize;

    while let Some(&byte) = dict.get(at) {
        match byte {
            // Operators are 0..=21, with 12 introducing a two-byte escape.
            0..=21 => {
                if byte == 12 {
                    at = at.checked_add(2)?;
                } else {
                    if byte == operator {
                        return operands.last().and_then(|value| u32::try_from(*value).ok());
                    }
                    at = at.checked_add(1)?;
                }
                operands.clear();
            }
            // 28: a two-byte signed integer.
            28 => {
                let value = i32::from(i16::from_be_bytes([
                    *dict.get(at.checked_add(1)?)?,
                    *dict.get(at.checked_add(2)?)?,
                ]));
                operands.push(value);
                at = at.checked_add(3)?;
            }
            // 29: a four-byte signed integer.
            29 => {
                let bytes = dict.get(at.checked_add(1)?..at.checked_add(5)?)?;
                let value = i32::from_be_bytes([
                    *bytes.first()?,
                    *bytes.get(1)?,
                    *bytes.get(2)?,
                    *bytes.get(3)?,
                ]);
                operands.push(value);
                at = at.checked_add(5)?;
            }
            // 30: a real number, terminated by a nibble of 0xf. Not needed for an offset,
            // but must be skipped correctly or everything after it is misread.
            30 => {
                at = at.checked_add(1)?;
                while let Some(&pair) = dict.get(at) {
                    at = at.checked_add(1)?;
                    if pair & 0x0f == 0x0f || pair >> 4 == 0x0f {
                        break;
                    }
                }
            }
            32..=246 => {
                operands.push(i32::from(byte) - 139);
                at = at.checked_add(1)?;
            }
            247..=250 => {
                let low = i32::from(*dict.get(at.checked_add(1)?)?);
                operands.push((i32::from(byte) - 247) * 256 + low + 108);
                at = at.checked_add(2)?;
            }
            251..=254 => {
                let low = i32::from(*dict.get(at.checked_add(1)?)?);
                operands.push(-(i32::from(byte) - 251) * 256 - low - 108);
                at = at.checked_add(2)?;
            }
            _ => at = at.checked_add(1)?,
        }
    }

    None
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test fixtures are built from small literal offsets"
)]
mod tests {
    use super::{glyph_count, wrap_in_sfnt};

    /// A minimal CFF: header, empty Name INDEX, a Top DICT naming a `CharStrings`
    /// offset, and a `CharStrings` INDEX holding three entries.
    fn minimal_cff() -> Vec<u8> {
        let mut cff = vec![1, 0, 4, 1]; // major, minor, hdrSize, offSize
        // Name INDEX: empty.
        cff.extend_from_slice(&0u16.to_be_bytes());
        // Top DICT INDEX: one entry.
        cff.extend_from_slice(&1u16.to_be_bytes());
        cff.push(1); // offSize
        cff.push(1); // offset[0]
        // The DICT itself: operand 30 (as a one-byte encoding) then operator 17.
        let charstrings_at = 0u8; // patched below
        let dict = vec![charstrings_at.wrapping_add(139), 17];
        cff.push(u8::try_from(dict.len() + 1).unwrap_or(2)); // offset[1]
        let dict_start = cff.len();
        cff.extend_from_slice(&dict);

        // The CharStrings index must sit where the DICT says. Patch the operand to the
        // actual offset, using the two-byte encoding so any offset fits.
        let charstrings_offset = cff.len();
        let mut patched = cff.clone();
        patched.truncate(dict_start);
        patched.push(28); // two-byte integer
        patched.extend_from_slice(
            &u16::try_from(charstrings_offset + 2)
                .unwrap_or(0)
                .to_be_bytes(),
        );
        patched.push(17);
        // Fix the DICT length recorded in the index offsets.
        patched[dict_start - 1] = 5;

        patched.extend_from_slice(&3u16.to_be_bytes()); // three glyphs
        patched.push(1); // offSize
        patched.extend_from_slice(&[1, 1, 1, 1]); // four offsets, all empty entries
        patched
    }

    #[test]
    fn glyphs_are_counted_from_the_charstrings_index() {
        assert_eq!(glyph_count(&minimal_cff()), Some(3));
    }

    #[test]
    fn the_wrapper_is_a_well_formed_otto_file() {
        let wrapped = wrap_in_sfnt(&minimal_cff()).expect("wraps");

        assert_eq!(
            wrapped.get(..4),
            Some(&b"OTTO"[..]),
            "CFF outlines use the OTTO tag"
        );
        assert_eq!(
            wrapped.get(4..6),
            Some(&3u16.to_be_bytes()[..]),
            "three tables: CFF, head and maxp"
        );

        // The table directory must name the tables in ascending tag order, since readers
        // binary-search it.
        let tags: Vec<&[u8]> = (0..3)
            .filter_map(|index| wrapped.get(12 + index * 16..12 + index * 16 + 4))
            .collect();
        assert_eq!(tags, vec![&b"CFF "[..], &b"head"[..], &b"maxp"[..]]);
        let mut sorted = tags.clone();
        sorted.sort_unstable();
        assert_eq!(tags, sorted, "tags must ascend");
    }

    #[test]
    fn malformed_input_is_refused_rather_than_guessed() {
        assert_eq!(wrap_in_sfnt(&[]), None);
        assert_eq!(wrap_in_sfnt(&[1, 0, 4, 1]), None, "no indexes at all");
    }
}
