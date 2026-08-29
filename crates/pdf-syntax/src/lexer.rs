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
    pub const fn is_whitespace(byte: u8) -> bool {
        matches!(byte, 0 | b'\t' | b'\n' | 0x0c | b'\r' | b' ')
    }

    /// The eight delimiter characters a byte of PDF syntax can be one of.
    ///
    /// ISO 32000-2 §7.2.3's Table 2 lists ten, and the two it lists beyond these are conditional
    /// rather than general — the clause says so in the sentence that introduces the table:
    ///
    /// > The delimiter characters { and } (LEFT CURLY BRACE (7Bh) and RIGHT CURLY BRACE (7Dh))
    /// > are additional delimiter characters within Type 4 PostScript calculator functions
    /// > (see 7.10.5 "Type 4 (PostScript calculator) functions").
    ///
    /// Errata Collection 3's Issue #365 writes that condition into the table itself, as a
    /// footnote on the `{` and `}` rows saying they are *additional delimiter characters only
    /// within Type 4 PostScript calculator functions* — so outside such a program the two are
    /// **regular** characters, and `/A{B}` is one name rather than a name and three tokens.
    ///
    /// This predicate is therefore the general classification and nothing here lexes a type 4
    /// program: `pdf_model::function::compile_postscript` tokenises one itself, where the two
    /// braces do delimit. `doc/errata-read.md` has the erratum with its rectangles.
    #[must_use]
    pub const fn is_delimiter(byte: u8) -> bool {
        matches!(byte, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'/' | b'%')
    }

    /// Which of the 256 byte values are regular, decided at compile time by the two
    /// predicates above.
    ///
    /// **A table because of what the predicates compile to in a per-byte loop.** The two
    /// `matches!` above are a dozen instructions together: the whitespace set fits one
    /// 64-bit mask, the delimiters straddled 37 to 125 and needed two — and `read_regular_run`
    /// asks the question once per byte of every token. On `doc/todo/44`'s witness, one page
    /// of 141 MiB carrying 20.8 million tokens, that function alone was **15.57%** of
    /// interpreting the page, at about seventeen instructions a byte; against a table it is
    /// a load and a test. ADR 0370 has the A/B, and it was taken when the delimiter set still
    /// held the two braces: dropping those narrows it to 37 through 93, so the predicate this
    /// table replaces is cheaper than the one that was measured. The table stays on the
    /// measurement that exists rather than on an argument about masks.
    ///
    /// The classification is still stated exactly once, in the two predicates: this is
    /// their answer, tabulated, not a second copy of §7.2.3's sets.
    const REGULAR: [bool; 256] = {
        let mut table = [false; 256];
        let mut code = 0_usize;
        while code < 256 {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the loop condition bounds `code` to 0..256, which is u8's range"
            )]
            let byte = code as u8;
            table[code] = !is_whitespace(byte) && !is_delimiter(byte);
            code = code.saturating_add(1);
        }
        table
    };

    /// A regular character: neither whitespace nor a delimiter.
    #[must_use]
    pub fn is_regular(byte: u8) -> bool {
        // The index is a `u8` widened to `usize`, so it is inside a 256-entry table by
        // construction and the compiler elides the bounds check.
        REGULAR[byte as usize]
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
    ///
    /// **The `peek` loop stays, and that is a measurement rather than an oversight.** Finding
    /// the run in one pass over the slice instead — a slice iterator carrying its own end, so
    /// the bounds check and the cursor store are paid once rather than per byte — was built
    /// and measured on the same witness at **+2.64%**, this function itself 1.1% worse: LLVM
    /// had already removed what the rewrite was meant to remove, and what it added was a
    /// second cursor to re-fuse. What was costing seventeen instructions a byte here was
    /// [`is_regular`], and that is where it went (ADR 0370).
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
    /// Handles nested parentheses, backslash escapes, octal escapes, line continuations and
    /// ISO 32000-2 §7.3.4.2's end-of-line rule. An unterminated string ends at end of input
    /// rather than failing: truncated files are common, and returning what was read lets the
    /// caller salvage the rest of the document.
    ///
    /// > An end-of-line marker appearing within a literal string without a preceding REVERSE
    /// > SOLIDUS shall be treated as a byte value of (0Ah), irrespective of whether the
    /// > end-of-line marker was a CARRIAGE RETURN (0Dh), a LINE FEED (0Ah), or both.
    ///
    /// **That sentence is the one this function did not implement**, and it is a `shall` about
    /// the *bytes* a string object holds rather than about how they are displayed: a literal
    /// string is one of the two forms every byte string may be written in — Errata Collection 3
    /// Issue #276 inserts "[u]nless otherwise stated in this document, a byte string may be
    /// either a literal string (see 7.3.4.2, "Literal strings") or a hexadecimal string (see
    /// 7.3.4.3, "Hexadecimal strings")" into §7.9.2.4 — so a CARRIAGE RETURN kept as itself is
    /// a `/U`, a `/Perms` or an `/ID` one byte different from what the file states, and inside a
    /// content stream it is a different glyph code. Escaped end-of-line markers are untouched:
    /// Table 3 gives `\r` and `\n` their own byte values, and a REVERSE SOLIDUS immediately
    /// before an end-of-line marker is the line continuation above.
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
                // §7.3.4.2's end-of-line rule, quoted above. A CARRIAGE RETURN followed by a
                // LINE FEED is one marker and therefore one byte; a LINE FEED followed by a
                // CARRIAGE RETURN is two markers and reaches here twice.
                b'\r' => {
                    if self.peek() == Some(b'\n') {
                        self.position = self.position.saturating_add(1);
                    }
                    out.push(b'\n');
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
        // §7.3.3's fixed format is read **straight off the cursor**, before the run this
        // function used to find first. Both statements are about the same bytes, so finding
        // the run and then parsing it walked every well-formed number twice — and on
        // `doc/todo/44`'s witness 17.65 million of the page's 20.83 million tokens are
        // numbers, so that second walk was most of what [`Self::read_regular_run`] did.
        // Fusing the two passes is worth **5.4%** of interpreting that page — 11 470.9 M
        // instructions to 10 848.7 M, callgrind, A/B in one sitting, with
        // [`Self::read_regular_run`] itself falling from 417.5 M to 16.1 M (ADR 0424).
        //
        // What it costs in clarity is one condition that has to be read beside §7.2.3
        // rather than beside §7.3.3: [`fixed_format_number`] stops at the first byte outside
        // its grammar, and *that byte must also end the token*. Where it does not — `12pt`,
        // `1.2.3`, `5f` — the fixed format has read a prefix of something longer, so the
        // answer is thrown away and the slower path below owns the run exactly as it always
        // did. A delimiter or white space, or the end of the input, means the run is over
        // and the parse stands.
        let rest = self.input.get(self.position..).unwrap_or_default();
        if let Some((value, taken)) = fixed_format_number(rest)
            && !rest.get(taken).copied().is_some_and(is_regular)
        {
            self.position = self.position.saturating_add(taken);
            return match value {
                Fixed::Integer(value) => Token::Integer(value),
                Fixed::Real(value) => Token::Real(value),
            };
        }

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
        // The readings below follow in the order a run meets them, and the clause decides
        // that order as well as each of them.

        // Almost every number a content stream states is the fixed format the two sentences
        // above define, and the standard library's parser — correct for exponents,
        // subnormals and worst-case roundings a PDF number never uses — was 15.1% of
        // interpreting a dense page (ADR 0341). [`fixed_format_number`], asked above, parses
        // exactly that grammar and nothing else; anything it declines falls through to the
        // two readings here, unchanged.
        //
        // **It is asked before the digit scan, and that is an ordering rather than a rule
        // change**: both forms are "one or more decimal digits", so a run that function
        // accepts is a run that holds one, and the scan below could only have agreed with it.
        // Asking the scan first walked every well-formed number's bytes a second time — three
        // passes over a token that is almost always four characters — and this is the pass
        // that is skipped when the fast path answers (ADR 0370).

        // A run holding no decimal digit is neither form, however many signs and points it
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

        // The run is parsed in place rather than copied into a `String` first — that copy
        // was one heap allocation per numeric token, on top of `read_regular_run`'s own
        // (ADR 0341). `from_utf8` refuses only a run holding a byte above 127, which no
        // numeric form contains; both parses below would refuse such a run anyway, so
        // falling straight through to the salvage is the same answer without the detour.
        //
        // **`str::parse` reads a third form the two sentences above do not have, and that is
        // deliberate.** §7.3.3's last paragraph names it and forbids exactly one party:
        //
        // > A PDF writer shall not use the PostScript language syntax for numbers with
        // > non-decimal radices (such as 16#FFFE) or in exponential format (such as 6.02E23).
        //
        // The `shall not` is the *writer's*, and the clause states nothing at all for a reader
        // that meets one — while naming the syntax and glossing its value in the same breath,
        // so `1e2` is a hundred by the standard's own example rather than by our invention.
        // Reading it is therefore the §7.3.10 answer one clause family along: a producer's
        // spelling of a number is not a question about what the number is, and refusing would
        // lose a mark to a requirement no sentence places on us. It is a *departure* all the
        // same, and one nothing here said until the eight-hundredth session — the two
        // blockquotes above are about digits, and the line under them quietly accepted an
        // exponent. `pdf-model/examples/numeric_form_census` counts the population it decides;
        // `sci-notation.pdf` is the pdf.js corpus's only witness, one run in 964 documents.
        //
        // `inf` and `NaN` are the other two spellings `f64::from_str` has and §7.3.3 does not,
        // and neither reaches here: both hold no decimal digit, so the condition above returns
        // each as the keyword it lexically is (ADR 0303).
        if let Ok(text) = std::str::from_utf8(raw) {
            if !text.contains('.')
                && let Ok(value) = text.parse::<i64>()
            {
                return Token::Integer(value);
            }
            if let Ok(value) = text.parse::<f64>() {
                return Token::Real(within_the_representation(value));
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

/// Parses §7.3.3's two numeric forms directly from their bytes, and says how many it read.
///
/// The clause states both forms it accepts, and this function accepts precisely those:
///
/// > An integer shall be written as one or more decimal digits optionally preceded by a
/// > sign.
///
/// > A real value shall be written as one or more decimal digits with an optional sign and
/// > a leading, trailing, or embedded PERIOD (2Eh) (decimal point).
///
/// One optional leading sign, decimal digits, at most one period, nothing else. It *stops*
/// at the first byte outside that grammar rather than refusing the whole input, and the
/// returned length is where it stopped — which is what lets [`Lexer::read_number`] read a
/// number without first finding the run it sits in. **Stopping is not accepting**: the
/// caller decides, and its rule is §7.2.3's, that the byte which stopped the scan must also
/// end the token. Everything else — an exponent, a repeated sign, a second period, a byte
/// that is no digit — therefore still reaches the caller's standing `parse`-then-salvage
/// path, so every malformed form keeps the reading it always had.
///
/// The digit count is refused past fifteen (below), and nothing else is refused: a run of
/// signs and points with no digit in it returns `None`, because both of the clause's forms
/// are "one or more decimal digits".
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
              at most 15 decimal digits and cannot overflow a u64; `read` is bounded by \
              `body.len()` because the loop ends when `get` returns nothing, `sign` is 0 \
              or 1, and `read - at - 1` is non-negative because `at` indexes a byte the \
              loop consumed before it"
)]
fn fixed_format_number(raw: &[u8]) -> Option<(Fixed, usize)> {
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
    let sign = raw.len() - body.len();

    // **Where the period is, rather than how many digits have followed it.** The loop used
    // to carry `fraction: Option<usize>` and increment it inside the digit arm, which is a
    // load, a test and a store on every digit of every number a page states — 104.5 million
    // of them on `doc/todo/44`'s witness. The index of the period says the same thing once,
    // because everything between it and the end of the scan is a digit by construction.
    // Worth 69.8 M instructions of that page, measured against this same loop keeping the
    // accumulator (ADR 0424).
    //
    // **And the loop is indexed rather than iterated, which is not a style choice.** It has
    // to say where it stopped, and a slice iterator cannot: `for &byte in body` with a
    // `read += 1` beside it is two cursors the compiler then has to keep in step, and it
    // does not — the same function, the same arithmetic, spelled that way costs
    // **750 M more instructions** on that page, 6.5% of interpreting it, almost all of it
    // in code with no line of this file to attribute it to. That is ADR 0370's finding
    // arriving from the other side: there a slice iterator replacing an index measured
    // *worse* for the same reason.
    let mut mantissa: u64 = 0;
    let mut digits = 0usize;
    let mut point: Option<usize> = None;
    let mut read = 0usize;
    while let Some(&byte) = body.get(read) {
        match byte {
            b'0'..=b'9' => {
                digits += 1;
                if digits > 15 {
                    return None;
                }
                mantissa = mantissa * 10 + u64::from(byte - b'0');
            }
            b'.' if point.is_none() => point = Some(read),
            // Anything else — a second period included — is where this grammar ends. The
            // caller reads that byte and decides whether the token ended with it.
            _ => break,
        }
        read += 1;
    }
    // "One or more decimal digits" in both forms, so a run stating none is not this
    // function's to read — the caller returns such a run as the keyword it lexically is.
    if digits == 0 {
        return None;
    }
    let taken = sign + read;

    match point {
        // Fits because the mantissa holds at most 15 decimal digits.
        None => i64::try_from(mantissa).ok().map(|magnitude| {
            (
                Fixed::Integer(if negative { -magnitude } else { magnitude }),
                taken,
            )
        }),
        Some(at) => {
            // Every byte after the period and before where the scan stopped is a digit, so
            // this is the count the old accumulator kept.
            let count = read - at - 1;
            #[expect(
                clippy::cast_precision_loss,
                reason = "the mantissa is below 2^53, where every integer is exactly \
                          representable — the doc comment's exactness argument rests on it"
            )]
            let magnitude = mantissa as f64 / POWERS_OF_TEN.get(count).copied()?;
            Some((
                Fixed::Real(if negative { -magnitude } else { magnitude }),
                taken,
            ))
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
    cleaned.parse::<f64>().ok().map(within_the_representation)
}

/// Brings a magnitude the file states, and a double cannot hold, inside the representation.
///
/// ISO 32000-2 §7.3.3 anticipates this and says what a processor's answer rests on:
///
/// > The range and precision of numbers may be limited by the internal representations used
/// > in the computer on which the PDF processor is running; Annex C, "Advice on maximising
/// > portability", gives these limits for typical implementations.
///
/// So *having* a limit is the clause's own permission, and what is left to decide is which
/// value the limit is. This returns the largest finite double carrying the sign the file
/// wrote — the nearest value the representation holds to the one stated. Annex C is
/// informative and states no figure: its Table C.1 says only that reals are "often" IEEE 754
/// single or double, which is the representation this bound is of.
///
/// **It returned zero until the eight-hundredth session, and zero is the worst value
/// available.** It is the smallest magnitude where the largest was written, it inverts the
/// ordering of every comparison the number then takes part in, and — unlike a refusal — it
/// *draws*: a coordinate at the origin, a font size of nought, a width of nothing, in place of
/// a mark the producer put off the sheet. That is the plausible fallback trap 5 forbids and
/// the same shape ADR 0303 took out of the run holding no digit at all, surviving one
/// condition below it because no corpus document exercises it — 964 pdf.js documents and an
/// 8300-document sample of the crawl state over seven hundred million runs in §7.3.3's own two
/// forms and **not one** of them overflows a double
/// (`pdf-model/examples/numeric_form_census`). A refusal is not the
/// alternative here and a `Keyword` would be the wrong one: the run in front of it is often a
/// perfectly conforming number, four hundred decimal digits and no sign of PostScript, and
/// saying "this is not a number" of it would be false.
///
/// A magnitude too *small* for a double is not this case and is not touched: zero is the
/// correctly rounded value of `1e-400` rather than a substitute for it.
fn within_the_representation(value: f64) -> f64 {
    if value.is_finite() {
        return value;
    }
    // An infinity, and never a `NaN`: every caller arrives from a parse of a run holding a
    // decimal digit, and the two spellings that parse to `NaN` hold none — they are returned
    // as keywords one condition earlier. The sign test is still total, so a `NaN` that found a
    // route here would be bounded rather than propagated.
    if value.is_sign_negative() {
        -f64::MAX
    } else {
        f64::MAX
    }
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

    /// **A long number states a value, not a different value.**
    ///
    /// [`super::fixed_format_number`]'s accumulator multiplies a `u64` by ten per digit, and
    /// the whole reason it refuses past fifteen digits is that the sixteenth could carry the
    /// mantissa out of the range an `f64` states exactly. A parser that let it run instead —
    /// `wrapping_mul(10)`, which is what `hayro`'s issue 1341 found in theirs — turns a
    /// coordinate into a number modulo 2^64, silently and without any bound on the error.
    ///
    /// CAD drivers emit coordinates at full `f64` precision, so seventeen significant digits
    /// plus the trailing noise of a decimal expansion is an ordinary thing for a content
    /// stream to contain. §7.3.3 states no length limit, and Annex C's figures are
    /// informative, so the only requirement here is the clause's own: the token states a
    /// number and the number is the one written.
    ///
    /// The guard is the *fall-through*, not the fast path — every case below is longer than
    /// fifteen digits and therefore reaches `str::parse`. What is asserted is correct
    /// rounding, which is what `f64::from_str` gives and what wrapping arithmetic cannot.
    #[test]
    fn a_number_longer_than_the_fast_path_is_still_the_number_written() {
        // 23 digits. Wrapping a u64 would give 12345678901234567890123 mod 2^64, which is
        // 3479235573345971275 — nowhere near the value and, crucially, not even the same
        // order of magnitude.
        assert_eq!(
            tokens(b"12345678901234567890123"),
            vec![Token::Real(1.234_567_890_123_456_8e22)]
        );
        // u64::MAX exactly: the value a wrapping accumulator lands on last before it starts
        // over, and the one an `as i64` cast turns into -1.
        assert_eq!(
            tokens(b"18446744073709551615"),
            vec![Token::Real(1.844_674_407_370_955_2e19)]
        );
        // 2^64 itself, the first value that wraps to zero.
        assert_eq!(
            tokens(b"18446744073709551616"),
            vec![Token::Real(1.844_674_407_370_955_2e19)]
        );
        // A real whose digits are all fractional: the count includes them, so this refuses
        // the fast path too, and it must come back as the tiny number rather than as 1.
        assert_eq!(tokens(b"0.000000000000000000001"), vec![Token::Real(1e-21)]);
        // Seventeen significant digits either side of the point, the CAD shape.
        assert_eq!(
            tokens(b"123456789012345678901.5"),
            vec![Token::Real(1.234_567_890_123_456_8e20)]
        );
        // An integer that is long but still inside `i64` stays an integer, exactly. This is
        // the boundary the two paths meet at: sixteen digits, past the fast path's fifteen.
        assert_eq!(
            tokens(b"1234567890123456"),
            vec![Token::Integer(1_234_567_890_123_456)]
        );
        assert_eq!(
            tokens(b"-9223372036854775808"),
            vec![Token::Integer(i64::MIN)]
        );
    }

    /// **§7.3.3's exponential format is the writer's `shall not`, and the value is still read.**
    ///
    /// > A PDF writer shall not use the PostScript language syntax for numbers with non-decimal
    /// > radices (such as 16#FFFE) or in exponential format (such as 6.02E23).
    ///
    /// Errata Collection 3's Issue #327 closes the grammar the other way round, with a railroad
    /// diagram of each form above its EXAMPLE: an optional sign, decimal digits, one PERIOD for
    /// the real form, and no other production in either figure. So a run carrying an exponent is
    /// outside both forms and this reader takes it anyway — the clause places its prohibition on
    /// the producer, states nothing for a reader, and glosses the value in the same sentence it
    /// forbids the spelling. `sci-notation.pdf` writes `/F1 1e2 Tf`, which is a font size of a
    /// hundred or nothing at all.
    #[test]
    fn the_exponential_format_is_read_as_the_value_the_clause_glosses() {
        // The clause's own example, and the corpus's.
        assert_eq!(tokens(b"6.02E23"), vec![Token::Real(6.02e23)]);
        assert_eq!(tokens(b"1e2"), vec![Token::Real(100.0)]);
        // Both signs, in both places, and a leading point — the forms `str::parse` composes.
        assert_eq!(tokens(b"-1e-2"), vec![Token::Real(-0.01)]);
        assert_eq!(tokens(b"+1E+2"), vec![Token::Real(100.0)]);
        assert_eq!(tokens(b".5e1"), vec![Token::Real(5.0)]);
        // An exponent with no digits after it is not this form at all: the salvage takes the
        // leading number and drops the rest, which is what it does for every other malformed run.
        assert_eq!(tokens(b"1e"), vec![Token::Integer(1)]);
    }

    /// **A magnitude beyond a double is the representation's limit, never zero.**
    ///
    /// §7.3.3 permits the limit and leaves its value open:
    ///
    /// > The range and precision of numbers may be limited by the internal representations used
    /// > in the computer on which the PDF processor is running; Annex C, "Advice on maximising
    /// > portability", gives these limits for typical implementations.
    ///
    /// Zero is the one answer that is both wrong and quiet — the smallest magnitude in place of
    /// the largest, and a value that draws. The first assertion below is the one that matters
    /// most, because its input is a **conforming** §7.3.3 integer: four hundred decimal digits
    /// with no sign of PostScript in them, which this reader used to hand back as `0`.
    #[test]
    fn a_magnitude_beyond_a_double_is_the_limit_rather_than_zero() {
        let four_hundred_nines = vec![b'9'; 400];
        assert_eq!(tokens(&four_hundred_nines), vec![Token::Real(f64::MAX)]);
        let mut negative = vec![b'-'];
        negative.extend_from_slice(&four_hundred_nines);
        assert_eq!(tokens(&negative), vec![Token::Real(-f64::MAX)]);
        // The same magnitude in the real form, so the `.` route overflows too.
        let mut fractional = four_hundred_nines.clone();
        fractional.extend_from_slice(b".5");
        assert_eq!(tokens(&fractional), vec![Token::Real(f64::MAX)]);
        // And through the exponent, which is how a file would ever reach it in practice.
        assert_eq!(tokens(b"1e400"), vec![Token::Real(f64::MAX)]);
        assert_eq!(tokens(b"-1e400"), vec![Token::Real(-f64::MAX)]);
        // A magnitude too *small* is not the same question: zero is the correctly rounded
        // value of this decimal rather than a substitute for it.
        assert_eq!(tokens(b"1e-400"), vec![Token::Real(0.0)]);
        // The salvage path reaches the same bound: `--` collapses to one sign, and what is left
        // still overflows.
        let mut salvaged = vec![b'-', b'-'];
        salvaged.extend_from_slice(&four_hundred_nines);
        assert_eq!(tokens(&salvaged), vec![Token::Real(-f64::MAX)]);
    }

    /// **`inf` and `NaN` are `str::parse`'s and not §7.3.3's, and they never reach the parse.**
    ///
    /// Both forms of the clause are "one or more decimal digits", so a run holding none is the
    /// keyword it lexically is (ADR 0303) — which is what keeps `f64::from_str`'s two extra
    /// spellings out of the numbers this lexer hands back. The parser refuses such a token where
    /// an object was expected and the interpreter reports it as an operator it does not know.
    #[test]
    fn the_spellings_str_parse_has_and_the_clause_does_not_are_keywords() {
        for spelling in [
            &b"inf"[..],
            b"Inf",
            b"infinity",
            b"-inf",
            b"+inf",
            b"NaN",
            b"nan",
            b"-NaN",
        ] {
            assert_eq!(
                tokens(spelling),
                vec![Token::Keyword(spelling)],
                "{} is not a numeric object",
                String::from_utf8_lossy(spelling)
            );
        }
    }

    /// **§7.2.3's token boundary: a run of regular characters is one token.**
    ///
    /// `f` is a regular character, so `5f` is a single token — not the number 5 followed by
    /// the `f` (fill) operator. `hayro`'s issue 994 is a hand-built content stream that
    /// distinguishes the two readings by whether a red square appears: a lexer that splits
    /// the run fills the rectangle, one that does not draws nothing.
    ///
    /// This tree does not split it, which is what the assertion pins: the whole run is
    /// consumed, so the `f` never reaches the interpreter as an operator and no fill happens.
    /// What it comes back *as* is a salvaged `5` rather than a keyword — [`super::Lexer`]
    /// reads `12pt` as 12 deliberately, and ADR 0303 scoped its correction to runs stating no
    /// digit at all. That leniency is why the second assertion is here: it would be a real
    /// regression for the leading value to be salvaged *and* the trailing letters to be
    /// re-offered as a token.
    #[test]
    fn a_digit_run_ending_in_letters_is_one_token() {
        assert_eq!(tokens(b"5f"), vec![Token::Integer(5)]);
        assert_eq!(tokens(b"12pt"), vec![Token::Integer(12)]);
        // The same bytes with the delimiter §7.2.3 asks for are two tokens, and *that* fills.
        assert_eq!(
            tokens(b"5 f"),
            vec![Token::Integer(5), Token::Keyword(b"f")]
        );
    }

    /// **A token ends where §7.2.3 says it does, whichever path read it.**
    ///
    /// [`super::Lexer::read_number`] parses §7.3.3's fixed format straight off the cursor,
    /// so that parse stops at the first byte outside *its* grammar — which is not the same
    /// place the token ends. `1e5`, `1,5` and `5f` are each one run of regular characters
    /// (§7.2.3) and therefore one token, and a fast path that took its own stopping point
    /// for the token's would hand the interpreter an `e`, a `,` or an `f` as an operator.
    ///
    /// Asserted over every byte value rather than over a list, because the discriminating
    /// input is exactly "a byte that is regular but is not part of a number", and which
    /// bytes those are is a table this test must not restate. The invariant is the clause's:
    /// after lexing `1<byte>`, either the whole input was consumed as one token, or the byte
    /// is one §7.2.3 classifies as ending a token.
    #[test]
    fn a_number_does_not_end_in_the_middle_of_a_run() {
        for byte in 0u8..=255 {
            let input = [b'1', byte];
            let mut lexer = Lexer::new(&input);
            assert!(lexer.next_token().is_some(), "{byte:#04x} lexed nothing");
            let ended = lexer.position();
            if super::is_regular(byte) {
                assert_eq!(
                    ended,
                    2,
                    "{byte:#04x} is a regular character, so `1{}` is one token",
                    char::from(byte)
                );
            } else {
                assert_eq!(
                    ended, 1,
                    "{byte:#04x} ends a token, so the number is the first of two"
                );
            }
        }
    }

    /// The other side of the rule above: a delimiter needs no white space in front of it,
    /// and the number before one is a whole number rather than a salvaged prefix.
    #[test]
    fn a_number_against_a_delimiter_is_a_whole_number() {
        assert_eq!(
            tokens(b"[1.5]"),
            vec![Token::ArrayOpen, Token::Real(1.5), Token::ArrayClose]
        );
        // And the counter-example that says the rule is §7.2.3's rather than arithmetic:
        // `-` is a regular character, so `1.5-2` is one run and one token, salvaged to its
        // leading value exactly as it was before the fast path read numbers off the cursor.
        assert_eq!(
            tokens(b"[1.5-2]"),
            vec![Token::ArrayOpen, Token::Real(1.5), Token::ArrayClose]
        );
        assert_eq!(
            tokens(b"3(x)"),
            vec![Token::Integer(3), Token::String(b"x".to_vec())]
        );
        assert_eq!(
            tokens(b"4%c\n5"),
            vec![Token::Integer(4), Token::Integer(5)],
            "a comment is a delimiter's business and ends the number before it"
        );
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
    ///
    /// # Under Miri it is a sample rather than a sweep, and the sample is the whole change
    ///
    /// Exhaustive is 1.8 million lexes of a freshly allocated string, and the interpreter is
    /// four orders of magnitude slower than the processor — which is most of an hour of the
    /// `nightly` job's ceiling for a test whose *subject* is a value rather than a memory
    /// operation. The sample keeps every shape the sweep has: the first hundred, so that one and
    /// two digits and the zero whose sign `to_bits` sees are all present, and then a prime stride
    /// through the rest for three, four and five — under all three signs, with the point in every
    /// position and absent. What is given up is the exhaustiveness — the claim that *no*
    /// five-digit string disagrees — which is a claim about arithmetic that the interpreter was
    /// never the instrument for, and which the same test makes in full on every other gate.
    /// Session 630; ADR 0463.
    #[test]
    fn the_fixed_format_parse_agrees_with_the_standard_library() {
        let sampled = |value: &u32| !cfg!(miri) || *value < 100 || value.is_multiple_of(997);
        for value in (0..=99_999u32).filter(sampled) {
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

    /// §7.3.4.2's end-of-line rule, which nothing here asked for until the
    /// seven-hundred-and-seventy-first session.
    ///
    /// The clause makes an unescaped end-of-line marker *one byte*, 0Ah, whichever of the three
    /// forms it was written in — so the four cases that matter are a bare CARRIAGE RETURN, a
    /// bare LINE FEED, the pair in that order, and the pair in the other order, which is two
    /// markers rather than one. The escaped forms are the control: Table 3 gives `\r` its own
    /// byte and a REVERSE SOLIDUS before a marker is a continuation, and neither is what this
    /// rule is about.
    #[test]
    fn an_unescaped_end_of_line_in_a_literal_string_is_one_line_feed() {
        assert_eq!(
            tokens(b"(a\rb)"),
            vec![Token::String(b"a\nb".to_vec())],
            "a bare carriage return is a line feed"
        );
        assert_eq!(
            tokens(b"(a\nb)"),
            vec![Token::String(b"a\nb".to_vec())],
            "a bare line feed stays one line feed"
        );
        assert_eq!(
            tokens(b"(a\r\nb)"),
            vec![Token::String(b"a\nb".to_vec())],
            "carriage return and line feed are one marker, so one byte"
        );
        assert_eq!(
            tokens(b"(a\n\rb)"),
            vec![Token::String(b"a\n\nb".to_vec())],
            "line feed then carriage return is two markers, so two bytes"
        );
        assert_eq!(
            tokens(b"(a\\rb)"),
            vec![Token::String(b"a\rb".to_vec())],
            "an escaped carriage return is Table 3's byte and is left alone"
        );
        assert_eq!(
            tokens(b"(a\\\r\nb)"),
            vec![Token::String(b"ab".to_vec())],
            "a backslash before the marker is still the line continuation"
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

    /// §7.2.3's two conditional delimiters, outside the one condition that makes them delimit.
    ///
    /// Table 2 lists ten delimiter characters and the clause's own sentence scopes two of them to
    /// type 4 PostScript calculator functions, which nothing lexed here is — so `{` and `}` are
    /// regular characters and end no token. What that decides is a *name*: `/A{B}` is one name of
    /// four bytes, where a lexer holding Table 2's list unconditionally reads a two-byte name and
    /// three tokens after it, and the two readings disagree about the key a dictionary states.
    #[test]
    fn a_brace_is_a_regular_character_outside_a_type_4_program() {
        assert_eq!(
            tokens(b"/A{B} 1"),
            vec![Token::Name(b"A{B}".to_vec()), Token::Integer(1)]
        );
        // The same bytes a type 4 program is made of, lexed here: one run apiece, because the
        // braces join their neighbours instead of standing alone. `pdf_model::function` splits a
        // program itself and is the only place the other reading is the right one.
        assert_eq!(
            tokens(b"{2 mul}"),
            vec![Token::Keyword(b"{2"), Token::Keyword(b"mul}")]
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
