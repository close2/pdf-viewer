//! The second sweep: every entry a note claims is unread, grepped against the tree.
//!
//! # The shape it exists for
//!
//! A ledger note that lists entries as unread — "Not read: `/I`, `/RI`, `/IX`…", "`/Usage` is
//! read by nothing" — is one claim per entry, and the claims decay independently: the session
//! that starts reading an entry is implementing some *other* clause and has no reason to come
//! back. Six of the first ten lists checked had a live entry (`doc/todo/01`, session 122);
//! §7.7.3.3's had eleven of eighteen; §8.11.2.1 recorded `/Usage` as unread for a hundred and
//! fifty sessions after §8.11.4.4 read it per group.
//!
//! So the sweep takes each such claim apart into keys and asks the tree: **does any Rust source
//! under [`crate::SOURCE_ROOTS`] read the entry it says nobody reads?** Reading, here, is the
//! quoted-string form a lookup uses — `document.get_key(annotation, "FS")` — because a `/FS` in
//! a comment is *naming* the entry, which is exactly what the note already does. A key quoted by
//! a file the row's own `code = [...]` lists is the sharpest hit there is: the row's implementing
//! file reads the entry the row calls unread.
//!
//! # What a hit is and is not
//!
//! A hit is a reading list, not a verdict, and the noise has one dominant shape this project has
//! met five times in one run: **one short key, three clauses**. §8.4.5's `/BG` is Table 57's
//! black generation while `appearance.rs`'s `"BG"` is Table 232's widget background; `/TR` is a
//! device transfer function in one clause and a soft-mask transfer in another. The witness paths
//! are printed so that a reader can tell in one look whether the quoting file is about this
//! clause at all. And a claim confirmed — a key quoted nowhere — is a result too: it says the
//! population has not drifted, which is the only way it is watched.
//!
//! # Why it is a program now
//!
//! It was one of `doc/todo/01`'s prose sweeps from the hundred-and-twenty-second session to the
//! four-hundred-and-eighty-ninth, re-derived from its own paragraph on every run — which is the
//! failure mode ADR 0319 records for the fifteenth sweep, and `CLAUDE.md`'s "write down the
//! command, not the answer" failing in the direction it was written for. A description is
//! rebuilt differently every time; a program is the same instrument twice. ADR 0324.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument: the known false-positive population does not shrink under a
//! tighter grep, because the same short key legitimately belongs to several clauses. It runs in
//! seconds and its output is read, like every sweep in `doc/todo/01`.

use std::path::PathBuf;

use crate::entries::Named;
use crate::ledger::{Ledger, Row, Status};

/// The phrases that make a sentence a claim of unreadness.
///
/// Lower-cased substrings, matched against one sentence of a note. `doc/todo/01`'s own lesson —
/// found on §8.11.2.1, which said "read by nothing" where the sweep's grep knew only
/// "Not read:" — is to grep the shape rather than the wording, so the list holds every way this
/// ledger has written the claim. "read by no" covers both "read by nothing" and "read by no
/// host".
pub const CLAIMS: [&str; 8] = [
    "not read",
    "unread",
    "read by no",
    "read nowhere",
    "nobody reads",
    "reads nothing",
    "never read",
    "none of which is read",
];

/// One entry a note claims is unread, and what the tree says about that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claimed {
    /// The key itself, without the solidus.
    pub key: String,
    /// Where the tree quotes it as a lookup string.
    pub named: Named,
    /// Up to [`WITNESSES`] of the files that quote it, the row's own first.
    pub witnesses: Vec<String>,
}

/// How many quoting files a claimed entry reports.
///
/// Enough to see whether the quoting file is about this clause at all — the one-short-key
/// false positive is settled by the path, not by the count.
pub const WITNESSES: usize = 3;

/// One ledger row with at least one unread-claimed entry the tree quotes.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The clause whose row it is.
    pub clause: crate::clause::ClauseNumber,
    /// The row's status, because an `implemented` row claiming an entry unread is still a claim.
    pub status: Status,
    /// The 1-based line of `ledger.toml` the row starts on.
    pub line: usize,
    /// The claimed entries some source quotes, in the order the note claims them.
    pub entries: Vec<Claimed>,
}

/// What one run of the sweep read and what it found.
#[derive(Debug, Clone)]
pub struct Report {
    /// How many rows claim at least one entry unread.
    pub population: usize,
    /// How many distinct keys those rows claim, over all of them.
    pub claimed: usize,
    /// How many claimed keys no source quotes — the claims the run confirms.
    pub confirmed: usize,
    /// The rows with at least one claimed entry the tree quotes.
    pub findings: Vec<Finding>,
}

