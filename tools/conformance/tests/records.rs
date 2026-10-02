//! A round's record is at most forty lines, and this is what counts them.
//!
//! # A number a document states is a number nobody measures
//!
//! `doc/todo/02` section 8 caps a round's record at forty lines, and every brief repeats it; a
//! cap nothing counts is exceeded unnoticed (ADR 1100). That is the shape `CLAUDE.md` ("Where
//! knowledge lives") names: a
//! fact that can be counted is not written down, and what is written down is the command that
//! counts it.
//!
//! # Why the bound starts at a number rather than at the beginning
//!
//! **Because a record may not be rewritten.** `CLAUDE.md` says `doc/adr/`, `doc/history/` and
//! `doc/reviews/` keep their chronology and that rewriting one for tidiness is the single thing a
//! record may not have done to it — so a check whose only remedy is an edit may not be pointed at
//! a record written before the check existed. [`COUNTED_FROM`] is where the counting began, which
//! is the batch that built the instrument and trimmed its own two.
//!
//! The twelve most recent are *printed* whatever their number, because the ones below the bound
//! are what the bound has to be read against: a budget nobody was over would be a budget nobody
//! needed.
//!
//! # A record states its gates
//!
//! A record is the only place a round's gates are kept once the report that carried them is gone,
//! and the report is never in the tree. So every record from [`GATES_FROM`] on carries a paragraph
//! opening `**Gates.**` — the shape most records already used — that names at least one figure: an
//! exit status or a pass count. "See the report" is refused by name, because it points at a text the
//! tree does not hold. The check sees *a figure* in that paragraph, not that the figure belongs to a
//! gate; a planted record of each verdict calibrates it, and the records since [`GATES_LISTED_FROM`]
//! are printed with theirs, written before the rule and never rewritten to meet it (ADR 1499).

#![expect(
    clippy::print_stdout,
    reason = "the gate prints the population it counted, which is what makes its verdict readable"
)]

use std::fmt::Write as _;

/// The most lines one round's record may run to.
///
/// `doc/todo/02` section 8's figure, and the only copy of it that anything reads.
const BUDGET: usize = 40;

/// The first record number this check holds to [`BUDGET`], the batch that built it (ADR 1100).
///
/// Every record before it is a record nothing may edit, and so is a record nothing may fail.
const COUNTED_FROM: u32 = 1086;

/// How many of the most recent records are printed beside the budget.
const PRINTED: usize = 12;

/// Where the records are.
const HISTORY: &str = "doc/history";

/// The first record that must state its gates, the batch that made the rule (ADR 1499).
const GATES_FROM: u32 = 1327;

/// The first record whose gates are printed, so the rule is read against the records before it.
const GATES_LISTED_FROM: u32 = 1284;

/// What a record says about its gates.
#[derive(Debug, PartialEq, Eq)]
enum Gates {
    /// A `**Gates.**` paragraph naming at least one figure.
    Stated,
    /// A `**Gates.**` paragraph that sends the reader to the report instead.
    SeeTheReport,
    /// A `**Gates.**` paragraph with no figure in it.
    NoFigure,
    /// Gates stated in another shape — a `## Gates` heading, `**Gates**:` or `Gates:`.
    OtherShape,
    /// No gates at all.
    Absent,
}

impl Gates {
    /// Whether a record from [`GATES_FROM`] on may carry this verdict.
    fn meets_the_rule(&self) -> bool {
        *self == Self::Stated
    }

    /// What is printed beside a record with this verdict.
    fn describe(&self) -> &'static str {
        match self {
            Self::Stated => "states its gates",
            Self::SeeTheReport => "says \"see the report\"",
            Self::NoFigure => "a Gates paragraph with no exit status or count",
            Self::OtherShape => "gates in another shape",
            Self::Absent => "no gates",
        }
    }
}

/// Whether one word of a `**Gates.**` paragraph is a figure: a count or an exit status (`0`,
/// `918`) or a count against a total (`39/39`), once the punctuation around it is set aside.
fn is_figure(word: &str) -> bool {
    let word = word.trim_matches(|c: char| c.is_ascii_punctuation() && c != '/');
    let all_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    match word.split_once('/') {
        Some((count, total)) => all_digits(count) && all_digits(total),
        None => all_digits(word),
    }
}

/// What a record's text says about its gates.
fn gates_of(text: &str) -> Gates {
    let lines: Vec<&str> = text.lines().collect();
    let mut verdict = None;
    for (at, line) in lines.iter().enumerate() {
        if !line.starts_with("**Gates.**") {
            continue;
        }
        // The paragraph runs to the first blank line.
        let paragraph: Vec<&str> = lines[at..]
            .iter()
            .take_while(|line| !line.trim().is_empty())
            .copied()
            .collect();
        let paragraph = paragraph.join(" ");
        let found = if paragraph.to_lowercase().contains("see the report") {
            Gates::SeeTheReport
        } else if paragraph.split_whitespace().any(is_figure) {
            Gates::Stated
        } else {
            Gates::NoFigure
        };
        if found == Gates::Stated {
            return found;
        }
        verdict.get_or_insert(found);
    }
    verdict.unwrap_or_else(|| {
        let other = lines.iter().any(|line| {
            line.starts_with("## Gates")
                || line.starts_with("**Gates**")
                || line.starts_with("Gates:")
                || line.starts_with("**Tests and gates.**")
        });
        if other {
            Gates::OtherShape
        } else {
            Gates::Absent
        }
    })
}

