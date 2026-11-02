//! The metadata family of the decision table: the rows whose requirement identifier begins
//! `metadata/`, the sentences those rows quote, and the extension-schema container a metadata
//! row's answer reads.
//!
//! Whichever of [`super::REMEDIES`], [`super::CONDITIONAL`], [`super::SHAPES`] and
//! [`super::REFUSED_BY_NAME`] this family has rows for reads them beside every other family's, and
//! the dispatch in [`super`] reads only those four tables, so where a row is written decides
//! nothing about its answer (`doc/adr/2634`).

use super::super::prepare::Prepared;
use super::super::rewrite::Rewrite;
use super::{Answer, Because, Conditional, Loss, Losses, Remedy};

/// This family's rows of [`super::REMEDIES`].
pub(super) const REMEDIES: &[Remedy] = &[
    // ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1: the catalog states a metadata
    // stream. Answered by the same rewrite as the schema itself, because a document with no packet
    // gets one that states the schema and a document with one keeps every other property in it.
    Remedy {
        requirement: "metadata/catalog-metadata-stream",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    // ISO 19005-2 section 6.6.4 and ISO 19005-4 section 6.7.3: the identification schema, which
    // is the file's own claim to be PDF/A. `doc/pdf-a-conversion-limits.md` section 4.2 calls it
    // **Mechanical**, "and the one place the converter states a claim about its own output" — the
    // claim being safe because `doc/adr/0947`'s third stage holds the output to the target again
    // before the file is written.
    Remedy {
        requirement: "metadata/identification-schema-prefix",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-part-number",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-conformance-level",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-declares-level-a",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-part-number-four",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-revision-year",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-states-no-flavour",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-declares-flavour-e",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    Remedy {
        requirement: "metadata/identification-declares-flavour-f",
        answer: Answer::Mechanical(Rewrite::IdentificationSchema),
    },
    // ISO 19005-2 section 6.6.2.3.1, and the only route of the three
    // `doc/pdf-a-conversion-limits.md` section 3.9 leaves open — which is why it is a loss rather
    // than a default.
    Remedy {
        requirement: "metadata/properties-use-known-schemas",
        answer: Answer::Loses(Loss::MetadataProperty, Rewrite::PropertyOutsideItsSchema),
    },
    // ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1, and the third route
    // `doc/pdf-a-mitigations.md` section 9 found: write a fresh conforming packet. It is a loss
    // rather than a mechanical rewrite because a packet this tree cannot read is a packet whose
    // spans cannot be edited, so what the producer recorded in it does not cross — and
    // `doc/questions/A48` closes the other route, which would be deciding what a malformed
    // packet meant. `remedy = "preserve"` with `original = "page"` keeps the producer's own
    // bytes on an appended page (`doc/adr/1245`); authorising the loss alone does not.
    //
    // The data-model row is part 2's alone, which is the requirement's own `Applies` rather than
    // anything here: part 4 asks a packet to be well-formed and part 2 asks it to conform.
    Remedy {
        requirement: "metadata/xmp-packets-well-formed",
        answer: Answer::Loses(Loss::MetadataPacket, Rewrite::FreshMetadataPacket),
    },
    Remedy {
        requirement: "metadata/xmp-packets-state-one-rdf-element",
        answer: Answer::Loses(Loss::MetadataPacket, Rewrite::FreshMetadataPacket),
    },
    Remedy {
        requirement: "metadata/xmp-packets-meet-the-xmp-data-model",
        answer: Answer::Loses(Loss::MetadataPacket, Rewrite::FreshMetadataPacket),
    },
    // ISO 19005-2 section 6.6.4, whose amendment and corrigendum identifiers are optional and
    // whose form is the number and the year separated by a colon. Removal is the one remedy the
    // subclause leaves — neither half of a malformed value is recoverable — and it is a loss
    // rather than a mechanical rewrite because the producer's claim goes with it (`doc/adr/1246`).
    // ISO 19005-2 section 6.6.2.3.2, which requires every extension schema a stream uses to be
    // described in that stream or in the catalog's, with section 6.6.2.3.3's container schema.
    // `doc/pdf-a-conversion-limits.md` section 4.2 makes describing them the default and calls it
    // authoring in a small way; the alternative is deleting the producer's properties, which is
    // worse. **`Stated` rather than `Mechanical`** because three of the container's required
    // fields are nowhere in any file, so what goes in them is this converter's sentence rather
    // than the producer's — and an operator has to be told that (`doc/adr/1245`).
    Remedy {
        requirement: "metadata/extension-schemas-embedded",
        answer: Answer::Stated(
            None,
            Rewrite::ExtensionSchemaDescribed,
            CONTAINER_DESCRIBES_SHAPE,
        ),
    },
    // ISO 19005-2 section 6.6.2.3.1, the half that reaches inside a structure: a field the
    // packet states and its own value type leaves out is described in that type. **`Stated` for
    // the container's reason, one level down** — the description Table 6 requires is nowhere in
    // the file, so what goes in it is this converter's sentence (`doc/adr/2050`).
    Remedy {
        requirement: "metadata/extension-schema-structure-fields-are-described",
        answer: Answer::Stated(
            None,
            Rewrite::ExtensionFieldDescribed,
            TYPE_DESCRIBES_FIELD_SHAPE,
        ),
    },
    Remedy {
        requirement: "metadata/identification-amendment-form",
        answer: Answer::Loses(
            Loss::AmendmentIdentifier,
            Rewrite::AmendmentIdentifierRemoved,
        ),
    },
    // ISO 19005-2 section 6.6.2.3.3's four tables, each naming the prefix its fields are to be
    // spelled with — and section 6.6.2.2 is why that is a requirement rather than a convention: a
    // prefix means nothing *except* where one is identified as required, and these four identify
    // one. **Mechanical because the only thing that moves is a prefix token.** A field stated in
    // the field namespace its table gives it, with its local name and its value the producer's,
    // and spelled with another prefix, is a file that already says what the clause asks for; the
    // packet is respelled in place and means afterwards exactly what it meant before.
    //
    // A field the packet does not state at all is *not* this rewrite, and `super::super::prepare` refuses
    // the document by name rather than half-correcting it: a schema's name, a property's
    // description and a value's category are nowhere in the file, so writing one would be this
    // converter inventing metadata about metadata. `doc/pdf-a-mitigations.md`'s entry leaves that
    // half to an operator's own `supply` or to a `discard` that loses the container.
    Remedy {
        requirement: "metadata/extension-schema-container-fields",
        answer: Answer::Mechanical(Rewrite::ExtensionSchemaPrefixes),
    },
    // ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1.
    Remedy {
        requirement: "metadata/xmp-packet-header-attributes",
        answer: Answer::Mechanical(Rewrite::PacketHeaderAttributes),
    },
];

/// This family's rows of [`super::CONDITIONAL`].
pub(super) const CONDITIONAL: &[(&str, Conditional)] = &[(
    "metadata/extension-schema-container-fields",
    Conditional::PerDocument(Losses::one(Loss::MetadataProperty)),
)];

/// This family's rows of [`super::REFUSED_BY_NAME`].
pub(super) const REFUSED_BY_NAME: &[(&str, Because)] = &[
    // ISO 19005-4 section 6.7.5's history entries.
    (
        "metadata/provenance-recorded-action-fields-four",
        Because::TheFence(HISTORY_IS_WHAT_HAPPENED),
    ),
    (
        "metadata/xmp-character-data-only-in-simple-values",
        Because::NotBuiltYet(XMP_STRAY_CHARACTER_DATA_NOT_REMOVED),
    ),
];

/// What an operator is told when this conversion describes a field its producer's value type left
/// out.
const TYPE_DESCRIBES_FIELD_SHAPE: &str = "a structure field this file states and its \
     own extension schema value type leaves out is described in that value type by this \
     conversion. The field's namespace and name are the packet's own and its value type is the \
     form the packet's own serialisation shows - but ISO 19005-2 section 6.6.2.3.3's Table 6 \
     requires a description no file states for a field its producer did not describe, and what \
     this conversion put in it is one fixed sentence saying so. The report names every field \
     described";

/// What an operator is told when this conversion describes a schema its producer did not.
const CONTAINER_DESCRIBES_SHAPE: &str = "an extension schema this file uses and \
     describes nowhere is described by a container this conversion wrote. The namespace, the \
     prefix and each property's name are the packet's own, and each value type is the form the \
     packet's own serialisation shows - but ISO 19005-2 section 6.6.2.3.3 requires three fields \
     no file states, the schema's name and every property's category and description, and what \
     this conversion put in them is one fixed sentence saying so. The archive therefore carries \
     a description of the shape of the producer's metadata and no statement of what it means; \
     the report names every schema described";

/// Why a producer's incomplete XMP history entry is not completed.
const HISTORY_IS_WHAT_HAPPENED: &str = "ISO 19005-4 section 6.7.5 requires every action recorded \
     in the XMP history to state what was done and when. An entry missing either records \
     something this converter did not witness, so filling it in would be writing down a \
     provenance nobody has — doc/questions/A48's line, and the opposite of what an audit trail \
     is for. This converter adds its own history entries for what it does \
     (doc/pdf-a-conversion-limits.md section 4.2); it does not complete a producer's";

/// Why character data outside a simple value is left where it is.
///
/// The census ratchet catches this (ADR 0956) — a requirement `pdf-archive` learns to check is a
/// requirement this verb owes an answer to, which is the ratchet's whole purpose.
const XMP_STRAY_CHARACTER_DATA_NOT_REMOVED: &str = "this packet writes non-white character data \
     somewhere the XMP standard's serialisation admits none — that standard allows white space \
     anywhere the RDF syntax does and confines everything else to the element content of a leaf \
     representing a simple value, so what is here expresses no XMP value at all. Cutting it is \
     the same span surgery doc/pdf-a-conversion-limits.md section 3.9 already describes for a \
     property, but the remover in pdf_model::xmp takes a property name and this text belongs to \
     no property — it needs a second entry point keyed on the span the reader already records. \
     Whether taking it out is mechanical or an authorised loss is a question this row does not \
     settle: it expresses nothing a reader of the packet can use, and it is still bytes somebody \
     wrote, which is section 3.9's shape exactly";

/// What a container missing a field the file states nowhere costs: section 3.9's loss.
pub(super) const CONTAINER_DISCARDED: Answer =
    Answer::Loses(Loss::MetadataProperty, Rewrite::ExtensionContainerDiscarded);

/// Whether the preparation discarded an extension schema container rather than respelling one.
pub(super) fn discards_a_container(prepared: &Prepared) -> bool {
    prepared
        .respelled
        .as_ref()
        .is_ok_and(|respelled| respelled.discarded > 0)
}
