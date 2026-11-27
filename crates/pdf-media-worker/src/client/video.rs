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
use crate::video::{MAX_GROUP_BYTES, MAX_PICTURE_PIXELS, Picture, REORDER_FRAMES};
use crate::wire::{
    FRAME_INTERLACED, FRAME_PICTURES, FRAME_REFUSED, FRAME_VIDEO_MOVIE, FRAME_VIDEO_TRACK,
    MAX_UNITS_PER_RUN, MAX_VIDEO_UNIT_BYTES, PICTURE_HEADER_BYTES, RunEnd, VideoRun, VideoTrack,
    decode_picture, decode_run_end, decode_video_track, encode_movie_request, encode_video_run,
    picture_bytes, sentence,
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
    /// The sentence for a video edit list this reader does not apply, where the track's is one.
    pub note: Option<String>,
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
    /// movie's timeline (ISO/IEC 14496-12:2012, 8.6.1 and 8.6.6): each sample's composition time
    /// through the track's edit list (ADR 3554).
    ///
    /// # Errors
    ///
    /// As [`Self::decode_mp4_video`].
    pub fn decode_mp4_video_timed(
        &mut self,
        file: &[u8],
        deliver: &mut dyn FnMut(Picture, Duration) -> ControlFlow<()>,
    ) -> Result<VideoStreamed, MediaError> {
        self.decode_mp4_video_from(file, Duration::ZERO, deliver)
    }

    /// [`Self::decode_mp4_video_timed`] begun at the last sync sample presented at or before
    /// `from` rather than at the track's start — where a paused clip resumes (ADR 3555). A sync
    /// sample is one "from which decoding may start" without what came before it, so the pictures
    /// from it on are the stream's own; those presented before `from` are still handed on, and the
    /// caller drops them. A `from` of zero decodes the whole track.
    ///
    /// # Errors
    ///
    /// As [`Self::decode_mp4_video`].
    pub fn decode_mp4_video_from(
        &mut self,
        file: &[u8],
        from: Duration,
        deliver: &mut dyn FnMut(Picture, Duration) -> ControlFlow<()>,
    ) -> Result<VideoStreamed, MediaError> {
        let track = self.video_track(file)?;
        let timing = Timing::of(&track);
        let first = timing.start_for(from, &track.sync);
        let mut streamed = VideoStreamed {
            width: track.width,
            height: track.height,
            time_scale: track.time_scale,
            pictures: 0,
            stopped: false,
            note: timing.note.clone(),
        };
        let order = timing.output_order(first);
        let mut output = 0usize;
        let mut timed = |picture: Picture| {
            // A picture past the last unit's is one a reordering decoder let out at the end; it is
            // held at the last time rather than invented one.
            let at = order
                .get(output)
                .or_else(|| order.last())
                .copied()
                .flatten();
            output = output.saturating_add(1);
            match at {
                Some(at) => deliver(picture, at),
                // Composed outside what the edit list presents: decoded, and not shown.
                None => ControlFlow::Continue(()),
            }
        };
        let units = track.units.get(first..).unwrap_or_default();
        let runs = runs(units);
        let count = runs.len();
        for (index, run) in runs.into_iter().enumerate() {
            let cut_units: Vec<&[u8]> = run
                .iter()
                .filter_map(|&(offset, size, _)| cut(file, offset, size))
                .collect();
            if cut_units.len() != run.len() {
                return Err(MediaError::Refused(
                    "an access unit the checked track placed outside the file".to_owned(),
                ));
            }
            let media: u64 = run
                .iter()
                .map(|&(_, _, duration)| u64::from(duration))
                .sum();
            let run = VideoRun::Forward {
                last: index.saturating_add(1) == count,
            };
            if self.run(
                &track,
                &cut_units,
                run,
                cut_units.len(),
                media,
                &mut timed,
                &mut streamed,
            )? == ControlFlow::Break(())
            {
                streamed.stopped = true;
                break;
            }
        }
        Ok(streamed)
    }

    /// [`Self::decode_mp4_video_timed`] played backward — ISO 32000-2 §13.4, Table 307: "If the
    /// value of this entry is negative, the movie shall be played backward with respect to Start
    /// and Duration" (ADR 3837).
    ///
    /// The pass begins at `until` on the movie's timeline — a section's end, or the track's end
    /// where `None` — and runs down. The track is decoded one group of pictures at a time, the
    /// last group first, each from its sync sample: the worker holds a group's pictures and lets
    /// them out last picture first ([`VideoRun::Backward`]). Each picture is handed on with its
    /// time in the backward pass: from `until` down to the picture's end, zero for the picture
    /// showing at `until`, so the times rise as the pictures arrive. A picture presented wholly at
    /// or after `until` is decoded and not handed on.
    ///
    /// `from` is a time in the pass where a paused clip resumes: the groups wholly before it in
    /// the pass are not decoded, and the pictures of the group it falls in that come before it
    /// are still handed on, for the caller to drop. A `from` of zero plays the pass whole.
    ///
    /// # Errors
    ///
    /// As [`Self::decode_mp4_video`], and [`MediaError::Refused`] naming
    /// [`MAX_GROUP_BYTES`] for a group of pictures longer than one backward pass holds — said
    /// before any of the group is decoded where the track's declared size already passes it.
    pub fn decode_mp4_video_backward(
        &mut self,
        file: &[u8],
        until: Option<Duration>,
        from: Duration,
        deliver: &mut dyn FnMut(Picture, Duration) -> ControlFlow<()>,
    ) -> Result<VideoStreamed, MediaError> {
        let track = self.video_track(file)?;
        let timing = Timing::of(&track);
        let spans = timing.spans(&track);
        let top = until.unwrap_or_else(|| {
            spans
                .iter()
                .flatten()
                .map(|&(_, end)| end)
                .max()
                .unwrap_or_default()
        });
        let mut streamed = VideoStreamed {
            width: track.width,
            height: track.height,
            time_scale: track.time_scale,
            pictures: 0,
            stopped: false,
            note: timing.note.clone(),
        };
        for group in groups(&track.sync).into_iter().rev() {
            // Each picture's time in the pass, `None` where it is not presented in it.
            let back = |index: usize| {
                spans
                    .get(index)
                    .copied()
                    .flatten()
                    .filter(|&(start, _)| start < top)
                    .map(|(_, end)| top.saturating_sub(end))
            };
            // A group ending before `from` in the pass, or presenting nothing in it, is not decoded.
            if group.clone().filter_map(back).all(|at| at < from) {
                continue;
            }
            // The worker lets the group out in reverse output order: latest composition first.
            let mut order: Vec<(i128, usize)> = group
                .clone()
                .map(|index| (timing.composition.get(index).copied().unwrap_or(0), index))
                .collect();
            order.sort_unstable_by(|left, right| right.cmp(left));
            let order: Vec<Option<Duration>> =
                order.into_iter().map(|(_, index)| back(index)).collect();
            if self.backward_group(file, &track, group, &order, deliver, &mut streamed)?
                == ControlFlow::Break(())
            {
                streamed.stopped = true;
                break;
            }
        }
        Ok(streamed)
    }

    /// One group of a backward pass: refused by name where its planes would pass
    /// [`MAX_GROUP_BYTES`] at the track's declared size, otherwise sent in runs whose last lets the
    /// group out, each picture handed on at its time in `order` — the times of the group's
    /// pictures latest composition first, as the worker lets them out.
    fn backward_group(
        &mut self,
        file: &[u8],
        track: &VideoTrack,
        group: std::ops::Range<usize>,
        order: &[Option<Duration>],
        deliver: &mut dyn FnMut(Picture, Duration) -> ControlFlow<()>,
        streamed: &mut VideoStreamed,
    ) -> Result<ControlFlow<()>, MediaError> {
        let planes = picture_bytes(usize::from(track.width), usize::from(track.height))
            .saturating_sub(PICTURE_HEADER_BYTES);
        let pictures = group.len();
        if pictures.saturating_mul(planes) > MAX_GROUP_BYTES {
            return Err(MediaError::Refused(format!(
                "a group of {pictures} pictures of {} by {} from sample {}, whose {} bytes of \
                 decoded planes are past the {MAX_GROUP_BYTES} a movie played backward holds",
                track.width,
                track.height,
                group.start.saturating_add(1),
                pictures.saturating_mul(planes)
            )));
        }
        let mut output = 0usize;
        let mut timed = |picture: Picture| {
            let at = order
                .get(output)
                .or_else(|| order.last())
                .copied()
                .flatten();
            output = output.saturating_add(1);
            match at {
                Some(at) => deliver(picture, at),
                None => ControlFlow::Continue(()),
            }
        };
        let units = track.units.get(group).unwrap_or_default();
        let media: u64 = units
            .iter()
            .map(|&(_, _, duration)| u64::from(duration))
            .sum();
        let runs = runs(units);
        let count = runs.len();
        for (index, run) in runs.into_iter().enumerate() {
            let cut_units: Vec<&[u8]> = run
                .iter()
                .filter_map(|&(offset, size, _)| cut(file, offset, size))
                .collect();
            if cut_units.len() != run.len() {
                return Err(MediaError::Refused(
                    "an access unit the checked track placed outside the file".to_owned(),
                ));
            }
            let ends = index.saturating_add(1) == count;
            // The run that ends the group lets out every picture of it, over the group's whole
            // media time; the runs before it decode their own and let out none.
            let (items, wall_media) = if ends {
                (pictures, media)
            } else {
                let own = run
                    .iter()
                    .map(|&(_, _, duration)| u64::from(duration))
                    .sum();
                (0, own)
            };
            let run = VideoRun::Backward { ends };
            if self.run(
                track, &cut_units, run, items, wall_media, &mut timed, streamed,
            )? == ControlFlow::Break(())
            {
                return Ok(ControlFlow::Break(()));
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    /// The movie of `file` sent to the worker, and its video track's checked reply.
    fn video_track(&mut self, file: &[u8]) -> Result<VideoTrack, MediaError> {
        let total = u64::try_from(file.len()).unwrap_or(u64::MAX);
        let movie = self.movie(file, total)?;
        self.ring()?;
        match self.exchange(FRAME_VIDEO_MOVIE, &encode_movie_request(total, movie))? {
            (FRAME_VIDEO_TRACK, bytes) => decode_video_track(&bytes, total).map_err(|wrong| {
                self.distrust();
                MediaError::Refused(wrong)
            }),
            (FRAME_INTERLACED, bytes) => Err(MediaError::Interlaced(sentence(&bytes))),
            (FRAME_REFUSED, bytes) => Err(MediaError::Refused(sentence(&bytes))),
            (kind, _) => {
                self.distrust();
                Err(MediaError::Refused(format!(
                    "a reply of kind {kind}, which no video movie request is answered with"
                )))
            }
        }
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
    #[expect(
        clippy::too_many_arguments,
        reason = "one run's request, its bounds and where its pictures go"
    )]
    fn run(
        &mut self,
        track: &VideoTrack,
        units: &[&[u8]],
        run: VideoRun,
        items: usize,
        media: u64,
        deliver: &mut dyn FnMut(Picture) -> ControlFlow<()>,
        streamed: &mut VideoStreamed,
    ) -> Result<ControlFlow<()>, MediaError> {
        let (width, height) = (usize::from(track.width), usize::from(track.height));
        // A run lets out at most `items` pictures — one a unit forward, a whole group at the end of
        // a backward one — and, at the last, every picture still held.
        let frames = items.saturating_add(REORDER_FRAMES).saturating_add(1);
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
        let request = encode_video_run(run, units);
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

/// When each of a video track's pictures is presented, on the movie's timeline (ISO/IEC
/// 14496-12:2012; ADR 3554).
///
/// - **Decoding time** (8.6.1.2): each sample's is the sum of the durations before it.
/// - **Composition time** (8.6.1.3): "CT(n) = DT(n) + CTTS(n) where CTTS(n) is the (uncompressed)
///   table entry for sample n", and "each sample must have a unique composition timestamp value".
///   A track without `ctts` composes each sample at its decoding time.
/// - **The edit list** (8.6.6) maps composition time onto the movie's: its media time is "the
///   starting time within the media of this edit segment (in media time scale units, in
///   composition time)", and leading empty edits delay the track. Read as a sound track's is
///   (`crate::mp4::edited`): a picture composed before the edit's media time, or past
///   its segment, is decoded and not presented; a list of another shape is not applied, and the
///   note says so.
///
/// **Which picture is which.** The decoder hands its pictures out in output order, the order of
/// their picture order counts, which a conforming stream keeps in the order of their composition
/// times; so the `k`-th picture out of a decode begun at sample `s` is the one with the `k`-th
/// smallest composition time among the samples from `s` on.
struct Timing {
    /// Each sample's composition time, in decoding order, in the media's time scale.
    composition: Vec<i128>,
    /// The edit list's delay, skip and kept span, in the media's time scale.
    delay: i128,
    skip: i128,
    keep: Option<i128>,
    time_scale: u32,
    note: Option<String>,
}

impl Timing {
    fn of(track: &VideoTrack) -> Self {
        let mut decoding = 0i128;
        let composition = track
            .units
            .iter()
            .enumerate()
            .map(|(index, &(_, _, duration))| {
                let offset = track.composition_offsets.get(index).copied().unwrap_or(0);
                let composed = decoding.saturating_add(i128::from(offset));
                decoding = decoding.saturating_add(i128::from(duration));
                composed
            })
            .collect();
        let presented = crate::mp4::edited(
            &track.edits,
            track.movie_time_scale,
            track.time_scale,
            "video",
        );
        Self {
            composition,
            delay: i128::from(presented.delay),
            skip: i128::from(presented.skip),
            keep: presented.keep.map(i128::from),
            time_scale: track.time_scale.max(1),
            note: presented.note,
        }
    }

    /// Where on the movie's timeline a sample composed at `composed` is presented, or `None` where
    /// the edit list presents no part of the media there.
    fn presented(&self, composed: i128) -> Option<Duration> {
        let into = composed.checked_sub(self.skip).filter(|into| *into >= 0)?;
        if self.keep.is_some_and(|keep| into >= keep) {
            return None;
        }
        let ticks = into.saturating_add(self.delay);
        let nanos = ticks
            .saturating_mul(1_000_000_000)
            .checked_div(i128::from(self.time_scale))
            .unwrap_or(0);
        Some(Duration::from_nanos(
            u64::try_from(nanos).unwrap_or(u64::MAX),
        ))
    }

    /// The presentation time of each picture a decode begun at sample `first` hands out, in output
    /// order; `None` for a picture the edit list does not present.
    fn output_order(&self, first: usize) -> Vec<Option<Duration>> {
        let mut composed = self.composition.get(first..).unwrap_or_default().to_vec();
        composed.sort_unstable();
        composed
            .into_iter()
            .map(|composed| self.presented(composed))
            .collect()
    }

    /// Each sample's span on the movie's timeline, in decoding order: where it is presented and
    /// where the next picture in composition order takes its place — its own duration after it
    /// for the last — or `None` where the edit list does not present it.
    fn spans(&self, track: &VideoTrack) -> Vec<Option<(Duration, Duration)>> {
        let mut order: Vec<usize> = (0..self.composition.len()).collect();
        order.sort_unstable_by_key(|&index| self.composition.get(index).copied().unwrap_or(0));
        let mut spans = vec![None; self.composition.len()];
        for (rank, &index) in order.iter().enumerate() {
            let composed = self.composition.get(index).copied().unwrap_or(0);
            let next = order
                .get(rank.saturating_add(1))
                .and_then(|&next| self.composition.get(next).copied())
                .unwrap_or_else(|| {
                    let own = track
                        .units
                        .get(index)
                        .map_or(0, |&(_, _, duration)| duration);
                    composed.saturating_add(i128::from(own))
                });
            if let (Some(start), Some(span)) = (self.presented(composed), spans.get_mut(index)) {
                let length = self.length(next.saturating_sub(composed));
                *span = Some((start, start.saturating_add(length)));
            }
        }
        spans
    }

    /// `ticks` of the media's time scale as a length of time.
    fn length(&self, ticks: i128) -> Duration {
        let nanos = ticks
            .max(0)
            .saturating_mul(1_000_000_000)
            .checked_div(i128::from(self.time_scale))
            .unwrap_or(0);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    /// The sample a decode that shows `from` on begins at: the last sync sample presented at or
    /// before `from`, and the first sample where none is or `from` is zero.
    fn start_for(&self, from: Duration, sync: &[bool]) -> usize {
        if from.is_zero() {
            return 0;
        }
        self.composition
            .iter()
            .enumerate()
            .filter(|&(index, _)| sync.get(index).copied().unwrap_or(true))
            .filter(|&(_, &composed)| self.presented(composed).is_some_and(|at| at <= from))
            .map(|(index, _)| index)
            .next_back()
            .unwrap_or(0)
    }
}

/// A track's groups of pictures, in decoding order: each from a sync sample to the next, the first
/// from the track's first sample whatever it is (ISO/IEC 14496-12:2012, 8.6.2).
fn groups(sync: &[bool]) -> Vec<std::ops::Range<usize>> {
    if sync.is_empty() {
        return Vec::new();
    }
    let starts: Vec<usize> = std::iter::once(0)
        .chain(
            sync.iter()
                .enumerate()
                .filter(|&(index, &flag)| flag && index > 0)
                .map(|(index, _)| index),
        )
        .collect();
    starts
        .iter()
        .zip(starts.iter().skip(1).copied().chain([sync.len()]))
        .map(|(&start, end)| start..end)
        .collect()
}

/// Access units in runs: at most [`MAX_UNITS_PER_RUN`] units and
/// [`MAX_VIDEO_UNIT_BYTES`] bytes a run, in decoding order.
fn runs(units: &[(u64, u32, u32)]) -> Vec<Vec<(u64, u32, u32)>> {
    let mut runs: Vec<Vec<(u64, u32, u32)>> = Vec::new();
    let mut bytes = 0usize;
    for &unit in units {
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
