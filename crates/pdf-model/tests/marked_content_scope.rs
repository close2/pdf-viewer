//! ISO 32000-2 §14.7.5.2's identifier is unique within *one content stream*, and not on a page.
//!
//! > The marked-content sequence shall contain a property list (see 14.6.2, "Property lists")
//! > containing an MCID entry, which shall be an integer marked-content identifier that uniquely
//! > identifies the marked-content sequence within its content stream
//!
//! and §14.7.5.2 permits a form `XObject`'s own stream to hold sequences of its own:
//!
//! > The content stream of a form XObject may contain one or more marked-content sequences that
//! > are associated with structure elements (see Example 5 in this subclause).
//!
//! while §14.7.5.4 gives that stream its own entry in the structural parent tree — "[t]he tree
//! shall contain an entry … for each content stream containing at least one marked-content
//! sequence that is a content item". Errata Collection 3's Issue #308 adds the NOTE that states
//! the consequence: identifiers are scoped by content stream and start at zero, so the same one
//! may reappear across pages or in a form.
//!
//! So a page whose `/Contents` and whose form both number from zero has **two different sequences
//! called `/MCID 0`**, and three things this crate answers are keyed on the identifier: the text
//! range a structure element covers (ADR 0134), §14.8.3.3's content rectangle (ADR 0486), and
//! §14.9's `/Alt` read from the element the sequence belongs to.
//!
//! The fixture is a pair (trap 8): **`collide`**, whose form reuses `/MCID 0`, and **`distinct`**,
//! whose form numbers from 1 and which is otherwise the same file. Everything asserted below holds
//! of both, which is what stops the test passing because the reader lost the form's content
//! altogether.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure, \
              and these offsets are within a fixture written in this file"
)]

use std::fmt::Write as _;

use pdf_model::content::ContentStream;
use pdf_model::structure::Tree;
use pdf_syntax::{Document, ObjectId};

/// The page object, the form `XObject` and the two structure elements the fixture states.
const PAGE: ObjectId = ObjectId {
    number: 3,
    generation: 0,
};
const FORM: ObjectId = ObjectId {
    number: 5,
    generation: 0,
};

/// How the fixture's structure element names the sequence its form drew.
#[derive(Clone, Copy)]
enum Names {
    /// Table 357's `/Stm`, which is what §14.7.5.2 asks a producer for.
    Stream,
    /// A bare integer, which §14.7.5.2 makes a statement that the sequence is in the page's own
    /// content stream — and which two of the corpus's tagged documents write anyway.
    AnInteger,
}

