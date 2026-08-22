//! The eighteenth sweep, and the thirteenth of them to be a program: a parent row asserting what
//! its own children deny.
//!
//! # The shape it exists for
//!
//! `doc/todo/01`'s fifth failure shape is a family head gone stale, and every instance recorded
//! before the six-hundred-and-forty-first session had the parent **understating** — `partial`
//! above children that are all settled, a gap named that its own subclause rows had closed. The
//! sixth sweep is the arithmetic for that direction and `--bin counts` the arithmetic for the
//! cardinals in its prose.
//!
//! §12.11 was the other direction. Its row said "Table 276's handlers" among the things read
//! while §12.11.1's said `/RH` "is unread" and §12.11.5's said "the `/RH` entry is read by
//! nobody", and nothing under `crates/` quoted `"RH"`. **No sweep could print it**, and the
//! reason is structural rather than an oversight: an overstating parent names a thing the tree
//! *lacks*, which is the seventh sweep's discriminator, and the seventh sweep reads only
//! `inapplicable` rows; the fourteenth reads a term the tree lacks under a row claiming a
//! **debt**, and an overstating row claims the opposite of a debt. The sign is reversed twice
//! over, and both existing sweeps are blind to it.
//!
//! # The discriminator, and why it is the ledger against itself
//!
//! A parent row that says an entry or a table **is read** is making a claim its own descendants
//! are the detail of. So the whole comparison is inside `ledger.toml`: no source is opened, no
//! vocabulary is guessed at, and the two sides are sentences a person wrote about the same
//! subject at different times.
//!
//! | | the claim | the contradiction |
//! |---|---|---|
//! | sixth sweep | a parent's **status** | its children's statuses |
//! | tenth sweep | a parent's **cardinal** | how many rows the family holds |
//! | this sweep | a parent's **capability** | a descendant saying nobody reads it |
//!
//! The denial vocabulary is [`crate::unread::CLAIMS`] unchanged — the same words the second
//! sweep greps the tree with, so the two sweeps cannot disagree about what a denial is. What is
//! new is the assertion side, [`ASSERTIONS`], and the one idiom that looks like an assertion and
//! is not: this ledger writes "**Read and kept** in the five-hundred-and-sixty-fifth", which
//! says a *round read the row* and claims nothing whatever about the tree ([`NOT_ASSERTIONS`]).
//!
//! # Stance is a property of a clause rather than of a sentence
//!
//! [`crate::unread::sentences`] splits on a full stop, for a reason its own doc gives, and this
//! sweep cannot use it. §14.12.4's row is the witness: "Table 409's `/Start` and `/DParts` are
//! read; Table 408 is not, and `partial` is for that half" holds both stances inside one full
//! stop, and read whole it asserts Table 408 is read — the exact opposite of what it says. So
//! [`parts`] splits on the semicolon and the colon as well. The cost is the reverse case, a
//! stance that carries across a colon into its own explanation, and it is the cheap direction:
//! a part dropped from the assertion list is a claim left for the next run, not a false hit.
//!
//! # The three rungs, and which to read first
//!
//! 1. **[`Rung::Denied`] — the descendant denies the term itself.** The term the parent asserts
//!    stands inside a denial in a descendant's own note. Sharpest, and where the first run's
//!    live defect was.
//! 2. **[`Rung::Owned`] — the descendant *owns* the term and denies reading.** Its note opens by
//!    naming the term, so the row is about it; it asserts nothing of it; and it denies reading
//!    something. §12.11 is this rung, because the parent named Table 276 and §12.11.5 denied
//!    `/RH` — the entry rather than the table — so no sweep matching term against term could
//!    have joined them.
//! 3. **[`Rung::Elsewhere`] — the denial is about another member of the same vocabulary.** A
//!    different table where a table was asserted, a different entry where an entry was: the row
//!    owns the term and its denial is about something else. Printed rather than dropped, because
//!    what makes it noise is a judgement about which of two tables a sentence is about.
//!
//! # The noise, printed rather than filtered
//!
//! - **A table read in part**, which is the dominant shape and the only one a program can mark.
//!   The parent names the entries it reads and the child names the entries nobody reads, and
//!   both sentences cite the same `Table NNN` — §7.3.8's `/Length`, `/Filter` and `/DecodeParms`
//!   against §7.3.8.1's `/FFilter` and `/FDecodeParms`. [`Finding::in_part`] says so where the
//!   parent **enumerates** entries of the asserted table and the child's denial names none of
//!   them. **The enumeration is attributed rather than counted**, which is the ninth sweep's rule
//!   and is what keeps §12.11 out of this bucket: its row lists "Table 273's `/S`, `/V` and
//!   `/Penalty`, Table 275's twenty-five types, Table 276's handlers", so a mark reading every key
//!   in the part as the asserted table's would demote the Table 276 claim on Table 273's keys —
//!   the one defect this sweep was built for, printed as noise. See [`keys_attributed_to`].
//! - **A capability read in part with no table to divide it.** §14.9.2 says three of the four
//!   places a `/Lang` may occupy are read and §14.9.2.2 says the fourth is read by nothing; both
//!   are true, both name `/Lang`, and only the partitive tells them apart. This one is left to
//!   the reader deliberately — a program deciding what "three of the four" governs is the
//!   guess-what-the-sentence-means failure every sweep here refuses.
//! - **A correction narrating its own retired wording**, marked [`Finding::history`] on
//!   [`crate::capabilities::HISTORY`], which is the oldest false positive in `doc/todo/01` and
//!   is marked rather than dropped in every sweep that has it. §12.11's corrected row is exactly
//!   this: it quotes the sentence the six-hundred-and-forty-first session removed.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, and one of its own: a parent row is *allowed* to summarise, and
//! the difference between a summary that overstates and one that is true in part is a reading of
//! two English sentences. It runs in a fraction of a second over the ledger alone and its output
//! is read.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::clause::ClauseNumber;
use crate::ledger::{Ledger, Row, Status};

