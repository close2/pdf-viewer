//! The veraPDF corpus, read as a population of witnesses rather than as an answer key.
//!
//! # What the corpus is, and the one thing it is not
//!
//! `doc/veraPDF-corpus` is a few thousand **atomic** documents, each built to exercise one
//! clause: the directory names the clause (`PDF_A-4/6.9 Embedded files`) and the file name
//! carries the test number and the verdict its author intended
//! (`…6-9-t02-fail-a.pdf`, `…6-9-t03-pass-a.pdf`). It is CC BY 4.0, which is what makes it
//! usable here at all; `doc/third-party-data.md` records the reading.
//!
//! **The intended verdict is somebody's reading of ISO 19005, and `CLAUDE.md` principle 5 is
//! explicit about what that is worth.** Agreement raises confidence that this crate read the
//! clause correctly. Disagreement is a question to take back to `doc/pdfa/`, and it has three
//! possible answers — this crate is wrong, the corpus is wrong, or the clause is genuinely
//! ambiguous — which have three different consequences. So this file **reports** and does not
//! gate: it prints where the two readings differ, by clause, and a round adjudicates each one
//! against the standard. Turning an adjudicated expectation into a gate is a later step and a
//! deliberate one.
//!
//! Ignored by default, because the corpus is 239 MB that no checkout is required to have:
//!
//! ```sh
//! cargo test -p pdf-archive --test corpus -- --ignored --nocapture
//! ```

#![expect(
    clippy::print_stdout,
    reason = "test code whose entire product is a report a person reads"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_archive::{Flavour, Level, Outcome, Target, check};
use pdf_syntax::{Document, FileBytes};

/// A disagreement that has been read against the standard, and how it was ruled.
///
/// # The owner's rule, stated on 2026-09-07
///
/// "If something is ambiguous, we trust veraPDF. Not when it's in conflict — then the spec
/// wins, unless I say otherwise." The words are the owner's, and they are deliberately not a
/// rustdoc blockquote: `tools/conformance` reads one as a claim to be quoting ISO 32000-2, and
/// this is a statement about who decides rather than about what a clause says.
///
/// Two buckets, and **the order of the test is the whole discipline**: the clause is read
/// *first*, and only a clause that genuinely fails to decide reaches the second bucket. A
/// disagreement is never evidence of ambiguity by itself — that inference is the one
/// `CLAUDE.md` principle 5 forbids, because it turns every difference into a licence to copy.
///
/// The two rulings no adjudication has reached yet are kept rather than added when first
/// needed, because the *set* is the policy: a reader of this table has to be able to see that
/// "the standard decided against us" and "the standard did not decide" are available answers,
/// or the only ruling ever recorded will be the comfortable one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    dead_code,
    reason = "the unreached rulings are the policy, not spare code"
)]
enum Ruling {
    /// The clause decides, and it decides for this crate. The corpus is wrong.
    ///
    /// Recorded rather than silently tolerated: a disagreement nobody has read looks exactly
    /// like one somebody has, and the point of this table is to tell them apart.
    SpecAgainstTheCorpus,
    /// The clause decides, and it decides against this crate. Work owed, not a ruling.
    SpecAgainstUs,
    /// The clause genuinely does not decide, so veraPDF's reading is adopted as a **choice**.
    ///
    /// Never written without the sentence saying *what* is undecided. `CLAUDE.md` requires a
    /// silence in the standard to be argued rather than asserted, and warns that such a claim
    /// decays — so each of these is revisitable, and the reason is what a later round revisits.
    AmbiguousSoTheirs,
}

