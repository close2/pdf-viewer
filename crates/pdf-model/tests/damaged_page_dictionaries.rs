//! §7.3.7's dictionary, where a page object's own bytes stop being readable part-way through.
//!
//! ISO 32000-2 §7.3.7 makes a dictionary a delimited sequence of pairs — it "shall be written
//! as a sequence of key-value pairs enclosed in double angle brackets" — and it says what the
//! pairs are, which is the sentence that decides what a *prefix* of one is:
//!
//! > The entries in a dictionary represent an associative table and as such shall be unordered
//! > even though an arbitrary order may be imposed upon them when written in a file. That
//! > ordering shall be ignored.
//!
//! So the entries read whole before the damage are a **subset** of the dictionary's, every
//! member of it the producer's own, selected by an order the clause tells a reader to ignore —
//! which is why `pdf_syntax::Document::get` still refuses the object outright and why exactly
//! one caller may ask for the subset: `Pages`' recovery scan, which runs only where the page
//! tree yields no page at all and takes a prefix only on the strength of Table 31's `/Type
//! /Page` inside it. ADR 0784.
//!
//! # Each test is a pair differing in one thing
//!
//! `page_tree_nodes.rs`'s discipline, and trap 28's demand: **a recovery's guard states when
//! the recovery is needed and its comment states when it is right, so the round that writes one
//! owes the file where those two disagree.** The pairs below are exactly those files — a tree
//! that reaches a page beside one that does not, a prefix that declares itself a page beside one
//! whose damage falls before the declaration, and the object asked through the ordinary door
//! beside the same object asked through this one.
//!
//! # Where the witnesses came from
//!
//! `examples/standing_count_census` over the Tika issue-tracker corpus: of the 16 818 documents
//! that open, 18 state a page count this reader produces no page for, and 6 of those hold a page
//! object whose dictionary opens, states `/Type /Page`, and then stops. The sharpest is
//! `corpus-cache/tika-issue-tracker/batch2/batch2/GHOSTSCRIPT/GHOSTSCRIPT-701034-0.pdf`, whose
//! object 2 is `<< /Type /Page /Parent 3 0 R /Resources 6 0 R /Contents 4 0 R /MediaBox
//! [0 0 292 3 >] /Rotate 0 >>` — one byte of one array, costing a nine-page document every page
//! it has.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use pdf_syntax::{Document, ObjectId};

/// A two-object tree whose page object's dictionary body is written verbatim.
///
/// `kids` is what object 1's `/Kids` names and `body` is object 2's whole dictionary body, so a
/// caller changes the tree or the damage and nothing else. Object 1 states the `/MediaBox`,
/// which is how a recovered page's §7.7.3.4 inheritance up its own `/Parent` becomes visible.
fn document(kids: &str, body: &str) -> Vec<u8> {
    // No cross-reference table: `Document::open` rebuilds by scanning for object headers, which
    // keeps the offsets out of a hand-written file where they would rot on the first edit.
    format!(
        "%PDF-1.7\n\
         1 0 obj\n<< /Type /Pages /Count 1 /Kids [{kids}] /MediaBox [0 0 200 100] >>\nendobj\n\
         2 0 obj\n<< {body} >>\nendobj\n\
         3 0 obj\n<< /Length 30 >>\nstream\n0 0 1 rg 10 10 100 50 re f\nendstream\nendobj\n\
         4 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
         trailer\n<< /Root 4 0 R /Size 5 >>\n%%EOF\n"
    )
    .into_bytes()
}

/// An intact page object, for the arm of each pair that has to keep working.
const WHOLE: &str = "/Type /Page /Parent 1 0 R /Resources << >> /Contents 3 0 R /Rotate 0";

/// The same object with one byte of its last entry replaced, exactly as the witness is damaged.
///
/// `>` is no object — §7.3.3 admits nothing spelled that way and the lexer hands it back as the
/// keyword it lexically is — so the array never closes and the dictionary never reaches its
/// `>>`. What is whole before it is `/Type`, `/Parent`, `/Resources` and `/Contents`.
const DAMAGED: &str =
    "/Type /Page /Parent 1 0 R /Resources << >> /Contents 3 0 R /CropBox [0 0 200 >]  /Rotate 0";

