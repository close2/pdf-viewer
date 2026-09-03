//! The file, as this reader holds it — whole in memory, or open on disk and read where the
//! file's own offsets point.
//!
//! A PDF is addressed by byte offset from its first byte: ISO 32000-2 §7.5.4 makes each
//! cross-reference entry "a 10-digit byte offset in the decoded stream … giving the number of
//! bytes from the beginning of the PDF file to the beginning of the object", §7.5.8.3's Table 18
//! does the same for a cross-reference stream's type 1 entries, and §7.5.5's `startxref` is one
//! more offset. So a reader that has the offsets has no need of the bytes between them, and how
//! it holds the file decides what a large file costs. §C.4 says what "large" can be (the `10 10`
//! is the conversion's rendering of 10¹⁰):
//!
//! > A PDF cross-reference table (see 7.5.4, "Cross-reference table") allocates ten digits to
//! > represent byte offsets, which limits the size of a PDF file to 10 10 bytes (approximately
//! > 10 gigabytes). However crossreference streams (see 7.5.8, "Cross-reference streams") allow
//! > PDF files to be even larger.
//!
//! Three things follow, and all are this module's:
//!
//! - **The bytes a host read are the bytes the document holds** (ADR 0795). [`Document`] used to
//!   keep an `Arc<[u8]>`, and `Arc<[u8]>: From<Vec<u8>>` *copies* — an `Arc` needs a header the
//!   `Vec` has no room for — so every open cost the file's length twice. [`FileBytes`] holds a
//!   `Vec<u8>` as it was given, behind an `Arc` so that a document can still be shared across
//!   threads, and copies only what arrived as a borrowed slice.
//! - **The room is asked for before the first byte is read** (ADR 0795). [`read_file`] reserves
//!   the file's whole length with `try_reserve_exact` and answers [`NoRoom`] — the length, by
//!   name — where the process cannot hold it. There is deliberately **no number here**:
//!   `doc/todo/10`'s brief is that a bound on an honest document is not this program's to set.
//!   What refuses is the process's own limit, asked once, and the answer is typed rather than an
//!   abort.
//! - **A file on disk is read where its offsets point, and nowhere else** (ADR 0809).
//!   [`FileBytes::on_disk`] opens the file and holds the handle; every reader in this crate then
//!   asks for the bytes *from an offset* through [`FileBytes::parse_from`], which hands a parser
//!   a window that grows until the parse depended on nothing at its end. `CLAUDE.md`'s startup
//!   rule — "[o]pening a document reads the trailer and the objects page one needs — not the
//!   whole file" — was true of the *parsing* and false of the *bytes* until this: a 6 GB document
//!   (`batch5/poppler`'s `poppler-44085-1.xz-0.pdf`, 2000 pages, `doc/todo/03` section 41) cost
//!   5.6 GiB of resident memory and half a second to over a second of reading before its trailer
//!   was looked at, and costs the trailer, the table and page one's objects now.
//!
//! **The interpretation is a function of the bytes alone, whichever way they are held.** That is
//! the oracle's premise (`CLAUDE.md` on the immutable `Document`), and it is kept by construction
//! rather than by care: a parser given a window is given the *same slice* it would see from the
//! offset in a whole file, and a window is accepted only where the parse examined nothing at its
//! end — the window's last byte is either the file's last byte or a byte the outcome did not
//! depend on. Where a reader needs the file whole — a scan for object headers, a `startxref`
//! that is not in the last two kilobytes — it asks [`FileBytes::whole`], which reads it once and
//! keeps it, or refuses by name where the process cannot hold it. So a damaged file costs on disk
//! what it costs in memory, and an intact one costs what it names.
//!
//! [`Document`]: crate::Document

use std::borrow::Cow;
use std::fmt;
use std::fs::File;
use std::io::{self, Read as _};
use std::ops::Range;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

