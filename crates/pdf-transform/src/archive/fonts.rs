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

mod bystanders;
mod cff_notdef_glyph;
mod cid_cff_notdef_glyph;
mod cid_notdef_glyph;
mod composite;
mod described;
pub(in crate::archive) mod embedded_cmap;
mod empty;
mod fresh_cid;
mod notdef_format;
mod notdef_glyph;
mod notices;
mod pack;
mod photographed;
mod rekeyed;
mod truetype_cmap;
mod twin_codes;
mod untagged;

pub use pack::{CjkPack, Collection, PackFiles, PackGap};

pub use described::Described;
pub(super) use described::{state_laid_out_widths, undrawn};
pub(super) use notdef_format::notdef_not_given;
pub use photographed::{
    DOTS_PER_INCH, FixedGroup, FixedGroups, MAX_GROUP_NAME_CHARS, MAX_GROUPS_NAMED,
    MAX_PHOTOGRAPHED_PAGES, MAX_TEXT_CHARS, PhotographedPage,
};
pub(super) use photographed::{Gate, elided, leaves_named};
pub(super) use truetype_cmap::ANSWERED as TRUETYPE_CMAP_ROWS;
pub(super) use truetype_cmap::NOTDEF_ROWS as TRUETYPE_NOTDEF_ROWS;
pub(super) use truetype_cmap::STANDARD_ROUTE_ROWS as TRUETYPE_ENCODING_ROWS;
pub(super) use truetype_cmap::{CmapRestated, restated as truetype_cmaps_restated};
pub(super) use truetype_cmap::{NOTDEF_NOT_GIVEN, notdef_fonts_have_truetype_programs};

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::{SelectedFont, Survey};
use pdf_font::standard::{ShippedFace, SuppliedFace};
use pdf_font::substitute::Format;
use pdf_font::{Code, LoadedFont};
use pdf_model::content::ledger::Outcome;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use super::COMPRESSION_LEVEL;
use super::decision::{Because, Losses};
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
/// [`super::Conversion::recorded`] — an `xmpMM:History` entry saying so. Beside them, what
/// embedding it lost, so that the report and the history name every loss per font.
// `Eq` is deliberately absent: `fitting` is the reader's own horizontal scale, a real number, and
// a type holding one has no reflexive equality to claim (`doc/adr/1210`'s reason).
#[derive(Debug, Clone, PartialEq)]
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
    /// What this conversion wrote around the program for a font that stated no descriptor, or
    /// `None` where the file's own descriptor received it (`doc/adr/2146`).
    pub described: Option<Described>,
    /// What embedding this font lost: `machine-face` where the viewer drew it from a face of this
    /// machine's (`doc/adr/2162`), `substitute-stretch` where it drew the shipped face narrowed to
    /// the file's widths (`doc/adr/2262`), and nothing where the output draws what the viewer did.
    pub losses: Losses,
    /// The horizontal scale the viewer drew the shipped face at, where it drew one, and what
    /// embedding did with it.
    pub fitting: Option<Fitting>,
    /// The codes the file's own encoding sent to `.notdef` that were given an empty glyph in the
    /// embedded program, which `doc/questions/A48` allows on condition that each is reported and
    /// recorded (`doc/adr/2458`).
    pub empty_glyphs: Vec<u8>,
    /// The codes Annex D encodes twice that the font's encoding now names for the character ISO
    /// 32000-1 says each means, so that each reaches a glyph of its own (`doc/adr/2723`).
    pub twin_codes: Vec<u8>,
    /// The program the file held only the front of, which this substitution dropped, where the
    /// font had one (`doc/adr/2636`).
    pub cut_program: Option<CutProgram>,
    /// The font this entry substituted, as a content stream's lookup finds it — its object, or
    /// its dictionary where a resource dictionary states it directly — so that an entry whose
    /// font no object of the output holds is not reported (`doc/adr/2454`).
    pub(crate) font: Outcome,
}

/// A font program the file does not hold whole — its decode stops short of the data its filters
/// describe, or it decodes whole and is not a program in the format it is filed under — dropped
/// so that a face takes the font's place (`doc/adr/2636`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutProgram {
    /// The descriptor key the program stood under: `FontFile`, `FontFile2` or `FontFile3`.
    pub key: &'static str,
    /// Where the decode stopped — [`pdf_syntax::filter::Damage::Truncated`],
    /// [`pdf_syntax::filter::Damage::Corrupt`] or [`pdf_syntax::filter::Damage::CheckValue`] —
    /// or `None` where it decoded whole and this reader's parse of the program's format refused
    /// the bytes ([`pdf_archive::program_unparsed`]).
    pub damage: Option<pdf_syntax::filter::Damage>,
}

impl CutProgram {
    /// What was dropped and why, in one clause, for the report and for the file's own
    /// `xmpMM:History` alike.
    #[must_use]
    pub fn describe(self) -> String {
        match self.damage {
            Some(damage) => format!(
                "its {} stream's data does not decode whole ({damage:?}), so the file held only the \
                 front of its program, which ISO 32000-2 section 9.9 does not count as embedded; \
                 that front was removed, and this reader drew none of its glyphs",
                self.key
            ),
            None => format!(
                "its {} stream's data decodes whole and is not a program in the format it is filed \
                 under, which ISO 32000-2 section 9.9 does not count as embedded; those bytes were \
                 removed, and this reader drew none of its glyphs",
                self.key
            ),
        }
    }
}

/// What an empty glyph given in place of `.notdef` is, in one clause naming the codes and the
/// clause, for the report and for the file's own `xmpMM:History` alike (`doc/questions/A48`).
#[must_use]
pub(super) fn empty_glyphs_given(codes: &[u8]) -> String {
    format!(
        "the codes {} reached .notdef under the file's own encoding, which ISO 19005-2 section \
         6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator to reference, \
         and were each given a glyph that draws nothing at the width the font dictionary states, \
         named outside the Adobe Glyph List so that no character is stated for them",
        empty::listed(codes)
    )
}

