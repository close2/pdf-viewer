//! The seventh sweep: an `inapplicable` or `out-of-scope` row whose own vocabulary, or whose own
//! clause number, the tree names.
//!
//! # The shape it exists for
//!
//! Every other sweep in `doc/todo/01` walks the rows that *owe* something — `partial`,
//! `reported`, `unreviewed` — because those are the rows a reader expects to come back to.
//! `inapplicable` is the status a row goes to when nobody expects to come back, and that is
//! exactly the property that lets a wrong reason live there undisturbed. §14.11.3's printer's
//! marks and §14.11.6.2's trap networks were `inapplicable` because "a screen is not a printer"
//! while `PrinterMark` and `TrapNet` had been in `annotation.rs`'s `STANDARD_SUBTYPES` from the
//! start, and **§12.5.6.20 and §12.5.6.21 said so in their own notes** (ADR 0205). §14.12.4 said
//! Table 409 was unread while `document_part.rs` read it, and its own parent row said the
//! opposite.
//!
//! So the sweep reads no reason at all. It takes the `/Key` names and the capitalised
//! identifiers out of an `inapplicable` row's **own title and note** and asks the tree whether
//! it names them. A row claiming the tree does not do a thing, whose own vocabulary the source
//! names, is a row to read.
//!
//! # The two judgements a program can settle that the grep could not
//!
//! **Rarity, instead of a stop-list.** `doc/todo/01` records the discriminator in one sentence —
//! "most are noise (`DeviceCMYK`, `XObject`, and the sweep's own English — `Nothing`,
//! `Whether`), and the signal is a *rare* word: `GoToDp` in three files under a `§14.12` row,
//! `DPart` in four" — and every hand-run implemented it by writing a stop-list from memory.
//! A stop-list is a guess about which words are common; [`Term::reach`] is the count, taken from
//! the tree the sweep is asking. Terms are printed rarest first and nothing is dropped, so a
//! word somebody would have excluded by hand is demoted rather than hidden.
//!
//! That is also what makes the run's numbers mean anything. The hand-runs recorded 25 of 83, 64
//! of 83, 72 of 83, 27 of 80, 66 of 80, 43 of 80, 61 of 80, 57 of 80 and 47 of 80 over rounds
//! that moved almost no `inapplicable` rows, because each run wrote its own stop-list — a
//! level that cannot be compared with the one before it says nothing at all, which is ADR 0360's
//! argument for the caller sweep arriving one sweep later.
//!
//! **A cousin row, which is the finding this sweep has actually had.** All five defects of its
//! first run were the seventh failure shape — two rows about one mechanism, disagreeing, one
//! naming a *capability* ("a screen is not a printer") and the other naming *code*. Those rows
//! are cousins rather than parent and child, so the arithmetic sweep cannot see them. Here a
//! [`Cousin`] is a row that is **not** `inapplicable` or `out-of-scope` and whose own prose names
//! the same term: the ledger holding both answers at once, printed as the pair it is.
//!
//! # Two statuses, and the row's own clause number
//!
//! `out-of-scope` is the other status nobody expects to come back to, and it decays the same way:
//! twenty-one rows of §13.2 still gave the clause to a media engine while `pdf_model::media` cited
//! §13.2.3.3, §13.2.4.3 and §13.2.7.2.2 by number and tested them, because this sweep read
//! `inapplicable` rows alone (trap 268, ADR 2376). So the population is [`SWEPT`], and every row in
//! it states one term more than its prose does: **its own clause number, as the tree cites it**
//! ([`Kind::Citation`]). An exclusion's notes are written to one template — each names `CLAUDE.md`
//! and the entry, and quotes one sentence of its clause — so their vocabulary is the entry's rather
//! than the clause's, and the citation is the one term such a row cannot fail to state. A source
//! citing a clause whose row says the clause is not this program's is a finding whatever the row's
//! status, which is the trap's rule; [`Report::cited`] counts them.
//!
//! # The noise, printed rather than filtered
//!
//! - **The standard's shared vocabulary.** `DeviceRGB`, `XObject` and `Subtype` are named by
//!   dozens of files under rows about clauses that have nothing to do with them. High reach, so
//!   they sort last.
//! - **A row's own correction.** A note narrating what it used to say reproduces the retired
//!   vocabulary, which is the false positive every sweep in `doc/todo/01` has.
//! - **A term the row itself names as the thing the tree *does*.** An `inapplicable` row is
//!   allowed to say what happens instead — §12.5.6.25's `RichMedia` annotation says its
//!   appearance streams are drawn like any other's — and a witness confirms that sentence rather
//!   than contradicting it.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, unchanged: most of the population is the standard's own words, and
//! no tighter predicate removes them without removing the rare ones too. It runs in a fraction of
//! a second and its output is read.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::clause::ClauseNumber;
use crate::ledger::{Ledger, Row, Status};

