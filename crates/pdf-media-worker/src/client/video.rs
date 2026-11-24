//! The host's half of a video track: the movie sent once, then the access units in runs, each run a
//! stream whose pictures arrive in the ring's slots (RFC 0015 sections 4.3 and 6, ADR 3527).
//!
//! The host reads no byte of the file as structure here either: it cuts the movie and each access
//! unit where the worker's checked answers say (ADR 3381), and it checks every picture against the
//! track's declared size before handing it on, as a compromised worker's output would have to be.

use std::num::NonZeroU32;
use std::ops::ControlFlow;
use std::time::Duration;

use confined_transport::ring::Shape;
use confined_transport::stream::{Budget, Spent, StreamError};

use super::{MediaError, MediaWorker, PROCESSOR_BUDGET};
use crate::video::{MAX_PICTURE_PIXELS, Picture, REORDER_FRAMES};
use crate::wire::{
    FRAME_INTERLACED, FRAME_PICTURES, FRAME_REFUSED, FRAME_VIDEO_MOVIE, FRAME_VIDEO_TRACK,
    MAX_UNITS_PER_RUN, MAX_VIDEO_UNIT_BYTES, RunEnd, VideoTrack, decode_picture, decode_run_end,
    decode_video_track, encode_movie_request, encode_video_run, picture_bytes, sentence,
};

/// The slots of a video worker's ring: enough that the worker decodes the next picture while the
/// host copies the last out, and few enough that a 1080-line ring is twelve mebibytes (ADR 3527).
pub const RING_SLOTS: u32 = 4;

/// The longest side of [`MAX_PICTURE_PIXELS`]'s frame, which every ring's slot is sized for, so one
/// ring a worker maps for its life holds every clip it will be sent.
const RING_PICTURE: (usize, usize) = (1920, 1088);
const _: () = assert!(RING_PICTURE.0 * RING_PICTURE.1 == MAX_PICTURE_PIXELS);

/// What a video track's decode did, besides the pictures it handed on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoStreamed {
    /// The track's declared picture size, the bound every picture was checked against.
    pub width: u16,
    /// The track's declared height.
    pub height: u16,
    /// The media's time scale, the clock of every picture's duration.
    pub time_scale: u32,
    /// How many pictures were handed on.
    pub pictures: usize,
    /// Whether `deliver` stopped the track before its end.
    pub stopped: bool,
}

impl MediaWorker {
    /// Decodes the first enabled H.264 video track of the MP4 file `file` in the confined worker,
    /// handing each picture to `deliver` in output order as it arrives.
    ///
    /// # Errors
    ///
    /// [`MediaError::Interlaced`] for field or MBAFF coding, which is the caller's to hand to this
    /// machine's own player and say so; [`MediaError::Refused`] for a file, a track or a stream the
    /// worker refuses, or an answer it could not have given; and the ring's, the stream's and the
    /// transport's failures.
    pub fn decode_mp4_video(
        &mut self,
        file: &[u8],
        deliver: &mut dyn FnMut(Picture) -> ControlFlow<()>,
    ) -> Result<VideoStreamed, MediaError> {
        self.decode_mp4_video_timed(file, &mut |picture, _| deliver(picture))
    }

