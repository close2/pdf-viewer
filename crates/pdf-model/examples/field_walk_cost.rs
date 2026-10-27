//! What reading every widget's field through §12.7.4.1's inheritance costs over a set of documents.
//!
//! ```sh
//! cargo run --release -p pdf-model --example field_walk_cost -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! A document is a *form document* where its catalog's `/AcroForm` states a non-empty `/Fields`
//! array; the rest are opened and passed over. For each form document every page's widgets are
//! described by `form::fields`, which reads each one's field through the inheritance, and the
//! figure is the wall time of those calls alone — not of opening the file — summed over the form
//! documents, with the number of fields described beside it so that two runs can be seen to have
//! read the same population. Each document is read once, on a fresh `Document`, which is how a
//! viewer meets it. ADR 2448 is the measurement's reason: the walk once per widget against each
//! field resolved once per document.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    reason = "a measuring binary whose output is its purpose; its sums count documents and \
              fields it read itself"
)]

use std::process::ExitCode;
use std::time::{Duration, Instant};

use pdf_model::form::fields;
use pdf_model::view::ViewState;
use pdf_syntax::Document;

fn main() -> ExitCode {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("field_walk_cost: give the documents to read");
        return ExitCode::FAILURE;
    }
    let (mut documents, mut described) = (0_usize, 0_usize);
    let mut spent = Duration::ZERO;
    for path in &paths {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(why) => {
                eprintln!("{path}: {why}");
                continue;
            }
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        if !has_fields(&document) {
            continue;
        }
        documents += 1;
        let view = ViewState::of(&document);
        let pages = pdf_model::Pages::new(&document);
        let started = Instant::now();
        for page in pages.every().into_iter().flatten() {
            described += fields(&document, &page, &view).len();
        }
        spent += started.elapsed();
    }
    println!(
        "form documents {documents}, fields described {described}, walk {:.3} s",
        spent.as_secs_f64()
    );
    ExitCode::SUCCESS
}

/// Whether the catalog's interactive form states a non-empty `/Fields` (§12.7.3, Table 224).
fn has_fields(document: &Document) -> bool {
    let Ok(root) = document.catalog() else {
        return false;
    };
    let form = document.get_key(&root, "AcroForm");
    form.as_dict().is_some_and(|form| {
        document
            .get_key(form, "Fields")
            .as_array()
            .is_some_and(|fields| !fields.is_empty())
    })
}
