//! The sound path measured in use: a window's [`Player`], the confined media worker and the sound
//! helper, playing onto a device whose sample clock this file reads frame by frame (RFC 0015
//! section 9's audio rows, A946's condition for video, ADR 3441).
//!
//! **The device is a stand-in, and it is said so.** This machine's user may open no sound card, so
//! the helper is given ALSA's `file` plugin over its `null` plugin, writing into a named pipe this
//! file reads at the device's rate on the monotonic clock: the pipe is the device's buffer, shrunk
//! to one page, and the reader is its sample clock — it takes a millisecond's frames when they are
//! due, and a writer that has not supplied them is an underrun. What that measures is everything
//! this program does: the player's thread, the worker's decoding, the conversion, the helper, its
//! `cpal` stream and ALSA's plugins up to the last byte handed to a device. What it does not is a
//! speaker: a card's own buffer and converter, and the drift of its crystal against the processor's,
//! which a clock derived from the monotonic clock cannot have.
//!
//! **The samples say where they are.** The measured sounds are 48 kHz stereo, the device's own
//! format, so the conversion is the identity, and each frame carries its own index — the low
//! sixteen bits in the left channel, the rest plus one in the right — so the reader knows which
//! frame of the sound the device is taking at every instant: a pause's resumption, a lost or
//! repeated frame and the media clock's error are read off the frames, not inferred.
//!
//! **Or the device is ALSA's loopback card** (`AUDIO_IN_USE_ROUTE=loopback`, ADR 3459): the helper
//! plays into `hw:Loopback,0,0` and `ffmpeg` captures `hw:Loopback,1,0` for this file to read. That
//! is a kernel device with its own timer, period and buffer, which the driver enforces, so it
//! measures what the pipe cannot — a driver's buffer and its period wakeups between the helper and
//! the frames taken. It still is no card: no converter, no crystal (the loopback's timer runs on the
//! machine's own clock), no speaker; and its frames reach this file a capture period late, so
//! every time read from it is late by up to a period (about 12 ms here), and an underrun is a
//! stretch of silence inside a sound whose frames go on in order, not a read that waited.
//!
//! `tools/audio-in-use.sh` builds the helper and the worker in the `gates` profile, makes or fetches
//! the AAC clips and runs this file; the variables it sets are named in [`Setup::from_environment`].
// not a gate: it needs a sound device this file builds from ALSA plugins, the gates-profile helper
// and worker, AAC clips fetched from ISO's Publicly Available Standards and ten minutes and more of
// wall time; it prints readings a person compares with RFC 0015's budgets, and asserts only that
// every sound reached the device.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::print_stdout,
    reason = "an instrument a person runs: a missing setup fails loudly, its counts are frames of a \
              sound of minutes, and its readings are printed"
)]

use std::fmt::Write as _;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use pdf_model::media::{Encoding, Playing, SampleFormat};
use pdf_syntax::ObjectId;
use viewer_core::{Associated, DocumentId, MediaOperation, Recording, SoundData};
use viewer_host::sound::{Heard, Played, Player};
use viewer_host::video::{Seen, Shown, VideoPlayer};

/// The device's rate and channel count, which every measured sound is made at.
const RATE: u32 = 48_000;
const CHANNELS: u16 = 2;
/// One frame on the pipe: two 32-bit floats, ALSA's `FLOAT_LE`.
const FRAME_BYTES: usize = 8;
/// What the device takes at a time: a millisecond's frames.
const CHUNK_FRAMES: usize = 48;
/// The pipe's capacity, the device's buffer: one page, 512 frames, 10.7 ms.
const PIPE_BYTES: usize = 4096;
/// A read that waited longer than this for frames that were due is an underrun.
const STARVED: Duration = Duration::from_millis(2);

const DOCUMENT: DocumentId = DocumentId(1);
const SCREEN: ObjectId = ObjectId {
    number: 4,
    generation: 0,
};

/// What `tools/audio-in-use.sh` hands this file.
struct Setup {
    /// `AUDIO_IN_USE_HELPER`: the sound helper program.
    helper: PathBuf,
    /// `AUDIO_IN_USE_WORKER`: the media worker program.
    worker: PathBuf,
    /// `AUDIO_IN_USE_DIR`: where the pipe, the ALSA configuration and the helper's wrapper go.
    directory: PathBuf,
    /// `AUDIO_IN_USE_CLIPS`: `name=path` pairs, `;`-separated, of MP4 clips of sound alone.
    clips: Vec<(String, PathBuf)>,
    /// `AUDIO_IN_USE_LONG_SECONDS`: the long sound's length; it is played four times over.
    long_seconds: u32,
    /// `AUDIO_IN_USE_ROUTE`: `pipe` (the default) or `loopback`.
    loopback: bool,
}

impl Setup {
    fn from_environment() -> Self {
        let path = |name: &str| {
            PathBuf::from(
                std::env::var(name)
                    .unwrap_or_else(|_| panic!("{name} is unset: run tools/audio-in-use.sh")),
            )
        };
        let clips = std::env::var("AUDIO_IN_USE_CLIPS")
            .unwrap_or_default()
            .split(';')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let (name, file) = pair.split_once('=').expect("a clip is name=path");
                (name.to_owned(), PathBuf::from(file))
            })
            .collect();
        let long_seconds = std::env::var("AUDIO_IN_USE_LONG_SECONDS")
            .ok()
            .map_or(160, |seconds| {
                seconds.parse().expect("a whole number of seconds")
            });
        Self {
            helper: path("AUDIO_IN_USE_HELPER"),
            worker: path("AUDIO_IN_USE_WORKER"),
            directory: path("AUDIO_IN_USE_DIR"),
            clips,
            long_seconds,
            loopback: match std::env::var("AUDIO_IN_USE_ROUTE").as_deref() {
                Ok("loopback") => true,
                Ok("pipe") | Err(_) => false,
                Ok(other) => panic!("AUDIO_IN_USE_ROUTE is pipe or loopback, not {other}"),
            },
        }
    }
}

/// What the device took in one read.
#[derive(Clone, Copy, Debug)]
struct Taken {
    /// When the read returned.
    at: Instant,
    /// The stream it belongs to: one per time a writer opened the pipe.
    stream: u32,
    /// How many frames it took, and how many of them were not silent.
    frames: usize,
    sounding: usize,
    /// The index the first and the last sounding frame carried, where the samples carry one.
    first: Option<u64>,
    last: Option<u64>,
    /// The first frame of the read louder than [`LOUD`], counted from the read's first: where a
    /// beep begins (ADR 3556).
    loud: Option<usize>,
}

/// A sample past this, either channel, is a beep's and not the silence between beeps or a
/// coder's noise around it.
const LOUD: f32 = 0.25;