/// The count, the first page's media box, and what the page says about its own dictionary.
fn pages_of(bytes: Vec<u8>) -> (usize, Option<[f32; 4]>, Option<usize>) {
    let document = Document::open(bytes).expect("the fixture opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0);
    (
        pages.len(),
        page.as_ref().map(|page| page.media_box),
        page.as_ref()
            .and_then(|page| page.damaged_dictionary.as_ref())
            .map(|damage| damage.entries),
    )
}

/// The control: an intact object under a tree that reaches it is the page, undamaged.
#[test]
fn an_intact_page_under_a_working_tree_reports_no_damage() {
    let (count, media_box, damage) = pages_of(document("2 0 R", WHOLE));
    assert_eq!(count, 1);
    assert_eq!(media_box, Some([0.0, 0.0, 200.0, 100.0]));
    assert_eq!(damage, None);
}

/// The finding: the same tree, the same object, one byte of one array changed.
///
/// Before this round the whole object was refused, the tree reached nothing, the recovery scan
/// found nothing to declare a page, and `/Count 1` stood over a document that showed nothing.
/// The entries before the damage are the producer's own, and §7.7.3.4's inheritance up the
/// page's own `/Parent` still supplies the rectangle object 1 states.
#[test]
fn a_page_object_that_stops_part_way_is_read_as_far_as_it_states() {
    let (count, media_box, damage) = pages_of(document("2 0 R", DAMAGED));
    assert_eq!(count, 1);
    assert_eq!(
        media_box,
        Some([0.0, 0.0, 200.0, 100.0]),
        "the prefix carries /Parent, so §7.7.3.4's inheritance reaches object 1's rectangle"
    );
    assert_eq!(
        damage,
        Some(4),
        "/Type, /Parent, /Resources and /Contents were whole; /CropBox and /Rotate are not here"
    );
}

/// The guard's pair, which is what trap 28 asks for: a tree that **does** reach a page.
///
/// The recovery scan runs only where the tree yields nothing, so a document whose tree works
/// must take its page from the tree even though a damaged page object is sitting in the same
/// file. Object 5 below is that object; object 2 is the page, and the media box says which.
#[test]
fn a_damaged_page_object_is_not_taken_where_the_tree_reaches_a_page() {
    let mut bytes = document("2 0 R", WHOLE);
    bytes.extend_from_slice(
        b"5 0 obj\n<< /Type /Page /MediaBox [0 0 999 999] /CropBox [0 0 9 >] >>\nendobj\n",
    );
    let (count, media_box, damage) = pages_of(bytes);
    assert_eq!(count, 1);
    assert_eq!(
        media_box,
        Some([0.0, 0.0, 200.0, 100.0]),
        "999 here would mean the scan ran beside a working tree and displaced its page"
    );
    assert_eq!(damage, None);
}

/// The declaration's pair: damage that falls **before** `/Type` leaves nothing to take.
///
/// The prefix is taken on the strength of Table 31's own required entry, which is the same
/// declaration ADR 0782's recovery rests on. A prefix that has not reached it says nothing about
/// what the object is, and inventing a page from `/Resources` and `/Contents` alone would be a
/// guess rather than a recovery.
#[test]
fn a_prefix_that_has_not_reached_its_own_type_is_not_a_page() {
    let (count, media_box, damage) = pages_of(document(
        "9 0 R",
        "/Parent 1 0 R /Contents 3 0 R /CropBox [0 0 200 >] /Type /Page",
    ));
    assert_eq!(
        count, 1,
        "nothing was examined and nothing was recovered, so §7.7.3.2's /Count stands (ADR 0782)"
    );
    assert_eq!(media_box, None, "and there is no page to give back");
    assert_eq!(damage, None);
}

/// And the ordinary door is unchanged: the damaged object is still nothing to every other reader.
///
/// The recovery is *additive* — it adds a page where there was none — and this is the half of
/// that claim which is about the rest of the document graph. A reference to object 2 from
/// anywhere else still resolves to §7.3.10's null, so nothing that read a whole dictionary
/// before now reads a partial one.
#[test]
fn the_damaged_object_is_still_null_through_the_ordinary_door() {
    let document = Document::open(document("2 0 R", DAMAGED)).expect("the fixture opens");
    let id = ObjectId::new(2, 0);
    assert_eq!(document.get(id), pdf_syntax::Object::Null);
    assert_eq!(
        document
            .damaged_dictionary(id)
            .map(|damaged| damaged.entries.len()),
        Some(4),
        "the same object, asked the other question, states four whole entries"
    );
}

/// The report reaches a page's interpretation, worded as a statement about the file.
///
/// Trap 5: a page assembled from part of its own dictionary must not be indistinguishable from
/// one whose producer wrote exactly that. Every entry after the damage is being read as Table
/// 31's default, and this is the sentence that says so.
#[test]
fn the_page_says_out_loud_that_its_dictionary_was_read_in_part() {
    let document = Document::open(document("2 0 R", DAMAGED)).expect("the fixture opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the recovered page");
    let reports = pdf_model::content::interpret(&document, &page).unsupported;
    let said = reports.iter().any(|report| {
        matches!(
            report,
            pdf_model::content::Unsupported::PageDictionary { detail }
                if detail.contains("4 entr") && detail.contains("stops being readable")
        )
    });
    assert!(said, "no report named the damaged dictionary: {reports:?}");
}
