//! A stream: one request answered by a bounded run of items, paced by the host.
//!
//! [`crate::Host::exchange`] is a question and its answer, and that is the whole wire a confined
//! worker had: one reply per request (RFC 0012 section 5.1). Video is not that shape. A clip is
//! asked for once and answered with frames for as long as it plays, so this module is the
//! **streaming arm** RFC 0015 section 4.3 asks of this crate (ADR 3501): one [`OPEN`] frame, then
//! [`ITEM`] frames, each framed and length-checked exactly as a reply is, then one [`END`].
//!
//! # Flow control: credit, granted only when the last has been spent
//!
//! **The worker never pushes.** The host grants credit — a number of items — in the open request
//! and in each [`CREDIT`] frame; the worker sends at most that many items and then reads its input
//! for the next grant before it writes anything else, an [`END`] included. The host grants again
//! only when every item of the last grant has arrived. So there is never more than one control
//! frame in flight, every control frame the host writes is one the worker reads, and nothing is
//! left in the worker's input when a stream ends — which is why a worker with no second thread and
//! blocking reads can honour it, and why a confined worker's input never holds a stale frame its
//! next request would trip over. The host reads one item at a time, so its memory for a stream is
//! one item's.
//!
//! **What the host can see of a worker that ignores its credit, and why that is enough.** The host
//! grants again as soon as the last grant has arrived, so an item it receives after a grant is one
//! it cannot tell from an item the worker sent before reading the grant — a worker that never reads
//! is indistinguishable, item by item, from one that reads promptly. It is seen where the host has
//! granted nothing: after a stop, and once the frames budget is spent, each of which ends the
//! worker. Until then the pipe's own buffer bounds what an over-sending worker has written, and the
//! frames budget bounds the stream, so over-sending costs the host nothing the budgets do not
//! already bound.
//!
//! # Cancellation: a stop the worker reads, then a kill it cannot decline
//!
//! The host stops a stream by reading what it had granted (at most one grant's items, which it
//! throws away) and writing [`STOP`] where the next [`CREDIT`] would have gone. The worker reads it
//! where it would have read a grant, and ends with [`END`]; the worker and whatever it holds
//! survive, and the host may ask it something else. That is the cooperative half, and it is
//! sufficient only for a worker that keeps to the contract. The other half is
//! [`crate::Canceller`]'s kill, and a stream reaches for it on its own: an item past the credit
//! granted, a frame of any other kind, a sequence number out of order, a malformed item — each is
//! a worker that has broken the contract, and it is not asked anything more.
//!
//! # Budgets: frames, bytes and wall clock, each the host's
//!
//! Principle 3's budgets, per stream ([`Budget`]). **Frames** bound the credit: the host never
//! grants past the frames left, so a worker that sends more is over-sending. **Bytes** are checked
//! against each item's header before its payload is allocated; an item past them is read past,
//! and the stream is stopped. **Wall clock** cannot be checked by a host blocked in a read on a
//! worker that has stalled, so a watchdog thread holds the deadline and ends the worker when it
//! passes — the same kill, for [`crate::Canceller`]'s reason: a stall the worker has to agree to
//! end is one the document can decline to end. Every budget spent is said by name
//! ([`StreamError::Spent`]), never a stream that quietly came up short.
//!
//! # Two carriages: the pipe, and the ring
//!
//! An item carries its body after a sequence number and a **carriage** byte ([`Carriage`]).
//! [`Carriage::Inline`] is the body copied through the pipe, which is what a small item is.
//! [`Carriage::Slot`] names a slot of the shared-memory ring a host attached to the worker
//! ([`crate::ring`], ADR 3513): the worker wrote the body there, and the item says which slot and
//! how many bytes. The control frames — [`OPEN`], [`CREDIT`], [`STOP`], [`END`] — and their rules
//! are the same for both; what the ring adds is that **a grant is also a lease**: item `q` is
//! written into slot `q mod slots`, a grant never covers more items than the ring has slots, and
//! the host copies a slot out before it writes the grant that hands the slot back. An item that
//! names any other slot is a broken contract.

use std::io::{Read, Write};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::frame;
use crate::host::{Framed, Host, TransportError};
use crate::ring::{RingWriter, Shape};
use crate::supervision::Canceller;

/// The kind of the frame that opens a stream: the asking protocol's own kind and request inside.
///
/// The stream kinds sit beside [`frame::RESOURCE_REQUEST`]'s in the range past both protocols'
/// numbering, for that constant's reason: they are this transport's, not a vocabulary's, and a
/// build of either protocol that has never heard of them refuses them by name.
pub const OPEN: u8 = 0xF3;
/// The kind of a grant: four bytes, big-endian, a number of items above zero.
pub const CREDIT: u8 = 0xF4;
/// The kind of a stop: no payload.
pub const STOP: u8 = 0xF5;
/// The kind of an item: a sequence number, a carriage byte and the body.
pub const ITEM: u8 = 0xF6;
/// The kind of a stream's last frame: a reason byte and the worker's closing words.
pub const END: u8 = 0xF7;

