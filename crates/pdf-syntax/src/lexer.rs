//! Byte-level tokenisation of PDF syntax.
//!
//! # Bytes, not text
//!
//! PDF is a byte format. Names, strings and comments may hold arbitrary bytes, and the
//! specification defines whitespace and delimiters as byte values. Decoding to `str`
//! anywhere in here would either reject valid files or invent replacement characters, so
//! the lexer works on `&[u8]` throughout.
//!
//! # Termination on any input
//!
//! Every method advances the cursor or returns. Malformed input is the normal case — real
//! files are routinely truncated or corrupt — so the lexer never loops waiting for a
//! terminator that may not arrive.

/// Byte classification per ISO 32000-2 §7.2.3.
mod class {
    /// Whitespace: null, tab, line feed, form feed, carriage return, space.
    #[must_use]
    pub fn is_whitespace(byte: u8) -> bool {
        matches!(byte, 0 | b'\t' | b'\n' | 0x0c | b'\r' | b' ')
    }

    /// The nine delimiter characters.
    #[must_use]
    pub fn is_delimiter(byte: u8) -> bool {
        matches!(
            byte,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    }

    /// A regular character: neither whitespace nor a delimiter.
    #[must_use]
    pub fn is_regular(byte: u8) -> bool {
        !is_whitespace(byte) && !is_delimiter(byte)
    }
}

pub use class::{is_delimiter, is_regular, is_whitespace};

/// A lexical token.
///
/// Numbers are not converted here. `1 0 R` is three tokens, and only the parser knows
/// whether a leading integer begins a reference, so the lexer reports what it saw and
/// lets the parser decide.
///
/// A keyword *borrows* its bytes from the input; names and strings own theirs. The split
/// is not taste but what each variant is: a keyword and a number are spans of the input
/// verbatim, while a name's `#`-escapes and a string's backslash and hexadecimal forms
/// are *decoded*, so their bytes may not exist in the input at all. Borrowing the
/// verbatim spans is what removed a heap allocation per token on a content stream of
/// twenty million tokens, where the allocator was a fifth of the whole interpretation
/// (ADR 0341).
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    /// An integer.
    Integer(i64),
    /// A real number.
    Real(f64),
    /// A name, with `#`-escapes already resolved.
    Name(Vec<u8>),
    /// A string, decoded from either literal `(...)` or hexadecimal `<...>` form.
    String(Vec<u8>),
    /// `[`
    ArrayOpen,
    /// `]`
    ArrayClose,
    /// `<<`
    DictOpen,
    /// `>>`
    DictClose,
    /// `{`, used only in type 4 (PostScript calculator) functions.
    BraceOpen,
    /// `}`
    BraceClose,
    /// A bare keyword such as `obj`, `endobj`, `stream`, `true`, `xref`, borrowed from
    /// the input it was lexed from.
    ///
    /// Also every run of regular characters that spells no object at all, which by §7.3.3
    /// includes a run holding no decimal digit — `.` and `-` are keywords here, not zeroes.
    Keyword(&'a [u8]),
}

/// A cursor over PDF bytes yielding tokens.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    input: &'a [u8],
    position: usize,
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "cursor arithmetic is saturating throughout and every index is taken with \
              `get`, so no operation can overflow or go out of bounds"
)]
impl<'a> Lexer<'a> {
    /// Creates a lexer over `input`.
    #[must_use]
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    /// Creates a lexer positioned at `offset`, clamped to the end of the input.
    ///
    /// Clamped rather than rejected because a cross-reference table pointing past the end
    /// of the file is a common corruption, and the caller recovers by scanning instead.
    #[must_use]
    pub fn at(input: &'a [u8], offset: usize) -> Self {
        Self {
            input,
            position: offset.min(input.len()),
        }
    }

    /// Returns the current byte offset.
    #[must_use]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Moves the cursor, clamped to the end of the input.
    pub fn seek(&mut self, offset: usize) {
        self.position = offset.min(self.input.len());
    }

