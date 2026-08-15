//! The eighth sweep: a pointer this project's prose makes into its own tree, checked.
//!
//! # The shape it exists for
//!
//! Every other sweep in `doc/todo/01` reads what a row or a comment *claims*. This one reads what
//! it **points at**, and a pointer decays faster than a claim — deleting a file is a thing rounds
//! do on purpose. §8.9.6.1's note cited `doc/todo/20` for a refusal ADR 0169 had implemented, and
//! the session that made the sentence false deleted the file it named in the same commit;
//! `doc/todo/12` stood in six places under `crates/` after the item was done. The sweep has run as
//! a grep since the three-hundred-and-seventy-fifth session. This is the same instrument as a
//! program, per `doc/todo/01`'s binding rule that a sweep round commits one before running any.
//!
//! # Why it is not [`crate::citation`]
//!
//! That module checks a citation of *the standard* — a `§` against ISO 32000-2's clause index.
//! This one checks a citation of *this tree*: a path, and the symbol half of a path. Two
//! populations, two questions, and the ledger gate already covers a third — the `code` and `test`
//! arrays, whose sites it resolves including the `::function` half ([`crate::ledger`]). What is
//! unchecked, and what this reads, is every pointer written in **prose**: a note's sentence, a
//! doc comment, an ADR's paragraph.
//!
//! # The two populations
//!
//! - **A path pointer** — `doc/todo/47`, `crates/pdf-model/src/view.rs`, `tests/import_policy.rs`
//!   — resolved as [`Reach`] describes.
//! - **A symbol pointer** — `content.rs::alternate_image` — where the file exists and the item it
//!   names has to be in it. `doc/todo/01` names this second half as a sweep somebody could
//!   build; it is here because it is the same sentence's other end, and because a test renamed
//!   away is exactly how the `test` array's own check has paid.
//!
//! # A pointer is resolved from where it is written
//!
//! `tests/import_policy.rs` in a doc comment under `crates/viewer-host/src/` means that crate's
//! `tests/` directory, and the same words in `doc/QUORRA_FEEDBACK.md` mean a directory in the
//! render library's tree. So a pointer whose head is [`RELATIVE_HEADS`] is joined to the crate
//! the mentioning file belongs to, and is [`Reach::Unrooted`] where the mentioning file is not
//! in a crate — which is the honest answer for a document addressed to another project, and it
//! keeps that whole population out of the reading list without a list of documents to skip.
//!
//! # The noise, classified rather than filtered
//!
//! Four shapes are known and none of them is a defect, so each gets a rung of its own rather
//! than a place in the reading list: a fragment that resolves in **another crate**
//! ([`Reach::AnotherCrate`] — `pdf-syntax` naming `examples/callgrind_interpret`, which is
//! `pdf-model`'s), a **form** rather than a citation ([`Reach::Placeholder`] —
//! `doc/todo/NN`, `crates/foo.rs`, a glob), a path the tree deliberately does not **carry**
//! ([`Reach::NotCarried`] — a submodule nobody checked out, a fuzz corpus a run builds, the
//! specifications unpacked from `doc/specifications.zip`), and the unrooted fragment above.
//! What is left is [`Reach::Absent`], and it is classified once more by
//! [`crate::retired::kind_of`]: a **correction quoting the pointer it retired** is this sweep's
//! oldest false positive — §8.9.6.1 has produced it on every run since the three-hundred-and-
//! seventy-fifth — and a **standing** dead pointer is the finding. Read the sentence before
//! believing a hit either way; one line of context tells a citation from a quotation.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, and one of its own: a dead pointer is sometimes the right thing to
//! write. A round may cite the file it is about to add, and a correction *has* to quote the
//! pointer it retired. A build that failed on either would be teaching rounds to write around
//! the checker. It runs in a fraction of a second over the ledger, the tree's comments and this
//! project's prose.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::ledger::Ledger;
use crate::retired::{self, Kind};

/// The path heads that are relative to the workspace root.
///
/// The workspace's own top-level directories, and nothing else: a token whose head is not one of
/// these or of [`RELATIVE_HEADS`] is not a pointer into this tree at all, which is how
/// `quorra-gpu/tests/two_rasters.rs` and `https://github.com/…` are never collected rather than
/// collected and excused.
pub const ROOTED_HEADS: [&str; 5] = ["doc", "crates", "tools", "fuzz", "data"];

