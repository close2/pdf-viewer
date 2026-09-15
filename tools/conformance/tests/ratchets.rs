//! Every hand-written bound in a gate file goes through `gate-ratchet`, so it is printed beside
//! the population it bounds and its slack is gated.
//!
//! Not a conformance question, and it lives here for `state_sections.rs`'s and `sandbox_gates.rs`'s
//! reason: this is the crate whose gates read the repository's own files rather than a PDF. It is
//! also a tier-1 line, which is where a check that must run in *every* round belongs — the gates
//! it is about are tier-2 and tier-3 walks, and a check that only ran when one of those ran could
//! not see a bound added beside them.
//!
//! # What it guards
//!
//! A ceiling far above its population cannot fire. `MAX_INCOMPLETE` stood at 91 against a
//! population of 61 and `MAX_PAGELESS` at 6 against 5; both gates print their population on every
//! run and neither printed its bound, so the two numbers never appeared on one line. The bounds in
//! *that* file were then checked by hand, once. This check is the other half: it holds every gate
//! file to the shape that makes the slack visible, on every round, so the checking is not a thing
//! somebody has to remember. ADR 1075 is the argument; `gate_ratchet` is what a call looks like.
//!
//! # Why the population is derived
//!
//! From `doc/todo/02` §2's own fenced blocks, which **own** the gate sequence — the same rule
//! `state_sections.rs` and `sandbox_gates.rs` follow, and the same `-p <package> --test <target>`
//! parse, duplicated here as it is duplicated there rather than exported: three tests reading one
//! document three ways is the drift ADR 0232 §4 is about, and three tests reading it the *same*
//! way is not.
//!
//! A hand-written list of gate files would be trap 25's shape, and the difference is not
//! hypothetical: the list this check was commissioned with named nine files, and the sequence
//! named three more with bounds in them — `pdf-model`'s `dates` and `xmp` gates, carrying five
//! ratchets between them that nobody had looked at.
//!
//! # The two rules, and what they cannot see
//!
//! 1. **A named bound is a `gate_ratchet` argument and nothing else.** An integer `const` in a gate
//!    file whose name is `MAX_…`, `MIN_…`, `…_FLOOR` or `…_CEILING` may appear in code only inside
//!    a call to this crate's `ceiling`, `floor`, `ceiling_with_headroom` or `floor_with_headroom`.
//!    A bare `assert!(count <= MAX_…)` is what hid both defects, and it is what this rejects.
//! 2. **A gate file does not write its own `floor` or `ceiling`.** Two of them did, each printing
//!    nothing, and one held twenty-eight bounds. A helper per file is a rule per file.
//!
//! What neither rule sees is a bound whose name says nothing — a `const THRESHOLD: usize = 40`
//! compared by hand. That is stated rather than papered over: the naming convention is the handle,
//! and a round adding a bound under another name is outside this instrument. Rule 2 narrows it,
//! because the call-site family is where such a bound would most naturally be asserted.
//!
//! # The escape, and why it has to exist
//!
//! Not every `MAX_…` in a gate file bounds a population. A recursion depth, a print limit and a
//! minimum word length are parameters of the instrument rather than facts about the corpus, and
//! ratcheting one would be nonsense. A comment line beginning `// not a ratchet:` within the ten
//! lines above the constant admits it, with the reason after the colon. It is checked in both
//! directions, as `state_sections.rs`'s excuse is: an excuse that admits no constant has outlived
//! its fact.

#![expect(
    clippy::expect_used,
    reason = "test code: a gate that cannot read the repository it is in has not found a defect, \
              and reporting that as one would be worse than stopping"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Where the repository root is, relative to this crate's manifest.
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the manifest directory of a workspace member has two ancestors")
}

