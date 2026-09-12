//! Which of ISO 8601's forms the corpora's XMP packets actually write in their `Date`
//! properties, and which values `metadata/properties-use-known-schemas` refuses, with why.
//!
//! ```sh
//! cargo run --release -p pdf-archive --example dates                       # the two tracked corpora
//! cargo run --release -p pdf-archive --example dates -- doc/corpora/pdfbox  # or any directories
//! ```
//!
//! Every metadata stream of every document is read, and every scalar of every property the XMP
//! Specification types as a `Date` — `dc:date`, `xmp:CreateDate` and the rest of
//! [`pdf_archive::dates_stated`]'s population — is classified by [`pdf_archive::iso_8601`]: the
//! form it is, printed in the working draft's own notation, or the refusal it earns. The point of
//! printing a population rather than a verdict is `CLAUDE.md`'s two denominators: the grammar is
//! implemented from the standard, and what a corpus can add is which of the standard's forms
//! producers write, and whether any value the check refuses was written by a producer that meant
//! a date by it. Session 993 ran it before and after widening the check; ADR 1013 has both
//! tables and names every value that moved.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a census a person reads"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_archive::{Examination, Level, Target, dates_stated, iso_8601};
use pdf_syntax::{Document, FileBytes, Object};

/// Every `.pdf` under a directory, symbolic links followed, sorted so that two runs print alike.
fn documents(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    paths.sort();
    for path in paths {
        // `metadata` follows a link where `file_type` would report the link itself, and the
        // corpora are links from a worktree to the main checkout.
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            documents(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "pdf") {
            out.push(path);
        }
    }
}

/// The two corpora every round walks, relative to the workspace root.
fn tracked() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    ["doc/pdf.js/test/pdfs", "doc/veraPDF-corpus"]
        .into_iter()
        .map(|corpus| root.join(corpus))
        .collect()
}

fn main() {
    let roots: Vec<PathBuf> = {
        let named: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
        if named.is_empty() { tracked() } else { named }
    };
    let mut files = Vec::new();
    for root in &roots {
        documents(root, &mut files);
    }

    let (mut opened, mut packets, mut values, mut empty) = (0usize, 0usize, 0usize, 0usize);
    let mut forms: BTreeMap<String, usize> = BTreeMap::new();
    let mut refused: Vec<String> = Vec::new();
    for path in &files {
        let Ok(bytes) = FileBytes::on_disk(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        // The target is immaterial: the object walk is the same for every one.
        let exam = Examination::new(&document, Target::Two(Level::B));
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        for (_, object) in exam.objects() {
            let Object::Stream(stream) = object else {
                continue;
            };
            if document
                .get_key(&stream.dict, "Type")
                .as_name()
                .is_none_or(|kind| kind.as_bytes() != b"Metadata")
            {
                continue;
            }
            let Some(packet) = document.decoded_stream_data(stream) else {
                continue;
            };
            packets = packets.saturating_add(1);
            for (property, text) in dates_stated(&packet) {
                values = values.saturating_add(1);
                let text = text.trim();
                if text.is_empty() {
                    // An empty element is present and says no value, which the row admits.
                    empty = empty.saturating_add(1);
                    continue;
                }
                match iso_8601::read(text) {
                    Ok(form) => {
                        let seen = forms.entry(form.to_string()).or_default();
                        *seen = seen.saturating_add(1);
                    }
                    Err(why) => refused.push(format!("{name}  {property}  {text:?}, {why}")),
                }
            }
        }
    }
    println!(
        "{opened} documents opened of {}, {packets} metadata streams, {values} date values \
         ({empty} empty), {} refused",
        files.len(),
        refused.len()
    );
    println!("\nadmitted, by form, in ISO/WD 8601-1's notation:");
    let mut by_count: Vec<_> = forms.iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (form, count) in by_count {
        println!("  {count:6}  {form}");
    }
    if !refused.is_empty() {
        println!("\nrefused, each with the form it missed:");
        for line in &refused {
            println!("  {line}");
        }
    }
}