/// The path heads that are relative to the crate the mentioning file is in.
///
/// Cargo's own directory names. A doc comment saying `tests/x.rs` means its own crate's tests,
/// which is what a reader does with it, and the same words in a document under `doc/` mean
/// nothing this program can resolve — see [`Reach::Unrooted`].
pub const RELATIVE_HEADS: [&str; 4] = ["src", "tests", "examples", "benches"];

/// The names this project writes when it is showing the *form* of a pointer.
///
/// Stated rather than quietly skipped, because an instrument that ignores something should say
/// what. `doc/todo/NN-slug.md` is how `doc/adr/0265` explains what a todo file is called,
/// `crates/foo.rs::some_test` is how `doc/todo/01` describes this very sweep, and `doc/x.pdf` is
/// the usage line two viewer binaries print. A segment of nothing but `N`s is the same device.
pub const PLACEHOLDERS: [&str; 6] = ["foo", "foo.rs", "bar.rs", "file.rs", "x.pdf", "NN-slug.md"];

/// The paths a checkout of this repository does not carry, with what puts them there.
///
/// - `doc/corpora`, `doc/pdf.js` and `doc/arlington-pdf-model` are submodules, and three of the
///   four corpora are optional in the strong sense (`doc/environment.md`): a pointer into one is
///   live for a developer who checked it out and absent for everybody else, so it is neither.
/// - `fuzz/corpus` is what a fuzzing run builds, and `.gitignore` covers it.
/// - `doc/md` and the specifications beside it are unpacked from `doc/specifications.zip`, which
///   ADR 0187 decided and `NOTICE` section 3 explains.
/// - `target` is the build directory.
pub const NOT_CARRIED: [&str; 6] = [
    "doc/corpora",
    "doc/pdf.js",
    "doc/arlington-pdf-model",
    "fuzz/corpus",
    "doc/md",
    "target",
];

/// What a pointer resolved to, in the order a person reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reach {
    /// Nothing at that path. The reading list.
    Absent,
    /// A fragment that names no file of the crate it is written in, and one file of another
    /// crate — `examples/callgrind_interpret` in `pdf-syntax`, which is `pdf-model`'s. The
    /// pointer resolves for a reader who searches; it does not say where.
    AnotherCrate,
    /// A fragment whose head is [`RELATIVE_HEADS`], written where no crate says what it is
    /// relative to — a document addressed to another project, most often.
    Unrooted,
    /// A form rather than a citation: [`PLACEHOLDERS`], or a glob.
    Placeholder,
    /// Under one of [`NOT_CARRIED`]: the tree deliberately does not have it here.
    NotCarried,
    /// Resolved. A file or a directory exists at it, or the number it names does.
    Live,
}

impl fmt::Display for Reach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Absent => "absent",
            Self::AnotherCrate => "in another crate",
            Self::Unrooted => "unrooted",
            Self::Placeholder => "a form",
            Self::NotCarried => "not carried",
            Self::Live => "live",
        })
    }
}

/// One pointer, where it is written, and what it reached.
#[derive(Debug, Clone)]
pub struct Pointer {
    /// The path as written.
    pub text: String,
    /// Where it is: `doc/conformance/ledger.toml:123 (§8.9.6.1, partial)`,
    /// `crates/viewer-host/src/policy.rs:60` or `doc/adr/0169-….md:41`.
    pub location: String,
    /// The sentence carrying it, whole.
    pub sentence: String,
    /// What it resolved to.
    pub reach: Reach,
    /// Whether the sentence narrates a retirement — the known false positive — or makes a claim.
    pub kind: Kind,
}

/// One `file.rs::item` pointer, and whether the file holds the item.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// The pointer as written.
    pub text: String,
    /// Where it is written.
    pub location: String,
    /// The sentence carrying it, whole.
    pub sentence: String,
    /// The source files whose path ends with the named one. Empty means the file itself is gone.
    pub files: Vec<String>,
    /// Whether one of them defines the item.
    pub defined: bool,
    /// Whether the sentence narrates a retirement.
    pub kind: Kind,
}

/// What the sweep found, in one value so that a report can count before it prints.
#[derive(Debug, Clone, Default)]
pub struct Sweep {
    /// Every path pointer collected, in the order the populations were read.
    pub pointers: Vec<Pointer>,
    /// Every symbol pointer collected.
    pub symbols: Vec<Symbol>,
}

