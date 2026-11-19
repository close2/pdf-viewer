//! Every sentence a portable test quotes is in the clause the test names, as written.
//!
//! `portable-conformance/tests/*.toml` each carry a `clause` and a `quote` array: the sentences of
//! ISO 32000-2 the test's expected outcome rests on (RFC 0016 section 4, ADR 3206). The suite is
//! read by people who do not hold the standard, so a quotation that drifted from the text would
//! be the one claim in a manifest that no reader of it could check. This holds each against
//! `doc/md/` by the same comparison the source's blockquotes get (`ClauseIndex::holds_quotation`),
//! and holds each manifest's `id` to its file name and its clause, which is how the runner orders
//! its table.
//!
//! The manifests are read line by line rather than by a TOML parser: a `quote` array is one
//! double-quoted string per line by the suite's own format (`portable-conformance/README.md`), and
//! this crate's `toml_subset` reads the ledger's shape, not inline tables and multi-line strings.

#![expect(
    clippy::print_stdout,
    reason = "the gate prints how many manifests and quotations it read, so a run that read none \
              says so"
)]

use std::fmt::Write as _;
use std::path::Path;

use conformance::clause::{ClauseIndex, ClauseNumber};

/// One manifest's `id`, `clause` and quotations.
struct Manifest {
    id: String,
    clause: String,
    quotes: Vec<String>,
}

/// The string between the first and the last double quote of a line, with `\"` and `\\` undone.
fn string_of(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line.rfind('"')?;
    let inner = line.get(start.checked_add(1)?..end)?;
    Some(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn read(path: &Path) -> std::io::Result<Manifest> {
    let text = std::fs::read_to_string(path)?;
    let mut manifest = Manifest {
        id: String::new(),
        clause: String::new(),
        quotes: Vec::new(),
    };
    let mut in_quote = false;
    for line in text.lines() {
        if in_quote {
            if line.trim() == "]" {
                in_quote = false;
            } else if let Some(quote) = string_of(line) {
                manifest.quotes.push(quote);
            }
        } else if line.starts_with("id = ") {
            manifest.id = string_of(line).unwrap_or_default();
        } else if line.starts_with("clause = ") {
            manifest.clause = string_of(line).unwrap_or_default();
        } else if line.starts_with("quote = [") {
            in_quote = true;
        }
    }
    Ok(manifest)
}

/// The faults of one manifest against the standard's text.
fn faults(index: &ClauseIndex, path: &Path, manifest: &Manifest) -> String {
    let mut wrong = String::new();
    let site = path.display();
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    if manifest.id != stem {
        let _ = writeln!(wrong, "{site}: id {:?} is not the file's name", manifest.id);
    }
    if !manifest.id.starts_with(&format!("{}-", manifest.clause)) {
        let _ = writeln!(
            wrong,
            "{site}: id does not begin with its clause {}",
            manifest.clause
        );
    }
    let Ok(clause) = manifest.clause.parse::<ClauseNumber>() else {
        let _ = writeln!(
            wrong,
            "{site}: {:?} is not a clause number",
            manifest.clause
        );
        return wrong;
    };
    if !index.contains(&clause) {
        let _ = writeln!(wrong, "{site}: the standard has no clause {clause}");
        return wrong;
    }
    if manifest.quotes.is_empty() {
        let _ = writeln!(wrong, "{site}: quotes no sentence of its clause");
    }
    for quote in &manifest.quotes {
        if !index.holds_quotation(&clause, quote) {
            let _ = writeln!(wrong, "{site}: §{clause} does not contain\n    {quote}");
        }
    }
    wrong
}

#[test]
fn every_portable_quotation_is_in_its_clause_as_written() {
    let root = conformance::workspace_root();
    let index = ClauseIndex::read(&root.join(conformance::STANDARD)).expect("the standard");
    let mut paths: Vec<_> = std::fs::read_dir(root.join("portable-conformance/tests"))
        .expect("the suite's tests")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "the suite holds no manifest");
    let mut wrong = String::new();
    let mut quotations = 0usize;
    for path in &paths {
        let manifest = read(path).expect("a manifest");
        quotations = quotations.saturating_add(manifest.quotes.len());
        wrong.push_str(&faults(&index, path, &manifest));
    }
    println!("{} manifests, {quotations} quotations", paths.len());
    assert!(wrong.is_empty(), "{wrong}");
}

/// The comparison is calibrated against a sentence the clause does not hold and a clause that
/// does not hold the sentence (trap 13): both must be reported.
#[test]
fn a_changed_quotation_and_a_wrong_clause_are_both_reported() {
    let root = conformance::workspace_root();
    let index = ClauseIndex::read(&root.join(conformance::STANDARD)).expect("the standard");
    let path = Path::new("8.4.3.3-planted.toml");
    let sentence = "There shall be no projection beyond the end of the path.";
    let held = Manifest {
        id: "8.4.3.3-planted".to_owned(),
        clause: "8.4.3.3".to_owned(),
        quotes: vec![sentence.to_owned()],
    };
    assert!(
        faults(&index, path, &held).is_empty(),
        "the control is held"
    );
    let changed = Manifest {
        quotes: vec![sentence.replace("no projection", "a projection")],
        ..held
    };
    assert!(
        !faults(&index, path, &changed).is_empty(),
        "a changed word is found"
    );
    let elsewhere = Manifest {
        id: "8.4.3.2-planted".to_owned(),
        clause: "8.4.3.2".to_owned(),
        quotes: vec![sentence.to_owned()],
    };
    let reported = faults(&index, Path::new("8.4.3.2-planted.toml"), &elsewhere);
    assert!(
        reported.contains("does not contain"),
        "a sentence of another clause is found: {reported}"
    );
}
