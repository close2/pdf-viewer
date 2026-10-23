//! An ADR states its decision under a Status line, and this is what reads the line.
//!
//! # The rule
//!
//! ADR 2242 decision 3: an ADR states a decision under a Status line, and an assessment, a survey,
//! a test plan or a diagnostic experiment is a review or a record's paragraph instead. Review 1704
//! finding 5 found fourteen of sixty ADRs with no Status line and five frozen at a state the merge
//! had already passed — "under validation", "integration pending", "proposed for integration" —
//! so that a reader of `doc/adr/` could not tell which of them was in force. A rule nothing reads
//! is the shape ADR 1100 names; this reads it.
//!
//! **What the line must say.** It stands in the ADR's opening, before the first `## ` heading, as
//! `Status:` followed by the decision's state, and the state's first word is one of [`STATES`].
//! Three things are refused even under one of those words, because each describes the merge rather
//! than the decision, and the merge does not come back to edit the line: a state that waits
//! ("pending"), one that is still being checked ("under validation"), and `proposed` with no
//! question to the owner named in it — an ADR may wait on the owner's word, which is a `Q` file
//! (`doc/todo/02` section 6a), and on nothing else. The state is read up to the end of its sentence,
//! so a second sentence may say what the line replaced.
//!
//! # Why the bound starts at a number
//!
//! **Because an ADR is a record.** `CLAUDE.md` ("Where knowledge lives") says a record keeps its
//! chronology, so a check whose only remedy is an edit may not be pointed at a record written before
//! the check existed — which is why `conformance::documents` keeps `doc/adr/` out of the population
//! its `§` rule reads, and why this check is a population of its own rather than a widening of that
//! one. [`STATUS_FROM`] is ADR 2242, the decision that made the rule. The ADRs before it that do not
//! meet it are *printed* from [`STATUS_LISTED_FROM`], the span review 1704 read, so the rule is read
//! against what it was made from; those were given the line the format owes, with no other word of
//! them changed, by ADR 2260, which is the remedy ADR 0265 names for a record whose state moved.

#![expect(
    clippy::print_stdout,
    reason = "the gate prints the population it counted, which is what makes its verdict readable"
)]

use std::fmt::Write as _;

/// The first ADR the Status rule holds, the decision that made it (ADR 2242 decision 3).
const STATUS_FROM: u32 = 2242;

/// The first ADR printed beside the rule: the span review 1704 found the defect in.
const STATUS_LISTED_FROM: u32 = 2178;

/// Where the ADRs are.
const DECISIONS: &str = "doc/adr";

/// The states a decision can be in, as the first word of its Status line.
///
/// `implemented` is the shape `accepted and built` takes in a docs round's ADR; `deferred` is a
/// decision not to build now, with what reopens it written below; `amended` and `superseded` are
/// ADR 0265's amendment at the top.
const STATES: [&str; 7] = [
    "accepted",
    "rejected",
    "deferred",
    "superseded",
    "amended",
    "implemented",
    "proposed",
];

/// What an ADR's opening says about its state.
#[derive(Debug, PartialEq, Eq)]
enum Status {
    /// A Status line naming a state from [`STATES`], and nothing the merge passes.
    Stated,
    /// No `Status:` in the opening.
    Absent,
    /// A `Status:` whose first word is not a state from [`STATES`].
    NotAState(String),
    /// A state that describes the merge rather than the decision.
    Frozen(&'static str),
}

impl Status {
    /// What is printed beside an ADR with this verdict.
    fn describe(&self) -> String {
        match self {
            Self::Stated => "states its decision".to_owned(),
            Self::Absent => "no Status line in its opening".to_owned(),
            Self::NotAState(word) => format!("\"{word}\" is not a decision's state"),
            Self::Frozen(why) => format!("frozen: {why}"),
        }
    }
}

/// What an ADR's text says about its state.
fn status_of(text: &str) -> Status {
    let opening: Vec<&str> = text
        .lines()
        .take_while(|line| !line.starts_with("## "))
        .collect();
    let opening = opening.join(" ").replace('*', "");
    let Some((_, rest)) = opening.split_once("Status:") else {
        return Status::Absent;
    };
    let rest = rest.trim_start();
    let state = rest.find(". ").map_or(rest, |end| &rest[..end]);
    let first: String = state
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect::<String>()
        .to_lowercase();
    if !STATES.contains(&first.as_str()) {
        return Status::NotAState(first);
    }
    let lowered = state.to_lowercase();
    if lowered.contains("pending") {
        return Status::Frozen("a state that waits on the merge");
    }
    if lowered.contains("under validation") {
        return Status::Frozen("a state still being checked");
    }
    if first == "proposed" && !names_a_question(state) {
        return Status::Frozen("proposed, and to no owner's question");
    }
    Status::Stated
}

/// Whether a text names a question to the owner, `Q` and its digits.
fn names_a_question(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| {
            word.strip_prefix('Q').is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            })
        })
}