/// What naming a font's twin codes restated, for the report and the file's own `xmpMM:History`
/// alike (`doc/adr/2723`).
#[must_use]
pub(super) fn twin_codes_named(codes: &[u8]) -> String {
    twin_codes::described(codes)
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

/// What a substitution did with the horizontal scale the viewer drew the shipped face at
/// (ADR 0358).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fitting {
    /// Embedding the face unmodified removed it: the figure the loss `substitute-stretch` is
    /// about (`doc/adr/2262`).
    Removed(f32),
    /// It was written into the embedded program's outlines, so that the output draws what the
    /// viewer did and nothing is lost (`doc/adr/2769`).
    Written(f32),
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

/// The requirement the embedding answers.
pub(super) const EMBEDDED_PROGRAMS: &str = "fonts/font-programs-embedded";

/// The font programs to embed, and the fonts they belong to.
#[derive(Debug, Default)]
pub(super) struct Substitutes {
    /// The font objects this conversion itself added, which `gains` writes whole into `written`
    /// where it would otherwise restate an object of the source (`doc/adr/2522`).
    added_fonts: BTreeSet<ObjectId>,
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
    /// The entries each font dictionary that stated no descriptor gains — `/FontDescriptor`, the
    /// widths it is laid out by, and where its face asks for one a `TrueType` `/Subtype` and an
    /// encoding §9.6.5.4 admits (`doc/adr/2146`).
    pub(super) described: BTreeMap<ObjectId, Dictionary>,
    /// Each font written directly inside a resource dictionary that gains entries, and the object
    /// it is written out as, entries included; the rewriter puts a reference to that object where
    /// the dictionary stood (`doc/adr/2162`).
    pub(super) direct: Vec<(Dictionary, ObjectId)>,
    /// The program stream each shipped face was written to, so that one document's fonts naming
    /// the same face share one copy of it.
    pub(super) programs: BTreeMap<&'static str, ObjectId>,
    /// What the CJK pack's route wrote into each descriptor, so that a font met again widens the
    /// one subset rather than writing a second (`doc/adr/2302`).
    packed: BTreeMap<ObjectId, pack::Packed>,
    /// What the re-keying route wrote into each descriptor, so that a font met again widens the
    /// one program rather than writing a second (`doc/adr/2378`).
    rekeyed: BTreeMap<ObjectId, rekeyed::Rekeyed>,
    /// The `Adobe-Identity` fonts no face can be keyed to, held until the pages that show them are
    /// known (`doc/adr/2394`).
    deferred: Vec<photographed::Deferred>,
    /// The content and resources each photographed page object is given, by the page's object.
    pub(super) photographs: BTreeMap<ObjectId, photographed::Photograph>,
    /// Every page photographed, for the report and the file's history.
    pub(super) photographed: Vec<PhotographedPage>,
    /// The content-stream requirements whose every page this conversion photographed, which the
    /// decision of each reads to know the page it fails on is no longer in the output.
    pub(super) fenced: BTreeSet<&'static str>,
}

impl Substitutes {
    /// Every loss the embedding incurs, over every font it embedded a face into: what the one
    /// decision at `fonts/font-programs-embedded` carries (`doc/adr/2262`).
    pub(super) fn losses(&self) -> Losses {
        let losses = self
            .done
            .iter()
            .flat_map(|font| font.losses.iter())
            .fold(Losses::default(), Losses::with);
        self.photograph_losses().iter().fold(losses, Losses::with)
    }

    /// Every loss the photographed pages incur: `text-as-image` for any, `page-as-image` where the
    /// wider gate admitted one, and `tagging-of-photographed-pages` where one lost a leaf of the
    /// document's structure tree (`doc/questions/A707`).
    pub(super) fn photograph_losses(&self) -> Losses {
        self.photographed
            .iter()
            .flat_map(PhotographedPage::losses)
            .fold(Losses::default(), Losses::with)
    }

    /// What the file's `xmpMM:History` states the embedding and the photographs did.
    pub(super) fn recorded(&self) -> String {
        let fonts = super::prepare::substituted_fonts_recorded(&self.done);
        let pages = photographed::recorded(&self.photographed);
        match (fonts.is_empty(), pages.is_empty()) {
            (_, true) => fonts,
            (true, false) => pages,
            (false, false) => format!("{fonts}; {pages}"),
        }
    }

    /// Whether this font is a direct dictionary equal to one already written out as an object.
    ///
    /// The survey keys a direct font by the page and the name it was selected by, so one
    /// dictionary inherited by many pages is many selections of the same value; it is written out
    /// once, and every place it stood is given the one reference (`doc/adr/2162`).
    fn wrote_out(&self, used: &SelectedFont) -> bool {
        used.id.is_none() && self.direct.iter().any(|(written, _)| *written == used.dict)
    }
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
    /// Notice streams allocated for rewritten programs.
    pub(super) written: BTreeMap<ObjectId, Object>,
    /// The replacement stream for each font program object.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// The programs whose `/Widths`, `/W` or `/DW` disagreement was answered.
    pub(super) horizontal: BTreeSet<ObjectId>,
    /// The programs whose `/DW2` or `/W2` disagreement was answered.
    pub(super) vertical: BTreeSet<ObjectId>,
    /// The dictionaries whose `/FontDescriptor` now names a copy of the descriptor they shared,
    /// each with that copy and the restated program it names, both among [`Self::written`].
    pub(super) descriptors: BTreeMap<ObjectId, (ObjectId, ObjectId)>,
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

impl Embedding {
    /// Writes the one program entry, removing descriptions of the formerly absent program.
    pub(super) fn write_into(self, out: &mut Dictionary) {
        // §9.9's Table 124 admits at most one of the three keys:
        //
        // > At most, only one of the FontFile , FontFile2 , and FontFile3 entries shall be
        // > present.
        //
        // This descriptor *states* none — the preparation's population is a descriptor whose
        // every such key resolves to null — but it may well hold one of the keys with
        // nothing behind it, which §7.3.7 makes the same thing and Table 124's sentence
        // counts all the same. So the other two go before this one is written.
        for key in ["FontFile", "FontFile2", "FontFile3"] {
            if key != self.key {
                out.remove(key);
            }
        }
        // **`/CharSet` and `/CIDSet` describe a program, and the program they describe was
        // never in this file.** §9.8.1's Table 122 makes both optional and makes each a list
        // of what the *embedded* program contains; ISO 19005-2 section 6.2.11.4.2 then
        // requires the list to be complete. A producer who wrote one for a font they did not
        // embed wrote a description of a font nobody had, so carrying it forward beside a
        // face this converter chose would leave the file asserting something false about its
        // own bytes — and would fail the very clause that checks the assertion, which is how
        // the corpus found this. Removing a description of an absent program takes nothing
        // from the document that a reader could have used.
        out.remove("CharSet");
        out.remove("CIDSet");
        out.insert(Name::new(self.key.as_bytes()), Object::Reference(self.at));
    }
}

/// Where one font's program is, and what reads it.
struct Embedded {
    /// The stream object the key names.
    at: ObjectId,
    /// Which reader the bytes belong to.
    format: Filed,
    /// Whether §9.9's Table 124 makes `/Length1` part of the entry, which a rewrite must restate.
    length1: bool,
}

/// The format of a program the file embeds, as §9.9's Table 124 files it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filed {
    /// An sfnt or a bare CFF, which [`pdf_font::restate::with_widths`] restates.
    Read(Format),
    /// A Type 1 program under `/FontFile`, in the three sections §9.9.1's Table 125 measures,
    /// which [`pdf_font::restate::type1_with_widths`] restates (`doc/adr/2685`).
    Type1,
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
    spare: &mut Spare,
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
        let Object::Stream(stream) = document.get(embedded.at) else {
            return Err(Because::NotBuiltYet(PROGRAM_NOT_DECODED));
        };
        let (restated, lengths) = match embedded.format {
            Filed::Read(format) => {
                let mut restated = bytes.to_vec();
                if !widths.is_empty() {
                    restated = pdf_font::restate::with_widths(&restated, format, &widths)
                        .map_err(|_| Because::NotBuiltYet(PROGRAM_NOT_RESTATABLE))?;
                }
                if !heights.is_empty() {
                    restated =
                        pdf_font::restate::with_vertical_advances(&restated, format, &heights)
                            .map_err(|_| Because::NotBuiltYet(VERTICAL_NOT_RESTATABLE))?;
                }
                restated = renamed_modified(&restated, format)?;
                proves(&restated, format, &widths, &heights)?;
                (restated, None)
            }
            Filed::Type1 => {
                let restated = type1_restated(document, &stream.dict, &bytes, &widths, &heights)?;
                (restated.program, Some(restated.lengths))
            }
        };
        let mut stream = program_stream(Some(&stream.dict), &restated, embedded.length1)
            .and_then(|written| cff_notdef_glyph::with_lengths(written, lengths))
            .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
        notices::attach(document, &bytes, &mut stream, spare, &mut metrics.written)?;
        // **A program another font shares is restated in a copy where restating it in place would
        // change what the other font draws**: §9.6.3's widths of the other dictionary agreed with
        // the program as it was, and ISO 19005-2 section 6.2.11.5 would fail them against the new
        // advances. The copy is this font's own descriptor and program, the file's own objects
        // with the advances restated, so nothing a page shows is new (`doc/adr/2640`).
        let shares = metrics.at.contains_key(&embedded.at)
            || another_font_differs(
                document, survey, used, &embedded, &widths, &heights, &metrics,
            );
        let at = if shares {
            copy_on_write(document, used, &embedded, stream, spare, &mut metrics)?
        } else {
            metrics.at.insert(embedded.at, stream);
            embedded.at
        };
        if !widths.is_empty() {
            metrics.horizontal.insert(at);
        }
        if !heights.is_empty() {
            metrics.vertical.insert(at);
        }
        metrics.done.push(RestatedFont {
            resource: used.name.clone(),
            requested: base_font(document, &used.dict),
            glyphs: widths.len(),
            heights: heights.len(),
        });
    }
    if metrics.at.is_empty() && metrics.descriptors.is_empty() {
        return Err(Because::NotBuiltYet(NO_PROGRAM_TO_RESTATE));
    }
    Ok(metrics)
}

