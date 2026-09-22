//! What a decision needs before it can be taken, worked out from the document once.
//!
//! Two constructions this converter can add to a file — ISO 19005-2 section 6.2.3's output intent
//! and the `DeviceN` `/DefaultCMYK` of its section 6.2.4.3, plus the metadata stream that carries
//! both parts' identification schema — and each of them depends on the *document* rather than on
//! the requirement alone: whether a profile can be shared with one the file already holds, whether
//! a producer's XMP packet can be edited in place, whether an object number is free.
//!
//! **A decision that cannot be carried out is a refusal rather than a plan**, so every preparation
//! either succeeds or yields the [`Because`] the requirements it would have answered are refused
//! with. [`super::decision::decide`] reads those reasons; nothing here decides anything.
use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::{DeviceFamily, Survey};
use pdf_archive::{Flavour, Level, Target, Verdict};
use pdf_archive::{MisusedProperty, Outcome};
use pdf_model::icc::Identification;
use pdf_model::xmp::{self, Name as XmpName, Schema};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use crate::json::Value;

use super::decision::{Because, REMEDIES};
use super::fonts::{self, Directions, Metrics, Substitutes};
use super::jpeg2000::{self, Specifications};
use super::preserve::{self, Composed};
use super::report::{Preserved, SetIn};
use super::rewrite::Rewrite;
use super::signatures::{self, ForeignHandlers, Signatures};
use super::sites::{
    self, AppearanceStates, ColorantEntries, CompletedOrders, DescriptorSets, PageResources,
    SharedProfile, Sites, StandardEncodings,
};
use super::tagged;
use super::to_unicode::{self, DerivedMaps};
use super::{ArchivePlan, COMPRESSION_LEVEL};

/// The RGB profile this program ships.
///
/// `doc/questions/A18`: ship the standard sRGB profile, with a flag to override it.
/// `data/icc/PROVENANCE.md` records which of the ICC's four sRGB profiles this is and why — the
/// short answer being that ISO 19005-2 section 6.2.4.2 names the ICC editions a profile may
/// conform to and the ICC's headline v4 download conforms to none of them. It is `static` data,
/// so it costs no parse time until something asks for it.
const SRGB: &[u8] = include_bytes!("../../../../data/icc/sRGB2014.icc");

/// The CMYK profile this program ships.
///
/// `doc/adr/1153`: the same decision as [`SRGB`]'s, taken for the other family a destination
/// profile can be — ISO 19005-2 section 6.2.4.3 licenses `DeviceCMYK` through an output intent
/// whose profile is CMYK, and the RGB one cannot answer that row whatever else it does.
/// `data/icc/PROVENANCE.md` carries the edition argument for a `prtr` class, the licence read
/// out of the file's own `cprt` tag, and the alternatives declined. Also `static`.
const CMYK: &[u8] = include_bytes!("../../../../data/icc/GRACoL2006_Coated1v2.icc");

/// The action this conversion records in `xmpMM:History` when it writes the `/DefaultCMYK`.
///
/// `doc/questions/A48` allows the construction on two conditions, and this is the second of them:
/// the clause is named in the file's own provenance, so that a later reader knows precisely which
/// approximation was applied and can undo the interpretation. ISO 19005-2 section 6.6.6 asks a
/// recorded action for what was done, with what, and when; ISO 19005-4 section 6.7.5 asks for two
/// of the three. All three are written, which answers both.
pub(super) const DEFAULT_CMYK_ACTION: &str = "converted";

/// The `parameters` field of that action.
///
/// The clause is named in words rather than with a section sign, because this string is written
/// into somebody else's file and the sign is this project's convention rather than the standard's.
pub(super) const DEFAULT_CMYK_PARAMETERS: &str = "a DeviceN DefaultCMYK was added over Cyan, \
     Magenta, \
     Yellow and Black, whose tint transform is the DeviceCMYK to DeviceRGB conversion of \
     ISO 32000-2 clause 10.4.2.5, with this file's ICC sRGB profile as the alternate space";

/// Why the `DeviceN` `/DefaultCMYK` could not be built for this document.
///
/// §10.4.2.5's transform produces `DeviceRGB`, so the alternate space this construction needs is
/// an **RGB** ICC profile: writing the transform's three numbers into a space of any other family
/// would be asserting an arithmetic the clause does not state.
const NO_RGB_ALTERNATE: &str = "the DeviceN DefaultCMYK this file needs states ISO 32000-2 \
     §10.4.2.5's transform, whose result is RGB, so its alternate space has to be an RGB ICC \
     profile — and the profile this conversion has is of another colour family";

/// Every font object the two Unicode requirements were found to fail at.
///
/// The population the derivation runs over, read off the validator's report rather than out of
/// the document: ISO 19005-2 section 6.2.11.7.2 excuses four kinds of font and which fonts those
/// are is the validator's reading, not this converter's.
fn unicode_fonts(input: &pdf_archive::Report) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = input
        .failures()
        .filter(|judgement| UNICODE_REQUIREMENTS.contains(&judgement.id))
        .filter_map(|judgement| match &judgement.outcome {
            Outcome::Failed { places, .. } => Some(places),
            _ => None,
        })
        .flatten()
        .filter_map(|finding| finding.place.object)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The two requirements a derived `/ToUnicode` `CMap` answers.
const UNICODE_REQUIREMENTS: [&str; 2] = [
    "fonts/to-unicode-present",
    "fonts/to-unicode-values-are-usable",
];

/// Why `/MarkInfo` with `/Marked true` is not written into a file with no structure tree.
///
/// `doc/pdf-a-conversion-limits.md` section 5.1, and it is a policy refusal rather than a gap:
/// the flag asserts that the file follows ISO 32000's Tagged PDF conventions, and only the tree
/// itself can make that true. ISO 19005-2 section 6.7.1 advises writers against adding structural
/// information not present in the source solely to achieve conformance, so declining here is the
/// standard's own counsel rather than this converter being strict.
const NO_STRUCTURE_TREE: &str = "ISO 19005-2 section 6.7.2.2's Marked flag says the file follows \
     the base standard's Tagged PDF conventions, and this document states no StructTreeRoot for \
     it to be true of. This converter will not invent a structure tree — deciding that a run of \
     glyphs is a heading and what an image depicts is authoring rather than converting, and \
     section 6.7.1 of that part advises writers against it — so the flag is not written either. \
     PDF/A-2u and PDF/A-2b ask nothing of logical structure and this document may well reach \
     both";

/// Why the `DeviceN` `/DefaultCMYK` was not written even though it could have been built.
///
/// `doc/questions/A48` allows the construction *on condition* that it is recorded in the file's
/// own `xmpMM:History` naming the clause. So a document whose packet will not take that entry
/// does not get the construction either: the permission and its condition are one thing, and a
/// converter that kept the first while dropping the second would be helping itself to a licence
/// it had not earned.
const NO_PLACE_TO_RECORD: &str = "the DeviceN DefaultCMYK is allowed on condition that it is \
     recorded in this file's own xmpMM:History naming the clause, and this document's XMP packet \
     will not take that entry — so the construction is not written either";

/// The action this conversion records in `xmpMM:History` when it removes a property.
///
/// `doc/pdf-a-conversion-limits.md` section 4.2: every **Ask** writes one entry, and the entry is
/// the audit trail that makes the Asks defensible. A removed property is the one loss in this
/// verb that a reader of the output cannot see the shape of — a deleted key leaves no hole — so
/// the provenance is where the file itself says what it lost.
pub(super) const REMOVED_PROPERTIES_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it departs from a requirement.
///
/// `doc/rfc/0007` section 4.7.3: a departure is recorded in the file itself, so the archive carries
/// the fact that it goes against the standard on purpose. The word is `converted`, like the other
/// recorded actions — ISO 19005-4 section 6.7.5's history records what a converter did — and the
/// parameters name the requirement, the predicate that narrowed the departure, and the operator's
/// stated reason, which section 4.7.2 requires and which is the whole of what makes a departure
/// something a reader of the archive can understand two years on.
pub(super) const DEPARTED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when a tool derived an artefact.
///
/// `doc/questions/A55` makes the record one of the four terms the mode is offered on: the *archive*
/// carries the fact that part of it is derived rather than original, instead of that fact living
/// only in a report somebody may not have kept. The word is `converted`, like every other recorded
/// action here — ISO 19005-4 section 6.7.5's history records what a converter did — and the
/// parameters carry the owner's own sentence, *this is derived, not original*, with what was
/// derived, from what, and by which tool.
pub(super) const DERIVED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` for a fact the operator supplied.
///
/// `doc/rfc/0007` section 5b.1's obligation, which `supply` carries and the other four remedy kinds
/// do not: it is the one kind the converter cannot get wrong and the person can, so the file itself
/// records that a human rather than the document is the value's source.
pub(super) const SUPPLIED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` for a stream an operator's tool fetched.
///
/// `doc/adr/1209`. The rewrite writes the fetched bytes where the stream's own were, so nothing in
/// the file says afterwards that they came from outside it — which is exactly the fact a reader of
/// an archive is entitled to find in the archive. The word is `converted`, like every other
/// recorded action here.
pub(super) const RESOLVED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when a page preserves content.
///
/// `doc/adr/1014` section 5's fifth bullet, which the amendment makes a condition of the
/// permission rather than a nicety: a page appended to keep content the target would otherwise
/// lose is a change to the document a reader of the archive is entitled to find in the archive.
/// The word is `converted`, like every other recorded action here.
pub(super) const PRESERVED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it removes a source's encryption.
///
/// `doc/pdf-a-mitigations.md` section 2's `preserve` beside the `discard`, and `doc/adr/1187` the
/// argument: an unencrypted archive cannot enforce §7.6.4.2's Table 22 flags, so what the producer
/// asserted about its reader goes into the file itself before the enforcement goes. The word is
/// `converted`, like every other recorded action here.
pub(super) const DECRYPTED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it embeds a substitute face.
///
/// ISO 19005-2 section 6.6.6's NOTE 1 and ISO 19005-4 section 6.7.5's NOTE both give font
/// substitution as an example of a converter action that changes a document's appearance and is
/// therefore to be recorded — which is `doc/pdf-a-conversion-limits.md` section 4.9's third
/// argument for the act being contemplated by the standard rather than merely not forbidden.
pub(super) const SUBSTITUTED_FONTS_ACTION: &str = "converted";

/// How many substituted fonts the `xmpMM:History` entry names before it stops listing them.
const MOST_FONTS_NAMED: usize = 8;

/// Why a font is refused rather than substituted when the caller asked for that.
///
/// `doc/pdf-a-conversion-limits.md` section 4.9's `--no-substitute`, and the sentence says which
/// of the four kinds of *no* this is — the one the caller can take back.
const SUBSTITUTION_DECLINED: &str = "this document renders a font it does not embed, and \
     --no-substitute was asked for. The default is to embed one of the faces this program ships \
     and report which, on the argument that a file whose font is not embedded has no appearance \
     of its own; the flag is for a curator who would rather be told the font is missing than be \
     given a stand-in. Drop the flag, or supply the font itself with --font";

/// Why a substitution was withdrawn even though it could have been carried out.
const NO_PLACE_TO_RECORD_A_SUBSTITUTION: &str = "embedding a face for a font this file never \
     carried is allowed on condition that it is recorded in the file's own xmpMM:History, which \
     is where ISO 19005-2 section 6.6.6's NOTE 1 puts font substitution by name — and this \
     document's XMP packet will not take that entry, so no face is embedded either";

/// How many removed properties the `xmpMM:History` entry names before it stops listing them.
///
/// A bound rather than a reading (principle 3): a packet may state hundreds of properties and the
/// provenance entry is prose. The conversion report names every one of them whatever this is.
const MOST_NAMED: usize = 16;

/// Why a packet's properties could not be removed from it.
const PACKET_NOT_CUTTABLE: &str = "this document states a metadata property in a predefined \
     schema that does not define it, and the packet it is in cannot be edited in place — so the \
     property cannot be removed without rewriting metadata its producer wrote. \
     doc/pdf-a-conversion-limits.md section 3.9 is the reading";

/// Why a property that survived the cut refuses the document.
const PROPERTY_NOT_CUT: &str = "a metadata property this conversion removed is still in the \
     packet afterwards, so the packet is half-edited rather than corrected, and a half-edited \
     packet is not written at all — doc/pdf-a-conversion-limits.md section 3.9";

/// Why a metadata stream that is not one is not cleaned.
const NOT_A_PACKET: &str = "this document fails the schema requirement in an object that is not \
     a stream this tree can decode, so there is no packet to take the property out of";

/// Why a container missing a field outright is not respelled.
///
/// ISO 19005-2 section 6.6.2.3.3's tables ask for two different things at once, and only one of
/// them is a fact about the serialisation. A field stated with the wrong prefix is the packet
/// spelling a name it already carries; a field the packet does not state is content nothing in
/// the file holds, and `doc/pdf-a-mitigations.md`'s entry for this requirement is the reading:
/// what is missing is a name for a schema, a description of what a property means, or the
/// category saying where a value came from, and the two honest answers to that are the operator's
/// own `supply` and a `discard` that loses the container. Neither is this rewrite.
const FIELD_NOT_IN_THE_FILE: &str = "this document describes an extension schema whose \
     description leaves out a field ISO 19005-2 section 6.6.2.3.3's tables require of it. What is \
     missing is a name for the schema, a description of what a property means, or the category \
     saying whether a property's value is derived from the document or supplied from outside it \
     — none of which the file states anywhere, so supplying one would be this converter writing \
     metadata about metadata that nobody produced. A field the packet does state and spells with \
     another prefix is corrected without asking anyone, because the name, the namespace and the \
     value are all already in the file";

/// Why a packet whose prefixes cannot be moved refuses the document.
const PACKET_NOT_RESPELLABLE: &str = "this document spells an extension schema container field \
     with a prefix other than the one ISO 19005-2 section 6.6.2.3.3's table requires, and the \
     packet it is in cannot be respelled in place: it is not one this tree can edit, or the \
     required prefix already means another namespace in the same packet, where taking it over \
     would change what names spelled with it mean";

/// Why a field still spelled wrongly afterwards refuses the document.
const PREFIX_NOT_RESPELLED: &str = "an extension schema container field this conversion respelled \
     still carries the wrong prefix afterwards, so the packet is half-edited rather than \
     corrected — and a half-edited packet is not written at all, which is the rule the property \
     removal beside it already follows";

/// Why the removal was withdrawn even though it could have been carried out.
///
/// `doc/pdf-a-conversion-limits.md` section 4.2 makes one `xmpMM:History` entry the audit trail of
/// every **Ask**, and a removal is the loss whose trace is hardest to find afterwards: a property
/// that is gone leaves nothing behind to notice. So a packet that will not take the entry does not
/// get the removal either.
const NO_PLACE_TO_RECORD_A_REMOVAL: &str = "removing a metadata property is recorded in this \
     file's own xmpMM:History, and this document's XMP packet will not take that entry — so the \
     properties are left where their producer wrote them and no file is written";

/// Why an annotation's appearance could not be constructed.
const APPEARANCE_NOT_DERIVABLE: &str = "this document holds an annotation with no appearance \
     dictionary whose own subtype clause states no artwork for it — a stamp's legend, a caret, an \
     unapplied redaction, a printer's mark, a trap network, a movie's poster, a watermark, 3D or \
     rich media artwork, or an annotation stating no Subtype or no readable Rect at all. \
     ISO 32000-2 Table 166 requires the dictionary and the subtype clause is what would say what \
     goes in it, so constructing one here would be putting a mark on the page the document never \
     described";

/// Why a partly derivable appearance is not written either.
const APPEARANCE_INCOMPLETE: &str = "this document holds an annotation whose appearance this \
     program can construct only in part — a border style stating no highlight colour, a caption \
     whose room the entry does not give, a field value that would not lay out. A partial \
     rendering written into an archive is not the appearance the clauses state, and a conforming \
     reader would then have nothing else to draw from, so it is refused rather than frozen";

/// Why a button field's widget is not given a constructed appearance.
const BUTTON_APPEARANCE_STATES: &str = "this document holds a widget of a button field with no \
     appearance dictionary, and §12.7.5.2.3 makes such a widget's N entry a subdictionary of one \
     appearance per state rather than a single stream. Which states the button has is what an \
     absent appearance dictionary does not say, and naming them would be inventing the control's \
     own vocabulary";

/// Why an annotation the page states inline is not given one.
const APPEARANCE_ON_A_DIRECT_ANNOTATION: &str = "this document writes an annotation directly into \
     a page's Annots array rather than as an object of its own, and this verb rewrites objects — \
     so there is nowhere to put the appearance stream the annotation would name";

/// How far up a form field's `/Parent` chain the inheritable `/FT` is looked for.
///
/// `pdf_syntax::Limits::DEFAULT`'s `max_depth`, which is the depth the parser admitted the
/// tree at: a `/Parent` chain longer than that is one no field in the document was read through.
const MAX_FIELD_DEPTH: usize = 256;

/// The two spellings the two parts print for the identification schema's namespace.
///
/// ISO 19005-2 section 6.6.4 gives it with an `http` scheme and ISO 19005-4 section 6.7.3 with an
/// `https` one. A conversion writes the one its target's part prints and **removes both**, because
/// a property left behind under the other spelling would be a second claim standing beside the one
/// just written.
const IDENTIFICATION_URIS: [&str; 2] = [
    "http://www.aiim.org/pdfa/ns/id/",
    "https://www.aiim.org/pdfa/ns/id/",
];

/// The prefix both parts make required for every property of the identification schema.
const IDENTIFICATION_PREFIX: &str = "pdfaid";

/// The publication year ISO 19005-4's own revision property names.
const REVISION_YEAR: &str = "2020";

/// Why no output intent could be prepared for this document.
const NO_USABLE_PROFILE: &str = "an output intent needs a destination profile that is an ICC \
     profile of an output or monitor class over grey, RGB or CMYK, and this conversion has none: \
     either the profile supplied with --output-intent-profile is not one, or the file already \
     holds a destination profile that is not — and ISO 19005 requires every entry of an \
     OutputIntents array to name the same profile object, so a second one cannot be added beside \
     it";

/// Why a producer's XMP packet could not be given the identification schema.
const PACKET_NOT_EDITABLE: &str = "this document's XMP packet cannot be edited in place, and \
     replacing it would throw away metadata its producer wrote — which is a loss nobody has been \
     asked to authorise. doc/pdf-a-conversion-limits.md section 4.2 is where that question \
     belongs, and this converter does not put it yet";

/// The placeholder for a construction no failed requirement asked for.
///
/// Never reported: `decide` reads a preparation's reason only for a requirement the table answers
/// with the rewrite that preparation builds, and such a requirement is exactly what makes the
/// preparation happen. It says so rather than borrowing another reason's sentence.
const NOT_ASKED_FOR: &str = "no requirement this document failed asked for this construction, so \
     none was prepared";

/// Why neither construction could be prepared for a document with no readable catalog.
///
/// Unreachable through [`run`], which refuses such a document before stage 1; it exists so that
/// the reason a requirement is refused with is never a reason that is not the actual one.
pub(super) const NO_CATALOG: &str = "this document has no readable catalog, so there is nowhere \
     to state an output intent or a metadata stream";

/// Why an object could not be added to the output.
const NO_SPARE_OBJECT: &str = "this document uses every object number a conversion could give to \
     the stream it has to add";

/// The properties the identification schema states for one target.
///
/// ISO 19005-2 section 6.6.4 asks for a part number of 2 and a conformance level of A, B or U;
/// ISO 19005-4 section 6.7.3 asks for a part number of 4 and a revision year, and reserves a
/// conformance property for the two annexes — a file that is neither PDF/A-4e nor PDF/A-4f states
/// none at all. **Part 4's own table spells that property with a `pdfa` prefix** in a schema whose
/// required prefix the same table gives as `pdfaid`; `pdf_archive`'s metadata tranche records why
/// it reads the property in the identification namespace whatever prefix spells it, and this
/// writes `pdfaid` because that is the prefix the subclause makes required and the one
/// `metadata/identification-schema-prefix` holds a file to.
fn identification_properties(target: Target) -> Vec<(&'static str, String)> {
    match target {
        Target::Two(level) => vec![
            ("part", "2".to_owned()),
            (
                "conformance",
                match level {
                    Level::A => "A",
                    Level::B => "B",
                    Level::U => "U",
                }
                .to_owned(),
            ),
        ],
        Target::Four(flavour) => {
            let mut out = vec![("part", "4".to_owned()), ("rev", REVISION_YEAR.to_owned())];
            match flavour {
                Flavour::Plain => {}
                Flavour::E => out.push(("conformance", "E".to_owned())),
                Flavour::F => out.push(("conformance", "F".to_owned())),
            }
            out
        }
    }
}