/// Length of an open frame's payload before the asking protocol's request: its kind and the
/// first grant.
pub const OPEN_HEADER_LEN: usize = 1 + 4;
/// Length of an item's payload before its body: the sequence number and the carriage.
pub const ITEM_HEADER_LEN: usize = 4 + 1;
/// Largest closing a worker may end a stream with, in bytes: a sentence, not a payload.
pub const MAX_CLOSING: usize = 4096;
/// Length of a grant's payload.
const CREDIT_LEN: usize = 4;

/// How an item's body crosses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Carriage {
    /// The body follows the item's header in the same frame, copied through the pipe.
    Inline,
    /// The body is in a slot of the ring; the item carries the slot and the length, four bytes
    /// each, big-endian ([`SLOT_LEN`]).
    Slot,
}

impl Carriage {
    fn byte(self) -> u8 {
        match self {
            Self::Inline => 0,
            Self::Slot => 1,
        }
    }
}

/// Length of a [`Carriage::Slot`] item's body: the slot and the length of what was written there.
pub const SLOT_LEN: usize = 4 + 4;

/// An item's body as it arrived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body<'a> {
    /// The bytes themselves, after the header.
    Inline(&'a [u8]),
    /// A slot of the ring and how many of its bytes the worker wrote.
    Slot {
        /// Which slot.
        slot: u32,
        /// How many bytes from the slot's start.
        length: u32,
    },
}

impl Body<'_> {
    /// The body's length in bytes, wherever it is.
    #[must_use]
    pub fn len(&self) -> u64 {
        match self {
            Self::Inline(bytes) => u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            Self::Slot { length, .. } => u64::from(*length),
        }
    }

    /// Whether the body is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Why a worker ended a stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// It had nothing more to send.
    Finished,
    /// It read a [`STOP`] where it had more to send.
    Stopped,
}

impl Reason {
    fn byte(self) -> u8 {
        match self {
            Self::Finished => 0,
            Self::Stopped => 1,
        }
    }
}

/// What one stream may cost the host: principle 3's bounds, set by the host and never by the
/// worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// Items, at most.
    pub frames: u32,
    /// Bytes of item bodies, at most, counted before each is allocated.
    pub bytes: u64,
    /// Time the host waits on the worker, from the open request to the worker's [`END`]. The time
    /// the host holds an item it was handed, until it reads again, is the host's and is not
    /// counted: a host that paces its reads — a window drained by a presentation clock, slowed
    /// by load — withholds the worker's credit, and a worker kept waiting has not stalled (ADR
    /// 3902).
    pub wall: Duration,
}

/// Which of a stream's budgets was spent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spent {
    /// The items: the worker had more to send. The worker is kept.
    Frames,
    /// The bytes: an item would have passed them, and it was read past. The worker is kept.
    Bytes,
    /// The wall clock: the worker was ended, because a stalled worker cannot be asked to stop.
    Wall,
}

impl std::fmt::Display for Spent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Frames => "frames",
            Self::Bytes => "bytes",
            Self::Wall => "wall-clock time",
        })
    }
}

/// A stream frame that does not read as one, in a sentence.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct Malformed(pub String);

/// Why a stream ended without its worker's [`END`] in the shape the host asked for.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StreamError {
    /// The wire itself failed, as an exchange's would.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The worker broke the stream's contract, and it was ended rather than asked anything more.
    #[error("the confined worker broke the stream's contract and was ended: {0}")]
    Violation(String),
    /// One of the stream's budgets was spent; see [`Spent`] for whether the worker was kept.
    #[error("the stream spent its budget of {0}")]
    Spent(Spent),
    /// The thread that holds the wall-clock budget could not be started, so no stream was opened.
    #[error("the stream's wall-clock watch could not be started: {0}")]
    Watch(#[source] std::io::Error),
}

/// An open request as the worker reads it.
#[derive(Debug, PartialEq, Eq)]
pub struct Opened<'a> {
    /// The asking protocol's own kind for the request.
    pub kind: u8,
    /// The first grant.
    pub credit: NonZeroU32,
    /// The asking protocol's own request.
    pub request: &'a [u8],
}

/// An open frame's payload.
#[must_use]
pub fn encode_open(kind: u8, credit: NonZeroU32, request: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(OPEN_HEADER_LEN.saturating_add(request.len()));
    out.push(kind);
    out.extend_from_slice(&credit.get().to_be_bytes());
    out.extend_from_slice(request);
    out
}

