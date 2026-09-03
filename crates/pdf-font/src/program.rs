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

use pdf_syntax::{Damage, Dictionary, Document};
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
        // `truncation` below is the structural half of [`whole_program`]'s rule, and kept: it
        // catches a program whose stream decoded whole and whose directory still overruns it.
        let data = whole_program(document, &stream.dict, &decoded, key, name)?;

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
        // prefix is no more readable than a truncated sfnt is. Same refusal, same reason — and
        // the same exception, for the same clause: [`stated_extent`] adds this program's three
        // sections up, and a decode that reaches their sum is not a prefix of anything.
        let decoded_data = whole_program(document, &stream.dict, &decoded, "FontFile", name)?;
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
        let program = if is_bare_cff(&decoded_data) {
            Program::BareCff
        } else {
            Program::Type1
        };
        return Ok(Embedded {
            data: decoded_data,
            program,
        });
    }
    Err(FontError::NotEmbedded {
        name: name.to_owned(),
    })
}

/// The whole font program `decoded` carries, or the refusal a prefix of one earns.
///
/// **A prefix of a font program is not a shorter font program**, which is the line ADR 0343 draws
/// between this and a content stream. §7.8.2 makes a content stream "a sequence of instructions",
/// so a prefix of one is a shorter sequence of the same kind and every instruction in it is the
/// producer's own. A font program is a *structure*: §9.9's `/FontFile2` and `/FontFile3` hold a
/// table directory whose offsets point forward, so a prefix is a directory describing bytes that
/// are not there, and reading one produces glyphs the producer never wrote rather than fewer of
/// the ones it did.
///
/// The witness is `issue13316_reduced.pdf`, whose `/FontFile2` is corrupt: read as a whole program
/// its 863 surviving bytes draw **A C E F** where the file's six CJK glyphs belong. Trap 5's own
/// test decides it — the marks a refusal gives up here are substitutive rather than additive (ADR
/// 0106), because the wrong glyphs stand *in place of* the right ones instead of beside them.
///
/// **Unless the bytes that arrived are the whole program**, which is a question the standard
/// answers rather than one this code has to guess at: [`stated_extent`] has Table 125's sentence.
/// A decode that reaches the length the file itself states has produced every byte of the program,
/// and what stopped short is the *filter's* end-of-data marker, which is outside it. That is not a
/// prefix, and the paragraph above does not reach it.
///
/// **Two conditions, and the second is not the length.** [`Damage::Truncated`] is the encoded data
/// running out before the filter's end-of-data, and every byte it produced is what the producer's
/// own compressor emitted from bytes the producer wrote — §7.4.1's "convert the information back
/// to its original form", achieved as far as it goes. [`Damage::Corrupt`] is the input violating
/// the filter's grammar at a definite point, past which nothing is the producer's; a program of
/// the right *length* whose tail is not its own is the wrong-glyph failure ADR 0343 refuses, not a
/// whole program. `issue13316_reduced.pdf` is why that is a condition rather than a remark: its
/// `/FontFile2` decodes to **168 808 bytes, which is its `/Length1` exactly**, and draws
/// **A C E F** where the file's six CJK glyphs belong.
///
/// **[`Damage::CheckValue`] is neither of those two and is refused for a reason of its own.** It
/// is a `FlateDecode` stream that reached RFC 1951's final block and produced every byte the
/// encoded data describes, over which RFC 1950's Adler-32 disagrees (ADR 0836) — so there is no
/// prefix, no shortfall against Table 125's extent, and nothing above reaches it. What it *does*
/// say is the one thing that decides this: the bytes are not the bytes that were compressed. A
/// checksum over a whole stream never says which of them, so a program admitted on one is a
/// program of the right length whose content may not be its own, which is the paragraph above's
/// case arriving by another road.
///
/// **Admitting it was measured before it was declined, and `issue13316_reduced.pdf` is what
/// declined it.** That file's `/FontFile2` is exactly this shape: 168 808 bytes, its `/Length1`
/// to the byte, RFC 1951 whole, and an Adler-32 that is not the one over them. Read as a program
/// it loads, and the page draws **A C E F** where `pdftoppm` draws five CJK glyphs and reports
/// nothing at all — because §9.6.5.4's `/Differences` names (`/g5167` and its four neighbours)
/// reach no glyph through the Adobe Glyph List and the program carries no `post` table, so the
/// clause's own closing permission takes over ("a PDF processor may supply a mapping of its
/// choosing") and the codes' own characters are what it supplies. Marks that stand *in place of*
/// the producer's are ADR 0106's substitutive failure and ADR 0459 already decided them.
///
/// **What the refusal costs is known and is written down rather than assumed.**
/// `PDFIUM-407-0.pdf` is the other side: three `/FontFile2` streams of this shape, of which two
/// carry a font that draws its page's German field labels exactly as `poppler` and `mupdf` draw
/// them — 8.507 levels of ink against their 15.919 and 15.175 while they are refused, 13.102 when
/// they are not. Both references read the same bytes and `mupdf` says so out loud
/// (`ignoring zlib error: incorrect data check`). That is evidence about a *file*, not about the
/// rule, and the rule is the one this tree already holds: a page missing marks and saying so
/// beats a page carrying marks nobody wrote.
///
/// # Errors
///
/// [`FontError::Malformed`] where the decode stopped short of the program the file states, where
/// no clause states an extent, where the damage is a corruption rather than a truncation, or
/// where the filter's check value disagrees over bytes that are otherwise whole.
fn whole_program(
    document: &Document,
    dict: &Dictionary,
    decoded: &pdf_syntax::Decoded,
    key: &str,
    name: &str,
) -> Result<Arc<[u8]>, FontError> {
    let Some(damage) = decoded.damage else {
        return Ok(Arc::clone(&decoded.data));
    };
    let whole = (damage == Damage::Truncated)
        .then(|| stated_extent(document, dict, key))
        .flatten()
        .and_then(|extent| decoded.data.get(..extent));
    whole.map(Arc::from).ok_or_else(|| FontError::Malformed {
        name: name.to_owned(),
        // Two sentences for two damages, because one of them was printed over both and was
        // false of the commoner: a stream that reached RFC 1951's final block is not a prefix,
        // and a report saying it is describes a file nobody has. ADR 0836.
        detail: match damage {
            Damage::CheckValue => format!(
                "/{key} decoded whole and its check value disagrees ({} bytes): RFC 1950's \
                 Adler-32 says these are not the bytes that were compressed, and a font program \
                 whose content may not be its own draws glyphs in place of the producer's",
                decoded.data.len()
            ),
            Damage::Truncated | Damage::Corrupt => format!(
                "/{key} decoded only as far as its damage ({damage:?}, {} bytes): a prefix of a \
                 font program is a directory describing bytes that are not there",
                decoded.data.len()
            ),
        },
    })
}

