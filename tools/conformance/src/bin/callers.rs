//! Sweeps every `pub fn` in `pdf-model` against everything in this tree that could ask it.
//!
//! ```sh
//! cargo run --release -p conformance --bin callers
//! cargo run --release -p conformance --bin callers -- pdf-syntax
//! ```
//!
//! `doc/todo/01`'s fifth sweep — "who calls it?" — and the seventh of the fifteen to be a program
//! rather than a description. [`conformance::callers`] says what the rungs are, why the consumers
//! come from the manifests rather than from a list typed into a script, and which two directions
//! the name matching is loose in.
//!
//! It prints the rungs from the bottom, because the bottom is where the findings have been:
//! `Collection::initial_document`, which no host could call; `ViewState::clear_field`, which
//! nothing named at all. It exits non-zero only where it cannot read what it needs — an unreached
//! `pub fn` is a reading list and not a build failure.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::process::ExitCode;

use conformance::callers::{self, Reach};
use conformance::entries;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("callers: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Why the sweep could not be run at all.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// The tree's sources could not be walked.
    #[error(transparent)]
    Sources(#[from] entries::Error),
    /// The workspace's manifests could not be read.
    #[error("cannot read the workspace's manifests: {0}")]
    Manifests(#[from] std::io::Error),
    /// No crate of this workspace has that name.
    #[error(
        "no crate of this workspace is called {0} — give a directory name under crates/ or tools/"
    )]
    NoSuchCrate(String),
}

/// The rungs, in the order the report reads them: the finding has always been at the bottom.
const RUNGS: [Reach; 4] = [
    Reach::Nothing,
    Reach::TestOrExample,
    Reach::Model,
    Reach::ToolOrFuzz,
];

fn run() -> Result<(), Error> {
    let crate_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| callers::ANSWERING.to_owned());
    let root = conformance::workspace_root();
    let sources = entries::sources(&root)?;
    let consumers = callers::consumers(&root, &crate_name)?;
    // Where the crate lives is looked up rather than assumed: a sweep of a *tool*'s public
    // surface is the same question, and a hard-coded `crates/` would answer it with an empty
    // population and no error.
    let answering = callers::directory_of(&root, &crate_name)
        .ok_or_else(|| Error::NoSuchCrate(crate_name.clone()))?;
    let report = callers::sweep(&answering, &sources, &consumers);

    println!(
        "{crate_name}: {} distinct `pub fn` name(s); {} crate(s) name it in a manifest.",
        report.functions.len(),
        report.consumers.len()
    );
    for consumer in &report.consumers {
        println!("    {} ({:?})", consumer.directory, consumer.dependency);
    }
    println!();

    for rung in RUNGS {
        println!("{} — {}:", rung, report.on(rung));
        for function in report.functions.iter().filter(|one| one.reach == rung) {
            let declared = if function.declared.is_empty() {
                function.name.clone()
            } else {
                function.declared.join(", ")
            };
            match function.witness.as_deref() {
                Some(witness) => println!("    {declared}   <- {witness}"),
                None => println!("    {declared}"),
            }
        }
        println!();
    }
    println!(
        "{} name(s) no crate under `crates/` asks; {} named by a dependent crate.",
        report.unasked_by_a_crate(),
        report.on(Reach::Dependent)
    );
    println!(
        "The delta is what this sweep produces, not the level: a whole new host program taking \
         no name off the bottom rungs is the strongest evidence there is that the entry points \
         are the answers. A short name shared with another type's method reads as named, and a \
         name reached through a wrapper reads as unnamed. Read the hit before believing it."
    );
    Ok(())
}
