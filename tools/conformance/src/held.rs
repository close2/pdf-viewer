//! The oracle's pages held by name, read from its test file's own constants without running it.
//!
//! The oracle (`crates/pdf-model/tests/oracle.rs`) sorts every page it compares into a verdict,
//! and every verdict but `agrees` is held by name: each page sits in a group — a `const
//! <STATUS>_<CAUSE>: [&str; N]` declaration whose doc comment says why — and the walk fails on a
//! page that arrives in a verdict no group holds or leaves one a group still names. So the groups
//! are a standing fact about the tree, readable without the walk, and until this module nothing
//! printed them: a robustness round read sixteen thousand lines to learn that one group holds 369
//! pages and which contradictions are held as a departure of ours (ADR 1512).
//!
//! **What it reads is the declaration, not the run.** `N` in `[&str; N]` is checked by the
//! compiler against the list, so the counts are the file's own; which pages the walk *finds* in
//! each verdict, how many agree, and which held page is furthest outside its bound are the walk's,
//! and `tools/state.sh oracle` runs it. The two can differ legitimately: the walk's ambiguous and
//! contradicted ratchets hold only pages it rendered *completely*, so a page drawn with a refusal
//! is counted by the walk and held by no group.
//!
//! **The next page to take** is the oracle's own sentence (ADR 1483): the highest-ranked
//! contradicted page whose group is held as a departure of ours. The rank is a run's, so this
//! names the candidates — every page of every group `WHOSE_DEPARTURE` marks `Whose::Ours` — and
//! leaves the order to the walk.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// Where the oracle's test file is, relative to the workspace root.
pub const ORACLE: &str = "crates/pdf-model/tests/oracle.rs";

/// The verdicts a group can hold, as the prefix its constant's name opens with and the word the
/// oracle prints for it. Longest prefix first, so `NOT_COMPARABLE_` is not read as `NO_`-anything
/// and `REFERENCE_GEOMETRY_` not as `GEOMETRY`.
const VERDICTS: [(&str, &str); 6] = [
    ("CONTRADICTED_", "contradicted"),
    ("AMBIGUOUS_", "ambiguous"),
    ("NOT_COMPARABLE_", "not comparable"),
    ("NO_RENDER_", "no render"),
    ("REFERENCE_GEOMETRY_", "reference geometry"),
    ("GEOMETRY", "our geometry"),
];

/// A group of held pages: its constant's name, the declared count, and the pages listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The constant's name, `AMBIGUOUS_IMAGE_REDUCTION`.
    pub name: String,
    /// `N` in its `[&str; N]`.
    pub declared: usize,
    /// The string literals between its `=` and the `];` that closes it.
    pub pages: Vec<String>,
}

/// Whose departure from ISO 32000-2 a contradicted group's note names, as `WHOSE_DEPARTURE` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departure {
    /// `Ours`, `References` or `Choice` — the enum variant's own name.
    pub whose: String,
    /// The clause the group's note decides it by.
    pub clause: String,
}

/// Everything the report prints, read from the file.
#[derive(Debug, Default)]
pub struct Held {
    /// The groups of each verdict, keyed by the word the oracle prints.
    pub by_verdict: BTreeMap<&'static str, Vec<Group>>,
    /// `WHOSE_DEPARTURE`'s rows, by group name.
    pub whose: BTreeMap<String, Departure>,
}

/// Why the oracle's file could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file is not where [`ORACLE`] says.
    #[error("{path}: {source}")]
    Read {
        /// The path tried.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
    /// The file was read and held no group, so the parse has stopped matching it.
    #[error(
        "{0} declares no `[&str; N]` group the parse recognises — the parse has drifted from the file"
    )]
    Empty(String),
}

/// Reads [`ORACLE`] under `root`.
///
/// # Errors
///
/// [`Error::Read`] where the file cannot be read, [`Error::Empty`] where it holds no group.
pub fn read(root: &Path) -> Result<Held, Error> {
    let path = root.join(ORACLE);
    let text = std::fs::read_to_string(&path).map_err(|source| Error::Read {
        path: path.display().to_string(),
        source,
    })?;
    let held = parse(&text);
    if held.by_verdict.values().all(Vec::is_empty) {
        return Err(Error::Empty(ORACLE.to_owned()));
    }
    Ok(held)
}

