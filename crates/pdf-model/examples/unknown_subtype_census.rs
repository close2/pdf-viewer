//! Annotations whose `/Subtype` is outside Table 171, and whether each states an appearance.
//!
//! Written for one question, and it is trap 11's: `annotation::decided` used to report every
//! annotation of a subtype it could not construct for, and ISO 32000-2 §12.5.3's Table 167 says
//! in its `Invisible` row that a reader owes such an annotation one thing only —
//!
//! > If clear, render such an unknown annotation using an appearance stream specified by its
//! > appearance dictionary, if any (see 12.5.5, "Appearance streams").
//!
//! — so the split that decides whether a report is owed at all is *with* an appearance dictionary
//! against *without* one. §12.5.5's own sentence covers the first half ("[i]f a PDF processor does
//! not have native support for a particular annotation type, the PDF processor shall render the
//! annotation with its normal (N) appearance"), and the second half is the population this counts.
//!
//! Table 171's twenty-eight names are listed here rather than imported because the list is
//! `pdf-model`'s private business; a census with its own copy is the honest way to ask the
//! question from outside, and a name this file and `annotation.rs` disagree about would show up as
//! a subtype counted here that the crate constructs for.
//!
//! An annotation stating **no** `/Subtype` is not counted: Table 166 makes the entry required, so
//! its absence is a broken file rather than a type nobody recognises, and this program keeps
//! reporting that one.
//!
//! Every page of every document is walked, not the first: an annotation is a thing a reader
//! scrolls to.
//!
//! An argument that is a **directory** is walked for every `.pdf` under it, which is what makes
//! the crawl one command and one number: `xargs` splits 65 944 paths into a dozen runs, and a
//! dozen partial answers is exactly the shape `doc/todo/01`'s counted-claim rule exists to stop.
//!
//! ```sh
//! cargo run --release -p pdf-model --example unknown_subtype_census -- doc/pdf.js/test/pdfs
//! cargo run --release -p pdf-model --example unknown_subtype_census -- \
//!   corpus-cache/safedocs/cc-main-2021-31
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Document, Object};
use rayon::prelude::*;

/// The annotation subtypes ISO 32000-2 Table 171 defines.
const STANDARD: [&[u8]; 28] = [
    b"3D",
    b"Caret",
    b"Circle",
    b"FileAttachment",
    b"FreeText",
    b"Highlight",
    b"Ink",
    b"Line",
    b"Link",
    b"Movie",
    b"Polygon",
    b"PolyLine",
    b"Popup",
    b"PrinterMark",
    b"Projection",
    b"Redact",
    b"RichMedia",
    b"Screen",
    b"Sound",
    b"Square",
    b"Squiggly",
    b"Stamp",
    b"StrikeOut",
    b"Text",
    b"TrapNet",
    b"Underline",
    b"Watermark",
    b"Widget",
];

/// One document's answer: the subtypes outside Table 171, each with its two counts.
type Tally = BTreeMap<String, (usize, usize)>;

/// Walks one document, or answers `None` where it does not open.
fn count(path: &str) -> Option<(Tally, String)> {
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let mut tally = Tally::new();
    let pages = pdf_model::Pages::new(&document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let annotations = document.get_key(&page.dict, "Annots");
        let Some(list) = annotations.as_array().map(<[Object]>::to_vec) else {
            continue;
        };
        for entry in &list {
            let resolved = document.resolve(entry);
            let Some(annotation) = resolved.as_dict() else {
                continue;
            };
            let subtype = document.get_key(annotation, "Subtype");
            let Some(name) = subtype.as_name() else {
                continue;
            };
            if STANDARD.contains(&name.as_bytes()) {
                continue;
            }
            // Table 170's `/N`, which is the entry §12.5.5 renders and the one the `Invisible`
            // row's "if any" is about. An `/AP` stating only `/R` or `/D` states no normal
            // appearance, so the dictionary's presence is not what is asked.
            let stated = document
                .get_key(annotation, "AP")
                .as_dict()
                .is_some_and(|appearance| !document.get_key(appearance, "N").is_null());
            let counts = tally
                .entry(String::from_utf8_lossy(name.as_bytes()).into_owned())
                .or_insert((0, 0));
            counts.0 = counts.0.saturating_add(1);
            if !stated {
                counts.1 = counts.1.saturating_add(1);
            }
        }
    }
    Some((tally, path.to_owned()))
}

/// Every `.pdf` at or under one argument, so that a directory names a population.
fn collect(path: &std::path::Path, into: &mut Vec<String>) {
    if path.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            collect(&entry.path(), into);
        }
    } else if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
    {
        into.push(path.to_string_lossy().into_owned());
    }
}

fn main() {
    let mut paths: Vec<String> = Vec::new();
    for argument in std::env::args().skip(1) {
        collect(std::path::Path::new(&argument), &mut paths);
    }
    paths.sort();
    paths.dedup();
    let answered: Vec<(Tally, String)> = paths.par_iter().filter_map(|path| count(path)).collect();

    let mut total = Tally::new();
    let mut witnesses: Vec<&str> = Vec::new();
    for (tally, path) in &answered {
        if !tally.is_empty() {
            witnesses.push(path.as_str());
        }
        for (name, counts) in tally {
            let into = total.entry(name.clone()).or_insert((0, 0));
            into.0 = into.0.saturating_add(counts.0);
            into.1 = into.1.saturating_add(counts.1);
        }
    }

    let annotations: usize = total.values().map(|counts| counts.0).sum();
    let bare: usize = total.values().map(|counts| counts.1).sum();
    println!(
        "{} path(s), {} opened; {} annotation(s) outside Table 171 in {} document(s), {} of them \
         stating no /AP /N",
        paths.len(),
        answered.len(),
        annotations,
        witnesses.len(),
        bare
    );
    for (name, (here, without)) in &total {
        println!("  /{name}: {here}, {without} with no /AP /N");
    }
    for witness in &witnesses {
        println!("  witness {witness}");
    }
}
