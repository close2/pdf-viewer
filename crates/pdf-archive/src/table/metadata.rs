//! Metadata: ISO 19005-2 section 6.6 and ISO 19005-4 section 6.7.
//!
//! The subclause that carries the file's own claim to be PDF/A, and it is the most consequential
//! row in this crate: everything else here decides whether a document *is* what it says, and the
//! identification schema is where it says it. Both parts close the subclause by warning that the
//! properties do not settle conformance on their own — [`crate::report`] is built on that
//! sentence — so what these rows judge is the claim, never the conformance.
//!
//! # Three things the two parts disagree about, and one printing error
//!
//! - - **The schema's namespace URI is printed differently.** ISO 19005-2 section 6.6.4 gives it
//!   with an `http` scheme and ISO 19005-4 section 6.7.3 with an `https` one. Nothing else about
//!   the identifier moves, and a producer writing either has named the schema the standard defines,
//!   so both are accepted for both parts — [`IDENTIFICATION_URIS`].
//! - **Part 4 adds `pdfaid:rev` and takes the conformance level away from every file but two.**
//!   Part 2's level is A, B or U and every conforming file states one; part 4's is E or F and
//!   only a PDF/A-4e or PDF/A-4f file may state anything at all.
//! - **Part 4's Table 2 spells the conformance property with a `pdfa` prefix**, in a schema whose
//!   required prefix the same table gives as `pdfaid` and whose namespace is the identification
//!   one. A `pdfa` prefix is bound to nothing anywhere in that part, so reading the property as
//!   belonging to any other namespace would make the table name a schema it never defines; these
//!   rows therefore look it up in the identification namespace, which is the reading that leaves
//!   the table self-consistent. That is a departure from the letter of one cell and it is written
//!   down here rather than buried in a predicate.
//! - Part 2 builds its own extension-schema machinery (section 6.6.2.3, five tables of it); part 4
//!   replaces the whole of it with one sentence pointing at ISO 16684-2, a standard this project
//!   does not hold. So section 6.6.2.3's three rows are implemented and section 6.7.2.3's is not,
//!   and the reasons are entirely different reasons.
//!
//! # section 6.6.2.3, and the standard this crate had to go and read
//!
//! Section 6.6.2.3.1 requires every property to use a predefined schema or an extension schema, and
//! *using* a schema means carrying the value type it gives the property. Deciding that needs the
//! XMP Specification's own tables, which ISO 19005-2 lists in its bibliography with a public
//! address and which are read here into [`PREDEFINED`]. The three rows divide the subclause like
//! this, and the division is stated because the clauses overlap:
//!
//! - **section 6.6.2.3.1** judges the *predefined* half — the shape of a value, the spelling of its
//!   scalar, and the language qualifiers a language alternative's items are defined to carry.
//! - - **section 6.6.2.3.2** judges the other half, a namespace no predefined schema owns and no
//!   embedded extension schema describes, because section 6.6.2.3.1 states that half by deferring
//!   to section 6.6.2.3.2.
//! - - **section 6.6.2.3.3** judges the description itself: every field of its four tables present,
//!   each spelled with the prefix its table requires.
//!
//! # section 6.6.6 and section 6.7.5, one subclause each, and only one of them a check
//!
//! The provenance subclauses ask that each high-level action be recorded in `xmpMM:History` and
//! then *require* fields of each action that is recorded — three in part 2, two in part 4, which
//! demotes `parameters` to a recommendation.
//!
//! **Part 2's is outside validation.** PDF Association `TechNote 0010`, which this tree now holds
//! and which `crate::clarification` reads, records at item A021 the ISO working group resolving
//! that parts 2 and 3 are to be read as if requirements on `xmpMM:History` were requirements on
//! the writing application and thus irrelevant to ISO 19005 validation. That is why veraPDF
//! checks neither subclause and why its corpus names every witness a pass; until session 941 this
//! crate had the resolution only at second hand, through a test fixture's outline and a
//! conference summary that contradicted it, and principle 5 rightly refused both. The row stays,
//! reports nothing, and says why — `doc/adr/0931`.
//!
//! **Part 4's stands on part 4's own text**, published in 2020 with `action` and `when` still
//! required, three years after a note in the technical note said the proposal had been accepted
//! in principle for the part then being drafted. A published standard outranks a note about an
//! intention.
//!
//! # section 6.6.5 and section 6.7.4, which are not unchecked for the reason they used to say
//!
//! The file-identifier rule compares one revision of a file against the previous one, and the
//! previous one is *in the file*: ISO 32000-2 §7.5.6's incremental update appends, so an updated
//! document still carries its earlier cross-reference sections, trailers and `/ID` arrays.
//! `pdf_syntax::xref::sections` names each of those sections' own trailers, so the earlier `/ID`
//! arrays are reachable and the old reason — that this crate is handed a document with no earlier
//! state — was wrong. What is still out of reach is the earlier revision's *packet*, which is what
//! the clause's condition needs; `doc/adr/0929` has the reading and what would close it.
//!
//! # The clause that binds a reader, and the one that binds nobody
//!
//! **A clause whose only requirement is addressed to a processor used to get no row here**, on the
//! reasoning that this crate judges a file. The reasoning was right and the conclusion was wrong —
//! a requirement named only in a module header is invisible in a verdict — so ISO 19005-2 section
//! 6.6.3 is now a [`Check::Processor`] row: its one requirement is that a conforming reader ignore
//! the document information dictionary, the file is expressly permitted to carry one, and the
//! consistency of its values with the metadata stream is a recommendation. Part 4 states the whole
//! subject in section 6.1.3 instead, which is the file-structure tranche's row.
//!
//! **ISO 19005-2 section 6.6.2.2 and ISO 19005-4 section 6.7.2.2 still have no row, and that is
//! different**: their namespace-prefix table is recommended rather than required, so there is no
//! requirement to carry. What those subclauses *do* state is the sentence that makes a prefix
//! binding wherever one is identified as required — which is what
//! `metadata/identification-schema-prefix` rests on.

use pdf_model::xmp::{Detail, Property as XmpProperty, Value, Xmp};
use pdf_syntax::{Document, Object, ObjectId, Stream};

use crate::Examination;
use crate::finding::{Findings, Where};
use crate::requirement::{Applies, Check, Clauses, Requirement};
use crate::target::{Flavour, Level, Target};

/// The rows this module contributes, which `super::TRANCHES` concatenates.
pub(super) static REQUIREMENTS: &[Requirement] = &[
    Requirement {
        id: "metadata/catalog-metadata-stream",
        asks: "The document catalog shall state a Metadata key whose value is a metadata stream \
               as ISO 32000-2 §14.3.2 defines one.",
        clauses: Clauses::both("6.6.2.1", "6.7.2.1"),
        applies: Applies::Always,
        check: Check::Implemented(catalog_metadata_stream),
    },
    Requirement {
        id: "metadata/xmp-packets-well-formed",
        asks: "Every metadata stream in the file shall carry an XMP packet that parses.",
        clauses: Clauses::both("6.6.2.1", "6.7.2.1"),
        applies: Applies::Always,
        check: Check::Implemented(xmp_packets_well_formed),
    },
    Requirement {
        id: "metadata/xmp-packets-meet-the-xmp-serialisation",
        asks: "Every XMP packet shall meet the XMP standard's own serialisation rules, which are \
               more than being well-formed XML — its encoding among them.",
        clauses: Clauses::both("6.6.2.1", "6.7.2.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "the other half of the sentence the row above answers, and the half that needs ISO \
             16684-1, which this project does not hold: what `well-formed as XMP defines it` adds \
             to well-formed XML cannot be read from anything here. The visible consequence is an \
             encoding — `pdf_model::xmp` decodes a UTF-16 or UTF-32 packet on purpose, which is a \
             reader being lenient with a real file and not a finding that the packet is one XMP \
             admits",
        ),
    },
    Requirement {
        id: "metadata/xmp-packet-header-attributes",
        asks: "No XMP packet header shall state the bytes or encoding attributes, both of which \
               the XMP standard deprecates.",
        clauses: Clauses::both("6.6.2.1", "6.7.2.1"),
        applies: Applies::Always,
        check: Check::Implemented(xmp_packet_header_attributes),
    },
    Requirement {
        id: "metadata/properties-use-known-schemas",
        asks: "Every property an XMP packet states in a predefined schema shall be used as that \
               schema defines it — the value type it is given, and the language qualifiers a \
               language alternative's items carry.",
        clauses: Clauses::only_two("6.6.2.3.1"),
        applies: Applies::Always,
        check: Check::Implemented(properties_use_known_schemas),
    },
    Requirement {
        id: "metadata/extension-schemas-embedded",
        asks: "Every extension schema a metadata stream uses shall be described inside that \
               stream or inside the catalog's, using the extension schema container schema.",
        clauses: Clauses::only_two("6.6.2.3.2"),
        applies: Applies::Always,
        check: Check::Implemented(extension_schemas_embedded),
    },
    Requirement {
        id: "metadata/extension-schema-container-fields",
        asks: "An extension schema container schema shall state every field of the four value \
               types the subclause tabulates, each spelled with the prefix its table requires.",
        clauses: Clauses::only_two("6.6.2.3.3"),
        applies: Applies::Always,
        check: Check::Implemented(extension_schema_container_fields),
    },
    Requirement {
        id: "metadata/schema-associated-file",
        asks: "Where a metadata stream names an associated file describing its schema, that \
               file's data shall conform to ISO 16684-2.",
        clauses: Clauses::only_four("6.7.2.3"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "checking it means validating an XMP schema description against ISO 16684-2, which \
             is not a standard this project holds",
        ),
    },
    Requirement {
        id: "metadata/identification-schema-prefix",
        asks: "Every property of the identification schema shall be spelled with the prefix the \
               subclause makes required for it, pdfaid.",
        clauses: Clauses::both("6.6.4", "6.7.3"),
        applies: Applies::Always,
        check: Check::Implemented(identification_schema_prefix),
    },
    Requirement {
        id: "metadata/document-information-dictionary-ignored",
        asks: "A conforming reader shall ignore a document information dictionary the file states.",
        clauses: Clauses::only_two("6.6.3"),
        applies: Applies::Always,
        check: Check::Processor(
            "the subclause's one requirement is what a reader does with the dictionary, and the \
             file is expressly permitted to carry it; the consistency of its values with the \
             metadata stream is the same subclause's recommendation, not a rule",
        ),
    },
    Requirement {
        id: "metadata/identification-part-number",
        asks: "The identification schema shall state a part number of 2.",
        clauses: Clauses::only_two("6.6.4"),
        applies: Applies::Always,
        check: Check::Implemented(identification_part_two),
    },
    Requirement {
        id: "metadata/identification-conformance-level",
        asks: "The identification schema shall state a conformance level of A, B or U.",
        clauses: Clauses::only_two("6.6.4"),
        applies: Applies::Always,
        check: Check::Implemented(identification_conformance_level),
    },
    Requirement {
        id: "metadata/identification-declares-level-a",
        asks: "A Level A file shall state A as its conformance level.",
        clauses: Clauses::only_two("6.6.4"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Implemented(identification_declares_level_a),
    },
    Requirement {
        id: "metadata/identification-amendment-and-corrigendum",
        asks: "Where the file conforms to a version of the part defined by an amendment or a \
               corrigendum, the identification schema shall state its number and year.",
        clauses: Clauses::only_two("6.6.4"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "the *presence* half alone, which is what is left after the row below: whether a \
             file was written against an amendment or a corrigendum is not a fact the file \
             states anywhere but in these two properties, so an absent entry and a correct one \
             look the same from here. The value of one that is stated is checked",
        ),
    },
    Requirement {
        id: "metadata/identification-amendment-form",
        asks: "An amendment or corrigendum identifier the identification schema does state shall \
               be the number and the year, separated by a colon.",
        clauses: Clauses::only_two("6.6.4"),
        applies: Applies::Always,
        check: Check::Implemented(identification_amendment_form),
    },
    Requirement {
        id: "metadata/identification-part-number-four",
        asks: "The identification schema shall state a part number of 4.",
        clauses: Clauses::only_four("6.7.3"),
        applies: Applies::Always,
        check: Check::Implemented(identification_part_four),
    },
    Requirement {
        id: "metadata/identification-revision-year",
        asks: "The identification schema shall state, as its revision, the four-digit publication \
               year of the revision of the part the file conforms to — 2020 for this one.",
        clauses: Clauses::only_four("6.7.3"),
        applies: Applies::Always,
        check: Check::Implemented(identification_revision_year),
    },
    Requirement {
        id: "metadata/identification-states-no-flavour",
        asks: "A file that is neither PDF/A-4e nor PDF/A-4f shall state no conformance property \
               at all.",
        clauses: Clauses::only_four("6.7.3"),
        applies: Applies::Flavours(&[Flavour::Plain]),
        check: Check::Implemented(identification_states_no_flavour),
    },
    Requirement {
        id: "metadata/identification-declares-flavour-e",
        asks: "A PDF/A-4e file shall state E as its conformance property.",
        clauses: Clauses::only_four("6.7.3"),
        applies: Applies::Flavours(&[Flavour::E]),
        check: Check::Implemented(identification_declares_flavour_e),
    },
    Requirement {
        id: "metadata/identification-declares-flavour-f",
        asks: "A PDF/A-4f file shall state F as its conformance property.",
        clauses: Clauses::only_four("6.7.3"),
        applies: Applies::Flavours(&[Flavour::F]),
        check: Check::Implemented(identification_declares_flavour_f),
    },
    Requirement {
        id: "metadata/file-identifier-changes-with-history",
        asks: "A file that gains an XMP history entry shall change the second half of the \
               trailer's file identifier.",
        clauses: Clauses::both("6.6.5", "6.7.4"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "the packet of the previous revision, and not the trailer of it. A file that has \
             been incrementally updated (ISO 32000-2 §7.5.6) contains its previous revisions, \
             and `pdf_syntax::xref::sections` now names each section's own trailer — so the \
             earlier `/ID` arrays are reachable and the reason this row used to give, that this \
             crate is handed a single document with no earlier state, was wrong. What is still \
             out of reach is the clause's *condition*: whether an `xmpMM:History` entry was \
             **added** is a comparison of two revisions' metadata packets, and resolving the \
             earlier one's `/Metadata` needs an object resolved as of an earlier section, which \
             `pdf_syntax` does not offer. A check firing on the trailers alone would report a \
             revision that added no history entry, which section 6.6.5 does not bind at all \
             (`doc/adr/0929`)",
        ),
    },
    Requirement {
        id: "metadata/provenance-recorded-action-fields",
        asks: "Every action recorded in the XMP history shall state its action, its parameters \
               and its time.",
        clauses: Clauses::only_two("6.6.6"),
        applies: Applies::Always,
        check: Check::OutsideValidation(
            "the sentence is in the published subclause and no erratum has withdrawn it, which \
             is why this row is carried and cited rather than dropped. What is not carried is a \
             check: the ISO working group's resolution named beside this row puts every \
             requirement on the xmpMM:History property outside ISO 19005 validation for parts 2 \
             and 3, and that reaches all three fields rather than only the parameters one that \
             raised the item. ISO 19005-4 section 6.7.5 is a different case and stays a check — \
             it was published three years afterwards and states its two fields on its own text",
        ),
    },
    Requirement {
        id: "metadata/provenance-recorded-action-fields-four",
        asks: "Every action recorded in the XMP history shall state its action and its time — \
               part 4 demotes the parameters field part 2 requires to a recommendation.",
        clauses: Clauses::only_four("6.7.5"),
        applies: Applies::Always,
        check: Check::Implemented(provenance_recorded_action_fields_four),
    },
];

/// The two spellings the two parts print for the identification schema's namespace.
///
/// ISO 19005-2 section 6.6.4 gives it with an `http` scheme and ISO 19005-4 section 6.7.3 with an
/// `https` one. A producer that writes either has named the schema each part defines, and a
/// validator that insisted on one spelling would refuse files over the standard's own
/// inconsistency.
const IDENTIFICATION_URIS: [&str; 2] = [
    "http://www.aiim.org/pdfa/ns/id/",
    "https://www.aiim.org/pdfa/ns/id/",
];

/// ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1.
///
/// Both parts point at the same table of the base standard for what a metadata stream *is*, and
/// ISO 32000-2 §14.3.2's Table 347 makes `/Type` and `/Subtype` required entries of it — so a
/// dictionary short of either has not stated the thing the clause names.
fn catalog_metadata_stream(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Ok(catalog) = document.catalog() else {
        findings.record(
            Where::file().named("Metadata"),
            "the document has no readable catalog, so it states no metadata stream",
        );
        return;
    };
    let object = document.get_key(&catalog, "Metadata");
    let Some(stream) = object.as_stream() else {
        findings.record(
            Where::file().named("Metadata"),
            if object.is_null() {
                "the catalog states no Metadata key".to_owned()
            } else {
                format!(
                    "the catalog's Metadata is a {} rather than a stream",
                    object.type_name()
                )
            },
        );
        return;
    };
    for (key, expected) in [("Type", "Metadata"), ("Subtype", "XML")] {
        let stated = document.get_key(&stream.dict, key);
        if stated
            .as_name()
            .is_none_or(|name| name.as_bytes() != expected.as_bytes())
        {
            findings.record(
                Where::file().named(key),
                format!("the catalog's metadata stream does not state /{key} /{expected}"),
            );
        }
    }
}

/// Every stream the file marks as a metadata stream, reached through the cross-reference table.
///
/// The population is `/Type /Metadata`, which ISO 32000-2 §14.3.2 Table 347 makes a required
/// entry of one: a stream that omits it has not declared itself a metadata stream, and the
/// catalog's own is checked for that entry by [`catalog_metadata_stream`] instead. Bounded by the
/// cross-reference table for the reason `file_structure.rs`'s walk is — both parts exempt an
/// indirect object no cross-reference section names.
fn for_each_metadata_stream(exam: &Examination<'_>, mut visit: impl FnMut(ObjectId, &Stream)) {
    let document = exam.document;
    for (id, object) in exam.objects() {
        let id = *id;
        if let Object::Stream(stream) = object
            && document
                .get_key(&stream.dict, "Type")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Metadata")
        {
            visit(id, stream);
        }
    }
}

/// ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1.
///
/// Part 2 asks for conformance with the XMP specification and well-formedness under XML 1.0 and
/// RDF/XML; part 4 asks for well-formedness under ISO 16684-1. What `pdf_model::xmp` can answer
/// is the well-formedness, which is the load-bearing half of both.
///
/// **A stream this reader could not get to is recorded too, and the message says which it was.**
/// A packet past `pdf_model::xmp`'s decompression bound or behind a filter that would not decode
/// has not been shown to be well-formed, and reporting the requirement as met over it would be
/// the silent-omission failure `doc/questions/Q20` exists to prevent. The finding's own sentence
/// is what tells a reader that the cause is this validator's bound rather than the document's XML.
fn xmp_packets_well_formed(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_metadata_stream(exam, |id, stream| {
        let Some(bytes) = document.decoded_stream_data(stream) else {
            findings.record(
                Where::object(id),
                "a metadata stream would not decode, so its packet could not be read",
            );
            return;
        };
        if let Err(error) = Xmp::parse(&bytes) {
            findings.record(
                Where::object(id),
                format!("a metadata stream's packet could not be read: {error}"),
            );
        }
    });
}

/// How much of a metadata stream is scanned for the packet header.
///
/// The header is the first processing instruction of the packet and every producer writes it
/// within a line or two; this bounds the copy [`packet_header`] makes rather than expressing a
/// reading of any clause.
const HEADER_SCAN: usize = 4096;

/// The XMP packet header a stream opens with, as ASCII, or `None` where it states none.
///
/// **The NUL bytes are dropped rather than the text decoded**, and that is the whole trick: ISO
/// 16684-1 lets a packet be UTF-8, UTF-16 or UTF-32, and an ASCII header in any of the three is
/// the same bytes once the padding NULs are gone. `pdf_model::xmp` decodes all three but hands
/// back resolved properties rather than the header, so this scan is what can see the processing
/// instruction at all.
///
/// A packet that states no header has no attributes to state, which is why `None` is not a
/// failure at the call site.
fn packet_header(data: &[u8]) -> Option<Vec<u8>> {
    let head: Vec<u8> = data
        .iter()
        .copied()
        .take(HEADER_SCAN)
        .filter(|byte| *byte != 0)
        .collect();
    let start = find(&head, b"<?xpacket")?;
    let rest = head.get(start..)?;
    let end = find(rest, b"?>")?;
    rest.get(..end).map(<[u8]>::to_vec)
}

/// The offset of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Whether a header states an attribute of this name.
///
/// The name has to be preceded by white space and followed by an equals sign, so that a value
/// which happens to contain the word does not read as the attribute itself.
fn states_attribute(header: &[u8], name: &[u8]) -> bool {
    header.windows(name.len()).enumerate().any(|(at, window)| {
        window == name
            && at > 0
            && header
                .get(at.saturating_sub(1))
                .is_some_and(u8::is_ascii_whitespace)
            && header
                .get(at.saturating_add(name.len())..)
                .unwrap_or_default()
                .iter()
                .copied()
                .find(|byte| !byte.is_ascii_whitespace())
                == Some(b'=')
    })
}

/// ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1.
fn xmp_packet_header_attributes(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_metadata_stream(exam, |id, stream| {
        let Some(bytes) = document.decoded_stream_data(stream) else {
            return;
        };
        let Some(header) = packet_header(&bytes) else {
            return;
        };
        for name in [&b"bytes"[..], b"encoding"] {
            if states_attribute(&header, name) {
                findings.record(
                    Where::object(id).named(String::from_utf8_lossy(name).into_owned()),
                    "an XMP packet header states an attribute the XMP standard deprecates",
                );
            }
        }
    });
}

/// The catalog's XMP packet, where the file states one this tree can read.
fn document_packet(document: &Document) -> Option<Xmp> {
    Xmp::document(document)?.ok()
}

/// One property of the identification schema, under either spelling of its namespace URI.
fn identification<'a>(packet: &'a Xmp, local: &str) -> Option<&'a Value> {
    packet
        .properties()
        .iter()
        .find(|(name, _)| {
            name.local == local && IDENTIFICATION_URIS.contains(&name.namespace.as_str())
        })
        .map(|(_, value)| value)
}

