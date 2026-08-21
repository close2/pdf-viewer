//! A bounded reader for the ASN.1 encoding a signature value is written in.
//!
//! ISO 32000-2 §12.8.3.3.1 says what a CMS signature value is: "the value of Contents shall be a
//! DER-encoded CMS binary data object containing the signature". Reading one therefore needs a
//! reader for ITU-T X.690's distinguished encoding rules, and this is it — the smallest one that
//! answers §12.8's questions, and no more.
//!
//! # Why a reader lives here rather than a dependency
//!
//! ADR 0215 argues it. The short form: this parses **untrusted input** — a byte string out of a
//! PDF a stranger wrote — and everything the clause needs is a tag, a length and a slice. There is
//! no arithmetic on the values, no allocation driven by anything the file states, and nothing is
//! decoded into an owned type: every accessor hands back a sub-slice of the caller's buffer. That
//! is a hundred and fifty lines that this crate's `#![forbid(unsafe_code)]` covers, against a
//! dependency tree for a general-purpose ASN.1 compiler.
//!
//! # DER is what the clause says and BER is what producers write
//!
//! **This reader accepts indefinite lengths, which DER forbids, and the tolerance is deliberate.**
//! X.690 clause 8.1.3.6 lets a *constructed* value state length octet `80` and close itself with an
//! end-of-contents marker instead of counting its bytes; X.690 clause 10.1 forbids that in DER. Four of the
//! ten signature values the corpus's nine signed documents hold are written that way — Adobe's
//! own signature handler emits `30 80` for the outer `ContentInfo` — so a reader that took the
//! clause's word for it would answer "unreadable" for the commonest real signature there is. The
//! cost is written down in ADR 0215: this reader cannot be used to *check* that a producer wrote
//! DER, and nothing here claims to.
//!
//! **That "four" is a count and now has a command**, which it lacked for as long as it has been
//! written here: [`Value::had_indefinite_length`] with
//! `cargo run --release -p pdf-model --example signature_algorithm_census`. It re-derives to four,
//! one value in each of four documents, so this sentence and §12.8.3.4.2's ledger row agree
//! despite counting different things.
//!
//! # The bounds, and where each comes from
//!
//! Untrusted input reaches this module first, so every loop is bounded by something that is not
//! the file's own arithmetic:
//!
//! - [`MAX_DEPTH`] caps nesting. A file can nest constructed values without limit and a reader
//!   that followed would recurse until the stack ended.
//! - [`MAX_VALUE`] caps the whole input. Finding where an indefinite-length value ends means
//!   scanning its contents, and a value nested `MAX_DEPTH` deep is scanned that many times, so the
//!   work is bounded only when the input is.
//! - A length is read from at most four octets, which is [`u32::MAX`] and already more than
//!   [`MAX_VALUE`] permits; a longer one is refused rather than truncated.
//!
//! Nothing here allocates at all.

/// How deeply constructed values may nest.
///
/// RFC 5652's `SignedData` reaches nine levels at the deepest thing this module looks at — a
/// signed attribute's value inside a `SET OF` inside `[0] IMPLICIT` inside a `SignerInfo` inside a
/// `SET OF` inside `SignedData` inside `[0] EXPLICIT` inside `ContentInfo` — so sixteen is that
/// with room, and it is not a number any real encoding approaches.
pub const MAX_DEPTH: u8 = 16;

/// The largest signature value this reader will look at, in bytes.
///
/// §12.8.3.3.2's NOTE says why the bound is not smaller: "CRLs can be large and therefore require
/// more pre-allocated space in the value of the Contents key". The largest in the 974-document
/// corpus is 33 680 bytes, so this is three orders of magnitude of room, and it exists to keep the
/// indefinite-length scan's work bounded rather than to express an opinion about signatures.
pub const MAX_VALUE: usize = 2 * 1024 * 1024;

