//! Deciding whether a document conforms to a stated part and level of ISO 19005 (PDF/A).
//!
//! # What this crate is
//!
//! A **reader**, over the parsers, colour engine, font loader and structure tree this project
//! already has. It adds no reader of its own, and that is the design rather than an economy: a
//! PDF/A requirement is a question asked of `pdf-syntax` and `pdf-model`, and one that needed a
//! new reader would be a gap in *those* crates rather than a feature here.
//!
//! Its product is a [`Report`] — a verdict, and beside it the list of requirements the verdict
//! is a verdict *over*. `doc/rfc/0006` argues that this half of PDF/A is the larger part of the
//! value and nearly all of the reusable work; `doc/pdf-a-conversion-limits.md` is what a
//! converter built on top of it would have to tell a user.
//!
//! # Provenance, and one rule about quotation
//!
//! Every requirement cites its clause in ISO 19005-2:2011 or ISO 19005-4:2020, read first-hand
//! from the copies `doc/questions/A16` records the owner buying. **Those copies are licensed to
//! a single reader, so nothing in this crate quotes them**: a requirement's `asks` field is
//! this crate's own sentence and the clause number is how a reader reaches the standard's.
//! ISO 32000-2 is quoted normally where a rule turns on its words, because `doc/md/` carries it
//! and `tools/conformance` can check the quotation.
//!
//! Parts 1 and 3 are not targets. `CLAUDE.md` principle 5 does not permit implementing a
//! requirement from somebody else's reading of a text this project does not have, and
//! `doc/questions/A17` settles those two: part 1 never, part 3 not bought.
//!
//! # The rule that shapes the output
//!
//! A requirement this crate does not check is **named in the verdict**, with its reason. See
//! [`report`] for why that is the crate's one unbreakable rule.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod clarification;
pub mod coverage;
pub mod editions;
pub mod errata;
pub mod examination;
pub mod finding;
pub mod iso_8601;
pub mod report;
pub mod requirement;
pub mod survey;
pub mod table;
pub mod target;

use pdf_syntax::Document;

pub use crate::clarification::Clarification;
pub use crate::coverage::{
    Binding, Carried, Ground, Reading, Sentence, Subclause, frontier, readings, subclauses,
};
pub use crate::editions::{Earlier, SHIFTS, Shift, shift_of};
pub use crate::errata::Erratum;
pub use crate::examination::Examination;
pub use crate::finding::{Finding, Findings, Where};
pub use crate::report::{Judgement, Outcome, Report, Verdict};
pub use crate::requirement::{Applies, Check, Clauses, Requirement};
pub use crate::table::interaction::{MissingAppearance, annotations_without_an_appearance};
pub use crate::table::metadata::{MisusedProperty, dates_stated, properties_outside_their_schema};
pub use crate::target::{Flavour, Level, Part, Target};

/// Holds one document to one target, and reports what it found.
///
/// Every requirement in [`table::REQUIREMENTS`] that binds the target is judged, in table
/// order, and the ones that do not bind it are absent rather than passed — an inapplicable
/// requirement is not a met one, and a report that conflated them would overstate what it had
/// established.
#[must_use]
pub fn check(document: &Document, target: Target) -> Report {
    // One examination for the whole report, so that the content walk thirteen requirements need
    // happens once rather than thirteen times. `examination` says what that was worth.
    let examination = Examination::new(document, target);
    let judgements = table::binding(target)
        .map(|requirement| judge(&examination, requirement))
        .collect();
    Report { target, judgements }
}

/// One requirement, applied.
fn judge(examination: &Examination<'_>, requirement: &Requirement) -> Judgement {
    let target = examination.target;
    let outcome = match requirement.check {
        Check::Unchecked(why) => Outcome::Unchecked(why),
        Check::Processor(why) => Outcome::Processor(why),
        Check::OutsideValidation(why) => Outcome::OutsideValidation(why),
        Check::Implemented(predicate) => {
            let mut findings = Findings::default();
            predicate(examination, &mut findings);
            if findings.met() {
                Outcome::Met
            } else {
                Outcome::Failed {
                    places: findings.kept().to_vec(),
                    total: findings.seen(),
                }
            }
        }
    };
    Judgement {
        id: requirement.id,
        amended_by: errata::amending(requirement.id),
        clarified_by: clarification::clarifying(requirement.id, target.part()),
        // A requirement only reaches here if `binds` said the target's part states it, so the
        // citation is always present; the fallback names the bug rather than panicking.
        citation: requirement
            .clauses
            .citation(target)
            .unwrap_or_else(|| "no clause for this part".to_owned()),
        asks: requirement.asks,
        outcome,
    }
}
