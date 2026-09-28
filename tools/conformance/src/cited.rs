//! The twenty-seventh sweep: a file that cites a clause, against that clause's own `code` list.
//!
//! # The shape it exists for
//!
//! A row's `code` array is the ledger's answer to *where is this implemented*, and a reader uses
//! it as an index: open the row, open the file. Nothing kept it complete. The gate checks that
//! every path a row names **exists** ([`crate::ledger`]'s `MissingSite`), and the ledger's own
//! `CitedButUnreviewed` check asks the reverse question once — a clause the code cites may not be
//! left `unreviewed` — but neither asks whether the files that cite a clause are the files its row
//! names.
//!
//! Session 1210 found the shape by reading: §12.7.3, §7.9.6 and §14.13.2 are each read by
//! `pdf-transform`'s archive converter, which cites all three by number in its comments, and not
//! one of the three rows named it. The converter had been there for batches. **A `code` list goes
//! stale in one direction only** — a round that adds a reader cites the clause beside the code,
//! because `CLAUDE.md` requires the citation, and editing a row in another file is the step it
//! forgets — so the citation is the live half and the list is the half that decays.
//!
//! # The discriminator, and why it is a rank rather than a filter
//!
//! A `§` in a comment is not always a claim to implement the clause. This tree cites a clause to
//! say what a value is *not* (§8.9.6.2 beside `/Mask`), to point at a neighbour, to explain a
//! refusal. Nothing mechanical separates those from an implementation, so the sweep ranks on the
//! two things that are decidable:
//!
//! - **What the row claims.** A row that says work was done — `implemented`, `partial`,
//!   `departed` — has a `code` list that is supposed to be complete, and a citing file missing
//!   from it is a gap in an index somebody uses. A row claiming no work has nothing for the file
//!   to be missing from, and a citation under one is usually a comment saying why.
//! - **How often the file cites it.** One `§` is a cross-reference as often as not; a file citing
//!   one clause several times is reading that clause.
//!
//! So a finding is a row claiming work cited [`REPEATED`] times or more by a library source it
//! does not name, and the rungs order the findings by what the row names of the file's crate:
//! [`Rung::UnknownCrate`] where it names no file of that crate at all, [`Rung::UnknownFile`] where
//! it names others. A citation cited fewer times, from a test or an example, or under a row that
//! claims no work is counted and not listed. Read the first rung and stop when the hits stop
//! being the rule.
//!
//! # What is counted rather than listed
//!
//! A file named by the row's `code` or by its `test` is [`Reach::Named`], and the `test` half is
//! not a concession: a test file citing the clause it tests is exactly where a `test` entry goes,
//! and reporting it would be asking the row to name the same file twice. A clause with no row at
//! all is one of two things. Where the number is a heading the ledger rows nothing under **by
//! design** — an informative annex, or a whole clause — it is [`Reach::Unrowed`], and
//! [`Unrowed`] names the class and its reason; the report prints each class as a count and the
//! numbers it holds, not as a list. Anything else is [`Reach::NoRow`], listed pair by pair and
//! read first, and the ones under [`RASTER`] are counted apart for the reason its own comment
//! gives.
//!
//! The split is what makes the last line an instrument. While every row-less pair shared one
//! count, the count held the standard's own informative annexes and could never reach zero, so a
//! real gap arriving beside them moved a number nobody expected to move. With the by-design
//! classes named, the no-row count is zero on a clean tree and rises only for a pair a person has
//! to read: a front-matter subclause, or a section sign that meant another document. The
//! conformance gate already refuses a number the standard does not print, so every pair in a
//! class names a real heading. What a class cannot see is a `§` that meant another document's
//! section and happens to spell an informative annex's number or a whole clause's; the numbers
//! each class prints are there so that a new one is visible. ADR 1355.
//!
//! # A checker citing the clause it checks
//!
//! A crate that *checks* a document against another standard cites the ISO 32000-2 clause each
//! check reads, and it cites it often: `pdf-archive`'s ISO 19005 requirements each name the clause
//! whose structure they examine. Those citations are claims about what a check reads, not about
//! where the clause is implemented, so the row is right not to name the file — and while the
//! checkers had no rung of their own every one of them sat on the top rung, re-read by every round
//! that ran the sweep. [`CHECKERS`] names the crates, [`Rung::Checker`] is where their pairs go,
//! and the report prints that rung as a count per crate rather than as a list, beside any crate
//! [`CHECKERS`] names that the workspace no longer has, so that the list cannot rot quietly.
//!
//! # Why it is not a gate
//!
//! [`crate::pointers`]' reason, in the other direction: the sweep cannot tell an implementing
//! citation from a cross-reference, so a build that failed on one would teach rounds to write
//! around the checker by dropping the citation `CLAUDE.md` asks for. It prints, and a person reads.
//!
//! ADR 1274.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::clause::ClauseNumber;
use crate::entries;
use crate::ledger::{Ledger, Status};

