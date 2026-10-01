//! A command line a live document writes names targets this tree has.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! A document's `cargo run -p … --example …` is a command a round copies, and a round copies it
//! without reading the tree first. When the crate is renamed, the example deleted or the profile
//! moved, cargo refuses the copied line or — worse — builds a profile the tree stopped
//! prescribing and spends minutes nobody asked for: the conformance sweeps left `--release` under
//! ADR 1463 and five documents went on writing it. `variables.rs` is the same question asked of an
//! environment variable; this is it asked of the rest of the line (ADR 1475).
//!
//! # What a command names, and what each must resolve to
//!
//! A command is `cargo` followed by `run`, `test`, `nextest run`, `bench`, `build`, `check` or
//! `clippy`, read up to the end of its code span, its line (a trailing `\` continues it), a shell
//! separator, a comment or the ` -- ` that hands the rest to the program. In it:
//!
//! - `-p`/`--package` names a workspace member, by the manifest's own `name`;
//! - `--example`, `--test` and `--bin` name a target of that package — or of any member when no
//!   package is named, which is how cargo resolves them in a virtual workspace — by cargo's own
//!   rules: a file under `examples/`, `tests/` or `src/bin/`, a directory there holding
//!   `main.rs`, a declared `[[example]]`, `[[test]]` or `[[bin]]`, or the package's own name
//!   for a `src/main.rs` no declared binary claims;
//! - `--release` on a `run`, `test` or `bench` line, beside a package `doc/verify.md` runs only
//!   under the default profile, is a finding, because `doc/verify.md` is where a command's profile
//!   is prescribed; a `build` keeps its profile, which is the binary's purpose (ADR 1475).
//!
//! And anywhere in a live document, a `tools/<name>.sh` or `tools/<name>.py` names a script the
//! tree carries.
//!
//! A name holding `<`, `$`, `{`, `…`, `*` or written in capitals is a form, not a citation.
//!
//! # The population
//!
//! `variables.rs`'s exactly: `CLAUDE.md`, `doc/` less its records and the owner's
//! `doc/questions/`, `raster/`'s own `CLAUDE.md` and `doc/HANDOVER.md`, and the scripts and prose
//! under `tools/`. A trap or a habit is an incident record, but the command in it is an
//! instruction a reader runs today, so it is in the population while its story is not rewritten
//! (ADR 1475). [`ADDRESSED_ELSEWHERE`] is the one exception, and it says why.
//!
//! # Why a zero, and the calibration
//!
//! There is no version of this tree in which a live document may hand a reader a command that
//! names a target the tree does not have. The second test plants a missing example, a retired
//! package, a dev-only `--release`, a form and a `build --release` of the dev-only package into the
//! sweep's own functions and fails unless the first three alone are found (trap 13).

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: the gate prints its denominators and its findings, so a failure and a \
              clean run are read the same way, and a gate that cannot read the workspace manifest \
              has not found a defect"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Directories no walk enters: build output, version control, and dependency checkouts.
const SKIPPED: [&str; 4] = ["target", ".git", "node_modules", "__pycache__"];

/// Live documents whose commands are written for another project's checkout.
///
/// `doc/JPEG2000_FEEDBACK.md` is a report to `hayro-jpeg2000`'s maintainer, and its commands run
/// in that crate's repository, where `-p hayro-jpeg2000` is the package.
const ADDRESSED_ELSEWHERE: [&str; 1] = ["doc/JPEG2000_FEEDBACK.md"];

/// The cargo subcommands whose lines name a package and its targets.
const SUBCOMMANDS: [&str; 7] = [
    "run", "test", "nextest", "bench", "build", "check", "clippy",
];

/// One workspace member's targets, as cargo would resolve them.
#[derive(Default)]
struct Targets {
    examples: BTreeSet<String>,
    tests: BTreeSet<String>,
    bins: BTreeSet<String>,
}

/// The names of every `[[section]]` table in a manifest, with its `path` where one is given.
fn declared(manifest: &str, section: &str) -> Vec<(String, Option<String>)> {
    let header = format!("[[{section}]]");
    let mut found = Vec::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == header;
            if inside {
                found.push((String::new(), None));
            }
            continue;
        }
        if !inside {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_owned();
        if let Some(last) = found.last_mut() {
            match key.trim() {
                "name" => last.0 = value,
                "path" => last.1 = Some(value),
                _ => {}
            }
        }
    }
    found
}

