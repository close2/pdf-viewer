//! §7.7.3.2's page tree node, and the one entry that decides whether a dictionary is a page.
//!
//! ISO 32000-2 §7.7.3.2 Table 30 makes a node's children **required**:
//!
//! > ( Required ) An array of indirect references to the immediate children of this node. The
//! > children shall only be page objects or other page tree nodes.
//!
//! and §7.7.3.3 Table 31 says what a page object calls itself:
//!
//! > ( Required ) The type of PDF object that this dictionary describes; shall be Page for a
//! > page object or Template for an invisible Template page (see 12.7.7, "Named pages").
//!
//! `pdf_model::page` walks the tree by asking for `/Kids` and treating a dictionary without one
//! as a leaf. That is deliberate and stays: a page object that omits `/Type` is common, and
//! trusting the entry in *that* direction would drop such pages from their documents. What it
//! could not distinguish was the file that answers the question itself — a dictionary stating
//! `/Type /Pages` and no `/Kids`, which has said in one breath that it is a node and that it has
//! no children. It was drawn as a page, and since it has no `/Contents` the page came back blank
//! **in silence**.
//!
//! # Each test is a pair differing in one entry
//!
//! Trap 8's fourth shape: the rule is what the *difference* between two files produces, so every
//! fixture below is `hello_world.pdf`'s two-object tree with exactly one thing changed. The
//! reason it needs pinning by hand is the census in ADR 0305: **not one** of the 65 703 crawled
//! web documents that open has such a node, so no corpus this project can grow will exercise it.
//!
//! # Where the witness came from
//!
//! `doc/corpora/format-corpus`, added in the four-hundred-and-seventieth session:
//! `pdf-handbuilt-test-corpus/T02-02_005_page-tree-no-kids.pdf`, one of 89 files carrying a
//! single deliberate structural defect apiece and all drawing the same *Hello PDF-world!*. It
//! rendered blank while the survey called it complete; it now draws the same ink as the intact
//! file, because the page its producer wrote is found by the recovery scan Table 31 justifies.
//! ADR 0305.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::path::Path;

use pdf_syntax::Document;

/// `hello_world.pdf`'s shape: a catalogue, one page tree node, one page, one content stream.
///
/// `root` is written verbatim as object 1's dictionary body and `page_type` as object 2's, so a
/// caller changes one entry and nothing else.
fn tree(root: &str, page_type: &str) -> Vec<u8> {
    // No cross-reference table: `Document::open` rebuilds by scanning for object headers, which
    // is what every fixture in `contents_entry.rs` relies on too and keeps the offsets out of a
    // hand-written file where they would rot on the first edit.
    format!(
        "%PDF-1.7\n\
         1 0 obj\n<< {root} >>\nendobj\n\
         2 0 obj\n<< /Parent 1 0 R /MediaBox [0 0 200 100] /Resources << >> {page_type} \
         /Contents 3 0 R >>\nendobj\n\
         3 0 obj\n<< /Length 30 >>\nstream\n0 0 1 rg 10 10 100 50 re f\nendstream\nendobj\n\
         4 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
         trailer\n<< /Root 4 0 R /Size 5 >>\n%%EOF\n"
    )
    .into_bytes()
}

/// How many pages the document has, and the media box of page one where there is one.
fn pages_of(bytes: Vec<u8>) -> (usize, Option<[f32; 4]>) {
    let document = Document::open(bytes).expect("the fixture opens");
    let pages = pdf_model::Pages::new(&document);
    (pages.len(), pages.get(0).map(|page| page.media_box))
}

/// The control: an intact tree yields its one page, and the page is object 2's.
///
/// Object 2 states a `/MediaBox` of its own that no other dictionary here states, so the
/// rectangle is what says *which* dictionary became the page.
#[test]
fn an_intact_node_yields_the_page_beneath_it() {
    let (count, media_box) = pages_of(tree("/Type /Pages /Count 1 /Kids [2 0 R]", "/Type /Page"));
    assert_eq!(count, 1);
    assert_eq!(media_box, Some([0.0, 0.0, 200.0, 100.0]));
}

