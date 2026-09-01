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
//! tree yields no page at all. ADR 0784.
//!
//! # Two doors, and the second one is the tree's
//!
//! A prefix is taken on the strength of Table 31's `/Type /Page` inside it — or, where the page
//! tree's own `/Kids` names the object, on the strength of an entry only a page object may
//! carry. §7.7.3.2 makes a child "a page object or [an]other page tree node" and then closes
//! what the second may hold: "a page tree node may contain further entries defining inherited
//! attributes for the page objects that are its descendants", which §7.7.3.4 enumerates as four.
//! So a named object stating `/Contents` is a page, and the evidence is a Table 31 entry being
//! *present* rather than a node's entry being absent — a distinction §7.3.7 forces, because a
//! subset can only say what the producer did write. ADR 0786.
//!
//! # Each test is a pair differing in one thing
//!
//! `page_tree_nodes.rs`'s discipline, and trap 28's demand: **a recovery's guard states when
//! the recovery is needed and its comment states when it is right, so the round that writes one
//! owes the file where those two disagree.** The pairs below are exactly those files — a tree
//! that reaches a page beside one that does not, a prefix that declares itself a page beside one
//! whose damage falls before the declaration, the object asked through the ordinary door beside
//! the same object asked through this one, and, for the second door, one `/Kids` array that names
//! the object beside one that does not, plus a tree that still reaches a page with a tree-named
//! damaged object sitting in it.
//!
//! # Where the witnesses came from
//!
//! `examples/standing_count_census` over the Tika issue-tracker corpus: of the 16 818 documents
//! that open, 18 stated a page count this reader produced no page for **before either door
//! existed**, and 6 of those held a page object whose dictionary opens, states `/Type /Page`, and
//! then stops — the census prints what is left. The sharpest of the 6 is
//! `corpus-cache/tika-issue-tracker/batch2/batch2/GHOSTSCRIPT/GHOSTSCRIPT-701034-0.pdf`, whose
//! object 2 is `<< /Type /Page /Parent 3 0 R /Resources 6 0 R /Contents 4 0 R /MediaBox
//! [0 0 292 3 >] /Rotate 0 >>` — one byte of one array, costing a nine-page document every page
//! it has.
//!
//! The second door's witnesses are the five the same census names under *no object declares
//! `/Type /Page`*, which is the cause it could not tell apart until it followed `/Kids`:
//! `GHOSTSCRIPT-698991-0`, `GHOSTSCRIPT-699018-0`, `GHOSTSCRIPT-699521-0`, `GHOSTSCRIPT-701846-0`
//! and `poppler-192-0`. The third of those is 795 × 842 with a `/Contents` that reads
//! `BT /F1 30 Tf 350 750 Td 20 TL 5 Tr (Hello world) Tj ET`, and its damage is a *second*
//! `/MediaBox` whose value is the bare keyword `e`.

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

