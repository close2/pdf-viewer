//! What the file reaches, what nothing reaches, and the one exemption that turns on the answer.
//!
//! `doc/todo/62` section 4's fourth point asks for this suite in as many words, and gives the
//! reason: the corpus cannot rank an exemption that withdraws failures no `-fail-` document
//! depends on, so the reach is pinned by fixtures or it is pinned by nothing. Every document here
//! is built to exercise exactly one edge of the answer.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::fmt::Write as _;

use pdf_archive::reach::exemption_narrows;
use pdf_archive::{Clauses, Entry, Examination, Flavour, Level, Outcome, Target, check};
use pdf_syntax::{Document, Name, ObjectId};

/// Wraps a body of numbered objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str, trailer: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R {trailer} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A one-page document whose page states the given `/Resources` and content stream.
///
/// Object 4 is the content stream, so a fixture's own objects begin at 5.
fn page_with(resources: &str, content: &str, extra: &str) -> Document {
    let length = content.len();
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
         /Contents 4 0 R /Resources {resources} >>\nendobj\n\
         4 0 obj\n<< /Length {length} >>\nstream\n{content}\nendstream\nendobj\n{extra}"
    );
    Document::open(assemble(&body, "")).expect("the fixture is a valid PDF")
}

/// The identifier of every requirement a report says was not met.
fn failed(document: &Document, target: Target) -> Vec<&'static str> {
    check(document, target)
        .judgements
        .into_iter()
        .filter(|judgement| matches!(judgement.outcome, Outcome::Failed { .. }))
        .map(|judgement| judgement.id)
        .collect()
}

/// A dictionary key, as a path entry reads.
fn key(name: &str) -> Entry {
    Entry::Key(Name(name.as_bytes().into()))
}

#[test]
fn the_path_to_a_page_names_the_entries_it_was_reached_through() {
    let document = page_with("<< >>", "", "");
    let exam = Examination::new(&document, Target::Four(Flavour::Plain));
    let path = exam
        .reaches()
        .path(ObjectId::new(3, 0))
        .expect("the page is reached from the trailer");
    assert_eq!(
        path,
        vec![key("Root"), key("Pages"), key("Kids"), Entry::Index(0)],
        "ISO 32000-2 §7.5.5's trailer, §7.7.2's catalog, §7.7.3.2's node and its first kid"
    );
}

#[test]
fn an_object_no_reference_names_is_unreferenced() {
    let document = page_with("<< >>", "", "5 0 obj\n<< /Type /Metadata >>\nendobj\n");
    let exam = Examination::new(&document, Target::Four(Flavour::Plain));
    let unreferenced = exam.reaches().unreferenced();
    assert!(
        unreferenced.contains(&ObjectId::new(5, 0)),
        "the cross-reference table names object 5 and no object does"
    );
    assert_eq!(
        unreferenced.len(),
        1,
        "and everything else in the fixture is reached: {unreferenced:?}"
    );
    assert!(
        exam.reaches().reaches(ObjectId::new(4, 0)),
        "the page's own content stream among them"
    );
}

/// The exemption's premise: a category entry whose name the content stream never states.
const UNINVOKED_STATE: &str = "5 0 obj\n<< /Type /ExtGState /TR /Identity >>\nendobj\n";

#[test]
fn a_named_resource_nothing_invokes_exempts_what_only_it_reaches() {
    let document = page_with(
        "<< /ExtGState << /GS0 5 0 R >> >>",
        "0 0 200 200 re f",
        UNINVOKED_STATE,
    );
    let exam = Examination::new(&document, Target::Four(Flavour::Plain));
    let exempt = exam.exempt();
    assert!(
        exempt.holds(ObjectId::new(5, 0)),
        "nothing in the content stream states /GS0, so the only route to object 5 is that entry"
    );
    assert_eq!(exempt.entries(), 1, "one such entry, and one only");
    assert!(
        exempt.stopped_at().is_none(),
        "and no bound stopped either walk"
    );
}

#[test]
fn a_named_resource_the_stream_invokes_exempts_nothing() {
    let document = page_with(
        "<< /ExtGState << /GS0 5 0 R >> >>",
        "/GS0 gs 0 0 200 200 re f",
        UNINVOKED_STATE,
    );
    let exam = Examination::new(&document, Target::Four(Flavour::Plain));
    assert!(
        exam.exempt().is_empty(),
        "the associated content stream states /GS0, so the resource is used for rendering"
    );
}

