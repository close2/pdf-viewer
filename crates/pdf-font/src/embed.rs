//! A face from this machine written into a file: ISO 32000-2 §9.9.1's embedded `TrueType`
//! program, §9.9.2's subset of it, and what the face's own licence permits. A face whose outlines
//! are a `CFF ` table is written by [`cff`] instead, under `/FontFile3` and a `CIDFontType0`: as a
//! subset where the licence permits one (ADR 1449), whole where it does not (ADR 1438).
//!
//! A layout that sets a value no compiled-in face can draw sets it in a face the machine offers
//! (ADR 1414), and drawing it asks nothing of the face's licence or size. Writing it into a
//! document asks both, and this module is where they are answered, on the program's bytes alone —
//! the font dictionaries around it are `pdf-model`'s, as [`crate::embedding`] keeps them (ADR 1425).
//!
//! # The tables a program for a `CIDFontType2` carries
//!
//! §9.9.1 names them twice. Table 124's `/FontFile2` row: "The font program shall include these
//! tables: "glyf", "head", "hhea", "hmtx", "loca", and "maxp". The "cvt " (notice the trailing
//! SPACE), "fpgm", and "prep" tables shall also be included if they are required by the font
//! instructions." And the paragraph on `TrueType` programs under a `CIDFont`:
//!
//! > If used with a CIDFont dictionary, the "cmap" table is not needed and shall not be present,
//! > since the mapping from character codes to glyph descriptions is provided separately.
//!
//! So [`for_embedding`] writes the six, the three instruction tables wherever the face has them
//! (whether the glyphs' own instructions call into them is not something this reads, so it keeps
//! them rather than guessing), `OS/2` so that the licence travels with the program, and nothing
//! else: no `cmap`, and none of the tables that index by glyph (`hdmx`, `kern`, `post`, the layout
//! and variation tables) whose entries a renumbering would make false.
//!
//! # A subset, and its name
//!
//! §9.9.2: "For a font subset, the PostScript name of the font, that is, the value of the font's
//! `BaseFont` entry and the font descriptor's `FontName` entry, shall begin with a tag followed by a
//! plus sign (+) followed by the PostScript name of the font from which the subset was created.
//! The tag shall consist of exactly six uppercase letters; the choice of letters is arbitrary, but
//! different subsets of the same font in the same PDF file shall have different tags. The glyph
//! name .notdef shall be defined in the font subset." Glyph 0 is a `TrueType` program's `.notdef`
//! and is always kept; the tag is a digest of the program and of the glyphs kept, so two different
//! subsets differ in it and an identical one is named identically.

use std::collections::{BTreeMap, BTreeSet};

use crate::sfnt::{be16, be32, checksummed, horizontal_metric, sfnt_tables};

mod accented;
mod cff;
mod reach;

/// Offset of `indexToLocFormat` within `head`.
const INDEX_TO_LOC_FORMAT: usize = 50;

/// Offset of `numberOfHMetrics` within `hhea`.
const NUMBER_OF_H_METRICS: usize = 34;

/// Offset of `numGlyphs` within `maxp`.
const NUM_GLYPHS: usize = 4;

/// Offset of `fsType` within `OS/2`, the table's fourth field in every version.
const FS_TYPE: usize = 8;

/// What the face's `OS/2` table says may be done with it by a document that embeds it.
///
/// The OpenType specification's `OS/2` table, field `fsType`: bits 0 to 3 are the usage
/// permission — none set is installable embedding, bit 1 restricted-licence embedding (the face
/// may not be embedded at all), bit 2 preview-and-print, bit 3 editable — and in a table of
/// version 3 or later exactly one of them is set; in an earlier one several may be, and the
/// specification has a reader take the least restrictive. Bit 8 forbids subsetting, and bit 9
/// permits embedding the face's bitmaps only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Licence {
    /// No usage bit set, or no `OS/2` table: nothing is restricted.
    Installable,
    /// Bit 3: the face may be embedded and a document carrying it edited.
    Editable,
    /// Bit 2: the face may be embedded for viewing and printing the document.
    PreviewAndPrint,
    /// Bit 1 with neither of the two above: the face may not be embedded.
    Restricted,
}

/// A face's embedding permission, read from its `OS/2` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permission {
    /// The usage permission of bits 0 to 3.
    pub licence: Licence,
    /// Bit 8 clear: the face may be subset before it is embedded.
    pub subsetting: bool,
    /// Bit 9 set: only the face's bitmaps may be embedded, never its outlines.
    pub bitmaps_only: bool,
}