/// The declaration's pair: damage before `/Type`, in an object **the tree does not name**.
///
/// Out of a scan of the whole file, a prefix that has not reached its own `/Type` says nothing
/// about what the object is, and inventing a page from `/Resources` and `/Contents` alone would
/// be a guess rather than a recovery. Object 9 does not exist, so nothing is named and nothing is
/// examined. Its partner is the test below, which changes the `/Kids` array and nothing else.
#[test]
fn a_prefix_with_no_type_that_no_kids_array_names_is_not_a_page() {
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

/// The second door, and the finding: the same prefix, in an object the tree **does** name.
///
/// One character of the `/Kids` array separates this from the test above, which is what makes it
/// the pair. §7.7.3.2 says the child "shall only be [a] page object[] or [an]other page tree
/// node[]", and §7.7.3.4's four inheritable attributes plus Table 30's four are the whole of what
/// the second may carry — so `/Contents` in the subset settles which it is. The page then draws
/// object 3's blue rectangle on the rectangle its own `/Parent` states. ADR 0786.
#[test]
fn a_prefix_the_tree_names_is_a_page_where_it_states_an_entry_only_a_page_carries() {
    let (count, media_box, damage) = pages_of(document(
        "2 0 R",
        "/Parent 1 0 R /Contents 3 0 R /CropBox [0 0 200 >] /Type /Page",
    ));
    assert_eq!(count, 1);
    assert_eq!(
        media_box,
        Some([0.0, 0.0, 200.0, 100.0]),
        "the prefix carries /Parent, so §7.7.3.4's inheritance reaches object 1's rectangle"
    );
    assert_eq!(
        damage,
        Some(2),
        "/Parent and /Contents were whole; /CropBox and the /Type after it are not here"
    );
}

/// And the entry has to be one **Table 31 names**, which is where `poppler-355-0.pdf` sits.
///
/// Its prefix is `/Parent`, `/CropBox` and a key in neither table — the file spells one
/// `/WinAnsiEncope` — and both of the first two are things §7.7.3.2 lets a page tree node carry.
/// Reading the third as evidence would mean deciding the object is a page because it does not
/// look like a node, which is the substitutive direction trap 5 forbids: §7.3.7's subset can say
/// what the producer wrote and never what it did not.
#[test]
fn a_prefix_the_tree_names_whose_entries_a_node_may_also_carry_is_not_a_page() {
    let (count, media_box, damage) = pages_of(document(
        "2 0 R",
        "/Parent 1 0 R /WinAnsiEncope 4 /CropBox [0 0 200 >] /Contents 3 0 R",
    ));
    assert_eq!(count, 1, "so §7.7.3.2's /Count stands");
    assert_eq!(media_box, None, "and there is no page to give back");
    assert_eq!(damage, None);
}

/// And a prefix that reached its own `/Type` and wrote `Pages` there is believed.
///
/// Table 30 makes `/Type` required of a node and says it "shall be Pages", so the file has stated
/// what the object is in the one place that settles it; a `/Contents` after that is the file
/// contradicting itself, and this reads the entry the producer wrote rather than overriding it.
#[test]
fn a_prefix_the_tree_names_that_calls_itself_a_node_is_not_a_page() {
    let (count, media_box, damage) = pages_of(document(
        "2 0 R",
        "/Type /Pages /Parent 1 0 R /Contents 3 0 R /CropBox [0 0 200 >]",
    ));
    assert_eq!(count, 1, "so §7.7.3.2's /Count stands");
    assert_eq!(media_box, None, "and there is no page to give back");
    assert_eq!(damage, None);
}

/// The second door's guard pair, which is what trap 28 asks of it: a tree that **does** reach a
/// page, with a tree-named damaged object beside it.
///
/// The rightness condition holds here — object 5 is named by the root's `/Kids` and its prefix
/// states `/Contents` — and the guard does not, because object 2 is a page the tree reaches. The
/// recovery must therefore not run at all: `/Count 2` stands over the one page the tree yields,
/// and the scan's ascending-object-number order never replaces the order the tree stated.
#[test]
fn a_tree_named_prefix_is_not_taken_where_the_tree_still_reaches_a_page() {
    let bytes = format!(
        "%PDF-1.7\n\
         1 0 obj\n<< /Type /Pages /Count 2 /Kids [2 0 R 5 0 R] /MediaBox [0 0 200 100] >>\nendobj\n\
         2 0 obj\n<< {WHOLE} >>\nendobj\n\
         3 0 obj\n<< /Length 30 >>\nstream\n0 0 1 rg 10 10 100 50 re f\nendstream\nendobj\n\
         4 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
         5 0 obj\n<< /Parent 1 0 R /Contents 3 0 R /MediaBox [0 0 999 >] >>\nendobj\n\
         trailer\n<< /Root 4 0 R /Size 6 >>\n%%EOF\n"
    )
    .into_bytes();
    let (count, media_box, damage) = pages_of(bytes);
    assert_eq!(
        count, 2,
        "the tree yielded a page, so /Count is the only statement in evidence (ADR 0782)"
    );
    assert_eq!(
        media_box,
        Some([0.0, 0.0, 200.0, 100.0]),
        "999 here would mean the scan ran beside a working tree and displaced its page"
    );
    assert_eq!(damage, None);
}

/// The report names which door the page came through, because they are two different claims.
///
/// One is Table 31's `/Type` read off the producer's own bytes; the other is this reader's
/// inference from §7.7.3.2 about an object that never declared itself. Trap 5's rule is that a
/// substitution is said out loud, and presenting the second as though it were the first would be
/// exactly the silence it forbids.
#[test]
fn the_page_says_which_evidence_made_it_a_page() {
    let bytes = document(
        "2 0 R",
        "/Parent 1 0 R /Contents 3 0 R /CropBox [0 0 200 >]",
    );
    let document = Document::open(bytes).expect("the fixture opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the recovered page");
    assert_eq!(
        page.damaged_dictionary
            .as_ref()
            .map(|damage| damage.identification),
        Some(pdf_model::page::PageIdentification::TheTreeAndAPageOnlyEntry("Contents"))
    );
    let reports = pdf_model::content::interpret(&document, &page).unsupported;
    let said = reports.iter().any(|report| {
        matches!(
            report,
            pdf_model::content::Unsupported::PageDictionary { detail }
                if detail.contains("the page tree's /Kids names it") && detail.contains("/Contents")
        )
    });
    assert!(said, "no report named the evidence: {reports:?}");
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

/// The door that stays shut, and what would happen if it did not.
///
/// `doc/todo/03` section 34 named a third door — resynchronising past a value that will not parse, so
/// that a `/Type /Page` sitting four bytes past the damage could be read. ADR 0787 read §7.3 for
/// it and closed it. The reading's surprise is that section 34's own objection is answerable: §7.2.1
/// puts tokens below objects, §7.2.3 gives a run of regular characters exactly one end — "[a]
/// sequence of consecutive regular characters comprises a single token" — and §7.3.1's nine
/// types with their stated introducers make the set of tokens that may *begin* a value closed. So
/// stepping over a regular run that begins no object is not a guess about a value's extent.
///
/// What refuses it is the sentence bounding §7.2.3: its rules "apply to all characters in the
/// file except within strings, streams, and comments". A reader knows it is outside those three
/// only by having tokenised continuously from the `<<`, and **continuity is exactly what ADR
/// 0784's subset argument rests on** — the entries are a subset because every one of them is the
/// producer's own. A gap ends that, and the `(` that would have said a `/` is string content
/// rather than a key is precisely the kind of byte the damage destroys.
///
/// The three tests below are that argument as files. They are a triple rather than a pair because
/// the manufacture needs a third arm to be visible at all: the tree names object 2 in each.
mod the_third_door {
    use super::{Document, document};

    /// A: the producer wrote one entry, whose value is a string that happens to contain a name.
    ///
    /// Nothing in the prefix is an entry only a page object carries, so ADR 0786's door declines
    /// and the object is refused — which is right, because the producer wrote no `/Contents`.
    const INTACT_STRING: &str = "/Note (junk /Contents 9 0 R more) /Rotate [0 >]";

    /// B: the same bytes with the string's opening `(` replaced by one regular character.
    ///
    /// One byte. `Zjunk` is now a token §7.2.3 makes whole and §7.3 gives no object to begin, so
    /// it is where a resynchronising reader would step over.
    const DAMAGED_STRING: &str = "/Note Zjunk /Contents 9 0 R more) /Rotate [0 >]";

    /// C: B as door 2 would read it — the entry whose value will not parse dropped, and the
    /// reading resumed where a key belongs.
    ///
    /// That is `doc/todo/03` section 34's own sketch of the door: skip to the next name after an
    /// unreadable value. `/Note` loses its value and therefore itself; `Zjunk` is the token
    /// §7.2.3 makes whole and §7.3 gives nothing to begin. Not a file anybody would write — it is
    /// the *reading*, expressed in the only way this tree can express it without building the
    /// door.
    const RESYNCHRONISED: &str = "/Contents 9 0 R more) /Rotate [0 >]";

    /// The keys of object 2's prefix, and whether a page came back.
    fn prefix_of(body: &str) -> (Vec<String>, bool) {
        let bytes = document("2 0 R", body);
        let opened = Document::open(bytes).expect("the fixture opens");
        let keys = opened
            .damaged_dictionary(pdf_syntax::ObjectId::new(2, 0))
            .map(|damaged| {
                damaged
                    .entries
                    .iter()
                    .map(|(key, _)| String::from_utf8_lossy(key.as_bytes()).into_owned())
                    .collect()
            })
            .unwrap_or_default();
        (keys, pdf_model::Pages::new(&opened).get(0).is_some())
    }

    /// The control: the string is intact, so its contents are a value and not entries.
    #[test]
    fn a_name_inside_a_string_is_not_an_entry() {
        let (keys, page) = prefix_of(INTACT_STRING);
        assert_eq!(
            keys,
            ["Note"],
            "§7.3.4.2's string is one value, whatever is inside it"
        );
        assert!(
            !page,
            "no entry only a page object carries, so ADR 0786's door declines"
        );
    }

    /// The finding's first half: one byte destroys the `(`, and this reader stops where it should.
    #[test]
    fn a_value_that_begins_no_object_stops_the_reading() {
        let (keys, page) = prefix_of(DAMAGED_STRING);
        assert!(
            keys.is_empty(),
            "the reading stops at /Note's value, so the prefix is empty: {keys:?}"
        );
        assert!(
            !page,
            "and the object is refused, with §7.7.3.2's /Count standing (ADR 0782)"
        );
    }

    /// The finding's second half, and the reason door 2 is closed.
    ///
    /// This is B as a resynchronising reader would read it, and the prefix it yields holds
    /// `/Contents` — one of `PAGE_ONLY_ENTRIES`. ADR 0786's door then fires and object 2 becomes a
    /// page whose content stream is object 9, on the strength of bytes the producer wrote inside a
    /// string. That is the substitutive direction trap 5 forbids, reached through the
    /// *discriminator* rather than through noise the recovery tolerates. **If this test ever fails
    /// because the arm above started behaving like this one, door 2 was built without its
    /// argument.** ADR 0787.
    #[test]
    fn a_reading_across_a_gap_manufactures_the_entry_the_recovery_acts_on() {
        let (keys, page) = prefix_of(RESYNCHRONISED);
        assert!(
            keys.contains(&"Contents".to_owned()),
            "the gap turns string content into a Table 31 page-only entry: {keys:?}"
        );
        assert!(page, "and that entry is what makes the object a page");

        let (refused, _) = prefix_of(DAMAGED_STRING);
        assert_ne!(
            refused, keys,
            "the two differ by one skipped token, and that is the whole of door 2"
        );
    }
}
