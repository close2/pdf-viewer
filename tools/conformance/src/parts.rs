//! The twenty-second sweep: a cardinal counting this tree's own parts, against the workspace.
//!
//! # The shape it exists for
//!
//! The tenth sweep ([`crate::counts`]) reads a cardinal only where it governs one of the ledger's
//! own words for a **row** — `row`, `subclause`, `child`. A sentence that says "both backends" is
//! the same kind of claim about a different population: this tree's own parts, which the workspace
//! states and which a program can therefore count. Nothing read that population at all.
//!
//! **It is a decay detector rather than a mistake detector, and the difference decides how its
//! output is read.** "Both backends" was true when it was written; the tree then grew a third
//! rasteriser, and every sentence that had counted two became wrong without anybody touching it.
//! So the population this sweep walks is *mostly correct sentences*, and what it can offer is an
//! ordering rather than a verdict.
//!
//! The seven-hundred-and-sixty-seventh session found the shape by hand. §8.9.6.2's `shall` —
//! interpolation during stencil masking smooths the mask's edges and does not interpolate the
//! painted colour — was answered in two ledger rows by naming the raster, correctly, and then
//! written down as **"both backends"** when there are three, with the third departing by 131 of
//! 255 on the painted channel (ADR 0697). The same two words stood in `pdf-render`'s
//! `Image::is_smoothed` doc comment, which is the item all three backends call.
//!
//! # The two sides, and why both are derived
//!
//! - **The claim** is a form that *presupposes* the population's size — [`PRESUPPOSING`]'s
//!   determiners, or a cardinal under [`DEFINITE`] — governing one of [`NOUNS`] **immediately**.
//!   Adjacency is the whole of what keeps a restrictive modifier out: "both **native** hosts" is
//!   right about two of three hosts, and a program that read across the adjective would call it
//!   wrong. Presupposition is what keeps a *count of a subset* out, which is the larger population
//!   of the two and is measured under [`DEFINITE`].
//! - **The answer** is [`Membership`], read off the workspace's own files — the member directories
//!   under `crates/` and `tools/`, each package's `src/bin/`, and `.gitmodules`. Not a number in
//!   this module: `CLAUDE.md`'s rule is that a fact which can be counted is not written down, and a
//!   sweep whose right-hand side is a constant measures the session that wrote it (ADR 0397).
//!
//! # The three rungs, and which to read first
//!
//! 1. **[`Rung::Whole`] — the place is upstream of every member of the population.** A comment in
//!    a crate that all three backends depend on cannot be talking about a chosen pair, because
//!    every backend there is downstream of the sentence. `pdf-render` is exactly that crate, and
//!    767's defect is on this rung.
//! 2. **[`Rung::Tree`] — the ledger, or one of this project's undated documents.** A ledger note
//!    describes what *the tree* does, so a count in one is a claim about the whole population.
//! 3. **[`Rung::Dated`] — a decision record, or a place inside the population.** `doc/adr/` is
//!    dated by its own numbering, which is the nineteenth sweep's rule ([`crate::overtaken`]), and
//!    an ADR written before a part existed counted correctly at its date. A comment inside one
//!    backend, or in a crate downstream of one, is usually about the pair under comparison.
//!
//! # The noise, printed rather than filtered
//!
//! - **The pair a comparison is about.** Every cross-backend test in this tree rasterises one
//!   scene two ways and calls them "both backends", correctly. That is what rung 3 is for, and it
//!   is the dominant shape.
//! - **A modifier that follows the noun.** Adjacency stops "both native hosts"; it does not stop
//!   "four submodules under `doc/corpora/`", which is right about the four there and is read here
//!   as a claim about all of them. This is the direction the sweep is loose in and it is left to
//!   the reader, exactly as the eighteenth sweep leaves a partitive with no table to divide it.
//! - **This project's own aphorism.** Trap 2's "a decision either backend can make alone is a
//!   decision neither has made" is a *rule* written verbatim in several files, so it arrives as
//!   half a dozen hits that are one sentence.
//! - **A round's own record of running this sweep.** The ninth sweep fires on the ADR cell that
//!   records it and so does this one: the paragraphs above name a count and a population.
//!
//! # Why it is not a gate
//!
//! ADR 0249's ratio argument, sharpened by what a decay detector is. A count that disagrees with
//! the workspace is a sentence to read, and most of them will be about a pair. It runs in a
//! fraction of a second and its output is read.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::ledger::Ledger;