/// A simple property's text, which is the only shape the identification schema's four properties
/// take: its table gives each of them a scalar value type, so an array or a structure there is a
/// producer stating something other than the schema.
fn as_text(value: &Value) -> Option<&str> {
    match value {
        Value::Text(text) => Some(text.as_str()),
        _ => None,
    }
}

/// A value cut down to something a finding can print.
///
/// `pdf_model::xmp` admits a property value of up to a megabyte, and a report is prose for a
/// person; the cut is on a character boundary so that the message stays text.
fn short(text: &str) -> String {
    const LIMIT: usize = 48;
    let trimmed = text.trim();
    match trimmed.char_indices().nth(LIMIT) {
        Some((at, _)) => format!("{}…", trimmed.get(..at).unwrap_or_default()),
        None => trimmed.to_owned(),
    }
}

/// What the identification schema says about one property, in the three shapes a check cares
/// about.
enum Stated<'a> {
    /// The property is absent, and so is the packet or the schema it would be in.
    Absent,
    /// The property is present but is not the simple value its table gives it.
    NotSimple,
    /// The property states this text.
    Text(&'a str),
}

/// Reads one identification property out of the document's own packet.
fn stated<'a>(packet: Option<&'a Xmp>, local: &str) -> Stated<'a> {
    let Some(packet) = packet else {
        return Stated::Absent;
    };
    match identification(packet, local) {
        None => Stated::Absent,
        Some(value) => match as_text(value) {
            Some(text) => Stated::Text(text),
            None => Stated::NotSimple,
        },
    }
}

/// The shared body of the two part-number rows: a claim to a numbered part of ISO 19005.
fn identification_part(document: &Document, findings: &mut Findings, part: &str) {
    let packet = document_packet(document);
    let place = || Where::file().named("pdfaid:part");
    match stated(packet.as_ref(), "part") {
        Stated::Text(text) if text.trim() == part => {}
        Stated::Text(text) => findings.record(
            place(),
            format!(
                "the file claims part {} rather than part {part}",
                short(text)
            ),
        ),
        Stated::NotSimple => findings.record(
            place(),
            "the identification schema's part number is not a simple value",
        ),
        Stated::Absent => findings.record(
            place(),
            "the file states no PDF/A identification part number",
        ),
    }
}

/// The prefix ISO 19005-2 section 6.6.4 and ISO 19005-4 section 6.7.3 each identify as required for
/// the identification schema.
const IDENTIFICATION_PREFIX: &str = "pdfaid";

/// ISO 19005-2 section 6.6.4, ISO 19005-4 section 6.7.3.
///
/// **A prefix is usually meaningless and here it is not**, which is the whole of this row. Both
/// parts say in their namespaces-and-prefixes subclause (section 6.6.2.2, section 6.7.2.2) that no
/// significance attaches to a prefix *except where a specific prefix is identified as required*,
/// and both then identify one for this schema in as many words. So a packet that binds the
/// identification namespace to any other prefix has broken a sentence the standard went out of its
/// way to make binding.
///
/// The population is every property in the identification namespace, under either spelling of its
/// URI: [`IDENTIFICATION_URIS`] takes the `http` scheme part 2 prints and the `https` scheme part 4
/// does, and a producer writing either has named the schema its part defines.
///
/// **Part 4's own Table 2 breaks this rule**, spelling the conformance property `pdfa:conformance`
/// in a schema whose required prefix the same table gives as `pdfaid`. Erratum #123 settles it as
/// an error the working group agreed requires fixing, so the corrected table is what this row
/// judges and a file copying the printed one is reported.
fn identification_schema_prefix(exam: &Examination<'_>, findings: &mut Findings) {
    let Some(properties) = catalog_detail(exam.document) else {
        return;
    };
    for property in properties {
        if !IDENTIFICATION_URIS.contains(&property.name.namespace.as_str())
            || property.prefix == IDENTIFICATION_PREFIX
        {
            continue;
        }
        findings.record(
            Where::file().named(format!("{}:{}", property.prefix, property.name.local)),
            format!(
                "an identification schema property is spelled with the prefix {} where the \
                 subclause requires {IDENTIFICATION_PREFIX}",
                short(&property.prefix)
            ),
        );
    }
}

/// The target a document's own identification schema claims, where it claims one this crate owns.
///
/// Both parts require a conforming file to say which part it conforms to — ISO 19005-2 section
/// 6.6.4, ISO 19005-4 section 6.7.3 — and to say the level or the flavour beside it, so a file's
/// own packet names one of this crate's six targets. That is what lets `document_level.rs` hold an
/// *embedded* file to ISO 19005 without guessing which part to hold it to.
///
/// `None` is every case where the file does not name one this crate can judge: no packet, no part
/// number, or a part that is not 2 or 4 (`doc/questions/A17`: part 1 never, part 3 not bought).
///
/// Where the part is named and the conformance property is not, the *weakest* reading of it is
/// taken — Level B for part 2, the plain profile for part 4 — because those are what each part says
/// an unqualified claim means: section 6.6.4 gives A, B and U and section 6.7.3 reserves E and F
/// for the two annexes, so a part 4 file stating nothing is claiming the plain profile exactly.
pub(super) fn declared_target(document: &Document) -> Option<Target> {
    let packet = document_packet(document)?;
    let part = as_text(identification(&packet, "part")?)?.trim();
    let conformance = identification(&packet, "conformance")
        .and_then(as_text)
        .map(str::trim);
    match part {
        "2" => Some(Target::Two(match conformance {
            Some("A") => Level::A,
            Some("U") => Level::U,
            _ => Level::B,
        })),
        "4" => Some(Target::Four(match conformance {
            Some("E") => Flavour::E,
            Some("F") => Flavour::F,
            _ => Flavour::Plain,
        })),
        _ => None,
    }
}

/// ISO 19005-2 section 6.6.4.
fn identification_part_two(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    identification_part(document, findings, "2");
}

/// ISO 19005-4 section 6.7.3.
fn identification_part_four(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    identification_part(document, findings, "4");
}

