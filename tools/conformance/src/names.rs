//! The twenty-sixth sweep: a Rust path a doc comment names, against the items this tree declares.
//!
//! # The shape it exists for
//!
//! `Edit::ChooseFile`'s doc comment said the host decides the request in
//! `viewer_host::policy::may_choose_file`. No such function existed. It was written when the
//! variant was, read by every round that opened the file, and nothing in this project could see
//! it: the citation gate checks a `§` against the standard, [`crate::pointers`] checks a *file
//! path* a note names and the symbol half of a `file.rs::item` pointer, and the ledger resolves
//! the `code` and `test` arrays' sites. **A Rust path written in prose — `` `crate::edit::apply`
//! ``, `` `Interpretation::text` `` — was read by nothing at all**, and it is the commonest way
//! this tree names its own code to a reader.
//!
//! A path in prose decays in both directions and neither leaves a mark: the function is renamed
//! and the sentence keeps the old name, or the sentence is written for code a round did not get
//! to and the name is a promise nobody kept. Both read exactly like a working reference, which is
//! what makes the second one expensive — a later round trusts the sentence and builds on a
//! capability that is not there.
//!
//! # Why rustdoc does not already do this
//!
//! Two thirds of these paths are written as intra-doc links, `` [`crate::edit::apply`] ``, and
//! `rustdoc` resolves those — under `cargo doc`, which is in none of `doc/todo/02` §2's three
//! tiers, so nothing runs it. The other third is written in plain backticks, which `rustdoc` does
//! not resolve at all and never will: a backticked span is code formatting, not a link. This
//! reads both populations the same way, because a reader does.
//!
//! # What resolution means here, and why it is deliberately loose
//!
//! **The last two segments are the whole of the question.** For `` `a::b::c` `` this asks whether
//! some item named `c` is declared in a context named `b` — a module, a type an `impl` block
//! names, a trait, an enum, a struct, or a crate. It does not check that `a` contains `b`, and it
//! does not check visibility, generics or re-exports. A stricter reader would need Rust's own name
//! resolution, and a sweep that half-implements name resolution reports its own gaps as this
//! tree's defects.
//!
//! One-segment paths are not checked at all: `` `Document` `` names a type without saying where,
//! and there is no second half to check it against.
//!
//! # The three things that are not findings, excluded mechanically rather than by a list
//!
//! Each is counted and printed, so that a clean run says what it was clean over:
//!
//! - **A generic prefix** ([`Reach::Generic`]) — `Self::`, `self::`, `super::`, `crate::` as the
//!   *immediate* prefix, or a bare type parameter. `Self::text` names whatever type the enclosing
//!   `impl` is for, and answering that question is name resolution again.
//! - **Another crate's** ([`Reach::Foreign`]) — the path opens with, or its prefix is, a name the
//!   workspace manifest declares as a dependency rather than a member, `std`, `core`, `alloc`, or
//!   a primitive type. A crate this tree re-exports is still that crate's:
//!   `raster_gpu::wgpu::SurfaceTarget` is `wgpu`'s. The set is derived from the manifest: a dependency added is excluded without
//!   this file being edited (trap 25).
//! - **A prefix this tree never declares** ([`Reach::Undeclared`]) — `ControlFlow::Wait`,
//!   `OwnedFd::drop`, `Qt::Key`. These are the same population as the one above reached without
//!   its crate name in front, and the discriminator is that **the prefix has to be something this
//!   workspace declares** before its contents are anybody's business here. It is the rung that
//!   makes the sweep readable: without it a third of the hits are the standard library's.
//!
//! What is left is [`Reach::Absent`]: a prefix this tree declares, and a name it does not declare
//! under it. That is the finding, and it is the population the gate holds at zero.
//!
//! # Why it is a gate and [`crate::pointers`] is not
//!
//! A dead *file* pointer is sometimes the right thing to write — a round may cite the file it is
//! about to add, and a correction has to quote the pointer it retired. A Rust path is not like
//! that: this project's comment rule says a comment carries the current reason and nothing about
//! how it got there, so a comment has no business quoting a name that was retired, and a name a
//! round is about to add is a name that round is adding. The population is zero and there is no
//! version of this tree in which a doc comment naming an item nobody declares is acceptable.
//!
//! ADR 1273.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::citation;
use crate::roots;

/// The empty set, for a crate whose reachable set was never derived.
static EMPTY: BTreeSet<String> = BTreeSet::new();

