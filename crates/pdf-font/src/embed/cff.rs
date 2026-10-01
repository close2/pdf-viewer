//! A machine face whose outlines are a `CFF ` table, written for §9.9.1's `/FontFile3`: as
//! §9.9.2's subset, a bare CID-keyed CFF under Table 125's `/Subtype /CIDFontType0C` (ADR 1449),
//! or whole as an `OpenType` program where the face forbids subsetting (ADR 1438). Either way the
//! descendant is a `CIDFontType0`.
//!
//! # Which row of Table 124
//!
//! Table 124 has two rows a `CFF ` face may go under a `CIDFontType0` by. `CIDFontType0C`: "Type 0
//! `CIDFont` program represented in the Compact Font Format (CFF), as described in Adobe Technical
//! Note #5176". And `OpenType`, whose `CFF ` cases each say "[i]n addition to the "CFF " table,
//! the font program shall include the "cmap" table".
//!
//! **A subset is written bare, under `CIDFontType0C`.** The wrapper would owe a `cmap` rebuilt for
//! the subset's glyphs, and `head`, `hhea`, `hmtx` and `maxp` restated for them, none of which a
//! reader of a `CIDFontType0` uses: §9.7.4.2 reaches its glyphs through the CFF's own charset and
//! §9.7.4.3 its widths through `/W`. The one thing the wrapper carries that the bare program does
//! not is `OS/2`'s `fsType`, and §9.9.1 already says what a reader does without it: "[i]n the
//! absence of explicit information to the contrary, embedded font programs shall be used only to
//! view and print the document and not for any other purposes". The licence is the writer's to
//! check — "[o]ne of the conditions may be that the font program cannot be embedded, in which
//! case it should not be incorporated into a PDF file" — and [`super::for_embedding`] checks it
//! before anything is written; what the file then permits a reader is the narrowest use.
//!
//! **A face that forbids subsetting is written whole**, as an `OpenType` program with the `cmap`
//! its row requires, beside the metric tables and `OS/2` (ADR 1438).
//!
//! # How a CID reaches a glyph
//!
//! The content stream a layout drew shows each glyph's index in the face as its CID, and it is
//! written as drawn. §9.7.4.2 then decides how that CID reaches a glyph:
//!
//! > The "CFF" font program has a Top DICT that uses CIDFont operators: The CIDs shall be used to
//! > determine the GID value for the glyph procedure using the charset table in the CFF program.
//!
//! So the subset is always CID-keyed, whatever the face was, and its charset gives each kept glyph
//! its index in the face as its CID: the stream is unchanged, and the subset's own glyph indices
//! are free to be dense. Table 117 states `/CIDToGIDMap` for "Type 2 `CIDFonts` with embedded font
//! programs" only, so no other route is available, and none is needed. The whole program keeps the
//! face's own charset, so there a CID-keyed face whose charset renumbers a shown glyph is refused
//! by name: under "[t]he CIDs shall be used directly as GID values" or the charset, one route
//! reaches the glyph and the other does not.
//!
//! # What a subset keeps
//!
//! - **The charstrings of the glyphs shown and of `.notdef`**, glyph 0 — §9.7.4.2's "[e]very
//!   `CIDFont` shall contain a glyph description for CID 0" and §9.9.2's "[t]he glyph name .notdef
//!   shall be defined in the font subset".
//! - **Every subroutine those reach**, found by [`super::reach`]. An unreached subroutine is
//!   replaced by a one-byte `endchar` rather than removed, so each INDEX keeps its count: Adobe
//!   Technical Note #5177 section 4.7 derives the bias a call adds back from the count, and a kept
//!   count keeps every call in every kept charstring right without rewriting any of them.
//!   An INDEX nothing reaches is written empty.
//! - **The Font DICTs the kept glyphs select**, renumbered in order, `FDSelect` rewritten in
//!   format 0 for the new glyph numbering, each with its Private DICT and its own local
//!   subroutines. A name-keyed face becomes one Font DICT holding its Private DICT.
//! - **The Top DICT** with `ROS` first, the face's descriptive entries (strings carried into the
//!   subset's String INDEX), a `CIDCount` one past the highest CID, and the four offsets.
//!
//! The `ROS` is the face's where it is CID-keyed and its charset already gives every kept glyph
//! its own index as its CID, and `Adobe-Identity-0` otherwise, because a glyph index is no
//! collection's CID. The name is §9.9.2's, "a tag followed by a plus sign (+) followed by the
//! PostScript name of the font from which the subset was created", and it is written into the
//! Name INDEX as well, because Table 117's `/BaseFont` for a Type 0 `CIDFont` "shall be the value
//! of the `CIDFontName` entry in the `CIDFont` program".
//!
//! A kept charstring in the accented-character form of `endchar` names two further glyphs by
//! standard code, which a CID-keyed program cannot carry; the subset holds in its place the
//! outline that form draws, composed by [`super::accented`] (ADR 1486). A face where that cannot
//! be done is written whole.

use std::collections::{BTreeMap, BTreeSet};

use skrifa::raw::FontRead as _;

use super::reach::{Reached, Walk};
use super::{Embedded, Outlines, Refusal, SystemInfo, assembled, postscript_name};
use crate::cff::{
    ESCAPE, OP_CHARSET, OP_CHARSTRINGS, OP_FD_ARRAY, OP_FD_SELECT, OP_PRIVATE, OP_SUBRS, Parts,
    dict_ints, int5, push_operator,
};
use crate::sfnt::sfnt_tables;

/// The tables written beside `CFF ` in the whole program: the `cmap` Table 124 requires, the
/// metrics a reader measures the face by, and `OS/2` so that the licence travels. Everything
/// else — the layout tables, the vertical metrics §9.9.1 says "shall never be used by a PDF
/// processor", variation data — is dropped, because the file draws only the glyphs its content
/// stream names.
const CARRIED: [[u8; 4]; 8] = [
    *b"CFF ", *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"OS/2", *b"post",
];

/// The sfnt version of an OpenType program whose outlines are CFF.
const OTTO: [u8; 4] = *b"OTTO";

/// Writes a `CFF ` face as the module documentation describes: a subset where `subsetting`, and
/// whole where the face's licence forbids one or a kept accented charstring cannot be composed.
///
/// # Errors
///
/// [`Refusal::Malformed`] where a table the write needs is absent or will not parse, and — for
/// the whole program only — [`Refusal::CidsAreNotGlyphs`] for a CID-keyed face whose charset
/// gives a glyph in `used` a CID other than its own index.
pub(super) fn for_embedding(
    program: &[u8],
    used: &BTreeSet<u16>,
    subsetting: bool,
) -> Result<Embedded, Refusal> {
    if subsetting && let Some(embedded) = subset(program, used)? {
        return Ok(embedded);
    }
    whole(program, used)
}

