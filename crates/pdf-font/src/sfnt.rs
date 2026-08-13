//! An sfnt's table directory: what it says the program holds, and the repairs a `glyf` and
//! `loca` pair sometimes need.
//!
//! Everything here reads bytes a document supplied, and two of these functions *write* at
//! offsets computed from those bytes — which is why [`repaired_font_program`] is public. It is
//! the door `fuzz/fuzz_targets/sfnt.rs` knocks on, because a repair is a parser and gets
//! fuzzed (ADR 0175).
//!
//! §9.6.5.4's route from a character code to a glyph in the same container is
//! [`crate::truetype`]'s, and the tests of both modules are there: the fixtures that build an
//! sfnt serve them equally, and one copy of that builder is easier to keep right than two.

use std::borrow::Cow;
use std::collections::BTreeMap;

/// How many bytes an `sfnt`'s own table directory says the program has, when that is more than
/// it has.
///
/// `None` for a program that is whole, which is the ordinary case.
///
/// # Why this is worth a check of its own
///
/// A truncated program does not fail in a way that names itself. `skrifa` reads the directory,
/// finds a record pointing past the end, and reports the *table* as missing — so two corpus
/// documents were refused for eighty sessions with "units per em is zero", which is what
/// `metrics()` answers when it cannot find `head`. Both are simply short:
/// `bug1050040.pdf` holds 45 240 bytes of a program whose directory describes 59 210, and
/// `issue11651.pdf` holds 512 bytes of a ten-table font. §9.9 Table 124 requires the program to
/// "include these tables: \"glyf\", \"head\", \"hhea\", \"hmtx\", \"loca\", and \"maxp\"", and every
/// one of them is *named* in these directories — what is absent is the bytes.
///
/// **A report is only as good as the condition it fires on** (trap 11), and this is the same
/// rule about a report's *wording*: a diagnosis nobody can act on is a silence with a sentence
/// in front of it.
pub(crate) fn truncation(data: &[u8]) -> Option<(String, u64)> {
    /// Offset of the first table record, after the twelve-byte directory header.
    const RECORDS: usize = 12;
    /// Bytes per table record: tag, checksum, offset, length.
    const RECORD: usize = 16;
    /// The one table of §9.9 Table 124's six whose absence stops the program drawing at all.
    ///
    /// **The condition was narrowed four times and each time by a document**, which is trap 11
    /// on a report's condition rather than on its wording. Counting every record refused two
    /// pages that draw: `issue3405r.pdf` carries a junk record for a table nobody reads,
    /// putting the program's end at 3.3 GB. Counting `glyf`, then `loca` and `hmtx`, then
    /// `maxp` refused a third, `issue13316_reduced.pdf`, which was reduced by cutting its font
    /// short — all four are read *per glyph* or not at all, so a cut costs the glyphs beyond
    /// it and no more, the same graceful loss §9.7.6.3 describes for an undefined character.
    ///
    /// `head` is different in kind: it carries the units per em and `indexToLocFormat`, so
    /// without it there is no scale to place a glyph at and no way to read `loca`. That is
    /// what this tree was already refusing — as "units per em is zero", which is what
    /// `metrics()` answers when it cannot find the table. The refusal is unchanged; what is
    /// new is that it says why.
    const REQUIRED: [&[u8; 4]; 1] = [b"head"];

    let count = usize::from(u16::from_be_bytes([*data.get(4)?, *data.get(5)?]));
    for index in 0..count {
        let at = RECORDS.checked_add(index.checked_mul(RECORD)?)?;
        let field = |offset: usize| -> Option<u32> {
            let bytes =
                data.get(at.checked_add(offset)?..at.checked_add(offset)?.checked_add(4)?)?;
            Some(u32::from_be_bytes([
                *bytes.first()?,
                *bytes.get(1)?,
                *bytes.get(2)?,
                *bytes.get(3)?,
            ]))
        };
        let tag = data.get(at..at.checked_add(4)?)?;
        if !REQUIRED.iter().any(|required| *required == tag) {
            continue;
        }
        let end = u64::from(field(8)?).checked_add(u64::from(field(12)?))?;
        if end > data.len() as u64 {
            return Some((String::from_utf8_lossy(tag).into_owned(), end));
        }
    }
    None
}

