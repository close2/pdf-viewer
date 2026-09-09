//! Everything after metadata: logical structure, embedded files, optional content, alternate
//! presentations, and the document requirements dictionary.
//!
//! ISO 19005-2 section 6.7 to section 6.11 and ISO 19005-4 section 6.8 to section 6.15 — five
//! subclauses in one part and eight in the other, and the mismatch is the interesting thing about
//! this tranche.
//!
//! # Where the two parts part company
//!
//! - **Logical structure is part 2's alone.** Section 6.7 is a full subclause of eight parts and
//!   applies only at Level A, which is why every row in that area carries
//!   `Applies::FromLevel(Level::A)`. ISO 19005-4 section 6.8 replaces the whole of it with two
//!   sentences of encouragement and **states no requirement at all** — not one `shall` — so those
//!   rows are `Clauses::only_two`. Part 2's own subclause is mostly recommendation too: of
//!   everything section 6.7 says, six sentences are `shall` and every one of the things a reader
//!   would call accessibility — alternate descriptions, replacement text, expansions, the default
//!   language — is `should`. Six sentences, six rows.
//! - **Part 4 requires more of an embedded file than part 2 does**, adding `/AFRelationship` to
//!   part 2's `/F` and `/UF`, and then Annexes A and B take the type restriction back off again
//!   for PDF/A-4f and PDF/A-4e. So the row that asks an embedded file to be a PDF/A file binds
//!   the plain profile alone.
//! - **`/AS` in an optional content configuration is forbidden by part 2 and merely ignored by
//!   part 4**, which is the one rule in this tranche a file can fail under one part and pass under
//!   the other with no other change.
//!
//! # The clauses that bind a processor, and why they are rows after all
//!
//! **A clause whose only requirement is addressed to a processor used to get no row here**, on the
//! reasoning that this crate judges a file and no file can fail such a rule. The reasoning was
//! right and the conclusion was wrong: a requirement named only in a module header is invisible to
//! every reader of a verdict, which is the silent-omission failure `doc/questions/Q20` exists to
//! prevent, and it is indistinguishable from a rule nobody noticed. [`Check::Processor`] is the
//! answer — carried, named, reported in its own section, and outside the coverage denominator
//! because it is neither met nor owed by a document:
//!
//! - the rendering of optional content in the default configuration, the display of the `/Order`
//!   array and of the list of configurations, and the instruction not to use `/Intent` — ISO
//!   19005-2 section 6.9 and ISO 19005-4 section 6.10, with part 4's instruction to ignore `/AS`
//!   beside them;
//! - the display of the names of embedded files — ISO 19005-2 section 6.8 and ISO 19005-4 section
//!   6.9;
//! - the instruction to ignore `/Trans` and `/Dur` — ISO 19005-2 section 6.10 and ISO 19005-4
//!   section 6.11;
//! - **the whole of ISO 19005-4 section 6.13**, whose subject is what a processor does about
//!   `/PrintScaling` and `/Enforce` when printing. It places nothing on a file.
//!
//! **ISO 19005-4 section 6.14 and section 6.15 place nothing on anybody.** Each is a single
//! sentence permitting a conforming file to carry the base standard's geospatial information
//! (section 6.14) or measurement properties (section 6.15) by any of the mechanisms ISO 32000-2
//! describes. A permission is not a requirement, and inventing a row for one would put a rule in
//! this table that the standard does not state — so those two, and only those two, are still
//! absent.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::sync::Arc;

use pdf_model::Pages;
use pdf_model::structure::{Child, MarkInfo, Tree, document_language};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::Examination;
use crate::finding::{Findings, Where};
use crate::requirement::{Applies, Check, Clauses, Requirement};
use crate::target::{Flavour, Level};

