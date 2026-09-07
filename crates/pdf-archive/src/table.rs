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

use crate::requirement::Requirement;

mod document_level;
mod file_structure;
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
