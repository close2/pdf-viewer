//! A message that names a flag names one the program accepts.
//!
//! # The defect this closes
//!
//! Five refusal messages `quorra-transform` prints, and two documents, told a user to supply a
//! missing font with `--font`. The program's own `KNOWN` list does not hold that flag, so the
//! command the message asks for is refused as a usage error. A message that names a flag the
//! program does not accept is worse than no message: it spends the user's next attempt, and it
//! reads exactly like a message that works.
//!
//! Nothing checked it, because the two halves live apart — the accepted set is in the binary's
//! argument parser and the mention is in a library's error text, often a different crate's file
//! entirely. This sweep puts them on one line.
//!
//! # The two populations, both derived
//!
//! - **The binaries**, from the workspace manifest ([`crate::roots::members`]) and each member's
//!   own manifest: every `[[bin]]` block, plus cargo's own defaults — `src/main.rs` named after
//!   the package, and each `src/bin/<name>.rs` named after its file. A hand-written list of
//!   programs is trap 25's shape, and two of this tree's binaries are renamed by a `[[bin]]`
//!   block, so a list derived from filenames alone would name programs that do not exist.
//! - **The accepted flags**, from each binary's own source set: its root file and every module it
//!   declares, followed through `mod x;` and `#[path = "…"] mod x;`. A flag is accepted if it
//!   appears as a string literal there, because a comparison against argv is written nowhere else.
//!   The cost of that rule is stated rather than hidden: a flag named in the binary's own source
//!   and never compared is read as accepted, so this sweep errs towards silence exactly where the
//!   program's own file mentions the flag.
//!
//! # What counts as a mention, and the three conditions that make it one
//!
//! A mention is a flag in a **message**: a string literal in a crate's `src/`, or a line of a
//! `doc/*.md`. Three conditions narrow that to the question being asked, and each exists because
//! a run without it produced findings that were not the rule (trap 11):
//!
//! 1. **A `#[cfg(test)]` module is not the program.** A test builds command lines for other
//!    programs and asserts on sample configurations; neither is a message a user is shown.
//! 2. **A literal with no whitespace around the flag is not a message.** `.arg("--silent")` is an
//!    argument this tree passes to `curl`; "supplying the font with --font resolves it" is a
//!    sentence addressed to a person. The first shape produced twelve findings, all of them
//!    another program's flags.
//! 3. **A command span belongs to the program it names, and cargo's `--` is where the program
//!    starts.** Where a flag stands in a run of command-like tokens — a backtick-quoted span, or a
//!    literal that is all words and flags — the program is the span's first token, unless a bare
//!    `--` stands before the flag, in which case it is the last binary of this tree named before
//!    that `--`. `cargo build -p pdf-sandbox --bins` is cargo's line and `--bins` is cargo's flag,
//!    whoever prints it; `cargo +nightly build --release --bin quorra --unit-graph` is cargo's
//!    too, all of it, which is why the rule is not "the last binary named"; and in `cargo run -p
//!    pdf-transform --bin quorra-transform -- archive --remedy-sites` the `--` hands the rest to
//!    ours.
//!
//! **Attribution outside a command span is by crate.** A message a program prints is written in
//! its own crate, so a flag in prose in crate C is asked of the binaries C declares — all of them,
//! and it is a finding only when not one accepts it. Attribution by *name on the line* was tried
//! first and is the weaker predicate: several of this tree's binaries are named for ordinary
//! English words (`counts`, `entries`, `parts`, `owed`), and "`--every` counts from 1" attributed
//! a `quorra-transform` message to the `counts` binary. That is trap 11's sixth instance, and the
//! rule that survives it is the crate's.
//!
//! **A document has no crate, so there condition 3 is the whole of the attribution**: a `doc/*.md`
//! line is asked about a flag only where the command span names one of these programs. The cost is
//! stated: a document that names a flag in prose without naming the program it belongs to is not
//! asked about. Attributing a document's line to every binary a fenced block mentions was tried
//! and is trap 11's shape again — `doc/verify.md`'s command listing names two dozen of them in one
//! fence, so every cargo and busctl flag in it became a finding against all of them.
//!
//! ADR 1213.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::roots;

