//! Which reader understands the embedded font program.
//!
//! ISO 32000-2 §9.9's Table 124 gives a font descriptor three doors — `/FontFile`,
//! `/FontFile2` and `/FontFile3` — and what lies behind each is decided here by the bytes'
//! own signature rather than by the key's spelling or by the `/Subtype` name, because a
//! producer can mislabel a stream and cannot mislabel a leading `OTTO`.
//!
//! The em square each reader states comes out of the same decision, which is why it is
//! answered here too: an sfnt keeps it in `head`, a bare CFF and a Type 1 program in a
//! `/FontMatrix`, and the three cannot be asked the same way.

use std::borrow::Cow;
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document};
use skrifa::prelude::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};

use crate::cff;
use crate::collection;
use crate::loading::FontError;
use crate::sfnt::{repaired_font_program, truncation};
use crate::substitute;
use crate::type1;

/// Which reader extracts outlines from the embedded font program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Program {
    /// An sfnt container — `TrueType` or `OpenType` — read through `skrifa`'s `FontRef`.
    Sfnt,
    /// A bare Type 1 program — §9.9's `/FontFile` — read through `read-fonts`' Type 1
    /// reader. Name-keyed exactly as a bare CFF is, which is why both reach
    /// [`crate::name_keyed::simple_code_table`] with the same value; see [`crate::name_keyed`].
    Type1,
    /// A bare CFF program, read through `read-fonts`' CFF reader directly.
    ///
    /// Wrapping it in a synthesised sfnt so that `FontRef` would accept it is possible,
    /// but pointless: the CFF reader draws from the bare program, and a synthesised
    /// container would be one more thing to get right for no gain.
    BareCff,
}

impl From<substitute::Format> for Program {
    /// A substitute's format, as this crate's own reader selection.
    ///
    /// The two enumerations exist separately because [`substitute::Format`] is what
    /// [`substitute::find`] can produce — an `sfnt` from the machine or from the compiled-in
    /// Liberation faces, or a bare **CFF** from the compiled-in Foxit ones — while [`Program`]
    /// also names a bare Type 1 program, which only an *embedded* `/FontFile` can be.
    ///
    /// This comment said "bare Type 1" for the Foxit faces, which are `Format::BareCff` and
    /// always were; the four-hundred-and-fifth session corrected it while reading this path
    /// for `program_widths`, where the same confusion had a cost rather than a spelling.
    fn from(format: substitute::Format) -> Self {
        match format {
            substitute::Format::Sfnt => Self::Sfnt,
            substitute::Format::BareCff => Self::BareCff,
        }
    }
}

/// An embedded font program and the reader that understands it.
pub(crate) struct Embedded {
    pub(crate) data: Arc<[u8]>,
    pub(crate) program: Program,
}

/// Extracts the embedded font program from a font descriptor.
pub(crate) fn embedded_program(
    document: &Document,
    descriptor: &Dictionary,
    name: &str,
) -> Result<Embedded, FontError> {
    // `/FontFile2` is TrueType and `/FontFile3` is CFF or OpenType, both of which skrifa
    // reads. `/FontFile` is bare Type1, which it does not.
    for key in ["FontFile2", "FontFile3"] {
        let object = document.get_key(descriptor, key);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Ok(decoded) = document.decoded_stream_data_reported(stream) else {
            return Err(FontError::Malformed {
                name: name.to_owned(),
                detail: format!("/{key} did not decode"),
            });
        };
        // **A prefix of a font program is not a shorter font program**, which is the line
        // ADR 0343 draws between this and a content stream. §7.8.2 makes a content stream "a
        // sequence of instructions", so a prefix of one is a shorter sequence of the same kind
        // and every instruction in it is the producer's own. A font program is a *structure*:
        // §9.9's `/FontFile2` and `/FontFile3` hold a table directory whose offsets point
        // forward, so a prefix is a directory describing bytes that are not there, and reading
        // one produces glyphs the producer never wrote rather than fewer of the ones it did.
        //
        // The witness is `issue13316_reduced.pdf`, whose `/FontFile2` is corrupt: read as a
        // whole program its 863 surviving bytes draw **A C E F** where the file's six CJK
        // glyphs belong. Trap 5's own test decides it — the marks a refusal gives up here are
        // substitutive rather than additive (ADR 0106), because the wrong glyphs stand *in
        // place of* the right ones instead of beside them.
        //
        // `truncation` below is the same rule read off the structure, and kept: it catches a
        // program whose stream decoded whole and whose directory still overruns it.
        if let Some(damage) = decoded.damage {
            return Err(FontError::Malformed {
                name: name.to_owned(),
                detail: format!(
                    "/{key} decoded only as far as its damage ({damage:?}, {} bytes): a prefix \
                     of a font program is a directory describing bytes that are not there",
                    decoded.data.len()
                ),
            });
        }
        let data = decoded.data;

        // `/FontFile3` holds either a full OpenType file or a *bare* CFF font program.
        // Its `/Subtype` says which — `Type1C` and `CIDFontType0C` for a bare CFF — but
        // the leading bytes say the same thing and cannot be mislabelled by a producer,
        // so the signature decides and the `/Subtype` is not consulted at all.
        let program = if is_bare_cff(&data) {
            Program::BareCff
        } else {
            Program::Sfnt
        };

        // A collection is not a font program (§9.9 Table 124) and four corpus documents
        // embed one anyway. The face is chosen by the descriptor's own `/FontName` and
        // copied out before anything else looks at the bytes, so nothing downstream has to
        // know the container existed.
        let data = if program == Program::Sfnt {
            let wanted = document.get_key(descriptor, "FontName");
            let wanted = wanted
                .as_name()
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned());
            collection::extract(&data, wanted.as_deref()).map_or(data, Arc::from)
        } else {
            data
        };

        let data = if program == Program::Sfnt {
            match repaired_font_program(&data) {
                Cow::Borrowed(_) => data,
                Cow::Owned(repaired) => Arc::from(repaired),
            }
        } else {
            data
        };

        if program == Program::Sfnt
            && let Some((table, end)) = truncation(&data)
        {
            return Err(FontError::Malformed {
                name: name.to_owned(),
                detail: format!(
                    "the font program is truncated: its \"{table}\" table ends at byte {end} \
                     and the stream holds {}",
                    data.len()
                ),
            });
        }

        return Ok(Embedded { data, program });
    }

    // `/FontFile` is a bare Type 1 program. Read last, because Table 120 says "At most,
    // only one of the FontFile , FontFile2 , and FontFile3 entries shall be present" and a
    // file writing two has said nothing about which it means — preferring the formats with
    // a self-identifying signature keeps that choice from turning on the key's spelling.
    if let Some(stream) = document.get_key(descriptor, "FontFile").as_stream() {
        let decoded =
            document
                .decoded_stream_data_reported(stream)
                .map_err(|_| FontError::Malformed {
                    name: name.to_owned(),
                    detail: "/FontFile did not decode".to_owned(),
                })?;
        // Type 1's own structure is a sequence of PostScript definitions rather than a table
        // directory, but its eexec-encrypted private portion is one blob with a checksum, so a
        // prefix is no more readable than a truncated sfnt is. Same refusal, same reason.
        if let Some(damage) = decoded.damage {
            return Err(FontError::Malformed {
                name: name.to_owned(),
                detail: format!(
                    "/FontFile decoded only as far as its damage ({damage:?}, {} bytes)",
                    decoded.data.len()
                ),
            });
        }
        return Ok(Embedded {
            data: decoded.data,
            program: Program::Type1,
        });
    }
    Err(FontError::NotEmbedded {
        name: name.to_owned(),
    })
}

