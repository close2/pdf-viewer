//! §7.3.7's dictionary again, where the object that stops part-way is a Type 3 `/CharProcs`.
//!
//! ADR 0784 built one door for a dictionary the file states only in part — `Document::get` still
//! refuses the object outright, and `Document::damaged_dictionary` answers the entries that were
//! whole to a caller that asks for them **by name** — and left exactly one consumer through it:
//! `Pages`' recovery scan. This file is the second consumer, and the reason it may come through
//! is a sentence of §9.6.4 rather than a relaxation of ADR 0784.
//!
//! # Why this consumer, and not the one round 896 refused
//!
//! §7.3.7 makes the entries "unordered even though an arbitrary order may be imposed upon them
//! when written in a file", so a prefix is a **subset of the producer's own entries** and never
//! "the dictionary". What decides whether a subset may be used is what the *consumer's* clause
//! does with the entries that are missing, and §9.6.4's step b) states it outright:
//!
//! > If the name is not present as a key in CharProcs , no glyph shall be painted.
//!
//! So the residue is an **omission the standard defines**, not a default this reader chose and
//! not a mark standing in place of the producer's. That is the difference from ADR 0836's
//! damaged *font program*, which is still refused: there §9.6.5.4's closing permission lets a
//! processor "supply a mapping of its choosing", so an admitted program draws **other glyphs**
//! where the producer's were — ADR 0106's substitutive failure. Here nothing is substituted, and
//! Table 110's `/Widths` lives in the font dictionary, which is whole, so a glyph that paints
//! nothing still advances by what the producer stated.
//!
//! # Each test is a pair differing in one thing
//!
//! `damaged_page_dictionaries.rs`'s discipline. The sharpest of the pairs is the last one, and it
//! is the file where the guard and its reason come apart: **a prefix admits a key whose value the
//! damage reached**, so a name can be present and still have no description. §9.6.4 b) asks about
//! a *key* and Table 110 says the value "shall be a content stream", so the count that belongs in
//! the report is of descriptions rather than of keys.
//!
//! # The witness
//!
//! `corpus-cache/tika-issue-tracker/batch5/cairo/cairo-85141-0.zip-3.pdf`, whose object 76 is
//! `/F16`'s `/CharProcs` and whose bytes stop mid-entry at `/a112 57` under another stream's
//! data. Round 908 stopped the parser walking out of that object (ADR 0858) and left forty glyph
//! procedures behind this door; ADR 0866 is what takes them.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::{Pages, Unsupported, interpret};
use pdf_syntax::{Document, Object, ObjectId};

/// Assembles the one-page fixture, with `char_procs` written as object 6's whole *value*.
///
/// Object 6 is the only thing that changes between the arms below. The page shows `(ba)`, so the
/// glyph the damage costs is drawn *first* and the one that survives is drawn after it — which is
/// how a lost advance would show up as a mark in the wrong place rather than as a missing mark.
///
/// No cross-reference table: `Document::open` rebuilds by scanning for object headers, which
/// keeps hand-written offsets out of a file whose object lengths change with every arm.
fn document(char_procs: &str) -> Vec<u8> {
    let square = "1000 0 0 0 1000 1000 d1\n0 0 500 500 re f\n";
    let triangle = "1000 0 0 0 1000 1000 d1\n0 0 400 400 re f\n";
    let content = "BT /F1 10 Tf 10 10 Td (ba) Tj ET\n";
    let mut out = String::from("%PDF-1.7\n");
    let _ = write!(
        out,
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 6 0 R \
         /Encoding << /Type /Encoding /Differences [97 /square /triangle] >> \
         /FirstChar 97 /LastChar 98 /Widths [1000 1000] >>\nendobj\n\
         6 0 obj\n{char_procs}\nendobj\n\
         7 0 obj\n<< /Length {} >>\nstream\n{square}endstream\nendobj\n\
         8 0 obj\n<< /Length {} >>\nstream\n{triangle}endstream\nendobj\n\
         trailer\n<< /Root 1 0 R /Size 9 >>\n%%EOF\n",
        content.len(),
        square.len(),
        triangle.len(),
    );
    out.into_bytes()
}

/// Both descriptions stated, which is what a conforming producer writes.
const WHOLE: &str = "<< /square 7 0 R /triangle 8 0 R >>";

/// The same object with the second entry's value cut off by the end of the object.
///
/// `/triangle`'s value reads as the integer `8` — the `0 R` that would have made it a reference
/// is not there — and the next token where a key belongs is `endobj`, which §7.3.10 gives a
/// structural meaning, so round 908's arm stops the body there (ADR 0858). The prefix therefore
/// holds **two** entries and **one** description.
const DAMAGED: &str = "<< /square 7 0 R /triangle 8";

