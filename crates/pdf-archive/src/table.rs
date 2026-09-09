//! The requirement table: every rule this crate knows, in clause order.
//!
//! One table for both owned parts, because the measurement says that is the cheaper shape —
//! `python3 tools/pdfa-text.py --overlap` counts how many of ISO 19005-2's and ISO 19005-4's
//! clause 6 requirements are the same rule, and most of them are. A shared rule is written once
//! as a predicate and cited twice through [`crate::Clauses`]; a level is a column
//! ([`crate::Applies`]) rather than an implementation. `doc/questions/Q46` has the argument.
//!
//! # Order, and what it is for
//!
//! Clause order, part 4's where the two differ, because a person reviewing this table against
//! the standard reads it with the standard open. The identifiers are not clause numbers for the
//! reason [`crate::Requirement::id`] gives: the two parts number the same rule differently.

use pdf_syntax::{Dictionary, Document, Object};

use crate::requirement::Requirement;

/// Whether a dictionary states an entry at all, in ISO 32000-2 §7.3.7's sense.
///
/// Not `Dictionary::get`, because a key written with nothing behind it is not a key:
///
/// > A dictionary entry whose value is null (see 7.3.9, "Null object") shall be treated the same
/// > as if the entry does not exist.
///
/// and §7.3.10 puts a reference to an object that is not there in the same place:
///
/// > An indirect reference to an undefined object shall not be considered an error by a PDF
/// > processor; it shall be treated as a reference to the null object.
///
/// Every rule that forbids a key, and every guard that skips a dictionary for not stating one,
/// asks through this — otherwise `/OPI null` would be reported as an `/OPI`, which is this crate
/// failing a document the base standard says states nothing there. It lives here rather than in
/// a tranche because three tranches ask it and the sentences behind it are the same in each; two
/// private copies with two different doc comments is how one of them drifts.
///
/// **Measured, because sixteen rules in the graphics tranche alone ask it while walking every
/// object.** `examples/cost.rs` over ISO 32000-1's own 127 000 objects: 1.923 s and 1.919 s of
/// predicates with the key read the cheap way, 1.927 s and 1.949 s with it resolved; over
/// ISO 32000-2's 101 000, 4.013 s against 4.075 s. Under two per cent, and inside the spread of
/// two runs — because `Document::get_key` fetches an object only where the entry *is* a
/// reference, and a forbidden key is absent from almost every dictionary this walks.
pub(crate) fn states(document: &Document, dict: &Dictionary, key: &str) -> bool {
    !matches!(document.get_key(dict, key), Object::Null)
}

/// The name a dictionary states for `key`, or `None` where it states something else.
pub(crate) fn name_of(document: &Document, dict: &Dictionary, key: &str) -> Option<Vec<u8>> {
    document
        .get_key(dict, key)
        .as_name()
        .map(|name| name.as_bytes().to_vec())
}

/// Whether a dictionary states `key` as exactly this name.
pub(crate) fn states_name(document: &Document, dict: &Dictionary, key: &str, name: &[u8]) -> bool {
    name_of(document, dict, key).is_some_and(|value| value == name)
}

mod document_level;
pub(crate) mod file_structure;
mod fonts;
mod graphics;
mod interaction;
mod metadata;

/// The tranches, in clause order, each owning one area of clause 6.
///
/// A slice of slices rather than one array, because the areas are written and reviewed
/// separately: a person checking the colour rules against the standard opens one file, and two
/// people extending different areas never edit the same line.
static TRANCHES: &[&[Requirement]] = &[
    file_structure::REQUIREMENTS,
    graphics::REQUIREMENTS,
    fonts::REQUIREMENTS,
    interaction::REQUIREMENTS,
    metadata::REQUIREMENTS,
    document_level::REQUIREMENTS,
];

/// Every requirement this crate knows, in clause order.
///
/// **Growing this table is the work.** A row added to a tranche is checked by every target it
/// binds with no further wiring, and a row whose [`crate::Check`] is `Unchecked` is a promise
/// kept rather than a gap hidden: it appears in every report that binds it, by name, with its
/// reason.
pub fn requirements() -> impl Iterator<Item = &'static Requirement> {
    TRANCHES.iter().copied().flatten()
}

/// The requirements one target is actually held to.
///
/// **The single place that decides**, because two answers to "does this bind" is one more than a
/// report can survive: an example that counted a rule the checker skipped would print a coverage
/// figure no verdict matches. Two filters, and the second is the reason this function exists —
/// [`Requirement::binds`] applies the standard as published, and `crate::errata` removes what
/// the committee has since withdrawn from a part.
pub fn binding(target: crate::Target) -> impl Iterator<Item = &'static Requirement> {
    requirements().filter(move |requirement| {
        requirement.binds(target)
            && (target.part() != crate::Part::Four
                || !crate::errata::withdrawn_from_part_four(requirement))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::requirements;
    use crate::target::Target;

    #[test]
    fn every_identifier_is_unique() {
        let mut seen = BTreeSet::new();
        for requirement in requirements() {
            assert!(
                seen.insert(requirement.id),
                "two requirements share the identifier {}",
                requirement.id
            );
        }
    }

    #[test]
    fn every_requirement_binds_at_least_one_target() {
        for requirement in requirements() {
            assert!(
                Target::ALL.iter().any(|target| requirement.binds(*target)),
                "{} binds no target, so nothing would ever check it",
                requirement.id
            );
        }
    }

    /// A row that states a clause for a part it never binds is a table error, not a subtlety.
    #[test]
    fn a_clause_is_stated_for_every_part_the_requirement_binds() {
        for requirement in requirements() {
            for target in Target::ALL {
                if requirement.binds(target) {
                    assert!(
                        requirement.clauses.citation(target).is_some(),
                        "{} binds {target} and cites no clause for it",
                        requirement.id
                    );
                }
            }
        }
    }
}
