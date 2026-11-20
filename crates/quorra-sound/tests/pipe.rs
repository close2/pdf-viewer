//! The window's half and the helper's half over real pipes: once in process with a recording device,
//! and once with the helper program itself, on whatever device this machine has — the build machine
//! has none, so that run proves the program's protocol and its refusal, never a sound (ADR 3317).

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use quorra_sound::{Device, Format, Helper, HelperError};

/// What the recording device saw.
#[derive(Debug, Default)]
struct Record {
    started: Vec<Vec<f32>>,
    stops: usize,
}

/// The device's double: it ends a sound at once when `ends` holds, and refuses an empty one.
struct Recording {
    record: Arc<Mutex<Record>>,
    ends: bool,
}

impl Device for Recording {
    fn format(&self) -> Format {
        Format {
            rate: 44_100,
            channels: 2,
        }
    }

    fn start(&mut self, samples: Vec<f32>, ended: Box<dyn FnOnce() + Send>) -> Result<(), String> {
        if samples.is_empty() {
            return Err("nothing to play".to_owned());
        }
        recorded(&self.record).started.push(samples);
        if self.ends {
            ended();
        }
        Ok(())
    }

    fn stop(&mut self) {
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
    };
    let serving = std::thread::spawn(move || {
        quorra_sound::serve(orders_read, replies_write, move || Ok(device))
    });
    let helper = Helper::over(orders_write, replies_read)?;
    Ok((helper, record, serving))
}

fn ended_into(sender: std::sync::mpsc::Sender<u32>, which: u32) -> Box<dyn FnOnce() + Send> {
    Box::new(move || {
        let _ = sender.send(which);
    })
}

fn next(ends: &Receiver<u32>) -> Option<u32> {
    ends.recv_timeout(Duration::from_secs(10)).ok()
}

#[test]
fn a_sound_crosses_whole_and_its_end_comes_back() {
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
    let (ends, ended) = channel();
    helper
        .play(&samples, ended_into(ends.clone(), 1))
        .expect("started");
    assert_eq!(next(&ended), Some(1));
    helper
        .play(&[0.5, -0.5], ended_into(ends, 2))
        .expect("started");
    assert_eq!(next(&ended), Some(2));
    drop(helper);
    serving
        .join()
        .expect("the helper's half ended at the pipe's end")
        .expect("every order was read");
    let record = recorded(&record);
    assert_eq!(record.started, vec![samples, vec![0.5, -0.5]]);
    // One stop before each sound, and one at the end of the input.
    assert_eq!(record.stops, 3);
}

#[test]
fn a_stop_is_carried_and_a_stopped_sound_s_end_is_not_called() {
    let (mut helper, record, serving) = served(false).expect("served");
    let (ends, ended) = channel();
    helper
        .play(&[0.25; 64], ended_into(ends, 1))
        .expect("started");
    helper.stop();
    drop(helper);
    serving
        .join()
        .expect("ended")
        .expect("every order was read");
    assert_eq!(ended.try_recv().ok(), None);
    assert_eq!(recorded(&record).stops, 3);
}

#[test]
fn a_sound_the_device_refuses_is_said() {
    let (mut helper, _record, serving) = served(true).expect("served");
    let (ends, ended) = channel();
    let refused = helper.play(&[], ended_into(ends.clone(), 1));
    assert!(
        matches!(&refused, Err(HelperError::Refused(why)) if why == "nothing to play"),
        "{refused:?}"
    );
    // The helper is still there for the next sound.
    helper.play(&[1.0], ended_into(ends, 2)).expect("started");
    assert_eq!(next(&ended), Some(2));
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
