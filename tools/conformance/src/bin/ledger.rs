//! Generates `doc/conformance/ledger.toml`, and regenerates it without losing what is in it.
//!
//! Run it after nothing in particular: the ledger is generated once and then edited by
//! people, and this program exists so that the *set of rows* is never edited by hand. It
//! reads the standard's clause index, keeps every existing row exactly as written, adds an
//! `unreviewed` row for any subclause that has none, refreshes each row's title from the
//! standard, and writes the file back in clause order.
//!
//! A row is never deleted by this program. If the standard's conversion loses a heading, the
//! row for it stays and the gate reports it as naming a clause that does not exist — which
//! is a finding about the conversion, and not something to fix by dropping the row.
//!
//! ```text
//! cargo run -p conformance --bin ledger
//! ```

#![forbid(unsafe_code)]
#![expect(
    clippy::print_stdout,
    reason = "a command-line program whose whole output is a report on what it wrote"
)]

use std::process::ExitCode;

use conformance::clause::ClauseIndex;
use conformance::ledger::{Exclusion, Ledger, NORMATIVE_ANNEXES, Row, Status, TECHNICAL_CLAUSES};

/// Written above the rows, as `#` comments, every time the file is generated.
const PREAMBLE: &str = "\
The conformance ledger: one row per subclause of ISO 32000-2's technical clauses.

GENERATED — the set of rows is. Their statuses are not: a status is a claim a person makes
after reading the clause against this code, and `cargo run -p conformance --bin ledger`
preserves every one of them. It only ever adds rows the standard has and this file lacks.

Statuses, and what each is for. The vocabulary exists to keep five situations from wearing
one word: the project choosing, the project not knowing, the project owing out loud, the
project owing in silence, and the requirement having no meaning for a screen.

  implemented   every normative requirement in the clause is executed; names code and test
  partial       some are; the note says which, which are not, and what is reported
  reported      not implemented yet, but detected and reported at runtime. Still owed
  silent        not implemented, and nothing says so. A page is drawn wrong without a word
  inapplicable  the requirement has no meaning for this device. Nothing is owed
  writer-side   addresses a PDF generator; this program writes only §7.5.6's updates
  out-of-scope  covered by CLAUDE.md principle 5's closed exclusion list, which the row names
  unreviewed    nobody has read this clause against this code

`unreviewed` is debt, not absence: it says only that the question has not been asked.
`silent` is the status worth hunting. Every missing subsystem here reports, because whoever
decided not to build it wrote the report; what ships is the gap inside a feature that is
already there, and only reading the clause finds one.

See doc/PLAN.md section 5a for the design, and tools/conformance for the checker that reads this.";

fn main() -> ExitCode {
    let root = conformance::workspace_root();
    let index = match ClauseIndex::read(&root.join(conformance::STANDARD)) {
        Ok(index) => index,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let path = root.join(conformance::LEDGER);
    let existing = if path.exists() {
        match Ledger::read(&path) {
            Ok(ledger) => ledger,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        Ledger::default()
    };

    let mut clauses: Vec<_> = TECHNICAL_CLAUSES
        .flat_map(|clause| index.subclauses_of(clause))
        .chain(
            NORMATIVE_ANNEXES
                .into_iter()
                .flat_map(|annex| index.numbers_of_annex(annex)),
        )
        .collect();
    clauses.sort();

    let mut generated = Ledger::default();
    let mut added = 0usize;
    for clause in clauses {
        let title = index.title(&clause).unwrap_or_default().to_owned();
        let row = if let Some(row) = existing.row(&clause) {
            Row {
                title,
                ..row.clone()
            }
        } else {
            added = added.saturating_add(1);
            new_row(clause, title)
        };
        generated.rows.push(row);
    }

    let dropped = existing
        .rows
        .iter()
        .filter(|row| generated.row(&row.clause).is_none())
        .count();
    if dropped > 0 {
        eprintln!(
            "{dropped} existing row(s) name a clause the standard's conversion no longer has. \
             They are kept; the gate will report them."
        );
        for row in &existing.rows {
            if generated.row(&row.clause).is_none() {
                generated.rows.push(row.clone());
            }
        }
    }

    if let Some(directory) = path.parent()
        && let Err(error) = std::fs::create_dir_all(directory)
    {
        eprintln!("cannot create {}: {error}", directory.display());
        return ExitCode::FAILURE;
    }
    if let Err(error) = std::fs::write(&path, generated.to_toml(PREAMBLE)) {
        eprintln!("cannot write {}: {error}", path.display());
        return ExitCode::FAILURE;
    }

    println!(
        "{}: {} rows, {added} new",
        path.display(),
        generated.rows.len()
    );
    for (status, count) in generated.counts() {
        if count > 0 {
            println!("  {status:<13} {count}");
        }
    }
    ExitCode::SUCCESS
}

/// A row for a subclause nobody has recorded yet.
///
/// Clause 13 is the one exception, and it is generated rather than left to be filled in:
/// `CLAUDE.md` principle 5 excludes multimedia and 3D by name, so every one of its 81
/// subclauses is `out-of-scope` from the start, carrying the exclusion that covers it. An
/// exclusion that is invisible is indistinguishable from an oversight, which is why those
/// rows are written out rather than omitted.
fn new_row(clause: conformance::clause::ClauseNumber, title: String) -> Row {
    if clause.clause() == Some(13) {
        return Row {
            status: Status::OutOfScope,
            exclusion: Some(Exclusion::Multimedia),
            ..Row::unreviewed(clause, title)
        };
    }
    Row::unreviewed(clause, title)
}