/// Every disagreement that has been adjudicated, so that a sweep can show only the new ones.
///
/// Keyed by the corpus file's own name, which is stable and which names the clause it is about.
static ADJUDICATED: &[(&str, Ruling, &str)] = &[
    (
        "veraPDF test suite 6-2-10-3-1-t01-pass-a.pdf",
        Ruling::SpecAgainstTheCorpus,
        "ISO 19005-4 section 6.2.10.3.1 requires the CIDFont's Supplement to be greater than or \
        equal \
         to the CMap's; this file states 1 against the CMap's 2 and is named a pass. Its \
         sibling `-t01-fail-c` states the *converse* and is named a fail, so the corpus has the \
         comparison inverted rather than merely disagreeing at a boundary. ISO 19005-2 \
         section 6.2.11.3.1's NOTE settles the direction by explaining the purpose: the \
         requirement \
         ensures the font has glyphs for all CIDs the CMap can reference, which is a floor on \
         the font and not a ceiling.",
    ),
    (
        "veraPDF test suite 6-2-10-3-1-t01-fail-c.pdf",
        Ruling::SpecAgainstTheCorpus,
        "the same inversion seen from the other side: the CIDFont's Supplement is greater than \
         the CMap's, which the clause permits in as many words, and the corpus expects a \
         failure.",
    ),
    // The same two files again under part 2, whose section 6.2.11.3.1 states the rule in the same
    // words. That the inversion appears in both halves of the corpus is what makes it a
    // property of the corpus rather than a slip in one file.
    (
        "veraPDF test suite 6-2-11-3-1-t01-pass-d.pdf",
        Ruling::SpecAgainstTheCorpus,
        "ISO 19005-2 section 6.2.11.3.1, the same inversion as `6-2-10-3-1-t01-pass-a`: the file's \
        own \
         outline says the CIDFont's Supplement is less than the CMap's, and the clause requires \
         it to be greater than or equal.",
    ),
    (
        "veraPDF test suite 6-2-11-3-1-t01-fail-c.pdf",
        Ruling::SpecAgainstTheCorpus,
        "ISO 19005-2 section 6.2.11.3.1, the converse: the CIDFont's Supplement is greater than \
        the \
         CMap's, which the clause permits, and the corpus expects a failure.",
    ),
    (
        "veraPDF test suite 6-2-5-t03-fail-b.pdf",
        Ruling::SpecAgainstTheCorpus,
        "ISO 19005-2 section 6.2.5 permits a halftone's `TransferFunction` only where the base \
        standard \
         requires it, and ISO 32000-2's Table 130 requires it of a component representing \
         'either a nonprimary or nonstandard primary colour component'. This file's Type 5 \
         halftone names `Red`, `Green` and `Blue` — which ISO 32000-2 §10.6.5.6 lists among the \
         'Primary colour components for the standard native device colour spaces … Red, Green, \
         and Blue for DeviceRGB'. So the entry is not required of them, and the corpus's own \
         outline calls them 'non-primary'. The standard names them primary in a list, which is \
         as decided as a clause gets.",
    ),
    // ISO 19005-2 section 6.6.2.3.3, two witnesses that were one argument and are now two.
    //
    // **`TN 0009` is settled, by a different document.** Session 940 ruled both files against the
    // corpus because the record said to be contrary — PDF Association TN 0009 section 4.3, on a
    // field that may be absent where a schema defines no custom types — returned HTTP 403 and
    // could not be read. `doc/TechNote0010.pdf` is now held, and its item A029 is that reading in
    // an authoritative form: the ISO working group resolved that parts 1 to 3 are read as if an
    // extension schema defining no custom value types may omit `pdfaSchema:valueType`, and a
    // value type defining no structured fields may omit `pdfaType:field`, a validator allowing
    // each absence and treating it as an empty array. `crate::clarification` carries it.
    //
    // **It settles one of the two files and not the other**, which is the clarification's own
    // asymmetry rather than a compromise. `t01-pass-e` omits `pdfaSchema:valueType` and its one
    // property uses the predefined `Boolean` type, so it is A029's first case exactly; it now
    // agrees and has left this table. `t05-pass-a` omits `pdfaSchema:property`, which A029 does
    // not mention in any of its four sentences — so section 6.6.2.3.2's own sentence stands for
    // that field and the file stays ruled against the corpus, below.
    //
    // **And one witness moved the other way**, which is recorded here because it does not appear
    // in this table: `6-6-2-3-3-t03-fail-b` omits `pdfaType:field` and is named a fail, and A029
    // permits that absence in as many words. It is now a `missed` rather than an agreement, and
    // that is the honest column for it — the file is non-conforming for a reason this crate does
    // not yet check, its own `fs:property1` carrying a `cvt:field1` that no value type describes,
    // which is section 6.6.2.3.1's question about a *custom* structure's fields. Catching it
    // again by keeping a rule A029 withdrew would be the right verdict for a reason the
    // committee has ruled out.
    //
    // **The row that remains is still the first to revisit** if the owner ever exercises the
    // fourth path `crate::errata` and ADR 0922 reserve to them — a conflict the standard decides,
    // where they rule that this crate follows veraPDF anyway. It is recorded as the standard
    // decides it, with the argument for the other answer written out so that it does not have to
    // be rediscovered.
    (
        "veraPDF test suite 6-6-2-3-3-t05-pass-a.pdf",
        Ruling::SpecAgainstTheCorpus,
        "ISO 19005-2 section 6.6.2.3.2's last sentence is literal: all fields described in each of \
         section 6.6.2.3.3's tables shall be present in any extension schema container schema, \
         and Table 3 lists `pdfaSchema:property` among them. This file omits it and is named a \
         pass. **TechNote 0010's A029 is now readable and does not reach this field** — it \
         grants `pdfaSchema:valueType` and `pdfaType:field` and no others — so the published \
         sentence still governs. Two further facts point the same way: **veraPDF's own \
         implementation disagrees with veraPDF's own corpus here**, `veraPDF-library` issue \
         #1257 being a user whose file was failed for exactly this missing field and closed with \
         the maintainer confirming the file was at fault; and a schema that describes no \
         properties at all describes nothing any property could use.",
    ),
    // ISO 19005-4's section 6.2.5 lost its transfer-function provision to erratum #314, and the
    // corpus predates the correction: three of its PDF/A-4 witnesses exercise a sentence part 4 no
    // longer states. `crate::errata` carries the erratum; this is what it does to a sweep.
    (
        "veraPDF test suite 6-2-5-t05-fail-a.pdf",
        Ruling::SpecAgainstTheCorpus,
        "a Type 5 halftone naming Red, Green and Blue whose components state no \
         TransferFunction, expected to fail ISO 19005-4 section 6.2.5. **Two independent reasons, \
         and \
         either alone settles it.** First, the provision is gone: PDF Association issue #314 is \
         ISO approved and its resolution is that the TransferFunction sentence is removed from \
         PDF/A-4 altogether, so no part 4 target is held to it — `crate::errata` withdraws the \
         row from part 4 and the report says so. Second, on the published sentence the entry \
         would not be required of these colourants anyway: ISO 32000-2 §10.6.5.6 lists Red, \
         Green and Blue among the 'Primary colour components for the standard native device \
         colour spaces', and §10.5 lists the same eight names, so Table 130's condition — a \
         component representing 'either a nonprimary or nonstandard primary colour component' — \
         does not hold. Part 4's own NOTE 1 narrows the primary set to the four process \
         colourants of DeviceCMYK, which is the shorter list this witness is built on; a note \
         is informative and the base standard's list is not. The \
         sibling `6-2-5-t03-fail-b` is the same disagreement seen under part 2, where the \
         provision does still stand.",
    ),
    (
        "veraPDF test suite 6-2-5-t05-fail-b.pdf",
        Ruling::SpecAgainstTheCorpus,
        "the converse construction — a Type 5 halftone naming Cyan, Magenta, Yellow and Black \
         with a TransferFunction present — and **this crate reads the sentence exactly as the \
         corpus does**: held to ISO 19005-2 section 6.2.5 the file is failed, under \
         `graphics/halftone-transfer-function-only-where-required`, at the Cyan component. It is \
         missed under PDF/A-4 for one reason only, which is that PDF Association issue #314 \
         removed the provision from part 4. So this row records an erratum rather than a \
         reading.",
    ),
    (
        "veraPDF test suite 6-2-5-t05-fail-c.pdf",
        Ruling::SpecAgainstTheCorpus,
        "a Type 1 halftone set as a graphics state's HT with a TransferFunction, which the base \
         standard requires only of a Type 5 halftone's component. The same erratum: held to \
         ISO 19005-2 section 6.2.5 this crate fails the file at object 12's HT entry, and PDF/A-4 \
         no \
         longer states the sentence that would fail it there.",
    ),
    (
        "veraPDF test suite 6-2-11-5-t01-fail-c.pdf",
        Ruling::SpecAgainstTheCorpus,
        "a Type 3 font whose `d0` disagrees with its `/Widths`, expected to fail ISO 19005-2 \
         section 6.2.11.5. **Part 2 states no Type 3 rule**: `d0` and `d1` appear nowhere in \
         ISO 19005-2, and ISO 19005-4 section 6.2.10.5 *added* the sentence. And section 6.2.11.5 \
         is about a \
         font embedded in a conforming file agreeing with the embedded font *program*, which \
         ISO 32000-2 §9.6.4 settles for this case: font dictionaries for other fonts \
         \"refer to a separate font program for the actual glyph descriptions; a Type 3 font \
         dictionary contains the glyph descriptions\". There is no font program for part 2's \
         sentence to be about, so the rule is part 4's alone — which is where this crate \
         implements it.",
    ),
    (
        "veraPDF test suite 6-2-11-7-2-t01-fail-f.pdf",
        Ruling::SpecAgainstTheCorpus,
        "a Type 0 font whose descendant CIDFont uses the Adobe-Japan1 character collection and \
         states no `ToUnicode`, expected to fail ISO 19005-2 section 6.2.11.7.2. That subclause's \
         third \
         exemption names the collection in as many words — Type 0 fonts whose descendant CIDFont \
         uses Adobe-GB1, Adobe-CNS1, Adobe-Japan1 or Adobe-Korea1 — so the font is exempt and \
         the file conforms. The witness's own outline states the exempt condition as its reason \
         for failing. Its siblings `-pass-h`, `-pass-i` and `-pass-j` are the same construction \
         over Korea1, GB1 and CNS1 and are expected to pass; there is no `-pass-g`, which is \
         what a file moved from the pass set to the fail set leaves behind.",
    ),
    // ISO 19005-2 section 6.6.6 and ISO 19005-4 section 6.7.5, five witnesses that were one
    // argument and are now two arguments with different answers.
    //
    // **veraPDF implements neither subclause**: `PDFA-2B.xml` and `PDFA-4.xml` carry no rule
    // whose clause is `6.6.6` or `6.7.5`, which is why every witness in both directories is named
    // a pass. Session 940 ruled all five against the corpus, on the reasoning that the resolution
    // said to justify them reached this tree only through a third party's test fixture's outline
    // and a conference summary that contradicted it — neither readable as a source of truth.
    //
    // **Part 2's three are no longer disagreements at all.** `doc/TechNote0010.pdf` is held, and
    // its item A021 is the resolution in an authoritative form: the ISO working group resolved
    // that ISO 19005-2 and ISO 19005-3 are read as if requirements on the `xmpMM:History`
    // property were requirements on the writing application, and therefore irrelevant to ISO
    // 19005 validation. `metadata/provenance-recorded-action-fields` is a
    // `Check::OutsideValidation` row on that record, so `6-6-6-t01-pass-b`, `-c` and `-d` — one
    // missing `action`, one missing `parameters`, one missing `when` — are now agreements and
    // have left this table. The corpus was right about all three, for a reason nobody here could
    // read until the document was obtained (`doc/adr/0931`).
    //
    // **Part 2's rule was the whole of the argument, and part 4's survives it.** A021's
    // `Pertaining` line names parts 2 and 3 and no other, and its closing note says only that the
    // proposal was accepted in principle for the part then being drafted. That part was published
    // in 2020 as ISO 19005-4, and its section 6.7.5 states `action` and `when` as requirements
    // anyway — so the note records an intention and the standard records a rule, and principle 5
    // reads the standard. The two witnesses below stay ruled against the corpus.
    (
        "veraPDF test suite 6-7-5-t01-pass-b.pdf",
        Ruling::SpecAgainstTheCorpus,
        "a recorded action whose time field is spelled with a capital W, named a pass under \
         ISO 19005-4 section 6.7.5. XML names are case-sensitive and the XMP Specification's \
         ResourceEvent value type spells the field in lower case, so the file states a field \
         that value type does not define and states none the subclause requires.",
    ),
    (
        "veraPDF test suite 6-7-5-t01-pass-c.pdf",
        Ruling::SpecAgainstTheCorpus,
        "a recorded action stating parameters and when and no action field, under the part that \
         requires action and when and only recommends parameters — so the one field it omits is \
         one of the two part 4 requires.",
    ),
    (
        "veraPDF test suite 6-6-4-t01-pass-b.pdf",
        Ruling::SpecAgainstTheCorpus,
        "an identification schema stating 2016 as its pdfaid:amd, named a pass. ISO 19005-2 \
         section 6.6.4 says an amendment identifier is the amendment number and the year separated \
         by \
         a colon, and this value is a year alone. **The file was built for a different rule**: \
         veraPDF's `PDFA-2B.xml` has exactly one rule about `amd`, that its namespace prefix be \
         `pdfaid`, and this document and its sibling `-pass-a` are the two witnesses for that \
         rule and its `corr` twin. `-pass-a` states `1:2007`, which is the form section 6.6.4 \
         describes; the value here was never the subject of the test.",
    ),
    (
        "veraPDF test suite 6-2-4-3-t02-fail-a.pdf",
        Ruling::SpecAgainstTheCorpus,
        "a `DeviceRGB` on a 3D stream's `ColorSpace` with no output intent, expected to fail \
         ISO 19005-4 section 6.2.4.3 under the engineering annex. **Annex B sends 3D artwork \
         colour \
         somewhere else.** section B.2.3 makes the `ColorSpace` key the space the artwork is \
         specified \
         in, defaults it to sRGB when absent, then says a conforming processor is not required \
         to colour manage 3D artwork at all — and that one which does shall follow section \
         6.2.4.2's \
         rules for ICCBased colours, with the profile based on that key or on sRGB. So the key \
         is the basis of a profile rather than a device-colour use on a page, section 6.2.4.2 \
         governs \
         rather than section 6.2.4.3, and the only `shall` in the subclause binds a processor. \
         This \
         crate reports section B.2.3 as a `Check::Processor` row, which is what it is.",
    ),
];