/// How many citations of one clause in one file make it a reading rather than a reference.
///
/// Three, and the calibration is what set it: at two, `pdf-transform`'s converter is still named
/// on the top rung and so is a run of files whose single `§` is a "see also". The constant is the
/// rank's only number and it moves nothing else.
pub const REPEATED: usize = 3;

/// The workspace members that check a document against another standard, rather than implement
/// ISO 32000-2, each with the reason it is one.
///
/// A list here rather than a marker in each crate so that the population is named where the
/// report is, and a crate joins it by an argument in this file rather than by a line nobody reads
/// beside its own code.
pub const CHECKERS: [(&str, &str); 1] = [(
    "crates/pdf-archive",
    "decides ISO 19005 conformance, citing the ISO 32000-2 clause each requirement examines",
)];

/// The directory whose no-row pairs the report lists on lines of their own.
///
/// `raster/` is the rendering library's own tree, with its own design documents under
/// `raster/doc/`, and a section of one of those is written "section N" there because a `§` reads
/// to [`crate::citation`] as a clause of ISO 32000-2. A no-row pair under it is therefore either
/// an ISO 32000-2 clause the ledger carries no row for or a section sign that means the library's
/// own document, and only reading the line says which; the report lists them so that what is left
/// is printed rather than written down.
pub const RASTER: &str = "raster/";

/// Why the sweep could not be run.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The tree's sources could not be walked.
    #[error("the tree's sources could not be walked: {0}")]
    Sources(#[from] std::io::Error),
    /// The tree's shape could not be derived from the workspace manifest.
    #[error(transparent)]
    Roots(#[from] crate::roots::Error),
}

/// What a citing file's relation to the clause's row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reach {
    /// The row names the file, in `code` or in `test`.
    Named,
    /// The ledger carries no row for the clause, because the number is a heading it rows nothing
    /// under by design.
    Unrowed(Unrowed),
    /// The ledger carries no row for the clause and the number belongs to no class that says
    /// why. Read first.
    NoRow,
    /// The row does not name the file. The finding.
    Unnamed,
}

/// A heading the ledger rows nothing under by design, as far as the number alone says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unrowed {
    /// A number of one of [`crate::ledger::INFORMATIVE_ANNEXES`], which state nothing a row
    /// could hold.
    InformativeAnnex,
    /// A top-level clause of the body, `§11` or `§2`: the container a covered clause's rows sit
    /// under, or a front-matter clause the ledger does not cover
    /// ([`crate::ledger::NORMATIVE_CLAUSES`]).
    WholeClause,
}

impl Unrowed {
    /// Every class, in report order.
    pub const ALL: [Self; 2] = [Self::InformativeAnnex, Self::WholeClause];

    /// The class a row-less number belongs to, or `None` where the number alone gives no reason
    /// for the ledger to carry no row.
    ///
    /// A normative annex's number is never in a class: [`crate::ledger::NORMATIVE_ANNEXES`] are
    /// rowed to the leaf, so a row-less one is a gap. Nor is a subclause of the front matter
    /// (clauses 1 to 5), which is where a section sign that meant an ADR's or an RFC's section
    /// most often lands.
    #[must_use]
    pub fn of(clause: &ClauseNumber) -> Option<Self> {
        match clause.annex() {
            Some(letter) => crate::ledger::INFORMATIVE_ANNEXES
                .contains(&letter)
                .then_some(Self::InformativeAnnex),
            None => (clause.depth() == 1).then_some(Self::WholeClause),
        }
    }

