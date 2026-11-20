//! The window's half: start the helper, read the device's format, hand it each sound.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::wire::{self, Format, Reply, WireError};

/// How long the helper has to open the device and say its format, and to answer each sound.
///
/// A choice: opening a device can wait on the sound server, and a server that has not answered in
/// ten seconds is one the person is better told about than waited on. The wait is on the player's
/// thread, never the window's.
const ANSWER_WITHIN: Duration = Duration::from_secs(10);

/// How long a closing helper has to stop its sound and exit before it is killed.
const EXIT_WITHIN: Duration = Duration::from_secs(2);

/// The exit status a loader gives a program it could not start, a missing shared library above
/// all (glibc's `ld.so`, and the shell's convention for "could not execute").
const LOADER_REFUSED: i32 = 127;

/// A sound's end, waited for: its serial and what to call.
type Waiting = Arc<Mutex<Option<(u64, Box<dyn FnOnce() + Send>)>>>;

/// Why the helper could not play a sound.
#[derive(Debug, thiserror::Error)]
pub enum HelperError {
    /// The program is not where it was looked for.
    #[error(
        "the sound helper `{}` was not found; build it with `cargo build -p quorra-sound`, or \
         name it in {}",
        program.display(),
        crate::HELPER_PATH_VARIABLE
    )]
    Missing {
        /// Where it was looked for.
        program: PathBuf,
    },
    /// The program is there and would not start.
    #[error("the sound helper `{}` could not be started: {source}", program.display())]
    Unstarted {
        /// The program.
        program: PathBuf,
        /// The system's reason.
        source: std::io::Error,
    },
    /// The helper ended before it said anything, which a loader does to a program whose library is
    /// missing.
    #[error("{}", ended_sentence(*.0))]
    Ended(Option<ExitStatus>),
    /// The helper could not open the device, and said why.
    #[error("{0}")]
    NoDevice(String),
    /// The device would not start this sound, and the helper said why.
    #[error("{0}")]
    Refused(String),
    /// The helper did not answer within `ANSWER_WITHIN`, ten seconds.
    #[error("the sound helper did not answer within {} seconds", ANSWER_WITHIN.as_secs())]
    Silent,
    /// The helper answered with a reply that does not answer what was asked.
    #[error("the sound helper answered out of turn")]
    OutOfTurn,
    /// The pipe to the helper failed, or the helper sent what this side does not read.
    #[error("the sound helper's pipe: {0}")]
    Wire(#[from] WireError),
}

/// The sentence for a helper that ended unasked, naming the loader's status where it is that.
fn ended_sentence(status: Option<ExitStatus>) -> String {
    match status.and_then(|status| status.code()) {
        Some(LOADER_REFUSED) => format!(
            "the sound helper could not be loaded (exit status {LOADER_REFUSED}): a library it \
             needs is missing on this machine — on Linux that is ALSA's `libasound.so.2` — so no \
             sound is played, and the window works on without it"
        ),
        Some(code) => format!("the sound helper ended with exit status {code}"),
        None => "the sound helper ended".to_owned(),
    }
}

/// A running helper: the pipe to it, the replies from it, and the sound whose end it waits for.
pub struct Helper {
    child: Option<Child>,
    input: Option<Box<dyn Write + Send>>,
    replies: Receiver<Reply>,
    waiting: Waiting,
    format: Format,
    serial: u64,
    reader: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Helper {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Helper")
            .field("child", &self.child.as_ref().map(Child::id))
            .field("format", &self.format)
            .field("serial", &self.serial)
            .finish_non_exhaustive()
    }
}

