//! An embedded program written again as this reader read it (ADR 3569).
//!
//! ISO 32000-2 §9.9 embeds a font program "as data contained in a PDF stream object", in the
//! format Table 124 files it under. Where the program departs from its format in a way this
//! reader repairs to draw it — a `loca` table rewritten (`sfnt-loca-*`), a table directory
//! naming a table past the program's last byte, a collection cut to one face, a CID-keyed CFF's
//! unreadable Font DICTs replaced (ADR 3565's program classes) — the bytes the window drew from
//! are not the bytes the file holds. [`program_as_read`] hands over the bytes it drew from, laid
//! out as a program of the format they are, and proves before handing them over that they are
//! the same program:
//!
//! - **they read back with no repair**: no `loca` to rewrite, no table past the end, no
//!   collection, no Font DICT that does not read;
//! - **every glyph draws the same outline at the same advance** as it drew from the repaired
//!   bytes — every point bit for bit, through the reader the window draws with, and for an sfnt
//!   the `cmap` table that selects the glyphs byte for byte.
//!
//! For the `loca` repairs, the collection and the Font DICTs the written bytes *are* the ones the
//! window drew from, so the comparison holds by construction and the read-back is what proves
//! them; for a table past the program's end the directory is laid out again without the records
//! whose bytes the file does not hold, and the comparison is what proves that no glyph drew from
//! one of them.

use pdf_syntax::{Dictionary, Document};
use skrifa::MetadataProvider as _;
use skrifa::outline::OutlinePen;
use skrifa::raw::TableProvider as _;

use crate::cff;
use crate::program::{Program, embedded_program};
use crate::sfnt;

/// How §9.9's Table 124 files a program written as read: the key and, for `/FontFile3`, the
/// `/Subtype`, decided by the bytes as the loader decides which reader reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filing {
    /// A `TrueType` program, under `/FontFile2`.
    TrueType,
    /// An `OpenType` program, under `/FontFile3` of `/Subtype /OpenType`.
    OpenType,
    /// A name-keyed bare CFF program, under `/FontFile3` of `/Subtype /Type1C`.
    Type1C,
    /// A CID-keyed bare CFF program, under `/FontFile3` of `/Subtype /CIDFontType0C`.
    CidFontType0C,
}

impl Filing {
    /// The descriptor key Table 124 files this format under.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::TrueType => "FontFile2",
            Self::OpenType | Self::Type1C | Self::CidFontType0C => "FontFile3",
        }
    }

    /// The `/Subtype` a `/FontFile3` stream states for this format, `None` for `/FontFile2`.
    #[must_use]
    pub const fn subtype(self) -> Option<&'static str> {
        match self {
            Self::TrueType => None,
            Self::OpenType => Some("OpenType"),
            Self::Type1C => Some("Type1C"),
            Self::CidFontType0C => Some("CIDFontType0C"),
        }
    }
}

/// A program as this reader read it, proved to draw what it drew.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsRead {
    /// The program's bytes.
    pub data: Vec<u8>,
    /// How Table 124 files them.
    pub filing: Filing,
    /// How many glyphs were compared, which is every glyph the program holds.
    pub glyphs: u16,
}

/// Why a program is not written as read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NotAsRead {
    /// The loader refuses the program, so there is nothing it read to write.
    #[error("the program does not load: {0}")]
    Unreadable(String),
    /// A bare Type 1 program, which none of the program repairs reads differently from its bytes.
    #[error("a Type 1 program, which this reader reads as written")]
    TypeOne,
    /// The table directory repeats a tag, so which bytes were read is not one answer.
    #[error("the table directory names one table twice")]
    DirectoryAmbiguous,
    /// The bytes laid out again still need a repair to be read.
    #[error("the program written again still needs a repair to be read")]
    StillRepaired,
    /// A glyph draws another outline or advance from the written bytes, or the glyphs are
    /// selected through another `cmap`.
    #[error("glyph {glyph} does not draw from the written program what it drew")]
    NotProved {
        /// The first glyph that disagrees; 0 also stands for the `cmap`.
        glyph: u16,
    },
}

