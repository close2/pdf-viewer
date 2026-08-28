//! Assembling tokens into objects.
//!
//! # Bounded by construction
//!
//! Rust prevents memory corruption here, but not resource exhaustion: a few hundred bytes
//! of `[[[[[[...` would recurse until the stack ran out, and `<</A 1>>` repeated can ask
//! for unbounded allocation. Every recursive construct is therefore depth-limited and
//! every container length-limited, with the limits in [`Limits`] rather than scattered
//! through the code. See `CLAUDE.md` principle 3.
//!
//! Exceeding a limit is an error naming the limit, not a panic and not a truncated
//! object: silently returning a shortened array would render a wrong page and report
//! success.

use std::sync::Arc;

use crate::error::{SyntaxError, SyntaxResult};
use crate::lexer::{Lexer, Token};
use crate::object::{Dictionary, Name, Object, ObjectId, Stream};

/// Resource bounds applied while parsing.
///
/// The defaults are far above what any legitimate document needs and far below what
/// exhausts a machine. They exist to convert a hostile file into an error rather than a
/// crash, so they are generous on purpose: rejecting an unusual but valid document would
/// be a worse failure than accepting a large one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum nesting depth of arrays and dictionaries.
    pub max_depth: usize,
    /// Maximum number of elements in one array.
    pub max_array_len: usize,
    /// Maximum number of entries in one dictionary.
    pub max_dict_len: usize,
    /// Maximum length of one string, in bytes.
    pub max_string_len: usize,
    /// Maximum length of one stream, in bytes, raw or decoded.
    ///
    /// **Both, and that is the point of one number rather than two**: §7.3.8.2 makes `/Length`
    /// a statement about the bytes in the file, and a filter turns those into more of them, so
    /// a bound on the first alone bounds nothing a decompression bomb does.
    ///
    /// It is also the bound on a page's whole `/Contents`, because Table 31 says the parts of
    /// the array are one stream — see `pdf_model::Page::content_with_report`.
    pub max_stream_len: usize,
}

impl Limits {
    /// The default bounds.
    pub const DEFAULT: Self = Self {
        // Deeply nested structures do occur — nested arrays in shading functions — but
        // nothing legitimate approaches this.
        max_depth: 256,
        max_array_len: 1 << 20,
        max_dict_len: 1 << 16,
        max_string_len: 1 << 26,
        // **One gibibyte.** Two gibibytes until the four-hundred-and-seventy-first session,
        // where it contradicted the ceiling the confined worker runs under, and the new figure
        // is bounded from both sides rather than chosen:
        //
        // - **From above by the ceiling.** `pdf_sandbox`'s `INTERPRETER_ADDRESS_SPACE_LIMIT` is
        //   4 GiB, of which `MAX_PIXELS` x 4 bytes = 1 GiB is the raster's. Decoding costs
        //   about *twice* the decoded length before the bytes are handed over — the inflate loop
        //   doubles its buffer because it cannot know where the stream ends, and `Arc<[u8]>` is
        //   then a copy of the result. So a bound of L costs about 2L, and 2L has to fit in the
        //   3 GiB the raster leaves. At 2 GiB it did not: one stream could ask for the whole
        //   ceiling and leave nothing to draw with.
        // - **From below by what documents contain.** The largest decoded stream in
        //   **5 047 187 streams over 65 967 crawled documents** is 483.84 MiB, and one stream in
        //   five million passes 256 MiB (`content_budget_census`). A gibibyte is twice the
        //   largest real one and refuses none of them.
        //
        // ADR 0306. **The arithmetic above was right and the code disobeyed it for two rounds**,
        // which is why this comment no longer names `read_to_end` — that adapter left in ADR
        // 0343, and the loop that replaced it grew through `Vec::reserve`, whose amortised
        // `max(2 x capacity, len + additional)` doubled the buffer *past* this number. A decode
        // of L cost up to 3L and a bomb up to 2 x this bound: 1811 MB measured for a 1.85 MB
        // file. ADR 0354 made the code cost what this derivation assumed, and moved no number.
        max_stream_len: 1 << 30,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Parses objects from PDF bytes.
#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    limits: Limits,
    depth: usize,
    /// Where the data of the last stream this parser read begins.
    ///
    /// Kept because §7.3.8.2 lets `/Length` be an indirect reference and a parser has no
    /// document to resolve one with, so the *file's* statement of a stream's extent can only be
    /// applied one layer up — and applying it needs the offset the guess was made at. See
    /// [`crate::Document`]'s `with_stated_length`. ADR 0366.
    stream_data_at: Option<usize>,
}

impl<'a> Parser<'a> {
    /// Creates a parser over `input` with the default limits.
    #[must_use]
    pub fn new(input: &'a [u8]) -> Self {
        Self::with_limits(input, Limits::DEFAULT)
    }

