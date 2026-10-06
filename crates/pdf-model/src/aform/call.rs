//! The one shape of script Tier 0 dispatches: a single `AF*` call with literal arguments.
//!
//! RFC 0008 section 4.1 draws the line, and ADR 1579 is the grammar's argument: a field's script is
//! run without an engine only where its text is *textually* one call — a name, a parenthesised list
//! of literals, an optional `;` — and nothing else. Whitespace anywhere a token may be separated,
//! both of ECMAScript's quote characters, and an array literal of strings for
//! `AFSimple_Calculate`'s second argument are tolerated; an identifier, an operator, a second
//! statement or a comment is not, because accepting any of them is the first step of an
//! interpreter this tier does not contain.
//!
//! What a literal *means* is ECMAScript's (ECMA-262 section 12.9, the literal productions), read as
//! far as the shape needs: a decimal number with an optional exponent, a string with the escape
//! sequences of section 12.9.4, `true` and `false`. A number's sign is accepted in front of it
//! because a producer writes `-1` as an argument and nothing in the shape can be misread by
//! taking it; the expression `- 1` with whitespace inside is the same literal and is taken the same
//! way.

use super::Function;

/// The longest script text this grammar reads, in bytes.
///
/// A one-call script is a few dozen bytes; `javascript_census`'s longest one-call script in the
/// census population is under two hundred. The bound is what keeps a hostile `/JS` of hundreds of
/// megabytes from being scanned in full on every keystroke — a script this long is not one call,
/// whatever it says.
pub(crate) const MAX_CALL_BYTES: usize = 64 * 1024;

/// One literal argument, as ECMAScript reads it.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// A numeric literal, with its sign folded in.
    Number(f64),
    /// A string literal, its escape sequences decoded.
    String(String),
    /// `true` or `false`.
    Boolean(bool),
    /// An array literal whose every element is a string literal.
    ///
    /// The one compound literal the grammar takes, and only because `AFSimple_Calculate`'s field
    /// list is written this way in real documents as often as it is written as one string.
    Array(Vec<String>),
}

/// A script that is exactly one call of this library, with its arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// Which of the library's functions is called.
    pub function: Function,
    /// The arguments, in order. Fewer than the function declares is ECMAScript's `undefined` for
    /// the rest, which each function reads as its own default.
    pub arguments: Vec<Literal>,
}

/// Why a script's text is not one call this tier runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotOneCall {
    /// The text is not the one-call shape at all — an expression, a statement list, a comment.
    Shape,
    /// The text is one call, of a function this library does not define.
    ///
    /// Carries the name, because "`AFFoo` is not in the library" is a more useful sentence than
    /// "this is a script", and because a name the world calls that is missing here is the next
    /// thing a round could add.
    Unknown(String),
}

impl Call {
    /// Reads a script's text as one call, or says why it is not one.
    ///
    /// # Errors
    ///
    /// [`NotOneCall::Shape`] for any text that is not exactly the shape this module describes,
    /// and [`NotOneCall::Unknown`] for one call of an `AF`-prefixed name this library has not got.
    pub fn parse(text: &str) -> Result<Self, NotOneCall> {
        if text.len() > MAX_CALL_BYTES {
            return Err(NotOneCall::Shape);
        }
        let mut reader = Reader { rest: text };
        reader.skip_space();
        let name = reader.identifier().ok_or(NotOneCall::Shape)?;
        reader.skip_space();
        if !reader.eat('(') {
            return Err(NotOneCall::Shape);
        }
        let mut arguments = Vec::new();
        reader.skip_space();
        if !reader.eat(')') {
            loop {
                reader.skip_space();
                arguments.push(reader.literal().ok_or(NotOneCall::Shape)?);
                reader.skip_space();
                if reader.eat(',') {
                    continue;
                }
                if reader.eat(')') {
                    break;
                }
                return Err(NotOneCall::Shape);
            }
        }
        reader.skip_space();
        if reader.eat(';') {
            reader.skip_space();
        }
        if !reader.rest.is_empty() {
            return Err(NotOneCall::Shape);
        }
        match Function::from_name(name) {
            Some(function) => Ok(Self {
                function,
                arguments,
            }),
            None if name.starts_with("AF") => Err(NotOneCall::Unknown(name.to_owned())),
            None => Err(NotOneCall::Shape),
        }
    }
}