#[test]
fn the_exemption_withdraws_a_failure_outside_the_carve_out() {
    let invoked = page_with(
        "<< /ExtGState << /GS0 5 0 R >> >>",
        "/GS0 gs 0 0 200 200 re f",
        UNINVOKED_STATE,
    );
    assert!(
        failed(&invoked, Target::Four(Flavour::Plain))
            .contains(&"graphics/no-transfer-function-in-a-graphics-state"),
        "section 6.2.5 forbids the TR key of a graphics state the page uses"
    );
    let uninvoked = page_with(
        "<< /ExtGState << /GS0 5 0 R >> >>",
        "0 0 200 200 re f",
        UNINVOKED_STATE,
    );
    assert!(
        !failed(&uninvoked, Target::Four(Flavour::Plain))
            .contains(&"graphics/no-transfer-function-in-a-graphics-state"),
        "and section 6.2.2's last sentence puts the same dictionary outside section 6.2.5's \
         population when nothing states its name, section 6.2.5 being outside both carve-outs"
    );
}

/// A stream ISO 19005 forbids the filter of, reached only by an uninvoked name.
const UNINVOKED_LZW: &str = "5 0 obj\n<< /Subtype /Form /Filter /LZWDecode /Length 0 /BBox \
                             [0 0 1 1] >>\nstream\n\nendstream\nendobj\n";

#[test]
fn the_carve_out_keeps_a_failure_the_exemption_would_otherwise_withdraw() {
    let document = page_with(
        "<< /XObject << /Fm0 5 0 R >> >>",
        "0 0 200 200 re f",
        UNINVOKED_LZW,
    );
    let exam = Examination::new(&document, Target::Four(Flavour::Plain));
    assert!(
        exam.exempt().holds(ObjectId::new(5, 0)),
        "the exemption's population reaches the form XObject nothing invokes"
    );
    assert!(
        failed(&document, Target::Four(Flavour::Plain)).contains(&"file-structure/no-lzw-filter"),
        "and ISO 19005-4 section 6.1.6.2 is inside the four object-syntax subclauses its own \
         section 6.2.2 takes back, so the failure stands"
    );
    assert!(
        failed(&document, Target::Two(Level::B)).contains(&"file-structure/no-lzw-filter"),
        "as ISO 19005-2 section 6.1.7.2 is inside TechNote 0010 A010's sections 6.1.2 to 6.1.13"
    );
}

#[test]
fn each_carve_out_is_the_range_its_own_part_states() {
    let two = Target::Two(Level::B);
    let four = Target::Four(Flavour::Plain);
    // A010's file-structure and implementation-limit subclauses, and part 4's four object-syntax
    // ones, are the clauses each part takes back from its own exemption.
    assert!(!exemption_narrows(Clauses::both("6.1.2", "6.1.2"), two));
    assert!(!exemption_narrows(Clauses::both("6.1.13", "6.1.13"), two));
    assert!(!exemption_narrows(Clauses::both("6.1.7.2", "6.1.6.2"), two));
    assert!(exemption_narrows(Clauses::both("6.2.5", "6.2.5"), two));
    assert!(exemption_narrows(Clauses::both("6.1.1", "6.1.1"), two));
    assert!(!exemption_narrows(Clauses::both("6.1.6", "6.1.6"), four));
    assert!(!exemption_narrows(Clauses::both("6.1.9", "6.1.9"), four));
    assert!(exemption_narrows(Clauses::both("6.1.13", "6.1.13"), four));
    assert!(exemption_narrows(Clauses::both("6.2.5", "6.2.5"), four));
    // Section 5.1's base standard is kept for both, which is A010's second sentence for part 2
    // and the direction that withdraws nothing for part 4.
    assert!(!exemption_narrows(Clauses::both("5.1", "5.1"), two));
    assert!(!exemption_narrows(Clauses::both("5.1", "5.1"), four));
    // A requirement the part does not state is not judged at all.
    assert!(!exemption_narrows(Clauses::only_four("6.2.5"), two));
}

/// An annex clause is inside the exemption, and that is the clause's answer rather than a parser's.
///
/// Both parts exempt an unreferenced named resource from every requirement of the **document**,
/// and a normative annex is part of the document, so nothing in Annex A or Annex B is carved back
/// out — only the file-structure ranges are. Three rows of the table cite an annex *with a
/// predicate*, and until session 1007 the answer for them was reached by a fall-through whose own
/// comment claimed the opposite; this pins it. `pdf_archive::withdrawal` carries the reading subclause by subclause.
#[test]
fn an_annex_clause_is_narrowed_like_any_other_requirement_of_the_document() {
    let two = Target::Two(Level::B);
    let four = Target::Four(Flavour::Plain);
    // ISO 19005-2 Annex B.1, what signing produces.
    assert!(exemption_narrows(Clauses::only_two("B.1"), two));
    // ISO 19005-4 Annex A.2, the embedded files a PDF/A-4f file carries, and Annex B.2.2, the
    // format of a 3D stream.
    assert!(exemption_narrows(Clauses::only_four("A.2"), four));
    assert!(exemption_narrows(Clauses::only_four("B.2.2"), four));
    // And the carve-outs are still the only thing that keeps a clause, whatever it is written in.
    assert!(!exemption_narrows(Clauses::only_four("6.1.6.1"), four));
}
