//! The file, as this reader holds it — and the one way it is read from disk.
//!
//! A PDF is addressed by byte offset from its first byte: ISO 32000-2 §7.5.4 makes each
//! cross-reference entry "a 10-digit byte offset in the decoded stream … giving the number of
//! bytes from the beginning of the PDF file to the beginning of the object", §7.5.8.3's Table 18
//! does the same for a cross-reference stream's type 1 entries, and §7.5.5's `startxref` is one
//! more offset. So a reader holds the whole file, and how it holds it decides what a large file
//! costs. §C.4 says what "large" can be (the `10 10` is the conversion's rendering of 10¹⁰):
//!
//! > A PDF cross-reference table (see 7.5.4, "Cross-reference table") allocates ten digits to
//! > represent byte offsets, which limits the size of a PDF file to 10 10 bytes (approximately
//! > 10 gigabytes). However crossreference streams (see 7.5.8, "Cross-reference streams") allow
//! > PDF files to be even larger.
//!
//! Two things follow, and both are this module's (ADR 0795):
//!
//! - **The bytes a host read are the bytes the document holds.** [`Document`] used to keep an
//!   `Arc<[u8]>`, and `Arc<[u8]>: From<Vec<u8>>` *copies* — an `Arc` needs a header the `Vec`
//!   has no room for — so every open cost the file's length twice, and the second copy was the
//!   one allocation in that path that could not fail gracefully. A 6 001 925 614-byte bug-report
//!   attachment (`batch5/poppler`'s `poppler-44085-1.xz-0.pdf`, `doc/todo/03` section 41) was read
//!   whole and then aborted the process on the copy. [`FileBytes`] holds a `Vec<u8>` as it was
//!   given, behind an `Arc` so that a document can still be shared across threads, and copies
//!   only what arrived as a borrowed slice.
//! - **The room is asked for before the first byte is read.** [`read_file`] reserves the file's
//!   whole length with `try_reserve_exact` and answers [`NoRoom`] — the length, by name — where
//!   the process cannot hold it. There is deliberately **no number here**: `doc/todo/10`'s brief
//!   is that a bound on an honest document is not this program's to set, and a 5.6 GB PDF is
//!   an honest document on a machine that holds it. What refuses is the process's own limit —
//!   `RLIMIT_DATA`, a cgroup, the confined worker's ceiling — asked once, before any allocation,
//!   and the answer is typed rather than an abort.
//!
//! [`Document`]: crate::Document

use std::fmt;
use std::fs::File;
use std::io::{self, Read as _};
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

/// The bytes a document was opened from, held once and shared by reference.
///
/// Built from whatever a caller has — a `Vec<u8>` a host read, an `Arc<[u8]>` two readers
/// share, or a borrowed slice in a test — and a `Vec` is *taken* rather than copied: see the
/// module comment for why that is the whole point.
///
/// Clones are cheap and name the same allocation, which [`FileBytes::same`] tests; a cache
/// that keys on a document holds a clone so that the address it compares cannot be reused
/// underneath it.
#[derive(Clone)]
pub struct FileBytes(Held);

/// How the bytes are held: as the `Vec` a host handed over, or as a slice already shared.
#[derive(Clone)]
enum Held {
    /// A `Vec<u8>` taken as it was given — no copy, one small allocation for the `Arc`.
    Owned(Arc<Vec<u8>>),
    /// A slice that was already reference-counted when it arrived.
    Shared(Arc<[u8]>),
}

impl FileBytes {
    /// Whether the two handles name the same bytes in memory.
    ///
    /// An address is a name only while something holds the allocation; a caller comparing
    /// one keeps a clone of what it compares against, as `pdf_model`'s font cache does.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        std::ptr::eq(self.as_ptr(), other.as_ptr()) && self.len() == other.len()
    }
}

impl Deref for FileBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match &self.0 {
            Held::Owned(vec) => vec,
            Held::Shared(slice) => slice,
        }
    }
}

impl AsRef<[u8]> for FileBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl fmt::Debug for FileBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileBytes({} bytes)", self.len())
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
/// [`io::ErrorKind::OutOfMemory`], so a host can read the length back out of it.
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

    /// The reason this type exists: a `Vec` handed over is held where it was, not copied.
    #[test]
    fn a_vector_is_held_where_it_was() {
        let bytes = b"%PDF-1.7\n".to_vec();
        let address = bytes.as_ptr();
        let held = FileBytes::from(bytes);
        assert!(std::ptr::eq(held.as_ptr(), address));
        assert_eq!(&*held, b"%PDF-1.7\n");
    }

    /// A shared slice is shared rather than copied, and a borrowed one has to be copied.
    #[test]
    fn a_shared_slice_is_shared_and_a_borrowed_one_is_copied() {
        let shared: Arc<[u8]> = Arc::from(b"abc".as_slice());
        let held = FileBytes::from(Arc::clone(&shared));
        assert!(std::ptr::eq(held.as_ptr(), shared.as_ptr()));

        let borrowed = b"abc".as_slice();
        let held = FileBytes::from(borrowed);
        assert!(!std::ptr::eq(held.as_ptr(), borrowed.as_ptr()));
        assert_eq!(&*held, borrowed);
    }

    /// Clones name the same allocation and distinct constructions do not.
    #[test]
    fn same_is_identity_and_not_equality() {
        let one = FileBytes::from(b"abc".to_vec());
        let clone = one.clone();
        let other = FileBytes::from(b"abc".to_vec());
        assert!(one.same(&clone));
        assert!(!one.same(&other));
        assert_eq!(&*one, &*other);
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
        assert!(std::ptr::eq(document.bytes().as_ptr(), address));
        assert_eq!(document.bytes().len(), length);
        assert!(document.catalog().is_ok());
    }

    /// An ordinary file reads as `std::fs::read` reads it.
    #[test]
    fn an_ordinary_file_is_read_whole() {
        let directory =
            std::env::temp_dir().join(format!("pdf-syntax-file-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        let path = directory.join("small.pdf");
        std::fs::write(&path, b"%PDF-1.7\n%%EOF\n").expect("a small file");
        let bytes = read_file(&path).expect("a small file is readable");
        assert_eq!(bytes, b"%PDF-1.7\n%%EOF\n");
        assert!(read_file(&directory.join("absent.pdf")).is_err());
        std::fs::remove_dir_all(&directory).expect("the temporary directory is removable");
    }
}
