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
//! # The same agreement going down the page
//!
//! ISO 19005-4 section 6.2.10.5's third paragraph asks it again of a composite font shown in
//! writing mode 1, between §9.7.4.3's `/DW2` and `/W2` and the program's `vmtx`. Part 2 states no
//! such rule, so the two are separate `Rewrite`s: at a PDF/A-2 target nothing asks for the
//! vertical one and `doc/adr/0947`'s first rule is that nothing else may happen.
//!
//! **Route 3 is forbidden there by a clause rather than by an inference.** §9.9.1:
//!
//! > The "vhea" and "vmtx" tables that specify vertical metrics shall never be used by a PDF
//! > processor. The only way to specify vertical metrics in PDF shall be by means of the DW2 and
//! > W2 entries in a CIDFont dictionary.
//!
//! So restating the program is unobservable to any conforming reader, and restating the
//! dictionary would move every glyph on a vertical line on the authority of a table nothing may
//! consult. **Both restatements replace the same stream**, which is why [`Metrics`] holds one
//! replacement per program object and a set per requirement saying which asked for it: a font
//! disagreeing in both directions is restated twice into one set of bytes, and a map per rewrite
//! would drop the second write in silence.
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

mod composite;

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::{SelectedFont, Survey};
use pdf_font::standard::{ShippedFace, SuppliedFace};
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
    /// The face this conversion embedded in its place.
    pub face: String,
    /// Whose face it was — this program's, or the operator's.
    pub authority: FaceAuthority,
    /// Which of section 4.9's two metric routes it took.
    pub route: MetricRoute,
}

/// Whose font program a substitution embedded.
///
/// **Two, and the difference is a licence rather than a preference.** ISO 32000-2 §9.9.1:
///
/// > One of the conditions may be that the font program cannot be embedded, in which case it
/// > should not be incorporated into a PDF file.
///
/// ISO 19005-2 section 6.2.11.4.1 admits only a program that may lawfully be embedded for
/// unlimited universal rendering. For a face this program ships, that is a fact the project
/// checked once and recorded in `data/standard-fonts/PROVENANCE.md`; for a face an operator
/// names on the command line it is a fact **only the operator can state**, and naming the file
/// is the statement. So the two are kept apart everywhere they are reported — in the run's own
/// report and in the output's `xmpMM:History` — rather than both appearing as *a face was
/// embedded* (`doc/rfc/0007`'s `supply`, `doc/adr/1209`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceAuthority {
    /// One of the faces this program ships, whose licence the project checked once.
    Shipped,
    /// A program the operator named with `--font`, on the operator's own authority.
    Operator,
}

impl FaceAuthority {
    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Shipped => "shipped",
            Self::Operator => "operator",
        }
    }

    /// Whose the face was, in one clause for a person reading the file's own provenance.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Shipped => "a face this program ships",
            Self::Operator => {
                "a program the converting operator named, who states it may lawfully be embedded                  for unlimited, universal rendering"
            }
        }
    }
}

/// One face a substitution may embed: this program's, or the one an operator named.
///
/// **One type rather than two paths**, because everything after the choice is the same: §9.9's
/// Table 124 decides which key carries it, [`face_widths`] asks it for the glyph each shown code
/// selects, and [`pdf_font::restate`] makes its advances the numbers the dictionary already
/// states so that no glyph moves. What differs is only where the bytes came from and who said
/// they could be embedded, which is [`FaceAuthority`].
enum Face {
    /// [`pdf_font::standard::shipped_face`]'s answer.
    Shipped(ShippedFace),
    /// [`pdf_font::standard::supplied_face`]'s answer, over bytes the operator named.
    Supplied(SuppliedFace, String),
}

impl Face {
    /// The program's bytes.
    fn program(&self) -> &[u8] {
        match self {
            Self::Shipped(face) => face.program,
            Self::Supplied(face, _) => &face.program,
        }
    }

    /// Which reader the bytes belong to, which §9.9's Table 124 turns into a `/FontFile` key.
    const fn format(&self) -> Format {
        match self {
            Self::Shipped(face) => face.format,
            Self::Supplied(face, _) => face.format,
        }
    }

