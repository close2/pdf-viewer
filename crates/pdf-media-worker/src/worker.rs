//! The worker's half: confine, greet, then answer requests until the host closes its end.

use std::io::{Read, Write};

use confined_transport::{frame, greeting};
use pdf_sandbox::lockdown::{self, Profile};

use crate::wire::{
    FRAME_BOX, FRAME_BOXED, FRAME_DECODE, FRAME_LAYOUT, FRAME_MOVIE, FRAME_PCM, FRAME_PLACED,
    FRAME_REFUSED, FRAME_SAMPLES, FRAME_TRACK, FRAME_UNITS, MAGIC, MAX_MOVIE_REQUEST_BYTES,
    MAX_REQUEST_BYTES, MAX_SAMPLES_PER_REQUEST, MAX_UNIT_BYTES, Track, decode_box_request,
    decode_layout_request, decode_movie_request, decode_request, decode_units, encode_boxed,
    encode_pcm, encode_placed, encode_samples, encode_track,
};
use crate::{aac, mp4, sound, wave};

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
    let mut worker = Worker::default();
    while let Some((kind, payload)) = read_frame(&mut input)? {
        let (kind, bytes) = match worker.answer(kind, payload) {
            Ok(answered) => answered,
            Err(refusal) => (FRAME_REFUSED, refusal.into_bytes()),
        };
        output.write_all(&frame::header(kind, bytes.len()))?;
        output.write_all(&bytes)?;
        output.flush()?;
    }
    Ok(())
}

/// One request's reply and its kind, or the sentence it is refused with, answered by a worker that
/// holds no clip.
///
/// Public for the fuzz targets, which run the worker's whole answer over their input.
///
/// # Errors
///
/// The refusal's sentence.
pub fn answer(kind: u8, payload: Option<Vec<u8>>) -> Result<(u8, Vec<u8>), String> {
    Worker::default().answer(kind, payload)
}

/// What a worker holds between requests: the decoder of the clip it is playing, from the clip's
/// movie request to its last run (ADR 3381).
#[derive(Debug, Default)]
pub struct Worker {
    session: Option<aac::Session>,
}

impl Worker {
    /// One request's reply and its kind, or the sentence it is refused with.
    ///
    /// # Errors
    ///
    /// The refusal's sentence.
    pub fn answer(&mut self, kind: u8, payload: Option<Vec<u8>>) -> Result<(u8, Vec<u8>), String> {
        if !matches!(
            kind,
            FRAME_DECODE | FRAME_LAYOUT | FRAME_BOX | FRAME_MOVIE | FRAME_UNITS
        ) {
            return Err(format!(
                "a frame of kind {kind}, which this worker does not answer"
            ));
        }
        let Some(payload) = payload else {
            return Err(format!(
                "a request past the {} bytes this worker reads",
                limit(kind)
            ));
        };
        match kind {
            FRAME_LAYOUT => {
                let (total, head) = decode_layout_request(&payload)?;
                Ok((FRAME_PLACED, encode_placed(&wave::layout(head, total)?)))
            }
            FRAME_BOX => {
                let (total, at, header) = decode_box_request(&payload)?;
                Ok((
                    FRAME_BOXED,
                    encode_boxed(&mp4::top_level_box(header, at, total)?),
                ))
            }
            FRAME_MOVIE => {
                // A new clip ends the last: its decoder goes whether or not this one starts.
                self.session = None;
                let (total, movie) = decode_movie_request(&payload)?;
                let track = mp4::movie_sound_track(movie, total)?;
                if let Some(unit) = track.access_units.iter().find(|unit| {
                    usize::try_from(unit.size).map_or(true, |size| size > MAX_UNIT_BYTES)
                }) {
                    return Err(format!(
                        "an access unit of {} bytes, past the {MAX_UNIT_BYTES} one run carries",
                        unit.size
                    ));
                }
                let session = aac::Session::new(track.object_type, &track.decoder_config)?;
                let reply = encode_track(&Track {
                    time_scale: track.time_scale,
                    presentation: mp4::presentation(&track),
                    units: track
                        .access_units
                        .iter()
                        .map(|unit| (unit.offset, unit.size))
                        .collect(),
                });
                self.session = Some(session);
                Ok((FRAME_TRACK, reply))
            }
            FRAME_UNITS => {
                let units = decode_units(&payload)?;
                let session = self
                    .session
                    .as_mut()
                    .ok_or_else(|| "a run of access units before any clip's movie".to_owned())?;
                let pcm = session.decode(&units, MAX_SAMPLES_PER_REQUEST)?;
                Ok((FRAME_PCM, encode_pcm(&pcm)))
            }
            _ => {
                let (format, run) = decode_request(&payload)?;
                let decoded = sound::decode(&format, run, MAX_SAMPLES_PER_REQUEST)?;
                Ok((FRAME_SAMPLES, encode_samples(&decoded)))
            }
        }
    }
}

/// The longest request of `kind` a worker reads: a movie's, or every other's.
fn limit(kind: u8) -> usize {
    if kind == FRAME_MOVIE {
        MAX_MOVIE_REQUEST_BYTES
    } else {
        MAX_REQUEST_BYTES
    }
}

/// A frame's kind, and its payload or `None` where it was past its kind's [`limit`].
type Frame = (u8, Option<Vec<u8>>);

/// Reads one frame — its kind, and its payload or `None` where the payload was past its kind's
/// [`limit`] and was read past rather than held — or `None` at the end of input.
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
    if length > limit(kind) {
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