/// A kind of part this tree has a countable population of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// A rasterising backend — a workspace member whose package name begins [`RENDER`].
    Backend,
    /// A workspace member.
    Crate,
    /// A window a person runs: a member with a `src/bin/` program named [`HOST_PROGRAM`]…
    Host,
    /// A git submodule, as `.gitmodules` states them.
    Submodule,
    /// A separate program the viewer spawns: a `src/bin/` program named …[`WORKER_PROGRAM`].
    Worker,
}

impl Part {
    /// One line naming the population, for the report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Backend => "backends",
            Self::Crate => "crates",
            Self::Host => "hosts",
            Self::Submodule => "submodules",
            Self::Worker => "workers",
        }
    }
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The prefix a rasterising backend's package name carries.
pub const RENDER: &str = "render-";

/// The prefix a host program's name carries under a member's `src/bin/`.
pub const HOST_PROGRAM: &str = "pdf-viewer";

/// The suffix a spawned worker program's name carries under a member's `src/bin/`.
pub const WORKER_PROGRAM: &str = "-worker";

/// The words for this tree's own parts, singular and plural, with the population each names.
///
/// Deliberately short, and every one of them is a noun the *workspace* has a membership for.
/// `module`, `panel`, `gate` and `sweep` are populations this project also counts and no file
/// states, so a cardinal governing one of them is judged by nothing and is not read here.
/// `window` is left out for the opposite reason and it is the sharper one: the workspace *does*
/// state the hosts, and a host may open as many windows as a person asks for — so the membership
/// is not the answer to "two windows" and offering it would be a wrong right-hand side rather
/// than a missing one.
///
/// `rasteriser` and `rasterizer` are one population under two spellings, which is this tree's
/// own usage rather than an indulgence: `doc/traps/pixels-and-rasterisers.md` and
/// `QuorraRasterizer` are both in it.
pub const NOUNS: [(&str, Part); 14] = [
    ("backend", Part::Backend),
    ("backends", Part::Backend),
    ("rasteriser", Part::Backend),
    ("rasterisers", Part::Backend),
    ("rasterizer", Part::Backend),
    ("rasterizers", Part::Backend),
    ("crate", Part::Crate),
    ("crates", Part::Crate),
    ("host", Part::Host),
    ("hosts", Part::Host),
    ("submodule", Part::Submodule),
    ("submodules", Part::Submodule),
    ("worker", Part::Worker),
    ("workers", Part::Worker),
];

/// The determiners that state a population's size by presupposing it.
///
/// "Both backends" does not count two of them; it says there are two. That is what makes it a
/// claim this sweep can judge, and it is the form 767's defect took.
pub const PRESUPPOSING: [(&str, usize); 3] = [("both", 2), ("neither", 2), ("either", 2)];

/// The article that turns a bare cardinal into a claim about the population's size.
///
/// **A bare cardinal is not read at all, and this is the rule that decides it.** "Two backends
/// draw the seam" counts two of them doing something and says nothing about how many there are;
/// "the two backends cannot answer it differently" says the set has two members. Measured on the
/// first run over this tree: reading bare cardinals as well put **293 further disagreements** in
/// the report, and a sample of them was counts of a subset — a host pair, a crate pair, the two
/// rasterisers a comparison names. Judging one would be a guess about what an English sentence
/// governs, which is the judgement every sweep in `doc/todo/01` refuses by construction.
pub const DEFINITE: &str = "the";