/// Everything the device saw.
#[derive(Default)]
struct DeviceLog {
    taken: Vec<Taken>,
    /// Each underrun: when, and how long the read waited past its frames' due time.
    underruns: Vec<(Instant, Duration)>,
    /// Each read that began more than [`STARVED`] after its frames were due, on the pipe: when,
    /// and how late — the reader's own lateness, which [`Self::underruns`] does not count.
    late: Vec<(Instant, Duration)>,
    /// Frames whose index was not the one before plus one, inside a stream: where, from, to.
    jumps: Vec<(Instant, u64, u64)>,
    /// Each stream's frames and the span they were taken over, for the clock's own rate.
    streams: Vec<(u64, Duration)>,
    /// Each phase of the walk, from when it began, so that an underrun is told by the phase.
    phases: Vec<(&'static str, Instant)>,
}

/// The stand-in device: the pipe, its reader's thread and what it saw.
struct Device {
    log: Arc<Mutex<DeviceLog>>,
    done: Arc<AtomicBool>,
    /// Whether the sound now playing carries its frames' indices, so a jump in them is a fault.
    indexed: Arc<AtomicBool>,
    reader: Option<std::thread::JoinHandle<()>>,
    /// The wrapper that starts the helper with this device as its default.
    wrapper: PathBuf,
    /// The loopback's capture, where the device is the loopback card.
    capture: Option<std::process::Child>,
}

impl Device {
    fn open(setup: &Setup) -> Self {
        let directory = &setup.directory;
        std::fs::create_dir_all(directory).expect("the device's directory");
        let (pipe, capture) = if setup.loopback {
            Self::loopback(directory)
        } else {
            (Self::pipe(directory), None)
        };
        let wrapper = directory.join("quorra-sound");
        std::fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nexec env HOME='{}' '{}' \"$@\"\n",
                directory.display(),
                setup.helper.display()
            ),
        )
        .expect("the helper's wrapper");
        std::fs::set_permissions(
            &wrapper,
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .expect("the wrapper made executable");
        let paced = !setup.loopback;
        let log = Arc::new(Mutex::new(DeviceLog::default()));
        let done = Arc::new(AtomicBool::new(false));
        let indexed = Arc::new(AtomicBool::new(true));
        let reader = {
            let log = Arc::clone(&log);
            let done = Arc::clone(&done);
            let indexed = Arc::clone(&indexed);
            std::thread::Builder::new()
                .name("device".to_owned())
                .spawn(move || take(pipe, &log, &done, &indexed, paced))
                .expect("the device's thread")
        };
        Self {
            log,
            done,
            indexed,
            reader: Some(reader),
            wrapper,
            capture,
        }
    }

