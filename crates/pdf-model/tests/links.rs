//! ISO 32000-2 §12.5.6.5's link annotations, over the corpus.
//!
//! What a link *is* is asserted beside the code; this asks the other question — how many there
//! are, how many lead somewhere this reader can follow, and what the rest are. A link that goes
//! nowhere is usually not a defect: a URI action needs a network this program does not have,
//! and §12.6.4.5's launch action is absent for the reason `CLAUDE.md` principle 3 gives.

use std::path::{Path, PathBuf};

use pdf_model::Pages;
use pdf_model::link::{links, target};
use pdf_syntax::Document;

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

/// Page one's links are found, and the ones that lead somewhere resolve to a page.
///
/// The two-sided fact again: a link whose destination *resolves* must name a page inside the
/// document, and a link with no destination must have no go-to action. The second half is what
/// keeps "few links resolve" from hiding a reader that cannot read one.
#[test]
fn a_links_region_is_found_and_its_destination_resolves() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut documents = 0usize;
    let mut found = 0usize;
    let mut leading = 0usize;
    let mut quads = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let pages = Pages::new(&document);
        let Some(page) = pages.get(0) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let here = links(&document, &page);
        if here.is_empty() {
            continue;
        }
        documents = documents.saturating_add(1);
        found = found.saturating_add(here.len());
        for link in &here {
            assert!(
                !link.region.is_empty(),
                "{name}: a link with no activation region at all"
            );
            quads = quads.saturating_add(link.region.len());
            if let Some(index) = target(&document, &pages, link) {
                leading = leading.saturating_add(1);
                assert!(
                    index < pages.len(),
                    "{name}: a link to page {index} of {}",
                    pages.len()
                );
            }
        }
    }

    println!(
        "{documents} documents have links on page one: {found} of them, {quads} quadrilaterals"
    );
    println!("  {leading} lead to a page of their own document");
    assert_eq!(documents, 54, "documents with a link on page one");
    // 32 768 of these are `bug1978317.pdf`'s, which is a stress test for exactly that: one page
    // with thirty-two thousand link annotations on it. The other 53 documents share 357, which
    // is the number worth reading.
    assert_eq!(found, 33_125);
    assert_eq!(
        quads, 33_127,
        "two links state /QuadPoints of more than one quad"
    );
    // Most links in this corpus are URIs — a web page printed to PDF — and a URI needs a
    // network this program does not have. The ones that lead somewhere lead *inside* the
    // document, which is the only kind this viewer can follow.
    assert_eq!(leading, 36);
}
