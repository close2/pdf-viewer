//! The fifth sweep: every `pub fn` one crate answers with, against everything that could ask it.
//!
//! # The shape it exists for
//!
//! The other sweeps ask what a ledger row *claims*. This one asks the question from the other
//! end, and the two-hundred-and-fifty-third and -fourth sessions found two clauses neither of the
//! others could see: §12.5.6.19's `/H` was `implemented`, argued in an ADR and tested with pixels,
//! while `viewer-core` took the pressed annotation from `link_at`, so no host could press a widget
//! for a hundred and fifteen sessions; §8.11.4.3's `/ListMode` was read into
//! `OptionalContent::list_mode` and asked by nothing, with a layer panel on the screen. **A
//! capability can reach the crate that implements a clause and never reach a program**, because
//! the code and its callers do not cite each other any more than two clauses do. It has produced
//! a finding on most of its runs since — `Signature::must_cover_whole_file`,
//! `Collection::initial_document`, `Attachment::checksum_matches` — and `doc/todo/01` records each.
//!
//! # Why it is a program now, and it is the only sweep whose *number* is the finding
//!
//! Every run of it has been a script written for that session, and the counts have therefore
//! never been comparable: the four-hundred-and-eighth recorded 246 `pub fn`s and 85 unnamed where
//! the four-hundred-and-fifth's script said 246 and 86 over the same crates at the same commit,
//! and the five-hundred-and-seventeenth printed 101 and 82 against the five-hundred-and-tenth's 92
//! and 77 with the population unchanged at 286. **What this sweep produces is a delta** — the
//! four-hundred-and-eighth's finding was that a whole new host program took *zero* names off the
//! list, and the four-hundred-and-thirteenth's that four more took exactly one — and a delta
//! cannot be read off two different instruments. The population is derived here, once, so that the
//! next run's number means something beside this one's.
//!
//! # What a program settles that the grep could not
//!
//! - **Who could possibly be a caller is a question for the manifests.** The by-hand runs grepped
//!   a list of host crates typed into the script, which grew from two to four to five to eight as
//!   hosts arrived and which nothing maintained. Here a consumer is a crate whose `Cargo.toml`
//!   names the answering crate, so the population maintains itself — and a crate that names it in
//!   `[dev-dependencies]` only, as `render-quorra` does, cannot call it from `src/` at all, which
//!   is a false positive no grep over crate directories can drop.
//! - **The three known populations are rungs rather than prose.** Every run has ended by sorting
//!   its unnamed names into "functions `pdf-model` calls itself", "functions only a test or an
//!   example reaches" and "functions nothing names at all"; [`Reach`] is that sorting, and the
//!   report prints the rungs in the order they are worth reading.
//! - **A tool is a consumer, and this sweep could not see one for 176 sessions.** `logical_text`
//!   read as unnamed from the two-hundred-and-fifty-third run onward while `tools/pdf-retrieve`
//!   had asked it since the four-hundred-and-twenty-first. [`crate::SOURCE_ROOTS`] is the
//!   population here, so `tools/` and `fuzz/` are asked with the crates.
//!
//! # The noise, printed rather than filtered
//!
//! - **A short name shared with another type's method reads as named.** `read`, `new` and `page`
//!   are matched as identifiers rather than as calls, so a file that calls somebody else's `read`
//!   answers for this one. That is the loose direction on purpose: a name reported as *unnamed* is
//!   then genuinely absent from every file that could call it, which is the half a reading acts on.
//! - **A name reached through a wrapper reads as unnamed.** `document_part::first_page` is reached
//!   by every `GoToDp` through `DocumentPartJump::page_in`, and the four-hundred-and-second
//!   recorded that this sweep cannot see it.
//! - **An example is not a host**, which is the right default and cost the three-hundred-and-
//!   eighty-sixth a false positive on `Collection::all_folders`. It is a rung here rather than a
//!   silence.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument. Most of the names below the top rung are helpers that happen to be
//! `pub`, and which of them is a clause's noun is the reading — `CLAUDE.md`'s own rule, since a
//! build that failed on an unreached `pub fn` would be failing on a judgement about the standard.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

