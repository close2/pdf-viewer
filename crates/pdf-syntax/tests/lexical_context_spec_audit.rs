//! ISO 32000-2 §§7.2.3–7.2.4: byte classes depend on lexical context.
use pdf_syntax::lexer::{Lexer, Token};

fn tokens(bytes: &[u8]) -> Vec<Token<'_>> {
    let mut lexer = Lexer::new(bytes);
    std::iter::from_fn(|| lexer.next_token()).collect()
}

#[test]
fn a_comment_ends_at_each_pdf_eol_and_preserves_percent_inside_a_string() {
    // Table 1's CR, LF and CRLF are EOL. VT and NUL inside the comment do not terminate it;
    // its punctuation must not become syntax. A percent sign within a string is data.
    for ending in [b"\r".as_slice(), b"\n", b"\r\n"] {
        let mut input = b"/Before% \x0b\x00/Hidden << [ (".to_vec();
        input.extend_from_slice(ending);
        input.extend_from_slice(b"/After(100% literal)");
        assert_eq!(
            tokens(&input),
            [
                Token::Name(b"Before".to_vec()),
                Token::Name(b"After".to_vec()),
                Token::String(b"100% literal".to_vec())
            ],
            "comment terminator {ending:?}"
        );
    }
}

#[test]
fn non_ascii_bytes_and_vertical_tab_remain_regular_name_bytes() {
    // Table 1 is a closed whitespace set: VT and NBSP are regular bytes here. Case is
    // significant, and high bytes are not Unicode-decoded by the syntax lexer.
    assert_eq!(
        tokens(b"/A\x0bB /A\xa0B /A\xffB /a /A"),
        [
            Token::Name(b"A\x0bB".to_vec()),
            Token::Name(b"A\xa0B".to_vec()),
            Token::Name(b"A\xffB".to_vec()),
            Token::Name(b"a".to_vec()),
            Token::Name(b"A".to_vec())
        ]
    );
    // Each of the six stated whitespace bytes does end a token outside a string/comment.
    assert_eq!(
        tokens(b"/A\x00/B\t/C\n/D\x0c/E\r/F /G"),
        [b"A", b"B", b"C", b"D", b"E", b"F", b"G"].map(|name| Token::Name(name.to_vec()))
    );
}