    /// Creates a parser with explicit limits.
    #[must_use]
    pub fn with_limits(input: &'a [u8], limits: Limits) -> Self {
        Self {
            lexer: Lexer::new(input),
            limits,
            depth: 0,
            stream_data_at: None,
        }
    }

    /// Creates a parser positioned at `offset`.
    #[must_use]
    pub fn at(input: &'a [u8], offset: usize, limits: Limits) -> Self {
        Self {
            lexer: Lexer::at(input, offset),
            limits,
            depth: 0,
            stream_data_at: None,
        }
    }

    /// Returns the current byte offset.
    #[must_use]
    pub fn position(&self) -> usize {
        self.lexer.position()
    }

    /// Moves to `offset`.
    pub fn seek(&mut self, offset: usize) {
        self.lexer.seek(offset);
    }

    /// Returns the input being parsed.
    #[must_use]
    pub fn input(&self) -> &'a [u8] {
        self.lexer.input()
    }

    /// Returns the limits in force.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Where the data of the last stream this parser read begins, if it read one.
    ///
    /// `None` until a stream is parsed. What it is for is in [`Self::stream_data_at`]'s field
    /// documentation: an indirect `/Length` is a statement only a document can read, and
    /// honouring it needs the offset this parser measured its own guess from.
    #[must_use]
    pub(crate) fn stream_data_at(&self) -> Option<usize> {
        self.stream_data_at
    }

    /// Parses the next object.
    ///
    /// # Errors
    ///
    /// [`SyntaxError::UnexpectedEnd`] at end of input, [`SyntaxError::LimitExceeded`] when
    /// a bound in [`Limits`] is reached, and [`SyntaxError::Unexpected`] for a token that
    /// cannot begin an object.
    pub fn parse_object(&mut self) -> SyntaxResult<Object> {
        let token = self.lexer.next_token().ok_or(SyntaxError::UnexpectedEnd {
            at: self.lexer.position(),
            expected: "an object",
        })?;
        self.parse_object_from(token)
    }

    /// Parses an object whose first token has already been read.
    fn parse_object_from(&mut self, token: Token<'a>) -> SyntaxResult<Object> {
        match token {
            Token::Integer(value) => Ok(self.integer_or_reference(value)),
            Token::Real(value) => Ok(Object::Real(value)),
            Token::Name(bytes) => Ok(Object::Name(Name::new(bytes))),
            Token::String(bytes) => {
                if bytes.len() > self.limits.max_string_len {
                    return Err(SyntaxError::LimitExceeded {
                        at: self.lexer.position(),
                        limit: "max_string_len",
                    });
                }
                Ok(Object::String(Arc::from(bytes.as_slice())))
            }
            Token::ArrayOpen => self.parse_array(),
            Token::DictOpen => self.parse_dictionary_or_stream(),
            Token::Keyword(word) => match word {
                b"true" => Ok(Object::Boolean(true)),
                b"false" => Ok(Object::Boolean(false)),
                b"null" => Ok(Object::Null),
                _ => Err(SyntaxError::Unexpected {
                    at: self.lexer.position(),
                    found: String::from_utf8_lossy(word).into_owned(),
                    expected: "an object",
                }),
            },
            Token::ArrayClose | Token::DictClose => Err(SyntaxError::Unexpected {
                at: self.lexer.position(),
                found: format!("{token:?}"),
                expected: "an object",
            }),
        }
    }

