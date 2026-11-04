//! The metadata family of the preparations: the XMP packets a conversion edits, replaces or
//! writes fresh, the identification schema, the extension schemas that describe a property, the
//! document information dictionary moved into the packet, and the deprecated attributes cut out
//! of a packet's header.
//!
//! They are one family because each edits the same bytes, the document's XMP, in a fixed order —
//! the header cut first, then the properties, then the packet written — so a writer that runs
//! later starts from what the earlier ones left. Most of the rows they serve are
//! `decision/metadata`'s, section 6 of `doc/pdfa-blocking-issues.md`; the document information
//! dictionary's are `file-structure/` rows whose remedy is that packet.

use super::super::config::{UNMAPPED_NEEDS_A_NAMESPACE, Unmapped};
use super::super::decision::Because;
use super::super::sites;
use super::{
    Cleaned, Fresh, NO_CATALOG, NO_SPARE_OBJECT, NOT_A_PACKET, NOT_ASKED_FOR, SCHEMA_REQUIREMENT,
    Spare,
};
use crate::json::Value;
use pdf_archive::{Flavour, Level, Target};
use pdf_archive::{MisusedProperty, Outcome};
use pdf_model::xmp::{self, Name as XmpName, Schema};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

// §14.3.3's Table 349, key by key, shared with the `update` and `merge` verbs so that the three
// writers cannot name an entry's XMP counterpart two ways.
use crate::update::TABLE_349;

/// The action this conversion records in `xmpMM:History` when it writes the `/DefaultCMYK`.
///
/// `doc/questions/A48` allows the construction on two conditions, and this is the second of them:
/// the clause is named in the file's own provenance, so that a later reader knows precisely which
/// approximation was applied and can undo the interpretation. ISO 19005-2 section 6.6.6 asks a
/// recorded action for what was done, with what, and when; ISO 19005-4 section 6.7.5 asks for two
/// of the three. All three are written, which answers both.
pub(in crate::archive) const DEFAULT_CMYK_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it removes a property.
///
/// `doc/pdf-a-conversion-limits.md` section 4.2: every **Ask** writes one entry, and the entry is
/// the audit trail that makes the Asks defensible. A removed property is the one loss in this
/// verb that a reader of the output cannot see the shape of — a deleted key leaves no hole — so
/// the provenance is where the file itself says what it lost.
pub(in crate::archive) const REMOVED_PROPERTIES_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it departs from a requirement.
///
/// `doc/rfc/0007` section 4.7.3: a departure is recorded in the file itself, so the archive carries
/// the fact that it goes against the standard on purpose. The word is `converted`, like the other
/// recorded actions — ISO 19005-4 section 6.7.5's history records what a converter did — and the
/// parameters name the requirement, the predicate that narrowed the departure, and the operator's
/// stated reason, which section 4.7.2 requires and which is the whole of what makes a departure
/// something a reader of the archive can understand two years on.
pub(in crate::archive) const DEPARTED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when a tool derived an artefact.
///
/// `doc/questions/A55` makes the record one of the four terms the mode is offered on: the *archive*
/// carries the fact that part of it is derived rather than original, instead of that fact living
/// only in a report somebody may not have kept. The word is `converted`, like every other recorded
/// action here — ISO 19005-4 section 6.7.5's history records what a converter did — and the
/// parameters carry the owner's own sentence, *this is derived, not original*, with what was
/// derived, from what, and by which tool.
pub(in crate::archive) const DERIVED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` for a fact the operator supplied.
///
/// `doc/rfc/0007` section 5b.1's obligation, which `supply` carries and the other four remedy kinds
/// do not: it is the one kind the converter cannot get wrong and the person can, so the file itself
/// records that a human rather than the document is the value's source.
pub(in crate::archive) const SUPPLIED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` for a stream an operator's tool fetched.
///
/// `doc/adr/1209`. The rewrite writes the fetched bytes where the stream's own were, so nothing in
/// the file says afterwards that they came from outside it — which is exactly the fact a reader of
/// an archive is entitled to find in the archive. The word is `converted`, like every other
/// recorded action here.
pub(in crate::archive) const RESOLVED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when a page preserves content.
///
/// `doc/adr/1014` section 5's fifth bullet, which the amendment makes a condition of the
/// permission rather than a nicety: a page appended to keep content the target would otherwise
/// lose is a change to the document a reader of the archive is entitled to find in the archive.
/// The word is `converted`, like every other recorded action here.
pub(in crate::archive) const PRESERVED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when a file keeps content.
///
/// The other `preserve` mechanism's own entry, and the reason is `doc/rfc/0007` section 2.1's
/// rather than a permission's: the bytes are in the archive under a §7.9.6 name nothing else in
/// the document states, and a reader who has only the file is entitled to find that out from the
/// file. **Not a condition of the remedy**, which is where it differs from the page: nothing is
/// composed and nothing is invented, so there is no permission an unwritten record would void
/// (`doc/adr/1270`). The word is `converted`, like every other recorded action here.
pub(in crate::archive) const ATTACHED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it removes a source's encryption.
///
/// `doc/pdf-a-mitigations.md` section 2's `preserve` beside the `discard`, and `doc/adr/1187` the
/// argument: an unencrypted archive cannot enforce §7.6.4.2's Table 22 flags, so what the producer
/// asserted about its reader goes into the file itself before the enforcement goes. The word is
/// `converted`, like every other recorded action here.
pub(in crate::archive) const DECRYPTED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it writes the structure it recovered.
///
/// `doc/adr/2502`: the source stated no page tree this program writes, the output states one, and
/// the file's own provenance says so rather than leaving the report the only place it is said.
/// The word is `converted`, like every other recorded action here.
pub(in crate::archive) const RECOVERED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it writes the target's header over a
/// source whose header stated no version (`doc/adr/2558`).
pub(in crate::archive) const HEADER_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it embeds a substitute face.
///
/// ISO 19005-2 section 6.6.6's NOTE 1 and ISO 19005-4 section 6.7.5's NOTE both give font
/// substitution as an example of a converter action that changes a document's appearance and is
/// therefore to be recorded — which is `doc/pdf-a-conversion-limits.md` section 4.9's third
/// argument for the act being contemplated by the standard rather than merely not forbidden.
pub(in crate::archive) const SUBSTITUTED_FONTS_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it removes a transfer function.
///
/// `doc/adr/1964`: §10.5's function is data, so what the producer specified goes into the file
/// before the archive stops applying it. The word is `converted`, like every other recorded
/// action here.
pub(in crate::archive) const TRANSFER_REMOVED_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it writes `/Interpolate` false.
///
/// `doc/adr/2498`: the loss `image-smoothing` is named in the file as well as in the report. The
/// word is `converted`, like every other recorded action here.
pub(in crate::archive) const SMOOTHING_OFF_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it rebuilds the parent tree.
///
/// `doc/adr/2674`: the entries are the structure tree read backwards, and the history says how
/// many were written. The word is `converted`, like every other recorded action here.
pub(in crate::archive) const PARENT_TREE_REBUILT_ACTION: &str = "converted";

/// The action this conversion records in `xmpMM:History` when it restates a TrueType font's `cmap`.
///
/// `doc/adr/2520`: the route this reader took is the file's statement from now on, and
/// `doc/questions/A48` makes the entry naming the clause the condition of that. The word is
/// `converted`, like every other recorded action here.
pub(in crate::archive) const CMAP_RESTATED_ACTION: &str = "converted";

/// Why a packet's properties could not be removed from it.
const PACKET_NOT_CUTTABLE: &str = "this document states a metadata property in a predefined \
     schema that does not define it, and the packet it is in cannot be edited in place — so the \
     property cannot be removed without rewriting metadata its producer wrote. \
     doc/pdf-a-conversion-limits.md section 3.9 is the reading";

/// Why a property that survived the cut refuses the document.
const PROPERTY_NOT_CUT: &str = "a metadata property this conversion removed is still in the \
     packet afterwards, so the packet is half-edited rather than corrected, and a half-edited \
     packet is not written at all — doc/pdf-a-conversion-limits.md section 3.9";

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

/// The two spellings the two parts print for the identification schema's namespace.
///
/// ISO 19005-2 section 6.6.4 gives it with an `http` scheme and ISO 19005-4 section 6.7.3 with an
/// `https` one, which erratum #123 corrects to `http` (`pdf_archive::errata`). A conversion writes
/// the first for every target and **removes both**, because a property left behind under the other
/// spelling would be a second claim standing beside the one just written (`doc/adr/2051`).
const IDENTIFICATION_URIS: [&str; 2] = [
    "http://www.aiim.org/pdfa/ns/id/",
    "https://www.aiim.org/pdfa/ns/id/",
];

/// The prefix both parts make required for every property of the identification schema.
const IDENTIFICATION_PREFIX: &str = "pdfaid";

/// The publication year ISO 19005-4's own revision property names.
const REVISION_YEAR: &str = "2020";

/// Why a producer's XMP packet could not be given the identification schema.
const PACKET_NOT_EDITABLE: &str = "this document's XMP packet cannot be edited in place, and \
     replacing it would throw away metadata its producer wrote — which is a loss nobody has been \
     asked to authorise. doc/pdf-a-conversion-limits.md section 4.2 is where that question \
     belongs, and this converter does not put it yet";

