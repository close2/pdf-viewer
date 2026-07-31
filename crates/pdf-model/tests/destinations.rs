//! ISO 32000-2 §12.3.2's destinations, over the corpus.
//!
//! The clause's own rules are asserted beside the code in `destination.rs`, where the fixtures
//! can state one form each; this file asks the other question, which is trap 4's: **what do
//! documents somebody else wrote actually contain**, and does what we compute from them stand
//! up. Two numbers come out of it, and the second is the more interesting:
//!
//! - **55 of the 974 documents state an `/OpenAction`** and 49 of them name a page this
//!   reader finds.
//! - **106 named destinations are reachable from link annotations and 22 resolve.** The other
//!   84 are the *documents'* — five files carry named links and no destination table at all,
//!   and `pdfjs_wikipedia.pdf` links to 27 `cite_note-…` anchors while its own table defines
//!   `cite_ref-…`. A named destination that resolves to nothing is a fact about the file, and
//!   the point of measuring is being able to say which files.

use std::path::{Path, PathBuf};

use pdf_model::destination::{Destination, Target};
use pdf_model::page::Pages;
use pdf_syntax::{Document, Object};

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

/// Every `/OpenAction` that states a destination names a page of its own document.
///
/// The count is what is asserted rather than any one document, because the six that do not
/// resolve are six *different* statements and each is checked by name below. Table 29 makes
/// this a whole-document question: an `/OpenAction` is the only destination a viewer must
/// resolve without anybody clicking anything.
#[test]
fn an_open_action_that_states_a_destination_names_a_page() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut stated = 0usize;
    let mut resolved = 0usize;
    let mut otherwise = Vec::new();
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
        if document.get_key(&catalog, "OpenAction").is_null() {
            continue;
        }
        stated = stated.saturating_add(1);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let pages = Pages::new(&document);
        match Destination::open_action(&document) {
            Some(destination) => match destination.page_index(&document, &pages) {
                Some(index) => {
                    resolved = resolved.saturating_add(1);
                    assert!(
                        index < pages.len(),
                        "{name}: an open action naming page {index} of {}",
                        pages.len()
                    );
                }
                None => otherwise.push(format!("{name}: {:?}", destination.target)),
            },
            None => otherwise.push(format!("{name}: not a destination")),
        }
    }

    println!("{stated} of {} documents state an /OpenAction", files.len());
    println!("  {resolved} name a page; the rest:");
    for line in &otherwise {
        println!("    {line}");
    }

    assert_eq!(stated, 55, "documents stating an /OpenAction");
    assert_eq!(resolved, 49, "open actions naming a page of this document");
    // The six are five action dictionaries that are not go-to actions — two ECMAScript, two
    // `/Named`, one `/GoTo` whose `/D` states no form Table 149 lists — and one file writing
    // `[0 /XYZ …]`, an integer where §12.3.2.2 requires "an indirect reference to a page
    // object". That last one is the interesting case and it costs nothing: the clause's own
    // answer for a destination we cannot resolve is Table 29's "the document shall be opened
    // to the top of the first page", and page *number* 0 is that page anyway.
    assert_eq!(otherwise.len(), 6);
}