/// The embedded program `descriptor` names, written as this reader read it (see the module
/// comment), or why it cannot be.
///
/// # Errors
///
/// See [`NotAsRead`].
pub fn program_as_read(
    document: &Document,
    descriptor: &Dictionary,
    name: &str,
) -> Result<AsRead, NotAsRead> {
    let embedded = embedded_program(document, descriptor, name)
        .map_err(|refusal| NotAsRead::Unreadable(refusal.to_string()))?;
    match embedded.program {
        Program::Type1 => Err(NotAsRead::TypeOne),
        Program::BareCff => cff_as_read(&embedded.data),
        Program::Sfnt => sfnt_as_read(&embedded.data),
    }
}

/// A bare CFF program with its Font DICTs made readable, which is what the loader draws from
/// (`cff::readable_font_dicts`).
///
/// The bytes written are the bytes the window drew from, so every glyph draws its outline at its
/// charstring width from them by construction; what is proved is that they read back with no
/// Font DICT left to replace, and every glyph is still drawn, or refused, by the same reader.
fn cff_as_read(data: &[u8]) -> Result<AsRead, NotAsRead> {
    let unreadable = |_| NotAsRead::Unreadable("the CFF program does not open".to_owned());
    let read = cff::readable_font_dicts(data)
        .map_err(unreadable)?
        .map_or_else(|| data.to_vec(), |repair| repair.data);
    if cff::readable_font_dicts(&read)
        .map_err(unreadable)?
        .is_some()
    {
        return Err(NotAsRead::StillRepaired);
    }
    let glyphs = u16::try_from(cff::glyph_count(&read).map_err(unreadable)?)
        .map_err(|_| NotAsRead::Unreadable("more glyphs than a CFF can index".to_owned()))?;
    let filing = if cff::uses_cid_operators(&read).map_err(unreadable)? {
        Filing::CidFontType0C
    } else {
        Filing::Type1C
    };
    Ok(AsRead {
        data: read,
        filing,
        glyphs,
    })
}

/// An sfnt with every table record whose bytes run past the program's end left out of a
/// directory laid out again, proved against the bytes the loader drew from.
fn sfnt_as_read(drawn: &[u8]) -> Result<AsRead, NotAsRead> {
    let written = sfnt::without_records_past_the_end(drawn).ok_or(NotAsRead::DirectoryAmbiguous)?;
    let (_, still) = sfnt::repaired_font_program_noting(&written);
    if !still.is_empty()
        || sfnt::table_past_the_end(&written)
        || crate::collection::extract(&written, None).is_some()
    {
        return Err(NotAsRead::StillRepaired);
    }
    let unreadable = || NotAsRead::Unreadable("the sfnt program does not open".to_owned());
    let before = skrifa::FontRef::new(drawn).map_err(|_| unreadable())?;
    let after = skrifa::FontRef::new(&written).map_err(|_| unreadable())?;
    let cmap = skrifa::raw::types::Tag::new(b"cmap");
    let table =
        |font: &skrifa::FontRef<'_>| font.table_data(cmap).map(|data| data.as_bytes().to_vec());
    if table(&before) != table(&after) {
        return Err(NotAsRead::NotProved { glyph: 0 });
    }
    let glyphs = before.maxp().map_err(|_| unreadable())?.num_glyphs();
    let metrics = |font: &skrifa::FontRef<'_>| {
        let metrics = font.glyph_metrics(
            skrifa::instance::Size::unscaled(),
            skrifa::instance::LocationRef::default(),
        );
        (0..glyphs)
            .map(|glyph| metrics.advance_width(skrifa::GlyphId::from(glyph)))
            .collect::<Vec<_>>()
    };
    let (advances_before, advances_after) = (metrics(&before), metrics(&after));
    for glyph in 0..glyphs {
        let index = usize::from(glyph);
        if advances_before
            .get(index)
            .map(|advance| advance.map(f32::to_bits))
            != advances_after
                .get(index)
                .map(|advance| advance.map(f32::to_bits))
            || sfnt_points(&before, glyph) != sfnt_points(&after, glyph)
        {
            return Err(NotAsRead::NotProved { glyph });
        }
    }
    let filing = if written.starts_with(b"OTTO")
        || after
            .table_data(skrifa::raw::types::Tag::new(b"CFF "))
            .is_some()
    {
        Filing::OpenType
    } else {
        Filing::TrueType
    };
    Ok(AsRead {
        data: written,
        filing,
        glyphs,
    })
}

