//! Bare CFF font programs.
//!
//! `/FontFile3` may hold a bare CFF font program rather than a complete `OpenType` file.
//! A CFF carries no `cmap` table, so a character code reaches a glyph by a different route
//! than in a `TrueType` font, and that route is what this module supplies.
//!
//! # The two routes, and why the distinction matters
//!
//! A *name-keyed* CFF — what a PDF simple font embeds — names every glyph in its charset.
//! A character code becomes a glyph name (from the PDF `/Encoding`, or from the encoding
//! the font itself carries) and the name becomes a glyph index. A *CID-keyed* CFF — what a
//! composite font embeds — has no glyph names at all: its charset assigns a CID per glyph,
//! and a code reaches a glyph by inverting that.
//!
//! Confusing the two is not a harmless error. Treating a character code as a glyph index,
//! which is what happens when neither route is taken, loads without complaint and draws
//! the wrong glyphs — text that looks like text and says something else. That failure is
//! why [`crate::LoadedFont::glyph_for`] refuses to guess.
//!
//! # Why this module is thin
//!
//! Every byte-level structure involved — the INDEX and DICT encodings, charset formats 0,
//! 1 and 2, encoding formats 0 and 1 with their supplements, the String INDEX and Adobe's
//! 391 standard strings — is parsed by `read-fonts`, which `skrifa` already brings in and
//! which is fuzzed and memory-safe. Hand-rolling that parsing would mean writing exactly
//! the untrusted-input byte handling this project chose `skrifa` to avoid, and would mean
//! transcribing a 391-entry table by hand. See `doc/adr/0006-cff-through-read-fonts.md`.
//!
//! What is left here is the PDF-specific part: deciding *which* route applies, and putting
//! the results in a form the loader can use.

use std::collections::BTreeMap;

use skrifa::GlyphId;
use skrifa::outline::OutlinePen;
use skrifa::raw::ps::cff::CffFontRef;

use crate::name_keyed::NameKeyed;

/// Why a bare CFF font program could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CffError {
    /// The program could not be parsed.
    #[error("the CFF font program could not be read: {detail}")]
    Malformed {
        /// What `read-fonts` reported.
        detail: String,
    },
    /// A glyph's advance could not be restated where the program states it.
    #[error("glyph {glyph}'s advance cannot be restated: {detail}")]
    AdvanceNotRestatable {
        /// Which glyph.
        glyph: u16,
        /// Why not.
        detail: String,
    },
    /// The program parsed, but carries no charset.
    ///
    /// Without one there is no way to reach a glyph from a name or a CID, and the only
    /// remaining option would be to assume the character code is a glyph index — the
    /// silent-wrong-glyph failure this module exists to prevent.
    #[error("the CFF font program has no charset, so its glyphs cannot be identified")]
    NoCharset,
}

/// The character-code mapping a bare CFF carries in place of a `cmap` table.
///
/// Built once when the font is loaded, because resolving it per glyph would mean
/// re-parsing the font program on every character drawn.
#[derive(Debug)]
pub enum CodeToGlyph {
    /// A name-keyed font, which is what a PDF *simple* font embeds.
    ///
    /// The payload is the same one a bare Type 1 program produces, because §9.6.5.2 is one
    /// algorithm for both formats — see [`crate::name_keyed`].
    Named(NameKeyed),
    /// A CID-keyed font, which is what a PDF *composite* font embeds.
    Keyed {
        /// Glyph index by CID, inverting the charset.
        by_cid: BTreeMap<u16, u16>,
    },
}

impl CodeToGlyph {
    /// Reads the mapping out of a bare CFF font program.
    ///
    /// # Errors
    ///
    /// See [`CffError`].
    pub fn read(data: &[u8]) -> Result<Self, CffError> {
        let font = open(data)?;
        let charset = font.charset().ok_or(CffError::NoCharset)?;

        // A CID-keyed font's charset assigns CIDs, not names, and its `Encoding` — which
        // `read-fonts` will still report as the default — is meaningless. Reading names
        // out of it would yield whatever standard string happened to share the CID's
        // number, which is how a composite font ends up drawing Latin letters for CJK.
        if font.is_cid() {
            let mut by_cid = BTreeMap::new();
            for (glyph, cid) in charset.iter() {
                if let Ok(glyph) = u16::try_from(glyph.to_u32()) {
                    by_cid.entry(cid.to_u16()).or_insert(glyph);
                }
            }
            return Ok(Self::Keyed { by_cid });
        }

        let mut by_glyph = BTreeMap::new();
        for (glyph, sid) in charset.iter() {
            let Ok(glyph) = u16::try_from(glyph.to_u32()) else {
                continue;
            };
            let Some(name) = font.string(sid) else {
                continue;
            };
            // Glyph names are ASCII by specification; a font that breaks that is
            // malformed, not a reason to give up on the glyphs that are well formed.
            let name = String::from_utf8_lossy(name).into_owned().into_boxed_str();
            by_glyph.entry(glyph).or_insert(name);
        }

        let mut builtin = Box::new([None; 256]);
        if let Some(encoding) = font.encoding() {
            for (code, slot) in builtin.iter_mut().enumerate() {
                let Ok(code) = u8::try_from(code) else {
                    continue;
                };
                *slot = encoding
                    .map(code)
                    .and_then(|glyph| u16::try_from(glyph.to_u32()).ok());
            }
        }

        Ok(Self::Named(NameKeyed::new(&by_glyph, builtin)))
    }
}

/// Reads a font program's units per em.
///
/// A bare CFF states its scale in its `FontMatrix` rather than in a `head` table; the
/// default is one thousandth, meaning 1000 units per em.
///
/// # Errors
///
/// See [`CffError`].
pub fn units_per_em(data: &[u8]) -> Result<f32, CffError> {
    let font = open(data)?;
    let upem = font.upem();
    if upem <= 0 {
        return Err(CffError::Malformed {
            detail: format!("units per em is {upem}"),
        });
    }
    // A units-per-em beyond f32's exact integer range is not a font scale.
    #[expect(
        clippy::cast_precision_loss,
        reason = "checked non-zero and, in any real font, far below f32's exact integer limit"
    )]
    Ok(upem as f32)
}

/// Draws one glyph from a bare CFF font program.
///
/// # Errors
///
/// See [`CffError`].
pub fn draw(data: &[u8], glyph: u16, pen: &mut impl OutlinePen) -> Result<(), CffError> {
    let font = open(data)?;
    let id = GlyphId::from(glyph);
    // A CID-keyed font splits its private dictionaries across subfonts selected per glyph,
    // so the subfont must be resolved before the charstring can be interpreted.
    let index = font.subfont_index(id).ok_or_else(|| CffError::Malformed {
        detail: format!("glyph {glyph} selects no subfont"),
    })?;
    let subfont = font.subfont(index, &[]).map_err(|e| malformed(&e))?;
    // No size in pixels per em: the outline stays in font units and the caller normalises
    // it, because a PDF text matrix scales it afterwards anyway.
    font.draw(&subfont, id, &[], None, pen)
        .map_err(|e| malformed(&e))?;
    Ok(())
}

/// The advance widths a bare CFF states for the given glyphs, in the program's own units.
///
/// One entry per requested glyph, `None` where the program states no width for it — a Type 2
/// charstring may omit the leading width operand, and a Private DICT need not carry a
/// `defaultWidthX`, in which case the format itself supplies no answer.
///
/// This is the width ISO 32000-2 §9.2.4 calls a glyph's horizontal displacement, read from
/// the program rather than from the document, which §9.6.2's Table 109 says the two shall
/// agree about: "These widths shall be consistent with the actual widths given in the font
/// program." It exists because a *substituted* standard 14 font states no `/Widths` and
/// Adobe's published metrics name only the standard character set, so a glyph outside it —
/// `.notdef` above all — has no other statement of its width anywhere.
///
/// The program is opened once for the whole batch: [`draw`] opens it per glyph because a
/// page draws a few dozen glyphs out of hundreds, while this is asked at load time for
/// every code at once.
///
/// # Errors
///
/// See [`CffError`]. A glyph the program cannot evaluate yields `None` rather than an error,
/// because one unreadable charstring is not a reason to lose the other 255 widths.
pub fn advances(data: &[u8], glyphs: &[u16]) -> Result<Vec<Option<f32>>, CffError> {
    /// Discards the outline: only the charstring's width operand is wanted here.
    struct NoOutline;
    impl OutlinePen for NoOutline {
        fn move_to(&mut self, _x: f32, _y: f32) {}
        fn line_to(&mut self, _x: f32, _y: f32) {}
        fn quad_to(&mut self, _cx: f32, _cy: f32, _x: f32, _y: f32) {}
        fn curve_to(&mut self, _cx0: f32, _cy0: f32, _cx1: f32, _cy1: f32, _x: f32, _y: f32) {}
        fn close(&mut self) {}
    }

    let font = open(data)?;
    Ok(glyphs
        .iter()
        .map(|glyph| {
            let id = GlyphId::from(*glyph);
            let index = font.subfont_index(id)?;
            let subfont = font.subfont(index, &[]).ok()?;
            font.draw(&subfont, id, &[], None, &mut NoOutline).ok()?
        })
        .collect())
}

