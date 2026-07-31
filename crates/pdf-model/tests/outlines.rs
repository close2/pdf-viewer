//! ISO 32000-2 §12.3.3's document outline, over the corpus.
//!
//! The clause states `/Count` as an **algorithm** rather than as a number — three numbered
//! steps over the visible descendants — which means a document states the same fact twice and
//! can be checked against itself. That is the habit LZW stream lengths and a byte-swapped
//! `indexToLocFormat` taught this project, applied one level up: **what does this file already
//! say about itself, and does it agree?**

use std::path::{Path, PathBuf};

use pdf_model::outline::Outline;
use pdf_model::page::Pages;
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

/// Every document with an outline produces one, and its items reach pages.
///
/// Two counts and one assertion. A document that states `/Outlines` and yields no item has
/// either a linked list this reader cannot walk or a root with no `/First`, and those are worth
/// telling apart — so the failures are named rather than summed.
#[test]
fn a_document_with_an_outline_produces_its_items() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut stating = 0usize;
    let mut with_items = 0usize;
    let mut items = 0usize;
    let mut with_a_destination = 0usize;
    let mut empty = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let Ok(catalog) = document.catalog() else {
            continue;
        };
        if document.get_key(&catalog, "Outlines").as_dict().is_none() {
            continue;
        }
        stating = stating.saturating_add(1);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let pages = Pages::new(&document);
        let outline = Outline::read(&document, &pages);
        if outline.is_empty() {
            // Two-sided: an empty outline must be the *document's* emptiness. Table 150 makes
            // `/First` "[r]equired if there are any open or closed outline entries", so a root
            // without one has no items to find and a root with one that yields nothing would be
            // this reader failing to walk a list.
            assert!(
                document
                    .get_key(&catalog, "Outlines")
                    .as_dict()
                    .is_some_and(|root| root.get("First").is_none()),
                "{name}: an outline root with a /First produced no items"
            );
            empty.push(name.into_owned());
            continue;
        }
        with_items = with_items.saturating_add(1);
        count(
            &outline.items,
            &document,
            &pages,
            &mut items,
            &mut with_a_destination,
        );
    }

    println!("{stating} of {} documents state an /Outlines", files.len());
    println!("  {with_items} yield items: {items} in all, {with_a_destination} reaching a page");
    println!("  {} yield none: {empty:?}", empty.len());

    assert_eq!(stating, 176, "documents with an outline dictionary");
    // Every one of the 26 is `<< /Type /Outlines /Count 0 >>` — an outline dictionary with no
    // `/First`, which Table 150 permits and which is a document whose outline is empty rather
    // than one this reader could not walk. The loop above asserts that per document; this is
    // the count, so a *new* one shows up here.
    assert_eq!(empty.len(), 26);
    assert_eq!(with_items, 150);
    assert_eq!(items, 343, "outline items in the corpus");
    assert_eq!(
        with_a_destination, 305,
        "items whose destination names a page"
    );
}

/// Counts items and the ones whose destination names a page.
fn count(
    items: &[pdf_model::outline::Item],
    document: &Document,
    pages: &Pages<'_>,
    total: &mut usize,
    reaching: &mut usize,
) {
    for item in items {
        *total = total.saturating_add(1);
        if item
            .destination
            .and_then(|destination| destination.page_index(document, pages))
            .is_some()
        {
            *reaching = reaching.saturating_add(1);
        }
        count(&item.children, document, pages, total, reaching);
    }
}

/// A document's own `/Count` agrees with the count the clause defines.
///
/// §12.3.3 gives three numbered steps for "the number of visible descendent outline items", and
/// the outline dictionary states the result. Running the steps over what was read and comparing
/// is a check on **this reader** — a walk that lost a level, took a closed item's children as
/// visible, or followed `/Next` past the end would disagree with the producer that ran the same
/// algorithm.
///
/// The disagreements are listed rather than tolerated as a ratio, because each is one of two
/// things and only reading it says which: a file whose count is stale, or a walk that is wrong.
#[test]
fn a_stated_count_agrees_with_the_count_the_clause_defines() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut compared = 0usize;
    let mut agreeing = 0usize;
    let mut differing = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let pages = Pages::new(&document);
        let outline = Outline::read(&document, &pages);
        let Some(stated) = outline.stated_count else {
            continue;
        };
        if outline.is_empty() {
            continue;
        }
        compared = compared.saturating_add(1);
        let ours = i64::try_from(outline.visible_count()).unwrap_or(i64::MAX);
        if ours == stated {
            agreeing = agreeing.saturating_add(1);
        } else {
            differing.push(format!(
                "{}: states {stated}, the clause's steps give {ours}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }

    println!("{compared} documents state a /Count over a non-empty outline, {agreeing} agreeing");
    for line in &differing {
        println!("  {line}");
    }
    // **144 of 146 producers ran the same three steps.** The two that did not are both
    // hand-written pdf.js fixtures, and each contradicts *itself* rather than us:
    // `nested_outline.pdf` gives every one of its three top-level items a positive `/Count 2`,
    // which by step 3 makes them open and their six children visible — 9 — while its root says
    // 3; `outline_goto_action.pdf` does the same with one parent and one child and says 1. A
    // number counted from the items is the clause's; a number written beside them is a claim.
    assert_eq!(differing.len(), 2);
    assert_eq!(compared, 146);
    assert_eq!(agreeing, 144);
}
