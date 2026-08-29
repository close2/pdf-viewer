//! The twenty-third sweep: a claim quantified over a corpus, and the population it names.
//!
//! # The shape it exists for
//!
//! Four rounds found the same defect, each of them by accident while doing something else:
//!
//! - a seeding recipe whose stated population was one corpus submodule, the smallest source of
//!   signatures this tree holds — widened, 22 certificates became 941 (ADR 0751);
//! - the identical defect in `doc/verify.md`'s recipe for the target one down, whose re-census
//!   then **falsified two written sentences** — "four of the six signature formats §12.8.3
//!   defines have no witness" (all six have witnesses) and "no corpus document carries a
//!   document timestamp" (twenty, in fifteen documents) — and a third, §12.8.3.4.2's "four
//!   corpus documents" (ADR 0754);
//! - a census that found a defect no gate could see, because every corpus witness it had was of
//!   one shape (ADR 0753);
//! - and a merge round that found the same shape one file over and could only write it down.
//!
//! Every one of those sentences was **true when it was written**, of the population it was
//! written over. What changed underneath them is the tree: `doc/pdf.js/test/pdfs` is what this
//! project has meant by *the corpus* since its first rounds, and the crawl beside it is two
//! orders of magnitude larger. So this is a **decay detector**, like the twenty-second sweep
//! ([`crate::parts`]) and for the same reason — most of the population it walks is sentences
//! that were right — and what it can offer is an ordering rather than a verdict.
//!
//! # The predicate, stated exactly
//!
//! **A sentence quantifies over a corpus, and does not say which corpus.** That is the whole of
//! it, and each half is decidable:
//!
//! - **Quantifies over a corpus** — one of [`ABSENT`], [`UNIQUE`] or a cardinal governs one of
//!   [`CORPUS`] or [`DOCUMENTS`], across at most [`MAX_MODIFIERS`] words and never past one of
//!   [`FUNCTION_WORDS`], in a sentence that mentions a corpus at all. [`MAX_MODIFIERS`] says why
//!   that is looser than [`crate::parts`]' adjacency and which of the four findings the tighter
//!   rule would have missed.
//! - **Says which corpus** — the sentence names a population: one of [`Populations`]' own names,
//!   read off the directories on disk, one of [`OUR_WORDS`], or a numeral denominator, which is
//!   a cardinal of at least [`MIN_CARDINAL`] under [`DENOMINATING`].
//!
//! # What the sweep cannot see, said plainly
//!
//! - **Whether the claim is true.** It reports that a denominator is unstated, never that a
//!   count is wrong. Re-counting is the reader's, and the report names the populations so that
//!   the reader knows what there is to count over.
//! - **A denominator in the sentence *before* this one.** A paragraph that opens "over the 974"
//!   and then makes four claims has denominated all four to a person and none of them here.
//!   This is the direction the sweep is loose in, and it is the same direction
//!   [`crate::parts`] is loose in for a modifier that follows its noun.
//! - **An absence stated without a quantifier over a noun** — "the corpus holds none of them",
//!   "nothing here carries one". The quantifier has to govern a noun for the noun to say what
//!   population is meant, and that costs these.
//! - **A recipe outside a Markdown fence.** `fuzz/seed_x509.py`'s own stated population — the
//!   first of the four findings — is a comment in a Python file, and this reads Rust comments,
//!   ledger notes and `doc/`. A seeder that names its corpora in its own docstring is not
//!   judged here.
//! - **A claim over a population that is not documents**: pages, glyphs, signature values,
//!   clauses. [`DOCUMENTS`] is the noun family the corpus is measured in.
//! - **A count that is right for a subset.** "The 4974 documents that name a `/ByteRange`" is a
//!   numeral denominator no population has, and that is [`Rung::Unmatched`]: a subset count and
//!   a population that moved underneath a sentence look identical from here, so both are
//!   printed and neither is judged.
//!
//! # The five rungs, and which to read first
//!
//! 1. **[`Rung::Recipe`]** — a fenced invocation that walks some of this tree's corpora and not
//!    the rest. It is not a sentence at all, and it reads first because two of the four rounds
//!    found the defect in exactly that shape: a recipe is where a narrow population turns into a
//!    number somebody then writes down.
//! 2. **[`Rung::Ledger`]** — an absence or a uniqueness in a ledger note, naming no population.
//!    One witness anywhere refutes it, and *anywhere* is now the whole of what the report's last
//!    line counts; a row's status rests on the sentence.
//! 3. **[`Rung::Code`]**, then **[`Rung::Prose`]** — the same shape beside the code, where it
//!    says what a fixture is for, and then in an undated document.
//! 4. **[`Rung::Counted`]** — a count, naming no population, wherever it is written. Refuting
//!    one takes a re-census rather than a witness, which is why it reads after the absences.
//! 5. **[`Rung::Unmatched`]** — a numeral denominator no population on disk has. The sentence
//!    did its job and the number is a question: a subset, or a population that changed size.
//!
//! A claim that names a population is **counted rather than listed**, because it is the right
//! shape and there are many of them. So is a claim in [`DATED`], on [`crate::parts`]' rule that
//! a decision record is dated by its own number and was right on its date.
//!
//! # The noise, printed rather than filtered
//!
//! - **"The corpus" has meant one thing here for eight hundred rounds**, and most of these
//!   sentences were written in a round that had no other corpus to mean. The report's own tally
//!   of *what* denominates the sentences that do — `974` and `pdf.js` above everything else — is
//!   the measurement of that convention, and the convention is the finding rather than any one
//!   sentence.
//! - **"Every one of the corpus's fourteen embedded `CMap`s"** arrives as a uniqueness governing
//!   `fourteen`, because `one` is the quantifier the scan reaches first. The sentence *is* an
//!   undenominated count; only the word under it is the wrong one.
//! - **A recipe that walks one corpus on purpose.** A census run over the crawl alone, or a
//!   `git clone` that creates the submodules and therefore names them, is right to name what it
//!   names — and says so in the prose around the fence, which this reads nothing of.
//!
//! # Why it is not a gate
//!
//! [`crate::parts`]' argument unchanged. An undenominated claim is a sentence to read, and a
//! sentence that has always meant the ratcheted corpus and is written in a round that had no
//! other is not a defect. Failing a build on the ordering would have it switched off.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::ledger::Ledger;

