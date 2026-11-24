//! The closing gate of the silent-repair campaign (ADR 3477, ADR 3573): every class of repair the
//! three reader crates declare is mapped by the validator, every mapping names a class, and every
//! row a mapping names is a row of the validator's table.
//!
//! # Why it exists
//!
//! The reader crates repair what they can — a table rebuilt by scanning, a stream read to its
//! `endstream`, an operand of the wrong type dropped — and each repair is a value of one class on
//! one channel (ADR 3505). `pdf-archive` turns a logged class into a finding through its table
//! `MAPPING`. A class added to a `CLASSES` slice and not to that table would be logged and judged
//! as section 5.1 alone by `breaches`'s fallback, with no one having decided so; this test makes
//! the decision a thing a new class cannot skip.
//!
//! # Why the sources are parsed, not linked
//!
//! This crate has no dependency but `thiserror`, by design (`Cargo.toml` says why): a checker
//! that links the reader crates and `pdf-archive` would build the whole document stack before it
//! could say anything about a citation, in every round. The three slices are written in one
//! macro's grammar (`IDENT = "name", "clause", "instead";`), and the table in three `const fn`
//! calls, so a line-level parse reads both exactly; the parse checks its own population (each
//! slice non-empty and in its crate's family, each table call one of the three) so a grammar
//! that drifted fails here rather than reading as an empty set. `pdf-archive`'s own unit test
//! holds the font half by type as well.

#![expect(
    clippy::panic,
    reason = "test code: a gate that cannot read the files it holds to each other has not found a \
              defect, and reporting that as one would be worse than stopping"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// The crates that declare classes, and the family prefix each one's names carry.
const DECLARERS: [(&str, &str); 3] = [
    ("pdf-syntax", "syntax/"),
    ("pdf-model", "model/"),
    ("pdf-font", "font/"),
];

/// Where `pdf-archive` maps the classes.
const MAPPING_FILE: &str = "crates/pdf-archive/src/repairs.rs";

/// Where `pdf-archive`'s requirement rows are declared, one tranche a file.
const TABLE_DIRECTORY: &str = "crates/pdf-archive/src/table";

/// Section 5.1's row, which every class mapped by `base` or `rows` breaches.
const BASE_STANDARD: &str = "conformance/adheres-to-the-base-standard";

/// One entry of `MAPPING`, as written.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    /// The class name the entry maps.
    class: String,
    /// The rows it names besides section 5.1's, resolved to their identifiers; empty for `base`.
    rows: Vec<String>,
    /// Whether it is a `no_row` entry, and its reason was found.
    no_row: Option<bool>,
}

/// Reads a file of the repository, or fails the test naming it.
fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("{relative} cannot be read: {error}"))
}

/// The source before its first top-level `#[cfg(test)]` item, skipping a `mod name;` declaration
/// under one: a test module's literals are not the crate's declarations.
fn outside_tests(source: &str) -> String {
    let mut kept = String::new();
    let mut lines = source.lines();
    while let Some(line) = lines.next() {
        if line == "#[cfg(test)]" {
            match lines.next() {
                Some(next) if next.trim_end().ends_with(';') => continue,
                _ => break,
            }
        }
        kept.push_str(line);
        kept.push('\n');
    }
    kept
}

/// The first string literal in `text` (no escapes inside: class names and row identifiers are
/// plain words), and what follows it.
fn first_literal(text: &str) -> Option<(&str, &str)> {
    let (_, rest) = text.split_once('"')?;
    rest.split_once('"')
}

/// The class names a `classes! { … }` invocation declares, in order.
fn declared_classes(source: &str) -> Vec<String> {
    let mut inside = false;
    let mut names = Vec::new();
    for line in source.lines() {
        if line.starts_with("classes! {") {
            inside = true;
            continue;
        }
        if inside && line.starts_with('}') {
            break;
        }
        if !inside {
            continue;
        }
        let trimmed = line.trim_start();
        let Some((item, rest)) = trimmed.split_once(" = ") else {
            continue;
        };
        if item.is_empty()
            || !item
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            continue;
        }
        if let Some((name, _)) = first_literal(rest) {
            names.push(name.to_owned());
        }
    }
    names
}