/// The forms in which the tree may name a `/Key`.
///
/// The same pair [`crate::entries`] uses and for the same reason: a reader names an entry as a
/// lookup string and a comment names it as the standard prints it, and the bare word is
/// deliberately not a form, because `Name`, `Type` and `Border` occur in a hundred sentences
/// about something else.
const KEY_FORMS: [fn(&str) -> String; 2] = [|key| format!("\"{key}\""), |key| format!("/{key}")];

/// How many witnessing files one term reports.
///
/// A path is what settles whether the naming file is about this clause at all, and three of them
/// is enough to see that — the count itself is [`Term::reach`].
pub const WITNESSES: usize = 3;

/// How many cousin rows one term reports.
pub const COUSINS: usize = 3;

/// The statuses this sweep reads: the two a row goes to when nobody expects to come back to it.
///
/// `inapplicable` says the requirement has no meaning for this device and `out-of-scope` that
/// `CLAUDE.md`'s closed exclusion list holds it, and either decays when the tree grows the thing
/// the row says is not this program's (trap 268, ADR 2376).
pub const SWEPT: [Status; 2] = [Status::Inapplicable, Status::OutOfScope];

/// The shortest identifier worth asking the tree about.
///
/// Three letters is `RGB` and `WKT`; two is the standard's `/N` and `/P`, which every file in the
/// tree contains under some other meaning and which the `/Key` half already covers in its quoted
/// form.
pub const SHORTEST: usize = 3;

/// How a term was written in the row that states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A `/Key` of some table, searched for in `KEY_FORMS`.
    Key,
    /// A capitalised identifier, searched for as a whole word.
    Identifier,
    /// The row's own clause number written as the tree cites one, `§` before it, searched for as
    /// a whole citation: `§13.2.3` is named by `§13.2.3's` and not by `§13.2.3.3`.
    Citation,
}

/// A ledger row that is neither `inapplicable` nor `out-of-scope` and names the same term.
///
/// The seventh failure shape as a pair: one row explaining itself by what this program *is*
/// while another, in a different family, describes the code that does it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cousin {
    /// The clause whose row it is.
    pub clause: ClauseNumber,
    /// What that row claims.
    pub status: Status,
    /// The 1-based line of the ledger it starts on.
    pub line: usize,
}

/// One word out of a swept row's own title and note, or its own clause number, and what the tree
/// says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    /// The word itself, without a solidus.
    pub text: String,
    /// How it was written where the row states it.
    pub kind: Kind,
    /// How many Rust sources name it.
    ///
    /// **The discriminator, and it replaces a stop-list.** A term dozens of files name is the
    /// standard's shared vocabulary; a term one or two name is this clause's own, under a row
    /// saying the tree does not do this.
    pub reach: usize,
    /// Up to [`WITNESSES`] of the naming files, in the order the scan reads them.
    pub witnesses: Vec<String>,
    /// Up to [`COUSINS`] ledger rows that are not [`SWEPT`] and name the same term.
    pub cousins: Vec<Cousin>,
}

