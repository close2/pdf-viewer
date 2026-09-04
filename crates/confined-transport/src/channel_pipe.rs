//! The host's end of the worker's standard input, where the platform passes no descriptor.
//!
//! A pipe, and nothing crosses it but bytes. The encoders above this crate say so by name rather
//! than by silence: a document that would have crossed as a descriptor crosses as its bytes, or
//! is refused with a sentence.

use std::io::Write;
use std::process::{Child, ChildStdin, Stdio};

/// A descriptor as the sender holds it — on a platform that passes none, a type nothing can
/// construct, so that the sending code reads the same on both and compiles on both.
#[derive(Debug, Clone, Copy)]
pub struct SentDescriptor<'a>(std::convert::Infallible, std::marker::PhantomData<&'a ()>);

/// The host's end of the worker's standard input.
#[derive(Debug)]
pub struct Channel {
    /// The pipe, which the child holds until it has been spawned.
    pipe: Option<ChildStdin>,
}

impl Channel {
    /// The worker's standard input as a pipe, to be taken from the child once it is spawned.
    ///
    /// # Errors
    ///
    /// Never on this platform; the signature matches the socket's.
    pub fn pair() -> std::io::Result<(Self, Stdio)> {
        Ok((Self { pipe: None }, Stdio::piped()))
    }

    /// This side's end of the pipe, which the child holds until it has been spawned.
    #[must_use]
    pub fn attach(self, child: &mut Child) -> Option<Self> {
        child.stdin.take().map(|pipe| Self { pipe: Some(pipe) })
    }

    /// Writes a frame's header. No descriptor crosses here, and no encoder ever offers one.
    ///
    /// # Errors
    ///
    /// The pipe's own.
    pub fn send_header(
        &mut self,
        header: &[u8],
        _descriptor: Option<SentDescriptor<'_>>,
    ) -> std::io::Result<()> {
        self.write_all(header)
    }
}

impl Write for Channel {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self.pipe.as_mut() {
            Some(pipe) => pipe.write(bytes),
            None => Err(std::io::Error::other(
                "the worker's input was never attached",
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self.pipe.as_mut() {
            Some(pipe) => pipe.flush(),
            None => Ok(()),
        }
    }
}