    /// 2185's stand-in: the helper's default is ALSA's `file` plugin over `null`, writing into a
    /// named pipe of one page that this file reads at the device's rate.
    ///
    /// The pipe's node is made on a memory file system, never beside the readings: a write into a
    /// named pipe updates its node's times, and on a journalled disk file system the writer then
    /// waits for the journal's commit — every five seconds, for 20 to 520 ms on this machine's
    /// scratch disk, which was every underrun the pipe showed before (ADR 3465).
    fn pipe(directory: &std::path::Path) -> std::fs::File {
        let fifo = fifo_path();
        let _ = std::fs::remove_file(&fifo);
        rustix::fs::mknodat(
            rustix::fs::CWD,
            &fifo,
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::from_raw_mode(0o600),
            0,
        )
        .expect("the device's pipe");
        // The helper's own ALSA configuration: `plug` makes whatever `cpal` writes 48 kHz stereo
        // floats, `file` writes them to the pipe, and `null` under it is a device that takes
        // anything at once — so the pipe's reader is the only clock there is.
        std::fs::write(
            directory.join(".asoundrc"),
            format!(
                "pcm.!default {{ type plug; slave {{ pcm \"measured\"; format FLOAT_LE; \
                 rate {RATE}; channels {CHANNELS} }} }}\n\
                 pcm.measured {{ type file; slave.pcm \"null\"; file \"{}\"; format \"raw\" }}\n",
                fifo.display()
            ),
        )
        .expect("the device's configuration");
        // Opened without waiting for a writer, shrunk before any writes, then made to wait.
        let fd = rustix::fs::open(
            &fifo,
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NONBLOCK | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .expect("the pipe's reading end");
        let size = rustix::pipe::fcntl_setpipe_size(&fd, PIPE_BYTES).expect("the pipe shrunk");
        assert_eq!(size, PIPE_BYTES, "the pipe is one page");
        rustix::fs::fcntl_setfl(&fd, rustix::fs::OFlags::empty()).expect("the pipe made to wait");
        std::fs::File::from(fd)
    }

    /// ALSA's loopback card (ADR 3459): the helper's default is `hw:Loopback,0,0` as 48 kHz
    /// stereo `S16_LE` — the format the capture opens first, which the loopback then holds the
    /// playback side to — and `ffmpeg` captures `hw:Loopback,1,0` as floats for this file to read.
    /// The indices survive: a frame's sixteen-bit halves are exact in `S16_LE`.
    fn loopback(directory: &std::path::Path) -> (std::fs::File, Option<std::process::Child>) {
        std::fs::write(
            directory.join(".asoundrc"),
            format!(
                "pcm.!default {{ type plug; slave {{ pcm \"hw:Loopback,0,0\"; format S16_LE; \
                 rate {RATE}; channels {CHANNELS} }} }}\n"
            ),
        )
        .expect("the device's configuration");
        let mut child = std::process::Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-f", "alsa"])
            .args([
                "-sample_rate",
                &RATE.to_string(),
                "-channels",
                &CHANNELS.to_string(),
            ])
            .args([
                "-i",
                "hw:Loopback,1,0",
                "-f",
                "f32le",
                "-flush_packets",
                "1",
                "pipe:1",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("ffmpeg captures the loopback (run under `sg audio`)");
        let out = child.stdout.take().expect("the capture's output");
        (
            std::fs::File::from(std::os::fd::OwnedFd::from(out)),
            Some(child),
        )
    }

    /// A player on this device and the gates-profile worker.
    fn player(&self, setup: &Setup) -> Player {
        let wrapper = self.wrapper.clone();
        Player::new(
            Box::new(move || viewer_host::sound::helper::open_at(&wrapper)),
            Box::new(|| {}),
        )
        .with_worker(setup.worker.clone())
    }

    /// A phase of the walk begins, playing sounds that carry their indices or not.
    fn phase(&self, name: &'static str, indexed: bool) {
        self.indexed.store(indexed, Ordering::Relaxed);
        self.log().phases.push((name, Instant::now()));
    }

    fn log(&self) -> std::sync::MutexGuard<'_, DeviceLog> {
        self.log.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The first read after `since` that took a sounding frame, waiting up to `limit` for one.
    fn first_sound_after(&self, since: Instant, limit: Duration) -> Option<Taken> {
        let deadline = since + limit;
        loop {
            if let Some(found) = self
                .log()
                .taken
                .iter()
                .rev()
                .take_while(|taken| taken.at >= since)
                .filter(|taken| taken.sounding > 0)
                .last()
                .copied()
            {
                return Some(found);
            }
            if Instant::now() > deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// The last read in `[from, to)` that took a sounding frame.
    fn last_sound_between(&self, from: Instant, to: Instant) -> Option<Taken> {
        self.log()
            .taken
            .iter()
            .rev()
            .find(|taken| taken.at >= from && taken.at < to && taken.sounding > 0)
            .copied()
    }

    fn close(mut self) -> DeviceLog {
        self.done.store(true, Ordering::Relaxed);
        // The capture's end ends the reader's last read.
        if let Some(mut capture) = self.capture.take() {
            let _ = capture.kill();
            let _ = capture.wait();
        }
        if let Some(reader) = self.reader.take() {
            reader.join().expect("the device's thread");
        }
        // The pipe's node, where this device made one; the loopback made none.
        let _ = std::fs::remove_file(fifo_path());
        std::mem::take(&mut *self.log())
    }
}

/// The device's thread: a millisecond's frames when they are due, for as long as a writer has the
/// pipe open, and a look each millisecond for the next writer while none has.
///
/// Where the device is the loopback (`paced` false), its frames come when its own clock gives
/// them, so nothing is waited for here, and an underrun is silence between two frames whose
/// indices follow one another, read off the frames.
fn take(
    mut pipe: std::fs::File,
    log: &Mutex<DeviceLog>,
    done: &AtomicBool,
    indexed: &AtomicBool,
    paced: bool,
) {
    let mut chunk = vec![0_u8; CHUNK_FRAMES * FRAME_BYTES];
    let mut stream = 0_u32;
    // While a stream lasts: when its frame zero was due, how many frames it has given, and when
    // the last of them came.
    let mut open: Option<(Instant, u64, Instant)> = None;
    let mut previous: Option<u64> = None;
    // Silent frames since the last sounding one, on the loopback.
    let mut silent = 0_u64;
    while !done.load(Ordering::Relaxed) {
        if paced && let Some((zero, frames, _)) = open {
            until_due(
                zero + Duration::from_nanos(frames * 1_000_000_000 / u64::from(RATE)),
                log,
            );
        }
        let asked = Instant::now();
        let mut filled = 0;
        while filled < chunk.len() {
            match pipe.read(&mut chunk[filled..]) {
                Ok(0) => break,
                Ok(read) => filled += read,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => panic!("the device's pipe: {error}"),
            }
        }
        let at = Instant::now();
        let mut log = log.lock().unwrap_or_else(PoisonError::into_inner);
        if filled < FRAME_BYTES {
            // The writer has gone: the stream is over, and the device waits for the next.
            if let Some((zero, frames, last)) = open.take() {
                log.streams
                    .push((frames, last.saturating_duration_since(zero)));
            }
            previous = None;
            drop(log);
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        let frames = filled / FRAME_BYTES;
        match open.as_mut() {
            None => {
                stream += 1;
                open = Some((at, 0, at));
            }
            Some((zero, _, _)) => {
                let waited = at.saturating_duration_since(asked);
                if paced && waited > STARVED {
                    log.underruns.push((at, waited));
                    // The device would have played silence; this one waits, and its clock goes
                    // on from where the frames came.
                    *zero += waited;
                }
            }
        }
        if let Some((_, given, last)) = open.as_mut() {
            *given += frames as u64;
            *last = at;
        }
        let mut taken = Taken {
            at,
            stream,
            frames,
            sounding: 0,
            first: None,
            last: None,
            loud: first_loud(&chunk[..frames * FRAME_BYTES]),
        };
        for frame in chunk[..frames * FRAME_BYTES].as_chunks::<FRAME_BYTES>().0 {
            let left = f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
            let right = f32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]);
            if left == 0.0 && right == 0.0 {
                silent += 1;
                continue;
            }
            taken.sounding += 1;
            let index = carried(left, right);
            if !paced && indexed.load(Ordering::Relaxed) {
                log.underruns
                    .extend(silence_inside(at, silent, index, previous));
            }
            silent = 0;
            if taken.first.is_none() {
                taken.first = index;
            }
            taken.last = index;
            if let (Some(index), Some(before)) = (index, previous)
                && index != before + 1
                && indexed.load(Ordering::Relaxed)
            {
                log.jumps.push((at, before, index));
            }
            previous = index;
        }
        log.taken.push(taken);
    }
    if let Some((zero, frames, last)) = open {
        log.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .streams
            .push((frames, last.saturating_duration_since(zero)));
    }
}

/// The first frame of `frames` louder than [`LOUD`] in either channel.
fn first_loud(frames: &[u8]) -> Option<usize> {
    frames
        .as_chunks::<FRAME_BYTES>()
        .0
        .iter()
        .position(|frame| {
            let left = f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
            let right = f32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]);
            left.abs() > LOUD || right.abs() > LOUD
        })
}

/// Where the stand-in's named pipe is made: `$XDG_RUNTIME_DIR`, else `/dev/shm` — memory file
/// systems both, whose nodes no journal holds (ADR 3465) — named for this process.
fn fifo_path() -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(|| PathBuf::from("/dev/shm"), PathBuf::from);
    runtime.join(format!("quorra-audio-in-use-{}.fifo", std::process::id()))
}

/// Sleeps until `due`, and logs the reader's own lateness where it woke more than [`STARVED`] after.
fn until_due(due: Instant, log: &Mutex<DeviceLog>) {
    let now = Instant::now();
    if due > now {
        std::thread::sleep(due - now);
    }
    let woke = Instant::now();
    let late = woke.saturating_duration_since(due);
    if late > STARVED {
        log.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .late
            .push((woke, late));
    }
}

/// An underrun on the loopback: `silent` frames between the frame `previous` and the frame
/// `index` that follows it, which the device played as silence where the helper had none ready.
fn silence_inside(
    at: Instant,
    silent: u64,
    index: Option<u64>,
    previous: Option<u64>,
) -> Option<(Instant, Duration)> {
    let (index, before) = (index?, previous?);
    (silent > 0 && index == before + 1).then(|| {
        (
            at,
            Duration::from_nanos(silent * 1_000_000_000 / u64::from(RATE)),
        )
    })
}

/// The index a frame of [`indexed`] carries; `None` where the frame is not one (a right channel
/// below one sixteen-bit step, or not a whole number of them).
fn carried(left: f32, right: f32) -> Option<u64> {
    let low = (f64::from(left) * 32_768.0).round() + 32_768.0;
    let high = (f64::from(right) * 32_768.0).round() - 1.0;
    if !(0.0..65_536.0).contains(&low) || !(0.0..32_767.0).contains(&high) {
        return None;
    }
    Some(((high as u64) << 16) | low as u64)
}

/// `seconds` of 48 kHz stereo frames, each carrying its index: the left channel the low sixteen
/// bits less 32,768, the right the rest plus one — never a silent frame.
fn indexed(seconds: u32) -> Vec<(i16, i16)> {
    (0..u64::from(seconds) * u64::from(RATE))
        .map(|index| {
            let low = (index & 0xFFFF) as i32 - 32_768;
            let high = (index >> 16) as i32 + 1;
            (low as i16, high as i16)
        })
        .collect()
}

/// The frames as §13.3's sound object holds them: Table 305's `Signed` sixteen bits, big-endian.
fn sound_object(frames: &[(i16, i16)]) -> Recording {
    let mut data = Vec::with_capacity(frames.len() * 4);
    for (left, right) in frames {
        data.extend_from_slice(&left.to_be_bytes());
        data.extend_from_slice(&right.to_be_bytes());
    }
    Recording::Samples {
        format: SampleFormat {
            rate: RATE,
            channels: CHANNELS,
            bits: 16,
            encoding: Encoding::Signed,
        },
        data,
    }
}

/// The frames as a RIFF WAVE file of PCM, sixteen bits little-endian.
fn wave_file(frames: &[(i16, i16)]) -> Recording {
    let data_len = (frames.len() * 4) as u32;
    let mut file = Vec::with_capacity(44 + frames.len() * 4);
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&(36 + data_len).to_le_bytes());
    file.extend_from_slice(b"WAVEfmt ");
    file.extend_from_slice(&16_u32.to_le_bytes());
    file.extend_from_slice(&1_u16.to_le_bytes());
    file.extend_from_slice(&CHANNELS.to_le_bytes());
    file.extend_from_slice(&RATE.to_le_bytes());
    file.extend_from_slice(&(RATE * 4).to_le_bytes());
    file.extend_from_slice(&4_u16.to_le_bytes());
    file.extend_from_slice(&16_u16.to_le_bytes());
    file.extend_from_slice(b"data");
    file.extend_from_slice(&data_len.to_le_bytes());
    for (left, right) in frames {
        file.extend_from_slice(&left.to_le_bytes());
        file.extend_from_slice(&right.to_le_bytes());
    }
    Recording::Wave(file)
}

