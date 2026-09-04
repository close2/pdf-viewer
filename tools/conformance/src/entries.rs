//! The fifteenth sweep: who reads the entries a clause states?
//!
//! # The shape it exists for
//!
//! `doc/todo/01` holds fifteen sweeps and fourteen of them read what a ledger row *says* — its
//! blocker, the capability it names, the string a correction retired, the file it points at, the
//! table number it cites, the arithmetic between a parent and its children. None of them can see
//! the sixth refusal shape `doc/habits.md` names, because a row in that shape says nothing wrong:
//! it retires its refusal by naming a **capability that arrived** — "drawn since the Nth
//! session", "this program now has a window" — and then nobody asks whether the *entry* that
//! turns the capability on was ever wired to it.
//!
//! So this sweep reads no reason at all. It takes the entries the clause's own tables state, out
//! of the standard, and asks two questions of each.
//!
//! 1. Does any Rust source under [`crate::SOURCE_ROOTS`] name the entry at all?
//! 2. Does any file the row itself lists in `code = [...]`?
//!
//! **The second question is the sweep.** `/Open` — the entry the four-hundred-and-fifty-ninth
//! session found unread — *is* named under `crates/`, by the popup reader under a different
//! table, so question 1 alone passes the very row this exists for. ADRs 0295 and 0315 are the two
//! findings it has produced: Table 187's required `/FS`, disposed of as "not a rendering
//! question" while §12.5.6.15's own sentence says what activating one of these does, and
//! §12.5.6.2's `/IRT` and `/RT`, disposed of as reaching "a comments pane rather than a raster"
//! while the paragraph four below Table 172 makes them a *group* with nine shared entries.
//!
//! # Why it is a script now
//!
//! It was described in `doc/todo/01` and never committed, so the four-hundred-and-eightieth
//! session rebuilt it from the description before it could run it. A sweep that has to be
//! reconstructed is a sweep that will not be run, and a reconstruction is not the same
//! instrument twice — which is the whole reason this project writes down commands rather than
//! their output (ADR 0281).
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, one population over, and this run is the second document of it: the
//! first run printed 57 entries over 30 rows and one was work; the second 43 over 24, and one was
//! work. Most hits are refusals `CLAUDE.md` already closes — an ECMAScript action's `/DS`, a
//! sound action's four entries, a seed value dictionary's — and a row whose `code` array has gone
//! stale beside a crate that grew a second file reads exactly like a defect and is not one.
//! **It is a reading list.** What decides whether a hit is work is a question no program can ask:
//! whether the row's disposal of the entry is a claim about *the entry* or about *the clause*.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::citation;
use crate::clause::{ClauseIndex, ClauseNumber, Heading};
use crate::ledger::{Ledger, Row, Status};

/// The phrases that put a row in the population: a note explaining itself by an arrival.
///
/// Lower-cased substrings, matched against the note. Each is a way this ledger writes "the thing
/// that was missing is here now", which is the sentence that stops a reader asking the second
/// question. They are deliberately loose — a population that is too large is a longer reading
/// list, and one that is too small is a sweep that cannot see the row it was built for.
pub const ARRIVALS: [&str; 7] = [
    "since the",
    "since adr",
    "since session",
    "now has",
    "now draws",
    "now reads",
    "arrived",
];

/// The two forms this tree writes a PDF dictionary key in.
///
/// A reader names the entry as a string — `document.get_key(annotation, "FS")` — and a comment
/// names it as the standard prints it. Either counts as naming it: the sweep is looking for a
/// key nothing in the tree mentions, and being generous here is what keeps a hit worth reading.
/// The bare word is deliberately **not** a form: `Name`, `Type` and `Border` occur in a hundred
/// sentences that have nothing to do with the clause.
const FORMS: [fn(&str) -> String; 2] = [|key| format!("\"{key}\""), |key| format!("/{key}")];

/// One of the standard's numbered tables, reduced to the entries its first column names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The table's number, as the standard prints it.
    pub number: u16,
    /// The table's title, for a report a person has to read.
    pub title: String,
    /// The keys of its first column, in the order the table prints them.
    pub keys: Vec<String>,
}

