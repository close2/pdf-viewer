//! The font programs a conversion embeds, and the advances it restates inside them.
//!
//! `doc/pdf-a-conversion-limits.md` section 4.9 is the specification for both, and the two are
//! one subject because they need the same construction: a program whose stated advances are the
//! numbers the font *dictionary* already states.
//!
//! # The two things done here, and the one that is never done
//!
//! - **A font the file never embedded** (ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section
//!   6.2.10.4.1) gets one of the faces this program ships, embedded and reported. Section 4.9's
//!   argument for making that the default rather than a refusal is that a PDF whose font is not
//!   embedded *has no appearance of its own* — every reader picks a face at display time and they
//!   pick different ones — so embedding one removes an indeterminacy rather than creating one,
//!   and `doc/questions/A47` settled it.
//! - **A font whose program disagrees with its dictionary** (ISO 19005-2 section 6.2.11.5,
//!   ISO 19005-4 section 6.2.10.5) has the *program* restated, never the dictionary.
//!
//! Section 4.9 sets out three routes to make the two statements agree and ranks them, and getting
//! the last two the wrong way round moves every line of the document:
//!
//! 1. **A metric-compatible face**, whose own advances already are the file's numbers. Free, and
//!    it is the commonest case by far, because the faces this program ships are metric-compatible
//!    with §9.6.2.2's fourteen.
//! 2. **The program's advances restated** to the numbers the file already states.
//!    [`pdf_font::restate`] is that, and ISO 32000-2 §9.2.4 is why it costs nothing on the page:
//!
//!    > Storing this information in the font dictionary, although redundant, enables a PDF
//!    > processor to determine glyph positioning without having to look inside the font program.
//!
//! 3. **`/Widths` restated to match the program.** **Never.** `/Widths` is what positions the
//!    glyphs, so this is the one of the three that moves the text — and nothing in this file
//!    writes a `/Widths`, a `/W` or a `/DW`, which is what makes that true by construction rather
//!    than by care.
//!
//! # Why the face is the compiled-in one and never the machine's
//!
//! [`pdf_font::standard::shipped_face`] rather than `pdf_font::substitute::find`, and the
//! difference is two requirements rather than a preference. ISO 19005-2 section 6.2.11.4.1
//! admits only a program that may lawfully be embedded for unlimited universal rendering, which
//! a face installed on somebody's machine generally may not be; and an archived file that
//! depended on which machine converted it would be the indeterminacy the format exists against.
//!
//! **Where no shipped face covers a document's characters, this refuses by name** rather than
//! embedding a face that draws the wrong thing — `doc/questions/A47`'s "refuse rather than
//! guess", which is also section 2.1's line about a substitution that answers *what does this
//! glyph look like* being unable to answer *which character is this code*.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use pdf_archive::survey::{SelectedFont, Survey};
use pdf_font::standard::ShippedFace;
use pdf_font::substitute::Format;
use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use super::COMPRESSION_LEVEL;
use super::decision::Because;
use super::prepare::Spare;

/// The tolerance both parts allow between a dictionary's width and its program's, in ems.
///
/// ISO 19005-2 section 6.2.11.5 and ISO 19005-4 section 6.2.10.5 state the same number: a
/// thousandth of a text space unit. It is restated here rather than taken from `pdf_archive`
/// because this decides *whether a rewrite is needed*, which is a different question from
/// whether the file conforms, and a converter that read the validator's constant would silently
/// follow it if it ever came to mean something else.
const CONSISTENT: f32 = 0.001;

/// Which of section 4.9's two permitted metric routes one font took.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricRoute {
    /// Route 1: the face's own advances already are the numbers the dictionary states.
    FaceMetrics,
    /// Route 2: the program's advances were restated to the dictionary's numbers.
    RestatedProgram,
}

impl MetricRoute {
    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::FaceMetrics => "face-metrics",
            Self::RestatedProgram => "restated-program",
        }
    }

    /// What the route did, in one clause for a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::FaceMetrics => {
                "the face's own advances are the widths the file states, so nothing was restated"
            }
            Self::RestatedProgram => {
                "the program's advances were restated to the widths the file already states, \
                 which is what keeps every glyph where the content stream put it"
            }
        }
    }
}

