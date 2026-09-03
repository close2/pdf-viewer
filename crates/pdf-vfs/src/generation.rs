//! The generation key, and the backing file it is asked of.
//!
//! RFC 0003 section 5.4 states the rule this module exists for: "[t]he backing file is the single
//! source of truth, and it can change three ways: our own committed write, another program's
//! incremental update, or a full rewrite", so "[e]very operation validates the key before
//! answering; a changed key rebuilds the virtual tree". That is a **correctness** requirement
//! rather than a nicety — a generation served after the document changed is a wrong answer, and
//! it is wrong silently, which is the worst shape a wrong answer takes.
//!
//! # The three components, and why the third one is there
//!
//! Modification time and size are the cheap two and they are not enough on their own: a file
//! system's timestamp granularity is coarser than a program's edit, and an incremental update
//! that replaces one object with another of the same length changes neither number. The third is
//! ISO 32000-2 §7.5.5's own offset:
//!
//! > The two preceding lines shall contain, one per line and in order, the keyword startxref and
//! > the byte offset in the decoded stream from the beginning of the PDF file to the beginning of
//! > the xref keyword in the last cross-reference section.
//!
//! §7.5.6 makes every update append "a cross-reference section" and a trailer of its own, so the
//! offset the last `startxref` names is different after every update this program or any other
//! makes. Reading it is a **bounded scan of the file's tail for a keyword** and not a parse: the
//! clause puts the line second from the end, this reads [`TAIL`] bytes and takes the last
//! occurrence, and a file that does not state one answers `None` — which is itself a stable
//! component, since a rebuilt document does not acquire one by being read again.
//!
//! # Where this sits in RFC 0003 section 6's posture
//!
//! On the **broker's** side. The frontends and the core "never parse PDF bytes", and this does
//! not: it stats a file and looks for an ASCII keyword in a fixed-size window. Everything that
//! reads the document as a document is behind [`crate::worker::Worker`].

use std::io::{self, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pdf_syntax::FileBytes;

/// How many bytes of the file's end are searched for the last `startxref`.
///
/// ISO 32000-2 §7.5.5 puts the keyword two lines from the end, so a well-formed file needs a few
/// dozen. The window is larger because a real file may carry trailing white space, a second
/// `%%EOF`, or a producer's comment after it; it is *fixed* because this is a key and not a
/// recovery — a document whose `startxref` is further back than this answers `None` and is then
/// keyed on its other two components, which is a weaker key and never a wrong one.
pub const TAIL: u64 = 4096;

/// What a document is, for the purpose of deciding whether it is still the same document.
///
/// `Eq` is the whole point: [`crate::Vfs`] compares the key it holds with the key the backing
/// answers now, and rebuilds on any difference. There is no ordering and no "close enough".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generation {
    /// The modification time in nanoseconds since the Unix epoch, where the file system states
    /// one. `None` for a backing that has no such notion, which is not a failure: the other two
    /// components still key it.
    pub modified_nanos: Option<i128>,
    /// The file's length in bytes.
    pub size: u64,
    /// The offset ISO 32000-2 §7.5.5's last `startxref` line states, where the tail states one.
    pub startxref: Option<u64>,
}

/// The file behind the mount, and the two questions this crate asks of it.
///
/// A trait rather than a path, for two reasons that are the same reason twice. RFC 0003
/// section 6 puts file I/O in the **frontends**, so the concrete answer belongs to whoever opened
/// the file — a FUSE daemon with a path, or a broker holding a descriptor it will pass on with
/// `SCM_RIGHTS` (ADR 0812, `pdf_syntax::FileBytes::from_handle`). And a test that must change
/// the document under the mount needs a backing it can change, which is what
/// [`crate::testing::MemoryBacking`] is.
pub trait Backing: Send + Sync + std::fmt::Debug {
    /// The document's generation key, as it is right now.
    ///
    /// # Errors
    ///
    /// The file system's, for a file that cannot be stat'd or read.
    fn generation(&self) -> io::Result<Generation>;

    /// The bytes, for a reader.
    ///
    /// Called once per generation: [`crate::Vfs`] hands the result to a worker and keeps the
    /// worker until the key changes, which is the same shape as passing a descriptor across a
    /// boundary once.
    ///
    /// # Errors
    ///
    /// The file system's.
    fn bytes(&self) -> io::Result<FileBytes>;

    /// What a face calls the document, for a message.
    fn describe(&self) -> String;
}

/// A backing that is a file on this file system.
#[derive(Debug)]
pub struct FileBacking {
    /// Where it is.
    path: PathBuf,
}