impl Sweep {
    /// The path pointers on one rung.
    #[must_use]
    pub fn reaching(&self, reach: Reach) -> Vec<&Pointer> {
        self.pointers
            .iter()
            .filter(|pointer| pointer.reach == reach)
            .collect()
    }

    /// The symbol pointers whose item no candidate file defines.
    #[must_use]
    pub fn undefined(&self) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|symbol| !symbol.defined)
            .collect()
    }
}

/// What exists, as the sweep asks about it: relative paths with forward slashes.
///
/// Built once from a walk of the workspace, so that resolving a pointer is a set lookup rather
/// than a filesystem call per mention — and so that the unit tests can state a tree in four
/// lines instead of creating one.
#[derive(Debug, Clone, Default)]
pub struct Tree {
    present: BTreeSet<String>,
    crates: Vec<String>,
}

impl Tree {
    /// A tree from the paths it holds. Every path is relative to the workspace root.
    ///
    /// A crate is a directory holding a `Cargo.toml`, which is what decides where
    /// [`RELATIVE_HEADS`] point from.
    #[must_use]
    pub fn of<I, S>(paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let present: BTreeSet<String> = paths
            .into_iter()
            .map(|path| path.as_ref().replace('\\', "/"))
            .collect();
        let mut crates: Vec<String> = present
            .iter()
            .filter_map(|path| path.strip_suffix("/Cargo.toml"))
            .map(str::to_owned)
            .collect();
        // Longest first, so that a file in a workspace member is attributed to the member
        // rather than to the workspace root above it.
        crates.sort_by_key(|directory| std::cmp::Reverse(directory.len()));
        Self { present, crates }
    }

    /// Walks the workspace, skipping what [`NOT_CARRIED`] names and every hidden directory.
    ///
    /// # Errors
    ///
    /// If a directory cannot be read: a sweep that skipped what it could not open would report a
    /// clean tree for a tree it had not looked at.
    pub fn walk(root: &Path) -> std::io::Result<Self> {
        let mut paths = Vec::new();
        collect(root, root, &mut paths)?;
        Ok(Self::of(paths))
    }

    /// The crate directory a file belongs to, or `None` where it is in no crate.
    #[must_use]
    pub fn crate_of(&self, path: &str) -> Option<&str> {
        self.crates
            .iter()
            .find(|directory| path.starts_with(&format!("{directory}/")))
            .map(String::as_str)
    }

    /// Whether a path exists, by itself or as the number a file's name begins with.
    ///
    /// `doc/todo/47` is how this project cites `doc/todo/47-what-a-trace-cannot-see.md`, and
    /// `doc/adr/0265` its ADR — so a pointer resolves when a name begins with it and then breaks
    /// at a separator. The separator is what keeps `doc/todo/1` from finding `doc/todo/13-…`.
    ///
    /// A space is one of the separators, and it is not a nicety: `doc/hayro vs this project.md`
    /// is a file in this tree, and a token stops at the space it holds.
    #[must_use]
    pub fn holds(&self, path: &str) -> bool {
        if self.present.contains(path) {
            return true;
        }
        self.present
            .range(path.to_owned()..)
            .take_while(|held| held.starts_with(path))
            .any(|held| {
                held.get(path.len()..)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(|character| matches!(character, '-' | '.' | ' '))
            })
    }

    /// Every source file whose path ends with `named`, at a path separator.
    #[must_use]
    pub fn files_ending_with(&self, named: &str) -> Vec<String> {
        let suffix = format!("/{named}");
        self.present
            .iter()
            .filter(|path| *path == named || path.ends_with(&suffix))
            .cloned()
            .collect()
    }

    /// Whether any crate holds this fragment, wherever it is.
    ///
    /// The same name rule as [`Self::holds`] one level down: a fragment resolves at a path
    /// separator, and by the number a file's name begins with.
    #[must_use]
    pub fn holds_anywhere(&self, fragment: &str) -> bool {
        let suffix = format!("/{fragment}");
        self.present.iter().any(|held| {
            held.ends_with(&suffix)
                || held.rfind(&suffix).is_some_and(|at| {
                    let rest = held
                        .get(at.saturating_add(suffix.len())..)
                        .unwrap_or_default();
                    rest.starts_with('.') || rest.starts_with('-')
                })
        })
    }
}

