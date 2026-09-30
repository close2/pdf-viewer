//! A machine face whose outlines are a `CFF ` table, written for §9.9.1's `/FontFile3` with
//! Table 125's `/Subtype /OpenType` under a `CIDFontType0` (ADR 1438).
//!
//! # Which subtype, and which font dictionary
//!
//! Table 124's `OpenType` row names the dictionaries such a program may appear under: a
//! `CIDFontType0` where the `CFF ` table's Top DICT uses `CIDFont` operators, and a `Type1` font or
//! a `CIDFontType0` where it does not — and of both it says "[i]n addition to the "CFF " table, the
//! font program shall include the "cmap" table". So both kinds of `CFF ` face are written as an
//! `OpenType` program under a `CIDFontType0`, never under the `CIDFontType2` a `glyf` face takes.
//!
//! # How a CID reaches a glyph, and why that decides what may be written
//!
//! The content stream a layout drew shows each glyph's index as its CID, and it is written as it
//! was drawn. §9.7.4.2 then decides whether that CID still reaches the glyph:
//!
//! > The "CFF" font program has a Top DICT that does not use `CIDFont` operators: The CIDs shall be
//! > used directly as GID values
//!
//! which holds for every name-keyed face, and, for a CID-keyed one, "[t]he CIDs shall be used to
//! determine the GID value for the glyph procedure using the charset table in the CFF program" —
//! which holds only where the charset gives each glyph written its own index as its CID. A
//! `CIDFontType0` has no `/CIDToGIDMap` to repair the difference (Table 117 states that entry for
//! Type 2 `CIDFonts` only), so a CID-keyed face whose charset renumbers a glyph the value shows is
//! refused by name rather than written to draw other glyphs.
//!
//! # The program is carried whole
//!
//! Subsetting a `CFF ` table is rebuilding its `CharStrings` INDEX, its charset and, for a
//! CID-keyed face, its `FDSelect`, with every local and global subroutine a kept charstring calls
//! — its own project. So the `CFF ` table goes into the file whole, every glyph at its own index,
//! beside the `cmap` the row requires and the metric tables a reader measures the face by. The
//! cost is the face's outline data, measured on this machine's faces: Source Sans 3's regular
//! weight writes 190 221 bytes of tables, 162 621 of them `CFF `, and Noto Sans Duployan 631 423,
//! 595 731 of them `CFF `; `machine_cff_face_written.rs` saves five Duployan glyphs in a file of
//! 252 423 bytes once `FlateDecode` has had it. The name is the face's PostScript name alone,
//! since §9.9.2's tag is for a subset and this is none.

use std::collections::{BTreeMap, BTreeSet};

use skrifa::raw::FontRead as _;

use super::{Embedded, Outlines, Refusal, SystemInfo, assembled, postscript_name};
use crate::sfnt::sfnt_tables;

/// The tables written beside `CFF `: the `cmap` Table 124 requires, the metrics a reader measures
/// the face by, and `OS/2` so that the licence travels. Everything else — the layout tables, the
/// vertical metrics §9.9.1 says "shall never be used by a PDF processor", variation data — is
/// dropped, because the file draws only the glyphs its content stream names.
const CARRIED: [[u8; 4]; 8] = [
    *b"CFF ", *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"OS/2", *b"post",
];

/// The sfnt version of an OpenType program whose outlines are CFF.
const OTTO: [u8; 4] = *b"OTTO";

/// Writes a `CFF ` face whole, as the module documentation describes.
///
/// # Errors
///
/// [`Refusal::Malformed`] where a table the write needs is absent or will not parse, and
/// [`Refusal::CidsAreNotGlyphs`] for a CID-keyed face whose charset gives a glyph in `used` a CID
/// other than its own index.
pub(super) fn for_embedding(program: &[u8], used: &BTreeSet<u16>) -> Result<Embedded, Refusal> {
    let tables = sfnt_tables(program).ok_or(Refusal::Malformed("table directory"))?;
    let table = |tag: &[u8; 4]| -> Option<&[u8]> {
        let (at, length) = *tables.get(tag.as_slice())?;
        program.get(at..at.checked_add(length)?)
    };
    let cff = table(b"CFF ").ok_or(Refusal::Malformed("CFF "))?;
    if table(b"cmap").is_none() {
        return Err(Refusal::Malformed("cmap"));
    }
    let count = crate::cff::glyph_count(cff).map_err(|_| Refusal::Malformed("CFF "))?;
    let count = u16::try_from(count).map_err(|_| Refusal::Malformed("CFF "))?;

    let cid_keyed = crate::cff::uses_cid_operators(cff).map_err(|_| Refusal::Malformed("CFF "))?;
    let system = if cid_keyed {
        let Ok(crate::cff::CodeToGlyph::Keyed { by_cid }) = crate::cff::CodeToGlyph::read(cff)
        else {
            return Err(Refusal::Malformed("CFF "));
        };
        if let Some(glyph) = used.iter().find(|glyph| by_cid.get(glyph) != Some(glyph)) {
            return Err(Refusal::CidsAreNotGlyphs(*glyph));
        }
        Some(system_info(cff).ok_or(Refusal::Malformed("CFF "))?)
    } else {
        None
    };

    let mut out: BTreeMap<[u8; 4], Vec<u8>> = BTreeMap::new();
    for tag in CARRIED {
        if let Some(bytes) = table(&tag) {
            out.insert(tag, bytes.to_vec());
        }
    }
    Ok(Embedded {
        program: assembled(&out, OTTO).ok_or(Refusal::Malformed("table directory"))?,
        glyphs: (0..count).map(|glyph| (glyph, glyph)).collect(),
        name: postscript_name(program),
        outlines: Outlines::Cff { system },
    })
}