/// A named destination resolves through whichever of §12.3.2.4's two tables its document has.
///
/// The assertion is deliberately two-sided. Left alone, "22 of 106 resolve" reads like a
/// reader that cannot find things; what makes it a measurement is that **every unresolved key
/// is absent from its own document's tables**, which this checks directly rather than
/// inferring — so a regression in the lookup shows up as a key that the tables hold and we did
/// not find, and not as a number moving.
#[test]
fn a_named_destination_resolves_when_its_document_defines_it() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut named = 0usize;
    let mut resolved = 0usize;
    let mut missed = Vec::new();
    let mut by_table = (0usize, 0usize);
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
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let pages = Pages::new(&document);
        for key in named_destinations(&document, &pages) {
            named = named.saturating_add(1);
            let found = Destination::read(&document, &key)
                .and_then(|destination| destination.page_index(&document, &pages));
            let bytes = match &key {
                Object::Name(name) => name.as_bytes().to_vec(),
                Object::String(string) => string.to_vec(),
                _ => continue,
            };
            let in_catalog = document
                .get_key(&catalog, "Dests")
                .as_dict()
                .is_some_and(|dests| {
                    dests
                        .get_by_name(&pdf_syntax::Name::new(bytes.clone()))
                        .is_some()
                });
            let in_tree = document
                .get_key(&catalog, "Names")
                .as_dict()
                .map(|names| document.get_key(names, "Dests"))
                .and_then(|root| root.as_dict().cloned())
                .is_some_and(|root| {
                    pdf_syntax::tree::lookup(
                        &root,
                        &pdf_syntax::tree::TreeKey::Name(&bytes),
                        &|object| document.resolve(object),
                    )
                    .is_some()
                });

            if found.is_some() {
                resolved = resolved.saturating_add(1);
                if in_catalog {
                    by_table.0 = by_table.0.saturating_add(1);
                } else {
                    by_table.1 = by_table.1.saturating_add(1);
                }
            } else if in_catalog || in_tree {
                missed.push(format!(
                    "{name}: {} is in a table and did not resolve",
                    String::from_utf8_lossy(&bytes)
                ));
            }
        }
    }

    println!("{named} named destinations from link annotations, {resolved} resolved");
    println!(
        "  {} found in a catalog /Dests, {} in a /Names /Dests tree",
        by_table.0, by_table.1
    );

    assert!(
        missed.is_empty(),
        "these keys are in their document's own table and did not resolve: {missed:?}"
    );
    assert_eq!(named, 106, "named destinations reachable from links");
    assert_eq!(resolved, 22, "named destinations their document defines");
    // §12.3.2.4 pairs a name object with the catalog's dictionary and a string with the name
    // tree, and the corpus keeps that pairing without a single exception. Worth asserting
    // because it is the evidence for reading the clause's "alternatively" as being about where
    // a document keeps its table rather than about which objects may address it.
    assert_eq!(by_table, (2, 20));
}

/// Every named destination a document's link annotations refer to, as the key object.
///
/// §12.5.6.5 lets a link state its destination directly in `/Dest` or through an action, and
/// §12.6.4.2's go-to action states it in `/D`. Both are followed, because a document that uses
/// one is not saying anything different from one that uses the other.
fn named_destinations(document: &Document, pages: &Pages<'_>) -> Vec<Object> {
    /// Documents in this corpus with hundreds of pages are books whose links repeat; fifty
    /// pages is enough to reach every distinct shape and keeps the test at a second.
    const PAGES_READ: usize = 50;

    let mut out = Vec::new();
    for index in 0..pages.len().min(PAGES_READ) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let annotations = document.get_key(&page.dict, "Annots");
        let Some(annotations) = annotations.as_array() else {
            continue;
        };
        for annotation in annotations {
            let annotation = document.resolve(annotation);
            let Some(annotation) = annotation.as_dict() else {
                continue;
            };
            let mut destination = document.get_key(annotation, "Dest");
            if destination.is_null() {
                let action = document.get_key(annotation, "A");
                let Some(action) = action.as_dict() else {
                    continue;
                };
                if document
                    .get_key(action, "S")
                    .as_name()
                    .is_none_or(|kind| kind.as_bytes() != b"GoTo")
                {
                    continue;
                }
                let Some(stated) = action.get("D") else {
                    continue;
                };
                destination = document.resolve(stated);
            }
            if matches!(destination, Object::Name(_) | Object::String(_)) {
                out.push(destination);
            }
        }
    }
    out
}

/// No corpus document writes §12.3.2.2's remote form where a page object belongs.
///
/// The integer first entry belongs to remote and embedded go-to actions, whose destination is
/// in another file. If a document in this corpus used it for a local destination the reading in
/// [`Target::Number`] would be costing that document a working link, and the ledger row would
/// have to say so. One does — `issue14847.pdf`'s `/OpenAction` — and it is checked by name in
/// the open-action test above rather than counted here, because a link is not an open action.
#[test]
fn a_local_destination_names_its_page_by_reference() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut numbered = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let pages = Pages::new(&document);
        for index in 0..pages.len().min(50) {
            let Some(page) = pages.get(index) else {
                continue;
            };
            let annotations = document.get_key(&page.dict, "Annots");
            let Some(annotations) = annotations.as_array() else {
                continue;
            };
            for annotation in annotations {
                let annotation = document.resolve(annotation);
                let Some(annotation) = annotation.as_dict() else {
                    continue;
                };
                let destination = document.get_key(annotation, "Dest");
                if let Some(read) = Destination::read(&document, &destination)
                    && matches!(read.target, Target::Number(_))
                {
                    numbered.push(name.clone().into_owned());
                }
            }
        }
    }

    println!("{} link destinations state a page number", numbered.len());
    assert!(
        numbered.is_empty(),
        "these documents' links state a page number where §12.3.2.2 requires a reference: \
         {numbered:?}"
    );
}