/// The properties the identification schema states for one target.
///
/// ISO 19005-2 section 6.6.4 asks for a part number of 2 and a conformance level of A, B or U;
/// ISO 19005-4 section 6.7.3 asks for a part number of 4 and a revision year, and reserves a
/// conformance property for the two annexes — a file that is neither PDF/A-4e nor PDF/A-4f states
/// none at all. Part 4's printed table spells that property `pdfa:conformance` and erratum #123
/// corrects it to `pdfaid:conformance`, the prefix the subclause makes required and the one
/// `metadata/identification-schema-prefix` holds a file to — so `pdfaid` is what this writes.
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

/// The namespace URI the target's own part states for the identification schema.
///
/// The same for both parts once erratum #123 is read: part 4's printed `https` is the error it
/// corrects, and the note it adds permits a reader to accept that spelling, not a writer to
/// produce it (`doc/adr/2051`).
const fn identification_uri(target: Target) -> &'static str {
    match target.part() {
        pdf_archive::Part::Two | pdf_archive::Part::Four => IDENTIFICATION_URIS[0],
    }
}

/// The packets a conversion is in a position to respell a container's prefixes in.
///
/// ISO 19005-2 section 6.6.2.3.3's four tables each name the prefix their fields are required to
/// be spelled with, and section 6.6.2.2 makes that the one place in an XMP packet where a prefix
/// is part of what is required rather than a convenience. So a field stated in the right namespace
/// with the wrong prefix is a *spelling* this converter can correct out of what the file already
/// holds — which is what separates it from the fields nothing in the file states.
#[derive(Debug)]
pub(in crate::archive) struct Respelled {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(in crate::archive) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// How many fields were respelled, for the report to count places by.
    pub(in crate::archive) fields: usize,
    /// The properties that went with a container missing a field the file states nowhere, where
    /// the catalogue's `discard` was prepared (`super::super::extension_container`).
    pub(in crate::archive) removed: Vec<MisusedProperty>,
    /// How many containers were discarded, each missing a field the file states nowhere.
    pub(in crate::archive) discarded: usize,
}

/// The metadata stream a conversion is in a position to write.
pub(in crate::archive) struct Metadata {
    /// The object the packet goes in, in the *source's* numbering.
    pub(in crate::archive) at: ObjectId,
    /// The stream this conversion adds, where the document had no packet to edit.
    pub(in crate::archive) written: Option<Object>,
    /// The packet, for the stream that carries it.
    pub(in crate::archive) packet: Vec<u8>,
}