/// Where the tree names an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Named {
    /// No Rust source under [`crate::SOURCE_ROOTS`] names it in either of [`FORMS`].
    Nowhere,
    /// Named somewhere, but by no file the row's own `code` array lists.
    ///
    /// This is the answer question 1 alone cannot give, and it is the one the sweep was built
    /// for: another clause's reader naming the same short key is not this clause reading it.
    Elsewhere,
    /// Named by a file the row itself says implements the clause. Nothing is owed.
    ByItsOwnCode,
}

impl fmt::Display for Named {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Nowhere => "named nowhere at all",
            Self::Elsewhere => "named only elsewhere",
            Self::ByItsOwnCode => "named by the row's own code",
        })
    }
}

/// One entry of one table, and where the tree names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The table that states it.
    pub table: u16,
    /// The key itself, without the solidus.
    pub key: String,
    /// Where the tree names it.
    pub named: Named,
    /// Whether the row's own note names the entry.
    ///
    /// The sweep's second run left one question it could not answer — whether a row's disposal
    /// of an entry is a claim about *the entry* or about *the clause* — and this is as far
    /// towards it as a program reaches: a note that never writes the key has made no claim about
    /// it at all. A note that does write it has, and only reading tells whether the claim is
    /// right. Both findings the sweep has produced were the second kind, which is why this
    /// splits the list rather than shortening it.
    pub disposed_of: bool,
}

/// One row of the ledger whose own code does not name every entry its clause states.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The clause whose row it is.
    pub clause: ClauseNumber,
    /// The row's status, because an `implemented` row missing an entry is the sharper case.
    pub status: Status,
    /// The line of `ledger.toml` the row starts on.
    pub line: usize,
    /// The entries the row's own code does not name, in table order.
    pub entries: Vec<Entry>,
}

/// What one run of the sweep read and what it found.
#[derive(Debug, Clone)]
pub struct Report {
    /// How many rows explained themselves by an arrival and named at least one file.
    pub population: usize,
    /// How many rows were in that population but name no code at all, and so cannot be asked
    /// question 2. Counted rather than silently skipped.
    pub without_code: usize,
    /// How many distinct table entries the population's clauses state.
    pub entries: usize,
    /// The rows with at least one entry their own code does not name.
    pub findings: Vec<Finding>,
}

impl Report {
    /// How many entries the run reports, over all its findings.
    #[must_use]
    pub fn reported(&self) -> usize {
        self.findings
            .iter()
            .map(|finding| finding.entries.len())
            .sum()
    }

    /// How many of the reported entries carry `named`.
    #[must_use]
    pub fn counted(&self, named: Named) -> usize {
        self.findings
            .iter()
            .flat_map(|finding| finding.entries.iter())
            .filter(|entry| entry.named == named)
            .count()
    }

    /// How many reported entries the row's own note never names.
    ///
    /// The shortest reading list this sweep produces: an entry the clause states, no file the row
    /// claims reads, and the row's own prose has never written down.
    #[must_use]
    pub fn undisposed(&self) -> usize {
        self.findings
            .iter()
            .flat_map(|finding| finding.entries.iter())
            .filter(|entry| !entry.disposed_of)
            .count()
    }
}

/// Why the sweep could not run.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A source root could not be walked.
    #[error("cannot read the tree's sources: {0}")]
    Sources(#[from] std::io::Error),
}

/// Every Rust source under [`crate::SOURCE_ROOTS`], with its text, paths relative to `root`.
///
/// The checker's own directory is read like any other here: unlike a citation, a key is not
/// something this crate writes as an example, and excluding it would hide a reader if one ever
/// moved in.
///
/// # Errors
///
/// If a directory cannot be walked or a file cannot be read. A sweep that skipped what it could
/// not open would report a clean tree for a tree it had not looked at.
pub fn sources(root: &Path) -> Result<Vec<(PathBuf, String)>, Error> {
    let roots: Vec<PathBuf> = crate::SOURCE_ROOTS
        .iter()
        .map(|name| root.join(name))
        .collect();
    let mut read = Vec::new();
    for path in citation::rust_sources(&roots)? {
        let text = std::fs::read_to_string(&path)?;
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        read.push((relative, text));
    }
    Ok(read)
}

/// Whether a note explains itself by an arrival.
#[must_use]
pub fn is_an_arrival(note: &str) -> bool {
    let note = note.to_ascii_lowercase();
    ARRIVALS.iter().any(|phrase| note.contains(phrase))
}