impl Report {
    /// How many claimed entries the run reports as quoted somewhere.
    #[must_use]
    pub fn live(&self) -> usize {
        self.findings
            .iter()
            .map(|finding| finding.entries.len())
            .sum()
    }

    /// How many of the reported entries are quoted by the row's own `code` files.
    ///
    /// The shortest reading list this sweep produces: the file the row says implements the
    /// clause reads the entry the same row says nobody reads.
    #[must_use]
    pub fn by_own_code(&self) -> usize {
        self.findings
            .iter()
            .flat_map(|finding| finding.entries.iter())
            .filter(|entry| entry.named == Named::ByItsOwnCode)
            .count()
    }
}

/// Runs the sweep over one ledger and one set of sources.
///
/// The sources are [`crate::entries::sources`]' — every Rust file under
/// [`crate::SOURCE_ROOTS`], tests included, because the population the prose sweep walked was
/// the whole tree and a witness path says what kind of file quotes the key.
#[must_use]
pub fn sweep(ledger: &Ledger, sources: &[(PathBuf, String)]) -> Report {
    let mut report = Report {
        population: 0,
        claimed: 0,
        confirmed: 0,
        findings: Vec::new(),
    };
    for row in &ledger.rows {
        let Some(note) = row.note.as_deref() else {
            continue;
        };
        let keys = claimed_keys(note);
        if keys.is_empty() {
            continue;
        }
        report.population = report.population.saturating_add(1);
        report.claimed = report.claimed.saturating_add(keys.len());
        let mut entries = Vec::new();
        for key in keys {
            let claimed = quoted_by(&key, row, sources);
            if claimed.named == Named::Nowhere {
                report.confirmed = report.confirmed.saturating_add(1);
            } else {
                entries.push(claimed);
            }
        }
        if !entries.is_empty() {
            report.findings.push(Finding {
                clause: row.clause.clone(),
                status: row.status,
                line: row.line,
                entries,
            });
        }
    }
    report
}

