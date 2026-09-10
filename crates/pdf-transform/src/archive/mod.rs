//! `archive` — one document converted to a stated part and level of ISO 19005 (PDF/A).
//!
//! # Provenance, and the one rule about quotation
//!
//! ISO 19005-2:2011 and ISO 19005-4:2020 are licensed to a single reader
//! (`doc/questions/A16`), so **nothing here quotes them**: a requirement is named by the
//! identifier `pdf-archive` gives it and cited by clause, and a reader who needs the sentence
//! opens `doc/pdfa/` at the clause. ISO 32000-2 is quoted normally, because `doc/md/` carries it
//! and `tools/conformance` checks the quotation. A section sign marks a clause of ISO 32000-2 and
//! nothing else.
//!
//! # Three stages, and the middle one is the product
//!
//! `doc/pdf-a-conversion-limits.md` is this verb's specification of behaviour, and its shape is
//! why converting is not one operation:
//!
//! 1. **Validate.** `pdf_archive::check` holds the document to the target and answers a
//!    requirement at a time. Nothing here re-reads the standard: the validator is the reading.
//! 2. **Decide.** Each requirement the document *failed* gets one of three answers, and they are
//!    three different things rather than three degrees of one thing — sections 2, 3 and 4 of
//!    `doc/pdf-a-conversion-limits.md` are a refusal, an authorised loss and a default.
//!    [`Decision`] is that answer, `decision::REMEDIES` is the table it comes from, and
//!    everything else in this module hangs off it.
//! 3. **Apply.** The rewrites the decisions call for, through the serializer this tree already
//!    has, and then **the output is validated again**. A file leaves this verb only if it
//!    conforms; anything less is refused, with the requirements it would still have failed
//!    named, and a requirement its *source* met is named again as the worse case. That is what
//!    makes the report's claim about the output a measurement rather than a promise, and it is
//!    what catches the cases the decision table cannot see.
//!
//! ADR 0947 has the argument for all three, and for the two rules the whole verb rests on:
//! nothing is changed that no failed requirement asked for, and no file is written that does not
//! conform.
//!
//! # What this verb does and what it refuses
//!
//! The mechanical rewrites of `doc/pdf-a-conversion-limits.md` section 4.7, the two section 4
//! *defaults* without which almost no real document can be made to conform at all — **the output
//! intent** (section 4.1) and **the identification schema** (section 4.2) — and, for the four
//! targets past PDF/A-2b, everything each of them adds that the file itself already evidences:
//!
//! - **Level U** (section 4.3): a `/ToUnicode` `CMap` derived from a font's own encoding, by
//!   ISO 32000-2 §9.10.2's second method. [`to_unicode`].
//! - **Level A** (section 5.1): `/MarkInfo` with `/Marked true`, where the file already carries
//!   the structure tree that flag is a claim about. **The tree itself is never invented.**
//! - **PDF/A-4f and PDF/A-4e** (ISO 19005-4 section 6.9, Annexes A.2 and B.4): an embedded file's
//!   `/F` and `/UF`, each written from the other, and its `/AFRelationship` written as §7.11.3's
//!   Table 43 own default.
//!
//! **Everything else is refused by name**: fonts (section 4.9), the structure tree and the role
//! map (section 5.1), a `/ToUnicode` entry no code in the file evidences (section 4.3),
//! encryption (section 3.5), an embedded file that would itself have to be converted
//! (section 3.1), `/Info` reconciliation and the extension schemas a producer's private XMP
//! property needs (section 4.2's other halves). A document needing one of those is told which
//! requirement, at which clause, this converter cannot meet — and no file is written. A stub that
//! wrote one anyway would be producing a file wearing a conformance claim it had not earned,
//! which is the failure section 7 of that document exists to prevent.
//!
//! **And "cannot meet" is two different sentences**, which `decision::REFUSED_BY_NAME` exists
//! to keep apart: a slice this converter still owes, and an act it will never perform. A user
//! told the first comes back tomorrow for the same answer if it was really the second.
//!
//! # The report is an output, not a log
//!
//! `doc/adr/0927`: the owner's four permissions to write something a producer did not — `A18`,
//! `A21`, `A48`, `A50` — all carry one condition, that **what was written is reported**, named
//! per document rather than inferable from a diff. `A18` is the first of the four this verb
//! exercises, and its condition is why [`Decision::Stated`] exists as a variant of its own: an
//! output intent loses nothing and all the same changes what every device colour in the file
//! means to a conforming reader, so the sentence saying so travels with the decision rather than
//! being left to a caller to remember. [`Conversion`] is the report, and it reaches a caller
//! through [`crate::Report::archive`] whether or not a file was written.
//!
//! # Five files, and the seam each is on
//!
//! The three stages are the seam, because they are three different kinds of work over the same
//! document, and a file that held two of them could not be read without reading both:
//!
//! | file | one responsibility |
//! |---|---|
//! | [`decision`] | the decision table and the five answers a failed requirement can get |
//! | [`prepare`] | what a decision needs built from the document before it can be taken |
//! | [`rewrite`] | the rewrites themselves, one object at a time, and the serializer walk |
//! | [`to_unicode`] | the `/ToUnicode` `CMap` a font's own encoding derives, and what it cannot |
//! | [`report`] | what was decided and done, worded for a person and for `--json` |
//! | this file | the three stages in order, the plan they run from, and the output's version |
//!
//! # One cost, stated
//!
//! The output is assembled in memory before it reaches the sink, because stage 3 validates what
//! it wrote and a validator needs a document rather than a stream of bytes. So this verb's peak
//! is the whole output file, where every other writer in this crate hands the sink an `Arc` and
//! keeps nothing. That is the price of the re-validation, and the re-validation is what the
//! verdict rests on.