/// Why the sweep could not be run.
///
/// Every variant is a refusal rather than a fallback, for [`crate::roots`]'s reason: a sweep that
/// answered "no mentions" when it could not read the tree would print a tick for a tree it had
/// not looked at.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The tree's shape could not be derived.
    #[error(transparent)]
    Roots(#[from] roots::Error),
    /// A file or directory could not be read.
    #[error("{path} could not be read: {source}")]
    Unreadable {
        /// What was being read.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
    /// The derivation found no binary at all.
    #[error(
        "no binary was derived from the workspace's members, so every mention would be \
         unattributable and this sweep would print a clean tree for a tree it had not looked at"
    )]
    NoBinaries,
}

/// One program this workspace builds, with the flags its own source names.
#[derive(Debug, Clone)]
pub struct Binary {
    /// What cargo calls it, which is what a user types.
    pub name: String,
    /// The package that declares it.
    pub package: String,
    /// Its root source file and every module that file declares, relative to the tree's root.
    pub sources: Vec<PathBuf>,
    /// Every long flag named as a string literal anywhere in [`Self::sources`].
    pub accepted: BTreeSet<String>,
}

/// One flag named in a message, and the binaries the message is addressed to.
#[derive(Debug, Clone)]
pub struct Mention {
    /// The file, relative to the tree's root.
    pub path: PathBuf,
    /// The line the flag stands on.
    pub line: usize,
    /// The binaries asked about it — one crate's, or the ones a document's line named.
    pub binaries: Vec<String>,
    /// The flag, with its two leading dashes.
    pub flag: String,
    /// The message, trimmed and shortened, so that a reader can judge the hit without opening
    /// the file.
    pub text: String,
}

/// What the sweep found, with the denominators it found it over.
#[derive(Debug, Default)]
pub struct Report {
    /// Every binary, in name order.
    pub binaries: Vec<Binary>,
    /// Every mention of a flag not one of its binaries accepts.
    pub unaccepted: Vec<Mention>,
    /// Packages declaring no binary, whose `src/` is therefore asked about nothing.
    pub packages_without_a_binary: usize,
    /// Rust source files read.
    pub sources_read: usize,
    /// Documents under `doc/` read.
    pub documents_read: usize,
    /// Flag mentions examined, accepted ones included, which is this sweep's denominator.
    pub mentions: usize,
}

/// How many characters of a message a report prints.
const SHOWN: usize = 110;

/// The plant this sweep is calibrated on, run through the same two attribution paths.
///
/// Trap 13: a sweep that comes back clean has said something about itself unless it has been run
/// against the defect. The plant is a flag no program of this tree accepts, put into a message of
/// the shape the defect had and into a documented command line naming a real binary — and the
/// binary and its package are taken from the derivation rather than written here, so the
/// calibration cannot outlive the program it names.
///
/// It plants into the functions rather than into a file of the tree, because a plant written into
/// the tree is a plant somebody has to remember to remove.
///
/// # Errors
///
/// As [`sweep`].
pub fn calibrate(root: &Path) -> Result<Vec<Mention>, Error> {
    let tree = Tree::derive(root)?;
    let (package, binary) = tree
        .by_package
        .iter()
        .find_map(|(package, binaries)| match binaries.as_slice() {
            [only] => Some((package.clone(), only.clone())),
            _ => None,
        })
        .ok_or(Error::NoBinaries)?;
    // Assembled from two pieces rather than written whole, so that this file's own source does
    // not carry a message naming a flag no program accepts — which this sweep would report, and
    // would be right to.
    let planted = format!("--{}", "frobnicate");
    let message = Literal {
        line: 1,
        text: format!(
            "this document renders a font it does not embed; supplying it with {planted} \
             resolves it"
        ),
    };
    let line = Literal {
        line: 1,
        text: format!("cargo run -p {package} --bin {binary} -- archive {planted}"),
    };
    let mut found = Vec::new();
    let asked = std::slice::from_ref(&binary);
    for literal in [&message, &line] {
        for (before, flag) in flags(&literal.text) {
            let asked = match program_of_the_span(&literal.text, before, &tree.names) {
                Span::Foreign => continue,
                Span::Ours(name) => std::slice::from_ref(name),
                Span::Prose => asked,
            };
            note(
                &mut found,
                &tree.accepted,
                asked,
                literal,
                before,
                &flag,
                Path::new("the calibration"),
            );
        }
    }
    Ok(found)
}

/// The binaries this workspace builds, and what each accepts.
struct Tree {
    /// Every binary, in name order.
    binaries: Vec<Binary>,
    /// The binaries each package declares.
    by_package: BTreeMap<String, Vec<String>>,
    /// Every binary's name, for the command-span rule.
    names: BTreeSet<String>,
    /// Every binary's accepted flags.
    accepted: BTreeMap<String, BTreeSet<String>>,
    /// The workspace's members, in manifest order.
    members: Vec<String>,
}

