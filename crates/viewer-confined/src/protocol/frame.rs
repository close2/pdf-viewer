//! Frame headers and the greeting, the bytes before any message.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

use pdf_sandbox::lockdown::Confinement;

use super::{
    FRAME_ANSWER, FRAME_COMMAND, FRAME_EVENTS, FRAME_HEADER_LEN, FRAME_QUERY, FRAME_REFUSAL,
    FRAME_ROUTE, HANDSHAKE_LEN, MAGIC,
};

/// A payload's kind and length, to be written in front of it.
///
/// **Nine bytes, written separately from the payload rather than in front of a copy of it.** A
/// document is 19.2 MB and a raster is 4.1 MB, so a `frame` that concatenated would be one whole
/// extra pass over the largest thing this transport carries — and the pipe's own cost for those
/// bytes is about a tenth of what the copies around it cost (ADR 0241). Two `write_all` calls on
/// a pipe are two system calls; the concatenation was megabytes of memory traffic and the page
/// faults to go with it.
pub(crate) fn header(kind: u8, length: usize) -> [u8; FRAME_HEADER_LEN] {
    confined_transport::frame::header(kind, length)
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

/// A count as the fixed-width number this format carries.
///
/// `try_from` rather than `as`: on every platform this compiles for `usize` is at most 64 bits,
/// so the conversion cannot fail — and on a hypothetical wider one the fallback is a length the
/// reader refuses as [`ProtocolError::Overlong`](super::error::ProtocolError::Overlong) rather than a number that is quietly wrong.
pub(super) fn as_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Reads a frame header, or `None` if the kind is not one this build defines or the length is
/// past [`MAX_MESSAGE`].
///
/// **The kind is checked here rather than in the transport**, which is the line ADR 0846 draws: a
/// wire two protocols share cannot hold either one's discriminants, so it answers a length and
/// this says whether the byte in front of it is one of the five frames *this* protocol defines.
///
/// [`MAX_MESSAGE`]: confined_transport::frame::MAX_MESSAGE
pub(crate) fn parse_frame_header(header: [u8; FRAME_HEADER_LEN]) -> Option<(u8, usize)> {
    let (kind, length) = confined_transport::frame::parse_header(header)?;
    if !matches!(
        kind,
        FRAME_COMMAND | FRAME_QUERY | FRAME_EVENTS | FRAME_ANSWER | FRAME_REFUSAL | FRAME_ROUTE
    ) {
        return None;
    }
    Some((kind, length))
}

/// Encodes the worker's greeting.
pub(crate) fn encode_handshake(confinement: Confinement) -> [u8; HANDSHAKE_LEN] {
    confined_transport::greeting::encode(MAGIC, confinement)
}

/// Reads the worker's greeting, or `None` if it is not one.
///
/// The host reads it inside `confined_transport::Host::start`, so what is left for this crate is
/// the assertion that *its* magic is what a greeting has to carry.
#[cfg(test)]
pub(crate) fn parse_handshake(greeting: &[u8; HANDSHAKE_LEN]) -> Option<Confinement> {
    confined_transport::greeting::parse(MAGIC, greeting)
}
