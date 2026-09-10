//! What one conversion did, as a report rather than a log.
//!
//! `doc/adr/0927`: the owner's four permissions to write something a producer did not all carry one
//! condition, that **what was written is reported**, named per document rather than inferable from
//! a diff. [`Conversion`] is that report — what conformed already, what was decided and under which
//! clause, what was refused and why, what was lost with authorisation, and what holding the
//! *output* to the target found — and it reaches a caller through [`crate::Report::archive`]
//! whether or not a file was written.
//!
//! Nothing here changes a document or takes a decision. It words what the other three files did,
//! for a person and for `--json` alike.
use std::collections::BTreeSet;

use pdf_archive::{Judgement, MisusedProperty, Outcome, Target};

use crate::json::Value;

use super::decision::Decision;
use super::prepare::{DestinationProfile, WrittenAppearance};

/// One requirement the input failed, with what was decided and what was done about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decided {
    /// The `pdf_archive` requirement identifier.
    pub requirement: &'static str,
    /// Its clause, as cited for the target that was asked for.
    pub citation: String,
    /// What it asks, in `pdf_archive`'s own words.
    pub asks: &'static str,
    /// How many places the input failed it, including any past the validator's own bound.
    pub places: usize,
    /// What the converter decided.
    pub decision: Decision,
    /// How many places the rewrite actually touched.
    ///
    /// Zero for a decision that performed none, and for a requirement the writer met by
    /// construction — there is no place to count when the whole file is the rewrite.
    pub changed: usize,
}

/// One requirement the validator did not check, carried into the conversion's report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotChecked {
    /// The requirement identifier.
    pub requirement: &'static str,
    /// Its clause.
    pub citation: String,
    /// Why it was not checked, in the validator's own words.
    ///
    /// **Never shortened and never softened.** `doc/questions/A20`: the reason *is* the report,
    /// and a conversion that trimmed it would be making its own verdict look better than the
    /// evidence for it.
    pub because: &'static str,
}

/// What holding the *output* to the target found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Achieved {
    /// Whether every requirement the validator checks was met by the output.
    pub conforms: bool,
    /// Every requirement the output still fails, by identifier.
    pub still_failing: Vec<&'static str>,
    /// Every requirement the *input* met and the output does not.
    ///
    /// A regression is what stops the file being written at all: a conversion that broke a
    /// requirement its source satisfied has not converted the document, it has damaged it.
    pub regressions: Vec<&'static str>,
    /// How many requirements the validator checked, of those that bind the target and are
    /// about a file at all.
    pub checked: usize,
}

/// What one conversion did, per document.
///
/// The report `doc/adr/0927` makes the condition of the owner's four permissions, and section 7 of
/// `doc/pdf-a-conversion-limits.md` the promise it keeps: what conformed already, what was
/// changed and under which clause, what was refused and why, what was lost with authorisation
/// — and the list of requirements the verdict is a verdict *over*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversion {
    /// Which source.
    pub source: usize,
    /// The target asked for.
    pub target: Target,
    /// Every requirement the input already met, by identifier.
    pub conformed: Vec<&'static str>,
    /// Every requirement the input failed, with the decision taken about it.
    pub decided: Vec<Decided>,
    /// Every requirement the validator did not check.
    pub not_checked: Vec<NotChecked>,
    /// What the output was found to be, where one was written.
    ///
    /// `None` means no file was written: some decision refused, and [`Conversion::decided`]
    /// says which.
    pub achieved: Option<Achieved>,
    /// The destination profile the output intent this conversion added names.
    ///
    /// `None` where no output intent was added. `doc/questions/A18` makes this half of the
    /// report the condition on the permission: a conversion that adds an output intent has
    /// changed what every device colour in the file means to a conforming reader, and a user is
    /// entitled to be told which profile decided that and whose profile it is.
    ///
    /// **Named for either of the two rewrites that use it**: the output intent states it as its
    /// destination profile, and the `/DefaultCMYK` states the same object as the alternate space
    /// its tint transform's result is interpreted in.
    pub profile: Option<DestinationProfile>,
    /// What this conversion wrote into the document's own `xmpMM:History`, where it wrote
    /// anything.
    ///
    /// `doc/questions/A48`'s second condition, and the reason it is a field rather than a line of
    /// prose: the permission to write a `/DefaultCMYK` is conditional on the clause being named
    /// in the file's own provenance, so a report that could not say whether the entry was written
    /// could not say whether the permission had been honoured. `doc/adr/0927` has the argument.
    pub recorded: Option<String>,
    /// Every metadata property this conversion removed, with the value that was there.
    ///
    /// `doc/pdf-a-conversion-limits.md` section 3.9's condition on the loss, and the reason it is
    /// a list rather than a count: a property that is gone leaves nothing in the output to notice,
    /// so the report is the only place a user can read what the file used to say — and it names
    /// the namespace as well as the property, because two schemas may spell a local name the same
    /// way. With those three a user can put the property back by hand, or go back to the source
    /// and correct it there.
    pub removed: Vec<MisusedProperty>,
    /// Every annotation appearance this conversion constructed.
    ///
    /// `doc/questions/A21`'s condition on the permission, in the answer's own words: report every
    /// appearance written, so the difference between the producer's file and ours is visible in
    /// the report rather than only in the bytes.
    pub appearances: Vec<WrittenAppearance>,
}

