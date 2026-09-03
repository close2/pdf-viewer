//! The FUSE face driven the way a kernel drives it, with no kernel and no mount.
//!
//! **A mount in a gate is a different question from a mount by hand**, and this file is the first
//! of the two. `fuser`'s pure-Rust path asks the kernel for `/dev/fuse` and runs `fusermount3` to
//! attach it, so a gate that mounted would be measuring the machine's kernel configuration, its
//! `fuse` group membership and `/etc/fuse.conf` — none of which is a property of this tree, and
//! any of which turns the gate into a coin toss (the same argument `viewer-ffi`'s C-compiler gate
//! makes for skipping). So what is held here is everything between the kernel's verb and the
//! core's answer:
//!
//! - the inode table, which is the one piece of state a FUSE face has that a core does not;
//! - `lookup`, `getattr` and `readdir` over RFC 0003 section 4's tree;
//! - `open`/`read`, and RFC 0003 section 5.4's rule that an open file keeps its generation;
//! - `create`/`write`/`flush`/`release`, which is what `cp` into a mount *is*, with §7.5.6's
//!   prefix property read off the file afterwards;
//! - every refusal RFC 0003 section 5.3 argues for, as the `errno` the kernel is handed.
//!
//! What is *not* held here is `fuser`'s wire format: the reply objects can only be made by a
//! session that has a channel. The one thing that could be checked without one is the table that
//! turns a [`pdf_vfs::Errno`] into a `fuser::Errno`, and that is checked in `kernel.rs`'s own
//! unit tests, exhaustively, against the numbers the core states.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::sync::{Arc, Mutex};

use pdf_fuse::{Face, ROOT};
use pdf_vfs::layout::Kind;
use pdf_vfs::worker::InProcessWorkers;
use pdf_vfs::{Config, Errno, MemoryBacking, Vfs};

/// A committed document, which every checkout has once `doc/specifications.zip` is unpacked.
fn committed(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

/// The five-page annex, which has an outline and §12.4.2 labels.
const FIVE_PAGES: &str = "PDF20_AN001-BPC.pdf";
/// The fourteen-page annex about associated files.
const FOURTEEN_PAGES: &str = "PDF20_AN002-AF.pdf";

/// A backing the test and the face both hold, so that "what reached the file" is readable here.
#[derive(Debug)]
struct SharedBacking(Arc<MemoryBacking>);

impl pdf_vfs::generation::Backing for SharedBacking {
    fn generation(&self) -> std::io::Result<pdf_vfs::generation::Generation> {
        self.0.generation()
    }
    fn bytes(&self) -> std::io::Result<pdf_syntax::FileBytes> {
        self.0.bytes()
    }
    fn describe(&self) -> String {
        self.0.describe()
    }
    fn commit(&self, bytes: &[u8]) -> std::io::Result<()> {
        self.0.commit(bytes)
    }
}

/// Everything the face logged, which is where RFC 0003 section 5.3's sentences go.
type Journal = Arc<Mutex<Vec<String>>>;

/// A face over a committed document, with its backing and its log.
fn mounted(name: &str) -> (Arc<MemoryBacking>, Face, Journal) {
    let bytes = std::fs::read(committed(name)).expect("a committed document");
    let backing = Arc::new(MemoryBacking::new(name, bytes));
    let journal: Journal = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&journal);
    let face = Face::new(
        Vfs::new(
            Box::new(SharedBacking(Arc::clone(&backing))),
            Box::new(InProcessWorkers),
            Config::default(),
        ),
        Box::new(move |sentence: &str| {
            sink.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(sentence.to_owned());
        }),
    );
    (backing, face, journal)
}

/// The whole document, as it is on the backing right now.
fn on_disk(backing: &MemoryBacking) -> Vec<u8> {
    use pdf_vfs::generation::Backing as _;
    let bytes = backing.bytes().expect("the backing reads");
    bytes.read(0..bytes.len()).into_owned()
}

/// The names one directory holds, sorted.
fn names(face: &Face, ino: u64) -> Vec<String> {
    let mut out: Vec<String> = face
        .readdir(ino)
        .expect("a directory")
        .into_iter()
        .map(|node| {
            node.path
                .rsplit_once('/')
                .map_or(node.path.clone(), |(_, name)| name.to_owned())
        })
        .collect();
    out.sort();
    out
}