/// The namespace URI the target's own part prints for the identification schema.
const fn identification_uri(target: Target) -> &'static str {
    match target.part() {
        pdf_archive::Part::Two => IDENTIFICATION_URIS[0],
        pdf_archive::Part::Four => IDENTIFICATION_URIS[1],
    }
}

/// Where an output intent's destination profile came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    /// One of the two profiles this program ships — `doc/questions/A18`'s default.
    ///
    /// Which of them is [`shipped_for`]'s decision, and the report says which by printing the
    /// profile's own `desc` tag and colour space beside this word rather than by having two.
    Shipped,
    /// One the caller supplied.
    Supplied,
    /// One the document already held.
    ///
    /// Not a choice this conversion made: ISO 19005 requires every entry of an `OutputIntents`
    /// array that states a destination profile to state the *same* object, so a file that already
    /// holds one decides what a new entry names.
    AlreadyInTheFile,
}

impl ProfileSource {
    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Shipped => "shipped",
            Self::Supplied => "supplied",
            Self::AlreadyInTheFile => "already-in-the-file",
        }
    }

    /// Where the profile came from, in one clause for a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Shipped => "a profile this program ships",
            Self::Supplied => "the profile you supplied",
            Self::AlreadyInTheFile => "the destination profile this file already held",
        }
    }
}

/// The destination profile a conversion's output intent names, as the report states it.
///
/// **The `cprt` tag is the load-bearing field.** `doc/pdf-a-conversion-limits.md` section 10.1
/// records the ICC's own guidance that a profile's copyright owner and terms of use live in its
/// header's creator field and its `cprt` tag, and turns that into a rule: a user who embeds
/// somebody else's press profile is told whose it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationProfile {
    /// Where it came from.
    pub source: ProfileSource,
    /// The profile's `desc` tag: what it calls itself.
    pub describes: Option<String>,
    /// Its `cprt` tag: whose profile it is, and on what terms.
    pub copyright: Option<String>,
    /// The colour space its own header states.
    pub space: String,
}

impl DestinationProfile {
    /// The profile as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("source".to_owned(), Value::text(self.source.word())),
            (
                "describes".to_owned(),
                self.describes
                    .as_ref()
                    .map_or(Value::Null, |text| Value::text(text.clone())),
            ),
            (
                "copyright".to_owned(),
                self.copyright
                    .as_ref()
                    .map_or(Value::Null, |text| Value::text(text.clone())),
            ),
            ("space".to_owned(), Value::text(self.space.clone())),
        ])
    }
}

/// The output intent a conversion is in a position to add.
pub(super) struct Intent {
    /// The object the destination profile is, in the *source's* numbering.
    pub(super) destination: ObjectId,
    /// The profile stream this conversion adds, where it adds one.
    pub(super) written: Option<Object>,
    /// The colour family the profile's own header states.
    pub(super) family: DeviceFamily,
    /// What the report says about the profile.
    pub(super) reported: DestinationProfile,
}

/// The `/DefaultCMYK` a conversion is in a position to write, and the objects it needs.
///
/// ISO 19005-2 section 6.2.4.3's second licence, built from the standard and nothing else:
/// §8.6.6.5's `DeviceN` over the four names that clause reserves for a CMYK device's process
/// colourants, §7.10.5's PostScript calculator function stating §10.4.2.5's conversion, and the
/// ICC sRGB profile the output intent would have named as the alternate space §8.6.6.4 says the
/// tint transform's output is interpreted in.
pub(super) struct DefaultCmyk {
    /// The colour space array, in the *source's* numbering.
    pub(super) space: ObjectId,
    /// The two objects this conversion adds for it.
    pub(super) written: [(ObjectId, Object); 2],
}

/// The packets a conversion is in a position to take a property out of.
///
/// ISO 19005-2 section 6.6.2.3.1 requires every property to *use* the schema it names, and
/// `doc/pdf-a-conversion-limits.md` section 3.9 records why removal is the only open route: a
/// corrected value would be content this converter invented, and an extension schema container
/// describing a predefined schema would misrepresent it in the file itself.
#[derive(Debug)]
pub(super) struct Cleaned {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(super) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// Every property removed, for the report, in the order the packets state them.
    pub(super) removed: Vec<MisusedProperty>,
}

/// The packets a conversion is in a position to respell a container's prefixes in.
///
/// ISO 19005-2 section 6.6.2.3.3's four tables each name the prefix their fields are required to
/// be spelled with, and section 6.6.2.2 makes that the one place in an XMP packet where a prefix
/// is part of what is required rather than a convenience. So a field stated in the right namespace
/// with the wrong prefix is a *spelling* this converter can correct out of what the file already
/// holds — which is what separates it from the fields nothing in the file states.
#[derive(Debug)]
pub(super) struct Respelled {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(super) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// How many fields were respelled, for the report to count places by.
    pub(super) fields: usize,
}

/// The appearances a conversion is in a position to construct.
///
/// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 require an appearance dictionary of
/// every annotation but three subtypes and the degenerate rectangle, and
/// `doc/pdf-a-conversion-limits.md` section 4.4 records why constructing one is not invention:
/// §12.5.5 and §12.7.4.3 make the appearance the standard's own construction out of entries the
/// annotation already states. `doc/questions/A21` allows it on one condition, which is
/// [`Self::constructed`].
#[derive(Debug)]
pub(super) struct Appearances {
    /// The `/AP` `/N` object each annotation is to name, in the *source's* numbering.
    pub(super) at: BTreeMap<ObjectId, ObjectId>,
    /// The form `XObject`s this conversion adds.
    pub(super) written: Vec<(ObjectId, Object)>,
    /// Every appearance written, for the report.
    pub(super) constructed: Vec<WrittenAppearance>,
    /// How many places the requirement failed at that section 6.3.1's removal takes off the page.
    ///
    /// **Not a gap and not a construction: a place that will not exist.** ISO 19005-2 section
    /// 6.3.3 and ISO 19005-4 section 6.3.3 ask an appearance dictionary of the annotations a
    /// *conforming file* holds, and section 6.3.1 forbids some subtypes outright — so an
    /// annotation the removal takes out of its page's `/Annots` is one the output does not hold
    /// and the appearance requirement has no place on. Counting them is what lets
    /// [`super::decision::answer_of`] tell a document whose every failing place goes with the
    /// removal from one that failed nowhere this preparation could see.
    pub(super) removed: usize,
}

/// One appearance this conversion constructed, as the report names it.
///
/// `doc/questions/A21`'s condition in one sentence: *report every appearance written, so the
/// difference between the producer's file and ours is visible in the report rather than only in
/// the bytes.* A count would not do it — which annotation on which page now draws this program's
/// marks is the thing a user has to be able to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenAppearance {
    /// The zero-based page the annotation is on.
    pub page: usize,
    /// Its `/Subtype`, or `an annotation stating no subtype` where it states none.
    pub subtype: String,
}

impl WrittenAppearance {
    /// One constructed appearance as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("page".to_owned(), Value::count(self.page)),
            ("subtype".to_owned(), Value::text(self.subtype.clone())),
        ])
    }
}

/// The metadata stream a conversion is in a position to write.
pub(super) struct Metadata {
    /// The object the packet goes in, in the *source's* numbering.
    pub(super) at: ObjectId,
    /// The stream this conversion adds, where the document had no packet to edit.
    pub(super) written: Option<Object>,
    /// The packet, for the stream that carries it.
    pub(super) packet: Vec<u8>,
}

/// What this slice's two constructions need, worked out before any decision is taken.
///
/// Both depend on the *document* rather than on the requirement alone — whether a profile can be
/// shared with one the file already holds, whether the packet a producer wrote can be edited in
/// place — and **a decision that cannot be carried out is a refusal rather than a plan**, which
/// is the rule [`version_for`] already answers to. So both are settled here, in stage 2, and the
/// reason each failed is the reason the requirements it would have answered are refused with.
///
/// Nothing is prepared that no failed requirement asked for: a document that already conforms
/// does not have its packet read, which is what keeps `doc/adr/0947`'s first rule true of this
/// slice as well.
pub(super) struct Prepared {
    /// The output intent to add, or why one cannot be.
    pub(super) intent: Result<Intent, Because>,
    /// The `/DefaultCMYK` to write, or why one cannot be.
    ///
    /// Prepared **after** the intent and **before** the metadata, because the three depend on
    /// each other in that order: the default's alternate space is the profile the intent settled,
    /// and the packet has to carry the `xmpMM:History` entry `doc/questions/A48` makes the
    /// default conditional on.
    pub(super) default_cmyk: Result<DefaultCmyk, Because>,
    /// The metadata stream to write, or why one cannot be.
    pub(super) metadata: Result<Metadata, Because>,
    /// The `/ToUnicode` `CMaps` to write, or why none can be.
    pub(super) to_unicode: Result<DerivedMaps, Because>,
    /// The appearances to construct, or why they cannot be.
    pub(super) appearances: Result<Appearances, Because>,
    /// The packets given a container describing the extension schemas they use, or why not.
    ///
    /// Prepared **last of the five writers over a packet**: ISO 19005-2 section 6.6.2.3.2's
    /// container describes what the packet says once the four editors before it have said their
    /// piece (`doc/adr/1245`).
    pub(super) described: Result<SchemasDescribed, Because>,
    /// The document's packet with a malformed amendment identifier cut out, or why not.
    ///
    /// Prepared **after** the three writers above and **before** the metadata, because it is the
    /// last of the four that edit a packet by span and the identification schema is restated
    /// into what it leaves. ISO 19005-2 section 6.6.4 (`doc/adr/1246`).
    pub(super) amended: Result<Amended, Because>,
    /// The packets with their container fields respelled, or why they cannot be.
    ///
    /// Prepared **after** the property removal and **before** the metadata, because all three
    /// write the same packets and each has to start from what the one before it left.
    pub(super) respelled: Result<Respelled, Because>,
    /// The packets replaced outright because this tree cannot read them, or why none are.
    ///
    /// Prepared **before every other writer over a packet**, and the streams it names are
    /// excluded from all of them: ISO 19005-2 section 6.6.2.1's packet is composed afresh where
    /// the producer's will not parse, states more than one `rdf:RDF` element or breaks the data
    /// model, and an edit by span has nothing to write into there.
    pub(super) fresh: Result<Fresh, Because>,
    /// The packets with their unusable properties taken out, or why they cannot be.
    ///
    /// Prepared **before** the metadata, because the catalog's own packet is one of these: the
    /// identification schema is restated into the packet the removal already edited, so that a
    /// document needing both edits gets one packet rather than two writers overwriting each other.
    pub(super) properties: Result<Cleaned, Because>,
    /// The font programs to embed where the file embedded none, or why none can be.
    ///
    /// Prepared **before** the metadata, like the `/DefaultCMYK` and for the same reason:
    /// ISO 19005-2 section 6.6.6's NOTE 1 and ISO 19005-4 section 6.7.5's NOTE name font
    /// substitution outright as a converter action to record in `xmpMM:History`, so a packet
    /// that will not take the entry withdraws the substitution rather than leaving it unrecorded.
    pub(super) substitutes: Result<Substitutes, Because>,
    /// The font programs whose stated advances are to be restated, or why none can be.
    ///
    /// **Not conditional on the packet**, unlike the substitution beside it: restating a
    /// program's advances changes no mark and no appearance, so it is not one of the actions
    /// either part's `xmpMM:History` subclause asks a converter to record.
    pub(super) metrics: Result<Metrics, Because>,
    /// Whether the file already carries the structure tree `/MarkInfo` would be a claim about.
    ///
    /// `Ok(())` where the catalog states a `/StructTreeRoot`, and the reason otherwise.
    /// `doc/pdf-a-conversion-limits.md` section 5.1: the converter will not invent a structure
    /// tree, and writing `/Marked true` over a file that has none would be the same act in one
    /// key — a claim that the file follows §14.8's conventions, made by the converter rather
    /// than demonstrated by the file.
    pub(super) structure: Result<(), Because>,
    /// Every signature the source carries, each verified over the source, and where the
    /// rewrite reaches it.
    ///
    /// **Prepared whenever the source will be rewritten, whether or not a failed requirement
    /// named a signature** — the one preparation not gated on `wanted`, and the reason is
    /// `doc/pdf-a-conversion-limits.md` section 3.6's: a rewrite moves every byte, so it
    /// invalidates every signature whatever requirement asked for it, and a part 4 target has
    /// no row that would notice. A conforming source is copied and never reaches here, so
    /// nothing is walked or hashed for it.
    pub(super) signatures: Signatures,
    /// `doc/pdf-a-mitigations.md` section 13.3's *owed, not optional* preparations.
    ///
    /// One field rather than seven because they are one class of work — see [`Owed`] — and
    /// because a reader of this struct should be able to see which of its parts are the two
    /// constructions this verb was built around and which are the lossless rewrites it grew.
    pub(super) owed: Owed,
    /// The streams whose data is brought inside the file, or why none can be.
    ///
    /// ISO 19005-2 section 6.1.7.1 and ISO 19005-4 section 6.1.6.1, answered out of the bytes
    /// the caller resolved ([`ArchivePlan::external_data`]) and, where a stream states the
    /// filter keys without a `/F`, out of the file's own (`doc/adr/1199`).
    pub(super) external_data: Result<super::external::Embedded, Because>,
    /// The out-of-range optional page boundary entries removed, or why none can be.
    ///
    /// ISO 19005-2 section 6.1.13's limit, answered by ISO 32000-2 §7.7.3.3's Table 31 and
    /// §14.11.2.1's defaults rather than by rescaling anything (`doc/adr/1210`).
    pub(super) boundaries: Result<super::boundaries::Boundaries, Because>,
    /// The content streams whose hexadecimal strings gain the final digit, or why none can.
    ///
    /// ISO 19005-2 section 6.1.6 and ISO 19005-4 section 6.1.5, inside a content stream: the
    /// whole-file rewrite writes every *object's* strings in the even-digit form by construction,
    /// and a string on a page is bytes that cross byte for byte unless this repair writes into
    /// them (`doc/adr/1176`).
    pub(super) hexadecimal: Result<super::hexadecimal::Completed, Because>,
    /// The entries the action clauses ask each holder to restate, or why none can be.
    ///
    /// ISO 19005-2 sections 6.4.1, 6.5.1 and 6.5.2 and ISO 19005-4 sections 6.4.1, 6.6.1 and
    /// 6.6.3, worked out once over the sites `pdf_archive` walks: three rewrites share one reading
    /// of §12.6's action trees, and computing them apart would walk the same trees three times
    /// (`doc/adr/1175`).
    pub(super) actions: Result<super::actions::Removals, Because>,
    /// Every annotation a target's section 6.3.1 does not admit, or why none can be removed.
    ///
    /// Prepared **before** [`Self::preserved`], because a `preserve` remedy at either subtype site
    /// keeps the normal appearances of exactly these annotations: the population is the removal's
    /// and the pages are composed from it.
    pub(super) forbidden_annotations: Result<ForbiddenAnnotations, Because>,
    /// Every annotation whose stated `/F` section 6.3.2 forbids, or why none can be removed.
    pub(super) hidden_annotations: Result<HiddenAnnotations, Because>,
    /// The appearance dictionaries that lose every key but `/N`, or why none do.
    pub(super) extra_appearance_states: Result<ExtraAppearanceStates, Because>,
    /// The optional content configurations that lose their `/AS`, or why none do.
    pub(super) automatic_states: Result<sites::AutomaticStates, Because>,
    /// The pages a `preserve` remedy appends, or why none can be.
    ///
    /// `doc/adr/1014`, and the field is `Err(NOT_ASKED_FOR)` for every conversion whose caller
    /// named no `preserve` remedy — which is every conversion until an operator's configuration
    /// says otherwise, so nothing is composed, no font is loaded and no page is measured for a
    /// document that asked for none.
    pub(super) preserved: Result<Composed, Because>,
    /// Whether this document's packet took the `xmpMM:History` entries this conversion has to
    /// record.
    ///
    /// **The condition on both configured remedies.** `doc/questions/A55` makes an `xmpMM:History`
    /// record part of the permission to derive at all, and `doc/rfc/0007` section 5b.1 makes the
    /// same demand of a supplied fact — so a document whose packet will not take the entry does not
    /// get the remedy either, exactly as `doc/questions/A48`'s constructions do not. The permission
    /// and its condition are one thing.
    pub(super) recorded_provenance: bool,
}

/// What this conversion has to write into the document's own provenance.
///
/// One argument rather than three, and the three are one question: *what does a reader of this
/// archive need told about how it was made that the file itself should say?* Each is `None` where
/// the conversion did none of that thing.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Provenance<'a> {
    /// The applied departures (`doc/rfc/0007` section 4.7.3).
    pub(super) departure: Option<&'a str>,
    /// The derived artefacts (`doc/questions/A55`).
    pub(super) derived: Option<&'a str>,
    /// The facts the operator supplied (`doc/rfc/0007` section 5b.1).
    pub(super) supplied: Option<&'a str>,
    /// What the source's encryption asserted, where one was removed (`doc/adr/1187`).
    pub(super) protection: Option<&'a str>,
    /// The streams an operator's tool fetched from outside the document (`doc/adr/1209`).
    pub(super) resolved: Option<&'a str>,
}