/// What the workspace says its own parts are.
///
/// Read from the files rather than stated here, for [`crate::counts`]' reason: a right-hand side
/// written into a program is a measurement of the day it was written.
#[derive(Debug, Clone, Default)]
pub struct Membership {
    /// Every member package's name.
    members: BTreeSet<String>,
    /// The members of each population.
    populations: BTreeMap<Part, BTreeSet<String>>,
    /// Each member's direct dependencies on other members, development ones included.
    edges: BTreeMap<String, BTreeSet<String>>,
    /// How many submodules `.gitmodules` states.
    submodules: usize,
}

/// Why the workspace could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A directory or file could not be read.
    #[error("{path}: {source}")]
    Unreadable {
        /// What was being read.
        path: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
}

impl Membership {
    /// Reads the workspace's own membership out of `root`.
    ///
    /// # Errors
    ///
    /// If a member directory cannot be listed or a manifest cannot be read. A sweep that skipped
    /// what it could not open would answer a claim about the tree with a count of part of it.
    pub fn read(root: &Path) -> Result<Self, Error> {
        let mut found = Self::default();
        let mut directories: BTreeMap<String, PathBuf> = BTreeMap::new();
        for group in crate::SOURCE_ROOTS {
            let base = root.join(group);
            // `fuzz/` is not a workspace member and has no member directories under it; its
            // absence is not a failure to read the workspace.
            let Ok(listing) = std::fs::read_dir(&base) else {
                continue;
            };
            for entry in listing {
                let entry = entry.map_err(|source| Error::Unreadable {
                    path: base.display().to_string(),
                    source,
                })?;
                let manifest = entry.path().join("Cargo.toml");
                let Ok(text) = std::fs::read_to_string(&manifest) else {
                    continue;
                };
                if let Some(name) = package_name(&text) {
                    found.members.insert(name.clone());
                    directories.insert(name, entry.path());
                }
            }
        }

        for (name, directory) in &directories {
            let manifest = directory.join("Cargo.toml");
            let text = std::fs::read_to_string(&manifest).map_err(|source| Error::Unreadable {
                path: manifest.display().to_string(),
                source,
            })?;
            found
                .edges
                .insert(name.clone(), named_members(&text, &found.members));
            if name.starts_with(RENDER) {
                found
                    .populations
                    .entry(Part::Backend)
                    .or_default()
                    .insert(name.clone());
            }
            for program in programs(&directory.join("src").join("bin")) {
                if program.starts_with(HOST_PROGRAM) && !program.ends_with(WORKER_PROGRAM) {
                    found
                        .populations
                        .entry(Part::Host)
                        .or_default()
                        .insert(name.clone());
                }
                if program.ends_with(WORKER_PROGRAM) {
                    found
                        .populations
                        .entry(Part::Worker)
                        .or_default()
                        .insert(name.clone());
                }
            }
        }
        found.populations.insert(Part::Crate, found.members.clone());
        found.submodules = std::fs::read_to_string(root.join(".gitmodules"))
            .unwrap_or_default()
            .matches("[submodule")
            .count();
        Ok(found)
    }

    /// How many members one population has, or `None` where the workspace states none.
    ///
    /// `None` rather than zero: a population no file states is one no claim can be judged
    /// against, and judging one against zero would print every sentence in the tree.
    #[must_use]
    pub fn count(&self, part: Part) -> Option<usize> {
        if part == Part::Submodule {
            return (self.submodules > 0).then_some(self.submodules);
        }
        self.populations
            .get(&part)
            .map(BTreeSet::len)
            .filter(|count| *count > 0)
    }