/// Why the sweep could not be run.
///
/// Every variant is a refusal. A sweep that skipped what it could not read would report a clean
/// tree for a tree it had not looked at, which is the condition it exists to end.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The tree's shape could not be derived from the workspace manifest.
    #[error(transparent)]
    Roots(#[from] roots::Error),
    /// A source file or directory could not be read.
    #[error("{path} could not be read: {source}")]
    Unreadable {
        /// What was being read.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
}

/// What a path's prefix resolved to.
///
/// The order is the report's: the finding first, then the three shapes that are counted rather
/// than listed, then the resolved majority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reach {
    /// The prefix names something this tree declares, and the last segment names nothing under
    /// it. The finding.
    Absent,
    /// The prefix is `Self`, `self`, `super`, `crate` or a type parameter — a name whose meaning
    /// is the item it is written inside.
    Generic,
    /// The path opens with a dependency's name, `std`, `core`, `alloc`, or a primitive type.
    Foreign,
    /// The prefix names nothing this workspace declares, so the path is another library's
    /// reached without its crate name in front.
    Undeclared,
    /// The last segment is declared in a context the prefix names.
    Resolved,
}

/// One backticked Rust path, where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    /// The file holding the doc comment, relative to the tree's root.
    pub path: PathBuf,
    /// The 1-based line it is written on.
    pub line: usize,
    /// The path, as written.
    pub named: String,
    /// What the prefix resolved to.
    pub reach: Reach,
    /// The comment line, trimmed — one line of context tells a reference from an example.
    pub text: String,
}

/// Every name this workspace declares, and where.
///
/// The map is from a declared name to the contexts it is declared in: the enclosing `mod`,
/// `impl`, `trait`, `enum`, `struct` or variant block, the file's own module name, and the
/// crate's name. A name declared at the top of `crates/pdf-model/src/icc.rs` would be reachable
/// as `icc::name` and as `pdf_model::name`, and both spellings are what this tree writes.
#[derive(Debug, Clone, Default)]
pub struct Index {
    declared: BTreeMap<(String, String), BTreeSet<String>>,
    contexts: BTreeSet<(String, String)>,
    crates: BTreeSet<String>,
    reachable: BTreeMap<String, BTreeSet<String>>,
    external: BTreeSet<String>,
    /// How many source files were read to build it.
    pub files_read: usize,
}

/// What one file's `use` statements bring into it from outside this tree.
#[derive(Debug, Clone, Default)]
pub struct Imports {
    /// Names brought in from a dependency, `std`, `core` or `alloc`.
    foreign: BTreeSet<String>,
}

impl Index {
    /// What a path's prefix and last segment resolve to, read in a crate that imports nothing.
    #[must_use]
    pub fn reach_of(&self, path: &str, home: &str) -> Reach {
        self.reach_of_in(path, home, &Imports::default())
    }

    /// What a path's prefix and last segment resolve to, read in the crate and the file they are
    /// written in.
    ///
    /// `path` is the whole backticked span; only its last two segments decide, for the reason the
    /// module comment gives. **Where** the prefix is looked for is the crate the comment is
    /// written in and the workspace crates that one depends on: `viewer-ui` declares a `Surface`
    /// and `render-raster` does not depend on `viewer-ui`, so a `` `Surface::configure` `` written
    /// there is `wgpu`'s and no claim about this tree at all.
    ///
    /// Once the prefix *is* something the comment's crate can reach, the last segment is looked
    /// for under that prefix **anywhere in the workspace**, which is the loose direction the
    /// module comment states: `pdf-model` re-exports `pdf-colour` as `colour`, and following a
    /// re-exported module to the crate that fills it is name resolution again.
    #[must_use]
    pub fn reach_of_in(&self, path: &str, home: &str, imports: &Imports) -> Reach {
        let segments: Vec<&str> = path.split("::").collect();
        let Some((last, rest)) = segments.split_last() else {
            return Reach::Generic;
        };
        let Some(prefix) = rest.last() else {
            return Reach::Generic;
        };
        let head = segments.first().copied().unwrap_or_default();
        if self.external.contains(head) || imports.foreign.contains(head) {
            return Reach::Foreign;
        }
        if is_generic(prefix) {
            return Reach::Generic;
        }
        if imports.foreign.contains(*prefix) || self.external.contains(*prefix) {
            return Reach::Foreign;
        }
        let within = self.reachable.get(home).unwrap_or(&EMPTY);
        // A path of three segments or more opens with a namespace, and a namespace this tree
        // neither is nor declares belongs to a library reached through a re-export — `vello`
        // hands `peniko` on, and `peniko::Compose::Copy` is `peniko`'s wherever it is written.
        if rest.len() > 1
            && !self.crates.contains(head)
            && !RELATIVE.contains(&head)
            && !within.iter().any(|crate_name| {
                self.contexts
                    .contains(&(head.to_owned(), crate_name.clone()))
            })
        {
            return Reach::Undeclared;
        }
        if !within.iter().any(|crate_name| {
            self.contexts
                .contains(&((*prefix).to_owned(), crate_name.clone()))
        }) {
            return Reach::Undeclared;
        }
        if self.declares(prefix, last) {
            Reach::Resolved
        } else {
            Reach::Absent
        }
    }