impl Prepared {
    /// Works out what can be built for this document, and only what a failed requirement asks for.
    #[expect(
        clippy::too_many_lines,
        reason = "this orchestrates every preparation the verb has — each a distinct fact about \
                  the document worked out once, in the dependency order the comments state — and \
                  splitting it would hide that order behind a call rather than reveal it"
    )]
    pub(super) fn of(
        plan: &ArchivePlan,
        document: &Document,
        input: &pdf_archive::Report,
        omit_identification: bool,
        provenance: Provenance<'_>,
        remedies: &super::remedies::Remedies,
    ) -> Self {
        let failed: BTreeSet<&'static str> =
            input.failures().map(|judgement| judgement.id).collect();
        let wanted = |rewrite: Rewrite| wanted_by(&failed, rewrite);
        let mut spare = Spare::of(document);
        let catalog = document.catalog().ok();
        let intent = match (wanted(Rewrite::OutputIntent), catalog.as_ref()) {
            (true, Some(catalog)) => prepare_intent(plan, document, catalog, &failed, &mut spare),
            (true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
            // Nothing failed that an output intent answers, so nothing is prepared and the
            // reason is never read: `decide` consults this only for a requirement `wanted`
            // has already found in the table.
            (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        // The `DeviceCMYK` sentence offers two licences and the output intent is the better one,
        // so the default is prepared only where the profile in hand cannot answer the clause —
        // which is `Answer::CmykUnderPartTwo`'s choice, taken here so that the packet knows
        // whether it has an action to record.
        let by_intent = intent
            .as_ref()
            .is_ok_and(|intent| intent.family == DeviceFamily::Cmyk);
        let default_cmyk = match (wanted(Rewrite::DefaultCmyk) && !by_intent, intent.as_ref()) {
            (true, Ok(intent)) => prepare_default_cmyk(document, intent, &mut spare),
            (true, Err(because)) => Err(*because),
            (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        // **One survey, for every preparation that asks what the pages actually drew** — the two
        // that read a font's advances and the two that prove a change of encoding moves no mark —
        // because a second walk of every content stream would double a large document's cost.
        // The repair `super::hexadecimal` makes needs the same walk the font rewrites do, so the
        // one survey answers both: a document that fails neither is never walked.
        let odd_digits = super::decision::odd_hexadecimal_digits_on_a_page(input);
        let survey = (SURVEYED.iter().any(|rewrite| wanted(*rewrite)) || odd_digits)
            .then(|| Survey::of(document));
        let hexadecimal = asked(odd_digits, || {
            super::hexadecimal::complete(document, survey.as_ref())
        });
        // **The replacement comes before every editor**, because a stream it replaces has no
        // producer bytes left for one to write into: ISO 19005-2 section 6.6.2.1's packet is
        // composed afresh where this tree cannot read the one the file holds, and the three
        // editors below skip those streams rather than editing bytes nothing will write.
        let fresh = asked(wanted(Rewrite::FreshMetadataPacket), || {
            prepare_fresh_packets(document, catalog.as_ref(), &failed)
        });
        // ISO 19005-2 section 6.6.2.1's header attributes are cut before the two writers that
        // rewrite the RDF this header wraps, on the bytes the file holds rather than theirs.
        let headers = asked(wanted(Rewrite::PacketHeaderAttributes), || {
            prepare_headers(document, input, fresh.as_ref().ok())
        });
        // Section 6.6.2.3.1's removals precede the restate — the catalog's packet is one they edit.
        let properties = asked(wanted(Rewrite::PropertyOutsideItsSchema), || {
            prepare_properties(document, input, headers.as_ref().ok(), fresh.as_ref().ok())
        });
        let removals = properties
            .as_ref()
            .map(|cleaned| names_removed(&cleaned.removed))
            .unwrap_or_default();
        // Section 6.6.2.3.3's prefixes move in what the removal left: the catalog's packet is one
        // both writers may edit, and a writer starting from the producer's original would undo
        // the one before it.
        let respelled = asked(wanted(Rewrite::ExtensionSchemaPrefixes), || {
            prepare_respellings(
                document,
                input,
                headers.as_ref().ok(),
                properties.as_ref().ok(),
                fresh.as_ref().ok(),
            )
        });
        // Section 6.6.4's amendment identifier is cut last of the four editors, in what the
        // three before it left: the schema is restated into this, so a cut taken from the
        // producer's original would undo them.
        let amended = asked(wanted(Rewrite::AmendmentIdentifierRemoved), || {
            prepare_amendment(
                document,
                catalog.as_ref(),
                Edited {
                    fresh: fresh.as_ref().ok(),
                    headers: headers.as_ref().ok(),
                    cleaned: properties.as_ref().ok(),
                    respelled: respelled.as_ref().ok(),
                    amended: None,
                    described: None,
                },
            )
        });
        // Section 6.6.2.3.2's container is written last of the five editors, into what the four
        // before it left: a schema described from a packet the removal has already cut is
        // described from what that packet now says.
        let described = asked(wanted(Rewrite::ExtensionSchemaDescribed), || {
            prepare_schema_descriptions(
                document,
                catalog.as_ref(),
                plan.preservations.iter().any(|preservation| {
                    preservation.site == SCHEMAS_REQUIREMENT && preservation.discard_undeterminable
                }),
                Edited {
                    fresh: fresh.as_ref().ok(),
                    headers: headers.as_ref().ok(),
                    cleaned: properties.as_ref().ok(),
                    respelled: respelled.as_ref().ok(),
                    amended: amended.as_ref().ok(),
                    described: None,
                },
            )
        });
        // **The appended pages, composed before the packet** and for the packet's sake: the
        // history entry naming them is a condition of the permission (`doc/adr/1014` section 5),
        // and an entry cannot be written for pages nobody has worked out yet. What they carry is
        // the producer's own packets — the bytes the removal above is about to edit — so this is
        // prepared after `properties` and reads what it found.
        // The annotations the two section 6.3.1 rows are about, worked out before the pages are
        // composed: a `preserve` remedy at either site keeps these annotations' normal
        // appearances, so the removal's population is what the composition is built from.
        let forbidden_annotations = asked(wanted(Rewrite::ForbiddenAnnotationRemoved), || {
            prepare_forbidden_annotations(document, plan.target)
        });
        // The three section 6.3.2 and 6.3.3 losses, each over the population its own requirement
        // reported, so that none of them can act on an annotation that requirement passed.
        let hidden_annotations = asked(wanted(Rewrite::HiddenAnnotationRemoved), || {
            prepare_hidden_annotations(document, plan.target)
        });
        let extra_appearance_states = asked(wanted(Rewrite::ExtraAppearanceStatesRemoved), || {
            prepare_extra_appearance_states(document, plan.target)
        });
        let automatic_states = asked(wanted(Rewrite::AutomaticStatesRemoved), || {
            sites::automatic_states(document)
        });
        let preserved = the_preserved_pages(
            plan,
            document,
            &failed,
            properties.as_ref(),
            fresh.as_ref(),
            forbidden_annotations.as_ref(),
            intent.is_ok() || states_an_output_intent(document, catalog.as_ref()),
            &mut spare,
        );
        let preserved_history = preserved
            .as_ref()
            .ok()
            .and_then(|composed| preserve::preserved_history(&composed.carried));
        let PreparedFonts {
            substitutes,
            metrics,
            recorded: font_history,
        } = prepare_fonts(
            plan,
            document,
            &mut spare,
            wanted(Rewrite::SubstituteFontProgram),
            metric_directions(&failed),
            survey.as_ref(),
        );
        let now = xmp::instant(std::time::SystemTime::now());
        let metadata = the_packet(
            plan.target,
            document,
            catalog.as_ref(),
            &mut spare,
            Recording {
                // Edited when the schema is wanted or when it is deliberately omitted (`A59`).
                schema: wanted(Rewrite::IdentificationSchema) || omit_identification,
                omit_identification,
                departure: provenance.departure,
                derived: provenance.derived,
                supplied: provenance.supplied,
                resolved: provenance.resolved,
                protection: provenance.protection,
                preserved: preserved_history.as_deref(),
                when: now.as_deref(),
                default_cmyk: default_cmyk.is_ok(),
                removals: &removals,
                substituted: &font_history,
                edited: Edited {
                    fresh: fresh.as_ref().ok(),
                    headers: headers.as_ref().ok(),
                    cleaned: properties.as_ref().ok(),
                    respelled: respelled.as_ref().ok(),
                    amended: amended.as_ref().ok(),
                    described: described.as_ref().ok(),
                },
            },
        );
        let recorded_it = now.is_some() && metadata.is_ok();
        let recorded_provenance = recorded_it;
        let nothing_removed = properties
            .as_ref()
            .is_ok_and(|cleaned| cleaned.removed.is_empty());
        let default_cmyk = unless_recorded(default_cmyk, recorded_it, NO_PLACE_TO_RECORD);
        let substitutes =
            unless_recorded(substitutes, recorded_it, NO_PLACE_TO_RECORD_A_SUBSTITUTION);
        let properties = unless_recorded(
            properties,
            recorded_it || nothing_removed,
            NO_PLACE_TO_RECORD_A_REMOVAL,
        );
        // The same construction, for the same reason: `doc/adr/1014` section 5 makes the history
        // entry part of the permission to append a page at all, so a packet that will not take it
        // withdraws the pages rather than leaving them unrecorded.
        let preserved = unless_recorded(preserved, recorded_it, NO_PLACE_TO_RECORD_A_PRESERVATION);
        // The fonts are the validator's own findings rather than a walk of this converter's:
        // the clause has four exemptions and reading them belongs to `pdf_archive`, so the set of
        // fonts that need a CMap is the set it named. A findings list is capped, and a document
        // failing at more fonts than the cap is one `doc/adr/0947`'s third stage refuses rather
        // than one this rewrite half-finishes.
        let to_unicode = asked(wanted(Rewrite::ToUnicode), || {
            to_unicode::derive(document, &unicode_fonts(input), &mut spare)
                .map_err(Because::NotBuiltYet)
        });
        // **Prepared after the removal and from it**, for the reason the composed pages are:
        // section 6.3.3's population is the annotations the *output* holds, and section 6.3.1's
        // removal is what says which of the source's those are (ADR 1105).
        let appearances = asked(wanted(Rewrite::AppearanceDictionary), || {
            prepare_appearances(
                document,
                plan.target,
                forbidden_annotations.as_ref().ok(),
                &mut spare,
            )
        });
        let structure = structure_tree(wanted(Rewrite::MarkInfo), document, catalog.as_ref());
        // One reading of §12.6's action trees for the three rewrites the action clauses ask for,
        // and the rules in force are the ones *this document failed*: a rule the file already
        // meets asks for nothing, which is `doc/adr/0947`'s second rule read over a population.
        let actions = asked(
            [
                Rewrite::ForbiddenActionRemoved,
                Rewrite::AdditionalActionsRemoved,
                Rewrite::WidgetActionEntryRemoved,
            ]
            .into_iter()
            .any(wanted),
            || super::actions::prepare(document, plan.target, &failed),
        );
        // The streams whose data the source keeps outside the file, answered out of what the
        // caller resolved and, for a stream stating the filter keys without a `/F`, out of the
        // file's own bytes (`doc/adr/1199`).
        // **The bytes are the same whoever fetched them** — `doc/adr/1199`'s seam, reached from
        // both sides: `--resolve-external-data` puts what this program's own rule permits in the
        // plan, and a configured `preserve` with a tool puts what the operator's program
        // returned here (`doc/adr/1209`). The plan wins a collision, because a caller who named
        // bytes for an object said so outright.
        let resolved: BTreeMap<ObjectId, std::sync::Arc<[u8]>> = remedies
            .external
            .iter()
            .map(|(at, bytes)| (*at, std::sync::Arc::clone(bytes)))
            .chain(
                plan.external_data
                    .iter()
                    .map(|(at, bytes)| (*at, std::sync::Arc::clone(bytes))),
            )
            .collect();
        let external_data = asked(wanted(Rewrite::ExternalDataEmbedded), || {
            super::external::embed(document, input, &resolved)
        });
        // ISO 19005-2 section 6.1.13's page-boundary limit, answered by the entry's own
        // absence: §7.7.3.3's Table 31 makes four of the five boxes optional and §14.11.2.1
        // gives each of those a default that is another box in the same file (`doc/adr/1210`).
        let boundaries = asked(wanted(Rewrite::PageBoundaryRemoved), || {
            super::boundaries::removal(document, input)
        });
        let already = Already {
            packet_headers: headers,
            survey: survey.as_ref(),
        };
        Self {
            intent,
            default_cmyk,
            metadata,
            to_unicode,
            appearances,
            fresh,
            amended,
            described,
            properties,
            respelled,
            substitutes,
            metrics,
            structure,
            signatures: signatures_if_rewritten(document, input),
            owed: Owed::of(plan, document, input, &mut spare, &failed, already),
            hexadecimal,
            external_data,
            boundaries,
            actions,
            forbidden_annotations,
            hidden_annotations,
            extra_appearance_states,
            automatic_states,
            preserved,
            recorded_provenance,
        }
    }

    /// What stops a rewrite this document's failures asked for, where something does.
    ///
    /// **The gate between the table and the document.** [`REMEDIES`] answers a requirement with a
    /// rewrite on the standard's evidence alone; whether *this* document can take that rewrite is
    /// a fact only a preparation knows, and `doc/adr/0947`'s rule is that a decision which cannot
    /// be carried out is a refusal rather than a plan. The reason returned is the preparation's
    /// own, never a paraphrase of it.
    pub(super) fn obstacle(&self, rewrite: Rewrite) -> Option<Because> {
        match rewrite {
            Rewrite::IdentificationSchema => self.metadata.as_ref().err().copied(),
            Rewrite::MarkInfo => self.structure.as_ref().err().copied(),
            Rewrite::ToUnicode => self.to_unicode.as_ref().err().copied(),
            Rewrite::PropertyOutsideItsSchema => self.properties.as_ref().err().copied(),
            Rewrite::FreshMetadataPacket => self.fresh.as_ref().err().copied(),
            Rewrite::AmendmentIdentifierRemoved => self.amended.as_ref().err().copied(),
            Rewrite::ExtensionSchemaDescribed => self.described.as_ref().err().copied(),
            Rewrite::ExtensionSchemaPrefixes => self.respelled.as_ref().err().copied(),
            Rewrite::AppearanceDictionary => self.appearances.as_ref().err().copied(),
            Rewrite::SubstituteFontProgram => self.substitutes.as_ref().err().copied(),
            Rewrite::RestateFontMetrics | Rewrite::RestateVerticalFontMetrics => {
                self.metrics.as_ref().err().copied()
            }
            Rewrite::RenderingIntent => self.owed.rendering_intents.as_ref().err().copied(),
            Rewrite::BlendModeNormal => self.owed.blend_modes.as_ref().err().copied(),
            Rewrite::NormalAppearanceFromState => {
                self.owed.appearance_states.as_ref().err().copied()
            }
            Rewrite::OptionalContentOrder => self.owed.orders.as_ref().err().copied(),
            Rewrite::PageResources => self.owed.page_resources.as_ref().err().copied(),
            Rewrite::DescriptorSetRemoved => self.owed.descriptor_sets.as_ref().err().copied(),
            Rewrite::CidToGidIdentity => self.owed.cid_to_gid.as_ref().err().copied(),
            Rewrite::PacketHeaderAttributes => self.owed.packet_headers.as_ref().err().copied(),
            Rewrite::SymbolicTrueTypeEncodingRemoved => {
                self.owed.symbolic_encodings.as_ref().err().copied()
            }
            Rewrite::StandardTrueTypeEncoding => {
                self.owed.standard_encodings.as_ref().err().copied()
            }
            Rewrite::SharedDestinationProfile => self.owed.shared_profile.as_ref().err().copied(),
            Rewrite::SpotColorantEntry => self.owed.colorants.as_ref().err().copied(),
            Rewrite::Jpeg2000ColourSpecifications => {
                self.owed.specifications.as_ref().err().copied()
            }
            Rewrite::PreservedAsPage => self.preserved.as_ref().err().copied(),
            Rewrite::ForbiddenAnnotationRemoved => {
                self.forbidden_annotations.as_ref().err().copied()
            }
            Rewrite::HiddenAnnotationRemoved => self.hidden_annotations.as_ref().err().copied(),
            Rewrite::ExtraAppearanceStatesRemoved => {
                self.extra_appearance_states.as_ref().err().copied()
            }
            Rewrite::AutomaticStatesRemoved => self.automatic_states.as_ref().err().copied(),
            Rewrite::HexadecimalDigitCompleted => self.hexadecimal.as_ref().err().copied(),
            Rewrite::ExternalDataEmbedded => self.external_data.as_ref().err().copied(),
            Rewrite::PageBoundaryRemoved => self.boundaries.as_ref().err().copied(),
            Rewrite::ForbiddenActionRemoved
            | Rewrite::AdditionalActionsRemoved
            | Rewrite::WidgetActionEntryRemoved => self.actions.as_ref().err().copied(),
            Rewrite::SignatureValueRemoved => self.signatures.obstacle,
            Rewrite::ForeignPermissionHandlers => {
                self.owed.foreign_handlers.as_ref().err().copied()
            }
            // Every other rewrite is decided by the standard and the requirement alone: it
            // either applies to an object or finds none, and finding none is not a refusal.
            _ => None,
        }
    }

    /// The objects the conversion adds, in the source's numbering, for [`enter`] to find.
    pub(super) fn added(&self) -> BTreeMap<ObjectId, Object> {
        let mut out = BTreeMap::new();
        if let Ok(intent) = &self.intent
            && let Some(object) = &intent.written
        {
            out.insert(intent.destination, object.clone());
        }
        if let Ok(metadata) = &self.metadata
            && let Some(object) = &metadata.written
        {
            out.insert(metadata.at, object.clone());
        }
        if let Ok(default) = &self.default_cmyk {
            for (id, object) in &default.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(maps) = &self.to_unicode {
            for (id, object) in &maps.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(appearances) = &self.appearances {
            for (id, object) in &appearances.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(substitutes) = &self.substitutes {
            for (id, object) in &substitutes.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(states) = &self.owed.appearance_states {
            for (id, object) in &states.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(composed) = &self.preserved {
            for (id, object) in &composed.written {
                out.insert(*id, object.clone());
            }
        }
        out
    }
}

/// `doc/pdf-a-mitigations.md` section 13.3's *owed, not optional* preparations, taken together.
///
/// A type of its own rather than seven more lines inside [`Prepared::of`], and the grouping is
/// the catalogue's own: each of these answers a requirement that was refused only because the
/// lossless rewrite had not been written. They share a rule with every other preparation —
/// **nothing is prepared that no failed requirement asked for** — so a conforming document has
/// its fonts, its annotations and its optional content left unread.
pub(super) struct Owed {
    /// The graphics states and images whose rendering intent is restated, or why none are.
    pub(super) rendering_intents: Result<Sites, Because>,
    /// The dictionaries whose `/BM` array is restated as `Normal`, or why none are.
    pub(super) blend_modes: Result<Sites, Because>,
    /// The annotations whose normal appearance collapses to one state, or why none do.
    pub(super) appearance_states: Result<AppearanceStates, Because>,
    /// The optional content configurations whose `/Order` is completed, or why none are.
    pub(super) orders: Result<CompletedOrders, Because>,
    /// The pages given the resources dictionary they inherit, or why none are.
    pub(super) page_resources: Result<PageResources, Because>,
    /// The font descriptors whose incomplete subset description goes, or why none do.
    pub(super) descriptor_sets: Result<DescriptorSets, Because>,
    /// The `CIDFont`s given `/CIDToGIDMap` `/Identity`, or why none are.
    pub(super) cid_to_gid: Result<Sites, Because>,
    /// The packets whose header loses a deprecated attribute, or why none do.
    ///
    /// **Prepared by [`Prepared::of`] rather than here**, and the reason is the one the field
    /// above it does not have: a packet takes up to three edits and this is the first of them, so
    /// it has to exist before the property removal and the identification schema are worked out.
    pub(super) packet_headers: Result<Headers, Because>,
    /// The symbolic TrueType fonts whose `/Encoding` goes, or why none do.
    pub(super) symbolic_encodings: Result<Sites, Because>,
    /// The non-symbolic TrueType fonts given one of the two admitted names, or why none are.
    pub(super) standard_encodings: Result<StandardEncodings, Because>,
    /// The output intents pointed at one shared destination profile, or why none are.
    pub(super) shared_profile: Result<SharedProfile, Because>,
    /// The `/Colorants` entries each `DeviceN` colour space gains, or why none are written.
    pub(super) colorants: Result<ColorantEntries, Because>,
    /// The `JPXDecode` images whose colour specification boxes are reduced, or why none are.
    pub(super) specifications: Result<Specifications, Because>,
    /// The permissions dictionary's keys outside Table 263, or why none are removed.
    pub(super) foreign_handlers: Result<ForeignHandlers, Because>,
}

impl Owed {
    /// Each of them, where a failed requirement asked for it.
    pub(super) fn of(
        plan: &ArchivePlan,
        document: &Document,
        input: &pdf_archive::Report,
        spare: &mut Spare,
        failed: &BTreeSet<&'static str>,
        already: Already<'_>,
    ) -> Self {
        let wanted = |rewrite: Rewrite| wanted_by(failed, rewrite);
        let Already {
            packet_headers,
            survey,
        } = already;
        Self {
            rendering_intents: asked(wanted(Rewrite::RenderingIntent), || {
                sites::rendering_intents(input)
            }),
            blend_modes: asked(wanted(Rewrite::BlendModeNormal), || {
                sites::blend_modes(document, input)
            }),
            appearance_states: asked(wanted(Rewrite::NormalAppearanceFromState), || {
                sites::appearance_states(document, input, spare)
            }),
            orders: asked(wanted(Rewrite::OptionalContentOrder), || {
                sites::completed_orders(document)
            }),
            page_resources: asked(wanted(Rewrite::PageResources), || {
                sites::page_resources(document, input)
            }),
            descriptor_sets: asked(wanted(Rewrite::DescriptorSetRemoved), || {
                sites::descriptor_sets(document, input)
            }),
            cid_to_gid: asked(wanted(Rewrite::CidToGidIdentity), || {
                sites::cid_to_gid_maps(document, input, plan.target)
            }),
            packet_headers,
            symbolic_encodings: asked(wanted(Rewrite::SymbolicTrueTypeEncodingRemoved), || {
                let survey = survey.ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
                sites::symbolic_truetype_encodings(document, input, survey)
            }),
            standard_encodings: asked(wanted(Rewrite::StandardTrueTypeEncoding), || {
                let survey = survey.ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
                sites::standard_truetype_encodings(document, input, survey)
            }),
            shared_profile: asked(wanted(Rewrite::SharedDestinationProfile), || {
                let catalog = document
                    .catalog()
                    .map_err(|_| Because::NotBuiltYet(NO_CATALOG))?;
                sites::shared_destination_profile(document, &catalog)
            }),
            colorants: asked(wanted(Rewrite::SpotColorantEntry), || {
                sites::colorant_entries(document, input)
            }),
            specifications: asked(wanted(Rewrite::Jpeg2000ColourSpecifications), || {
                jpeg2000::reduce_specifications(document, input)
            }),
            foreign_handlers: asked(wanted(Rewrite::ForeignPermissionHandlers), || {
                signatures::foreign_handlers(document)
            }),
        }
    }
}

/// Every signature the source carries, where the source is going to be rewritten.
///
/// A conforming source is copied byte for byte and keeps every signature it has, so nothing is
/// walked or hashed for it; a failing one is rewritten, and a rewrite invalidates every
/// signature whatever requirement asked for it — `doc/pdf-a-conversion-limits.md` section 3.6.
fn signatures_if_rewritten(document: &Document, input: &pdf_archive::Report) -> Signatures {
    if input.verdict() == Verdict::Fails {
        signatures::find(document)
    } else {
        Signatures::default()
    }
}

/// Whether the file already carries the structure tree a `/MarkInfo` would be a claim about.
///
/// `doc/pdf-a-conversion-limits.md` section 5.1: the converter will not invent a structure tree,
/// and writing `/Marked true` over a file that has none would be the same act in one key — a
/// claim that the file follows §14.8's conventions, made by the converter rather than
/// demonstrated by the file. So this is the fence rather than a debt, and says so.
fn structure_tree(
    wanted: bool,
    document: &Document,
    catalog: Option<&Dictionary>,
) -> Result<(), Because> {
    match (wanted, catalog) {
        (true, Some(catalog)) if !document.get_key(catalog, "StructTreeRoot").is_null() => Ok(()),
        (true, Some(_)) => Err(Because::TheFence(NO_STRUCTURE_TREE)),
        (true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
        (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
    }
}

/// What [`Prepared::of`] has already worked out by the time the owed rewrites are prepared.
///
/// Two things rather than seven arguments, and each is here because its *order* matters: the
/// header cut has to exist before the two writers that edit the same packets, and the survey has
/// to be made once for the four preparations that ask what the pages drew.
pub(super) struct Already<'a> {
    /// The packets whose header attributes were cut, or why none were.
    packet_headers: Result<Headers, Because>,
    /// The one content-stream survey, where any preparation asked for one.
    survey: Option<&'a Survey>,
}

/// The rewrites whose preparation reads what the pages drew.
///
/// Every one of them needs `pdf_archive::survey::Survey`, which walks each page's content
/// streams — so the walk is made once where any of them is wanted and not at all where none is.
const SURVEYED: [Rewrite; 5] = [
    Rewrite::SubstituteFontProgram,
    Rewrite::RestateFontMetrics,
    Rewrite::RestateVerticalFontMetrics,
    Rewrite::SymbolicTrueTypeEncodingRemoved,
    Rewrite::StandardTrueTypeEncoding,
];

/// Which of the two metric restatements a document's failures ask for.
///
/// Separate questions rather than one, because the two requirements are separate: ISO 19005-2
/// states only the horizontal one, so a part 2 target must leave a disagreeing `vmtx` exactly
/// where its producer left it.
fn metric_directions(failed: &BTreeSet<&'static str>) -> Directions {
    Directions {
        widths: wanted_by(failed, Rewrite::RestateFontMetrics),
        vertical: wanted_by(failed, Rewrite::RestateVerticalFontMetrics),
    }
}

/// Whether any requirement the document failed is answered by this rewrite.
fn wanted_by(failed: &BTreeSet<&'static str>, rewrite: Rewrite) -> bool {
    REMEDIES.iter().any(|remedy| {
        failed.contains(remedy.requirement) && remedy.answer.rewrites().contains(&Some(rewrite))
    })
}

/// One preparation, run only where a failed requirement asked for it.
fn asked<T>(wanted: bool, prepare: impl FnOnce() -> Result<T, Because>) -> Result<T, Because> {
    if wanted {
        prepare()
    } else {
        Err(Because::NotBuiltYet(NOT_ASKED_FOR))
    }
}

/// Object numbers nothing in the source resolves to, for the objects a conversion adds.
///
/// Past the highest number any cross-reference section names, and then checked one at a time:
/// **the objects a conversion adds are built in the source's numbering** like every other object
/// this verb rewrites, so a number that collided with a source object would silently replace it.
pub(super) struct Spare {
    /// The next number to try.
    pub(super) next: u32,
}

impl Spare {
    /// Begins past the highest object number the document's cross-reference sections name.
    fn of(document: &Document) -> Self {
        let highest = document.xref().object_numbers().max().unwrap_or(0);
        Self {
            next: highest.saturating_add(1),
        }
    }

    /// The next number the document resolves to nothing.
    pub(super) fn take(&mut self, document: &Document) -> Option<ObjectId> {
        for _ in 0..MAX_SPARE_NUMBERS {
            let id = ObjectId::new(self.next, 0);
            self.next = self.next.checked_add(1)?;
            if document.get(id) == Object::Null {
                return Some(id);
            }
        }
        None
    }
}

/// How many object numbers [`Spare`] tries before giving up.
const MAX_SPARE_NUMBERS: usize = 64;

/// The two rows that say a page drew in `DeviceRGB` where nothing licensed it.
///
/// ISO 19005-2 section 6.2.4.3 and ISO 19005-4 section 6.2.4.3, one row per part.
const DEVICE_RGB_ROWS: [&str; 2] = [
    "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent",
    "graphics/device-rgb-needs-a-default-a-blending-space-or-an-rgb-output-intent",
];

/// The two rows that say a page drew in `DeviceCMYK` where nothing licensed it.
///
/// The same two subclauses. `DeviceGray` has rows of its own and they are deliberately absent
/// from both lists: each part licenses grey through a PDF/A output intent of *any* family, so a
/// grey page asks nothing of the choice below.
const DEVICE_CMYK_ROWS: [&str; 2] = [
    "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
    "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent",
];

/// Which of the two shipped profiles this document's own device colour asks for.
///
/// **One document, one destination profile.** ISO 19005-2 section 6.2.3 and ISO 19005-4
/// section 6.2.3 require every entry of one `OutputIntents` array that states a
/// `DestOutputProfile` to name the *same* object, and section 6.2.4.3 of each licenses a device
/// colour space only through a destination profile of that space's own family. A page drawing in
/// both `DeviceRGB` and `DeviceCMYK` therefore cannot be licensed by output intents at all —
/// whichever profile is embedded, one of the two rows stays failed. So this is not a preference
/// between two good profiles; it is which row gets answered.
///
/// The rule is one sentence: **the CMYK profile where the document's unlicensed device colour is
/// CMYK and none of it is RGB, and the RGB profile otherwise.** What settles the tie is that only
/// the CMYK row has a second licence — ISO 19005-2 section 6.2.4.3 admits a `DeviceN`-based
/// `/DefaultCMYK`, which [`prepare_default_cmyk`] builds out of §10.4.2.5's own transform and
/// which `Answer::CmykUnderPartTwo` takes exactly when the profile in hand is not CMYK. Under
/// part 2, then, this order licenses a mixed document *completely* and the other order does not.
/// Part 4 states no such second licence, so a mixed document there keeps one refusal whichever
/// way the choice goes; taking the same order in both parts is what makes this one rule rather
/// than two, and `WRONG_FAMILY` is the sentence part 4's CMYK row is then refused with.
///
/// `doc/adr/1153` is the argument, including why part 4's page-level output intents are not the
/// answer to a document whose RGB and CMYK live on different pages.
fn shipped_for(failed: &BTreeSet<&'static str>) -> &'static [u8] {
    let cmyk = DEVICE_CMYK_ROWS.iter().any(|id| failed.contains(id));
    let rgb = DEVICE_RGB_ROWS.iter().any(|id| failed.contains(id));
    if cmyk && !rgb { CMYK } else { SRGB }
}

/// The `/OutputIntents` array the catalog states, resolved to its entries.
pub(super) fn output_intent_entries(document: &Document, catalog: &Dictionary) -> Vec<Object> {
    document
        .get_key(catalog, "OutputIntents")
        .as_array()
        .map(<[Object]>::to_vec)
        .unwrap_or_default()
}

/// The colour family a profile's header names, where it is one of ISO 19005's three.
fn family_of(stated: &Identification) -> Option<DeviceFamily> {
    // ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3 admit an output or a monitor
    // profile and no other class, and grey, RGB or CMYK and no other space. `pdf_archive` judges
    // both of a file's own profile; this asks the same question of one about to be written, so
    // that a conversion cannot add a profile the validator would then reject.
    if stated.class != *b"prtr" && stated.class != *b"mntr" {
        return None;
    }
    match &stated.space {
        b"GRAY" => Some(DeviceFamily::Gray),
        b"RGB " => Some(DeviceFamily::Rgb),
        b"CMYK" => Some(DeviceFamily::Cmyk),
        _ => None,
    }
}

/// Prepares the output intent: the profile it names, and where that profile comes from.
fn prepare_intent(
    plan: &ArchivePlan,
    document: &Document,
    catalog: &Dictionary,
    failed: &BTreeSet<&'static str>,
    spare: &mut Spare,
) -> Result<Intent, Because> {
    let wrong = Because::NotBuiltYet(NO_USABLE_PROFILE);
    // ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3: where an OutputIntents array holds
    // more than one entry, every entry stating a destination profile states the *same* object. So
    // a file that already holds one decides what a new entry may name, and adding a second
    // profile beside it is not open to this conversion at all.
    if let Some(destination) = held_destination_profile(document, catalog) {
        let Object::Stream(stream) = document.get(destination) else {
            return Err(wrong);
        };
        let data = document.decoded_stream_data(&stream).ok_or(wrong)?;
        let stated = Identification::read(&data).ok_or(wrong)?;
        let family = family_of(&stated).ok_or(wrong)?;
        return Ok(Intent {
            destination,
            written: None,
            family,
            reported: DestinationProfile {
                source: ProfileSource::AlreadyInTheFile,
                space: stated.space_name(),
                describes: stated.description,
                copyright: stated.copyright,
            },
        });
    }

    let (source, bytes) = match &plan.profile {
        Some(supplied) => (ProfileSource::Supplied, supplied.to_vec()),
        None => (ProfileSource::Shipped, shipped_for(failed).to_vec()),
    };
    let stated = Identification::read(&bytes).ok_or(wrong)?;
    let family = family_of(&stated).ok_or(wrong)?;
    let destination = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    Ok(Intent {
        destination,
        written: Some(profile_stream(&bytes, family)),
        family,
        reported: DestinationProfile {
            source,
            space: stated.space_name(),
            describes: stated.description,
            copyright: stated.copyright,
        },
    })
}

/// The object an entry of the catalog's `OutputIntents` array already names as its destination
/// profile, where one does.
fn held_destination_profile(document: &Document, catalog: &Dictionary) -> Option<ObjectId> {
    output_intent_entries(document, catalog)
        .iter()
        .filter_map(|entry| document.resolve(entry).as_dict().cloned())
        .find_map(|entry| {
            entry
                .get("DestOutputProfile")
                .and_then(Object::as_reference)
        })
}

/// The ICC profile stream an output intent names.
///
/// §14.11.5's Table 401 makes `DestOutputProfile` "[a]n ICC profile stream defining the
/// transformation from the PDF document's source colours to output device colourants" and says
/// that "[t]he format of the profile stream is the same as that used in specifying an `ICCBased`
/// colour space", which is §8.6.5.5's — hence the `/N`.
fn profile_stream(bytes: &[u8], family: DeviceFamily) -> Object {
    let components = match family {
        DeviceFamily::Gray => 1,
        DeviceFamily::Rgb => 3,
        DeviceFamily::Cmyk => 4,
    };
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"N"[..]), Object::Integer(components));
    let data = match flate_encode(bytes, COMPRESSION_LEVEL) {
        Some(encoded) => {
            dict.insert(
                Name::new(&b"Filter"[..]),
                Object::Name(Name::new(&b"FlateDecode"[..])),
            );
            encoded
        }
        None => bytes.to_vec(),
    };
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}

/// The output intent dictionary a conversion adds.
///
/// §14.11.5's Table 400 and Table 401 decide every entry, and one of them is a choice this file
/// records rather than derives. `OutputConditionIdentifier` is required and the table says that
/// "[i]f the intended production condition is not a recognised standard, the value of this entry
/// may be `Custom` or an application-specific, machine-readable name" — so `Custom` is the
/// standard's own word for exactly this case, and no convention is being copied from anywhere.
/// The same sentence then says that "[t]he `DestOutputProfile` entry defines the ICC profile, and
/// the `Info` entry shall be used for further human-readable identification", which is why `Info`
/// carries the profile's own `desc` tag: it is read out of the profile rather than written about
/// it. `S` is `GTS_PDFA1`, which both parts' section 6.2.3 requires of a PDF/A output intent.
pub(super) fn intent_dictionary(intent: &Intent) -> Dictionary {
    let mut out = Dictionary::new();
    out.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"OutputIntent"[..])),
    );
    out.insert(
        Name::new(&b"S"[..]),
        Object::Name(Name::new(&b"GTS_PDFA1"[..])),
    );
    out.insert(
        Name::new(&b"OutputConditionIdentifier"[..]),
        Object::String(b"Custom".to_vec().into()),
    );
    let info = intent
        .reported
        .describes
        .clone()
        .unwrap_or_else(|| "the ICC profile embedded beside this entry".to_owned());
    out.insert(
        Name::new(&b"Info"[..]),
        Object::String(info.into_bytes().into()),
    );
    out.insert(
        Name::new(&b"DestOutputProfile"[..]),
        Object::Reference(intent.destination),
    );
    out
}

/// The four names §8.6.6.5 reserves for the subtractive process colourants of a CMYK device.
///
/// > The names Cyan , Magenta , Yellow and Black are reserved to name the subtractive process
/// > colourants of a CMYK device.
///
/// So a `DeviceN` over exactly those four names the four components a `DeviceCMYK` value already
/// has, in the order §8.6.4.4 gives them, and none of them is a spot colourant — which is why the
/// space needs no `Colorants` entry under ISO 19005-2 section 6.2.4.4.
const PROCESS_COLOURANTS: [&[u8]; 4] = [b"Cyan", b"Magenta", b"Yellow", b"Black"];

/// A PostScript calculator function, of the kind §7.10.5 defines, stating one clause's arithmetic.
///
/// The clause is §10.4.2.5:
///
/// > The black component shall be added to each of the other components, which shall then be
/// > converted to their complementary colours by subtracting them each from 1.0.
///
/// and its formula is `red = 1.0 - min(1.0, cyan + black)` with green from magenta and blue from
/// yellow. The operator set of §7.10.5.2 has no `min`, so the clamp is the `dup 1 gt { pop 1 } if`
/// each line ends its addition with — the same value by the operators that clause does define.
///
/// The stack begins as `c m y k` and each line rotates one component to the top, adds the `k`
/// that is now one place further down, clamps, complements, and leaves the result behind; the
/// last line rotates the spent `k` to the top and drops it, leaving `red green blue`.
const CMYK_TO_RGB: &[u8] = b"{ 4 3 roll 1 index add dup 1 gt { pop 1 } if 1 exch sub\n\
    4 3 roll 2 index add dup 1 gt { pop 1 } if 1 exch sub\n\
    4 3 roll 3 index add dup 1 gt { pop 1 } if 1 exch sub\n\
    4 3 roll pop }";

/// Prepares the `DeviceN` `/DefaultCMYK`: the tint transform, and the space that names it.
fn prepare_default_cmyk(
    document: &Document,
    intent: &Intent,
    spare: &mut Spare,
) -> Result<DefaultCmyk, Because> {
    if intent.family != DeviceFamily::Rgb {
        return Err(Because::NotBuiltYet(NO_RGB_ALTERNATE));
    }
    let missing = Because::NotBuiltYet(NO_SPARE_OBJECT);
    let tint = spare.take(document).ok_or(missing)?;
    let colour_space = spare.take(document).ok_or(missing)?;
    Ok(DefaultCmyk {
        space: colour_space,
        written: [
            (tint, tint_transform()),
            (colour_space, device_n(tint, intent.destination)),
        ],
    })
}

/// The tint transform stream: §7.10.5's type 4 function over §10.4.2.5's arithmetic.
///
/// `/Domain` and `/Range` are §7.10.2's Table 38 entries, four in and three out, each component
/// over the unit interval §8.6.4.4 gives a `DeviceCMYK` component and §8.6.4.3 an additive one.
/// Written unfiltered, for the reason §14.3.2's metadata packet is: it is forty bytes of text,
/// and a reader who opens the file with an editor can see what was asserted about their colours.
fn tint_transform() -> Object {
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"FunctionType"[..]), Object::Integer(4));
    let unit_interval = |components: usize| {
        Object::Array(
            std::iter::repeat_with(|| [Object::Integer(0), Object::Integer(1)])
                .take(components)
                .flatten()
                .collect(),
        )
    };
    dict.insert(Name::new(&b"Domain"[..]), unit_interval(4));
    dict.insert(Name::new(&b"Range"[..]), unit_interval(3));
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(CMYK_TO_RGB.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: CMYK_TO_RGB.to_vec().into(),
        decryption_failed: false,
    }))
}

/// The colour space array: §8.6.6.5's `DeviceN`, over §8.6.5.5's `ICCBased` alternate.
fn device_n(tint: ObjectId, profile: ObjectId) -> Object {
    let names = PROCESS_COLOURANTS
        .into_iter()
        .map(|name| Object::Name(Name::new(name)))
        .collect();
    Object::Array(vec![
        Object::Name(Name::new(&b"DeviceN"[..])),
        Object::Array(names),
        Object::Array(vec![
            Object::Name(Name::new(&b"ICCBased"[..])),
            Object::Reference(profile),
        ]),
        Object::Reference(tint),
    ])
}

/// Constructs the appearance stream every annotation the requirement named would be given.
///
/// **Nothing is judged here either.** `pdf_archive::annotations_without_an_appearance` is the same
/// reading the two requirement rows are, and `pdf_model::appearance` is the construction the
/// viewer already draws from the annotation's own entries. What this decides is whether the
/// construction may be *written down*, and it refuses four cases rather than writing something
/// weaker than the clauses state.
///
/// **`forbidden` narrows the population to the annotations the output will still hold**, which is
/// what the clause asks about: ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 bind the
/// annotations of a conforming *file*, and section 6.3.1's removal takes some of the source's out
/// of it. Constructing an appearance for one of those would be drawing a picture for an
/// annotation this conversion is about to delete — and refusing the document because no such
/// picture can be drawn, which is what happened until ADR 1105, refuses it over a place the
/// output does not have. `None` where the removal was not prepared at all, which is every
/// document whose section 6.3.1 row did not fail (ADR 1099).
fn prepare_appearances(
    document: &Document,
    target: Target,
    forbidden: Option<&ForbiddenAnnotations>,
    spare: &mut Spare,
) -> Result<Appearances, Because> {
    let mut at = BTreeMap::new();
    let mut written = Vec::new();
    let mut constructed = Vec::new();
    let mut removed = 0_usize;
    for missing in pdf_archive::annotations_without_an_appearance(document, target) {
        let annotation = missing
            .at
            .ok_or(Because::NotBuiltYet(APPEARANCE_ON_A_DIRECT_ANNOTATION))?;
        if forbidden.is_some_and(|forbidden| forbidden.at.contains(&annotation)) {
            removed = removed.saturating_add(1);
            continue;
        }
        let dict = document
            .get(annotation)
            .as_dict()
            .cloned()
            .ok_or(Because::TheFence(APPEARANCE_NOT_DERIVABLE))?;
        if button_widget(document, &dict) {
            return Err(Because::NotBuiltYet(BUTTON_APPEARANCE_STATES));
        }
        let built = pdf_model::appearance::for_annotation(document, &dict)
            .ok_or(Because::TheFence(APPEARANCE_NOT_DERIVABLE))?;
        // Two refusals rather than one, because they are two facts and only one of them is about
        // this program: a construction that put marks on the page and owes an entry beside them is
        // partial, and one that put none is a subtype whose clause states no artwork anywhere.
        // The sentence a person reads is the difference, and the second is ADR 0816's fence.
        if built.owed.is_some() {
            return Err(Because::TheFence(if built.drawn {
                APPEARANCE_INCOMPLETE
            } else {
                APPEARANCE_NOT_DERIVABLE
            }));
        }
        let stream = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        at.insert(annotation, stream);
        written.push((stream, built.stream));
        constructed.push(WrittenAppearance {
            page: missing.page,
            subtype: missing
                .subtype
                .unwrap_or_else(|| "an annotation stating no subtype".to_owned()),
        });
    }
    Ok(Appearances {
        at,
        written,
        constructed,
        removed,
    })
}

/// Every annotation a target's part does not admit, and what taking each off the page costs.
///
/// ISO 19005-2 section 6.3.1 and ISO 19005-4 section 6.3.1, as one population.
/// `pdf_archive::annotations_of_a_forbidden_subtype` is the reading and this is what the rewrite
/// and the `preserve` remedy are both built from, so neither can act on an annotation the
/// requirement's own predicate would have passed.
#[derive(Debug)]
pub(super) struct ForbiddenAnnotations {
    /// The objects to take out of their pages' `/Annots` arrays.
    ///
    /// The popup of a removed annotation is in here beside it: §12.5.6.14 makes a popup the
    /// window belonging to some other annotation, and one whose parent has gone is a window onto
    /// nothing.
    pub(super) at: BTreeSet<ObjectId>,
    /// Every annotation removed, in page order, for the report and for `preserve`.
    pub(super) removed: Vec<RemovedAnnotation>,
}

/// One annotation a target does not admit, as the report names it and as `preserve` reads it.
///
/// **A list rather than a count**, for the reason `Conversion::removed` is one: an annotation that
/// is gone leaves nothing in the output to notice, and `doc/pdf-a-conversion-limits.md` section
/// 3.2 asks the report to say which page lost a mark. Whether it had one to lose is
/// [`Self::appearance`], because an annotation with no appearance stream took no mark off the
/// page when it went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedAnnotation {
    /// The annotation object itself.
    pub at: ObjectId,
    /// The zero-based page it was on.
    pub page: usize,
    /// Its `/Subtype`, or the words for an annotation that states none.
    pub subtype: String,
    /// The form `XObject` its `/AP` `/N` names, where it names one.
    ///
    /// `Some` means the page loses a mark when the annotation goes — the marks
    /// `remedy = "preserve"` keeps. `None` means it drew nothing of its own.
    pub appearance: Option<ObjectId>,
    /// The `/F` value its producer wrote, where the flags are why it was removed.
    ///
    /// §12.5.3's Table 167 numbers the bits, and a report that says which of them the file set is
    /// what lets a reader see whether the annotation was hidden, kept off paper or kept off the
    /// screen. `None` where the removal was about the subtype rather than the flags.
    pub flags: Option<i64>,
}

impl RemovedAnnotation {
    /// One removed annotation as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(
            vec![
                ("page".to_owned(), Value::count(self.page)),
                ("subtype".to_owned(), Value::text(self.subtype.clone())),
                (
                    "object".to_owned(),
                    Value::text(format!("{} {}", self.at.number, self.at.generation)),
                ),
                ("drew".to_owned(), Value::Bool(self.appearance.is_some())),
            ]
            .into_iter()
            .chain(
                self.flags
                    .map(|flags| ("flags".to_owned(), Value::Integer(flags))),
            )
            .collect(),
        )
    }
}