/// One font this conversion embedded a face into, named for the report.
///
/// `doc/pdf-a-conversion-limits.md` section 4.9 asks for exactly these four things per font: the
/// face requested, the face used, which metric route was taken, and — through
/// [`super::Conversion::recorded`] — an `xmpMM:History` entry saying so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstitutedFont {
    /// The resource name the content stream selected the font by.
    pub resource: String,
    /// The `/BaseFont` the document asked for.
    pub requested: String,
    /// The face this program embedded in its place.
    pub face: &'static str,
    /// Which of section 4.9's two metric routes it took.
    pub route: MetricRoute,
}

/// One font whose own embedded program had its advances restated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestatedFont {
    /// The resource name the content stream selected the font by.
    pub resource: String,
    /// The `/BaseFont` the document states.
    pub requested: String,
    /// How many of the program's glyphs had their advance restated.
    pub glyphs: usize,
}

/// The font programs to embed, and the fonts they belong to.
#[derive(Debug, Default)]
pub(super) struct Substitutes {
    /// The `/FontFile` entry each descriptor object gains.
    pub(super) at: BTreeMap<ObjectId, Embedding>,
    /// The stream objects this conversion adds, in the source's numbering.
    pub(super) written: BTreeMap<ObjectId, Object>,
    /// What was done, per font.
    pub(super) done: Vec<SubstitutedFont>,
}

/// The font programs to replace, by the stream object each is.
#[derive(Debug, Default)]
pub(super) struct Metrics {
    /// The replacement stream for each font program object.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// What was done, per font.
    pub(super) done: Vec<RestatedFont>,
}

/// One `/FontFile` entry a font descriptor gains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Embedding {
    /// Which of §9.9's three keys carries it (Table 124).
    pub(super) key: &'static str,
    /// The stream object holding the program.
    pub(super) at: ObjectId,
}

/// Where one font's program is, and what reads it.
struct Embedded {
    /// The stream object the key names.
    at: ObjectId,
    /// Which reader the bytes belong to.
    format: Format,
    /// Whether §9.9's Table 124 makes `/Length1` part of the entry, which a rewrite must restate.
    length1: bool,
}

// ----------------------------------------------------------------------------------------------
// Route 2 over a program the file already carries: ISO 19005-2 6.2.11.5, ISO 19005-4 6.2.10.5.
// ----------------------------------------------------------------------------------------------

/// Restates the advances of every embedded program that disagrees with its own dictionary.
///
/// The population is the validator's own: a font a content stream rendered, whose program this
/// tree reads, and whose dictionary states a width for a shown code that the program does not
/// state within [`CONSISTENT`]. Nothing else is touched — a program that already agrees crosses
/// the conversion byte for byte, which is `doc/adr/0947`'s first rule.
pub(super) fn restate_metrics(document: &Document, survey: &Survey) -> Result<Metrics, Because> {
    let mut metrics = Metrics::default();
    for used in survey.fonts() {
        if !used.rendered {
            continue;
        }
        let Some((font, embedded)) = own_program(document, used) else {
            continue;
        };
        // **Only the glyphs that actually disagree**, which is `doc/adr/0947`'s first rule read
        // one level down: nothing is changed that no failed requirement asked for, and a glyph
        // whose two statements already agree asked for nothing. A glyph the program states no
        // advance for is left alone too — that answer is a fact about this reader rather than
        // about the file, and `pdf_archive` passes over the same case for the same reason.
        let wanted: BTreeMap<u16, f32> = shown_widths(&font, used)?
            .into_iter()
            .filter(|(_, (width, program))| program.is_some_and(|own| disagrees(*width, own)))
            .map(|(glyph, (width, _))| (glyph, width))
            .collect();
        if wanted.is_empty() {
            continue;
        }
        let bytes = document
            .get(embedded.at)
            .as_stream()
            .and_then(|stream| document.decoded_stream_data(stream))
            .ok_or(Because::NotBuiltYet(PROGRAM_NOT_DECODED))?;
        let restated = pdf_font::restate::with_widths(&bytes, embedded.format, &wanted)
            .map_err(|_| Because::NotBuiltYet(PROGRAM_NOT_RESTATABLE))?;
        let Object::Stream(stream) = document.get(embedded.at) else {
            return Err(Because::NotBuiltYet(PROGRAM_NOT_DECODED));
        };
        metrics.at.insert(
            embedded.at,
            program_stream(Some(&stream.dict), &restated, embedded.length1)
                .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?,
        );
        metrics.done.push(RestatedFont {
            resource: used.name.clone(),
            requested: base_font(document, &used.dict),
            glyphs: wanted.len(),
        });
    }
    if metrics.at.is_empty() {
        return Err(Because::NotBuiltYet(NO_PROGRAM_TO_RESTATE));
    }
    Ok(metrics)
}

