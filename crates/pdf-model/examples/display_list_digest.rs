//! A digest of every corpus document's first page, for proving a change drew nothing differently.
//!
//! `CLAUDE.md`'s rule 1 makes interpretation a pure function of the document and the view state,
//! and the oracle's 1794-page comparison rests on it. So a round that touches `content::interpret`
//! owes a demonstration rather than an argument that every existing caller still produces the
//! display list it produced before — and the gates' *summary* numbers are the wrong instrument for
//! that, because two different lists can rasterise to the same verdict.
//!
//! This prints the artefact itself, reduced: one line per document giving the command count, the
//! byte length of the list's `Debug` rendering and a hash of it. Run it on two revisions and
//! `diff` the two files; an empty diff is the claim.
//!
//! ```sh
//! cargo run --release -p pdf-model --example display_list_digest -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! **Run both arms with the same `pdf-sandbox-worker` on disk, or 106 documents move on their
//! own.** A JBIG2 or JPEG 2000 image is decoded in a confined worker, and a viewer that cannot
//! find one *refuses* the image rather than falling back in process — which is the security
//! posture working, and which means the page's display list holds no command for it. So this
//! artefact is a digest of what the program could decode as well as of what it interpreted, and
//! it changes the moment `cargo build -p pdf-sandbox --bins` has run. Session 535 compared two
//! pairs an hour apart and read the worker's arrival as a difference in the code.
//!
//! **The hash is [`std::collections::hash_map::DefaultHasher`]**, which the standard library
//! documents as unspecified across releases. That is exactly good enough here and no more: both
//! sides of the comparison are run in one sitting with one compiler, and what is being detected is
//! a *difference*. The command count and the length beside it are what make a lone hash collision
//! not a false pass.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pdf_model::{Pages, interpret};
use pdf_syntax::Document;

fn main() {
    let mut documents = 0_usize;
    let mut pages_read = 0_usize;
    for path in std::env::args().skip(1) {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            println!("{name}\tunreadable");
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            println!("{name}\tunopened");
            continue;
        };
        documents = documents.saturating_add(1);
        let Some(page) = Pages::new(&document).get(0) else {
            println!("{name}\tno page");
            continue;
        };
        let interpretation = interpret(&document, &page);
        let rendered = format!("{:?}", interpretation.display_list);
        let mut hasher = DefaultHasher::new();
        rendered.hash(&mut hasher);
        pages_read = pages_read.saturating_add(1);
        println!(
            "{name}\t{}\t{}\t{:016x}",
            interpretation.display_list.commands().len(),
            rendered.len(),
            hasher.finish()
        );
    }
    println!("# {documents} document(s) opened, {pages_read} first page(s) interpreted");
}