/// Whether a font other than `used` draws from the same program and states, for a glyph it shows,
/// an advance the restated `widths` or `heights` contradict.
///
/// A font that cannot be read is counted as one that differs: whether restating the program in
/// place would break it is then unknown, and the copy costs two objects where the guess could
/// cost the other font its agreement.
fn another_font_differs(
    document: &Document,
    survey: &Survey,
    used: &SelectedFont,
    embedded: &Embedded,
    widths: &BTreeMap<u16, f32>,
    heights: &BTreeMap<u16, f32>,
    metrics: &Metrics,
) -> bool {
    survey.fonts().any(|other| {
        let same = match (other.id, used.id) {
            (Some(left), Some(right)) => left == right,
            _ => other.name == used.name && other.dict == used.dict,
        };
        let redirected = holder_of(document, other)
            .is_some_and(|holder| metrics.descriptors.contains_key(&holder));
        let draws_from_it = descriptor_dict(document, other)
            .and_then(|descriptor| embedded_program(document, &descriptor))
            .is_some_and(|program| program.at == embedded.at);
        if same || redirected || !draws_from_it {
            return false;
        }
        let Some((font, _)) = own_program(document, other) else {
            return true;
        };
        let contradicts = |stated: Result<BTreeMap<u16, (f32, Option<f32>)>, Because>,
                           restated: &BTreeMap<u16, f32>| {
            stated.map_or(true, |stated| {
                stated.iter().any(|(glyph, (stated, _))| {
                    restated
                        .get(glyph)
                        .is_some_and(|restated| disagrees(*stated, *restated))
                })
            })
        };
        (!widths.is_empty() && contradicts(shown_widths(&font, other), widths))
            || (!heights.is_empty()
                && font.is_vertical()
                && contradicts(shown_heights(&font, other), heights))
    })
}

/// Writes `stream` into a fresh object, with a copy of `used`'s descriptor naming it, and points
/// the dictionary that names the descriptor at the copy; the program's own object is left to the
/// fonts that still share it. Returns the fresh program's object.
fn copy_on_write(
    document: &Document,
    used: &SelectedFont,
    embedded: &Embedded,
    stream: Object,
    spare: &mut Spare,
    metrics: &mut Metrics,
) -> Result<ObjectId, Because> {
    let not_copied = Because::NotBuiltYet(SHARED_PROGRAM_NOT_COPIED);
    let holder = holder_of(document, used).ok_or(not_copied)?;
    let descriptor = descriptor_of(document, used).ok_or(not_copied)?;
    let mut copy = document
        .get(descriptor)
        .as_dict()
        .cloned()
        .ok_or(not_copied)?;
    let key = ["FontFile", "FontFile2", "FontFile3"]
        .into_iter()
        .find(|key| copy.get(key).and_then(Object::as_reference) == Some(embedded.at))
        .ok_or(not_copied)?;
    let program = spare.take(document).ok_or(not_copied)?;
    let copied = spare.take(document).ok_or(not_copied)?;
    copy.insert(Name::new(key.as_bytes()), Object::Reference(program));
    metrics.written.insert(program, stream);
    metrics.written.insert(copied, Object::Dictionary(copy));
    metrics.descriptors.insert(holder, (copied, program));
    Ok(program)
}

/// The indirect dictionary that names a rendered font's descriptor: the font's own for a simple
/// font, its one descendant's for a `Type0` (§9.7.6.2), where either is an indirect object.
fn holder_of(document: &Document, used: &SelectedFont) -> Option<ObjectId> {
    if name_of(document, &used.dict, "Subtype").as_deref() != Some("Type0") {
        return used.id;
    }
    let array = document.get_key(&used.dict, "DescendantFonts");
    document
        .resolve(&array)
        .as_array()?
        .first()
        .and_then(Object::as_reference)
}

/// Reads the rewritten program back, and refuses it unless every glyph named actually moved.
///
/// **ADR 0988's rule, and it is a rule because a rewrite that silently missed a site would
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