/// Every record a round wrote, as its session number, its file name and its text, in order.
#[expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read the records has not found a defect, and \
              reporting that as one would be worse than stopping"
)]
fn records() -> Vec<(u32, String, String)> {
    let root = conformance::workspace_root();
    let mut records = Vec::new();
    for entry in std::fs::read_dir(root.join(HISTORY)).expect("the records") {
        let path = entry.expect("a record").path();
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        // A record is `<session>-<slug>.md`. `README.md` is the directory's own note and is not
        // one, and it is told apart by the number rather than by its name — a list of exceptions
        // is trap 25's shape however short it is.
        let Some(session) = name
            .split('-')
            .next()
            .and_then(|digits| digits.parse::<u32>().ok())
        else {
            continue;
        };
        let text = std::fs::read_to_string(&path).expect("a record's text");
        records.push((session, name, text));
    }
    records.sort_unstable();
    records
}

/// Every record a round wrote since the budget was counted fits inside it.
#[test]
fn every_record_since_the_budget_was_counted_fits_inside_it() {
    let records: Vec<(u32, String, usize)> = records()
        .into_iter()
        .map(|(session, name, text)| (session, name, text.lines().count()))
        .collect();

    let mut over = String::new();
    for (session, name, lines) in &records {
        if *session >= COUNTED_FROM && *lines > BUDGET {
            let _ = writeln!(
                over,
                "{HISTORY}/{name}: {lines} lines, over the budget of {BUDGET}. Cut restatement, \
                 never a fact: what the round found, what it measured and what it left owed are \
                 the record, and the sentences saying them again are not."
            );
        }
    }

    println!("the last {PRINTED} records, against a budget of {BUDGET} lines:");
    for (session, name, lines) in records.iter().rev().take(PRINTED).rev() {
        let verdict = if *lines > BUDGET {
            if *session >= COUNTED_FROM {
                "over"
            } else {
                "over, and written before the count existed"
            }
        } else {
            ""
        };
        println!("  {session} {lines:4} {verdict:<38} {name}");
    }
    println!(
        "{} records, {} of them counted against the budget",
        records.len(),
        records
            .iter()
            .filter(|(session, _, _)| *session >= COUNTED_FROM)
            .count()
    );

    assert!(
        records.len() > 100,
        "only {} records found: the walk is not reaching {HISTORY}",
        records.len()
    );
    assert!(over.is_empty(), "\n{over}");
}

/// Every record from [`GATES_FROM`] on states its gates, and the ones since [`GATES_LISTED_FROM`]
/// that do not are printed.
#[test]
fn every_record_since_the_rule_states_its_gates() {
    let records = records();
    let mut owed = String::new();
    println!(
        "records since {GATES_LISTED_FROM} that do not state their gates in a **Gates.** paragraph:"
    );
    for (session, name, text) in &records {
        if *session < GATES_LISTED_FROM {
            continue;
        }
        let gates = gates_of(text);
        if gates.meets_the_rule() {
            continue;
        }
        let bound = if *session >= GATES_FROM {
            let _ = writeln!(
                owed,
                "{HISTORY}/{name}: {}. A record states its gates in a paragraph opening \
                 `**Gates.**`, each with its exit status or pass count (ADR 1499).",
                gates.describe()
            );
            "owed"
        } else {
            "before the rule"
        };
        println!("  {session} {:<46} {bound}", gates.describe());
    }
    assert!(owed.is_empty(), "\n{owed}");
}

/// The check, against a planted record of each verdict.
#[test]
fn the_gates_check_is_calibrated_against_planted_records() {
    let planted = [
        (
            "# 9999 — planted\n\n**Gates.** rustfmt 0; clippy 0; nextest 918 passed;\n`conformance` 0.\n",
            Gates::Stated,
        ),
        (
            "**Gates.** fmt, clippy, tests:\nall exit 0.\n\n**Left.** nothing\n",
            Gates::Stated,
        ),
        ("**Gates.** headless_gpu 39/39.\n", Gates::Stated),
        ("**Gates.** fmt; clippy at 4×.\n", Gates::NoFigure),
        (
            "**Gates.** Corpus 1× and 4× cpu, raster_golden: see the report.\n",
            Gates::SeeTheReport,
        ),
        (
            "**Gates.** See the report; 0 failed.\n",
            Gates::SeeTheReport,
        ),
        (
            "**Gates.** fmt; clippy; nextest; conformance.\n",
            Gates::NoFigure,
        ),
        (
            "**Gates.** fmt; clippy.\n\nThe page drew 3 marks.\n",
            Gates::NoFigure,
        ),
        ("## Gates\nrustfmt 0; clippy 0.\n", Gates::OtherShape),
        (
            "# 9999 — planted\n\nNothing about gates at all, 3 rows moved.\n",
            Gates::Absent,
        ),
        ("Mid-line **Gates.** 0\n", Gates::Absent),
    ];
    for (text, expected) in planted {
        assert_eq!(gates_of(text), expected, "planted record:\n{text}");
    }
}
