//! The tree's own `.gitignore` patterns, read so that a sweep can say *which* one ignores a path.
//!
//! # Why a matcher here rather than a list
//!
//! [`crate::pointers`] classes a pointer to something a checkout deliberately lacks as not
//! carried rather than dead. That class was a hand-written list beside `.gitignore`, and a list
//! beside the file that decides is trap 25's shape: the two agree until one of them is edited.
//! The patterns are the deciding text, so the sweep reads them and prints the one that matched
//! (ADR 1391).
//!
//! # The subset
//!
//! What gitignore(5) states for the patterns this tree writes, and nothing it does not use:
//!
//! - blank lines and `#` comments carry nothing; trailing spaces are not part of a pattern;
//! - a leading `!` re-includes, and the last pattern that matches a path decides;
//! - a trailing `/` matches a directory only;
//! - a pattern with a `/` at its start or in its middle is anchored to the directory of the
//!   `.gitignore` it is in, and one without is matched against a name at any depth below it;
//! - `*` and `?` match within one segment, and a whole segment `**` matches any number of them;
//! - a path is ignored when it or a directory above it is.
//!
//! A bracket class (`[abc]`) is matched literally: no `.gitignore` in this tree writes one.
//!
//! A pointer names a path that may not exist, so whether its last segment is a directory is
//! unknown; a directory-only pattern is taken to match it, because a pointer at a name like
//! `__pycache__` names the directory.

use std::path::Path;

/// One pattern, and where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    /// The `.gitignore` it is in, relative to the workspace root.
    pub file: String,
    /// The line as written, trailing spaces removed.
    pub text: String,
    /// The directory the pattern is relative to: empty for the root's file.
    base: String,
    /// Whether it re-includes (`!`).
    negated: bool,
    /// Whether it matches a directory only (a trailing `/`).
    directory_only: bool,
    /// Whether it is matched against the whole path below `base`, rather than a name.
    anchored: bool,
    /// The pattern's segments.
    segments: Vec<String>,
}

impl Pattern {
    /// Reads one line of `.gitignore` text; `None` for a blank line or a comment.
    fn parse(file: &str, base: &str, line: &str) -> Option<Self> {
        let text = line.trim_end_matches(' ');
        if text.is_empty() || text.starts_with('#') {
            return None;
        }
        let (negated, body) = match text.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (directory_only, body) = match body.strip_suffix('/') {
            Some(rest) => (true, rest),
            None => (false, body),
        };
        let anchored = body.contains('/');
        let body = body.strip_prefix('/').unwrap_or(body);
        if body.is_empty() {
            return None;
        }
        Some(Self {
            file: file.to_owned(),
            text: text.to_owned(),
            base: base.to_owned(),
            negated,
            directory_only,
            anchored,
            segments: body.split('/').map(str::to_owned).collect(),
        })
    }

    /// Whether this pattern matches exactly this path (not an ancestor of it).
    ///
    /// `is_directory` is `None` where the path's kind is unknown — a pointer to nothing.
    fn matches(&self, path: &str, is_directory: Option<bool>) -> bool {
        if self.directory_only && is_directory == Some(false) {
            return false;
        }
        let below = if self.base.is_empty() {
            path
        } else {
            match path
                .strip_prefix(&self.base)
                .and_then(|rest| rest.strip_prefix('/'))
            {
                Some(rest) => rest,
                None => return false,
            }
        };
        let path_segments: Vec<&str> = below.split('/').collect();
        if self.anchored {
            segments_match(&self.segments, &path_segments)
        } else {
            path_segments
                .last()
                .zip(self.segments.first())
                .is_some_and(|(name, pattern)| wildcard(pattern.as_bytes(), name.as_bytes()))
        }
    }
}

/// Every pattern of the tree's `.gitignore` files, in the order git gives them precedence.
#[derive(Debug, Clone, Default)]
pub struct Ignore {
    patterns: Vec<Pattern>,
}

impl Ignore {
    /// Adds the patterns of one `.gitignore`, whose path relative to the root is `file`.
    ///
    /// Files are added shallowest first — the order a depth-first walk meets them — because a
    /// deeper file's patterns take precedence over its parent's, and the last match decides.
    pub fn add(&mut self, file: &str, text: &str) {
        let base = file
            .strip_suffix(".gitignore")
            .unwrap_or_default()
            .trim_end_matches('/');
        self.patterns.extend(
            text.lines()
                .filter_map(|line| Pattern::parse(file, base, line)),
        );
    }

    /// Reads one `.gitignore` from disk and adds it.
    ///
    /// # Errors
    ///
    /// If the file cannot be read: a sweep that skipped a pattern file would class what it
    /// ignores as dead.
    pub fn read(&mut self, root: &Path, file: &str) -> std::io::Result<()> {
        let text = std::fs::read_to_string(root.join(file))?;
        self.add(file, &text);
        Ok(())
    }