/// The tables the clause states in its **own** text, subclauses excluded.
///
/// A subclause has a row of its own and owns its own tables; attributing them to the parent as
/// well would print §12.5's row with every annotation table in the standard under it, which is a
/// reading list nobody reads.
#[must_use]
pub fn tables_of(index: &ClauseIndex, number: &ClauseNumber) -> Vec<Table> {
    let mut tables = Vec::new();
    for heading in index
        .headings()
        .iter()
        .filter(|heading| &heading.number == number)
    {
        for table in tables_in(own_text(index, heading)) {
            if !tables
                .iter()
                .any(|seen: &Table| seen.number == table.number)
            {
                tables.push(table);
            }
        }
    }
    tables
}

/// The text belonging to a heading itself, up to the next numbered heading of any kind.
///
/// [`Heading::span`] runs to the end of the clause's *descendants*, which is what a quotation
/// needs and is the opposite of what this sweep needs.
fn own_text<'a>(index: &'a ClauseIndex, heading: &Heading) -> &'a str {
    let end = index
        .headings()
        .iter()
        .map(|later| later.span.start)
        .filter(|start| *start > heading.span.start)
        .min()
        .unwrap_or(heading.span.end)
        .min(heading.span.end);
    index.text_in(heading.span.start..end)
}

/// Every `Table N -Title` in one span of the conversion whose first column is `Key`.
///
/// The conversion writes a caption on a line of its own and the table under it as a Markdown
/// pipe table, so the keys are the first cell of every row after the header and its rule. Three
/// things this has to survive, each met in the conversion rather than imagined:
///
/// - **Not every table has keys.** Table 104's first column is a rendering mode's number and
///   Table 22's is a bit position; a table whose header does not say `Key` states no entries and
///   is skipped rather than mined for whatever its first column holds.
/// - **A caption is sometimes promoted to a heading.** `## Table 246 -Entries in the FDF
///   dictionary` is one; the hashes come off before the caption is matched, which is what
///   [`ClauseIndex::table_title`] does one level up.
/// - **One caption, several blocks.** A table longer than a page comes out of the conversion as a
///   run of pipe tables, each repeating the `| Key |` header, separated by whatever the page
///   break carried — a blank line, a running footer, or a base64 image. So a caption's span runs
///   to the *next caption*, and every block inside it whose header names `Key` states entries;
///   a block whose header does not (Table 92's `Full Name`) states none, and the header is asked
///   again for each block rather than once for the table. **This read the first block only until
///   the five-hundred-and-forty-fifth**, which cost Table 31 twenty-two of its twenty-eight keys
///   and made every citation of one of them a suspect in the ninth sweep.
/// - **The columns shift.** Table 200's rows come out of `doc/md/` with the columns displaced,
///   so four of its five keys read as prose and only one reaches the output. That is a silence
///   in the conversion rather than in the standard, and `doc/HANDOVER.md`'s caveat about
///   `doc/md/` is the answer: check the PDF before believing an absence.
#[must_use]
pub fn tables_in(text: &str) -> Vec<Table> {
    let mut tables: Vec<Table> = Vec::new();
    let mut current: Option<Table> = None;
    let mut column = None::<usize>;
    for line in text.lines() {
        let bare = line.trim_start_matches('#').trim();
        if let Some(table) = bare.strip_prefix("Table ").and_then(caption_of) {
            push(&mut tables, current.take());
            current = Some(table);
            column = None;
            continue;
        }
        let Some(table) = current.as_mut() else {
            continue;
        };
        let Some(cells) = row_of(line) else {
            // A blank line inside a Markdown table does not occur, so anything that is not a row
            // ends the *block*. The table itself ends at the next caption: what follows a block
            // is as often the rest of the same table, one page break later, as it is prose.
            if !line.trim().is_empty() {
                column = None;
            }
            continue;
        };
        match column {
            // Each block's header decides whether that block states entries.
            None => {
                if cells.first().is_some_and(|first| first == "Key") {
                    column = Some(0);
                }
            }
            Some(index) => {
                let Some(cell) = cells.get(index) else {
                    continue;
                };
                if let Some(key) = key_of(cell).filter(|key| !table.keys.contains(key)) {
                    table.keys.push(key);
                }
            }
        }
    }
    push(&mut tables, current);
    tables
}

