//! The resource port: how a process that cannot open a file is given a face anyway.
//!
//! # Why this module exists
//!
//! [`crate::substitute`] answers §9.10.2's question — which face stands in for one the document
//! named and did not embed — by walking the machine's font directories. A *confined* worker
//! cannot: `openat` is not on `pdf-sandbox`'s allow-list, whose action is
//! `SECCOMP_RET_KILL_PROCESS`, so the walk is fatal rather than fallible, and
//! [`crate::substitute::no_machine_fonts`] is the statement that stops it (ADR 0870). What that
//! bought was a live worker at a stated fidelity cost: a document naming an uninstalled CJK or
//! Arabic face is drawn from the compiled-in Latin faces.
//!
//! This module is how the cost is paid back **without giving the worker a filesystem**.
//!
//! # What the description is, and why the standard already names it
//!
//! ISO 32000-2 §9.8.1, on the font descriptor:
//!
//! > These font metrics provide information that enables a PDF processor to synthesise a
//! > substitute font or select a similar font when the font program is unavailable.
//!
//! **So the thing that crosses this port is the thing the clause says the descriptor is for.**
//! [`crate::substitute::Request`] is derived from the document alone — Table 120's `/Flags`,
//! `/FontWeight` and `/ItalicAngle`, and the `/BaseFont` name — and the characters beside it are
//! §9.10.2's, the ones a composite font's script needs before a face can be said to draw it. A
//! path is not among them and could not be: the document never states one.
//!
//! # A port, not a permission
//!
//! The worker's system-call set does not change and no host can change it. What changes is that
//! the worker may *ask*, by description, and be handed an answer:
//!
//! - it asks with a [`crate::substitute::Request`] — a family, a weight, a slope, and the
//!   characters a script needs. **Never a path**, so a `/BaseFont` out of an untrusted file never
//!   becomes a path lookup inside the process parsing untrusted bytes;
//! - the broker — unconfined, and the process that already opens the document — matches that
//!   description with [`crate::substitute::machine_face`], opens the file, and hands back an open
//!   **descriptor** beside the face's name, over the channel §7.5.6's document already crosses
//!   (`SCM_RIGHTS`, ADR 0812);
//! - the worker reads the descriptor positionally with the `pread64` it already has.
//!
//! # `can`, not `must`
//!
//! Nothing is installed by default. [`offered`] answers `None` until a host has both armed the
//! worker with [`faces_come_from`] and answered its requests, so a host that ignores this layer
//! is exactly the host that shipped before it: [`crate::substitute::find`] still never fails,
//! still substitutes from [`crate::standard`]'s compiled-in faces, and `pdf_model::interpret`
//! still reports the shortfall under §9.10.2. Nothing here turns a substitution into a failure.
//!
//! # One matcher, not two
//!
//! The broker's matching is [`crate::substitute::machine_face`] — the same walk, in the same
//! order, that an unconfined process uses for itself. [`open_a_face`] is a decode, that call, and
//! an open; there is no second implementation of "which face answers this description" for a
//! confined worker to disagree with an unconfined one about.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use crate::substitute::{Family, Request};

/// Whether a host offers a confined worker the faces installed on this machine.
///
/// **The default is [`Self::Withheld`]**, and that is `CLAUDE.md` principle 3 rather than caution:
/// a host that says nothing gets the worker that shipped before this port existed. Offering is a
/// deliberate act — a flag on a command line, a setting in a window — by a host that has decided
/// its user's own font files may be read on its user's behalf.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MachineFaces {
    /// Nothing is offered. A worker substitutes from [`crate::standard`]'s compiled-in faces and
    /// reports the shortfall under §9.10.2, exactly as it did before this layer existed.
    #[default]
    Withheld,
    /// The broker matches a description against this machine and hands over a descriptor.
    Offered,
}

/// How a confined worker puts a request to its broker.
///
/// The bytes in are [`encode_request`]'s; the bytes out are the face's identity as
/// [`encode_identity`] wrote it, and the face's own program. `None` means the broker offered
/// nothing, which is the default answer and is not an error.
///
/// **A function pointer rather than a trait object**, so that arming the port is one line in the
/// worker and needs no type of this crate's on the transport's side of the boundary: the crate
/// that owns the wire (`confined-transport`) knows nothing about fonts, and this crate knows
/// nothing about the wire.
pub type Ask = fn(&[u8]) -> Option<(Vec<u8>, Vec<u8>)>;

/// The installed way of asking, if a worker armed one.
static ASK: OnceLock<Ask> = OnceLock::new();

/// Whether anything has been armed at all, read on the fast path without touching the `OnceLock`.
static ARMED: AtomicBool = AtomicBool::new(false);

/// One remembered offer: the request as it went out, and what came back.
type Offered = (Vec<u8>, Option<(Arc<[u8]>, String)>);