/// The words that assert a corpus holds none of something.
pub const ABSENT: [&str; 3] = ["no", "zero", "neither"];

/// The words that assert a corpus holds exactly one of something.
///
/// `one` is read here where [`crate::parts`] refuses it, and the difference is what the word
/// governs. There, a bare `one` before a part is English's determiner — "the one crate that owns
/// it" names a crate rather than counting the population. Here the noun is the corpus itself, so
/// "the one corpus document that draws it" *is* the claim: it says the rest do not.
pub const UNIQUE: [&str; 3] = ["one", "only", "sole"];

/// The nouns for a corpus.
pub const CORPUS: [&str; 2] = ["corpus", "corpora"];

/// The nouns a corpus is measured in.
///
/// Documents and nothing else. A corpus also holds pages, glyphs and signature values, and a
/// claim counting one of those has a denominator this sweep cannot derive — the populations
/// below are counted in files.
pub const DOCUMENTS: [&str; 4] = ["document", "documents", "file", "files"];

/// The words that put a cardinal in front of a denominator rather than in front of a count.
///
/// "Over the 974", "all 67 460", "of the 974" — the cardinal states the size of the population
/// the claim is made over. A bare cardinal in front of a noun counts a subset, which is
/// [`crate::parts::DEFINITE`]'s rule arriving at the same place from the other side.
pub const DENOMINATING: [&str; 4] = ["the", "all", "over", "of"];

/// How many words may sit between the quantifier and the noun it governs.
///
/// **This is where the sweep departs from the twenty-second's adjacency rule, and the departure
/// is the calibration's.** [`crate::parts`] refuses to read across a modifier because "both
/// *native* hosts" counts a subset of the hosts and says nothing about how many hosts there are.
/// Here the modifier does the opposite: "nine **signed** corpus documents" narrows *what* is
/// counted and leaves the population it is counted over exactly where it was — and that
/// sentence, in `doc/verify.md`'s recipe for the `cms` target, is one of the four this sweep was
/// built from. Adjacency would have missed it.
///
/// Bounded rather than unbounded, and terminated by [`FUNCTION_WORDS`], because a quantifier
/// reaches its noun across adjectives and not across a clause: "no *more than* four" is a
/// comparison rather than a claim about a corpus.
pub const MAX_MODIFIERS: usize = 3;

/// The words that end a noun phrase, so that a quantifier does not reach past them.
pub const FUNCTION_WORDS: [&str; 24] = [
    "more", "fewer", "less", "than", "in", "to", "for", "with", "by", "and", "or", "but", "is",
    "are", "was", "were", "has", "have", "had", "that", "which", "from", "as", "at",
];

/// The units that make a cardinal a magnitude rather than a count of documents.
///
/// "The corpus gate reads a 96 MB document" counts megabytes; the noun after the unit is what
/// the magnitude is *of*, and reading across one would make every size in this tree a claim
/// about a corpus.
pub const UNITS: [&str; 9] = ["b", "kb", "mb", "gb", "kib", "mib", "gib", "pt", "px"];

/// The words a partitive quantifier reaches its noun through.
///
/// "One of the corpus documents" is a uniqueness over the corpus written the long way round, and
/// it is as much a claim as "the corpus's one document" is.
pub const PARTITIVE: [&str; 4] = ["of", "the", "its", "our"];

/// The smallest cardinal that is read as naming a population rather than counting something.
///
/// **Denomination removes a hit**, so this is the one direction in which being loose hides a
/// case rather than adding noise, and the threshold is here for that reason. Below a hundred a
/// cardinal in one of these sentences is a count of what the claim is about far more often than
/// the size of a corpus — "the four corpus documents", "the eight locked ones" — and reading it
/// as a denominator would silence exactly the claims this sweep is for. What it costs is that a
/// population smaller than this cannot be named by its size; the report prints every population
/// with its **name** beside its count, and a sentence naming one of those names is denominated
/// whatever its arithmetic.
pub const MIN_CARDINAL: usize = 100;

/// This project's own words for a population that no directory name spells.
///
/// Written down rather than derived, because nothing on disk states them — and kept to the words
/// that name a *population* rather than a property of one. `crawl` is `corpus-cache/safedocs`,
/// `curated` is what this project calls the submodules and the ratcheted corpus together,
/// `submodule` is how a sentence says "not the crawl" without naming which, and **both
/// populations** is the phrase a re-derivation over the ratcheted corpus and the crawl together
/// writes — which is a denominator, and the one a round paying this sweep leaves behind.
pub const OUR_WORDS: [&str; 6] = [
    "crawl",
    "crawls",
    "curated",
    "submodule",
    "submodules",
    "both populations",
];

/// The corpus this project's ratchets are measured over.
///
/// The one root that is named here rather than derived, and the reason is that no directory
/// listing distinguishes it: `doc/pdf.js` is a checkout of another project, whose test documents
/// are one directory inside it. Every other root below is a directory this tree keeps *for*
/// holding documents, so listing its parent is the derivation.
pub const RATCHETED: &str = "doc/pdf.js/test/pdfs";

/// What that root is called in a sentence.
pub const RATCHETED_NAME: &str = "pdf.js";

/// The directories whose immediate children are each a corpus.
pub const CONTAINERS: [&str; 2] = ["doc/corpora", "corpus-cache"];

/// The directory under `doc/` whose prose carries its own date.
///
/// [`crate::parts::DATED`]'s rule: an ADR number is a date, and a record counting a population
/// counted correctly on the day it was written.
pub const DATED: &str = "doc/adr/";

