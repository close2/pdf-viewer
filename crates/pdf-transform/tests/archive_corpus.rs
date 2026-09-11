//! `archive` over the veraPDF corpus: three properties, over a population of witnesses.
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
//! 3. **No glyph moves.** Every glyph advance the output's font dictionaries state is one the
//!    source stated. `doc/pdf-a-conversion-limits.md` section 4.9 offers three ways to make a
//!    font dictionary agree with its embedded program and calls the third — restating the
//!    dictionary — **never**, because ISO 32000-2 §9.2.4 makes those numbers what a reader
//!    positions glyphs by; a converter that took it would move every line of every page it
//!    touched and nothing in its report would say so. [`every_stated_width`] is the check, and
//!    it is absolute rather than sampled: the whole set of statements, before and after.
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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_archive::{Flavour, Level, Target, Verdict};
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{ArchivePlan, Authorisations, Decision, Loss};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// Every target, with the corpus directory whose documents were written for it.
///
/// **All six, and the four small ones are the point.** A target nothing exercises is a target
/// whose refusals nobody has seen: `PDF_A-4f` holds eleven documents and `PDF_A-2a` twenty-seven,
/// which is few enough to prove nothing statistically and enough to catch a panic, an `Err`, or a
/// conforming document this verb quietly breaks. The directory names are veraPDF's own.
const TARGETS: [(&str, Target); 6] = [
    ("PDF_A-2b", Target::Two(Level::B)),
    ("PDF_A-2u", Target::Two(Level::U)),
    ("PDF_A-2a", Target::Two(Level::A)),
    ("PDF_A-4", Target::Four(Flavour::Plain)),
    ("PDF_A-4f", Target::Four(Flavour::F)),
    ("PDF_A-4e", Target::Four(Flavour::E)),
];

/// How many of a target's refused requirements the sweep prints.
///
/// Enough to see the shape of what is left and short enough to read; the tail is a long list of
/// requirements one document each failed, which ranks nothing.
const MOST_REFUSED: usize = 10;

/// Every loss authorised, which is what makes one of this sweep's two runs a list of *gaps*.
///
/// A loss nobody authorised is a question put to a user rather than something the converter
/// cannot do, and a run that left them unauthorised would rank
/// `doc/pdf-a-conversion-limits.md`'s section 3 beside its section 5 — a document waiting for an
/// answer beside one no answer reaches.
///
/// **Both runs are reported, because the two numbers answer different questions and one of them
/// silently replacing the other would misstate the converter.** The authorised run says what this
/// converter can reach; the default run says what a person who types the command and answers
/// nothing actually gets, and that is the number a release note would have to carry. Session 952
/// moved this sweep from the default to the authorised run and the converted count went from 98
/// to 405 at PDF/A-2b — most of which is the *question* being answered rather than a gap being
/// closed.
fn authorise_everything() -> Authorisations {
    let mut authorised = Authorisations::default();
    for loss in Loss::ALL {
        authorised.authorise(loss);
    }
    authorised
}

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
    /// For each requirement that stopped a conversion, how many documents it stopped.
    ///
    /// **The next slice's work list, counted rather than guessed.** A refusal is by name, so the
    /// name is what says where the converter's remaining gaps are and how much each is worth;
    /// `CLAUDE.md`'s rule about derived facts is why this is printed by the sweep rather than
    /// written down anywhere.
    refusals: BTreeMap<&'static str, usize>,
}

