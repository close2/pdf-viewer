//! A TrueType program given more glyphs, each with its own glyph 0's outline.
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator
//! to reference the `.notdef` glyph, and in a TrueType program that glyph is glyph 0 by position
//! (the TrueType Reference Manual's `loca` and `post` chapters). A code the producer's file sends
//! to glyph 0 draws the program's own `.notdef` outline, and that outline is what this reader
//! draws for it (`doc/questions/A694` item 3). This module copies that outline, byte for byte,
//! to new glyphs at the end of the program, so that a `cmap` can send each code there: the page
//! draws the producer's marks and no code reaches `.notdef` (`doc/adr/2543`).
//!
//! **One glyph per advance, not one per program**: ISO 19005-2 section 6.2.11.5 holds the
//! program's advance for the glyph a code reaches to the dictionary's width for that code, and
//! two codes the producer gave two widths cannot share one glyph. A new glyph's advance is the
//! width the reader advanced by, which is the dictionary's (ISO 32000-2 §9.2.4); its bearing and
//! outline are glyph 0's.
//!
//! Every table the Manual sizes by the glyph count grows by one entry a glyph: `maxp`'s count,
//! `loca` and `glyf`, `hmtx` (with `hhea`'s count of long metrics), `vmtx` and `vhea` where the
//! program has them, `hdmx` (the advance at each record's size), `LTSH`, and `post`'s names where
//! its format lists them. A program carrying a table indexed by glyph that this does not know how
//! to extend is left alone: `None`, and the font keeps its refusal.

use std::collections::{BTreeMap, BTreeSet};

/// The tables the new glyphs change, keyed by tag, and the glyphs' indices.
pub(in crate::archive) struct GivenGlyphs {
    /// Each table that changes, whole, to be put into the program in place of its own.
    pub(in crate::archive) tables: Vec<([u8; 4], Vec<u8>)>,
    /// The index of each new glyph, in the order of the advances asked for, from the program's
    /// glyph count before the copy.
    pub(in crate::archive) glyphs: Vec<u16>,
}

/// The most glyphs one program is given: one for each code of a simple font.
pub(in crate::archive) const MOST_GIVEN: usize = 256;

/// The most bytes the copies of glyph 0's outline may add to `glyf` together, checked before any
/// is made: a `.notdef` outline is a box of a few dozen bytes, and a program whose glyph 0 is
/// large would otherwise grow by [`MOST_GIVEN`] times its own size.
pub(in crate::archive) const MOST_COPIED_OUTLINE_BYTES: usize = 1 << 20;

/// Tables indexed by glyph that a new glyph would make false and that this does not extend.
///
/// Their entries are per glyph and of a shape the Manual describes per format (`VDMX` is not one:
/// it is keyed by ratio and height). A program carrying one keeps its refusal rather than leaving
/// a table that states one glyph fewer than the program has.
const NOT_EXTENDED: [[u8; 4]; 4] = [*b"EBLC", *b"CBLC", *b"bloc", *b"sbix"];

/// The first name of `notdef`, `notdef.1`, `notdef.2`, … tried for the new glyph in a `post`
/// table that names glyphs; [`MAX_NAME_TRIES`] are tried.
const NAME_STEM: &str = "notdef";

/// How many candidate names, in order, are tried for all the copies together before the program
/// is refused: enough for each of [`MOST_GIVEN`] copies to get a name after passing every name a
/// table of a 16-bit glyph count can already hold.
const MAX_NAME_TRIES: usize = MOST_GIVEN + (1 << 16);

/// The program's tables by tag, as byte ranges, read off its table directory.
///
/// The directory is the Manual's: a version, a 16-bit table count, three search fields, and one
/// 16-byte record per table (tag, checksum, offset, length). The count bounds the loop.
fn directory(program: &[u8]) -> Option<BTreeMap<[u8; 4], &[u8]>> {
    let count = usize::from(u16_at(program, 4)?);
    let mut tables = BTreeMap::new();
    for index in 0..count {
        let record = index.checked_mul(16)?.checked_add(12)?;
        let tag: [u8; 4] = program.get(span(record, 4)?)?.try_into().ok()?;
        let offset = usize::try_from(u32_at(program, record.checked_add(8)?)?).ok()?;
        let length = usize::try_from(u32_at(program, record.checked_add(12)?)?).ok()?;
        let bytes = program.get(offset..offset.checked_add(length)?)?;
        tables.insert(tag, bytes);
    }
    Some(tables)
}