/// The bytes a document was opened from: held once in memory, or open on disk.
///
/// Built from whatever a caller has — a `Vec<u8>` a host read, an `Arc<[u8]>` two readers
/// share, a borrowed slice in a test, or a path through [`FileBytes::on_disk`] — and a `Vec` is
/// *taken* rather than copied: see the module comment for why that is the whole point.
///
/// Clones are cheap and name the same bytes, which [`FileBytes::same`] tests; a cache that keys
/// on a document holds a clone so that the identity it compares cannot be reused underneath it.
///
/// A file on disk is read through [`Self::read`], [`Self::parse_from`] and [`Self::whole`];
/// there is no `Deref` to a slice, because a slice of the whole file is exactly the thing a
/// reader of a large file must not need.
#[derive(Clone)]
pub struct FileBytes(Held);

/// How the bytes are held.
#[derive(Clone)]
enum Held {
    /// A `Vec<u8>` taken as it was given — no copy, one small allocation for the `Arc`.
    Owned(Arc<Vec<u8>>),
    /// A slice that was already reference-counted when it arrived.
    Shared(Arc<[u8]>),
    /// A file open on disk, read where the document's offsets point.
    OnDisk(Arc<OnDisk>),
}

/// A file open on disk, and what has been read of it.
struct OnDisk {
    file: File,
    /// The file's length when it was opened. A file that grows afterwards is read to this
    /// length; one that shrinks reads as though it ended where it now does. Both are outside
    /// the contract — a document is a function of its bytes, and bytes that change are another
    /// document — and neither is a panic or an abort.
    length: usize,
    /// The whole file, once something needed it whole. See [`FileBytes::whole`].
    whole: OnceLock<Vec<u8>>,
    /// The first read that did not deliver what was asked for, kept so a host can ask.
    failure: Mutex<Option<ReadFailure>>,
    /// Serialises reads on a platform whose positional read moves the file's cursor.
    #[cfg(not(unix))]
    cursor: Mutex<()>,
}

/// A read of the file on disk that did not deliver what was asked for.
///
/// Kept by the file and answered by [`FileBytes::read_failure`]. A short read is treated as
/// the end of the file — the bytes that arrived are the file's own and the parse goes on over
/// them — so this is the record of *why* the file ended early, for a host that asks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("reading {wanted} bytes at offset {at} of the file on disk failed: {message}")]
pub struct ReadFailure {
    /// The offset the read began at.
    pub at: usize,
    /// How many bytes were asked for.
    pub wanted: usize,
    /// The operating system's own sentence, or the allocation refusal's.
    pub message: String,
}

/// How many bytes a window starts with where the caller gave no better estimate.
///
/// Four kibibytes is one page of memory and holds every object in this tree's corpus that is
/// not a stream; a window grows by doubling, and by the parser's own statement of what it
/// needed where it made one, so the constant decides the first read and not the last.
pub(crate) const FIRST_WINDOW: usize = 4096;