    /// Why a number of this class carries no row.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::InformativeAnnex => "an informative annex, which states nothing a row could hold",
            Self::WholeClause => {
                "a whole clause, the heading the ledger's rows sit under or front matter it does \
                 not cover"
            }
        }
    }
}

/// Where a finding sits in the reading order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// A row claiming work whose `code` names **no file of this crate at all**, read
    /// [`REPEATED`] times or more by a library source of it. Session 1210's own shape.
    UnknownCrate,
    /// A row claiming work whose `code` names this crate and not this file, read [`REPEATED`]
    /// times or more by a library source of it.
    UnknownFile,
    /// Either of the two above, in a crate [`CHECKERS`] names: a checker citing the clause it
    /// checks, which is not a gap in the row. Counted per crate rather than listed.
    Checker,
}

/// One file's citations of one clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citing {
    /// The clause cited.
    pub clause: ClauseNumber,
    /// The file citing it, relative to the tree's root.
    pub path: String,
    /// How many times it cites it.
    pub citations: usize,
    /// The 1-based line of the first citation.
    pub first: usize,
    /// What the row says of the file.
    pub reach: Reach,
    /// What the row claims, where there is one.
    pub status: Option<Status>,
    /// Whether the citing file is a library source rather than a test, an example or a bench.
    ///
    /// A `code` list is where a clause is implemented and a `test` list is what holds it, so a
    /// test citing the clause it tests is named in the other array or in neither on purpose.
    pub is_library: bool,
    /// Whether the row's `code` names any file of the citing file's own crate.
    pub crate_named: bool,
    /// The [`CHECKERS`] member the citing file lies in, where it lies in one.
    pub checker: Option<&'static str>,
}

impl Citing {
    /// Which rung a finding sits on, or `None` where it is not a finding.
    #[must_use]
    pub fn rung(&self) -> Option<Rung> {
        if self.reach != Reach::Unnamed
            || !self.status.is_some_and(claims_work)
            || !self.is_library
            || self.citations < REPEATED
        {
            return None;
        }
        if self.checker.is_some() {
            Some(Rung::Checker)
        } else if self.crate_named {
            Some(Rung::UnknownFile)
        } else {
            Some(Rung::UnknownCrate)
        }
    }
}

/// The workspace member a path lies in, chosen from the members themselves.
///
/// The longest member directory that is a prefix of the path, so that a site the ledger lists
/// outside Cargo's own directory names — `crates/viewer-qt/cpp/window.cpp`, which is the C++ half
/// of that crate — is still that crate's. Derived from the manifest rather than from a list of
/// directory names here (trap 25); a list of names read `cpp/` as no crate at all and put a file
/// its own crate *does* name on the wrong rung.
#[must_use]
pub fn crate_of<'a>(members: &'a [String], path: &str) -> Option<&'a str> {
    members
        .iter()
        .filter(|member| path.starts_with(&format!("{member}/")))
        .max_by_key(|member| member.len())
        .map(String::as_str)
}

/// The [`CHECKERS`] member a path lies in, where it lies in one.
#[must_use]
pub fn checker_of(members: &[String], path: &str) -> Option<&'static str> {
    let home = crate_of(members, path)?;
    CHECKERS
        .iter()
        .map(|(member, _)| *member)
        .find(|member| *member == home)
}

/// Whether a path is a library source rather than a test, an example, a bench or a fuzz target.
#[must_use]
pub fn is_library(path: &str) -> bool {
    path.contains("/src/") && !path.contains("/src/bin/")
}

/// Whether a status says work was done, so that its `code` list is supposed to be complete.
#[must_use]
pub fn claims_work(status: Status) -> bool {
    matches!(
        status,
        Status::Implemented | Status::Partial | Status::Departed
    )
}

/// Everything one run found.
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// Every (file, clause) pair, in file order.
    pub citing: Vec<Citing>,
    /// How many source files were read.
    pub files_read: usize,
    /// Every crate [`CHECKERS`] names that the workspace does not have.
    pub absent_checkers: Vec<&'static str>,
}