/// The program with glyph 0 copied once for each of `advances`, in font units, as the tables
/// that change.
///
/// `None` where no advance or more than [`MOST_GIVEN`] are asked for, where the program is not a
/// `glyf` sfnt this reads, where the new glyphs would pass the most a 16-bit index reaches, or
/// where it carries a table [`NOT_EXTENDED`] names or a `post` format that is not 1, 2 or 3.
pub(in crate::archive) fn with_notdef_copied(
    program: &[u8],
    advances: &[u16],
) -> Option<GivenGlyphs> {
    if advances.is_empty() || advances.len() > MOST_GIVEN {
        return None;
    }
    let tables = directory(program)?;
    if NOT_EXTENDED.iter().any(|tag| tables.contains_key(tag)) {
        return None;
    }
    let maxp = *tables.get(b"maxp")?;
    let count = u16_at(maxp, 4)?;
    let added = u16::try_from(advances.len()).ok()?;
    let grown = count.checked_add(added)?;
    let glyphs: Vec<u16> = (count..grown).collect();
    let mut out = Vec::new();

    let mut new_maxp = maxp.to_vec();
    put_u16(&mut new_maxp, 4, grown)?;
    out.push((*b"maxp", new_maxp));

    let head = *tables.get(b"head")?;
    let long_offsets = match i16_at(head, 50)? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let offsets = loca_offsets(tables.get(b"loca")?, count, long_offsets)?;
    let glyf = *tables.get(b"glyf")?;
    let first = *offsets.first()?;
    let second = *offsets.get(1)?;
    let end = *offsets.last()?;
    let outline = glyf.get(first..second)?;
    if outline.len().checked_add(1)?.checked_mul(advances.len())? > MOST_COPIED_OUTLINE_BYTES {
        return None;
    }
    let mut new_glyf = glyf.get(..end)?.to_vec();
    // A short `loca` stores each offset halved, so under it every glyph starts on an even byte;
    // a long one stores the byte offset, and a copy is then the outline's bytes and nothing more.
    if !long_offsets && new_glyf.len() % 2 == 1 {
        new_glyf.push(0);
    }
    let mut new_offsets = offsets.clone();
    new_offsets.pop();
    for _ in advances {
        new_offsets.push(new_glyf.len());
        new_glyf.extend_from_slice(outline);
        if !long_offsets && new_glyf.len() % 2 == 1 {
            new_glyf.push(0);
        }
    }
    new_offsets.push(new_glyf.len());
    // A short `loca` reaches 0x1FFFE bytes of `glyf`; past it the program changes format, which
    // `head` states.
    let long = long_offsets || new_glyf.len() > 2 * usize::from(u16::MAX);
    out.push((*b"loca", loca_bytes(&new_offsets, long)?));
    out.push((*b"glyf", new_glyf));
    if long != long_offsets {
        let mut new_head = head.to_vec();
        put_u16(&mut new_head, 50, 1)?;
        out.push((*b"head", new_head));
    }

    let (_, bearing) = metric(&tables, *b"hhea", *b"hmtx", 0)?;
    let horizontal: Vec<(u16, i16)> = advances.iter().map(|advance| (*advance, bearing)).collect();
    let (hhea, hmtx) = metrics_grown(&tables, (*b"hhea", *b"hmtx"), count, &horizontal)?;
    out.push((*b"hhea", hhea));
    out.push((*b"hmtx", hmtx));
    if tables.contains_key(b"vmtx") || tables.contains_key(b"vhea") {
        let vertical = vec![metric(&tables, *b"vhea", *b"vmtx", 0)?; advances.len()];
        let (vhea, vmtx) = metrics_grown(&tables, (*b"vhea", *b"vmtx"), count, &vertical)?;
        out.push((*b"vhea", vhea));
        out.push((*b"vmtx", vmtx));
    }
    if let Some(hdmx) = tables.get(b"hdmx") {
        let units = u16_at(head, 18)?;
        out.push((*b"hdmx", hdmx_grown(hdmx, count, (advances, units))?));
    }
    if let Some(ltsh) = tables.get(b"LTSH") {
        out.push((*b"LTSH", ltsh_grown(ltsh, count, added)?));
    }
    if let Some(post) = tables.get(b"post")
        && let PostGrown::Replaced(grown_post) = post_grown(post, count, added)?
    {
        out.push((*b"post", grown_post));
    }
    Some(GivenGlyphs {
        tables: out,
        glyphs,
    })
}