    /// Disambiguates `1`, `1 0 R` and `1 0 obj`.
    ///
    /// Requires lookahead of two tokens, and must restore the cursor when they turn out
    /// not to form a reference — `[1 2 3]` would otherwise lose elements.
    fn integer_or_reference(&mut self, first: i64) -> Object {
        let rewind = self.lexer.position();
        let mut probe = self.lexer.clone();

        if let (Some(Token::Integer(generation)), Some(Token::Keyword(word))) =
            (probe.next_token(), probe.next_token())
            && word == b"R"
            && let (Ok(number), Ok(generation)) = (u32::try_from(first), u16::try_from(generation))
        {
            self.lexer = probe;
            return Object::Reference(ObjectId::new(number, generation));
        }
        // Falling through covers both "not a reference" and a reference whose number or
        // generation is out of range: the latter cannot name a real object, so it is
        // treated as the integer it lexically is, leaving the following tokens to be
        // reported where they appear.

        self.lexer.seek(rewind);
        Object::Integer(first)
    }

    fn parse_array(&mut self) -> SyntaxResult<Object> {
        self.enter()?;
        let mut items = Vec::new();

        loop {
            let Some(token) = self.lexer.next_token() else {
                // Truncated input. Returning what was collected loses the rest of the
                // document silently, so this is an error.
                self.depth = self.depth.saturating_sub(1);
                return Err(SyntaxError::UnexpectedEnd {
                    at: self.lexer.position(),
                    expected: "']'",
                });
            };
            if token == Token::ArrayClose {
                break;
            }
            if items.len() >= self.limits.max_array_len {
                self.depth = self.depth.saturating_sub(1);
                return Err(SyntaxError::LimitExceeded {
                    at: self.lexer.position(),
                    limit: "max_array_len",
                });
            }
            match self.parse_object_from(token) {
                Ok(object) => items.push(object),
                Err(error) => {
                    self.depth = self.depth.saturating_sub(1);
                    return Err(error);
                }
            }
        }

        self.depth = self.depth.saturating_sub(1);
        Ok(Object::Array(items))
    }

    fn parse_dictionary_or_stream(&mut self) -> SyntaxResult<Object> {
        let dict = self.parse_dictionary_body()?;

        // `stream` may follow a dictionary, making it a stream object.
        let rewind = self.lexer.position();
        let mut probe = self.lexer.clone();
        if probe.next_token() == Some(Token::Keyword(b"stream")) {
            self.lexer = probe;
            return self.parse_stream_data(dict);
        }
        self.lexer.seek(rewind);

        Ok(Object::Dictionary(dict))
    }