/// Every path in the tree, relative to `root`.
fn collect(root: &Path, directory: &Path, into: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || NOT_CARRIED.iter().any(|skipped| relative == *skipped) {
            continue;
        }
        into.push(relative);
        // A symbolic link to a directory is how a worktree reaches the checkouts beside it;
        // following one would walk a third-party tree, or the same tree twice.
        if path.is_dir() && !path.is_symlink() {
            collect(root, &path, into)?;
        }
    }
    Ok(())
}

/// Runs both halves of the sweep over the ledger's notes, the tree's comments and its prose.
///
/// `sources` are the Rust files under [`crate::SOURCE_ROOTS`] with their text — they are both a
/// population to read and the population a symbol pointer is resolved against — and `documents`
/// the Markdown under `doc/`. Two directories are read by nothing here, for the reasons
/// [`crate::retired::NOT_SWEPT`] and [`crate::NOT_SCANNED`] give: a round's own record is not
/// another round's to correct, and this checker's own prose quotes the findings as examples.
#[must_use]
pub fn sweep(
    tree: &Tree,
    ledger: &Ledger,
    sources: &[(PathBuf, String)],
    documents: &[(PathBuf, String)],
) -> Sweep {
    let mut places: Vec<(String, String, Option<String>)> = Vec::new();
    for row in &ledger.rows {
        if let Some(note) = row.note.as_deref() {
            places.push((
                format!(
                    "{}:{} (§{}, {})",
                    crate::LEDGER,
                    row.line,
                    row.clause,
                    row.status.as_str()
                ),
                note.to_owned(),
                None,
            ));
        }
    }
    for (path, text) in sources {
        let shown = path.to_string_lossy().replace('\\', "/");
        if shown.starts_with(crate::NOT_SCANNED) {
            continue;
        }
        let home = tree.crate_of(&shown).map(str::to_owned);
        for (line, block) in crate::blockers::comment_blocks(text) {
            places.push((format!("{shown}:{line}"), block, home.clone()));
        }
    }
    for (path, text) in documents {
        let shown = path.to_string_lossy().replace('\\', "/");
        if shown.starts_with(retired::NOT_SWEPT) {
            continue;
        }
        for (line, block) in retired::paragraphs(text) {
            places.push((format!("{shown}:{line}"), block, None));
        }
    }

    let index = source_index(sources);
    let mut found = Sweep::default();
    for (location, block, home) in &places {
        for sentence in crate::unread::sentences(block) {
            let kind = retired::kind_of(sentence);
            for text in paths_in(sentence) {
                found.pointers.push(Pointer {
                    reach: reach_of(&text, home.as_deref(), tree),
                    text,
                    location: location.clone(),
                    sentence: sentence.to_owned(),
                    kind,
                });
            }
            for (path, item) in symbols_in(sentence) {
                let files = tree.files_ending_with(&path);
                let defined = files
                    .iter()
                    .filter_map(|file| index.get(file.as_str()))
                    .any(|text| defines(text, &item));
                found.symbols.push(Symbol {
                    text: format!("{path}::{item}"),
                    location: location.clone(),
                    sentence: sentence.to_owned(),
                    files,
                    defined,
                    kind,
                });
            }
        }
    }
    found
}

/// The sources by relative path, for the symbol half.
fn source_index(sources: &[(PathBuf, String)]) -> BTreeMap<String, &str> {
    sources
        .iter()
        .map(|(path, text)| (path.to_string_lossy().replace('\\', "/"), text.as_str()))
        .collect()
}

/// What a pointer reaches, given the crate it was written in.
fn reach_of(text: &str, home: Option<&str>, tree: &Tree) -> Reach {
    if text.contains('*') || text.split('/').any(is_a_placeholder) {
        return Reach::Placeholder;
    }
    let head = text.split('/').next().unwrap_or_default();
    let path = if ROOTED_HEADS.contains(&head) {
        text.to_owned()
    } else if RELATIVE_HEADS.contains(&head) {
        match home {
            Some(directory) => format!("{directory}/{text}"),
            None => return Reach::Unrooted,
        }
    } else {
        return Reach::Unrooted;
    };
    if tree.holds(&path) {
        Reach::Live
    } else if RELATIVE_HEADS.contains(&head) && tree.holds_anywhere(text) {
        Reach::AnotherCrate
    } else if NOT_CARRIED
        .iter()
        .any(|carried| path == *carried || path.starts_with(&format!("{carried}/")))
        || is_a_specification(&path)
    {
        Reach::NotCarried
    } else {
        Reach::Absent
    }
}