/// The words that make a part of a note a claim that this tree *does* the thing.
///
/// Matched lower-cased and **on a word boundary**, which is what lets the list be this short:
/// "read" as a whole word is the ledger's verb for this tree consulting something, and it is not
/// reached by "unread", "reader", "reading" or "already". The list stops at the reading verbs on
/// purpose — "supported", "covered", "handled" buy rows whose verb is about the standard rather
/// than about this tree, and the contradiction this sweep hunts is always about *reading*.
pub const ASSERTIONS: [&str; 5] = ["read", "reads", "consulted", "honoured", "obeyed"];

/// The idiom that looks like an assertion and claims nothing about the tree.
///
/// "**Read and kept** in the five-hundred-and-sixty-fifth" says a round read *the row*, which is
/// this ledger's way of recording that a claim was checked and survived. Two of the first run's
/// hits were this sentence and nothing else.
pub const NOT_ASSERTIONS: [&str; 3] = ["read and kept", "re-read", "read off the blame list"];

/// A `/Key` or a `Table NNN` — the two things a row claims to read.
///
/// Nothing else is a term. A capitalised identifier would drag in every type name this ledger
/// mentions, and the seventh and fourteenth sweeps already ask that question of the tree; here
/// both sides are prose written by the same project about the same table, so the standard's own
/// nouns are the ones that join them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    /// A numbered table of the standard.
    Table(u16),
    /// An entry, without the solidus.
    Key(String),
}

impl Term {
    /// Whether two terms are the same *kind* of thing.
    ///
    /// What [`Rung::Elsewhere`] turns on: a denial naming another table is about another table,
    /// and a denial naming another entry is about another entry, but a denial naming an entry
    /// under an asserted table is the §12.11 shape and is not demoted.
    #[must_use]
    pub fn same_kind(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Table(_), Self::Table(_)) | (Self::Key(_), Self::Key(_))
        )
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Table(number) => write!(f, "Table {number}"),
            Self::Key(key) => write!(f, "/{key}"),
        }
    }
}

/// How close the contradiction is, and therefore which hits to read first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// The descendant denies the asserted term itself.
    Denied,
    /// The descendant's note opens by naming the term and denies reading.
    Owned,
    /// The descendant owns the term and its denial names another table, or another entry.
    Elsewhere,
}