/// Whether a file's disagreement with this crate has already been read against the clause.
fn adjudicated(path: &Path) -> Option<(Ruling, &'static str)> {
    let name = path.file_name()?.to_str()?;
    ADJUDICATED
        .iter()
        .find(|(file, _, _)| *file == name)
        .map(|(_, ruling, why)| (*ruling, *why))
}

/// Where the corpus is, when somebody has fetched it.
fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/veraPDF-corpus");
    root.is_dir().then_some(root)
}

/// What a corpus file's own name says about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intended {
    /// The author built it to conform.
    Pass,
    /// The author built it to break the clause its directory names.
    Fail,
}

/// The clause a file exercises and the verdict its author intended, from its path.
///
/// The corpus states both in its own naming pattern, which its README describes as deliberate:
/// the directory names the clause and the file name carries `-pass-` or `-fail-`. Reading them
/// rather than maintaining a table beside them means a corpus update cannot silently desynchronise
/// from this test.
fn intent(path: &Path) -> Option<(String, Intended)> {
    let name = path.file_name()?.to_str()?;
    let intended = if name.contains("-fail-") {
        Intended::Fail
    } else if name.contains("-pass-") {
        Intended::Pass
    } else {
        return None;
    };
    let directory = path.parent()?.file_name()?.to_str()?;
    // `6.9 Embedded files` — the clause is the leading number.
    let clause = directory.split_whitespace().next()?.to_owned();
    clause
        .starts_with(|c: char| c.is_ascii_digit())
        .then_some((clause, intended))
}

