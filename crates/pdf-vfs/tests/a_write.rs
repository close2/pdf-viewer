//! The write side: RFC 0003 section 5.2's five verbs, and the transaction around them.
//!
//! The claims this file exists to hold, each of them a sentence the round would otherwise only
//! have written down:
//!
//! - **An ordinal is a position, not an identity.** Insert before the fourth page and the
//!   incumbent is the fifth; delete a page and everything after it moves up. The listing after a
//!   write is a listing of the document as it now is.
//! - **The commit is atomic and §7.5.6's property is *checked*.** The file after a write begins
//!   with the file before it, byte for byte, because "changes shall be appended to the end of the
//!   file, leaving its original contents intact".
//! - **A write that is not flushed did not happen.** The staged bytes are in the tree and not in
//!   the document, and dropping the handle leaves the file exactly as it was.
//! - **Our own commit is our own.** The generation key changes with the file, and the generation
//!   built for it says so: `Provenance::Ours` for a write of ours, `Provenance::Foreign` for
//!   anybody else's.
//! - **The four levels reach a file system**, and each one has an `errno`.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::sync::Arc;

use pdf_vfs::worker::InProcessWorkers;
use pdf_vfs::{Config, Errno, MemoryBacking, Provenance, Vfs, VfsError};

/// A committed document, which every checkout has once `doc/specifications.zip` is unpacked.
fn committed(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

/// A corpus document's path, or `None` when the submodule is not checked out.
fn corpus(name: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    path.exists().then_some(path)
}

/// The five-page annex, which has an outline and §12.4.2 labels.
const FIVE_PAGES: &str = "PDF20_AN001-BPC.pdf";
/// The fourteen-page annex about associated files — a different document of a different length.
const FOURTEEN_PAGES: &str = "PDF20_AN002-AF.pdf";

/// A backing the test and the tree both hold, so that "the file changed" is a thing the test can
/// *do* to the same object the tree is reading.
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

/// A tree over a document held in memory.
fn mounted(name: &str) -> (Arc<MemoryBacking>, Vfs) {
    mounted_bytes(
        name,
        std::fs::read(committed(name)).expect("a committed document"),
    )
}

/// A tree over bytes the caller chose, under a name.
fn mounted_bytes(name: &str, bytes: Vec<u8>) -> (Arc<MemoryBacking>, Vfs) {
    let backing = Arc::new(MemoryBacking::new(name, bytes));
    let vfs = Vfs::new(
        Box::new(SharedBacking(Arc::clone(&backing))),
        Box::new(InProcessWorkers),
        Config::default(),
    );
    (backing, vfs)
}

/// Every name in a listing, sorted.
fn names(vfs: &Vfs, path: &str) -> Vec<String> {
    let mut out: Vec<String> = vfs
        .list(path)
        .unwrap_or_else(|error| panic!("{path}: {error}"))
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    out.sort();
    out
}

/// One file's bytes out of the tree.
fn read(vfs: &Vfs, path: &str) -> Vec<u8> {
    vfs.open(path)
        .unwrap_or_else(|error| panic!("{path}: {error}"))
        .bytes()
        .to_vec()
}

/// The whole document, as it is on disk right now.
fn on_disk(backing: &MemoryBacking) -> Vec<u8> {
    use pdf_vfs::generation::Backing as _;
    let bytes = backing.bytes().expect("the backing reads");
    bytes.read(0..bytes.len()).into_owned()
}

/// Insert before the fourth page: the incumbent becomes the fifth, and the names renumber.
///
/// RFC 0003 section 5.2 states both halves — "`cp new.pdf pages/0004.pdf` inserts before the
/// current fourth page (the incumbent 0004 and everything after shift up on the next listing)"
/// and "[o]rdinal names are **positions, not identities**". The text of a page is what identifies
/// it here, because a page taken out of the mount after the write is extracted from a document
/// whose object numbers have moved.
#[test]
fn an_insertion_puts_the_pages_at_the_name_and_the_incumbent_moves_down() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let other = std::fs::read(committed(FOURTEEN_PAGES)).expect("a committed document");
    let before = on_disk(&backing);
    let incumbent = read(&vfs, "/text/0004.txt");
    let fifth = read(&vfs, "/text/0005.txt");
    assert_eq!(names(&vfs, "/pages").len(), 5);

    let committed_write = vfs.write("/pages/0004.pdf", &other).expect("inserted");
    assert_eq!(committed_write.pages, 19);
    assert_ne!(committed_write.from, committed_write.to);

    // §7.5.6, checked rather than believed.
    let after = on_disk(&backing);
    assert!(after.len() > before.len());
    assert_eq!(&after[..before.len()], &before[..]);

    // The listing renumbered, and the incumbent is where the rule says.
    assert_eq!(names(&vfs, "/pages").len(), 19);
    assert_eq!(
        names(&vfs, "/text").len(),
        20,
        "and document.txt beside them"
    );
    assert_eq!(read(&vfs, "/text/0018.txt"), incumbent);
    assert_eq!(read(&vfs, "/text/0019.txt"), fifth);
    assert_eq!(
        read(&vfs, "/pages/0004.pdf"),
        read(&vfs, "/pages/0004.pdf"),
        "and the same page twice is the same bytes"
    );
}