mod decision;
mod prepare;
mod report;
mod rewrite;
mod to_unicode;

use std::collections::BTreeSet;
use std::io::Write as _;

use pdf_archive::{Outcome, Target, Verdict};
use pdf_model::Pages;
use pdf_syntax::{Document, Version};

use crate::pattern::{Fill, Pattern};
use crate::{Declined, Origin, Output, Refusal, Report, Sinks};

pub use decision::{Authorisations, Because, Decision, Loss, answered, refused_by_name};
pub use prepare::{DestinationProfile, ProfileSource};
pub use report::{Achieved, Conversion, Decided, NotChecked};
pub use rewrite::Rewrite;

use decision::decide;
use prepare::{DEFAULT_CMYK_ACTION, DEFAULT_CMYK_PARAMETERS, Prepared};
use report::describe_decision;
use rewrite::convert;

/// zlib's effort for a stream this verb re-encodes.
///
/// The serializer's own default, measured in ADR 0842 over the pdf.js corpus: level 9 saves
/// 13.30% of the whole against level 6's 12.60%. A conversion is not on a latency path, so the
/// same choice is made here and there is no flag, because a caller cannot choose the filter
/// either — ISO 19005 chose it.
const COMPRESSION_LEVEL: u32 = 9;

/// One document converted to a stated part and level of ISO 19005.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchivePlan {
    /// Which source.
    pub source: usize,
    /// How the one output is named.
    pub names: Pattern,
    /// The part, level or flavour to convert to.
    ///
    /// All six of `pdf_archive`'s targets are selectable, and none is a default: section 9 of
    /// `doc/pdf-a-conversion-limits.md` is the case of a user whose deposit rule names one and
    /// for whom "use PDF/A-4f instead" is a restatement of their problem rather than an answer.
    pub target: Target,
    /// What the caller has authorised this conversion to lose.
    pub authorised: Authorisations,
    /// The ICC profile an output intent this conversion adds states as its destination profile.
    ///
    /// `None` is the shipped sRGB profile, which is `doc/questions/A18`'s answer and
    /// `prepare::SRGB`'s reason for existing. A caller supplies one — `--output-intent-profile` —
    /// for a document produced for a press, which is the case
    /// `doc/pdf-a-conversion-limits.md` section 10.1 says nobody but the document's owner can
    /// decide. A supplied profile's own `cprt` tag is named in the report, because a user
    /// embedding somebody else's profile is entitled to be told whose it is.
    pub profile: Option<std::sync::Arc<[u8]>>,
}

