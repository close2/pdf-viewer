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
    if conversion.profile.is_none() {
        conversion.profile = second.profile;
    }
    conversion.recorded = match (conversion.recorded.take(), second.recorded) {
        (Some(first), Some(second)) => Some(format!("{first}; {second}")),
        (first, second) => first.or(second),
    };
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
}
