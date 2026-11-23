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
}

/// Everything the device saw.
#[derive(Default)]
struct DeviceLog {
    taken: Vec<Taken>,
    /// Each underrun: when, and how long the read waited past its frames' due time.
    underruns: Vec<(Instant, Duration)>,
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
}

impl Device {
    fn open(setup: &Setup) -> Self {
        let directory = &setup.directory;
        std::fs::create_dir_all(directory).expect("the device's directory");
        let fifo = directory.join("device.fifo");
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
        let log = Arc::new(Mutex::new(DeviceLog::default()));
        let done = Arc::new(AtomicBool::new(false));
        let indexed = Arc::new(AtomicBool::new(true));
        let reader = {
            let log = Arc::clone(&log);
            let done = Arc::clone(&done);
            let indexed = Arc::clone(&indexed);
            std::thread::Builder::new()
                .name("device".to_owned())
                .spawn(move || take(std::fs::File::from(fd), &log, &done, &indexed))
                .expect("the device's thread")
        };
        Self {
            log,
            done,
            indexed,
            reader: Some(reader),
            wrapper,
        }
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
        if let Some(reader) = self.reader.take() {
            reader.join().expect("the device's thread");
        }
        std::mem::take(&mut *self.log())
    }
}

/// The device's thread: a millisecond's frames when they are due, for as long as a writer has the
/// pipe open, and a look each millisecond for the next writer while none has.
fn take(mut pipe: std::fs::File, log: &Mutex<DeviceLog>, done: &AtomicBool, indexed: &AtomicBool) {
    let mut chunk = vec![0_u8; CHUNK_FRAMES * FRAME_BYTES];
    let mut stream = 0_u32;
    // While a stream lasts: when its frame zero was due, how many frames it has given, and when
    // the last of them came.
    let mut open: Option<(Instant, u64, Instant)> = None;
    let mut previous: Option<u64> = None;
    while !done.load(Ordering::Relaxed) {
        if let Some((zero, frames, _)) = open {
            let due = zero + Duration::from_nanos(frames * 1_000_000_000 / u64::from(RATE));
            let now = Instant::now();
            if due > now {
                std::thread::sleep(due - now);
            }
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
                if waited > STARVED {
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
        };
        for frame in chunk[..frames * FRAME_BYTES].as_chunks::<FRAME_BYTES>().0 {
            let left = f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
            let right = f32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]);
            if left == 0.0 && right == 0.0 {
                continue;
            }
            taken.sounding += 1;
            let index = carried(left, right);
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
        let gone = device
            .last_sound_between(paused, resumed)
            .expect("the paused sound was heard before it paused");
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

/// What the device saw over the whole run: its own clock, frames out of order, underruns by
/// phase, and the silence between the long sound's passes.
fn device_read(seen: &DeviceLog, began: Instant, origin: Instant, report: &mut String) {
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
    let _ = writeln!(
        report,
        "device: ALSA plug -> file -> null, read from a {PIPE_BYTES}-byte pipe at {RATE} Hz stereo FLOAT_LE on the monotonic clock"
    );
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
