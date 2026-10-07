//! A [`Request`] and an [`Outcome`] as bytes, for the process boundary RFC 0008 section 6.2 puts
//! the engine behind.
//!
//! `confined-transport` carries a kind byte and a length and knows nothing of what is inside; this
//! is what is inside. Every integer is little-endian, every string is a `u32` byte length and that
//! many bytes of UTF-8, every boolean is one byte that is 0 or 1, and every enumeration is a tag
//! byte followed by its fields. Both encodings begin with a version byte, so that a worker and a
//! host built from different trees refuse each other rather than misread each other. Decoding
//! reads a length only against the bytes that remain, so a hostile length asks for nothing.

use std::time::Duration;

use pdf_model::aform::Trigger;

use crate::{Ending, Event, Exceeded, Outcome, Refusal, RefusalKind, Request};

/// The first byte of every encoding this module writes.
pub const VERSION: u8 = 1;

/// Why bytes are not an encoding this module wrote.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WireError {
    /// The bytes end inside a field.
    #[error("the message ends inside a field")]
    Truncated,
    /// The version byte is not [`VERSION`].
    #[error("the message is version {0}, and this reads version {VERSION}")]
    Version(u8),
    /// A tag or a boolean has a value no encoding uses.
    #[error("the message's {0} has a value no encoding uses")]
    Invalid(&'static str),
    /// A string is not UTF-8.
    #[error("a string in the message is not UTF-8")]
    Utf8,
    /// Bytes remain after the message.
    #[error("{0} bytes follow the message")]
    Trailing(usize),
}

/// Encodes a request.
#[must_use]
pub fn encode_request(request: &Request) -> Vec<u8> {
    let mut out = vec![VERSION];
    put_u8(&mut out, trigger_tag(request.trigger));
    put_str(&mut out, &request.field);
    put_str(&mut out, &request.script);
    put_str(&mut out, &request.event.value);
    put_str(&mut out, &request.event.change);
    put_u32(&mut out, request.event.selection_start);
    put_u32(&mut out, request.event.selection_end);
    put_bool(&mut out, request.event.will_commit);
    put_u64(&mut out, request.moment);
    out.extend_from_slice(&request.utc_offset_seconds.to_le_bytes());
    out
}

/// Decodes a request.
///
/// # Errors
///
/// [`WireError`] for bytes that are not exactly one request of this version.
pub fn decode_request(bytes: &[u8]) -> Result<Request, WireError> {
    let mut reader = Reader::new(bytes)?;
    let trigger = match reader.u8()? {
        0 => Trigger::Keystroke,
        1 => Trigger::Format,
        2 => Trigger::Validate,
        3 => Trigger::Calculate,
        _ => return Err(WireError::Invalid("trigger")),
    };
    let request = Request {
        trigger,
        field: reader.string()?,
        script: reader.string()?,
        event: Event {
            value: reader.string()?,
            change: reader.string()?,
            selection_start: reader.u32()?,
            selection_end: reader.u32()?,
            will_commit: reader.boolean()?,
        },
        moment: reader.u64()?,
        utc_offset_seconds: i32::from_le_bytes(reader.array()?),
    };
    reader.finish()?;
    Ok(request)
}

/// Encodes an outcome.
#[must_use]
pub fn encode_outcome(outcome: &Outcome) -> Vec<u8> {
    let mut out = vec![VERSION];
    put_bool(&mut out, outcome.rc);
    put_optional(&mut out, outcome.value.as_deref());
    put_optional(&mut out, outcome.change.as_deref());
    match &outcome.ending {
        Ending::Finished => put_u8(&mut out, 0),
        Ending::Exceeded(exceeded) => {
            put_u8(&mut out, 1);
            put_exceeded(&mut out, *exceeded);
        }
        Ending::Threw(text) => {
            put_u8(&mut out, 2);
            put_str(&mut out, text);
        }
        Ending::Unparsed(text) => {
            put_u8(&mut out, 3);
            put_str(&mut out, text);
        }
        Ending::Declined(text) => {
            put_u8(&mut out, 4);
            put_str(&mut out, text);
        }
    }
    put_len(&mut out, outcome.refusals.len());
    for refusal in &outcome.refusals {
        put_str(&mut out, &refusal.member);
        match &refusal.kind {
            RefusalKind::Excluded(reason) => {
                put_u8(&mut out, 0);
                put_str(&mut out, reason);
            }
            RefusalKind::NotBridged => put_u8(&mut out, 1),
            RefusalKind::Unreachable(why) => {
                put_u8(&mut out, 2);
                put_str(&mut out, why);
            }
            RefusalKind::Library(why) => {
                put_u8(&mut out, 3);
                put_str(&mut out, why);
            }
        }
    }
    put_len(&mut out, outcome.log.len());
    for line in &outcome.log {
        put_str(&mut out, line);
    }
    out
}