    fn parse_dictionary_body(&mut self) -> SyntaxResult<Dictionary> {
        self.enter()?;
        let mut dict = Dictionary::new();

        let result = loop {
            let Some(token) = self.lexer.next_token() else {
                break Err(SyntaxError::UnexpectedEnd {
                    at: self.lexer.position(),
                    expected: "'>>'",
                });
            };
            match token {
                Token::DictClose => break Ok(()),
                Token::Name(bytes) => {
                    if dict.len() >= self.limits.max_dict_len {
                        break Err(SyntaxError::LimitExceeded {
                            at: self.lexer.position(),
                            limit: "max_dict_len",
                        });
                    }
                    match self.parse_object() {
                        Ok(value) => {
                            // ISO 32000-2 §7.3.7:
                            //
                            // > A dictionary entry whose value is null (see 7.3.9, "Null
                            // > object") shall be treated the same as if the entry does not
                            // > exist.
                            //
                            // Dropped here rather than at every reader, because "the same
                            // as" leaves nothing for one to distinguish. `Document::get_key`
                            // already answers `Null` for an absent key and for a reference
                            // that resolves to nothing (§7.3.10), so this closes the one
                            // remaining spelling — a *direct* null, which `Dictionary::get`,
                            // `len` and `iter` would otherwise show.
                            if matches!(value, Object::Null) {
                                continue;
                            }
                            // A duplicate key is malformed — "[m]ultiple entries in the same
                            // dictionary shall not have the same key" — and the clause
                            // states no recovery. Nor can one be derived: the entries "shall
                            // be unordered even though an arbitrary order may be imposed
                            // upon them when written in a file. That ordering shall be
                            // ignored", so a rule preferring the first or the last would be
                            // reading the very order the clause discards. **This is a
                            // documented choice, not a derivation** (`CLAUDE.md` principle
                            // 5): first wins. poppler agrees, which is evidence that the
                            // choice is unsurprising and is not a reason for it.
                            let key = Name::new(bytes);
                            if dict.get_by_name(&key).is_none() {
                                dict.insert(key, value);
                            }
                        }
                        Err(error) => break Err(error),
                    }
                }
                // A non-name where a key belongs. Skipped rather than fatal: files with a
                // stray value between entries are recoverable, and the alternative loses
                // the whole dictionary.
                _ => {}
            }
        };

        self.depth = self.depth.saturating_sub(1);
        result.map(|()| dict)
    }

    /// Reads stream data, having consumed the `stream` keyword.
    ///
    /// The length comes from `/Length` when it is a direct integer that agrees with the
    /// data, and otherwise from searching for `endstream`. Both paths are needed: an
    /// indirect `/Length` cannot be resolved without the document, and a wrong `/Length`
    /// is one of the most common corruptions in real files.
    fn parse_stream_data(&mut self, dict: Dictionary) -> SyntaxResult<Object> {
        // The keyword is followed by CRLF or LF — but not CR alone, per the specification.
        // Tolerate CR alone anyway, since files contain it.
        match self.lexer.peek() {
            Some(b'\r') => {
                self.lexer.seek(self.lexer.position().saturating_add(1));
                if self.lexer.peek() == Some(b'\n') {
                    self.lexer.seek(self.lexer.position().saturating_add(1));
                }
            }
            Some(b'\n') => self.lexer.seek(self.lexer.position().saturating_add(1)),
            _ => {}
        }

        let start = self.lexer.position();
        let input = self.lexer.input();

        let declared = dict
            .get("Length")
            .and_then(Object::as_integer)
            .and_then(|value| usize::try_from(value).ok());

        let end = match declared {
            // Trust the declared length only if `endstream` actually follows it. That
            // check is what turns a corrupt length into recovery instead of garbage.
            Some(length)
                if start.saturating_add(length) <= input.len()
                    && endstream_follows(input, start.saturating_add(length)) =>
            {
                start.saturating_add(length)
            }
            _ => find_endstream(input, start).unwrap_or(input.len()),
        };

        let length = end.saturating_sub(start);
        if length > self.limits.max_stream_len {
            return Err(SyntaxError::LimitExceeded {
                at: start,
                limit: "max_stream_len",
            });
        }

        let data = input.get(start..end).unwrap_or_default();
        self.stream_data_at = Some(start);
        self.lexer.seek(end);

        // Consume `endstream` if present. Its absence is not fatal: the data has already
        // been delimited, and rejecting the object would lose a page over a missing
        // keyword.
        let rewind = self.lexer.position();
        if self.lexer.next_token() != Some(Token::Keyword(b"endstream")) {
            self.lexer.seek(rewind);
        }

        Ok(Object::Stream(Arc::new(Stream {
            dict,
            data: Arc::from(data),
            // Nothing here knows about encryption; `Document` sets this when it decrypts.
            decryption_failed: false,
        })))
    }

