//! A stream's shared-memory ring: frames too large to copy through the pipe are written by the
//! worker into a slot of memory both processes map, and the item that announces one names its slot
//! (RFC 0015 section 4.3, ADR 3513).
//!
//! RFC 0012 section 5.1 measured the wire one reply per request, copying its bytes; a 1080p RGBA
//! frame is 8.3 MB, and through a pipe of 64 KiB that is a hundred and thirty wake-ups of each
//! process and a copy into the kernel and out again, every frame. The ring is that frame written
//! once, in place, by the worker, and read once by the host.
//!
//! # The object, and its seals
//!
//! The host creates a `memfd` of `slots × slot_len` bytes, sized from the stream's own frame and a
//! small slot count ([`Shape`]), and seals it before anything maps it:
//!
//! - **`F_SEAL_SHRINK`**, the one that protects the host. A mapping whose object is truncated under
//!   it faults on the next access with `SIGBUS`, and the host is the process that reads every slot;
//!   with the seal, no process holding the descriptor — the worker included, whatever a later
//!   profile admits — can take pages from under the host's mapping.
//! - **`F_SEAL_GROW`**, so that the size the host announced in the ring's frame is the object's size
//!   for its whole life, and the worker's mapping and the host's cover the same bytes.
//! - **`F_SEAL_SEAL`**, so that the set above is the set for good: no holder adds or tries another.
//!
//! `F_SEAL_WRITE` is not among them, because the worker must write; `MFD_CLOEXEC` keeps the host's
//! descriptor out of every other child it starts.
//!
//! # Passed once
//!
//! The descriptor crosses once, beside the [`RING`] frame's header by `SCM_RIGHTS`, before the
//! first stream that uses it, and the worker answers with an empty [`RING`] when it has mapped it.
//! **The worker keeps that descriptor for its life**: closing one is a system call
//! `Profile::Media` does not admit, and the descriptor it keeps fills the one slot the profile's
//! descriptor ceiling leaves, so the kernel refuses every later descriptor sent to it (ADR 3514).
//! A host therefore attaches one ring to a worker, and a stream whose frames would not fit it asks
//! for a new worker.
//!
//! # Who may touch a slot, and when — the ownership rule
//!
//! Item `q` of a stream is written into slot `q mod slots`, and nowhere else. Then:
//!
//! - **A slot is the worker's from the grant that covers the item it will carry until that item
//!   arrives**, and the host's at every other moment. The host clamps every grant to the slot
//!   count, so the items one grant covers name distinct slots.
//! - **The host reads a slot after its item has arrived and before it writes its next grant.** It
//!   grants again only when every item of the last grant has arrived (ADR 3501), and it writes that
//!   grant in the call that follows the one that copied the last slot out — so a slot is never
//!   read while the worker may write it, and never written while the host may read it.
//! - **The worker writes a slot only once it holds the credit for the item that slot will carry**:
//!   [`crate::stream::Emitter::emit_in_ring`] reads the host's grant first and hands the writing
//!   closure the slot second.
//!
//! An item that names a slot other than `q mod slots`, or a length past a slot, is the worker
//! claiming a slot it does not hold, and it is ended for it, as for any other broken contract.
//! **A worker that writes memory it does not hold without saying so cannot be seen** — a write is
//! not a message — and it costs the host nothing: the host never holds a reference into the
//! mapping, only copies bytes out of it (`inherited_descriptors::shared_mapping`), so the most a
//! racing write can do is change the pixels of the frame being copied, which the worker could have
//! sent anyway.
//!
//! # Other platforms
//!
//! `memfd` and its seals are Linux's. Elsewhere [`crate::Host::attach_ring`] answers
//! [`RingError::Unsupported`] and every item crosses inline, as the transport's platform code
//! already chooses for descriptors: Windows has no `SCM_RIGHTS` here (its channel is a pipe), and a
//! ring there would be its own shared section passed its own way, argued when that platform's
//! confinement is (`doc/todo/35`).

use std::num::NonZeroU32;

use crate::stream::Malformed;

/// The kind of the frame that attaches a ring: its [`Shape`], with the object's descriptor beside
/// the header. The worker answers with the same kind and no payload once it has mapped it.
///
/// Past [`crate::stream::END`] in the transport's own range, for the stream kinds' reason.
pub const RING: u8 = 0xF8;

/// Length of a ring frame's payload: the slot length and the slot count, four bytes each.
pub const SHAPE_LEN: usize = 8;

/// Most slots a ring may have.
///
/// Three is what a player double-buffering one frame on screen and decoding the next needs; sixteen
/// leaves room for a host that queues ahead without letting a ring become a second frame store.
pub const MAX_SLOTS: u32 = 16;