/// Opens a bare CFF font program.
///
/// The units per em is left unstated so it is taken from the font's own `FontMatrix`;
/// passing one would only be right for a CFF inside an `OpenType` file, which has a `head`
/// table to take it from.
fn open(data: &[u8]) -> Result<CffFontRef<'_>, CffError> {
    CffFontRef::new_cff(data, 0, None).map_err(|e| malformed(&e))
}

fn malformed(error: &skrifa::raw::ps::error::Error) -> CffError {
    CffError::Malformed {
        detail: format!("{error:?}"),
    }
}

/// A CID-keyed program some of whose Font DICTs could not be read, with those replaced.
///
/// # What was wrong, and what this is
///
/// §9.7.4.2 reaches a CID-keyed CFF's glyph through its charset and "the `CharStrings` INDEX
/// table"; the Private DICT the charstring is then interpreted against is selected per glyph
/// through the program's `FDSelect` and read out of its `FDArray`, both of which the Top DICT
/// locates by absolute offset (Adobe Technical Note #5176, sections 18 and 19). Table 124
/// requires the program to "conform to Adobe Technical Note #5176", and the one from
/// `batch5/FOP`'s `FOP-2736-4.pdf` — Apache FOP 2.3.0-SNAPSHOT's subsetter — does not: its Top
/// DICT puts the `FDArray` at offset 8001, which is *inside* its own format-0 `FDSelect` (one
/// byte per glyph from 208 to 8133), so any reader finds a zero count there and no Font DICT
/// for the FD every glyph selects. The 7925 charstrings are intact, and both `poppler` and
/// `mupdf` draw the page — `FreeType`, under both, interprets a CID font with no subfonts
/// against its top-level defaults. `doc/pdf.js`'s `issue9278.pdf` is the other shape: an
/// `FDArray` of nineteen Font DICTs of which the first four state no Private DICT, selected by
/// glyphs the page shows, beside fifteen that read and carry the subroutines their glyphs call.
///
/// A charstring's outline depends on its Private DICT in exactly one way: `callsubr` reaches
/// the DICT's local subroutines, and nothing else in Type 2 does (Adobe Technical Note #5177,
/// section 4.1 — `defaultWidthX` and `nominalWidthX` decide the *advance*, which a `CIDFont`
/// takes from `/W` and `/DW` under §9.7.4.3 rather than from the program). So every Font DICT a
/// glyph selects that cannot be read is replaced by one with an *empty* Private DICT, every
/// Font DICT that reads is kept as it is, and the glyphs under a replaced DICT that call a local
/// subroutine — which the empty DICT cannot hold — are named rather than guessed at:
/// [`FontDictRepair::lost`] is what a page reports, for the codes it shows that reach one.
///
/// # How
///
/// The Top DICT is re-encoded with every offset operand in the five-byte form, so its length is
/// known before the offsets are, and everything after the Top DICT INDEX is copied verbatim and
/// shifted by the difference; the Top DICT and the Font DICTs are the only places a CFF states
/// an absolute offset (a Private DICT's `Subrs` is relative to the DICT). A fresh `FDArray` is
/// appended — each kept Font DICT re-encoded with its Private DICT's offset shifted, each
/// unreadable one an empty Private DICT — and the Top DICT points at it; the `FDSelect` is the
/// program's own. The same shape as `pdf_model::image::frame_as_defined` (ADR 0799): the bytes
/// are made to say what the program states before the reader that cannot tolerate the
/// statement's form sees them. ADR 0808.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontDictRepair {
    /// The program with its unreadable Font DICTs replaced.
    pub data: Vec<u8>,
    /// How many glyphs the program holds.
    pub glyphs: usize,
    /// How many Font DICTs the rebuilt `FDArray` holds — every FD a glyph selects, and every
    /// one the program's own `FDArray` held.
    pub font_dicts: usize,
    /// How many of those were replaced because they could not be read.
    pub replaced: usize,
    /// The glyphs under a replaced Font DICT that call a local subroutine, which the
    /// replacement cannot hold, and therefore draw nothing — by glyph index, so that a page
    /// can be told which of the codes it shows reached one.
    pub lost: std::collections::BTreeSet<u16>,
    /// Where the Top DICT said the `FDArray` was.
    pub fd_array_at: usize,
}

/// Replaces the Font DICTs of a CID-keyed program that cannot be read, keeping the rest.
///
/// `Ok(None)` for a name-keyed program and for a CID-keyed one whose every selected Font DICT
/// reads, which is nearly every one; see [`FontDictRepair`] for the rest.
///
/// # Errors
///
/// See [`CffError`]. A program that does not open is not repaired — the reader reports it.
pub fn readable_font_dicts(data: &[u8]) -> Result<Option<FontDictRepair>, CffError> {
    let font = open(data)?;
    if !font.is_cid() {
        return Ok(None);
    }
    let glyphs = usize::try_from(font.num_glyphs()).unwrap_or(usize::MAX);
    let glyph_ids = 0..u16::try_from(glyphs).unwrap_or(u16::MAX);
    // Every FD a glyph selects, each read once: a font names a handful of subfonts.
    let selected: std::collections::BTreeSet<u16> = glyph_ids
        .clone()
        .filter_map(|glyph| font.subfont_index(GlyphId::from(glyph)))
        .collect();
    let unreadable: std::collections::BTreeSet<u16> = selected
        .iter()
        .copied()
        .filter(|index| font.subfont(*index, &[]).is_err())
        .collect();
    if unreadable.is_empty() {
        return Ok(None);
    }

    let layout = Layout::read(data).ok_or_else(|| CffError::Malformed {
        detail: "the Top DICT INDEX could not be located".to_owned(),
    })?;
    let fd_array_at = layout.fd_array_at.unwrap_or(0);
    // The program's own Font DICTs, where its FDArray reads at all; the witness's does not.
    let own: Vec<&[u8]> = index_items(data, fd_array_at).unwrap_or_default();
    let count = selected
        .iter()
        .max()
        .map_or(0, |index| usize::from(*index).saturating_add(1))
        .max(own.len());
    let font_dicts: Vec<Option<&[u8]>> = (0..count)
        .map(|index| {
            let readable = u16::try_from(index).is_ok_and(|fd| {
                selected.contains(&fd) && !unreadable.contains(&fd)
                    || !selected.contains(&fd) && font.subfont(fd, &[]).is_ok()
            });
            own.get(index).copied().filter(|_| readable)
        })
        .collect();
    let lost = glyph_ids
        .filter(|glyph| {
            font.subfont_index(GlyphId::from(*glyph))
                .is_some_and(|fd| unreadable.contains(&fd))
                && font
                    .charstrings()
                    .get(usize::from(*glyph))
                    .is_ok_and(|charstring| calls_local_subr(charstring, &font, 0))
        })
        .collect();
    let replaced = font_dicts.iter().filter(|dict| dict.is_none()).count();
    let data = layout.with_font_dicts(data, &font_dicts);
    Ok(Some(FontDictRepair {
        data,
        glyphs,
        font_dicts: count,
        replaced,
        lost,
        fd_array_at,
    }))
}

/// Where a CFF's Top DICT sits, and what it says, read by hand.
///
/// `read-fonts` parses the DICT but does not say where the INDEX holding it begins and ends,
/// and the rewrite needs both; the INDEX format is a count, an offset size and an offset array
/// (Adobe Technical Note #5176, section 5), which is little enough to read here.
struct Layout {
    /// The Top DICT INDEX, header included.
    top_index: std::ops::Range<usize>,
    /// The Top DICT's entries: the operands' bytes and their operator.
    entries: Vec<(Vec<u8>, u16)>,
    fd_array_at: Option<usize>,
}

/// The two-byte escape's operators, as `escape + second byte`.
const ESCAPE: u16 = 0x0c00;
const OP_CHARSET: u16 = 15;
const OP_ENCODING: u16 = 16;
const OP_CHARSTRINGS: u16 = 17;
const OP_PRIVATE: u16 = 18;
const OP_FD_ARRAY: u16 = ESCAPE | 0x24;
const OP_FD_SELECT: u16 = ESCAPE | 0x25;