/// Offers already made, so a miss costs one round trip rather than one per page.
///
/// **Bounded by the description rather than by a ceiling**, which is why there is no eviction: a
/// key is a family (five), a weight and a slope (four combinations), a `skip` under
/// `substitute::MAX_OFFERS`, and the characters a registered character collection samples. A
/// document cannot invent a key outside that set, so what this holds is at most the machine's
/// faces for the families a document names — the same population `substitute::LOADED` holds in a
/// process that reads them itself.
static OFFERS: OnceLock<RwLock<Vec<Offered>>> = OnceLock::new();

/// Arms this process to ask its broker for faces it cannot look up itself.
///
/// Called by a worker in the same paragraph as [`crate::substitute::no_machine_fonts`] — before
/// the confinement, because that is the last moment anything can be decided — and it is the
/// *only* thing that turns this port on inside a worker. A process that never calls it never
/// asks.
///
/// Calling it twice keeps the first arming, which is the safe direction: a port that could be
/// re-aimed would be a way to redirect a confined process's font requests after the fact.
pub fn faces_come_from(ask: Ask) {
    if ASK.set(ask).is_ok() {
        ARMED.store(true, Ordering::Release);
    }
}

/// Whether this process has been armed to ask for faces.
#[must_use]
pub fn armed() -> bool {
    ARMED.load(Ordering::Acquire)
}

/// The face a broker offers for a description, and what it calls it.
///
/// `skip` names how many of the matcher's own answers to pass over, which is what lets a caller
/// that judges a face by its *contents* — `crate::substitute::installed_wider` compares code
/// tables — walk the same preference list a machine-reading process walks rather than being
/// handed one candidate and no way to ask for the next.
///
/// Answers `None` where nothing is armed, where the broker offered nothing, or where the answer
/// did not decode. Each of those is the same thing to a caller: this machine offers no face, and
/// [`crate::standard`]'s compiled-in one is what draws the text.
#[must_use]
pub fn offered(request: Request, wanted: &[char], skip: u32) -> Option<(Arc<[u8]>, String)> {
    if !armed() {
        return None;
    }
    let asked = encode_request(request, wanted, skip);

    let memo = OFFERS.get_or_init(|| RwLock::new(Vec::new()));
    if let Ok(held) = memo.read()
        && let Some((_, answer)) = held.iter().find(|(cached, _)| *cached == asked)
    {
        return answer.clone();
    }

    let ask = ASK.get()?;
    let answer = ask(&asked).and_then(|(identity, program)| {
        let name = decode_identity(&identity)?;
        Some((Arc::<[u8]>::from(program), name))
    });
    if let Ok(mut held) = memo.write() {
        held.push((asked, answer.clone()));
    }
    answer
}

/// The broker's side: matches a request against this machine and reads the face it names.
///
/// **This is the only place a path appears in the whole port**, and it is in the unconfined
/// process by construction: the request that arrives carries a family and a set of characters,
/// the answer that leaves carries a font program and the file's own name. Neither direction
/// carries a path, so nothing a document says can aim this at a file.
///
/// There are no errors, deliberately: a request that does not decode, a machine with no matching
/// face, and a file that cannot be read are one answer — `None`, which the worker reads as *this
/// machine offers nothing* and answers from the compiled-in faces.
#[must_use]
pub fn open_a_face(request: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let (request, wanted, skip) = decode_request(request)?;
    let path = crate::substitute::machine_face(request, &wanted, skip)?;
    let content = std::fs::read(&path).ok()?;
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    Some((encode_identity(&name), content))
}

/// This encoding's version byte, so a worker and a broker built apart refuse rather than guess.
const VERSION: u8 = 1;

/// A family as one byte.
fn family_byte(family: Family) -> u8 {
    match family {
        Family::Serif => 0,
        Family::SansSerif => 1,
        Family::Monospace => 2,
        Family::Symbol => 3,
        Family::ZapfDingbats => 4,
    }
}

/// A family from one byte.
fn family_of(byte: u8) -> Option<Family> {
    Some(match byte {
        0 => Family::Serif,
        1 => Family::SansSerif,
        2 => Family::Monospace,
        3 => Family::Symbol,
        4 => Family::ZapfDingbats,
        _ => return None,
    })
}