/// One corpus this tree holds, as the filesystem states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corpus {
    /// Where it is, as this project writes a path.
    pub path: String,
    /// What a sentence calls it.
    pub name: String,
    /// How many PDFs are under it.
    pub documents: usize,
}

/// Every population a claim about "the corpus" could be about.
///
/// Read off the disk rather than written down, for [`crate::parts::Membership`]'s reason: a
/// sweep whose right-hand side is a constant measures the session that wrote it (ADR 0397).
#[derive(Debug, Clone, Default)]
pub struct Populations {
    /// Each corpus, in path order.
    corpora: Vec<Corpus>,
}

/// Why the populations could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A directory could not be listed.
    #[error("{path}: {source}")]
    Unreadable {
        /// What was being read.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
}

impl Populations {
    /// Counts every corpus under `root`.
    ///
    /// A root that is not checked out contributes nothing and is not an error: `doc/corpora/`'s
    /// submodules are optional in the strong sense (`doc/environment.md`), and `corpus-cache/`
    /// is machine-local. What the report prints is the populations it *found*, so a tree without
    /// the crawl says so rather than judging a claim against a world it cannot see.
    ///
    /// # Errors
    ///
    /// [`Error::Unreadable`] where a directory that exists cannot be walked. A sweep that
    /// skipped what it could not open would answer a claim about the corpus with a count of part
    /// of it.
    pub fn read(root: &Path) -> Result<Self, Error> {
        let mut corpora = Vec::new();
        let ratcheted = root.join(RATCHETED);
        if ratcheted.is_dir() {
            corpora.push(Corpus {
                path: RATCHETED.to_owned(),
                name: RATCHETED_NAME.to_owned(),
                documents: documents_under(&ratcheted)?,
            });
        }
        for container in CONTAINERS {
            let base = root.join(container);
            let Ok(listing) = std::fs::read_dir(&base) else {
                continue;
            };
            let mut found: Vec<PathBuf> = Vec::new();
            for entry in listing {
                let entry = entry.map_err(|source| Error::Unreadable {
                    path: base.display().to_string(),
                    source,
                })?;
                if entry.path().is_dir() {
                    found.push(entry.path());
                }
            }
            found.sort();
            for directory in found {
                let Some(name) = directory.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                let documents = documents_under(&directory)?;
                if documents == 0 {
                    continue;
                }
                corpora.push(Corpus {
                    path: format!("{container}/{name}"),
                    name: name.to_owned(),
                    documents,
                });
            }
        }
        Ok(Self { corpora })
    }

    /// The populations, in path order.
    #[must_use]
    pub fn corpora(&self) -> &[Corpus] {
        &self.corpora
    }

    /// Every document this tree holds.
    #[must_use]
    pub fn whole(&self) -> usize {
        self.corpora.iter().fold(0usize, |total, corpus| {
            total.saturating_add(corpus.documents)
        })
    }

    /// Whether `count` is the size of a population this tree holds, the whole included.
    #[must_use]
    pub fn holds_size(&self, count: usize) -> bool {
        count == self.whole() || self.corpora.iter().any(|corpus| corpus.documents == count)
    }

    /// The name of each population, lower-cased, for a sentence to be searched for.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.corpora
            .iter()
            .map(|corpus| corpus.name.to_ascii_lowercase())
            .collect()
    }
}

/// Every PDF under one directory, counted recursively.
///
/// A symbolic link to a directory is not followed: a parallel worktree reaches the corpora
/// through links into the main checkout, and following one from `doc/corpora/` would count the
/// same documents twice.
fn documents_under(directory: &Path) -> Result<usize, Error> {
    let listing = std::fs::read_dir(directory).map_err(|source| Error::Unreadable {
        path: directory.display().to_string(),
        source,
    })?;
    let mut count = 0usize;
    for entry in listing {
        let entry = entry.map_err(|source| Error::Unreadable {
            path: directory.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            if path.is_symlink() {
                continue;
            }
            count = count.saturating_add(documents_under(&path)?);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

/// What a claim asserts about how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {
    /// The corpus holds none of it — refutable by one witness.
    Absent,
    /// The corpus holds exactly one — refutable by a second.
    Unique,
    /// The corpus holds this many, and refuting it takes a re-census.
    Counted,
}

impl Shape {
    /// One word for the report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "an absence",
            Self::Unique => "a uniqueness",
            Self::Counted => "a count",
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One quantification over a corpus, before the sentence is asked what it is denominated by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// What it asserts about how many.
    pub shape: Shape,
    /// The word that made the claim.
    pub word: String,
    /// The noun it governs.
    pub noun: String,
}

/// What denominates a sentence, where anything does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Denominator {
    /// A population's own name, or one of [`OUR_WORDS`].
    Named(String),
    /// A cardinal that is the size of a population this tree holds.
    Size(usize),
    /// A cardinal under [`DENOMINATING`] that no population has.
    Unmatched(usize),
}

/// How close the disagreement is, and therefore which hits to read first.
///
/// The ordering is **shape first, then place**, and both halves are the report's argument. An
/// absence is refutable by one witness where a count needs a re-census, so an absence is what a
/// round can act on in an afternoon; and among absences, the ledger is where a claim about the
/// corpus decides a *status*, the code is where it decides what a fixture is for, and a document
/// is where it decides what the next round reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// A recipe that walks some of this tree's corpora and not the rest.
    Recipe,
    /// An absence or a uniqueness in a ledger note, naming no population.
    Ledger,
    /// The same, in a comment beside the code.
    Code,
    /// The same, in one of this project's undated documents.
    Prose,
    /// A count, naming no population, wherever it is written.
    Counted,
    /// A numeral denominator no population on disk has.
    Unmatched,
}

impl Rung {
    /// One line saying what this rung is, for the report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recipe => {
                "an invocation that walks some of this tree's corpora and not the rest, so what \
                 it harvests is a population and not the population"
            }
            Self::Ledger => {
                "an absence or a uniqueness in a ledger note, over a corpus it does not name — \
                 one witness anywhere refutes it, and the row's status rests on it"
            }
            Self::Code => "the same, beside the code, where it says what a fixture is for",
            Self::Prose => "the same, in an undated document",
            Self::Counted => {
                "a count over a corpus the sentence does not name — refutable by a re-census \
                 rather than by a witness"
            }
            Self::Unmatched => {
                "a denominator stated as a number, which no population on disk has — a subset, \
                 or a population that moved"
            }
        }
    }
}