/// Converts one document to the target and writes it.
///
/// `at` is the document's position among the opened ones — one, for this verb.
///
/// # Errors
///
/// [`Refusal::NoSuchSource`], [`Refusal::Reconstructed`] where the source states its structure
/// only through §C.4's recovery, [`Refusal::Assembly`] where the document cannot be rewritten at
/// all, and [`Refusal::Sink`] where the output cannot be written. A document that cannot be
/// *converted* is not an error: it is [`Report::refused`] beside the conversion's own report,
/// which is where a caller reads which requirement stopped it.
pub(crate) fn run(
    plan: &ArchivePlan,
    at: usize,
    documents: &[Document],
    sinks: &dyn Sinks,
    report: &mut Report,
) -> Result<(), Refusal> {
    let document = documents.get(at).ok_or(Refusal::NoSuchSource {
        at: plan.source,
        count: documents.len(),
    })?;
    let root = crate::optimize::catalog_of(document)?;
    crate::optimize::refuse_a_document_only_recovery_reads(document, root)?;

    // Stage 1: the validator is the reading. Nothing below re-reads ISO 19005.
    let input = pdf_archive::check(document, plan.target);
    // Stage 2: one decision per failed requirement.
    let (mut conversion, version, prepared) = decide_every_failure(plan, document, &input);
    if !conversion.proceeds() {
        for decided in &conversion.decided {
            if decided.decision.proceeds() {
                continue;
            }
            report.refused.push(Declined {
                source: plan.source,
                page: None,
                subject: format!("{} ({})", decided.requirement, decided.citation),
                detail: describe_decision(decided, false),
            });
        }
        report.archive = Some(conversion);
        return Ok(());
    }
    // Stage 3: apply, hold the output to the same target, and write it only if it stands.
    let outcome = apply_the_decisions(
        plan,
        document,
        &input,
        &mut conversion,
        version,
        &prepared,
        sinks,
    );
    let written = match outcome {
        Ok(written) => written,
        Err(refusal) => {
            report.archive = Some(conversion);
            return Err(refusal);
        }
    };
    match written {
        Written::Refused(declined) => report.refused.push(declined),
        Written::File(output) => report.outputs.push(output),
    }
    report.archive = Some(conversion);
    Ok(())
}

/// Stage 2: what to do about every requirement the input failed, and the header's version.
///
/// A pure function of the validator's report, the document's own version and the caller's
/// authorisations. The version is answered here rather than in the rewrite because the
/// *feasibility* of the header rewrite depends on the document — [`version_for`] — and a
/// decision that cannot be carried out is a refusal rather than a plan.
fn decide_every_failure(
    plan: &ArchivePlan,
    document: &Document,
    input: &pdf_archive::Report,
) -> (Conversion, Option<Version>, Prepared) {
    let mut conversion = Conversion {
        source: plan.source,
        target: plan.target,
        conformed: input
            .judgements
            .iter()
            .filter(|judgement| judgement.outcome == Outcome::Met)
            .map(|judgement| judgement.id)
            .collect(),
        decided: Vec::new(),
        not_checked: input.unchecked().map(NotChecked::of).collect(),
        achieved: None,
        profile: None,
        recorded: None,
    };
    let prepared = Prepared::of(plan, document, input);
    let mut version = None;
    for judgement in input.failures() {
        let places = match &judgement.outcome {
            Outcome::Failed { total, .. } => *total,
            // `Report::failures` yields only `Outcome::Failed`, so this arm is unreachable; it
            // reports no places rather than panicking.
            _ => 0,
        };
        let mut decision = decide(judgement, plan.authorised, &prepared);
        if decision.rewrite() == Some(Rewrite::FileHeader) {
            match version_for(document, plan.target) {
                Ok(stated) => version = Some(stated),
                Err(because) => decision = Decision::Refused(because),
            }
        }
        conversion.decided.push(Decided {
            requirement: judgement.id,
            citation: judgement.citation.clone(),
            asks: judgement.asks,
            places,
            decision,
            changed: 0,
        });
    }
    (conversion, version, prepared)
}

