//! `doc/questions/`'s own rule, checked: one number, one question, and an answer that can be
//! addressed to it.
//!
//! Not a conformance question, and it lives here for the reason `bounded.rs`, `sandbox_gates.rs`,
//! `submodules.rs` and `workspaces.rs` do: this is the crate whose gates read the repository's own
//! files rather than a PDF, and `cargo test -p conformance` is the last line of `doc/todo/02` §2's
//! sequence — which every merge round runs, and a merge is where this defect is made.
//!
//! # What it guards
//!
//! The directory's convention is that a question is a `Q` file and the owner answers it with an
//! **`A` file of the same name**. That is the whole index: a `Q` with no `A` is open, and no round
//! needs to read anything else. It rests on a number identifying exactly one question — and on
//! 2026-09-04 two rounds took `Q27`, each by reading `ls doc/questions/` on a branch that could not
//! see the other's, and the collision reached `main` three rounds later in a merge with every gate
//! green. Nothing dangled, because each reference cited a filename rather than a number; what broke
//! is that **one of the two could never be answered** — an `A27` would have named a question and
//! not said which. Session 934 renumbered one and wrote [ADR 0908]; this is the half that keeps it
//! from happening again quietly.
//!
//! [ADR 0908]: ../../../doc/adr/0908-two-questions-called-q27.md
//!
//! # What it does not guard
//!
//! It cannot make the collision *impossible*: two branches still cannot see each other, and the
//! allocator is outside the tree — the block of numbers a round is given in its instruction. What
//! this makes impossible is a duplicate surviving the merge that creates it, which is the step
//! where somebody is looking. `doc/questions/README.md` states both halves.
//!
//! **Gaps are not a defect and are not checked.** A round takes a number from its own reserved
//! block, so the numbers here are sparse by construction, and a test that demanded a run from 1
//! would fail every round that used its block honestly.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read its own repository's questions directory has \
              not found a defect, and reporting that as one would be worse than stopping"
)]
#![expect(
    clippy::print_stdout,
    reason = "the gate prints the index it checked, which is what makes a passing run worth \
              reading"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The directory this test is about, relative to the repository root.
const QUESTIONS: &str = "doc/questions";

/// The one file in it that is not a question.
const README: &str = "README.md";

fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`, so two levels up is the root. This
    // cannot fail for a crate that is in the workspace, which is the only way this test runs.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// One file of `doc/questions/`, split into the three parts its name is made of.
struct Entry {
    /// `Q` for a question, `A` for the owner's answer.
    letter: char,
    /// The number the two share, as written — so that `Q7` and `Q07` are told apart rather than
    /// silently reconciled, which is a collision of its own shape.
    number: String,
    /// Everything after the number's hyphen, without the `.md`.
    slug: String,
    /// The whole file name, for the report.
    file: String,
}

/// Split `Q27-a-font-the-file-does-not-carry.md` into its letter, number and slug.
///
/// Returns `None` for anything that is not `<letter><digits>-<slug>.md`, which the caller reports:
/// a name this cannot read is a name no round can resolve an answer against either.
fn parse(file: &str) -> Option<Entry> {
    let stem = file.strip_suffix(".md")?;
    let mut characters = stem.chars();
    let letter = characters.next()?;
    if letter != 'Q' && letter != 'A' {
        return None;
    }
    let rest = characters.as_str();
    let hyphen = rest.find('-')?;
    let (number, slug) = rest.split_at(hyphen);
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let slug = slug.get(1..)?;
    if slug.is_empty() {
        return None;
    }
    Some(Entry {
        letter,
        number: number.to_owned(),
        slug: slug.to_owned(),
        file: file.to_owned(),
    })
}

fn read_directory(root: &Path) -> Vec<PathBuf> {
    let directory = root.join(QUESTIONS);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("doc/questions/ is a directory of this repository")
        .map(|entry| entry.expect("a readable directory entry").path())
        .collect();
    files.sort();
    files
}

#[test]
fn every_question_has_a_number_of_its_own() {
    let root = repository_root();
    let mut wrong = String::new();

    // Keyed by number so that a duplicate is a value with more than one entry, and ordered so that
    // the report reads in the same order as the directory.
    let mut questions: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    let mut answers: BTreeMap<String, Vec<Entry>> = BTreeMap::new();

    for path in read_directory(root) {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a file name that is UTF-8")
            .to_owned();
        if name == README {
            continue;
        }
        let Some(entry) = parse(&name) else {
            let _ = writeln!(
                wrong,
                "  {QUESTIONS}/{name} is not <Q or A><number>-<slug>.md, so no answer can be \
                 addressed to it"
            );
            continue;
        };
        match entry.letter {
            'Q' => questions
                .entry(entry.number.clone())
                .or_default()
                .push(entry),
            _ => answers.entry(entry.number.clone()).or_default().push(entry),
        }
    }

    // The defect this test was written for. Two questions sharing a number is not a tidiness
    // matter: the directory's whole index is "an `A` file of the same name", and one of the two
    // could never be answered.
    for (number, entries) in &questions {
        if entries.len() > 1 {
            let names: Vec<&str> = entries.iter().map(|e| e.file.as_str()).collect();
            let _ = writeln!(
                wrong,
                "  the number {number} is taken by {} questions — {} — so an A{number} would name \
                 a question and not say which. Renumber one out of the round's own reserved block \
                 (doc/questions/README.md)",
                entries.len(),
                names.join(", ")
            );
        }
    }
    for (number, entries) in &answers {
        if entries.len() > 1 {
            let names: Vec<&str> = entries.iter().map(|e| e.file.as_str()).collect();
            let _ = writeln!(
                wrong,
                "  the number {number} is taken by {} answers — {}",
                entries.len(),
                names.join(", ")
            );
        }
    }

    // The other half of the convention, and the half a rename breaks: an answer's name is its
    // question's with the letter changed, so a slug that drifted is an answer nobody's `ls` pairs
    // up.
    for (number, entries) in &answers {
        let Some(question) = questions.get(number).and_then(|q| q.first()) else {
            let _ = writeln!(
                wrong,
                "  {QUESTIONS}/{} answers Q{number}, which is not in this directory",
                entries[0].file
            );
            continue;
        };
        for answer in entries {
            if answer.slug != question.slug {
                let _ = writeln!(
                    wrong,
                    "  {QUESTIONS}/{} should be named for the question it answers — {} with A for \
                     Q",
                    answer.file, question.file
                );
            }
        }
    }

    assert!(wrong.is_empty(), "\n{wrong}");

    let open: Vec<&str> = questions
        .iter()
        .filter(|(number, _)| !answers.contains_key(*number))
        .filter_map(|(_, entries)| entries.first().map(|entry| entry.file.as_str()))
        .collect();
    println!(
        "{QUESTIONS}/ holds {} questions with a number apiece, {} of them answered and {} open:",
        questions.len(),
        answers.len(),
        open.len()
    );
    for file in &open {
        println!("  {file}");
    }
}
