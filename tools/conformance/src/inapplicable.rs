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
//! # The tables the clause's own text names
//!
//! A clause whose subject is a dictionary is executed under that dictionary's **table**, not under
//! the clause's number: `media.rs` judged §13.2.4.1's clip by "Table 284's `/S`" and §13.2.6.1's
//! screen parameters by Table 294's `MH`, and walked §13.2.7.4.2's software identifier as
//! "§13.2.7.4's software identifier", so none of the three was named by its citation and all three
//! were found by hand (ADR 2376). So a row also states the terms of **the tables its own clause
//! names in `doc/md/`** — its own text, subclauses excluded, read under the row's own number
//! rather than any word of its note (trap 272): every table the text captions or cites by number,
//! and every table whose caption's subject the text names as a dictionary, which is how
//! §13.2.7.4.2's "a software identifier dictionary" reaches Table 303 under §13.2.7.4.1. Each such
//! table is two terms ([`Kind::Table`] and [`Kind::Subject`]): its number as a source cites one,
//! and the caption's subject — "Entries in a **software identifier** dictionary" — as a source's
//! prose writes it, because a reader names a dictionary by what it is as often as by its table.
//! [`Report::tabled`] counts the rows a source names that way (ADR 2410).
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
//! - **A table the clause names for another clause's sake.** A clause citing a neighbour's table
//!   to say what its own entry holds states that table's terms too; the witness path says which
//!   dictionary the naming file reads.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, unchanged: most of the population is the standard's own words, and
//! no tighter predicate removes them without removing the rare ones too. It runs in a fraction of
//! a second and its output is read.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::clause::{self, ClauseIndex, ClauseNumber};
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
    /// A table the row's own clause names in `doc/md/`, written `Table 284` and searched for as a
    /// whole designation: `Table 28` is not named by `Table 284`.
    Table,
    /// The subject of such a table's caption — `media clip` out of "Entries common to all media
    /// clip dictionaries" — searched for as a whole phrase in a source's prose, case and line
    /// breaks and comment markers folded away ([`fold`]).
    Subject,
}

/// One caption of the standard, as this sweep reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caption {
    /// The table's designation, `284` or `Annex O.3`.
    pub designation: String,
    /// What the table's entries belong to, lower-cased — `software identifier` out of "Entries in
    /// a software identifier dictionary" — or `None` where the title names no dictionary.
    pub subject: Option<String>,
}