/// Every `cargo test … -p <package> --test <target>` line inside a fenced block of `text`.
///
/// `state_sections.rs`'s parse, and its comment says why the fence matters: §2 also *mentions*
/// gate lines in prose, and a scrape over every line reads those as commands.
fn gate_targets(text: &str) -> BTreeSet<(String, String)> {
    let mut found = BTreeSet::new();
    let mut fenced = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            fenced = line.starts_with("```sh") || line.starts_with("```bash");
            continue;
        }
        if !fenced || !line.contains("cargo test") {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (mut package, mut target) = (None, None);
        while let Some(field) = fields.next() {
            match field {
                "-p" => package = fields.next().map(str::to_owned),
                "--test" => target = fields.next().map(str::to_owned),
                _ => {}
            }
        }
        if let (Some(package), Some(target)) = (package, target) {
            found.insert((package, target));
        }
    }
    found
}

/// The file a `-p <package> --test <target>` line runs, where the tree holds one.
fn gate_file(root: &Path, package: &str, target: &str) -> Option<PathBuf> {
    ["crates", "tools", "raster/crates"]
        .into_iter()
        .map(|members| {
            root.join(members)
                .join(package)
                .join("tests")
                .join(format!("{target}.rs"))
        })
        .find(|path| path.exists())
}

/// Whether `name` is the name this project gives a bound on a population.
///
/// The four spellings the tree uses, and the module comment says what the convention cannot see.
fn names_a_bound(name: &str) -> bool {
    name.starts_with("MAX_")
        || name.starts_with("MIN_")
        || name.ends_with("_FLOOR")
        || name.ends_with("_CEILING")
}

/// The name of the integer `const` declared on `line`, where the line declares one.
///
/// A `&[&str]` named `…_FLOOR` is a *named population* rather than a bound — the strongest shape
/// this project has, held by equality in both directions — and a `f64` tolerance is a verdict
/// threshold and not a count. Only an integer literal is a bound.
fn integer_constant(line: &str) -> Option<&str> {
    let declaration = line.trim_start();
    let rest = declaration
        .strip_prefix("const ")
        .or_else(|| declaration.strip_prefix("pub const "))?;
    let (name, rest) = rest.split_once(':')?;
    let (kind, value) = rest.split_once('=')?;
    let kind = kind.trim();
    let integer = matches!(kind, "usize" | "isize")
        || matches!(kind.split_at_checked(1), Some(("u" | "i", width)) if !width.is_empty()
            && width.chars().all(|digit| digit.is_ascii_digit()));
    let value = value.trim().trim_end_matches(';');
    let numeric = !value.is_empty()
        && value
            .chars()
            .all(|digit| digit.is_ascii_digit() || digit == '_');
    if integer && numeric {
        Some(name.trim())
    } else {
        None
    }
}

/// The line spans covered by a call into `gate_ratchet`, as half-open `(first, last)` pairs.
///
/// A call runs from the line naming the crate to the first line whose trimmed text ends the
/// statement. rustfmt decides where the arguments break, so the span is read rather than assumed.
fn call_spans(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut open: Option<usize> = None;
    for (index, line) in lines.iter().enumerate() {
        if open.is_none() && line.contains("gate_ratchet::") {
            open = Some(index);
        }
        if let Some(first) = open
            && line.trim_end().ends_with(");")
        {
            spans.push((first, index));
            open = None;
        }
    }
    spans
}

/// Whether `index` is a line inside one of `spans`.
fn inside(spans: &[(usize, usize)], index: usize) -> bool {
    spans
        .iter()
        .any(|&(first, last)| index >= first && index <= last)
}

/// Whether `name` occurs on `line` as a whole word.
fn mentions(line: &str, name: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(name).any(|(at, _)| {
        let before = line[..at].chars().next_back().is_none_or(|c| !word(c));
        let after = line[at.saturating_add(name.len())..]
            .chars()
            .next()
            .is_none_or(|c| !word(c));
        before && after
    })
}

/// The comment a constant carries when it is deliberately not a ratchet, with its reason after it.
///
/// Spelled in two pieces so that this file, which is not in the population it scans but could
/// become so, does not carry what it looks for — `state_sections.rs`'s rule for the same hazard.
const EXCUSE: &str = concat!("// not a ratchet", ":");