/// One swept row with at least one term the tree names.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The clause whose row it is.
    pub clause: ClauseNumber,
    /// Which of the [`SWEPT`] statuses the row holds.
    pub status: Status,
    /// The 1-based line of the ledger it starts on.
    pub line: usize,
    /// The standard's title for the clause, because a title is what makes a term readable.
    pub title: String,
    /// The named terms, rarest first.
    pub terms: Vec<Term>,
}

impl Finding {
    /// The reach of this row's rarest named term, which is what the rows are ordered by.
    #[must_use]
    pub fn rarest(&self) -> usize {
        self.terms.iter().map(|term| term.reach).min().unwrap_or(0)
    }
}

/// What one run of the sweep read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// How many rows the ledger holds under the [`SWEPT`] statuses.
    pub population: usize,
    /// How many of those rows have their own clause cited by a source — trap 268's shape, a
    /// clause the tree executes under a row that says it is not this program's.
    pub cited: usize,
    /// How many distinct terms those rows state, over all of them.
    pub stated: usize,
    /// How many stated terms no Rust source names — the claims this run confirms.
    pub unnamed: usize,
    /// The rows with at least one term the tree names, rarest first.
    pub findings: Vec<Finding>,
}

impl Report {
    /// How many named terms the run reports.
    #[must_use]
    pub fn named(&self) -> usize {
        self.findings.iter().map(|row| row.terms.len()).sum()
    }

    /// How many reported terms carry a cousin row — the pair worth reading first.
    #[must_use]
    pub fn with_cousin(&self) -> usize {
        self.findings
            .iter()
            .flat_map(|row| &row.terms)
            .filter(|term| !term.cousins.is_empty())
            .count()
    }
}

/// Runs the sweep over the ledger's [`SWEPT`] rows and the tree's sources.
///
/// The checker's own directory witnesses nothing, for [`crate::NOT_SCANNED`]'s second reason one
/// step along: this module's documentation quotes the very rows it exists to find, so scanning it
/// would witness every one of them against itself.
#[must_use]
pub fn sweep(ledger: &Ledger, sources: &[(PathBuf, String)]) -> Report {
    let scanned: Vec<(String, &String)> = sources
        .iter()
        .map(|(path, text)| (path.to_string_lossy().replace('\\', "/"), text))
        .filter(|(path, _)| !path.starts_with(crate::NOT_SCANNED))
        .collect();
    let mut report = Report::default();
    for row in &ledger.rows {
        if !SWEPT.contains(&row.status) {
            continue;
        }
        report.population = report.population.saturating_add(1);
        let mut terms = Vec::new();
        let citation = (format!("\u{a7}{}", row.clause), Kind::Citation);
        for (text, kind) in std::iter::once(citation).chain(terms_of(row)) {
            report.stated = report.stated.saturating_add(1);
            let (reach, witnesses) = named_by(&text, kind, &scanned);
            if reach == 0 {
                report.unnamed = report.unnamed.saturating_add(1);
                continue;
            }
            if kind == Kind::Citation {
                report.cited = report.cited.saturating_add(1);
            }
            terms.push(Term {
                cousins: cousins_of(&text, kind, row, ledger),
                text,
                kind,
                reach,
                witnesses,
            });
        }
        if terms.is_empty() {
            continue;
        }
        terms.sort_by_key(|term| term.reach);
        report.findings.push(Finding {
            clause: row.clause.clone(),
            status: row.status,
            line: row.line,
            title: row.title.clone(),
            terms,
        });
    }
    report.findings.sort_by_key(Finding::rarest);
    report
}

/// The terms one row's prose states, in the order they are first written, without repeats.
///
/// The row's *own* title and note and nothing else: the point of this sweep is that the row
/// supplies both the claim and the vocabulary to test it with. The row's clause number is
/// [`sweep`]'s to add, not this function's, because the fourteenth sweep reads the same prose for
/// the names a debt would have and a citation is no such name.
#[must_use]
pub fn terms_of(row: &Row) -> Vec<(String, Kind)> {
    let mut seen = BTreeSet::new();
    let mut terms = Vec::new();
    let prose = format!("{} {}", row.title, row.note.as_deref().unwrap_or_default());
    for (text, kind) in keys(&prose).into_iter().chain(identifiers(&prose)) {
        if seen.insert(text.clone()) {
            terms.push((text, kind));
        }
    }
    terms
}

