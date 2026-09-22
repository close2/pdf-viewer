//! What ISO 32000-2 §9.9's Table 124 asks of a font program's own bytes.
//!
//! Table 124 decides which `/FontFile` key may carry a program, and three of its four rows
//! decide it on something only the *program* says: whether an sfnt carries a `glyf` table or a
//! `CFF ` one, and — for the `CFF ` case — whether that table's Top DICT uses `CIDFont`
//! operators. A caller about to write a program into a font descriptor has to ask those
//! questions of bytes it holds and nothing else, which is what this module answers.
//!
//! > A Type1 font dictionary or CIDFontType0 `CIDFont` dictionary, if the embedded font program
//! > contains a "CFF " table without `CIDFont` operators. In addition to the "CFF " table, the
//! > font program shall include the "cmap" table.
//!
//! **The reading of the table is the caller's and not this module's**, deliberately: the pairing
//! is between a *dictionary's* `/Subtype` and these facts, and a module that knew both would be
//! answering a question about a document from a crate that was handed no document.

use crate::cff;
use crate::substitute::Format;

/// Offset of `numGlyphs` within `maxp`, which is that table's second field.
const NUM_GLYPHS: usize = 4;

/// Whether the `CFF ` table an sfnt carries is keyed by CID.
///
/// §9.7.4.2 makes this the difference between two glyph-selection routes rather than a detail of
/// the format: a Top DICT that uses `CIDFont` operators reaches a glyph "using the charset table in
/// the CFF program", and one that does not uses the CIDs "directly as GID values".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keying {
    /// The Top DICT uses `CIDFont` operators, so the charset maps CIDs to glyph indices.
    ByCid,
    /// The Top DICT uses none, so a glyph is reached by name or, under a `CIDFont`, by index.
    ByName,
}

/// Which tables one sfnt carries, and how its `CFF ` table is keyed.
///
/// **The tags are answered by lookup rather than as named fields**, because which tags matter is
/// Table 124's question and not this reader's: the `FontFile2` row names six, the `OpenType`
/// row's three bullets name different ones each, and ISO 19005-4 section 6.2.10.5 adds `vmtx`.
/// A reader that enumerated them would have to be edited whenever a caller read one more line of
/// the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SfntTables {
    /// How the program's `CFF ` table is keyed, where it carries one this crate can read.
    pub cff: Option<Keying>,
    /// Every tag the container's table directory names.
    tags: std::collections::BTreeSet<[u8; 4]>,
}

impl SfntTables {
    /// Whether the container names this table.
    #[must_use]
    pub fn carries(&self, tag: &[u8; 4]) -> bool {
        self.tags.contains(tag)
    }

    /// Whether the container names every one of these tables, which is how Table 124's rows ask.
    #[must_use]
    pub fn carries_all(&self, tags: &[[u8; 4]]) -> bool {
        tags.iter().all(|tag| self.carries(tag))
    }
}

/// What Table 124 asks after in an sfnt, or `None` where the bytes are not an sfnt at all.
///
/// A `CFF ` table this crate's own reader cannot open leaves [`SfntTables::cff`] `None` while
/// [`SfntTables::carries`] still answers for the tag, so a caller reading Table 124 finds no row
/// that admits the program and refuses it rather than writing a key on a guess.
///
/// The container is identified by the version tag ISO/IEC 14496-22 puts at offset zero, and a
/// `ttcf` collection is deliberately not one of them: §9.9.1 requires an embedded font file to
/// consist of exactly one font, so which face of a collection a descriptor carries is not a
/// question these bytes answer.
#[must_use]
pub fn sfnt_tables(program: &[u8]) -> Option<SfntTables> {
    if !matches!(
        program.get(..4),
        Some(b"\x00\x01\x00\x00" | b"true" | b"OTTO")
    ) {
        return None;
    }
    let directory = crate::sfnt::sfnt_tables(program)?;
    let cff = sfnt_table(program, b"CFF ")
        .and_then(|table| cff::uses_cid_operators(table).ok())
        .map(|keyed| if keyed { Keying::ByCid } else { Keying::ByName });
    Some(SfntTables {
        cff,
        tags: directory
            .keys()
            .filter_map(|tag| <[u8; 4]>::try_from(tag.as_slice()).ok())
            .collect(),
    })
}