/// Reads an open frame's payload.
///
/// # Errors
///
/// A payload shorter than its header, or a first grant of nothing.
pub fn decode_open(payload: &[u8]) -> Result<Opened<'_>, Malformed> {
    let (Some(&kind), Some(credit), Some(request)) = (
        payload.first(),
        payload.get(1..OPEN_HEADER_LEN),
        payload.get(OPEN_HEADER_LEN..),
    ) else {
        return Err(Malformed(format!(
            "an open request of {} bytes, shorter than its {OPEN_HEADER_LEN}-byte header",
            payload.len()
        )));
    };
    let credit = decode_credit(credit)?;
    Ok(Opened {
        kind,
        credit,
        request,
    })
}

/// A grant's payload.
#[must_use]
pub fn encode_credit(credit: NonZeroU32) -> [u8; CREDIT_LEN] {
    credit.get().to_be_bytes()
}

/// Reads a grant's payload.
///
/// # Errors
///
/// A payload that is not four bytes, or a grant of nothing.
pub fn decode_credit(payload: &[u8]) -> Result<NonZeroU32, Malformed> {
    let bytes: [u8; CREDIT_LEN] = payload.try_into().map_err(|_| {
        Malformed(format!(
            "a grant of {} bytes, which is {CREDIT_LEN}",
            payload.len()
        ))
    })?;
    NonZeroU32::new(u32::from_be_bytes(bytes))
        .ok_or_else(|| Malformed("a grant of no items".to_owned()))
}

/// An item's header, which its body follows.
#[must_use]
pub fn encode_item_header(sequence: u32, carriage: Carriage) -> [u8; ITEM_HEADER_LEN] {
    let mut out = [0u8; ITEM_HEADER_LEN];
    out[..4].copy_from_slice(&sequence.to_be_bytes());
    out[4] = carriage.byte();
    out
}

/// A [`Carriage::Slot`] item's body.
#[must_use]
pub fn encode_slot(slot: u32, length: u32) -> [u8; SLOT_LEN] {
    let mut out = [0u8; SLOT_LEN];
    out[..4].copy_from_slice(&slot.to_be_bytes());
    out[4..].copy_from_slice(&length.to_be_bytes());
    out
}

/// Reads an item's payload into its sequence number and its body.
///
/// # Errors
///
/// A payload shorter than its header, a carriage this build does not read, or a slot item whose
/// body is not [`SLOT_LEN`] bytes.
pub fn decode_item(payload: &[u8]) -> Result<(u32, Body<'_>), Malformed> {
    let (Some(sequence), Some(&carriage), Some(body)) = (
        payload.get(..4),
        payload.get(4),
        payload.get(ITEM_HEADER_LEN..),
    ) else {
        return Err(Malformed(format!(
            "an item of {} bytes, shorter than its {ITEM_HEADER_LEN}-byte header",
            payload.len()
        )));
    };
    let sequence: [u8; 4] = sequence
        .try_into()
        .map_err(|_| Malformed("an item's sequence number is not four bytes".to_owned()))?;
    let body = if carriage == Carriage::Inline.byte() {
        Body::Inline(body)
    } else if carriage == Carriage::Slot.byte() {
        let (Some(slot), Some(length), SLOT_LEN) =
            (be_u32(body.get(..4)), be_u32(body.get(4..)), body.len())
        else {
            return Err(Malformed(format!(
                "a slot item whose body is {} bytes, which is {SLOT_LEN}",
                body.len()
            )));
        };
        Body::Slot { slot, length }
    } else {
        return Err(Malformed(format!(
            "an item of carriage {carriage}, which this build does not read"
        )));
    };
    Ok((u32::from_be_bytes(sequence), body))
}

/// Four bytes, big-endian, where `bytes` is exactly four.
pub(crate) fn be_u32(bytes: Option<&[u8]>) -> Option<u32> {
    bytes?.try_into().ok().map(u32::from_be_bytes)
}

/// An end frame's payload.
#[must_use]
pub fn encode_end(reason: Reason, closing: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(closing.len().saturating_add(1));
    out.push(reason.byte());
    out.extend_from_slice(closing);
    out
}

/// Reads an end frame's payload into its reason and the worker's closing words.
///
/// # Errors
///
/// An empty payload, a reason this build does not know, or a closing past [`MAX_CLOSING`].
pub fn decode_end(payload: &[u8]) -> Result<(Reason, &[u8]), Malformed> {
    let Some((&reason, closing)) = payload.split_first() else {
        return Err(Malformed("an end with no reason".to_owned()));
    };
    if closing.len() > MAX_CLOSING {
        return Err(Malformed(format!(
            "an end whose closing is {} bytes, past the {MAX_CLOSING} a closing may be",
            closing.len()
        )));
    }
    let reason = match reason {
        0 => Reason::Finished,
        1 => Reason::Stopped,
        other => {
            return Err(Malformed(format!(
                "an end of reason {other}, which this build does not know"
            )));
        }
    };
    Ok((reason, closing))
}