/// The crate the by-hand runs have always swept, and this sweep's default.
///
/// It is the crate that implements clauses 7 to 14 and answers no window, so a requirement it
/// executes reaches a person only through somebody else's call.
pub const ANSWERING: &str = "pdf-model";

/// How far from the answering crate the nearest name of a function is.
///
/// The rungs are ordered by *who asks the question*, and the report reads them from the bottom:
/// the three below [`Reach::ToolOrFuzz`] are the three populations every by-hand run has sorted
/// its output into by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reach {
    /// Nothing in the tree names it but its own definition.
    Nothing,
    /// Only a test, an example or a benchmark — the tree's own scaffolding, which asks a
    /// question rather than carrying one for a person.
    TestOrExample,
    /// Only the answering crate's own `src/`. The commonest explanation there is: a `pub fn`
    /// that is an internal helper, which the sweep cannot tell from an entry point by its name.
    Model,
    /// A tool or a fuzz target. A tool is a consumer — `tools/pdf-retrieve` is the only caller
    /// of several of §14.8's readers — and a fuzz target is a program that runs the code.
    ToolOrFuzz,
    /// Another crate's `src/`: the rung that means a program can reach the clause at all.
    Dependent,
}

impl Reach {
    /// The rung's name, as the report prints it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nothing => "named by nothing",
            Self::TestOrExample => "named only by a test or an example",
            Self::Model => "named only inside its own crate",
            Self::ToolOrFuzz => "named by a tool or a fuzz target",
            Self::Dependent => "named by a dependent crate",
        }
    }
}

impl fmt::Display for Reach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How a crate's manifest names the answering crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dependency {
    /// An ordinary or build dependency: the crate's own sources can call it.
    Normal,
    /// A dev-dependency: only its tests, examples and benchmarks can. `render-quorra` names
    /// `pdf-model` this way, so a match in its `src/` is a word rather than a call.
    Dev,
}

/// One crate of this workspace, as this sweep needs it.
#[derive(Debug, Clone)]
pub struct Consumer {
    /// The directory, relative to the workspace root: `crates/viewer-core`, `fuzz`.
    pub directory: String,
    /// How its manifest names the answering crate.
    pub dependency: Dependency,
}

/// One `pub fn` the answering crate states, by name.
///
/// The population is *distinct names* rather than definitions, because that is what a caller
/// writes: two `read`s in two modules are one question to grep for, and every by-hand run has
/// counted them that way.
#[derive(Debug, Clone)]
pub struct Function {
    /// The bare name, which is what a caller writes.
    pub name: String,
    /// Where it is defined, qualified by the `impl` block it sits in where there is one —
    /// `ViewState::clear_field`, `attachment::checksum_matches`. For the reader; the matching
    /// is done on [`Self::name`].
    pub declared: Vec<String>,
    /// The nearest rung anything names it from.
    pub reach: Reach,
    /// The first file at that rung, so that a hit can be read rather than believed.
    pub witness: Option<String>,
}

/// What one run found.
#[derive(Debug, Clone)]
pub struct Report {
    /// Every distinct `pub fn` name, ordered by rung and then by name.
    pub functions: Vec<Function>,
    /// The crates whose manifests name the answering crate, the answering crate excluded.
    pub consumers: Vec<Consumer>,
}

impl Report {
    /// How many names sit on one rung.
    #[must_use]
    pub fn on(&self, reach: Reach) -> usize {
        self.functions
            .iter()
            .filter(|function| function.reach == reach)
            .count()
    }

    /// How many names no crate under `crates/` asks — the number every by-hand run recorded as
    /// "named by no host", and the one whose delta is this sweep's finding.
    #[must_use]
    pub fn unasked_by_a_crate(&self) -> usize {
        self.functions
            .iter()
            .filter(|function| function.reach < Reach::Dependent)
            .count()
    }
}