impl Tree {
    /// Derives the whole of it from the workspace's manifests and sources.
    fn derive(root: &Path) -> Result<Self, Error> {
        let members = roots::members(root)?;
        let mut binaries = Vec::new();
        let mut by_package: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for member in &members {
            for binary in binaries_of(root, member)? {
                by_package
                    .entry(binary.package.clone())
                    .or_default()
                    .push(binary.name.clone());
                binaries.push(binary);
            }
        }
        if binaries.is_empty() {
            return Err(Error::NoBinaries);
        }
        binaries.sort_by(|a, b| a.name.cmp(&b.name));
        let names = binaries.iter().map(|binary| binary.name.clone()).collect();
        let accepted = binaries
            .iter()
            .map(|binary| (binary.name.clone(), binary.accepted.clone()))
            .collect();
        Ok(Self {
            binaries,
            by_package,
            names,
            accepted,
            members,
        })
    }
}

/// Runs the sweep over the tree at `root`.
///
/// # Errors
///
/// [`Error`] — the tree's shape could not be derived, a file could not be read, or the
/// derivation produced no binaries at all.
pub fn sweep(root: &Path) -> Result<Report, Error> {
    let Tree {
        binaries,
        by_package,
        names,
        accepted,
        members,
    } = Tree::derive(root)?;
    let mut report = Report {
        binaries,
        ..Report::default()
    };

    for member in &members {
        let package = package_name(member);
        let Some(asked) = by_package.get(&package) else {
            report.packages_without_a_binary = report.packages_without_a_binary.saturating_add(1);
            continue;
        };
        for path in rust_sources(&root.join(member).join("src"))? {
            report.sources_read = report.sources_read.saturating_add(1);
            let text = read(root, &path)?;
            let relative = relative(root, &path);
            for literal in literals(&text) {
                for (before, flag) in flags(&literal.text) {
                    report.mentions = report.mentions.saturating_add(1);
                    if !is_a_message(&literal.text, before, &flag) {
                        continue;
                    }
                    let asked = match program_of_the_span(&literal.text, before, &names) {
                        Span::Foreign => continue,
                        Span::Ours(name) => std::slice::from_ref(name),
                        Span::Prose => asked.as_slice(),
                    };
                    note(
                        &mut report.unaccepted,
                        &accepted,
                        asked,
                        &literal,
                        before,
                        &flag,
                        &relative,
                    );
                }
            }
        }
    }

    for path in documents(&root.join("doc"))? {
        report.documents_read = report.documents_read.saturating_add(1);
        let text = read(root, &path)?;
        let relative = relative(root, &path);
        for logical in document_lines(&text) {
            for (before, flag) in flags(&logical.text) {
                report.mentions = report.mentions.saturating_add(1);
                let Span::Ours(name) = program_of_the_span(&logical.text, before, &names) else {
                    continue;
                };
                note(
                    &mut report.unaccepted,
                    &accepted,
                    std::slice::from_ref(name),
                    &logical,
                    before,
                    &flag,
                    &relative,
                );
            }
        }
    }
    report
        .unaccepted
        .sort_by(|a, b| (&a.path, a.line, &a.flag).cmp(&(&b.path, b.line, &b.flag)));
    Ok(report)
}

/// Records one mention where not one of the binaries asked accepts the flag.
fn note(
    into: &mut Vec<Mention>,
    accepted: &BTreeMap<String, BTreeSet<String>>,
    asked: &[String],
    literal: &Literal,
    before: usize,
    flag: &str,
    path: &Path,
) {
    if asked.is_empty() {
        return;
    }
    if asked
        .iter()
        .any(|name| accepted.get(name).is_some_and(|set| set.contains(flag)))
    {
        return;
    }
    into.push(Mention {
        path: path.to_path_buf(),
        line: literal.line.saturating_add(newlines(&literal.text, before)),
        binaries: asked.to_vec(),
        flag: flag.to_owned(),
        text: shorten(literal.text.trim()),
    });
}

/// The package a member directory declares, which is its last path segment.
fn package_name(member: &str) -> String {
    member.rsplit('/').next().unwrap_or(member).to_owned()
}