    /// The members of one population, named so that a hit says what it was judged against.
    #[must_use]
    pub fn members_of(&self, part: Part) -> Vec<String> {
        self.populations
            .get(&part)
            .map(|members| members.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Whether `member` is a workspace member at all.
    #[must_use]
    pub fn holds(&self, member: &str) -> bool {
        self.members.contains(member)
    }

    /// Whether every member of `part` depends on `crate_name`, which is not itself one of them.
    ///
    /// [`Rung::Whole`]'s whole test. A sentence written in a crate the entire population is
    /// downstream of cannot mean a chosen pair of it: there is no pair there to choose.
    #[must_use]
    pub fn upstream_of_all(&self, crate_name: &str, part: Part) -> bool {
        let members = self.populations.get(&part);
        let Some(members) = members else {
            return false;
        };
        if members.is_empty() || members.contains(crate_name) || !self.members.contains(crate_name)
        {
            return false;
        }
        members
            .iter()
            .all(|member| self.reaches(member, crate_name))
    }

    /// Whether `from` depends on `to`, directly or through other members.
    fn reaches(&self, from: &str, to: &str) -> bool {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut pending: Vec<&str> = vec![from];
        while let Some(at) = pending.pop() {
            let Some(edges) = self.edges.get(at) else {
                continue;
            };
            for next in edges {
                if next == to {
                    return true;
                }
                if seen.insert(next.as_str()) {
                    pending.push(next.as_str());
                }
            }
        }
        false
    }
}

/// The `name = "…"` of a `[package]` section.
fn package_name(manifest: &str) -> Option<String> {
    manifest
        .lines()
        .find_map(|line| line.strip_prefix("name = \""))
        .and_then(|rest| rest.split('"').next())
        .map(str::to_owned)
}

/// The other members a manifest names, in any of its dependency sections.
///
/// This workspace pins every internal version once in the root manifest and names it as
/// `member.workspace = true`, so a dependency is a line beginning with a member's own name. Both
/// `[dependencies]` and `[dev-dependencies]` count: a test that rasterises two ways is exactly the
/// place a pair is meant, and the sweep wants to know that the pair is reachable.
fn named_members(manifest: &str, members: &BTreeSet<String>) -> BTreeSet<String> {
    let mut named = BTreeSet::new();
    for line in manifest.lines() {
        let head = line
            .split(['.', ' ', '='])
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        if members.contains(&head) {
            named.insert(head);
        }
    }
    named
}

/// The program names under one `src/bin/` directory.
fn programs(directory: &Path) -> Vec<String> {
    let Ok(listing) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    listing
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let stem = path.file_stem()?.to_str()?.to_owned();
            // A `src/bin/name.rs` file and a `src/bin/name/` module directory are one program,
            // and `viewer-ui` has both. The set below takes care of the duplicate.
            (path.extension().is_none_or(|extension| extension == "rs")).then_some(stem)
        })
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// How close the disagreement is, and therefore which hits to read first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// The place is a crate every member of the population depends on.
    Whole,
    /// The ledger, or one of this project's undated documents.
    Tree,
    /// A decision record, whose number is a date, or a place inside the population.
    Dated,
}

impl Rung {
    /// One line saying what this rung is, for the report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Whole => "in a crate the whole population depends on, so no pair can be meant",
            Self::Tree => "in the ledger or an undated document, which speaks about the tree",
            Self::Dated => "in a dated record, or inside the population, where a pair may be meant",
        }
    }
}

/// One sentence counting a population the workspace counts differently.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Where the sentence is, as `path:line`.
    pub location: String,
    /// Which population it counts.
    pub part: Part,
    /// The word that made the claim.
    pub word: String,
    /// What the sentence says the population is.
    pub stated: usize,
    /// What the workspace says it is.
    pub population: usize,
    /// How close the disagreement is.
    pub rung: Rung,
    /// The sentence, whole, because a hit is a reading list and not a verdict.
    pub sentence: String,
}

/// What one run read and what it found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// How many cardinals govern one of [`NOUNS`] immediately, agreeing or not.
    pub mentions: usize,
    /// How many of those the workspace agrees with.
    pub agreeing: usize,
    /// The disagreements, closest rung first.
    pub findings: Vec<Finding>,
    /// Each population's size, for the line that says what a clean run was clean over.
    pub populations: Vec<(Part, usize)>,
}