/// The groups and `WHOSE_DEPARTURE` of an oracle source text.
#[must_use]
pub fn parse(text: &str) -> Held {
    let mut held = Held::default();
    let lines: Vec<&str> = text.lines().collect();
    for (at, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("const ") else {
            continue;
        };
        let Some((name, tail)) = rest.split_once(':') else {
            continue;
        };
        let Some(declared) = tail
            .trim_start()
            .strip_prefix("[&str; ")
            .and_then(|count| count.split_once(']'))
            .and_then(|(count, _)| count.trim().parse::<usize>().ok())
        else {
            continue;
        };
        let Some((_, verdict)) = VERDICTS.iter().find(|(prefix, _)| name.starts_with(prefix))
        else {
            continue;
        };
        let rest_of_file = lines.get(at..).unwrap_or_default();
        let closing = rest_of_file
            .iter()
            .position(|body| body.trim_end().ends_with("];"))
            .unwrap_or(rest_of_file.len());
        let body = rest_of_file
            .get(..=closing)
            .unwrap_or(rest_of_file)
            .join("\n");
        let after_equals = body.split_once('=').map_or("", |(_, value)| value);
        held.by_verdict.entry(verdict).or_default().push(Group {
            name: name.trim().to_owned(),
            declared,
            pages: string_literals(after_equals),
        });
    }
    if let Some(start) = text.find("const WHOSE_DEPARTURE") {
        let table = text.get(start..).unwrap_or_default();
        let table = table.split_once("\n];").map_or(table, |(table, _)| table);
        for row in table.split('(').skip(1) {
            let literals = string_literals(row);
            let whose = row
                .split_once("Whose::")
                .map(|(_, after)| {
                    after
                        .chars()
                        .take_while(char::is_ascii_alphanumeric)
                        .collect::<String>()
                })
                .unwrap_or_default();
            if let [group, clause] = literals.as_slice()
                && !whose.is_empty()
            {
                held.whose.insert(
                    group.clone(),
                    Departure {
                        whose,
                        clause: clause.clone(),
                    },
                );
            }
        }
    }
    held
}

/// Every `"…"` in `text`, without its quotes. The oracle's page names hold no escaped quote.
fn string_literals(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The words `WHOSE_DEPARTURE`'s variant stands for, as the oracle prints them.
fn phrase(whose: &str) -> &str {
    match whose {
        "Ours" => "a departure of ours",
        "References" => "a departure of the references'",
        "Choice" => "a choice the clause leaves to the processor",
        other => other,
    }
}

/// The report: per verdict its held count, then its groups by size with their pages where a group
/// is small enough to read on one line, then the contradicted split and the next page's candidates.
#[must_use]
pub fn report(held: &Held) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "held by name in {ORACLE} — the declarations, not a run; the walk's verdicts, the agreeing \
         pages and the ranking are tools/state.sh oracle's"
    );
    for (_, verdict) in VERDICTS {
        let groups = held.by_verdict.get(verdict).map_or(&[][..], Vec::as_slice);
        let pages: usize = groups.iter().map(|group| group.declared).sum();
        let filled = groups.iter().filter(|group| group.declared > 0).count();
        let _ = writeln!(
            out,
            "  {verdict:<19} {pages:>4} page(s) in {filled} group(s), {} declared",
            groups.len()
        );
    }
    for (_, verdict) in VERDICTS {
        let mut groups: Vec<&Group> = held
            .by_verdict
            .get(verdict)
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter(|group| group.declared > 0)
            .collect();
        if groups.is_empty() {
            continue;
        }
        groups.sort_by(|a, b| b.declared.cmp(&a.declared).then(a.name.cmp(&b.name)));
        let _ = writeln!(out, "\n{verdict}, by size:");
        for group in groups {
            let _ = write!(out, "  {:>4}  {}", group.declared, group.name);
            if let Some(departure) = held.whose.get(&group.name) {
                let _ = write!(
                    out,
                    " — {} ({})",
                    phrase(&departure.whose),
                    departure.clause
                );
            }
            if group.declared <= 3 {
                let _ = write!(out, ": {}", group.pages.join(", "));
            }
            out.push('\n');
        }
    }
    departures(held, &mut out);
    out
}

