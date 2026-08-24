//! The nineteenth sweep: a page-list note the tree moved under.
//!
//! # The shape it exists for
//!
//! `crates/pdf-model/tests/oracle.rs` sorts every non-agreeing corpus page into a named list
//! whose doc comment carries the diagnosis — the measurements, the clause, and why the page is
//! where it is. Those notes are what a round reads before deciding whether a page is our defect,
//! and **nothing verified them against the decisions taken after they were written**. The
//! six-hundred-and-sixty-second session found the consequence: ADR 0476 made this tree's edge
//! coverage exact, and three sessions later `CONTRADICTED_TIGHT_CONSENSUS` still said ours was
//! the quarter-quantised form. The correction had reached `doc/traps/`, §10.7.4's ledger row and
//! `doc/todo/11` item 7 — everywhere except the group it was about (ADR 0489). That is a third
//! way a note can be wrong, after its name and its reading: *a sentence that was true when
//! written and that nothing pointed at when the tree moved under it.*
//!
//! # The discriminator
//!
//! `doc/adr/` is numbered in the order this project took decisions, so a number *is* a date. A
//! note's ADR citations are therefore a claim about which decisions it has read, and the sweep
//! compares two numbers:
//!
//! > the newest ADR the note cites, against the newest ADR that names one of the documents the
//! > note is about.
//!
//! An ADR later than the note's newest citation, naming a page the note diagnoses, is a decision
//! taken *after* the note was last revised *about a page the note explains*. That is a fact
//! about two files rather than a reading of either, and it is the fact 662 established by hand.
//!
//! # Why it is none of the other sweeps
//!
//! [`crate::pointers`] reads what a note **points at** and already covers this source set — it
//! finds nothing in `oracle.rs`, because these notes' paths and symbols are live. What decayed
//! was a *measurement*, and a measurement names no file. [`crate::retired`] would find it, and
//! only if somebody typed the right noun: run over `quarter` at the planted revision it returns
//! 254 mentions with the stale sentence at rank 100, because a noun that broad is most of a
//! rasteriser's vocabulary. This sweep derives its own population, which is the whole difference.
//!
//! # What is a document, and why the vocabulary comes from the lists
//!
//! A `.pdf` token could be the standard itself, a usage line's placeholder or a file this corpus
//! does not have. So the vocabulary is not guessed: it is exactly the set of documents the page
//! lists themselves name ([`Corpus`]). An ADR "names a document" when it writes one of those.
//!
//! # The rungs — read the first one first
//!
//! **Every rung requires a shared page.** That is not a detail: the first run of this sweep put
//! 24 of 123 notes on rung 1 and ADR 0489 was in all 24, because a *census* ADR prints the name
//! of every list it counted. Naming a constant says the ADR mentioned the list; only a shared
//! page says it disturbed it. So the name ranks a hit and never makes one.
//!
//! 1. **[`Rung::Group`] — the later ADR names one of the note's pages *and* writes the list's own
//!    name.** No judgement is left: it is about this list, and the note does not cite it.
//! 2. **[`Rung::Prose`] — the later ADR names a document the note's *prose* names.** The prose is
//!    where the argument is, so a decision about one of those pages lands on the argument.
//! 3. **[`Rung::Member`] — the later ADR names only a list member the prose never mentions.** A
//!    370-page list collects these for free and most are nothing.
//!
//! # The noise, classified rather than filtered
//!
//! - **Rung 3 is mostly noise by construction.** `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` holds 370
//!   pages and any corpus-wide ADR names some of them. It is printed because a large list is
//!   also where a stale diagnosis hides longest, and ranked last because of the ratio.
//! - **A standing witness is named by everything near it.** `colors.pdf` and `issue21346.pdf`
//!   are this project's scan-conversion witnesses, so an ADR about any part of §10.7.4 names
//!   them whether or not it disturbs a given note.
//! - **A note may deliberately not cite a later ADR** that is about a different property of the
//!   same page. That is the reverse of a defect and only the sentence tells them apart.
//! - **[`Report::uncited`] is a ranking, not a finding.** A note citing no ADR at all has no
//!   left-hand side to compare, so it is counted rather than listed among the hits — the
//!   comparison is undefined, not failed.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, and one of its own: a note is allowed to be about one property of
//! a page while a later decision was about another, and a build that failed on that would teach
//! rounds to cite ADRs they had not read. It runs in a fraction of a second over the tree's
//! sources and `doc/adr/`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The directory holding the decision records, relative to the workspace root.
pub const DECISIONS: &str = "doc/adr";

