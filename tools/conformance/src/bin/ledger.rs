//! Counts `doc/conformance/ledger.toml`'s statuses, and — only when asked to — regenerates it
//! without losing what is in it.
//!
//! **Run without arguments it writes nothing.** It reads the standard's clause index and the
//! ledger, builds the file the generator would write, prints the status counts of that file and
//! says whether the one on disk is already in that form. `tools/state.sh ledger` runs this mode:
//! a command that counts may not write, because six rounds edit this file at once and a counting
//! run that rewrote it — reordering nothing a person changed, but writing the whole file — is a
//! write nobody asked for, which the next fast-forward then refuses (ADR 1487).
//!
//! **`--write` is the generator.** The ledger is generated once and then edited by people, and
//! this program exists so that the *set of rows* is never edited by hand. With `--write` it keeps
//! every existing row exactly as written, adds an `unreviewed` row for any subclause that has
//! none, refreshes each row's title from the standard, and writes the file back in clause order.
//! A round that edits the ledger and wants it in generated form runs it, alone, on its own tree.
//!
//! A row is never deleted by this program. If the standard's conversion loses a heading, the
//! row for it stays and the gate reports it as naming a clause that does not exist — which
//! is a finding about the conversion, and not something to fix by dropping the row.
//!
//! ```text
//! cargo run -p conformance --bin ledger              # counts; writes nothing
//! cargo run -p conformance --bin ledger -- --write   # regenerates the rows, keeps every status
//! ```

#![forbid(unsafe_code)]
#![expect(
    clippy::print_stdout,
    reason = "a command-line program whose whole output is a report on what it read or wrote"
)]

use std::path::Path;
use std::process::ExitCode;

use conformance::clause::ClauseIndex;
use conformance::ledger::{
    Exclusion, HoldingCounts, Ledger, NORMATIVE_ANNEXES, NORMATIVE_CLAUSES, Row, Status,
    TestClassifier,
};

/// Written above the rows, as `#` comments, every time the file is generated.
const PREAMBLE: &str = "\
The conformance ledger: one row per subclause of ISO 32000-2's normative clauses — 6 to 14,
and the eight normative annexes. What decides that population is not this list but the
standard: `cargo test -p conformance` reports a clause that states `shall` and is covered by
neither the population nor an argued exclusion in `conformance::ledger`.

GENERATED — the set of rows is. Their statuses are not: a status is a claim a person makes
after reading the clause against this code, and `cargo run -p conformance --bin ledger`
preserves every one of them. It only ever adds rows the standard has and this file lacks.

Statuses, and what each is for. The vocabulary exists to keep six situations from wearing
one word: the project choosing for a whole clause, the project choosing for one sentence
inside a clause otherwise executed, the project not knowing, the project owing out loud, the
project owing in silence, and the requirement having no meaning for a screen.

  implemented   every normative requirement in the clause is executed; names code and test
  partial       some are; the note says which, which are not, and what is reported
  departed      every requirement of the clause is executed except the one the note names,
                which was decided against with its cost recorded. Nothing is owed
  reported      not implemented yet, but detected and reported at runtime. Still owed
  silent        not implemented, and nothing says so. A page is drawn wrong without a word
  inapplicable  the requirement has no meaning for this device. Nothing is owed
  writer-side   addresses a generator; this program's writers emit structure, never content
  out-of-scope  covered by CLAUDE.md principle 5's closed exclusion list, which the row names
  unreviewed    nobody has read this clause against this code

`unreviewed` is debt, not absence: it says only that the question has not been asked.
`departed` is the owner's word, given 2026-09-14 in answer to doc/questions/Q63, and its row
names the ADR that decided the departure; the count is printed beside `implemented` and
`partial` and is never folded into either, so a decision stays visible as a decision.
`silent` is the status worth hunting. Every missing subsystem here reports, because whoever
decided not to build it wrote the report; what ships is the gap inside a feature that is
already there, and only reading the clause finds one.

See doc/PLAN.md section 5a for the design, and tools/conformance for the checker that reads this.";

fn main() -> ExitCode {
    let write = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => false,
        [flag] if flag == "--write" => true,
        _ => {
            eprintln!(
                "usage: ledger [--write]\n  without arguments it counts and writes nothing; \
                 --write regenerates doc/conformance/ledger.toml"
            );
            return ExitCode::from(2);
        }
    };
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

    let mut clauses: Vec<_> = NORMATIVE_CLAUSES
        .into_iter()
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

    let text = generated.to_toml(PREAMBLE);
    let outcome = if write {
        write_ledger(&path, &text)
    } else {
        compare_ledger(&path, &text)
    };
    match outcome {
        Ok(state) => println!(
            "{}: {} rows, {added} {state}",
            path.display(),
            generated.rows.len()
        ),
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    }
    for (status, count) in generated.counts() {
        if count > 0 {
            println!("  {status:<13} {count}");
        }
    }
    print_holdings(&root, &generated);
    ExitCode::SUCCESS
}

/// What holds each row, beside how many there are: a fixture whose expected value the clause
/// derives, only the robustness instrument — corpus witnesses, ignored walks, censuses — or no
/// test at all. ADR 1497.
fn print_holdings(root: &Path, ledger: &Ledger) {
    println!("  held by: a fixture / only walks or corpus witnesses / no test");
    let mut classifier = TestClassifier::new(root);
    for (status, held) in classifier.holdings(ledger) {
        if held != HoldingCounts::default() {
            println!(
                "  {status:<13} {} / {} / {}",
                held.fixture, held.only_walks, held.empty
            );
        }
    }
}

/// Writes the generated ledger over `path`: the one thing this program does only under `--write`.
fn write_ledger(path: &Path, text: &str) -> Result<String, String> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    }
    std::fs::write(path, text)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok("new, written".to_owned())
}

/// Says whether the ledger at `path` is already the generated `text`, and writes nothing.
///
/// A ledger that does not exist yet reads as empty: every row is then one the standard has and
/// the file lacks, which is the true answer about a missing file.
fn compare_ledger(path: &Path, text: &str) -> Result<String, String> {
    let on_disk = if path.exists() {
        std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?
    } else {
        String::new()
    };
    Ok(match first_difference(&on_disk, text) {
        None => {
            "the standard has and the file lacks; in its generated form; nothing written".to_owned()
        }
        Some(line) => format!(
            "the standard has and the file lacks; differs from its generated form from line \
             {line}; nothing written (`--write` regenerates it, run by a round editing the ledger)"
        ),
    })
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

/// The first line, counted from 1, at which `on_disk` and `generated` differ, if they do.
fn first_difference(on_disk: &str, generated: &str) -> Option<usize> {
    if on_disk == generated {
        return None;
    }
    let mut ours = on_disk.lines();
    let mut theirs = generated.lines();
    let mut line = 1usize;
    loop {
        match (ours.next(), theirs.next()) {
            (Some(a), Some(b)) if a == b => line = line.saturating_add(1),
            _ => return Some(line),
        }
    }
}