/// The contradicted pool split by `WHOSE_DEPARTURE`, and the pages a departure of ours holds — the
/// candidates for ADR 1483's next page to take.
fn departures(held: &Held, out: &mut String) {
    let contradicted = held
        .by_verdict
        .get("contradicted")
        .map_or(&[][..], Vec::as_slice);
    let count = |wanted: &str| -> usize {
        contradicted
            .iter()
            .filter(|group| {
                held.whose
                    .get(&group.name)
                    .is_some_and(|departure| departure.whose == wanted)
            })
            .map(|group| group.declared)
            .sum()
    };
    let total: usize = contradicted.iter().map(|group| group.declared).sum();
    let named = count("Ours")
        .saturating_add(count("References"))
        .saturating_add(count("Choice"));
    let _ = writeln!(
        out,
        "\nof the {total} held contradicted page(s), {} are held as a departure of ours, {} as the \
         references' and {} as a choice the clause leaves to the processor; {} in a group \
         WHOSE_DEPARTURE does not name",
        count("Ours"),
        count("References"),
        count("Choice"),
        total.saturating_sub(named)
    );
    let candidates: Vec<String> = contradicted
        .iter()
        .filter_map(|group| {
            let departure = held.whose.get(&group.name)?;
            (departure.whose == "Ours").then(|| {
                group
                    .pages
                    .iter()
                    .map(|page| format!("{page} ({}, {})", group.name, departure.clause))
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect();
    if candidates.is_empty() {
        let _ = writeln!(
            out,
            "no page is held as a departure of ours, so every contradiction left is a reference's \
             or a choice the clause leaves open"
        );
    } else {
        let _ = writeln!(
            out,
            "the next page to take is one of the {} held as a departure of ours — {} — and which \
             is first is the walk's ranking by how far each sits outside its bound (ADR 1483)",
            candidates.len(),
            candidates.join("; ")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, report};

    /// The calibration: each declaration shape the oracle uses — one line, wrapped after `=`, a
    /// list across lines, an empty group — and a `WHOSE_DEPARTURE` row of each width.
    #[test]
    fn each_declaration_shape_is_read_and_the_departures_are_joined_to_their_groups() {
        let text = "\
const DPI: u32 = 72;
const CONTRADICTED_SUBPIXEL_IMAGE: [&str; 1] = [\"a.pdf page 1\"];
const CONTRADICTED_PATTERN_CELL_STATE: [&str; 1] =
    [\"b.pdf page 1\"];
const CONTRADICTED_PAGE_ROUNDING: [&str; 0] = [];
const AMBIGUOUS_WIDE: [&str; 4] = [
    \"c.pdf page 1\",
    \"d.pdf page 1\",
    \"e.pdf page 2\",
    \"f.pdf page 1\",
];
const NOT_COMPARABLE_X: [&str; 1] = [\"g.pdf page 1\"];
const NO_RENDER_Y: [&str; 1] = [\"h.pdf page 1\"];
const REFERENCE_GEOMETRY_Z: [&str; 0] = [];
const GEOMETRY: [&str; 0] = [];
const JUDGED_WITHOUT_A_THIRD_READING: [&str; 1] = [\"i.pdf page 1\"];
const WHOSE_DEPARTURE: &[(&str, Whose, &str)] = &[
    (\"CONTRADICTED_SUBPIXEL_IMAGE\", Whose::Ours, \"\u{a7}10.7.4\"),
    (
        \"CONTRADICTED_PATTERN_CELL_STATE\",
        Whose::References,
        \"\u{a7}8.7.3.1\",
    ),
];
";
        let held = parse(text);
        let groups = |verdict: &str| held.by_verdict.get(verdict).map_or(0, Vec::len);
        assert_eq!(
            (
                groups("contradicted"),
                groups("ambiguous"),
                groups("not comparable"),
                groups("no render"),
                groups("reference geometry"),
                groups("our geometry"),
            ),
            (3, 1, 1, 1, 1, 1)
        );
        let wide = held
            .by_verdict
            .get("ambiguous")
            .and_then(|groups| groups.first())
            .map(|group| group.pages.len());
        assert_eq!(wide, Some(4));
        assert_eq!(held.whose.len(), 2);
        let text = report(&held);
        assert!(text.contains("1 are held as a departure of ours, 1 as the references'"));
        assert!(text.contains("a.pdf page 1 (CONTRADICTED_SUBPIXEL_IMAGE, \u{a7}10.7.4)"));
        assert!(text.contains("  contradicted           2 page(s) in 2 group(s), 3 declared"));
        assert!(
            !text.contains("i.pdf"),
            "a list that is no verdict was counted: {text}"
        );
    }
}
