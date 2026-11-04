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
//!
//! **The preparations are divided by requirement family**, the families `super::decision` uses,
//! one module each under `prepare/`: `font_rewrites`, `colour_and_graphics`,
//! `files_and_structure`, `annotations_and_attachments`, `metadata`, and `appended_pages` for the
//! pages a conversion adds or edits to carry what its target would otherwise lose. This file
//! keeps what more than one family reads — [`Prepared`] and `Prepared::of`, the one entry that
//! fills it, the objects a preparation still owes and the spare object numbers they take.
use super::ArchivePlan;
use super::decision::{Because, Loss};
use super::destination::RestatedDestination;
use super::fonts::{self, Metrics, Substitutes};
use super::jpeg2000::{self, Specifications};
use super::preserve::{self, Composed};
use super::reader_resources::{self, ReaderResources};
use super::rewrite::Rewrite;
use super::signatures::{self, ForeignHandlers, Signatures, Site};
use super::sites::{
    self, AppearanceStates, ColorantEntries, CompletedOrders, DescriptorSets, PageResources,
    SharedProfile, Sites, StandardEncodings, StandardRouteNames, Type3Renames,
};
use super::to_unicode::{self, DerivedMaps};
use crate::json::Value;
use pdf_archive::Verdict;
use pdf_archive::survey::{DeviceFamily, Survey};
use pdf_archive::{MisusedProperty, Outcome};
use pdf_model::xmp;
use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};
use std::collections::{BTreeMap, BTreeSet};

mod annotations_and_attachments;
mod appended_pages;
mod colour_and_graphics;
mod files_and_structure;
mod font_rewrites;
mod metadata;

pub use annotations_and_attachments::WrittenAppearance;
pub(super) use annotations_and_attachments::{
    Appearances, ExtraAppearanceStates, FieldAppearances, HiddenAnnotations,
};
use annotations_and_attachments::{
    appends_derived_pages, attachment_outcomes, prepare_appearances,
    prepare_extra_appearance_states, prepare_field_appearances, prepare_forbidden_annotations,
    prepare_hidden_annotations, the_attached_files,
};
pub(super) use appended_pages::{MOST_ATTACHMENTS_AS_PAGES, attached_bytes};
use appended_pages::{
    fonts_may_be_stated, names_discarded, names_removed, runs_may_be_stated, stated_runs,
    the_preserved_pages,
};
pub(super) use colour_and_graphics::{
    CMYK, DefaultCmyk, Intent, ReplacedDestination, SRGB, SuppliedDestination, intent_dictionary,
    needs_a_profile, output_intent_entries, profile_stream, the_destination_object,
};
use colour_and_graphics::{
    DEVICE_CMYK_ROWS, DEVICE_RGB_ROWS, prepare_default_cmyk, prepare_intent, replaced_destination,
    states_an_output_intent, supplied_destination,
};
pub use colour_and_graphics::{DestinationProfile, ProfileSource};
use files_and_structure::structure_tree;
pub(super) use font_rewrites::substituted_fonts_recorded;
use font_rewrites::{PreparedFonts, metric_directions, prepare_fonts, unicode_fonts, wanted_by};
pub(super) use metadata::{
    AMENDMENT_REQUIREMENT, Amended, DEFAULT_CMYK_ACTION, DescribedField, DescribedSchema,
    FieldsDescribed, HEADER_ACTION, Headers, Metadata, MovedEntry, RemovedIdentifier,
    ReplacedPacket, Respelled, SUBSTITUTED_FONTS_ACTION, SchemasDescribed, field_key,
    metadata_stream, names_a_field, undescribed_fields_in,
};
use metadata::{
    Edited, Information, Recording, document_packet, prepare_amendment, prepare_field_descriptions,
    prepare_fresh_packets, prepare_headers, prepare_information, prepare_properties,
    prepare_respellings, prepare_schema_descriptions, schema_packets, the_packet,
};

/// The `parameters` field of that action.
///
/// The clause is named in words rather than with a section sign, because this string is written
/// into somebody else's file and the sign is this project's convention rather than the standard's.
pub(super) const DEFAULT_CMYK_PARAMETERS: &str = "a DeviceN DefaultCMYK was added over Cyan, \
     Magenta, \
     Yellow and Black, whose tint transform is the DeviceCMYK to DeviceRGB conversion of \
     ISO 32000-2 clause 10.4.2.5, with this file's ICC sRGB profile as the alternate space";

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

/// Why no TrueType program was copied for a file another requirement refuses by name.
const CMAP_NOT_PREPARED: &str = "another requirement this file fails is refused by name, so \
     the file is refused whatever its TrueType fonts say, and no program was copied to find out \
     whether their cmap could be restated (doc/adr/2520)";

/// Why a font's `cmap` was not restated even though the conversion could prove it.
const NO_PLACE_TO_RECORD_CMAP: &str = "restating a TrueType font's cmap writes the fonts it \
     reached into this file's own xmpMM:History naming the clause, and this document's XMP \
     packet will not take that entry \u{2014} so the fonts stay as they are and no file is \
     written (doc/adr/2520)";

/// Why a transfer function was not removed even though the removal was authorised.
const NO_PLACE_TO_RECORD_A_TRANSFER: &str = "removing a transfer function other than the \
     identity writes what it said into this file's own xmpMM:History first, and this document's \
     XMP packet will not take that entry — so the function stays and no file is written \
     (doc/adr/1964)";

/// Why `/Interpolate` was not written false even though the loss was authorised.
const NO_PLACE_TO_RECORD_SMOOTHING: &str = "turning an image's smoothing off writes the images \
     it reached into this file's own xmpMM:History, and this document's XMP packet will not take \
     that entry — so the images stay as they are and no file is written (doc/adr/2498)";

/// Why a run was not written as the number this reader draws even though the loss was authorised.
const NO_PLACE_TO_RECORD_A_RUN: &str = "writing a run as the number this reader draws names \
     each run in this file's own xmpMM:History, and this document's XMP packet will not take that \
     entry, so the runs stay as they are and no file is written (doc/adr/2638)";

/// Why the target's header was not written over a header that stated no version.
const NO_PLACE_TO_RECORD_HEADER: &str = "this document's header states no version this tree \
     reads, so the output's header states the target's and the source's header line is written \
     into this file's own xmpMM:History, and this document's XMP packet will not take that entry \
     — so no file is written (doc/adr/2558)";

/// Why a substitution was withdrawn even though it could have been carried out.
const NO_PLACE_TO_RECORD_A_SUBSTITUTION: &str = "embedding a face for a font this file never \
     carried is allowed on condition that it is recorded in the file's own xmpMM:History, which \
     is where ISO 19005-2 section 6.6.6's NOTE 1 puts font substitution by name — and this \
     document's XMP packet will not take that entry, so no face is embedded either";

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

/// Why a `/NeedAppearances` a conversion could not make true is left standing.
const WIDGET_APPEARANCE_NOT_CONSTRUCTED: &str = "this document asks a processor to build the \
     appearances of its form fields, and this conversion could not build one for every visible \
     widget the file provides none for — the report names each field it could not, under \
     `unconstructed`. Clearing the flag would leave the file stating that appearances had been \
     provided for widgets that have none, so the flag stays and the requirement with it";

/// Why a dynamic XFA form's `/XFA` is not removed unless the operator says so.
const DYNAMIC_XFA_DECLINED: &str = "this document states NeedsRendering true, which ISO 32000-2 \
     Table 29 makes the claim that its pages are regenerated when it is first opened — so the \
     pages in the file are not what a reader of this form shows, and the AcroForm this conversion \
     would keep is not the form. Answer forms/no-xfa-key with dynamic = \"discard\" to remove the \
     key anyway, accepting a document whose form nothing will build the pages of";

/// The placeholder for a construction no failed requirement asked for.
///
/// Never reported: `decide` reads a preparation's reason only for a requirement the table answers
/// with the rewrite that preparation builds, and such a requirement is exactly what makes the
/// preparation happen. It says so rather than borrowing another reason's sentence.
const NOT_ASKED_FOR: &str = "no requirement this document failed asked for this construction, so \
     none was prepared";

/// Why neither construction could be prepared for a document with no readable catalog.
///
/// Unreachable through [`super::run`], which refuses such a document before stage 1; it exists so that
/// the reason a requirement is refused with is never a reason that is not the actual one.
pub(super) const NO_CATALOG: &str = "this document has no readable catalog, so there is nowhere \
     to state an output intent or a metadata stream";

/// Why an object could not be added to the output.
const NO_SPARE_OBJECT: &str = "this document uses every object number a conversion could give to \
     the stream it has to add";

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