/// What one finding is about.
#[derive(Debug, Clone)]
pub enum What {
    /// A quantification over a corpus the sentence does not name.
    Claim {
        /// What it asserts.
        claim: Claim,
        /// The numeral it denominated itself by, where the rung is [`Rung::Unmatched`].
        stated: Option<usize>,
    },
    /// An invocation that walks some of this tree's corpora and not the rest.
    Recipe {
        /// The corpora it names.
        names: Vec<String>,
        /// The corpora it leaves out.
        omits: Vec<String>,
    },
}

/// One place whose population is narrower than the tree's, or cannot be read off it.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Where it is, as `path:line`.
    pub location: String,
    /// What it is about.
    pub what: What,
    /// Which rung it sits on.
    pub rung: Rung,
    /// The sentence or the invocation, because a hit is a reading list and not a verdict.
    pub sentence: String,
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// How many claims quantify over a corpus at all.
    pub claims: usize,
    /// How many of those name a population this tree holds.
    pub denominated: usize,
    /// Which denominator each of those named, and how often.
    ///
    /// Printed rather than summed, because **denomination removes a hit** and a name that
    /// denominates by accident is the one way this sweep can be quiet about a claim. `pdfbox` is
    /// a corpus under `doc/corpora/` *and* the frozen text-extraction reference, so a tally
    /// that suddenly grows under one name is the reader's cue to look.
    pub named: Vec<(String, usize)>,
    /// How many sit in a dated record, and are counted rather than listed.
    pub dated: usize,
    /// How many invocations name a corpus at all, whether or not they name all of them.
    pub recipes: usize,
    /// The rest, closest rung first.
    pub findings: Vec<Finding>,
    /// The populations judged against, for the line that says what a clean run was clean over.
    pub populations: Vec<Corpus>,
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
}

/// Runs the sweep over the ledger's notes, the tree's comments and this project's prose.
///
/// `sources` are the Rust files under [`crate::SOURCE_ROOTS`] and `documents` the Markdown under
/// `doc/`. Two directories are read by nothing, for the reasons [`crate::retired::NOT_SWEPT`]
/// and [`crate::NOT_SCANNED`] give: a round's own record is not another round's to correct, and
/// this checker's own prose states the example sentences.
#[must_use]
pub fn sweep(
    populations: &Populations,
    ledger: &Ledger,
    sources: &[(PathBuf, String)],
    documents: &[(PathBuf, String)],
) -> Report {
    let mut report = Report {
        populations: populations.corpora().to_vec(),
        ..Report::default()
    };
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    for (location, place, block) in places(ledger, sources, documents) {
        for sentence in crate::unread::sentences(&block) {
            read_sentence(
                populations,
                &location,
                place,
                sentence,
                &mut report,
                &mut tally,
            );
        }
    }
    read_recipes(populations, documents, &mut report);

    report.findings.sort_by(|left, right| {
        left.rung
            .cmp(&right.rung)
            .then_with(|| left.location.cmp(&right.location))
    });
    report.named = tally.into_iter().collect();
    report
        .named
        .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    report
}

/// Every place a claim can be written, with the rung its kind puts an absence on.
fn places(
    ledger: &Ledger,
    sources: &[(PathBuf, String)],
    documents: &[(PathBuf, String)],
) -> Vec<(String, Rung, String)> {
    let mut places: Vec<(String, Rung, String)> = Vec::new();
    for row in &ledger.rows {
        if let Some(note) = row.note.as_deref() {
            places.push((
                format!("{}:{} (\u{a7}{})", crate::LEDGER, row.line, row.clause),
                Rung::Ledger,
                note.to_owned(),
            ));
        }
    }
    for (path, text) in sources {
        let shown = shown(path);
        if shown.starts_with(crate::NOT_SCANNED) {
            continue;
        }
        for (line, block) in crate::blockers::comment_blocks(text) {
            places.push((format!("{shown}:{line}"), Rung::Code, block));
        }
    }
    for (path, text) in documents {
        let shown = shown(path);
        if shown.starts_with(crate::retired::NOT_SWEPT) {
            continue;
        }
        for (line, block) in crate::retired::paragraphs(text) {
            places.push((format!("{shown}:{line}"), Rung::Prose, block));
        }
        for (line, block) in fenced_prose(text) {
            places.push((format!("{shown}:{line}"), Rung::Prose, block));
        }
    }
    places
}

/// Reads one sentence, counting what it denominates and recording what it does not.
fn read_sentence(
    populations: &Populations,
    location: &str,
    place: Rung,
    sentence: &str,
    report: &mut Report,
    tally: &mut BTreeMap<String, usize>,
) {
    let claims = claims_in(sentence);
    if claims.is_empty() {
        return;
    }
    let denominator = denominator_of(sentence, populations, &populations.names());
    for claim in claims {
        report.claims = report.claims.saturating_add(1);
        let rung = match &denominator {
            Some(Denominator::Named(name)) => {
                report.denominated = report.denominated.saturating_add(1);
                let seen = tally.entry(name.clone()).or_default();
                *seen = seen.saturating_add(1);
                continue;
            }
            Some(Denominator::Size(count)) => {
                report.denominated = report.denominated.saturating_add(1);
                let seen = tally.entry(count.to_string()).or_default();
                *seen = seen.saturating_add(1);
                continue;
            }
            Some(Denominator::Unmatched(_)) => Rung::Unmatched,
            None if claim.shape == Shape::Counted => Rung::Counted,
            None => place,
        };
        if location.starts_with(DATED) {
            report.dated = report.dated.saturating_add(1);
            continue;
        }
        report.findings.push(Finding {
            location: location.to_owned(),
            what: What::Claim {
                claim,
                stated: match &denominator {
                    Some(Denominator::Unmatched(count)) => Some(*count),
                    _ => None,
                },
            },
            rung,
            sentence: sentence.to_owned(),
        });
    }
}