/// The program's units per em, from `head`.
pub(in crate::archive) fn units_per_em(program: &[u8]) -> Option<u16> {
    u16_at(directory(program)?.get(b"head")?, 18).filter(|units| *units > 0)
}

/// One glyph's outline bytes in a program, by its `loca`.
pub(in crate::archive) fn outline(program: &[u8], glyph: u16) -> Option<&[u8]> {
    let tables = directory(program)?;
    let count = u16_at(tables.get(b"maxp")?, 4)?;
    if glyph >= count {
        return None;
    }
    let long = match i16_at(tables.get(b"head")?, 50)? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let offsets = loca_offsets(tables.get(b"loca")?, count, long)?;
    let at = usize::from(glyph);
    tables
        .get(b"glyf")?
        .get(*offsets.get(at)?..*offsets.get(at.checked_add(1)?)?)
}

/// One glyph's horizontal metric, its advance and left side bearing, by `hhea` and `hmtx`.
#[cfg(test)]
fn horizontal_metric(program: &[u8], glyph: u16) -> Option<(u16, i16)> {
    let tables = directory(program)?;
    metric(&tables, *b"hhea", *b"hmtx", glyph)
}

/// The `count + 1` offsets a `loca` table of either format holds, in bytes, each checked to be no
/// smaller than the one before.
fn loca_offsets(loca: &[u8], count: u16, long: bool) -> Option<Vec<usize>> {
    let entries = usize::from(count).checked_add(1)?;
    let mut offsets = Vec::with_capacity(entries);
    for index in 0..entries {
        let offset = if long {
            usize::try_from(u32_at(loca, index.checked_mul(4)?)?).ok()?
        } else {
            usize::from(u16_at(loca, index.checked_mul(2)?)?).checked_mul(2)?
        };
        if offsets.last().is_some_and(|before| offset < *before) {
            return None;
        }
        offsets.push(offset);
    }
    Some(offsets)
}

/// A `loca` table of the given format holding these offsets.
fn loca_bytes(offsets: &[usize], long: bool) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(offsets.len().checked_mul(if long { 4 } else { 2 })?);
    for offset in offsets {
        if long {
            out.extend_from_slice(&u32::try_from(*offset).ok()?.to_be_bytes());
        } else {
            out.extend_from_slice(&u16::try_from(offset / 2).ok()?.to_be_bytes());
        }
    }
    Some(out)
}

/// One glyph's metric from a header and a metrics table of the `hhea`/`hmtx` shape.
///
/// The header's 16-bit field at offset 34 counts the long metrics; a glyph past them takes the
/// last long metric's advance and its own bearing from the array after them (the Manual's `hmtx`
/// and `vmtx` chapters, which share the layout).
fn metric(
    tables: &BTreeMap<[u8; 4], &[u8]>,
    header: [u8; 4],
    metrics: [u8; 4],
    glyph: u16,
) -> Option<(u16, i16)> {
    let long = u16_at(tables.get(&header)?, 34)?;
    let table = *tables.get(&metrics)?;
    if long == 0 {
        return None;
    }
    let at = usize::from(glyph);
    if glyph < long {
        let start = at.checked_mul(4)?;
        Some((u16_at(table, start)?, i16_at(table, start.checked_add(2)?)?))
    } else {
        let long = usize::from(long);
        let advance = u16_at(table, long.checked_sub(1)?.checked_mul(4)?)?;
        let bearing_at = long
            .checked_mul(4)?
            .checked_add(at.checked_sub(long)?.checked_mul(2)?)?;
        let bearing = i16_at(table, bearing_at)?;
        Some((advance, bearing))
    }
}