/// How many decoded bytes §9.9's Table 125 says this font program is, where it says at all.
///
/// The entry is the standard's own statement of an embedded program's extent, and it is stated
/// in terms of the *decoded* bytes, which is what makes it usable here:
///
/// > Length1 | integer | ( Required for Type 1 and TrueType font programs ) The length in bytes
/// > of the clear-text portion of the Type 1 font program, or the entire TrueType font program,
/// > after it has been decoded using the filters specified by the stream's Filter entry, if any.
///
/// So `/FontFile2`'s extent is `/Length1` alone — "the entire TrueType font program" — while
/// `/FontFile`'s is the sum of the three sections a Type 1 program has, `/Length2` being "the
/// length in bytes of the encrypted portion" and `/Length3` "the length in bytes of the
/// fixed-content portion". Where `/Length3` is zero the clause says the 512 zeros and the
/// `cleartomark` are absent from the stream and are the reader's to add, so the sum is still the
/// whole of what the stream carries.
///
/// **`/FontFile3` has no extent and gets `None` deliberately.** §9.9 says of a CFF program that
/// the three lengths "are not needed in that case and shall not be present", so there is nothing
/// to compare a short decode against and the caller's refusal stands.
///
/// This is ADR 0356's rule read one clause along: ask whether the standard states the thing's
/// extent before asking whether a filter failed, because a decode that reached a stated extent
/// is whole however it ended.
fn stated_extent(document: &Document, dict: &Dictionary, key: &str) -> Option<usize> {
    let length = |name: &str| {
        usize::try_from(document.get_key(dict, name).as_integer().unwrap_or(0)).unwrap_or(0)
    };
    match key {
        "FontFile2" => match length("Length1") {
            0 => None,
            stated => Some(stated),
        },
        // Table 125 makes `/Length1` and `/Length2` required of a Type 1 program and this asks
        // for both, because a file that states only the first has described the clear-text
        // header and said nothing about where the charstrings end — which is the prefix the
        // caller is right to refuse. `/Length3` may legitimately be zero, by the clause's own
        // sentence about the 512 zeros, so it is added rather than demanded.
        "FontFile" => match (length("Length1"), length("Length2")) {
            (0, _) | (_, 0) => None,
            (clear, encrypted) => Some(
                clear
                    .saturating_add(encrypted)
                    .saturating_add(length("Length3")),
            ),
        },
        _ => None,
    }
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

    use super::{Embedded, FontError, Program, embedded_program};

    /// A two-object document: object 1 is a font descriptor whose `key` is object 2.
    ///
    /// Table 120 gives the descriptor three keys for a program and this module chooses its
    /// reader by the bytes rather than by the key, so which one the fixture writes is a variable
    /// rather than a constant.
    /// `entries` is written into the stream dictionary beside its `/Length`, which is where
    /// Table 125's `/Length1` and Table 5's `/Filter` go — the two the damage tests below vary.
    fn descriptor_with(key: &[u8], entries: &str, program: &[u8]) -> Document {
        let mut out = Vec::from(*b"%PDF-1.7\n");
        let mut offsets = Vec::new();

        offsets.push(out.len());
        out.extend_from_slice(b"1 0 obj\n<< /Flags 4 /");
        out.extend_from_slice(key);
        out.extend_from_slice(b" 2 0 R >>\nendobj\n");

        offsets.push(out.len());
        out.extend_from_slice(
            format!(
                "2 0 obj\n<< /Length {} {entries} >>\nstream\n",
                program.len()
            )
            .as_bytes(),
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
        try_embedded_of(key, "", bytes).expect("the descriptor embeds a program")
    }

    fn try_embedded_of(key: &[u8], entries: &str, bytes: &[u8]) -> Result<Embedded, FontError> {
        let document = descriptor_with(key, entries, bytes);
        let descriptor = document
            .get(ObjectId {
                number: 1,
                generation: 0,
            })
            .as_dict()
            .expect("object 1 is the descriptor")
            .clone();
        embedded_program(&document, &descriptor, "/F1")
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

    /// `data` as a zlib stream of one stored block, with or without RFC 1951's final-block bit.
    ///
    /// A stored block needs no compressor, which is why the fixture uses one: RFC 1951 section 3.2.4
    /// gives it a header byte, a sixteen-bit length and its complement, and then the bytes
    /// themselves. Written non-final and with no adler32 after it, the input runs out before any
    /// block with `BFINAL` set — which is `Damage::Truncated` in this tree's words and is
    /// byte-for-byte the shape `0669424.pdf` carries: every byte of the program present, the
    /// filter's own end never written.
    fn zlib_stored(data: &[u8], last: bool) -> Vec<u8> {
        let length = u16::try_from(data.len()).expect("a fixture under 64 KiB");
        // CMF 0x78: deflate with a 32 KiB window. FLG 0x01 makes the pair a multiple of 31.
        let mut out = vec![0x78, 0x01, u8::from(last)];
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(data);
        if last {
            let (mut a, mut b) = (1u32, 0u32);
            for byte in data {
                a = a.saturating_add(u32::from(*byte)) % 65521;
                b = b.saturating_add(a) % 65521;
            }
            out.extend_from_slice(&(b.saturating_mul(65536).saturating_add(a)).to_be_bytes());
        }
        out
    }

    /// A program whose `/Length1` the decode reaches is whole, however the filter ended.
    ///
    /// §9.9's Table 125 states the extent — "the entire TrueType font program, after it has been
    /// decoded using the filters specified by the stream's Filter entry, if any" — so a decode
    /// that produced that many bytes has produced every byte of the program. What stopped short
    /// is RFC 1951's final block, which is the filter's framing and not the font's.
    ///
    /// `0669424.pdf` of the `SafeDocs` crawl is the witness (session 625): three `/FontFile2`
    /// streams, each decoding to exactly its `/Length1` and each ending without a final block,
    /// and 941 text operations refused for it while `poppler`, `mupdf` and `ghostscript` drew
    /// the page.
    #[test]
    fn a_font_program_that_reaches_its_stated_length_survives_a_truncated_filter() {
        let cff: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");
        let wrapped = otto(&[(b"CFF ", cff), (b"cmap", &[0, 0, 0, 0])]);
        let entries = format!("/Filter /FlateDecode /Length1 {}", wrapped.len());

        let whole = try_embedded_of(b"FontFile2", &entries, &zlib_stored(&wrapped, true))
            .expect("an undamaged stream is read");
        let damaged = try_embedded_of(b"FontFile2", &entries, &zlib_stored(&wrapped, false))
            .expect("a stream that reached its /Length1 is read");
        assert_eq!(damaged.program, whole.program);
        assert_eq!(
            &*damaged.data, &*whole.data,
            "the same program comes out of both"
        );
    }

    /// And the other side of it: bytes short of the stated extent are the prefix ADR 0343 refuses.
    ///
    /// The fixture is the same truncated stream under a `/Length1` one byte past what arrived,
    /// which is the only difference between the two tests — so what is being pinned is the
    /// comparison rather than the presence of the entry.
    #[test]
    fn a_font_program_short_of_its_stated_length_is_still_refused() {
        let cff: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");
        let wrapped = otto(&[(b"CFF ", cff), (b"cmap", &[0, 0, 0, 0])]);
        let entries = format!(
            "/Filter /FlateDecode /Length1 {}",
            wrapped.len().saturating_add(1)
        );

        let Err(FontError::Malformed { detail, .. }) =
            try_embedded_of(b"FontFile2", &entries, &zlib_stored(&wrapped, false))
        else {
            panic!("a program short of its stated length is a prefix and is refused")
        };
        assert!(
            detail.contains("decoded only as far as its damage"),
            "unexpected refusal: {detail}"
        );
    }

    /// A `/FontFile3` states no extent at all, so its refusal is unchanged.
    ///
    /// §9.9 says of a CFF program that `/Length1`, `/Length2` and `/Length3` "are not needed in
    /// that case and shall not be present". Nothing states where such a program ends, so there
    /// is nothing to compare a short decode against — which is why the exception is written as a
    /// question about the *clause* rather than about the damage.
    #[test]
    fn a_compact_font_program_has_no_stated_extent_and_is_refused() {
        let cff: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");
        let wrapped = otto(&[(b"CFF ", cff), (b"cmap", &[0, 0, 0, 0])]);
        let entries = format!("/Filter /FlateDecode /Length1 {}", wrapped.len());
        assert!(
            try_embedded_of(b"FontFile3", &entries, &zlib_stored(&wrapped, false)).is_err(),
            "no clause states a CFF program's length"
        );
    }

    /// A program of the stated *length* whose bytes are not the producer's is still refused.
    ///
    /// The two damages are not two grades of the same thing. `Damage::Truncated` is the encoded
    /// data running out before the filter's end-of-data, and every byte it produced is what the
    /// producer's compressor emitted; `Damage::Corrupt` is the input violating RFC 1951's grammar
    /// at a definite point, past which nothing is the producer's. Table 125 says how many decoded
    /// bytes the program is and says nothing about whether they are the right ones, so the length
    /// alone cannot separate the two.
    ///
    /// **And a length test alone would not have separated these two, on the corpus's own
    /// witness.** `issue13316_reduced.pdf`'s `/FontFile2` decodes to 168 808 bytes with a corrupt
    /// tail and its `/Length1` is 168 808, so it reaches its stated extent exactly — and read as a
    /// whole program it draws **A C E F** where the file's six CJK glyphs belong, which is the
    /// substitutive failure ADR 0343 exists to refuse. `tests/silent_fonts.rs` holds that page;
    /// this is the same rule on a fixture, so the rule is pinned where the code is.
    ///
    /// The fixture puts the whole program in a stored block and follows it with a block header
    /// whose type is RFC 1951's reserved `11`, so the decode produces every byte the length asks
    /// for and *then* meets something the grammar does not admit. The refusal is asserted to name
    /// `Corrupt`, which is what keeps the test from passing because the bytes ran out instead.
    #[test]
    fn a_corrupt_program_of_the_stated_length_is_refused() {
        let cff: &[u8] = include_bytes!("../../../data/standard-fonts/FoxitSerif.pfb");
        let wrapped = otto(&[(b"CFF ", cff), (b"cmap", &[0, 0, 0, 0])]);
        let entries = format!("/Filter /FlateDecode /Length1 {}", wrapped.len());

        let mut corrupt = zlib_stored(&wrapped, false);
        corrupt.push(0b0000_0110);

        let Err(FontError::Malformed { detail, .. }) =
            try_embedded_of(b"FontFile2", &entries, &corrupt)
        else {
            panic!("a corrupt stream is not a whole program however long it is")
        };
        assert!(
            detail.contains("Corrupt"),
            "and it is refused for the corruption rather than for a shortfall: {detail}"
        );
    }
}