impl Conversion {
    /// Whether every decision lets the conversion proceed.
    #[must_use]
    pub fn proceeds(&self) -> bool {
        self.decided
            .iter()
            .all(|decided| decided.decision.proceeds())
    }

    /// The report as RFC 0002 section 4.5's JSON.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::Object(vec![
            ("source".to_owned(), Value::count(self.source)),
            ("target".to_owned(), Value::text(self.target.to_string())),
            (
                "conformed".to_owned(),
                Value::Array(self.conformed.iter().map(|id| Value::text(*id)).collect()),
            ),
            (
                "decided".to_owned(),
                Value::Array(self.decided.iter().map(Decided::to_json).collect()),
            ),
            (
                "not_checked".to_owned(),
                Value::Array(self.not_checked.iter().map(NotChecked::to_json).collect()),
            ),
            (
                "achieved".to_owned(),
                self.achieved
                    .as_ref()
                    .map_or(Value::Null, Achieved::to_json),
            ),
            (
                "icc_profile".to_owned(),
                self.profile
                    .as_ref()
                    .map_or(Value::Null, DestinationProfile::to_json),
            ),
            (
                "recorded_in_xmp_history".to_owned(),
                self.recorded
                    .as_ref()
                    .map_or(Value::Null, |action| Value::text(action.clone())),
            ),
            (
                "removed_metadata_properties".to_owned(),
                Value::Array(self.removed.iter().map(removed_to_json).collect()),
            ),
            (
                "constructed_appearances".to_owned(),
                Value::Array(
                    self.appearances
                        .iter()
                        .map(WrittenAppearance::to_json)
                        .collect(),
                ),
            ),
        ])
    }

    /// The report as text, for a person.
    ///
    /// The shape is the one section 7 of that document asks for: what the target was, what
    /// conformed, what was done and under which clause, what was refused and why — and the
    /// not-checked list last, never
    /// omitted, so that a reader who has learned to look for it can see it is empty rather than
    /// absent.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}: {} of the requirements this file was held to were already met",
            self.target,
            self.conformed.len()
        );
        // One rewrite can answer a dozen requirements — an output intent answers most of the
        // colour subclause at once — and its sentence about what the file now asserts is the same
        // sentence every time. Said once, so that a reader meets it rather than skims past it;
        // the JSON keeps it against each decision, where a machine wants it.
        let mut said: BTreeSet<&'static str> = BTreeSet::new();
        for decided in &self.decided {
            let repeated = match decided.decision {
                Decision::Stated { reinterprets, .. } => !said.insert(reinterprets),
                _ => false,
            };
            let _ = writeln!(
                out,
                "  {} ({}) — {} place(s)\n      {}",
                decided.requirement,
                decided.citation,
                decided.places,
                describe_decision(decided, repeated)
            );
        }
        if let Some(profile) = &self.profile {
            let _ = writeln!(
                out,
                "  the ICC profile this conversion states is {}{}, over {}",
                profile.source.describe(),
                profile
                    .describes
                    .as_ref()
                    .map_or_else(String::new, |name| format!(" ({name})")),
                profile.space
            );
            if let Some(copyright) = &profile.copyright {
                let _ = writeln!(out, "      its copyright tag says: {copyright}");
            }
        }
        if let Some(recorded) = &self.recorded {
            let _ = writeln!(
                out,
                "  recorded in this file's own xmpMM:History: {recorded}"
            );
        }
        out.push_str(&self.render_what_was_written());
        if let Some(achieved) = &self.achieved {
            let verdict = if achieved.conforms {
                "conforms"
            } else {
                "does not conform"
            };
            let _ = writeln!(
                out,
                "  the output was held to {} again and {verdict}, over {} requirements",
                self.target, achieved.checked
            );
            for id in &achieved.still_failing {
                let sharper = if achieved.regressions.contains(id) {
                    " — which this document met, so the conversion broke it"
                } else {
                    ""
                };
                let _ = writeln!(out, "      still failed: {id}{sharper}");
            }
            if !achieved.conforms {
                let _ = writeln!(
                    out,
                    "  no file was written, because a conversion whose result is not {} has not \
                     converted the document",
                    self.target
                );
            }
        } else {
            let _ = writeln!(out, "  no file was written");
        }
        let _ = writeln!(out, "  not checked ({}):", self.not_checked.len());
        for row in &self.not_checked {
            let _ = writeln!(out, "      {} ({})", row.requirement, row.citation);
        }
        out
    }
}

