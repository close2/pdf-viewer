//! ISO 32000-2 §13.2.5's play parameters and §12.6.4.9's Table 212 honoured by the window's sound
//! player, and the media clock its device's count drives (RFC 0015 section 5, steps 6 and 7; ADRs
//! 3341, 3342, 3343), each proved with a recording sink standing in for the audio device.
//!
//! **No speaker is in these tests, and no person has heard a sound played with these
//! parameters**: the build machine has no sound card. What is proved is the arithmetic — the
//! samples, the passes and the times — the clauses' own numbers give.
//!
//! The worker is the program beside the test binary: `cargo build -p pdf-media-worker --bins`
//! first (trap 10's reason).

use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use pdf_model::media::{Playing, SimpleDuration};
use pdf_model::restriction::Operation;
use viewer_core::{
    Command, DocumentId, Event, RestrictionLevel, RestrictionPolicy, RestrictionScope, SoundData,
    Viewer,
};
use viewer_host::sound::{DeviceFormat, Heard, Played, Player, Progress, Report, Sink, Sounds};
use viewer_host::{Handed, Handing, MediaFiles};

/// The device: 8000 Hz, mono, so that a sound's 8-bit `Signed` samples reach it unconverted —
/// `0x40` as `+½` of full scale and `0xC0` as `−½` (Table 305, §13.3).
const FORMAT: DeviceFormat = DeviceFormat {
    rate: 8000,
    channels: 1,
};

/// How the double answers a sound it is handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Device {
    /// It reports half the frames, then all of them, then the end, at once.
    Ends,
    /// It reports half the frames and goes on sounding.
    Halfway,
}

/// What the recording sink saw: every sound started, in order, and the stops.
#[derive(Debug, Default)]
struct Record {
    started: Vec<Vec<f32>>,
    stops: usize,
}

struct Recording {
    record: Arc<Mutex<Record>>,
    device: Device,
}

impl Sink for Recording {
    fn format(&self) -> DeviceFormat {
        FORMAT
    }

    fn start(&mut self, samples: Vec<f32>, mut report: Report) -> Result<(), String> {
        let frames = u64::try_from(samples.len()).unwrap_or(u64::MAX);
        record(&self.record).started.push(samples);
        report(Progress::Played(frames / 2));
        if self.device == Device::Ends {
            report(Progress::Played(frames));
            report(Progress::Ended);
        }
        Ok(())
    }

    fn stop(&mut self) {
        let mut record = record(&self.record);
        record.stops = record.stops.saturating_add(1);
    }
}

fn record(record: &Arc<Mutex<Record>>) -> MutexGuard<'_, Record> {
    record.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A player over the double, its record, and what wakes the test.
fn player(device: Device) -> (Player, Arc<Mutex<Record>>, Receiver<()>) {
    let kept = Arc::new(Mutex::new(Record::default()));
    let shared = Arc::clone(&kept);
    let (wake, woken) = channel();
    let player = Player::new(
        Box::new(move || {
            Ok(Box::new(Recording {
                record: Arc::clone(&shared),
                device,
            }) as Box<dyn Sink>)
        }),
        Box::new(move || {
            let _ = wake.send(());
        }),
    );
    (player, kept, woken)
}

/// What the player says, until it has said `count` things or thirty seconds have passed.
fn heard(player: &mut Player, woken: &Receiver<()>, count: usize) -> Vec<Heard> {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(30))
        .unwrap_or_else(Instant::now);
    let mut heard = player.heard();
    while heard.len() < count {
        if woken
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .is_err()
        {
            break;
        }
        heard.extend(player.heard());
    }
    heard
}

/// `samples` as 8-bit `Signed` at 8000 Hz, mono, played as `playing` says.
fn played(samples: &[u8], playing: Playing) -> Played {
    Played {
        document: DocumentId(1),
        label: "the sound".to_owned(),
        sound: SoundData {
            recording: viewer_core::Recording::Samples {
                format: pdf_model::media::SampleFormat {
                    rate: 8000,
                    channels: 1,
                    bits: 8,
                    encoding: pdf_model::media::Encoding::Signed,
                },
                data: samples.to_vec(),
            },
            playing,
            screen: None,
            rendition: None,
        },
    }
}

