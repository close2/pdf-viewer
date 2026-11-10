//! Focused checks for the byte classes and lexical boundaries in ISO 32000-2 §7.2.3–7.2.4.

use pdf_syntax::lexer::{is_delimiter, is_regular, is_whitespace};
use pdf_syntax::{Lexer, Name, Object, Token};

/// Table 1's six bytes separate tokens, including NUL, while repeated white space stays a
/// separator rather than creating empty tokens.
#[test]
fn each_table_1_whitespace_byte_separates_tokens() {
    for whitespace in [0x00, b'\t', b'\n', 0x0c, b'\r', b' '] {
        assert!(is_whitespace(whitespace));
        assert!(!is_regular(whitespace));

        let input = [b'1', whitespace, whitespace, b'2'];
        let mut lexer = Lexer::new(&input);
        assert_eq!(
            lexer.next_token(),
            Some(Token::Integer(1)),
            "{whitespace:#04x}"
        );
        assert_eq!(
            lexer.next_token(),
            Some(Token::Integer(2)),
            "{whitespace:#04x}"
        );
        assert_eq!(lexer.next_token(), None, "{whitespace:#04x}");
    }
}

/// The eight general Table 2 delimiters are excluded from regular runs. The braces are regular
/// outside Type 4 calculator programs, and bytes above ASCII remain regular PDF bytes.
#[test]
fn table_2_general_delimiters_and_regular_bytes_have_their_pdf_classes() {
    for delimiter in *b"()<>[]/%" {
        assert!(is_delimiter(delimiter), "{delimiter:#04x}");
        assert!(!is_regular(delimiter), "{delimiter:#04x}");
        assert!(!is_whitespace(delimiter), "{delimiter:#04x}");
    }

    for regular in [b'A', b'a', b'{', b'}', b'#', 0x80, 0xff] {
        assert!(is_regular(regular), "{regular:#04x}");
        assert!(!is_delimiter(regular), "{regular:#04x}");
        assert!(!is_whitespace(regular), "{regular:#04x}");
    }

    let mut lexer = Lexer::new(b"A\x80a A a");
    assert_eq!(lexer.next_token(), Some(Token::Keyword(b"A\x80a")));
    assert_eq!(lexer.next_token(), Some(Token::Keyword(b"A")));
    assert_eq!(lexer.next_token(), Some(Token::Keyword(b"a")));
    assert_eq!(lexer.next_token(), None);
}

/// A comment occupies one lexical separator: its contents do not become tokens, and CR, LF,
/// and CR-LF each end it so the following regular run remains a distinct token.
#[test]
fn comments_separate_tokens_and_stop_at_each_eol_form() {
    for eol in [&b"\r"[..], b"\n", b"\r\n"] {
        let mut input = b"left% ignored % ( [ < \xff".to_vec();
        input.extend_from_slice(eol);
        input.extend_from_slice(b"right");

        let mut lexer = Lexer::new(&input);
        assert_eq!(lexer.next_token(), Some(Token::Keyword(b"left")), "{eol:?}");
        assert_eq!(
            lexer.next_token(),
            Some(Token::Keyword(b"right")),
            "{eol:?}"
        );
        assert_eq!(lexer.next_token(), None, "{eol:?}");
    }
}

/// This writer chooses §7.3.4.3's hexadecimal spelling for string bytes, keeping the syntax
/// representation ASCII even when the value contains arbitrary binary bytes.
#[test]
fn binary_string_values_are_written_as_ascii_pdf_syntax() {
    let value = Object::String(vec![0x00, 0xff].into());
    let mut output = Vec::new();
    pdf_syntax::write::object(&value, &mut output);
    assert_eq!(output, b"<00FF>");
    assert!(output.is_ascii());

    let name = Object::Name(Name::new(&b"key/value"[..]));
    output.clear();
    pdf_syntax::write::object(&name, &mut output);
    assert_eq!(output, b"/key#2Fvalue");
    assert!(output.is_ascii());
}