/// Reads [`Permission`] from an sfnt's `OS/2` table.
///
/// **A face with no `OS/2` table states no restriction, and is read as installable.** The table
/// is required of an OpenType font; a `TrueType` font from before it — the TrueType Reference
/// Manual's own, on which §9.9.1 bases `/FontFile2` — may lack it, and then carries no statement
/// in the program about embedding at all. §9.9.1 says the conditions are "recorded either in the
/// font program or as part of a separate license", and a separate licence is not something this
/// program can read; the choice is written down here rather than taken silently (ADR 1425).
#[must_use]
pub fn permission(program: &[u8]) -> Permission {
    let fs_type = sfnt_tables(program)
        .and_then(|tables| tables.get(b"OS/2".as_slice()).copied())
        .and_then(|(at, length)| {
            (length >= FS_TYPE.checked_add(2)?).then_some(())?;
            be16(program, at.checked_add(FS_TYPE)?)
        })
        .unwrap_or(0);
    // The least restrictive usage bit set wins, which for a table of version 3 or later — where
    // exactly one may be set — is simply that one.
    let licence = if fs_type & 0x0008 != 0 {
        Licence::Editable
    } else if fs_type & 0x0004 != 0 {
        Licence::PreviewAndPrint
    } else if fs_type & 0x0002 != 0 {
        Licence::Restricted
    } else {
        Licence::Installable
    };
    Permission {
        licence,
        subsetting: fs_type & 0x0100 == 0,
        bitmaps_only: fs_type & 0x0200 != 0,
    }
}

/// Why a face is not written into a file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// The face's licence forbids embedding it (`fsType`'s restricted-licence usage).
    #[error("the face's licence does not permit embedding it (its OS/2 fsType is restricted)")]
    Restricted,
    /// The face's licence permits embedding its bitmaps only, and what is drawn is its outlines.
    #[error("the face's licence permits embedding its bitmaps only (its OS/2 fsType)")]
    BitmapsOnly,
    /// The face has neither `glyf` nor `CFF ` outlines, so no row of Table 124 carries it.
    #[error("the face has neither glyf nor CFF outlines")]
    NoOutlines,
    /// A CID-keyed `CFF ` face's charset gives this glyph a CID other than its own index, so the
    /// CID the content stream shows would reach another glyph under §9.7.4.2 (ADR 1438).
    #[error(
        "the face is a CID-keyed CFF program whose charset gives glyph {0} another CID, and a \
         CIDFontType0 has no CIDToGIDMap to say otherwise"
    )]
    CidsAreNotGlyphs(u16),
    /// The face's tables could not be taken apart consistently.
    #[error("the face's {0} table could not be read")]
    Malformed(&'static str),
}

/// Which row of §9.9.1's Table 124 a written program belongs under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outlines {
    /// `glyf` outlines: `/FontFile2` under a `CIDFontType2`.
    TrueType,
    /// A `CFF ` table carried whole in an `OpenType` program: `/FontFile3` with
    /// `/Subtype /OpenType`, under a `CIDFontType0` (ADR 1438).
    Cff {
        /// The character collection a CID-keyed program's `ROS` names, which §9.7.4.2 says
        /// "should be copied into the PDF `CIDFont` dictionary"; `None` for a name-keyed one.
        system: Option<SystemInfo>,
    },
    /// A subset written as a bare CID-keyed CFF program: `/FontFile3` with
    /// `/Subtype /CIDFontType0C`, under a `CIDFontType0` whose CIDs its charset maps (ADR 1449).
    CompactCid {
        /// The character collection the program's `ROS` names, for `/CIDSystemInfo`.
        system: SystemInfo,
    },
}

/// A CID-keyed `CFF ` program's `ROS`: Table 114's `/Registry`, `/Ordering` and `/Supplement`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    /// The issuer of the character collection.
    pub registry: Vec<u8>,
    /// The character collection within the registry.
    pub ordering: Vec<u8>,
    /// The supplement number of the character collection.
    pub supplement: i64,
}

/// A program ready for a font file stream, and how its glyphs were renumbered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Embedded {
    /// The sfnt bytes, the stream's decoded data and Table 125's `/Length1`.
    pub program: Vec<u8>,
    /// Each glyph index of the face that was kept, to its index in [`Self::program`].
    pub glyphs: BTreeMap<u16, u16>,
    /// The name §9.9.2 gives a subset — six uppercase letters, a plus sign and the face's
    /// PostScript name — or the PostScript name alone where the face forbids subsetting and
    /// every glyph was kept.
    pub name: String,
    /// Which font file and which `CIDFont` subtype the program is written under.
    pub outlines: Outlines,
}

