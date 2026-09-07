//! The verdict, and the discipline that makes it worth having.
//!
//! ISO 19005-2 §6.6.4 and ISO 19005-4 §6.7.3 both end their identification subclause with the
//! same warning: the `pdfaid` properties do not by themselves determine conformance, and the
//! actual determination is made against clause 5. So a file *claiming* to be PDF/A-4 and a
//! file *being* PDF/A-4 are different facts, and this report answers the second.
//!
//! # A pass is relative to a stated list
//!
//! `doc/questions/Q20` and `CLAUDE.md` principle 5 give this crate its one unbreakable rule:
//! **a requirement that was not checked is named in the verdict.** [`Verdict::Conforms`]
//! therefore does not mean "conforms"; it means "no requirement this crate checks was failed,
//! and here are the ones it did not check". A validator that omitted the second half would be
//! indistinguishable from one that had checked everything, which is the failure the rule exists
//! to prevent.

use core::fmt::Write as _;

use crate::finding::Finding;
use crate::target::Target;

/// What became of one requirement when the document was held to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Checked, and the document met it.
    Met,
    /// Checked, and the document did not — with the places, and whether the list is a prefix.
    Failed {
        /// Where it was not met, up to the bound `crate::finding::Findings` applies.
        places: Vec<Finding>,
        /// How many places in total, including any past that bound.
        total: usize,
    },
    /// Not checked, with the reason. `doc/questions/Q20`.
    Unchecked(&'static str),
}

/// One requirement's row in a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judgement {
    /// The requirement's stable identifier.
    pub id: &'static str,
    /// Its clause, as cited for the target that was asked for.
    pub citation: String,
    /// What it asks, in this crate's own words.
    pub asks: &'static str,
    /// The approved erratum that changes this requirement, where one exists.
    ///
    /// Printed beside the citation rather than folded into it, because a reader checking a
    /// verdict against their own copy of the standard has the *published* sentence in front of
    /// them and needs to be told which correction this crate applied.
    pub amended_by: Option<crate::errata::Erratum>,
    /// What became of it.
    pub outcome: Outcome,
}

/// The answer to "does this document conform".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every requirement this crate checked was met.
    ///
    /// **Read it with [`Report::unchecked`].** The name is deliberately not `Conformant`: this
    /// crate reports what it checked, and the difference between that and the standard's whole
    /// clause 6 is the report's second column rather than a rounding error.
    Conforms,
    /// At least one checked requirement was not met.
    Fails,
}

/// What holding one document to one target produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The target the document was held to.
    pub target: Target,
    /// Every requirement that binds the target, in table order, with its outcome.
    pub judgements: Vec<Judgement>,
}

impl Report {
    /// The verdict, which is `Fails` if any checked requirement was not met.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        if self
            .judgements
            .iter()
            .any(|judgement| matches!(judgement.outcome, Outcome::Failed { .. }))
        {
            Verdict::Fails
        } else {
            Verdict::Conforms
        }
    }

    /// The requirements that were not met.
    pub fn failures(&self) -> impl Iterator<Item = &Judgement> {
        self.judgements
            .iter()
            .filter(|judgement| matches!(judgement.outcome, Outcome::Failed { .. }))
    }

    /// The requirements this crate did not check, which every verdict has to be read against.
    pub fn unchecked(&self) -> impl Iterator<Item = &Judgement> {
        self.judgements
            .iter()
            .filter(|judgement| matches!(judgement.outcome, Outcome::Unchecked(_)))
    }

    /// How many of the binding requirements were actually checked.
    #[must_use]
    pub fn checked(&self) -> usize {
        self.judgements
            .iter()
            .filter(|judgement| !matches!(judgement.outcome, Outcome::Unchecked(_)))
            .count()
    }

    /// The report as text, for a person.
    ///
    /// The shape is fixed by the rule above: the verdict, then **how much of the target it is a
    /// verdict over**, then the failures with their places, then the requirements that were not
    /// checked. The last section is never omitted, even when it is empty, because a reader who
    /// has learned to look for it must be able to see that it is empty rather than absent.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let verdict = match self.verdict() {
            Verdict::Conforms => "conforms",
            Verdict::Fails => "does not conform",
        };
        let _ = writeln!(out, "{}: {verdict}", self.target);
        let _ = writeln!(
            out,
            "  {} of {} requirements checked",
            self.checked(),
            self.judgements.len()
        );
        let failures: Vec<&Judgement> = self.failures().collect();
        if !failures.is_empty() {
            let _ = writeln!(out, "\nfailed:");
            for judgement in failures {
                let Outcome::Failed { places, total } = &judgement.outcome else {
                    continue;
                };
                let _ = writeln!(
                    out,
                    "  {} ({}) — {}",
                    judgement.id, judgement.citation, judgement.asks
                );
                if let Some(erratum) = judgement.amended_by {
                    let _ = writeln!(
                        out,
                        "      as corrected by {} erratum #{}: {}",
                        erratum.standard, erratum.issue, erratum.change
                    );
                }
                for place in places {
                    let _ = writeln!(out, "      {}: {}", render_place(place), place.what);
                }
                if *total > places.len() {
                    let _ = writeln!(
                        out,
                        "      … and {} more",
                        total.saturating_sub(places.len())
                    );
                }
            }
        }
        let unchecked: Vec<&Judgement> = self.unchecked().collect();
        let _ = writeln!(out, "\nnot checked ({}):", unchecked.len());
        for judgement in unchecked {
            let Outcome::Unchecked(why) = judgement.outcome else {
                continue;
            };
            let _ = writeln!(out, "  {} ({}) — {why}", judgement.id, judgement.citation);
        }
        out
    }
}

/// One finding's place, as a person reads it.
fn render_place(finding: &Finding) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(page) = finding.place.page {
        // One-based, because a report is read beside a viewer and every viewer counts from one.
        parts.push(format!("page {}", page.saturating_add(1)));
    }
    if let Some(object) = finding.place.object {
        parts.push(format!("object {} {}", object.number, object.generation));
    }
    if let Some(name) = &finding.place.name {
        parts.push(name.clone());
    }
    if parts.is_empty() {
        "the file".to_owned()
    } else {
        parts.join(", ")
    }
}