    /// Parses `<number> <generation> obj ... endobj` at the cursor.
    ///
    /// # Errors
    ///
    /// [`SyntaxError::Unexpected`] if the header is not an indirect object header, plus
    /// anything [`Self::parse_object`] reports.
    pub fn parse_indirect_object(&mut self) -> SyntaxResult<(ObjectId, Object)> {
        let at = self.lexer.position();

        let Some(Token::Integer(number)) = self.lexer.next_token() else {
            return Err(SyntaxError::Unexpected {
                at,
                found: "not an object number".to_owned(),
                expected: "an indirect object header",
            });
        };
        let Some(Token::Integer(generation)) = self.lexer.next_token() else {
            return Err(SyntaxError::Unexpected {
                at,
                found: "not a generation number".to_owned(),
                expected: "an indirect object header",
            });
        };
        if self.lexer.next_token() != Some(Token::Keyword(b"obj")) {
            return Err(SyntaxError::Unexpected {
                at,
                found: "missing 'obj'".to_owned(),
                expected: "an indirect object header",
            });
        }

        let id = ObjectId::new(
            u32::try_from(number).map_err(|_| SyntaxError::Unexpected {
                at,
                found: format!("object number {number}"),
                expected: "a number within range",
            })?,
            u16::try_from(generation).unwrap_or(0),
        );

        let object = self.parse_object()?;

        // `endobj` is frequently missing or misplaced. The object is already complete, so
        // its absence is tolerated.
        let rewind = self.lexer.position();
        if self.lexer.next_token() != Some(Token::Keyword(b"endobj")) {
            self.lexer.seek(rewind);
        }

        Ok((id, object))
    }

    /// Increments depth, failing if the limit is reached.
    fn enter(&mut self) -> SyntaxResult<()> {
        self.depth = self.depth.saturating_add(1);
        if self.depth > self.limits.max_depth {
            return Err(SyntaxError::LimitExceeded {
                at: self.lexer.position(),
                limit: "max_depth",
            });
        }
        Ok(())
    }
}

/// Returns `true` if `endstream` appears at `offset`, allowing leading whitespace.
pub(crate) fn endstream_follows(input: &[u8], offset: usize) -> bool {
    let rest = input.get(offset..).unwrap_or_default();
    let trimmed = rest
        .iter()
        .position(|&byte| !crate::lexer::is_whitespace(byte))
        .unwrap_or(rest.len());
    rest.get(trimmed..)
        .unwrap_or_default()
        .starts_with(b"endstream")
}

/// Finds the offset of the `endstream` keyword at or after `from`.
///
/// Returns the offset of the data end, excluding the end-of-line that precedes the
/// keyword — that byte belongs to the delimiter, not the data, and including it corrupts
/// every stream recovered this way.
///
/// # Why it looks for one byte before comparing nine
///
/// This search is not an error path. Table 5 makes `/Length` "shall be an indirect
/// reference" for a producer that does not know the length until the data is written, and a
/// parser cannot follow one (see [`Document::with_stated_length`]) — so on such a file this
/// runs on the launch path, from the stream's first byte to wherever `endstream` is.
///
/// `windows(9).position(..)` compares nine bytes at every offset and measured **five
/// instructions a byte**: 446 M of the 11 471 M it takes to interpret page one of
/// `doc/todo/44`'s witness, whose page content is one 49.7 MB stream with an indirect
/// length. Looking for `e` and comparing the other eight only where one is found is the
/// same answer for a fraction of the work, because compressed stream data holds about one
/// `e` in 256. ADR 0424 has the A/B.
///
/// [`Document::with_stated_length`]: crate::Document
fn find_endstream(input: &[u8], from: usize) -> Option<usize> {
    const NEEDLE: &[u8] = b"endstream";
    let haystack = input.get(from..)?;

    let mut at = 0usize;
    let found = loop {
        let hit = haystack
            .get(at..)?
            .iter()
            .position(|&byte| byte == b'e')?
            .saturating_add(at);
        if haystack
            .get(hit..)
            .is_some_and(|tail| tail.starts_with(NEEDLE))
        {
            break hit;
        }
        // Past the `e` that did not begin one, so the search always advances.
        at = hit.saturating_add(1);
    };

    let mut end = from.saturating_add(found);
    // Trim one end-of-line sequence.
    if end > from && input.get(end.saturating_sub(1)) == Some(&b'\n') {
        end = end.saturating_sub(1);
    }
    if end > from && input.get(end.saturating_sub(1)) == Some(&b'\r') {
        end = end.saturating_sub(1);
    }
    Some(end)
}