/// How many decimal digits an ADR's file name begins with.
const NUMBER_DIGITS: usize = 4;

/// One decision record: its number, and what it names.
#[derive(Debug, Clone)]
pub struct Decision {
    /// The number the file name begins with, which is also the order it was taken in.
    pub number: u32,
    /// The file, as it is printed: `doc/adr/0489-….md`.
    pub file: String,
    /// The corpus documents it names.
    pub documents: BTreeSet<String>,
    /// The page-list constants it names.
    pub groups: BTreeSet<String>,
}

/// One page-list note: the doc comment above a `const NAME: [&str; N]` of corpus pages.
#[derive(Debug, Clone)]
pub struct Note {
    /// The constant's name.
    pub name: String,
    /// The file, as it is printed.
    pub file: String,
    /// The 1-based line the doc comment starts on.
    pub line: usize,
    /// The ADR numbers the note cites.
    pub cited: BTreeSet<u32>,
    /// The documents the note's prose names.
    pub prose: BTreeSet<String>,
    /// The documents the list itself holds.
    pub members: BTreeSet<String>,
    /// The doc comment's own lines, each with the 1-based line it sits on.
    ///
    /// This sweep folds them into one string and never looks at a line again; they are kept
    /// because [`crate::quoted`] asks a second question of the same population and its answer
    /// has to name a line a person can open. One scanner, two questions.
    pub body: Vec<(usize, String)>,
    /// The pages the list itself holds, as it writes them: `issue7891_bc1.pdf page 1`.
    ///
    /// [`Self::members`] is the same list with the page numbers taken off, which is what an ADR
    /// can be compared against. A figure is quoted about a *page*, so the finer form is kept too.
    pub pages: BTreeSet<String>,
}

impl Note {
    /// The newest ADR the note cites, or `None` where it cites none.
    #[must_use]
    pub fn newest_cited(&self) -> Option<u32> {
        self.cited.iter().copied().next_back()
    }

    /// Every document the note is about: the prose's and the list's together.
    #[must_use]
    pub fn documents(&self) -> BTreeSet<String> {
        let mut all = self.prose.clone();
        all.extend(self.members.iter().cloned());
        all
    }
}

/// How close a later decision sits to what the note argues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// The later ADR writes the list's own name.
    Group,
    /// The later ADR names a document the note's prose names.
    Prose,
    /// The later ADR names only a list member the prose never mentions.
    Member,
}

impl Rung {
    /// One line saying what this rung means, printed beside every hit on it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Group => "the later ADR is about one of these pages and names this list",
            Self::Prose => "the later ADR is about a page the note's prose argues",
            Self::Member => "the later ADR names a list member the prose does not",
        }
    }
}

/// One later decision, and what makes it later than the note.
#[derive(Debug, Clone)]
pub struct Overtaking {
    /// The decision's number.
    pub number: u32,
    /// Its file, as it is printed.
    pub file: String,
    /// How close it sits to what the note argues.
    pub rung: Rung,
    /// The documents it shares with the note.
    pub documents: BTreeSet<String>,
}

/// One note, with every decision taken after its newest citation about one of its own pages.
#[derive(Debug, Clone)]
pub struct Finding {
    /// The note.
    pub note: Note,
    /// The decisions that overtook it, closest rung first and newest first within a rung.
    pub overtaking: Vec<Overtaking>,
}