impl Embedded {
    /// Whether any kept glyph changed its index, which is when a `/CIDToGIDMap` of
    /// `/Identity` would no longer reach it.
    #[must_use]
    pub fn renumbered(&self) -> bool {
        self.glyphs.iter().any(|(old, new)| old != new)
    }
}

/// Whether a face's outlines are a `CFF ` table rather than `glyf`, which decides the `CIDFont`
/// it is described by: Table 124 carries a `CFF ` program under a `CIDFontType0` and a `glyf` one
/// under a `CIDFontType2`.
#[must_use]
pub fn has_cff_outlines(program: &[u8]) -> bool {
    sfnt_tables(program).is_some_and(|tables| {
        !tables.contains_key(b"glyf".as_slice()) && tables.contains_key(b"CFF ".as_slice())
    })
}

/// Whether each of `glyphs`, shown as a CID equal to its index in the face, reaches that glyph
/// under the `CIDFont` [`has_cff_outlines`] describes the face by — the question §9.7.4.2 answers
/// differently for the two `CFF ` cases. Always for a `glyf` face (`/CIDToGIDMap /Identity`) and a
/// name-keyed `CFF ` face ("[t]he CIDs shall be used directly as GID values"); for a CID-keyed one
/// only where its charset gives each its own index as its CID.
#[must_use]
pub fn glyph_indices_reach(program: &[u8], glyphs: &BTreeSet<u16>) -> bool {
    !has_cff_outlines(program) || cff::cids_reach_glyphs(program, glyphs)
}

/// The face written for a `CIDFontType2`'s `/FontFile2`, keeping `used` and what draws them.
///
/// `used` are glyph indices of `program`. Glyph 0 is added (§9.9.2's `.notdef`), and every
/// component a kept composite glyph names, to any depth — a composite whose components were
/// dropped would draw nothing. Where the face forbids subsetting, every glyph is kept at its own
/// index and only the tables §9.9.1 excludes are dropped.
///
/// # Errors
///
/// [`Refusal::Restricted`] and [`Refusal::BitmapsOnly`] where the face's licence stops it being
/// written — the font vendor's, not the document's, so no reader's level turns it off —
/// [`Refusal::NoOutlines`] for a program with neither `glyf` nor `CFF `,
/// [`Refusal::CidsAreNotGlyphs`] for a CID-keyed `CFF ` program [`cff`] cannot write so that the
/// stream's CIDs still reach their glyphs, and [`Refusal::Malformed`] where a table this needs is
/// absent or inconsistent.
///
/// A `CFF ` program is subset by [`cff`] under the same three answers of the licence: where
/// `fsType` forbids subsetting it is written whole, as a `glyf` one keeps every glyph.
pub fn for_embedding(program: &[u8], used: &BTreeSet<u16>) -> Result<Embedded, Refusal> {
    let permitted = permission(program);
    if permitted.licence == Licence::Restricted {
        return Err(Refusal::Restricted);
    }
    if permitted.bitmaps_only {
        return Err(Refusal::BitmapsOnly);
    }
    let tables = sfnt_tables(program).ok_or(Refusal::Malformed("table directory"))?;
    if !tables.contains_key(b"glyf".as_slice()) {
        return if tables.contains_key(b"CFF ".as_slice()) {
            cff::for_embedding(program, used, permitted.subsetting)
        } else {
            Err(Refusal::NoOutlines)
        };
    }
    let face = Face::read(program)?;
    let kept = if permitted.subsetting {
        face.closure(used)?
    } else {
        // Written whole, the face still holds every glyph the stream shows and `.notdef` with
        // them — the rule the closure applies to a subset (ADR 1495).
        if used.iter().chain([&0]).any(|glyph| *glyph >= face.count) {
            return Err(Refusal::Malformed("glyf"));
        }
        (0..face.count).collect()
    };
    let glyphs: BTreeMap<u16, u16> = kept
        .iter()
        .zip(0u16..)
        .map(|(old, new)| (*old, new))
        .collect();
    let program_out = assembled(&face.rebuilt(&glyphs)?, TRUE_TYPE)
        .ok_or(Refusal::Malformed("table directory"))?;
    let postscript = postscript_name(program);
    let name = if permitted.subsetting {
        format!("{}+{postscript}", tag(program, &kept))
    } else {
        postscript
    };
    Ok(Embedded {
        program: program_out,
        glyphs,
        name,
        outlines: Outlines::TrueType,
    })
}

