//! Which of a corpus's documents are XFA's dynamic half, and the template each one's page one
//! would be laid out from — the measurement `doc/scope-ratifications.md` asks for before the
//! dynamic half's exclusion is revisited (`doc/todo/78`).
//!
//! The dynamic half is named by two tests, and this counts both (ADR 2380):
//!
//! - ISO 32000-2 Table 29's `/NeedsRendering`, "whether the document shall be regenerated when the
//!   document is first opened", read by [`pdf_model::xfa::stated`];
//! - XFA 3.3 page 333's grammar, an `occur` whose bounds differ or a `subformSet`, read by
//!   [`pdf_model::xfa::Template::dynamic`] over the template [`pdf_model::xfa::Resource::read`]
//!   parses.
//!
//! Paths are read one per line from standard input, because a corpus's list is longer than an
//! argument vector holds. One tab-separated line is printed per document that states either of
//! Annex K's entries: the path, `/NeedsRendering`, whether `/XFA` is a resource in one of §K.2's
//! two forms, page 333's reason or `static`, and the reader's error where it stopped. With
//! `--dump <dir>`, every resource's packets are written there concatenated in the array's order —
//! §K.2's "individual packets that together make up the XFA resource" — as `<line>.xdp`, the line
//! being the document's position in the input, for `tools/xfa-census.py` to count the layout
//! grammar in.
//!
//! ```sh
//! find doc/pdf.js -name '*.pdf' | cargo run --release -p pdf-model --example xfa_dynamic_census \
//!     -- --dump ~/scratch/agent/xfa-dump > xfa.tsv
//! ```

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]

use std::io::BufRead as _;
use std::path::PathBuf;

use pdf_model::xfa::{Resource, stated};
use pdf_syntax::{Document, FileBytes, Object};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let mut dump: Option<PathBuf> = None;
    while let Some(argument) = arguments.next() {
        if argument == "--dump" {
            dump = arguments.next().map(PathBuf::from);
        } else {
            eprintln!("usage: xfa_dynamic_census [--dump <dir>] < paths");
            std::process::exit(2);
        }
    }
    if let Some(dir) = &dump
        && let Err(error) = std::fs::create_dir_all(dir)
    {
        eprintln!("{}: {error}", dir.display());
        std::process::exit(1);
    }

    let mut read = 0_usize;
    let mut unopened = 0_usize;
    let mut stating = 0_usize;
    for (line, path) in std::io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .enumerate()
    {
        // Read where the document's offsets point rather than whole: a census of tens of
        // thousands of files reads the trailer, the catalog and the form dictionary of each, and
        // the rest of the bytes stay on the disk.
        let Ok(bytes) = FileBytes::on_disk(std::path::Path::new(&path)) else {
            unopened = unopened.saturating_add(1);
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            unopened = unopened.saturating_add(1);
            continue;
        };
        read = read.saturating_add(1);
        let Some(entries) = stated(&document) else {
            continue;
        };
        stating = stating.saturating_add(1);
        let (verdict, error) = match Resource::read(&document) {
            None => ("no-resource", String::new()),
            Some(Ok(resource)) => (
                resource.template.dynamic().unwrap_or("static"),
                String::new(),
            ),
            Some(Err(error)) => ("unread", error.to_string()),
        };
        if let Some(dir) = &dump
            && entries.resource
        {
            let text = resource_text(&document);
            if let Err(error) = std::fs::write(dir.join(format!("{line}.xdp")), text) {
                eprintln!("{path}: the dump would not write: {error}");
            }
        }
        println!(
            "{line}\t{path}\t{}\t{}\t{verdict}\t{error}",
            entries.needs_rendering, entries.resource
        );
    }
    eprintln!(
        "{read} document(s) read, {unopened} would not open, {stating} state /XFA or \
         /NeedsRendering"
    );
}

/// The resource's bytes as §K.2 assembles them: the one stream, or the array's packet streams in
/// the array's order. A stream that will not decode contributes nothing, and the dump is then
/// short — the census line's `unread` already says why.
fn resource_text(document: &Document) -> Vec<u8> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return Vec::new();
    };
    let streams = match document.get_key(form, "XFA") {
        Object::Stream(stream) => vec![stream],
        Object::Array(packets) => packets
            .chunks_exact(2)
            .filter_map(|pair| match pair {
                [_, Object::Reference(id)] => document.get(*id).as_stream().cloned(),
                [_, Object::Stream(stream)] => Some(stream.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let mut text = Vec::new();
    for stream in &streams {
        if let Some(bytes) = document.decoded_stream_data(stream) {
            text.extend_from_slice(&bytes);
        }
    }
    text
}