    /// Whether any crate of this workspace declares `name` under `context`.
    fn declares(&self, context: &str, name: &str) -> bool {
        self.declared
            .range((context.to_owned(), String::new())..)
            .take_while(|((held, _), _)| held == context)
            .any(|(_, names)| names.contains(name))
    }

    /// What one file's `use` statements bring into it from outside this tree.
    #[must_use]
    pub fn imports_of(&self, text: &str) -> Imports {
        let mut imports = Imports::default();
        for (head, leaf) in use_leaves(text) {
            if self.external.contains(&head) {
                imports.foreign.insert(leaf);
            }
        }
        imports
    }
}

/// Everything one run found.
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// The index the mentions were resolved against.
    pub index: Index,
    /// Every mention, in file order.
    pub mentions: Vec<Mention>,
    /// How many doc-comment lines were read.
    pub comment_lines: usize,
}

impl Found {
    /// The mentions on one rung.
    #[must_use]
    pub fn on(&self, reach: Reach) -> Vec<&Mention> {
        self.mentions
            .iter()
            .filter(|mention| mention.reach == reach)
            .collect()
    }
}

/// The names whose meaning is the item they are written inside.
///
/// Stated rather than quietly skipped. A single uppercase letter is a type parameter by this
/// tree's own convention and is handled beside them, in [`is_generic`].
pub const RELATIVE: [&str; 4] = ["Self", "self", "super", "crate"];

/// The names of Rust's primitive types, which no manifest declares and which carry associated
/// items a doc comment reaches for — `f32::EPSILON`, `u32::MAX`.
pub const PRIMITIVES: [&str; 17] = [
    "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "str", "u8", "u16",
    "u32", "u64", "u128", "usize",
];

/// The crates every Rust program has without declaring a dependency on them.
pub const IMPLICIT: [&str; 3] = ["std", "core", "alloc"];

/// Whether a prefix's meaning is the item it is written inside.
#[must_use]
pub fn is_generic(prefix: &str) -> bool {
    RELATIVE.contains(&prefix)
        || (prefix.len() == 1 && prefix.starts_with(|first: char| first.is_ascii_uppercase()))
}

/// Builds the index over every Rust source under [`roots::source_roots`].
///
/// # Errors
///
/// If the manifest or a source cannot be read.
pub fn index(root: &Path) -> Result<Index, Error> {
    let mut index = Index {
        external: external_crates(root)?,
        ..Index::default()
    };
    let read = sources(root)?;
    for (path, _) in &read {
        if let Some(name) = crate_name(path) {
            index.crates.insert(name);
        }
    }
    index.reachable = reachable(root, &index.crates)?;
    for (path, text) in &read {
        let module = module_name(path);
        let home = crate_name(path).unwrap_or_else(|| module.clone());
        declare(text, &module, &home, &mut index);
        index.files_read = index.files_read.saturating_add(1);
    }
    Ok(index)
}

/// Runs the sweep: every backticked Rust path in a doc comment, resolved against the index.
///
/// # Errors
///
/// If the manifest or a source cannot be read.
pub fn sweep(root: &Path) -> Result<Found, Error> {
    let index = index(root)?;
    let mut found = Found {
        index,
        ..Found::default()
    };
    for (path, text) in sources(root)? {
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        let (mentions, lines) = mentions_in(&text);
        let module = module_name(&path);
        let home = crate_name(&path).unwrap_or(module);
        let imports = found.index.imports_of(&text);
        found.comment_lines = found.comment_lines.saturating_add(lines);
        for (line, named, context) in mentions {
            let reach = found.index.reach_of_in(&named, &home, &imports);
            found.mentions.push(Mention {
                path: relative.clone(),
                line,
                named,
                reach,
                text: context,
            });
        }
    }
    Ok(found)
}

/// The calibration: a path whose prefix this tree declares and whose last segment it does not,
/// run through the same two functions the sweep runs.
///
/// Trap 13 — a sweep that comes back clean is a sentence about the sweep until the defect has
/// been planted back. It is planted into the functions rather than into a source file, because a
/// plant written into the tree is a plant somebody has to remember to remove.
///
/// # Errors
///
/// If the index cannot be built.
pub fn calibrate(root: &Path) -> Result<Vec<(String, Reach)>, Error> {
    let index = index(root)?;
    // `ledger` is a module of this crate, so the prefix resolves; nothing declares the second
    // name anywhere, which is exactly the shape ADR 1240's defect had.
    let planted = "/// The host decides it in `ledger::no_function_of_this_name_is_declared`.\n";
    let (mentions, _) = mentions_in(planted);
    Ok(mentions
        .into_iter()
        .map(|(_, named, _)| {
            let reach = index.reach_of(&named, "conformance");
            (named, reach)
        })
        .collect())
}

