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

/// A confined worker's side of the resource port: how it asks its host for a file it cannot open.
///
/// One request out, one answer back, on the socket and the pipe the worker already speaks — see
/// [`crate::frame::RESOURCE_REQUEST`] for why that pair is the transport's own rather than either
/// protocol's, and [`crate::frame::RESOURCE_ANSWER`] for why the resource crosses as bytes.
///
/// # Why it does not use the worker's own reader or `std::io::stdout`
///
/// Both workers in this tree hold a [`std::io::StdoutLock`] for the whole of their serve loop and
/// are *inside* an answer when a font is wanted, so a second `stdout().lock()` from the rasterising
/// thread would wait for a lock the answering thread does not release until the answer is written.
/// The request is therefore written with `write` on the descriptor itself, which takes no lock of
/// the standard library's, and serialised by [`ASKING`] instead — a request is a whole exchange
/// and two of them may not interleave on one socket.
#[cfg(unix)]
mod resources {
    use std::sync::Mutex;

    use super::{Link, Source as _};
    use crate::frame;

    /// The link a resource request is asked over, once a worker has armed one.
    ///
    /// `None` until [`requests_go_to_the_host`] is called, which is what makes this port
    /// something a worker opts into rather than something the transport does behind it.
    static ASKING: Mutex<Option<Link>> = Mutex::new(None);

    /// Arms this process to ask its host for resources it cannot open itself.
    ///
    /// Called before the confinement, in the paragraph that already states what the process
    /// cannot reach. Arming twice keeps the first arming.
    pub fn requests_go_to_the_host(program: &'static str) {
        if let Ok(mut held) = ASKING.lock()
            && held.is_none()
        {
            *held = Some(Link::stdin(program));
        }
    }

    /// Asks the host for a resource, and answers with what identifies it and its bytes.
    ///
    /// `None` where nothing is armed, where the host offered nothing, or where the answer was not
    /// one — each of which a caller reads the same way: this resource is not available, carry on
    /// without it. **Nothing here can fail in a way that stops the work**, which is the floor
    /// `doc/todo/59` states: a worker whose host provides nothing still renders.
    ///
    /// The signature is bytes in, bytes out, because this crate has no opinion about what a
    /// resource is. `pdf_font::provider` writes the description and reads the identity.
    #[must_use]
    pub fn ask_the_host(request: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        use std::os::fd::AsFd as _;

        let mut held = ASKING.lock().ok()?;
        let link = held.as_mut()?;
        if request.len() > frame::MAX_RESOURCE_REQUEST {
            return None;
        }

        // `write` on the descriptor rather than through `std::io::Stdout`: see the module's own
        // comment. `write` is on the confinement's allow-list and so is the `recvmsg` the answer
        // arrives by — **the port adds no system call at all**, which is what makes it a port and
        // not a permission.
        let stdout = std::io::stdout();
        write_all(
            stdout.as_fd(),
            &frame::header(frame::RESOURCE_REQUEST, request.len()),
        )
        .ok()?;
        write_all(stdout.as_fd(), request).ok()?;

        let mut header = [0u8; frame::HEADER_LEN];
        let mut descriptors = Vec::new();
        link.fill(&mut header, &mut descriptors).ok()?;
        let (kind, length) = frame::parse_header(header)?;
        // The host is the *privileged* side here, so its length is not a claim in the sense the
        // worker's is — but it is still a number this process would allocate from, and a bound
        // that exists at both ends is the one that survives a host with a defect.
        if kind != frame::RESOURCE_ANSWER || length > frame::MAX_RESOURCE {
            return None;
        }
        // Nothing offered: an empty answer, which is what a host with no broker gives and what
        // this whole layer defaults to.
        if length == 0 {
            return None;
        }
        let mut payload = Vec::new();
        payload.try_reserve_exact(length).ok()?;
        payload.resize(length, 0);
        link.fill(&mut payload, &mut descriptors).ok()?;

        let named = usize::try_from(u32::from_be_bytes(payload.get(..4)?.try_into().ok()?)).ok()?;
        let identity = payload.get(4..4usize.checked_add(named)?)?.to_vec();
        let content = payload.get(4usize.checked_add(named)?..)?.to_vec();
        if content.is_empty() {
            return None;
        }
        Some((identity, content))
    }

    /// Writes a whole buffer to a descriptor, retrying a short write.
    fn write_all(
        descriptor: std::os::fd::BorrowedFd<'_>,
        mut bytes: &[u8],
    ) -> Result<(), rustix::io::Errno> {
        while !bytes.is_empty() {
            match rustix::io::write(descriptor, bytes) {
                Ok(0) => return Err(rustix::io::Errno::IO),
                Ok(written) => bytes = bytes.get(written..).unwrap_or_default(),
                Err(rustix::io::Errno::INTR) => {}
                Err(errno) => return Err(errno),
            }
        }
        Ok(())
    }
}

/// The port where there is no socket to ask over: armed by nobody, offering nothing.
///
/// The two functions exist so that a worker's confinement paragraph reads the same everywhere and
/// compiles everywhere; what they answer is the default this whole layer has — nothing.
#[cfg(not(unix))]
mod resources {
    /// Arms nothing, because there is nothing to arm.
    pub fn requests_go_to_the_host(_program: &'static str) {}

    /// Offers nothing, always.
    #[must_use]
    pub fn ask_the_host(_request: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        None
    }
}

pub use resources::{ask_the_host, requests_go_to_the_host};