/// Repairs a byte-swapped `indexToLocFormat`, returning the corrected bytes.
///
/// # Why this is a derivation and not a heuristic
///
/// `issue2537r.pdf` embeds a 60-glyph Helvetica-Bold subset whose `head` table states
/// `indexToLocFormat` as **0x0100**. The field is defined by ISO/IEC 14496-22 to be 0 (short
/// offsets) or 1 (long); 0x0100 is 1 written in the wrong byte order and is neither. `skrifa`
/// reads it strictly and reaches the wrong offsets, so the page drew `.notdef` boxes where
/// three references draw `LINE UP` — and reported nothing, because the font loaded and
/// produced *some* glyphs.
///
/// The file says which format it is, twice, in its own table directory, and that is what this
/// function reads rather than guessing:
///
/// - the last `loca` entry is the length of `glyf` — 2056 here under the long reading and 0
///   under the short one, against a `glyf` table of 2056 bytes;
/// - `loca` holds `numGlyphs + 1` entries, so its length is `2 × (n + 1)` or `4 × (n + 1)` —
///   244 here, which is the long form for 60 glyphs and twice the short form's 122.
///
/// Both agree, and only one format satisfies either. So this is the same shape as the
/// twenty-seventh session's LZW finding: **a file that states one fact twice can check
/// itself**, and no other implementation's behaviour is involved.
///
/// Returns `None` when the field is already 0 or 1, when the tables it needs are absent or
/// short, or when *neither* reading satisfies both tests — in which case the font is broken in
/// a way this cannot name, and `skrifa`'s own answer stands.
pub(crate) fn repaired_loca_format(data: &[u8]) -> Option<Vec<u8>> {
    /// Offset of `indexToLocFormat` within the `head` table.
    const INDEX_TO_LOC: usize = 50;

    let be16 = |at: usize| -> Option<u16> {
        let bytes = data.get(at..at.checked_add(2)?)?;
        Some(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]))
    };
    let be32 = |at: usize| -> Option<u32> {
        let bytes = data.get(at..at.checked_add(4)?)?;
        Some(u32::from_be_bytes([
            *bytes.first()?,
            *bytes.get(1)?,
            *bytes.get(2)?,
            *bytes.get(3)?,
        ]))
    };

    let count = be16(4)?;
    let directory = 12usize.checked_add(usize::from(count).checked_mul(16)?)?;
    let mut tables = BTreeMap::new();
    for index in 0..usize::from(count) {
        let entry = 12usize.checked_add(index.checked_mul(16)?)?;
        let tag = data.get(entry..entry.checked_add(4)?)?.to_vec();
        let offset = usize::try_from(be32(entry.checked_add(8)?)?).ok()?;
        let length = usize::try_from(be32(entry.checked_add(12)?)?).ok()?;
        // No table begins inside the table directory; see [`sfnt_tables`] for what a font
        // that says otherwise did to this function.
        if offset < directory {
            return None;
        }
        // One tag, one table; see [`sfnt_tables`].
        if tables.insert(tag, (offset, length)).is_some() {
            return None;
        }
    }

    let (head, head_length) = *tables.get(b"head".as_slice())?;
    // **The field must lie inside the table that owns it.** A directory is bytes a document
    // supplied, so nothing stops one naming a `head` that overlaps the directory itself — and
    // the patch below writes two bytes at a computed offset. `fuzz/fuzz_targets/sfnt.rs` found
    // exactly that: a `head` pointing into the table directory, where correcting
    // `indexToLocFormat` scribbled over a table's *tag* and handed the caller a font whose
    // directory this repair had damaged. Confining the write to `head`'s own stated extent is
    // the condition that makes "this rewrites two tables" true rather than intended.
    if head_length < INDEX_TO_LOC.checked_add(2)? {
        return None;
    }
    let stated = be16(head.checked_add(INDEX_TO_LOC)?)?;
    if stated <= 1 {
        return None;
    }

    let (loca, loca_length) = *tables.get(b"loca".as_slice())?;
    let (_, glyf_length) = *tables.get(b"glyf".as_slice())?;
    let (maxp, _) = *tables.get(b"maxp".as_slice())?;
    let glyphs = usize::from(be16(maxp.checked_add(4)?)?);
    let entries = glyphs.checked_add(1)?;

    // Short offsets are stored halved, which is why the last one is doubled to compare.
    let short = be16(loca.checked_add(entries.checked_sub(1)?.checked_mul(2)?)?)
        .map(|value| usize::from(value).checked_mul(2));
    let long = be32(loca.checked_add(entries.checked_sub(1)?.checked_mul(4)?)?)
        .and_then(|value| usize::try_from(value).ok());
    let fits = |width: usize, last: Option<usize>| {
        last == Some(glyf_length) && entries.checked_mul(width) == Some(loca_length)
    };

    let corrected: u16 = if fits(2, short.flatten()) {
        0
    } else if fits(4, long) {
        1
    } else {
        return None;
    };

    let mut repaired = data.to_vec();
    let slot = repaired.get_mut(
        head.checked_add(INDEX_TO_LOC)?..head.checked_add(INDEX_TO_LOC)?.checked_add(2)?,
    )?;
    slot.copy_from_slice(&corrected.to_be_bytes());
    Some(repaired)
}

