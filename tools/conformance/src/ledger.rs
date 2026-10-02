//! The conformance ledger: one row per subclause of the standard's normative clauses.
//!
//! # What it is for
//!
//! The two gates this project already runs — the corpus and the reference oracle — both
//! take a file set as their universe, so both answer "what share of the documents that exist
//! do we draw correctly". Neither can rank a requirement no document exercises, notice a
//! clause nobody implemented, or do anything but declare success when the last file goes
//! green. The ledger is the other denominator: the standard's own requirements, one row
//! each, with a status a person sets after reading the clause against this code.
//!
//! It is deliberately not computed. A status here is a claim someone makes and signs with a
//! code site and a test; what the checker verifies is that the claim is *well formed* — the
//! clause exists, the evidence it names exists, an exclusion is one principle 5 allows —
//! never that the code implements the clause. Only reading the clause can establish that,
//! which is why the statuses are words rather than a percentage.
//!
//! # The status that would rot first
//!
//! `out-of-scope`, and it is the one the reader constrains hardest: a row may carry it only
//! with an `exclusion` naming one of `CLAUDE.md` principle 5's closed list. Widening the
//! list means editing principle 5 and [`Exclusion`] together, in a commit that says so.
//! Without that, `out-of-scope` becomes the graveyard every clause goes to once it turns out
//! to be difficult — which is precisely the escape hatch principle 5 refuses.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::citation::Citation;
use crate::clause::{ClauseIndex, ClauseNumber};
use crate::toml_subset::{self, Value};

/// The clauses of the standard's body the ledger covers: conformance through document
/// interchange.
///
/// **Clause 6 is in it** because it states eleven `shall`s and every one of them is
/// addressed to a PDF file or to a PDF processor — §6.3.2.2's three obligations on a processor
/// that renders a page are the ranking `CLAUDE.md`'s *what done means* is written around, and
/// this tree cites that subclause across five crates, so the ledger carries a row for it.
/// ADR 0984.
///
/// The list is a checked claim rather than a constant: [`check`] reads every clause of the
/// standard and reports one that states a `shall` and appears in neither this list nor
/// [`EXCLUDED_CLAUSES`].
pub const NORMATIVE_CLAUSES: [u16; 9] = [6, 7, 8, 9, 10, 11, 12, 13, 14];

/// A clause of the body that states a `shall` and carries no row, with the reason.
///
/// One entry, and it is clause 4. Its nineteen `shall`s bind how *this document* is written
/// and read rather than what a processor does: §4.1's requires the standard's own prose to
/// name a token character "by their INCITS 4-1986 (R2017) (ASCII 7-bit USA codes) character
/// name written in upper case", and §4.2's eighteen are all of the form "[a]ny use of the term
/// X throughout this document shall be inferred as referring to" some other standard. The
/// second eighteen do bind this program — they decide which edition of IEC 61966-2-1 `sRGB`
/// means, which of ISO/IEC 10918 `JPEG` means, which Adobe character collection
/// `Adobe-Japan1` means — but never at a site of their own: each is discharged in the clause
/// that uses the term, and a row here would be a second place to keep that in step. ADR 0984.
///
/// [`check`] reports an entry whose clause states no `shall`, so the list cannot go stale by
/// naming a clause that has stopped needing the excuse (trap 25).
pub const EXCLUDED_CLAUSES: [(u16, &str); 1] = [(
    4,
    "its `shall`s bind the notation of the standard's own prose and the terms it uses, and \
     each term binding is discharged in the clause that uses the term",
)];

/// The standard's normative annexes, which the ledger covers for the same reason.
///
/// Without it the eight letters in it would be outside every instrument this project has: not
/// citable, not checkable and not recorded. `CLAUDE.md`'s scope section names clauses because
/// that is how
/// the standard's *body* is organised, and its closed exclusion list says nothing about an
/// annex — so the annexes are in scope. Annexes
/// A, B, C, G, H, J, M, N and P are informative and stay out: they state no requirement.
/// ADR 0206.
pub const NORMATIVE_ANNEXES: [char; 8] = ['D', 'E', 'F', 'I', 'K', 'L', 'O', 'Q'];

/// The standard's informative annexes, which carry no row and are cited freely.
///
/// The list exists so that [`check`] can tell an annex nobody has classified from one this
/// project decided about. Three of these do print the word `shall` — Annex J twenty times,
/// Annex N twice — and that is not a finding: ISO/IEC Directives Part 2 makes an informative
/// annex a place for information, so a `shall` there restates a requirement clause 7 or
/// clause 10 already states, which is exactly how J.3.1 writes it — "[c]lause 7.3.2, "Boolean
/// objects" clearly states that the keywords shall be true and false". The rows those
/// sentences belong to are clause 7's and clause 10's.
///
/// A, B, C, G, H, J, M, N and P, and with [`NORMATIVE_ANNEXES`] that is every letter the
/// standard prints; [`check`] reports an annex in neither list.
pub const INFORMATIVE_ANNEXES: [char; 9] = ['A', 'B', 'C', 'G', 'H', 'J', 'M', 'N', 'P'];

/// What is known about one subclause.
///
/// The vocabulary exists to keep six different situations from wearing one word: the
/// project *choosing* for a whole clause ([`Status::OutOfScope`]), the project *choosing*
/// for one sentence inside a clause every other requirement of which is executed
/// ([`Status::Departed`]), the project *not knowing* ([`Status::Unreviewed`]), the project
/// *owing out loud* ([`Status::Reported`], and [`Status::Partial`] for part of a clause),
/// the project *owing in silence* ([`Status::Silent`]), and the requirement having no
/// meaning for a screen ([`Status::Inapplicable`]).
///
/// The distinction between the last two kinds of debt is the one this project cares about
/// most: a gap that reports is a gap you can schedule, and a gap that does not is a gap that
/// ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// Every normative requirement in the clause is executed. Names the code and the test.
    Implemented,
    /// Some requirements are implemented; the note says which, and what is reported for the
    /// rest.
    Partial,
    /// Every requirement of the clause is executed except the one the note names, which was
    /// decided against with its cost recorded.
    ///
    /// The project owner's word, given 2026-09-14 in answer to `doc/questions/Q63`: "Add
    /// `departed`." Before it, the ledger could say *decided* for a whole clause
    /// ([`Status::OutOfScope`], [`Status::Inapplicable`]) and had no way to say it about one
    /// sentence inside a clause otherwise implemented — so a departure wore [`Status::Partial`],
    /// the same word as a row nobody has finished, and every figure for how much is left counted
    /// it. ADR 1035 section 3 found twenty such rows and forbade re-statusing them to
    /// [`Status::Implemented`] meanwhile, because that would hide the sentence.
    ///
    /// It settles, so [`Status::owes`] is false for it and [`Ledger::is_aggregate`] lets a parent
    /// above one settle too (ADR 1035 section 5). What keeps it from being the hiding that ADR refused
    /// is that the departure stays visible as a figure of its own — `tools/state.sh` prints a
    /// `departed` count beside `implemented` and `partial`, never folded into either — and that
    /// [`check`] refuses a row whose note names no ADR, since "with its cost recorded" is a claim
    /// about a document somebody can open. ADR 1119.
    Departed,
    /// Deliberately not implemented yet, and detected and reported at runtime rather than
    /// skipped silently. Still owed.
    Reported,
    /// Not implemented, and **nothing says so**: a document exercising this clause is drawn
    /// wrong with nothing reported.
    ///
    /// The most valuable rows in the ledger, and the reason it is worth generating at all.
    /// Every missing *subsystem* in this tree reports — `LZWDecode`, encryption, Type 3
    /// fonts — because somebody wrote the report while deciding not to write the feature.
    /// The gaps that ship are the ones inside something implemented, where the operator is
    /// handled and the code path exists: `Tr` was parsed with four of its eight modes
    /// changing a clip nobody built, `/SMask` was honoured while `/Mask` beside it was not,
    /// knockout groups composite as if they were not knockouts. Reading the clause is the
    /// only thing that finds those, and this status is where the finding goes.
    Silent,
    /// The requirement has no meaning for this device. Not the same as excluded: nothing is
    /// owed, because nothing applies.
    ///
    /// **The definition names no device**, because ISO 32000-2 does not draw that line:
    /// §8.3.2.2's term is a "raster output device *such as a display or a printer*", which is
    /// why §10.5's transfer function is not in this status (ADR 0204). A status whose
    /// *definition* names a device the standard does not is a status that invites the mistake.
    Inapplicable,
    /// The requirement addresses a generator of the *content* this program does not generate.
    ///
    /// It follows `CLAUDE.md`'s authoring exclusion as amended for §7.5.6's incremental update
    /// and by RFC 0002 §11.1, ratified 2026-09-03 (ADR 0816), which admits a whole-file
    /// serializer — `pdf_syntax::serialize` — for deriving a new document from documents that
    /// exist.
    ///
    /// **So the boundary this status now names is `CLAUDE.md`'s own enforceable test: does the
    /// operation invent marks?** This program emits §7.5.2's header, §7.5.3's body, §7.5.4's
    /// and §7.5.8's cross-reference sections, §7.5.5's trailer and §14.4's identifiers, and
    /// decides nothing about what is on a page. A requirement is `writer-side` when it is
    /// addressed to whoever decided the content, or to a construct this program's writers do
    /// not emit — §7.6's encryption on the way out and Annex F's linearisation are both of the
    /// second kind, and both stop being `writer-side` the day a writer emits them.
    ///
    /// `ledger.toml`'s header carries whichever sentence is here, because this enum is where
    /// the generated header's vocabulary lives, and a second copy in the generator would stamp
    /// a corrected sentence back (ADR 0345).
    WriterSide,
    /// Covered by principle 5's closed exclusion list, which the row must name.
    OutOfScope,
    /// Nobody has read this clause against this code. The initial state of every row.
    Unreviewed,
}