/// Where one crate of this workspace lives, relative to its root.
///
/// A directory under [`crate::SOURCE_ROOTS`] holding a `Cargo.toml` and named for the crate. The
/// answer is looked up rather than assumed because `tools/` holds crates too, and a sweep of a
/// tool's public surface is the same question this one asks of `pdf-model`.
#[must_use]
pub fn directory_of(root: &Path, name: &str) -> Option<String> {
    crate::SOURCE_ROOTS
        .iter()
        .map(|source_root| format!("{source_root}/{name}"))
        .chain(std::iter::once(name.to_owned()))
        .find(|directory| root.join(directory).join("Cargo.toml").is_file())
}

/// Reads the workspace's manifests for the crates that could call `answering`.
///
/// A directory under [`crate::SOURCE_ROOTS`] holding a `Cargo.toml` is a crate; the roots
/// themselves are checked too, because `fuzz/` is one crate rather than a directory of them.
///
/// # Errors
///
/// If a directory cannot be walked or a manifest cannot be read. A sweep that skipped a manifest
/// it could not open would report a caller-less function for a crate it had not looked at.
pub fn consumers(root: &Path, answering: &str) -> std::io::Result<Vec<Consumer>> {
    let mut found = Vec::new();
    for name in crate::SOURCE_ROOTS {
        let source_root = root.join(name);
        let mut directories = vec![source_root.clone()];
        if source_root.is_dir() {
            for entry in std::fs::read_dir(&source_root)? {
                let path = entry?.path();
                if path.is_dir() {
                    directories.push(path);
                }
            }
        }
        for directory in directories {
            let manifest = directory.join("Cargo.toml");
            if !manifest.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&manifest)?;
            let shown = shown(directory.strip_prefix(root).unwrap_or(&directory));
            if shown.ends_with(answering) {
                continue;
            }
            if let Some(dependency) = dependency_on(&text, answering) {
                found.push(Consumer {
                    directory: shown,
                    dependency,
                });
            }
        }
    }
    found.sort_by(|left, right| left.directory.cmp(&right.directory));
    Ok(found)
}

/// How a manifest names one dependency, if it names it at all.
///
/// The ledger's own reason for reading TOML by hand applies here (`toml_subset`): the shapes a
/// manifest states a dependency in are `name.workspace = true`, `name = { … }` and a
/// `[dependencies.name]` table, and which section the line is in is the whole question.
#[must_use]
pub fn dependency_on(manifest: &str, name: &str) -> Option<Dependency> {
    let mut section = Dependency::Normal;
    let mut in_dependencies = false;
    let mut found: Option<Dependency> = None;
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(heading) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            let heading = heading.trim_start_matches('[').trim_end_matches(']');
            in_dependencies = heading.ends_with("dependencies")
                || heading.contains("dependencies.")
                || heading.ends_with(&format!("dependencies.{name}"));
            section = if heading.contains("dev-dependencies") {
                Dependency::Dev
            } else {
                Dependency::Normal
            };
            if in_dependencies && heading.ends_with(&format!("dependencies.{name}")) {
                found = Some(promote(found, section));
            }
            continue;
        }
        if !in_dependencies || line.starts_with('#') {
            continue;
        }
        let key = line.split(['=', '.', ' ']).next().unwrap_or_default();
        if key == name {
            found = Some(promote(found, section));
        }
    }
    found
}

/// A crate depending on the answering crate twice takes the wider of the two: `[dependencies]`
/// beats `[dev-dependencies]`, because what the sweep asks is what the crate's `src/` may call.
fn promote(found: Option<Dependency>, section: Dependency) -> Dependency {
    match (found, section) {
        (Some(Dependency::Normal), _) | (_, Dependency::Normal) => Dependency::Normal,
        _ => Dependency::Dev,
    }
}

/// One definition, as [`definitions`] reads it off a line.
#[derive(Debug, Clone)]
struct Definition {
    name: String,
    qualified: String,
}

