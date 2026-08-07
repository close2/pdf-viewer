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
//! into a catch-all arm. The two messages this transport deliberately does not carry —
//! [`viewer_core::Command::RenderReady`] and [`viewer_core::Event::NeedsRender`] — are named as
//! [`Uncarried`], which is a refusal a caller can read, and they are not carried because the
//! confined process answers them *itself*: it holds the rasteriser, so the render round trip
//! never crosses the pipe. That is `doc/HANDOVER.md`'s section 0, "one boundary, not two".
//!
//! **A message that cannot be decoded is a refusal that says so.** [`ProtocolError`] names what
//! was truncated or unrecognised; nothing here defaults, clamps or guesses.

use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
use pdf_model::restriction::Operation;
use pdf_model::view::Markup;
use pdf_render::{Point, Raster, RasterFormat, Rect, Size};
use pdf_sandbox::lockdown::{Confinement, LandlockLevel, SystemCalls};
use pdf_syntax::{Name, ObjectId};
use viewer_core::{
    Answer, Command, DocumentId, Edit, Event, FocusMove, PageGeometry, PageTarget, PointerAction,
    Purpose, Query, RestrictionLevel, Selection, Zoom,
};

use crate::Reply;

/// Greeting bytes, changed whenever this format changes incompatibly.
///
/// A host and a worker from different builds must not talk to each other, and the cheapest
/// place to find that out is the first thing either says.
const MAGIC: &[u8; 8] = b"PDFVCF01";

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

    fn strings(&mut self, what: &'static str) -> Result<Vec<String>, ProtocolError> {
        let count = self.usize(what)?;
        // One string is at least its own eight-byte length, so a count larger than what is left
        // cannot be honest — and reserving from it would be allocating from a claim.
        if count > self.rest.len() {
            return Err(ProtocolError::Overlong {
                what,
                claimed: count,
                available: self.rest.len(),
            });
        }
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            out.push(self.string(what)?);
        }
        Ok(out)
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

/// Prefixes a payload with its kind and length.
pub(crate) fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len().saturating_add(FRAME_HEADER_LEN));
    out.push(kind);
    out.extend_from_slice(&as_u64(payload.len()).to_be_bytes());
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
                .option_str(password.as_deref())
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
            password: reader.option_string("a password")?,
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
        k::EDIT => Command::Edit(decode_edit(&mut reader)?),
        k::UNDO => Command::Undo,
        k::REDO => Command::Redo,
        k::EXTRACT => Command::Extract {
            name: reader.string("an attachment's name")?,
        },
        k::SAVE => Command::Save,
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
        Edit::SetField { field, value } => {
            writer.u8(0).str(field).option_str(value.as_deref());
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
    }
}

