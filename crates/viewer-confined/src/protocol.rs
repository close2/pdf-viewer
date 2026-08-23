//! The wire format between the host and the confined viewer.
//!
//! `viewer-core`'s vocabulary, made of bytes. Everything here is fixed-width, big-endian and
//! length-checked before anything is allocated from it, for the reason `pdf_sandbox::protocol`
//! gives: the confined side is the untrusted side of this boundary, so a length it states is a
//! claim rather than a fact.
//!
//! # Two properties this module exists to hold
//!
//! **Nothing is dropped in silence.** Every `match` over a `viewer_core` enum here names every
//! variant, so a message added to that crate fails to compile in this one rather than falling
//! into a catch-all arm — and every `let` over a `pdf-model` *struct* in [`panels`] names every
//! field, which is the same property for a type that has no arms. The two messages this transport
//! deliberately does not carry —
//! [`viewer_core::Command::RenderReady`] and [`viewer_core::Event::NeedsRender`] — are named as
//! [`Uncarried`], which is a refusal a caller can read, and they are not carried because the
//! confined process answers them *itself*: it holds the rasteriser, so the render round trip
//! never crosses the pipe. That is `doc/ui-boundary.md`'s "one boundary, not two".
//!
//! **A message that cannot be decoded is a refusal that says so.** [`ProtocolError`] names what
//! was truncated or unrecognised; nothing here defaults, clamps or guesses.

use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
use pdf_model::restriction::Operation;
use pdf_model::view::{Entered, Markup, WidgetAppearances};
use pdf_render::{Point, Raster, RasterFormat, Rect, Size};
use pdf_sandbox::lockdown::{Confinement, LandlockLevel, SystemCalls};
use pdf_syntax::{Name, ObjectId};
use viewer_core::{
    Answer, Command, DocumentId, Edit, Event, Extraction, Find, FindDirection, FocusMove, Found,
    PageGeometry, PageTarget, PointerAction, PresentationMode, Purpose, Query, RestrictionLevel,
    Selection, Zoom,
};

use crate::Reply;

mod panels;

/// Greeting bytes, changed whenever this format changes incompatibly.
///
/// A host and a worker from different builds must not talk to each other, and the cheapest
/// place to find that out is the first thing either says.
const MAGIC: &[u8; 8] = b"PDFVCF03";

/// Length of the worker's greeting: the magic, the Landlock level, the address-space limit, and
/// whether system calls are filtered — the same three facts `pdf_sandbox`'s own worker reports,
/// because they are the same [`Confinement`].
pub(crate) const HANDSHAKE_LEN: usize = 8 + 1 + 8 + 1;

/// Length of a frame header: the kind and the payload length.
pub(crate) const FRAME_HEADER_LEN: usize = 1 + 8;

/// Largest message either side will read, in bytes.
///
/// A document's bytes and a page's pixels both cross this pipe, so the bound cannot be small:
/// ISO 32000-2 itself is 25 MB and a 4K page of RGBA is 33 MB. Two gibibytes is a bound against
/// a length that is a claim rather than a size, which is the only thing it is for — the reader
/// refuses before it allocates, rather than believing a header and asking for the machine.
pub(crate) const MAX_MESSAGE: u64 = 2 << 30;

/// How many elements a list reserves before it has read any of them.
///
/// See [`Reader::list`]: a count on the wire is a claim, and the claim's *reservation* is a
/// separate cost from the claim's length. Two hundred and fifty-six is past every list a real
/// document produces here — a page's popups, a document's schema columns, an outline level — so
/// the growth path is the exceptional one and the reallocations it costs are nobody's hot loop.
const RESERVE: usize = 256;

/// Frame kind: a command for the viewer, from the host.
pub(crate) const FRAME_COMMAND: u8 = 1;
/// Frame kind: a question for the viewer, from the host.
pub(crate) const FRAME_QUERY: u8 = 2;
/// Frame kind: everything a command caused, from the worker.
pub(crate) const FRAME_EVENTS: u8 = 3;
/// Frame kind: the answer to a question, from the worker.
pub(crate) const FRAME_ANSWER: u8 = 4;
/// Frame kind: the worker refused the message, and this says why.
///
/// A refusal is a *response*, not a transport failure: a host that asked for something this
/// transport does not carry gets a sentence back and keeps its worker, exactly as a malformed
/// image keeps `pdf-sandbox`'s.
pub(crate) const FRAME_REFUSAL: u8 = 5;

/// A message this transport deliberately does not carry, and why.
///
/// Produced where a `match` over `viewer-core`'s vocabulary reaches a variant that stays on one
/// side of the boundary. It is an error rather than a silent skip because a host that asked for
/// something and got nothing back would have no way to tell that from an answer of nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{message} does not cross this boundary: {reason}")]
pub struct Uncarried {
    /// The variant's name, as `viewer-core` spells it.
    pub message: &'static str,
    /// Why it stays where it is.
    pub reason: &'static str,
}

/// Why a message could not be read.
///
/// Every variant names what was being read when it failed, because "malformed input" without a
/// subject is a diagnosis nobody can act on.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// The bytes ended in the middle of a field.
    #[error("the message ended while reading {what}")]
    Truncated {
        /// What was being read.
        what: &'static str,
    },
    /// A discriminant this build does not define.
    #[error("{what} {value} is not one this build defines")]
    Unrecognised {
        /// Which enumeration.
        what: &'static str,
        /// The value on the wire.
        value: u32,
    },
    /// A string field was not UTF-8.
    ///
    /// Every string crossing this boundary has already been decoded from the document's own
    /// encoding by `pdf-syntax`, so one that is not UTF-8 here is a transport fault rather than
    /// a document's.
    #[error("{what} was not UTF-8")]
    NotText {
        /// Which field.
        what: &'static str,
    },
    /// A length field describes more bytes than the message holds.
    #[error("{what} claims {claimed} bytes and the message holds {available}")]
    Overlong {
        /// Which field.
        what: &'static str,
        /// What the length said.
        claimed: usize,
        /// What was left.
        available: usize,
    },
    /// Bytes were left over after a message was read.
    ///
    /// Checked rather than ignored: a trailing field this build does not know about means the
    /// two sides disagree about the format, and the magic did not catch it.
    #[error("{left} byte(s) left after reading {what}")]
    Trailing {
        /// What was read.
        what: &'static str,
        /// How many bytes were not consumed.
        left: usize,
    },
    /// A tree on the wire nests deeper than this reader follows.
    ///
    /// Four of the answers a panel is made of are trees — the outline, the layer order,
    /// §12.3.5.2's folders and a collection's own items — and a decoder that followed one as deep
    /// as it was told to would let a message of a few hundred bytes exhaust the host's stack.
    /// The bound is [`panels::MAX_TREE_DEPTH`] and it is well past what any of the readers that
    /// *produce* these trees will hand over.
    #[error("{what} nests deeper than the {limit} this reader follows")]
    TooDeep {
        /// Which tree.
        what: &'static str,
        /// How deep this reader goes.
        limit: usize,
    },
}

/// Appends fields to a message.
#[derive(Debug, Default)]
pub(crate) struct Writer {
    out: Vec<u8>,
}

impl Writer {
    /// An empty message.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// What has been written.
    pub(crate) fn finish(self) -> Vec<u8> {
        self.out
    }

    fn u8(&mut self, value: u8) -> &mut Self {
        self.out.push(value);
        self
    }

    fn u32(&mut self, value: u32) -> &mut Self {
        self.out.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// A machine-word count, as a fixed 64 bits.
    ///
    /// Fixed width and not `usize`: the two sides of this pipe are the same build today and a
    /// format whose field widths depend on the pointer size is one that stops being true the day
    /// they are not.
    fn usize(&mut self, value: usize) -> &mut Self {
        self.out.extend_from_slice(&as_u64(value).to_be_bytes());
        self
    }

    fn i64(&mut self, value: i64) -> &mut Self {
        self.out.extend_from_slice(&value.to_be_bytes());
        self
    }

    fn u64(&mut self, value: u64) -> &mut Self {
        self.out.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// An `f64` as its bits, for [`Writer::f32`]'s reason.
    fn f64(&mut self, value: f64) -> &mut Self {
        self.out.extend_from_slice(&value.to_bits().to_be_bytes());
        self
    }

    /// An `f32` as its bits.
    ///
    /// Bits rather than a decimal spelling, because a coordinate that made a round trip through
    /// text would be a different number on the other side and this boundary carries geometry a
    /// host draws with.
    fn f32(&mut self, value: f32) -> &mut Self {
        self.out.extend_from_slice(&value.to_bits().to_be_bytes());
        self
    }

    fn bool(&mut self, value: bool) -> &mut Self {
        self.u8(u8::from(value))
    }

    fn bytes(&mut self, value: &[u8]) -> &mut Self {
        self.usize(value.len());
        self.out.extend_from_slice(value);
        self
    }

    fn str(&mut self, value: &str) -> &mut Self {
        self.bytes(value.as_bytes())
    }

    fn option_str(&mut self, value: Option<&str>) -> &mut Self {
        match value {
            Some(text) => self.u8(1).str(text),
            None => self.u8(0),
        }
    }

    fn option_bytes(&mut self, value: Option<&[u8]>) -> &mut Self {
        match value {
            Some(bytes) => self.u8(1).bytes(bytes),
            None => self.u8(0),
        }
    }

    fn option_f32(&mut self, value: Option<f32>) -> &mut Self {
        match value {
            Some(number) => self.u8(1).f32(number),
            None => self.u8(0),
        }
    }

    fn option_i64(&mut self, value: Option<i64>) -> &mut Self {
        match value {
            Some(number) => self.u8(1).i64(number),
            None => self.u8(0),
        }
    }

    fn option_usize(&mut self, value: Option<usize>) -> &mut Self {
        match value {
            Some(number) => self.u8(1).usize(number),
            None => self.u8(0),
        }
    }

    fn option_bool(&mut self, value: Option<bool>) -> &mut Self {
        match value {
            Some(flag) => self.u8(1).bool(flag),
            None => self.u8(0),
        }
    }

    fn option_object(&mut self, value: Option<ObjectId>) -> &mut Self {
        match value {
            Some(object) => {
                self.u8(1);
                self.object(object)
            }
            None => self.u8(0),
        }
    }

    /// A fixed-length run of coordinates: a colour's three, a rectangle's four.
    fn numbers(&mut self, values: &[f32]) -> &mut Self {
        for value in values {
            self.f32(*value);
        }
        self
    }

    fn option_numbers(&mut self, values: Option<&[f32]>) -> &mut Self {
        match values {
            Some(values) => {
                self.u8(1);
                self.numbers(values)
            }
            None => self.u8(0),
        }
    }

    fn strings(&mut self, values: &[String]) -> &mut Self {
        self.usize(values.len());
        for value in values {
            self.str(value);
        }
        self
    }

    fn point(&mut self, (x, y): (f32, f32)) -> &mut Self {
        self.f32(x).f32(y)
    }

    fn quad(&mut self, quad: [f32; 8]) -> &mut Self {
        for value in quad {
            self.f32(value);
        }
        self
    }

    fn document(&mut self, document: DocumentId) -> &mut Self {
        self.u64(document.0)
    }

    fn object(&mut self, object: ObjectId) -> &mut Self {
        self.u32(object.number).u32(u32::from(object.generation))
    }
}

/// Reads fields from a message, refusing rather than defaulting.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader over a whole message.
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    /// Refuses unless everything was consumed.
    fn end(&self, what: &'static str) -> Result<(), ProtocolError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(ProtocolError::Trailing {
                what,
                left: self.rest.len(),
            })
        }
    }

    fn take(&mut self, count: usize, what: &'static str) -> Result<&'a [u8], ProtocolError> {
        let (head, tail) = self
            .rest
            .split_at_checked(count)
            .ok_or(ProtocolError::Truncated { what })?;
        self.rest = tail;
        Ok(head)
    }

    fn u8(&mut self, what: &'static str) -> Result<u8, ProtocolError> {
        self.take(1, what)?
            .first()
            .copied()
            .ok_or(ProtocolError::Truncated { what })
    }

    fn u32(&mut self, what: &'static str) -> Result<u32, ProtocolError> {
        let bytes: [u8; 4] = self
            .take(4, what)?
            .try_into()
            .map_err(|_| ProtocolError::Truncated { what })?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn u64(&mut self, what: &'static str) -> Result<u64, ProtocolError> {
        let bytes: [u8; 8] = self
            .take(8, what)?
            .try_into()
            .map_err(|_| ProtocolError::Truncated { what })?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn i64(&mut self, what: &'static str) -> Result<i64, ProtocolError> {
        let bytes: [u8; 8] = self
            .take(8, what)?
            .try_into()
            .map_err(|_| ProtocolError::Truncated { what })?;
        Ok(i64::from_be_bytes(bytes))
    }

    /// A count, refused where this machine cannot hold it.
    fn usize(&mut self, what: &'static str) -> Result<usize, ProtocolError> {
        let value = self.u64(what)?;
        usize::try_from(value).map_err(|_| ProtocolError::Overlong {
            what,
            claimed: usize::MAX,
            available: self.rest.len(),
        })
    }

    fn f32(&mut self, what: &'static str) -> Result<f32, ProtocolError> {
        Ok(f32::from_bits(self.u32(what)?))
    }

    fn f64(&mut self, what: &'static str) -> Result<f64, ProtocolError> {
        Ok(f64::from_bits(self.u64(what)?))
    }

    fn bool(&mut self, what: &'static str) -> Result<bool, ProtocolError> {
        match self.u8(what)? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            }),
        }
    }

    /// A length-prefixed byte string, checked against what is left before it is taken.
    fn bytes(&mut self, what: &'static str) -> Result<&'a [u8], ProtocolError> {
        let claimed = self.usize(what)?;
        if claimed > self.rest.len() {
            return Err(ProtocolError::Overlong {
                what,
                claimed,
                available: self.rest.len(),
            });
        }
        self.take(claimed, what)
    }

    fn string(&mut self, what: &'static str) -> Result<String, ProtocolError> {
        let bytes = self.bytes(what)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| ProtocolError::NotText { what })
    }

    fn option_string(&mut self, what: &'static str) -> Result<Option<String>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.string(what)?))
        } else {
            Ok(None)
        }
    }

    fn option_bytes(&mut self, what: &'static str) -> Result<Option<Vec<u8>>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.bytes(what)?.to_vec()))
        } else {
            Ok(None)
        }
    }

    fn option_f32(&mut self, what: &'static str) -> Result<Option<f32>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.f32(what)?))
        } else {
            Ok(None)
        }
    }

    fn option_i64(&mut self, what: &'static str) -> Result<Option<i64>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.i64(what)?))
        } else {
            Ok(None)
        }
    }

    fn option_usize(&mut self, what: &'static str) -> Result<Option<usize>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.usize(what)?))
        } else {
            Ok(None)
        }
    }

    fn option_bool(&mut self, what: &'static str) -> Result<Option<bool>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.bool(what)?))
        } else {
            Ok(None)
        }
    }

    fn option_object(&mut self, what: &'static str) -> Result<Option<ObjectId>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.object(what)?))
        } else {
            Ok(None)
        }
    }

    /// A colour's three components.
    fn colour(&mut self, what: &'static str) -> Result<[f32; 3], ProtocolError> {
        Ok([self.f32(what)?, self.f32(what)?, self.f32(what)?])
    }

    fn option_colour(&mut self, what: &'static str) -> Result<Option<[f32; 3]>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.colour(what)?))
        } else {
            Ok(None)
        }
    }

    /// A rectangle's four numbers, in the order the file states them.
    fn rect(&mut self, what: &'static str) -> Result<[f32; 4], ProtocolError> {
        Ok([
            self.f32(what)?,
            self.f32(what)?,
            self.f32(what)?,
            self.f32(what)?,
        ])
    }

    fn option_rect(&mut self, what: &'static str) -> Result<Option<[f32; 4]>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.rect(what)?))
        } else {
            Ok(None)
        }
    }

    /// A length-prefixed list, read one element at a time.
    ///
    /// **Two bounds, and the second is the one that took a round to see.** The first is the
    /// obvious one: every element of every list this format carries costs at least one byte, so a
    /// count larger than the bytes that are left is a claim rather than a length and is refused
    /// before anything is read.
    ///
    /// The second is what the count is allowed to *reserve*. `Vec::with_capacity(count)` after
    /// that check is still `count × size_of::<T>()` bytes, and [`MAX_MESSAGE`] is two gibibytes —
    /// so a subverted worker sending nine bytes of header and a count of 2^31 would have the host
    /// ask its allocator for tens of gibibytes of `String` headers and abort. The list therefore
    /// reserves at most [`RESERVE`] elements and grows into the rest, which costs a handful of
    /// reallocations on a list longer than that and bounds a lie at a few kilobytes.
    fn list<T>(
        &mut self,
        what: &'static str,
        mut element: impl FnMut(&mut Self) -> Result<T, ProtocolError>,
    ) -> Result<Vec<T>, ProtocolError> {
        let count = self.usize(what)?;
        if count > self.rest.len() {
            return Err(ProtocolError::Overlong {
                what,
                claimed: count,
                available: self.rest.len(),
            });
        }
        let mut out = Vec::with_capacity(count.min(RESERVE));
        for _ in 0..count {
            out.push(element(self)?);
        }
        Ok(out)
    }

    fn strings(&mut self, what: &'static str) -> Result<Vec<String>, ProtocolError> {
        self.list(what, |reader| reader.string(what))
    }

    fn point(&mut self, what: &'static str) -> Result<(f32, f32), ProtocolError> {
        Ok((self.f32(what)?, self.f32(what)?))
    }

    fn quad(&mut self, what: &'static str) -> Result<[f32; 8], ProtocolError> {
        let mut quad = [0.0; 8];
        for value in &mut quad {
            *value = self.f32(what)?;
        }
        Ok(quad)
    }

    fn document(&mut self, what: &'static str) -> Result<DocumentId, ProtocolError> {
        Ok(DocumentId(self.u64(what)?))
    }

    fn object(&mut self, what: &'static str) -> Result<ObjectId, ProtocolError> {
        let number = self.u32(what)?;
        let generation =
            u16::try_from(self.u32(what)?).map_err(|_| ProtocolError::Unrecognised {
                what: "a generation number",
                value: u32::MAX,
            })?;
        Ok(ObjectId::new(number, generation))
    }
}

