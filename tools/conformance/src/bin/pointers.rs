//! Sweeps the ledger, the tree's comments and this project's prose for a pointer into this tree.
//!
//! ```sh
//! cargo run --release -p conformance --bin pointers
//! ```
//!
//! `doc/todo/01`'s eighth sweep — does the file a note names still exist, and does the file hold
//! the item the other half of the pointer names — and the eighth of the fifteen to be a program
//! rather than a description. [`conformance::pointers`] says why a pointer is resolved from where
//! it is written, which three shapes get a rung of their own instead of a place in the reading
//! list, and why a dead pointer is a question for a person rather than a build failure.
//!
//! It prints the absent pointers first, standing claims above corrections, then the symbol
//! pointers no file defines, then the counts on every rung — so that a clean run says what it was
//! clean over. It exits non-zero only where it cannot read what it needs.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use conformance::entries;
use conformance::ledger::Ledger;
use conformance::pointers::{self, Pointer, Reach, Symbol, Tree};
use conformance::retired::Kind;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pointers: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Why the sweep could not be run at all.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// The ledger could not be read.
    #[error(transparent)]
    Ledger(#[from] conformance::ledger::LedgerError),
    /// The tree's sources could not be walked.
    #[error(transparent)]
    Sources(#[from] entries::Error),
    /// The documents could not be walked.
    #[error(transparent)]
    Documents(#[from] conformance::prose::Error),
    /// The workspace could not be walked.
    #[error("the workspace could not be walked: {0}")]
    Tree(#[from] std::io::Error),
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
    let root = conformance::workspace_root();
    let tree = Tree::walk(&root)?;
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

    let found = pointers::sweep(&tree, &ledger, &sources, &documents);

    let mut absent = found.reaching(Reach::Absent);
    absent.sort_by_key(|pointer| pointer.kind == Kind::Correction);
    println!("Absent — nothing at the path, resolved from where it is written:");
    for pointer in &absent {
        print(pointer);
    }
    if absent.is_empty() {
        println!("    none.");
    }

    let undefined = found.undefined();
    println!();
    println!("Undefined — the file is there and the item it names is not:");
    for symbol in &undefined {
        print_symbol(symbol);
    }
    if undefined.is_empty() {
        println!("    none.");
    }

    println!();
    println!(
        "{} path pointer(s): {} live, {} absent, {} in another crate, {} unrooted, {} a form, \
         {} not carried. {} symbol pointer(s), {} undefined.",
        found.pointers.len(),
        found.reaching(Reach::Live).len(),
        absent.len(),
        found.reaching(Reach::AnotherCrate).len(),
        found.reaching(Reach::Unrooted).len(),
        found.reaching(Reach::Placeholder).len(),
        found.reaching(Reach::NotCarried).len(),
        found.symbols.len(),
        undefined.len(),
    );
    println!(
        "An absent pointer is a reading list rather than a verdict. A correction quotes the \
         pointer it retired, which is this sweep's oldest false positive and is why it is marked \
         rather than dropped; a round may also cite the file it is about to add. Read the \
         sentence before believing a hit."
    );
    Ok(())
}

/// Prints one absent pointer: where it is, what it says, and the sentence it says it in.
fn print(pointer: &Pointer) {
    let mark = if pointer.kind == Kind::Correction {
        "[correction]"
    } else {
        "[standing]  "
    };
    println!("    {mark} `{}` — {}", pointer.text, pointer.location);
    println!("        {}", pointer.sentence);
}

/// Prints one symbol pointer whose item nothing defines, with the files that could have.
fn print_symbol(symbol: &Symbol) {
    let mark = if symbol.kind == Kind::Correction {
        "[correction]"
    } else {
        "[standing]  "
    };
    let files = if symbol.files.is_empty() {
        "no file of that name".to_owned()
    } else {
        symbol.files.join(", ")
    };
    println!("    {mark} `{}` — {}", symbol.text, symbol.location);
    println!("        looked in: {files}");
    println!("        {}", symbol.sentence);
}
