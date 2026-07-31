//! The conformance gate: citations checked against the standard, and the ledger kept honest.
//!
//! Runs with the rest of the workspace's tests and needs nothing but the tree — the
//! standard's conversion in `doc/md/` is committed, unlike the pdf.js corpus, so none of
//! these has a skip path.
//!
//! Three of the four failures below have been seen for real, which is why they are here:
//! `§8.9.6.5` and `§11.4.5.6` were cited for a year and name nothing; three of five sampled
//! quotations were paraphrases wearing quotation marks; and 146 citations existed beside no
//! record of what any of those clauses required.

#![expect(
    clippy::print_stdout,
    reason = "the gate reports the coverage summary it ratchets, which is the point of \
              running it"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use conformance::clause::ClauseIndex;
use conformance::ledger::{self, Ledger, Status};
use conformance::quote;

/// How many subclauses nobody has read against this code.
///
/// **This number may only fall.** It is the project's honest count of unasked questions, and
/// it starts at nearly all of them: the ledger was generated in the ninth session with every
/// row `unreviewed` except clause 13's, which principle 5 excludes by name, and the clauses
/// the source already cited, which were filled in the same session.
///
/// Lowering it means reading a clause family against the code and recording what is there —
/// 20 to 60 minutes for a family, and the only instrument in this tree that can find a
/// requirement nobody has thought about. Neither gate that renders a page can: a corpus
/// ranks what documents ask for, and a demand curve cannot rank a requirement no file
/// exercises.
const UNREVIEWED_CEILING: usize = 314;

/// How many `implemented` rows name a whole test *file* rather than a test.
///
/// **This number may only fall**, and it exists because of two rows found wrong in two
/// sessions in exactly the same way. §8.7.3.1's said "`/BBox` clips the cell" and §8.7.2's said
/// the pattern base "is what `base` holds while a form is being run"; both were written from
/// the clause, both were true of no code, and neither could have been caught here — a row that
/// names `tests/tiling.rs` names a file that passes whatever it contains.
///
/// A row naming `file.rs::a_test` is a claim something would fail if it stopped being true. A
/// row naming `file.rs` is a claim nothing checks. The gate cannot tell whether a named test
/// *covers* the clause, so this is a count rather than a rule; what it does is keep the
/// population where a false claim can hide from growing, and say how large it is.
const FILE_ONLY_EVIDENCE_CEILING: usize = 58;

/// Clauses this tree cites while their rows still say nobody has read them.
///
/// **The rule is that a clause the code cites may not be `unreviewed`** — code that names a
/// clause has read it closely enough to name it, and leaving the row empty is how 146
/// citations came to exist beside no record of what any of them required. This list is the
/// debt that rule found on its first run, and it is a ratchet in both directions: a clause
/// not on it fails the gate immediately, and a clause on it that *has* been reviewed must be
/// deleted from it.
///
/// It is a list rather than a count because the alternative was worse. Making the first run
/// green would have meant filling thirty-six rows in one sitting, and a row filled to make a
/// gate pass is exactly the rubber stamp the ledger exists to prevent — this project's own
/// habit says a clause-family review costs 20 to 60 minutes and produces findings, which is
/// not something to do thirty-six of against the clock.
/// Empty since the thirty-fourth session, and worth keeping as a ratchet rather than
/// deleting: a clause the code cites and nobody has read is the cheapest debt this project
/// can accrue, and an empty list fails loudly the moment one appears.
const REVIEW_OWED: &[&str] = &[];

/// Every `§` in the tree names a clause ISO 32000-2 has.
///
/// The cheapest check here and the one with the best record: two of the thirty-six distinct
/// clause numbers this tree cited were wrong, and both had been copied into further comments
/// and into two written reports before anything looked.
#[test]
fn every_citation_names_a_clause_that_exists() {
    let root = conformance::workspace_root();
    let index = ClauseIndex::read(&root.join(conformance::STANDARD)).expect("the standard");
    let scanned = conformance::scan_tree(&root).expect("the tree's sources");

    let mut wrong = String::new();
    let mut citations = 0usize;
    for (path, scan) in &scanned {
        for citation in &scan.citations {
            citations = citations.saturating_add(1);
            if !index.contains(&citation.number) {
                let _ = writeln!(
                    wrong,
                    "{}:{}: §{} is not a clause of ISO 32000-2",
                    path.display(),
                    citation.line,
                    citation.number
                );
            }
        }
        for malformed in &scan.malformed {
            let _ = writeln!(
                wrong,
                "{}:{}: `§{}` is not a citation this checker can read",
                path.display(),
                malformed.line,
                malformed.text
            );
        }
    }

    assert!(
        citations > 100,
        "only {citations} citations found: the scan is not reaching the tree"
    );
    assert!(wrong.is_empty(), "\n{wrong}");
    println!("{citations} citations, all naming clauses the standard has");
}

