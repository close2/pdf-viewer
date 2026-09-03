//! The host's end of the worker's standard input, where the platform can pass a descriptor.
//!
//! A socket rather than a pipe, and the difference is the one thing a pipe cannot do: carry a
//! descriptor. A document open on disk crosses as its open file, sent with `SCM_RIGHTS` beside a
//! frame's header (ADR 0812), and a socket is the only transport the kernel passes one over. The
//! worker reads its frames from it with `recvmsg`; what it writes back still comes up a pipe,
//! because nothing crosses that way but bytes.

use std::io::Write;
use std::process::{Child, Stdio};

/// A descriptor as the sender holds it while the frame it rides beside is written.
pub type SentDescriptor<'a> = std::os::fd::BorrowedFd<'a>;

/// The host's end of the worker's standard input.
#[derive(Debug)]
pub struct Channel {
    /// The host's half of the pair; the worker holds the other as its standard input.
    socket: std::os::unix::net::UnixStream,
}

impl Channel {
    /// Both ends: this side's, and the one to give the worker as its standard input.
    ///
    /// # Errors
    ///
    /// The platform's, where a socket pair cannot be made.
    pub fn pair() -> std::io::Result<(Self, Stdio)> {
        let (host_end, worker_end) = std::os::unix::net::UnixStream::pair()?;
        Ok((
            Self { socket: host_end },
            Stdio::from(std::os::fd::OwnedFd::from(worker_end)),
        ))
    }

    /// This side's end, once the worker is running.
    ///
    /// The socket was made before the spawn, so there is nothing to take from the child; the
    /// signature matches the pipe's so that [`crate::Host`] reads the same on both.
    #[must_use]
    pub fn attach(self, _child: &mut Child) -> Option<Self> {
        Some(self)
    }

    /// Writes a frame's header, with the descriptor beside it where there is one.
    ///
    /// A short send is completed with plain writes: the descriptor is attached to the first byte
    /// the kernel accepts, so the rest of the header needs no ancillary data.
    ///
    /// # Errors
    ///
    /// The socket's own, and one of this crate's where a buffer sized for exactly one descriptor
    /// declines to take one — which cannot happen and is reported rather than asserted.
    pub fn send_header(
        &mut self,
        header: &[u8],
        descriptor: Option<SentDescriptor<'_>>,
    ) -> std::io::Result<()> {
        use rustix::net::{SendAncillaryBuffer, SendAncillaryMessage, SendFlags};

        let Some(descriptor) = descriptor else {
            return self.socket.write_all(header);
        };
        let rights = [descriptor];
        let mut space = [std::mem::MaybeUninit::<u8>::uninit(); rustix::cmsg_space!(ScmRights(1))];
        let mut ancillary = SendAncillaryBuffer::new(&mut space);
        if !ancillary.push(SendAncillaryMessage::ScmRights(&rights)) {
            return Err(std::io::Error::other(
                "the ancillary buffer for one descriptor would not take one",
            ));
        }
        let sent = rustix::net::sendmsg(
            &self.socket,
            &[std::io::IoSlice::new(header)],
            &mut ancillary,
            SendFlags::empty(),
        )?;
        self.socket
            .write_all(header.get(sent..).unwrap_or_default())
    }
}

impl Write for Channel {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.socket.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }
}
