//! What a record or an ADR relies on is in the tree, and this is what reads where it points.
//!
//! # The rule
//!
//! ADR 2242 decision 4: what a claim rests on is in the tree, or is reproduced by a command in the
//! tree. A path under `/home/AI/` or `scratchpad/` is a pointer for the session that wrote it and
//! never the evidence — the owner cannot read the one (`doc/environment.md`, "The machine, the
//! account and the display"), and the other is gone by the next batch. Review 1704 section 1.5
//! counted 63 lines of one span naming `/home/AI/`, and every relative `scratchpad/r16xx/` path its
//! ADRs cited had already gone.
//!
//! **A path may still be named as where a session worked**, and the check tells the two uses apart
//! by the sentence's own words, so a reader can too: a sentence that names such a path is a
//! *workplace* sentence when it says the session "worked in" or "worked under" it and every such
//! path in it is a directory, written with its closing `/`. Any other sentence naming one — "the
//! figures are in …", "compared by page against …", "the receipt is …" — names evidence, and is
//! refused: the figure, the comparison or the receipt goes into the tree, or the command in the
//! tree that reproduces it goes where the path was. A file is never a workplace, because what a
//! sentence names a file for is what the file holds.
//!
//! # Why the bound starts at a number
//!
//! **Because a record may not be rewritten.** `CLAUDE.md` ("Where knowledge lives") keeps
//! `doc/history/`, `doc/adr/` and `doc/reviews/` as records, so a check whose only remedy is an edit
//! holds only what was written after the check existed: the records and reviews of batch one hundred
//! and ten on, numbered by session from [`SESSIONS_FROM`], and its ADRs from [`DECISIONS_FROM`].
//! A batch commit's message is under the same rule and is not in the tree for a test to read
//! (`doc/todo/02` section 8 step 4 carries it).

#![expect(
    clippy::print_stdout,
    reason = "the gate prints the population it counted, which is what makes its verdict readable"
)]

use std::fmt::Write as _;

/// The first session whose record or review this check holds, batch one hundred and ten's first.
const SESSIONS_FROM: u32 = 1712;

/// The first ADR this check holds, batch one hundred and ten's first number.
const DECISIONS_FROM: u32 = 2258;

/// The directories read, each with the first number it holds.
const POPULATION: [(&str, u32); 3] = [
    ("doc/history", SESSIONS_FROM),
    ("doc/reviews", SESSIONS_FROM),
    ("doc/adr", DECISIONS_FROM),
];

/// What marks a path that is not in the tree.
const OUTSIDE: [&str; 2] = ["/home/AI/", "scratchpad/"];

/// The words that make a sentence say where a session worked.
const WORKED: [&str; 2] = ["worked in ", "worked under "];

/// One sentence's use of the paths outside the tree that it names.
#[derive(Debug, PartialEq, Eq)]
enum Use {
    /// It names none.
    None,
    /// It says the session worked in them, and each is a directory.
    Workplace,
    /// It names them for what they hold.
    Evidence(String),
}

/// The paths outside the tree a sentence names, as written, with the quoting around them removed.
///
/// A marker on its own — `scratchpad/`, `/home/AI/` — names the kind of place and points at
/// nothing, which is how this rule itself is written down, so it is not one of them.
fn outside_paths(sentence: &str) -> Vec<String> {
    sentence
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| "`'\"()[]{},.;:*\u{201c}\u{201d}".contains(c))
                .to_owned()
        })
        .filter(|word| {
            OUTSIDE
                .iter()
                .any(|marker| word.contains(marker) && word != marker)
        })
        .collect()
}

/// What one sentence does with the paths outside the tree it names.
fn use_of(sentence: &str) -> Use {
    let paths = outside_paths(sentence);
    let Some(first) = paths.first() else {
        return Use::None;
    };
    let lowered = sentence.to_lowercase();
    let says_worked = WORKED.iter().any(|words| lowered.contains(words));
    if says_worked {
        if let Some(file) = paths.iter().find(|path| !path.ends_with('/')) {
            return Use::Evidence(file.clone());
        }
        return Use::Workplace;
    }
    Use::Evidence(first.clone())
}

/// A document's sentences, each with the line it starts on.
///
/// A sentence ends at a full stop followed by white space, at a blank line, or where a list item,
/// a table row or a heading begins, so that two items are never read as one sentence.
fn sentences(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut current = String::new();
    let mut starts = 1usize;
    for (at, line) in text.lines().enumerate() {
        let number = at.saturating_add(1);
        let trimmed = line.trim_start();
        let begins_a_unit = trimmed.is_empty()
            || trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with('|')
            || trimmed.starts_with('#');
        if begins_a_unit && !current.trim().is_empty() {
            found.push((starts, std::mem::take(&mut current)));
        }
        if current.trim().is_empty() {
            current.clear();
            starts = number;
        }
        for piece in line.split_inclusive(". ") {
            current.push_str(piece);
            if piece.ends_with(". ") {
                found.push((starts, std::mem::take(&mut current)));
                starts = number;
            }
        }
        current.push(' ');
        if current.trim_end().ends_with('.') {
            found.push((starts, std::mem::take(&mut current)));
        }
    }
    if !current.trim().is_empty() {
        found.push((starts, current));
    }
    found
}