/// Whether a segment is one of the names this project writes to show a *form*.
fn is_a_placeholder(segment: &str) -> bool {
    PLACEHOLDERS.contains(&segment)
        || (!segment.is_empty() && segment.chars().all(|character| character == 'N'))
}

/// Whether a path names one of the documents unpacked from `doc/specifications.zip`.
///
/// They are `doc/*.pdf` and everything under `doc/md/`; the second is [`NOT_CARRIED`]'s and this
/// is the first. ADR 0187: the standard is free to obtain and not free to redistribute, so the
/// repository carries it encrypted and a developer unpacks it.
fn is_a_specification(path: &str) -> bool {
    path.starts_with("doc/") && path.matches('/').count() == 1 && extension_is(path, "pdf")
}

/// Whether a path's extension is this one.
///
/// Through [`Path`] rather than by a suffix, so that a name ending in the letters without the
/// stop is not one — and case-insensitively, which is what the filesystem this may run on
/// decides rather than what this project's own names look like.
fn extension_is(path: &str, extension: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}

/// Every path-shaped token in one sentence.
///
/// A token is a run of path characters holding at least one separator. Backticks, quotation
/// marks and brackets are not path characters, so a citation written the way this project writes
/// one is bounded by its own punctuation; a trailing full stop is a sentence's rather than a
/// path's and comes off.
fn paths_in(sentence: &str) -> Vec<String> {
    let mut found = Vec::new();
    let bytes: Vec<char> = sentence.chars().collect();
    let mut at = 0usize;
    while at < bytes.len() {
        if !is_path_character(bytes[at]) {
            at = at.saturating_add(1);
            continue;
        }
        let start = at;
        while at < bytes.len() && is_path_character(bytes[at]) {
            at = at.saturating_add(1);
        }
        let token: String = bytes[start..at].iter().collect();
        let token = token.trim_end_matches(['.', '-', '/']).to_owned();
        if token.contains('/') && !token.starts_with('/') && !token.contains("//") {
            found.push(token);
        }
    }
    found
}

/// The characters a path is written with here.
fn is_path_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '/' | '.' | '_' | '-' | '*')
}

/// Every `file.rs::item` in one sentence, as the file and the item.
fn symbols_in(sentence: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (at, _) in sentence.match_indices("::") {
        let before = sentence.get(..at).unwrap_or_default();
        let path: String = before
            .chars()
            .rev()
            .take_while(|character| is_path_character(*character))
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect();
        if !extension_is(&path, "rs") {
            continue;
        }
        let after = sentence.get(at.saturating_add(2)..).unwrap_or_default();
        let item: String = after
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        if !item.is_empty() {
            found.push((path, item));
        }
    }
    found
}

/// The item keywords a definition is written with.
///
/// A pointer's other half is an item, and an item is declared. Matching a bare identifier
/// instead would count the file's own prose about a neighbour as a definition, which is the one
/// thing this half is for. The cost is that a citation of a field or of a macro reads as
/// undefined — a hit to read rather than a claim missed.
const DEFINITIONS: [&str; 8] = [
    "fn", "const", "static", "struct", "enum", "trait", "type", "mod",
];