/// Where a rebuilt program's Top DICT is to point, for the offsets that are not simply shifted.
///
/// Every absolute offset a Top DICT states moves by the same `delta` when the Top DICT INDEX
/// changes length and the rest of the program is copied after it — except the one table the
/// rebuild has *relocated*, which is stated outright. One field per relocatable table rather
/// than a pair of numbers, so that a rebuild which forgets to say where it put something gets
/// the shifted offset rather than zero.
#[derive(Debug, Clone, Copy, Default)]
struct Moved {
    /// How far everything copied after the Top DICT INDEX has moved.
    delta: i64,
    /// Where the `FDArray` now is, where the rebuild wrote a fresh one.
    fd_array_at: Option<i64>,
    /// Where the `CharStrings` INDEX now is, where the rebuild wrote a fresh one.
    charstrings_at: Option<i64>,
}

impl Moved {
    /// Everything after the Top DICT INDEX shifted by `delta`, nothing relocated.
    const fn by(delta: i64) -> Self {
        Self {
            delta,
            fd_array_at: None,
            charstrings_at: None,
        }
    }

    /// The same, with a fresh `FDArray` at `at`.
    const fn fd_array_to(mut self, at: i64) -> Self {
        self.fd_array_at = Some(at);
        self
    }

    /// The same, with a fresh `CharStrings` INDEX at `at`.
    const fn charstrings_to(mut self, at: i64) -> Self {
        self.charstrings_at = Some(at);
        self
    }
}

impl Layout {
    fn read(data: &[u8]) -> Option<Self> {
        let header = usize::from(*data.get(2)?);
        let name_index = index_extent(data, header)?;
        let top_index = index_extent(data, name_index.end)?;
        let top_dict = index_item(data, top_index.start, 0)?;
        let entries = dict_entries(top_dict)?;
        let fd_array_at = entries
            .iter()
            .find(|(_, op)| *op == OP_FD_ARRAY)
            .and_then(|(operands, _)| dict_int(operands))
            .and_then(|at| usize::try_from(at).ok());
        Some(Self {
            top_index,
            entries,
            fd_array_at,
        })
    }

    /// The program with the Top DICT re-encoded and a fresh `FDArray` at the end: each `Some`
    /// the program's own Font DICT, kept with its offset shifted, each `None` an empty one.
    fn with_font_dicts(&self, data: &[u8], font_dicts: &[Option<&[u8]>]) -> Vec<u8> {
        // Pass one: the Top DICT with placeholder offsets, which has the length of the real one
        // because every offset is written in the five-byte form.
        let placeholder = self.top_dict(Moved::by(0).fd_array_to(0));
        let new_index_len = 3usize
            .saturating_add(2 * 4)
            .saturating_add(placeholder.len());
        let old_index_len = self.top_index.end.saturating_sub(self.top_index.start);
        let delta = i64::try_from(new_index_len)
            .unwrap_or(0)
            .saturating_sub(i64::try_from(old_index_len).unwrap_or(0));
        let fd_array_at = i64::try_from(data.len()).unwrap_or(0).saturating_add(delta);

        let top_dict = self.top_dict(Moved::by(delta).fd_array_to(fd_array_at));
        let mut out = Vec::with_capacity(data.len().saturating_add(64));
        out.extend_from_slice(data.get(..self.top_index.start).unwrap_or(data));
        push_index(&mut out, std::slice::from_ref(&top_dict));
        out.extend_from_slice(data.get(self.top_index.end..).unwrap_or(&[]));

        // The FDArray: an INDEX of Font DICTs. An empty Private DICT is size 0 at any offset the
        // data holds, and the FDArray's own is one.
        let dicts: Vec<Vec<u8>> = font_dicts
            .iter()
            .map(|dict| {
                dict.map_or_else(
                    || {
                        let mut empty = Vec::with_capacity(11);
                        empty.extend_from_slice(&int5(0));
                        empty.extend_from_slice(&int5(fd_array_at));
                        empty.push(u8::try_from(OP_PRIVATE).unwrap_or(18));
                        empty
                    },
                    |own| font_dict_shifted(own, delta),
                )
            })
            .collect();
        push_index(&mut out, &dicts);
        out
    }

    /// The program with a fresh `CharStrings` INDEX at its end and the Top DICT pointing there.
    ///
    /// The same move [`Self::with_font_dicts`] makes for the `FDArray`, and for the same reason:
    /// the Top DICT is re-encoded with every offset in the five-byte form, so its length is known
    /// before the offsets are, everything after the Top DICT INDEX is copied verbatim and shifted
    /// by the difference, and the table being replaced is appended past the end. The program's own
    /// `CharStrings` INDEX stays where it was and is no longer reached — Adobe Technical Note
    /// #5176 locates it by the Top DICT's offset alone, so bytes nothing points at are inert.
    fn with_charstrings(&self, data: &[u8], charstrings: &[Vec<u8>]) -> Vec<u8> {
        let placeholder = self.top_dict(Moved::by(0).charstrings_to(0));
        let new_index_len = 3usize
            .saturating_add(2 * 4)
            .saturating_add(placeholder.len());
        let old_index_len = self.top_index.end.saturating_sub(self.top_index.start);
        let delta = i64::try_from(new_index_len)
            .unwrap_or(0)
            .saturating_sub(i64::try_from(old_index_len).unwrap_or(0));
        let charstrings_at = i64::try_from(data.len()).unwrap_or(0).saturating_add(delta);
        let top_dict = self.top_dict(Moved::by(delta).charstrings_to(charstrings_at));

        let mut out = Vec::with_capacity(data.len().saturating_mul(2));
        out.extend_from_slice(data.get(..self.top_index.start).unwrap_or(data));
        push_index(&mut out, std::slice::from_ref(&top_dict));
        out.extend_from_slice(data.get(self.top_index.end..).unwrap_or(&[]));
        push_index(&mut out, charstrings);
        out
    }

    /// The Top DICT's bytes, every offset it states restated under `moved`.
    fn top_dict(&self, moved: Moved) -> Vec<u8> {
        let delta = moved.delta;
        let mut out = Vec::new();
        for (operands, op) in &self.entries {
            match *op {
                // A charset or Encoding operand below the first custom offset names a
                // predefined table (ISOAdobe/Expert/ExpertSubset, standard/expert) and is
                // not an offset.
                OP_CHARSET | OP_ENCODING => {
                    let at = dict_int(operands).unwrap_or(0);
                    let custom = if *op == OP_CHARSET { at > 2 } else { at > 1 };
                    out.extend_from_slice(&int5(if custom {
                        at.saturating_add(delta)
                    } else {
                        at
                    }));
                }
                OP_CHARSTRINGS => out.extend_from_slice(&int5(
                    moved
                        .charstrings_at
                        .unwrap_or_else(|| dict_int(operands).unwrap_or(0).saturating_add(delta)),
                )),
                OP_FD_SELECT => {
                    out.extend_from_slice(&int5(
                        dict_int(operands).unwrap_or(0).saturating_add(delta),
                    ));
                }
                OP_PRIVATE => {
                    let (size, at) = dict_two_ints(operands).unwrap_or((0, 0));
                    out.extend_from_slice(&int5(size));
                    out.extend_from_slice(&int5(at.saturating_add(delta)));
                }
                OP_FD_ARRAY => out.extend_from_slice(&int5(
                    moved
                        .fd_array_at
                        .unwrap_or_else(|| dict_int(operands).unwrap_or(0).saturating_add(delta)),
                )),
                _ => out.extend_from_slice(operands),
            }
            push_operator(&mut out, *op);
        }
        out
    }
}

/// A Font DICT re-encoded with its Private DICT's offset — the one absolute offset it states —
/// shifted by `delta`; every other entry verbatim. A DICT that does not parse is kept as it is.
fn font_dict_shifted(dict: &[u8], delta: i64) -> Vec<u8> {
    let Some(entries) = dict_entries(dict) else {
        return dict.to_vec();
    };
    let mut out = Vec::with_capacity(dict.len().saturating_add(8));
    for (operands, op) in &entries {
        if *op == OP_PRIVATE {
            let (size, at) = dict_two_ints(operands).unwrap_or((0, 0));
            out.extend_from_slice(&int5(size));
            out.extend_from_slice(&int5(at.saturating_add(delta)));
        } else {
            out.extend_from_slice(operands);
        }
        push_operator(&mut out, *op);
    }
    out
}