/// A Type 1 program with its advances restated and proved, and the three lengths its stream is
/// to state (`doc/adr/2685`).
///
/// Adobe Type 1 Font Format section 6.4 puts a glyph's advance in the `hsbw` or `sbw` that opens
/// its charstring, which [`pdf_font::restate::type1_with_widths`] rewrites. A Type 1 program is a
/// simple font's, which §9.7.4.3's vertical metrics never reach, so `heights` is always empty; a
/// non-empty one is refused rather than dropped. The stream's `/Length1` and `/Length2` are what
/// divide the decoded program into its sections, so a stream that states neither is refused as
/// one whose sections cannot be found.
///
/// A dictionary stating a `/FontDescriptor` and not all of `/FirstChar`, `/LastChar` and
/// `/Widths` is restated too: `fonts/simple-font-states-its-widths` writes those three entries
/// as the advances this reader lays the glyphs by, which are the ones `widths` holds the program
/// to, so the dictionary and the program agree once both are written (`doc/adr/2686`).
fn type1_restated(
    document: &Document,
    stream: &Dictionary,
    bytes: &[u8],
    widths: &BTreeMap<u16, f32>,
    heights: &BTreeMap<u16, f32>,
) -> Result<pdf_font::type1::AdvancesRestated, Because> {
    if !heights.is_empty() {
        return Err(Because::NotBuiltYet(VERTICAL_NOT_RESTATABLE));
    }
    let stated = |key: &str| usize::try_from(document.get_key(stream, key).as_integer()?).ok();
    let lengths = stated("Length1")
        .zip(stated("Length2"))
        .ok_or(Because::NotBuiltYet(PROGRAM_NOT_RESTATABLE))?;
    let restated = pdf_font::restate::type1_with_widths(bytes, lengths, widths)
        .map_err(|_| Because::NotBuiltYet(PROGRAM_NOT_RESTATABLE))?;
    for (glyph, width) in widths {
        let stated = pdf_font::restate::type1_advance(&restated.program, *glyph)
            .ok_or(Because::NotBuiltYet(RESTATEMENT_NOT_PROVED))?;
        if disagrees(stated, *width) {
            return Err(Because::NotBuiltYet(RESTATEMENT_NOT_PROVED));
        }
    }
    Ok(restated)
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
                    slot.insert((stated, program_statement(font, code, stated)));
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

/// The program's statement of a code's advance that the dictionary's `stated` is judged by.
///
/// `hmtx`'s, unless it agrees and an `OpenType` program's `CFF ` table states a width that does
/// not: `pdf_archive`'s width rule asks both of the program's statements, so a glyph either
/// disagrees with is one to restate, and [`pdf_font::restate::with_widths`] restates both.
fn program_statement(font: &LoadedFont, code: Code, stated: f32) -> Option<f32> {
    let program = font.program_advance(code)?;
    if disagrees(stated, program) {
        return Some(program);
    }
    Some(
        font.program_charstring_advance(code)
            .filter(|charstring| disagrees(stated, *charstring))
            .unwrap_or(program),
    )
}

/// Whether two statements of one advance are further apart than ISO 19005 allows.
///
/// ISO 19005-2 section 6.2.11.5 admits a difference of exactly one unit at the usual
/// `0.001` matrix, and `f32` stores 0.544 and 0.543 0.00100005 apart; the margin is the
/// operands' own rounding, four `f32` epsilons of each magnitude, as the validator allows it.
fn disagrees(stated: f32, program: f32) -> bool {
    let margin =
        4.0 * f64::from(f32::EPSILON) * (f64::from(stated.abs()) + f64::from(program.abs()));
    (f64::from(stated) - f64::from(program)).abs() > f64::from(CONSISTENT) + margin
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
/// - **A font with no descriptor** is the standard 14 stated by name alone, and [`described`]
///   writes the descriptor, the widths and the program the viewer draws it with, or refuses by
///   name where the result would not be the page this reader shows (`doc/adr/2146`); a font
///   dictionary written directly inside its resource dictionary is written out as an object of
///   its own to hold them ([`gains`], `doc/adr/2162`). A descriptor written directly inside the
///   font dictionary is no object to add a key to, and is refused by name.
/// - **A dictionary whose subtype the face's format may not go in**, which §9.9's Table 124
///   decides rather than this converter.
/// - **A code the face has no glyph for**, which is `doc/questions/A47`'s "refuse rather than
///   guess" and is what keeps both parts' `.notdef` clause satisfied without anything being
///   invented.
///
/// # Photographing around a font (A694 item 1)
///
/// Where the gate is `anywhere`, a font that any of the routes below refuses — except one whose
/// refusal is about the font's own widths — is held for the photographed page instead, and the
/// pass is made again without it: a route that failed part-way has written objects for the font
/// that nothing should now point at, and a pass that never began the font writes none. At most
/// [`MAX_FONTS_PHOTOGRAPHED_AROUND`] fonts are held that way, so a document of hostile fonts costs
/// that many passes and no more.
pub(super) fn embed_faces(
    document: &Document,
    survey: &Survey,
    spare: &mut Spare,
    routes: &Routes<'_>,
) -> Result<Substitutes, Because> {
    let gate = routes.gate;
    let mut around: BTreeMap<Outcome, Because> = BTreeMap::new();
    loop {
        let mut failing = None;
        let pass = embed_pass(document, survey, spare, routes, &around, &mut failing);
        let Err(refusal) = pass else {
            return pass;
        };
        let held = gate.offered
            && gate.anywhere
            && around.len() < MAX_FONTS_PHOTOGRAPHED_AROUND
            && photographable(refusal);
        let font = failing.and_then(|index| survey.fonts().nth(index));
        match font {
            Some(used) if held => {
                around.insert(selected(used), refusal);
            }
            _ => return Err(refusal),
        }
    }
}

/// What the routes of [`embed_pass`] are given besides the document.
#[derive(Clone, Copy)]
pub(super) struct Routes<'a> {
    /// The programs the operator named.
    pub(super) supplied: &'a BTreeMap<String, std::sync::Arc<[u8]>>,
    /// The CJK pack, where one was found.
    pub(super) pack: Option<&'a CjkPack>,
    /// The target the conversion is for.
    pub(super) target: pdf_archive::Target,
    /// What decides whether pages are photographed.
    pub(super) gate: Gate<'a>,
    /// The standard-fourteen definitions the conversion itself added, by name.
    pub(super) defined: &'a BTreeMap<Vec<u8>, ObjectId>,
    /// Whether the viewer's fitting of a shipped face is written into the program embedded
    /// (`doc/adr/2769`): so where the profile withholds the loss `substitute-stretch`, and not
    /// where it authorises it, which keeps the unmodified face `doc/questions/A566` chose.
    pub(super) write_fitting: bool,
}

/// How many fonts one conversion photographs around ([`embed_faces`]).
const MAX_FONTS_PHOTOGRAPHED_AROUND: usize = 16;

/// Whether a refusal of one font is one a photograph of the pages showing it answers.
///
/// Not a font whose widths the file states twice (`TWO_WIDTHS_FOR_ONE_GLYPH`), nor one whose
/// descriptor is written inside the font dictionary: those are the font refiling of the list's
/// R10, a different remedy that photographing would hide rather than answer, and `A694` item 1
/// names R08 and R03 alone.
fn photographable(refusal: Because) -> bool {
    !matches!(refusal, Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH))
        && !matches!(refusal, Because::NotBuiltYet(DESCRIPTOR_NOT_AN_OBJECT))
}