/// Why an annotation nothing can name is not removed.
///
/// §7.7.3.3's Table 31 requires `/Annots` to "contain indirect references to all annotations
/// associated with the page", and files exist that write a dictionary straight into the array
/// instead. This rewrite acts on objects, so there is nothing for it to take out;
/// `prepare_appearances` refuses the same case for the same reason.
const REMOVAL_OF_A_DIRECT_ANNOTATION: &str = "an annotation of a subtype ISO 19005 does not admit \
     is written directly into its page's Annots array rather than being an object of its own, and \
     the rewrite that removes one acts on objects. Nothing here can reach it";

/// The annotations the two section 6.3.1 rows are about, read off the document once.
fn prepare_forbidden_annotations(
    document: &Document,
    target: Target,
) -> Result<ForbiddenAnnotations, Because> {
    let mut at = BTreeSet::new();
    let mut removed = Vec::new();
    for forbidden in pdf_archive::annotations_of_a_forbidden_subtype(document, target) {
        let annotation = forbidden
            .at
            .ok_or(Because::NotBuiltYet(REMOVAL_OF_A_DIRECT_ANNOTATION))?;
        at.insert(annotation);
        // §12.5.6.2's Table 172 gives a markup annotation a `/Popup` naming the window it opens,
        // and that window is an annotation of the page in its own right. It goes with its parent
        // rather than being left pointing at an object the output does not hold.
        if let Some(dict) = document.get(annotation).as_dict()
            && let Some(popup) = dict.get("Popup").and_then(Object::as_reference)
        {
            at.insert(popup);
        }
        removed.push(RemovedAnnotation {
            at: annotation,
            page: forbidden.page,
            subtype: forbidden
                .subtype
                .unwrap_or_else(|| "an annotation stating no subtype".to_owned()),
            appearance: forbidden.normal_appearance,
            flags: None,
        });
    }
    Ok(ForbiddenAnnotations { at, removed })
}