/// The rows this module contributes, which `super::TRANCHES` concatenates.
pub(super) static REQUIREMENTS: &[Requirement] = &[
    Requirement {
        id: "logical-structure/tagged-pdf",
        asks: "A Level A file shall meet every requirement the base standard sets for Tagged PDF.",
        clauses: Clauses::only_two("6.7.2.1"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Unchecked(
            "the requirement is a whole clause family of the base standard rather than one rule; \
             the parts of it this tree can ask are the rows beneath it",
        ),
    },
    Requirement {
        id: "logical-structure/mark-info-marked",
        asks: "The document catalog shall state a MarkInfo dictionary whose Marked entry is true.",
        clauses: Clauses::only_two("6.7.2.2"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Implemented(mark_info_marked),
    },
    Requirement {
        id: "logical-structure/word-boundaries",
        asks: "In a script that separates words with space characters, the shown strings shall \
               put a space character at every word boundary.",
        clauses: Clauses::only_two("6.7.3.2"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Unchecked(
            "asking it needs a walk of every page's text-showing operators and a decision, per \
             run, about whether its script separates words that way — neither of which this \
             crate reaches from a dictionary",
        ),
    },
    Requirement {
        id: "logical-structure/structure-tree-root",
        asks: "The document catalog shall state a StructTreeRoot describing the file's logical \
               structure.",
        clauses: Clauses::only_two("6.7.3.3"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Implemented(structure_tree_root),
    },
    Requirement {
        id: "logical-structure/role-map-terminates-at-a-standard-type",
        asks: "Every non-standard structure type shall be mapped through the role map, directly \
               or through further non-standard types, to a standard structure type.",
        clauses: Clauses::only_two("6.7.3.4"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Implemented(role_map_terminates_at_a_standard_type),
    },
    Requirement {
        id: "logical-structure/catalog-language-identifier",
        asks: "Where the document catalog states a Lang entry, its value shall be a language \
               identifier the base standard defines.",
        clauses: Clauses::only_two("6.7.4"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Implemented(catalog_language_identifier),
    },
    Requirement {
        id: "logical-structure/element-and-property-list-language-identifiers",
        asks: "Where a structure element or a marked-content property list states a Lang entry, \
               its value shall be a language identifier the base standard defines.",
        clauses: Clauses::only_two("6.7.4"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Implemented(element_language_identifiers),
    },
    Requirement {
        id: "embedded-files/file-and-unicode-names",
        asks: "Every file specification carrying an embedded file shall state both the F and the \
               UF keys.",
        clauses: Clauses::both("6.8", "6.9"),
        applies: Applies::Always,
        check: Check::Implemented(file_and_unicode_names),
    },
    Requirement {
        id: "embedded-files/associated-file-media-type",
        asks: "An embedded file stream the document associates with one of its objects shall \
               state a Subtype that is a MIME media type.",
        clauses: Clauses::only_four("6.9"),
        applies: Applies::Always,
        check: Check::Implemented(associated_file_media_type),
    },
    Requirement {
        id: "embedded-files/relationship-stated",
        asks: "Every file specification carrying an embedded file shall state an AFRelationship \
               saying how the file relates to the document.",
        clauses: Clauses::only_four("6.9"),
        applies: Applies::Always,
        check: Check::Implemented(relationship_stated),
    },
    Requirement {
        id: "embedded-files/embedded-file-is-itself-pdfa",
        asks: "Every embedded file shall itself conform to a part of ISO 19005.",
        clauses: Clauses::only_two("6.8"),
        applies: Applies::Always,
        check: Check::Implemented(embedded_file_is_itself_pdfa),
    },
    Requirement {
        id: "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
        asks: "Every embedded file shall itself conform to a part of ISO 19005 — which Annex A \
               lifts for PDF/A-4f and Annex B for PDF/A-4e, both letting an embedded file be of \
               any type.",
        clauses: Clauses::only_four("6.9"),
        applies: Applies::Flavours(&[Flavour::Plain]),
        check: Check::Implemented(embedded_file_is_itself_pdfa),
    },
    Requirement {
        id: "embedded-files/names-displayable",
        asks: "A conforming interactive processor shall offer a way to display the name strings \
               the EmbeddedFiles name tree states.",
        clauses: Clauses::both("6.8", "6.9"),
        applies: Applies::Always,
        check: Check::Processor(
            "the sentence is addressed to the processor showing the file, not to the file: a \
             document carrying an EmbeddedFiles tree has done everything it can towards it",
        ),
    },
    Requirement {
        id: "embedded-files/pdfa-4f-carries-embedded-files",
        asks: "A PDF/A-4f file shall state an EmbeddedFiles key in the name dictionary of its \
               document catalog.",
        clauses: Clauses::only_four("A.2"),
        applies: Applies::Flavours(&[Flavour::F]),
        check: Check::Implemented(pdfa_4f_carries_embedded_files),
    },
    Requirement {
        id: "optional-content/configuration-names",
        asks: "Every optional content configuration dictionary shall state a Name, and no two of \
               them shall state the same one.",
        clauses: Clauses::both("6.9", "6.10"),
        applies: Applies::Always,
        check: Check::Implemented(configuration_names),
    },
    Requirement {
        id: "optional-content/order-lists-every-group",
        asks: "Where an optional content configuration states an Order, that array shall \
               reference every optional content group in the file.",
        clauses: Clauses::both("6.9", "6.10"),
        applies: Applies::Always,
        check: Check::Implemented(order_lists_every_group),
    },
    Requirement {
        id: "optional-content/no-automatic-states",
        asks: "No optional content configuration dictionary shall state the AS key, which part 4 \
               permits and has its processors ignore instead.",
        clauses: Clauses::only_two("6.9"),
        applies: Applies::Always,
        check: Check::Implemented(no_automatic_states),
    },
    Requirement {
        id: "optional-content/automatic-states-ignored",
        asks: "A conforming processor shall ignore an AS key in an optional content \
               configuration dictionary — part 4's replacement for part 2's prohibition of it.",
        clauses: Clauses::only_four("6.10"),
        applies: Applies::Always,
        check: Check::Processor(
            "part 4 permits the entry outright and puts the restriction on what a processor does \
             with it, so a file that states one has broken nothing",
        ),
    },
    Requirement {
        id: "optional-content/default-configuration-rendered",
        asks: "Absent instructions to the contrary, a conforming processor shall render the file \
               in the default state the OCProperties dictionary's D key sets.",
        clauses: Clauses::both("6.9", "6.10"),
        applies: Applies::Always,
        check: Check::Processor(
            "a rule about which variant a processor shows, and no property of a document decides \
             it; the state the file has to offer is the D key the rows above judge",
        ),
    },
    Requirement {
        id: "optional-content/order-and-configurations-displayable",
        asks: "A conforming interactive processor shall offer a way to display the Order key of \
               every configuration, and the list of configurations to choose among.",
        clauses: Clauses::both("6.9", "6.10"),
        applies: Applies::Always,
        check: Check::Processor(
            "an obligation on the interactive processor's user interface; the file's half of it \
               is the Order array the row above judges",
        ),
    },
    Requirement {
        id: "optional-content/intent-not-used",
        asks: "A conforming processor shall not use the value of an optional content group's \
               Intent key.",
        clauses: Clauses::both("6.9", "6.10"),
        applies: Applies::Always,
        check: Check::Processor(
            "both parts leave the entry permitted and forbid the processor to act on it, so no \
             document can fail the sentence",
        ),
    },
    Requirement {
        id: "alternate-presentations/none-in-the-name-dictionary",
        asks: "The document's name dictionary shall state no AlternatePresentations key.",
        clauses: Clauses::both("6.10", "6.11"),
        applies: Applies::Always,
        check: Check::Implemented(no_alternate_presentations),
    },
    Requirement {
        id: "alternate-presentations/no-presentation-steps",
        asks: "No page dictionary shall state a PresSteps key.",
        clauses: Clauses::both("6.10", "6.11"),
        applies: Applies::Always,
        check: Check::Implemented(no_presentation_steps),
    },
    Requirement {
        id: "alternate-presentations/transitions-ignored",
        asks: "A conforming interactive processor shall ignore the Trans and Dur keys a page \
               dictionary states.",
        clauses: Clauses::both("6.10", "6.11"),
        applies: Applies::Always,
        check: Check::Processor(
            "both parts leave the two entries permitted and tell the processor to disregard \
             them, which is the opposite of a rule a file could break",
        ),
    },
    Requirement {
        id: "document-requirements/no-requirements-dictionary",
        asks: "The document catalog shall state no Requirements key.",
        clauses: Clauses::both("6.11", "6.12"),
        applies: Applies::Always,
        check: Check::Implemented(no_requirements_dictionary),
    },
    Requirement {
        id: "print-scaling/print-scaling-obeyed",
        asks: "A conforming processor shall obey the viewer preferences dictionary's PrintScaling \
               key, and an Enforce array naming PrintScaling shall make it binding on every page.",
        clauses: Clauses::only_four("6.13"),
        applies: Applies::Always,
        check: Check::Processor(
            "the whole subclause is about what a processor does when it prints; it places nothing \
             on the file, which is free to state either key or neither",
        ),
    },
];

/// ISO 19005-2 section 6.7.2.2.
fn mark_info_marked(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if !MarkInfo::read(document).marked {
        findings.record(
            Where::file().named("MarkInfo"),
            "the catalog does not state a MarkInfo dictionary with Marked true",
        );
    }
}

/// ISO 19005-2 section 6.7.3.3.
fn structure_tree_root(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if Tree::of(document).is_none() {
        findings.record(
            Where::file().named("StructTreeRoot"),
            "the catalog states no structure tree root, so the file describes no logical structure",
        );
    }
}

/// ISO 19005-2 section 6.7.3.4.
///
/// `pdf_model::structure::Tree::role` follows the role map to a fixed point and
/// `Tree::standard_role` answers what the name it ends at *means*, so a `Some` role with a `None`
/// standard role is exactly a mapping that did not terminate at a standard type.
///
/// Two deliberate leniencies, both in the document's favour:
///
/// - An element stating no type at all is passed over. Table 355 of the base standard makes the
///   entry required, but that is a different requirement from this one, and reporting it here
///   would put a finding under a clause that is not about it.
/// - `StandardType` is ISO 32000-2's vocabulary, which is a superset of the one ISO 19005-2's own
///   base document defines. So a part 2 file mapping to a type PDF 2.0 added is accepted; the
///   alternative is refusing a document over the edition of the base standard this tree owns,
///   which `crate::target::Part::Two` already names as the limitation it is.
fn role_map_terminates_at_a_standard_type(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Some(tree) = Tree::of(document) else {
        return;
    };
    for (_, child) in tree.walk(document).items {
        let Child::Element(element) = child else {
            continue;
        };
        let Some(role) = tree.role(document, &element) else {
            continue;
        };
        if tree.standard_role(document, &element).is_none() {
            findings.record(
                Where::file().named(role),
                "a structure type is mapped to no standard structure type",
            );
        }
    }
}

/// ISO 19005-2 section 6.7.4, which is one `shall` inside a subclause of recommendations.
///
/// Everything a reader would call the accessibility of language is `should` there — the default
/// `/Lang` on the catalog, a `/Lang` wherever the text departs from it, the escape sequence inside
/// a Unicode string — and a row for any of those would be this table asserting a rule the standard
/// does not state. What the subclause *requires* is that a `/Lang` which is present be a language
/// identifier as the base standard defines one, and §14.9.2.2 defines that:
///
/// > A language identifier shall either be the empty text string, to indicate that the
/// > language is unknown, or a Language-Tag as defined in BCP 47.
///
/// So three things pass and everything else fails. `pdf_model::structure::document_language`
/// already applies exactly that test to the catalog's entry — it answers `None` for an absent, an
/// empty, or an ill-formed tag — which makes "the entry is there and not empty, and that reader
/// still answers `None`" the failure the sentence describes, without a second BCP 47 grammar in
/// this crate.
///
/// An entry that is not a text string at all is reported too: §14.9.2.2 makes a language
/// identifier a text string, so a name or a number there is not one.
/// ISO 19005-2 section 6.7.4's `shall`, at every structure element that states a `/Lang`.
///
/// The same sentence as `catalog_language_identifier`, applied where §14.9.2's inheritance
/// actually begins: the clause makes the requirement about a `/Lang` *wherever* it is present,
/// and a document whose catalog is silent may still state a malformed one on an element.
///
/// **Judged with `pdf-model`'s own grammar rather than a second one.** `well_formed_language_tag`
/// was made public for this: RFC 5646's ABNF and its grandfathered list are ninety lines that
/// this project should not have two of, and `pdf-archive` states that it adds no reader of its
/// own.
///
/// # The second place a `/Lang` stands
///
/// A marked-content property list is the other, and the clause names it beside the structure
/// element. It is not an object where a producer wrote it into a `BDC` or `DP` operator's
/// operands (ISO 32000-2 §14.6.2), so the content survey is what sees it —
/// [`crate::survey::Survey::marked_languages`] records both forms, the inline one and the one
/// named through the resources' `/Properties`, and this reads them under the same sentence as
/// the elements. The corpus witness is `6-7-4-t01-fail-c`, whose primary language subtag is
/// Cyrillic inside an inline property list.
fn element_language_identifiers(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for stated in exam.survey().marked_languages() {
        match &stated.value {
            Err(kind) => findings.record(
                Where::page(stated.page).named((*kind).to_owned()),
                "a marked-content property list's Lang is not a text string",
            ),
            Ok(bytes) => {
                let text = pdf_syntax::text_string(bytes);
                // The empty text string is §14.9.2.2's identifier for an unknown language.
                if text.is_empty() || pdf_model::structure::well_formed_language_tag(&text) {
                    continue;
                }
                findings.record(
                    Where::page(stated.page).named(text),
                    "a marked-content property list's Lang is not a language identifier the \
                     base standard defines",
                );
            }
        }
    }
    let Some(tree) = Tree::of(document) else {
        return;
    };
    for (_, child) in tree.walk(document).items {
        let Child::Element(element) = child else {
            continue;
        };
        let stated = document.get_key(&element, "Lang");
        if stated.is_null() {
            continue;
        }
        let Some(text) = stated.as_string().map(pdf_syntax::text_string) else {
            findings.record(
                Where::file().named(stated.type_name().to_owned()),
                "a structure element's Lang is not a text string",
            );
            continue;
        };
        // The empty text string is §14.9.2.2's identifier for an unknown language, and is not a
        // malformed tag — the same carve-out the catalog rule makes.
        if text.is_empty() || pdf_model::structure::well_formed_language_tag(&text) {
            continue;
        }
        findings.record(
            Where::file().named(text),
            "a structure element's Lang is not a language identifier the base standard defines",
        );
    }
}

fn catalog_language_identifier(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let stated = document.get_key(&catalog, "Lang");
    // Absent is not a failure: the subclause only *recommends* that a file state a default.
    if stated.is_null() {
        return;
    }
    let text = stated.as_string().map(pdf_syntax::text_string);
    // The empty text string is the identifier §14.9.2.2 gives to an unknown language.
    if text.as_ref().is_some_and(String::is_empty) {
        return;
    }
    if document_language(document).is_none() {
        findings.record(
            Where::file().named(text.unwrap_or_else(|| stated.type_name().to_owned())),
            "the catalog's Lang is not a language identifier the base standard defines",
        );
    }
}

/// Every file specification in the document that carries an embedded file.
///
/// The population is the indirect objects the cross-reference table names — the same bound
/// `file_structure.rs` walks under, and the same one both parts' own exemption for an object no
/// cross-reference section names describes — plus any specification the `/EmbeddedFiles` tree
/// files as a *direct* dictionary, which no object number would reach.
///
/// It is the union of the two populations the two parts describe: part 2 states its rule of the
/// file specification of an embedded file wherever it is, and part 4 states its rules of the
/// specifications the `/EmbeddedFiles` tree names. The union is what part 2 asks for, and part 4's
/// own opening sentence puts requirements on embedded files rather than on the tree, so holding
/// the wider population to it is the reading that does not depend on where a producer filed the
/// file.
fn for_each_embedded_file_specification(
    exam: &Examination<'_>,
    mut visit: impl FnMut(Where, &Dictionary),
) {
    let document = exam.document;
    for (id, object) in exam.objects() {
        let id = *id;
        if let Object::Dictionary(specification) = object
            && document.get_key(specification, "EF").as_dict().is_some()
        {
            visit(Where::object(id), specification);
        }
    }

    for (key, value) in embedded_files_entries(document) {
        // The indirect ones are the loop above; what is left is a leaf the producer wrote out in
        // place, which has no object number for a report to name.
        if value.as_reference().is_some() {
            continue;
        }
        if let Some(specification) = value.as_dict()
            && document.get_key(specification, "EF").as_dict().is_some()
        {
            visit(
                Where::file().named(pdf_syntax::text_string(&key)),
                specification,
            );
        }
    }
}

/// The `/EmbeddedFiles` name tree's leaves, each as the leaf states it.
///
/// Unresolved, because [`for_each_embedded_file_specification`] needs to tell a reference from a
/// dictionary written in place, and resolving would throw that distinction away.
fn embedded_files_entries(document: &Document) -> Vec<(Vec<u8>, Object)> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let names = document.get_key(&catalog, "Names");
    let Some(names) = names.as_dict() else {
        return Vec::new();
    };
    let embedded = document.get_key(names, "EmbeddedFiles");
    let Some(embedded) = embedded.as_dict() else {
        return Vec::new();
    };
    pdf_syntax::tree::name_entries(embedded, &|object| document.resolve(object))
}

/// ISO 19005-2 section 6.8, ISO 19005-4 section 6.9.
fn file_and_unicode_names(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_embedded_file_specification(exam, |place, specification| {
        for key in ["F", "UF"] {
            if document.get_key(specification, key).is_null() {
                findings.record(
                    place.clone().named(key),
                    format!("an embedded file's specification states no {key} key"),
                );
            }
        }
    });
}

/// ISO 19005-4 section 6.9, by way of the base standard that its section 5.1 makes binding.
///
/// **Part 4 states no rule about media types, and one binds anyway.** Section 5.1 says a conforming
/// file adheres to every requirement of ISO 32000-2 as part 4 modifies it, and part 4 modifies
/// nothing here — so §14.13.2's sentence about an embedded file stream used as an associated file
/// is a requirement of a PDF/A-4 file, cited under the subclause that is *about* embedded files.
/// That is the same construction `metadata/catalog-metadata-stream` uses for Table 347's entries.
///
/// > The embedded file stream dictionary shall include a valid MIME type value for the Subtype
/// > key.
///
/// The sentence after it names `application/octet-stream` as the value to use where the type is
/// not known, which is a producer's instruction rather than a second condition on the file.
///
/// **The population is the associated files and not every embedded file**, which is §14.13.1's
/// own distinction: an associated file is one an object's `AF` array names, and the
/// `AFRelationship` key part 4 section 6.9 requires of every embedded file is described there as
/// something such a specification *should* include rather than as what makes it one. Table 44
/// agrees from the other side, making `Subtype` optional in general and required of an embedded
/// file stream used as an associated file.
///
/// What counts as valid is read no further than Internet RFC 2046, which Table 44 cites for the
/// names: a media type is a top-level type and a subtype, so a value carrying no solidus has named
/// no media type. The seven top-level types that RFC lists are deliberately *not* required — ISO
/// 32000-2 itself names `model/u3d`, `model/prc` and `model/step` for 3D streams, and `model` is
/// registered outside that list.
fn associated_file_media_type(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let associated = associated_specifications(exam);
    for_each_embedded_file_specification(exam, |place, specification| {
        if !place.object.is_some_and(|id| associated.contains(&id)) {
            return;
        }
        let files = document.get_key(specification, "EF");
        let Some(files) = files.as_dict() else {
            return;
        };
        for key in EMBEDDED_FILE_STREAM_KEYS {
            let stream = document.get_key(files, key);
            let Some(stream) = stream.as_stream() else {
                continue;
            };
            let stated = document.get_key(&stream.dict, "Subtype");
            let named = stated.as_name().map(|name| name.as_bytes().to_vec());
            if named.as_deref().is_some_and(is_media_type) {
                continue;
            }
            findings.record(
                place.clone().named("Subtype"),
                match named {
                    Some(name) => format!(
                        "an associated file's stream states {} as its Subtype, which is not a \
                         MIME media type",
                        String::from_utf8_lossy(&name)
                    ),
                    None => "an associated file's stream states no Subtype media type".to_owned(),
                },
            );
        }
    });
}

/// Whether a name is a MIME media type as Internet RFC 2046 composes one: a type and a subtype.
///
/// PDF writes the solidus in a name as `#2F` and this reader has already decoded it, so what
/// arrives here is the media type as the RFC spells it.
fn is_media_type(name: &[u8]) -> bool {
    let mut halves = name.splitn(2, |byte| *byte == b'/');
    let (Some(top), Some(sub)) = (halves.next(), halves.next()) else {
        return false;
    };
    let token = |part: &[u8]| {
        !part.is_empty()
            && part
                .iter()
                .all(|byte| byte.is_ascii_graphic() && *byte != b'/')
    };
    token(top) && token(sub)
}

/// Every file specification an `AF` array names, by object number.
///
/// §14.13.1 lists eight kinds of dictionary that may carry the key and says the array is what
/// connects a specification to the object it is associated with, so the population is taken from
/// every object rather than from that list: a walk that named the eight would miss a ninth the
/// standard adds, and there is nothing else an `AF` array can mean.
fn associated_specifications(exam: &Examination<'_>) -> BTreeSet<ObjectId> {
    let document = exam.document;
    let mut out = BTreeSet::new();
    for (_, object) in exam.objects() {
        let dictionary = match object {
            Object::Dictionary(dictionary) => dictionary,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        let listed = document.get_key(dictionary, "AF");
        let Some(items) = listed.as_array() else {
            continue;
        };
        out.extend(items.iter().filter_map(Object::as_reference));
    }
    out
}

/// ISO 19005-4 section 6.9.
fn relationship_stated(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_embedded_file_specification(exam, |place, specification| {
        if document
            .get_key(specification, "AFRelationship")
            .as_name()
            .is_none()
        {
            findings.record(
                place.named("AFRelationship"),
                "an embedded file's specification states no AFRelationship",
            );
        }
    });
}

/// ISO 32000-2 Table 43's keys in an `/EF` dictionary, each of which is an embedded file stream.
///
/// All five rather than the usual two, because the rule is about the *file* that is embedded and a
/// producer filing it under `/DOS` has embedded it just as much as one filing it under `/F`.
static EMBEDDED_FILE_STREAM_KEYS: &[&str] = &["F", "UF", "DOS", "Mac", "Unix"];

/// How deep a chain of embedded documents this rule follows.
///
/// Judging an embedded file means running this crate's whole table over it, and that table
/// contains this rule — so a document embedding a document embedding a document is a recursion,
/// and a hostile one is unbounded. `CLAUDE.md` principle 3 asks for an explicit budget rather than
/// a stack that runs out: two levels is past every document anybody has written on purpose and
/// short of anything a bomb could exploit.
const MAX_EMBEDDING_DEPTH: usize = 2;

thread_local! {
    /// How many embedded documents deep the report now running is.
    ///
    /// Not a field on [`Examination`], because it is a property of the *chain* of reports rather
    /// than of any one of them: each embedded document is examined in its own right, and what has
    /// to be bounded is how many of those examinations are stacked on top of each other.
    static EMBEDDING_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// Raises [`EMBEDDING_DEPTH`] for as long as it is held.
///
/// A guard rather than a pair of statements so that the count comes back down by whichever route
/// the recursive report leaves — a counter that could be left raised would silently switch this
/// rule off for the rest of the thread, which is worse than the recursion it guards against.
struct Descent;

impl Descent {
    /// Descends one level, or answers `None` at the budget.
    fn one_level() -> Option<Self> {
        let depth = EMBEDDING_DEPTH.get();
        (depth < MAX_EMBEDDING_DEPTH).then(|| {
            EMBEDDING_DEPTH.set(depth.saturating_add(1));
            Self
        })
    }
}

impl Drop for Descent {
    fn drop(&mut self) {
        EMBEDDING_DEPTH.set(EMBEDDING_DEPTH.get().saturating_sub(1));
    }
}

/// ISO 19005-2 section 6.8, ISO 19005-4 section 6.9: an embedded file has to be a conforming file
/// itself.
///
/// **Two of the three answers a file can give are decidable here, and the third is not.** The
/// bytes either open as a PDF document or they do not, and a thing that is not a PDF conforms to no
/// part of ISO 19005 — that is the whole of the first half, and it needs nothing but `pdf_syntax`.
/// Beyond it, an embedded *document* is held to the part **it declares**: both parts this crate
/// owns require a conforming file to state its part number (ISO 19005-2 section 6.6.4, ISO 19005-4
/// Section 6.7.3), so a file's own identification schema says which target it is asking to be
/// judged against, and `metadata::declared_target` reads it.
///
/// What is left undecided is written down rather than hidden, and it is the part-1 case: an
/// embedded file declaring ISO 19005-1 is passed over, because part 1 is not a target
/// (`doc/questions/A17`) and refusing it would be refusing a document over a text this project
/// does not hold. A file declaring nothing is passed over for the weaker version of the same
/// reason — it may be a part 1 or part 3 file, and this crate cannot tell.
///
/// The rule is also **wider than part 2's sentence in one direction and narrower in another**, and
/// deliberately: section 6.8 admits only ISO 19005-1 and part 2 where section 6.9 admits parts 1, 2
/// and 4, so an embedded PDF/A-4 file inside a PDF/A-2 document is held to part 4 here and passes,
/// where the clause would refuse it for its part number alone. That is a rule left unimplemented
/// rather than implemented wrongly; the row above states the requirement in full.
///
/// A stream this reader cannot decode is passed over in silence rather than reported. A filter
/// `pdf_syntax` does not decode is a fact about this program, and recording it as a fault would be
/// failing a document over the reader in front of it.
fn embedded_file_is_itself_pdfa(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_embedded_file_specification(exam, |place, specification| {
        let files = document.get_key(specification, "EF");
        let Some(files) = files.as_dict() else {
            return;
        };
        let mut seen = BTreeSet::new();
        for key in EMBEDDED_FILE_STREAM_KEYS {
            let entry = files.get(key);
            // One stream filed under two keys is one embedded file, and reporting it twice would
            // count two faults against a document that has one.
            if let Some(entry) = entry
                && let Some(id) = entry.as_reference()
                && !seen.insert(id)
            {
                continue;
            }
            let object = document.get_key(files, key);
            let Some(stream) = object.as_stream() else {
                continue;
            };
            let Some(bytes) = document.decoded_stream_data(stream) else {
                continue;
            };
            judge_embedded(&place.clone().named((*key).to_owned()), bytes, findings);
        }
    });
}

/// One embedded file's bytes, held to whatever part of ISO 19005 they claim.
fn judge_embedded(place: &Where, bytes: Arc<[u8]>, findings: &mut Findings) {
    let Ok(embedded) = Document::open(bytes) else {
        findings.record(
            place.clone(),
            "an embedded file does not open as a PDF document, so it conforms to no part of \
             ISO 19005",
        );
        return;
    };
    let Some(target) = super::metadata::declared_target(&embedded) else {
        return;
    };
    let Some(_descent) = Descent::one_level() else {
        return;
    };
    let report = crate::check(&embedded, target);
    if let Some(failure) = report.failures().next() {
        findings.record(
            place.clone(),
            format!(
                "an embedded file declares {target} and does not conform to it: {}",
                failure.id
            ),
        );
    }
}

/// ISO 19005-4 section A.2, which is the one requirement Annex A adds rather than relaxes.
fn pdfa_4f_carries_embedded_files(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let present = document.catalog().is_ok_and(|catalog| {
        let names = document.get_key(&catalog, "Names");
        names
            .as_dict()
            .is_some_and(|names| document.get_key(names, "EmbeddedFiles").as_dict().is_some())
    });
    if !present {
        findings.record(
            Where::file().named("EmbeddedFiles"),
            "the name dictionary states no EmbeddedFiles tree, so the file embeds nothing",
        );
    }
}

/// Every optional content configuration dictionary the subclause governs: the default one and
/// each element of `/Configs`.
///
/// The order is the clause's own — `/D` first, then the array — so a report reads in the order a
/// person checking the document would look.
fn for_each_configuration(document: &Document, mut visit: impl FnMut(Where, &Dictionary)) {
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let properties = document.get_key(&catalog, "OCProperties");
    let Some(properties) = properties.as_dict() else {
        return;
    };

    let place = |entry: &Object, name: &str| match entry.as_reference() {
        Some(id) => Where::object(id),
        None => Where::file().named(name),
    };

    if let Some(entry) = properties.get("D") {
        let resolved = document.resolve(entry);
        if let Some(configuration) = resolved.as_dict() {
            visit(place(entry, "D"), configuration);
        }
    }

    let configs = document.get_key(properties, "Configs");
    if let Some(configs) = configs.as_array() {
        for entry in configs {
            let resolved = document.resolve(entry);
            if let Some(configuration) = resolved.as_dict() {
                visit(place(entry, "Configs"), configuration);
            }
        }
    }
}

/// ISO 19005-2 section 6.9, ISO 19005-4 section 6.10.
///
/// Uniqueness is compared over the *text* each `/Name` decodes to rather than over its bytes,
/// because that is what makes two configurations tell a person apart — the entry is a text string,
/// and two strings that show the same characters have not identified two variants.
fn configuration_names(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for_each_configuration(document, |place, configuration| {
        let stated = document.get_key(configuration, "Name");
        let Some(bytes) = stated.as_string() else {
            findings.record(
                place.named("Name"),
                "an optional content configuration states no Name",
            );
            return;
        };
        let name = pdf_syntax::text_string(bytes);
        if !seen.insert(name.clone()) {
            findings.record(
                place.named(name),
                "two optional content configurations state the same Name",
            );
        }
    });
}

/// How deep an `/Order` array may nest before this walk stops.
///
/// The clause lets the array be a hierarchy rather than a flat list and states no depth; this is a
/// bound on what a hostile document can make this walk do, not a reading (principle 3).
const MAX_ORDER_DEPTH: usize = 32;

/// Every optional content group an `/Order` array references, however deeply nested.
fn order_references(
    document: &Document,
    entries: &[Object],
    depth: usize,
    out: &mut BTreeSet<ObjectId>,
) {
    if depth >= MAX_ORDER_DEPTH {
        return;
    }
    for entry in entries {
        if let Some(id) = entry.as_reference() {
            out.insert(id);
        }
        // A nested array groups the entries under the label before it, and may itself be an
        // indirect object, so it is resolved before being descended into.
        if let Some(nested) = document.resolve(entry).as_array() {
            order_references(document, nested, depth.saturating_add(1), out);
        }
    }
}

/// ISO 19005-2 section 6.9, ISO 19005-4 section 6.10.
///
/// The groups a file has are the ones `/OCProperties /OCGs` lists: the base standard requires that
/// array to name every one, and this crate has no better census of them than the document's own.
fn order_lists_every_group(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let properties = document.get_key(&catalog, "OCProperties");
    let Some(properties) = properties.as_dict() else {
        return;
    };
    let groups: BTreeSet<ObjectId> = document
        .get_key(properties, "OCGs")
        .as_array()
        .map(|array| array.iter().filter_map(Object::as_reference).collect())
        .unwrap_or_default();
    if groups.is_empty() {
        return;
    }

    for_each_configuration(document, |place, configuration| {
        let order = document.get_key(configuration, "Order");
        let Some(order) = order.as_array() else {
            return;
        };
        let mut referenced = BTreeSet::new();
        order_references(document, order, 0, &mut referenced);
        for missing in groups.difference(&referenced) {
            findings.record(
                place.clone().named(format!("{missing:?}")),
                "an Order array does not reference every optional content group in the file",
            );
        }
    });
}

/// ISO 19005-2 section 6.9, and part 2's alone: ISO 19005-4 section 6.10 permits the entry and has
/// a processor ignore it instead.
fn no_automatic_states(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_configuration(document, |place, configuration| {
        if configuration.get("AS").is_some() {
            findings.record(
                place.named("AS"),
                "an optional content configuration states an AS key",
            );
        }
    });
}

/// ISO 19005-2 section 6.10, ISO 19005-4 section 6.11.
fn no_alternate_presentations(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let names = document.get_key(&catalog, "Names");
    if let Some(names) = names.as_dict()
        && names.get("AlternatePresentations").is_some()
    {
        findings.record(
            Where::file().named("AlternatePresentations"),
            "the name dictionary states an AlternatePresentations key",
        );
    }
}

/// ISO 19005-2 section 6.10, ISO 19005-4 section 6.11.
fn no_presentation_steps(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let pages = Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        if page.dict.get("PresSteps").is_some() {
            findings.record(
                Where::page(index).named("PresSteps"),
                "a page dictionary states a PresSteps key",
            );
        }
    }
}

/// ISO 19005-2 section 6.11, ISO 19005-4 section 6.12.
fn no_requirements_dictionary(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if let Ok(catalog) = document.catalog()
        && catalog.get("Requirements").is_some()
    {
        findings.record(
            Where::file().named("Requirements"),
            "the document catalog states a Requirements key",
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::Examination;
    use std::fmt::Write as _;

    use pdf_syntax::Document;

    use super::{
        Findings, REQUIREMENTS, configuration_names, file_and_unicode_names,
        no_alternate_presentations, no_automatic_states, no_presentation_steps,
        no_requirements_dictionary, order_lists_every_group, pdfa_4f_carries_embedded_files,
        relationship_stated, role_map_terminates_at_a_standard_type, structure_tree_root,
    };
    use super::{associated_file_media_type, catalog_language_identifier};
    use super::{embedded_file_is_itself_pdfa, is_media_type};
    use crate::target::{Flavour, Level, Target};

    /// A one-page document with whatever the fixture adds to its catalog and to its object body.
    fn document(catalog: &str, extra: &str) -> Document {
        page_document(catalog, "", extra)
    }

    /// The same, with entries added to the page dictionary as well.
    fn page_document(catalog: &str, page: &str, extra: &str) -> Document {
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R{catalog} >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200]{page} >>\nendobj\n\
             {extra}"
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

    /// How many places a predicate found.
    fn found(predicate: fn(&Examination<'_>, &mut Findings), document: &Document) -> usize {
        let mut findings = Findings::default();
        let exam = Examination::new(document, Target::Four(Flavour::Plain));
        predicate(&exam, &mut findings);
        findings.seen()
    }

    /// The applicability columns this tranche turns on, asserted rather than described.
    ///
    /// ISO 19005-4 section 6.8 states no requirement, so every logical-structure row is part 2's
    /// and binds Level A alone; ISO 19005-4 Annex A §A.2 is PDF/A-4f's alone; and the row that asks
    /// an embedded file to be a PDF/A file is lifted by both annexes, so under part 4 it binds the
    /// plain profile and nothing else.
    #[test]
    fn the_rows_bind_the_levels_and_flavours_their_clauses_reach() {
        let row = |id: &str| {
            REQUIREMENTS
                .iter()
                .find(|requirement| requirement.id == id)
                .expect("the row is in this module")
        };

        for id in [
            "logical-structure/tagged-pdf",
            "logical-structure/mark-info-marked",
            "logical-structure/word-boundaries",
            "logical-structure/structure-tree-root",
            "logical-structure/role-map-terminates-at-a-standard-type",
            "logical-structure/catalog-language-identifier",
            "logical-structure/element-and-property-list-language-identifiers",
        ] {
            for target in Target::ALL {
                assert_eq!(
                    row(id).binds(target),
                    target == Target::Two(Level::A),
                    "{id} against {target}"
                );
            }
        }

        for target in Target::ALL {
            assert_eq!(
                row("embedded-files/pdfa-4f-carries-embedded-files").binds(target),
                target == Target::Four(Flavour::F),
                "{target}"
            );
            assert_eq!(
                row("embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile")
                    .binds(target),
                target == Target::Four(Flavour::Plain),
                "{target}"
            );
            assert_eq!(
                row("optional-content/no-automatic-states").binds(target),
                target.flavour().is_none(),
                "{target}: part 4 permits the AS key that part 2 forbids"
            );
        }
    }

    /// The three outright prohibitions, each on a fixture that states exactly the forbidden key.
    #[test]
    fn the_three_forbidden_keys_are_each_found_where_they_are_stated() {
        let clean = document("", "");
        assert_eq!(found(no_requirements_dictionary, &clean), 0);
        assert_eq!(found(no_alternate_presentations, &clean), 0);
        assert_eq!(found(no_presentation_steps, &clean), 0);

        assert_eq!(
            found(
                no_requirements_dictionary,
                &document(" /Requirements []", "")
            ),
            1
        );
        assert_eq!(
            found(
                no_alternate_presentations,
                &document(" /Names << /AlternatePresentations 4 0 R >>", "")
            ),
            1
        );

        let with_steps = page_document("", " /PresSteps << /Type /NavNode >>", "");
        assert_eq!(found(no_presentation_steps, &with_steps), 1);
    }

    /// ISO 19005-2 section 6.9: a configuration without a name, and two configurations sharing one.
    #[test]
    fn a_configuration_states_a_name_and_no_two_state_the_same_one() {
        let named = document(
            " /OCProperties << /OCGs [] /D << /Name (Default) >> \
              /Configs [<< /Name (German) >>] >>",
            "",
        );
        assert_eq!(found(configuration_names, &named), 0);

        let nameless = document(" /OCProperties << /OCGs [] /D << >> >>", "");
        assert_eq!(found(configuration_names, &nameless), 1);

        let duplicated = document(
            " /OCProperties << /OCGs [] /D << /Name (Same) >> /Configs [<< /Name (Same) >>] >>",
            "",
        );
        assert_eq!(found(configuration_names, &duplicated), 1);
    }

    /// ISO 19005-2 section 6.9: an `/Order` that names one group of two, including through nesting.
    #[test]
    fn an_order_array_has_to_reach_every_group_the_file_declares() {
        let complete = document(
            " /OCProperties << /OCGs [4 0 R 5 0 R] \
              /D << /Name (D) /Order [4 0 R [(Label) 5 0 R]] >> >>",
            "4 0 obj\n<< /Type /OCG /Name (One) >>\nendobj\n\
             5 0 obj\n<< /Type /OCG /Name (Two) >>\nendobj\n",
        );
        assert_eq!(
            found(order_lists_every_group, &complete),
            0,
            "a group named inside a nested array is still named"
        );

        let partial = document(
            " /OCProperties << /OCGs [4 0 R 5 0 R] /D << /Name (D) /Order [4 0 R] >> >>",
            "4 0 obj\n<< /Type /OCG /Name (One) >>\nendobj\n\
             5 0 obj\n<< /Type /OCG /Name (Two) >>\nendobj\n",
        );
        assert_eq!(found(order_lists_every_group, &partial), 1);
    }

    /// `/AS` is part 2's prohibition alone; the row is `Clauses::only_two`, and this is its body.
    #[test]
    fn an_automatic_state_entry_is_found_in_any_configuration() {
        let file = document(
            " /OCProperties << /OCGs [] /D << /Name (D) /AS [] >> >>",
            "",
        );
        assert_eq!(found(no_automatic_states, &file), 1);
    }

    /// ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9, over a specification the name tree
    /// files indirectly.
    #[test]
    fn an_embedded_file_states_its_two_names_and_its_relationship() {
        let bare = document(
            " /Names << /EmbeddedFiles << /Names [(data) 4 0 R] >> >>",
            "4 0 obj\n<< /Type /Filespec /EF << /F 5 0 R >> >>\nendobj\n\
             5 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
        );
        assert_eq!(
            found(file_and_unicode_names, &bare),
            2,
            "neither F nor UF is stated, and each is its own place"
        );
        assert_eq!(found(relationship_stated, &bare), 1);

        let complete = document(
            " /Names << /EmbeddedFiles << /Names [(data) 4 0 R] >> >>",
            "4 0 obj\n<< /Type /Filespec /F (data.pdf) /UF (data.pdf) \
             /AFRelationship /Source /EF << /F 5 0 R >> >>\nendobj\n\
             5 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
        );
        assert_eq!(found(file_and_unicode_names, &complete), 0);
        assert_eq!(found(relationship_stated, &complete), 0);
        assert_eq!(found(pdfa_4f_carries_embedded_files, &complete), 0);
        assert_eq!(
            found(pdfa_4f_carries_embedded_files, &document("", "")),
            1,
            "ISO 19005-4 section A.2 makes the tree required of a PDF/A-4f file"
        );
    }

    /// ISO 19005-2 section 6.7.3.3 and section 6.7.3.4: a tree that is there, and a type that maps
    /// nowhere.
    #[test]
    fn a_structure_type_that_maps_to_no_standard_type_is_named() {
        let untagged = document("", "");
        assert_eq!(found(structure_tree_root, &untagged), 1);
        assert_eq!(
            found(role_map_terminates_at_a_standard_type, &untagged),
            0,
            "a file with no tree states no type that could fail to map"
        );

        let mapped = document(
            " /StructTreeRoot 4 0 R",
            "4 0 obj\n<< /Type /StructTreeRoot /K [5 0 R] /RoleMap << /Chapter /Sect >> >>\nendobj\n\
             5 0 obj\n<< /Type /StructElem /S /Chapter /P 4 0 R >>\nendobj\n",
        );
        assert_eq!(found(structure_tree_root, &mapped), 0);
        assert_eq!(found(role_map_terminates_at_a_standard_type, &mapped), 0);

        let unmapped = document(
            " /StructTreeRoot 4 0 R",
            "4 0 obj\n<< /Type /StructTreeRoot /K [5 0 R] >>\nendobj\n\
             5 0 obj\n<< /Type /StructElem /S /Chapter /P 4 0 R >>\nendobj\n",
        );
        assert_eq!(found(role_map_terminates_at_a_standard_type, &unmapped), 1);
    }

    /// ISO 19005-2 section 6.7.4, over the four shapes §14.9.2.2's sentence distinguishes.
    ///
    /// The tags are the corpus's own witnesses for the clause, which is why they are these rather
    /// than invented ones: `zh-Hant-HK` and `ru-petr1708` exercise the script, region and variant
    /// subtags, and each refusal names a different way of not being a `Language-Tag` — a digit
    /// where the primary subtag goes, a separator that is not a hyphen, and an empty subtag.
    #[test]
    fn a_catalog_language_is_a_bcp_47_tag_or_the_empty_string() {
        for accepted in ["zh-Hant-HK", "lv", "ru-petr1708", "hr-ba", ""] {
            let file = document(&format!(" /Lang ({accepted})"), "");
            assert_eq!(
                found(catalog_language_identifier, &file),
                0,
                "{accepted:?} is a language identifier"
            );
        }
        assert_eq!(
            found(catalog_language_identifier, &document("", "")),
            0,
            "the subclause only recommends that a file state a default language"
        );
        for refused in ["12-BE", "de/AT", "-BG"] {
            let file = document(&format!(" /Lang ({refused})"), "");
            assert_eq!(
                found(catalog_language_identifier, &file),
                1,
                "{refused:?} is not"
            );
        }
        assert_eq!(
            found(catalog_language_identifier, &document(" /Lang /en", "")),
            1,
            "a language identifier is a text string, so a name is not one"
        );
    }

    /// ISO 19005-2 section 6.8, ISO 19005-4 section 6.9: the half of the rule that needs no
    /// recursion.
    ///
    /// A stream that is not a PDF at all conforms to no part of ISO 19005, and a stream that
    /// declares no part is passed over — it may be the part 1 or part 3 file this crate cannot
    /// judge, and refusing it would be refusing a document over a text the project does not hold.
    #[test]
    fn an_embedded_file_that_is_not_a_pdf_conforms_to_no_part_of_iso_19005() {
        let embedding = |data: &str| {
            let length = data.len();
            document(
                " /Names << /EmbeddedFiles << /Names [(data) 4 0 R] >> >>",
                &format!(
                    "4 0 obj\n<< /Type /Filespec /F (data) /UF (data) /AFRelationship /Source \
                     /EF << /F 5 0 R >> >>\nendobj\n\
                     5 0 obj\n<< /Length {length} >>\nstream\n{data}\nendstream\nendobj\n"
                ),
            )
        };
        assert_eq!(
            found(embedded_file_is_itself_pdfa, &embedding("Test text")),
            1,
            "nine bytes of text are not a PDF document"
        );
        assert_eq!(
            found(embedded_file_is_itself_pdfa, &document("", "")),
            0,
            "a document that embeds nothing has nothing to be judged"
        );
    }

    /// Internet RFC 2046's composition, and nothing beyond it.
    ///
    /// The two `model/` types are ISO 32000-2 §13.6.3's own, and they are why the seven top-level
    /// types the RFC lists are not required here.
    #[test]
    fn a_media_type_is_a_type_and_a_subtype_around_a_solidus() {
        for good in [
            &b"application/pdf"[..],
            b"application/octet-stream",
            b"model/u3d",
            b"text/csv",
        ] {
            assert!(is_media_type(good), "{}", String::from_utf8_lossy(good));
        }
        for bad in [
            &b"application"[..],
            b"/pdf",
            b"application/",
            b"application/pdf; charset=utf-8",
            b"",
        ] {
            assert!(!is_media_type(bad), "{}", String::from_utf8_lossy(bad));
        }
    }

    /// The rule reaches an embedded file the document associates and no other.
    ///
    /// §14.13.1 makes an `AF` array what connects a specification to an object, so the same
    /// malformed `Subtype` is a failure in the first document and nothing in the second.
    #[test]
    fn only_an_associated_files_media_type_is_judged() {
        let specification = "4 0 obj\n<< /Type /Filespec /F (data.pdf) /UF (data.pdf) \
                             /AFRelationship /Data /EF << /F 5 0 R >> >>\nendobj\n\
                             5 0 obj\n<< /Type /EmbeddedFile /Subtype /application /Length 0 >>\n\
                             stream\n\nendstream\nendobj\n";
        let associated = document(" /AF [4 0 R]", specification);
        assert_eq!(found(associated_file_media_type, &associated), 1);

        let unassociated = document("", specification);
        assert_eq!(found(associated_file_media_type, &unassociated), 0);

        let proper = "4 0 obj\n<< /Type /Filespec /F (data.pdf) /UF (data.pdf) \
                      /AFRelationship /Data /EF << /F 5 0 R >> >>\nendobj\n\
                      5 0 obj\n<< /Type /EmbeddedFile /Subtype /application#2Fpdf /Length 0 >>\n\
                      stream\n\nendstream\nendobj\n";
        assert_eq!(
            found(
                associated_file_media_type,
                &document(" /AF [4 0 R]", proper)
            ),
            0,
            "a name writing the solidus as #2F is the media type it decodes to"
        );
    }
}