/// One pass over the rendered fonts, leaving in `failing` the position in the survey's order of the
/// font a refusal came from, and `None` for a refusal that is no font's.
#[expect(
    clippy::too_many_lines,
    reason = "one ordered preparation: select, prove, allocate, attach and report each font"
)]
fn embed_pass(
    document: &Document,
    survey: &Survey,
    spare: &mut Spare,
    routes: &Routes<'_>,
    around: &BTreeMap<Outcome, Because>,
    failing: &mut Option<usize>,
) -> Result<Substitutes, Because> {
    let Routes {
        supplied,
        pack,
        target,
        gate,
        defined,
        write_fitting,
    } = *routes;
    let mut substitutes = Substitutes::default();
    let mut drawn_whole = false;
    // **The standard fourteen this conversion itself defined**, each an object of its own that
    // every definition of its name references: the face is embedded once for the name, from the
    // text drawn under it in every stream (`doc/adr/2522`).
    for drawn in survey.undefined_standard_fonts() {
        let Some(id) = defined.get(drawn.name.as_bytes()) else {
            continue;
        };
        let used = SelectedFont {
            id: Some(*id),
            ..drawn.clone()
        };
        substitutes.added_fonts.insert(*id);
        described::describe(
            document,
            &used,
            spare,
            &mut substitutes,
            (target, write_fitting),
        )?;
    }
    for (index, used) in survey.fonts().enumerate() {
        if !used.rendered {
            continue;
        }
        let cut = if needs_a_program(document, &used.dict) {
            None
        } else {
            match cut_program(document, used) {
                Shortfall::Cut(cut) => Some(cut),
                Shortfall::DrawnWhole => {
                    drawn_whole = true;
                    continue;
                }
                Shortfall::None => continue,
            }
        };
        if substitutes.wrote_out(used) {
            continue;
        }
        if !around.is_empty()
            && let Some(refusal) = around.get(&selected(used))
        {
            photographed::defer(document, used, *refusal, true, &mut substitutes);
            continue;
        }
        *failing = Some(index);
        let subtype = name_of(document, &used.dict, "Subtype").unwrap_or_default();
        if subtype == "Type3" {
            continue;
        }
        if subtype == "Type0" {
            let before = substitutes.done.len();
            composite::embed(document, used, spare, supplied, pack, &mut substitutes)?;
            for entry in substitutes.done.iter_mut().skip(before) {
                entry.cut_program = cut;
            }
            continue;
        }
        let requested = base_font(document, &used.dict);
        if document.get_key(&used.dict, "FontDescriptor").is_null() {
            // The descriptor written here describes the face this program ships, so a program the
            // operator named is not one it can stand beside.
            if named_program(supplied, &requested).is_some() {
                return Err(Because::NotBuiltYet(NO_DESCRIPTOR_FOR_A_SUPPLIED_PROGRAM));
            }
            described::describe(
                document,
                used,
                spare,
                &mut substitutes,
                (target, write_fitting),
            )?;
            continue;
        }
        // The *raw* entry rather than the resolved one: a program is embedded by writing a key
        // into the descriptor object, so a descriptor written directly inside the font
        // dictionary has no object for this walk to rewrite and is refused by name.
        let descriptor = used
            .dict
            .get("FontDescriptor")
            .and_then(Object::as_reference)
            .ok_or(Because::NotBuiltYet(DESCRIPTOR_NOT_AN_OBJECT))?;
        // Every reading of the font from here on is of the font a cut program leaves: the
        // descriptor's names and the dictionary's widths, and no program (`doc/adr/2636`). The
        // writes go to the font and the descriptor the file holds.
        let dropped = cut.map(|_| program_dropped(document, used));
        let reading = dropped.as_ref().unwrap_or(used);
        // **The operator's face first, and only where the operator named one.** ISO 19005-2
        // section 6.2.11.4.1 admits a program that may lawfully be embedded for unlimited
        // universal rendering, which ISO 32000-2 §9.9.1 makes a fact about a licence — so it is
        // the operator's to state and never this program's to find. `--font` is the statement,
        // and it is never a default: a font nobody named takes the shipped route exactly as
        // before (`doc/adr/1209`, `doc/questions/A47`).
        let face = face_for(document, supplied, &requested, reading)?;
        let (row, restated) = row_for(document, reading, &subtype, &face)?;
        let font = LoadedFont::load(document, &reading.dict, &reading.name)
            .map_err(|_| Because::NotBuiltYet(FONT_NOT_READ))?;
        // **A code Annex D encodes twice, at a width of its own**: named for the character ISO
        // 32000-1 says it means, it reaches the face's glyph for that character, and the face
        // is read again under the restated encoding (`twin_codes`, `doc/adr/2723`). Not beside a
        // restatement of the dictionary's kind, which writes an encoding of its own.
        let (face, font, found, twins) = match face_widths(&font, &face, reading, &[]) {
            Ok(found) => (face, font, found, None),
            Err(refusal @ Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH)) if restated.is_none() => {
                let twins = twin_codes::renamed(
                    document,
                    &reading.dict,
                    &shown_bytes(&font, reading),
                    |code| font.advance(Code::single_byte(code)),
                    target,
                )
                .ok_or(refusal)?;
                let renamed = SelectedFont {
                    dict: twins.dict.clone(),
                    ..reading.clone()
                };
                let face = face_for(document, supplied, &requested, &renamed)?;
                let font = LoadedFont::load(document, &renamed.dict, &renamed.name)
                    .map_err(|_| Because::NotBuiltYet(FONT_NOT_READ))?;
                let found =
                    face_widths(&font, &face, &renamed, &twins.codes).map_err(|_| refusal)?;
                (face, font, found, Some(twins))
            }
            Err(refusal) => return Err(refusal),
        };
        let FaceWidths {
            mut widths,
            route,
            splits,
        } = found;
        // A renamed code that still reaches its twin's glyph is given a copy of it (`twin_codes`).
        let copied = if splits.is_empty() {
            None
        } else {
            Some(
                split_twins(&face, &splits, &mut widths)
                    .ok_or(Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH))?,
            )
        };
        // Which face the viewer drew the font from decides what embedding the shipped one loses:
        // the restatement's proof has already held the font to it (`doc/adr/2162`). The unrestated
        // route, whose own comparison is of advances, reads the same answer off the font as
        // loaded: the fitting it removes from the shipped face (`doc/adr/2262`), or a machine's
        // face it embeds over, which is the loss `machine-face` there exactly as on the other two
        // routes (`doc/adr/2279`). An operator's face is the operator's statement and costs
        // neither.
        let drawn = match (&face, &restated) {
            (Face::Supplied(..), _) => None,
            (Face::Shipped(_), Some(restatement)) => Some(restatement.drawn),
            (Face::Shipped(shipped), None) => described::drawn_by_the_viewer(&font, shipped),
        };
        // The fitting written into the face where the profile keeps it (`doc/adr/2769`), before
        // the advances are restated: the scale moves outlines only, and the advances are then
        // made the file's widths exactly as for a face embedded unscaled.
        let base = copied.as_deref().unwrap_or(face.program());
        let fitting_written =
            described::with_the_fitting(base, face.format(), drawn, &font, write_fitting);
        let program = match (route, &fitting_written) {
            (MetricRoute::FaceMetrics, None) => face.program().to_vec(),
            (MetricRoute::FaceMetrics, Some((scaled, _))) => {
                renamed_modified(scaled, face.format())?
            }
            (_, scaled) => {
                let base = scaled
                    .as_ref()
                    .map_or(base, |(scaled, _)| scaled.as_slice());
                let modified = pdf_font::restate::with_widths(base, face.format(), &widths)
                    .map_err(|_| Because::NotBuiltYet(FACE_NOT_RESTATABLE))?;
                renamed_modified(&modified, face.format())?
            }
        };
        // **The invariant the whole supply rests on, proved rather than trusted.** Section 4.9
        // ranks restating the program above restating `/Widths` because `/Widths` is what
        // positions the glyphs; a face whose advances do not read back as the file's numbers
        // would move every line of the page, so the program is asked again through the same two
        // readers a caller holding nothing but bytes has.
        proves(&program, face.format(), &widths, &BTreeMap::new())?;
        if let Some(restatement) = &restated {
            described::restatement_proved(
                document,
                reading,
                restatement,
                (
                    &program,
                    fitting_written.as_ref().map(|(_, stretch)| *stretch),
                ),
            )?;
        }
        let at = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        let mut stream = program_stream(
            subtype_dictionary(row).as_ref(),
            &program,
            row.key == "FontFile2",
        )
        .ok_or(Because::NotBuiltYet(PROGRAM_NOT_ENCODED))?;
        notices::attach(
            document,
            face.program(),
            &mut stream,
            spare,
            &mut substitutes.written,
        )?;
        substitutes.written.insert(at, stream);
        let own = embed_at_descriptor(
            document,
            descriptor,
            Embedding { key: row.key, at },
            restated.as_ref().map(|restatement| &restatement.descriptor),
            spare,
            &mut substitutes,
        )?;
        let mut entries = restated
            .as_ref()
            .map_or_else(Dictionary::new, |restatement| restatement.entries.clone());
        entries.insert(Name::new(&b"FontDescriptor"[..]), Object::Reference(own));
        if let Some(twins) = &twins {
            entries.insert(Name::new(&b"Encoding"[..]), twins.encoding.clone());
        }
        // Table 109 makes a font's `/FirstChar`, `/LastChar`, `/Widths` and `/FontDescriptor` all
        // present or all absent for the fourteen, and a font that states a descriptor and no widths
        // has its glyphs positioned by the viewer's own: the output states those numbers.
        if document.get_key(&used.dict, "Widths").is_null() {
            state_laid_out_widths(&mut entries, &font);
        }
        let (losses, fitting) = match (drawn, &fitting_written) {
            (_, Some((_, stretch))) => (Losses::default(), Some(Fitting::Written(*stretch))),
            (None, None) => (Losses::default(), None),
            (Some(drawn), None) => described::losses_of(drawn, &font),
        };
        substitutes.done.push(SubstitutedFont {
            resource: used.name.clone(),
            requested,
            face: face.describe(),
            authority: face.authority(),
            route,
            described: restated.as_ref().map(|restatement| restatement.described),
            losses,
            fitting,
            empty_glyphs: Vec::new(),
            twin_codes: twins.map(|twins| twins.codes).unwrap_or_default(),
            cut_program: cut,
            font: selected(used),
        });
        if !entries.is_empty() {
            gains(document, used, entries, spare, &mut substitutes)?;
        }
    }
    *failing = None;
    // Once every font is in, the descriptors the routes renamed are known, and so are the fonts
    // that name one without being part of the decision (`doc/adr/2476`).
    bystanders::keep_their_own_descriptors(document, spare, &mut substitutes)?;
    // Last: a page is photographed whole, so whether it may be is a question about everything it
    // shows rather than about one font (`doc/adr/2394`).
    photographed::photograph(document, gate, spare, &mut substitutes)?;
    substitutes.fenced = gate.fenced.pages.values().flatten().copied().collect();
    if substitutes.at.is_empty()
        && substitutes.added_fonts.is_empty()
        && substitutes.described.is_empty()
        && substitutes.direct.is_empty()
        && substitutes.photographs.is_empty()
    {
        return Err(Because::NotBuiltYet(if drawn_whole {
            CUT_PROGRAM_DRAWN_WHOLE
        } else {
            NO_FONT_TO_EMBED
        }));
    }
    Ok(substitutes)
}