impl Finding {
    /// The closest rung any of the overtaking decisions sits on.
    ///
    /// [`Rung::Member`] where there are none, which [`sweep`] never constructs: a finding with
    /// no overtaking decision is not a finding.
    #[must_use]
    pub fn rung(&self) -> Rung {
        self.overtaking
            .iter()
            .map(|overtaking| overtaking.rung)
            .min()
            .unwrap_or(Rung::Member)
    }

    /// The newest decision that overtook the note.
    #[must_use]
    pub fn newest(&self) -> u32 {
        self.overtaking
            .iter()
            .map(|overtaking| overtaking.number)
            .max()
            .unwrap_or_default()
    }
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// The hits, closest rung first and newest decision first within a rung.
    pub findings: Vec<Finding>,
    /// How many page-list notes were read.
    pub notes: usize,
    /// How many of those cite no ADR at all, so the comparison has no left-hand side.
    pub uncited: usize,
    /// How many decision records were read.
    pub decisions: usize,
    /// How many distinct documents the page lists name between them.
    pub corpus: usize,
}

impl Report {
    /// How many hits sit on one rung.
    #[must_use]
    pub fn on(&self, rung: Rung) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rung() == rung)
            .count()
    }
}

/// The documents the page lists name between them — this sweep's whole vocabulary.
///
/// Derived rather than guessed, because a `.pdf` token in prose may be the standard itself
/// (`doc/ISO_32000-2_sponsored_EC3.pdf`), a usage line's `doc/x.pdf` or a file this corpus does
/// not carry. A document is one this project sorted a page of, or it is not a document here.
#[derive(Debug, Clone, Default)]
pub struct Corpus {
    /// The names, sorted.
    pub names: BTreeSet<String>,
}

impl Corpus {
    /// The members of `named` that are documents this corpus holds.
    #[must_use]
    pub fn narrow(&self, named: &BTreeSet<String>) -> BTreeSet<String> {
        named
            .iter()
            .filter(|name| self.names.contains(*name))
            .cloned()
            .collect()
    }
}

/// Reads every page-list note out of the tree's Rust sources.
///
/// `sources` are the Rust files under [`crate::SOURCE_ROOTS`] with their text, as
/// [`crate::entries::sources`] returns them. The checker's own directory is skipped, as
/// everywhere here: this module's own examples would otherwise be swept.
#[must_use]
pub fn notes(sources: &[(PathBuf, String)]) -> Vec<Note> {
    let mut found = Vec::new();
    for (path, text) in sources {
        let shown = shown(path);
        if shown.starts_with(crate::NOT_SCANNED) {
            continue;
        }
        found.extend(notes_in(&shown, text));
    }
    found
}

/// Reads the decision records.
///
/// # Errors
///
/// If `doc/adr/` cannot be listed, or one of its files cannot be read: a sweep that skipped
/// what it could not open would report a clean tree for a tree it had not looked at.
pub fn decisions(root: &Path) -> std::io::Result<Vec<Decision>> {
    let directory = root.join(DECISIONS);
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&directory)? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(number) = leading_number(&name) else {
            continue;
        };
        let text = std::fs::read_to_string(&path)?;
        records.push(Decision {
            number,
            file: format!("{DECISIONS}/{name}"),
            documents: documents_in(&text),
            groups: constants_in(&text),
        });
    }
    records.sort_by_key(|decision| decision.number);
    Ok(records)
}