impl Conversion {
    /// The two lists the owner's conditions make part of the report rather than of a diff.
    ///
    /// `doc/questions/A21` asks for every appearance written and
    /// `doc/pdf-a-conversion-limits.md` section 3.9 for every property removed, each named rather
    /// than counted. They are one function because they are one obligation seen twice: what this
    /// program wrote that the producer did not, and what it took away that the producer did.
    fn render_what_was_written(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        if !self.appearances.is_empty() {
            let _ = writeln!(
                out,
                "  {} appearance stream(s) constructed, each from the entries the annotation's \
                 own subtype clause states:",
                self.appearances.len()
            );
            for appearance in &self.appearances {
                let _ = writeln!(
                    out,
                    "      a {} annotation on page {}",
                    appearance.subtype,
                    appearance.page.saturating_add(1)
                );
            }
        }
        if !self.removed.is_empty() {
            let _ = writeln!(
                out,
                "  {} metadata propert(ies) removed, because the predefined schema each names \
                 does not define the value it held:",
                self.removed.len()
            );
            for property in &self.removed {
                let _ = writeln!(
                    out,
                    "      {} in {} held {} — {}",
                    property.spelled, property.name.namespace, property.stated, property.because
                );
            }
        }
        out
    }
}

/// One removed property as JSON.
fn removed_to_json(property: &MisusedProperty) -> Value {
    Value::Object(vec![
        ("property".to_owned(), Value::text(property.spelled.clone())),
        (
            "namespace".to_owned(),
            Value::text(property.name.namespace.clone()),
        ),
        ("local".to_owned(), Value::text(property.name.local.clone())),
        ("stated".to_owned(), Value::text(property.stated.clone())),
        ("because".to_owned(), Value::text(property.because.clone())),
    ])
}

/// One decision, worded for a person.
pub(super) fn describe_decision(decided: &Decided, repeated: bool) -> String {
    match decided.decision {
        Decision::Mechanical(rewrite) => {
            format!(
                "changed, losing nothing: {} ({} done)",
                rewrite.describe(),
                decided.changed
            )
        }
        Decision::Stated {
            rewrite,
            reinterprets,
        } => {
            let sentence = if repeated {
                "the same interpretation as above"
            } else {
                reinterprets
            };
            format!(
                "changed, stating an interpretation the standard defines: {} ({} done)\n      {}",
                rewrite.describe(),
                decided.changed,
                sentence
            )
        }
        Decision::Authorised { loss, rewrite } => format!(
            "changed with your authorisation: {} — {} ({} done)",
            rewrite.describe(),
            loss.describe(),
            decided.changed
        ),
        Decision::Unauthorised { loss, rewrite } => format!(
            "not done, because it loses something nobody authorised: {} — {}; \
             --authorise {} allows it",
            rewrite.describe(),
            loss.describe(),
            loss.word()
        ),
        Decision::Refused(because) => format!("refused: {}", because.sentence()),
    }
}

impl Decided {
    /// One decision as JSON.
    fn to_json(&self) -> Value {
        let mut fields = vec![
            ("requirement".to_owned(), Value::text(self.requirement)),
            ("clause".to_owned(), Value::text(self.citation.clone())),
            ("asks".to_owned(), Value::text(self.asks)),
            ("places".to_owned(), Value::count(self.places)),
            ("decision".to_owned(), Value::text(self.decision.word())),
            ("changed".to_owned(), Value::count(self.changed)),
        ];
        match self.decision {
            Decision::Mechanical(rewrite) => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
            }
            Decision::Stated {
                rewrite,
                reinterprets,
            } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
                fields.push(("reinterprets".to_owned(), Value::text(reinterprets)));
            }
            Decision::Authorised { loss, rewrite } | Decision::Unauthorised { loss, rewrite } => {
                fields.push(("rewrite".to_owned(), Value::text(rewrite.word())));
                fields.push(("loss".to_owned(), Value::text(loss.word())));
                fields.push(("loses".to_owned(), Value::text(loss.describe())));
            }
            Decision::Refused(because) => {
                fields.push(("because".to_owned(), Value::text(because.word())));
                fields.push(("reason".to_owned(), Value::text(because.sentence())));
            }
        }
        Value::Object(fields)
    }
}

impl NotChecked {
    /// One of the validator's unchecked rows, carried across.
    pub(super) fn of(judgement: &Judgement) -> Self {
        Self {
            requirement: judgement.id,
            citation: judgement.citation.clone(),
            because: match judgement.outcome {
                Outcome::Unchecked(why) => why,
                // `Report::unchecked` yields only `Outcome::Unchecked`, so this arm is
                // unreachable; it names the bug rather than panicking.
                _ => "the validator reported no reason",
            },
        }
    }

    /// One unchecked requirement as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("requirement".to_owned(), Value::text(self.requirement)),
            ("clause".to_owned(), Value::text(self.citation.clone())),
            ("not_checked_because".to_owned(), Value::text(self.because)),
        ])
    }
}

impl Achieved {
    /// The output's verdict as JSON.
    fn to_json(&self) -> Value {
        Value::Object(vec![
            ("conforms".to_owned(), Value::Bool(self.conforms)),
            ("checked".to_owned(), Value::count(self.checked)),
            (
                "still_failing".to_owned(),
                Value::Array(
                    self.still_failing
                        .iter()
                        .map(|id| Value::text(*id))
                        .collect(),
                ),
            ),
            (
                "regressions".to_owned(),
                Value::Array(self.regressions.iter().map(|id| Value::text(*id)).collect()),
            ),
        ])
    }
}