/// What stopped a value from being read.
///
/// Every one of these is a statement about the *file*, which is why the type is public and why
/// [`crate::signature::Integrity`] carries one rather than flattening them all into "unreadable".
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DerError {
    /// A tag-length-value header, or the value it introduces, runs past the end of the input.
    #[error("an ASN.1 value runs past the end of the signature")]
    Truncated,
    /// The input is longer than [`MAX_VALUE`].
    #[error("the signature value is larger than {MAX_VALUE} bytes")]
    TooLarge,
    /// A length is stated in more than four octets, which is more than [`MAX_VALUE`] can need.
    #[error("an ASN.1 length is stated in more than four octets")]
    LengthTooLong,
    /// A *primitive* value states the indefinite length X.690 clause 8.1.3.6 permits only to a
    /// constructed one.
    #[error("a primitive ASN.1 value states an indefinite length")]
    IndefinitePrimitive,
    /// An indefinite-length value is never closed by an end-of-contents marker.
    #[error("an indefinite-length ASN.1 value is not closed")]
    Unterminated,
    /// Values nest more than [`MAX_DEPTH`] deep.
    #[error("ASN.1 values nest more than {MAX_DEPTH} deep")]
    TooDeep,
}

/// X.690's tag class, in the two high-order bits of an identifier octet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// The types X.680 defines: `SEQUENCE`, `SET`, `OBJECT IDENTIFIER` and the rest.
    Universal,
    /// A field of a module's own, written `[0]`, `[1]` — how RFC 5652 spells its optional
    /// members.
    ContextSpecific,
    /// X.690's `APPLICATION` class, which nothing this module reads uses.
    Application,
    /// X.690's `PRIVATE` class, likewise.
    Private,
}

/// `SEQUENCE`, constructed and universal — X.690's identifier octet for it.
pub const SEQUENCE: u8 = 0x30;
/// `SET` and `SET OF`, likewise.
pub const SET: u8 = 0x31;
/// `OBJECT IDENTIFIER`, primitive and universal.
pub const OBJECT_IDENTIFIER: u8 = 0x06;
/// `OCTET STRING`, primitive and universal.
pub const OCTET_STRING: u8 = 0x04;
/// `INTEGER`, primitive and universal.
pub const INTEGER: u8 = 0x02;

/// One tag-length-value: what it is, and the bytes between its header and its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Value<'a> {
    /// The identifier octet, whole — class, the constructed bit, and the tag number.
    pub identifier: u8,
    /// The value's contents, which for a constructed value is the encoding of its children.
    pub contents: &'a [u8],
    /// How many constructed values enclose this one.
    depth: u8,
    /// Whether this value's own length octets were X.690 clause 8.1.3.6's indefinite form.
    indefinite: bool,
}

impl<'a> Value<'a> {
    /// The tag's class.
    #[must_use]
    pub fn class(&self) -> Class {
        match self.identifier & 0xC0 {
            0x40 => Class::Application,
            0x80 => Class::ContextSpecific,
            0xC0 => Class::Private,
            _ => Class::Universal,
        }
    }

    /// Whether the value's contents are further values rather than an unstructured string.
    #[must_use]
    pub fn is_constructed(&self) -> bool {
        self.identifier & 0x20 != 0
    }

    /// The tag number, for the low-tag-number form this module is limited to.
    ///
    /// X.690 clause 8.1.2.4 spells a number above 30 across several octets; RFC 5652 uses none, so a
    /// value that does is read as tag 31 and matches nothing this module asks for.
    #[must_use]
    pub fn tag_number(&self) -> u8 {
        self.identifier & 0x1F
    }

    /// Whether this is a context-specific tag with the given number, constructed or not.
    ///
    /// The two spellings matter because RFC 5652 writes its optional members `[0] IMPLICIT` — so
    /// `certificates` is `A0` where a `SET OF` would be `31`, and `eContent` is `A0` holding an
    /// `OCTET STRING`.
    #[must_use]
    pub fn is_context(&self, number: u8) -> bool {
        self.class() == Class::ContextSpecific && self.tag_number() == number
    }

