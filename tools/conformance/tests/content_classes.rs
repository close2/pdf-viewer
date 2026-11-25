//! Every class of repair logged inside content is read by the validator's walk or named as the
//! interpreter's, with its reason (ADR 3630).
//!
//! # Why it exists
//!
//! `pdf-archive`'s survey does not interpret (ADR 3542): it walks the tokens and asks
//! `pdf_model::content::grammar` what the interpreter would have repaired. A class only the
//! interpreter logs reaches the repair log when a page is drawn, which for the converter is after
//! its fence has read the log and decided — the trap ADR 3614 found with marked-content balance and
//! ADR 3629 closed for the rest of what the tokens decide. The grammar's table `COVERAGE` states,
//! for each class, whether the grammar, the reader the walk shares with the interpreter, or the
//! interpreter alone reads it; this gate holds that table to the sources, so a class added to the
//! interpreter cannot reopen the trap unannounced.
//!
//! # Why the sources are parsed, not linked
//!
//! As for `repair_classes.rs`: this crate links nothing but `thiserror`. The classes are referred
//! to as `crate::repair::NAME` in `pdf-model` and `repair::NAME` in `pdf-syntax`'s lexer, and the
//! table's entries as `(&repair::NAME, Reading::…)`, so a line-level reading is exact; it checks
//! its own population so that a drifted spelling fails here rather than reading as nothing.

#![expect(
    clippy::panic,
    reason = "test code: a gate that cannot read the files it holds to each other has not found a \
              defect, and reporting that as one would be worse than stopping"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// The grammar, which holds the table.
const GRAMMAR: &str = "crates/pdf-model/src/content/grammar.rs";

/// Where `pdf-model`'s content reader and interpreter live.
const CONTENT: &str = "crates/pdf-model/src/content";

/// §8.9.7's inline-image scan, which both walks run.
const INLINE_IMAGE: &str = "crates/pdf-model/src/inline_image.rs";

/// The content reader and its inline-object reader, which both walks run.
const READER: &str = "crates/pdf-model/src/content/reader.rs";

/// The lexer under both readers.
const LEXER: &str = "crates/pdf-syntax/src/lexer.rs";

/// The survey's content loop and its inline-image step.
const SURVEY_LOOP: &str = "crates/pdf-archive/src/survey/operators.rs";
const SURVEY_IMAGES: &str = "crates/pdf-archive/src/survey/xobjects.rs";

/// Who reads a class, as the table writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Reading {
    Grammar,
    Reader,
    /// The name of the constant holding the reason.
    Interpreter(String),
}

/// Reads a file of the repository, or fails the test naming it.
fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("{relative} cannot be read: {error}"))
}

/// The source before its first top-level `#[cfg(test)]`: a test module's references are no site.
fn code(source: &str) -> &str {
    source
        .find("\n#[cfg(test)]")
        .map_or(source, |at| source.get(..at).unwrap_or(source))
}

/// Every `NAME` written after `prefix` in `text`, where `NAME` is a class constant's spelling.
fn names_after(text: &str, prefix: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(at) = rest.find(prefix) {
        rest = rest
            .get(at.saturating_add(prefix.len())..)
            .unwrap_or_default();
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        if !name.is_empty() {
            found.insert(name);
        }
    }
    found
}

