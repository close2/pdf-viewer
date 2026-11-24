//! ISO 32000-2 §12.6.4.14's stop, pause and resume, and Table 218's `/OP` 4, performed by the player
//! on the sound a rendition action associated with a screen annotation (ADR 3433), with a
//! recording sink standing in for the audio device.
//!
//! What is proved is what reached the device and what was said: a pause stops the device's stream
//! and a resume starts the rest of the samples from where the device's count stopped, not from the
//! start; an operation naming another annotation, or a rendition not in the state it acts on, has
//! "no effect" and says so; `/OP` 4 resumes the paused rendition instead of playing a second one
//! and does nothing while it sounds; the rectangle Table 294's `/W` 3 paints stays painted while
//! paused and is cleared by the stop.
//!
//! The worker is the program beside the test binary: `cargo build -p pdf-media-worker --bins`
//! first (trap 10).

use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use pdf_syntax::ObjectId;
use viewer_core::{Associated, Command, DocumentId, MediaOperation, Screen, SoundData};
use viewer_host::sound::{DeviceFormat, Heard, Held, Played, Player, Progress, Report, Sink};

/// The frames the stand-in device says it has played of each start before it is stopped.
const PLAYED: u64 = 2;

/// Every start the device was handed, and how many stops.
#[derive(Debug, Default)]
struct Record {
    started: Vec<Vec<f32>>,
    stops: usize,
    opened: usize,
}

/// The device's double: 8000 Hz mono, reporting [`PLAYED`] frames of each start and never its end,
/// and answering a pause with the frames it says it reached, where it says any.
struct Device(Arc<Mutex<Record>>, Held, Option<u64>);

impl Sink for Device {
    fn format(&self) -> DeviceFormat {
        DeviceFormat {
            rate: 8000,
            channels: 1,
        }
    }

    fn load(&mut self) -> Result<(), String> {
        self.1 = Held::new();
        Ok(())
    }

    fn append(&mut self, samples: &[f32]) -> Result<(), String> {
        self.1.append(samples.to_vec());
        Ok(())
    }

    fn complete(&mut self) -> Result<(), String> {
        self.1.complete();
        Ok(())
    }

    // The double cannot say where a device reached: the resumption is where the reports put it.
    fn pause(&mut self) -> Option<u64> {
        self.2
    }

    fn start(&mut self, from: u64, frames: u64, mut report: Report) -> Result<(), String> {
        let channels = usize::from(1u16);
        let at = |frame: u64| {
            usize::try_from(frame)
                .unwrap_or(usize::MAX)
                .saturating_mul(channels)
        };
        let samples = self.1.copied(at(from), at(from.saturating_add(frames)));
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .started
            .push(samples);
        report(Progress::Played(PLAYED));
        Ok(())
    }

    fn stop(&mut self) {
        let mut record = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        record.stops = record.stops.saturating_add(1);
    }
}

fn player() -> (Player, Arc<Mutex<Record>>, Receiver<()>) {
    player_reaching(None)
}