/// The fewest words a caption's subject needs to be asked about.
///
/// One word is the tree's own English — `viewport`, `measure`, `rendition`, `border` name a
/// hundred things a viewer has that are not the table — while two are a phrase only the
/// dictionary's readers write; a one-word subject's table is still asked about by its number.
pub const SUBJECT_WORDS: usize = 2;

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
    /// How many of those rows have a table their own clause names, by number or by its caption's
    /// subject, named by a source — the clause executed under its dictionary's table (ADR 2410).
    pub tabled: usize,
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
///
/// `index` is the standard's conversion, read for the tables each row's own clause names
/// ([`tables_named`]).
#[must_use]
pub fn sweep(ledger: &Ledger, index: &ClauseIndex, sources: &[(PathBuf, String)]) -> Report {
    let scanned: Vec<Scanned<'_>> = sources
        .iter()
        .map(|(path, text)| (path.to_string_lossy().replace('\\', "/"), text))
        .filter(|(path, _)| !path.starts_with(crate::NOT_SCANNED))
        .map(|(path, text)| Scanned {
            folded: fold(text),
            path,
            text,
        })
        .collect();
    let standard = captions(index);
    let mut report = Report::default();
    for row in &ledger.rows {
        if !SWEPT.contains(&row.status) {
            continue;
        }
        report.population = report.population.saturating_add(1);
        let mut terms = Vec::new();
        let mut tabled = false;
        let citation = (format!("\u{a7}{}", row.clause), Kind::Citation);
        let stated = std::iter::once(citation)
            .chain(tables_named(index, &row.clause, &standard))
            .chain(terms_of(row));
        for (text, kind) in stated {
            report.stated = report.stated.saturating_add(1);
            let (reach, witnesses) = named_by(&text, kind, &scanned);
            if reach == 0 {
                report.unnamed = report.unnamed.saturating_add(1);
                continue;
            }
            if kind == Kind::Citation {
                report.cited = report.cited.saturating_add(1);
            }
            tabled |= matches!(kind, Kind::Table | Kind::Subject);
            terms.push(Term {
                cousins: cousins_of(&text, kind, row, ledger),
                text,
                kind,
                reach,
                witnesses,
            });
        }
        if tabled {
            report.tabled = report.tabled.saturating_add(1);
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

/// One Rust source as the sweep reads it.
struct Scanned<'a> {
    /// Its path, relative to the root and with forward solidi.
    path: String,
    /// Its text, as written.
    text: &'a str,
    /// Its text as [`fold`] gives it, for [`Kind::Subject`], folded once rather than per term.
    folded: String,
}

/// Every caption in the standard, each designation once, in document order.
#[must_use]
pub fn captions(index: &ClauseIndex) -> Vec<Caption> {
    let mut found: Vec<Caption> = Vec::new();
    for heading in index.headings() {
        for line in crate::entries::own_text(index, heading).lines() {
            let Some((designation, title)) = clause::caption_of(line) else {
                continue;
            };
            if found.iter().any(|seen| seen.designation == designation) {
                continue;
            }
            found.push(Caption {
                designation: designation.to_owned(),
                subject: subject_of(title),
            });
        }
    }
    found
}

/// What a caption's entries belong to, lower-cased, where its title names a dictionary.
///
/// The standard writes an entry table's title in a handful of shapes — "Entries in a …
/// dictionary", "Additional entries specific to an … dictionary", "Entries common to all …
/// dictionaries" — and every one puts the subject between the first article and the word
/// *dictionary*. A title that is not about entries ("Monitor specifier values") or names no
/// dictionary ("Entries for specific types of requirements") has no subject.
#[must_use]
pub fn subject_of(title: &str) -> Option<String> {
    let title = title.to_lowercase();
    let words: Vec<&str> = title.split_whitespace().collect();
    if !words.contains(&"entries") {
        return None;
    }
    let opening = words
        .iter()
        .position(|word| matches!(*word, "a" | "an" | "the" | "all"))?;
    let closing = words
        .iter()
        .position(|word| matches!(*word, "dictionary" | "dictionaries"))?;
    let subject = words.get(opening.saturating_add(1)..closing)?;
    if subject.is_empty() {
        return None;
    }
    Some(subject.join(" "))
}

/// The terms of every table the clause `number` names in its own text, subclauses excluded.
///
/// A table is named where the text captions it or cites it by number (`Table 284`, as a whole
/// designation), or names its caption's subject as a dictionary — "a software identifier
/// dictionary". Each table states its number ([`Kind::Table`]) and, where its subject has
/// [`SUBJECT_WORDS`] words or more, the subject ([`Kind::Subject`]); each term once.
#[must_use]
pub fn tables_named(
    index: &ClauseIndex,
    number: &ClauseNumber,
    standard: &[Caption],
) -> Vec<(String, Kind)> {
    let own: Vec<&str> = index
        .headings()
        .iter()
        .filter(|heading| &heading.number == number)
        .map(|heading| crate::entries::own_text(index, heading))
        .collect();
    let folded: Vec<String> = own.iter().map(|text| fold(text)).collect();
    let mut terms: Vec<(String, Kind)> = Vec::new();
    for caption in standard {
        let table = format!("Table {}", caption.designation);
        let cited = own.iter().any(|text| whole_citation(text, &table));
        let as_dictionary = caption.subject.as_ref().is_some_and(|subject| {
            folded.iter().any(|text| {
                whole_word(text, &format!("{subject} dictionary"))
                    || whole_word(text, &format!("{subject} dictionaries"))
            })
        });
        if !cited && !as_dictionary {
            continue;
        }
        let subject = caption
            .subject
            .clone()
            .filter(|subject| subject.split(' ').count() >= SUBJECT_WORDS)
            .map(|subject| (subject, Kind::Subject));
        for term in std::iter::once((table, Kind::Table)).chain(subject) {
            if !terms.contains(&term) {
                terms.push(term);
            }
        }
    }
    terms
}

/// A text as prose: lower-cased, each comment marker opening a line dropped, every run of
/// whitespace one space.
///
/// A doc comment breaks "software identifier" across `///` lines as readily as a sentence of the
/// standard breaks across a page, and a phrase is asked about as a reader writes it.
#[must_use]
pub fn fold(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line.trim_start();
        let line = line
            .strip_prefix("//!")
            .or_else(|| line.strip_prefix("///"))
            .or_else(|| line.strip_prefix("//"))
            .unwrap_or(line);
        for word in line.split_whitespace() {
            if !folded.is_empty() {
                folded.push(' ');
            }
            folded.push_str(&word.to_lowercase());
        }
    }
    folded
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
fn named_by(text: &str, kind: Kind, sources: &[Scanned<'_>]) -> (usize, Vec<String>) {
    let mut reach = 0usize;
    let mut witnesses = Vec::new();
    for source in sources {
        let named = match kind {
            Kind::Subject => whole_word(&source.folded, text),
            _ => names(source.text, text, kind),
        };
        if !named {
            continue;
        }
        reach = reach.saturating_add(1);
        if witnesses.len() < WITNESSES {
            witnesses.push(source.path.clone());
        }
    }
    (reach, witnesses)
}

/// Whether one text names a term: a key in either of `KEY_FORMS`, an identifier as a whole
/// word, a citation or a table as a whole designation, a caption's subject as a whole phrase of
/// the text's [`fold`].
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
        Kind::Table => whole_table(text, term),
        Kind::Subject => whole_word(&fold(text), term),
    }
}

/// Whether `text` holds the citation `term` with nothing after it that continues a clause number:
/// no digit, and no full stop followed by one, so a sentence ending `§13.2.3.` still cites it.
fn whole_citation(text: &str, term: &str) -> bool {
    whole_designation(text, term, |_| true)
}

/// Whether `text` cites the table `term` by ISO 32000-2's number: a whole designation, as
/// [`whole_citation`] reads one, that no `32000-1` within [`OTHER_EDITION_REACH`] bytes before it
/// gives to the other edition — `structure.rs`'s "ISO 32000-1's Table 334" is a heading level, and
/// this edition's Table 334 a `RichMediaSettings` dictionary.
fn whole_table(text: &str, term: &str) -> bool {
    whole_designation(text, term, |at| {
        let window = &text.as_bytes()[at.saturating_sub(OTHER_EDITION_REACH)..at];
        !window
            .windows(OTHER_EDITION.len())
            .any(|seen| seen == OTHER_EDITION)
    })
}

/// The edition a table number belongs to when a sentence says so, and this sweep's are not its.
const OTHER_EDITION: &[u8] = b"32000-1";

/// How far before a table number `OTHER_EDITION` is looked for: "ISO 32000-1:2008's Table" puts
/// it fifteen bytes back.
pub const OTHER_EDITION_REACH: usize = 16;

/// Whether `text` holds `term` with nothing after it that continues a number, at a position
/// `admits` accepts.
fn whole_designation(text: &str, term: &str, admits: impl Fn(usize) -> bool) -> bool {
    let mut from = 0usize;
    while let Some(found) = text[from..].find(term) {
        let at = from.saturating_add(found);
        let end = at.saturating_add(term.len());
        let mut after = text[end..].chars();
        let continues = match after.next() {
            Some(character) if character.is_ascii_digit() => true,
            Some('.') => after.next().is_some_and(|next| next.is_ascii_digit()),
            _ => false,
        };
        if !continues && admits(at) {
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

    /// A conversion captioning no table, for the tests about a row's own prose.
    fn no_standard() -> ClauseIndex {
        ClauseIndex::parse("## 1 Scope\nThis document specifies a format.\n".to_owned())
    }

    /// ADR 2376's three missed rows, planted as they stood: each clause is executed under its
    /// dictionary's table — Table 284's `/S`, Table 294's `MH`, and §13.2.7.4.2's software
    /// identifier dictionary, whose table §13.2.7.4.1 captions — and no source cites the row's own
    /// number, so only the tables its clause names can list it.
    #[test]
    fn a_clause_executed_under_its_tables_number_or_subject_is_listed() {
        let standard = ClauseIndex::parse(
            "## 13.2.4.1 General\nA media clip object shall be a dictionary.\n\n\
             Table 284 -Entries common to all media clip dictionaries\n\n\
             | Key | Type | Value |\n|---|---|---|\n| S | name | (Required) |\n\n\
             ## 13.2.4.2 Media clip data\nOther text.\n\n\
             ## 13.2.6.1 General\nScreen parameters.\n\n\
             Table 293 -Entries in a media screen parameters dictionary\n\n\
             Table 294 -Entries in a media screen parameters MH/BE dictionary\n\n\
             ## 13.2.7.4.1 General\nIts entries are listed in Table 303.\n\n\
             Table 303 -Entries in a software identifier dictionary\n\n\
             ## 13.2.7.4.2 Algorithm: software identifier\n\
             The following procedure shall be used to determine whether a piece of software is \
             considered to match a software identifier dictionary:\n\n\
             ## 13.2.7.4.3 Software URIs\nThe U entry is a URI.\n"
                .to_owned(),
        );
        let template = "CLAUDE.md's clause 13 exclusion holds it: a media engine's.";
        let ledger = Ledger {
            rows: vec![
                row("13.2.4.1", Status::OutOfScope, "General", template),
                row("13.2.6.1", Status::OutOfScope, "General", template),
                row(
                    "13.2.7.4.2",
                    Status::OutOfScope,
                    "Algorithm: software identifier",
                    template,
                ),
            ],
        };
        let sources = vec![source(
            "crates/pdf-model/src/media.rs",
            "// Table 284's /S: the clip is non-viable.\n\
             let screen = get_key(dict, \"SP\"); // Table 294's MH\n\
             /// a player is found only by \u{a7}13.2.7.4's software\n\
             /// identifier.\n",
        )];
        let report = sweep(&ledger, &standard, &sources);
        assert_eq!(report.cited, 0, "no source cites the rows' own numbers");
        assert_eq!(report.tabled, 3);
        let first = |clause: &str| -> (String, Kind) {
            let finding = report
                .findings
                .iter()
                .find(|finding| finding.clause.to_string() == clause)
                .expect("the row is listed");
            (finding.terms[0].text.clone(), finding.terms[0].kind)
        };
        assert_eq!(first("13.2.4.1").1, Kind::Table);
        assert_eq!(first("13.2.4.1").0, "Table 284");
        assert_eq!(first("13.2.6.1").0, "Table 294");
        assert_eq!(
            first("13.2.7.4.2"),
            ("software identifier".to_owned(), Kind::Subject)
        );
    }

    /// A caption's subject is what lies between its first article and *dictionary*, and a title
    /// that names no dictionary has none.
    #[test]
    fn a_captions_subject_is_what_its_entries_belong_to() {
        assert_eq!(
            subject_of("Entries in a software identifier dictionary").as_deref(),
            Some("software identifier")
        );
        assert_eq!(
            subject_of("Entries common to all media clip dictionaries").as_deref(),
            Some("media clip")
        );
        assert_eq!(
            subject_of("Additional entries specific to a sound object"),
            None
        );
        assert_eq!(subject_of("Monitor specifier values"), None);
    }

    /// A table is a whole designation, and a subject a whole phrase across a comment's line break.
    #[test]
    fn a_table_is_a_whole_designation_and_a_subject_a_folded_phrase() {
        assert!(names("Table 284's /S", "Table 284", Kind::Table));
        assert!(!names("Table 284's /S", "Table 28", Kind::Table));
        assert!(!names(
            "level six (ISO 32000-1's Table 334)",
            "Table 334",
            Kind::Table
        ));
        assert!(names(
            "/// the media\n    /// Clip it names",
            "media clip",
            Kind::Subject
        ));
        assert!(!names("multimedia clip", "media clip", Kind::Subject));
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
        let report = sweep(&ledger, &no_standard(), &sources);
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
        let report = sweep(&ledger, &no_standard(), &sources);
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
            &no_standard(),
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
        let report = sweep(&ledger, &no_standard(), &sources);
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
        let report = sweep(&ledger, &no_standard(), &sources);
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
        assert!(sweep(&ledger, &no_standard(), &sources).findings.is_empty());
    }
}