    /// The glyph a code selects in this face, where it selects one.
    fn glyph(&self, code: u8) -> Option<u16> {
        match self {
            Self::Shipped(face) => face.glyph(code),
            Self::Supplied(face, _) => face.glyph(code),
        }
    }

    /// The face's own name, for a report that has to say which face was embedded.
    fn describe(&self) -> String {
        match self {
            Self::Shipped(face) => face.describe().to_owned(),
            Self::Supplied(_, named) => named.clone(),
        }
    }

    /// Whose face it was.
    const fn authority(&self) -> FaceAuthority {
        match self {
            Self::Shipped(_) => FaceAuthority::Shipped,
            Self::Supplied(..) => FaceAuthority::Operator,
        }
    }
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
    /// How many had their `vmtx` advance height restated, going down the page.
    ///
    /// ISO 19005-4 section 6.2.10.5's third paragraph, and zero for every font a page does not
    /// set vertically — which is nearly all of them. The two counts are separate because the
    /// two requirements are: a document can fail either without the other.
    pub heights: usize,
}

/// The font programs to embed, and the fonts they belong to.
#[derive(Debug, Default)]
pub(super) struct Substitutes {
    /// The `/FontFile` entry each descriptor object gains.
    pub(super) at: BTreeMap<ObjectId, Embedding>,
    /// The stream objects this conversion adds, in the source's numbering.
    pub(super) written: BTreeMap<ObjectId, Object>,
    /// The descendant `CIDFont` objects that gain a `/CIDToGIDMap` of `Identity`.
    ///
    /// §9.7.4.2 makes a Type 2 `CIDFont`'s `/CIDToGIDMap` what maps a CID to a glyph index of the
    /// embedded program, and Table 115 makes the entry required once a program is embedded — so
    /// a dictionary that stated none while its program was elsewhere owes one the moment this
    /// conversion writes a program in. `Identity` is the only value written, which is what the
    /// glyphs were chosen under (`doc/adr/1222`).
    pub(super) identity_cid_to_gid: BTreeSet<ObjectId>,
    /// What was done, per font.
    pub(super) done: Vec<SubstitutedFont>,
}

/// The font programs to replace, by the stream object each is.
///
/// **One replacement per program object, whichever of the two requirements asked for it.** A
/// font whose dictionary disagrees with its program in both directions is one stream, restated
/// twice, and two maps keyed by the same object would have let the second rewrite be dropped
/// silently. The two sets say which requirement each replacement answers, so the report counts
/// them apart without the rewriter having to write the stream twice.
#[derive(Debug, Default)]
pub(super) struct Metrics {
    /// The replacement stream for each font program object.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// The programs whose `/Widths`, `/W` or `/DW` disagreement was answered.
    pub(super) horizontal: BTreeSet<ObjectId>,
    /// The programs whose `/DW2` or `/W2` disagreement was answered.
    pub(super) vertical: BTreeSet<ObjectId>,
    /// What was done, per font.
    pub(super) done: Vec<RestatedFont>,
}