/// Every `/Key` in a piece of prose, without the solidus.
fn keys(prose: &str) -> Vec<(String, Kind)> {
    let mut keys = Vec::new();
    let bytes: Vec<char> = prose.chars().collect();
    let mut at = 0usize;
    while at < bytes.len() {
        if bytes[at] == '/'
            && bytes
                .get(at.saturating_add(1))
                .is_some_and(char::is_ascii_alphabetic)
        {
            let start = at.saturating_add(1);
            let mut end = start;
            while bytes.get(end).is_some_and(char::is_ascii_alphanumeric) {
                end = end.saturating_add(1);
            }
            keys.push((bytes[start..end].iter().collect(), Kind::Key));
            at = end;
        } else {
            at = at.saturating_add(1);
        }
    }
    keys
}

/// Every capitalised identifier of [`SHORTEST`] characters or more, `::` paths kept whole.
///
/// **Two derived rules and no word list**, which is the other half of replacing the hand-runs'
/// stop-lists. English capitalises a word for one reason this prose uses constantly — a sentence
/// begins — and the standard's identifiers are recognisable without knowing any English:
///
/// - a capital that is **not the first letter** makes a word an identifier wherever it stands,
///   which is every camel case, acronym, `_` and `::` the standard writes — `DPart`, `TrapNet`,
///   `GoToDp`, `RGB`, `STANDARD_SUBTYPES`, `Command::Zoom` — and which no English word has;
/// - a plain `Capitalised` word is an identifier only where it does **not** open a sentence, so
///   §12.5.6.25's "a `Rendition` annotation" is one and "Nothing is read here" is not.
///
/// Neither rule knows which words are common; that is [`Term::reach`]'s job, and a plain word
/// that survives rule two sorts last on its own reach rather than being guessed at.
fn identifiers(prose: &str) -> Vec<(String, Kind)> {
    let mut found = Vec::new();
    let mut word = String::new();
    let mut opening = true;
    for character in prose.chars() {
        if character.is_ascii_alphanumeric() || character == '_' || character == ':' {
            word.push(character);
            continue;
        }
        push_identifier(&mut found, &word, opening);
        if !word.is_empty() {
            opening = false;
        }
        word.clear();
        if matches!(character, '.' | '!' | '?' | '\n' | ';' | ':') {
            opening = true;
        }
    }
    push_identifier(&mut found, &word, opening);
    found
}

/// Adds `word` to `found` where it is a capitalised identifier worth asking about.
///
/// `opening` says the word stands where a sentence begins, which is where English capitalises a
/// word it would not otherwise.
fn push_identifier(found: &mut Vec<(String, Kind)>, word: &str, opening: bool) {
    let trimmed = word.trim_matches(':');
    let shaped = trimmed
        .chars()
        .skip(1)
        .any(|character| character.is_ascii_uppercase() || character == '_' || character == ':');
    if trimmed.len() >= SHORTEST
        && trimmed.starts_with(|first: char| first.is_ascii_uppercase())
        && !trimmed.contains(":::")
        && (shaped || !opening)
    {
        found.push((trimmed.to_owned(), Kind::Identifier));
    }
}

/// How many sources name a term, and up to [`WITNESSES`] of them.
fn named_by(text: &str, kind: Kind, sources: &[(String, &String)]) -> (usize, Vec<String>) {
    let mut reach = 0usize;
    let mut witnesses = Vec::new();
    for (path, source) in sources {
        if !names(source, text, kind) {
            continue;
        }
        reach = reach.saturating_add(1);
        if witnesses.len() < WITNESSES {
            witnesses.push(path.clone());
        }
    }
    (reach, witnesses)
}

