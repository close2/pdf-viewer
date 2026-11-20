//! ISO 32000-2 §13.3's sound played in the window, end to end with a recording sink standing in for
//! the audio device: the document's sound object, `viewer_core`'s `Event::Media`, the reader's
//! level, the confined media worker, the conversion to the device's format, and what reached the
//! device (RFC 0015 section 5, ADR 3301).
//!
//! **No speaker is in this test, and no person has heard a sound through this path**: the build
//! machine has no sound card. What is proved is that the samples the device would be handed are
//! the ones §13.3's arithmetic gives, and that the player's orders end what they should.
//!
//! The worker is the program beside the test binary: `cargo build -p pdf-media-worker --bins`
//! first, as `pdf-sandbox`'s worker is built before `pdf-model`'s tests (trap 10).

use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pdf_model::restriction::Operation;
use viewer_core::{
    Command, DocumentId, Event, RestrictionLevel, RestrictionPolicy, RestrictionScope, SoundData,
    Viewer,
};
use viewer_host::sound::{DeviceFormat, Heard, Played, Player, Progress, Report, Sink};
use viewer_host::{Handed, Handing, MediaFiles};

/// What the recording sink saw.
#[derive(Debug, Default)]
struct Record {
    /// How many times the device was opened.
    opened: usize,
    /// Every sound started, in order.
    started: Vec<Vec<f32>>,
    /// How many stops it was asked for.
    stops: usize,
}

/// The audio device's double: a format, and a record of what reached it.
struct Recording {
    format: DeviceFormat,
    record: Arc<Mutex<Record>>,
    /// Whether a sound reaches its end as soon as it starts.
    ends: bool,
}

impl Sink for Recording {
    fn format(&self) -> DeviceFormat {
        self.format
    }

    fn start(&mut self, samples: Vec<f32>, mut report: Report) -> Result<(), String> {
        let frames = u64::try_from(
            samples
                .len()
                .checked_div(usize::from(self.format.channels))
                .unwrap_or_default(),
        )
        .unwrap_or(u64::MAX);
        self.record
            .lock()
            .map_err(|_| "the record is poisoned".to_owned())?
            .started
            .push(samples);
        // The device's count, as the sound helper reports it: half the frames, then all of them,
        // then the end — the media clock's master (ADR 3342).
        if self.ends {
            report(Progress::Played(frames / 2));
            report(Progress::Played(frames));
            report(Progress::Ended);
        }
        Ok(())
    }

    fn stop(&mut self) {
        if let Ok(mut record) = self.record.lock() {
            record.stops = record.stops.saturating_add(1);
        }
    }
}

/// A player over a recording sink of `format`, and what wakes the test.
fn player(format: DeviceFormat, ends: bool) -> (Player, Arc<Mutex<Record>>, Receiver<()>) {
    let record = Arc::new(Mutex::new(Record::default()));
    let shared = Arc::clone(&record);
    let (wake, woken) = channel();
    let player = Player::new(
        Box::new(move || {
            if let Ok(mut record) = shared.lock() {
                record.opened = record.opened.saturating_add(1);
            }
            Ok(Box::new(Recording {
                format,
                record: Arc::clone(&shared),
                ends,
            }) as Box<dyn Sink>)
        }),
        Box::new(move || {
            let _ = wake.send(());
        }),
    );
    (player, record, woken)
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

/// A page with a sound annotation (object 4) whose sound object (object 5) holds two 8-bit
/// `Signed` samples at 8000 Hz, mono: `0x40` and `0xC0`, half of full scale up and down (Table
/// 305; §13.3's "Twos-complement values").
fn a_sounding_document() -> Vec<u8> {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Sound /Rect [0 0 9 9] /Sound 5 0 R >>",
        "<< /Type /Sound /R 8000 /E /Signed /Filter /ASCIIHexDecode /Length 5 >>\nstream\n\
         40C0>\nendstream",
    ])
}

/// The events of activating the sound annotation, under a core level that withholds nothing.
fn activated() -> Vec<Event> {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::HandOverMedia, RestrictionLevel::Off),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: a_sounding_document().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect()
}

/// A sound played directly, without a document: `samples` 8-bit `Signed` at 8000 Hz, mono.
fn played(document: u64, samples: &[u8]) -> Played {
    Played {
        document: DocumentId(document),
        label: format!("sound {document}"),
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
            playing: pdf_model::media::Playing::default(),
        },
    }
}