/// A description on the wire: what the document asked for, and nothing about this machine.
///
/// Fixed-width and self-describing, for the reason `confined_transport::frame` gives about its
/// own header: the side that wrote it is the untrusted side of some boundary, so every length is
/// a claim to be checked before anything is sized from it.
#[must_use]
pub fn encode_request(request: Request, wanted: &[char], skip: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(11usize.saturating_add(wanted.len().saturating_mul(4)));
    out.push(VERSION);
    out.push(family_byte(request.family));
    let flags = u8::from(request.bold)
        | (u8::from(request.italic) << 1)
        | (u8::from(request.standard) << 2);
    out.push(flags);
    out.extend_from_slice(&skip.to_be_bytes());
    out.extend_from_slice(
        &u32::try_from(wanted.len())
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    for character in wanted {
        out.extend_from_slice(&(*character as u32).to_be_bytes());
    }
    out
}

/// Reads a description written by [`encode_request`], or `None` where it is not one.
#[must_use]
pub fn decode_request(bytes: &[u8]) -> Option<(Request, Vec<char>, u32)> {
    if bytes.first().copied()? != VERSION {
        return None;
    }
    let family = family_of(bytes.get(1).copied()?)?;
    let flags = bytes.get(2).copied()?;
    let skip = u32::from_be_bytes(bytes.get(3..7)?.try_into().ok()?);
    let count = u32::from_be_bytes(bytes.get(7..11)?.try_into().ok()?);
    let count = usize::try_from(count).ok()?;
    // The count is a claim about a slice that is already in hand, so it is checked against the
    // slice rather than believed: `wanted` is sized from what arrived, never from the number.
    let rest = bytes.get(11..)?;
    if rest.len() != count.checked_mul(4)? {
        return None;
    }
    let mut wanted = Vec::with_capacity(count);
    for chunk in rest.chunks_exact(4) {
        let value = u32::from_be_bytes(chunk.try_into().ok()?);
        wanted.push(char::from_u32(value)?);
    }
    Some((
        Request {
            family,
            bold: flags & 1 != 0,
            italic: flags & 2 != 0,
            standard: flags & 4 != 0,
        },
        wanted,
        skip,
    ))
}

/// What identifies the face that answered, which is its file's own name and not its path.
///
/// **The name rather than the path, and the difference is the whole point of the port.** A worker
/// has no use for a path — it cannot open one — and a name is what a report about a substitution
/// would have to print. Sending the path would be telling a process that parses untrusted bytes
/// where this machine keeps its files, for nothing.
#[must_use]
pub fn encode_identity(name: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(1usize.saturating_add(name.len()));
    out.push(VERSION);
    out.extend_from_slice(name.as_bytes());
    out
}

/// Reads an identity written by [`encode_identity`], or `None` where it is not one.
#[must_use]
pub fn decode_identity(bytes: &[u8]) -> Option<String> {
    if bytes.first().copied()? != VERSION {
        return None;
    }
    Some(String::from_utf8_lossy(bytes.get(1..)?).into_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        decode_identity, decode_request, encode_identity, encode_request, offered, open_a_face,
    };
    use crate::substitute::{Family, Request};

    /// The description a worker sends is the description a broker reads.
    #[test]
    fn a_description_round_trips_and_carries_no_path() {
        let request = Request {
            family: Family::Serif,
            bold: true,
            italic: false,
            standard: false,
        };
        let wanted = ['中', 'あ'];
        let bytes = encode_request(request, &wanted, 3);
        let (back, chars, skip) = decode_request(&bytes).expect("a description");
        assert_eq!(back, request);
        assert_eq!(chars, wanted.to_vec());
        assert_eq!(skip, 3);
    }

    /// A version byte this build does not know is refused rather than guessed at.
    #[test]
    fn a_description_from_another_build_is_refused() {
        let mut bytes = encode_request(
            Request {
                family: Family::SansSerif,
                bold: false,
                italic: false,
                standard: false,
            },
            &[],
            0,
        );
        bytes[0] = 99;
        assert!(decode_request(&bytes).is_none());
        assert!(open_a_face(&bytes).is_none());
    }

    /// A stated character count that the bytes do not carry is refused before anything is sized
    /// from it, which is `confined_transport::frame`'s rule one layer down.
    #[test]
    fn a_stated_count_the_bytes_do_not_carry_is_refused() {
        let mut bytes = encode_request(
            Request {
                family: Family::Serif,
                bold: false,
                italic: false,
                standard: false,
            },
            &['A'],
            0,
        );
        bytes[7..11].copy_from_slice(&1000u32.to_be_bytes());
        assert!(decode_request(&bytes).is_none());
    }

    /// A face's identity round trips, and it is a name rather than a path.
    #[test]
    fn an_identity_round_trips() {
        assert_eq!(
            decode_identity(&encode_identity("DroidSansFallback.ttf")).as_deref(),
            Some("DroidSansFallback.ttf")
        );
    }

    /// **The default provides nothing**, which is what makes a host that ignores this layer the
    /// host that shipped before it. Nothing in this test process arms the port.
    #[test]
    fn nothing_is_offered_until_a_worker_arms_the_port() {
        assert!(!super::armed());
        assert!(
            offered(
                Request {
                    family: Family::Serif,
                    bold: false,
                    italic: false,
                    standard: false,
                },
                &[],
                0
            )
            .is_none()
        );
    }
}
