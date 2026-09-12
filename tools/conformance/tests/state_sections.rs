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
//!
//! # And the other population: every `#[ignore]`d test file is in the sequence or says why not
//!
//! The direction review (`doc/reviews/984-direction-and-boundaries.md`, Finding 3) matched every
//! test file carrying `#[ignore]` against §2 and found seven in no gate line — among them the
//! validator's walk over the veraPDF corpus and the converter's walk over the same corpus, which
//! is the instrument that found ADR 1006's lying signature, on a merge, because nothing ran it
//! per round. A walk that is `#[ignore]`d is invisible to `cargo nextest run --workspace`, so the
//! sequence is the only thing that runs it, and a walk in neither is run when somebody remembers.
//! ADR 1005 §3 proposed the check and ADR 1015 built it.
//!
//! The second test derives that population from the index — every tracked `.rs` under `crates/`
//! and `tools/` with an `#[ignore` attribute at the start of a line — and fails, by name, on a
//! file of the shape a gate line can name (`<members>/<package>/tests/<target>.rs`) that no §2
//! line names and that carries no line beginning `// not a gate:` with its reason. It fails the
//! other way too: an excuse in a file the sequence does name has outlived its fact. Two things it
//! prints rather than fails, each for a stated reason in its message: an ignored test in a file no
//! `--test` line can name (a unit test under `src/`, run by `--lib -- --ignored`), and an
//! *untracked* test file carrying the attribute — that is somebody's mid-edit, and it will fail
//! this check the day it is added, which is the notice the print gives.
//!
//! The review's own count was one too many, and the reason is written into the matcher:
//! `viewer-confined --test confined` was counted on the evidence of a doc comment reading "[i]t
//! was `#[ignore]`d when it was written" — the attribute is gone since ADR 0888 and only the
//! sentence remains. A grep over source for a name finds comments (trap 11's sixth instance), so
//! the attribute is matched where an attribute is, at the start of a line.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a gate that cannot read its own repository has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

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

/// The attribute that keeps a test out of the ordinary workspace run.
///
/// Spelled in two pieces so that this file, which is in the population it scans, does not carry
/// what it looks for (`doc/habits/tests-gates-and-reports.md`: a scanner reads its own source).
/// Matched at the start of a trimmed line, which is where an attribute is and where a sentence
/// about one is not — see the module comment for the file that sentence miscounted.
const IGNORED: &str = concat!("#[", "ignore");

/// The line a `#[ignore]`d test file carries when it is deliberately in no gate line, with the
/// reason after the colon. The same shape as `sandbox_gates.rs`'s `// no sandbox worker:`, and
/// for the same reason: a forgetting and a decision look identical from outside the file.
const EXCUSE: &str = concat!("// not a gate", ":");