/// Every metadata stream one metadata requirement was found to fail at.
///
/// The validator's own findings rather than a walk of this converter's, for
/// [`unicode_fonts`](super::font_rewrites::unicode_fonts)'s reason: which properties a predefined schema defines is `pdf_archive`'s
/// reading. **The list it names is capped**, so where the requirement failed at more places than
/// the list kept, the population is every metadata stream the validator walks
/// (`pdf_archive::metadata_streams`) — each caller asks `pdf_archive` about each packet again and
/// passes over one with nothing to do, so the wider population changes nothing but which packets
/// are reached (ADR 2295).
pub(super) fn schema_packets(
    document: &Document,
    input: &pdf_archive::Report,
    requirement: &str,
) -> Vec<ObjectId> {
    let failed: Vec<_> = input
        .failures()
        .filter(|judgement| judgement.id == requirement)
        .filter_map(|judgement| match &judgement.outcome {
            Outcome::Failed { places, total } => Some((places, *total)),
            _ => None,
        })
        .collect();
    if failed.iter().any(|(places, total)| *total > places.len()) {
        // A stream that does not decode is a fault of another row's (section 6.6.2.1's), which
        // the callers would refuse on as though this row had named it.
        return pdf_archive::metadata_streams(document)
            .into_iter()
            .filter(|id| match document.get(*id) {
                Object::Stream(stream) => document.decoded_stream_data(&stream).is_some(),
                _ => false,
            })
            .collect();
    }
    let mut out: Vec<ObjectId> = failed
        .into_iter()
        .flat_map(|(places, _)| places)
        .filter_map(|finding| finding.place.object)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Takes every property a predefined schema does not define, and every property stated outside
/// the form its extension schema's description gives it, out of the packets that state them.
///
/// **Nothing is judged here.** `pdf_archive::properties_outside_their_schema` and
/// `properties_outside_their_description` are the same reading the requirement's own row is,
/// asked of one packet's bytes, so a property this cuts is exactly a property that row reported. What this decides is only whether the cut can be made at all — and
/// a packet left holding one afterwards refuses the document rather than leaving it half-edited,
/// which is `doc/pdf-a-conversion-limits.md` section 3.9's line.
pub(super) fn prepare_properties(
    document: &Document,
    input: &pdf_archive::Report,
    headers: Option<&Headers>,
    fresh: Option<&Fresh>,
) -> Result<Cleaned, Because> {
    // Section 6.6.2.3.2 lets the catalog's packet describe a schema for every stream, so its
    // descriptions type a property here as well, read as the header cut left them.
    let catalog_packet = document.catalog().ok().and_then(|catalog| {
        let at = catalog.get("Metadata").and_then(Object::as_reference)?;
        let Object::Stream(stream) = document.get(at) else {
            return None;
        };
        headers
            .and_then(|headers| headers.packet(at).cloned())
            .or_else(|| {
                document
                    .decoded_stream_data(&stream)
                    .map(|bytes| bytes.to_vec())
            })
    });
    let mut packets = BTreeMap::new();
    let mut removed = Vec::new();
    let mut replaced: usize = 0;
    for at in schema_packets(document, input, SCHEMA_REQUIREMENT) {
        // The fresh packet states no property at all, so there is nothing here to cut out of it
        // and the producer's own bytes are not what this stream will carry.
        if fresh.is_some_and(|fresh| fresh.packet(at).is_some()) {
            replaced = replaced.saturating_add(1);
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
        let misused = outside_their_schemas(&bytes, catalog_packet.as_deref());
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
        if !outside_their_schemas(&cut, catalog_packet.as_deref()).is_empty() {
            return Err(Because::NotBuiltYet(PROPERTY_NOT_CUT));
        }
        packets.insert(at, cut);
        removed.extend(misused);
    }
    Ok(Cleaned {
        packets,
        removed,
        replaced,
    })
}

/// Both halves of the requirement's reading, asked of one packet: the properties a predefined
/// schema does not define as stated, and those stated outside the form a description gives them.
fn outside_their_schemas(packet: &[u8], catalog_packet: Option<&[u8]>) -> Vec<MisusedProperty> {
    let mut misused = pdf_archive::properties_outside_their_schema(packet);
    misused.extend(pdf_archive::properties_outside_their_description(
        packet,
        catalog_packet,
    ));
    misused
}

/// The packets that gain a container describing the extension schemas they use.
#[derive(Debug, Default)]
pub(in crate::archive) struct SchemasDescribed {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(in crate::archive) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// Every schema described, for the report, in the order the packets state them.
    pub(in crate::archive) described: Vec<DescribedSchema>,
    /// Every property `undeterminable = "discard"` took out of a packet, which the report names
    /// as it names every other property a conversion removes (`doc/adr/1991`).
    pub(in crate::archive) cut: Vec<MisusedProperty>,
    /// Every metadata stream the requirement failed at whose packet section 6.6.2.1's replacement
    /// composes afresh, stating no schema at all: there the replacement is what meets the
    /// requirement, and nothing is described (`doc/adr/2442`).
    pub(in crate::archive) replaced: Vec<ObjectId>,
}

impl SchemasDescribed {
    /// Whether every stream the requirement failed at was replaced and none described, so that
    /// the replacement's decision is the row's (`doc/adr/2442`).
    pub(in crate::archive) fn only_replaced(&self) -> bool {
        self.packets.is_empty() && !self.replaced.is_empty()
    }
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
    pub(in crate::archive) fn to_json(&self) -> Value {
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
pub(super) fn prepare_schema_descriptions(
    document: &Document,
    catalog: Option<&Dictionary>,
    failing: &[ObjectId],
    discard_undeterminable: bool,
    edited: Edited<'_>,
) -> Result<SchemasDescribed, Because> {
    let catalog_packet = document_packet(document, catalog, edited);
    let mut out = SchemasDescribed::default();
    for at in pdf_archive::metadata_streams(document) {
        // A stream the fresh packet replaces carries no schema of the producer's, so nothing in it
        // is described; where the requirement failed there, the replacement meets it, and the
        // decision is the replacement's own (`doc/adr/2442`).
        if edited.fresh.is_some_and(|fresh| fresh.packet(at).is_some()) {
            if failing.contains(&at) {
                out.replaced.push(at);
            }
            continue;
        }
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
            out.cut.extend(undeterminable_properties(&undescribed));
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
    if out.packets.is_empty() && out.replaced.is_empty() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    Ok(out)
}

/// The packets whose value types gain a description of the structure fields they state.
#[derive(Debug, Default)]
pub(in crate::archive) struct FieldsDescribed {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(in crate::archive) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// Every field described, for the report, in the order the packets state them.
    pub(in crate::archive) described: Vec<DescribedField>,
    /// Every metadata stream the requirement failed at whose packet section 6.6.2.1's replacement
    /// composes afresh, stating no value type at all: there the replacement is what meets the
    /// requirement, and nothing is described (`doc/adr/2455`).
    pub(in crate::archive) replaced: Vec<ObjectId>,
}

impl FieldsDescribed {
    /// Whether every stream the requirement failed at was replaced and none described, so that
    /// the replacement's decision is the row's (`doc/adr/2455`).
    pub(in crate::archive) fn only_replaced(&self) -> bool {
        self.packets.is_empty() && !self.replaced.is_empty()
    }
}

/// One structure field this conversion described in its value type, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribedField {
    /// The value type's `pdfaType:type`, which is the packet's own.
    pub type_name: String,
    /// The value type's field namespace, which is the packet's own.
    pub namespace: String,
    /// The field's local name, which is the packet's own.
    pub name: String,
    /// The value type its serialisation showed.
    pub value_type: String,
    /// The `pdfaField:description` written: the operator's sentence where their
    /// `field-descriptions` names the field, and [`FIELD_DESCRIPTION`] otherwise.
    pub description: String,
    /// Whether [`Self::description`] is the operator's (`doc/adr/2082`).
    pub supplied: bool,
}

impl DescribedField {
    /// One described field as JSON.
    pub(in crate::archive) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("type".to_owned(), Value::text(self.type_name.clone())),
            ("namespace".to_owned(), Value::text(self.namespace.clone())),
            ("name".to_owned(), Value::text(self.name.clone())),
            (
                "value_type".to_owned(),
                Value::text(self.value_type.clone()),
            ),
            (
                "description".to_owned(),
                Value::text(self.description.clone()),
            ),
            (
                "described_by".to_owned(),
                Value::text(if self.supplied {
                    "operator"
                } else {
                    "converter"
                }),
            ),
        ])
    }
}

/// The key an operator's `field-descriptions` names a field by: its namespace URI and its local
/// name written together, which is how RDF names a property, so that the key does not depend on
/// the prefix one packet happens to spell the namespace with (ISO 19005-2 section 6.6.2.2).
pub(in crate::archive) fn field_key(namespace: &str, local: &str) -> String {
    let mut key = String::with_capacity(namespace.len().saturating_add(local.len()));
    key.push_str(namespace);
    key.push_str(local);
    key
}

/// Whether a `field-descriptions` key could be a [`field_key`]: a namespace URI, which holds a
/// scheme's colon, with a local name after its last `/`, `#` or `:`.
pub(in crate::archive) fn names_a_field(key: &str) -> bool {
    key.contains(':')
        && key
            .rsplit(['/', '#', ':'])
            .next()
            .is_some_and(|local| !local.is_empty())
}

/// The `pdfaField:description` field, which no file states for a field its producer described
/// nowhere.
///
/// ISO 19005-2 section 6.6.2.3.3's Table 6 makes the field required and calls it the field's
/// description. A producer whose description of the value type left the field out stated nothing
/// about what it means, so what goes here is a sentence saying exactly that, for [`SCHEMA_NAME`]'s
/// reason (`doc/adr/2050`).
const FIELD_DESCRIPTION: &str = "a field this file states for a value of this type, described by \
     the converter that archived it because the producer's description of the type left it out";

/// Why a field named in another namespace than its value type's is not described.
const FIELD_IN_ANOTHER_NAMESPACE: &str = "this packet states a structure field in a namespace \
     other than the field namespace its value type's own description gives, and a value type \
     describes fields in its one namespace alone (ISO 19005-2 section 6.6.2.3.3, Table 5), so \
     there is no description the field could be added to; what remains is cutting a value \
     somebody wrote, which is section 3.9's authorised loss and is not built for a field of a \
     structure";

/// Why a field holding a structure of its own is not described.
const FIELD_TYPE_NOT_DETERMINABLE: &str = "this packet states a structure field whose own value \
     is a structure, so the value type a pdfaField:valueType would have to name is a custom one \
     whose fields' types the file does not state - the extension schema container's \
     undeterminable case, one level down";

/// Why the value type a field belongs in could not be given its description.
const FIELD_NOT_WRITTEN: &str = "the value type this packet's structure field belongs to is not \
     described exactly once in this packet in a form a field can be added to - described twice, \
     described only in the catalog's packet, or written as a self-closing element - and \
     describing it anywhere else would leave the packet's own description contradicting it";

/// Why a packet whose fields were described is not written after all.
const FIELD_STILL_UNDESCRIBED: &str = "a structure field this conversion described is still \
     undescribed in the packet afterwards, so the packet is half-edited rather than corrected - \
     and a half-edited packet is not written at all, which is the rule the property removal \
     states";

/// Describes, in its own value type, every structure field a packet states and that type leaves
/// out.
///
/// **Nothing is judged here.** `pdf_archive::undescribed_fields` is the same reading the
/// requirement's own row is, asked of one packet's bytes, so a field this describes is exactly
/// one that row reported. The field's namespace, name and the value type its serialisation shows
/// are the packet's own; the description Table 6 requires is nowhere in the file, so it is
/// [`FIELD_DESCRIPTION`], and the report names every field described (`doc/adr/2050`) — except
/// where the operator's `field-descriptions` names the field, whose sentence is then theirs
/// (`supplied`, `doc/adr/2082`). A field this cannot describe without deciding something — one in
/// another namespace than its type's, one whose value is itself a structure, one whose type the
/// packet does not describe once — refuses the document by name.
pub(super) fn prepare_field_descriptions(
    document: &Document,
    catalog: Option<&Dictionary>,
    failing: &[ObjectId],
    edited: Edited<'_>,
    supplied: &BTreeMap<String, String>,
) -> Result<FieldsDescribed, Because> {
    let mut out = FieldsDescribed::default();
    // A stream the fresh packet replaces states no value type of the producer's, so no field in
    // it is described; where the requirement failed there, the replacement meets it, and the
    // decision is the replacement's own (`doc/adr/2455`, `doc/adr/2442`'s construction).
    if let Some(fresh) = edited.fresh {
        out.replaced.extend(
            failing
                .iter()
                .copied()
                .filter(|at| fresh.packet(*at).is_some()),
        );
    }
    for (at, bytes, inherited, undescribed) in fields_by_packet(document, catalog, edited)? {
        let inherited = inherited.as_deref();
        let mut rows = Vec::with_capacity(undescribed.len());
        for field in &undescribed {
            if field.namespace != field.type_namespace {
                return Err(Because::NotBuiltYet(FIELD_IN_ANOTHER_NAMESPACE));
            }
            let Some(value_type) = field.value_type else {
                return Err(Because::NotBuiltYet(FIELD_TYPE_NOT_DETERMINABLE));
            };
            let operator = supplied.get(&field_key(&field.type_namespace, &field.local));
            rows.push(DescribedField {
                type_name: field.type_name.clone(),
                namespace: field.type_namespace.clone(),
                name: field.local.clone(),
                value_type: value_type.to_owned(),
                description: operator
                    .map_or(FIELD_DESCRIPTION, String::as_str)
                    .to_owned(),
                supplied: operator.is_some(),
            });
        }
        let descriptions: Vec<xmp::FieldDescription<'_>> = rows
            .iter()
            .map(|row| xmp::FieldDescription {
                type_name: &row.type_name,
                type_namespace: &row.namespace,
                name: &row.name,
                value_type: &row.value_type,
                description: &row.description,
            })
            .collect();
        let written = xmp::describe_fields(&bytes, &descriptions)
            .map_err(|_| Because::NotBuiltYet(FIELD_NOT_WRITTEN))?;
        // Read back rather than trusted, which is `prepare_properties`'s rule.
        if !pdf_archive::undescribed_fields(&written, inherited).is_empty() {
            return Err(Because::NotBuiltYet(FIELD_STILL_UNDESCRIBED));
        }
        out.packets.insert(at, written);
        out.described.extend(rows);
    }
    if out.packets.is_empty() && out.replaced.is_empty() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    Ok(out)
}

/// One metadata stream, its packet as the editors before this one left it, the catalog's
/// packet it inherits descriptions from and the structure fields its value types leave out.
///
/// The catalog's own packet is read alone: inheriting it from itself would put its unedited
/// description of a type ahead of the one [`prepare_field_descriptions`] writes. A packet with
/// nothing undescribed is left out.
type FieldsOfOnePacket = (
    ObjectId,
    Vec<u8>,
    Option<Vec<u8>>,
    Vec<pdf_archive::UndescribedField>,
);

/// [`FieldsOfOnePacket`] for every metadata stream, in the order the validator reads them.
fn fields_by_packet(
    document: &Document,
    catalog: Option<&Dictionary>,
    edited: Edited<'_>,
) -> Result<Vec<FieldsOfOnePacket>, Because> {
    let catalog_at = catalog
        .and_then(|catalog| catalog.get("Metadata"))
        .and_then(Object::as_reference);
    let catalog_packet = catalog_at.and_then(|at| match document.get(at) {
        Object::Stream(stream) => edited
            .packet(at)
            .or_else(|| document.decoded_stream_data(&stream).map(|it| it.to_vec())),
        _ => None,
    });
    let mut out = Vec::new();
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
        let inherited = (catalog_at != Some(at))
            .then(|| catalog_packet.clone())
            .flatten();
        let undescribed = pdf_archive::undescribed_fields(&bytes, inherited.as_deref());
        if !undescribed.is_empty() {
            out.push((at, bytes, inherited, undescribed));
        }
    }
    Ok(out)
}

