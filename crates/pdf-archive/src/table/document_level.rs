//! Everything after metadata: logical structure, embedded files, optional content, alternate
//! presentations, and the document requirements dictionary.
//!
//! ISO 19005-2 §6.7 to §6.11 and ISO 19005-4 §6.8 to §6.15 — five subclauses in one part and
//! eight in the other, and the mismatch is the interesting thing about this tranche.
//!
//! # Where the two parts part company
//!
//! - **Logical structure is part 2's alone.** §6.7 is a full subclause of eight parts and applies
//!   only at Level A, which is why every row in that area carries `Applies::FromLevel(Level::A)`.
//!   ISO 19005-4 §6.8 replaces the whole of it with two sentences of encouragement and **states no
//!   requirement at all** — not one `shall` — so those rows are `Clauses::only_two`. Part 2's own
//!   subclause is mostly recommendation too: of everything §6.7 says, six sentences are `shall`
//!   and every one of the things a reader would call accessibility — alternate descriptions,
//!   replacement text, expansions, the default language — is `should`. Six sentences, six rows.
//! - **Part 4 requires more of an embedded file than part 2 does**, adding `/AFRelationship` to
//!   part 2's `/F` and `/UF`, and then Annexes A and B take the type restriction back off again
//!   for PDF/A-4f and PDF/A-4e. So the row that asks an embedded file to be a PDF/A file binds
//!   the plain profile alone.
//! - **`/AS` in an optional content configuration is forbidden by part 2 and merely ignored by
//!   part 4**, which is the one rule in this tranche a file can fail under one part and pass under
//!   the other with no other change.
//!
//! # What has no row here, and why that is not an omission
//!
//! **A clause whose only requirement is addressed to a processor gets no row**, because this crate
//! judges a file and no file can fail such a rule. Naming them is what keeps the absence visible:
//!
//! - the rendering of optional content in the default configuration, the display of the `/Order`
//!   array and of the list of configurations, and the instruction not to use `/Intent` — ISO
//!   19005-2 §6.9 and ISO 19005-4 §6.10, with part 4's instruction to ignore `/AS` beside them;
//! - the display of the names of embedded files — ISO 19005-2 §6.8 and ISO 19005-4 §6.9;
//! - the instruction to ignore `/Trans` and `/Dur` — ISO 19005-2 §6.10 and ISO 19005-4 §6.11;
//! - **the whole of ISO 19005-4 §6.13**, whose subject is what a processor does about
//!   `/PrintScaling` and `/Enforce` when printing. It places nothing on a file.
//!
//! **ISO 19005-4 §6.14 and §6.15 place nothing on anybody.** Each is a single sentence permitting
//! a conforming file to carry the base standard's geospatial information (§6.14) or measurement
//! properties (§6.15) by any of the mechanisms ISO 32000-2 describes. A permission is not a
//! requirement, and inventing a row for one would put a rule in this table that the standard does
//! not state.

use std::collections::BTreeSet;

use pdf_model::Pages;
use pdf_model::structure::{Child, MarkInfo, Tree};
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
        id: "logical-structure/language-identifiers",
        asks: "Where a Lang entry is stated — in the catalog, in a structure element or in a \
               property list — its value shall be a language identifier the base standard \
               defines.",
        clauses: Clauses::only_two("6.7.4"),
        applies: Applies::FromLevel(Level::A),
        check: Check::Unchecked(
            "two halves are missing: a property list's Lang lives in a content stream this crate \
             does not walk, and deciding whether a string is a language identifier is IETF BCP \
             47's grammar, which no reader in this tree implements",
        ),
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
        check: Check::Unchecked(EMBEDDED_FILE_IS_ITSELF_PDFA),
    },
    Requirement {
        id: "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
        asks: "Every embedded file shall itself conform to a part of ISO 19005 — which Annex A \
               lifts for PDF/A-4f and Annex B for PDF/A-4e, both letting an embedded file be of \
               any type.",
        clauses: Clauses::only_four("6.9"),
        applies: Applies::Flavours(&[Flavour::Plain]),
        check: Check::Unchecked(EMBEDDED_FILE_IS_ITSELF_PDFA),
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
        id: "document-requirements/no-requirements-dictionary",
        asks: "The document catalog shall state no Requirements key.",
        clauses: Clauses::both("6.11", "6.12"),
        applies: Applies::Always,
        check: Check::Implemented(no_requirements_dictionary),
    },
];

/// Why neither part's embedded-file rule is checked, shared by the two rows that state it.
///
/// One string rather than two, because it is one reason: the parts differ in which flavours the
/// rule still binds, not in what asking it would take.
const EMBEDDED_FILE_IS_ITSELF_PDFA: &str = "answering it means holding the embedded bytes to ISO 19005 as a document in their own right, \
     and nothing here opens an embedded stream as a `pdf_syntax::Document` to run this crate over \
     it again";

/// ISO 19005-2 §6.7.2.2.
fn mark_info_marked(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if !MarkInfo::read(document).marked {
        findings.record(
            Where::file().named("MarkInfo"),
            "the catalog does not state a MarkInfo dictionary with Marked true",
        );
    }
}

/// ISO 19005-2 §6.7.3.3.
fn structure_tree_root(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if Tree::of(document).is_none() {
        findings.record(
            Where::file().named("StructTreeRoot"),
            "the catalog states no structure tree root, so the file describes no logical structure",
        );
    }
}

/// ISO 19005-2 §6.7.3.4.
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

/// ISO 19005-2 §6.8, ISO 19005-4 §6.9.
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

/// ISO 19005-4 §6.9.
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

/// ISO 19005-4 §A.2, which is the one requirement Annex A adds rather than relaxes.
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

/// ISO 19005-2 §6.9, ISO 19005-4 §6.10.
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

/// ISO 19005-2 §6.9, ISO 19005-4 §6.10.
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

/// ISO 19005-2 §6.9, and part 2's alone: ISO 19005-4 §6.10 permits the entry and has a processor
/// ignore it instead.
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

/// ISO 19005-2 §6.10, ISO 19005-4 §6.11.
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

/// ISO 19005-2 §6.10, ISO 19005-4 §6.11.
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

/// ISO 19005-2 §6.11, ISO 19005-4 §6.12.
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
    /// ISO 19005-4 §6.8 states no requirement, so every logical-structure row is part 2's and
    /// binds Level A alone; ISO 19005-4 Annex A §A.2 is PDF/A-4f's alone; and the row that asks an
    /// embedded file to be a PDF/A file is lifted by both annexes, so under part 4 it binds the
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
            "logical-structure/language-identifiers",
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

    /// ISO 19005-2 §6.9: a configuration without a name, and two configurations sharing one.
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

    /// ISO 19005-2 §6.9: an `/Order` that names one group of two, including through nesting.
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

    /// ISO 19005-2 §6.8 and ISO 19005-4 §6.9, over a specification the name tree files indirectly.
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
            "ISO 19005-4 §A.2 makes the tree required of a PDF/A-4f file"
        );
    }

    /// ISO 19005-2 §6.7.3.3 and §6.7.3.4: a tree that is there, and a type that maps nowhere.
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
}
