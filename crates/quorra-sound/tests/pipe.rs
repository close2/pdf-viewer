//! The window's half and the helper's half over real pipes: once in process with a recording device,
//! and once with the helper program itself, on whatever device this machine has — the build machine
//! has none, so that run proves the program's protocol and its refusal, never a sound (ADR 3317).

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use quorra_sound::{Device, Format, Helper, HelperError, Pass, Progress, Report};

/// What the recording device saw.
#[derive(Debug, Default)]
struct Record {
    /// Each pass's samples, read from the one copy the helper holds.
    started: Vec<Vec<f32>>,
    stops: usize,
    pauses: usize,
}

/// The frames the double says the device had reached at a pause.
const REACHED: u64 = 7;

/// The device's double: when `ends` holds it plays a pass as fast as its samples arrive — on a
/// thread of its own, as a device does, so that a pass can start before its sound is whole — and
/// reports half its frames, then all of them, then its end. It refuses a pass of no frames.
struct Recording {
    record: Arc<Mutex<Record>>,
    ends: bool,
    playing: Option<std::thread::JoinHandle<()>>,
}

impl Device for Recording {
    fn format(&self) -> Format {
        Format {
            rate: 44_100,
            channels: 2,
        }
    }

    fn start(&mut self, pass: Pass, mut report: Report) -> Result<(), String> {
        if pass.frames == 0 {
            return Err("nothing to play".to_owned());
        }
        let from = usize::try_from(pass.from)
            .unwrap_or(usize::MAX)
            .saturating_mul(2);
        let end = usize::try_from(pass.frames)
            .unwrap_or(usize::MAX)
            .saturating_mul(2)
            .saturating_add(from);
        let mut reader = pass.held.reader(from, end);
        let record = Arc::clone(&self.record);
        if !self.ends {
            recorded(&record).started.push(Vec::new());
            return Ok(());
        }
        self.playing = Some(std::thread::spawn(move || {
            let mut samples = Vec::new();
            let mut buffer = [0.0_f32; 64];
            let deadline = std::time::Instant::now().checked_add(Duration::from_secs(10));
            loop {
                let read = reader.read(&mut buffer);
                samples.extend_from_slice(buffer.get(..read.copied).unwrap_or_default());
                if read.over || deadline.is_none_or(|deadline| std::time::Instant::now() > deadline)
                {
                    break;
                }
                if read.copied == 0 {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            let frames = u64::try_from(samples.len() / 2).unwrap_or(u64::MAX);
            recorded(&record).started.push(samples);
            report(Progress::Played(frames / 2));
            report(Progress::Played(frames));
            report(Progress::Ended);
        }));
        Ok(())
    }

    fn pause(&mut self) -> Option<u64> {
        let mut record = recorded(&self.record);
        record.pauses = record.pauses.saturating_add(1);
        Some(REACHED)
    }

    fn stop(&mut self) {
        if let Some(playing) = self.playing.take() {
            let _ = playing.join();
        }
        let mut record = recorded(&self.record);
        record.stops = record.stops.saturating_add(1);
    }
}

/// The record, read.
fn recorded(record: &Arc<Mutex<Record>>) -> std::sync::MutexGuard<'_, Record> {
    record.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The helper's half, serving on a thread.
type Serving = std::thread::JoinHandle<Result<(), quorra_sound::wire::WireError>>;

/// A helper, its device's record, and its serving thread.
type Served = (Helper, Arc<Mutex<Record>>, Serving);

/// A helper served on a thread over two pipes, and what its device recorded.
fn served(ends: bool) -> Result<Served, Box<dyn std::error::Error>> {
    let (orders_read, orders_write) = std::io::pipe()?;
    let (replies_read, replies_write) = std::io::pipe()?;
    let record = Arc::new(Mutex::new(Record::default()));
    let device = Recording {
        record: Arc::clone(&record),
        ends,
        playing: None,
    };
    let serving = std::thread::spawn(move || {
        quorra_sound::serve(orders_read, replies_write, move || Ok(device))
    });
    let helper = Helper::over(orders_write, replies_read)?;
    Ok((helper, record, serving))
}

/// A report that sends `which` with every count of frames, and with `u64::MAX` at the end.
fn ended_into(sender: std::sync::mpsc::Sender<(u32, u64)>, which: u32) -> Report {
    Box::new(move |said| {
        let _ = sender.send(match said {
            Progress::Played(frames) => (which, frames),
            Progress::Ended => (which, u64::MAX),
        });
    })
}

/// The next sound to end, with the frame counts reported for it on the way.
fn next(ends: &Receiver<(u32, u64)>) -> Option<(u32, Vec<u64>)> {
    let mut played = Vec::new();
    loop {
        match ends.recv_timeout(Duration::from_secs(10)).ok()? {
            (which, u64::MAX) => return Some((which, played)),
            (_, frames) => played.push(frames),
        }
    }
}

/// The sound whole, then complete: what [`Helper::load`], [`Helper::append`] and
/// [`Helper::complete`] send for a sound already in hand.
fn sent(helper: &mut Helper, samples: &[f32]) -> Result<(), HelperError> {
    helper.load()?;
    helper.append(samples)?;
    helper.complete()
}

#[test]
fn a_sound_is_held_once_and_each_pass_reads_it() {
    let (mut helper, record, serving) = served(true).expect("served");
    assert_eq!(
        helper.format(),
        Format {
            rate: 44_100,
            channels: 2
        }
    );
    let samples: Vec<f32> = (0..100_000_u32)
        .map(|n| if n % 2 == 0 { 0.25 } else { -0.25 })
        .collect();
    sent(&mut helper, &samples).expect("sent");
    let (ends, ended) = channel();
    helper
        .play(0, u64::MAX, ended_into(ends.clone(), 1))
        .expect("started");
    // 100,000 samples of two channels are 50,000 frames: the device's counts cross the wire as
    // the media clock reads them (ADR 3342).
    assert_eq!(next(&ended), Some((1, vec![25_000, 50_000])));
    // A repeat is a pass over the same copy, which nothing sends again (ADR 3457).
    helper
        .play(49_999, 10, ended_into(ends.clone(), 2))
        .expect("started");
    assert_eq!(next(&ended), Some((2, vec![0, 1])));
    sent(&mut helper, &[0.5, -0.5]).expect("sent");
    helper
        .play(0, u64::MAX, ended_into(ends, 3))
        .expect("started");
    assert_eq!(next(&ended), Some((3, vec![0, 1])));
    drop(helper);
    serving
        .join()
        .expect("the helper's half ended at the pipe's end")
        .expect("every order was read");
    let record = recorded(&record);
    assert_eq!(
        record.started,
        vec![samples, vec![0.25, -0.25], vec![0.5, -0.5]]
    );
}

/// A pass starts on the first run and plays the runs that follow as they arrive; it ends only once
/// the sound is said complete (ADR 3457).
#[test]
fn a_pass_starts_before_its_sound_is_whole() {
    let (mut helper, record, serving) = served(true).expect("served");
    helper.load().expect("a new sound");
    helper.append(&[0.1, 0.2]).expect("the first run");
    let (ends, ended) = channel();
    helper
        .play(0, u64::MAX, ended_into(ends, 1))
        .expect("started on the first run");
    helper
        .append(&[0.3, 0.4, 0.5, 0.6])
        .expect("the second run");
    helper.complete().expect("its end");
    assert_eq!(next(&ended), Some((1, vec![1, 3])));
    drop(helper);
    serving
        .join()
        .expect("ended")
        .expect("every order was read");
    assert_eq!(
        recorded(&record).started,
        vec![vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6]]
    );
}

#[test]
fn a_stop_is_carried_and_a_stopped_sound_s_end_is_not_called() {
    let (mut helper, record, serving) = served(false).expect("served");
    sent(&mut helper, &[0.25; 64]).expect("sent");
    let (ends, ended) = channel();
    helper
        .play(0, u64::MAX, ended_into(ends, 1))
        .expect("started");
    helper.stop();
    drop(helper);
    serving
        .join()
        .expect("ended")
        .expect("every order was read");
    assert_eq!(ended.try_recv().ok(), None);
    // One at the new sound, one before the pass, the order's, and one at the end of the input.
    assert_eq!(recorded(&record).stops, 4);
}

/// A pause stops the pass and brings back the frame the device reached; a pause with nothing
/// sounding brings back nothing (ADR 3458).
#[test]
fn a_pause_says_where_the_device_reached() {
    let (mut helper, record, serving) = served(false).expect("served");
    sent(&mut helper, &[0.25; 64]).expect("sent");
    let (ends, ended) = channel();
    helper
        .play(0, u64::MAX, ended_into(ends, 1))
        .expect("started");
    assert_eq!(helper.pause().expect("answered"), Some(REACHED));
    assert_eq!(helper.pause().expect("answered"), None);
    drop(helper);
    serving
        .join()
        .expect("ended")
        .expect("every order was read");
    assert_eq!(ended.try_recv().ok(), None);
    assert_eq!(recorded(&record).pauses, 2);
}

#[test]
fn a_sound_the_device_refuses_is_said() {
    let (mut helper, _record, serving) = served(true).expect("served");
    sent(&mut helper, &[1.0, 1.0]).expect("sent");
    let (ends, ended) = channel();
    let refused = helper.play(0, 0, ended_into(ends.clone(), 1));
    assert!(
        matches!(&refused, Err(HelperError::Refused(why)) if why == "nothing to play"),
        "{refused:?}"
    );
    // The helper is still there for the next pass.
    helper
        .play(0, u64::MAX, ended_into(ends, 2))
        .expect("started");
    assert_eq!(next(&ended), Some((2, vec![0, 1])));
    drop(helper);
    serving
        .join()
        .expect("ended")
        .expect("every order was read");
}

#[test]
fn a_device_that_will_not_open_is_said_and_the_helper_ends() {
    let (orders_read, orders_write) = std::io::pipe().expect("a pipe");
    let (replies_read, replies_write) = std::io::pipe().expect("a pipe");
    let serving = std::thread::spawn(move || {
        quorra_sound::serve(orders_read, replies_write, || {
            Err::<Recording, _>("no device here".to_owned())
        })
    });
    let opened = Helper::over(orders_write, replies_read);
    assert!(
        matches!(&opened, Err(HelperError::NoDevice(why)) if why == "no device here"),
        "{opened:?}"
    );
    serving
        .join()
        .expect("ended")
        .expect("every order was read");
}

/// The program itself: it either opens this machine's device and says its format, or says why it
/// could not — and with its input closed it exits either way.
#[test]
fn the_helper_program_answers_over_its_pipes() {
    let program = PathBuf::from(env!("CARGO_BIN_EXE_quorra-sound"));
    match Helper::spawn(&program) {
        Ok(helper) => {
            let format = helper.format();
            assert!(format.rate > 0 && format.channels > 0, "{format:?}");
        }
        Err(HelperError::NoDevice(why)) => assert!(why.chars().next().is_some()),
        Err(other) => panic!("the helper neither opened a device nor said why not: {other}"),
    }
}

#[test]
fn a_missing_helper_is_said_by_name() {
    let missing = std::env::temp_dir().join("quorra-sound-that-is-not-there");
    let error = Helper::spawn(&missing).expect_err("there is no such program");
    assert!(matches!(error, HelperError::Missing { .. }), "{error:?}");
    assert!(error.to_string().contains("was not found"), "{error}");
}

/// What a machine without `libasound.so.2` does to the helper: its loader exits with 127 before
/// `main`. A stand-in that exits the same way is said as a missing library.
#[cfg(unix)]
#[test]
fn a_helper_the_loader_refuses_is_said_as_a_missing_library() {
    use std::os::unix::fs::PermissionsExt;
    let directory =
        std::env::temp_dir().join(format!("quorra-sound-loader-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a directory");
    let stand_in = directory.join("quorra-sound");
    std::fs::write(&stand_in, "#!/bin/sh\nexit 127\n").expect("written");
    std::fs::set_permissions(&stand_in, std::fs::Permissions::from_mode(0o755))
        .expect("made runnable");
    let error = Helper::spawn(&stand_in).expect_err("the stand-in says nothing");
    let _ = std::fs::remove_dir_all(&directory);
    assert!(
        matches!(error, HelperError::Ended(Some(status)) if status.code() == Some(127)),
        "{error:?}"
    );
    assert!(error.to_string().contains("libasound.so.2"), "{error}");
}

/// The positive half of `viewer-ui`'s `no_audio_library` test: the helper is where `libasound` is
/// linked, so the same reading of the dynamic section finds it here.
#[cfg(target_os = "linux")]
#[test]
fn the_helper_is_where_libasound_is_linked() {
    let program = env!("CARGO_BIN_EXE_quorra-sound");
    let dynamic = std::process::Command::new("readelf")
        .args(["-d", program])
        .output()
        .expect("readelf runs (binutils)");
    assert!(dynamic.status.success());
    let text = String::from_utf8_lossy(&dynamic.stdout);
    assert!(
        text.lines()
            .any(|line| line.contains("(NEEDED)") && line.contains("libasound")),
        "{text}"
    );
}