/// How a program's charstrings are keyed, where the program carries a `CFF ` table at all.
///
/// §9.7.4.2 makes this the difference between two glyph-selection routes under a `CIDFont`: a Top
/// DICT that uses `CIDFont` operators reaches a glyph through the program's charset, and one that
/// does not uses the CIDs "directly as GID values". `None` is a program carrying no `CFF ` table
/// — a `glyf` sfnt, whose glyph indices a `/CIDToGIDMap` maps to — or one this crate cannot read.
#[must_use]
pub fn cff_keying(program: &[u8], format: Format) -> Option<Keying> {
    match format {
        Format::BareCff => cff::uses_cid_operators(program)
            .ok()
            .map(|keyed| if keyed { Keying::ByCid } else { Keying::ByName }),
        Format::Sfnt => sfnt_tables(program)?.cff,
    }
}

/// A copy of an sfnt with some of its tables left out and its directory rebuilt.
///
/// §9.9.1 requires it of a TrueType program embedded under a `CIDFont` dictionary: "If used with a
/// `CIDFont` dictionary, the "cmap" table is not needed and shall not be present, since the mapping
/// from character codes to glyph descriptions is provided separately." A table can only be
/// dropped by writing a new directory, because the directory's own length is what every table's
/// offset is measured from.
///
/// Every kept table's bytes are the program's, copied; what is written afresh is the directory,
/// its search fields and — through the same routine [`crate::restate`] uses — every checksum,
/// which a rebuilt file would otherwise state falsely about itself.
///
/// `None` where the bytes are not an sfnt this reader can take apart, and the program unchanged
/// where it carries none of the tables named.
#[must_use]
pub fn without_tables(program: &[u8], dropped: &[[u8; 4]]) -> Option<Vec<u8>> {
    crate::sfnt::without_tables(program, dropped)
}

/// One table's bytes out of an sfnt, where the container holds it.
fn sfnt_table<'a>(program: &'a [u8], tag: &[u8]) -> Option<&'a [u8]> {
    let tables = crate::sfnt::sfnt_tables(program)?;
    let &(at, length) = tables.get(tag)?;
    program.get(at..at.checked_add(length)?)
}