/// Why the host is ending a stream before its worker has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stopping {
    /// The host's caller asked.
    Asked,
    /// A budget was spent.
    Spent(Spent),
}

/// What the host writes before it next reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Nothing: items it granted are still to come, or the end is.
    Read,
    /// A grant of this many items.
    Credit(NonZeroU32),
    /// A stop.
    Stop,
}

/// What one frame read inside a stream was.
#[derive(Debug, PartialEq, Eq)]
pub enum Accepted<'a> {
    /// An item for the host's caller: its body, or the slot it is in.
    Item(Body<'a>),
    /// An item read while the stream is stopping, or one past the bytes: counted and thrown away.
    Discarded,
    /// The worker's end.
    Ended {
        /// Why it ended.
        reason: Reason,
        /// What it said.
        closing: &'a [u8],
    },
}

/// The host's side of one stream, without the wire: what to write next and whether what was read
/// keeps the contract.
///
/// Public so the fuzz target can drive it over arbitrary frames; [`crate::Host::stream`] is how a
/// host uses it.
#[derive(Debug)]
pub struct Reception {
    budget: Budget,
    window: NonZeroU32,
    /// The ring the worker holds, if the host attached one.
    ring: Option<Shape>,
    /// Items granted and not yet arrived.
    outstanding: u32,
    /// Items arrived, which is also the sequence number the next must carry.
    received: u32,
    /// Item body bytes arrived.
    bytes: u64,
    stopping: Option<Stopping>,
    stop_sent: bool,
    ended: bool,
}

impl Reception {
    /// A stream paced `window` items a grant under `budget`, and its first grant — or `None` for a
    /// budget of no frames, which has nothing to open.
    #[must_use]
    pub fn new(window: NonZeroU32, budget: Budget) -> Option<(Self, NonZeroU32)> {
        Self::with_ring(window, budget, None)
    }

    /// As [`Self::new`], on a worker that holds a ring of `ring`'s shape: the window is clamped to
    /// the ring's slots, so that the items one grant covers name distinct slots ([`crate::ring`]'s
    /// ownership rule).
    #[must_use]
    pub fn with_ring(
        window: NonZeroU32,
        budget: Budget,
        ring: Option<Shape>,
    ) -> Option<(Self, NonZeroU32)> {
        let window = ring.map_or(window, |ring| window.min(ring.slots()));
        let first = NonZeroU32::new(window.get().min(budget.frames))?;
        Some((
            Self {
                budget,
                window,
                ring,
                outstanding: first.get(),
                received: 0,
                bytes: 0,
                stopping: None,
                stop_sent: false,
                ended: false,
            },
            first,
        ))
    }

    /// Items arrived so far, whether handed on or thrown away.
    #[must_use]
    pub fn received(&self) -> u32 {
        self.received
    }

    /// Items granted and not yet arrived.
    #[must_use]
    pub fn outstanding(&self) -> u32 {
        self.outstanding
    }

    /// Whether the worker's end has been read.
    #[must_use]
    pub fn is_ended(&self) -> bool {
        self.ended
    }

    /// Why the host is ending the stream, if it is.
    #[must_use]
    pub fn stopping(&self) -> Option<Stopping> {
        self.stopping
    }

    /// Asks the stream to stop: the items already granted are read and thrown away, and the stop
    /// goes where the next grant would have.
    pub fn stop(&mut self) {
        self.stopping.get_or_insert(Stopping::Asked);
    }

    /// What the host writes before it next reads, recorded as written.
    pub fn to_write(&mut self) -> Next {
        if self.ended || self.stop_sent || self.outstanding != 0 {
            return Next::Read;
        }
        let left = self.budget.frames.saturating_sub(self.received);
        let grant = NonZeroU32::new(self.window.get().min(left));
        match (self.stopping, grant) {
            (None, Some(grant)) => {
                self.outstanding = grant.get();
                Next::Credit(grant)
            }
            (stopping, _) => {
                self.stopping = Some(stopping.unwrap_or(Stopping::Spent(Spent::Frames)));
                self.stop_sent = true;
                Next::Stop
            }
        }
    }

    /// The longest frame the host reads now; a longer one is read past and given to
    /// [`Self::refused`].
    #[must_use]
    pub fn admitted(&self) -> usize {
        let left = self.budget.bytes.saturating_sub(self.bytes);
        let item = usize::try_from(left)
            .unwrap_or(usize::MAX)
            .saturating_add(ITEM_HEADER_LEN);
        item.max(MAX_CLOSING.saturating_add(1))
    }