fn decode_edit(reader: &mut Reader<'_>) -> Result<Edit, ProtocolError> {
    let what = "an edit";
    Ok(match reader.u8(what)? {
        0 => Edit::SetField {
            field: reader.string("a field name")?,
            value: reader.option_string("a field value")?,
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
        Event::Saved { document, bytes } => {
            writer.u8(k::SAVED).document(*document).bytes(bytes);
        }
        Event::Extracted {
            document,
            name,
            bytes,
        } => {
            writer
                .u8(k::EXTRACTED)
                .document(*document)
                .str(name)
                .bytes(bytes);
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
            name: reader.string("an attachment's name")?,
            bytes: reader.bytes("an attachment")?.to_vec(),
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
    let count = reader.usize(what)?;
    // An event is at least its own nine bytes, so a count past what is left is a claim rather
    // than a length — and reserving from a claim is what this refuses.
    if count > reader.rest.len() {
        return Err(ProtocolError::Overlong {
            what,
            claimed: count,
            available: reader.rest.len(),
        });
    }
    let mut events = Vec::with_capacity(count);
    for _ in 0..count {
        events.push(decode_event(reader.bytes(what)?)?);
    }
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
}

/// Why a panel-shaped question does not cross yet.
///
/// One sentence, shared by the eleven of them, because they are one fact rather than eleven:
/// each answers with a `pdf-model` type — an outline's tree, Table 147 whole, a decoded
/// thumbnail, §14.7's structure — and encoding those is the second half of this boundary rather
/// than a hole in the first. A host asking for one is told, which is the difference between a
/// boundary that is incomplete and one that is quietly wrong.
const PANEL_ANSWER: &str = "it answers with a document-model type this transport does not encode yet; \
     doc/todo/34 is what is left";

/// Encodes one question.
///
/// # Errors
///
/// [`Uncarried`] for the eleven questions whose answers are document-model types.
pub(crate) fn encode_query(query: Query<'_>) -> Result<Vec<u8>, Uncarried> {
    use query_kind as k;

    let uncarried = |message| {
        Err(Uncarried {
            message,
            reason: PANEL_ANSWER,
        })
    };

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
        Query::Frame => {
            writer.u8(k::FRAME);
        }
        Query::Reports => {
            writer.u8(k::REPORTS);
        }
        Query::Outline => return uncarried("Query::Outline"),
        Query::Layers => return uncarried("Query::Layers"),
        Query::Attachments => return uncarried("Query::Attachments"),
        Query::Collection => return uncarried("Query::Collection"),
        Query::Articles => return uncarried("Query::Articles"),
        Query::Thumbnail(_) => return uncarried("Query::Thumbnail"),
        Query::Properties => return uncarried("Query::Properties"),
        Query::Opening => return uncarried("Query::Opening"),
        Query::Preferences => return uncarried("Query::Preferences"),
        Query::Popups => return uncarried("Query::Popups"),
        Query::AccessibilityTree => return uncarried("Query::AccessibilityTree"),
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
    Caret { at: (f32, f32), offset: usize },
    Dirty,
    Selection,
    LogicalSelection,
    Focus,
    Frame,
    Reports,
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
                PlainQuery::Dirty => Query::Dirty,
                PlainQuery::Selection => Query::Selection,
                PlainQuery::LogicalSelection => Query::LogicalSelection,
                PlainQuery::Focus => Query::Focus,
                PlainQuery::Frame => Query::Frame,
                PlainQuery::Reports => Query::Reports,
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
        k::DIRTY => OwnedQuery::Plain(PlainQuery::Dirty),
        k::FIND => OwnedQuery::Find(reader.string("a search string")?),
        k::SELECTION => OwnedQuery::Plain(PlainQuery::Selection),
        k::LOGICAL_SELECTION => OwnedQuery::Plain(PlainQuery::LogicalSelection),
        k::FOCUS => OwnedQuery::Plain(PlainQuery::Focus),
        k::FRAME => OwnedQuery::Plain(PlainQuery::Frame),
        k::REPORTS => OwnedQuery::Plain(PlainQuery::Reports),
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
}

/// Encodes one answer.
///
/// # Errors
///
/// [`Uncarried`] for the answers whose questions [`encode_query`] refuses, which is the same
/// list said from the other end: a host cannot ask for them, and a worker cannot send them.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per variant of a `viewer-core` enum, and the count is that enum's. Splitting it would put half the vocabulary in another function and lose the property the whole module rests on: the compiler naming the variant nobody handled"
)]
pub(crate) fn encode_answer(answer: &Answer<'_>) -> Result<Vec<u8>, Uncarried> {
    use answer_kind as k;

    let uncarried = |message| {
        Err(Uncarried {
            message,
            reason: PANEL_ANSWER,
        })
    };

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
            writer.u8(k::SELECTED).str(selected.text);
            writer.usize(selected.quads.len());
            for quad in &selected.quads {
                writer.quad(*quad);
            }
        }
        Answer::Field { name, value } => {
            writer
                .u8(k::FIELD)
                .str(&name.qualified)
                .option_str(name.alternative.as_deref())
                .option_str(value.as_deref());
        }
        Answer::Caret { from, to } => {
            writer.u8(k::CARET).point(*from).point(*to);
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
        Answer::Dirty(dirty) => {
            writer.u8(k::DIRTY).bool(*dirty);
        }
        Answer::Focus { object, quad } => {
            writer.u8(k::FOCUS).object(*object).quad(*quad);
        }
        Answer::LogicalSelection(text) => {
            writer.u8(k::LOGICAL_SELECTION).str(text);
        }
        Answer::Frame(frame) => {
            // The raster is why this boundary is worth having: the confined process draws the
            // page and the host is handed pixels, which is the whole of `doc/HANDOVER.md`'s
            // section 0 tier 1. `RasterFormat` is written out rather than assumed, because a second
            // format would otherwise be read as the first.
            let format = match frame.raster.format {
                RasterFormat::Rgba8 => 0,
                // `RasterFormat` is `#[non_exhaustive]`, so a second layout can arrive without
                // this crate being rebuilt against it. Refusing is the honest answer: a raster
                // whose bytes mean something this build cannot name must not cross as though it
                // were RGBA.
                _ => {
                    return Err(Uncarried {
                        message: "Answer::Frame",
                        reason: "the raster is in a pixel layout this build does not name",
                    });
                }
            };
            writer
                .u8(k::FRAME)
                .usize(frame.page)
                .u32(frame.raster.width)
                .u32(frame.raster.height)
                .u8(format)
                .bytes(&frame.raster.data)
                .point(frame.origin);
        }
        Answer::Reports(notes) => {
            writer.u8(k::REPORTS).usize(notes.len());
            for note in *notes {
                writer.str(note);
            }
        }
        Answer::Accessibility(_) => return uncarried("Answer::Accessibility"),
        Answer::Outline(_) => return uncarried("Answer::Outline"),
        Answer::Layers(_) => return uncarried("Answer::Layers"),
        Answer::Attachments(_) => return uncarried("Answer::Attachments"),
        Answer::Articles(_) => return uncarried("Answer::Articles"),
        Answer::Collection(_) => return uncarried("Answer::Collection"),
        Answer::Thumbnail(_) => return uncarried("Answer::Thumbnail"),
        Answer::Popups(_) => return uncarried("Answer::Popups"),
        Answer::Properties { .. } => return uncarried("Answer::Properties"),
        Answer::Opening(_) => return uncarried("Answer::Opening"),
        Answer::Preferences(_) => return uncarried("Answer::Preferences"),
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
            value: reader.option_string("a field's value")?,
        },
        k::CARET => Reply::Caret {
            from: reader.point("a caret")?,
            to: reader.point("a caret")?,
        },
        k::FOUND => {
            let what = "a search result";
            let count = reader.usize(what)?;
            if count > reader.rest.len() {
                return Err(ProtocolError::Overlong {
                    what,
                    claimed: count,
                    available: reader.rest.len(),
                });
            }
            let mut occurrences = Vec::with_capacity(count);
            for _ in 0..count {
                occurrences.push(read_quads(&mut reader, what)?);
            }
            Reply::Found(occurrences)
        }
        k::DIRTY => Reply::Dirty(reader.bool("a dirty flag")?),
        k::FOCUS => Reply::Focus {
            object: reader.object("an annotation")?,
            quad: reader.quad("a focus ring")?,
        },
        k::LOGICAL_SELECTION => Reply::LogicalSelection(reader.string("the selected text")?),
        k::FRAME => {
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
            // The worker is the untrusted side, so its dimensions are checked against the bytes
            // it actually sent rather than believed — the same rule `pdf_sandbox`'s parent
            // applies to a decoded image, and for the same reason.
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
            Reply::Frame {
                page,
                raster: Raster {
                    width,
                    height,
                    format,
                    data,
                },
                origin: reader.point("an origin")?,
            }
        }
        k::REPORTS => Reply::Reports(reader.strings("a report's notes")?),
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
    let count = reader.usize(what)?;
    // A quadrilateral is 32 bytes, so a count larger than what is left is not a short message,
    // it is a claim — and `with_capacity` from a claim is the allocation this refuses.
    if count > reader.rest.len() {
        return Err(ProtocolError::Overlong {
            what,
            claimed: count,
            available: reader.rest.len(),
        });
    }
    let mut quads = Vec::with_capacity(count);
    for _ in 0..count {
        quads.push(reader.quad(what)?);
    }
    Ok(quads)
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
    fn every_carried_command_round_trips() {
        let commands = vec![
            Command::Open {
                id: DocumentId(7),
                bytes: b"%PDF-2.0".to_vec(),
                password: Some("secret".to_owned()),
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
            Command::Edit(Edit::SetField {
                field: "A.NOM".to_owned(),
                value: Some("typed".to_owned()),
            }),
            Command::Edit(Edit::SetField {
                field: "A.NOM".to_owned(),
                value: None,
            }),
            Command::Edit(Edit::Markup {
                kind: Markup::Squiggly,
                colour: [1.0, 0.5, 0.0],
            }),
            Command::Undo,
            Command::Redo,
            Command::Extract {
                name: "attachment.txt".to_owned(),
            },
            Command::Save,
            Command::Select(Selection::All),
            Command::Select(Selection::None),
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

    /// Every event that crosses, encoded and read back.
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
                name: "readme.txt".to_owned(),
                bytes: b"hello".to_vec(),
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

    /// Every question that crosses, and every one that does not.
    ///
    /// Both halves in one test on purpose: the list of what is refused is as much a part of this
    /// boundary's description as the list of what is carried, and it is the thing `doc/todo/34`
    /// is measured against.
    #[test]
    fn every_query_is_either_carried_or_named() {
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
            Query::Dirty,
            Query::Find("needle"),
            Query::Selection,
            Query::LogicalSelection,
            Query::Focus,
            Query::Frame,
            Query::Reports,
        ];
        for query in carried {
            let encoded = encode_query(query).unwrap();
            let read = decode_query(&encoded).unwrap();
            assert_eq!(
                format!("{:?}", read.as_query()),
                format!("{query:?}"),
                "a query changed on the way through"
            );
        }

        let refused = [
            Query::Outline,
            Query::Layers,
            Query::Attachments,
            Query::Collection,
            Query::Articles,
            Query::Thumbnail(0),
            Query::Properties,
            Query::Opening,
            Query::Preferences,
            Query::Popups,
            Query::AccessibilityTree,
        ];
        for query in refused {
            let error = encode_query(query).unwrap_err();
            assert!(
                error.message.starts_with("Query::"),
                "a refusal names the message it refused: {error}"
            );
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
                password: Some("p".to_owned()),
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
                text: "selected",
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