/// A payload's kind and length, to be written in front of it.
///
/// **Nine bytes, written separately from the payload rather than in front of a copy of it.** A
/// document is 19.2 MB and a raster is 4.1 MB, so a `frame` that concatenated would be one whole
/// extra pass over the largest thing this transport carries — and the pipe's own cost for those
/// bytes is about a tenth of what the copies around it cost (ADR 0241). Two `write_all` calls on
/// a pipe are two system calls; the concatenation was megabytes of memory traffic and the page
/// faults to go with it.
pub(crate) fn header(kind: u8, length: usize) -> [u8; FRAME_HEADER_LEN] {
    let mut out = [0u8; FRAME_HEADER_LEN];
    out[0] = kind;
    out[1..].copy_from_slice(&as_u64(length).to_be_bytes());
    out
}

/// Prefixes a payload with its kind and length, in one buffer.
///
/// [`header`] is what the two ends actually write. This is for the tests below, which want a whole
/// frame as a value in order to take it apart again.
#[cfg(test)]
pub(crate) fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len().saturating_add(FRAME_HEADER_LEN));
    out.extend_from_slice(&header(kind, payload.len()));
    out.extend_from_slice(payload);
    out
}

/// Reads a frame header, or `None` if the kind is not one this build defines or the length is
/// past [`MAX_MESSAGE`].
pub(crate) fn parse_frame_header(header: [u8; FRAME_HEADER_LEN]) -> Option<(u8, usize)> {
    let kind = *header.first()?;
    if !matches!(
        kind,
        FRAME_COMMAND | FRAME_QUERY | FRAME_EVENTS | FRAME_ANSWER | FRAME_REFUSAL
    ) {
        return None;
    }
    let bytes: [u8; 8] = header.get(1..9)?.try_into().ok()?;
    let length = u64::from_be_bytes(bytes);
    if length > MAX_MESSAGE {
        return None;
    }
    usize::try_from(length).ok().map(|length| (kind, length))
}

/// A count as the fixed-width number this format carries.
///
/// `try_from` rather than `as`: on every platform this compiles for `usize` is at most 64 bits,
/// so the conversion cannot fail — and on a hypothetical wider one the fallback is a length the
/// reader refuses as [`ProtocolError::Overlong`] rather than a number that is quietly wrong.
fn as_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Encodes the worker's greeting.
pub(crate) fn encode_handshake(confinement: Confinement) -> [u8; HANDSHAKE_LEN] {
    let mut greeting = [0u8; HANDSHAKE_LEN];
    let (magic, rest) = greeting.split_at_mut(8);
    magic.copy_from_slice(MAGIC);
    let (level, rest) = rest.split_at_mut(1);
    level[0] = match confinement.landlock {
        LandlockLevel::Enforced => 2,
        LandlockLevel::Partial => 1,
        LandlockLevel::Unavailable => 0,
    };
    let (limit, filtered) = rest.split_at_mut(8);
    limit.copy_from_slice(&confinement.address_space_limit.to_be_bytes());
    filtered[0] = u8::from(confinement.system_calls == SystemCalls::Filtered);
    greeting
}

/// Reads the worker's greeting, or `None` if it is not one.
pub(crate) fn parse_handshake(greeting: &[u8; HANDSHAKE_LEN]) -> Option<Confinement> {
    let (magic, rest) = greeting.split_at(8);
    if magic != MAGIC {
        return None;
    }
    let (level, rest) = rest.split_at(1);
    let landlock = match level.first()? {
        2 => LandlockLevel::Enforced,
        1 => LandlockLevel::Partial,
        0 => LandlockLevel::Unavailable,
        _ => return None,
    };
    let (limit, filtered) = rest.split_at(8);
    let bytes: [u8; 8] = limit.try_into().ok()?;
    let system_calls = match filtered.first()? {
        1 => SystemCalls::Filtered,
        0 => SystemCalls::Unfiltered,
        _ => return None,
    };
    Some(Confinement {
        landlock,
        address_space_limit: u64::from_be_bytes(bytes),
        system_calls,
    })
}

/// Command discriminants. One per variant of [`viewer_core::Command`] that crosses.
mod command_kind {
    pub(super) const OPEN: u8 = 1;
    pub(super) const TICK: u8 = 2;
    pub(super) const CLOSE: u8 = 3;
    pub(super) const FOCUS: u8 = 4;
    pub(super) const RESIZE: u8 = 5;
    pub(super) const GO_TO: u8 = 6;
    pub(super) const ZOOM: u8 = 7;
    pub(super) const SCROLL: u8 = 8;
    pub(super) const RESTRICT: u8 = 9;
    pub(super) const EDIT: u8 = 10;
    pub(super) const UNDO: u8 = 11;
    pub(super) const REDO: u8 = 12;
    pub(super) const EXTRACT: u8 = 13;
    pub(super) const SAVE: u8 = 14;
    pub(super) const SELECT: u8 = 15;
    pub(super) const FOCUSED: u8 = 16;
    pub(super) const ACTIVATE: u8 = 17;
    pub(super) const SET_GROUP: u8 = 18;
    pub(super) const POINTER: u8 = 19;
    pub(super) const SUPPLY: u8 = 20;
    pub(super) const DELEGATE: u8 = 21;
    pub(super) const FIND: u8 = 22;
    pub(super) const PRESENT: u8 = 23;
    pub(super) const LAYOUT: u8 = 24;
}

/// Encodes one command.
///
/// # Errors
///
/// [`Uncarried`] for [`Command::RenderReady`], which the confined process answers itself.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn encode_command(command: &Command) -> Result<Vec<u8>, Uncarried> {
    use command_kind as k;

    let mut writer = Writer::new();
    match command {
        Command::Open {
            id,
            bytes,
            password,
            fragment,
        } => {
            writer
                .u8(k::OPEN)
                .document(*id)
                .bytes(bytes)
                .option_str(password.as_ref().map(viewer_core::Secret::reveal))
                .option_str(fragment.as_deref());
        }
        Command::Tick { millis } => {
            writer.u8(k::TICK).u32(*millis);
        }
        Command::Close(document) => {
            writer.u8(k::CLOSE).document(*document);
        }
        Command::Focus(document) => {
            writer.u8(k::FOCUS).document(*document);
        }
        Command::Resize {
            width,
            height,
            scale,
        } => {
            writer.u8(k::RESIZE).u32(*width).u32(*height).f32(*scale);
        }
        Command::GoTo(target) => {
            writer.u8(k::GO_TO);
            encode_page_target(&mut writer, *target);
        }
        Command::Zoom { zoom, at } => {
            writer.u8(k::ZOOM);
            encode_zoom(&mut writer, *zoom);
            match at {
                Some(point) => {
                    writer.u8(1).point(*point);
                }
                None => {
                    writer.u8(0);
                }
            }
        }
        Command::Scroll { dx, dy } => {
            writer.u8(k::SCROLL).f32(*dx).f32(*dy);
        }
        Command::Restrict(level) => {
            writer.u8(k::RESTRICT).u8(match level {
                RestrictionLevel::On => 0,
                RestrictionLevel::Off => 1,
            });
        }
        // Table 29's arrangement crosses for the reason every other policy value does: the
        // confined process is the one that decides which pages to interpret and where each of
        // them lands, and only the host knows what the person reading has chosen.
        Command::Layout(layout) => {
            writer.u8(k::LAYOUT).u8(panels::layout_code(*layout));
        }
        // §12.4.4's mode crosses for the reason every other policy does: the confined process
        // holds the document and therefore §12.4.4.2's current navigation node, and whether a
        // presentation is running is a fact about a window on the other side of the pipe.
        Command::Present(mode) => {
            writer.u8(k::PRESENT).u8(match mode {
                PresentationMode::Off => 0,
                PresentationMode::On => 1,
            });
        }
        // §6.3.2.2's instruction crosses the confinement like every other policy: the confined
        // process interprets, so the party that draws the widgets has to tell it.
        Command::Delegate(appearances) => {
            writer.u8(k::DELEGATE).u8(match appearances {
                WidgetAppearances::Drawn => 0,
                WidgetAppearances::Delegated => 1,
            });
        }
        Command::Edit(edit) => {
            writer.u8(k::EDIT);
            encode_edit(&mut writer, edit);
        }
        Command::Undo => {
            writer.u8(k::UNDO);
        }
        Command::Redo => {
            writer.u8(k::REDO);
        }
        Command::Extract { name } => {
            writer.u8(k::EXTRACT).str(name);
        }
        Command::Save => {
            writer.u8(k::SAVE);
        }
        Command::Select(selection) => {
            writer.u8(k::SELECT).u8(match selection {
                Selection::All => 0,
                Selection::None => 1,
            });
        }
        Command::Find(find) => {
            writer.u8(k::FIND);
            match find {
                Find::Start { needle, direction } => {
                    writer.u8(0).str(needle).u8(match direction {
                        FindDirection::Forward => 0,
                        FindDirection::Backward => 1,
                    });
                }
                Find::Continue => {
                    writer.u8(1);
                }
                Find::Stop => {
                    writer.u8(2);
                }
            }
        }
        Command::Focused(move_) => {
            writer.u8(k::FOCUSED).u8(match move_ {
                FocusMove::Next => 0,
                FocusMove::Previous => 1,
                FocusMove::None => 2,
            });
        }
        Command::Activate(object) => {
            writer.u8(k::ACTIVATE).object(*object);
        }
        Command::SetGroup { group, on } => {
            writer.u8(k::SET_GROUP).object(*group).bool(*on);
        }
        Command::Pointer { at, action } => {
            writer.u8(k::POINTER).point(*at).u8(match action {
                PointerAction::Moved => 0,
                PointerAction::Pressed => 1,
                PointerAction::Dragged => 2,
                PointerAction::Released => 3,
            });
        }
        Command::Supply { purpose, bytes } => {
            writer.u8(k::SUPPLY).u8(match purpose {
                Purpose::ImportData => 0,
            });
            match bytes {
                Some(bytes) => {
                    writer.u8(1).bytes(bytes);
                }
                None => {
                    writer.u8(0);
                }
            }
        }
        Command::RenderReady { .. } => {
            return Err(Uncarried {
                message: "Command::RenderReady",
                reason: "the confined process holds the rasteriser, so a render request never \
                         leaves it and the answer to one never comes back in",
            });
        }
    }
    Ok(writer.finish())
}

/// Reads one command.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated, a discriminant is not one this build defines,
/// or bytes are left over.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn decode_command(bytes: &[u8]) -> Result<Command, ProtocolError> {
    use command_kind as k;

    let mut reader = Reader::new(bytes);
    let what = "a command";
    let command = match reader.u8(what)? {
        k::OPEN => Command::Open {
            id: reader.document(what)?,
            bytes: reader.bytes("a document's bytes")?.to_vec(),
            password: reader.option_string("a password")?.map(Into::into),
            fragment: reader.option_string("a fragment identifier")?,
        },
        k::TICK => Command::Tick {
            millis: reader.u32("a tick")?,
        },
        k::CLOSE => Command::Close(reader.document(what)?),
        k::FOCUS => Command::Focus(reader.document(what)?),
        k::RESIZE => Command::Resize {
            width: reader.u32("a viewport width")?,
            height: reader.u32("a viewport height")?,
            scale: reader.f32("a display scale")?,
        },
        k::GO_TO => Command::GoTo(decode_page_target(&mut reader)?),
        k::ZOOM => {
            let zoom = decode_zoom(&mut reader)?;
            let at = if reader.bool("a zoom's anchor")? {
                Some(reader.point("a zoom's anchor")?)
            } else {
                None
            };
            Command::Zoom { zoom, at }
        }
        k::SCROLL => Command::Scroll {
            dx: reader.f32("a scroll")?,
            dy: reader.f32("a scroll")?,
        },
        k::LAYOUT => Command::Layout(panels::layout_of(reader.u8("a page layout")?)?),
        k::RESTRICT => Command::Restrict(match reader.u8("a restriction level")? {
            0 => RestrictionLevel::On,
            1 => RestrictionLevel::Off,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a restriction level",
                    value: u32::from(value),
                });
            }
        }),
        k::PRESENT => Command::Present(match reader.u8("a presentation mode")? {
            0 => PresentationMode::Off,
            1 => PresentationMode::On,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a presentation mode",
                    value: u32::from(value),
                });
            }
        }),
        k::DELEGATE => Command::Delegate(match reader.u8("a widget-appearance policy")? {
            0 => WidgetAppearances::Drawn,
            1 => WidgetAppearances::Delegated,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a widget-appearance policy",
                    value: u32::from(value),
                });
            }
        }),
        k::EDIT => Command::Edit(decode_edit(&mut reader)?),
        k::UNDO => Command::Undo,
        k::REDO => Command::Redo,
        k::EXTRACT => Command::Extract {
            name: reader.string("an attachment's name")?,
        },
        k::SAVE => Command::Save,
        k::FIND => Command::Find(match reader.u8("a find step")? {
            0 => Find::Start {
                needle: reader.string("a search string")?,
                direction: match reader.u8("a search direction")? {
                    0 => FindDirection::Forward,
                    1 => FindDirection::Backward,
                    other => {
                        return Err(ProtocolError::Unrecognised {
                            what: "a search direction",
                            value: other.into(),
                        });
                    }
                },
            },
            1 => Find::Continue,
            2 => Find::Stop,
            other => {
                return Err(ProtocolError::Unrecognised {
                    what: "a find step",
                    value: other.into(),
                });
            }
        }),
        k::SELECT => Command::Select(match reader.u8("a selection")? {
            0 => Selection::All,
            1 => Selection::None,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a selection",
                    value: u32::from(value),
                });
            }
        }),
        k::FOCUSED => Command::Focused(match reader.u8("a focus move")? {
            0 => FocusMove::Next,
            1 => FocusMove::Previous,
            2 => FocusMove::None,
            value => {
                return Err(ProtocolError::Unrecognised {
                    what: "a focus move",
                    value: u32::from(value),
                });
            }
        }),
        k::ACTIVATE => Command::Activate(reader.object("an object")?),
        k::SET_GROUP => Command::SetGroup {
            group: reader.object("an optional content group")?,
            on: reader.bool("a group's state")?,
        },
        k::POINTER => Command::Pointer {
            at: reader.point("a pointer position")?,
            action: match reader.u8("a pointer action")? {
                0 => PointerAction::Moved,
                1 => PointerAction::Pressed,
                2 => PointerAction::Dragged,
                3 => PointerAction::Released,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a pointer action",
                        value: u32::from(value),
                    });
                }
            },
        },
        k::SUPPLY => Command::Supply {
            purpose: match reader.u8("a purpose")? {
                0 => Purpose::ImportData,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a purpose",
                        value: u32::from(value),
                    });
                }
            },
            bytes: if reader.bool("a supplied file")? {
                Some(reader.bytes("a supplied file")?.to_vec())
            } else {
                None
            },
        },
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    reader.end(what)?;
    Ok(command)
}