/// The keys a note's own claim sentences name.
///
/// Sentence-scoped deliberately: a note is corrected by appending, so most notes hold both the
/// claim and its history, and a key three sentences from the claim is usually the subject of a
/// different statement — §12.7.5.3's `/MaxLen` sat in a sentence *about* something else and was
/// this sweep's second known false-positive shape when the grep was run by hand over whole
/// notes.
#[must_use]
pub fn claimed_keys(note: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for sentence in sentences(note) {
        if !is_a_claim(sentence) {
            continue;
        }
        for key in keys_in(sentence) {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    keys
}

/// Whether one sentence claims an entry is unread.
#[must_use]
pub fn is_a_claim(sentence: &str) -> bool {
    let sentence = sentence.to_ascii_lowercase();
    CLAIMS.iter().any(|phrase| sentence.contains(phrase))
}

/// A note's sentences, split where a full stop ends one.
///
/// A full stop followed by whitespace or the end ends a sentence; one inside `§12.5.6.4` or
/// `1.7` is followed by neither and does not. An abbreviation's stop splits a sentence early,
/// which costs a key that sits after it in the same sentence — the cheap direction, since a key
/// dropped from the list is a claim left for the by-hand run, not a false hit.
///
/// Shared with [`crate::blockers`], whose claims are sentence-scoped for the same reason.
pub(crate) fn sentences(note: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (index, _) in note.match_indices('.') {
        let followed = note[index.saturating_add(1)..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace);
        if followed && index >= start {
            let sentence = note[start..=index].trim();
            if !sentence.is_empty() {
                out.push(sentence);
            }
            start = index.saturating_add(1);
        }
    }
    let rest = note.get(start..).unwrap_or("").trim();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// The `/Key` tokens of one sentence.
///
/// A key here is a SOLIDUS followed by an ASCII upper-case letter and alphanumerics — the form
/// every entry this ledger has claimed unread takes. The upper-case requirement is what keeps a
/// path out of the list: `crates/pdf-model/src/view.rs` would otherwise contribute `pdf`, `src`
/// and `view`. Its cost is a lower-case key like Table 166's `/ca`, which no claim so far has
/// listed and which the by-hand run still covers.
fn keys_in(sentence: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for (index, character) in sentence.char_indices() {
        if character != '/' {
            continue;
        }
        let rest = &sentence[index.saturating_add(1)..];
        let mut end = 0usize;
        for (offset, candidate) in rest.char_indices() {
            if candidate.is_ascii_alphanumeric() {
                end = offset.saturating_add(candidate.len_utf8());
            } else {
                break;
            }
        }
        let key = &rest[..end];
        if key.starts_with(|first: char| first.is_ascii_uppercase())
            && !keys.iter().any(|seen| seen == key)
        {
            keys.push(key.to_owned());
        }
    }
    keys
}

/// Where the tree quotes one key, given the row that claims it unread.
fn quoted_by(key: &str, row: &Row, sources: &[(PathBuf, String)]) -> Claimed {
    let needle = format!("\"{key}\"");
    let mut named = Named::Nowhere;
    let mut witnesses = Vec::new();
    for (path, text) in sources {
        if !text.contains(needle.as_str()) {
            continue;
        }
        let shown = path.to_string_lossy().replace('\\', "/");
        if row
            .code
            .iter()
            .any(|listed| crate::entries::covered_by(listed, &shown))
        {
            named = Named::ByItsOwnCode;
            witnesses.insert(0, shown);
        } else {
            if named == Named::Nowhere {
                named = Named::Elsewhere;
            }
            witnesses.push(shown);
        }
    }
    witnesses.truncate(WITNESSES);
    Claimed {
        key: key.to_owned(),
        named,
        witnesses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Status;

    fn row_with(note: &str, code: &[&str]) -> Row {
        Row {
            clause: "8.11.2.1".parse().expect("a clause number"),
            title: "Optional content groups".to_owned(),
            status: Status::Partial,
            code: code.iter().map(|&path| path.to_owned()).collect(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    /// §8.11.2.1's own wording — "read by nothing" rather than "Not read:" — is the miss that
    /// taught this sweep to grep the shape, and every spelling the ledger has used is a claim.
    #[test]
    fn every_spelling_of_the_claim_is_one() {
        assert!(is_a_claim("Two Table 96 entries are read by nothing"));
        assert!(is_a_claim("Not read: /I, /RI and /IX."));
        assert!(is_a_claim("/Usage is unread"));
        assert!(is_a_claim("an entry nobody reads"));
        assert!(!is_a_claim(
            "Both entries are read since the thirty-fifth session."
        ));
    }

    /// The claim's own sentence supplies the keys; a key elsewhere in the note is the subject of
    /// a different statement — the false-positive shape the by-hand runs met on §12.7.5.3.
    #[test]
    fn only_the_claim_sentences_keys_are_claimed() {
        let keys = claimed_keys(
            "Table 96's /Intent is honoured since §8.11.2.3 landed. Not read: /Name and /Usage. \
             §12.5.6.4 is unrelated.",
        );
        assert_eq!(keys, ["Name", "Usage"]);
    }

    /// A full stop inside a clause number is not a sentence boundary, so a claim after one keeps
    /// its keys.
    #[test]
    fn a_clause_number_does_not_end_a_sentence() {
        let keys = claimed_keys("Under §8.11.4.4, /Usage is read by nothing.");
        assert_eq!(keys, ["Usage"]);
    }

    /// The sharpest hit: the row's own implementing file quotes the key the row calls unread.
    #[test]
    fn a_key_quoted_by_the_rows_own_code_is_the_first_thing_to_read() {
        let sources = vec![
            (
                PathBuf::from("crates/pdf-model/src/optional_content.rs"),
                "let usage = document.get_key(group, \"Usage\");".to_owned(),
            ),
            (
                PathBuf::from("crates/viewer-ui/src/chrome.rs"),
                "let name = document.get_key(group, \"Name\");".to_owned(),
            ),
        ];
        let row = row_with(
            "Two Table 96 entries are read by nothing: /Name and /Usage.",
            &["crates/pdf-model/src/optional_content.rs"],
        );
        let ledger = Ledger { rows: vec![row] };
        let report = sweep(&ledger, &sources);
        assert_eq!(report.population, 1);
        assert_eq!(report.claimed, 2);
        assert_eq!(report.confirmed, 0);
        assert_eq!(report.live(), 2);
        assert_eq!(report.by_own_code(), 1);
        let finding = report.findings.first().expect("one finding");
        let usage = finding
            .entries
            .iter()
            .find(|entry| entry.key == "Usage")
            .expect("the /Usage hit");
        assert_eq!(usage.named, Named::ByItsOwnCode);
        assert_eq!(
            usage.witnesses.first().map(String::as_str),
            Some("crates/pdf-model/src/optional_content.rs")
        );
    }

    /// A key no source quotes confirms the claim, which is a result rather than a silence: it is
    /// counted, and the row is not a finding.
    #[test]
    fn a_claim_no_source_contradicts_is_confirmed() {
        let sources = vec![(
            PathBuf::from("crates/pdf-model/src/annotation.rs"),
            "// nothing about ink lists".to_owned(),
        )];
        let ledger = Ledger {
            rows: vec![row_with("Not read: /IX.", &[])],
        };
        let report = sweep(&ledger, &sources);
        assert_eq!(report.population, 1);
        assert_eq!(report.claimed, 1);
        assert_eq!(report.confirmed, 1);
        assert!(report.findings.is_empty());
    }
}