/// ISO 19005-2 section 6.6.4.
///
/// The level a *particular* file has to state is decided by clause 5 and therefore by the target,
/// which a predicate is not given — so this row checks the claim's shape, and
/// [`identification_declares_level_a`] is the one level whose exact value a row can bind to,
/// because `Applies::FromLevel(Level::A)` on a part-2-only clause reaches Level A and nothing
/// else.
fn identification_conformance_level(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let packet = document_packet(document);
    let place = || Where::file().named("pdfaid:conformance");
    match stated(packet.as_ref(), "conformance") {
        Stated::Text(text) if matches!(text.trim(), "A" | "B" | "U") => {}
        Stated::Text(text) => findings.record(
            place(),
            format!(
                "the file claims a conformance level of {} rather than A, B or U",
                short(text)
            ),
        ),
        Stated::NotSimple => findings.record(
            place(),
            "the identification schema's conformance level is not a simple value",
        ),
        Stated::Absent => findings.record(place(), "the file states no PDF/A conformance level"),
    }
}

/// ISO 19005-2 section 6.6.4, for the one level of part 2 a row can bind to exactly.
fn identification_declares_level_a(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let packet = document_packet(document);
    if !matches!(stated(packet.as_ref(), "conformance"), Stated::Text(text) if text.trim() == "A") {
        findings.record(
            Where::file().named("pdfaid:conformance"),
            "the file does not claim Level A conformance",
        );
    }
}

/// The publication year of the only revision of ISO 19005-4 there is.
///
/// Section 6.7.3 asks for "the four digit year of that revision" and Table 2 for the year of
/// publication or revision, neither of which names a number on its own — the number comes from the
/// part's own date, ISO 19005-4:2020. Erratum #253 is what makes it exact rather than inferred: the
/// working group reworded the requirement to the four-digit publication year of that specific
/// revision and reconfirmed that it is 2020 for the current one. A later dated revision moves this
/// constant, which is why it is a constant.
const REVISION_YEAR: &str = "2020";

/// ISO 19005-4 section 6.7.3, as erratum #253 rewords it.
fn identification_revision_year(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let packet = document_packet(document);
    let place = || Where::file().named("pdfaid:rev");
    match stated(packet.as_ref(), "rev") {
        Stated::Text(text) if text.trim() == REVISION_YEAR => {}
        Stated::Text(text) => findings.record(
            place(),
            format!(
                "the revision {} is not {REVISION_YEAR}, the publication year of the only \
                 revision of this part",
                short(text)
            ),
        ),
        Stated::NotSimple => findings.record(
            place(),
            "the identification schema's revision is not a simple value",
        ),
        Stated::Absent => findings.record(place(), "the file states no PDF/A revision year"),
    }
}

/// ISO 19005-4 section 6.7.3, for a file held to the plain profile.
///
/// The clause reserves the conformance property for the two annexes, so a file that states one is
/// claiming to be PDF/A-4e or PDF/A-4f rather than the plain profile this target holds it to.
fn identification_states_no_flavour(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let packet = document_packet(document);
    let place = || Where::file().named("conformance");
    match stated(packet.as_ref(), "conformance") {
        Stated::Absent => {}
        Stated::Text(text) => findings.record(
            place(),
            format!(
                "the file claims a conformance property of {}, which is reserved for PDF/A-4e and \
                 PDF/A-4f",
                short(text)
            ),
        ),
        Stated::NotSimple => findings.record(
            place(),
            "the file states a conformance property, which is reserved for PDF/A-4e and PDF/A-4f",
        ),
    }
}

/// The shared body of the two flavour rows: Annex A's F and Annex B's E, both stated through
/// ISO 19005-4 section 6.7.3.
fn identification_flavour(document: &Document, findings: &mut Findings, flavour: &str) {
    let packet = document_packet(document);
    let place = || Where::file().named("conformance");
    match stated(packet.as_ref(), "conformance") {
        Stated::Text(text) if text.trim() == flavour => {}
        Stated::Text(text) => findings.record(
            place(),
            format!(
                "the file claims a conformance property of {} rather than {flavour}",
                short(text)
            ),
        ),
        Stated::NotSimple => findings.record(
            place(),
            "the identification schema's conformance property is not a simple value",
        ),
        Stated::Absent => findings.record(
            place(),
            format!("the file states no conformance property, so it does not claim {flavour}"),
        ),
    }
}

/// ISO 19005-4 section 6.7.3, which Annex B §B.5 requires a PDF/A-4e file to satisfy.
fn identification_declares_flavour_e(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    identification_flavour(document, findings, "E");
}

/// ISO 19005-4 section 6.7.3, which Annex A section A.3 requires a PDF/A-4f file to satisfy.
fn identification_declares_flavour_f(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    identification_flavour(document, findings, "F");
}

/// ISO 19005-2 section 6.6.2.3.3's five namespaces: the container schema and its four value types.
const EXTENSION_URI: &str = "http://www.aiim.org/pdfa/ns/extension/";
/// The Schema value type's field namespace, ISO 19005-2 section 6.6.2.3.3 Table 3.
const SCHEMA_URI: &str = "http://www.aiim.org/pdfa/ns/schema#";
/// The Property value type's field namespace, Table 4.
const PROPERTY_URI: &str = "http://www.aiim.org/pdfa/ns/property#";
/// The `ValueType` value type’s field namespace, Table 5.
const TYPE_URI: &str = "http://www.aiim.org/pdfa/ns/type#";
/// The Field value type's field namespace, Table 6.
const FIELD_URI: &str = "http://www.aiim.org/pdfa/ns/field#";

/// The shape a value takes, in the five forms [`Detail`] distinguishes.
///
/// The XMP Specification's value types collapse onto these: `bag Text` and `bag ProperName`
/// differ in what an item means and not in what a reader can see, so both are
/// [`Shape::Unordered`], and what separates them is [`Lexical`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// A simple property: one string.
    Simple,
    /// A language alternative — an `rdf:Alt` whose every item states an `xml:lang`.
    Language,
    /// An `rdf:Alt` of something other than text, where no item is required to name a language.
    Alternative,
    /// `rdf:Seq`.
    Ordered,
    /// `rdf:Bag`.
    Unordered,
    /// A structure.
    Structure,
}

impl Shape {
    /// Whether a stated value has this shape.
    ///
    /// A language alternative and an alternative array are the same `rdf:Alt` to a reader; what
    /// separates them is the requirement on the items, which [`language_alternative`] checks.
    fn accepts(self, stated: &Detail) -> bool {
        matches!(
            (self, stated),
            (Self::Simple, Detail::Text(_))
                | (Self::Language | Self::Alternative, Detail::Alt(_))
                | (Self::Ordered, Detail::Seq(_))
                | (Self::Unordered, Detail::Bag(_))
                | (Self::Structure, Detail::Structure(_))
        )
    }

    /// What to call this shape in a finding.
    const fn describe(self) -> &'static str {
        match self {
            Self::Simple => "a simple value",
            Self::Language => "a language alternative",
            Self::Alternative => "an alternative array",
            Self::Ordered => "an ordered array",
            Self::Unordered => "an unordered array",
            Self::Structure => "a structure",
        }
    }
}

/// The scalar value type the XMP Specification gives a property, and the form a validator holds
/// it to.
///
/// Only the six types it defines by their spelling are here. `Text`, `URI`, `AgentName`,
/// `ProperName`, `MIMEType` and the rest are Unicode strings with no form to check, and a
/// vocabulary — the specification's `Choice` — is a string too, so a value drawn from one is
/// held to the type its members have and not to the list, which this reader does not carry.
///
/// # `TechNote 0010` A020, which decides how much of a type is a validator's business
///
/// The working group resolved that parts 1 to 3 are read as if an XMP value were validated on its
/// type alone, with any further meaning inferred from a property's name or description
/// disregarded, and it listed the rule for each basic type. Two of those bear on this enum:
///
/// - **`Rational` admits any string.** It is listed beside `Text`, `URI` and the rest rather than
///   beside `Integer` and `Real`, so a validator may not hold `exif:XResolution` to a quotient —
///   [`Lexical::accepts`] returns true for it, and the variant stays because the table's business
///   is to record what the schema defines.
/// - **`Date` is ISO 8601.** [`date`] implements the six profiles the XMP Specification lists,
///   which are profiles *of* ISO 8601 — so nothing accepted here is outside A020's rule, while a
///   date written in some other ISO 8601 form would be reported. Narrowing that would mean
///   implementing a standard this project does not hold; `doc/questions/Q53` carries it.
///
/// `MimeType` is A020's third named form, RFC 2046, and no property is judged against it: `Any`
/// is what `dc:format` and its like carry, which under-reports rather than misreports.
/// `GPSCoordinate` is the type A020's list does not mention at all, and [`Lexical::Coordinate`]
/// keeps the specification's form for it — a resolution that says nothing about a type withdraws
/// nothing, and `doc/questions/Q53` asks whether that is the reading to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lexical {
    /// A string, with nothing to check.
    Any,
    /// `True` or `False`, spelled exactly so.
    Boolean,
    /// A decimal integer with an optional sign.
    Integer,
    /// A decimal number with an optional sign and at most one point.
    Real,
    /// The specification's `Rational`, written as a quotient — and admitting any string, because
    /// `TechNote 0010` A020 puts it among the types a validator judges as text.
    Rational,
    /// One of the six ISO 8601 profiles the specification lists.
    Date,
    /// `DDD,MM,SSk` or `DDD,MM.mmk`, where `k` names a direction.
    Coordinate,
}

impl Lexical {
    /// Whether a value is written in this form.
    ///
    /// A value that states nothing at all is accepted whatever the type: an empty element says
    /// the property is present and says no value, which is a different complaint from a value in
    /// the wrong form and is not this row's.
    fn accepts(self, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return true;
        }
        match self {
            // `Rational` sits here rather than beside `Integer` and `Real` because `TechNote
            // 0010` A020 lists it among the types validated as any string: the quotient this
            // crate used to require is a rule the working group withdrew.
            Self::Any | Self::Rational => true,
            Self::Boolean => matches!(text, "True" | "False"),
            Self::Integer => integer(text),
            Self::Real => real(text),
            Self::Date => date(text),
            Self::Coordinate => coordinate(text),
        }
    }

    /// What to call this form in a finding.
    const fn describe(self) -> &'static str {
        match self {
            Self::Any => "text",
            Self::Boolean => "True or False",
            Self::Integer => "an integer",
            Self::Real => "a real number",
            // Unreachable while A020 stands, because nothing fails a `Rational`; kept so that the
            // match stays a total description of the enum rather than a partial one.
            Self::Rational => "a rational",
            Self::Date => "an ISO 8601 date",
            Self::Coordinate => "a GPS coordinate",
        }
    }
}

/// A decimal integer with an optional sign, which is the specification's `Integer`.
fn integer(text: &str) -> bool {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

/// A decimal number with an optional sign and at most one point, the specification's `Real`.
fn real(text: &str) -> bool {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (whole, fraction) = match digits.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (digits, ""),
    };
    let decimal = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    !(whole.is_empty() && fraction.is_empty()) && decimal(whole) && decimal(fraction)
}

/// One of the six date profiles the specification lists, from a bare year to a fractional second
/// with a time zone designator.
fn date(text: &str) -> bool {
    let digits = |part: &str, count: usize| {
        part.len() == count && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    let (calendar, clock) = match text.split_once('T') {
        Some((calendar, clock)) => (calendar, Some(clock)),
        None => (text, None),
    };
    let mut parts = calendar.split('-');
    let dated = matches!(parts.next(), Some(year) if digits(year, 4))
        && parts.clone().all(|part| digits(part, 2))
        && parts.count() <= 2;
    if !dated {
        return false;
    }
    let Some(clock) = clock else {
        // A date with no time is one of the three shorter profiles, and the day is not optional
        // once the time is there: `YYYY-MM-DDThh:mm` is the shortest profile that states one.
        return true;
    };
    let (time, zone) = match clock.rfind(['Z', '+', '-']) {
        Some(at) => (clock.get(..at).unwrap_or_default(), clock.get(at..)),
        None => (clock, None),
    };
    let zoned = match zone {
        Some("Z") => true,
        Some(offset) => {
            let hours_minutes = offset.get(1..).unwrap_or_default();
            matches!(hours_minutes.split_once(':'), Some((hours, minutes)) if digits(hours, 2) && digits(minutes, 2))
        }
        None => false,
    };
    if !zoned {
        return false;
    }
    let mut fields = time.split(':');
    let (Some(hours), Some(minutes)) = (fields.next(), fields.next()) else {
        return false;
    };
    if !digits(hours, 2) || !digits(minutes, 2) {
        return false;
    }
    match fields.next() {
        None => fields.next().is_none(),
        Some(seconds) => {
            let (whole, fraction) = match seconds.split_once('.') {
                Some((whole, fraction)) => (whole, Some(fraction)),
                None => (seconds, None),
            };
            digits(whole, 2)
                && fraction.is_none_or(|fraction| {
                    !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
                })
                && fields.next().is_none()
        }
    }
}

/// `DDD,MM,SSk` or `DDD,MM.mmk`, the specification's `GPSCoordinate`.
fn coordinate(text: &str) -> bool {
    let Some(figures) = text.strip_suffix(['N', 'S', 'E', 'W']) else {
        return false;
    };
    let Some((degrees, rest)) = figures.split_once(',') else {
        return false;
    };
    let unsigned = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    if !unsigned(degrees) {
        return false;
    }
    match rest.split_once([',', '.']) {
        Some((minutes, seconds)) => unsigned(minutes) && unsigned(seconds),
        None => unsigned(rest),
    }
}

/// One schema the XMP Specification predefines, and the properties it gives a value type.
struct Schema {
    /// The namespace URI, which is the schema's name.
    uri: &'static str,
    /// What to call it in a finding.
    name: &'static str,
    /// The prefix the specification prefers for it, for a finding that has to spell a property.
    prefix: &'static str,
    /// The properties, sorted by local name, each with the shape and lexical form of its value.
    properties: &'static [(&'static str, Shape, Lexical)],
}

impl Schema {
    /// What this schema says about one property, where it defines it.
    fn property(&self, local: &str) -> Option<(Shape, Lexical)> {
        self.properties
            .iter()
            .find(|(name, _, _)| *name == local)
            .map(|&(_, shape, lexical)| (shape, lexical))
    }
}