/// Every `.rs` file under `crates/` and `tools/` the index knows, tracked and — separately —
/// untracked but not ignored, as paths relative to the repository root.
///
/// From git rather than a directory walk, for `workspaces.rs`'s reason: a worktree round's
/// submodules and corpus cache are symlinks into the primary checkout. The untracked list is
/// asked for on its own so that the test can tell a file somebody is still writing from one the
/// tree holds, and treat the two differently.
fn sources(root: &Path, untracked: bool) -> Vec<String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).arg("ls-files");
    if untracked {
        command.args(["--others", "--exclude-standard"]);
    }
    let output = command
        .args(["--", "crates/*.rs", "tools/*.rs"])
        .output()
        .expect("git is on the path wherever this workspace builds");
    assert!(
        output.status.success(),
        "the index could not be listed, so this gate cannot say anything"
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The `package --test target` a gate line would use to name `path`, if it has the shape one
/// can name: `<crates|tools>/<package>/tests/<target>.rs` and nothing deeper. A module under
/// `tests/support/` and a unit test under `src/` are `None` — no `--test` line reaches them.
fn gate_name(path: &str) -> Option<String> {
    let mut parts = path.split('/');
    let (members, package, tests, file) =
        (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || !matches!(members, "crates" | "tools") || tests != "tests" {
        return None;
    }
    let target = file.strip_suffix(".rs")?;
    Some(format!("{package} --test {target}"))
}

/// Whether `text` carries the attribute, and — if so — the first reason it gives, for the
/// messages below. `#[cfg_attr(miri, ignore = …)]` starts with `#[cfg_attr` and is not counted:
/// that test runs in the workspace line and is skipped only under miri.
fn ignored_reason(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| line.starts_with(IGNORED))
        .map(|line| {
            line.split_once('"')
                .and_then(|(_, rest)| rest.split_once('"'))
                .map_or_else(
                    || "(no reason given)".to_owned(),
                    |(reason, _)| reason.to_owned(),
                )
        })
}

#[test]
fn every_ignored_test_file_is_named_by_the_sequence_or_says_why_it_is_not() {
    let root = repository_root();
    let sequence = gate_lines(
        &std::fs::read_to_string(root.join("doc/todo/02-every-round.md"))
            .expect("doc/todo/02 is this gate's population"),
    );
    // This file is in the population and carries neither marker as an attribute; skipped by name
    // all the same, so that the statement above stays a fact a reader can check here rather than
    // one that depends on how the constants happen to be spelled.
    let this_file = "tools/conformance/tests/state_sections.rs";

    let mut population = 0_usize;
    let mut named = 0_usize;
    let mut unnamed: Vec<String> = Vec::new();
    let mut spent: Vec<String> = Vec::new();
    for path in sources(root, false) {
        if path == this_file {
            continue;
        }
        let text = std::fs::read_to_string(root.join(&path))
            .unwrap_or_else(|why| panic!("{path} is tracked and could not be read: {why}"));
        let Some(reason) = ignored_reason(&text) else {
            continue;
        };
        population = population.saturating_add(1);
        let excuse = text
            .lines()
            .map(str::trim)
            .find_map(|line| line.strip_prefix(EXCUSE))
            .map(str::trim);
        let in_sequence = gate_name(&path).is_some_and(|name| sequence.contains(&name));
        match (in_sequence, excuse, gate_name(&path)) {
            (true, Some(excuse), _) => spent.push(format!("{path}: {excuse}")),
            (true, None, _) => named = named.saturating_add(1),
            (false, Some(excuse), _) => println!("not a gate, and says why: {path} — {excuse}"),
            (false, None, Some(name)) => unnamed.push(format!(
                "{path} (`-p {name}`, ignored because \"{reason}\")"
            )),
            (false, None, None) => println!(
                "{path} carries an ignored test no `--test` line can name (\"{reason}\"); it runs \
                 only under `--lib -- --ignored`, and the `{EXCUSE}` line saying why it is not a \
                 gate is owed there too — printed rather than failed until the one instance \
                 carries it"
            ),
        }
    }
    for path in sources(root, true) {
        let Ok(text) = std::fs::read_to_string(root.join(&path)) else {
            continue;
        };
        if let Some(reason) = ignored_reason(&text) {
            println!(
                "{path} is not tracked yet and carries an ignored test (\"{reason}\"); it will fail \
                 this check when it is added unless a doc/todo/02 §2 line names it or it carries \
                 a `{EXCUSE}` line"
            );
        }
    }

    assert!(
        population > 5 && named > 5,
        "{population} tracked test files carry `{IGNORED}` and {named} of them are named by \
         doc/todo/02 §2, so this check is measuring nothing — either the index listing or the \
         attribute match stopped working"
    );
    assert!(
        spent.is_empty(),
        "these files say they are not a gate and doc/todo/02 §2 names them, so the excuse has \
         outlived its fact — delete the `{EXCUSE}` line or the gate line: {spent:?}"
    );
    assert!(
        unnamed.is_empty(),
        "these `{IGNORED}`d test files are in no doc/todo/02 §2 gate line and carry no line \
         beginning `{EXCUSE}` — nothing but memory runs them (review 984, Finding 3). Add the gate \
         line, with a `tools/state.sh` section, or the excuse with a reason a reader can check: \
         {unnamed:?}"
    );
}
