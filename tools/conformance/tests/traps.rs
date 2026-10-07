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
//!
//! **A row is four cells, and a bar inside one is written `\|`.** A literal `|` in a cell — a
//! shell pipe quoted in a rule, inside a code span or not — ends the cell under GitHub's table
//! syntax, so the row renders with five columns and a reader that wants four drops it — and the
//! only complaint left is a heading "with no row naming that group" beside a row that is plainly
//! there. So a line shaped like a row whose cell count is not four is named as itself, by its line
//! and its trap, with the spelling that fixes it (ADR 1663).

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

/// The cells of a table line, trimmed, split at every `|` that is not written `\|` — GitHub's
/// table syntax, under which an escaped bar is a character of its cell, a code span's included.
/// `None` for a line that does not open and close with a bar.
fn cells(line: &str) -> Option<Vec<String>> {
    let inner = line.trim_end().strip_prefix('|')?.strip_suffix('|')?;
    let mut cells = vec![String::new()];
    let mut escaped = false;
    for character in inner.chars() {
        if character == '|' && !escaped {
            cells.push(String::new());
        } else if let Some(cell) = cells.last_mut() {
            cell.push(character);
        }
        escaped = character == '\\' && !escaped;
    }
    Some(cells.iter().map(|cell| cell.trim().to_owned()).collect())
}

/// The cells of every line shaped like a trap's row — a table line whose first cell opens with a
/// digit — with its line number, whatever their count.
fn row_shaped(index: &str) -> impl Iterator<Item = (usize, Vec<String>)> + '_ {
    index.lines().enumerate().filter_map(|(at, line)| {
        let cells = cells(line)?;
        let numbered = cells
            .first()?
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit());
        numbered.then(|| (at.saturating_add(1), cells))
    })
}

/// The number of cells a row of the index has: the number, the position, the rule, the group.
const ROW_CELLS: usize = 4;

/// The index's rows: each trap's number and the group word in its last column.
fn index_rows(index: &str) -> Vec<(String, String)> {
    row_shaped(index)
        .filter(|(_, cells)| cells.len() == ROW_CELLS)
        .filter_map(|(_, cells)| Some((cells.first()?.clone(), cells.last()?.clone())))
        .collect()
}

/// The lines shaped like a row that are not [`ROW_CELLS`] cells: `(line, trap, cells)`.
fn malformed_rows(index: &str) -> Vec<(usize, String, usize)> {
    row_shaped(index)
        .filter(|(_, cells)| cells.len() != ROW_CELLS)
        .map(|(line, cells)| {
            (
                line,
                cells.first().cloned().unwrap_or_default(),
                cells.len(),
            )
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
    let index = read(INDEX);
    let rows = index_rows(&index);
    assert!(!rows.is_empty(), "{INDEX} holds no rows this gate can read");
    let malformed = malformed_rows(&index);

    let headed: BTreeMap<&str, BTreeSet<String>> = GROUPS
        .iter()
        .map(|(word, file)| (*word, headed_numbers(&read(file))))
        .collect();

    let mut failures = String::new();
    for (line, number, count) in &malformed {
        let _ = writeln!(
            failures,
            "  trap {number}: its row ({INDEX} line {line}) has {count} cells, not {ROW_CELLS} — \
             a `|` inside a cell ends the cell; write it `\\|`"
        );
    }
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
            // A row named above as malformed is the reason its entry has no row, and is said once.
            let said = malformed
                .iter()
                .any(|(_, malformed, _)| malformed == number);
            if !said && !indexed.contains(&(number.as_str(), *group)) {
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
/// that keeps a clean run from being a reader that found nothing — and names a row a bare `|`
/// split, which is the defect planted back (trap 13).
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
    // A bar written `\|` is a character of its cell, inside a code span or not; a bare one is
    // where a cell ends, and the row it splits is named by its line rather than dropped.
    let escaped = "| 127 | a build sits behind `\\| tail` | so \\| it | instruments |\n";
    assert_eq!(
        index_rows(escaped),
        [("127".to_owned(), "instruments".to_owned())]
    );
    assert!(malformed_rows(escaped).is_empty());
    let split =
        "| # | a | b | group |\n| 127 | a build sits behind `| tail` | so | instruments |\n";
    assert!(index_rows(split).is_empty());
    assert_eq!(malformed_rows(split), [(2, "127".to_owned(), 5)]);
    assert!(malformed_rows(index).is_empty());

    let group = "## Traps\n### 53. A title\n#### 4. Merged\n## 2. Not a trap\n### Things. No\n";
    assert_eq!(
        headed_numbers(group),
        BTreeSet::from(["4".to_owned(), "53".to_owned()])
    );
}