/// A header and a metrics table restated with every glyph's metric long, and `added` appended.
///
/// Every existing glyph keeps its advance and bearing; what changes is only that none takes its
/// advance from the last long metric any more, so each new one can carry its own advance whatever
/// the last glyph's was.
fn metrics_grown(
    tables: &BTreeMap<[u8; 4], &[u8]>,
    (header, metrics): ([u8; 4], [u8; 4]),
    count: u16,
    added: &[(u16, i16)],
) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut table = Vec::with_capacity(
        usize::from(count)
            .checked_add(added.len())?
            .checked_mul(4)?,
    );
    for glyph in 0..count {
        let (advance, bearing) = metric(tables, header, metrics, glyph)?;
        table.extend_from_slice(&advance.to_be_bytes());
        table.extend_from_slice(&bearing.to_be_bytes());
    }
    for (advance, bearing) in added {
        table.extend_from_slice(&advance.to_be_bytes());
        table.extend_from_slice(&bearing.to_be_bytes());
    }
    let mut new_header = tables.get(&header)?.to_vec();
    let long = count.checked_add(u16::try_from(added.len()).ok()?)?;
    put_u16(&mut new_header, 34, long)?;
    Some((new_header, table))
}

/// An `hdmx` table with one more width per record for each new glyph: its advance at the
/// record's pixel size, rounded to a whole pixel, which is what the table states for an outline
/// no instruction changes the width of.
///
/// The Manual's layout: a version, a record count, a record size, then each record's pixel size,
/// maximum width and one width byte per glyph, padded to a multiple of four.
fn hdmx_grown(hdmx: &[u8], count: u16, (advances, units): (&[u16], u16)) -> Option<Vec<u8>> {
    let records = usize::from(u16_at(hdmx, 2)?);
    let size = usize::try_from(u32_at(hdmx, 4)?).ok()?;
    let glyphs = usize::from(count);
    let per_record = glyphs.checked_add(2)?;
    if size < per_record || units == 0 {
        return None;
    }
    // The records the header states must be in the table before the output is reserved for
    // them, so a count the table does not hold cannot reserve memory the input never had.
    if hdmx.len() < records.checked_mul(size)?.checked_add(8)? {
        return None;
    }
    let grown_record = per_record.checked_add(advances.len())?;
    let new_size = grown_record.div_ceil(4).checked_mul(4)?;
    let mut out = Vec::with_capacity(records.checked_mul(new_size)?.checked_add(8)?);
    out.extend_from_slice(hdmx.get(..4)?);
    out.extend_from_slice(&u32::try_from(new_size).ok()?.to_be_bytes());
    for record in 0..records {
        let at = record.checked_mul(size)?.checked_add(8)?;
        let entry = hdmx.get(span(at, per_record)?)?;
        out.extend_from_slice(entry);
        let pixels = u32::from(*entry.first()?);
        let mut widest = *entry.get(1)?;
        for advance in advances {
            let scaled = u32::from(*advance)
                .checked_mul(pixels)?
                .checked_add(u32::from(units) / 2)?
                .checked_div(u32::from(units))?;
            let width = u8::try_from(scaled).ok()?;
            widest = widest.max(width);
            out.push(width);
        }
        let record_at = out.len().checked_sub(grown_record)?.checked_add(1)?;
        *out.get_mut(record_at)? = widest;
        out.resize(
            record
                .checked_add(1)?
                .checked_mul(new_size)?
                .checked_add(8)?,
            0,
        );
    }
    Some(out)
}

/// An `LTSH` table with glyph 0's threshold appended for each new glyph and its count raised.
fn ltsh_grown(ltsh: &[u8], count: u16, added: u16) -> Option<Vec<u8>> {
    if u16_at(ltsh, 2)? != count {
        return None;
    }
    let mut out = ltsh.get(..4 + usize::from(count))?.to_vec();
    put_u16(&mut out, 2, count.checked_add(added)?)?;
    let threshold = *ltsh.get(4)?;
    out.extend(std::iter::repeat_n(threshold, usize::from(added)));
    Some(out)
}

