//! The census half of ADR 3477's closing gate (ADR 3574): every default the three reader crates
//! put in place of a value is either beside a class of repair or says why it is not one.
//!
//! # The rule
//!
//! A **site** is a line of code that calls `unwrap_or…`, `.ok()` or `.clamp(` — the three shapes
//! ADR 3477 names, the ones by which a value the file stated, or failed to state, becomes one the
//! reader chose. The population is every Rust source of `pdf-syntax`, `pdf-model` and `pdf-font`
//! but their test code (a file named `…_tests.rs`, `…_audit.rs`, `tests.rs` or `fixture.rs`, a
//! `tests/` directory, and everything from a file's first top-level `#[cfg(test)]` module on) and
//! their writers (`WRITERS`: what this program emits is not a file it was given). A site is
//! **accounted for** when, within `WINDOW` lines of it,
//!
//! - a class of repair is named — any item a `classes! { … }` block of the three crates declares,
//!   or a call that records one (`record_repair`, `record_repairs`, `note_repair`, `noted`): the
//!   default is a repair, and it is logged; or
//! - a comment above it, or on its own line, begins `// not a repair:` and says why — the clause
//!   that makes the default the specification's own, or that the value is not the file's at all
//!   (a count, an index, a bound of this reader's).
//!
//! The window stands in for ADR 3477's "the function logs": a line count is what a census can
//! check without a parser, and twelve lines is the reach of one statement and its guard here.
//!
//! # Why a ratchet, and why it is per file
//!
//! Most of the sites the rule finds are on values that are no file's (an iterator's maximum, a
//! slice's bound), and each needs a reading before it may be marked — a marker written without one is the census lying
//! in the other direction, and a site read as a repair is left counted until its class is logged.
//! `tools/state.sh` does not print the count; this test does, on every run. So `HELD` keeps each
//! file's count of sites not yet accounted for, and the test fails when a file has **more** (a
//! new default arrived silent) and when it has **fewer** (the bound must fall with the change
//! that earned it, so it can only fall). A file not in `HELD` holds zero. The message prints the
//! table as it should read.
//!
//! # What it cannot see
//!
//! A default written in another shape — a `match` arm, an `if let … else`, a `filter_map` — is
//! outside the rule, which is ADR 3477's own grep; review 2194's inventory read those by hand. A
//! class named within the window that is not the one this default makes is accounted for all the
//! same: the window proves proximity, not meaning.

#![expect(
    clippy::panic,
    reason = "test code: a census that cannot read the tree it counts has not found a defect, and \
              reporting that as one would be worse than stopping"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// The three reader crates.
const CRATES: [&str; 3] = ["pdf-syntax", "pdf-model", "pdf-font"];

/// The writers, by path under `crates/`: ADR 3505's and review 2194's list, the code that emits a
/// file rather than reading one.
const WRITERS: [&str; 7] = [
    "pdf-syntax/src/serialize.rs",
    "pdf-syntax/src/write.rs",
    "pdf-syntax/src/linearize.rs",
    "pdf-font/src/embed.rs",
    "pdf-font/src/embed/",
    "pdf-font/src/embedding.rs",
    "pdf-model/src/view/writing.rs",
];

/// How many lines either side of a site a class, a recording call or a marker may stand.
const WINDOW: usize = 12;

/// How many lines above a site its marker may stand: the marker's comment and the statement the
/// site ends, and no further, so that a marker over one default cannot account for a repair a
/// few lines below it.
const MARKER_REACH: usize = 6;

/// The marker that says a default is the specification's, or the value not the file's.
const MARKER: &str = "// not a repair:";

/// The calls that record a class on the channel.
const RECORDERS: [&str; 4] = ["record_repair", "note_repair", "noted(", "record_repairs"];

