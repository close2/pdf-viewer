//! Emits the specifications' annotations, and reports the quotations that land on retired text.
//!
//! ```sh
//! # what the conversion dropped, keyed to page and clause
//! cargo run --release -p spec-errata -- emit doc/*.pdf > doc/errata.md
//! # the count, which is the part that may be written down
//! cargo run --release -p spec-errata -- census doc/*.pdf
//! # the hazard: rustdoc blockquotes that quote a sentence an erratum struck out
//! cargo run --release -p spec-errata -- check doc/*.pdf
//! ```
//!
//! `emit`'s output is derived from documents this project may not redistribute (ADR 0187) and
//! belongs under the same `.gitignore` as `doc/md/`. `check`'s output is about *this tree's*
//! source and may be acted on in the open.

#![expect(
    clippy::print_stdout,
    reason = "a command-line tool whose entire output is a report"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use spec_errata::{Note, Role};

/// What the tool was asked for.
enum Command {
    /// The notes as Markdown.
    Emit,
    /// Counts alone.
    Census,
    /// Quotations in `crates/` that quote struck-out text, and struck text `doc/md/` still
    /// carries.
    Check,
}

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let command = match arguments.next().as_deref() {
        Some("emit") => Command::Emit,
        Some("census") => Command::Census,
        Some("check") => Command::Check,
        _ => {
            println!("usage: spec-errata <emit|census|check> <document.pdf>...");
            return std::process::ExitCode::FAILURE;
        }
    };
    let paths: Vec<PathBuf> = arguments.map(PathBuf::from).collect();
    let mut notes = Vec::new();
    for path in &paths {
        match spec_errata::read(path) {
            Ok(read) => notes.extend(read),
            Err(error) => {
                println!("{error}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    match command {
        Command::Emit => print!("{}", spec_errata::markdown(&notes)),
        Command::Census => census(&notes),
        Command::Check => {
            if let Err(error) = check(&notes) {
                println!("{error}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    std::process::ExitCode::SUCCESS
}

/// The counts, per document and by subtype and role.
fn census(notes: &[Note]) {
    let mut by_document: BTreeMap<&str, Vec<&Note>> = BTreeMap::new();
    for note in notes {
        by_document
            .entry(note.document.as_str())
            .or_default()
            .push(note);
    }
    for (document, notes) in by_document {
        let mut subtypes: BTreeMap<&str, usize> = BTreeMap::new();
        let mut roles: BTreeMap<&str, usize> = BTreeMap::new();
        let mut subjects: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut retiring = 0_usize;
        let mut retired_words = 0_usize;
        for note in &notes {
            let subtype = subtypes.entry(note.subtype.as_str()).or_default();
            *subtype = subtype.saturating_add(1);
            let role = roles.entry(note.role.as_str()).or_default();
            *role = role.saturating_add(1);
            if let Some(subject) = note.subject.as_deref() {
                subjects.insert(subject);
            }
            if note.retires_text() {
                retiring = retiring.saturating_add(1);
                retired_words = retired_words.saturating_add(
                    note.covered
                        .as_deref()
                        .map_or(0, |text| text.split_whitespace().count()),
                );
            }
        }
        println!("{document}");
        println!("  {} note(s), links and popups excluded", notes.len());
        println!("  subtypes: {}", joined(&subtypes));
        println!("  roles: {}", joined(&roles));
        println!("  distinct /Subj: {}", subjects.len());
        println!("  strikeouts over text: {retiring} ({retired_words} words retired)");
        let sections = notes
            .iter()
            .filter(|note| note.role != Role::Reply)
            .filter_map(|note| note.section.as_deref())
            .collect::<std::collections::BTreeSet<&str>>();
        println!("  sections touched: {}", sections.len());
    }
}

/// The two questions the census raised, asked of this tree and of `doc/md/`.
///
/// # Errors
///
/// Whatever [`spec_errata::landings`] or [`spec_errata::still_in_conversion`] answered.
fn check(notes: &[Note]) -> Result<(), spec_errata::Error> {
    let still = spec_errata::still_in_conversion(notes, &PathBuf::from("doc/md"))?;
    println!(
        "{} struck passage(s) of {} words or more that doc/md/ still carries as current text",
        still.len(),
        spec_errata::MIN_WORDS
    );
    for note in &still {
        println!(
            "  {} p.{} {} — {}",
            note.document,
            note.page,
            note.subject.as_deref().unwrap_or("(no /Subj)"),
            note.section.as_deref().unwrap_or("(no section)")
        );
    }
    let mut landings =
        spec_errata::landings(notes, &[PathBuf::from("crates"), PathBuf::from("tools")])?;
    landings.extend(spec_errata::ledger_landings(
        notes,
        &PathBuf::from("doc/conformance/ledger.toml"),
    )?);
    let (cited, elsewhere): (Vec<_>, Vec<_>) = landings
        .iter()
        .partition(|landing| spec_errata::Landing::in_clause(landing));
    println!(
        "\n{} quotation(s) quote text struck out of the clause they cite{}",
        cited.len(),
        by_kind(&cited)
    );
    for landing in &cited {
        print_landing(landing);
    }
    println!(
        "\n{} more match a passage struck out of another clause{} — a repeated phrase rather than \
         a finding, until somebody reads one",
        elsewhere.len(),
        by_kind(&elsewhere)
    );
    for landing in &elsewhere {
        print_landing(landing);
    }
    Ok(())
}

/// The three populations' shares of a list of landings, since only one of them has a gate.
fn by_kind(landings: &[&spec_errata::Landing]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for landing in landings {
        let count = counts.entry(landing.kind.as_str()).or_default();
        *count = count.saturating_add(1);
    }
    if counts.is_empty() {
        return String::new();
    }
    format!(" ({})", joined(&counts))
}

/// One landing, with the erratum's own words beside the quotation's.
fn print_landing(landing: &spec_errata::Landing) {
    println!(
        "  [{}] {}:{} §{} — {} p.{} {} [{}]",
        landing.kind.as_str(),
        landing.file.display(),
        landing.line,
        landing.clause.as_deref().unwrap_or("?"),
        landing.note.document,
        landing.note.page,
        landing.note.subject.as_deref().unwrap_or("(no /Subj)"),
        landing
            .note
            .states
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>()
            .join(", ")
    );
    println!("      quoted: {}", landing.quotation);
    if let Some(covered) = landing.note.covered.as_deref() {
        println!("      struck: {covered}");
    }
}

/// A counter map as `key=count` pairs on one line.
fn joined(counts: &BTreeMap<&str, usize>) -> String {
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<String>>()
        .join(" ")
}