fn played(recording: Recording, playing: Playing, rendition: bool) -> Played {
    Played {
        document: DOCUMENT,
        label: "the measured sound".to_owned(),
        sound: SoundData {
            recording,
            playing,
            screen: None,
            rendition: rendition.then_some(Associated {
                annotation: SCREEN,
                or_resume: false,
            }),
            window: None,
        },
    }
}

/// Waits until the player says `wanted`, up to `limit`, and says when; a refusal fails the walk.
fn until(player: &mut Player, limit: Duration, wanted: fn(&Heard) -> bool) -> Instant {
    let deadline = Instant::now() + limit;
    loop {
        for heard in player.heard() {
            if let Heard::Refused(why) = &heard {
                panic!("the player refused: {why}");
            }
            if wanted(&heard) {
                return Instant::now();
            }
        }
        assert!(
            Instant::now() < deadline,
            "the player said nothing awaited in {limit:?}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// What `/proc` says of one of this process's children: its peak resident set, its peak address
/// space, and its processor time.
#[derive(Debug, Default, Clone, Copy)]
struct Used {
    resident_kib: u64,
    address_kib: u64,
    processor: Duration,
}

/// This process's children by their program's name, as the kernel lists each thread's.
fn children() -> Vec<(String, u32)> {
    let mut found = Vec::new();
    for task in std::fs::read_dir("/proc/self/task")
        .expect("this process's threads")
        .flatten()
    {
        let Ok(list) = std::fs::read_to_string(task.path().join("children")) else {
            continue;
        };
        for pid in list
            .split_whitespace()
            .filter_map(|pid| pid.parse::<u32>().ok())
        {
            if let Ok(name) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
                found.push((name.trim().to_owned(), pid));
            }
        }
    }
    found
}

fn used(pid: u32) -> Option<Used> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let kib = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|number| number.parse::<u64>().ok())
            .unwrap_or(0)
    };
    let schedstat = std::fs::read_to_string(format!("/proc/{pid}/schedstat")).ok()?;
    let nanos = schedstat.split_whitespace().next()?.parse::<u64>().ok()?;
    Some(Used {
        resident_kib: kib("VmHWM:"),
        address_kib: kib("VmPeak:"),
        processor: Duration::from_nanos(nanos),
    })
}

/// The child whose program's name starts with `name`, and what it used.
fn child_used(name: &str) -> Option<Used> {
    children()
        .into_iter()
        .find(|(program, _)| name.starts_with(program.as_str()))
        .and_then(|(_, pid)| used(pid))
}

fn milliseconds(span: Duration) -> f64 {
    span.as_secs_f64() * 1000.0
}

fn signed_ms(later: Instant, earlier: Instant) -> f64 {
    if later >= earlier {
        milliseconds(later - earlier)
    } else {
        -milliseconds(earlier - later)
    }
}

fn spread(readings: &[f64]) -> String {
    let mut sorted = readings.to_vec();
    sorted.sort_by(f64::total_cmp);
    let median = sorted.get(sorted.len() / 2).copied().unwrap_or(f64::NAN);
    format!(
        "median {median:.1} ms (min {:.1}, max {:.1}, n {})",
        sorted.first().copied().unwrap_or(f64::NAN),
        sorted.last().copied().unwrap_or(f64::NAN),
        sorted.len()
    )
}

/// A process's major page faults so far and its memory swapped out, in KiB.
fn paged(pid: u32) -> (u64, u64) {
    let faults = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            // The fields after the parenthesised name: the major faults are the tenth of them.
            let (_, after) = stat.rsplit_once(')')?;
            after.split_whitespace().nth(9)?.parse().ok()
        })
        .unwrap_or(0);
    let swapped = std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|line| line.strip_prefix("VmSwap:"))
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|number| number.parse().ok())
        })
        .unwrap_or(0);
    (faults, swapped)
}

/// The kernel's one-minute load average.
fn one_minute_load() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or("?")
        .to_owned()
}

/// The load the machine was under, as the kernel's load averages and its count of processors.
fn load() -> String {
    let averages = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let processors = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    format!(
        "load averages {} on {processors} processors",
        averages
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
    )
}

