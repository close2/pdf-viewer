//! The helper's half: open the device, say its format, and play what the window sends.

use std::io::{Read, Write};
use std::sync::mpsc::{Sender, channel};

use crate::wire::{self, Format, Order, Reply, WireError};

/// The audio device the helper plays on: `cpal`'s default output in the program, a recording double
/// in a test.
pub trait Device {
    /// The device's rate and channel count, which the window converts each sound to.
    fn format(&self) -> Format;

    /// Starts `samples`, interleaved at [`Self::format`], in place of whatever sounds; `ended` is
    /// called once, from whichever thread the device runs on, when the last has been written.
    ///
    /// # Errors
    ///
    /// The sentence to say where the device would not start.
    fn start(&mut self, samples: Vec<f32>, ended: Box<dyn FnOnce() + Send>) -> Result<(), String>;

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
/// An order that could not be read. Its sound, if one sounds, is stopped first. A device that would
/// not open is not an error here: the window has been told, and the helper's work is over.
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
    // The writer ends when the last sender has gone: this one and every sound's `ended`, which the
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
    while let Some(order) = wire::read_order(input)? {
        match order {
            Order::Stop => device.stop(),
            Order::Play { serial, samples } => {
                device.stop();
                let ended = replies.clone();
                let reply = match device.start(
                    samples,
                    Box::new(move || {
                        let _ = ended.send(Reply::Ended(serial));
                    }),
                ) {
                    Ok(()) => Reply::Started(serial),
                    Err(why) => Reply::Refused { serial, why },
                };
                let _ = replies.send(reply);
            }
        }
    }
    Ok(())
}
