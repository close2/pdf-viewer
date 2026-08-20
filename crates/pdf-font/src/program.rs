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

        // **An sfnt with no `head` is not a broken font when its outlines are a `CFF ` table.**
        // §9.9's Table 124, for a `/FontFile3` whose `/Subtype` is `OpenType`, says which tables
        // such a program owes:
        //
        // > A Type1 font dictionary or CIDFontType0 CIDFont dictionary, if the embedded font
        // > program contains a "CFF " table without CIDFont operators. In addition to the "CFF "
        // > table, the font program shall include the "cmap" table.
        //
        // and says outright that the container's own list does not bind:
        //
        // > ISO/IEC 14496-22 describes a set of required tables; however, not all tables are
        // > required in the font file, as described for each type of font dictionary that can
        // > include this entry.
        //
        // So `head` may legitimately be absent — and `head` is where every sfnt reader looks for
        // the em square, which is why `units_per_em` answered such a program "units per em is
        // zero" and the page lost every glyph of it. The `CFF ` table is a whole font program
        // with its own `FontMatrix`, a charset and an encoding, so it is read as the bare CFF it
        // is and the rest of this crate needs to know nothing about the wrapper.
        //
        // The condition is the *absent table* rather than the container, deliberately: a program
        // that states a `head` states its scale, and its `cmap` and `hmtx` are what §9.6.5.4's
        // route reads, so it stays where it was.
        let (data, program) = match extracted_cff(program, &data) {
            Some(cff) => (Arc::from(cff), Program::BareCff),
            None => (data, program),
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
        // **The signature decides here too, and this door did not ask it.** This module's own
        // first paragraph says the reader is chosen "by the bytes' own signature rather than by
        // the key's spelling", and `/FontFile3` above asks; `/FontFile` assumed Type 1 from the
        // key alone until the five-hundred-and-fourteenth session, which is the rule stated in
        // one place and applied in two of three.
        //
        // `issue5751.pdf` is the witness the oracle supplied: a `CIDFontType0` whose descriptor
        // writes `/FontFile`, whose stream states only `/Length`, and whose first bytes are
        // `01 00 04 03` followed by the Name INDEX string `MyriadArabic-Regular` — a bare CFF.
        // Read as Type 1 it is `InvalidFontFormat` and the page draws nothing where four
        // reference renderers draw *Open Access*.
        //
        // Three statements the file makes disagree with the key, and the standard supplies all
        // three. Table 124 gives `/FontFile` "Type 1 font program, in the original (noncompact)
        // format" and says it "may appear in the font descriptor for a Type1 or MMType1 font
        // dictionary" — this is neither. Table 125 makes `/Length1`, `/Length2` and `/Length3`
        // "Required for Type 1 font programs" and the stream states none of them. And §9.7.4.2
        // puts a Type 0 CIDFont's CFF under `/FontFile3`. The bytes are the only one of the four
        // a producer cannot mislabel, so they are what is believed — the same choice, for the
        // same reason, that `/FontFile3` makes above by ignoring its `/Subtype`.
        //
        // Type 1's own formats cannot collide with it: a PFA begins `%!`, a PFB `80 01`, and
        // neither can begin `01 00`. So this reroutes a program that could not be read at all
        // and leaves every readable one where it was.
        let program = if is_bare_cff(&decoded.data) {
            Program::BareCff
        } else {
            Program::Type1
        };
        return Ok(Embedded {
            data: decoded.data,
            program,
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
/// The `CFF ` table of an sfnt that states no `head`, which is the program to read instead.
///
/// `None` for everything else, which is every ordinary font: a container that states a `head`
/// states its em square and is read through `skrifa` as before, and a program that is already
/// bare is already the CFF. See the call site for §9.9 Table 124's two sentences.
fn extracted_cff(program: Program, data: &[u8]) -> Option<Vec<u8>> {
    if program != Program::Sfnt {
        return None;
    }
    let tables = crate::sfnt::sfnt_tables(data)?;
    if tables.contains_key(b"head".as_slice()) {
        return None;
    }
    let &(at, length) = tables.get(b"CFF ".as_slice())?;
    Some(data.get(at..at.checked_add(length)?)?.to_vec())
}

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

#[cfg(test)]
mod font_file_signature {
    use pdf_syntax::{Document, ObjectId};

    use super::{Embedded, Program, embedded_program};

    /// A two-object document: object 1 is a font descriptor whose `key` is object 2.
    ///
    /// Table 120 gives the descriptor three keys for a program and this module chooses its
    /// reader by the bytes rather than by the key, so which one the fixture writes is a variable
    /// rather than a constant.
    fn descriptor_with(key: &[u8], program: &[u8]) -> Document {
        let mut out = Vec::from(*b"%PDF-1.7\n");
        let mut offsets = Vec::new();

        offsets.push(out.len());
        out.extend_from_slice(b"1 0 obj\n<< /Flags 4 /");
        out.extend_from_slice(key);
        out.extend_from_slice(b" 2 0 R >>\nendobj\n");

        offsets.push(out.len());
        out.extend_from_slice(
            format!("2 0 obj\n<< /Length {} >>\nstream\n", program.len()).as_bytes(),
        );
        out.extend_from_slice(program);
        out.extend_from_slice(b"\nendstream\nendobj\n");

        let xref_at = out.len();
        out.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
        for offset in &offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes(),
        );
        Document::open(out).expect("the fixture is a valid PDF")
    }

    fn program_of(bytes: &[u8]) -> Program {
        embedded_of(b"FontFile", bytes).program
    }

    fn embedded_of(key: &[u8], bytes: &[u8]) -> Embedded {
        let document = descriptor_with(key, bytes);
        let descriptor = document
            .get(ObjectId {
                number: 1,
                generation: 0,
            })
            .as_dict()
            .expect("object 1 is the descriptor")
            .clone();
        embedded_program(&document, &descriptor, "/F1").expect("the descriptor embeds a program")
    }

    /// `/FontFile` names a Type 1 program in Table 120 and Table 124, and this module chooses a
    /// reader by the bytes rather than by the key — so a stream whose header is a CFF's is read
    /// as one. `issue5751.pdf` is the corpus witness: a `CIDFontType0` whose descriptor writes
    /// `/FontFile`, whose stream states no `/Length1`, and whose first bytes are a CFF header.
    /// Read from the key it was `InvalidFontFormat` and the page drew nothing.
    #[test]
    fn a_font_file_whose_bytes_are_a_cff_is_read_as_one() {
        // A CFF header: major 1, minor 0, header size 4, offset size 3.
        assert_eq!(program_of(b"\x01\x00\x04\x03rest"), Program::BareCff);
    }

    /// The other side of the same rule: neither of Type 1's own two packagings can begin `01 00`,
    /// so believing the signature reroutes only a program that could not be read at all.
    #[test]
    fn a_font_file_that_really_is_type_1_is_untouched() {
        assert_eq!(program_of(b"%!PS-AdobeFont-1.0: Fixture\n"), Program::Type1);
        // The PFB segment header, which is the other way a Type 1 program arrives.
        assert_eq!(program_of(b"\x80\x01\x20\x00\x00\x00rest"), Program::Type1);
    }

    /// An `OTTO` container holding `tables`, in the order given, with a `head` only if named.
    ///
    /// The directory's search fields are left zero: nothing in this tree reads them, and a font
    /// this small has no binary search to describe.
    fn otto(tables: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let count = u16::try_from(tables.len()).expect("a fixture of a few tables");
        let mut out = Vec::from(*b"OTTO");
        out.extend_from_slice(&count.to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        let mut at = u32::try_from(12usize.saturating_add(16usize.saturating_mul(tables.len())))
            .expect("a fixture of a few tables");
        let mut body: Vec<u8> = Vec::new();
        for (tag, data) in tables {
            let length = u32::try_from(data.len()).expect("a fixture of small tables");
            out.extend_from_slice(*tag);
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&at.to_be_bytes());
            out.extend_from_slice(&length.to_be_bytes());
            body.extend_from_slice(data);
            // Every table begins on a four-byte boundary, so the padding is part of the offset.
            let padded = length.next_multiple_of(4);
            let pad = usize::try_from(padded.saturating_sub(length)).expect("at most three bytes");
            body.resize(body.len().saturating_add(pad), 0);
            at = at.saturating_add(padded);
        }
        out.extend_from_slice(&body);
        out
    }

    /// Table 124 requires a `CFF ` table and a `cmap`, and this file has exactly those.
    ///
    /// §9.9's Table 124, for a `/FontFile3` whose `/Subtype` is `OpenType`:
    ///
    /// > A Type1 font dictionary or CIDFontType0 CIDFont dictionary, if the embedded font program
    /// > contains a "CFF " table without CIDFont operators. In addition to the "CFF " table, the
    /// > font program shall include the "cmap" table.
    ///
    /// and, of the tables ISO/IEC 14496-22 would otherwise require:
    ///
    /// > ISO/IEC 14496-22 describes a set of required tables; however, not all tables are required
    /// > in the font file, as described for each type of font dictionary that can include this
    /// > entry.
    ///
    /// So `head` may be absent, and with it the em square every sfnt reader asks for. This tree
    /// refused two crawled documents with "units per em is zero" for exactly that (session 619):
    /// a Minion Pro subset under a `/Subtype /Type1` font dictionary, carrying `BASE`, `CFF `,
    /// `GPOS`, `GSUB`, `OS/2` and `cmap` and nothing else. The `CFF ` table is a whole font
    /// program with its own `FontMatrix`, so it is read as the bare CFF it is.
    #[test]
    fn an_opentype_program_with_no_head_is_read_as_the_cff_it_carries() {
        let cff: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");
        let wrapped = otto(&[(b"CFF ", cff), (b"cmap", &[0, 0, 0, 0])]);
        let embedded = embedded_of(b"FontFile3", &wrapped);
        assert_eq!(embedded.program, Program::BareCff);
        assert_eq!(&*embedded.data, cff, "the CFF table is handed on unchanged");
        assert!(
            super::units_per_em(&embedded.data, embedded.program, "/F1").is_ok(),
            "the em square comes from the CFF's own FontMatrix"
        );
    }

    /// An `OTTO` that *does* carry a `head` stays on the sfnt route, which is the ordinary file.
    ///
    /// The rule above turns on the one table whose absence leaves an sfnt reader without a scale,
    /// and not on the container: a font with a `head` states its own em square and may carry
    /// `hmtx` widths and a `cmap` this tree reads through `skrifa`. Narrowing it that way is what
    /// keeps the change to the programs that could not be read at all.
    #[test]
    fn an_opentype_program_with_a_head_is_still_an_sfnt() {
        let cff: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");
        // A `head` table's fifty-four bytes, of which only the units per em at offset 18 is read
        // here; 1000 is what a CFF-based face ordinarily states.
        let mut head = vec![0u8; 54];
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        let wrapped = otto(&[(b"CFF ", cff), (b"cmap", &[0, 0, 0, 0]), (b"head", &head)]);
        assert_eq!(embedded_of(b"FontFile3", &wrapped).program, Program::Sfnt);
    }
}