/// Gives each simple font its own descriptor: even unused aliases must keep their old value.
///
/// `gains` is what a restated dictionary's descriptor states beside the program — the
/// non-symbolic `/Flags` of `doc/adr/2278` — written into this font's copy alone.
fn embed_at_descriptor(
    document: &Document,
    descriptor: ObjectId,
    embedding: Embedding,
    gains: Option<&Dictionary>,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<ObjectId, Because> {
    let mut own = document
        .get(descriptor)
        .as_dict()
        .cloned()
        .ok_or(Because::NotBuiltYet(DESCRIPTOR_NOT_AN_OBJECT))?;
    embedding.write_into(&mut own);
    for (key, value) in gains.into_iter().flat_map(Dictionary::iter) {
        own.insert(key.clone(), value.clone());
    }
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    substitutes.written.insert(at, Object::Dictionary(own));
    Ok(at)
}

/// Records the entries a font dictionary gains, wherever the dictionary is written.
///
/// A font that is an object of its own gains them at that object. **One written directly inside
/// its resource dictionary is first written out as an object of its own**, the entries included,
/// and the rewriter puts an indirect reference to it where the dictionary stood. ISO 32000-2
/// §7.3.10 makes the two spellings one value:
///
/// > Except where documented to the contrary, any object value may be a direct or an indirect
/// > reference; the semantics are equivalent.
///
/// Nothing documents the contrary for an entry of a resource dictionary's `/Font` subdictionary,
/// so the page selects the same font by the same name and no reader can draw the difference
/// (`doc/adr/2162`).
pub(super) fn gains(
    document: &Document,
    used: &SelectedFont,
    entries: Dictionary,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    if let Some(font) = used.id {
        if substitutes.added_fonts.contains(&font) {
            let mut written = used.dict.clone();
            for (key, value) in entries.iter() {
                written.insert(key.clone(), value.clone());
            }
            substitutes
                .written
                .insert(font, Object::Dictionary(written));
        } else {
            substitutes.described.insert(font, entries);
        }
        return Ok(());
    }
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    let mut written = used.dict.clone();
    for (key, value) in entries.iter() {
        written.insert(key.clone(), value.clone());
    }
    // The font is a font by where it stood, and the object written for it states the `/Type`
    // §9.6.2.1's Table 109 requires with its one value, which the rewriter's own walk leaves to
    // this object rather than change the value it finds the font by (`doc/adr/2608`).
    written.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Font"[..])),
    );
    substitutes.written.insert(at, Object::Dictionary(written));
    substitutes.direct.push((used.dict.clone(), at));
    Ok(())
}

/// The Table 124 row a face goes under, and the restatement of the dictionary it needs, if any.
///
/// **A dictionary of the other kind from its face is restated as the face's kind**, where Table
/// 124 admits the face under no key of the dictionary's own `/Subtype` and the face is one this
/// program ships; the restatement is held to the page the viewer draws before anything is
/// written (`doc/adr/2147`). An operator's face is not restated: the refusal names the pairing
/// and the kind of face that resolves it.
fn row_for(
    document: &Document,
    used: &SelectedFont,
    subtype: &str,
    face: &Face,
) -> Result<(Table124, Option<described::Restatement>), Because> {
    if let Some(row) = key_for(subtype, face.program()) {
        return Ok((row, None));
    }
    let unadmitted = || Because::NotBuiltYet(no_row(subtype, face.format(), face.program()));
    let Face::Shipped(shipped) = face else {
        return Err(unadmitted());
    };
    let restatement =
        described::restated_kind(document, used, subtype, shipped)?.ok_or_else(unadmitted)?;
    Ok((restatement.row, Some(restatement)))
}

/// The face a substitution embeds for `reading`: the operator's where they named one for
/// `requested`, this program's otherwise.
fn face_for(
    document: &Document,
    supplied: &BTreeMap<String, std::sync::Arc<[u8]>>,
    requested: &str,
    reading: &SelectedFont,
) -> Result<Face, Because> {
    Ok(match named_program(supplied, requested) {
        Some((named, program)) => Face::Supplied(
            pdf_font::standard::supplied_face(
                document,
                &reading.dict,
                &reading.name,
                std::sync::Arc::clone(program),
            )
            .map_err(|_| Because::NotBuiltYet(SUPPLIED_FACE_NOT_READ))?,
            named,
        ),
        None => Face::Shipped(
            pdf_font::standard::shipped_face(document, &reading.dict, &reading.name)
                .map_err(|_| Because::NotBuiltYet(NO_SHIPPED_FACE_COVERS_IT))?,
        ),
    })
}

/// The face's program with each split code reaching a copy of its glyph, and each copy's advance
/// added to `widths`; `None` where the face is not a `glyf` program [`twin_codes::split`] rewrites.
fn split_twins(
    face: &Face,
    splits: &[(u8, u16, f32)],
    widths: &mut BTreeMap<u16, f32>,
) -> Option<Vec<u8>> {
    if face.format() != Format::Sfnt {
        return None;
    }
    // Each copy starts at advance 0 and is given the dictionary's width by the restatement every
    // split takes (`MetricRoute::RestatedProgram`), whose read-back `proves` checks.
    let asked: Vec<(u8, u16, u16)> = splits
        .iter()
        .map(|(code, glyph, _)| (*code, *glyph, 0))
        .collect();
    let twin_codes::Split { program, reached } = twin_codes::split(face.program(), &asked)?;
    for (code, copy) in reached {
        let (_, _, width) = splits.iter().find(|(split, ..)| *split == code)?;
        widths.insert(copy, *width);
    }
    Some(program)
}

/// The single-byte codes a simple font's shown text holds.
fn shown_bytes(font: &LoadedFont, used: &SelectedFont) -> Vec<u8> {
    let codes: BTreeSet<u8> = used
        .shown
        .keys()
        .flat_map(|text| font.decode(text))
        .filter_map(single_byte)
        .collect();
    codes.into_iter().collect()
}