impl Rung {
    /// One line saying what this rung is, for the report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Denied => "the child denies the term itself",
            Self::Owned => "the child owns the term and denies reading",
            Self::Elsewhere => "the child's denial names another one of its kind",
        }
    }
}

/// One parent claim a descendant contradicts.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The row making the claim.
    pub parent: ClauseNumber,
    /// Its status, because an `implemented` parent overstating is still overstating.
    pub status: Status,
    /// The 1-based line of `ledger.toml` the parent row starts on.
    pub parent_line: usize,
    /// The descendant contradicting it.
    pub child: ClauseNumber,
    /// The 1-based line the descendant row starts on.
    pub child_line: usize,
    /// What was claimed.
    pub term: Term,
    /// How close the contradiction is.
    pub rung: Rung,
    /// The parent's own words.
    pub asserted: String,
    /// The descendant's own words.
    pub denied: String,
    /// Whether the two parts name disjoint entries of the asserted table — a table read in part.
    pub in_part: bool,
    /// Whether either side's words read as a correction narrating what the row used to say.
    ///
    /// Either side, because the shape arrives on both: a parent quoting the claim it retired,
    /// and a child recording that this sweep corrected it. §9.9.1 became the second the moment
    /// the first run's finding was written down.
    pub history: bool,
}

/// What one run read, and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// How many rows have at least one descendant row — the population that can overstate.
    pub population: usize,
    /// How many term claims those rows make between them.
    pub asserted: usize,
    /// How many of those a descendant asserts too — the claims the run confirms.
    pub corroborated: usize,
    /// The contradictions, closest rung first.
    pub findings: Vec<Finding>,
}

impl Report {
    /// How many findings sit on one rung.
    #[must_use]
    pub fn on(&self, rung: Rung) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rung == rung)
            .count()
    }

    /// How many findings carry a mark that demotes them — a table read in part, or a history.
    #[must_use]
    pub fn marked(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.in_part || finding.history)
            .count()
    }
}