/// The tables [`for_embedding`] reads, located once.
struct Face<'a> {
    /// The whole program, for the tables copied unchanged.
    program: &'a [u8],
    /// Tag to offset and length.
    tables: BTreeMap<Vec<u8>, (usize, usize)>,
    /// `head`, `hhea`, `maxp`, `hmtx`, `loca` and `glyf`, in that order.
    head: &'a [u8],
    hhea: &'a [u8],
    maxp: &'a [u8],
    hmtx: &'a [u8],
    loca: &'a [u8],
    glyf: &'a [u8],
    /// `maxp`'s `numGlyphs`.
    count: u16,
    /// Whether `loca` holds long offsets.
    long: bool,
    /// `hhea`'s `numberOfHMetrics`.
    pairs: usize,
}

impl<'a> Face<'a> {
    /// Locates the six tables Table 124's `/FontFile2` row names and the three counts they state.
    fn read(program: &'a [u8]) -> Result<Self, Refusal> {
        let tables = sfnt_tables(program).ok_or(Refusal::Malformed("table directory"))?;
        if !tables.contains_key(b"glyf".as_slice()) {
            return Err(Refusal::NoOutlines);
        }
        let table = |tag: &'static str| -> Result<&'a [u8], Refusal> {
            let (at, length) = *tables.get(tag.as_bytes()).ok_or(Refusal::Malformed(tag))?;
            program
                .get(at..at.checked_add(length).ok_or(Refusal::Malformed(tag))?)
                .ok_or(Refusal::Malformed(tag))
        };
        let (head, hhea, maxp, hmtx, loca, glyf) = (
            table("head")?,
            table("hhea")?,
            table("maxp")?,
            table("hmtx")?,
            table("loca")?,
            table("glyf")?,
        );
        let count = be16(maxp, NUM_GLYPHS).ok_or(Refusal::Malformed("maxp"))?;
        let long = match be16(head, INDEX_TO_LOC_FORMAT) {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(Refusal::Malformed("head")),
        };
        let pairs = usize::from(be16(hhea, NUMBER_OF_H_METRICS).ok_or(Refusal::Malformed("hhea"))?);
        if pairs == 0 {
            return Err(Refusal::Malformed("hhea"));
        }
        Ok(Self {
            program,
            tables,
            head,
            hhea,
            maxp,
            hmtx,
            loca,
            glyf,
            count,
            long,
            pairs,
        })
    }