/// What stage 3 produced: a file, or a refusal to write one.
enum Written {
    /// The converted document, written to the sink.
    File(Output),
    /// The conversion was carried out and its result would not stand. Nothing was written.
    Refused(Declined),
}

/// Stage 3: the rewrites, the output's own verdict, and the sink.
fn apply_the_decisions(
    plan: &ArchivePlan,
    document: &Document,
    input: &pdf_archive::Report,
    conversion: &mut Conversion,
    version: Option<Version>,
    prepared: &Prepared,
    sinks: &dyn Sinks,
) -> Result<Written, Refusal> {
    // Asked here for a document whose header already conformed, which is every document that
    // reaches this line without a `FileHeader` decision.
    let version = match version {
        Some(version) => version,
        None => version_for(document, plan.target).map_err(|because| {
            Refusal::Assembly(format!(
                "the header's version cannot be stated for {}: {}",
                plan.target,
                because.sentence()
            ))
        })?,
    };
    let wanted: BTreeSet<Rewrite> = conversion
        .decided
        .iter()
        .filter_map(|decided| decided.decision.rewrite())
        .collect();
    // The profile is reported wherever it is *used*, which is `doc/questions/A18`'s condition
    // and now two rewrites: the output intent names it as its destination profile, and the
    // `/DefaultCMYK` names the same object as its alternate space.
    if (wanted.contains(&Rewrite::OutputIntent) || wanted.contains(&Rewrite::DefaultCmyk))
        && let Ok(intent) = &prepared.intent
    {
        conversion.profile = Some(intent.reported.clone());
    }
    if wanted.contains(&Rewrite::DefaultCmyk) {
        conversion.recorded = Some(format!("{DEFAULT_CMYK_ACTION} — {DEFAULT_CMYK_PARAMETERS}"));
    }
    let converted = convert(document, plan.target, &wanted, version, prepared)?;
    for decided in &mut conversion.decided {
        if let Some(rewrite) = decided.decision.rewrite() {
            decided.changed = converted.applied.get(&rewrite).copied().unwrap_or(0);
        }
    }

    let achieved = hold_the_output_to_the_target(&converted.bytes, input, plan)?;
    let stands = achieved.conforms;
    let declined = declined_output(plan, &achieved);
    conversion.achieved = Some(achieved);
    if !stands {
        return Ok(Written::Refused(declined));
    }

    let expanded = plan.names.expand(&Fill {
        ordinal: 1,
        count: 1,
        page: None,
        label: None,
        title: None,
    });
    let mut writer = sinks.open(&expanded.name).map_err(|error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    })?;
    let sank = |error| Refusal::Sink {
        name: expanded.name.clone(),
        error,
    };
    writer.write_all(&converted.bytes).map_err(sank)?;
    writer.flush().map_err(sank)?;
    drop(writer);

    Ok(Written::File(Output {
        name: expanded.name,
        bytes: u64::try_from(converted.bytes.len()).unwrap_or(u64::MAX),
        sanitised: expanded.sanitised,
        origin: Origin::Archived {
            source: plan.source,
            target: plan.target.to_string(),
            pages: Pages::new(document).len(),
            changed: conversion
                .decided
                .iter()
                .filter(|decided| decided.decision.rewrite().is_some())
                .count(),
        },
    }))
}