/// Whether every glyph of `after` draws point for point, bit for bit, as the same glyph of
/// `before` does, unhinted in the program's own units through `skrifa` — the outline half of
/// [`sfnt_as_read`]'s proof, for a program whose metrics were restated (ADR 3642). `Err` names the
/// first glyph that does not, zero where either program does not open.
pub(crate) fn sfnt_draws_alike(before: &[u8], after: &[u8]) -> Result<(), u16> {
    let before = skrifa::FontRef::new(before).map_err(|_| 0_u16)?;
    let after = skrifa::FontRef::new(after).map_err(|_| 0_u16)?;
    let glyphs = before.maxp().map_err(|_| 0_u16)?.num_glyphs();
    if after.maxp().map_err(|_| 0_u16)?.num_glyphs() != glyphs {
        return Err(0);
    }
    (0..glyphs)
        .find(|glyph| sfnt_points(&before, *glyph) != sfnt_points(&after, *glyph))
        .map_or(Ok(()), Err)
}

/// Every point and command of a glyph's outline as its bits, so that two readings compare exactly.
#[derive(Debug, Default, PartialEq, Eq)]
struct Points(Vec<(u8, u32, u32)>);

impl Points {
    fn push(&mut self, verb: u8, x: f32, y: f32) {
        self.0.push((verb, x.to_bits(), y.to_bits()));
    }
}

impl OutlinePen for Points {
    fn move_to(&mut self, x: f32, y: f32) {
        self.push(b'M', x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push(b'L', x, y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.push(b'q', cx, cy);
        self.push(b'Q', x, y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.push(b'c', cx0, cy0);
        self.push(b'c', cx1, cy1);
        self.push(b'C', x, y);
    }
    fn close(&mut self) {
        self.push(b'Z', 0.0, 0.0);
    }
}

/// One glyph of an sfnt unhinted in its own units, `None` where the reader draws nothing.
fn sfnt_points(font: &skrifa::FontRef<'_>, glyph: u16) -> Option<Points> {
    let mut points = Points::default();
    // not a repair: the proof's own drawing; a glyph that does not draw ends the proof (`None`), it reads no file value leniently.
    font.outline_glyphs()
        .get(skrifa::GlyphId::from(glyph))?
        .draw(skrifa::instance::Size::unscaled(), &mut points)
        .ok()?;
    Some(points)
}

#[cfg(test)]
mod tests {
    use super::{Filing, sfnt_as_read};
    use crate::sfnt;

    /// A shipped face whose `post` record is moved past the program's end: the written program
    /// lacks that record, reads back with no repair, and keeps every glyph and the `cmap`.
    #[test]
    fn a_record_past_the_end_is_left_out_and_every_glyph_kept() {
        let face = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");
        let mut cut = face.to_vec();
        let count = usize::from(sfnt::be16(&cut, 4).expect("a directory"));
        let post = (0..count)
            .map(|index| 12 + index * 16)
            .find(|at| cut.get(*at..*at + 4) == Some(b"post".as_slice()))
            .expect("the face states a post table");
        cut[post + 8..post + 12].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(sfnt::table_past_the_end(&cut));
        let read = sfnt_as_read(&cut).expect("the program is written as read");
        assert_eq!(read.filing, Filing::TrueType);
        assert!(!sfnt::table_past_the_end(&read.data));
        assert!(sfnt::table(&read.data, *b"post").is_none());
        assert!(sfnt::table(&read.data, *b"cmap").is_some());
        assert!(read.glyphs > 0);
    }

    /// A program that needs no repair is written as it is.
    #[test]
    fn a_whole_program_is_written_unchanged() {
        let face = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");
        let read = sfnt_as_read(face).expect("a whole program reads");
        assert_eq!(read.data, face.to_vec());
    }
}