impl Found {
    /// The findings on one rung, worst-cited first.
    #[must_use]
    pub fn on(&self, rung: Rung) -> Vec<&Citing> {
        let mut found: Vec<&Citing> = self
            .citing
            .iter()
            .filter(|citing| citing.rung() == Some(rung))
            .collect();
        found.sort_by(|left, right| {
            right
                .citations
                .cmp(&left.citations)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.clause.cmp(&right.clause))
        });
        found
    }

    /// The pairs with no row at all whose citing file lies under `prefix`.
    #[must_use]
    pub fn no_row_under(&self, prefix: &str) -> Vec<&Citing> {
        self.citing
            .iter()
            .filter(|citing| citing.reach == Reach::NoRow && citing.path.starts_with(prefix))
            .collect()
    }

    /// The pairs citing a heading of one [`Unrowed`] class.
    #[must_use]
    pub fn unrowed(&self, class: Unrowed) -> Vec<&Citing> {
        self.citing
            .iter()
            .filter(|citing| citing.reach == Reach::Unrowed(class))
            .collect()
    }

    /// How many pairs reached the row the way they should.
    #[must_use]
    pub fn counted(&self, reach: Reach) -> usize {
        self.citing
            .iter()
            .filter(|citing| citing.reach == reach)
            .count()
    }
}

/// Runs the sweep over the tree's sources and a ledger.
///
/// # Errors
///
/// If a source or a directory cannot be read.
pub fn sweep(root: &Path, ledger: &Ledger) -> Result<Found, Error> {
    let scanned = crate::scan_tree(root)?;
    let members = crate::roots::source_roots(root)?;
    Ok(judge(&scanned, ledger, &members))
}

/// The sweep's own arithmetic, over sources already scanned.
///
/// Apart from [`sweep`] so that the calibration can run it against a ledger held in memory
/// rather than against a file somebody has to remember to restore.
#[must_use]
pub fn judge(
    scanned: &[(PathBuf, crate::citation::Scan)],
    ledger: &Ledger,
    members: &[String],
) -> Found {
    let mut found = Found {
        files_read: scanned.len(),
        absent_checkers: CHECKERS
            .iter()
            .map(|(member, _)| *member)
            .filter(|member| !members.iter().any(|known| known == member))
            .collect(),
        ..Found::default()
    };
    for (path, scan) in scanned {
        let named = path.to_string_lossy().replace('\\', "/");
        let mut per_clause: BTreeMap<ClauseNumber, (usize, usize)> = BTreeMap::new();
        for citation in &scan.citations {
            let entry = per_clause
                .entry(citation.number.clone())
                .or_insert((0, citation.line));
            entry.0 = entry.0.saturating_add(1);
            entry.1 = entry.1.min(citation.line);
        }
        for (clause, (citations, first)) in per_clause {
            let row = ledger.row(&clause);
            let reach = match row {
                None => Unrowed::of(&clause).map_or(Reach::NoRow, Reach::Unrowed),
                Some(row) => {
                    let listed = row.code.iter().chain(row.test.iter()).any(|site| {
                        let site = site
                            .split_once("::")
                            .map_or(site.as_str(), |(path, _)| path);
                        entries::covered_by(site, &named)
                    });
                    if listed { Reach::Named } else { Reach::Unnamed }
                }
            };
            let crate_named = row.is_some_and(|row| {
                crate_of(members, &named).is_some_and(|home| {
                    row.code
                        .iter()
                        .any(|site| crate_of(members, site).is_some_and(|listed| listed == home))
                })
            });
            found.citing.push(Citing {
                clause,
                path: named.clone(),
                citations,
                first,
                reach,
                status: row.map(|row| row.status),
                is_library: is_library(&named),
                crate_named,
                checker: checker_of(members, &named),
            });
        }
    }
    found
}