    /// Returns the whole input.
    #[must_use]
    pub fn input(&self) -> &'a [u8] {
        self.input
    }

    /// Returns the byte at the cursor without consuming it.
    #[must_use]
    pub fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    /// Returns `true` when the cursor is at or past the end.
    #[must_use]
    pub fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    /// Skips whitespace and comments.
    ///
    /// A comment runs from `%` to the next end of line. This is folded in with whitespace
    /// because the specification permits a comment anywhere whitespace is allowed, so
    /// every caller that skips one must skip the other.
    pub fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek() {
            if is_whitespace(byte) {
                self.position = self.position.saturating_add(1);
            } else if byte == b'%' {
                while let Some(byte) = self.peek() {
                    if byte == b'\n' || byte == b'\r' {
                        break;
                    }
                    self.position = self.position.saturating_add(1);
                }
            } else {
                break;
            }
        }
    }

    /// Reads the next token, or `None` at end of input.
    ///
    /// Unrecognised bytes are consumed as a keyword rather than raising an error, so the
    /// parser sees `Keyword` and decides. That keeps recovery policy in one place instead
    /// of spread between lexer and parser.
    pub fn next_token(&mut self) -> Option<Token<'a>> {
        self.skip_whitespace();
        let byte = self.peek()?;

        match byte {
            b'[' => {
                self.position = self.position.saturating_add(1);
                Some(Token::ArrayOpen)
            }
            b']' => {
                self.position = self.position.saturating_add(1);
                Some(Token::ArrayClose)
            }
            b'{' => {
                self.position = self.position.saturating_add(1);
                Some(Token::BraceOpen)
            }
            b'}' => {
                self.position = self.position.saturating_add(1);
                Some(Token::BraceClose)
            }
            b'/' => {
                self.position = self.position.saturating_add(1);
                Some(Token::Name(self.read_name()))
            }
            b'(' => {
                self.position = self.position.saturating_add(1);
                Some(Token::String(self.read_literal_string()))
            }
            b'<' => {
                if self.input.get(self.position.saturating_add(1)) == Some(&b'<') {
                    self.position = self.position.saturating_add(2);
                    Some(Token::DictOpen)
                } else {
                    self.position = self.position.saturating_add(1);
                    Some(Token::String(self.read_hex_string()))
                }
            }
            b'>' => {
                if self.input.get(self.position.saturating_add(1)) == Some(&b'>') {
                    self.position = self.position.saturating_add(2);
                    Some(Token::DictClose)
                } else {
                    // A lone `>` is malformed. Consumed so the cursor always advances;
                    // reported as a keyword so the parser can complain about it.
                    self.position = self.position.saturating_add(1);
                    Some(Token::Keyword(b">"))
                }
            }
            b')' => {
                // An unmatched `)`. As above: consume and report.
                self.position = self.position.saturating_add(1);
                Some(Token::Keyword(b")"))
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => Some(self.read_number()),
            _ => {
                let keyword = self.read_regular_run();
                if keyword.is_empty() {
                    // Not whitespace, not a delimiter we handle, yet no regular bytes:
                    // unreachable given the classification above, but consuming one byte
                    // guarantees progress rather than trusting that analysis.
                    self.position = self.position.saturating_add(1);
                    return self.next_token();
                }
                Some(Token::Keyword(keyword))
            }
        }
    }

    /// Consumes a run of regular characters, borrowed from the input.
    ///
    /// Borrowed rather than copied: on the largest content stream any instrument of this
    /// project has printed — 141 MiB carrying 20.8 million tokens, `doc/todo/44` — the
    /// `.to_vec()` this ended in was one short-lived heap allocation per token and put the
    /// allocator at ~20.8% of the whole interpretation (ADR 0341 has the A/B).
    fn read_regular_run(&mut self) -> &'a [u8] {
        let start = self.position;
        while self.peek().is_some_and(is_regular) {
            self.position = self.position.saturating_add(1);
        }
        self.input.get(start..self.position).unwrap_or_default()
    }

    /// Reads a name body, resolving `#xx` escapes.
    ///
    /// A malformed escape — `#` not followed by two hex digits — is kept literally. Real
    /// files contain them, and dropping the `#` would silently change the name into a
    /// different valid one, which is worse than preserving what was written.
    fn read_name(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(byte) = self.peek() {
            if !is_regular(byte) {
                break;
            }
            self.position = self.position.saturating_add(1);

            if byte == b'#' {
                let high = self.peek().and_then(hex_value);
                let low = self
                    .input
                    .get(self.position.saturating_add(1))
                    .copied()
                    .and_then(hex_value);
                if let (Some(high), Some(low)) = (high, low) {
                    out.push(high.saturating_mul(16).saturating_add(low));
                    self.position = self.position.saturating_add(2);
                    continue;
                }
                out.push(b'#');
            } else {
                out.push(byte);
            }
        }
        out
    }

    /// Reads a literal string, having consumed the opening parenthesis.
    ///
    /// Handles nested parentheses, backslash escapes, octal escapes and line
    /// continuations. An unterminated string ends at end of input rather than failing:
    /// truncated files are common, and returning what was read lets the caller salvage
    /// the rest of the document.
    fn read_literal_string(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut depth = 1usize;

        while let Some(byte) = self.peek() {
            self.position = self.position.saturating_add(1);
            match byte {
                b'\\' => {
                    let Some(escape) = self.peek() else { break };
                    self.position = self.position.saturating_add(1);
                    match escape {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'(' => out.push(b'('),
                        b')' => out.push(b')'),
                        b'\\' => out.push(b'\\'),
                        // A backslash before an end of line is a line continuation and
                        // contributes nothing to the string.
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.position = self.position.saturating_add(1);
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            // Up to three octal digits, fewer if what follows is not one.
                            let mut value = u16::from(escape - b'0');
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(digit @ b'0'..=b'7') => {
                                        value = value
                                            .saturating_mul(8)
                                            .saturating_add(u16::from(digit - b'0'));
                                        self.position = self.position.saturating_add(1);
                                    }
                                    _ => break,
                                }
                            }
                            // §7.3.4.2 states the truncation itself — "[h]igh-order overflow
                            // shall be ignored" — so the low byte is the clause's answer and
                            // not a convention. This comment said the opposite, and cited
                            // other implementations for a rule the standard prints; the
                            // four-hundred-and-seventeenth session found it while reading
                            // Errata Collection 3's Issue #494, which retitles the escape
                            // "[b]yte with value ddd in octal" and confirms the same reading
                            // from the other side.
                            out.push(u8::try_from(value & 0xff).unwrap_or(0));
                        }
                        // Any other escaped character stands for itself.
                        other => out.push(other),
                    }
                }
                b'(' => {
                    depth = depth.saturating_add(1);
                    out.push(b'(');
                }
                b')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                    out.push(b')');
                }
                other => out.push(other),
            }
        }

        out
    }

    /// Reads a hexadecimal string, having consumed the opening angle bracket.
    ///
    /// Non-hex bytes are ignored, as the specification requires, and a trailing odd digit
    /// is padded with zero.
    fn read_hex_string(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut pending: Option<u8> = None;

        while let Some(byte) = self.peek() {
            self.position = self.position.saturating_add(1);
            if byte == b'>' {
                break;
            }
            let Some(value) = hex_value(byte) else {
                continue;
            };
            match pending.take() {
                Some(high) => out.push(high.saturating_mul(16).saturating_add(value)),
                None => pending = Some(value),
            }
        }

        if let Some(high) = pending {
            out.push(high.saturating_mul(16));
        }
        out
    }

    /// Reads a number, or the keyword a run stating no digit lexically is.
    ///
    /// Accepts the malformed forms that occur in practice: multiple signs, a sign after
    /// digits, several decimal points. A run that states no digit at all is not a number
    /// and is returned as the keyword it lexically is; see the condition below for why.
    ///
    /// A run that *does* state a digit and still salvages nothing — `.-1`, where the sign
    /// arrives after the point and before any digit — keeps the older reading of zero.
    /// That is a different question from this one and the corpus offers no witness for it.
    fn read_number(&mut self) -> Token<'a> {
        let raw = self.read_regular_run();

        // §7.3.3 states both numeric forms in terms of digits. An integer:
        //
        // > An integer shall be written as one or more decimal digits optionally preceded by
        // > a sign.
        //
        // and a real:
        //
        // > A real value shall be written as one or more decimal digits with an optional
        // > sign and a leading, trailing, or embedded PERIOD (2Eh) (decimal point).
        //
        // So a run holding no decimal digit is neither, however many signs and points it
        // carries: `.` is not a numeric object, and neither is `-`. What it *is* is a run of
        // regular characters (§7.2.3) that does not spell an object, which is what `Keyword`
        // is for — and the parser then decides what one means where it stands, an error in a
        // file body and an unrecognised operator in a content stream (§7.8.2).
        //
        // Reading it as zero instead is the plausible fallback trap 5 forbids, and it cost a
        // mark: `/F0 . Tf` set a text font size of nought, so the show that followed drew
        // nothing and nothing was said about it.
        if !raw.iter().any(u8::is_ascii_digit) {
            return Token::Keyword(raw);
        }

        // Almost every number a content stream states is §7.3.3's own fixed format, and
        // the standard library's parser — correct for exponents, subnormals and worst-case
        // roundings a PDF number never uses — was 15.1% of interpreting a dense page
        // (ADR 0341). The fast path parses exactly the clause's grammar and nothing else;
        // anything it declines falls through to the standing path below, unchanged.
        match fixed_format_number(raw) {
            Some(Fixed::Integer(value)) => return Token::Integer(value),
            Some(Fixed::Real(value)) => return Token::Real(value),
            None => {}
        }

        // The run is parsed in place rather than copied into a `String` first — that copy
        // was one heap allocation per numeric token, on top of `read_regular_run`'s own
        // (ADR 0341). `from_utf8` refuses only a run holding a byte above 127, which no
        // numeric form contains; both parses below would refuse such a run anyway, so
        // falling straight through to the salvage is the same answer without the detour.
        if let Ok(text) = std::str::from_utf8(raw) {
            if !text.contains('.')
                && let Ok(value) = text.parse::<i64>()
            {
                return Token::Integer(value);
            }
            if let Ok(value) = text.parse::<f64>() {
                return if value.is_finite() {
                    Token::Real(value)
                } else {
                    Token::Integer(0)
                };
            }
        }

        // Salvage a leading numeric prefix from forms like `--5` or `1.2.3`.
        match salvage_number(raw) {
            Some(value) if value.fract() == 0.0 && value.abs() < 9.0e15 =>
            {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "guarded to values with no fractional part and well inside \
                              i64's exact range"
                )]
                Token::Integer(value as i64)
            }
            Some(value) => Token::Real(value),
            // A digit is present — the condition above saw to that — but nothing before it
            // could be read as one. See this function's doc comment for why that keeps the
            // older reading rather than joining the case above.
            None => Token::Integer(0),
        }
    }
}