/// The annotations whose stated `/F` ISO 19005 forbids, and what removing each costs.
///
/// [`ForbiddenAnnotations`]' shape over ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2's
/// population rather than section 6.3.1's, and a type of its own because the two removals are two
/// authorisations: an operator who accepts losing a `Movie` annotation has said nothing about the
/// review comment its producer hid.
#[derive(Debug)]
pub(super) struct HiddenAnnotations {
    /// The objects to take out of their pages' `/Annots` arrays.
    ///
    /// The popup of a removed annotation is in here beside it, on
    /// [`ForbiddenAnnotations::at`]'s reading of §12.5.6.14.
    pub(super) at: BTreeSet<ObjectId>,
    /// Every annotation removed, in page order, for the report.
    pub(super) removed: Vec<RemovedAnnotation>,
}

/// Why an annotation nothing can name is not removed for its flags.
///
/// [`REMOVAL_OF_A_DIRECT_ANNOTATION`]'s case at the other removal's site: §7.7.3.3's Table 31
/// requires `/Annots` to hold indirect references, files exist that write a dictionary straight
/// into the array, and a rewrite that acts on objects has nothing to take out.
const REMOVAL_OF_A_DIRECT_HIDDEN_ANNOTATION: &str = "an annotation whose F entry ISO 19005 \
     forbids is written directly into its page's Annots array rather than being an object of its \
     own, and the rewrite that removes one acts on objects. Nothing here can reach it";

/// The annotations the two section 6.3.2 flag rows are about, read off the document once.
fn prepare_hidden_annotations(
    document: &Document,
    target: Target,
) -> Result<HiddenAnnotations, Because> {
    let mut at = BTreeSet::new();
    let mut removed = Vec::new();
    for hidden in pdf_archive::annotations_the_flags_forbid(document, target) {
        let annotation = hidden
            .at
            .ok_or(Because::NotBuiltYet(REMOVAL_OF_A_DIRECT_HIDDEN_ANNOTATION))?;
        at.insert(annotation);
        if let Some(dict) = document.get(annotation).as_dict()
            && let Some(popup) = dict.get("Popup").and_then(Object::as_reference)
        {
            at.insert(popup);
        }
        removed.push(RemovedAnnotation {
            at: annotation,
            page: hidden.page,
            subtype: hidden
                .subtype
                .unwrap_or_else(|| "an annotation stating no subtype".to_owned()),
            appearance: hidden.normal_appearance,
            flags: Some(hidden.flags),
        });
    }
    Ok(HiddenAnnotations { at, removed })
}

/// The appearance dictionaries that lose every key but `/N`.
#[derive(Debug, Default)]
pub(super) struct ExtraAppearanceStates {
    /// The annotation objects whose `/AP` is rewritten to hold `/N` alone.
    pub(super) at: BTreeSet<ObjectId>,
}

/// Why an appearance dictionary with nothing to fall back to is not reduced.
///
/// §12.5.5's Table 170 gives `/R` and `/D` the default "the value of the N entry", which is what
/// makes dropping them a loss of artwork rather than of behaviour. Where the dictionary states no
/// `/N`, that default names nothing, and the reduction would leave an annotation carrying an
/// appearance dictionary that describes no appearance.
const NO_NORMAL_APPEARANCE_TO_FALL_BACK_TO: &str = "this annotation's appearance dictionary \
     states a rollover or down appearance and no normal one. ISO 32000-2 Table 170 makes the \
     normal appearance the default of both keys ISO 19005 section 6.3.3 forbids, so there is \
     nothing here for a reader to fall back to and dropping them would leave an appearance \
     dictionary describing no appearance";

/// Why an appearance dictionary nothing can name is not reduced.
const REDUCTION_OF_A_DIRECT_ANNOTATION: &str = "an annotation whose appearance dictionary states \
     more than the normal appearance is written directly into its page's Annots array rather \
     than being an object of its own, and this rewrite acts on objects. Nothing here can reach it";

/// The annotations the two section 6.3.3 appearance-dictionary rows are about.
fn prepare_extra_appearance_states(
    document: &Document,
    target: Target,
) -> Result<ExtraAppearanceStates, Because> {
    let mut at = BTreeSet::new();
    for extra in pdf_archive::annotations_with_extra_appearance_states(document, target) {
        if !extra.normal {
            return Err(Because::NotBuiltYet(NO_NORMAL_APPEARANCE_TO_FALL_BACK_TO));
        }
        at.insert(
            extra
                .at
                .ok_or(Because::NotBuiltYet(REDUCTION_OF_A_DIRECT_ANNOTATION))?,
        );
    }
    Ok(ExtraAppearanceStates { at })
}

/// Whether this annotation is the widget of a button field.
///
/// §12.7.4.1's Table 228 makes `/FT` inheritable, and `TechNote 0010`'s A023 resolves that an
/// unmerged widget's field type is read from the form field dictionary above it — which is why
/// this walks `/Parent` rather than reading the key off the annotation alone.
fn button_widget(document: &Document, annotation: &Dictionary) -> bool {
    if document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"Widget")
    {
        return false;
    }
    let mut node = annotation.clone();
    for _ in 0..MAX_FIELD_DEPTH {
        if let Some(kind) = document.get_key(&node, "FT").as_name() {
            return kind.as_bytes() == b"Btn";
        }
        let Some(parent) = node.get("Parent").and_then(Object::as_reference) else {
            return false;
        };
        let Some(above) = document.get(parent).as_dict().cloned() else {
            return false;
        };
        node = above;
    }
    false
}

/// Every metadata stream one metadata requirement was found to fail at.
///
/// The validator's own findings rather than a walk of this converter's, for
/// [`unicode_fonts`]'s reason: which properties a predefined schema defines is `pdf_archive`'s
/// reading. The *list* it names is capped, so a document failing at more streams than the cap
/// leaves one uncleaned — and `doc/adr/0947`'s third stage refuses that file rather than this
/// preparation half-finishing it.
fn schema_packets(input: &pdf_archive::Report, requirement: &str) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = input
        .failures()
        .filter(|judgement| judgement.id == requirement)
        .filter_map(|judgement| match &judgement.outcome {
            Outcome::Failed { places, .. } => Some(places),
            _ => None,
        })
        .flatten()
        .filter_map(|finding| finding.place.object)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The requirement a removed property answers.
const SCHEMA_REQUIREMENT: &str = "metadata/properties-use-known-schemas";

/// Takes every property a predefined schema does not define out of the packets that state them.
///
/// **Nothing is judged here.** `pdf_archive::properties_outside_their_schema` is the same reading
/// the requirement's own row is, asked of one packet's bytes, so a property this cuts is exactly a
/// property that row reported. What this decides is only whether the cut can be made at all — and
/// a packet left holding one afterwards refuses the document rather than leaving it half-edited,
/// which is `doc/pdf-a-conversion-limits.md` section 3.9's line.
fn prepare_properties(
    document: &Document,
    input: &pdf_archive::Report,
    headers: Option<&Headers>,
    fresh: Option<&Fresh>,
) -> Result<Cleaned, Because> {
    let mut packets = BTreeMap::new();
    let mut removed = Vec::new();
    for at in schema_packets(input, SCHEMA_REQUIREMENT) {
        // The fresh packet states no property at all, so there is nothing here to cut out of it
        // and the producer's own bytes are not what this stream will carry.
        if fresh.is_some_and(|fresh| fresh.packet(at).is_some()) {
            continue;
        }
        let Object::Stream(stream) = document.get(at) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
        // The header cut edited this packet already where it edited any, so the properties come
        // out of what that left rather than out of the producer's original.
        let bytes = headers
            .and_then(|headers| headers.packet(at).cloned())
            .unwrap_or_else(|| bytes.to_vec());
        let misused = pdf_archive::properties_outside_their_schema(&bytes);
        if misused.is_empty() {
            continue;
        }
        let names: Vec<XmpName> = misused
            .iter()
            .map(|property| property.name.clone())
            .collect();
        let cut =
            xmp::remove(&bytes, &names).map_err(|_| Because::NotBuiltYet(PACKET_NOT_CUTTABLE))?;
        // Read back rather than trusted: the writer says what it cut and the packet says what it
        // holds, and only the second of those is what a validator will see.
        if !pdf_archive::properties_outside_their_schema(&cut).is_empty() {
            return Err(Because::NotBuiltYet(PROPERTY_NOT_CUT));
        }
        packets.insert(at, cut);
        removed.extend(misused);
    }
    Ok(Cleaned { packets, removed })
}

/// The requirement the container answers.
const SCHEMAS_REQUIREMENT: &str = "metadata/extension-schemas-embedded";

/// The packets that gain a container describing the extension schemas they use.
#[derive(Debug, Default)]
pub(super) struct SchemasDescribed {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(super) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// Every schema described, for the report, in the order the packets state them.
    pub(super) described: Vec<DescribedSchema>,
}

/// One extension schema this conversion described, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribedSchema {
    /// The namespace URI, which is the packet's own.
    pub namespace: String,
    /// The prefix the packet spelled it with, which is the packet's own.
    pub prefix: String,
    /// Each property described: the local name and the value type its serialisation showed.
    pub properties: Vec<(String, String)>,
}

