//! `FormCalc`'s lexical grammar: XFA 3.3 chapter 25, *Lexical Grammar* (pages 1054–1058),
//! productions 1 to 26, cited and paraphrased.
//!
//! The source is scanned left to right, the longest sequence that forms an input element taken
//! each time (page 1054); white space, line terminators and comments separate tokens and are
//! dropped. Keywords are case insensitive and identifiers are not (page 1057).

use std::fmt;

/// One token and the byte offset in the source it begins at.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What the token is.
    pub kind: Kind,
    /// The byte offset of its first character.
    pub at: usize,
}

/// The kinds of production 26's `Token`.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Production 11's number literal, as the IEEE 754 64-bit value page 1056 converts it to.
    Number(f64),
    /// Production 8's string literal, its escapes resolved.
    String(String),
    /// Production 14's identifier, case kept.
    Identifier(String),
    /// Productions 15 and 16's keywords and reserved keywords.
    Keyword(Keyword),
    /// Production 17's operators and production 25's separators.
    Punct(Punct),
}

/// Productions 15 and 16, in the lower case they compare in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(missing_docs, reason = "each variant is the keyword its name spells")]
pub enum Keyword {
    If,
    Then,
    Elseif,
    Else,
    Endif,
    Or,
    And,
    Not,
    Eq,
    Ne,
    Le,
    Ge,
    Lt,
    Gt,
    This,
    Null,
    Nan,
    Inf,
    Infinity,
    While,
    Do,
    Endwhile,
    End,
    For,
    Upto,
    Downto,
    Step,
    Endfor,
    Foreach,
    In,
    Break,
    Continue,
    Var,
    Func,
    Endfunc,
    Throw,
    Return,
    Exit,
}

impl Keyword {
    /// The keyword `word` spells, compared without case (page 1057); `None` for an identifier.
    ///
    /// `inf` is production 11's number literal rather than a keyword of production 15, which
    /// spells `infinity`; both are taken, so that neither can name a field.
    #[must_use]
    pub fn of(word: &str) -> Option<Self> {
        const ALL: [(&str, Keyword); 38] = [
            ("if", Keyword::If),
            ("then", Keyword::Then),
            ("elseif", Keyword::Elseif),
            ("else", Keyword::Else),
            ("endif", Keyword::Endif),
            ("or", Keyword::Or),
            ("and", Keyword::And),
            ("not", Keyword::Not),
            ("eq", Keyword::Eq),
            ("ne", Keyword::Ne),
            ("le", Keyword::Le),
            ("ge", Keyword::Ge),
            ("lt", Keyword::Lt),
            ("gt", Keyword::Gt),
            ("this", Keyword::This),
            ("null", Keyword::Null),
            ("nan", Keyword::Nan),
            ("inf", Keyword::Inf),
            ("infinity", Keyword::Infinity),
            ("while", Keyword::While),
            ("do", Keyword::Do),
            ("endwhile", Keyword::Endwhile),
            ("end", Keyword::End),
            ("for", Keyword::For),
            ("upto", Keyword::Upto),
            ("downto", Keyword::Downto),
            ("step", Keyword::Step),
            ("endfor", Keyword::Endfor),
            ("foreach", Keyword::Foreach),
            ("in", Keyword::In),
            ("break", Keyword::Break),
            ("continue", Keyword::Continue),
            ("var", Keyword::Var),
            ("func", Keyword::Func),
            ("endfunc", Keyword::Endfunc),
            ("throw", Keyword::Throw),
            ("return", Keyword::Return),
            ("exit", Keyword::Exit),
        ];
        ALL.iter()
            .find(|(spelling, _)| spelling.eq_ignore_ascii_case(word))
            .map(|&(_, keyword)| keyword)
    }
}

/// Production 17's operators and production 25's separators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Punct {
    /// `=`, the assignment.
    Assign,
    /// `|`, the logical or.
    Bar,
    /// `&`, the logical and.
    Ampersand,
    /// `==`.
    Equal,
    /// `<>`.
    NotEqual,
    /// `<=`.
    LessEqual,
    /// `>=`.
    GreaterEqual,
    /// `<`.
    Less,
    /// `>`.
    Greater,
    /// `+`.
    Plus,
    /// `-`.
    Minus,
    /// `*`, multiplication or the `[*]` of every occurrence.
    Star,
    /// `/`.
    Slash,
    /// `(`.
    OpenParen,
    /// `)`.
    CloseParen,
    /// `[`.
    OpenBracket,
    /// `]`.
    CloseBracket,
    /// `,`.
    Comma,
    /// `.`.
    Dot,
    /// `..`, the descendant accessor.
    DotDot,
    /// `.#`, the accessor of a property by class.
    DotHash,
    /// `.*`, every child.
    DotStar,
}

