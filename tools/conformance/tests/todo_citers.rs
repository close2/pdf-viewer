//! A done todo file's `Cited by:` line names citers that cite it.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! A todo file whose item is done stays whole only while something cites it, and its header says
//! what (ADR 1416): "Cited by: comments in `crates/pdf-model`; §10.5's ledger note". That line is
//! the file's reason to exist and nothing held it. When the last citer moves on — a comment
//! rewritten, a note that stops naming the file — the line goes on claiming a reader who is no
//! longer there, and the file outlives its reason in silence. So each claim is read against the
//! place it names: a directory holds a file mentioning `todo/NN`, a named document mentions it,
//! and a ledger row's note mentions it.
//!
//! # What a claim is
//!
//! Only the line's first sentence, up to ` — ` or the first full stop, which is where every one of
//! these lines states its citers; `doc/todo/12`'s later sentences name files that cite an
//! *earlier* item of the same number, and those are not citers. Within it: a backticked path to a
//! directory is "comments in" it; a backticked path to a file (`CLAUDE.md`, a `.md` under `doc/`)
//! is that file; a backticked command is neither; and where the sentence speaks of a ledger note,
//! each `§` in it is a row.

#![expect(
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              the gate prints its population so that a clean run is read the same way as a failure"
)]

use std::path::Path;

use conformance::ledger::Ledger;

/// Whether `text` mentions todo item `number` by its directory and number, not a longer number.
fn mentions(text: &str, number: &str) -> bool {
    text.split(format!("todo/{number}").as_str())
        .skip(1)
        .any(|after| !after.starts_with(|next: char| next.is_ascii_digit()))
}

/// Whether any Rust source under `directory` mentions item `number`.
fn directory_mentions(directory: &Path, number: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            directory_mentions(&path, number)
        } else {
            // "Comments in" a directory: its Rust sources, not the fixtures beside them.
            path.extension().is_some_and(|suffix| suffix == "rs")
                && std::fs::read_to_string(&path).is_ok_and(|text| mentions(&text, number))
        }
    })
}

/// The claims a `Cited by:` sentence makes, each with whether it holds.
fn unresolved(root: &Path, ledger: &Ledger, number: &str, sentence: &str) -> Vec<String> {
    let mut wrong = Vec::new();
    // A backticked span with a space in it is a command the line mentions beside its citers —
    // `doc/todo/39`'s `tools/state.sh annex-o` — and not a place that cites.
    for path in sentence
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|span| !span.contains(char::is_whitespace))
    {
        let at = root.join(path);
        let holds = if at.is_dir() {
            directory_mentions(&at, number)
        } else if at.is_file() {
            std::fs::read_to_string(&at).is_ok_and(|text| mentions(&text, number))
        } else {
            false
        };
        if !holds {
            wrong.push(format!("`{path}` does not mention todo/{number}"));
        }
    }
    if sentence.contains("ledger note") {
        for clause in sentence.split('§').skip(1) {
            let clause: String = clause
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let clause = clause.trim_end_matches('.');
            let note = ledger
                .rows
                .iter()
                .find(|row| row.clause.to_string() == clause)
                .and_then(|row| row.note.as_deref());
            if !note.is_some_and(|note| mentions(note, number)) {
                wrong.push(format!(
                    "§{clause}'s ledger note does not mention todo/{number}"
                ));
            }
        }
    }
    wrong
}

/// The first sentence of a `Cited by:` header, which runs until a line that starts another
/// header field, and ends at ` — ` or the first full stop followed by a space.
fn cited_by(text: &str) -> Option<String> {
    let mut lines = text
        .lines()
        .skip_while(|line| !line.starts_with("Cited by:"));
    let mut header = lines.next()?.trim_start_matches("Cited by:").to_owned();
    for line in lines {
        let starts_field = line
            .split_once(':')
            .is_some_and(|(field, _)| !field.is_empty() && field.chars().all(char::is_alphabetic));
        if starts_field || line.is_empty() {
            break;
        }
        header.push(' ');
        header.push_str(line);
    }
    let end = [header.find(" — "), header.find(". ")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(header.len());
    Some(header[..end].to_owned())
}

#[test]
fn every_cited_by_line_names_citers_that_cite_the_file() {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER)).expect("the ledger");
    let mut files = 0usize;
    let mut wrong = Vec::new();
    for entry in std::fs::read_dir(root.join("doc/todo")).expect("doc/todo") {
        let path = entry.expect("a directory entry").path();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(number) = name.split('-').next().filter(|n| n.len() == 2) else {
            continue;
        };
        let text = std::fs::read_to_string(&path).expect("a todo file reads");
        let Some(sentence) = cited_by(&text) else {
            continue;
        };
        files += 1;
        wrong.extend(
            unresolved(&root, &ledger, number, &sentence)
                .into_iter()
                .map(|why| format!("doc/todo/{name}: {why}")),
        );
    }
    println!("{files} todo file(s) carry a `Cited by:` line");
    assert!(
        files >= 5,
        "only {files} `Cited by:` lines found: the parse is measuring nothing"
    );
    assert!(
        wrong.is_empty(),
        "a done todo file stays whole because something cites it (ADR 1416), and these claims of \
         its header no longer hold — correct the line, or cut the file if nothing cites it:\n  {}",
        wrong.join("\n  ")
    );
}

/// Trap 13: the sweep is run against the defect it looks for before it is believed — a line
/// naming a citer that does not cite, beside ones that do.
#[test]
fn a_cited_by_line_naming_a_citer_that_does_not_cite_is_a_finding() {
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER)).expect("the ledger");
    let text = "Status: done.\nCited by: `CLAUDE.md`'s clause 10 entry; comments in \
                `crates/pdf-model` and `crates/pdf-ccitt`; §10.5's ledger note — the reason.\n\
                Priority: 13\n";
    let sentence = cited_by(text).expect("the planted line");
    let wrong = unresolved(&root, &ledger, "13", &sentence);
    assert_eq!(
        wrong,
        vec!["`crates/pdf-ccitt` does not mention todo/13".to_owned()],
        "{sentence}"
    );
    assert!(!mentions("see doc/todo/130", "13"));
}