/// The report, findings first and then what the run was clean over.
#[must_use]
pub fn report(found: &Found) -> String {
    let mut out = String::new();
    let absent = found.on(Reach::Absent);
    let _ = writeln!(
        out,
        "{} path(s) in {} doc comment line(s) of {} source file(s)",
        found.mentions.len(),
        found.comment_lines,
        found.index.files_read,
    );
    let _ = writeln!(
        out,
        "{} name(s) this workspace declares\n",
        found.index.declared.len(),
    );
    if absent.is_empty() {
        out.push_str("no doc comment names an item under a prefix that does not declare it\n");
    } else {
        let _ = writeln!(
            out,
            "{} path(s) whose prefix this tree declares and whose name it does not:",
            absent.len(),
        );
        for mention in absent {
            let _ = writeln!(
                out,
                "  {}:{} {}\n    {}",
                mention.path.display(),
                mention.line,
                mention.named,
                mention.text,
            );
        }
    }
    out.push('\n');
    for (reach, why) in [
        (Reach::Resolved, "resolved"),
        (Reach::Generic, "a relative or generic prefix"),
        (Reach::Foreign, "another crate's, named"),
        (Reach::Undeclared, "a prefix this tree does not declare"),
    ] {
        let _ = writeln!(out, "{:>6}  {why}", found.on(reach).len());
    }
    out
}

/// What each member of this workspace can name: itself, and the members its own manifest depends
/// on.
///
/// Derived from the manifests rather than from the imports, because a comment may name a type of
/// a crate this file does not itself import while the crate's manifest still admits it. The
/// direction matters: `pdf-model` depends on `pdf-colour`, so a comment in `pdf-model` naming
/// `colour::MAX_PRESSES` is about `pdf-colour`'s constant; `render-raster` does not depend on
/// `viewer-ui`, so a `Surface` written there cannot be `viewer-ui`'s.
fn reachable(
    root: &Path,
    crates: &BTreeSet<String>,
) -> Result<BTreeMap<String, BTreeSet<String>>, Error> {
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for member in roots::source_roots(root)? {
        let manifest = root.join(&member).join(roots::MANIFEST);
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        let Some(name) = member.rsplit('/').next().map(|name| name.replace('-', "_")) else {
            continue;
        };
        let mut within = BTreeSet::new();
        within.insert(name.clone());
        let mut in_dependencies = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_dependencies = trimmed.contains("dependencies");
                continue;
            }
            if !in_dependencies {
                continue;
            }
            let Some((key, _)) = trimmed.split_once('=') else {
                continue;
            };
            // `thing.workspace = true` and `thing = { … }` both name `thing`.
            let key = key.trim().split('.').next().unwrap_or_default().trim();
            let key = key.replace('-', "_");
            if crates.contains(&key) {
                within.insert(key);
            }
        }
        found.insert(name, within);
    }
    Ok(found)
}

/// Every Rust source under the workspace's own members, with its text.
fn sources(root: &Path) -> Result<Vec<(PathBuf, String)>, Error> {
    let directories: Vec<PathBuf> = roots::source_roots(root)?
        .iter()
        .map(|name| root.join(name))
        .collect();
    let paths = citation::rust_sources(&directories).map_err(|source| Error::Unreadable {
        path: root.display().to_string(),
        source,
    })?;
    let mut read = Vec::with_capacity(paths.len());
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Unreadable {
            path: path.display().to_string(),
            source,
        })?;
        read.push((path, text));
    }
    Ok(read)
}

/// The dependency names the workspace manifest declares without a path — everything outside this
/// tree — with the crates every program has and the primitive types beside them.
fn external_crates(root: &Path) -> Result<BTreeSet<String>, Error> {
    let path = root.join(roots::MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|source| Error::Unreadable {
        path: path.display().to_string(),
        source,
    })?;
    let mut found: BTreeSet<String> = IMPLICIT
        .iter()
        .chain(PRIMITIVES.iter())
        .map(|name| (*name).to_owned())
        .collect();
    let mut within = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            within = trimmed == "[workspace.dependencies]";
            continue;
        }
        if !within {
            continue;
        }
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        // A member of this workspace is declared with a `path`; anything else is somebody
        // else's crate. Deriving it this way means a dependency added is excluded without this
        // file being edited, and a member moved is not silently excluded with it.
        if value.contains("path =") || value.contains("path=") {
            continue;
        }
        let name = name.trim().trim_matches('"');
        if !name.is_empty() {
            found.insert(name.replace('-', "_"));
        }
    }
    Ok(found)
}

