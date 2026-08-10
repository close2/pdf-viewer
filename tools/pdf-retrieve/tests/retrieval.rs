//! What a program gets when it asks for a page, a section, and the annotations on either.
//!
//! Two populations, and each answers a question the other cannot.
//!
//! **A synthetic document** pins the joins: which text belongs to a section, which annotation
//! belongs to that text, and what changes when a caller asks for annotations and when it does
//! not. Trap 8's rule — a corpus finds what documents contain — applies with force here, because
//! the case that matters is an annotation *just past* a section's end, and no real file was
//! written to have one.
//!
//! **Two committed documents** pin the two claims this crate makes about the outside world: that
//! its default answer is the readback `pdf-model/tests/text_extraction.rs` measures against
//! `pdftotext` and not a rearrangement of it, and that §9.6.5.4 of ISO 32000-2 comes back as
//! §9.6.5.4 — the demonstration `doc/todo/36` asked for, run as a test so that it stays true.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pdf_retrieve::{Retrieval, Wanted};

/// A committed document, which every checkout has once `doc/specifications.zip` is unpacked.
fn committed(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

/// A three-object fixture written to a temporary file, because [`Retrieval::open`] takes a path.
fn fixture() -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("pdf-retrieve-fixture-{}.pdf", std::process::id()));
    std::fs::write(&path, two_sections()).expect("the temporary directory is writable");
    path
}

/// A two-page document with two outline items, one annotation over each section's own text and
/// one attached to a point.
///
/// The layout is what the assertions rest on, so it is stated here rather than inferred: every
/// glyph is half an em wide at 12 units, so a character is 6 units, and each line's baseline is
/// 20 below the last. Page one shows `1 Alpha` then `alpha body one`; page two shows
/// `alpha body two`, then the next section's heading `2 Beta`, then `beta body`. So §1's text
/// runs from the top of page one to the middle of page two, and the highlight over `beta` is
/// four words past its end — which is the case the section join exists to get right.
fn two_sections() -> Vec<u8> {
    let first = "BT /F1 12 Tf 20 260 Td (1 Alpha) Tj 0 -20 Td (alpha body one) Tj ET";
    let second = "BT /F1 12 Tf 20 260 Td (alpha body two) Tj 0 -20 Td (2 Beta) Tj \
                  0 -20 Td (beta body) Tj ET";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Outlines 10 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
         /Resources << /Font << /F1 6 0 R >> >> /Contents 5 0 R /Annots [7 0 R 9 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
         /Resources << /Font << /F1 6 0 R >> >> /Contents 8 0 R /Annots [11 0 R] >>\nendobj\n\
         5 0 obj\n<< /Length {} >>\nstream\n{first}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 32 \
         /LastChar 122 /Widths {} /FontDescriptor 14 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /Annot /Subtype /Highlight /Rect [18 236 52 252] \
         /QuadPoints [18 252 52 252 52 236 18 236] /T (a reader) /Subj (over alpha) \
         /Contents (the first word of the body) >>\nendobj\n\
         8 0 obj\n<< /Length {} >>\nstream\n{second}\nendstream\nendobj\n\
         9 0 obj\n<< /Type /Annot /Subtype /Text /Rect [200 200 220 220] \
         /Contents (a note stuck to a point) >>\nendobj\n\
         10 0 obj\n<< /Type /Outlines /First 12 0 R /Last 13 0 R /Count 2 >>\nendobj\n\
         11 0 obj\n<< /Type /Annot /Subtype /Highlight /Rect [18 216 46 232] \
         /QuadPoints [18 232 46 232 46 216 18 216] /Subj (over beta) \
         /Contents (a word of the next section) >>\nendobj\n\
         12 0 obj\n<< /Title (1 Alpha) /Parent 10 0 R /Next 13 0 R /Dest [3 0 R /Fit] >>\nendobj\n\
         13 0 obj\n<< /Title (2 Beta) /Parent 10 0 R /Prev 12 0 R /Dest [4 0 R /Fit] >>\nendobj\n\
         14 0 obj\n<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 \
         /FontBBox [0 -200 1000 900] /ItalicAngle 0 /StemV 80 /Ascent 718 /Descent -207 \
         >>\nendobj\n",
        first.len().saturating_add(1),
        widths(),
        second.len().saturating_add(1),
    );
    assemble(&body)
}