/// Which of ISO 19005-4 section 6.2.10.5's two disagreements a conversion was asked to answer.
///
/// **Both are asked separately and neither implies the other**, which is `doc/adr/0947`'s first
/// rule: a part 2 target states no vertical requirement at all, so restating a `vmtx` there
/// would be changing a program nothing had asked to be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Directions {
    /// Whether `fonts/widths-agree-with-the-program` failed.
    pub(super) widths: bool,
    /// Whether `fonts/vertical-metrics-agree-with-the-program` failed.
    pub(super) vertical: bool,
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
pub(super) fn restate_metrics(
    document: &Document,
    survey: &Survey,
    wants: Directions,
) -> Result<Metrics, Because> {
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
        let widths: BTreeMap<u16, f32> = if wants.widths {
            shown_widths(&font, used)?
                .into_iter()
                .filter(|(_, (width, program))| program.is_some_and(|own| disagrees(*width, own)))
                .map(|(glyph, (width, _))| (glyph, width))
                .collect()
        } else {
            BTreeMap::new()
        };
        // The same question going down the page, asked only of a font a `CMap` sets vertically:
        // §9.7.5.1's `/WMode` is what [`pdf_font::LoadedFont::is_vertical`] answers, and it is
        // the condition ISO 19005-4 section 6.2.10.5's third paragraph states.
        let heights: BTreeMap<u16, f32> = if wants.vertical && font.is_vertical() {
            shown_heights(&font, used)?
                .into_iter()
                .filter(|(_, (stated, program))| program.is_some_and(|own| disagrees(*stated, own)))
                .map(|(glyph, (stated, _))| (glyph, stated))
                .collect()
        } else {
            BTreeMap::new()
        };
        if widths.is_empty() && heights.is_empty() {
            continue;
        }
        let bytes = document
            .get(embedded.at)
            .as_stream()
            .and_then(|stream| document.decoded_stream_data(stream))
            .ok_or(Because::NotBuiltYet(PROGRAM_NOT_DECODED))?;
        let mut restated = bytes.to_vec();
        if !widths.is_empty() {
            restated = pdf_font::restate::with_widths(&restated, embedded.format, &widths)
                .map_err(|_| Because::NotBuiltYet(PROGRAM_NOT_RESTATABLE))?;
        }
        if !heights.is_empty() {
            restated =
                pdf_font::restate::with_vertical_advances(&restated, embedded.format, &heights)
                    .map_err(|_| Because::NotBuiltYet(VERTICAL_NOT_RESTATABLE))?;
        }
        proves(&restated, embedded.format, &widths, &heights)?;
        let Object::Stream(stream) = document.get(embedded.at) else {
            return Err(Because::NotBuiltYet(PROGRAM_NOT_DECODED));
        };
        metrics.at.insert(
            embedded.at,
            program_stream(Some(&stream.dict), &restated, embedded.length1)
                .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?,
        );
        if !widths.is_empty() {
            metrics.horizontal.insert(embedded.at);
        }
        if !heights.is_empty() {
            metrics.vertical.insert(embedded.at);
        }
        metrics.done.push(RestatedFont {
            resource: used.name.clone(),
            requested: base_font(document, &used.dict),
            glyphs: widths.len(),
            heights: heights.len(),
        });
    }
    if metrics.at.is_empty() {
        return Err(Because::NotBuiltYet(NO_PROGRAM_TO_RESTATE));
    }
    Ok(metrics)
}

/// Reads the rewritten program back, and refuses it unless every glyph named actually moved.
///
/// **Session 971's rule, and it is a rule because a rewrite that silently missed a site would
/// convert a document into one that still fails the requirement it was converted for.** The
/// program is asked, through the same two readers a caller holding nothing but bytes has, what
/// it now states for each glyph the restatement named; a number that is not the one asked for
/// stops the conversion rather than reaching a file.
fn proves(
    program: &[u8],
    format: Format,
    widths: &BTreeMap<u16, f32>,
    heights: &BTreeMap<u16, f32>,
) -> Result<(), Because> {
    for (glyph, width) in widths {
        let stated = pdf_font::restate::advance(program, format, *glyph)
            .ok_or(Because::NotBuiltYet(RESTATEMENT_NOT_PROVED))?;
        if disagrees(stated, *width) {
            return Err(Because::NotBuiltYet(RESTATEMENT_NOT_PROVED));
        }
    }
    for (glyph, displacement) in heights {
        let stated = pdf_font::restate::vertical_advance(program, format, *glyph)
            .ok_or(Because::NotBuiltYet(RESTATEMENT_NOT_PROVED))?;
        if disagrees(stated, *displacement) {
            return Err(Because::NotBuiltYet(RESTATEMENT_NOT_PROVED));
        }
    }
    Ok(())
}