/// Runs the sweep.
///
/// The vocabulary is taken from the notes themselves, so this is the one entry point: a
/// [`Corpus`] built from a subset of the lists would silently narrow what an ADR can be seen to
/// name.
#[must_use]
pub fn sweep(notes: &[Note], decisions: &[Decision]) -> Report {
    let corpus = corpus_of(notes);
    let mut findings = Vec::new();
    let mut uncited = 0usize;

    for note in notes {
        let Some(newest_cited) = note.newest_cited() else {
            uncited = uncited.saturating_add(1);
            continue;
        };
        // Both halves of every comparison below run through the vocabulary, so a `.pdf` token
        // this project never sorted a page of cannot make a hit from either side.
        let prose = corpus.narrow(&note.prose);
        let about: BTreeSet<String> = prose.union(&note.members).cloned().collect();
        if about.is_empty() {
            continue;
        }
        let mut overtaking = Vec::new();
        for decision in decisions.iter().filter(|d| d.number > newest_cited) {
            let named = corpus.narrow(&decision.documents);
            let shared: BTreeSet<String> = named.intersection(&about).cloned().collect();
            // A shared page is what every rung rests on, the first one included. The first run
            // of this sweep named 24 of 123 notes on rung 1 and ADR 0489 was in every one of
            // them, because a *census* ADR prints the name of every list it counted — so a
            // constant's name alone says the ADR mentioned the list, not that it disturbed it.
            if shared.is_empty() {
                continue;
            }
            let rung = if decision.groups.contains(&note.name) {
                Rung::Group
            } else if shared.iter().any(|name| prose.contains(name)) {
                Rung::Prose
            } else {
                Rung::Member
            };
            overtaking.push(Overtaking {
                number: decision.number,
                file: decision.file.clone(),
                rung,
                documents: shared,
            });
        }
        if overtaking.is_empty() {
            continue;
        }
        overtaking.sort_by_key(|o| (o.rung, std::cmp::Reverse(o.number)));
        findings.push(Finding {
            note: note.clone(),
            overtaking,
        });
    }

    findings.sort_by_key(|finding| (finding.rung(), std::cmp::Reverse(finding.newest())));
    Report {
        findings,
        notes: notes.len(),
        uncited,
        decisions: decisions.len(),
        corpus: corpus.names.len(),
    }
}

/// The vocabulary the page lists name between them.
#[must_use]
pub fn corpus_of(notes: &[Note]) -> Corpus {
    let mut names = BTreeSet::new();
    for note in notes {
        names.extend(note.members.iter().cloned());
    }
    Corpus { names }
}

/// Every page-list note in one file.
///
/// A note is the run of `///` lines immediately above a `const NAME: [&str; N]`, and the list is
/// everything up to the closing `];`. A constant with no doc comment above it contributes
/// nothing: there is no claim to be overtaken.
fn notes_in(shown: &str, text: &str) -> Vec<Note> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut comment: Vec<&str> = Vec::new();
    let mut comment_at = 0usize;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("///") {
            if comment.is_empty() {
                comment_at = index.saturating_add(1);
            }
            comment.push(rest.trim());
            continue;
        }
        match page_list_name(trimmed) {
            Some(name) if !comment.is_empty() => {
                let prose = comment.join(" ");
                let body = list_body(&lines, index);
                found.push(Note {
                    name,
                    file: shown.to_owned(),
                    line: comment_at,
                    cited: citations_in(&prose),
                    prose: documents_in(&prose),
                    members: documents_in(&body),
                    body: comment
                        .iter()
                        .enumerate()
                        .map(|(offset, text)| {
                            (comment_at.saturating_add(offset), (*text).to_owned())
                        })
                        .collect(),
                    pages: pages_in(&body),
                });
            }
            _ => {}
        }
        comment.clear();
    }
    found
}

/// Every page an array literal holds, as it writes it.
///
/// A member is a string literal and the whole of it is the page's name, so the quotes are the
/// delimiter and nothing has to be guessed about the shape of what is between them. A literal
/// naming no document is dropped: the lists in this tree hold pages, and anything else on such
/// a line is a comment or a trailing token rather than a member.
fn pages_in(body: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = body;
    while let Some(open) = rest.find('"') {
        let after = rest.get(open.saturating_add(1)..).unwrap_or_default();
        let Some(close) = after.find('"') else { break };
        let literal = after.get(..close).unwrap_or_default();
        if !documents_in(literal).is_empty() {
            found.insert(literal.to_owned());
        }
        rest = after.get(close.saturating_add(1)..).unwrap_or_default();
    }
    found
}

