//! A comment that says a table, a clause or an erratum is not in `doc/md/` says so truly.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! A sentence saying the standard's conversion lacks something is a claim about the
//! specification, and `CLAUDE.md` principle 5 says such a claim decays. `attachment.rs` said that
//! Table 409a was not in `doc/md/` while the table sat there under its erratum's heading, and the
//! function it documented was waiting to narrow on the day the table could be read. Nothing read
//! the sentence against the directory it was about.
//!
//! # What it reads
//!
//! Every comment paragraph — a run of `//`, `///` or `//!` lines — under the workspace's source
//! roots ([`conformance::roots::source_roots`]), cut into segments: sentences, and the clauses a
//! semicolon, a colon, a parenthesis or a dash sets apart inside one. A segment is a **claim**
//! when a phrase of absence (`absent from`, `not in`, `missing from`, `has neither`, `holds no`,
//! `carries none` and their kin) sits within a few words of `doc/md`. Its **subjects** are what it
//! names that the directory can be searched for: a `Table` number, an erratum's `Issue #` number,
//! a `§` clause, and a backticked PDF name. A claim that says *the table*, *the clause*, *the
//! erratum* or *the name* also takes the last one of that kind named earlier in the paragraph,
//! which is how the sentence that started this was written.
//!
//! A subject is **present** when `doc/md/`'s text holds it: a table, an issue or a name as that
//! label with nothing alphanumeric after it, and a clause as a Markdown heading opening with its
//! number. A claim with a present subject fails, naming the file, the line and
//! the sentence. A claim with no searchable subject — "the caret's text", "the standard's
//! equations" — is counted and printed, because the directory cannot answer it.
//!
//! # And the calibration
//!
//! A sweep that comes back clean is a sentence about the sweep (trap 13). The second test runs the
//! sweep's own functions over the sentence that started this and over a claim that is true, and
//! fails unless the first is found and the second is not. ADR 1624.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              the gate prints the population it judged, which is what makes its verdict readable"
)]

use std::fs;
use std::path::{Path, PathBuf};

/// The phrases that say something is absent. Matched case-insensitively, as whole words.
const ABSENCE: [&str; 17] = [
    "absent from",
    "not in",
    "missing from",
    "nowhere in",
    "has neither",
    "has no",
    "holds no",
    "holds not",
    "carries no",
    "carries none",
    "does not hold",
    "does not carry",
    "does not have",
    "does not contain",
    "lacks",
    "omits",
    "is without",
];

/// How far, in bytes, a phrase of absence may sit from `doc/md` and still be about it — a few
/// words, so that `doc/md/` named as a source two clauses away is not read as the claim's object.
const NEAR: usize = 40;

/// One thing a claim names that `doc/md/` can be searched for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Subject {
    /// `Table 409a`.
    Table(String),
    /// `Issue #374`.
    Issue(String),
    /// `§14.13.5`.
    Clause(String),
    /// A backticked PDF name, `/BrotliDecode`, searched for without its solidus.
    Name(String),
}

impl Subject {
    fn label(&self) -> String {
        match self {
            Self::Table(number) => format!("Table {number}"),
            Self::Issue(number) => format!("Issue #{number}"),
            Self::Clause(number) => format!("§{number}"),
            Self::Name(name) => format!("`/{name}`"),
        }
    }
}

/// A claim of absence whose subject `doc/md/` holds.
#[derive(Debug)]
struct Finding {
    at: String,
    subject: String,
    sentence: String,
}

/// What the sweep read.
#[derive(Debug, Default)]
struct Swept {
    claims: usize,
    unanswerable: Vec<String>,
    findings: Vec<Finding>,
}

/// The standard's conversion, every file of `doc/md/` concatenated, and its heading lines apart.
struct Markdown {
    text: String,
    headings: Vec<String>,
}

