//! The confined side's input: where frames come from, and what can arrive beside them.
//!
//! A worker's standard input is a socket when a [`crate::Host`] started it — one end of a pair the
//! host made — and a document open on disk arrives as its descriptor beside the frame's header,
//! sent with `SCM_RIGHTS` (ADR 0812). `read` cannot see ancillary data, so a frame is read with
//! `recvmsg`, and every descriptor that arrives is collected beside the bytes it came with. A test
//! hands over a slice, which carries none.

use std::io::Read as _;

/// A descriptor as the receiver holds it: the kernel's duplicate, owned.
#[cfg(unix)]
pub type ReceivedDescriptor = std::os::fd::OwnedFd;

/// A descriptor as the receiver holds it, on a platform that passes none: a type nothing can
/// construct, so that the receiving code reads the same on both and compiles on both.
#[cfg(not(unix))]
#[derive(Debug)]
pub struct ReceivedDescriptor(std::convert::Infallible);

/// Where a confined worker reads its frames.
pub trait Source {
    /// Fills `buffer` entirely, adding whatever descriptors arrived beside its bytes.
    ///
    /// # Errors
    ///
    /// [`std::io::ErrorKind::UnexpectedEof`] where the input ended first, as `read_exact` reports
    /// it; and the platform's own errors.
    fn fill(
        &mut self,
        buffer: &mut [u8],
        descriptors: &mut Vec<ReceivedDescriptor>,
    ) -> Result<(), std::io::Error>;

    /// Reads and throws away `bytes` bytes, closing any descriptor that arrives with them.
    ///
    /// The refusal path: a frame the budget will not admit has been written already, and a reader
    /// that left it in the socket would read the next frame out of the middle of this one. A
    /// descriptor beside a refused frame is nobody's and is closed here.
    ///
    /// # Errors
    ///
    /// As [`Source::fill`].
    fn skip(&mut self, mut bytes: usize) -> Result<(), std::io::Error> {
        /// Enough to empty a socket quickly and small enough to be nobody's memory problem.
        const SCRATCH: usize = 64 * 1024;

        let mut scratch = vec![0u8; SCRATCH.min(bytes.max(1))];
        let mut unclaimed = Vec::new();
        while bytes > 0 {
            let take = bytes.min(scratch.len());
            let Some(slice) = scratch.get_mut(..take) else {
                break;
            };
            self.fill(slice, &mut unclaimed)?;
            unclaimed.clear();
            bytes = bytes.saturating_sub(take);
        }
        Ok(())
    }
}

/// A slice as a source: what a test hands over. Nothing arrives beside it.
impl Source for &[u8] {
    fn fill(
        &mut self,
        buffer: &mut [u8],
        _descriptors: &mut Vec<ReceivedDescriptor>,
    ) -> Result<(), std::io::Error> {
        self.read_exact(buffer)
    }
}

/// The worker's standard input.
///
/// Read with `recvmsg` while it is a socket, so that a descriptor sent beside a frame's header
/// comes through with the header; and with `read` once it has turned out not to be one —
/// `recvmsg` on a pipe is `ENOTSOCK`, which is what a worker run by hand gets, and it is an answer
/// rather than a kill because `recvmsg` is on the allow-list.
#[derive(Debug)]
pub struct Link {
    /// The process's own standard input.
    stdin: std::io::Stdin,
    /// What the worker calls itself, for the one line it may have to print.
    program: &'static str,
    /// Whether `recvmsg` has been refused on this input, after which it is a plain reader.
    #[cfg(unix)]
    not_a_socket: bool,
}

impl Link {
    /// This process's standard input, under the name the program answers to.
    #[must_use]
    pub fn stdin(program: &'static str) -> Self {
        Self {
            stdin: std::io::stdin(),
            program,
            #[cfg(unix)]
            not_a_socket: false,
        }
    }
}

#[cfg(unix)]
impl Source for Link {
    fn fill(
        &mut self,
        buffer: &mut [u8],
        descriptors: &mut Vec<ReceivedDescriptor>,
    ) -> Result<(), std::io::Error> {
        use std::os::fd::AsFd as _;

        use rustix::net::{RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags};

        if self.not_a_socket {
            return self.stdin.lock().read_exact(buffer);
        }
        // Room for one descriptor per call, which is what a frame carries: a host sends the
        // document's descriptor beside the header and nothing beside the payload.
        let mut space = [std::mem::MaybeUninit::<u8>::uninit(); rustix::cmsg_space!(ScmRights(1))];
        let mut filled = 0usize;
        while filled < buffer.len() {
            let Some(rest) = buffer.get_mut(filled..) else {
                break;
            };
            let mut ancillary = RecvAncillaryBuffer::new(&mut space);
            let received = match rustix::net::recvmsg(
                self.stdin.as_fd(),
                &mut [std::io::IoSliceMut::new(rest)],
                &mut ancillary,
                RecvFlags::empty(),
            ) {
                Ok(received) => received,
                Err(rustix::io::Errno::NOTSOCK) => {
                    self.not_a_socket = true;
                    return self.stdin.lock().read_exact(rest);
                }
                Err(rustix::io::Errno::INTR) => continue,
                Err(errno) => return Err(errno.into()),
            };
            for message in ancillary.drain() {
                if let RecvAncillaryMessage::ScmRights(arrived) = message {
                    descriptors.extend(arrived);
                }
            }
            // `MSG_CTRUNC` is the kernel saying it had a descriptor for this process and no slot
            // to put it in — `RLIMIT_NOFILE` is eight here — so it closed it. Nothing is read as
            // though it had arrived; the frame goes on to be decoded without it, and the decoder
            // refuses a document that needed one, by name.
            if received.flags.contains(ReturnFlags::CTRUNC) {
                eprintln!(
                    "{}: a descriptor sent with a frame was dropped by the kernel because this \
                     process has no free descriptor slot",
                    self.program
                );
            }
            if received.bytes == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
            }
            filled = filled.saturating_add(received.bytes);
        }
        Ok(())
    }
}

#[cfg(not(unix))]
impl Source for Link {
    fn fill(
        &mut self,
        buffer: &mut [u8],
        _descriptors: &mut Vec<ReceivedDescriptor>,
    ) -> Result<(), std::io::Error> {
        self.stdin.lock().read_exact(buffer)
    }
}