/// The module name a file declares, by Cargo's own rules: `lib.rs` and `main.rs` are the crate,
/// `mod.rs` is its directory, and anything else is its own stem.
fn module_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    if stem == "mod" {
        return path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or(stem)
            .replace('-', "_");
    }
    if stem == "lib" || stem == "main" {
        return crate_name(path).unwrap_or_else(|| stem.to_owned());
    }
    stem.replace('-', "_")
}

/// The crate a source belongs to: the directory holding the `src`, `tests`, `examples`, `benches`
/// or `build.rs` the file is under.
fn crate_name(path: &Path) -> Option<String> {
    let mut directory = path.parent();
    while let Some(here) = directory {
        let name = here.file_name().and_then(|name| name.to_str());
        if name.is_some_and(|name| {
            ["src", "tests", "examples", "benches", "fuzz_targets"].contains(&name)
        }) {
            return here
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(|name| name.replace('-', "_"));
        }
        directory = here.parent();
    }
    None
}

/// What a block of source opens, for the context stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `mod m {`, `trait T {`, `impl T {` — a context whose members a path may name.
    Block,
    /// `enum E {`, whose members are its variants.
    Enum,
    /// `struct S {`, and a variant with named fields, whose members are its fields.
    Fields,
}

/// One open block, and the brace depth it sits at.
#[derive(Debug, Clone)]
struct Frame {
    name: String,
    kind: Kind,
    depth: usize,
}

/// Reads one source's declarations into the index.
///
/// The scanner tracks brace depth rather than parsing Rust: what it needs is the *name* of the
/// block a declaration sits in, and a name is on the same line as the brace that opens it.
fn declare(text: &str, module: &str, home: &str, index: &mut Index) {
    let mut stack: Vec<Frame> = Vec::new();
    let mut depth: usize = 0;
    let mut pending_derives: Vec<&'static str> = Vec::new();
    index.contexts.insert((module.to_owned(), home.to_owned()));
    index.contexts.insert((home.to_owned(), home.to_owned()));
    for name in reexports(text) {
        index.contexts.insert((name.clone(), home.to_owned()));
        index
            .declared
            .entry((module.to_owned(), home.to_owned()))
            .or_default()
            .insert(name.clone());
        index
            .declared
            .entry((home.to_owned(), home.to_owned()))
            .or_default()
            .insert(name);
    }
    for line in text.lines() {
        let trimmed = line.trim_start();
        let code = without_comment(line);
        let mut opening: Option<(Kind, String)> = None;
        if !trimmed.starts_with("//") {
            if let Some(derived) = derives(trimmed) {
                pending_derives = derived;
            } else if let Some((kind, name)) = declaration(trimmed) {
                for method in &pending_derives {
                    index
                        .declared
                        .entry((name.to_owned(), home.to_owned()))
                        .or_default()
                        .insert((*method).to_owned());
                }
                record(index, &stack, module, home, name);
                pending_derives.clear();
                if let Some(kind) = kind {
                    index.contexts.insert((name.to_owned(), home.to_owned()));
                    opening = Some((kind, name.to_owned()));
                }
            } else if let Some(name) = impl_head(trimmed) {
                index.contexts.insert((name.to_owned(), home.to_owned()));
                opening = Some((Kind::Block, name.to_owned()));
            } else if let Some(frame) = stack.last() {
                match frame.kind {
                    Kind::Enum => {
                        if let Some(name) = variant(trimmed) {
                            let holder = frame.name.clone();
                            index
                                .declared
                                .entry((holder, home.to_owned()))
                                .or_default()
                                .insert(name.to_owned());
                            index.contexts.insert((name.to_owned(), home.to_owned()));
                            if code.contains('{') {
                                opening = Some((Kind::Fields, name.to_owned()));
                            }
                        }
                    }
                    Kind::Fields => {
                        if let Some(name) = field(trimmed) {
                            let holder = frame.name.clone();
                            index
                                .declared
                                .entry((holder, home.to_owned()))
                                .or_default()
                                .insert(name.to_owned());
                        }
                    }
                    Kind::Block => {}
                }
            }
        }
        let opens = code.matches('{').count();
        let closes = code.matches('}').count();
        if let Some((kind, name)) = opening {
            if opens > closes {
                stack.push(Frame { name, kind, depth });
            } else if kind == Kind::Fields && opens > 0 {
                // A variant or a struct written on one line — `Group { alpha_is_shape: bool }` —
                // opens and closes its block before the stack can hold it, so its fields are
                // read here or not at all.
                inline_fields(code, &name, home, index);
            }
        }
        depth = depth.saturating_add(opens).saturating_sub(closes);
        while stack.last().is_some_and(|frame| frame.depth >= depth) {
            stack.pop();
        }
    }
}

