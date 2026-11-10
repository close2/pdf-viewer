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
//! why `crate::LoadedFont::glyph_for` refuses to guess.
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
use skrifa::raw::FontRead;
use skrifa::raw::ps::cff::CffFontRef;
use skrifa::raw::ps::cff::charset::CharsetKind;
use skrifa::raw::ps::cff::dict;
use skrifa::raw::ps::cs::CommandSink;
use skrifa::raw::ps::transform::Transform;
use skrifa::raw::tables::cff::Cff;
use skrifa::raw::types::Fixed;

use crate::name_keyed::NameKeyed;

/// How many string identifiers the CFF specification predefines, 0 to 390 — every identifier a
/// predefined encoding can name (Technical Note 5176, Appendix A, cited and not quoted).
const STANDARD_STRINGS: usize = 391;

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
        // The charset inverted once, for the predefined encodings below: `read-fonts`'
        // `Charset::glyph_id` walks a custom charset from its start on every call, which is the
        // glyph count once per code (ADR 1596). A predefined encoding names only standard
        // strings, so only their identifiers are kept, the lowest glyph for each.
        let mut by_sid = [None; STANDARD_STRINGS];
        for (glyph, sid) in charset.iter() {
            let Ok(glyph) = u16::try_from(glyph.to_u32()) else {
                continue;
            };
            if let Some(slot @ None) = by_sid.get_mut(usize::from(sid.to_u16())) {
                *slot = Some(glyph);
            }
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
            // Only a custom charset is walked; a predefined one is a static table `read-fonts`
            // answers directly, and is left to answer.
            let custom = matches!(charset.kind(), CharsetKind::Custom(_));
            let predefined = encoding.predefined().filter(|_| custom);
            for (code, slot) in builtin.iter_mut().enumerate() {
                let Ok(code) = u8::try_from(code) else {
                    continue;
                };
                *slot = match predefined {
                    Some(predefined) => predefined
                        .sid(code)
                        .and_then(|sid| by_sid.get(usize::from(sid.to_u16())).copied().flatten()),
                    None => encoding
                        .map(code)
                        .and_then(|glyph| u16::try_from(glyph.to_u32()).ok()),
                };
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
    open(data)?;
    let cff = Cff::read(data.into()).map_err(|error| malformed(&error.into()))?;
    let top = cff.top_dicts().get(0).ok_or_else(|| CffError::Malformed {
        detail: "the Top DICT INDEX is empty".to_owned(),
    })?;
    let mut upem = 1000.0;
    for entry in dict::entries(top, None) {
        if let dict::Entry::FontMatrix(matrix) = entry.map_err(|error| malformed(&error))? {
            // Technical Note 5176, Table 9: FontMatrix maps glyph space to text space.
            // The outline reader normalizes its Y scale and rounds the reciprocal to an
            // integer UPEM. Read the original matrix so the caller's normalization keeps
            // the fractional scale for both outlines and charstring advances (ADR 2180).
            let elements = matrix.matrix.elements();
            let factor = if elements[3].to_bits() == 0 {
                elements[1].to_f64().abs()
            } else {
                elements[3].to_f64().abs()
            };
            if factor == 0.0 || matrix.scale <= 0 {
                return Err(CffError::Malformed {
                    detail: "FontMatrix has no positive normalization scale".to_owned(),
                });
            }
            upem = f64::from(matrix.scale) / factor;
        }
    }
    Ok(crate::metrics::narrow(upem))
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
/// The width is the charstring's own number, fractions kept: a Type 2 charstring may state a
/// width as a 16.16 fixed operand or reach it through `div`, and ISO 19005-2 6.2.11.5 compares
/// the program's width with the document's, so a reader that rounded 776.89 to 777 would call a
/// disagreement agreement. The outline reader's [`CffFontRef::draw`] rounds the width to whole
/// units before it applies the matrix, which suits a rasteriser at a pixel size and is not what
/// the program states; this evaluates the charstring and applies the same normalised matrix and
/// scale itself, without that rounding (ADR 2540).
///
/// # Errors
///
/// See [`CffError`]. A glyph the program cannot evaluate yields `None` rather than an error,
/// because one unreadable charstring is not a reason to lose the other 255 widths.
pub fn advances(data: &[u8], glyphs: &[u16]) -> Result<Vec<Option<f32>>, CffError> {
    evaluated_widths(data, glyphs, horizontal_metric)
}

/// The width each glyph's charstring states, in the program's own units and before its
/// `FontMatrix`: the charstring's width operand added to its Private DICT's `nominalWidthX`, or
/// the `defaultWidthX` where the charstring states none (Adobe Technical Note #5177, section 3.1).
///
/// This is the number [`with_advances`] is asked to write, read the way the outline reader reads
/// it, so that a restated program can be held to stating what it was asked to — the `cff_write`
/// fuzz target's question of whether the writer and the reader read one program alike.
///
/// # Errors
///
/// See [`CffError`]; a glyph the program cannot evaluate yields `None`, as in [`advances`].
pub fn charstring_widths(data: &[u8], glyphs: &[u16]) -> Result<Vec<Option<f32>>, CffError> {
    evaluated_widths(data, glyphs, |_, width| width.to_f32())
}

/// Each glyph's charstring width, carried through `metric` with the program's transform.
fn evaluated_widths(
    data: &[u8],
    glyphs: &[u16],
    metric: impl Fn(&Transform, Fixed) -> f32,
) -> Result<Vec<Option<f32>>, CffError> {
    /// Discards the outline: only the charstring's width operand is wanted here.
    struct NoCommands;
    impl CommandSink for NoCommands {
        fn move_to(&mut self, _x: Fixed, _y: Fixed) {}
        fn line_to(&mut self, _x: Fixed, _y: Fixed) {}
        fn curve_to(
            &mut self,
            _cx0: Fixed,
            _cy0: Fixed,
            _cx1: Fixed,
            _cy1: Fixed,
            _x: Fixed,
            _y: Fixed,
        ) {
        }
        fn close(&mut self) {}
    }

    let font = open(data)?;
    Ok(glyphs
        .iter()
        .map(|glyph| {
            let id = GlyphId::from(*glyph);
            let index = font.subfont_index(id)?;
            let subfont = font.subfont(index, &[]).ok()?;
            let width = font
                .evaluate_charstring(&subfont, id, &[], &mut NoCommands)
                .ok()??;
            Some(metric(&font.transform(&subfont, None), width))
        })
        .collect())
}

/// A horizontal metric in font units carried through a program's normalised `FontMatrix`.
///
/// The steps are those of the outline reader's own metric transform — the matrix's `xx`, then
/// its `dx`, then the scale that restates the matrix's units per em in the reader's (Technical
/// Note 5176, Table 9; ADR 2180) — with the metric kept as the fixed-point number the charstring
/// produced. The scale is a 26.6 factor, which is why it is divided by 64. A width below zero is
/// read as zero, as the outline reader reads it.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "Fixed's product is formed in i64, which two i32 operands cannot overflow"
)]
fn horizontal_metric(transform: &Transform, metric: Fixed) -> f32 {
    let mut metric = metric;
    if transform.matrix.xx != Fixed::ONE {
        metric *= transform.matrix.xx;
    }
    metric = metric.saturating_add(transform.matrix.dx);
    if let Some(scale) = transform.scale {
        metric = metric.mul_div(scale, Fixed::from_i32(64));
    }
    metric.to_f32().max(0.0)
}

/// Whether a CFF program's Top DICT uses `CIDFont` operators.
///
/// §9.9's Table 124 turns this into which `/FontFile3` `/Subtype` a program may be written
/// under, and §9.7.4.2 into how a CID reaches a glyph inside it — the charset for a program that
/// uses them, and the CID as the glyph index directly for one that does not. It is a fact about
/// the program's own bytes, which is why a converter asks it here rather than trusting the
/// `/Subtype` a producer wrote.
///
/// # Errors
///
/// See [`CffError`]: a program that does not open is not classified.
pub fn uses_cid_operators(data: &[u8]) -> Result<bool, CffError> {
    Ok(open(data)?.is_cid())
}

/// How many glyphs a CFF program holds, which is the length of its `CharStrings` INDEX.
///
/// # Errors
///
/// See [`CffError`].
pub fn glyph_count(data: &[u8]) -> Result<u32, CffError> {
    Ok(open(data)?.num_glyphs())
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

#[cfg(test)]
#[path = "cff_scale_tests.rs"]
mod scale_tests;

#[cfg(test)]
#[path = "cff_budget_tests.rs"]
mod budget_tests;

#[cfg(test)]
#[path = "cff_operand_tests.rs"]
mod operand_tests;

#[cfg(test)]
#[path = "cff_stop_tests.rs"]
mod stop_tests;

#[path = "cff_cids_given.rs"]
mod cids_given;

#[path = "cff_stretch.rs"]
mod stretched;
pub use cids_given::{CidToGive, MOST_CID_BYTES_GIVEN, highest_cid, with_cids_given};
pub use stretched::with_outlines_scaled;

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
    // The Top DICT is restated whole and each kept Font DICT's `Private` offset with it, so
    // those are the entries this rewrite follows; no Private DICT is read, only copied.
    if let Some(misstated) = misstated_in(&layout.entries, "Top", top_dict_entry) {
        return Err(misstated.refusal());
    }
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
                    .is_some_and(|charstring| {
                        calls_local_subr(charstring, &font, 0, &mut { CHARSTRING_OPERATIONS })
                    })
        })
        .collect();
    if let Some(misstated) = font_dicts
        .iter()
        .flatten()
        .find_map(|dict| misstated_in(&dict_entries(dict)?, "Font", top_dict_entry))
    {
        return Err(misstated.refusal());
    }
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
    /// The first entry a writer here follows that the program states otherwise than Adobe
    /// Technical Note #5176 gives it, in the Top DICT, a Font DICT or a Private DICT; see
    /// [`misstated_entry`].
    misstated: Option<Misstated>,
}

/// The two-byte escape's operators, as `escape + second byte`.
pub(crate) const ESCAPE: u16 = 0x0c00;
pub(crate) const OP_CHARSET: u16 = 15;
const OP_ENCODING: u16 = 16;
pub(crate) const OP_CHARSTRINGS: u16 = 17;
pub(crate) const OP_PRIVATE: u16 = 18;
pub(crate) const OP_FD_ARRAY: u16 = ESCAPE | 0x24;
pub(crate) const OP_FD_SELECT: u16 = ESCAPE | 0x25;

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
    /// Where the charset now is, where the rebuild wrote a fresh one.
    charset_at: Option<i64>,
    /// Where the `FDSelect` now is, where the rebuild wrote a fresh one.
    fd_select_at: Option<i64>,
}

