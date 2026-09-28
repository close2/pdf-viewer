//! The trap index resolves every number in one hop, and this is what checks the hop.
//!
//! # What it guards
//!
//! `doc/traps/README.md` is one line per trap, and the line's last column names the group file
//! where the incident, the evidence and the argument live (ADR 1036). The index is a lookup key and
//! nothing may be decided from it alone, so a row whose group file has no entry of that number is
//! a key to nothing — and the rows are added by a merge that writes the index and the group file in
//! two edits, one of which can miss its anchor. Nothing counted that until now; a round read both
//! files by eye.
//!
//! Both directions are checked. **Every row names a group whose file has a heading of that
//! number**, and **every numbered heading in a group file has a row naming that group** — so a
//! trap written into a group file and never indexed is as visible as an index row that points at
//! nothing. A trap merged into another keeps its row and its number, and its incident sits under
//! the survivor as a `####` heading of its own number (trap 4 under trap 8, trap 29 under
//! trap 13); those count. A row demoted to a habit names `*habits*` and is resolved there instead.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// The index, relative to the repository root.
const INDEX: &str = "doc/traps/README.md";

/// The group column's words and the file each names, as the index's own table of files states
/// them. A row naming a word outside this list fails the gate rather than being skipped.
const GROUPS: [(&str, &str); 5] = [
    ("pixels", "doc/traps/pixels-and-rasterisers.md"),
    ("oracle", "doc/traps/oracle-and-references.md"),
    ("parsers", "doc/traps/parsers-and-streams.md"),
    ("interactive loop", "doc/traps/the-interactive-loop.md"),
    ("instruments", "doc/traps/instruments-and-reports.md"),
];

/// The group word of a trap demoted to a habit, and where its incident now lives.
const HABITS: (&str, &str) = ("*habits*", "doc/habits/measuring.md");

/// The index's rows: each trap's number and the group word in its last column.
fn index_rows(index: &str) -> Vec<(String, String)> {
    index
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line
                .strip_prefix('|')?
                .strip_suffix('|')?
                .split('|')
                .map(str::trim)
                .collect();
            let number = *cells.first()?;
            let group = *cells.last()?;
            let starts_with_digit = number.chars().next().is_some_and(|c| c.is_ascii_digit());
            (starts_with_digit && cells.len() == 4).then(|| (number.to_owned(), group.to_owned()))
        })
        .collect()
}

/// The trap numbers a group file gives a heading: `### 53. …` and a merged trap's `#### 4. …`.
fn headed_numbers(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim_start_matches('#');
            let level = line.len().saturating_sub(rest.len());
            if !(3..=4).contains(&level) {
                return None;
            }
            let (number, _) = rest.trim_start().split_once('.')?;
            let well_formed = number.chars().next().is_some_and(|c| c.is_ascii_digit())
                && number.chars().all(|c| c.is_ascii_alphanumeric());
            well_formed.then(|| number.to_owned())
        })
        .collect()
}

/// Every row of the index has an entry of its number in the group file it names, and every
/// numbered entry of a group file has a row naming that group.
#[test]
fn every_trap_row_has_its_entry_and_every_entry_its_row() {
    let root = conformance::workspace_root();
    let read = |path: &str| {
        std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    let rows = index_rows(&read(INDEX));
    assert!(!rows.is_empty(), "{INDEX} holds no rows this gate can read");

    let headed: BTreeMap<&str, BTreeSet<String>> = GROUPS
        .iter()
        .map(|(word, file)| (*word, headed_numbers(&read(file))))
        .collect();

    let mut failures = String::new();
    for (number, group) in &rows {
        if *group == HABITS.0 {
            if !read(HABITS.1).contains(&format!("trap {number}")) {
                let _ = writeln!(failures, "  trap {number}: {} never names it", HABITS.1);
            }
            continue;
        }
        match headed.get(group.as_str()) {
            None => {
                let _ = writeln!(
                    failures,
                    "  trap {number}: no group file is called `{group}`"
                );
            }
            Some(numbers) if !numbers.contains(number) => {
                let _ = writeln!(
                    failures,
                    "  trap {number}: `{group}`'s file has no entry for it"
                );
            }
            Some(_) => {}
        }
    }
    let indexed: BTreeSet<(&str, &str)> = rows
        .iter()
        .map(|(number, group)| (number.as_str(), group.as_str()))
        .collect();
    for (group, numbers) in &headed {
        for number in numbers {
            if !indexed.contains(&(number.as_str(), *group)) {
                let _ = writeln!(
                    failures,
                    "  trap {number}: an entry in `{group}`'s file with no row naming that group"
                );
            }
        }
    }
    assert!(
        failures.is_empty(),
        "the trap index and its group files disagree:\n{failures}"
    );
}

/// The reader recognises the shapes the files are written in, and nothing else — the control
/// that keeps a clean run from being a reader that found nothing.
#[test]
fn the_reader_finds_rows_and_headings_and_nothing_else() {
    let index = "| # | you are | rule | group |\n|---|---|---|---|\n\
                 | 10a | x | y | instruments |\n| 36 | x | y | *habits* |\n";
    assert_eq!(
        index_rows(index),
        [
            ("10a".to_owned(), "instruments".to_owned()),
            ("36".to_owned(), "*habits*".to_owned())
        ]
    );
    let group = "## Traps\n### 53. A title\n#### 4. Merged\n## 2. Not a trap\n### Things. No\n";
    assert_eq!(
        headed_numbers(group),
        BTreeSet::from(["4".to_owned(), "53".to_owned()])
    );
}
