//! What it would cost to check this project's citations against the PDF instead of `doc/md/`.
//!
//! `cargo run --profile gates -p pdf-retrieve --example substitution_cost`
//!
//! `doc/todo/36`'s success condition is that `tools/conformance` stops needing the Markdown
//! conversion of the standard, and `doc/todo/48`'s item 5 is the migration behind it. Both were
//! written without a number. This measures one, by asking the *same two questions the gate asks*
//! of both substrates and printing where they disagree:
//!
//! 1. **Clause existence.** Every distinct clause this tree cites, against the conversion's 1034
//!    headings and against the 946 numbered items of §12.3.3's outline.
//! 2. **Verbatim quotation.** Every rustdoc blockquote, against the conversion's text for the
//!    clause it cites and against this reader's own extraction of that clause's pages.
//!
//! It is a measurement and not a gate: nothing this project generates may become what the gate
//! checks the standard against while the question of whether to switch is open (ADR 0252), and
//! the point of the number is to decide that question rather than to pre-empt it.
//!
//! **Its output names clauses and counts and prints no sentence of the standard**, so it may be
//! read in the open — which is deliberate, because the whole reason to run it is to write the
//! number down (ADR 0187).

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    reason = "a measurement binary whose output is its purpose"
)]
#![expect(
    clippy::too_many_lines,
    reason = "one measurement, run once, whose two halves are only meaningful side by side: \
              splitting it would hide that both questions are asked of one pair of substrates"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use conformance::clause::{ClauseIndex, ClauseNumber};
use pdf_retrieve::Retrieval;

fn main() {
    let root = conformance::workspace_root();
    let started = Instant::now();
    let markdown = ClauseIndex::read(&root.join(conformance::STANDARD))
        .expect("doc/md/ is unpacked; see doc/environment.md");
    println!(
        "doc/md/: {} headings in {:?}",
        markdown.headings().len(),
        started.elapsed()
    );

    let started = Instant::now();
    let retrieval = Retrieval::open(&root.join("doc/ISO_32000-2_sponsored_EC3.pdf"))
        .expect("the PDF is committed");
    // The PDF-side substrate for question 1: the outline's own numbers. Nothing is interpreted
    // to build it, which is why it is 40 ms against the conversion's 24 MB of Markdown.
    let numbered: BTreeMap<String, &str> = retrieval
        .sections()
        .iter()
        .filter_map(|section| {
            section
                .number
                .as_deref()
                .map(|number| (number.to_owned(), section.title.as_str()))
        })
        .collect();
    println!(
        "the PDF's outline: {} numbered items of {} in {:?}",
        numbered.len(),
        retrieval.sections().len(),
        started.elapsed()
    );

    let scanned = conformance::scan_tree(&root).expect("the source tree is readable");
    let citations = conformance::citations(&scanned);
    let distinct: BTreeSet<String> = citations
        .iter()
        .map(|(_, citation)| citation.number.to_string())
        .collect();
    let missing_from_outline: Vec<&String> = distinct
        .iter()
        .filter(|number| !numbered.contains_key(*number))
        .collect();
    let missing_from_markdown: Vec<&String> = distinct
        .iter()
        .filter(|number| {
            number
                .parse::<ClauseNumber>()
                .is_ok_and(|parsed| !markdown.contains(&parsed))
        })
        .collect();
    println!(
        "\n{} citations over {} distinct clauses",
        citations.len(),
        distinct.len()
    );
    println!("  {} not in doc/md/", missing_from_markdown.len());
    println!(
        "  {} not in the outline: {:?}",
        missing_from_outline.len(),
        missing_from_outline
    );

    // Question 2. Every page interpreted **once**, because a per-section retrieval re-reads the
    // pages its neighbours share and 988 sections over 1023 pages is several times the document.
    let started = Instant::now();
    let mut pages = Vec::with_capacity(retrieval.page_count());
    for index in 0..retrieval.page_count() {
        pages.push(
            retrieval
                .page(index, &pdf_retrieve::Wanted::default())
                .map(|read| read.text)
                .unwrap_or_default(),
        );
    }
    let extraction = started.elapsed();
    let bytes: usize = pages.iter().map(String::len).sum();
    println!(
        "\nextracting all {} pages: {:?}, {bytes} bytes",
        pages.len(),
        extraction
    );

    let quotations: Vec<(&std::path::PathBuf, &conformance::citation::Quotation)> = scanned
        .iter()
        .flat_map(|(path, scan)| {
            scan.quotations
                .iter()
                .map(move |quotation| (path, quotation))
        })
        .collect();
    let mut attributed = 0_usize;
    let mut in_markdown = 0_usize;
    let mut in_extraction = 0_usize;
    let mut in_extraction_squeezed = 0_usize;
    let mut in_extraction_dashed = 0_usize;
    let mut only_markdown = Vec::new();
    for (path, quotation) in &quotations {
        let Some(number) = quotation.clause.as_ref() else {
            continue;
        };
        attributed = attributed.saturating_add(1);
        let held = markdown.holds_quotation(number, &quotation.text);
        if held {
            in_markdown = in_markdown.saturating_add(1);
        }
        // The same question of the extraction: is the quotation inside the pages the outline
        // gives that clause? `ClauseIndex::holds_quotation` bounds its search by the heading's
        // line range and this bounds it by the section's pages, which is the same claim in the
        // substrate that has pages instead of lines.
        let Some(section) =
            pdf_model::retrieval::section(retrieval.sections(), &number.to_string())
        else {
            continue;
        };
        let text: String = pages
            .get(section.first_page..=section.last_page)
            .unwrap_or_default()
            .join("\n");
        let found = conformance::quote::occurs_in(&text, &quotation.text);
        if found {
            in_extraction = in_extraction.saturating_add(1);
        }
        if occurs_squeezed(&text, &quotation.text, false) {
            in_extraction_squeezed = in_extraction_squeezed.saturating_add(1);
        }
        if occurs_squeezed(&text, &quotation.text, true) {
            in_extraction_dashed = in_extraction_dashed.saturating_add(1);
        } else if held {
            only_markdown.push(format!("{}:{}", path.display(), quotation.line));
        }
    }
    println!(
        "\n{} blockquotes, {attributed} of them attributed to a clause",
        quotations.len()
    );
    println!("  {in_markdown} found in doc/md/, which is what the gate checks today");
    println!("  {in_extraction} found in this reader's extraction, comparing as the gate does");
    println!("  {in_extraction_squeezed} found with the spaces taken out (ADR 0253's comparison)");
    println!("  {in_extraction_dashed} found with the dashes folded together as well");
    println!(
        "  {} would have to be re-verified by hand: {:?}",
        only_markdown.len(),
        only_markdown
    );
}