/// Walks a path from the root the way the kernel does: one `lookup` per component.
fn walk(face: &Face, path: &str) -> Result<pdf_fuse::Node, Errno> {
    let mut ino = ROOT;
    let mut node = face.getattr(ROOT)?;
    for component in path.split('/').filter(|part| !part.is_empty()) {
        node = face.lookup(ino, component)?;
        ino = node.ino;
    }
    Ok(node)
}

/// The root is RFC 0003 section 4's six directories, and every inode is its own.
#[test]
fn the_root_is_the_layouts_own_tree_and_every_name_keeps_its_number() {
    let (_backing, face, _log) = mounted(FIVE_PAGES);
    assert_eq!(
        names(&face, ROOT),
        vec!["attachments", "images", "meta", "pages", "renders", "text"]
    );
    let root = face.getattr(ROOT).expect("the root stats");
    assert_eq!(root.kind, Kind::Directory);
    assert_eq!(root.ino, ROOT);

    // One inode per path, stable for the life of the mount and never shared between two names —
    // which is the whole of what a POSIX inode promises and the most a tree of *positions* can
    // honestly give (`pdf_fuse`'s own module comment has the argument).
    let first = face.lookup(ROOT, "pages").expect("pages/").ino;
    assert_eq!(face.lookup(ROOT, "pages").expect("again").ino, first);
    let mut seen = std::collections::BTreeSet::new();
    for name in names(&face, ROOT) {
        assert!(
            seen.insert(face.lookup(ROOT, &name).expect("a directory").ino),
            "{name} shares an inode with something else"
        );
    }
    assert!(!seen.contains(&ROOT), "nothing else is the root");
}

/// A page out of the mount: `stat` states its true size, and a read is the core's own bytes.
///
/// RFC 0003 section 5.5's rule is what makes this a test rather than a demonstration — "no
/// virtual file is stat'd before it is generated" — because the kernel clamps a read at the size
/// a `stat` gave it, so a size that is not the file's length truncates every page.
#[test]
fn a_page_states_its_true_size_and_reads_back_in_pieces() {
    let (_backing, face, _log) = mounted(FIVE_PAGES);
    let node = walk(&face, "/pages/0003.pdf").expect("the third page");
    assert_eq!(node.kind, Kind::File);
    let size = node.size.expect("a file states a size");
    assert!(size > 0);

    let handle = face.open(node.ino).expect("opens");
    let whole = face.read(handle, 0, u32::MAX).expect("reads");
    assert_eq!(u64::try_from(whole.len()).expect("fits"), size);
    assert!(whole.starts_with(b"%PDF-"), "a page out of pages/ is a PDF");

    // A short read at the end and an empty one past it, which is what `read(2)` does.
    let head = face.read(handle, 0, 16).expect("reads");
    assert_eq!(head, whole.get(..16).expect("long enough"));
    let tail = face
        .read(handle, size.saturating_sub(4), 64)
        .expect("reads the end");
    assert_eq!(tail.len(), 4);
    assert!(
        face.read(handle, size, 16)
            .expect("past the end")
            .is_empty()
    );
    face.release(handle);

    // A handle this mount did not hand out is `ENOENT` rather than somebody else's file.
    assert_eq!(face.read(handle, 0, 8), Err(Errno::NoSuchFile));
}

/// An open file keeps the generation it was opened under, whatever happens to the document.
///
/// RFC 0003 section 5.4, and the reason [`pdf_vfs::Handle`] holds bytes rather than a path: "No
/// reader ever receives a splice of two generations."
#[test]
fn an_open_file_survives_the_document_being_replaced_underneath_it() {
    let (backing, face, _log) = mounted(FIVE_PAGES);
    let node = walk(&face, "/text/0002.txt").expect("page two's text");
    let handle = face.open(node.ino).expect("opens");
    let before = face.read(handle, 0, u32::MAX).expect("reads");

    backing.replace(std::fs::read(committed(FOURTEEN_PAGES)).expect("a document"));

    assert_eq!(
        face.read(handle, 0, u32::MAX).expect("still reads"),
        before,
        "the bytes are the generation's, and that generation is gone"
    );
    // And the next open sees the new document: fourteen pages where there were five.
    assert_eq!(
        names(&face, walk(&face, "/pages").expect("pages/").ino).len(),
        14
    );
    face.release(handle);
}

