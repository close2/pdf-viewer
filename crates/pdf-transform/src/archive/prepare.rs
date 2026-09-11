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

use pdf_archive::survey::DeviceFamily;
use pdf_archive::{Flavour, Level, Target};
use pdf_archive::{MisusedProperty, Outcome};
use pdf_model::icc::Identification;
use pdf_model::xmp::{self, Name as XmpName, Schema};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use crate::json::Value;

use super::decision::{Because, REMEDIES};
use super::fonts::{self, Metrics, Substitutes};
use super::rewrite::Rewrite;
use super::sites::{self, AppearanceStates, CompletedOrders, DescriptorSets, PageResources, Sites};
use super::to_unicode::{self, DerivedMaps};
use super::{ArchivePlan, COMPRESSION_LEVEL};

/// The ICC profile this program ships, and the default destination profile of an output intent
/// it adds.
///
/// `doc/questions/A18`: ship the standard sRGB profile, with a flag to override it.
/// `data/icc/PROVENANCE.md` records which of the ICC's four sRGB profiles this is and why — the
/// short answer being that ISO 19005-2 section 6.2.4.2 names the ICC editions a profile may
/// conform to and the ICC's headline v4 download conforms to none of them. It is `static` data,
/// so it costs no parse time until something asks for it.
const SRGB: &[u8] = include_bytes!("../../../../data/icc/sRGB2014.icc");

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
     unapplied redaction, a printer's mark, a trap network, 3D or rich media artwork, or an \
     annotation stating no Subtype or no readable Rect at all. ISO 32000-2 Table 166 requires the \
     dictionary and the subtype clause is what would say what goes in it, so constructing one \
     here would be putting a mark on the page the document never described";

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
const NO_CATALOG: &str = "this document has no readable catalog, so there is nowhere to state an \
     output intent or a metadata stream";

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
    /// The sRGB profile this program ships — `doc/questions/A18`'s default.
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
            Self::Shipped => "the sRGB profile this program ships",
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
    /// `doc/pdf-a-mitigations.md` section 13.3's *owed, not optional* preparations.
    ///
    /// One field rather than seven because they are one class of work — see [`Owed`] — and
    /// because a reader of this struct should be able to see which of its parts are the two
    /// constructions this verb was built around and which are the lossless rewrites it grew.
    pub(super) owed: Owed,
}