/// How many lines above a constant the excuse may sit: its doc comment, and no further.
const EXCUSE_REACH: usize = 10;

/// What one gate file yielded: the defects by name, and the two things that were in order.
#[derive(Default)]
struct Findings {
    /// A bound compared by hand, with where it is compared.
    bare: Vec<String>,
    /// A `floor` or `ceiling` the file wrote for itself.
    own_helper: Vec<String>,
    /// An excuse admitting no constant.
    stale_excuse: Vec<String>,
    /// Bounds that do go through the crate.
    routed: usize,
    /// Constants a `// not a ratchet:` line admits.
    excused: usize,
}

impl Findings {
    /// Absorbs one file's findings, so the loop below is a fold rather than five accumulators.
    fn absorb(&mut self, from: Self) {
        self.bare.extend(from.bare);
        self.own_helper.extend(from.own_helper);
        self.stale_excuse.extend(from.stale_excuse);
        self.routed = self.routed.saturating_add(from.routed);
        self.excused = self.excused.saturating_add(from.excused);
    }
}

/// Reads one gate file against the two rules, naming the file and line in everything it reports.
fn scan(shown: &str, source: &str) -> Findings {
    let lines: Vec<&str> = source.lines().collect();
    let spans = call_spans(&lines);
    let mut findings = Findings::default();

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("fn floor(")
            || trimmed.starts_with("fn ceiling(")
            || trimmed.starts_with("let floor =")
            || trimmed.starts_with("let ceiling =")
        {
            findings
                .own_helper
                .push(format!("{shown}:{}: {trimmed}", index.saturating_add(1)));
        }
    }

    let mut unclaimed: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with(EXCUSE))
        .map(|(index, _)| index)
        .collect();

    for (index, line) in lines.iter().enumerate() {
        let Some(name) = integer_constant(line).filter(|name| names_a_bound(name)) else {
            continue;
        };
        let reach = index.saturating_sub(EXCUSE_REACH);
        if let Some(at) = unclaimed.iter().position(|&at| at >= reach && at < index) {
            unclaimed.remove(at);
            findings.excused = findings.excused.saturating_add(1);
            continue;
        }
        let uses: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|&(at, line)| {
                at != index && !line.trim_start().starts_with("//") && mentions(line, name)
            })
            .map(|(at, _)| at)
            .collect();
        let outside: Vec<usize> = uses
            .iter()
            .copied()
            .filter(|&at| !inside(&spans, at))
            .collect();
        if uses.is_empty() || !outside.is_empty() {
            let where_ = outside.first().map_or_else(
                || "it is never used".to_owned(),
                |at| format!("line {}", at.saturating_add(1)),
            );
            findings.bare.push(format!(
                "{shown}:{}: {name} ({where_})",
                index.saturating_add(1)
            ));
        } else {
            findings.routed = findings.routed.saturating_add(1);
        }
    }

    for at in unclaimed {
        findings
            .stale_excuse
            .push(format!("{shown}:{}", at.saturating_add(1)));
    }
    findings
}