/// Why a ring could not be attached.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RingError {
    /// The shape asked for is not one a ring may have.
    #[error("{0}")]
    Shape(String),
    /// This worker already holds a ring, and has no descriptor slot for a second.
    #[error("the confined worker already holds a ring and can be handed no second descriptor")]
    Attached,
    /// This platform has no ring; items cross inline.
    #[error("this platform has no shared-memory ring; items cross inline")]
    Unsupported,
    /// The object could not be created, sealed or mapped.
    #[error("the ring's shared memory could not be made: {0}")]
    System(#[source] std::io::Error),
    /// The worker did not answer the ring with an empty [`RING`], and was ended.
    #[error("the confined worker answered the ring with a frame of kind {kind} and {length} bytes")]
    Refused {
        /// The answer's kind.
        kind: u8,
        /// The answer's length.
        length: usize,
    },
    /// The wire failed while the ring crossed.
    #[error(transparent)]
    Transport(#[from] crate::TransportError),
}

/// A ring's slots: how many, and how long each is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    slot_len: NonZeroU32,
    slots: NonZeroU32,
}

impl Shape {
    /// `slots` slots of `slot_len` bytes, within `budget` bytes for the whole ring.
    ///
    /// The budget is the host's word for the worker's memory a ring may take; RFC 0015 section 4's
    /// table puts a video worker's ceiling at a gibibyte, and [`crate::Host::attach_ring`] holds a
    /// ring to half of what the worker's greeting says its address space is.
    ///
    /// # Errors
    ///
    /// An empty slot, a slot past four gibibytes, no slots or more than [`MAX_SLOTS`], or a ring
    /// past `budget`.
    pub fn new(slot_len: usize, slots: u32, budget: u64) -> Result<Self, RingError> {
        let shape = Self::checked(slot_len, slots).map_err(|wrong| RingError::Shape(wrong.0))?;
        if shape.total_u64() > budget {
            return Err(RingError::Shape(format!(
                "a ring of {slots} slots of {slot_len} bytes is {} bytes, past the {budget} its \
                 budget allows",
                shape.total_u64()
            )));
        }
        Ok(shape)
    }

    fn checked(slot_len: usize, slots: u32) -> Result<Self, Malformed> {
        let slot_len = u32::try_from(slot_len)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or_else(|| Malformed(format!("a ring slot of {slot_len} bytes")))?;
        let slots = NonZeroU32::new(slots)
            .filter(|slots| slots.get() <= MAX_SLOTS)
            .ok_or_else(|| {
                Malformed(format!(
                    "a ring of {slots} slots, where one to {MAX_SLOTS} is a ring"
                ))
            })?;
        let shape = Self { slot_len, slots };
        usize::try_from(shape.total_u64()).map_err(|_| {
            Malformed(format!(
                "a ring of {} bytes, past this process's address space",
                shape.total_u64()
            ))
        })?;
        Ok(shape)
    }

    /// Bytes in a slot.
    #[must_use]
    pub fn slot_len(&self) -> usize {
        // A `u32` is a `usize` on every target this tree builds for, which `checked` also proved
        // of the whole ring.
        usize::try_from(self.slot_len.get()).unwrap_or(usize::MAX)
    }

    /// Slots in the ring.
    #[must_use]
    pub fn slots(&self) -> NonZeroU32 {
        self.slots
    }

    /// Bytes in the whole ring.
    #[must_use]
    pub fn total(&self) -> usize {
        usize::try_from(self.total_u64()).unwrap_or(usize::MAX)
    }

    fn total_u64(self) -> u64 {
        // Two `u32`s multiply inside a `u64` without overflow; saturating says so in the code.
        u64::from(self.slot_len.get()).saturating_mul(u64::from(self.slots.get()))
    }

    /// The slot item `sequence` is written into: the ownership rule's `q mod slots`.
    #[must_use]
    pub fn slot_of(&self, sequence: u32) -> u32 {
        sequence % self.slots
    }

    /// Where `slot` starts in the object, or `None` for a slot the ring does not have.
    #[must_use]
    pub fn offset(&self, slot: u32) -> Option<usize> {
        (slot < self.slots.get()).then(|| {
            usize::try_from(u64::from(slot).saturating_mul(u64::from(self.slot_len.get())))
                .unwrap_or(usize::MAX)
        })
    }

    /// A ring frame's payload.
    #[must_use]
    pub fn encode(&self) -> [u8; SHAPE_LEN] {
        let mut out = [0u8; SHAPE_LEN];
        out[..4].copy_from_slice(&self.slot_len.get().to_be_bytes());
        out[4..].copy_from_slice(&self.slots.get().to_be_bytes());
        out
    }

