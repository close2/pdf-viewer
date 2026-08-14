//! What reading every page of a document costs on one thread, on N sharing a document, and on
//! N each opening their own.
//!
//! This is the instrument behind ADR 0260. A document-wide search is a loop over pages that
//! walks §7.7.3.2's tree and interprets the leaf it finds (ADRs 0330 and 0335 have the split), and the
//! question this answers is what putting that loop on more than one thread buys — against the
//! only alternative that needs no shared state at all, which is one [`pdf_syntax::Document`] per
//! thread.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example parallel_sweep -- file.pdf [threads]
//! ```
//!
//! Three arrangements, each sweeping every page and adding up the text it read, so that a run
//! that skipped a page cannot look fast:
//!
//! - **one thread**, one document — what the viewer does today;
//! - **shared**, N threads over one `&Document`, which is what `RwLock` on the caches made
//!   possible and which is the arrangement this example exists to price;
//! - **per-thread**, N documents opened from the same bytes, one to a rayon worker — no shared
//!   state, N parses and N caches, and therefore the memory the project owner asked about.
//!
//! Each is swept **twice**, because a viewer searches a document it already has open and the
//! two arrangements differ most on the second sweep: `shared` finds its objects parsed, and
//! `per-thread` opens its documents again.
//!
//! N is the pool's size and not merely the number of tasks: `interpret` bands §8.9.5's colour
//! conversion across `rayon::current_num_threads()` of its own, so a run that split the pages
//! N ways inside the *global* pool would be measuring N tasks on 24 threads. Every parallel
//! section below runs inside a pool built with exactly `threads`.
//!
//! It prints the wall clock of each and the readback bytes all three must agree on, and — on
//! Linux, from `/proc/self/status` — the process's peak resident size. That last number is a
//! *process* high-water mark rather than a phase's, so name one arrangement with the fourth
//! argument (`one`, `shared` or `per-thread`) when it is the memory that is being asked about.
#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]

use std::sync::Arc;
use std::time::Instant;

use rayon::prelude::{IntoParallelIterator, ParallelIterator};

use pdf_model::view::ViewState;
use pdf_model::{Pages, content};
use pdf_syntax::Document;

/// The process's peak resident size in bytes, as Linux's `/proc/self/status` reports it.
///
/// `VmHWM` is the kernel's own high-water mark, so it survives the memory having been freed
/// again — which is the whole point of asking, since every arrangement below has given its
/// pages back by the time it prints. `None` where the file does not exist or does not say.
fn peak_resident() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kilobytes: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    kilobytes.checked_mul(1024)
}

/// Reads every page of one document on the calling thread, returning the readback's size.
fn sweep(document: &Document, range: std::ops::Range<usize>) -> usize {
    let pages = Pages::new(document);
    let state = ViewState::of(document);
    let mut bytes = 0_usize;
    for index in range {
        let Some(page) = pages.get(index) else {
            continue;
        };
        bytes = bytes.saturating_add(content::interpret_with(document, &page, &state).text.len());
    }
    bytes
}

/// Sweeps every page twice, reporting both, since a viewer searches a document twice.
fn twice<F: Fn() -> usize>(name: &str, sweep_once: F) {
    let started = Instant::now();
    let read = sweep_once();
    let cold = started.elapsed();
    let started = Instant::now();
    let again = sweep_once();
    let warm = started.elapsed();
    println!("  {name:<10} cold {cold:>9.2?}   warm {warm:>9.2?}   {read} bytes read");
    assert_eq!(
        read, again,
        "{name} read a different document the second time"
    );
}

/// One contiguous share of the pages for each of `threads` workers.
fn shares(pages: usize, threads: usize) -> Vec<std::ops::Range<usize>> {
    let each = pages.div_ceil(threads.max(1));
    (0..threads)
        .map(|worker| {
            let start = (worker.saturating_mul(each)).min(pages);
            start..(start.saturating_add(each)).min(pages)
        })
        .collect()
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        println!("usage: parallel_sweep <file.pdf> [threads] [one|shared|per-thread]");
        return;
    };
    let threads: usize = arguments
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or_else(rayon::current_num_threads);
    let only = arguments.next();
    let Ok(bytes) = std::fs::read(&path) else {
        println!("cannot read {path}");
        return;
    };
    let bytes: Arc<[u8]> = Arc::from(bytes);
    let Ok(document) = Document::open(Arc::clone(&bytes)) else {
        println!("{path} did not open");
        return;
    };
    let pages = Pages::new(&document).len();
    println!("{path}: {pages} pages, {threads} threads");

    let wanted = |name: &str| only.as_deref().is_none_or(|asked| asked == name);
    let Ok(pool) = rayon::ThreadPoolBuilder::new().num_threads(threads).build() else {
        println!("no thread pool of {threads}");
        return;
    };

    // One thread, on a document opened for this arrangement alone so that no cache above pays
    // for it. Every arrangement below opens its own for the same reason.
    if wanted("one") {
        let Ok(document) = Document::open(Arc::clone(&bytes)) else {
            return;
        };
        twice("one", || sweep(&document, 0..pages));
    }

    // N threads over one `&Document`: `Sync`, so the caches are shared and each page is parsed
    // once however many threads want it — and stays parsed for the second sweep.
    if wanted("shared") {
        let Ok(document) = Document::open(Arc::clone(&bytes)) else {
            return;
        };
        twice("shared", || {
            pool.install(|| {
                shares(pages, threads)
                    .into_par_iter()
                    .map(|range| sweep(&document, range))
                    .sum()
            })
        });
    }

    // N documents, one to a worker: no shared state and no lock, at the price of N parses and
    // N caches. The open is inside the timed region because it is a cost this arrangement has
    // and the other two do not — and it is a cost it pays again on the second sweep.
    if wanted("per-thread") {
        twice("per-thread", || {
            pool.install(|| {
                shares(pages, threads)
                    .into_par_iter()
                    .map(|range| {
                        Document::open(Arc::clone(&bytes)).map_or(0, |own| sweep(&own, range))
                    })
                    .sum()
            })
        });
    }

    match peak_resident() {
        Some(bytes) => println!("  peak resident {bytes} bytes"),
        None => println!("  peak resident: /proc/self/status does not say"),
    }
}