impl Report {
    /// How many findings sit on one rung.
    #[must_use]
    pub fn on(&self, rung: Rung) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.rung == rung)
            .count()
    }
}

/// Runs the sweep over the ledger's notes, the tree's comments and this project's prose.
///
/// `sources` are the Rust files under [`crate::SOURCE_ROOTS`] and `documents` the Markdown under
/// `doc/`. Two directories are read by nothing, for the reasons [`crate::retired::NOT_SWEPT`] and
/// [`crate::NOT_SCANNED`] give: a round's own record is not another round's to correct, and this
/// checker's own prose states the wrong counts as examples.
#[must_use]
pub fn sweep(
    membership: &Membership,
    ledger: &Ledger,
    sources: &[(PathBuf, String)],
    documents: &[(PathBuf, String)],
) -> Report {
    let mut places: Vec<(String, Option<String>, String)> = Vec::new();
    for row in &ledger.rows {
        if let Some(note) = row.note.as_deref() {
            places.push((
                format!("{}:{} (§{})", crate::LEDGER, row.line, row.clause),
                None,
                note.to_owned(),
            ));
        }
    }
    for (path, text) in sources {
        let shown = shown(path);
        if shown.starts_with(crate::NOT_SCANNED) {
            continue;
        }
        let owner = crate_of(&shown).filter(|name| membership.holds(name));
        for (line, block) in crate::blockers::comment_blocks(text) {
            places.push((format!("{shown}:{line}"), owner.clone(), block));
        }
    }
    for (path, text) in documents {
        let shown = shown(path);
        if shown.starts_with(crate::retired::NOT_SWEPT) {
            continue;
        }
        for (line, block) in crate::retired::paragraphs(text) {
            places.push((format!("{shown}:{line}"), None, block));
        }
    }

    let mut report = Report {
        populations: [
            Part::Backend,
            Part::Crate,
            Part::Host,
            Part::Submodule,
            Part::Worker,
        ]
        .into_iter()
        .filter_map(|part| membership.count(part).map(|count| (part, count)))
        .collect(),
        ..Report::default()
    };
    for (location, owner, block) in &places {
        for sentence in crate::unread::sentences(block) {
            for claim in claims_in(sentence) {
                let Some(population) = membership.count(claim.part) else {
                    continue;
                };
                report.mentions = report.mentions.saturating_add(1);
                if claim.stated == population {
                    report.agreeing = report.agreeing.saturating_add(1);
                    continue;
                }
                report.findings.push(Finding {
                    location: location.clone(),
                    part: claim.part,
                    word: claim.word,
                    stated: claim.stated,
                    population,
                    rung: rung(membership, location, owner.as_deref(), claim.part),
                    sentence: sentence.to_owned(),
                });
            }
        }
    }
    report.findings.sort_by(|left, right| {
        left.rung
            .cmp(&right.rung)
            .then_with(|| left.location.cmp(&right.location))
    });
    report
}

/// A path as this project writes one.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The workspace member a source path belongs to.
fn crate_of(shown: &str) -> Option<String> {
    let mut parts = shown.split('/');
    let group = parts.next()?;
    let member = parts.next()?;
    (crate::SOURCE_ROOTS.contains(&group) && parts.next().is_some()).then(|| member.to_owned())
}

/// Which rung one place sits on, for one population.
fn rung(membership: &Membership, location: &str, owner: Option<&str>, part: Part) -> Rung {
    if let Some(owner) = owner {
        if membership.upstream_of_all(owner, part) {
            return Rung::Whole;
        }
        return Rung::Dated;
    }
    if location.starts_with(DATED) {
        Rung::Dated
    } else {
        Rung::Tree
    }
}

/// The one directory under `doc/` whose prose carries its own date.
///
/// An ADR number is a date — the nineteenth sweep's rule ([`crate::overtaken`]) — so a decision
/// record counting two backends counted correctly on the day it was written.
pub const DATED: &str = "doc/adr/";

/// One cardinal claim about one population, before the workspace is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// Which population it counts.
    pub part: Part,
    /// The word that made the claim.
    pub word: String,
    /// What it says the population is.
    pub stated: usize,
}