/// The structure fields the producer's own packets state and their value types leave out, read
/// before any editor runs: the population an operator's `field-descriptions` is matched against
/// for the report's supplied facts (`doc/adr/2082`). A document whose packets cannot be read
/// yields nothing, and [`prepare_field_descriptions`] then names the refusal.
pub(in crate::archive) fn undescribed_fields_in(
    document: &Document,
) -> Vec<pdf_archive::UndescribedField> {
    let catalog = document.catalog().ok();
    fields_by_packet(document, catalog.as_ref(), Edited::default())
        .map(|packets| {
            packets
                .into_iter()
                .flat_map(|(_, _, _, undescribed)| undescribed)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether a packet already states an extension schema container.
fn states_a_container(packet: &[u8]) -> bool {
    xmp::Xmp::parse(packet).is_ok_and(|read| {
        read.properties()
            .iter()
            .any(|(name, _)| name.namespace == xmp::EXTENSION && name.local == "schemas")
    })
}

/// Why a property the undeterminable case cut is gone, as the report says it.
const CUT_UNDETERMINABLE: &str = "its value is a structure, so the value type an extension \
     schema would have to name is a custom one whose fields' types the file does not state, and \
     the configuration's undeterminable = \"discard\" took it out rather than describe it";

/// The properties [`cut_the_undeterminable`] takes out, named as the report names a removal.
fn undeterminable_properties(
    undescribed: &[pdf_archive::UndescribedSchema],
) -> Vec<MisusedProperty> {
    let mut out = Vec::new();
    for schema in undescribed {
        for property in &schema.properties {
            if property.value_type.is_some() {
                continue;
            }
            out.push(MisusedProperty {
                name: XmpName {
                    namespace: schema.namespace.clone(),
                    local: property.local.clone(),
                },
                spelled: format!("{}:{}", schema.prefix, property.local),
                stated: "a structured value".to_owned(),
                because: CUT_UNDETERMINABLE.to_owned(),
            });
        }
    }
    out
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

/// One metadata stream whose packet this conversion replaced, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacedPacket {
    /// The metadata stream, in the source's numbering.
    pub at: ObjectId,
    /// Whether it is the one the catalog states, which is the document's own metadata.
    pub catalogs: bool,
    /// What the producer's packet broke, in the words the requirement's own row uses.
    pub because: String,
    /// Where no page could keep the producer's packet and its row's second step removed it, what
    /// was removed and where that is stated (`doc/adr/2678`); `None` where the packet was kept,
    /// or replaced under a row that keeps nothing.
    pub removed: Option<RemovedPacket>,
}

impl ReplacedPacket {
    /// One replaced packet as JSON.
    pub(in crate::archive) fn to_json(&self) -> Value {
        let mut fields = vec![
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
        ];
        if let Some(removed) = &self.removed {
            fields.push(("removed".to_owned(), removed.to_json()));
        }
        Value::Object(fields)
    }
}

/// A replaced packet whose bytes no appended page could lay out, removed by its row's second step
/// (`doc/adr/2678`).
///
/// The packet row's keep lays the producer's packet out as text, and ISO 16684-1 makes a packet
/// Unicode; a stream whose data, decoded through the filters its dictionary states, is not UTF-8
/// has no text to lay out. ISO 32000-2 §7.4 is why no other decoding is tried:
///
/// > The filter or filters for a stream shall be specified by the Filter entry in the stream's
/// > dictionary (or the FFilter entry if the stream is external).
///
/// so bytes that would read as a packet through a filter the stream does not state are not that
/// stream's data, and reading them so would be this converter guessing. Every field is computed
/// from the source, which is what lets the statement page name the loss in facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedPacket {
    /// The packet row whose second step removed it.
    pub site: &'static str,
    /// The metadata stream, in the source's numbering.
    pub at: ObjectId,
    /// The bytes between `stream` and `endstream`, as the file holds them.
    pub length: usize,
    /// The filters the stream's dictionary states, in order; empty where it states none.
    pub filters: Vec<String>,
    /// Why no page kept it.
    pub because: &'static str,
    /// The first page of the statement that lists it, zero-based, where one does.
    pub stated_on: Option<usize>,
}

/// Why a replaced packet's bytes were removed rather than laid out (`doc/adr/2678`).
pub(in crate::archive) const PACKET_NOT_TEXT: &str = "its stream data, decoded through the \
     filters its dictionary states, is not UTF-8 text, so no page can lay it out as a packet \
     (ISO 16684-1 makes a packet Unicode), and ISO 32000-2 section 7.4 lets no filter the \
     dictionary does not state be applied to it";

impl RemovedPacket {
    /// The packet's fate, in the words the report and `xmpMM:History` print.
    #[must_use]
    pub fn sentence(&self) -> String {
        let filters = if self.filters.is_empty() {
            "no filter".to_owned()
        } else {
            self.filters.join(" ")
        };
        let mut out = format!(
            "the XMP metadata packet object {} {} held ({} bytes of stream data under {filters}) \
             is removed (metadata-packet): {}",
            self.at.number, self.at.generation, self.length, self.because
        );
        if let Some(page) = self.stated_on {
            // Writing into a `String` cannot fail.
            let _ = write!(
                out,
                "; its removal is stated on page {}",
                page.saturating_add(1)
            );
        }
        out
    }

    /// The removal as JSON, the page one-based as every page the report names.
    pub(in crate::archive) fn to_json(&self) -> Value {
        let mut fields = vec![
            ("site".to_owned(), Value::text(self.site)),
            ("loss".to_owned(), Value::text("metadata-packet")),
            ("length".to_owned(), Value::count(self.length)),
            (
                "filters".to_owned(),
                Value::Array(self.filters.iter().map(Value::text).collect()),
            ),
            ("because".to_owned(), Value::text(self.because)),
        ];
        if let Some(page) = self.stated_on {
            fields.push(("stated-on".to_owned(), Value::count(page.saturating_add(1))));
        }
        Value::Object(fields)
    }
}

/// What one fault costs a reader, in the words the report prints.
fn why_a_packet_was_replaced(fault: pdf_archive::PacketFault) -> &'static str {
    match fault {
        pdf_archive::PacketFault::WillNotDecode => {
            "the stream's data would not decode, so there was no packet to read"
        }
        pdf_archive::PacketFault::WillNotParse => "the packet is not well-formed XML",
        pdf_archive::PacketFault::UndeclaredPrefix => {
            "the packet uses a namespace prefix no declaration binds, so it is not \
             namespace-well-formed XML"
        }
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
pub(super) fn prepare_fresh_packets(
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
            removed: None,
        });
    }
    if fresh.packets.is_empty() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    Ok(fresh)
}

/// The packets whose identification schema loses a malformed amendment identifier.
#[derive(Debug, Default)]
pub(in crate::archive) struct Amended {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(in crate::archive) packets: BTreeMap<ObjectId, Vec<u8>>,
    /// Every identifier removed, spelled and with the value that was there, for the report.
    pub(in crate::archive) removed: Vec<RemovedIdentifier>,
    /// The catalog's metadata stream, where section 6.6.2.1's replacement composes its packet
    /// afresh: a packet stating no property states no amendment identifier, so the replacement is
    /// what meets the requirement, and nothing is cut (`doc/adr/2455`).
    pub(in crate::archive) replaced: Option<ObjectId>,
}

impl Amended {
    /// Whether the packet the requirement failed in was replaced rather than edited, so that the
    /// replacement's decision is the row's (`doc/adr/2455`).
    pub(in crate::archive) fn only_replaced(&self) -> bool {
        self.packets.is_empty() && self.replaced.is_some()
    }
}