/// Decodes an outcome.
///
/// # Errors
///
/// [`WireError`] for bytes that are not exactly one outcome of this version.
pub fn decode_outcome(bytes: &[u8]) -> Result<Outcome, WireError> {
    let mut reader = Reader::new(bytes)?;
    let rc = reader.boolean()?;
    let value = reader.optional()?;
    let change = reader.optional()?;
    let ending = match reader.u8()? {
        0 => Ending::Finished,
        1 => Ending::Exceeded(reader.exceeded()?),
        2 => Ending::Threw(reader.string()?),
        3 => Ending::Unparsed(reader.string()?),
        4 => Ending::Declined(reader.string()?),
        _ => return Err(WireError::Invalid("ending")),
    };
    let count = reader.u32()?;
    let mut refusals = Vec::new();
    for _ in 0..count {
        let member = reader.string()?;
        let kind = match reader.u8()? {
            0 => RefusalKind::Excluded(reader.string()?),
            1 => RefusalKind::NotBridged,
            2 => RefusalKind::Unreachable(reader.string()?),
            3 => RefusalKind::Library(reader.string()?),
            _ => return Err(WireError::Invalid("refusal")),
        };
        refusals.push(Refusal { member, kind });
    }
    let count = reader.u32()?;
    let mut log = Vec::new();
    for _ in 0..count {
        log.push(reader.string()?);
    }
    reader.finish()?;
    Ok(Outcome {
        rc,
        value,
        change,
        ending,
        refusals,
        log,
    })
}

/// The tag a trigger is written with.
fn trigger_tag(trigger: Trigger) -> u8 {
    match trigger {
        Trigger::Keystroke => 0,
        Trigger::Format => 1,
        Trigger::Validate => 2,
        Trigger::Calculate => 3,
    }
}

/// Writes which budget was exceeded.
fn put_exceeded(out: &mut Vec<u8>, exceeded: Exceeded) {
    match exceeded {
        Exceeded::Wall(limit) => {
            put_u8(out, 0);
            put_u64(out, u64::try_from(limit.as_micros()).unwrap_or(u64::MAX));
        }
        Exceeded::Steps(limit) => {
            put_u8(out, 1);
            put_u64(out, limit);
        }
        Exceeded::LoopIterations(limit) => {
            put_u8(out, 2);
            put_u64(out, limit);
        }
        Exceeded::Recursion(limit) => {
            put_u8(out, 3);
            put_u32(out, limit);
        }
        Exceeded::Stack(limit) => {
            put_u8(out, 4);
            put_u32(out, limit);
        }
        Exceeded::Elements { asked, ceiling } => {
            put_u8(out, 5);
            put_u64(out, asked);
            put_u64(out, ceiling);
        }
        Exceeded::StringUnits { asked, ceiling } => {
            put_u8(out, 6);
            put_u64(out, asked);
            put_u64(out, ceiling);
        }
    }
}

/// Writes one byte.
fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

/// Writes a `u32`.
fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Writes a `u64`.
fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Writes a boolean.
fn put_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

/// Writes a count, which no message this crate builds takes past `u32`.
fn put_len(out: &mut Vec<u8>, length: usize) {
    put_u32(out, u32::try_from(length).unwrap_or(u32::MAX));
}

/// Writes a string. A string longer than `u32` bytes is not one a run produces — a script's
/// strings are bounded by [`crate::Budget::string_units`] — and is cut at the last character
/// boundary that fits rather than written with a length that lies.
fn put_str(out: &mut Vec<u8>, text: &str) {
    let mut end = text.len().min(u32::MAX as usize);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let kept = text.get(..end).unwrap_or_default();
    put_len(out, kept.len());
    out.extend_from_slice(kept.as_bytes());
}

/// Writes a string that may be absent.
fn put_optional(out: &mut Vec<u8>, text: Option<&str>) {
    match text {
        None => put_u8(out, 0),
        Some(text) => {
            put_u8(out, 1);
            put_str(out, text);
        }
    }
}

/// Reads an encoding from the front.
struct Reader<'a> {
    /// What has not been read.
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader past the version byte, which it checks.
    fn new(bytes: &'a [u8]) -> Result<Self, WireError> {
        let mut reader = Self { rest: bytes };
        match reader.u8()? {
            VERSION => Ok(reader),
            other => Err(WireError::Version(other)),
        }
    }

    /// The next `count` bytes.
    fn take(&mut self, count: usize) -> Result<&'a [u8], WireError> {
        if self.rest.len() < count {
            return Err(WireError::Truncated);
        }
        let (taken, rest) = self.rest.split_at(count);
        self.rest = rest;
        Ok(taken)
    }

    /// The next `N` bytes, as an array.
    fn array<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        let mut array = [0_u8; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    /// One byte.
    fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.array::<1>()?[0])
    }

    /// A `u32`.
    fn u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    /// A `u64`.
    fn u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    /// A boolean.
    fn boolean(&mut self) -> Result<bool, WireError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WireError::Invalid("boolean")),
        }
    }

    /// A string.
    fn string(&mut self) -> Result<String, WireError> {
        let length = usize::try_from(self.u32()?).map_err(|_| WireError::Truncated)?;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| WireError::Utf8)
    }

    /// A string that may be absent.
    fn optional(&mut self) -> Result<Option<String>, WireError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.string()?)),
            _ => Err(WireError::Invalid("optional string")),
        }
    }

    /// Which budget was exceeded.
    fn exceeded(&mut self) -> Result<Exceeded, WireError> {
        Ok(match self.u8()? {
            0 => Exceeded::Wall(Duration::from_micros(self.u64()?)),
            1 => Exceeded::Steps(self.u64()?),
            2 => Exceeded::LoopIterations(self.u64()?),
            3 => Exceeded::Recursion(self.u32()?),
            4 => Exceeded::Stack(self.u32()?),
            5 => Exceeded::Elements {
                asked: self.u64()?,
                ceiling: self.u64()?,
            },
            6 => Exceeded::StringUnits {
                asked: self.u64()?,
                ceiling: self.u64()?,
            },
            _ => return Err(WireError::Invalid("budget")),
        })
    }

    /// Succeeds where nothing remains.
    fn finish(self) -> Result<(), WireError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(WireError::Trailing(self.rest.len()))
        }
    }
}