/// Four frames: `+½, −½, +½, −½`.
const FOUR: [u8; 4] = [0x40, 0xC0, 0x40, 0xC0];

/// Table 291's `/V` as arithmetic: a level of one half scales `±½` to `±¼`, and the sentence
/// says the level. The wrong answers excluded: the level dropped, and it read as a percentage.
#[test]
fn the_level_scales_every_sample() {
    let (mut player, recorded, woken) = player(Device::Ends);
    player.play(played(
        &FOUR,
        Playing {
            level: 0.5,
            ..Playing::default()
        },
    ));
    let said = heard(&mut player, &woken, 2);
    assert!(
        matches!(said.as_slice(), [Heard::Playing(playing), Heard::Ended(_)] if playing.contains("at 50 %")),
        "the worker must be built: `cargo build -p pdf-media-worker --bins`; {said:?}"
    );
    assert_eq!(
        record(&recorded).started,
        vec![vec![0.25, -0.25, 0.25, -0.25]]
    );
}

/// Table 292's explicit duration cuts a longer sound: 0.25 ms at the device's 8000 Hz is two
/// frames of the four.
#[test]
fn a_duration_shorter_than_the_sound_cuts_it() {
    let (mut player, recorded, woken) = player(Device::Ends);
    player.play(played(
        &FOUR,
        Playing {
            duration: SimpleDuration::Seconds(0.000_25),
            ..Playing::default()
        },
    ));
    let said = heard(&mut player, &woken, 2);
    assert!(
        matches!(said.as_slice(), [Heard::Playing(_), Heard::Ended(_)]),
        "{said:?}"
    );
    assert_eq!(record(&recorded).started, vec![vec![0.5, -0.5]]);
}

/// Table 291's `/RC 2.5` over the sound's own duration: two whole passes and a third cut to half
/// of it, then the end. The wrong answers excluded: the count rounded to two or to three.
#[test]
fn a_repeat_count_plays_whole_passes_and_a_fraction() {
    let (mut player, recorded, woken) = player(Device::Ends);
    player.play(played(
        &FOUR,
        Playing {
            repeat: 2.5,
            ..Playing::default()
        },
    ));
    let said = heard(&mut player, &woken, 2);
    assert!(
        matches!(said.as_slice(), [Heard::Playing(playing), Heard::Ended(_)] if playing.contains("2.5 times")),
        "{said:?}"
    );
    let whole = vec![0.5, -0.5, 0.5, -0.5];
    assert_eq!(
        record(&recorded).started,
        vec![whole.clone(), whole, vec![0.5, -0.5]]
    );
}

/// §13.2.5's freeze: a duration of 50 ms over a one-frame sound sounds the frame and then waits,
/// silent, until the duration has elapsed — twice, for `/RC 2` — so the end is said no sooner
/// than 100 ms after the start, by the clock the device's count and the freeze drive.
#[test]
fn a_duration_longer_than_the_sound_is_frozen_silent() {
    let (mut player, recorded, woken) = player(Device::Ends);
    let began = Instant::now();
    player.play(played(
        &[0x40],
        Playing {
            duration: SimpleDuration::Seconds(0.05),
            repeat: 2.0,
            ..Playing::default()
        },
    ));
    let said = heard(&mut player, &woken, 2);
    assert!(
        matches!(said.as_slice(), [Heard::Playing(_), Heard::Ended(_)]),
        "{said:?}"
    );
    assert!(
        began.elapsed() >= Duration::from_millis(100),
        "{:?}",
        began.elapsed()
    );
    assert_eq!(record(&recorded).started, vec![vec![0.5], vec![0.5]]);
}

/// "Zero means repeat forever": the passes go on until the sound is stopped, and the stop is said.
#[test]
fn a_repeat_count_of_zero_plays_until_it_is_stopped() {
    let (mut player, recorded, woken) = player(Device::Ends);
    player.play(played(
        &FOUR,
        Playing {
            repeat: 0.0,
            ..Playing::default()
        },
    ));
    let first = heard(&mut player, &woken, 1);
    assert!(
        matches!(first.as_slice(), [Heard::Playing(playing)] if playing.contains("until it is stopped")),
        "{first:?}"
    );
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(30))
        .unwrap_or_else(Instant::now);
    while record(&recorded).started.len() < 3 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    player.stop();
    let stopped = heard(&mut player, &woken, 1);
    assert!(
        matches!(stopped.as_slice(), [Heard::Stopped(_)]),
        "{stopped:?}"
    );
    assert!(record(&recorded).started.len() >= 3);
}