impl Markdown {
    fn read(root: &Path) -> Self {
        let mut paths: Vec<PathBuf> = fs::read_dir(root.join("doc").join("md"))
            .expect("doc/md/ is unpacked from doc/specifications.zip (ADR 0187)")
            .map(|entry| entry.expect("a directory entry is readable").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .collect();
        paths.sort();
        let mut text = String::new();
        for path in &paths {
            text.push_str(&fs::read_to_string(path).expect("a doc/md file is readable"));
            text.push('\n');
        }
        let headings = text
            .lines()
            .filter(|line| line.starts_with('#'))
            .map(|line| line.trim_start_matches('#').trim().to_owned())
            .collect();
        Self { text, headings }
    }

    fn holds(&self, subject: &Subject) -> bool {
        match subject {
            Subject::Table(number) => labelled(&self.text, &format!("Table {number}")),
            Subject::Issue(number) => labelled(&self.text, &format!("Issue #{number}")),
            Subject::Clause(number) => self.headings.iter().any(|heading| {
                heading
                    .strip_prefix(number.as_str())
                    .is_some_and(|rest| rest.starts_with(' '))
            }),
            Subject::Name(name) => labelled(&self.text, name),
        }
    }
}

/// Whether `text` holds `label` with nothing alphanumeric after it, so that `Table 40` is not
/// found inside `Table 409a`.
fn labelled(text: &str, label: &str) -> bool {
    text.match_indices(label).any(|(index, _)| {
        text.get(index.saturating_add(label.len())..)
            .and_then(|after| after.chars().next())
            .is_none_or(|next| !next.is_alphanumeric())
    })
}

/// The paragraphs of comment text in one source, each with the line it starts on: a run of comment
/// lines, ended by a line that is not one or by a comment line with nothing on it.
fn paragraphs(source: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (number, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//") {
            let rest = rest
                .strip_prefix('/')
                .or_else(|| rest.strip_prefix('!'))
                .unwrap_or(rest)
                .trim();
            if rest.is_empty() {
                found.extend(current.take());
                continue;
            }
            let (_, text) =
                current.get_or_insert_with(|| (number.saturating_add(1), String::new()));
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(rest);
        } else if let Some(done) = current.take() {
            found.push(done);
        }
    }
    found.extend(current);
    found
}

/// A paragraph cut into segments, each with its byte offset: at a sentence's end — a `.`, `!` or
/// `?` followed, past any closing emphasis or quotation, by white space, which leaves `§10.4.2.4`
/// whole — and at the marks that set a
/// clause apart inside one, `;`, `:`, a parenthesis and a dash. A segment is the unit a claim is
/// read in, so that a parenthesis saying *the caret's text is not in `doc/md/`* is not charged
/// with the clause number the sentence around it names.
fn segments(paragraph: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut start = 0;
    for (index, character) in paragraph.char_indices() {
        let end = index.saturating_add(character.len_utf8());
        let rest = paragraph.get(end..).unwrap_or_default();
        let next_is_space = rest
            .trim_start_matches(['*', '`', ')', '"'])
            .chars()
            .next()
            .is_none_or(char::is_whitespace);
        let ends = match character {
            '.' | '!' | '?' => next_is_space,
            ';' | ':' | '(' | ')' | '—' => true,
            _ => false,
        };
        if ends {
            found.push((start, &paragraph[start..end]));
            start = end;
        }
    }
    found.push((start, &paragraph[start..]));
    found.retain(|(_, segment)| !segment.trim().is_empty());
    found
}

/// Whether `segment` says something is absent from `doc/md`.
fn is_claim(segment: &str) -> bool {
    let lower = segment.to_lowercase();
    lower.match_indices("doc/md").any(|(at, _)| {
        ABSENCE.iter().any(|phrase| {
            lower.match_indices(phrase).any(|(index, _)| {
                let whole = lower[..index]
                    .chars()
                    .next_back()
                    .is_none_or(|before| !before.is_alphanumeric())
                    && lower[index.saturating_add(phrase.len())..]
                        .chars()
                        .next()
                        .is_none_or(|after| !after.is_alphanumeric());
                let distance = if index < at {
                    at.saturating_sub(index.saturating_add(phrase.len()))
                } else {
                    index.saturating_sub(at.saturating_add("doc/md".len()))
                };
                whole && distance <= NEAR
            })
        })
    })
}

