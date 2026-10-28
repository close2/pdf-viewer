//! A site's ordered answer, `try = [...]`: `doc/rfc/0014` section 3, adopted by the owner in `A706`.
//!
//! # What a list is
//!
//! One word per step, of a closed grammar whose kind before the first colon is `doc/rfc/0007`
//! section 2's vocabulary: `stop`, `discard`, `append-statement`, `preserve`, `preserve:attach`,
//! `preserve:append`, `derive`, `derive:<tool>:attach`, `derive:<tool>:append`, `supply` and
//! `depart:<name>`. A bare `preserve` or `derive` reads the site's own keys (`placement`,
//! `tool`) exactly as `remedy` does, so `remedy = "x"` with `on-failure = "y"` is the list
//! `["x", "y"]` and a file written before the list existed reads unchanged.
//!
//! # How it is resolved
//!
//! **Against the target, when the file is read.** Each step is made into the row it would be on
//! its own and asked the questions [`super::check_rows`] asks every row; a step a target refuses
//! by name ([`ConfigError::NotBuiltThatWay`]) is passed over and listed with that sentence, which
//! is what lets one line serve six targets. The steps that remain are the row's answer, in order:
//! the first is its remedy and the second what is done with an object the first could not
//! answer. A list no step of which a target admits is an error at read time, so an answer that
//! could never act is found when the file is read rather than when a document arrives.
//!
//! # What this version carries out
//!
//! **Two steps per object**, which is what the executor holds: a keeping step, then `stop`,
//! `discard` or `append-statement` for each object it could not keep. Where a second step is
//! carried out is the executor's own list — the embedded-file rows' `preserve:append`, the only
//! row whose outcome is reported per object (`doc/adr/2536`) — and a second step anywhere else,
//! or a keeping step in second place, is refused by name rather than read and not done. A step
//! after one that never hands an object on (`stop`, `discard`, `append-statement`, `supply`) is
//! an error, because it would never be taken: `stop` is implicit at the end of every list.

use std::collections::BTreeMap;

use pdf_archive::Target;

use crate::tool::Tool;

use super::super::toml::Table;
use super::{ConfigError, DERIVABLE, Kind, Placement, Row, check_rows};

/// The step that takes a site's loss and states it on an appended page (`doc/rfc/0014` section 4).
pub(in crate::archive) const APPEND_STATEMENT: &str = "append-statement";

/// Why `append-statement` is built only at the two embedded-file rows.
pub(in crate::archive) const STATED_ONLY_PER_OBJECT: &str = "the page append-statement appends \
     lists each object the loss took, so it is built where the loss is reported one object at a \
     time: the two embedded-file rows (ISO 19005-2 section 6.8, ISO 19005-4 section 6.9). At \
     every other site the step is not built yet, and `discard` takes the loss without the page";

/// Why `derive:<tool>:append` is passed over in this version.
const DERIVED_PAGES_UNBUILT: &str = "a derived file appended as pages is doc/rfc/0014 section \
     5's third stage, which this version does not build; derive:<tool>:attach attaches the \
     derived file";

/// Why `depart:<name>` is passed over as a step.
const DEPARTURE_IS_ITS_OWN_TABLE: &str = "a departure is stated in its own [depart.\"…\"] table, \
     whose predicate decides which objects it covers, and this version reads no departure as a \
     step of a site's list";

/// Why a second step is refused where the executor carries none out.
const SECOND_STEP_UNBUILT: &str = "this version carries out a second step only after \
     preserve:append at the two embedded-file rows, whose outcome is one per file \
     (doc/adr/2536), and only where that step is stop, discard or append-statement; anywhere \
     else the list holds one step and the implicit stop";

/// One step of a site's list this target does not admit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// The site.
    pub site: String,
    /// The step, as the file wrote it.
    pub step: String,
    /// Why this target passes it over.
    pub why: &'static str,
}

/// What the page `append-statement` appends says was tried before the loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// The step taken before `append-statement` for each object, where there was one.
    pub before: Option<String>,
    /// The steps this target passed over, each with its sentence.
    pub skipped: Vec<(String, &'static str)>,
}

/// One word of the grammar, read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step<'a> {
    Stop,
    Discard,
    AppendStatement,
    Preserve(Option<Placement>),
    Derive {
        tool: Option<&'a str>,
        placement: Option<Placement>,
    },
    Supply,
    Depart,
}