/// The vertical displacement the dictionary states and the one the program states, per glyph.
///
/// [`shown_widths`]'s counterpart for ISO 19005-4 section 6.2.10.5's third paragraph, keyed by
/// glyph for the same reason and refusing two disagreeing statements for one glyph for the same
/// reason: `vmtx` holds one advance height per glyph, so a dictionary asking for two of them is
/// a document no rewrite of the program can satisfy.
///
/// Only §9.7.4.3's displacement `w1` is compared, and deliberately not its position vector `v`:
/// `vmtx` states the first outright and derives nothing about the second, which is the same
/// reading `pdf_archive`'s own rule makes and the reason the two agree about which files fail.
fn shown_heights(
    font: &LoadedFont,
    used: &SelectedFont,
) -> Result<BTreeMap<u16, (f32, Option<f32>)>, Because> {
    let mut heights: BTreeMap<u16, (f32, Option<f32>)> = BTreeMap::new();
    for text in used.shown.keys() {
        for code in font.decode(text) {
            let Some(glyph) = font.glyph_index(code) else {
                continue;
            };
            let (displacement, _) = font.vertical_metrics(code);
            let Some(stated) = displacement.get(1).copied() else {
                continue;
            };
            match heights.entry(glyph) {
                Entry::Vacant(slot) => {
                    slot.insert((stated, font.program_vertical_advance(code)));
                }
                Entry::Occupied(held) if disagrees(held.get().0, stated) => {
                    return Err(Because::TheFence(TWO_HEIGHTS_FOR_ONE_GLYPH));
                }
                Entry::Occupied(_) => {}
            }
        }
    }
    Ok(heights)
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
/// - **A composite font nobody named a program for.** §9.7.4.2 makes a CID an index into the
///   glyphs of the font that defined it, so a substitute cannot be addressed by one at all;
///   section 2.1 is the reading. A program the operator *did* name goes into the descendant
///   `CIDFont`'s descriptor, which is [`composite`]'s and carries its own refusals.
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
    supplied: &BTreeMap<String, std::sync::Arc<[u8]>>,
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
            composite::embed(document, used, spare, supplied, &mut substitutes)?;
            continue;
        }
        // The *raw* entry rather than the resolved one: a program is embedded by writing a key
        // into the descriptor object, so a descriptor written directly inside the font
        // dictionary has no object for this walk to rewrite and is refused by name.
        let descriptor = used
            .dict
            .get("FontDescriptor")
            .and_then(Object::as_reference)
            .ok_or(Because::NotBuiltYet(NO_DESCRIPTOR_TO_EMBED_INTO))?;
        // **The operator's face first, and only where the operator named one.** ISO 19005-2
        // section 6.2.11.4.1 admits a program that may lawfully be embedded for unlimited
        // universal rendering, which ISO 32000-2 §9.9.1 makes a fact about a licence — so it is
        // the operator's to state and never this program's to find. `--font` is the statement,
        // and it is never a default: a font nobody named takes the shipped route exactly as
        // before (`doc/adr/1209`, `doc/questions/A47`).
        let requested = base_font(document, &used.dict);
        let face = match named_program(supplied, &requested) {
            Some((named, program)) => Face::Supplied(
                pdf_font::standard::supplied_face(
                    document,
                    &used.dict,
                    &used.name,
                    std::sync::Arc::clone(program),
                )
                .map_err(|_| Because::NotBuiltYet(SUPPLIED_FACE_NOT_READ))?,
                named,
            ),
            None => Face::Shipped(
                pdf_font::standard::shipped_face(document, &used.dict, &used.name)
                    .map_err(|_| Because::NotBuiltYet(NO_SHIPPED_FACE_COVERS_IT))?,
            ),
        };
        let row = key_for(&subtype, face.format(), face.program())
            .ok_or_else(|| Because::NotBuiltYet(no_row(&subtype, face.format(), face.program())))?;
        let font = LoadedFont::load(document, &used.dict, &used.name)
            .map_err(|_| Because::NotBuiltYet(FONT_NOT_READ))?;
        let (widths, route) = face_widths(&font, &face, used)?;
        let program = if route == MetricRoute::FaceMetrics {
            face.program().to_vec()
        } else {
            pdf_font::restate::with_widths(face.program(), face.format(), &widths)
                .map_err(|_| Because::NotBuiltYet(FACE_NOT_RESTATABLE))?
        };
        // **The invariant the whole supply rests on, proved rather than trusted.** Section 4.9
        // ranks restating the program above restating `/Widths` because `/Widths` is what
        // positions the glyphs; a face whose advances do not read back as the file's numbers
        // would move every line of the page, so the program is asked again through the same two
        // readers a caller holding nothing but bytes has.
        proves(&program, face.format(), &widths, &BTreeMap::new())?;
        let at = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        let stream = program_stream(
            subtype_dictionary(row).as_ref(),
            &program,
            row.key == "FontFile2",
        )
        .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
        substitutes.written.insert(at, stream);
        substitutes
            .at
            .insert(descriptor, Embedding { key: row.key, at });
        substitutes.done.push(SubstitutedFont {
            resource: used.name.clone(),
            requested,
            face: face.describe(),
            authority: face.authority(),
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
    face: &Face,
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
                .ok_or(Because::NotBuiltYet(match face.authority() {
                    FaceAuthority::Shipped => NO_SHIPPED_FACE_COVERS_IT,
                    FaceAuthority::Operator => NO_SUPPLIED_FACE_COVERS_IT,
                }))?;
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
            if pdf_font::restate::advance(face.program(), face.format(), glyph)
                .is_none_or(|own| disagrees(stated, own))
            {
                route = MetricRoute::RestatedProgram;
            }
        }
    }
    Ok((widths, route))
}