/// Every table entry's own description, keyed by the table it is in and the key that names it.
///
/// [`tables_in`] answers *which* keys a table states, which is what a citation has to be checked
/// against. This answers what the standard **says about** one of them — the cell that opens
/// `( Optional; PDF 2.0 )` and then describes the value — and it exists for the twenty-fourth
/// sweep, which reads a `partial` row's debt against the modal verb governing it
/// ([`crate::permitted`]).
///
/// Two rules the conversion forces, both the same ones [`tables_in`] obeys:
///
/// - **The description is the row's last cell**, because the standard's entry tables are `Key |
///   Type | Value` and the occasional one is `Key | Value`; the last is the description in both.
/// - **A row whose first cell names no key continues the row above it.** The conversion breaks a
///   long description across several rows with an empty first column, and a description read
///   without them stops mid-sentence — which for this sweep's question is the difference between
///   seeing a `shall` and not.
#[must_use]
pub fn descriptions_in(text: &str) -> BTreeMap<(u16, String), String> {
    let mut described = BTreeMap::new();
    let mut table = None::<u16>;
    let mut column = None::<usize>;
    let mut current = None::<(u16, String)>;
    for line in text.lines() {
        let bare = line.trim_start_matches('#').trim();
        if let Some(caption) = bare.strip_prefix("Table ").and_then(caption_of) {
            table = Some(caption.number);
            column = None;
            current = None;
            continue;
        }
        let Some(number) = table else {
            continue;
        };
        let Some(cells) = row_of(line) else {
            if !line.trim().is_empty() {
                column = None;
                current = None;
            }
            continue;
        };
        let Some(index) = column else {
            if cells.first().is_some_and(|first| first == "Key") {
                column = Some(0);
            }
            continue;
        };
        let Some(cell) = cells.get(index) else {
            continue;
        };
        let Some(description) = cells.last() else {
            continue;
        };
        match key_of(cell) {
            Some(key) => {
                current = Some((number, key.clone()));
                described
                    .entry((number, key))
                    .or_insert_with(|| description.clone());
            }
            None => {
                if let Some(at) = current.as_ref()
                    && let Some(held) = described.get_mut(at)
                {
                    held.push(' ');
                    held.push_str(description);
                }
            }
        }
    }
    described
}

/// Files a finished table if it named any entry at all.
fn push(tables: &mut Vec<Table>, table: Option<Table>) {
    let Some(table) = table else {
        return;
    };
    if !table.keys.is_empty() && !tables.iter().any(|seen| seen.number == table.number) {
        tables.push(table);
    }
}

/// A caption's number and title, from the text after `Table `.
fn caption_of(caption: &str) -> Option<Table> {
    let (number, title) = caption.split_once('-')?;
    let number = number.trim().parse::<u16>().ok()?;
    let title = title.trim();
    if title.is_empty() {
        return None;
    }
    Some(Table {
        number,
        title: title.to_owned(),
        keys: Vec::new(),
    })
}

/// The cells of one Markdown table row, or `None` for a line that is not one.
fn row_of(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    if inner
        .chars()
        .all(|character| matches!(character, '-' | ':' | '|' | ' '))
    {
        // The rule under the header. Not a row, and not the end of the table either.
        return Some(Vec::new());
    }
    Some(
        inner
            .split('|')
            .map(|cell| cell.trim().to_owned())
            .collect(),
    )
}

