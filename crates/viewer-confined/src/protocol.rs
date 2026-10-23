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
//! never crosses the pipe. That is `doc/ui-boundary.md`'s "one boundary, not two". A third,
//! [`viewer_core::Command::Archive`] with its grant, is [`Uncarried`] because a conversion parses
//! the whole document again, and the window's process is the side of this pipe that parses none
//! (ADR 1954).
//!
//! **A message that cannot be decoded is a refusal that says so.** [`ProtocolError`] names what
//! was truncated or unrecognised; nothing here defaults, clamps or guesses.
//!
//! # Where each concern lives
//!
//! This file holds the greeting, the frame kinds and what a sibling file reads; the codecs are
//! one module per concern (ADR 2256).
//!
//! | module | holds |
//! |---|---|
//! | `wire` | [`Writer`] and [`Reader`], the primitives every codec uses |
//! | `frame` | a frame header and the greeting |
//! | `error` | [`Uncarried`] and [`ProtocolError`] |
//! | `command`, `event`, `query`, `answer` | one message family each: discriminants, encode, decode |
//! | `view_fields`, `edit` | the fields a command shares |
//! | `marks`, `payload` | what a page crosses as, and the lists a host holds |
//! | `measurement` | §12.9's measurement and a located point |
//! | `panels`, `display_list`, `windowed`, `script_dialogues`, `published_policy` | the larger payloads |

mod answer;
mod command;
pub(crate) mod display_list;
mod edit;
mod error;
mod event;
mod frame;
mod marks;
mod measurement;
mod panels;
mod payload;
mod published_policy;
mod query;
mod script_dialogues;
#[cfg(test)]
mod tests;
mod view_fields;
pub(crate) mod windowed;
mod wire;

pub(crate) use answer::{decode_answer, decode_answer_reusing, encode_answer};
pub(crate) use command::{
    command_descriptor, decode_command, decode_command_holding, encode_command,
};
pub use error::{ProtocolError, Uncarried};
pub(crate) use event::{decode_events, encode_events};
pub(crate) use frame::{encode_handshake, header, parse_frame_header};
pub(crate) use marks::{Carries, Marks, decode_route, encode_route};
pub(crate) use payload::HeldLists;
pub(crate) use query::{OwnedQuery, decode_query, encode_query};
pub(crate) use wire::{Reader, Writer};

/// Greeting bytes, changed whenever this format changes incompatibly.
///
/// A host and a worker from different builds must not talk to each other, and the cheapest
/// place to find that out is the first thing either says.
///
/// It moves where something new genuinely crosses, and only there: a frame's payload discriminant
/// and the display list on one arm of it moved it, where the codec alone did not, because nothing
/// crossed in a frame then (ADR 0626). It also moves when a command, a question and an answer are
/// added even though each takes the *next* discriminant and nothing that crossed before changes
/// meaning, for a reason the bytes alone would not demand: an older worker cannot answer the new
/// question — and a host would discover that at the worst moment there is, in the middle of putting
/// a reader back after a death, as a refusal of something the reader never asked for. The greeting
/// is the cheap place to find it out instead.
pub(crate) const MAGIC: &[u8; 8] = b"PDFVCF18";

/// Length of the worker's greeting: the magic, the Landlock level, the address-space limit, and
/// whether system calls are filtered — the same three facts `pdf_sandbox`'s own worker reports,
/// because they are the same [`Confinement`](pdf_sandbox::lockdown::Confinement).
///
/// The bytes are `confined_transport`'s, because a greeting is transport rather than vocabulary:
/// `pdf-vfs`'s worker says the same three things under its own [`MAGIC`] (ADR 0846).
pub(crate) const HANDSHAKE_LEN: usize = confined_transport::greeting::LEN;

/// Length of a frame header: the kind and the payload length.
pub(crate) const FRAME_HEADER_LEN: usize = confined_transport::frame::HEADER_LEN;

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
/// Frame kind: how a page past the bound whose marks cross is to cross from its next frame on,
/// from the host, which prices the two routes (ADR 2017, `doc/questions/A477`).
///
/// A frame of its own rather than a command, because it is not `viewer-core`'s vocabulary: the
/// viewer asks for a page and is answered `Listed` either way, and which of the two forms crosses
/// is a question between this process and its host alone.
pub(crate) const FRAME_ROUTE: u8 = 6;

/// Frame payload: the pixels the confined process drew.
const PAYLOAD_RASTER: u8 = 0;
/// Frame payload: the marks, for a host that draws them itself (ADR 0607).
const PAYLOAD_LIST: u8 = 1;
/// Frame payload discriminant: a window of a page whose marks cannot cross, drawn here (ADRs 1986,
/// 2016).
const PAYLOAD_WINDOW: u8 = 2;

/// A restriction level byte that is not a level: this document departs from the window in nothing
/// here.
///
/// The fifth value of a four-value field, and only under the document's scope. `None` is not a
/// level — it is the absence of one — so it is spelled as a value the level arms do not claim
/// rather than by leaving the byte out, which keeps a policy six bytes wide whichever scope sent
/// it (ADR 1145).
const NO_DEPARTURE: u8 = 4;
