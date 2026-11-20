//! What crosses the helper's two pipes: orders on its standard input, replies on its standard output.
//!
//! Each message is a tag byte and its fields, little-endian, with no frame around it: both ends are
//! this crate, built from one tree and installed together, so there is no version to negotiate.
//!
//! | from the window | fields |
//! |---|---|
//! | [`PLAY`] | the sound's serial (`u64`), its sample count (`u32`), the samples (`f32` each) |
//! | [`STOP`] | none |
//!
//! The end of the window's pipe is the order to end: the helper stops what sounds and exits.
//!
//! | from the helper | fields |
//! |---|---|
//! | [`OPENED`] | the device's rate (`u32`) and channel count (`u16`), once, first |
//! | [`FAILED`] | why the device was not opened (text), once, in place of [`OPENED`]; the helper then exits |
//! | [`STARTED`] | the serial of the sound now sounding |
//! | [`REFUSED`] | the serial of a sound the device would not start, and why (text) |
//! | [`ENDED`] | the serial of a sound whose last sample has been written |
//!
//! Text is a `u32` byte count and that many bytes of UTF-8, at most [`MAX_TEXT`]. Every count is
//! checked against its bound before anything is allocated.

use std::io::{Read, Write};

/// An order to play a sound in place of whatever sounds.
pub const PLAY: u8 = 1;
/// An order to stop whatever sounds.
pub const STOP: u8 = 2;

/// The device is open, and this is its format.
pub const OPENED: u8 = 1;
/// The device was not opened, and this is why.
pub const FAILED: u8 = 2;
/// The sound with this serial sounds.
pub const STARTED: u8 = 3;
/// The sound with this serial was not started, and this is why.
pub const REFUSED: u8 = 4;
/// The sound with this serial came to its end.
pub const ENDED: u8 = 5;

/// The most samples one sound may carry: `viewer_host::sound::MAX_PLAYED_SAMPLES`, which bounds
/// what the window converts, held by the helper as its own bound on what it reads (64 MiB of
/// `f32`).
pub const MAX_SAMPLES: usize = 1 << 24;

/// The longest text either side reads, in bytes: a sentence from the device, never a document's.
pub const MAX_TEXT: usize = 4096;

/// How many samples cross per write and per read, so that a sound is never held twice in full as
/// bytes beside its samples.
const CHUNK: usize = 16 * 1024;

/// The device's format, which the window converts each sound to before it is sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Format {
    /// Frames per second.
    pub rate: u32,
    /// Samples per frame.
    pub channels: u16,
}

/// An order, as the helper reads it.
#[derive(Debug, PartialEq)]
pub enum Order {
    /// Play `samples`, interleaved at the device's format, as the sound numbered `serial`.
    Play {
        /// The window's number for this sound, which its replies carry back.
        serial: u64,
        /// The samples, as fractions of full scale.
        samples: Vec<f32>,
    },
    /// Stop whatever sounds.
    Stop,
}

/// A reply, as the window reads it.
#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    /// The device is open, in this format.
    Opened(Format),
    /// The device was not opened, for this reason.
    Failed(String),
    /// The sound with this serial sounds.
    Started(u64),
    /// The sound with this serial was not started.
    Refused {
        /// Its serial.
        serial: u64,
        /// Why.
        why: String,
    },
    /// The sound with this serial came to its end.
    Ended(u64),
}

/// Why a message could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum WireError {
    /// The pipe failed, or ended inside a message.
    #[error("the pipe: {0}")]
    Pipe(#[from] std::io::Error),
    /// A tag neither side writes.
    #[error("a message tagged {0}, which is not one of this wire's")]
    UnknownTag(u8),
    /// A sound longer than [`MAX_SAMPLES`].
    #[error("a sound of {0} samples, past the {MAX_SAMPLES} one sound may carry")]
    TooManySamples(u64),
    /// A text longer than [`MAX_TEXT`].
    #[error("a text of {0} bytes, past the {MAX_TEXT} a reply may carry")]
    TooLong(u64),
    /// A text that is not UTF-8.
    #[error("a text that is not UTF-8")]
    NotText,
}

