//! Every PDF/A erratum the held snapshot of `pdf-association/pdf-issues` carries has a verdict.
//!
//! # Why this is a gate
//!
//! `CLAUDE.md` principle 5 makes an erratum the working group agreed the standard, and the printed
//! text retired (ADR 3581). That rule is only as good as the reading of each erratum: a correction
//! nobody here has read is one the validator silently does not apply, and #77's supplement
//! comparison stood two years in that state while this tree filed it upstream as another
//! implementation's error (ADR 3582). `doc/errata/` holds the tracker, and the post-merge train
//! re-fetches it daily (ADR 3583), so a new issue or a new resolution arrives as a changed file.
//! This test makes the arrival a failure until somebody has read it: every issue the snapshot
//! holds about ISO 19005 has one row in `doc/errata-read-pdfa.md` with one of the three verdicts,
//! and every row there names an issue the snapshot holds (ADR 3590).
//!
//! # Which issues
//!
//! The rule `doc/errata/pdfa.md` was written by: an issue carrying a label that begins `PDF/A-`,
//! or naming `19005` or `PDF/A` in its title — plus every issue the site's ISO 19005 pages cite in
//! a `data-issue`, so that a published erratum is held to a verdict even if its labels move.
//!
//! # What it does not judge
//!
//! Whether a verdict is *right*. It checks that one was given, in the vocabulary, with a reason,
//! and that an `applied` verdict points at the code or record that applies it; reading the
//! thread is the round's work.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read the repository's own held snapshot has not found \
              a defect, and reporting that as one would be worse than stopping"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The verdicts, as `doc/errata-read-pdfa.md` writes them.
const VERDICTS: [&str; 3] = ["applied", "not applicable", "recorded disagreement"];

fn repository_root() -> &'static Path {
    // `CARGO_MANIFEST_DIR` is `<root>/tools/conformance`, so two levels up is the root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// The value of a top-level JSON string field in one record of `issues.jsonl`, unescaped as far
/// as a title needs.
///
/// The records are written by `tools/fetch-errata.sh` with fixed fields and sorted keys, one a
/// line, so a field's key appears unescaped exactly once: inside a string value every `"` is
/// written `\"`, and a `"key":` cannot occur there.
fn string_field(record: &str, key: &str) -> Option<String> {
    let start = record.find(&format!("\"{key}\":\""))?;
    let mut value = String::new();
    let mut characters = record[start.saturating_add(key.len()).saturating_add(4)..].chars();
    while let Some(character) = characters.next() {
        match character {
            '"' => return Some(value),
            '\\' => value.push(characters.next()?),
            other => value.push(other),
        }
    }
    None
}

fn number_field(record: &str) -> Option<u32> {
    let start = record.find("\"number\":")?;
    record[start.saturating_add(9)..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

fn labels(record: &str) -> Vec<String> {
    let Some(start) = record.find("\"labels\":[") else {
        return Vec::new();
    };
    let rest = &record[start.saturating_add(10)..];
    let end = rest.find(']').unwrap_or(rest.len());
    rest[..end]
        .split(',')
        .map(|label| label.trim().trim_matches('"').to_owned())
        .filter(|label| !label.is_empty())
        .collect()
}

/// The issues the snapshot holds about ISO 19005, by the rule the module states.
fn pdfa_issues(root: &Path) -> BTreeSet<u32> {
    let issues = std::fs::read_to_string(root.join("doc/errata/issues.jsonl"))
        .expect("doc/errata/issues.jsonl is held in the repository");
    let mut found = BTreeSet::new();
    for record in issues.lines().filter(|line| !line.trim().is_empty()) {
        let number = number_field(record).expect("every record states its number");
        let title = string_field(record, "title").unwrap_or_default();
        if labels(record)
            .iter()
            .any(|label| label.starts_with("PDF/A-"))
            || title.contains("19005")
            || title.contains("PDF/A")
        {
            found.insert(number);
        }
    }
    let pages = root.join("doc/errata/site/docs");
    for entry in std::fs::read_dir(&pages).expect("the site's source is held") {
        let directory = entry.expect("a directory entry reads").path();
        if !directory
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("19005-"))
        {
            continue;
        }
        for page in std::fs::read_dir(&directory).expect("a standard's pages read") {
            let page = page.expect("a directory entry reads").path();
            let text = std::fs::read_to_string(&page).expect("a page reads");
            for tail in text.split("data-issue=\"").skip(1) {
                let list = tail.split('"').next().unwrap_or_default();
                found.extend(
                    list.split(',')
                        .filter_map(|issue| issue.trim().parse::<u32>().ok()),
                );
            }
        }
    }
    found
}

/// One row of the verdict table: the issue, the verdict, the reason.
fn verdict_rows(path: &PathBuf) -> Vec<(u32, String, String)> {
    let text = std::fs::read_to_string(path).expect("doc/errata-read-pdfa.md is in the tree");
    text.lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // `| #77 | applied | … |` — a leading empty cell, then the issue.
            let issue = cells.get(1)?.strip_prefix('#')?.parse().ok()?;
            Some((
                issue,
                (*cells.get(2)?).to_owned(),
                cells.get(3..cells.len().saturating_sub(1))?.join(" | "),
            ))
        })
        .collect()
}

#[test]
fn every_pdfa_erratum_in_the_held_snapshot_has_a_verdict() {
    let root = repository_root();
    let issues = pdfa_issues(root);
    assert!(
        issues.len() >= 34,
        "the rule found {} issues, fewer than doc/errata/pdfa.md listed — the parser is wrong",
        issues.len()
    );
    let rows = verdict_rows(&root.join("doc/errata-read-pdfa.md"));
    let mut by_issue: BTreeMap<u32, Vec<(&str, &str)>> = BTreeMap::new();
    for (issue, verdict, reason) in &rows {
        by_issue
            .entry(*issue)
            .or_default()
            .push((verdict.as_str(), reason.as_str()));
    }
    let mut failures = Vec::new();
    for issue in &issues {
        match by_issue.get(issue).map(Vec::as_slice) {
            None => failures.push(format!(
                "#{issue} is a PDF/A issue in doc/errata/ and has no verdict in \
                 doc/errata-read-pdfa.md: read its thread in comments.jsonl and give one"
            )),
            Some([(verdict, reason)]) => {
                if !VERDICTS.contains(verdict) {
                    failures.push(format!(
                        "#{issue}'s verdict `{verdict}` is not one of {VERDICTS:?}"
                    ));
                }
                if reason.split_whitespace().count() < 4 {
                    failures.push(format!("#{issue}'s verdict states no reason"));
                }
                if *verdict == "applied" && !(reason.contains("ADR ") || reason.contains("crates/"))
                {
                    failures.push(format!(
                        "#{issue} is `applied` and names neither the ADR nor the code that \
                         applies it"
                    ));
                }
            }
            Some(more) => failures.push(format!("#{issue} has {} verdicts", more.len())),
        }
    }
    for issue in by_issue.keys() {
        if !issues.contains(issue) {
            failures.push(format!(
                "#{issue} has a verdict and is not a PDF/A issue of the held snapshot"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_field_readers_read_the_records_shape() {
    let record = r#"{"body":"a \"title\":\"x\" in a body","labels":["PDF/A-4","ISO approved"],"number":123,"title":"6.7.3 \"pdfaid\" prefix"}"#;
    assert_eq!(number_field(record), Some(123));
    assert_eq!(labels(record), vec!["PDF/A-4", "ISO approved"]);
    assert_eq!(
        string_field(record, "title").as_deref(),
        Some("6.7.3 \"pdfaid\" prefix")
    );
}
