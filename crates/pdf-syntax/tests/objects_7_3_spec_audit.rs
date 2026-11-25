//! Specification audit of ISO 32000-2 §7.3's objects against the reader and the incremental
//! writer (review 2243, `doc/reviews/2243-specification-audit-7-1-7-3.md`).
//!
//! Each `*_spec_audit` test states a clause's requirement and fails on the tree it was written
//! against; each has a control beside it that passes, so the failure is the clause and not the
//! fixture.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use pdf_syntax::object::{Name, Object, ObjectId, Stream};
use pdf_syntax::write::incremental_update;
use pdf_syntax::{Document, Lexer, Parser, Token};

/// A one-page document whose object 4 is the content stream `stream`, written verbatim (its
/// dictionary, the keyword, the data and `endstream`), with an exact cross-reference table.
fn with_stream(stream: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R >>".to_owned(),
        stream.to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Object 4 of `bytes`, taken from the document with one entry added to its dictionary, written
/// back by §7.5.6's incremental update; answers the `/Length` the update states for it and the
/// number of bytes the update wrote between `stream` and `endstream`.
fn rewritten_length(bytes: Vec<u8>) -> (Option<i64>, usize) {
    let document = Document::open(bytes).expect("the fixture opens");
    let id = ObjectId::new(4, 0);
    let read = document.get(id);
    let stream = read.as_stream().expect("object 4 is a stream");
    let mut dict = stream.dict.clone();
    dict.insert(Name::new(&b"Marked"[..]), Object::Boolean(true));
    let replacement = Object::Stream(Arc::new(Stream {
        dict,
        data: Arc::clone(&stream.data),
        decryption_failed: false,
    }));
    let mut replacements = BTreeMap::new();
    replacements.insert(id, replacement);
    let written = incremental_update(&document, &replacements).expect("the update is written");

    // The update's own object 4: the last `4 0 obj` in the file, its data between the end of
    // line after `stream` and the end of line before `endstream` (the writer puts one LF each).
    let at = find_last(&written, b"4 0 obj").expect("the update writes object 4");
    let tail = written.get(at..).expect("in the file");
    let opened = find_first(tail, b"stream\n")
        .expect("a stream keyword")
        .saturating_add(b"stream\n".len());
    let closed = find_first(tail, b"\nendstream").expect("an endstream keyword");
    let mut parser = Parser::new(tail);
    let (_, object) = parser.parse_indirect_object().expect("the object reads");
    let stated = object
        .as_stream()
        .and_then(|stream| stream.dict.get("Length"))
        .and_then(Object::as_integer);
    (stated, closed.saturating_sub(opened))
}

fn find_first(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

/// ISO 32000-2 §7.3.8.2: "Every stream dictionary shall have a Length entry that indicates how
/// many bytes of the PDF file are used for the stream's data", and Table 5's `/Length`: "The number
/// of bytes from the beginning of the line following the keyword stream to the last byte just
/// before the keyword endstream". A stream whose file stated a `/Length` too small (the reader
/// found its data by `endstream`, and logged that) is written back by the incremental update with
/// the same stale `/Length` over the bytes it actually wrote. `serialize` and `linearize` re-derive
/// the entry from the bytes; `write::incremental_update` writes the dictionary as handed.
#[test]
fn an_incremental_update_states_the_length_of_the_bytes_it_writes_spec_audit() {
    let (stated, written) = rewritten_length(with_stream(
        "<< /Length 3 >>\nstream\n0 0 m 10 10 l S\nendstream",
    ));
    assert_eq!(
        stated,
        i64::try_from(written).ok(),
        "the update's /Length is the number of bytes it wrote between stream and endstream"
    );
}

/// The control: a stream whose `/Length` was right is written back with a `/Length` that is.
#[test]
fn an_incremental_update_keeps_a_length_that_was_right() {
    let (stated, written) = rewritten_length(with_stream(
        "<< /Length 15 >>\nstream\n0 0 m 10 10 l S\nendstream",
    ));
    assert_eq!(stated, Some(15));
    assert_eq!(written, 15);
}

/// ISO 32000-2 §7.3.8.1: "All streams shall be indirect objects (see 7.3.10, "Indirect objects")
/// and the stream dictionary shall be a direct object", and §7.3.7 / §7.3.6: "An indirect
/// reference (see 7.3.10, "Indirect objects") shall be used as the value to refer to a stream
/// object". A stream written *directly* as a dictionary's value is read as a stream value of that
/// dictionary, and nothing is logged: a repair of a malformed file with no class (ADR 3477).
#[test]
fn a_stream_written_as_a_direct_value_is_not_read_in_silence_spec_audit() {
    let source = b"<< /Inner << /Length 3 >>\nstream\nabc\nendstream >>";
    let mut parser = Parser::new(source);
    let object = parser.parse_object().expect("the dictionary reads");
    let inner = object
        .as_dict()
        .and_then(|dict| dict.get("Inner"))
        .cloned()
        .expect("an /Inner entry");
    assert!(
        !matches!(inner, Object::Stream(_)) || !parser.repairs().is_empty(),
        "a stream that is not an indirect object is either refused or logged as a repair; read \
         {inner:?} with repairs {:?}",
        parser.repairs()
    );
}

/// The control: the same stream as an indirect object reads with no repair.
#[test]
fn a_stream_written_as_an_indirect_object_reads_without_a_repair() {
    let source = b"7 0 obj\n<< /Length 3 >>\nstream\nabc\nendstream\nendobj\n";
    let mut parser = Parser::new(source);
    let (_, object) = parser.parse_indirect_object().expect("the object reads");
    assert!(matches!(object, Object::Stream(_)));
    assert!(parser.repairs().is_empty(), "{:?}", parser.repairs());
}

/// ISO 32000-2 §7.3.5: "a name object is an atomic symbol uniquely defined by a sequence of any
/// characters (8-bit values) except null (character code 0)". `/A#00B` writes a null into a name
/// through the number-sign escape; the lexer expands it into the name's bytes and logs nothing, so
/// a name §7.3.5 says cannot exist reaches every reader of the dictionary — and the writer, which
/// escapes it back out — as an ordinary one.
#[test]
fn a_null_written_into_a_name_by_its_escape_is_not_read_in_silence_spec_audit() {
    let mut lexer = Lexer::new(b"/A#00B ");
    let token = lexer.next_token();
    assert!(
        !matches!(&token, Some(Token::Name(bytes)) if bytes.contains(&0))
            || !lexer.repairs().is_empty(),
        "a name holding character code 0 is either refused or logged as a repair; read {token:?} \
         with repairs {:?}",
        lexer.repairs()
    );
}

/// The control: `/A#20B` is the three-character name A, SPACE, B (§7.3.5's own example) and reads
/// without a repair.
#[test]
fn a_space_written_into_a_name_by_its_escape_reads_without_a_repair() {
    let mut lexer = Lexer::new(b"/A#20B ");
    assert_eq!(lexer.next_token(), Some(Token::Name(b"A B".to_vec())));
    assert!(lexer.repairs().is_empty(), "{:?}", lexer.repairs());
}

/// ISO 32000-2 §7.3.10's object identifier as Errata Collection's issue #379 states its lexical
/// grammar (`doc/errata/site/docs/32000-2-2020/PDF-ObjectNumber.ebnf` and
/// `PDF-GenerationNumber.ebnf`, published, so the standard under ADR 3581): an object number is
/// `"1 - 9", [{"0 - 9"}-]` and a generation number `"0" | "1 - 9", [{"0 - 9"}-]` — no sign and no
/// leading zero. `+1 0 R`, `01 0 R` and `1 00 R` are each read as a reference with nothing logged,
/// the tolerance `doc/errata-read.md`'s #379 row calls undocumented.
#[test]
fn an_object_identifier_outside_the_amended_grammar_is_not_read_in_silence_spec_audit() {
    for source in [&b"[+1 0 R]"[..], b"[01 0 R]", b"[1 00 R]"] {
        let mut parser = Parser::new(source);
        let object = parser.parse_object().expect("the array reads");
        let referenced = object.as_array().is_some_and(|items| {
            items
                .iter()
                .any(|item| matches!(item, Object::Reference(_)))
        });
        assert!(
            !referenced || !parser.repairs().is_empty(),
            "{} is either not a reference or a logged repair; read {object:?} with repairs {:?}",
            String::from_utf8_lossy(source),
            parser.repairs()
        );
    }
}

/// The control: `[1 0 R]` is a reference and reads without a repair.
#[test]
fn an_object_identifier_in_the_amended_grammar_reads_without_a_repair() {
    let mut parser = Parser::new(b"[1 0 R]");
    let object = parser.parse_object().expect("the array reads");
    assert_eq!(
        object,
        Object::Array(vec![Object::Reference(ObjectId::new(1, 0))])
    );
    assert!(parser.repairs().is_empty(), "{:?}", parser.repairs());
}