/// What this slice's two constructions need, worked out before any decision is taken.
///
/// Both depend on the *document* rather than on the requirement alone — whether a profile can be
/// shared with one the file already holds, whether the packet a producer wrote can be edited in
/// place — and **a decision that cannot be carried out is a refusal rather than a plan**, which
/// is the rule [`super::version_for`] already answers to. So both are settled here, in stage 2, and the
/// reason each failed is the reason the requirements it would have answered are refused with.
///
/// Nothing is prepared that no failed requirement asked for: a document that already conforms
/// does not have its packet read, which is what keeps `doc/adr/0947`'s first rule true of this
/// slice as well.
pub(super) struct Prepared {
    /// The requirements this preparation found failing in the file it will write although the
    /// source met them, each with the places: `fonts/font-programs-embedded` where a name was
    /// defined as one of §9.6.2.2's fourteen (`doc/adr/2522`).
    pub(super) raised: Vec<(&'static str, usize)>,
    /// The destination profile written over one this reader does not draw through, or why none
    /// is (`doc/adr/2007`).
    ///
    /// Prepared **before** the intent, which then names the profile written rather than the one
    /// it replaces.
    pub(super) replaced_destination: Result<ReplacedDestination, Because>,
    /// The destination profile whose header is restated as what its own bytes are, or why it is
    /// not (`doc/adr/2021`).
    pub(super) restated_destination: Result<RestatedDestination, Because>,
    /// The output intent to add, or why one cannot be.
    pub(super) intent: Result<Intent, Because>,
    /// The PDF/A output intents stating no destination profile, given the intent's, or why they
    /// are not (`doc/adr/2035`).
    pub(super) supplied_destination: Result<SuppliedDestination, Because>,
    /// The `/DefaultCMYK` to write, or why one cannot be.
    ///
    /// Prepared **after** the intent and **before** the metadata, because the three depend on
    /// each other in that order: the default's alternate space is the profile the intent settled,
    /// and the packet has to carry the `xmpMM:History` entry `doc/questions/A48` makes the
    /// default conditional on.
    pub(super) default_cmyk: Result<DefaultCmyk, Because>,
    /// The `/DefaultRGB` to write, or why one cannot be (`doc/adr/2114`).
    ///
    /// Prepared after the survey, because whether the document draws in `DeviceRGB` at all is
    /// the survey's to say: six of the rows it answers state no family of their own.
    pub(super) default_rgb: Result<super::default_rgb::DefaultRgb, Because>,
    /// The `/DefaultCMYK` over the operator's press profile, or why one cannot be
    /// (`doc/adr/2130`).
    ///
    /// Prepared before the `DeviceN` default, which is then not prepared at all: an operator who
    /// named the press has said what the four numbers mean, and the file records one statement
    /// about them rather than two.
    pub(super) supplied_cmyk: Result<super::supplied_cmyk::SuppliedCmyk, Because>,
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
    /// The packets whose value types gain a description of every structure field they state, or
    /// why not.
    ///
    /// Prepared **last of the six writers over a packet**, for the container's reason: ISO
    /// 19005-2 section 6.6.2.3.1 is held against the value type the packet states once the five
    /// editors before it have said their piece (`doc/adr/2050`).
    pub(super) fields_described: Result<FieldsDescribed, Because>,
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
    /// Whether the file's history can say that images were set to `/Interpolate false`
    /// (`doc/adr/2498`).
    pub(super) smoothing_recorded: Result<(), Because>,
    /// Whether the file's history can name each run written as the number this reader draws
    /// (`doc/adr/2638`).
    pub(super) as_drawn_recorded: Result<(), Because>,
    /// What the file's history says the source's header was, where the source's header stated no
    /// version and the output's states the target's (`doc/adr/2558`): `Ok(None)` where nothing is
    /// recorded, an error where the packet will not take the entry.
    pub(super) header_recorded: Result<Option<String>, Because>,
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
    /// The content streams written with the number this reader draws in place of each run it
    /// salvages, or why none are (`doc/adr/2638`).
    ///
    /// ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2, where the operator the base
    /// standard does not define is a run ISO 32000-2 §7.3.3 spells as no number.
    pub(super) as_drawn: Result<super::as_drawn::Drawn, Because>,
    /// The `/Lang` entries that are not language identifiers, and what each object becomes.
    ///
    /// ISO 19005-2 section 6.7.4, at the catalog, the structure elements and the property lists
    /// the survey's content streams reach. `Err` only where neither site was asked for; a half
    /// that cannot be carried out says so in its own obstacle (`doc/adr/2066`).
    pub(super) languages: Result<super::languages::Restated, Because>,
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
    /// The reference `XObject`s whose `Ref` entry a `preserve` removes, or why none can be.
    ///
    /// `Err(NOT_ASKED_FOR)` unless the requirement failed **and** a configuration answered it with
    /// `preserve`: the decision table refuses the site, so nothing but the operator's answer asks
    /// for this (`doc/adr/1285`).
    pub(super) reference_xobjects: Result<super::in_place::ReferenceXObjects, Because>,
    /// The JPEG 2000 images a `preserve` transcodes to `FlateDecode`, or why they cannot be.
    ///
    /// `Err(NOT_ASKED_FOR)` unless the baseline requirement failed and a configuration answered it
    /// with `preserve` (`doc/adr/1400`).
    pub(super) transcodes: Result<super::transcode::Transcodes, Because>,
    /// The `CMap` streams a `preserve` embeds from the published set, or why none can be.
    ///
    /// `Err(NOT_ASKED_FOR)` unless the requirement failed and a configuration answered it with
    /// `source = "shipped-cmaps"` (`doc/adr/1286`).
    pub(super) shipped_cmaps: Result<super::cmaps::ShippedCMaps, Because>,
    /// The appearance dictionaries that lose every key but `/N`, or why none do.
    pub(super) extra_appearance_states: Result<ExtraAppearanceStates, Because>,
    /// The optional content configurations that lose their `/AS`, or why none do.
    pub(super) automatic_states: Result<sites::AutomaticStates, Because>,
    /// Where the interactive form dictionary is, for the two section 6.4 rewrites that edit it.
    pub(super) form: Result<Site, Because>,
    /// The same site, refused where the document says its own pages are generated.
    ///
    /// A second field rather than a second read: the `/XFA` removal is the one of the two whose
    /// answer depends on what kind of form this is, and `doc/adr/1257` is the predicate.
    pub(super) xfa: Result<Site, Because>,
    /// The widget appearances Table 224's condition asks for, or why they cannot be built.
    pub(super) field_appearances: Result<FieldAppearances, Because>,
    /// The embedded files a target's own rule does not admit, or why none can be removed.
    pub(super) embedded_files: Result<sites::EmbeddedFiles, Because>,
    /// Whether the operator asked for those appearances at all (`construct`).
    ///
    /// Carried beside the construction rather than read from the plan again, because it is what
    /// `super::decision::answer_of` routes on: with the appearances built the flag's removal
    /// loses nothing, and without them it loses a field its producer gave no stream.
    pub(super) constructs_field_appearances: bool,
    /// The pages a `preserve` remedy appends, or why none can be.
    ///
    /// `doc/adr/1014`, and the field is `Err(NOT_ASKED_FOR)` for every conversion whose caller
    /// named no `preserve` remedy — which is every conversion until an operator's configuration
    /// says otherwise, so nothing is composed, no font is loaded and no page is measured for a
    /// document that asked for none.
    pub(super) preserved: Result<Composed, Because>,
    /// What became of each embedded file the embedded-file requirements failed at, one entry per
    /// file (`doc/adr/2536`); empty where none failed or the files are derived rather than removed.
    pub(super) attachments: Vec<super::report::AttachmentOutcome>,
    /// The producer's own bytes kept as embedded files, or why none can be.
    ///
    /// `doc/adr/1270`, and the field is `Err(NOT_ASKED_FOR)` for every conversion whose caller
    /// named no attaching `preserve` — which is every conversion until an operator's
    /// configuration says otherwise, so nothing is read and no object is numbered for a document
    /// that asked for none.
    pub(super) attached: Result<super::attach::Attached, Because>,
    /// What §14.3.3's document information dictionary becomes, or why it cannot move.
    ///
    /// Prepared **after the five writers over a packet and before the packet itself**: what the
    /// packet already states is what decides whether an entry may be written at all (§14.3.4),
    /// and the properties this settles are supplemented into the packet the writer below composes.
    pub(super) information: Result<Information, Because>,
    /// Whether this document's packet took the `xmpMM:History` entries this conversion has to
    /// record.
    ///
    /// **The condition on both configured remedies.** `doc/questions/A55` makes an `xmpMM:History`
    /// record part of the permission to derive at all, and `doc/rfc/0007` section 5b.1 makes the
    /// same demand of a supplied fact — so a document whose packet will not take the entry does not
    /// get the remedy either, exactly as `doc/questions/A48`'s constructions do not. The permission
    /// and its condition are one thing.
    pub(super) recorded_provenance: bool,
    /// Whether pages appended from a derived file were licensed by that file's own output intent,
    /// which does not come across, so that the archive's intent licenses them (`doc/adr/2539`) —
    /// or a statement page lists pages not held, which no failed row's intent licenses
    /// (`doc/adr/2596`).
    pub(super) derived_pages_need_an_intent: bool,
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
    /// The structure this viewer recovered and the file now states (`doc/adr/2502`).
    pub(super) recovered: Option<&'a str>,
    /// The rebuilt structure and the pages the source states and the rebuilt file does not hold,
    /// each where the profile asked for its loss to be stated on the appended page
    /// (`doc/adr/2669`, `doc/adr/2596`).
    pub(super) stated: super::recovered::Stated<'a>,
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
        source: &Document,
        input: &pdf_archive::Report,
        omit_identification: bool,
        provenance: Provenance<'_>,
        remedies: &super::remedies::Remedies,
    ) -> Self {
        let mut failed: BTreeSet<&'static str> =
            input.failures().map(|judgement| judgement.id).collect();
        // An unreadable profile's alternate route writes device colour the input did not use, and
        // section 6.2.4.3 of either part licenses it through an output intent: so the intent is
        // prepared as the input's own device rows would have asked for it (`doc/adr/1990`).
        let reading = super::icc::Reading::of(plan.target);
        if failed.contains(reading.site())
            && let Ok(alternates) = super::icc::Alternates::of(document, reading)
        {
            for family in alternates.families {
                failed.insert(match family {
                    DeviceFamily::Gray => DEVICE_GRAY_ROW,
                    DeviceFamily::Rgb => DEVICE_RGB_ROWS[0],
                    DeviceFamily::Cmyk => DEVICE_CMYK_ROWS[0],
                });
            }
        }
        // The same for a derived file's appended pages: their device colour was licensed by the
        // derived file's own output intent, which does not come across, so the archive's intent
        // is prepared as the colour of that intent's family would ask for it (`doc/adr/2539`).
        // A drawn attachment's pages the same way (`doc/adr/2536`): a converted PDF's colour was
        // licensed by its own intent, and an image's samples are in the family its file states.
        let mut derived_pages_need_an_intent = false;
        let drawn = remedies
            .drawn
            .values()
            .filter_map(|drawn| drawn.as_ref().ok())
            .map(|file| (file.site, &file.families));
        for (site, families) in remedies
            .appended
            .values()
            .map(|file| (file.site, &file.families))
            .chain(drawn)
        {
            if !failed.contains(site) {
                continue;
            }
            derived_pages_need_an_intent |= !families.is_empty();
            for family in families {
                failed.insert(match family {
                    DeviceFamily::Gray => DEVICE_GRAY_ROW,
                    DeviceFamily::Rgb => DEVICE_RGB_ROWS[0],
                    DeviceFamily::Cmyk => DEVICE_CMYK_ROWS[0],
                });
            }
        }
        // And for a statement page listing a rebuilt structure or pages not held, which no failed
        // row composes: its text paints in Table 51's initial `DeviceGray`, so the intent is
        // prepared as a gray row would have asked for it (`doc/adr/2596`, `doc/adr/2669`).
        if !provenance.stated.is_empty() {
            derived_pages_need_an_intent = true;
            failed.insert(DEVICE_GRAY_ROW);
        }
        // And for a statement page the font row may compose, the same way: whether it lists
        // anything is known only once the fonts are prepared, below the intent, so the intent is
        // prepared wherever that preparation can photograph a page or embed a face — and written
        // only where a page is appended (`doc/adr/2600`).
        if fonts_may_be_stated(plan, &failed) {
            derived_pages_need_an_intent = true;
            failed.insert(DEVICE_GRAY_ROW);
        }
        // And for one the operator row may compose over the runs it writes (`doc/adr/2638`).
        if runs_may_be_stated(plan, &failed) {
            derived_pages_need_an_intent = true;
            failed.insert(DEVICE_GRAY_ROW);
        }
        let failed = failed;
        let wanted = |rewrite: Rewrite| wanted_by(&failed, rewrite);
        let mut spare = Spare::of(document);
        let catalog = document.catalog().ok();
        let replaced_destination = match (
            wanted(Rewrite::DestinationProfileReplaced),
            catalog.as_ref(),
        ) {
            (true, Some(catalog)) => replaced_destination(plan, document, catalog),
            (true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
            (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        let restated_destination = match (
            wanted(Rewrite::DestinationProfileRestated),
            catalog.as_ref(),
        ) {
            (true, Some(catalog)) => super::destination::restated(document, catalog),
            (true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
            (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        // A PDF/A entry stating no profile is given the profile this conversion's own entry would
        // name, so the intent is prepared for it as for the entry (`doc/adr/2035`).
        let intent = match (
            &replaced_destination,
            wanted(Rewrite::OutputIntent) || wanted(Rewrite::DestinationProfileSupplied),
            catalog.as_ref(),
        ) {
            // The intent names the profile every entry already names, and that object is the one
            // this conversion writes: so it is what the report names and what any entry added
            // beside them must name too.
            (Ok(replaced), _, _) => Ok(replaced.intent()),
            (Err(_), true, Some(catalog)) => {
                prepare_intent(plan, document, catalog, &failed, &mut spare)
            }
            (Err(_), true, None) => Err(Because::NotBuiltYet(NO_CATALOG)),
            // Nothing failed that an output intent answers, so nothing is prepared and the
            // reason is never read: `decide` consults this only for a requirement `wanted`
            // has already found in the table.
            (Err(_), false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        let supplied_destination = match (
            wanted(Rewrite::DestinationProfileSupplied),
            catalog.as_ref(),
            intent.as_ref(),
        ) {
            (true, Some(catalog), Ok(intent)) => supplied_destination(document, catalog, intent),
            (true, None, _) => Err(Because::NotBuiltYet(NO_CATALOG)),
            (true, Some(_), Err(because)) => Err(*because),
            (false, _, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        // The `DeviceCMYK` sentence offers two licences and the output intent is the better one,
        // so the default is prepared only where the profile in hand cannot answer the clause —
        // which is `Answer::CmykUnderPartTwo`'s choice, taken here so that the packet knows
        // whether it has an action to record.
        let by_intent = intent
            .as_ref()
            .is_ok_and(|intent| intent.family == DeviceFamily::Cmyk);
        // The `/DefaultCMYK` over the operator's press profile asks the survey only where a
        // profile was supplied beside a file whose own intent is of another family, so a
        // conversion nobody supplied a press for walks nothing more than it did (`doc/adr/2130`).
        // The shipped press stands in for a supply only where the caller authorised the loss
        // `press-assumed` and the target is part 4, whose second licence states no press at all
        // (`doc/adr/2518`).
        let assumed_press = (plan.target.part() == pdf_archive::Part::Four
            && plan.authorised.grants(Loss::PressAssumed))
        .then_some(CMYK);
        let cmyk_by_supply = wanted(Rewrite::SuppliedDefaultCmyk)
            && !by_intent
            && (plan.profile.is_some() || assumed_press.is_some())
            && intent.is_ok();
        // **One survey, for every preparation that asks what the pages actually drew** — the two
        // that read a font's advances and the two that prove a change of encoding moves no mark —
        // because a second walk of every content stream would double a large document's cost.
        // The repair `super::hexadecimal` makes needs the same walk the font rewrites do, so the
        // one survey answers both: a document that fails neither is never walked.
        let odd_digits = super::decision::odd_hexadecimal_digits_on_a_page(input);
        // A non-RGB intent, or a proved part-4 page override, can need `/DefaultRGB`.
        // Ordinary sRGB output with no override still requests no extra content survey
        // (`doc/adr/2114`, `doc/adr/2230`).
        let rgb_page_contexts = match (wanted(Rewrite::DefaultRgb), intent.as_ref()) {
            (true, Ok(intent)) => super::default_rgb::page_contexts(document, plan.target, intent),
            _ => Ok(None),
        };
        let rgb_by_default = wanted(Rewrite::DefaultRgb)
            && intent
                .as_ref()
                .is_ok_and(|intent| intent.family != DeviceFamily::Rgb)
            || matches!(&rgb_page_contexts, Ok(Some(_)));
        // The pages that hold a fault of their content stream are found in the survey, so a
        // profile that photographs them asks for it as a font rewrite does (A694 item 1).
        let photographs_fenced = plan.authorised.grants(Loss::PageAsImage)
            && fonts::Gate::admits(plan.target)
            && input
                .failures()
                .any(|judgement| super::fenced::ROWS.contains(&judgement.id));
        // A run the reader draws as a number is found in the content streams the survey reaches,
        // where the profile authorised writing it so (`doc/adr/2638`).
        let runs_drawn = plan.authorised.grants(Loss::ContentAsDrawn)
            && input
                .failures()
                .any(|judgement| judgement.id == super::fenced::OPERATORS);
        let survey = (SURVEYED.iter().any(|rewrite| wanted(*rewrite))
            || odd_digits
            || runs_drawn
            || rgb_by_default
            || cmyk_by_supply
            || photographs_fenced)
            .then(|| Survey::of(document));
        // **The definitions are worked out before the fonts**, because a name this reader draws
        // from its own faces is defined as a font the embedding route then has to embed
        // (`doc/adr/2522`). `fonts/font-programs-embedded` did not fail in the source, which
        // defined no such font; it fails in the file this conversion writes, so it is raised here
        // as a failed requirement of the source would have been, and the output is held to it
        // afterwards like any other (the same rule as the device rows above, `doc/adr/1990`).
        let form_resources = asked(wanted(Rewrite::PageResources), || {
            let survey = survey.as_ref().ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
            reader_resources::assign(document, survey, &mut spare)
        });
        let defined_resources = asked(wanted(Rewrite::EmptyResourceDefined), || {
            let survey = survey.as_ref().ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
            reader_resources::supply(document, survey, &mut spare, form_resources.as_ref().ok())
        });
        let standard_fonts: BTreeMap<Vec<u8>, ObjectId> = defined_resources
            .as_ref()
            .ok()
            .zip(survey.as_ref())
            .map(|(defined, survey)| defined.rendered_standard_fonts(survey))
            .unwrap_or_default();
        let raised: Vec<(&'static str, usize)> = if standard_fonts.is_empty()
            || failed.contains(fonts::EMBEDDED_PROGRAMS)
            || !input.judgements.iter().any(|judgement| {
                judgement.id == fonts::EMBEDDED_PROGRAMS && judgement.outcome == Outcome::Met
            }) {
            Vec::new()
        } else {
            vec![(fonts::EMBEDDED_PROGRAMS, standard_fonts.len())]
        };
        let failed = if raised.is_empty() {
            failed
        } else {
            let mut raising = failed;
            raising.extend(raised.iter().map(|(id, _)| *id));
            raising
        };
        let wanted = |rewrite: Rewrite| wanted_by(&failed, rewrite);
        let hexadecimal = asked(odd_digits, || {
            super::hexadecimal::complete(document, survey.as_ref())
        });
        let supplied_cmyk = match (wanted(Rewrite::SuppliedDefaultCmyk), intent.as_ref()) {
            (true, Ok(intent)) => super::supplied_cmyk::prepare(
                document,
                intent,
                plan.profile.as_deref(),
                assumed_press,
                survey.as_ref(),
                &mut spare,
            ),
            (true, Err(because)) => Err(*because),
            (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        let default_cmyk = match (
            wanted(Rewrite::DefaultCmyk) && !by_intent && supplied_cmyk.is_err(),
            intent.as_ref(),
        ) {
            (true, Ok(intent)) => prepare_default_cmyk(document, intent, &mut spare),
            (true, Err(because)) => Err(*because),
            (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        let default_rgb = match (wanted(Rewrite::DefaultRgb), intent.as_ref()) {
            (true, Ok(intent)) => super::default_rgb::prepare(
                document,
                intent,
                survey.as_ref(),
                &rgb_page_contexts,
                &mut spare,
            ),
            (true, Err(because)) => Err(*because),
            (false, _) => Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
        };
        // After the repair, because a content stream both reach is respelled from its bytes: the
        // value written is the operator's where a `supply` answered the site, and §14.9.2.2's
        // empty identifier otherwise (`doc/adr/2066`).
        let languages = asked(
            wanted(Rewrite::CatalogLanguageRestated) || wanted(Rewrite::MarkedLanguagesRestated),
            || {
                let tag = |site: &str| remedies.languages.get(site).map_or("", String::as_str);
                Ok(super::languages::restate(
                    document,
                    survey.as_ref(),
                    hexadecimal.as_ref().ok(),
                    super::languages::Values {
                        catalog: tag(super::languages::CATALOG_SITE),
                        marked: tag(super::languages::MARKED_SITE),
                    },
                ))
            },
        );
        // After both other stream repairs, because a stream either writes is refused rather than
        // written twice (`super::as_drawn`).
        let as_drawn = asked(runs_drawn, || {
            let drawn =
                super::as_drawn::write(document, survey.as_ref(), hexadecimal.as_ref().ok())?;
            super::as_drawn::apart_from(drawn, languages.as_ref().ok())
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
        let removals = properties
            .as_ref()
            .map(|cleaned| names_removed(&cleaned.removed))
            .unwrap_or_default();
        // A discarded container is its own recorded action, because its reason is section
        // 6.6.2.3.3's and not the misused property's.
        let discards = respelled
            .as_ref()
            .map(|respelled| names_discarded(&respelled.removed))
            .unwrap_or_default();
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
                    fields: None,
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
                &schema_packets(document, input, SCHEMAS_REQUIREMENT),
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
                    fields: None,
                },
            )
        });
        // Section 6.6.2.3.1's undescribed structure fields are described last of the six editors,
        // in what the five before it left: the value type a field is added to is the one the
        // packet states once every other edit is in it (`doc/adr/2050`).
        let fields_described = asked(wanted(Rewrite::ExtensionFieldDescribed), || {
            prepare_field_descriptions(
                document,
                catalog.as_ref(),
                &schema_packets(document, input, FIELDS_SITE),
                Edited {
                    fresh: fresh.as_ref().ok(),
                    headers: headers.as_ref().ok(),
                    cleaned: properties.as_ref().ok(),
                    respelled: respelled.as_ref().ok(),
                    amended: amended.as_ref().ok(),
                    described: described.as_ref().ok(),
                    fields: None,
                },
                &remedies.field_descriptions,
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
        let hidden_annotations = asked(
            wanted(Rewrite::HiddenAnnotationRemoved)
                || (plan.annotation_flags.is_some() && wanted(Rewrite::AnnotationFlags)),
            || prepare_hidden_annotations(document, plan.target, plan.annotation_flags),
        );
        // `doc/adr/1286`: the table refuses the site, so only an operator's `preserve` asks for
        // this, and the streams take object numbers from the same spare range as every addition.
        let shipped_cmaps = asked(
            failed.contains(super::cmaps::SITE)
                && plan
                    .preservations
                    .iter()
                    .any(|preservation| preservation.site == super::cmaps::SITE),
            || super::cmaps::prepare(document, plan.target, &mut spare),
        );
        let reference_xobjects = asked(
            failed.contains(super::in_place::NO_REFERENCE_XOBJECTS)
                && plan.preservations.iter().any(|preservation| {
                    preservation.site == super::in_place::NO_REFERENCE_XOBJECTS
                }),
            || super::in_place::prepare_reference_xobjects(document, plan.target),
        );
        // ADR 1412: the four JPEG 2000 sites one transcode answers, each where it failed and
        // the plan preserves it.
        let transcode_sites: Vec<&str> = super::transcode::SITES
            .into_iter()
            .filter(|site| {
                failed.contains(site)
                    && plan
                        .preservations
                        .iter()
                        .any(|preservation| preservation.site == *site)
            })
            .collect();
        let transcodes = asked(!transcode_sites.is_empty(), || {
            super::transcode::prepare(document, input, &mut spare, &transcode_sites)
        });
        let extra_appearance_states = asked(wanted(Rewrite::ExtraAppearanceStatesRemoved), || {
            prepare_extra_appearance_states(document, plan.target)
        });
        let automatic_states = asked(wanted(Rewrite::AutomaticStatesRemoved), || {
            sites::automatic_states(document)
        });
        // ISO 19005-2 section 6.4.1 and 6.4.2, and ISO 19005-4's same two: both edit the one
        // dictionary §12.7.3 puts `/NeedAppearances` and `/XFA` in, so the site is read once.
        let form = asked(wanted(Rewrite::NeedAppearancesCleared), || {
            sites::interactive_form(document)
        });
        let xfa = asked(wanted(Rewrite::XfaRemoved), || {
            // `doc/adr/1257`: a form whose own document says its pages are regenerated is the
            // dynamic case, where the AcroForm is not the form and removing the key leaves a
            // document nothing will build the pages of. `dynamic` is the operator's word for it.
            if sites::xfa_form_is_dynamic(document) && !plan.forms.remove_dynamic_xfa {
                return Err(Because::Declined(DYNAMIC_XFA_DECLINED));
            }
            sites::interactive_form(document)
        });
        // **Read before the pages are composed and after the annotations' removal**: an
        // attachment the plan appends as pages is one of these files, and a file attachment
        // annotation section 6.3.1 takes off the page no longer reaches its file (`doc/adr/2536`).
        let as_pages = ATTACHMENT_REQUIREMENTS.into_iter().any(|site| {
            failed.contains(site)
                && (plan
                    .preservations
                    .iter()
                    .any(|preservation| preservation.site == site && preservation.by_page)
                    || appends_derived_pages(remedies, site))
        });
        let embedded_files = asked(wanted(Rewrite::EmbeddedFileRemoved), || {
            sites::embedded_files(
                document,
                input,
                forbidden_annotations
                    .as_ref()
                    .ok()
                    .map(|removals| &removals.at),
                as_pages,
            )
        });
        // Before the preserved pages, because the statement page lists the pages the fonts'
        // preparation photographed and the faces it embedded with a loss (`doc/adr/2600`).
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
            (survey.as_ref(), input, as_drawn.is_ok()),
            &standard_fonts,
        );
        let preserved = the_preserved_pages(
            plan,
            document,
            input,
            &failed,
            properties.as_ref(),
            fresh.as_ref(),
            forbidden_annotations.as_ref(),
            embedded_files.as_ref(),
            intent.is_ok() || states_an_output_intent(document, catalog.as_ref()),
            remedies,
            provenance.stated,
            substitutes.as_ref().ok(),
            stated_runs(plan, &failed, as_drawn.as_ref().ok()),
            &mut spare,
        );
        let attached = the_attached_files(
            plan,
            document,
            catalog.as_ref(),
            &failed,
            fresh.as_ref(),
            xfa.as_ref().ok().copied(),
            forbidden_annotations.as_ref().ok(),
            embedded_files.as_ref().ok(),
            &mut spare,
        );
        // The statement's own entry beside what the pages preserved: it carries no content of
        // the document's, so it is not one of the rows `preserved_history` opens with
        // `PRESERVED_AS_A_PAGE` (`doc/rfc/0014` section 4).
        let preserved_history = preserved.as_ref().ok().and_then(|composed| {
            let kept = preserve::preserved_history(&composed.carried);
            let stated = composed
                .statement
                .as_ref()
                .map(super::report::StatementPage::sentence);
            match (kept, stated) {
                (Some(kept), Some(stated)) => Some(format!("{kept}; {stated}")),
                (kept, stated) => kept.or(stated),
            }
        });
        let attachments = attachment_outcomes(
            plan,
            &failed,
            embedded_files.as_ref().ok(),
            preserved.as_ref().ok(),
        );
        let attachments_history = super::report::attachments_history(&attachments);
        let attached_history = attached
            .as_ref()
            .ok()
            .and_then(|files| super::attach::attached_history(&files.rows));
        let edited = Edited {
            fresh: fresh.as_ref().ok(),
            headers: headers.as_ref().ok(),
            cleaned: properties.as_ref().ok(),
            respelled: respelled.as_ref().ok(),
            amended: amended.as_ref().ok(),
            described: described.as_ref().ok(),
            fields: fields_described.as_ref().ok(),
        };
        // ISO 19005-4 section 6.1.3's two rows, read against the packet as the five writers
        // above left it: §14.3.4 permits an addition only into a silence, so what the packet
        // already says has to be the packet this conversion is actually going to write.
        let information = asked(wanted(Rewrite::InformationMovedIntoThePacket), || {
            prepare_information(
                document,
                catalog.as_ref(),
                document_packet(document, catalog.as_ref(), edited).as_deref(),
                // The operator's answers for a key Table 349 names nothing for and for a value
                // not of its key's form, read the way the extension schema container reads
                // `undeterminable`: a site the configuration says nothing about states `stop`,
                // which refuses either by name.
                plan.preservations
                    .iter()
                    .find(|preservation| {
                        INFORMATION_REQUIREMENTS.contains(&preservation.site.as_str())
                    })
                    .map_or((super::config::Unmapped::Stop, false), |preservation| {
                        (
                            preservation.unmapped,
                            preservation.unreadable == super::config::Kind::Discard,
                        )
                    }),
            )
        });
        let supplements: Vec<xmp::Supplement<'static>> = information
            .as_ref()
            .map(|moved| moved.supplements.clone())
            .unwrap_or_default();
        // ISO 19005-2 section 6.2.5's transfer functions, prepared before the packet: a function
        // other than the identity is written into its `xmpMM:History` before it goes, and the
        // record is a condition of the removal (`doc/adr/1964`).
        let transfers = asked(wanted(Rewrite::IdentityTransferFunctionRemoved), || {
            super::transfer::removal(document, input)
        });
        let transfer_history = transfers
            .as_ref()
            .ok()
            .and_then(super::transfer::TransferFunctions::history);
        // The images set to `/Interpolate false` are named in the same history, and the entry is a
        // condition of the rewrite, withdrawn below where the packet will not take it (ADRs 2498,
        // 1964).
        let smoothing_history = wanted(Rewrite::InterpolationOff)
            .then(|| super::smoothing::history(input))
            .flatten();
        // Each run written as a number is named in the same history, a condition of the rewrite
        // as smoothing's entry is of its own (`doc/adr/2638`).
        let as_drawn_history = as_drawn.as_ref().ok().map(|drawn| {
            let citation = input
                .failures()
                .find(|judgement| judgement.id == super::fenced::OPERATORS)
                .map_or("", |judgement| judgement.citation.as_str());
            super::as_drawn::history(drawn, citation)
        });
        // The source's header, where it named no version: the output's header states the target's,
        // and the record of what the producer wrote there is a condition of writing it
        // (`doc/adr/2558`). Read off the source, not a structure this program rebuilt from it.
        let header_history = super::header::history(source, plan.target);
        // The TrueType routes are prepared before the packet too, because a restated `cmap` is
        // written into the file's own history (`doc/questions/A48`, `doc/adr/2520`).
        let truetype = {
            let survey = survey.as_ref();
            let symbolic_encodings =
                asked(wanted(Rewrite::SymbolicTrueTypeEncodingRemoved), || {
                    let survey = survey.ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
                    sites::symbolic_truetype_encodings(document, input, survey)
                });
            let standard_encodings = asked(wanted(Rewrite::StandardTrueTypeEncoding), || {
                let survey = survey.ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
                sites::standard_truetype_encodings(document, input, survey)
            });
            let standard_route = asked(wanted(Rewrite::StandardRouteNames), || {
                let survey = survey.ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
                sites::standard_route_names(document, input, survey)
            });
            // The restatement answers a font the three rows' own preparations refuse, so it is
            // made for them only where one of those rows failed and its preparation refused.
            // And not at all where the table refuses another failed requirement by name: the
            // document is refused whatever the fonts say, and a copy of each program is the most
            // this conversion spends on a font. A photographed page can still answer such a row
            // (`Loss::PageAsImage`).
            let rows = fonts::TRUETYPE_ENCODING_ROWS;
            let encoding_row_refused = [
                (rows[0], standard_route.is_err()),
                (rows[1], symbolic_encodings.is_err()),
                (rows[2], standard_encodings.is_err()),
            ]
            .iter()
            .any(|(row, refused)| *refused && failed.contains(row));
            let restated_programs = metrics
                .as_ref()
                .map(|metrics| metrics.at.clone())
                .unwrap_or_default();
            // The `.notdef` rows join the restatement only where every font they name has a
            // TrueType program, because a font the restatement cannot carry refuses it whole,
            // and the fonts the other rows name should not lose their answer to it
            // (`doc/adr/2543`).
            let notdef_too = wanted(Rewrite::NotdefGlyphGiven)
                && fonts::TRUETYPE_NOTDEF_ROWS
                    .iter()
                    .any(|row| failed.contains(row))
                && fonts::notdef_fonts_have_truetype_programs(document, input);
            let asks = (wanted(Rewrite::TrueTypeCmapRestated)
                && (fonts::TRUETYPE_CMAP_ROWS
                    .iter()
                    .any(|row| failed.contains(row))
                    || encoding_row_refused))
                || notdef_too;
            let refused_anyway = !plan.authorised.grants(Loss::PageAsImage)
                && failed.iter().any(|id| super::decision::answers_nothing(id));
            let cmap_restated = if asks && refused_anyway {
                Err(Because::NotBuiltYet(CMAP_NOT_PREPARED))
            } else {
                asked(asks, || {
                    fonts::truetype_cmaps_restated(
                        document,
                        input,
                        &mut spare,
                        &restated_programs,
                        (encoding_row_refused, notdef_too),
                        survey,
                    )
                })
            };
            TrueTypeRoutes {
                symbolic_encodings,
                standard_encodings,
                standard_route,
                cmap_restated,
            }
        };
        let cmap_history = truetype
            .cmap_restated
            .as_ref()
            .ok()
            .and_then(fonts::CmapRestated::history);
        let now = xmp::instant(plan.clock.now());
        let metadata = the_packet(
            plan.target,
            document,
            catalog.as_ref(),
            &mut spare,
            &Recording {
                // Edited when the schema is wanted or when it is deliberately omitted (`A59`), and
                // when the catalog's packet is composed afresh: the replacement states no property,
                // so an identification the producer's packet already stated rightly goes with it
                // unless it is written again (`doc/adr/2455`).
                schema: wanted(Rewrite::IdentificationSchema)
                    || omit_identification
                    || fresh.as_ref().is_ok_and(|fresh| {
                        catalog
                            .as_ref()
                            .and_then(|catalog| catalog.get("Metadata"))
                            .and_then(Object::as_reference)
                            .is_some_and(|at| fresh.packet(at).is_some())
                    }),
                omit_identification,
                departure: provenance.departure,
                derived: provenance.derived,
                supplied: provenance.supplied,
                resolved: provenance.resolved,
                protection: provenance.protection,
                recovered: provenance.recovered,
                preserved: preserved_history.as_deref(),
                attachments: attachments_history.as_deref(),
                attached: attached_history.as_deref(),
                transfers: transfer_history.as_deref(),
                smoothing: smoothing_history.as_deref(),
                as_drawn: as_drawn_history.as_deref(),
                header: header_history.as_deref(),
                cmap: cmap_history.as_deref(),
                when: now.as_deref(),
                // At most one of the two constructions is prepared, so one entry says which.
                default_cmyk: if let Ok(supplied) = &supplied_cmyk {
                    Some(match supplied.press {
                        super::supplied_cmyk::Press::Supplied => super::supplied_cmyk::PARAMETERS,
                        super::supplied_cmyk::Press::Assumed => {
                            super::supplied_cmyk::ASSUMED_PARAMETERS
                        }
                    })
                } else {
                    default_cmyk.is_ok().then_some(DEFAULT_CMYK_PARAMETERS)
                },
                removals: &removals,
                discards: &discards,
                substituted: &font_history,
                supplements: &supplements,
                edited,
            },
        );
        let recorded_it = now.is_some() && metadata.is_ok();
        let recorded_provenance = recorded_it;
        let nothing_removed = properties
            .as_ref()
            .is_ok_and(|cleaned| cleaned.removed.is_empty());
        let default_cmyk = unless_recorded(default_cmyk, recorded_it, NO_PLACE_TO_RECORD);
        let cmyk_not_recorded = match &supplied_cmyk {
            Ok(supplied) if supplied.press == super::supplied_cmyk::Press::Assumed => {
                super::supplied_cmyk::ASSUMED_NO_PLACE_TO_RECORD
            }
            _ => super::supplied_cmyk::NO_PLACE_TO_RECORD,
        };
        let supplied_cmyk = unless_recorded(supplied_cmyk, recorded_it, cmyk_not_recorded);
        let substitutes =
            unless_recorded(substitutes, recorded_it, NO_PLACE_TO_RECORD_A_SUBSTITUTION);
        // A page the fonts' preparation photographed no longer names the content stream it did,
        // so neither content-stream rewrite writes into a stream only such pages named: what is
        // left there is whatever else the file names it as (`doc/adr/2640`).
        let taken_off = match &substitutes {
            Ok(substitutes) => super::hexadecimal::taken_off_by(
                document,
                survey.as_ref(),
                &substitutes.photographs.keys().copied().collect(),
            ),
            Err(_) => BTreeSet::new(),
        };
        let hexadecimal = hexadecimal.map(|completed| completed.leaving_out(&taken_off));
        let languages = languages.map(|restated| restated.leaving_out(&taken_off));
        let properties = unless_recorded(
            properties,
            recorded_it || nothing_removed,
            NO_PLACE_TO_RECORD_A_REMOVAL,
        );
        let nothing_discarded = respelled
            .as_ref()
            .is_ok_and(|respelled| respelled.discarded == 0);
        let respelled = unless_recorded(
            respelled,
            recorded_it || nothing_discarded,
            NO_PLACE_TO_RECORD_A_REMOVAL,
        );
        // The same construction, for the same reason: `doc/adr/1014` section 5 makes the history
        // entry part of the permission to append a page at all, so a packet that will not take it
        // withdraws the pages rather than leaving them unrecorded.
        let preserved = unless_recorded(preserved, recorded_it, NO_PLACE_TO_RECORD_A_PRESERVATION);
        // And for a transfer function: what it said is the one trace of it the archive keeps, so
        // a packet that will not take the record withdraws the removal (`doc/adr/1964`). An
        // identity-only removal records nothing and needs nothing taken.
        let transfers = match transfers {
            Ok(functions) if !functions.costs_nothing() && !recorded_it => {
                Err(Because::NotBuiltYet(NO_PLACE_TO_RECORD_A_TRANSFER))
            }
            transfers => transfers,
        };
        // And for the images set to `/Interpolate false`: the entry is how the file itself says
        // that smoothing was turned off, so a packet that will not take it withdraws the rewrite
        // (`doc/adr/2498`).
        // And for the restated `cmap`: the file's own history is where A48 requires the clause to
        // be named, so a packet that will not take the entry withdraws the restatement
        // (`doc/adr/2520`).
        let mut truetype = truetype;
        if cmap_history.is_some() && !recorded_it {
            truetype.cmap_restated = Err(Because::NotBuiltYet(NO_PLACE_TO_RECORD_CMAP));
        }
        let smoothing_recorded = match smoothing_history {
            Some(_) if !recorded_it => Err(Because::NotBuiltYet(NO_PLACE_TO_RECORD_SMOOTHING)),
            _ => Ok(()),
        };
        let as_drawn_recorded = match as_drawn_history {
            Some(_) if !recorded_it => Err(Because::NotBuiltYet(NO_PLACE_TO_RECORD_A_RUN)),
            _ => Ok(()),
        };
        let header_recorded = match header_history {
            Some(_) if !recorded_it => Err(Because::NotBuiltYet(NO_PLACE_TO_RECORD_HEADER)),
            history => Ok(history),
        };
        // The fonts are the validator's own findings rather than a walk of this converter's:
        // the clause has four exemptions and reading them belongs to `pdf_archive`, so the set of
        // fonts that need a CMap is the set it named, every one of them.
        let to_unicode = asked(wanted(Rewrite::ToUnicode), || {
            unicode_fonts(input)
                .and_then(|fonts| to_unicode::derive(document, &fonts, plan.target, &mut spare))
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
        // **Prepared after the section 6.3.3 construction and from it**: an annotation with no
        // appearance dictionary at all is that rewrite's, and a widget in both populations would
        // otherwise have its `/AP` written twice. `construct = false` is the operator's other
        // answer — the flag goes and the blank fields go with it — so nothing is built.
        let field_appearances = asked(wanted(Rewrite::NeedAppearancesCleared), || {
            if plan.forms.construct_appearances {
                prepare_field_appearances(
                    document,
                    plan.target,
                    appearances.as_ref().ok(),
                    &mut spare,
                )
            } else {
                Ok(FieldAppearances::default())
            }
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
            || {
                // The annotations the subtype rows remove take their action trees with them
                // (ADR 2568), so the walk reads that removal first.
                let removed = forbidden_annotations
                    .as_ref()
                    .map(|forbidden| forbidden.at.clone())
                    .unwrap_or_default();
                super::actions::prepare(document, plan.target, &failed, &removed)
            },
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
            transfers,
            form_resources,
            defined_resources,
            truetype,
        };
        Self {
            raised,
            replaced_destination,
            restated_destination,
            intent,
            supplied_destination,
            default_cmyk,
            default_rgb,
            supplied_cmyk,
            metadata,
            to_unicode,
            appearances,
            fresh,
            amended,
            described,
            fields_described,
            properties,
            respelled,
            substitutes,
            metrics,
            structure,
            signatures: signatures_if_rewritten(document, source, input),
            owed: Owed::of(plan, document, input, &mut spare, &failed, already),
            smoothing_recorded,
            as_drawn_recorded,
            header_recorded,
            hexadecimal,
            as_drawn,
            languages,
            external_data,
            boundaries,
            actions,
            forbidden_annotations,
            hidden_annotations,
            reference_xobjects,
            transcodes,
            shipped_cmaps,
            extra_appearance_states,
            automatic_states,
            form,
            xfa,
            field_appearances,
            embedded_files,
            constructs_field_appearances: plan.forms.construct_appearances,
            preserved,
            attachments,
            attached,
            information,
            recorded_provenance,
            derived_pages_need_an_intent,
        }
    }

    /// What stops a rewrite this document's failures asked for, where something does.
    ///
    /// **The gate between the table and the document.** [`REMEDIES`](super::decision::REMEDIES) answers a requirement with a
    /// rewrite on the standard's evidence alone; whether *this* document can take that rewrite is
    /// a fact only a preparation knows, and `doc/adr/0947`'s rule is that a decision which cannot
    /// be carried out is a refusal rather than a plan. The reason returned is the preparation's
    /// own, never a paraphrase of it.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per rewrite a preparation can stop, each naming the preparation it reads; \
                  split, the one place a reader sees which rewrites a document can refuse would \
                  become several"
    )]
    pub(super) fn obstacle(&self, rewrite: Rewrite) -> Option<Because> {
        match rewrite {
            Rewrite::IdentificationSchema => self.metadata.as_ref().err().copied(),
            Rewrite::InterpolationOff => self.smoothing_recorded.as_ref().err().copied(),
            Rewrite::FileHeader => self.header_recorded.as_ref().err().copied(),
            Rewrite::DestinationProfileReplaced => {
                self.replaced_destination.as_ref().err().copied()
            }
            Rewrite::DestinationProfileRestated => {
                self.restated_destination.as_ref().err().copied()
            }
            Rewrite::DestinationProfileSupplied => {
                self.supplied_destination.as_ref().err().copied()
            }
            Rewrite::MarkInfo => self.structure.as_ref().err().copied(),
            Rewrite::ToUnicode => self.to_unicode.as_ref().err().copied(),
            Rewrite::PropertyOutsideItsSchema => self.properties.as_ref().err().copied(),
            Rewrite::FreshMetadataPacket => self.fresh.as_ref().err().copied(),
            Rewrite::AmendmentIdentifierRemoved => self.amended.as_ref().err().copied(),
            Rewrite::ExtensionSchemaDescribed => self.described.as_ref().err().copied(),
            Rewrite::ExtensionFieldDescribed => self.fields_described.as_ref().err().copied(),
            Rewrite::ExtensionSchemaPrefixes | Rewrite::ExtensionContainerDiscarded => {
                self.respelled.as_ref().err().copied()
            }
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
            Rewrite::PageResources => self
                .owed
                .page_resources
                .as_ref()
                .err()
                .or_else(|| self.owed.form_resources.as_ref().err())
                .copied(),
            Rewrite::EmptyResourceDefined => self.owed.defined_resources.as_ref().err().copied(),
            Rewrite::PageCountRestated => self.owed.page_counts.as_ref().err().copied(),
            Rewrite::DescriptorSetRemoved => self.owed.descriptor_sets.as_ref().err().copied(),
            Rewrite::CidToGidIdentity => self.owed.cid_to_gid.as_ref().err().copied(),
            Rewrite::FontTypeStated => self.owed.font_types.as_ref().err().copied(),
            Rewrite::FontProgramRefiled => self.owed.font_files.as_ref().err().copied(),
            Rewrite::FilterNamesSpelledOut => self.owed.filter_names.as_ref().err().copied(),
            Rewrite::HalftoneNameRemoved => self.owed.halftone_names.as_ref().err().copied(),
            Rewrite::HalftoneWrittenDefault => self.owed.halftone_defaults.as_ref().err().copied(),
            Rewrite::HalftonePhaseRemoved => self.owed.halftone_phases.as_ref().err().copied(),
            Rewrite::HalftoneOriginRemoved => self.owed.halftone_origins.as_ref().err().copied(),
            Rewrite::DestinationProfileReferenceRemoved => {
                self.owed.profile_references.as_ref().err().copied()
            }
            Rewrite::OverprintModeRestated | Rewrite::OverprintModeZeroed => {
                self.owed.overprint_modes.as_ref().err().copied()
            }
            Rewrite::IdentityTransferFunctionRemoved | Rewrite::TransferFunctionRemoved => {
                self.owed.transfers.as_ref().err().copied()
            }
            Rewrite::ProfileAlternateWritten => match &self.owed.profile_alternates {
                Err(because) => Some(*because),
                Ok(alternates) => {
                    alternates.unlicensed_by(self.intent.as_ref().ok().map(|intent| intent.family))
                }
            },
            Rewrite::FontWidthArrayRestated => self.owed.font_widths.as_ref().err().copied(),
            Rewrite::IccComponentCountRestated => {
                self.owed.component_counts.as_ref().err().copied()
            }
            Rewrite::DuplicateProfileWrittenDeviceCmyk => {
                self.owed.duplicate_profiles.as_ref().err().copied()
            }
            Rewrite::PacketHeaderAttributes => self.owed.packet_headers.as_ref().err().copied(),
            Rewrite::SymbolicTrueTypeEncodingRemoved => {
                self.owed.symbolic_encodings.as_ref().err().copied()
            }
            Rewrite::StandardTrueTypeEncoding => {
                self.owed.standard_encodings.as_ref().err().copied()
            }
            Rewrite::StandardRouteNames => self.owed.standard_route.as_ref().err().copied(),
            Rewrite::TrueTypeCmapRestated => self.owed.cmap_restated.as_ref().err().copied(),
            // The restatement goes unasked for a failed `.notdef` row only where a font the row
            // names has no program the copy reaches, so that is the reason it states, naming the
            // program's format (`doc/adr/2601`).
            Rewrite::NotdefGlyphGiven => match &self.owed.cmap_restated {
                Err(Because::NotBuiltYet(NOT_ASKED_FOR)) => {
                    Some(Because::NotBuiltYet(self.owed.notdef_not_given))
                }
                Err(because) => Some(*because),
                Ok(restated) if restated.notdef_answered => None,
                Ok(_) => Some(Because::NotBuiltYet(self.owed.notdef_not_given)),
            },
            Rewrite::Type3NotdefRenamed => self.owed.type3_notdef.as_ref().err().copied(),
            Rewrite::SharedDestinationProfile => self.owed.shared_profile.as_ref().err().copied(),
            Rewrite::SpotColorantEntry => self.owed.colorants.as_ref().err().copied(),
            Rewrite::Jpeg2000ColourSpecifications => {
                self.owed.specifications.as_ref().err().copied()
            }
            Rewrite::PreservedAsPage => self.preserved.as_ref().err().copied(),
            Rewrite::ForbiddenAnnotationRemoved => {
                self.forbidden_annotations.as_ref().err().copied()
            }
            Rewrite::HiddenAnnotationRemoved
            | Rewrite::HiddenAnnotationShown
            | Rewrite::AnnotationFlagsByShape => self.hidden_annotations.as_ref().err().copied(),
            Rewrite::ReferenceXObjectProxied => self.reference_xobjects.as_ref().err().copied(),
            Rewrite::Jpeg2000TranscodedToFlate => self.transcodes.as_ref().err().copied(),
            Rewrite::ShippedCMapEmbedded => self.shipped_cmaps.as_ref().err().copied(),
            Rewrite::ExtraAppearanceStatesRemoved => {
                self.extra_appearance_states.as_ref().err().copied()
            }
            Rewrite::AutomaticStatesRemoved => self.automatic_states.as_ref().err().copied(),
            Rewrite::NeedAppearancesCleared => self
                .form
                .as_ref()
                .err()
                .copied()
                .or_else(|| self.field_appearances.as_ref().err().copied())
                .or_else(|| {
                    self.field_appearances
                        .as_ref()
                        .ok()
                        .filter(|built| !built.unconstructed.is_empty())
                        .map(|_| Because::TheFence(WIDGET_APPEARANCE_NOT_CONSTRUCTED))
                }),
            Rewrite::XfaRemoved => self.xfa.as_ref().err().copied(),
            Rewrite::EmbeddedFileRemoved => self.embedded_files.as_ref().err().copied(),
            Rewrite::HexadecimalDigitCompleted => self.hexadecimal.as_ref().err().copied(),
            Rewrite::RunWrittenAsDrawn => self
                .as_drawn
                .as_ref()
                .err()
                .copied()
                .or_else(|| self.as_drawn_recorded.as_ref().err().copied()),
            Rewrite::CatalogLanguageRestated => match &self.languages {
                Ok(restated) => restated.catalog_obstacle,
                Err(because) => Some(*because),
            },
            Rewrite::MarkedLanguagesRestated => match &self.languages {
                Ok(restated) => restated.marked_obstacle,
                Err(because) => Some(*because),
            },
            Rewrite::ExternalDataEmbedded => self.external_data.as_ref().err().copied(),
            Rewrite::PageBoundaryRemoved => self.boundaries.as_ref().err().copied(),
            Rewrite::InformationMovedIntoThePacket => self.information.as_ref().err().copied(),
            Rewrite::PreservedAsAttachment => self.attached.as_ref().err().copied(),
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

    /// The objects the conversion adds, in the source's numbering, for `super::rewrite::enter` to find.
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
        if let Ok(default) = &self.default_rgb {
            for (id, object) in &default.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(default) = &self.supplied_cmyk {
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
        if let Ok(built) = &self.field_appearances {
            for (id, object) in &built.written {
                out.insert(*id, object.clone());
            }
        }
        // Before the substitutes: a standard font defined here is written plain, and the font the
        // embedding route made of it is written over it at the same number.
        for resources in [&self.owed.form_resources, &self.owed.defined_resources]
            .into_iter()
            .flatten()
        {
            for (id, object) in &resources.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(substitutes) = &self.substitutes {
            for (id, object) in &substitutes.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(metrics) = &self.metrics {
            for (id, object) in &metrics.written {
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
        if let Ok(attached) = &self.attached {
            for (id, object) in &attached.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(shipped) = &self.shipped_cmaps {
            for (id, object) in &shipped.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(transcodes) = &self.transcodes {
            for (id, object) in &transcodes.written {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(refiled) = &self.owed.font_files {
            for (id, object) in refiled.written() {
                out.insert(*id, object.clone());
            }
        }
        if let Ok(restated) = &self.owed.cmap_restated {
            for (id, object) in restated.written() {
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
    /// The simple fonts whose widths-array length is restated, or why none are.
    pub(super) font_widths: Result<super::font_widths::WidthArrays, Because>,
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
    /// The forms, patterns and appearances given the dictionary of their page, or why none are
    /// (`doc/adr/2500`).
    pub(super) form_resources: Result<ReaderResources, Because>,
    /// The empty resources that define the names nothing defined, or why none do (`doc/adr/2500`).
    pub(super) defined_resources: Result<ReaderResources, Because>,
    /// The page tree nodes whose `/Count` is restated, or why none are (`doc/adr/2456`).
    pub(super) page_counts: Result<super::page_tree::RestatedCounts, Because>,
    /// The font descriptors whose incomplete subset description goes, or why none do.
    pub(super) descriptor_sets: Result<DescriptorSets, Because>,
    /// The `CIDFont`s given `/CIDToGIDMap` `/Identity`, or why none are.
    pub(super) cid_to_gid: Result<sites::CidToGidSites, Because>,
    /// The font dictionaries given `/Type /Font`, or why none are (`doc/adr/2608`).
    pub(super) font_types: Result<sites::FontTypeSites, Because>,
    /// The font programs filed under the key and name of their format, or why none are (ADR 2307).
    pub(super) font_files: Result<super::font_files::Refiled, Because>,
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
    /// The TrueType fonts whose shown codes are named so §9.6.5.4's own route reaches their
    /// glyphs, or why none are.
    pub(super) standard_route: Result<StandardRouteNames, Because>,
    /// The TrueType fonts whose program's `cmap` is restated, with the objects that hold them.
    pub(super) cmap_restated: Result<fonts::CmapRestated, Because>,
    /// What a `.notdef` row the restatement does not answer is refused with: the format of the
    /// program a font it names embeds, where the restatement does not extend that format
    /// (`doc/adr/2601`).
    pub(super) notdef_not_given: &'static str,
    /// The Type 3 fonts whose `.notdef` glyph is renamed, or why none are.
    pub(super) type3_notdef: Result<Type3Renames, Because>,
    /// The output intents pointed at one shared destination profile, or why none are.
    pub(super) shared_profile: Result<SharedProfile, Because>,
    /// The `/Colorants` entries each `DeviceN` colour space gains, or why none are written.
    pub(super) colorants: Result<ColorantEntries, Because>,
    /// The `JPXDecode` images whose colour specification boxes are reduced, or why none are.
    pub(super) specifications: Result<Specifications, Because>,
    /// The permissions dictionary's keys outside Table 263, or why none are removed.
    pub(super) foreign_handlers: Result<ForeignHandlers, Because>,
    /// Whether every non-standard filter name is an abbreviation spelled out, or why not.
    pub(super) filter_names: Result<(), Because>,
    /// The halftones whose `HalftoneName` goes, or why none do.
    pub(super) halftone_names: Result<Sites, Because>,
    /// The graphics states whose `HT` is written `Default`, or why none are (`doc/adr/2034`).
    pub(super) halftone_defaults: Result<Sites, Because>,
    /// The graphics states whose `HTP` goes, or why none do (`doc/adr/2035`).
    pub(super) halftone_phases: Result<Sites, Because>,
    /// The graphics states whose `HTO` goes, or why none do (`doc/adr/2034`).
    pub(super) halftone_origins: Result<Sites, Because>,
    /// The output intents whose `DestOutputProfileRef` goes, or why none do (`doc/adr/2083`).
    pub(super) profile_references: Result<Sites, Because>,
    /// The graphics states whose overprint mode becomes 0, or why none do.
    pub(super) overprint_modes: Result<super::overprint::OverprintModes, Because>,
    /// The graphics states whose `/TR` goes, or why none do.
    pub(super) transfers: Result<super::transfer::TransferFunctions, Because>,
    /// The space each unreadable ICC profile's arrays are written as, or why none are.
    pub(super) profile_alternates: Result<super::icc::Alternates, Because>,
    /// The profiles whose `ICCBased` arrays are named `DeviceCMYK`, or why none are
    /// (`doc/adr/2098`).
    pub(super) duplicate_profiles: Result<super::duplicate_profiles::Duplicates, Because>,
    /// The `ICCBased` profile streams whose `N` is restated, or why none are (`doc/adr/2099`).
    pub(super) component_counts: Result<super::component_counts::ComponentCounts, Because>,
}

impl Owed {
    /// Each of them, where a failed requirement asked for it.
    #[expect(
        clippy::too_many_lines,
        reason = "one field per preparation, each asked only where a failed requirement wants \
                  it; the list is read straight down, and splitting it would hide which \
                  preparation a rewrite is given behind a call"
    )]
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
            transfers,
            form_resources,
            defined_resources,
            truetype,
        } = already;
        let TrueTypeRoutes {
            symbolic_encodings,
            standard_encodings,
            standard_route,
            cmap_restated,
        } = truetype;
        Self {
            font_widths: asked(wanted(Rewrite::FontWidthArrayRestated), || {
                super::font_widths::restated(document, input)
            }),
            rendering_intents: asked(wanted(Rewrite::RenderingIntent), || {
                sites::rendering_intents(document, input)
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
                sites::page_resources(document, input, survey)
            }),
            defined_resources,
            form_resources,
            page_counts: asked(wanted(Rewrite::PageCountRestated), || {
                super::page_tree::restated(document)
            }),
            descriptor_sets: asked(wanted(Rewrite::DescriptorSetRemoved), || {
                sites::descriptor_sets(document, input)
            }),
            cid_to_gid: asked(wanted(Rewrite::CidToGidIdentity), || {
                sites::cid_to_gid_maps(document, input, plan.target)
            }),
            font_types: asked(wanted(Rewrite::FontTypeStated), || {
                sites::font_types(document, input, plan.target)
            }),
            font_files: asked(wanted(Rewrite::FontProgramRefiled), || {
                super::font_files::refiled(document, input, spare)
            }),
            filter_names: asked(wanted(Rewrite::FilterNamesSpelledOut), || {
                sites::abbreviated_filters(input)
            }),
            halftone_names: asked(wanted(Rewrite::HalftoneNameRemoved), || {
                super::halftones::removal(document, input)
            }),
            halftone_defaults: asked(wanted(Rewrite::HalftoneWrittenDefault), || {
                super::halftones::defaults(document, input)
            }),
            halftone_phases: asked(wanted(Rewrite::HalftonePhaseRemoved), || {
                super::halftones::phases(document, input)
            }),
            halftone_origins: asked(wanted(Rewrite::HalftoneOriginRemoved), || {
                super::halftones::origins(document, input)
            }),
            profile_references: asked(wanted(Rewrite::DestinationProfileReferenceRemoved), || {
                sites::profile_references(document, input)
            }),
            overprint_modes: asked(wanted(Rewrite::OverprintModeRestated), || {
                let survey = survey.ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
                super::overprint::zeroing(document, survey)
            }),
            profile_alternates: asked(wanted(Rewrite::ProfileAlternateWritten), || {
                super::icc::Alternates::of(document, super::icc::Reading::of(plan.target))
            }),
            duplicate_profiles: asked(wanted(Rewrite::DuplicateProfileWrittenDeviceCmyk), || {
                let survey = survey.ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
                super::duplicate_profiles::collapse(document, survey)
            }),
            component_counts: asked(wanted(Rewrite::IccComponentCountRestated), || {
                super::component_counts::restated(document)
            }),
            transfers,
            packet_headers,
            symbolic_encodings,
            standard_encodings,
            type3_notdef: asked(wanted(Rewrite::Type3NotdefRenamed), || {
                sites::type3_notdef_renames(document, input)
            }),
            cmap_restated,
            notdef_not_given: if wanted(Rewrite::NotdefGlyphGiven) {
                fonts::notdef_not_given(document, input)
            } else {
                fonts::NOTDEF_NOT_GIVEN
            },
            standard_route,
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
///
/// Where `document` is the structure this reader recovered from `source`, it is always rewritten
/// and the signatures are read against `source`, the only bytes they were ever made over
/// (`doc/adr/2523`).
fn signatures_if_rewritten(
    document: &Document,
    source: &Document,
    input: &pdf_archive::Report,
) -> Signatures {
    if !std::ptr::eq(document, source) {
        signatures::find_over(document, source)
    } else if input.verdict() == Verdict::Fails {
        signatures::find(document)
    } else {
        Signatures::default()
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
    /// The transfer functions removed, prepared before the packet that records them.
    transfers: Result<super::transfer::TransferFunctions, Because>,
    /// The resources given to the forms that state none, prepared before the fonts they define.
    form_resources: Result<ReaderResources, Because>,
    /// The names defined as empty resources, prepared before the fonts they define.
    defined_resources: Result<ReaderResources, Because>,
    /// The four TrueType preparations, made before the packet because the restated `cmap` is
    /// recorded in it.
    truetype: TrueTypeRoutes,
}

/// The preparations that put a rendered TrueType font's codes on §9.6.5.4's route.
///
/// Together because the restated `cmap` answers each of the three encoding rows where that
/// row's own preparation refuses (`Answer::OrStated`), so the four are decided as one
/// (`doc/adr/2520`).
struct TrueTypeRoutes {
    /// The symbolic fonts whose `/Encoding` is removed, or why none are.
    symbolic_encodings: Result<Sites, Because>,
    /// The non-symbolic fonts given one of the two admitted names, or why none are.
    standard_encodings: Result<StandardEncodings, Because>,
    /// The fonts whose shown codes are named so the route reaches their glyphs, or why none are.
    standard_route: Result<StandardRouteNames, Because>,
    /// The fonts whose program's `cmap` is restated, or why none are.
    cmap_restated: Result<fonts::CmapRestated, Because>,
}

/// The rewrites whose preparation reads what the pages drew.
///
/// Every one of them needs `pdf_archive::survey::Survey`, which walks each page's content
/// streams — so the walk is made once where any of them is wanted and not at all where none is.
const SURVEYED: [Rewrite; 12] = [
    Rewrite::PageResources,
    Rewrite::EmptyResourceDefined,
    Rewrite::MarkedLanguagesRestated,
    Rewrite::DuplicateProfileWrittenDeviceCmyk,
    Rewrite::OverprintModeRestated,
    Rewrite::SubstituteFontProgram,
    Rewrite::RestateFontMetrics,
    Rewrite::RestateVerticalFontMetrics,
    Rewrite::SymbolicTrueTypeEncodingRemoved,
    Rewrite::StandardTrueTypeEncoding,
    Rewrite::StandardRouteNames,
    Rewrite::NotdefGlyphGiven,
];

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
/// From [`pdf_syntax::XrefTable::next_object_number`] — past every number a cross-reference
/// section states and past Table 15's `/Size` (ADR 2362) — and then checked one at a time:
/// **the objects a conversion adds are built in the source's numbering** like every other object
/// this verb rewrites, so a number that collided with a source object would silently replace it.
pub(super) struct Spare {
    /// The next number to try.
    pub(super) next: u32,
}

impl Spare {
    /// Begins where every writer in this tree numbers its first new object.
    fn of(document: &Document) -> Self {
        Self {
            next: document.xref().next_object_number(),
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

/// ISO 19005-2 section 6.2.4.3's grey row, which any PDF/A output intent answers.
const DEVICE_GRAY_ROW: &str = "graphics/device-gray-needs-a-default-or-an-output-intent";

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

/// The requirement a removed property answers.
const SCHEMA_REQUIREMENT: &str = "metadata/properties-use-known-schemas";

/// The requirement the container answers.
pub(super) const SCHEMAS_REQUIREMENT: &str = "metadata/extension-schemas-embedded";

/// ISO 19005-2 section 6.6.2.3.1's half that reaches inside a structure, the site
/// [`prepare_field_descriptions`] answers.
pub(super) const FIELDS_SITE: &str = "metadata/extension-schema-structure-fields-are-described";

/// The three requirements a fresh packet answers.
///
/// ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1. The data-model row is part 2's
/// alone; its identifier from [`pdf_archive::PacketFault::requirement`] is absent from a part 4
/// target's failed requirements because the rule does not bind there, and
/// [`prepare_fresh_packets`] tests that set before replacing a packet for this fault.
pub(super) const PACKET_REQUIREMENTS: [&str; 3] = [
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

/// The two embedded-file requirements, one per part, that a `preserve` by page answers.
pub(super) const ATTACHMENT_REQUIREMENTS: [&str; 2] = [
    "embedded-files/embedded-file-is-itself-pdfa",
    "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
];

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

/// ISO 19005-4 section 6.1.3's two rows, whose `unmapped` answer is one answer for both.
const INFORMATION_REQUIREMENTS: [&str; 2] = [
    "file-structure/document-information-dictionary-needs-piece-info",
    "file-structure/document-information-dictionary-holds-only-a-modification-date",
];
