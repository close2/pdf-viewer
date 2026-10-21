//! Identity names for a metric-modified Liberation program (owner answer A550).
//!
//! The SIL OFL's reserved name applies to the modified font's identity, not to its copyright
//! and licence records. OpenType's `name` records may share storage; changed identities therefore
//! get their own strings, leaving legal notices and language-tag records untouched.

use crate::sfnt;

/// Renames Liberation identity records to `QuorraText`, preserving styles and legal notices.
///
/// Call only after modifying an sfnt program. Other names are retained; the PDF dictionary's
/// name is not used as evidence about the program. Returns `None` for a malformed name table or
/// a replacement whose string offset cannot be represented. Every sfnt checksum is repaired.
#[must_use]
pub fn liberation(program: &[u8]) -> Option<Vec<u8>> {
    let tables = sfnt::sfnt_tables(program)?;
    let Some(&(start, length)) = tables.get(b"name".as_slice()) else {
        return Some(program.to_vec());
    };
    let source = program.get(start..start.checked_add(length)?)?;
    if word(source, 0)? > 1 {
        return None;
    }
    let count = usize::from(word(source, 2)?);
    let storage = usize::from(word(source, 4)?);
    let records_end = 6usize.checked_add(count.checked_mul(12)?)?;
    if storage < records_end || storage > source.len() {
        return None;
    }
    let mut out = source.to_vec();
    let mut changed = false;
    for index in 0..count {
        let at = 6usize.checked_add(index.checked_mul(12)?)?;
        // OpenType name IDs: family/subfamily, unique/full/PostScript, typographic, compatible
        // full, CID findfont, WWS and variation PostScript prefix. Never copyright or licence.
        if !matches!(word(source, at.checked_add(6)?)?, 1..=4 | 6 | 16..=18 | 20..=22 | 25) {
            continue;
        }
        let length = usize::from(word(source, at.checked_add(8)?)?);
        let offset = usize::from(word(source, at.checked_add(10)?)?);
        let begin = storage.checked_add(offset)?;
        let bytes = source.get(begin..begin.checked_add(length)?)?;
        let mut replacement = bytes.to_vec();
        replace(&mut replacement, b"Liberation", b"QuorraText");
        let old: Vec<u8> = "Liberation"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect();
        let new: Vec<u8> = "QuorraText"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect();
        replace(&mut replacement, &old, &new);
        if replacement == bytes {
            continue;
        }
        let offset = u16::try_from(out.len().checked_sub(storage)?).ok()?;
        out.get_mut(at.checked_add(10)?..at.checked_add(12)?)?
            .copy_from_slice(&offset.to_be_bytes());
        out.extend_from_slice(&replacement);
        changed = true;
    }
    if changed {
        sfnt::with_name_table(program, out)
    } else {
        Some(program.to_vec())
    }
}