/// Phases 1 and 2: activation to the first frame at the device, cold and warm; the cold readings.
fn starts(device: &Device, setup: &Setup, report: &mut String) -> Vec<f64> {
    let ten = Duration::from_secs(10);
    device.phase("the cold and warm starts", true);
    // 1. Activation to the first frame at the device, cold: a player that has started nothing.
    let short = indexed(2);
    let mut cold = Vec::new();
    let mut cold_said = Vec::new();
    for _ in 0..5 {
        let mut player = device.player(setup);
        let activated = Instant::now();
        player.play(played(sound_object(&short), Playing::default(), false));
        let said = until(&mut player, ten, |heard| matches!(heard, Heard::Playing(_)));
        let first = device
            .first_sound_after(activated, ten)
            .expect("the cold sound reached the device");
        cold.push(signed_ms(first.at, activated));
        cold_said.push(signed_ms(said, activated));
        until(&mut player, ten, |heard| matches!(heard, Heard::Ended(_)));
        drop(player);
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = writeln!(
        report,
        "cold start, activation -> first frame at the device (2 s §13.3 sound): {}",
        spread(&cold)
    );
    let _ = writeln!(
        report,
        "cold start, activation -> the player says Playing: {}",
        spread(&cold_said)
    );

    // 2. The same, warm: the helper, its device and the worker already started.
    let mut warm = Vec::new();
    {
        let mut player = device.player(setup);
        player.play(played(sound_object(&short), Playing::default(), false));
        until(&mut player, ten, |heard| matches!(heard, Heard::Ended(_)));
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(200));
            let activated = Instant::now();
            player.play(played(sound_object(&short), Playing::default(), false));
            let first = device
                .first_sound_after(activated, ten)
                .expect("the warm sound reached the device");
            warm.push(signed_ms(first.at, activated));
            until(&mut player, ten, |heard| matches!(heard, Heard::Ended(_)));
        }
    }
    let _ = writeln!(
        report,
        "warm start, activation -> first frame at the device: {}",
        spread(&warm)
    );

    cold
}