    /// The values inside a constructed one.
    ///
    /// # Errors
    ///
    /// [`DerError::TooDeep`] where this value already sits [`MAX_DEPTH`] levels down. A primitive
    /// value yields an empty reader rather than an error: its contents are bytes, and asking for
    /// children is the caller saying it expected otherwise.
    pub fn children(&self) -> Result<Reader<'a>, DerError> {
        let depth = self.depth.saturating_add(1);
        if depth >= MAX_DEPTH {
            return Err(DerError::TooDeep);
        }
        Ok(Reader {
            rest: if self.is_constructed() {
                self.contents
            } else {
                &[]
            },
            depth,
        })
    }

    /// Whether this value's length was written in X.690 clause 8.1.3.6's indefinite form.
    ///
    /// DER forbids it and this reader accepts it anyway, for the reason the module comment gives.
    /// **What that tolerance costs is a claim about the world**, not about the standard — how many
    /// files would stop being readable if it were withdrawn — and §12.8.3.4.2's ledger row states
    /// such a number. Nothing in the tree could produce it until this accessor existed, which is
    /// `CLAUDE.md`'s rule about a counted fact applied to a decision's stated cost.
    #[must_use]
    pub const fn had_indefinite_length(&self) -> bool {
        self.indefinite
    }

    /// This value's contents where it is an `OBJECT IDENTIFIER`, and `None` otherwise.
    ///
    /// The octets are handed back as they are encoded. Nothing in §12.8 needs an OID's *numbers* —
    /// every use is a comparison against a constant — so decoding one would be inventing a
    /// requirement, and the encoding is unique for a given identifier.
    #[must_use]
    pub fn object_identifier(&self) -> Option<&'a [u8]> {
        (self.identifier == OBJECT_IDENTIFIER).then_some(self.contents)
    }
}

/// A sequence of values, read one at a time from a slice.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    rest: &'a [u8],
    depth: u8,
}

impl<'a> Reader<'a> {
    /// A reader over a whole signature value.
    ///
    /// # Errors
    ///
    /// [`DerError::TooLarge`] where the input exceeds [`MAX_VALUE`]. That is refused here, once,
    /// rather than in each of the loops below.
    pub fn new(bytes: &'a [u8]) -> Result<Self, DerError> {
        if bytes.len() > MAX_VALUE {
            return Err(DerError::TooLarge);
        }
        Ok(Self {
            rest: bytes,
            depth: 0,
        })
    }

    /// The next value, or `None` at the end of the input or at an end-of-contents marker.
    ///
    /// # Errors
    ///
    /// Any [`DerError`] the encoding produces. A reader that has returned an error is not reset
    /// and should not be asked again; the callers here stop at the first one.
    pub fn next_value(&mut self) -> Result<Option<Value<'a>>, DerError> {
        let Some((&identifier, after_identifier)) = self.rest.split_first() else {
            return Ok(None);
        };
        // X.690 clause 8.1.5: two zero octets end an indefinite-length value's contents. Reaching one
        // here means the enclosing value is finished, which is the same answer as running out.
        if identifier == 0x00 && after_identifier.first() == Some(&0x00) {
            self.rest = &[];
            return Ok(None);
        }
        let Some((&first_length, after_length)) = after_identifier.split_first() else {
            return Err(DerError::Truncated);
        };
        if first_length == 0x80 {
            if identifier & 0x20 == 0 {
                return Err(DerError::IndefinitePrimitive);
            }
            let end = end_of_contents(after_length, self.depth)?;
            let (contents, rest) = after_length.split_at(end);
            // Two octets for the marker itself; `end` is where it starts and `end_of_contents`
            // has already established that both are there.
            self.rest = rest.get(2..).unwrap_or(&[]);
            return Ok(Some(Value {
                identifier,
                contents,
                depth: self.depth,
                indefinite: true,
            }));
        }
        let (length, body) = if first_length & 0x80 == 0 {
            (usize::from(first_length), after_length)
        } else {
            let count = usize::from(first_length & 0x7F);
            if count == 0 || count > 4 {
                return Err(DerError::LengthTooLong);
            }
            let Some(octets) = after_length.get(..count) else {
                return Err(DerError::Truncated);
            };
            let mut length = 0usize;
            for &octet in octets {
                // Four octets of a `usize` that is at least 32 bits wide, so neither the shift
                // nor the sum can wrap; `MAX_VALUE` is checked against the result below.
                length = (length << 8) | usize::from(octet);
            }
            (length, after_length.get(count..).unwrap_or(&[]))
        };
        let Some(contents) = body.get(..length) else {
            return Err(DerError::Truncated);
        };
        self.rest = body.get(length..).unwrap_or(&[]);
        Ok(Some(Value {
            identifier,
            contents,
            depth: self.depth,
            indefinite: false,
        }))
    }
}