/// Every rustdoc blockquote is the standard's own words, within the clause it cites.
///
/// `CLAUDE.md` principle 5: quotation marks mean verbatim, and a load-bearing normative
/// sentence goes in as a blockquote under its clause number so that this test can find it.
/// Paraphrase is fine and often clearer — paraphrase that claims to be a quote is not.
///
/// Compared with layout, Markdown markup and typography normalised away, and with `…`
/// allowed to elide, so what is left is the words. If a quotation fails and the words look
/// right, check `doc/`'s PDF before editing the comment: `doc/md/` is a *conversion*, and it
/// splits words across page breaks in a handful of places.
#[test]
fn every_quotation_is_the_standards_own_words() {
    let root = conformance::workspace_root();
    let index = ClauseIndex::read(&root.join(conformance::STANDARD)).expect("the standard");
    let scanned = conformance::scan_tree(&root).expect("the tree's sources");

    let mut wrong = String::new();
    let mut quotations = 0usize;
    for (path, scan) in &scanned {
        for quotation in &scan.quotations {
            quotations = quotations.saturating_add(1);
            let site = format!("{}:{}", path.display(), quotation.line);
            let Some(clause) = &quotation.clause else {
                let _ = writeln!(
                    wrong,
                    "{site}: a quotation with no clause cited before it in the same comment. \
                     An unattributed quotation is one nothing can check."
                );
                continue;
            };
            if !index.contains(clause) {
                continue; // Already a failure of the citation test; not worth saying twice.
            }
            if !index.holds_quotation(clause, &quotation.text) {
                let _ = writeln!(
                    wrong,
                    "{site}: §{clause} does not contain\n    {}\n  as written. If it is a \
                     paraphrase, drop the blockquote and say it in prose.",
                    quote::normalise(&quotation.text)
                );
            }
        }
    }

    assert!(wrong.is_empty(), "\n{wrong}");
    println!("{quotations} quotations, all verbatim within the clause they cite");
}

/// Every `Table N` a comment names is a table the standard has, and its title is printed.
///
/// The clause half of a citation has been checked since the ninth session; the table half
/// was not, and one was already wrong. Four comments, two tests and a written report said
/// "§9.3.6 Table 106" for the text rendering modes, which are Table 104 — Table 106 is the
/// text-*positioning* operators, two subclauses away. The clause existed, the table existed,
/// and the pair was wrong.
///
/// # The stronger rule was tried and does not hold
///
/// The obvious check is that the clause cited beside a table reference must be one the
/// standard discusses that table in. It fails 14 of this tree's references and **all
/// fourteen are correct writing**: a comment about §9.3.5 that says "Table 103 names exactly
/// four operators" is naming Table 103 truthfully, and Table 103 belongs to §9.3.1. A
/// comment routinely cites a clause and names a table from another, because a sentence about
/// one thing often needs the other. A gate with fourteen exceptions out of twenty-five is
/// not a gate.
///
/// So the assertion is the weaker true one — the number names a table that exists — and the
/// *title* is printed for every distinct table the tree cites. That listing is what would
/// have caught the original error in one glance: "Table 106 — Text-positioning operators"
/// beside a file that is entirely about rendering modes. A checker cannot read a comment's
/// intent; a person reading eleven lines can.
#[test]
fn every_table_reference_names_a_table_the_standard_has() {
    let root = conformance::workspace_root();
    let index = ClauseIndex::read(&root.join(conformance::STANDARD)).expect("the standard");
    let scanned = conformance::scan_tree(&root).expect("the tree's sources");

    let mut wrong = String::new();
    let mut cited: BTreeMap<u16, BTreeSet<String>> = BTreeMap::new();
    for (path, scan) in &scanned {
        for reference in &scan.tables {
            let site = format!("{}:{}", path.display(), reference.line);
            match index.table_title(reference.table) {
                Some(title) => {
                    cited
                        .entry(reference.table)
                        .or_default()
                        .insert(title.to_owned());
                }
                None => {
                    let _ = writeln!(
                        wrong,
                        "{site}: ISO 32000-2 has no Table {}",
                        reference.table
                    );
                }
            }
        }
    }

    assert!(wrong.is_empty(), "\n{wrong}");
    println!("{} distinct tables cited by this tree:", cited.len());
    for (number, titles) in &cited {
        for title in titles {
            println!("  Table {number} — {title}");
        }
    }
}

