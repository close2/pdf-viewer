//! Byte-level string and name checks for ISO 32000-2 §7.3.4 and §7.3.5.

use std::sync::Arc;

use pdf_syntax::{Lexer, Limits, Name, Object, Parser, SyntaxError, Token, write};

/// Table 3 mappings, unknown escapes, balanced nesting, and an empty literal string.
#[test]
fn literal_escape_table_and_nesting_produce_the_stated_bytes() {
    let mut lexer = Lexer::new(br"(a(b)c) (\n\r\t\b\f\(\)\\\q) ()");
    assert_eq!(
        lexer.next_token(),
        Some(Token::String(b"a(b)c".to_vec())),
        "balanced parentheses are part of the literal value"
    );
    assert_eq!(
        lexer.next_token(),
        Some(Token::String(vec![
            b'\n', b'\r', b'\t', 0x08, 0x0c, b'(', b')', b'\\', b'q'
        ])),
        "Table 3's eight escapes map to bytes, and an unlisted escape drops its backslash"
    );
    assert_eq!(
        lexer.next_token(),
        Some(Token::String(Vec::new())),
        "a literal string may have zero bytes"
    );
    assert_eq!(lexer.next_token(), None);
}

/// §7.3.4.2 allows one to three octal digits and discards overflow above the low byte.
#[test]
fn literal_octal_escapes_take_three_digits_and_keep_only_the_low_byte() {
    let mut lexer = Lexer::new(br"(\1|\12|\123|\400|\777|\0053)");
    assert_eq!(
        lexer.next_token(),
        Some(Token::String(vec![
            1, b'|', 10, b'|', 0o123, b'|', 0, b'|', 0xff, b'|', 5, b'3'
        ])),
        "one/two/three digits, overflow, and a following decimal digit are distinct cases"
    );
}

/// A backslash immediately before CR, LF, or CRLF removes that whole line ending.
#[test]
fn literal_line_continuations_remove_cr_lf_and_crlf() {
    for source in [b"(a\\\rb)".as_slice(), b"(a\\\nb)", b"(a\\\r\nb)"] {
        let mut lexer = Lexer::new(source);
        assert_eq!(
            lexer.next_token(),
            Some(Token::String(b"ab".to_vec())),
            "a line continuation contributes no string byte"
        );
    }
}

/// Every Table 1 whitespace byte is ignored inside a hexadecimal string.
#[test]
fn hexadecimal_strings_ignore_all_six_pdf_whitespace_bytes() {
    let mut lexer = Lexer::new(b"<4\0 1\t4\n2\x0c\r>");
    assert_eq!(
        lexer.next_token(),
        Some(Token::String(b"AB".to_vec())),
        "null, tab, line feed, form feed, carriage return, and space are ignored"
    );
    assert_eq!(lexer.next_token(), None);
}

/// Literal and hexadecimal forms feed the same byte object and the configured bound counts it.
#[test]
fn both_string_forms_share_the_byte_value_and_length_limit() {
    let mut literal = Parser::new(b"(AB)");
    let mut hexadecimal = Parser::new(b"<4142>");
    let expected = Object::String(Arc::from(&b"AB"[..]));
    assert_eq!(literal.parse_object(), Ok(expected.clone()));
    assert_eq!(hexadecimal.parse_object(), Ok(expected));

    let limits = Limits {
        max_string_len: 2,
        ..Limits::DEFAULT
    };
    let mut at_limit = Parser::with_limits(b"<4142>", limits);
    assert!(at_limit.parse_object().is_ok());

    let mut over_limit = Parser::with_limits(b"(ABC)", limits);
    assert!(matches!(
        over_limit.parse_object(),
        Err(SyntaxError::LimitExceeded {
            limit: "max_string_len",
            ..
        })
    ));
}

/// Hex decoding is checked against manually derived bytes, not against the writer.
#[test]
fn name_escapes_are_expanded_before_binary_identity_is_compared() {
    let mut lexer = Lexer::new(b"/A#42 /AB /A#43 /A#ff");
    assert_eq!(lexer.next_token(), Some(Token::Name(b"AB".to_vec())));
    assert_eq!(
        lexer.next_token(),
        Some(Token::Name(b"AB".to_vec())),
        "different written forms with the same expanded bytes are the same name"
    );

    assert_eq!(lexer.next_token(), Some(Token::Name(b"AC".to_vec())));
    assert_eq!(lexer.next_token(), Some(Token::Name(vec![b'A', 0xff])));
    assert_eq!(lexer.next_token(), None);
}

/// The reachable object writer emits legal byte-oriented string/name syntax.
#[test]
fn object_writer_emits_hand_expected_hex_string_and_name_bytes() {
    let mut output = Vec::new();
    write::object(&Object::String(Arc::from(&b"\0/\xff"[..])), &mut output);
    assert_eq!(
        output, b"<002FFF>",
        "the writer uses the hexadecimal string form"
    );

    output.clear();
    write::object(&Object::String(Arc::from(&b""[..])), &mut output);
    assert_eq!(
        output, b"<>",
        "the empty string has an empty hex representation"
    );

    output.clear();
    write::object(&Object::Name(Name::new(&b"Lime Green"[..])), &mut output);
    assert_eq!(
        output, b"/Lime#20Green",
        "the slash introduces the name and space is escaped"
    );

    output.clear();
    write::object(&Object::Name(Name::new(&b"A\xf4"[..])), &mut output);
    assert_eq!(
        output, b"/A#F4",
        "the writer uses ASCII hex for a high byte"
    );
}
