//! The helper's half: open the device, say its format, and play what the window sends.

use std::io::{Read, Write};
use std::sync::mpsc::{Sender, channel};

use crate::Progress;
use crate::held::Held;
use crate::wire::{self, Format, MAX_SAMPLES, Order, Reply, WireError};

/// One pass of the sound held: where in it the pass starts and how far it may go.
#[derive(Clone, Debug)]
pub struct Pass {
    /// The sound, which may still be arriving.
    pub held: Held,
    /// The frame the pass starts at.
    pub from: u64,
    /// The most frames it plays; it ends sooner where the sound does.
    pub frames: u64,
}

/// The audio device the helper plays on: `cpal`'s default output in the program, a recording double
/// in a test.
pub trait Device {
    /// The device's rate and channel count, which the window converts each sound to.
    fn format(&self) -> Format;

    /// Starts `pass`, interleaved at [`Self::format`], in place of whatever sounds; `report` is
    /// told, from whichever thread the device runs on, how many of its frames have been written as
    /// they are — often enough to time against, every twentieth of a second at most — and once that
    /// the last has been (ADR 3342). Where the device reaches samples that have not arrived yet it
    /// writes silence and counts nothing until they do (ADR 3457).
    ///
    /// # Errors
    ///
    /// The sentence to say where the device would not start.
    fn start(&mut self, pass: Pass, report: crate::Report) -> Result<(), String>;

    /// Stops the pass that sounds and answers how many of its frames the device had reached — the
    /// frames written less those the device said it still held unplayed — or `None` where nothing
    /// sounded (ADR 3458).
    fn pause(&mut self) -> Option<u64>;

    /// Stops what sounds; nothing where nothing does.
    fn stop(&mut self);
}

/// Opens the device with `open`, says its format on `output` or why it was not opened, then plays
/// each sound `input` orders until `input` ends.
///
/// The replies are written by a thread of their own, so a sound's end — reported from the device's
/// thread, which must not wait on a pipe — is a send on a channel and never a write.
///
/// # Errors
///
/// An order that could not be read, or a sound past [`MAX_SAMPLES`]. Its sound, if one sounds, is
/// stopped first. A device that would not open is not an error here: the window has been told, and
/// the helper's work is over.
pub fn serve<D: Device>(
    mut input: impl Read,
    output: impl Write + Send + 'static,
    open: impl FnOnce() -> Result<D, String>,
) -> Result<(), WireError> {
    let (replies, outbox) = channel::<Reply>();
    let writer = std::thread::Builder::new()
        .name("replies".to_owned())
        .spawn(move || {
            let mut output = output;
            for reply in outbox {
                // A window that has gone reads nothing more; its pipe's end ends the input too.
                if wire::write_reply(&mut output, &reply).is_err() {
                    return;
                }
            }
        })?;
    let read = match open() {
        Ok(mut device) => {
            let _ = replies.send(Reply::Opened(device.format()));
            let read = played(&mut input, &mut device, &replies);
            device.stop();
            read
        }
        Err(why) => {
            let _ = replies.send(Reply::Failed(why));
            Ok(())
        }
    };
    drop(replies);
    // The writer ends when the last sender has gone: this one and every sound's `report`, which the
    // device dropped with its stream above.
    let _ = writer.join();
    read
}

/// The orders, until the window's pipe ends.
fn played(
    input: &mut impl Read,
    device: &mut impl Device,
    replies: &Sender<Reply>,
) -> Result<(), WireError> {
    // The one copy of the sound, which every pass reads (ADR 3457).
    let mut held = Held::new();
    // The serial of the pass now sounding, which a pause's reply carries.
    let mut sounding = None;
    while let Some(order) = wire::read_order(input)? {
        match order {
            Order::Stop => {
                device.stop();
                sounding = None;
            }
            Order::Sound => {
                device.stop();
                sounding = None;
                held = Held::new();
            }
            Order::Samples(samples) => {
                let total = held.len().saturating_add(samples.len());
                if total > MAX_SAMPLES {
                    return Err(WireError::TooManySamples(
                        u64::try_from(total).unwrap_or(u64::MAX),
                    ));
                }
                held.append(samples);
            }
            Order::Complete => held.complete(),
            // Answered whatever sounds, so that the window, which waits for the answer, is never
            // left waiting: serial 0, which no pass is numbered, where nothing did.
            Order::Pause => {
                let reached = device.pause();
                let _ = replies.send(Reply::Paused {
                    serial: sounding.take().unwrap_or(0),
                    frames: reached.unwrap_or(0),
                });
            }
            Order::Play {
                serial,
                from,
                frames,
            } => {
                device.stop();
                let progress = replies.clone();
                let pass = Pass {
                    held: held.clone(),
                    from,
                    frames,
                };
                let reply = match device.start(
                    pass,
                    Box::new(move |said| {
                        let _ = progress.send(match said {
                            Progress::Played(frames) => Reply::Played { serial, frames },
                            Progress::Heard(frames) => Reply::Heard { serial, frames },
                            Progress::Ended => Reply::Ended(serial),
                        });
                    }),
                ) {
                    Ok(()) => {
                        sounding = Some(serial);
                        Reply::Started(serial)
                    }
                    Err(why) => {
                        sounding = None;
                        Reply::Refused { serial, why }
                    }
                };
                let _ = replies.send(reply);
            }
        }
    }
    Ok(())
}
