//! A confined worker that streams what its request names, and breaks the stream's contract when
//! its request asks it to.
//!
//! Not meant to be run by hand: `tests/stream.rs` starts it through `confined_transport::Host`,
//! as a window starts `pdf-media-worker`, so that `confined_transport::stream`'s flow control,
//! cancellation and budgets are tested against a process the kernel confines and a kill can end —
//! a stall and a kill are not things a byte buffer can show (ADR 3502).
//!
//! Its vocabulary is two kinds. [`PING`] is answered with [`PING`] and `pong`, which is how a test
//! shows the worker kept after a stream. A stream opened with [`COUNT`] carries a mode byte, a
//! count and an item size, each number four bytes big-endian; see [`Mode`].

#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::process::ExitCode;

use confined_transport::stream::{self, Emitted, Emitter};
use confined_transport::{frame, greeting};
use pdf_sandbox::lockdown::{self, Profile};

/// The greeting's magic.
const MAGIC: &[u8; 8] = b"STRMWIT1";
/// A request answered by one reply.
const PING: u8 = 1;
/// A stream's request.
const COUNT: u8 = 2;
/// Largest request this worker reads, in bytes.
const MAX_REQUEST: usize = 64;

/// What a [`COUNT`] stream does.
#[derive(Clone, Copy, Debug)]
enum Mode {
    /// Keeps the contract: `count` items of `size` bytes, then the end.
    Keep,
    /// Ignores its credit: writes `count` items as fast as the pipe takes them.
    OverSend,
    /// Sends nothing, and waits for a frame the host will not send.
    Stall,
    /// Sends two items under its credit, then an item whose carriage no build reads.
    Malformed,
}

fn main() -> ExitCode {
    if let Err(error) = inherited_descriptors::keep_only_the_standard_descriptors() {
        eprintln!("stream-witness: cannot close the descriptors it inherited: {error}");
        return ExitCode::FAILURE;
    }
    match serve() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Standard error is a pipe the host reads.
            eprintln!("stream-witness: {error}");
            ExitCode::FAILURE
        }
    }
}

fn serve() -> Result<(), std::io::Error> {
    let confinement = lockdown::apply_for(Profile::Media).map_err(std::io::Error::other)?;
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    output.write_all(&greeting::encode(MAGIC, confinement))?;
    output.flush()?;
    while let Some((kind, payload)) = read_request(&mut input)? {
        match kind {
            PING => reply(&mut output, PING, b"pong")?,
            stream::OPEN => {
                let opened = stream::decode_open(&payload).map_err(std::io::Error::other)?;
                if opened.kind != COUNT {
                    return Err(std::io::Error::other(
                        "a stream of a kind this worker lacks",
                    ));
                }
                let (mode, count, size) = decode_count(opened.request)?;
                run(mode, count, size, opened.credit, &mut input, &mut output)?;
            }
            other => {
                return Err(std::io::Error::other(format!(
                    "a frame of kind {other}, which this worker does not answer"
                )));
            }
        }
    }
    Ok(())
}

fn run(
    mode: Mode,
    count: u32,
    size: usize,
    credit: std::num::NonZeroU32,
    input: &mut impl Read,
    output: &mut impl Write,
) -> Result<(), std::io::Error> {
    let body = vec![0x5A; size];
    match mode {
        Mode::Keep => {
            let mut emitter = Emitter::new(credit);
            for _ in 0..count {
                if emitter.emit(input, output, &body)? == Emitted::Stopped {
                    return Ok(());
                }
            }
            emitter.finish(input, output, b"done")
        }
        Mode::OverSend => {
            for sequence in 0..count {
                item(output, sequence, 0, &body)?;
            }
            Ok(())
        }
        Mode::Stall => {
            // The host is waiting for an item and this waits for a frame: neither comes.
            stream::read_control(input).map(|_| ())
        }
        Mode::Malformed => {
            for sequence in 0..2 {
                item(output, sequence, 0, &body)?;
            }
            item(output, 2, 9, &body)?;
            stream::read_control(input).map(|_| ())
        }
    }
}

/// An item written by hand, so that a mode can write one the contract forbids.
fn item(
    output: &mut impl Write,
    sequence: u32,
    carriage: u8,
    body: &[u8],
) -> Result<(), std::io::Error> {
    let mut payload = sequence.to_be_bytes().to_vec();
    payload.push(carriage);
    payload.extend_from_slice(body);
    reply(output, stream::ITEM, &payload)
}

fn reply(output: &mut impl Write, kind: u8, payload: &[u8]) -> Result<(), std::io::Error> {
    output.write_all(&frame::header(kind, payload.len()))?;
    output.write_all(payload)?;
    output.flush()
}

fn decode_count(request: &[u8]) -> Result<(Mode, u32, usize), std::io::Error> {
    let number = |at: usize| -> Result<u32, std::io::Error> {
        request
            .get(at..at.saturating_add(4))
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_be_bytes)
            .ok_or_else(|| std::io::Error::other("a count request shorter than nine bytes"))
    };
    let mode = match request.first() {
        Some(0) => Mode::Keep,
        Some(1) => Mode::OverSend,
        Some(2) => Mode::Stall,
        Some(3) => Mode::Malformed,
        _ => return Err(std::io::Error::other("a count request of no known mode")),
    };
    let size = usize::try_from(number(5)?).map_err(std::io::Error::other)?;
    Ok((mode, number(1)?, size))
}

fn read_request(input: &mut impl Read) -> Result<Option<(u8, Vec<u8>)>, std::io::Error> {
    let mut header = [0u8; frame::HEADER_LEN];
    match input.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let Some((kind, length)) = frame::parse_header(header) else {
        return Err(std::io::Error::other(
            "a frame header this build cannot read",
        ));
    };
    if length > MAX_REQUEST {
        return Err(std::io::Error::other(format!(
            "a request of {length} bytes, past the {MAX_REQUEST} this worker reads"
        )));
    }
    let mut payload = vec![0u8; length];
    input.read_exact(&mut payload)?;
    Ok(Some((kind, payload)))
}