/// Records a declaration under every context a reader may name it by.
fn record(index: &mut Index, stack: &[Frame], module: &str, home: &str, name: &str) {
    let mut under = |context: String| {
        index
            .declared
            .entry((context, home.to_owned()))
            .or_default()
            .insert(name.to_owned());
    };
    under(module.to_owned());
    under(home.to_owned());
    if let Some(frame) = stack.last() {
        under(frame.name.clone());
    }
}

/// Every `use` statement's head and each leaf name it brings in, read whole so that a statement
/// running over several lines is read as one.
///
/// The head is the first segment — a crate's name, or `crate`, `self` or `super` — and the leaf
/// is what the name is called in this file. `a::B as C` gives `C`; `a::B` gives `B`.
fn use_leaves(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut statement: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        let body = if let Some(mut held) = statement.take() {
            held.push(' ');
            held.push_str(trimmed);
            held
        } else {
            let Some(rest) = strip_visibility(trimmed).strip_prefix("use ") else {
                continue;
            };
            rest.to_owned()
        };
        let Some((complete, _)) = body.split_once(';') else {
            statement = Some(body);
            continue;
        };
        let head = complete
            .trim_start()
            .split("::")
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned();
        for part in complete.split([',', '{', '}']) {
            let part = part.trim();
            if part.is_empty() || part == "*" {
                continue;
            }
            let named = part.rsplit(" as ").next().unwrap_or(part);
            let named = named.rsplit("::").next().unwrap_or(named).trim();
            if named == "self" {
                continue;
            }
            if let Some(name) = identifier(named)
                && name.len() == named.len()
            {
                found.push((head.clone(), name.to_owned()));
            }
        }
    }
    found
}

/// The names a file offers a reader under its own module path: the leaves of its `pub use`
/// statements.
///
/// A re-export is a declaration as far as a path in prose is concerned — `pdf_model::content`
/// re-exports `Placed`, and a comment naming `` `content::Placed` `` is right.
fn reexports(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut statement: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        let body = if let Some(mut held) = statement.take() {
            held.push(' ');
            held.push_str(trimmed);
            held
        } else {
            if !trimmed.starts_with("pub") {
                continue;
            }
            let Some(rest) = strip_visibility(trimmed).strip_prefix("use ") else {
                continue;
            };
            rest.to_owned()
        };
        let Some((complete, _)) = body.split_once(';') else {
            statement = Some(body);
            continue;
        };
        for part in complete.split([',', '{', '}']) {
            let part = part.trim();
            if part.is_empty() || part == "*" {
                continue;
            }
            let named = part.rsplit(" as ").next().unwrap_or(part);
            let named = named.rsplit("::").next().unwrap_or(named).trim();
            if named == "self" {
                continue;
            }
            if let Some(name) = identifier(named)
                && name.len() == named.len()
            {
                found.push(name.to_owned());
            }
        }
    }
    found
}

/// The methods a `#[derive(...)]` attribute gives the item below it.
///
/// A derived method is declared by the compiler and by nothing in this tree, so without this a
/// comment naming `` `Asked::default` `` reads as a finding. The table is the standard library's
/// derivable traits and the one method each contributes that a comment has reason to name;
/// `Copy` and `Eq` contribute none.
fn derives(trimmed: &str) -> Option<Vec<&'static str>> {
    let rest = trimmed.strip_prefix("#[derive(")?;
    let rest = rest.split(')').next().unwrap_or(rest);
    let mut found = Vec::new();
    for part in rest.split(',') {
        match part.trim() {
            "Default" => found.push("default"),
            "Clone" => found.push("clone"),
            "Debug" | "Display" => found.push("fmt"),
            "Hash" => found.push("hash"),
            "PartialEq" => {
                found.push("eq");
                found.push("ne");
            }
            "Ord" => found.push("cmp"),
            "PartialOrd" => found.push("partial_cmp"),
            _ => {}
        }
    }
    Some(found)
}

/// Records the named fields of a block written on one line, under the name that holds them.
fn inline_fields(code: &str, holder: &str, home: &str, index: &mut Index) {
    let Some(open) = code.find('{') else {
        return;
    };
    let body = &code[open.saturating_add(1)..];
    let body = body.rfind('}').map_or(body, |end| &body[..end]);
    for part in body.split(',') {
        if let Some(name) = field(part.trim_start()) {
            index
                .declared
                .entry((holder.to_owned(), home.to_owned()))
                .or_default()
                .insert(name.to_owned());
        }
    }
}