/// The ledger's rows are the standard's subclauses, and every claim in it is well formed.
///
/// What this establishes is that a claim is checkable: the clause exists, the row is where
/// it belongs, the code and test it names are on disk, an `out-of-scope` row names one of
/// principle 5's closed exclusions, and no clause the code cites is still `unreviewed`.
/// Whether the code implements the clause is not checkable and is not attempted.
#[test]
fn the_ledger_agrees_with_the_standard_and_with_the_tree() {
    let root = conformance::workspace_root();
    let index = ClauseIndex::read(&root.join(conformance::STANDARD)).expect("the standard");
    let scanned = conformance::scan_tree(&root).expect("the tree's sources");
    let ledger = Ledger::read(&root.join(conformance::LEDGER)).expect("the ledger");

    let problems = ledger::check(&ledger, &index, &conformance::citations(&scanned), &root);
    let mut report = String::new();
    let mut owed_and_still_owed = Vec::new();
    for problem in &problems {
        if let ledger::Problem::CitedButUnreviewed { clause, .. } = problem
            && REVIEW_OWED.contains(&clause.to_string().as_str())
        {
            owed_and_still_owed.push(clause.to_string());
            continue;
        }
        let _ = writeln!(report, "  {problem}");
    }
    assert!(problems.is_empty() || report.is_empty(), "\n{report}");

    // The other direction, which is what makes it a ratchet rather than a list: a clause
    // that has been reviewed must leave, or the list stops meaning "owed".
    let settled: Vec<&&str> = REVIEW_OWED
        .iter()
        .filter(|clause| !owed_and_still_owed.iter().any(|owed| owed == *clause))
        .collect();
    assert!(
        settled.is_empty(),
        "REVIEW_OWED names {settled:?}, which no longer need reviewing. Delete those lines."
    );
    println!("{} cited clauses still owe a review", REVIEW_OWED.len());

    let counts = ledger.counts();
    let unreviewed = counts
        .iter()
        .find(|(status, _)| *status == Status::Unreviewed)
        .map_or(0, |(_, count)| *count);

    println!("conformance ledger: {} subclauses", ledger.rows.len());
    for (status, count) in &counts {
        if *count > 0 {
            println!("  {status:<13} {count}");
        }
    }
    println!("{}", by_clause(&ledger));

    // Evidence that is a whole file cannot fail when a claim stops being true; see
    // `FILE_ONLY_EVIDENCE_CEILING` for the two rows that taught this.
    let file_only: Vec<String> = ledger
        .rows
        .iter()
        .filter(|row| row.status == Status::Implemented)
        .filter(|row| !row.test.iter().any(|test| test.contains("::")))
        .map(|row| row.clause.to_string())
        .collect();
    println!(
        "  {} of the implemented rows name a test file rather than a test",
        file_only.len()
    );
    assert!(
        file_only.len() <= FILE_ONLY_EVIDENCE_CEILING,
        "{} implemented rows name a whole file as their evidence, above the ratchet of \
         {FILE_ONLY_EVIDENCE_CEILING}: {file_only:?}. A row that names a file names something \
         that passes whatever it contains. This number may only fall.",
        file_only.len()
    );

    assert!(
        unreviewed <= UNREVIEWED_CEILING,
        "{unreviewed} subclauses are unreviewed, above the ratchet of {UNREVIEWED_CEILING}. \
         This number may only fall."
    );
    if unreviewed < UNREVIEWED_CEILING {
        println!("UNREVIEWED_CEILING can come down from {UNREVIEWED_CEILING} to {unreviewed}.");
    }
}

/// The per-clause breakdown, which is what says where the debt is rather than how much.
fn by_clause(ledger: &Ledger) -> String {
    let mut out = String::from("  clause  rows  unreviewed\n");
    for clause in ledger::TECHNICAL_CLAUSES {
        let rows: Vec<_> = ledger
            .rows
            .iter()
            .filter(|row| row.clause.clause() == clause)
            .collect();
        let unreviewed = rows
            .iter()
            .filter(|row| row.status == Status::Unreviewed)
            .count();
        let _ = writeln!(out, "  {clause:>6}  {:>4}  {unreviewed:>10}", rows.len());
    }
    out
}