    /// [`Self::decode_mp4_video`], each picture handed on with its presentation time on the
    /// track's own clock: the `k`-th picture in output order at the `k`-th sample's decoding time,
    /// the sum of the durations of the samples before it (ISO/IEC 14496-12, 8.6.1.2).
    ///
    /// That is the presentation time exactly where a track states no composition offsets
    /// (`ctts`, 8.6.1.3), whose pictures are presented at their decoding times, and where the
    /// offsets only reorder pictures of one duration; this reader reads no `ctts` and no video
    /// edit list (8.6.6), and a window timing frames by it says so (ADR 3538).
    ///
    /// # Errors
    ///
    /// As [`Self::decode_mp4_video`].
    pub fn decode_mp4_video_timed(
        &mut self,
        file: &[u8],
        deliver: &mut dyn FnMut(Picture, Duration) -> ControlFlow<()>,
    ) -> Result<VideoStreamed, MediaError> {
        let total = u64::try_from(file.len()).unwrap_or(u64::MAX);
        let movie = self.movie(file, total)?;
        self.ring()?;
        let track = match self.exchange(FRAME_VIDEO_MOVIE, &encode_movie_request(total, movie))? {
            (FRAME_VIDEO_TRACK, bytes) => decode_video_track(&bytes, total).map_err(|wrong| {
                self.distrust();
                MediaError::Refused(wrong)
            })?,
            (FRAME_INTERLACED, bytes) => return Err(MediaError::Interlaced(sentence(&bytes))),
            (FRAME_REFUSED, bytes) => return Err(MediaError::Refused(sentence(&bytes))),
            (kind, _) => {
                self.distrust();
                return Err(MediaError::Refused(format!(
                    "a reply of kind {kind}, which no video movie request is answered with"
                )));
            }
        };
        let mut streamed = VideoStreamed {
            width: track.width,
            height: track.height,
            time_scale: track.time_scale,
            pictures: 0,
            stopped: false,
        };
        let starts = decoding_times(&track);
        let mut output = 0usize;
        let mut timed = |picture: Picture| {
            // A picture past the last sample's start is one a reordering decoder let out at the
            // end; it is held at the last start rather than invented a time.
            let at = starts
                .get(output)
                .or_else(|| starts.last())
                .copied()
                .unwrap_or_default();
            output = output.saturating_add(1);
            deliver(picture, at)
        };
        let runs = runs(&track);
        let count = runs.len();
        for (index, run) in runs.into_iter().enumerate() {
            let units: Vec<&[u8]> = run
                .iter()
                .filter_map(|&(offset, size, _)| cut(file, offset, size))
                .collect();
            if units.len() != run.len() {
                return Err(MediaError::Refused(
                    "an access unit the checked track placed outside the file".to_owned(),
                ));
            }
            let media: u64 = run
                .iter()
                .map(|&(_, _, duration)| u64::from(duration))
                .sum();
            let last = index.saturating_add(1) == count;
            if self.run(&track, &units, last, media, &mut timed, &mut streamed)?
                == ControlFlow::Break(())
            {
                streamed.stopped = true;
                break;
            }
        }
        Ok(streamed)
    }

    /// The worker's ring, attached once for its life, sized for the largest picture it outputs.
    fn ring(&mut self) -> Result<(), MediaError> {
        if self
            .host
            .as_ref()
            .is_some_and(|host| host.ring_shape().is_some())
        {
            return Ok(());
        }
        // The ring is attached before the worker has answered anything else of this clip; a worker
        // that is not running yet is started by an exchange the attachment needs anyway.
        if self.host.is_none() {
            self.start()?;
        }
        let slot = picture_bytes(RING_PICTURE.0, RING_PICTURE.1);
        let shape = Shape::new(slot, RING_SLOTS, u64::MAX)?;
        let host = self
            .host
            .as_mut()
            .ok_or_else(|| MediaError::Start("the worker ended before its ring".to_owned()))?;
        if let Err(error) = host.attach_ring(shape) {
            self.distrust();
            return Err(error.into());
        }
        Ok(())
    }

    /// One run: a stream of its pictures, each checked and handed on, and its closing read.
    fn run(
        &mut self,
        track: &VideoTrack,
        units: &[&[u8]],
        last: bool,
        media: u64,
        deliver: &mut dyn FnMut(Picture) -> ControlFlow<()>,
        streamed: &mut VideoStreamed,
    ) -> Result<ControlFlow<()>, MediaError> {
        let (width, height) = (usize::from(track.width), usize::from(track.height));
        // A run lets out at most one picture a unit and, at the last, every picture still held.
        let frames = units.len().saturating_add(REORDER_FRAMES).saturating_add(1);
        let budget = Budget {
            frames: u32::try_from(frames).unwrap_or(u32::MAX),
            bytes: u64::try_from(frames.saturating_mul(picture_bytes(width, height)))
                .unwrap_or(u64::MAX),
            wall: wall(media, track.time_scale),
        };
        let host = self
            .host
            .as_mut()
            .ok_or_else(|| MediaError::Start("the worker ended before its run".to_owned()))?;
        let window = host
            .ring_shape()
            .map_or(NonZeroU32::MIN, |shape| shape.slots());
        let request = encode_video_run(last, units);
        let outcome = Self::stream_run(
            host, &request, window, budget, width, height, deliver, streamed,
        );
        match outcome {
            Ok(flow) => Ok(flow),
            Err(Failed::Stream(error)) => {
                self.distrust();
                Err(match error {
                    StreamError::Spent(Spent::Wall) => MediaError::Stalled { after: budget.wall },
                    other => MediaError::Stream(other),
                })
            }
            Err(Failed::Distrusted(wrong)) => {
                self.distrust();
                Err(MediaError::Refused(wrong))
            }
            Err(Failed::Said(error)) => Err(error),
        }
    }

