//! `archive` over the veraPDF corpus: two properties, over a population of witnesses.
//!
//! `crates/pdf-transform/tests/archive.rs` states what the converter *does*, from a fixture built
//! out of the clauses. This states what it may never do, over a few hundred documents nobody here
//! wrote:
//!
//! 1. **A document the validator already finds conforming converts, and the output still
//!    conforms.** The conversion has nothing to decide, so anything it changed was a change no
//!    failed requirement asked for — which is the rule the whole verb rests on.
//! 2. **Nothing panics, and nothing comes back as an error.** A conversion either writes a file or
//!    refuses by name; `crate::Refusal` is for a source that could not be read at all, and a
//!    corpus document that reaches one is a bug in this verb rather than a fact about the file.
//!
//! **The corpus is a population, never an answer key** — `crates/pdf-archive/tests/corpus.rs` has
//! that argument in full, and `CLAUDE.md` principle 5 is where it comes from. Nothing here treats
//! veraPDF's own verdict as the expected value: the expected value is *this tree's* validator run
//! twice, before and after, which is a statement about the converter and not about the standard.
//!
//! ```sh
//! cargo test --profile gates -p pdf-transform --test archive_corpus -- --ignored --nocapture
//! ```
//!
//! `doc/veraPDF-corpus` is 239 MB and not part of a checkout, so this is `#[ignore]`d and says so
//! when the submodule is absent.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "test code whose entire product is a report a person reads, and whose two \
              properties must fail loudly when a corpus document breaks one"
)]

use std::path::{Path, PathBuf};

use pdf_archive::{Flavour, Level, Target, Verdict};
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{ArchivePlan, Authorisations};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// The corpus root, or `None` where the submodule is not checked out.
fn corpus() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/veraPDF-corpus");
    root.is_dir().then_some(root)
}

/// Every PDF under one of the corpus's directories, in a stable order.
fn documents(root: &Path, part: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(&root.join(part), &mut out);
    out.sort();
    out
}

/// One directory, recursively.
fn collect(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            out.push(path);
        }
    }
}

/// What one target's sweep found.
#[derive(Debug, Default)]
struct Tally {
    /// Documents the validator found conforming before the conversion.
    conforming: usize,
    /// Of those, how many converted into a file that still conforms.
    stayed: usize,
    /// Documents the validator found failing that this verb nonetheless converted.
    converted: usize,
    /// Documents refused by name.
    refused: usize,
    /// Documents whose bytes this tree does not open at all, which is not this verb's business.
    unreadable: usize,
}

/// Converts every corpus document under `part` to `target` and states the two properties.
fn sweep(root: &Path, part: &str, target: Target) -> Tally {
    let mut tally = Tally::default();
    for path in documents(root, part) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open_with_limits(bytes.clone(), Limits::DEFAULT) else {
            tally.unreadable = tally.unreadable.saturating_add(1);
            continue;
        };
        let conformed = pdf_archive::check(&document, target).verdict() == Verdict::Conforms;
        drop(document);

        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Archive(ArchivePlan {
                source: 0,
                names: "out.pdf".parse().expect("a pattern"),
                target,
                authorised: Authorisations::default(),
            }),
            &[Source::new(bytes)],
            &sinks,
            &Policy::default(),
            &Budget::default(),
        );
        // Property 2: a conversion refuses by name or writes a file. An `Err` here is this verb
        // failing to read a document the line above read.
        let report = report.unwrap_or_else(|refusal| {
            panic!(
                "{}: the conversion errored rather than refusing: {refusal}",
                path.display()
            )
        });
        let output = sinks.into_outputs().pop().map(|(_, bytes)| bytes);

        match (conformed, output) {
            // Property 1, both halves: nothing was decided, and what came out still conforms.
            (true, Some(output)) => {
                tally.conforming = tally.conforming.saturating_add(1);
                let conversion = report.archive.as_ref().expect("a conversion is reported");
                assert!(
                    conversion.decided.is_empty(),
                    "{}: already conforming, and the converter decided {:?}",
                    path.display(),
                    conversion.decided
                );
                let held = Document::open_with_limits(output, Limits::DEFAULT)
                    .expect("the converted document re-opens");
                assert_eq!(
                    pdf_archive::check(&held, target).verdict(),
                    Verdict::Conforms,
                    "{}: conformed before the conversion and not after",
                    path.display()
                );
                tally.stayed = tally.stayed.saturating_add(1);
            }
            (true, None) => panic!(
                "{}: conformed already and no file was written",
                path.display()
            ),
            (false, Some(_)) => tally.converted = tally.converted.saturating_add(1),
            (false, None) => tally.refused = tally.refused.saturating_add(1),
        }
    }
    tally
}

#[test]
#[ignore = "needs doc/veraPDF-corpus, which is 239 MB and not part of a checkout"]
fn a_conforming_document_stays_conforming_and_nothing_errors() {
    let Some(root) = corpus() else {
        println!("doc/veraPDF-corpus is not here; nothing to sweep");
        return;
    };
    for (part, target) in [
        ("PDF_A-4", Target::Four(Flavour::Plain)),
        ("PDF_A-2b", Target::Two(Level::B)),
    ] {
        let tally = sweep(&root, part, target);
        println!(
            "archive {target}: {} conforming, {} of them still conforming after the conversion; \
             {} failing documents converted, {} refused by name, {} unreadable",
            tally.conforming, tally.stayed, tally.converted, tally.refused, tally.unreadable
        );
        assert_eq!(
            tally.conforming, tally.stayed,
            "every document that conformed before the conversion conforms after it"
        );
        assert!(
            tally.converted > 0,
            "the corpus holds documents this slice's rewrites fix, and none was fixed"
        );
    }
}
