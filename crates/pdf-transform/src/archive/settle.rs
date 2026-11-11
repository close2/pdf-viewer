//! A conversion's output converted once more, as a source of its own.
//!
//! An appearance this conversion constructs for a widget or an annotation that had none
//! (ISO 19005-2 section 6.3.3, ISO 32000-2 §12.5.5 and §12.7.4.3) is content no pass over the
//! source ever saw: the font it names is not in the survey the font pass embedded from, and the
//! device colour it selects is not in the one the default colour spaces were written for. The
//! output then fails requirements the source *met* (the second verdict's `regressions`,
//! `doc/adr/0947`) or fails, at the appearance's own font, one the source failed elsewhere, and
//! refusing the document over them refuses a file whose every remedy exists.
//!
//! The same is true of two things the first pass writes without having judged them: a face
//! substituted for a font the source did not embed, whose program's `cmap` no row of the source
//! read (ISO 19005-2 section 6.2.11.6), and an object the source's check never enumerated because
//! its cross-reference chain was cut and the reader found it by its own header (ISO 32000-2
//! Table 15's `/Prev`), which the output's fresh table lists. Each shows as a requirement the
//! source met and the output fails, so that is the second condition for the pass
//! (`doc/adr/2795`). Where the source failed the requirement too, at places its check did
//! enumerate, the first pass changes those and the output still fails it at the object the check
//! never reached: a requirement the first pass changed and the output still fails is the third
//! condition (`doc/adr/2803`).
//!
//! The remedy is the one the converter already has: the output, which now holds those
//! appearances as ordinary content, is converted as a source. One extra pass at most (`settling`
//! is cleared for the nested conversion), and it is taken only when the first output would
//! otherwise be refused, so a conversion that stands is not converted twice (`doc/adr/2534`).

use pdf_syntax::Document;

use super::{ArchivePlan, Conversion, Report, hold_the_output_to_the_target, run_pass};
use crate::{Refusal, Sinks};

/// What the second conversion produced, held to the target against the *source's* verdict.
pub(super) struct Second {
    /// The second conversion's file.
    pub(super) bytes: Vec<u8>,
    /// That file, opened.
    pub(super) output: Document,
    /// The verdict on it, with `regressions` read against the source.
    pub(super) achieved: super::Achieved,
    /// The second conversion's own report, for what it decided and embedded.
    pub(super) conversion: Conversion,
}

/// Converts `first` once more under the same plan, or `None` where it cannot be made to stand.
///
/// `None` is not an error: the first conversion's own refusal then stands. A refusal of the
/// nested conversion is dropped for that reason, and nothing is written to `sinks`.
pub(super) fn second_conversion(
    plan: &ArchivePlan,
    first: &[u8],
    input: &pdf_archive::Report,
    budget: &crate::Budget,
) -> Option<Second> {
    let document = Document::open_with_limits(first.to_vec(), budget.limits).ok()?;
    let sinks = crate::MemorySinks::new();
    let mut report = Report::default();
    let ran: Result<(), Refusal> = run_pass(
        plan,
        0,
        std::slice::from_ref(&document),
        &sinks as &dyn Sinks,
        &mut report,
        false,
        budget,
    );
    ran.ok()?;
    let conversion = report.archive?;
    let (_, bytes) = sinks.into_outputs().pop()?;
    let (output, achieved) = hold_the_output_to_the_target(&bytes, input, plan.target).ok()?;
    Some(Second {
        bytes,
        output,
        achieved,
        conversion,
    })
}

/// Folds what the second conversion did into the first one's report.
///
/// The requirements it decided that the first did not are added to `decided`, so the report names
/// each clause that changed something under it, and one both decided counts both passes' places; the faces it embedded and the metrics it restated
/// are the report's `substituted` and `restated`; and the id of every requirement it decided is
/// kept in [`Conversion::settled`], a row the first pass also held included, so a reader can tell
/// that a second pass ran and what it answered.
///
/// The second conversion runs the first's plan, so a loss it takes under a row naming
/// `append-statement` — a page it photographed for a face only its constructed appearances show —
/// is stated on a page it appends after the first's; [`fold_statement`] makes the report's one
/// statement name both, and the sites the second handed on are the report's `unstated` with the
/// first's.
pub(super) fn fold(conversion: &mut Conversion, second: Conversion) {
    fold_decided(
        &mut conversion.decided,
        &mut conversion.settled,
        second.decided,
    );
    conversion.substituted.extend(second.substituted);
    conversion.photographed.extend(second.photographed);
    conversion.restated.extend(second.restated);
    conversion.supplied.extend(second.supplied);
    fold_statement(&mut conversion.statement, second.statement);
    conversion.unstated.extend(second.unstated);
    if conversion.profile.is_none() {
        conversion.profile = second.profile;
    }
    conversion.recorded = match (conversion.recorded.take(), second.recorded) {
        (Some(first), Some(second)) => Some(format!("{first}; {second}")),
        (first, second) => first.or(second),
    };
}