/// A checked big-endian field in the name table.
fn word(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

/// Equal-length tokens preserve each record's encoding and length.
fn replace(bytes: &mut [u8], old: &[u8], new: &[u8]) {
    let mut cursor = 0;
    while let Some(tail) = bytes.get_mut(cursor..) {
        let Some(relative) = tail.windows(old.len()).position(|part| part == old) else {
            break;
        };
        let Some(part) = tail.get_mut(relative..relative.saturating_add(old.len())) else {
            break;
        };
        part.copy_from_slice(new);
        cursor = cursor.saturating_add(relative).saturating_add(old.len());
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        reason = "fixed font fixtures and checked table records"
    )]

    use super::{liberation, word};
    use crate::substitute::{Family, Request};

    /// The active records, not orphaned string storage a table replacement may leave behind.
    fn records(program: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let tables = crate::sfnt::sfnt_tables(program).expect("sfnt");
        let (start, length) = tables[b"name".as_slice()];
        let names = &program[start..start + length];
        let storage = usize::from(word(names, 4).expect("storage"));
        (0..usize::from(word(names, 2).expect("count")))
            .map(|index| {
                let at = 6 + index * 12;
                let id = word(names, at + 6).expect("id");
                let length = usize::from(word(names, at + 8).expect("length"));
                let offset = storage + usize::from(word(names, at + 10).expect("offset"));
                (id, names[offset..offset + length].to_vec())
            })
            .collect()
    }

    #[test]
    fn every_liberation_style_loses_only_its_reserved_identity_and_keeps_its_glyph_tables() {
        // A550 and OFL condition 3 reserve Liberation in Modified Versions. OpenType's name
        // table supplies every platform/language identity; copyright and licence are not names.
        for bold in [false, true] {
            for italic in [false, true] {
                let (before, _) = crate::standard::face(Request {
                    family: Family::SansSerif,
                    bold,
                    italic,
                    standard: true,
                });
                let after = liberation(before).expect("rename");
                let old = records(before);
                let new = records(&after);
                assert_ne!(old, new);
                for ((id, old), (new_id, new)) in old.iter().zip(&new) {
                    assert_eq!(id, new_id);
                    if matches!(id, 0 | 7..=15) {
                        assert_eq!(old, new, "legal and attribution record {id}");
                    } else {
                        assert!(!new.windows(10).any(|part| part == b"Liberation"));
                        let utf16: Vec<u8> = "Liberation"
                            .encode_utf16()
                            .flat_map(u16::to_be_bytes)
                            .collect();
                        assert!(!new.windows(utf16.len()).any(|part| part == utf16));
                    }
                }
                let old_tables = crate::sfnt::sfnt_tables(before).expect("old");
                let new_tables = crate::sfnt::sfnt_tables(&after).expect("new");
                for (tag, &(at, length)) in &old_tables {
                    if tag == b"name" || tag == b"head" {
                        continue;
                    }
                    let (new_at, new_length) = new_tables[tag];
                    assert_eq!(length, new_length);
                    assert_eq!(
                        &before[at..at + length],
                        &after[new_at..new_at + new_length],
                        "table {tag:?}"
                    );
                }
                let sum = after.chunks(4).fold(0u32, |sum, bytes| {
                    let mut word = [0; 4];
                    word[..bytes.len()].copy_from_slice(bytes);
                    sum.wrapping_add(u32::from_be_bytes(word))
                });
                assert_eq!(sum, 0xb1b0_afba, "OpenType head checkSumAdjustment");
                assert_eq!(liberation(&after).expect("already renamed"), after);
            }
        }
    }

    #[test]
    fn malformed_name_storage_is_refused_before_mutation() {
        let (program, _) = crate::standard::face(Request {
            family: Family::SansSerif,
            bold: false,
            italic: false,
            standard: true,
        });
        let tables = crate::sfnt::sfnt_tables(program).expect("sfnt");
        let (start, _) = tables[b"name".as_slice()];
        let mut bad = program.to_vec();
        bad[start + 4..start + 6].copy_from_slice(&0u16.to_be_bytes());
        assert!(liberation(&bad).is_none());
    }
    #[test]
    fn shared_legal_storage_survives_and_bad_identity_offsets_refuse() {
        let (program, _) = crate::standard::face(Request {
            family: Family::SansSerif,
            bold: false,
            italic: false,
            standard: true,
        });
        // Two records deliberately share the exact bytes: copyright and family identity.
        let mut names = vec![0, 0, 0, 2, 0, 30];
        for id in [0u16, 1] {
            for value in [1u16, 0, 0, id, 10, 0] {
                names.extend_from_slice(&value.to_be_bytes());
            }
        }
        names.extend_from_slice(b"Liberation");
        let source = crate::sfnt::with_name_table(program, names.clone()).expect("fixture");
        let renamed = liberation(&source).expect("rename shared storage");
        assert_eq!(
            records(&renamed),
            vec![(0, b"Liberation".to_vec()), (1, b"QuorraText".to_vec())]
        );
        names[28..30].copy_from_slice(&u16::MAX.to_be_bytes());
        let malformed = crate::sfnt::with_name_table(program, names).expect("malformed fixture");
        assert!(liberation(&malformed).is_none());
    }
}
