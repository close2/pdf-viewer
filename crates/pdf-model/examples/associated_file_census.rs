//! §14.13's associated files, counted by the *form* their file specification takes.
//!
//! §14.13.2 states two forms and one preference: "[t]he file specification for an associated file
//! represents either a file external to the PDF file or an embedded file stream (see 7.11.4,
//! 'Embedded file streams') within the PDF file", and its NOTE 1 says "[b]oth types are allowed
//! for associated files but the embedded form is recommended". A reader that reads only the
//! recommended one drops the other without saying so, and the question this answers is how often
//! that happens: **how many documents state an `/AF` array, how many specifications those arrays
//! hold between them, and how many of those carry no `/EF`.**
//!
//! ```sh
//! cargo run --release -p pdf-model --example associated_file_census -- --pdfjs
//! cargo run --release -p pdf-model --example associated_file_census -- --crawl
//! ```
//!
//! The walk is the object graph rather than the eight carriers §14.13.1 lists, because an `/AF`
//! array means the same thing wherever it hangs and enumerating the carriers would measure this
//! program's reach instead of the corpus's contents. `/MCAF` is counted beside it: §14.13.5 puts
//! a graphics object's array in a marked-content property list, and Errata Collection 3's Issue
//! #374 names that key.
//!
//! A specification with no `/EF` is the *external* form. It is counted rather than judged: an
//! external associated file is a conforming construction, and what this program does with one is
//! §7.11.1's architectural refusal — which is about following the file, not about naming it.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]

use std::path::{Path, PathBuf};

use rayon::prelude::*;

use pdf_syntax::{Document, Name, Object, ObjectId};

/// How deep a nested object is followed before the walk gives up.
const MAX_DEPTH: usize = 32;

/// What one document contributes.
#[derive(Default, Clone)]
struct Counts {
    /// `/AF` or `/MCAF` arrays stated anywhere in the object graph.
    arrays: usize,
    /// File specifications those arrays name.
    specifications: usize,
    /// Of those, the ones carrying an `/EF` — §14.13.2's recommended form.
    embedded: usize,
    /// Of those, the ones carrying none — the form this program used to drop in silence.
    external: usize,
    /// External ones that state a `/AFRelationship`, which is what a reader could still say.
    external_with_relationship: usize,
}

impl Counts {
    /// Whether this document is a witness at all.
    fn states_any(&self) -> bool {
        self.arrays > 0
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scope: &[&str] = if args.iter().any(|a| a == "--crawl") {
        &["corpus-cache/safedocs/cc-main-2021-31"]
    } else if args.iter().any(|a| a == "--pdfjs") {
        &["doc/pdf.js/test/pdfs"]
    } else {
        &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"]
    };
    let mut files = Vec::new();
    for relative in scope {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    eprintln!("{} PDF(s) in the population", files.len());

    let measured: Vec<(String, Counts)> = files
        .par_iter()
        .map(|path| {
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (label, measure(path))
        })
        .collect();

    let witnesses: Vec<&(String, Counts)> = measured
        .iter()
        .filter(|(_, counts)| counts.states_any())
        .collect();
    let total = |pick: fn(&Counts) -> usize| -> usize {
        measured.iter().map(|(_, counts)| pick(counts)).sum()
    };
    println!(
        "{} PDF(s) read; {} state an /AF or /MCAF array",
        measured.len(),
        witnesses.len()
    );
    println!(
        "  {} array(s) naming {} file specification(s): {} embedded, {} external",
        total(|c| c.arrays),
        total(|c| c.specifications),
        total(|c| c.embedded),
        total(|c| c.external),
    );
    println!(
        "  {} of the external one(s) state an /AFRelationship",
        total(|c| c.external_with_relationship)
    );
    for (label, counts) in witnesses {
        println!(
            "  {label}: {} specification(s), {} embedded, {} external",
            counts.specifications, counts.embedded, counts.external
        );
    }
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
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// Reads one document's whole object graph.
fn measure(path: &Path) -> Counts {
    let mut counts = Counts::default();
    let Ok(bytes) = std::fs::read(path) else {
        return counts;
    };
    let Ok(document) = Document::open(bytes) else {
        return counts;
    };
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        walk(&document, &object, 0, &mut counts);
    }
    counts
}

/// Finds every `/AF` and `/MCAF` array under one object and counts what it names.
fn walk(document: &Document, object: &Object, depth: usize, counts: &mut Counts) {
    if depth > MAX_DEPTH {
        return;
    }
    match object {
        Object::Array(items) => {
            for item in items {
                walk(document, item, depth.saturating_add(1), counts);
            }
        }
        Object::Dictionary(dict) => entries(document, dict.iter(), depth, counts),
        Object::Stream(stream) => entries(document, stream.dict.iter(), depth, counts),
        Object::Null
        | Object::Boolean(_)
        | Object::Integer(_)
        | Object::Real(_)
        | Object::String(_)
        | Object::Name(_)
        | Object::Reference(_) => {}
    }
}

/// One dictionary's entries: the two keys that carry an array, and then everything below them.
fn entries<'a>(
    document: &Document,
    pairs: impl Iterator<Item = (&'a Name, &'a Object)>,
    depth: usize,
    counts: &mut Counts,
) {
    for (key, value) in pairs {
        if key.as_bytes() == b"AF" || key.as_bytes() == b"MCAF" {
            tally(document, value, counts);
        }
        walk(document, value, depth.saturating_add(1), counts);
    }
}

/// Counts one `/AF` array's file specifications by the form each takes.
fn tally(document: &Document, array: &Object, counts: &mut Counts) {
    let resolved = document.resolve(array);
    let Some(items) = resolved.as_array() else {
        return;
    };
    counts.arrays = counts.arrays.saturating_add(1);
    for item in items {
        let specification = document.resolve(item);
        let Some(specification) = specification.as_dict() else {
            continue;
        };
        counts.specifications = counts.specifications.saturating_add(1);
        if document
            .get_key(specification, "EF")
            .as_dict()
            .is_some_and(|files| {
                ["UF", "F", "DOS", "Mac", "Unix"]
                    .into_iter()
                    .any(|key| document.get_key(files, key).as_stream().is_some())
            })
        {
            counts.embedded = counts.embedded.saturating_add(1);
        } else {
            counts.external = counts.external.saturating_add(1);
            if document
                .get_key(specification, "AFRelationship")
                .as_name()
                .is_some()
            {
                counts.external_with_relationship =
                    counts.external_with_relationship.saturating_add(1);
            }
        }
    }
}