    /// A frame of `kind` that was longer than [`Self::admitted`] and was read past.
    ///
    /// # Errors
    ///
    /// The broken contract, where it was not an item the host had granted.
    pub fn refused(&mut self, kind: u8, length: usize) -> Result<(), Malformed> {
        if kind != ITEM {
            return Err(Malformed(format!(
                "a frame of kind {kind} and {length} bytes inside a stream"
            )));
        }
        self.count_item()?;
        self.stopping.get_or_insert(Stopping::Spent(Spent::Bytes));
        Ok(())
    }

    /// One frame read inside the stream.
    ///
    /// # Errors
    ///
    /// The broken contract: a frame after the end, an item past the credit granted or out of
    /// sequence, an end giving a stop nobody asked for, a frame of any other kind, or one that
    /// does not read.
    pub fn accept<'a>(&mut self, kind: u8, payload: &'a [u8]) -> Result<Accepted<'a>, Malformed> {
        if self.ended {
            return Err(Malformed(format!(
                "a frame of kind {kind} after the stream's end"
            )));
        }
        match kind {
            ITEM => {
                let (sequence, body) = decode_item(payload)?;
                if sequence != self.received {
                    return Err(Malformed(format!(
                        "item {sequence} where item {} was next",
                        self.received
                    )));
                }
                if let Body::Slot { slot, length } = body {
                    self.check_slot(sequence, slot, length)?;
                }
                self.count_item()?;
                self.bytes = self.bytes.saturating_add(body.len());
                if self.bytes > self.budget.bytes {
                    self.stopping.get_or_insert(Stopping::Spent(Spent::Bytes));
                }
                Ok(if self.stopping.is_some() {
                    Accepted::Discarded
                } else {
                    Accepted::Item(body)
                })
            }
            END => {
                let (reason, closing) = decode_end(payload)?;
                if reason == Reason::Stopped && !self.stop_sent {
                    return Err(Malformed(
                        "an end giving a stop the host had not sent".to_owned(),
                    ));
                }
                self.ended = true;
                Ok(Accepted::Ended { reason, closing })
            }
            other => Err(Malformed(format!(
                "a frame of kind {other} inside a stream"
            ))),
        }
    }

    /// Whether a slot item names the slot the worker holds for it, and fits it.
    fn check_slot(&self, sequence: u32, slot: u32, length: u32) -> Result<(), Malformed> {
        let Some(ring) = self.ring else {
            return Err(Malformed(format!(
                "item {sequence} in ring slot {slot} on a worker that holds no ring"
            )));
        };
        let held = ring.slot_of(sequence);
        if slot != held {
            return Err(Malformed(format!(
                "item {sequence} in ring slot {slot}, where the worker holds slot {held} for it"
            )));
        }
        if usize::try_from(length).map_or(true, |length| length > ring.slot_len()) {
            return Err(Malformed(format!(
                "item {sequence} of {length} bytes in a ring slot of {}",
                ring.slot_len()
            )));
        }
        Ok(())
    }

    fn count_item(&mut self) -> Result<(), Malformed> {
        self.outstanding = self.outstanding.checked_sub(1).ok_or_else(|| {
            Malformed(format!(
                "an item past the credit granted, after {} items",
                self.received
            ))
        })?;
        self.received = self.received.saturating_add(1);
        Ok(())
    }
}

/// The thread that holds a stream's wall-clock budget, and ends the worker when it passes.
///
/// A thread rather than a deadline checked between reads, because the read it would have to
/// interrupt is the one a stalled worker never answers. One thread a stream is a few tens of
/// microseconds against a stream's seconds, and it exits the moment the stream ends.
///
/// **The clock stops while the host holds an item** ([`Budget::wall`], ADR 3902): from the item
/// handed out to the host's next read, the worker is waiting on the host's credit, and charging it
/// that time would stop a worker for its host's pace.
#[derive(Debug)]
struct Watch {
    /// What the stream tells the thread; dropped to disarm it.
    tell: Option<mpsc::Sender<Watched>>,
    /// Whether the host holds an item now, so each hold and each read is told once.
    held: bool,
    fired: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// What a stream tells its watch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Watched {
    /// The host was handed an item and holds it: the clock stops.
    Held,
    /// The host reads again: the clock runs on from where it stopped.
    Reading,
}

impl Watch {
    fn arm(canceller: Canceller, wall: Duration) -> Result<Self, std::io::Error> {
        let (tell, told) = mpsc::channel::<Watched>();
        let fired = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&fired);
        let thread = std::thread::Builder::new()
            .name("stream-wall-clock".to_owned())
            .spawn(move || {
                let mut left = wall;
                loop {
                    let began = Instant::now();
                    // A disarm is the sender dropped, which both receives report as disconnected.
                    match told.recv_timeout(left) {
                        Ok(Watched::Held) => {
                            left = left.saturating_sub(began.elapsed());
                            // The host's hold is unbounded here; its next read starts the clock.
                            loop {
                                match told.recv() {
                                    Ok(Watched::Reading) => break,
                                    Ok(Watched::Held) => {}
                                    Err(mpsc::RecvError) => return,
                                }
                            }
                        }
                        Ok(Watched::Reading) => left = left.saturating_sub(began.elapsed()),
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            flag.store(true, Ordering::SeqCst);
                            canceller.cancel();
                            return;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                }
            })?;
        Ok(Self {
            tell: Some(tell),
            held: false,
            fired,
            thread: Some(thread),
        })
    }