/// A big-endian `u16` at a byte offset, or `None` past the end.
fn be16(data: &[u8], at: usize) -> Option<u16> {
    let bytes = data.get(at..at.checked_add(2)?)?;
    Some(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]))
}

/// A big-endian `u32` at a byte offset, or `None` past the end.
pub(crate) fn be32(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_be_bytes([
        *bytes.first()?,
        *bytes.get(1)?,
        *bytes.get(2)?,
        *bytes.get(3)?,
    ]))
}

/// An sfnt's table directory: tag to offset and length.
pub(crate) fn sfnt_tables(data: &[u8]) -> Option<BTreeMap<Vec<u8>, (usize, usize)>> {
    let count = be16(data, 4)?;
    let directory = 12usize.checked_add(usize::from(count).checked_mul(16)?)?;
    let mut tables = BTreeMap::new();
    for index in 0..usize::from(count) {
        let entry = 12usize.checked_add(index.checked_mul(16)?)?;
        let tag = data.get(entry..entry.checked_add(4)?)?.to_vec();
        let offset = usize::try_from(be32(data, entry.checked_add(8)?)?).ok()?;
        let length = usize::try_from(be32(data, entry.checked_add(12)?)?).ok()?;
        // **No table begins inside the table directory.** The directory is bytes a document
        // supplied and the two repairs *write* at offsets computed from it, so a `head`
        // pointing at the directory turns "correct `indexToLocFormat`" into "scribble on a
        // table's tag" — which `fuzz/fuzz_targets/sfnt.rs` produced within a minute of being
        // seeded with real fonts. Refusing the whole directory rather than the one entry is
        // deliberate: a font that overlaps itself is not one this can reason about, and
        // `skrifa`'s own answer for it stands.
        if offset < directory {
            return None;
        }
        // **A tag names one table.** A directory that repeats one leaves this map holding the
        // *last* entry while [`rewritten_sfnt`] patches the *first*, so the repair would write
        // one entry and read another — and `repaired_font_program` would find work to do on
        // its own output, for ever. The fuzz target's idempotence assertion is what caught it;
        // refusing the font is the same answer as the overlap above, and for the same reason.
        if tables.insert(tag, (offset, length)).is_some() {
            return None;
        }
    }
    Some(tables)
}

/// A copy of an sfnt with some tables replaced, by **appending** the new data and repointing.
///
/// Appending rather than rebuilding the file keeps every other table where it was, which is what
/// lets a caller go on using offsets it read before the repair — `repaired_loca_order` patches
/// `head` afterwards for exactly that reason. The old bytes stay in the file unreferenced, which
/// costs the size of the table being replaced and nothing else; `checkSumAdjustment` is left
/// alone, as it is by every producer that edits a font in place.
fn rewritten_sfnt(
    data: &[u8],
    tables: &BTreeMap<Vec<u8>, (usize, usize)>,
    replacements: &[(&[u8; 4], Vec<u8>)],
) -> Option<Vec<u8>> {
    let count = usize::from(be16(data, 4)?);
    let mut out = data.to_vec();
    for (tag, bytes) in replacements {
        // Which directory entry names this table, found by tag rather than by position: the
        // directory is sorted by tag and a caller has no business assuming where one sits.
        let entry = (0..count).find(|index| {
            12usize
                .checked_add(index.checked_mul(16).unwrap_or(usize::MAX))
                .and_then(|at| data.get(at..at.checked_add(4)?))
                .is_some_and(|found| found == tag.as_slice())
        })?;
        let at = 12usize.checked_add(entry.checked_mul(16)?)?;
        let offset = u32::try_from(out.len()).ok()?;
        let length = u32::try_from(bytes.len()).ok()?;
        out.extend_from_slice(bytes);
        // Every table in an sfnt begins on a four-byte boundary.
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
        out.get_mut(at.checked_add(8)?..at.checked_add(12)?)?
            .copy_from_slice(&offset.to_be_bytes());
        out.get_mut(at.checked_add(12)?..at.checked_add(16)?)?
            .copy_from_slice(&length.to_be_bytes());
    }
    let _ = tables;
    Some(out)
}