impl Status {
    /// The word the ledger writes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Partial => "partial",
            Self::Departed => "departed",
            Self::Reported => "reported",
            Self::Silent => "silent",
            Self::Inapplicable => "inapplicable",
            Self::WriterSide => "writer-side",
            Self::OutOfScope => "out-of-scope",
            Self::Unreviewed => "unreviewed",
        }
    }

    /// Whether a row wearing this status still owes the standard something.
    ///
    /// The five settled statuses are the five ways a row stops being work: the requirement is
    /// executed, it was decided against with its cost recorded, it has no meaning for this
    /// device, it addresses a generator, or principle 5's closed list covers it. The other four
    /// are debt — `partial` and `reported` know what they owe, `silent` does not say it,
    /// `unreviewed` has not been asked. [`Ledger::owing`] is what reads this, and ADR 1035 is why
    /// it is a function rather than a `match` copied per sweep.
    ///
    /// `departed` joined the settled side on the owner's answer to `doc/questions/Q63`, which is
    /// what ADR 1035 section 5's aggregate rule then inherits: a `departed` row owes nothing, so
    /// a parent above it owes nothing on its account. ADR 1119.
    #[must_use]
    pub fn owes(self) -> bool {
        match self {
            Self::Partial | Self::Reported | Self::Silent | Self::Unreviewed => true,
            Self::Implemented
            | Self::Departed
            | Self::Inapplicable
            | Self::WriterSide
            | Self::OutOfScope => false,
        }
    }

    /// Every status, in the order the summary prints them.
    #[must_use]
    pub fn all() -> [Self; 9] {
        [
            Self::Implemented,
            Self::Partial,
            Self::Departed,
            Self::Reported,
            Self::Silent,
            Self::Inapplicable,
            Self::WriterSide,
            Self::OutOfScope,
            Self::Unreviewed,
        ]
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Status {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::all()
            .into_iter()
            .find(|status| status.as_str() == text)
            .ok_or_else(|| {
                let known: Vec<&str> = Self::all().iter().map(|status| status.as_str()).collect();
                format!(
                    "`{text}` is not a status; expected one of {}",
                    known.join(", ")
                )
            })
    }
}

/// The closed list of exclusions `CLAUDE.md` principle 5 states.
///
/// Deliberately an enum rather than free text. An exclusion written as prose is an exclusion
/// nobody can count, and "decided once, each with a reason" is not a property a string has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Exclusion {
    /// Clause 13, multimedia and 3D: a media engine, not a rendering question.
    Multimedia,
    /// XFA, on Annex K's own permission: "a PDF processor may choose to not implement this
    /// feature" (§K.1).
    ///
    /// Annex K is normative and *inside* the standard (ADR 0206), so the exclusion rests on the
    /// permission the annex grants rather than on where it was printed (ADR 0345).
    Xfa,
    /// JavaScript and script-driven form behaviour. Field *appearance* is not excluded.
    Script,
    /// Writer-side requirements: they address whoever generated the content, or a construct
    /// this program's writers do not emit (see [`Status::WriterSide`]). No row carries it today:
    /// Annex F's rows, the last that did, are read against `pdf_syntax::linearize`, which emits
    /// the annex since `CLAUDE.md`'s scope list admits it (ADR 1293).
    WriterSide,
}

impl Exclusion {
    /// The word the ledger writes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Multimedia => "clause-13-multimedia",
            Self::Xfa => "xfa",
            Self::Script => "script-behaviour",
            Self::WriterSide => "writer-side",
        }
    }

    /// Every exclusion principle 5 allows.
    #[must_use]
    pub fn all() -> [Self; 4] {
        [Self::Multimedia, Self::Xfa, Self::Script, Self::WriterSide]
    }
}

impl fmt::Display for Exclusion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Exclusion {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::all()
            .into_iter()
            .find(|exclusion| exclusion.as_str() == text)
            .ok_or_else(|| {
                let known: Vec<&str> = Self::all()
                    .iter()
                    .map(|exclusion| exclusion.as_str())
                    .collect();
                format!(
                    "`{text}` is not one of principle 5's exclusions; expected one of {}",
                    known.join(", ")
                )
            })
    }
}

/// Every number the ledger is responsible for: the normative clauses' subclauses and the
/// normative annexes.
fn covered(index: &ClauseIndex) -> impl Iterator<Item = ClauseNumber> + use<'_> {
    NORMATIVE_CLAUSES
        .into_iter()
        .flat_map(|clause| index.subclauses_of(clause))
        .chain(
            NORMATIVE_ANNEXES
                .into_iter()
                .flat_map(|annex| index.numbers_of_annex(annex)),
        )
}

/// One subclause's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The subclause this row is about.
    pub clause: ClauseNumber,
    /// The standard's title for it, carried so the file reads without the standard beside
    /// it — and checked against the standard, so it cannot drift.
    pub title: String,
    /// What is known about it.
    pub status: Status,
    /// Where the requirement is implemented, as workspace-relative paths.
    pub code: Vec<String>,
    /// What holds it, as `path` or `path::test_name`.
    pub test: Vec<String>,
    /// Which of principle 5's exclusions covers it, for `out-of-scope` rows.
    pub exclusion: Option<Exclusion>,
    /// Why the status is what it is. Required wherever the status alone would not say.
    pub note: Option<String>,
    /// The 1-based line of the ledger the row starts on, for error messages.
    pub line: usize,
}

impl Row {
    /// A row for a clause nobody has read yet.
    #[must_use]
    pub fn unreviewed(clause: ClauseNumber, title: String) -> Self {
        Self {
            clause,
            title,
            status: Status::Unreviewed,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: None,
            line: 0,
        }
    }
}

/// Every row, in ascending clause order.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    /// The rows.
    pub rows: Vec<Row>,
}

/// Why the ledger could not be read at all.
#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    /// The file could not be read.
    #[error("cannot read the ledger at {path}: {source}")]
    Unreadable {
        /// The path that was tried.
        path: String,
        /// The underlying failure.
        source: std::io::Error,
    },
    /// The file is not in the subset [`toml_subset`] accepts.
    ///
    /// The row is named as well as the line. A note is one line and the ledger is 883 of them,
    /// so "line 1454" sends a reader counting while "§8.5.3.3.1's row" sends them to the row
    /// they were editing — and the commonest way in is a bare `"` inside a note, which the
    /// reader refuses at the character after the string it accidentally ended. ADR 1166.
    #[error("{path}: {source}{}", in_row(clause.as_deref()))]
    Malformed {
        /// The path that was read.
        path: String,
        /// The clause of the row the failing line belongs to, where the file names one above it.
        clause: Option<String>,
        /// Where the reader stopped.
        source: toml_subset::TomlError,
    },
    /// A table is not a `[[clause]]`, or a row's keys are wrong.
    #[error("{path} line {line}: {problem}")]
    BadRow {
        /// The path that was read.
        path: String,
        /// The line the row starts on.
        line: usize,
        /// What is wrong with it.
        problem: String,
    },
}

impl Ledger {
    /// Reads the ledger.
    ///
    /// # Errors
    ///
    /// If the file cannot be read, is outside the accepted subset, or holds a row whose
    /// keys or values this type does not define.
    pub fn read(path: &Path) -> Result<Self, LedgerError> {
        let text = std::fs::read_to_string(path).map_err(|source| LedgerError::Unreadable {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text).map_err(|error| match error {
            ParseError::Toml { source, clause } => LedgerError::Malformed {
                path: path.display().to_string(),
                clause,
                source,
            },
            ParseError::Row { line, problem } => LedgerError::BadRow {
                path: path.display().to_string(),
                line,
                problem,
            },
        })
    }

    /// Reads the ledger from text.
    ///
    /// # Errors
    ///
    /// As [`Ledger::read`], without the file.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let tables = toml_subset::parse(text).map_err(|source| ParseError::Toml {
            clause: clause_above(text, source.line),
            source,
        })?;
        let mut rows = Vec::new();
        for table in tables {
            let line = table.line;
            let fail = |problem: String| ParseError::Row { line, problem };
            if table.name != "clause" {
                return Err(fail(format!(
                    "`[[{}]]`: the ledger holds `[[clause]]` tables only",
                    table.name
                )));
            }
            let text_key = |key: &str| -> Result<Option<&str>, ParseError> {
                match table.get(key) {
                    None => Ok(None),
                    Some(value) => value
                        .as_text()
                        .map(Some)
                        .ok_or_else(|| fail(format!("`{key}` is a string, not a list"))),
                }
            };
            let list_key = |key: &str| -> Vec<String> {
                table.get(key).map_or_else(Vec::new, |value| {
                    value.as_list().into_iter().map(str::to_owned).collect()
                })
            };

            for (key, _) in &table.entries {
                if !matches!(
                    key.as_str(),
                    "clause" | "title" | "status" | "code" | "test" | "exclusion" | "note"
                ) {
                    return Err(fail(format!("`{key}` is not a ledger key")));
                }
            }

            let clause = text_key("clause")?.ok_or_else(|| fail("no `clause`".to_owned()))?;
            let clause = clause
                .parse::<ClauseNumber>()
                .map_err(|error| fail(error.to_string()))?;
            let title = text_key("title")?
                .ok_or_else(|| fail("no `title`".to_owned()))?
                .to_owned();
            let status = text_key("status")?
                .ok_or_else(|| fail("no `status`".to_owned()))?
                .parse::<Status>()
                .map_err(fail)?;
            let exclusion = text_key("exclusion")?
                .map(|text| text.parse::<Exclusion>().map_err(fail))
                .transpose()?;

            rows.push(Row {
                clause,
                title,
                status,
                code: list_key("code"),
                test: list_key("test"),
                exclusion,
                note: text_key("note")?.map(str::to_owned),
                line,
            });
        }
        Ok(Self { rows })
    }

    /// Writes the ledger in the form [`Ledger::parse`] reads.
    #[must_use]
    pub fn to_toml(&self, preamble: &str) -> String {
        let mut out = String::new();
        for line in preamble.lines() {
            if line.is_empty() {
                out.push('\n');
            } else {
                out.push_str("# ");
                out.push_str(line);
                out.push('\n');
            }
        }
        for row in &self.rows {
            out.push_str("\n[[clause]]\n");
            write_key(&mut out, "clause", &Value::Text(row.clause.to_string()));
            write_key(&mut out, "title", &Value::Text(row.title.clone()));
            write_key(
                &mut out,
                "status",
                &Value::Text(row.status.as_str().to_owned()),
            );
            if !row.code.is_empty() {
                write_key(&mut out, "code", &Value::List(row.code.clone()));
            }
            if !row.test.is_empty() {
                write_key(&mut out, "test", &Value::List(row.test.clone()));
            }
            if let Some(exclusion) = row.exclusion {
                write_key(
                    &mut out,
                    "exclusion",
                    &Value::Text(exclusion.as_str().to_owned()),
                );
            }
            if let Some(note) = &row.note {
                write_key(&mut out, "note", &Value::Text(note.clone()));
            }
        }
        out
    }

    /// The row for a clause, if the ledger has one.
    #[must_use]
    pub fn row(&self, clause: &ClauseNumber) -> Option<&Row> {
        self.rows.iter().find(|row| &row.clause == clause)
    }

    /// How many rows carry each status.
    #[must_use]
    pub fn counts(&self) -> Vec<(Status, usize)> {
        Status::all()
            .into_iter()
            .map(|status| {
                (
                    status,
                    self.rows.iter().filter(|row| row.status == status).count(),
                )
            })
            .collect()
    }

    /// The rows whose clause number this one is a strict ancestor of.
    ///
    /// The standard's numbering *is* the containment, so this needs no table: §12.8 holds
    /// §12.8.3.3.1 because the first is a prefix of the second, which [`ClauseNumber`] answers.
    #[must_use]
    pub fn descendants(&self, clause: &ClauseNumber) -> Vec<&Row> {
        self.rows
            .iter()
            .filter(|row| clause.is_ancestor_of(&row.clause))
            .collect()
    }

    /// Whether this row is an **aggregate**: a heading whose debt is its subclauses'.
    ///
    /// The rule is mechanical and that is the point of it (ADR 1035). A row is an aggregate
    /// when it has at least one descendant row and at least one of those descendants still
    /// [`Status::owes`] something. Such a row cannot be assigned to a round — it flips when its
    /// last unsettled child flips — so counting it as a piece of work inflates the debt by the
    /// depth of the numbering rather than by anything the standard requires. 58 of the
    /// ledger's 201 `partial` rows are aggregates by this test, which is why "206 `partial`
    /// rows" was never 206 pieces of work (`doc/reviews/1012-where-the-effort-goes.md` §2.1).
    ///
    /// A settled row is never an aggregate: it has already answered for itself.
    #[must_use]
    pub fn is_aggregate(&self, row: &Row) -> bool {
        row.status.owes()
            && self
                .descendants(&row.clause)
                .iter()
                .any(|below| below.status.owes())
    }

    /// The rows that owe a debt of their own — every unsettled row that is not an aggregate.
    ///
    /// This is the number a round allocating work should read, and it is smaller than the
    /// status counts by every heading in the tree.
    #[must_use]
    pub fn owing(&self) -> Vec<&Row> {
        self.rows
            .iter()
            .filter(|row| row.status.owes() && !self.is_aggregate(row))
            .collect()
    }
}