/// Each file's sites not yet accounted for, which may only fall (path under `crates/`, count).
const HELD: &[(&str, usize)] = &[
    ("pdf-model/src/appearance.rs", 1),
    ("pdf-model/src/xfdf.rs", 2),
    ("pdf-model/src/view/scripts/keystroke.rs", 2),
    ("pdf-model/src/view/scripts/calculation.rs", 2),
    ("pdf-model/src/view/scripts.rs", 2),
    ("pdf-model/src/view/script_annotations.rs", 2),
    ("pdf-model/src/view/fields.rs", 2),
    ("pdf-model/src/structure/inline.rs", 2),
    ("pdf-model/src/structure.rs", 2),
    ("pdf-model/src/content/transparency/into_parent.rs", 2),
    ("pdf-model/src/content/transparency.rs", 2),
    ("pdf-model/src/content/divide/pipeline.rs", 2),
    ("pdf-model/src/attachment/filing.rs", 2),
    ("pdf-model/src/aform/number.rs", 2),
    ("pdf-model/src/accessibility.rs", 2),
    ("pdf-font/src/loading.rs", 2),
    ("pdf-syntax/src/document/sources.rs", 1),
    ("pdf-syntax/src/document/decoding.rs", 1),
    ("pdf-model/src/xfa/picture.rs", 1),
    ("pdf-model/src/xfa/package.rs", 1),
    ("pdf-model/src/xfa/calculate.rs", 1),
    ("pdf-model/src/view/scripts/queries.rs", 1),
    ("pdf-model/src/view/scripts/commit.rs", 1),
    ("pdf-model/src/view/markup.rs", 1),
    ("pdf-model/src/variable_text.rs", 1),
    ("pdf-model/src/structure/attributes.rs", 1),
    ("pdf-model/src/rich_text/span.rs", 1),
    ("pdf-model/src/rich_text.rs", 1),
    ("pdf-model/src/requirements.rs", 1),
    ("pdf-model/src/reference.rs", 1),
    ("pdf-model/src/media/playing.rs", 1),
    ("pdf-model/src/media/container/asf.rs", 1),
    ("pdf-model/src/media/container.rs", 1),
    ("pdf-model/src/media.rs", 1),
    ("pdf-model/src/image/ccitt.rs", 1),
    ("pdf-model/src/form.rs", 1),
    ("pdf-model/src/document_part.rs", 1),
    ("pdf-model/src/content/transparency/soft_mask.rs", 1),
    ("pdf-model/src/content/run/xobject_ops.rs", 1),
    ("pdf-model/src/content/run/text_ops.rs", 1),
    ("pdf-model/src/content/run/nested.rs", 1),
    ("pdf-model/src/content/pattern/tiling.rs", 1),
    ("pdf-model/src/content/pattern/lattice.rs", 1),
    ("pdf-model/src/content/pattern.rs", 1),
    ("pdf-model/src/content/marked.rs", 1),
    ("pdf-model/src/content/divide.rs", 1),
    ("pdf-model/src/content/annotations.rs", 1),
    ("pdf-model/src/aform/call.rs", 1),
    ("pdf-font/src/vertical.rs", 1),
    ("pdf-font/src/tounicode.rs", 1),
    ("pdf-font/src/type1.rs", 1),
    ("pdf-font/src/shaping/mod.rs", 1),
    ("pdf-font/src/program.rs", 1),
    ("pdf-font/src/panose.rs", 1),
    ("pdf-font/src/loading/program.rs", 1),
    ("pdf-font/src/loading/opening.rs", 1),
    ("pdf-font/src/loading/advances.rs", 1),
    ("pdf-font/src/glyph_names.rs", 1),
    ("pdf-font/src/composite.rs", 1),
];

/// Whether `relative` (under `crates/`) is test code or a writer.
fn outside_the_population(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or(relative);
    WRITERS.iter().any(|writer| relative.starts_with(writer))
        || relative.contains("/tests/")
        || name.ends_with("_tests.rs")
        || name.ends_with("_audit.rs")
        || name == "tests.rs"
        || name == "fixture.rs"
}

/// Every `.rs` file under `directory`, recursively, in path order.
fn sources(directory: &Path, into: &mut Vec<PathBuf>) {
    let listing = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{} cannot be listed: {error}", directory.display()));
    for item in listing {
        let path = item
            .unwrap_or_else(|error| panic!("{} cannot be listed: {error}", directory.display()))
            .path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

/// The lines before the first top-level `#[cfg(test)]` module, keeping the code after a
/// `#[cfg(test)] mod name;` declaration.
fn code_lines(source: &str) -> Vec<&str> {
    let mut kept = Vec::new();
    let mut lines = source.lines();
    while let Some(line) = lines.next() {
        if line == "#[cfg(test)]" {
            match lines.next() {
                Some(next) if next.trim_end().ends_with(';') => continue,
                _ => break,
            }
        }
        kept.push(line);
    }
    kept
}

/// Whether `line` is a site: code, not a comment, calling one of the three shapes.
fn is_site(line: &str) -> bool {
    let code = line.split_once("//").map_or(line, |(code, _)| code);
    code.contains("unwrap_or") || code.contains(".ok()") || code.contains(".clamp(")
}

/// The item names every `classes! { … }` block of the three crates declares.
fn class_items(root: &Path) -> BTreeSet<String> {
    let mut items = BTreeSet::new();
    for krate in CRATES {
        let path = root.join(format!("crates/{krate}/src/repair.rs"));
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} cannot be read: {error}", path.display()));
        let mut inside = false;
        for line in source.lines() {
            if line.starts_with("classes! {") {
                inside = true;
            } else if inside && line.starts_with('}') {
                break;
            } else if inside
                && let Some((item, rest)) = line.trim_start().split_once(" = \"")
                && rest.contains('/')
                && !item.is_empty()
                && item
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                items.insert(item.to_owned());
            }
        }
    }
    items
}

/// Whether `line` names a class item as a whole word.
fn names_a_class(line: &str, items: &BTreeSet<String>) -> bool {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|word| items.contains(word))
}