/// Every binary one member declares, with the flags its own sources name.
fn binaries_of(root: &Path, member: &str) -> Result<Vec<Binary>, Error> {
    let directory = root.join(member);
    let package = package_name(member);
    let manifest = read(root, &directory.join(roots::MANIFEST))?;
    let renamed = bin_blocks(&manifest);
    let mut found = Vec::new();
    let mut candidates = Vec::new();
    let main = directory.join("src").join("main.rs");
    if main.is_file() {
        candidates.push((main, package.clone()));
    }
    // Cargo's own rule, and the recursion an earlier draft used was wrong about it: a program is
    // `src/bin/<name>.rs` or `src/bin/<name>/main.rs`, and every other file under `src/bin/` is a
    // module of one of those. Walking the tree made `viewer-ui`'s nineteen `src/bin/quorra/`
    // modules into nineteen programs, each with a flag set of its own.
    for path in bin_targets(&directory.join("src").join("bin"))? {
        let name = match path.file_stem().and_then(|stem| stem.to_str()) {
            Some("main") => path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .map(str::to_owned),
            Some(stem) => Some(stem.to_owned()),
            None => None,
        };
        let Some(name) = name else {
            continue;
        };
        candidates.push((path.clone(), name));
    }
    for (path, default) in candidates {
        let relative = relative(&directory, &path);
        let name = renamed
            .get(&relative.to_string_lossy().replace('\\', "/"))
            .cloned()
            .unwrap_or(default);
        let sources = source_set(root, &path)?;
        let mut accepted = BTreeSet::new();
        for source in &sources {
            for literal in literals(&read(root, &root.join(source))?) {
                for (_, flag) in flags(&literal.text) {
                    accepted.insert(flag);
                }
            }
        }
        found.push(Binary {
            name,
            package: package.clone(),
            sources,
            accepted,
        });
    }
    Ok(found)
}

/// Cargo's automatic binary targets under `src/bin`: each `<name>.rs` and each
/// `<name>/main.rs`, and nothing deeper.
fn bin_targets(directory: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut found = Vec::new();
    if !directory.is_dir() {
        return Ok(found);
    }
    for entry in entries(directory)? {
        if entry.is_file() && entry.extension().is_some_and(|extension| extension == "rs") {
            found.push(entry);
            continue;
        }
        let main = entry.join("main.rs");
        if entry.is_dir() && main.is_file() {
            found.push(main);
        }
    }
    found.sort();
    Ok(found)
}

/// The `name`/`path` pairs of a manifest's `[[bin]]` blocks, keyed by the path.
///
/// Read with the same line-wise reader [`crate::roots`] uses on the workspace manifest: a member
/// that renames its program is what makes a derivation from filenames alone wrong, and two of
/// this tree's do.
fn bin_blocks(manifest: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut inside = false;
    let mut name = None;
    let mut path = None;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if let (Some(name), Some(path)) = (name.take(), path.take()) {
                found.insert(path, name);
            }
            inside = line == "[[bin]]";
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(value) = quoted_value(line, "name") {
            name = Some(value);
        }
        if let Some(value) = quoted_value(line, "path") {
            path = Some(value);
        }
    }
    if let (Some(name), Some(path)) = (name, path) {
        found.insert(path, name);
    }
    found
}

/// `key = "value"`, where the line states that key.
fn quoted_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start().strip_prefix('=')?;
    let rest = rest.trim_start().strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

/// A binary's root file and every module reachable from it, relative to `root`.
///
/// Both spellings: `mod x;`, which cargo resolves beside the file or in a directory named after
/// it, and `#[path = "…"] mod x;`, which every binary in this tree with modules of its own uses
/// because a `src/bin/` program's modules live in a directory beside it.
fn source_set(root: &Path, start: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut seen = vec![start.to_path_buf()];
    let mut queue = vec![start.to_path_buf()];
    while let Some(current) = queue.pop() {
        let text = read(root, &current)?;
        let base = current.parent().unwrap_or(root).to_path_buf();
        let stem = current
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();
        for declared in module_declarations(&text) {
            let candidates = match declared {
                Declared::At(path) => vec![base.join(path)],
                Declared::Named(name) => vec![
                    base.join(&stem).join(format!("{name}.rs")),
                    base.join(format!("{name}.rs")),
                ],
            };
            for candidate in candidates {
                if candidate.is_file() && !seen.contains(&candidate) {
                    seen.push(candidate.clone());
                    queue.push(candidate);
                    break;
                }
            }
        }
    }
    Ok(seen.iter().map(|path| relative(root, path)).collect())
}

/// A module declaration, in the two spellings a binary uses.
enum Declared {
    /// `#[path = "…"] mod x;`
    At(String),
    /// `mod x;`
    Named(String),
}