/// The program an operator named for one `/BaseFont`, where they named one.
///
/// **The subset tag is passed over, and §9.9.2 is why.** A subset's `/BaseFont` "shall begin
/// with a tag followed by a plus sign (+) followed by the PostScript name of the font from which
/// the subset was created", and the tag "shall consist of exactly six uppercase letters" chosen
/// arbitrarily — so an operator naming a face cannot be expected to know which six letters this
/// document's producer picked, and two subsets of one face in one file have different ones. The
/// exact name is tried first all the same, so an operator who does write the tag gets the font
/// they asked for and nothing else.
///
/// The returned name is what the operator typed, because that is what the report has to say: it
/// is the operator's statement about a licence, and paraphrasing it would be reporting this
/// program's reading of it instead.
fn named_program<'a>(
    supplied: &'a BTreeMap<String, std::sync::Arc<[u8]>>,
    base_font: &str,
) -> Option<(String, &'a std::sync::Arc<[u8]>)> {
    if let Some(program) = supplied.get(base_font) {
        return Some((base_font.to_owned(), program));
    }
    let untagged = subset_tag_removed(base_font)?;
    supplied
        .get(untagged)
        .map(|program| (untagged.to_owned(), program))
}

/// The PostScript name inside a §9.9.2 subset `/BaseFont`, where the name carries a tag.
fn subset_tag_removed(base_font: &str) -> Option<&str> {
    let (tag, name) = base_font.split_once('+')?;
    let six_upper = tag.len() == 6 && tag.bytes().all(|byte| byte.is_ascii_uppercase());
    (six_upper && !name.is_empty()).then_some(name)
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

/// The tables §9.9's Table 124 requires of a program carrying `glyf` outlines.
///
/// The `FontFile2` row: "The font program shall include these tables: "glyf", "head", "hhea",
/// "hmtx", "loca", and "maxp"." The `OpenType` row's first bullet names the same six.
const GLYF_PROGRAM: [[u8; 4]; 6] = [*b"glyf", *b"head", *b"hhea", *b"hmtx", *b"loca", *b"maxp"];

/// One row of §9.9's Table 124: a descriptor key, and the stream `/Subtype` that goes with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Table124 {
    /// Which of §9.9's three keys the descriptor names the program by.
    pub(super) key: &'static str,
    /// The `/Subtype` the stream dictionary states, which Table 125 requires of a `/FontFile3`.
    pub(super) stream_subtype: Option<&'static str>,
}

