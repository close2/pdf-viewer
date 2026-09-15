//! The `§` rule, held over the documents this project writes about itself.
//!
//! `doc/pdf-a-mitigations.md` states the rule in one sentence — a `§` is a clause of ISO 32000-2
//! and nothing else, and another standard's section is written out in words — and until the
//! one-thousand-and-ninety-sixth session it was held over Rust sources and the ledger's notes
//! and nowhere else. The prose those are written beside is where this project reasons about
//! *other* standards most: ISO 19005's parts, ISO 14289, the ETSI profiles, the ITU-T
//! Recommendation behind DER. `conformance::documents` says which documents and why not the
//! rest.
//!
//! **Why a wrong sign is worse here than a wrong number.** A `§6.3.3` written after "ISO 19005-2"
//! is checked against ISO 32000-2's clause list by every instrument this project has, and ISO
//! 32000-2 *has* a §6.3.3 — so it resolves, the gate passes, and a round following the citation
//! reads the conformance clause of the wrong standard. The number being valid is what makes it
//! invisible.

#![expect(
    clippy::print_stdout,
    reason = "the gate prints the population it counted, which is what makes its verdict readable"
)]

use std::fmt::Write as _;

/// Every `§` in an instruction document belongs to ISO 32000-2.
#[test]
fn every_section_sign_in_an_instruction_document_is_iso_32000_2s() {
    let root = conformance::workspace_root();
    let scanned = conformance::documents::scan(&root).expect("the instruction documents");

    let mut wrong = String::new();
    let mut signs = 0usize;
    for (path, scan) in &scanned {
        signs = signs
            .saturating_add(scan.citations.len())
            .saturating_add(scan.sections.len())
            .saturating_add(scan.foreign.len());
        for foreign in &scan.foreign {
            let _ = writeln!(
                wrong,
                "{}:{}: a `\u{a7}` after {}, which is not ISO 32000-2. Write \"{} section N\": a \
                 `\u{a7}` here is checked against this standard's clauses and would pass by \
                 landing on one.",
                path.display(),
                foreign.line,
                foreign.document,
                foreign.document
            );
        }
    }

    assert!(
        scanned.len() > 100,
        "only {} documents scanned: the walk is not reaching the population",
        scanned.len()
    );
    assert!(wrong.is_empty(), "\n{wrong}");
    println!(
        "{signs} `\u{a7}` in {} instruction documents, every one of them ISO 32000-2's",
        scanned.len()
    );
}