/// The width the dictionary states and the one the program states, per glyph of a shown code.
///
/// Keyed by glyph rather than by code because that is what the program states an advance for,
/// and because two codes reaching one glyph is the case a restatement cannot serve: a program
/// holds one advance per glyph, so a dictionary asking for two of them is a document no rewrite
/// of the program can satisfy and one this refuses rather than half-corrects.
fn shown_widths(
    font: &LoadedFont,
    used: &SelectedFont,
) -> Result<BTreeMap<u16, (f32, Option<f32>)>, Because> {
    let mut widths: BTreeMap<u16, (f32, Option<f32>)> = BTreeMap::new();
    for text in used.shown.keys() {
        for code in font.decode(text) {
            let Some(glyph) = font.glyph_index(code) else {
                continue;
            };
            let stated = font.advance(code);
            match widths.entry(glyph) {
                Entry::Vacant(slot) => {
                    slot.insert((stated, font.program_advance(code)));
                }
                Entry::Occupied(held) if disagrees(held.get().0, stated) => {
                    return Err(Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH));
                }
                Entry::Occupied(_) => {}
            }
        }
    }
    Ok(widths)
}

/// Whether two statements of one advance are further apart than ISO 19005 allows.
fn disagrees(stated: f32, program: f32) -> bool {
    (stated - program).abs() > CONSISTENT
}

// ----------------------------------------------------------------------------------------------
// Section 4.9's default: a face embedded where the file embedded none.
// ----------------------------------------------------------------------------------------------

/// Embeds a shipped face into every rendered font whose descriptor carries no program.
///
/// # What is refused, and each refusal is by name
///
/// - **A composite font.** §9.7.4.2 makes a CID an index into the glyphs of the font that
///   defined it, so a substitute cannot be addressed by one at all; section 2.1 is the reading.
/// - **A font with no descriptor**, which is the standard 14 stated by name alone. Embedding
///   needs one, and §9.8.1's Table 122 makes `/StemV` required of it — a measurement of a face,
///   which this converter does not take on the document's behalf.
/// - **A dictionary whose subtype the face's format may not go in**, which §9.9's Table 124
///   decides rather than this converter.
/// - **A code the face has no glyph for**, which is `doc/questions/A47`'s "refuse rather than
///   guess" and is what keeps both parts' `.notdef` clause satisfied without anything being
///   invented.
pub(super) fn embed_faces(
    document: &Document,
    survey: &Survey,
    spare: &mut Spare,
) -> Result<Substitutes, Because> {
    let mut substitutes = Substitutes::default();
    for used in survey.fonts() {
        if !used.rendered || !needs_a_program(document, &used.dict) {
            continue;
        }
        let subtype = name_of(document, &used.dict, "Subtype").unwrap_or_default();
        if subtype == "Type3" {
            continue;
        }
        if subtype == "Type0" {
            return Err(Because::NotBuiltYet(COMPOSITE_NOT_SUBSTITUTED));
        }
        // The *raw* entry rather than the resolved one: a program is embedded by writing a key
        // into the descriptor object, so a descriptor written directly inside the font
        // dictionary has no object for this walk to rewrite and is refused by name.
        let descriptor = used
            .dict
            .get("FontDescriptor")
            .and_then(Object::as_reference)
            .ok_or(Because::NotBuiltYet(NO_DESCRIPTOR_TO_EMBED_INTO))?;
        let face = pdf_font::standard::shipped_face(document, &used.dict, &used.name)
            .map_err(|_| Because::NotBuiltYet(NO_SHIPPED_FACE_COVERS_IT))?;
        let key = key_for(&subtype, face.format).ok_or(Because::NotBuiltYet(NO_SHIPPED_FORMAT))?;
        let font = LoadedFont::load(document, &used.dict, &used.name)
            .map_err(|_| Because::NotBuiltYet(FONT_NOT_READ))?;
        let (widths, route) = face_widths(&font, &face, used)?;
        let program = if route == MetricRoute::FaceMetrics {
            face.program.to_vec()
        } else {
            pdf_font::restate::with_widths(face.program, face.format, &widths)
                .map_err(|_| Because::NotBuiltYet(FACE_NOT_RESTATABLE))?
        };
        let at = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        let stream = program_stream(
            subtype_dictionary(key).as_ref(),
            &program,
            key == "FontFile2",
        )
        .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
        substitutes.written.insert(at, stream);
        substitutes.at.insert(descriptor, Embedding { key, at });
        substitutes.done.push(SubstitutedFont {
            resource: used.name.clone(),
            requested: base_font(document, &used.dict),
            face: face.describe(),
            route,
        });
    }
    if substitutes.at.is_empty() {
        return Err(Because::NotBuiltYet(NO_FONT_TO_EMBED));
    }
    Ok(substitutes)
}

