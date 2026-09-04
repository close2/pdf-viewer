//! Deterministic mutation testing, for the CI that cannot run a fuzzer.
//!
//! `cargo fuzz` needs nightly and unbounded time; this runs on stable in seconds. It is
//! not a substitute — a fuzzer explores far more — but it means every push exercises the
//! parser against corrupt input rather than only against the well-formed fixture.
//!
//! Deterministic on purpose. A random seed would make a failure unreproducible, which is
//! the one thing a robustness test must never be: the value of catching a crash is the
//! ability to fix it.

use std::fmt::Write as _;

use pdf_syntax::{Document, Limits, Parser};

/// A small xorshift generator.
///
/// Written out rather than pulled in as a dependency: the requirement is reproducible
/// bytes, and adding a crate to the tree for eight lines of arithmetic would not be worth
/// the supply-chain surface.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        let bound = u64::try_from(bound).unwrap_or(u64::MAX);
        usize::try_from(self.next().checked_rem(bound).unwrap_or(0)).unwrap_or(0)
    }
}

/// Bounds tight enough that the limit paths are reached by small inputs.
fn limits() -> Limits {
    Limits {
        max_depth: 32,
        max_array_len: 1024,
        max_dict_len: 256,
        max_string_len: 1 << 16,
        max_stream_len: 1 << 20,
    }
}

/// Single-byte corruptions of a valid file must never panic or hang.
///
/// This is the corruption a real file suffers in transit, and the case where a parser is
/// most likely to trust something it should have checked.
#[test]
fn single_byte_corruptions_are_survived() {
    let original = test_scenes::basic_pdf();
    let mut rng = Rng(0x5eed_1234_abcd_0001);

    for _ in 0..4000 {
        let mut bytes = original.clone();
        let at = rng.below(bytes.len());
        let replacement = u8::try_from(rng.next() & 0xff).unwrap_or(0);
        if let Some(slot) = bytes.get_mut(at) {
            *slot = replacement;
        }

        // Both outcomes are fine. Panicking, hanging or aborting are not.
        if let Ok(document) = Document::open_with_limits(bytes, limits())
            && let Ok(catalog) = document.catalog()
            && let Some(pages) = document.get_key(&catalog, "Pages").as_dict()
        {
            let _ = document.get_key(pages, "Count");
        }
    }
}

/// Truncation at every possible point.
#[test]
fn every_truncation_is_survived() {
    let original = test_scenes::basic_pdf();
    for length in 0..=original.len() {
        let bytes = original.get(..length).unwrap_or_default().to_vec();
        if let Ok(document) = Document::open_with_limits(bytes, limits()) {
            let _ = document.catalog();
        }
    }
}

/// Structures designed to exhaust the stack or the heap must hit a limit instead.
///
/// Rust does not protect against these — that is the whole reason `Limits` exists — so each
/// is checked to fail as a *bounded error* rather than by running out of something.
#[test]
fn adversarial_structures_hit_their_limits() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("deeply nested arrays", b"[".repeat(10_000)),
        ("deeply nested dictionaries", b"<</A ".repeat(10_000)),
        ("unterminated array", b"[1 2 3".to_vec()),
        ("unterminated dictionary", b"<</A 1".to_vec()),
        ("array of many items", {
            let mut input = b"[".to_vec();
            for _ in 0..5000 {
                input.extend_from_slice(b"1 ");
            }
            input
        }),
    ];

    for (label, input) in cases {
        let mut parser = Parser::with_limits(&input, limits());
        // The requirement is that it returns at all, with a bounded error rather than a
        // stack overflow or an allocation failure.
        let result = parser.parse_object();
        assert!(
            result.is_err(),
            "{label} should be refused by a limit, but parsed successfully"
        );
    }
}

/// A self-referential object graph must not loop.
#[test]
fn a_reference_cycle_resolves_to_null_rather_than_looping() {
    // Object 1 points at 2, and 2 points back at 1.
    let body = b"%PDF-1.7\n\
                 1 0 obj\n2 0 R\nendobj\n\
                 2 0 obj\n1 0 R\nendobj\n\
                 3 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
                 trailer\n<< /Root 3 0 R >>\n";

    let document =
        Document::open_with_limits(body.to_vec(), limits()).expect("the file is openable");
    let catalog = document.catalog().expect("the catalogue is reachable");

    // The cycle must terminate. Null is the right answer: the reference resolves to
    // nothing, which is what the specification says a broken reference is worth.
    let pages = document.get_key(&catalog, "Pages");
    assert!(
        pages.is_null(),
        "a cycle should resolve to null, got {}",
        pages.type_name()
    );
}

/// A stream declaring a length far beyond the file must not be trusted.
#[test]
fn a_lying_stream_length_is_recovered_from() {
    let body = b"%PDF-1.7\n\
                 1 0 obj\n<< /Length 999999 >>\nstream\nHELLO\nendstream\nendobj\n\
                 trailer\n<< /Root 1 0 R >>\n";

    let mut parser = Parser::with_limits(body, limits());
    parser.seek(9);
    let (_, object) = parser
        .parse_indirect_object()
        .expect("the object is parseable");
    let stream = object.as_stream().expect("it is a stream");

    // The declared length is impossible, so `endstream` decides the extent instead.
    assert_eq!(&*stream.data, b"HELLO", "the data should stop at endstream");
}