    /// The host was handed an item: the clock stops until [`Self::reading`].
    fn hold(&mut self) {
        if !self.held {
            self.held = true;
            self.tell(Watched::Held);
        }
    }

    /// The host reads again: the clock runs.
    fn reading(&mut self) {
        if self.held {
            self.held = false;
            self.tell(Watched::Reading);
        }
    }

    fn tell(&self, watched: Watched) {
        if let Some(tell) = &self.tell {
            // A send fails only once the thread has ended — fired or disarmed — which `fired`
            // already says; there is no clock left to stop or start.
            let _ = tell.send(watched);
        }
    }

    fn fired(&self) -> bool {
        self.fired.load(Ordering::SeqCst)
    }

    /// Stops the watch, and answers whether it had already ended the worker.
    fn disarm(&mut self) -> bool {
        drop(self.tell.take());
        if let Some(thread) = self.thread.take() {
            // The thread's body cannot panic: it receives, stores and kills, and `kill` already
            // treats a failure as the outcome asked for. A join error would carry nothing to act on.
            let _ = thread.join();
        }
        self.fired()
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.disarm();
    }
}

/// One open stream on a [`Host`]; see the module for the contract.
///
/// **A stream dropped before its end ends its worker**: the worker is mid-stream, and the only way
/// to leave the wire in a state the next request can use without reading is to not have a worker.
/// Read it to `None`, or call [`Self::stop`].
#[derive(Debug)]
pub struct Stream<'h> {
    host: &'h mut Host,
    reception: Reception,
    watch: Watch,
    closing: Option<Vec<u8>>,
    broken: bool,
}

/// What a step inside a stream came to.
enum Step {
    Item(Vec<u8>),
    Ended(Reason, Vec<u8>),
    Again,
}

impl<'h> Stream<'h> {
    pub(crate) fn open(
        host: &'h mut Host,
        kind: u8,
        request: &[u8],
        window: NonZeroU32,
        budget: Budget,
    ) -> Result<Self, StreamError> {
        host.still_running()?;
        let (reception, credit) = Reception::with_ring(window, budget, host.ring_shape())
            .ok_or(StreamError::Spent(Spent::Frames))?;
        let watch = Watch::arm(host.canceller(), budget.wall).map_err(StreamError::Watch)?;
        host.write(OPEN, &encode_open(kind, credit, request))?;
        Ok(Self {
            host,
            reception,
            watch,
            closing: None,
            broken: false,
        })
    }

    /// The next item's body, or `None` once the worker has ended the stream with nothing more to
    /// send.
    ///
    /// # Errors
    ///
    /// See [`StreamError`]: a budget spent, the contract broken, the wire failed.
    pub fn next_item(&mut self) -> Result<Option<Vec<u8>>, StreamError> {
        loop {
            if self.closing.is_some() || self.broken {
                return Ok(None);
            }
            match self.step()? {
                Step::Item(body) => {
                    self.watch.hold();
                    return Ok(Some(body));
                }
                Step::Ended(reason, closing) => {
                    self.closing = Some(closing);
                    return match (self.reception.stopping(), reason) {
                        (
                            None | Some(Stopping::Asked | Stopping::Spent(Spent::Frames)),
                            Reason::Finished,
                        ) => Ok(None),
                        (Some(Stopping::Spent(spent)), _) => Err(StreamError::Spent(spent)),
                        (_, Reason::Stopped) => Ok(None),
                    };
                }
                Step::Again => {}
            }
        }
    }

    /// Stops the stream: what was granted is read and thrown away, the worker reads the stop and
    /// ends, and its closing words are returned. The worker is kept.
    ///
    /// # Errors
    ///
    /// As [`Self::next_item`]; a budget spent before the stop is still said, and a stream whose
    /// earlier step failed has already ended its worker, so it answers
    /// [`TransportError::Cancelled`].
    pub fn stop(mut self) -> Result<Vec<u8>, StreamError> {
        if self.broken {
            return Err(StreamError::Transport(TransportError::Cancelled));
        }
        self.reception.stop();
        while self.closing.is_none() && !self.broken {
            if let Step::Ended(_, closing) = self.step()? {
                self.closing = Some(closing);
            }
        }
        match self.reception.stopping() {
            Some(Stopping::Spent(Spent::Bytes)) => Err(StreamError::Spent(Spent::Bytes)),
            _ => Ok(self.closing.take().unwrap_or_default()),
        }
    }

