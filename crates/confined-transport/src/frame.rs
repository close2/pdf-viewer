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

/// The kind a confined worker asks its host for a resource with.
///
/// **This is the one kind this crate defines, and the exception is argued rather than assumed.**
/// Everything else about a frame's kind byte is the caller's — see [`parse_header`] — because a
/// transport that knew both protocols' discriminants could not be shared by both. This pair is
/// not either protocol's: it is a *request in the other direction*, from the confined side to the
/// privileged one, and it is answered inside [`crate::Host`] before a protocol's own reader ever
/// sees it. Neither `viewer-confined`'s vocabulary nor `pdf-vfs`'s gains an arm, and neither
/// crate's brokers gain a re-entrant call site — which is the cost ADR 0874 declined to pay for
/// the *ask* level and does not have to pay here.
///
/// The value is past both protocols' ranges (each numbers its frames from 1), so a build of
/// either that has never heard of this one refuses it by name rather than reading it as a
/// message.
pub const RESOURCE_REQUEST: u8 = 0xF0;

/// The kind a host answers [`RESOURCE_REQUEST`] with.
///
/// Its payload is a big-endian `u32` naming the length of the *identity* — whatever the asking
/// protocol calls the thing, which this crate does not read — followed by that identity and then
/// the resource's own bytes. An empty payload is *nothing offered*, which is the default answer
/// and is not an error.
///
/// # Why the bytes and not a descriptor, which is what was written first
///
/// `doc/todo/59` stated a descriptor, beside the document's (ADR 0812), and the first
/// implementation sent one: it worked, and it **killed every debug build**. The descriptor arrives
/// as a `std::os::fd::OwnedFd`, a font file is read once and the descriptor is then dropped — and
/// `OwnedFd::drop` asks `fcntl(fd, F_GETFD)` before `close`, under
/// `core::ub_checks::check_library_ub()`, to catch a double close. `fcntl` is not on the
/// confinement's allow-list and the allow-list's action is `SECCOMP_RET_KILL_PROCESS`, so the
/// worker died with `SIGSYS` on the line *after* the `pread64` succeeded. From the worker's own
/// `strace`:
///
/// ```text
/// recvmsg(0, …, cmsg_type=SCM_RIGHTS, cmsg_data=[3] …) = 9
/// pread64(3, "OTTO\0\f\0\200\0\3\0@CFF …", 82264, 0) = 82264
/// fcntl(3, F_GETFD)                       = 0x48
/// +++ killed by SIGSYS (core dumped) +++
/// ```
///
/// There is no way to close a descriptor from safe Rust without that check — every std type that
/// owns one closes through `OwnedFd` — and the two ways out are both refused here: widening the
/// allow-list is what `doc/todo/61` exists to forbid, and leaking the descriptor spends one of the
/// eight `RLIMIT_NOFILE` leaves. So the resource crosses as bytes, which costs one copy of a file
/// that is tens of megabytes at worst and is the same arm `Command::Open` already has for a
/// document held in memory.
///
/// **The finding is larger than this port and is recorded rather than fixed here**: the document's
/// own descriptor is dropped when a document is *closed*, so ADR 0812's route has the same latent
/// kill in a build with library-UB checks on. `doc/todo/61` carries it.
pub const RESOURCE_ANSWER: u8 = 0xF1;

/// Largest resource request a host will read before refusing it.
///
/// A description is a family, a weight and a handful of characters. The bound is here because the
/// worker is the untrusted side of this boundary and a length it states is a claim.
pub const MAX_RESOURCE_REQUEST: usize = 64 * 1024;

/// Largest resource that crosses this wire, in bytes.
///
/// The largest font file on a normal machine is a CJK collection of a few tens of megabytes. This
/// is well past that and well under the worker's address-space ceiling, and it exists at both ends
/// so that neither side turns the other's stated length into an allocation it cannot survive.
pub const MAX_RESOURCE: usize = 256 << 20;

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