/// A line's code, with a trailing `//` comment removed.
///
/// Approximate on purpose: a `//` inside a string literal loses the rest of that line's braces.
/// What that costs is a context frame closing late, which widens what resolves — the direction
/// this sweep is loose in by design.
fn without_comment(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

/// The declaration a line opens, and whether it opens a block of that kind.
fn declaration(trimmed: &str) -> Option<(Option<Kind>, &str)> {
    let mut rest = trimmed;
    rest = strip_visibility(rest);
    loop {
        let (word, after) = first_word(rest);
        match word {
            "default" | "async" | "unsafe" => rest = after,
            "extern" => {
                let after = after.trim_start();
                rest = after.strip_prefix('"').map_or(after, |quoted| {
                    quoted
                        .find('"')
                        .map_or(after, |end| &quoted[end.saturating_add(1)..])
                });
            }
            "const" => {
                // `const fn` is a function; a bare `const` is an item of its own.
                let (next, _) = first_word(after.trim_start());
                if next == "fn" || next == "unsafe" || next == "extern" {
                    rest = after;
                } else {
                    return identifier(after.trim_start()).map(|name| (None, name));
                }
            }
            "fn" | "static" | "type" => {
                return identifier(after.trim_start()).map(|name| (None, name));
            }
            "mod" | "trait" => {
                return identifier(after.trim_start()).map(|name| (Some(Kind::Block), name));
            }
            "enum" => {
                return identifier(after.trim_start()).map(|name| (Some(Kind::Enum), name));
            }
            "struct" | "union" => {
                return identifier(after.trim_start()).map(|name| (Some(Kind::Fields), name));
            }
            _ => return None,
        }
        rest = rest.trim_start();
    }
}

/// The type an `impl` block is for: the name after `for` where there is one, else the name after
/// `impl` and its generics.
fn impl_head(trimmed: &str) -> Option<&str> {
    let (word, after) = first_word(trimmed);
    if word != "impl" {
        return None;
    }
    let after = skip_generics(after.trim_start());
    let body = after.split('{').next().unwrap_or(after);
    let subject = body.rsplit(" for ").next().unwrap_or(body).trim();
    let subject = subject.trim_start_matches(['&', '*']).trim();
    let subject = subject.split('<').next().unwrap_or(subject);
    let subject = subject.rsplit("::").next().unwrap_or(subject).trim();
    identifier(subject)
}

/// An enum variant: an identifier at the head of a line inside an `enum` block.
fn variant(trimmed: &str) -> Option<&str> {
    if trimmed.starts_with('#') {
        return None;
    }
    let name = identifier(trimmed)?;
    if !name.starts_with(|first: char| first.is_ascii_uppercase()) {
        return None;
    }
    let after = trimmed[name.len()..].trim_start();
    if after.is_empty() || after.starts_with(['{', '(', ',', '=']) {
        Some(name)
    } else {
        None
    }
}

/// A named field: an identifier followed by a colon that is not a path separator.
fn field(trimmed: &str) -> Option<&str> {
    let trimmed = strip_visibility(trimmed);
    let name = identifier(trimmed)?;
    let after = trimmed[name.len()..].trim_start();
    if after.starts_with(':') && !after.starts_with("::") {
        Some(name)
    } else {
        None
    }
}

/// A leading `pub`, with its `(…)` restriction where it has one.
fn strip_visibility(trimmed: &str) -> &str {
    let Some(after) = trimmed.strip_prefix("pub") else {
        return trimmed;
    };
    if let Some(open) = after.strip_prefix('(') {
        return open
            .find(')')
            .map_or(after, |end| &open[end.saturating_add(1)..])
            .trim_start();
    }
    if after.starts_with(char::is_whitespace) {
        after.trim_start()
    } else {
        trimmed
    }
}

/// A leading `<…>`, matched by depth so that a nested generic does not end it early.
fn skip_generics(text: &str) -> &str {
    if !text.starts_with('<') {
        return text;
    }
    let mut depth = 0usize;
    for (at, character) in text.char_indices() {
        match character {
            '<' => depth = depth.saturating_add(1),
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return text[at.saturating_add(1)..].trim_start();
                }
            }
            _ => {}
        }
    }
    text
}

/// The first word of a line, and what follows it.
fn first_word(text: &str) -> (&str, &str) {
    let text = text.trim_start();
    let end = text
        .find(|character: char| !(character.is_alphanumeric() || character == '_'))
        .unwrap_or(text.len());
    (&text[..end], &text[end..])
}