/// The stems of `dir`'s `.rs` files and of its subdirectories holding `main.rs`.
fn stems(dir: &Path) -> BTreeSet<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return BTreeSet::new();
    };
    entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter_map(|path| {
            if path.is_dir() {
                path.join("main.rs")
                    .is_file()
                    .then(|| path.file_name()?.to_str().map(str::to_owned))
                    .flatten()
            } else if path.extension().is_some_and(|e| e == "rs") {
                path.file_stem()?.to_str().map(str::to_owned)
            } else {
                None
            }
        })
        .collect()
}

/// The package name a manifest's `[package]` table states.
fn package_name(manifest: &str) -> Option<String> {
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[package]";
            continue;
        }
        if inside
            && let Some(rest) = line.strip_prefix("name")
            && let Some((_, value)) = rest.split_once('=')
        {
            return Some(value.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// Every workspace member by package name, with its targets.
fn workspace(root: &Path) -> BTreeMap<String, Targets> {
    let members = conformance::roots::members(root).expect("the workspace manifest states members");
    let mut packages = BTreeMap::new();
    for member in members {
        let dir = root.join(&member);
        let manifest = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
        let Some(name) = package_name(&manifest) else {
            continue;
        };
        let mut targets = Targets {
            examples: stems(&dir.join("examples")),
            tests: stems(&dir.join("tests")),
            bins: stems(&dir.join("src/bin")),
        };
        targets
            .examples
            .extend(declared(&manifest, "example").into_iter().map(|(n, _)| n));
        targets
            .tests
            .extend(declared(&manifest, "test").into_iter().map(|(n, _)| n));
        let bins = declared(&manifest, "bin");
        let main_claimed = bins
            .iter()
            .any(|(_, path)| path.as_deref() == Some("src/main.rs"));
        if dir.join("src/main.rs").is_file() && !main_claimed {
            targets.bins.insert(name.clone());
        }
        targets.bins.extend(bins.into_iter().map(|(n, _)| n));
        packages.insert(name, targets);
    }
    packages
}

/// What one command line names.
#[derive(Debug, Default, PartialEq, Eq)]
struct Command {
    /// Whether the subcommand runs something — `run`, `test`, `nextest run`, `bench` — which is
    /// what `doc/verify.md` prescribes a profile for; a `build` or a `check` is asked for a
    /// binary or a target and takes the profile its purpose needs.
    runs: bool,
    packages: Vec<String>,
    examples: Vec<String>,
    tests: Vec<String>,
    bins: Vec<String>,
    release: bool,
}

/// Whether a name is a form a reader fills in rather than a citation.
fn is_a_form(name: &str) -> bool {
    name.is_empty()
        || name.contains(['<', '>', '$', '{', '…', '*', '['])
        || name.contains("...")
        || name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c == '-')
}

/// The command starting at `cargo`, cut where its code span, shell separator, comment or program
/// arguments begin.
fn command_text(from_cargo: &str) -> &str {
    let mut end = from_cargo.len();
    for stop in ["`", "|", ";", "&&", " -- ", " #", ")"] {
        if let Some(at) = from_cargo.find(stop) {
            end = end.min(at);
        }
    }
    &from_cargo[..end]
}

/// How many following lines a command may run onto: a code span wrapped by the prose around it,
/// or a shell line continued with `\`.
const CONTINUATION: usize = 3;

/// Every command in `text`, a whole document, with the line each begins on.
///
/// A command runs onto the next line while it sits in a code span that has not closed — prose
/// wraps at its width, not at a command's end — or while its line ends in `\`.
fn commands_in(text: &str) -> Vec<(usize, Command)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let mut from = 0;
        while let Some(at) = line[from..].find("cargo") {
            let start = from.saturating_add(at);
            from = start.saturating_add("cargo".len());
            let before = line[..start].chars().next_back();
            let after = line[from..].chars().next();
            if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '/')
                || after.is_some_and(|c| c != ' ')
            {
                continue;
            }
            let in_span = line[..start].matches('`').count() % 2 == 1;
            let mut joined = line[start..].to_owned();
            for more in lines
                .iter()
                .skip(index.saturating_add(1))
                .take(CONTINUATION)
            {
                let open_span = in_span && !joined.contains('`');
                let continued = joined.trim_end().ends_with('\\');
                if !open_span && !continued {
                    break;
                }
                joined = format!(
                    "{} {}",
                    joined.trim_end().trim_end_matches('\\'),
                    more.trim()
                );
            }
            if let Some(command) = parse(command_text(&joined)) {
                found.push((index.saturating_add(1), command));
            }
        }
    }
    found
}