/// A one-page fixture whose `/Contents` and whose form `XObject` both mark with an `/MCID`.
///
/// The page's sequence is `/MCID 0` and draws `PAGE`; the form's is `form_mcid` and draws `FORM`.
/// Each stream states a `/StructParents` of its own (§14.7.5.4 Table 359) and the parent tree has
/// an entry for each, so the file is conforming under either numbering — which is the point: the
/// collision is legal, and a reader that flattens the two numberings is what is wrong.
///
/// The `Do` is deliberately **outside** any marked-content sequence, because §14.7.5.2 requires it
/// of a form that carries sequences of its own:
///
/// > any Do operator that paints the form XObject shall not be part of a logical structure content
/// > item
fn fixture(form_mcid: i64, form_entry: &str, names: Names) -> Vec<u8> {
    let page_content = "/P << /MCID 0 >> BDC\n\
         0 0 1 rg 10 10 20 20 re f\n\
         BT /F1 12 Tf 10 50 Td (PAGE) Tj ET\n\
         EMC\n\
         q 1 0 0 1 0 0 cm /Fm Do Q\n";
    let form_content = format!(
        "/P << /MCID {form_mcid} >> BDC\n\
         1 0 0 rg 120 10 40 40 re f\n\
         BT /F1 12 Tf 120 60 Td (FORM) Tj ET\n\
         EMC\n"
    );
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /XObject << /Fm 5 0 R >> /Font << /F1 10 0 R >> >> \
         /Contents 4 0 R /StructParents 0 >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page_content}endstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 200 100] \
         /Resources << /Font << /F1 10 0 R >> >> /StructParents 1 /Length {} >>\n\
         stream\n{form_content}endstream\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R 8 0 R] /ParentTree 9 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /Alt (the page's own) /K 0 >>\n\
         endobj\n\
         8 0 obj\n<< /Type /StructElem /S /Figure /P 6 0 R /Pg 3 0 R /Alt (the form's own) \
         /K {} >>\nendobj\n\
         9 0 obj\n<< /Nums [0 [7 0 R] 1 {form_entry}] >>\nendobj\n\
         10 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        page_content.len(),
        form_content.len(),
        match names {
            Names::Stream => {
                format!("<< /Type /MCR /Pg 3 0 R /Stm 5 0 R /MCID {form_mcid} >>")
            }
            Names::AnInteger => format!("{form_mcid}"),
        },
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        let number: usize = object
            .split_whitespace()
            .next()
            .and_then(|word| word.parse().ok())
            .expect("every object states its number");
        offsets.insert(number, cursor);
        cursor += object.len();
    }
    out.push_str(&body);
    let xref_at = out.len();
    let size = offsets.keys().copied().max().unwrap_or(0) + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for number in 1..size {
        match offsets.get(&number) {
            Some(offset) => {
                let _ = writeln!(out, "{offset:010} 00000 n ");
            }
            None => {
                let _ = writeln!(out, "0000000000 65535 f ");
            }
        }
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The file whose form reuses the page's `/MCID 0`, which §14.7.5.2 permits.
fn collide() -> Vec<u8> {
    fixture(0, "[8 0 R]", Names::Stream)
}

/// The same file with the form numbering from 1 — the twin that cannot collide.
///
/// The form's own parent tree entry is `[null 8 0 R]`, because §14.7.5.4 makes the identifier "a
/// zero-based index into the array" of *that stream's* entry, and Issue #308 adds the array may
/// hold null "for unused marked content identifiers (MCIDs) or those that do not have a structural
/// parent".
fn distinct() -> Vec<u8> {
    fixture(1, "[null 8 0 R]", Names::Stream)
}

/// Interprets the fixture's one page.
fn read(bytes: Vec<u8>) -> (Document, pdf_model::Interpretation) {
    let document = Document::open(bytes).expect("the fixture is a valid file");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has one page");
    let interpreted = pdf_model::interpret(&document, &page);
    (document, interpreted)
}

/// Each sequence is recorded against the stream it was read out of, not against the page.
#[test]
fn a_span_carries_the_content_stream_its_identifier_is_unique_within() {
    for (name, bytes, form_mcid) in [("collide", collide(), 0), ("distinct", distinct(), 1)] {
        let (_, interpreted) = read(bytes);
        let streams: Vec<(i64, ContentStream)> = interpreted
            .marked
            .iter()
            .map(|span| (span.mcid, span.stream))
            .collect();
        assert_eq!(
            streams,
            vec![
                (0, ContentStream::Page),
                (form_mcid, ContentStream::Object(FORM)),
            ],
            "{name}: the page's sequence and the form's, each against its own stream"
        );
    }
}

/// §14.8.2.5.1's logical text takes each element's own sequence and no other's.
///
/// The failure this pins is exact: keyed on the identifier alone, the element holding `/K 0`
/// matches *both* spans of `collide` and so does the one holding `/MCID 0` through a `/Stm`, so the
/// page reads back as its own text and the form's, twice over.
#[test]
fn logical_text_does_not_take_another_stream_s_sequence() {
    for (name, bytes) in [("collide", collide()), ("distinct", distinct())] {
        let (document, interpreted) = read(bytes);
        let tree = Tree::of(&document).expect("the fixture states a structure tree");
        let logical = tree
            .logical_text(&document, PAGE, &interpreted)
            .expect("the walk is not truncated");
        // The newline is §14.8.2.6.2's inferred separator, placed when the form's first glyph
        // showed and therefore inside the form's sequence rather than between the two.
        assert_eq!(
            logical, "PAGE\nFORM",
            "{name}: the page's element then the form's, each once"
        );
    }
}

/// §14.8.3.3's content rectangle is the *element's own* sequence's marks and no other's.
///
/// The page's sequence fills `10 10 20 20` and shows text at `10 50`; the form's fills
/// `120 10 40 40` and shows text at `120 60`. So the two rectangles are disjoint in x, and an
/// element given both would be visibly wider than the marks it names.
#[test]
fn a_content_rectangle_stops_at_its_own_stream() {
    for (name, bytes) in [("collide", collide()), ("distinct", distinct())] {
        let (_, interpreted) = read(bytes);
        let page = interpreted
            .marked
            .iter()
            .find(|span| span.stream == ContentStream::Page)
            .and_then(|span| span.drawn)
            .expect("the page's sequence marked the page");
        let form = interpreted
            .marked
            .iter()
            .find(|span| span.stream == ContentStream::Object(FORM))
            .and_then(|span| span.drawn)
            .expect("the form's sequence marked the page");
        assert!(
            page[2] < form[0],
            "{name}: the two sequences drew in different places: {page:?} against {form:?}"
        );
    }
}

/// §14.9.3's `/Alt` comes from the element *this stream's* parent tree names.
///
/// §14.7.5.4 keys the tree by the stream's own `/StructParents`, so the form's `/MCID` indexes the
/// form's array. Read with the page's array instead, `collide`'s form sequence would be spoken as
/// the page's paragraph — the two descriptions in the fixture are chosen so that the mistake has a
/// name rather than a shape.
#[test]
fn an_alternate_description_comes_from_the_stream_s_own_parent_tree() {
    for (name, bytes) in [("collide", collide()), ("distinct", distinct())] {
        let (_, interpreted) = read(bytes);
        let spoken: Vec<Option<&str>> = interpreted
            .described
            .iter()
            .map(|described| described.alt.as_deref())
            .collect();
        assert_eq!(
            spoken,
            vec![Some("the page's own"), Some("the form's own")],
            "{name}: each sequence takes the description of the element that owns it"
        );
    }
}

/// §14.8.2.5's rearrangement of a *selection* keys on the same two halves.
///
/// The whole readback is asked for, so the answer must be a rearrangement of exactly those bytes:
/// a span taken from the wrong stream would either duplicate text or leave a byte uncovered, and
/// [`Tree::logical_range`] answers `None` rather than a partial copy in the second case.
#[test]
fn a_logical_selection_covers_each_byte_once() {
    for (name, bytes) in [("collide", collide()), ("distinct", distinct())] {
        let (document, interpreted) = read(bytes);
        let tree = Tree::of(&document).expect("the fixture states a structure tree");
        let whole = tree.logical_range(
            &document,
            PAGE,
            &interpreted.text,
            &interpreted.marked,
            0..interpreted.text.len(),
        );
        assert_eq!(
            whole.as_deref(),
            Some("PAGE\nFORM"),
            "{name}: every byte of the readback, once, in the structure's order"
        );
    }
}

/// A bare integer naming a sequence that is *not* in the page's own stream: the one recovery.
///
/// §14.7.5.2 makes an absent `/Stm` a statement — "[i]f this entry is absent, the marked-content
/// sequence shall be contained in the content stream of the page identified by Pg" — so a file
/// whose sequence is in a form and whose `/K` is an integer has broken a `shall`. Two of the
/// corpus's 153 tagged documents do, and read strictly they say nothing to a screen reader at all.
///
/// [`pdf_model::content::named_sequences`] answers them where the page's own stream holds no such
/// identifier and exactly one other stream does. The two halves of that condition are what the two
/// cases below are: the form numbers from 1, which the page's stream never used, and then from 0,
/// which it did — and there the clause's `shall` decides outright and the recovery does not fire.
#[test]
fn an_integer_naming_no_sequence_of_the_page_is_read_where_the_file_meant_it() {
    let recovered = fixture(1, "[null 8 0 R]", Names::AnInteger);
    let (document, interpreted) = read(recovered);
    let tree = Tree::of(&document).expect("the fixture states a structure tree");
    assert_eq!(
        tree.logical_text(&document, PAGE, &interpreted).as_deref(),
        Some("PAGE\nFORM"),
        "the identifier exists in exactly one stream on this page, and that is what it names"
    );

    // The page's own stream *does* carry `/MCID 0`, so there is nothing to recover: the clause
    // says where an absent `/Stm` puts the sequence, and it is the page's.
    let taken = fixture(0, "[8 0 R]", Names::AnInteger);
    let (document, interpreted) = read(taken);
    let tree = Tree::of(&document).expect("the fixture states a structure tree");
    assert_eq!(
        tree.logical_text(&document, PAGE, &interpreted).as_deref(),
        Some("PAGEPAGE"),
        "both elements name the page's own sequence, because that is what the clause says an \
         absent /Stm means — and neither is given the form's as well"
    );
}