/// Writes an INDEX holding `items`, header and data (Adobe Technical Note #5176, section 5).
///
/// The offset size is always four, which holds any offset a CFF can state, so the header's
/// length is a function of the item count alone — which is what lets a rebuild work out where
/// the INDEX ends before it knows how long the items are.
fn push_index(out: &mut Vec<u8>, items: &[Vec<u8>]) {
    out.extend_from_slice(&u16::try_from(items.len()).unwrap_or(u16::MAX).to_be_bytes());
    if items.is_empty() {
        // "An empty INDEX is represented by a count field with a 0 value and no additional
        // fields", so an offset size would be one byte too many.
        return;
    }
    out.push(4);
    let mut at = 1u32;
    out.extend_from_slice(&at.to_be_bytes());
    for item in items {
        at = at.saturating_add(u32::try_from(item.len()).unwrap_or(u32::MAX));
        out.extend_from_slice(&at.to_be_bytes());
    }
    for item in items {
        out.extend_from_slice(item);
    }
}

/// Writes a DICT operator, in its one- or two-byte form.
fn push_operator(out: &mut Vec<u8>, op: u16) {
    if op & ESCAPE == ESCAPE {
        out.push(12);
    }
    out.push(u8::try_from(op & 0xff).unwrap_or(0));
}

/// A DICT integer in the five-byte form, which holds any offset a CFF can state.
fn int5(value: i64) -> [u8; 5] {
    let value = i32::try_from(value).unwrap_or(i32::MAX);
    let [a, b, c, d] = value.to_be_bytes();
    [29, a, b, c, d]
}

/// The extent of an INDEX beginning at `at`, header and data both.
fn index_extent(data: &[u8], at: usize) -> Option<std::ops::Range<usize>> {
    let count = usize::from(u16::from_be_bytes([
        *data.get(at)?,
        *data.get(at.checked_add(1)?)?,
    ]));
    if count == 0 {
        return Some(at..at.checked_add(2)?);
    }
    let off_size = usize::from(*data.get(at.checked_add(2)?)?);
    if !(1..=4).contains(&off_size) {
        return None;
    }
    let offsets = at.checked_add(3)?;
    let base = offsets
        .checked_add(count.checked_add(1)?.checked_mul(off_size)?)?
        .checked_sub(1)?;
    let last = index_offset(data, offsets, off_size, count)?;
    Some(at..base.checked_add(last)?)
}

/// The `n`th offset of an INDEX whose offset array begins at `offsets`.
fn index_offset(data: &[u8], offsets: usize, off_size: usize, n: usize) -> Option<usize> {
    let at = offsets.checked_add(n.checked_mul(off_size)?)?;
    let mut value = 0usize;
    for byte in data.get(at..at.checked_add(off_size)?)? {
        value = value.checked_shl(8)?.checked_add(usize::from(*byte))?;
    }
    Some(value)
}

/// Item `n` of the INDEX beginning at `at`.
fn index_item(data: &[u8], at: usize, n: usize) -> Option<&[u8]> {
    index_items(data, at)?.get(n).copied()
}

/// Every item of the INDEX beginning at `at`, or `None` where no INDEX reads there.
fn index_items(data: &[u8], at: usize) -> Option<Vec<&[u8]>> {
    let count = usize::from(u16::from_be_bytes([
        *data.get(at)?,
        *data.get(at.checked_add(1)?)?,
    ]));
    if count == 0 {
        return Some(Vec::new());
    }
    let off_size = usize::from(*data.get(at.checked_add(2)?)?);
    if !(1..=4).contains(&off_size) {
        return None;
    }
    let offsets = at.checked_add(3)?;
    let base = offsets
        .checked_add(count.checked_add(1)?.checked_mul(off_size)?)?
        .checked_sub(1)?;
    let mut items = Vec::with_capacity(count);
    for n in 0..count {
        let start = base.checked_add(index_offset(data, offsets, off_size, n)?)?;
        let end = base.checked_add(index_offset(data, offsets, off_size, n.checked_add(1)?)?)?;
        items.push(data.get(start..end)?);
    }
    Some(items)
}

/// A DICT's entries as (operand bytes, operator), the operator carrying [`ESCAPE`] for the
/// two-byte form (Adobe Technical Note #5176, section 4).
fn dict_entries(dict: &[u8]) -> Option<Vec<(Vec<u8>, u16)>> {
    let mut entries = Vec::new();
    let mut operands = Vec::new();
    let mut i = 0usize;
    while let Some(&b0) = dict.get(i) {
        let width = match b0 {
            0..=11 | 13..=21 => {
                entries.push((std::mem::take(&mut operands), u16::from(b0)));
                i = i.checked_add(1)?;
                continue;
            }
            12 => {
                let b1 = *dict.get(i.checked_add(1)?)?;
                entries.push((std::mem::take(&mut operands), ESCAPE | u16::from(b1)));
                i = i.checked_add(2)?;
                continue;
            }
            28 => 3,
            29 => 5,
            30 => {
                // A real number: nibbles until one is 0xf.
                let mut end = i.checked_add(1)?;
                loop {
                    let nibbles = *dict.get(end)?;
                    end = end.checked_add(1)?;
                    if nibbles & 0x0f == 0x0f || nibbles >> 4 == 0x0f {
                        break;
                    }
                }
                end.checked_sub(i)?
            }
            32..=246 => 1,
            247..=254 => 2,
            22..=27 | 31 | 255 => return None,
        };
        operands.extend_from_slice(dict.get(i..i.checked_add(width)?)?);
        i = i.checked_add(width)?;
    }
    Some(entries)
}

/// The value a one- or two-byte operand encodes and its width, for a first byte in
/// `32..=254` — the forms a DICT and a Type 2 charstring share (Adobe Technical Note #5176,
/// Table 3; #5177, section 3.2). `b1` is read only for the two-byte forms.
fn small_operand(b0: u8, b1: u8) -> (i64, usize) {
    let (b0, b1) = (i64::from(b0), i64::from(b1));
    match b0 {
        32..=246 => (b0.saturating_sub(139), 1),
        247..=250 => (
            b0.saturating_sub(247)
                .saturating_mul(256)
                .saturating_add(b1)
                .saturating_add(108),
            2,
        ),
        _ => (
            b0.saturating_sub(251)
                .saturating_mul(256)
                .saturating_add(b1)
                .saturating_add(108)
                .saturating_neg(),
            2,
        ),
    }
}