    /// One run's stream on `host`: each picture checked against the track's size and handed on, then
    /// the closing read.
    #[expect(
        clippy::too_many_arguments,
        reason = "one run's whole state, split from `run` only so the stream's borrow of the host \
                  ends before the worker is distrusted"
    )]
    fn stream_run(
        host: &mut confined_transport::Host,
        request: &[u8],
        window: NonZeroU32,
        budget: Budget,
        width: usize,
        height: usize,
        deliver: &mut dyn FnMut(Picture) -> ControlFlow<()>,
        streamed: &mut VideoStreamed,
    ) -> Result<ControlFlow<()>, Failed> {
        let mut stream = host
            .stream(FRAME_PICTURES, request, window, budget)
            .map_err(Failed::Stream)?;
        while let Some(item) = stream.next_item().map_err(Failed::Stream)? {
            let picture = decode_picture(&item, width, height).map_err(Failed::Distrusted)?;
            streamed.pictures = streamed.pictures.saturating_add(1);
            if deliver(picture) == ControlFlow::Break(()) {
                stream.stop().map_err(Failed::Stream)?;
                return Ok(ControlFlow::Break(()));
            }
        }
        match decode_run_end(stream.closing().unwrap_or_default()) {
            Ok(RunEnd::Decoded) => Ok(ControlFlow::Continue(())),
            Ok(RunEnd::Interlaced(sentence)) => Err(Failed::Said(MediaError::Interlaced(sentence))),
            Ok(RunEnd::Refused(sentence)) => Err(Failed::Said(MediaError::Refused(sentence))),
            Err(wrong) => Err(Failed::Distrusted(wrong)),
        }
    }
}

/// How a run's stream failed, so that the worker is distrusted after the stream's borrow ends.
enum Failed {
    /// The stream broke or spent its budget.
    Stream(StreamError),
    /// The worker sent what no run could hold.
    Distrusted(String),
    /// The worker said why it stopped, and stays.
    Said(MediaError),
}

/// Each sample's decoding time in decoding order, on the track's clock: the sum of the durations
/// before it (ISO/IEC 14496-12, 8.6.1.2's `stts`, which the worker's checked answer carries).
fn decoding_times(track: &VideoTrack) -> Vec<Duration> {
    let scale = u128::from(track.time_scale.max(1));
    let mut ticks = 0u128;
    track
        .units
        .iter()
        .map(|&(_, _, duration)| {
            let nanos = ticks
                .saturating_mul(1_000_000_000)
                .checked_div(scale)
                .unwrap_or(0);
            ticks = ticks.saturating_add(u128::from(duration));
            Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
        })
        .collect()
}

/// The track's access units in runs: at most [`MAX_UNITS_PER_RUN`] units and
/// [`MAX_VIDEO_UNIT_BYTES`] bytes a run, in decoding order.
fn runs(track: &VideoTrack) -> Vec<Vec<(u64, u32, u32)>> {
    let mut runs: Vec<Vec<(u64, u32, u32)>> = Vec::new();
    let mut bytes = 0usize;
    for &unit in &track.units {
        let size = usize::try_from(unit.1).unwrap_or(usize::MAX);
        let fits = runs.last().is_some_and(|run| {
            run.len() < MAX_UNITS_PER_RUN && bytes.saturating_add(size) <= MAX_VIDEO_UNIT_BYTES
        });
        if fits {
            bytes = bytes.saturating_add(size);
            if let Some(run) = runs.last_mut() {
                run.push(unit);
            }
        } else {
            bytes = size;
            runs.push(vec![unit]);
        }
    }
    runs
}

/// The bytes of `file` the worker's checked answer places at `offset`.
fn cut(file: &[u8], offset: u64, size: u32) -> Option<&[u8]> {
    let start = usize::try_from(offset).ok()?;
    file.get(start..start.checked_add(usize::try_from(size).ok()?)?)
}

/// A run's wall clock: the processor budget every request has, and the run's own media time
/// besides, so a decoder that falls behind real time by more than the budget is stopped and the
/// clip handed off (RFC 0015 section 4.1's deadline, ADR 3527).
fn wall(media: u64, time_scale: u32) -> Duration {
    let scale = u64::from(time_scale.max(1));
    let seconds = media.checked_div(scale).unwrap_or(0);
    let rest = media.checked_rem(scale).unwrap_or(0);
    let nanos = rest
        .saturating_mul(1_000_000_000)
        .checked_div(scale)
        .unwrap_or(0);
    PROCESSOR_BUDGET
        .saturating_add(Duration::from_secs(seconds))
        .saturating_add(Duration::from_nanos(nanos))
}