impl Moved {
    /// Everything after the Top DICT INDEX shifted by `delta`, nothing relocated.
    const fn by(delta: i64) -> Self {
        Self {
            delta,
            fd_array_at: None,
            charstrings_at: None,
            charset_at: None,
            fd_select_at: None,
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

    /// The same, with a fresh charset at `at`.
    const fn charset_to(mut self, at: i64) -> Self {
        self.charset_at = Some(at);
        self
    }

    /// The same, with a fresh `FDSelect` at `at`.
    const fn fd_select_to(mut self, at: i64) -> Self {
        self.fd_select_at = Some(at);
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
        let misstated = misstated_entry(data, &entries)
            .or_else(|| offset_outside(data, &entries, top_index.end, outside_copied));
        Some(Self {
            top_index,
            entries,
            fd_array_at,
            misstated,
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
    ///
    /// **A CID-keyed program's `FDArray` is written afresh as well**, after the `CharStrings`, each
    /// Font DICT's offsets shifted as [`Self::with_font_dicts`] shifts them: a `Private` offset is
    /// absolute (Adobe Technical Note #5176, Table 9 and section 18), so an `FDArray` copied verbatim would
    /// name, for every Font DICT, bytes `delta` away from its Private DICT — its `nominalWidthX`,
    /// against which every restated width is stated, and its local subroutines.
    fn with_charstrings(&self, data: &[u8], charstrings: &[Vec<u8>]) -> Vec<u8> {
        let placeholder = self.top_dict(Moved::by(0).charstrings_to(0).fd_array_to(0));
        let new_index_len = 3usize
            .saturating_add(2 * 4)
            .saturating_add(placeholder.len());
        let old_index_len = self.top_index.end.saturating_sub(self.top_index.start);
        let delta = i64::try_from(new_index_len)
            .unwrap_or(0)
            .saturating_sub(i64::try_from(old_index_len).unwrap_or(0));
        let charstrings_at = i64::try_from(data.len()).unwrap_or(0).saturating_add(delta);
        let mut charstrings_index = Vec::new();
        push_index(&mut charstrings_index, charstrings);
        let font_dicts: Option<Vec<Vec<u8>>> = self.fd_array_at.map(|at| {
            index_items(data, at)
                .unwrap_or_default()
                .iter()
                .map(|dict| font_dict_shifted(dict, delta))
                .collect()
        });
        let mut moved = Moved::by(delta).charstrings_to(charstrings_at);
        if font_dicts.is_some() {
            moved =
                moved
                    .fd_array_to(charstrings_at.saturating_add(
                        i64::try_from(charstrings_index.len()).unwrap_or(i64::MAX),
                    ));
        }
        let top_dict = self.top_dict(moved);

        let mut out = Vec::with_capacity(data.len().saturating_mul(2));
        out.extend_from_slice(data.get(..self.top_index.start).unwrap_or(data));
        push_index(&mut out, std::slice::from_ref(&top_dict));
        out.extend_from_slice(data.get(self.top_index.end..).unwrap_or(&[]));
        out.extend_from_slice(&charstrings_index);
        if let Some(font_dicts) = font_dicts {
            push_index(&mut out, &font_dicts);
        }
        out
    }

    /// The program with a String INDEX, a `CharStrings` INDEX and a charset of its own.
    ///
    /// [`Self::with_charstrings`]'s move with one more table in it: the String INDEX sits between
    /// the Top DICT INDEX and everything the Top DICT's offsets locate (Adobe Technical Note #5176,
    /// section 2), so both are re-encoded, everything after the String INDEX is copied verbatim
    /// and shifted by the two differences together, and the fresh `CharStrings` INDEX and charset
    /// are appended past the end. A name-keyed program's only other absolute offsets — its
    /// `Encoding` and its `Private` DICT — move with the copied bytes, and a Private DICT's
    /// `Subrs` is relative to the Private DICT itself.
    ///
    /// **A CID-keyed program's `fd_select` is appended after the charset, and its `FDArray`
    /// rewritten after that** — a name-keyed program's too where it states one that reads — each
    /// Font DICT's offsets shifted as [`Self::with_charstrings`] shifts them, since they are
    /// absolute and the bytes they name moved with everything copied.
    ///
    /// Refused where the String INDEX or the `FDArray` does not read, and as
    /// [`Misstated::InStrings`] where an offset this shifts points into the String INDEX, whose
    /// bytes are re-encoded rather than copied.
    fn with_glyph_tables(
        &self,
        data: &[u8],
        strings: &[Vec<u8>],
        tables: (&[Vec<u8>], &[u8]),
        fd_select: Option<&[u8]>,
    ) -> Result<Vec<u8>, CffError> {
        let malformed = |detail: &str| CffError::Malformed {
            detail: detail.to_owned(),
        };
        let old_strings = index_extent(data, self.top_index.end)
            .ok_or_else(|| malformed("the String INDEX could not be located"))?;
        if let Some(misstated) =
            offset_outside(data, &self.entries, old_strings.end, inside_strings)
        {
            return Err(misstated.refusal());
        }
        self.glyph_tables_written(data, strings, old_strings.end, tables, fd_select)
            .ok_or_else(|| malformed("the FDArray could not be read, or an offset overflows"))
    }

    /// [`Self::with_glyph_tables`]'s rebuild, once its String INDEX is located at
    /// `..strings_end` and its offsets are known to point past it.
    fn glyph_tables_written(
        &self,
        data: &[u8],
        strings: &[Vec<u8>],
        strings_end: usize,
        (charstrings, charset): (&[Vec<u8>], &[u8]),
        fd_select: Option<&[u8]>,
    ) -> Option<Vec<u8>> {
        let placeholder = self.top_dict(Moved::by(0).charstrings_to(0).charset_to(0));
        let new_index_len = 3usize
            .saturating_add(2 * 4)
            .saturating_add(placeholder.len());
        let mut strings_index = Vec::new();
        push_index(&mut strings_index, strings);
        let grown = i64::try_from(new_index_len.saturating_add(strings_index.len())).ok()?;
        let old = i64::try_from(strings_end.saturating_sub(self.top_index.start)).ok()?;
        let delta = grown.saturating_sub(old);
        let charstrings_at = i64::try_from(data.len()).ok()?.saturating_add(delta);
        let mut charstrings_index = Vec::new();
        push_index(&mut charstrings_index, charstrings);
        let charset_at =
            charstrings_at.saturating_add(i64::try_from(charstrings_index.len()).ok()?);
        let mut moved = Moved::by(delta)
            .charstrings_to(charstrings_at)
            .charset_to(charset_at);
        let mut cid_tables = Vec::new();
        let fd_select_at = charset_at.saturating_add(i64::try_from(charset.len()).ok()?);
        if let Some(fd_select) = fd_select {
            moved = moved.fd_select_to(fd_select_at);
            cid_tables.extend_from_slice(fd_select);
        }
        // A CID-keyed program's Font DICTs are rewritten whatever else it states; a name-keyed
        // one's are where its `FDArray` reads, since the offsets they state are shifted only in a
        // rewritten copy. A name-keyed `FDArray` that does not read is copied and shifted whole.
        let font_dicts = match (self.fd_array_at, fd_select) {
            (None, Some(_)) => return None,
            (None, None) => None,
            (Some(at), Some(_)) => Some(index_items(data, at)?),
            (Some(at), None) => index_items(data, at),
        };
        if let Some(font_dicts) = font_dicts {
            let font_dicts: Vec<Vec<u8>> = font_dicts
                .iter()
                .map(|dict| font_dict_shifted(dict, delta))
                .collect();
            moved = moved
                .fd_array_to(fd_select_at.saturating_add(i64::try_from(cid_tables.len()).ok()?));
            push_index(&mut cid_tables, &font_dicts);
        }
        let top_dict = self.top_dict(moved);
        let mut out = Vec::with_capacity(data.len().saturating_mul(2));
        out.extend_from_slice(data.get(..self.top_index.start)?);
        push_index(&mut out, std::slice::from_ref(&top_dict));
        out.extend_from_slice(&strings_index);
        out.extend_from_slice(data.get(strings_end..)?);
        out.extend_from_slice(&charstrings_index);
        out.extend_from_slice(charset);
        out.extend_from_slice(&cid_tables);
        Some(out)
    }

    /// The Top DICT's bytes, every offset it states restated under `moved`.
    fn top_dict(&self, moved: Moved) -> Vec<u8> {
        dict_restated(&self.entries, moved)
    }
}

/// A Top DICT's or a Font DICT's `entries` re-encoded, every offset they state restated under
/// `moved` and every other operand verbatim.
///
/// A Font DICT is read with the Top DICT's operators: Adobe Technical Note #5176 gives a Font DICT
/// no operator table of its own, and section 18 asks of it only that it specify its Private DICT.
/// So an offset a Font DICT states — a `charset`, an `Encoding`, a `CharStrings`, an `FDSelect` or
/// an `FDArray` as well as its `Private` — names the bytes the Top DICT's same operator would, and
/// is shifted with them; left as it stood, it names bytes `delta` away once the String INDEX or the
/// Top DICT INDEX in front of them has grown.
fn dict_restated(entries: &[(Vec<u8>, u16)], moved: Moved) -> Vec<u8> {
    let delta = moved.delta;
    let mut out = Vec::new();
    for (operands, op) in entries {
        match *op {
            // A charset or Encoding operand below the first custom offset names a
            // predefined table (ISOAdobe/Expert/ExpertSubset, standard/expert) and is
            // not an offset.
            OP_CHARSET if moved.charset_at.is_some() => {
                out.extend_from_slice(&int5(moved.charset_at.unwrap_or(0)));
            }
            OP_CHARSET | OP_ENCODING => {
                let at = dict_int(operands).unwrap_or(0);
                let custom = if *op == OP_CHARSET { at > 2 } else { at > 1 };
                out.extend_from_slice(&int5(if custom { at.saturating_add(delta) } else { at }));
            }
            OP_CHARSTRINGS => out.extend_from_slice(&int5(
                moved
                    .charstrings_at
                    .unwrap_or_else(|| dict_int(operands).unwrap_or(0).saturating_add(delta)),
            )),
            OP_FD_SELECT => {
                out.extend_from_slice(&int5(
                    moved
                        .fd_select_at
                        .unwrap_or_else(|| dict_int(operands).unwrap_or(0).saturating_add(delta)),
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
    // A Top DICT that states no charset means the predefined ISOAdobe one (Adobe Technical
    // Note #5176, section 13), so a fresh charset is reached only by an entry added for it.
    if let Some(at) = moved.charset_at
        && !entries.iter().any(|(_, op)| *op == OP_CHARSET)
    {
        out.extend_from_slice(&int5(at));
        push_operator(&mut out, OP_CHARSET);
    }
    out
}

/// A Font DICT re-encoded with every offset it states shifted by `delta`, as [`dict_restated`]
/// shifts them; every other entry verbatim. A DICT that does not parse is kept as it is.
fn font_dict_shifted(dict: &[u8], delta: i64) -> Vec<u8> {
    dict_entries(dict).map_or_else(
        || dict.to_vec(),
        |entries| dict_restated(&entries, Moved::by(delta)),
    )
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
pub(crate) fn push_operator(out: &mut Vec<u8>, op: u16) {
    if op & ESCAPE == ESCAPE {
        out.push(12);
    }
    out.push(u8::try_from(op & 0xff).unwrap_or(0));
}

/// A DICT integer in the five-byte form, which holds any offset a CFF can state.
pub(crate) fn int5(value: i64) -> [u8; 5] {
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
            30 => real_width(dict.get(i.checked_add(1)?..)?)?.checked_add(1)?,
            32..=246 => 1,
            247..=254 => 2,
            22..=27 | 31 | 255 => return None,
        };
        operands.extend_from_slice(dict.get(i..i.checked_add(width)?)?);
        i = i.checked_add(width)?;
    }
    Some(entries)
}

/// How many bytes a real number's nibbles take after its first byte, 30, read as `read-fonts`
/// reads them; `None` where the bytes end first.
///
/// Adobe Technical Note #5176, section 4, writes a real as nibbles — digits, a decimal point, an
/// exponent and a minus sign — ended by a 0xf nibble, and reserves the nibble 0xd; it says nothing of a nibble standing where its grammar admits none.
/// `read-fonts` ends the number with the byte holding the first such nibble, and with the byte
/// in which an exponent past 1000 meets another digit, and reads the next byte as the next
/// operand. A writer restates what that reader will read, so a DICT is tokenised here at the
/// same byte: ending it only at a 0xf nibble took the operands after a reserved one into the
/// number, and the writer reckoned against an entry the reader never saw (ADR 2610).
fn real_width(nibbles: &[u8]) -> Option<usize> {
    #[derive(Clone, Copy)]
    enum Part {
        Integer,
        Fraction,
        Exponent,
    }
    let mut part = Part::Integer;
    let mut exponent = 0u32;
    for (at, &byte) in nibbles.iter().enumerate() {
        for nibble in [byte >> 4, byte & 0x0f] {
            part = match (part, nibble) {
                (Part::Exponent, 0..=9) if exponent > 1000 => return at.checked_add(1),
                (Part::Exponent, 0..=9) => {
                    exponent = exponent
                        .saturating_mul(10)
                        .saturating_add(u32::from(nibble));
                    Part::Exponent
                }
                (Part::Integer, 0..=9 | 0x0e) => Part::Integer,
                (Part::Integer, 0x0a) | (Part::Fraction, 0..=9) => Part::Fraction,
                (Part::Integer | Part::Fraction, 0x0b | 0x0c) => Part::Exponent,
                _ => return at.checked_add(1),
            };
        }
    }
    None
}

/// The value a one- or two-byte operand encodes and its width, for a first byte in
/// `32..=254` — the forms a DICT and a Type 2 charstring share (Adobe Technical Note #5176,
/// Table 3; #5177, section 3.2). `b1` is read only for the two-byte forms.
pub(crate) fn small_operand(b0: u8, b1: u8) -> (i64, usize) {
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
pub(crate) fn dict_ints(operands: &[u8]) -> Vec<i64> {
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

/// How many operands a DICT entry states, integers and reals alike: the forms are told apart
/// by their first byte (Adobe Technical Note #5176, section 4 and Table 3).
///
/// Every operand is at least one byte wide, so the count is bounded by the bytes.
fn operand_count(operands: &[u8]) -> usize {
    let mut count = 0usize;
    let mut i = 0usize;
    while let Some(&b0) = operands.get(i) {
        let width = match b0 {
            28 => 3,
            29 => 5,
            30 => operands
                .get(i.saturating_add(1)..)
                .and_then(real_width)
                .map_or(operands.len(), |width| width.saturating_add(1)),
            247..=254 => 2,
            _ => 1,
        };
        count = count.saturating_add(1);
        i = i.saturating_add(width);
    }
    count
}

/// The integers an entry states, where it states exactly `N` operands and each is an integer.
///
/// **The operand rule every reader and writer of DICT offsets here shares** (ADR 2610).
/// Technical Note #5176's section 4 has an operator preceded by the operand(s) that specify its
/// value, and lets only an array's or a delta's length be told by counting them; Tables 9, 10
/// and 23 give each offset this crate follows one number, and `Private` two. An entry stated
/// with more, with fewer or with a real has no value the note defines, so it reads here as no
/// value — never as its first operands, and never as its last. [`misstated_entry`] finds such an
/// entry before any of this is read, so that a writer refuses the program by name rather than
/// taking the entry as absent.
fn exactly<const N: usize>(operands: &[u8]) -> Option<[i64; N]> {
    if operand_count(operands) != N {
        return None;
    }
    <[i64; N]>::try_from(dict_ints(operands)).ok()
}

/// The one integer an entry states, under [`exactly`]'s rule.
fn dict_int(operands: &[u8]) -> Option<i64> {
    exactly::<1>(operands).map(|[value]| value)
}

/// The two integers an entry states — `Private`'s size and offset — under [`exactly`]'s rule.
fn dict_two_ints(operands: &[u8]) -> Option<(i64, i64)> {
    exactly::<2>(operands).map(|[size, at]| (size, at))
}

/// How Technical Note #5176 states an entry the writers here follow: its operator's name, how
/// many operands it takes, and whether they are integers (an offset, a size) or any number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stated {
    operator: &'static str,
    operands: usize,
    integers: bool,
}

/// The Top DICT entries a writer here follows or restates — Table 9's `charset`, `Encoding`,
/// `CharStrings` and `Private`, Table 10's `FDArray` and `FDSelect` — and, by the same table, a
/// Font DICT's (section 19), of which only `Private` is followed.
const fn top_dict_entry(op: u16) -> Option<Stated> {
    let (operator, operands) = match op {
        OP_CHARSET => ("charset", 1),
        OP_ENCODING => ("Encoding", 1),
        OP_CHARSTRINGS => ("CharStrings", 1),
        OP_PRIVATE => ("Private", 2),
        OP_FD_ARRAY => ("FDArray", 1),
        OP_FD_SELECT => ("FDSelect", 1),
        _ => return None,
    };
    Some(Stated {
        operator,
        operands,
        integers: true,
    })
}

/// The Private DICT entries a writer here follows (Table 23): `Subrs`, an offset, and the two
/// widths a restated charstring's width operand is reckoned against, each a number.
const fn private_dict_entry(op: u16) -> Option<Stated> {
    match op {
        OP_SUBRS => Some(Stated {
            operator: "Subrs",
            operands: 1,
            integers: true,
        }),
        OP_DEFAULT_WIDTH => Some(Stated {
            operator: "defaultWidthX",
            operands: 1,
            integers: false,
        }),
        OP_NOMINAL_WIDTH => Some(Stated {
            operator: "nominalWidthX",
            operands: 1,
            integers: false,
        }),
        _ => None,
    }
}

/// `defaultWidthX` and `nominalWidthX` (Adobe Technical Note #5176, Table 23).
const OP_DEFAULT_WIDTH: u16 = 20;
const OP_NOMINAL_WIDTH: u16 = 21;

/// An entry a writer here follows, stated otherwise than Adobe Technical Note #5176 gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Misstated {
    /// An entry stated twice in one DICT, or with other operands than the note's table gives it.
    Entry {
        /// `Top`, `Font` or `Private`.
        dict: &'static str,
        /// The operator's name in the note's table.
        operator: &'static str,
        /// How many operands the table gives it.
        operands: usize,
    },
    /// A Font DICT or a Private DICT the program names whose bytes do not read as operands and
    /// operators: a byte section 4 reserves ("Byte values 22–27, 31, and 255 are reserved"),
    /// or an operand the DICT's end cuts short. What such a DICT states is then not something
    /// a writer here can tell, while the outline reader passes over the reserved byte and reads
    /// on, so a width reckoned against it would be reckoned against another number.
    Untokenised {
        /// `Font` or `Private`.
        dict: &'static str,
    },
    /// An offset a writer here shifts that points below zero, into the header, the Name INDEX or
    /// the Top DICT INDEX, which section 2 places first ("the first five occupy fixed locations"), or
    /// past the program's end: the structure it names would be the Top DICT's own bytes or none,
    /// and a rewrite that re-encodes the Top DICT, shifts every offset past it and appends what
    /// it rebuilt would name other bytes.
    Outside {
        /// `Top` or `Font`.
        dict: &'static str,
        /// The operator's name in the note's table.
        operator: &'static str,
    },
    /// An offset a writer that restates the String INDEX shifts that points into that INDEX.
    /// The String INDEX is re-encoded, not copied, so a byte inside it moves by less than the
    /// growth every offset past it is shifted by, and the shifted offset would name other bytes.
    InStrings {
        /// `Top` or `Font`.
        dict: &'static str,
        /// The operator's name in the note's table.
        operator: &'static str,
    },
    /// A `Private` entry whose size is negative. Table 9 gives the operator a size and an
    /// offset, and a size counts the DICT's bytes; the outline reader drops a negative one where
    /// its end overflows and refuses the font where it does not, so whether the program reads at
    /// all turns on where the offset sits, which every writer here moves.
    NegativeSize {
        /// `Top` or `Font`.
        dict: &'static str,
    },
    /// An `FDArray` the Top DICT names that does not read as section 5 gives an INDEX.
    FdArrayUnread,
    /// A `Subrs` a Private DICT states whose Local Subr INDEX does not read whole inside the
    /// program: what the calls of a charstring reach would then turn on the bytes after the
    /// program's end, which a rewrite's output may hold where its input did not (ADR 2628).
    SubrsUnread,
    /// Private DICTs that together hold more bytes than the program, which only overlapping
    /// ones can: each is a structure of its own (section 15), and reading every one is then
    /// work no well-formed program asks for.
    Overlapping,
}

impl Misstated {
    /// The refusal every writer gives for it, naming the entry.
    pub(crate) fn refusal(self) -> CffError {
        let detail = match self {
            Self::Entry {
                dict,
                operator,
                operands,
            } => format!(
                "the {dict} DICT states {operator} more than once, or with other than the \
                 {operands} operand{} Adobe Technical Note #5176 gives it",
                if operands == 1 { "" } else { "s" }
            ),
            Self::Untokenised { dict } => format!(
                "a {dict} DICT the program names holds a byte Adobe Technical Note #5176 \
                 section 4 reserves, or an operand its end cuts short"
            ),
            Self::Outside { dict, operator } => format!(
                "the {dict} DICT's {operator} offset points before the program's start, into \
                 the header, the Name INDEX or the Top DICT INDEX, which Adobe Technical Note \
                 #5176 section 2 places first, or past the program's end, or names a Private DICT \
                 ending past it"
            ),
            Self::InStrings { dict, operator } => format!(
                "the {dict} DICT's {operator} offset points into the String INDEX, which a \
                 writer that adds strings re-encodes rather than shifts"
            ),
            Self::NegativeSize { dict } => format!(
                "the {dict} DICT's Private entry states a negative size, where Adobe Technical \
                 Note #5176 Table 9 gives it the DICT's length in bytes"
            ),
            Self::FdArrayUnread => "the FDArray the Top DICT names does not read as the INDEX \
                                    Adobe Technical Note #5176 section 5 gives"
                .to_owned(),
            Self::SubrsUnread => "a Private DICT's Subrs names a Local Subr INDEX that does not \
                                  read inside the program as Adobe Technical Note #5176 section \
                                  5 gives an INDEX"
                .to_owned(),
            Self::Overlapping => "the Private DICTs the program names overlap, holding more \
                                  bytes together than the program does"
                .to_owned(),
        };
        CffError::Malformed { detail }
    }
}

/// The first of `entries` that `rule` gives a statement and the DICT states otherwise: a second
/// time, or with another count or kind of operand.
fn misstated_in(
    entries: &[(Vec<u8>, u16)],
    dict: &'static str,
    rule: fn(u16) -> Option<Stated>,
) -> Option<Misstated> {
    let mut seen = std::collections::BTreeSet::new();
    entries.iter().find_map(|(operands, op)| {
        let stated = rule(*op)?;
        let count = operand_count(operands);
        let well = seen.insert(*op)
            && count == stated.operands
            && (!stated.integers || dict_ints(operands).len() == count);
        (!well).then_some(Misstated::Entry {
            dict,
            operator: stated.operator,
            operands: stated.operands,
        })
    })
}

/// The first entry a writer here follows that the program states otherwise than Technical
/// Note #5176 gives it: in the Top DICT, in a Font DICT of its `FDArray`, or in a Private DICT
/// either names.
///
/// A structure that is not there is not this function's finding — each writer reports that
/// itself — so an `FDArray` or a Private DICT the program does not hold is passed over. One
/// that is there and does not tokenise is [`Misstated::Untokenised`]: its entries are what a
/// writer follows, and the outline reader reads past what this reader stops at. The Private
/// DICTs are read once each however many Font DICTs share one, and refused as
/// [`Misstated::Overlapping`] before any is read where together they would hold more bytes
/// than the program, which bounds the reading by the program's length.
fn misstated_entry(data: &[u8], top: &[(Vec<u8>, u16)]) -> Option<Misstated> {
    if let Some(found) = misstated_in(top, "Top", top_dict_entry) {
        return Some(found);
    }
    let private_of = |entries: &[(Vec<u8>, u16)]| -> Option<(usize, usize)> {
        let (size, at) = entries
            .iter()
            .find(|(_, op)| *op == OP_PRIVATE)
            .and_then(|(operands, _)| dict_two_ints(operands))?;
        let (at, size) = (usize::try_from(at).ok()?, usize::try_from(size).ok()?);
        data.get(at..at.checked_add(size)?).map(|_| (at, size))
    };
    let mut privates: std::collections::BTreeSet<(usize, usize)> =
        private_of(top).into_iter().collect();
    let fd_array_at = top
        .iter()
        .find(|(_, op)| *op == OP_FD_ARRAY)
        .and_then(|(operands, _)| dict_int(operands));
    let font_dicts = match fd_array_at {
        None => Vec::new(),
        // A rewrite restates the `FDArray` from its items, so one that does not read would be
        // written as an INDEX of none, which names another subfont than the reader found.
        Some(at) => match usize::try_from(at)
            .ok()
            .and_then(|at| index_items(data, at))
        {
            Some(items) => items,
            None => return Some(Misstated::FdArrayUnread),
        },
    };
    for dict in font_dicts {
        let Some(entries) = dict_entries(dict) else {
            return Some(Misstated::Untokenised { dict: "Font" });
        };
        if let Some(found) = misstated_in(&entries, "Font", top_dict_entry) {
            return Some(found);
        }
        privates.extend(private_of(&entries));
    }
    let held = privates
        .iter()
        .fold(0usize, |sum, (_, size)| sum.saturating_add(*size));
    if held > data.len() {
        return Some(Misstated::Overlapping);
    }
    privates.iter().find_map(|(at, size)| {
        // `private_of` kept only a range the program holds, so the slice is there.
        let Some(entries) = dict_entries(data.get(*at..at.saturating_add(*size))?) else {
            return Some(Misstated::Untokenised { dict: "Private" });
        };
        misstated_in(&entries, "Private", private_dict_entry)
            .or_else(|| subrs_unread(data, *at, &entries))
    })
}

/// [`Misstated::SubrsUnread`] where the Private DICT at `at`, whose entries are `entries`, states
/// a `Subrs` (an offset from the DICT's own start, Adobe Technical Note #5176 Table 23) whose
/// INDEX the program does not hold whole.
fn subrs_unread(data: &[u8], at: usize, entries: &[(Vec<u8>, u16)]) -> Option<Misstated> {
    let relative = entries
        .iter()
        .find(|(_, op)| *op == OP_SUBRS)
        .and_then(|(operands, _)| dict_int(operands))?;
    let held = usize::try_from(relative)
        .ok()
        .and_then(|relative| at.checked_add(relative))
        .and_then(|index| index_items(data, index))
        .is_some();
    (!held).then_some(Misstated::SubrsUnread)
}

/// [`Misstated::Outside`], as [`offset_outside`] names it for every writer.
const fn outside_copied(dict: &'static str, operator: &'static str) -> Misstated {
    Misstated::Outside { dict, operator }
}

/// [`Misstated::InStrings`], as [`offset_outside`] names it for a writer that adds strings.
const fn inside_strings(dict: &'static str, operator: &'static str) -> Misstated {
    Misstated::InStrings { dict, operator }
}

/// The first offset a rewrite shifts that names a structure inside the first `copied_from`
/// bytes — the header, the Name INDEX and the Top DICT INDEX, which every writer here copies or
/// re-encodes rather than shifts — or at or past the program's end, where every writer here
/// appends what it rebuilt, or a `Private` ending past it, in the Top DICT or a Font DICT of its
/// `FDArray`.
///
/// A `charset` of 0 to 2 and an `Encoding` of 0 or 1 name a predefined table rather than an
/// offset (Adobe Technical Note #5176, sections 12 and 13), and a `Private` of no bytes names
/// nothing, so none of them is an offset into anything; a `Private` of fewer than none is
/// [`Misstated::NegativeSize`] wherever its offset points.
///
/// `outside` names the finding for the writer asking: [`Misstated::Outside`] for the bytes every
/// writer re-encodes, [`Misstated::InStrings`] for the String INDEX past them, which only a
/// writer that adds strings re-encodes as well.
fn offset_outside(
    data: &[u8],
    top: &[(Vec<u8>, u16)],
    copied_from: usize,
    outside: fn(&'static str, &'static str) -> Misstated,
) -> Option<Misstated> {
    let inside =
        |value: i64| usize::try_from(value).map_or(true, |at| at < copied_from || at >= data.len());
    let named = |entries: &[(Vec<u8>, u16)], dict: &'static str| {
        entries.iter().find_map(|(operands, op)| {
            let (operator, at, predefined) = match *op {
                OP_CHARSET => ("charset", dict_int(operands)?, 2),
                OP_ENCODING => ("Encoding", dict_int(operands)?, 1),
                OP_CHARSTRINGS => ("CharStrings", dict_int(operands)?, -1),
                OP_FD_ARRAY => ("FDArray", dict_int(operands)?, -1),
                OP_FD_SELECT => ("FDSelect", dict_int(operands)?, -1),
                OP_PRIVATE => match dict_two_ints(operands)? {
                    (0, _) => return None,
                    (size, _) if size < 0 => return Some(Misstated::NegativeSize { dict }),
                    // A Private DICT that ends past the program states bytes the program does
                    // not hold, and whether a rewrite's output holds them turns on how long the
                    // output is (ADR 2628).
                    (size, at)
                        if at >= 0
                            && usize::try_from(at.saturating_add(size))
                                .map_or(true, |end| end > data.len()) =>
                    {
                        return Some(outside(dict, "Private"));
                    }
                    (_, at) => ("Private", at, -1),
                },
                _ => return None,
            };
            // An offset that names no predefined table is one wherever it points, below zero
            // included: the outline reader drops a negative `Private` where its end overflows,
            // and a shift that brings it past zero hands the reader bytes it never read.
            ((predefined < 0 || at > predefined) && inside(at)).then(|| outside(dict, operator))
        })
    };
    named(top, "Top").or_else(|| {
        top.iter()
            .find(|(_, op)| *op == OP_FD_ARRAY)
            .and_then(|(operands, _)| dict_int(operands))
            .and_then(|at| usize::try_from(at).ok())
            .and_then(|at| index_items(data, at))
            .unwrap_or_default()
            .into_iter()
            .find_map(|dict| named(&dict_entries(dict)?, "Font"))
    })
}

/// Whether a Type 2 charstring reaches `callsubr`, itself or through the global subroutines it
/// calls (Adobe Technical Note #5177, section 4.7).
///
/// The operand stack is followed far enough to know which global subroutine a `callgsubr`
/// names; a call whose operand cannot be told is counted as reaching one, which errs on the
/// side of reporting. The stem count is kept for `hintmask`'s data bytes, which would
/// otherwise be read as operators, and both are carried into and out of every subroutine called
/// ([`LocalSubrWalk`]); an `endchar` ends the walk wherever it is met. `operations` is what is left of the glyph's
/// [`CHARSTRING_OPERATIONS`], shared across the calls; a walk that spends it is counted as
/// reaching one too, on the same side.
fn calls_local_subr(
    charstring: &[u8],
    font: &CffFontRef<'_>,
    depth: u8,
    operations: &mut u32,
) -> bool {
    let mut walk = LocalSubrWalk {
        font,
        stack: Vec::new(),
        stems: 0,
        operations,
    };
    walk.run(charstring, depth) == Stopped::LocalSubr
}

/// Where [`LocalSubrWalk::run`] stopped in the charstring or subroutine it was given.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stopped {
    /// At a `callsubr`, or at a point counted as one.
    LocalSubr,
    /// At `return`, or past the last byte, where the caller goes on.
    Returned,
    /// At `endchar`, which ends the glyph in whichever subroutine it is run.
    Ended,
}

/// [`calls_local_subr`]'s walk, its state the charstring's and every subroutine's it calls.
///
/// Adobe Technical Note #5177 section 4.7 leaves what the caller pushed before a call and what the
/// subroutine pushes on the one argument stack, and section 4.3 sizes a `hintmask`'s data bytes
/// by every stem hint the charstring declared, inside a subroutine or not, so the operand stack
/// and the stem count are the walk's, not a call's: a call that starts at no stems reads a mask
/// after the caller's hints as too short, and the bytes after it as operators. Section 4.2 makes
/// `endchar` the last operator of the outline, so a subroutine that runs it ends the walk.
struct LocalSubrWalk<'w, 'f> {
    font: &'w CffFontRef<'f>,
    stack: Vec<i64>,
    /// The stems declared so far, which fix a mask's length.
    stems: usize,
    /// What is left of the glyph's [`CHARSTRING_OPERATIONS`].
    operations: &'w mut u32,
}

impl LocalSubrWalk<'_, '_> {
    fn run(&mut self, charstring: &[u8], depth: u8) -> Stopped {
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

        let gsubrs = self.font.global_subrs();
        let bias: i64 = match gsubrs.count() {
            0..1240 => 107,
            1240..33900 => 1131,
            _ => 32768,
        };
        let mut i = 0usize;
        while let Some(&b0) = charstring.get(i) {
            let Some(left) = self.operations.checked_sub(1) else {
                return Stopped::LocalSubr;
            };
            *self.operations = left;
            match b0 {
                32..=254 => {
                    let b1 = charstring.get(i.saturating_add(1)).copied().unwrap_or(0);
                    let (value, width) = small_operand(b0, b1);
                    self.stack.push(value);
                    i = i.saturating_add(width);
                }
                28 => {
                    let value = charstring
                        .get(i.saturating_add(1)..i.saturating_add(3))
                        .map_or(0, |b| i64::from(i16::from_be_bytes([b[0], b[1]])));
                    self.stack.push(value);
                    i = i.saturating_add(3);
                }
                255 => {
                    // 16.16 fixed; only the integer part could name a subroutine.
                    let value = charstring
                        .get(i.saturating_add(1)..i.saturating_add(5))
                        .map_or(0, |b| {
                            i64::from(i32::from_be_bytes([b[0], b[1], b[2], b[3]])) >> 16
                        });
                    self.stack.push(value);
                    i = i.saturating_add(5);
                }
                CALLSUBR => return Stopped::LocalSubr,
                CALLGSUBR => {
                    let Some(operand) = self.stack.pop() else {
                        return Stopped::LocalSubr;
                    };
                    let index = usize::try_from(operand.saturating_add(bias)).unwrap_or(usize::MAX);
                    if depth < MAX_DEPTH
                        && let Some(subr) = gsubrs.get(index)
                    {
                        match self.run(subr, depth.saturating_add(1)) {
                            Stopped::Returned => {}
                            stopped => return stopped,
                        }
                    }
                    i = i.saturating_add(1);
                }
                RETURN => return Stopped::Returned,
                ENDCHAR => return Stopped::Ended,
                HSTEM | VSTEM | HSTEMHM | VSTEMHM => {
                    self.stems = self.stems.saturating_add(self.stack.len() / 2);
                    self.stack.clear();
                    i = i.saturating_add(1);
                }
                HINTMASK | CNTRMASK => {
                    self.stems = self.stems.saturating_add(self.stack.len() / 2);
                    self.stack.clear();
                    i = i
                        .saturating_add(1)
                        .saturating_add(self.stems.saturating_add(7) / 8);
                }
                ESCAPE_BYTE => {
                    self.stack.clear();
                    i = i.saturating_add(2);
                }
                _ => {
                    self.stack.clear();
                    i = i.saturating_add(1);
                }
            }
        }
        Stopped::Returned
    }
}

// ---------------------------------------------------------------------------------------------
// Restating a program's advances, which is the one thing a converter may change about a font.
// ---------------------------------------------------------------------------------------------

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
/// `None` is [`LeadingWidth`]'s third answer: the width is not in this charstring at all, or
/// the first operator is stated with fewer operands than it takes, so that no width can be
/// put before it that the count would read as one.
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
                let odd = operands.len() % 2 == 1;
                return has_width(&operands, odd, !odd);
            }
            RMOVETO => return has_width(&operands, operands.len() > 2, operands.len() == 2),
            HMOVETO | VMOVETO => {
                return has_width(&operands, operands.len() > 1, operands.len() == 1);
            }
            ENDCHAR => {
                let count = operands.len();
                return has_width(
                    &operands,
                    count == 1 || count == 5,
                    count == 0 || count == 4,
                );
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

/// The first operand's span where `present`, which is where the width sits when there is one;
/// [`LeadingWidth::Absent`] where it is not and `insertable`; and `None` where it is neither.
///
/// `insertable` is whether one operand more before the operator is read as its width by the
/// same count: `rmoveto` stated with one operand, or `endchar` with two, takes a second or a
/// third as an argument rather than as a width, so a width inserted there would be read as
/// none and the glyph would keep the `defaultWidthX` it had.
fn has_width(
    operands: &[std::ops::Range<usize>],
    present: bool,
    insertable: bool,
) -> Option<LeadingWidth> {
    match operands.first().filter(|_| present) {
        Some(span) => Some(LeadingWidth::At(span.clone())),
        None => insertable.then_some(LeadingWidth::Absent),
    }
}

/// The `Subrs` operator of a Private DICT (Adobe Technical Note #5176, Table 23, operator 19),
/// whose operand is an offset **relative to the Private DICT's own start**.
pub(crate) const OP_SUBRS: u16 = 19;

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

/// How many operands and operators one walk of a glyph's charstring may read, every one inside
/// the subroutines it calls counted.
///
/// Nesting alone does not bound the work: Adobe Technical Note #5177's Appendix B allows ten
/// levels, so a subroutine that calls the next one *k* times is entered about *k*⁹ times, and a
/// charstring has room for thousands of calls. `read-fonts`' evaluator, which draws every glyph
/// and reads every width, stops a glyph at this many (its own `MAX_OPERATIONS`); this crate's
/// own walks of a charstring stop at the same count, so a program that one refuses is a program
/// the other gives up on as well (ADR 2572).
pub(crate) const CHARSTRING_OPERATIONS: u32 = 1_000_000;

/// The subroutine bias, which Adobe Technical Note #5177 section 4.7 makes a function of how
/// many subroutines the INDEX holds.
pub(crate) fn bias(count: usize) -> i64 {
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

/// How many bytes of a subroutine are its body: the bytes before the `return` that ends it, read
/// as Type 2, or all of them where it states none.
///
/// A subroutine is spliced into a charstring in place of the call, which is what `callsubr`
/// does; the `return` that ends it has nothing to return to once it is inlined, so it is
/// dropped. The first `return` read at the subroutine's own level is the one that ends it,
/// wherever it stands: the interpreter goes back to the caller there (Adobe Technical Note #5177
/// section 4.7), so a byte after it is never run, and splicing that byte in would run it (ADR
/// 2628).
///
/// **Finding it means running the subroutine as the interpreter would, nested calls and all**
/// (ADR 2779). `hintmask` and `cntrmask` are followed by one data bit per stem hint declared so
/// far (section 4.3), and a mask byte may be 11. The count includes the stems the caller's
/// operands declare when the subroutine's first operator takes them, and the stems declared
/// inside every subroutine the body calls before the mask — URW's Nimbus faces factor the hints
/// of a glyph into a subroutine that calls another for its first stems and then states a mask —
/// so `head` is the caller's operands before the call, its subroutine number left out, and
/// every call the body makes is run to count what it declares. An `endchar` ends the glyph
/// wherever it is run: at the subroutine's own level the body is the bytes through it, and in a
/// nested call the whole subroutine, whose later bytes are never run.
///
/// `None` where the subroutine is empty, where a call names no subroutine or nests past Appendix
/// B's ten levels, where the walk passes [`CHARSTRING_OPERATIONS`], or where an escaped operator
/// other than the four flex operators is met: an arithmetic or storage operator changes the
/// stack by a rule this walk does not keep, and a stem count after it would be a guess.
fn subroutine_body(subr: &[u8], head: &[u8], subrs: &Subroutines<'_>) -> Option<usize> {
    if subr.is_empty() {
        return None;
    }
    let mut walk = BodyWalk {
        subrs,
        stack: Vec::new(),
        stems: 0,
        operations: CHARSTRING_OPERATIONS,
    };
    // The caller's operands are numbers only, so running them pushes them and returns.
    walk.run(head, 0)?;
    match walk.run(subr, 1)? {
        BodyEnd::Returned(at) | BodyEnd::Ended(at) => Some(at),
    }
}

/// Where [`BodyWalk::run`] stopped in the code it was given.
enum BodyEnd {
    /// At a `return` at this offset, or at the end of the code, which returns as well.
    Returned(usize),
    /// Just past an `endchar`, which ends the glyph; the code's end where a call it made ran one.
    Ended(usize),
}

/// [`subroutine_body`]'s walk: the operand stack, the stems declared, and the operations left.
struct BodyWalk<'s, 'a> {
    /// The subroutines a call reaches.
    subrs: &'s Subroutines<'a>,
    /// The operand stack, bottom first.
    stack: Vec<i64>,
    /// The stem hints declared so far, which size a mask.
    stems: usize,
    /// How many operands and operators are still to be read.
    operations: u32,
}

impl BodyWalk<'_, '_> {
    /// Runs `code` at nesting `depth` until its `return`, its end or an `endchar`.
    fn run(&mut self, code: &[u8], depth: usize) -> Option<BodyEnd> {
        use stack_clearing::{CNTRMASK, ENDCHAR, HINTMASK, HSTEM, HSTEMHM, VSTEM, VSTEMHM};
        // Adobe Technical Note #5177 Appendix B bounds subroutine nesting to ten levels.
        const MAX_NESTING: usize = 10;
        // `hflex`, `flex`, `hflex1` and `flex1` (section 4.1): path operators, which clear.
        const FLEX: std::ops::RangeInclusive<u8> = 34..=37;
        let mut at = 0usize;
        while let Some(&b0) = code.get(at) {
            self.operations = self.operations.checked_sub(1)?;
            let step = match b0 {
                28 => {
                    let bytes = code.get(at.checked_add(1)?..at.checked_add(3)?)?;
                    self.stack
                        .push(i64::from(i16::from_be_bytes([bytes[0], bytes[1]])));
                    3
                }
                32..=254 => {
                    let next = code.get(at.saturating_add(1)).copied().unwrap_or(0);
                    let (value, width) = small_operand(b0, next);
                    self.stack.push(value);
                    width
                }
                255 => {
                    // 16.16 fixed; only the integer part could name a subroutine.
                    let bytes = code.get(at.checked_add(1)?..at.checked_add(5)?)?;
                    self.stack.push(
                        i64::from(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                            >> 16,
                    );
                    5
                }
                CALL_LOCAL | CALL_GLOBAL => {
                    let named = self.stack.pop()?;
                    let table = if b0 == CALL_LOCAL {
                        self.subrs.local
                    } else {
                        self.subrs.global
                    };
                    let index = usize::try_from(named.checked_add(bias(table.len()))?).ok()?;
                    if depth >= MAX_NESTING {
                        return None;
                    }
                    if let BodyEnd::Ended(_) =
                        self.run(table.get(index)?, depth.saturating_add(1))?
                    {
                        return Some(BodyEnd::Ended(code.len()));
                    }
                    1
                }
                RETURN => return Some(BodyEnd::Returned(at)),
                ENDCHAR => return Some(BodyEnd::Ended(at.saturating_add(1))),
                HSTEM | VSTEM | HSTEMHM | VSTEMHM => {
                    self.declare();
                    1
                }
                HINTMASK | CNTRMASK => {
                    // Operands before a mask are an implicit vstem (section 4.3).
                    self.declare();
                    1usize.saturating_add(self.stems.div_ceil(8))
                }
                ESCAPE_BYTE => {
                    if !code
                        .get(at.saturating_add(1))
                        .is_some_and(|b1| FLEX.contains(b1))
                    {
                        return None;
                    }
                    self.stack.clear();
                    2
                }
                _ => {
                    self.stack.clear();
                    1
                }
            };
            at = at.checked_add(step)?;
            if at > code.len() {
                return None;
            }
        }
        Some(BodyEnd::Returned(code.len()))
    }

    /// The stems a stem hint or a mask declares out of the stack, which it clears; a width the
    /// first of them takes is the odd operand the halving drops.
    fn declare(&mut self) {
        self.stems = self.stems.saturating_add(self.stack.len() / 2);
        self.stack.clear();
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
                let keep = subroutine_body(body, charstring.get(..index.start)?, subrs)?;
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

/// The refusal's sentence where a copy of `.notdef`'s charstring cannot carry the advance asked.
const NOTDEF_NOT_RESTATED: &str = "the advance does not fit a charstring's width \
                                              operand, or .notdef's charstring states its width \
                                              inside a subroutine";

/// Why [`charstring_with_advance`] wrote no charstring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotRestated {
    /// No width can be put where Adobe Technical Note #5177 section 3.1 reads one, or the
    /// advance does not fit the operand; the caller's own sentence says which of its cases.
    Placement,
    /// An operator after the first pops the operand stack while it is empty
    /// ([`pops_an_empty_stack`]).
    EmptyStack,
    /// The first operator is stated with every operand the argument stack holds, so a width
    /// written in front of it would be one past [`MOST_OPERANDS`] ([`leading_operands`]).
    FullStack,
}

impl NotRestated {
    /// The refusal for `glyph`, `placement` being the caller's sentence for
    /// [`NotRestated::Placement`].
    fn refusal(self, glyph: u16, placement: &str) -> CffError {
        let detail = match self {
            Self::Placement => placement.to_owned(),
            Self::EmptyStack => "its charstring states an operator that takes operands with the \
                                 operand stack empty after the first, where the outline reader \
                                 takes a value left from before, and the width written in front \
                                 is that value"
                .to_owned(),
            Self::FullStack => "its charstring states its first operator with the 48 operands \
                                Adobe Technical Note #5177's Appendix B lets the argument stack \
                                hold, so a width written in front of them would be a forty-ninth"
                .to_owned(),
        };
        CffError::AdvanceNotRestatable { glyph, detail }
    }
}

/// Whether `charstring` states, after its first operator and before any operand, an operator
/// that takes operands — a `callsubr`, a `callgsubr`, a move, a line, a curve or an escaped
/// operator such as `seac` — while the operand stack is empty.
///
/// Adobe Technical Note #5177 section 4 gives every such operator its operands, so a charstring
/// stating one with none states nothing the note defines. `read-fonts`' evaluator does not refuse
/// it: popping an empty stack answers the value its bottom slot last held, and the first operator
/// clears the stack without overwriting that slot. The bottom slot is where a width is written,
/// so a width inserted or restated in front changes what the operator reads — the subroutine a
/// `callsubr` calls above all — until the next operand overwrites it. Only stem hints, masks,
/// `endchar` and `return`, which take nothing off an empty stack, are passed over on the way.
fn pops_an_empty_stack(charstring: &[u8]) -> bool {
    use stack_clearing::{CNTRMASK, ENDCHAR, HINTMASK, HSTEM, HSTEMHM, VSTEM, VSTEMHM};
    let mut operands = 0usize;
    let mut stems = 0usize;
    let mut cleared = false;
    let mut at = 0usize;
    while let Some(&b0) = charstring.get(at) {
        let step = match b0 {
            28 | 32..=246 | 247..=254 | 255 if cleared => return false,
            28 => 3,
            32..=246 => 1,
            247..=254 => 2,
            255 => 5,
            HSTEM | VSTEM | HSTEMHM | VSTEMHM => {
                stems = stems.saturating_add(operands / 2);
                1
            }
            HINTMASK | CNTRMASK => {
                stems = stems.saturating_add(operands / 2);
                1usize.saturating_add(stems.div_ceil(8))
            }
            ENDCHAR | RETURN => return false,
            _ if cleared => return true,
            _ => 1,
        };
        if matches!(b0, 28 | 32..=255) {
            operands = operands.saturating_add(1);
        } else {
            operands = 0;
            cleared = true;
        }
        at = at.saturating_add(step);
    }
    false
}

/// The second byte of the escaped operator 12 6, which Adobe Technical Note #5177's Appendix A
/// lists as reserved and which Type 1's `seac` was.
const RESERVED_SEAC: u8 = 6;

/// How many operands and operators [`first_reserved_seac`] reads over the whole program, every
/// glyph's walk and every subroutine it enters counted.
///
/// One glyph's walk stops at [`CHARSTRING_OPERATIONS`], but this walk visits every glyph, and
/// 65 535 glyphs at that bound is work no conversion can wait for. Thirty-two million is room for
/// 65 535 glyphs of almost five hundred operations each; a program past it is refused by name,
/// on the side the walk errs on (ADR 2628).
const PROGRAM_SEAC_OPERATIONS: u32 = 32_000_000;

/// How many operands Adobe Technical Note #5177's Appendix B lets the argument stack hold.
const MOST_OPERANDS: usize = 48;

/// Where [`SeacWalk::walk`] left a charstring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reached {
    /// A `return`, or the end of the bytes, with the walk to go on in the caller.
    Return,
    /// `endchar`, or a fault the outline reader stops the glyph at.
    End,
    /// The escaped operator 12 6, or a point past which the walk cannot tell what is read.
    Reserved,
}

/// A walk of one glyph's charstring through the subroutines it calls, far enough to tell
/// whether it reaches the escaped operator 12 6.
///
/// The operand stack is followed so that the subroutine a call names is the one entered, and the
/// stems are counted across calls so that a `hintmask`'s data bytes are skipped as the reader
/// skips them. Whatever the walk cannot follow — a call on an empty stack, which the outline
/// reader answers from a stale slot, an operand an arithmetic operator computed, a spent bound —
/// counts as reaching 12 6, the side on which a writer refuses rather than writes.
struct SeacWalk<'a> {
    /// The subroutines this glyph's calls reach.
    subrs: &'a Subroutines<'a>,
    /// The operands since the last operator that cleared them.
    stack: Vec<i64>,
    /// Whether `stack` is what the reader's holds: false after an operator whose results this
    /// walk does not compute.
    known: bool,
    /// The stems declared so far, which fix a mask's length.
    stems: usize,
    /// What is left of the bound on this glyph's walk.
    operations: u32,
}

impl SeacWalk<'_> {
    /// Adobe Technical Note #5177 section 4.7 bounds subroutine nesting to ten levels.
    const MAX_DEPTH: u8 = 10;

    /// Walks `charstring`, entered at `depth` calls below the glyph's own.
    fn walk(&mut self, charstring: &[u8], depth: u8) -> Reached {
        use stack_clearing::{CNTRMASK, ENDCHAR, HINTMASK, HSTEM, HSTEMHM, VSTEM, VSTEMHM};
        let mut at = 0usize;
        while let Some(&b0) = charstring.get(at) {
            let Some(left) = self.operations.checked_sub(1) else {
                return Reached::Reserved;
            };
            self.operations = left;
            let step = match b0 {
                28 | 32..=255 => {
                    if self.stack.len() >= MOST_OPERANDS {
                        return Reached::Reserved;
                    }
                    let (value, width) = charstring_operand(charstring, at);
                    self.stack.push(value);
                    width
                }
                CALL_LOCAL | CALL_GLOBAL => {
                    let subrs = if b0 == CALL_LOCAL {
                        self.subrs.local
                    } else {
                        self.subrs.global
                    };
                    let Some(operand) = self.stack.pop().filter(|_| self.known) else {
                        return Reached::Reserved;
                    };
                    let Some(subr) = usize::try_from(operand.saturating_add(bias(subrs.len())))
                        .ok()
                        .and_then(|index| subrs.get(index))
                    else {
                        return Reached::End;
                    };
                    if depth >= Self::MAX_DEPTH {
                        return Reached::End;
                    }
                    match self.walk(subr, depth.saturating_add(1)) {
                        Reached::Return => 1,
                        other => return other,
                    }
                }
                RETURN => return Reached::Return,
                ENDCHAR => return Reached::End,
                HSTEM | VSTEM | HSTEMHM | VSTEMHM | HINTMASK | CNTRMASK => {
                    if !self.known {
                        return Reached::Reserved;
                    }
                    self.stems = self.stems.saturating_add(self.stack.len() / 2);
                    self.stack.clear();
                    if matches!(b0, HINTMASK | CNTRMASK) {
                        1usize.saturating_add(self.stems.div_ceil(8))
                    } else {
                        1
                    }
                }
                ESCAPE_BYTE => {
                    let Some(&b1) = charstring.get(at.saturating_add(1)) else {
                        return Reached::End;
                    };
                    if b1 == RESERVED_SEAC {
                        return Reached::Reserved;
                    }
                    // The four flex operators clear the stack as Technical Note #5177 section
                    // 4.1 defines them; any other escaped operator the reader may compute with,
                    // or, as Type 1's `sbw` and `dotsection` are read, leave the stack under.
                    self.known = matches!(b1, 34..=37);
                    self.stack.clear();
                    2
                }
                // An operator Technical Note #5177's Appendix A reserves is one the reader may
                // leave the stack under, as it does Type 1's `hsbw` (13); every other clears it.
                _ => {
                    self.known = !matches!(b0, 0 | 2 | 9 | 13 | 15..=17);
                    self.stack.clear();
                    1
                }
            };
            at = at.saturating_add(step);
        }
        Reached::Return
    }
}

/// One charstring operand at `at`, and how many bytes it takes (Adobe Technical Note #5177
/// section 3.2); a 16.16 fixed number is kept by its integer part, the only part that could name
/// a subroutine.
fn charstring_operand(charstring: &[u8], at: usize) -> (i64, usize) {
    let byte = |offset: usize| {
        charstring
            .get(at.saturating_add(offset))
            .copied()
            .unwrap_or(0)
    };
    match byte(0) {
        28 => (i64::from(i16::from_be_bytes([byte(1), byte(2)])), 3),
        255 => (
            i64::from(i32::from_be_bytes([byte(1), byte(2), byte(3), byte(4)])) >> 16,
            5,
        ),
        b0 => small_operand(b0, byte(1)),
    }
}

/// The first glyph whose charstring reaches the escaped operator 12 6, itself or through the
/// subroutines it calls, or that a walk bounded as [`SeacWalk`] is cannot tell of.
///
/// Adobe Technical Note #5177's Appendix A lists 12 6 as reserved: a Type 2 charstring makes an
/// accented character with `endchar`'s four optional arguments (section 4.1), and the operator
/// that did so in Type 1 is not one of its own. `read-fonts`' evaluator reads 12 6 as Type 1's
/// `seac` all the same and, where the glyph has stated no width before it, gives the glyph its
/// base character's width — the base being `.notdef` where the stack held no codes. So a glyph
/// that states 12 6 has an advance some other glyph's charstring decides, and a writer that
/// restates any advance cannot say it kept that glyph's.
fn first_reserved_seac<'a>(
    charstrings: &[Vec<u8>],
    subrs_of: impl Fn(u16) -> Subroutines<'a>,
) -> Option<u16> {
    let mut program = PROGRAM_SEAC_OPERATIONS;
    for (glyph, charstring) in (0..=u16::MAX).zip(charstrings) {
        let subrs = subrs_of(glyph);
        let mut walk = SeacWalk {
            subrs: &subrs,
            stack: Vec::new(),
            known: true,
            stems: 0,
            operations: CHARSTRING_OPERATIONS.min(program),
        };
        let reached = walk.walk(charstring, 0);
        let spent = CHARSTRING_OPERATIONS
            .min(program)
            .saturating_sub(walk.operations);
        program = program.saturating_sub(spent);
        if reached == Reached::Reserved {
            return Some(glyph);
        }
    }
    None
}

/// One charstring with its width operand restated as `advance`, in the program's own units.
///
/// The outline is untouched: what changes is the one leading operand Adobe Technical Note #5177
/// section 3.1 defines, written in the three-byte form so that its length does not depend on its
/// value. [`NotRestated::Placement`] where the charstring states its width through a subroutine
/// ([`width_span`]) or where the difference from `nominal` will not fit a 16-bit operand, and
/// [`NotRestated::EmptyStack`] where what the width is written in front of would read it as
/// another operator's operand, and [`NotRestated::FullStack`] where it would put the operand
/// stack past the limit Adobe Technical Note #5177's Appendix B sets.
fn charstring_with_advance(
    charstring: &[u8],
    advance: i64,
    nominal: i64,
    subrs: &Subroutines<'_>,
) -> Result<Vec<u8>, NotRestated> {
    let restated =
        charstring_restated(charstring, advance, nominal, subrs).ok_or(NotRestated::Placement)?;
    if leading_operands(&restated) > MOST_OPERANDS {
        return Err(NotRestated::FullStack);
    }
    if pops_an_empty_stack(&restated) {
        return Err(NotRestated::EmptyStack);
    }
    Ok(restated)
}

/// How many operands a charstring states before its first operator.
///
/// A width inserted where the charstring stated none is one operand more on the stack its first
/// operator clears, and a program whose first operator already takes 48 would come back with 49:
/// past Appendix B's limit, which [`SeacWalk`] reads as a point it cannot tell past, so the
/// writer's own output would be refused by the walk its input passed (ADR 2628, decision 8).
fn leading_operands(charstring: &[u8]) -> usize {
    let mut count = 0usize;
    let mut at = 0usize;
    while let Some(&b0) = charstring.get(at) {
        let width = match b0 {
            28 => 3,
            32..=246 => 1,
            247..=254 => 2,
            255 => 5,
            _ => break,
        };
        count = count.saturating_add(1);
        at = at.saturating_add(width);
    }
    count
}

/// [`charstring_with_advance`]'s rewrite, before the operators after the width are read.
fn charstring_restated(
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

/// A Private DICT's `nominalWidthX` as the charstring evaluator adds it, which is zero where the
/// DICT does not state one.
///
/// Technical Note 5176's Table 23 makes the operand a number, and a producer may write it as a
/// real (operand 30). `read-fonts` adds the operand floored to a whole unit to every charstring
/// width it evaluates, so a width restated as `advance - nominal` reads back as `advance` only
/// when `nominal` is that same floor; an integer-only reading took a real as zero and wrote a
/// width the read-back then found larger by the whole nominal.
fn nominal_width(private: &[u8]) -> i64 {
    dict::entries(private, None)
        .filter_map(Result::ok)
        .find_map(|entry| match entry {
            dict::Entry::NominalWidthX(width) => Some(i64::from(width.floor().to_i32())),
            _ => None,
        })
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
    if let Some(misstated) = layout.misstated {
        return Err(misstated.refusal());
    }
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
    let subfont_of = |glyph: u16| {
        if cid_keyed {
            font.subfont_index(GlyphId::from(glyph)).unwrap_or(0)
        } else {
            0
        }
    };
    let subrs_of = |glyph: u16| Subroutines {
        global: &global,
        local: locals
            .get(usize::from(subfont_of(glyph)))
            .map_or(&[][..], Vec::as_slice),
    };
    if !advances.is_empty()
        && let Some(glyph) = first_reserved_seac(&charstrings, subrs_of)
    {
        return Err(CffError::AdvanceNotRestatable {
            glyph,
            detail: "its charstring reaches the escaped operator 12 6, which Technical Note \
                     #5177 reserves and the outline reader takes as Type 1's seac, giving the \
                     glyph its base character's width, so restating any advance may restate \
                     this one; or the walk that would tell spent its bound"
                .to_owned(),
        });
    }
    for (glyph, advance) in advances {
        let slot =
            charstrings
                .get_mut(usize::from(*glyph))
                .ok_or(CffError::AdvanceNotRestatable {
                    glyph: *glyph,
                    detail: "the program's CharStrings INDEX has no entry for it".to_owned(),
                })?;
        let against = nominal
            .get(usize::from(subfont_of(*glyph)))
            .copied()
            .unwrap_or(0);
        *slot =
            charstring_with_advance(slot, *advance, against, &subrs_of(*glyph)).map_err(|why| {
                why.refusal(
                *glyph,
                "its charstring does not begin with one of the stack-clearing operators a width \
                 is stated against, so the width is inside a subroutine, or begins with one \
                 stated with too few operands for a width before it to be read as one",
            )
            })?;
    }
    Ok(layout.with_charstrings(data, &charstrings))
}

/// One glyph [`with_empty_glyphs`] added, by index and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedGlyph {
    /// Its index in the rebuilt program's `CharStrings` INDEX.
    pub glyph: u16,
    /// The name the rebuilt program's charset gives it.
    pub name: String,
}

/// A name-keyed program with one empty glyph appended per advance, each under a fresh name.
///
/// **What this is for is ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 under
/// `doc/questions/A48`'s permission**: a code a substituted font's encoding sends to `.notdef`
/// is given a real glyph that draws nothing instead, so that no text-showing operator references
/// `.notdef` and the page shows what it showed. The glyph is a Type 2 charstring of one width
/// operand and `endchar` (Adobe Technical Note #5177, sections 3.1 and 4.1), so it has no
/// outline at all, and its advance is the one asked for — the font dictionary's own width for
/// the code, which section 6.2.11.5 holds the program to.
///
/// **The names are `notdef`, `notdef.1`, `notdef.2`, … , the first the program does not already
/// hold.** None is in the Adobe Glyph List, so a code naming one is given no character by
/// ISO 32000-2 §9.10.2's glyph-name method: the absence the page had is kept rather than filled
/// in, which is the condition A48 sets. Every glyph the program held keeps its index, its name
/// and its charstring byte for byte.
///
/// # Errors
///
/// [`CffError::Malformed`] where the program does not open, is CID-keyed (which names no glyph),
/// or its INDEX structures do not read; [`CffError::NoCharset`] where it states no charset; and
/// [`CffError::AdvanceNotRestatable`] where an advance does not fit a charstring's width operand.
pub fn with_empty_glyphs(
    data: &[u8],
    advances: &[i64],
) -> Result<(Vec<u8>, Vec<AddedGlyph>), CffError> {
    let glyphs: Vec<GlyphToGive> = advances
        .iter()
        .map(|advance| GlyphToGive {
            draws: GivenOutline::Nothing,
            name: None,
            advance: *advance,
        })
        .collect();
    with_glyphs_given(data, &glyphs)
}

/// What a glyph [`with_glyphs_given`] appends draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GivenOutline {
    /// Nothing: a charstring of one width operand and `endchar` (Adobe Technical Note #5177,
    /// sections 3.1 and 4.1).
    Nothing,
    /// Glyph 0's own outline: `.notdef`'s charstring, every byte of it but the width operand,
    /// interpreted against the same Private DICT and the same subroutines.
    NotdefsOwn,
}

/// One glyph for [`with_glyphs_given`] to append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphToGive {
    /// What it draws.
    pub draws: GivenOutline,
    /// The name its charset entry states, or `None` for the first of `notdef`, `notdef.1`, … the
    /// program does not hold. A name the program already holds is refused, since two glyphs of
    /// one name leave a name-keyed lookup choosing between them.
    pub name: Option<String>,
    /// Its advance, in the program's units.
    pub advance: i64,
}

/// The most glyphs one call appends: a simple font has 256 codes, so no conversion asks for
/// more, and the bound is checked before any table is grown.
pub const MOST_GLYPHS_GIVEN: usize = 256;

/// A name-keyed program with the glyphs asked for appended, each under its own name.
///
/// **The construction ISO 19005-2 section 6.2.11.8 and section 6.2.11.4.1 (ISO 19005-4 sections
/// 6.2.10.9 and 6.2.10.4.1) are answered by** where a code of an embedded program reaches
/// `.notdef`, or reaches no glyph: the code is given a glyph of its own that draws what this
/// reader drew for it — [`GivenOutline::NotdefsOwn`] for a code on glyph 0, whose outline this
/// reader drew, [`GivenOutline::Nothing`] for one that reached no glyph — at the advance the font
/// dictionary states (`doc/adr/2578`). The `CharStrings` INDEX grows by one entry per glyph, the
/// charset is restated in format 0 with one SID per new glyph (a standard string's own SID where
/// the name is one of the 391, Adobe Technical Note #5176 section 10 and Appendix A; a new
/// String INDEX entry otherwise), and every offset of the Top DICT is restated by
/// `Layout::with_glyph_tables`. Every glyph the program held keeps its index, its name and its
/// charstring byte for byte; the program's built-in `Encoding` is not touched, and with it any
/// code the program encodes keeps its glyph.
///
/// # Errors
///
/// [`CffError::Malformed`] where the program does not open, is CID-keyed, holds a name asked
/// for already, is asked for more than [`MOST_GLYPHS_GIVEN`] glyphs, or its INDEX structures do
/// not read; [`CffError::NoCharset`] where it states no charset; and
/// [`CffError::AdvanceNotRestatable`] where an advance does not fit a charstring's width operand
/// or `.notdef`'s charstring states its width where it cannot be restated.
pub fn with_glyphs_given(
    data: &[u8],
    glyphs: &[GlyphToGive],
) -> Result<(Vec<u8>, Vec<AddedGlyph>), CffError> {
    let malformed = |detail: &str| CffError::Malformed {
        detail: detail.to_owned(),
    };
    if glyphs.len() > MOST_GLYPHS_GIVEN {
        return Err(malformed(
            "more glyphs are asked for than a simple font has codes",
        ));
    }
    let font = open(data)?;
    if font.is_cid() {
        return Err(malformed(
            "the program is CID-keyed, and a CID-keyed program names no glyph",
        ));
    }
    let layout =
        Layout::read(data).ok_or_else(|| malformed("the Top DICT INDEX could not be located"))?;
    if let Some(misstated) = layout.misstated {
        return Err(misstated.refusal());
    }
    let charstrings_at = layout
        .entries
        .iter()
        .find(|(_, op)| *op == OP_CHARSTRINGS)
        .and_then(|(operands, _)| dict_int(operands))
        .and_then(|at| usize::try_from(at).ok())
        .ok_or_else(|| malformed("the Top DICT states no CharStrings offset"))?;
    let mut charstrings: Vec<Vec<u8>> = index_items(data, charstrings_at)
        .ok_or_else(|| malformed("the CharStrings INDEX could not be read"))?
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect();
    let mut strings: Vec<Vec<u8>> = index_items(data, layout.top_index.end)
        .ok_or_else(|| malformed("the String INDEX could not be read"))?
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect();
    let charset = font.charset().ok_or(CffError::NoCharset)?;
    // Charset format 0 (Technical Note #5176, section 13): a format byte, then one SID for every
    // glyph after `.notdef`, which has none.
    let mut table = vec![0u8];
    let mut named: std::collections::BTreeSet<Vec<u8>> = std::collections::BTreeSet::new();
    for (glyph, sid) in charset.iter() {
        if let Some(name) = font.string(sid) {
            named.insert(name.to_vec());
        }
        if glyph.to_u32() != 0 {
            table.extend_from_slice(&sid.to_u16().to_be_bytes());
        }
    }
    // Every string the program holds, used as a glyph name or not, so that no fresh name spells
    // one: the String INDEX is the whole of what a lookup by name could confuse it with.
    named.insert(b".notdef".to_vec());
    let mut held = named.clone();
    held.extend(strings.iter().cloned());
    let nominal = nominal_widths(data, &layout, false)
        .first()
        .copied()
        .unwrap_or(0);
    let no_subroutines = Subroutines {
        global: &[],
        local: &[],
    };
    let notdef = charstrings
        .first()
        .cloned()
        .ok_or_else(|| malformed("the CharStrings INDEX holds no .notdef glyph"))?;
    let global = global_subroutines(data);
    let top = index_item(data, layout.top_index.start, 0).unwrap_or_default();
    let local = local_subroutines(data, top);
    let notdefs_subroutines = Subroutines {
        global: &global,
        local: &local,
    };
    let mut added = Vec::with_capacity(glyphs.len());
    let mut suffix = 0usize;
    for asked in glyphs {
        let name = name_for(asked, (&named, &held), &mut suffix)?;
        named.insert(name.as_bytes().to_vec());
        held.insert(name.as_bytes().to_vec());
        let glyph = u16::try_from(charstrings.len()).map_err(|_| {
            malformed("the program already holds as many glyphs as an INDEX counts")
        })?;
        let sid = string_id(&name, &mut strings)
            .ok_or_else(|| malformed("the String INDEX holds as many strings as a SID counts"))?;
        let (source, subroutines): (&[u8], _) = match asked.draws {
            GivenOutline::Nothing => (&[stack_clearing::ENDCHAR], &no_subroutines),
            GivenOutline::NotdefsOwn => (&notdef, &notdefs_subroutines),
        };
        let charstring = charstring_with_advance(source, asked.advance, nominal, subroutines)
            .map_err(|why| why.refusal(glyph, NOTDEF_NOT_RESTATED))?;
        charstrings.push(charstring);
        table.extend_from_slice(&sid.to_be_bytes());
        added.push(AddedGlyph { glyph, name });
    }
    let rebuilt = layout.with_glyph_tables(data, &strings, (&charstrings, &table), None)?;
    read_back(&rebuilt, &charstrings, &added)?;
    Ok((rebuilt, added))
}

/// Whether every entry a writer here follows is stated once and with the operands Technical
/// Note #5176 gives it, in the Top DICT, its Font DICTs and their Private DICTs — the shared rule
/// of [`exactly`] and [`misstated_entry`], as a test; [`Misstated::refusal`] names the entry.
fn offsets_stated_once(layout: &Layout) -> bool {
    layout.misstated.is_none()
}

/// The name `asked` is given: its own, refused where a glyph already holds it, or the first of
/// `notdef`, `notdef.1`, … that no string held spells, `suffix` counting the candidates tried.
fn name_for(
    asked: &GlyphToGive,
    (named, held): (
        &std::collections::BTreeSet<Vec<u8>>,
        &std::collections::BTreeSet<Vec<u8>>,
    ),
    suffix: &mut usize,
) -> Result<String, CffError> {
    if let Some(name) = &asked.name {
        if named.contains(name.as_bytes()) {
            return Err(CffError::Malformed {
                detail: "a name asked for is one the program already holds".to_owned(),
            });
        }
        return Ok(name.clone());
    }
    // At most one candidate past every name held, so the walk is bounded by them.
    loop {
        let candidate = if *suffix == 0 {
            "notdef".to_owned()
        } else {
            format!("notdef.{suffix}")
        };
        *suffix = suffix.saturating_add(1);
        if !held.contains(candidate.as_bytes()) {
            return Ok(candidate);
        }
    }
}

/// The identifier `name` is charted under: a standard string's own (Adobe Technical Note #5176,
/// Appendix A), one the String INDEX already holds, or a new entry pushed onto `strings`; `None`
/// where no 16-bit identifier is left.
fn string_id(name: &str, strings: &mut Vec<Vec<u8>>) -> Option<u16> {
    let standard = skrifa::raw::ps::string::STANDARD_STRINGS
        .iter()
        .position(|standard| *standard == name);
    // A name the String INDEX already holds keeps its identifier, so no string is written
    // twice.
    let stored = strings
        .iter()
        .position(|string| string.as_slice() == name.as_bytes());
    match (standard, stored) {
        (Some(sid), _) => u16::try_from(sid).ok(),
        (None, Some(at)) => u16::try_from(STANDARD_STRINGS.saturating_add(at)).ok(),
        (None, None) => {
            let sid = u16::try_from(STANDARD_STRINGS.saturating_add(strings.len())).ok();
            strings.push(name.as_bytes().to_vec());
            sid
        }
    }
}

/// Whether `rebuilt` reads, by `read-fonts`, as the program it was written to be: every
/// charstring byte for byte, and every added glyph under its name.
///
/// The caller's proof compares what the glyphs draw, and a charstring cut short by a mis-stated
/// offset can still draw what it drew — an empty glyph that has lost its `endchar` draws nothing
/// either way — so the INDEX is read back here before any caller sees the program.
fn read_back(
    rebuilt: &[u8],
    charstrings: &[Vec<u8>],
    added: &[AddedGlyph],
) -> Result<(), CffError> {
    let differs = || CffError::Malformed {
        detail: "the rebuilt program does not read back as it was written".to_owned(),
    };
    let font = open(rebuilt)?;
    let read = font.charstrings();
    if read.count() as usize != charstrings.len() {
        return Err(differs());
    }
    for (at, written) in charstrings.iter().enumerate() {
        if read.get(at) != Some(written.as_slice()) {
            return Err(differs());
        }
    }
    let charset = font.charset().ok_or(CffError::NoCharset)?;
    for glyph in added {
        let sid = charset
            .string_id(GlyphId::from(glyph.glyph))
            .ok_or_else(differs)?;
        if font.string(sid) != Some(glyph.name.as_bytes()) {
            return Err(differs());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The pieces a subsetter rebuilds a program from.
// ---------------------------------------------------------------------------------------------

/// A CFF program taken apart into the structures a subset is rebuilt from (Adobe Technical Note
/// #5176, section 2's order: header, Name INDEX, Top DICT INDEX, String INDEX, Global Subr INDEX,
/// then whatever the Top DICT's offsets locate).
///
/// Every piece borrows from the program, and nothing is interpreted beyond the INDEX and DICT
/// encodings; `pdf_font::embed`'s CFF writer decides what of it is kept.
pub(crate) struct Parts<'a> {
    /// The Top DICT's entries, as (operand bytes, operator).
    pub(crate) top: Vec<(Vec<u8>, u16)>,
    /// The String INDEX's items, SID 391 first.
    pub(crate) strings: Vec<&'a [u8]>,
    /// The Global Subr INDEX's items.
    pub(crate) global_subrs: Vec<&'a [u8]>,
    /// The `CharStrings` INDEX's items, by glyph index.
    pub(crate) charstrings: Vec<&'a [u8]>,
    /// Whether the Top DICT uses `CIDFont` operators.
    pub(crate) cid_keyed: bool,
    /// The Font DICTs: a CID-keyed program's `FDArray`, or for a name-keyed one a single entry
    /// holding the Top DICT's Private DICT and no Font DICT entries of its own.
    pub(crate) font_dicts: Vec<FontDictParts<'a>>,
    /// The program, opened, for `FDSelect`.
    font: CffFontRef<'a>,
}

/// One Font DICT of [`Parts`], with the Private DICT and local subroutines it names.
pub(crate) struct FontDictParts<'a> {
    /// The Font DICT's own entries; empty for a name-keyed program's one.
    pub(crate) entries: Vec<(Vec<u8>, u16)>,
    /// The Private DICT's entries.
    pub(crate) private: Vec<(Vec<u8>, u16)>,
    /// The Local Subr INDEX's items, which the Private DICT's `Subrs` locates.
    pub(crate) subrs: Vec<&'a [u8]>,
}

impl<'a> Parts<'a> {
    /// Takes `data` apart, or `None` where any of the structures does not read.
    pub(crate) fn read(data: &'a [u8]) -> Option<Self> {
        let header = usize::from(*data.get(2)?);
        let name_index = index_extent(data, header)?;
        let top_index = index_extent(data, name_index.end)?;
        let top_bytes = index_item(data, name_index.end, 0)?;
        let top = dict_entries(top_bytes)?;
        if misstated_entry(data, &top).is_some() {
            return None;
        }
        let string_index = index_extent(data, top_index.end)?;
        let strings = index_items(data, top_index.end)?;
        let global_subrs = index_items(data, string_index.end)?;
        let offset = |wanted: u16| {
            top.iter()
                .find(|(_, op)| *op == wanted)
                .and_then(|(operands, _)| dict_int(operands))
                .and_then(|at| usize::try_from(at).ok())
        };
        let charstrings = index_items(data, offset(OP_CHARSTRINGS)?)?;
        let font = open(data).ok()?;
        let cid_keyed = font.is_cid();
        let font_dicts = if cid_keyed {
            index_items(data, offset(OP_FD_ARRAY)?)?
                .into_iter()
                .map(|dict| FontDictParts::read(data, dict, true))
                .collect::<Option<Vec<_>>>()?
        } else {
            vec![FontDictParts::read(data, top_bytes, false)?]
        };
        Some(Self {
            top,
            strings,
            global_subrs,
            charstrings,
            cid_keyed,
            font_dicts,
            font,
        })
    }

    /// The index into [`Self::font_dicts`] a glyph's charstring is read against: `FDSelect`'s
    /// answer for a CID-keyed program, the one entry for a name-keyed one.
    pub(crate) fn font_dict_of(&self, glyph: u16) -> Option<usize> {
        if !self.cid_keyed {
            return Some(0);
        }
        self.font
            .subfont_index(GlyphId::from(glyph))
            .map(usize::from)
    }
}

impl<'a> FontDictParts<'a> {
    /// A DICT's `Private` entry followed to its Private DICT and that DICT's `Subrs`, the
    /// DICT's own entries kept where `own` (a Font DICT) and not where it is a Top DICT.
    fn read(data: &'a [u8], dict: &[u8], own: bool) -> Option<Self> {
        let entries = dict_entries(dict)?;
        let located = entries
            .iter()
            .find(|(_, op)| *op == OP_PRIVATE)
            .and_then(|(operands, _)| dict_two_ints(operands));
        let (private, subrs) = match located {
            Some((size, at)) => {
                let at = usize::try_from(at).ok()?;
                let bytes = data.get(at..at.checked_add(usize::try_from(size).ok()?)?)?;
                let private = dict_entries(bytes)?;
                let relative = private
                    .iter()
                    .find(|(_, op)| *op == OP_SUBRS)
                    .and_then(|(operands, _)| dict_int(operands));
                let subrs = match relative {
                    Some(relative) => {
                        index_items(data, at.checked_add(usize::try_from(relative).ok()?)?)?
                    }
                    None => Vec::new(),
                };
                (private, subrs)
            }
            None => (Vec::new(), Vec::new()),
        };
        Some(Self {
            entries: if own { entries } else { Vec::new() },
            private,
            subrs,
        })
    }
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
    pub(super) fn index(items: &[&[u8]]) -> Vec<u8> {
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
    pub(super) fn cid_keyed(
        glyph: &[u8],
        gsubr: Option<&[u8]>,
        fd_array_readable: bool,
    ) -> Vec<u8> {
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

#[cfg(test)]
mod empty_glyphs {
    use super::*;

    /// Every point a pen is handed, in order, which is what "the outline is untouched" means.
    #[derive(Default, PartialEq, Debug)]
    struct Points(Vec<(u8, i64, i64)>);
    impl Points {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a test's comparison key: coordinates scaled and rounded to integers"
        )]
        fn push(&mut self, kind: u8, x: f32, y: f32) {
            self.0
                .push((kind, (x * 64.0).round() as i64, (y * 64.0).round() as i64));
        }
    }
    impl OutlinePen for Points {
        fn move_to(&mut self, x: f32, y: f32) {
            self.push(0, x, y);
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.push(1, x, y);
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            self.push(2, cx, cy);
            self.push(2, x, y);
        }
        fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
            self.push(3, ax, ay);
            self.push(3, bx, by);
            self.push(3, x, y);
        }
        fn close(&mut self) {
            self.push(4, 0.0, 0.0);
        }
    }

    /// The glyph names a program's charset gives, by glyph index.
    fn names(data: &[u8]) -> Vec<String> {
        let font = open(data).expect("opens");
        font.charset()
            .expect("a charset")
            .iter()
            .map(|(_, sid)| {
                String::from_utf8_lossy(font.string(sid).expect("a string")).into_owned()
            })
            .collect()
    }

    /// Each compiled-in name-keyed face gains its empty glyphs, and keeps every glyph it had.
    #[test]
    fn an_empty_glyph_is_appended_and_nothing_else_moves() {
        let faces: [&[u8]; 4] = [
            include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb"),
            include_bytes!("../../../data/standard-fonts/FoxitFixedBold.pfb"),
            include_bytes!("../../../data/standard-fonts/FoxitSymbol.pfb"),
            include_bytes!("../../../data/standard-fonts/FoxitDingbats.pfb"),
        ];
        for face in faces {
            let (rebuilt, added) = with_empty_glyphs(face, &[250, 0, 778]).expect("rebuilt");
            let before = glyph_count(face).expect("counts");
            assert_eq!(glyph_count(&rebuilt), Ok(before + 3));
            let held = u16::try_from(before).expect("a glyph count");
            assert_eq!(
                added,
                ["notdef", "notdef.1", "notdef.2"]
                    .iter()
                    .zip(held..)
                    .map(|(name, glyph)| AddedGlyph {
                        glyph,
                        name: (*name).to_owned()
                    })
                    .collect::<Vec<_>>()
            );
            let (old, new) = (names(face), names(&rebuilt));
            assert_eq!(new.get(..old.len()), Some(old.as_slice()));
            assert_eq!(new.get(old.len()..).map(<[String]>::len), Some(3));
            let glyphs: Vec<u16> = added.iter().map(|added| added.glyph).collect();
            let stated = advances(&rebuilt, &glyphs).expect("advances");
            let wanted = vec![Some(250.0), Some(0.0), Some(778.0)];
            assert_eq!(stated, wanted);
            for glyph in glyphs {
                let mut points = Points::default();
                draw(&rebuilt, glyph, &mut points).expect("draws");
                assert!(points.0.is_empty(), "an added glyph draws nothing");
            }
            let every: Vec<u16> = (0..held).collect();
            assert_eq!(
                advances(&rebuilt, &every).expect("advances"),
                advances(face, &every).expect("advances")
            );
            for glyph in every {
                let (mut was, mut is) = (Points::default(), Points::default());
                draw(face, glyph, &mut was).expect("draws");
                draw(&rebuilt, glyph, &mut is).expect("draws");
                assert_eq!(was, is, "glyph {glyph}'s outline");
            }
        }
    }

    /// `FoxitSerif` with its `.notdef` charstring replaced by glyph 34's, so that a copy of it has
    /// an outline to be compared: every shipped face's `.notdef` draws nothing.
    fn drawn_notdef() -> Vec<u8> {
        let face: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");
        let layout = Layout::read(face).expect("a Top DICT");
        let at = layout
            .entries
            .iter()
            .find(|(_, op)| *op == OP_CHARSTRINGS)
            .and_then(|(operands, _)| dict_int(operands))
            .and_then(|at| usize::try_from(at).ok())
            .expect("a CharStrings offset");
        let mut charstrings: Vec<Vec<u8>> = index_items(face, at)
            .expect("a CharStrings INDEX")
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect();
        charstrings[0] = charstrings[34].clone();
        let drawn = layout.with_charstrings(face, &charstrings);
        let mut points = Points::default();
        draw(&drawn, 0, &mut points).expect("draws");
        assert!(!points.0.is_empty(), "the replaced .notdef draws");
        drawn
    }

    /// A copy of `.notdef` draws `.notdef`'s outline at its own advance, a glyph given a standard
    /// name is charted under that name's own identifier, and every glyph held keeps its outline.
    #[test]
    fn a_copy_of_notdef_draws_its_outline_under_the_name_asked_for() {
        let face = &drawn_notdef();
        let asked = [
            GlyphToGive {
                draws: GivenOutline::NotdefsOwn,
                name: None,
                advance: 600,
            },
            GlyphToGive {
                draws: GivenOutline::NotdefsOwn,
                name: Some("uni2423".to_owned()),
                advance: 250,
            },
        ];
        let (rebuilt, added) = with_glyphs_given(face, &asked).expect("rebuilt");
        let held = u16::try_from(glyph_count(face).expect("counts")).expect("a glyph count");
        assert_eq!(
            added.iter().map(|added| added.glyph).collect::<Vec<_>>(),
            vec![held, held + 1]
        );
        let new = names(&rebuilt);
        assert_eq!(
            new.get(usize::from(held)..),
            Some(&["notdef".to_owned(), "uni2423".to_owned()][..])
        );
        assert_eq!(
            advances(&rebuilt, &[held, held + 1]).expect("advances"),
            vec![Some(600.0), Some(250.0)]
        );
        let mut notdef = Points::default();
        draw(face, 0, &mut notdef).expect("draws");
        for glyph in [held, held + 1] {
            let mut copy = Points::default();
            draw(&rebuilt, glyph, &mut copy).expect("draws");
            assert_eq!(copy, notdef, "glyph {glyph} is .notdef's outline");
        }
        for glyph in 0..held {
            let (mut was, mut is) = (Points::default(), Points::default());
            draw(face, glyph, &mut was).expect("draws");
            draw(&rebuilt, glyph, &mut is).expect("draws");
            assert_eq!(was, is, "glyph {glyph}'s outline");
        }
    }

    /// A name the program holds is refused rather than given a second glyph, and a request past
    /// [`MOST_GLYPHS_GIVEN`] is refused before any table is grown; one at the bound is written.
    #[test]
    fn a_held_name_and_a_request_past_the_bound_are_refused() {
        let face: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");
        let held_name = names(face).get(1).cloned().expect("a second glyph");
        let held = GlyphToGive {
            draws: GivenOutline::Nothing,
            name: Some(held_name),
            advance: 0,
        };
        assert!(matches!(
            with_glyphs_given(face, &[held]),
            Err(CffError::Malformed { .. })
        ));
        let one = GlyphToGive {
            draws: GivenOutline::Nothing,
            name: None,
            advance: 500,
        };
        let at_the_bound = vec![one.clone(); MOST_GLYPHS_GIVEN];
        assert!(with_glyphs_given(face, &at_the_bound).is_ok());
        let past = vec![one; MOST_GLYPHS_GIVEN + 1];
        assert!(matches!(
            with_glyphs_given(face, &past),
            Err(CffError::Malformed { .. })
        ));
    }
}
