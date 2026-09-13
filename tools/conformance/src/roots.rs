//! Where this tree's code is, derived from the workspace manifest rather than restated beside it.
//!
//! # The defect this closes
//!
//! Until the one-thousand-and-tenth session the scan read a hand-written list:
//!
//! ```text
//! pub const SOURCE_ROOTS: [&str; 3] = ["crates", "tools", "fuzz"];
//! ```
//!
//! beside a workspace manifest whose members are a *glob* — `["crates/*", "tools/*",
//! "raster/crates/*"]`. A list and a glob are two populations, and only one of them grew when
//! `raster/` was folded into this workspace on 2026-09-06: **1,884 clause citations in 243 of
//! that sub-project's 283 Rust files sat outside the citation and quotation gate for the
//! following four months, producing no findings because nothing read them.** Session 1004 found
//! it (ADR 1024 §4) and could only report it from `tools/round.sh`, because this crate was
//! another round's; the derivation here is the fix, and
//! `tests/conformance.rs::every_workspace_member_is_scanned` is what keeps it one.
//!
//! It is trap 25 with the population on the *instrument's* side rather than the tree's: an
//! instrument whose denominator is written by hand reports cleanly over the part of the world it
//! was told about.
//!
//! # The two rules, and why there are two
//!
//! A crate of this tree is either
//!
//! - **a member of the workspace**, which the manifest's `members` globs state exactly; or
//! - **a directory at the top of the tree holding a `Cargo.toml` of its own**, which is what
//!   `fuzz/` is. It declares its own `[workspace]` on purpose — `cargo-fuzz` builds it with its
//!   own sanitiser and profile settings, and membership would apply those to everything — so the
//!   root manifest does not mention it anywhere, and no derivation from that manifest alone can
//!   find it. Dropping it would have traded one silently unread directory for another.
//!
//! The second rule is bounded to depth one deliberately: a walk deep enough to find a second
//! excluded workspace is also deep enough to walk into `doc/corpora`, which is a submodule in a
//! primary checkout and a symlink into one in a worktree round. What catches a crate this rule
//! cannot see is the test named above, which asks `git` for every tracked manifest instead —
//! test code, where running a command is already this crate's habit (`tests/workspaces.rs`), and
//! a population derived a second, independent way is what makes the comparison worth making.
//!
//! # The head list one file along
//!
//! `pointers.rs` held the same defect in the same shape: `ROOTED_HEADS` was five names — `doc,
//! crates, tools, fuzz, data` — so no pointer written under `raster/` or `kio/` resolved at all.
//! That one is not derived here but from the walk `pointers::Tree` already does, which knows the
//! tree's top level directly and answers for `doc/` and `data/` as well as for the crates; see
//! [`crate::pointers::Tree::is_a_head`].

use std::collections::BTreeSet;
use std::path::Path;

/// The workspace manifest, relative to the tree's root.
pub const MANIFEST: &str = "Cargo.toml";

/// The build directory, which holds no source of this project's.
const BUILD: &str = "target";

/// Why the tree's shape could not be read.
///
/// Every variant is a refusal rather than a fallback. A derivation that answered "no roots" when
/// it could not read the manifest would hand every sweep in this crate an empty population and a
/// clean report, which is the exact failure this module exists to close.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file or directory could not be read.
    #[error("{path} could not be read: {source}")]
    Unreadable {
        /// What was being read.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
    /// The manifest states no `[workspace] members`.
    #[error(
        "{MANIFEST} states no `[workspace] members` list, so this tree's members cannot be \
         derived from it"
    )]
    NoMembers,
    /// A member pattern outside the shape this reader accepts.
    #[error(
        "{MANIFEST} states the member pattern `{0}`, which this reader does not accept: a \
         member is a path, optionally ending in a single `*` segment"
    )]
    Pattern(String),
}

impl From<Error> for std::io::Error {
    /// Carries the typed error as the `io::Error`'s source rather than flattening it to a string,
    /// so that [`crate::scan_tree`]'s existing signature stays while the reason survives.
    fn from(error: Error) -> Self {
        Self::other(error)
    }
}

/// Reads a path, naming it if it cannot be read.
fn read(path: &Path) -> Result<String, Error> {
    std::fs::read_to_string(path).map_err(|source| Error::Unreadable {
        path: path.display().to_string(),
        source,
    })
}

/// The member patterns the workspace manifest states, in the order it writes them.
///
/// A line-wise reader rather than a TOML one, for [`crate::toml_subset`]'s reason and with its
/// property: the shape it does not accept is *rejected* by [`Error::Pattern`] rather than
/// misread. `members` is looked for only inside the `[workspace]` table, because
/// `[workspace.metadata]` or a future table could hold a key of the same name.
fn member_patterns(root: &Path) -> Result<Vec<String>, Error> {
    let text = read(&root.join(MANIFEST))?;
    let mut in_workspace = false;
    let mut collecting = false;
    let mut patterns = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !collecting && trimmed.starts_with('[') {
            in_workspace = trimmed == "[workspace]";
            continue;
        }
        if !in_workspace {
            continue;
        }
        if !collecting {
            let Some(rest) = trimmed
                .strip_prefix("members")
                .map(str::trim_start)
                .and_then(|rest| rest.strip_prefix('='))
            else {
                continue;
            };
            collecting = true;
            patterns.extend(quoted(rest));
            if rest.contains(']') {
                return Ok(patterns);
            }
            continue;
        }
        patterns.extend(quoted(trimmed));
        if trimmed.contains(']') {
            return Ok(patterns);
        }
    }
    Err(Error::NoMembers)
}