    /// Items the worker has sent so far, whether handed on or thrown away.
    #[must_use]
    pub fn received(&self) -> u32 {
        self.reception.received()
    }

    /// The worker's closing words, once it has ended the stream.
    #[must_use]
    pub fn closing(&self) -> Option<&[u8]> {
        self.closing.as_deref()
    }

    fn step(&mut self) -> Result<Step, StreamError> {
        self.watch.reading();
        let result = self.step_inner();
        if result.is_err() {
            // Whatever failed, the worker is somewhere inside a stream the host can no longer
            // follow, so it is not asked anything more (the module's cancellation).
            self.broken = true;
            self.host.canceller().cancel();
            self.watch.disarm();
        }
        result
    }

    fn step_inner(&mut self) -> Result<Step, StreamError> {
        if self.watch.fired() {
            return Err(StreamError::Spent(Spent::Wall));
        }
        match self.reception.to_write() {
            Next::Read => {}
            Next::Credit(credit) => self
                .host
                .write(CREDIT, &encode_credit(credit))
                .map_err(|error| self.transport(error))?,
            Next::Stop => self
                .host
                .write(STOP, &[])
                .map_err(|error| self.transport(error))?,
        }
        let framed = self
            .host
            .read_frame_within(self.reception.admitted())
            .map_err(|error| self.transport(error))?;
        match framed {
            Framed::Discarded { kind, length } => {
                self.reception
                    .refused(kind, length)
                    .map_err(|wrong| StreamError::Violation(wrong.0))?;
                Ok(Step::Again)
            }
            Framed::Frame(kind, mut payload) => {
                match self.reception.accept(kind, &payload) {
                    Ok(Accepted::Item(Body::Inline(_))) => {
                        // The body is the payload past the item's header; dropping the header in
                        // place moves the body once rather than copying it into a second buffer.
                        payload.drain(..ITEM_HEADER_LEN);
                        Ok(Step::Item(payload))
                    }
                    Ok(Accepted::Item(Body::Slot { slot, length })) => {
                        // The slot is the host's from the item's arrival to the next grant, which
                        // this copy comes before: the next grant is written by the next step.
                        let length = usize::try_from(length).unwrap_or(usize::MAX);
                        self.host
                            .copy_from_ring(slot, length)
                            .map(Step::Item)
                            .ok_or_else(|| {
                                StreamError::Violation(format!(
                                    "an item in ring slot {slot} of {length} bytes, which the \
                                     ring does not have"
                                ))
                            })
                    }
                    Ok(Accepted::Discarded) => Ok(Step::Again),
                    Ok(Accepted::Ended { reason, closing }) => {
                        let closing = closing.to_vec();
                        if self.watch.disarm() {
                            return Err(StreamError::Spent(Spent::Wall));
                        }
                        Ok(Step::Ended(reason, closing))
                    }
                    Err(wrong) => Err(StreamError::Violation(wrong.0)),
                }
            }
        }
    }

    fn transport(&self, error: TransportError) -> StreamError {
        if self.watch.fired() {
            return StreamError::Spent(Spent::Wall);
        }
        StreamError::Transport(error)
    }
}

impl Drop for Stream<'_> {
    fn drop(&mut self) {
        if !self.reception.is_ended() && !self.broken {
            self.host.canceller().cancel();
        }
    }
}

/// A control frame as the worker reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// A grant of this many items.
    Credit(NonZeroU32),
    /// A stop.
    Stop,
}

/// Reads the one control frame the host owes a worker whose credit is spent.
///
/// # Errors
///
/// [`std::io::ErrorKind::InvalidData`] for a frame that is not a grant or a stop, or one that
/// does not read; and the input's own errors.
pub fn read_control(input: &mut impl Read) -> Result<Control, std::io::Error> {
    let mut header = [0u8; frame::HEADER_LEN];
    input.read_exact(&mut header)?;
    let invalid = |what: String| std::io::Error::new(std::io::ErrorKind::InvalidData, what);
    match frame::parse_header(header) {
        Some((CREDIT, CREDIT_LEN)) => {
            let mut payload = [0u8; CREDIT_LEN];
            input.read_exact(&mut payload)?;
            decode_credit(&payload)
                .map(Control::Credit)
                .map_err(|wrong| invalid(wrong.0))
        }
        Some((STOP, 0)) => Ok(Control::Stop),
        Some((kind, length)) => Err(invalid(format!(
            "a frame of kind {kind} and {length} bytes where a grant or a stop was owed"
        ))),
        None => Err(invalid("a frame header this build cannot read".to_owned())),
    }
}

/// What [`Emitter::emit`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emitted {
    /// The item was sent.
    Sent,
    /// The host stopped the stream; the item was not sent and the end has been written.
    Stopped,
}

