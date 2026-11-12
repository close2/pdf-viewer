//! What a record or an ADR relies on is in the tree, and this is what reads where it points.
//!
//! # The rule
//!
//! ADR 2242 decision 4: what a claim rests on is in the tree, or is reproduced by a command in the
//! tree. A path under a machine's own directories or under `scratchpad/` is a pointer for the
//! session that wrote it and never the evidence — the one is on one machine only, and may be under an
//! account the owner cannot read (`doc/environment.md`, "The machine, the account and the display"),
//! and the other is gone by the next batch. Review 1704 section 1.5 counted 63 lines of one span
//! naming the notebook agent's home directory, and every relative `scratchpad/r16xx/` path its ADRs
//! cited had already gone.
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
//!
//! # Why each marker starts at a number of its own
//!
//! The same reason, a second time. The first markers were the notebook's: its agent's home
//! directory and `scratchpad/`. When the tools stopped naming any home directory and took their
//! places from `tools/places.sh` (ADR 2656), what a new record would name outside the tree became any
//! home or scratch directory, `~/`, or a variable of that file — and a record written before then may
//! name `~/.claude/…` as the configuration it describes, under no rule that existed. So each marker
//! is held from the first session and the first ADR after it was added, [`OUTSIDE`] says which, and
//! the records already held under the first markers stay held under them.

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

/// What marks a path that is not in the tree, with the first session and the first ADR whose
/// documents it is held against.
///
/// A `$NAME/` marker is a root `tools/places.sh` defines, matched as `$NAME/` or `${NAME}/`; the
/// calibration test holds each to a name that file assigns (trap 25).
const OUTSIDE: [(&str, u32, u32); 10] = [
    ("/home/AI/", SESSIONS_FROM, DECISIONS_FROM),
    ("scratchpad/", SESSIONS_FROM, DECISIONS_FROM),
    ("/home/", PLACES_FROM.0, PLACES_FROM.1),
    ("/scratch/", PLACES_FROM.0, PLACES_FROM.1),
    ("~/", PLACES_FROM.0, PLACES_FROM.1),
    ("$HOME/", PLACES_FROM.0, PLACES_FROM.1),
    ("$PDF_VIEWER_SCRATCH/", PLACES_FROM.0, PLACES_FROM.1),
    ("$PDF_VIEWER_AGENT/", PLACES_FROM.0, PLACES_FROM.1),
    ("$PDF_VIEWER_BUILDS/", PLACES_FROM.0, PLACES_FROM.1),
    ("$BATCH_ARMS_ROOT/", PLACES_FROM.0, PLACES_FROM.1),
];

/// The first session and the first ADR held against the markers `tools/places.sh` made necessary:
/// the round that wrote it, and its ADR (ADR 2656).
const PLACES_FROM: (u32, u32) = (1911, 2656);

/// The markers a document is held against: each whose first number the document's reaches.
fn markers_for(directory: &str, number: u32) -> Vec<&'static str> {
    OUTSIDE
        .iter()
        .filter(|(_, sessions, decisions)| {
            number
                >= if directory == "doc/adr" {
                    *decisions
                } else {
                    *sessions
                }
        })
        .map(|(marker, _, _)| *marker)
        .collect()
}

/// Every marker, as the newest document is held against them.
fn every_marker() -> Vec<&'static str> {
    markers_for("doc/history", u32::MAX)
}

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
/// A marker on its own — `scratchpad/`, `~/`, `$PDF_VIEWER_AGENT/` — names the kind of place and
/// points at nothing, which is how this rule itself is written down, so it is not one of them.
fn outside_paths(sentence: &str, markers: &[&str]) -> Vec<String> {
    sentence
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| "`'\"()[]{},.;:*\u{201c}\u{201d}".contains(c))
                .to_owned()
        })
        .filter(|word| {
            let unbraced = unbraced(word);
            !markers.contains(&unbraced.as_str())
                && markers.iter().any(|marker| unbraced.contains(marker))
        })
        .collect()
}