impl FileBytes {
    /// Opens a file on disk, to be read where the document's offsets point.
    ///
    /// Nothing is read here but the file's length. A directory is refused rather than read as
    /// an empty file, so that a host's own sentence about a path it cannot open stays true.
    ///
    /// # Errors
    ///
    /// The file system's, for a path that cannot be opened; and [`NoRoom`] under
    /// [`io::ErrorKind::OutOfMemory`] for a length this reader cannot address at all.
    pub fn on_disk(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if metadata.is_dir() {
            return Err(io::Error::from(io::ErrorKind::IsADirectory));
        }
        let length = usize::try_from(metadata.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::OutOfMemory,
                NoRoom {
                    length: metadata.len(),
                },
            )
        })?;
        Ok(Self(Held::OnDisk(Arc::new(OnDisk {
            file,
            length,
            whole: OnceLock::new(),
            failure: Mutex::new(None),
            #[cfg(not(unix))]
            cursor: Mutex::new(()),
        }))))
    }

    /// The file's length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.0 {
            Held::Owned(bytes) => bytes.len(),
            Held::Shared(bytes) => bytes.len(),
            Held::OnDisk(disk) => disk.length,
        }
    }

    /// Whether there are no bytes at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the file is open on disk rather than held in memory.
    #[must_use]
    pub fn is_on_disk(&self) -> bool {
        matches!(self.0, Held::OnDisk(_))
    }

    /// Whether the two handles name the same bytes.
    ///
    /// In memory that is the same allocation; on disk it is the same open file. An address is a
    /// name only while something holds it; a caller comparing one keeps a clone of what it
    /// compares against, as `pdf_model`'s font cache does.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Held::Owned(this), Held::Owned(that)) => Arc::ptr_eq(this, that),
            (Held::Shared(this), Held::Shared(that)) => Arc::ptr_eq(this, that),
            (Held::OnDisk(this), Held::OnDisk(that)) => Arc::ptr_eq(this, that),
            _ => false,
        }
    }

    /// The bytes in `range`, clipped to the file: borrowed where the file is in memory and
    /// read where it is on disk.
    ///
    /// A range past the end comes back short, as a slice's `get` would — the caller checks the
    /// length where the length is the question. On disk a read that fails comes back short too,
    /// and [`Self::read_failure`] says why.
    #[must_use]
    pub fn read(&self, range: Range<usize>) -> Cow<'_, [u8]> {
        match &self.0 {
            Held::Owned(bytes) => Cow::Borrowed(clipped(bytes, range)),
            Held::Shared(bytes) => Cow::Borrowed(clipped(bytes, range)),
            Held::OnDisk(disk) => {
                Cow::Owned(disk.window(range.start, range.end.saturating_sub(range.start)))
            }
        }
    }

    /// The whole file, read once and kept where it is on disk.
    ///
    /// This is what a *scan* asks for — a rebuild of the cross-reference table, a search for a
    /// `startxref` the last two kilobytes do not carry, a look for every damaged dictionary —
    /// because a scan reads every byte and a window at a time would read them all twice over.
    /// An intact document never asks; a damaged one costs on disk what it costs in memory.
    ///
    /// # Errors
    ///
    /// [`NoRoom`] where the process cannot hold the file, asked with `try_reserve_exact`
    /// before a byte is read, exactly as [`read_file`] asks.
    pub fn whole(&self) -> Result<&[u8], NoRoom> {
        match &self.0 {
            Held::Owned(bytes) => Ok(bytes),
            Held::Shared(bytes) => Ok(bytes),
            Held::OnDisk(disk) => disk.whole(),
        }
    }

    /// The first read of the file on disk that did not deliver what was asked for, if any.
    ///
    /// Always `None` for a file held in memory. A host that wants to know whether a document
    /// read short because of the disk rather than because of the file asks here after the
    /// fact; nothing in this crate stops on a failed read, because the bytes that did arrive
    /// are the file's own and are read as the prefix they are.
    #[must_use]
    pub fn read_failure(&self) -> Option<ReadFailure> {
        match &self.0 {
            Held::OnDisk(disk) => disk.failure.lock().ok().and_then(|held| held.clone()),
            Held::Owned(_) | Held::Shared(_) => None,
        }
    }

    /// Runs `read` over the bytes from `offset`, on a slice that is the same whether the file is
    /// in memory or on disk.
    ///
    /// `read` is given the bytes and whether they stop short of the file's end, and answers
    /// its value beside **how many bytes of the slice the value depended on** — the parser's own
    /// count, [`crate::Parser::examined`]. In memory the slice is the rest of the file and `read`
    /// runs once. On disk it is a window of `first` bytes, which grows — doubling, and to at
    /// least what `read` said it needed — until either the window reaches the file's end or the
    /// value depended on nothing at the window's end; only then is the value taken. So a value
    /// read through a window is the value the whole file would have given, because every byte it
    /// depended on was the same byte.
    ///
    /// An offset at or past the end is an empty slice that is not short of the end, which is what
    /// a slice's `get(offset..)` would give a whole file's reader: §7.5.4's offset pointing past
    /// the file is a common corruption and the caller recovers by scanning.
    pub(crate) fn parse_from<T>(
        &self,
        offset: usize,
        first: usize,
        mut read: impl FnMut(&[u8], bool) -> (T, usize),
    ) -> T {
        let disk = match &self.0 {
            Held::Owned(bytes) => return read(bytes.get(offset..).unwrap_or_default(), false).0,
            Held::Shared(bytes) => return read(bytes.get(offset..).unwrap_or_default(), false).0,
            Held::OnDisk(disk) => disk,
        };
        let Some(remaining) = disk.length.checked_sub(offset).filter(|rest| *rest > 0) else {
            return read(&[], false).0;
        };
        let mut want = first.clamp(1, remaining);
        loop {
            let window = disk.window(offset, want);
            // A window that stops short of what was asked for is the file's end as far as this
            // reader can see it — a read that failed is recorded, and the bytes that arrived are
            // read as the prefix they are.
            let at_end = window.len() < want || want == remaining;
            let (value, examined) = read(&window, !at_end);
            if at_end || examined < window.len() {
                return value;
            }
            want = want
                .saturating_mul(2)
                .max(examined.saturating_add(1))
                .min(remaining);
        }
    }
}