impl<'a> Step<'a> {
    /// The step a word names, or `None` for a word outside the grammar.
    fn parse(word: &'a str) -> Option<Self> {
        let mut parts = word.split(':');
        let kind = parts.next()?;
        let rest: Vec<&str> = parts.collect();
        match (kind, rest.as_slice()) {
            ("stop", []) => Some(Self::Stop),
            ("discard", []) => Some(Self::Discard),
            (APPEND_STATEMENT, []) => Some(Self::AppendStatement),
            ("supply", []) => Some(Self::Supply),
            ("preserve", []) => Some(Self::Preserve(None)),
            ("preserve", [placement]) => {
                Placement::parse(placement).map(|p| Self::Preserve(Some(p)))
            }
            ("derive", []) => Some(Self::Derive {
                tool: None,
                placement: None,
            }),
            ("derive", [tool, placement]) if !tool.is_empty() => {
                Placement::parse(placement).map(|placement| Self::Derive {
                    tool: Some(tool),
                    placement: Some(placement),
                })
            }
            ("depart", [name]) if !name.is_empty() => Some(Self::Depart),
            _ => None,
        }
    }

    /// Whether an object this step could not answer is handed to the next one.
    ///
    /// `stop` refuses and `discard` always holds; `supply` writes the operator's fact or refuses
    /// with its own sentence, which this version hands to no later step.
    const fn hands_on(self) -> bool {
        matches!(self, Self::Preserve(_) | Self::Derive { .. })
    }
}

/// Reads `try`, checking it against `remedy` and `on-failure` and against the grammar.
///
/// Empty for a table stating no `try`, which then answers with `remedy` as it always has.
fn listed(tbl: &Table, site: &str) -> Result<Vec<String>, ConfigError> {
    let Some(value) = tbl.get("try") else {
        return Ok(Vec::new());
    };
    for other in ["remedy", "on-failure"] {
        if tbl.get(other).is_some() {
            return Err(ConfigError::TryBesideRemedy {
                line: tbl.line,
                site: site.to_owned(),
                other,
            });
        }
    }
    let Some(words) = value.as_list() else {
        return Err(ConfigError::WrongValue {
            line: tbl.line,
            site: site.to_owned(),
            key: "try".to_owned(),
            wanted: "a list of step words",
            found: value.kind(),
        });
    };
    if let Some(unknown) = words.iter().find(|word| Step::parse(word).is_none()) {
        return Err(ConfigError::UnknownStep {
            line: tbl.line,
            site: site.to_owned(),
            step: (*unknown).to_owned(),
        });
    }
    if words.is_empty() {
        return Err(ConfigError::NoStepAdmitted {
            line: tbl.line,
            site: site.to_owned(),
            target: "of any part".to_owned(),
            skipped: "the list is empty".to_owned(),
        });
    }
    Ok(words.into_iter().map(str::to_owned).collect())
}