/// Reads the fenced invocations, which are the one population here that is not a sentence.
fn read_recipes(populations: &Populations, documents: &[(PathBuf, String)], report: &mut Report) {
    for (path, text) in documents {
        let shown = shown(path);
        // A decision record's invocation is dated exactly as its prose is, and it was run over
        // the corpora that existed on its date - [`DATED`]'s rule, applied to the one population
        // here that is not a sentence.
        if shown.starts_with(crate::retired::NOT_SWEPT) || shown.starts_with(DATED) {
            continue;
        }
        for (line, block) in fenced(text) {
            let Some((names, omits)) = walked(&block, populations) else {
                continue;
            };
            report.recipes = report.recipes.saturating_add(1);
            if omits.is_empty() {
                continue;
            }
            report.findings.push(Finding {
                location: format!("{shown}:{line}"),
                what: What::Recipe { names, omits },
                rung: Rung::Recipe,
                sentence: one_line(&block),
            });
        }
    }
}

/// A path as this project writes one.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Every claim one sentence quantifies over a corpus.
///
/// Three forms, and each is adjacency-bound so that a claim about a subset is not read as a
/// claim about the whole:
///
/// - a quantifier immediately before one of [`CORPUS`] — "no corpus document", "four corpus
///   documents", "the only corpus";
/// - a quantifier immediately before one of [`DOCUMENTS`], in a sentence that mentions a corpus
///   — "no document in the corpus carries one";
/// - one of [`CORPUS`] immediately before a quantifier, which is the possessive — "the corpus's
///   one witness".
#[must_use]
pub fn claims_in(sentence: &str) -> Vec<Claim> {
    let words: Vec<String> = sentence.split_whitespace().map(bare).collect();
    let mentions_a_corpus = words.iter().any(|word| CORPUS.contains(&word.as_str()));
    if !mentions_a_corpus {
        return Vec::new();
    }
    let mut found = Vec::new();
    for (position, word) in words.iter().enumerate() {
        let after = |offset: usize| words.get(position.saturating_add(offset));
        if let Some(shape) = shape_of(word) {
            if let Some(noun) = head_after(&words, position) {
                found.push(Claim {
                    shape,
                    word: word.clone(),
                    noun,
                });
            }
            continue;
        }
        // The possessive: `bare` takes the `'s` off, so "the corpus's one witness" arrives here
        // as a corpus noun followed by a quantifier.
        if !CORPUS.contains(&word.as_str()) {
            continue;
        }
        let Some(next) = after(1) else { continue };
        let Some(shape) = shape_of(next) else {
            continue;
        };
        found.push(Claim {
            shape,
            word: next.clone(),
            noun: after(2).cloned().unwrap_or_else(|| next.clone()),
        });
    }
    found
}

/// The noun a quantifier at `position` governs, where it governs one of this sweep's.
///
/// Scans forward at most [`MAX_MODIFIERS`] words, through an adjective or one of [`PARTITIVE`]
/// and never past one of [`FUNCTION_WORDS`]. A head that is one of [`CORPUS`] hands back the
/// word after it — "corpus documents" counts documents — and a head that is one of
/// [`DOCUMENTS`] hands back itself. A [`UNITS`] word ends the phrase for the same reason a
/// [`FUNCTION_WORDS`] one does: what follows it is what the magnitude is of.
fn head_after(words: &[String], position: usize) -> Option<String> {
    for step in 1..=MAX_MODIFIERS.saturating_add(1) {
        let at = position.checked_add(step)?;
        let word = words.get(at)?.as_str();
        if CORPUS.contains(&word) {
            return Some(
                words
                    .get(at.saturating_add(1))
                    .cloned()
                    .unwrap_or_else(|| word.to_owned()),
            );
        }
        if DOCUMENTS.contains(&word) {
            return Some(word.to_owned());
        }
        if FUNCTION_WORDS.contains(&word) || UNITS.contains(&word) {
            return None;
        }
        if !PARTITIVE.contains(&word) && !word.chars().all(char::is_alphabetic) {
            return None;
        }
    }
    None
}

/// What one word asserts about how many, where it asserts anything.
fn shape_of(word: &str) -> Option<Shape> {
    if ABSENT.contains(&word) {
        return Some(Shape::Absent);
    }
    if UNIQUE.contains(&word) {
        return Some(Shape::Unique);
    }
    cardinal(word)
        .filter(|count| *count > 1)
        .map(|_| Shape::Counted)
}

/// What a sentence names as its population, where it names anything.
///
/// A name is looked for anywhere in the sentence and a numeral only under [`DENOMINATING`],
/// which is the difference between saying which corpus and counting something in one.
#[must_use]
pub fn denominator_of(
    sentence: &str,
    populations: &Populations,
    names: &[String],
) -> Option<Denominator> {
    let lowered = sentence.to_ascii_lowercase();
    let spelled = tokens(&lowered);
    for name in names.iter().map(String::as_str).chain(OUR_WORDS) {
        let found = if name.contains(' ') {
            lowered.contains(name)
        } else {
            spelled.iter().any(|token| token == name)
        };
        if found {
            return Some(Denominator::Named(name.to_owned()));
        }
    }
    let words: Vec<String> = sentence.split_whitespace().map(bare).collect();
    let mut unmatched = None;
    for position in 0..words.len() {
        let before = position
            .checked_sub(1)
            .and_then(|at| words.get(at))
            .map(String::as_str);
        if !before.is_some_and(|before| DENOMINATING.contains(&before)) {
            continue;
        }
        let Some(count) = grouped(&words, position) else {
            continue;
        };
        if count < MIN_CARDINAL {
            continue;
        }
        if populations.holds_size(count) {
            return Some(Denominator::Size(count));
        }
        unmatched.get_or_insert(count);
    }
    unmatched.map(Denominator::Unmatched)
}