/// `bytes[range]`, with both ends clipped to the slice rather than panicking.
fn clipped(bytes: &[u8], range: Range<usize>) -> &[u8] {
    let start = range.start.min(bytes.len());
    let end = range.end.clamp(start, bytes.len());
    bytes.get(start..end).unwrap_or_default()
}

impl OnDisk {
    /// Up to `want` bytes from `offset`, fewer at the file's end or where a read failed.
    fn window(&self, offset: usize, want: usize) -> Vec<u8> {
        let want = want.min(self.length.saturating_sub(offset));
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(want).is_err() {
            self.record(
                offset,
                want,
                "this process cannot hold the window".to_owned(),
            );
            return bytes;
        }
        let mut reader = Positional {
            disk: self,
            at: offset,
        };
        if let Err(error) = (&mut reader).take(want as u64).read_to_end(&mut bytes) {
            self.record(offset, want, error.to_string());
        }
        bytes
    }

    /// The whole file, read once. See [`FileBytes::whole`].
    fn whole(&self) -> Result<&[u8], NoRoom> {
        if let Some(bytes) = self.whole.get() {
            return Ok(bytes);
        }
        let mut bytes = hold(self.length as u64)?;
        let mut reader = Positional { disk: self, at: 0 };
        if let Err(error) = (&mut reader)
            .take(self.length as u64)
            .read_to_end(&mut bytes)
        {
            self.record(0, self.length, error.to_string());
        }
        // Two threads asking at once each read the file; the first to finish is kept and the
        // other's copy is dropped, which costs one read and never a wrong answer.
        let _ = self.whole.set(bytes);
        self.whole.get().map(Vec::as_slice).ok_or(NoRoom {
            length: self.length as u64,
        })
    }

    /// Keeps the first failure, which is the one that explains every short read after it.
    fn record(&self, at: usize, wanted: usize, message: String) {
        if let Ok(mut held) = self.failure.lock()
            && held.is_none()
        {
            *held = Some(ReadFailure {
                at,
                wanted,
                message,
            });
        }
    }

    /// One positional read, which does not move any cursor another thread is using.
    #[cfg(unix)]
    fn read_at(&self, at: u64, buffer: &mut [u8]) -> io::Result<usize> {
        use std::os::unix::fs::FileExt as _;
        self.file.read_at(buffer, at)
    }

    /// One positional read, serialised because the platform's moves the file's cursor.
    #[cfg(not(unix))]
    fn read_at(&self, at: u64, buffer: &mut [u8]) -> io::Result<usize> {
        use std::io::{Seek as _, SeekFrom};
        let _held = self.cursor.lock().map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "the file's cursor lock was poisoned")
        })?;
        let mut file = &self.file;
        file.seek(SeekFrom::Start(at))?;
        file.read(buffer)
    }
}

/// A cursor over the file on disk, so that `read_to_end` can fill a vector without this crate
/// touching uninitialised memory.
struct Positional<'a> {
    disk: &'a OnDisk,
    at: usize,
}

impl io::Read for Positional<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.disk.read_at(self.at as u64, buffer)?;
        self.at = self.at.saturating_add(read);
        Ok(read)
    }
}