/// The value of each `const NAME: &str = "…"` in `source` whose literal opens on its line.
fn string_constants(source: &str) -> BTreeMap<String, String> {
    let mut constants = BTreeMap::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let trimmed = trimmed
            .strip_prefix("pub(crate) ")
            .or_else(|| trimmed.strip_prefix("pub "))
            .unwrap_or(trimmed);
        let Some(rest) = trimmed.strip_prefix("const ") else {
            continue;
        };
        let Some((name, value)) = rest.split_once(": &str = ") else {
            continue;
        };
        let value = value.trim();
        let Some(body) = value.strip_prefix('"') else {
            continue;
        };
        let text = body
            .split_once('"')
            .map_or(body.trim_end_matches('\\'), |(text, _)| text);
        constants.insert(name.trim().to_owned(), text.trim().to_owned());
    }
    constants
}

/// The text of `MAPPING`'s initialiser with its comments removed.
fn mapping_body(source: &str) -> String {
    let mut inside = false;
    let mut body = String::new();
    for line in source.lines() {
        if line.starts_with("pub static MAPPING: &[Mapped] = &[") {
            inside = true;
            continue;
        }
        if inside && line.starts_with("];") {
            return body;
        }
        if inside {
            let code = line.split_once("//").map_or(line, |(code, _)| code);
            body.push_str(code.trim());
            body.push(' ');
        }
    }
    panic!("{MAPPING_FILE} holds no `pub static MAPPING: &[Mapped] = &[` … `];`");
}

/// Resolves one argument of a `rows` list or a `no_row` reason: a literal, or a constant's name.
fn resolve(argument: &str, constants: &BTreeMap<String, String>) -> Option<String> {
    let argument = argument.trim();
    if let Some((literal, _)) = first_literal(argument) {
        return Some(literal.to_owned());
    }
    constants.get(argument).cloned()
}

/// The entries of `MAPPING`, and every call in it that is not `base`, `rows` or `no_row`.
fn mapping_entries(source: &str) -> (Vec<Entry>, Vec<String>) {
    let constants = string_constants(source);
    let body = mapping_body(source);
    let mut entries = Vec::new();
    let mut strange = Vec::new();
    // Each entry is a call ending in `),`; split on that and read each call.
    for call in body
        .split("),")
        .map(str::trim)
        .filter(|call| !call.is_empty())
    {
        let Some((function, arguments)) = call.split_once('(') else {
            strange.push(call.to_owned());
            continue;
        };
        let Some((class, rest)) = first_literal(arguments) else {
            strange.push(call.to_owned());
            continue;
        };
        let class = class.to_owned();
        match function.trim() {
            "base" => entries.push(Entry {
                class,
                rows: Vec::new(),
                no_row: None,
            }),
            "rows" => {
                let list = rest
                    .split_once("&[")
                    .and_then(|(_, list)| list.split_once(']'))
                    .map_or("", |(list, _)| list);
                let rows = list
                    .split(',')
                    .filter(|argument| !argument.trim().is_empty())
                    .map(|argument| {
                        resolve(argument, &constants)
                            .unwrap_or_else(|| format!("<unresolved {}>", argument.trim()))
                    })
                    .collect();
                entries.push(Entry {
                    class,
                    rows,
                    no_row: None,
                });
            }
            "no_row" => {
                let reason = rest
                    .trim()
                    .trim_start_matches(',')
                    .trim()
                    .trim_end_matches(',')
                    .trim();
                let found = resolve(reason, &constants).is_some_and(|why| why.len() > 1);
                entries.push(Entry {
                    class,
                    rows: Vec::new(),
                    no_row: Some(found),
                });
            }
            _ => strange.push(call.to_owned()),
        }
    }
    (entries, strange)
}

/// Every requirement identifier the validator's table declares.
fn table_rows(root: &Path) -> BTreeSet<String> {
    let mut rows = BTreeSet::new();
    let directory = root.join(TABLE_DIRECTORY);
    let listing = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("{TABLE_DIRECTORY} cannot be listed: {error}"));
    for item in listing {
        let path = item
            .unwrap_or_else(|error| panic!("{TABLE_DIRECTORY} cannot be listed: {error}"))
            .path();
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} cannot be read: {error}", path.display()));
        for line in outside_tests(&source).lines() {
            if let Some(rest) = line.trim_start().strip_prefix("id: ")
                && let Some((id, _)) = first_literal(rest)
            {
                rows.insert(id.to_owned());
            }
        }
    }
    rows
}