/// A page deleted is a page the tree no longer names, and everything after it moves up.
#[test]
fn a_deletion_renumbers_and_says_what_stays_in_the_file() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let before = on_disk(&backing);
    let third = read(&vfs, "/text/0003.txt");

    let committed_write = vfs.remove("/pages/0002.pdf").expect("deleted");
    assert_eq!(committed_write.pages, 4);
    assert_eq!(names(&vfs, "/pages").len(), 4);
    assert_eq!(read(&vfs, "/text/0002.txt"), third);
    assert!(
        matches!(vfs.stat("/pages/0005.pdf"), Err(VfsError::NoSuchPath(_))),
        "the fifth name is gone with the fifth position"
    );

    // RFC 0003 section 5.3 insists this be said where a person deletes.
    assert!(
        committed_write
            .warnings
            .iter()
            .any(|detail| detail.contains("§7.5.6")),
        "{:?}",
        committed_write.warnings
    );
    let after = on_disk(&backing);
    assert_eq!(&after[..before.len()], &before[..]);
}

/// A file embedded, listed, read back byte for byte, and removed again.
#[test]
fn a_file_embedded_is_listed_read_back_and_removed() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let payload = b"the bytes a person copied in\n".to_vec();
    assert!(!names(&vfs, "/attachments").contains(&"note.txt".to_owned()));

    vfs.write("/attachments/note.txt", &payload)
        .expect("embedded");
    assert!(names(&vfs, "/attachments").contains(&"note.txt".to_owned()));
    assert_eq!(read(&vfs, "/attachments/note.txt"), payload);

    // §7.7.4's tree files one name once, and this verb writes one update.
    match vfs.write("/attachments/note.txt", b"again") {
        Err(error @ VfsError::AlreadyFiled { .. }) => assert_eq!(error.errno(), Errno::Exists),
        other => panic!("a second file under one name: {other:?}"),
    }

    vfs.remove("/attachments/note.txt").expect("removed");
    assert!(!names(&vfs, "/attachments").contains(&"note.txt".to_owned()));
    assert!(matches!(
        vfs.stat("/attachments/note.txt"),
        Err(VfsError::NoSuchPath(_))
    ));
}

/// `meta/info.json` read, written straight back, and read again: the same file.
///
/// The property that makes the read and the write one thing rather than two, and the reason the
/// round answered RFC 0003 section 9's fourth open question with *yes*.
#[test]
fn writing_info_json_back_unchanged_changes_nothing_it_states() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let before = read(&vfs, "/meta/info.json");
    vfs.write("/meta/info.json", &before).expect("set");
    assert_eq!(read(&vfs, "/meta/info.json"), before);

    // And a change is a change: the file is the whole of Table 349, so a key it omits is an
    // entry the document no longer states, and one it names is what the document says now.
    vfs.write(
        "/meta/info.json",
        br#"{"title": "a title this round chose", "trapped": "False"}"#,
    )
    .expect("set");
    let after = String::from_utf8(read(&vfs, "/meta/info.json")).expect("UTF-8");
    assert!(
        after.contains("\"title\": \"a title this round chose\""),
        "{after}"
    );
    assert!(after.contains("\"trapped\": \"False\""), "{after}");
    assert!(after.contains("\"producer\": null"), "{after}");
}

/// A file this file is not: refused by name rather than coerced into an entry.
#[test]
fn a_meta_info_file_that_is_not_one_is_refused() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    for bad in [
        &b"[]"[..],
        &b"{\"title\": 3}"[..],
        &b"{\"publisher\": \"nobody\"}"[..],
        &b"{\"created\": \"2026-09-03\"}"[..],
        &b"{"[..],
    ] {
        match vfs.write("/meta/info.json", bad) {
            Err(error) => assert_eq!(
                error.errno(),
                Errno::InputOutput,
                "{}",
                String::from_utf8_lossy(bad)
            ),
            Ok(_) => panic!("{} was written", String::from_utf8_lossy(bad)),
        }
    }
}

