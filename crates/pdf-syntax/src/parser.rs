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
    /// Maximum length of one stream's raw data, in bytes.
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
        max_stream_len: 1 << 31,
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
        }
    }

    /// Creates a parser positioned at `offset`.
    #[must_use]
    pub fn at(input: &'a [u8], offset: usize, limits: Limits) -> Self {
        Self {
            lexer: Lexer::at(input, offset),
            limits,
            depth: 0,
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
    fn parse_object_from(&mut self, token: Token) -> SyntaxResult<Object> {
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
            Token::Keyword(word) => match word.as_slice() {
                b"true" => Ok(Object::Boolean(true)),
                b"false" => Ok(Object::Boolean(false)),
                b"null" => Ok(Object::Null),
                _ => Err(SyntaxError::Unexpected {
                    at: self.lexer.position(),
                    found: String::from_utf8_lossy(&word).into_owned(),
                    expected: "an object",
                }),
            },
            Token::ArrayClose | Token::DictClose | Token::BraceOpen | Token::BraceClose => {
                Err(SyntaxError::Unexpected {
                    at: self.lexer.position(),
                    found: format!("{token:?}"),
                    expected: "an object",
                })
            }
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
        if probe.next_token() == Some(Token::Keyword(b"stream".to_vec())) {
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
                            // A duplicate key is malformed and the specification does not
                            // say which wins. First wins here, matching poppler, so at
                            // least the choice is the same as the majority of readers.
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
        self.lexer.seek(end);

        // Consume `endstream` if present. Its absence is not fatal: the data has already
        // been delimited, and rejecting the object would lose a page over a missing
        // keyword.
        let rewind = self.lexer.position();
        if self.lexer.next_token() != Some(Token::Keyword(b"endstream".to_vec())) {
            self.lexer.seek(rewind);
        }

        Ok(Object::Stream(Arc::new(Stream {
            dict,
            data: Arc::from(data),
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
        if self.lexer.next_token() != Some(Token::Keyword(b"obj".to_vec())) {
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
        if self.lexer.next_token() != Some(Token::Keyword(b"endobj".to_vec())) {
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
fn endstream_follows(input: &[u8], offset: usize) -> bool {
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
fn find_endstream(input: &[u8], from: usize) -> Option<usize> {
    let haystack = input.get(from..)?;
    let found = haystack
        .windows(b"endstream".len())
        .position(|window| window == b"endstream")?;

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
