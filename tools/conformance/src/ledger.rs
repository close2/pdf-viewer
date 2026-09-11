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
/// **This was `TECHNICAL_CLAUSES: 7..=14` until the nine-hundred-and-seventy-third session**,
/// and clause 6 is what it gained. Clause 6 states eleven `shall`s and every one of them is
/// addressed to a PDF file or to a PDF processor — §6.3.2.2's three obligations on a processor
/// that renders a page are the ranking `CLAUDE.md`'s *what done means* is written around, and
/// this tree cited that subclause forty-seven times across five crates while the ledger had no
/// row for it at all. ADR 0984.
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
/// **This constant did not exist until the three-hundred-and-sixtieth session**, and the
/// eight letters in it were outside every instrument this project has: not citable, not
/// checkable and not recorded. `CLAUDE.md`'s scope section names clauses because that is how
/// the standard's *body* is organised, and its closed exclusion list says nothing about an
/// annex — so the annexes were in scope all along and nothing was looking at them. Annexes
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
/// The vocabulary exists to keep five different situations from wearing one word: the
/// project *choosing* ([`Status::OutOfScope`]), the project *not knowing*
/// ([`Status::Unreviewed`]), the project *owing out loud* ([`Status::Reported`], and
/// [`Status::Partial`] for part of a clause), the project *owing in silence*
/// ([`Status::Silent`]), and the requirement having no meaning for a screen
/// ([`Status::Inapplicable`]).
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
    /// **The wording was "describes a marking device rather than a screen" until the
    /// three-hundred-and-seventy-fifth session**, and ISO 32000-2 does not contain that
    /// phrase: §8.3.2.2's term is a "raster output device *such as a display or a
    /// printer*", which is why §10.5's transfer function spent three hundred and
    /// fifty-seven sessions in this status wrongly (ADR 0204). A status whose *definition*
    /// names a device the standard does not is a status that invites the mistake.
    Inapplicable,
    /// The requirement addresses a generator of the *content* this program does not generate.
    ///
    /// **The wording was "we do not create files" until the five-hundred-and-tenth session and
    /// "this program writes only §7.5.6's updates" until the eight-hundred-and-eighty-sixth**,
    /// each time because the exclusion behind it had moved and this enum had not. The first
    /// correction followed `CLAUDE.md`'s amendment for §7.5.6's incremental update; the second
    /// follows RFC 0002 §11.1's, ratified 2026-09-03 (ADR 0816), which admits a whole-file
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
    /// the generated header's vocabulary lives; the generator once held a sentence a session
    /// had corrected in the file and stamped it back (ADR 0345).
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
            Self::Reported => "reported",
            Self::Silent => "silent",
            Self::Inapplicable => "inapplicable",
            Self::WriterSide => "writer-side",
            Self::OutOfScope => "out-of-scope",
            Self::Unreviewed => "unreviewed",
        }
    }

    /// Every status, in the order the summary prints them.
    #[must_use]
    pub fn all() -> [Self; 8] {
        [
            Self::Implemented,
            Self::Partial,
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
    /// **This doc comment said "deprecated by ISO 32000-2 itself and specified outside it"
    /// until the five-hundred-and-tenth session**, the exact wording `CLAUDE.md` amended after
    /// the three-hundred-and-sixtieth found Annex K normative and *inside* the standard —
    /// §K's own row quotes the correction, and this copy one directory from the ledger was
    /// never swept with it (ADR 0345).
    Xfa,
    /// JavaScript and script-driven form behaviour. Field *appearance* is not excluded.
    Script,
    /// Writer-side requirements: they address whoever generated the content, or a construct
    /// this program's writers do not emit (see [`Status::WriterSide`]). Annex F's linearisation
    /// is the standing member — `CLAUDE.md` keeps it excluded "until linearisation is
    /// separately ratified", which is a statement about this program's writers rather than
    /// about the annex.
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
    #[error("{path}: {source}")]
    Malformed {
        /// The path that was read.
        path: String,
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
            ParseError::Toml(source) => LedgerError::Malformed {
                path: path.display().to_string(),
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
        let tables = toml_subset::parse(text).map_err(ParseError::Toml)?;
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

/// Why a ledger's text could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The text is outside the accepted subset.
    #[error(transparent)]
    Toml(toml_subset::TomlError),
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
                "§{clause} has no row; regenerate the ledger with `cargo run -p conformance --bin ledger`"
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

    problems.extend(check_population(index));

    problems
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
}