impl Punct {
    /// How the token is spelled in the source.
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::Bar => "|",
            Self::Ampersand => "&",
            Self::Equal => "==",
            Self::NotEqual => "<>",
            Self::LessEqual => "<=",
            Self::GreaterEqual => ">=",
            Self::Less => "<",
            Self::Greater => ">",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::OpenParen => "(",
            Self::CloseParen => ")",
            Self::OpenBracket => "[",
            Self::CloseBracket => "]",
            Self::Comma => ",",
            Self::Dot => ".",
            Self::DotDot => "..",
            Self::DotHash => ".#",
            Self::DotStar => ".*",
        }
    }
}

/// Why a source is not a sequence of production 1's input elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    /// A character no production starts with, at this byte offset.
    Unexpected(char, usize),
    /// A string literal with no closing quote, opened at this byte offset.
    UnterminatedString(usize),
    /// A `\u` escape not followed by four hexadecimal digits, at this byte offset.
    BadEscape(usize),
    /// A number literal whose exponent has no digits, at this byte offset.
    BadNumber(usize),
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unexpected(c, at) => write!(f, "unexpected character {c:?} at byte {at}"),
            Self::UnterminatedString(at) => {
                write!(f, "string literal opened at byte {at} never closes")
            }
            Self::BadEscape(at) => write!(
                f,
                "escape at byte {at} is not followed by four hexadecimal digits"
            ),
            Self::BadNumber(at) => write!(
                f,
                "number literal at byte {at} has an exponent without digits"
            ),
        }
    }
}

impl std::error::Error for LexError {}

/// Every token of `source`, in order.
///
/// # Errors
///
/// [`LexError`] at the first input element no production of the lexical grammar spells.
pub fn tokens(source: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer { source, at: 0 };
    let mut out = Vec::new();
    while let Some(token) = lexer.next()? {
        out.push(token);
    }
    Ok(out)
}