/// The second conversion's statement joined to the first's, so the report's one statement names
/// every page a loss of either pass is stated on (`doc/rfc/0014` section 4, `doc/adr/2640`).
///
/// The second conversion's source is the first's output, which already ends in the first's
/// statement pages, and it appends its own after every page it was given; so the two are one
/// run of pages, the first's entries then the second's. The face stays the first's: the second
/// sets its text in the document's own face only because the first embedded the face it names.
fn fold_statement(
    statement: &mut Option<super::report::StatementPage>,
    second: Option<super::report::StatementPage>,
) {
    let Some(second) = second else {
        return;
    };
    let Some(first) = statement else {
        *statement = Some(second);
        return;
    };
    for site in second.sites {
        if !first.sites.contains(&site) {
            first.sites.push(site);
        }
    }
    first.pages.extend(second.pages);
    first.listed = first.listed.plus(second.listed);
}

/// The second pass's decisions joined to the first's, each requirement once.
///
/// A requirement both passes decided was met at the first pass's places and at the second's, which
/// are places in the first output the source did not have, so its one row counts both.
fn fold_decided(
    decided: &mut Vec<super::Decided>,
    settled: &mut Vec<&'static str>,
    second: Vec<super::Decided>,
) {
    for row in second {
        settled.push(row.requirement);
        if let Some(held) = decided
            .iter_mut()
            .find(|held| held.requirement == row.requirement)
        {
            held.places = held.places.saturating_add(row.places);
            held.changed = held.changed.saturating_add(row.changed);
            continue;
        }
        decided.push(row);
    }
}

#[cfg(test)]
mod tests {
    use super::super::Decided;
    use super::super::report::StatementPage;
    use super::*;

    fn decided(places: usize, changed: usize) -> Decided {
        Decided {
            requirement: "fonts/font-programs-embedded",
            citation: String::new(),
            asks: "",
            places,
            decision: super::super::Decision::Refused(super::super::Because::NotBuiltYet("")),
            changed,
        }
    }

    #[test]
    fn a_requirement_both_passes_decided_counts_both_passes_places() {
        let mut first = vec![decided(1, 1)];
        let mut settled = Vec::new();
        fold_decided(&mut first, &mut settled, vec![decided(2, 2)]);
        assert_eq!(first.len(), 1, "one row for the requirement");
        assert_eq!((first[0].places, first[0].changed), (3, 3));
        assert_eq!(settled, vec!["fonts/font-programs-embedded"]);
    }

    fn stated(pages: Vec<usize>, face: super::super::report::SetIn) -> StatementPage {
        StatementPage {
            sites: vec!["fonts/font-programs-embedded"],
            pages,
            listed: super::super::report::Listed {
                font_records: 1,
                ..Default::default()
            },
            face,
        }
    }

    #[test]
    fn a_page_the_second_pass_states_is_on_the_one_statement_the_report_names() {
        let ships = super::super::report::SetIn::AFaceThisProgramShips("serif".to_owned());
        let mut first = Some(stated(vec![1], ships.clone()));
        let second = stated(vec![2], super::super::report::SetIn::TheDocumentsOwnFace);
        fold_statement(&mut first, Some(second.clone()));
        assert_eq!(
            first,
            Some(StatementPage {
                pages: vec![1, 2],
                listed: super::super::report::Listed {
                    font_records: 2,
                    ..Default::default()
                },
                ..stated(vec![], ships)
            })
        );
        let mut none = None;
        fold_statement(&mut none, Some(second.clone()));
        assert_eq!(
            none,
            Some(second),
            "the second's statement where the first composed none"
        );
    }
}