impl DescribedSchema {
    /// One described schema as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("namespace".to_owned(), Value::text(self.namespace.clone())),
            ("prefix".to_owned(), Value::text(self.prefix.clone())),
            (
                "properties".to_owned(),
                Value::Array(
                    self.properties
                        .iter()
                        .map(|(name, kind)| {
                            Value::Object(vec![
                                ("name".to_owned(), Value::text(name.clone())),
                                ("value_type".to_owned(), Value::text(kind.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}

/// The `pdfaSchema:schema` field, which no file states.
///
/// ISO 19005-2 section 6.6.2.3.3's Table 3 makes the field required and calls it the schema's
/// human-readable name. A producer who embedded no description of their own schema stated no
/// name for it either, so what goes here is a sentence saying exactly that rather than a name
/// this converter made up for somebody else's schema (`doc/adr/1245`).
const SCHEMA_NAME: &str = "an extension schema this file uses, described by the converter that \
     archived it because the producer described it nowhere";

/// The `pdfaProperty:category` field, which no file states.
///
/// Table 4 admits two values, `internal` and `external`, and the difference is whether a
/// property's value is derived from the document's content. Nothing derived this one: it is a
/// value the producer put in the packet, and this converter computed none of it. So `external`
/// is the one of the two that is true of what happened, and it is also the conservative one — a
/// reader treating a property as external does not recompute it.
const CATEGORY: &str = "external";

/// The `pdfaProperty:description` field, which no file states.
///
/// Table 4 makes it required and asks what the property means. The file does not say, and
/// `doc/pdf-a-mitigations.md`'s entry is plain that a container written from a packet describes
/// *shape* rather than meaning — so this says so rather than claiming a meaning nobody recorded.
const PROPERTY_DESCRIPTION: &str = "stated by this file's producer, who recorded no description \
     of it; what this container states is the value type the packet's own serialisation shows, \
     and nothing about what the property means";

/// Why a schema holding a property of undeterminable type is not described.
const TYPE_NOT_DETERMINABLE: &str = "this packet states a property in an extension schema it \
     describes nowhere, and the property's value is a structure - so the value type a container \
     would have to name is a custom one whose own fields' types are no more in the file than its \
     name is. doc/pdf-a-mitigations.md's entry calls this the undeterminable case and gives the \
     operator two answers: answer the site with `undeterminable = \"discard\"`, which takes the \
     property out of the packet and describes the rest, or correct the source";

/// Why a packet already carrying a container is not given a second one.
const CONTAINER_ALREADY_THERE: &str = "this packet states an extension schema container that \
     does not describe every schema it uses, and this conversion writes a container as a \
     description of its own rather than merging into one a producer wrote. Two would make the \
     packet state pdfaExtension:schemas twice, which the XMP data model forbids - a property \
     name is unique within its packet — so the file is refused rather than made worse";

/// Why a packet that cannot be written into gains no container.
const CONTAINER_NOT_WRITTEN: &str = "this packet uses an extension schema it describes nowhere, \
     and the packet cannot be written into: it is not one this tree can edit by span";

/// Why a packet still missing a description after the container refuses the document.
const SCHEMA_STILL_UNDESCRIBED: &str = "an extension schema this conversion described is still \
     undescribed in the packet afterwards, so the packet is half-edited rather than corrected - \
     and a half-edited packet is not written at all, which is the rule the property removal \
     states";

/// Writes a container for every extension schema a packet uses and describes nowhere.
///
/// **Nothing is judged here.** `pdf_archive::undescribed_schemas` is the same reading the
/// requirement's own row is, asked of one packet's bytes, so a schema this describes is exactly
/// one that row reported.
///
/// What this decides is the three fields ISO 19005-2 section 6.6.2.3.3 requires and no file
/// states — [`SCHEMA_NAME`], [`CATEGORY`] and [`PROPERTY_DESCRIPTION`] — each a fixed sentence
/// saying what it is, and each argued in `doc/adr/1245`. A property whose value type the
/// serialisation does not show stops the conversion unless the operator answered the site with
/// `undeterminable = "discard"`, in which case it is cut out of the packet and the rest
/// described.
fn prepare_schema_descriptions(
    document: &Document,
    catalog: Option<&Dictionary>,
    discard_undeterminable: bool,
    edited: Edited<'_>,
) -> Result<SchemasDescribed, Because> {
    let catalog_packet = catalog
        .and_then(|catalog| catalog.get("Metadata"))
        .and_then(Object::as_reference)
        .and_then(|at| match document.get(at) {
            Object::Stream(stream) => edited
                .packet(at)
                .or_else(|| document.decoded_stream_data(&stream).map(|it| it.to_vec())),
            _ => None,
        });
    let mut out = SchemasDescribed::default();
    for at in pdf_archive::metadata_streams(document) {
        let Object::Stream(stream) = document.get(at) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
        let Some(bytes) = edited.packet(at).or_else(|| {
            document
                .decoded_stream_data(&stream)
                .map(|bytes| bytes.to_vec())
        }) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
        let undescribed = pdf_archive::undescribed_schemas(&bytes, catalog_packet.as_deref());
        if undescribed.is_empty() {
            continue;
        }
        if states_a_container(&bytes) {
            return Err(Because::NotBuiltYet(CONTAINER_ALREADY_THERE));
        }
        let (bytes, undescribed) = if discard_undeterminable {
            cut_the_undeterminable(&bytes, undescribed, catalog_packet.as_deref())?
        } else {
            for schema in &undescribed {
                if schema
                    .properties
                    .iter()
                    .any(|property| property.value_type.is_none())
                {
                    return Err(Because::NotBuiltYet(TYPE_NOT_DETERMINABLE));
                }
            }
            (bytes, undescribed)
        };
        if undescribed.is_empty() {
            out.packets.insert(at, bytes);
            continue;
        }
        let rows: Vec<DescribedSchema> = undescribed
            .iter()
            .map(|schema| DescribedSchema {
                namespace: schema.namespace.clone(),
                prefix: schema.prefix.clone(),
                properties: schema
                    .properties
                    .iter()
                    .filter_map(|property| {
                        property
                            .value_type
                            .map(|kind| (property.local.clone(), kind.to_owned()))
                    })
                    .collect(),
            })
            .collect();
        let properties: Vec<Vec<xmp::PropertyDescription<'_>>> = rows
            .iter()
            .map(|schema| {
                schema
                    .properties
                    .iter()
                    .map(|(name, kind)| xmp::PropertyDescription {
                        name,
                        value_type: kind,
                        category: CATEGORY,
                        description: PROPERTY_DESCRIPTION,
                    })
                    .collect()
            })
            .collect();
        let schemas: Vec<xmp::SchemaDescription<'_>> = rows
            .iter()
            .zip(properties.iter())
            .map(|(schema, properties)| xmp::SchemaDescription {
                name: SCHEMA_NAME,
                namespace: &schema.namespace,
                prefix: &schema.prefix,
                properties,
            })
            .collect();
        let written = xmp::describe(&bytes, &schemas)
            .map_err(|_| Because::NotBuiltYet(CONTAINER_NOT_WRITTEN))?;
        // Read back rather than trusted, which is `prepare_properties`'s rule: the writer says
        // what it described and the packet says what it holds, and only the second is what a
        // validator will see.
        if !pdf_archive::undescribed_schemas(&written, catalog_packet.as_deref()).is_empty() {
            return Err(Because::NotBuiltYet(SCHEMA_STILL_UNDESCRIBED));
        }
        out.packets.insert(at, written);
        out.described.extend(rows);
    }
    if out.packets.is_empty() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    Ok(out)
}

/// Whether a packet already states an extension schema container.
fn states_a_container(packet: &[u8]) -> bool {
    xmp::Xmp::parse(packet).is_ok_and(|read| {
        read.properties()
            .iter()
            .any(|(name, _)| name.namespace == xmp::EXTENSION && name.local == "schemas")
    })
}

/// Takes out the properties whose value type the serialisation does not show, and re-reads.
///
/// `doc/pdf-a-mitigations.md`'s `undeterminable = "discard"`: the property goes and the schema is
/// described without it. The packet is re-read afterwards rather than trusted, and a schema left
/// with no property at all is one nothing in the packet uses any more — so it needs no container.
fn cut_the_undeterminable(
    bytes: &[u8],
    undescribed: Vec<pdf_archive::UndescribedSchema>,
    catalog_packet: Option<&[u8]>,
) -> Result<(Vec<u8>, Vec<pdf_archive::UndescribedSchema>), Because> {
    let mut names: Vec<XmpName> = Vec::new();
    for schema in &undescribed {
        for property in &schema.properties {
            if property.value_type.is_none() {
                names.push(XmpName {
                    namespace: schema.namespace.clone(),
                    local: property.local.clone(),
                });
            }
        }
    }
    if names.is_empty() {
        return Ok((bytes.to_vec(), undescribed));
    }
    let cut = xmp::remove(bytes, &names).map_err(|_| Because::NotBuiltYet(PACKET_NOT_CUTTABLE))?;
    let left = pdf_archive::undescribed_schemas(&cut, catalog_packet);
    if left
        .iter()
        .any(|schema| schema.properties.iter().any(|it| it.value_type.is_none()))
    {
        return Err(Because::NotBuiltYet(PROPERTY_NOT_CUT));
    }
    Ok((cut, left))
}

/// The three requirements a fresh packet answers.
///
/// ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1. The data-model row is part 2's
/// alone; a stream failing it under a part 4 target is never in [`PacketFault::requirement`]'s
/// answer for that target because the requirement does not bind there, and
/// [`prepare_fresh_packets`] asks only about the sites this document actually failed.
const PACKET_REQUIREMENTS: [&str; 3] = [
    "metadata/xmp-packets-well-formed",
    "metadata/xmp-packets-state-one-rdf-element",
    "metadata/xmp-packets-meet-the-xmp-data-model",
];

/// The packets this conversion replaces outright, because it cannot edit them.
#[derive(Debug, Default)]
pub(super) struct Fresh {
    /// The packet each such metadata stream is to carry in place of the one it holds.
    pub(super) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// What each stream's own packet broke, for the report, in object order.
    pub(super) replaced: Vec<ReplacedPacket>,
}

impl Fresh {
    /// The fresh packet one stream is to carry, where it is one of these.
    pub(super) fn packet(&self, at: ObjectId) -> Option<&Vec<u8>> {
        self.packets.get(&at)
    }
}

/// One metadata stream whose packet this conversion replaced, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacedPacket {
    /// The metadata stream, in the source's numbering.
    pub at: ObjectId,
    /// Whether it is the one the catalog states, which is the document's own metadata.
    pub catalogs: bool,
    /// What the producer's packet broke, in the words the requirement's own row uses.
    pub because: String,
}

impl ReplacedPacket {
    /// One replaced packet as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            (
                "object".to_owned(),
                Value::count(self.at.number.try_into().unwrap_or(usize::MAX)),
            ),
            (
                "generation".to_owned(),
                Value::count(self.at.generation.into()),
            ),
            ("catalog".to_owned(), Value::Bool(self.catalogs)),
            ("because".to_owned(), Value::text(self.because.clone())),
        ])
    }
}

/// What one fault costs a reader, in the words the report prints.
fn why_a_packet_was_replaced(fault: pdf_archive::PacketFault) -> &'static str {
    match fault {
        pdf_archive::PacketFault::WillNotDecode => {
            "the stream's data would not decode, so there was no packet to read"
        }
        pdf_archive::PacketFault::WillNotParse => "the packet is not well-formed XML",
        pdf_archive::PacketFault::ManyRdfElements => {
            "the packet serialises more than one rdf:RDF element, and the XMP standard \
             serialises one packet as one"
        }
        pdf_archive::PacketFault::BreaksTheDataModel => {
            "the packet breaks the XMP data model: a name repeated where it has to be unique, a \
             name carrying no namespace or one of the two the model keeps for itself, or an \
             array whose items are not all of one form"
        }
    }
}

/// Composes a fresh packet for every metadata stream whose own packet cannot be edited.
///
/// **A second reading of the three rows rather than their findings**, which is `doc/adr/1234`'s
/// rule: a findings list is capped where a document's metadata streams are not, so a replacement
/// driven off findings would leave a file carrying more bad packets than the cap half-edited.
/// `pdf_archive::packet_faults` is the rows' own judgement asked of one packet's bytes, so a
/// packet this replaces is exactly one those rows reported.
///
/// **Only the sites this document failed.** A stream breaking a rule the target does not state —
/// the data model under a part 4 target — is left alone, because `doc/adr/0947`'s first rule is
/// that nothing is changed that no failed requirement asked for.
///
/// Every packet written here states **no property**: the catalog's gains the identification
/// schema from [`prepare_metadata`], which is the one place in this verb that writes it, and an
/// object's own packet gains nothing because nothing in the file says what the unreadable one
/// meant.
fn prepare_fresh_packets(
    document: &Document,
    catalog: Option<&Dictionary>,
    failed: &BTreeSet<&'static str>,
) -> Result<Fresh, Because> {
    let catalog_packet = catalog
        .and_then(|catalog| catalog.get("Metadata"))
        .and_then(Object::as_reference);
    let mut fresh = Fresh::default();
    for at in pdf_archive::metadata_streams(document) {
        let Object::Stream(stream) = document.get(at) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
        // A stream whose data will not decode has no packet to judge, and that is itself the
        // well-formedness row's finding — so it is replaced on the same terms as one whose XML
        // is broken, rather than being passed over for want of bytes.
        let faults = match document.decoded_stream_data(&stream) {
            Some(bytes) => pdf_archive::packet_faults(&bytes),
            None => vec![pdf_archive::PacketFault::WillNotDecode],
        };
        let mut because: Vec<&'static str> = Vec::new();
        for fault in faults {
            if failed.contains(fault.requirement()) {
                because.push(why_a_packet_was_replaced(fault));
            }
        }
        if because.is_empty() {
            continue;
        }
        fresh.packets.insert(at, xmp::empty_packet());
        fresh.replaced.push(ReplacedPacket {
            at,
            catalogs: catalog_packet == Some(at),
            because: because.join("; "),
        });
    }
    if fresh.packets.is_empty() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    Ok(fresh)
}

/// The packets whose identification schema loses a malformed amendment identifier.
#[derive(Debug, Default)]
pub(super) struct Amended {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(super) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// Every identifier removed, spelled and with the value that was there, for the report.
    pub(super) removed: Vec<RemovedIdentifier>,
}

/// One amendment or corrigendum identifier this conversion cut, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedIdentifier {
    /// How the report spells it — the required prefix and the local name.
    pub spelled: String,
    /// What the packet stated for it, cut down to something a report can print.
    pub stated: String,
}

impl RemovedIdentifier {
    /// One removed identifier as JSON.
    pub(super) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("property".to_owned(), Value::text(self.spelled.clone())),
            ("stated".to_owned(), Value::text(self.stated.clone())),
        ])
    }
}

/// Why a packet holding a malformed amendment identifier cannot lose it.
const IDENTIFIER_NOT_CUT: &str = "this file states an amendment or corrigendum identifier that \
     is not the number and the year separated by a colon, and the packet it is in cannot be \
     edited in place: it is not one this tree can write into by span";

/// Why a packet still holding the identifier after the cut refuses the document.
const IDENTIFIER_STILL_THERE: &str = "an amendment identifier this conversion cut is still in \
     the packet afterwards, so the packet is half-edited rather than corrected - and a \
     half-edited packet is not written at all, which is the rule the property removal beside \
     this one states";

/// How long a stated value the report prints before cutting it.
///
/// A bound rather than a reading (`CLAUDE.md` principle 3): a packet is attacker-supplied and a
/// report is prose for a person. The cut is on a character boundary, so what is printed stays
/// text.
const STATED_LIMIT: usize = 48;

/// One stated value, cut down to something a report can print.
fn short(text: &str) -> String {
    match text.char_indices().nth(STATED_LIMIT) {
        Some((at, _)) => format!("{}...", &text[..at]),
        None => text.to_owned(),
    }
}

/// Cuts every amendment identifier of the wrong form out of the packet that states it.
///
/// **Nothing is judged here.** `pdf_archive::malformed_amendment_identifiers` is the same reading
/// the requirement's own row is, asked of one packet's bytes, so an entry this cuts is exactly an
/// entry that row reported. What this decides is only whether the cut can be made — and a packet
/// still holding one afterwards refuses the document rather than being left half-edited, which is
/// [`prepare_properties`]'s rule at the site beside it.
///
/// The population is the **document's own packet**, because ISO 19005-2 section 6.6.4 is a rule
/// about the identification schema and the row reads it out of the catalog's metadata stream.
fn prepare_amendment(
    document: &Document,
    catalog: Option<&Dictionary>,
    edited: Edited<'_>,
) -> Result<Amended, Because> {
    let at = catalog
        .and_then(|catalog| catalog.get("Metadata"))
        .and_then(Object::as_reference)
        .ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
    let Object::Stream(stream) = document.get(at) else {
        return Err(Because::NotBuiltYet(NOT_A_PACKET));
    };
    let bytes = match edited.packet(at) {
        Some(bytes) => bytes,
        None => document
            .decoded_stream_data(&stream)
            .ok_or(Because::NotBuiltYet(NOT_A_PACKET))?
            .to_vec(),
    };
    let malformed = pdf_archive::malformed_amendment_identifiers(&bytes);
    if malformed.is_empty() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    let stated: Vec<RemovedIdentifier> = xmp::Xmp::parse(&bytes)
        .map(|packet| {
            packet
                .properties()
                .iter()
                .filter(|(name, _)| malformed.contains(name))
                .map(|(name, value)| RemovedIdentifier {
                    spelled: format!("{IDENTIFICATION_PREFIX}:{}", name.local),
                    stated: match value {
                        xmp::Value::Text(text) => short(text),
                        _ => "a value that is not a simple one".to_owned(),
                    },
                })
                .collect()
        })
        .unwrap_or_default();
    let cut =
        xmp::remove(&bytes, &malformed).map_err(|_| Because::NotBuiltYet(IDENTIFIER_NOT_CUT))?;
    // Read back rather than trusted, which is `prepare_properties`'s rule: the writer says what
    // it cut and the packet says what it holds, and only the second is what a validator sees.
    if !pdf_archive::malformed_amendment_identifiers(&cut).is_empty() {
        return Err(Because::NotBuiltYet(IDENTIFIER_STILL_THERE));
    }
    Ok(Amended {
        packets: BTreeMap::from([(at, cut)]),
        removed: stated,
    })
}

/// The requirement the respelling answers.
const CONTAINER_REQUIREMENT: &str = "metadata/extension-schema-container-fields";

/// Moves every container field's prefix to the one its table requires, in the packets that state
/// them.
///
/// **Nothing is judged here**, [`prepare_properties`]'s rule: `pdf_archive`'s
/// `extension_container_fields` is the same reading the requirement's own row is, asked of one
/// packet's bytes, so a prefix this moves is exactly a prefix that row reported. What this decides
/// is only whether the move can be made — and a container missing a field outright cannot be
/// corrected at all, because nothing in the file says what the field would hold.
fn prepare_respellings(
    document: &Document,
    input: &pdf_archive::Report,
    headers: Option<&Headers>,
    cleaned: Option<&Cleaned>,
    fresh: Option<&Fresh>,
) -> Result<Respelled, Because> {
    let required: Vec<xmp::RequiredPrefix<'_>> = pdf_archive::REQUIRED_PREFIXES
        .iter()
        .map(|(namespace, prefix)| xmp::RequiredPrefix { namespace, prefix })
        .collect();
    let mut packets = BTreeMap::new();
    let mut fields = 0usize;
    for at in schema_packets(input, CONTAINER_REQUIREMENT) {
        // Replaced rather than edited, for [`prepare_properties`]'s reason: a fresh packet
        // describes no extension schema, so it states no container field to respell.
        if fresh.is_some_and(|fresh| fresh.packet(at).is_some()) {
            continue;
        }
        let Object::Stream(stream) = document.get(at) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
        // The two writers before this one edited this packet already where they edited any, so
        // the prefixes move in what they left rather than in the producer's original.
        let bytes = cleaned
            .and_then(|cleaned| cleaned.packets.get(&at).cloned())
            .or_else(|| headers.and_then(|headers| headers.packet(at).cloned()))
            .unwrap_or_else(|| bytes.to_vec());
        let faults = pdf_archive::extension_container_fields(&bytes);
        if faults.is_empty() {
            continue;
        }
        if faults.iter().any(|fault| !fault.misspelled) {
            return Err(Because::NotBuiltYet(FIELD_NOT_IN_THE_FILE));
        }
        let respelled = xmp::respell(&bytes, &required)
            .map_err(|_| Because::NotBuiltYet(PACKET_NOT_RESPELLABLE))?;
        // Read back rather than trusted, for the removal's reason: the writer says what it moved
        // and the packet says what it holds, and only the second is what a validator will see.
        if !pdf_archive::extension_container_fields(&respelled).is_empty() {
            return Err(Because::NotBuiltYet(PREFIX_NOT_RESPELLED));
        }
        packets.insert(at, respelled);
        fields = fields.saturating_add(faults.len());
    }
    Ok(Respelled { packets, fields })
}

/// The pages a configured `preserve` remedy appends, or why none are.
///
/// **Nothing is composed for a conversion that asked for none** — `doc/adr/0947`'s first rule
/// applied to a remedy — so a caller naming no preservation never loads a font or measures a page.
///
/// Two sites reach a page, and they are the two kinds of content `preserve` knows:
///
/// - ISO 19005-2 section 6.6.2.3.1's removal is about to edit a packet, so the packet **as the
///   producer wrote it** is appended: the whole packet rather than the properties alone, because a
///   value cut down to what a report can print is not the value, and because the packet is what
///   `doc/rfc/0007` section 2 names — *instead of losing metadata it could be appended or prefixed
///   as an extra page*;
/// - either section 6.3.1 row's removal is about to take an annotation off a page, so its **normal
///   appearance** is appended: the form `XObject` the producer wrote, invoked where §12.5.5 puts
///   it. `doc/adr/1099` is the argument.
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "each argument is one preparation this composition reads and none of them can be \
              bundled without a struct built at exactly one call site; and the body is one \
              sequence per kind of content a page carries, whose order the comments state"
)]
fn the_preserved_pages(
    plan: &ArchivePlan,
    document: &Document,
    failed: &BTreeSet<&'static str>,
    properties: Result<&Cleaned, &Because>,
    fresh: Result<&Fresh, &Because>,
    forbidden: Result<&ForbiddenAnnotations, &Because>,
    has_output_intent: bool,
    spare: &mut Spare,
) -> Result<Composed, Because> {
    let asked = |site: &'static str| {
        plan.preservations
            .iter()
            .any(|preservation| preservation.site == site && preservation.by_page)
            && failed.contains(site)
    };
    // ISO 19005-2 section 6.6.2.1's three rows, whose rewrite replaces a packet rather than
    // editing it: the bytes the producer wrote stop being metadata, so `preserve` keeps them as
    // content. One keep per replaced stream, under the first of the three sites the operator
    // answered and this document failed, because one page of bytes cannot be split between rows
    // that all named the same stream.
    let replaced_sites: Vec<&'static str> = PACKET_REQUIREMENTS
        .into_iter()
        .filter(|site| asked(site))
        .collect();
    let replaced_site = replaced_sites.first().copied();
    let replaced = match replaced_site {
        // The replacement is what makes the file conform and the page is what keeps what it
        // replaces, so a document whose packets cannot be replaced gets neither — the property
        // site's own construction, for its reason.
        Some(_) => {
            let fresh = fresh.map_err(|because| *because)?;
            let mut packets: Vec<(ObjectId, std::sync::Arc<[u8]>)> = Vec::new();
            for at in fresh.packets.keys() {
                let Object::Stream(stream) = document.get(*at) else {
                    return Err(Because::NotBuiltYet(NOT_A_PACKET));
                };
                // The one place a preservation gives up rather than refusing the conversion:
                // there are no bytes to lay out, so there is nothing this page could carry, and
                // the stream is replaced with nothing preserved. Named in the report all the
                // same, because the replacement itself is reported per stream.
                let Some(bytes) = document.decoded_stream_data(&stream) else {
                    continue;
                };
                packets.push((*at, bytes));
            }
            packets
        }
        None => Vec::new(),
    };
    let packets = if asked(SCHEMA_REQUIREMENT) {
        // The removal is what makes the file conform; the pages are what keep what it removes. A
        // document whose packet cannot be cut gets neither, and the reason is the cut's own.
        let cleaned = properties.map_err(|because| *because)?;
        let mut packets: Vec<(ObjectId, std::sync::Arc<[u8]>)> = Vec::new();
        for at in cleaned.packets.keys() {
            let Object::Stream(stream) = document.get(*at) else {
                return Err(Because::NotBuiltYet(NOT_A_PACKET));
            };
            let Some(bytes) = document.decoded_stream_data(&stream) else {
                return Err(Because::NotBuiltYet(NOT_A_PACKET));
            };
            packets.push((*at, bytes));
        }
        packets
    } else {
        Vec::new()
    };
    let annotation_site = SUBTYPE_REQUIREMENTS
        .into_iter()
        .find(|site| asked(site))
        .map(|site| {
            // **Before the relocation is planned, not after.** `doc/adr/1163`: marks put back onto
            // the producer's own page are drawn by operators no marked-content sequence brackets,
            // which §14.8.2.2.1 makes an artifact in a tagged document — and the marks are the
            // producer's real content. Naming a structure type for them is what this conversion
            // will not do, so the refusal is the same one an appended page of marks gets.
            if tagged::describes_its_content(document) {
                return Err(Because::NotBuiltYet(tagged::MARKS_IN_A_TAGGED_DOCUMENT));
            }
            // The removal is what makes the file conform and the marks are what a preserve keeps,
            // so a document whose annotations cannot be removed gets neither — the packet site's
            // own construction, for its reason.
            let removals = forbidden.map_err(|because| *because)?;
            let marks = marks_of(document, removals)?;
            relocate_marks(document, site, removals, &marks, spare).map(|plan| (site, plan))
        })
        .transpose()?;
    if packets.is_empty() && replaced.is_empty() && annotation_site.is_none() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    let mut keeps: Vec<preserve::Keep<'_>> = packets
        .iter()
        .map(|(at, bytes)| preserve::Keep {
            site: SCHEMA_REQUIREMENT,
            subject: format!(
                "the XMP metadata packet object {} {} holds, as the producer wrote it",
                at.number, at.generation
            ),
            content: preserve::Kept::Text(bytes),
            declined: None,
        })
        .collect();
    if let Some(site) = replaced_site {
        keeps.extend(replaced.iter().map(|(at, bytes)| preserve::Keep {
            site,
            subject: format!(
                "the XMP metadata packet object {} {} held, as the producer wrote it",
                at.number, at.generation
            ),
            content: preserve::Kept::Text(bytes),
            declined: None,
        }));
    }
    // **The relocations are decided before the appended pages are composed**, because a mark that
    // could not be relocated onto the producer's own page (`doc/adr/1123`'s two refusals) falls
    // back to the appended page, which is `doc/adr/1099`'s mechanism kept whole. So the fallback
    // marks join the keeps `compose` lays out, carrying the sentence that says why.
    let relocation = annotation_site.map(|(site, plan)| {
        for (marks, subject, declined) in plan.fallback {
            keeps.push(preserve::Keep {
                site,
                subject,
                content: preserve::Kept::Marks(marks),
                declined: Some(declined),
            });
        }
        (plan.relocations, plan.written, plan.rows)
    });
    let mut composed = preserve::compose(document, &keeps, has_output_intent, spare)?;
    // **One act answers all three rows, and each of them gets a row saying so.** A stream's
    // packet is laid out once however many of ISO 19005-2 section 6.6.2.1's rules it broke, so
    // the pages are the same pages — and `super::configured` asks, per requirement, whether the
    // preservation carried anything for it. A row left out would refuse a requirement this
    // conversion did answer.
    let also: Vec<Preserved> = composed
        .carried
        .iter()
        .filter(|row| Some(row.site) == replaced_site)
        .flat_map(|row| {
            replaced_sites.iter().skip(1).map(|site| Preserved {
                site,
                ..row.clone()
            })
        })
        .collect();
    composed.carried.extend(also);
    if let Some((relocations, written, rows)) = relocation {
        composed.written.extend(written);
        composed.carried.extend(rows);
        composed.relocations = relocations;
    }
    Ok(composed)
}