/// Writes the order to play `samples` as the sound numbered `serial`.
///
/// # Errors
///
/// [`WireError::TooManySamples`] past [`MAX_SAMPLES`], before anything is written; otherwise the
/// pipe's failure.
pub fn write_play(to: &mut impl Write, serial: u64, samples: &[f32]) -> Result<(), WireError> {
    let count = u32::try_from(samples.len())
        .ok()
        .filter(|count| usize::try_from(*count).is_ok_and(|count| count <= MAX_SAMPLES))
        .ok_or_else(|| {
            WireError::TooManySamples(u64::try_from(samples.len()).unwrap_or(u64::MAX))
        })?;
    to.write_all(&[PLAY])?;
    to.write_all(&serial.to_le_bytes())?;
    to.write_all(&count.to_le_bytes())?;
    let mut bytes = Vec::with_capacity(CHUNK.saturating_mul(4));
    for chunk in samples.chunks(CHUNK) {
        bytes.clear();
        for sample in chunk {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        to.write_all(&bytes)?;
    }
    to.flush()?;
    Ok(())
}

/// Writes the order to stop whatever sounds.
///
/// # Errors
///
/// The pipe's failure.
pub fn write_stop(to: &mut impl Write) -> Result<(), WireError> {
    to.write_all(&[STOP])?;
    to.flush()?;
    Ok(())
}

/// Reads the next order; `None` where the window's pipe ended between orders.
///
/// # Errors
///
/// The pipe's failure, an end inside an order, an unknown tag, or a sound past [`MAX_SAMPLES`].
pub fn read_order(from: &mut impl Read) -> Result<Option<Order>, WireError> {
    let Some(tag) = read_tag(from)? else {
        return Ok(None);
    };
    match tag {
        PLAY => {
            let serial = u64::from_le_bytes(read_array(from)?);
            let count = u32::from_le_bytes(read_array(from)?);
            let count = usize::try_from(count)
                .ok()
                .filter(|count| *count <= MAX_SAMPLES)
                .ok_or(WireError::TooManySamples(u64::from(count)))?;
            let mut samples = Vec::with_capacity(count);
            let mut bytes = vec![0_u8; CHUNK.saturating_mul(4)];
            while samples.len() < count {
                let wanted = count.saturating_sub(samples.len()).min(CHUNK);
                let Some(part) = bytes.get_mut(..wanted.saturating_mul(4)) else {
                    break;
                };
                from.read_exact(part)?;
                samples.extend(
                    part.as_chunks::<4>()
                        .0
                        .iter()
                        .map(|word| f32::from_le_bytes(*word)),
                );
            }
            Ok(Some(Order::Play { serial, samples }))
        }
        STOP => Ok(Some(Order::Stop)),
        other => Err(WireError::UnknownTag(other)),
    }
}

/// Writes one reply and flushes it.
///
/// # Errors
///
/// The pipe's failure.
pub fn write_reply(to: &mut impl Write, reply: &Reply) -> Result<(), WireError> {
    match reply {
        Reply::Opened(format) => {
            to.write_all(&[OPENED])?;
            to.write_all(&format.rate.to_le_bytes())?;
            to.write_all(&format.channels.to_le_bytes())?;
        }
        Reply::Failed(why) => {
            to.write_all(&[FAILED])?;
            write_text(to, why)?;
        }
        Reply::Started(serial) => {
            to.write_all(&[STARTED])?;
            to.write_all(&serial.to_le_bytes())?;
        }
        Reply::Refused { serial, why } => {
            to.write_all(&[REFUSED])?;
            to.write_all(&serial.to_le_bytes())?;
            write_text(to, why)?;
        }
        Reply::Ended(serial) => {
            to.write_all(&[ENDED])?;
            to.write_all(&serial.to_le_bytes())?;
        }
    }
    to.flush()?;
    Ok(())
}

/// Reads the next reply; `None` where the helper's pipe ended between replies.
///
/// # Errors
///
/// The pipe's failure, an end inside a reply, an unknown tag, or a text past [`MAX_TEXT`] or not
/// UTF-8.
pub fn read_reply(from: &mut impl Read) -> Result<Option<Reply>, WireError> {
    let Some(tag) = read_tag(from)? else {
        return Ok(None);
    };
    let reply = match tag {
        OPENED => Reply::Opened(Format {
            rate: u32::from_le_bytes(read_array(from)?),
            channels: u16::from_le_bytes(read_array(from)?),
        }),
        FAILED => Reply::Failed(read_text(from)?),
        STARTED => Reply::Started(u64::from_le_bytes(read_array(from)?)),
        REFUSED => Reply::Refused {
            serial: u64::from_le_bytes(read_array(from)?),
            why: read_text(from)?,
        },
        ENDED => Reply::Ended(u64::from_le_bytes(read_array(from)?)),
        other => return Err(WireError::UnknownTag(other)),
    };
    Ok(Some(reply))
}

/// The tag byte, or `None` at an end between messages.
fn read_tag(from: &mut impl Read) -> Result<Option<u8>, WireError> {
    let mut tag = [0_u8; 1];
    loop {
        match from.read(&mut tag) {
            Ok(0) => return Ok(None),
            Ok(_) => return Ok(Some(tag[0])),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
}

fn read_array<const N: usize>(from: &mut impl Read) -> Result<[u8; N], WireError> {
    let mut bytes = [0_u8; N];
    from.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// A text, cut at [`MAX_TEXT`] bytes on a character boundary so that a long sentence is still said.
fn write_text(to: &mut impl Write, text: &str) -> Result<(), WireError> {
    let mut end = text.len().min(MAX_TEXT);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let bytes = text.as_bytes().get(..end).unwrap_or_default();
    // At most `MAX_TEXT`, which a `u32` holds.
    let length = u32::try_from(bytes.len()).unwrap_or(0);
    to.write_all(&length.to_le_bytes())?;
    to.write_all(bytes)?;
    Ok(())
}

fn read_text(from: &mut impl Read) -> Result<String, WireError> {
    let length = u32::from_le_bytes(read_array(from)?);
    let length = usize::try_from(length)
        .ok()
        .filter(|length| *length <= MAX_TEXT)
        .ok_or(WireError::TooLong(u64::from(length)))?;
    let mut bytes = vec![0_u8; length];
    from.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| WireError::NotText)
}

#[cfg(test)]
mod tests {
    use super::{
        Format, MAX_SAMPLES, MAX_TEXT, Order, Reply, WireError, read_order, read_reply, write_play,
        write_reply, write_stop,
    };

    #[test]
    fn every_order_reads_back_as_written() {
        let samples: Vec<f32> = (0..40_000_u16).map(|n| f32::from(n) / 40_000.0).collect();
        let mut pipe = Vec::new();
        write_play(&mut pipe, 7, &samples).expect("a vector takes it");
        write_stop(&mut pipe).expect("a vector takes it");
        let mut from = pipe.as_slice();
        assert_eq!(
            read_order(&mut from).expect("well formed"),
            Some(Order::Play { serial: 7, samples })
        );
        assert_eq!(
            read_order(&mut from).expect("well formed"),
            Some(Order::Stop)
        );
        assert_eq!(read_order(&mut from).expect("an end between orders"), None);
    }

    #[test]
    fn every_reply_reads_back_as_written() {
        let replies = [
            Reply::Opened(Format {
                rate: 48_000,
                channels: 2,
            }),
            Reply::Failed("no device".to_owned()),
            Reply::Started(3),
            Reply::Refused {
                serial: 4,
                why: "busy".to_owned(),
            },
            Reply::Ended(3),
        ];
        let mut pipe = Vec::new();
        for reply in &replies {
            write_reply(&mut pipe, reply).expect("a vector takes it");
        }
        let mut from = pipe.as_slice();
        for reply in replies {
            assert_eq!(read_reply(&mut from).expect("well formed"), Some(reply));
        }
        assert_eq!(read_reply(&mut from).expect("an end"), None);
    }

    /// A count past the bound is refused before its samples are allocated.
    #[test]
    fn a_sound_past_the_bound_is_refused_unread() {
        let mut pipe = vec![super::PLAY];
        pipe.extend_from_slice(&1_u64.to_le_bytes());
        pipe.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            read_order(&mut pipe.as_slice()),
            Err(WireError::TooManySamples(count)) if count == u64::from(u32::MAX)
        ));
        let too_many = vec![0.0_f32; MAX_SAMPLES + 1];
        assert!(matches!(
            write_play(&mut Vec::new(), 1, &too_many),
            Err(WireError::TooManySamples(_))
        ));
    }

    /// A long sentence is cut on a character boundary, and a short read inside a message is the
    /// pipe's error rather than an end.
    #[test]
    fn a_long_text_is_cut_and_a_cut_message_is_an_error() {
        let long = "é".repeat(MAX_TEXT);
        let mut pipe = Vec::new();
        write_reply(&mut pipe, &Reply::Failed(long)).expect("a vector takes it");
        let Some(Reply::Failed(said)) = read_reply(&mut pipe.as_slice()).expect("well formed")
        else {
            panic!("not the reply written");
        };
        assert!(said.len() <= MAX_TEXT && said.chars().all(|c| c == 'é'));
        let cut = pipe.get(..3).expect("longer than three bytes");
        assert!(matches!(read_reply(&mut &cut[..]), Err(WireError::Pipe(_))));
        assert!(matches!(
            read_reply(&mut &[9_u8][..]),
            Err(WireError::UnknownTag(9))
        ));
    }
}