fn write_key(out: &mut String, key: &str, value: &Value) {
    out.push_str(key);
    out.push_str(" = ");
    match value {
        Value::Text(text) => {
            // `write_string` only fails if a `String` cannot be written to, which cannot
            // happen: `String`'s `fmt::Write` is infallible.
            let _ = toml_subset::write_string(out, text);
        }
        Value::List(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                let _ = toml_subset::write_string(out, item);
            }
            out.push(']');
        }
    }
    out.push('\n');
}

/// The clause of the row `line` belongs to, read out of the text without parsing it.
///
/// The reader has stopped by the time this is asked, so it cannot use the parse: it walks the
/// lines above, forgets the clause at each `[[` header and remembers the last `clause = "…"`.
/// A file whose very first row is malformed above its own `clause` key yields `None`, and the
/// line number is then all there is to say. ADR 1166.
fn clause_above(text: &str, line: usize) -> Option<String> {
    let mut current = None;
    for source in text.lines().take(line) {
        let source = source.trim();
        if source.starts_with("[[") {
            current = None;
        } else if let Some(rest) = source.strip_prefix("clause = \"")
            && let Some((clause, _)) = rest.split_once('"')
        {
            current = Some(clause.to_owned());
        }
    }
    current
}

/// The row a malformed line belongs to, as a phrase to put after a reader's message.
fn in_row(clause: Option<&str>) -> String {
    clause.map_or_else(String::new, |clause| {
        format!(" \u{2014} in \u{a7}{clause}'s row")
    })
}

/// Why a ledger's text could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The text is outside the accepted subset.
    #[error("{source}{}", in_row(clause.as_deref()))]
    Toml {
        /// Where the reader stopped.
        source: toml_subset::TomlError,
        /// The clause of the row that line belongs to, where the file names one above it.
        clause: Option<String>,
    },
    /// A row's keys or values are not the ledger's.
    #[error("line {line}: {problem}")]
    Row {
        /// The line the row starts on.
        line: usize,
        /// What is wrong with it.
        problem: String,
    },
}

/// Something wrong with the ledger, found by [`check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// A row names a clause the standard does not have.
    UnknownClause {
        /// The clause named.
        clause: ClauseNumber,
        /// The ledger line.
        line: usize,
    },
    /// Two rows name the same clause.
    DuplicateRow {
        /// The clause named twice.
        clause: ClauseNumber,
        /// The second row's line.
        line: usize,
    },
    /// The standard has a subclause the ledger does not.
    MissingRow {
        /// The clause with no row.
        clause: ClauseNumber,
    },
    /// The rows are not in ascending clause order.
    OutOfOrder {
        /// The row that goes backwards.
        clause: ClauseNumber,
        /// The ledger line.
        line: usize,
    },
    /// A row's title is not the standard's.
    WrongTitle {
        /// The clause.
        clause: ClauseNumber,
        /// What the ledger says.
        ledger: String,
        /// What the standard says.
        standard: String,
    },
    /// A status does not carry the evidence it claims.
    MissingEvidence {
        /// The clause.
        clause: ClauseNumber,
        /// What is missing.
        missing: String,
    },
    /// A row names a code site or test that is not there.
    MissingSite {
        /// The clause.
        clause: ClauseNumber,
        /// The path, as the row writes it.
        site: String,
        /// Why it could not be found.
        why: String,
    },
    /// A clause of the standard states a requirement and the ledger's population excludes it.
    ///
    /// The population used to be a constant nothing read the standard against, so a clause
    /// outside it was invisible to every instrument here: no row, and therefore no
    /// [`Problem::MissingRow`] and no [`Problem::CitedButUnreviewed`] either, both of which
    /// walk the covered numbers. That is how clause 6's eleven `shall`s went unrecorded while
    /// §6.3.2.2 was cited forty-seven times (ADR 0984).
    UnrecordedRequirement {
        /// The clause number or annex letter, as the standard writes it.
        clause: String,
        /// How many times `shall` appears under it.
        shalls: usize,
    },
    /// An entry of [`EXCLUDED_CLAUSES`] whose clause states no requirement to excuse.
    ///
    /// Trap 25: a hand-written population can name a thing that never existed, and finding
    /// nothing there reads as a pass.
    StaleExclusion {
        /// The clause the list names.
        clause: u16,
        /// Why the list says it has no row.
        reason: String,
    },
    /// The code cites a clause whose row says nobody has read it.
    CitedButUnreviewed {
        /// The clause cited.
        clause: ClauseNumber,
        /// Where it is cited, as `path:line`.
        first_site: String,
        /// How many citations there are.
        citations: usize,
    },
    /// A heading still owes something that not one of its subclauses owes.
    ///
    /// The counterpart of [`Ledger::is_aggregate`], and the only way that rule can be wrong in
    /// the direction that matters. A heading carrying its subclauses' debt is bookkeeping; a
    /// heading carrying debt *after* every subclause has settled is either a debt of its own
    /// that the note has never named, or a status somebody forgot to move when the last child
    /// moved. Both are findings, and neither is visible to a sweep that reads statuses one row
    /// at a time. ADR 1035.
    AggregateWithoutDebt {
        /// The heading.
        clause: ClauseNumber,
        /// The status it still wears.
        status: Status,
        /// How many subclause rows it has, every one of them settled.
        settled_below: usize,
    },
    /// A `departed` row names an ADR that has no file in `doc/adr/`.
    ///
    /// "Decided against with its cost recorded" is a claim about a document somebody can open,
    /// and [`check_evidence`] asks only that the row name one. A number naming no file is the
    /// same failure one step later: the argument cannot be re-read, so the row settles on prose
    /// alone. ADR 1166.
    ArgumentMissing {
        /// The clause.
        clause: ClauseNumber,
        /// The number the note names.
        adr: u16,
    },
    /// A note names, in code type, a crate, a program or a corpus this tree does not have.
    ///
    /// `code` and `test` paths are checked by [`Problem::MissingSite`]; a note's prose was not,
    /// and ten rows went on naming `render-quorra` after the crate became `render-raster` — a
    /// claim about the tree that was false and that nothing could see. ADR 1437.
    UnknownProgram {
        /// The clause.
        clause: ClauseNumber,
        /// The name, as the note writes it.
        name: String,
    },
    /// The workspace's crates, programs or corpora could not be listed, so no note's names could
    /// be checked. Reported for trap 13's reason, as [`Problem::ArgumentsUnreadable`] is.
    ProgramsUnreadable {
        /// What went wrong.
        why: String,
    },
    /// An `implemented` row none of whose tests is a [`TestKind::Fixture`].
    ///
    /// Every test it names is a corpus witness, an ignored walk, a census or a whole file: the
    /// robustness instrument, which passes when nothing crashed and the floors held, and which
    /// on a machine without the corpus checkout passes having read nothing. A finding rather
    /// than a failure — the gate admits the rows it names up to a ratchet that may only fall
    /// (ADR 1497).
    OnlyWalks {
        /// The clause.
        clause: ClauseNumber,
        /// Each test the row names, with its kind.
        tests: Vec<(String, TestKind)>,
    },
    /// `doc/adr/` could not be listed, so no `departed` row's argument could be checked.
    ///
    /// Reported rather than passed over: a sweep that could not open its population and said
    /// nothing has printed a tick about itself (trap 13). ADR 1166.
    ArgumentsUnreadable {
        /// What went wrong.
        why: String,
    },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownClause { clause, line } => {
                write!(f, "line {line}: §{clause} is not a clause of ISO 32000-2")
            }
            Self::DuplicateRow { clause, line } => {
                write!(f, "line {line}: §{clause} already has a row")
            }
            Self::MissingRow { clause } => write!(
                f,
                "§{clause} has no row; regenerate the ledger with `cargo run -p conformance --bin ledger -- --write`"
            ),
            Self::OutOfOrder { clause, line } => write!(
                f,
                "line {line}: §{clause} is out of order; rows ascend by clause number"
            ),
            Self::WrongTitle {
                clause,
                ledger,
                standard,
            } => write!(
                f,
                "§{clause}: the ledger titles it {ledger:?}, the standard titles it {standard:?}"
            ),
            Self::MissingEvidence { clause, missing } => {
                write!(f, "§{clause}: {missing}")
            }
            Self::MissingSite { clause, site, why } => {
                write!(f, "§{clause}: {site} — {why}")
            }
            Self::UnrecordedRequirement { clause, shalls } => write!(
                f,
                "§{clause} states `shall` {shalls} time(s) and the ledger covers none of it; \
                 add it to `NORMATIVE_CLAUSES`/`NORMATIVE_ANNEXES` or say why in \
                 `EXCLUDED_CLAUSES`/`INFORMATIVE_ANNEXES`"
            ),
            Self::StaleExclusion { clause, reason } => write!(
                f,
                "`EXCLUDED_CLAUSES` excuses §{clause} — {reason} — but §{clause} states no \
                 `shall`, so the excuse is about a clause that no longer needs one"
            ),
            Self::CitedButUnreviewed {
                clause,
                first_site,
                citations,
            } => write!(
                f,
                "§{clause} is cited {citations} time(s), first at {first_site}, and its row is \
                 still `unreviewed`. Code that cites a clause has read it; record what it found."
            ),
            Self::AggregateWithoutDebt {
                clause,
                status,
                settled_below,
            } => write!(
                f,
                "§{clause} is `{status}` and all {settled_below} of its subclause rows are \
                 settled, so it is carrying a debt none of them carries. Either its note says \
                 what it owes of its own, or the status moves with its last child's."
            ),
            Self::ArgumentMissing { clause, adr } => write!(
                f,
                "§{clause} is `departed` and its note names ADR {adr:04}, which has no file in \
                 {directory}/. A departure's cost is recorded in a document somebody can open, \
                 so the number has to name one.",
                directory = crate::departures::ADR_DIRECTORY
            ),
            Self::OnlyWalks { clause, tests } => {
                write!(
                    f,
                    "§{clause} is `implemented` and no test it names is a fixture: "
                )?;
                for (index, (site, kind)) in tests.iter().enumerate() {
                    let separator = if index == 0 { "" } else { ", " };
                    write!(f, "{separator}{site} ({kind})")?;
                }
                write!(
                    f,
                    ". Write the fixture whose expected value the clause derives (ADR 1497)."
                )
            }
            Self::ArgumentsUnreadable { why } => {
                write!(f, "no `departed` row's argument could be checked: {why}")
            }
            Self::UnknownProgram { clause, name } => write!(
                f,
                "§{clause}'s note names `{name}`, which is no crate, program or corpus of this \
                 tree. A name in a note is a claim about the tree; write the one it has now."
            ),
            Self::ProgramsUnreadable { why } => {
                write!(
                    f,
                    "no note's crate or program names could be checked: {why}"
                )
            }
        }
    }
}