/// Whether a source defines the item.
fn defines(text: &str, item: &str) -> bool {
    DEFINITIONS.iter().any(|keyword| {
        text.match_indices(&format!("{keyword} {item}"))
            .any(|(at, matched)| {
                let before = text.get(..at).and_then(|text| text.chars().next_back());
                let after = text
                    .get(at.saturating_add(matched.len())..)
                    .and_then(|text| text.chars().next());
                before.is_none_or(|character| !character.is_ascii_alphanumeric())
                    && after.is_none_or(|character| {
                        !character.is_ascii_alphanumeric() && character != '_'
                    })
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{Row, Status};

    fn tree() -> Tree {
        Tree::of([
            "crates",
            "crates/viewer-host",
            "crates/viewer-host/Cargo.toml",
            "crates/viewer-host/src",
            "crates/viewer-host/src/policy.rs",
            "crates/viewer-host/tests",
            "crates/viewer-host/tests/host_mappings.rs",
            "doc",
            "doc/todo",
            "doc/todo/01-ledger-partial-rows.md",
            "doc/corpora",
        ])
    }

    fn row(clause: &str, note: &str) -> Row {
        Row {
            clause: clause.parse().expect("a clause number"),
            title: String::new(),
            status: Status::Partial,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    fn ledger(rows: Vec<Row>) -> Ledger {
        Ledger { rows }
    }

    fn file(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_owned())
    }

    /// The sweep's own subject: a note pointing at a file no round left in the tree.
    #[test]
    fn a_note_naming_a_file_that_is_gone_is_the_finding() {
        let ledger = ledger(vec![row(
            "8.9.6.1",
            "The refusal is `doc/todo/20`'s to close.",
        )]);
        let found = sweep(&tree(), &ledger, &[], &[]);
        let absent = found.reaching(Reach::Absent);
        assert_eq!(absent.len(), 1);
        assert_eq!(absent.first().expect("one").text, "doc/todo/20");
    }

    /// A pointer to a todo or an ADR names the number, and the file's name begins with it.
    #[test]
    fn a_number_resolves_the_file_whose_name_begins_with_it() {
        assert!(tree().holds("doc/todo/01"));
        assert!(tree().holds("doc/todo/01-ledger-partial-rows.md"));
        assert!(!tree().holds("doc/todo/0"));
        // A token stops at a space, and a file in this tree has one in its name.
        assert!(Tree::of(["doc/hayro vs this project.md"]).holds("doc/hayro"));
    }

    /// A fragment is resolved from the crate it is written in — which is what a reader does
    /// with it, and where the five-hundred-and-thirty-seventh's first defect was.
    #[test]
    fn a_fragment_is_resolved_from_the_crate_it_is_written_in() {
        let sources = vec![
            file(
                "crates/viewer-host/src/policy.rs",
                "/// Testable without a filesystem, which is what `tests/import_policy.rs` does.\nfn resolve() {}\n",
            ),
            file(
                "crates/viewer-host/src/other.rs",
                "// The mappings are `tests/host_mappings.rs`'s.\nfn other() {}\n",
            ),
        ];
        let found = sweep(&tree(), &ledger(Vec::new()), &sources, &[]);
        assert_eq!(found.reaching(Reach::Absent).len(), 1);
        assert_eq!(
            found.reaching(Reach::Absent).first().expect("one").text,
            "tests/import_policy.rs"
        );
        assert_eq!(found.reaching(Reach::Live).len(), 1);
    }

    /// A fragment that names no file of its own crate and one of another's is neither dead nor
    /// exactly right: the pointer resolves for a reader who searches, and does not say where.
    #[test]
    fn a_fragment_that_resolves_in_another_crate_is_its_own_rung() {
        let tree = Tree::of([
            "crates/pdf-syntax/Cargo.toml",
            "crates/pdf-syntax/src/document.rs",
            "crates/pdf-model/Cargo.toml",
            "crates/pdf-model/examples/callgrind_interpret.rs",
        ]);
        let sources = vec![file(
            "crates/pdf-syntax/src/document.rs",
            "// The cost is `examples/callgrind_interpret`'s to print.\nfn open() {}\n",
        )];
        let found = sweep(&tree, &ledger(Vec::new()), &sources, &[]);
        assert_eq!(found.reaching(Reach::AnotherCrate).len(), 1);
        assert!(found.reaching(Reach::Absent).is_empty());
    }

    /// The same fragment in a document under `doc/` has no crate to be relative to, which is
    /// the honest answer for a document addressed to another project's tree.
    #[test]
    fn a_fragment_with_no_crate_is_unrooted_rather_than_dead() {
        let documents = vec![file(
            "doc/QUORRA_FEEDBACK.md",
            "The pair is `tests/two_rasters.rs`'s claim.\n",
        )];
        let found = sweep(&tree(), &ledger(Vec::new()), &[], &documents);
        assert_eq!(found.reaching(Reach::Unrooted).len(), 1);
        assert!(found.reaching(Reach::Absent).is_empty());
    }

    /// A token whose head is neither this workspace's nor a crate's is not a pointer into this
    /// tree at all, so it is never collected as one.
    #[test]
    fn another_trees_path_is_not_collected() {
        let documents = vec![file(
            "doc/QUORRA_UPGRADE.md",
            "See https://github.com/pdf-association/pdf20examples for the rest.\n",
        )];
        let found = sweep(&tree(), &ledger(Vec::new()), &[], &documents);
        assert!(found.reaching(Reach::Absent).is_empty());
    }

    /// A form is not a citation: the metavariable names and a glob are their own rung.
    #[test]
    fn a_form_is_not_a_citation() {
        let documents = vec![file(
            "doc/adr/0265-an-adr-is-a-dated-record.md",
            "A round adds `doc/todo/NN-slug.md` and cites `doc/corpora/*/**/*.pdf`.\n",
        )];
        let found = sweep(&tree(), &ledger(Vec::new()), &[], &documents);
        assert_eq!(found.reaching(Reach::Placeholder).len(), 2);
    }

    /// A submodule nobody checked out is not a dead pointer, and neither is a specification
    /// unpacked from the archive.
    #[test]
    fn a_path_the_tree_does_not_carry_is_its_own_rung() {
        let documents = vec![file(
            "doc/oracle-and-corpus.md",
            "The file is `doc/corpora/pdfbox/one.pdf`, quoted from `doc/ISO_32000-2.pdf`.\n",
        )];
        let found = sweep(&tree(), &ledger(Vec::new()), &[], &documents);
        assert_eq!(found.reaching(Reach::NotCarried).len(), 2);
        assert!(found.reaching(Reach::Absent).is_empty());
    }

    /// The oldest false positive: a correction quotes the pointer it retired, and the sweep
    /// marks it rather than hiding it.
    #[test]
    fn a_correction_quoting_its_own_pointer_is_marked() {
        let ledger = ledger(vec![row(
            "8.9.6.1",
            "This row used to send the reader to `doc/todo/20`.",
        )]);
        let found = sweep(&tree(), &ledger, &[], &[]);
        let absent = found.reaching(Reach::Absent);
        assert_eq!(absent.first().expect("one").kind, Kind::Correction);
    }

    /// The symbol half: the file is there and the item it names is not.
    #[test]
    fn a_symbol_the_file_does_not_define_is_a_finding() {
        let sources = vec![file(
            "crates/viewer-host/src/policy.rs",
            "//! The policy is `policy.rs::resolve_import`'s and `policy.rs::gone`'s.\npub fn resolve_import() {}\n",
        )];
        let tree = Tree::of([
            "crates",
            "crates/viewer-host",
            "crates/viewer-host/Cargo.toml",
            "crates/viewer-host/src/policy.rs",
        ]);
        let found = sweep(&tree, &ledger(Vec::new()), &sources, &[]);
        assert_eq!(found.symbols.len(), 2);
        let undefined = found.undefined();
        assert_eq!(undefined.len(), 1);
        assert_eq!(undefined.first().expect("one").text, "policy.rs::gone");
    }

    /// A definition is an item declaration, not a mention: a comment naming a neighbour is how
    /// this tree explains itself, and it is not evidence the item is here.
    #[test]
    fn a_mention_is_not_a_definition() {
        assert!(defines("pub fn hash_2b(x: u8) {}", "hash_2b"));
        assert!(defines(
            "const REFUSED_AT_FOUR: usize = 4;",
            "REFUSED_AT_FOUR"
        ));
        assert!(!defines("// hash_2b builds exactly that.", "hash_2b"));
        assert!(!defines("fn hash_2b_extended() {}", "hash_2b"));
    }

    /// A path is bounded by the punctuation this project writes round one, and a sentence's
    /// full stop is not part of it.
    #[test]
    fn a_path_is_bounded_by_its_punctuation() {
        assert_eq!(
            paths_in("It is `crates/pdf-model/src/view.rs`, and doc/todo/47."),
            vec![
                "crates/pdf-model/src/view.rs".to_owned(),
                "doc/todo/47".to_owned()
            ]
        );
        assert_eq!(paths_in("A ratio of 1/2 and a word.").len(), 1);
    }

    /// The checker's own prose quotes this sweep's findings as examples, so it is read by
    /// nothing here — the rule every module in this crate keeps.
    #[test]
    fn the_checkers_own_documentation_is_not_swept() {
        let sources = vec![file(
            "tools/conformance/src/pointers.rs",
            "// The finding was `doc/todo/20`.\n",
        )];
        let found = sweep(&tree(), &ledger(Vec::new()), &sources, &[]);
        assert!(found.pointers.is_empty());
    }

    /// A round's own record is not another round's to correct, so a history file is read by
    /// nothing here however many dead pointers it carries.
    #[test]
    fn a_rounds_own_record_is_not_swept() {
        let documents = vec![file(
            "doc/history/375-the-eighth-sweep.md",
            "The finding was `doc/todo/20`.\n",
        )];
        let found = sweep(&tree(), &ledger(Vec::new()), &[], &documents);
        assert!(found.pointers.is_empty());
    }
}
