//! Parsing one indirect object at an offset: its stated length, and the extent when it is wrong.

use std::sync::Arc;

use super::{Document, ENDSTREAM_WINDOW, write};
use crate::object::{Object, ObjectId, Stream};
use crate::parser::Parser;

impl Document {
    /// Parses the indirect object at `offset` and decrypts it if the document is encrypted.
    ///
    /// The generation used is the one written in the file rather than the one the
    /// cross-reference table records, because §7.6.3.2 step (a) takes both numbers "from
    /// the object identifier of the string or stream to be encrypted" — that is, from the
    /// object as written.
    pub(super) fn parse_at(&self, offset: usize) -> Option<(ObjectId, Object)> {
        // The bytes from `offset`, which is the rest of the file in memory and a window that
        // grows to what the parse examined on disk — the same slice either way, which is what
        // keeps the object a function of the bytes alone (ADR 0809). The parser's positions are
        // the window's, so the one it hands out is put back into the file's.
        let (found, object, data_at, padding) =
            self.bytes
                .parse_from(offset, crate::file::FIRST_WINDOW, |input, _| {
                    let mut parser = Parser::at(input, 0, self.limits);
                    let parsed = parser.parse_indirect_object().ok().map(|(found, object)| {
                        let data_at = parser.stream_data_at().map(|at| at.saturating_add(offset));
                        let padding = parser.stream_keyword_padding().map(|padding| {
                            crate::parser::KeywordPadding {
                                keyword_end: padding.keyword_end.saturating_add(offset),
                                bytes: padding.bytes,
                            }
                        });
                        (found, object, data_at, padding)
                    });
                    (parsed, parser.examined())
                })?;
        let (object, padding) = self.with_stated_length(data_at, padding, object);
        if let Some(padding) = padding {
            write(&self.padded).insert(found.number, padding.bytes);
        }
        Some((found, self.decrypt_object(found, object)))
    }

    /// Applies a `/Length` the parser could not follow, §7.3.8.2.
    ///
    /// > Every stream dictionary shall have a Length entry that indicates how many bytes of the
    /// > PDF file are used for the stream's data.
    ///
    /// Table 5 makes that entry "(Required; shall be an indirect reference)" for a stream whose
    /// length a producer does not know until it has written the data, and a **parser** cannot
    /// follow a reference: resolving one needs the document, and the document is built out of
    /// parsed objects. So [`Parser::parse_stream_data`] falls back to searching for `endstream`
    /// and trimming one end-of-line — the delimiter's, per §7.3.8.1's "there should be an
    /// end-of-line marker" — and where the producer wrote none, the byte it trims is the
    /// **data's**. That is a stream one byte short of what the file says it is, and for a
    /// `FlateDecode` stream the byte it loses is usually the last of RFC 1951's final block, so
    /// the stream reads as damaged while being whole. Two of the 65 944 crawled documents are
    /// exactly that, and finding them is what ADR 0366's object-stream rule did first.
    ///
    /// This is where the file's own statement is applied, one layer up, under the same guard the
    /// parser puts on a direct length: the stated end is taken only where `endstream` is actually
    /// there, so a *wrong* `/Length` still loses to the search. §7.3.8.2's "[a]ll of these
    /// constraints shall be consistent" is what makes that check the right arbiter either way.
    ///
    /// **And it is the indirect half of the guard on the parser's padding recovery** (ADR
    /// 1365): where the parser skipped spaces before the `stream` keyword's end-of-line marker
    /// and the stated length is confirmed only from the keyword's own end, the padding was the
    /// data's by the file's own count, and it is read that way and not reported.
    fn with_stated_length(
        &self,
        data_at: Option<usize>,
        padding: Option<crate::parser::KeywordPadding>,
        object: Object,
    ) -> (Object, Option<crate::parser::KeywordPadding>) {
        let Object::Stream(stream) = &object else {
            return (object, padding);
        };
        // Only an indirect one: a direct `/Length` is the parser's own business and it has
        // already decided, with more context than this has.
        if !matches!(stream.dict.get("Length"), Some(Object::Reference(_))) {
            return (object, padding);
        }
        let Some(start) = data_at else {
            return (object, padding);
        };
        let Some(stated) = self
            .get_key(&stream.dict, "Length")
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())
        else {
            return (object, padding);
        };
        if stated > self.limits.max_stream_len {
            return (object, padding);
        }
        if stated == stream.data.len() {
            return (object, padding);
        }
        if let Some(data) = self.stated_extent(start, stated) {
            let stream = Object::Stream(Arc::new(Stream {
                dict: stream.dict.clone(),
                data: Arc::from(&*data),
                decryption_failed: stream.decryption_failed,
            }));
            return (stream, padding);
        }
        let Some(unpadded) = padding else {
            return (object, padding);
        };
        match self.stated_extent(unpadded.keyword_end, stated) {
            Some(data) => (
                Object::Stream(Arc::new(Stream {
                    dict: stream.dict.clone(),
                    data: Arc::from(&*data),
                    decryption_failed: stream.decryption_failed,
                })),
                None,
            ),
            None => (object, padding),
        }
    }

    /// The `stated` bytes from `start`, where `endstream` follows them in the file — the check
    /// the parser puts on a direct length — and `None` where it does not, so that a *wrong*
    /// `/Length` still loses to the search.
    fn stated_extent(&self, start: usize, stated: usize) -> Option<std::borrow::Cow<'_, [u8]>> {
        let end = start.checked_add(stated)?;
        if end > self.bytes.len() {
            return None;
        }
        // Whether `endstream` follows the stated end, read from there: a small window that
        // grows over any whitespace before the keyword, as the parser's own check does.
        let follows = self.bytes.parse_from(end, ENDSTREAM_WINDOW, |input, _| {
            crate::parser::endstream_examined(input, 0)
        });
        if !follows {
            return None;
        }
        let data = self.bytes.read(start..end);
        (data.len() == stated).then_some(data)
    }
}