/// Checks the ledger against the standard, the tree's citations and the tree itself.
///
/// What this can establish is that every claim is *well formed*: the clause exists, the row
/// is where it belongs, the evidence it names is on disk, an exclusion is one principle 5
/// allows, and no clause the code cites is still unread. Whether the code implements the
/// clause is not checkable here and is not attempted — that is what a person reading the
/// clause is for.
#[must_use]
pub fn check(
    ledger: &Ledger,
    index: &ClauseIndex,
    citations: &[(PathBuf, Citation)],
    root: &Path,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut previous: Option<&ClauseNumber> = None;

    for row in &ledger.rows {
        if !index.contains(&row.clause) {
            problems.push(Problem::UnknownClause {
                clause: row.clause.clone(),
                line: row.line,
            });
            continue;
        }
        // A repeat is reported as a duplicate and not also as an ordering failure: two
        // findings for one row would make a mechanical mistake read as two.
        if previous.is_some_and(|previous| previous == &row.clause) {
            problems.push(Problem::DuplicateRow {
                clause: row.clause.clone(),
                line: row.line,
            });
        } else if previous.is_some_and(|previous| previous > &row.clause) {
            problems.push(Problem::OutOfOrder {
                clause: row.clause.clone(),
                line: row.line,
            });
        }
        previous = Some(&row.clause);

        if let Some(standard) = index.title(&row.clause)
            && standard != row.title
        {
            problems.push(Problem::WrongTitle {
                clause: row.clause.clone(),
                ledger: row.title.clone(),
                standard: standard.to_owned(),
            });
        }

        problems.extend(check_evidence(row));
        for site in row.code.iter().chain(row.test.iter()) {
            if let Some(why) = missing_site(root, site, row.test.contains(site)) {
                problems.push(Problem::MissingSite {
                    clause: row.clause.clone(),
                    site: site.clone(),
                    why,
                });
            }
        }
    }

    for clause in covered(index) {
        if ledger.row(&clause).is_none() {
            problems.push(Problem::MissingRow { clause });
        }
    }

    // A clause the code cites is a clause somebody has read closely enough to name. Leaving
    // its row `unreviewed` is how 146 citations came to exist beside no record at all.
    for clause in covered(index) {
        let sites: Vec<&(PathBuf, Citation)> = citations
            .iter()
            .filter(|(_, citation)| citation.number == clause)
            .collect();
        let Some((path, citation)) = sites.first() else {
            continue;
        };
        if ledger
            .row(&clause)
            .is_some_and(|row| row.status == Status::Unreviewed)
        {
            problems.push(Problem::CitedButUnreviewed {
                clause,
                first_site: format!("{}:{}", path.display(), citation.line),
                citations: sites.len(),
            });
        }
    }

    // A heading's debt is its subclauses' until the last of them settles. What this catches is
    // the moment after that: a row still owing something no row under it owes (ADR 1035).
    for row in &ledger.rows {
        let below = ledger.descendants(&row.clause);
        if row.status.owes() && !below.is_empty() && below.iter().all(|row| !row.status.owes()) {
            problems.push(Problem::AggregateWithoutDebt {
                clause: row.clause.clone(),
                status: row.status,
                settled_below: below.len(),
            });
        }
    }

    // A row held only by the robustness instrument is a finding about its evidence, not its
    // status: under the owner's A100 a requirement executed under a control is executed, so the
    // status stands and the fixture is what is owed (ADR 1497).
    let mut classifier = TestClassifier::new(root);
    for row in &ledger.rows {
        if row.status != Status::Implemented || row.test.is_empty() {
            continue;
        }
        let tests: Vec<(String, TestKind)> = row
            .test
            .iter()
            .map(|site| (site.clone(), classifier.classify(site)))
            .collect();
        let only_walks = tests.iter().all(|(_, kind)| *kind != TestKind::Fixture)
            && tests.iter().any(|(_, kind)| *kind != TestKind::Missing);
        if only_walks {
            problems.push(Problem::OnlyWalks {
                clause: row.clause.clone(),
                tests,
            });
        }
    }

    problems.extend(check_arguments(ledger, root));
    problems.extend(check_named_programs(ledger, root));
    problems.extend(check_population(index));

    problems
}

/// Every ADR a `departed` row names is a file in `doc/adr/`.
///
/// The half of ADR 1119's check that it could not take: that check asks a `departed` row to name
/// an argument and cannot judge whether the argument fits, and this one asks that the argument
/// *exists*. It reads the directory listing and no prose — the judgement stays a person's, and
/// [`crate::departures`] prints the reading list that person needs. ADR 1166.
///
/// Nothing is read at all unless some row is `departed`, so a ledger without one pays no listing.
fn check_arguments(ledger: &Ledger, root: &Path) -> Vec<Problem> {
    let departed: Vec<&Row> = ledger
        .rows
        .iter()
        .filter(|row| row.status == Status::Departed)
        .collect();
    if departed.is_empty() {
        return Vec::new();
    }
    let on_disk = match crate::departures::numbers_on_disk(root) {
        Ok(numbers) => numbers,
        Err(why) => {
            return vec![Problem::ArgumentsUnreadable {
                why: why.to_string(),
            }];
        }
    };
    let mut problems = Vec::new();
    for row in departed {
        let note = row.note.as_deref().unwrap_or_default();
        for adr in crate::departures::citations(note) {
            if !on_disk.contains(&adr) {
                problems.push(Problem::ArgumentMissing {
                    clause: row.clause.clone(),
                    adr,
                });
            }
        }
    }
    problems
}

/// The prefixes a crate or program of this workspace is named with; a token wearing one is a
/// claim that such a thing exists.
const PROGRAM_PREFIXES: [&str; 5] = ["pdf", "render", "viewer", "raster", "quorra"];

