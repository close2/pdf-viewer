//! A shared-memory object mapped by the two processes of a stream's ring: the host reads it, the
//! confined worker writes it, and neither ever holds a reference the other can invalidate.
//!
//! `confined_transport::ring` creates the object as a sealed `memfd`, passes it to the worker once
//! with `SCM_RIGHTS`, and says who may touch which slot when (ADR 3513). This module is the part of
//! that which `unsafe` is needed for, and nothing else: mapping is a safe call (`memmap2`'s raw
//! mappings hand out pointers, never slices), and **reading or writing through the pointer is the
//! `unsafe`**, so it is here, in the one crate the tree's sweep names for a confined helper's
//! `unsafe` (ADRs 3361, 3515; `doc/questions/A979`).
//!
//! **The two halves are not symmetric, and the asymmetry is the soundness argument.**
//!
//! - The **host** maps the object read-only ([`Reader`]) and copies a slot out through a raw
//!   pointer into memory it owns. The worker is the untrusted side and can write any byte of the
//!   object at any moment, contract or no contract, so the host never forms a `&[u8]` into the
//!   mapping — a reference whose bytes another process changes under it is exactly what Rust's
//!   aliasing rules forbid. A copy through a pointer reads bytes, `u8` has no invalid value, and the
//!   worst a racing worker can do is tear the frame it is sending, which it could have sent torn.
//! - The **worker** maps the object read-write ([`Writer`]) and is handed `&mut [u8]` slots. That is
//!   sound because the only other mapping is the host's, which is read-only, and the host writes the
//!   object through no other route: nothing else writes the memory the reference covers.
//!
//! The object's size is fixed by its seals before either maps it (`F_SEAL_SHRINK`), so neither
//! mapping can find its pages gone from under it; a mapping past the object's end would fault on
//! access, and the lengths here are the ones the host sealed.

use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};

/// The host's mapping: read-only, read by copying.
#[derive(Debug)]
pub struct Reader {
    map: memmap2::MmapRaw,
}

impl Reader {
    /// Maps the first `length` bytes of `object` shared and read-only.
    ///
    /// # Errors
    ///
    /// The kernel's, where the mapping is refused.
    pub fn map(object: BorrowedFd<'_>, length: usize) -> io::Result<Self> {
        let map = memmap2::MmapOptions::new()
            .len(length)
            .map_raw_read_only(object.as_raw_fd())?;
        Ok(Self { map })
    }

    /// The mapping's length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the mapping is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.len() == 0
    }

    /// Copies `into.len()` bytes from `offset` into `into`, or answers `false` and copies nothing
    /// where that range is not inside the mapping.
    #[expect(
        unsafe_code,
        reason = "copies out of memory another process may be writing, through a pointer and never \
                  a reference (ADR 3515)"
    )]
    #[must_use]
    pub fn copy_out(&self, offset: usize, into: &mut [u8]) -> bool {
        let Some(end) = offset.checked_add(into.len()) else {
            return false;
        };
        if end > self.map.len() {
            return false;
        }
        // SAFETY: `offset..end` is inside the mapping, checked above, and the mapping is alive for
        // `&self`; `into` is this process's own memory and cannot overlap a mapping made by
        // `mmap`. The source may be written by the worker during the copy — the module's argument:
        // no reference to it exists, the copy reads plain bytes, and a racing write can change only
        // which bytes arrive.
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.map.as_ptr().add(offset),
                into.as_mut_ptr(),
                into.len(),
            );
        }
        true
    }
}

/// The worker's mapping: read-write, written through a slice.
#[derive(Debug)]
pub struct Writer {
    map: memmap2::MmapRaw,
}

impl Writer {
    /// Maps the first `length` bytes of the object open as `object` shared and read-write.
    ///
    /// A raw descriptor rather than a borrowed one, because the worker gives up ownership of the
    /// descriptor before it maps (`confined_transport::ring`): closing it is a system call its
    /// profile does not admit, and the descriptor it keeps is what holds the one slot its
    /// descriptor ceiling leaves.
    ///
    /// # Errors
    ///
    /// The kernel's, where the mapping is refused.
    pub fn map(object: RawFd, length: usize) -> io::Result<Self> {
        let map = memmap2::MmapOptions::new().len(length).map_raw(object)?;
        Ok(Self { map })
    }

    /// The mapping's length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the mapping is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.len() == 0
    }

    /// The `length` bytes from `offset`, to write into, or `None` where that range is not inside
    /// the mapping.
    #[expect(
        unsafe_code,
        reason = "a slice of a shared mapping no other process writes (ADR 3515)"
    )]
    pub fn region_mut(&mut self, offset: usize, length: usize) -> Option<&mut [u8]> {
        let end = offset.checked_add(length)?;
        if end > self.map.len() {
            return None;
        }
        // SAFETY: `offset..end` is inside the mapping, checked above, and the slice borrows `self`
        // mutably, so this process makes no second reference to it while it lives. No other process
        // writes it: the only other mapping is the host's, made read-only by `Reader::map`, and the
        // host writes the object by no other route — the module's argument.
        Some(unsafe { std::slice::from_raw_parts_mut(self.map.as_mut_ptr().add(offset), length) })
    }
}