/// Why an assembled output was not written.
///
/// **The rule is that a file leaves this verb only if it conforms**, and it is stronger than
/// checking that nothing regressed: a conversion whose output still fails a requirement has not
/// converted the document, and writing it would put a file into an archive under a claim
/// nothing had established. A *regression* — a requirement the source met and the output does
/// not — is named separately inside that, because it is the worse of the two failures and a
/// reader should not have to diff two lists to find it.
fn declined_output(plan: &ArchivePlan, achieved: &Achieved) -> Declined {
    use std::fmt::Write as _;

    let mut detail = format!(
        "the output would still fail {} requirement(s): {}. No file is written, because a \
         conversion whose result is not {} has not converted the document",
        achieved.still_failing.len(),
        achieved.still_failing.join(", "),
        plan.target,
    );
    if !achieved.regressions.is_empty() {
        let _ = write!(
            detail,
            ". {} of them the source met, which is worse: {}",
            achieved.regressions.len(),
            achieved.regressions.join(", ")
        );
    }
    Declined {
        source: plan.source,
        page: None,
        subject: format!("the converted file would not be {}", plan.target),
        detail,
    }
}

/// Holds the bytes just written to the same target, and says how they differ from the input.
fn hold_the_output_to_the_target(
    bytes: &[u8],
    input: &pdf_archive::Report,
    plan: &ArchivePlan,
) -> Result<Achieved, Refusal> {
    let output = Document::open(bytes.to_vec()).map_err(|error| {
        Refusal::Assembly(format!("the converted document does not re-open: {error}"))
    })?;
    let held = pdf_archive::check(&output, plan.target);
    let met: BTreeSet<&'static str> = input
        .judgements
        .iter()
        .filter(|judgement| judgement.outcome == Outcome::Met)
        .map(|judgement| judgement.id)
        .collect();
    let still_failing: Vec<&'static str> = held.failures().map(|judgement| judgement.id).collect();
    let regressions = still_failing
        .iter()
        .filter(|id| met.contains(*id))
        .copied()
        .collect();
    Ok(Achieved {
        conforms: held.verdict() == Verdict::Conforms,
        still_failing,
        regressions,
        checked: held.checked(),
    })
}

/// The version the output's header states, or why it cannot be stated.
///
/// **Raising a version and lowering one are not the same act**, and this is where that is
/// decided rather than in the rewrite:
///
/// - The source's major already matches the part's: the minor is kept where the part admits it,
///   and clamped to the part's own floor where it does not. Nothing is asserted that the source
///   did not.
/// - **Raising** (a PDF 1.x source to a part-4 target): allowed. Every construct ISO 32000-1
///   defines, ISO 32000-2 still defines; what changes is that some are deprecated, and ISO
///   19005-4 section 5.1's prohibition on deprecated features is a requirement the validator
///   reports as not checked either way, so the raise asserts nothing the report conceals.
/// - **Lowering** (a PDF 2.x source to a part-2 target): refused. section 6 of
///   `doc/pdf-a-conversion-limits.md` states the cost — every PDF 2.0-only construct is
///   translated or refused — and this converter translates none of them, so a `%PDF-1.7` header
///   over 2.0 constructs would be a file whose own header disowned its contents.
/// - A source whose header states no version this tree can read: refused, because choosing one
///   would be inventing what the producer wrote.
fn version_for(document: &Document, target: Target) -> Result<Version, Because> {
    // ISO 19005-2 section 6.1.2 admits 1.0 to 1.7; ISO 19005-4 section 6.1.2 admits 2.0 to 2.9.
    let (major, top) = match target.part() {
        pdf_archive::Part::Two => (1, 7),
        pdf_archive::Part::Four => (2, 9),
    };
    let Some(stated) = document.version() else {
        return Err(Because::NotBuiltYet(
            "this document's header states no version this tree reads, and choosing one for it \
             would be inventing what its producer wrote",
        ));
    };
    if stated.major == major {
        return Ok(Version {
            major,
            minor: stated.minor.min(top),
        });
    }
    if stated.major < major {
        return Ok(Version { major, minor: 0 });
    }
    Err(Because::NotThisTarget(
        "this document is PDF 2.0 or later and the target's part requires a %PDF-1.n header, so \
         every construct PDF 2.0 added would have to be translated or refused one at a time — \
         doc/pdf-a-conversion-limits.md section 6 — and this converter translates none of them",
    ))
}