/// A note's parts, split where a full stop, a semicolon or a colon ends one.
///
/// Not [`crate::unread::sentences`], and the module doc says why: §14.12.4 holds both stances
/// inside one full stop. The same "followed by white space or the end" rule keeps `§12.5.6.4`
/// and `1.7` whole.
#[must_use]
pub fn parts(note: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (index, _) in note.match_indices(['.', ';', ':']) {
        let followed = note[index.saturating_add(1)..]
            .chars()
            .next()
            .is_none_or(char::is_whitespace);
        if followed && index >= start {
            let part = note[start..=index].trim();
            if !part.is_empty() {
                out.push(part);
            }
            start = index.saturating_add(1);
        }
    }
    let rest = note.get(start..).unwrap_or("").trim();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// The terms one part of a note names.
#[must_use]
pub fn terms_in(part: &str) -> BTreeSet<Term> {
    let mut terms: BTreeSet<Term> = keys_in(part).into_iter().map(Term::Key).collect();
    for number in tables_in(part) {
        terms.insert(Term::Table(number));
    }
    terms
}

/// The `Table NNN` numbers one part names.
fn tables_in(part: &str) -> Vec<u16> {
    let mut numbers = Vec::new();
    for (index, _) in part.match_indices("Table ") {
        let rest = &part[index.saturating_add("Table ".len())..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(number) = digits.parse::<u16>() {
            numbers.push(number);
        }
    }
    numbers
}

/// The `/Key` tokens one part names, without the solidus.
///
/// The same shape [`crate::unread`] greps for, and the upper-case first letter is what keeps a
/// path out of the list.
fn keys_in(part: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for (index, _) in part.match_indices('/') {
        let rest = &part[index.saturating_add(1)..];
        let mut end = 0usize;
        for (offset, candidate) in rest.char_indices() {
            if candidate.is_ascii_alphanumeric() {
                end = offset.saturating_add(candidate.len_utf8());
            } else {
                break;
            }
        }
        let key = &rest[..end];
        if key.starts_with(|first: char| first.is_ascii_uppercase()) {
            keys.push(key.to_owned());
        }
    }
    keys
}

/// Whether one part claims this tree reads the thing.
#[must_use]
pub fn is_an_assertion(part: &str) -> bool {
    let lowered = part.to_ascii_lowercase();
    if NOT_ASSERTIONS.iter().any(|idiom| lowered.contains(idiom)) {
        return false;
    }
    ASSERTIONS.iter().any(|verb| contains_word(&lowered, verb))
}

/// Whether `text` contains `word` with neither an alphanumeric character on either side.
///
/// The whole of what keeps [`ASSERTIONS`] to five words: "read" is the verb and "unread",
/// "reader", "reading" and "already" are not it.
fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(index, _)| {
        let before = text[..index].chars().next_back();
        let after = text[index.saturating_add(word.len())..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Whether the words read as a correction quoting what the row used to say.
fn is_history(part: &str) -> bool {
    let lowered = part.to_ascii_lowercase();
    crate::capabilities::HISTORY
        .iter()
        .any(|phrase| lowered.contains(phrase))
}

/// What one row says about each term it names.
#[derive(Debug, Clone, Default)]
struct Stance {
    /// Term to the part asserting it.
    asserted: BTreeMap<Term, String>,
    /// Term to the part denying it.
    denied: BTreeMap<Term, String>,
    /// The row's first denial of reading anything, whatever it is about.
    first_denial: Option<String>,
    /// The terms the note's opening part names — what the row is about.
    subject: BTreeSet<Term>,
}

/// Reads one row's note into the stance it takes on each term.
///
/// A denial wins over an assertion within one part, because the denial vocabulary is the
/// specific one: "`/Length1` is **not read**" contains "read".
fn stance(row: &Row) -> Stance {
    let mut stance = Stance::default();
    let Some(note) = row.note.as_deref() else {
        return stance;
    };
    for (which, part) in parts(note).into_iter().enumerate() {
        let terms = terms_in(part);
        if which == 0 {
            stance.subject.clone_from(&terms);
        }
        if crate::unread::is_a_claim(part) {
            if stance.first_denial.is_none() {
                stance.first_denial = Some(part.to_owned());
            }
            for term in terms {
                stance.denied.entry(term).or_insert_with(|| part.to_owned());
            }
        } else if is_an_assertion(part) {
            for term in terms {
                stance
                    .asserted
                    .entry(term)
                    .or_insert_with(|| part.to_owned());
            }
        }
    }
    stance
}

/// Runs the sweep over the ledger, and over nothing else.
#[must_use]
pub fn sweep(ledger: &Ledger) -> Report {
    let stances: Vec<Stance> = ledger.rows.iter().map(stance).collect();
    let mut report = Report::default();
    for (index, row) in ledger.rows.iter().enumerate() {
        let children: Vec<usize> = ledger
            .rows
            .iter()
            .enumerate()
            .filter(|(_, other)| row.clause.is_ancestor_of(&other.clause))
            .map(|(at, _)| at)
            .collect();
        if children.is_empty() {
            continue;
        }
        report.population = report.population.saturating_add(1);
        let Some(stance) = stances.get(index) else {
            continue;
        };
        for (term, asserted) in &stance.asserted {
            report.asserted = report.asserted.saturating_add(1);
            if children
                .iter()
                .filter_map(|at| stances.get(*at))
                .any(|child| child.asserted.contains_key(term))
            {
                report.corroborated = report.corroborated.saturating_add(1);
                continue;
            }
            let claim = Claim {
                parent: row,
                term,
                asserted,
            };
            report
                .findings
                .extend(contradictions(ledger, &stances, &children, &claim));
        }
    }
    report.findings.sort_by(|left, right| {
        left.rung
            .cmp(&right.rung)
            .then_with(|| left.parent.cmp(&right.parent))
    });
    report
}

/// One parent row's claim about one term, as the contradiction hunt needs it.
struct Claim<'a> {
    /// The row making it.
    parent: &'a Row,
    /// What is claimed.
    term: &'a Term,
    /// The parent's own words.
    asserted: &'a str,
}

/// Every descendant contradicting one asserted term, on the closest rung each reaches.
///
/// The rungs do not mix: a term some descendant denies outright is reported there and the
/// weaker readings of the same claim are not printed beside it.
fn contradictions(
    ledger: &Ledger,
    stances: &[Stance],
    children: &[usize],
    claim: &Claim<'_>,
) -> Vec<Finding> {
    let mut denied = Vec::new();
    let mut owned = Vec::new();
    for at in children {
        let (Some(child), Some(stance)) = (ledger.rows.get(*at), stances.get(*at)) else {
            continue;
        };
        if let Some(part) = stance.denied.get(claim.term) {
            denied.push(finding(claim, child, Rung::Denied, part));
        } else if let (true, Some(part)) = (
            stance.subject.contains(claim.term),
            stance.first_denial.as_deref(),
        ) {
            let elsewhere = terms_in(part)
                .iter()
                .any(|other| other != claim.term && other.same_kind(claim.term));
            let rung = if elsewhere {
                Rung::Elsewhere
            } else {
                Rung::Owned
            };
            owned.push(finding(claim, child, rung, part));
        }
    }
    if denied.is_empty() { owned } else { denied }
}

/// One finding: a claim, the row that contradicts it, and how close the contradiction is.
fn finding(claim: &Claim<'_>, child: &Row, rung: Rung, denied: &str) -> Finding {
    Finding {
        parent: claim.parent.clause.clone(),
        status: claim.parent.status,
        parent_line: claim.parent.line,
        child: child.clause.clone(),
        child_line: child.line,
        term: claim.term.clone(),
        rung,
        asserted: claim.asserted.to_owned(),
        denied: denied.to_owned(),
        in_part: read_in_part(claim.term, claim.asserted, denied),
        history: is_history(claim.asserted) || is_history(denied),
    }
}

/// Whether the parent's claim about a table is really a claim about named entries of it.
///
/// Only for a table, and the test is the parent's own enumeration: "Table 5's entries are read
/// where they are used — `/Length` and `/Filter` and `/DecodeParms`" claims three entries rather
/// than a table, so a child denying `/FFilter` contradicts nothing. Where the child denies one
/// of the very keys the parent enumerates, the mark does not fire and the contradiction stands.
/// A parent that names no key of *this* table — "Table 119's entries are read" — is claiming the
/// whole table and is never marked.
///
/// **The attribution matters and is the ninth sweep's rule**: a key belongs to the table the
/// sentence attaches it to, not to whichever table the sentence happens also to mention. "Table
/// 273's `/S`, `/V` and `/Penalty`, Table 275's types, Table 276's handlers" enumerates three
/// entries of 273 and none of 276 — and 276 is the claim §12.11's children denied, so counting
/// its neighbour's keys as its own would have demoted the one defect this sweep was built for.
fn read_in_part(term: &Term, asserted: &str, denied: &str) -> bool {
    let Term::Table(number) = *term else {
        return false;
    };
    let ours = keys_attributed_to(number, asserted);
    let theirs: BTreeSet<String> = keys_in(denied).into_iter().collect();
    !ours.is_empty() && ours.is_disjoint(&theirs)
}

/// The keys one part attaches to one table.
///
/// A mention of `Table NNN` opens a span that runs to the next mention of any table, and the
/// keys inside it are that table's. Keys before the first mention belong to no table and are
/// left out.
pub fn keys_attributed_to(number: u16, part: &str) -> BTreeSet<String> {
    let mut attributed = BTreeSet::new();
    let mut current: Option<u16> = None;
    let mut opened = 0usize;
    let collect = |from: usize, to: usize, table: Option<u16>, into: &mut BTreeSet<String>| {
        if table == Some(number) {
            into.extend(keys_in(part.get(from..to).unwrap_or("")));
        }
    };
    for (index, _) in part.match_indices("Table ") {
        collect(opened, index, current, &mut attributed);
        let digits: String = part
            .get(index.saturating_add("Table ".len())..)
            .unwrap_or("")
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        current = digits.parse::<u16>().ok();
        opened = index;
    }
    collect(opened, part.len(), current, &mut attributed);
    attributed
}

#[cfg(test)]
mod tests {
    use super::{Rung, Term, is_an_assertion, parts, read_in_part, sweep, terms_in};
    use crate::clause::ClauseNumber;
    use crate::ledger::{Ledger, Row, Status};

    fn row(clause: &str, status: Status, note: &str) -> Row {
        Row {
            clause: clause.parse::<ClauseNumber>().expect("a clause number"),
            title: "General".to_owned(),
            status,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    /// §14.12.4's row holds both stances inside one full stop, which is why the parts are not
    /// [`crate::unread::sentences`].
    #[test]
    fn a_semicolon_separates_two_opposite_stances_in_one_sentence() {
        let split = parts("Table 409's `/Start` and `/DParts` are read; Table 408 is not.");
        assert_eq!(split.len(), 2);
        assert!(terms_in(split[0]).contains(&Term::Table(409)));
        assert!(!terms_in(split[0]).contains(&Term::Table(408)));
    }

    /// "Read and kept in the five-hundred-and-sixty-fifth" is a round reading the row.
    #[test]
    fn a_round_reading_the_row_is_not_the_tree_reading_the_entry() {
        assert!(is_an_assertion("Table 119's entries are read"));
        assert!(!is_an_assertion(
            "**Read and kept in the five-hundred-and-sixty-fifth**: nothing in the tree names \
             Table 225's `/CO`"
        ));
    }

    /// The §12.11 shape: the parent asserts a table, the child that owns it denies an entry.
    #[test]
    fn a_parent_asserting_a_table_its_own_child_says_nobody_reads_is_named() {
        let ledger = Ledger {
            rows: vec![
                row(
                    "12.11",
                    Status::Partial,
                    "Read in full — Table 273's `/S` and Table 276's handlers.",
                ),
                row(
                    "12.11.5",
                    Status::Partial,
                    "Table 276's alternative requirement handlers. This program runs no \
                     ECMAScript, so the `/RH` entry is read by nobody.",
                ),
            ],
        };
        let report = sweep(&ledger);
        let found = report.findings.first().expect("the contradiction");
        assert_eq!(found.term, Term::Table(276));
        assert_eq!(found.rung, Rung::Owned);
        assert_eq!(found.child.to_string(), "12.11.5");
        assert!(!found.history);
    }

    /// A child asserting the same term is corroboration, and prints nothing.
    #[test]
    fn a_child_that_agrees_is_counted_rather_than_printed() {
        let ledger = Ledger {
            rows: vec![
                row("9.9", Status::Implemented, "Table 124's keys are read."),
                row(
                    "9.9.1",
                    Status::Partial,
                    "Table 124's keys are read in full.",
                ),
            ],
        };
        let report = sweep(&ledger);
        assert_eq!(report.corroborated, 1);
        assert!(report.findings.is_empty());
    }

    /// §7.3.8's shape: the parent enumerates the entries it reads, so both sentences are true.
    #[test]
    fn a_table_read_in_part_is_marked_rather_than_dropped() {
        assert!(read_in_part(
            &Term::Table(5),
            "Table 5's entries are read — `/Length` and `/Filter`",
            "Table 5's `/FFilter` and `/FDecodeParms` are unread."
        ));
        assert!(!read_in_part(
            &Term::Table(5),
            "Table 5's entries are read — `/Length` and `/Filter`",
            "Table 5's `/Filter` is unread."
        ));
        assert!(!read_in_part(
            &Term::Table(125),
            "Table 125 states where an embedded program ends, read in the six-hundred-and-\
             twenty-fifth session.",
            "Table 125's `/Length1`, `/Length2` and `/Length3` are read by nobody."
        ));
    }

    /// §12.11's own wording: three keys belong to Table 273 and none of them to Table 276, so
    /// the mark that demotes an enumerated claim may not reach the table beside it.
    #[test]
    fn a_key_belongs_to_the_table_the_sentence_attaches_it_to() {
        let asserted = "Read in full — Table 273's `/S`, `/V` and `/Penalty`, Table 275's \
                        twenty-five types, Table 276's handlers.";
        assert!(read_in_part(&Term::Table(273), asserted, "It is unread."));
        assert!(!read_in_part(&Term::Table(276), asserted, "It is unread."));
    }
}