/// The worker's side of one stream: it sends no item it holds no credit for, and reads the host's
/// next grant or stop when it holds none.
#[derive(Debug)]
pub struct Emitter {
    credit: u32,
    sequence: u32,
    ended: bool,
}

impl Emitter {
    /// A stream the host opened with `credit`.
    #[must_use]
    pub fn new(credit: NonZeroU32) -> Self {
        Self {
            credit: credit.get(),
            sequence: 0,
            ended: false,
        }
    }

    /// Sends `body` as the next item, first reading a grant or a stop if the credit is spent.
    ///
    /// # Errors
    ///
    /// A stream already ended, a control frame that is not one ([`read_control`]), and the pipes'
    /// own errors.
    pub fn emit(
        &mut self,
        input: &mut impl Read,
        output: &mut impl Write,
        body: &[u8],
    ) -> Result<Emitted, std::io::Error> {
        if !self.hold_credit(input, output)? {
            return Ok(Emitted::Stopped);
        }
        output.write_all(&frame::header(
            ITEM,
            ITEM_HEADER_LEN.saturating_add(body.len()),
        ))?;
        output.write_all(&encode_item_header(self.sequence, Carriage::Inline))?;
        output.write_all(body)?;
        output.flush()?;
        self.spend_credit();
        Ok(Emitted::Sent)
    }

    /// Sends the next item through the ring: first the grant or the stop, if the credit is spent,
    /// and only then the slot, to `write`, which fills it and answers how many bytes it wrote.
    ///
    /// The order is the ring's ownership rule ([`crate::ring`]): the slot item `q` is written into
    /// is the worker's once it holds the credit for item `q`, and not before, so the closure is not
    /// called on a stopped stream at all.
    ///
    /// # Errors
    ///
    /// As [`Self::emit`], and a length `write` answers past the slot it was given.
    pub fn emit_in_ring(
        &mut self,
        input: &mut impl Read,
        output: &mut impl Write,
        ring: &mut RingWriter,
        write: impl FnOnce(&mut [u8]) -> usize,
    ) -> Result<Emitted, std::io::Error> {
        if !self.hold_credit(input, output)? {
            return Ok(Emitted::Stopped);
        }
        let Some((slot, region)) = ring.slot_mut(self.sequence) else {
            return Err(std::io::Error::other("a ring slot this ring does not have"));
        };
        let capacity = region.len();
        let written = write(region);
        let length = u32::try_from(written)
            .ok()
            .filter(|_| written <= capacity)
            .ok_or_else(|| {
                std::io::Error::other(format!(
                    "{written} bytes written into a ring slot of {capacity}"
                ))
            })?;
        output.write_all(&frame::header(ITEM, ITEM_HEADER_LEN + SLOT_LEN))?;
        output.write_all(&encode_item_header(self.sequence, Carriage::Slot))?;
        output.write_all(&encode_slot(slot, length))?;
        output.flush()?;
        self.spend_credit();
        Ok(Emitted::Sent)
    }

    /// Whether the worker holds credit for one more item, reading the host's grant or stop where
    /// it is spent; a stop is answered with the end, and with `false`.
    fn hold_credit(
        &mut self,
        input: &mut impl Read,
        output: &mut impl Write,
    ) -> Result<bool, std::io::Error> {
        if self.ended {
            return Err(std::io::Error::other("an item after the stream's end"));
        }
        if self.credit == 0 {
            match read_control(input)? {
                Control::Credit(credit) => self.credit = credit.get(),
                Control::Stop => {
                    self.ended = true;
                    write_end(output, Reason::Stopped, &[])?;
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn spend_credit(&mut self) {
        self.credit = self.credit.saturating_sub(1);
        self.sequence = self.sequence.wrapping_add(1);
    }

    /// Ends the stream with nothing more to send, first reading the grant or stop the host owes a
    /// worker whose credit is spent — either is answered by the same end.
    ///
    /// # Errors
    ///
    /// As [`Self::emit`], and a closing past [`MAX_CLOSING`].
    pub fn finish(
        mut self,
        input: &mut impl Read,
        output: &mut impl Write,
        closing: &[u8],
    ) -> Result<(), std::io::Error> {
        if self.ended {
            return Ok(());
        }
        if closing.len() > MAX_CLOSING {
            return Err(std::io::Error::other(format!(
                "a closing of {} bytes, past the {MAX_CLOSING} a closing may be",
                closing.len()
            )));
        }
        if self.credit == 0 {
            read_control(input)?;
        }
        self.ended = true;
        write_end(output, Reason::Finished, closing)
    }
}

fn write_end(
    output: &mut impl Write,
    reason: Reason,
    closing: &[u8],
) -> Result<(), std::io::Error> {
    let payload = encode_end(reason, closing);
    output.write_all(&frame::header(END, payload.len()))?;
    output.write_all(&payload)?;
    output.flush()
}

#[cfg(test)]
mod tests;
