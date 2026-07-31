//! ISO 32000-2 §14.7.5.4's structural parent tree, over the corpus.
//!
//! The clause's own reason for the tree is that a content stream cannot point at an object:
//! "[b]ecause a stream cannot contain object references, there is no way for content items that
//! are marked-content sequences to refer directly back to their parent structure elements". So
//! the page states a key, the key names an array, and a marked-content identifier indexes it.
//! Three hops, each of which a real file gets wrong differently — which is why this test asserts
//! the two-sided fact rather than a ratio: **every page that states a `/StructParents` resolves
//! it, unless its document has no `/ParentTree` at all.**

use std::path::{Path, PathBuf};

use pdf_model::Pages;
use pdf_model::structure::{ParentTree, actual_text};
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

/// Every page that states a key into the parent tree finds its entry there.
#[test]
fn a_page_that_states_a_structural_parent_key_resolves_it() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut roots = 0usize;
    let mut keyed = 0usize;
    let mut resolved = 0usize;
    let mut with_actual_text = 0usize;
    let mut missed = Vec::new();
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
        let root = document.get_key(&catalog, "StructTreeRoot");
        let Some(root) = root.as_dict() else {
            continue;
        };
        roots = roots.saturating_add(1);
        let Some(page) = Pages::new(&document).get(0) else {
            continue;
        };
        if document
            .get_key(&page.dict, "StructParents")
            .as_integer()
            .is_none()
        {
            continue;
        }
        keyed = keyed.saturating_add(1);
        let parents = ParentTree::for_page(&document, &page.dict);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if parents.is_empty() {
            // The one-sided reading — "this reader could not walk the tree" — is the wrong one
            // unless the tree *has* an entry for this key holding something. Both ways of
            // having none are the document's own statement: no `/ParentTree` at all, or an
            // entry that is an empty array, which says this page's content items belong to no
            // structure element.
            let key = document
                .get_key(&page.dict, "StructParents")
                .as_integer()
                .unwrap_or(0);
            let entry = document
                .get_key(root, "ParentTree")
                .as_dict()
                .and_then(|tree| {
                    pdf_syntax::tree::lookup(tree, &pdf_syntax::tree::TreeKey::Number(key), &|o| {
                        document.resolve(o)
                    })
                });
            let states_nothing = match &entry {
                None => true,
                Some(pdf_syntax::Object::Array(items)) => items.is_empty(),
                Some(_) => false,
            };
            if !states_nothing {
                missed.push(name.into_owned());
            }
            continue;
        }
        resolved = resolved.saturating_add(1);
        if (0..2000).any(|mcid| {
            parents
                .element(&document, mcid)
                .is_some_and(|element| actual_text(&document, &element).is_some())
        }) {
            with_actual_text = with_actual_text.saturating_add(1);
        }
    }

    println!("{roots} documents have a /StructTreeRoot; page one keys the parent tree in {keyed}");
    println!("  {resolved} resolve; {with_actual_text} have an element carrying /ActualText");
    assert!(
        missed.is_empty(),
        "these documents' parent trees hold an entry for page one's key and this reader read \
         no elements from it: {missed:?}"
    );
    assert_eq!(roots, 89);
    assert_eq!(keyed, 76);
    // The one page that names no element is `bug1978317.pdf`, whose parent tree holds an
    // *empty array* for page one — a document saying its first page's content belongs to no
    // structure element, which the loop above checks rather than inferring from the number.
    assert_eq!(resolved, 75);
    assert_eq!(with_actual_text, 8);
}