/// The record, read.
fn record(record: &Arc<Mutex<Record>>) -> std::sync::MutexGuard<'_, Record> {
    match record.lock() {
        Ok(record) => record,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// The whole path. `viewer_core` carries the sound object's samples beside the written file; the
/// level `open` plays it here; the worker decodes `0x40, 0xC0` to `+½, −½` of full scale; the
/// conversion raises 8000 Hz mono to the device's 16,000 Hz stereo — four frames, the second
/// halfway between the first two, the last held, each on both channels; and the device is opened
/// once, at this first sound. The wrong answers excluded: the written WAVE file's bytes decoded as
/// samples, a silent right channel, and every other frame repeated rather than interpolated.
#[test]
fn a_document_s_sound_is_decoded_converted_and_handed_to_the_device() {
    let events = activated();
    let Some((media, sound)) = events.into_iter().find_map(|event| match event {
        Event::Media { media, sound, .. } => Some((media, sound.map(|sound| *sound))),
        _ => None,
    }) else {
        panic!("the sound annotation hands its sound over");
    };
    let handing = viewer_host::handing_here("doc.pdf", None, None, media, sound, MediaFiles::Open);
    let Handing::Hand {
        handed: Handed::Here { name, sound },
        note: None,
    } = handing
    else {
        panic!("at open a sound holding its samples is played here: {handing:?}");
    };
    let (mut player, recorded, woken) = player(
        DeviceFormat {
            rate: 16_000,
            channels: 2,
        },
        false,
    );
    assert!(!player.started(), "made, the player has started nothing");
    assert_eq!(record(&recorded).opened, 0);
    player.play(Played {
        document: DocumentId(1),
        label: name,
        sound,
    });
    let said = heard(&mut player, &woken, 1);
    assert!(
        matches!(said.as_slice(), [Heard::Playing(_)]),
        "the worker must be built: `cargo build -p pdf-media-worker --bins`; {said:?}"
    );
    let record = record(&recorded);
    assert_eq!(record.opened, 1);
    assert_eq!(
        record.started,
        vec![vec![0.5, 0.5, 0.0, 0.0, -0.5, -0.5, -0.5, -0.5]]
    );
}

/// A second sound stops the first (Table 210's `/Mix` at its default); closing another document
/// leaves the sound; closing its own stops it; a stop with nothing sounding says nothing; and the
/// device is opened once for them all.
#[test]
fn a_second_sound_stops_the_first_and_its_document_closing_stops_it() {
    let (mut player, recorded, woken) = player(
        DeviceFormat {
            rate: 8000,
            channels: 1,
        },
        false,
    );
    player.play(played(1, &[0x40]));
    let first = heard(&mut player, &woken, 1);
    assert!(matches!(first.as_slice(), [Heard::Playing(_)]), "{first:?}");
    player.play(played(2, &[0xC0]));
    let second = heard(&mut player, &woken, 2);
    assert!(
        matches!(second.as_slice(), [Heard::Stopped(said), Heard::Playing(_)] if said.contains("sound 1")),
        "{second:?}"
    );
    player.closed(DocumentId(1));
    player.closed(DocumentId(2));
    let closed = heard(&mut player, &woken, 1);
    assert!(
        matches!(closed.as_slice(), [Heard::Stopped(said)] if said.contains("sound 2")),
        "{closed:?}"
    );
    player.stop();
    drop(player);
    let record = record(&recorded);
    assert_eq!(record.opened, 1);
    assert_eq!(record.started, vec![vec![0.5], vec![-0.5]]);
}

/// A sound that reaches its end is said to have ended, and a stop after it says nothing more.
#[test]
fn a_sound_at_its_end_is_said_to_have_ended() {
    let (mut player, _, woken) = player(
        DeviceFormat {
            rate: 8000,
            channels: 1,
        },
        true,
    );
    player.play(played(1, &[0x40, 0x00]));
    let said = heard(&mut player, &woken, 2);
    assert!(
        matches!(said.as_slice(), [Heard::Playing(_), Heard::Ended(_)]),
        "{said:?}"
    );
    player.stop();
    player.play(played(1, &[0x40]));
    let again = heard(&mut player, &woken, 2);
    assert!(
        matches!(again.as_slice(), [Heard::Playing(_), Heard::Ended(_)]),
        "no stop is said for a sound that had ended: {again:?}"
    );
}

/// A sound the media worker's client refuses — Table 305's `/B` past 32 bits — is said, and the
/// device is not opened for it: the decode comes before the opening.
#[test]
fn a_refused_sound_is_said_and_opens_no_device() {
    let (mut player, recorded, woken) = player(
        DeviceFormat {
            rate: 8000,
            channels: 1,
        },
        false,
    );
    let mut refused = played(1, &[0; 8]);
    if let viewer_core::Recording::Samples { format, .. } = &mut refused.sound.recording {
        format.bits = 33;
    }
    player.play(refused);
    let said = heard(&mut player, &woken, 1);
    assert!(
        matches!(said.as_slice(), [Heard::Refused(why)] if why.contains("was not played")),
        "{said:?}"
    );
    assert_eq!(record(&recorded).opened, 0);
}

/// A device that will not open is said, and the next sound tries it again.
#[test]
fn a_device_that_will_not_open_is_said_and_tried_again() {
    let (wake, woken) = channel();
    let attempts = Arc::new(Mutex::new(0_usize));
    let counted = Arc::clone(&attempts);
    let mut player = Player::new(
        Box::new(move || {
            if let Ok(mut attempts) = counted.lock() {
                *attempts = attempts.saturating_add(1);
            }
            Err("this machine offers no audio output device".to_owned())
        }),
        Box::new(move || {
            let _ = wake.send(());
        }),
    );
    player.play(played(1, &[0x40]));
    player.play(played(1, &[0x40]));
    let said = heard(&mut player, &woken, 2);
    assert!(
        said.iter().all(
            |heard| matches!(heard, Heard::Refused(why) if why.contains("no audio output device"))
        ),
        "{said:?}"
    );
    assert_eq!(said.len(), 2);
    assert_eq!(
        *attempts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        2
    );
}

/// The level over a sound played here: `refuse` says so and plays nothing, `ask` puts the question
/// with the play-here wording, `warn` plays it and says so afterwards. A clip with no samples of
/// its own is [`viewer_host::handing`]'s, written for this machine's player as before.
#[test]
fn the_four_levels_answer_a_sound_played_here() {
    let sound = played(1, &[0x40]).sound;
    let media = || viewer_core::Media::Embedded {
        name: "sound.wav".to_owned(),
        bytes: b"RIFF".to_vec(),
    };
    let at = |level, sound| viewer_host::handing_here("doc.pdf", None, None, media(), sound, level);
    assert!(matches!(
        at(MediaFiles::Refuse, Some(sound.clone())),
        Handing::Refuse(said) if said.contains("not played") && said.contains("--media-files=refuse")
    ));
    assert!(matches!(
        at(MediaFiles::Ask, Some(sound.clone())),
        Handing::Ask { handed: Handed::Here { .. }, question }
            if question.reasons.contains("play it in this window")
    ));
    assert!(matches!(
        at(MediaFiles::Warn, Some(sound.clone())),
        Handing::Hand { handed: Handed::Here { .. }, note: Some(note) } if note.contains("played without asking")
    ));
    assert!(matches!(
        at(MediaFiles::Open, None),
        Handing::Hand {
            handed: Handed::Written { .. },
            note: None
        }
    ));
}

/// A window whose sound helper is missing says so for the sound, opens nothing, and goes on: the
/// next sound is tried again, and the player is still there for it (ADR 3317).
#[test]
fn a_missing_sound_helper_is_said_and_the_player_goes_on() {
    let missing = std::env::temp_dir().join("quorra-sound-that-is-not-there");
    let (wake, woken) = channel();
    let mut player = Player::new(
        Box::new(move || viewer_host::sound::helper::open_at(&missing)),
        Box::new(move || {
            let _ = wake.send(());
        }),
    );
    for document in [1, 2] {
        player.play(played(document, &[0x00, 0x40, 0x00, 0xC0]));
        let said = heard(&mut player, &woken, 1);
        assert!(
            matches!(said.as_slice(), [Heard::Refused(sentence)]
                if sentence.contains("the audio device: the sound helper")
                    && sentence.contains("was not found")),
            "{said:?}"
        );
    }
    assert!(player.started());
}