/// What the calibration answered: the pair it chose, and the rung that pair lands on with the
/// file plucked out of its row and with the list intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calibration {
    /// The clause and file the sweep chose to pluck, for the failure message.
    pub named: String,
    /// The rung with the file taken out of its row's `code`. Must be `Some`.
    pub gapped: Option<Rung>,
    /// The rung with the list intact. Must be `None`.
    pub intact: Option<Rung>,
}

/// The calibration: a row has one of its own citing files taken out of its `code` list, and the
/// sweep is asked whether it now names it.
///
/// Trap 13 — a sweep that comes back clean is a sentence about the sweep. The plant is made in
/// memory, against the real tree, so nothing has to be restored afterwards, and **the pair is
/// chosen by the sweep rather than written here**: the first library file the ledger already
/// names for a clause it reads [`REPEATED`] times or more. A hand-written pair would be a name
/// that can rot into a calibration nobody notices has stopped calibrating (trap 25).
///
/// The answer is the rung with the file plucked out and the rung with the list intact, which must
/// be `Some` and `None` in that order.
///
/// # Errors
///
/// If a source, a directory or the manifest cannot be read.
pub fn calibrate(root: &Path, ledger: &Ledger) -> Result<Option<Calibration>, Error> {
    let scanned = crate::scan_tree(root)?;
    let members = crate::roots::source_roots(root)?;
    let intact = judge(&scanned, ledger, &members);
    let Some(subject) = intact.citing.iter().find(|citing| {
        citing.reach == Reach::Named
            && citing.is_library
            && citing.status.is_some_and(claims_work)
            && citing.citations >= REPEATED
            && ledger
                .row(&citing.clause)
                .is_some_and(|row| row.code.contains(&citing.path))
    }) else {
        return Ok(None);
    };
    let mut plucked = ledger.clone();
    for row in &mut plucked.rows {
        if row.clause == subject.clause {
            row.code.retain(|site| site != &subject.path);
            row.test.retain(|site| !site.starts_with(&subject.path));
        }
    }
    let gapped = judge(&scanned, &plucked, &members);
    let rung_of = |found: &Found| {
        found
            .citing
            .iter()
            .find(|citing| citing.clause == subject.clause && citing.path == subject.path)
            .and_then(Citing::rung)
    };
    Ok(Some(Calibration {
        named: format!("§{} {}", subject.clause, subject.path),
        gapped: rung_of(&gapped),
        intact: rung_of(&intact),
    }))
}

/// The report: the two rungs a person reads, the checkers' rung by crate, then what the run was
/// clean over.
#[must_use]
pub fn report(found: &Found) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} (file, clause) pair(s) over {} source file(s)\n",
        found.citing.len(),
        found.files_read,
    );
    for (rung, why) in [
        (
            Rung::UnknownCrate,
            "a row claiming work whose `code` names no file of the reading crate at all",
        ),
        (
            Rung::UnknownFile,
            "a row claiming work whose `code` names the reading crate and not the reading file",
        ),
    ] {
        let hits = found.on(rung);
        let _ = writeln!(out, "{} {why}", hits.len());
        for citing in hits {
            let _ = writeln!(
                out,
                "  §{} {}:{} cited {}",
                citing.clause, citing.path, citing.first, citing.citations,
            );
        }
    }
    let checked = found.on(Rung::Checker);
    let mut per_checker: BTreeMap<&str, usize> = BTreeMap::new();
    for citing in &checked {
        if let Some(checker) = citing.checker {
            let count = per_checker.entry(checker).or_default();
            *count = count.saturating_add(1);
        }
    }
    let _ = writeln!(
        out,
        "{} pair(s) a checker cites for the clause it checks, not listed",
        checked.len()
    );
    for (member, why) in CHECKERS {
        let _ = writeln!(
            out,
            "  {member}: {} — {why}",
            per_checker.get(member).copied().unwrap_or(0)
        );
    }
    for member in &found.absent_checkers {
        let _ = writeln!(
            out,
            "  {member} is named as a checker and is not a workspace member"
        );
    }
    let _ = writeln!(
        out,
        "\n{} pair(s) the row already names",
        found.counted(Reach::Named),
    );
    for class in Unrowed::ALL {
        let pairs = found.unrowed(class);
        let mut numbers: Vec<&ClauseNumber> = pairs.iter().map(|citing| &citing.clause).collect();
        numbers.sort();
        numbers.dedup();
        let numbers: Vec<String> = numbers.iter().map(|number| format!("§{number}")).collect();
        let _ = writeln!(
            out,
            "{} pair(s) cite {} — not listed; numbers: {}",
            pairs.len(),
            class.reason(),
            if numbers.is_empty() {
                "none".to_owned()
            } else {
                numbers.join(" ")
            },
        );
    }
    let _ = writeln!(
        out,
        "{} pair(s) with no row at all and no class that says why",
        found.counted(Reach::NoRow),
    );
    let _ = writeln!(
        out,
        "{} of the no-row pair(s) lie under {RASTER}",
        found.no_row_under(RASTER).len(),
    );
    for citing in found.no_row_under("") {
        let _ = writeln!(
            out,
            "  §{} {}:{} cited {} — {}",
            citing.clause,
            citing.path,
            citing.first,
            citing.citations,
            why_no_row(&citing.clause),
        );
    }
    out
}