/// Whether the site at `at` is accounted for, by the rule this file's comment states.
fn accounted_for(lines: &[&str], at: usize, items: &BTreeSet<String>) -> bool {
    let from = at.saturating_sub(WINDOW);
    let to = at.saturating_add(WINDOW).saturating_add(1).min(lines.len());
    let marked = lines[at.saturating_sub(MARKER_REACH)..=at]
        .iter()
        .any(|line| {
            line.split_once(MARKER)
                .is_some_and(|(_, why)| why.trim().len() > 1)
        });
    marked
        || lines[from..to].iter().any(|line| {
            RECORDERS.iter().any(|recorder| line.contains(recorder)) || names_a_class(line, items)
        })
}

/// Each file's sites not accounted for (path under `crates/`), and the total population.
fn census(root: &Path, items: &BTreeSet<String>) -> (BTreeMap<String, usize>, usize) {
    let mut silent = BTreeMap::new();
    let mut population: usize = 0;
    for krate in CRATES {
        let mut files = Vec::new();
        sources(&root.join(format!("crates/{krate}/src")), &mut files);
        files.sort();
        for path in files {
            let relative = path
                .strip_prefix(root.join("crates"))
                .unwrap_or_else(|_| panic!("{} is under crates/", path.display()))
                .to_string_lossy()
                .replace('\\', "/");
            if outside_the_population(&relative) {
                continue;
            }
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} cannot be read: {error}", path.display()));
            let lines = code_lines(&source);
            for (at, line) in lines.iter().enumerate() {
                if line.trim_start().starts_with("//") || !is_site(line) {
                    continue;
                }
                population = population.saturating_add(1);
                if !accounted_for(&lines, at, items) {
                    let count = silent.entry(relative.clone()).or_insert(0_usize);
                    *count = count.saturating_add(1);
                }
            }
        }
    }
    (silent, population)
}

#[test]
fn every_default_is_beside_a_class_or_says_why_it_is_none() {
    let root = conformance::workspace_root();
    let items = class_items(&root);
    assert!(
        items.len() > 50,
        "only {} class items were read: the `classes! {{ … }}` grammar has changed under this test",
        items.len()
    );
    let (silent, population) = census(&root, &items);
    let held: BTreeMap<&str, usize> = HELD.iter().copied().collect();
    let total: usize = silent.values().sum();
    println!(
        "{population} sites in the three reader crates, {} accounted for, {total} held",
        population.saturating_sub(total)
    );
    let mut wrong = Vec::new();
    for file in silent
        .keys()
        .map(String::as_str)
        .chain(held.keys().copied())
        .collect::<BTreeSet<_>>()
    {
        let found = silent.get(file).copied().unwrap_or(0);
        let bound = held.get(file).copied().unwrap_or(0);
        if found > bound {
            wrong.push(format!(
                "{file}: {found} sites with no class and no `{MARKER}`, {bound} held — log the \
                 new default's class or mark it"
            ));
        } else if found < bound {
            wrong.push(format!(
                "{file}: {found} sites held at {bound} — lower the ratchet with the change that \
                 earned it"
            ));
        }
    }
    if !wrong.is_empty() {
        let table: Vec<String> = silent
            .iter()
            .map(|(file, count)| format!("    (\"{file}\", {count}),"))
            .collect();
        panic!(
            "ADR 3477's census:\n  {}\n\nHELD as it reads now:\n{}",
            wrong.join("\n  "),
            table.join("\n")
        );
    }
}

/// Trap 13: a planted silent default is counted, a marker or a logged class beside it accounts
/// for it, and the writers and test code stay out.
#[test]
fn the_census_counts_a_planted_default_and_its_answers() {
    let items: BTreeSet<String> = ["XREF_REBUILT".to_owned()].into_iter().collect();
    let silent = ["fn f(d: &D) -> i64 {", "    d.get(\"N\").unwrap_or(0)", "}"];
    assert!(is_site(silent[1]) && !accounted_for(&silent, 1, &items));
    let marked = [
        "    // not a repair: §7.3.10, an absent entry's default.",
        "    d.get(\"N\").unwrap_or(0)",
    ];
    assert!(accounted_for(&marked, 1, &items));
    let empty_marker = ["    // not a repair:", "    d.get(\"N\").unwrap_or(0)"];
    assert!(!accounted_for(&empty_marker, 1, &items));
    let logged = [
        "    let n = d.get(\"N\").unwrap_or(0);",
        "    document.record_repair(&repair::XREF_REBUILT, site);",
    ];
    assert!(accounted_for(&logged, 0, &items));
    assert!(!is_site("    // a comment that says unwrap_or"));
    assert!(outside_the_population("pdf-syntax/src/write.rs"));
    assert!(outside_the_population("pdf-font/src/embed/cff.rs"));
    assert!(outside_the_population("pdf-font/src/cff_reals_tests.rs"));
    assert!(!outside_the_population("pdf-syntax/src/xref.rs"));
}