/// The widths the face has to state for the codes this document shows, and which route that is.
///
/// Route 1 where every one of them already agrees with the dictionary to within [`CONSISTENT`] —
/// which is what "metric-compatible" means and what the faces this program ships are, for
/// §9.6.2.2's fourteen — and route 2 otherwise. A code the face has no glyph for is a refusal
/// rather than a width: showing it would reference `.notdef`, which ISO 19005-2 section 6.2.11.8
/// and ISO 19005-4 section 6.2.10.9 forbid.
fn face_widths(
    font: &LoadedFont,
    face: &ShippedFace,
    used: &SelectedFont,
) -> Result<(BTreeMap<u16, f32>, MetricRoute), Because> {
    let mut widths: BTreeMap<u16, f32> = BTreeMap::new();
    let mut route = MetricRoute::FaceMetrics;
    for text in used.shown.keys() {
        for code in font.decode(text) {
            let byte = single_byte(code).ok_or(Because::NotBuiltYet(NOT_A_SIMPLE_CODE))?;
            let glyph = face
                .glyph(byte)
                .filter(|glyph| *glyph != pdf_font::NOTDEF_GLYPH)
                .ok_or(Because::NotBuiltYet(NO_SHIPPED_FACE_COVERS_IT))?;
            let stated = font.advance(code);
            match widths.entry(glyph) {
                Entry::Vacant(slot) => {
                    slot.insert(stated);
                }
                Entry::Occupied(held) if disagrees(*held.get(), stated) => {
                    return Err(Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH));
                }
                Entry::Occupied(_) => {}
            }
            if pdf_font::restate::advance(face.program, face.format, glyph)
                .is_none_or(|own| disagrees(stated, own))
            {
                route = MetricRoute::RestatedProgram;
            }
        }
    }
    Ok((widths, route))
}

/// A simple font's code as the one byte §9.10.3 makes it, or nothing for a wider one.
fn single_byte(code: Code) -> Option<u8> {
    u8::try_from(code.value()).ok()
}

// ----------------------------------------------------------------------------------------------
// Reading the document: where a program is, and what a rewritten one looks like.
// ----------------------------------------------------------------------------------------------

/// One rendered font loaded from **its own** embedded program, with where that program is.
///
/// The same population `pdf_archive`'s font rules take, and for the same reasons: a font with no
/// program has nothing to restate, and a font whose program this tree could not read is one
/// whose bytes it must not rewrite.
fn own_program(document: &Document, used: &SelectedFont) -> Option<(LoadedFont, Embedded)> {
    let subtype = name_of(document, &used.dict, "Subtype")?;
    if subtype == "Type3" {
        return None;
    }
    let holder = if subtype == "Type0" {
        descendant(document, &used.dict)?
    } else {
        used.dict.clone()
    };
    let descriptor = holder
        .get("FontDescriptor")
        .and_then(Object::as_reference)?;
    let embedded = embedded_program(document, descriptor)?;
    let font = LoadedFont::load(document, &used.dict, &used.name).ok()?;
    (!font.is_substituted()).then_some((font, embedded))
}

/// The `/FontFile` entry a descriptor states, and which reader its bytes belong to.
///
/// §9.9's Table 124 is the mapping from key and `/Subtype` to format. `/FontFile` — a Type 1
/// program in §9.9's own encrypted form — is deliberately absent: [`pdf_font::restate`] does not
/// restate one, and answering `None` here is what turns that into a refusal by name rather than
/// a program written wrongly.
fn embedded_program(document: &Document, descriptor: ObjectId) -> Option<Embedded> {
    let dict = document.get(descriptor).as_dict().cloned()?;
    if let Some(at) = dict.get("FontFile2").and_then(Object::as_reference) {
        return Some(Embedded {
            at,
            format: Format::Sfnt,
            length1: true,
        });
    }
    let at = dict.get("FontFile3").and_then(Object::as_reference)?;
    let stream = document.get(at);
    let subtype = name_of(document, &stream.as_stream()?.dict, "Subtype")?;
    let format = match subtype.as_str() {
        "Type1C" | "CIDFontType0C" => Format::BareCff,
        "OpenType" => Format::Sfnt,
        _ => return None,
    };
    Some(Embedded {
        at,
        format,
        length1: false,
    })
}