/// A number in exactly the fixed format ISO 32000-2 §7.3.3 states.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Fixed {
    /// The clause's integer form.
    Integer(i64),
    /// The clause's real form.
    Real(f64),
}

/// Parses §7.3.3's two numeric forms directly from their bytes.
///
/// The clause states both forms it accepts, and this function accepts precisely those:
///
/// > An integer shall be written as one or more decimal digits optionally preceded by a
/// > sign.
///
/// > A real value shall be written as one or more decimal digits with an optional sign and
/// > a leading, trailing, or embedded PERIOD (2Eh) (decimal point).
///
/// One optional leading sign, decimal digits, at most one period, nothing else. Everything
/// outside that grammar — an exponent, a repeated sign, a second period, a byte that is no
/// digit — is declined to the caller's standing `parse`-then-salvage path, so every
/// malformed form keeps the reading it always had.
///
/// # Why the arithmetic is exact rather than approximate
///
/// The digits accumulate into an integer mantissa `m`, refused past 15 digits, and the
/// value is `m / 10^f` for `f` digits after the period. `m < 10^15 < 2^53`, so `m` is
/// exactly representable as an `f64`; so is `10^f` for `f ≤ 15`; and IEEE 754 division
/// rounds its mathematically exact quotient once, to nearest — so the result is the
/// correctly rounded value of the decimal the bytes state, which is the same value
/// `f64::from_str` returns. Bit for bit, which
/// `the_fixed_format_parse_agrees_with_the_standard_library` exercises; the reason it is
/// worth having is the benchmark in ADR 0341, where the library parser's generality was
/// 15.1% of interpreting a dense content stream.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`digits` is checked against 15 before every multiply, so the mantissa holds \
              at most 15 decimal digits and cannot overflow a u64; the counters are \
              bounded by the run's length"
)]
fn fixed_format_number(raw: &[u8]) -> Option<Fixed> {
    /// `10^f` for every fractional length the mantissa bound admits; each is a power of
    /// ten below `2^53` and therefore exactly representable.
    const POWERS_OF_TEN: [f64; 16] = [
        1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15,
    ];

    let (negative, body) = match raw.split_first() {
        Some((b'-', rest)) => (true, rest),
        Some((b'+', rest)) => (false, rest),
        _ => (false, raw),
    };

    let mut mantissa: u64 = 0;
    let mut digits = 0usize;
    let mut fraction: Option<usize> = None;
    for &byte in body {
        match byte {
            b'0'..=b'9' => {
                digits += 1;
                if digits > 15 {
                    return None;
                }
                mantissa = mantissa * 10 + u64::from(byte - b'0');
                if let Some(count) = fraction.as_mut() {
                    *count += 1;
                }
            }
            b'.' if fraction.is_none() => fraction = Some(0),
            _ => return None,
        }
    }
    // "One or more decimal digits" in both forms, so a run stating none is not this
    // function's to read — the caller has already returned such a run as a keyword.
    if digits == 0 {
        return None;
    }

    match fraction {
        // Fits because the mantissa holds at most 15 decimal digits.
        None => i64::try_from(mantissa)
            .ok()
            .map(|magnitude| Fixed::Integer(if negative { -magnitude } else { magnitude })),
        Some(count) => {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the mantissa is below 2^53, where every integer is exactly \
                          representable — the doc comment's exactness argument rests on it"
            )]
            let magnitude = mantissa as f64 / POWERS_OF_TEN.get(count).copied()?;
            Some(Fixed::Real(if negative { -magnitude } else { magnitude }))
        }
    }
}