/// What a `post` table becomes when the program gains glyphs.
enum PostGrown {
    /// The table's format names no glyph, so it stays as it is.
    Unchanged,
    /// The table restated with the new glyphs' names.
    Replaced(Vec<u8>),
}

/// A `post` table that names the new glyphs: [`PostGrown::Unchanged`] where its format names no
/// glyph and so needs no change, `None` where it cannot be extended.
///
/// Format 3 names none. Format 2 lists a name index per glyph, then the names past the Macintosh
/// set as Pascal strings; each new glyph gets a new name (index 258 and up), the first of
/// [`NAME_STEM`], `notdef.1`, … the table does not already hold. Format 1 is the Macintosh set
/// alone, so it is restated as format 2 with the same 258 indices and the new ones. Any other
/// format is refused.
fn post_grown(post: &[u8], count: u16, added: u16) -> Option<PostGrown> {
    const MACINTOSH_NAMES: u16 = 258;
    let (indices, strings, names): (Vec<u8>, &[u8], Vec<&[u8]>) = match u32_at(post, 0)? {
        0x0003_0000 => return Some(PostGrown::Unchanged),
        0x0001_0000 => {
            if count != MACINTOSH_NAMES {
                return None;
            }
            let mut indices = post.get(..32)?.to_vec();
            put_u32(&mut indices, 0, 0x0002_0000)?;
            indices.extend_from_slice(&count.to_be_bytes());
            for index in 0..count {
                indices.extend_from_slice(&index.to_be_bytes());
            }
            (indices, &[], Vec::new())
        }
        0x0002_0000 => {
            if u16_at(post, 32)? != count {
                return None;
            }
            let indices_end = usize::from(count).checked_mul(2)?.checked_add(34)?;
            let strings = post.get(indices_end..)?;
            let mut names = Vec::new();
            let mut at = 0;
            while let Some(&length) = strings.get(at) {
                let start = at.checked_add(1)?;
                names.push(strings.get(span(start, usize::from(length))?)?);
                at = start.checked_add(usize::from(length))?;
            }
            (post.get(..indices_end)?.to_vec(), strings.get(..at)?, names)
        }
        _ => return None,
    };
    let mut out = indices;
    put_u16(&mut out, 32, count.checked_add(added)?)?;
    let held: BTreeSet<&[u8]> = names.iter().copied().collect();
    let mut taken: Vec<String> = Vec::new();
    let mut tried = 0_usize;
    for _ in 0..added {
        // The candidates run on from where the last copy's search stopped, so the names given
        // are distinct and the search over every copy is one pass of at most MAX_NAME_TRIES.
        let name = loop {
            if tried >= MAX_NAME_TRIES {
                return None;
            }
            let candidate = if tried == 0 {
                NAME_STEM.to_owned()
            } else {
                format!("{NAME_STEM}.{tried}")
            };
            tried = tried.checked_add(1)?;
            if !held.contains(candidate.as_bytes()) {
                break candidate;
            }
        };
        let index = MACINTOSH_NAMES
            .checked_add(u16::try_from(names.len().checked_add(taken.len())?).ok()?)?;
        out.extend_from_slice(&index.to_be_bytes());
        taken.push(name);
    }
    out.extend_from_slice(strings);
    for name in &taken {
        pascal(&mut out, name)?;
    }
    Some(PostGrown::Replaced(out))
}

/// Appends a Pascal string: one length byte, then the bytes.
fn pascal(out: &mut Vec<u8>, name: &str) -> Option<()> {
    out.push(u8::try_from(name.len()).ok()?);
    out.extend_from_slice(name.as_bytes());
    Some(())
}

/// The `len` bytes starting at `at`, as a range; `None` where the end is not a `usize`.
fn span(at: usize, len: usize) -> Option<std::ops::Range<usize>> {
    Some(at..at.checked_add(len)?)
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(span(at, 2)?)?.try_into().ok()?,
    ))
}

fn i16_at(bytes: &[u8], at: usize) -> Option<i16> {
    Some(i16::from_be_bytes(
        bytes.get(span(at, 2)?)?.try_into().ok()?,
    ))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(span(at, 4)?)?.try_into().ok()?,
    ))
}