/// Every `pub fn` a crate's `src/` states, in file order.
///
/// `pub(crate)` and `pub(super)` are not public surface and are not read: the question is what
/// somebody outside the crate could call. The `impl` block a definition sits in is tracked for
/// the *report* rather than for the matching — a qualified name is what makes
/// `ViewState::clear_field` legible where `clear_field` is not — and it is a heuristic on the
/// nearest preceding `impl` line, which is what a reader of the output needs it to be.
fn definitions(lines: &[&str]) -> Vec<Definition> {
    let mut found = Vec::new();
    let mut context: Option<String> = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("impl") {
            context = implemented_type(trimmed);
        }
        let Some(name) = public_function_name(trimmed) else {
            continue;
        };
        let qualified = match context.as_deref() {
            Some(kind) => format!("{kind}::{name}"),
            None => name.clone(),
        };
        found.push(Definition { name, qualified });
    }
    found
}

/// The name a line defines, where the line defines a public function.
fn public_function_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("pub ")?;
    let rest = rest
        .strip_prefix("const ")
        .or_else(|| rest.strip_prefix("async "))
        .or_else(|| rest.strip_prefix("unsafe "))
        .unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// The type an `impl` line implements for, generics and traits stripped.
fn implemented_type(line: &str) -> Option<String> {
    let body = line.trim_end_matches('{').trim();
    let after = body.rsplit(" for ").next().unwrap_or(body);
    let after = after
        .strip_prefix("impl")
        .map_or(after, |rest| rest.trim_start_matches(char::is_whitespace));
    let after = after.strip_prefix('<').map_or(after, |_| {
        after.split_once('>').map_or(after, |(_, rest)| rest.trim())
    });
    let name: String = after
        .trim_start()
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Every identifier a file's *code* writes, with the names it defines left out.
///
/// Two exclusions, and both are the question the sweep asks rather than tidiness:
///
/// - **A definition names itself**, so a file's own `pub fn read` would answer "does anything
///   name `read`" for the crate that defines it. The token after `fn` is dropped for that reason
///   and no other — every other occurrence on the same line is kept, because a default argument
///   or a returned closure is a use like any other.
/// - **A comment is not a caller.** A doc comment naming a neighbouring function is how this tree
///   explains itself, and counting it would have put `Collection::all_folders` — whose only
///   caller is an example — on the rung that means its own crate uses it. The by-hand grep counted
///   both and the distinction had to be made by opening the file.
fn identifiers(lines: &[&str]) -> HashSet<String> {
    let mut found = HashSet::new();
    for line in lines {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let mut after_fn = false;
        for token in
            line.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        {
            if token.is_empty() {
                continue;
            }
            if after_fn {
                after_fn = false;
                continue;
            }
            if token == "fn" {
                after_fn = true;
                continue;
            }
            found.insert(token.to_owned());
        }
    }
    found
}

/// A file's lines, split into its code and its `#[cfg(test)]` items.
///
/// A unit test lives *inside* `src/` in this tree, so a sweep that read a file whole would report
/// a function as used by the crate that only tests it: `all_folders` is named twice in
/// `collection.rs` and both are its own unit tests, which is exactly the shape the
/// three-hundred-and-eighty-sixth met from the other side when an example read as no caller. The
/// split is brace counting from the attribute, which is a heuristic and is stated as one — an
/// unbalanced brace inside a string literal would move the boundary rather than lose a line.
fn split_tests(text: &str) -> (Vec<&str>, Vec<&str>) {
    let mut code = Vec::new();
    let mut tests = Vec::new();
    let mut depth = 0i32;
    let mut in_tests = false;
    for line in text.lines() {
        if !in_tests && line.trim_start().starts_with("#[cfg(test)]") {
            in_tests = true;
            depth = 0;
            continue;
        }
        if in_tests {
            tests.push(line);
            let opened = i32::try_from(line.matches('{').count()).unwrap_or(0);
            let closed = i32::try_from(line.matches('}').count()).unwrap_or(0);
            depth = depth.saturating_add(opened).saturating_sub(closed);
            if depth <= 0 && closed > 0 {
                in_tests = false;
            }
        } else {
            code.push(line);
        }
    }
    (code, tests)
}

/// Whether a path is a crate's own program source rather than its scaffolding.
///
/// `src/` and — because a fuzz target is a program that runs the code rather than a test of it —
/// `fuzz_targets/`. `tests/`, `examples/`, `benches/` and a `build.rs` are the other side.
fn is_program_source(relative: &str) -> bool {
    relative.starts_with("src/") || relative.starts_with("fuzz_targets/")
}

/// Runs the sweep.
///
/// `sources` are the Rust files under [`crate::SOURCE_ROOTS`] with their text, as
/// [`crate::entries::sources`] reads them, and `consumers` what [`consumers`] found. `answering`
/// is the crate directory whose `pub fn`s are the population — `crates/pdf-model`.
#[must_use]
pub fn sweep(answering: &str, sources: &[(PathBuf, String)], consumers: &[Consumer]) -> Report {
    let mut declared: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut reach: BTreeMap<String, (Reach, Option<String>)> = BTreeMap::new();
    let mut asked: Vec<(Reach, String, HashSet<String>)> = Vec::new();

    for (path, text) in sources {
        let shown = shown(path);
        if shown.starts_with(crate::NOT_SCANNED) {
            continue;
        }
        let (code, in_file_tests) = split_tests(text);
        let own = shown
            .strip_prefix(answering)
            .and_then(|rest| rest.strip_prefix('/'));
        if let Some(relative) = own {
            if is_program_source(relative) {
                for definition in definitions(&code) {
                    declared
                        .entry(definition.name.clone())
                        .or_default()
                        .insert(definition.qualified);
                    reach
                        .entry(definition.name)
                        .or_insert((Reach::Nothing, None));
                }
                asked.push((Reach::Model, shown.clone(), identifiers(&code)));
                asked.push((Reach::TestOrExample, shown, identifiers(&in_file_tests)));
            } else {
                asked.push((Reach::TestOrExample, shown, identifiers(&code)));
            }
            continue;
        }
        let Some(consumer) = consumers
            .iter()
            .find(|consumer| shown.starts_with(&format!("{}/", consumer.directory)))
        else {
            continue;
        };
        let relative = shown
            .get(consumer.directory.len().saturating_add(1)..)
            .unwrap_or_default()
            .to_owned();
        let rung = if is_program_source(&relative) {
            match consumer.dependency {
                Dependency::Normal if consumer.directory.starts_with("crates/") => Reach::Dependent,
                Dependency::Normal => Reach::ToolOrFuzz,
                // A dev-dependency cannot be called from `src/`, so a match there is a word.
                Dependency::Dev => continue,
            }
        } else {
            Reach::TestOrExample
        };
        asked.push((rung, shown.clone(), identifiers(&code)));
        asked.push((Reach::TestOrExample, shown, identifiers(&in_file_tests)));
    }

    for (rung, path, names) in asked {
        for (name, found) in &mut reach {
            if rung > found.0 && names.contains(name) {
                *found = (rung, Some(path.clone()));
            }
        }
    }

    let mut functions: Vec<Function> = reach
        .into_iter()
        .map(|(name, (rung, witness))| Function {
            declared: declared
                .get(&name)
                .map(|names| names.iter().cloned().collect())
                .unwrap_or_default(),
            name,
            reach: rung,
            witness,
        })
        .collect();
    functions.sort_by(|left, right| {
        left.reach
            .cmp(&right.reach)
            .then_with(|| left.name.cmp(&right.name))
    });
    Report {
        functions,
        consumers: consumers.to_vec(),
    }
}

/// A path as it is printed and compared: relative, with forward slashes.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_owned())
    }

    fn consumer(directory: &str, dependency: Dependency) -> Consumer {
        Consumer {
            directory: directory.to_owned(),
            dependency,
        }
    }

    /// The sweep's own subject: a `pub fn` implementing a clause that no program asks.
    #[test]
    fn a_function_no_dependent_crate_names_falls_below_the_top_rung() {
        let sources = vec![
            file(
                "crates/pdf-model/src/collection.rs",
                "impl Collection {\n    pub fn initial_document(&self) -> u8 { 0 }\n}\n",
            ),
            file(
                "crates/viewer-core/src/open.rs",
                "fn open() { let _ = all_folders(); }\n",
            ),
        ];
        let consumers = vec![consumer("crates/viewer-core", Dependency::Normal)];
        let report = sweep("crates/pdf-model", &sources, &consumers);
        let function = report.functions.first().expect("one function");
        assert_eq!(function.name, "initial_document");
        assert_eq!(function.declared, ["Collection::initial_document"]);
        assert_eq!(function.reach, Reach::Nothing);
        assert_eq!(report.unasked_by_a_crate(), 1);
    }

    /// A dependent crate's `src/` is the rung that means a program can reach the clause.
    #[test]
    fn a_dependent_crates_source_is_the_top_rung() {
        let sources = vec![
            file(
                "crates/pdf-model/src/optional_content.rs",
                "pub fn list_mode() {}\n",
            ),
            file(
                "crates/viewer-ui/src/chrome.rs",
                "fn draw() { list_mode(); }\n",
            ),
        ];
        let consumers = vec![consumer("crates/viewer-ui", Dependency::Normal)];
        let report = sweep("crates/pdf-model", &sources, &consumers);
        let function = report.functions.first().expect("one function");
        assert_eq!(function.reach, Reach::Dependent);
        assert_eq!(
            function.witness.as_deref(),
            Some("crates/viewer-ui/src/chrome.rs")
        );
    }

    /// A tool is a consumer, and this sweep could not see one for 176 sessions: `logical_text`
    /// read as unnamed while `tools/pdf-retrieve` had asked it for eight rounds.
    #[test]
    fn a_tool_is_a_rung_of_its_own() {
        let sources = vec![
            file("crates/pdf-model/src/text.rs", "pub fn logical_text() {}\n"),
            file(
                "tools/pdf-retrieve/src/lib.rs",
                "fn find() { logical_text(); }\n",
            ),
        ];
        let consumers = vec![consumer("tools/pdf-retrieve", Dependency::Normal)];
        let report = sweep("crates/pdf-model", &sources, &consumers);
        assert_eq!(report.on(Reach::ToolOrFuzz), 1);
        assert_eq!(report.unasked_by_a_crate(), 1);
    }

    /// An example is not a host — the right default, and a rung here rather than a silence.
    #[test]
    fn an_example_is_a_rung_below_a_program() {
        let sources = vec![
            file(
                "crates/pdf-model/src/collection.rs",
                "pub fn all_folders() {}\n",
            ),
            file(
                "crates/viewer-confined/examples/confined_panels.rs",
                "fn main() { all_folders(); }\n",
            ),
        ];
        let consumers = vec![consumer("crates/viewer-confined", Dependency::Normal)];
        let report = sweep("crates/pdf-model", &sources, &consumers);
        assert_eq!(report.on(Reach::TestOrExample), 1);
    }

    /// The definition names itself, and a sweep that counted that would report every function
    /// as reached by its own crate.
    #[test]
    fn a_definition_is_not_a_call() {
        let sources = vec![file(
            "crates/pdf-model/src/lib.rs",
            "pub fn checksum_matches() -> bool { true }\n",
        )];
        let report = sweep("crates/pdf-model", &sources, &[]);
        assert_eq!(report.functions.first().expect("one").reach, Reach::Nothing);
    }

    /// The answering crate calling its own function is an explanation the report gives by name:
    /// `unresolved_usage` is read by `content.rs`, and every run has had to sort that out by hand.
    #[test]
    fn the_crate_calling_itself_is_its_own_rung() {
        let sources = vec![
            file(
                "crates/pdf-model/src/usage.rs",
                "pub fn unresolved_usage() {}\n",
            ),
            file(
                "crates/pdf-model/src/content.rs",
                "fn interpret() { unresolved_usage(); }\n",
            ),
        ];
        let report = sweep("crates/pdf-model", &sources, &[]);
        assert_eq!(report.on(Reach::Model), 1);
    }

    /// `pub(crate)` is not public surface: the question is what somebody outside could call.
    #[test]
    fn a_crate_visible_function_is_not_in_the_population() {
        let sources = vec![file(
            "crates/pdf-model/src/lib.rs",
            "pub(crate) fn helper() {}\npub const fn version() -> u8 { 2 }\n",
        )];
        let report = sweep("crates/pdf-model", &sources, &[]);
        assert_eq!(report.functions.len(), 1);
        assert_eq!(report.functions.first().expect("one").name, "version");
    }

    /// A dev-dependency cannot be called from `src/`, which `render-quorra` is the tree's own
    /// instance of — a match there is a word rather than a call.
    #[test]
    fn a_dev_dependency_cannot_call_from_its_own_source() {
        let sources = vec![
            file("crates/pdf-model/src/page.rs", "pub fn page_group() {}\n"),
            file(
                "crates/render-quorra/src/lib.rs",
                "// page_group is drawn here\n",
            ),
            file(
                "crates/render-quorra/tests/corpus.rs",
                "fn t() { page_group(); }\n",
            ),
        ];
        let consumers = vec![consumer("crates/render-quorra", Dependency::Dev)];
        let report = sweep("crates/pdf-model", &sources, &consumers);
        assert_eq!(report.on(Reach::TestOrExample), 1);
    }

    /// The three shapes a manifest states a dependency in, and the section it is in deciding
    /// which of the two kinds it is.
    #[test]
    fn a_manifest_states_a_dependency_three_ways() {
        assert_eq!(
            dependency_on("[dependencies]\npdf-model.workspace = true\n", "pdf-model"),
            Some(Dependency::Normal)
        );
        assert_eq!(
            dependency_on(
                "[dependencies]\npdf-model = { path = \"../crates/pdf-model\" }\n",
                "pdf-model"
            ),
            Some(Dependency::Normal)
        );
        assert_eq!(
            dependency_on(
                "[dev-dependencies]\npdf-model.workspace = true\n",
                "pdf-model"
            ),
            Some(Dependency::Dev)
        );
        assert_eq!(
            dependency_on("[dependencies]\npdf-syntax.workspace = true\n", "pdf-model"),
            None
        );
    }

    /// A crate depending on it both ways takes the wider of the two, because what the sweep
    /// asks is what the crate's `src/` may call.
    #[test]
    fn a_dependency_stated_twice_takes_the_wider_kind() {
        assert_eq!(
            dependency_on(
                "[dependencies]\npdf-model.workspace = true\n\n[dev-dependencies]\n\
                 pdf-model.workspace = true\n",
                "pdf-model"
            ),
            Some(Dependency::Normal)
        );
    }

    /// A definition is qualified by the `impl` block it sits in, so that the report says
    /// `ViewState::clear_field` where the name alone says nothing.
    #[test]
    fn a_definition_is_qualified_by_its_impl_block() {
        let text = "impl<'a> ViewState<'a> {\n    pub fn clear_field(&mut self) {}\n}\n\
                    impl fmt::Display for Reach {\n    pub fn as_str(&self) {}\n}\n";
        let lines: Vec<&str> = text.lines().collect();
        let found = definitions(&lines);
        let names: Vec<&str> = found.iter().map(|one| one.qualified.as_str()).collect();
        assert_eq!(names, ["ViewState::clear_field", "Reach::as_str"]);
    }

    /// A unit test lives inside `src/` in this tree, so a file read whole would report a
    /// function as used by the crate that only tests it — which is what `all_folders` did.
    #[test]
    fn a_unit_test_inside_a_source_file_is_the_scaffolding_rung() {
        let sources = vec![file(
            "crates/pdf-model/src/collection.rs",
            "pub fn all_folders() {}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    \
             #[test]\n    fn folders() { all_folders(); }\n}\n",
        )];
        let report = sweep("crates/pdf-model", &sources, &[]);
        assert_eq!(report.on(Reach::TestOrExample), 1);
        assert_eq!(report.on(Reach::Model), 0);
    }

    /// The split ends where the test module's braces balance, so code written after one is
    /// code again.
    #[test]
    fn the_test_split_ends_with_the_modules_braces() {
        let (code, tests) = split_tests(
            "fn before() {}\n#[cfg(test)]\nmod tests {\n    fn inside() {}\n}\nfn after() {}\n",
        );
        assert_eq!(code, ["fn before() {}", "fn after() {}"]);
        assert_eq!(tests.len(), 3);
    }
}
