//! An environment variable a document names is one the code reads.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! A knob is how a round points a gate somewhere else — `PDFVIEWER_RASTER_COVERAGE=gpu` runs the
//! corpus gate over the other coverage lane — and a knob's name in a document is a command a round
//! copies. When the crate that reads it is renamed and the document is not, the copied command runs
//! the *default* configuration and reports it as the one asked for: an unknown variable is not an
//! error to the program, only to the reader. `doc/verify.md` and `doc/todo/02` named
//! `PDFVIEWER_QUORRA_SCALE` and its siblings after the corpus gate read `PDFVIEWER_RASTER_SCALE`,
//! and nothing could see it, because no test runs a document.
//!
//! # The two populations
//!
//! **Read**: every `PDFVIEWER_…` written as a string literal in a Rust source of this workspace or
//! of `raster/`, outside a comment — `std::env::var("PDFVIEWER_RASTER_SCALE")` is the shape.
//!
//! **Named**: every `PDFVIEWER_…` in a live document — `CLAUDE.md`, `doc/` less its records,
//! `raster/`'s own `CLAUDE.md` and `doc/HANDOVER.md`, and the scripts and prose under `tools/` —
//! and in a Rust comment anywhere. A record (`conformance::pointers::RECORDS`,
//! `conformance::prose::RECORDS`, the owner's `doc/questions/`, `raster/doc/`'s dated notes)
//! names the variables of its own date and is never rewritten, so a name retired since is its date
//! showing rather than a defect (ADR 1403). A name ending in `_` is a family and resolves when a
//! read name begins with it.
//!
//! # Why a zero, and the calibration
//!
//! There is no version of this tree in which a live document may name a knob nothing reads. The
//! second test plants a retired name and a read one into the sweep's own functions and fails
//! unless the first alone is found (trap 13), planting into the functions rather than into a file
//! for `names.rs`'s reason.

#![expect(
    clippy::print_stdout,
    reason = "test code: the gate prints its denominators and its findings, so a failure and a \
              clean run are read the same way"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The prefix every knob of this project's own carries.
const PREFIX: &str = "PDFVIEWER_";

/// Directories no walk enters: build output, version control, and dependency checkouts.
const SKIPPED: [&str; 4] = ["target", ".git", "node_modules", "__pycache__"];

/// Every name beginning with [`PREFIX`] in `text`, with whether a `"` stands right before it.
fn names_in(text: &str) -> Vec<(String, bool)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find(PREFIX) {
        let start = from.saturating_add(at);
        let preceded = text[..start].chars().next_back();
        let end = text[start..]
            .find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
            .map_or(text.len(), |length| start.saturating_add(length));
        if !preceded.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
            found.push((text[start..end].to_owned(), preceded == Some('"')));
        }
        from = end.max(start.saturating_add(PREFIX.len()));
    }
    found
}

/// Every file under `dir` whose extension is one of `extensions`, recursively.
fn files(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        if path.is_dir() {
            if !SKIPPED.contains(&name.as_str()) {
                files(&path, extensions, out);
            }
        } else if path
            .extension()
            .is_some_and(|e| extensions.contains(&&*e.to_string_lossy()))
        {
            out.push(path);
        }
    }
}

/// Whether a document under `doc/` is a record, by its path relative to the workspace root.
fn is_a_record(relative: &str) -> bool {
    conformance::pointers::RECORDS
        .iter()
        .any(|prefix| relative.starts_with(prefix))
        || conformance::prose::RECORDS.iter().any(|record| {
            relative
                .strip_prefix("doc/")
                .is_some_and(|rest| rest.starts_with(record))
        })
        || relative.starts_with("doc/questions/")
}

/// One place a live document or a comment names a knob.
struct Mention {
    at: String,
    name: String,
}

