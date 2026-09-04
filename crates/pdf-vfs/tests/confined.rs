//! The confined generator: that it answers, and that it is killed for asking.
//!
//! RFC 0003 section 6 puts every byte of PDF parsing behind a process with no filesystem and
//! no network, and `doc/todo/58` §4 says no face may ship before it exists. This is the pair of
//! statements that makes the confinement a boundary rather than a claim, which is exactly the
//! shape `viewer-confined`'s own suite has:
//!
//! - **It answers.** Every question in the layout, over a real document handed across as a
//!   descriptor, giving byte-for-byte what the in-process worker gives. A confinement that broke
//!   an answer would be a different program, not a safer one.
//! - **It is killed.** A probe that confines itself with the profile this worker uses and then
//!   opens a file, opens a socket, starts a program or stats a descriptor — each ending as
//!   `SIGSYS` or a refusal, never as success. And the two halves of ADR 0812's descriptor route:
//!   a `pread64` that works and an `fstat` that does not.
//! - **A death is an error, and the next question gets a fresh worker.** A mount that hung when
//!   its worker died would be worse than one that had none.
//!
//! # Trap 10
//!
//! `pdf-vfs-worker` is a separate binary and Cargo will not rebuild it for a test run. Every test
//! here that starts one says so by name rather than passing quietly when it is missing.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::sync::Arc;

use pdf_vfs::worker::{Answer, InProcessWorkers, Query, Worker, WorkerError, Workers};
use pdf_vfs::{Config, ConfinedWorkers, FileBacking, Vfs};