/// Which of §9.9's keys may carry this program in a dictionary of this subtype.
///
/// **Table 124 decides it and this converter does not**, which is why the match reads as the
/// table's rows do. The table pairs a *key and stream subtype* with the font dictionary
/// `/Subtype`s that may name it and — for the `OpenType` row — with what the program's own bytes
/// contain:
///
/// > • A TrueType font dictionary or a CIDFontType2 CIDFont dictionary, if the embedded font
/// > program contains a "glyf" table.
///
/// > • A CIDFontType0 CIDFont dictionary, if the embedded font program contains a "CFF " table
/// > (notice the trailing SPACE) with a Top DICT that uses CIDFont operators (this is equivalent
/// > to subtype CIDFontType0C ). In addition to the "CFF " table, the font program shall include
/// > the "cmap" table.
///
/// > • A Type1 font dictionary or CIDFontType0 CIDFont dictionary, if the embedded font program
/// > contains a "CFF " table without CIDFont operators. In addition to the "CFF " table, the font
/// > program shall include the "cmap" table.
///
/// Two things the rows say that a reading in a hurry loses. **`MMType1` is in the `Type1C` row
/// and not in the `OpenType` one**, so a multiple-master dictionary takes a bare CFF and not a
/// wrapped one. And **a `glyf` program has no row under a `Type1` dictionary at all** — every key
/// Table 124 opens to a `Type1` dictionary carries CFF — so a face this program ships that
/// happens to be `glyf`-based cannot go there whatever else is true of it.
///
/// Every pairing the table does not state is `None`, which becomes a refusal naming the pairing
/// rather than a file the table does not admit.
fn key_for(subtype: &str, format: Format, program: &[u8]) -> Option<Table124> {
    /// A `/FontFile` or `/FontFile2`, which Table 125 gives no `/Subtype`.
    const fn plain(key: &'static str) -> Table124 {
        Table124 {
            key,
            stream_subtype: None,
        }
    }
    /// A `/FontFile3`, whose stream dictionary names the format (Table 125).
    const fn font_file3(stream_subtype: &'static str) -> Table124 {
        Table124 {
            key: "FontFile3",
            stream_subtype: Some(stream_subtype),
        }
    }
    let sfnt = (format == Format::Sfnt)
        .then(|| pdf_font::embedding::sfnt_tables(program))
        .flatten();
    let glyf = sfnt
        .as_ref()
        .is_some_and(|tables| tables.carries_all(&GLYF_PROGRAM));
    let wrapped_cff = sfnt.as_ref().and_then(|tables| {
        tables
            .cff
            .filter(|_| tables.carries(b"cmap"))
            .map(|keying| keying == pdf_font::embedding::Keying::ByCid)
    });
    let bare_cid = (format == Format::BareCff)
        .then(|| pdf_font::embedding::cff_keying(program, format))
        .flatten()
        .map(|keying| keying == pdf_font::embedding::Keying::ByCid);
    let row = match subtype {
        // One row, two dictionaries: the `FontFile2` row says the key may appear in the font
        // descriptor for a TrueType font dictionary or, since PDF 1.3, for a CIDFontType2 CIDFont
        // dictionary — so these are not two decisions.
        "TrueType" | "CIDFontType2" if glyf => plain("FontFile2"),
        "Type1" | "MMType1" if bare_cid == Some(false) => font_file3("Type1C"),
        "Type1" if wrapped_cff == Some(false) => font_file3("OpenType"),
        // §9.7.4.2 puts both keyings under a `CIDFontType0` dictionary — the charset for a Top
        // DICT that uses CIDFont operators, the CIDs "directly as GID values" for one that does
        // not — and Table 124's `CIDFontType0C` row is the key for a bare program either way.
        "CIDFontType0" if bare_cid.is_some() => font_file3("CIDFontType0C"),
        "CIDFontType0" if wrapped_cff.is_some() => font_file3("OpenType"),
        _ => return None,
    };
    Some(row)
}