/// [`player`] over a device that answers a pause with `reached` frames.
fn player_reaching(reached: Option<u64>) -> (Player, Arc<Mutex<Record>>, Receiver<()>) {
    let record = Arc::new(Mutex::new(Record::default()));
    let shared = Arc::clone(&record);
    let (wake, woken) = channel();
    let player = Player::new(
        Box::new(move || {
            let mut record = shared.lock().unwrap_or_else(PoisonError::into_inner);
            record.opened = record.opened.saturating_add(1);
            drop(record);
            Ok(Box::new(Device(Arc::clone(&shared), Held::new(), reached)) as Box<dyn Sink>)
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

const DOCUMENT: DocumentId = DocumentId(1);
const SCREEN: ObjectId = ObjectId {
    number: 4,
    generation: 0,
};
const OTHER: ObjectId = ObjectId {
    number: 9,
    generation: 0,
};

/// Six 8-bit `Signed` samples at 8000 Hz, mono — `+1/8` to `+6/8` of full scale — played for a
/// rendition action naming [`SCREEN`], `/OP` 4 where `or_resume`, in its rectangle.
fn played(or_resume: bool) -> Played {
    Played {
        document: DOCUMENT,
        label: "the rendition".to_owned(),
        sound: SoundData {
            recording: viewer_core::Recording::Samples {
                format: pdf_model::media::SampleFormat {
                    rate: 8000,
                    channels: 1,
                    bits: 8,
                    encoding: pdf_model::media::Encoding::Signed,
                },
                data: vec![0x10, 0x20, 0x30, 0x40, 0x50, 0x60],
            },
            playing: pdf_model::media::Playing::default(),
            screen: Some(Screen {
                annotation: SCREEN,
                background: pdf_model::media::Background::default(),
            }),
            rendition: Some(Associated {
                annotation: SCREEN,
                or_resume,
            }),
            window: None,
        },
    }
}

/// The screen annotations the core is told are painted, in order: `Some(n)` painted, `None`
/// cleared.
fn painted(player: &mut Player) -> Vec<Option<u32>> {
    player
        .repainted()
        .into_iter()
        .filter_map(|command| match command {
            Command::PlayingIn { screen, .. } => {
                Some(screen.map(|screen| screen.annotation.number))
            }
            _ => None,
        })
        .collect()
}

fn one(player: &mut Player, woken: &Receiver<()>) -> Heard {
    let mut said = heard(player, woken, 1);
    assert_eq!(said.len(), 1, "one answer was owed: {said:?}");
    said.pop()
        .unwrap_or_else(|| Heard::Unchanged("nothing was said".to_owned()))
}

/// The whole round of operations on one associated sound.
#[test]
fn a_rendition_is_paused_resumed_where_it_stopped_and_stopped() {
    let (mut player, recorded, woken) = player();
    player.play(played(false));
    let first = one(&mut player, &woken);
    assert!(
        matches!(first, Heard::Playing(_)),
        "the worker must be built: `cargo build -p pdf-media-worker --bins`; {first:?}"
    );
    assert_eq!(painted(&mut player), [Some(4)]);

    // Another annotation's rendition, and a resume of one not paused: no effect, said.
    player.operate(DOCUMENT, OTHER, MediaOperation::Stop);
    let other = one(&mut player, &woken);
    assert!(
        matches!(&other, Heard::Unchanged(said) if said.contains("9 0 R") && said.contains("no effect")),
        "{other:?}"
    );
    player.operate(DocumentId(2), SCREEN, MediaOperation::Pause);
    assert!(matches!(one(&mut player, &woken), Heard::Unchanged(_)));
    player.operate(DOCUMENT, SCREEN, MediaOperation::Resume);
    let unpaused = one(&mut player, &woken);
    assert!(
        matches!(&unpaused, Heard::Unchanged(said) if said.contains("not paused")),
        "{unpaused:?}"
    );

    // Paused after the device counted two frames: the stream stops, the rectangle stays painted.
    player.operate(DOCUMENT, SCREEN, MediaOperation::Pause);
    let paused = one(&mut player, &woken);
    assert!(
        matches!(&paused, Heard::Paused(said) if said.contains("/OP 2")),
        "{paused:?}"
    );
    assert_eq!(player.position(), Some(Duration::from_micros(250)));
    assert!(
        painted(&mut player).is_empty(),
        "a pause keeps the painting"
    );
    player.operate(DOCUMENT, SCREEN, MediaOperation::Pause);
    let again = one(&mut player, &woken);
    assert!(
        matches!(&again, Heard::Unchanged(said) if said.contains("already paused")),
        "{again:?}"
    );

    // Resumed: the rest of the samples, from the third.
    player.operate(DOCUMENT, SCREEN, MediaOperation::Resume);
    let resumed = one(&mut player, &woken);
    assert!(
        matches!(&resumed, Heard::Playing(said) if said.contains("resumed where it was paused")),
        "{resumed:?}"
    );

    // `/OP` 4 while it sounds does nothing; paused, it resumes it rather than playing another.
    player.play(played(true));
    let nothing = one(&mut player, &woken);
    assert!(
        matches!(&nothing, Heard::Unchanged(said) if said.contains("/OP 4")),
        "{nothing:?}"
    );
    player.operate(DOCUMENT, SCREEN, MediaOperation::Pause);
    assert!(matches!(one(&mut player, &woken), Heard::Paused(_)));
    player.play(played(true));
    assert!(matches!(one(&mut player, &woken), Heard::Playing(_)));

    // Stopped: the association removed and the rectangle cleared; a second stop has no effect.
    player.operate(DOCUMENT, SCREEN, MediaOperation::Stop);
    let stopped = one(&mut player, &woken);
    assert!(
        matches!(&stopped, Heard::Stopped(said) if said.contains("/OP 1")),
        "{stopped:?}"
    );
    assert_eq!(painted(&mut player), [None]);
    player.operate(DOCUMENT, SCREEN, MediaOperation::Stop);
    assert!(matches!(one(&mut player, &woken), Heard::Unchanged(_)));
    drop(player);

    let record = recorded.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(record.opened, 1, "a pause keeps the device");
    assert_eq!(
        record.started,
        vec![
            vec![0.125, 0.25, 0.375, 0.5, 0.625, 0.75],
            vec![0.375, 0.5, 0.625, 0.75],
            vec![0.625, 0.75],
        ],
        "each resumption starts where the device's count stopped"
    );
}

/// `/OP` 0 for the annotation in hand stops it and plays the new one; the window has nothing to
/// resume.
#[test]
fn op_zero_replaces_the_rendition_associated_with_the_annotation() {
    let (mut player, recorded, woken) = player();
    player.play(played(false));
    assert!(matches!(one(&mut player, &woken), Heard::Playing(_)));
    player.play(played(false));
    let replaced = heard(&mut player, &woken, 2);
    assert!(
        matches!(replaced.as_slice(), [Heard::Stopped(_), Heard::Playing(_)]),
        "{replaced:?}"
    );
    drop(player);
    assert_eq!(
        recorded
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .started
            .len(),
        2
    );
}

/// A device that says it had reached fewer frames than it had reported written is believed: the
/// clock stands at its count, and the resumption starts at the frame after it, so nothing the
/// device had not played is skipped and nothing it had is played again (ADR 3458).
#[test]
fn a_pause_resumes_at_the_frame_the_device_reached() {
    let (mut player, recorded, woken) = player_reaching(Some(1));
    player.play(played(false));
    assert!(matches!(one(&mut player, &woken), Heard::Playing(_)));
    player.operate(DOCUMENT, SCREEN, MediaOperation::Pause);
    assert!(matches!(one(&mut player, &woken), Heard::Paused(_)));
    // One frame of an 8000 Hz device, where the reports had said two.
    assert_eq!(player.position(), Some(Duration::from_micros(125)));
    player.operate(DOCUMENT, SCREEN, MediaOperation::Resume);
    assert!(matches!(one(&mut player, &woken), Heard::Playing(_)));
    let record = recorded.lock().unwrap_or_else(PoisonError::into_inner);
    let (first, resumed) = (&record.started[0], &record.started[1]);
    assert_eq!(resumed.as_slice(), &first[1..]);
}
