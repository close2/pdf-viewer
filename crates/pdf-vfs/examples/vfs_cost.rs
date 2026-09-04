//! What a mount costs: a worker per generation, a question in each transport, and the peak.
//!
//! `cargo run --profile gates -p pdf-vfs --example vfs_cost -- [DOCUMENT ...]`
//!
//! `doc/todo/58` §5 said of RFC 0003's core that "[n]othing is measured. There is no gate on this
//! crate and no number anywhere: what a `stat` costs, what `ls images/` costs on a scanned book,
//! what the cache's hit rate is under a `cp -r`." This is the answer to the first half of it, and
//! the reason it exists at all: the confined worker of ADR 0847 puts a process spawn and a socket
//! round trip in front of every answer, and a cost nobody printed is a cost nobody can defend.
//!
//! # What is measured, and why each
//!
//! - **A worker per generation.** RFC 0003 section 5.4 rebuilds the tree whenever the file
//!   changes, and *rebuilding* means starting a worker and handing it the document. It is the
//!   one cost the
//!   confinement adds that has no in-process counterpart, so it is printed on its own — and it is
//!   what a face pays on the first touch of a mount and again after every write.
//! - **One question, in each transport, warm.** The same [`Query`] answered in this process and by
//!   the confined worker, so the difference is the socket and nothing else. Cheap questions
//!   (a page count, a page's text) are where a round trip shows; expensive ones (a 300 dpi render)
//!   are where it disappears into the work.
//! - **A wide directory, listed and read whole.** `doc/todo/58` §5 asked for "`ls images/` on a
//!   scanned book, the cache's hit rate under a `cp -r` of one directory", and this is that: the
//!   first page of the document that states any image has its `images/NNNN/` listed, listed again,
//!   and then every entry of it `stat`ed and read — which is what `cp -r` of that directory is.
//!   **The clock is the whole of the discriminator here, and that is worth saying**: round 923
//!   found `images/NNNN/` costing one extraction per *question* rather than per run, and
//!   `Vfs::generated` — the tree's own count of what it produced — stayed at one throughout,
//!   because the work was in validating the name rather than in producing the bytes (ADR 0886).
//!   It is printed beside the clock all the same, as the statement that the reads came out of the
//!   cache.
//! - **The peak, and where it is.** The broker's own resident high-water mark beside the largest
//!   answer this document produces — which is small, and that is the finding rather than an
//!   omission: the page's pixels are allocated in the *worker*, and what the broker ever holds is
//!   the encoded file. The worker's peak is bounded by the confinement rather than measured here,
//!   and [`pdf_vfs::message_budget`] prints the bound: a worker whose answer would not fit refuses
//!   it by name instead of writing a frame nobody can hold.
//!
//! Every figure is this machine's and this build's. Nothing here is a gate: a wall clock printed
//! by an example is a measurement, and `doc/todo/02` §2's floors are elsewhere.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    missing_docs,
    reason = "an example: it prints numbers, and a fixture it cannot read must fail loudly"
)]

use std::time::{Duration, Instant};

use pdf_vfs::worker::{Answer, InProcessWorkers, Query, Worker, Workers};
use pdf_vfs::{Config, ConfinedWorkers, FileBacking, MachineFaces, Vfs};

/// How many times each question is asked, so that one scheduling accident is not the number.
const ROUNDS: usize = 5;

fn main() {
    let named: Vec<String> = std::env::args().skip(1).collect();
    let documents: Vec<std::path::PathBuf> = if named.is_empty() {
        ["PDF20_AN001-BPC.pdf", "Tagged-PDF-Best-Practice-Guide.pdf"]
            .iter()
            .map(|name| committed(name))
            .filter(|path| path.is_file())
            .collect()
    } else {
        named.iter().map(std::path::PathBuf::from).collect()
    };
    assert!(
        !documents.is_empty(),
        "no document to measure: unpack doc/specifications.zip or name one"
    );

    for document in &documents {
        measure(document);
    }
}