/// Phase 3: each format whole, on a fresh player.
fn formats(device: &Device, setup: &Setup, report: &mut String) {
    // 3. Each format whole, on a fresh player: the first frame, and what the worker and the helper
    //    used per second of sound heard.
    let thirty = indexed(30);
    let mut formats: Vec<(String, Recording)> = vec![
        (
            "§13.3 raw, 16-bit 48 kHz stereo, 30 s".to_owned(),
            sound_object(&thirty),
        ),
        (
            "RIFF WAVE, 16-bit 48 kHz stereo, 30 s".to_owned(),
            wave_file(&thirty),
        ),
    ];
    drop(thirty);
    for (name, path) in &setup.clips {
        formats.push((
            format!("MP4 {name}"),
            Recording::Mp4(std::fs::read(path).expect("a held clip")),
        ));
    }
    for (name, recording) in formats {
        let clip = matches!(recording, Recording::Mp4(_));
        device.phase(
            if clip {
                "the AAC clips"
            } else {
                "the raw and WAVE sounds"
            },
            !clip,
        );
        let mut player = device.player(setup);
        let activated = Instant::now();
        player.play(played(recording, Playing::default(), false));
        let said = until(&mut player, Duration::from_secs(60), |heard| {
            matches!(heard, Heard::Playing(_))
        });
        let first = device
            .first_sound_after(activated, Duration::from_secs(60))
            .expect("the clip reached the device");
        let ended = until(&mut player, Duration::from_secs(120), |heard| {
            matches!(heard, Heard::Ended(_))
        });
        // The device has the frames the helper wrote but it has not taken yet.
        std::thread::sleep(Duration::from_millis(100));
        let heard = device
            .log()
            .taken
            .iter()
            .filter(|taken| taken.stream == first.stream)
            .map(|taken| taken.sounding)
            .sum::<usize>() as f64
            / f64::from(RATE);
        let worker = child_used(pdf_media_worker::WORKER_PROGRAM);
        let helper = child_used(quorra_sound::HELPER_PROGRAM);
        let per = |used: Option<Used>| {
            used.map_or_else(
                || "unreadable".to_owned(),
                |used| {
                    format!(
                        "peak resident {:.1} MiB, peak address space {:.1} MiB, processor {:.1} ms \
                         = {:.2} ms per second of sound",
                        used.resident_kib as f64 / 1024.0,
                        used.address_kib as f64 / 1024.0,
                        milliseconds(used.processor),
                        milliseconds(used.processor) / heard
                    )
                },
            )
        };
        let _ = writeln!(
            report,
            "{name}: activation -> the player says Playing {:.1} ms, -> first frame {:.1} ms, -> the player says Ended {:.1} s, {heard:.2} s heard\n  worker: {}\n  helper: {}",
            signed_ms(said, activated),
            signed_ms(first.at, activated),
            signed_ms(ended, activated) / 1000.0,
            per(worker),
            per(helper)
        );
        drop(player);
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Phase 4: pause, resume and stop.
fn pauses(device: &Device, setup: &Setup, report: &mut String) {
    let ten = Duration::from_secs(10);
    // 4. Pause, resume and stop (§12.6.4.14, Table 218's /OP 2, 3 and 1) on a 20-second sound.
    let twenty = indexed(20);
    device.phase("pause, resume and stop", true);
    for trial in 1..=3 {
        let mut player = device.player(setup);
        player.play(played(sound_object(&twenty), Playing::default(), true));
        until(&mut player, ten, |heard| matches!(heard, Heard::Playing(_)));
        std::thread::sleep(Duration::from_secs(3));
        let paused = Instant::now();
        player.operate(DOCUMENT, SCREEN, MediaOperation::Pause);
        std::thread::sleep(Duration::from_secs(1));
        let resumed = Instant::now();
        player.operate(DOCUMENT, SCREEN, MediaOperation::Resume);
        let back = device
            .first_sound_after(resumed, ten)
            .expect("the resumed sound reached the device");
        std::thread::sleep(Duration::from_secs(2));
        let stopped = Instant::now();
        player.stop();
        std::thread::sleep(Duration::from_millis(500));
        // On the loopback a capture period can carry the pause's last frames entirely before it,
        // so a trial that took none after the pause is said rather than read.
        let Some(gone) = device.last_sound_between(paused, resumed) else {
            let _ = writeln!(
                report,
                "pause/resume/stop trial {trial}: no sounding frame was taken between the pause and the resumption"
            );
            player.stop();
            continue;
        };
        let last = device
            .last_sound_between(stopped, Instant::now())
            .map_or(0.0, |taken| signed_ms(taken.at, stopped));
        let skipped = match (gone.last, back.first) {
            (Some(before), Some(after)) => after as i64 - before as i64 - 1,
            _ => i64::MIN,
        };
        let _ = writeln!(
            report,
            "pause/resume/stop trial {trial}: pause -> last frame {:.1} ms; resume -> first frame {:.1} ms; \
             frames skipped (+) or repeated (-) across the pause {skipped} ({:.1} ms); stop -> last frame {last:.1} ms",
            signed_ms(gone.at, paused),
            signed_ms(back.at, resumed),
            skipped as f64 * 1000.0 / f64::from(RATE)
        );
        drop(player);
        std::thread::sleep(Duration::from_millis(200));
    }
    drop(twenty);
}

/// Phase 5: the long sound played four times; the clock's samples, from where it began.
fn long_sound(
    device: &Device,
    setup: &Setup,
    report: &mut String,
) -> (Instant, u64, Vec<(Instant, Duration)>) {
    // 5. The media clock against the device's frames over the long sound, played four times.
    let length = setup.long_seconds;
    let frames = indexed(length);
    let pass = frames.len() as u64;
    let mut player = device.player(setup);
    device.phase("the long sound", true);
    let began = Instant::now();
    let playing = Playing {
        repeat: 4.0,
        ..Playing::default()
    };
    player.play(played(sound_object(&frames), playing, false));
    drop(frames);
    until(&mut player, Duration::from_secs(60), |heard| {
        matches!(heard, Heard::Playing(_))
    });
    let mut samples: Vec<(Instant, Duration)> = Vec::new();
    let mut loads: Vec<String> = Vec::new();
    // The helper's pages: its major faults and its swapped-out memory, read each minute, because a
    // sound held whole for minutes is memory the kernel may reclaim before the device reaches it.
    let helper_pid = children()
        .into_iter()
        .find(|(program, _)| quorra_sound::HELPER_PROGRAM.starts_with(program.as_str()))
        .map(|(_, pid)| pid);
    let mut faults = helper_pid.map_or((0, 0), paged).0;
    let deadline = Instant::now() + Duration::from_secs(u64::from(length) * 4 + 120);
    let mut ended = false;
    while !ended && Instant::now() < deadline {
        std::thread::sleep(Duration::from_secs(1));
        if let Some(position) = player.position() {
            samples.push((Instant::now(), position));
            if samples.len().is_multiple_of(60) {
                let (now_faults, swapped) = helper_pid.map_or((0, 0), paged);
                loads.push(format!(
                    "{} major-faults {} swapped {swapped} KiB",
                    one_minute_load(),
                    now_faults.saturating_sub(faults)
                ));
                faults = now_faults;
            }
        }
        ended = player
            .heard()
            .iter()
            .any(|heard| matches!(heard, Heard::Ended(_)));
    }
    assert!(ended, "the long sound came to its end");
    minutes(device, began, &loads, report);
    let helper = child_used(quorra_sound::HELPER_PROGRAM);
    drop(player);
    let resident = std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .unwrap_or_default()
        .to_owned();

    let _ = writeln!(
        report,
        "long sound's helper: {}",
        helper.map_or_else(
            || "unreadable".to_owned(),
            |used| format!(
                "peak resident {:.1} MiB, processor {:.1} ms",
                used.resident_kib as f64 / 1024.0,
                milliseconds(used.processor)
            )
        )
    );
    let _ = writeln!(report, "this process (the window's half): {resident}");
    (began, pass, samples)
}

/// The underruns of each minute since `began`, beside the machine's one-minute load average at
/// that minute's end (`loads`), so that a cluster of them is read against what else the machine ran.
fn minutes(device: &Device, began: Instant, loads: &[String], report: &mut String) {
    let seen = device.log();
    let mut minute = 0_u32;
    let mut line = String::new();
    loop {
        let from = began + Duration::from_secs(u64::from(minute) * 60);
        if from > Instant::now() {
            break;
        }
        let to = from + Duration::from_secs(60);
        let inside: Vec<Duration> = seen
            .underruns
            .iter()
            .filter(|(at, _)| *at >= from && *at < to)
            .map(|(_, waited)| *waited)
            .collect();
        let _ = write!(
            line,
            "\n    {minute}: {} underruns, {:.0} ms; load {}",
            inside.len(),
            milliseconds(inside.iter().sum()),
            loads.get(minute as usize).map_or("?", String::as_str)
        );
        minute += 1;
    }
    let _ = writeln!(
        report,
        "the long sound's underruns by minute (minute:count/time@the one-minute load average, and the helper's major faults in the minute and memory swapped out, at its end):{line}"
    );
}

/// The clock's samples against the device's frames, and what the device saw over the whole run;
/// the offsets read.
fn clock_read(
    seen: &DeviceLog,
    began: Instant,
    pass: u64,
    samples: &[(Instant, Duration)],
    report: &mut String,
) -> usize {
    // The device's frame at each sample of the clock: the last index it had taken, counted over
    // passes, against the clock's reading.
    let mut offsets: Vec<(f64, f64)> = Vec::new();
    let mut passes = 0_u64;
    let mut last_index = 0_u64;
    let mut cursor = seen.taken.partition_point(|taken| taken.at < began);
    let origin = samples.first().map_or_else(Instant::now, |first| first.0);
    for (at, position) in samples {
        while cursor < seen.taken.len() && seen.taken[cursor].at <= *at {
            if let Some(index) = seen.taken[cursor].last {
                if index + pass / 2 < last_index {
                    passes += 1;
                }
                last_index = index;
            }
            cursor += 1;
        }
        let heard = (passes * pass + last_index + 1) as f64 / f64::from(RATE);
        offsets.push((
            signed_ms(*at, origin) / 1000.0,
            (position.as_secs_f64() - heard) * 1000.0,
        ));
    }
    let n = offsets.len() as f64;
    let (sx, sy) = offsets
        .iter()
        .fold((0.0, 0.0), |(sx, sy), (x, y)| (sx + x, sy + y));
    let (mx, my) = (sx / n, sy / n);
    let (sxy, sxx) = offsets.iter().fold((0.0, 0.0), |(sxy, sxx), (x, y)| {
        (sxy + (x - mx) * (y - my), sxx + (x - mx) * (x - mx))
    });
    // Milliseconds per second is parts per thousand; a million parts are a thousand of those.
    let ppm = sxy / sxx * 1000.0;
    let (low, high) = offsets
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), (_, y)| {
            (low.min(*y), high.max(*y))
        });
    let _ = writeln!(
        report,
        "media clock - device's frames over {:.0} s ({} s x 4, {} readings): drift {ppm:.2} ppm by least squares; \
         offset from {low:.1} to {high:.1} ms (first {:.1}, last {:.1})",
        offsets.last().map_or(0.0, |last| last.0),
        pass / u64::from(RATE),
        offsets.len(),
        offsets.first().map_or(0.0, |first| first.1),
        offsets.last().map_or(0.0, |last| last.1),
    );
    device_read(seen, began, origin, report);
    offsets.len()
}