/// Every crate, program and corpus a note names in code type is one this tree has.
///
/// A backticked token shaped like a package name (`render-raster`) or like the first segment of
/// a Rust path (`raster_scene::GroupSpec`) is looked up among the package and target names of
/// every manifest under `crates/`, `raster/crates/`, `tools/` and `fuzz/`, the programs under
/// each `src/bin/`, and the corpora under `doc/corpora/`. Nothing is listed unless some note
/// names such a token, so a ledger without one pays no listing. ADR 1437.
fn check_named_programs(ledger: &Ledger, root: &Path) -> Vec<Problem> {
    let named: Vec<(&Row, String)> = ledger
        .rows
        .iter()
        .flat_map(|row| {
            named_programs(row.note.as_deref().unwrap_or_default())
                .into_iter()
                .map(move |name| (row, name))
        })
        .collect();
    if named.is_empty() {
        return Vec::new();
    }
    let known = match programs_on_disk(root) {
        Ok(known) => known,
        Err(why) => {
            return vec![Problem::ProgramsUnreadable {
                why: why.to_string(),
            }];
        }
    };
    named
        .into_iter()
        .filter(|(_, name)| !known.contains(name.as_str()))
        .map(|(row, name)| Problem::UnknownProgram {
            clause: row.clause.clone(),
            name,
        })
        .collect()
}

/// The package-shaped names a note writes in code type, each as a package would spell it.
///
/// A Rust path's first segment is spelled with underscores and is returned with hyphens, which
/// is how Cargo derives a library's name from its package's; a segment that is itself a
/// target name spelled with underscores is still found, because [`programs_on_disk`] records
/// both spellings.
fn named_programs(note: &str) -> Vec<String> {
    let is_named = |text: &str, separator: char| {
        PROGRAM_PREFIXES.iter().any(|prefix| {
            text.strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix(separator))
                .is_some_and(|rest| {
                    !rest.is_empty()
                        && !rest.ends_with(separator)
                        && rest
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == separator)
                })
        })
    };
    let mut names = Vec::new();
    for span in note.split('`').skip(1).step_by(2) {
        for token in span.split(|c: char| c.is_whitespace() || "(),;'".contains(c)) {
            if is_named(token, '-') {
                names.push(token.to_owned());
            } else if let Some((first, _)) = token.split_once("::")
                && is_named(first, '_')
            {
                names.push(first.replace('_', "-"));
            }
        }
    }
    names
}

/// The names a note may give a crate, a program or a corpus: every manifest `name`, every
/// `src/bin/` entry and every `doc/corpora/` directory, each also with its underscores
/// read as hyphens.
fn programs_on_disk(root: &Path) -> std::io::Result<std::collections::BTreeSet<String>> {
    let mut known = std::collections::BTreeSet::new();
    let mut manifests = vec![root.join("fuzz/Cargo.toml")];
    for parent in ["crates", "raster/crates", "tools"] {
        for entry in std::fs::read_dir(root.join(parent))? {
            let directory = entry?.path();
            manifests.push(directory.join("Cargo.toml"));
            let bins = directory.join("src/bin");
            if bins.is_dir() {
                for bin in std::fs::read_dir(bins)? {
                    let bin = bin?.path();
                    if let Some(stem) = bin.file_stem().and_then(|stem| stem.to_str()) {
                        known.insert(stem.to_owned());
                    }
                }
            }
        }
    }
    for manifest in manifests.into_iter().filter(|manifest| manifest.is_file()) {
        for line in std::fs::read_to_string(manifest)?.lines() {
            if let Some(name) = line
                .strip_prefix("name = \"")
                .and_then(|rest| rest.strip_suffix('"'))
            {
                known.insert(name.to_owned());
            }
        }
    }
    for corpus in std::fs::read_dir(root.join("doc/corpora"))? {
        if let Some(name) = corpus?.file_name().to_str() {
            known.insert(name.to_owned());
        }
    }
    let hyphenated: Vec<String> = known.iter().map(|name| name.replace('_', "-")).collect();
    known.extend(hyphenated);
    Ok(known)
}

/// The ledger's population, read against the standard rather than against itself.
///
/// Every top-level clause and every annex is sorted into one of four lists — covered
/// ([`NORMATIVE_CLAUSES`], [`NORMATIVE_ANNEXES`]) or excused by argument
/// ([`EXCLUDED_CLAUSES`], [`INFORMATIVE_ANNEXES`]) — and a clause that states `shall` and is
/// in none of them is the finding. An annex in none of them is a finding whether it states a
/// `shall` or not: an annex letter nobody has classified is a letter no instrument here can
/// see, which is the state all seventeen of them were in before ADR 0206.
fn check_population(index: &ClauseIndex) -> Vec<Problem> {
    let mut problems = Vec::new();
    let groups = requirements_by_group(index);
    for (group, shalls) in groups.clone() {
        let covered = match group.parse::<u16>() {
            Ok(top) => NORMATIVE_CLAUSES.contains(&top),
            Err(_) => group
                .chars()
                .next()
                .is_some_and(|letter| NORMATIVE_ANNEXES.contains(&letter)),
        };
        if covered {
            continue;
        }
        let excused = match group.parse::<u16>() {
            Ok(top) => {
                shalls == 0
                    || EXCLUDED_CLAUSES
                        .iter()
                        .any(|(excluded, _)| *excluded == top)
            }
            Err(_) => group
                .chars()
                .next()
                .is_some_and(|letter| INFORMATIVE_ANNEXES.contains(&letter)),
        };
        if !excused {
            problems.push(Problem::UnrecordedRequirement {
                clause: group,
                shalls,
            });
        }
    }
    for (clause, reason) in EXCLUDED_CLAUSES {
        let shalls = groups
            .iter()
            .find(|(group, _)| group.parse::<u16>() == Ok(clause))
            .map_or(0, |(_, shalls)| *shalls);
        if shalls == 0 {
            problems.push(Problem::StaleExclusion {
                clause,
                reason: reason.to_owned(),
            });
        }
    }
    problems
}

/// How many times `shall` appears under each top-level clause and each annex, in the order
/// the standard states them.
///
/// A heading's own span includes its subclauses', so counting over those would count a
/// sentence once per level it sits under. What is counted here is the text from each numbered
/// heading to the next one, which partitions the document exactly once.
fn requirements_by_group(index: &ClauseIndex) -> Vec<(String, usize)> {
    let headings = index.headings();
    let mut groups: Vec<(String, usize)> = Vec::new();
    for (position, heading) in headings.iter().enumerate() {
        let end = headings
            .get(position.saturating_add(1))
            .map_or(heading.span.end, |next| next.span.start);
        let text = index.text_in(heading.span.start..end);
        let shalls = text
            .split_whitespace()
            .filter(|word| {
                word.trim_matches(|character: char| !character.is_ascii_alphabetic())
                    .eq_ignore_ascii_case("shall")
            })
            .count();
        let group = match heading.number.annex() {
            Some(letter) => letter.to_string(),
            None => heading.number.clause().unwrap_or_default().to_string(),
        };
        if let Some(entry) = groups.iter_mut().find(|(name, _)| *name == group) {
            entry.1 = entry.1.saturating_add(shalls);
        } else {
            groups.push((group, shalls));
        }
    }
    groups
}

fn check_evidence(row: &Row) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut require = |condition: bool, missing: &str| {
        if !condition {
            problems.push(Problem::MissingEvidence {
                clause: row.clause.clone(),
                missing: missing.to_owned(),
            });
        }
    };
    match row.status {
        Status::Implemented => {
            require(!row.code.is_empty(), "`implemented` names its `code`");
            require(!row.test.is_empty(), "`implemented` names its `test`");
        }
        Status::Partial => {
            require(!row.code.is_empty(), "`partial` names its `code`");
            require(!row.test.is_empty(), "`partial` names its `test`");
            require(
                row.note.is_some(),
                "`partial` needs a `note` saying which requirements are implemented, which \
                 are not, and what is reported for the rest",
            );
        }
        Status::Departed => {
            require(!row.code.is_empty(), "`departed` names its `code`");
            require(!row.test.is_empty(), "`departed` names its `test`");
            require(
                row.note.is_some(),
                "`departed` needs a `note` whose first sentence says what was departed from",
            );
            // "Decided against with its cost recorded" is a claim about a document somebody can
            // open, so the row has to name one. An ADR number is the only form that claim takes
            // here — `CLAUDE.md` principle 1 requires a deliberate departure to carry its cost in
            // writing, and principle 4 puts the reasoning in `doc/adr/`. Without this the new word
            // is exactly the hiding ADR 1035 section 3 refused: a settled status nobody comes back
            // to, resting on prose that names no argument. ADR 1119.
            require(
                row.note.as_deref().is_some_and(names_an_adr),
                "`departed` needs a `note` naming the ADR that decided the departure and \
                 priced it",
            );
        }
        Status::Reported => require(
            row.note.is_some(),
            "`reported` needs a `note` saying what is reported and where",
        ),
        Status::Silent => require(
            row.note.is_some(),
            "`silent` needs a `note` saying what is drawn wrong, and what would report it",
        ),
        Status::Inapplicable => require(
            row.note.is_some(),
            "`inapplicable` needs a `note` saying why the requirement cannot apply to a screen",
        ),
        Status::WriterSide => {}
        Status::OutOfScope => require(
            row.exclusion.is_some(),
            "`out-of-scope` needs an `exclusion` naming which of principle 5's closed list \
             covers it",
        ),
        Status::Unreviewed => {
            require(
                row.code.is_empty() && row.test.is_empty(),
                "`unreviewed` means nobody has read the clause, so it cannot name evidence",
            );
        }
    }
    if row.exclusion.is_some() && row.status != Status::OutOfScope {
        problems.push(Problem::MissingEvidence {
            clause: row.clause.clone(),
            missing: "an `exclusion` belongs only to an `out-of-scope` row".to_owned(),
        });
    }
    problems
}