/// What a command's text names, or `None` when it is not a subcommand this sweep reads.
fn parse(text: &str) -> Option<Command> {
    let mut words = text
        .split_whitespace()
        .map(|word| {
            let quotes = |c: char| c == '\'' || c == '"' || c == ',' || c == '.';
            word.trim_matches(quotes)
                .trim_end_matches("\\n")
                .trim_matches(quotes)
        })
        .skip(1)
        .skip_while(|word| word.starts_with('+'))
        .peekable();
    let subcommand = words.next()?;
    if !SUBCOMMANDS.contains(&subcommand) {
        return None;
    }
    let mut command = Command {
        runs: matches!(subcommand, "run" | "test" | "nextest" | "bench"),
        ..Command::default()
    };
    let words: Vec<&str> = words.collect();
    let mut index = 0;
    while let Some(word) = words.get(index) {
        let value = words.get(index.saturating_add(1)).map(|v| (*v).to_owned());
        let (flag, inline) = word
            .split_once('=')
            .map_or((*word, None), |(f, v)| (f, Some(v.to_owned())));
        let slot = match flag {
            "-p" | "--package" => Some(&mut command.packages),
            "--example" => Some(&mut command.examples),
            "--test" => Some(&mut command.tests),
            "--bin" => Some(&mut command.bins),
            "--release" => {
                command.release = true;
                None
            }
            _ => None,
        };
        if let Some(slot) = slot {
            let name = inline.or(value).unwrap_or_default();
            if !is_a_form(&name) && !name.starts_with('-') {
                slot.push(name);
            }
            if word.contains('=') {
                index = index.saturating_add(1);
            } else {
                index = index.saturating_add(2);
            }
        } else {
            index = index.saturating_add(1);
        }
    }
    Some(command)
}

/// Every file under `dir` whose extension is one of `extensions`, recursively.
fn files(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        if path.is_dir() {
            if !SKIPPED.contains(&name.as_str()) {
                files(&path, extensions, out);
            }
        } else if path
            .extension()
            .is_some_and(|e| extensions.contains(&&*e.to_string_lossy()))
        {
            out.push(path);
        }
    }
}

/// Whether a document is outside the population, by its path relative to the workspace root.
fn is_outside(relative: &str) -> bool {
    conformance::pointers::RECORDS
        .iter()
        .any(|prefix| relative.starts_with(prefix))
        || conformance::prose::RECORDS.iter().any(|record| {
            relative
                .strip_prefix("doc/")
                .is_some_and(|rest| rest.starts_with(record))
        })
        || relative.starts_with("doc/questions/")
        || ADDRESSED_ELSEWHERE.contains(&relative)
}

/// The live documents, relative to `root`.
fn documents(root: &Path) -> Vec<(String, String)> {
    let mut paths = vec![
        root.join("CLAUDE.md"),
        root.join("raster/CLAUDE.md"),
        root.join("raster/doc/HANDOVER.md"),
    ];
    files(&root.join("doc"), &["md"], &mut paths);
    files(&root.join("tools"), &["sh", "py", "md", "toml"], &mut paths);
    paths
        .into_iter()
        .filter_map(|path| {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            if is_outside(&relative) {
                return None;
            }
            Some((relative, fs::read_to_string(&path).ok()?))
        })
        .collect()
}

/// The packages `doc/verify.md` runs only under the default profile — in a `run`, `test` or
/// `bench` line, never with `--release` or `--profile`.
fn dev_only(verify: &str) -> BTreeSet<String> {
    let mut dev = BTreeSet::new();
    let mut other = BTreeSet::new();
    for line in verify.lines() {
        let Some(at) = line.find("cargo ") else {
            continue;
        };
        let text = command_text(&line[at..]);
        let Some(command) = parse(text) else {
            continue;
        };
        if !command.runs {
            continue;
        }
        let profiled = command.release || text.contains("--profile");
        for package in command.packages {
            if profiled {
                other.insert(package);
            } else {
                dev.insert(package);
            }
        }
    }
    dev.difference(&other).cloned().collect()
}