struct Lexer<'a> {
    source: &'a str,
    at: usize,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<char> {
        self.source[self.at..].chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        self.source[self.at..].chars().nth(1)
    }

    /// Moves past `bytes` bytes already matched; the sum stays within the source, so saturating
    /// is only the lint's proof.
    fn advance(&mut self, bytes: usize) {
        self.at = self.at.saturating_add(bytes);
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.advance(c.len_utf8());
        Some(c)
    }

    fn next(&mut self) -> Result<Option<Token>, LexError> {
        self.skip_insignificant();
        let at = self.at;
        let Some(c) = self.peek() else {
            return Ok(None);
        };
        let kind = if c == '"' {
            Kind::String(self.string()?)
        } else if c.is_ascii_digit()
            || (c == '.' && self.peek_second().is_some_and(|d| d.is_ascii_digit()))
        {
            Kind::Number(self.number()?)
        } else if c.is_alphabetic() || matches!(c, '_' | '$' | '!') {
            let word = self.identifier();
            match Keyword::of(word) {
                Some(keyword) => Kind::Keyword(keyword),
                None => Kind::Identifier(word.to_owned()),
            }
        } else {
            Kind::Punct(self.punct().ok_or(LexError::Unexpected(c, at))?)
        };
        Ok(Some(Token { kind, at }))
    }

    /// Productions 3 to 5: white space, line terminators, and a comment from `;` or `//` to the
    /// end of its line.
    fn skip_insignificant(&mut self) {
        while let Some(c) = self.peek() {
            if matches!(c, '\t' | '\u{b}' | '\u{c}' | ' ' | '\n' | '\r') {
                self.bump();
            } else if c == ';' || (c == '/' && self.peek_second() == Some('/')) {
                while self.peek().is_some_and(|c| c != '\n' && c != '\r') {
                    self.bump();
                }
            } else {
                break;
            }
        }
    }

    /// Productions 6 to 8. A doubled quote is a quote. The escape is the six characters page
    /// 1055's prose describes, `\u` and four hexadecimal digits, with production 7's `\U` taken
    /// as the same: production 7's eight-digit alternative names a character outside the Basic
    /// Multilingual Plane, which production 2's character set leaves out, so it is read as a
    /// four-digit escape followed by four characters.
    fn string(&mut self) -> Result<String, LexError> {
        let opened = self.at;
        self.bump();
        let mut text = String::new();
        loop {
            match self.bump() {
                None => return Err(LexError::UnterminatedString(opened)),
                Some('"') if self.peek() == Some('"') => {
                    self.bump();
                    text.push('"');
                }
                Some('"') => return Ok(text),
                Some('\\') if matches!(self.peek(), Some('u' | 'U')) => {
                    let escape = self.at.saturating_sub(1);
                    self.bump();
                    let digits = self
                        .source
                        .get(self.at..self.at.saturating_add(4))
                        .ok_or(LexError::BadEscape(escape))?;
                    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return Err(LexError::BadEscape(escape));
                    }
                    let unit =
                        u32::from_str_radix(digits, 16).map_err(|_| LexError::BadEscape(escape))?;
                    self.advance(4);
                    // A lone surrogate is no character; it is kept as the replacement character
                    // rather than refusing the script, as a string literal has no other spelling
                    // of it.
                    text.push(char::from_u32(unit).unwrap_or(char::REPLACEMENT_CHARACTER));
                }
                Some(c) => text.push(c),
            }
        }
    }

    /// Productions 9 to 11: digits, an optional fraction, an optional exponent.
    fn number(&mut self) -> Result<f64, LexError> {
        let start = self.at;
        self.digits();
        if self.peek() == Some('.')
            && self
                .peek_second()
                .is_none_or(|c| !matches!(c, '.' | '#' | '*'))
        {
            self.bump();
            self.digits();
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let rest = &self.source[self.at.saturating_add(1)..];
            let signed = rest.starts_with(['+', '-']);
            let after = if signed { &rest[1..] } else { rest };
            if after.starts_with(|c: char| c.is_ascii_digit()) {
                self.bump();
                if signed {
                    self.bump();
                }
                self.digits();
            } else if after.starts_with(|c: char| c.is_alphabetic() || c == '_') {
                // `1e` followed by a letter is not an exponent; the number ends at the digits and
                // what follows is the next token's, which the parser will refuse.
            } else {
                return Err(LexError::BadNumber(start));
            }
        }
        self.source[start..self.at]
            .parse()
            .map_err(|_| LexError::BadNumber(start))
    }

    fn digits(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
    }

    /// Production 14: an alphabetic character, `_`, `$` or `!`, then alphanumerics, `_` and `$`.
    fn identifier(&mut self) -> &str {
        let start = self.at;
        self.bump();
        while self
            .peek()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '$'))
        {
            self.bump();
        }
        &self.source[start..self.at]
    }

    /// Productions 17 and 25, the longest spelling first.
    fn punct(&mut self) -> Option<Punct> {
        let two = [
            ("==", Punct::Equal),
            ("<>", Punct::NotEqual),
            ("<=", Punct::LessEqual),
            (">=", Punct::GreaterEqual),
            ("..", Punct::DotDot),
            (".#", Punct::DotHash),
            (".*", Punct::DotStar),
        ];
        let rest = &self.source[self.at..];
        if let Some(&(spelling, punct)) =
            two.iter().find(|(spelling, _)| rest.starts_with(spelling))
        {
            self.advance(spelling.len());
            return Some(punct);
        }
        let punct = match self.peek()? {
            '=' => Punct::Assign,
            '|' => Punct::Bar,
            '&' => Punct::Ampersand,
            '<' => Punct::Less,
            '>' => Punct::Greater,
            '+' => Punct::Plus,
            '-' => Punct::Minus,
            '*' => Punct::Star,
            '/' => Punct::Slash,
            '(' => Punct::OpenParen,
            ')' => Punct::CloseParen,
            '[' => Punct::OpenBracket,
            ']' => Punct::CloseBracket,
            ',' => Punct::Comma,
            '.' => Punct::Dot,
            _ => return None,
        };
        self.bump();
        Some(punct)
    }
}