fn encode_page_target(writer: &mut Writer, target: PageTarget) {
    match target {
        PageTarget::Index(index) => {
            writer.u8(0).usize(index);
        }
        PageTarget::First => {
            writer.u8(1);
        }
        PageTarget::Last => {
            writer.u8(2);
        }
        PageTarget::Next => {
            writer.u8(3);
        }
        PageTarget::Previous => {
            writer.u8(4);
        }
        PageTarget::Relative(by) => {
            // `try_from` for `as_u64`'s reason: `isize` is at most 64 bits everywhere this
            // builds, and a saturated value is one the reader refuses rather than a wrong page.
            writer.u8(5).i64(i64::try_from(by).unwrap_or(i64::MAX));
        }
    }
}

fn decode_page_target(reader: &mut Reader<'_>) -> Result<PageTarget, ProtocolError> {
    let what = "a page target";
    Ok(match reader.u8(what)? {
        0 => PageTarget::Index(reader.usize("a page index")?),
        1 => PageTarget::First,
        2 => PageTarget::Last,
        3 => PageTarget::Next,
        4 => PageTarget::Previous,
        5 => PageTarget::Relative(isize::try_from(reader.i64("a relative page")?).map_err(
            |_| ProtocolError::Unrecognised {
                what: "a relative page",
                value: u32::MAX,
            },
        )?),
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    })
}

fn encode_zoom(writer: &mut Writer, zoom: Zoom) {
    match zoom {
        Zoom::FitPage => {
            writer.u8(0);
        }
        Zoom::FitWidth => {
            writer.u8(1);
        }
        Zoom::FitHeight => {
            writer.u8(2);
        }
        Zoom::Scale(scale) => {
            writer.u8(3).f32(scale);
        }
        Zoom::In => {
            writer.u8(4);
        }
        Zoom::Out => {
            writer.u8(5);
        }
    }
}

fn decode_zoom(reader: &mut Reader<'_>) -> Result<Zoom, ProtocolError> {
    let what = "a zoom";
    Ok(match reader.u8(what)? {
        0 => Zoom::FitPage,
        1 => Zoom::FitWidth,
        2 => Zoom::FitHeight,
        3 => Zoom::Scale(reader.f32("a magnification")?),
        4 => Zoom::In,
        5 => Zoom::Out,
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    })
}

fn encode_edit(writer: &mut Writer, edit: &Edit) {
    match edit {
        // §12.7.5.4's three shapes, carried since the four-hundred-and-twelfth session: this was
        // one optional string until Table 233 bit 22's list box needed several. ADR 0248.
        Edit::SetField { field, value } => {
            writer.u8(0).str(field);
            match value {
                Entered::Cleared => {
                    writer.u8(0);
                }
                Entered::Text(text) => {
                    writer.u8(1).str(text);
                }
                Entered::Chosen(indices) => {
                    writer.u8(2).usize(indices.len());
                    for index in indices {
                        writer.usize(*index);
                    }
                }
            }
        }
        Edit::Markup { kind, colour } => {
            writer
                .u8(1)
                .u8(match kind {
                    Markup::Highlight => 0,
                    Markup::Underline => 1,
                    Markup::StrikeOut => 2,
                    Markup::Squiggly => 3,
                })
                .f32(colour[0])
                .f32(colour[1])
                .f32(colour[2]);
        }
        // §12.5.6.6, carried since the four-hundred-and-first session. ADR 0238.
        Edit::FreeText { from, to, colour } => {
            writer
                .u8(2)
                .point(*from)
                .point(*to)
                .f32(colour[0])
                .f32(colour[1])
                .f32(colour[2]);
        }
        Edit::SetFreeText { annotation, text } => {
            writer.u8(3).object(*annotation).str(text);
        }
    }
}

fn decode_edit(reader: &mut Reader<'_>) -> Result<Edit, ProtocolError> {
    let what = "an edit";
    Ok(match reader.u8(what)? {
        0 => Edit::SetField {
            field: reader.string("a field name")?,
            value: {
                let what = "a field value";
                match reader.u8(what)? {
                    0 => Entered::Cleared,
                    1 => Entered::Text(reader.string(what)?),
                    2 => Entered::Chosen(
                        reader.list("a chosen option", |reader| reader.usize("a chosen option"))?,
                    ),
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what,
                            value: u32::from(value),
                        });
                    }
                }
            },
        },
        1 => Edit::Markup {
            kind: match reader.u8("a markup kind")? {
                0 => Markup::Highlight,
                1 => Markup::Underline,
                2 => Markup::StrikeOut,
                3 => Markup::Squiggly,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a markup kind",
                        value: u32::from(value),
                    });
                }
            },
            colour: [
                reader.f32("a markup colour")?,
                reader.f32("a markup colour")?,
                reader.f32("a markup colour")?,
            ],
        },
        2 => Edit::FreeText {
            from: reader.point("a free text corner")?,
            to: reader.point("a free text corner")?,
            colour: [
                reader.f32("a free text colour")?,
                reader.f32("a free text colour")?,
                reader.f32("a free text colour")?,
            ],
        },
        3 => Edit::SetFreeText {
            annotation: reader.object("a free text annotation")?,
            text: reader.string("a free text annotation's contents")?,
        },
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    })
}

/// Event discriminants. One per variant of [`viewer_core::Event`] that crosses.
mod event_kind {
    pub(super) const OPENED: u8 = 1;
    pub(super) const OPEN_FAILED: u8 = 2;
    pub(super) const PASSWORD_REQUIRED: u8 = 3;
    pub(super) const CLOSED: u8 = 4;
    pub(super) const PAGE_CHANGED: u8 = 5;
    pub(super) const DAMAGE: u8 = 6;
    pub(super) const OPEN_URI: u8 = 7;
    pub(super) const NEEDS_FILE: u8 = 8;
    pub(super) const TRANSITION: u8 = 9;
    pub(super) const DIRTY: u8 = 10;
    pub(super) const SAVED: u8 = 11;
    pub(super) const EXTRACTED: u8 = 12;
    pub(super) const REFUSED: u8 = 13;
    pub(super) const REPORTED: u8 = 14;
    pub(super) const SEARCHED: u8 = 15;
}

/// Encodes one event.
///
/// # Errors
///
/// [`Uncarried`] for [`Event::NeedsRender`], which the confined process answers itself.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn encode_event(event: &Event) -> Result<Vec<u8>, Uncarried> {
    use event_kind as k;

    let mut writer = Writer::new();
    match event {
        Event::Opened { document, pages } => {
            writer.u8(k::OPENED).document(*document).usize(*pages);
        }
        Event::OpenFailed { document, reason } => {
            writer.u8(k::OPEN_FAILED).document(*document).str(reason);
        }
        Event::PasswordRequired { document } => {
            writer.u8(k::PASSWORD_REQUIRED).document(*document);
        }
        Event::Closed(document) => {
            writer.u8(k::CLOSED).document(*document);
        }
        Event::PageChanged {
            document,
            index,
            label,
            of,
            section,
        } => {
            writer
                .u8(k::PAGE_CHANGED)
                .document(*document)
                .usize(*index)
                .option_str(label.as_deref())
                .usize(*of)
                .option_str(section.as_deref());
        }
        Event::Damage(rect) => {
            writer
                .u8(k::DAMAGE)
                .f32(rect.min.x)
                .f32(rect.min.y)
                .f32(rect.max.x)
                .f32(rect.max.y);
        }
        Event::OpenUri { document, uri } => {
            writer.u8(k::OPEN_URI).document(*document).str(uri);
        }
        Event::NeedsFile {
            document,
            purpose,
            name,
        } => {
            writer
                .u8(k::NEEDS_FILE)
                .document(*document)
                .u8(match purpose {
                    Purpose::ImportData => 0,
                })
                .str(name);
        }
        Event::Transition {
            document,
            transition,
        } => {
            writer.u8(k::TRANSITION).document(*document);
            encode_transition(&mut writer, transition);
        }
        Event::Dirty { document, dirty } => {
            writer.u8(k::DIRTY).document(*document).bool(*dirty);
        }
        Event::Searched {
            document,
            found,
            remaining,
            wrapped,
        } => {
            writer.u8(k::SEARCHED).document(*document);
            match found {
                Some(found) => {
                    writer
                        .u8(1)
                        .usize(found.page)
                        .usize(found.range.0)
                        .usize(found.range.1);
                }
                None => {
                    writer.u8(0);
                }
            }
            writer.usize(*remaining).bool(*wrapped);
        }
        Event::Saved { document, bytes } => {
            writer.u8(k::SAVED).document(*document).bytes(bytes);
        }
        Event::Extracted {
            document,
            asked,
            name,
            bytes,
            fragment,
        } => {
            writer
                .u8(k::EXTRACTED)
                .document(*document)
                // What asked for the file, which decides whether the host on the other side of
                // this pipe may write it without a person: Annex O's `ef` is a URI's sentence and
                // a click is a person's. One byte, and the decode below refuses any other value
                // rather than guessing at the safer one.
                .u8(match asked {
                    Extraction::Asked => 0,
                    Extraction::Fragment => 1,
                })
                .str(name)
                .bytes(bytes)
                // §O.2.1's remaining parameters, which travel with the file: a host that opens
                // these bytes as a document hands them straight back as `Command::Open`'s
                // fragment, and one on the far side of a pipe needs them as much as one in the
                // same process (ADR 0431).
                .option_str(fragment.as_deref());
        }
        Event::Refused {
            document,
            operation,
            notes,
        } => {
            writer
                .u8(k::REFUSED)
                .document(*document)
                .u8(match operation {
                    Operation::FillInForm => 0,
                    Operation::Annotate => 1,
                })
                .strings(notes);
        }
        Event::Reported {
            document,
            page,
            notes,
        } => {
            writer.u8(k::REPORTED).document(*document);
            match page {
                Some(page) => {
                    writer.u8(1).usize(*page);
                }
                None => {
                    writer.u8(0);
                }
            }
            writer.strings(notes);
        }
        Event::NeedsRender(_) => {
            return Err(Uncarried {
                message: "Event::NeedsRender",
                reason: "the confined process rasterises what it interprets, so the host is sent \
                         the pixels rather than the display list",
            });
        }
    }
    Ok(writer.finish())
}

/// Reads one event.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated, a discriminant is not one this build defines,
/// or bytes are left over.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn decode_event(bytes: &[u8]) -> Result<Event, ProtocolError> {
    use event_kind as k;

    let mut reader = Reader::new(bytes);
    let what = "an event";
    let event = match reader.u8(what)? {
        k::OPENED => Event::Opened {
            document: reader.document(what)?,
            pages: reader.usize("a page count")?,
        },
        k::OPEN_FAILED => Event::OpenFailed {
            document: reader.document(what)?,
            reason: reader.string("a reason")?,
        },
        k::PASSWORD_REQUIRED => Event::PasswordRequired {
            document: reader.document(what)?,
        },
        k::CLOSED => Event::Closed(reader.document(what)?),
        k::PAGE_CHANGED => Event::PageChanged {
            document: reader.document(what)?,
            index: reader.usize("a page index")?,
            label: reader.option_string("a page label")?,
            of: reader.usize("a page count")?,
            section: reader.option_string("an outline section")?,
        },
        k::DAMAGE => Event::Damage(Rect {
            min: Point::new(reader.f32("a rectangle")?, reader.f32("a rectangle")?),
            max: Point::new(reader.f32("a rectangle")?, reader.f32("a rectangle")?),
        }),
        k::OPEN_URI => Event::OpenUri {
            document: reader.document(what)?,
            uri: reader.string("a URI")?,
        },
        k::NEEDS_FILE => Event::NeedsFile {
            document: reader.document(what)?,
            purpose: match reader.u8("a purpose")? {
                0 => Purpose::ImportData,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a purpose",
                        value: u32::from(value),
                    });
                }
            },
            name: reader.string("a file specification's name")?,
        },
        k::TRANSITION => Event::Transition {
            document: reader.document(what)?,
            transition: decode_transition(&mut reader)?,
        },
        k::SEARCHED => Event::Searched {
            document: reader.document("a document")?,
            found: match reader.u8("whether a search found anything")? {
                0 => None,
                1 => Some(Found {
                    page: reader.usize("a page index")?,
                    range: (
                        reader.usize("a match's first offset")?,
                        reader.usize("a match's last offset")?,
                    ),
                }),
                other => {
                    return Err(ProtocolError::Unrecognised {
                        what: "a search result",
                        value: other.into(),
                    });
                }
            },
            remaining: reader.usize("how many pages are left to search")?,
            wrapped: reader.bool("whether the search wrapped")?,
        },
        k::DIRTY => Event::Dirty {
            document: reader.document(what)?,
            dirty: reader.bool("a dirty flag")?,
        },
        k::SAVED => Event::Saved {
            document: reader.document(what)?,
            bytes: reader.bytes("a saved file")?.to_vec(),
        },
        k::EXTRACTED => Event::Extracted {
            document: reader.document(what)?,
            asked: match reader.u8("what asked for an attachment")? {
                0 => Extraction::Asked,
                1 => Extraction::Fragment,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "what asked for an attachment",
                        value: u32::from(value),
                    });
                }
            },
            name: reader.string("an attachment's name")?,
            bytes: reader.bytes("an attachment")?.to_vec(),
            fragment: reader.option_string("a fragment identifier")?,
        },
        k::REFUSED => Event::Refused {
            document: reader.document(what)?,
            operation: match reader.u8("an operation")? {
                0 => Operation::FillInForm,
                1 => Operation::Annotate,
                value => {
                    return Err(ProtocolError::Unrecognised {
                        what: "an operation",
                        value: u32::from(value),
                    });
                }
            },
            notes: reader.strings("a refusal's notes")?,
        },
        k::REPORTED => Event::Reported {
            document: reader.document(what)?,
            page: if reader.bool("a page number")? {
                Some(reader.usize("a page number")?)
            } else {
                None
            },
            notes: reader.strings("a report's notes")?,
        },
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    reader.end(what)?;
    Ok(event)
}

/// Encodes everything one command caused, as one payload.
///
/// # Errors
///
/// [`Uncarried`] where one of the events does not cross. The whole answer fails rather than the
/// one event being dropped: a host given three of four events would have no way to know.
pub(crate) fn encode_events(events: &[Event]) -> Result<Vec<u8>, Uncarried> {
    let mut writer = Writer::new();
    writer.usize(events.len());
    for event in events {
        writer.bytes(&encode_event(event)?);
    }
    Ok(writer.finish())
}

/// Reads everything one command caused.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated or a discriminant is not one this build defines.
pub(crate) fn decode_events(bytes: &[u8]) -> Result<Vec<Event>, ProtocolError> {
    let what = "a run of events";
    let mut reader = Reader::new(bytes);
    let events = reader.list(what, |reader| decode_event(reader.bytes(what)?))?;
    reader.end(what)?;
    Ok(events)
}