/// Every `"…"` on one line, in order.
fn quoted(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('"') {
        let after = &rest[open.saturating_add(1)..];
        let Some(close) = after.find('"') else {
            break;
        };
        found.push(after[..close].to_owned());
        rest = &after[close.saturating_add(1)..];
    }
    found
}

/// Every workspace member's directory, relative to `root`, sorted and without repeats.
///
/// A pattern is either a literal path or a path ending in one `*` segment; the `*` is expanded
/// against the filesystem, keeping the directories that hold a `Cargo.toml`. Anything else is
/// [`Error::Pattern`] — a member shape this reader guessed at would be a second hand-written
/// rule wearing a derivation's clothes.
///
/// # Errors
///
/// If the manifest cannot be read, states no members, or states a pattern outside that shape.
pub fn members(root: &Path) -> Result<Vec<String>, Error> {
    let mut found = BTreeSet::new();
    for pattern in member_patterns(root)? {
        let Some((base, last)) = pattern.rsplit_once('/') else {
            if pattern.contains('*') {
                return Err(Error::Pattern(pattern));
            }
            found.insert(pattern);
            continue;
        };
        if base.contains('*') {
            return Err(Error::Pattern(pattern));
        }
        if last != "*" {
            if last.contains('*') {
                return Err(Error::Pattern(pattern));
            }
            found.insert(pattern);
            continue;
        }
        for name in directories(&root.join(base))? {
            let member = format!("{base}/{name}");
            if root.join(&member).join(MANIFEST).is_file() {
                found.insert(member);
            }
        }
    }
    Ok(found.into_iter().collect())
}

/// The directories whose Rust sources are scanned for citations, relative to `root`.
///
/// Every workspace member, and every crate sitting at the top of the tree outside the workspace —
/// the module comment says why those are two rules and not one. Sorted and without repeats, so
/// that a report's order is the tree's rather than the manifest's.
///
/// # Errors
///
/// If the manifest or a directory this derivation reads cannot be read. A derivation that skipped
/// what it could not open would hand every sweep a population it had not looked at, which is the
/// defect this module was written to close.
pub fn source_roots(root: &Path) -> Result<Vec<String>, Error> {
    let mut found: BTreeSet<String> = members(root)?.into_iter().collect();
    for name in directories(root)? {
        if root.join(&name).join(MANIFEST).is_file() {
            found.insert(name);
        }
    }
    Ok(found.into_iter().collect())
}

/// The names of `directory`'s own subdirectories, sorted, without the hidden ones or the build
/// directory.
///
/// A hidden directory is tooling's rather than this project's (`.git`, `.github`), and `target`
/// holds build output that is large enough for reading it to be felt.
fn directories(directory: &Path) -> Result<Vec<String>, Error> {
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
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == BUILD {
            continue;
        }
        found.push(name);
    }
    found.sort();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifests_members_are_read_as_written() {
        let root = crate::workspace_root();
        let patterns = member_patterns(&root).expect("the workspace manifest states its members");
        assert!(
            patterns.iter().any(|pattern| pattern.contains('*')),
            "the manifest's members are a glob, which is the whole reason this reader exists: {patterns:?}"
        );
    }

    #[test]
    fn a_member_glob_expands_to_directories_that_hold_a_manifest() {
        let root = crate::workspace_root();
        let members = members(&root).expect("the workspace's members");
        assert!(
            members
                .iter()
                .all(|member| root.join(member).join(MANIFEST).is_file()),
            "a member without a manifest is a pattern misread, not a crate"
        );
    }

    #[test]
    fn the_roots_hold_the_workspaces_outside_this_one() {
        let root = crate::workspace_root();
        let roots = source_roots(&root).expect("the tree's crates");
        let members = members(&root).expect("the workspace's members");
        // `fuzz/` is the standing case, and the second rule exists for it alone today.
        for outside in roots.iter().filter(|root| !members.contains(root)) {
            assert!(
                !outside.contains('/'),
                "only a top-level directory joins the roots outside the workspace: {outside}"
            );
        }
    }

    #[test]
    fn quoted_reads_a_line_of_strings() {
        assert_eq!(
            quoted(r#" ["crates/*", "tools/*"] "#),
            vec!["crates/*".to_owned(), "tools/*".to_owned()]
        );
        assert_eq!(quoted("members = ["), Vec::<String>::new());
    }
}