/// Every fenced block of a Markdown document, with the line its fence opens on.
///
/// The one population in this sweep that is *not* prose, and it is here because two of the four
/// rounds that found this defect found it in an invocation rather than in a sentence: a recipe
/// that walks one corpus harvests one corpus, and the count it produces is then written down as
/// a fact about the tree. [`crate::prose::blocks`] and [`crate::retired::paragraphs`] both skip
/// a fence, correctly — a shell line is not a claim about the standard — so this reads what they
/// put down.
fn fenced(text: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<(usize, Vec<String>)> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            match current.take() {
                Some((start, lines)) => blocks.push((start, lines.join("\n"))),
                None => current = Some((index.saturating_add(2), Vec::new())),
            }
            continue;
        }
        if let Some((_, lines)) = current.as_mut() {
            lines.push(line.to_owned());
        }
    }
    blocks
}

/// The prose written *inside* a fenced invocation, as comment lines.
///
/// `doc/verify.md` states each fuzz target's whole argument in `#` comments beside the command
/// that runs it, and one of those comments is the fourth of the four findings this sweep was
/// built from: "the eleven `/Contents` blobs the **nine signed corpus documents** hold". Both
/// readers of Markdown in this crate skip a fence, correctly and for the same reason — a shell
/// line is not a sentence — so that claim was in no population at all until this function
/// existed. Only comment lines are taken, which is what keeps the commands themselves out.
fn fenced_prose(text: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    for (start, block) in fenced(text) {
        let mut current: Option<(usize, Vec<String>)> = None;
        for (index, line) in block.lines().enumerate() {
            let trimmed = line.trim_start();
            let comment = trimmed
                .strip_prefix('#')
                .or_else(|| trimmed.strip_prefix("//"));
            match (comment, current.as_mut()) {
                (Some(body), Some((_, lines))) => lines.push(body.trim().to_owned()),
                (Some(body), None) => {
                    current = Some((start.saturating_add(index), vec![body.trim().to_owned()]));
                }
                (None, _) => {
                    if let Some((line, lines)) = current.take() {
                        blocks.push((line, lines.join(" ")));
                    }
                }
            }
        }
        if let Some((line, lines)) = current.take() {
            blocks.push((line, lines.join(" ")));
        }
    }
    blocks
}

/// Which corpora an invocation walks, and which it leaves out, where it walks any.
///
/// A root counts as walked when the block names its **directory**. A path ending in `.pdf` is
/// one document rather than a population, which is what keeps every example opening a single
/// file off this rung; and a bare container — `doc/corpora`, `corpus-cache` — walks every corpus
/// under it, which is what `find -L doc/corpora corpus-cache` means and is not a narrowing of
/// those.
///
/// `None` where the block names no corpus at all, which is almost every fenced block in the
/// tree.
fn walked(block: &str, populations: &Populations) -> Option<(Vec<String>, Vec<String>)> {
    let mut names: Vec<String> = Vec::new();
    let mut omits: Vec<String> = Vec::new();
    let words: Vec<&str> = block.split_whitespace().collect();
    for corpus in populations.corpora() {
        let container = corpus
            .path
            .rsplit_once('/')
            .map_or(corpus.path.as_str(), |(head, _)| head);
        let walks = words.iter().any(|word| {
            let word = word.trim_matches(|character: char| {
                !character.is_ascii_alphanumeric() && character != '/' && character != '.'
            });
            if word.to_ascii_lowercase().ends_with(".pdf") {
                return false;
            }
            word.contains(corpus.path.as_str())
                || (CONTAINERS.contains(&container) && word.contains(container))
        });
        if walks {
            names.push(corpus.name.clone());
        } else {
            omits.push(corpus.name.clone());
        }
    }
    (!names.is_empty()).then_some((names, omits))
}

/// A fenced block as one line, short enough to sit under a finding.
fn one_line(block: &str) -> String {
    let joined = block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join(" ");
    match joined.char_indices().nth(RECIPE_SHOWN) {
        Some((at, _)) => format!("{}\u{2026}", joined.get(..at).unwrap_or_default()),
        None => joined,
    }
}

/// How much of an invocation is printed under a finding.
///
/// Enough to tell one recipe from another; the location is what a reader opens.
const RECIPE_SHOWN: usize = 220;

/// The tokens of an already-lower-cased sentence, as a *name* is written in one.
///
/// A path separator, a backtick and every other punctuation mark divide, so `doc/corpora/pdfbox`
/// offers `pdfbox`; a dot and a hyphen **inside** a token do not, so `pdf.js` and
/// `format-corpus` survive whole. That pairing is the whole of what a substring test got wrong:
/// `PDFBOX-4352-0.pdf` is a document in the *ratcheted* corpus and contains a corpus's name, so
/// a sentence naming it read as a sentence naming that corpus — and denomination is the one
/// direction in which being loose makes this sweep quiet about a claim rather than noisy.
fn tokens(lowered: &str) -> Vec<String> {
    lowered
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && character != '.' && character != '-'
        })
        .map(|token| token.trim_matches(['.', '-']).to_owned())
        .filter(|token| !token.is_empty())
        .collect()
}

/// A cardinal at `position`, joined across the spaces this project sets its thousands with.
///
/// `doc/`'s own typography writes sixty-seven thousand as `67 460`, so a reader splitting on
/// whitespace sees two numbers and a denominator of sixty-seven. Only a following group of
/// **exactly three** digits is joined, which is what a thousands separator is and what keeps
/// "the 974 documents in 12 directories" from becoming one number.
fn grouped(words: &[String], position: usize) -> Option<usize> {
    let mut count = cardinal(words.get(position)?)?;
    let mut at = position;
    loop {
        at = at.saturating_add(1);
        let Some(next) = words.get(at) else { break };
        if next.len() != 3 || !next.bytes().all(|byte| byte.is_ascii_digit()) {
            break;
        }
        let group: usize = next.parse().ok()?;
        count = count.saturating_mul(1000).saturating_add(group);
    }
    Some(count)
}