impl fmt::Debug for FileBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let held = match &self.0 {
            Held::Owned(_) | Held::Shared(_) => "in memory",
            Held::OnDisk(_) => "on disk",
        };
        write!(f, "FileBytes({} bytes, {held})", self.len())
    }
}

impl Default for FileBytes {
    /// No bytes at all — what [`crate::Document::empty`] holds.
    fn default() -> Self {
        Self(Held::Owned(Arc::new(Vec::new())))
    }
}

impl From<Vec<u8>> for FileBytes {
    /// Takes the vector as it is: no copy of its bytes.
    fn from(bytes: Vec<u8>) -> Self {
        Self(Held::Owned(Arc::new(bytes)))
    }
}

impl From<Arc<[u8]>> for FileBytes {
    fn from(bytes: Arc<[u8]>) -> Self {
        Self(Held::Shared(bytes))
    }
}

impl From<Box<[u8]>> for FileBytes {
    fn from(bytes: Box<[u8]>) -> Self {
        Self(Held::Owned(Arc::new(bytes.into_vec())))
    }
}

impl From<&[u8]> for FileBytes {
    /// A borrowed slice has to be copied to be held; this is the one route that copies.
    fn from(bytes: &[u8]) -> Self {
        Self(Held::Owned(Arc::new(bytes.to_vec())))
    }
}

impl From<&Vec<u8>> for FileBytes {
    fn from(bytes: &Vec<u8>) -> Self {
        Self::from(bytes.as_slice())
    }
}