/// The row one step makes on its own, or the sentence it is passed over with.
fn alone(row: &Row, step: Step<'_>) -> Result<Result<Row, &'static str>, ConfigError> {
    let mut alone = Row {
        on_failure: Kind::Stop,
        states_the_loss: false,
        steps: Vec::new(),
        skipped: Vec::new(),
        ..row.clone()
    };
    match step {
        Step::Stop => alone.remedy = Kind::Stop,
        Step::Discard => alone.remedy = Kind::Discard,
        Step::AppendStatement => {
            alone.remedy = Kind::Discard;
            alone.states_the_loss = true;
        }
        Step::Supply => alone.remedy = Kind::Supply,
        Step::Preserve(placement) => {
            alone.remedy = Kind::Preserve;
            alone.placement = placement.or(row.placement);
        }
        Step::Derive {
            placement: Some(Placement::Append),
            ..
        } => return Ok(Err(DERIVED_PAGES_UNBUILT)),
        Step::Derive { tool, .. } => {
            alone.remedy = Kind::Derive;
            alone.tool = tool.map(str::to_owned).or_else(|| row.tool.clone());
            // `A55`'s guardrail, for the step as for the remedy: the site and the tool, both.
            if alone.tool.is_none() {
                return Err(ConfigError::DeriveWithoutTool {
                    line: row.line,
                    site: row.site.clone(),
                });
            }
        }
        Step::Depart => return Ok(Err(DEPARTURE_IS_ITS_OWN_TABLE)),
    }
    Ok(Ok(alone))
}

/// The row a `try` list resolves to at one target, with the steps it passed over recorded.
///
/// # Errors
///
/// [`ConfigError::NoStepAdmitted`] where the target admits no step, [`ConfigError::StepNeverReached`]
/// for a step after one that never hands an object on, [`ConfigError::NotBuiltThatWay`] for a
/// second step the executor does not carry out, and any error a step's own row is (an undeclared
/// tool, a `derive` naming none).
fn resolved(row: &Row, tools: &BTreeMap<String, Tool>, target: Target) -> Result<Row, ConfigError> {
    let mut admitted: Vec<(&str, Step<'_>, Row)> = Vec::new();
    let mut skipped: Vec<(String, &'static str)> = Vec::new();
    for word in &row.steps {
        // `listed` parsed every word already; one that does not parse here is still named.
        let Some(step) = Step::parse(word) else {
            return Err(ConfigError::UnknownStep {
                line: row.line,
                site: row.site.clone(),
                step: word.clone(),
            });
        };
        let candidate = match alone(row, step)? {
            Ok(candidate) => candidate,
            Err(why) => {
                skipped.push((word.clone(), why));
                continue;
            }
        };
        match check_rows(std::slice::from_ref(&candidate), tools, target) {
            Ok(()) => admitted.push((word.as_str(), step, candidate)),
            Err(ConfigError::NotBuiltThatWay { why, .. }) => skipped.push((word.clone(), why)),
            Err(other) => return Err(other),
        }
    }
    let Some((first_word, first, mut resolved)) = admitted.first().cloned() else {
        return Err(ConfigError::NoStepAdmitted {
            line: row.line,
            site: row.site.clone(),
            target: target.to_string(),
            skipped: skipped
                .iter()
                .map(|(step, why)| format!("{step}: {why}"))
                .collect::<Vec<_>>()
                .join("; "),
        });
    };
    if let Some((second_word, second, _)) = admitted.get(1) {
        if !first.hands_on() {
            return Err(ConfigError::StepNeverReached {
                line: row.line,
                site: row.site.clone(),
                step: (*second_word).to_owned(),
                after: first_word.to_owned(),
            });
        }
        let carried = DERIVABLE.contains(&row.site.as_str())
            && resolved.remedy == Kind::Preserve
            && resolved.placement == Some(Placement::Append);
        if second.hands_on() || (!carried && *second != Step::Stop) {
            return Err(ConfigError::NotBuiltThatWay {
                line: row.line,
                site: row.site.clone(),
                asked: format!("the step {second_word:?} after {first_word:?}"),
                why: SECOND_STEP_UNBUILT,
            });
        }
        resolved.on_failure = match second {
            Step::Discard | Step::AppendStatement => Kind::Discard,
            _ => Kind::Stop,
        };
        resolved.states_the_loss = *second == Step::AppendStatement;
        if let Some((third_word, _, _)) = admitted.get(2) {
            return Err(ConfigError::StepNeverReached {
                line: row.line,
                site: row.site.clone(),
                step: (*third_word).to_owned(),
                after: (*second_word).to_owned(),
            });
        }
    }
    resolved.skipped = skipped;
    Ok(resolved)
}

/// What a row's statement page says was tried, where the row's loss is stated.
pub(super) fn statement(row: &Row) -> Option<Statement> {
    row.states_the_loss.then(|| Statement {
        before: (row.remedy != Kind::Discard).then(|| step_word(row)),
        skipped: row.skipped.clone(),
    })
}

/// The step word a row's first step is written as.
fn step_word(row: &Row) -> String {
    match (row.remedy, row.placement, row.tool.as_deref()) {
        (Kind::Preserve, Some(placement), _) => format!("preserve:{}", placement.word()),
        (Kind::Derive, _, Some(tool)) => format!("derive:{tool}:attach"),
        (kind, _, _) => kind.word().to_owned(),
    }
}

/// One `[site."…"]` table read into a [`Row`]: its `remedy`, its `try`, and the keys both read.
pub(super) fn site_row(tbl: &Table, site: String) -> Result<Row, ConfigError> {
    let (remedy, states_first) = super::site_remedy(tbl, &site)?;
    let steps = listed(tbl, &site)?;
    let mut row = super::row(tbl, site, remedy)?;
    row.states_the_loss |= states_first;
    row.steps = steps;
    Ok(row)
}

/// Resolves every row stating `try`, in place.
pub(super) fn resolve_all(
    sites: &mut [Row],
    tools: &BTreeMap<String, Tool>,
    target: Target,
) -> Result<(), ConfigError> {
    for row in sites.iter_mut().filter(|row| !row.steps.is_empty()) {
        // A site this target does not bind is never asked, so its list is read for its words
        // and not resolved: a step a part that binds the site would refuse is no error here.
        if super::requirement_binds(&row.site, target) {
            *row = resolved(row, tools, target)?;
        } else {
            row.steps.clear();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pdf_archive::{Flavour, Level, Target};

    use super::super::{ConfigError, Configuration, Kind};

    const TWO_B: Target = Target::Two(Level::B);
    const SITE: &str = "embedded-files/embedded-file-is-itself-pdfa";

    fn listing(keys: &str) -> String {
        format!("[site.\"{SITE}\"]\n{keys}\n")
    }

    #[test]
    fn a_list_passes_over_the_step_the_target_does_not_admit_and_lists_it() {
        let text = listing(r#"try = ["preserve:attach", "preserve:append", "append-statement"]"#);
        let config = Configuration::read(&text, TWO_B).expect("the list reads");
        let skipped = config.skipped();
        assert_eq!(skipped.len(), 1, "{skipped:?}");
        assert_eq!(skipped[0].step, "preserve:attach");
        let rows = config.preservations(TWO_B);
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert!(
            rows[0].by_page,
            "preserve:append is the first admitted step"
        );
        assert_eq!(rows[0].no_page_carries, Kind::Discard);
        let statement = rows[0].statement.as_ref().expect("the loss is stated");
        assert_eq!(statement.before.as_deref(), Some("preserve:append"));
        assert_eq!(statement.skipped.len(), 1);
    }

    #[test]
    fn remedy_with_on_failure_is_the_two_step_list() {
        let listed = Configuration::read(
            &listing(r#"try = ["preserve:append", "append-statement"]"#),
            TWO_B,
        )
        .expect("the list reads");
        let keyed = Configuration::read(
            &listing(
                "remedy = \"preserve\"\nplacement = \"append\"\non-failure = \"append-statement\"",
            ),
            TWO_B,
        )
        .expect("the keys read");
        assert_eq!(listed.preservations(TWO_B), keyed.preservations(TWO_B));
    }

    #[test]
    fn a_list_beside_a_remedy_is_an_error_naming_the_site() {
        let error =
            Configuration::read(&listing("remedy = \"discard\"\ntry = [\"discard\"]"), TWO_B)
                .expect_err("both forms");
        assert!(
            matches!(error, ConfigError::TryBesideRemedy { ref site, .. } if site == SITE),
            "{error}"
        );
    }

    #[test]
    fn a_word_outside_the_grammar_is_named() {
        let error = Configuration::read(&listing(r#"try = ["keep"]"#), TWO_B).expect_err("unknown");
        assert!(
            matches!(error, ConfigError::UnknownStep { ref step, .. } if step == "keep"),
            "{error}"
        );
    }

    #[test]
    fn an_unknown_on_failure_word_is_named_rather_than_read_as_stop() {
        let error = Configuration::read(
            &listing("remedy = \"preserve\"\nplacement = \"append\"\non-failure = \"drop\""),
            TWO_B,
        )
        .expect_err("unknown");
        assert!(
            matches!(error, ConfigError::UnknownRemedy { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_list_no_step_of_which_the_target_admits_is_an_error_at_read_time() {
        let text = "[site.\"file-structure/no-encryption\"]\ntry = [\"append-statement\"]\n";
        let error = Configuration::read(text, TWO_B).expect_err("nothing admitted");
        assert!(
            matches!(error, ConfigError::NoStepAdmitted { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_step_after_one_that_always_holds_is_never_reached() {
        let error =
            Configuration::read(&listing(r#"try = ["discard", "append-statement"]"#), TWO_B)
                .expect_err("unreached");
        assert!(
            matches!(error, ConfigError::StepNeverReached { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_keeping_step_in_second_place_is_refused_by_name() {
        let text = format!(
            "{}[tool.to-pdf]\nprogram = \"/usr/bin/true\"\nargs = [\"{{in}}\", \"{{out}}\"]\n\
             expects = \"application/pdf\"\ntimeout = \"60s\"\noutput-limit = \"1MiB\"\n",
            listing(r#"try = ["preserve:append", "derive:to-pdf:attach"]"#)
        );
        let error = Configuration::read(&text, TWO_B).expect_err("not carried");
        assert!(
            matches!(error, ConfigError::NotBuiltThatWay { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_site_the_target_does_not_bind_reads_its_list_and_asks_nothing() {
        let text = format!(
            "[site.\"{SITE}-in-the-plain-profile\"]\ntry = [\"preserve:attach\", \"preserve:append\"]\n"
        );
        let config = Configuration::read(&text, Target::Four(Flavour::F)).expect("never asked");
        assert!(config.preservations(Target::Four(Flavour::F)).is_empty());
    }
}