/// The knobs the code reads, and where every live document or comment names one.
fn sweep(root: &Path) -> (BTreeSet<String>, Vec<Mention>, usize) {
    let mut read = BTreeSet::new();
    let mut mentions = Vec::new();
    let mut sources = Vec::new();
    for top in ["crates", "tools", "raster"] {
        files(&root.join(top), &["rs"], &mut sources);
    }
    for path in &sources {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        let text = fs::read_to_string(path).unwrap_or_default();
        for (number, line) in (1_usize..).zip(text.lines()) {
            let (code, comment) = line.find("//").map_or((line, ""), |at| line.split_at(at));
            for (name, quoted) in names_in(code) {
                if quoted {
                    read.insert(name);
                }
            }
            for (name, _) in names_in(comment) {
                mentions.push(Mention {
                    at: format!("{relative}:{number}"),
                    name,
                });
            }
        }
    }
    let mut documents = vec![
        root.join("CLAUDE.md"),
        root.join("raster/CLAUDE.md"),
        root.join("raster/doc/HANDOVER.md"),
    ];
    files(&root.join("doc"), &["md"], &mut documents);
    files(
        &root.join("tools"),
        &["sh", "py", "md", "toml"],
        &mut documents,
    );
    let mut read_documents: usize = 0;
    for path in &documents {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        if is_a_record(&relative) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        read_documents = read_documents.saturating_add(1);
        for (number, line) in (1_usize..).zip(text.lines()) {
            for (name, _) in names_in(line) {
                mentions.push(Mention {
                    at: format!("{relative}:{number}"),
                    name,
                });
            }
        }
    }
    (read, mentions, read_documents)
}

/// The mentions that name nothing `read` holds.
fn unread<'a>(read: &BTreeSet<String>, mentions: &'a [Mention]) -> Vec<&'a Mention> {
    mentions
        .iter()
        .filter(|mention| {
            if mention.name.ends_with('_') {
                !read.iter().any(|name| name.starts_with(&mention.name))
            } else {
                !read.contains(&mention.name)
            }
        })
        .collect()
}

#[test]
fn every_variable_a_live_document_names_is_one_the_code_reads() {
    let root = conformance::workspace_root();
    let (read, mentions, documents) = sweep(&root);
    println!(
        "{} variable(s) read by the code, {} mention(s) in {documents} live document(s) and the \
         Rust comments",
        read.len(),
        mentions.len()
    );
    assert!(
        !read.is_empty() && !mentions.is_empty() && documents > 0,
        "the sweep read nothing: {} read, {} mention(s), {documents} document(s) — a clean answer \
         over an empty population is a sentence about the sweep",
        read.len(),
        mentions.len(),
    );
    let absent: Vec<String> = unread(&read, &mentions)
        .iter()
        .map(|mention| format!("{} {}", mention.at, mention.name))
        .collect();
    assert!(
        absent.is_empty(),
        "a document names an environment variable no code reads, so a command copied from it runs \
         the default and reports it as what was asked. Name the variable the code reads:\n  {}",
        absent.join("\n  ")
    );
}

#[test]
fn the_sweep_names_a_planted_variable_nothing_reads() {
    let read: BTreeSet<String> = names_in(r#"std::env::var("PDFVIEWER_RASTER_SCALE")"#)
        .into_iter()
        .filter_map(|(name, quoted)| quoted.then_some(name))
        .collect();
    assert_eq!(read.len(), 1, "a string literal is a read: {read:?}");
    let planted: Vec<Mention> = names_in(
        "`PDFVIEWER_QUORRA_SCALE=4` beside `PDFVIEWER_RASTER_SCALE=4`, the `PDFVIEWER_RASTER_` \
         family, and XPDFVIEWER_NOT_OURS",
    )
    .into_iter()
    .map(|(name, _)| Mention {
        at: "plant".to_owned(),
        name,
    })
    .collect();
    assert_eq!(
        planted.len(),
        3,
        "the prefix inside another word is not a knob"
    );
    let found: Vec<&str> = unread(&read, &planted)
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(found, ["PDFVIEWER_QUORRA_SCALE"]);
}