/// Encodes Table 164's transition, whole.
fn encode_transition(writer: &mut Writer, transition: &Transition) {
    match &transition.style {
        Style::Split => writer.u8(0),
        Style::Blinds => writer.u8(1),
        Style::Box => writer.u8(2),
        Style::Wipe => writer.u8(3),
        Style::Dissolve => writer.u8(4),
        Style::Glitter => writer.u8(5),
        Style::Replace => writer.u8(6),
        Style::Fly => writer.u8(7),
        Style::Push => writer.u8(8),
        Style::Cover => writer.u8(9),
        Style::Uncover => writer.u8(10),
        Style::Fade => writer.u8(11),
        Style::Unrecognised(name) => writer.u8(12).bytes(&name.0),
    };
    writer
        .f32(transition.duration)
        .u8(match transition.dimension {
            Dimension::Horizontal => 0,
            Dimension::Vertical => 1,
        })
        .u8(match transition.motion {
            Motion::Inward => 0,
            Motion::Outward => 1,
        });
    match transition.direction {
        Direction::Degrees(degrees) => writer.u8(0).f32(degrees),
        Direction::None => writer.u8(1),
    };
    writer.f32(transition.scale).bool(transition.opaque);
}

/// Reads Table 164's transition.
fn decode_transition(reader: &mut Reader<'_>) -> Result<Transition, ProtocolError> {
    let what = "a transition style";
    let style = match reader.u8(what)? {
        0 => Style::Split,
        1 => Style::Blinds,
        2 => Style::Box,
        3 => Style::Wipe,
        4 => Style::Dissolve,
        5 => Style::Glitter,
        6 => Style::Replace,
        7 => Style::Fly,
        8 => Style::Push,
        9 => Style::Cover,
        10 => Style::Uncover,
        11 => Style::Fade,
        12 => Style::Unrecognised(Name::new(
            reader.bytes("a transition style's name")?.to_vec(),
        )),
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    let duration = reader.f32("a transition's duration")?;
    let dimension = match reader.u8("a transition's dimension")? {
        0 => Dimension::Horizontal,
        1 => Dimension::Vertical,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a transition's dimension",
                value: u32::from(value),
            });
        }
    };
    let motion = match reader.u8("a transition's motion")? {
        0 => Motion::Inward,
        1 => Motion::Outward,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a transition's motion",
                value: u32::from(value),
            });
        }
    };
    let direction = match reader.u8("a transition's direction")? {
        0 => Direction::Degrees(reader.f32("a transition's direction")?),
        1 => Direction::None,
        value => {
            return Err(ProtocolError::Unrecognised {
                what: "a transition's direction",
                value: u32::from(value),
            });
        }
    };
    Ok(Transition {
        style,
        duration,
        dimension,
        motion,
        direction,
        scale: reader.f32("a transition's scale")?,
        opaque: reader.bool("a transition's opacity")?,
    })
}

/// Query discriminants. One per variant of [`viewer_core::Query`] that crosses.
mod query_kind {
    pub(super) const PAGE_COUNT: u8 = 1;
    pub(super) const CURRENT_PAGE: u8 = 2;
    pub(super) const PAGE_GEOMETRY: u8 = 3;
    pub(super) const PAGE_LABEL: u8 = 4;
    pub(super) const LINK_AT: u8 = 5;
    pub(super) const FIELD_AT: u8 = 6;
    pub(super) const CARET: u8 = 7;
    pub(super) const DIRTY: u8 = 8;
    pub(super) const FIND: u8 = 9;
    pub(super) const SELECTION: u8 = 10;
    pub(super) const LOGICAL_SELECTION: u8 = 11;
    pub(super) const FOCUS: u8 = 12;
    pub(super) const FRAME: u8 = 13;
    pub(super) const REPORTS: u8 = 14;
    // The eleven a panel is made of, carried since the three-hundred-and-eighty-sixth session.
    // Each answers with a `pdf-model` type, which is why they were second: `protocol::panels` is
    // the encoding of those types and ADR 0223 is the argument.
    pub(super) const OUTLINE: u8 = 15;
    pub(super) const LAYERS: u8 = 16;
    pub(super) const ATTACHMENTS: u8 = 17;
    pub(super) const COLLECTION: u8 = 18;
    pub(super) const ARTICLES: u8 = 19;
    pub(super) const THUMBNAIL: u8 = 20;
    pub(super) const PROPERTIES: u8 = 21;
    pub(super) const OPENING: u8 = 22;
    pub(super) const PREFERENCES: u8 = 23;
    pub(super) const POPUPS: u8 = 24;
    pub(super) const ACCESSIBILITY_TREE: u8 = 25;
    // The caret's inverse and the shapes over a selected range of a field's value, carried since
    // the three-hundred-and-eighty-eighth session. ADR 0225.
    pub(super) const OFFSET: u8 = 26;
    pub(super) const FIELD_SELECTION: u8 = 27;
    // §12.7's form fields, carried since the three-hundred-and-ninety-eighth session: the sixth of
    // ADR 0235's six chrome populations and the last to cross.
    pub(super) const FIELDS: u8 = 28;
    // §12.5.6.6's annotation at a point, carried since the four-hundred-and-first session: the way
    // in to typing on the one markup subtype whose text is the annotation. ADR 0238.
    pub(super) const FREE_TEXT_AT: u8 = 29;
    // Annex O's highlighted rectangle, carried since the five-hundred-and-twenty-second session:
    // the fragment identifier's own shape, which no host can derive because no host sees the
    // fragment. ADR 0357.
    pub(super) const HIGHLIGHT: u8 = 30;
    // What the page's codes cost the readback, carried since the five-hundred-and-eighty-seventh
    // session: §9.10.2's own "there is no way", counted rather than reported. ADR 0422.
    pub(super) const READBACK: u8 = 31;
}

/// Encodes one question.
///
/// **Every question crosses**, which is the three-hundred-and-eighty-sixth session's change and
/// the reason this function no longer returns [`Uncarried`] for anything. It still returns a
/// `Result` because the answers do — see [`encode_answer`], where a collection value outside
/// Table 47 and an `#[non_exhaustive]` metadata failure are the two refusals left in this module
/// besides the render round trip.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the symmetry with `encode_command` and `encode_answer` is the shape a caller reads, \
              and a question this transport cannot carry is a thing that has existed twice"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's — the same \
              reason `encode_answer` carries this, and splitting it would lose the property this \
              module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn encode_query(query: Query<'_>) -> Result<Vec<u8>, Uncarried> {
    use query_kind as k;

    let mut writer = Writer::new();
    match query {
        Query::PageCount => {
            writer.u8(k::PAGE_COUNT);
        }
        Query::CurrentPage => {
            writer.u8(k::CURRENT_PAGE);
        }
        Query::PageGeometry(index) => {
            writer.u8(k::PAGE_GEOMETRY).usize(index);
        }
        Query::PageLabel(index) => {
            writer.u8(k::PAGE_LABEL).usize(index);
        }
        Query::LinkAt(at) => {
            writer.u8(k::LINK_AT).point(at);
        }
        Query::FieldAt(at) => {
            writer.u8(k::FIELD_AT).point(at);
        }
        Query::Caret { at, offset } => {
            writer.u8(k::CARET).point(at).usize(offset);
        }
        Query::Offset { at, point } => {
            writer.u8(k::OFFSET).point(at).point(point);
        }
        Query::FieldSelection { at, from, to } => {
            writer
                .u8(k::FIELD_SELECTION)
                .point(at)
                .usize(from)
                .usize(to);
        }
        Query::FreeTextAt { at } => {
            writer.u8(k::FREE_TEXT_AT).point(at);
        }
        Query::Dirty => {
            writer.u8(k::DIRTY);
        }
        Query::Find(needle) => {
            writer.u8(k::FIND).str(needle);
        }
        Query::Selection => {
            writer.u8(k::SELECTION);
        }
        Query::LogicalSelection => {
            writer.u8(k::LOGICAL_SELECTION);
        }
        Query::Focus => {
            writer.u8(k::FOCUS);
        }
        Query::Highlight => {
            writer.u8(k::HIGHLIGHT);
        }
        Query::Frame => {
            writer.u8(k::FRAME);
        }
        Query::Reports => {
            writer.u8(k::REPORTS);
        }
        Query::Readback => {
            writer.u8(k::READBACK);
        }
        Query::Outline => {
            writer.u8(k::OUTLINE);
        }
        Query::Layers => {
            writer.u8(k::LAYERS);
        }
        Query::Attachments => {
            writer.u8(k::ATTACHMENTS);
        }
        Query::Collection => {
            writer.u8(k::COLLECTION);
        }
        Query::Articles => {
            writer.u8(k::ARTICLES);
        }
        Query::Thumbnail(index) => {
            writer.u8(k::THUMBNAIL).usize(index);
        }
        Query::Properties => {
            writer.u8(k::PROPERTIES);
        }
        Query::Opening => {
            writer.u8(k::OPENING);
        }
        Query::Preferences => {
            writer.u8(k::PREFERENCES);
        }
        Query::Popups => {
            writer.u8(k::POPUPS);
        }
        Query::Fields => {
            writer.u8(k::FIELDS);
        }
        Query::AccessibilityTree => {
            writer.u8(k::ACCESSIBILITY_TREE);
        }
    }
    Ok(writer.finish())
}

/// A question read on the confined side.
///
/// Owned, because [`Query::Find`] borrows the string a host already has and there is no such
/// string on this side of the pipe — it arrived in the message and has to outlive the reader.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OwnedQuery {
    /// A question carrying nothing a caller has to keep alive.
    Plain(PlainQuery),
    /// [`Query::Find`], with the string it searches for.
    Find(String),
}

/// Every carried question except [`Query::Find`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PlainQuery {
    PageCount,
    CurrentPage,
    PageGeometry(usize),
    PageLabel(usize),
    LinkAt((f32, f32)),
    FieldAt((f32, f32)),
    Caret {
        at: (f32, f32),
        offset: usize,
    },
    Offset {
        at: (f32, f32),
        point: (f32, f32),
    },
    FieldSelection {
        at: (f32, f32),
        from: usize,
        to: usize,
    },
    FreeTextAt {
        at: (f32, f32),
    },
    Dirty,
    Selection,
    LogicalSelection,
    Focus,
    Highlight,
    Frame,
    Reports,
    Readback,
    Outline,
    Layers,
    Attachments,
    Collection,
    Articles,
    Thumbnail(usize),
    Properties,
    Opening,
    Preferences,
    Popups,
    Fields,
    AccessibilityTree,
}

impl OwnedQuery {
    /// The question, borrowed for as long as this value lives.
    pub(crate) fn as_query(&self) -> Query<'_> {
        match self {
            Self::Find(needle) => Query::Find(needle),
            Self::Plain(plain) => match *plain {
                PlainQuery::PageCount => Query::PageCount,
                PlainQuery::CurrentPage => Query::CurrentPage,
                PlainQuery::PageGeometry(index) => Query::PageGeometry(index),
                PlainQuery::PageLabel(index) => Query::PageLabel(index),
                PlainQuery::LinkAt(at) => Query::LinkAt(at),
                PlainQuery::FieldAt(at) => Query::FieldAt(at),
                PlainQuery::Caret { at, offset } => Query::Caret { at, offset },
                PlainQuery::Offset { at, point } => Query::Offset { at, point },
                PlainQuery::FieldSelection { at, from, to } => {
                    Query::FieldSelection { at, from, to }
                }
                PlainQuery::FreeTextAt { at } => Query::FreeTextAt { at },
                PlainQuery::Dirty => Query::Dirty,
                PlainQuery::Selection => Query::Selection,
                PlainQuery::LogicalSelection => Query::LogicalSelection,
                PlainQuery::Focus => Query::Focus,
                PlainQuery::Highlight => Query::Highlight,
                PlainQuery::Frame => Query::Frame,
                PlainQuery::Reports => Query::Reports,
                PlainQuery::Readback => Query::Readback,
                PlainQuery::Outline => Query::Outline,
                PlainQuery::Layers => Query::Layers,
                PlainQuery::Attachments => Query::Attachments,
                PlainQuery::Collection => Query::Collection,
                PlainQuery::Articles => Query::Articles,
                PlainQuery::Thumbnail(index) => Query::Thumbnail(index),
                PlainQuery::Properties => Query::Properties,
                PlainQuery::Opening => Query::Opening,
                PlainQuery::Preferences => Query::Preferences,
                PlainQuery::Popups => Query::Popups,
                PlainQuery::Fields => Query::Fields,
                PlainQuery::AccessibilityTree => Query::AccessibilityTree,
            },
        }
    }
}

