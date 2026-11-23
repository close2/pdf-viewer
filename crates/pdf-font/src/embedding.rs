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

/// A copy of an sfnt with the tables given put in, each replacing the program's own table of
/// its tag or added beside the rest, and its directory rebuilt with every checksum.
///
/// What a fixture needs to state a table's rule against a real program: a face's outlines and
/// metrics kept, and the one table under test written byte for byte (ADR 1682's pair kerning).
/// `None` where the bytes are not an sfnt this reader can take apart.
#[must_use]
pub fn with_tables(program: &[u8], tables: &[([u8; 4], Vec<u8>)]) -> Option<Vec<u8>> {
    let mut kept = std::collections::BTreeMap::new();
    for (tag, &(at, length)) in &crate::sfnt::sfnt_tables(program)? {
        let tag: [u8; 4] = tag.as_slice().try_into().ok()?;
        kept.insert(tag, program.get(at..at.checked_add(length)?)?.to_vec());
    }
    for (tag, bytes) in tables {
        kept.insert(*tag, bytes.clone());
    }
    crate::embed::assembled(&kept, program.get(..4)?.try_into().ok()?)
}

/// The face of a font collection the descriptor's `/FontName` names, written out as one font
/// program with every checksum stated.
///
/// Table 124 files a TrueType program "as described in the TrueType Reference Manual", whose font
/// file is one font opening with its own font directory, and a `ttcf` collection is several (ADR
/// 2327). The face's own tables are copied byte for byte and no table of another face is taken;
/// what is written afresh is the directory, its search fields, every table's checksum and
/// `head`'s `checkSumAdjustment`, which the Manual's font directory states and a file this tree
/// hands to a reader does not leave false (ADR 2343). `None` where the bytes are no collection,
/// where no face carries the name, or where the face cannot be taken apart.
#[must_use]
pub fn collection_face(program: &[u8], font_name: &str) -> Option<Vec<u8>> {
    crate::sfnt::checksummed(crate::collection::extract_named(program, font_name)?)
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

/// The glyph name each glyph of an OpenType program's name-keyed `CFF ` table is selected by.
///
/// ISO 32000-2 §9.9.1 sends a `Type1` font dictionary over such a program through §9.6.5.2, which
/// looks a code's glyph name up in the CFF charset. A glyph is listed under a name only where that
/// lookup answers it: a charset naming two glyphs alike answers the first, so the second is
/// reached by no name and is left out. `None` where the program carries no `CFF ` table this crate
/// reads, or a CID-keyed one, whose charset names no glyph.
#[must_use]
pub fn wrapped_cff_glyph_names(program: &[u8]) -> Option<std::collections::BTreeMap<u16, String>> {
    match cff::CodeToGlyph::read(crate::sfnt::table(program, *b"CFF ")?).ok()? {
        cff::CodeToGlyph::Named(keyed) => Some(
            keyed
                .by_name
                .iter()
                .map(|(name, glyph)| (*glyph, name.as_ref().to_owned()))
                .collect(),
        ),
        cff::CodeToGlyph::Keyed { .. } => None,
    }
}

/// The units per em of an `OpenType` program's `CFF ` table, read off its `FontMatrix` as
/// [`cff::units_per_em`] reads a bare program's: the units [`with_empty_cff_glyphs`] takes its
/// advances in. `None` where the program carries no `CFF ` table this crate reads.
#[must_use]
pub fn wrapped_cff_units_per_em(program: &[u8]) -> Option<f32> {
    cff::units_per_em(sfnt_table(program, b"CFF ")?).ok()
}

/// An `OpenType` program with `CFF ` outlines, with one empty glyph appended per advance — the
/// construction [`cff::with_empty_glyphs`] makes of a bare program, carried through the tables an
/// sfnt counts its glyphs in — and the glyphs added.
///
/// The `CFF ` table gains the glyphs and their names, `maxp` counts them, and `hmtx` states each
/// one's advance and a zero left side bearing, written as long metrics for every glyph so that the
/// appended advances are their own rather than the last one repeated (`hhea`'s
/// `numberOfHMetrics`, OpenType's `hmtx`). Where the face carries vertical metrics, `vmtx` gains an
/// entry per glyph at the last advance height the table states, which `vhea` counts the same way.
/// The advances are in the program's units, as [`cff::with_empty_glyphs`] takes them.
/// Every glyph the program held keeps its index, its name, its charstring and its metrics.
///
/// `None` where the bytes are not an sfnt carrying a `CFF ` table and the metrics tables, or
/// where [`cff::with_empty_glyphs`] refuses the table.
#[must_use]
pub fn with_empty_cff_glyphs(
    program: &[u8],
    advances: &[i64],
) -> Option<(Vec<u8>, Vec<cff::AddedGlyph>)> {
    let (table, added) = cff::with_empty_glyphs(sfnt_table(program, b"CFF ")?, advances).ok()?;
    let held = glyph_count(program, Format::Sfnt)?;
    let count = held.checked_add(u16::try_from(added.len()).ok()?)?;
    let mut maxp = sfnt_table(program, b"maxp")?.to_vec();
    maxp.get_mut(NUM_GLYPHS..NUM_GLYPHS.checked_add(2)?)?
        .copy_from_slice(&count.to_be_bytes());
    let given: Vec<Option<u16>> = advances
        .iter()
        .map(|advance| u16::try_from(*advance).ok().map(Some))
        .collect::<Option<_>>()?;
    let (hhea, hmtx) = long_metrics(
        sfnt_table(program, b"hhea")?,
        sfnt_table(program, b"hmtx")?,
        held,
        &given,
    )?;
    let mut tables = vec![
        (*b"CFF ", table),
        (*b"maxp", maxp),
        (*b"hhea", hhea),
        (*b"hmtx", hmtx),
    ];
    if let (Some(vhea), Some(vmtx)) = (sfnt_table(program, b"vhea"), sfnt_table(program, b"vmtx")) {
        let (vhea, vmtx) = long_metrics(vhea, vmtx, held, &vec![None; given.len()])?;
        tables.extend([(*b"vhea", vhea), (*b"vmtx", vmtx)]);
    }
    Some((with_tables(program, &tables)?, added))
}

/// A metrics header and table (`hhea` and `hmtx`, or `vhea` and `vmtx`, which share a layout)
/// restated as one long metric per glyph: `held` glyphs as the table states them, then one per
/// entry of `added` at a zero side bearing — at the advance given, or at the last advance the
/// table states where none is.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every count is a glyph index under 2^16 and `long` is checked to lie in 1..=held, so \
              no offset passes 2^18 and no subtraction goes below zero"
)]
fn long_metrics(
    header: &[u8],
    table: &[u8],
    held: u16,
    added: &[Option<u16>],
) -> Option<(Vec<u8>, Vec<u8>)> {
    let read = |bytes: &[u8], at: usize| -> Option<u16> {
        Some(u16::from_be_bytes(
            bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
        ))
    };
    let long = usize::from(read(header, 34)?);
    let held = usize::from(held);
    if long == 0 || long > held {
        return None;
    }
    let last = read(table, (long - 1) * 4)?;
    let mut metrics: Vec<(u16, u16)> = Vec::with_capacity(held + added.len());
    for glyph in 0..held {
        metrics.push(if glyph < long {
            (read(table, glyph * 4)?, read(table, glyph * 4 + 2)?)
        } else {
            (last, read(table, long * 4 + (glyph - long) * 2)?)
        });
    }
    metrics.extend(added.iter().map(|advance| (advance.unwrap_or(last), 0)));
    let mut header = header.to_vec();
    let count = u16::try_from(metrics.len()).ok()?;
    header
        .get_mut(34..36)?
        .copy_from_slice(&count.to_be_bytes());
    let table = metrics
        .iter()
        .flat_map(|(advance, bearing)| {
            advance
                .to_be_bytes()
                .into_iter()
                .chain(bearing.to_be_bytes())
        })
        .collect();
    Some((header, table))
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
