//! A `TrueType` Collection embedded where ISO 32000-2 §9.9 states a font program.
//!
//! Table 127 says what `/FontFile2` holds:
//!
//! > ( PDF 1.1 ) TrueType font program, as described in the TrueType Reference Manual .
//!
//! A collection is not one: it is a container of several, sharing their tables and introduced
//! by a `ttcf` header rather than by a table directory. **A file embedding one is malformed**,
//! and two of the pdf.js corpus's first pages do it — `issue9262_reduced.pdf` and
//! `issue13193.pdf`. Until the hundred-and-fifty-seventh session this tree refused them with
//! `Invalid sfnt version 0x74746366`, which is `ttcf` spelled in hexadecimal and is exactly the
//! right report for a reader that has decided to do nothing.
//!
//! # Which font of the collection, and why that is a derivation rather than a choice
//!
//! The container holds several faces, so reading one is a decision — and the file makes it.
//! Table 122 gives the descriptor a `/FontName`, which §9.6.2.1 requires to be "the PostScript
//! name of the font", and every face in a collection carries its own PostScript name in its
//! `name` table. Matching the two is reading the document rather than guessing at it, and it is
//! what [`extract`] does; **taking face zero is the fallback and it is recorded as one**,
//! because a collection whose faces the descriptor names none of has told us nothing.
//!
//! Both corpus documents were opened and their collections listed, and each pays for one half
//! of [`same_face`]. `issue9262_reduced.pdf` names `MSMincho` and holds `MS-Mincho` and
//! `MS-PMincho`: the hyphen has to go, and `MS-PMincho` must *not* match. `issue13193.pdf`
//! names `DCWGQU+CambriaMath` and holds `Cambria` and `CambriaMath`: §9.6.4's subset prefix has
//! to go, and **face zero would be the wrong face** — which is what makes the match load-bearing
//! rather than a nicety.
//!
//! # Why the face is copied out rather than referred to
//!
//! `read_fonts::FontRef::from_index` would open the face in place, and every one of this
//! crate's eight `FontRef::new` sites would then have to carry an index that is zero for every
//! font but these. Copying the chosen face's tables into a standalone `sfnt` makes the
//! collection a fact about *loading* and nothing downstream has to know. It costs one copy of
//! one face at load time, which is the same order as the decompression that produced the bytes.

use std::collections::BTreeSet;

use read_fonts::{FileRef, FontRef, TableProvider as _};

/// Bounds the tables copied out of one face, which a malformed header could otherwise inflate.
///
/// A face of a real collection states a few dozen; `sfnt`'s own directory is a `u16` count, so
/// this is a bound on the *work*, not a new restriction on the format.
const MAX_TABLES: usize = 512;

/// Extracts the face `wanted` names from a `TrueType` Collection, as a standalone `sfnt`.
///
/// `None` when `data` is not a collection — which is the ordinary case and is how a caller
/// tells "not a collection" from "a collection this cannot read".
#[must_use]
pub fn extract(data: &[u8], wanted: Option<&str>) -> Option<Vec<u8>> {
    let FileRef::Collection(collection) = FileRef::new(data).ok()? else {
        return None;
    };
    let chosen = wanted
        .and_then(|wanted| {
            collection
                .iter()
                .flatten()
                .find(|face| postscript_name(face).is_some_and(|held| same_face(&held, wanted)))
        })
        .or_else(|| collection.get(0).ok())?;
    rebuild(&chosen)
}

/// A face's PostScript name, from its `name` table (name ID 6).
fn postscript_name(font: &FontRef<'_>) -> Option<String> {
    let name = font.name().ok()?;
    name.name_record()
        .iter()
        .find(|record| record.name_id() == read_fonts::tables::name::NameId::POSTSCRIPT_NAME)
        .and_then(|record| record.string(name.string_data()).ok())
        .map(|string| string.chars().collect())
}