/// Every document in the population, as its directory, its file name and its text.
#[expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read the records has not found a defect, and \
              reporting that as one would be worse than stopping"
)]
fn population() -> (usize, Vec<(&'static str, String, String)>) {
    let root = conformance::workspace_root();
    let mut walked = 0usize;
    let mut found = Vec::new();
    for (directory, from) in POPULATION {
        for entry in std::fs::read_dir(root.join(directory)).expect("a record directory") {
            let path = entry.expect("a record").path();
            if !path.is_file() || path.extension().is_none_or(|suffix| suffix != "md") {
                continue;
            }
            walked = walked.saturating_add(1);
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let Some(number) = name
                .split('-')
                .next()
                .and_then(|digits| digits.parse::<u32>().ok())
            else {
                continue;
            };
            if number < from {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("a record's text");
            found.push((directory, name, text));
        }
    }
    found.sort_unstable();
    (walked, found)
}

/// No record, review or ADR from this batch on names a path outside the tree as evidence.
#[test]
fn no_record_since_the_rule_rests_on_a_path_outside_the_tree() {
    let (walked, documents) = population();
    let mut refused = String::new();
    let mut workplaces = 0usize;
    for (directory, name, text) in &documents {
        for (line, sentence) in sentences(text) {
            match use_of(&sentence) {
                Use::None => {}
                Use::Workplace => workplaces = workplaces.saturating_add(1),
                Use::Evidence(path) => {
                    let _ = writeln!(
                        refused,
                        "{directory}/{name}:{line}: `{path}` is named for what it holds. Put the \
                         figure, the comparison or the receipt in the tree, or the command in the \
                         tree that reproduces it; a path outside it is named only as where the \
                         session \"worked in\", a directory with its closing `/` (ADR 2242 \
                         decision 4, ADR 2260)."
                    );
                }
            }
        }
    }
    println!(
        "{} records, reviews and ADRs from session {SESSIONS_FROM} and ADR {DECISIONS_FROM} on; \
         {workplaces} sentences name where a session worked",
        documents.len()
    );
    assert!(
        walked > 1000,
        "only {walked} documents walked: the population is not being reached"
    );
    assert!(refused.is_empty(), "\n{refused}");
}

/// The check, against a planted sentence of each use (trap 13).
#[test]
fn the_evidence_check_is_calibrated_against_planted_sentences() {
    let planted = [
        ("The round drew three pages and moved no row.", Use::None),
        (
            "A path under `/home/AI/` or `scratchpad/` is never the evidence.",
            Use::None,
        ),
        (
            "The session worked in `scratchpad/r9999/`, and nothing there is evidence.",
            Use::Workplace,
        ),
        (
            "Its export worked under `/home/AI/pdf-viewer-rounds/scratchpad/r9999/export/`.",
            Use::Workplace,
        ),
        (
            "The twelve figures are in `scratchpad/r9999/costs.txt`.",
            Use::Evidence("scratchpad/r9999/costs.txt".to_owned()),
        ),
        (
            "The six arms held by page against `/home/AI/arms-9999/`.",
            Use::Evidence("/home/AI/arms-9999/".to_owned()),
        ),
        (
            "The session worked in `scratchpad/r9999/` and the receipt is \
             `scratchpad/r9999/gates.log`.",
            Use::Evidence("scratchpad/r9999/gates.log".to_owned()),
        ),
        (
            "It worked in /home/AI/arms-9999/README.",
            Use::Evidence("/home/AI/arms-9999/README".to_owned()),
        ),
    ];
    for (sentence, expected) in planted {
        assert_eq!(use_of(sentence), expected, "planted sentence: {sentence}");
    }

    // A workplace in one list item does not license a path in the next, nor one sentence another.
    let text = "- the session worked in `scratchpad/r9999/`\n- the log is \
                `scratchpad/r9999/a.log`\n\nIt worked in `scratchpad/r9999/`. The figures are \
                in `/home/AI/x/`.\n";
    let uses: Vec<Use> = sentences(text)
        .into_iter()
        .map(|(_, sentence)| use_of(&sentence))
        .filter(|found| *found != Use::None)
        .collect();
    assert_eq!(
        uses,
        [
            Use::Workplace,
            Use::Evidence("scratchpad/r9999/a.log".to_owned()),
            Use::Workplace,
            Use::Evidence("/home/AI/x/".to_owned()),
        ]
    );
    let lines: Vec<usize> = sentences(text).into_iter().map(|(line, _)| line).collect();
    assert_eq!(lines.first(), Some(&1), "{lines:?}");
}