/// A whole dictionary stating only the entry the damaged one states readably.
///
/// The arm §9.6.4 b) predicts the damaged one draws identically to: "[i]f the name is not present
/// as a key in `CharProcs`, no glyph shall be painted", and a key whose value is not a content
/// stream paints nothing for Table 110's reason.
const ONLY_SQUARE: &str = "<< /square 7 0 R >>";

/// Every `Unsupported::Font` detail a page raises.
fn font_reports(bytes: Vec<u8>) -> Vec<String> {
    let document = Document::open(bytes).expect("the fixture opens");
    let page = Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    interpret(&document, &page)
        .unsupported
        .into_iter()
        .filter_map(|item| match item {
            Unsupported::Font { detail } => Some(detail),
            _ => None,
        })
        .collect()
}

/// What the page draws, as a string, so that two arms can be compared mark for mark.
fn drawing(bytes: Vec<u8>) -> String {
    let document = Document::open(bytes).expect("the fixture opens");
    let page = Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    format!("{:?}", interpret(&document, &page).display_list.commands())
}

/// The control: an intact `/CharProcs` says nothing and draws both glyphs.
#[test]
fn an_intact_char_procs_reports_nothing() {
    assert_eq!(
        font_reports(document(WHOLE)),
        Vec::<String>::new(),
        "a font whose /CharProcs object parses has nothing to report"
    );
    assert_ne!(
        drawing(document(WHOLE)),
        drawing(document(ONLY_SQUARE)),
        "and the two glyphs are two different drawings, so the comparison below discriminates"
    );
}

/// The prefix is taken, the surviving description draws, and the report names what it lost.
///
/// The counts are the whole of "naming what it lost": Table 110's `/Encoding` cell and §9.6.5.3
/// make `/Differences` "the complete character encoding for this font", and it is in the font
/// dictionary, which is whole — so the glyph names with no description are a list rather than an
/// estimate.
#[test]
fn a_damaged_char_procs_draws_what_it_states_and_names_what_it_lost() {
    let reports = font_reports(document(DAMAGED));
    assert_eq!(
        reports.len(),
        1,
        "one report, and it is the font's: {reports:?}"
    );
    let report = reports.first().expect("one report");
    assert!(
        report
            .starts_with("font /F1's /CharProcs states 2 entr(ies) and then stops being readable"),
        "the prefix's own size, not the producer's: {report}"
    );
    assert!(
        report.contains("1 of the 2 glyph names its /Encoding states have a description here and 1 do not and paint nothing"),
        "counted in descriptions rather than in keys — /triangle is a key whose value the damage \
         reached, so it is present and undescribed: {report}"
    );
    assert!(
        report.contains("§9.6.4 step b)"),
        "and the clause that decides the residue is named in it: {report}"
    );
}

/// §9.6.4 b) made checkable: the prefix draws exactly what a whole dictionary of it draws.
///
/// This is the decision's whole content. If the two differ, something the damage took is being
/// drawn as *something else* — which is what ADR 0836 refuses a damaged font program for — and if
/// they agree, the residue is the clause's own "no glyph shall be painted" and nothing more.
#[test]
fn the_prefix_draws_what_a_whole_dictionary_of_the_same_entries_draws() {
    assert_eq!(
        drawing(document(DAMAGED)),
        drawing(document(ONLY_SQUARE)),
        "the surviving glyph is in the producer's place, and the lost one paints nothing — \
         Table 110's /Widths is in the font dictionary, so the advance it did not lose is what \
         keeps the second mark where it was"
    );
}

/// The additive claim: the ordinary door still answers §7.3.10's null.
///
/// ADR 0784's rule, unchanged by this consumer — no reference anywhere in the document graph
/// resolves to more of the file than it used to, and the prefix exists only for a caller that
/// asked for it by name.
#[test]
fn the_damaged_object_is_still_null_through_the_ordinary_door() {
    let document = Document::open(document(DAMAGED)).expect("the fixture opens");
    let id = ObjectId::new(6, 0);
    assert_eq!(
        document.get(id),
        Object::Null,
        "Document::get is what every other reader of this object sees"
    );
    let damaged = document
        .damaged_dictionary(id)
        .expect("and the door by name answers the prefix");
    assert_eq!(damaged.entries.len(), 2, "two entries were whole");
}

/// The guard: an object that resolves to something readable is never replaced by a prefix.
///
/// The prefix is a second answer to a caller *already refused*, so the refusal has to have
/// happened. Here `/CharProcs` names an object that parses — to an array, which is not a
/// dictionary — and Table 110's entry is therefore absent rather than damaged.
#[test]
fn a_char_procs_that_resolves_to_something_readable_is_not_replaced_by_a_prefix() {
    let bytes = String::from_utf8(document(WHOLE))
        .expect("the fixture is text")
        .replace(WHOLE, "[]")
        .into_bytes();
    assert_eq!(
        font_reports(bytes),
        vec!["font /F1 is a Type 3 font with no /CharProcs dictionary".to_owned()],
        "Table 110's entry is not a dictionary, and no prefix stands in for it"
    );
}