/// The run of characters after `prefix` that `keep` accepts, at each place `prefix` occurs.
fn after<'a>(text: &'a str, prefix: &str, keep: impl Fn(char) -> bool + Copy) -> Vec<&'a str> {
    text.match_indices(prefix)
        .filter_map(|(index, _)| {
            let rest = &text[index.saturating_add(prefix.len())..];
            let end = rest.find(|c: char| !keep(c)).unwrap_or(rest.len());
            let run = rest[..end].trim_end_matches('.');
            (!run.is_empty() && run.starts_with(|c: char| c.is_ascii_digit())).then_some(run)
        })
        .collect()
}

/// Everything `text` names that `doc/md/` can be searched for, in the order it names them. A
/// backticked name counts when it is a PDF name, `/BrotliDecode` — or, where `bare` is set, any
/// backticked word, which is what *the name* can refer back to.
fn subjects(text: &str, bare: bool) -> Vec<(usize, Subject)> {
    let mut found = Vec::new();
    for (index, _) in text.match_indices("Table ") {
        if let Some(run) = after(&text[index..], "Table ", |c| c.is_ascii_alphanumeric()).first() {
            found.push((index, Subject::Table((*run).to_owned())));
        }
    }
    for (index, _) in text.match_indices("Issue #") {
        if let Some(run) = after(&text[index..], "Issue #", |c| c.is_ascii_digit()).first() {
            found.push((index, Subject::Issue((*run).to_owned())));
        }
    }
    for (index, _) in text.match_indices('§') {
        if let Some(run) = after(&text[index..], "§", |c| c.is_ascii_digit() || c == '.').first() {
            found.push((index, Subject::Clause((*run).to_owned())));
        }
    }
    let mut ticks = text.match_indices('`').map(|(index, _)| index);
    while let (Some(open), Some(close)) = (ticks.next(), ticks.next()) {
        let ticked = &text[open.saturating_add(1)..close];
        let Some(name) = ticked.strip_prefix('/').or(bare.then_some(ticked)) else {
            continue;
        };
        let pdf_name = name.chars().next().is_some_and(char::is_alphabetic)
            && name.chars().all(char::is_alphanumeric);
        if pdf_name {
            found.push((open, Subject::Name(name.to_owned())));
        }
    }
    found.sort_by_key(|(index, _)| *index);
    found
}

/// The subjects of one claim: its own, and the last of each kind it refers back to — *the table*,
/// *the clause*, *the erratum*, *the name* — among what the paragraph named before it.
fn claim_subjects(before: &str, segment: &str) -> Vec<Subject> {
    let mut found: Vec<Subject> = subjects(segment, false)
        .into_iter()
        .map(|(_, s)| s)
        .collect();
    let lower = segment.to_lowercase();
    let earlier: Vec<Subject> = subjects(before, true).into_iter().map(|(_, s)| s).collect();
    let last = |wanted: fn(&Subject) -> bool| earlier.iter().rev().find(|s| wanted(s)).cloned();
    if lower.contains("the table") {
        found.extend(last(|s| matches!(s, Subject::Table(_))));
    }
    if lower.contains("the clause") || lower.contains("the subclause") {
        found.extend(last(|s| matches!(s, Subject::Clause(_))));
    }
    if lower.contains("the erratum") || lower.contains("the issue") {
        found.extend(last(|s| matches!(s, Subject::Issue(_))));
    }
    if lower.contains("the name") {
        found.extend(last(|s| matches!(s, Subject::Name(_))));
    }
    found.dedup();
    found
}

