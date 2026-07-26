//! Deterministic mutation testing, for the CI that cannot run a fuzzer.
//!
//! `cargo fuzz` needs nightly and unbounded time; this runs on stable in seconds. It is
//! not a substitute — a fuzzer explores far more — but it means every push exercises the
//! parser against corrupt input rather than only against the well-formed fixture.
//!
//! Deterministic on purpose. A random seed would make a failure unreproducible, which is
//! the one thing a robustness test must never be: the value of catching a crash is the
//! ability to fix it.

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