/// The three conditions ADR 3477 names, each failure one line.
fn defects(
    classes: &BTreeSet<String>,
    entries: &[Entry],
    strange: &[String],
    rows: &BTreeSet<String>,
) -> Vec<String> {
    let mut defects = Vec::new();
    let mut mapped = BTreeSet::new();
    for entry in entries {
        if !mapped.insert(entry.class.clone()) {
            defects.push(format!("{} is mapped twice", entry.class));
        }
        if !classes.contains(&entry.class) {
            defects.push(format!(
                "{} is mapped, and no reader crate declares it",
                entry.class
            ));
        }
        for row in &entry.rows {
            if !rows.contains(row) {
                defects.push(format!(
                    "{} names {row}, which is no row of pdf-archive's table",
                    entry.class
                ));
            }
        }
        if entry.no_row == Some(false) {
            defects.push(format!(
                "{} is mapped to no row without a reason",
                entry.class
            ));
        }
    }
    for class in classes.difference(&mapped) {
        defects.push(format!(
            "{class} is declared, and pdf_archive::repairs::MAPPING does not map it"
        ));
    }
    for call in strange {
        defects.push(format!(
            "MAPPING holds `{call}`, which is not base, rows or no_row"
        ));
    }
    if !rows.contains(BASE_STANDARD) {
        defects.push(format!("{BASE_STANDARD} is no row of pdf-archive's table"));
    }
    defects
}

/// The repository's declared classes, its table's entries, and its rows.
fn the_tree() -> (BTreeSet<String>, Vec<Entry>, Vec<String>, BTreeSet<String>) {
    let root = conformance::workspace_root();
    let mut classes = BTreeSet::new();
    for (krate, family) in DECLARERS {
        let declared = declared_classes(&read(&root, &format!("crates/{krate}/src/repair.rs")));
        assert!(
            !declared.is_empty(),
            "crates/{krate}/src/repair.rs declares no class this parse can read: its \
             `classes! {{ … }}` grammar has changed under this test"
        );
        for name in declared {
            assert!(
                name.starts_with(family),
                "{krate} declares {name}, outside its family {family}"
            );
            assert!(classes.insert(name.clone()), "{name} is declared twice");
        }
    }
    let (entries, strange) = mapping_entries(&read(&root, MAPPING_FILE));
    let rows = table_rows(&root);
    (classes, entries, strange, rows)
}

#[test]
fn every_repair_class_is_mapped_to_rows_the_validator_holds() {
    let (classes, entries, strange, rows) = the_tree();
    println!(
        "{} classes declared, {} mapped, {} rows in the table",
        classes.len(),
        entries.len(),
        rows.len()
    );
    let defects = defects(&classes, &entries, &strange, &rows);
    assert!(
        defects.is_empty(),
        "ADR 3477's closing gate:\n  {}",
        defects.join("\n  ")
    );
}

/// Trap 13: the gate is shown to fire on each of its conditions, planted into the tree's own
/// population, so a parse that silently read nothing cannot pass.
#[test]
fn the_gate_names_each_planted_defect() {
    let (mut classes, mut entries, strange, rows) = the_tree();
    assert_eq!(
        defects(&classes, &entries, &strange, &rows),
        Vec::<String>::new()
    );
    classes.insert("syntax/planted-and-unmapped".to_owned());
    entries.push(Entry {
        class: "model/planted-and-undeclared".to_owned(),
        rows: Vec::new(),
        no_row: None,
    });
    entries.push(Entry {
        class: "syntax/planted-and-unmapped".to_owned(),
        rows: vec!["planted/no-such-row".to_owned()],
        no_row: None,
    });
    let found = defects(&classes, &entries, &strange, &rows);
    for expected in [
        "model/planted-and-undeclared is mapped, and no reader crate declares it",
        "syntax/planted-and-unmapped names planted/no-such-row, which is no row",
    ] {
        assert!(
            found.iter().any(|defect| defect.starts_with(expected)),
            "the gate missed: {expected}\nit said: {found:?}"
        );
    }
    entries.pop();
    let unmapped = defects(&classes, &entries, &strange, &rows);
    assert!(
        unmapped.iter().any(|defect| defect
            == "syntax/planted-and-unmapped is declared, and pdf_archive::repairs::MAPPING does \
                not map it"),
        "the gate missed an unmapped class: {unmapped:?}"
    );
}