/// What a row-less number outside every [`Unrowed`] class most likely is, for the line that lists
/// it.
///
/// The pairs that are *wrong* — a section of an ADR, an RFC or a `doc/todo` file whose name sits
/// at the end of the line before, so that the sign reads as ISO 32000-2's — look exactly like a
/// citation of the standard, and only the line tells them apart, so the reading is a pointer and
/// not a verdict.
#[must_use]
pub fn why_no_row(clause: &ClauseNumber) -> &'static str {
    match (clause.annex(), clause.clause()) {
        (Some(_), _) => "a normative annex's number the ledger does not carry: read the line",
        (None, Some(top)) if !crate::ledger::NORMATIVE_CLAUSES.contains(&top) => {
            "a subclause of the front matter the ledger does not cover: read the line"
        }
        _ => "a number the ledger does not carry: read the line",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn citing(citations: usize, reach: Reach, status: Option<Status>) -> Citing {
        Citing {
            clause: "8.4.5".parse().expect("a clause number"),
            path: "crates/a/src/b.rs".to_owned(),
            citations,
            first: 1,
            reach,
            status,
            is_library: true,
            crate_named: false,
            checker: None,
        }
    }

    #[test]
    fn a_named_file_is_no_finding_however_often_it_cites() {
        assert_eq!(
            citing(9, Reach::Named, Some(Status::Implemented)).rung(),
            None
        );
    }

    #[test]
    fn a_heading_the_ledger_does_not_row_by_design_has_a_class_and_nothing_else_does() {
        let class = |text: &str| Unrowed::of(&text.parse().expect("a clause number"));
        assert_eq!(class("C.4"), Some(Unrowed::InformativeAnnex));
        assert_eq!(class("B.2"), Some(Unrowed::InformativeAnnex));
        assert_eq!(class("11"), Some(Unrowed::WholeClause));
        assert_eq!(class("2"), Some(Unrowed::WholeClause));
        assert_eq!(class("F.9"), None, "a normative annex is rowed to the leaf");
        assert_eq!(class("5.3"), None, "a front-matter subclause is read");
        assert_eq!(class("8.4.99"), None);
        let reason = |text: &str| why_no_row(&text.parse().expect("a clause number"));
        assert!(reason("F.9").starts_with("a normative annex's number"));
        assert!(reason("5.3").starts_with("a subclause of the front matter"));
        assert!(reason("8.4.99").starts_with("a number the ledger does not carry"));
    }

    #[test]
    fn a_classed_pair_is_counted_by_class_and_not_as_a_pair_with_no_row() {
        let annex = Citing {
            clause: "C.4".parse().expect("a clause number"),
            ..citing(1, Reach::Unrowed(Unrowed::InformativeAnnex), None)
        };
        let gap = Citing {
            clause: "5.3".parse().expect("a clause number"),
            ..citing(1, Reach::NoRow, None)
        };
        let found = Found {
            citing: vec![annex, gap],
            ..Found::default()
        };
        let report = report(&found);
        assert!(report.contains(
            "1 pair(s) cite an informative annex, which states nothing a row could hold — not \
             listed; numbers: §C.4"
        ));
        assert!(report.contains("0 pair(s) cite a whole clause"));
        assert!(report.contains("1 pair(s) with no row at all and no class that says why"));
        assert!(report.contains("  §5.3 crates/a/src/b.rs:1"));
        assert!(!report.contains("  §C.4 "));
    }

    #[test]
    fn a_clause_with_no_row_is_no_finding() {
        assert_eq!(citing(9, Reach::NoRow, None).rung(), None);
    }

    #[test]
    fn the_no_row_pairs_under_raster_are_counted_and_listed_apart() {
        let under = Citing {
            path: "raster/crates/raster-gpu/src/a.rs".to_owned(),
            ..citing(1, Reach::NoRow, None)
        };
        let found = Found {
            citing: vec![
                under.clone(),
                citing(1, Reach::NoRow, None),
                Citing {
                    reach: Reach::Named,
                    ..under.clone()
                },
            ],
            ..Found::default()
        };
        assert_eq!(found.no_row_under(RASTER), vec![&under]);
        let report = report(&found);
        assert!(report.contains("1 of the no-row pair(s) lie under raster/"));
        assert!(report.contains("  §8.4.5 raster/crates/raster-gpu/src/a.rs:1"));
    }

    #[test]
    fn the_rank_is_the_row_s_claim_the_count_and_whether_the_crate_is_named() {
        assert_eq!(
            citing(REPEATED, Reach::Unnamed, Some(Status::Implemented)).rung(),
            Some(Rung::UnknownCrate)
        );
        let mut named = citing(REPEATED, Reach::Unnamed, Some(Status::Partial));
        named.crate_named = true;
        assert_eq!(named.rung(), Some(Rung::UnknownFile));
    }

    #[test]
    fn a_row_claiming_no_work_a_thin_citation_and_a_test_are_not_findings() {
        assert_eq!(
            citing(9, Reach::Unnamed, Some(Status::Silent)).rung(),
            None,
            "a row that claims no work has no `code` list for a file to be missing from"
        );
        assert_eq!(
            citing(REPEATED - 1, Reach::Unnamed, Some(Status::Implemented)).rung(),
            None,
            "one or two citations is a cross-reference as often as a reading"
        );
        let mut test = citing(9, Reach::Unnamed, Some(Status::Implemented));
        test.is_library = false;
        assert_eq!(
            test.rung(),
            None,
            "a test citing the clause it tests belongs to the `test` array"
        );
    }

    #[test]
    fn a_checker_citing_the_clause_it_checks_has_a_rung_of_its_own() {
        let mut checked = citing(REPEATED, Reach::Unnamed, Some(Status::Implemented));
        checked.checker = Some(CHECKERS[0].0);
        assert_eq!(checked.rung(), Some(Rung::Checker));
        checked.crate_named = true;
        assert_eq!(
            checked.rung(),
            Some(Rung::Checker),
            "whether or not the row names the crate"
        );
        let members = vec![CHECKERS[0].0.to_owned()];
        assert_eq!(
            checker_of(&members, &format!("{}/src/survey.rs", CHECKERS[0].0)),
            Some(CHECKERS[0].0)
        );
        assert_eq!(checker_of(&members, "crates/pdf-model/src/page.rs"), None);
    }

    #[test]
    fn a_member_is_chosen_by_the_longest_prefix_and_not_by_cargos_directory_names() {
        let members = vec!["crates/viewer-qt".to_owned(), "crates/viewer-ui".to_owned()];
        assert_eq!(
            crate_of(&members, "crates/viewer-qt/cpp/window.cpp"),
            Some("crates/viewer-qt")
        );
        assert_eq!(
            crate_of(&members, "crates/viewer-qt/src/host.rs"),
            Some("crates/viewer-qt")
        );
        assert_eq!(crate_of(&members, "doc/todo/01.md"), None);
    }

    #[test]
    fn a_module_root_covers_the_files_split_out_of_it() {
        assert!(entries::covered_by(
            "crates/pdf-model/src/content.rs",
            "crates/pdf-model/src/content/text.rs"
        ));
    }
}