    /// The pattern that ignores this path or a directory above it, or `None` where nothing does.
    ///
    /// `is_directory` is what the caller knows of the path itself; every ancestor is a directory.
    #[must_use]
    pub fn ignoring(&self, path: &str, is_directory: Option<bool>) -> Option<&Pattern> {
        let segments: Vec<&str> = path.split('/').collect();
        for end in 1..=segments.len() {
            let prefix = segments.get(..end).unwrap_or_default().join("/");
            let kind = if end == segments.len() {
                is_directory
            } else {
                Some(true)
            };
            if let Some(pattern) = self.last_match(&prefix, kind)
                && !pattern.negated
            {
                return Some(pattern);
            }
        }
        None
    }

    /// The last pattern matching exactly this path, negated or not.
    fn last_match(&self, path: &str, is_directory: Option<bool>) -> Option<&Pattern> {
        self.patterns
            .iter()
            .rev()
            .find(|pattern| pattern.matches(path, is_directory))
    }
}

/// Whether pattern segments match path segments, `**` taking any number of them.
fn segments_match(pattern: &[String], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((first, rest)) if first == "**" => {
            (0..=path.len()).any(|skip| segments_match(rest, path.get(skip..).unwrap_or_default()))
        }
        Some((first, rest)) => path.split_first().is_some_and(|(segment, remaining)| {
            wildcard(first.as_bytes(), segment.as_bytes()) && segments_match(rest, remaining)
        }),
    }
}

/// Whether one segment matches one name: `*` any run, `?` one character.
fn wildcard(pattern: &[u8], name: &[u8]) -> bool {
    match pattern.split_first() {
        None => name.is_empty(),
        Some((b'*', rest)) => {
            (0..=name.len()).any(|skip| wildcard(rest, name.get(skip..).unwrap_or_default()))
        }
        Some((b'?', rest)) => name
            .split_first()
            .is_some_and(|(_, remaining)| wildcard(rest, remaining)),
        Some((literal, rest)) => name
            .split_first()
            .is_some_and(|(first, remaining)| first == literal && wildcard(rest, remaining)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ignore() -> Ignore {
        let mut ignore = Ignore::default();
        ignore.add(
            ".gitignore",
            "/target\n**/*.rs.bk\n*.profraw\n\n# a comment\n__pycache__/\n/doc/*.pdf\n/doc/md\n.cargo/config.toml\n/fuzz/corpus\n",
        );
        ignore.add("doc/.gitignore", "pdfa\nveraPDF-library\n");
        ignore
    }

    fn text(pattern: Option<&Pattern>) -> Option<(&str, &str)> {
        pattern.map(|pattern| (pattern.file.as_str(), pattern.text.as_str()))
    }

    /// An anchored pattern is relative to its file's directory, and covers what is below it.
    #[test]
    fn an_anchored_pattern_covers_its_path_and_what_is_below_it() {
        let ignore = ignore();
        assert_eq!(
            text(ignore.ignoring("doc/md/ISO_32000-2/7.md", None)),
            Some((".gitignore", "/doc/md"))
        );
        assert_eq!(
            text(ignore.ignoring("doc/ISO_32000-2.pdf", None)),
            Some((".gitignore", "/doc/*.pdf"))
        );
        // `*` stays within a segment, and the anchor is the root.
        assert_eq!(ignore.ignoring("doc/adr/one.pdf", None), None);
        assert_eq!(ignore.ignoring("crates/target", None), None);
        assert_eq!(
            text(ignore.ignoring(".cargo/config.toml", Some(false))),
            Some((".gitignore", ".cargo/config.toml"))
        );
    }

    /// A pattern without a slash names a name at any depth below its own directory, and a
    /// nested file's patterns reach nothing outside it.
    #[test]
    fn an_unanchored_pattern_is_a_name_at_any_depth_below_its_file() {
        let ignore = ignore();
        assert_eq!(
            text(ignore.ignoring("doc/pdfa/19005-2.pdf", None)),
            Some(("doc/.gitignore", "pdfa"))
        );
        assert_eq!(ignore.ignoring("crates/pdfa", None), None);
        assert_eq!(
            text(ignore.ignoring("crates/pdf-model/src/x.rs.bk", Some(false))),
            Some((".gitignore", "**/*.rs.bk"))
        );
        assert_eq!(
            text(ignore.ignoring("fuzz/default.profraw", Some(false))),
            Some((".gitignore", "*.profraw"))
        );
    }

    /// A directory-only pattern does not match a file, and matches a path of unknown kind.
    #[test]
    fn a_directory_only_pattern_skips_a_file() {
        let ignore = ignore();
        assert_eq!(ignore.ignoring("fuzz/__pycache__", Some(false)), None);
        assert!(ignore.ignoring("fuzz/__pycache__", None).is_some());
        assert!(
            ignore
                .ignoring("fuzz/__pycache__/seed.pyc", Some(false))
                .is_some()
        );
    }

    /// The last match decides, so a negation re-includes what an earlier line excluded.
    #[test]
    fn a_negation_re_includes() {
        let mut ignore = Ignore::default();
        ignore.add(".gitignore", "*.log\n!keep.log\n");
        assert!(ignore.ignoring("a.log", Some(false)).is_some());
        assert_eq!(ignore.ignoring("keep.log", Some(false)), None);
    }
}
