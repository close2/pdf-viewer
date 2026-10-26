//! Who ISO 32000-2 §7.5.5's `/Size` sentence, Table 15's last, reaches — asked of the documents.
//!
//! > Any object in a cross-reference section whose number is greater than this value shall be
//! > ignored and defined to be missing by a PDF reader.
//!
//! For every document whose cross-reference table was read from the file rather than rebuilt by
//! a scan, this counts the in-use entries numbered above the trailer's `/Size` and walks every
//! reference reachable from the trailer, so that a document is classed by whether the sentence
//! would take away an object its own graph reaches, and whether `/Root` is among them. The last
//! column, page one present, is the reader's answer, so the same census run on a reader that
//! obeys the sentence prints what obeying costs — which is how ADR 2346 calibrated it.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example size_census -- <file-or-directory>…
//! cargo run --profile gates -p pdf-model --example size_census -- --page-one <file>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pdf_model::Pages;
use pdf_syntax::{Document, Object};

/// How many objects one document's walk visits before it stops counting.
const WALK_BOUND: usize = 200_000;

/// What one document says.
struct Finding {
    /// The trailer's `/Size`, where it states an integer.
    size: Option<i64>,
    /// Table entries numbered above `/Size`.
    beyond: usize,
    /// Of those, the ones the trailer's own graph reaches.
    reached: usize,
    /// Of the entries above `/Size`, the ones the table places inside an object stream.
    compressed: usize,
    /// Whether `/Root` itself is numbered above `/Size`.
    root_beyond: bool,
    /// Whether the reader shows a page one.
    page_one: bool,
    /// The table came from a scan rather than from the file.
    rebuilt: bool,
}

fn census(path: &Path) -> Option<Finding> {
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let xref = document.xref();
    let size = xref.trailer().get("Size").and_then(Object::as_integer);
    let beyond: BTreeSet<u32> = match size {
        Some(size) => xref
            .object_numbers()
            .filter(|&number| i64::from(number) > size)
            .collect(),
        None => BTreeSet::new(),
    };
    let root_beyond = xref
        .trailer()
        .get("Root")
        .and_then(Object::as_reference)
        .is_some_and(|id| beyond.contains(&id.number));
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut stack: Vec<Object> = xref
        .trailer()
        .iter()
        .filter(|(key, _)| key.as_bytes() != b"Prev")
        .map(|(_, value)| value.clone())
        .collect();
    while let Some(object) = stack.pop() {
        if seen.len() >= WALK_BOUND {
            break;
        }
        match &object {
            Object::Reference(id) => {
                if seen.insert(id.number) {
                    stack.push(document.get(*id));
                }
            }
            Object::Array(items) => stack.extend(items.iter().cloned()),
            other => {
                if let Some(dict) = other.as_dict() {
                    stack.extend(dict.iter().map(|(_, value)| value.clone()));
                }
            }
        }
    }
    let reached = beyond.intersection(&seen).count();
    let compressed = beyond
        .iter()
        .filter(|&&number| {
            matches!(
                xref.location(number),
                Some(pdf_syntax::Location::InStream { .. })
            )
        })
        .count();
    let page_one = Pages::new(&document).get(0).is_some();
    Some(Finding {
        size,
        beyond: beyond.len(),
        reached,
        compressed,
        root_beyond,
        page_one,
        rebuilt: xref.recovered_by_scan(),
    })
}

fn files(arguments: impl Iterator<Item = String>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for argument in arguments {
        let path = PathBuf::from(argument);
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                found.extend(
                    entries
                        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf")),
                );
            }
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn main() {
    let mut arguments = std::env::args().skip(1).peekable();
    // `--page-one` opens each document and asks for its first page and does nothing else, so
    // that an instruction count under callgrind is the cost of reaching page one on that reader.
    if arguments
        .peek()
        .is_some_and(|argument| argument == "--page-one")
    {
        let mut reached = 0_usize;
        for path in files(arguments.skip(1)) {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            if let Ok(document) = Document::open(bytes)
                && Pages::new(&document).get(0).is_some()
            {
                reached = reached.saturating_add(1);
            }
        }
        println!("page one reached in {reached} document(s)");
        return;
    }
    let (mut read, mut opened, mut rebuilt, mut stating, mut reaching, mut root, mut pageless) = (
        0_usize, 0_usize, 0_usize, 0_usize, 0_usize, 0_usize, 0_usize,
    );
    for path in files(arguments) {
        read = read.saturating_add(1);
        let Some(finding) = census(&path) else {
            continue;
        };
        opened = opened.saturating_add(1);
        if finding.rebuilt {
            rebuilt = rebuilt.saturating_add(1);
        }
        if !finding.page_one {
            pageless = pageless.saturating_add(1);
        }
        // A table a scan rebuilt has no cross-reference section, so the sentence has no subject in
        // it, whatever its recovered trailer's `/Size` says.
        if finding.beyond == 0 || finding.rebuilt {
            continue;
        }
        stating = stating.saturating_add(1);
        if finding.reached > 0 {
            reaching = reaching.saturating_add(1);
        }
        if finding.root_beyond {
            root = root.saturating_add(1);
        }
        println!(
            "{}\tsize={}\tbeyond={}\treached={}\tcompressed={}\troot_beyond={}\tpage_one={}",
            path.display(),
            finding
                .size
                .map_or_else(|| "-".to_owned(), |size| size.to_string()),
            finding.beyond,
            finding.reached,
            finding.compressed,
            finding.root_beyond,
            finding.page_one
        );
    }
    println!(
        "size census: {read} read, {opened} opened, {rebuilt} on a rebuilt table, {pageless} with \
         no page one; {stating} read from the file state an entry above /Size, {reaching} reach one from the \
         trailer, {root} name a /Root above it"
    );
}