/// Every ADR, as its number, its file name and its text, in order.
#[expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read the ADRs has not found a defect, and reporting \
              that as one would be worse than stopping"
)]
fn decisions() -> Vec<(u32, String, String)> {
    let root = conformance::workspace_root();
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root.join(DECISIONS)).expect("the ADRs") {
        let path = entry.expect("an ADR").path();
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        if path.extension().is_none_or(|suffix| suffix != "md") {
            continue;
        }
        let Some(number) = name
            .split('-')
            .next()
            .and_then(|digits| digits.parse::<u32>().ok())
        else {
            continue;
        };
        let text = std::fs::read_to_string(&path).expect("an ADR's text");
        found.push((number, name, text));
    }
    found.sort_unstable();
    found
}

/// Every ADR from [`STATUS_FROM`] on states its decision, and the ones since
/// [`STATUS_LISTED_FROM`] that do not are printed.
#[test]
fn every_adr_since_the_rule_states_its_decision() {
    let decisions = decisions();
    let mut owed = String::new();
    let mut held = 0usize;
    println!("ADRs since {STATUS_LISTED_FROM} whose Status line does not meet the rule:");
    for (number, name, text) in &decisions {
        if *number < STATUS_LISTED_FROM {
            continue;
        }
        let status = status_of(text);
        if *number >= STATUS_FROM {
            held = held.saturating_add(1);
        }
        if status == Status::Stated {
            continue;
        }
        let bound = if *number >= STATUS_FROM {
            let _ = writeln!(
                owed,
                "{DECISIONS}/{name}: {}. An ADR opens with `Status:` and its decision's state — \
                 one of {STATES:?} — and nothing that waits on the merge (ADR 2242 decision 3).",
                status.describe()
            );
            "owed"
        } else {
            "before the rule"
        };
        println!("  {number} {:<48} {bound}", status.describe());
    }
    println!("{held} ADRs from {STATUS_FROM} on held to the Status rule");
    assert!(
        decisions.len() > 1000,
        "only {} ADRs found: the walk is not reaching {DECISIONS}",
        decisions.len()
    );
    assert!(held > 0, "no ADR from {STATUS_FROM} on was found");
    assert!(owed.is_empty(), "\n{owed}");
}

/// The check, against a planted opening of each verdict (trap 13).
#[test]
fn the_status_check_is_calibrated_against_planted_adrs() {
    let planted = [
        (
            "# 9999 — planted\n\nStatus: **accepted**. Context: ADR 1.\n\n## 1. Decision\n",
            Status::Stated,
        ),
        (
            "# 9999 — planted\n\nA docs round. Status: **accepted** and\n**built** \
             in `x.rs`. Context: ADR 1.\n",
            Status::Stated,
        ),
        (
            "# 9999 — planted\n\nStatus: **superseded by ADR 9998** for all five cards.\n",
            Status::Stated,
        ),
        (
            "# 9999 — planted\n\nStatus: **accepted** at the merge that took it. The round wrote \
             \u{201c}accepted; implementation under validation\u{201d}.\n",
            Status::Stated,
        ),
        (
            "# 9999 — planted\n\nStatus: **proposed**, the owner's answer to Q9999 awaited.\n",
            Status::Stated,
        ),
        (
            "# 9999 — planted\n\nA paragraph and no state.\n\n## Status\n\nStatus: accepted.\n",
            Status::Absent,
        ),
        (
            "# 9999 — planted\n\nAccepted: the gates passed.\n",
            Status::Absent,
        ),
        (
            "# 9999 — planted\n\nStatus: diagnostic experiment; a record follows.\n",
            Status::NotAState("diagnostic".to_owned()),
        ),
        (
            "# 9999 — planted\n\nStatus: accepted; final batch integration pending. Context: ADR 1.\n",
            Status::Frozen("a state that waits on the merge"),
        ),
        (
            "# 9999 — planted\n\nAn archive round. Status: accepted; implementation under \
             validation.\n",
            Status::Frozen("a state still being checked"),
        ),
        (
            "# 9999 — planted\n\nStatus: proposed for integration; paired cost measured.\n",
            Status::Frozen("proposed, and to no owner's question"),
        ),
    ];
    for (text, expected) in planted {
        assert_eq!(status_of(text), expected, "planted ADR:\n{text}");
    }
}