impl Prepared {
    /// Works out what can be built for this document, and only what a failed requirement asks for.
    pub(super) fn of(plan: &ArchivePlan, document: &Document, input: &pdf_archive::Report) -> Self {
        let failed: BTreeSet<&'static str> =
            input.failures().map(|judgement| judgement.id).collect();
        let wanted = |rewrite: Rewrite| wanted_by(&failed, rewrite);
        let mut spare = Spare::of(document);
        let catalog = document.catalog().ok();
        let intent = match (wanted(Rewrite::OutputIntent), catalog.as_ref()) {
            (true, Some(catalog)) => prepare_intent(plan, document, catalog, &mut spare),
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
        // ISO 19005-2 section 6.6.2.3.1's removals are worked out before the packet is restated,
        // because the catalog's packet is one of the ones they edit.
        let properties = if wanted(Rewrite::PropertyOutsideItsSchema) {
            prepare_properties(document, input)
        } else {
            Err(Because::NotBuiltYet(NOT_ASKED_FOR))
        };
        let removals = properties
            .as_ref()
            .map(|cleaned| names_removed(&cleaned.removed))
            .unwrap_or_default();
        let PreparedFonts {
            substitutes,
            metrics,
            recorded: font_history,
        } = prepare_fonts(
            plan,
            document,
            &mut spare,
            wanted(Rewrite::SubstituteFontProgram),
            wanted(Rewrite::RestateFontMetrics),
        );
        let now = xmp::instant(std::time::SystemTime::now());
        let metadata = the_packet(
            plan.target,
            document,
            catalog.as_ref(),
            &mut spare,
            Recording {
                schema: wanted(Rewrite::IdentificationSchema),
                when: now.as_deref(),
                default_cmyk: default_cmyk.is_ok(),
                removals: &removals,
                substituted: &font_history,
                cleaned: properties.as_ref().ok(),
            },
        );
        let recorded_it = now.is_some() && metadata.is_ok();
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
        // The fonts are the validator's own findings rather than a walk of this converter's:
        // the clause has four exemptions and reading them belongs to `pdf_archive`, so the set of
        // fonts that need a CMap is the set it named. A findings list is capped, and a document
        // failing at more fonts than the cap is one `doc/adr/0947`'s third stage refuses rather
        // than one this rewrite half-finishes.
        let to_unicode = if wanted(Rewrite::ToUnicode) {
            to_unicode::derive(document, &unicode_fonts(input), &mut spare)
                .map_err(Because::NotBuiltYet)
        } else {
            Err(Because::NotBuiltYet(NOT_ASKED_FOR))
        };
        let appearances = if wanted(Rewrite::AppearanceDictionary) {
            prepare_appearances(document, plan.target, &mut spare)
        } else {
            Err(Because::NotBuiltYet(NOT_ASKED_FOR))
        };
        let structure = match (wanted(Rewrite::MarkInfo), catalog.as_ref()) {
            (true, Some(catalog)) if !document.get_key(catalog, "StructTreeRoot").is_null() => {
                Ok(())
            }
            (true, Some(_)) => Err(Because::TheFence(NO_STRUCTURE_TREE)),
            (true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
            (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        Self {
            intent,
            default_cmyk,
            metadata,
            to_unicode,
            appearances,
            properties,
            substitutes,
            metrics,
            structure,
            owed: Owed::of(plan, document, input, &mut spare, &failed),
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
            Rewrite::AppearanceDictionary => self.appearances.as_ref().err().copied(),
            Rewrite::SubstituteFontProgram => self.substitutes.as_ref().err().copied(),
            Rewrite::RestateFontMetrics => self.metrics.as_ref().err().copied(),
            Rewrite::RenderingIntent => self.owed.rendering_intents.as_ref().err().copied(),
            Rewrite::BlendModeNormal => self.owed.blend_modes.as_ref().err().copied(),
            Rewrite::NormalAppearanceFromState => {
                self.owed.appearance_states.as_ref().err().copied()
            }
            Rewrite::OptionalContentOrder => self.owed.orders.as_ref().err().copied(),
            Rewrite::PageResources => self.owed.page_resources.as_ref().err().copied(),
            Rewrite::DescriptorSetRemoved => self.owed.descriptor_sets.as_ref().err().copied(),
            Rewrite::CidToGidIdentity => self.owed.cid_to_gid.as_ref().err().copied(),
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
}

impl Owed {
    /// Each of the seven, where a failed requirement asked for it.
    pub(super) fn of(
        plan: &ArchivePlan,
        document: &Document,
        input: &pdf_archive::Report,
        spare: &mut Spare,
        failed: &BTreeSet<&'static str>,
    ) -> Self {
        let wanted = |rewrite: Rewrite| wanted_by(failed, rewrite);
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
        }
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
        None => (ProfileSource::Shipped, SRGB.to_vec()),
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
/// construction may be *written down*, and it refuses three cases rather than writing something
/// weaker than the clauses state.
fn prepare_appearances(
    document: &Document,
    target: Target,
    spare: &mut Spare,
) -> Result<Appearances, Because> {
    let mut at = BTreeMap::new();
    let mut written = Vec::new();
    let mut constructed = Vec::new();
    for missing in pdf_archive::annotations_without_an_appearance(document, target) {
        let annotation = missing
            .at
            .ok_or(Because::NotBuiltYet(APPEARANCE_ON_A_DIRECT_ANNOTATION))?;
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
        if built.owed.is_some() {
            return Err(Because::TheFence(APPEARANCE_INCOMPLETE));
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
    })
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

/// Every metadata stream the schema requirement was found to fail at.
///
/// The validator's own findings rather than a walk of this converter's, for
/// [`unicode_fonts`]'s reason: which properties a predefined schema defines is `pdf_archive`'s
/// reading. The *list* it names is capped, so a document failing at more streams than the cap
/// leaves one uncleaned — and `doc/adr/0947`'s third stage refuses that file rather than this
/// preparation half-finishing it.
fn schema_packets(input: &pdf_archive::Report) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = input
        .failures()
        .filter(|judgement| judgement.id == SCHEMA_REQUIREMENT)
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
) -> Result<Cleaned, Because> {
    let mut packets = BTreeMap::new();
    let mut removed = Vec::new();
    for at in schema_packets(input) {
        let Object::Stream(stream) = document.get(at) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            return Err(Because::NotBuiltYet(NOT_A_PACKET));
        };
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
    wants_metrics: bool,
) -> PreparedFonts {
    let survey =
        (wants_substitutes || wants_metrics).then(|| pdf_archive::survey::Survey::of(document));
    let substitutes = match (wants_substitutes, survey.as_ref()) {
        (true, _) if !plan.substitute_fonts => Err(Because::Declined(SUBSTITUTION_DECLINED)),
        (true, Some(survey)) => fonts::embed_faces(document, survey, spare),
        _ => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
    };
    let metrics = match (wants_metrics, survey.as_ref()) {
        (true, Some(survey)) => fonts::restate_metrics(document, survey),
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
                "{} was not embedded and {} was embedded in its place ({})",
                font.requested,
                font.face,
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
    /// The instant every recorded action carries, where a clock answered.
    when: Option<&'a str>,
    /// Whether the `/DefaultCMYK` is being written, which is one recorded action.
    default_cmyk: bool,
    /// The removal's own recorded action, empty where nothing was removed.
    removals: &'a str,
    /// The substitution's own recorded action, empty where no face was embedded.
    substituted: &'a str,
    /// The packets a removal has already edited, one of which may be the catalog's.
    cleaned: Option<&'a Cleaned>,
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
        &events,
        recording.cleaned,
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
fn prepare_metadata(
    target: Target,
    document: &Document,
    catalog: &Dictionary,
    spare: &mut Spare,
    schema_wanted: bool,
    recorded: &[xmp::Event<'_>],
    cleaned: Option<&Cleaned>,
) -> Result<Metadata, Because> {
    let properties = identification_properties(target);
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
        && let Some(bytes) = document.decoded_stream_data(&stream)
    {
        // The removal edited this packet already where it edited any, so the schema is restated
        // into what that left rather than into the producer's original — two writers over one
        // packet, in the order the second can see the first's work.
        let held = cleaned
            .and_then(|cleaned| cleaned.packets.get(&at).cloned())
            .unwrap_or_else(|| bytes.to_vec());
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