/// Where the helper keeps a trace of its callbacks (`QUORRA_SOUND_TRACE`, ADR 3465), the device's
/// underruns and its reader's late reads go beside it as `device.tsv`, each on the wall clock in
/// nanoseconds since the epoch, so that the two are read against each other.
fn traced(seen: &DeviceLog) {
    let Some(directory) = std::env::var_os("QUORRA_SOUND_TRACE") else {
        return;
    };
    let (now, wall) = (Instant::now(), std::time::SystemTime::now());
    let epoch = wall
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let on_the_wall = |at: Instant| {
        epoch
            .saturating_sub(now.saturating_duration_since(at))
            .as_nanos()
    };
    let mut lines = String::from("wall_ns\tkind\tus\n");
    for (kind, list) in [("underrun", &seen.underruns), ("late", &seen.late)] {
        for (at, span) in list {
            let _ = writeln!(lines, "{}\t{kind}\t{}", on_the_wall(*at), span.as_micros());
        }
    }
    std::fs::write(PathBuf::from(directory).join("device.tsv"), lines).expect("the device's trace");
}

/// What the device saw over the whole run: its own clock, frames out of order, underruns by
/// phase, and the silence between the long sound's passes.
fn device_read(seen: &DeviceLog, began: Instant, origin: Instant, report: &mut String) {
    traced(seen);
    // The device's own rate against the monotonic clock, over every stream longer than a second.
    let (frames, span) = seen
        .streams
        .iter()
        .filter(|(_, span)| *span > Duration::from_secs(1))
        .fold((0_u64, Duration::ZERO), |(frames, span), (more, longer)| {
            (frames + more, span + *longer)
        });
    let _ = writeln!(
        report,
        "the stand-in device's clock: {frames} frames over {:.3} s = {:.1} ppm from {RATE} Hz",
        span.as_secs_f64(),
        (frames as f64 / span.as_secs_f64() / f64::from(RATE) - 1.0) * 1e6
    );
    let gaps: Vec<String> = seen
        .jumps
        .iter()
        .map(|(at, from, to)| format!("{:.1} s: {from} -> {to}", signed_ms(*at, origin) / 1000.0))
        .collect();
    let _ = writeln!(
        report,
        "frames out of order inside a stream (pass boundaries and pauses included): {}\n  {}",
        seen.jumps.len(),
        gaps.iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    let worst = seen
        .underruns
        .iter()
        .map(|(_, waited)| *waited)
        .max()
        .unwrap_or_default();
    let waited_in_all: Duration = seen.underruns.iter().map(|(_, waited)| *waited).sum();
    let _ = writeln!(
        report,
        "underruns (a read that waited more than {} ms for due frames): {} over the run, {:.1} ms in all, the longest {:.1} ms",
        STARVED.as_millis(),
        seen.underruns.len(),
        milliseconds(waited_in_all),
        milliseconds(worst)
    );
    for (number, (name, from)) in seen.phases.iter().enumerate() {
        let to = seen.phases.get(number + 1).map(|next| next.1);
        let inside: Vec<Duration> = seen
            .underruns
            .iter()
            .filter(|(at, _)| at >= from && to.is_none_or(|to| *at < to))
            .map(|(_, waited)| *waited)
            .collect();
        let _ = writeln!(
            report,
            "  in {name}: {}, {:.1} ms in all",
            inside.len(),
            milliseconds(inside.iter().sum())
        );
    }
    // The silence between one pass of the long sound and the next, at the device: the silent
    // frames taken from the last frame of a pass to the first of the next, which a wrap of the
    // frames' indices marks.
    let mut between = Vec::new();
    let mut silent = 0_usize;
    let mut before: Option<u64> = None;
    for taken in seen.taken.iter().filter(|taken| taken.at >= began) {
        if let (Some(first), Some(last)) = (taken.first, before)
            && first < last
        {
            between.push(silent as f64 * 1000.0 / f64::from(RATE));
        }
        if taken.sounding > 0 {
            silent = taken.frames - taken.sounding;
            before = taken.last.or(before);
        } else {
            silent += taken.frames;
        }
    }
    let _ = writeln!(
        report,
        "the long sound's passes at the device: silence between one pass and the next {}",
        spread(&between)
    );
}

#[test]
#[ignore = "an instrument: run by tools/audio-in-use.sh, which builds its device and its programs"]
fn the_sound_path_measured_in_use() {
    let setup = Setup::from_environment();
    let device = Device::open(&setup);
    let mut report = String::new();
    let _ = if setup.loopback {
        writeln!(
            report,
            "device: ALSA plug -> hw:Loopback,0,0 at {RATE} Hz stereo S16_LE, captured from hw:Loopback,1,0 by ffmpeg (times late by up to a capture period)"
        )
    } else {
        writeln!(
            report,
            "device: ALSA plug -> file -> null, read from a {PIPE_BYTES}-byte pipe at {RATE} Hz stereo FLOAT_LE on the monotonic clock"
        )
    };
    let _ = writeln!(report, "at the start: {}", load());
    let cold = starts(&device, &setup, &mut report);
    formats(&device, &setup, &mut report);
    pauses(&device, &setup, &mut report);
    let (began, pass, samples) = long_sound(&device, &setup, &mut report);
    let _ = writeln!(report, "at the end: {}", load());
    let seen = device.close();
    let offsets = clock_read(&seen, began, pass, &samples, &mut report);
    println!("{report}");
    let out = setup.directory.join("readings.txt");
    std::fs::write(&out, &report).expect("the readings written");
    assert!(
        !cold.is_empty() && offsets > 0,
        "every sound reached the device"
    );
}

/// A clip's pictures measured against its sound (RFC 0015 section 6 round 9, ADR 3556): the clip
/// `AUDIO_IN_USE_AV` — made by `tools/audio-in-use.sh` with `ffmpeg`, its picture `25 × n` and the
/// start of its `n`-th beep both at `n` seconds of the file — is played by the sound player on the
/// device and shown by the video player beside it, as a window shows it. Each second's picture is
/// timed when [`VideoPlayer::due`] hands it on, and each beep when the device took its first loud
/// frame; the difference is the pictures' offset from the sound, positive where the picture comes
/// after. Its spread over the clip is the drift the slaving exists to prevent.
///
/// `AUDIO_IN_USE_AV_UNSLAVED=1` is the control: the pictures on the monotonic clock from the moment
/// the clip is handed over, not following the sound, whose offset is the sound's start-up.
///
/// What it does not measure: a window's present after `due` (a frame at most), and on the loopback
/// the capture's period, by which every beep is read late — both the same at every second, so they
/// move the mean and not the drift.
#[test]
#[ignore = "an instrument: run by tools/audio-in-use.sh, which builds its device and its programs"]
fn a_clip_s_pictures_measured_against_its_sound() {
    let setup = Setup::from_environment();
    let Some(clip) = std::env::var_os("AUDIO_IN_USE_AV").map(PathBuf::from) else {
        println!("no AUDIO_IN_USE_AV clip: the pictures were not measured");
        return;
    };
    let file = std::fs::read(&clip).expect("the clip");
    let device = Device::open(&setup);
    device.phase("a clip's pictures and its sound", false);
    let mut sound = device.player(&setup);
    let mut video = VideoPlayer::new(Box::new(|| {})).with_worker(setup.worker.clone());
    sound.play(played(
        Recording::Mp4(file.clone()),
        Playing::default(),
        false,
    ));
    video.play_beside(
        Shown {
            document: DOCUMENT,
            label: "the measured clip".to_owned(),
            file: Arc::from(file.as_slice()),
            screen: Some(SCREEN),
            associated: None,
            auto_play: true,
            repeat: 1.0,
            duration: pdf_model::media::SimpleDuration::Intrinsic,
            section: pdf_model::media::Section::WHOLE,
            controls: false,
            fit: pdf_model::media::Fit::PlayersDefault,
            window: None,
            quarter_turns: 0,
            backward: false,
            palindrome: false,
            synchronous: false,
        },
        // The instrument's control: `AUDIO_IN_USE_AV_UNSLAVED` runs the pictures on the monotonic
        // clock instead, as they ran before ADR 3555, which the reading must tell apart.
        std::env::var_os("AUDIO_IN_USE_AV_UNSLAVED")
            .is_none()
            .then(|| sound.listening()),
    );
    let began = Instant::now();
    // Each picture handed on: its time in the clip and when it was handed.
    let mut handed: Vec<(Duration, Instant)> = Vec::new();
    let mut said = Vec::new();
    'shown: while began.elapsed() < Duration::from_secs(600) {
        if let Some(frame) = video.due(Instant::now()) {
            handed.push((frame.at, Instant::now()));
        }
        for seen in video.seen() {
            let over = matches!(
                seen,
                Seen::Ended(_) | Seen::Stopped(_) | Seen::Refused(_) | Seen::HandOn(_)
            );
            said.push(seen.sentence().to_owned());
            if over {
                break 'shown;
            }
        }
        let _ = sound.heard();
        std::thread::sleep(Duration::from_millis(1));
    }
    sound.stop();
    let seen = device.close();
    let (beeps, offsets) = av_offsets(&seen, began, &handed);
    let report = av_report(&clip, &handed, &beeps, &offsets, &said);
    println!("{report}");
    let out = setup.directory.join("av-readings.txt");
    std::fs::write(&out, &report).expect("the readings written");
    assert!(
        !offsets.is_empty(),
        "the pictures and the beeps were both seen"
    );
}