/// A write in flight is in the tree and not in the document, and abandoning it changes nothing.
///
/// The four answers `pdf_vfs::commit` states, as one test: what a partial write looks like, what
/// a second reader sees, what happens when `flush` never comes, and what the document holds while
/// all of that is going on.
#[test]
fn a_write_that_is_never_flushed_is_in_the_tree_and_not_in_the_document() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let before = on_disk(&backing);

    let id = vfs.create("/attachments/half.txt").expect("created");
    vfs.write_at(id, 0, b"half a file").expect("written");

    // A second reader — this test — sees it, because `cp` stats what it has just written.
    assert!(names(&vfs, "/attachments").contains(&"half.txt".to_owned()));
    assert_eq!(
        vfs.stat("/attachments/half.txt").expect("stat").size,
        Some(11)
    );
    assert_eq!(read(&vfs, "/attachments/half.txt"), b"half a file");
    // And the document does not.
    assert_eq!(on_disk(&backing), before);
    assert_eq!(vfs.pending().len(), 1);

    // `ftruncate(2)` on a staged file, which is what a face passes a `setattr` with a size to —
    // and the only truncation this tree implements, because `O_TRUNC` on a create is what
    // starting the staging buffer empty already is.
    vfs.truncate(id, 4).expect("truncated");
    assert_eq!(
        vfs.stat("/attachments/half.txt").expect("stat").size,
        Some(4)
    );
    assert_eq!(read(&vfs, "/attachments/half.txt"), b"half");
    vfs.truncate(id, 6).expect("grown");
    assert_eq!(
        read(&vfs, "/attachments/half.txt"),
        b"half\0\0",
        "growing a file fills with zero bytes, as a sparse write to a real one does"
    );
    assert_eq!(on_disk(&backing), before, "and none of it reached the file");

    let abandoned = vfs.release(id).expect("never flushed");
    assert_eq!(abandoned.size, 6);
    assert!(abandoned.sentence().contains("nothing was committed"));
    assert_eq!(on_disk(&backing), before, "the document is untouched");
    assert!(!names(&vfs, "/attachments").contains(&"half.txt".to_owned()));
    assert!(vfs.pending().is_empty());
}

/// A second `flush` does nothing, because the kernel issues one per `close(2)`.
#[test]
fn flushing_twice_writes_once() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let id = vfs.create("/attachments/once.txt").expect("created");
    vfs.write_at(id, 0, b"once").expect("written");
    let first = vfs.flush(id).expect("committed");
    let after = on_disk(&backing);
    let second = vfs.flush(id).expect("a second flush is nothing");
    assert_eq!(second.from, second.to, "no generation was stepped");
    assert_eq!(on_disk(&backing), after);
    assert_ne!(first.from, first.to);
    assert!(vfs.release(id).is_none(), "it was committed");
}

/// The document changed under a staged write: refused, `ESTALE`, and nothing written.
///
/// RFC 0003 section 5.4's key applied to a write. The update the worker computes is a function of
/// the document it was computed from, so committing it over somebody else's edit would throw that
/// edit away.
#[test]
fn a_write_staged_against_a_generation_that_is_gone_is_refused() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let id = vfs.create("/attachments/racing.txt").expect("created");
    vfs.write_at(id, 0, b"mine").expect("written");

    // Somebody else replaces the file.
    backing.replace(std::fs::read(committed(FOURTEEN_PAGES)).expect("a document"));
    let theirs = on_disk(&backing);

    match vfs.flush(id) {
        Err(error @ VfsError::Changed { .. }) => assert_eq!(error.errno(), Errno::Stale),
        other => panic!("a write over somebody else's edit: {other:?}"),
    }
    assert_eq!(on_disk(&backing), theirs, "their edit is still there");
}

/// Our own commit is ours, and somebody else's is theirs.
#[test]
fn the_generation_after_our_own_commit_says_it_is_ours() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    assert_eq!(vfs.provenance().expect("opened"), Provenance::Opened);

    vfs.write("/attachments/ours.txt", b"ours")
        .expect("written");
    assert_eq!(
        vfs.provenance().expect("after our commit"),
        Provenance::Ours,
        "a write of ours must not look to the tree like somebody editing the file underneath it"
    );

    backing.replace(std::fs::read(committed(FOURTEEN_PAGES)).expect("a document"));
    assert_eq!(vfs.provenance().expect("after theirs"), Provenance::Foreign);
}