/// A leading Rust identifier.
fn identifier(text: &str) -> Option<&str> {
    let mut end = 0;
    for (at, character) in text.char_indices() {
        if character.is_alphanumeric() || character == '_' {
            end = at.saturating_add(character.len_utf8());
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    let name = &text[..end];
    if name.starts_with(|first: char| first.is_ascii_digit()) {
        return None;
    }
    Some(name)
}

/// Every backticked Rust path written in a `///` or `//!` comment, with the doc-comment line
/// count beside it.
///
/// A fenced block inside a doc comment is skipped: its contents are an *example*, and an example
/// names types the tree has no reason to declare.
fn mentions_in(text: &str) -> (Vec<(usize, String, String)>, usize) {
    let mut found = Vec::new();
    let mut lines = 0usize;
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let Some(body) = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
        else {
            continue;
        };
        lines = lines.saturating_add(1);
        if body.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        for named in paths_in(body) {
            found.push((index.saturating_add(1), named, trimmed.to_owned()));
        }
    }
    (found, lines)
}

/// Every backticked span of one doc-comment line that reads as a multi-segment Rust path.
fn paths_in(body: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find('`') {
        let after = &rest[open.saturating_add(1)..];
        let Some(close) = after.find('`') else {
            break;
        };
        let span = &after[..close];
        rest = &after[close.saturating_add(1)..];
        if let Some(path) = as_path(span) {
            found.push(path);
        }
    }
    found
}

/// A span read as a Rust path, or `None`.
///
/// It has to be identifiers all the way down, separated by `::`, with at least two of them. A
/// span carrying a call, a generic argument or a space is prose about a path rather than a path.
fn as_path(span: &str) -> Option<String> {
    let span = span.trim();
    if !span.contains("::") {
        return None;
    }
    let mut segments = 0usize;
    for segment in span.split("::") {
        if identifier(segment).is_none_or(|name| name.len() != segment.len()) {
            return None;
        }
        segments = segments.saturating_add(1);
    }
    if segments < 2 {
        return None;
    }
    Some(span.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(text: &str) -> Index {
        let mut index = Index::default();
        index.crates.insert("some_crate".to_owned());
        index.reachable.insert(
            "some_crate".to_owned(),
            std::iter::once("some_crate".to_owned()).collect(),
        );
        declare(text, "thing", "some_crate", &mut index);
        index
    }

    fn reach(index: &Index, path: &str) -> Reach {
        index.reach_of(path, "some_crate")
    }

    #[test]
    fn a_function_resolves_under_its_module_and_its_crate() {
        let index = index_of("pub fn apply() {}\n");
        assert_eq!(reach(&index, "thing::apply"), Reach::Resolved);
        assert_eq!(reach(&index, "some_crate::apply"), Reach::Resolved);
        assert_eq!(reach(&index, "thing::absent"), Reach::Absent);
    }

    #[test]
    fn a_method_resolves_under_the_type_its_impl_is_for() {
        let index = index_of("impl<'a> Display for Report<'a> {\n    fn shortfall(&self) {}\n}\n");
        assert_eq!(reach(&index, "Report::shortfall"), Reach::Resolved);
    }

    #[test]
    fn a_variant_and_a_variants_field_resolve_under_their_holders() {
        let index = index_of("pub enum Command {\n    Group { alpha_is_shape: bool },\n}\n");
        assert_eq!(reach(&index, "Command::Group"), Reach::Resolved);
        assert_eq!(reach(&index, "Group::alpha_is_shape"), Reach::Resolved);
    }

    #[test]
    fn a_struct_field_resolves_under_its_struct() {
        let index = index_of("pub struct Limits {\n    pub max_stream_len: usize,\n}\n");
        assert_eq!(reach(&index, "Limits::max_stream_len"), Reach::Resolved);
    }

    #[test]
    fn a_nested_module_closes_with_its_brace() {
        let index = index_of("mod inner {\n    fn held() {}\n}\nfn outside() {}\n");
        assert_eq!(reach(&index, "inner::held"), Reach::Resolved);
        assert_eq!(reach(&index, "inner::outside"), Reach::Absent);
    }

    #[test]
    fn the_three_shapes_that_are_not_findings_are_told_apart() {
        let mut index = index_of("pub fn apply() {}\n");
        index.external.insert("tiny_skia".to_owned());
        assert_eq!(reach(&index, "Self::apply"), Reach::Generic);
        assert_eq!(reach(&index, "tiny_skia::Pixmap::new"), Reach::Foreign);
        assert_eq!(reach(&index, "ControlFlow::Wait"), Reach::Undeclared);
    }

    #[test]
    fn a_single_segment_path_is_not_read_at_all() {
        assert!(as_path("Document").is_none());
        assert!(as_path("a::b").is_some());
        assert!(as_path("a::b(c)").is_none());
        assert!(as_path("Vec<T>::len").is_none());
    }

    #[test]
    fn a_fenced_example_inside_a_doc_comment_is_not_read() {
        let (found, lines) = mentions_in(
            "/// A sentence naming `thing::apply`.\n/// ```\n/// `made_up::name`\n/// ```\n",
        );
        assert_eq!(lines, 4);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].1, "thing::apply");
    }
}
