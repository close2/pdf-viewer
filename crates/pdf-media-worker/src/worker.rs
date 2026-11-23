//! The worker's half: confine, greet, then answer requests until the host closes its end.

use std::io::{Read, Write};

use confined_transport::link::{Link, ReceivedDescriptor, Source as _};
use confined_transport::ring::{self, RingWriter};
use confined_transport::stream::{self, Emitted, Emitter};
use confined_transport::{frame, greeting};
use pdf_sandbox::lockdown::{self, Profile};

use crate::wire::{
    FRAME_BOX, FRAME_BOXED, FRAME_DECODE, FRAME_LAYOUT, FRAME_MOVIE, FRAME_PCM, FRAME_REFUSED,
    FRAME_SAMPLES, FRAME_TRACK, FRAME_UNITS, MAGIC, MAX_LAYOUT_REQUEST_BYTES,
    MAX_MOVIE_REQUEST_BYTES, MAX_REQUEST_BYTES, MAX_SAMPLES_PER_REQUEST, MAX_UNIT_BYTES, Track,
    decode_box_request, decode_layout_request, decode_movie_request, decode_request, decode_units,
    encode_boxed, encode_found, encode_pcm, encode_samples, encode_track,
};
use crate::wire::{
    FRAME_INTERLACED, FRAME_PICTURES, FRAME_VIDEO_MOVIE, FRAME_VIDEO_TRACK, MAX_VIDEO_RUN_BYTES,
    MAX_VIDEO_UNIT_BYTES, RunEnd, VideoTrack, decode_video_run, encode_picture_into,
    encode_run_end, encode_video_track, picture_bytes,
};
use crate::{aac, mp4, sound, video, wave};

/// The worker's standard input as a reader that keeps the descriptors arriving beside its bytes:
/// the ring's, which [`RingWriter::attach`] takes (ADR 3513).
struct Input {
    link: Link,
    arrived: Vec<ReceivedDescriptor>,
}

