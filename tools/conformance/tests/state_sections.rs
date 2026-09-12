//! Every gate line `doc/todo/02` §2 states is a line `tools/state.sh` runs.
//!
//! Not a conformance question; it lives here for `sandbox_gates.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! `doc/todo/02` §2 says of itself that it **owns** the gate sequence and nothing else states it,
//! and says of `tools/state.sh` that it "runs the same sequence and prints each gate's own summary
//! lines". Both are true only while somebody keeps them true: the sequence is a fenced block in a
//! document and the script is a list of shell functions, each written by hand, and a line added to
//! one and not the other reads as a gate that ran — `state.sh` prints a heading per section and a
//! round reading its output cannot see the section that is not there. That is the shape ADR 0232 §4
//! records for the *two copies* this document used to have of its own sequence, which drifted by
//! two tests and one whole gate before anybody compared them.
//!
//! The nine-hundred-and-ninetieth session measured the two by hand and found them equal — and
//! then made them unequal, deliberately, by giving the save round-trip a `state.sh` section
//! before its §2 line existed (ADR 1011). So the check is one-directional where it fails and
//! two-directional where it prints: a §2 line the script does not run **fails**, because the
//! document's claim about the script is false; a script section §2 does not list is **printed**,
//! because an instrument may honestly run ahead of the sequence while its numbers are watched.
//!
//! # Why the population is derived
//!
//! Both lists are read out of the files rather than restated here, which is the same rule
//! `sandbox_gates.rs` follows: a list in this test would be a third copy of the sequence.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::collections::BTreeSet;
use std::path::Path;

/// Where the repository root is, relative to this crate's manifest.
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// Every `cargo test … -p <package> --test <target>` line in `text`, as `package --test target`.
///
/// The same parse `sandbox_gates.rs` makes of the document, applied to the script as well: a
/// gate line is recognised by its two flags and nothing else, so neither the profile nor the
/// arguments after `--` can make the two copies look different when they are not.
fn gate_lines(text: &str) -> BTreeSet<String> {
    // Only lines inside a fenced ```sh block are commands. `doc/todo/02` §2 also *mentions* gate
    // lines in prose and in the map's table cells — `` `cargo test … --test gate`, `` with the
    // backtick and comma still attached — and a scrape over every line took those for commands
    // and named a gate the script runs as one it does not.
    // A shell script has no fences and every `cargo test` in it is a command; a Markdown file
    // has them, and only what is inside one is.
    let has_fences = text.contains("```");
    let mut found = BTreeSet::new();
    let mut fenced = !has_fences;
    for line in text.lines() {
        let line = line.trim();
        if has_fences && line.starts_with("```") {
            fenced = line.starts_with("```sh") || line.starts_with("```bash");
            continue;
        }
        if !fenced || !line.contains("cargo test") {
            continue;
        }
        let mut fields = line.split_whitespace();
        let mut package = None;
        let mut target = None;
        while let Some(field) = fields.next() {
            match field {
                "-p" => package = fields.next().map(str::to_owned),
                "--test" => target = fields.next().map(str::to_owned),
                _ => {}
            }
        }
        if let (Some(package), Some(target)) = (package, target) {
            found.insert(format!("{package} --test {target}"));
        }
    }
    found
}

#[test]
fn every_gate_line_the_sequence_states_is_one_the_state_script_runs() {
    let root = repository_root();
    let read = |path: &str| {
        std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|why| panic!("{path} is this gate's population: {why}"))
    };
    let sequence = gate_lines(&read("doc/todo/02-every-round.md"));
    let script = gate_lines(&read("tools/state.sh"));

    assert!(
        sequence.len() > 5 && script.len() > 5,
        "doc/todo/02 §2 yielded {} gate lines and tools/state.sh {}, so this check is measuring \
         nothing — either a file moved or the parse stopped working",
        sequence.len(),
        script.len()
    );

    let not_run: Vec<&String> = sequence.difference(&script).collect();
    let not_listed: Vec<&String> = script.difference(&sequence).collect();
    for line in &not_listed {
        println!("tools/state.sh runs `{line}`, which doc/todo/02 §2 does not list yet");
    }
    assert!(
        not_run.is_empty(),
        "doc/todo/02 §2 states these gate lines and tools/state.sh runs no section for them, so \
         its claim to run the same sequence is false — add the section or delete the line: \
         {not_run:?}"
    );
}