/// What relocating a page's forbidden-annotation marks decided: the on-page edits and the fallbacks.
///
/// `doc/adr/1123`. The marks that could be put back where §12.5.5 had them are in [`Self::rows`]
/// and drive [`Self::relocations`] and [`Self::written`]; the marks a refusal sent to an appended
/// page instead are in [`Self::fallback`], each with the sentence naming its refusal.
struct RelocationPlan {
    /// The producer pages the rewrite edits, keyed by page object.
    relocations: BTreeMap<ObjectId, preserve::RelocatedPage>,
    /// The `q`-prepend and closing content streams the walk adds, in the source's numbering.
    written: Vec<(ObjectId, Object)>,
    /// The report rows for marks relocated onto the producer's own page.
    rows: Vec<Preserved>,
    /// The marks a refusal sent to an appended page: the marks, the report subject, the reason.
    fallback: Vec<(preserve::Marks, String, &'static str)>,
}

/// One removed annotation's report subject, named as the report names it.
fn annotation_subject(removed: &RemovedAnnotation) -> String {
    format!(
        "the normal appearance of the {} annotation object {} {} held on page {}, as the producer \
         wrote it",
        removed.subtype,
        removed.at.number,
        removed.at.generation,
        removed.page.saturating_add(1)
    )
}

/// Decides, per page, which forbidden-annotation marks relocate onto the producer's page.
///
/// **The construction `doc/adr/1123` builds.** Marks are grouped by the page they were on, because
/// the `q`-prepend, the closing stream and the `/Contents` array are one edit per page however many
/// appearances land on it. A page whose producer content is unbalanced or too deeply nested, or
/// whose `/Contents` this cannot wrap, sends all of its marks to an appended page; a single
/// appearance a remaining unhidden annotation would sit over sends only itself.
fn relocate_marks(
    document: &Document,
    site: &'static str,
    removals: &ForbiddenAnnotations,
    marks: &[(&RemovedAnnotation, preserve::Marks)],
    spare: &mut Spare,
) -> Result<RelocationPlan, Because> {
    let tree = pdf_model::Pages::new(document);
    let mut plan = RelocationPlan {
        relocations: BTreeMap::new(),
        written: Vec::new(),
        rows: Vec::new(),
        fallback: Vec::new(),
    };
    // The marks in the order `marks_of` produced them — page order — grouped by page index so that
    // one page's edit is decided once. `BTreeMap` keeps the groups in page order for the report.
    let mut by_page: BTreeMap<usize, Vec<&(&RemovedAnnotation, preserve::Marks)>> = BTreeMap::new();
    for entry in marks {
        by_page.entry(entry.0.page).or_default().push(entry);
    }
    for (page_index, group) in by_page {
        let fall_back = |plan: &mut RelocationPlan, reason: &'static str| {
            for (removed, marks) in group.iter().map(|entry| (entry.0, entry.1.clone())) {
                plan.fallback
                    .push((marks, annotation_subject(removed), reason));
            }
        };
        // A page the model cannot resolve to an object of its own cannot be the one this edits, so
        // its marks take the appended page. This is `doc/adr/1099`'s mechanism, unchanged.
        let Some(page) = tree.get(page_index) else {
            fall_back(&mut plan, PAGE_NOT_RESOLVED);
            continue;
        };
        let Some(page_id) = page.id else {
            fall_back(&mut plan, PAGE_NOT_RESOLVED);
            continue;
        };
        // The `q`/`Q` count must be taken over *all* of the producer's content, so a part that
        // does not decode — an `LZWDecode` stream the reader does not implement, a damaged one —
        // would leave the count short and the closing stream unbalanced. That page's marks take an
        // appended page rather than a miscounted relocation (trap 5: an unsupported input stays
        // loud). §7.8.2 concatenates the parts into the one sequence §8.4.2 is about.
        let (content, issues) = page.content_with_report(document);
        if !issues.is_empty() {
            fall_back(&mut plan, CONTENT_DID_NOT_DECODE);
            continue;
        }
        let depth = match preserve::producer_open_depth(document, &content, &page.resources) {
            Ok(depth) => depth,
            Err(Because::NotBuiltYet(reason)) => {
                fall_back(&mut plan, reason);
                continue;
            }
            Err(other) => return Err(other),
        };
        let original = page.dict.get("Contents").cloned().unwrap_or(Object::Null);
        let remaining = remaining_unhidden_rects(document, &page, &removals.at);
        let mut on_page: Vec<preserve::OnPage> = Vec::new();
        let mut relocated_rows: Vec<Preserved> = Vec::new();
        let mut taken: BTreeSet<Vec<u8>> = BTreeSet::new();
        for (removed, marks) in group.iter().map(|entry| (entry.0, entry.1.clone())) {
            let Some(rect) = annotation_rect(document, removed.at) else {
                // §12.5.5 fits the appearance to the /Rect, so a mark whose landing rectangle
                // cannot be read is one whose overlap this cannot judge — it takes the appended page.
                plan.fallback
                    .push((marks, annotation_subject(removed), NO_RECTANGLE_TO_JUDGE));
                continue;
            };
            let rect = preserve::normalise_rect(rect);
            if remaining
                .iter()
                .any(|other| preserve::overlaps(rect, *other))
            {
                plan.fallback.push((
                    marks,
                    annotation_subject(removed),
                    preserve::REMAINING_ANNOTATION_OVER_THE_MARKS,
                ));
                continue;
            }
            let name = preserve::free_xobject_name(document, &page.resources, &mut taken);
            on_page.push(preserve::OnPage {
                appearance: marks.appearance,
                placement: marks.placement,
                name,
            });
            relocated_rows.push(Preserved {
                site,
                subject: annotation_subject(removed),
                pages: vec![page_index],
                placement: preserve::PLACEMENT_ON_PAGE,
                face: SetIn::NoText,
                declined: None,
            });
        }
        if on_page.is_empty() {
            continue;
        }
        let (relocated, written) = preserve::relocate_a_page(
            document,
            depth,
            &original,
            &page.resources,
            &on_page,
            spare,
        )?;
        plan.relocations.insert(page_id, relocated);
        plan.written.extend(written);
        plan.rows.extend(relocated_rows);
    }
    Ok(plan)
}

/// The normalised rectangles of the annotations that remain on a page and would draw over marks.
///
/// The population `doc/adr/1120` section 4 fixes: every annotation the removal leaves behind
/// (`removed` is [`ForbiddenAnnotations::at`]), that §12.5.3's flags do not hide — clear of both
/// `Hidden` (bit 2) and `NoView` (bit 6) — with a rectangle to read. The standard states no
/// painting order among annotations, so this is the whole of what a converter can know is over the
/// marks, and it over-refuses rather than guess an order.
fn remaining_unhidden_rects(
    document: &Document,
    page: &pdf_model::Page,
    removed: &BTreeSet<ObjectId>,
) -> Vec<[f32; 4]> {
    let Some(annots) = document
        .get_key(&page.dict, "Annots")
        .as_array()
        .map(<[Object]>::to_vec)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in &annots {
        let Some(id) = entry.as_reference() else {
            continue;
        };
        if removed.contains(&id) {
            continue;
        }
        let Some(dict) = document.get(id).as_dict().cloned() else {
            continue;
        };
        // §12.5.3's Table 167: bit 2 is `Hidden` and bit 6 is `NoView`. An annotation set to
        // either draws nothing a reader sees, so it is not a mark the relocated appearance can go
        // under. An absent or non-integer `/F` states no flags and is neither.
        let flags = document.get_key(&dict, "F").as_integer().unwrap_or(0);
        if flags & HIDDEN != 0 || flags & NO_VIEW != 0 {
            continue;
        }
        if let Some(rect) = annotation_rect(document, id) {
            out.push(preserve::normalise_rect(rect));
        }
    }
    out
}

/// One annotation's `/Rect` as four finite numbers, or `None` where it states none this can read.
fn annotation_rect(document: &Document, id: ObjectId) -> Option<[f32; 4]> {
    let dict = document.get(id).as_dict().cloned()?;
    let rect = document.get_key(&dict, "Rect");
    let values = rect.as_array()?;
    if values.len() != 4 {
        return None;
    }
    let mut out = [0.0_f32; 4];
    for (slot, value) in out.iter_mut().zip(values) {
        let number = document.resolve(value).as_number()?;
        if !number.is_finite() {
            return None;
        }
        // A `/Rect` is device-independent points; the cast to the geometry the overlap test uses
        // loses no coordinate any page states at the scale annotations are placed.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a rectangle coordinate in points, compared at a rectangle's own scale"
        )]
        let number = number as f32;
        *slot = number;
    }
    Some(out)
}

/// §12.5.3's Table 167 `Hidden` flag: bit 2, counting from 1.
const HIDDEN: i64 = 1 << 1;

/// §12.5.3's Table 167 `NoView` flag: bit 6, counting from 1.
const NO_VIEW: i64 = 1 << 5;

/// Why a page with no object identity of its own takes the appended page for its marks.
const PAGE_NOT_RESOLVED: &str = "the page the annotation was on could not be resolved to an object \
     this conversion can add content to, so its marks take an appended page";

/// Why a mark whose landing rectangle cannot be read takes the appended page.
const NO_RECTANGLE_TO_JUDGE: &str = "the annotation states no rectangle ISO 32000-2 \u{a7}12.5.5 \
     could fit its appearance to, so whether a remaining annotation would sit over the relocated \
     marks cannot be judged, and they take an appended page";

/// Why a page whose own content does not fully decode takes the appended page for its marks.
const CONTENT_DID_NOT_DECODE: &str = "part of the producer's own page content does not decode — a \
     filter this reader does not implement, or a damaged stream — so the q/Q balance ISO 32000-2 \
     \u{a7}8.4.2 requires cannot be counted over all of it, and the marks take an appended page \
     rather than a relocation whose closing stream might be unbalanced";

/// The two requirements whose refusal a page of preserved marks answers.
const SUBTYPE_REQUIREMENTS: [&str; 2] = [
    "annotations/subtype-defined-in-iso-32000-1",
    "annotations/subtype-defined-in-iso-32000-2",
];

/// Why an annotation with no normal appearance stream is refused rather than dropped quietly.
///
/// The operator asked for what the removal would lose to be kept, and for this annotation there is
/// nothing in the file to keep: §12.5.5 makes the normal appearance the marks a reader draws, and
/// an annotation stating none has no marks of its own. Constructing one from the annotation's
/// appearance characteristics is what `pdf_model::appearance` does for the requirement that *asks*
/// for a dictionary; doing it here would mean preserving a picture this program drew and calling
/// it the producer's, which `doc/questions/A48` forbids and `doc/adr/1014` bounds. So the document
/// is refused, naming the annotation, rather than converted with the operator's request silently
/// unmet.
const NO_APPEARANCE_TO_PRESERVE: &str = "this configuration answers an annotation ISO 19005 does \
     not admit by keeping its normal appearance on an appended page, and one of the annotations to \
     be removed states no normal appearance stream — so there are no marks of the producer's to \
     keep. The report names it. Authorising the loss with --authorise forbidden-annotation removes \
     it without a page, which loses nothing that was ever drawn";

/// Why an appearance whose placement §12.5.5 cannot compute is refused.
const NO_PLACEMENT: &str = "an annotation to be removed states a normal appearance and neither it \
     nor the stream states a rectangle ISO 32000-2 \u{a7}12.5.5's algorithm can map onto, so there \
     is nowhere on a page to put the marks that is the producer's choice rather than this \
     program's";

/// Why an appearance on a page this converter cannot measure is refused.
const NO_PAGE_FOR_THE_MARKS: &str = "an annotation to be removed sits on a page whose own boxes \
     this converter cannot read, and a page of preserved marks states the boxes of the page the \
     annotation was on. Inventing a paper size would put the producer's marks somewhere the \
     producer did not";

/// Each removed annotation's normal appearance, with §12.5.5's matrix and its page's geometry.
fn marks_of<'a>(
    document: &Document,
    removals: &'a ForbiddenAnnotations,
) -> Result<Vec<(&'a RemovedAnnotation, preserve::Marks)>, Because> {
    let tree = pdf_model::Pages::new(document);
    let mut out = Vec::new();
    for removed in &removals.removed {
        let appearance = removed
            .appearance
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_TO_PRESERVE))?;
        let annotation = document
            .get(removed.at)
            .as_dict()
            .cloned()
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_TO_PRESERVE))?;
        let stream = document
            .get(appearance)
            .as_stream()
            .map(|stream| stream.dict.clone())
            .ok_or(Because::NotBuiltYet(NO_APPEARANCE_TO_PRESERVE))?;
        let placement = pdf_model::appearance::placement(document, &annotation, &stream)
            .ok_or(Because::NotBuiltYet(NO_PLACEMENT))?;
        let page = tree
            .get(removed.page)
            .ok_or(Because::NotBuiltYet(NO_PAGE_FOR_THE_MARKS))?;
        if page.substituted_media_box.is_some() {
            return Err(Because::NotBuiltYet(NO_PAGE_FOR_THE_MARKS));
        }
        out.push((
            removed,
            preserve::Marks {
                appearance,
                placement: [
                    placement.a,
                    placement.b,
                    placement.c,
                    placement.d,
                    placement.e,
                    placement.f,
                ],
                media_box: page.media_box,
                crop_box: page.crop_box,
                rotate: i64::from(page.rotate),
            },
        ));
    }
    Ok(out)
}

/// Whether the source already states an output intent of its own.
///
/// Asked beside the one this conversion may be adding, because the appended page needs *an* output
/// intent to exist in the output rather than needing this conversion to have written it —
/// [`preserve::NO_OUTPUT_INTENT`] is the reading.
fn states_an_output_intent(document: &Document, catalog: Option<&Dictionary>) -> bool {
    catalog.is_some_and(|catalog| !output_intent_entries(document, catalog).is_empty())
}

/// The `parameters` field of the removal's recorded action: what went, by name.
///
/// Empty where nothing was removed, which is what keeps a document needing no removal from
/// carrying an entry saying so.
fn names_removed(removed: &[MisusedProperty]) -> String {
    if removed.is_empty() {
        return String::new();
    }
    let named: Vec<&str> = removed
        .iter()
        .take(MOST_NAMED)
        .map(|property| property.spelled.as_str())
        .collect();
    let rest = removed.len().saturating_sub(named.len());
    let tail = if rest == 0 {
        String::new()
    } else {
        format!(", and {rest} more")
    };
    format!(
        "these properties were removed because the predefined schema each names does not define \
         the value it held, which ISO 19005-2 clause 6.6.2.3.1 requires: {}{tail}. The conversion \
         report names each one, its namespace and the value that was there",
        named.join(", ")
    )
}

/// Why appended pages a packet could not record are not appended.
const NO_PLACE_TO_RECORD_A_PRESERVATION: &str = "preserving this content on a page appended to \
     the document is allowed on condition that the page is recorded in the file's own \
     xmpMM:History (doc/adr/1014 section 5), and this document's XMP packet will not take that \
     entry, so the page is not appended either";

/// A construction whose provenance entry could not be written, withdrawn rather than left
/// unrecorded.
///
/// **`doc/adr/0927`'s condition, applied in one place because all three of them carry it.** The
/// owner's permissions to write something a producer did not are each conditional on the writing
/// being visible — for the `/DefaultCMYK` (`doc/questions/A48`) and for a substituted face
/// (`doc/pdf-a-conversion-limits.md` section 4.9, on ISO 19005-2 section 6.6.6's NOTE 1, which
/// names font substitution outright) that means an `xmpMM:History` entry in the file itself, and
/// for a removed property (section 4.2) it is sharper still: a property that is gone leaves
/// nothing in the output to notice.
///
/// `recorded` is the test on the *event* rather than on the packet — a packet edited for the
/// identification schema alone carries no action — so a construction whose entry the packet
/// would not take, or for which no clock answered, is not made at all.
fn unless_recorded<T>(
    built: Result<T, Because>,
    recorded: bool,
    because: &'static str,
) -> Result<T, Because> {
    match built {
        Ok(_) if !recorded => Err(Because::NotBuiltYet(because)),
        built => built,
    }
}

/// What the two font rewrites need, worked out from one content walk.
struct PreparedFonts {
    /// The faces to embed where the file embedded none, or why none can be.
    substitutes: Result<Substitutes, Because>,
    /// The programs whose stated advances are to be restated, or why none can be.
    metrics: Result<Metrics, Because>,
    /// The `xmpMM:History` parameters the substitution has to record, empty where there is none.
    recorded: String,
}

/// Works out both font rewrites, from one walk of the document's content.
///
/// **The content walk is made once and only where a font rewrite asked for it.** Both
/// preparations need the codes the content streams showed — which glyph a width belongs to is a
/// question about what was drawn — and `pdf_archive::check` has already made this walk for its
/// own rules without keeping it. Walking a second time is the cost of that seam; a document
/// whose fonts all conform pays none of it.
fn prepare_fonts(
    plan: &ArchivePlan,
    document: &Document,
    spare: &mut Spare,
    wants_substitutes: bool,
    wants_metrics: Directions,
    survey: Option<&Survey>,
) -> PreparedFonts {
    let substitutes = match (wants_substitutes, survey) {
        (true, _) if !plan.substitute_fonts => Err(Because::Declined(SUBSTITUTION_DECLINED)),
        (true, Some(survey)) => fonts::embed_faces(document, survey, spare, &plan.supplied_fonts),
        _ => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
    };
    let metrics = match (wants_metrics.widths || wants_metrics.vertical, survey) {
        (true, Some(survey)) => fonts::restate_metrics(document, survey, wants_metrics),
        _ => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
    };
    let recorded = substitutes
        .as_ref()
        .map(|built| substituted_fonts_recorded(&built.done))
        .unwrap_or_default();
    PreparedFonts {
        substitutes,
        metrics,
        recorded,
    }
}