/// The constant's name, where the line declares an array of string literals.
///
/// `const CONTRADICTED_GLYPH_EDGES: [&str; 26] = [` and its empty and single-line forms. The
/// element type is what makes it a page list rather than any other constant.
fn page_list_name(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("const ")
        .or_else(|| line.strip_prefix("pub const "))?;
    let (name, tail) = rest.split_once(':')?;
    if !tail.trim_start().starts_with("[&str;") {
        return None;
    }
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
    {
        return None;
    }
    Some(name.to_owned())
}

/// The array literal beginning on `index`, up to and including its closing bracket.
///
/// Bounded by [`LIST_LINES`] rather than by the end of the file, so a missing terminator costs a
/// truncated member list and not a scan of everything below it.
fn list_body(lines: &[&str], index: usize) -> String {
    let end = index.saturating_add(LIST_LINES).min(lines.len());
    let mut body = String::new();
    for line in lines.get(index..end).unwrap_or_default() {
        body.push_str(line);
        body.push('\n');
        if line.contains("];") {
            break;
        }
    }
    body
}

/// How many lines of an array literal are read before it is cut.
///
/// The longest list in this tree holds 370 pages one per line; the bound is generous against it
/// and finite against a file that lost its bracket.
const LIST_LINES: usize = 2000;

/// Every ADR number a text cites, in either form this project writes.
///
/// `ADR 0489` is the sentence form and `doc/adr/0489-…` the pointer form; both are counted,
/// because a note citing the file has read the decision as surely as one citing the number.
fn citations_in(text: &str) -> BTreeSet<u32> {
    let mut found = BTreeSet::new();
    for (marker, skip) in [("ADR ", 4usize), ("adr/", 4)] {
        let mut from = 0usize;
        while let Some(at) = text.get(from..).and_then(|rest| rest.find(marker)) {
            let start = from.saturating_add(at).saturating_add(skip);
            if let Some(number) = leading_number(text.get(start..).unwrap_or_default()) {
                found.insert(number);
            }
            from = start;
        }
    }
    found
}

/// The number a string begins with, where it begins with exactly [`NUMBER_DIGITS`] digits.
///
/// An ADR is `0489`, never `489` or `04891`, so a fixed width is what tells a citation from a
/// measurement that happens to follow the word.
fn leading_number(text: &str) -> Option<u32> {
    let digits: String = text.chars().take(NUMBER_DIGITS).collect();
    if digits.len() != NUMBER_DIGITS || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if text
        .chars()
        .nth(NUMBER_DIGITS)
        .is_some_and(|c| c.is_ascii_digit())
    {
        return None;
    }
    digits.parse().ok()
}

/// Every `*.pdf` token a text names, unfiltered.
///
/// [`Corpus`] is what narrows this to documents this project actually sorted a page of; keeping
/// the two apart is what lets the vocabulary be derived from the lists rather than guessed.
///
/// # A slash is part of a page's name, and this function used to end one
///
/// It stopped the scan at `/` and then **rejected** whatever it had found, on the reasoning that
/// "a name preceded by a path separator is a file in this tree rather than a corpus document —
/// `doc/ISO_32000-2_sponsored_EC3.pdf` is the standard, not a page". That was true when it was
/// written and stopped being true one round later: ADR 0541 gave every page of a submodule
/// corpus its corpus's label — `pdfbox/attachment.pdf page 1` — *because* three of those
/// documents share a bare file name with one of the 974 and only two of the three share their
/// bytes. So the label is the identity rather than a path, and the rule that kept the standard
/// out was silently discarding **every** page of the two voted corpora from this sweep's
/// vocabulary and from [`crate::quoted`]'s, which reads the same [`Note::pages`].
///
/// The exclusion it replaced was never needed: [`Corpus`] is built from the lists' own members,
/// so a `.pdf` token in prose that no list holds is narrowed away whatever its shape. The scan
/// therefore takes the separator as part of the name — which is what makes the label survive —
/// and the standard reaches [`Corpus::narrow`] and is dropped there, one filter instead of two.
fn documents_in(text: &str) -> BTreeSet<String> {
    const SUFFIX: &str = ".pdf";
    let mut found = BTreeSet::new();
    let bytes = text.as_bytes();
    let mut from = 0usize;
    while let Some(at) = text.get(from..).and_then(|rest| rest.find(SUFFIX)) {
        let end = from.saturating_add(at).saturating_add(SUFFIX.len());
        let mut start = from.saturating_add(at);
        while start > 0 {
            let previous = bytes.get(start.saturating_sub(1)).copied().unwrap_or(b' ');
            if previous.is_ascii_alphanumeric()
                || previous == b'_'
                || previous == b'-'
                || previous == b'.'
                || previous == b'/'
            {
                start = start.saturating_sub(1);
            } else {
                break;
            }
        }
        let named = (start < from.saturating_add(at))
            .then(|| text.get(start..end))
            .flatten();
        if let Some(name) = named {
            found.insert(name.to_owned());
        }
        from = end;
    }
    found
}

