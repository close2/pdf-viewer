//! ISO 32000-2 §12.4.3's articles, measured against the corpus.
//!
//! The measurement is the finding: **no document in the 974-document corpus states an
//! article**. Two catalogs carry a `/Threads` entry and neither carries a thread — one is an
//! empty array, the other a reference that resolves to null — and not one page carries the `/B`
//! array Table 31 recommends beside beads. So §12.4.3 is trap 8 in its purest form: a clause
//! whose reader can only be built from the clause, checked against the clause's own EXAMPLE 2
//! in `article.rs`'s unit tests.
//!
//! This file is therefore a *ratchet on the corpus rather than on the code*. If a document with
//! a real thread ever arrives, these numbers change and this test says so — which is the only
//! way a reader written with no witnesses acquires one.

use std::path::{Path, PathBuf};

use pdf_model::article::Articles;
use pdf_syntax::{Document, Object, ObjectId};

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// What the corpus contains of §12.4.3, stated as the two counts that could change.
///
/// Both halves are checked, because they fail differently. A catalog stating `/Threads` and
/// yielding no thread is either a document with nothing to say or a reader that cannot walk a
/// ring, and the two are told apart by looking at what the entry *is* — which is why the two
/// documents are named here with their shapes. A page carrying `/B` while its catalog states no
/// thread would be an article reachable only from pages, which this reader would not see at
/// all; there is none, and that is worth a number rather than an assumption.
#[test]
fn the_corpus_states_no_article() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut with_the_entry = Vec::new();
    let mut with_a_thread = Vec::new();
    let mut pages_with_beads = 0usize;
    let mut beads_reaching_a_page = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if let Ok(catalog) = document.catalog()
            && catalog.get("Threads").is_some()
        {
            with_the_entry.push(format!(
                "{name}: {:?}",
                document.get_key(&catalog, "Threads")
            ));
        }
        let articles = Articles::read(&document);
        if !articles.is_empty() {
            with_a_thread.push(name.into_owned());
            beads_reaching_a_page = beads_reaching_a_page.saturating_add(
                articles
                    .threads
                    .iter()
                    .flat_map(|thread| thread.beads.iter())
                    .filter(|bead| bead.page.is_some())
                    .count(),
            );
        }

        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(dict) = object.as_dict() else {
                continue;
            };
            let is_page = dict
                .get("Type")
                .and_then(Object::as_name)
                .is_some_and(|kind| kind.as_bytes() == b"Page");
            if is_page && dict.get("B").is_some() {
                pages_with_beads = pages_with_beads.saturating_add(1);
            }
        }
    }

    println!(
        "{} documents state /Threads: {with_the_entry:?}",
        with_the_entry.len()
    );
    println!("{} yield a thread: {with_a_thread:?}", with_a_thread.len());
    println!("{pages_with_beads} pages carry a /B, {beads_reaching_a_page} beads name a page");

    assert_eq!(
        with_the_entry.len(),
        2,
        "catalogs carrying a /Threads entry: {with_the_entry:?}"
    );
    assert!(
        with_a_thread.is_empty(),
        "the entry is an empty array in one document and a null in the other: {with_the_entry:?}"
    );
    assert_eq!(
        pages_with_beads, 0,
        "no page states beads its catalog does not thread"
    );
}