/// Reads one question.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated, a discriminant is not one this build defines,
/// or bytes are left over.
pub(crate) fn decode_query(bytes: &[u8]) -> Result<OwnedQuery, ProtocolError> {
    use query_kind as k;

    let mut reader = Reader::new(bytes);
    let what = "a query";
    let query = match reader.u8(what)? {
        k::PAGE_COUNT => OwnedQuery::Plain(PlainQuery::PageCount),
        k::CURRENT_PAGE => OwnedQuery::Plain(PlainQuery::CurrentPage),
        k::PAGE_GEOMETRY => {
            OwnedQuery::Plain(PlainQuery::PageGeometry(reader.usize("a page index")?))
        }
        k::PAGE_LABEL => OwnedQuery::Plain(PlainQuery::PageLabel(reader.usize("a page index")?)),
        k::LINK_AT => OwnedQuery::Plain(PlainQuery::LinkAt(reader.point("a point")?)),
        k::FIELD_AT => OwnedQuery::Plain(PlainQuery::FieldAt(reader.point("a point")?)),
        k::CARET => OwnedQuery::Plain(PlainQuery::Caret {
            at: reader.point("a point")?,
            offset: reader.usize("a caret offset")?,
        }),
        k::OFFSET => OwnedQuery::Plain(PlainQuery::Offset {
            at: reader.point("a point")?,
            point: reader.point("a point")?,
        }),
        k::FIELD_SELECTION => OwnedQuery::Plain(PlainQuery::FieldSelection {
            at: reader.point("a point")?,
            from: reader.usize("a selection offset")?,
            to: reader.usize("a selection offset")?,
        }),
        k::FREE_TEXT_AT => OwnedQuery::Plain(PlainQuery::FreeTextAt {
            at: reader.point("a point")?,
        }),
        k::DIRTY => OwnedQuery::Plain(PlainQuery::Dirty),
        k::FIND => OwnedQuery::Find(reader.string("a search string")?),
        k::SELECTION => OwnedQuery::Plain(PlainQuery::Selection),
        k::LOGICAL_SELECTION => OwnedQuery::Plain(PlainQuery::LogicalSelection),
        k::FOCUS => OwnedQuery::Plain(PlainQuery::Focus),
        k::HIGHLIGHT => OwnedQuery::Plain(PlainQuery::Highlight),
        k::FRAME => OwnedQuery::Plain(PlainQuery::Frame),
        k::REPORTS => OwnedQuery::Plain(PlainQuery::Reports),
        k::READBACK => OwnedQuery::Plain(PlainQuery::Readback),
        k::OUTLINE => OwnedQuery::Plain(PlainQuery::Outline),
        k::LAYERS => OwnedQuery::Plain(PlainQuery::Layers),
        k::ATTACHMENTS => OwnedQuery::Plain(PlainQuery::Attachments),
        k::COLLECTION => OwnedQuery::Plain(PlainQuery::Collection),
        k::ARTICLES => OwnedQuery::Plain(PlainQuery::Articles),
        k::THUMBNAIL => OwnedQuery::Plain(PlainQuery::Thumbnail(reader.usize("a page index")?)),
        k::PROPERTIES => OwnedQuery::Plain(PlainQuery::Properties),
        k::OPENING => OwnedQuery::Plain(PlainQuery::Opening),
        k::PREFERENCES => OwnedQuery::Plain(PlainQuery::Preferences),
        k::POPUPS => OwnedQuery::Plain(PlainQuery::Popups),
        k::FIELDS => OwnedQuery::Plain(PlainQuery::Fields),
        k::ACCESSIBILITY_TREE => OwnedQuery::Plain(PlainQuery::AccessibilityTree),
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    reader.end(what)?;
    Ok(query)
}

/// Answer discriminants. One per variant of [`viewer_core::Answer`] that crosses.
mod answer_kind {
    pub(super) const NONE: u8 = 1;
    pub(super) const COUNT: u8 = 2;
    pub(super) const PAGE: u8 = 3;
    pub(super) const GEOMETRY: u8 = 4;
    pub(super) const LABEL: u8 = 5;
    pub(super) const LINK: u8 = 6;
    pub(super) const SELECTED: u8 = 7;
    pub(super) const FIELD: u8 = 8;
    pub(super) const CARET: u8 = 9;
    pub(super) const FOUND: u8 = 10;
    pub(super) const DIRTY: u8 = 11;
    pub(super) const FOCUS: u8 = 12;
    pub(super) const LOGICAL_SELECTION: u8 = 13;
    pub(super) const FRAME: u8 = 14;
    pub(super) const REPORTS: u8 = 15;
    // The eleven a panel is made of. `protocol::panels` is what each of them costs.
    pub(super) const OUTLINE: u8 = 16;
    pub(super) const LAYERS: u8 = 17;
    pub(super) const ATTACHMENTS: u8 = 18;
    pub(super) const COLLECTION: u8 = 19;
    pub(super) const ARTICLES: u8 = 20;
    pub(super) const THUMBNAIL: u8 = 21;
    pub(super) const PROPERTIES: u8 = 22;
    pub(super) const OPENING: u8 = 23;
    pub(super) const PREFERENCES: u8 = 24;
    pub(super) const POPUPS: u8 = 25;
    pub(super) const ACCESSIBILITY: u8 = 26;
    // The caret's inverse and a field selection's shapes, since the three-hundred-and-eighty-eighth
    // session. ADR 0225.
    pub(super) const OFFSET: u8 = 27;
    pub(super) const FIELD_SELECTION: u8 = 28;
    // §12.7's form fields, the twelfth answer a panel — or a native form — is made of. ADR 0235.
    pub(super) const FIELDS: u8 = 29;
    // §12.5.6.6's annotation and its `/Contents`, since the four-hundred-and-first. ADR 0238.
    pub(super) const FREE_TEXT: u8 = 30;
    // Annex O's highlighted rectangles, since the five-hundred-and-twenty-second. ADR 0357.
    pub(super) const HIGHLIGHTED: u8 = 31;
    // The three per-code counts, since the five-hundred-and-eighty-seventh session. ADR 0422.
    pub(super) const READBACK: u8 = 32;
}

/// Encodes one answer.
///
/// # Errors
///
/// [`Uncarried`] in three places, and each names what it refused: a raster in a pixel layout this
/// build cannot spell, a §7.11.6 collection value outside Table 47's three kinds, and a metadata
/// failure `pdf_model::xmp` added after this build. The eleven panel answers that used to be here
/// cross since the three-hundred-and-eighty-sixth session.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn encode_answer(answer: &Answer<'_>) -> Result<Vec<u8>, Uncarried> {
    use answer_kind as k;

    let mut writer = Writer::new();
    match answer {
        Answer::None => {
            writer.u8(k::NONE);
        }
        Answer::Count(count) => {
            writer.u8(k::COUNT).usize(*count);
        }
        Answer::Page {
            document,
            index,
            label,
            of,
        } => {
            writer
                .u8(k::PAGE)
                .document(*document)
                .usize(*index)
                .option_str(label.as_deref())
                .usize(*of);
        }
        Answer::Geometry(geometry) => {
            writer
                .u8(k::GEOMETRY)
                .f32(geometry.page.width)
                .f32(geometry.page.height)
                .f32(geometry.scale)
                .u32(geometry.width)
                .u32(geometry.height)
                .point(geometry.origin);
        }
        Answer::Label(label) => {
            writer.u8(k::LABEL).str(label);
        }
        Answer::Link(there) => {
            writer.u8(k::LINK).bool(*there);
        }
        Answer::Selected(selected) => {
            writer.u8(k::SELECTED).str(&selected.text);
            writer.usize(selected.quads.len());
            for quad in &selected.quads {
                writer.quad(*quad);
            }
        }
        Answer::Field { name, value } => {
            writer
                .u8(k::FIELD)
                .str(&name.qualified)
                .option_str(name.alternative.as_deref());
            panels::encode_shown(&mut writer, value.as_ref());
        }
        Answer::Caret { from, to } => {
            writer.u8(k::CARET).point(*from).point(*to);
        }
        Answer::Offset(offset) => {
            writer.u8(k::OFFSET).usize(*offset);
        }
        Answer::FieldSelection(quads) => {
            writer.u8(k::FIELD_SELECTION).usize(quads.len());
            for quad in quads {
                writer.quad(*quad);
            }
        }
        Answer::Found(occurrences) => {
            writer.u8(k::FOUND).usize(occurrences.len());
            for occurrence in occurrences {
                writer.usize(occurrence.len());
                for quad in occurrence {
                    writer.quad(*quad);
                }
            }
        }
        Answer::Highlighted(quads) => {
            writer.u8(k::HIGHLIGHTED).usize(quads.len());
            for quad in quads {
                writer.quad(*quad);
            }
        }
        Answer::FreeText { annotation, text } => {
            writer.u8(k::FREE_TEXT).object(*annotation).str(text);
        }
        Answer::Dirty(dirty) => {
            writer.u8(k::DIRTY).bool(*dirty);
        }
        Answer::Focus { object, quad } => {
            writer.u8(k::FOCUS).object(*object).quad(*quad);
        }
        Answer::LogicalSelection(text) => {
            writer.u8(k::LOGICAL_SELECTION).str(text);
        }
        Answer::Frame(frames) => {
            // The raster is why this boundary is worth having: the confined process draws the
            // page and the host is handed pixels, which is the whole of `doc/ui-boundary.md`'s
            // tier 1. `RasterFormat` is written out rather than assumed, because a second
            // format would otherwise be read as the first.
            // Exhaustive since ADR 0247: `RasterFormat` is no longer `#[non_exhaustive]`, so a
            // second pixel layout fails to compile here and has to be given a wire byte
            // deliberately. The *reading* side keeps its refusal, because a byte arriving from
            // the confined process is a claim rather than a variant.
            // **A list since Table 29's `/PageLayout` was obeyed**: `OneColumn` puts several
            // pages in one window, and a wire that carried the first of them would show the host
            // a continuous view with a hole in it.
            writer.u8(k::FRAME).usize(frames.len());
            for frame in frames {
                let format = match frame.raster.format {
                    RasterFormat::Rgba8 => 0,
                };
                writer
                    .usize(frame.page)
                    .u32(frame.raster.width)
                    .u32(frame.raster.height)
                    .u8(format)
                    .bytes(&frame.raster.data)
                    .point(frame.origin);
            }
        }
        // One entry per page the arrangement shows, each carrying its page: the same shape
        // `Answer::Frame` takes and for the same reason (ADR 0445).
        Answer::Reports(pages) => {
            writer.u8(k::REPORTS).usize(pages.len());
            for page in pages {
                writer.usize(page.page).usize(page.notes.len());
                for note in page.notes {
                    writer.str(note);
                }
            }
        }
        Answer::Readback(pages) => {
            writer.u8(k::READBACK).usize(pages.len());
            for page in pages {
                // Destructured rather than written field by field off `shortfall`, so that a
                // field added to either struct fails to compile here instead of crossing as a
                // zero.
                let pdf_model::content::Shortfall {
                    unnamed:
                        pdf_model::content::UnnamedCodes {
                            empty_mapping,
                            incomplete_to_unicode,
                            unlisted_name,
                            unnamed_cid,
                            unaddressable_cid,
                            unnamed_glyph,
                        },
                    without_a_glyph,
                    reaching_a_blank_glyph,
                } = page.shortfall;
                writer
                    .usize(page.page)
                    .usize(empty_mapping)
                    .usize(incomplete_to_unicode)
                    .usize(unlisted_name)
                    .usize(unnamed_cid)
                    .usize(unaddressable_cid)
                    .usize(unnamed_glyph)
                    .usize(without_a_glyph)
                    .usize(reaching_a_blank_glyph);
            }
        }
        Answer::Accessibility(pages) => {
            writer.u8(k::ACCESSIBILITY).usize(pages.len());
            for page in pages {
                writer.usize(page.page);
                panels::encode_accessibility(&mut writer, &page.nodes);
            }
        }
        Answer::Outline(outline) => {
            writer.u8(k::OUTLINE);
            panels::encode_outline(&mut writer, outline);
        }
        Answer::Layers(layers) => {
            writer.u8(k::LAYERS);
            panels::encode_layers(&mut writer, layers);
        }
        Answer::Attachments(attachments) => {
            writer.u8(k::ATTACHMENTS);
            panels::encode_attachments(&mut writer, attachments);
        }
        Answer::Articles(threads) => {
            writer.u8(k::ARTICLES);
            panels::encode_articles(&mut writer, threads);
        }
        Answer::Collection {
            collection,
            initial,
        } => {
            writer.u8(k::COLLECTION);
            panels::encode_collection(&mut writer, collection)?;
            panels::encode_initial(&mut writer, initial);
        }
        Answer::Thumbnail(thumbnail) => {
            writer.u8(k::THUMBNAIL);
            panels::encode_thumbnail(&mut writer, thumbnail);
        }
        Answer::Popups(popups) => {
            writer.u8(k::POPUPS);
            panels::encode_popups(&mut writer, popups);
        }
        Answer::Fields(fields) => {
            writer.u8(k::FIELDS);
            panels::encode_fields(&mut writer, fields);
        }
        Answer::Properties {
            information,
            metadata,
        } => {
            writer.u8(k::PROPERTIES);
            panels::encode_properties(&mut writer, information, metadata.as_ref())?;
        }
        Answer::Opening(opening) => {
            writer.u8(k::OPENING);
            panels::encode_opening(&mut writer, *opening);
        }
        Answer::Preferences(preferences) => {
            writer.u8(k::PREFERENCES);
            panels::encode_preferences(&mut writer, preferences);
        }
    }
    Ok(writer.finish())
}

/// Reads one answer.
///
/// # Errors
///
/// [`ProtocolError`] where a field is truncated, a discriminant is not one this build defines,
/// or bytes are left over.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn decode_answer(bytes: &[u8]) -> Result<Reply, ProtocolError> {
    use answer_kind as k;

    let mut reader = Reader::new(bytes);
    let what = "an answer";
    let answer = match reader.u8(what)? {
        k::NONE => Reply::None,
        k::COUNT => Reply::Count(reader.usize("a page count")?),
        k::PAGE => Reply::Page {
            document: reader.document(what)?,
            index: reader.usize("a page index")?,
            label: reader.option_string("a page label")?,
            of: reader.usize("a page count")?,
        },
        k::GEOMETRY => Reply::Geometry(PageGeometry {
            page: Size {
                width: reader.f32("a page size")?,
                height: reader.f32("a page size")?,
            },
            scale: reader.f32("a scale")?,
            width: reader.u32("a raster width")?,
            height: reader.u32("a raster height")?,
            origin: reader.point("an origin")?,
        }),
        k::LABEL => Reply::Label(reader.string("a page label")?),
        k::LINK => Reply::Link(reader.bool("a link")?),
        k::SELECTED => Reply::Selected {
            text: reader.string("the selected text")?,
            quads: read_quads(&mut reader, "a selection's shapes")?,
        },
        k::FIELD => Reply::Field {
            qualified: reader.string("a field's qualified name")?,
            alternative: reader.option_string("a field's alternative name")?,
            value: panels::decode_shown(&mut reader)?,
        },
        k::CARET => Reply::Caret {
            from: reader.point("a caret")?,
            to: reader.point("a caret")?,
        },
        k::OFFSET => Reply::Offset(reader.usize("a caret offset")?),
        k::FIELD_SELECTION => {
            Reply::FieldSelection(read_quads(&mut reader, "a field selection's shapes")?)
        }
        k::FOUND => {
            let what = "a search result";
            Reply::Found(reader.list(what, |reader| read_quads(reader, what))?)
        }
        k::HIGHLIGHTED => {
            Reply::Highlighted(read_quads(&mut reader, "Annex O's highlighted rectangles")?)
        }
        k::FREE_TEXT => Reply::FreeText {
            annotation: reader.object("a free text annotation")?,
            text: reader.string("a free text annotation's contents")?,
        },
        k::DIRTY => Reply::Dirty(reader.bool("a dirty flag")?),
        k::FOCUS => Reply::Focus {
            object: reader.object("an annotation")?,
            quad: reader.quad("a focus ring")?,
        },
        k::LOGICAL_SELECTION => Reply::LogicalSelection(reader.string("the selected text")?),
        k::FRAME => {
            let count = reader.usize("a frame count")?;
            let mut frames = Vec::new();
            for _ in 0..count {
                let page = reader.usize("a page index")?;
                let width = reader.u32("a raster width")?;
                let height = reader.u32("a raster height")?;
                let format = match reader.u8("a raster format")? {
                    0 => RasterFormat::Rgba8,
                    value => {
                        return Err(ProtocolError::Unrecognised {
                            what: "a raster format",
                            value: u32::from(value),
                        });
                    }
                };
                let data = reader.bytes("a raster")?.to_vec();
                // The worker is the untrusted side, so its dimensions are checked against the
                // bytes it actually sent rather than believed — the same rule `pdf_sandbox`'s
                // parent applies to a decoded image, and for the same reason.
                let expected = usize::try_from(width)
                    .ok()
                    .zip(usize::try_from(height).ok())
                    .and_then(|(width, height)| width.checked_mul(height))
                    .and_then(|pixels| pixels.checked_mul(4))
                    .ok_or(ProtocolError::Overlong {
                        what: "a raster",
                        claimed: usize::MAX,
                        available: data.len(),
                    })?;
                if data.len() != expected {
                    return Err(ProtocolError::Overlong {
                        what: "a raster",
                        claimed: expected,
                        available: data.len(),
                    });
                }
                frames.push(crate::Framed {
                    page,
                    raster: Raster {
                        width,
                        height,
                        format,
                        data,
                    },
                    origin: reader.point("an origin")?,
                });
            }
            Reply::Frame(frames)
        }
        k::REPORTS => Reply::Reports(reader.list("a page's reports", |reader| {
            Ok(crate::Reported {
                page: reader.usize("a report's page")?,
                notes: reader.strings("a report's notes")?,
            })
        })?),
        k::READBACK => Reply::Readback(reader.list("a page's readback", |reader| {
            Ok(crate::ReadShort {
                page: reader.usize("a readback's page")?,
                shortfall: pdf_model::content::Shortfall {
                    unnamed: pdf_model::content::UnnamedCodes {
                        empty_mapping: reader.usize("an unnamed-code count")?,
                        incomplete_to_unicode: reader.usize("an unnamed-code count")?,
                        unlisted_name: reader.usize("an unnamed-code count")?,
                        unnamed_cid: reader.usize("an unnamed-code count")?,
                        unaddressable_cid: reader.usize("an unnamed-code count")?,
                        unnamed_glyph: reader.usize("an unnamed-code count")?,
                    },
                    without_a_glyph: reader.usize("a missing-glyph count")?,
                    reaching_a_blank_glyph: reader.usize("a blank-glyph count")?,
                },
            })
        })?),
        k::OUTLINE => Reply::Outline(panels::decode_outline(&mut reader)?),
        k::LAYERS => Reply::Layers(panels::decode_layers(&mut reader)?),
        k::ATTACHMENTS => Reply::Attachments(panels::decode_attachments(&mut reader)?),
        k::COLLECTION => Reply::Collection {
            collection: Box::new(panels::decode_collection(&mut reader)?),
            initial: panels::decode_initial(&mut reader)?,
        },
        k::ARTICLES => Reply::Articles(panels::decode_articles(&mut reader)?),
        k::THUMBNAIL => Reply::Thumbnail(panels::decode_thumbnail(&mut reader)?),
        k::PROPERTIES => {
            let (information, metadata) = panels::decode_properties(&mut reader)?;
            Reply::Properties {
                information: Box::new(information),
                metadata,
            }
        }
        k::OPENING => Reply::Opening(panels::decode_opening(&mut reader)?),
        k::PREFERENCES => Reply::Preferences(Box::new(panels::decode_preferences(&mut reader)?)),
        k::POPUPS => Reply::Popups(panels::decode_popups(&mut reader)?),
        k::FIELDS => Reply::Fields(panels::decode_fields(&mut reader)?),
        k::ACCESSIBILITY => Reply::Accessibility(reader.list("a page's structure", |reader| {
            Ok(crate::Structured {
                page: reader.usize("a structure tree's page")?,
                nodes: panels::decode_accessibility(reader)?,
            })
        })?),
        value => {
            return Err(ProtocolError::Unrecognised {
                what,
                value: u32::from(value),
            });
        }
    };
    reader.end(what)?;
    Ok(answer)
}