/// Every page-list constant name a text writes.
///
/// Upper-case runs of at least [`CONSTANT_MINIMUM`] characters holding an underscore. An ADR
/// naming `CONTRADICTED_TIGHT_CONSENSUS` is unambiguously about that list; the threshold keeps
/// `PDF`, `ISO` and `BBox` out.
fn constants_in(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut current = String::new();
    for character in text.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_uppercase() || character == '_' || character.is_ascii_digit() {
            current.push(character);
        } else {
            if current.len() >= CONSTANT_MINIMUM && current.contains('_') {
                found.insert(current.clone());
            }
            current.clear();
        }
    }
    found
}

/// How long an upper-case run must be to be taken for a page list's name.
const CONSTANT_MINIMUM: usize = 12;

/// A path as it is printed and compared: relative, with forward slashes.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(name: &str, prose: &str, members: &[&str]) -> Note {
        Note {
            name: name.to_owned(),
            file: "crates/x/tests/oracle.rs".to_owned(),
            line: 1,
            cited: citations_in(prose),
            prose: documents_in(prose),
            members: members.iter().map(|m| (*m).to_owned()).collect(),
            body: vec![(1, prose.to_owned())],
            pages: members.iter().map(|m| (*m).to_owned()).collect(),
        }
    }

    fn decision(number: u32, text: &str) -> Decision {
        Decision {
            number,
            file: format!("doc/adr/{number:04}-x.md"),
            documents: documents_in(text),
            groups: constants_in(text),
        }
    }

    #[test]
    fn a_citation_is_four_digits_after_either_marker() {
        let cited = citations_in("priced in ADR 0474; see doc/adr/0476-the-nine-pieces.md");
        assert_eq!(cited, BTreeSet::from([474, 476]));
    }

    #[test]
    fn a_measurement_after_the_word_is_not_a_citation() {
        assert!(citations_in("ADR 0474 says 0.0023 and 12345").contains(&474));
        assert_eq!(citations_in("ADR 12345"), BTreeSet::new());
    }

    /// A corpus label is part of the page's identity and survives the scan (ADR 0541).
    #[test]
    fn a_corpus_label_is_part_of_the_document_name() {
        let named = documents_in("pdfbox/attachment.pdf page 1");
        assert_eq!(named, BTreeSet::from(["pdfbox/attachment.pdf".to_owned()]));
    }

    /// And the standard, which is a path rather than a page, is dropped by the corpus instead.
    #[test]
    fn a_path_is_not_a_corpus_document() {
        let named = documents_in("colors.pdf page 1 and doc/ISO_32000-2_sponsored_EC3.pdf");
        assert_eq!(
            named,
            BTreeSet::from([
                "colors.pdf".to_owned(),
                "doc/ISO_32000-2_sponsored_EC3.pdf".to_owned()
            ])
        );
        let corpus = corpus_of(&[note("A_LIST_OF_PAGES", "", &["colors.pdf"])]);
        assert_eq!(
            corpus.narrow(&named),
            BTreeSet::from(["colors.pdf".to_owned()])
        );
    }

    #[test]
    fn a_declaration_of_string_literals_is_a_page_list() {
        assert_eq!(
            page_list_name("const CONTRADICTED_GLYPH_EDGES: [&str; 26] = ["),
            Some("CONTRADICTED_GLYPH_EDGES".to_owned())
        );
        assert_eq!(page_list_name("const DPI: u32 = 72;"), None);
        assert_eq!(page_list_name("const SCALE: f32 = 1.0;"), None);
    }

    /// The six-hundred-and-sixty-second session's own case, which is what trap 13 asks for: a
    /// note citing ADR 0474 where ADR 0489 names the same document is the shape, and the same
    /// note citing 0489 is not.
    #[test]
    fn a_later_decision_about_the_same_page_is_the_finding() {
        let decisions = vec![
            decision(474, "colors.pdf pages 1 and 2, quantised to a quarter"),
            decision(
                489,
                "colors.pdf re-derived, and CONTRADICTED_TIGHT_CONSENSUS rewritten",
            ),
        ];

        let stale = vec![note(
            "CONTRADICTED_TIGHT_CONSENSUS",
            "priced in ADR 0474",
            &["colors.pdf"],
        )];
        let report = sweep(&stale, &decisions);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rung(), Rung::Group);
        assert_eq!(report.findings[0].newest(), 489);

        let current = vec![note(
            "CONTRADICTED_TIGHT_CONSENSUS",
            "ADR 0489 has the argument",
            &["colors.pdf"],
        )];
        assert!(sweep(&current, &decisions).findings.is_empty());
    }

    /// The list's own pages and the note's own lines are kept for [`crate::quoted`], which asks
    /// a figure of a *page* rather than of a document, and prints the line a person opens.
    #[test]
    fn a_note_keeps_its_lines_and_its_pages() {
        let source = "\
/// The first line, ADR 0474.
/// The second, about colors.pdf.
const A_LIST_OF_PAGES: [&str; 2] = [
    \"colors.pdf page 1\",
    \"colors.pdf page 2\",
];
";
        let found = notes_in("crates/x/tests/oracle.rs", source);
        assert_eq!(found.len(), 1);
        let note = &found[0];
        assert_eq!(note.line, 1);
        assert_eq!(note.body.len(), 2);
        assert_eq!(
            note.body[1],
            (2, "The second, about colors.pdf.".to_owned())
        );
        assert_eq!(
            note.pages,
            BTreeSet::from([
                "colors.pdf page 1".to_owned(),
                "colors.pdf page 2".to_owned()
            ])
        );
        assert_eq!(note.members, BTreeSet::from(["colors.pdf".to_owned()]));
    }

    #[test]
    fn a_note_citing_nothing_has_no_left_hand_side() {
        let decisions = vec![decision(489, "colors.pdf")];
        let report = sweep(
            &[note("A_LIST_OF_PAGES", "no citation here", &["colors.pdf"])],
            &decisions,
        );
        assert!(report.findings.is_empty());
        assert_eq!(report.uncited, 1);
    }

    #[test]
    fn a_document_outside_the_lists_is_not_in_the_vocabulary() {
        let corpus = corpus_of(&[note("A_LIST_OF_PAGES", "", &["colors.pdf"])]);
        let named = documents_in("colors.pdf and other.pdf");
        assert_eq!(
            corpus.narrow(&named),
            BTreeSet::from(["colors.pdf".to_owned()])
        );
    }

    /// A decision naming only a document the note lists but never argues sits a rung down, and
    /// that is most of this sweep's noise.
    #[test]
    fn a_member_the_prose_never_mentions_is_the_last_rung() {
        let decisions = vec![
            decision(300, "about nothing"),
            decision(400, "issue4436r.pdf"),
        ];
        let listed = vec![note("A_LIST_OF_PAGES", "ADR 0300", &["issue4436r.pdf"])];
        let report = sweep(&listed, &decisions);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rung(), Rung::Member);
    }
}
