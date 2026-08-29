//! ISO 32000-2 §7.3.5's escaping, written and read back, pinned by hand-built pairs.
//!
//! One rule, two directions. [`pdf_syntax::Name::escaped`] writes a name into a file and
//! [`pdf_syntax::Lexer`] reads one out of it, and the clause states them as halves of the same
//! sentence:
//!
//! > When writing a name in a PDF file, a SOLIDUS (2Fh) (/) shall be used to introduce a name.
//! > The SOLIDUS is not part of the name but is a prefix indicating that what follows is a
//! > sequence of characters representing the name in the PDF file and shall follow these rules:
//!
//! and what makes two names one:
//!
//! > Uniquely defined means that any two name objects that, after all escaping is expanded (see
//! > below), and the resulting sequences of bytes are not an exact binary match denote different
//! > objects.
//!
//! # Why this is hand-built, all of it
//!
//! Trap 8. A corpus finds what documents contain, and what this file is about is what a *writer*
//! owes — the names below are ones no producer writes, which is exactly why nothing else in this
//! tree would notice them going wrong. The four shapes are the four the clause's rules a), b) and
//! c) divide the 256 byte values into: a regular character inside `!`..`~`, the number sign, a
//! character that is not regular, and a regular character above `~`.
//!
//! Until the six-hundred-and-seventeenth session §7.3.5's writing half was implemented once, for
//! dictionary keys, and `pdf_model::variable_text` — which writes a font name into a content
//! stream it constructs — could not reach it and wrote the name raw (ADR 0453). So the direction
//! this file pins is the one that had no test at all.

#![expect(
    clippy::panic,
    reason = "test code: a name that will not lex back must fail loudly, and naming what it \
              lexed as instead is the whole diagnosis"
)]

use pdf_syntax::{Lexer, Name, Token};

/// The name a lexer reads out of `/` followed by `escaped`.
///
/// The reader is the judge here for the same reason it is in `incremental_update.rs`: what a
/// writer owes is not a spelling but a file the reader agrees with.
fn read_back(escaped: &str) -> Vec<u8> {
    let source = format!("/{escaped}");
    let mut lexer = Lexer::new(source.as_bytes());
    match lexer.next_token() {
        Some(Token::Name(bytes)) => bytes,
        other => panic!("{source:?} lexed as {other:?} rather than as one name"),
    }
}

/// The four shapes §7.3.5's three rules divide a byte into, and what each is written as.
///
/// Rule c) — "[a]ny character that is not a regular character shall be written using its 2-digit
/// hexadecimal code" — covers the space and the solidus; rule a) covers the number sign, whose
/// escape is stated on its own because it is the escape character; and the byte above `~` is the
/// clause's own narrowing of rule b)'s choice:
///
/// > Regular characters that are outside the range EXCLAMATION MARK(21h) (!) to TILDE (7Eh) (~)
/// > should be written using the hexadecimal notation.
const SHAPES: [(&[u8], &str); 5] = [
    (b"Plain", "Plain"),
    (b"Lime Green", "Lime#20Green"),
    (b"The_Key_of_F#_Minor", "The_Key_of_F#23_Minor"),
    (b"paired()parentheses", "paired#28#29parentheses"),
    (b"A\xf4", "A#F4"),
];

/// Each shape's written form is the one §7.3.5's rules state.
#[test]
fn each_of_the_clauses_four_shapes_is_written_as_the_rule_says() {
    for (bytes, expected) in SHAPES {
        assert_eq!(
            Name::new(bytes).escaped(),
            expected,
            "§7.3.5 writes {bytes:?} as /{expected}"
        );
    }
}

/// Table 4's rows, in the direction the clause fixes the answer for: reading.
///
/// NOTE 1 says the other direction is not a function — "[t]here is not a unique encoding of names
/// into the PDF file because regular characters can be coded in either of two ways" — which is
/// why `/A#42` is here and not in [`SHAPES`]: a writer that emits `AB` for it is as correct as
/// one that emits `A#42`, and a reader that does not answer `AB` is not.
#[test]
fn table_4s_literal_names_read_as_table_4_says() {
    const TABLE_4: [(&str, &[u8]); 10] = [
        ("Name1", b"Name1"),
        ("ASomewhatLongerName", b"ASomewhatLongerName"),
        (
            "A;Name_With-Various***Characters?",
            b"A;Name_With-Various***Characters?",
        ),
        ("1.2", b"1.2"),
        ("$$", b"$$"),
        ("@pattern", b"@pattern"),
        (".notdef", b".notdef"),
        ("Lime#20Green", b"Lime Green"),
        ("paired#28#29parentheses", b"paired()parentheses"),
        ("A#42", b"AB"),
    ];

    for (written, expected) in TABLE_4 {
        assert_eq!(read_back(written), expected, "Table 4: /{written}");
    }
}