    /// One glyph's `glyf` entry, as `loca` places it.
    fn outline(&self, glyph: u16) -> Result<&'a [u8], Refusal> {
        let offset = |index: usize| -> Option<usize> {
            if self.long {
                usize::try_from(be32(self.loca, index.checked_mul(4)?)?).ok()
            } else {
                usize::from(be16(self.loca, index.checked_mul(2)?)?).checked_mul(2)
            }
        };
        let index = usize::from(glyph);
        let start = offset(index).ok_or(Refusal::Malformed("loca"))?;
        let end = offset(index.checked_add(1).ok_or(Refusal::Malformed("loca"))?)
            .ok_or(Refusal::Malformed("loca"))?;
        self.glyf.get(start..end).ok_or(Refusal::Malformed("glyf"))
    }

    /// `used`, glyph 0, and every component a kept composite names, to any depth.
    ///
    /// Each glyph is expanded once, so a composite that names itself — which ADR 1411 finds
    /// describes no outline — ends the walk rather than looping it.
    fn closure(&self, used: &BTreeSet<u16>) -> Result<BTreeSet<u16>, Refusal> {
        let mut kept = BTreeSet::new();
        let mut pending: Vec<u16> = used.iter().copied().chain([0]).collect();
        while let Some(glyph) = pending.pop() {
            if glyph >= self.count {
                return Err(Refusal::Malformed("glyf"));
            }
            if !kept.insert(glyph) {
                continue;
            }
            for (_, component) in components(self.outline(glyph)?)? {
                if !kept.contains(&component) {
                    pending.push(component);
                }
            }
        }
        Ok(kept)
    }

    /// The tables of the program that keeps `glyphs`, each at its new index.
    fn rebuilt(&self, glyphs: &BTreeMap<u16, u16>) -> Result<BTreeMap<[u8; 4], Vec<u8>>, Refusal> {
        let mut glyf = Vec::new();
        let mut loca = Vec::with_capacity(glyphs.len().saturating_add(1).saturating_mul(4));
        let mut hmtx = Vec::with_capacity(glyphs.len().saturating_mul(4));
        for old in glyphs.keys() {
            loca.extend_from_slice(&offset_of(&glyf)?.to_be_bytes());
            let mut bytes = self.outline(*old)?.to_vec();
            for (at, component) in components(&bytes)? {
                let renumbered = glyphs.get(&component).ok_or(Refusal::Malformed("glyf"))?;
                bytes
                    .get_mut(at..at.saturating_add(2))
                    .ok_or(Refusal::Malformed("glyf"))?
                    .copy_from_slice(&renumbered.to_be_bytes());
            }
            glyf.extend_from_slice(&bytes);
            // Each glyph begins on a four-byte boundary, which long `loca` offsets permit and
            // ISO/IEC 14496-22 recommends.
            while !glyf.len().is_multiple_of(4) {
                glyf.push(0);
            }
            let (advance, bearing) = horizontal_metric(self.hmtx, self.pairs, usize::from(*old));
            hmtx.extend_from_slice(&advance.to_be_bytes());
            hmtx.extend_from_slice(&bearing.to_be_bytes());
        }
        loca.extend_from_slice(&offset_of(&glyf)?.to_be_bytes());
        let kept = u16::try_from(glyphs.len()).map_err(|_| Refusal::Malformed("maxp"))?;

        // Long offsets, whatever the face used: a four-byte-aligned `glyf` needs them past 128
        // KiB, and one format for every subset is one fewer case to be wrong in.
        let mut head = self.head.to_vec();
        patch(&mut head, INDEX_TO_LOC_FORMAT, 1, "head")?;
        let mut hhea = self.hhea.to_vec();
        patch(&mut hhea, NUMBER_OF_H_METRICS, kept, "hhea")?;
        let mut maxp = self.maxp.to_vec();
        patch(&mut maxp, NUM_GLYPHS, kept, "maxp")?;

        let mut out: BTreeMap<[u8; 4], Vec<u8>> = BTreeMap::new();
        out.insert(*b"head", head);
        out.insert(*b"hhea", hhea);
        out.insert(*b"maxp", maxp);
        out.insert(*b"hmtx", hmtx);
        out.insert(*b"loca", loca);
        out.insert(*b"glyf", glyf);
        for tag in [*b"cvt ", *b"fpgm", *b"prep", *b"OS/2"] {
            if let Some(&(at, length)) = self.tables.get(tag.as_slice()) {
                let bytes = self
                    .program
                    .get(at..at.saturating_add(length))
                    .ok_or(Refusal::Malformed("table directory"))?;
                out.insert(tag, bytes.to_vec());
            }
        }
        Ok(out)
    }
}

/// The glyph indices a composite `glyf` entry names, each with the byte offset it sits at.
///
/// Empty for a simple glyph and for an empty entry. The component records are ISO/IEC
/// 14496-22's: a flags word, the component's glyph index, two arguments of one or two bytes
/// each, an optional transformation of two, four or eight bytes, and bit 5 saying whether
/// another follows.
fn components(entry: &[u8]) -> Result<Vec<(usize, u16)>, Refusal> {
    const MALFORMED: Refusal = Refusal::Malformed("glyf");
    if entry.len() < 10 || i16::from_be_bytes([entry[0], entry[1]]) >= 0 {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    let mut at = 10usize;
    loop {
        let flags = be16(entry, at).ok_or(MALFORMED)?;
        let index_at = at.checked_add(2).ok_or(MALFORMED)?;
        found.push((index_at, be16(entry, index_at).ok_or(MALFORMED)?));
        at = index_at.checked_add(2).ok_or(MALFORMED)?;
        at = at
            .checked_add(if flags & 0x0001 != 0 { 4 } else { 2 })
            .ok_or(MALFORMED)?;
        at = at
            .checked_add(if flags & 0x0008 != 0 {
                2
            } else if flags & 0x0040 != 0 {
                4
            } else if flags & 0x0080 != 0 {
                8
            } else {
                0
            })
            .ok_or(MALFORMED)?;
        if flags & 0x0020 == 0 {
            return Ok(found);
        }
        // Every glyph index occupies two bytes of the entry, so a well-formed loop ends before
        // the entry does; one that has not is refused rather than followed.
        if found.len() > entry.len() / 2 {
            return Err(MALFORMED);
        }
    }
}

/// The current length of a `glyf` being built, as a long `loca` offset.
fn offset_of(glyf: &[u8]) -> Result<u32, Refusal> {
    u32::try_from(glyf.len()).map_err(|_| Refusal::Malformed("loca"))
}

/// Writes a big-endian `u16` into a copied table at a field's offset.
fn patch(table: &mut [u8], at: usize, value: u16, tag: &'static str) -> Result<(), Refusal> {
    table
        .get_mut(at..at.saturating_add(2))
        .ok_or(Refusal::Malformed(tag))?
        .copy_from_slice(&value.to_be_bytes());
    Ok(())
}

/// The sfnt version of a program whose outlines are `glyf`.
const TRUE_TYPE: [u8; 4] = 0x0001_0000_u32.to_be_bytes();

/// An sfnt laid out from its tables under `version`, in the tag order a directory is kept in,
/// every checksum stated.
fn assembled(tables: &BTreeMap<[u8; 4], Vec<u8>>, version: [u8; 4]) -> Option<Vec<u8>> {
    let count = u16::try_from(tables.len()).ok()?;
    // ISO/IEC 14496-22's three search fields, from the table count.
    let selector = u16::try_from(15u32.checked_sub(count.leading_zeros())?).ok()?;
    let search = 16u16.checked_mul(1u16.checked_shl(u32::from(selector))?)?;
    let mut out = version.to_vec();
    out.extend_from_slice(&count.to_be_bytes());
    out.extend_from_slice(&search.to_be_bytes());
    out.extend_from_slice(&selector.to_be_bytes());
    out.extend_from_slice(&count.checked_mul(16)?.checked_sub(search)?.to_be_bytes());
    let directory = 12usize.checked_add(usize::from(count).checked_mul(16)?)?;
    let mut body: Vec<u8> = Vec::new();
    for (tag, bytes) in tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(
            &u32::try_from(directory.checked_add(body.len())?)
                .ok()?
                .to_be_bytes(),
        );
        out.extend_from_slice(&u32::try_from(bytes.len()).ok()?.to_be_bytes());
        body.extend_from_slice(bytes);
        while !body.len().is_multiple_of(4) {
            body.push(0);
        }
    }
    out.extend_from_slice(&body);
    checksummed(out)
}