/// The media clock's master is the device's count: a device that has taken two of a sound's four
/// frames at 8000 Hz puts the sound at 0.25 ms, whatever the processor's clock says; the clock is
/// gone once the sound is stopped.
#[test]
fn the_clock_is_the_device_s_count() {
    let (mut player, _, woken) = player(Device::Halfway);
    assert_eq!(player.position(), None);
    player.play(played(&FOUR, Playing::default()));
    let said = heard(&mut player, &woken, 1);
    assert!(matches!(said.as_slice(), [Heard::Playing(_)]), "{said:?}");
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(30))
        .unwrap_or_else(Instant::now);
    while player.position() != Some(Duration::from_micros(250)) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(player.position(), Some(Duration::from_micros(250)));
    player.stop();
    let stopped = heard(&mut player, &woken, 1);
    assert!(
        matches!(stopped.as_slice(), [Heard::Stopped(_)]),
        "{stopped:?}"
    );
    assert_eq!(player.position(), None);
}

/// Table 291's `/A` false holds the sound at its start until it is resumed, and the device is
/// handed nothing before then.
#[test]
fn a_sound_that_does_not_play_at_once_is_held_until_resumed() {
    let (mut player, recorded, woken) = player(Device::Ends);
    player.play(played(
        &FOUR,
        Playing {
            auto_play: false,
            ..Playing::default()
        },
    ));
    let held = heard(&mut player, &woken, 1);
    assert!(
        matches!(held.as_slice(), [Heard::Paused(said)] if said.contains("/A is false")),
        "{held:?}"
    );
    assert_eq!(record(&recorded).started, Vec::<Vec<f32>>::new());
    player.resume();
    let resumed = heard(&mut player, &woken, 2);
    assert!(
        matches!(resumed.as_slice(), [Heard::Playing(_), Heard::Ended(_)]),
        "{resumed:?}"
    );
    assert_eq!(record(&recorded).started.len(), 1);
}

/// A document's sound action, end to end: Table 212's `/Volume 0.5 /Synchronous true` crosses
/// `viewer_core`'s `Event::Media` with the samples, the level `open` plays it here, the device is
/// handed `±¼`, and the window's [`Sounds`] holds the reader until Escape cancels the sound (ADR
/// 3343) — the one key it takes.
#[test]
fn a_sound_action_s_parameters_reach_the_device_and_hold_the_reader() {
    let document = assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Link /Rect [0 0 9 9] /A << /S /Sound /Sound 5 0 R \
         /Volume 0.5 /Synchronous true >> >>",
        "<< /Type /Sound /R 8000 /E /Signed /Filter /ASCIIHexDecode /Length 9 >>\nstream\n\
         40C040C0>\nendstream",
    ]);
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::HandOverMedia, RestrictionLevel::Off),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: document.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    let Some((media, sound)) = events.into_iter().find_map(|event| match event {
        Event::Media { media, sound, .. } => Some((media, sound.map(|sound| *sound))),
        _ => None,
    }) else {
        panic!("the sound action hands its sound over");
    };
    let Handing::Hand {
        handed: Handed::Here { name, sound },
        ..
    } = viewer_host::handing_here("doc.pdf", None, None, media, sound, MediaFiles::Open)
    else {
        panic!("at open a sound holding its samples is played here");
    };
    let kept = Arc::new(Mutex::new(Record::default()));
    let shared = Arc::clone(&kept);
    let mut sounds = Sounds::opening_with(Box::new(move || {
        Ok(Box::new(Recording {
            record: Arc::clone(&shared),
            device: Device::Halfway,
        }) as Box<dyn Sink>)
    }));
    sounds.play(
        Played {
            document: DocumentId(1),
            label: name,
            sound,
        },
        || Box::new(|| {}),
    );
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(30))
        .unwrap_or_else(Instant::now);
    let mut said = Vec::new();
    while !sounds.holds_the_reader() && Instant::now() < deadline {
        said.extend(sounds.heard());
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(sounds.holds_the_reader(), "{said:?}");
    assert_eq!(record(&kept).started, vec![vec![0.25, -0.25, 0.25, -0.25]]);
    assert!(sounds.stop(), "Escape cancels the sound");
    while sounds.holds_the_reader() && Instant::now() < deadline {
        sounds.heard();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!sounds.holds_the_reader());
    sounds.end();
}