/// A word with each `${NAME}` written as `$NAME`, so that both spellings of a variable meet one
/// marker.
fn unbraced(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut rest = word;
    while let Some(at) = rest.find("${") {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        let name = &after[2..];
        if let Some(end) = name.find('}') {
            out.push('$');
            out.push_str(&name[..end]);
            rest = &name[end.saturating_add(1)..];
        } else {
            out.push_str(after);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// What one sentence does with the paths outside the tree it names, read against `markers`.
fn use_of(sentence: &str, markers: &[&str]) -> Use {
    let paths = outside_paths(sentence, markers);
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

/// Records the project owner wrote, which no round may edit, and which this rule therefore cannot
/// bind: review 1983, the owner's specification audit, names its local working data outside the
/// tree, and what a round adds about it goes beside it, in its directory's `fixes.md`.
const OWNERS_RECORDS: [&str; 1] = ["1983-specification-audit.md"];

/// Every document in the population, as its directory, its file name and its text.
#[expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read the records has not found a defect, and \
              reporting that as one would be worse than stopping"
)]
fn population() -> (usize, Vec<(&'static str, u32, String, String)>) {
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
            if number < from || OWNERS_RECORDS.contains(&name.as_str()) {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("a record's text");
            found.push((directory, number, name, text));
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
    for (directory, number, name, text) in &documents {
        let markers = markers_for(directory, *number);
        for (line, sentence) in sentences(text) {
            match use_of(&sentence, &markers) {
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
    let markers = every_marker();
    for (sentence, expected) in planted {
        assert_eq!(
            use_of(sentence, &markers),
            expected,
            "planted sentence: {sentence}"
        );
    }

    // A workplace in one list item does not license a path in the next, nor one sentence another.
    let text = "- the session worked in `scratchpad/r9999/`\n- the log is \
                `scratchpad/r9999/a.log`\n\nIt worked in `scratchpad/r9999/`. The figures are \
                in `/home/AI/x/`.\n";
    let uses: Vec<Use> = sentences(text)
        .into_iter()
        .map(|(_, sentence)| use_of(&sentence, &markers))
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

/// The markers `tools/places.sh` made necessary, each in the spellings a record would use, and
/// held only from the session and the ADR that added them (ADR 2657; trap 13).
#[test]
fn the_markers_the_places_file_added_are_held_from_their_round() {
    let markers = every_marker();
    let places = [
        (
            "The arms held by page against `~/scratch/agent/arms-9999/`.",
            "~/scratch/agent/arms-9999/",
        ),
        (
            "The receipt is /home/somebody/scratch/agent/batch-gates.log.",
            "/home/somebody/scratch/agent/batch-gates.log",
        ),
        (
            "The receipt is /scratch/somebody/cargo-target/r9999-head/x.txt.",
            "/scratch/somebody/cargo-target/r9999-head/x.txt",
        ),
        (
            "The six arms held against `${PDF_VIEWER_AGENT}/arms-9999/`.",
            "${PDF_VIEWER_AGENT}/arms-9999/",
        ),
        (
            "The figures are in `$BATCH_ARMS_ROOT/arms-9999/README`.",
            "$BATCH_ARMS_ROOT/arms-9999/README",
        ),
        ("The log is `$HOME/x.log`.", "$HOME/x.log"),
    ];
    for (sentence, path) in places {
        assert_eq!(
            use_of(sentence, &markers),
            Use::Evidence(path.to_owned()),
            "planted sentence: {sentence}"
        );
        assert_eq!(
            use_of(sentence, &markers_for("doc/history", PLACES_FROM.0 - 1)),
            Use::None,
            "a record from before the marker was added is held to it: {sentence}"
        );
        assert_eq!(
            use_of(sentence, &markers_for("doc/adr", PLACES_FROM.1 - 1)),
            Use::None,
            "an ADR from before the marker was added is held to it: {sentence}"
        );
    }
    for bare in [
        "A path under `$PDF_VIEWER_AGENT/`, `${PDF_VIEWER_BUILDS}/` or `~/` is never the evidence.",
        "A path under `/home/AI/` or `/home/` is never the evidence.",
        "The session worked in `~/scratch/agent/arms-9999/`.",
    ] {
        assert!(
            matches!(use_of(bare, &markers), Use::None | Use::Workplace),
            "planted sentence: {bare}"
        );
    }

    // Each `$NAME/` marker is a place `tools/places.sh` assigns, so that none names a variable
    // that never existed (trap 25).
    let places_sh = std::fs::read_to_string(conformance::workspace_root().join("tools/places.sh"))
        .unwrap_or_default();
    for (marker, _, _) in OUTSIDE {
        let Some(name) = marker
            .strip_prefix('$')
            .and_then(|rest| rest.strip_suffix('/'))
        else {
            continue;
        };
        assert!(
            name == "HOME" || places_sh.contains(&format!(": \"${{{name}:=")),
            "the marker {marker} names no place tools/places.sh assigns"
        );
    }
}