    /// Reads a ring frame's payload.
    ///
    /// # Errors
    ///
    /// A payload that is not [`SHAPE_LEN`] bytes, or a shape no ring may have.
    pub fn decode(payload: &[u8]) -> Result<Self, Malformed> {
        let (Some(slot_len), Some(slots), SHAPE_LEN) = (
            crate::stream::be_u32(payload.get(..4)),
            crate::stream::be_u32(payload.get(4..)),
            payload.len(),
        ) else {
            return Err(Malformed(format!(
                "a ring of {} bytes, which is {SHAPE_LEN}",
                payload.len()
            )));
        };
        let slot_len = usize::try_from(slot_len)
            .map_err(|_| Malformed("a ring slot past this process's address space".to_owned()))?;
        Self::checked(slot_len, slots)
    }
}

#[cfg(target_os = "linux")]
pub(crate) use linux::HostRing;
#[cfg(target_os = "linux")]
pub use linux::RingWriter;

#[cfg(not(target_os = "linux"))]
pub(crate) use elsewhere::HostRing;
#[cfg(not(target_os = "linux"))]
pub use elsewhere::RingWriter;

#[cfg(target_os = "linux")]
mod linux {
    use std::os::fd::{AsFd as _, BorrowedFd, IntoRawFd as _, OwnedFd};

    use inherited_descriptors::shared_mapping::{Reader, Writer};
    use rustix::fs::{MemfdFlags, SealFlags};

    use super::{RingError, Shape};
    use crate::link::ReceivedDescriptor;

    /// The host's ring: the sealed object, and its read-only mapping.
    #[derive(Debug)]
    pub(crate) struct HostRing {
        object: OwnedFd,
        view: Reader,
        shape: Shape,
    }

    impl HostRing {
        /// Creates, sizes, seals and maps the object; see the module for each seal.
        pub(crate) fn create(shape: Shape) -> Result<Self, RingError> {
            let system = |error: rustix::io::Errno| RingError::System(error.into());
            let object = rustix::fs::memfd_create(
                "confined-ring",
                MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
            )
            .map_err(system)?;
            let length = u64::try_from(shape.total()).unwrap_or(u64::MAX);
            rustix::fs::ftruncate(&object, length).map_err(system)?;
            rustix::fs::fcntl_add_seals(
                &object,
                SealFlags::SHRINK | SealFlags::GROW | SealFlags::SEAL,
            )
            .map_err(system)?;
            let view = Reader::map(object.as_fd(), shape.total()).map_err(RingError::System)?;
            Ok(Self {
                object,
                view,
                shape,
            })
        }

        pub(crate) fn shape(&self) -> Shape {
            self.shape
        }

        pub(crate) fn descriptor(&self) -> BorrowedFd<'_> {
            self.object.as_fd()
        }