/// ISO 19005-2 section 6.6.4's form of an amendment identifier, the requirement
/// [`prepare_amendment`] answers.
pub(in crate::archive) const AMENDMENT_REQUIREMENT: &str = "metadata/identification-amendment-form";

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
    pub(in crate::archive) fn to_json(&self) -> Value {
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
pub(super) fn prepare_amendment(
    document: &Document,
    catalog: Option<&Dictionary>,
    edited: Edited<'_>,
) -> Result<Amended, Because> {
    let at = catalog
        .and_then(|catalog| catalog.get("Metadata"))
        .and_then(Object::as_reference)
        .ok_or(Because::NotBuiltYet(NOT_ASKED_FOR))?;
    // The requirement is read out of this one packet, so where the fresh packet replaces it the
    // replacement meets the requirement and the decision is the replacement's own (`doc/adr/2455`,
    // `doc/adr/2442`'s construction).
    if edited.fresh.is_some_and(|fresh| fresh.packet(at).is_some()) {
        return Ok(Amended {
            replaced: Some(at),
            ..Amended::default()
        });
    }
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
        replaced: None,
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
pub(super) fn prepare_respellings(
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
    let mut removed = Vec::new();
    let mut discarded = 0usize;
    for at in schema_packets(document, input, CONTAINER_REQUIREMENT) {
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
        // A field nothing in the file states cannot be respelled into being; the catalogue's
        // `discard` is prepared instead, and the decision asks whether it was authorised.
        let (bytes, faults) = if faults.iter().any(|fault| !fault.misspelled) {
            let (cut, went) = super::super::extension_container::discarded(&bytes)?;
            removed.extend(went);
            discarded = discarded.saturating_add(1);
            // A narrower cut keeps the container's complete descriptions, which may still spell
            // a field with another prefix: that is this writer's ordinary work on what is left
            // (`doc/adr/2131`).
            let left = pdf_archive::extension_container_fields(&cut);
            if left.is_empty() {
                packets.insert(at, cut);
                continue;
            }
            (cut, left)
        } else {
            (bytes, faults)
        };
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
    Ok(Respelled {
        packets,
        fields,
        removed,
        discarded,
    })
}

/// What the packet has to carry besides the producer's own bytes.
///
/// One argument rather than five, because every one of them is a fact about *this conversion*
/// that only [`Prepared::of`](super::Prepared::of) knows, and a function taking five booleans and two slices is one
/// nobody can call correctly twice.
#[derive(Debug, Clone, Copy)]
pub(super) struct Recording<'a> {
    /// Whether the identification schema is being restated into the packet.
    pub(super) schema: bool,
    /// The applied departures' `xmpMM:History` parameters, where the conversion departed.
    ///
    /// `doc/rfc/0007` section 4.7.3: every departure is recorded in the file's own history, so the
    /// archive carries the fact rather than relying on a report nobody kept. `None` for a
    /// conversion that departed from nothing.
    pub(super) departure: Option<&'a str>,
    /// What a `derive` remedy made, where anything was derived (`doc/questions/A55`).
    pub(super) derived: Option<&'a str>,
    /// What the operator stated that the document does not (`doc/rfc/0007` section 5b.1).
    pub(super) supplied: Option<&'a str>,
    /// What an operator's tool fetched from outside the document (`doc/adr/1209`).
    pub(super) resolved: Option<&'a str>,
    /// What the removed encryption asserted about the reader (`doc/adr/1187`).
    pub(super) protection: Option<&'a str>,
    /// What this viewer recovered of the page tree and the file now states (`doc/adr/2502`).
    pub(super) recovered: Option<&'a str>,
    /// What an appended page preserved, where one was appended (`doc/adr/1014` section 5).
    pub(super) preserved: Option<&'a str>,
    /// What became of each embedded file a requirement failed at, one clause per file
    /// (`doc/adr/2536`).
    pub(super) attachments: Option<&'a str>,
    /// What an attachment preserved, where anything was attached (`doc/adr/1270`).
    pub(super) attached: Option<&'a str>,
    /// What every removed transfer function other than the identity said (`doc/adr/1964`).
    pub(super) transfers: Option<&'a str>,
    /// Which images had `/Interpolate` written false, where any did (`doc/adr/2498`).
    pub(super) smoothing: Option<&'a str>,
    /// Which runs were written as the number this reader draws, where any were (`doc/adr/2638`).
    pub(super) as_drawn: Option<&'a str>,
    /// What the source's header was, where it stated no version (`doc/adr/2558`).
    pub(super) header: Option<&'a str>,
    /// Which TrueType fonts had their program's `cmap` restated, where any did (`doc/adr/2520`).
    pub(super) cmap: Option<&'a str>,
    /// How many parent tree entries were rebuilt, where any were (`doc/adr/2674`).
    pub(super) parent_tree: Option<&'a str>,
    /// Whether the identification schema's properties are deliberately omitted (`A59`).
    ///
    /// A departed conversion that does not claim conformance still writes the packet, so the
    /// catalog states a metadata stream, but with **no** `pdfaid:*` properties — so the file does
    /// not claim to be PDF/A. `schema` is then true (the packet is edited) and this is true (what
    /// is written is nothing), which strips any identification the source stated.
    pub(super) omit_identification: bool,
    /// The instant every recorded action carries, where a clock answered.
    pub(super) when: Option<&'a str>,
    /// What the `/DefaultCMYK` being written is, where one is: the `DeviceN` construction's
    /// parameters or the operator's press profile's (`doc/adr/2130`), one recorded action either
    /// way.
    pub(super) default_cmyk: Option<&'static str>,
    /// The removal's own recorded action, empty where nothing was removed.
    pub(super) removals: &'a str,
    /// The discarded containers' recorded action, empty where none was discarded.
    pub(super) discards: &'a str,
    /// The substitution's own recorded action, empty where no face was embedded.
    pub(super) substituted: &'a str,
    /// The properties §14.3.3's information dictionary adds to the packet, where it moves.
    ///
    /// Additive by construction: `pdf_model::xmp::supplement` writes only a property the packet
    /// does not already state, which is the one addition §14.3.4 permits.
    pub(super) supplements: &'a [xmp::Supplement<'a>],
    /// What the writers before this one left in each packet, one of which may be the catalog's.
    pub(super) edited: Edited<'a>,
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
pub(super) struct Edited<'a> {
    /// The packets this conversion replaced outright, because it could not edit them.
    ///
    /// **Before all three of the others, and disjoint from them**: a replaced stream is skipped
    /// by the header cut, the property removal and the container respelling, so at most one of
    /// the four has anything to say about any one stream.
    pub(super) fresh: Option<&'a Fresh>,
    /// The packets whose header attributes have been cut.
    pub(super) headers: Option<&'a Headers>,
    /// The packets a property removal has already edited.
    pub(super) cleaned: Option<&'a Cleaned>,
    /// The packets a container respelling has already edited.
    pub(super) respelled: Option<&'a Respelled>,
    /// The packet a malformed amendment identifier has already been cut out of.
    pub(super) amended: Option<&'a Amended>,
    /// The packets an extension schema container has already been written into.
    pub(super) described: Option<&'a SchemasDescribed>,
    /// The packets whose value types have already gained a description of their fields.
    pub(super) fields: Option<&'a FieldsDescribed>,
}

impl Edited<'_> {
    /// What one stream's packet holds after every edit made before the caller's.
    fn packet(self, at: ObjectId) -> Option<Vec<u8>> {
        self.fields
            .and_then(|fields| fields.packets.get(&at).cloned())
            .or_else(|| {
                self.described
                    .and_then(|described| described.packets.get(&at).cloned())
            })
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

/// The document's own packet as every writer before the caller left it, or as its producer wrote
/// it where none of them wrote.
///
/// The same two sources [`prepare_metadata`] starts from, and asked in the same order: a reader
/// of *what the packet already states* has to read the bytes this conversion is going to write,
/// not the ones the file holds.
pub(super) fn document_packet(
    document: &Document,
    catalog: Option<&Dictionary>,
    edited: Edited<'_>,
) -> Option<Vec<u8>> {
    let at = catalog?.get("Metadata").and_then(Object::as_reference)?;
    let Object::Stream(stream) = document.get(at) else {
        return None;
    };
    edited.packet(at).or_else(|| {
        document
            .decoded_stream_data(&stream)
            .map(|bytes| bytes.to_vec())
    })
}

/// The actions a conversion records in `xmpMM:History`, one per kind of thing it did that the
/// file's own provenance has to carry, all at the one instant `recording.when` states. The table
/// pairs each action with the parameters its remedy recorded, and an action whose remedy recorded
/// nothing is left out; a remedy that gains a history entry adds one row here and nothing else.
fn recorded_events<'a>(recording: &Recording<'a>) -> Vec<xmp::Event<'a>> {
    let Some(when) = recording.when else {
        return Vec::new();
    };
    let non_empty = |text: &'a str| (!text.is_empty()).then_some(text);
    let actions: [(&'a str, Option<&'a str>); 19] = [
        (DEFAULT_CMYK_ACTION, recording.default_cmyk),
        (REMOVED_PROPERTIES_ACTION, non_empty(recording.removals)),
        (REMOVED_PROPERTIES_ACTION, non_empty(recording.discards)),
        (SUBSTITUTED_FONTS_ACTION, non_empty(recording.substituted)),
        (DEPARTED_ACTION, recording.departure),
        (DERIVED_ACTION, recording.derived),
        (SUPPLIED_ACTION, recording.supplied),
        (RESOLVED_ACTION, recording.resolved),
        (DECRYPTED_ACTION, recording.protection),
        (RECOVERED_ACTION, recording.recovered),
        (ATTACHED_ACTION, recording.attached),
        (PRESERVED_ACTION, recording.preserved),
        (PRESERVED_ACTION, recording.attachments),
        (TRANSFER_REMOVED_ACTION, recording.transfers),
        (SMOOTHING_OFF_ACTION, recording.smoothing),
        (SMOOTHING_OFF_ACTION, recording.as_drawn),
        (HEADER_ACTION, recording.header),
        (CMAP_RESTATED_ACTION, recording.cmap),
        (PARENT_TREE_REBUILT_ACTION, recording.parent_tree),
    ];
    actions
        .into_iter()
        .filter_map(|(action, parameters)| {
            parameters.map(|parameters| xmp::Event {
                action,
                parameters,
                when,
            })
        })
        .collect()
}