/// Every `.rs` file under `directory`, recursively.
fn sources(directory: &Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{} cannot be listed: {error}", directory.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
            .path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// The table: each entry's class, keyed `model/NAME` or `syntax/NAME` by the crate it names, and
/// who reads it.
fn table(grammar: &str) -> BTreeMap<String, Reading> {
    let start = grammar
        .find("pub static COVERAGE")
        .unwrap_or_else(|| panic!("{GRAMMAR} declares no COVERAGE"));
    let block = grammar.get(start..).unwrap_or_default();
    let block = block
        .split_once("= &[")
        .unwrap_or_else(|| panic!("{GRAMMAR}'s COVERAGE is no slice"))
        .1;
    let end = block
        .find("\n];")
        .unwrap_or_else(|| panic!("{GRAMMAR}'s COVERAGE does not end"));
    // Without white space, so that an entry `rustfmt` breaks over lines reads as one.
    let block: String = block
        .get(..end)
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let mut entries = BTreeMap::new();
    for entry in block.split("(&").skip(1) {
        let (crate_of, rest) = if let Some(rest) = entry.strip_prefix("pdf_syntax::repair::") {
            ("syntax", rest)
        } else if let Some(rest) = entry.strip_prefix("repair::") {
            ("model", rest)
        } else {
            panic!("a COVERAGE entry names no class: {entry}");
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        let reading = if rest.contains("Reading::Grammar") {
            Reading::Grammar
        } else if rest.contains("Reading::Reader") {
            Reading::Reader
        } else if let Some(at) = rest.find("Reading::Interpreter(") {
            let reason: String = rest
                .get(at.saturating_add("Reading::Interpreter(".len())..)
                .unwrap_or_default()
                .chars()
                .take_while(|c| *c != ')')
                .collect();
            Reading::Interpreter(reason.trim().to_owned())
        } else {
            panic!("COVERAGE's entry for {name} states no reading");
        };
        let key = format!("{crate_of}/{name}");
        assert!(
            entries.insert(key.clone(), reading).is_none(),
            "COVERAGE names {key} twice"
        );
    }
    assert!(
        entries.len() > 20,
        "COVERAGE read as {} entries",
        entries.len()
    );
    entries
}

/// **The gate**: every class `pdf-model`'s content code or `pdf-syntax`'s lexer logs is in the
/// table, every entry is one of them, each reading is where the table says, and the survey runs
/// what the table credits it with.
#[test]
fn every_content_class_is_read_by_the_walk_or_named_as_the_interpreters() {
    let root = conformance::workspace_root();
    let grammar_source = read(&root, GRAMMAR);
    let entries = table(&grammar_source);

    let mut files = Vec::new();
    sources(&root.join(CONTENT), &mut files);
    let mut logged: BTreeSet<String> = BTreeSet::new();
    for file in files.iter().filter(|file| !file.ends_with("grammar.rs")) {
        let text = fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("{} cannot be read: {error}", file.display()));
        logged.extend(
            names_after(code(&text), "crate::repair::")
                .into_iter()
                .map(|name| format!("model/{name}")),
        );
    }
    let inline_image = read(&root, INLINE_IMAGE);
    logged.extend(
        names_after(code(&inline_image), "crate::repair::")
            .into_iter()
            .map(|name| format!("model/{name}")),
    );
    let lexer = read(&root, LEXER);
    let lexical = names_after(code(&lexer), "repair::");
    logged.extend(lexical.iter().map(|name| format!("syntax/{name}")));
    assert!(
        logged.len() > 20 && !lexical.is_empty(),
        "the sources read as {} classes",
        logged.len()
    );

    let mut defects = Vec::new();
    for class in &logged {
        if !entries.contains_key(class) {
            defects.push(format!(
                "{class} is logged inside content and COVERAGE does not say who reads it"
            ));
        }
    }
    let grammar_code = code(&grammar_source);
    let reader_code = format!("{}{}", code(&read(&root, READER)), code(&inline_image));
    for (class, reading) in &entries {
        if !logged.contains(class) {
            defects.push(format!(
                "COVERAGE names {class}, which no content code logs"
            ));
        }
        let (family, name) = class.split_once('/').unwrap_or_default();
        let place = match (family, reading) {
            ("model", Reading::Grammar) => {
                // Once in the table, and at least once where the grammar answers it.
                grammar_code
                    .matches(&format!("repair::{name}"))
                    .count()
                    .saturating_sub(1)
                    > 0
            }
            ("model", Reading::Reader) => reader_code.contains(&format!("repair::{name}")),
            ("syntax", Reading::Reader) => lexical.contains(name),
            (_, Reading::Interpreter(reason)) => {
                grammar_code.contains(&format!("const {reason}: &str = \""))
                    && !grammar_code.contains(&format!("const {reason}: &str = \"\""))
            }
            _ => false,
        };
        if !place {
            defects.push(format!(
                "COVERAGE says {reading:?} reads {class}, and that is not where it is read"
            ));
        }
    }

    // The survey runs what the table credits it with: the grammar's answers at each operator, at
    // each stream's end and at a named property list, and the reader's inline objects.
    let survey_loop = read(&root, SURVEY_LOOP);
    let survey_images = read(&root, SURVEY_IMAGES);
    for (file, text, call) in [
        (SURVEY_LOOP, &survey_loop, "found.classes()"),
        (SURVEY_LOOP, &survey_loop, "grammar.end()"),
        (SURVEY_LOOP, &survey_loop, "grammar.property_list("),
        (SURVEY_LOOP, &survey_loop, "inline_dictionary(reader)"),
        (SURVEY_IMAGES, &survey_images, "inline_image::scan("),
        (
            SURVEY_IMAGES,
            &survey_images,
            "repair::CONTENT_PAST_A_BOUND",
        ),
    ] {
        if !code(text).contains(call) {
            defects.push(format!("{file} no longer calls {call}"));
        }
    }

    assert!(defects.is_empty(), "{}", defects.join("\n"));
}