/// Applies both of the sfnt repairs to a font program, returning the bytes to load.
///
/// The two are [`repaired_loca_format`] — a byte-swapped `indexToLocFormat` — and
/// [`repaired_loca_order`] — a `loca` whose offsets do not ascend — and they compose in that
/// order because the second reads the format the first corrects. Returns the input unchanged
/// where neither applies, which is every well-formed font.
///
/// # Why this is public when `LoadedFont::load` is the only caller in the tree
///
/// **A font program is untrusted input and this is a parser over it**, which `CLAUDE.md`
/// principle 3 says gets fuzzed from its first commit. Both repairs walk a table directory, a
/// `loca` and a `glyf` taken from bytes a document supplied, and both *rewrite* an sfnt — so the
/// door they need is one a fuzz target can knock on, and `fuzz/fuzz_targets/sfnt.rs` is what
/// knocks. The alternative, fuzzing through `LoadedFont::load`, would need a whole `Document`
/// around every input and would spend nearly all its budget in the parser it already has a
/// target for.
#[must_use]
pub fn repaired_font_program(data: &[u8]) -> Cow<'_, [u8]> {
    let formatted = repaired_loca_format(data).map_or(Cow::Borrowed(data), Cow::Owned);
    match repaired_loca_order(&formatted) {
        Some(ordered) => Cow::Owned(ordered),
        None => formatted,
    }
}

