//! A frame: one kind byte, one big-endian length, and the payload after them.
//!
//! Fixed-width and length-checked before anything is allocated from it, for the reason
//! `pdf_sandbox::protocol` gives: the confined side is the untrusted side of the boundary, so a
//! length it states is a claim rather than a fact.

/// Length of a frame header: the kind and the payload length.
pub const HEADER_LEN: usize = 1 + 8;

/// Largest message either side will read, in bytes.
///
/// A document's bytes and a page's pixels both cross this wire, so the bound cannot be small:
/// ISO 32000-2 itself is 25 MB and a 4K page of RGBA is 33 MB. Two gibibytes is a bound against a
/// length that is a claim rather than a size, which is the only thing it is for — the reader
/// refuses before it allocates, rather than believing a header and asking for the machine.
pub const MAX_MESSAGE: u64 = 2 << 30;

/// A payload's kind and length, to be written in front of it.
///
/// **Nine bytes, written separately from the payload rather than in front of a copy of it.** A
/// document is 19.2 MB and a raster is 4.1 MB, so a frame that concatenated would be one whole
/// extra pass over the largest thing this transport carries — and the pipe's own cost for those
/// bytes is about a tenth of what the copies around it cost (ADR 0241). Two `write_all` calls on
/// a socket are two system calls; the concatenation was megabytes of memory traffic and the page
/// faults to go with it.
#[must_use]
pub fn header(kind: u8, length: usize) -> [u8; HEADER_LEN] {
    let mut out = [0u8; HEADER_LEN];
    out[0] = kind;
    out[1..].copy_from_slice(&as_u64(length).to_be_bytes());
    out
}

/// Reads a frame header, or `None` where the length is past [`MAX_MESSAGE`].
///
/// **The kind comes back untouched and is the caller's to recognise.** A transport that validated
/// it would have to hold both protocols' discriminants, which is the one thing that would stop it
/// being shareable; each crate's own reader matches the byte against its own set and refuses what
/// it does not define.
#[must_use]
pub fn parse_header(header: [u8; HEADER_LEN]) -> Option<(u8, usize)> {
    let kind = *header.first()?;
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
/// reader refuses rather than a number that is quietly wrong.
fn as_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{HEADER_LEN, MAX_MESSAGE, header, parse_header};

    #[test]
    fn a_header_round_trips() {
        assert_eq!(parse_header(header(7, 12345)), Some((7, 12345)));
    }

    /// A length past the bound is refused at the header, before a buffer is sized from it.
    #[test]
    fn a_length_past_the_bound_is_refused_before_anything_is_allocated() {
        let mut absurd = [0u8; HEADER_LEN];
        absurd[0] = 1;
        absurd[1..].copy_from_slice(&(MAX_MESSAGE + 1).to_be_bytes());
        assert_eq!(parse_header(absurd), None);
    }

    /// And a kind this transport has never heard of comes back, because it is not this
    /// transport's to judge.
    #[test]
    fn an_unknown_kind_is_the_callers_to_refuse() {
        assert_eq!(parse_header(header(200, 0)), Some((200, 0)));
    }
}