impl Helper {
    /// Starts `program` and waits for it to open the device.
    ///
    /// # Errors
    ///
    /// [`HelperError::Missing`] where there is no such program, [`HelperError::Ended`] where it
    /// ended before it said anything, [`HelperError::NoDevice`] where it could not open the device,
    /// and [`HelperError::Silent`] where it did not answer in time.
    pub fn spawn(program: &Path) -> Result<Self, HelperError> {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // The helper's own diagnostics, and the audio library's, go where the window's go.
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::NotFound {
                    HelperError::Missing {
                        program: program.to_path_buf(),
                    }
                } else {
                    HelperError::Unstarted {
                        program: program.to_path_buf(),
                        source,
                    }
                }
            })?;
        let (Some(input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HelperError::Ended(None));
        };
        Self::started(Some(child), Box::new(input), output)
    }

    /// The same over pipes the caller made, with no process behind them: a helper's [`crate::serve`]
    /// run on a thread, in a test.
    ///
    /// # Errors
    ///
    /// As [`Self::spawn`], but never `Missing`.
    pub fn over(
        input: impl Write + Send + 'static,
        output: impl Read + Send + 'static,
    ) -> Result<Self, HelperError> {
        Self::started(None, Box::new(input), output)
    }

    fn started(
        child: Option<Child>,
        input: Box<dyn Write + Send>,
        output: impl Read + Send + 'static,
    ) -> Result<Self, HelperError> {
        let waiting: Waiting = Arc::new(Mutex::new(None));
        let (said, replies) = channel();
        let ends = Arc::clone(&waiting);
        let reader = std::thread::Builder::new()
            .name("sound-helper".to_owned())
            .spawn(move || read_replies(output, &said, &ends));
        let mut helper = Self {
            child,
            input: Some(input),
            replies,
            waiting,
            format: Format {
                rate: 0,
                channels: 0,
            },
            serial: 0,
            reader: None,
        };
        helper.reader = Some(reader.map_err(|source| HelperError::Unstarted {
            program: PathBuf::from(crate::HELPER_PROGRAM),
            source,
        })?);
        match helper.answer()? {
            Reply::Opened(format) => {
                helper.format = format;
                Ok(helper)
            }
            Reply::Failed(why) => Err(HelperError::NoDevice(why)),
            _ => Err(HelperError::OutOfTurn),
        }
    }

    /// The device's format, which each sound is converted to before [`Self::play`].
    #[must_use]
    pub fn format(&self) -> Format {
        self.format
    }

    /// Plays `samples` in place of whatever sounds; `ended` is called once, on the helper's reader
    /// thread, when the helper says the last has been written.
    ///
    /// # Errors
    ///
    /// [`HelperError::Refused`] with the device's reason, or the helper's having ended, gone
    /// silent, or broken its pipe.
    pub fn play(
        &mut self,
        samples: &[f32],
        ended: Box<dyn FnOnce() + Send>,
    ) -> Result<(), HelperError> {
        self.serial = self.serial.wrapping_add(1);
        let serial = self.serial;
        // Set before the order is written: a short sound's end may arrive before its start.
        *self.waiting.lock().unwrap_or_else(PoisonError::into_inner) = Some((serial, ended));
        let written = match self.input.as_mut() {
            Some(input) => wire::write_play(input, serial, samples).map_err(HelperError::from),
            None => Err(HelperError::Ended(None)),
        };
        let answered = written.and_then(|()| {
            loop {
                match self.answer()? {
                    Reply::Started(started) if started == serial => break Ok(()),
                    Reply::Refused {
                        serial: refused,
                        why,
                    } if refused == serial => {
                        break Err(HelperError::Refused(why));
                    }
                    // A reply to an earlier sound, superseded before it was read.
                    Reply::Started(_) | Reply::Refused { .. } => {}
                    Reply::Opened(_) | Reply::Failed(_) | Reply::Ended(_) => {
                        break Err(HelperError::OutOfTurn);
                    }
                }
            }
        });
        if answered.is_err() {
            self.forget();
        }
        answered.map_err(|error| self.with_status(error))
    }

    /// Stops whatever sounds; its `ended` is not called. Nothing where the helper has gone, since
    /// then nothing sounds.
    pub fn stop(&mut self) {
        self.forget();
        if let Some(input) = self.input.as_mut() {
            let _ = wire::write_stop(input);
        }
    }

    /// The next reply that is not a sound's end, within `ANSWER_WITHIN`, ten seconds.
    fn answer(&mut self) -> Result<Reply, HelperError> {
        match self.replies.recv_timeout(ANSWER_WITHIN) {
            Ok(reply) => Ok(reply),
            Err(RecvTimeoutError::Timeout) => Err(HelperError::Silent),
            Err(RecvTimeoutError::Disconnected) => Err(self.with_status(HelperError::Ended(None))),
        }
    }

    /// An `Ended` with the child's exit status, where the child has exited.
    fn with_status(&mut self, error: HelperError) -> HelperError {
        let HelperError::Ended(None) = error else {
            return error;
        };
        let status = self.child.as_mut().and_then(|child| {
            let deadline = Instant::now().checked_add(EXIT_WITHIN)?;
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => return Some(status),
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Ok(None) | Err(_) => return None,
                }
            }
        });
        HelperError::Ended(status)
    }

    fn forget(&self) {
        *self.waiting.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

impl Drop for Helper {
    /// Closes the pipe, which is the helper's order to stop its sound and exit; kills it where it
    /// has not within `EXIT_WITHIN`, two seconds, so that closing a window never waits on a stuck
    /// device.
    fn drop(&mut self) {
        self.forget();
        drop(self.input.take());
        if let Some(child) = self.child.as_mut() {
            let deadline = Instant::now().checked_add(EXIT_WITHIN);
            let exited = loop {
                match child.try_wait() {
                    Ok(Some(_)) => break true,
                    Ok(None) if deadline.is_some_and(|deadline| Instant::now() < deadline) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Ok(None) | Err(_) => break false,
                }
            };
            if !exited {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

/// The reader thread: each sound's end is called here, every other reply is passed on.
fn read_replies(mut output: impl Read, said: &std::sync::mpsc::Sender<Reply>, waiting: &Waiting) {
    // An end of the pipe or a malformed reply both end the helper as far as this side is concerned;
    // the sender's drop is what tells the window's side so.
    while let Ok(Some(reply)) = wire::read_reply(&mut output) {
        if let Reply::Ended(serial) = reply {
            let mut slot = waiting.lock().unwrap_or_else(PoisonError::into_inner);
            if slot.as_ref().is_some_and(|(waited, _)| *waited == serial)
                && let Some((_, ended)) = slot.take()
            {
                drop(slot);
                ended();
            }
        } else if said.send(reply).is_err() {
            return;
        }
    }
}