/// Whether the CID a layout shows for each of `glyphs` — its index in the face — reaches that
/// glyph when the face is read under a `CIDFontType0`: always for a name-keyed `CFF ` face, and for
/// a CID-keyed one where its charset gives each its own index as its CID (§9.7.4.2).
pub(super) fn cids_reach_glyphs(program: &[u8], glyphs: &BTreeSet<u16>) -> bool {
    let Ok(cff) = cff_table(program) else {
        return false;
    };
    match crate::cff::uses_cid_operators(cff) {
        Ok(false) => true,
        Ok(true) => match crate::cff::CodeToGlyph::read(cff) {
            Ok(crate::cff::CodeToGlyph::Keyed { by_cid }) => {
                glyphs.iter().all(|glyph| by_cid.get(glyph) == Some(glyph))
            }
            _ => false,
        },
        Err(_) => false,
    }
}

/// The face written whole under `OpenType` (ADR 1438).
fn whole(program: &[u8], used: &BTreeSet<u16>) -> Result<Embedded, Refusal> {
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
        if let Some(glyph) = used
            .iter()
            .find(|glyph| !cids_reach_glyphs(program, &BTreeSet::from([**glyph])))
        {
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

/// The one-byte charstring an unreached subroutine is replaced with: `endchar`.
///
/// The INDEX keeps its count, because the bias every call adds back is a function of the count
/// (Adobe Technical Note #5177, section 4.7) and a count that changed would move every call the
/// kept charstrings make. `endchar` rather than an empty item, so that a call the closure missed
/// finishes the glyph visibly early rather than running on into whatever follows.
const UNREACHED: &[u8] = &[14];

/// The registry, ordering and supplement of a subset whose CIDs are the face's glyph indices.
const IDENTITY: (&[u8], &[u8], i64) = (b"Adobe", b"Identity", 0);

/// Writes the subset the module documentation describes, or `None` where a kept charstring uses
/// the accented-character `endchar` and [`super::accented::composed`] cannot compose it.
fn subset(program: &[u8], used: &BTreeSet<u16>) -> Result<Option<Embedded>, Refusal> {
    let cff = cff_table(program)?;
    let parts = Parts::read(cff).ok_or(Refusal::Malformed("CFF "))?;
    let kept: BTreeSet<u16> = used.iter().copied().chain([0]).collect();
    if kept
        .iter()
        .any(|glyph| usize::from(*glyph) >= parts.charstrings.len())
    {
        return Err(Refusal::Malformed("CFF "));
    }
    let Some(closure) = Closure::of(cff, &parts, &kept)? else {
        return Ok(None);
    };
    let system = subset_system(cff, &parts, &kept)?;
    let name = format!(
        "{}+{}",
        super::tag(program, &kept),
        postscript_name(program)
    );
    let written = written(&parts, &closure, &system, name.as_bytes())?;
    Ok(Some(Embedded {
        program: written,
        glyphs: kept
            .iter()
            .zip(0u16..)
            .map(|(old, new)| (*old, new))
            .collect(),
        name,
        outlines: Outlines::CompactCid { system },
    }))
}

/// The `CFF ` table of an sfnt.
fn cff_table(program: &[u8]) -> Result<&[u8], Refusal> {
    let tables = sfnt_tables(program).ok_or(Refusal::Malformed("table directory"))?;
    let (at, length) = *tables
        .get(b"CFF ".as_slice())
        .ok_or(Refusal::Malformed("CFF "))?;
    program
        .get(at..at.checked_add(length).ok_or(Refusal::Malformed("CFF "))?)
        .ok_or(Refusal::Malformed("CFF "))
}

/// What a subset keeps: its glyphs with the Font DICT each is read against, and the subroutines
/// they reach.
struct Closure {
    /// The kept glyphs, in the order the subset numbers them, each with its Font DICT's index in
    /// the source.
    glyphs: Vec<(u16, usize)>,
    /// The Global Subr INDEX positions reached.
    global: BTreeSet<usize>,
    /// Per source Font DICT, the Local Subr INDEX positions reached.
    local: Vec<BTreeSet<usize>>,
    /// The charstrings written in place of a kept glyph's own: an accented character's composed
    /// outline (ADR 1486).
    composed: BTreeMap<u16, Vec<u8>>,
}

impl Closure {
    /// Walks every kept charstring of `cff` ([`super::reach`]); `None` where one is the accented
    /// form and cannot be composed.
    fn of(cff: &[u8], parts: &Parts<'_>, kept: &BTreeSet<u16>) -> Result<Option<Self>, Refusal> {
        let mut closure = Self {
            glyphs: Vec::with_capacity(kept.len()),
            global: BTreeSet::new(),
            local: vec![BTreeSet::new(); parts.font_dicts.len()],
            composed: BTreeMap::new(),
        };
        let mut everything = false;
        for glyph in kept {
            let font_dict = parts
                .font_dict_of(*glyph)
                .filter(|index| *index < parts.font_dicts.len())
                .ok_or(Refusal::Malformed("CFF "))?;
            let charstring = parts
                .charstrings
                .get(usize::from(*glyph))
                .ok_or(Refusal::Malformed("CFF "))?;
            let mut reached = Reached {
                global: &parts.global_subrs,
                local: &parts.font_dicts[font_dict].subrs,
                global_entered: BTreeSet::new(),
                local_entered: BTreeSet::new(),
            };
            match reached.walk(charstring) {
                Walk::Followed => {}
                Walk::Unfollowable => everything = true,
                Walk::Seac(accented) => {
                    let Some(outline) = super::accented::composed(cff, parts, &accented) else {
                        return Ok(None);
                    };
                    // The composed outline calls no subroutine, so none it entered is kept for it.
                    closure.composed.insert(*glyph, outline);
                    closure.glyphs.push((*glyph, font_dict));
                    continue;
                }
            }
            closure.global.extend(reached.global_entered);
            closure.local[font_dict].extend(reached.local_entered);
            closure.glyphs.push((*glyph, font_dict));
        }
        if everything {
            // A number the walk could not know may name any subroutine, so all are kept.
            closure.global = (0..parts.global_subrs.len()).collect();
            for (local, dict) in closure.local.iter_mut().zip(&parts.font_dicts) {
                *local = (0..dict.subrs.len()).collect();
            }
        }
        Ok(Some(closure))
    }

    /// The kept glyphs' charstrings in the subset's order: each glyph's own, or the outline
    /// composed in its place.
    fn charstrings<'a>(&'a self, parts: &Parts<'a>) -> Option<Vec<&'a [u8]>> {
        self.glyphs
            .iter()
            .map(|(glyph, _)| match self.composed.get(glyph) {
                Some(outline) => Some(outline.as_slice()),
                None => parts.charstrings.get(usize::from(*glyph)).copied(),
            })
            .collect()
    }
}

/// The character collection the subset states in its `ROS` and the PDF `CIDFont` copies.
///
/// The subset's CIDs are the face's glyph indices, since those are what the content stream
/// shows. A CID-keyed face whose charset already gives every kept glyph its own index as its CID
/// keeps its own `ROS`, since each of those CIDs still means what its collection says; any other
/// face's glyph indices are no collection's CIDs, and are stated as `Adobe-Identity-0`.
fn subset_system(
    cff: &[u8],
    parts: &Parts<'_>,
    kept: &BTreeSet<u16>,
) -> Result<SystemInfo, Refusal> {
    if parts.cid_keyed {
        let Ok(crate::cff::CodeToGlyph::Keyed { by_cid }) = crate::cff::CodeToGlyph::read(cff)
        else {
            return Err(Refusal::Malformed("CFF "));
        };
        if kept.iter().all(|glyph| by_cid.get(glyph) == Some(glyph)) {
            return system_info(cff).ok_or(Refusal::Malformed("CFF "));
        }
    }
    Ok(SystemInfo {
        registry: IDENTITY.0.to_vec(),
        ordering: IDENTITY.1.to_vec(),
        supplement: IDENTITY.2,
    })
}

/// The subset's String INDEX, built as the DICTs that name its strings are written.
#[derive(Default)]
struct Strings(Vec<Vec<u8>>);

impl Strings {
    /// The SID of `string` in the subset, added where it is not yet held.
    fn sid(&mut self, string: &[u8]) -> Result<i64, Refusal> {
        let at = if let Some(at) = self.0.iter().position(|held| held == string) {
            at
        } else {
            self.0.push(string.to_vec());
            self.0.len().saturating_sub(1)
        };
        i64::try_from(at)
            .ok()
            .and_then(|at| at.checked_add(STANDARD_STRINGS))
            .ok_or(Refusal::Malformed("CFF "))
    }

    /// A source DICT's SID operand restated for the subset: a standard string keeps its SID,
    /// and a custom one is carried into the subset's String INDEX.
    fn carried(&mut self, parts: &Parts<'_>, operands: &[u8]) -> Result<[u8; 5], Refusal> {
        let sid = dict_ints(operands)
            .first()
            .copied()
            .ok_or(Refusal::Malformed("CFF "))?;
        if sid < STANDARD_STRINGS {
            return Ok(int5(sid));
        }
        let custom = usize::try_from(sid.saturating_sub(STANDARD_STRINGS))
            .ok()
            .and_then(|at| parts.strings.get(at))
            .ok_or(Refusal::Malformed("CFF "))?;
        Ok(int5(self.sid(custom)?))
    }
}

/// How many strings Adobe Technical Note #5176's Appendix A predefines, SIDs 0 to 390, before a
/// program's own String INDEX begins.
const STANDARD_STRINGS: i64 = 391;

/// The Top DICT operators whose operand is a SID carried into the subset: `version`, `Notice`,
/// `FullName`, `FamilyName`, `Weight`, `Copyright` (Adobe Technical Note #5176, Table 9).
const CARRIED_STRINGS: [u16; 6] = [0, 1, 2, 3, 4, ESCAPE];

/// The Top DICT operators the subset states itself or does not state: `UniqueID` and `XUID`,
/// which name the whole face and not a subset of it; `charset`, `Encoding`, `CharStrings` and
/// `Private`, which are offsets or belong in a Font DICT; `SyntheticBase`, `PostScript`,
/// `BaseFontName` and `BaseFontBlend`, which describe relations to other fonts; and `ROS`,
/// `CIDCount`, `UIDBase`, `FDArray`, `FDSelect` and `FontName`, which are written fresh.
const RESTATED: [u16; 16] = [
    13,
    14,
    15,
    16,
    17,
    18,
    ESCAPE | 0x14,
    ESCAPE | 0x15,
    ESCAPE | 0x16,
    ESCAPE | 0x17,
    ESCAPE | 0x1e,
    ESCAPE | 0x22,
    ESCAPE | 0x23,
    OP_FD_ARRAY,
    OP_FD_SELECT,
    ESCAPE | 0x26,
];

/// `ROS` (Adobe Technical Note #5176, Table 10).
const OP_ROS: u16 = ESCAPE | 0x1e;
/// `CIDCount` (Table 10).
const OP_CID_COUNT: u16 = ESCAPE | 0x22;
/// A Font DICT's `FontName` (Table 10).
const OP_FONT_NAME: u16 = ESCAPE | 0x26;

/// Where the tables the Top DICT and the Font DICTs point at were placed.
#[derive(Clone, Copy, Default)]
struct Placed {
    charset: i64,
    fd_select: i64,
    charstrings: i64,
    fd_array: i64,
}

/// The subset program's bytes.
///
/// **Every offset is written in the five-byte integer form**, which is Adobe Technical Note
/// #5176's own way out of the circularity a DICT has: its offsets say where tables begin, and
/// where they begin depends on how long the DICT is. With every offset five bytes long, the
/// DICT's length is known before any offset is, so the layout is computed once from placeholder
/// DICTs and the DICTs are then written with the real numbers at the same length.
///
/// The order is the note's section 2: header, Name INDEX, Top DICT INDEX, String INDEX, Global
/// Subr INDEX, then the charset, `FDSelect`, `CharStrings`, `FDArray`, and each kept Font DICT's
/// Private DICT with its Local Subr INDEX directly after it.
fn written(
    parts: &Parts<'_>,
    closure: &Closure,
    system: &SystemInfo,
    name: &[u8],
) -> Result<Vec<u8>, Refusal> {
    const MALFORMED: Refusal = Refusal::Malformed("CFF ");
    // The source Font DICTs a kept glyph uses, renumbered in order.
    let font_dicts: Vec<usize> = closure
        .glyphs
        .iter()
        .map(|(_, font_dict)| *font_dict)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let new_font_dict = |source: usize| font_dicts.iter().position(|kept| *kept == source);

    let mut strings = Strings::default();
    let top_head = top_entries(parts, closure, system, &mut strings)?;
    let dicts_head = font_dict_entries(parts, &font_dicts, &mut strings)?;

    // The tables that hold no offsets, laid out once.
    let charset = {
        let mut out = vec![0u8];
        for (glyph, _) in closure.glyphs.iter().skip(1) {
            out.extend_from_slice(&glyph.to_be_bytes());
        }
        out
    };
    let fd_select = {
        let mut out = vec![0u8];
        for (_, font_dict) in &closure.glyphs {
            let new = new_font_dict(*font_dict).ok_or(MALFORMED)?;
            out.push(u8::try_from(new).map_err(|_| MALFORMED)?);
        }
        out
    };
    let charstrings = closure.charstrings(parts).ok_or(MALFORMED)?;
    let global = stubbed(&parts.global_subrs, &closure.global);
    let privates: Vec<(Vec<u8>, Vec<u8>)> = font_dicts
        .iter()
        .map(|source| {
            let dict = parts.font_dicts.get(*source).ok_or(MALFORMED)?;
            let reached = closure.local.get(*source).ok_or(MALFORMED)?;
            let subrs = stubbed(&dict.subrs, reached);
            let mut private = Vec::new();
            for (operands, op) in &dict.private {
                if *op != OP_SUBRS {
                    private.extend_from_slice(operands);
                    push_operator(&mut private, *op);
                }
            }
            if subrs.is_empty() {
                return Ok((private, Vec::new()));
            }
            // `Subrs` is relative to the Private DICT's own start, and the INDEX follows the
            // DICT directly, so the operand is the DICT's length with this entry in it.
            let length = private.len().saturating_add(6);
            private.extend_from_slice(&int5(i64::try_from(length).map_err(|_| MALFORMED)?));
            push_operator(&mut private, OP_SUBRS);
            Ok((private, index(&subrs).ok_or(MALFORMED)?))
        })
        .collect::<Result<_, Refusal>>()?;

    // Pass one, placeholders: every length is the final one.
    let top = top_dict(&top_head, Placed::default());
    let dicts = font_dicts_with(&dicts_head, &privates, 0)?;
    let header = [1u8, 0, 4, 4];
    let name_index = index(&[name]).ok_or(MALFORMED)?;
    let top_index = index(&[top.as_slice()]).ok_or(MALFORMED)?;
    let string_index = index(&strings.0).ok_or(MALFORMED)?;
    let global_index = index(&global).ok_or(MALFORMED)?;
    let charstring_index = index(&charstrings).ok_or(MALFORMED)?;
    let fd_array_len = index(&dicts).ok_or(MALFORMED)?.len();

    let offset = |length: usize| i64::try_from(length).map_err(|_| MALFORMED);
    let charset_at = header
        .len()
        .saturating_add(name_index.len())
        .saturating_add(top_index.len())
        .saturating_add(string_index.len())
        .saturating_add(global_index.len());
    let fd_select_at = charset_at.saturating_add(charset.len());
    let charstrings_at = fd_select_at.saturating_add(fd_select.len());
    let fd_array_at = charstrings_at.saturating_add(charstring_index.len());
    let privates_at = fd_array_at.saturating_add(fd_array_len);
    let placed = Placed {
        charset: offset(charset_at)?,
        fd_select: offset(fd_select_at)?,
        charstrings: offset(charstrings_at)?,
        fd_array: offset(fd_array_at)?,
    };

    // Pass two, the real offsets, at the lengths pass one measured.
    let top = top_dict(&top_head, placed);
    let dicts = font_dicts_with(&dicts_head, &privates, offset(privates_at)?)?;
    let mut out = Vec::with_capacity(privates_at.saturating_add(4096));
    out.extend_from_slice(&header);
    out.extend_from_slice(&name_index);
    out.extend_from_slice(&index(&[top.as_slice()]).ok_or(MALFORMED)?);
    out.extend_from_slice(&string_index);
    out.extend_from_slice(&global_index);
    out.extend_from_slice(&charset);
    out.extend_from_slice(&fd_select);
    out.extend_from_slice(&charstring_index);
    out.extend_from_slice(&index(&dicts).ok_or(MALFORMED)?);
    for (private, subrs) in &privates {
        out.extend_from_slice(private);
        out.extend_from_slice(subrs);
    }
    Ok(out)
}

/// A subroutine INDEX with every position the closure did not reach replaced by [`UNREACHED`],
/// or empty where it reached none — an INDEX nothing calls needs no count to keep a bias for.
fn stubbed<'a>(subrs: &[&'a [u8]], reached: &BTreeSet<usize>) -> Vec<&'a [u8]> {
    if reached.is_empty() {
        return Vec::new();
    }
    subrs
        .iter()
        .enumerate()
        .map(|(at, subr)| {
            if reached.contains(&at) {
                *subr
            } else {
                UNREACHED
            }
        })
        .collect()
}