/// A CID-keyed Top DICT's `ROS` operator: the character collection §9.7.4.2 says the program
/// identifies, and which "should be copied into the PDF `CIDFont` dictionary".
fn system_info(cff: &[u8]) -> Option<SystemInfo> {
    use skrifa::raw::ps::cff::dict::{Entry, entries};
    let font = skrifa::raw::tables::cff::Cff::read(skrifa::raw::FontData::new(cff)).ok()?;
    let top = font.top_dicts().get(0).ok()?;
    entries(top, None).find_map(|entry| match entry {
        Ok(Entry::Ros {
            registry,
            ordering,
            supplement,
        }) => Some(SystemInfo {
            registry: font.string(registry)?.to_vec(),
            ordering: font.string(ordering)?.to_vec(),
            supplement: i64::from(supplement.to_i32()),
        }),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{Outlines, for_embedding};
    use skrifa::MetadataProvider as _;
    use std::collections::BTreeSet;

    /// A `CFF ` face this machine holds, found by `fc-list`, or `None` where it holds none or
    /// has no `fc-list` (ADR 1154).
    fn machine_cff_face() -> Option<Vec<u8>> {
        let listed = std::process::Command::new("fc-list")
            .args([":fontformat=CFF", "file"])
            .output()
            .ok()?;
        String::from_utf8_lossy(&listed.stdout)
            .lines()
            .filter_map(|line| line.split(':').next())
            .filter_map(|path| std::fs::read(path).ok())
            .find(|program| program.starts_with(b"OTTO"))
    }

    /// A `CFF ` face is written whole as an OpenType program: the `CFF ` table and the `cmap`
    /// Table 124 requires, no `glyf`, every glyph at its own index, the face's own name, and the
    /// licence carried in `OS/2` (ADR 1438).
    #[test]
    fn a_cff_face_is_written_whole_with_its_cmap_and_no_glyf() {
        let Some(program) = machine_cff_face() else {
            println!("skipped: this machine offers no CFF face (ADR 1154)");
            return;
        };
        let used = BTreeSet::from([1u16, 2, 3]);
        let embedded = match for_embedding(&program, &used) {
            Ok(embedded) => embedded,
            Err(refusal) => {
                // A face whose own licence forbids the write is a refusal this test does not
                // test; the licence reading is `permission`'s tests'.
                println!("skipped: the machine's first CFF face is refused: {refusal}");
                return;
            }
        };
        assert!(embedded.program.starts_with(b"OTTO"));
        let tables = crate::sfnt::sfnt_tables(&embedded.program).expect("a directory");
        for tag in [b"CFF ", b"cmap", b"OS/2"] {
            assert!(tables.contains_key(tag.as_slice()), "{tag:?}");
        }
        for tag in [b"glyf", b"GSUB", b"GPOS", b"vmtx"] {
            assert!(!tables.contains_key(tag.as_slice()), "{tag:?}");
        }
        assert!(!embedded.renumbered());
        assert!(!embedded.name.contains('+'), "no subset, so no tag");
        assert!(matches!(embedded.outlines, Outlines::Cff { .. }));
        // An independent reader opens the written program and maps a character to the glyph the
        // face maps it to, which is what the `cmap` is carried for.
        let written = skrifa::FontRef::new(&embedded.program).expect("an OpenType program");
        let original = skrifa::FontRef::new(&program).expect("the face");
        assert_eq!(written.charmap().map('A'), original.charmap().map('A'));
    }
}