/// A staged write past the ceiling is refused at the write that crosses it.
#[test]
fn a_write_in_flight_has_a_ceiling() {
    let bytes = std::fs::read(committed(FIVE_PAGES)).expect("a committed document");
    let backing = Arc::new(MemoryBacking::new(FIVE_PAGES, bytes));
    let vfs = Vfs::new(
        Box::new(SharedBacking(Arc::clone(&backing))),
        Box::new(InProcessWorkers),
        Config {
            max_staged_bytes: 16,
            ..Config::default()
        },
    );
    let id = vfs.create("/attachments/big.txt").expect("created");
    vfs.write_at(id, 0, &[0; 16]).expect("up to the ceiling");
    match vfs.write_at(id, 16, b"one more") {
        Err(error @ VfsError::TooLarge { .. }) => assert_eq!(error.errno(), Errno::TooBig),
        other => panic!("past the ceiling: {other:?}"),
    }
    vfs.release(id);
}

/// Positions the tree does not have, and names it cannot file.
#[test]
fn a_position_past_one_past_the_end_and_a_name_a_directory_cannot_hold() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let other = std::fs::read(committed(FOURTEEN_PAGES)).expect("a committed document");

    // One past the end appends, which is what RFC 0003 section 5.2's positions mean.
    let id = vfs.create("/pages/0006.pdf").expect("one past the end");
    vfs.release(id);
    for path in ["/pages/0007.pdf", "/pages/0000.pdf"] {
        match vfs.write(path, &other) {
            Err(error @ VfsError::NoSuchPath(_)) => assert_eq!(error.errno(), Errno::NoSuchFile),
            other => panic!("{path}: {other:?}"),
        }
    }
    // §7.7.4's tree admits a name a directory cannot show, so the write is refused rather than
    // quietly renamed — the listing and the read have to agree.
    match vfs.write("/attachments/a:name", b"x") {
        Err(error @ VfsError::Unnameable { .. }) => assert_eq!(error.errno(), Errno::Invalid),
        other => panic!("a colon in a name: {other:?}"),
    }
}

/// Every refusal RFC 0003 section 5.3 argues for, as the `errno` a file manager will show.
#[test]
fn the_refusals_by_design_each_have_their_own_errno() {
    let (_backing, vfs) = mounted(FIVE_PAGES);
    let cases = [
        // RFC 0003 section 5.3's derived artefacts: a read-only view of something else.
        ("/renders/150dpi/0001.png", Errno::ReadOnly),
        ("/meta/xmp.xml", Errno::ReadOnly),
        ("/meta/outline.json", Errno::ReadOnly),
        // RFC 0003 section 5.3's semantic refusals: this program will not do it.
        ("/text/0001.txt", Errno::OperationNotPermitted),
        ("/text/document.txt", Errno::OperationNotPermitted),
        ("/images/0001/01.png", Errno::OperationNotPermitted),
        ("/pages", Errno::OperationNotPermitted),
    ];
    for (path, errno) in cases {
        let error = vfs.write(path, b"x").expect_err(path);
        assert_eq!(error.errno(), errno, "{path}: {error}");
    }
    // A rename is RFC 0003 section 5.3's first refusal and is refused whatever it names.
    let error = vfs
        .rename("/pages/0001.pdf", "/pages/0002.pdf")
        .expect_err("a reorder");
    assert_eq!(error.errno(), Errno::OperationNotPermitted);
}