fn put_u16(bytes: &mut [u8], at: usize, value: u16) -> Option<()> {
    bytes
        .get_mut(span(at, 2)?)?
        .copy_from_slice(&value.to_be_bytes());
    Some(())
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) -> Option<()> {
    bytes
        .get_mut(span(at, 4)?)?
        .copy_from_slice(&value.to_be_bytes());
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal `glyf` sfnt: `glyphs` outlines, short `loca`, two long metrics and the rest
    /// short, and a format 2 `post` naming each glyph past 0 by a custom name.
    fn program(glyphs: &[&[u8]]) -> Vec<u8> {
        program_with(glyphs, false)
    }

    fn program_with(glyphs: &[&[u8]], long: bool) -> Vec<u8> {
        let count = u16::try_from(glyphs.len()).expect("a handful of glyphs");
        let mut glyf = Vec::new();
        let mut loca = Vec::new();
        let offset = |loca: &mut Vec<u8>, at: usize| {
            if long {
                loca.extend_from_slice(&u32::try_from(at).expect("small").to_be_bytes());
            } else {
                loca.extend_from_slice(&u16::try_from(at / 2).expect("small").to_be_bytes());
            }
        };
        for outline in glyphs {
            offset(&mut loca, glyf.len());
            glyf.extend_from_slice(outline);
            if !long && glyf.len() % 2 == 1 {
                glyf.push(0);
            }
        }
        offset(&mut loca, glyf.len());
        let mut head = vec![0; 54];
        head[50..52].copy_from_slice(&i16::from(long).to_be_bytes());
        let mut maxp = vec![0, 0, 0x50, 0];
        maxp.extend_from_slice(&count.to_be_bytes());
        let mut hhea = vec![0; 36];
        hhea[34..36].copy_from_slice(&2_u16.to_be_bytes());
        let mut hmtx = Vec::new();
        hmtx.extend_from_slice(&500_u16.to_be_bytes());
        hmtx.extend_from_slice(&7_i16.to_be_bytes());
        hmtx.extend_from_slice(&600_u16.to_be_bytes());
        hmtx.extend_from_slice(&8_i16.to_be_bytes());
        for bearing in 2..count {
            hmtx.extend_from_slice(&i16::try_from(bearing).expect("small").to_be_bytes());
        }
        let mut post = vec![0; 32];
        post[0..4].copy_from_slice(&0x0002_0000_u32.to_be_bytes());
        post.extend_from_slice(&count.to_be_bytes());
        post.extend_from_slice(&0_u16.to_be_bytes());
        for index in 1..count {
            post.extend_from_slice(&257_u16.checked_add(index).expect("small").to_be_bytes());
        }
        for index in 1..count {
            let name = if index == 1 {
                "notdef".to_owned()
            } else {
                format!("g{index}")
            };
            post.push(u8::try_from(name.len()).expect("short"));
            post.extend_from_slice(name.as_bytes());
        }
        let tables: Vec<([u8; 4], Vec<u8>)> = vec![
            (*b"glyf", glyf),
            (*b"head", head),
            (*b"hhea", hhea),
            (*b"hmtx", hmtx),
            (*b"loca", loca),
            (*b"maxp", maxp),
            (*b"post", post),
        ];
        let mut out = vec![0, 1, 0, 0];
        out.extend_from_slice(&u16::try_from(tables.len()).expect("seven").to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        let directory_end = tables
            .len()
            .checked_mul(16)
            .and_then(|n| n.checked_add(12))
            .expect("seven tables");
        let mut offset = directory_end;
        let mut body = Vec::new();
        for (tag, bytes) in &tables {
            out.extend_from_slice(tag);
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&u32::try_from(offset).expect("small").to_be_bytes());
            out.extend_from_slice(&u32::try_from(bytes.len()).expect("small").to_be_bytes());
            body.extend_from_slice(bytes);
            while body.len() % 4 != 0 {
                body.push(0);
            }
            offset = directory_end.checked_add(body.len()).expect("small");
        }
        out.extend_from_slice(&body);
        out
    }

    fn applied(program: &[u8], given: &GivenGlyphs) -> Vec<u8> {
        pdf_font::embedding::with_tables(program, &given.tables).expect("reassembles")
    }

    #[test]
    fn the_new_glyph_is_glyph_zero_s_outline_and_metric_and_the_rest_are_untouched() {
        let notdef: &[u8] = &[1, 2, 3, 4, 5];
        let source = program(&[notdef, &[9, 9], &[7, 7, 7, 7], &[6]]);
        let given = with_notdef_copied(&source, &[500, 640]).expect("a glyf program");
        assert_eq!(given.glyphs, [4, 5]);
        let after = applied(&source, &given);
        for glyph in [4, 5] {
            assert_eq!(outline(&after, glyph), outline(&source, 0));
            assert!(outline(&after, glyph).is_some_and(|bytes| bytes.starts_with(notdef)));
        }
        assert_eq!(horizontal_metric(&after, 4), Some((500, 7)));
        assert_eq!(
            horizontal_metric(&after, 5),
            Some((640, 7)),
            "its own advance, 0's bearing"
        );
        for glyph in 0..4 {
            assert_eq!(outline(&after, glyph), outline(&source, glyph));
            assert_eq!(
                horizontal_metric(&after, glyph),
                horizontal_metric(&source, glyph)
            );
        }
    }

    #[test]
    fn the_new_name_is_one_the_post_table_does_not_already_hold() {
        let source = program(&[&[1, 1], &[2, 2], &[3, 3]]);
        let given = with_notdef_copied(&source, &[1, 2]).expect("a glyf program");
        let (_, post) = given
            .tables
            .iter()
            .find(|(tag, _)| tag == b"post")
            .expect("format 2 is restated");
        assert_eq!(u16_at(post, 32), Some(5));
        assert_eq!(u16_at(post, 34 + 2 * 3), Some(258 + 2));
        assert_eq!(u16_at(post, 34 + 2 * 4), Some(258 + 3));
        assert!(
            post.ends_with(b"\x08notdef.1\x08notdef.2"),
            "`notdef` is glyph 1's already"
        );
    }

    #[test]
    fn a_program_at_the_most_glyphs_an_index_reaches_is_refused() {
        let mut source = program(&[&[1, 1]]);
        let record = (0..7)
            .map(|index| 12 + 16 * index)
            .find(|record| &source[*record..record + 4] == b"maxp")
            .expect("the directory names maxp");
        let at = usize::try_from(u32_at(&source, record + 8).expect("an offset")).expect("small");
        source[at + 4..at + 6].copy_from_slice(&(u16::MAX - 1).to_be_bytes());
        assert!(
            with_notdef_copied(&source, &[1, 2]).is_none(),
            "two past the last index"
        );
        assert!(with_notdef_copied(&source, &[]).is_none());
        assert!(with_notdef_copied(&source, &[0; MOST_GIVEN + 1]).is_none());
    }

    #[test]
    fn under_a_long_loca_an_odd_outline_is_copied_with_no_pad_byte() {
        let notdef: &[u8] = &[1, 2, 3, 4, 5];
        let source = program_with(&[notdef, &[9, 9, 9]], true);
        let given = with_notdef_copied(&source, &[500, 640]).expect("a glyf program");
        let after = applied(&source, &given);
        for glyph in [0, 1, 2, 3] {
            let expected: &[u8] = if glyph == 1 { &[9, 9, 9] } else { notdef };
            assert_eq!(outline(&after, glyph), Some(expected), "glyph {glyph}");
        }
    }

    #[test]
    fn the_most_copies_a_simple_font_needs_are_each_named() {
        let source = program(&[&[1, 2], &[9, 9]]);
        let given = with_notdef_copied(&source, &[500; MOST_GIVEN]).expect("every copy named");
        assert_eq!(given.glyphs.len(), MOST_GIVEN);
    }

    #[test]
    fn copies_past_the_outline_bound_are_refused_before_they_are_made() {
        // Under a long `loca` each copy is the outline exactly, and the bound counts one more.
        let outline = vec![1; 8191];
        let source = program_with(&[&outline, &[9, 9]], true);
        let at_the_bound = MOST_COPIED_OUTLINE_BYTES / 8192;
        assert!(with_notdef_copied(&source, &vec![500; at_the_bound]).is_some());
        assert!(with_notdef_copied(&source, &vec![500; at_the_bound + 1]).is_none());
    }
}