impl<const N: usize> From<[u8; N]> for FileBytes {
    fn from(bytes: [u8; N]) -> Self {
        Self::from(bytes.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for FileBytes {
    fn from(bytes: &[u8; N]) -> Self {
        Self::from(bytes.as_slice())
    }
}

/// A file this process cannot hold, refused before a byte of it was read.
///
/// Carried inside the [`io::Error`] that [`read_file`] returns, under
/// [`io::ErrorKind::OutOfMemory`], so a host can read the length back out of it; and answered
/// by [`FileBytes::whole`] where a file open on disk is needed whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the file is {length} bytes and this process cannot hold it")]
pub struct NoRoom {
    /// The file's length, as the file system stated it.
    pub length: u64,
}

/// Reads a whole file, having first asked for the room to hold it.
///
/// The length comes from the file's metadata and the room from `try_reserve_exact`, so the
/// process's own limit answers before the first byte is read; a file it cannot hold comes back
/// as [`io::ErrorKind::OutOfMemory`] carrying a [`NoRoom`] that names the length. Everything
/// else is [`std::fs::read`]'s behaviour, including a file that grew between the two calls,
/// which `read_to_end` follows.
///
/// This is the route for a host that has to *hand the bytes on* — the confined viewer's, whose
/// worker has no file system and receives the document over a pipe. A host that opens the
/// document itself uses [`FileBytes::on_disk`] and reads none of the file it does not need.
///
/// # Errors
///
/// The file system's, for a file that cannot be opened or read; and [`NoRoom`] as above.
pub fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let mut bytes =
        hold(length).map_err(|no_room| io::Error::new(io::ErrorKind::OutOfMemory, no_room))?;
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// An empty vector with room for `length` bytes, or the refusal by name.
///
/// Separated from [`read_file`] so that the refusal can be tested without a file that large:
/// a length above what a `Vec` can address is refused by the same path a length the process
/// cannot afford is.
fn hold(length: u64) -> Result<Vec<u8>, NoRoom> {
    let mut bytes = Vec::new();
    match usize::try_from(length) {
        Ok(fits) if bytes.try_reserve_exact(fits).is_ok() => Ok(bytes),
        _ => Err(NoRoom { length }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of this test's own, named after the process so parallel rounds cannot share it.
    fn scratch(name: &str) -> std::path::PathBuf {
        let directory =
            std::env::temp_dir().join(format!("pdf-syntax-file-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        directory
    }

    /// The reason this type exists: a `Vec` handed over is held where it was, not copied.
    #[test]
    fn a_vector_is_held_where_it_was() {
        let bytes = b"%PDF-1.7\n".to_vec();
        let address = bytes.as_ptr();
        let held = FileBytes::from(bytes);
        assert!(std::ptr::eq(
            held.whole().expect("in memory").as_ptr(),
            address
        ));
        assert_eq!(held.whole().expect("in memory"), b"%PDF-1.7\n");
    }

    /// A shared slice is shared rather than copied, and a borrowed one has to be copied.
    #[test]
    fn a_shared_slice_is_shared_and_a_borrowed_one_is_copied() {
        let shared: Arc<[u8]> = Arc::from(b"abc".as_slice());
        let held = FileBytes::from(Arc::clone(&shared));
        assert!(std::ptr::eq(
            held.whole().expect("in memory").as_ptr(),
            shared.as_ptr()
        ));

        let borrowed = b"abc".as_slice();
        let held = FileBytes::from(borrowed);
        assert!(!std::ptr::eq(
            held.whole().expect("in memory").as_ptr(),
            borrowed.as_ptr()
        ));
        assert_eq!(held.whole().expect("in memory"), borrowed);
    }

    /// Clones name the same allocation and distinct constructions do not.
    #[test]
    fn same_is_identity_and_not_equality() {
        let one = FileBytes::from(b"abc".to_vec());
        let clone = one.clone();
        let other = FileBytes::from(b"abc".to_vec());
        assert!(one.same(&clone));
        assert!(!one.same(&other));
        assert_eq!(
            one.whole().expect("in memory"),
            other.whole().expect("in memory")
        );
    }

    /// A length no `Vec` can hold is refused by name, with the length in the refusal.
    #[test]
    fn a_length_the_process_cannot_hold_is_refused_by_name() {
        let refused = hold(u64::MAX).expect_err("no process holds 2^64 bytes");
        assert_eq!(refused, NoRoom { length: u64::MAX });
        assert_eq!(
            refused.to_string(),
            "the file is 18446744073709551615 bytes and this process cannot hold it"
        );
        // `isize::MAX` is where `try_reserve_exact` refuses on capacity alone, whatever the
        // machine; the file this module was written for is 6 001 925 614 bytes and was refused
        // by the process's limit through the same arm.
        let too_long = u64::try_from(isize::MAX)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        assert_eq!(hold(too_long), Err(NoRoom { length: too_long }));
        assert!(hold(0).is_ok_and(|bytes| bytes.is_empty()));
    }

    /// The refusal travels inside an `io::Error` of the kind a host tests for.
    #[test]
    fn a_refused_file_is_an_out_of_memory_error_carrying_the_length() {
        let error = io::Error::new(io::ErrorKind::OutOfMemory, NoRoom { length: 7 });
        assert_eq!(error.kind(), io::ErrorKind::OutOfMemory);
        let inner = error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<NoRoom>())
            .copied();
        assert_eq!(inner, Some(NoRoom { length: 7 }));
    }

    /// The document holds the vector a host read, where it was: no second copy of the file.
    ///
    /// Run against `Arc<[u8]>` storage the addresses differ, because `Arc<[u8]>: From<Vec<u8>>`
    /// copies — which is the allocation that took a 5.6 GB document down (ADR 0795).
    #[test]
    fn a_document_holds_the_vector_it_was_opened_from() {
        use std::fmt::Write as _;

        let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
                    2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n";
        let mut file = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(file.len());
            file.push_str(object);
        }
        let xref_at = file.len();
        file.push_str("xref\n0 3\n0000000000 65535 f \n");
        for offset in offsets {
            let _ = writeln!(file, "{offset:010} 00000 n ");
        }
        let _ = write!(
            file,
            "trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        let bytes = file.into_bytes();
        let address = bytes.as_ptr();
        let length = bytes.len();
        let document = crate::Document::open(bytes).expect("the fixture opens");
        let held = document.bytes().whole().expect("in memory");
        assert!(std::ptr::eq(held.as_ptr(), address));
        assert_eq!(held.len(), length);
        assert!(document.catalog().is_ok());
    }

    /// An ordinary file reads as `std::fs::read` reads it.
    #[test]
    fn an_ordinary_file_is_read_whole() {
        let directory = scratch("whole");
        let path = directory.join("small.pdf");
        std::fs::write(&path, b"%PDF-1.7\n%%EOF\n").expect("a small file");
        let bytes = read_file(&path).expect("a small file is readable");
        assert_eq!(bytes, b"%PDF-1.7\n%%EOF\n");
        assert!(read_file(&directory.join("absent.pdf")).is_err());
        std::fs::remove_dir_all(&directory).expect("the temporary directory is removable");
    }

    /// A file on disk reads nothing at open, answers its length, and reads ranges clipped to
    /// the file exactly as a slice would.
    #[test]
    fn a_file_on_disk_is_read_by_range_and_clipped_at_its_end() {
        let directory = scratch("ranges");
        let path = directory.join("ranged.pdf");
        std::fs::write(&path, b"0123456789").expect("a small file");
        let disk = FileBytes::on_disk(&path).expect("opens");
        let memory = FileBytes::from(b"0123456789".to_vec());
        assert!(disk.is_on_disk() && !memory.is_on_disk());
        assert_eq!(disk.len(), 10);
        let reversed = Range { start: 5, end: 2 };
        for range in [0..0, 0..3, 3..7, 7..10, 8..20, 10..12, 12..14, reversed] {
            assert_eq!(
                disk.read(range.clone()),
                memory.read(range.clone()),
                "{range:?}"
            );
        }
        assert_eq!(disk.whole().expect("small"), b"0123456789");
        assert!(disk.read_failure().is_none());
        assert!(
            FileBytes::on_disk(&directory).is_err(),
            "a directory is refused"
        );
        std::fs::remove_dir_all(&directory).expect("the temporary directory is removable");
    }

    /// A window grows until the parse examined nothing at its end, and never past the file.
    ///
    /// The reader here counts how far it looked and asks for more whenever the window ended
    /// before the closing `>`, which is the shape every parser in this crate reports through
    /// [`crate::Parser::examined`].
    #[test]
    fn a_window_grows_to_what_the_reader_examined_and_stops_at_the_file() {
        let directory = scratch("windows");
        let path = directory.join("windowed.pdf");
        let mut file = b"<".to_vec();
        file.extend(std::iter::repeat_n(b'x', 10_000));
        file.push(b'>');
        file.extend(b"tail");
        std::fs::write(&path, &file).expect("a small file");
        let disk = FileBytes::on_disk(&path).expect("opens");
        let memory = FileBytes::from(file.clone());

        let bracketed = |bytes: &[u8], _short: bool| {
            let close = bytes.iter().position(|byte| *byte == b'>');
            let examined = close.map_or(bytes.len(), |at| at.saturating_add(1));
            (
                close.map(|at| bytes.get(..=at).unwrap_or_default().to_vec()),
                examined,
            )
        };
        let mut attempts = 0usize;
        let from_disk = disk.parse_from(0, 16, |bytes, short| {
            attempts = attempts.saturating_add(1);
            bracketed(bytes, short)
        });
        let from_memory = memory.parse_from(0, 16, bracketed);
        assert_eq!(from_disk, from_memory);
        assert_eq!(from_disk.map(|found| found.len()), Some(10_002));
        assert!(attempts > 1, "a 16-byte window had to grow");

        // From inside the tail, where nothing closes: both readers examine to the end and both
        // answer `None`, and the window is told it is not short once it reaches the file's end.
        let mut told_short = Vec::new();
        let none = disk.parse_from(10_002, 2, |bytes, short| {
            told_short.push(short);
            bracketed(bytes, short)
        });
        assert_eq!(none, None);
        assert_eq!(told_short.last(), Some(&false));
        assert_eq!(memory.parse_from(10_002, 2, bracketed), None);

        // Past the end, both hand the reader nothing.
        assert_eq!(disk.parse_from(20_000, 4, bracketed), None);
        assert_eq!(memory.parse_from(20_000, 4, bracketed), None);
        std::fs::remove_dir_all(&directory).expect("the temporary directory is removable");
    }
}