/// Every module a source file declares.
fn module_declarations(text: &str) -> Vec<Declared> {
    let mut found = Vec::new();
    let mut at = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#[path") {
            at = rest
                .trim_start()
                .strip_prefix('=')
                .and_then(|rest| rest.trim_start().strip_prefix('"'))
                .and_then(|rest| rest.find('"').map(|end| rest[..end].to_owned()));
            continue;
        }
        let declaration = line.strip_prefix("pub mod ").or(line.strip_prefix("mod "));
        let Some(name) = declaration.and_then(|rest| rest.strip_suffix(';')) else {
            at = None;
            continue;
        };
        let name = name.trim();
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            at = None;
            continue;
        }
        match at.take() {
            Some(path) => found.push(Declared::At(path)),
            None => found.push(Declared::Named(name.to_owned())),
        }
    }
    found
}

/// One string literal, and the line it begins on.
struct Literal {
    /// The one-based line of the literal's opening quote.
    line: usize,
    /// Its text, with escapes left as written and a line continuation replaced by a space.
    text: String,
}

/// Every string literal in a Rust source, comments and `#[cfg(test)]` modules skipped.
///
/// Hand-written rather than taken from a crate, for this crate's own stated reason: a checker
/// that needs a dependency to run is a checker that can stop running. It handles the three
/// shapes this tree writes — a normal literal with `\` escapes and `\`-newline continuations, a
/// raw literal with any number of hashes, and a character literal, which is skipped so that an
/// apostrophe in a lifetime cannot open a string.
#[expect(
    clippy::too_many_lines,
    reason = "one scanner over one input: splitting it would put the cursor's state in a struct \
              and make every case a method, which is harder to read than the pass itself"
)]
fn literals(text: &str) -> Vec<Literal> {
    let bytes: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    let mut line = 1_usize;
    // Depth of the `#[cfg(test)]` module the cursor stands in, and the brace depth it began at.
    let mut braces = 0_usize;
    let mut test_module: Option<usize> = None;
    let mut pending_test = false;
    while index < bytes.len() {
        let c = bytes[index];
        if c == '\n' {
            line = line.saturating_add(1);
            index = index.saturating_add(1);
            continue;
        }
        if c == '/' && bytes.get(index.saturating_add(1)) == Some(&'/') {
            while index < bytes.len() && bytes[index] != '\n' {
                index = index.saturating_add(1);
            }
            continue;
        }
        if c == '/' && bytes.get(index.saturating_add(1)) == Some(&'*') {
            let mut depth = 1_usize;
            index = index.saturating_add(2);
            while index < bytes.len() && depth > 0 {
                if bytes[index] == '\n' {
                    line = line.saturating_add(1);
                }
                if bytes[index] == '/' && bytes.get(index.saturating_add(1)) == Some(&'*') {
                    depth = depth.saturating_add(1);
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == '*' && bytes.get(index.saturating_add(1)) == Some(&'/') {
                    depth = depth.saturating_sub(1);
                    index = index.saturating_add(2);
                    continue;
                }
                index = index.saturating_add(1);
            }
            continue;
        }
        if c == '{' {
            braces = braces.saturating_add(1);
            if pending_test && test_module.is_none() {
                test_module = Some(braces);
            }
            pending_test = false;
            index = index.saturating_add(1);
            continue;
        }
        if c == '}' {
            if test_module == Some(braces) {
                test_module = None;
            }
            braces = braces.saturating_sub(1);
            index = index.saturating_add(1);
            continue;
        }
        if starts_with(&bytes, index, "#[cfg(test)]") {
            pending_test = true;
            index = index.saturating_add("#[cfg(test)]".len());
            continue;
        }
        if c == 'r'
            && matches!(bytes.get(index.saturating_add(1)), Some('#' | '"'))
            && !bytes
                .get(index.wrapping_sub(1))
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            let mut hashes = 0_usize;
            let mut cursor = index.saturating_add(1);
            while bytes.get(cursor) == Some(&'#') {
                hashes = hashes.saturating_add(1);
                cursor = cursor.saturating_add(1);
            }
            if bytes.get(cursor) == Some(&'"') {
                let began = line;
                cursor = cursor.saturating_add(1);
                let mut body = String::new();
                while cursor < bytes.len() && !closes_raw(&bytes, cursor, hashes) {
                    if bytes[cursor] == '\n' {
                        line = line.saturating_add(1);
                    }
                    body.push(bytes[cursor]);
                    cursor = cursor.saturating_add(1);
                }
                if test_module.is_none() {
                    found.push(Literal {
                        line: began,
                        text: body,
                    });
                }
                index = cursor.saturating_add(hashes.saturating_add(1));
                continue;
            }
        }
        if c == '\'' {
            // A character literal or a lifetime, and the two are stepped over differently. A
            // lifetime has no closing quote, so one character is right; a character literal
            // does, and stepping one character over `'{'` leaves its brace to be counted — which
            // desynchronised the `#[cfg(test)]` depth below and let a test module's literals read
            // as the program's messages.
            index = index.saturating_add(char_literal(&bytes, index));
            continue;
        }
        if c == '"' {
            let began = line;
            let mut cursor = index.saturating_add(1);
            let mut body = String::new();
            while cursor < bytes.len() && bytes[cursor] != '"' {
                if bytes[cursor] == '\\' {
                    if bytes.get(cursor.saturating_add(1)) == Some(&'\n') {
                        line = line.saturating_add(1);
                        body.push('\n');
                    } else {
                        body.push(' ');
                    }
                    cursor = cursor.saturating_add(2);
                    continue;
                }
                if bytes[cursor] == '\n' {
                    line = line.saturating_add(1);
                }
                body.push(bytes[cursor]);
                cursor = cursor.saturating_add(1);
            }
            if test_module.is_none() {
                found.push(Literal {
                    line: began,
                    text: body,
                });
            }
            index = cursor.saturating_add(1);
            continue;
        }
        index = index.saturating_add(1);
    }
    found
}