/// The metadata stream this conversion writes, with every action it has to record in it.
///
/// ISO 19005-2 section 6.6.6 and ISO 19005-4 section 6.7.5 ask a converter to record what it did,
/// and `doc/pdf-a-conversion-limits.md` section 4.2 turns that into one entry per **Ask**. The
/// entries are appended in the order they are built, so a packet already holding a producer's
/// history keeps it and gains these after it.
pub(super) fn the_packet(
    target: Target,
    document: &Document,
    catalog: Option<&Dictionary>,
    spare: &mut Spare,
    recording: &Recording<'_>,
) -> Result<Metadata, Because> {
    let events = recorded_events(recording);
    // Nothing is prepared that no failed requirement asked for: a document needing neither the
    // schema nor an entry does not have its packet read at all, and the reason is never shown —
    // `decide` consults it only for a requirement the table answers with this rewrite.
    let Some(catalog) = catalog else {
        return Err(Because::NotBuiltYet(NO_CATALOG));
    };
    if !recording.schema && events.is_empty() && recording.supplements.is_empty() {
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
        recording.supplements,
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
    supplements: &[xmp::Supplement<'_>],
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
        let packet = xmp::supplement(&packet, supplements)
            .map_err(|_| Because::NotBuiltYet(PACKET_NOT_EDITABLE))?;
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

/// What §14.3.3's document information dictionary becomes, and what the packet gains from it.
///
/// ISO 19005-4 section 6.1.3 admits `/Info` only beside a `/PieceInfo`, and then holding only a
/// `/ModDate`. §14.3.3 says where the rest goes — the dictionary is deprecated "\[f]or any other
/// document level metadata" and Table 349's NOTEs name the XMP property each key pairs with — so
/// the two rows are answered by moving the values and dropping the dictionary. `doc/adr/1269`.
pub(in crate::archive) struct Information {
    /// The properties the document's packet gains, in Table 349's own key order.
    ///
    /// Additive: `pdf_model::xmp::supplement` writes only what the packet does not already state,
    /// which is the only addition §14.3.4 permits.
    pub(in crate::archive) supplements: Vec<Gained>,
    /// What became of each entry, for the report.
    pub(in crate::archive) entries: Vec<MovedEntry>,
    /// The `/ModDate` the output's own trailer still states, where the clause keeps it there.
    ///
    /// `None` is the ordinary case: the dictionary goes and `xmp:ModifyDate` carries the date.
    /// `Some` is ISO 19005-4 section 6.1.3's own carve-out — a catalog stating a `/PieceInfo`
    /// keeps the entry, because §14.5 makes it what a page-piece dictionary's `LastModified` is
    /// compared against.
    pub(in crate::archive) kept_modification_date: Option<Object>,
}

/// One property the packet gains from the dictionary.
///
/// Owned rather than an [`xmp::Supplement`], because a carried key's local name is spelled from
/// the key itself ([`carried_local_name`]) and so is no `'static` string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::archive) struct Gained {
    /// The namespace URI the property belongs to.
    namespace: &'static str,
    /// The prefix to spell it with.
    prefix: &'static str,
    /// Its local name.
    local: String,
    /// Its value.
    value: String,
    /// Which of ISO 16684-1's shapes to write it in.
    form: xmp::Form,
}

impl Gained {
    /// The property as the packet writer takes it.
    pub(in crate::archive) fn supplement(&self) -> xmp::Supplement<'_> {
        xmp::Supplement {
            namespace: self.namespace,
            prefix: self.prefix,
            local: &self.local,
            value: self.value.clone(),
            form: self.form,
        }
    }
}

/// The namespace a document information dictionary key Table 349 does not define is carried in.
///
/// **A choice, and stated as one** (`doc/adr/2680`). ISO 32000-2 §14.3.3 sends "any other
/// document level metadata" to a metadata stream and names no property for a key Table 349 does
/// not define, and ISO 19005-4 section 6.7.2 admits any well-formed XMP. This namespace is this
/// project's, and it defines one thing about each of its properties: that its value is the text
/// string the source's document information dictionary stated under the key the local name
/// spells ([`carried_local_name`]). It says nothing about what the key means, which no file
/// states — that is `doc/adr/1269` section 5's objection, and a namespace of provenance does not
/// meet it.
pub const CARRIED_INFORMATION_NAMESPACE: &str =
    "https://github.com/close2/pdf-viewer/ns/document-information/1.0/";

/// The prefix [`CARRIED_INFORMATION_NAMESPACE`] is spelled with.
pub const CARRIED_INFORMATION_PREFIX: &str = "docinfo";

/// The XMP local name a document information dictionary key is carried under, or `None` for the
/// empty key, which no XML name can spell.
///
/// ISO 16684-1 makes a property name an XML qualified name, and a §7.3.5 name may hold any byte
/// but null. So the spelling is an escape, and it is injective: an ASCII letter, digit, `-` or `.`
/// stands for itself, except that a first character that is not a letter is escaped; every other
/// byte of the name, `_` included, is written `_HH_` with its two upper-case hexadecimal digits.
/// `_` appears only as an escape's delimiter, so the key is recovered from the local name
/// byte for byte. `/WPS-ARTICLEDOI` is `docinfo:WPS-ARTICLEDOI`, `/Base#20Target` is
/// `docinfo:Base_20_Target`.
#[must_use]
pub fn carried_local_name(key: &[u8]) -> Option<String> {
    use std::fmt::Write as _;
    if key.is_empty() {
        return None;
    }
    let mut local = String::with_capacity(key.len());
    for (at, &byte) in key.iter().enumerate() {
        let plain = byte.is_ascii_alphabetic()
            || (at > 0 && (byte.is_ascii_digit() || byte == b'-' || byte == b'.'));
        if plain {
            local.push(char::from(byte));
        } else {
            // Writing into a `String` cannot fail.
            let _ = write!(local, "_{byte:02X}_");
        }
    }
    Some(local)
}

/// Whether XML 1.0 carries `text` as element content unchanged.
///
/// Its `Char` production excludes every C0 control but the three line characters, and U+FFFE and
/// U+FFFF,
/// so a packet holding one is not well-formed (ISO 19005-4 section 6.7.2.1); and its end-of-line
/// handling reads a carriage return as a line feed, so a value holding one would come back
/// changed. Either is a value this converter cannot carry as the dictionary stated it.
fn xml_carries(text: &str) -> bool {
    text.chars().all(|character| {
        matches!(character, '\t' | '\n')
            || (character >= ' ' && !matches!(character, '\u{FFFE}' | '\u{FFFF}'))
    })
}

/// One entry of the document information dictionary, and where it went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedEntry {
    /// The key, as Table 349 spells it.
    pub key: String,
    /// The XMP property it was written to, where it was written to one.
    pub property: Option<String>,
    /// What happened to it, in one clause for a person.
    pub fate: &'static str,
    /// Where `unmapped = "carry"` could not carry the value and the row's second step removed
    /// it, what was removed and where that is stated (`doc/adr/2681`); `None` everywhere else.
    pub removed: Option<RemovedInformation>,
}

impl MovedEntry {
    /// One entry as JSON.
    pub(in crate::archive) fn to_json(&self) -> Value {
        let mut fields = vec![
            ("key".to_owned(), Value::text(self.key.clone())),
            ("fate".to_owned(), Value::text(self.fate.to_owned())),
        ];
        if let Some(property) = &self.property {
            fields.push(("property".to_owned(), Value::text(property.clone())));
        }
        if let Some(removed) = &self.removed {
            fields.push(("removed".to_owned(), removed.to_json()));
        }
        Value::Object(fields)
    }
}

/// What becomes of a value `unmapped = "carry"` cannot carry as the dictionary stated it.
///
/// The row's second step where it names one, `doc/rfc/0014` section 3's mechanism at a row whose
/// outcome is one per entry (`doc/adr/2681`), and otherwise the row's `unreadable` answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::archive) enum NotCarried {
    /// Removed, and listed on the statement page under the row this names: `append-statement`.
    Stated(&'static str),
    /// Removed, and named in the report only: `discard`, as the second step or as `unreadable`.
    Dropped,
    /// The site keeps its refusal, [`INFORMATION_VALUE_NOT_CARRIED`].
    Refused,
}

/// A document information dictionary entry `unmapped = "carry"` could not carry, removed by its
/// row's second step (`doc/adr/2681`).
///
/// ISO 32000-2 §14.3.3 requires the value of a key Table 349 does not define to be a text string,
/// and ISO 19005-4 section 6.1.3 removes the dictionary, so a value that is not one has no form in
/// the packet and leaves with the dictionary. Every field is computed from the source — the key's
/// name, the value's type and size — and the value itself is never written: it is exactly what
/// has no form to be written in, which is what lets the statement page name the loss in facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedInformation {
    /// The information row whose second step removed it.
    pub site: &'static str,
    /// The key, written as ISO 32000-2 §7.3.5 writes a name, solidus and `#xx` escapes included.
    pub key: String,
    /// The value's type in this program's fixed words, with its size where it has one.
    pub value: String,
    /// The value's length in bytes, where it is a string.
    pub length: Option<usize>,
    /// Why it could not be carried, one of three fixed sentences.
    pub because: &'static str,
    /// The first page of the statement that lists it, zero-based, where one does.
    pub stated_on: Option<usize>,
}

impl RemovedInformation {
    /// The entry's fate, in the words the report and `xmpMM:History` print.
    #[must_use]
    pub fn sentence(&self) -> String {
        let mut out = format!(
            "the document information dictionary entry {} (value: {}) is removed \
             (document-information-entry): {}",
            self.key, self.value, self.because
        );
        if let Some(page) = self.stated_on {
            // Writing into a `String` cannot fail.
            let _ = write!(
                out,
                "; its removal is stated on page {}",
                page.saturating_add(1)
            );
        }
        out
    }

    /// The removal as JSON, the page one-based as every page the report names.
    pub(in crate::archive) fn to_json(&self) -> Value {
        let mut fields = vec![
            ("site".to_owned(), Value::text(self.site)),
            ("loss".to_owned(), Value::text("document-information-entry")),
            ("key".to_owned(), Value::text(self.key.clone())),
            ("value".to_owned(), Value::text(self.value.clone())),
        ];
        if let Some(length) = self.length {
            fields.push(("length".to_owned(), Value::count(length)));
        }
        fields.push(("because".to_owned(), Value::text(self.because)));
        if let Some(page) = self.stated_on {
            fields.push(("stated-on".to_owned(), Value::count(page.saturating_add(1))));
        }
        Value::Object(fields)
    }
}

/// Why a value is not carried: §14.3.3 asks a text string of it, and it is none.
pub(in crate::archive) const NOT_A_TEXT_STRING: &str = "ISO 32000-2 section 14.3.3 requires the \
     value of a key Table 349 does not define to be a text string, and this value is not one \
     (or its bytes contradict the encoding they claim), so the XMP packet has no form that \
     carries it as the dictionary stated it";

/// Why a value is not carried: XML 1.0 would not carry its text unchanged.
pub(in crate::archive) const NOT_CARRIED_BY_XML: &str = "the value is a text string holding a \
     character XML 1.0 does not carry unchanged (a control character it excludes, or a carriage \
     return its end-of-line handling reads as a line feed), so the XMP packet cannot carry it as \
     the dictionary stated it";