        /// The first `length` bytes of `slot`, copied out, or `None` for a slot or a length the
        /// ring does not have.
        pub(crate) fn copy_slot(&self, slot: u32, length: usize) -> Option<Vec<u8>> {
            if length > self.shape.slot_len() {
                return None;
            }
            let offset = self.shape.offset(slot)?;
            let mut out = vec![0u8; length];
            self.view.copy_out(offset, &mut out).then_some(out)
        }
    }

    /// The worker's ring: the object's mapping, written a slot at a time.
    #[derive(Debug)]
    pub struct RingWriter {
        shape: Shape,
        view: Writer,
    }

    impl RingWriter {
        /// Maps the ring a [`RING`](super::RING) frame announced, from its payload and the descriptor that
        /// arrived beside its header.
        ///
        /// **The descriptor is given up and never closed**: the mapping holds the object, closing
        /// needs a system call `Profile::Media` does not admit, and the descriptor left open fills
        /// the one slot the profile's descriptor ceiling leaves, so that no second can arrive (ADR
        /// 3514). It is released from its owner rather than dropped, so nothing closes it on the
        /// way out of the process either.
        ///
        /// # Errors
        ///
        /// A payload that is not a shape, a frame with no descriptor or more than one beside it —
        /// a descriptor the kernel dropped for want of a slot is no descriptor — and the kernel's
        /// refusal to map.
        pub fn attach(
            payload: &[u8],
            descriptors: &mut Vec<ReceivedDescriptor>,
        ) -> std::io::Result<Self> {
            let shape = Shape::decode(payload).map_err(std::io::Error::other)?;
            if descriptors.len() != 1 {
                return Err(std::io::Error::other(format!(
                    "a ring frame with {} descriptors beside it, where one is a ring",
                    descriptors.len()
                )));
            }
            let Some(object) = descriptors.pop() else {
                return Err(std::io::Error::other("a ring frame with no descriptor"));
            };
            let view = Writer::map(object.into_raw_fd(), shape.total())?;
            Ok(Self { shape, view })
        }

        /// The ring's shape.
        #[must_use]
        pub fn shape(&self) -> Shape {
            self.shape
        }

        /// The slot item `sequence` is written into, whole.
        pub(crate) fn slot_mut(&mut self, sequence: u32) -> Option<(u32, &mut [u8])> {
            let slot = self.shape.slot_of(sequence);
            let offset = self.shape.offset(slot)?;
            let length = self.shape.slot_len();
            self.view
                .region_mut(offset, length)
                .map(|region| (slot, region))
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod elsewhere {
    use super::{RingError, Shape};
    use crate::link::ReceivedDescriptor;

    /// No ring on this platform: a type nothing constructs.
    #[derive(Debug)]
    pub(crate) struct HostRing(std::convert::Infallible);

    impl HostRing {
        pub(crate) fn create(_shape: Shape) -> Result<Self, RingError> {
            Err(RingError::Unsupported)
        }

        pub(crate) fn shape(&self) -> Shape {
            match self.0 {}
        }

        pub(crate) fn copy_slot(&self, _slot: u32, _length: usize) -> Option<Vec<u8>> {
            match self.0 {}
        }

        pub(crate) fn descriptor(&self) -> crate::SentDescriptor<'_> {
            match self.0 {}
        }
    }

    /// No ring on this platform: a type nothing constructs.
    #[derive(Debug)]
    pub struct RingWriter(std::convert::Infallible);

    impl RingWriter {
        /// Refuses: this platform has no ring, and its items cross inline.
        ///
        /// # Errors
        ///
        /// Always [`std::io::ErrorKind::Unsupported`].
        pub fn attach(
            _payload: &[u8],
            _descriptors: &mut Vec<ReceivedDescriptor>,
        ) -> std::io::Result<Self> {
            Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
        }

        /// The ring's shape.
        #[must_use]
        pub fn shape(&self) -> Shape {
            match self.0 {}
        }

        pub(crate) fn slot_mut(&mut self, _sequence: u32) -> Option<(u32, &mut [u8])> {
            match self.0 {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shape_reads_back_as_written() {
        let shape = Shape::new(8_294_400, 3, 1 << 30).expect("a 1080p frame thrice");
        assert_eq!(Shape::decode(&shape.encode()).expect("reads"), shape);
        assert_eq!(shape.total(), 3 * 8_294_400);
        assert_eq!((shape.slot_of(0), shape.slot_of(4)), (0, 1), "q mod slots");
        assert_eq!(shape.offset(2), Some(2 * 8_294_400));
        assert_eq!(shape.offset(3), None, "no fourth slot");
    }

    #[test]
    fn a_shape_past_its_budget_or_its_bounds_is_refused() {
        assert!(Shape::new(8_294_400, 3, 8_294_400 * 2).is_err(), "budget");
        assert!(Shape::new(0, 3, u64::MAX).is_err(), "empty slot");
        assert!(Shape::new(16, 0, u64::MAX).is_err(), "no slots");
        assert!(Shape::new(16, MAX_SLOTS + 1, u64::MAX).is_err(), "too many");
        assert!(Shape::decode(&[0; 7]).is_err(), "short");
        assert!(
            Shape::decode(&[0, 0, 0, 1, 0, 0, 0, 0]).is_err(),
            "no slots"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_object_is_sealed_against_shrinking_growing_and_new_seals() {
        let ring = HostRing::create(Shape::new(4096, 2, 1 << 20).expect("small"))
            .expect("a memfd on Linux");
        let seals = rustix::fs::fcntl_get_seals(ring.descriptor()).expect("seals read");
        assert!(seals.contains(
            rustix::fs::SealFlags::SHRINK
                | rustix::fs::SealFlags::GROW
                | rustix::fs::SealFlags::SEAL
        ));
        assert!(
            !seals.contains(rustix::fs::SealFlags::WRITE),
            "the worker writes"
        );
        assert!(
            rustix::fs::ftruncate(ring.descriptor(), 0).is_err(),
            "a holder cannot shrink it"
        );
        assert_eq!(ring.copy_slot(1, 4096).map(|bytes| bytes.len()), Some(4096));
        assert_eq!(ring.copy_slot(2, 1), None, "no third slot");
        assert_eq!(ring.copy_slot(0, 4097), None, "no longer than a slot");
    }
}