/// Whether the characters at `index` are `needle`.
fn starts_with(bytes: &[char], index: usize, needle: &str) -> bool {
    needle
        .chars()
        .enumerate()
        .all(|(offset, c)| bytes.get(index.saturating_add(offset)) == Some(&c))
}

/// How many characters a character literal or a lifetime at `index` occupies.
///
/// A lifetime is one character here — its quote closes nothing — and a character literal is the
/// whole of `'x'` or `'\\n'`, so that a brace inside one is not counted as a brace of the source.
fn char_literal(bytes: &[char], index: usize) -> usize {
    if bytes.get(index.saturating_add(1)) == Some(&'\\') {
        let mut cursor = index.saturating_add(2);
        while cursor < bytes.len() && bytes[cursor] != '\'' {
            cursor = cursor.saturating_add(1);
        }
        return cursor.saturating_add(1).saturating_sub(index);
    }
    if bytes.get(index.saturating_add(2)) == Some(&'\'') {
        return 3;
    }
    1
}

/// Whether a raw literal's closing quote and hashes stand at `index`.
fn closes_raw(bytes: &[char], index: usize, hashes: usize) -> bool {
    bytes.get(index) == Some(&'"')
        && (0..hashes)
            .all(|offset| bytes.get(index.saturating_add(offset).saturating_add(1)) == Some(&'#'))
}

/// Every long flag in a text, with the character offset it begins at.
///
/// A flag is `--` followed by a lowercase letter and then letters, digits and hyphens, not
/// preceded by a word character or a hyphen and not ending in one. The last condition is what
/// keeps a multipart form boundary — `--quorra-form-data-0--` — from reading as a flag, which is
/// trap 11's "a suffix is not a name" in the other direction.
fn flags(text: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '-' || chars.get(index.saturating_add(1)) != Some(&'-') {
            index = index.saturating_add(1);
            continue;
        }
        if index > 0
            && chars
                .get(index.saturating_sub(1))
                .is_some_and(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        {
            index = index.saturating_add(1);
            continue;
        }
        let mut end = index.saturating_add(2);
        if !chars.get(end).is_some_and(char::is_ascii_lowercase) {
            index = index.saturating_add(1);
            continue;
        }
        while chars
            .get(end)
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        {
            end = end.saturating_add(1);
        }
        let flag: String = chars[index..end].iter().collect();
        if !flag.ends_with('-') {
            found.push((index, flag));
        }
        index = end;
    }
    found
}

/// Whether the flag at `before` stands in a message rather than in a bare argument.
///
/// The second of the three conditions: a literal whose whole content is the flag, or a list of
/// such tokens, is an argument this tree hands to another program. A message has words around it.
fn is_a_message(text: &str, before: usize, flag: &str) -> bool {
    let after = before.saturating_add(flag.chars().count());
    let head: String = text.chars().take(before).collect();
    let tail: String = text.chars().skip(after).collect();
    head.split_whitespace().any(is_a_word) || tail.split_whitespace().any(is_a_word)
}