/// The schemas the XMP Specification predefines, with the value type it gives each property.
///
/// # Where this comes from, and why it is here at all
///
/// ISO 19005-2 section 6.6.2.3.1 requires a property to *use* a predefined schema, and a schema is
/// only usable as defined: `xmpDM:logComment` is a text property, so a file that states a
/// sequence there has borrowed the namespace rather than used the schema. Deciding that needs
/// the specification's own tables, which are not in `doc/pdfa/` — ISO 19005-2 lists the XMP
/// Specification (Adobe Systems, September 2005) in its bibliography with a public address, and
/// this table is that document's clause 4 read property by property. Like `doc/pdfa/`'s two
/// files it is licensed to a reader rather than a republisher, so nothing here is quoted: every
/// row is a value type reduced to what a reader can check.
///
/// # What a row does and does not say
///
/// [`Shape`] is the value's form and [`Lexical`] is the spelling of its scalar. A value type the
/// specification defines by a vocabulary rather than a spelling — its `Choice` — reduces to the
/// type of the vocabulary's members, because the members are what a value has to look like and
/// this table does not carry the lists themselves.
///
/// One row is not the September 2005 edition's: that edition types `xmp:Rating` as a choice of
/// integers, and the current one (Part 2, *Additional Properties*) types it as a choice of reals.
/// Section 6.6.2.3.1 cites the specification without a date, so the later and wider type is the one
/// a file is entitled to, and `1.0` is a rating.
static PREDEFINED: &[Schema] = &[
    Schema {
        uri: "http://purl.org/dc/elements/1.1/",
        name: "Dublin Core",
        prefix: "dc",
        properties: &[
            ("contributor", Shape::Unordered, Lexical::Any),
            ("coverage", Shape::Simple, Lexical::Any),
            ("creator", Shape::Ordered, Lexical::Any),
            ("date", Shape::Ordered, Lexical::Date),
            ("description", Shape::Language, Lexical::Any),
            ("format", Shape::Simple, Lexical::Any),
            ("identifier", Shape::Simple, Lexical::Any),
            ("language", Shape::Unordered, Lexical::Any),
            ("publisher", Shape::Unordered, Lexical::Any),
            ("relation", Shape::Unordered, Lexical::Any),
            ("rights", Shape::Language, Lexical::Any),
            ("source", Shape::Simple, Lexical::Any),
            ("subject", Shape::Unordered, Lexical::Any),
            ("title", Shape::Language, Lexical::Any),
            ("type", Shape::Unordered, Lexical::Any),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/xap/1.0/",
        name: "XMP Basic",
        prefix: "xmp",
        properties: &[
            ("Advisory", Shape::Unordered, Lexical::Any),
            ("BaseURL", Shape::Simple, Lexical::Any),
            ("CreateDate", Shape::Simple, Lexical::Date),
            ("CreatorTool", Shape::Simple, Lexical::Any),
            ("Identifier", Shape::Unordered, Lexical::Any),
            ("Label", Shape::Simple, Lexical::Any),
            ("MetadataDate", Shape::Simple, Lexical::Date),
            ("ModifyDate", Shape::Simple, Lexical::Date),
            ("Nickname", Shape::Simple, Lexical::Any),
            ("Rating", Shape::Simple, Lexical::Real),
            ("Thumbnails", Shape::Alternative, Lexical::Any),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/xmp/Identifier/qual/1.0/",
        name: "XMP identifier qualifier",
        prefix: "xmpidq",
        properties: &[("Scheme", Shape::Simple, Lexical::Any)],
    },
    Schema {
        uri: "http://ns.adobe.com/xap/1.0/rights/",
        name: "XMP Rights Management",
        prefix: "xmpRights",
        properties: &[
            ("Certificate", Shape::Simple, Lexical::Any),
            ("Marked", Shape::Simple, Lexical::Boolean),
            ("Owner", Shape::Unordered, Lexical::Any),
            ("UsageTerms", Shape::Language, Lexical::Any),
            ("WebStatement", Shape::Simple, Lexical::Any),
        ],
    },
    Schema {
        uri: MEDIA_MANAGEMENT_URI,
        name: "XMP Media Management",
        prefix: "xmpMM",
        properties: &[
            ("DerivedFrom", Shape::Structure, Lexical::Any),
            ("DocumentID", Shape::Simple, Lexical::Any),
            ("History", Shape::Ordered, Lexical::Any),
            ("InstanceID", Shape::Simple, Lexical::Any),
            ("LastURL", Shape::Simple, Lexical::Any),
            ("ManageTo", Shape::Simple, Lexical::Any),
            ("ManageUI", Shape::Simple, Lexical::Any),
            ("ManagedFrom", Shape::Structure, Lexical::Any),
            ("Manager", Shape::Simple, Lexical::Any),
            ("ManagerVariant", Shape::Simple, Lexical::Any),
            ("RenditionClass", Shape::Simple, Lexical::Any),
            ("RenditionOf", Shape::Structure, Lexical::Any),
            ("RenditionParams", Shape::Simple, Lexical::Any),
            ("SaveID", Shape::Simple, Lexical::Integer),
            ("VersionID", Shape::Simple, Lexical::Any),
            ("Versions", Shape::Ordered, Lexical::Any),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/xap/1.0/bj/",
        name: "XMP Basic Job Ticket",
        prefix: "xmpBJ",
        properties: &[("JobRef", Shape::Unordered, Lexical::Any)],
    },
    Schema {
        uri: "http://ns.adobe.com/xap/1.0/t/pg/",
        name: "XMP Paged-Text",
        prefix: "xmpTPg",
        properties: &[
            ("Colorants", Shape::Ordered, Lexical::Any),
            ("Fonts", Shape::Unordered, Lexical::Any),
            ("MaxPageSize", Shape::Structure, Lexical::Any),
            ("NPages", Shape::Simple, Lexical::Integer),
            ("PlateNames", Shape::Ordered, Lexical::Any),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
        name: "XMP Dynamic Media",
        prefix: "xmpDM",
        properties: &[
            ("absPeakAudioFilePath", Shape::Simple, Lexical::Any),
            ("album", Shape::Simple, Lexical::Any),
            ("altTapeName", Shape::Simple, Lexical::Any),
            ("altTimecode", Shape::Structure, Lexical::Any),
            ("artist", Shape::Simple, Lexical::Any),
            ("audioChannelType", Shape::Simple, Lexical::Any),
            ("audioCompressor", Shape::Simple, Lexical::Any),
            ("audioModDate", Shape::Simple, Lexical::Date),
            ("audioSampleRate", Shape::Simple, Lexical::Integer),
            ("audioSampleType", Shape::Simple, Lexical::Any),
            ("beatSpliceParams", Shape::Structure, Lexical::Any),
            ("composer", Shape::Simple, Lexical::Any),
            ("contributedMedia", Shape::Unordered, Lexical::Any),
            ("copyright", Shape::Simple, Lexical::Any),
            ("duration", Shape::Structure, Lexical::Any),
            ("engineer", Shape::Simple, Lexical::Any),
            ("fileDataRate", Shape::Simple, Lexical::Rational),
            ("genre", Shape::Simple, Lexical::Any),
            ("instrument", Shape::Simple, Lexical::Any),
            ("introTime", Shape::Structure, Lexical::Any),
            ("key", Shape::Simple, Lexical::Any),
            ("logComment", Shape::Simple, Lexical::Any),
            ("loop", Shape::Simple, Lexical::Boolean),
            ("markers", Shape::Ordered, Lexical::Any),
            ("metadataModDate", Shape::Simple, Lexical::Date),
            ("numberOfBeats", Shape::Simple, Lexical::Real),
            ("outCue", Shape::Structure, Lexical::Any),
            ("projectRef", Shape::Structure, Lexical::Any),
            ("pullDown", Shape::Simple, Lexical::Any),
            ("relativePeakAudioFilePath", Shape::Simple, Lexical::Any),
            ("relativeTimestamp", Shape::Structure, Lexical::Any),
            ("releaseDate", Shape::Simple, Lexical::Date),
            ("resampleParams", Shape::Structure, Lexical::Any),
            ("scaleType", Shape::Simple, Lexical::Any),
            ("scene", Shape::Simple, Lexical::Any),
            ("shotDate", Shape::Simple, Lexical::Date),
            ("shotLocation", Shape::Simple, Lexical::Any),
            ("shotName", Shape::Simple, Lexical::Any),
            ("speakerPlacement", Shape::Simple, Lexical::Any),
            ("startTimecode", Shape::Structure, Lexical::Any),
            ("stretchMode", Shape::Simple, Lexical::Any),
            ("tapeName", Shape::Simple, Lexical::Any),
            ("tempo", Shape::Simple, Lexical::Real),
            ("timeScaleParams", Shape::Structure, Lexical::Any),
            ("timeSignature", Shape::Simple, Lexical::Any),
            ("trackNumber", Shape::Simple, Lexical::Integer),
            ("videoAlphaMode", Shape::Simple, Lexical::Any),
            ("videoAlphaPremultipleColor", Shape::Structure, Lexical::Any),
            (
                "videoAlphaUnityIsTransparent",
                Shape::Simple,
                Lexical::Boolean,
            ),
            ("videoColorSpace", Shape::Simple, Lexical::Any),
            ("videoCompressor", Shape::Simple, Lexical::Any),
            ("videoFieldOrder", Shape::Simple, Lexical::Any),
            ("videoFrameRate", Shape::Simple, Lexical::Any),
            ("videoFrameSize", Shape::Structure, Lexical::Any),
            ("videoModDate", Shape::Simple, Lexical::Date),
            ("videoPixelAspectRatio", Shape::Simple, Lexical::Rational),
            ("videoPixelDepth", Shape::Simple, Lexical::Any),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/pdf/1.3/",
        name: "Adobe PDF",
        prefix: "pdf",
        properties: &[
            ("Keywords", Shape::Simple, Lexical::Any),
            ("PDFVersion", Shape::Simple, Lexical::Any),
            ("Producer", Shape::Simple, Lexical::Any),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/photoshop/1.0/",
        name: "Photoshop",
        prefix: "photoshop",
        properties: &[
            ("AuthorsPosition", Shape::Simple, Lexical::Any),
            ("CaptionWriter", Shape::Simple, Lexical::Any),
            ("Category", Shape::Simple, Lexical::Any),
            ("City", Shape::Simple, Lexical::Any),
            ("Country", Shape::Simple, Lexical::Any),
            ("Credit", Shape::Simple, Lexical::Any),
            ("DateCreated", Shape::Simple, Lexical::Date),
            ("Headline", Shape::Simple, Lexical::Any),
            ("Instructions", Shape::Simple, Lexical::Any),
            ("Source", Shape::Simple, Lexical::Any),
            ("State", Shape::Simple, Lexical::Any),
            ("SupplementalCategories", Shape::Unordered, Lexical::Any),
            ("TransmissionReference", Shape::Simple, Lexical::Any),
            ("Urgency", Shape::Simple, Lexical::Integer),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/camera-raw-settings/1.0/",
        name: "Camera Raw",
        prefix: "crs",
        properties: &[
            ("AutoBrightness", Shape::Simple, Lexical::Boolean),
            ("AutoContrast", Shape::Simple, Lexical::Boolean),
            ("AutoExposure", Shape::Simple, Lexical::Boolean),
            ("AutoShadows", Shape::Simple, Lexical::Boolean),
            ("BlueHue", Shape::Simple, Lexical::Integer),
            ("BlueSaturation", Shape::Simple, Lexical::Integer),
            ("Brightness", Shape::Simple, Lexical::Integer),
            ("CameraProfile", Shape::Simple, Lexical::Any),
            ("ChromaticAberrationB", Shape::Simple, Lexical::Integer),
            ("ChromaticAberrationR", Shape::Simple, Lexical::Integer),
            ("ColorNoiseReduction", Shape::Simple, Lexical::Integer),
            ("Contrast", Shape::Simple, Lexical::Integer),
            ("CropAngle", Shape::Simple, Lexical::Real),
            ("CropBottom", Shape::Simple, Lexical::Real),
            ("CropHeight", Shape::Simple, Lexical::Real),
            ("CropLeft", Shape::Simple, Lexical::Real),
            ("CropRight", Shape::Simple, Lexical::Real),
            ("CropTop", Shape::Simple, Lexical::Real),
            ("CropUnits", Shape::Simple, Lexical::Integer),
            ("CropWidth", Shape::Simple, Lexical::Real),
            ("Exposure", Shape::Simple, Lexical::Real),
            ("GreenHue", Shape::Simple, Lexical::Integer),
            ("GreenSaturation", Shape::Simple, Lexical::Integer),
            ("HasCrop", Shape::Simple, Lexical::Boolean),
            ("HasSettings", Shape::Simple, Lexical::Boolean),
            ("LuminanceSmoothing", Shape::Simple, Lexical::Integer),
            ("RawFileName", Shape::Simple, Lexical::Any),
            ("RedHue", Shape::Simple, Lexical::Integer),
            ("RedSaturation", Shape::Simple, Lexical::Integer),
            ("Saturation", Shape::Simple, Lexical::Integer),
            ("ShadowTint", Shape::Simple, Lexical::Integer),
            ("Shadows", Shape::Simple, Lexical::Integer),
            ("Sharpness", Shape::Simple, Lexical::Integer),
            ("Temperature", Shape::Simple, Lexical::Integer),
            ("Tint", Shape::Simple, Lexical::Integer),
            ("ToneCurve", Shape::Ordered, Lexical::Any),
            ("ToneCurveName", Shape::Simple, Lexical::Any),
            ("Version", Shape::Simple, Lexical::Any),
            ("VignetteAmount", Shape::Simple, Lexical::Integer),
            ("VignetteMidpoint", Shape::Simple, Lexical::Integer),
            ("WhiteBalance", Shape::Simple, Lexical::Any),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/tiff/1.0/",
        name: "EXIF for TIFF properties",
        prefix: "tiff",
        properties: &[
            ("Artist", Shape::Simple, Lexical::Any),
            ("BitsPerSample", Shape::Ordered, Lexical::Integer),
            ("Compression", Shape::Simple, Lexical::Integer),
            ("Copyright", Shape::Language, Lexical::Any),
            ("DateTime", Shape::Simple, Lexical::Date),
            ("ImageDescription", Shape::Language, Lexical::Any),
            ("ImageLength", Shape::Simple, Lexical::Integer),
            ("ImageWidth", Shape::Simple, Lexical::Integer),
            ("Make", Shape::Simple, Lexical::Any),
            ("Model", Shape::Simple, Lexical::Any),
            ("Orientation", Shape::Simple, Lexical::Integer),
            ("PhotometricInterpretation", Shape::Simple, Lexical::Integer),
            ("PlanarConfiguration", Shape::Simple, Lexical::Integer),
            ("PrimaryChromaticities", Shape::Ordered, Lexical::Rational),
            ("ReferenceBlackWhite", Shape::Ordered, Lexical::Rational),
            ("ResolutionUnit", Shape::Simple, Lexical::Integer),
            ("SamplesPerPixel", Shape::Simple, Lexical::Integer),
            ("Software", Shape::Simple, Lexical::Any),
            ("TransferFunction", Shape::Ordered, Lexical::Integer),
            ("WhitePoint", Shape::Ordered, Lexical::Rational),
            ("XResolution", Shape::Simple, Lexical::Rational),
            ("YCbCrCoefficients", Shape::Ordered, Lexical::Rational),
            ("YCbCrPositioning", Shape::Simple, Lexical::Integer),
            ("YCbCrSubSampling", Shape::Ordered, Lexical::Integer),
            ("YResolution", Shape::Simple, Lexical::Rational),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/exif/1.0/",
        name: "EXIF-specific properties",
        prefix: "exif",
        properties: &[
            ("ApertureValue", Shape::Simple, Lexical::Rational),
            ("BrightnessValue", Shape::Simple, Lexical::Rational),
            ("CFAPattern", Shape::Structure, Lexical::Any),
            ("ColorSpace", Shape::Simple, Lexical::Integer),
            ("ComponentsConfiguration", Shape::Ordered, Lexical::Integer),
            ("CompressedBitsPerPixel", Shape::Simple, Lexical::Rational),
            ("Contrast", Shape::Simple, Lexical::Integer),
            ("CustomRendered", Shape::Simple, Lexical::Integer),
            ("DateTimeDigitized", Shape::Simple, Lexical::Date),
            ("DateTimeOriginal", Shape::Simple, Lexical::Date),
            ("DeviceSettingDescription", Shape::Structure, Lexical::Any),
            ("DigitalZoomRatio", Shape::Simple, Lexical::Rational),
            ("ExifVersion", Shape::Simple, Lexical::Any),
            ("ExposureBiasValue", Shape::Simple, Lexical::Rational),
            ("ExposureIndex", Shape::Simple, Lexical::Rational),
            ("ExposureMode", Shape::Simple, Lexical::Integer),
            ("ExposureProgram", Shape::Simple, Lexical::Integer),
            ("ExposureTime", Shape::Simple, Lexical::Rational),
            ("FNumber", Shape::Simple, Lexical::Rational),
            ("FileSource", Shape::Simple, Lexical::Integer),
            ("Flash", Shape::Structure, Lexical::Any),
            ("FlashEnergy", Shape::Simple, Lexical::Rational),
            ("FlashpixVersion", Shape::Simple, Lexical::Any),
            ("FocalLength", Shape::Simple, Lexical::Rational),
            ("FocalLengthIn35mmFilm", Shape::Simple, Lexical::Integer),
            ("FocalPlaneResolutionUnit", Shape::Simple, Lexical::Integer),
            ("FocalPlaneXResolution", Shape::Simple, Lexical::Rational),
            ("FocalPlaneYResolution", Shape::Simple, Lexical::Rational),
            ("GPSAltitude", Shape::Simple, Lexical::Rational),
            ("GPSAltitudeRef", Shape::Simple, Lexical::Integer),
            ("GPSAreaInformation", Shape::Simple, Lexical::Any),
            ("GPSDOP", Shape::Simple, Lexical::Rational),
            ("GPSDestBearing", Shape::Simple, Lexical::Rational),
            ("GPSDestBearingRef", Shape::Simple, Lexical::Any),
            ("GPSDestDistance", Shape::Simple, Lexical::Rational),
            ("GPSDestDistanceRef", Shape::Simple, Lexical::Any),
            ("GPSDestLatitude", Shape::Simple, Lexical::Coordinate),
            ("GPSDestLongitude", Shape::Simple, Lexical::Coordinate),
            ("GPSDifferential", Shape::Simple, Lexical::Integer),
            ("GPSImgDirection", Shape::Simple, Lexical::Rational),
            ("GPSImgDirectionRef", Shape::Simple, Lexical::Any),
            ("GPSLatitude", Shape::Simple, Lexical::Coordinate),
            ("GPSLongitude", Shape::Simple, Lexical::Coordinate),
            ("GPSMapDatum", Shape::Simple, Lexical::Any),
            ("GPSMeasureMode", Shape::Simple, Lexical::Any),
            ("GPSProcessingMethod", Shape::Simple, Lexical::Any),
            ("GPSSatellites", Shape::Simple, Lexical::Any),
            ("GPSSpeed", Shape::Simple, Lexical::Rational),
            ("GPSSpeedRef", Shape::Simple, Lexical::Any),
            ("GPSStatus", Shape::Simple, Lexical::Any),
            ("GPSTimeStamp", Shape::Simple, Lexical::Date),
            ("GPSTrack", Shape::Simple, Lexical::Rational),
            ("GPSTrackRef", Shape::Simple, Lexical::Any),
            ("GPSVersionID", Shape::Simple, Lexical::Any),
            ("GainControl", Shape::Simple, Lexical::Integer),
            ("ISOSpeedRatings", Shape::Ordered, Lexical::Integer),
            ("ImageUniqueID", Shape::Simple, Lexical::Any),
            ("LightSource", Shape::Simple, Lexical::Integer),
            ("MaxApertureValue", Shape::Simple, Lexical::Rational),
            ("MeteringMode", Shape::Simple, Lexical::Integer),
            ("OECF", Shape::Structure, Lexical::Any),
            ("PixelXDimension", Shape::Simple, Lexical::Integer),
            ("PixelYDimension", Shape::Simple, Lexical::Integer),
            ("RelatedSoundFile", Shape::Simple, Lexical::Any),
            ("Saturation", Shape::Simple, Lexical::Integer),
            ("SceneCaptureType", Shape::Simple, Lexical::Integer),
            ("SceneType", Shape::Simple, Lexical::Integer),
            ("SensingMethod", Shape::Simple, Lexical::Integer),
            ("Sharpness", Shape::Simple, Lexical::Integer),
            ("ShutterSpeedValue", Shape::Simple, Lexical::Rational),
            ("SpatialFrequencyResponse", Shape::Structure, Lexical::Any),
            ("SpectralSensitivity", Shape::Simple, Lexical::Any),
            ("SubjectArea", Shape::Ordered, Lexical::Integer),
            ("SubjectDistance", Shape::Simple, Lexical::Rational),
            ("SubjectDistanceRange", Shape::Simple, Lexical::Integer),
            ("SubjectLocation", Shape::Ordered, Lexical::Integer),
            ("UserComment", Shape::Language, Lexical::Any),
            ("WhiteBalance", Shape::Simple, Lexical::Integer),
        ],
    },
    Schema {
        uri: "http://ns.adobe.com/exif/1.0/aux/",
        name: "additional EXIF properties",
        prefix: "aux",
        properties: &[
            ("Lens", Shape::Simple, Lexical::Any),
            ("SerialNumber", Shape::Simple, Lexical::Any),
        ],
    },
];

/// The schema a namespace URI names, where the XMP Specification predefines one.
fn predefined(namespace: &str) -> Option<&'static Schema> {
    PREDEFINED.iter().find(|schema| schema.uri == namespace)
}

/// Whether a namespace is one ISO 19005 itself defines rather than the XMP Specification.
///
/// ISO 19005-2 section 6.6.2.3.1 admits the schemas of ISO 19005-1 and of part 2 alongside the XMP
/// Specification's: the identification schema of each part's own subclause, and section 6.6.2.3.3's
/// container schema together with the four value types whose fields it is built from.
fn defined_by_iso_19005(namespace: &str) -> bool {
    IDENTIFICATION_URIS.contains(&namespace)
        || matches!(
            namespace,
            EXTENSION_URI | SCHEMA_URI | PROPERTY_URI | TYPE_URI | FIELD_URI
        )
}

/// Every metadata stream's packet, read with the structure fields `Value` drops.
///
/// A packet that will not parse is [`xmp_packets_well_formed`]'s finding and not these rows',
/// which is why one is passed over here in silence rather than reported twice.
fn for_each_packet(exam: &Examination<'_>, mut visit: impl FnMut(ObjectId, &[XmpProperty])) {
    let document = exam.document;
    for_each_metadata_stream(exam, |id, stream| {
        let Some(bytes) = document.decoded_stream_data(stream) else {
            return;
        };
        if let Ok(properties) = Xmp::parse_detail(&bytes) {
            visit(id, &properties);
        }
    });
}

/// A property as a finding spells it: the preferred prefix where the schema is predefined, and
/// the packet's own where it is not.
fn spelled(property: &XmpProperty) -> String {
    let prefix = predefined(&property.name.namespace)
        .map_or(property.prefix.as_str(), |schema| schema.prefix);
    if prefix.is_empty() {
        property.name.local.clone()
    } else {
        format!("{prefix}:{}", property.name.local)
    }
}

/// ISO 19005-2 section 6.6.2.3.1.
///
/// The subclause requires every property to use a predefined schema or an extension schema
/// complying with section 6.6.2.3.2, and *using* a schema is more than borrowing its namespace: a
/// property the specification gives a value type has to carry a value of that type, or the file
/// has invented a property inside somebody else's schema. So this row judges the predefined half
/// — the shape of the value, the lexical form of its scalar, and the language qualifiers a
/// language alternative's items are defined to have. The other half, a namespace no predefined
/// schema owns, is [`extension_schemas_embedded`]'s row, because section 6.6.2.3.1 states it by
/// deferring to section 6.6.2.3.2 and a reader that reported both would report each file twice.
///
/// **A property whose local name the table does not carry is not judged**, and that is a reading
/// rather than a gap. [`PREDEFINED`] is the September 2005 edition of the XMP Specification, the
/// one ISO 19005-2's bibliography names; the subclause cites the specification without a date,
/// so a later edition's additions are predefined too, and refusing a name this table has not
/// heard of would fail conforming files over the age of the table.
///
/// # `TechNote 0010` A020, which is the committee saying how far this row may go
///
/// The working group resolved that parts 1 to 3 are read as if an XMP value were validated **on
/// its type alone**, with any other meaning inferable from a property's name or description
/// disregarded. That is the approach this row already took — a value is held to the shape and the
/// lexical form its type has, and to nothing a human-readable description says about permitted
/// values — and A020 also settles the one place it went further than a type: a `Rational` is now
/// admitted as any string, which [`Lexical`] records with the rest of the note's list and the
/// reasons this table's readings agree or are asked about in `doc/questions/Q53`.
///
/// The language-qualifier finding below survives the resolution and is worth saying why: an `Alt`
/// whose items carry `xml:lang` is what the *type* language alternative is, in the XMP
/// Specification's own definition of it, rather than something read off the property's name.
fn properties_use_known_schemas(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_packet(exam, |id, properties| {
        for property in properties {
            let Some(schema) = predefined(&property.name.namespace) else {
                continue;
            };
            let Some((shape, lexical)) = schema.property(&property.name.local) else {
                continue;
            };
            judge_against_schema(id, property, schema, shape, lexical, findings);
        }
    });
}

/// One property held to the value type its predefined schema gives it.
fn judge_against_schema(
    id: ObjectId,
    property: &XmpProperty,
    schema: &Schema,
    shape: Shape,
    lexical: Lexical,
    findings: &mut Findings,
) {
    let place = || Where::object(id).named(spelled(property));
    if !shape.accepts(&property.value) {
        findings.record(
            place(),
            format!(
                "the {} schema defines {} as {}, and the packet states {}",
                schema.name,
                spelled(property),
                shape.describe(),
                shaped(&property.value)
            ),
        );
        return;
    }
    if shape == Shape::Language
        && let Some(items) = property.value.alternatives()
        && items.iter().any(|(language, _)| language.is_none())
    {
        findings.record(
            place(),
            format!(
                "{} is a language alternative, whose items are each defined to carry an \
                 xml:lang qualifier, and one of them states none",
                spelled(property)
            ),
        );
    }
    for text in scalars(&property.value) {
        if !lexical.accepts(text) {
            findings.record(
                place(),
                format!(
                    "the {} schema defines {} as {}, and the packet states {}",
                    schema.name,
                    spelled(property),
                    lexical.describe(),
                    short(text)
                ),
            );
        }
    }
}

/// What to call the shape a packet actually stated, for a finding.
fn shaped(stated: &Detail) -> &'static str {
    match stated {
        Detail::Text(_) => Shape::Simple.describe(),
        Detail::Alt(_) => Shape::Alternative.describe(),
        Detail::Seq(_) => Shape::Ordered.describe(),
        Detail::Bag(_) => Shape::Unordered.describe(),
        Detail::Structure(_) => Shape::Structure.describe(),
    }
}

/// The strings a value holds directly: itself where it is simple, its items where it is an array.
///
/// A structure's fields are not among them: the specification types a field inside its own value
/// type's table rather than in the schema's, and this reader carries the schema tables only.
fn scalars(value: &Detail) -> Vec<&str> {
    match value {
        Detail::Text(text) => vec![text.as_str()],
        Detail::Alt(items) => items.iter().filter_map(|(_, item)| item.text()).collect(),
        Detail::Seq(items) | Detail::Bag(items) => items.iter().filter_map(Detail::text).collect(),
        Detail::Structure(_) => Vec::new(),
    }
}

/// The namespace URIs the extension schemas described in one packet name.
///
/// Table 2's container is a bag; a sequence is accepted here as well, because what this function
/// answers is which schemas a packet *describes*, and a producer that ordered the bag has still
/// described them. Whether the container has the shape the table gives it is
/// [`extension_schema_container_fields`]'s row.
fn described_schemas(properties: &[XmpProperty]) -> Vec<String> {
    let mut described = Vec::new();
    for property in properties {
        if property.name.namespace != EXTENSION_URI || property.name.local != "schemas" {
            continue;
        }
        for schema in property.value.array().unwrap_or_default() {
            if let Some(field) = schema.field(SCHEMA_URI, "namespaceURI")
                && let Some(uri) = field.value.text()
            {
                described.push(uri.trim().to_owned());
            }
        }
    }
    described
}

/// ISO 19005-2 section 6.6.2.3.2.
///
/// A property whose namespace no predefined schema owns needs an extension schema describing it,
/// and the subclause says where that description may be: in the stream the property is in, or in
/// the catalog's, whose schemas every stream inherits. So the catalog's packet is read once and
/// its descriptions carried into every stream.
///
/// A property in no namespace at all is passed over: the packet has not named a schema for it,
/// and neither can this row.
fn extension_schemas_embedded(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let inherited = catalog_detail(document)
        .map(|properties| described_schemas(&properties))
        .unwrap_or_default();
    for_each_packet(exam, |id, properties| {
        let mut described = inherited.clone();
        described.extend(described_schemas(properties));
        for property in properties {
            let namespace = &property.name.namespace;
            if namespace.is_empty()
                || predefined(namespace).is_some()
                || defined_by_iso_19005(namespace)
                || described.iter().any(|uri| uri == namespace)
            {
                continue;
            }
            findings.record(
                Where::object(id).named(spelled(property)),
                format!(
                    "the packet states {}, whose schema {namespace} is neither predefined nor \
                     described by an extension schema in this stream or the catalog's",
                    spelled(property)
                ),
            );
        }
    });
}

/// The catalog's packet, read with its structure fields.
fn catalog_detail(document: &Document) -> Option<Vec<XmpProperty>> {
    let catalog = document.catalog().ok()?;
    let stream = document.get_key(&catalog, "Metadata");
    let bytes = document.decoded_stream_data(stream.as_stream()?)?;
    Xmp::parse_detail(&bytes).ok()
}

/// One of ISO 19005-2 section 6.6.2.3.3's four value types, as its table defines it.
struct ValueType {
    /// The field namespace URI the table gives it.
    uri: &'static str,
    /// The prefix the table *requires* — section 6.6.2.2 makes a prefix meaningless except where
    /// one is identified as required, and each of these four tables identifies one.
    prefix: &'static str,
    /// What to call it in a finding.
    name: &'static str,
    /// Every field the table describes, all of which section 6.6.2.3.2 requires to be present.
    fields: &'static [&'static str],
    /// The fields of that list a validator allows to be absent, and why they are allowed.
    ///
    /// Two of them, and both come from PDF Association `TechNote 0010`'s item A029, where the
    /// ISO working group resolved that parts 1 to 3 are read as if a schema describing no custom
    /// value types may omit `pdfaSchema:valueType`, and a value type describing no structured
    /// fields may omit `pdfaType:field` — a validator allowing each absence and treating it as
    /// an empty array. `crate::clarification` carries the record and the report cites it.
    ///
    /// It is a field of the table rather than a branch in the predicate because the exemption is
    /// a property of *which table* a structure is held to, which is what this type is.
    may_be_absent: &'static [&'static str],
}

/// Table 3, the Schema value type.
static SCHEMA_TYPE: ValueType = ValueType {
    uri: SCHEMA_URI,
    prefix: "pdfaSchema",
    name: "extension schema description",
    fields: &["schema", "namespaceURI", "prefix", "property", "valueType"],
    may_be_absent: &["valueType"],
};

/// Table 4, the Property value type.
static PROPERTY_TYPE: ValueType = ValueType {
    uri: PROPERTY_URI,
    prefix: "pdfaProperty",
    name: "extension schema property",
    fields: &["name", "valueType", "category", "description"],
    may_be_absent: &[],
};

/// Table 5, the `ValueType` value type.
static TYPE_TYPE: ValueType = ValueType {
    uri: TYPE_URI,
    prefix: "pdfaType",
    name: "extension schema value type",
    fields: &["type", "namespaceURI", "prefix", "description", "field"],
    may_be_absent: &["field"],
};

/// Table 6, the Field value type.
static FIELD_TYPE: ValueType = ValueType {
    uri: FIELD_URI,
    prefix: "pdfaField",
    name: "extension schema value type field",
    fields: &["name", "valueType", "description"],
    may_be_absent: &[],
};

/// ISO 19005-2 section 6.6.2.3.3, and the sentence of section 6.6.2.3.2 that binds it.
///
/// Section 6.6.2.3.2 requires every field described in each of section 6.6.2.3.3's tables to be
/// present in any extension schema container schema, and each of those tables names the prefix its
/// fields are required to be spelled with — which section 6.6.2.2 makes load-bearing by saying a
/// prefix means nothing *except* where one is identified as required.
///
/// **One of Table 3's five fields is exempt and one is not, and the difference is a document this
/// tree now holds.** `TechNote 0010`'s A029 records the ISO working group resolving that a schema
/// describing no custom value types may omit `pdfaSchema:valueType`, and a value type describing
/// no structured fields may omit `pdfaType:field`; each absence is allowed and read as an empty
/// array, which [`ValueType::may_be_absent`] implements. The resolution reaches those two field
/// names and no others — so a schema that describes no properties still has to state
/// `pdfaSchema:property`, on the sentence as published. That asymmetry is the clarification's own
/// and is why the corpus's two witnesses for this subclause are now ruled opposite ways
/// (`tests/corpus.rs`, `doc/adr/0931`).
fn extension_schema_container_fields(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_packet(exam, |id, properties| {
        for property in properties {
            if property.name.namespace != EXTENSION_URI || property.name.local != "schemas" {
                continue;
            }
            let Some(schemas) = property.value.array() else {
                findings.record(
                    Where::object(id).named("pdfaExtension:schemas"),
                    format!(
                        "the extension schema container states {} where Table 2 gives it a bag \
                         of schema descriptions",
                        shaped(&property.value)
                    ),
                );
                continue;
            };
            for schema in schemas {
                check_value_type(id, schema, &SCHEMA_TYPE, findings);
                check_sequence(id, schema, SCHEMA_URI, "property", &PROPERTY_TYPE, findings);
                for value_type in sequence(schema, SCHEMA_URI, "valueType") {
                    check_value_type(id, value_type, &TYPE_TYPE, findings);
                    check_sequence(id, value_type, TYPE_URI, "field", &FIELD_TYPE, findings);
                }
            }
        }
    });
}

/// The items of one field of a structure, where that field is an array.
fn sequence<'a>(structure: &'a Detail, uri: &str, local: &str) -> &'a [Detail] {
    structure
        .field(uri, local)
        .and_then(|field| field.value.array())
        .unwrap_or_default()
}

/// Every item of one field's array held to a value type's table.
fn check_sequence(
    id: ObjectId,
    structure: &Detail,
    uri: &str,
    local: &str,
    expected: &ValueType,
    findings: &mut Findings,
) {
    for item in sequence(structure, uri, local) {
        check_value_type(id, item, expected, findings);
    }
}

/// One structure held to the table that defines it: every field present, spelled with the
/// prefix the table requires.
fn check_value_type(
    id: ObjectId,
    structure: &Detail,
    expected: &ValueType,
    findings: &mut Findings,
) {
    let place = |name: &str| Where::object(id).named(format!("{}:{name}", expected.prefix));
    let Some(fields) = structure.fields() else {
        findings.record(
            Where::object(id).named(expected.name),
            format!(
                "an {} is {} where section 6.6.2.3.3 defines it as a structure",
                expected.name,
                shaped(structure)
            ),
        );
        return;
    };
    for wanted in expected.fields {
        let stated = fields
            .iter()
            .find(|field| field.name.namespace == expected.uri && field.name.local == *wanted);
        match stated {
            None if expected.may_be_absent.contains(wanted) => {}
            None => findings.record(
                place(wanted),
                format!(
                    "an {} states no {}:{wanted}, which its table describes",
                    expected.name, expected.prefix
                ),
            ),
            Some(field) if field.prefix != expected.prefix => findings.record(
                place(wanted),
                format!(
                    "an {} spells {wanted} with the prefix {}, where its table requires {}",
                    expected.name, field.prefix, expected.prefix
                ),
            ),
            Some(_) => {}
        }
    }
}

/// The XMP Media Management namespace, whose `History` property records the actions.
///
/// ISO 19005-2 Table 1 and ISO 19005-4 Table 1 both bind it to the `xmpMM` prefix, and both
/// Section 6.6.6 and section 6.7.5 name the property by that prefix.
const MEDIA_MANAGEMENT_URI: &str = "http://ns.adobe.com/xap/1.0/mm/";

/// The field namespace of the XMP Specification's `ResourceEvent` value type.
///
/// **Where this comes from.** ISO 19005-2 section 6.6.6 and ISO 19005-4 section 6.7.5 name the
/// fields — `action`, `parameters`, `when` — and name no namespace for them, because the structure
/// they are fields of is the XMP Specification's: `xmpMM:History` is a sequence of `ResourceEvent`
/// structures, and that value type is where the field names are defined. That specification is the
/// same one [`PREDEFINED`] is read from, for the same reason and on the same terms — ISO 19005-2
/// lists it in its bibliography with a public address — so this is a primary source already in use
/// here rather than a new one, and `doc/questions/A20`'s prohibition on checking from a secondary
/// source is not engaged.
const RESOURCE_EVENT_URI: &str = "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#";

/// The prefix the XMP Specification uses for [`RESOURCE_EVENT_URI`], for a finding to print.
///
/// **Not a required prefix and not treated as one**: the lookup is by namespace URI, as section
/// 6.6.2.2 and section 6.7.2.2 require of every prefix neither part identifies as required. This is
/// here only so that a message names the field the way the reader's packet spells it.
const RESOURCE_EVENT_PREFIX: &str = "stEvt";

/// Every action the catalog's packet records, as ISO 19005-2 section 6.6.6 and ISO 19005-4 section
/// 6.7.5 mean the phrase: one item of `xmpMM:History`.
///
/// **The catalog's packet and no other.** Both subclauses say the property is inside the metadata
/// stream that is the value of the `Metadata` entry in the document catalog dictionary, so a
/// history in a page's or a font's stream is not what either clause is about.
///
/// A `History` that is not an array is passed over here: what shape the property takes is
/// [`properties_use_known_schemas`]'s row, which holds it to the XMP Specification's `seq`, and
/// reporting the same fault under two requirements would make a report state it twice.
fn for_each_recorded_action(document: &Document, mut visit: impl FnMut(usize, &Detail)) {
    let Some(properties) = catalog_detail(document) else {
        return;
    };
    let mut ordinal = 0usize;
    for property in &properties {
        if property.name.namespace != MEDIA_MANAGEMENT_URI || property.name.local != "History" {
            continue;
        }
        for action in property.value.array().unwrap_or_default() {
            ordinal = ordinal.saturating_add(1);
            visit(ordinal, action);
        }
    }
}

/// Every action recorded in the history states the fields the part requires of it.
///
/// **This used to be the shared body of two rows and is now part 4's alone.** Part 2's row is a
/// [`Check::OutsideValidation`] on `TechNote 0010`'s A021, so the list is no longer a parameter
/// that varies by part; it stays a parameter because the caller reads better for naming the two
/// fields it is about, and because a part that added a third would add it there.
fn recorded_action_fields(document: &Document, findings: &mut Findings, required: &[&str]) {
    for_each_recorded_action(document, |ordinal, action| {
        let Some(fields) = action.fields() else {
            findings.record(
                Where::file().named("xmpMM:History"),
                format!(
                    "history entry {ordinal} is {} where a recorded action is a structure of \
                     fields",
                    shaped(action)
                ),
            );
            return;
        };
        for wanted in required {
            let stated = fields.iter().any(|field| {
                field.name.namespace == RESOURCE_EVENT_URI && field.name.local == *wanted
            });
            if !stated {
                findings.record(
                    Where::file().named(format!("{RESOURCE_EVENT_PREFIX}:{wanted}")),
                    format!(
                        "history entry {ordinal} specifies no {wanted} field, which the \
                         subclause requires of each recorded action"
                    ),
                );
            }
        }
    });
}

/// ISO 19005-4 section 6.7.5.
///
/// Part 4 states that the actions *may* be recorded and then requires two fields of each action
/// that is, so the rule is conditional on the file's own `xmpMM:History` and vacuous for a file
/// that states none. `parameters` is a recommendation here, alongside `softwareAgent` and
/// `instanceID`.
///
/// **Why part 2's twin is outside validation and this row is not.** `TechNote 0010`'s A021 names
/// ISO 19005-2 and ISO 19005-3 and no other part, and it appends a note that the proposal was
/// accepted in principle for the part then being drafted. That part was published in 2020 as
/// ISO 19005-4 — and it states these two fields as requirements anyway. Where a note about an
/// intention and a published standard disagree, the standard is the one this project reads
/// (`CLAUDE.md` principle 5), so the later text governs and the clause binds.
fn provenance_recorded_action_fields_four(exam: &Examination<'_>, findings: &mut Findings) {
    recorded_action_fields(exam.document, findings, &["action", "when"]);
}

/// ISO 19005-2 section 6.6.4, the form of an amendment or corrigendum identifier that is stated.
///
/// Two sentences of the subclause, one shape: where a file conforms to a version of the part
/// defined by an amendment, `pdfaid:amd` is the amendment number and year separated by a colon,
/// and where it conforms to one defined by a corrigendum, `pdfaid:corr` is the corrigendum number
/// and year separated by a colon.
///
/// **What this row can and cannot decide.** Whether a file *ought* to state one is
/// `metadata/identification-amendment-and-corrigendum`'s row and is not decidable here: no other
/// fact in the file says which amendment it was written against. What is decidable is the value
/// of one the file *does* state, because stating the property is the file naming the amendment it
/// conforms to — Table 8 gives the two properties no other meaning — and the sentence then says
/// what that name looks like.
///
/// The form is read no further than the subclause states it: a colon, a number before it, a year
/// after it, both decimal. Section 6.6.4 says `year` where ISO 19005-4 section 6.7.3 says `four
/// digit year`, so four digits are not required here.
fn identification_amendment_form(exam: &Examination<'_>, findings: &mut Findings) {
    let packet = document_packet(exam.document);
    for (local, what) in [("amd", "amendment"), ("corr", "corrigendum")] {
        let Stated::Text(text) = stated(packet.as_ref(), local) else {
            continue;
        };
        if numbered_and_dated(text) {
            continue;
        }
        findings.record(
            Where::file().named(format!("{IDENTIFICATION_PREFIX}:{local}")),
            format!(
                "the file states {} as its {what} identifier, where the subclause requires the \
                 {what} number and year separated by a colon",
                short(text)
            ),
        );
    }
}

/// Whether a value is a number and a year separated by a colon, ISO 19005-2 section 6.6.4's form.
fn numbered_and_dated(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed
        .split_once(':')
        .is_some_and(|(number, year)| integer(number.trim()) && integer(year.trim()))
}

#[cfg(test)]
mod tests {
    use crate::Examination;
    use std::fmt::Write as _;

    use pdf_syntax::Document;

    use super::{
        Findings, PREDEFINED, REQUIREMENTS, catalog_metadata_stream, date,
        extension_schema_container_fields, extension_schemas_embedded,
        identification_amendment_form, identification_conformance_level,
        identification_declares_flavour_f, identification_declares_level_a,
        identification_part_four, identification_part_two, identification_revision_year,
        identification_states_no_flavour, packet_header, properties_use_known_schemas,
        provenance_recorded_action_fields_four, states_attribute, xmp_packet_header_attributes,
        xmp_packets_well_formed,
    };
    use super::{declared_target, identification_schema_prefix};
    use crate::target::{Flavour, Level, Target};

    /// A one-page document whose catalog names object 4 as its metadata stream, carrying `packet`.
    ///
    /// Passing an empty `packet` builds the same document with no `/Metadata` at all, which is the
    /// case every identification row has to answer.
    fn document(packet: &str) -> Document {
        let (metadata, object) = if packet.is_empty() {
            (String::new(), String::new())
        } else {
            (
                " /Metadata 4 0 R".to_owned(),
                format!(
                    "4 0 obj\n<< /Type /Metadata /Subtype /XML /Length {} >>\n\
                     stream\n{packet}\nendstream\nendobj\n",
                    packet.len()
                ),
            )
        };
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R{metadata} >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>\nendobj\n{object}"
        );

        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(out.len());
            out.push_str(object);
        }
        let xref_at = out.len();
        let size = offsets.len().saturating_add(1);
        let _ = writeln!(out, "xref\n0 {size}");
        out.push_str("0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture is a valid PDF")
    }

    /// An XMP packet stating the identification properties given, under `namespace`.
    fn packet(namespace: &str, properties: &str) -> String {
        format!(
            "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\" xmlns:pdfaid=\"{namespace}\">\n\
             {properties}\n\
             </rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>"
        )
    }

    /// How many places a predicate found, which is all most of these assertions need.
    fn found(predicate: fn(&Examination<'_>, &mut Findings), document: &Document) -> usize {
        let mut findings = Findings::default();
        let exam = Examination::new(document, Target::Four(Flavour::Plain));
        predicate(&exam, &mut findings);
        findings.seen()
    }

    /// The three rows that bind exactly one target, which is how a predicate with no view of the
    /// target still checks a rule that depends on it.
    ///
    /// `Applies::FromLevel(Level::A)` on a clause only part 2 states reaches Level A and nothing
    /// else, because a part 4 target fails the `stated` half of `Requirement::binds`; and
    /// `Applies::Flavours` names its flavour outright.
    #[test]
    fn the_rows_that_depend_on_the_target_bind_exactly_one_of_them() {
        for (id, only) in [
            (
                "metadata/identification-declares-level-a",
                Target::Two(Level::A),
            ),
            (
                "metadata/identification-declares-flavour-e",
                Target::Four(Flavour::E),
            ),
            (
                "metadata/identification-declares-flavour-f",
                Target::Four(Flavour::F),
            ),
            (
                "metadata/identification-states-no-flavour",
                Target::Four(Flavour::Plain),
            ),
        ] {
            let row = REQUIREMENTS
                .iter()
                .find(|requirement| requirement.id == id)
                .expect("the row is in this module");
            for target in Target::ALL {
                assert_eq!(row.binds(target), target == only, "{id} against {target}");
            }
        }
    }

    /// An XMP packet stating `properties` under a namespace bound to `prefix`.
    fn schema_packet(prefix: &str, namespace: &str, properties: &str) -> String {
        format!(
            "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\" xmlns:{prefix}=\"{namespace}\"\n\
               xmlns:pdfaExtension=\"http://www.aiim.org/pdfa/ns/extension/\"\n\
               xmlns:pdfaSchema=\"http://www.aiim.org/pdfa/ns/schema#\"\n\
               xmlns:pdfaProperty=\"http://www.aiim.org/pdfa/ns/property#\"\n\
               xmlns:pdfaType=\"http://www.aiim.org/pdfa/ns/type#\"\n\
               xmlns:pdfaField=\"http://www.aiim.org/pdfa/ns/field#\">\n\
             {properties}\n\
             </rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>"
        )
    }

    /// The Dublin Core schema is `dc`, and the packet's own prefix for it makes no difference.
    const DC: &str = "http://purl.org/dc/elements/1.1/";
    /// The XMP Dynamic Media schema, whose properties are the ones the corpus exercises most.
    const DM: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";

    /// ISO 19005-2 section 6.6.2.3.1: a predefined property carrying a value of another type has
    /// borrowed the namespace rather than used the schema.
    #[test]
    fn a_predefined_property_is_held_to_the_value_type_its_schema_gives_it() {
        // `xmpDM:logComment` is text and `xmpDM:projectRef` is a structure.
        let wrong = document(&schema_packet(
            "dm",
            DM,
            "<dm:logComment><rdf:Seq><rdf:li>a comment</rdf:li></rdf:Seq></dm:logComment>\n\
             <dm:projectRef>http://example.test/</dm:projectRef>",
        ));
        assert_eq!(found(properties_use_known_schemas, &wrong), 2);

        let right = document(&schema_packet(
            "dm",
            DM,
            "<dm:logComment>a comment</dm:logComment>\n\
             <dm:projectRef rdf:parseType=\"Resource\"><dm:type>custom</dm:type></dm:projectRef>",
        ));
        assert_eq!(found(properties_use_known_schemas, &right), 0);
    }

    /// The scalar half of the same rule, and the one property whose type the current edition of
    /// the XMP Specification widens.
    #[test]
    fn a_scalar_is_held_to_the_form_the_specification_spells_out() {
        // `tiff:ImageWidth` is an integer, `crs:AutoBrightness` a boolean spelled exactly.
        let wrong = document(&schema_packet(
            "tiff",
            "http://ns.adobe.com/tiff/1.0/",
            "<tiff:ImageWidth>256 mm</tiff:ImageWidth>",
        ));
        assert_eq!(found(properties_use_known_schemas, &wrong), 1);
        let lowercase = document(&schema_packet(
            "crs",
            "http://ns.adobe.com/camera-raw-settings/1.0/",
            "<crs:AutoBrightness>false</crs:AutoBrightness>",
        ));
        assert_eq!(found(properties_use_known_schemas, &lowercase), 1);

        // A rating is a real in the current edition of the specification, which section 6.6.2.3.1
        // cites undated — so both spellings of one are a value of the type.
        for rating in ["1", "1.0"] {
            let file = document(&schema_packet(
                "xmp",
                "http://ns.adobe.com/xap/1.0/",
                &format!("<xmp:Rating>{rating}</xmp:Rating>"),
            ));
            assert_eq!(found(properties_use_known_schemas, &file), 0, "{rating}");
        }
    }

    /// `TechNote 0010` A020: a `Rational` is one of the types validated as any string.
    ///
    /// `tiff:XResolution` is a rational, and this crate used to require the quotient the XMP
    /// Specification writes one as. The working group's list of the basic types puts the type
    /// beside `Text` and `URI` rather than beside `Integer` and `Real`, so a validator holding it
    /// to a form is validating more than the type — which is the whole of what A020 withdraws.
    #[test]
    fn a_rational_admits_any_string() {
        for value in ["72/1", "72", "seventy-two", "1/0"] {
            let file = document(&schema_packet(
                "tiff",
                "http://ns.adobe.com/tiff/1.0/",
                &format!("<tiff:XResolution>{value}</tiff:XResolution>"),
            ));
            assert_eq!(found(properties_use_known_schemas, &file), 0, "{value}");
        }

        // The shape of the value is still the type's, and A020 does not touch that: an ordered
        // array where the schema defines a simple value is not a rational spelled oddly.
        let shaped = document(&schema_packet(
            "tiff",
            "http://ns.adobe.com/tiff/1.0/",
            "<tiff:XResolution><rdf:Seq><rdf:li>72/1</rdf:li></rdf:Seq></tiff:XResolution>",
        ));
        assert_eq!(found(properties_use_known_schemas, &shaped), 1);
    }

    /// A language alternative's items are each defined to carry a language, which is what
    /// separates one from an alternative array of anything else.
    #[test]
    fn a_language_alternative_states_a_language_on_every_item() {
        let unqualified = document(&schema_packet(
            "dc",
            DC,
            "<dc:title><rdf:Alt><rdf:li>Report</rdf:li></rdf:Alt></dc:title>",
        ));
        assert_eq!(found(properties_use_known_schemas, &unqualified), 1);

        let qualified = document(&schema_packet(
            "dc",
            DC,
            "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Report</rdf:li></rdf:Alt></dc:title>",
        ));
        assert_eq!(found(properties_use_known_schemas, &qualified), 0);
    }

    /// A property in a schema nobody predefines needs an extension schema describing it —
    /// ISO 19005-2 section 6.6.2.3.2 — and this row is the one that says so.
    #[test]
    fn a_custom_property_needs_a_schema_description_and_the_description_satisfies_it() {
        let bare = document(&schema_packet(
            "ex",
            "http://example.test/ns/",
            "<ex:Serial>17</ex:Serial>",
        ));
        assert_eq!(found(extension_schemas_embedded, &bare), 1);
        assert_eq!(
            found(properties_use_known_schemas, &bare),
            0,
            "and the predefined-schema row says nothing about a schema nobody predefines"
        );

        let described = document(&schema_packet(
            "ex",
            "http://example.test/ns/",
            &format!("<ex:Serial>17</ex:Serial>\n{DESCRIPTION}"),
        ));
        assert_eq!(found(extension_schemas_embedded, &described), 0);
        assert_eq!(found(extension_schema_container_fields, &described), 0);
    }

    /// A complete extension schema description of `http://example.test/ns/`, with every field of
    /// ISO 19005-2 section 6.6.2.3.3's four tables.
    const DESCRIPTION: &str = r#"<pdfaExtension:schemas><rdf:Bag><rdf:li rdf:parseType="Resource">
      <pdfaSchema:schema>Example</pdfaSchema:schema>
      <pdfaSchema:namespaceURI>http://example.test/ns/</pdfaSchema:namespaceURI>
      <pdfaSchema:prefix>ex</pdfaSchema:prefix>
      <pdfaSchema:property><rdf:Seq><rdf:li rdf:parseType="Resource">
        <pdfaProperty:name>Serial</pdfaProperty:name>
        <pdfaProperty:valueType>Text</pdfaProperty:valueType>
        <pdfaProperty:category>internal</pdfaProperty:category>
        <pdfaProperty:description>A serial number</pdfaProperty:description>
      </rdf:li></rdf:Seq></pdfaSchema:property>
      <pdfaSchema:valueType><rdf:Seq><rdf:li rdf:parseType="Resource">
        <pdfaType:type>Machine</pdfaType:type>
        <pdfaType:namespaceURI>http://example.test/ns/machine#</pdfaType:namespaceURI>
        <pdfaType:prefix>mc</pdfaType:prefix>
        <pdfaType:description>A machine</pdfaType:description>
        <pdfaType:field><rdf:Seq><rdf:li rdf:parseType="Resource">
          <pdfaField:name>model</pdfaField:name>
          <pdfaField:valueType>Text</pdfaField:valueType>
          <pdfaField:description>The model</pdfaField:description>
        </rdf:li></rdf:Seq></pdfaType:field>
      </rdf:li></rdf:Seq></pdfaSchema:valueType>
    </rdf:li></rdf:Bag></pdfaExtension:schemas>"#;

    /// ISO 19005-2 section 6.6.2.3.3: every field of every table, spelled with the required prefix.
    #[test]
    fn a_container_schema_states_every_field_with_the_prefix_its_table_requires() {
        let whole = document(&schema_packet("ex", "http://example.test/ns/", DESCRIPTION));
        assert_eq!(found(extension_schema_container_fields, &whole), 0);

        // One field taken out of each of the four tables, found once each.
        for gone in [
            "<pdfaSchema:prefix>ex</pdfaSchema:prefix>",
            "<pdfaProperty:category>internal</pdfaProperty:category>",
            "<pdfaType:prefix>mc</pdfaType:prefix>",
            "<pdfaField:valueType>Text</pdfaField:valueType>",
        ] {
            let short = DESCRIPTION.replace(gone, "");
            let file = document(&schema_packet("ex", "http://example.test/ns/", &short));
            assert_eq!(
                found(extension_schema_container_fields, &file),
                1,
                "removing {gone}"
            );
        }

        // The prefix is required, so the same field under the same namespace under another
        // prefix is not the field the table describes.
        let renamed = schema_packet("ex", "http://example.test/ns/", DESCRIPTION)
            .replace("xmlns:pdfaSchema=", "xmlns:other=")
            .replace("pdfaSchema:", "other:");
        assert_eq!(
            found(extension_schema_container_fields, &document(&renamed)),
            5,
            "all five of Table 3's fields are spelled with a prefix the table does not name"
        );
    }

    /// The table is sorted and free of repeats, which is what makes a lookup in it answerable.
    #[test]
    fn every_predefined_schema_lists_its_properties_once() {
        for schema in PREDEFINED {
            let mut names: Vec<&str> = schema.properties.iter().map(|(name, _, _)| *name).collect();
            let stated = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), stated, "{} repeats a property", schema.name);
            assert!(!schema.uri.is_empty());
        }
    }

    /// The six date profiles the XMP Specification lists, and the shapes that are none of them.
    #[test]
    fn a_date_is_one_of_the_profiles_the_specification_lists() {
        for good in [
            "2016",
            "2016-02",
            "2016-02-01",
            "2016-02-01T13:19Z",
            "2016-02-01T13:19:21+01:00",
            "2016-02-01T13:19:21.5-06:00",
        ] {
            assert!(date(good), "{good}");
        }
        for bad in [
            "Date: 2016-02-01T13:19:21+01:00",
            "2016-02-01T13:19:21",
            "16-02-01",
            "2016-2-1",
            "2016-02-01T13:19:21.Z",
        ] {
            assert!(!date(bad), "{bad}");
        }
    }

    /// ISO 19005-2 section 6.6.4, read off a packet spelled the way a producer spells one.
    #[test]
    fn a_part_two_identification_is_read_and_its_level_is_checked_against_the_target() {
        let file = document(&packet(
            "http://www.aiim.org/pdfa/ns/id/",
            "<pdfaid:part>2</pdfaid:part>\n<pdfaid:conformance>B</pdfaid:conformance>",
        ));
        assert_eq!(found(catalog_metadata_stream, &file), 0);
        assert_eq!(found(xmp_packets_well_formed, &file), 0);
        assert_eq!(found(identification_part_two, &file), 0);
        assert_eq!(found(identification_conformance_level, &file), 0);
        assert_eq!(
            found(identification_declares_level_a, &file),
            1,
            "the file claims Level B, and the Level A row is the one that binds only 2a"
        );
        assert_eq!(
            found(identification_part_four, &file),
            1,
            "a part 2 claim is not a part 4 claim"
        );
    }

    /// The two parts print the schema's namespace with different schemes, and both are the schema.
    #[test]
    fn either_printing_of_the_namespace_names_the_same_schema() {
        for namespace in [
            "http://www.aiim.org/pdfa/ns/id/",
            "https://www.aiim.org/pdfa/ns/id/",
        ] {
            let file = document(&packet(
                namespace,
                "<pdfaid:part>4</pdfaid:part>\n<pdfaid:rev>2020</pdfaid:rev>",
            ));
            assert_eq!(found(identification_part_four, &file), 0, "{namespace}");
            assert_eq!(found(identification_revision_year, &file), 0, "{namespace}");
            assert_eq!(
                found(identification_states_no_flavour, &file),
                0,
                "{namespace}: a plain PDF/A-4 file states no conformance property"
            );
            assert_eq!(
                found(identification_declares_flavour_f, &file),
                1,
                "{namespace}: and therefore does not claim to be PDF/A-4f"
            );
        }
    }

    /// A revision that is not four digits is not the year ISO 19005-4 section 6.7.3 asks for.
    #[test]
    fn a_revision_year_is_four_digits_and_nothing_else() {
        let file = document(&packet(
            "http://www.aiim.org/pdfa/ns/id/",
            "<pdfaid:part>4</pdfaid:part>\n<pdfaid:rev>20</pdfaid:rev>",
        ));
        assert_eq!(found(identification_revision_year, &file), 1);
    }

    /// ISO 19005-4 section 6.7.3 as erratum #253 rewords it: the year of the revision, not any
    /// year.
    ///
    /// Part 4 has one revision and it was published in 2020, so a four-digit year that is not
    /// that one names no edition of the part that exists — which is why 2018 is refused here and
    /// was accepted before the erratum was read.
    #[test]
    fn a_revision_year_names_the_revision_the_part_actually_has() {
        let year = |stated: &str| {
            document(&packet(
                "http://www.aiim.org/pdfa/ns/id/",
                &format!("<pdfaid:part>4</pdfaid:part>\n<pdfaid:rev>{stated}</pdfaid:rev>"),
            ))
        };
        assert_eq!(found(identification_revision_year, &year("2020")), 0);
        assert_eq!(
            found(identification_revision_year, &year("2018")),
            1,
            "no revision of ISO 19005-4 is dated 2018"
        );
    }

    /// ISO 19005-2 section 6.6.4 and ISO 19005-4 section 6.7.3: the one prefix both parts make
    /// binding.
    ///
    /// The failing spelling is part 4's own Table 2, which erratum #123 records as an error: the
    /// table gives the conformance property a `pdfa` prefix in a schema whose required prefix the
    /// same table gives as `pdfaid`.
    #[test]
    fn the_identification_schema_is_spelled_with_the_prefix_its_subclause_requires() {
        let correct = document(&packet(
            "http://www.aiim.org/pdfa/ns/id/",
            "<pdfaid:part>4</pdfaid:part>\n<pdfaid:rev>2020</pdfaid:rev>",
        ));
        assert_eq!(found(identification_schema_prefix, &correct), 0);

        let wrong = document(
            "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\" \
             xmlns:pdfa=\"http://www.aiim.org/pdfa/ns/id/\" \
             pdfa:part=\"4\" pdfa:rev=\"2020\"/>\n\
             </rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>",
        );
        assert_eq!(
            found(identification_schema_prefix, &wrong),
            2,
            "both properties are in the identification namespace under the wrong prefix"
        );
    }

    /// The target an embedded file's own packet claims, which is how `document_level.rs` knows
    /// which of the six to hold it to.
    #[test]
    fn a_file_names_the_target_it_asks_to_be_judged_against() {
        let claiming =
            |properties: &str| document(&packet("http://www.aiim.org/pdfa/ns/id/", properties));
        assert_eq!(
            declared_target(&claiming("<pdfaid:part>4</pdfaid:part>")),
            Some(Target::Four(Flavour::Plain)),
            "section 6.7.3 reserves the conformance property for the two annexes, so stating none \
            is \
             the plain profile exactly"
        );
        assert_eq!(
            declared_target(&claiming(
                "<pdfaid:part>4</pdfaid:part>\n<pdfaid:conformance>F</pdfaid:conformance>"
            )),
            Some(Target::Four(Flavour::F))
        );
        assert_eq!(
            declared_target(&claiming(
                "<pdfaid:part>2</pdfaid:part>\n<pdfaid:conformance>A</pdfaid:conformance>"
            )),
            Some(Target::Two(Level::A))
        );
        assert_eq!(
            declared_target(&claiming("<pdfaid:part>2</pdfaid:part>")),
            Some(Target::Two(Level::B)),
            "an unqualified part 2 claim is read at the weakest level it could mean"
        );
        assert_eq!(
            declared_target(&claiming("<pdfaid:part>1</pdfaid:part>")),
            None,
            "part 1 is not a target, so a file claiming it is not judged here"
        );
        assert_eq!(declared_target(&document("")), None);
    }

    /// A file with no metadata stream fails the stream row and every claim that would rest on it.
    #[test]
    fn a_file_with_no_metadata_stream_makes_no_claim_at_all() {
        let file = document("");
        assert_eq!(found(catalog_metadata_stream, &file), 1);
        assert_eq!(found(identification_part_two, &file), 1);
        assert_eq!(found(identification_conformance_level, &file), 1);
        assert_eq!(
            found(xmp_packets_well_formed, &file),
            0,
            "there is no metadata stream to be malformed"
        );
        assert_eq!(
            found(identification_states_no_flavour, &file),
            0,
            "and a file claiming nothing has not claimed a flavour"
        );
    }

    /// A packet whose XML does not close is not well-formed, and the row that says so is the one
    /// about packets rather than the one about the catalog's entry.
    #[test]
    fn a_malformed_packet_is_caught_by_the_well_formedness_row() {
        let file = document("<?xpacket begin=\"\"?>\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">");
        assert_eq!(found(catalog_metadata_stream, &file), 0);
        assert_eq!(found(xmp_packets_well_formed, &file), 1);
    }

    /// The two deprecated header attributes, in each of the encodings ISO 16684-1 permits.
    #[test]
    fn the_deprecated_header_attributes_are_found_whatever_the_packet_is_encoded_in() {
        let clean = b"<?xpacket begin=\"\" id=\"W5M0Mp\"?><x:xmpmeta/>";
        let header = packet_header(clean).unwrap_or_default();
        assert!(!states_attribute(&header, b"bytes"));
        assert!(!states_attribute(&header, b"encoding"));

        let stated = b"<?xpacket begin=\"\" id=\"W5M0Mp\" bytes=\"120\" encoding=\"UTF-8\"?>";
        let header = packet_header(stated).unwrap_or_default();
        assert!(states_attribute(&header, b"bytes"));
        assert!(states_attribute(&header, b"encoding"));

        // The same header as UTF-16LE. Dropping the NUL padding is what makes one scan serve all
        // three of the encodings the XMP standard permits.
        let wide: Vec<u8> = stated.iter().flat_map(|byte| [*byte, 0]).collect();
        let header = packet_header(&wide).unwrap_or_default();
        assert!(states_attribute(&header, b"bytes"));

        // A value that merely contains the word is not the attribute.
        let innocent = b"<?xpacket begin=\"\" id=\"bytes and encoding\"?>";
        let header = packet_header(innocent).unwrap_or_default();
        assert!(!states_attribute(&header, b"bytes"));
        assert!(!states_attribute(&header, b"encoding"));
    }

    /// And the row wired to that scan reports both attributes of one packet separately.
    #[test]
    fn a_header_stating_both_attributes_is_reported_twice() {
        let file = document(
            "<?xpacket begin=\"\" bytes=\"7\" encoding=\"UTF-8\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\"></x:xmpmeta>\n<?xpacket end=\"w\"?>",
        );
        assert_eq!(found(xmp_packet_header_attributes, &file), 2);
    }

    /// A packet whose catalog metadata states one `xmpMM:History` entry with these fields.
    ///
    /// The shape is the one every producer writes and the one the corpus's own provenance
    /// witnesses use: a `rdf:Seq` of `rdf:parseType="Resource"` items whose fields are in the
    /// XMP Specification's `ResourceEvent` namespace.
    fn history(fields: &str) -> Document {
        document(&format!(
            "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
             <rdf:Description rdf:about=\"\" \
             xmlns:xmpMM=\"http://ns.adobe.com/xap/1.0/mm/\" \
             xmlns:stEvt=\"http://ns.adobe.com/xap/1.0/sType/ResourceEvent#\">\n\
             <xmpMM:History><rdf:Seq><rdf:li rdf:parseType=\"Resource\">\n{fields}\n\
             </rdf:li></rdf:Seq></xmpMM:History>\n\
             </rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>"
        ))
    }

    /// ISO 19005-4 section 6.7.5's two fields, over three shapes of recorded action.
    ///
    /// **The middle case used to be the whole difference between the parts** — an entry stating
    /// `action` and `when` and no `parameters` met part 4's list and missed one of part 2's — and
    /// there is no part 2 half to this test any more, because `TechNote 0010`'s A021 puts section
    /// 6.6.6 outside validation altogether. `a_clarified_row_reports_nothing_and_says_why` below
    /// is what replaced it.
    #[test]
    fn a_recorded_action_is_held_to_the_fields_part_four_requires() {
        let whole = history(
            "<stEvt:action>created</stEvt:action>\n\
             <stEvt:parameters>by hand</stEvt:parameters>\n\
             <stEvt:when>2016-04-05T13:19:21+01:00</stEvt:when>",
        );
        assert_eq!(found(provenance_recorded_action_fields_four, &whole), 0);

        let no_parameters = history(
            "<stEvt:action>created</stEvt:action>\n\
             <stEvt:when>2016-04-05T13:19:21+01:00</stEvt:when>",
        );
        assert_eq!(
            found(provenance_recorded_action_fields_four, &no_parameters),
            0,
            "part 4 only recommends it"
        );

        let neither = history("<stEvt:parameters>by hand</stEvt:parameters>");
        assert_eq!(
            found(provenance_recorded_action_fields_four, &neither),
            2,
            "action and when, reported separately"
        );
    }

    /// The row ISO 19005-2 section 6.6.6 states is carried, binds part 2, and reports nothing.
    ///
    /// All three halves matter and the test asserts each: a row that had been deleted would take
    /// the clause out of the verdict, a row that still checked would fail the files A021 says are
    /// not a validator's business, and a row whose reason did not name the record would leave a
    /// reader unable to tell a reading from a defect.
    #[test]
    fn a_clarified_row_reports_nothing_and_says_why() {
        let file = history("<stEvt:parameters>by hand</stEvt:parameters>");
        let report = crate::check(&file, Target::Two(Level::B));
        let judged = report
            .judgements
            .iter()
            .find(|judgement| judgement.id == "metadata/provenance-recorded-action-fields")
            .expect("part 2 states section 6.6.6, so the row is in every part 2 report");
        assert!(matches!(
            judged.outcome,
            crate::Outcome::OutsideValidation(_)
        ));
        assert_eq!(
            judged.clarified_by.map(|record| record.item),
            Some("A021"),
            "the verdict has to name the record that put the clause outside validation"
        );
        assert!(
            report.render().contains("outside validation"),
            "and the rendered report has to show it to a person"
        );
    }

    /// XML names are case-sensitive, and the `ResourceEvent` value type spells its fields in
    /// lower case — so a capital `When` states a field that value type does not define.
    ///
    /// The corpus witness `6-7-5-t01-pass-b.pdf` is exactly this construction.
    #[test]
    fn a_field_spelled_with_the_wrong_case_is_not_the_field_the_subclause_names() {
        let file = history(
            "<stEvt:action>created</stEvt:action>\n\
             <stEvt:When>2016-04-05T13:19:21+01:00</stEvt:When>",
        );
        assert_eq!(found(provenance_recorded_action_fields_four, &file), 1);
    }

    /// A file with no history at all has recorded no action, so the rule reaches nothing.
    #[test]
    fn a_file_that_records_no_action_meets_the_provenance_row_vacuously() {
        let file = document(&packet(
            "http://www.aiim.org/pdfa/ns/id/",
            "<pdfaid:part>4</pdfaid:part>",
        ));
        assert_eq!(found(provenance_recorded_action_fields_four, &file), 0);
    }

    /// ISO 19005-2 section 6.6.4's form for an amendment or corrigendum identifier that is stated.
    ///
    /// A file that states neither is not judged here — whether it should have is the row above
    /// this one, and nothing in the file settles it.
    #[test]
    fn an_amendment_identifier_is_a_number_and_a_year_around_a_colon() {
        let stated =
            |properties: &str| document(&packet("http://www.aiim.org/pdfa/ns/id/", properties));

        assert_eq!(
            found(
                identification_amendment_form,
                &stated("<pdfaid:part>2</pdfaid:part>"),
            ),
            0,
            "a file stating neither property"
        );
        assert_eq!(
            found(
                identification_amendment_form,
                &stated("<pdfaid:corr>1:2007</pdfaid:corr>"),
            ),
            0,
            "the form the subclause describes"
        );
        assert_eq!(
            found(
                identification_amendment_form,
                &stated("<pdfaid:amd>2016</pdfaid:amd>"),
            ),
            1,
            "a year alone is not a number and a year separated by a colon"
        );
        assert_eq!(
            found(
                identification_amendment_form,
                &stated("<pdfaid:amd>1:2016</pdfaid:amd>\n<pdfaid:corr>later</pdfaid:corr>"),
            ),
            1,
            "and the two properties are judged separately"
        );
    }
}
