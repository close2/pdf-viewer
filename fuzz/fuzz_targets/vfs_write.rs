//! Fuzzes `pdf_vfs`'s write side: RFC 0003 section 5.2's five verbs over a document held in
//! memory, each committed as ISO 32000-2 §7.5.6's incremental update (ADRs 0874, 0802).
//!
//! Two inputs are untrusted at once: the document the tree is mounted over, and the bytes a
//! program copies into it — a PDF dropped into `pages/`, a file into `attachments/`, a
//! `meta/info.json` somebody edited by hand. The input is a verb byte, a selector byte, the
//! document's length as four bytes little-endian, the document, and then the bytes written.
//! The worker is `InProcessWorkers`, which runs the very queries the confined one does.
//!
//! Beyond never panicking — overflow checks stay on in this profile — three properties are under
//! test:
//!
//! - **A commit appends.** §7.5.6: "changes shall be appended to the end of the file, leaving its
//!   original contents intact". The backing is asked to commit only bytes that begin with the
//!   file it holds, byte for byte; the broker checks the same property before it writes, and this
//!   is the second reading of it at the one place a write reaches.
//! - **A deleted page is one page.** A committed `DeletePage` leaves the document one page
//!   shorter than its listing was.
//! - **The tree still reads after a commit**: the root lists, which is a worker opening the file
//!   the commit wrote.

#![no_main]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a selector taken modulo a listing's length plus one, which is never zero"
)]

use std::sync::{Arc, Mutex};

use libfuzzer_sys::fuzz_target;
use pdf_syntax::FileBytes;
use pdf_vfs::generation::{Backing, Generation};
use pdf_vfs::{Config, InProcessWorkers, MemoryBacking, Vfs};

/// A backing that holds the file it last committed and refuses, by failing the run, a commit
/// that does not begin with it.
#[derive(Debug)]
struct Appending {
    inner: MemoryBacking,
    held: Mutex<Vec<u8>>,
}

impl Backing for Appending {
    fn generation(&self) -> std::io::Result<Generation> {
        self.inner.generation()
    }
    fn bytes(&self) -> std::io::Result<FileBytes> {
        self.inner.bytes()
    }
    fn describe(&self) -> String {
        self.inner.describe()
    }
    fn commit(&self, bytes: &[u8]) -> std::io::Result<()> {
        let mut held = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            bytes.starts_with(&held),
            "§7.5.6: a commit of {} bytes does not begin with the {} the file held",
            bytes.len(),
            held.len()
        );
        held.clear();
        held.extend_from_slice(bytes);
        self.inner.commit(bytes)
    }
}

/// Wraps the shared backing so that the tree owns a box and the target keeps a handle.
#[derive(Debug)]
struct Shared(Arc<Appending>);

impl Backing for Shared {
    fn generation(&self) -> std::io::Result<Generation> {
        self.0.generation()
    }
    fn bytes(&self) -> std::io::Result<FileBytes> {
        self.0.bytes()
    }
    fn describe(&self) -> String {
        self.0.describe()
    }
    fn commit(&self, bytes: &[u8]) -> std::io::Result<()> {
        self.0.commit(bytes)
    }
}

/// The names a directory lists, in its order, or none where it does not list.
fn names(vfs: &Vfs, directory: &str) -> Vec<String> {
    vfs.list(directory)
        .map(|entries| entries.into_iter().map(|entry| entry.name).collect())
        .unwrap_or_default()
}

/// One of `names` by `selector`, or a name one past them shaped like the first.
fn chosen(names: &[String], selector: u8, past: impl FnOnce() -> String) -> String {
    let index = usize::from(selector) % names.len().saturating_add(1);
    names.get(index).cloned().unwrap_or_else(past)
}

fuzz_target!(|data: &[u8]| {
    let Some((&[verb, selector], rest)) = data.split_first_chunk::<2>() else {
        return;
    };
    let Some((length, rest)) = rest.split_first_chunk::<4>() else {
        return;
    };
    let length = usize::try_from(u32::from_le_bytes(*length)).unwrap_or(usize::MAX);
    let Some((document, written)) = rest.split_at_checked(length.min(rest.len())) else {
        return;
    };
    let backing = Arc::new(Appending {
        inner: MemoryBacking::new("fuzzed.pdf", document.to_vec()),
        held: Mutex::new(document.to_vec()),
    });
    let vfs = Vfs::new(
        Box::new(Shared(Arc::clone(&backing))),
        Box::new(InProcessWorkers),
        Config::default(),
    );
    let Ok(pages) = vfs.pages() else {
        return;
    };

    let committed = match verb % 6 {
        0 => {
            let listed = names(&vfs, "/pages");
            let width = listed
                .first()
                .map_or(4, |name| name.len().saturating_sub(".pdf".len()));
            let name = chosen(&listed, selector, || {
                format!("{:0width$}.pdf", pages.saturating_add(1))
            });
            vfs.write(&format!("/pages/{name}"), written)
        }
        1 => {
            let listed = names(&vfs, "/pages");
            let name = chosen(&listed, selector, || "0000.pdf".to_owned());
            let removed = vfs.remove(&format!("/pages/{name}"));
            if let Ok(committed) = &removed {
                assert_eq!(
                    committed.pages.saturating_add(1),
                    pages,
                    "a committed page deletion leaves one page fewer"
                );
            }
            removed
        }
        2 => {
            let (name, payload) = written.split_at(written.len().min(usize::from(selector % 24)));
            let name = String::from_utf8_lossy(name);
            vfs.write(&format!("/attachments/{name}"), payload)
        }
        3 => {
            let listed = names(&vfs, "/attachments");
            let name = chosen(&listed, selector, || "absent".to_owned());
            vfs.remove(&format!("/attachments/{name}"))
        }
        4 => vfs.write("/meta/info.json", written),
        _ => {
            // The transaction as a file system drives it: a write at an offset, which leaves a
            // gap of zero bytes, and a truncation, before the flush that commits.
            let Ok(id) = vfs.create("/attachments/staged.bin") else {
                return;
            };
            let offset = u64::from(selector % 16);
            let staged = vfs
                .write_at(id, offset, written)
                .and_then(|_| vfs.truncate(id, u64::from(selector).saturating_mul(3)))
                .and_then(|()| vfs.flush(id));
            vfs.release(id);
            staged
        }
    };
    if committed.is_ok() {
        assert!(
            vfs.list("/").is_ok(),
            "the root lists after a committed write"
        );
    }
});