/// `CLAUDE.md` principle 3's four levels, on a file system that has nobody to ask.
///
/// `bug1815476.pdf` is encrypted with `/P -1084` — §7.6.4.2's Table 22 bit 4 clear — so it
/// withholds `Operation::Modify`, which is the bit an embedded file falls under (ADR 0802). The
/// levels are the host's, supplied through `Config::policy` and asked exactly once at the
/// transform seam.
#[test]
fn the_four_levels_reach_a_mount_and_two_of_them_refuse() {
    use pdf_model::restriction::Level;

    let Some(path) = corpus("bug1815476.pdf") else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let bytes = std::fs::read(path).expect("a corpus document");

    let under = |level: Level| {
        let backing = Arc::new(MemoryBacking::new("restricted", bytes.clone()));
        let vfs = Vfs::new(
            Box::new(SharedBacking(Arc::clone(&backing))),
            Box::new(InProcessWorkers),
            Config {
                policy: pdf_transform::Policy {
                    restrictions: level,
                },
                ..Config::default()
            },
        );
        let outcome = vfs.write("/attachments/level.txt", b"x");
        (backing, outcome)
    };

    // `off` — the program is the reader's, and this is the level `CLAUDE.md` says shall always
    // be possible. It is `Config`'s default.
    let (backing, outcome) = under(Level::Off);
    let unchanged = bytes.len();
    outcome.expect("off proceeds");
    assert!(on_disk(&backing).len() > unchanged);

    // `on` — §7.6.4.1's own `shall`, kept: refused, `EACCES`, and the sentence names the bit.
    let (backing, outcome) = under(Level::On);
    let error = outcome.expect_err("on refuses");
    assert_eq!(error.errno(), Errno::PermissionDenied);
    assert!(error.to_string().contains("Table 22 bit 4"), "{error}");
    assert_eq!(on_disk(&backing).len(), unchanged);

    // `ask` — a file system has no dialogue, so the question is answered as a refusal rather
    // than as a silent proceed. Its own variant, so a face can log a different sentence.
    let (backing, outcome) = under(Level::Ask);
    let error = outcome.expect_err("ask has nobody to ask");
    assert_eq!(error.errno(), Errno::PermissionDenied);
    assert!(
        matches!(
            error,
            VfsError::Worker(pdf_vfs::worker::WorkerError::Unanswerable(_))
        ),
        "{error}"
    );
    assert_eq!(on_disk(&backing).len(), unchanged);

    // `warn` — proceeds, and says so afterwards.
    let (backing, outcome) = under(Level::Warn);
    let committed = outcome.expect("warn proceeds");
    assert!(
        committed
            .warnings
            .iter()
            .any(|detail| detail.contains("Table 22 bit 4")),
        "{:?}",
        committed.warnings
    );
    assert!(on_disk(&backing).len() > unchanged);
}

/// The two round trips of ADR 0874: the question crosses, a person answers, the operation runs.
///
/// **The whole point of the round.** `bug1815476.pdf` is encrypted with `/P -1084`, so §7.6.4.2's
/// Table 22 bit 4 is clear (`Operation::Modify`, which an embedded file falls under, ADR 0802)
/// and so is bit 11 (`Operation::Assemble`, "[a]ssemble the document (insert, rotate, or delete
/// pages …)"). At `Level::Ask` the operation was a refusal in every face before this, because the
/// decision is taken inside a process RFC 0003 section 6 gives no channel to a person. Now the
/// broker asks first, the answer comes back through `Vfs::answer`, and the verb is issued
/// unchanged.
///
/// Four properties, and each is one a wrong construction would lose:
///
/// - the question names the operation and the bit, so a face has something to show;
/// - **a `no` leaves the document byte for byte what it was**, and refuses by name rather than
///   silently doing nothing;
/// - a `yes` performs it;
/// - the consent is spent **once** — a yes to deleting one page is not a yes to the next.
#[test]
fn a_question_crosses_the_confinement_and_both_answers_are_obeyed() {
    use pdf_model::restriction::Level;
    use pdf_vfs::{Consulted, Verb};

    let Some(path) = corpus("bug1815476.pdf") else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let bytes = std::fs::read(path).expect("a corpus document");
    let asking = || {
        let backing = Arc::new(MemoryBacking::new("restricted", bytes.clone()));
        let vfs = Vfs::new(
            Box::new(SharedBacking(Arc::clone(&backing))),
            Box::new(InProcessWorkers),
            Config {
                policy: pdf_transform::Policy {
                    restrictions: Level::Ask,
                },
                ..Config::default()
            },
        );
        (backing, vfs)
    };

    // The question, about a write: Table 22 bit 4, worded once by `pdf_transform::consult`.
    let (backing, vfs) = asking();
    let unchanged = bytes.len();
    let consulted = vfs
        .consult("/attachments/level.txt", Verb::Write)
        .expect("the question crosses");
    let Consulted::Ask { operation, reasons } = &consulted else {
        panic!("a document that withholds it asks: {consulted:?}");
    };
    assert_eq!(*operation, "modifying the document");
    assert_eq!(reasons, "Table 22 bit 4 is clear");
    let question = consulted.question().expect("an ask is a question");
    assert!(question.ends_with("Do it anyway?"), "{question}");

    // A `no`. The edit is not done, the refusal says a question went unanswered, and the file on
    // disk is what it was.
    assert!(vfs.answer(false).expect("a question was outstanding"));
    let error = vfs
        .write("/attachments/level.txt", b"x")
        .expect_err("a no is not done");
    assert!(
        matches!(
            error,
            VfsError::Worker(pdf_vfs::worker::WorkerError::Unanswerable(_))
        ),
        "{error}"
    );
    assert_eq!(on_disk(&backing), bytes, "answering no changed the file");

    // A `yes`, on a fresh mount so that the two answers cannot interfere.
    let (backing, vfs) = asking();
    assert!(matches!(
        vfs.consult("/attachments/level.txt", Verb::Write)
            .expect("asked"),
        Consulted::Ask { .. }
    ));
    assert!(vfs.answer(true).expect("a question was outstanding"));
    vfs.write("/attachments/level.txt", b"x")
        .expect("a yes is done");
    assert!(on_disk(&backing).len() > unchanged);

    // And spent once. The same mount, no second question: the next write is refused again.
    let error = vfs
        .write("/attachments/second.txt", b"x")
        .expect_err("one yes is one operation");
    assert!(
        matches!(
            error,
            VfsError::Worker(pdf_vfs::worker::WorkerError::Unanswerable(_))
        ),
        "{error}"
    );

    // A deletion is a different operation — Table 22 bit 11 — and is asked about separately.
    let (_, vfs) = asking();
    let consulted = vfs
        .consult("/pages/0001.pdf", Verb::Delete)
        .expect("the question crosses");
    let Consulted::Ask { operation, reasons } = &consulted else {
        panic!("bit 11 is clear in this document too: {consulted:?}");
    };
    assert_eq!(*operation, "assembling a document out of these pages");
    assert_eq!(reasons, "Table 22 bit 11 is clear");
    assert!(vfs.answer(true).expect("outstanding"));
    // A yes releases the policy and nothing else: this document has one page, so §7.7.3.2's own
    // shape refuses the deletion — and *that* refusal, rather than the policy's, is the proof
    // that the consent was spent and the operation actually ran.
    let error = vfs.remove("/pages/0001.pdf").expect_err("its last page");
    assert!(
        !matches!(
            error,
            VfsError::Worker(
                pdf_vfs::worker::WorkerError::Unanswerable(_)
                    | pdf_vfs::worker::WorkerError::Restricted(_)
            )
        ),
        "the yes was not spent: {error}"
    );
    assert!(error.to_string().contains("one page"), "{error}");

    // Nothing outstanding is a `false` rather than a silent yes: a face that answers a question
    // nobody asked has a defect, and it is told so.
    assert!(!vfs.answer(true).expect("no question"));
}