/// Whether one text names a term: a key in either of `KEY_FORMS`, an identifier as a whole
/// word, a citation as a whole clause number.
///
/// Case-sensitive, because these are the standard's identifiers rather than English: `DPart` is
/// a document part and `dpart` is nothing.
#[must_use]
pub fn names(text: &str, term: &str, kind: Kind) -> bool {
    match kind {
        Kind::Key => KEY_FORMS
            .iter()
            .any(|form| text.contains(form(term).as_str())),
        Kind::Identifier => whole_word(text, term),
        Kind::Citation => whole_citation(text, term),
    }
}

/// Whether `text` holds the citation `term` with nothing after it that continues a clause number:
/// no digit, and no full stop followed by one, so a sentence ending `§13.2.3.` still cites it.
fn whole_citation(text: &str, term: &str) -> bool {
    let mut from = 0usize;
    while let Some(found) = text[from..].find(term) {
        let end = from.saturating_add(found).saturating_add(term.len());
        let mut after = text[end..].chars();
        let continues = match after.next() {
            Some(character) if character.is_ascii_digit() => true,
            Some('.') => after.next().is_some_and(|next| next.is_ascii_digit()),
            _ => false,
        };
        if !continues {
            return true;
        }
        from = end;
    }
    false
}

/// Whether `text` holds `term` bounded on both sides by something that does not continue an
/// identifier.
fn whole_word(text: &str, term: &str) -> bool {
    let mut from = 0usize;
    while let Some(found) = text[from..].find(term) {
        let at = from.saturating_add(found);
        let end = at.saturating_add(term.len());
        let before = text[..at].chars().next_back();
        let after = text[end..].chars().next();
        let bounded = |character: Option<char>| {
            character.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_')
        };
        if bounded(before) && bounded(after) {
            return true;
        }
        from = end;
    }
    false
}