/// Rebuilds a `glyf` and `loca` pair whose offsets do not ascend, returning the corrected bytes.
///
/// # Why a rebuild rather than [`repaired_loca_format`]'s two bytes
///
/// The glyph table's own standard defines a glyph's data as running from `loca[i]` to
/// `loca[i + 1]`, which
/// makes the offsets ascending by construction. `issue11131_reduced.pdf` embeds a 71-glyph
/// `CIDFontType2` subset whose table is the right *shape* — 72 long entries, and the last of them
/// is `glyf`'s length — and whose contents begin
///
/// ```text
/// 16776  16776  16776  16776  10674  2188  2590  1886
/// ```
///
/// **36 of the 71 glyphs therefore state a negative length**, `read-fonts` refuses each of them,
/// and the page drew half its sentence with nothing reported: a font that produces *some* glyphs
/// is a font that loaded. The three reference renderers built on `FreeType` draw the whole
/// sentence, because it takes the entry's extent from the entry rather than from the pair.
///
/// Nothing here is recoverable by changing one number: `loca[i + 1]` is also glyph `i + 1`'s
/// start and is right as such. The glyphs sit in `glyf` in an order the offsets do not follow, so
/// the repair is to put them in one.
///
/// # Why it is a derivation and not a guess
///
/// **A `glyf` entry is self-describing**, which is what makes each glyph's true length readable
/// from its own bytes: `numberOfContours` decides simple or composite, a simple glyph's extent
/// follows from its contour count, instruction length and flag stream, and a composite's from its
/// component loop. So the file states each glyph's extent twice — once in `loca`, once in the
/// entry — and only one of the two readings is self-consistent. The same shape as
/// [`repaired_loca_format`] and as the twenty-seventh session's LZW finding, one table over.
///
/// Glyph ids do not move, so a composite's references to other glyphs stay valid, and every other
/// table is copied through unchanged.
///
/// Returns `None` when the offsets already ascend — which is every well-formed font, so this
/// costs one pass over `loca` and nothing else on the common path — and when any glyph's own
/// bytes cannot be read, in which case the font is damaged in a way this cannot name and
/// `skrifa`'s own answer stands.
pub(crate) fn repaired_loca_order(data: &[u8]) -> Option<Vec<u8>> {
    let tables = sfnt_tables(data)?;
    let (head, head_length) = *tables.get(b"head".as_slice())?;
    let (maxp, _) = *tables.get(b"maxp".as_slice())?;
    let (loca_at, loca_length) = *tables.get(b"loca".as_slice())?;
    let (glyf_at, glyf_length) = *tables.get(b"glyf".as_slice())?;

    let long = be16(data, head.checked_add(50)?)? == 1;
    let glyphs = usize::from(be16(data, maxp.checked_add(4)?)?);
    let entries = glyphs.checked_add(1)?;
    let width = if long { 4 } else { 2 };
    if entries.checked_mul(width)? > loca_length {
        return None;
    }

    let offset = |index: usize| -> Option<usize> {
        let at = loca_at.checked_add(index.checked_mul(width)?)?;
        if long {
            usize::try_from(be32(data, at)?).ok()
        } else {
            Some(usize::from(be16(data, at)?).checked_mul(2)?)
        }
    };

    // The common path: one pass, no allocation, and every well-formed font leaves here.
    let starts: Vec<usize> = (0..entries).map(offset).collect::<Option<_>>()?;
    if starts.windows(2).all(|pair| pair[0] <= pair[1]) {
        return None;
    }

    let glyf = data.get(glyf_at..glyf_at.checked_add(glyf_length)?)?;
    let mut rebuilt: Vec<u8> = Vec::with_capacity(glyf_length);
    let mut offsets: Vec<u32> = Vec::with_capacity(entries);
    for (index, start) in starts.get(..glyphs)?.iter().enumerate() {
        offsets.push(u32::try_from(rebuilt.len()).ok()?);
        // **An empty glyph stays empty.** The glyph table's own standard says a glyph with no
        // outline is written by giving it and its successor the same offset, and that statement
        // is self-consistent whatever the rest of the table does — unlike a *descending* pair,
        // which is what this repair exists to overrule. Reading such a glyph's length from its
        // own bytes hands it whichever entry happens to begin there, which is a real glyph.
        //
        // `issue7074_reduced.pdf` is the witness and it is `issue11131_reduced.pdf`'s font one
        // defect over: `loca` runs 0, 108, 0, 108, 108, 282, … so glyph 3 — the space, under
        // `Identity-H` — has start 108 and successor 108, and the entry at 108 is glyph 4. The
        // page drew `Our|2015|Graduates`, a narrow mark where each space belongs, while three
        // references drew the spaces.
        if starts.get(index.checked_add(1)?) == Some(start) {
            continue;
        }
        // An offset past the table is what a well-formed file uses for an empty glyph, and so is
        // a length of zero; both arrive here as nothing appended.
        let Some(length) = glyph_length(glyf, *start) else {
            continue;
        };
        rebuilt.extend_from_slice(glyf.get(*start..start.checked_add(length)?)?);
        // A long `loca` needs no alignment, but padding each entry to a two-byte boundary keeps
        // the table readable under either format and costs at most one byte a glyph.
        if !rebuilt.len().is_multiple_of(2) {
            rebuilt.push(0);
        }
    }
    offsets.push(u32::try_from(rebuilt.len()).ok()?);

    let mut loca = Vec::with_capacity(offsets.len().checked_mul(4)?);
    for value in &offsets {
        loca.extend_from_slice(&value.to_be_bytes());
    }
    let mut repaired = rewritten_sfnt(data, &tables, &[(b"glyf", rebuilt), (b"loca", loca)])?;
    // `indexToLocFormat` must lie inside `head`, for `repaired_loca_format`'s reason: the
    // directory is untrusted and this writes at a computed offset.
    if head_length < 52 {
        return None;
    }
    // The rebuilt `loca` is long whatever the original was, and `head` has to say so.
    let slot = repaired.get_mut(head.checked_add(50)?..head.checked_add(52)?)?;
    slot.copy_from_slice(&1u16.to_be_bytes());
    Some(repaired)
}