/// Every `.pdf` under one directory, sorted, so a run is reproducible.
fn documents(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_owned()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// What this crate said about one corpus file, against what its name said.
#[derive(Debug, Default, Clone)]
struct Tally {
    /// Both readings agree.
    agreed: usize,
    /// The corpus expects a failure and this crate found none — a rule not yet implemented, or
    /// a reading of the clause that differs.
    missed: usize,
    /// The corpus expects conformance and this crate found a failure. **The serious column**: a
    /// false positive is a validator telling a user their conforming file does not conform.
    over: usize,
    /// The document could not be opened at all.
    unreadable: usize,
    /// The corpus expected a failure, this crate found one, but under a **different** clause.
    ///
    /// Not a disagreement, and counting it as one was a defect in this harness rather than in the
    /// crate: several of ISO 19005's clauses state their requirement by *delegating* to another —
    /// Section 6.2.2 says a content stream's rendering intent shall conform to section 6.2.6, and
    /// its resource rule carries the file-structure clauses' requirements about `Length` and about
    /// names — so a witness filed under the delegating clause is correctly failed under the
    /// delegated one. Adding a duplicate row under the referring clause would make a report state
    /// one fault twice.
    ///
    /// Kept as its own column rather than folded into `agreed`, because the two are different
    /// facts: this crate caught the document, and it caught it somewhere else.
    elsewhere: usize,
    /// Disagreements already read against the clause and ruled on.
    ///
    /// Subtracted from the two columns above rather than hidden: a sweep's value is that the
    /// *new* disagreements stand out, and one that re-reported every settled one would bury
    /// them.
    settled: usize,
    /// The names behind `missed`, so the column can be worked on rather than only counted.
    ///
    /// A number tells a reader how much is left and nothing about what; twenty-three names fit
    /// on a screen and each one is a document some author wrote to break a specific rule.
    missed_names: Vec<String>,
}

/// Whether a judgement's citation names this clause, and not one that merely starts with it.
///
/// `Clauses::citation` writes `ISO 19005-2 section 6.1.13`, so the match is on the text after
/// `section ` — and it has to end there. A bare `contains` would let section 6.1.1 match section
/// 6.1.13's
/// citation and score a witness against the wrong row; the corpus has both numbers, so this is
/// a real collision rather than a hypothetical one.
fn cites(citation: &str, clause: &str) -> bool {
    let needle = format!("section {clause}");
    citation.match_indices(&needle).any(|(at, _)| {
        let after = citation
            .get(at.saturating_add(needle.len())..)
            .unwrap_or_default();
        !after.starts_with(|c: char| c.is_ascii_digit() || c == '.')
    })
}

#[test]
fn a_citation_names_one_clause_and_not_the_one_it_starts_with() {
    let citation = "ISO 19005-2 section 6.1.13";
    assert!(cites(citation, "6.1.13"));
    // The collision this guards is in the corpus: both section 6.1.1 and section 6.1.13 have
    // witnesses, and a
    // `contains` would score every section 6.1.13 failure against section 6.1.1's row as well.
    assert!(!cites(citation, "6.1.1"));
    assert!(!cites(citation, "6.1"));
    assert!(cites("ISO 19005-4 section 6.9", "6.9"));
    assert!(!cites("ISO 19005-4 section 6.9", "6.10"));
}

/// Runs one target's corner of the corpus and tallies the two readings by clause.
fn sweep(root: &Path, folder: &str, target: Target) -> BTreeMap<String, Tally> {
    let mut by_clause: BTreeMap<String, Tally> = BTreeMap::new();
    for path in documents(&root.join(folder)) {
        let Some((clause, intended)) = intent(&path) else {
            continue;
        };
        let tally = by_clause.entry(clause.clone()).or_default();
        let Ok(bytes) = FileBytes::on_disk(&path) else {
            tally.unreadable = tally.unreadable.saturating_add(1);
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            tally.unreadable = tally.unreadable.saturating_add(1);
            continue;
        };
        let report = check(&document, target);
        // Only the clause this file is *about*: a corpus document built to break section 6.9 may
        // well break something else incidentally, and counting that against it would measure the
        // fixture rather than this crate.
        let failures = || {
            report
                .judgements
                .iter()
                .filter(|judgement| matches!(judgement.outcome, Outcome::Failed { .. }))
        };
        let touched = failures().any(|judgement| cites(&judgement.citation, &clause));
        let anywhere = failures().next().is_some();
        match (intended, touched) {
            (Intended::Fail, true) | (Intended::Pass, false) => {
                tally.agreed = tally.agreed.saturating_add(1);
            }
            _ if adjudicated(&path).is_some() => tally.settled = tally.settled.saturating_add(1),
            // The corpus wanted this document failed and it is failed, under a clause that
            // states the requirement rather than the one whose directory it sits in.
            (Intended::Fail, false) if anywhere => {
                tally.elsewhere = tally.elsewhere.saturating_add(1);
            }
            (Intended::Fail, false) => {
                tally.missed = tally.missed.saturating_add(1);
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    tally.missed_names.push(name.to_owned());
                }
            }
            (Intended::Pass, true) => tally.over = tally.over.saturating_add(1),
        }
    }
    by_clause
}