/// Which refusal a pairing Table 124 admits no row for earns, which is the pairing itself.
///
/// One case is worth naming apart from the rest, because it is the one an operator is most
/// likely to have walked into and the one a `--font` cannot fix by naming a different file of the
/// same face: **a `glyf`-based program under a `Type1` or `MMType1` dictionary**. Every key Table
/// 124 opens to those two carries Compact Font Format — `/FontFile` a Type 1 program, `/FontFile3`
/// `/Type1C` a bare CFF, `/FontFile3` `/OpenType` an sfnt carrying a `CFF ` table — so no
/// TrueType-flavoured face qualifies however complete it is, and the answer is a CFF-flavoured
/// face rather than another sfnt.
fn no_row(subtype: &str, format: Format, program: &[u8]) -> &'static str {
    let glyf = format == Format::Sfnt
        && pdf_font::embedding::sfnt_tables(program).is_some_and(|tables| tables.carries(b"glyf"));
    match subtype {
        "Type1" | "MMType1" if glyf => GLYF_UNDER_A_TYPE1_DICTIONARY,
        _ => NO_SHIPPED_FORMAT,
    }
}

/// The stream dictionary a `/FontFile3` needs, which §9.9's Table 125 makes `/Subtype` on.
///
/// Table 125 rather than Table 124: the key is the descriptor's and the subtype is the *stream
/// dictionary's*, which is what "Additional entries in an embedded font stream dictionary" lists.
/// Table 126 is §10.6.5's predefined spot functions and names nothing here.
fn subtype_dictionary(row: Table124) -> Option<Dictionary> {
    row.stream_subtype.map(|subtype| {
        let mut dict = Dictionary::new();
        dict.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(subtype.as_bytes())),
        );
        dict
    })
}

/// A font program as a stream, compressed, with the lengths §9.9's Table 125 requires.
///
/// `/Length1` is the program's own length before any filter, which Table 125 makes required of a
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

/// Why a program whose vertical advances are stated where this does not rewrite is not restated.
const VERTICAL_NOT_RESTATABLE: &str = "this font's embedded program states the advance height \
     this conversion would have to restate somewhere it cannot be overwritten in place — a bare \
     CFF program, which carries no vmtx at all, or a glyph in the tail of a vmtx whose advance \
     the table states only by inheritance from the last of its long vertical metrics. Giving \
     that glyph an advance of its own means lengthening the table, which would restate every \
     other glyph in the tail at the same time and none of those was asked for. The alternative \
     is restating /DW2 and /W2 to match the program, and ISO 32000-2 §9.9.1 forbids it outright: \
     a PDF processor shall never use vhea and vmtx, so a dictionary rewritten to agree with them \
     would move every glyph on a vertical line on the authority of a table no reader may read";

/// Why a restatement that did not land where it said it would is withdrawn.
const RESTATEMENT_NOT_PROVED: &str = "the font program this conversion restated does not read \
     back stating the advance it was given for every glyph named, so the rewrite is withdrawn \
     rather than written: a program that missed a site would leave the document failing the \
     requirement it was converted for, and saying nothing about it";

/// Why a document whose dictionary asks two widths of one glyph is refused.
const TWO_WIDTHS_FOR_ONE_GLYPH: &str = "two of this font's codes reach the same glyph and its \
     dictionary states different widths for them. A font program states one advance per glyph, \
     so no program can satisfy both — and the only remedy left would be restating /Widths, which \
     doc/pdf-a-conversion-limits.md section 4.9 forbids because it is what positions the glyphs";

/// Why a document whose dictionary asks two vertical displacements of one glyph is refused.
const TWO_HEIGHTS_FOR_ONE_GLYPH: &str = "two of this font's codes reach the same glyph and its \
     DW2 or W2 entries displace them differently going down the page. A font program states one \
     advance height per glyph, so no program can satisfy both — and the only remedy left would \
     be restating DW2 and W2, which ISO 32000-2 §9.7.4.3 makes what positions a glyph on a \
     vertical line";

/// Why a composite font nobody supplied a program for is not given a substitute.
const COMPOSITE_NOT_SUBSTITUTED: &str = "this document renders a composite font it does not \
     embed, and no program was named for it. ISO 32000-2 §9.7.4.2 makes a CID an index into the \
     glyphs of the font that defined it, so a code of this font names a glyph of a program that \
     is not here and names nothing in any other face — a substitute could be given the right \
     shapes only by deciding which character each code was for, which is the evidence the file \
     never carried. doc/pdf-a-conversion-limits.md section 2.1 is the reading. Supplying the \
     producer's own program resolves it, because its CIDs then index the glyphs they were \
     written for: --font <base-font>=<path> naming either this font's BaseFont or its descendant \
     CIDFont's writes the program into the descendant's own descriptor, against §9.7.4.3's /W \
     and /DW";