/// A first-column cell reduced to the key it names, or `None` where it names none.
///
/// The standard sets a key as one word; a cell holding a sentence is a continuation of the row
/// above, which the conversion produces whenever a description wraps.
fn key_of(cell: &str) -> Option<String> {
    let key = cell.trim().trim_end_matches(['*', '†', '‡']);
    let key = key.trim();
    if key.is_empty() || key == "Key" || key.contains(char::is_whitespace) {
        return None;
    }
    if !key.starts_with(|character: char| character.is_ascii_alphabetic()) {
        return None;
    }
    if !key
        .chars()
        .all(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(key.to_owned())
}

/// Runs the sweep over one ledger, one conversion and one tree.
#[must_use]
pub fn sweep(ledger: &Ledger, index: &ClauseIndex, sources: &[(PathBuf, String)]) -> Report {
    let mut report = Report {
        population: 0,
        without_code: 0,
        entries: 0,
        findings: Vec::new(),
    };
    for row in &ledger.rows {
        if !row.note.as_deref().is_some_and(is_an_arrival) {
            continue;
        }
        if row.code.is_empty() {
            report.without_code = report.without_code.saturating_add(1);
            continue;
        }
        report.population = report.population.saturating_add(1);
        let mut entries = Vec::new();
        for table in tables_of(index, &row.clause) {
            for key in &table.keys {
                report.entries = report.entries.saturating_add(1);
                let named = where_named(key, row, sources);
                if named != Named::ByItsOwnCode {
                    entries.push(Entry {
                        table: table.number,
                        key: key.clone(),
                        named,
                        disposed_of: row.note.as_deref().is_some_and(|note| mentions(note, key)),
                    });
                }
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

/// Whether a note names one entry, in either of the two forms a note writes a key in.
///
/// The ledger writes `/Open` and, inside a quotation of the standard, `Open`. The bare word is
/// not a form here for the same reason it is not one in the source: a note that happens to
/// contain the word *Name* has said nothing about `/Name`.
fn mentions(note: &str, key: &str) -> bool {
    let solidus = format!("/{key}");
    note.contains(&solidus)
        || note
            .split(['`', '"', ' ', ',', '.', ';', ':', '\''])
            .any(|word| word == key)
}

/// Whether a `code = [...]` path covers one source file.
///
/// Equality, plus one rule Rust's own module system states: a module root `…/foo.rs` owns
/// everything under `…/foo/`. The four-hundred-and-eighties split three of this tree's largest
/// files into exactly that shape — `content.rs` kept as the root over `content/`,
/// `pdf-viewer.rs` over `pdf-viewer/` — *so that every citation of the path stays valid*, and on
/// this sweep's first run after the splits 34 entries moved from "named by the row's own code"
/// to "named only elsewhere" without one line of the ledger or the readers changing. A row
/// naming a module root is naming the module, and an instrument that read it as one file was
/// measuring the split rather than the tree.
#[must_use]
pub fn covered_by(listed: &str, path: &str) -> bool {
    if listed == path {
        return true;
    }
    listed
        .strip_suffix(".rs")
        .and_then(|stem| path.strip_prefix(stem))
        .is_some_and(|rest| rest.starts_with('/'))
}

/// Where the tree names one key, given the row that claims the clause.
fn where_named(key: &str, row: &Row, sources: &[(PathBuf, String)]) -> Named {
    let needles: Vec<String> = FORMS.iter().map(|form| form(key)).collect();
    let mut anywhere = false;
    for (path, text) in sources {
        if !needles.iter().any(|needle| text.contains(needle.as_str())) {
            continue;
        }
        anywhere = true;
        let named = path.to_string_lossy().replace('\\', "/");
        if row.code.iter().any(|listed| covered_by(listed, &named)) {
            return Named::ByItsOwnCode;
        }
    }
    if anywhere {
        Named::Elsewhere
    } else {
        Named::Nowhere
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The conversion's own shape, with the caption on its own line and the keys in the first
    /// column — `doc/md/`'s Table 187, trimmed.
    const FILE_ATTACHMENT: &str = "\
Table 187 -Additional entries specific to a file attachment annotation

| Key     | Type               | Value                                     |
|---------|--------------------|-------------------------------------------|
| Subtype | name               | (Required) shall be FileAttachment.       |
| FS      | file specification | (Required) The file associated with this. |

## 12.5.6.16 Sound annotations
";

    #[test]
    fn a_tables_keys_are_its_first_column() {
        let tables = tables_in(FILE_ATTACHMENT);
        assert_eq!(tables.len(), 1, "{tables:?}");
        let table = tables.first().expect("one table");
        assert_eq!(table.number, 187);
        assert_eq!(table.keys, ["Subtype", "FS"]);
    }

    /// Table 104's first column is a rendering mode's number, and mining it for entries would
    /// print `0`, `1`, `2` as unread keys of every clause that shows a mode.
    #[test]
    fn a_table_whose_first_column_is_not_a_key_states_no_entries() {
        let text = "\
Table 104 -Text rendering modes

| Mode | Example | Description  |
|------|---------|--------------|
| 0    | x       | Fill text.   |
";
        assert_eq!(tables_in(text), Vec::new());
    }

    /// A table longer than a page arrives as several blocks under one caption, and what separates
    /// them is whatever the page break carried — here the base64 image `doc/md/` puts between
    /// Table 31's first and second blocks. Reading the first block alone cost that table
    /// twenty-two of its keys.
    #[test]
    fn one_captions_blocks_are_one_table() {
        let text = "\
Table 31 -Entries in a page object

| Key      | Type      | Value                    |
|----------|-----------|--------------------------|
| Type     | name      | ( Required ) The type.   |
| MediaBox | rectangle | ( Required ) A rectangle.|

![Image](data:image/png;base64,iVBORw0KGgo)

| Key    | Type    | Value                     |
|--------|---------|---------------------------|
| Rotate | integer | ( Optional ) The degrees. |

## 7.7.3.4 Inheritance of page attributes
";
        let tables = tables_in(text);
        let table = tables.first().expect("one table");
        assert_eq!(tables.len(), 1, "{tables:?}");
        assert_eq!(table.keys, ["Type", "MediaBox", "Rotate"]);
    }

    /// The finding the sweep was built for is a *conjunction*: the entry is named in the tree
    /// and not by the code the row claims. Question one alone passes this row.
    #[test]
    fn an_entry_named_by_another_clauses_reader_is_named_only_elsewhere() {
        let sources = vec![
            (
                PathBuf::from("crates/pdf-model/src/popup.rs"),
                "let open = document.get_key(popup, \"Open\");".to_owned(),
            ),
            (
                PathBuf::from("crates/pdf-model/src/appearance.rs"),
                "// the icon, and nothing else".to_owned(),
            ),
        ];
        let row = Row {
            clause: "12.5.6.4".parse().expect("a clause number"),
            title: "Text annotations".to_owned(),
            status: Status::Implemented,
            code: vec!["crates/pdf-model/src/appearance.rs".to_owned()],
            test: Vec::new(),
            exclusion: None,
            note: Some("drawn since the three-hundred-and-twelfth session".to_owned()),
            line: 1,
        };
        assert_eq!(where_named("Open", &row, &sources), Named::Elsewhere);
        assert_eq!(where_named("Popup", &row, &sources), Named::Nowhere);
    }

    /// A module root owns its directory: `content.rs` was split into `content/` with the root
    /// kept exactly so that a citation of the path stays valid, and a reader that moved into a
    /// submodule is still the row's own code.
    #[test]
    fn a_module_roots_directory_is_its_own_code() {
        assert!(covered_by(
            "crates/pdf-model/src/content.rs",
            "crates/pdf-model/src/content/ext_gstate.rs"
        ));
        assert!(covered_by(
            "crates/pdf-model/src/content.rs",
            "crates/pdf-model/src/content.rs"
        ));
        // A sibling that happens to share the prefix as a *word* is not under the module.
        assert!(!covered_by(
            "crates/pdf-model/src/content.rs",
            "crates/pdf-model/src/content_stream.rs"
        ));
        assert!(!covered_by(
            "crates/pdf-model/src/content.rs",
            "crates/pdf-model/src/popup.rs"
        ));
    }

    /// The row's own reader answers, which is the only outcome that owes nothing.
    #[test]
    fn an_entry_the_rows_own_code_names_is_settled() {
        let sources = vec![(
            PathBuf::from("crates/pdf-model/src/attachment.rs"),
            "document.get_key(annotation, \"FS\")".to_owned(),
        )];
        let row = Row {
            clause: "12.5.6.15".parse().expect("a clause number"),
            title: "File attachment annotations".to_owned(),
            status: Status::Implemented,
            code: vec!["crates/pdf-model/src/attachment.rs".to_owned()],
            test: Vec::new(),
            exclusion: None,
            note: Some(
                "all four are drawn since the two-hundred-and-sixty-sixth session".to_owned(),
            ),
            line: 1,
        };
        assert_eq!(where_named("FS", &row, &sources), Named::ByItsOwnCode);
    }

    /// The population is rows that explain themselves by an arrival; a row that owes something
    /// out loud is every other sweep's subject and not this one's.
    #[test]
    fn the_population_is_a_note_that_names_an_arrival() {
        assert!(is_an_arrival(
            "the popup window /Open selects is drawn since the three-hundred-and-twelfth session"
        ));
        assert!(is_an_arrival("Read since ADR 0295."));
        assert!(!is_an_arrival(
            "What is owed is the value: a text field's /V needs variable text."
        ));
    }
}