/// The rows that are not [`SWEPT`] and whose own prose names the term.
fn cousins_of(text: &str, kind: Kind, row: &Row, ledger: &Ledger) -> Vec<Cousin> {
    ledger
        .rows
        .iter()
        .filter(|other| other.clause != row.clause && !SWEPT.contains(&other.status))
        .filter(|other| {
            let prose = format!(
                "{} {}",
                other.title,
                other.note.as_deref().unwrap_or_default()
            );
            names(&prose, text, kind)
        })
        .take(COUSINS)
        .map(|other| Cousin {
            clause: other.clause.clone(),
            status: other.status,
            line: other.line,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(clause: &str, status: Status, title: &str, note: &str) -> Row {
        Row {
            clause: clause.parse().expect("a clause number"),
            title: title.to_owned(),
            status,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    fn source(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_owned())
    }

    /// The vocabulary comes out of the row's own two fields, keys first and without repeats.
    #[test]
    fn a_rows_own_title_and_note_are_the_vocabulary() {
        let stated = terms_of(&row(
            "14.11.3",
            Status::Inapplicable,
            "Printer's marks",
            "A PrinterMark annotation states /MN, and a PrinterMark is not a screen's business.",
        ));
        assert!(stated.contains(&("MN".to_owned(), Kind::Key)));
        assert!(stated.contains(&("PrinterMark".to_owned(), Kind::Identifier)));
        assert_eq!(
            stated
                .iter()
                .filter(|(text, _)| text == "PrinterMark")
                .count(),
            1,
            "a term is stated once however often the row writes it"
        );
    }

    /// The population is the two statuses nothing else sweeps, and a row that owes is not in it.
    #[test]
    fn only_the_rows_nobody_expects_to_come_back_to_are_swept() {
        let ledger = Ledger {
            rows: vec![
                row(
                    "12.5.6.20",
                    Status::Partial,
                    "Printer's marks",
                    "PrinterMark is drawn.",
                ),
                row(
                    "14.11.3",
                    Status::Inapplicable,
                    "Printer's marks",
                    "PrinterMark is a press's.",
                ),
                row(
                    "13.2.7.2.2",
                    Status::OutOfScope,
                    "Algorithm: Media Player",
                    "A PrinterMark is a media engine's.",
                ),
            ],
        };
        let sources = vec![source(
            "crates/pdf-model/src/annotation.rs",
            "PrinterMark,\n",
        )];
        let report = sweep(&ledger, &sources);
        assert_eq!(report.population, 2);
        let swept: Vec<(String, Status)> = report
            .findings
            .iter()
            .map(|finding| (finding.clause.to_string(), finding.status))
            .collect();
        assert!(swept.contains(&("14.11.3".to_owned(), Status::Inapplicable)));
        assert!(swept.contains(&("13.2.7.2.2".to_owned(), Status::OutOfScope)));
        assert_eq!(swept.len(), 2);
    }

    /// Trap 268's shape, planted: an `out-of-scope` row whose prose names nothing the tree has,
    /// over a clause a source cites and tests, is named by its own clause number, and the
    /// implemented row that cites the same clause is printed beside it as its cousin.
    #[test]
    fn an_out_of_scope_row_whose_clause_the_tree_cites_is_named_by_the_citation() {
        let ledger = Ledger {
            rows: vec![
                row(
                    "13.2.2",
                    Status::Implemented,
                    "Viability",
                    "The judge walks \u{a7}13.2.3.3's rendition tree.",
                ),
                row(
                    "13.2.3",
                    Status::OutOfScope,
                    "Renditions",
                    "a heading, and a media engine's.",
                ),
                row(
                    "13.2.3.3",
                    Status::OutOfScope,
                    "Selector renditions",
                    "the renditions should be ordered by preference, a media engine's.",
                ),
            ],
        };
        let sources = vec![source(
            "crates/pdf-model/src/media.rs",
            "// \u{a7}13.2.3.3: the first viable media rendition shall be played.\n",
        )];
        let report = sweep(&ledger, &sources);
        assert_eq!(report.population, 2);
        assert_eq!(
            report.cited, 1,
            "\u{a7}13.2.3 is not cited by \u{a7}13.2.3.3"
        );
        assert_eq!(report.findings.len(), 1);
        let finding = &report.findings[0];
        assert_eq!(finding.clause.to_string(), "13.2.3.3");
        assert_eq!(finding.status, Status::OutOfScope);
        let term = &finding.terms[0];
        assert_eq!(term.kind, Kind::Citation);
        assert_eq!(
            term.witnesses,
            vec!["crates/pdf-model/src/media.rs".to_owned()]
        );
        assert_eq!(term.cousins.len(), 1);
        assert_eq!(term.cousins[0].clause.to_string(), "13.2.2");
    }

    /// A citation is the whole clause number: a deeper subclause does not cite its parent, and a
    /// possessive or a sentence's full stop does not stop a citation being one.
    #[test]
    fn a_citation_is_a_whole_clause_number() {
        let cites = |text: &str| names(text, "\u{a7}13.2.3", Kind::Citation);
        assert!(cites("walks \u{a7}13.2.3's tree"));
        assert!(cites("as \u{a7}13.2.3."));
        assert!(cites("(\u{a7}13.2.3)"));
        assert!(!cites("\u{a7}13.2.3.3 and \u{a7}13.2.30"));
        assert!(cites("\u{a7}13.2.3.3 and then \u{a7}13.2.3 itself"));
        assert!(!cites("13.2.3 without its sign"));
    }

    /// A row whose vocabulary nothing names is the claim holding, and it is counted rather than
    /// printed.
    #[test]
    fn a_term_nothing_names_confirms_the_row() {
        let ledger = Ledger {
            rows: vec![row(
                "14.11.6.2",
                Status::Inapplicable,
                "Trap networks",
                "A TrapNet is a press's business.",
            )],
        };
        let report = sweep(
            &ledger,
            &[source("crates/pdf-model/src/lib.rs", "fn draw() {}\n")],
        );
        assert!(report.findings.is_empty());
        assert_eq!(report.unnamed, report.stated);
    }

    /// The rarest term sorts first, which is the judgement that replaces a stop-list.
    #[test]
    fn the_rarest_term_comes_first() {
        let ledger = Ledger {
            rows: vec![row(
                "14.12.4",
                Status::Inapplicable,
                "Document parts",
                "Neither DPart nor XObject reaches a screen.",
            )],
        };
        let sources = vec![
            source("crates/a.rs", "XObject DPart\n"),
            source("crates/b.rs", "XObject\n"),
            source("crates/c.rs", "XObject\n"),
        ];
        let report = sweep(&ledger, &sources);
        let terms = &report.findings[0].terms;
        assert_eq!(terms[0].text, "DPart");
        assert_eq!(terms[0].reach, 1);
        assert_eq!(terms[1].text, "XObject");
        assert_eq!(terms[1].reach, 3);
        assert_eq!(terms[0].witnesses, vec!["crates/a.rs".to_owned()]);
    }

    /// A row that is not `inapplicable` and names the same term is the seventh failure shape,
    /// printed as the pair it is; another `inapplicable` row is not.
    #[test]
    fn a_settled_row_naming_the_same_term_is_a_cousin() {
        let ledger = Ledger {
            rows: vec![
                row(
                    "14.11.3",
                    Status::Inapplicable,
                    "Printer's marks",
                    "PrinterMark is a press's.",
                ),
                row(
                    "12.5.6.20",
                    Status::Partial,
                    "Printer's mark annotations",
                    "PrinterMark is in STANDARD_SUBTYPES.",
                ),
                row(
                    "14.11.6.2",
                    Status::Inapplicable,
                    "Trap networks",
                    "PrinterMark again.",
                ),
            ],
        };
        let sources = vec![source(
            "crates/pdf-model/src/annotation.rs",
            "PrinterMark,\n",
        )];
        let report = sweep(&ledger, &sources);
        let cousins = &report.findings[0].terms[0].cousins;
        assert_eq!(cousins.len(), 1);
        assert_eq!(cousins[0].clause.to_string(), "12.5.6.20");
        assert_eq!(cousins[0].status, Status::Partial);
    }

    /// A key is named in a lookup string or as the standard prints it, and never as a bare word:
    /// `/MN` is a printer's mark and `MN` is two letters of English.
    #[test]
    fn a_key_is_named_in_either_form_and_not_as_a_bare_word() {
        assert!(names("get_key(dict, \"MN\")", "MN", Kind::Key));
        assert!(names("Table 398's /MN", "MN", Kind::Key));
        assert!(!names("the MN of it", "MN", Kind::Key));
    }

    /// An identifier is a whole word and case-sensitive: `DPart` is not `DPartRoot` and not
    /// `dpart`.
    #[test]
    fn an_identifier_is_a_whole_word_and_case_sensitive() {
        assert!(names(
            "let part = DPart::read();",
            "DPart",
            Kind::Identifier
        ));
        assert!(!names("DPartRoot", "DPart", Kind::Identifier));
        assert!(!names("dpart", "DPart", Kind::Identifier));
    }

    /// Two letters are not an identifier and a `::` path is kept whole, so a row naming
    /// `Command::Zoom` asks about that rather than about `Command`.
    #[test]
    fn a_path_is_one_identifier_and_two_letters_are_none() {
        let found = identifiers("An RGB Command::Zoom and an It.");
        let words: Vec<&str> = found.iter().map(|(text, _)| text.as_str()).collect();
        assert!(words.contains(&"RGB"));
        assert!(words.contains(&"Command::Zoom"));
        assert!(!words.contains(&"It"));
    }

    /// The checker's own directory witnesses nothing: this module's documentation names every
    /// row the sweep exists to find.
    #[test]
    fn the_checkers_own_documentation_is_not_a_witness() {
        let ledger = Ledger {
            rows: vec![row(
                "14.11.3",
                Status::Inapplicable,
                "Printer's marks",
                "A PrinterMark is a press's business.",
            )],
        };
        let sources = vec![source(
            "tools/conformance/src/inapplicable.rs",
            "//! PrinterMark is the standing example.\n",
        )];
        assert!(sweep(&ledger, &sources).findings.is_empty());
    }
}