/// A mount at `off` costs a consultation nothing, and one at `on` is told so rather than asked.
///
/// The two verdicts a face must not put in front of a person: `Proceed` because there is nothing
/// to decide, `Refuse` because the level has already decided it. A face may therefore consult
/// before every verb and pay one round trip and no dialogue.
#[test]
fn a_consultation_answers_the_other_three_levels_without_a_question() {
    use pdf_model::restriction::Level;
    use pdf_vfs::{Consulted, Verb};

    let Some(path) = corpus("bug1815476.pdf") else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let bytes = std::fs::read(path).expect("a corpus document");
    let under = |level: Level| {
        let vfs = Vfs::new(
            Box::new(SharedBacking(Arc::new(MemoryBacking::new(
                "restricted",
                bytes.clone(),
            )))),
            Box::new(InProcessWorkers),
            Config {
                policy: pdf_transform::Policy {
                    restrictions: level,
                },
                ..Config::default()
            },
        );
        let consulted = vfs
            .consult("/attachments/level.txt", Verb::Write)
            .expect("the question crosses");
        // Whatever the verdict, nothing is outstanding unless it was an ask.
        let outstanding = vfs.answer(true).expect("asked");
        (consulted, outstanding)
    };

    assert_eq!(under(Level::Off).0, Consulted::Proceed);
    assert!(!under(Level::Off).1, "nothing to answer at off");
    assert!(matches!(under(Level::On).0, Consulted::Refuse { .. }));
    assert!(!under(Level::On).1, "on has already decided");
    assert!(matches!(under(Level::Warn).0, Consulted::Warn { .. }));
    assert!(!under(Level::Warn).1, "warn is a statement, not a question");
    assert!(
        under(Level::Ask).0.question().is_some(),
        "only an ask is a question"
    );
}