/// [`conformance::quote::occurs_in`] with the spaces taken out of both sides.
///
/// The ellipsis handling is copied rather than shared because it is the *comparison* being
/// measured: `CLAUDE.md`'s convention for quoting part of a sentence is `…`, and a measurement
/// that compared whole quotations would price the ellipses as misses and overstate the cost.
fn occurs_squeezed(haystack: &str, quotation: &str, fold_dashes: bool) -> bool {
    let quotation = squeezed(quotation, fold_dashes);
    let haystack = squeezed(haystack, fold_dashes);
    let mut rest = haystack.as_str();
    for fragment in quotation
        .split(['\u{2026}'])
        .flat_map(|part| part.split("..."))
    {
        if fragment.is_empty() {
            continue;
        }
        let Some(position) = rest.find(fragment) else {
            return false;
        };
        rest = rest
            .get(position.saturating_add(fragment.len())..)
            .unwrap_or_default();
    }
    true
}

/// A passage with its spaces removed, which is how two extractions of one sentence are compared.
///
/// ADR 0253's finding, one substrate over: `doc/md/` and this tree are two programs extracting
/// the same glyphs, and PDF positions glyphs rather than words, so one writes `inthe` where the
/// other writes `in the`. A migration measured with the spaces kept would price work that is not
/// there.
fn squeezed(text: &str, fold_dashes: bool) -> String {
    conformance::quote::normalise(text)
        .chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| {
            // Every dash the two substrates spell one sentence with. The conversion writes
            // `Table 87 -Additional entries`, the PDF writes `Table 87 — Additional entries`,
            // and a quotation copied out of the conversion carries the conversion's dash — so
            // this measures how much of the gap is *typography* rather than words.
            if fold_dashes && matches!(character, '\u{2010}'..='\u{2015}' | '\u{2212}') {
                '-'
            } else {
                character
            }
        })
        .collect()
}