/// A `/Widths` covering codes 32 to 122, every glyph half an em wide.
fn widths() -> String {
    let mut out = String::from("[");
    for _ in 32..=122 {
        out.push_str("500 ");
    }
    out.push(']');
    out
}

/// Wraps a body of numbered objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str) -> Vec<u8> {
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
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A section is its own text and stops where the next one starts.
#[test]
fn a_section_runs_from_its_heading_to_the_next() {
    let path = fixture();
    let retrieval = Retrieval::open(&path).expect("the fixture is a valid PDF");
    assert_eq!(
        retrieval.sections().len(),
        2,
        "two outline items, both placed"
    );

    let alpha = retrieval
        .section("1", &Wanted::default())
        .expect("the first item is addressed by its number");
    assert_eq!(
        alpha.pages,
        vec![0, 1],
        "it begins on one page and ends on the next"
    );
    assert_eq!(alpha.section.ends_at.as_deref(), Some("2 Beta"));
    assert!(
        alpha.trimmed_start && alpha.trimmed_end,
        "both headings were found"
    );
    assert!(
        alpha.text.starts_with("1 Alpha"),
        "it starts at its own heading: {:?}",
        alpha.text
    );
    assert!(
        alpha.text.contains("alpha body two"),
        "it carries the part of it on the second page: {:?}",
        alpha.text
    );
    assert!(
        !alpha.text.contains("beta body"),
        "and stops before the next section's body: {:?}",
        alpha.text
    );

    let beta = retrieval
        .section("2 Beta", &Wanted::default())
        .expect("the second item is addressed by its title");
    assert!(
        beta.text.starts_with("2 Beta") && beta.text.contains("beta body"),
        "{:?}",
        beta.text
    );
    assert!(!beta.trimmed_end, "nothing follows it, so nothing ended it");
    let _ = std::fs::remove_file(&path);
}

/// The same section with and without its annotations: the text does not move, and the
/// annotations that arrive are the ones over the section's own words.
///
/// The `beta` highlight is the assertion that matters. It is on a page the section *touches* —
/// page two is in the range because the section ends there — and it covers text past the end, so
/// a join that filtered by page would return it and a join that filters by where the covered
/// text sits does not.
#[test]
fn asking_for_annotations_changes_what_is_attached_and_not_what_is_read() {
    let path = fixture();
    let retrieval = Retrieval::open(&path).expect("the fixture is a valid PDF");
    let bare = retrieval.section("1", &Wanted::default()).expect("§1");
    let with = retrieval
        .section(
            "1",
            &Wanted {
                annotations: true,
                ..Wanted::default()
            },
        )
        .expect("§1");

    assert!(bare.annotations.is_empty(), "none were asked for");
    assert_eq!(
        bare.text, with.text,
        "asking for annotations does not move the text"
    );

    let subjects: Vec<Option<&str>> = with
        .annotations
        .iter()
        .map(|note| note.subject.as_deref())
        .collect();
    assert_eq!(
        subjects,
        vec![Some("over alpha"), None],
        "the highlight over its own text, and the note attached to a point on its first page — \
         and not the highlight over `beta`, which is four words past the end: {:?}",
        with.annotations
    );
    let highlight = with.annotations.first().expect("the highlight");
    assert_eq!(highlight.covers.as_deref(), Some("alpha"));
    assert_eq!(
        highlight.contents.as_deref(),
        Some("the first word of the body")
    );
    assert_eq!(highlight.title.as_deref(), Some("a reader"));

    // A filter by `/Subtype` narrows the same answer, which is how "the errata on this clause"
    // is asked of a document whose errata are `StrikeOut` and `Caret` marks.
    let only = retrieval
        .section(
            "1",
            &Wanted {
                subtypes: vec!["Text".to_owned()],
                ..Wanted::default()
            },
        )
        .expect("§1");
    assert_eq!(only.annotations.len(), 1);
    assert_eq!(
        only.annotations.first().map(|note| note.subtype.as_str()),
        Some("Text")
    );
    let _ = std::fs::remove_file(&path);
}

