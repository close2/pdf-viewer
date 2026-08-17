//! Which corpus documents state a given name, asked at three layers so that the layers disagree.
//!
//! The instrument the five-hundred-and-seventieth session owed. ADR 0403 found that `grep -rl`
//! classes a PDF as binary and suppresses the match, so a ledger row had spent thirty-one rounds
//! saying the corpus stated no `FieldMDP` transform on the strength of a measurement of `grep`.
//! `grep -a` fixes that one failure and leaves a larger one standing: **a name inside an object
//! stream or a content stream is not in the file's bytes at all**, because those bytes are
//! deflated. A raw byte search — with or without `-a` — cannot see them.
//!
//! So each term is asked three times, of the same document:
//!
//! | layer | what it sees | what it misses |
//! |---|---|---|
//! | `raw` | the file's own bytes, which is exactly what `grep -a` reads | anything compressed |
//! | `objects` | every object the cross-reference table names, **including the ones inside §7.5.7 object streams**, walked as names rather than as text | a name that appears only inside stream *data* |
//! | `streams` | every stream's decoded data — content streams, embedded files, appearance streams | nothing, but it matches text rather than syntax, so it over-reports |
//!
//! `objects` is the layer to believe for a claim about what a document *states*: it matches a
//! `Name` token as a token, so `/Lock` does not match the word `Locked` and a name inside a
//! string does not match at all. `raw` is printed beside it precisely so that the gap is visible
//! — a term whose `raw` count is below its `objects` count is a term a grep would have undercounted,
//! and `doc/habits.md`'s *Measuring* section is about exactly that.
//!
//! ```sh
//! cargo run --release -p pdf-model --example witness_census -- Collection Threads Trans Lock
//! cargo run --release -p pdf-model --example witness_census -- --pdfjs Collection  # the 974 only
//! cargo run --release -p pdf-model --example witness_census -- --names        # every name, ranked
//! ```
//!
//! **`--pdfjs` narrows the population to the pdf.js corpus**, which is what most of this project's
//! written claims are about — "the 974". Without it the four `doc/corpora/` submodules and this
//! project's own fixtures are included, which is the population ADR 0403 measured. A claim is only
//! true or false against a stated population, so the scope is a flag rather than a default.
//!
//! With `--names` and no terms it prints how many documents state each distinct name, which is
//! what turns "is there a witness for this entry" into a lookup rather than a run.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus of a few thousand files, four orders of magnitude below \
              what a usize counts; this is a measurement rather than a shipped path"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object, ObjectId};
use rayon::prelude::*;

/// How deep an object's own structure is walked before the walk gives up.
///
/// A document may nest arrays and dictionaries arbitrarily; the walk is over one *object* at a
/// time and never follows a reference, so the depth is the object's own and a small bound is
/// generous. It exists so that a hostile file cannot make this example recurse without end.
const MAX_DEPTH: usize = 64;

/// How many witnessing document names are printed per term before the list is truncated.
const MAX_NAMED: usize = 12;

/// Every PDF this project can measure over: the pdf.js corpus, the four `doc/corpora/`
/// submodules, and this project's own fixtures.
///
/// The population ADR 0403 used, so that a claim re-checked here is re-checked against the same
/// world the claim was made about.
fn corpus(pdfjs_only: bool) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let scope: &[&str] = if pdfjs_only {
        &["doc/pdf.js/test/pdfs"]
    } else {
        &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"]
    };
    for relative in scope {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// Every `.pdf` under one directory, recursively.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pdf")) {
            into.push(path);
        }
    }
}

/// What one document said about one set of terms.
#[derive(Default)]
struct Found {
    /// Terms present in the file's own bytes — what `grep -a` would report.
    raw: BTreeSet<String>,
    /// Terms present as a `Name` token in some object, object streams included.
    objects: BTreeSet<String>,
    /// Terms present in some stream's decoded data.
    streams: BTreeSet<String>,
    /// Every distinct name the document states, when `--names` asked for it.
    names: BTreeSet<String>,
    /// Whether the document could be opened at all.
    opened: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let want_names = args.iter().any(|a| a == "--names");
    let pdfjs_only = args.iter().any(|a| a == "--pdfjs");
    let terms: Vec<String> = args.into_iter().filter(|a| !a.starts_with("--")).collect();
    if terms.is_empty() && !want_names {
        eprintln!("usage: witness_census [--names] [--pdfjs] <name> [<name> ...]");
        return;
    }

    let files = corpus(pdfjs_only);
    eprintln!("{} PDF(s) in the population", files.len());

    let results: Vec<(String, Found)> = files
        .par_iter()
        .map(|path| {
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (label, measure(path, &terms, want_names))
        })
        .collect();