/// Extracts a usable number from a malformed numeric token.
///
/// `--5` yields -5, `1.2.3` yields 1.2, `-` yields nothing. Real files contain all of
/// these.
///
/// A repeated leading sign collapses to one rather than invalidating the number:
/// producers emit `--5` by prepending a minus to an already-negative value, and both
/// Acrobat and pdf.js read it as -5. Reading it as +5 would silently mirror geometry.
#[expect(
    clippy::match_same_arms,
    reason = "the two `break` arms have different guards for different reasons — a stray \
              sign versus a second decimal point — and merging them into one guard \
              obscures both, as an earlier attempt that broke `1.5` demonstrated"
)]
fn salvage_number(text: &[u8]) -> Option<f64> {
    let mut cleaned = String::with_capacity(text.len());
    let mut seen_dot = false;
    let mut seen_digit = false;
    let mut in_leading_signs = true;

    // Bytes rather than `char`s, and the two walks are the same walk: every byte the loop
    // keeps is ASCII, and any other byte — including each byte of a multi-byte sequence —
    // terminates the number exactly where a decoded character would have.
    for &byte in text {
        match byte {
            b'-' | b'+' if in_leading_signs => {
                // Only the first sign counts; later ones in the run are dropped.
                if cleaned.is_empty() {
                    cleaned.push(char::from(byte));
                }
            }
            // Anything else that cannot extend the number terminates it. A sign after
            // the digits have started is not ignored: `1-2` is two numbers jammed
            // together in the wild, and taking the first is closer to what was meant
            // A sign once the number has started terminates it: `1-2` is two numbers
            // jammed together in the wild, and taking the first is closer to what was
            // meant than reading `12`.
            b'-' | b'+' => break,
            // A second decimal point likewise ends the number rather than being ignored.
            b'.' if seen_dot => break,
            b'.' => {
                in_leading_signs = false;
                seen_dot = true;
                cleaned.push('.');
            }
            b'0'..=b'9' => {
                in_leading_signs = false;
                seen_digit = true;
                cleaned.push(char::from(byte));
            }
            _ => break,
        }
    }

    if !seen_digit {
        return None;
    }
    cleaned
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

/// Returns the numeric value of a hexadecimal digit.
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "each arm's range guarantees the subtraction is in range and the sum is at \
              most 15"
)]
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Lexer, Token};

    fn tokens(input: &[u8]) -> Vec<Token<'_>> {
        let mut lexer = Lexer::new(input);
        std::iter::from_fn(|| lexer.next_token()).collect()
    }

    #[test]
    fn integers_and_reals_are_distinguished() {
        assert_eq!(
            tokens(b"0 1 -2 +3"),
            vec![
                Token::Integer(0),
                Token::Integer(1),
                Token::Integer(-2),
                Token::Integer(3)
            ]
        );
        assert_eq!(
            tokens(b"1.5 -0.25 4."),
            vec![Token::Real(1.5), Token::Real(-0.25), Token::Real(4.0)]
        );
    }

    /// `.5` and `-.5` are legal PDF and appear constantly in content streams.
    #[test]
    fn a_leading_decimal_point_is_accepted() {
        assert_eq!(tokens(b".5 -.5"), vec![Token::Real(0.5), Token::Real(-0.5)]);
    }

    /// Malformed numbers occur in real files; other viewers accept them, so we must.
    #[test]
    fn malformed_numbers_salvage_a_leading_value() {
        assert_eq!(tokens(b"--5"), vec![Token::Integer(-5)]);
        assert_eq!(tokens(b"1.2.3"), vec![Token::Real(1.2)]);
    }

    /// §7.3.3 writes both numeric forms as "one or more decimal digits", so a run stating
    /// none is not a numeric object at all. It is a run of regular characters that does not
    /// spell one, which is a keyword — and the parser refuses it where an object was
    /// expected instead of inventing a zero nobody wrote.
    #[test]
    fn a_run_with_no_digit_is_not_a_number() {
        for run in [&b"."[..], b"-", b"+", b"--", b"-.", b".-"] {
            assert_eq!(
                tokens(run),
                vec![Token::Keyword(run)],
                "{} states no digit and is therefore no number",
                String::from_utf8_lossy(run)
            );
        }
        // The forms §7.3.3's own EXAMPLE 2 prints stay numbers, which is what the condition
        // has to leave alone: each of them states a digit.
        assert_eq!(
            tokens(b"34.5 -3.62 +123.6 4. -.002 0"),
            vec![
                Token::Real(34.5),
                Token::Real(-3.62),
                Token::Real(123.6),
                Token::Real(4.0),
                Token::Real(-0.002),
                Token::Integer(0),
            ]
        );
    }

    /// `fixed_format_number` must agree with the standard library bit for bit.
    ///
    /// The exactness argument is in its doc comment; this is that argument exercised
    /// rather than trusted: every digit string up to five digits, under every sign, with
    /// the decimal point in every position including absent, lexes to the same bits
    /// `str::parse` produces — including the sign of `-0.0`, which `to_bits` sees and
    /// `==` would not.
    #[test]
    fn the_fixed_format_parse_agrees_with_the_standard_library() {
        for value in 0..=99_999u32 {
            let digits = value.to_string();
            for sign in ["", "-", "+"] {
                for dot in (0..=digits.len()).map(Some).chain([None]) {
                    let text = match dot {
                        Some(at) => format!("{sign}{}.{}", &digits[..at], &digits[at..]),
                        None => format!("{sign}{digits}"),
                    };
                    let lexed = tokens(text.as_bytes());
                    match (dot, lexed.as_slice()) {
                        (Some(_), [Token::Real(actual)]) => {
                            let expected: f64 = text.parse().unwrap();
                            assert_eq!(
                                actual.to_bits(),
                                expected.to_bits(),
                                "{text}: {actual} against the library's {expected}"
                            );
                        }
                        (None, [Token::Integer(actual)]) => {
                            let expected: i64 = text.parse().unwrap();
                            assert_eq!(*actual, expected, "{text}");
                        }
                        (_, other) => panic!("{text} lexed as {other:?}"),
                    }
                }
            }
        }
    }

    #[test]
    fn names_resolve_hash_escapes() {
        assert_eq!(tokens(b"/Name"), vec![Token::Name(b"Name".to_vec())]);
        assert_eq!(tokens(b"/A#20B"), vec![Token::Name(b"A B".to_vec())]);
        assert_eq!(
            tokens(b"/"),
            vec![Token::Name(Vec::new())],
            "the empty name is legal"
        );
    }

    /// Dropping the `#` would turn the name into a different valid name, silently.
    #[test]
    fn a_malformed_hash_escape_is_kept_literally() {
        assert_eq!(tokens(b"/A#ZZ"), vec![Token::Name(b"A#ZZ".to_vec())]);
        assert_eq!(tokens(b"/A#"), vec![Token::Name(b"A#".to_vec())]);
    }

    #[test]
    fn literal_strings_handle_nesting_and_escapes() {
        assert_eq!(tokens(b"(plain)"), vec![Token::String(b"plain".to_vec())]);
        assert_eq!(tokens(b"(a(b)c)"), vec![Token::String(b"a(b)c".to_vec())]);
        assert_eq!(
            tokens(br"(\n\r\t\(\))"),
            vec![Token::String(b"\n\r\t()".to_vec())]
        );
        assert_eq!(
            tokens(br"(\101)"),
            vec![Token::String(b"A".to_vec())],
            "octal escape"
        );
        assert_eq!(
            tokens(b"(a\\\nb)"),
            vec![Token::String(b"ab".to_vec())],
            "a backslash before a newline is a line continuation"
        );
    }

    /// Truncated files are ordinary. Returning what was read lets the rest be salvaged.
    #[test]
    fn an_unterminated_string_ends_at_end_of_input() {
        assert_eq!(tokens(b"(abc"), vec![Token::String(b"abc".to_vec())]);
    }

    #[test]
    fn hex_strings_ignore_junk_and_pad_an_odd_digit() {
        assert_eq!(tokens(b"<4142>"), vec![Token::String(b"AB".to_vec())]);
        assert_eq!(tokens(b"<41 42>"), vec![Token::String(b"AB".to_vec())]);
        assert_eq!(
            tokens(b"<414>"),
            vec![Token::String(vec![0x41, 0x40])],
            "odd digit pads"
        );
        assert_eq!(tokens(b"<>"), vec![Token::String(Vec::new())]);
    }

    #[test]
    fn dictionary_and_array_delimiters_are_recognised() {
        assert_eq!(
            tokens(b"<< >> [ ]"),
            vec![
                Token::DictOpen,
                Token::DictClose,
                Token::ArrayOpen,
                Token::ArrayClose
            ]
        );
    }

    #[test]
    fn comments_are_skipped() {
        assert_eq!(
            tokens(b"1 % comment\n2"),
            vec![Token::Integer(1), Token::Integer(2)]
        );
        assert_eq!(tokens(b"% only a comment"), Vec::new());
    }

    #[test]
    fn keywords_are_returned_verbatim() {
        assert_eq!(
            tokens(b"obj endobj"),
            vec![Token::Keyword(b"obj"), Token::Keyword(b"endobj")]
        );
    }

    /// The property fuzzing will lean on: the lexer must terminate and make progress on
    /// anything at all, including bytes that classify as neither whitespace, delimiter
    /// nor regular character.
    #[test]
    fn every_byte_value_terminates() {
        for byte in 0u8..=255 {
            let input = [byte, byte, byte];
            let mut lexer = Lexer::new(&input);
            let mut count = 0;
            while lexer.next_token().is_some() {
                count += 1;
                assert!(
                    count <= input.len(),
                    "byte {byte:#04x} produced too many tokens"
                );
            }
            assert!(
                lexer.is_at_end(),
                "byte {byte:#04x} left the cursor short of the end"
            );
        }
    }

    #[test]
    fn unmatched_closers_are_consumed_rather_than_looping() {
        assert_eq!(tokens(b")"), vec![Token::Keyword(b")")]);
        assert_eq!(tokens(b">"), vec![Token::Keyword(b">")]);
    }
}