/// `cp` into `attachments/`, verb by verb: create, write, flush, release.
///
/// The four the kernel actually sends, in the order it sends them, with §7.5.6's prefix property
/// read off the file after the commit — "changes shall be appended to the end of the file,
/// leaving its original contents intact".
#[test]
fn copying_a_file_into_attachments_is_create_write_flush_release() {
    let (backing, face, log) = mounted(FIVE_PAGES);
    let before = on_disk(&backing);
    let directory = walk(&face, "/attachments").expect("attachments/").ino;
    let payload = b"the bytes a person copied in\n";

    let (node, handle) = face.create(directory, "note.txt").expect("created");
    assert_eq!(node.size, Some(0));
    assert!(node.writable);
    assert_eq!(
        face.write(handle, 0, payload).expect("written"),
        u32::try_from(payload.len()).expect("fits")
    );

    // The staged write is in the tree and not in the document — which is what makes `cp` work,
    // because every copying tool stats what it has just written.
    assert_eq!(
        face.getattr(node.ino).expect("stat").size,
        Some(u64::try_from(payload.len()).expect("fits"))
    );
    assert_eq!(on_disk(&backing), before, "and nothing is in the document");

    face.flush(handle).expect("committed");
    let after = on_disk(&backing);
    assert!(after.len() > before.len());
    assert_eq!(&after[..before.len()], &before[..], "§7.5.6's prefix");
    face.release(handle);

    // Read it back through the tree, the way a second program would.
    let node = walk(&face, "/attachments/note.txt").expect("the embedded file");
    let handle = face.open(node.ino).expect("opens");
    assert_eq!(face.read(handle, 0, u32::MAX).expect("reads"), payload);
    face.release(handle);
    assert!(
        log.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty(),
        "a copy that worked has nothing to say"
    );

    // And out again. RFC 0003 section 5.3 insists §7.5.6's one consequence be said where a
    // person deletes — "[a] deleted page or attachment is unreferenced, not erased" — and a
    // mount's only place to say it is this log.
    face.unlink(directory, "note.txt").expect("removed");
    // Snapshotted before the lookup below, which is *meant* to fail and would log its own
    // `ENOENT` into the same journal.
    let said = log
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .join("\n");
    assert_eq!(walk(&face, "/attachments/note.txt"), Err(Errno::NoSuchFile));
    assert!(said.contains("§7.5.6"), "{said}");
    assert!(said.contains("note.txt"), "{said}");
}

/// A write the kernel never flushes: nothing is committed, and the face says so.
///
/// RFC 0003 section 5.4 is why `release` is where this is noticed and not where it is decided:
/// "`release` reaches nobody, which is why it is only cleanup".
#[test]
fn a_write_that_is_released_without_a_flush_says_nothing_was_committed() {
    let (backing, face, log) = mounted(FIVE_PAGES);
    let before = on_disk(&backing);
    let directory = walk(&face, "/attachments").expect("attachments/").ino;

    let (node, handle) = face.create(directory, "half.txt").expect("created");
    face.write(handle, 0, b"half a file").expect("written");
    face.truncate(handle, 4).expect("truncated");
    assert_eq!(face.getattr(node.ino).expect("stat").size, Some(4));
    face.release(handle);

    assert_eq!(on_disk(&backing), before, "the document is untouched");
    assert_eq!(walk(&face, "/attachments/half.txt"), Err(Errno::NoSuchFile));
    let said = log
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .join("\n");
    assert!(said.contains("nothing was committed"), "{said}");
}

/// A page inserted through the mount, and the renumbering the next listing shows.
#[test]
fn a_document_copied_into_pages_inserts_at_the_names_position() {
    let (backing, face, _log) = mounted(FIVE_PAGES);
    let before = on_disk(&backing);
    let other = std::fs::read(committed(FOURTEEN_PAGES)).expect("a committed document");
    let directory = walk(&face, "/pages").expect("pages/").ino;
    assert_eq!(names(&face, directory).len(), 5);

    let (_node, handle) = face.create(directory, "0004.pdf").expect("created");
    let mut at = 0_u64;
    // In pieces, because that is how the kernel delivers a copy of any size.
    for chunk in other.chunks(4096) {
        let written = face.write(handle, at, chunk).expect("written");
        at = at.saturating_add(u64::from(written));
    }
    face.flush(handle).expect("committed");
    face.release(handle);

    assert_eq!(names(&face, directory).len(), 19);
    let after = on_disk(&backing);
    assert_eq!(&after[..before.len()], &before[..], "§7.5.6's prefix");

    // And a page deleted, which is `rm`.
    face.unlink(directory, "0001.pdf").expect("deleted");
    assert_eq!(names(&face, directory).len(), 18);
}