/// Why a font with no descriptor is not given a substitute.
const NO_DESCRIPTOR_TO_EMBED_INTO: &str = "this rendered font states no FontDescriptor, which is \
     what §9.6.2.2 lets one of the standard 14 do, and a font program is embedded into a \
     descriptor. Writing one means stating the entries §9.8.1's Table 122 makes required of it — \
     /StemV above all, which is a measurement of a face's stems that nothing in this file states \
     — so the descriptor would be this converter's description of a face rather than the \
     document's. --font names a program and not a descriptor, so it does not reach this case";

/// Why no face covers a document's characters.
const NO_SHIPPED_FACE_COVERS_IT: &str = "this document renders a font it does not embed, and no \
     face this program ships has a glyph for every code it shows. doc/questions/A47 is explicit \
     that where no shipped face covers a document's characters the answer is to refuse rather \
     than guess: embedding a face that draws nothing for those codes would reference the .notdef \
     glyph, which ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid outright. \
     Naming the face with --font <base-font>=<path> resolves it: ISO 19005-2 section 6.2.11.4.1 \
     admits only a program that may lawfully be embedded for unlimited, universal rendering, \
     which ISO 32000-2 §9.9.1 makes a fact about a licence that only you can state";

/// Why a program the operator named is not embedded.
const SUPPLIED_FACE_NOT_READ: &str = "the font program named with --font for this document's \
     BaseFont could not be read as either of the two formats ISO 32000-2 \u{a7}9.9's Table 124 \
     admits under a simple font dictionary - an sfnt (TrueType or OpenType) or a bare Compact \
     Font Format program - or its glyphs could not be matched to the codes this document's own \
     Encoding names. A font collection (ttcf) is refused for the same reason \u{a7}9.9.1 gives \
     for CFF: an embedded font file shall consist of exactly one font";

/// Why a face the operator named that does not cover the document's codes is not embedded.
const NO_SUPPLIED_FACE_COVERS_IT: &str = "the font program named with --font has no glyph for \
     every code this document shows. Embedding it would reference the .notdef glyph for the \
     rest, which ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid outright, \
     so the face is not embedded rather than embedded with holes in it. Naming a face with the \
     document's whole repertoire resolves it";

/// Why a face of the wrong format for the dictionary is not embedded.
const NO_SHIPPED_FORMAT: &str = "the face this conversion would embed is not in a format ISO \
     32000-2 §9.9's Table 124 admits under this font dictionary's own Subtype. Every pairing the \
     table states is written here — a glyf-based sfnt into a /FontFile2 under a TrueType or \
     CIDFontType2 dictionary, a bare Compact Font Format program into a /FontFile3 with Subtype \
     Type1C under a Type1 or MMType1 one and with Subtype CIDFontType0C under a CIDFontType0 \
     one, and an sfnt carrying a CFF table and a cmap into a /FontFile3 with Subtype OpenType \
     under a Type1 or CIDFontType0 one — and this program and this dictionary are none of them. \
     Naming a program of the format the table admits with --font <base-font>=<path> resolves it";

/// Why a `glyf`-based face is not embedded under a `Type1` or `MMType1` dictionary.
const GLYF_UNDER_A_TYPE1_DICTIONARY: &str = "this document states a Type1 or MMType1 font \
     dictionary, and the face that would be embedded into it is a TrueType-flavoured sfnt — one \
     carrying a glyf table. ISO 32000-2 §9.9's Table 124 opens three keys to those two \
     dictionaries and all three carry Compact Font Format: /FontFile a Type 1 program, \
     /FontFile3 with Subtype Type1C a bare CFF, and /FontFile3 with Subtype OpenType an sfnt \
     whose CFF table has a Top DICT without CIDFont operators. A glyf-based face is admitted by \
     none of them, however complete it is, so naming a different file of the same face does not \
     resolve it — naming a CFF-flavoured one with --font <base-font>=<path> does";

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
