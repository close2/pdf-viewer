//! §7.7.3.2's page tree node, and the one entry that decides whether a dictionary is a page.
//!
//! ISO 32000-2 §7.7.3.2 Table 30 makes a node's children **required**:
//!
//! > ( Required ) An array of indirect references to the immediate children of this node. The
//! > children shall only be page objects or other page tree nodes.
//!
//! Errata Collection 3's Issue #271 adds two rules to that cell — *null entries shall not be
//! present* and *the length of the array shall be at least one* — and a third to `/Count`, which
//! *shall be 1 or greater*. They are written here as prose rather than as part of the quotation
//! because `doc/md/` is what `cargo test -p conformance` verifies a blockquote against.
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

/// An **empty** `/Kids` is not children either, so the `/Count` beside it is not believed.
///
/// The pair above is a root with no `/Kids` at all; this is the root that writes the entry and
/// leaves it empty, which the published Table 30 gave a reader nothing to say about — an array
/// is an array. Errata Collection 3's Issue #271 inserts into the cell that the array's *length
/// shall be at least one*, so `[]` is the same self-contradiction as the absent entry: a node
/// stating its children and stating none of them.
///
/// What it cost was a document reporting three pages and producing not one of them, in silence,
/// while the page its producer wrote sat in the file for the recovery scan to find.
#[test]
fn a_count_over_an_empty_kids_is_not_believed() {
    let (count, media_box) = pages_of(tree("/Type /Pages /Count 3 /Kids []", "/Type /Page"));
    assert_eq!(
        count, 1,
        "an empty array states no children, so /Count describes a subtree the file never wrote"
    );
    assert_eq!(
        media_box,
        Some([0.0, 0.0, 200.0, 100.0]),
        "object 2 declares itself a page and the recovery scan finds it"
    );
}

/// And an empty-`/Kids` node *inside* a tree does not consume the pages it claims.
///
/// The same erratum one level down, where the price is a page's index rather than a page count:
/// `find_leaf` skips a whole subtree on its `/Count` to keep a lookup in a hundred-thousand-page
/// document cheap, and a childless node claiming one page took one off the walk's countdown — so
/// every page after it answered to the number of the page before. Two pages behind such a node
/// is the smallest tree where that is visible, and the root states no `/Count` so that the walk
/// rather than an entry decides how many pages there are.
#[test]
fn an_empty_kids_node_does_not_consume_the_pages_its_count_claims() {
    let bytes = b"%PDF-1.7\n\
         1 0 obj\n<< /Type /Pages /Kids [6 0 R 2 0 R 5 0 R] >>\nendobj\n\
         6 0 obj\n<< /Type /Pages /Parent 1 0 R /Count 1 /Kids [] >>\nendobj\n\
         2 0 obj\n<< /Type /Page /Parent 1 0 R /MediaBox [0 0 200 100] >>\nendobj\n\
         5 0 obj\n<< /Type /Page /Parent 1 0 R /MediaBox [0 0 300 400] >>\nendobj\n\
         4 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
         trailer\n<< /Root 4 0 R /Size 7 >>\n%%EOF\n"
        .to_vec();
    let document = Document::open(bytes).expect("the fixture opens");
    let pages = pdf_model::Pages::new(&document);
    assert_eq!(pages.len(), 2, "object 6 has no children and is no page");
    assert_eq!(
        pages.get(0).map(|page| page.media_box),
        Some([0.0, 0.0, 200.0, 100.0])
    );
    assert_eq!(
        pages.get(1).map(|page| page.media_box),
        Some([0.0, 0.0, 300.0, 400.0]),
        "a skip taken on the empty node's /Count would answer page one here and hide page two"
    );
}

/// A `/Kids` entry naming nothing is not a page, and it does not move the pages after it.
///
/// Table 30 says "[t]he children shall only be page objects or other page tree nodes", so an
/// entry resolving to neither is not a child at all — and §7.3.10 makes a reference to an
/// undefined object resolve to null rather than be an error, which is how a file arrives in this
/// shape. Issue #271 states the same conclusion in the cell, *null entries shall not be present*,
/// which makes stepping over one a recovery from a malformed file rather than a reading of a
/// legal array. **The half worth pinning is the counting**: an entry read as a childless page would
/// consume one of the pages the walk counts down on its way to the index it was asked for, and
/// every page after it would answer to the index of the one before. Two pages, with the dangling
/// entry in front of both, is the smallest tree where that is visible.
///
/// It is here because the walk stopped copying the objects it steps over (ADR 0330): a node held
/// by name has to be *asked* whether it is a dictionary, where resolving one answered by handing
/// over nothing.
#[test]
fn a_kids_entry_that_names_no_node_is_not_a_page() {
    // Two pages of different sizes under a root with no `/Count`, so the tree is walked rather
    // than believed and each page's own rectangle says which page came back.
    let bytes = b"%PDF-1.7\n\
         1 0 obj\n<< /Type /Pages /Kids [9 0 R 2 0 R 5 0 R] >>\nendobj\n\
         2 0 obj\n<< /Type /Page /Parent 1 0 R /MediaBox [0 0 200 100] >>\nendobj\n\
         5 0 obj\n<< /Type /Page /Parent 1 0 R /MediaBox [0 0 300 400] >>\nendobj\n\
         4 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
         trailer\n<< /Root 4 0 R /Size 6 >>\n%%EOF\n"
        .to_vec();
    let document = Document::open(bytes).expect("the fixture opens");
    let pages = pdf_model::Pages::new(&document);
    assert_eq!(pages.len(), 2, "object 9 does not exist and is not a page");
    assert_eq!(
        pages.get(0).map(|page| page.media_box),
        Some([0.0, 0.0, 200.0, 100.0])
    );
    assert_eq!(
        pages.get(1).map(|page| page.media_box),
        Some([0.0, 0.0, 300.0, 400.0]),
        "an entry counted as a page would have stopped the walk one page early"
    );
}

/// The corpus's one empty-`/Kids` witness keeps all three of its pages, in order.
///
/// `examples/kidless_node_census` over `doc/pdf.js` and `doc/corpora` finds exactly one document
/// whose tree holds a node with an empty `/Kids`, and it writes `/Count 0` beside it — the value
/// Issue #271's third insertion outlaws in the same breath as the empty array. That zero is why
/// the file reads correctly today: `find_leaf` skips a subtree only on a positive `/Count`, so
/// the node consumed nothing. The pair with the fixture above is the point — the fixture writes
/// the count the erratum forbids and the witness writes the one it forbids differently — and
/// this end of it is what says the change is not an over-correction on a real file.
#[test]
fn the_corpus_witness_with_an_empty_kids_yields_its_pages_in_order() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/issue8088.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: {} is not checked out", path.display());
        return;
    };
    let document = Document::open(bytes).expect("the witness opens");
    let pages = pdf_model::Pages::new(&document);
    assert_eq!(pages.len(), 3);
    let numbers: Vec<Option<u32>> = (0..3)
        .map(|index| {
            pages
                .get(index)
                .and_then(|page| page.id)
                .map(|id| id.number)
        })
        .collect();
    assert_eq!(
        numbers,
        vec![Some(10), Some(1), Some(5)],
        "the tree's own order: the empty node sits between the root and the two pages under it"
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
