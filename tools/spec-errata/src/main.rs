//! Emits the specifications' annotations, and reports the quotations that land on retired text.
//!
//! ```sh
//! # what the conversion dropped, keyed to page and clause
//! cargo run --release -p spec-errata -- emit doc/*.pdf > doc/errata.md
//! # the count, which is the part that may be written down
//! cargo run --release -p spec-errata -- census doc/*.pdf
//! # the hazard: rustdoc blockquotes that quote a sentence an erratum struck out
//! cargo run --release -p spec-errata -- check doc/*.pdf
//! # the other direction: a clause number an erratum moves, and what stands on it here
//! cargo run --release -p spec-errata -- moved doc/*.pdf
//! # the same for a *table*, which `moved`'s predicate cannot reach
//! cargo run --release -p spec-errata -- renumbered doc/*.pdf
//! # the third: a place that records an erratum and quotes the words it removed
//! cargo run --release -p spec-errata -- applied doc/*.pdf
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

use spec_errata::renumbered::Rung;
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
    /// Errata that move a clause *number*, and what this tree has standing on each.
    Moved,
    /// Errata that renumber a *table*, and what this tree has standing on each.
    Renumbered,
    /// Places that name an erratum and quote the text it removed.
    Applied,
}

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let command = match arguments.next().as_deref() {
        Some("emit") => Command::Emit,
        Some("census") => Command::Census,
        Some("check") => Command::Check,
        Some("moved") => Command::Moved,
        Some("renumbered") => Command::Renumbered,
        Some("applied") => Command::Applied,
        _ => {
            println!(
                "usage: spec-errata <emit|census|check|moved|renumbered|applied> <document.pdf>..."
            );
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
        Command::Moved => {
            if let Err(error) = moved(&notes) {
                println!("{error}");
                return std::process::ExitCode::FAILURE;
            }
        }
        Command::Renumbered => {
            if let Err(error) = renumbered(&notes) {
                println!("{error}");
                return std::process::ExitCode::FAILURE;
            }
        }
        Command::Applied => {
            if let Err(error) = applied(&notes) {
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
    landings.extend(spec_errata::document_landings(
        notes,
        &PathBuf::from("doc"),
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

/// The errata that move a clause number, and what this tree has standing on each.
///
/// **The other direction of `check`'s question**, and the reason it needs a command of its own: a
/// renumbering strikes a *heading*, so no quotation can land on it and `check` is blind to it by
/// construction. The five-hundred-and-sixty-second session filtered `emit`'s 1097 annotations for
/// these verbs by hand; this prints the same population with the ground each one moves counted
/// beside it, which is what a round writing a new citation needs to see.
///
/// # Errors
///
/// Whatever [`spec_errata::standing_on`] answered.
fn moved(notes: &[Note]) -> Result<(), spec_errata::Error> {
    let structural = spec_errata::structural(notes);
    println!(
        "{} of {} annotation(s) move, renumber, insert or delete a numbered clause.",
        structural.len(),
        notes.len()
    );
    for erratum in &structural {
        let ground = spec_errata::standing_on(
            &erratum.clauses,
            &PathBuf::from("doc/conformance/ledger.toml"),
            &[
                PathBuf::from("crates"),
                PathBuf::from("tools"),
                PathBuf::from("fuzz"),
            ],
            &PathBuf::from("doc"),
        )?;
        println!(
            "\n{} p.{} {} [{}] — {}",
            erratum.note.document,
            erratum.note.page,
            erratum.note.subject.as_deref().unwrap_or("(no /Subj)"),
            erratum
                .note
                .states
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>()
                .join(", "),
            erratum.verbs.join(", ")
        );
        println!(
            "  says: {}",
            erratum.note.contents.as_deref().unwrap_or("(no /Contents)")
        );
        for standing in &ground {
            println!(
                "  §{} — {} ledger row(s), {} source citation(s), {} in this project's documents",
                standing.clause,
                standing.rows.len(),
                standing.citations.len(),
                standing.documents.len()
            );
            for row in &standing.rows {
                println!("      row §{row}");
            }
            for (file, line) in &standing.citations {
                println!("      {}:{}", file.display(), line);
            }
        }
    }
    println!(
        "\nThe numbers are not changed anywhere: `doc/md/` is the published text and the citation \
         gate resolves against it, so a post-erratum number is refused outright. What this answers \
         is which published numbers stand on ground an erratum has moved, and how much of this tree \
         stands with them — `doc/errata-read.md` carries the reading."
    );
    Ok(())
}

/// The errata that renumber a table, and what this tree has standing on each.
///
/// **The question `moved` cannot ask.** That command's predicate is one of four verbs in the
/// annotation's own `/Contents` and a *clause* number named there; a renumbering written as a
/// bare strike over a caption's designation and a caret carrying the replacement has neither, and
/// `check` cannot see it either — the struck text is under the four-word floor, and no quotation
/// lands on a caption. [`spec_errata::renumbered`] has the whole argument.
///
/// # Errors
///
/// Whatever [`spec_errata::renumbered::renumbered`] or
/// [`spec_errata::renumbered::standing_on`] answered.
fn renumbered(notes: &[Note]) -> Result<(), spec_errata::Error> {
    let renumberings = spec_errata::renumbered::renumbered(notes, &PathBuf::from("doc/md"))?;
    let ground = spec_errata::renumbered::standing_on(
        &renumberings,
        &PathBuf::from("doc/conformance/ledger.toml"),
        &[
            PathBuf::from("crates"),
            PathBuf::from("tools"),
            PathBuf::from("fuzz"),
        ],
        &PathBuf::from("doc"),
    )?;
    let near = ground
        .iter()
        .filter(|standing| standing.renumbering.rung == Rung::InSection)
        .count();
    println!(
        "{} of {} annotation(s) strike a designation the conversion captions and pair it with \
         another; {} strike it inside the clause that captions that very table, which is the \
         rung to read.",
        ground.len(),
        notes.len(),
        near
    );
    for standing in &ground {
        let erratum = &standing.renumbering;
        let heading = format!(
            "{} p.{} {} [{}] — Table {} becomes Table {}",
            erratum.note.document,
            erratum.note.page,
            erratum.note.subject.as_deref().unwrap_or("(no /Subj)"),
            erratum
                .note
                .states
                .iter()
                .map(String::as_str)
                .collect::<Vec<&str>>()
                .join(", "),
            erratum.retired,
            erratum.replacement
        );
        if erratum.rung == Rung::Elsewhere {
            // Counted and named rather than expanded: the far rung is where an integer struck in
            // body text lands, and printing its ground would bury the near rung under the
            // hundreds of lines a two-digit table number reaches.
            println!(
                "\n  elsewhere: {heading} — filed under {}, {} place(s)",
                erratum.note.section.as_deref().unwrap_or("(no section)"),
                standing.places()
            );
            continue;
        }
        println!("\n{heading}");
        println!("  captioned: Table {} — {}", erratum.retired, erratum.title);
        println!(
            "  {} source citation(s), {} in this project's documents and the ledger",
            standing.citations.len(),
            standing.documents.len()
        );
        for (file, line) in standing.citations.iter().chain(&standing.documents) {
            println!("      {}:{}", file.display(), line);
        }
    }
    println!(
        "\nThe designations are not changed anywhere, for `moved`'s own reason: `doc/md/` is the \
         published text every citation resolves against, so a tree citing a post-erratum caption \
         would cite one no reader can find. What this answers is which published table numbers \
         stand on ground an erratum has moved, and how much of this tree stands with them — \
         `doc/errata-read.md` carries the reading."
    );
    Ok(())
}

/// Places that name an erratum and quote the words it removed.
///
/// **The question `check` cannot ask, from the third direction.** `check` asks whether a
/// quotation lands on struck text and cannot see whether the writer had read the erratum;
/// `moved` asks whether an erratum shifts ground this tree stands on. This one asks whether a
/// place that *records* an erratum has **applied** it — the five-hundred-and-ninetieth session's
/// finding, whose lesson is that a row recording an erratum is not a row that has applied it.
///
/// The rungs are printed in the order they are worth reading: a live quotation of retired text,
/// then a correction quoting the wording it retired, then this project's own record of the
/// reading, which is that shape from end to end.
///
/// # Errors
///
/// Whatever [`spec_errata::applied::ledger_places`], [`spec_errata::applied::source_places`] or
/// [`spec_errata::applied::document_places`] answered.
fn applied(notes: &[Note]) -> Result<(), spec_errata::Error> {
    let errata = spec_errata::applied::errata(notes);
    let mut places =
        spec_errata::applied::ledger_places(&PathBuf::from("doc/conformance/ledger.toml"))?;
    places.extend(spec_errata::applied::source_places(&[
        PathBuf::from("crates"),
        PathBuf::from("tools"),
        PathBuf::from("fuzz"),
    ])?);
    places.extend(spec_errata::applied::document_places(&PathBuf::from(
        "doc",
    ))?);
    let report = spec_errata::applied::sweep(&errata, &places);

    println!(
        "{} change(s) over {} issue number(s); {} strike out {} words or more and can be \
         compared against.",
        report.errata,
        report.issues,
        report.comparable,
        spec_errata::MIN_WORDS
    );
    println!(
        "{} place(s) read — a ledger note, a comment run or a Markdown block — of which {} name \
         an erratum this collection carries; {} `#NNN` token(s) name none and are dropped.",
        report.places, report.citing, report.unknown
    );
    println!(
        "{} quotation-against-erratum comparison(s): {} quote the erratum's own replacement, {} \
         match both sides, {} quote what it struck out.",
        report.compared,
        report.applied,
        report.both,
        report.hits.len()
    );
    println!(
        "\n{} of those carry no mark of a correction and are the list to read first; {} read like \
         a correction quoting the wording it retired; {} are in {}, which is the reading itself.",
        report.unmarked(),
        report
            .hits
            .len()
            .saturating_sub(report.unmarked())
            .saturating_sub(report.reading()),
        report.reading(),
        spec_errata::applied::THE_READING
    );
    for hit in &report.hits {
        let marks = match (hit.reading, hit.history) {
            (true, _) => " [the reading]",
            (false, true) => " [history]",
            (false, false) => "",
        };
        println!(
            "\n  [{}] {} — Issue #{} — {}{marks}",
            hit.kind.as_str(),
            hit.location,
            hit.issue,
            hit.erratum
        );
        println!("      quoted: {}", hit.quotation);
        println!("      struck: {}", hit.struck);
        if hit.replacement.is_empty() {
            println!("      the erratum states no replacement: it deletes.");
        } else {
            for replacement in &hit.replacement {
                println!("      instead: {replacement}");
            }
        }
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