/// A committed document, which every checkout has once `doc/specifications.zip` is unpacked.
fn committed(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

/// The five-page annex: an outline, §12.4.2 labels, and pictures on its pages.
const FIVE_PAGES: &str = "PDF20_AN001-BPC.pdf";
/// The ten-page PDF Declarations note, which files two §7.11.4 embedded files.
const WITH_ATTACHMENTS: &str = "PDF-Declarations.pdf";
/// The fourteen-page annex about associated files, which is the document the write comparison
/// inserts: a *different* document of a different length.
const FOURTEEN_PAGES: &str = "PDF20_AN002-AF.pdf";
/// The seventy-two page tagging guide, whose page 60 is the only one here holding **two** images.
const TWO_IMAGES_ON_A_PAGE: &str = "Tagged-PDF-Best-Practice-Guide.pdf";
/// Which page that is.
const THE_PAGE_WITH_TWO: usize = 60;

/// Every question the layout can ask of the five-page annex, in one list.
///
/// Written out rather than derived, because what it is for is *comparison*: the same list goes to
/// both workers and the answers must be equal.
fn every_question() -> Vec<Query> {
    let mut questions = vec![
        Query::PageCount,
        Query::Information,
        Query::MetadataStream,
        Query::Outline,
    ];
    for page in 1..=5 {
        questions.push(Query::ExtractPage { page });
        questions.push(Query::PageText { page });
        questions.push(Query::ExtractImages { page });
    }
    questions.push(Query::RenderPage { page: 1, dpi: 150 });
    // The five write queries. Each of them computes §7.5.6's update and hands the **whole**
    // document back, so what this compares is the byte-for-byte identity of a file two workers
    // wrote — which is RFC 0002 section 9's first layer applied across the confinement, and the
    // reason nothing here has a clock in it.
    questions.push(Query::DeletePage { page: 2 });
    questions.push(Query::InsertPages {
        at: 3,
        document: std::fs::read(committed(FOURTEEN_PAGES)).expect("a committed document"),
    });
    questions.push(Query::Attach {
        name: String::from("crossing.txt"),
        bytes: b"a file written into the document".to_vec(),
    });
    questions.push(Query::SetInformation {
        json: br#"{"title": "set across the boundary"}"#.to_vec(),
    });
    // A name the document does not file: the refusal has to cross as the same refusal.
    questions.push(Query::Detach {
        name: String::from("nothing is filed under this"),
    });
    questions
}

/// A confined worker over a document on disk, or the reason there is none.
fn confined(name: &str) -> Box<dyn Worker> {
    let bytes = pdf_syntax::FileBytes::on_disk(&committed(name)).expect("a committed document");
    assert!(
        bytes.is_on_disk(),
        "the document was not opened on disk, so nothing here would test the descriptor route"
    );
    Box::new(started(&bytes))
}

/// A confined worker, or the reason there is none said in trap 10's words.
fn started(bytes: &pdf_syntax::FileBytes) -> pdf_vfs::Confined {
    ConfinedWorkers::start(
        bytes,
        None,
        pdf_transform::Policy::default(),
        pdf_transform::Budget::default(),
    )
    .unwrap_or_else(|error| {
        panic!(
            "no confined generator: {error} — trap 10, build it with `cargo build -p pdf-vfs \
             --bins`"
        )
    })
}

/// A corpus document, or `None` where `doc/pdf.js` is not checked out.
fn corpus(name: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    path.exists().then_some(path)
}

/// `CLAUDE.md` principle 3's *ask* level, across the real socket.
///
/// **The wire is what this adds to `a_write.rs`'s version of the same story.** The question and
/// the consent both cross `crates/confined-transport`'s frames — `Query::Consult` out,
/// `Answer::Consulted` back, then the operation wrapped in `Query::Consented` — so a face that
/// asks gets the same two answers whether the generator is in this process or behind a seccomp
/// filter. That is RFC 0003 section 6's own requirement on this seam, and the reason the *ask*
/// level could not be honoured before ADR 0874: the decision is taken where the parsing is, and
/// nothing there can put a question to anybody.
///
/// `bug1815476.pdf` is encrypted with `/P -1084`, so §7.6.4.2's Table 22 bit 4 is clear.
#[test]
fn a_question_and_a_consent_cross_the_confinement() {
    use pdf_vfs::{Consulted, Verb};

    let Some(path) = corpus("bug1815476.pdf") else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let asking = || {
        Vfs::new(
            Box::new(FileBacking::new(path.clone())),
            Box::new(ConfinedWorkers),
            Config {
                policy: pdf_transform::Policy {
                    restrictions: pdf_model::restriction::Level::Ask,
                },
                ..Config::default()
            },
        )
    };

    let vfs = asking();
    let consulted = vfs
        .consult("/pages/0001.pdf", Verb::Read)
        .expect("the question crosses the confinement");
    let Consulted::Ask { operation, reasons } = &consulted else {
        panic!("a confined worker answers the same question: {consulted:?}");
    };
    assert_eq!(*operation, "assembling a document out of these pages");
    assert_eq!(reasons, "Table 22 bit 11 is clear");

    // A no: the page does not come out, and the refusal is the *ask* level's own.
    assert!(vfs.answer(false).expect("a question was outstanding"));
    let error = vfs.open("/pages/0001.pdf").expect_err("a no is not done");
    assert!(
        matches!(
            error,
            pdf_vfs::VfsError::Worker(WorkerError::Unanswerable(_))
        ),
        "{error}"
    );

    // A yes. The question is put again first, because a `no` *forgets* it: an answer is an
    // answer to one question, and a face that answered twice would be answering nothing.
    assert!(matches!(
        vfs.consult("/pages/0001.pdf", Verb::Read).expect("asked"),
        Consulted::Ask { .. }
    ));
    assert!(vfs.answer(true).expect("a question was outstanding"));
    let page = vfs.open("/pages/0001.pdf").expect("a yes is done");
    assert!(page.bytes().starts_with(b"%PDF-"), "not a PDF");

    // And a consent is not a blanket. Table 22 bit 5's extraction is a *different* operation
    // from bit 11's assembly, so the yes just given does not release it and the listing of a
    // page's images is refused with the level's own sentence.
    let error = vfs
        .list("/images/0001")
        .expect_err("a yes to one operation is not a yes to another");
    assert!(
        matches!(
            error,
            pdf_vfs::VfsError::Worker(WorkerError::Unanswerable(_))
        ),
        "{error}"
    );
}

/// The same worker in this process, for comparison.
fn in_process(name: &str) -> Box<dyn Worker> {
    let bytes = pdf_syntax::FileBytes::on_disk(&committed(name)).expect("a committed document");
    InProcessWorkers
        .spawn(
            bytes,
            None,
            pdf_transform::Policy::default(),
            pdf_transform::Budget::default(),
        )
        .expect("an in-process worker cannot fail to start")
}

/// **The confinement is a transport and nothing else.**
///
/// Every question, both ways, compared. A page taken out of a confined worker is the same bytes a
/// page taken out of this process is, and so is a render, an image, a page's text, §14.3.3's
/// information, §14.3.2's metadata and §12.3.3's outline. That is the whole of what makes
/// `doc/todo/58` §4 a change of *where* the parsing happens.
#[test]
fn a_confined_worker_answers_exactly_what_the_in_process_one_answers() {
    let here = in_process(FIVE_PAGES);
    let there = confined(FIVE_PAGES);
    for question in every_question() {
        let ours = here.ask(&question).map_err(|error| error.to_string());
        let theirs = there.ask(&question).map_err(|error| error.to_string());
        assert_eq!(ours, theirs, "{question:?} answered differently");
    }
}

/// A page with **two** images, which is the first question that dispatches onto a second thread.
///
/// **Every probe in this file passed while this was broken, and that is the point of it.** A
/// mount by hand asked for `images/0060/` on the tagging guide and the worker died with
/// `SIGSYS`; the kernel's own audit line says `syscall=257`, `openat`. Nothing in the extraction
/// opens a file — what does is `glibc`, creating a per-thread allocator arena at the *first*
/// allocation of `rayon`'s pool thread and sizing the arena count from
/// `/sys/devices/system/cpu/online`. One image never reached that thread, because `rayon` runs a
/// single item on the caller's; two did (round 911, ADR 0864).
///
/// So the discriminator is **two**, and the fix is `MALLOC_ARENA_MAX` in
/// `confined_transport::Host::start`, which is the only place early enough to be read.
#[test]
fn a_page_with_two_images_does_not_kill_the_worker() {
    let question = Query::ExtractImages {
        page: THE_PAGE_WITH_TWO,
    };
    let here = in_process(TWO_IMAGES_ON_A_PAGE);
    let there = confined(TWO_IMAGES_ON_A_PAGE);
    let ours = here.ask(&question).expect("two images, unconfined");
    let theirs = there
        .ask(&question)
        .expect("two images, confined — a `SIGSYS` here is the allocator's arena, not the codec");
    assert_eq!(ours, theirs);
    let Answer::Files(files) = ours else {
        panic!("images are files")
    };
    assert_eq!(files.len(), 2, "the page that made this a question");
    assert!(
        there.is_alive(),
        "and the worker is still there to ask again"
    );
}

/// And an attachment's inventory and its bytes, which is the one question with a name in it.
#[test]
fn an_attachment_crosses_the_confinement_under_the_name_the_document_files_it_by() {
    let here = in_process(WITH_ATTACHMENTS);
    let there = confined(WITH_ATTACHMENTS);
    let Answer::Attachments(listed) = here
        .ask(&Query::AttachmentInventory)
        .expect("the inventory")
    else {
        panic!("the inventory is not an inventory");
    };
    assert!(
        !listed.is_empty(),
        "{WITH_ATTACHMENTS} states no attachment"
    );
    assert_eq!(
        there.ask(&Query::AttachmentInventory).ok(),
        Some(Answer::Attachments(listed.clone())),
        "the inventory changed on the way across"
    );
    for entry in &listed {
        let question = Query::ExtractAttachment {
            name: entry.name.clone(),
        };
        assert_eq!(
            here.ask(&question).ok(),
            there.ask(&question).ok(),
            "{} came back different from the confined worker",
            entry.name
        );
    }
}

/// The whole tree, driven the way a face drives it, over a confined worker.
///
/// `Vfs` is the broker: it holds the file, asks for the generation key and never parses. This is
/// that arrangement end to end, with `FileBacking` — the backing a face actually uses — so the
/// document reaches the worker as ADR 0812's descriptor rather than as bytes.
#[test]
fn a_mount_over_a_confined_worker_lists_stats_and_reads() {
    let vfs = Vfs::new(
        Box::new(FileBacking::new(committed(FIVE_PAGES))),
        Box::new(ConfinedWorkers),
        Config::default(),
    );
    assert_eq!(vfs.pages().expect("a page count"), 5);

    let root: Vec<String> = vfs
        .list("/")
        .expect("the root lists")
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert!(root.contains(&"pages".to_owned()), "{root:?}");

    let page = vfs.open("/pages/0002.pdf").expect("a page comes out");
    assert!(page.bytes().starts_with(b"%PDF-"), "not a PDF");
    assert_eq!(
        vfs.stat("/pages/0002.pdf").expect("a stat").size,
        Some(page.len()),
        "the stat did not state the size the read produced"
    );

    let text = vfs.open("/text/0001.txt").expect("a page's text");
    assert!(!text.is_empty(), "the first page extracted nothing");
}

/// **A worker that dies is an error, and the next question gets a fresh one.**
///
/// The requirement `doc/todo/58` §4 states as "never a hang". The worker is ended the one way a
/// confined worker can be ended — `SIGKILL`, which the document cannot decline — and three things
/// are asserted: the next question is a named error, the worker says it is no longer alive, and a
/// mount over it recovers on the operation after that rather than staying broken.
#[test]
fn a_killed_worker_becomes_an_error_and_the_next_question_gets_a_fresh_worker() {
    let vfs = Vfs::new(
        Box::new(FileBacking::new(committed(FIVE_PAGES))),
        Box::new(Recording),
        Config::default(),
    );
    assert_eq!(vfs.pages().expect("a page count"), 5);

    let first = Recording::last().expect("a worker was made");
    assert!(first.is_alive());
    first.canceller().cancel();

    // The mount was holding that worker for this generation, so the question that follows the
    // kill is the one that finds it gone. It is an error with a sentence, and it arrives — the
    // point of the test is that this call *returns*.
    let died = vfs.open("/pages/0001.pdf");
    assert!(
        matches!(
            died,
            Err(pdf_vfs::VfsError::Worker(WorkerError::Transport(_)))
        ),
        "a killed worker did not become a transport error: {died:?}"
    );
    assert!(!first.is_alive(), "a killed worker still says it is alive");

    // And the next one starts over. Same file, same generation key, new worker — which is what
    // `Vfs::current` asking `is_alive` is for.
    let page = vfs
        .open("/pages/0001.pdf")
        .expect("the mount did not recover from a dead worker");
    assert!(page.bytes().starts_with(b"%PDF-"));
    let second = Recording::last().expect("a second worker was made");
    assert!(second.is_alive());
}

/// A `Workers` that keeps what it made, so a test can end one.
///
/// A face would not need this: it holds a `Vfs` and not the workers under it. The test does,
/// because "the worker died" has to be something it *does* rather than something it waits for.
#[derive(Debug)]
struct Recording;

impl Recording {
    /// The most recent worker this factory produced.
    fn last() -> Option<Arc<pdf_vfs::Confined>> {
        LAST.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// The most recent worker, shared between the factory and the test.
static LAST: std::sync::Mutex<Option<Arc<pdf_vfs::Confined>>> = std::sync::Mutex::new(None);

impl Workers for Recording {
    fn spawn(
        &self,
        bytes: pdf_syntax::FileBytes,
        password: Option<pdf_transform::Secret>,
        policy: pdf_transform::Policy,
        budget: pdf_transform::Budget,
    ) -> Result<Box<dyn Worker>, WorkerError> {
        let shared = Arc::new(ConfinedWorkers::start(
            &bytes,
            password.as_ref(),
            policy,
            budget,
        )?);
        *LAST
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::clone(&shared));
        Ok(Box::new(Shared(shared)))
    }
}

/// The worker the test kept, as the trait object the mount holds.
#[derive(Debug)]
struct Shared(Arc<pdf_vfs::Confined>);

impl Worker for Shared {
    fn ask(&self, query: &Query) -> Result<Answer, WorkerError> {
        self.0.ask(query)
    }

    fn is_alive(&self) -> bool {
        self.0.is_alive()
    }
}

/// The confinement the worker reached is the one it reports.
///
/// A kernel can refuse what a build offers, so this is the assertion that on *this* machine the
/// build's offer was accepted: the filter is installed, the ceiling is there, and the Landlock
/// domain is enforced.
#[test]
#[cfg(target_os = "linux")]
fn the_confined_generator_reports_a_confinement_it_actually_has() {
    let bytes = pdf_syntax::FileBytes::on_disk(&committed(FIVE_PAGES)).expect("a document");
    let confinement = started(&bytes).confinement();
    assert!(
        confinement.is_enforced(),
        "the generator is not confined: {:?}",
        confinement.shortfall()
    );
    assert_eq!(
        confinement.landlock,
        pdf_sandbox::lockdown::LandlockLevel::Enforced,
        "{:?}",
        confinement.shortfall()
    );
    assert!(confinement.address_space_limit > 0);
}

/// Names the probe a re-executed test binary should run.
#[cfg(target_os = "linux")]
const PROBE_VARIABLE: &str = "PDF_VFS_TEST_PROBE";

/// Exit code from a probe whose forbidden operation was refused.
#[cfg(target_os = "linux")]
const REFUSED: i32 = 17;
/// Exit code from a probe whose forbidden operation *succeeded*, which is the failure.
#[cfg(target_os = "linux")]
const ALLOWED: i32 = 18;
/// Exit code from a probe that derived every file, which is what says the profile permits the
/// work.
#[cfg(target_os = "linux")]
const DERIVED: i32 = 19;

/// Whether a probe was stopped rather than served.
///
/// Two outcomes count. `SIGSYS` is the seccomp filter firing, which is what happens when the
/// system call is not on the allow-list at all. A clean `REFUSED` exit is the operation having
/// failed for a reason the confinement caused before the call could be made. Both are the
/// confinement working; only `ALLOWED` is not.
#[cfg(target_os = "linux")]
fn refused(status: std::process::ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal() == Some(libc::SIGSYS) || status.code() == Some(REFUSED)
}

/// Runs one probe in a fresh child and waits for it.
///
/// The child is this same test binary, re-executed with a filter that selects the one test below.
/// Confinement cannot be tested in the process doing the testing: it is irreversible and
/// process-wide, so the first thing it would break is the test harness.
#[cfg(target_os = "linux")]
fn run_probe(probe: &str) -> std::process::ExitStatus {
    std::process::Command::new(std::env::current_exe().expect("a test binary knows where it is"))
        .args(["--exact", "confined_probe", "--test-threads=1"])
        .env(PROBE_VARIABLE, probe)
        .output()
        .expect("the probe runs")
        .status
}

#[test]
#[cfg(target_os = "linux")]
fn a_confined_generator_cannot_open_a_file() {
    let status = run_probe("open");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined generator read a file it should not have been able to name"
    );
    assert!(
        refused(status),
        "expected the open to be refused or the process killed, got {status:?}"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn a_confined_generator_cannot_reach_the_network() {
    let status = run_probe("socket");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined generator opened a socket"
    );
    assert!(
        refused(status),
        "expected the socket to be refused or the process killed, got {status:?}"
    );
}

/// A confined generator cannot start another program.
///
/// The one that makes the interpreter profile's extra system calls defensible for this worker
/// too: it permits a *thread* and this says it does not permit a *program*. It is also why the
/// worker decodes §7.4.7's and §7.4.9's codecs in-process — it could not spawn `pdf-sandbox`'s
/// own worker, and a filter that let it would have given the confined process the one capability
/// the confinement is for.
#[test]
#[cfg(target_os = "linux")]
fn a_confined_generator_cannot_start_a_program() {
    let status = run_probe("spawn");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined generator started another program"
    );
    assert!(
        refused(status),
        "expected the spawn to be refused or the process killed, got {status:?}"
    );
}

/// A confined generator can read a descriptor it was handed, where the file's offsets point.
///
/// ADR 0812's `pread64`, proved against the kernel rather than against a list: a file opened
/// *before* the confinement is read at an offset behind it. The next test is this one's other
/// half.
#[test]
#[cfg(target_os = "linux")]
fn a_confined_generator_can_read_a_descriptor_it_holds() {
    let status = run_probe("pread");
    assert_eq!(
        status.code(),
        Some(ALLOWED),
        "a confined generator could not read a descriptor it holds: {status:?}"
    );
}

/// And it cannot ask the file system about that descriptor.
///
/// `fstat` is not on the allow-list, and that is the whole of what keeps a descriptor to one file
/// from being a file system: `statx` takes a path, so admitting it for the document would let a
/// confined process ask whether any file exists. The length crosses beside the descriptor instead
/// — `crate::wire`'s open frame states it. **This is the test that fails if somebody admits
/// `statx` "for the file's length".**
#[test]
#[cfg(target_os = "linux")]
fn a_confined_generator_cannot_stat_a_descriptor_it_holds() {
    let status = run_probe("fstat");
    assert_ne!(
        status.code(),
        Some(ALLOWED),
        "a confined generator asked the file system about a descriptor and was answered"
    );
    assert!(
        refused(status),
        "expected the stat to be refused or the process killed, got {status:?}"
    );
}

/// A substituted font is looked for on the machine, and a confined generator has no machine.
///
/// **The nine-hundred-and-fourteenth session's corpus walk found this on its first sixty
/// documents, and every probe in this file passed while it was broken** — which is the same
/// sentence the two-images test above carries, one layer further in. Four of those sixty name a
/// CJK or Arabic face without embedding it; `pdf_font::substitute` then walks
/// `/usr/share/fonts` to stand in for it, and `read_dir` is `openat`, which is off the
/// allow-list. A filter whose action is `SECCOMP_RET_KILL_PROCESS` does not return the `Err`
/// that code is written to shrug off: the generator dies, and the mount loses the generation.
///
/// So the reachability of the machine's fonts is *stated* before the confinement — that is what
/// `pdf_vfs::confine` does and what this probe exercises — and a confined worker then behaves
/// like a machine with no fonts installed, which `substitute::find` already guarantees never
/// fails. ADR 0870.
///
/// Calibrated against the tree without the fix: with the `no_machine_fonts` line in
/// `serve::confine` commented out, this probe is killed by `SIGSYS`.
#[test]
#[cfg(target_os = "linux")]
fn a_confined_generator_can_stand_in_for_a_font_it_cannot_look_up() {
    let status = run_probe("substitute");
    assert_eq!(
        status.code(),
        Some(ALLOWED),
        "a confined generator could not substitute a font: {status:?} — a `SIGSYS` here is the          walk over the machine's font directories"
    );
}

/// And it *can* still derive every file the layout offers.
///
/// The other half of the four above: a filter that refused everything would pass them all and be
/// useless. This one confines the process and then does the work — a page taken out, a page drawn
/// at 300 dpi, a page's images extracted, a page's text read and §14.3.3's information — which is
/// what says `Profile::Interpreter` is wide enough for this worker without being widened.
#[test]
#[cfg(target_os = "linux")]
fn a_confined_generator_can_still_derive_every_file() {
    let status = run_probe("derive");
    assert_eq!(
        status.code(),
        Some(DERIVED),
        "a confined generator could not derive the files the layout offers: {status:?}"
    );
}

/// The child half of the six confinement tests, and not a test of anything by itself.
///
/// Run normally it does nothing, because the variable is unset. Run by [`run_probe`] it confines
/// itself exactly as `pdf-vfs-worker` does and then attempts the thing in question, reporting the
/// answer as an exit code — the only channel it has left.
#[test]
#[cfg(target_os = "linux")]
fn confined_probe() {
    let Ok(probe) = std::env::var(PROBE_VARIABLE) else {
        return;
    };

    // Opened before the confinement, because a confined process has no filesystem — which is the
    // whole reason a document reaches the real worker as a descriptor beside its open frame. This
    // is that descriptor, for the two probes about what a confined process may do with one.
    let handle = std::fs::File::open(committed(FIVE_PAGES)).expect("the committed document opens");
    let bytes = pdf_syntax::FileBytes::on_disk(&committed(FIVE_PAGES)).expect("a document");

    let limits = pdf_vfs::confine().expect("a probe that cannot confine itself proves nothing");

    let permitted = match probe.as_str() {
        // `/proc/self/maps` rather than anything under `/etc`, because it is guaranteed to exist
        // and to be readable by this user: a failure here has to be the confinement.
        "open" => std::fs::File::open("/proc/self/maps").is_ok(),
        // One positional read of the header, which is `pread64` and nothing else; the answer is
        // checked so that a read that returned nothing would not count as permitted.
        "pread" => {
            use std::os::unix::fs::FileExt as _;
            let mut header = [0u8; 5];
            handle
                .read_at(&mut header, 0)
                .is_ok_and(|read| read == 5 && &header == b"%PDF-")
        }
        "fstat" => handle.metadata().is_ok(),
        // A `/BaseFont` that is *not* one of §9.6.2.2's fourteen, so the answer is looked for on
        // the machine before the compiled-in faces — which is the path that reads a directory.
        "substitute" => {
            let (bytes, _) = pdf_font::substitute::find(pdf_font::substitute::Request {
                family: pdf_font::substitute::Family::Serif,
                bold: false,
                italic: false,
                standard: false,
            });
            !bytes.is_empty() && !pdf_font::substitute::machine_fonts()
        }
        "socket" => std::net::UdpSocket::bind("127.0.0.1:0").is_ok(),
        "spawn" => std::process::Command::new("/bin/true").status().is_ok(),
        "derive" => {
            let worker = pdf_vfs::worker::InProcess::new(
                pdf_transform::Source::new(bytes),
                pdf_transform::Policy::default(),
                pdf_transform::Budget::default(),
                Some(limits.strips),
            );
            let derived = [
                Query::PageCount,
                Query::ExtractPage { page: 1 },
                Query::RenderPage { page: 1, dpi: 300 },
                Query::ExtractImages { page: 1 },
                Query::PageText { page: 1 },
                Query::AttachmentInventory,
                Query::Information,
                Query::MetadataStream,
                Query::Outline,
            ]
            .iter()
            .all(|question| worker.ask(question).is_ok());
            std::process::exit(if derived { DERIVED } else { REFUSED });
        }
        other => panic!("no probe named {other}"),
    };

    std::process::exit(if permitted { ALLOWED } else { REFUSED });
}