/// The layout table's operations are `pdf_transform::Plan::operation`'s own, not a second reading.
///
/// **Two mappings that must agree and are only *said* to agree is how they stop agreeing.** The
/// broker names an operation from a path and a verb (`layout::Write::operation`,
/// `layout::Generator::operation`); the seam names one from the plan it is about to run
/// (`Plan::operation`). A consent given against the first and spent against the second would be
/// spent on the wrong operation the day they diverge. So the *witness* is the tree itself: at
/// `Level::On` a consultation refuses exactly where the operation refuses, path by path.
#[test]
fn what_the_layout_says_a_path_performs_is_what_the_seam_asks_about() {
    use pdf_model::restriction::Level;
    use pdf_vfs::{Consulted, Verb};

    let Some(path) = corpus("bug1815476.pdf") else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let bytes = std::fs::read(path).expect("a corpus document");
    let vfs = || {
        Vfs::new(
            Box::new(SharedBacking(Arc::new(MemoryBacking::new(
                "restricted",
                bytes.clone(),
            )))),
            Box::new(InProcessWorkers),
            Config {
                policy: pdf_transform::Policy {
                    restrictions: Level::On,
                },
                ..Config::default()
            },
        )
    };

    // `/P -1084` clears bits 4, 5 and 11 and leaves bit 3 set, so a render comes out and a page,
    // an image and a write do not. Each pair is (the consultation, the operation itself).
    let reading = [
        ("/pages/0001.pdf", true),
        ("/renders/150dpi/0001.png", false),
        ("/images/0001", true),
    ];
    for (path, restricted) in reading {
        let tree = vfs();
        let consulted = tree
            .consult(path, Verb::Read)
            .expect("the question crosses");
        assert_eq!(
            matches!(consulted, Consulted::Refuse { .. }),
            restricted,
            "{path}: the consultation says {consulted:?}"
        );
        let performed = if path == "/images/0001" {
            tree.list(path).map(|_| ())
        } else {
            tree.open(path).map(|_| ())
        };
        assert_eq!(
            performed.is_err(),
            restricted,
            "{path}: the operation itself disagreed with the consultation ({performed:?})"
        );
    }

    let tree = vfs();
    assert!(matches!(
        tree.consult("/pages/0001.pdf", Verb::Delete)
            .expect("asked"),
        Consulted::Refuse { .. }
    ));
    assert!(tree.remove("/pages/0001.pdf").is_err());
    let tree = vfs();
    assert!(matches!(
        tree.consult("/attachments/x.txt", Verb::Write)
            .expect("asked"),
        Consulted::Refuse { .. }
    ));
    assert!(tree.write("/attachments/x.txt", b"x").is_err());
}

/// A copy cut short is a document this reader recovers, and it says so rather than refusing.
///
/// **The round expected a refusal here and the tree gave a recovery**, which is the more honest
/// answer and is worth the test that found it: `pdf_syntax` rebuilds a cross-reference table by
/// scanning, so two thirds of a fourteen-page document opens with the eight pages the scan
/// found. Refusing would mean declining a *damaged* file somebody meant to insert, because a
/// truncated copy and a damaged document are the same bytes — nothing anywhere knows how long
/// the copy was meant to be. So the insertion proceeds and the recovery is named (trap 5).
#[test]
fn a_copy_cut_short_is_recovered_and_says_so() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let before = on_disk(&backing);
    let other = std::fs::read(committed(FOURTEEN_PAGES)).expect("a committed document");
    let torn = &other[..other.len() / 3];

    let committed_write = vfs.write("/pages/0004.pdf", torn).expect("a torn copy");
    assert!(
        committed_write
            .warnings
            .iter()
            .any(|detail| detail.contains("rebuilt \nby scanning")
                || detail.contains("rebuilt by scanning")),
        "{:?}",
        committed_write.warnings
    );
    assert!(committed_write.pages > 5);
    // And §7.5.6's property holds all the same: the producer's bytes are still there.
    let after = on_disk(&backing);
    assert_eq!(&after[..before.len()], &before[..]);
}

/// A file that is not a PDF at all cannot be inserted, and the document is untouched.
#[test]
fn a_file_that_is_not_a_document_is_not_inserted() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let before = on_disk(&backing);
    let error = vfs
        .write("/pages/0004.pdf", b"this is not a PDF at all\n")
        .expect_err("not a document");
    assert_eq!(error.errno(), Errno::InputOutput);
    assert_eq!(on_disk(&backing), before);
    assert_eq!(names(&vfs, "/pages").len(), 5);
}