/// The `parameters` field of the substitution's recorded action: which face went where.
///
/// Empty where nothing was substituted. Each font is named by what the file asked for, what was
/// embedded and which of section 4.9's two metric routes it took, because those three are what
/// section 4.9 asks a converter to report per font and a provenance entry a reader can act on
/// has to carry them in the file rather than only in the run's own report.
pub(super) fn substituted_fonts_recorded(fonts: &[fonts::SubstitutedFont]) -> String {
    if fonts.is_empty() {
        return String::new();
    }
    let named: Vec<String> = fonts
        .iter()
        .take(MOST_FONTS_NAMED)
        .map(|font| {
            format!(
                "{} was not embedded and {} was embedded in its place, {}, taking the {} route",
                font.requested,
                font.face,
                font.authority.describe(),
                font.route.word()
            )
        })
        .collect();
    let rest = fonts.len().saturating_sub(named.len());
    let tail = if rest == 0 {
        String::new()
    } else {
        format!(", and {rest} more")
    };
    format!(
        "fonts were substituted so that this file carries the programs ISO 19005-2 clause \
         6.2.11.4.1 requires: {}{tail}. The glyph widths the font dictionaries state were not \
         changed, so no mark moved; the shapes drawn are the substituted faces'",
        named.join("; ")
    )
}

/// What the packet has to carry besides the producer's own bytes.
///
/// One argument rather than five, because every one of them is a fact about *this conversion*
/// that only [`Prepared::of`] knows, and a function taking five booleans and two slices is one
/// nobody can call correctly twice.
#[derive(Debug, Clone, Copy)]
struct Recording<'a> {
    /// Whether the identification schema is being restated into the packet.
    schema: bool,
    /// The applied departures' `xmpMM:History` parameters, where the conversion departed.
    ///
    /// `doc/rfc/0007` section 4.7.3: every departure is recorded in the file's own history, so the
    /// archive carries the fact rather than relying on a report nobody kept. `None` for a
    /// conversion that departed from nothing.
    departure: Option<&'a str>,
    /// What a `derive` remedy made, where anything was derived (`doc/questions/A55`).
    derived: Option<&'a str>,
    /// What the operator stated that the document does not (`doc/rfc/0007` section 5b.1).
    supplied: Option<&'a str>,
    /// What an operator's tool fetched from outside the document (`doc/adr/1209`).
    resolved: Option<&'a str>,
    /// What the removed encryption asserted about the reader (`doc/adr/1187`).
    protection: Option<&'a str>,
    /// What an appended page preserved, where one was appended (`doc/adr/1014` section 5).
    preserved: Option<&'a str>,
    /// Whether the identification schema's properties are deliberately omitted (`A59`).
    ///
    /// A departed conversion that does not claim conformance still writes the packet, so the
    /// catalog states a metadata stream, but with **no** `pdfaid:*` properties — so the file does
    /// not claim to be PDF/A. `schema` is then true (the packet is edited) and this is true (what
    /// is written is nothing), which strips any identification the source stated.
    omit_identification: bool,
    /// The instant every recorded action carries, where a clock answered.
    when: Option<&'a str>,
    /// Whether the `/DefaultCMYK` is being written, which is one recorded action.
    default_cmyk: bool,
    /// The removal's own recorded action, empty where nothing was removed.
    removals: &'a str,
    /// The substitution's own recorded action, empty where no face was embedded.
    substituted: &'a str,
    /// What the writers before this one left in each packet, one of which may be the catalog's.
    edited: Edited<'a>,
}

/// The edits a metadata packet has already taken, newest first.
///
/// **Five writers over one packet, of which the first excludes the rest**: a packet this tree
/// cannot read is replaced outright ([`Fresh`]) and the other writers skip that stream, because
/// an edit by span needs spans the producer's bytes no longer supply. For every other packet the
/// order is the whole of what this type carries: ISO
/// 19005-2 section 6.6.2.1's header attributes come off first, section 6.6.2.3.1's misused
/// properties come out of what that left, section 6.6.2.3.3's container prefixes move in what
/// *that* left, and section 6.6.4's identification schema is restated into the last of them. A
/// writer reading the producer's original instead would silently undo the writer before it.
#[derive(Debug, Clone, Copy, Default)]
struct Edited<'a> {
    /// The packets this conversion replaced outright, because it could not edit them.
    ///
    /// **Before all three of the others, and disjoint from them**: a replaced stream is skipped
    /// by the header cut, the property removal and the container respelling, so at most one of
    /// the four has anything to say about any one stream.
    fresh: Option<&'a Fresh>,
    /// The packets whose header attributes have been cut.
    headers: Option<&'a Headers>,
    /// The packets a property removal has already edited.
    cleaned: Option<&'a Cleaned>,
    /// The packets a container respelling has already edited.
    respelled: Option<&'a Respelled>,
    /// The packet a malformed amendment identifier has already been cut out of.
    amended: Option<&'a Amended>,
    /// The packets an extension schema container has already been written into.
    described: Option<&'a SchemasDescribed>,
}

impl Edited<'_> {
    /// What one stream's packet holds after every edit made before the caller's.
    fn packet(self, at: ObjectId) -> Option<Vec<u8>> {
        self.described
            .and_then(|described| described.packets.get(&at).cloned())
            .or_else(|| {
                self.amended
                    .and_then(|amended| amended.packets.get(&at).cloned())
            })
            .or_else(|| {
                self.respelled
                    .and_then(|respelled| respelled.packets.get(&at).cloned())
            })
            .or_else(|| {
                self.cleaned
                    .and_then(|cleaned| cleaned.packets.get(&at).cloned())
            })
            .or_else(|| self.headers.and_then(|headers| headers.packet(at).cloned()))
            .or_else(|| self.fresh.and_then(|fresh| fresh.packet(at).cloned()))
    }
}

/// The metadata stream this conversion writes, with every action it has to record in it.
///
/// ISO 19005-2 section 6.6.6 and ISO 19005-4 section 6.7.5 ask a converter to record what it did,
/// and `doc/pdf-a-conversion-limits.md` section 4.2 turns that into one entry per **Ask**. The
/// entries are appended in the order they are built, so a packet already holding a producer's
/// history keeps it and gains these after it.
fn the_packet(
    target: Target,
    document: &Document,
    catalog: Option<&Dictionary>,
    spare: &mut Spare,
    recording: Recording<'_>,
) -> Result<Metadata, Because> {
    let mut events: Vec<xmp::Event<'_>> = Vec::new();
    if let Some(when) = recording.when {
        if recording.default_cmyk {
            events.push(xmp::Event {
                action: DEFAULT_CMYK_ACTION,
                parameters: DEFAULT_CMYK_PARAMETERS,
                when,
            });
        }
        if !recording.removals.is_empty() {
            events.push(xmp::Event {
                action: REMOVED_PROPERTIES_ACTION,
                parameters: recording.removals,
                when,
            });
        }
        if !recording.substituted.is_empty() {
            events.push(xmp::Event {
                action: SUBSTITUTED_FONTS_ACTION,
                parameters: recording.substituted,
                when,
            });
        }
        if let Some(departure) = recording.departure {
            events.push(xmp::Event {
                action: DEPARTED_ACTION,
                parameters: departure,
                when,
            });
        }
        if let Some(derived) = recording.derived {
            events.push(xmp::Event {
                action: DERIVED_ACTION,
                parameters: derived,
                when,
            });
        }
        if let Some(supplied) = recording.supplied {
            events.push(xmp::Event {
                action: SUPPLIED_ACTION,
                parameters: supplied,
                when,
            });
        }
        if let Some(resolved) = recording.resolved {
            events.push(xmp::Event {
                action: RESOLVED_ACTION,
                parameters: resolved,
                when,
            });
        }
        if let Some(protection) = recording.protection {
            events.push(xmp::Event {
                action: DECRYPTED_ACTION,
                parameters: protection,
                when,
            });
        }
        if let Some(preserved) = recording.preserved {
            events.push(xmp::Event {
                action: PRESERVED_ACTION,
                parameters: preserved,
                when,
            });
        }
    }
    // Nothing is prepared that no failed requirement asked for: a document needing neither the
    // schema nor an entry does not have its packet read at all, and the reason is never shown —
    // `decide` consults it only for a requirement the table answers with this rewrite.
    let Some(catalog) = catalog else {
        return Err(Because::NotBuiltYet(NO_CATALOG));
    };
    if !recording.schema && events.is_empty() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    prepare_metadata(
        target,
        document,
        catalog,
        spare,
        recording.schema,
        recording.omit_identification,
        &events,
        recording.edited,
    )
}

/// Prepares the metadata stream: the packet, and the object it goes in.
///
/// Two cases, and the difference between them is the whole of what makes this safe. A document
/// with a packet has it **edited in place** — `pdf_model::xmp::restate` cuts the identification
/// schema's properties out of the producer's own bytes and puts this target's in, leaving every
/// other byte alone. A document with none gets a fresh packet stating the schema and nothing
/// else. What is never done is reading a producer's packet to a value and printing it again: this
/// tree's reader keeps neither an `rdf:about` subject nor a qualifier other than `xml:lang`, so a
/// packet round-tripped through it would come back quietly poorer.
#[expect(
    clippy::too_many_arguments,
    reason = "the packet's writers are each a fact about one conversion the caller cannot bundle \
              without a struct that would be built at exactly one call site — the same shape \
              `Recording` already is one level up"
)]
fn prepare_metadata(
    target: Target,
    document: &Document,
    catalog: &Dictionary,
    spare: &mut Spare,
    schema_wanted: bool,
    omit_identification: bool,
    recorded: &[xmp::Event<'_>],
    edited: Edited<'_>,
) -> Result<Metadata, Because> {
    // `A59`: a departed conversion that does not claim conformance writes no `pdfaid:*` at all, so
    // the packet is edited (the schema is restated, cutting the source's identification under both
    // namespace spellings) but the properties put in its place are none.
    let properties = if omit_identification {
        Vec::new()
    } else {
        identification_properties(target)
    };
    let schema = Schema {
        namespace: identification_uri(target),
        prefix: IDENTIFICATION_PREFIX,
        properties: &properties,
    };
    // A recorded action is *appended*, so a packet may need editing for that alone — a document
    // whose identification schema is already right and whose `DeviceCMYK` is not.
    let record = |packet: Vec<u8>| {
        recorded.iter().try_fold(packet, |packet, event| {
            xmp::record(&packet, event).map_err(|_| Because::NotBuiltYet(PACKET_NOT_EDITABLE))
        })
    };
    if let Some(at) = catalog.get("Metadata").and_then(Object::as_reference)
        && let Object::Stream(stream) = document.get(at)
        // The schema is restated into what the writers before it left rather than into the
        // producer's original — `Edited` is that order — and the producer's own bytes are what
        // it starts from only where none of them wrote. **The replacement is asked first and
        // without decoding**: a stream whose data will not decode is one whose packet is being
        // composed afresh, so asking the document for bytes there would refuse a document this
        // conversion can in fact write.
        && let Some(held) = edited.packet(at).or_else(|| {
            document
                .decoded_stream_data(&stream)
                .map(|bytes| bytes.to_vec())
        })
    {
        let restated = if schema_wanted {
            xmp::restate(&held, &IDENTIFICATION_URIS, &schema)
                .map_err(|_| Because::NotBuiltYet(PACKET_NOT_EDITABLE))?
        } else {
            held
        };
        return Ok(Metadata {
            at,
            written: None,
            packet: record(restated)?,
        });
    }
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    // A document with no packet at all is one whose catalog states no metadata stream, which is
    // a requirement of its own and therefore always among the failures when this line is reached.
    let packet = record(xmp::packet(&schema))?;
    Ok(Metadata {
        at,
        written: Some(metadata_stream(&Dictionary::new(), &packet)),
        packet,
    })
}

/// One metadata stream, with the two entries §14.3.2's Table 347 requires of it.
///
/// `/Type`:
///
/// > ( Required ) The type of PDF object that this dictionary describes; shall be Metadata for a
/// > metadata stream.
///
/// `/Subtype`:
///
/// > ( Required ) The type of metadata stream that this dictionary describes; shall be XML .
///
/// The packet is written uncompressed, which is what §14.3.2's own EXAMPLE shows and what leaves
/// a document's metadata legible to a reader that is not a PDF parser. Any filter the source's
/// stream stated goes with the bytes it decoded.
pub(super) fn metadata_stream(from: &Dictionary, packet: &[u8]) -> Object {
    let mut dict = from.clone();
    dict.remove("Filter");
    dict.remove("DecodeParms");
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Metadata"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"XML"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(packet.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: packet.to_vec().into(),
        decryption_failed: false,
    }))
}

/// Why a packet header this conversion could not edit refuses the document.
const HEADER_NOT_EDITABLE: &str = "an XMP packet in this file states the deprecated bytes or \
     encoding attribute in its header, and this conversion cannot cut it out: the packet is not \
     one this tree can decode, its header is written in one of the wide encodings ISO 16684-1 \
     admits — where a byte-level cut would leave the padding it does not understand — or the \
     attribute's value has no quotation marks to cut to. Both attributes describe the packet's \
     own framing, so a cut that could be made would lose nothing at all";

/// Why a header that still states an attribute afterwards refuses the document.
const HEADER_NOT_CUT: &str = "a deprecated XMP packet header attribute this conversion removed is \
     still in the header afterwards, so the packet is half-edited rather than corrected — and a \
     half-edited packet is not written at all, which is the rule the property removal beside it \
     already follows";

/// How far into a packet the header is looked for.
///
/// `pdf_archive`'s own figure, and it has to be: a header this preparation did not find in the
/// window the validator searched is a header whose attribute the output's verdict would still
/// report.
const HEADER_SCAN: usize = 4096;

/// The XMP packets whose headers lose a deprecated attribute.
///
/// ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1. **Prepared before every other edit
/// to a packet**, because the other two rewrite the RDF this header wraps and the cut has to be
/// made once, on bytes the later writers then carry: `prepare_properties` starts from what this
/// left and [`the_packet`] from what that left in turn.
#[derive(Debug, Default)]
pub(super) struct Headers {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(super) packets: BTreeMap<ObjectId, Vec<u8>>,
}

impl Headers {
    /// The edited packet for one stream, where this preparation edited it.
    pub(super) fn packet(&self, at: ObjectId) -> Option<&Vec<u8>> {
        self.packets.get(&at)
    }
}

/// Cuts the deprecated attributes out of every packet header the validator reported one in.
///
/// **Nothing is judged here.** Which streams state one is `pdf_archive`'s finding; what this
/// decides is only whether the cut can be made, and a header still holding an attribute
/// afterwards refuses the document rather than leaving it half-edited.
fn prepare_headers(
    document: &Document,
    input: &pdf_archive::Report,
    fresh: Option<&Fresh>,
) -> Result<Headers, Because> {
    let sites = sites::packet_headers(input)?;
    let mut packets = BTreeMap::new();
    for at in sites.at {
        // A stream whose packet is being replaced outright has no header of the producer's left
        // to cut: the fresh packet states none of the deprecated attributes by construction, so
        // editing this one would be editing bytes nothing will write.
        if fresh.is_some_and(|fresh| fresh.packet(at).is_some()) {
            continue;
        }
        let Object::Stream(stream) = document.get(at) else {
            return Err(Because::NotBuiltYet(HEADER_NOT_EDITABLE));
        };
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            return Err(Because::NotBuiltYet(HEADER_NOT_EDITABLE));
        };
        let cut = without_deprecated_header_attributes(&bytes)
            .ok_or(Because::NotBuiltYet(HEADER_NOT_EDITABLE))?;
        // Read back rather than trusted, which is `prepare_properties`'s rule: the writer says
        // what it cut and the packet says what it holds, and only the second is what a validator
        // sees.
        if states_a_deprecated_header_attribute(&cut) {
            return Err(Because::NotBuiltYet(HEADER_NOT_CUT));
        }
        packets.insert(at, cut);
    }
    Ok(Headers { packets })
}

/// The two attributes ISO 16684-1 deprecates and ISO 19005 forbids in a packet header.
const DEPRECATED_HEADER_ATTRIBUTES: [&[u8]; 2] = [b"bytes", b"encoding"];

/// A packet with the deprecated `bytes` and `encoding` attributes cut out of its header.
///
/// `None` where the cut cannot be made, and the three cases are the whole of what this declines:
/// a packet stating no `<?xpacket` header in the window the validator searches, a header padded
/// with the NUL bytes one of ISO 16684-1's wide encodings writes — where a byte-level cut would
/// leave half a character behind — and an attribute whose value is not delimited by quotation
/// marks, where there is nothing to cut to.
///
/// Every byte outside the attribute and the white space in front of it crosses unchanged, which
/// is what makes this lossless: both attributes describe the packet's own framing rather than
/// anything a reader of the metadata uses.
pub(super) fn without_deprecated_header_attributes(packet: &[u8]) -> Option<Vec<u8>> {
    let window = packet.get(..HEADER_SCAN.min(packet.len()))?;
    let start = find(window, b"<?xpacket")?;
    let instruction = window.get(start..)?;
    let end = start.checked_add(find(instruction, b"?>")?)?;
    let header = packet.get(start..end)?;
    if header.contains(&0) {
        return None;
    }
    let mut out = header.to_vec();
    for name in DEPRECATED_HEADER_ATTRIBUTES {
        while let Some(span) = attribute_span(&out, name) {
            if span.1 > out.len() {
                return None;
            }
            out.drain(span.0..span.1);
        }
    }
    let mut whole = packet.get(..start)?.to_vec();
    whole.extend_from_slice(&out);
    whole.extend_from_slice(packet.get(end..)?);
    Some(whole)
}

/// Whether a packet's header still states either of the deprecated attributes.
fn states_a_deprecated_header_attribute(packet: &[u8]) -> bool {
    let Some(window) = packet.get(..HEADER_SCAN.min(packet.len())) else {
        return false;
    };
    let Some(start) = find(window, b"<?xpacket") else {
        return false;
    };
    let Some(instruction) = window.get(start..) else {
        return false;
    };
    let Some(end) = find(instruction, b"?>") else {
        return false;
    };
    let Some(header) = instruction.get(..end) else {
        return false;
    };
    DEPRECATED_HEADER_ATTRIBUTES
        .into_iter()
        .any(|name| attribute_span(header, name).is_some())
}

/// The half-open range one attribute occupies in a header, white space in front of it included.
///
/// The name has to be preceded by white space and followed by an equals sign, which is
/// `pdf_archive`'s own test for the same thing: without it a value that happens to contain the
/// word would read as the attribute. The range then runs to the closing quotation mark, so the
/// value's own white space and any equals sign inside it are cut with it rather than read.
fn attribute_span(header: &[u8], name: &[u8]) -> Option<(usize, usize)> {
    let (at, _) = header
        .windows(name.len())
        .enumerate()
        .find(|(at, window)| {
            *window == name
                && *at > 0
                && header
                    .get(at.saturating_sub(1))
                    .is_some_and(u8::is_ascii_whitespace)
        })?;
    let after = at.checked_add(name.len())?;
    let equals = after.checked_add(
        header
            .get(after..)?
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())?,
    )?;
    if header.get(equals) != Some(&b'=') {
        return None;
    }
    let value = equals.checked_add(1)?;
    let quote_at = value.checked_add(
        header
            .get(value..)?
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())?,
    )?;
    let quote = *header.get(quote_at)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let opened = quote_at.checked_add(1)?;
    let close = opened.checked_add(
        header
            .get(opened..)?
            .iter()
            .position(|byte| *byte == quote)?,
    )?;
    let mut from = at;
    while from > 0
        && header
            .get(from.checked_sub(1)?)
            .is_some_and(u8::is_ascii_whitespace)
    {
        from = from.checked_sub(1)?;
    }
    Some((from, close.checked_add(1)?))
}

/// The offset of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