/// Whether a token is a word rather than a flag, a punctuation mark or a placeholder.
fn is_a_word(token: &str) -> bool {
    let trimmed = token.trim_matches(|c: char| !c.is_alphanumeric());
    trimmed.len() > 1 && !trimmed.starts_with('-') && trimmed.chars().any(char::is_alphabetic)
}

/// Who a flag standing in a command-like span belongs to.
enum Span<'a> {
    /// A program this workspace does not build, so the flag is not ours to check.
    Foreign,
    /// One of ours, named in the span before the flag.
    Ours(&'a String),
    /// Not a command span at all — prose, to be attributed by crate or by the document's line.
    Prose,
}

/// The third condition: the program a command-like span names.
///
/// The span runs from the nearest backtick before the flag (or the text's start) to the flag. It
/// is command-like when every token in it is a word, a short flag, a long flag or a bare `--` — a
/// comma or a full stop ends it, because a sentence is not a command line. Inside such a span the
/// program is the **first** token, unless a bare `--` stands before the flag: cargo's argument
/// terminator is where one program's flags stop and the named program's begin, so the last binary
/// of this tree named before that `--` owns what follows it.
fn program_of_the_span<'a>(text: &str, before: usize, names: &'a BTreeSet<String>) -> Span<'a> {
    let head: String = text.chars().take(before).collect();
    let span = head.rsplit('`').next().unwrap_or(&head);
    let tokens: Vec<&str> = span.split_whitespace().collect();
    let Some(first) = tokens.first() else {
        return Span::Prose;
    };
    if !tokens.iter().all(|token| is_command_like(token)) {
        return Span::Prose;
    }
    if let Some(terminator) = tokens.iter().position(|token| *token == "--") {
        return tokens
            .get(..terminator)
            .unwrap_or_default()
            .iter()
            .rev()
            .find_map(|token| names.get(*token))
            .map_or(Span::Foreign, Span::Ours);
    }
    names.get(*first).map_or(Span::Foreign, Span::Ours)
}

/// Whether a token can stand in a command line without ending a sentence.
fn is_command_like(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || matches!(c, '-' | '_' | '.' | '/' | '=' | '+' | '{' | '}')
        })
}

/// How many newlines stand before `before` in a literal, so a multi-line message reports the
/// line the flag is on rather than the line the quote opened.
fn newlines(text: &str, before: usize) -> usize {
    text.chars().take(before).filter(|c| *c == '\n').count()
}

/// A message, shortened for a report and with its newlines flattened.
fn shorten(text: &str) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= SHOWN {
        return flat;
    }
    flat.chars().take(SHOWN).collect::<String>() + "…"
}

/// Every line of a document, with a `\`-continued command joined into one.
///
/// A fenced command runs over several lines in this tree's documents and the program is named on
/// the first of them, so a continuation has to be read with what it continues or the span that
/// names the program is not there to read. The line reported is still the physical one.
fn document_lines(text: &str) -> Vec<Literal> {
    let mut found: Vec<Literal> = Vec::new();
    let mut joining = false;
    for (index, line) in text.lines().enumerate() {
        let continues = line.trim_end().ends_with('\\');
        if joining {
            if let Some(last) = found.last_mut() {
                last.text.push('\n');
                last.text.push_str(line);
            }
        } else {
            found.push(Literal {
                line: index.saturating_add(1),
                text: line.to_owned(),
            });
        }
        joining = continues;
    }
    found
}

/// Every `.rs` under a directory, sorted, or nothing where the directory does not exist.
fn rust_sources(directory: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut found = Vec::new();
    if !directory.is_dir() {
        return Ok(found);
    }
    for path in entries(directory)? {
        if path.is_dir() {
            found.extend(rust_sources(&path)?);
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(found)
}

/// One directory's entries, sorted, naming the directory if it cannot be read.
fn entries(directory: &Path) -> Result<Vec<PathBuf>, Error> {
    let listing = std::fs::read_dir(directory).map_err(|source| Error::Unreadable {
        path: directory.display().to_string(),
        source,
    })?;
    let mut found = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|source| Error::Unreadable {
            path: directory.display().to_string(),
            source,
        })?;
        found.push(entry.path());
    }
    found.sort();
    Ok(found)
}