/// The `tools/<name>.sh` and `tools/<name>.py` paths a line names.
fn scripts_in(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = line[from..].find("tools/") {
        let start = from.saturating_add(at);
        from = start.saturating_add("tools/".len());
        if line[..start].chars().next_back().is_some_and(|c| {
            c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '/' || c == '.'
        }) {
            continue;
        }
        let name: String = line[from..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
            .collect();
        let name = name.trim_end_matches('.');
        let script = Path::new(name)
            .extension()
            .is_some_and(|extension| extension == "sh" || extension == "py");
        if script && !is_a_form(name) {
            found.push(format!("tools/{name}"));
        }
    }
    found
}

/// Which of a package's target sets a flag names.
type TargetsOf = fn(&Targets) -> &BTreeSet<String>;

/// What a command names that the tree does not have, one sentence per finding.
fn findings(
    packages: &BTreeMap<String, Targets>,
    dev: &BTreeSet<String>,
    command: &Command,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut scope: Vec<&Targets> = Vec::new();
    for package in &command.packages {
        match packages.get(package) {
            Some(targets) => scope.push(targets),
            None => out.push(format!("no workspace member is called `{package}`")),
        }
    }
    if command.packages.is_empty() {
        scope.extend(packages.values());
    }
    if !out.is_empty() {
        return out;
    }
    let kinds: [(&str, &Vec<String>, TargetsOf); 3] = [
        ("example", &command.examples, |t| &t.examples),
        ("test", &command.tests, |t| &t.tests),
        ("bin", &command.bins, |t| &t.bins),
    ];
    for (kind, names, of) in kinds {
        for name in names {
            if !scope.iter().any(|targets| of(targets).contains(name)) {
                out.push(format!(
                    "no {kind} target `{name}` in {:?}",
                    command.packages
                ));
            }
        }
    }
    if command.release && command.runs {
        for package in &command.packages {
            if dev.contains(package) {
                out.push(format!(
                    "`--release` on `{package}`, which doc/verify.md runs under the default profile"
                ));
            }
        }
    }
    out
}

/// Every finding over the live documents, and the population's sizes.
fn sweep(root: &Path) -> (Vec<String>, usize, usize) {
    let packages = workspace(root);
    let verify = fs::read_to_string(root.join("doc/verify.md")).unwrap_or_default();
    let dev = dev_only(&verify);
    let mut out = Vec::new();
    let documents = documents(root);
    let mut commands: usize = 0;
    for (relative, text) in &documents {
        for (line, command) in commands_in(text) {
            commands = commands.saturating_add(1);
            for finding in findings(&packages, &dev, &command) {
                out.push(format!("{relative}:{line} {finding}"));
            }
        }
        for (number, line) in (1_usize..).zip(text.lines()) {
            for script in scripts_in(line) {
                if !root.join(&script).exists() {
                    out.push(format!("{relative}:{number} no script `{script}`"));
                }
            }
        }
    }
    (out, documents.len(), commands)
}

#[test]
fn every_command_a_live_document_writes_names_what_the_tree_has() {
    let root = conformance::workspace_root();
    let (found, documents, commands) = sweep(&root);
    println!("{commands} cargo command(s) in {documents} live document(s)");
    for finding in &found {
        println!("  {finding}");
    }
    assert!(
        documents > 0 && commands > 0,
        "the sweep read nothing: {documents} document(s), {commands} command(s) — a clean answer \
         over an empty population is a sentence about the sweep"
    );
    assert!(
        found.is_empty(),
        "a live document writes a command that names what the tree does not have, so a reader who \
         copies it is refused or builds what nobody prescribes. Write the command the tree runs:\n  \
         {}",
        found.join("\n  ")
    );
}

#[test]
fn the_sweep_names_a_planted_command_the_tree_cannot_run() {
    let mut packages = BTreeMap::new();
    let mut targets = Targets::default();
    targets.examples.insert("frame_budget".to_owned());
    packages.insert("render-raster".to_owned(), targets);
    let mut conformance = Targets::default();
    conformance.bins.insert("ledger".to_owned());
    packages.insert("conformance".to_owned(), conformance);
    let dev: BTreeSet<String> = dev_only("cargo run -p conformance --bin ledger")
        .into_iter()
        .collect();
    assert_eq!(
        dev.len(),
        1,
        "a package verify.md runs only under dev: {dev:?}"
    );
    let planted = "`cargo run --release -p render-raster --example frame_budget -- x`\n\
                   `cargo run --release -p render-raster --example zoom_ladder`\n\
                   `cargo test --release -p render-quorra --test corpus`\n\
                   `cargo run --release -p conformance --bin ledger`\n\
                   `cargo run -p <crate> --example <name>` and cargo's own lock\n\
                   `cargo build --release -p conformance --bins`\n";
    let found: Vec<String> = commands_in(planted)
        .iter()
        .flat_map(|(line, command)| {
            findings(&packages, &dev, command)
                .into_iter()
                .map(move |finding| format!("{line} {finding}"))
        })
        .collect();
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found[0].starts_with("2 no example target `zoom_ladder`"),
        "{found:?}"
    );
    assert!(found[1].starts_with("3 no workspace member"), "{found:?}");
    assert!(
        found[2].starts_with("4 `--release` on `conformance`"),
        "{found:?}"
    );
    assert_eq!(
        scripts_in("run `tools/state.sh` and tools/<script>.sh"),
        ["tools/state.sh"]
    );
}
