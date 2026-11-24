//! A rendered font's embedded program that this reader repaired to draw, written again as the
//! program it read (ADR 3569).
//!
//! `fonts/font-programs-embedded` (ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section
//! 6.2.10.4.1) fails such a font through the reader's log: its `loca` was rewritten, its table
//! directory names a table past the program's end, it is a collection cut to one face, or its
//! CID-keyed CFF's Font DICTs were replaced (`pdf_archive::program_read_repaired`, the row's own
//! reading). The program is there and the window drew from it, so the face routes that answer a
//! font with no program do not apply: [`pdf_font::as_read::program_as_read`] hands over the bytes
//! the window drew from, laid out as a program of their format and proved to draw every glyph's
//! outline at its advance as it drew, and they are written at the program's own object under the
//! producer's dictionary, refiled by Table 124 where the bytes are another format than the
//! `/Subtype` stated. Nothing a reader sees changes and nothing is invented, so no loss word is
//! asked, as for a program written whole again (ADR 3256).
//!
//! Refused by name ([`NOT_WRITTEN_AS_READ`]): a program the proof does not hold for, one the
//! loader refuses, and one whose bytes are filed under a key their format does not take.

use pdf_archive::survey::SelectedFont;
use pdf_font::as_read::{AsRead, Filing, NotAsRead, program_as_read};
use pdf_font::substitute::Format;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use super::super::COMPRESSION_LEVEL;
use super::super::decision::Because;
use super::Shortfall;

/// Why a repaired program is not written as read.
pub(super) const NOT_WRITTEN_AS_READ: &str = "a font a content stream renders embeds a program \
     this reader repaired to draw — a loca table rewritten, a table directory naming a table past \
     the program's end, a collection cut to one face, or a CID-keyed CFF's Font DICTs replaced — \
     so the program the file embeds is not the program drawn, which ISO 19005-2 section 6.2.11.4.1 \
     asks of every rendered font. This conversion writes the program as it was read only where \
     the bytes read back with no repair and every glyph draws the outline and the advance it drew \
     from them, under the key ISO 32000-2 \u{a7}9.9's Table 124 files their format under; this \
     program is not that";

/// What [`super::cut_program`] answers for a rendered font whose program is neither cut nor
/// unparsed: [`Shortfall::None`] where the reader repaired nothing in it, the program written as
/// read, or [`Shortfall::NotAsRead`].
pub(super) fn shortfall(
    document: &Document,
    used: &SelectedFont,
    descriptor: &Dictionary,
) -> Shortfall {
    let Some((key, _)) = pdf_archive::program_read_repaired(document, descriptor) else {
        return Shortfall::None;
    };
    let Some(at) = descriptor.get(key).and_then(Object::as_reference) else {
        return Shortfall::NotAsRead;
    };
    match program_as_read(document, descriptor, &used.name) {
        Ok(read) if read.filing.key() == key => written(document, at, &read)
            .map_or(Shortfall::NotAsRead, |stream| {
                Shortfall::WrittenAsRead(at, stream)
            }),
        Ok(_) | Err(_) => Shortfall::NotAsRead,
    }
}

/// The program a rendered font's descriptor names as this reader read it, where the reader
/// repaired it to draw and it can be written so; `None` where nothing was repaired. A program the
/// proof refuses is `Some` of the refusal, so the caller refuses rather than restating the bytes
/// the file holds: those are not what was drawn.
fn restatable(document: &Document, used: &SelectedFont) -> Option<Result<AsRead, NotAsRead>> {
    let descriptor = super::descriptor_dict(document, used)?;
    pdf_archive::program_read_repaired(document, &descriptor)?;
    Some(program_as_read(document, &descriptor, &used.name))
}

/// The bytes the metrics route restates advances in: a program this reader repaired to draw has
/// them restated in the bytes it drew, so that the program written is the one read as well as the
/// one whose advances agree (ADR 3569); any other, the file's own `bytes`.
///
/// # Errors
///
/// [`NOT_WRITTEN_AS_READ`] where the program was repaired and cannot be written as read in the
/// format it is filed under, since restating the file's bytes would keep the repair.
pub(super) fn to_restate(
    document: &Document,
    used: &SelectedFont,
    filed: super::Filed,
    bytes: std::sync::Arc<[u8]>,
) -> Result<std::sync::Arc<[u8]>, Because> {
    match (restatable(document, used), filed) {
        (None, _) => Ok(bytes),
        (Some(Ok(read)), super::Filed::Read(format)) if read_as(read.filing, format) => {
            Ok(std::sync::Arc::from(read.data))
        }
        (Some(_), _) => Err(Because::NotBuiltYet(NOT_WRITTEN_AS_READ)),
    }
}

/// Whether a program filed as `filing` is read by the reader `format` names, so that the metrics
/// route restates it in its own format.
const fn read_as(filing: Filing, format: Format) -> bool {
    matches!(
        (filing, format),
        (Filing::TrueType | Filing::OpenType, Format::Sfnt)
            | (Filing::Type1C | Filing::CidFontType0C, Format::BareCff)
    )
}

/// The program stream at `at` holding `read`: the producer's dictionary, one `FlateDecode`, the
/// extent Table 125 states for a `/FontFile2` and the `/Subtype` Table 124 names for a
/// `/FontFile3`.
fn written(document: &Document, at: ObjectId, read: &AsRead) -> Option<Object> {
    let Object::Stream(stream) = document.get(at) else {
        return None;
    };
    // The data is decrypted when read, so the stream written holds it in the clear, as every
    // stream this conversion writes does.
    let encoded = pdf_syntax::serialize::flate_encode(&read.data, COMPRESSION_LEVEL)?;
    let mut dict = stream.dict.clone();
    dict.remove("DecodeParms");
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(encoded.len()).ok()?),
    );
    match read.filing.subtype() {
        Some(subtype) => {
            dict.insert(
                Name::new(&b"Subtype"[..]),
                Object::Name(Name::new(subtype.as_bytes())),
            );
        }
        None => {
            dict.insert(
                Name::new(&b"Length1"[..]),
                Object::Integer(i64::try_from(read.data.len()).ok()?),
            );
        }
    }
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}