/// The integers a DICT's operand bytes encode, in order; a real number ends the list.
fn dict_ints(operands: &[u8]) -> Vec<i64> {
    let mut values = Vec::new();
    let mut i = 0usize;
    while let Some(&b0) = operands.get(i) {
        let (value, width) = match b0 {
            32..=246 => small_operand(b0, 0),
            247..=254 => {
                let Some(&b1) = operands.get(i.saturating_add(1)) else {
                    break;
                };
                small_operand(b0, b1)
            }
            28 => {
                let Some(bytes) = operands.get(i.saturating_add(1)..i.saturating_add(3)) else {
                    break;
                };
                (i64::from(i16::from_be_bytes([bytes[0], bytes[1]])), 3)
            }
            29 => {
                let Some(bytes) = operands.get(i.saturating_add(1)..i.saturating_add(5)) else {
                    break;
                };
                (
                    i64::from(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
                    5,
                )
            }
            _ => break,
        };
        values.push(value);
        i = i.saturating_add(width);
    }
    values
}

fn dict_int(operands: &[u8]) -> Option<i64> {
    dict_ints(operands).first().copied()
}

fn dict_two_ints(operands: &[u8]) -> Option<(i64, i64)> {
    let values = dict_ints(operands);
    Some((*values.first()?, *values.get(1)?))
}

/// Whether a Type 2 charstring reaches `callsubr`, itself or through the global subroutines it
/// calls (Adobe Technical Note #5177, section 4.1).
///
/// The operand stack is followed far enough to know which global subroutine a `callgsubr`
/// names; a call whose operand cannot be told is counted as reaching one, which errs on the
/// side of reporting. The stem count is kept for `hintmask`'s data bytes, which would
/// otherwise be read as operators.
fn calls_local_subr(charstring: &[u8], font: &CffFontRef<'_>, depth: u8) -> bool {
    const CALLSUBR: u8 = 10;
    const RETURN: u8 = 11;
    const CALLGSUBR: u8 = 29;
    const ENDCHAR: u8 = 14;
    const HSTEM: u8 = 1;
    const VSTEM: u8 = 3;
    const HSTEMHM: u8 = 18;
    const VSTEMHM: u8 = 23;
    const HINTMASK: u8 = 19;
    const CNTRMASK: u8 = 20;
    const ESCAPE_BYTE: u8 = 12;
    // Type 2 bounds subroutine nesting to ten levels.
    const MAX_DEPTH: u8 = 10;

    let gsubrs = font.global_subrs();
    let bias: i64 = match gsubrs.count() {
        0..1240 => 107,
        1240..33900 => 1131,
        _ => 32768,
    };
    let mut stack: Vec<i64> = Vec::new();
    let mut stems = 0usize;
    let mut i = 0usize;
    while let Some(&b0) = charstring.get(i) {
        match b0 {
            32..=254 => {
                let b1 = charstring.get(i.saturating_add(1)).copied().unwrap_or(0);
                let (value, width) = small_operand(b0, b1);
                stack.push(value);
                i = i.saturating_add(width);
            }
            28 => {
                let value = charstring
                    .get(i.saturating_add(1)..i.saturating_add(3))
                    .map_or(0, |b| i64::from(i16::from_be_bytes([b[0], b[1]])));
                stack.push(value);
                i = i.saturating_add(3);
            }
            255 => {
                // 16.16 fixed; only the integer part could name a subroutine.
                let value = charstring
                    .get(i.saturating_add(1)..i.saturating_add(5))
                    .map_or(0, |b| {
                        i64::from(i32::from_be_bytes([b[0], b[1], b[2], b[3]])) >> 16
                    });
                stack.push(value);
                i = i.saturating_add(5);
            }
            CALLSUBR => return true,
            CALLGSUBR => {
                let Some(operand) = stack.pop() else {
                    return true;
                };
                let index = usize::try_from(operand.saturating_add(bias)).unwrap_or(usize::MAX);
                if depth < MAX_DEPTH
                    && gsubrs
                        .get(index)
                        .is_ok_and(|subr| calls_local_subr(subr, font, depth.saturating_add(1)))
                {
                    return true;
                }
                i = i.saturating_add(1);
            }
            RETURN | ENDCHAR => return false,
            HSTEM | VSTEM | HSTEMHM | VSTEMHM => {
                stems = stems.saturating_add(stack.len() / 2);
                stack.clear();
                i = i.saturating_add(1);
            }
            HINTMASK | CNTRMASK => {
                stems = stems.saturating_add(stack.len() / 2);
                stack.clear();
                i = i
                    .saturating_add(1)
                    .saturating_add(stems.saturating_add(7) / 8);
            }
            ESCAPE_BYTE => {
                stack.clear();
                i = i.saturating_add(2);
            }
            _ => {
                stack.clear();
                i = i.saturating_add(1);
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------------------------
// Restating a program's advances, which is the one thing a converter may change about a font.
// ---------------------------------------------------------------------------------------------

/// The Private DICT operator naming the advance every charstring's width is stated against
/// (Adobe Technical Note #5176, Table 23: `nominalWidthX`, operator 21, default 0).
const OP_NOMINAL_WIDTH: u16 = 21;

/// The Type 2 operators that clear the operand stack, and may therefore carry a width.
///
/// Adobe Technical Note #5177, section 3.1: "The first stack-clearing operator, which must be
/// one of hstem, hstemhm, cntrmask, hintmask, hmoveto, vmoveto, rmoveto, or endchar, takes an
/// additional argument — the width — which may be expressed as zero or one numeric argument."
/// `vstem` and `vstemhm` are not in that sentence's list and are stack-clearing all the same, so
/// they are treated the same way: a charstring beginning with one is the shape the sentence
/// describes, whatever the list leaves out.
mod stack_clearing {
    /// `hstem`.
    pub(super) const HSTEM: u8 = 1;
    /// `vstem`.
    pub(super) const VSTEM: u8 = 3;
    /// `vmoveto`.
    pub(super) const VMOVETO: u8 = 4;
    /// `rmoveto`.
    pub(super) const RMOVETO: u8 = 21;
    /// `hmoveto`.
    pub(super) const HMOVETO: u8 = 22;
    /// `endchar`.
    pub(super) const ENDCHAR: u8 = 14;
    /// `hstemhm`.
    pub(super) const HSTEMHM: u8 = 18;
    /// `hintmask`.
    pub(super) const HINTMASK: u8 = 19;
    /// `cntrmask`.
    pub(super) const CNTRMASK: u8 = 20;
    /// `vstemhm`.
    pub(super) const VSTEMHM: u8 = 23;
}

/// Where a Type 2 charstring states its width, where the rule can say at all.
///
/// Three answers rather than two, and the third is what keeps this honest: a charstring whose
/// first operator is not one of the stack-clearing set — a `callsubr` before any of them being
/// the shape that occurs — states its width inside a subroutine, and a caller must follow the
/// call rather than prepend a second width.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LeadingWidth {
    /// The width is the operand occupying this span.
    At(std::ops::Range<usize>),
    /// The charstring states no width, so one is inserted rather than replaced.
    Absent,
}

/// Where a Type 2 charstring's leading width operand is, where it has one.
///
/// `None` is [`LeadingWidth`]'s third answer: the width is not in this charstring at all.
///
/// The rule is positional (Adobe Technical Note #5177, section 3.1): the width is present when
/// the stack holds one operand more than the operator takes.
fn width_span(charstring: &[u8]) -> Option<LeadingWidth> {
    use stack_clearing::{
        CNTRMASK, ENDCHAR, HINTMASK, HMOVETO, HSTEM, HSTEMHM, RMOVETO, VMOVETO, VSTEM, VSTEMHM,
    };
    let mut operands: Vec<std::ops::Range<usize>> = Vec::new();
    let mut at = 0usize;
    while let Some(&b0) = charstring.get(at) {
        let width = match b0 {
            28 => 3,
            32..=246 => 1,
            247..=254 => 2,
            255 => 5,
            HSTEM | VSTEM | HSTEMHM | VSTEMHM | HINTMASK | CNTRMASK => {
                return Some(has_width(&operands, operands.len() % 2 == 1));
            }
            RMOVETO => return Some(has_width(&operands, operands.len() > 2)),
            HMOVETO | VMOVETO => return Some(has_width(&operands, operands.len() > 1)),
            ENDCHAR => {
                return Some(has_width(
                    &operands,
                    operands.len() == 1 || operands.len() == 5,
                ));
            }
            _ => return None,
        };
        let end = at.checked_add(width)?;
        charstring.get(at..end)?;
        operands.push(at..end);
        at = end;
    }
    None
}

/// The first operand's span where `present`, which is where the width sits when there is one.
fn has_width(operands: &[std::ops::Range<usize>], present: bool) -> LeadingWidth {
    match operands.first().filter(|_| present) {
        Some(span) => LeadingWidth::At(span.clone()),
        None => LeadingWidth::Absent,
    }
}

/// The `Subrs` operator of a Private DICT (Adobe Technical Note #5176, Table 23, operator 19),
/// whose operand is an offset **relative to the Private DICT's own start**.
const OP_SUBRS: u16 = 19;

/// `callsubr` and `callgsubr` (Adobe Technical Note #5177, section 4.7).
const CALL_LOCAL: u8 = 10;
/// `callgsubr`.
const CALL_GLOBAL: u8 = 29;
/// `return`.
const RETURN: u8 = 11;
/// The two-byte operator escape.
const ESCAPE_BYTE: u8 = 12;

/// How many subroutine calls are followed before a charstring's width is given up on.
///
/// A subroutinizer factors a common *prefix*, so the width is reached in one or two calls in
/// every program this has been run against; the bound is what keeps a program whose subroutines
/// call each other in a ring from being followed for ever.
const MAX_INLINE_DEPTH: usize = 8;

/// The subroutine bias, which Adobe Technical Note #5177 section 4.7 makes a function of how
/// many subroutines the INDEX holds.
fn bias(count: usize) -> i64 {
    if count < 1240 {
        107
    } else if count < 33900 {
        1131
    } else {
        32768
    }
}

/// The subroutines a charstring can call: the program's global INDEX and one Font DICT's local.
struct Subroutines<'a> {
    /// The Global Subr INDEX, which every charstring of the program shares.
    global: &'a [Vec<u8>],
    /// The Local Subr INDEX of the Private DICT this glyph's charstring is read against.
    local: &'a [Vec<u8>],
}

/// How many bytes of a subroutine are its body, with a trailing `return` left off.
///
/// A subroutine is spliced into a charstring in place of the call, which is what `callsubr`
/// does; the `return` that ends it has nothing to return to once it is inlined, so it is
/// dropped. Finding it means reading the subroutine as Type 2 rather than looking at its last
/// byte, because `hintmask`'s mask bytes follow the operator as data and one of them may be 11.
///
/// `None` where the subroutine cannot be read that far: a mask whose length depends on stems
/// counted inside a *nested* call is one this does not follow, and a subroutine holding both is
/// declined rather than guessed at.
fn subroutine_body(subr: &[u8]) -> Option<usize> {
    let mut stems = 0usize;
    let mut operands = 0usize;
    let mut called = false;
    let mut at = 0usize;
    let mut last: Option<(usize, u8)> = None;
    while let Some(&b0) = subr.get(at) {
        last = Some((at, b0));
        let step = match b0 {
            28 => {
                operands = operands.saturating_add(1);
                3
            }
            32..=246 => {
                operands = operands.saturating_add(1);
                1
            }
            247..=254 => {
                operands = operands.saturating_add(1);
                2
            }
            255 => {
                operands = operands.saturating_add(1);
                5
            }
            stack_clearing::HSTEM
            | stack_clearing::VSTEM
            | stack_clearing::HSTEMHM
            | stack_clearing::VSTEMHM => {
                stems = stems.saturating_add(operands / 2);
                operands = 0;
                1
            }
            stack_clearing::HINTMASK | stack_clearing::CNTRMASK => {
                if called {
                    return None;
                }
                stems = stems.saturating_add(operands / 2);
                operands = 0;
                1usize.saturating_add(stems.div_ceil(8))
            }
            ESCAPE_BYTE => {
                operands = 0;
                2
            }
            CALL_LOCAL | CALL_GLOBAL => {
                called = true;
                operands = operands.saturating_sub(1);
                1
            }
            _ => {
                operands = 0;
                1
            }
        };
        at = at.checked_add(step)?;
        if at > subr.len() {
            return None;
        }
    }
    match last {
        Some((start, RETURN)) => Some(start),
        Some(_) => Some(subr.len()),
        None => None,
    }
}

/// The charstring with the leading subroutine call it begins with spliced out.
///
/// `None` where the head is not a call, where the subroutine is not there, or where
/// [`subroutine_body`] declines it. Splicing is exactly what the interpreter does, so the outline
/// the charstring draws is unchanged — which `an_inlined_charstring_draws_what_the_call_drew`
/// checks over every glyph of the compiled-in faces rather than asserting here.
fn inline_leading_call(charstring: &[u8], subrs: &Subroutines<'_>) -> Option<Vec<u8>> {
    let mut operands: Vec<std::ops::Range<usize>> = Vec::new();
    let mut at = 0usize;
    while let Some(&b0) = charstring.get(at) {
        let width = match b0 {
            28 => 3,
            32..=246 => 1,
            247..=254 => 2,
            255 => 5,
            CALL_LOCAL | CALL_GLOBAL => {
                let index = operands.last()?;
                let named = dict_ints(charstring.get(index.clone())?).first().copied()?;
                let table = if b0 == CALL_LOCAL {
                    subrs.local
                } else {
                    subrs.global
                };
                let chosen = usize::try_from(named.checked_add(bias(table.len()))?).ok()?;
                let body = table.get(chosen)?;
                let keep = subroutine_body(body)?;
                let mut out = Vec::with_capacity(charstring.len().saturating_add(body.len()));
                out.extend_from_slice(charstring.get(..index.start)?);
                out.extend_from_slice(body.get(..keep)?);
                out.extend_from_slice(charstring.get(at.checked_add(1)?..)?);
                return Some(out);
            }
            _ => return None,
        };
        let end = at.checked_add(width)?;
        charstring.get(at..end)?;
        operands.push(at..end);
        at = end;
    }
    None
}

/// One charstring with its width operand restated as `advance`, in the program's own units.
///
/// The outline is untouched: what changes is the one leading operand Adobe Technical Note #5177
/// section 3.1 defines, written in the three-byte form so that its length does not depend on its
/// value. `None` where the charstring states its width through a subroutine ([`width_span`]) or
/// where the difference from `nominal` will not fit a 16-bit operand.
fn charstring_with_advance(
    charstring: &[u8],
    advance: i64,
    nominal: i64,
    subrs: &Subroutines<'_>,
) -> Option<Vec<u8>> {
    // A subroutinized program factors the width into a subroutine, so the call at the head is
    // spliced out until the width is where Adobe Technical Note #5177 section 3.1 puts it.
    let mut charstring = charstring.to_vec();
    for _ in 0..MAX_INLINE_DEPTH {
        if width_span(&charstring).is_some() {
            break;
        }
        charstring = inline_leading_call(&charstring, subrs)?;
    }
    let charstring = charstring.as_slice();
    let span = width_span(charstring)?;
    let value = i16::try_from(advance.checked_sub(nominal)?).ok()?;
    let mut out = Vec::with_capacity(charstring.len().saturating_add(3));
    out.push(28);
    out.extend_from_slice(&value.to_be_bytes());
    match span {
        LeadingWidth::At(span) => {
            out.extend_from_slice(charstring.get(..span.start)?);
            out.extend_from_slice(charstring.get(span.end..)?);
        }
        LeadingWidth::Absent => out.extend_from_slice(charstring),
    }
    Some(out)
}

/// A DICT's `nominalWidthX`, which is zero where the DICT does not state one.
fn nominal_width(private: &[u8]) -> i64 {
    dict_entries(private)
        .unwrap_or_default()
        .iter()
        .find(|(_, op)| *op == OP_NOMINAL_WIDTH)
        .and_then(|(operands, _)| dict_int(operands))
        .unwrap_or(0)
}

/// The Private DICT a DICT's `Private` entry names, as a slice of the program.
fn private_dict<'a>(data: &'a [u8], dict: &[u8]) -> Option<&'a [u8]> {
    let (size, at) = dict_entries(dict)?
        .iter()
        .find(|(_, op)| *op == OP_PRIVATE)
        .and_then(|(operands, _)| dict_two_ints(operands))?;
    let at = usize::try_from(at).ok()?;
    let size = usize::try_from(size).ok()?;
    data.get(at..at.checked_add(size)?)
}

/// The Global Subr INDEX, which sits directly after the String INDEX (Adobe Technical Note
/// #5176, section 1: the header, then the Name, Top DICT, String and Global Subr INDEXes).
fn global_subroutines(data: &[u8]) -> Vec<Vec<u8>> {
    let read = || -> Option<Vec<Vec<u8>>> {
        let header = usize::from(*data.get(2)?);
        let name = index_extent(data, header)?;
        let top = index_extent(data, name.end)?;
        let strings = index_extent(data, top.end)?;
        Some(
            index_items(data, strings.end)?
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect(),
        )
    };
    read().unwrap_or_default()
}

/// The Local Subr INDEX a Private DICT names, whose offset the DICT states relative to itself.
fn local_subroutines(data: &[u8], dict: &[u8]) -> Vec<Vec<u8>> {
    let read = || -> Option<Vec<Vec<u8>>> {
        let (size, at) = dict_entries(dict)?
            .iter()
            .find(|(_, op)| *op == OP_PRIVATE)
            .and_then(|(operands, _)| dict_two_ints(operands))?;
        let at = usize::try_from(at).ok()?;
        let private = data.get(at..at.checked_add(usize::try_from(size).ok()?)?)?;
        let relative = dict_entries(private)?
            .iter()
            .find(|(_, op)| *op == OP_SUBRS)
            .and_then(|(operands, _)| dict_int(operands))?;
        Some(
            index_items(data, at.checked_add(usize::try_from(relative).ok()?)?)?
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect(),
        )
    };
    read().unwrap_or_default()
}

/// Every `nominalWidthX` the program states, by Font DICT index, and the one a glyph uses.
///
/// A name-keyed program has one Private DICT, named by the Top DICT; a CID-keyed one has a
/// Private DICT per Font DICT, and `FDSelect` says which glyph uses which (Adobe Technical Note
/// #5176, sections 15, 18 and 19). Both are read here so that a rebuilt charstring's width
/// operand is stated against the same number the reader will subtract it from.
fn nominal_widths(data: &[u8], layout: &Layout, cid_keyed: bool) -> Vec<i64> {
    if !cid_keyed {
        let top = index_item(data, layout.top_index.start, 0).unwrap_or_default();
        return vec![private_dict(data, top).map_or(0, nominal_width)];
    }
    index_items(data, layout.fd_array_at.unwrap_or(0))
        .unwrap_or_default()
        .iter()
        .map(|dict| private_dict(data, dict).map_or(0, nominal_width))
        .collect()
}

/// The program with the advance of each named glyph restated, in the program's own units.
///
/// **What section 4.9 of `doc/pdf-a-conversion-limits.md` calls the second metric route.**
/// ISO 19005-2
/// section 6.2.11.5 and ISO 19005-4 section 6.2.10.5 require a font dictionary's stated widths
/// and its embedded program's own to agree, and ISO 32000-2 §9.2.4 says which of the two
/// positions the glyphs:
///
/// > Storing this information in the font dictionary, although redundant, enables a PDF processor
/// > to determine glyph positioning without having to look inside the font program.
///
/// So the program's number is the one that may move, and this is the move: the leading width
/// operand of each named glyph's charstring is restated and every other byte of the charstring —
/// every byte of the outline — is copied. A glyph not named keeps the advance it had.
///
/// # Errors
///
/// [`CffError::Malformed`] where the program does not open or its `CharStrings` INDEX does not
/// read, and [`CffError::AdvanceNotRestatable`] where a named glyph's charstring states its width
/// somewhere this does not follow.
pub fn with_advances(data: &[u8], advances: &BTreeMap<u16, i64>) -> Result<Vec<u8>, CffError> {
    let font = open(data)?;
    let layout = Layout::read(data).ok_or_else(|| CffError::Malformed {
        detail: "the Top DICT INDEX could not be located".to_owned(),
    })?;
    let charstrings_at = layout
        .entries
        .iter()
        .find(|(_, op)| *op == OP_CHARSTRINGS)
        .and_then(|(operands, _)| dict_int(operands))
        .and_then(|at| usize::try_from(at).ok())
        .ok_or_else(|| CffError::Malformed {
            detail: "the Top DICT states no CharStrings offset".to_owned(),
        })?;
    let mut charstrings: Vec<Vec<u8>> = index_items(data, charstrings_at)
        .ok_or_else(|| CffError::Malformed {
            detail: "the CharStrings INDEX could not be read".to_owned(),
        })?
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect();
    let cid_keyed = font.is_cid();
    let nominal = nominal_widths(data, &layout, cid_keyed);
    let global = global_subroutines(data);
    // One Local Subr INDEX per Font DICT, read once: a charstring's `callsubr` reaches the
    // subroutines of the Private DICT it is interpreted against and no other.
    let dicts: Vec<Vec<u8>> = if cid_keyed {
        index_items(data, layout.fd_array_at.unwrap_or(0))
            .unwrap_or_default()
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect()
    } else {
        vec![
            index_item(data, layout.top_index.start, 0)
                .unwrap_or_default()
                .to_vec(),
        ]
    };
    let locals: Vec<Vec<Vec<u8>>> = dicts
        .iter()
        .map(|dict| local_subroutines(data, dict))
        .collect();
    for (glyph, advance) in advances {
        let slot =
            charstrings
                .get_mut(usize::from(*glyph))
                .ok_or(CffError::AdvanceNotRestatable {
                    glyph: *glyph,
                    detail: "the program's CharStrings INDEX has no entry for it".to_owned(),
                })?;
        let subfont = if cid_keyed {
            font.subfont_index(GlyphId::from(*glyph)).unwrap_or(0)
        } else {
            0
        };
        let against = nominal.get(usize::from(subfont)).copied().unwrap_or(0);
        let subrs = Subroutines {
            global: &global,
            local: locals
                .get(usize::from(subfont))
                .map_or(&[][..], Vec::as_slice),
        };
        *slot = charstring_with_advance(slot, *advance, against, &subrs).ok_or(
            CffError::AdvanceNotRestatable {
                glyph: *glyph,
                detail: "its charstring does not begin with one of the stack-clearing operators \
                         a width is stated against, so the width is inside a subroutine"
                    .to_owned(),
            },
        )?;
    }
    Ok(layout.with_charstrings(data, &charstrings))
}

#[cfg(test)]
mod font_dict_repair {
    use super::*;

    /// Counts a pen's segments, which is all a test needs to know an outline was drawn.
    struct Segments(u32);
    impl OutlinePen for Segments {
        fn move_to(&mut self, _x: f32, _y: f32) {
            self.0 = self.0.saturating_add(1);
        }
        fn line_to(&mut self, _x: f32, _y: f32) {
            self.0 = self.0.saturating_add(1);
        }
        fn quad_to(&mut self, _cx: f32, _cy: f32, _x: f32, _y: f32) {
            self.0 = self.0.saturating_add(1);
        }
        fn curve_to(&mut self, _a: f32, _b: f32, _c: f32, _d: f32, _x: f32, _y: f32) {
            self.0 = self.0.saturating_add(1);
        }
        fn close(&mut self) {}
    }

    /// A 500-unit square: `100 100 rmoveto 500 hlineto 500 vlineto -500 hlineto endchar`.
    const SQUARE: &[u8] = &[239, 239, 21, 248, 136, 6, 248, 136, 7, 252, 136, 6, 14];
    /// `0 callsubr endchar`: a glyph that needs its Private DICT's local subroutines.
    const NEEDS_SUBR: &[u8] = &[139, 10, 14];
    /// `-107 callgsubr endchar`: global subroutine 0, under the bias a small INDEX has.
    const VIA_GSUBR: &[u8] = &[32, 29, 14];

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a fixture builder over a font of two glyphs: every quantity is a few hundred, \
                  and a wrong table is what the assertions exist to catch"
    )]
    fn index(items: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(
            &u16::try_from(items.len())
                .expect("a few items")
                .to_be_bytes(),
        );
        if items.is_empty() {
            return out;
        }
        out.push(1);
        let mut at = 1u8;
        out.push(at);
        for item in items {
            at += u8::try_from(item.len()).expect("small items");
            out.push(at);
        }
        for item in items {
            out.extend_from_slice(item);
        }
        out
    }

    /// A two-glyph CID-keyed CFF — `.notdef` and `glyph`, the latter CID 5 — whose `FDArray`
    /// is well formed when `fd_array_readable`, and otherwise points into the empty global
    /// subroutine INDEX exactly as `FOP-2736-4.pdf`'s points into its `FDSelect`: a zero
    /// count where a Font DICT INDEX should be.
    #[expect(
        clippy::cast_possible_wrap,
        clippy::arithmetic_side_effects,
        reason = "a fixture builder over a font of two glyphs: every quantity is a few hundred, \
                  and a wrong table is what the assertions exist to catch"
    )]
    fn cid_keyed(glyph: &[u8], gsubr: Option<&[u8]>, fd_array_readable: bool) -> Vec<u8> {
        let header = [1u8, 0, 4, 4];
        let names = index(&[b"F"]);
        let strings = index(&[b"Adobe", b"Identity"]);
        let gsubrs = gsubr.map_or_else(|| index(&[]), |subr| index(&[subr]));
        let charset = [0u8, 0, 5];
        let fd_select = [3u8, 0, 1, 0, 0, 0, 0, 2];
        let charstrings = index(&[&[14], glyph]);
        // ROS, charset, FDSelect, CharStrings, FDArray: 17 + 6 + 7 + 6 + 7 bytes.
        let top_len = 43;
        let top_index_len = 3 + 2 + top_len;
        let strings_at = header.len() + names.len() + top_index_len;
        let gsubrs_at = strings_at + strings.len();
        let charset_at = gsubrs_at + gsubrs.len();
        let fd_select_at = charset_at + charset.len();
        let charstrings_at = fd_select_at + fd_select.len();
        let fd_array_at = charstrings_at + charstrings.len();
        let mut top = Vec::new();
        top.extend_from_slice(&int5(391));
        top.extend_from_slice(&int5(392));
        top.extend_from_slice(&int5(0));
        top.extend_from_slice(&[12, 30]);
        top.extend_from_slice(&int5(charset_at as i64));
        top.push(15);
        top.extend_from_slice(&int5(fd_select_at as i64));
        top.extend_from_slice(&[12, 37]);
        top.extend_from_slice(&int5(charstrings_at as i64));
        top.push(17);
        top.extend_from_slice(&int5(if fd_array_readable {
            fd_array_at
        } else {
            gsubrs_at
        } as i64));
        top.extend_from_slice(&[12, 36]);
        assert_eq!(top.len(), top_len);
        let mut font_dict = Vec::new();
        font_dict.extend_from_slice(&int5(0));
        font_dict.extend_from_slice(&int5(fd_array_at as i64));
        font_dict.push(18);
        let fd_array = index(&[&font_dict]);

        let mut out = Vec::new();
        out.extend_from_slice(&header);
        out.extend_from_slice(&names);
        out.extend_from_slice(&index(&[&top]));
        out.extend_from_slice(&strings);
        out.extend_from_slice(&gsubrs);
        out.extend_from_slice(&charset);
        out.extend_from_slice(&fd_select);
        out.extend_from_slice(&charstrings);
        out.extend_from_slice(&fd_array);
        out
    }

    fn segments(data: &[u8], glyph: u16) -> Result<u32, CffError> {
        let mut pen = Segments(0);
        draw(data, glyph, &mut pen).map(|()| pen.0)
    }

    /// `-107 callsubr endchar`: local subroutine 0, under the bias a small INDEX has.
    const VIA_SUBR: &[u8] = &[32, 10, 14];

    /// A three-glyph CID-keyed CFF with two Font DICTs: glyph 1 (CID 5) under FD 0, whose
    /// Private DICT holds one local subroutine, and glyph 2 (CID 6) under FD 1 — which the
    /// `FDArray` holds only when `fd1_present`. `issue9278.pdf`'s shape: a readable Font DICT
    /// with subroutines its glyphs call, beside one a glyph selects and nobody can read.
    #[expect(
        clippy::cast_possible_wrap,
        clippy::arithmetic_side_effects,
        reason = "a fixture builder over a font of three glyphs: every quantity is a few \
                  hundred, and a wrong table is what the assertions exist to catch"
    )]
    fn two_fd_cid_keyed(glyph1: &[u8], glyph2: &[u8], subr: &[u8], fd1_present: bool) -> Vec<u8> {
        let header = [1u8, 0, 4, 4];
        let names = index(&[b"F"]);
        let strings = index(&[b"Adobe", b"Identity"]);
        let gsubrs = index(&[]);
        let charset = [0u8, 0, 5, 0, 6];
        let fd_select = [3u8, 0, 2, 0, 0, 0, 0, 2, 1, 0, 3];
        let charstrings = index(&[&[14], glyph1, glyph2]);
        let top_len = 43;
        let top_index_len = 3 + 2 + top_len;
        let strings_at = header.len() + names.len() + top_index_len;
        let gsubrs_at = strings_at + strings.len();
        let charset_at = gsubrs_at + gsubrs.len();
        let fd_select_at = charset_at + charset.len();
        let charstrings_at = fd_select_at + fd_select.len();
        // The Private DICT: `Subrs` at 6 bytes from its own start, which is its own length.
        let private_at = charstrings_at + charstrings.len();
        let mut private = Vec::new();
        private.extend_from_slice(&int5(6));
        private.push(19);
        let subrs = index(&[subr]);
        let fd_array_at = private_at + private.len() + subrs.len();
        let mut top = Vec::new();
        top.extend_from_slice(&int5(391));
        top.extend_from_slice(&int5(392));
        top.extend_from_slice(&int5(0));
        top.extend_from_slice(&[12, 30]);
        top.extend_from_slice(&int5(charset_at as i64));
        top.push(15);
        top.extend_from_slice(&int5(fd_select_at as i64));
        top.extend_from_slice(&[12, 37]);
        top.extend_from_slice(&int5(charstrings_at as i64));
        top.push(17);
        top.extend_from_slice(&int5(fd_array_at as i64));
        top.extend_from_slice(&[12, 36]);
        assert_eq!(top.len(), top_len);
        let mut fd0 = Vec::new();
        fd0.extend_from_slice(&int5(private.len() as i64));
        fd0.extend_from_slice(&int5(private_at as i64));
        fd0.push(18);
        let mut fd1 = Vec::new();
        fd1.extend_from_slice(&int5(0));
        fd1.extend_from_slice(&int5(private_at as i64));
        fd1.push(18);
        let fd_array = if fd1_present {
            index(&[&fd0, &fd1])
        } else {
            index(&[&fd0])
        };

        let mut out = Vec::new();
        out.extend_from_slice(&header);
        out.extend_from_slice(&names);
        out.extend_from_slice(&index(&[&top]));
        out.extend_from_slice(&strings);
        out.extend_from_slice(&gsubrs);
        out.extend_from_slice(&charset);
        out.extend_from_slice(&fd_select);
        out.extend_from_slice(&charstrings);
        out.extend_from_slice(&private);
        out.extend_from_slice(&subrs);
        out.extend_from_slice(&fd_array);
        out
    }

    /// A Font DICT that reads is kept — its glyph still draws through its own local
    /// subroutine after the repair — and only the one a glyph selects and nobody can read is
    /// replaced; a glyph under the replacement that needs a subroutine is the only loss.
    #[test]
    fn a_readable_font_dict_is_kept_beside_the_replaced_one() {
        let whole = two_fd_cid_keyed(VIA_SUBR, SQUARE, SQUARE, true);
        assert_eq!(
            segments(&whole, 1),
            Ok(4),
            "the fixture draws through its subroutine"
        );
        assert_eq!(segments(&whole, 2), Ok(4));
        assert_eq!(readable_font_dicts(&whole), Ok(None));

        let short = two_fd_cid_keyed(VIA_SUBR, SQUARE, SQUARE, false);
        assert_eq!(segments(&short, 1), Ok(4), "FD 0 reads before the repair");
        assert!(
            segments(&short, 2).is_err(),
            "glyph 2 selects an FD the FDArray lacks"
        );
        let repair = readable_font_dicts(&short)
            .expect("opens")
            .expect("FD 1 is unreadable");
        assert_eq!((repair.font_dicts, repair.replaced), (2, 1));
        assert!(repair.lost.is_empty());
        assert_eq!(
            segments(&repair.data, 1),
            Ok(4),
            "the kept DICT's subroutine still draws"
        );
        assert_eq!(segments(&repair.data, 2), Ok(4));

        let lossy = two_fd_cid_keyed(VIA_SUBR, VIA_SUBR, SQUARE, false);
        let repair = readable_font_dicts(&lossy)
            .expect("opens")
            .expect("FD 1 is unreadable");
        assert_eq!(repair.lost.iter().copied().collect::<Vec<_>>(), vec![2]);
        assert_eq!(segments(&repair.data, 1), Ok(4));
        assert!(segments(&repair.data, 2).is_err());
    }

    #[test]
    fn a_program_whose_font_dicts_read_is_left_alone() {
        let data = cid_keyed(SQUARE, None, true);
        assert_eq!(
            segments(&data, 1),
            Ok(4),
            "the fixture draws before anything is repaired"
        );
        assert_eq!(readable_font_dicts(&data), Ok(None));
    }

    /// The witness's shape: the charstrings are whole and the `FDArray` is nowhere, so before
    /// the repair no glyph draws, and after it the square does — through the same charset,
    /// which is what §9.7.4.2's route depends on.
    #[test]
    fn an_unreadable_fd_array_is_replaced_and_the_glyphs_draw() {
        let broken = cid_keyed(SQUARE, None, false);
        assert!(
            segments(&broken, 1).is_err(),
            "the broken fixture must not draw as it is"
        );
        let repair = readable_font_dicts(&broken)
            .expect("the program opens")
            .expect("the Font DICTs are unreadable");
        assert_eq!(
            (repair.glyphs, repair.font_dicts, repair.replaced),
            (2, 1, 1)
        );
        assert!(repair.lost.is_empty());
        assert_eq!(segments(&repair.data, 1), Ok(4));
        let CodeToGlyph::Keyed { by_cid } = CodeToGlyph::read(&repair.data).expect("still a CFF")
        else {
            panic!("the repaired program is still CID-keyed")
        };
        assert_eq!(by_cid.get(&5), Some(&1));
        assert_eq!(
            readable_font_dicts(&repair.data),
            Ok(None),
            "repaired once is repaired"
        );
    }

    /// What the replacement DICT cannot hold is counted: a glyph calling a local subroutine
    /// directly, and one reaching it through a global subroutine.
    #[test]
    fn a_glyph_that_needs_a_local_subroutine_is_counted_as_lost() {
        let direct = readable_font_dicts(&cid_keyed(NEEDS_SUBR, None, false))
            .expect("opens")
            .expect("repaired");
        assert_eq!(
            (
                direct.glyphs,
                direct.lost.iter().copied().collect::<Vec<_>>()
            ),
            (2, vec![1])
        );
        assert!(
            segments(&direct.data, 1).is_err(),
            "no DICT holds the subroutine it calls"
        );

        let through = readable_font_dicts(&cid_keyed(VIA_GSUBR, Some(NEEDS_SUBR), false))
            .expect("opens")
            .expect("repaired");
        assert_eq!(
            (
                through.glyphs,
                through.lost.iter().copied().collect::<Vec<_>>()
            ),
            (2, vec![1])
        );

        let harmless = readable_font_dicts(&cid_keyed(VIA_GSUBR, Some(SQUARE), false))
            .expect("opens")
            .expect("repaired");
        assert_eq!((harmless.glyphs, harmless.lost.len()), (2, 0));
        assert_eq!(segments(&harmless.data, 1), Ok(4));
    }
}