/// Reads one source's comments against `doc/md/`, adding what it finds to `swept`.
fn sweep_source(markdown: &Markdown, at: &str, source: &str, swept: &mut Swept) {
    for (line, paragraph) in paragraphs(source) {
        for (offset, segment) in segments(&paragraph) {
            if !is_claim(segment) {
                continue;
            }
            swept.claims = swept.claims.saturating_add(1);
            let segment = segment.trim();
            let named = claim_subjects(&paragraph[..offset], segment);
            if named.is_empty() {
                swept.unanswerable.push(format!("{at}:{line}: {segment}"));
            }
            for subject in named.iter().filter(|subject| markdown.holds(subject)) {
                swept.findings.push(Finding {
                    at: format!("{at}:{line}"),
                    subject: subject.label(),
                    sentence: segment.to_owned(),
                });
            }
        }
    }
}

/// Every `.rs` file under `directory`, build output and hidden directories left out.
fn rust_sources(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("a directory entry is readable").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if path.is_dir() {
            rust_sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

#[test]
fn a_comment_saying_doc_md_lacks_something_is_read_against_doc_md() {
    let root = conformance::workspace_root();
    let markdown = Markdown::read(&root);
    let own = Path::new(file!())
        .file_name()
        .expect("this test's own path names a file");
    let mut sources = Vec::new();
    for directory in conformance::roots::source_roots(&root).expect("the workspace's source roots")
    {
        rust_sources(&root.join(directory), &mut sources);
    }
    sources.sort();
    let mut swept = Swept::default();
    for path in &sources {
        let is_this_file = path.file_name() == Some(own)
            && path.starts_with(root.join("tools").join("conformance").join("tests"));
        if is_this_file {
            continue;
        }
        let source = fs::read_to_string(path).expect("a source file of the tree is readable");
        let relative = path.strip_prefix(&root).unwrap_or(path);
        sweep_source(
            &markdown,
            &relative.display().to_string(),
            &source,
            &mut swept,
        );
    }
    println!(
        "{} source(s), {} claim(s) that doc/md/ lacks something, {} with no subject doc/md/ can \
         be searched for:",
        sources.len(),
        swept.claims,
        swept.unanswerable.len()
    );
    for claim in &swept.unanswerable {
        println!("  {claim}");
    }
    assert!(
        !sources.is_empty(),
        "the sweep read no source — a clean answer over an empty population is a sentence about \
         the sweep"
    );
    let findings: Vec<String> = swept
        .findings
        .iter()
        .map(|finding| {
            format!(
                "{} says {} is not in doc/md/, and it is: {}",
                finding.at, finding.subject, finding.sentence
            )
        })
        .collect();
    assert!(
        findings.is_empty(),
        "a comment says the standard's conversion lacks something it holds — a claim about the \
         specification that has decayed (`CLAUDE.md` principle 5). Read the thing and state what \
         it says:\n  {}",
        findings.join("\n  ")
    );
}

#[test]
fn the_sweep_finds_a_planted_claim_and_passes_a_true_one() {
    let markdown = Markdown::read(&conformance::workspace_root());
    let planted = "\
/// Errata Collection 3 states the key against \"Table 409a - Property list entries for associated
/// files\". `doc/md/` has neither the caret nor the table, so the key cannot be verified.
fn planted() {}

/// Table 9999 is absent from `doc/md/`, and so is Issue #99999.
fn true_claim() {}
";
    let mut swept = Swept::default();
    sweep_source(&markdown, "planted.rs", planted, &mut swept);
    assert_eq!(swept.claims, 2, "both sentences are claims: {swept:?}");
    let labels: Vec<&str> = swept
        .findings
        .iter()
        .map(|finding| finding.subject.as_str())
        .collect();
    assert_eq!(
        labels,
        ["Table 409a"],
        "the sentence that started this refers back to Table 409a, which doc/md/ holds under \
         Issue #374; the true claim names a table and an erratum that do not exist"
    );
    assert!(
        !is_claim("`doc/md/` is where the quotation is checked, and the table is not in this file"),
        "an absence more than a few words from doc/md is not a claim about doc/md"
    );
}