/// Whether two PostScript names name the same face.
///
/// Two normalisations, each with a reason in a file rather than in a style guide. §9.6.4's
/// subset prefix — "six uppercase letters, followed by a plus sign" — is part of the name the
/// *document* writes and never part of the name the *font* carries. And a hyphen is written
/// inconsistently between the two: `issue9262_reduced.pdf` asks for `MSMincho` and its
/// collection offers `MS-Mincho`.
fn same_face(one: &str, other: &str) -> bool {
    fn plain(name: &str) -> String {
        let name = name
            .split_once('+')
            .filter(|(prefix, _)| {
                prefix.len() == 6 && prefix.chars().all(|c| c.is_ascii_uppercase())
            })
            .map_or(name, |(_, rest)| rest);
        name.chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }
    plain(one) == plain(other)
}

/// Writes one face's tables out as a standalone `sfnt`.
///
/// The directory is rebuilt rather than copied because a collection's records point into the
/// whole file: the offsets are what has to change, and every other field of a record is the
/// face's own.
fn rebuild(font: &FontRef<'_>) -> Option<Vec<u8>> {
    let tags: BTreeSet<read_fonts::types::Tag> = font
        .table_directory()
        .table_records()
        .iter()
        .take(MAX_TABLES)
        .map(read_fonts::TableRecord::tag)
        .collect();
    let tables: Vec<(read_fonts::types::Tag, &[u8])> = tags
        .into_iter()
        .filter_map(|tag| font.table_data(tag).map(|data| (tag, data.as_bytes())))
        .collect();
    if tables.is_empty() {
        return None;
    }

    let count = u16::try_from(tables.len()).ok()?;
    // The three fields after the count are a binary-search hint the format states and no
    // reader in this tree consults; they are written correctly anyway, because a file this
    // program produces should not need a reader that tolerates it.
    let entry_selector = count.ilog2();
    let search_range = (1_u32 << entry_selector).checked_mul(16)?;
    let range_shift = u32::from(count)
        .checked_mul(16)?
        .checked_sub(search_range)?;

    let mut out = Vec::new();
    out.extend_from_slice(&font.table_directory().sfnt_version().to_be_bytes());
    out.extend_from_slice(&count.to_be_bytes());
    out.extend_from_slice(&u16::try_from(search_range).ok()?.to_be_bytes());
    out.extend_from_slice(&u16::try_from(entry_selector).ok()?.to_be_bytes());
    out.extend_from_slice(&u16::try_from(range_shift).ok()?.to_be_bytes());

    // Every table is aligned to four bytes, which the format requires of the offsets and
    // which a checksum would be computed over.
    let directory = 12_usize.checked_add(tables.len().checked_mul(16)?)?;
    let mut at = directory;
    for (tag, data) in &tables {
        out.extend_from_slice(&tag.to_be_bytes());
        // The checksum is zero rather than recomputed: nothing in this tree verifies one, and
        // a wrong value would be a claim, where a zero is visibly not one.
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(&u32::try_from(at).ok()?.to_be_bytes());
        out.extend_from_slice(&u32::try_from(data.len()).ok()?.to_be_bytes());
        at = at.checked_add(data.len())?.checked_next_multiple_of(4)?;
    }
    for (_, data) in &tables {
        out.extend_from_slice(data);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::same_face;

    /// The two normalisations, each on the shape that motivated it.
    #[test]
    fn a_subset_prefix_and_a_hyphen_do_not_make_two_faces() {
        assert!(same_face("MS-Mincho", "MSMincho"));
        assert!(same_face("MS-Mincho", "ABCDEF+MSMincho"));
        assert!(same_face("MSGothic", "MS-Gothic"));
        // A prefix that is not §9.6.4's is part of the name.
        assert!(!same_face("MS-Mincho", "ABC+MSMincho"));
        assert!(!same_face("MS-Mincho", "MS-Gothic"));
    }
}