/// A cursor over the script's text.
struct Reader<'a> {
    /// What has not been read yet.
    rest: &'a str,
}

impl<'a> Reader<'a> {
    /// The next character, without taking it.
    fn peek(&self) -> Option<char> {
        self.rest.chars().next()
    }

    /// Takes the next character.
    fn bump(&mut self) -> Option<char> {
        let next = self.peek()?;
        self.rest = self.rest.get(next.len_utf8()..).unwrap_or_default();
        Some(next)
    }

    /// Takes `wanted` if it is next.
    fn eat(&mut self, wanted: char) -> bool {
        if self.peek() == Some(wanted) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Skips ECMA-262 section 12.2's white space and section 12.3's line terminators.
    fn skip_space(&mut self) {
        while self.peek().is_some_and(is_space) {
            self.bump();
        }
    }

    /// An identifier of ASCII letters, digits and `_`, not starting with a digit.
    ///
    /// Narrower than ECMAScript's identifier on purpose: every name this library defines is ASCII,
    /// and a wider reading would only let more text reach [`NotOneCall::Unknown`].
    fn identifier(&mut self) -> Option<&'a str> {
        let length = self
            .rest
            .char_indices()
            .find(|(at, c)| {
                !(c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                    || (*at == 0 && c.is_ascii_digit())
            })
            .map_or(self.rest.len(), |(at, _)| at);
        if length == 0 {
            return None;
        }
        let (name, rest) = self.rest.split_at(length);
        self.rest = rest;
        Some(name)
    }

    /// One literal argument.
    fn literal(&mut self) -> Option<Literal> {
        match self.peek()? {
            '"' | '\'' => self.string().map(Literal::String),
            '[' => self.array().map(Literal::Array),
            't' | 'f' => {
                let word = self.identifier()?;
                match word {
                    "true" => Some(Literal::Boolean(true)),
                    "false" => Some(Literal::Boolean(false)),
                    _ => None,
                }
            }
            _ => self.number().map(Literal::Number),
        }
    }

    /// An array literal of string literals: `[ "a", 'b' ]`, a trailing comma allowed as
    /// ECMA-262 section 13.2.4 allows it.
    fn array(&mut self) -> Option<Vec<String>> {
        if !self.eat('[') {
            return None;
        }
        let mut items = Vec::new();
        loop {
            self.skip_space();
            if self.eat(']') {
                return Some(items);
            }
            items.push(self.string()?);
            self.skip_space();
            if self.eat(',') {
                continue;
            }
            self.skip_space();
            if self.eat(']') {
                return Some(items);
            }
            return None;
        }
    }

    /// A numeric literal with an optional sign: `-1`, `+2.5`, `.5`, `1e3`, `0x1F`.
    fn number(&mut self) -> Option<f64> {
        let mut negative = false;
        if self.eat('-') {
            negative = true;
            self.skip_space();
        } else if self.eat('+') {
            self.skip_space();
        }
        let start = self.rest;
        // ECMA-262 section 12.9.3's hexadecimal integer literal, which a producer writing a
        // separator style or a flag occasionally uses.
        if let Some(hex) = start
            .strip_prefix("0x")
            .or_else(|| start.strip_prefix("0X"))
        {
            let digits = hex
                .char_indices()
                .find(|(_, c)| !c.is_ascii_hexdigit())
                .map_or(hex.len(), |(at, _)| at);
            if digits == 0 {
                return None;
            }
            let value = u64::from_str_radix(hex.get(..digits)?, 16).ok()?;
            self.rest = hex.get(digits..).unwrap_or_default();
            #[expect(
                clippy::cast_precision_loss,
                reason = "ECMAScript's numeric literal is a double: a hexadecimal literal past \
                          2^53 loses precision in every implementation, which is the literal's \
                          own meaning"
            )]
            let value = value as f64;
            return Some(if negative { -value } else { value });
        }
        let mut length = 0_usize;
        let mut digits = 0_usize;
        let bytes = start.as_bytes();
        while bytes.get(length).is_some_and(u8::is_ascii_digit) {
            length = length.saturating_add(1);
            digits = digits.saturating_add(1);
        }
        if bytes.get(length) == Some(&b'.') {
            length = length.saturating_add(1);
            while bytes.get(length).is_some_and(u8::is_ascii_digit) {
                length = length.saturating_add(1);
                digits = digits.saturating_add(1);
            }
        }
        if digits == 0 {
            return None;
        }
        if matches!(bytes.get(length), Some(b'e' | b'E')) {
            let mut at = length.saturating_add(1);
            if matches!(bytes.get(at), Some(b'+' | b'-')) {
                at = at.saturating_add(1);
            }
            let exponent_start = at;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at = at.saturating_add(1);
            }
            if at == exponent_start {
                return None;
            }
            length = at;
        }
        let text = start.get(..length)?;
        let value: f64 = text.parse().ok()?;
        self.rest = start.get(length..).unwrap_or_default();
        Some(if negative { -value } else { value })
    }

    /// A string literal in either quote character, its escapes decoded (ECMA-262 section 12.9.4).
    fn string(&mut self) -> Option<String> {
        let quote = self.bump()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let mut out = String::new();
        loop {
            let next = self.bump()?;
            if next == quote {
                return Some(out);
            }
            match next {
                // A line terminator ends a string literal unescaped (section 12.9.4's
                // `DoubleStringCharacter` excludes it), so a string that reaches one is not one.
                '\n' | '\r' | '\u{2028}' | '\u{2029}' => return None,
                '\\' => self.escape(&mut out)?,
                other => out.push(other),
            }
        }
    }

    /// One escape sequence after a `\`, appended to `out`.
    fn escape(&mut self, out: &mut String) -> Option<()> {
        let next = self.bump()?;
        match next {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            '0' if !self.peek().is_some_and(|c| c.is_ascii_digit()) => out.push('\0'),
            'x' => {
                let value = self.hex_digits(2)?;
                out.push(char::from_u32(value)?);
            }
            'u' => {
                let value = if self.eat('{') {
                    let mut value = 0_u32;
                    let mut any = false;
                    while let Some(c) = self.peek() {
                        if c == '}' {
                            break;
                        }
                        let digit = c.to_digit(16)?;
                        value = value.checked_mul(16)?.checked_add(digit)?;
                        any = true;
                        self.bump();
                    }
                    if !any || !self.eat('}') {
                        return None;
                    }
                    value
                } else {
                    self.hex_digits(4)?
                };
                // A UTF-16 surrogate pair written as two `\u` escapes is one character.
                if (0xD800..0xDC00).contains(&value) && self.rest.starts_with("\\u") {
                    let saved = self.rest;
                    self.bump();
                    self.bump();
                    if let Some(low) = self.hex_digits(4)
                        && (0xDC00..0xE000).contains(&low)
                    {
                        let high = value.checked_sub(0xD800)?;
                        let low = low.checked_sub(0xDC00)?;
                        let combined = high
                            .checked_mul(0x400)?
                            .checked_add(low)?
                            .checked_add(0x1_0000)?;
                        out.push(char::from_u32(combined)?);
                        return Some(());
                    }
                    self.rest = saved;
                }
                out.push(char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER));
            }
            // A line continuation contributes nothing (section 12.9.4's `LineContinuation`).
            '\n' | '\u{2028}' | '\u{2029}' => {}
            '\r' => {
                self.eat('\n');
            }
            // Every other character escapes to itself — `\"`, `\'`, `\\`, and `\q` alike.
            other => out.push(other),
        }
        Some(())
    }

    /// Exactly `count` hexadecimal digits.
    fn hex_digits(&mut self, count: usize) -> Option<u32> {
        let mut value = 0_u32;
        for _ in 0..count {
            let digit = self.bump()?.to_digit(16)?;
            value = value.checked_mul(16)?.checked_add(digit)?;
        }
        Some(value)
    }
}

/// ECMA-262 section 12.2's `WhiteSpace` and section 12.3's `LineTerminator`, which the grammar
/// skips between tokens.
fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\u{b}'
            | '\u{c}'
            | ' '
            | '\u{a0}'
            | '\u{feff}'
            | '\n'
            | '\r'
            | '\u{2028}'
            | '\u{2029}'
    ) || (c != '\u{200b}' && c.is_whitespace())
}