/// Why a value is not carried: its key is the empty name.
pub(in crate::archive) const NO_LOCAL_NAME: &str = "the key is the empty name, which no XML name \
     can spell, so the XMP packet has no property to carry its value in";

/// A value's type in the statement page's fixed words, with its size where it has one.
fn value_type(value: &Object) -> (String, Option<usize>) {
    match value {
        Object::Null => ("null".to_owned(), None),
        Object::Boolean(_) => ("boolean".to_owned(), None),
        Object::Integer(_) => ("integer".to_owned(), None),
        Object::Real(_) => ("real number".to_owned(), None),
        Object::String(bytes) => (
            format!("string of {} bytes", bytes.len()),
            Some(bytes.len()),
        ),
        Object::Name(_) => ("name".to_owned(), None),
        Object::Array(items) => (format!("array of {} elements", items.len()), None),
        Object::Dictionary(dict) => (format!("dictionary of {} entries", dict.len()), None),
        Object::Stream(_) => ("stream".to_owned(), None),
        Object::Reference(_) => ("indirect reference to no object".to_owned(), None),
    }
}

/// The entry's value is now the packet's, under the property Table 349's NOTE names.
const MOVED: &str = "written into the XMP packet";

/// The packet already stated the property, so §14.3.4 leaves what it says alone.
const ALREADY_STATED: &str = "the packet already states this property, which §14.3.4 leaves \
     unchanged, so the dictionary's value is dropped";

/// Table 349 names no counterpart, and the operator answered `unmapped = "carry"`.
const CARRIED: &str = "Table 349 names no XMP property for this key and the configuration answers \
     `unmapped = \"carry\"`, so the value is written into the XMP packet unchanged, under this \
     converter's namespace for a dictionary key, which states where the value came from and \
     nothing about what it means";

/// `unmapped = "carry"` could not carry the value, and the row's second step stated its removal.
const STATED_REMOVED: &str = "Table 349 names no XMP property for this key and the configuration \
     answers `unmapped = \"carry\"`, but the value has no form the packet carries as the \
     dictionary stated it, so the row's second step removes it and the statement page states the \
     removal";

/// Table 349 names no counterpart, and the operator answered `unmapped = "discard"`.
const DROPPED: &str = "Table 349 names no XMP property for this key and the configuration \
     answers `unmapped = \"discard\"`, so the value leaves the document";

/// The one entry the clause leaves in the dictionary.
const KEPT: &str = "kept in the dictionary, which ISO 19005-4 section 6.1.3 admits beside the \
     catalog's PieceInfo";

/// The two keys whose value §14.3.3 makes a date rather than a text string.
const DATE_KEYS: [&str; 2] = ["CreationDate", "ModDate"];

/// Why a `/Info` key with no XMP counterpart stops the conversion.
const UNMAPPED_KEY_DECLINED: &str = "this document information dictionary states a key ISO \
     32000-2 Table 349 does not define, and Table 349's NOTEs are what name every other key's XMP \
     counterpart — so there is no property this value belongs in. The configuration's `unmapped` \
     answer decides: `carry` writes the value into the packet under this converter's namespace \
     for a dictionary key, `discard` drops the value and the report names the key, and `stop`, \
     which is what a row saying nothing states, keeps this refusal";

/// Why a date this converter cannot read stops the conversion, where the value would move.
const INFORMATION_DATE_NOT_READABLE: &str = "this document information dictionary states a \
     CreationDate or ModDate that is not §7.9.4's date, so there is no instant to write into the \
     packet's own ISO 8601 spelling of one. Deciding what a malformed date meant is the repair \
     doc/questions/A48 closes, and writing the text across unconverted would put a value in \
     xmp:CreateDate that ISO 16684-1's grammar does not admit. The configuration's `unreadable` \
     answer decides: `discard` drops the value and the report names the key, and `stop`, which is \
     what a row saying nothing states, keeps this refusal";

/// Why a value that is not a string stops the conversion, where the value would move.
const INFORMATION_VALUE_NOT_TEXT: &str = "§14.3.3 requires every document information dictionary \
     entry other than CreationDate and ModDate to be a text string, and Table 349 makes Trapped a \
     name; this dictionary states one that is neither, so there is no text to carry into the \
     packet and this converter will not invent a spelling for it. The configuration's \
     `unreadable` answer decides: `discard` drops the value and the report names the key, and \
     `stop`, which is what a row saying nothing states, keeps this refusal";

/// Why a key the configuration asks to carry stops the conversion, where its value cannot be.
const INFORMATION_VALUE_NOT_CARRIED: &str = "§14.3.3 requires the value of a document \
     information dictionary key Table 349 does not define to be a text string, and the \
     configuration answers `unmapped = \"carry\"`; this dictionary states such a key whose value \
     is not a text string (or whose bytes contradict the encoding they claim), holds a character XML 1.0 does not carry unchanged (a control \
     character, or a carriage return its end-of-line handling reads as a line feed), or whose \
     name is empty, so there is no property to carry it in as the dictionary stated it. The \
     row's second step decides: `on-failure = \"append-statement\"` removes the value and states \
     the removal on an appended page (doc/adr/2681), `discard` there or as the `unreadable` answer \
     drops the value and the report names the key, and `stop`, which is what a row saying nothing \
     states, keeps this refusal";

/// Why a modification date the clause keeps beside a packet that states one stops the conversion.
///
/// No `unreadable` answer reaches it: beside a `/PieceInfo` the entry is not moved but kept, and
/// what binds it is §14.3.4's equivalence with the packet's `xmp:ModifyDate`, so a date with no
/// instant is a comparison this converter cannot make rather than a value it could drop.
const KEPT_DATE_NOT_READABLE: &str = "the catalog states a PieceInfo, so ISO 19005-4 section \
     6.1.3 keeps this document's ModDate in the dictionary and the conversion writes both \
     sources, and §14.3.4 requires the two to be fully equivalent where both are written — but \
     the packet's xmp:ModifyDate names an instant and this ModDate is not §7.9.4's date, so it \
     names none to compare";

/// The value is not of the form ISO 32000-2 states for its key, and the operator answered
/// `unreadable = "discard"`.
const UNREADABLE_DROPPED: &str = "the value is not of the form ISO 32000-2 states for this key \
     — a date that is not §7.9.4's, or neither a text string nor Trapped's name — so the property \
     Table 349 names cannot hold it, and the configuration answers `unreadable = \"discard\"`, so \
     the value leaves the document";

/// Which of §14.3.3's two forms a value failed, so that the caller can name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unreadable {
    /// A `/CreationDate` or `/ModDate` that is not §7.9.4's date.
    Date,
    /// A value that is neither a text string nor, for `/Trapped`, a name.
    NotText,
}

impl Unreadable {
    /// The refusal a value of this kind earns where it would move into the packet.
    const fn sentence(self) -> &'static str {
        match self {
            Self::Date => INFORMATION_DATE_NOT_READABLE,
            Self::NotText => INFORMATION_VALUE_NOT_TEXT,
        }
    }
}

/// Why a modification date the packet contradicts stops the conversion.
const MODIFICATION_DATES_DISAGREE: &str = "the catalog states a PieceInfo, so ISO 19005-4 \
     section 6.1.3 keeps this document's ModDate in the dictionary and the conversion writes both \
     sources — and §14.3.4 requires that \"the data in the document information dictionary and \
     the document level metadata stream -if both are written -are fully equivalent\". The \
     packet's xmp:ModifyDate names another instant than the dictionary's ModDate, and choosing \
     between them is deciding which of the producer's two statements was true";