/// One word as a cardinal, in digits or in English.
///
/// [`crate::counts::cardinal_of`] answers the English forms and the digits below a hundred; the
/// populations here are counted in thousands, so a digit string of any length is read as well.
fn cardinal(word: &str) -> Option<usize> {
    if !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit()) {
        if word.len() > 1 && word.starts_with('0') {
            return None;
        }
        return word.parse().ok();
    }
    crate::counts::cardinal_of(word)
}

/// One word with this project's markup, punctuation and possessive removed, lower-cased.
///
/// [`crate::parts`]' own, and the two must agree: a sentence is read the same way by every sweep
/// that reads sentences.
fn bare(word: &str) -> String {
    let trimmed = word.trim_matches(|character: char| !character.is_ascii_alphanumeric());
    let trimmed = trimmed
        .strip_suffix("'s")
        .or_else(|| trimmed.strip_suffix("\u{2019}s"))
        .unwrap_or(trimmed);
    trimmed.to_ascii_lowercase()
}

/// Every name a population goes by, for a caller that has no [`Populations`] to hand.
#[must_use]
pub fn our_words() -> BTreeSet<String> {
    OUR_WORDS.iter().map(|word| (*word).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        Claim, Denominator, Populations, Rung, Shape, claims_in, denominator_of, our_words, sweep,
    };
    use crate::clause::ClauseNumber;
    use crate::ledger::{Ledger, Row, Status};

    fn row(clause: &str, note: &str) -> Row {
        Row {
            clause: clause.parse::<ClauseNumber>().expect("a clause number"),
            title: "General".to_owned(),
            status: Status::Partial,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    /// The populations this tree holds, read off its own directories.
    fn populations() -> Populations {
        Populations::read(&crate::workspace_root()).expect("the corpora on disk")
    }

    /// Trap 13's calibration, and it is the defect itself: §12.8.5's sentence, written over the
    /// ratcheted corpus, which a crawl two orders of magnitude wider refuted.
    #[test]
    fn the_sentence_four_rounds_found_is_caught() {
        let claims = claims_in("No corpus document carries a document timestamp.");
        assert_eq!(
            claims,
            vec![Claim {
                shape: Shape::Absent,
                word: "no".to_owned(),
                noun: "document".to_owned(),
            }]
        );
    }

    /// The other half of the calibration: a sentence that says which corpus is not a hit, and
    /// both spellings of saying so — the population's name, and its size.
    #[test]
    fn a_sentence_that_names_its_denominator_is_not_a_finding() {
        let populations = populations();
        let names = populations.names();
        let whole = populations.whole();
        for sentence in [
            "No document in the pdf.js corpus carries a document timestamp.",
            "No corpus document in the crawl carries a document timestamp.",
        ] {
            assert!(
                matches!(
                    denominator_of(sentence, &populations, &names),
                    Some(Denominator::Named(_))
                ),
                "{sentence}"
            );
        }
        let by_size = format!("No corpus document of the {whole} carries a document timestamp.");
        assert_eq!(
            denominator_of(&by_size, &populations, &names),
            Some(Denominator::Size(whole)),
            "{by_size}"
        );
    }

    /// The departure from the twenty-second sweep's adjacency rule, and the sentence that
    /// earned it: `doc/verify.md`'s `cms` recipe said "the nine **signed** corpus documents",
    /// which a wider population falsified and which adjacency would never have printed. A
    /// modifier narrows what is counted and leaves the denominator alone, which is the question
    /// here.
    #[test]
    fn a_modifier_narrows_what_is_counted_and_not_the_population_it_is_counted_over() {
        assert_eq!(
            claims_in("Seed it with the blobs the nine signed corpus documents hold.")
                .first()
                .map(|claim| (claim.shape, claim.noun.as_str())),
            Some((Shape::Counted, "documents"))
        );
        assert_eq!(
            claims_in("No encrypted corpus document carries one.")
                .first()
                .map(|claim| claim.shape),
            Some(Shape::Absent)
        );
        assert_eq!(
            claims_in("One of the corpus documents states it.")
                .first()
                .map(|claim| claim.shape),
            Some(Shape::Unique),
            "a partitive is a claim written the long way round"
        );
    }

    /// A quantifier reaches its noun across adjectives and not across a clause, or every
    /// comparison in a sentence about a corpus arrives as a claim about one.
    #[test]
    fn a_quantifier_does_not_reach_past_a_function_word() {
        // `no` is a comparison here and reaches nothing; the `four` beyond it governs its noun
        // directly and is a count over the corpus, which is correct.
        assert_eq!(
            claims_in("No more than four documents of the corpus do."),
            vec![Claim {
                shape: Shape::Counted,
                word: "four".to_owned(),
                noun: "documents".to_owned(),
            }]
        );
        assert_eq!(
            claims_in("Four are documents and the corpus holds them."),
            Vec::new()
        );
    }

    /// A sentence that mentions no corpus at all is judged by nothing: this sweep's denominator
    /// is a population of documents, and a count of anything else has none here.
    #[test]
    fn a_quantifier_outside_a_sentence_about_a_corpus_is_not_a_claim() {
        assert!(claims_in("No document states a `/DocTimeStamp`.").is_empty());
        assert!(claims_in("Four documents write an indefinite length.").is_empty());
    }

    /// The three forms, each of which a round wrote and a later round had to correct.
    #[test]
    fn the_three_forms_a_corpus_claim_takes_are_all_read() {
        let absent = claims_in("No corpus document carries a document timestamp.");
        assert_eq!(absent.first().map(|claim| claim.shape), Some(Shape::Absent));
        let unique = claims_in("The corpus's one certification signature states `/P 2`.");
        assert_eq!(unique.first().map(|claim| claim.shape), Some(Shape::Unique));
        assert_eq!(
            unique.first().map(|claim| claim.noun.as_str()),
            Some("certification")
        );
        let counted = claims_in("Four corpus documents write an indefinite length.");
        assert_eq!(
            counted.first().map(|claim| claim.shape),
            Some(Shape::Counted)
        );
    }

    /// The thousands separator this project's prose sets with a space, which a reader splitting
    /// on whitespace would otherwise take for sixty-seven.
    #[test]
    fn a_denominator_written_with_a_thousands_space_is_one_number() {
        let populations = populations();
        let names = populations.names();
        let whole = populations.whole();
        let thousands = whole.checked_div(1000).expect("a divisor that is not zero");
        let rest = whole.checked_rem(1000).expect("a divisor that is not zero");
        let sentence =
            format!("No corpus document of the {thousands} {rest:03} carries a timestamp.");
        assert_eq!(
            denominator_of(&sentence, &populations, &names),
            Some(Denominator::Size(whole)),
            "{sentence}"
        );
    }

    /// A document *named* in one corpus can spell another corpus's name, and reading that as a
    /// denominator is how this sweep would go quiet about a claim. `PDFBOX-4352-0.pdf` is a
    /// document of the ratcheted corpus and `pdfbox` is a corpus under `doc/corpora/`.
    #[test]
    fn a_documents_own_name_does_not_denominate_the_sentence_that_names_it() {
        let populations = populations();
        let names = populations.names();
        assert_eq!(
            denominator_of(
                "No corpus document but `PDFBOX-4352-0.pdf` states it.",
                &populations,
                &names
            ),
            None
        );
        assert!(
            matches!(
                denominator_of(
                    "No corpus document under `doc/corpora/pdfbox` states it.",
                    &populations,
                    &names
                ),
                Some(Denominator::Named(_))
            ),
            "a path names the corpus it ends in"
        );
    }

    /// A cardinal below [`super::MIN_CARDINAL`] counts what the claim is about rather than
    /// naming the population, and reading it as a denominator would silence the claim.
    #[test]
    fn a_small_cardinal_does_not_denominate() {
        let populations = populations();
        let names = populations.names();
        assert_eq!(
            denominator_of("The four corpus documents write it.", &populations, &names),
            None
        );
    }

    /// A ledger note is on the tree's own rung; a decision record is dated and counted.
    #[test]
    fn a_note_is_listed_and_a_decision_record_is_counted() {
        let populations = populations();
        let ledger = Ledger {
            rows: vec![
                row("12.8.5", "No corpus document carries a document timestamp."),
                row(
                    "12.8.3.4.2",
                    "No document in the pdf.js corpus carries a document timestamp.",
                ),
            ],
        };
        let report = sweep(&populations, &ledger, &[], &[]);
        assert_eq!(report.claims, 2);
        assert_eq!(report.denominated, 1);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.on(Rung::Ledger), 1);
        assert_eq!(
            report
                .named
                .first()
                .map(|(name, count)| (name.as_str(), *count)),
            Some(("pdf.js", 1)),
            "a denomination says which name answered it, because a name that answers by \
             accident is the one way this sweep goes quiet"
        );

        let documents = vec![(
            std::path::PathBuf::from("doc/adr/0754-a-record.md"),
            "No corpus document carries a document timestamp.\n".to_owned(),
        )];
        let dated = sweep(&populations, &Ledger { rows: Vec::new() }, &[], &documents);
        assert_eq!(dated.dated, 1);
        assert!(dated.findings.is_empty());
    }

    /// The ordering is shape first and place second, and a catch-all arm that read the place
    /// before the shape would put a count above an absence — which is the reading order this
    /// sweep exists to get right.
    #[test]
    fn a_count_ranks_below_an_absence_wherever_each_is_written() {
        let populations = populations();
        let ledger = Ledger {
            rows: vec![row(
                "8.7.3.1",
                "21 corpus documents ask for something else.",
            )],
        };
        let sources = vec![(
            std::path::PathBuf::from("crates/pdf-model/src/cms.rs"),
            "//! No corpus document carries a document timestamp.\n".to_owned(),
        )];
        let documents = vec![(
            std::path::PathBuf::from("doc/verify.md"),
            "The corpus's one witness is a fixture.\n".to_owned(),
        )];
        let report = sweep(&populations, &ledger, &sources, &documents);
        assert_eq!(
            report
                .findings
                .iter()
                .map(|finding| finding.rung)
                .collect::<Vec<Rung>>(),
            vec![Rung::Code, Rung::Prose, Rung::Counted]
        );
    }

    /// The checker's own prose states the example sentences, so it witnesses nothing.
    #[test]
    fn the_checkers_own_documentation_is_not_a_place() {
        let populations = populations();
        let sources = vec![(
            std::path::PathBuf::from("tools/conformance/src/undenominated.rs"),
            "//! No corpus document carries a document timestamp.\n".to_owned(),
        )];
        let report = sweep(&populations, &Ledger { rows: Vec::new() }, &sources, &[]);
        assert_eq!(report.claims, 0);
    }

    /// The right-hand side is the disk, and this tree holds more than one corpus. A count of
    /// zero would denominate nothing and print every claim in the tree.
    #[test]
    fn the_corpora_on_disk_are_what_answers_a_claim() {
        let populations = populations();
        assert!(
            populations.corpora().len() > 1,
            "this tree holds more than one corpus"
        );
        let whole = populations.whole();
        for corpus in populations.corpora() {
            assert!(corpus.documents > 0, "{} is empty", corpus.path);
            assert!(corpus.documents <= whole);
            assert!(populations.holds_size(corpus.documents));
        }
        assert!(populations.holds_size(whole));
        assert!(our_words().contains("crawl"));
    }
}