/// The widths the face has to state for the codes this document shows, and which route that is.
///
/// Route 1 where every one of them already agrees with the dictionary to within [`CONSISTENT`] —
/// which is what "metric-compatible" means and what the faces this program ships are, for
/// §9.6.2.2's fourteen — and route 2 otherwise. A code the face has no glyph for is a refusal
/// rather than a width: showing it would reference `.notdef`, which ISO 19005-2 section 6.2.11.8
/// and ISO 19005-4 section 6.2.10.9 forbid.
///
/// A code of `splittable` that reaches a glyph another code already holds at another width is
/// returned as a split — the code, the glyph and its own width — rather than refused, and is
/// asked after every other code so that the glyph's own advance is the other code's
/// (`twin_codes`, `doc/adr/2723`).
fn face_widths(
    font: &LoadedFont,
    face: &Face,
    used: &SelectedFont,
    splittable: &[u8],
) -> Result<FaceWidths, Because> {
    let mut widths: BTreeMap<u16, f32> = BTreeMap::new();
    let mut route = MetricRoute::FaceMetrics;
    let mut splits: Vec<(u8, u16, f32)> = Vec::new();
    let mut codes: Vec<Code> = used
        .shown
        .keys()
        .flat_map(|text| font.decode(text))
        .collect();
    codes.sort_by_key(|code| single_byte(*code).is_some_and(|byte| splittable.contains(&byte)));
    {
        for code in codes {
            let byte = single_byte(code).ok_or(Because::NotBuiltYet(NOT_A_SIMPLE_CODE))?;
            // A code the file's own encoding sends to `.notdef` reaches it in any program, so no
            // face resolves it; the empty glyph `doc/questions/A48` allows is written on the route
            // that writes the descriptor (`empty`, `doc/adr/2458`).
            let missing = if empty::reaches_notdef(font, code) {
                Because::NotBuiltYet(empty::NOTDEF_BESIDE_A_DESCRIPTOR)
            } else {
                Because::NotBuiltYet(match face.authority() {
                    FaceAuthority::Shipped => NO_SHIPPED_FACE_COVERS_IT,
                    FaceAuthority::Operator => NO_SUPPLIED_FACE_COVERS_IT,
                })
            };
            let glyph = face
                .glyph(byte)
                .filter(|glyph| *glyph != pdf_font::NOTDEF_GLYPH)
                .ok_or(missing)?;
            let stated = font.advance(code);
            match widths.entry(glyph) {
                Entry::Vacant(slot) => {
                    slot.insert(stated);
                }
                Entry::Occupied(held) if disagrees(*held.get(), stated) => {
                    if !splittable.contains(&byte) {
                        return Err(Because::TheFence(TWO_WIDTHS_FOR_ONE_GLYPH));
                    }
                    if !splits.iter().any(|(split, ..)| *split == byte) {
                        splits.push((byte, glyph, stated));
                    }
                    route = MetricRoute::RestatedProgram;
                    continue;
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
    Ok(FaceWidths {
        widths,
        route,
        splits,
    })
}

/// What [`face_widths`] finds: the advance each glyph is to state, the route, and each code that
/// is to reach a copy of its glyph — the code, the glyph, its own width.
struct FaceWidths {
    widths: BTreeMap<u16, f32>,
    route: MetricRoute,
    splits: Vec<(u8, u16, f32)>,
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
    let embedded = embedded_program(document, &descriptor_dict(document, used)?)?;
    let font = LoadedFont::load(document, &used.dict, &used.name).ok()?;
    (!font.is_substituted()).then_some((font, embedded))
}

/// The descriptor of a rendered font that is not Type 3, read through a `Type0`'s descendant,
/// where the descriptor is an object of its own.
fn descriptor_of(document: &Document, used: &SelectedFont) -> Option<ObjectId> {
    descriptor_entry(document, used)?.as_reference()
}

/// The descriptor of a rendered font that is not Type 3, read through a `Type0`'s descendant,
/// whether it is an object of its own or written directly inside the font.
///
/// **What the descriptor says does not depend on where it is written**: ISO 32000-2 §7.3.10
/// makes a direct value and a reference to it one value except where a clause documents the
/// contrary, so a descriptor written inside its font still names the program the font draws
/// from, and that program — a stream, and so always an object of its own (§7.3.8.1) — is what
/// the restatement rewrites (`doc/adr/2722`).
fn descriptor_dict(document: &Document, used: &SelectedFont) -> Option<Dictionary> {
    document
        .resolve(&descriptor_entry(document, used)?)
        .as_dict()
        .cloned()
}

/// The `/FontDescriptor` entry of a rendered font that is not Type 3, as written.
fn descriptor_entry(document: &Document, used: &SelectedFont) -> Option<Object> {
    let subtype = name_of(document, &used.dict, "Subtype")?;
    if subtype == "Type3" {
        return None;
    }
    let holder = if subtype == "Type0" {
        descendant(document, &used.dict)?
    } else {
        used.dict.clone()
    };
    holder.get("FontDescriptor").cloned()
}

/// The `/FontFile` entry a descriptor states, and which reader its bytes belong to.
///
/// §9.9's Table 124 is the mapping from key and `/Subtype` to format: `/FontFile` a Type 1
/// program in its own encrypted form, whose three lengths a rewrite states again, `/FontFile2`
/// an sfnt, and `/FontFile3` what its `/Subtype` names.
fn embedded_program(document: &Document, dict: &Dictionary) -> Option<Embedded> {
    if let Some(at) = dict.get("FontFile").and_then(Object::as_reference) {
        return Some(Embedded {
            at,
            format: Filed::Type1,
            length1: false,
        });
    }
    if let Some(at) = dict.get("FontFile2").and_then(Object::as_reference) {
        return Some(Embedded {
            at,
            format: Filed::Read(Format::Sfnt),
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
        format: Filed::Read(format),
        length1: false,
    })
}

/// The dictionary that states a font's descriptor: a Type 0 font's descendant, or the font.
fn descriptor_holder(document: &Document, dict: &Dictionary) -> Option<Dictionary> {
    match name_of(document, dict, "Subtype").as_deref() {
        Some("Type0") => descendant(document, dict),
        _ => Some(dict.clone()),
    }
}

/// Whether the program a rendered font's descriptor states is one the file holds only the front
/// of, or one whose whole bytes are not a program in the format it is filed under, and for the
/// first whether this reader draws from it all the same.
///
/// ISO 32000-2 §9.9 embeds a program "as data contained in a PDF stream object", and a decode
/// that stops short has handed over the front of one: `fonts/font-programs-embedded` fails it as
/// it fails a font that names none (`doc/adr/2619`), by the reading
/// [`pdf_archive::program_stopping_short`] makes here too. So it is answered as that font is: the
/// front is dropped and a face takes the font's place under the descriptor's names and the
/// dictionary's widths (`doc/adr/2636`). Not where this reader draws the program whole all the
/// same — a truncation reaching the extent §9.9's Table 125 states — because dropping it would
/// change shapes the window drew, and writing the stream whole is a remedy of its own.
fn cut_program(document: &Document, used: &SelectedFont) -> Shortfall {
    let Some(descriptor) = descriptor_holder(document, &used.dict).and_then(|holder| {
        document
            .get_key(&holder, "FontDescriptor")
            .as_dict()
            .cloned()
    }) else {
        return Shortfall::None;
    };
    let Some((key, damage)) = pdf_archive::program_stopping_short(document, &descriptor) else {
        // Bytes that decode whole and are not a program in the format they are filed under are
        // no program either, and this reader draws none of them by the reading that finds them.
        return pdf_archive::program_unparsed(document, &used.dict, &used.name)
            .map_or(Shortfall::None, |(key, _)| {
                Shortfall::Cut(CutProgram { key, damage: None })
            });
    };
    if LoadedFont::load(document, &used.dict, &used.name).is_ok() {
        Shortfall::DrawnWhole
    } else {
        Shortfall::Cut(CutProgram {
            key,
            damage: Some(damage),
        })
    }
}

/// What [`cut_program`] found of a rendered font's embedded program.
enum Shortfall {
    /// The decode did not stop short, or there is no program to decode.
    None,
    /// The decode stopped short, or the bytes are not a program in their format, and this reader
    /// draws none of the program: the substitution's.
    Cut(CutProgram),
    /// The decode stopped short and this reader draws the program all the same: the refusal
    /// [`CUT_PROGRAM_DRAWN_WHOLE`]'s.
    DrawnWhole,
}

/// A simple font as it reads with its cut program dropped: the descriptor written into the font
/// without §9.9's three keys, so that each reading of it loads the face a font naming no program
/// is drawn from.
fn program_dropped(document: &Document, used: &SelectedFont) -> SelectedFont {
    let mut descriptor = document
        .get_key(&used.dict, "FontDescriptor")
        .as_dict()
        .cloned()
        .unwrap_or_default();
    for key in ["FontFile", "FontFile2", "FontFile3"] {
        descriptor.remove(key);
    }
    let mut dict = used.dict.clone();
    dict.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Dictionary(descriptor),
    );
    SelectedFont {
        dict,
        ..used.clone()
    }
}

/// Whether a rendered font's descriptor states no program at all.
fn needs_a_program(document: &Document, dict: &Dictionary) -> bool {
    let Some(holder) = descriptor_holder(document, dict) else {
        return false;
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
    //
    // A program under a filter no reader of the standard decodes was never read, and the archive
    // leaves it out under `undecodable-stream` (`super::undecodable`, `doc/adr/2689`), so it is
    // no program here either. Where that loss is not granted the filter row refuses the document.
    ["FontFile", "FontFile2", "FontFile3"].iter().all(|key| {
        let program = descriptor.get(key).cloned().unwrap_or(Object::Null);
        document.resolve(&program).is_null()
            || super::undecodable::decodes_through_no_known_filter(document, &program)
    })
}

// One row of §9.9's Table 124 — a descriptor key, and the stream `/Subtype` that goes with it —
// as the validator reads it, so that the writer and the second verdict name the same rows.
pub(super) use pdf_archive::Table124Row as Table124;

/// Which of §9.9's keys this converter writes a program of its own under, in a dictionary of this
/// subtype.
///
/// **Table 124 decides it and this converter does not**: the rows are the validator's own
/// reading, `pdf_archive::table_124_rows`, asked of the bytes the writer is about to embed, so a
/// filing written here is one the output's second verdict admits. Where more than one row admits
/// the program the first is taken, which for a `glyf` program is `/FontFile2` before
/// `/FontFile3` `/OpenType`.
///
/// Two of the table's distinctions decide refusals here, and the reading names them. **`MMType1`
/// is in the `Type1C` row and not in the `OpenType` one**, so a multiple-master dictionary takes
/// a bare CFF and not a wrapped one. And **a `glyf` program has no row under a `Type1`
/// dictionary at all** — every key Table 124 opens to a `Type1` dictionary carries CFF — so a
/// face this program ships that happens to be `glyf`-based cannot go there whatever else is true
/// of it.
///
/// Every pairing the table does not state is `None`, which becomes a refusal naming the pairing
/// rather than a file the table does not admit (ADR 2327).
fn key_for(subtype: &str, program: &[u8]) -> Option<Table124> {
    pdf_archive::table_124_rows(subtype, program)?
        .first()
        .copied()
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
    row.subtype.map(|subtype| {
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
pub(super) fn program_stream(
    from: Option<&Dictionary>,
    program: &[u8],
    length1: bool,
) -> Option<Object> {
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

/// Owner A550: a metric-modified Liberation face cannot retain its reserved identity.
fn renamed_modified(program: &[u8], format: Format) -> Result<Vec<u8>, Because> {
    if format != Format::Sfnt {
        return Ok(program.to_vec());
    }
    pdf_font::modified_name::liberation(program).ok_or(Because::NotBuiltYet(
        "a modified font's name table cannot be read or rewritten safely to remove Liberation's reserved font name, so the program is not written",
    ))
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

/// A selected font as a content stream's lookup finds it: its object, or for a direct entry its
/// dictionary as `Debug` prints it, which is how the ledger names one (`Ledger::select`).
fn selected(used: &SelectedFont) -> Outcome {
    used.id.map_or_else(
        || Outcome::Direct(format!("{:?}", Object::Dictionary(used.dict.clone()))),
        Outcome::Reference,
    )
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

/// Why a program whose decode stops short is not dropped where this reader draws it all the same
/// (`doc/adr/2636`).
const CUT_PROGRAM_DRAWN_WHOLE: &str = "a font a content stream renders has an embedded program \
     whose stream's data stops short of what its filter describes, so ISO 32000-2 section 9.9 \
     does not count it as embedded; but the part the file holds is a program this reader \
     loads and draws from, so dropping it for a substitute face would change the shapes this \
     viewer shows, and writing the stream whole again is a remedy this version does not carry out";

/// Why a program this tree cannot decode is not restated.
const PROGRAM_NOT_DECODED: &str = "the embedded font program whose advances would have to be \
     restated could not be decoded from its stream, so there are no bytes to restate";

/// Why a program another font shares is not restated where restating it in place would break
/// that font and no copy can be written.
const SHARED_PROGRAM_NOT_COPIED: &str = "this font's embedded program is shared with another \
     font whose widths agree with it as it is, so restating it in place would make that font's \
     widths disagree instead, and the copy this font would be given cannot be made: the \
     dictionary naming its descriptor is not an indirect object, the descriptor does not name \
     the program under FontFile2 or FontFile3, or no object number is free for the copy";

/// Why a program whose advances are stated somewhere unreachable is not restated.
const PROGRAM_NOT_RESTATABLE: &str = "this font's embedded program states a glyph's advance \
     somewhere this converter does not rewrite — a Type 1 charstring that does not open with \
     hsbw or sbw over plain numbers or whose stream does not state the lengths of its sections, \
     or a CFF charstring whose width is inside a subroutine that is reached in a way \
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

/// Why a font whose descriptor is written inside its own dictionary is not given a substitute.
const DESCRIPTOR_NOT_AN_OBJECT: &str = "this rendered font writes its FontDescriptor directly \
     inside the font dictionary rather than as an object of its own, and a program is embedded by \
     adding a key to the descriptor object; ISO 32000-2 \u{a7}9.6.2.1's Table 109 makes the entry \
     an indirect reference, so there is no object here for the key to go into";

/// Why a program the operator named for a font that stated no descriptor is not embedded.
const NO_DESCRIPTOR_FOR_A_SUPPLIED_PROGRAM: &str = "--font names a program for a font that states \
     no FontDescriptor. The descriptor this conversion writes for one of the standard 14 is read \
     off the face this program ships and holds the page to what this reader draws with it, so it \
     describes that face and not the named one; without --font for this font the shipped face is \
     embedded";

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