/// Returns `true` for a bare CFF font program.
///
/// A CFF file starts with a header whose first two bytes are its major and minor version,
/// conventionally 1 and 0. An sfnt file starts with a recognisable tag instead — `0x00010000`,
/// `OTTO`, `true` or `ttcf` — so a leading `01 00` that is none of those is CFF.
fn is_bare_cff(data: &[u8]) -> bool {
    match data.get(..4) {
        // The four sfnt container signatures.
        Some([0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"true" | b"ttcf") => false,
        // A CFF header begins with its major and minor version.
        Some([1, 0, ..]) => true,
        _ => false,
    }
}

/// Parses a bare Type 1 program, for the one kind of program that is kept parsed.
///
/// Done once at load rather than in `build_outline`, because the units per em, the code
/// mapping and every outline come out of the same parse and that parse is the expensive
/// one; see [`type1::Program`].
/// The em square of a simple font's program, from whichever reader parsed it.
///
/// A parsed Type 1 program answers from its own `/FontMatrix` and everything else from the
/// program's header; the two cannot be asked the same way, which is the whole of why this exists
/// as a function rather than as two lines inside [`crate::loading::LoadedFont::load_simple`].
///
/// # Errors
///
/// [`FontError::Malformed`] where the program states an em square that cannot be read.
pub(crate) fn simple_units_per_em(
    type1: Option<&type1::Program>,
    data: &[u8],
    program: Program,
    name: &str,
) -> Result<f32, FontError> {
    match type1 {
        Some(parsed) => parsed.units_per_em().map_err(|e| FontError::Malformed {
            name: name.to_owned(),
            detail: e.to_string(),
        }),
        None => units_per_em(data, program, name),
    }
}

pub(crate) fn parsed_type1(
    program: Program,
    data: &[u8],
    name: &str,
) -> Result<Option<type1::Program>, FontError> {
    match program {
        Program::Type1 => type1::Program::parse(data)
            .map(Some)
            .map_err(|e| FontError::Malformed {
                name: name.to_owned(),
                detail: e.to_string(),
            }),
        Program::Sfnt | Program::BareCff => Ok(None),
    }
}

/// Reads a font's units per em, which every outline is scaled by.
fn units_per_em(data: &[u8], program: Program, name: &str) -> Result<f32, FontError> {
    if program == Program::BareCff {
        return cff::units_per_em(data).map_err(|e| FontError::Malformed {
            name: name.to_owned(),
            detail: e.to_string(),
        });
    }

    let font = FontRef::new(data).map_err(|e| FontError::Malformed {
        name: name.to_owned(),
        detail: e.to_string(),
    })?;
    let units = font
        .metrics(Size::unscaled(), LocationRef::default())
        .units_per_em;
    if units == 0 {
        return Err(FontError::Malformed {
            name: name.to_owned(),
            detail: "units per em is zero".to_owned(),
        });
    }
    Ok(f32::from(units))
}