/// Prints one sweep, clause by clause.
fn report(name: &str, by_clause: &BTreeMap<String, Tally>) {
    let mut total = Tally::default();
    let mut missed: Vec<String> = Vec::new();
    println!("\n== {name} ==");
    println!("  clause  agreed  missed  over  settled  elsewhere  unreadable");
    for (clause, tally) in by_clause {
        println!(
            "  {clause:<7} {:>6} {:>7} {:>5} {:>8} {:>10} {:>11}",
            tally.agreed,
            tally.missed,
            tally.over,
            tally.settled,
            tally.elsewhere,
            tally.unreadable
        );
        total.agreed = total.agreed.saturating_add(tally.agreed);
        total.missed = total.missed.saturating_add(tally.missed);
        total.over = total.over.saturating_add(tally.over);
        total.settled = total.settled.saturating_add(tally.settled);
        for name in &tally.missed_names {
            missed.push(format!("{clause}  {name}"));
        }
        total.elsewhere = total.elsewhere.saturating_add(tally.elsewhere);
        total.unreadable = total.unreadable.saturating_add(tally.unreadable);
    }
    println!(
        "  {:<7} {:>6} {:>7} {:>5} {:>8} {:>10} {:>11}",
        "all",
        total.agreed,
        total.missed,
        total.over,
        total.settled,
        total.elsewhere,
        total.unreadable
    );
    println!(
        "  `missed`    a requirement not implemented, or a clause read differently\n  \
         `over`      this crate failing a document its author built to conform — the column\n              \
         that matters, and every one is a question for doc/pdfa/ before it is a bug\n  \
         `settled`   a disagreement already read against the clause and ruled on\n  \
         `elsewhere` caught, under the clause that states the rule rather than the one whose\n              \
         directory the witness sits in — a delegation, not a disagreement"
    );
    if !missed.is_empty() {
        println!("  the `missed` documents, by the clause their name claims:");
        for line in &missed {
            println!("    {line}");
        }
    }
}

#[test]
#[ignore = "needs doc/veraPDF-corpus, which is 239 MB and not part of a checkout"]
fn the_two_readings_of_iso_19005_compared_clause_by_clause() {
    let Some(root) = corpus() else {
        println!("doc/veraPDF-corpus is not here; nothing to compare");
        return;
    };
    for (folder, target) in [
        ("PDF_A-4", Target::Four(Flavour::Plain)),
        ("PDF_A-4f", Target::Four(Flavour::F)),
        ("PDF_A-4e", Target::Four(Flavour::E)),
        ("PDF_A-2b", Target::Two(Level::B)),
        ("PDF_A-2u", Target::Two(Level::U)),
        ("PDF_A-2a", Target::Two(Level::A)),
    ] {
        if !root.join(folder).is_dir() {
            continue;
        }
        report(folder, &sweep(&root, folder, target));
    }
}