/// How many glyphs a program holds, which bounds every glyph index into it.
///
/// An sfnt states it in `maxp`; a CFF states it as the length of its `CharStrings` INDEX. A
/// caller embedding a program under a composite font needs it because §9.7.4.2 lets a CID *be* a
/// glyph index, and a CID past the program's last glyph selects nothing at all.
#[must_use]
pub fn glyph_count(program: &[u8], format: Format) -> Option<u16> {
    match format {
        Format::BareCff => u16::try_from(cff::glyph_count(program).ok()?).ok(),
        Format::Sfnt => {
            let maxp = sfnt_table(program, b"maxp")?;
            let pair = maxp.get(NUM_GLYPHS..NUM_GLYPHS.checked_add(2)?)?;
            Some(u16::from_be_bytes([*pair.first()?, *pair.get(1)?]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Keying, cff_keying, glyph_count, sfnt_tables, without_tables};
    use crate::substitute::Format;

    /// A compiled-in bare CFF face, which is name-keyed and states its own glyph count.
    const FOXIT: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");

    /// A compiled-in `glyf` sfnt, which Table 124 admits under a TrueType dictionary alone.
    const LIBERATION: &[u8] =
        include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

    /// A `glyf` face answers `glyf` and no `CFF `, which is the row Table 124 gives `FontFile2`.
    #[test]
    fn a_glyf_sfnt_carries_no_cff_table() {
        let tables = sfnt_tables(LIBERATION).expect("a compiled-in sfnt is an sfnt");
        assert!(
            tables.carries_all(&[*b"glyf", *b"head", *b"hhea", *b"hmtx", *b"loca", *b"maxp"]),
            "Liberation Sans carries every table Table 124's FontFile2 row names"
        );
        assert_eq!(tables.cff, None, "and carries no CFF table");
        assert!(tables.carries(b"cmap"), "and the cmap a simple font needs");
    }

    /// A bare CFF is not an sfnt, which is what keeps the two rows of Table 124 apart.
    #[test]
    fn a_bare_cff_is_not_an_sfnt() {
        assert_eq!(sfnt_tables(FOXIT), None);
    }

    /// §9.9's Table 124 `OpenType` row: a `CFF ` table without `CIDFont` operators, and a `cmap`.
    #[test]
    fn a_cff_wrapped_in_an_sfnt_is_read_out_of_the_wrapper_and_found_name_keyed() {
        let wrapped = otto(&[(b"CFF ", FOXIT), (b"cmap", &[0, 0, 0, 0])]);
        let tables = sfnt_tables(&wrapped).expect("an OTTO container is an sfnt");
        assert_eq!(tables.cff, Some(Keying::ByName));
        assert!(!tables.carries(b"glyf"));
        assert!(tables.carries(b"cmap"));
        assert_eq!(
            cff_keying(&wrapped, Format::Sfnt),
            Some(Keying::ByName),
            "a name-keyed program's CIDs are its glyph indices (§9.7.4.2)"
        );
    }

    /// The glyph count is the program's own, whichever of the two formats states it.
    #[test]
    fn a_programs_glyph_count_bounds_every_index_into_it() {
        let cff = glyph_count(FOXIT, Format::BareCff).expect("a CFF states its CharStrings count");
        let sfnt = glyph_count(LIBERATION, Format::Sfnt).expect("an sfnt states maxp numGlyphs");
        assert!(cff > 0 && sfnt > 0, "{cff} and {sfnt}");
    }

    /// §9.9.1: a TrueType program under a `CIDFont` dictionary carries no `cmap`, so one goes.
    #[test]
    fn a_cmap_is_dropped_and_every_other_table_is_the_programs_own() {
        let without = without_tables(LIBERATION, &[*b"cmap"]).expect("the face rebuilds");
        let before = sfnt_tables(LIBERATION).expect("an sfnt");
        let after = sfnt_tables(&without).expect("still an sfnt");
        assert!(before.carries(b"cmap") && !after.carries(b"cmap"));
        assert!(
            after.carries_all(&[*b"glyf", *b"head", *b"hhea", *b"hmtx", *b"loca", *b"maxp"]),
            "§9.9.1's tables are all still there"
        );
        assert_eq!(
            glyph_count(&without, Format::Sfnt),
            glyph_count(LIBERATION, Format::Sfnt),
            "and the program still holds every glyph it held"
        );
    }

    /// An `OTTO` container holding `tables`, in the order given.
    fn otto(tables: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let count = u16::try_from(tables.len()).expect("a fixture of a few tables");
        let mut out = Vec::from(*b"OTTO");
        out.extend_from_slice(&count.to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        let mut at = u32::try_from(12usize.saturating_add(16usize.saturating_mul(tables.len())))
            .expect("a fixture of a few tables");
        let mut body: Vec<u8> = Vec::new();
        for (tag, data) in tables {
            let length = u32::try_from(data.len()).expect("a fixture of small tables");
            out.extend_from_slice(*tag);
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&at.to_be_bytes());
            out.extend_from_slice(&length.to_be_bytes());
            body.extend_from_slice(data);
            let padded = length.next_multiple_of(4);
            let pad = usize::try_from(padded.saturating_sub(length)).expect("at most three bytes");
            body.resize(body.len().saturating_add(pad), 0);
            at = at.saturating_add(padded);
        }
        out.extend_from_slice(&body);
        out
    }
}