/// Every claim one sentence makes about the size of a population of this tree's own parts.
///
/// Two rules, and each removes a population of sentences that count something narrower:
///
/// - **The noun follows the number immediately.** That is what keeps a restrictive modifier out:
///   "both native hosts" counts the two of three hosts that are native, and a reach of even one
///   word would read it as a claim about all of them.
/// - **The form presupposes the size** — [`PRESUPPOSING`], or a cardinal under [`DEFINITE`].
///   A bare cardinal counts some of the population and claims nothing about it.
#[must_use]
pub fn claims_in(sentence: &str) -> Vec<Claim> {
    let words: Vec<String> = sentence.split_whitespace().map(bare).collect();
    let mut found = Vec::new();
    for (position, word) in words.iter().enumerate() {
        let stated =
            if let Some((_, size)) = PRESUPPOSING.iter().find(|(name, _)| *name == word.as_str()) {
                *size
            } else {
                let definite = position
                    .checked_sub(1)
                    .and_then(|before| words.get(before))
                    .is_some_and(|before| before == DEFINITE);
                // A bare `one` before a noun is English's determiner rather than a quantity — "the
                // one crate that owns it" — which is [`crate::counts`]' own finding about its first
                // run and holds identically here.
                match crate::counts::cardinal_of(word) {
                    Some(count) if definite && count > 1 => count,
                    _ => continue,
                }
            };
        let Some(next) = words.get(position.saturating_add(1)) else {
            continue;
        };
        let Some((_, part)) = NOUNS.iter().find(|(name, _)| *name == next.as_str()) else {
            continue;
        };
        found.push(Claim {
            part: *part,
            word: word.clone(),
            stated,
        });
    }
    found
}