/// A page's default text is the readback and nothing else.
///
/// **This is the whole bootstrapping argument in one assertion.** `doc/todo/36` asks that this
/// project be able to trust its own extraction, and what makes that different from asserting it
/// is `pdf-model/tests/text_extraction.rs`, which compares the same string against `pdftotext`
/// over 974 documents at 99.2% and over the fourteen specification PDFs at 100% of its words. So
/// this crate's default answer has to *be* that string, byte for byte: a helpful tidy-up here
/// would put this tool between a caller and the only independent measurement of it.
#[test]
fn the_default_answer_is_the_string_the_text_gate_measures() {
    for name in ["PDF20_AN001-BPC.pdf", "PDF20_AN002-AF.pdf"] {
        let path = committed(name);
        let retrieval = Retrieval::open(&path)
            .unwrap_or_else(|error| panic!("{name} is a committed document: {error}"));
        let read = retrieval.page(0, &Wanted::default()).expect("page one");
        let document = retrieval.document();
        let pages = pdf_model::Pages::new(document);
        let page = pages.get(0).expect("page one");
        let interpretation = pdf_model::interpret(document, &page);
        assert!(!interpretation.text.is_empty(), "{name} shows text");
        assert_eq!(
            read.text, interpretation.text,
            "{name}: the tool's default page text is the interpreter's readback"
        );
        assert_eq!(read.order, pdf_retrieve::Order::Content);
    }
}

/// §9.6.5.4 of ISO 32000-2, retrieved by its number, and the errata attached to it.
///
/// The demonstration `doc/todo/36` asked for, pinned so that it stays true. Three facts about
/// the file rather than about its prose, so that nothing the licence covers is written down
/// here: which pages the clause occupies, that both of its edges were found, and how many words
/// are between them.
#[test]
fn a_clause_of_the_standard_is_addressed_by_its_number() {
    let path = committed("ISO_32000-2_sponsored_EC3.pdf");
    let retrieval = Retrieval::open(&path)
        .unwrap_or_else(|error| panic!("ISO 32000-2 is a committed document: {error}"));
    assert_eq!(retrieval.page_count(), 1023);
    assert_eq!(retrieval.sections().len(), 988, "one per outline item");

    let clause = retrieval
        .section(
            "9.6.5.4",
            &Wanted {
                subtypes: vec!["StrikeOut".to_owned(), "Caret".to_owned()],
                drop_artifacts: true,
                ..Wanted::default()
            },
        )
        .expect("§9.6.5.4 is an outline item");
    assert_eq!(clause.section.title, "9.6.5.4 Encodings for TrueType fonts");
    assert_eq!(clause.pages, vec![339, 340, 341]);
    assert_eq!(
        clause.section.ends_at.as_deref(),
        Some("9.7 Composite fonts")
    );
    assert!(clause.trimmed_start && clause.trimmed_end);
    assert!(clause.complete, "the three pages interpret whole");
    assert_eq!(clause.text.split_whitespace().count(), 1077);
    assert!(
        clause.annotations.is_empty(),
        "Errata Collection 3 strikes nothing out of this clause: {:?}",
        clause.annotations
    );

    // And one it *does* strike, which is the clause ADR 0253 found this tree implementing from
    // retired text: §12.5.2's closing sentence lists the entries a reader ignores when an
    // appearance dictionary is present, and `BM` is struck out of that list.
    let annotated = retrieval
        .section(
            "12.5.2",
            &Wanted {
                subtypes: vec!["StrikeOut".to_owned(), "Caret".to_owned()],
                drop_artifacts: true,
                ..Wanted::default()
            },
        )
        .expect("§12.5.2 is an outline item");
    assert_eq!(annotated.pages, vec![481, 482, 483, 484]);
    assert_eq!(annotated.annotations.len(), 23);
    assert!(
        annotated
            .annotations
            .iter()
            .any(|note| note.covers.as_deref() == Some("BM, ")),
        "the erratum that struck the blend mode out of §12.5.2's list is one call away"
    );

    // **And the same annotations whether or not the artifacts are dropped**, which is the
    // regression guard for the defect that had two coordinate systems sharing one set of
    // offsets: a `/QuadPoints` span indexes the raw readback, so a section whose text has had
    // §14.8.2.2's running heads taken out of it must not use those offsets to decide what is
    // inside it. It reported 23 one way and 24 the other before `Retrieval::section` assembled
    // the pages twice.
    let kept = retrieval
        .section(
            "12.5.2",
            &Wanted {
                subtypes: vec!["StrikeOut".to_owned(), "Caret".to_owned()],
                ..Wanted::default()
            },
        )
        .expect("§12.5.2 is an outline item");
    assert_eq!(kept.annotations, annotated.annotations);
    assert!(
        kept.text.len() > annotated.text.len(),
        "and the text itself is longer with the running heads in it"
    );
}