impl FileBacking {
    /// The file at `path`. Nothing is opened here.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Where it is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Backing for FileBacking {
    fn generation(&self) -> io::Result<Generation> {
        let mut file = std::fs::File::open(&self.path)?;
        let metadata = file.metadata()?;
        if metadata.is_dir() {
            return Err(io::Error::from(io::ErrorKind::IsADirectory));
        }
        let size = metadata.len();
        let modified_nanos = metadata.modified().ok().map(|time| {
            match time.duration_since(std::time::UNIX_EPOCH) {
                Ok(since) => i128::try_from(since.as_nanos()).unwrap_or(i128::MAX),
                // A timestamp before the epoch is a file system's business and not a reason to
                // fail; it is still a *key*, and negating it keeps the two sides distinct.
                Err(before) => i128::try_from(before.duration().as_nanos())
                    .map_or(i128::MIN, i128::saturating_neg),
            }
        });
        let mut tail = vec![0_u8; usize::try_from(TAIL.min(size)).unwrap_or(0)];
        if !tail.is_empty() {
            file.seek(SeekFrom::Start(size.saturating_sub(TAIL)))?;
            file.read_exact(&mut tail)?;
        }
        Ok(Generation {
            modified_nanos,
            size,
            startxref: startxref_in(&tail),
        })
    }

    fn bytes(&self) -> io::Result<FileBytes> {
        FileBytes::on_disk(&self.path)
    }

    fn describe(&self) -> String {
        self.path.display().to_string()
    }
}

/// The offset the last `startxref` in `tail` states.
///
/// ISO 32000-2 §7.5.5's line is `startxref`, a line break, and a decimal offset. Nothing here
/// validates the offset against the file — that is the reader's job and this is a key — but a
/// value larger than an offset can be is not one, and answers `None`.
fn startxref_in(tail: &[u8]) -> Option<u64> {
    const KEYWORD: &[u8] = b"startxref";
    let at = tail
        .windows(KEYWORD.len())
        .rposition(|window| window == KEYWORD)?;
    let after = tail.get(at.saturating_add(KEYWORD.len())..)?;
    let digits: Vec<u8> = after
        .iter()
        .copied()
        .skip_while(u8::is_ascii_whitespace)
        .take_while(u8::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        return None;
    }
    std::str::from_utf8(&digits).ok()?.parse().ok()
}

/// A backing whose bytes a test holds and can replace, so that "the file changed under the
/// mount" is a thing a test can *do* rather than a thing it simulates.
#[derive(Debug)]
pub struct MemoryBacking {
    /// The bytes, and the modification time a change bumps.
    state: Mutex<(Vec<u8>, i128)>,
    /// What it is called.
    name: String,
}

impl MemoryBacking {
    /// A backing holding these bytes.
    #[must_use]
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            state: Mutex::new((bytes, 0)),
            name: name.into(),
        }
    }

    /// Replaces the document, as another program's rewrite would.
    ///
    /// The modification time is advanced as well as the bytes, because a file system would
    /// advance it and a key that only ever changed because the content did would not be
    /// exercising the key.
    pub fn replace(&self, bytes: Vec<u8>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.0 = bytes;
        state.1 = state.1.saturating_add(1);
    }
}

impl Backing for MemoryBacking {
    fn generation(&self) -> io::Result<Generation> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let size = u64::try_from(state.0.len()).unwrap_or(u64::MAX);
        let from = usize::try_from(size.saturating_sub(TAIL)).unwrap_or(0);
        let tail = state.0.get(from..).unwrap_or(&[]);
        Ok(Generation {
            modified_nanos: Some(state.1),
            size,
            startxref: startxref_in(tail),
        })
    }

    fn bytes(&self) -> io::Result<FileBytes> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(FileBytes::from(state.0.clone()))
    }

    fn describe(&self) -> String {
        self.name.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{Backing as _, MemoryBacking, startxref_in};

    #[test]
    fn the_last_startxref_is_the_one_read() {
        let tail = b"startxref\n11\n%%EOF\nstartxref\n2345\n%%EOF\n";
        assert_eq!(startxref_in(tail), Some(2345));
    }

    #[test]
    fn a_tail_with_no_keyword_states_no_offset() {
        assert_eq!(startxref_in(b"%%EOF\n"), None);
        assert_eq!(startxref_in(b"startxref\n"), None);
    }

    #[test]
    fn replacing_the_bytes_changes_the_key() {
        let backing = MemoryBacking::new("d", b"startxref\n9\n%%EOF\n".to_vec());
        let before = backing.generation().expect("keyed");
        backing.replace(b"startxref\n77\n%%EOF\n".to_vec());
        let after = backing.generation().expect("keyed");
        assert_ne!(before, after);
        assert_eq!(after.startxref, Some(77));
    }
}