/// Reads a length-prefixed run of quadrilaterals.
fn read_quads(reader: &mut Reader<'_>, what: &'static str) -> Result<Vec<[f32; 8]>, ProtocolError> {
    reader.list(what, |reader| reader.quad(what))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_greeting_round_trips_and_an_unconfined_one_is_legible() {
        let confinement = Confinement {
            landlock: LandlockLevel::Enforced,
            address_space_limit: 4 << 30,
            system_calls: SystemCalls::Filtered,
        };
        assert_eq!(
            parse_handshake(&encode_handshake(confinement)),
            Some(confinement)
        );

        let none = Confinement {
            landlock: LandlockLevel::Unavailable,
            address_space_limit: 0,
            system_calls: SystemCalls::Unfiltered,
        };
        let parsed = parse_handshake(&encode_handshake(none)).unwrap();
        assert!(!parsed.is_enforced());
        assert!(parsed.shortfall().is_some());
    }

    #[test]
    fn a_greeting_from_another_program_is_rejected() {
        let mut noise = [0u8; HANDSHAKE_LEN];
        noise[..8].copy_from_slice(b"usage: p");
        assert_eq!(parse_handshake(&noise), None);
    }

    /// Every command that crosses, encoded and read back.
    ///
    /// The list is written out rather than generated: a variant added to `viewer-core` makes
    /// `encode_command`'s match fail to compile, and this is where somebody then notices that
    /// the new one has no round trip.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one entry per command this transport carries, and the count is `viewer-core`'s. \
                  A second list would be a list somebody forgets to add to"
    )]
    fn every_carried_command_round_trips() {
        let commands = vec![
            Command::Open {
                id: DocumentId(7),
                bytes: b"%PDF-2.0".to_vec(),
                password: Some("secret".to_owned().into()),
                fragment: Some("page=3".to_owned()),
            },
            Command::Tick { millis: 16 },
            Command::Close(DocumentId(7)),
            Command::Focus(DocumentId(7)),
            Command::Resize {
                width: 800,
                height: 1000,
                scale: 2.0,
            },
            Command::GoTo(PageTarget::Index(4)),
            Command::GoTo(PageTarget::First),
            Command::GoTo(PageTarget::Last),
            Command::GoTo(PageTarget::Next),
            Command::GoTo(PageTarget::Previous),
            Command::GoTo(PageTarget::Relative(-3)),
            Command::Zoom {
                zoom: Zoom::Scale(1.5),
                at: Some((10.0, 20.0)),
            },
            Command::Zoom {
                zoom: Zoom::FitPage,
                at: None,
            },
            Command::Zoom {
                zoom: Zoom::FitWidth,
                at: None,
            },
            Command::Zoom {
                zoom: Zoom::FitHeight,
                at: None,
            },
            Command::Zoom {
                zoom: Zoom::In,
                at: None,
            },
            Command::Zoom {
                zoom: Zoom::Out,
                at: None,
            },
            Command::Scroll { dx: -1.5, dy: 2.5 },
            Command::Restrict(RestrictionLevel::Off),
            Command::Restrict(RestrictionLevel::On),
            Command::Present(PresentationMode::On),
            Command::Present(PresentationMode::Off),
            Command::Edit(Edit::SetField {
                field: "A.NOM".to_owned(),
                value: Entered::Text("typed".to_owned()),
            }),
            Command::Edit(Edit::SetField {
                field: "A.NOM".to_owned(),
                value: Entered::Cleared,
            }),
            // §12.7.5.4's list box, since the four-hundred-and-twelfth: Table 233 bit 22 permits
            // several items at once, and all three of `Entered`'s shapes cross so that a confined
            // host has no less than an unconfined one (ADR 0248).
            Command::Edit(Edit::SetField {
                field: "A.NOM".to_owned(),
                value: Entered::Chosen(Vec::new()),
            }),
            Command::Edit(Edit::SetField {
                field: "A.NOM".to_owned(),
                value: Entered::Chosen(vec![0, 2, 5]),
            }),
            Command::Edit(Edit::Markup {
                kind: Markup::Squiggly,
                colour: [1.0, 0.5, 0.0],
            }),
            // §12.5.6.6's two, since the four-hundred-and-first session: a rectangle a person
            // drew, and what they typed in it.
            Command::Edit(Edit::FreeText {
                from: (12.0, 34.0),
                to: (56.0, 78.0),
                colour: [0.7, 0.1, 0.1],
            }),
            Command::Edit(Edit::SetFreeText {
                annotation: ObjectId::new(19, 0),
                text: "Reviewed".to_owned(),
            }),
            Command::Undo,
            Command::Redo,
            Command::Extract {
                name: "attachment.txt".to_owned(),
            },
            Command::Save,
            Command::Select(Selection::All),
            Command::Select(Selection::None),
            // Annex O's `search` and a find bar's *next*, since the four-hundred-and-fourteenth:
            // all three steps and both directions, because a confined host has to be able to
            // drive a search exactly as an unconfined one does (ADR 0250).
            Command::Find(Find::Start {
                needle: "transparency group".to_owned(),
                direction: FindDirection::Forward,
            }),
            Command::Find(Find::Start {
                needle: String::new(),
                direction: FindDirection::Backward,
            }),
            Command::Find(Find::Continue),
            Command::Find(Find::Stop),
            Command::Focused(FocusMove::Next),
            Command::Focused(FocusMove::Previous),
            Command::Focused(FocusMove::None),
            Command::Activate(ObjectId::new(12, 1)),
            Command::SetGroup {
                group: ObjectId::new(3, 0),
                on: true,
            },
            Command::Pointer {
                at: (4.0, 5.0),
                action: PointerAction::Released,
            },
            Command::Supply {
                purpose: Purpose::ImportData,
                bytes: Some(b"%FDF-1.2".to_vec()),
            },
            Command::Supply {
                purpose: Purpose::ImportData,
                bytes: None,
            },
        ];
        for command in &commands {
            let encoded = encode_command(command).unwrap();
            let read = decode_command(&encoded).unwrap();
            assert_eq!(
                format!("{read:?}"),
                format!("{command:?}"),
                "a command changed on the way through"
            );
        }
    }

    /// §7.6.4.1's password crosses whole, which the comparison above can no longer see.
    ///
    /// `Command`'s `Debug` compares every field of every variant, and since the
    /// six-hundred-and-ninety-fifth session a `viewer_core::Secret` prints how many characters it
    /// holds and not which — the property that keeps it out of a launch log. That is exactly the
    /// property that would let a transport corrupt a password into another of the same length with
    /// the test above still green, so the one field the general check went blind to gets its own.
    #[test]
    fn a_password_crosses_the_transport_unchanged() {
        let command = Command::Open {
            id: DocumentId(7),
            bytes: b"%PDF-2.0".to_vec(),
            password: Some("m\u{fc}hsam gew\u{e4}hlt".to_owned().into()),
            fragment: None,
        };
        let read = decode_command(&encode_command(&command).unwrap()).unwrap();
        let Command::Open { password, .. } = read else {
            panic!("an Open decoded as something else");
        };
        assert_eq!(
            password.as_ref().map(viewer_core::Secret::reveal),
            Some("m\u{fc}hsam gew\u{e4}hlt"),
            "the password changed on the way through"
        );
    }

    /// Every event that crosses, encoded and read back.
    #[expect(
        clippy::too_many_lines,
        reason = "test code: one fixture per variant of a `viewer-core` enum, and the count is \
                  that enum's"
    )]
    #[test]
    fn every_carried_event_round_trips() {
        let document = DocumentId(2);
        let events = vec![
            Event::Opened {
                document,
                pages: 1023,
            },
            Event::OpenFailed {
                document,
                reason: "unsupported encryption".to_owned(),
            },
            Event::PasswordRequired { document },
            Event::Closed(document),
            Event::PageChanged {
                document,
                index: 3,
                label: Some("iv".to_owned()),
                of: 5,
                section: None,
            },
            Event::Damage(Rect {
                min: Point::new(0.0, 1.0),
                max: Point::new(2.0, 3.0),
            }),
            Event::Searched {
                document,
                found: Some(Found {
                    page: 7,
                    range: (12, 30),
                }),
                remaining: 0,
                wrapped: true,
            },
            Event::Searched {
                document,
                found: None,
                remaining: 1022,
                wrapped: false,
            },
            Event::OpenUri {
                document,
                uri: "https://example.invalid/".to_owned(),
            },
            Event::NeedsFile {
                document,
                purpose: Purpose::ImportData,
                name: "data.fdf".to_owned(),
            },
            Event::Transition {
                document,
                transition: Transition {
                    style: Style::Fly,
                    duration: 1.5,
                    dimension: Dimension::Vertical,
                    motion: Motion::Outward,
                    direction: Direction::Degrees(315.0),
                    scale: 0.5,
                    opaque: true,
                },
            },
            Event::Transition {
                document,
                transition: Transition {
                    style: Style::Unrecognised(Name::new(b"Swirl".to_vec())),
                    duration: 1.0,
                    dimension: Dimension::Horizontal,
                    motion: Motion::Inward,
                    direction: Direction::None,
                    scale: 1.0,
                    opaque: false,
                },
            },
            Event::Dirty {
                document,
                dirty: true,
            },
            Event::Saved {
                document,
                bytes: b"%PDF-2.0 ...".to_vec(),
            },
            Event::Extracted {
                document,
                asked: Extraction::Asked,
                name: "readme.txt".to_owned(),
                bytes: b"hello".to_vec(),
                fragment: None,
            },
            // Both of §O.2.1's provenances on the wire, because the byte that carries them is what
            // lets the host on the other side decline one and write the other.
            Event::Extracted {
                document,
                asked: Extraction::Fragment,
                name: "readme.txt".to_owned(),
                bytes: b"hello".to_vec(),
                // §O.2.1's remainder, on the wire: `#ef=readme.txt&page=3` is the case the far
                // side has to be able to carry out rather than only to be told about.
                fragment: Some("page=3".to_owned()),
            },
            Event::Refused {
                document,
                operation: Operation::Annotate,
                notes: vec!["this document's author certified it".to_owned()],
            },
            Event::Reported {
                document,
                page: Some(0),
                notes: vec!["one".to_owned(), "two".to_owned()],
            },
            Event::Reported {
                document,
                page: None,
                notes: Vec::new(),
            },
        ];
        for event in &events {
            let encoded = encode_event(event).unwrap();
            let read = decode_event(&encoded).unwrap();
            assert_eq!(
                format!("{read:?}"),
                format!("{event:?}"),
                "an event changed on the way through"
            );
        }
    }

    /// The two messages that do not cross, refused by name rather than dropped.
    ///
    /// This is the property the whole module is built around: a message that stays on one side
    /// is an error a caller can read, because a boundary that silently swallowed one would be
    /// indistinguishable from a viewer that did nothing.
    #[test]
    fn the_two_messages_that_stay_where_they_are_say_so() {
        // A real render request, because a `RenderToken` cannot be constructed outside
        // `viewer-core` — which is itself the point: the token is the confined process's own
        // bookkeeping, and there is no way for a host to make one up.
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
        )
        .unwrap();
        let mut viewer = viewer_core::Viewer::new(400, 500, 1.0);
        let events: Vec<Event> = viewer
            .handle(Command::Open {
                id: DocumentId(1),
                bytes,
                password: None,
                fragment: None,
            })
            .collect();
        let request = events
            .iter()
            .find_map(|event| match event {
                Event::NeedsRender(request) => Some(request.clone()),
                _ => None,
            })
            .unwrap();

        let refused = encode_command(&Command::RenderReady {
            token: request.token,
            rendered: viewer_core::Rendered::Presented,
        })
        .unwrap_err();
        assert_eq!(refused.message, "Command::RenderReady");

        let refused = encode_event(&Event::NeedsRender(request)).unwrap_err();
        assert_eq!(refused.message, "Event::NeedsRender");
    }

    /// Every question `viewer-core` states, encoded and read back.
    ///
    /// **The list used to have two halves** — what crossed and what was refused by name — and the
    /// second half is empty since the three-hundred-and-eighty-sixth session. All of them are
    /// here, written out rather than generated, so that a question added to `viewer-core` makes
    /// `encode_query`'s match fail to compile and somebody then notices there is no round trip
    /// for it.
    #[test]
    fn every_query_is_carried() {
        let carried = [
            Query::PageCount,
            Query::CurrentPage,
            Query::PageGeometry(2),
            Query::PageLabel(2),
            Query::LinkAt((3.0, 4.0)),
            Query::FieldAt((3.0, 4.0)),
            Query::Caret {
                at: (3.0, 4.0),
                offset: 5,
            },
            Query::Offset {
                at: (3.0, 4.0),
                point: (11.0, 12.0),
            },
            Query::FieldSelection {
                at: (3.0, 4.0),
                from: 2,
                to: 6,
            },
            Query::FreeTextAt { at: (3.0, 4.0) },
            Query::Dirty,
            Query::Find("needle"),
            Query::Selection,
            Query::LogicalSelection,
            Query::Focus,
            Query::Highlight,
            Query::Frame,
            Query::Reports,
            Query::Outline,
            Query::Layers,
            Query::Attachments,
            Query::Collection,
            Query::Articles,
            Query::Thumbnail(7),
            Query::Properties,
            Query::Opening,
            Query::Preferences,
            Query::Popups,
            Query::Fields,
            Query::AccessibilityTree,
            Query::Readback,
        ];
        assert_eq!(carried.len(), 31, "every question `viewer-core` states");
        for query in carried {
            let encoded = encode_query(query).unwrap();
            let read = decode_query(&encoded).unwrap();
            assert_eq!(
                format!("{:?}", read.as_query()),
                format!("{query:?}"),
                "a query changed on the way through"
            );
        }
    }

    /// A value for every field of every type the eleven panel answers are made of.
    ///
    /// **Nothing here is a default.** A round trip over `Default::default()` would pass with an
    /// encoder that wrote nothing and a decoder that read nothing, which is the exact defect this
    /// module has to be free of: an encoding that drops a field is a panel showing less on the
    /// confined path than off it, and no gate in this tree looks at a panel. So every `Option` is
    /// `Some`, every `bool` is the opposite of its default where the clause states one, and every
    /// list has more than one element.
    fn a_populated_outline() -> pdf_model::outline::Outline {
        use pdf_model::destination::{Destination, Target, View};
        use pdf_model::outline::{Item, Outline};

        let leaf = Item {
            id: ObjectId::new(9, 2),
            title: "a leaf".to_owned(),
            destination: Some(Destination {
                target: Target::Number(11),
                view: View::FitR {
                    rect: [1.0, 2.0, 3.0, 4.0],
                },
            }),
            open: false,
            italic: true,
            bold: false,
            colour: [0.25, 0.5, 0.75],
            children: Vec::new(),
        };
        Outline {
            items: vec![
                Item {
                    id: ObjectId::new(4, 0),
                    title: "a section".to_owned(),
                    destination: Some(Destination {
                        target: Target::Object(ObjectId::new(5, 0)),
                        view: View::Xyz {
                            left: Some(10.0),
                            top: None,
                            zoom: Some(2.0),
                        },
                    }),
                    open: true,
                    italic: false,
                    bold: true,
                    colour: [1.0, 0.0, 0.0],
                    children: vec![leaf],
                },
                Item {
                    id: ObjectId::new(6, 0),
                    title: "an item with no destination".to_owned(),
                    destination: None,
                    open: false,
                    italic: false,
                    bold: false,
                    colour: [0.0, 0.0, 0.0],
                    children: Vec::new(),
                },
            ],
            stated_count: Some(-3),
        }
    }

    /// One of every shape §12.3.5's collection dictionary can take.
    fn a_populated_collection() -> pdf_model::collection::Collection {
        use pdf_model::collection::{
            Collection, Colours, Field, FieldKind, Folder, Item, Layout, Navigator, Sort, Split,
            SplitDirection, Value, View,
        };

        let mut values = std::collections::BTreeMap::new();
        values.insert(
            "author".to_owned(),
            Value {
                data: pdf_syntax::Object::String(b"a name".as_slice().into()),
                prefix: Some("by ".to_owned()),
            },
        );
        values.insert(
            "revision".to_owned(),
            Value {
                data: pdf_syntax::Object::Real(1.5),
                prefix: None,
            },
        );

        let mut schema = std::collections::BTreeMap::new();
        schema.insert(
            "author".to_owned(),
            Field {
                kind: FieldKind::Text,
                name: "Author".to_owned(),
                order: Some(2),
                visible: false,
                editable: true,
            },
        );
        schema.insert(
            "custom".to_owned(),
            Field {
                kind: FieldKind::Other("Widget".to_owned()),
                name: "Custom".to_owned(),
                order: None,
                visible: true,
                editable: false,
            },
        );

        Collection {
            schema,
            initial: Some("first.pdf".to_owned()),
            view: View::Navigator,
            sort: Some(Sort {
                fields: vec!["author".to_owned(), "revision".to_owned()],
                ascending: vec![true, false],
            }),
            navigator: Some(Navigator {
                layouts: vec![
                    Layout::View(View::Tile),
                    Layout::FilmStrip,
                    Layout::FreeForm,
                    Layout::Linear,
                    Layout::Tree,
                    Layout::Custom("Mosaic".to_owned()),
                ],
            }),
            colours: Colours {
                background: Some([0.1, 0.2, 0.3]),
                card_background: Some([0.4, 0.5, 0.6]),
                card_border: None,
                primary_text: Some([0.7, 0.8, 0.9]),
                secondary_text: Some([1.0, 1.0, 1.0]),
            },
            split: Some(Split {
                direction: SplitDirection::Vertical,
                position: Some(30.0),
            }),
            folders: Some(Folder {
                id: 0,
                name: "root".to_owned(),
                description: Some("everything".to_owned()),
                item: Item {
                    values: values.clone(),
                },
                has_thumbnail: true,
                children: vec![Folder {
                    id: 7,
                    name: "drafts".to_owned(),
                    description: None,
                    item: Item { values },
                    has_thumbnail: false,
                    children: Vec::new(),
                }],
            }),
        }
    }

    /// Every panel answer, encoded and read back, field for field.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "eleven answers with every field of each one populated; the length is the \
                  vocabulary's, and splitting it would hide which of the eleven a failure is in"
    )]
    fn every_panel_answer_round_trips() {
        use pdf_model::article::{Bead, Thread};
        use pdf_model::attachment::{Attachment as FileAttachment, Relationship};
        use pdf_model::metadata::{Information, Trapped};
        use pdf_model::page::Boundary;
        use pdf_model::thumbnail::Thumbnail;
        use pdf_model::viewer_preferences::{
            Direction as TextDirection, Duplex, Opening, PageLayout, PageMode, PrintScaling,
            ViewerPreferences,
        };
        use pdf_model::xmp::{Name as XmpName, Value as XmpValue, Xmp};
        use viewer_core::{AccessibilityNode, Character, Layer, PopupWindow, TextLine};

        // §12.3.3.
        let outline = a_populated_outline();
        let Reply::Outline(read) = round_trip(&Answer::Outline(outline.clone())) else {
            panic!("an outline comes back as one");
        };
        assert_eq!(read, outline, "an outline changed on the way through");

        // §8.11.4.3.
        let layers = vec![
            Layer::Collection {
                label: Some("a heading".to_owned()),
                children: vec![Layer::Group {
                    group: ObjectId::new(12, 0),
                    name: Some("a layer".to_owned()),
                    on: false,
                    locked: true,
                }],
            },
            Layer::Group {
                group: ObjectId::new(13, 1),
                name: None,
                on: true,
                locked: false,
            },
        ];
        let Reply::Layers(read) = round_trip(&Answer::Layers(layers.clone())) else {
            panic!("a layer order comes back as one");
        };
        assert_eq!(read, layers, "a layer order changed on the way through");

        // §7.11.4. The stream is the one thing that does not cross, which is why the expected
        // value is written out rather than derived from the input: an assertion that compared
        // the two types field by field could not have been written at all.
        let attachments = vec![FileAttachment {
            name: "readme.txt".to_owned(),
            file_name: Some("readme.txt".to_owned()),
            description: Some("what this is".to_owned()),
            media_type: Some("text/plain".to_owned()),
            size: Some(42),
            created: Some("D:20240101120000Z".to_owned()),
            modified: Some("D:20240202120000Z".to_owned()),
            checksum: Some(vec![0u8; 16]),
            relationship: Relationship::Other("Annex".to_owned()),
            stream: std::sync::Arc::new(pdf_syntax::Stream {
                dict: pdf_syntax::Dictionary::new(),
                data: b"hello".as_slice().into(),
                decryption_failed: false,
            }),
        }];
        let Reply::Attachments(read) = round_trip(&Answer::Attachments(attachments)) else {
            panic!("an attachment list comes back as one");
        };
        assert_eq!(
            read,
            vec![crate::Attachment {
                name: "readme.txt".to_owned(),
                file_name: Some("readme.txt".to_owned()),
                description: Some("what this is".to_owned()),
                media_type: Some("text/plain".to_owned()),
                size: Some(42),
                created: Some("D:20240101120000Z".to_owned()),
                modified: Some("D:20240202120000Z".to_owned()),
                checksum: Some(vec![0u8; 16]),
                relationship: Relationship::Other("Annex".to_owned()),
            }],
            "an attachment changed on the way through"
        );

        // §12.3.5.
        let collection = a_populated_collection();
        // §12.3.5.1's resolved `/D` crosses beside the dictionary: `Initial::Embedded` is the
        // variant that carries a name, so a codec that dropped the name would show here.
        let initial = pdf_model::collection::Initial::Embedded("<1>letter.pdf".to_owned());
        let Reply::Collection {
            collection: read,
            initial: read_initial,
        } = round_trip(&Answer::Collection {
            collection: collection.clone(),
            initial: initial.clone(),
        })
        else {
            panic!("a collection comes back as one");
        };
        assert_eq!(*read, collection, "a collection changed on the way through");
        assert_eq!(
            read_initial, initial,
            "the initial document changed on the way through"
        );

        // §12.4.3.
        let threads = vec![Thread {
            id: ObjectId::new(20, 0),
            title: Some("a thread".to_owned()),
            beads: vec![
                Bead {
                    id: ObjectId::new(21, 0),
                    page: Some(ObjectId::new(3, 0)),
                    rect: Some([0.0, 1.0, 2.0, 3.0]),
                },
                Bead {
                    id: ObjectId::new(22, 0),
                    page: None,
                    rect: None,
                },
            ],
        }];
        let Reply::Articles(read) = round_trip(&Answer::Articles(threads.clone())) else {
            panic!("article threads come back as themselves");
        };
        assert_eq!(
            read, threads,
            "an article thread changed on the way through"
        );

        // §12.3.4.
        let thumbnail = Thumbnail {
            image: pdf_render::Image {
                width: 2,
                height: 3,
                data: (0u8..24).collect::<Vec<u8>>().into(),
                interpolate: true,
            },
            permitted_colour_space: false,
            permitted_subtype: false,
        };
        let Reply::Thumbnail(read) = round_trip(&Answer::Thumbnail(thumbnail.clone())) else {
            panic!("a thumbnail comes back as one");
        };
        assert_eq!(read, thumbnail, "a thumbnail changed on the way through");

        // §14.3.3 and §14.3.2.
        let information = Information {
            title: Some("a title".to_owned()),
            author: Some("an author".to_owned()),
            subject: Some("a subject".to_owned()),
            keywords: Some("keywords".to_owned()),
            creator: Some("a creator".to_owned()),
            producer: Some("a producer".to_owned()),
            created: Some("D:20200101000000Z".to_owned()),
            modified: Some("D:20210101000000Z".to_owned()),
            trapped: Trapped::Fully,
        };
        let xmp = Xmp::from_properties(vec![
            (
                XmpName {
                    namespace: "http://purl.org/dc/elements/1.1/".to_owned(),
                    local: "title".to_owned(),
                },
                XmpValue::Alt(vec![
                    (Some("x-default".to_owned()), "a title".to_owned()),
                    (None, "no language".to_owned()),
                ]),
            ),
            (
                XmpName {
                    namespace: "http://purl.org/dc/elements/1.1/".to_owned(),
                    local: "creator".to_owned(),
                },
                XmpValue::Seq(vec!["one".to_owned(), "two".to_owned()]),
            ),
            (
                XmpName {
                    namespace: String::new(),
                    local: "unqualified".to_owned(),
                },
                XmpValue::Bag(vec!["a".to_owned()]),
            ),
            (
                XmpName {
                    namespace: "urn:x".to_owned(),
                    local: "simple".to_owned(),
                },
                XmpValue::Text("plain".to_owned()),
            ),
            (
                XmpName {
                    namespace: "urn:x".to_owned(),
                    local: "structured".to_owned(),
                },
                XmpValue::Structure,
            ),
        ]);
        let Reply::Properties {
            information: read_information,
            metadata,
        } = round_trip(&Answer::Properties {
            information: information.clone(),
            metadata: Some(Ok(xmp.clone())),
        })
        else {
            panic!("properties come back as properties");
        };
        assert_eq!(*read_information, information);
        assert_eq!(metadata, Some(Ok(xmp)));

        // Every `XmpError` this build names, including the four budgets whose `&'static str` is
        // reconstructed rather than allocated.
        for error in [
            pdf_model::xmp::XmpError::Undecodable,
            pdf_model::xmp::XmpError::TooLarge { bytes: 1 << 21 },
            pdf_model::xmp::XmpError::NotText,
            pdf_model::xmp::XmpError::Malformed {
                line: 3,
                column: 7,
                detail: "unexpected token".to_owned(),
            },
            pdf_model::xmp::XmpError::Unbalanced {
                detail: "<a></b>".to_owned(),
            },
            pdf_model::xmp::XmpError::TooMuch {
                what: "nesting depth",
            },
            pdf_model::xmp::XmpError::TooMuch { what: "properties" },
            pdf_model::xmp::XmpError::TooMuch {
                what: "array items",
            },
            pdf_model::xmp::XmpError::TooMuch {
                what: "value length",
            },
        ] {
            let Reply::Properties { metadata, .. } = round_trip(&Answer::Properties {
                information: Information::default(),
                metadata: Some(Err(error.clone())),
            }) else {
                panic!("properties come back as properties");
            };
            assert_eq!(metadata, Some(Err(error)));
        }

        // Table 29 and Table 147.
        let opening = Opening {
            mode: PageMode::UseAttachments,
            layout: PageLayout::TwoPageRight,
        };
        let Reply::Opening(read) = round_trip(&Answer::Opening(opening)) else {
            panic!("an opening pair comes back as one");
        };
        assert_eq!(read, opening);

        let preferences = ViewerPreferences {
            hide_toolbar: true,
            hide_menubar: true,
            hide_window_ui: true,
            fit_window: true,
            center_window: true,
            display_doc_title: true,
            non_full_screen_page_mode: PageMode::UseOptionalContent,
            direction: TextDirection::RightToLeft,
            view_area: Boundary::Media,
            view_clip: Boundary::Bleed,
            print_area: Boundary::Trim,
            print_clip: Boundary::Art,
            print_scaling: PrintScaling::NoScaling,
            duplex: Some(Duplex::FlipShortEdge),
            pick_tray_by_pdf_size: Some(true),
            print_page_range: vec![(1, 4), (9, 9)],
            num_copies: Some(3),
            enforce_print_scaling: true,
        };
        let Reply::Preferences(read) = round_trip(&Answer::Preferences(preferences.clone())) else {
            panic!("preferences come back as preferences");
        };
        assert_eq!(*read, preferences);

        // §12.5.6.14.
        let popups = vec![
            PopupWindow {
                annotation: ObjectId::new(30, 0),
                parent: Some(ObjectId::new(31, 0)),
                quad: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
                title: Some("a title".to_owned()),
                text: Some("a note".to_owned()),
                modified: Some("D:20240101000000Z".to_owned()),
                colour: Some(pdf_render::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 0.4,
                }),
            },
            PopupWindow {
                annotation: ObjectId::new(32, 0),
                parent: None,
                quad: [0.0; 8],
                title: None,
                text: None,
                modified: None,
                colour: None,
            },
        ];
        let Reply::Popups(read) = round_trip(&Answer::Popups(popups.clone())) else {
            panic!("popups come back as popups");
        };
        assert_eq!(read, popups);

        // §14.7.
        let nodes = vec![
            AccessibilityNode {
                parent: None,
                role: "Document".to_owned(),
                name: String::new(),
                substituted: false,
                language: Some("en-GB".to_owned()),
                quads: Vec::new(),
                header_scope: None,
                bounds: None,
                control: None,
                annotation: None,
                headers: Vec::new(),
                lines: Vec::new(),
                drawn: None,
            },
            // A `Figure` marks no text, so Table 379's `/BBox` is the only place it has — and
            // a rectangle that did not cross would leave a magnifier nothing to point at.
            AccessibilityNode {
                parent: Some(0),
                role: "Figure".to_owned(),
                name: "a chart of sales".to_owned(),
                substituted: true,
                language: None,
                quads: vec![[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]],
                header_scope: None,
                bounds: Some([8.0, 9.0, 10.0, 11.0]),
                control: None,
                annotation: None,
                headers: Vec::new(),
                lines: Vec::new(),
                // §14.8.3.3's content rectangle beside the stated one, because the host chooses
                // between them and a boundary that carried only one would make that choice here.
                drawn: Some([7.5, 8.5, 10.5, 11.5]),
            },
            // A `TH` whose axis Table 384 states, because a header cell that crossed as a
            // column's when the document called it a row's would be read out backwards.
            AccessibilityNode {
                parent: Some(0),
                role: "TH".to_owned(),
                name: "Region".to_owned(),
                substituted: false,
                language: None,
                quads: Vec::new(),
                header_scope: Some(pdf_model::structure::HeaderScope::Row),
                bounds: None,
                control: None,
                annotation: None,
                headers: Vec::new(),
                lines: Vec::new(),
                drawn: None,
            },
            // §14.8.4.7.2's `Form`, which is one widget annotation: the control it names is
            // what makes a screen reader say "check box, ticked" instead of "group", so a
            // field that did not cross would be the whole of this feature lost on this
            // boundary.
            AccessibilityNode {
                parent: Some(0),
                role: "Form".to_owned(),
                name: String::new(),
                substituted: false,
                language: None,
                quads: Vec::new(),
                header_scope: None,
                bounds: Some([12.0, 13.0, 14.0, 15.0]),
                control: Some(pdf_model::form::Control::CheckBox { on: true }),
                annotation: Some(ObjectId::new(7, 0)),
                headers: Vec::new(),
                lines: Vec::new(),
                drawn: None,
            },
            // And a `TD` that §14.8.4.8.3's search gave the header beside it, because a cell
            // whose headers did not cross would be announced with nothing in front of it.
            AccessibilityNode {
                parent: Some(0),
                role: "TD".to_owned(),
                name: "north".to_owned(),
                substituted: false,
                language: None,
                quads: Vec::new(),
                header_scope: None,
                bounds: None,
                control: None,
                annotation: None,
                headers: vec![2],
                // The lines a caret moves through, which are the only field of this answer whose
                // two halves have to agree: the characters' byte counts sum to the text's length,
                // and `read_lines` refuses a message where they do not. A `TD` whose lines did not
                // cross would be a cell an assistive technology could hear and not move through.
                lines: vec![TextLine {
                    text: "north".to_owned(),
                    characters: vec![
                        Character {
                            bytes: 1,
                            bounds: [0.0, 0.0, 4.0, 10.0],
                        },
                        Character {
                            bytes: 4,
                            bounds: [4.0, 0.0, 20.0, 10.0],
                        },
                    ],
                }],
                // A cell whose only place is what its content drew, which is the population
                // §14.8.3.3's content rectangle exists for and the one this field carries.
                drawn: Some([0.0, 0.0, 20.0, 10.0]),
            },
        ];
        // Two pages, because a column crosses several and a decoder that read one page's list
        // and stopped would pass with one. The second is deliberately **untagged**: an empty
        // list is an answer (§14.7 lets a page state no structure) and a wire that dropped the
        // entry would turn "this page says nothing" into "there is no such page".
        let pages = vec![
            viewer_core::PageStructure {
                page: 3,
                nodes: nodes.clone(),
            },
            viewer_core::PageStructure {
                page: 4,
                nodes: Vec::new(),
            },
        ];
        let Reply::Accessibility(read) = round_trip(&Answer::Accessibility(pages.clone())) else {
            panic!("a structure tree comes back as one");
        };
        assert_eq!(read.len(), pages.len());
        for (read, ours) in read.iter().zip(&pages) {
            assert_eq!(read.page, ours.page);
            assert_eq!(read.nodes, ours.nodes);
        }

        // §12.7, and one of every control §12.7.5 defines: a host on this boundary builds the
        // same form as one off it or it builds a different program.
        let fields = a_populated_form();
        let Reply::Fields(read) = round_trip(&Answer::Fields(fields.clone())) else {
            panic!("form fields come back as form fields");
        };
        assert_eq!(read, fields);

        // The eight per-code counts, each a different number: an encoder that wrote them in the
        // wrong order would pass with any two of them equal, and a host reading "3 codes no
        // /ToUnicode named" for "3 codes an Identity ordering left unaddressable" would be told
        // the wrong thing about whose gap it is.
        let shortfall = pdf_model::content::Shortfall {
            unnamed: pdf_model::content::UnnamedCodes {
                empty_mapping: 1,
                incomplete_to_unicode: 2,
                unlisted_name: 3,
                unnamed_cid: 4,
                unaddressable_cid: 5,
                unnamed_glyph: 6,
            },
            without_a_glyph: 7,
            reaching_a_blank_glyph: 8,
        };
        let counted = vec![viewer_core::PageReadback { page: 9, shortfall }];
        let Reply::Readback(read) = round_trip(&Answer::Readback(counted)) else {
            panic!("a readback shortfall comes back as one");
        };
        let [read] = read.as_slice() else {
            panic!("one page's counts crossed");
        };
        assert_eq!(read.page, 9);
        assert_eq!(read.shortfall, shortfall);
        assert_eq!(read.shortfall.unnamed.total(), 21);

        // And the sentences, per page, with the pages named: a status bar under a column that
        // took the first entry for all of them would say one page's refusals about four.
        let first: Vec<String> = vec!["a shading this reader could not draw".to_owned()];
        let second: Vec<String> = vec!["one font".to_owned(), "one image".to_owned()];
        let said = vec![
            viewer_core::PageReports {
                page: 2,
                notes: &first,
            },
            viewer_core::PageReports {
                page: 5,
                notes: &second,
            },
        ];
        let Reply::Reports(read) = round_trip(&Answer::Reports(said)) else {
            panic!("reports come back as reports");
        };
        assert_eq!(
            read,
            vec![
                crate::Reported {
                    page: 2,
                    notes: first,
                },
                crate::Reported {
                    page: 5,
                    notes: second,
                },
            ]
        );
    }

    /// One field per control §12.7.5 defines, with no default among them.
    ///
    /// Seven, because §12.7.5.2 splits its own type three ways and Table 226 lets a file state no
    /// `/FT` at all. Every flag is the opposite of the standard's default, every `Option` is
    /// `Some` and every list has more than one element — see the note on
    /// [`a_populated_outline`]: a round trip over defaults would pass with an encoder that wrote
    /// nothing.
    fn a_populated_form() -> Vec<viewer_core::FormField> {
        use pdf_model::form::{Choice, ChoiceControl, Control, TextControl};
        use pdf_model::view::FieldName;
        use viewer_core::{FormField, FormWidget};

        let widget = |serial: u16, on_state: Option<&str>, export: Option<&str>| FormWidget {
            annotation: ObjectId::new(u32::from(serial), 0),
            quad: [
                f32::from(serial),
                1.0,
                2.0,
                3.0,
                4.0,
                5.0,
                6.0,
                f32::from(serial),
            ],
            on_state: on_state.map(str::to_owned),
            export: export.map(str::to_owned),
            on: on_state.is_some(),
        };
        let field = |name: &str, control: Control, widgets: Vec<FormWidget>| FormField {
            name: FieldName {
                qualified: format!("outer.{name}"),
                alternative: Some(format!("the {name}")),
            },
            partial: name.to_owned(),
            control,
            value: Some(pdf_model::view::ShownValue {
                text: format!("{name}'s value"),
                obscured: false,
            }),
            read_only: true,
            required: true,
            no_export: true,
            widgets,
        };

        vec![
            field(
                "text",
                Control::Text(TextControl {
                    multiline: true,
                    password: true,
                    file_select: true,
                    do_not_spell_check: true,
                    do_not_scroll: true,
                    comb: Some(9),
                    max_len: Some(12),
                    rich_text: true,
                }),
                vec![widget(40, None, None), widget(41, None, None)],
            ),
            field(
                "choice",
                Control::Choice(ChoiceControl {
                    combo: true,
                    editable: true,
                    multi_select: true,
                    do_not_spell_check: true,
                    commit_on_selection: true,
                    options: vec![
                        Choice {
                            export: Some("r".to_owned()),
                            label: "Red".to_owned(),
                        },
                        Choice {
                            export: None,
                            label: "Blue".to_owned(),
                        },
                    ],
                    selected: vec![0, 1],
                    top: 1,
                }),
                vec![widget(42, None, None)],
            ),
            field(
                "box",
                Control::CheckBox { on: true },
                vec![widget(43, Some("Yes"), Some("yes"))],
            ),
            field(
                "radio",
                Control::RadioButton {
                    on: true,
                    no_toggle_to_off: true,
                    in_unison: true,
                },
                vec![
                    widget(44, Some("visa"), Some("Visa")),
                    widget(45, None, None),
                ],
            ),
            field("push", Control::PushButton, vec![widget(46, None, None)]),
            field("sign", Control::Signature, vec![widget(47, None, None)]),
            field("odd", Control::Unstated, vec![widget(48, None, None)]),
        ]
    }

    /// Encodes an answer and reads it back, failing loudly rather than returning a `Result`.
    fn round_trip(answer: &Answer<'_>) -> Reply {
        let encoded = encode_answer(answer).expect("this answer crosses");
        decode_answer(&encoded).expect("what was written reads back")
    }

    /// A §7.11.6 value Table 47 does not describe is refused, and the refusal names the answer.
    #[test]
    fn a_collection_value_outside_table_47_is_refused_by_name() {
        use pdf_model::collection::{Collection, Folder, Item, Value};

        let mut values = std::collections::BTreeMap::new();
        values.insert(
            "odd".to_owned(),
            Value {
                // An array is not the text string, date string or number Table 47 describes.
                data: pdf_syntax::Object::Array(vec![pdf_syntax::Object::Integer(1)]),
                prefix: None,
            },
        );
        let collection = Collection {
            folders: Some(Folder {
                id: 0,
                name: "root".to_owned(),
                description: None,
                item: Item { values },
                has_thumbnail: false,
                children: Vec::new(),
            }),
            ..Collection::default()
        };
        let refused = encode_answer(&Answer::Collection {
            collection,
            initial: pdf_model::collection::Initial::Container,
        })
        .unwrap_err();
        assert_eq!(refused.message, "Answer::Collection");
    }

    /// A tree that nests past what this reader follows is a refusal, not a deep recursion.
    ///
    /// Written by hand rather than by encoding a deep outline, because `pdf_model::outline` will
    /// not build one — which is the point: these bytes cannot have come from that reader, and the
    /// bound is on the *message*.
    #[test]
    fn a_tree_nested_past_the_bound_is_refused_rather_than_followed() {
        let mut bytes = vec![answer_kind::LAYERS];
        // One layer collection inside another, over and over: a count of one, the collection
        // discriminant, and no label.
        bytes.extend_from_slice(&1u64.to_be_bytes());
        for _ in 0..(panels::MAX_TREE_DEPTH + 2) {
            bytes.push(1);
            bytes.push(0);
            bytes.extend_from_slice(&1u64.to_be_bytes());
        }
        let error = decode_answer(&bytes).unwrap_err();
        assert!(
            matches!(error, ProtocolError::TooDeep { .. }),
            "a tree past the bound is refused: {error}"
        );
    }

    /// A structure tree whose parent links do not point backwards is refused.
    ///
    /// A host walking the answer follows `parent` upwards; a node naming itself, or naming one it
    /// has not read yet, is a loop rather than a tree — and the confined side produces the answer
    /// parent-first, so a single comparison is the whole check.
    #[test]
    fn a_structure_node_whose_parent_is_not_behind_it_is_refused() {
        let mut bytes = vec![answer_kind::ACCESSIBILITY];
        // One page on the screen, which is page one: the answer has been a list of pages since
        // Table 29's arrangements were obeyed, and the check below is inside one of its entries.
        bytes.extend_from_slice(&1u64.to_be_bytes());
        bytes.extend_from_slice(&0u64.to_be_bytes());
        bytes.extend_from_slice(&1u64.to_be_bytes());
        bytes.push(1);
        bytes.extend_from_slice(&0u64.to_be_bytes());
        let error = decode_answer(&bytes).unwrap_err();
        assert!(
            matches!(error, ProtocolError::Unrecognised { .. }),
            "a node that is its own parent is refused: {error}"
        );
    }

    /// What each of the eleven costs to cross, on a document that has one of each.
    ///
    /// A measurement rather than a threshold, and it prints rather than asserts for a reason: the
    /// numbers are a property of the *document*, so a bound written here would be a bound on
    /// whichever file this test happens to open. What it does assert is the thing a number cannot
    /// say — that each of them round trips on real content.
    ///
    /// Run it with `cargo test -p viewer-confined --lib -- --nocapture panels`.
    #[test]
    fn what_each_panel_costs_to_cross() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for name in [
            "doc/PDF20_AN002-AF.pdf",
            "doc/PDF-Declarations.pdf",
            "doc/ISO_32000-2_sponsored_EC3.pdf",
        ] {
            let Ok(bytes) = std::fs::read(root.join(name)) else {
                panic!("{name} is committed");
            };
            let mut viewer = viewer_core::Viewer::new(900, 1200, 1.0);
            for _ in viewer.handle(Command::Open {
                id: DocumentId(1),
                bytes,
                password: None,
                fragment: None,
            }) {}

            println!("{name}");
            for (label, query) in [
                ("outline", Query::Outline),
                ("layers", Query::Layers),
                ("attachments", Query::Attachments),
                ("collection", Query::Collection),
                ("articles", Query::Articles),
                ("thumbnail", Query::Thumbnail(0)),
                ("properties", Query::Properties),
                ("opening", Query::Opening),
                ("preferences", Query::Preferences),
                ("popups", Query::Popups),
                ("fields", Query::Fields),
                ("structure", Query::AccessibilityTree),
            ] {
                let answer = viewer.query(query);
                let at = std::time::Instant::now();
                let encoded = encode_answer(&answer).expect("this answer crosses");
                let encoded_in = at.elapsed();
                let at = std::time::Instant::now();
                let read = decode_answer(&encoded).expect("what was written reads back");
                println!(
                    "  {label:12} {:>9} bytes  encode {:>7.3} ms  decode {:>7.3} ms  {}",
                    encoded.len(),
                    encoded_in.as_secs_f64() * 1e3,
                    at.elapsed().as_secs_f64() * 1e3,
                    if matches!(read, Reply::None) {
                        "(nothing to answer with)"
                    } else {
                        ""
                    }
                );
            }
        }
    }

    #[test]
    fn a_truncated_message_is_a_refusal_that_says_where() {
        let encoded = encode_command(&Command::Resize {
            width: 800,
            height: 1000,
            scale: 1.0,
        })
        .unwrap();
        let short = &encoded[..encoded.len() - 1];
        let error = decode_command(short).unwrap_err();
        assert!(
            matches!(error, ProtocolError::Truncated { what } if what == "a display scale"),
            "{error}"
        );
    }

    #[test]
    fn a_discriminant_this_build_does_not_define_is_refused() {
        let error = decode_command(&[200]).unwrap_err();
        assert!(
            matches!(
                error,
                ProtocolError::Unrecognised {
                    what: "a command",
                    value: 200
                }
            ),
            "{error}"
        );
    }

    /// A length is a claim, and a claim from the other side of the pipe is checked.
    ///
    /// The shape of a message from a worker that has been subverted: it says the string is four
    /// gigabytes long and sends none of it. Allocating from that number is the failure.
    #[test]
    fn a_length_larger_than_the_message_is_refused_before_anything_is_allocated() {
        let mut bytes = vec![command_kind::EXTRACT];
        bytes.extend_from_slice(&(1u64 << 32).to_be_bytes());
        let error = decode_command(&bytes).unwrap_err();
        assert!(matches!(error, ProtocolError::Overlong { .. }), "{error}");
    }

    #[test]
    fn bytes_left_over_are_a_refusal_rather_than_a_silence() {
        let mut encoded = encode_command(&Command::Save).unwrap();
        encoded.push(0);
        let error = decode_command(&encoded).unwrap_err();
        assert!(
            matches!(error, ProtocolError::Trailing { left: 1, .. }),
            "{error}"
        );
    }

    /// A frame's length field is three bytes shorter than the message it heads on purpose.
    /// Every truncation and every single-byte change of a valid message, decoded.
    ///
    /// **The confined side is the untrusted side of this boundary**, so its messages are a parser
    /// over hostile input in the same sense a content stream is — and this project's rule is that
    /// a parser is fuzzed. This is the deterministic half of that: every prefix of a valid
    /// message and every one-byte change to it, which is where a length that is a claim, a
    /// discriminant this build does not define and a `usize` that does not fit all live. The
    /// requirement is not that any particular one is refused; it is that **none of them panics**,
    /// because a panic here is the confined process reaching into the host.
    #[test]
    fn no_truncation_or_flip_of_a_message_makes_a_decoder_panic() {
        let messages = [
            encode_command(&Command::Open {
                id: DocumentId(3),
                bytes: b"%PDF-2.0\n1 0 obj".to_vec(),
                password: Some("p".to_owned().into()),
                fragment: None,
            })
            .unwrap(),
            encode_command(&Command::Edit(Edit::Markup {
                kind: Markup::Highlight,
                colour: [1.0, 1.0, 0.0],
            }))
            .unwrap(),
            encode_events(&[
                Event::Opened {
                    document: DocumentId(3),
                    pages: 5,
                },
                Event::Reported {
                    document: DocumentId(3),
                    page: Some(0),
                    notes: vec!["a note".to_owned()],
                },
            ])
            .unwrap(),
            encode_answer(&Answer::Selected(viewer_core::Selected {
                text: std::borrow::Cow::Borrowed("selected"),
                quads: vec![[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]],
            }))
            .unwrap(),
            encode_query(Query::Find("needle")).unwrap(),
        ];

        for message in &messages {
            for cut in 0..=message.len() {
                let Some(prefix) = message.get(..cut) else {
                    continue;
                };
                let _ = decode_command(prefix);
                let _ = decode_events(prefix);
                let _ = decode_answer(prefix);
                let _ = decode_query(prefix);
            }
            for at in 0..message.len() {
                for value in [0x00u8, 0x01, 0x7F, 0x80, 0xFF] {
                    let mut changed = message.clone();
                    if let Some(byte) = changed.get_mut(at) {
                        *byte = value;
                    }
                    let _ = decode_command(&changed);
                    let _ = decode_events(&changed);
                    let _ = decode_answer(&changed);
                    let _ = decode_query(&changed);
                }
            }
        }
    }

    #[test]
    fn a_frame_header_names_its_kind_and_length() {
        let framed = frame(FRAME_COMMAND, b"payload");
        let header: [u8; FRAME_HEADER_LEN] = framed[..FRAME_HEADER_LEN].try_into().unwrap();
        assert_eq!(parse_frame_header(header), Some((FRAME_COMMAND, 7)));
        assert_eq!(parse_frame_header([200, 0, 0, 0, 0, 0, 0, 0, 0]), None);
        // A length past `MAX_MESSAGE` is refused at the header, before a buffer is sized from it.
        let mut absurd = [0u8; FRAME_HEADER_LEN];
        absurd[0] = FRAME_EVENTS;
        absurd[1..].copy_from_slice(&(MAX_MESSAGE + 1).to_be_bytes());
        assert_eq!(parse_frame_header(absurd), None);
    }
}