/// The round trip: write a name, read it back, and get the same bytes.
///
/// Every byte value, including the ones no name in this tree has ever held. Null is left out
/// because the clause leaves it out — a name is "a sequence of any characters (8-bit values)
/// except null (character code 0)" — and a name holding one is not a name to round-trip.
#[test]
fn every_byte_a_name_may_hold_survives_being_written_and_read() {
    for byte in 1..=u8::MAX {
        let name = Name::new(vec![b'A', byte, b'B']);
        let escaped = name.escaped();
        assert!(
            escaped.is_ascii(),
            "a written name is ASCII, so a content stream holding one can be a String: {escaped:?}"
        );
        assert_eq!(
            read_back(&escaped),
            name.as_bytes(),
            "byte {byte:#04x} written as /{escaped}"
        );
    }
}

/// A name is written as a *single token*, which is what rule c) is for.
///
/// The failure this catches is not a mis-decoded byte but a name that ends early: §7.2.3 makes a
/// delimiter or a white-space character end a token, so an unescaped one turns one name into a
/// name and something else. `/Odd Name` is `/Odd` followed by the keyword `Name`, which is what
/// `pdf_model::variable_text` used to write a `Tf` operand as.
#[test]
fn a_written_name_is_one_token_however_the_name_is_spelled() {
    for (bytes, _) in SHAPES {
        let source = format!("/{} 12 Tf", Name::new(bytes).escaped());
        let mut lexer = Lexer::new(source.as_bytes());
        assert_eq!(
            lexer.next_token(),
            Some(Token::Name(bytes.to_vec())),
            "{source:?}"
        );
        assert_eq!(lexer.next_token(), Some(Token::Integer(12)), "{source:?}");
        assert_eq!(
            lexer.next_token(),
            Some(Token::Keyword(b"Tf")),
            "{source:?}"
        );
        assert_eq!(lexer.next_token(), None, "{source:?}");
    }
}

/// The one byte pair this writer escapes by choice rather than by rule, pinned as a choice.
///
/// §7.2.3 makes `{` and `}` delimiters only inside a type 4 PostScript calculator function, so in
/// a name they are regular characters and rule b) — "shall be written as itself **or** by using
/// its 2-digit hexadecimal code" — leaves the writer both. It writes the code, so that a reader
/// holding Table 2's ten delimiters unconditionally reads back the name this program meant. The
/// reader here is this tree's own, which no longer holds them that way, and it agrees either way:
/// what the assertion pins is the *spelling*, which is where the choice lives.
#[test]
fn a_brace_in_a_name_is_written_as_its_hexadecimal_code_by_choice() {
    let name = Name::new(b"curly{braces}".to_vec());
    assert_eq!(name.escaped(), "curly#7Bbraces#7D");
    assert_eq!(read_back(&name.escaped()), name.as_bytes());
    // And the raw spelling rule b) also permits is one this tree's reader answers with the same
    // name, which is what makes the choice a choice rather than a repair.
    assert_eq!(read_back("curly{braces}"), name.as_bytes());
}

/// §7.3.5's own sentence, at the writer: two names that are not an exact binary match are two
/// names, so their written forms differ too.
///
/// The pairs differ in one byte apiece, and each byte is one the naive writer would have dropped
/// or folded: the two outside UTF-8 are the pair 604's sweep used everywhere else (ADR 0439), and
/// the two whose difference is a *delimiter* are the ones a writer with no escaping conflates by
/// ending the token in different places.
#[test]
fn two_names_that_differ_in_one_byte_are_written_differently() {
    const PAIRS: [(&[u8], &[u8]); 3] = [
        (b"A\xf4", b"A\xf5"),
        (b"Odd Name", b"OddName"),
        (b"a/b", b"ab"),
    ];

    for (left, right) in PAIRS {
        assert_ne!(left, right, "the pair differs before anything is written");
        let (left, right) = (Name::new(left), Name::new(right));
        assert_ne!(
            left.escaped(),
            right.escaped(),
            "two names, two written forms"
        );
        assert_eq!(read_back(&left.escaped()), left.as_bytes());
        assert_eq!(read_back(&right.escaped()), right.as_bytes());
    }
}