/// The face's PostScript name (`name` table, name ID 6), as a name may spell it.
///
/// Only the printable ASCII a PostScript name is made of is kept, without the characters §7.2.3
/// makes delimiters. A face stating none is named for what it is.
fn postscript_name(program: &[u8]) -> String {
    use skrifa::MetadataProvider as _;
    let stated = skrifa::FontRef::new(program)
        .ok()
        .and_then(|font| {
            font.localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
                .english_or_first()
                .map(|name| name.chars().collect::<String>())
        })
        .unwrap_or_default();
    let name: String = stated
        .chars()
        .filter(|c| c.is_ascii_graphic() && !"()<>[]{}/%#".contains(*c))
        .collect();
    if name.is_empty() {
        "MachineFace".to_owned()
    } else {
        name
    }
}

/// §9.9.2's six uppercase letters, a digest of the program and of the glyphs its subset keeps.
///
/// FNV-1a over the program's length, its bytes and the kept glyph indices: the letters are
/// arbitrary, as the clause allows, and deterministic, so a save of the same value writes the
/// same file.
fn tag(program: &[u8], kept: &BTreeSet<u16>) -> String {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    };
    for byte in u64::try_from(program.len())
        .unwrap_or(u64::MAX)
        .to_be_bytes()
    {
        feed(byte);
    }
    for byte in program {
        feed(*byte);
    }
    for glyph in kept {
        for byte in glyph.to_be_bytes() {
            feed(byte);
        }
    }
    (0..6)
        .map(|_| {
            // `hash % 26` is below 26, so the letter is one of `A` to `Z`.
            let letter = b'A'.saturating_add(u8::try_from(hash % 26).unwrap_or(0));
            hash /= 26;
            char::from(letter)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Licence, Refusal, for_embedding, permission, tag};
    use crate::sfnt::{be16, be32, sfnt_tables};
    use std::collections::{BTreeMap, BTreeSet};

    /// A simple glyph: one contour of three on-curve points, `mark` in its `xMax`.
    fn simple(mark: u8) -> Vec<u8> {
        let mut glyph = vec![0, 1, 0, 0, 0, 0, 0, mark, 0, 10];
        glyph.extend_from_slice(&2u16.to_be_bytes()); // endPtsOfContours
        glyph.extend_from_slice(&0u16.to_be_bytes()); // instructionLength
        glyph.extend_from_slice(&[0x01, 0x01, 0x01]); // three on-curve points, two-byte coordinates
        glyph.extend_from_slice(&[0, 0, 0, mark, 0, 0]); // x
        glyph.extend_from_slice(&[0, 0, 0, 10, 0, 0]); // y
        glyph
    }

    /// A composite of the named glyphs, byte offsets for its arguments.
    fn composite(of: &[u16]) -> Vec<u8> {
        let mut glyph = vec![0xff, 0xff, 0, 0, 0, 0, 0, 20, 0, 10];
        for (index, component) in of.iter().enumerate() {
            let more = if index.saturating_add(1) < of.len() {
                0x0020
            } else {
                0
            };
            glyph.extend_from_slice(&(0x0002u16 | more).to_be_bytes()); // ARGS_ARE_XY_VALUES
            glyph.extend_from_slice(&component.to_be_bytes());
            glyph.extend_from_slice(&[0, 0]);
        }
        glyph
    }

    /// An sfnt of `glyphs` (by index), with short `loca` offsets and an `OS/2` stating `fs_type`.
    fn face(glyphs: &[Vec<u8>], fs_type: u16) -> Vec<u8> {
        let mut glyf = Vec::new();
        let mut loca = Vec::new();
        let mut hmtx = Vec::new();
        for (index, glyph) in glyphs.iter().enumerate() {
            loca.extend_from_slice(&u16::try_from(glyf.len() / 2).expect("small").to_be_bytes());
            glyf.extend_from_slice(glyph);
            if glyf.len() % 2 == 1 {
                glyf.push(0);
            }
            // Advances of 100, 200, … so each glyph's metric is its own.
            let advance =
                u16::try_from(index.saturating_add(1).saturating_mul(100)).expect("small");
            hmtx.extend_from_slice(&advance.to_be_bytes());
            hmtx.extend_from_slice(&0u16.to_be_bytes());
        }
        loca.extend_from_slice(&u16::try_from(glyf.len() / 2).expect("small").to_be_bytes());
        let count = u16::try_from(glyphs.len()).expect("small");
        let mut head = vec![0u8; 54];
        head[18..20].copy_from_slice(&1000u16.to_be_bytes()); // unitsPerEm
        let mut hhea = vec![0u8; 36];
        hhea[34..36].copy_from_slice(&count.to_be_bytes());
        let mut maxp = vec![0u8; 6];
        maxp[0..4].copy_from_slice(&0x0000_5000u32.to_be_bytes());
        maxp[4..6].copy_from_slice(&count.to_be_bytes());
        let mut os2 = vec![0u8; 78];
        os2[0..2].copy_from_slice(&4u16.to_be_bytes());
        os2[8..10].copy_from_slice(&fs_type.to_be_bytes());
        let mut tables = BTreeMap::new();
        tables.insert(*b"OS/2", os2);
        tables.insert(*b"cmap", vec![0u8; 4]);
        tables.insert(*b"glyf", glyf);
        tables.insert(*b"head", head);
        tables.insert(*b"hhea", hhea);
        tables.insert(*b"hmtx", hmtx);
        tables.insert(*b"loca", loca);
        tables.insert(*b"maxp", maxp);
        super::assembled(&tables, super::TRUE_TYPE).expect("the fixture lays out")
    }

    fn glyf_entry(program: &[u8], glyph: usize) -> Vec<u8> {
        let tables = sfnt_tables(program).expect("a directory");
        let (loca, _) = tables[b"loca".as_slice()];
        let (glyf, _) = tables[b"glyf".as_slice()];
        let at = |index: usize| {
            let entry = loca.saturating_add(index.saturating_mul(4));
            usize::try_from(be32(program, entry).expect("in range")).expect("small")
        };
        let (start, end) = (at(glyph), at(glyph.saturating_add(1)));
        program[glyf.saturating_add(start)..glyf.saturating_add(end)].to_vec()
    }

    /// A subset keeps `.notdef`, the glyphs asked for and every component they name, renumbered
    /// in order, with the composite's component indices rewritten and each metric carried.
    #[test]
    fn a_subset_keeps_notdef_and_the_composite_closure_renumbered() {
        // 0 .notdef, 1..=5 simple, 6 a composite of 2 and 4, 7 a composite of 6 and 5.
        let mut glyphs: Vec<Vec<u8>> = (0..6).map(simple).collect();
        glyphs.push(composite(&[2, 4]));
        glyphs.push(composite(&[6, 5]));
        let program = face(&glyphs, 0);
        let embedded = for_embedding(&program, &BTreeSet::from([7, 1])).expect("installable");

        let expected: BTreeMap<u16, u16> =
            [(0, 0), (1, 1), (2, 2), (4, 3), (5, 4), (6, 5), (7, 6)].into();
        assert_eq!(embedded.glyphs, expected);
        assert!(embedded.renumbered());
        let out = &embedded.program;
        let tables = sfnt_tables(out).expect("the subset is an sfnt");
        let tags: Vec<&[u8]> = tables.keys().map(Vec::as_slice).collect();
        assert_eq!(
            tags,
            [
                &b"OS/2"[..],
                b"glyf",
                b"head",
                b"hhea",
                b"hmtx",
                b"loca",
                b"maxp"
            ],
            "no cmap under a CIDFont (§9.9.1)"
        );
        let (maxp, _) = tables[b"maxp".as_slice()];
        assert_eq!(be16(out, maxp + 4), Some(7));
        let (head, _) = tables[b"head".as_slice()];
        assert_eq!(be16(out, head + 50), Some(1), "long offsets");
        // The old glyph 7 is new 6, naming old 6 and 5 as new 5 and 4.
        let entry = glyf_entry(out, 6);
        assert_eq!(be16(&entry, 12), Some(5));
        assert_eq!(be16(&entry, 18), Some(4));
        // Old 4, now 3, is still the simple glyph marked 4, with old 4's advance of 500.
        assert_eq!(glyf_entry(out, 3)[7], 4);
        let (hmtx, _) = tables[b"hmtx".as_slice()];
        assert_eq!(be16(out, hmtx + 3 * 4), Some(500));
        assert!(
            skrifa::FontRef::new(out).is_ok(),
            "a reader opens the subset"
        );
        assert_eq!(embedded.name.len(), "ABCDEF+MachineFace".len());
        assert_eq!(embedded.name.as_bytes()[6], b'+');
        assert!(embedded.name[..6].bytes().all(|b| b.is_ascii_uppercase()));
    }

    /// Different subsets of one face get different tags (§9.9.2), and one subset the same tag.
    #[test]
    fn different_subsets_are_tagged_differently() {
        let program = face(&(0..4).map(simple).collect::<Vec<_>>(), 0);
        let one = tag(&program, &BTreeSet::from([0, 1]));
        assert_eq!(one, tag(&program, &BTreeSet::from([0, 1])));
        assert_ne!(one, tag(&program, &BTreeSet::from([0, 2])));
    }

    /// `fsType`'s restricted-licence usage stops the write, and so does bitmaps-only.
    #[test]
    fn a_face_whose_licence_forbids_embedding_is_not_written() {
        let glyphs: Vec<Vec<u8>> = (0..3).map(simple).collect();
        assert_eq!(
            for_embedding(&face(&glyphs, 0x0002), &BTreeSet::from([1])),
            Err(Refusal::Restricted)
        );
        assert_eq!(
            for_embedding(&face(&glyphs, 0x0200), &BTreeSet::from([1])),
            Err(Refusal::BitmapsOnly)
        );
        // An earlier table may set several usage bits, and the least restrictive is read.
        assert_eq!(
            permission(&face(&glyphs, 0x0006)).licence,
            Licence::PreviewAndPrint
        );
        assert_eq!(
            permission(&face(&glyphs, 0x0008)).licence,
            Licence::Editable
        );
        assert!(for_embedding(&face(&glyphs, 0x0004), &BTreeSet::from([1])).is_ok());
    }

    /// A face that forbids subsetting keeps every glyph at its own index, under its plain name.
    #[test]
    fn a_face_that_forbids_subsetting_keeps_every_glyph() {
        let glyphs: Vec<Vec<u8>> = (0..5).map(simple).collect();
        let embedded =
            for_embedding(&face(&glyphs, 0x0100), &BTreeSet::from([3])).expect("installable");
        assert_eq!(embedded.glyphs.len(), 5);
        assert!(!embedded.renumbered());
        assert_eq!(embedded.name, "MachineFace");
    }

    /// A face written whole is refused a glyph it does not hold, as a subset is: one past its
    /// last, and every glyph of a face holding none at all, which has no `.notdef` (the `embed`
    /// fuzz target's finding, ADR 1495).
    #[test]
    fn a_face_written_whole_is_refused_a_glyph_it_does_not_hold() {
        let glyphs: Vec<Vec<u8>> = (0..5).map(simple).collect();
        assert_eq!(
            for_embedding(&face(&glyphs, 0x0100), &BTreeSet::from([5])),
            Err(Refusal::Malformed("glyf")),
            "one past the last glyph, written whole"
        );
        assert_eq!(
            for_embedding(&face(&glyphs, 0x0000), &BTreeSet::from([5])),
            Err(Refusal::Malformed("glyf")),
            "the same glyph, subset"
        );
        assert!(
            matches!(
                for_embedding(&face(&[], 0x0100), &BTreeSet::new()),
                Err(Refusal::Malformed(_))
            ),
            "a face of no glyphs has no .notdef"
        );
    }
}