/// Each beep's start as the device took it, and each second's picture's offset from its beep in
/// milliseconds: [`a_clip_s_pictures_measured_against_its_sound`]'s reading of what it saw.
fn av_offsets(
    seen: &DeviceLog,
    began: Instant,
    handed: &[(Duration, Instant)],
) -> (Vec<Instant>, Vec<f64>) {
    let started = seen
        .phases
        .iter()
        .find(|(name, _)| *name == "a clip's pictures and its sound")
        .map_or(began, |(_, at)| *at);
    // Each beep's start: a read with a loud frame after half a second without one.
    let mut beeps = Vec::new();
    let mut last_loud: Option<Instant> = None;
    for taken in seen.taken.iter().filter(|taken| taken.at >= started) {
        let Some(loud) = taken.loud else {
            continue;
        };
        let before =
            Duration::from_nanos(((taken.frames - loud) as u64) * 1_000_000_000 / u64::from(RATE));
        let onset = taken.at.checked_sub(before).unwrap_or(taken.at);
        if last_loud
            .is_none_or(|last| onset.saturating_duration_since(last) > Duration::from_millis(500))
        {
            beeps.push(onset);
        }
        last_loud = Some(onset);
    }
    // Each second's picture: the first handed on at or after the second, moved back by how far
    // past it it was (a picture passed over late is not handed on).
    let seconds = handed.last().map_or(0, |(at, _)| at.as_secs());
    let pictures: Vec<Instant> = (0..=seconds)
        .filter_map(|second| {
            let (at, when) = handed
                .iter()
                .find(|(at, _)| *at >= Duration::from_secs(second))?;
            let late = at.saturating_sub(Duration::from_secs(second));
            Some(when.checked_sub(late).unwrap_or(*when))
        })
        .collect();
    // Each beep against the second's picture nearest it: a beep the device's start swallowed
    // leaves its second out rather than pairing every later beep with the second before its own.
    let offsets = beeps
        .iter()
        .filter_map(|beep| {
            pictures
                .iter()
                .map(|picture| signed_ms(*picture, *beep))
                .min_by(|one, other| one.abs().total_cmp(&other.abs()))
        })
        .collect();
    (beeps, offsets)
}

/// The A/V reading's sentences.
fn av_report(
    clip: &std::path::Path,
    handed: &[(Duration, Instant)],
    beeps: &[Instant],
    offsets: &[f64],
    said: &[String],
) -> String {
    let mut report = String::new();
    let _ = writeln!(
        report,
        "a clip's pictures against its sound ({}): {} pictures handed on, {} beeps heard, {} seconds compared",
        clip.display(),
        handed.len(),
        beeps.len(),
        offsets.len()
    );
    for sentence in said {
        let _ = writeln!(report, "  the video player said: {sentence}");
    }
    if !offsets.is_empty() {
        let mean = offsets.iter().sum::<f64>() / offsets.len() as f64;
        let (low, high) = offsets.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(low, high), &offset| (low.min(offset), high.max(offset)),
        );
        let tenth = (offsets.len() / 10).max(1);
        let head = offsets[..tenth].iter().sum::<f64>() / tenth as f64;
        let tail = offsets[offsets.len() - tenth..].iter().sum::<f64>() / tenth as f64;
        let _ = writeln!(
            report,
            "  picture after sound: mean {mean:.1} ms, from {low:.1} to {high:.1} ms; the first tenth's mean {head:.1} ms, the last tenth's {tail:.1} ms (drift {:.1} ms)",
            tail - head
        );
        let _ = writeln!(report, "  picture after sound: {}", spread(offsets));
        let _ = writeln!(
            report,
            "  each second: {}",
            offsets
                .iter()
                .map(|offset| format!("{offset:.0}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    report
}