/// A rendition's RIFF WAVE clip, end to end (ADRs 3349 to 3351): a host that plays here has the
/// rendition judged for its own player, which honours Table 291's `MH` `/V 50` and `/A false`; the
/// file crosses `Event::Media` whole, the window's [`Sounds`] holds it at its start and its title
/// says Space plays it; Space — [`Sounds::resume`] — starts it, and the device is handed the
/// file's sixteen-bit `±½` at half its level, `±¼`.
#[test]
fn a_rendition_s_wave_clip_is_held_at_its_start_until_resumed() {
    // RIFF, 40 bytes of WAVE: a 16-byte fmt chunk (PCM, mono, 8000 Hz, 16000 bytes a second, a
    // two-byte block, 16 bits) and a 4-byte data chunk of 0x4000 and 0xC000, least significant
    // byte first.
    let wave = "52494646280000005741564566 6D74201000000001000100401F0000803E0000020010006461746104000000004000C0>";
    let file = format!(
        "<< /Type /EmbeddedFile /Filter /ASCIIHexDecode /Length {} >>\nstream\n{wave}\nendstream",
        wave.len()
    );
    let document = assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Screen /P 3 0 R /Rect [0 0 9 9] /A 5 0 R >>",
        "<< /Type /Action /S /Rendition /OP 0 /AN 4 0 R /R 6 0 R >>",
        "<< /Type /Rendition /S /MR /N (chime) /C 7 0 R /P << /MH << /V 50 /A false >> >> >>",
        "<< /Type /MediaClip /S /MCD /CT (audio/wav) /D 8 0 R >>",
        "<< /Type /Filespec /F (chime.wav) /UF (chime.wav) /EF << /F 9 0 R >> >>",
        &file,
    ]);
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer.handle(Command::PlaysHere(true)).for_each(drop);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: document.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    let Some((media, sound)) = events.iter().find_map(|event| match event {
        Event::Media { media, sound, .. } => Some((media.clone(), sound.clone().map(|s| *s))),
        _ => None,
    }) else {
        panic!("the rendition hands its clip over: {events:?}");
    };
    let Handing::Hand {
        handed: Handed::Here { name, sound },
        ..
    } = viewer_host::handing_here(
        "doc.pdf",
        None,
        Some("audio/wav"),
        media,
        sound,
        MediaFiles::Open,
    )
    else {
        panic!("at open a WAVE clip judged for this player is played here");
    };
    let kept = Arc::new(Mutex::new(Record::default()));
    let shared = Arc::clone(&kept);
    let mut sounds = Sounds::opening_with(Box::new(move || {
        Ok(Box::new(Recording {
            record: Arc::clone(&shared),
            device: Device::Halfway,
        }) as Box<dyn Sink>)
    }));
    assert!(!sounds.resume(), "nothing is held before a sound");
    sounds.play(
        Played {
            document: DocumentId(1),
            label: name,
            sound,
        },
        || Box::new(|| {}),
    );
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(30))
        .unwrap_or_else(Instant::now);
    let mut said = Vec::new();
    while sounds.title() != Some(viewer_host::sound::PAUSED) && Instant::now() < deadline {
        said.extend(sounds.heard());
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(sounds.title(), Some(viewer_host::sound::PAUSED), "{said:?}");
    assert!(
        record(&kept).started.is_empty(),
        "held, so the device has nothing"
    );
    assert!(sounds.resume(), "Space plays the sound held at its start");
    while record(&kept).started.is_empty() && Instant::now() < deadline {
        sounds.heard();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(record(&kept).started, vec![vec![0.25, -0.25]]);
    sounds.end();
}

/// Assembles a document from object bodies, with object 1 the catalog.
fn assembled(objects: &[&str]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(
            format!("{} 0 obj\n{body}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}