/// A subsection that promises more entries than it supplies must still find its trailer.
///
/// The last thing a classic `xref` table's entries run into is the `trailer` keyword. A
/// subsection header whose count overruns the entries present therefore reads that keyword
/// as an entry, and a reader that resumes from *after* the offending token has stepped over
/// the only thing that names `/Root`. The result is a document with no catalogue and no
/// pages, produced from a file whose every object is intact — which is exactly the kind of
/// failure that looks like a missing feature rather than a parsing bug.
///
/// Found in the pdf.js corpus: `outline_goto_action.pdf` declares twelve entries and writes
/// eleven. The fixture here reproduces it in twenty lines so the regression is pinned
/// without the submodule.
#[test]
fn a_subsection_counting_more_entries_than_it_has_still_finds_the_trailer() {
    let objects = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        "2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n",
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n",
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    // Five entries are declared -- the free entry and four objects -- but only the free
    // entry and three objects are written. The count is the lie under test, and what the
    // fifth entry runs into is the `trailer` keyword.
    out.push_str("xref\n0 5\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );

    let document = Document::open(out.into_bytes()).expect("the objects are all intact");
    let catalog = document
        .catalog()
        .expect("the trailer names /Root and must be found");
    assert!(
        document.get_key(&catalog, "Pages").as_dict().is_some(),
        "and the page tree must be reachable through it"
    );
}

/// ISO 32000-2 §7.5.2: every byte offset in a file is measured from its `%PDF-`.
///
/// > This provision allows for arbitrary bytes preceding the %PDF- without impacting the
/// > viability of the PDF file and its byte offsets.
///
/// So the file below is *valid*, not corrupt: its cross-reference table is exactly right and
/// its numbers are all short by the length of what precedes the header. The test is written
/// as a comparison rather than as an assertion about one file, because the property that
/// matters is that the junk changes nothing — and because a reader that ignores the rule
/// still opens this file by scanning for `N G obj` headers, which reads the whole file to
/// reach the same catalog and would make a plain "it opens" assertion pass either way.
/// `XrefTable::recovered_by_scan` is what separates the two, and it is what this asserts.
#[test]
fn junk_before_the_header_shifts_every_offset_in_the_file() {
    let build = |junk: &str| {
        let objects = [
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            "2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n",
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n",
        ];
        // Offsets are measured from the header, which is what the clause requires and what
        // a writer producing this file would have done.
        let mut body = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for object in objects {
            offsets.push(body.len());
            body.push_str(object);
        }
        let xref_at = body.len();
        body.push_str("xref\n0 4\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(body, "{offset:010} 00000 n ");
        }
        let _ = write!(
            body,
            "trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        format!("{junk}{body}").into_bytes()
    };

    for junk in ["", "Some junk before the header\n\n"] {
        let document = Document::open(build(junk)).expect("a valid file, junk or not");
        assert!(
            !document.was_recovered(),
            "with {} bytes of junk the table should still be read, not rebuilt",
            junk.len()
        );
        let catalog = document.catalog().expect("/Root");
        assert!(document.get_key(&catalog, "Pages").as_dict().is_some());
    }
}

/// A dictionary that stops in mid-entry must not be completed out of the next object.
///
/// ISO 32000-2 §7.3.10 gives `obj` and `endobj` their meaning — an indirect object's definition
/// is
///
/// > followed by the value of the object bracketed between the keywords obj and endobj
///
/// — and §7.3.8.1 gives `stream` and `endstream` theirs, a stream being a dictionary followed by
/// zero or more bytes
///
/// > bracketed between the keywords stream (followed by newline) and endstream
///
/// None of the four can stand where a key belongs, so meeting one inside a dictionary body is
/// proof that the `>>` being looked for is not in this object at all, and the reading stops there.
///
/// Found in `corpus-cache/tika-issue-tracker/batch5/cairo/cairo-85141-0.zip-3.pdf`, whose
/// object 76 is a Type 3 font's `/CharProcs` overwritten in mid-entry by another stream's
/// data. Reading on through `endstream endobj 78 0 obj <<` answered a **stream** built from
/// 76's forty surviving entries, 78's `/Length` and `/Filter`, and 78's data — an object no
/// producer wrote, handed back with no error at all, so the page drew forty glyphs of a font
/// this reader had assembled and reported success. ADR 0858. The fixture is that shape in
/// twenty lines, so the regression is pinned without the corpus.
#[test]
fn a_dictionary_body_stops_at_the_keyword_that_ends_its_object() {
    let mut out = String::from("%PDF-1.7\n");
    out.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    out.push_str("2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n");
    out.push_str("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n");
    // The damage: object 4's third entry has a key and no value, and then the object ends.
    let damaged_at = out.len();
    out.push_str("4 0 obj\n<< /Kept 1 /AlsoKept 2 /Lost\nendobj\n");
    out.push_str("5 0 obj\n<< /Length 3 /Filter /FlateDecode >>\nstream\nabc\nendstream\nendobj\n");
    let _ = write!(
        out,
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{damaged_at}\n%%EOF\n"
    );

    let document = Document::open(out.into_bytes()).expect("the catalogue is intact");
    let damaged = pdf_syntax::ObjectId {
        number: 4,
        generation: 0,
    };

    let object = document.get(damaged);
    assert!(
        matches!(object, pdf_syntax::Object::Null),
        "an object whose dictionary never closes is §7.3.10's null, not a splice: got {object:?}"
    );

    // The prefix is still available through the door that says what it is, and it stops at
    // the object's own end rather than in the middle of the next object's dictionary.
    let prefix = document
        .damaged_dictionary(damaged)
        .expect("the entries before the damage are the file's own");
    assert_eq!(
        prefix.entries.len(),
        2,
        "only the two whole entries, and neither of object 5's: {:?}",
        prefix.entries
    );
    assert!(
        prefix
            .entries
            .get_by_name(&pdf_syntax::Name::new(b"Length".to_vec()))
            .is_none(),
        "object 5's /Length must not appear under object 4's number"
    );
}