/// The length of one `glyf` entry, read from the entry itself.
///
/// `None` where the entry runs off the table or states a contour count this cannot follow, which
/// is what makes [`repaired_loca_order`] give up on a font rather than truncate one.
pub(crate) fn glyph_length(glyf: &[u8], start: usize) -> Option<usize> {
    /// `numberOfContours`, `xMin`, `yMin`, `xMax`, `yMax`.
    const HEADER: usize = 10;

    let contours = i16::from_be_bytes([*glyf.get(start)?, *glyf.get(start.checked_add(1)?)?]);
    if contours >= 0 {
        let contours = usize::try_from(contours).ok()?;
        let mut at = start
            .checked_add(HEADER)?
            .checked_add(contours.checked_mul(2)?)?;
        // A glyph with no contours has no point data at all — not even an instruction count.
        if contours == 0 {
            return at.checked_sub(start);
        }
        let points = usize::from(be16(glyf, at.checked_sub(2)?)?).checked_add(1)?;
        let instructions = usize::from(be16(glyf, at)?);
        at = at.checked_add(2)?.checked_add(instructions)?;
        // The flags are run-length encoded: bit 3 says the next byte is a repeat count.
        let mut flags: Vec<u8> = Vec::with_capacity(points);
        while flags.len() < points {
            let flag = *glyf.get(at)?;
            at = at.checked_add(1)?;
            flags.push(flag);
            if flag & 0x08 != 0 {
                let repeats = usize::from(*glyf.get(at)?);
                at = at.checked_add(1)?;
                for _ in 0..repeats {
                    if flags.len() >= points {
                        break;
                    }
                    flags.push(flag);
                }
            }
        }
        // x then y, each coordinate one byte, two bytes, or absent — bit 1 and bit 4 for x, bit
        // 2 and bit 5 for y, in the clause's own pairing of a short flag with a "same" flag.
        for (short, same) in [(0x02u8, 0x10u8), (0x04, 0x20)] {
            for flag in &flags {
                let width = if flag & short != 0 {
                    1
                } else if flag & same != 0 {
                    0
                } else {
                    2
                };
                at = at.checked_add(width)?;
            }
        }
        return at.checked_sub(start);
    }

    // A composite: a loop of components, each of which says whether another follows.
    let mut at = start.checked_add(HEADER)?;
    let mut instructions = false;
    loop {
        let flags = be16(glyf, at)?;
        at = at.checked_add(4)?; // the flags and the component's glyph index
        at = at.checked_add(if flags & 0x0001 != 0 { 4 } else { 2 })?;
        at = at.checked_add(if flags & 0x0008 != 0 {
            2
        } else if flags & 0x0040 != 0 {
            4
        } else if flags & 0x0080 != 0 {
            8
        } else {
            0
        })?;
        instructions |= flags & 0x0100 != 0;
        if flags & 0x0020 == 0 {
            break;
        }
    }
    if instructions {
        let length = usize::from(be16(glyf, at)?);
        at = at.checked_add(2)?.checked_add(length)?;
    }
    at.checked_sub(start)
}

/// ISO 32000-2 §9.6.5.4, one rule at a time, on fonts built to isolate it.
///
/// The corpus cannot do this. It can show that a real document draws, and it did — but
/// every real font carries several `cmap` subtables, so a page drawing correctly proves
/// only that *some* route worked, and a page drawing wrongly does not say which route was
/// missing. Each font here carries exactly one subtable, so exactly one rule of the
/// subclause can possibly apply to it, and a rule that stops working fails one test by
/// name. This is trap 8's argument in the handover, from the other direction.
#[cfg(test)]
mod truncation_tests {
    use super::truncation;

    /// Builds a table directory naming one table at `offset` for `length` bytes.
    fn directory(tag: [u8; 4], offset: u32, length: u32, total: usize) -> Vec<u8> {
        let mut out = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        out.extend_from_slice(&tag);
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(&offset.to_be_bytes());
        out.extend_from_slice(&length.to_be_bytes());
        out.resize(total, 0);
        out
    }

    /// The condition, on the table it is about and on a table it is not.
    #[test]
    fn only_a_head_table_beyond_the_bytes_is_a_truncation() {
        // `head` ending at 4000 in a 512-byte program: `issue11651.pdf`'s shape.
        assert_eq!(
            truncation(&directory(*b"head", 3900, 100, 512)),
            Some(("head".to_owned(), 4000))
        );
        // The same overrun on `glyf`, which is read per glyph: not a refusal.
        assert_eq!(truncation(&directory(*b"glyf", 3900, 100, 512)), None);
        // A whole program says nothing.
        assert_eq!(truncation(&directory(*b"head", 100, 54, 512)), None);
    }
}