fn committed(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

fn measure(path: &std::path::Path) {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    println!("\n== {name}");

    let bytes = pdf_syntax::FileBytes::on_disk(path).expect("a document on disk");
    let policy = pdf_transform::Policy::default();
    let budget = pdf_transform::Budget::default();

    // A worker per generation, both ways. The in-process one opens nothing and is the floor the
    // spawn is measured against; the confined one is a `fork`, an `execve`, a seccomp filter, a
    // Landlock ruleset, a greeting and the document's descriptor.
    let here = timed(ROUNDS, || {
        drop(
            InProcessWorkers
                .spawn(bytes.clone(), None, policy, budget)
                .expect("an in-process worker"),
        );
    });
    let there = timed(ROUNDS, || {
        drop(
            ConfinedWorkers::start(&bytes, None, policy, budget, MachineFaces::Withheld)
                .expect("a confined generator — trap 10, `cargo build -p pdf-vfs --bins`"),
        );
    });
    // The in-process figure is nearly nothing and is printed anyway, because what it says is
    // the honest thing: `InProcessWorkers::spawn` opens no file and reads no byte — it wraps the
    // bytes it was handed — so the whole of the second column is what the confinement costs.
    println!(
        "  a worker per generation   in process {:>9}   confined {:>9}   (+{:.2} ms)",
        span(here),
        span(there),
        there
            .as_secs_f64()
            .mul_add(1000.0, -(here.as_secs_f64() * 1000.0))
    );

    let unconfined = InProcessWorkers
        .spawn(bytes.clone(), None, policy, budget)
        .expect("an in-process worker");
    let confined = ConfinedWorkers::start(&bytes, None, policy, budget, MachineFaces::Withheld)
        .expect("a confined one");
    let pages = match unconfined.ask(&Query::PageCount).expect("a page count") {
        Answer::Count(pages) => pages,
        other => panic!("a page count is not {other:?}"),
    };
    println!("  {pages} page(s)");

    let questions = [
        ("page count      ", Query::PageCount),
        ("a page's text   ", Query::PageText { page: 1 }),
        ("§14.3.3 info    ", Query::Information),
        ("a page out      ", Query::ExtractPage { page: 1 }),
        ("a page's images ", Query::ExtractImages { page: 1 }),
        ("150 dpi render  ", Query::RenderPage { page: 1, dpi: 150 }),
        ("300 dpi render  ", Query::RenderPage { page: 1, dpi: 300 }),
    ];
    for (label, question) in &questions {
        let size = answer_bytes(&*unconfined, question);
        let ours = timed(ROUNDS, || drop(unconfined.ask(question)));
        let theirs = timed(ROUNDS, || drop(confined.ask(question)));
        println!(
            "  {label} {:>9} in process   {:>9} confined   answer {:>10}",
            span(ours),
            span(theirs),
            bytes_of(size)
        );
    }

    // The biggest answer this document can produce through the mount, and what the worker's
    // address space peaked at making it. `VmHWM` is the resident high-water mark, which is what a
    // machine actually paid; the confinement's own ceiling is on `VmSize`, and
    // `pdf_vfs::message_budget` is the arithmetic between the two.
    let biggest = Query::RenderPage { page: 1, dpi: 300 };
    let size = answer_bytes(&confined, &biggest);
    println!(
        "  the biggest answer        {:>10}   the broker's own peak {:>10}",
        bytes_of(size),
        bytes_of(high_water())
    );
    println!(
        "  the answer the confinement will not carry is anything past {:>10} \
         (a 4 GiB ceiling, {} pixels a page, two copies)",
        bytes_of(
            usize::try_from(pdf_vfs::message_budget(
                4 << 30,
                16 << 20,
                budget.max_pixels
            ))
            .unwrap_or(usize::MAX)
        ),
        budget.max_pixels
    );

    // And the same thing through the tree a face drives, so that the layout's own overhead — the
    // generation key, the path resolution, the cache — is in the number rather than beside it.
    let vfs = Vfs::new(
        Box::new(FileBacking::new(path)),
        Box::new(ConfinedWorkers::default()),
        Config::default(),
    );
    let cold = timed(1, || {
        std::hint::black_box(vfs.stat("/renders/300dpi/0001.png").expect("a stat"));
    });
    let warm = timed(ROUNDS, || {
        std::hint::black_box(vfs.stat("/renders/300dpi/0001.png").expect("a stat"));
    });
    println!(
        "  a `stat` that generates   {:>9} cold (the mount had no worker)   {:>9} cached",
        span(cold),
        span(warm)
    );
    wide_directory(&vfs, pages);
}

/// How many pages are asked for an image before the search gives up.
///
/// Each ask is one `pdf_transform::images` run, so this is a real cost and is stated rather than
/// left as `pages`. Sixty-four reaches page 60 of `Tagged-PDF-Best-Practice-Guide.pdf`, which is
/// the only page of it with more than one image and therefore the one worth timing.
const LOOKED_AT: usize = 64;

/// `ls -l` and then `cp -r` of one `images/NNNN/`, on the first page that states an image.
///
/// The module comment says what this is for. The listing is timed twice because the second one is
/// what says whether anything was remembered, and every entry is `stat`ed and read because that is
/// what a copy does — twenty thousand questions on the document that made this worth writing.
fn wide_directory(vfs: &Vfs, pages: usize) {
    // The search *is* the cold listing — a page asked for its images has had them extracted — so
    // the clock on the one that answers is the figure, and timing a second call afterwards would
    // print a warm listing under a cold heading.
    let mut found = None;
    for page in 1..=pages.min(LOOKED_AT) {
        let at = format!("/images/{page:04}");
        let started = Instant::now();
        let listed = vfs.list(&at).is_ok_and(|entries| !entries.is_empty());
        if listed {
            found = Some((at, started.elapsed()));
            break;
        }
    }
    let Some((at, cold)) = found else {
        println!("  no page of the first {LOOKED_AT} states an image");
        return;
    };
    let entries = vfs.list(&at).expect("a listing");
    let warm = timed(ROUNDS, || {
        std::hint::black_box(vfs.list(&at).expect("a listing"));
    });
    let before = vfs.generated();
    let copied = Instant::now();
    let mut bytes = 0_usize;
    for entry in &entries {
        let path = format!("{at}/{}", entry.name);
        vfs.stat(&path).expect("a stat");
        bytes = bytes.saturating_add(
            usize::try_from(vfs.open(&path).expect("an open").len()).unwrap_or(usize::MAX),
        );
    }
    let copied = copied.elapsed();
    println!(
        "  {at} ({} entries)   listed {:>9}   listed again {:>9}",
        entries.len(),
        span(cold),
        span(warm)
    );
    println!(
        "  a `cp -r` of it           {:>9} for {} stats and {} reads ({}), {} of them generated",
        span(copied),
        entries.len(),
        entries.len(),
        bytes_of(bytes),
        vfs.generated().saturating_sub(before)
    );
}

/// How many bytes one answer is, or 0 where it was refused.
fn answer_bytes(worker: &dyn Worker, query: &Query) -> usize {
    match worker.ask(query) {
        Ok(Answer::Bytes(bytes)) => bytes.len(),
        Ok(Answer::Files(files)) => files.values().map(Vec::len).sum(),
        Ok(_) | Err(_) => 0,
    }
}

/// This process's resident high-water mark in bytes, or 0 where it cannot be read.
///
/// `VmHWM` rather than `VmRSS`, because what a face pays for one answer is the peak and not
/// whatever is resident once the answer has been dropped.
fn high_water() -> usize {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(0, |kilobytes| kilobytes.saturating_mul(1024))
}

/// The best of `rounds` runs.
///
/// The best rather than the mean, for `doc/habits.md`'s reason: a slower run is a run that lost a
/// core to something else, and the fastest is the one where the machine got out of the way.
fn timed(rounds: usize, mut run: impl FnMut()) -> Duration {
    let mut best = Duration::MAX;
    for _ in 0..rounds.max(1) {
        let started = Instant::now();
        run();
        best = best.min(started.elapsed());
    }
    best
}

fn span(duration: Duration) -> String {
    let millis = duration.as_secs_f64() * 1000.0;
    if millis < 1.0 {
        format!("{:.0} µs", millis * 1000.0)
    } else {
        format!("{millis:.2} ms")
    }
}

fn bytes_of(count: usize) -> String {
    if count >= 1 << 20 {
        format!("{:.1} MiB", count as f64 / (1u64 << 20) as f64)
    } else if count >= 1 << 10 {
        format!("{:.1} KiB", count as f64 / 1024.0)
    } else {
        format!("{count} B")
    }
}