/// Whether a rendered font's descriptor states no program at all.
fn needs_a_program(document: &Document, dict: &Dictionary) -> bool {
    let holder = match name_of(document, dict, "Subtype").as_deref() {
        Some("Type0") => match descendant(document, dict) {
            Some(descendant) => descendant,
            None => return false,
        },
        _ => dict.clone(),
    };
    let Some(descriptor) = document
        .get_key(&holder, "FontDescriptor")
        .as_dict()
        .cloned()
    else {
        // A rendered font with no descriptor at all still fails the embedding requirement, and
        // is refused by name in [`embed_faces`] rather than passed over here.
        return true;
    };
    // **Resolved rather than counted**, which §7.3.7 makes the difference between a key and a
    // statement: "A dictionary entry whose value is null (see 7.3.9, "Null object") shall be
    // treated the same as if the entry does not exist", and §7.3.10 puts a dangling reference in
    // the same place. So `/FontFile3 19 0 R` where object 19 is null embeds no program, and this
    // is the corpus's `6-2-11-4-1-t01-fail-a` exactly.
    ["FontFile", "FontFile2", "FontFile3"]
        .iter()
        .all(|key| document.get_key(&descriptor, key).is_null())
}

/// Which of §9.9's keys may carry a face of this format in a dictionary of this subtype.
///
/// Table 124 decides it and this converter does not: a `glyf`-based sfnt goes in a `/FontFile2`
/// under a `/TrueType` dictionary, and a bare CFF in a `/FontFile3` with `/Subtype /Type1C`
/// under a `/Type1` or `/MMType1` one. Every other pairing the table does not state is `None`,
/// which becomes a refusal naming the pairing rather than a file the table does not admit.
fn key_for(subtype: &str, format: Format) -> Option<&'static str> {
    match (subtype, format) {
        ("TrueType", Format::Sfnt) => Some("FontFile2"),
        ("Type1" | "MMType1", Format::BareCff) => Some("FontFile3"),
        _ => None,
    }
}

/// The stream dictionary a `/FontFile3` needs, which §9.9's Table 126 makes `/Subtype` on.
fn subtype_dictionary(key: &str) -> Option<Dictionary> {
    (key == "FontFile3").then(|| {
        let mut dict = Dictionary::new();
        dict.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"Type1C"[..])),
        );
        dict
    })
}