/// One word with this project's markup, punctuation and possessive removed, lower-cased.
fn bare(word: &str) -> String {
    let trimmed = word.trim_matches(|character: char| !character.is_ascii_alphanumeric());
    let trimmed = trimmed
        .strip_suffix("'s")
        .or_else(|| trimmed.strip_suffix("’s"))
        .unwrap_or(trimmed);
    trimmed.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{Claim, Membership, NOUNS, Part, Rung, claims_in, sweep};
    use crate::clause::ClauseNumber;
    use crate::ledger::{Ledger, Row, Status};

    fn row(clause: &str, note: &str) -> Row {
        Row {
            clause: clause.parse::<ClauseNumber>().expect("a clause number"),
            title: "General".to_owned(),
            status: Status::Partial,
            code: Vec::new(),
            test: Vec::new(),
            exclusion: None,
            note: Some(note.to_owned()),
            line: 1,
        }
    }

    /// The workspace this tree is, read off the files rather than written down here.
    fn workspace() -> Membership {
        Membership::read(&crate::workspace_root()).expect("the workspace's own membership")
    }

    /// A modifier between the number and the noun means the sentence counts something narrower,
    /// and the sweep may not read across it. "Both native hosts" is right about three hosts.
    #[test]
    fn a_modifier_between_the_number_and_the_noun_is_a_different_population() {
        assert_eq!(claims_in("Both native hosts draw it."), Vec::new());
        assert_eq!(claims_in("Both toolkit hosts changed."), Vec::new());
        assert_eq!(
            claims_in("Both backends ask this.")
                .first()
                .map(|claim| claim.part),
            Some(Part::Backend)
        );
    }

    /// "Both" states the size by presupposing it; a bare "two" counts two of them and says
    /// nothing about how many there are, so it is not read at all.
    #[test]
    fn only_a_form_that_presupposes_the_size_is_a_claim_about_the_population() {
        let both = claims_in("Both rasterisers agree.");
        assert_eq!(both.len(), 1);
        assert_eq!(both[0].stated, 2);
        assert!(
            claims_in("Two rasterisers agree.").is_empty(),
            "a bare cardinal counts a subset"
        );
        let definite = claims_in("The two rasterisers cannot answer it differently.");
        assert_eq!(definite.len(), 1);
        assert_eq!(definite[0].stated, 2);
        assert!(
            claims_in("The one crate that owns it.").is_empty(),
            "a bare `one` is a determiner rather than a quantity"
        );
    }

    /// The nouns are this tree's own parts and nothing else: a count of the standard's furniture
    /// is judged by nothing here.
    #[test]
    fn a_count_of_something_the_workspace_does_not_state_is_not_a_claim() {
        assert!(claims_in("Four of them are stream filters.").is_empty());
        assert!(claims_in("Three subclauses are owed.").is_empty());
        assert!(NOUNS.iter().all(|(name, _)| name.is_ascii()));
    }

    /// The right-hand side is the workspace's own files, and the populations this project's
    /// prose counts are all in it. A count of zero would print every sentence in the tree, so a
    /// population the workspace does not state is judged by nothing.
    #[test]
    fn the_workspaces_own_membership_is_what_answers_a_claim() {
        let membership = workspace();
        for part in [
            Part::Backend,
            Part::Crate,
            Part::Host,
            Part::Submodule,
            Part::Worker,
        ] {
            let count = membership.count(part).expect("a stated population");
            assert!(count > 1, "{part} is stated by more than one file");
        }
        assert!(
            membership.count(Part::Crate) > membership.count(Part::Backend),
            "every backend is a crate and not every crate is a backend"
        );
    }

    /// `pdf-render` is the crate every backend depends on and depends on none of them, which is
    /// what puts 767's defect on the closest rung; a backend crate is not upstream of itself.
    #[test]
    fn the_crate_every_backend_depends_on_is_upstream_of_the_whole_population() {
        let membership = workspace();
        assert!(membership.upstream_of_all("pdf-render", Part::Backend));
        for backend in membership.members_of(Part::Backend) {
            assert!(
                !membership.upstream_of_all(&backend, Part::Backend),
                "{backend} is a member of its own population"
            );
        }
    }

    /// A ledger note counting a population the workspace counts differently is a hit on the
    /// rung that speaks about the tree, and one that agrees is counted rather than printed.
    #[test]
    fn a_note_that_disagrees_with_the_workspace_is_named_and_one_that_agrees_is_counted() {
        let membership = workspace();
        let population = membership
            .count(Part::Backend)
            .expect("this tree states its backends");
        let ledger = Ledger {
            rows: vec![
                row("8.9.6.2", "Both backends premultiply before they filter."),
                row(
                    "8.9.6.3",
                    &format!("The {population} backends premultiply before they filter."),
                ),
            ],
        };
        let report = sweep(&membership, &ledger, &[], &[]);
        assert_eq!(report.mentions, 2);
        assert_eq!(report.agreeing, 1);
        assert_eq!(report.findings.len(), 1);
        let found = &report.findings[0];
        assert_eq!(found.rung, Rung::Tree);
        assert_eq!(found.stated, 2);
        assert_eq!(found.population, population);
    }

    /// The checker's own directory states the wrong counts as examples, so it witnesses nothing.
    #[test]
    fn the_checkers_own_documentation_is_not_a_place() {
        let membership = workspace();
        let ledger = Ledger { rows: Vec::new() };
        let sources = vec![(
            std::path::PathBuf::from("tools/conformance/src/parts.rs"),
            "//! Both backends ask this rather than deciding for themselves.\n".to_owned(),
        )];
        assert_eq!(sweep(&membership, &ledger, &sources, &[]).mentions, 0);
    }

    /// A `Claim` is compared in the modifier test, so it owes the derive that makes that possible.
    #[test]
    fn a_claim_names_its_population_and_its_word() {
        let claim = claims_in("Both workers are spawned.")
            .first()
            .cloned()
            .expect("the claim");
        assert_eq!(
            claim,
            Claim {
                part: Part::Worker,
                word: "both".to_owned(),
                stated: 2,
            }
        );
    }
}