/// The finding. The same file with `/Kids` deleted — and nothing else — must not draw object 1.
///
/// Table 30 makes the entry required, so a node without it has no children; Table 31 then makes
/// object 2 findable by its own declaration, which is `Pages::new`'s recovery scan. The page is
/// the *same* page: same media box, same content.
#[test]
fn a_node_without_kids_is_not_the_page() {
    let (count, media_box) = pages_of(tree("/Type /Pages /Count 1", "/Type /Page"));
    assert_eq!(count, 1);
    assert_eq!(
        media_box,
        Some([0.0, 0.0, 200.0, 100.0]),
        "object 1 has no /MediaBox, so the default US Letter rectangle here would mean the node \
         itself was drawn as the page"
    );
}

/// And where the scan finds nothing either, the document has no first page and says so.
///
/// The pair is the test above with object 2's `/Type` removed: a file whose tree yields nothing
/// and whose objects declare nothing is a file with no page, which every layer above reports by
/// name. A blank page invented here would be this reader's, not the producer's.
#[test]
fn a_node_without_kids_and_nothing_declaring_a_page_has_no_page() {
    let (count, media_box) = pages_of(tree("/Type /Pages /Count 1", ""));
    assert_eq!(count, 0);
    assert_eq!(media_box, None);
}

/// The rule the leaf walk was written for, unchanged: no `/Kids` and no `/Type` is a leaf.
///
/// This is the pair that stops the fix from being "trust `/Type`". A one-page file whose
/// catalogue points straight at a page object that omits Table 31's required entry still has its
/// page, and it is that dictionary rather than a recovered one.
#[test]
fn a_dictionary_with_neither_kids_nor_a_type_is_still_a_leaf() {
    let (count, media_box) = pages_of(tree(
        "/MediaBox [0 0 300 400] /Resources << >> /Contents 3 0 R",
        "/Type /Page",
    ));
    assert_eq!(count, 1);
    assert_eq!(
        media_box,
        Some([0.0, 0.0, 300.0, 400.0]),
        "object 1 is the leaf the catalogue names, and its own rectangle says so"
    );
}

/// A `/Count` on a node with no `/Kids` does not conjure the pages it claims.
///
/// `Pages::new` takes `/Count` as authoritative to keep opening a large document cheap, which is
/// right for every tree that has one. The pair here is the same root with and without children
/// under a `/Count` of 3: with them the entry is believed, without them the walk settles it.
#[test]
fn a_count_without_kids_is_not_believed() {
    let (with_kids, _) = pages_of(tree("/Type /Pages /Count 3 /Kids [2 0 R]", "/Type /Page"));
    assert_eq!(with_kids, 3, "a stated /Count over a real tree is believed");
    let (without_kids, _) = pages_of(tree("/Type /Pages /Count 3", "/Type /Page"));
    assert_eq!(
        without_kids, 1,
        "with no children the entry describes a subtree the file never wrote"
    );
}

/// The witness, from `doc/corpora/format-corpus` — a real file rather than a fixture.
///
/// Skipped where the submodule is not checked out, which is the pattern `contents_entry.rs` and
/// `tests/corpus.rs` use: a checkout without submodules is not a broken build, and no gate in
/// `doc/todo/02` §2 is made to depend on this corpus by its being here.
#[test]
fn the_handbuilt_witness_draws_its_page_rather_than_a_blank_one() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../doc/corpora/format-corpus/pdf-handbuilt-test-corpus/\
         T02-02_005_page-tree-no-kids.pdf",
    );
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: {} is not checked out", path.display());
        return;
    };
    let document = Document::open(bytes).expect("the witness opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the witness has a page");
    let (content, issues) = page.content_with_report(&document);
    assert_eq!(
        issues,
        Vec::new(),
        "the page's own content stream is intact"
    );
    assert!(
        String::from_utf8_lossy(&content).contains("Hello PDF-world!"),
        "the page found is the one the producer wrote, not the node above it"
    );
}
