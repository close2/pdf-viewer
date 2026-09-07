//! The approved corrections to ISO 19005, as an input to the table rather than a footnote.
//!
//! # Why a validator must read these
//!
//! An erratum is not commentary on the standard; it is a correction *to* it, approved by the
//! committee that wrote it. So the corrected text is what a requirement means, and a validator
//! implementing the published sentence where a correction exists is implementing a sentence its
//! own authors have withdrawn. `CLAUDE.md` principle 5 makes the specification the only source
//! of truth, and an approved erratum is part of that specification.
//!
//! This is also why the errata are read *before* another implementation is consulted: several of
//! the disagreements a corpus produces are not readings at all, but one side having the
//! correction and the other not.
//!
//! # Where they come from, and what is not here
//!
//! The PDF Association records corrections at <https://pdf-issues.pdfa.org/> and tracks them as
//! issues in `pdf-association/pdf-issues`. **Only the entries labelled `ISO approved` are
//! errata**; the rest are questions, some of them open, and this table carries none of those —
//! an open question is not a correction, and treating it as one would be adopting a proposal.
//!
//! **Errata are published for ISO 19005-4:2020 alone.** The site states no errata pages for
//! parts 1, 2 and 3 and directs a reader to the Technical Notes instead, so a part 2 row is
//! amended by nothing here, and PDF Association `TechNote 0010` — which clarifies parts 1 to 3 —
//! is the document that would change that. It has not been read: the copy at `pdfa.org` returned
//! HTTP 403 when this table was written, and a clarification nobody here has read may not be
//! implemented (principle 5 again).
//!
//! # What an erratum does to a row
//!
//! Two things, and the second is why this is a table rather than a comment. It **changes what a
//! row means**, which the row's own predicate has to reflect; and it **is cited in the report**,
//! so that a person checking a verdict against their own copy of the standard is told why the
//! two differ. A verdict that silently applied a correction the reader cannot see would be as
//! opaque as one that ignored it.

use crate::requirement::Requirement;

/// One approved correction, as this crate cites it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Erratum {
    /// The issue number in `pdf-association/pdf-issues`, which is how it is looked up.
    pub issue: u32,
    /// Which standard it corrects.
    pub standard: &'static str,
    /// What the correction does, in one sentence of this crate's own words.
    pub change: &'static str,
}

/// Every approved erratum that bears on a row of the table, by requirement identifier.
///
/// Reviewable against the tracker in one reading, which is the point of keeping it in one place
/// rather than scattering the notes through six tranche files.
static ERRATA: &[(&str, Erratum)] = &[
    (
        "graphics/halftone-transfer-function-only-where-required",
        Erratum {
            issue: 314,
            standard: "ISO 19005-4:2020",
            change: "the provision is removed from PDF/A-4 altogether — the PDF/A and PDF/X \
                     working groups agreed to drop it from part 4 and leave PDF/X-6 as it was — \
                     so no PDF/A-4 target is held to it",
        },
    ),
    (
        "metadata/identification-conformance-level",
        Erratum {
            issue: 123,
            standard: "ISO 19005-4:2020",
            change: "Table 2 states the conformance property with a `pdfa` prefix where the \
                     subclause requires `pdfaid`, and states the namespace with an `https` \
                     scheme where it should be `http`; both are errors the working group agreed \
                     require fixing, with a note that an implementation may accept `https` as \
                     well",
        },
    ),
    (
        "metadata/identification-revision-year",
        Erratum {
            issue: 253,
            standard: "ISO 19005-4:2020",
            change: "the requirement is reworded to make the value the four-digit publication \
                     year of that specific revision of the document, and the working group \
                     reconfirmed that the property is required and is 2020 for the current one",
        },
    ),
];

/// The approved correction bearing on one requirement, if there is one.
#[must_use]
pub fn amending(id: &str) -> Option<Erratum> {
    ERRATA
        .iter()
        .find(|(row, _)| *row == id)
        .map(|(_, erratum)| *erratum)
}

/// Every erratum in the table, for a caller that wants to list them.
pub fn all() -> impl Iterator<Item = (&'static str, Erratum)> {
    ERRATA.iter().copied()
}

/// Whether an erratum removes this requirement from a part altogether.
///
/// The one shape that cannot be left to a predicate: a rule the committee has *withdrawn* from a
/// part must not bind that part's targets at all, because a predicate that ran and passed would
/// still be counted among the requirements checked — overstating what the verdict covers.
#[must_use]
pub fn withdrawn_from_part_four(requirement: &Requirement) -> bool {
    amending(requirement.id).is_some_and(|erratum| erratum.issue == 314)
}

#[cfg(test)]
mod tests {
    use super::{ERRATA, all, amending};
    use crate::table;

    /// An erratum naming a row that does not exist is a table that has drifted.
    #[test]
    fn every_erratum_names_a_requirement_that_exists() {
        for (id, erratum) in all() {
            assert!(
                table::requirements().any(|requirement| requirement.id == id),
                "erratum #{} names {id}, which is not a requirement",
                erratum.issue
            );
        }
    }

    #[test]
    fn an_erratum_is_found_by_the_identifier_it_names() {
        for (id, erratum) in ERRATA {
            assert_eq!(amending(id).map(|found| found.issue), Some(erratum.issue));
        }
        assert_eq!(amending("nothing/at/all"), None);
    }
}