/// The Top DICT's entries that hold no offset, `ROS` first as the note requires of a `CIDFont`,
/// then the source's own entries the subset keeps, then `CIDCount`.
fn top_entries(
    parts: &Parts<'_>,
    closure: &Closure,
    system: &SystemInfo,
    strings: &mut Strings,
) -> Result<Vec<u8>, Refusal> {
    let mut out = Vec::new();
    out.extend_from_slice(&int5(strings.sid(&system.registry)?));
    out.extend_from_slice(&int5(strings.sid(&system.ordering)?));
    out.extend_from_slice(&int5(system.supplement));
    push_operator(&mut out, OP_ROS);
    for (operands, op) in &parts.top {
        if RESTATED.contains(op) {
            continue;
        }
        if CARRIED_STRINGS.contains(op) {
            out.extend_from_slice(&strings.carried(parts, operands)?);
        } else {
            out.extend_from_slice(operands);
        }
        push_operator(&mut out, *op);
    }
    let highest = closure
        .glyphs
        .iter()
        .map(|(glyph, _)| *glyph)
        .max()
        .unwrap_or(0);
    out.extend_from_slice(&int5(i64::from(highest).saturating_add(1)));
    push_operator(&mut out, OP_CID_COUNT);
    Ok(out)
}

/// The Top DICT: [`top_entries`] and then the four offsets, each in the five-byte form.
fn top_dict(head: &[u8], placed: Placed) -> Vec<u8> {
    let mut out = head.to_vec();
    for (at, op) in [
        (placed.charset, OP_CHARSET),
        (placed.fd_select, OP_FD_SELECT),
        (placed.charstrings, OP_CHARSTRINGS),
        (placed.fd_array, OP_FD_ARRAY),
    ] {
        out.extend_from_slice(&int5(at));
        push_operator(&mut out, op);
    }
    out
}

