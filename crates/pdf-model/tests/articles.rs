//! ISO 32000-2 §12.4.3's articles, measured against the corpus.
//!
//! **The population depends on which corpus is meant, and this file used to name only one of
//! them.** Of the pdf.js corpus's 974 documents, none states an article: two catalogs carry a
//! `/Threads` entry and neither carries a thread — one is an empty array, the other a reference
//! that resolves to null — and not one page carries the `/B` array Table 31 recommends beside
//! beads. That much is unchanged and is the first test below.
//!
//! What was wrong is the sentence this file drew from it, which read "no document in the
//! 974-document corpus states an article" in the module comment and **"no corpus document states
//! an article"** in four other places in the tree. The four `doc/corpora/` submodules are part of
//! the corpus this project measures over — every other absence claim in this tree is stated over
//! them — and they hold **four documents with real threads and 115 beads between them**, two of
//! them named for the fact (`PDFBOX-3110-poems-beads.pdf`). So §12.4.3 is no longer trap 8's
//! purest case: it has producers' files behind it, and
//! [`a_producers_own_thread_is_walked_to_its_beads`] is what reads one.
//!
//! Found in the five-hundred-and-seventieth session by the census ADR 0403 asked for; ADR 0405
//! has why an absence claim decays without anybody touching it.
//!
//! Both tests are ratchets on the *corpus* rather than on the code: if a pdf.js document with a
//! real thread ever arrives, the first says so.

#![expect(
    clippy::panic,
    reason = "test code: an optional corpus that is present but does not open is a broken \
              checkout, and must fail loudly rather than skip"
)]

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

/// A `doc/corpora/pdfbox` document, or `None` when that optional submodule is not checked out.
///
/// Absent is a skip and present-but-unopenable is a panic, which is `composite_fonts.rs`'s rule
/// and `doc/habits.md`'s: a missing corpus is a skip, a corpus that lacks what a test needs is a
/// failure.
fn pdfbox_document(name: &str) -> Option<Document> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/corpora/pdfbox/pdfbox/src/test/resources/input")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    Some(Document::open(bytes).unwrap_or_else(|e| panic!("{name} does not open: {e}")))
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
fn no_pdfjs_document_states_an_article() {
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

/// The first real thread this reader has ever been shown, from a producer rather than a fixture.
///
/// `PDFBOX-3110-poems-beads.pdf` is two poems laid out as two article threads, which is exactly
/// what §12.4.3 describes an article for — "a single logical flow of content", read in bead
/// order rather than in page order. Until the five-hundred-and-seventieth session every
/// assertion this tree made about §12.4.3 was against a file it had written itself, on the
/// strength of a claim that the corpus held none.
///
/// What it checks is the part a hand-built fixture cannot: that the ring closes. §12.4.3 makes
/// `/N` and `/V` a doubly linked *circular* list — "the first bead's `/V` shall point to the
/// last bead" — so a reader that walks `/N` without a visited set either loops for ever or stops
/// early, and only a file somebody else wrote decides which of those this one does.
#[test]
fn a_producers_own_thread_is_walked_to_its_beads() {
    let Some(document) = pdfbox_document("PDFBOX-3110-poems-beads.pdf") else {
        println!("skipped: doc/corpora/pdfbox is not checked out");
        return;
    };
    let articles = Articles::read(&document);
    let shapes: Vec<String> = articles
        .threads
        .iter()
        .map(|thread| {
            format!(
                "{:?} with {} bead(s), {} naming a page",
                thread.title,
                thread.beads.len(),
                thread.beads.iter().filter(|b| b.page.is_some()).count()
            )
        })
        .collect();
    println!("PDFBOX-3110-poems-beads.pdf: {shapes:?}");

    assert_eq!(articles.threads.len(), 2, "the two poems: {shapes:?}");
    // Table 162's `/I` is a document information dictionary, so its `/Title` is a §7.9.2.2 text
    // string — and the first poem's is `Erlkönig`, which is the one assertion here that a
    // Latin-1 reading would pass and a byte-for-byte one would not.
    assert_eq!(
        articles
            .threads
            .iter()
            .map(|thread| thread.title.clone())
            .collect::<Vec<_>>(),
        vec![Some("Erlkönig".to_owned()), Some("Moulière".to_owned())],
        "§12.6.4.7 names a thread by this title: {shapes:?}"
    );
    let beads: usize = articles.threads.iter().map(|t| t.beads.len()).sum();
    assert_eq!(beads, 11, "the beads the two threads hold: {shapes:?}");
    assert!(
        articles
            .threads
            .iter()
            .all(|thread| thread.beads.iter().all(|bead| bead.page.is_some())),
        "every bead of a real thread names the page it sits on: {shapes:?}"
    );

    // The ring: following `/N` from the first bead of a thread returns to it after exactly as
    // many steps as the thread has beads, which is what "circular" means and what a `/V` that
    // pointed somewhere else would break.
    for thread in &articles.threads {
        let first = thread.beads.first().expect("a thread with beads").id;
        let mut at = first;
        for _ in 0..thread.beads.len() {
            at = articles.next(at).expect("every bead has a successor").id;
        }
        assert_eq!(at, first, "the thread's ring closes on its first bead");
    }
}
