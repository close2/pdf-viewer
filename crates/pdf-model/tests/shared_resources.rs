//! A page's resource dictionary is shared among the pages that name it, never copied per page.
//!
//! ISO 32000-2 §7.8.3 describes the sharing this reads:
//!
//! > If the content streams of multiple pages require exactly the same set of resources, a
//! > single Resources dictionary may be shared between them by using indirect references.
//!
//! and §7.7.3.4's inheritance is the other route to the same dictionary for every page beneath a
//! node. Resolving a reference hands back a copy of the whole object, so a copy kept per page
//! multiplies the dictionary by the pages naming it, and 2 000 pages of one 20 000-entry
//! dictionary is then past the transform worker's memory. `Page::resources` is an `Arc` that every
//! page of one `Pages::every` naming the same object holds (ADR 2588); these tests pin that the
//! pages share it, that two objects stay two, and that a page's own change reaches no other page
//! and not the document.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::Page;
use pdf_syntax::{Document, Name, Object, ObjectId};

/// How each page reaches its resources.
#[derive(Clone, Copy)]
enum Shape {
    /// Every page states `/Resources 3 0 R`, §7.8.3's sharing by reference.
    ByReference,
    /// The page tree node states `/Resources 3 0 R` and no page does, §7.7.3.4's inheritance.
    Inherited,
    /// Every page writes an equal dictionary of its own, directly — equal, and not one object.
    Direct,
}

/// `pages` pages under one node, each reaching a resource dictionary of `width` graphics states.
///
/// Object 1 is the node, 2 the catalogue, 3 the resource dictionary, 4 the one content stream,
/// and the pages are 5 onwards. No cross-reference table: `Document::open` rebuilds one by
/// scanning, as `page_tree_nodes.rs`'s fixtures rely on.
fn document(pages: usize, width: usize, shape: Shape) -> Vec<u8> {
    let mut states = String::new();
    for at in 0..width {
        write!(
            states,
            "/G{at} << /Type /ExtGState /LW {} >> ",
            (at % 7).saturating_add(1)
        )
        .expect("a String");
    }
    let resources = format!("<< /ExtGState << {states}>> >>");
    let kids: Vec<String> = (0..pages)
        .map(|at| format!("{} 0 R", at.saturating_add(5)))
        .collect();
    let (on_node, on_page) = match shape {
        Shape::ByReference => ("", "/Resources 3 0 R".to_owned()),
        Shape::Inherited => ("/Resources 3 0 R", String::new()),
        Shape::Direct => ("", format!("/Resources {resources}")),
    };
    let mut file = format!(
        "%PDF-1.7\n\
         1 0 obj\n<< /Type /Pages /Kids [{}] /Count {pages} /MediaBox [0 0 200 200] {on_node} >>\nendobj\n\
         2 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
         3 0 obj\n{resources}\nendobj\n\
         4 0 obj\n<< /Length 30 >>\nstream\n0 0 1 rg 10 10 100 50 re f\nendstream\nendobj\n",
        kids.join(" ")
    );
    for at in 0..pages {
        write!(
            file,
            "{} 0 obj\n<< /Type /Page /Parent 1 0 R /Contents 4 0 R {on_page} >>\nendobj\n",
            at.saturating_add(5)
        )
        .expect("a String");
    }
    file.push_str("trailer\n<< /Root 2 0 R >>\n%%EOF\n");
    file.into_bytes()
}

/// Every page of the document, from the one walk `Pages::every` makes.
fn every(document: &Document) -> Vec<Page> {
    pdf_model::Pages::new(document)
        .every()
        .into_iter()
        .map(|page| page.expect("every page of an intact tree is found"))
        .collect()
}

/// Object 3, the dictionary every page reaches, as the document holds it.
fn object_three(document: &Document) -> pdf_syntax::Dictionary {
    document
        .get(ObjectId::new(3, 0))
        .as_dict()
        .cloned()
        .expect("object 3 is the resource dictionary")
}

/// Pages naming one dictionary by reference hold one dictionary, and it is the file's.
#[test]
fn pages_naming_one_resource_dictionary_share_it() {
    let document = Document::open(document(300, 40, Shape::ByReference)).expect("opens");
    let pages = every(&document);
    assert_eq!(pages.len(), 300);
    assert!(
        pages
            .iter()
            .all(|page| Arc::ptr_eq(&page.resources, &pages[0].resources)),
        "every page holds the one dictionary"
    );
    assert_eq!(*pages[0].resources, object_three(&document));
}

/// Pages inheriting the node's dictionary hold the node's one dictionary.
#[test]
fn pages_inheriting_one_resource_dictionary_share_it() {
    let document = Document::open(document(300, 40, Shape::Inherited)).expect("opens");
    let pages = every(&document);
    assert_eq!(pages.len(), 300);
    assert!(
        pages
            .iter()
            .all(|page| Arc::ptr_eq(&page.resources, &pages[0].resources)),
        "every page holds the node's one dictionary"
    );
    assert_eq!(*pages[0].resources, object_three(&document));
}

/// Equal dictionaries written separately are separate: sharing follows the reference, never the
/// content, because only the reference says the pages name one object.
#[test]
fn equal_dictionaries_written_per_page_are_not_merged() {
    let document = Document::open(document(3, 4, Shape::Direct)).expect("opens");
    let pages = every(&document);
    assert_eq!(*pages[0].resources, *pages[1].resources);
    assert_eq!(*pages[0].resources, object_three(&document));
    assert!(!Arc::ptr_eq(&pages[0].resources, &pages[1].resources));
}

/// One page lookup reads the same dictionary the walk shares.
#[test]
fn a_page_looked_up_alone_reads_what_the_walk_shares() {
    for shape in [Shape::ByReference, Shape::Inherited, Shape::Direct] {
        let document = Document::open(document(5, 4, shape)).expect("opens");
        let pages = pdf_model::Pages::new(&document);
        let walked = every(&document);
        for (at, page) in walked.iter().enumerate() {
            let alone = pages.get(at).expect("the page is there");
            assert_eq!(*alone.resources, *page.resources);
        }
    }
}

/// A change one page makes to its resources stays on that page, as an edit beside the document
/// must: `Arc::make_mut` copies the dictionary the other pages still share, and neither they nor
/// the document see the change.
#[test]
fn a_page_s_own_change_reaches_no_other_page_and_not_the_document() {
    let document = Document::open(document(3, 4, Shape::ByReference)).expect("opens");
    let mut pages = every(&document);
    let untouched = object_three(&document);
    Arc::make_mut(&mut pages[0].resources).insert(Name::new(&b"Added"[..]), Object::Integer(1));
    assert!(pages[0].resources.get("Added").is_some());
    assert!(Arc::ptr_eq(&pages[1].resources, &pages[2].resources));
    assert!(pages[1].resources.get("Added").is_none());
    assert_eq!(*pages[1].resources, untouched);
    assert_eq!(object_three(&document), untouched);
    assert_eq!(*every(&document)[0].resources, untouched);
}

/// The shape that is past the worker's memory when copied per page, scaled to a test: many pages over one wide
/// dictionary hold it once, so the walk's memory is the dictionary's and not pages times it.
#[test]
fn many_pages_over_one_wide_dictionary_hold_it_once() {
    let document = Document::open(document(2_000, 2_000, Shape::ByReference)).expect("opens");
    let pages = every(&document);
    assert_eq!(pages.len(), 2_000);
    assert_eq!(pages[0].resources.len(), 1);
    // Every page's handle and none besides: the walk's memo is dropped with the walk.
    assert_eq!(Arc::strong_count(&pages[0].resources), 2_000);
}
