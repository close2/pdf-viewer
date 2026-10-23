//! The two ends of a message: [`Writer`] appends fixed-width fields, [`Reader`] takes them back and refuses.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use pdf_syntax::ObjectId;
use viewer_core::DocumentId;

use super::RESERVE;
use super::error::ProtocolError;
use super::frame::as_u64;

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

    /// How many bytes have been written so far.
    ///
    /// Exists for [`display_list`](super::display_list)'s budget: an encoder that is going to be compared against a
    /// number can stop at it, and stopping needs the running total. See
    /// [`display_list::Uncodable::TooLarge`](super::display_list::Uncodable::TooLarge).
    pub(super) fn len(&self) -> usize {
        self.out.len()
    }

    /// Puts another message's bytes on the end of this one.
    ///
    /// Exists for [`display_list`](super::display_list), whose *tables* have to precede the command stream that
    /// indexes into them while the interning that fills those tables happens as the commands
    /// are written. So the commands go into a second writer and arrive here, once.
    pub(super) fn append(&mut self, other: &Self) -> &mut Self {
        self.out.extend_from_slice(&other.out);
        self
    }

    pub(super) fn u8(&mut self, value: u8) -> &mut Self {
        self.out.push(value);
        self
    }

    pub(super) fn u32(&mut self, value: u32) -> &mut Self {
        self.out.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// A machine-word count, as a fixed 64 bits.
    ///
    /// Fixed width and not `usize`: the two sides of this pipe are the same build today and a
    /// format whose field widths depend on the pointer size is one that stops being true the day
    /// they are not.
    pub(super) fn usize(&mut self, value: usize) -> &mut Self {
        self.out.extend_from_slice(&as_u64(value).to_be_bytes());
        self
    }

    pub(super) fn i64(&mut self, value: i64) -> &mut Self {
        self.out.extend_from_slice(&value.to_be_bytes());
        self
    }

    pub(super) fn u64(&mut self, value: u64) -> &mut Self {
        self.out.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// An `f64` as its bits, for [`Writer::f32`]'s reason.
    pub(super) fn f64(&mut self, value: f64) -> &mut Self {
        self.out.extend_from_slice(&value.to_bits().to_be_bytes());
        self
    }

    /// An `f32` as its bits.
    ///
    /// Bits rather than a decimal spelling, because a coordinate that made a round trip through
    /// text would be a different number on the other side and this boundary carries geometry a
    /// host draws with.
    pub(super) fn f32(&mut self, value: f32) -> &mut Self {
        self.out.extend_from_slice(&value.to_bits().to_be_bytes());
        self
    }

    pub(super) fn bool(&mut self, value: bool) -> &mut Self {
        self.u8(u8::from(value))
    }

    pub(super) fn bytes(&mut self, value: &[u8]) -> &mut Self {
        self.usize(value.len());
        self.out.extend_from_slice(value);
        self
    }

    pub(super) fn str(&mut self, value: &str) -> &mut Self {
        self.bytes(value.as_bytes())
    }

    pub(super) fn option_str(&mut self, value: Option<&str>) -> &mut Self {
        match value {
            Some(text) => self.u8(1).str(text),
            None => self.u8(0),
        }
    }

    pub(super) fn option_bytes(&mut self, value: Option<&[u8]>) -> &mut Self {
        match value {
            Some(bytes) => self.u8(1).bytes(bytes),
            None => self.u8(0),
        }
    }

    pub(super) fn option_f32(&mut self, value: Option<f32>) -> &mut Self {
        match value {
            Some(number) => self.u8(1).f32(number),
            None => self.u8(0),
        }
    }

    pub(super) fn option_i64(&mut self, value: Option<i64>) -> &mut Self {
        match value {
            Some(number) => self.u8(1).i64(number),
            None => self.u8(0),
        }
    }

    pub(super) fn option_usize(&mut self, value: Option<usize>) -> &mut Self {
        match value {
            Some(number) => self.u8(1).usize(number),
            None => self.u8(0),
        }
    }

    pub(super) fn option_bool(&mut self, value: Option<bool>) -> &mut Self {
        match value {
            Some(flag) => self.u8(1).bool(flag),
            None => self.u8(0),
        }
    }

    pub(super) fn option_object(&mut self, value: Option<ObjectId>) -> &mut Self {
        match value {
            Some(object) => {
                self.u8(1);
                self.object(object)
            }
            None => self.u8(0),
        }
    }

    /// A fixed-length run of coordinates: a colour's three, a rectangle's four.
    pub(super) fn numbers(&mut self, values: &[f32]) -> &mut Self {
        for value in values {
            self.f32(*value);
        }
        self
    }

    pub(super) fn option_numbers(&mut self, values: Option<&[f32]>) -> &mut Self {
        match values {
            Some(values) => {
                self.u8(1);
                self.numbers(values)
            }
            None => self.u8(0),
        }
    }

    pub(super) fn strings(&mut self, values: &[String]) -> &mut Self {
        self.usize(values.len());
        for value in values {
            self.str(value);
        }
        self
    }

    pub(super) fn point(&mut self, (x, y): (f32, f32)) -> &mut Self {
        self.f32(x).f32(y)
    }

    pub(super) fn quad(&mut self, quad: [f32; 8]) -> &mut Self {
        for value in quad {
            self.f32(value);
        }
        self
    }

    pub(super) fn document(&mut self, document: DocumentId) -> &mut Self {
        self.u64(document.0)
    }

    pub(super) fn object(&mut self, object: ObjectId) -> &mut Self {
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

    /// How many bytes are still unread.
    ///
    /// The denominator of every "is this count a length or a claim?" question, which is why it
    /// is exposed rather than each check reaching for the field: [`display_list`](super::display_list)'s tables
    /// multiply it by the smallest record they can hold, which is a tighter bound than
    /// [`Self::list`]'s one-byte-per-element assumption.
    pub(super) fn remaining(&self) -> usize {
        self.rest.len()
    }

    /// Refuses unless everything was consumed.
    pub(super) fn end(&self, what: &'static str) -> Result<(), ProtocolError> {
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

    pub(super) fn u8(&mut self, what: &'static str) -> Result<u8, ProtocolError> {
        self.take(1, what)?
            .first()
            .copied()
            .ok_or(ProtocolError::Truncated { what })
    }

    pub(super) fn u32(&mut self, what: &'static str) -> Result<u32, ProtocolError> {
        let bytes: [u8; 4] = self
            .take(4, what)?
            .try_into()
            .map_err(|_| ProtocolError::Truncated { what })?;
        Ok(u32::from_be_bytes(bytes))
    }

    pub(super) fn u64(&mut self, what: &'static str) -> Result<u64, ProtocolError> {
        let bytes: [u8; 8] = self
            .take(8, what)?
            .try_into()
            .map_err(|_| ProtocolError::Truncated { what })?;
        Ok(u64::from_be_bytes(bytes))
    }

    pub(super) fn i64(&mut self, what: &'static str) -> Result<i64, ProtocolError> {
        let bytes: [u8; 8] = self
            .take(8, what)?
            .try_into()
            .map_err(|_| ProtocolError::Truncated { what })?;
        Ok(i64::from_be_bytes(bytes))
    }

    /// A count, refused where this machine cannot hold it.
    pub(super) fn usize(&mut self, what: &'static str) -> Result<usize, ProtocolError> {
        let value = self.u64(what)?;
        usize::try_from(value).map_err(|_| ProtocolError::Overlong {
            what,
            claimed: usize::MAX,
            available: self.rest.len(),
        })
    }

    pub(super) fn f32(&mut self, what: &'static str) -> Result<f32, ProtocolError> {
        Ok(f32::from_bits(self.u32(what)?))
    }

    pub(super) fn f64(&mut self, what: &'static str) -> Result<f64, ProtocolError> {
        Ok(f64::from_bits(self.u64(what)?))
    }

    pub(super) fn bool(&mut self, what: &'static str) -> Result<bool, ProtocolError> {
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
    pub(super) fn bytes(&mut self, what: &'static str) -> Result<&'a [u8], ProtocolError> {
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

    /// A length-prefixed byte string, copied out, refusing rather than aborting where there is
    /// no room for the copy.
    ///
    /// **The one shape of allocation on this boundary whose size the *other side* chose**, and
    /// therefore the one that is a `try_reserve`. Everything else here is sized by the work — a
    /// display list, a page's pixels — and is bounded where that work is bounded. A document's
    /// bytes, a saved file, an attachment and a raster all arrive through this.
    pub(super) fn owned_bytes(&mut self, what: &'static str) -> Result<Vec<u8>, ProtocolError> {
        let bytes = self.bytes(what)?;
        let mut out = Vec::new();
        out.try_reserve_exact(bytes.len())
            .map_err(|_| ProtocolError::NoRoom {
                what,
                bytes: bytes.len(),
            })?;
        out.extend_from_slice(bytes);
        Ok(out)
    }

    pub(super) fn string(&mut self, what: &'static str) -> Result<String, ProtocolError> {
        let bytes = self.owned_bytes(what)?;
        String::from_utf8(bytes).map_err(|_| ProtocolError::NotText { what })
    }

    pub(super) fn option_string(
        &mut self,
        what: &'static str,
    ) -> Result<Option<String>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.string(what)?))
        } else {
            Ok(None)
        }
    }

    pub(super) fn option_bytes(
        &mut self,
        what: &'static str,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.owned_bytes(what)?))
        } else {
            Ok(None)
        }
    }

    pub(super) fn option_f32(&mut self, what: &'static str) -> Result<Option<f32>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.f32(what)?))
        } else {
            Ok(None)
        }
    }

    pub(super) fn option_i64(&mut self, what: &'static str) -> Result<Option<i64>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.i64(what)?))
        } else {
            Ok(None)
        }
    }

    pub(super) fn option_usize(
        &mut self,
        what: &'static str,
    ) -> Result<Option<usize>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.usize(what)?))
        } else {
            Ok(None)
        }
    }

    pub(super) fn option_bool(
        &mut self,
        what: &'static str,
    ) -> Result<Option<bool>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.bool(what)?))
        } else {
            Ok(None)
        }
    }

    pub(super) fn option_object(
        &mut self,
        what: &'static str,
    ) -> Result<Option<ObjectId>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.object(what)?))
        } else {
            Ok(None)
        }
    }

    /// A colour's three components.
    pub(super) fn colour(&mut self, what: &'static str) -> Result<[f32; 3], ProtocolError> {
        Ok([self.f32(what)?, self.f32(what)?, self.f32(what)?])
    }

    pub(super) fn option_colour(
        &mut self,
        what: &'static str,
    ) -> Result<Option<[f32; 3]>, ProtocolError> {
        if self.bool(what)? {
            Ok(Some(self.colour(what)?))
        } else {
            Ok(None)
        }
    }

    /// A rectangle's four numbers, in the order the file states them.
    pub(super) fn rect(&mut self, what: &'static str) -> Result<[f32; 4], ProtocolError> {
        Ok([
            self.f32(what)?,
            self.f32(what)?,
            self.f32(what)?,
            self.f32(what)?,
        ])
    }

    pub(super) fn option_rect(
        &mut self,
        what: &'static str,
    ) -> Result<Option<[f32; 4]>, ProtocolError> {
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
    ///
    /// [`MAX_MESSAGE`]: confined_transport::frame::MAX_MESSAGE
    pub(super) fn list<T>(
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

    pub(super) fn strings(&mut self, what: &'static str) -> Result<Vec<String>, ProtocolError> {
        self.list(what, |reader| reader.string(what))
    }

    pub(super) fn point(&mut self, what: &'static str) -> Result<(f32, f32), ProtocolError> {
        Ok((self.f32(what)?, self.f32(what)?))
    }

    pub(super) fn quad(&mut self, what: &'static str) -> Result<[f32; 8], ProtocolError> {
        let mut quad = [0.0; 8];
        for value in &mut quad {
            *value = self.f32(what)?;
        }
        Ok(quad)
    }

    pub(super) fn document(&mut self, what: &'static str) -> Result<DocumentId, ProtocolError> {
        Ok(DocumentId(self.u64(what)?))
    }

    pub(super) fn object(&mut self, what: &'static str) -> Result<ObjectId, ProtocolError> {
        let number = self.u32(what)?;
        let generation =
            u16::try_from(self.u32(what)?).map_err(|_| ProtocolError::Unrecognised {
                what: "a generation number",
                value: u32::MAX,
            })?;
        Ok(ObjectId::new(number, generation))
    }
}

/// Reads a length-prefixed run of quadrilaterals.
pub(super) fn read_quads(
    reader: &mut Reader<'_>,
    what: &'static str,
) -> Result<Vec<[f32; 8]>, ProtocolError> {
    reader.list(what, |reader| reader.quad(what))
}