/// Each kept Font DICT's entries other than `Private`: a CID-keyed source's own, `FontName`
/// carried into the subset's strings; nothing for the one a name-keyed source's Private DICT
/// sits under.
fn font_dict_entries(
    parts: &Parts<'_>,
    font_dicts: &[usize],
    strings: &mut Strings,
) -> Result<Vec<Vec<u8>>, Refusal> {
    font_dicts
        .iter()
        .map(|source| {
            let dict = parts
                .font_dicts
                .get(*source)
                .ok_or(Refusal::Malformed("CFF "))?;
            let mut out = Vec::new();
            for (operands, op) in &dict.entries {
                match *op {
                    OP_PRIVATE => continue,
                    OP_FONT_NAME => out.extend_from_slice(&strings.carried(parts, operands)?),
                    _ => out.extend_from_slice(operands),
                }
                push_operator(&mut out, *op);
            }
            Ok(out)
        })
        .collect()
}

/// The Font DICTs with their `Private` entries, each Private DICT placed in turn from `at`.
fn font_dicts_with(
    heads: &[Vec<u8>],
    privates: &[(Vec<u8>, Vec<u8>)],
    mut at: i64,
) -> Result<Vec<Vec<u8>>, Refusal> {
    heads
        .iter()
        .zip(privates)
        .map(|(head, (private, subrs))| {
            let size = i64::try_from(private.len()).map_err(|_| Refusal::Malformed("CFF "))?;
            let mut out = head.clone();
            out.extend_from_slice(&int5(size));
            out.extend_from_slice(&int5(at));
            push_operator(&mut out, OP_PRIVATE);
            let span = private.len().saturating_add(subrs.len());
            at = at.saturating_add(i64::try_from(span).map_err(|_| Refusal::Malformed("CFF "))?);
            Ok(out)
        })
        .collect()
}