#[test]
fn every_hand_written_bound_in_a_gate_is_printed_beside_its_population() {
    let root = repository_root();
    let sequence = std::fs::read_to_string(root.join("doc/todo/02-every-round.md"))
        .expect("doc/todo/02-every-round.md is this gate's population");
    let targets = gate_targets(&sequence);
    let files: Vec<PathBuf> = targets
        .iter()
        .filter_map(|(package, target)| gate_file(root, package, target))
        .collect();
    assert!(
        files.len() > 15,
        "doc/todo/02 §2 yielded {} gate files, so this check is measuring nothing — either a file \
         moved or the parse stopped working",
        files.len()
    );

    let mut found = Findings::default();
    for path in &files {
        let source = std::fs::read_to_string(path).expect("a gate file the sequence names");
        let shown = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        found.absorb(scan(&shown, &source));
    }
    let Findings {
        bare,
        own_helper,
        stale_excuse,
        routed,
        excused,
    } = found;

    println!(
        "ratchets: {} gate file(s) from doc/todo/02 §2, {routed} bound(s) through gate_ratchet, \
         {excused} excused as not a ratchet",
        files.len()
    );
    assert!(
        own_helper.is_empty(),
        "a gate file writes its own floor or ceiling, which prints nothing and is a rule that \
         stops at that file — call gate_ratchet instead:\n  {}",
        own_helper.join("\n  ")
    );
    assert!(
        bare.is_empty(),
        "a bound is compared by hand, so nothing puts it beside the population it bounds and its \
         slack is invisible — the shape MAX_INCOMPLETE had at 91 against 61. Pass it to \
         gate_ratchet::ceiling or ::floor, or say `{EXCUSE} <reason>` above it:\n  {}",
        bare.join("\n  ")
    );
    assert!(
        stale_excuse.is_empty(),
        "`{EXCUSE}` sits above no constant this check would otherwise name, so it has outlived \
         its fact — delete it:\n  {}",
        stale_excuse.join("\n  ")
    );
    // **This floor was 10 and is 8, and the reason is a conversion rather than a loss.** The
    // thousand-and-sixty-seventh session turned three of `corpus.rs`'s integer ceilings — the
    // documents that need a password, the one encryption this reader declines, the five with no
    // reachable first page — into named populations held by `gate_ratchet::population`, which is
    // the stronger shape and the one `integer_constant` deliberately does not count (ADR 1081). A
    // `&[&str]` is not a bound this scan can read, so a gate getting *better* takes three off this
    // number. What the floor still guards is the scan itself: a reading of two would be the parse
    // breaking rather than the gates going bare.
    assert!(
        routed > 8,
        "only {routed} bound(s) were found routed through gate_ratchet, which is fewer than the \
         tree holds — the scan is reading the sources wrongly rather than the gates being bare"
    );
}

/// Trap 13: the scan above has to be shown naming what it looks for, or its clean run is a
/// sentence about the scan.
///
/// Each of the three defects is planted into a source of the shape a gate file has, and the
/// helpers that decide are asked directly — the same construction `corpus.rs`'s own calibration
/// test uses for its classification.
#[test]
fn the_scan_names_each_defect_it_looks_for() {
    let bare = [
        "const MAX_LOCKED: usize = 10;",
        "    assert!(n <= MAX_LOCKED);",
    ];
    let spans = call_spans(&bare);
    assert_eq!(
        integer_constant(bare[0]),
        Some("MAX_LOCKED"),
        "an integer const declares a bound"
    );
    assert!(
        !inside(&spans, 1),
        "a bare assert is outside every gate_ratchet call, which is the defect"
    );

    let routed = [
        "const MAX_LOCKED: usize = 10;",
        "    gate_ratchet::ceiling(",
        "        \"documents that need a password\",",
        "        tally.locked.len(),",
        "        MAX_LOCKED,",
        "    );",
    ];
    let spans = call_spans(&routed);
    assert!(
        inside(&spans, 4),
        "a bound inside a multi-line gate_ratchet call is routed, however rustfmt broke it"
    );

    // The three shapes that are *not* a bound on a population, each of which the tree holds and
    // none of which may be dragged into a ratchet by its name.
    assert_eq!(
        integer_constant("const TEXT_BELOW_FLOOR: [&str; 22] = ["),
        None,
        "a named population is not a bound"
    );
    assert_eq!(
        integer_constant("const PDFJS_FLOOR: f64 = 0.90;"),
        None,
        "a per-document quality threshold is not a count"
    );
    assert!(
        !names_a_bound("PER_DOCUMENT_BUDGET"),
        "a time budget is not spelled like a bound on a population"
    );

    // And the word match is a word match: a longer name that contains a bound's name is not it.
    assert!(mentions("        MAX_LOCKED,", "MAX_LOCKED"));
    assert!(!mentions("    MAX_LOCKED_EXTRA,", "MAX_LOCKED"));
}