/// Where an indefinite-length value's contents stop: the offset of its end-of-contents marker.
///
/// The scan walks the children, because a marker is only this value's if it is not inside one of
/// them, and skipping a definite-length child costs its length rather than a walk. The depth bound
/// is what stops a file from nesting indefinite lengths until the stack ends.
fn end_of_contents(bytes: &[u8], depth: u8) -> Result<usize, DerError> {
    if depth.saturating_add(1) >= MAX_DEPTH {
        return Err(DerError::TooDeep);
    }
    let mut at = 0usize;
    loop {
        let Some(rest) = bytes.get(at..) else {
            return Err(DerError::Unterminated);
        };
        let [identifier, first_length, ..] = *rest else {
            return Err(DerError::Unterminated);
        };
        if identifier == 0x00 && first_length == 0x00 {
            return Ok(at);
        }
        // Every step below moves `at` forward by at least the two header octets just read, so the
        // loop is bounded by the input's length.
        let header = at.saturating_add(2);
        if first_length == 0x80 {
            if identifier & 0x20 == 0 {
                return Err(DerError::IndefinitePrimitive);
            }
            let Some(inner) = bytes.get(header..) else {
                return Err(DerError::Unterminated);
            };
            let inner_end = end_of_contents(inner, depth.saturating_add(1))?;
            at = header.saturating_add(inner_end).saturating_add(2);
            continue;
        }
        let (length, header) = if first_length & 0x80 == 0 {
            (usize::from(first_length), header)
        } else {
            let count = usize::from(first_length & 0x7F);
            if count == 0 || count > 4 {
                return Err(DerError::LengthTooLong);
            }
            let Some(octets) = bytes.get(header..header.saturating_add(count)) else {
                return Err(DerError::Unterminated);
            };
            let mut length = 0usize;
            for &octet in octets {
                length = (length << 8) | usize::from(octet);
            }
            (length, header.saturating_add(count))
        };
        at = header.saturating_add(length);
        if at > bytes.len() {
            return Err(DerError::Unterminated);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Class, DerError, MAX_VALUE, OBJECT_IDENTIFIER, Reader, SEQUENCE};

    /// Every value in a definite-length sequence, with its contents.
    #[test]
    fn a_definite_length_sequence_yields_its_members() {
        // SEQUENCE { OBJECT IDENTIFIER 1.2.840.113549.1.7.2, INTEGER 1 }
        let bytes = [
            SEQUENCE,
            0x0E,
            OBJECT_IDENTIFIER,
            0x09,
            0x2A,
            0x86,
            0x48,
            0x86,
            0xF7,
            0x0D,
            0x01,
            0x07,
            0x02,
            0x02,
            0x01,
            0x01,
        ];
        let mut reader = Reader::new(&bytes).expect("within MAX_VALUE");
        let outer = reader
            .next_value()
            .expect("a well-formed sequence")
            .expect("one value");
        assert_eq!(outer.class(), Class::Universal);
        assert!(outer.is_constructed());
        let mut inner = outer.children().expect("depth 1");
        let oid = inner.next_value().expect("well formed").expect("an oid");
        assert_eq!(
            oid.object_identifier(),
            Some([0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02].as_slice())
        );
        let integer = inner
            .next_value()
            .expect("well formed")
            .expect("an integer");
        assert_eq!(integer.contents, [0x01]);
        assert_eq!(inner.next_value().expect("well formed"), None);
    }

    /// X.690 clause 8.1.3.6's indefinite length, which DER forbids and Adobe's handler writes.
    ///
    /// The outer sequence states length octet `80` and is closed by `00 00`; the inner one counts
    /// its bytes. Both spellings have to come out as the same three values, or the tolerance this
    /// module documents would be a different reading of the same file.
    #[test]
    fn an_indefinite_length_value_ends_at_its_marker() {
        let bytes = [
            SEQUENCE, 0x80, // outer, indefinite
            0x02, 0x01, 0x07, // INTEGER 7
            SEQUENCE, 0x03, 0x02, 0x01, 0x09, // SEQUENCE { INTEGER 9 }
            0x00, 0x00, // end of contents
            0x04, 0x01, 0x2A, // an OCTET STRING after it, which must still be read
        ];
        let mut reader = Reader::new(&bytes).expect("within MAX_VALUE");
        let outer = reader.next_value().expect("well formed").expect("a value");
        assert_eq!(outer.contents.len(), 8, "everything before the marker");
        let mut inner = outer.children().expect("depth 1");
        assert_eq!(
            inner
                .next_value()
                .expect("well formed")
                .expect("an integer")
                .contents,
            [0x07]
        );
        assert!(
            inner
                .next_value()
                .expect("well formed")
                .expect("a sequence")
                .is_constructed()
        );
        assert_eq!(inner.next_value().expect("well formed"), None);
        let after = reader.next_value().expect("well formed").expect("a value");
        assert_eq!(after.contents, [0x2A], "the marker is consumed, not left");
    }

    /// The zero padding §12.8.3.3.1 requires is not a value and does not become one.
    ///
    /// That clause tells a producer to pad `/Contents` with zeros up to the space allocated for it,
    /// because a CMS object's length cannot be predicted before it is built. A pair of zero octets
    /// is also X.690's end-of-contents marker, so a reader that stripped the padding before parsing
    /// would cut that marker off an indefinite-length encoding — which is exactly how this test came
    /// to exist, on the first draft of this module.
    #[test]
    fn trailing_padding_is_not_read_as_a_value() {
        let bytes = [SEQUENCE, 0x03, 0x02, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00];
        let mut reader = Reader::new(&bytes).expect("within MAX_VALUE");
        assert!(reader.next_value().expect("well formed").is_some());
        assert_eq!(
            reader.next_value().expect("well formed"),
            None,
            "the padding ends the reader rather than becoming a value"
        );
    }

    /// A length that runs past the end of the input is refused rather than clamped.
    #[test]
    fn a_length_past_the_end_is_truncated_rather_than_clamped() {
        let bytes = [SEQUENCE, 0x7F, 0x02, 0x01, 0x05];
        let mut reader = Reader::new(&bytes).expect("within MAX_VALUE");
        assert_eq!(reader.next_value(), Err(DerError::Truncated));
    }

    /// The bounds, each checked on the shape that would otherwise run away.
    #[test]
    fn the_bounds_hold() {
        // An indefinite length with no marker anywhere.
        let mut reader = Reader::new(&[SEQUENCE, 0x80, 0x02, 0x01, 0x05]).expect("small");
        assert_eq!(reader.next_value(), Err(DerError::Unterminated));

        // A primitive value may not state one at all (X.690 clause 8.1.3.6 applies to constructed).
        let mut reader = Reader::new(&[0x04, 0x80, 0x00, 0x00]).expect("small");
        assert_eq!(reader.next_value(), Err(DerError::IndefinitePrimitive));

        // Five length octets is more than MAX_VALUE could ever need.
        let mut reader = Reader::new(&[SEQUENCE, 0x85, 0, 0, 0, 0, 1, 0]).expect("small");
        assert_eq!(reader.next_value(), Err(DerError::LengthTooLong));

        // Nesting past MAX_DEPTH, built as sequences one inside the next.
        let mut nested = vec![0x02, 0x01, 0x00];
        for _ in 0..super::MAX_DEPTH {
            let mut wrapper = vec![SEQUENCE, u8::try_from(nested.len()).unwrap_or(0)];
            wrapper.append(&mut nested);
            nested = wrapper;
        }
        let mut reader = Reader::new(&nested).expect("small");
        let mut value = reader.next_value().expect("well formed").expect("a value");
        let deepest = loop {
            match value.children() {
                Err(error) => break error,
                Ok(mut inner) => match inner.next_value() {
                    Ok(Some(next)) => value = next,
                    Ok(None) | Err(_) => panic!("the nesting should end at MAX_DEPTH"),
                },
            }
        };
        assert_eq!(deepest, DerError::TooDeep);

        // And a value larger than MAX_VALUE is refused before any of that.
        assert_eq!(
            Reader::new(&vec![0u8; MAX_VALUE.saturating_add(1)]).map(|_| ()),
            Err(DerError::TooLarge)
        );
    }
}