impl Read for Input {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.link.fill(buffer, &mut self.arrived)?;
        Ok(buffer.len())
    }
}

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
    let mut input = Input {
        link: Link::stdin("pdf-media-worker"),
        arrived: Vec::new(),
    };
    let mut output = std::io::stdout().lock();
    output.write_all(&greeting::encode(MAGIC, confinement))?;
    output.flush()?;
    let mut worker = Worker::default();
    while let Some((kind, payload)) = read_frame(&mut input)? {
        if kind == stream::OPEN {
            worker.stream(payload, &mut input, &mut output)?;
            continue;
        }
        let answered = if kind == ring::RING {
            worker.attach_ring(payload, &mut input.arrived)
        } else {
            worker.answer(kind, payload)
        };
        let (kind, bytes) = match answered {
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
/// movie request to its last run (ADR 3381), the video track's H.264 session (ADR 3526), and the
/// ring its pictures are written into (ADR 3513).
#[derive(Debug, Default)]
pub struct Worker {
    session: Option<aac::Session>,
    video: Option<video::Session>,
    ring: Option<RingWriter>,
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
            FRAME_DECODE | FRAME_LAYOUT | FRAME_BOX | FRAME_MOVIE | FRAME_UNITS | FRAME_VIDEO_MOVIE
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
                let request = decode_layout_request(&payload)?;
                Ok(encode_found(&wave::layout(
                    request.window,
                    request.at,
                    request.total,
                    request.described,
                )?))
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
            FRAME_VIDEO_MOVIE => {
                self.video = None;
                let (total, movie) = decode_movie_request(&payload)?;
                self.video_movie(movie, total)
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

impl Worker {
    /// Reads a movie's video track and starts its H.264 session: the track's reply, or the
    /// interlaced stream's hand-off, which is a reply of its own so the host can say it by name.
    fn video_movie(&mut self, movie: &[u8], total: u64) -> Result<(u8, Vec<u8>), String> {
        let track = mp4::movie_video_track(movie, total)?;
        if let Some(unit) = track.access_units.iter().find(|unit| {
            usize::try_from(unit.size).map_or(true, |size| size > MAX_VIDEO_UNIT_BYTES)
        }) {
            return Err(format!(
                "an access unit of {} bytes, past the {MAX_VIDEO_UNIT_BYTES} one run carries",
                unit.size
            ));
        }
        let pixels = usize::from(track.width).saturating_mul(usize::from(track.height));
        if pixels > video::MAX_PICTURE_PIXELS {
            return Err(format!(
                "a video track of {} by {}, past the {} samples a picture may hold",
                track.width,
                track.height,
                video::MAX_PICTURE_PIXELS
            ));
        }
        let session = match video::Session::for_track(track.length_size, &track.parameter_sets) {
            Ok(session) => session,
            Err(handed @ video::VideoError::Interlaced(_)) => {
                return Ok((FRAME_INTERLACED, handed.to_string().into_bytes()));
            }
            Err(video::VideoError::Refused(sentence)) => return Err(sentence),
        };
        let reply = encode_video_track(&VideoTrack {
            width: track.width,
            height: track.height,
            time_scale: track.time_scale,
            units: track
                .access_units
                .iter()
                .map(|unit| (unit.offset, unit.size, unit.duration))
                .collect(),
        });
        self.video = Some(session);
        Ok((FRAME_VIDEO_TRACK, reply))
    }

    /// Maps the ring the host passed beside a `RING` frame: once, for the worker's life.
    fn attach_ring(
        &mut self,
        payload: Option<Vec<u8>>,
        arrived: &mut Vec<ReceivedDescriptor>,
    ) -> Result<(u8, Vec<u8>), String> {
        if self.ring.is_some() {
            return Err("a second ring, where a worker maps one for its life".to_owned());
        }
        let payload = payload.ok_or_else(|| "a ring's shape past its length".to_owned())?;
        let ring = RingWriter::attach(&payload, arrived)
            .map_err(|error| format!("the ring could not be mapped: {error}"))?;
        self.ring = Some(ring);
        Ok((ring::RING, Vec::new()))
    }

    /// Answers one stream: a [`FRAME_PICTURES`] run decoded, each picture it lets out written into
    /// its slot of the ring under the host's grants, and a closing that says how the run ended.
    ///
    /// A stream the host stops ends here with its end already written; a request this worker cannot
    /// read as a stream at all ends the worker, which its host reads as a death.
    fn stream(
        &mut self,
        payload: Option<Vec<u8>>,
        input: &mut impl Read,
        output: &mut impl Write,
    ) -> Result<(), std::io::Error> {
        let payload = payload.ok_or_else(|| std::io::Error::other("a stream past its length"))?;
        let opened = stream::decode_open(&payload).map_err(std::io::Error::other)?;
        let mut emitter = Emitter::new(opened.credit);
        let end = if opened.kind == FRAME_PICTURES {
            match self.pictures(opened.request, &mut emitter, input, output)? {
                Some(end) => end,
                None => return Ok(()),
            }
        } else {
            RunEnd::Refused(format!(
                "a stream of kind {}, which this worker does not answer",
                opened.kind
            ))
        };
        emitter.finish(input, output, &encode_run_end(&end))
    }

    /// Decodes one run and emits its pictures; `None` where the host stopped the stream.
    fn pictures(
        &mut self,
        request: &[u8],
        emitter: &mut Emitter,
        input: &mut impl Read,
        output: &mut impl Write,
    ) -> Result<Option<RunEnd>, std::io::Error> {
        let (last, units) = match decode_video_run(request) {
            Ok(run) => run,
            Err(sentence) => return Ok(Some(RunEnd::Refused(sentence))),
        };
        let (Some(session), Some(ring)) = (self.video.as_mut(), self.ring.as_mut()) else {
            return Ok(Some(RunEnd::Refused(
                "a video run before a video track's movie, or with no ring attached".to_owned(),
            )));
        };
        let slot = ring.shape().slot_len();
        let mut next = units.iter();
        let mut finished = !last;
        loop {
            let answered = if let Some(unit) = next.next() {
                session.sample(unit)
            } else if finished {
                break;
            } else {
                finished = true;
                Ok(session.finish())
            };
            let pictures = match answered {
                Ok(pictures) => pictures,
                Err(error) => {
                    self.video = None;
                    return Ok(Some(match error {
                        video::VideoError::Interlaced(_) => RunEnd::Interlaced(error.to_string()),
                        video::VideoError::Refused(sentence) => RunEnd::Refused(sentence),
                    }));
                }
            };
            for picture in &pictures {
                let length = picture_bytes(picture.width, picture.height);
                if length > slot {
                    self.video = None;
                    return Ok(Some(RunEnd::Refused(format!(
                        "a picture of {} by {} in {length} bytes, past the ring's slot of {slot}",
                        picture.width, picture.height
                    ))));
                }
                let outcome = emitter.emit_in_ring(input, output, ring, |slot| {
                    encode_picture_into(slot, picture).unwrap_or(usize::MAX)
                })?;
                if outcome == Emitted::Stopped {
                    return Ok(None);
                }
            }
        }
        Ok(Some(RunEnd::Decoded))
    }
}

/// The longest request of `kind` a worker reads: a movie's, or every other's.
fn limit(kind: u8) -> usize {
    if kind == FRAME_MOVIE || kind == FRAME_VIDEO_MOVIE {
        MAX_MOVIE_REQUEST_BYTES
    } else if kind == stream::OPEN {
        // A stream's opening: its kind, its credit and the run, with room for the opening's header.
        MAX_VIDEO_RUN_BYTES.saturating_add(16)
    } else if kind == FRAME_LAYOUT {
        MAX_LAYOUT_REQUEST_BYTES
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