/// Moves §14.3.3's document information dictionary into the XMP packet.
///
/// **Nothing is decided about a value.** Each entry is written under the property Table 349's own
/// NOTE names for its key, spelled the way ISO 16684-1 spells that property's shape; a date is
/// converted between the two grammars §7.9.4 and ISO 16684-1 define and nothing else changes. The
/// packet is supplemented rather than rewritten, so a property the producer already stated stands
/// — §14.3.4's instruction, made structural by `pdf_model::xmp::supplement` having no way to
/// overwrite.
pub(super) fn prepare_information(
    document: &Document,
    catalog: Option<&Dictionary>,
    packet: Option<&[u8]>,
    (unmapped, discard_unreadable, not_carried): (Unmapped, bool, NotCarried),
) -> Result<Information, Because> {
    let Some(catalog) = catalog else {
        return Err(Because::NotBuiltYet(NO_CATALOG));
    };
    // A trailer naming an object that does not exist states no dictionary, and §7.3.9 says what
    // that is: "[a]n indirect object reference (see 7.3.10, "Indirect objects") to a nonexistent
    // object shall be treated the same as a null object". So it moves what an empty dictionary
    // moves, which is nothing, and the key goes with it (`doc/adr/2358`).
    let info = match document.get_key(document.trailer(), "Info") {
        Object::Dictionary(info) => info,
        Object::Null => Dictionary::new(),
        _ => return Err(Because::NotBuiltYet(NOT_ASKED_FOR)),
    };
    if unmapped == Unmapped::ExtensionSchema {
        return Err(Because::NotBuiltYet(UNMAPPED_NEEDS_A_NAMESPACE));
    }
    // ISO 19005-4 section 6.1.3's carve-out, and §14.5 is its reason: a page-piece dictionary's
    // `LastModified` is compared against this date, so a catalog holding private data keeps it.
    let keeps = document.get_key(catalog, "PieceInfo").as_dict().is_some();
    let stated = packet.and_then(|bytes| xmp::Xmp::parse(bytes).ok());
    let mut supplements = Vec::new();
    let mut entries = Vec::new();
    let mut kept_modification_date = None;
    let keys: Vec<Vec<u8>> = info
        .iter()
        .map(|(name, _)| name.as_bytes().to_vec())
        .collect();
    for spelled in keys {
        let key = String::from_utf8_lossy(&spelled).into_owned();
        let value = document.get_key(&info, &key);
        if matches!(value, Object::Null) {
            continue;
        }
        let Some(&(_, namespace, prefix, local, form)) =
            TABLE_349.iter().find(|(name, ..)| *name == key)
        else {
            unmapped_entry(
                (key, &spelled, &value),
                (unmapped, not_carried),
                stated.as_ref(),
                (&mut entries, &mut supplements),
            )?;
            continue;
        };
        let property = format!("{prefix}:{local}");
        if key == "ModDate" && keeps {
            // Both sources are written, so §14.3.4's fourth rule binds where the packet states
            // the date too: they have to name the same instant, and this converter will not
            // choose between two the producer stated. Where it does not, the entry stays as its
            // producer wrote it and is never read, so its form decides nothing (`doc/adr/2358`).
            if let Some(already) = stated.as_ref().and_then(|xmp| xmp.text(namespace, local)) {
                match information_value(&value, &key) {
                    Ok(text) if text == already => {}
                    Ok(_) => return Err(Because::NotBuiltYet(MODIFICATION_DATES_DISAGREE)),
                    Err(_) => return Err(Because::NotBuiltYet(KEPT_DATE_NOT_READABLE)),
                }
            }
            kept_modification_date = Some(value.clone());
            entries.push(MovedEntry {
                removed: None,
                key,
                property: Some(property),
                fate: KEPT,
            });
            continue;
        }
        let text = match information_value(&value, &key) {
            Ok(text) => text,
            // The value has no home the property's form admits, so it is the operator's to
            // drop, as a key Table 349 names nothing for is (`doc/adr/2358`).
            Err(_) if discard_unreadable => {
                entries.push(MovedEntry {
                    removed: None,
                    key,
                    property: None,
                    fate: UNREADABLE_DROPPED,
                });
                continue;
            }
            Err(unreadable) => return Err(Because::Declined(unreadable.sentence())),
        };
        if stated
            .as_ref()
            .is_some_and(|xmp| xmp.value(namespace, local).is_some())
        {
            entries.push(MovedEntry {
                removed: None,
                key,
                property: Some(property),
                fate: ALREADY_STATED,
            });
            continue;
        }
        supplements.push(Gained {
            namespace,
            prefix,
            local: local.to_owned(),
            value: text,
            form,
        });
        entries.push(MovedEntry {
            removed: None,
            key,
            property: Some(property),
            fate: MOVED,
        });
    }
    // An empty dictionary, or one whose every entry is null and so absent (§7.3.9), moves
    // nothing into the packet: the trailer's key is all that goes, which is what the clause asks.
    Ok(Information {
        supplements,
        entries,
        kept_modification_date,
    })
}

/// A key Table 349 does not define, answered by the configuration's `unmapped` word.
///
/// `discard` drops it and names it; `carry` writes it under [`CARRIED_INFORMATION_NAMESPACE`],
/// or hands a value it cannot carry as stated to [`NotCarried`] — the row's second step, which
/// removes it and states it (`doc/adr/2681`), or `unreadable`'s answer; `stop` and the
/// recognised, unbuilt `extension-schema` refuse by name (`doc/adr/1269`, `doc/adr/2680`).
fn unmapped_entry(
    (key, spelled, value): (String, &[u8], &Object),
    (unmapped, not_carried): (Unmapped, NotCarried),
    stated: Option<&xmp::Xmp>,
    (entries, supplements): (&mut Vec<MovedEntry>, &mut Vec<Gained>),
) -> Result<(), Because> {
    match unmapped {
        Unmapped::Discard => {
            entries.push(MovedEntry {
                removed: None,
                key,
                property: None,
                fate: DROPPED,
            });
        }
        Unmapped::Carry => match carried(value, spelled, stated) {
            Ok(Some(gained)) => {
                entries.push(MovedEntry {
                    removed: None,
                    key,
                    property: Some(format!("{}:{}", gained.prefix, gained.local)),
                    fate: CARRIED,
                });
                supplements.push(gained);
            }
            Ok(None) => entries.push(MovedEntry {
                removed: None,
                property: carried_local_name(spelled)
                    .map(|local| format!("{CARRIED_INFORMATION_PREFIX}:{local}")),
                key,
                fate: ALREADY_STATED,
            }),
            Err(because) => match not_carried {
                NotCarried::Stated(site) => {
                    let (value, length) = value_type(value);
                    entries.push(MovedEntry {
                        key,
                        property: None,
                        fate: STATED_REMOVED,
                        removed: Some(RemovedInformation {
                            site,
                            key: format!("/{}", Name::new(spelled).escaped()),
                            value,
                            length,
                            because,
                            stated_on: None,
                        }),
                    });
                }
                NotCarried::Dropped => entries.push(MovedEntry {
                    removed: None,
                    key,
                    property: None,
                    fate: UNREADABLE_DROPPED,
                }),
                NotCarried::Refused => {
                    return Err(Because::Declined(INFORMATION_VALUE_NOT_CARRIED));
                }
            },
        },
        Unmapped::ExtensionSchema | Unmapped::Stop => {
            return Err(Because::Declined(UNMAPPED_KEY_DECLINED));
        }
    }
    Ok(())
}

/// A key Table 349 does not define, as the property that carries it.
///
/// `Ok(None)` where the packet already states the property, which §14.3.4 leaves as it is;
/// `Err`, with the reason in the statement page's words, where the value cannot be carried as the
/// dictionary stated it — not a §7.9.2.2 text
/// string, which §14.3.3 requires of it (a string whose bytes contradict their encoding, such as
/// a byte Table D.2 leaves undefined, is not one), or holding a character XML does not carry unchanged
/// ([`xml_carries`]) — or the key is empty and so has no local name.
fn carried(
    value: &Object,
    key: &[u8],
    stated: Option<&xmp::Xmp>,
) -> Result<Option<Gained>, &'static str> {
    let local = carried_local_name(key).ok_or(NO_LOCAL_NAME)?;
    let bytes = value.as_string().ok_or(NOT_A_TEXT_STRING)?;
    if !pdf_syntax::text_string::is_text_string(bytes) {
        return Err(NOT_A_TEXT_STRING);
    }
    let text = pdf_syntax::text_string::text_string(bytes);
    if !xml_carries(&text) {
        return Err(NOT_CARRIED_BY_XML);
    }
    if stated.is_some_and(|xmp| xmp.value(CARRIED_INFORMATION_NAMESPACE, &local).is_some()) {
        return Ok(None);
    }
    Ok(Some(Gained {
        namespace: CARRIED_INFORMATION_NAMESPACE,
        prefix: CARRIED_INFORMATION_PREFIX,
        local,
        value: text,
        form: xmp::Form::Simple,
    }))
}

/// One entry's value as the text the packet is to carry.
///
/// Three shapes, and each is Table 349's. A date becomes ISO 16684-1's spelling of the same
/// instant; `/Trapped` is a name, whose three values Table 349 fixes; everything else is a
/// §7.9.2.2 text string. Anything else is [`Unreadable`], and nothing guesses at a spelling.
///
/// **A date is read as a text string first.** §7.9.4 makes it "a text string value", and its
/// NOTE 1 says where the grammar applies: "\[t]he description above relates to the text string
/// value after appropriate processing" — so a date written in UTF-16BE or with §7.9.2.2's UTF-8
/// lead bytes is the same date as the one written in `PDFDocEncoding` (`doc/adr/2358`).
fn information_value(value: &Object, key: &str) -> Result<String, Unreadable> {
    if DATE_KEYS.contains(&key) {
        let text = value.as_string().ok_or(Unreadable::NotText)?;
        let spelled = pdf_syntax::Date::parse(&pdf_syntax::text_string::text_string(text))
            .ok_or(Unreadable::Date)?;
        return Ok(xmp::spelled_date(&spelled));
    }
    if let Some(name) = value.as_name() {
        return Ok(String::from_utf8_lossy(name.as_bytes()).into_owned());
    }
    let text = value.as_string().ok_or(Unreadable::NotText)?;
    Ok(pdf_syntax::text_string::text_string(text))
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
pub(in crate::archive) fn metadata_stream(from: &Dictionary, packet: &[u8]) -> Object {
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
pub(in crate::archive) struct Headers {
    /// The packet each metadata stream is to carry in place of the one it holds.
    pub(in crate::archive) packets: BTreeMap<ObjectId, Vec<u8>>,
}

impl Headers {
    /// The edited packet for one stream, where this preparation edited it.
    pub(in crate::archive) fn packet(&self, at: ObjectId) -> Option<&Vec<u8>> {
        self.packets.get(&at)
    }
}

/// Cuts the deprecated attributes out of every packet header the validator reported one in.
///
/// **Nothing is judged here.** Which streams state one is `pdf_archive`'s finding; what this
/// decides is only whether the cut can be made, and a header still holding an attribute
/// afterwards refuses the document rather than leaving it half-edited.
pub(super) fn prepare_headers(
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
pub(in crate::archive) fn without_deprecated_header_attributes(packet: &[u8]) -> Option<Vec<u8>> {
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