    let opened = results.iter().filter(|(_, f)| f.opened).count();
    println!("{} PDF(s) read, {opened} opened by this tree", results.len());

    if want_names {
        report_names(&results);
    }
    if terms.is_empty() {
        return;
    }

    for term in &terms {
        let mut raw = Vec::new();
        let mut objects = Vec::new();
        let mut streams = Vec::new();
        for (label, found) in &results {
            if found.raw.contains(term) {
                raw.push(label.as_str());
            }
            if found.objects.contains(term) {
                objects.push(label.as_str());
            }
            if found.streams.contains(term) {
                streams.push(label.as_str());
            }
        }
        println!();
        println!(
            "/{term}: {} raw, {} as a name, {} in stream data",
            raw.len(),
            objects.len(),
            streams.len()
        );
        // The `objects` line is the answer; `raw` is printed beside it so that a term a grep
        // would have undercounted is visible as a term, rather than as an argument.
        show("  stated as a name ", &objects);
        show("  only in a stream ", &only(&streams, &objects));
        show("  a raw grep misses", &only(&objects, &raw));
    }
}

/// The labels in `all` that are not in `other`.
fn only<'a>(all: &[&'a str], other: &[&str]) -> Vec<&'a str> {
    all.iter()
        .filter(|label| !other.contains(*label))
        .copied()
        .collect()
}

/// Prints one line of witnesses, truncated so that a large population stays readable.
fn show(what: &str, labels: &[&str]) {
    if labels.is_empty() {
        return;
    }
    let count = labels.len();
    if count > MAX_NAMED {
        println!("{what} ({count}): {} …", labels[..MAX_NAMED].join(" "));
    } else {
        println!("{what} ({count}): {}", labels.join(" "));
    }
}

/// Ranks every distinct name by how many documents state it.
fn report_names(results: &[(String, Found)]) {
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, found) in results {
        for name in &found.names {
            *tally.entry(name.as_str()).or_default() += 1;
        }
    }
    let mut ranked: Vec<(&str, usize)> = tally.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("{} distinct names stated across the population", ranked.len());
    for (name, count) in &ranked {
        println!("  {count:>5}  /{name}");
    }
}

/// Asks one document all three questions.
fn measure(path: &Path, terms: &[String], want_names: bool) -> Found {
    let mut found = Found::default();
    let Ok(bytes) = std::fs::read(path) else {
        return found;
    };
    for term in terms {
        if contains(&bytes, term.as_bytes()) {
            found.raw.insert(term.clone());
        }
    }
    let Ok(document) = Document::open(bytes) else {
        return found;
    };
    found.opened = true;

    let wanted: BTreeSet<&str> = terms.iter().map(String::as_str).collect();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        walk(&object, 0, &wanted, want_names, &mut found);
        if let Some(stream) = object.as_stream()
            && let Some(data) = document.decoded_stream_data(stream)
        {
            for term in terms {
                if contains(&data, term.as_bytes()) {
                    found.streams.insert(term.clone());
                }
            }
        }
    }
    found
}

/// Collects the names one object states, without following a reference.
///
/// A dictionary's *keys* are names and so are its name-valued entries, and a claim about an
/// entry ("no corpus document states a `/Collection`") is about a key while a claim about a
/// value ("no corpus document states a `/TrapNet`") is about a value. Both go in one set, which
/// over-reports rather than under-reports — the direction a census of absences must err in.
fn walk(
    object: &Object,
    depth: usize,
    wanted: &BTreeSet<&str>,
    want_names: bool,
    found: &mut Found,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let note = |bytes: &[u8], found: &mut Found| {
        let text = String::from_utf8_lossy(bytes).into_owned();
        if wanted.contains(text.as_str()) {
            found.objects.insert(text.clone());
        }
        if want_names {
            found.names.insert(text);
        }
    };
    match object {
        Object::Name(name) => note(name.as_bytes(), found),
        Object::Array(items) => {
            for item in items {
                walk(item, depth + 1, wanted, want_names, found);
            }
        }
        Object::Dictionary(dict) => {
            for (key, value) in dict.iter() {
                note(key.as_bytes(), found);
                walk(value, depth + 1, wanted, want_names, found);
            }
        }
        Object::Stream(stream) => {
            for (key, value) in stream.dict.iter() {
                note(key.as_bytes(), found);
                walk(value, depth + 1, wanted, want_names, found);
            }
        }
        Object::Null
        | Object::Boolean(_)
        | Object::Integer(_)
        | Object::Real(_)
        | Object::String(_)
        | Object::Reference(_) => {}
    }
}

/// Whether `haystack` contains `needle`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
