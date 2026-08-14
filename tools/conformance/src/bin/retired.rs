//! Sweeps the ledger, the tree's comments and this project's prose for a claim a round retired.
//!
//! ```sh
//! cargo run --release -p conformance --bin retired -- knockout '/AIS' 'free text'
//! ```
//!
//! `doc/todo/01`'s fourth sweep — the string a correction retired, grepped over every other row,
//! over the tree's comments and over this project's own prose — and the sixth of the fifteen to
//! be a program rather than a description. [`conformance::retired`] says why its nouns come from
//! the caller rather than from the file, what the correction-versus-standing classification
//! settles, why a noun carrying both shapes is the defect's own signature, and which one
//! directory under `doc/` it does not read.
//!
//! It prints one block per noun, the both-shapes nouns first and each noun's corrections above
//! its standing mentions, and ends with the counts so that a clean run says what it was clean
//! over. It exits non-zero only where it cannot read what it needs, or where it was given no
//! noun to look for: a retired claim is a question for a person, not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use conformance::entries;
use conformance::ledger::Ledger;
use conformance::retired::{self, Found, Kind};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("retired: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Why the sweep could not be run at all.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// No noun was given, so there is nothing to look for.
    #[error(
        "give the nouns the last rounds' corrections were about, one argument each — \
         `cargo run --release -p conformance --bin retired -- knockout '/AIS' 'free text'`. \
         The sweep cannot derive them: what was retired is what those rounds decided."
    )]
    NoNouns,
    /// The ledger could not be read.
    #[error(transparent)]
    Ledger(#[from] conformance::ledger::LedgerError),
    /// The tree's sources could not be walked.
    #[error(transparent)]
    Sources(#[from] entries::Error),
    /// The ADRs could not be walked.
    #[error(transparent)]
    Documents(#[from] conformance::prose::Error),
    /// A document could not be read.
    #[error("{path}: {source}")]
    Unreadable {
        /// The file.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
}

fn run() -> Result<(), Error> {
    let nouns: Vec<String> = std::env::args().skip(1).collect();
    if nouns.is_empty() {
        return Err(Error::NoNouns);
    }
    let root = conformance::workspace_root();
    let ledger = Ledger::read(&root.join(conformance::LEDGER))?;
    let sources = entries::sources(&root)?;

    let mut documents: Vec<(PathBuf, String)> = Vec::new();
    for path in conformance::prose::documents(&root.join("doc"))? {
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Unreadable {
            path: path.display().to_string(),
            source,
        })?;
        documents.push((
            path.strip_prefix(&root).unwrap_or(&path).to_path_buf(),
            text,
        ));
    }

    let mut found = retired::sweep(&nouns, &ledger, &sources, &documents);
    found.sort_by_key(|noun| !noun.both_shapes());

    let mut mentions = 0usize;
    let mut both = 0usize;
    for noun in &found {
        mentions = mentions.saturating_add(noun.mentions.len());
        if noun.both_shapes() {
            both = both.saturating_add(1);
        }
        print(noun);
    }
    println!(
        "{} noun(s), {mentions} mention(s), {both} carrying both shapes — a correction and a \
         standing claim about one mechanism, which is where this sweep's findings have always \
         been.",
        found.len()
    );
    println!(
        "A mention is a reading list rather than a verdict. A correction quotes the wording it \
         retired, which is this sweep's oldest false positive and is why the quotation is \
         printed; a standing mention is usually an ordinary true sentence, and one short noun \
         lands in three clauses that mean three different things. Read the sentence before \
         believing a hit."
    );
    Ok(())
}

/// Prints one noun: its counts, then its mentions, corrections first.
fn print(found: &Found) {
    let shape = if found.both_shapes() {
        " — BOTH SHAPES"
    } else {
        ""
    };
    println!(
        "`{}`: {} mention(s), {} correction(s), {} standing{shape}",
        found.noun,
        found.mentions.len(),
        found.corrections(),
        found.standing(),
    );
    for mention in &found.mentions {
        let mark = if mention.kind == Kind::Correction {
            "[correction]"
        } else {
            "[standing]  "
        };
        println!("    {mark} {}", mention.location);
        println!("        {}", mention.sentence);
    }
    println!();
}