/// Every glyph advance a document's font dictionaries state, in one comparable list.
///
/// ISO 32000-2 §9.6.2.1's `/Widths` for a simple font and §9.7.4.3's `/W` and `/DW` for a
/// `CIDFont` — plus §9.8.1's `/MissingWidth`, which is what a simple font's unlisted codes are —
/// gathered over every object the cross-reference table reaches. Sorted rather than keyed by
/// object, because what is being asserted is that the *set of statements* is the same and not
/// that the serializer chose the same numbering.
///
/// **Only a dictionary whose `/Type` is `Font` or `FontDescriptor` is looked at**, because `/W`
/// is not only a `CIDFont`'s: §7.5.8.2 gives a cross-reference stream one too, and the
/// serializer writes a cross-reference stream where a source wrote a table.
fn every_stated_width(document: &Document) -> Vec<String> {
    let mut stated = Vec::new();
    for number in document.xref().object_numbers().collect::<Vec<_>>() {
        let object = document.get(pdf_syntax::ObjectId::new(number, 0));
        let Some(dict) = object.as_dict() else {
            continue;
        };
        let kind = document.get_key(dict, "Type");
        let kind = kind.as_name().map(|name| name.as_bytes().to_vec());
        if !matches!(kind.as_deref(), Some(b"Font" | b"FontDescriptor")) {
            continue;
        }
        for key in ["Widths", "W", "DW", "DW2", "W2", "MissingWidth"] {
            let value = document.get_key(dict, key);
            if !value.is_null() {
                stated.push(format!("{key}={:?}", document.resolve(&value)));
            }
        }
    }
    stated.sort_unstable();
    stated
}

/// Converts every corpus document under `part` to `target` and states the three properties.
fn sweep(root: &Path, part: &str, target: Target, authorised: Authorisations) -> Tally {
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

        let kept = bytes.clone();
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Archive(ArchivePlan {
                source: 0,
                names: "out.pdf".parse().expect("a pattern"),
                target,
                authorised,
                profile: None,
                substitute_fonts: true,
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
            (false, Some(output)) => {
                // **Property 3: no glyph moves.** `doc/pdf-a-conversion-limits.md` section 4.9
                // sets out three ways to make a font dictionary's widths agree with its
                // program's and calls the third — restating the dictionary — *never*, because
                // ISO 32000-2 §9.2.4 makes those numbers what positions the glyphs. A converter
                // that took it would move every line of every page it touched, and nothing in
                // the report would say so. So it is checked where it can be checked absolutely:
                // every width every font in the output states is the width the source stated.
                let held = Document::open_with_limits(output, Limits::DEFAULT)
                    .expect("the converted document re-opens");
                let source =
                    Document::open_with_limits(kept, Limits::DEFAULT).expect("the source re-opens");
                assert_eq!(
                    every_stated_width(&held),
                    every_stated_width(&source),
                    "{}: a width the file states was rewritten, which moves the text",
                    path.display()
                );
                tally.converted = tally.converted.saturating_add(1);
            }
            (false, None) => {
                tally.refused = tally.refused.saturating_add(1);
                let conversion = report.archive.as_ref().expect("a conversion is reported");
                for decided in &conversion.decided {
                    if matches!(
                        decided.decision,
                        Decision::Refused(_) | Decision::Unauthorised { .. }
                    ) {
                        let seen = tally.refusals.entry(decided.requirement).or_default();
                        *seen = seen.saturating_add(1);
                    }
                }
            }
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
    let mut converted = 0_usize;
    let nothing_authorised = Authorisations::default();
    let everything = authorise_everything();
    for (part, target) in TARGETS {
        // Two runs, because they answer different questions. The default one is what a person who
        // types the command and answers nothing gets; the authorised one is what this converter
        // can reach, and only its refusals rank a *gap*.
        let plain = sweep(&root, part, target, nothing_authorised);
        let tally = sweep(&root, part, target, everything);
        println!(
            "archive {target}: {} conforming, {} of them still conforming after the conversion; \
             {} failing documents converted, {} refused by name, {} unreadable",
            tally.conforming, tally.stayed, tally.converted, tally.refused, tally.unreadable
        );
        println!(
            "    answering nothing, which is what a user gets by default: {} converted, \
             {} refused",
            plain.converted, plain.refused
        );
        assert_eq!(
            plain.conforming, plain.stayed,
            "a conforming document stays conforming when no loss is authorised either"
        );
        assert_eq!(
            tally.conforming, tally.stayed,
            "every document that conformed before the conversion conforms after it"
        );
        let mut ranked: Vec<_> = tally.refusals.iter().collect();
        ranked.sort_by_key(|(id, count)| (std::cmp::Reverse(**count), **id));
        for (id, count) in ranked.iter().take(MOST_REFUSED) {
            println!("    {count:>4} refused on {id}");
        }
        converted = converted.saturating_add(tally.converted);
    }
    assert!(
        converted > 0,
        "the corpus holds documents these rewrites fix, and none was fixed"
    );
}