/// Whether a note names an ADR — `ADR` followed by a number.
///
/// Deliberately the shallowest test that can be wrong in only one direction: it cannot tell
/// whether the ADR it finds is *about* the departure, and does not claim to, for the reason
/// [`Ledger::is_aggregate`] reads no prose (ADR 1035 section 4). What it can do is refuse a
/// `departed` row that names no argument at all, which is the failure the status would otherwise
/// make cheap.
fn names_an_adr(note: &str) -> bool {
    note.match_indices("ADR ").any(|(at, marker)| {
        note[at.saturating_add(marker.len())..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
    })
}

/// Why a named site cannot be found, or `None` if it can.
///
/// A test site may name a function — `crates/pdf-model/tests/corpus.rs::draws_page_one` — and
/// then the function has to be in the file. A row naming a test that was renamed away is a
/// row claiming evidence that no longer exists.
fn missing_site(root: &Path, site: &str, is_test: bool) -> Option<String> {
    let (path, function) = match site.split_once("::") {
        Some((path, function)) => (path, Some(function)),
        None => (site, None),
    };
    let full = root.join(path);
    if !full.is_file() {
        return Some(format!("no such file (looked in {})", full.display()));
    }
    let function = function?;
    if !is_test {
        return Some("only a `test` site may name a function".to_owned());
    }
    let text = match std::fs::read_to_string(&full) {
        Ok(text) => text,
        Err(error) => return Some(format!("cannot be read: {error}")),
    };
    if text.contains(&format!("fn {function}(")) {
        None
    } else {
        Some(format!("holds no `fn {function}`"))
    }
}

/// What kind of evidence one `test =` entry names.
///
/// The checker has always asked that a named test *exists*; this asks what it is. Principle 5
/// wants a test's expected value derived from the clause, and `CLAUDE.md`'s two denominators
/// keep the corpus for the robustness question — so a row whose only test is a walk over a
/// corpus is held by the second instrument while claiming the first's answer. ADR 1497.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TestKind {
    /// A `#[test]` without `#[ignore]` that reads no optional corpus: its input is in the tree,
    /// so it runs on every machine and cannot pass by finding nothing to read.
    Fixture,
    /// A `#[test]` without `#[ignore]` that reads a document from an optional corpus checkout
    /// ([`CORPUS_ROOTS`]), directly or through a helper in its own file. Its expected value may
    /// well be the clause's, but where the checkout is absent it returns having checked nothing.
    CorpusWitness,
    /// A test marked `#[ignore]`: a walk or a census run on request, behind the heavy-walk lock.
    Walk,
    /// A function of a program rather than a test — under `src/bin/` or `examples/`.
    Census,
    /// A whole file rather than a function, which the file-only ratchets already count.
    File,
    /// A function the file does not hold, or a file that is not there — already a
    /// [`Problem::MissingSite`].
    Missing,
}

impl TestKind {
    /// The word printed for this kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fixture => "fixture",
            Self::CorpusWitness => "corpus witness",
            Self::Walk => "ignored walk",
            Self::Census => "census",
            Self::File => "whole file",
            Self::Missing => "missing",
        }
    }
}

impl fmt::Display for TestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The optional corpus checkouts a test may read, as workspace-relative path prefixes.
///
/// Each is a submodule or a downloaded collection a machine may not have, which is what makes
/// a test reading one a [`TestKind::CorpusWitness`]: `doc/corpora-own/` is tracked and is not
/// on this list, so a test reading it is a fixture.
pub const CORPUS_ROOTS: [&str; 3] = ["doc/pdf.js", "doc/corpora/", "doc/veraPDF-corpus"];

/// How a row is held, read from the kinds of its `test =` entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holding {
    /// At least one entry is a [`TestKind::Fixture`].
    Fixture,
    /// Entries exist and none is a fixture: every one is a corpus witness, an ignored walk, a
    /// census or a whole file.
    OnlyWalks,
    /// The row names no test.
    Empty,
}

/// Per status, how many rows are held by a fixture, only by walks, and by nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HoldingCounts {
    /// Rows holding at least one fixture.
    pub fixture: usize,
    /// Rows whose tests are all walks, witnesses, censuses or files.
    pub only_walks: usize,
    /// Rows naming no test.
    pub empty: usize,
}

/// One source file's functions, read once however many rows name it.
#[derive(Debug, Default)]
struct SourceFunctions {
    /// Function name to its kind, for the first definition of each name in the file.
    kinds: std::collections::HashMap<String, TestKind>,
}

/// Classifies `test =` entries, reading each file once.
#[derive(Debug)]
pub struct TestClassifier {
    root: PathBuf,
    files: std::collections::HashMap<String, Option<SourceFunctions>>,
}

impl TestClassifier {
    /// A classifier reading files under the workspace `root`.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            files: std::collections::HashMap::new(),
        }
    }

    /// What one `test =` entry names.
    pub fn classify(&mut self, site: &str) -> TestKind {
        let Some((path, function)) = site.split_once("::") else {
            return if self.root.join(site).is_file() {
                TestKind::File
            } else {
                TestKind::Missing
            };
        };
        let root = &self.root;
        let functions = self.files.entry(path.to_owned()).or_insert_with(|| {
            std::fs::read_to_string(root.join(path))
                .ok()
                .map(|text| read_functions(&text))
        });
        let Some(functions) = functions else {
            return TestKind::Missing;
        };
        let Some(kind) = functions.kinds.get(function).copied() else {
            return TestKind::Missing;
        };
        // A program's function is evidence of a different kind whatever its attributes say.
        if path.contains("/src/bin/") || path.contains("/examples/") {
            return TestKind::Census;
        }
        kind
    }

    /// How `row` is held.
    pub fn holding(&mut self, row: &Row) -> Holding {
        if row.test.is_empty() {
            return Holding::Empty;
        }
        if row
            .test
            .iter()
            .any(|site| self.classify(site) == TestKind::Fixture)
        {
            Holding::Fixture
        } else {
            Holding::OnlyWalks
        }
    }

    /// Per status, how the ledger's rows are held, in [`Status::all`]'s order.
    pub fn holdings(&mut self, ledger: &Ledger) -> Vec<(Status, HoldingCounts)> {
        let mut out: Vec<(Status, HoldingCounts)> = Status::all()
            .into_iter()
            .map(|status| (status, HoldingCounts::default()))
            .collect();
        for row in &ledger.rows {
            let holding = self.holding(row);
            if let Some((_, counts)) = out.iter_mut().find(|(status, _)| *status == row.status) {
                let slot = match holding {
                    Holding::Fixture => &mut counts.fixture,
                    Holding::OnlyWalks => &mut counts.only_walks,
                    Holding::Empty => &mut counts.empty,
                };
                *slot = slot.saturating_add(1);
            }
        }
        out
    }
}

/// One function definition found in a source file.
struct Definition<'a> {
    name: &'a str,
    /// The attribute and comment lines directly above it, joined.
    attributes: String,
    /// From the signature line to the closing brace at the signature's own indentation.
    body: String,
}

/// Every function of a rustfmt-formatted file, classified.
///
/// Deliberately a reading of the file's *layout* rather than a parse: rustfmt puts a function's
/// closing brace at its signature's indentation, and its attributes on the lines directly above
/// it, so both are found without a Rust parser — the checker's only dependency stays
/// `thiserror` (PLAN.md §5a). The cost is that a corpus read through a helper in *another*
/// file, a `support` module, is seen only if this file's own text names a corpus root.
fn read_functions(text: &str) -> SourceFunctions {
    let lines: Vec<&str> = text.lines().collect();
    let mut definitions = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(name) = function_name(line) else {
            continue;
        };
        if line.trim_end().ends_with(';') {
            continue;
        }
        let indent = line.strip_suffix(line.trim_start()).unwrap_or_default();
        let closing = format!("{indent}}}");
        let end = if line.trim_end().ends_with('}') {
            index
        } else {
            lines
                .iter()
                .enumerate()
                .skip(index.saturating_add(1))
                .find(|(_, candidate)| **candidate == closing)
                .map_or(lines.len().saturating_sub(1), |(end, _)| end)
        };
        let body = lines[index..=end].join("\n");
        let mut above = Vec::new();
        for candidate in lines[..index].iter().rev() {
            let trimmed = candidate.trim();
            let attribute_or_comment = trimmed.starts_with("#[") || trimmed.starts_with("//");
            if trimmed.is_empty()
                || (!attribute_or_comment
                    && (trimmed.ends_with('{') || trimmed.ends_with('}') || trimmed.ends_with(';')))
            {
                break;
            }
            above.push(trimmed);
        }
        definitions.push(Definition {
            name,
            attributes: above.join(" "),
            body,
        });
    }

    // The functions that read a corpus, directly or through another function of this file that
    // does: grown to a fixed point, so a helper two calls away is found.
    let mut reads_corpus: std::collections::HashSet<&str> = definitions
        .iter()
        .filter(|definition| {
            CORPUS_ROOTS
                .iter()
                .any(|root| definition.body.contains(root))
        })
        .map(|definition| definition.name)
        .collect();
    loop {
        let grown: Vec<&str> = definitions
            .iter()
            .filter(|definition| !reads_corpus.contains(definition.name))
            .filter(|definition| {
                reads_corpus
                    .iter()
                    .any(|helper| calls(&definition.body, helper))
            })
            .map(|definition| definition.name)
            .collect();
        if grown.is_empty() {
            break;
        }
        reads_corpus.extend(grown);
    }

    let mut kinds = std::collections::HashMap::new();
    for definition in &definitions {
        let is_test =
            definition.attributes.contains("#[test]") || definition.attributes.contains("#[test ");
        let kind = if definition.attributes.contains("#[ignore") {
            TestKind::Walk
        } else if !is_test {
            TestKind::Census
        } else if reads_corpus.contains(definition.name) {
            TestKind::CorpusWitness
        } else {
            TestKind::Fixture
        };
        kinds.entry(definition.name.to_owned()).or_insert(kind);
    }
    SourceFunctions { kinds }
}

/// The name a line defines with `fn`, if it is a function's signature line.
fn function_name(line: &str) -> Option<&str> {
    let mut rest = line.trim_start();
    for prefix in [
        "pub(crate) ",
        "pub(super) ",
        "pub ",
        "const ",
        "async ",
        "unsafe ",
    ] {
        rest = rest.strip_prefix(prefix).unwrap_or(rest);
    }
    let rest = rest.strip_prefix("fn ")?;
    let end = rest
        .find(|character: char| !(character.is_alphanumeric() || character == '_'))
        .unwrap_or(rest.len());
    let name = &rest[..end];
    (!name.is_empty() && rest[end..].starts_with(['(', '<'])).then_some(name)
}