/// Every `doc/*.md`, sorted. The top level only: `doc/`'s subdirectories are records — ADRs,
/// histories, reviews — and a record states what was true when it was written.
fn documents(directory: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut found = Vec::new();
    if !directory.is_dir() {
        return Ok(found);
    }
    let listing = std::fs::read_dir(directory).map_err(|source| Error::Unreadable {
        path: directory.display().to_string(),
        source,
    })?;
    for entry in listing {
        let entry = entry.map_err(|source| Error::Unreadable {
            path: directory.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "md") {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// Reads a file, naming it if it cannot be read.
fn read(root: &Path, path: &Path) -> Result<String, Error> {
    let full = if path.is_absolute() || path.starts_with(root) {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    std::fs::read_to_string(&full).map_err(|source| Error::Unreadable {
        path: full.display().to_string(),
        source,
    })
}

/// A path relative to `root`, or the path itself where it is not under it.
fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

/// The report, as a person reads it.
#[must_use]
pub fn report(found: &Report) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} binary(ies) over {} package(s) with one, {} package(s) declaring none; {} Rust \
         source(s) and {} document(s) read, {} flag mention(s) examined",
        found.binaries.len(),
        found
            .binaries
            .iter()
            .map(|binary| binary.package.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        found.packages_without_a_binary,
        found.sources_read,
        found.documents_read,
        found.mentions,
    );
    for binary in &found.binaries {
        let _ = writeln!(
            out,
            "  {:<22} {:<18} {:>2} source file(s), {:>2} accepted flag(s)",
            binary.name,
            binary.package,
            binary.sources.len(),
            binary.accepted.len(),
        );
    }
    let _ = writeln!(
        out,
        "\n{} mention(s) of a flag the program does not accept",
        found.unaccepted.len()
    );
    for mention in &found.unaccepted {
        let _ = writeln!(
            out,
            "  {}:{} {} is named to {}\n      {}",
            mention.path.display(),
            mention.line,
            mention.flag,
            mention.binaries.join(", "),
            mention.text,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comment_is_not_a_literal_and_an_apostrophe_does_not_open_one() {
        let found = literals("// a \" quote\nlet x = 'a';\nlet y = \"--real\";\n");
        assert_eq!(
            found.len(),
            1,
            "{:?}",
            found.iter().map(|l| &l.text).collect::<Vec<_>>()
        );
        assert_eq!(found[0].text, "--real");
        assert_eq!(found[0].line, 3);
    }

    #[test]
    fn a_test_module_is_not_the_program() {
        let found = literals(
            "fn a() { let x = \"--one\"; }\n#[cfg(test)]\nmod t { fn b() { let y = \"--two\"; } }\n",
        );
        let texts: Vec<&str> = found.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["--one"],
            "a #[cfg(test)] module's literals are not messages"
        );
    }

    #[test]
    fn a_form_boundary_is_not_a_flag() {
        assert!(flags("--quorra-form-data-0--").is_empty());
        assert_eq!(flags("pass --font here")[0].1, "--font");
    }

    #[test]
    fn a_bare_argument_is_not_a_message() {
        assert!(!is_a_message("--silent", 0, "--silent"));
        assert!(is_a_message("supply it with --font", 15, "--font"));
    }

    #[test]
    fn a_command_span_belongs_to_the_program_it_names() {
        let names: BTreeSet<String> = ["quorra-transform".to_owned()].into_iter().collect();
        let text = "build it with `cargo build -p pdf-sandbox --bins`";
        let (before, flag) = flags(text).remove(0);
        assert_eq!(flag, "--bins");
        assert!(matches!(
            program_of_the_span(text, before, &names),
            Span::Foreign
        ));

        let ours = "cargo run --bin quorra-transform -- archive --remedy-sites";
        let (before, flag) = flags(ours)
            .into_iter()
            .find(|(_, f)| f == "--remedy-sites")
            .expect("the flag");
        assert_eq!(flag, "--remedy-sites");
        assert!(matches!(
            program_of_the_span(ours, before, &names),
            Span::Ours(_)
        ));

        let prose = "supplying the font itself, with --font, resolves it";
        let (before, _) = flags(prose).remove(0);
        assert!(matches!(
            program_of_the_span(prose, before, &names),
            Span::Prose
        ));
    }

    #[test]
    fn a_bin_block_renames_the_program() {
        let manifest = "[package]\nname = \"pdf-fuse\"\n\n[[bin]]\nname = \"quorrafs\"\npath = \"src/main.rs\"\n";
        let blocks = bin_blocks(manifest);
        assert_eq!(
            blocks.get("src/main.rs").map(String::as_str),
            Some("quorrafs")
        );
    }
}