/// A font program as a stream, compressed, with the lengths §9.9's Table 126 requires.
///
/// `/Length1` is the program's own length before any filter, which Table 126 makes required of a
/// `/FontFile2`; a rewritten program is a different length from the one the producer wrote, so a
/// conversion that left the producer's number would be handing a reader a lie about its own
/// bytes. Every other key of the source's stream dictionary is carried, because a `/FontFile3`'s
/// `/Subtype` is one of them and losing it would leave a program no reader could classify.
fn program_stream(from: Option<&Dictionary>, program: &[u8], length1: bool) -> Option<Object> {
    let mut dict = from.cloned().unwrap_or_default();
    dict.remove("Length1");
    dict.remove("Length2");
    dict.remove("Length3");
    if length1 {
        dict.insert(
            Name::new(&b"Length1"[..]),
            Object::Integer(i64::try_from(program.len()).ok()?),
        );
    }
    let data = flate_encode(program, COMPRESSION_LEVEL)?;
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    dict.remove("DecodeParms");
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// A composite font's one descendant `CIDFont`, which §9.7.6.2 makes the array's only entry.
fn descendant(document: &Document, dict: &Dictionary) -> Option<Dictionary> {
    let array = document.get_key(dict, "DescendantFonts");
    let first = document.resolve(&array).as_array()?.first().cloned()?;
    document.resolve(&first).as_dict().cloned()
}

/// A dictionary's name-valued key as a string.
fn name_of(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    document
        .get_key(dict, key)
        .as_name()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
}

/// The `/BaseFont` a font dictionary names, for a report that has to say what was asked for.
fn base_font(document: &Document, dict: &Dictionary) -> String {
    name_of(document, dict, "BaseFont").unwrap_or_else(|| "an unnamed font".to_owned())
}

// ----------------------------------------------------------------------------------------------
// The refusals, each of which says which of the three kinds of *no* it is.
// ----------------------------------------------------------------------------------------------

/// Why nothing was restated: no failed requirement asked for it, or none of the fonts needed it.
const NO_PROGRAM_TO_RESTATE: &str = "no font in this document has an embedded program whose \
     stated advances this conversion had to restate";

/// Why no face was embedded.
const NO_FONT_TO_EMBED: &str = "no font a content stream renders in this document lacks an \
     embedded program";

/// Why a program this tree cannot decode is not restated.
const PROGRAM_NOT_DECODED: &str = "the embedded font program whose advances would have to be \
     restated could not be decoded from its stream, so there are no bytes to restate";

/// Why a program whose advances are stated somewhere unreachable is not restated.
const PROGRAM_NOT_RESTATABLE: &str = "this font's embedded program states a glyph's advance \
     somewhere this converter does not rewrite — a Type 1 program's encrypted charstrings, or a \
     CFF charstring whose width is inside a subroutine that is reached in a way \
     pdf_font::restate does not follow. The alternative is restating /Widths to match the \
     program, and doc/pdf-a-conversion-limits.md section 4.9 calls that one never: /Widths is \
     what positions the glyphs, so it is the one change that would move every line of the page";

/// Why a program that will not compress into a stream is not written.
const PROGRAM_NOT_ENCODED: &str = "the font program this conversion built is longer than a \
     stream's /Length can state";

/// Why a document whose dictionary asks two widths of one glyph is refused.
const TWO_WIDTHS_FOR_ONE_GLYPH: &str = "two of this font's codes reach the same glyph and its \
     dictionary states different widths for them. A font program states one advance per glyph, \
     so no program can satisfy both — and the only remedy left would be restating /Widths, which \
     doc/pdf-a-conversion-limits.md section 4.9 forbids because it is what positions the glyphs";

/// Why a composite font is not given a substitute.
const COMPOSITE_NOT_SUBSTITUTED: &str = "this document renders a composite font it does not \
     embed. ISO 32000-2 §9.7.4.2 makes a CID an index into the glyphs of the font that defined \
     it, so a code of this font names a glyph of a program that is not here and names nothing in \
     any other face — a substitute could be given the right shapes only by deciding which \
     character each code was for, which is the evidence the file never carried. \
     doc/pdf-a-conversion-limits.md section 2.1 is the reading, and supplying the font itself \
     with --font resolves it";

/// Why a font with no descriptor is not given a substitute.
const NO_DESCRIPTOR_TO_EMBED_INTO: &str = "this rendered font states no FontDescriptor, which is \
     what §9.6.2.2 lets one of the standard 14 do, and a font program is embedded into a \
     descriptor. Writing one means stating the entries §9.8.1's Table 122 makes required of it — \
     /StemV above all, which is a measurement of a face's stems that nothing in this file states \
     — so the descriptor would be this converter's description of a face rather than the \
     document's";

/// Why no face covers a document's characters.
const NO_SHIPPED_FACE_COVERS_IT: &str = "this document renders a font it does not embed, and no \
     face this program ships has a glyph for every code it shows. doc/questions/A47 is explicit \
     that where no shipped face covers a document's characters the answer is to refuse rather \
     than guess: embedding a face that draws nothing for those codes would reference the .notdef \
     glyph, which ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid outright. \
     Supplying the font with --font resolves it";

/// Why a face of the wrong format for the dictionary is not embedded.
const NO_SHIPPED_FORMAT: &str = "the face this program ships for the font this document asks for \
     is not in a format ISO 32000-2 §9.9's Table 124 admits under this font dictionary's own \
     Subtype — a Type1 dictionary takes a Compact Font Format program and the sans-serif face \
     shipped here is a glyf-based sfnt. Shipping an open sans-serif face in CFF form would close \
     it, which is a licence question rather than a code one";

/// Why a face whose advances cannot be restated is not embedded.
const FACE_NOT_RESTATABLE: &str = "the face this program ships would have to have its advances \
     restated to the widths this file states, and one of the glyphs it shows states its advance \
     in a place pdf_font::restate does not rewrite";

/// Why a font this reader cannot load is not given a substitute.
const FONT_NOT_READ: &str = "embedding a face means knowing which of its glyphs each code the \
     page shows selects, and this font's own encoding could not be read";

/// Why a composite code reaching this path is refused.
const NOT_A_SIMPLE_CODE: &str = "this font's dictionary states a simple font's subtype and its \
     content streams show a code wider than the one byte ISO 32000-2 §9.10.3 gives one, so which \
     glyph of a face the code selects is not a question this converter can answer";

/// Why an object number could not be found for a font program.
const NO_SPARE_OBJECT: &str = "no unused object number could be found for the font program this \
     conversion would embed";