/// Two writes in flight at once in one mount: the second is **not** somebody else's edit.
///
/// The generation key moves under a staged write for two different reasons, and refusing both as
/// `ESTALE` cost the second write of every pair. RFC 0003 section 5.4's rule is about
/// *somebody else's* update — committing over it "would discard whatever changed it" — and our
/// own commit discards nothing, because §7.5.6 appends. A mount by hand lost the second of two
/// files copied into `attachments/` with both descriptors open, and lost it quietly, because
/// `close(2)`'s error is a thing most programs do not look at (round 911).
///
/// What still decides is whether the **name** means what it meant. An embedded file's name and
/// the information dictionary are identities. A page's ordinal is a position — RFC 0003 section
/// 5.2 — so an insertion staged before a commit that renumbered stays `ESTALE`, and the last
/// third of this test is that half of the rule.
#[test]
fn two_writes_in_flight_in_one_mount_both_land() {
    let (backing, vfs) = mounted(FIVE_PAGES);
    let before = on_disk(&backing);

    let first = vfs.create("/attachments/one.txt").expect("created");
    let second = vfs.create("/attachments/two.txt").expect("created");
    vfs.write_at(first, 0, b"one").expect("written");
    vfs.write_at(second, 0, b"two").expect("written");
    vfs.flush(first).expect("the first commits");
    vfs.flush(second).expect("and so does the second");
    vfs.release(first);
    vfs.release(second);

    assert_eq!(names(&vfs, "/attachments"), vec!["one.txt", "two.txt"]);
    assert_eq!(read(&vfs, "/attachments/two.txt"), b"two");
    let after = on_disk(&backing);
    assert_eq!(
        &after[..before.len()],
        &before[..],
        "§7.5.6's prefix, twice"
    );

    // A page's ordinal is a position, so an insertion staged across a commit that renumbered is
    // still `ESTALE` — it would land somewhere nobody asked for.
    let one_page = std::fs::read(committed(FOURTEEN_PAGES)).expect("a document");
    let staged = vfs.create("/pages/0002.pdf").expect("created");
    vfs.write_at(staged, 0, &one_page).expect("written");
    vfs.write("/attachments/three.txt", b"three")
        .expect("a commit of ours in between");
    match vfs.flush(staged) {
        Err(error @ VfsError::Changed { .. }) => assert_eq!(error.errno(), Errno::Stale),
        other => panic!("an ordinal staged across a renumbering: {other:?}"),
    }
    assert_eq!(names(&vfs, "/pages").len(), 5, "and nothing was inserted");
}

/// A size the cache has produced outlives the bytes, so a second listing is free.
///
/// RFC 0003 section 5.5 makes a `stat` generate because "an under-estimate silently truncates a
/// page" — and a length taken off the bytes themselves is not an estimate. With a budget too
/// small to hold a directory, every entry is evicted before the listing comes round again, and
/// `ls -l` used to cost the whole extraction *every time*: on ISO 32000-2's 1023 pages, 2 min 45 s
/// the first time and 4 min 03 s the second (round 911).
#[test]
fn a_stat_after_an_eviction_does_not_generate_again() {
    let bytes = std::fs::read(committed(FIVE_PAGES)).expect("a committed document");
    let backing = Arc::new(MemoryBacking::new(FIVE_PAGES, bytes));
    let vfs = Vfs::new(
        Box::new(SharedBacking(Arc::clone(&backing))),
        Box::new(InProcessWorkers),
        Config {
            // One page out of this document is tens of kilobytes, so nothing survives a second
            // entry and the cache is doing what a real one does on a document too big for it.
            cache_bytes: 1024,
            ..Config::default()
        },
    );
    let mut sizes = Vec::new();
    for page in 1..=5 {
        let path = format!("/pages/{page:04}.pdf");
        sizes.push(vfs.stat(&path).expect("a page stats").size);
    }
    let after_the_first_listing = vfs.generated();
    assert_eq!(after_the_first_listing, 5, "one generation per page, once");

    // The second listing, which is what a file manager does the moment somebody scrolls back.
    for (page, size) in (1..=5).zip(&sizes) {
        assert_eq!(
            vfs.stat(&format!("/pages/{page:04}.pdf"))
                .expect("again")
                .size,
            *size
        );
    }
    assert_eq!(
        vfs.generated(),
        after_the_first_listing,
        "and it produced nothing: a size that came off real bytes is not an estimate"
    );

    // And the size it remembered is still the file's own length, which is the property RFC 0003
    // section 5.5 refuses to have estimated.
    for (page, size) in (1..=5).zip(&sizes) {
        let path = format!("/pages/{page:04}.pdf");
        assert_eq!(
            u64::try_from(read(&vfs, &path).len()).expect("fits"),
            size.expect("a file states a size")
        );
    }
}