/// An INDEX of `items` with the smallest offset size that holds its data (Adobe Technical Note
/// #5176, section 5); `None` past the 65 535 items a count can state.
fn index<T: AsRef<[u8]>>(items: &[T]) -> Option<Vec<u8>> {
    let count = u16::try_from(items.len()).ok()?;
    let mut out = count.to_be_bytes().to_vec();
    if count == 0 {
        return Some(out);
    }
    let data: usize = items.iter().map(|item| item.as_ref().len()).sum();
    let last = u32::try_from(data.checked_add(1)?).ok()?;
    let off_size: usize = match last {
        0..=0xff => 1,
        0x100..=0xffff => 2,
        0x1_0000..=0xff_ffff => 3,
        _ => 4,
    };
    out.push(u8::try_from(off_size).ok()?);
    let mut at = 1u32;
    let push = |value: u32, out: &mut Vec<u8>| {
        out.extend_from_slice(&value.to_be_bytes()[4usize.saturating_sub(off_size)..]);
    };
    push(at, &mut out);
    for item in items {
        at = at.checked_add(u32::try_from(item.as_ref().len()).ok()?)?;
        push(at, &mut out);
    }
    for item in items {
        out.extend_from_slice(item.as_ref());
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::super::{Outlines, Refusal, for_embedding};
    use super::{Closure, Parts, subset_system, written};
    use crate::cff::{CodeToGlyph, draw};
    use skrifa::MetadataProvider as _;
    use skrifa::outline::OutlinePen;
    use std::collections::{BTreeMap, BTreeSet};

    /// A `CFF ` face this machine holds, found by `fc-list`, or `None` where it holds none or
    /// has no `fc-list` (ADR 1154).
    fn machine_cff_face() -> Option<Vec<u8>> {
        let listed = std::process::Command::new("fc-list")
            .args([":fontformat=CFF", "file"])
            .output()
            .ok()?;
        let mut paths: Vec<String> = String::from_utf8_lossy(&listed.stdout)
            .lines()
            .filter_map(|line| line.split(':').next().map(str::to_owned))
            .collect();
        paths.sort();
        paths
            .into_iter()
            .filter_map(|path| std::fs::read(path).ok())
            .find(|program| {
                program.starts_with(b"OTTO") && super::super::permission(program).subsetting
            })
    }

    /// An outline as the pen was driven, every coordinate as drawn.
    #[derive(Default, Debug, PartialEq)]
    struct Recorded(Vec<(char, [f32; 6])>);
    impl OutlinePen for Recorded {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push(('M', [x, y, 0.0, 0.0, 0.0, 0.0]));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push(('L', [x, y, 0.0, 0.0, 0.0, 0.0]));
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            self.0.push(('Q', [cx, cy, x, y, 0.0, 0.0]));
        }
        fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
            self.0.push(('C', [cx0, cy0, cx1, cy1, x, y]));
        }
        fn close(&mut self) {
            self.0.push(('Z', [0.0; 6]));
        }
    }

    fn outline(cff: &[u8], glyph: u16) -> Recorded {
        let mut pen = Recorded::default();
        draw(cff, glyph, &mut pen).expect("the glyph draws");
        pen
    }

    /// Every kept glyph of `subset` draws exactly as its source glyph in `whole` does, through
    /// the crate's own CFF interpreter, and is reached from its old index as a CID through the
    /// subset's charset — §9.7.4.2's route for a Top DICT with `CIDFont` operators.
    fn assert_same_outlines(whole: &[u8], subset: &[u8], glyphs: &BTreeMap<u16, u16>) {
        let Ok(CodeToGlyph::Keyed { by_cid }) = CodeToGlyph::read(subset) else {
            panic!("the subset is CID-keyed");
        };
        for (old, new) in glyphs {
            assert_eq!(by_cid.get(old), Some(new), "CID {old} reaches its glyph");
            assert_eq!(outline(whole, *old), outline(subset, *new), "glyph {old}");
        }
    }

    // --- A fixture CFF, built by hand (Adobe Technical Note #5176), so every byte is known. ---

    /// `100 100 rmoveto 500 hlineto 500 vlineto -500 hlineto endchar`.
    const SQUARE: &[u8] = &[239, 239, 21, 248, 136, 6, 248, 136, 7, 252, 136, 6, 14];
    /// Operands that name subroutine 0, 1 and 2 of a small INDEX: the index less the bias of
    /// 107, as a one-byte operand (Adobe Technical Note #5177, section 3.2).
    const SUBR_0: u8 = 32;
    const SUBR_1: u8 = 33;
    const SUBR_2: u8 = 34;

    /// Global subroutines: `300 hlineto return`, `500 hlineto return`.
    fn gsubrs() -> Vec<Vec<u8>> {
        vec![vec![247, 192, 6, 11], vec![248, 136, 6, 11]]
    }

    /// Local subroutines: `200 vlineto return`, `500 vlineto return`, and one that calls global
    /// subroutine 0 and returns.
    fn lsubrs() -> Vec<Vec<u8>> {
        vec![
            vec![247, 92, 7, 11],
            vec![248, 136, 7, 11],
            vec![SUBR_0, 29, 11],
        ]
    }

    /// The fixture's glyphs: `.notdef`; the square; one calling global 1 and local 1; one
    /// reaching global 0 only through local 2, then local 0; and one whose `hintmask` data byte
    /// is 29, `callgsubr`'s opcode, which a walk reading it as an operator would follow.
    fn glyphs() -> Vec<Vec<u8>> {
        vec![
            vec![14],
            SQUARE.to_vec(),
            vec![239, 239, 21, SUBR_1, 29, SUBR_1, 10, 252, 136, 6, 14],
            vec![239, 239, 21, SUBR_2, 10, SUBR_0, 10, 14],
            vec![
                // Eight stems, then a mask of one byte, 0x1d.
                139, 149, 159, 149, 179, 149, 199, 149, 18, 139, 149, 159, 149, 179, 149, 199, 149,
                23, 19, 0x1d, 239, 239, 21, SUBR_1, 10, 14,
            ],
        ]
    }

    #[expect(
        clippy::arithmetic_side_effects,
        clippy::cast_possible_truncation,
        reason = "a fixture builder over a font of a few glyphs"
    )]
    fn index(items: &[Vec<u8>]) -> Vec<u8> {
        let mut out = (items.len() as u16).to_be_bytes().to_vec();
        if items.is_empty() {
            return out;
        }
        out.push(2);
        let mut at = 1u16;
        out.extend_from_slice(&at.to_be_bytes());
        for item in items {
            at += item.len() as u16;
            out.extend_from_slice(&at.to_be_bytes());
        }
        for item in items {
            out.extend_from_slice(item);
        }
        out
    }

    fn int5(value: usize) -> Vec<u8> {
        let mut out = vec![29];
        out.extend_from_slice(&u32::try_from(value).expect("small").to_be_bytes());
        out
    }

    /// A name-keyed CFF of [`glyphs`] with [`gsubrs`] and [`lsubrs`].
    #[expect(clippy::arithmetic_side_effects, reason = "a fixture builder")]
    fn name_keyed() -> Vec<u8> {
        let header = vec![1u8, 0, 4, 4];
        let names = index(&[b"Fixture".to_vec()]);
        let strings = index(&[b"Fixture Notice".to_vec()]);
        let global = index(&gsubrs());
        let charstrings = index(&glyphs());
        let mut private = int5(6);
        private.push(19);
        let local = index(&lsubrs());
        // Notice (SID 391), CharStrings, Private: 6 + 6 + 11 bytes.
        let top_len = 23;
        let top_index_len = 2 + 1 + 2 * 2 + top_len;
        let charstrings_at =
            header.len() + names.len() + top_index_len + strings.len() + global.len();
        let private_at = charstrings_at + charstrings.len();
        let mut top = int5(391);
        top.push(1);
        top.extend(int5(charstrings_at));
        top.push(17);
        top.extend(int5(private.len()));
        top.extend(int5(private_at));
        top.push(18);
        assert_eq!(top.len(), top_len);
        [
            header,
            names,
            index(&[top]),
            strings,
            global,
            charstrings,
            private,
            local,
        ]
        .concat()
    }

    /// A CID-keyed CFF of three glyphs under two Font DICTs, each with a local subroutine 0 of
    /// its own: glyph 1 under FD 0 draws `100 100 rmoveto`, its FD's subroutine (`200 vlineto`)
    /// and `endchar`; glyph 2 under FD 1 the same through FD 1's (`500 hlineto`). `cids` is the
    /// charset's CID for glyphs 1 and 2, and the `ROS` is `Adobe-Japan1-6`.
    #[expect(clippy::arithmetic_side_effects, reason = "a fixture builder")]
    fn cid_keyed(cids: [u16; 2]) -> Vec<u8> {
        let header = vec![1u8, 0, 4, 4];
        let names = index(&[b"FixtureCID".to_vec()]);
        let strings = index(&[b"Adobe".to_vec(), b"Japan1".to_vec()]);
        let global = index(&[]);
        let mut charset = vec![0u8];
        for cid in cids {
            charset.extend_from_slice(&cid.to_be_bytes());
        }
        // Format 3: glyph 0 and 1 under FD 0, glyph 2 under FD 1, sentinel 3.
        let fd_select = vec![3u8, 0, 2, 0, 0, 0, 0, 2, 1, 0, 3];
        let charstrings = index(&[
            vec![14],
            vec![239, 239, 21, SUBR_0, 10, 14],
            vec![239, 239, 21, SUBR_0, 10, 14],
        ]);
        // ROS, charset, FDSelect, CharStrings, FDArray: 17 + 6 + 7 + 6 + 7 bytes.
        let top_len = 43;
        let top_index_len = 2 + 1 + 2 * 2 + top_len;
        let charset_at = header.len() + names.len() + top_index_len + strings.len() + global.len();
        let fd_select_at = charset_at + charset.len();
        let charstrings_at = fd_select_at + fd_select.len();
        let mut private = int5(6);
        private.push(19);
        let subrs0 = index(&[vec![247, 92, 7, 11]]);
        let subrs1 = index(&[vec![248, 136, 6, 11]]);
        let private0_at = charstrings_at + charstrings.len();
        let private1_at = private0_at + private.len() + subrs0.len();
        let fd_array_at = private1_at + private.len() + subrs1.len();
        let mut top = int5(391);
        top.extend(int5(392));
        top.extend(int5(6));
        top.extend([12, 30]);
        top.extend(int5(charset_at));
        top.push(15);
        top.extend(int5(fd_select_at));
        top.extend([12, 37]);
        top.extend(int5(charstrings_at));
        top.push(17);
        top.extend(int5(fd_array_at));
        top.extend([12, 36]);
        assert_eq!(top.len(), top_len);
        let font_dict = |at: usize| {
            let mut dict = int5(private.len());
            dict.extend(int5(at));
            dict.push(18);
            dict
        };
        let fd_array = index(&[font_dict(private0_at), font_dict(private1_at)]);
        [
            header,
            names,
            index(&[top]),
            strings,
            global,
            charset,
            fd_select,
            charstrings,
            private.clone(),
            subrs0,
            private,
            subrs1,
            fd_array,
        ]
        .concat()
    }

    /// An OpenType program around `cff`, with an `OS/2` stating `fs_type` and a PostScript name.
    fn wrapped(cff: &[u8], fs_type: u16) -> Vec<u8> {
        let mut os2 = vec![0u8; 78];
        os2[0..2].copy_from_slice(&4u16.to_be_bytes());
        os2[8..10].copy_from_slice(&fs_type.to_be_bytes());
        let mut tables = BTreeMap::new();
        tables.insert(*b"CFF ", cff.to_vec());
        tables.insert(*b"OS/2", os2);
        tables.insert(*b"cmap", vec![0u8; 4]);
        tables.insert(*b"head", vec![0u8; 54]);
        super::super::assembled(&tables, super::OTTO).expect("the fixture lays out")
    }

    /// The subset's `CFF ` bytes and what the closure reached, for the name-keyed fixture.
    fn closure_of(used: &[u16]) -> (BTreeSet<usize>, BTreeSet<usize>) {
        let cff = name_keyed();
        let parts = Parts::read(&cff).expect("the fixture reads");
        let kept: BTreeSet<u16> = used.iter().copied().chain([0]).collect();
        let closure = Closure::of(&cff, &parts, &kept)
            .expect("walks")
            .expect("no accented endchar");
        (closure.global, closure.local[0].clone())
    }

    /// The closure follows calls through both INDEXes and through one another, and steps over a
    /// `hintmask`'s data rather than reading it as an operator.
    #[test]
    fn a_subset_keeps_the_subroutines_its_charstrings_reach() {
        assert_eq!(closure_of(&[1]), (BTreeSet::new(), BTreeSet::new()));
        assert_eq!(closure_of(&[2]), ([1].into(), [1].into()));
        assert_eq!(
            closure_of(&[3]),
            ([0].into(), [0, 2].into()),
            "global 0 is reached only through local 2"
        );
        assert_eq!(
            closure_of(&[4]),
            (BTreeSet::new(), [1].into()),
            "the mask byte 29 is data, not callgsubr"
        );
    }

    /// Every kept glyph of a subset draws as the whole face's does — byte-identical outlines —
    /// and a subset whose closure lost one subroutine does not: the comparison is what holds the
    /// closure to the glyphs.
    #[test]
    fn a_subset_draws_its_kept_glyphs_as_the_whole_face_does() {
        let cff = name_keyed();
        let program = wrapped(&cff, 0);
        let embedded = for_embedding(&program, &BTreeSet::from([2, 3, 4])).expect("installable");
        let expected: BTreeMap<u16, u16> = [(0, 0), (2, 1), (3, 2), (4, 3)].into();
        assert_eq!(embedded.glyphs, expected);
        assert!(!embedded.program.starts_with(b"OTTO"), "a bare CFF");

        assert_eq!(
            crate::cff::glyph_count(&embedded.program),
            Ok(4),
            "the square was not asked for and is gone"
        );
        assert_same_outlines(&cff, &embedded.program, &embedded.glyphs);
        let Outlines::CompactCid { system } = &embedded.outlines else {
            panic!("a CIDFontType0C program");
        };
        assert_eq!(
            (system.registry.as_slice(), system.ordering.as_slice()),
            (&b"Adobe"[..], &b"Identity"[..])
        );

        // The plant: the same subset written from a closure without global subroutine 0, which
        // glyph 3 reaches through local subroutine 2.
        let parts = Parts::read(&cff).expect("reads");
        let kept = BTreeSet::from([0, 3]);
        let mut closure = Closure::of(&cff, &parts, &kept)
            .expect("walks")
            .expect("no seac");
        // Global subroutine 1 stands in for it, so the INDEX keeps its count and subroutine 0 is
        // the one-byte `endchar` an unreached position is written as.
        assert!(closure.global.remove(&0));
        closure.global.insert(1);
        let system = subset_system(&cff, &parts, &kept).expect("a system");
        let planted = written(&parts, &closure, &system, b"ABCDEF+Fixture").expect("writes");
        assert_ne!(
            outline(&cff, 3),
            outline(&planted, 1),
            "a subset missing a reached subroutine draws the glyph differently"
        );
    }

    /// §9.9.2: the name is six uppercase letters, a plus sign and the face's PostScript name, and
    /// it is the program's own name too — Table 117's `/BaseFont` for a Type 0 `CIDFont` "shall be
    /// the value of the `CIDFontName` entry in the `CIDFont` program".
    #[test]
    fn a_subset_is_tagged_and_names_itself_as_the_file_does() {
        let embedded =
            for_embedding(&wrapped(&name_keyed(), 0), &BTreeSet::from([1])).expect("installable");
        let (tag, rest) = embedded.name.split_at(6);
        assert!(tag.bytes().all(|b| b.is_ascii_uppercase()), "{tag}");
        assert_eq!(
            rest, "+MachineFace",
            "the fixture states no PostScript name"
        );
        // The Name INDEX after the four-byte header: a count of one, an offset size, two offsets.
        let program = &embedded.program;
        let off_size = usize::from(program[6]);
        let start = 7 + 2 * off_size;
        assert_eq!(
            &program[start..start + embedded.name.len()],
            embedded.name.as_bytes()
        );
        let other =
            for_embedding(&wrapped(&name_keyed(), 0), &BTreeSet::from([2])).expect("installable");
        assert_ne!(
            other.name, embedded.name,
            "different subsets, different tags"
        );
    }

    /// A CID-keyed face keeps only the Font DICTs its kept glyphs select, `FDSelect` rebuilt to
    /// the new numbering, each Font DICT's own subroutines drawn under it; the `ROS` is the
    /// face's where its charset gives each kept glyph its own index as its CID, and
    /// `Adobe-Identity-0` where it does not.
    #[test]
    fn a_cid_keyed_subset_rebuilds_fd_select_and_keeps_each_font_dicts_subroutines() {
        let identity = cid_keyed([1, 2]);
        let embedded =
            for_embedding(&wrapped(&identity, 0), &BTreeSet::from([2])).expect("installable");
        assert_same_outlines(&identity, &embedded.program, &embedded.glyphs);
        let parts = Parts::read(&embedded.program).expect("reads");
        assert_eq!(
            parts.font_dicts.len(),
            2,
            "glyph 0 is under FD 0, glyph 2 under FD 1"
        );
        assert_eq!(parts.font_dict_of(1), Some(1));
        let Outlines::CompactCid { system } = &embedded.outlines else {
            panic!("a CIDFontType0C program");
        };
        assert_eq!(
            (system.ordering.as_slice(), system.supplement),
            (&b"Japan1"[..], 6)
        );

        let only_fd0 =
            for_embedding(&wrapped(&identity, 0), &BTreeSet::from([1])).expect("installable");
        let parts = Parts::read(&only_fd0.program).expect("reads");
        assert_eq!(parts.font_dicts.len(), 1, "FD 1 selected by no kept glyph");
        assert_same_outlines(&identity, &only_fd0.program, &only_fd0.glyphs);

        let renumbering = cid_keyed([7, 3]);
        let embedded =
            for_embedding(&wrapped(&renumbering, 0), &BTreeSet::from([1, 2])).expect("written");
        assert_same_outlines(&renumbering, &embedded.program, &embedded.glyphs);
        let Outlines::CompactCid { system } = &embedded.outlines else {
            panic!("a CIDFontType0C program");
        };
        assert_eq!(system.ordering, b"Identity");
    }

    /// A name-keyed CFF whose glyph 3, `Aacute`, is `450 30 600 65 194 endchar` — Appendix C's
    /// accented character over `A` (glyph 1, the square under an `hstem`) and `acute` (glyph 2,
    /// `200 0 rmoveto 100 hlineto 50 100 rlineto endchar`), its width 450 past a `nominalWidthX` of
    /// 100 — and whose glyph 4, `Bacute`, names `B`, which the face does not hold.
    #[expect(clippy::arithmetic_side_effects, reason = "a fixture builder")]
    fn accented_face() -> Vec<u8> {
        use skrifa::raw::ps::encoding::PredefinedEncoding;
        let sid = |code: u8| {
            PredefinedEncoding::Standard
                .sid(code)
                .expect("a standard code")
                .to_u16()
        };
        let header = vec![1u8, 0, 4, 4];
        let names = index(&[b"Accented".to_vec()]);
        let strings = index(&[b"Aacute".to_vec(), b"Bacute".to_vec()]);
        let global = index(&[]);
        let mut charset = vec![0u8];
        for glyph_sid in [sid(65), sid(194), 391, 392] {
            charset.extend_from_slice(&glyph_sid.to_be_bytes());
        }
        let charstrings = index(&[
            vec![14],
            [&[139, 189, 1][..], SQUARE].concat(),
            vec![247, 92, 139, 21, 239, 6, 189, 239, 5, 14],
            vec![248, 86, 169, 248, 236, 204, 247, 86, 14],
            vec![248, 86, 169, 248, 236, 205, 247, 86, 14],
        ]);
        // nominalWidthX 100.
        let private = vec![239, 21];
        // charset, CharStrings, Private: 6 + 6 + 11 bytes.
        let top_len = 23;
        let top_index_len = 2 + 1 + 2 * 2 + top_len;
        let charset_at = header.len() + names.len() + top_index_len + strings.len() + global.len();
        let charstrings_at = charset_at + charset.len();
        let private_at = charstrings_at + charstrings.len();
        let mut top = int5(charset_at);
        top.push(15);
        top.extend(int5(charstrings_at));
        top.push(17);
        top.extend(int5(private.len()));
        top.extend(int5(private_at));
        top.push(18);
        assert_eq!(top.len(), top_len);
        [
            header,
            names,
            index(&[top]),
            strings,
            global,
            charset,
            charstrings,
            private,
        ]
        .concat()
    }

    /// Appendix C's accented `endchar` names its components by standard code, which the CID-keyed
    /// subset cannot carry, so the subset holds the outline it draws: the accent's origin at
    /// `(adx, ady)` from the base's, the width the charstring states, and no component kept that
    /// was not asked for (ADR 1486).
    #[test]
    fn an_accented_character_is_subset_as_the_outline_it_draws() {
        let cff = accented_face();
        let embedded = for_embedding(&wrapped(&cff, 0), &BTreeSet::from([3])).expect("installable");
        assert!(
            matches!(embedded.outlines, Outlines::CompactCid { .. }),
            "subset, not whole"
        );
        assert_eq!(embedded.glyphs, [(0, 0), (3, 1)].into());
        assert_eq!(crate::cff::glyph_count(&embedded.program), Ok(2));

        // The accent's `200 0 rmoveto` from its origin at (30, 600), then the base's square from
        // its own at (0, 0): Appendix C's placement worked by hand.
        let drawn = outline(&embedded.program, 1);
        let moves: Vec<[f32; 2]> = drawn
            .0
            .iter()
            .filter(|(op, _)| *op == 'M')
            .map(|(_, p)| [p[0], p[1]])
            .collect();
        assert_eq!(moves, [[230.0, 600.0], [100.0, 100.0]]);
        assert_eq!(
            crate::cff::advances(&embedded.program, &[1]),
            Ok(vec![Some(550.0)]),
            "450 past the nominal width of 100"
        );

        // The reader this tree draws the whole face with composes the same outline.
        assert_same_outlines(&cff, &embedded.program, &embedded.glyphs);
        assert_eq!(
            crate::cff::advances(&cff, &[3]),
            crate::cff::advances(&embedded.program, &[1])
        );

        // With its components shown too, each keeps its own charstring, hints and all.
        let all = for_embedding(&wrapped(&cff, 0), &BTreeSet::from([1, 2, 3])).expect("written");
        assert!(matches!(all.outlines, Outlines::CompactCid { .. }));
        assert_same_outlines(&cff, &all.program, &all.glyphs);
    }

    /// An accented character whose base the face does not hold cannot be composed, so the face
    /// is written whole as before.
    #[test]
    fn an_accented_character_that_names_a_missing_glyph_keeps_the_face_whole() {
        let embedded = for_embedding(&wrapped(&accented_face(), 0), &BTreeSet::from([4]))
            .expect("installable");
        assert!(matches!(embedded.outlines, Outlines::Cff { .. }));
    }

    /// The face's licence decides what is written: restricted and bitmap-only faces are not;
    /// one that forbids subsetting is written whole under `OpenType`, with no tag.
    #[test]
    fn the_licence_decides_whether_a_cff_face_is_subset_written_whole_or_not_at_all() {
        let cff = name_keyed();
        let used = BTreeSet::from([2]);
        assert_eq!(
            for_embedding(&wrapped(&cff, 0x0002), &used),
            Err(Refusal::Restricted)
        );
        assert_eq!(
            for_embedding(&wrapped(&cff, 0x0200), &used),
            Err(Refusal::BitmapsOnly)
        );
        let whole = for_embedding(&wrapped(&cff, 0x0100), &used).expect("installable");
        assert!(whole.program.starts_with(b"OTTO"));
        assert!(matches!(whole.outlines, Outlines::Cff { .. }));
        assert_eq!(whole.name, "MachineFace");
        let subset = for_embedding(&wrapped(&cff, 0x0004), &used).expect("preview and print");
        assert!(matches!(subset.outlines, Outlines::CompactCid { .. }));
    }

    /// A `CFF ` face that forbids subsetting is written whole as an OpenType program: the
    /// `CFF ` table and the `cmap` Table 124 requires, no `glyf`, every glyph at its own index,
    /// the face's own name, and the licence carried in `OS/2` (ADR 1438).
    #[test]
    fn a_cff_face_is_written_whole_with_its_cmap_and_no_glyf() {
        let Some(program) = machine_cff_face() else {
            println!("skipped: this machine offers no CFF face (ADR 1154)");
            return;
        };
        let embedded = super::whole(&program, &BTreeSet::from([1u16, 2, 3])).expect("written");
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
        // An independent reader maps a character to the glyph the face maps it to, which is what
        // the `cmap` is carried for.
        let written = skrifa::FontRef::new(&embedded.program).expect("an OpenType program");
        let original = skrifa::FontRef::new(&program).expect("the face");
        assert_eq!(written.charmap().map('A'), original.charmap().map('A'));
    }

    /// A machine face subset to a few glyphs draws them as the whole face does, and so does a
    /// "subset" of every glyph — the closure walked over every charstring the face has.
    #[test]
    fn a_machine_cff_face_subset_draws_as_the_whole_face_does() {
        let Some(program) = machine_cff_face() else {
            println!("skipped: this machine offers no CFF face that permits subsetting (ADR 1154)");
            return;
        };
        let whole = super::cff_table(&program).expect("a CFF table").to_vec();
        let face = skrifa::FontRef::new(&program).expect("the face");
        let used: BTreeSet<u16> = "Aggregate"
            .chars()
            .filter_map(|c| face.charmap().map(c))
            .map(|glyph| u16::try_from(glyph.to_u32()).expect("a u16 glyph"))
            .collect();
        let embedded = for_embedding(&program, &used).expect("a permitted face");
        assert_same_outlines(&whole, &embedded.program, &embedded.glyphs);
        println!(
            "{} glyphs: {} bytes of CFF subset against {} of the whole table",
            embedded.glyphs.len(),
            embedded.program.len(),
            whole.len()
        );
        assert!(embedded.program.len() < whole.len() / 4);

        let count = u16::try_from(crate::cff::glyph_count(&whole).expect("counts")).expect("u16");
        let every: BTreeSet<u16> = (0..count).collect();
        let all = for_embedding(&program, &every).expect("a permitted face");
        assert_same_outlines(&whole, &all.program, &all.glyphs);
    }
}