/// ISO 32000-2 §7.3.7's dictionary, on the two rules a reader can get wrong.
#[cfg(test)]
mod tests {
    use super::Parser;
    use crate::{Limits, Object};

    fn dictionary(source: &str) -> crate::Dictionary {
        let mut parser = Parser::at(source.as_bytes(), 0, Limits::default());
        match parser.parse_object() {
            Ok(Object::Dictionary(dict)) => dict,
            other => panic!("expected a dictionary, got {other:?}"),
        }
    }

    /// §7.3.3 admits no object spelled `.`, so nothing may be parsed from one.
    ///
    /// Both numeric forms are stated as "one or more decimal digits", so a run of regular
    /// characters holding none is not a numeric object; the lexer returns it as the keyword
    /// it is and this is what the parser then does with it. The pair is the point (trap 8):
    /// the two dictionaries differ in one character, and the conforming half has to read as
    /// a number for the refusal of the other half to mean anything.
    ///
    /// Reading `.` as zero is what a fallback would do, and in a `/MediaBox` or a `/Matrix`
    /// that is a page the wrong size rather than a file the reader complained about.
    #[test]
    fn a_number_with_no_digit_is_refused_rather_than_read_as_zero() {
        let conforming = dictionary("<< /Rotate 0 >>");
        assert_eq!(conforming.get("Rotate"), Some(&Object::Integer(0)));

        let mut parser = Parser::at(b"<< /Rotate . >>", 0, Limits::default());
        let refused = parser.parse_object();
        assert!(
            matches!(refused, Err(crate::SyntaxError::Unexpected { ref found, .. }) if found == "."),
            "`.` is no object and has to be said so, not read as zero: {refused:?}"
        );
    }

    /// §7.3.7: "Multiple entries in the same dictionary shall not have the same key."
    ///
    /// A requirement on the *file*, with no recovery stated, so keeping the first is a
    /// decision. It is pinned here because the alternative is equally defensible and a
    /// silent change of mind would move pixels in files nobody would think to re-check.
    #[test]
    fn a_repeated_key_keeps_the_first_value() {
        let dict = dictionary("<< /Type /Page /Type /Pages >>");

        assert_eq!(
            dict.get("Type"),
            Some(&Object::Name(crate::Name::new(*b"Page")))
        );
    }

    /// §7.3.7:
    ///
    /// > A dictionary entry whose value is null … shall be treated the same as if the entry
    /// > does not exist.
    ///
    /// This used to be asserted through `Document::get_key`, which answers `Null` for an
    /// absent key — so the assertion was the implementation restated and passed while
    /// `Dictionary::get` still handed back `Some(Null)`, `len` still counted the entry and
    /// `iter` still yielded it. "The same as if the entry does not exist" is a statement
    /// about the *dictionary*, so it is asserted about the dictionary.
    #[test]
    fn a_null_value_reads_the_same_as_an_absent_key() {
        let present = dictionary("<< /Mask null >>");
        let absent = dictionary("<< >>");

        assert_eq!(present, absent);
        assert_eq!(present.get("Mask"), None);
        assert_eq!(present.len(), 0);
    }

    /// The same rule, and the one place it could still be seen: §8.9.7's inline images
    /// decide between an abbreviated key and its full spelling by *presence*, so a
    /// `/Filter null` would have made a dictionary that both has and has not got a filter.
    #[test]
    fn a_null_value_does_not_survive_beside_a_real_one() {
        let dict = dictionary("<< /F null /Filter /FlateDecode >>");

        assert_eq!(dict.get("F"), None);
        assert_eq!(dict.len(), 1);
    }
}