/// Every refusal RFC 0003 section 5.3 argues for, as the `errno` the kernel is handed — and the
/// sentence beside it, which is the half FUSE has no room for.
#[test]
fn the_refusals_reach_the_kernel_as_numbers_and_the_log_as_sentences() {
    let (_backing, face, log) = mounted(FIVE_PAGES);
    let cases = [
        // Derived artefacts: a read-only view of something else.
        ("/renders/150dpi", "0001.png", Errno::ReadOnly),
        ("/meta", "xmp.xml", Errno::ReadOnly),
        ("/meta", "outline.json", Errno::ReadOnly),
        // Semantic refusals: this program will not do it.
        ("/text", "0001.txt", Errno::OperationNotPermitted),
        ("/images/0001", "01.png", Errno::OperationNotPermitted),
    ];
    for (directory, name, expected) in cases {
        let parent = walk(&face, directory).expect(directory).ino;
        let outcome = face.create(parent, name);
        assert!(
            matches!(outcome, Err(error) if error == expected),
            "{directory}/{name}: {:?}",
            outcome.map(|(node, _)| node.path)
        );
    }

    // A rename is RFC 0003 section 5.3's first refusal, whatever it names.
    let pages = walk(&face, "/pages").expect("pages/").ino;
    assert_eq!(
        face.rename(pages, "0001.pdf", pages, "0002.pdf"),
        Errno::OperationNotPermitted
    );
    // And a directory of the caller's own is not a thing this tree has.
    assert_eq!(face.mkdir(ROOT, "mine"), Errno::OperationNotPermitted);

    // Every one of them said why, because FUSE hands the caller a number and nothing else.
    let said = log
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    assert!(said.len() >= 7, "{said:?}");
    for sentence in &said {
        assert!(
            sentence.contains('E'),
            "a refusal's log line names its errno: {sentence}"
        );
    }
}

/// Names the kernel can send that this tree cannot hold.
#[test]
fn a_name_with_a_solidus_and_an_inode_nobody_handed_out() {
    let (_backing, face, _log) = mounted(FIVE_PAGES);
    assert_eq!(face.lookup(ROOT, "pages/0001.pdf"), Err(Errno::Invalid));
    assert_eq!(face.lookup(ROOT, ""), Err(Errno::Invalid));
    assert_eq!(face.getattr(9_999_999), Err(Errno::NoSuchFile));
    assert_eq!(face.readdir(9_999_999), Err(Errno::NoSuchFile));
    assert_eq!(face.lookup(ROOT, "fonts"), Err(Errno::NoSuchFile));
}

/// The invalidation thread's question, answered by the generation key and nothing else.
///
/// RFC 0003 section 5.4 puts the notifications in a separate task; what that task needs from the
/// face is *whether to send any*, and this is that.
#[test]
fn the_notifier_is_told_exactly_when_the_document_changed() {
    let (backing, face, _log) = mounted(FIVE_PAGES);
    let first = face.changed_since(None).expect("a first key").key;
    assert!(
        face.changed_since(Some(first)).is_none(),
        "an unchanged document is nothing to invalidate"
    );

    // The names the thread would invalidate are the ones this mount has handed out, each under
    // the directory it is in.
    walk(&face, "/pages/0002.pdf").expect("a page");
    let known = face.known();
    assert!(
        known
            .iter()
            .any(|(_, _, name)| name == "0002.pdf" || name == "pages"),
        "{known:?}"
    );
    assert!(
        known.iter().all(|(parent, ino, _)| *parent != *ino),
        "nothing is its own parent"
    );

    backing.replace(std::fs::read(committed(FOURTEEN_PAGES)).expect("a document"));
    let second = face
        .changed_since(Some(first))
        .expect("the document changed")
        .key;
    assert_ne!(first, second);
    assert!(face.changed_since(Some(second)).is_none());
}
