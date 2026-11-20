//! The worker's half: confine, greet, then answer requests until the host closes its end.

use std::io::{Read, Write};

use confined_transport::{frame, greeting};
use pdf_sandbox::lockdown::{self, Profile};

use crate::wire::{
    FRAME_DECODE, FRAME_LAYOUT, FRAME_PLACED, FRAME_REFUSED, FRAME_SAMPLES, MAGIC,
    MAX_REQUEST_BYTES, MAX_SAMPLES_PER_REQUEST, decode_layout_request, decode_request,
    encode_placed, encode_samples,
};
use crate::{sound, wave};

/// Confines this process with [`Profile::Media`], writes the greeting, then decodes each request
/// read from standard input and writes its reply to standard output.
///
/// The confinement comes first: nothing is read before it is in place, and a worker that cannot
/// confine itself writes no greeting, which its host reads as a death rather than as a worker that
/// runs unconfined.
///
/// # Errors
///
/// The confinement's refusal, and a pipe that fails.
pub fn serve() -> Result<(), std::io::Error> {
    let confinement = lockdown::apply_for(Profile::Media).map_err(std::io::Error::other)?;
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    output.write_all(&greeting::encode(MAGIC, confinement))?;
    output.flush()?;
    while let Some((kind, payload)) = read_frame(&mut input)? {
        let (kind, bytes) = match answer(kind, payload) {
            Ok(answered) => answered,
            Err(refusal) => (FRAME_REFUSED, refusal.into_bytes()),
        };
        output.write_all(&frame::header(kind, bytes.len()))?;
        output.write_all(&bytes)?;
        output.flush()?;
    }
    Ok(())
}

/// One request's reply and its kind, or the sentence it is refused with.
///
/// Public for the fuzz target, which runs the worker's whole answer over its input.
///
/// # Errors
///
/// The refusal's sentence.
pub fn answer(kind: u8, payload: Option<Vec<u8>>) -> Result<(u8, Vec<u8>), String> {
    if kind != FRAME_DECODE && kind != FRAME_LAYOUT {
        return Err(format!(
            "a frame of kind {kind}, which this worker does not answer"
        ));
    }
    let Some(payload) = payload else {
        return Err(format!(
            "a request past the {MAX_REQUEST_BYTES} bytes this worker reads"
        ));
    };
    if kind == FRAME_LAYOUT {
        let (total, head) = decode_layout_request(&payload)?;
        return Ok((FRAME_PLACED, encode_placed(&wave::layout(head, total)?)));
    }
    let (format, run) = decode_request(&payload)?;
    let decoded = sound::decode(&format, run, MAX_SAMPLES_PER_REQUEST)?;
    Ok((FRAME_SAMPLES, encode_samples(&decoded)))
}

/// A frame's kind, and its payload or `None` where it was past [`MAX_REQUEST_BYTES`].
type Frame = (u8, Option<Vec<u8>>);

/// Reads one frame — its kind, and its payload or `None` where the payload was past
/// [`MAX_REQUEST_BYTES`] and was read past rather than held — or `None` at the end of input.
///
/// The length is checked before anything is allocated from it: it is the host's claim, and a
/// worker whose host has been taken over is the case the bound is for.
fn read_frame(input: &mut impl Read) -> Result<Option<Frame>, std::io::Error> {
    let mut header = [0u8; frame::HEADER_LEN];
    match input.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let Some((kind, length)) = frame::parse_header(header) else {
        return Err(std::io::Error::other(
            "a frame whose length this build does not read",
        ));
    };
    let wanted = u64::try_from(length).unwrap_or(u64::MAX);
    if length > MAX_REQUEST_BYTES {
        let skipped = std::io::copy(&mut input.take(wanted), &mut std::io::sink())?;
        if skipped < wanted {
            return Ok(None);
        }
        return Ok(Some((kind, None)));
    }
    let mut payload = vec![0u8; length];
    input.read_exact(&mut payload)?;
    Ok(Some((kind, Some(payload))))
}