/// Whether `body` calls `name` — the name followed by `(` and not preceded by part of a longer
/// identifier, so `corpus` is not found inside `pdfjs_corpus(`.
fn calls(body: &str, name: &str) -> bool {
    let pattern = format!("{name}(");
    body.match_indices(&pattern).any(|(at, _)| {
        body[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !(before.is_alphanumeric() || before == '_'))
            && !body[..at].ends_with("fn ")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> ClauseIndex {
        ClauseIndex::parse(
            "## 8 Graphics\nx\n\n## 8.1 General\ny\n\n## 8.2 Graphics objects\nz\n".to_owned(),
        )
    }

    fn number(text: &str) -> ClauseNumber {
        text.parse().unwrap()
    }

    fn ledger(rows: &str) -> Ledger {
        Ledger::parse(rows).unwrap()
    }

    #[test]
    fn a_row_round_trips_through_the_file() {
        let mut original = Ledger::default();
        original.rows.push(Row {
            clause: number("8.1"),
            title: "General".to_owned(),
            status: Status::Partial,
            code: vec!["a.rs".to_owned(), "b.rs".to_owned()],
            test: vec!["t.rs::name".to_owned()],
            exclusion: None,
            note: Some("half of it".to_owned()),
            line: 0,
        });
        let written = original.to_toml("a preamble\n\nover two paragraphs");
        let read = Ledger::parse(&written).unwrap();
        assert_eq!(
            read.rows.first().unwrap().note.as_deref(),
            Some("half of it")
        );
        assert_eq!(read.rows.first().unwrap().status, Status::Partial);
        assert_eq!(read.rows.first().unwrap().code.len(), 2);
    }

    /// A malformed line is named by the row it is in as well as by its number.
    ///
    /// The bare `"` a person leaves in a note is refused by `toml_subset` wherever it sits — that
    /// is established there — and what this adds is *which* of 883 rows it was. The second case
    /// is the honest limit: a file whose first row breaks above its own `clause` key has no row
    /// to name, and the line number is then all there is. ADR 1166.
    #[test]
    fn a_line_outside_the_subset_is_named_by_its_row() {
        let error = Ledger::parse(
            "[[clause]]\nclause = \"8.1\"\nnote = \"fine\"\n\n\
             [[clause]]\nclause = \"8.5.3.3.1\"\nnote = \"a stray \" quote\"\n",
        )
        .unwrap_err();
        let said = error.to_string();
        assert!(said.contains("line 7"), "{said}");
        assert!(said.contains("§8.5.3.3.1's row"), "{said}");

        let headless = Ledger::parse("[[clause]]\ntitle = \"a stray \" quote\"\n").unwrap_err();
        let said = headless.to_string();
        assert!(said.contains("line 2"), "{said}");
        assert!(!said.contains("row"), "{said}");
    }

    #[test]
    fn a_clause_the_standard_does_not_have_is_a_finding() {
        let problems = check(
            &ledger("[[clause]]\nclause = \"8.9.6.5\"\ntitle = \"x\"\nstatus = \"unreviewed\"\n"),
            &index(),
            &[],
            Path::new("."),
        );
        assert!(matches!(
            problems.first(),
            Some(Problem::UnknownClause { .. })
        ));
    }

    /// The status that would rot first. A row may claim it only by naming an exclusion from
    /// principle 5's closed list, and `unknown` is not on that list.
    #[test]
    fn out_of_scope_needs_one_of_principle_5s_exclusions() {
        let missing = check(
            &ledger(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"out-of-scope\"\n",
            ),
            &index(),
            &[],
            Path::new("."),
        );
        assert!(matches!(
            missing.first(),
            Some(Problem::MissingEvidence { .. })
        ));
        let invented = Ledger::parse(
            "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"out-of-scope\"\n\
             exclusion = \"too hard\"\n",
        );
        assert!(invented.is_err());
    }

    #[test]
    fn implemented_must_name_evidence_that_exists() {
        let problems = check(
            &ledger(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"implemented\"\n\
                 code = [\"src/nowhere.rs\"]\ntest = [\"tests/nowhere.rs\"]\n",
            ),
            &index(),
            &[],
            Path::new("."),
        );
        let sites = problems
            .iter()
            .filter(|problem| matches!(problem, Problem::MissingSite { .. }))
            .count();
        assert_eq!(sites, 2, "{problems:?}");
    }

    #[test]
    fn a_missing_row_is_a_finding_and_so_is_a_row_out_of_order() {
        let problems = check(
            &ledger(
                "[[clause]]\nclause = \"8.2\"\ntitle = \"Graphics objects\"\nstatus = \"unreviewed\"\n\
                 \n[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"unreviewed\"\n",
            ),
            &index(),
            &[],
            Path::new("."),
        );
        assert!(
            problems
                .iter()
                .any(|problem| matches!(problem, Problem::OutOfOrder { .. }))
        );
        assert!(
            !problems
                .iter()
                .any(|problem| matches!(problem, Problem::MissingRow { .. }))
        );
    }

    #[test]
    fn a_cited_clause_may_not_stay_unreviewed() {
        let citations = vec![(
            PathBuf::from("crates/pdf-model/src/content.rs"),
            Citation {
                number: number("8.1"),
                line: 42,
            },
        )];
        let problems = check(
            &ledger(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"unreviewed\"\n\
                 \n[[clause]]\nclause = \"8.2\"\ntitle = \"Graphics objects\"\nstatus = \"unreviewed\"\n",
            ),
            &index(),
            &citations,
            Path::new("."),
        );
        assert!(matches!(
            problems.first(),
            Some(Problem::CitedButUnreviewed { .. })
        ));
    }

    /// Trap 13, for the status the owner added in answer to `doc/questions/Q63`: a `departed`
    /// row whose note names no ADR is planted, and the check has to name it. Without this the
    /// requirement would be a sentence in a doc comment and the word would settle a row on prose
    /// alone, which is the hiding ADR 1035 section 3 refused. ADR 1119.
    #[test]
    fn a_departed_row_naming_no_adr_is_a_finding() {
        let problems = check(
            &ledger(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"departed\"\n\
                 code = [\"a.rs\"]\ntest = [\"t.rs\"]\nnote = \"one sentence declined.\"\n",
            ),
            &index(),
            &[],
            &crate::workspace_root(),
        );
        assert!(
            problems.iter().any(|problem| matches!(
                problem,
                Problem::MissingEvidence { clause, missing }
                    if clause == &number("8.1") && missing.contains("ADR")
            )),
            "{problems:?}"
        );
    }

    /// The other half of the plant: the same row, naming an ADR, is not a finding — otherwise the
    /// check above would pass for every `departed` row and mean nothing.
    #[test]
    fn a_departed_row_naming_an_adr_is_not_a_finding() {
        let problems = check(
            &ledger(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"departed\"\n\
                 code = [\"a.rs\"]\ntest = [\"t.rs\"]\n\
                 note = \"one sentence declined, priced in ADR 0036.\"\n",
            ),
            &index(),
            &[],
            &crate::workspace_root(),
        );
        assert!(
            !problems
                .iter()
                .any(|problem| matches!(problem, Problem::MissingEvidence { .. })),
            "{problems:?}"
        );
    }

    /// The third half of that plant, and the one ADR 1119's check could not take: a row naming an
    /// ADR *number* that names no file settles on prose exactly as a row naming none does. The
    /// fixture names one real argument and one invented, so the check has to tell them apart
    /// rather than reporting whichever it meets first. ADR 1166.
    #[test]
    fn a_departed_row_naming_an_adr_with_no_file_is_a_finding() {
        let problems = check(
            &ledger(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"departed\"\n\
                 code = [\"a.rs\"]\ntest = [\"t.rs\"]\n\
                 note = \"declined, priced in ADR 0036, and again in ADR 9999.\"\n",
            ),
            &index(),
            &[],
            &crate::workspace_root(),
        );
        let named: Vec<u16> = problems
            .iter()
            .filter_map(|problem| match problem {
                Problem::ArgumentMissing { adr, .. } => Some(*adr),
                _ => None,
            })
            .collect();
        assert_eq!(named, vec![9999], "{problems:?}");
    }

    /// Trap 13's other direction: an instrument that cannot open its population says so instead
    /// of returning nothing, which would read as a clean tree. ADR 1166.
    #[test]
    fn arguments_that_cannot_be_listed_are_reported_rather_than_passed_over() {
        let problems = check(
            &ledger(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"departed\"\n\
                 code = [\"a.rs\"]\ntest = [\"t.rs\"]\nnote = \"declined, ADR 0036.\"\n",
            ),
            &index(),
            &[],
            Path::new("no/such/tree"),
        );
        assert!(
            problems
                .iter()
                .any(|problem| matches!(problem, Problem::ArgumentsUnreadable { .. })),
            "{problems:?}"
        );
    }

    /// Trap 13: the sweep is run against the defect it looks for before it is believed — the
    /// renamed crate this check was written for, beside names the tree has. ADR 1437.
    #[test]
    fn a_note_naming_a_crate_the_tree_does_not_have_is_a_finding() {
        let problems = check(
            &ledger(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"implemented\"\n\
                 code = [\"a.rs\"]\ntest = [\"t.rs\"]\n\
                 note = \"`render-quorra` and `quorra_scene::GroupSpec` draw it; `render-raster`, \
                 `raster_scene::GroupSpec`, `pdf_model::view`, `quorra-gtk`, `pdf-view-worker` and \
                 `doc/corpora/pdf-differences` are here, and `quorra_collection_read` is a \
                 function.\"\n",
            ),
            &index(),
            &[],
            &crate::workspace_root(),
        );
        let named: Vec<&str> = problems
            .iter()
            .filter_map(|problem| match problem {
                Problem::UnknownProgram { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(named, vec!["render-quorra", "quorra-scene"], "{problems:?}");
    }

    /// A `departed` row settles, so a heading above one stops being an aggregate on its account
    /// (ADR 1035 section 5, inherited by ADR 1119) — and that is the direction
    /// [`Problem::AggregateWithoutDebt`] watches.
    #[test]
    fn a_departed_row_owes_nothing_and_a_heading_above_it_owes_nothing_on_its_account() {
        let read = ledger(
            "[[clause]]\nclause = \"8\"\ntitle = \"Graphics\"\nstatus = \"implemented\"\n\
             code = [\"a.rs\"]\ntest = [\"t.rs\"]\n\
             \n[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"departed\"\n\
             code = [\"a.rs\"]\ntest = [\"t.rs\"]\nnote = \"declined, ADR 0036.\"\n",
        );
        assert!(!read.rows[1].status.owes());
        assert!(!read.is_aggregate(&read.rows[0]));
        assert!(read.owing().is_empty());
    }

    /// Trap 13: the sweep is run against the defect it looks for before it is believed. The
    /// defect is clause 6's — a clause of the body that states a requirement and that the
    /// population's constant does not name — so the fixture is one of those.
    #[test]
    fn a_clause_outside_the_population_that_states_a_requirement_is_a_finding() {
        let standard = ClauseIndex::parse(
            "## 5 Version designations\nA PDF processor shall do the thing.\n\n\
             ## 8 Graphics\nx\n\n## 8.1 General\ny\n\n## 8.2 Graphics objects\nz\n"
                .to_owned(),
        );
        let problems = check(&Ledger::default(), &standard, &[], Path::new("."));
        let found = problems.iter().find_map(|problem| match problem {
            Problem::UnrecordedRequirement { clause, shalls } => Some((clause.as_str(), *shalls)),
            _ => None,
        });
        assert_eq!(found, Some(("5", 1)));
    }

    /// The other half of trap 13: the same clause, stating nothing, is not a finding. Without
    /// this the check would pass for every clause of the standard and mean nothing.
    #[test]
    fn a_clause_outside_the_population_that_states_nothing_is_not_a_finding() {
        let standard = ClauseIndex::parse(
            "## 5 Version designations\nHow the versions are named.\n\n\
             ## 8 Graphics\nx\n\n## 8.1 General\ny\n"
                .to_owned(),
        );
        let problems = check(&Ledger::default(), &standard, &[], Path::new("."));
        assert!(
            !problems
                .iter()
                .any(|problem| matches!(problem, Problem::UnrecordedRequirement { .. }))
        );
    }

    /// An annex nobody has sorted into normative or informative is a finding whatever it
    /// states: a letter no list names is a letter no instrument here can see, which is where
    /// all seventeen of them were before ADR 0206.
    #[test]
    fn an_annex_in_neither_list_is_a_finding() {
        let standard = ClauseIndex::parse(
            "## Annex R (normative) Something new\nR states nothing at all.\n".to_owned(),
        );
        let problems = check(&Ledger::default(), &standard, &[], Path::new("."));
        assert!(problems.iter().any(|problem| matches!(
            problem,
            Problem::UnrecordedRequirement { clause, .. } if clause == "R"
        )));
    }

    /// Trap 25 the other way round: an excuse for a clause that has stopped needing one reads
    /// as a pass, because the sweep it silences finds nothing there either.
    #[test]
    fn an_excuse_for_a_clause_that_states_no_requirement_is_stale() {
        let standard = ClauseIndex::parse("## 8 Graphics\nx\n\n## 8.1 General\ny\n".to_owned());
        let problems = check(&Ledger::default(), &standard, &[], Path::new("."));
        assert!(
            problems
                .iter()
                .any(|problem| matches!(problem, Problem::StaleExclusion { clause: 4, .. }))
        );
    }

    /// A heading's span covers its subclauses', so counting over spans would count one
    /// sentence once per level it sits under. Clause 8 states one `shall`, in §8.1.
    #[test]
    fn a_requirement_is_counted_once_rather_than_once_per_level() {
        let standard =
            ClauseIndex::parse("## 8 Graphics\nx\n\n## 8.1 General\nIt shall be so.\n".to_owned());
        assert_eq!(
            requirements_by_group(&standard),
            vec![("8".to_owned(), 1usize)]
        );
    }

    /// Rows for §8, §8.1 and §8.2 with the statuses given, in that order.
    fn family(head: &str, first: &str, second: &str) -> Ledger {
        ledger(&format!(
            "[[clause]]\nclause = \"8\"\ntitle = \"Graphics\"\nstatus = \"{head}\"\n\
             code = [\"a.rs\"]\ntest = [\"t.rs\"]\nnote = \"n\"\n\n\
             [[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"{first}\"\n\
             code = [\"a.rs\"]\ntest = [\"t.rs\"]\nnote = \"n\"\n\n\
             [[clause]]\nclause = \"8.2\"\ntitle = \"Graphics objects\"\nstatus = \"{second}\"\n\
             code = [\"a.rs\"]\ntest = [\"t.rs\"]\nnote = \"n\"\n"
        ))
    }

    #[test]
    fn a_heading_whose_subclause_still_owes_is_an_aggregate_and_not_a_debt() {
        let ledger = family("partial", "partial", "implemented");
        let head = ledger.row(&number("8")).unwrap();
        assert!(ledger.is_aggregate(head));
        // The heading is not work; the subclause that owes is.
        assert_eq!(
            ledger
                .owing()
                .iter()
                .map(|row| row.clause.to_string())
                .collect::<Vec<_>>(),
            vec!["8.1".to_owned()]
        );
    }

    #[test]
    fn a_leaf_is_never_an_aggregate_however_deep_its_number() {
        let ledger = family("implemented", "partial", "implemented");
        let leaf = ledger.row(&number("8.1")).unwrap();
        assert!(!ledger.is_aggregate(leaf));
    }

    #[test]
    fn a_settled_heading_over_an_owing_subclause_is_not_an_aggregate() {
        // `is_aggregate` says "this row's debt is its children's"; a row with no debt has none
        // to attribute, so the answer is no rather than yes-by-accident.
        let ledger = family("implemented", "partial", "implemented");
        let head = ledger.row(&number("8")).unwrap();
        assert!(!ledger.is_aggregate(head));
    }

    #[test]
    fn a_heading_owing_what_no_subclause_owes_is_a_finding() {
        let problems = check(
            &family("partial", "implemented", "implemented"),
            &index(),
            &[],
            Path::new("."),
        );
        assert!(problems.iter().any(|problem| matches!(
            problem,
            Problem::AggregateWithoutDebt { clause, settled_below: 2, .. } if clause == &number("8")
        )));
    }

    #[test]
    fn a_heading_whose_last_child_still_owes_is_not_that_finding() {
        let problems = check(
            &family("partial", "implemented", "reported"),
            &index(),
            &[],
            Path::new("."),
        );
        assert!(
            !problems
                .iter()
                .any(|problem| matches!(problem, Problem::AggregateWithoutDebt { .. }))
        );
    }

    #[test]
    fn a_title_that_drifts_from_the_standard_is_a_finding() {
        let problems = check(
            &ledger("[[clause]]\nclause = \"8.1\"\ntitle = \"Generel\"\nstatus = \"unreviewed\"\n"),
            &index(),
            &[],
            Path::new("."),
        );
        assert!(
            problems
                .iter()
                .any(|problem| matches!(problem, Problem::WrongTitle { .. }))
        );
    }

    /// A tree of one test file holding each kind of function, under a fresh directory.
    fn evidence_tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "conformance-evidence-{name}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::write(
            root.join("tests/t.rs"),
            "fn corpus(name: &str) -> Option<Vec<u8>> {\n    \
             std::fs::read(format!(\"../../doc/pdf.js/test/pdfs/{name}\")).ok()\n}\n\n\
             fn open(name: &str) -> Option<Vec<u8>> {\n    corpus(name)\n}\n\n\
             fn pdfjs_corpus() {}\n\n\
             /// Built here.\n#[test]\n#[expect(\n    clippy::float_cmp,\n    \
             reason = \"exact\"\n)]\nfn a_fixture() {\n    pdfjs_corpus();\n}\n\n\
             #[test]\nfn a_witness() {\n    let Some(_) = open(\"a.pdf\") else {\n        \
             return;\n    };\n}\n\n\
             #[test]\n#[ignore = \"a walk\"]\nfn a_walk() {}\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn each_kind_of_test_is_told_apart() {
        let root = evidence_tree("kinds");
        let mut classifier = TestClassifier::new(&root);
        assert_eq!(
            classifier.classify("tests/t.rs::a_fixture"),
            TestKind::Fixture
        );
        assert_eq!(
            classifier.classify("tests/t.rs::a_witness"),
            TestKind::CorpusWitness,
            "a corpus read two helpers away is still a corpus read"
        );
        assert_eq!(classifier.classify("tests/t.rs::a_walk"), TestKind::Walk);
        assert_eq!(classifier.classify("tests/t.rs::open"), TestKind::Census);
        assert_eq!(classifier.classify("tests/t.rs"), TestKind::File);
        assert_eq!(classifier.classify("tests/t.rs::gone"), TestKind::Missing);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Calibrated both ways (trap 13): the same row is a finding with a walk and a witness, and
    /// stops being one the moment it names a fixture.
    #[test]
    fn an_implemented_row_held_only_by_walks_is_a_finding_and_a_fixture_ends_it() {
        let root = evidence_tree("rows");
        std::fs::write(root.join("a.rs"), "").unwrap();
        let row = |tests: &str| {
            ledger(&format!(
                "[[clause]]\nclause = \"8.1\"\ntitle = \"General\"\nstatus = \"implemented\"\n\
                 code = [\"a.rs\"]\ntest = [{tests}]\n"
            ))
        };
        let only_walks = |ledger: &Ledger| {
            check(ledger, &index(), &[], &root)
                .into_iter()
                .filter(|problem| matches!(problem, Problem::OnlyWalks { .. }))
                .count()
        };
        let walks = row("\"tests/t.rs::a_walk\", \"tests/t.rs::a_witness\"");
        assert_eq!(only_walks(&walks), 1);
        let held = row("\"tests/t.rs::a_walk\", \"tests/t.rs::a_fixture\"");
        assert_eq!(only_walks(&held), 0);
        let mut classifier = TestClassifier::new(&root);
        let counts = classifier.holdings(&walks);
        assert!(counts.contains(&(
            Status::Implemented,
            HoldingCounts {
                fixture: 0,
                only_walks: 1,
                empty: 0
            }
        )));
        std::fs::remove_dir_all(root).unwrap();
    }
}
