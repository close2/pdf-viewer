//! What a linearised document costs this reader before page one, against what Annex F allows.
//!
//! ISO 32000-2 §6.3.2.1 recommends that "Linearized files should be read as specified in Annex F",
//! and Annex F's subject is `CLAUDE.md` principle 2's: F.1 states the goal as "[w]hen a document
//! is opened, display the first page as quickly as possible". A recommendation this tree declines
//! is a departure to argue with a measurement rather than a gap to leave open, and this is the
//! measurement.
//!
//! # What it asks of each document
//!
//! - **Is it linearised?** F.3.3: "The linearization parameter dictionary shall be entirely
//!   contained within the first 1024 bytes of the PDF file. This limits the amount of data a PDF
//!   processor will have to read before deciding whether the file is linearized." So the test is
//!   the standard's own, on the standard's own window.
//! - **What does this reader read before page one?** `rchar` and `syscr` from `/proc/self/io`
//!   across `FileBytes::on_disk` plus [`Document::open`], then across the walk from `/Root` down
//!   to the leftmost leaf of the page tree. Those two are the whole of what stands between a file
//!   on disk and the object page one is drawn from.
//! - **What would Annex F let it read?** The linearization parameter dictionary's `E`, which
//!   Table F.1 defines as "[t]he offset of the end of the first page ... relative to the beginning
//!   of the PDF file". Everything Annex F's reader needs for page one — the header, the parameter
//!   dictionary, the first-page cross-reference table, the document-level objects, the primary
//!   hint stream and the first-page section itself — lies below it, and F.2's reader fetches that
//!   region in one request because a round trip over HTTP "can take up to several seconds".
//! - **What does this reader touch that Annex F's would not?** Every object the page-one walk
//!   resolves whose cross-reference offset is at or beyond `E`, counted and reported. F.3.10 puts
//!   the page tree in part 9, at the far end of the file, and says why: "This object can be
//!   located in this section because the PDF processor never needs to consult it" — never, because
//!   Annex F's reader takes the first page's object number from the parameter dictionary's `O`
//!   instead of walking the tree.
//!
//! The counters are per process and cumulative, so each figure here is a delta across one stage
//! of one document, in a process that opens documents one at a time.
//!
//! ```sh
//! cargo build --release -p pdf-syntax --example linearised_census
//! target/release/examples/linearised_census doc/corpora doc/pdf.js/test/pdfs
//! ```
//!
//! With no argument it walks nothing and says so: a census whose population is empty must not
//! print a tick (trap 13).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pdf_syntax::{Dictionary, Document, FileBytes, Limits, Object, ObjectId, xref::Location};

/// How many bytes of a file's front the linearization parameter dictionary may occupy.
///
/// ISO 32000-2 §F.3.3 fixes it, and fixes it so that this very question is cheap to ask:
///
/// > The linearization parameter dictionary shall be entirely contained within the first 1024
/// > bytes of the PDF file. This limits the amount of data a PDF processor will have to read
/// > before deciding whether the file is linearized.
const PARAMETER_WINDOW: usize = 1024;

/// How deep the page-tree walk will descend before giving up.
///
/// A page tree is a tree and this walk takes one child at each level, so the depth is the
/// document's, not its page count. A file that claims more levels than this has a cycle, and the
/// point of a bound is that a census over a crawled corpus meets one.
const MAX_TREE_DEPTH: usize = 64;

/// One counter of `/proc/self/io`, or `None` where there is no `/proc`.
fn io_counter(key: &str) -> Option<u64> {
    let io = std::fs::read_to_string("/proc/self/io").ok()?;
    io.lines()
        .find_map(|line| line.strip_prefix(key))?
        .trim()
        .parse()
        .ok()
}

/// How many bytes this process has had returned by a read, and how many read calls it has made.
///
/// `rchar` rather than `read_bytes`, for `launch_path.rs`'s reason: the second counts what reached
/// the block layer, which is zero for a file in the page cache and is therefore a measurement of
/// the cache rather than of the reader. What Annex F is about is how much the reader *asks for*.
fn reads() -> (u64, u64) {
    (
        io_counter("rchar:").unwrap_or(0),
        io_counter("syscr:").unwrap_or(0),
    )
}

/// What the linearization parameter dictionary states, as far as this census asks.
///
/// Read by scanning the first [`PARAMETER_WINDOW`] bytes rather than by opening the file, because
/// that is the decision F.3.3 sizes the window for — whether the file is linearised at all — and a
/// census that had to open a document to ask would be measuring the thing under test.
#[derive(Debug, Clone, Copy)]
struct Parameters {
    /// Table F.1's `L`: "[t]he length of the entire PDF file in bytes."
    ///
    /// **The standard's own disqualifier, and the reason it is read back.** Table F.1: "It shall
    /// be exactly equal to the actual length of the PDF file. A mismatch indicates that the file
    /// is not linearized and shall be treated as ordinary PDF file, ignoring linearization
    /// information." So a file whose `/Linearized` this census found is not necessarily one Annex
    /// F applies to, and asking is one comparison.
    length: Option<u64>,
    /// Table F.1's `E`: "[t]he offset of the end of the first page ... relative to the beginning
    /// of the PDF file."
    first_page_end: Option<u64>,
    /// Table F.1's `T`: the offset of the main cross-reference table, or of the main
    /// cross-reference stream object in a file that uses streams exclusively.
    main_xref: Option<u64>,
    /// Table F.1's `N`: "[t]he number of pages in the document."
    pages: Option<u64>,
    /// Table F.1's `O`: "[t]he object number of the first page's page object."
    first_page_object: Option<u64>,
}

/// The integer that follows `key` in `head`, where `key` is there and an integer follows it.
///
/// A scan rather than a parse, because the window is 1024 bytes of a file this census has not yet
/// decided is a PDF, and running the object parser over it would be a claim about the file's
/// syntax where all that is wanted is a number the standard puts in a fixed place.
///
/// **A name ends where the next character is not part of it, and this scan says so.** `/L` is a
/// prefix of both `/Linearized` and `/Length`, which Table F.1 and a hint stream's own dictionary
/// put inside the same window, so the byte after the key has to be white space or a digit for the
/// match to be this key at all. Every occurrence is tried, and the first one an integer follows
/// wins.
fn integer_after(head: &[u8], key: &[u8]) -> Option<u64> {
    let mut from = 0_usize;
    while let Some(found) = head
        .get(from..)
        .and_then(|rest| rest.windows(key.len()).position(|window| window == key))
    {
        let at = from.saturating_add(found).saturating_add(key.len());
        from = from.saturating_add(found).saturating_add(1);
        let rest = head.get(at..)?;
        let ends_here = rest
            .first()
            .is_some_and(|next| next.is_ascii_whitespace() || next.is_ascii_digit());
        if !ends_here {
            continue;
        }
        let digits: Vec<u8> = rest
            .iter()
            .copied()
            .skip_while(u8::is_ascii_whitespace)
            .take_while(u8::is_ascii_digit)
            .collect();
        if digits.is_empty() {
            continue;
        }
        if let Ok(number) = std::str::from_utf8(&digits).unwrap_or("").parse() {
            return Some(number);
        }
    }
    None
}

/// What F.3.3's window says about this file, or `None` where it does not name `/Linearized`.
fn parameters(head: &[u8]) -> Option<Parameters> {
    let window = head.get(..PARAMETER_WINDOW.min(head.len()))?;
    if !window
        .windows(b"/Linearized".len())
        .any(|candidate| candidate == b"/Linearized")
    {
        return None;
    }
    Some(Parameters {
        length: integer_after(window, b"/L"),
        first_page_end: integer_after(window, b"/E"),
        main_xref: integer_after(window, b"/T"),
        pages: integer_after(window, b"/N"),
        first_page_object: integer_after(window, b"/O"),
    })
}

/// Where the objects a page-one lookup resolved sit, and how many of them there were.
#[derive(Debug, Default)]
struct Touched {
    /// The cross-reference offsets of the objects this walk resolved, in file order.
    ///
    /// An object living inside an object stream contributes its *stream's* offset, which is where
    /// the reader actually has to read — F.4.1 says the same of a hint table's own answer:
    /// "PDF processors need to locate the object via a cross-reference stream, as it would if the
    /// hint table were not present."
    offsets: BTreeSet<usize>,
    /// How many objects the walk resolved that the cross-reference table gives no offset for.
    unplaced: usize,
}

/// Records where `id` lives, following an object stream to the stream's own offset.
fn note(document: &Document, id: ObjectId, into: &mut Touched) {
    match document.xref().location(id.number) {
        Some(Location::Offset(at)) => {
            into.offsets.insert(at);
        }
        Some(Location::InStream { stream, .. }) => match document.xref().location(stream) {
            Some(Location::Offset(at)) => {
                into.offsets.insert(at);
            }
            _ => into.unplaced = into.unplaced.saturating_add(1),
        },
        None => into.unplaced = into.unplaced.saturating_add(1),
    }
}

/// Walks `/Root` down to the leftmost leaf of the page tree, recording every object it resolves.
///
/// **This is what Annex F's `O` replaces**, and the whole of the difference between the two
/// routes once the file is on a local disk. F.3.10 puts the page tree at the end of a linearised
/// file precisely because the reader it is written for never walks it.
fn locate_the_first_page(document: &Document, into: &mut Touched) -> Option<Dictionary> {
    let catalog = document.catalog().ok()?;
    let mut node = match catalog.get("Pages") {
        Some(reference @ Object::Reference(id)) => {
            note(document, *id, into);
            document.resolve(reference).as_dict().cloned()?
        }
        Some(Object::Dictionary(direct)) => direct.clone(),
        _ => return None,
    };
    for _ in 0..MAX_TREE_DEPTH {
        let kids = match node.get("Kids") {
            Some(kids) => document.resolve(kids),
            None => return Some(node),
        };
        let Some(first) = kids.as_array().and_then(<[Object]>::first).cloned() else {
            return Some(node);
        };
        if let Object::Reference(id) = first {
            note(document, id, into);
        }
        let Some(child) = document.resolve(&first).as_dict().cloned() else {
            return Some(node);
        };
        node = child;
    }
    None
}

/// Drops one file's pages from the page cache, and says whether it could.
///
/// `posix_fadvise(POSIX_FADV_DONTNEED)`, which is what `dd oflag=nocache` is, and which an
/// unprivileged user may do to a file they can open for writing — `launch_path.rs`'s own method,
/// and `doc/habits/measuring.md`'s. Never the repository's own file: dropping a cache means
/// opening for writing, so the caller hands over a copy.
fn drop_the_page_cache(path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("dd")
        .arg("if=/dev/null")
        .arg(format!("of={}", path.display()))
        .arg("oflag=nocache")
        .arg("conv=notrunc,fdatasync")
        .arg("count=0")
        .output()
        .map_err(|error| format!("dd did not run: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "dd refused: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// How many cold samples a timed document is the minimum of.
///
/// A minimum rather than a mean, for `launch_path.rs`'s reason: contention adds time and never
/// removes it, so the fastest of several is the closest a shared machine can come to a quiet one.
const COLD_SAMPLES: usize = 7;

/// What one document cost, in the two stages a launch pays before page one is there to draw.
#[derive(Debug)]
struct Measured {
    /// Milliseconds the open took, where this run was asked for a clock.
    open_ms: Option<f64>,
    /// Milliseconds the walk to page one took, where this run was asked for a clock.
    page_ms: Option<f64>,
    /// Bytes `FileBytes::on_disk` and [`Document::open`] asked for.
    open_bytes: u64,
    /// Read calls the same two made.
    open_calls: u64,
    /// Bytes the walk from `/Root` to the leftmost leaf asked for.
    page_bytes: u64,
    /// Read calls that walk made.
    page_calls: u64,
    /// How many cross-reference sections the `/Prev` chain names.
    sections: usize,
    /// Where the objects that walk resolved sit.
    touched: Touched,
    /// Whether a first page was found at all.
    found: bool,
}

/// Opens one document and measures both stages.
fn measure(path: &Path, clock: bool) -> Option<Measured> {
    let bytes = FileBytes::on_disk(path).ok()?;
    let (before_bytes, before_calls) = reads();
    let began = std::time::Instant::now();
    let document = Document::open(bytes.clone()).ok()?;
    let open_ms = began.elapsed().as_secs_f64() * 1e3;
    let (after_open_bytes, after_open_calls) = reads();
    let mut touched = Touched::default();
    let walking = std::time::Instant::now();
    let found = locate_the_first_page(&document, &mut touched).is_some();
    let page_ms = walking.elapsed().as_secs_f64() * 1e3;
    let (after_page_bytes, after_page_calls) = reads();
    // A second walk of the same chain, after both figures are taken, so that asking how many
    // sections there are cannot be counted as bytes the opening path read (`xref::sections`'s own
    // doc comment says nothing on the opening path calls it).
    let sections = pdf_syntax::xref::sections(&bytes, Limits::default()).len();
    Some(Measured {
        open_ms: clock.then_some(open_ms),
        page_ms: clock.then_some(page_ms),
        open_bytes: after_open_bytes.saturating_sub(before_bytes),
        open_calls: after_open_calls.saturating_sub(before_calls),
        page_bytes: after_page_bytes.saturating_sub(after_open_bytes),
        page_calls: after_page_calls.saturating_sub(after_open_calls),
        sections,
        touched,
        found,
    })
}

/// Every `.pdf` under `root`, or `root` itself where it is one.
fn documents(root: &Path, into: &mut Vec<PathBuf>) {
    if root.is_file() {
        into.push(root.to_owned());
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                || path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        })
        .collect();
    found.sort();
    for path in found {
        if path.is_dir() {
            documents(&path, into);
        } else {
            into.push(path);
        }
    }
}

/// What the whole population came to.
#[derive(Debug, Default)]
struct Totals {
    /// How many files were looked at.
    documents: usize,
    /// How many of them state `/Linearized` inside F.3.3's window.
    linearised: usize,
    /// How many linearised ones this reader could open and find a first page in.
    measured: usize,
    /// How many of those read fewer bytes before page one than `E`, which is the region Annex F's
    /// reader fetches.
    under_e: usize,
    /// How many resolved an object at or beyond `E` — the page tree F.3.10 puts at the far end.
    past_e: usize,
    /// How many resolved an object at or beyond `T`, the main cross-reference table's own offset.
    past_t: usize,
    /// How many state an `L` that is not the file's own length, which Table F.1 says makes the
    /// file "not linearized" and to be "treated as ordinary PDF file, ignoring linearization
    /// information" — almost always an incremental update appended after the fact (F.3.6).
    stale: usize,
    /// How many of [`Self::measured`] are still linearised by that test.
    live: usize,
    /// Of those, how many read fewer bytes before page one than `E`.
    live_under_e: usize,
    /// Of those, how many resolved an object at or beyond `E`.
    live_past_e: usize,
    /// Bytes read before page one across those, and the same sum of `E`.
    live_bytes: u64,
    /// See [`Self::live_bytes`].
    live_regions: u64,
    /// How many have more than one cross-reference section, so that this reader read the main
    /// table as well as the first-page one.
    chained: usize,
    /// Bytes read before page one, summed.
    bytes: u64,
    /// The same sum of `E`.
    first_page_regions: u64,
}

#[expect(
    clippy::print_stdout,
    reason = "a census whose output is the table a round reads"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one census, printed in the order a reader wants it: a line per document, then the \
              totals. A split would scatter one table."
)]
fn main() {
    let roots: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        println!(
            "linearised-census: no directory given, so nothing was walked — name the corpora to \
             measure"
        );
        return;
    }
    let mut files = Vec::new();
    for root in &roots {
        documents(root, &mut files);
    }
    let verbose = std::env::var("LINEARISED_CENSUS_VERBOSE").is_ok();
    // **A cold figure is asked for rather than assumed**, because taking one means copying every
    // document and evicting its pages, which over a corpus is minutes of disk for a question most
    // runs are not asking. Named directory, because the copy must be on a file system whose pages
    // can be dropped at all — `/tmp` here is a `tmpfs`, where they cannot.
    let cold = std::env::var("LINEARISED_CENSUS_COLD")
        .ok()
        .map(PathBuf::from);
    if let Some(directory) = cold.as_deref() {
        if let Err(error) = std::fs::create_dir_all(directory) {
            println!(
                "linearised-census: {} is not a directory to copy into: {error}",
                directory.display()
            );
            return;
        }
        println!(
            "linearised-census: cold figures, each the quickest of {COLD_SAMPLES} opens of a copy \
             in {} whose pages were dropped first; load average {}",
            directory.display(),
            std::fs::read_to_string("/proc/loadavg").map_or_else(
                |_| "unknown".to_owned(),
                |text| text
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_owned()
            )
        );
    }
    let mut totals = Totals::default();
    let mut ratios: Vec<(f64, String)> = Vec::new();
    for path in &files {
        totals.documents = totals.documents.saturating_add(1);
        let Ok(head) = std::fs::read(path).map(|whole| {
            whole
                .get(..PARAMETER_WINDOW.min(whole.len()))
                .unwrap_or_default()
                .to_vec()
        }) else {
            continue;
        };
        let Some(stated) = parameters(&head) else {
            continue;
        };
        totals.linearised = totals.linearised.saturating_add(1);
        // The cold arm reads a copy whose pages have just been dropped, and keeps the quickest;
        // the warm arm reads the document where it lies. Both on the copy where there is one, so
        // that the only difference between two figures is the cache.
        let source = cold.as_deref().map(|directory| {
            let copy = directory.join(path.file_name().unwrap_or_else(|| "document.pdf".as_ref()));
            let _ = std::fs::copy(path, &copy);
            copy
        });
        let reading = source.as_deref().unwrap_or(path);
        let mut quickest: Option<Measured> = None;
        for _ in 0..(if cold.is_some() { COLD_SAMPLES } else { 1 }) {
            if let Some(Err(error)) = source.as_deref().map(drop_the_page_cache) {
                println!("linearised-census: {} stayed warm: {error}", path.display());
            }
            let Some(taken) = measure(reading, cold.is_some()) else {
                break;
            };
            let slower = quickest.as_ref().is_some_and(|seen| {
                seen.open_ms.unwrap_or(f64::INFINITY) <= taken.open_ms.unwrap_or(f64::INFINITY)
            });
            if !slower {
                quickest = Some(taken);
            }
        }
        let Some(measured) = quickest else {
            println!("linearised-census: {} does not open", path.display());
            continue;
        };
        if !measured.found {
            println!("linearised-census: {} has no first page", path.display());
            continue;
        }
        totals.measured = totals.measured.saturating_add(1);
        // Table F.1's own test, asked before anything is concluded from this file: a length that
        // is not the file's length says the file "shall be treated as ordinary PDF file, ignoring
        // linearization information", so Annex F's route is not open on it whatever a reader does.
        let size = std::fs::metadata(path).map(|found| found.len()).ok();
        let live = matches!((stated.length, size), (Some(stated), Some(size)) if stated == size);
        if live {
            totals.live = totals.live.saturating_add(1);
        } else {
            totals.stale = totals.stale.saturating_add(1);
        }
        let before_page_one = measured.open_bytes.saturating_add(measured.page_bytes);
        totals.bytes = totals.bytes.saturating_add(before_page_one);
        if measured.sections > 1 {
            totals.chained = totals.chained.saturating_add(1);
        }
        let Some(end) = stated.first_page_end else {
            println!(
                "linearised-census: {} states no E, so Annex F's own bound is not available",
                path.display()
            );
            continue;
        };
        totals.first_page_regions = totals.first_page_regions.saturating_add(end);
        if before_page_one < end {
            totals.under_e = totals.under_e.saturating_add(1);
        }
        if live {
            totals.live_bytes = totals.live_bytes.saturating_add(before_page_one);
            totals.live_regions = totals.live_regions.saturating_add(end);
            if before_page_one < end {
                totals.live_under_e = totals.live_under_e.saturating_add(1);
            }
        }
        let beyond = measured
            .touched
            .offsets
            .iter()
            .filter(|&&at| u64::try_from(at).unwrap_or(u64::MAX) >= end)
            .count();
        if beyond > 0 {
            totals.past_e = totals.past_e.saturating_add(1);
            if live {
                totals.live_past_e = totals.live_past_e.saturating_add(1);
            }
        }
        // The same question against `T`, which Table F.1 puts at the main cross-reference table:
        // an object resolved beyond it is one this reader fetched from the very end of a file
        // Annex F arranged so that page one would need nothing there.
        let beyond_t = stated.main_xref.map_or(0, |main| {
            measured
                .touched
                .offsets
                .iter()
                .filter(|&&at| u64::try_from(at).unwrap_or(u64::MAX) >= main)
                .count()
        });
        if beyond_t > 0 {
            totals.past_t = totals.past_t.saturating_add(1);
        }
        // A ratio rather than a difference, so that a 15 MB document and a 40 KB one are
        // comparable: what Annex F's reader fetches for page one is `E`, and what this reader
        // asks for is the numerator.
        #[expect(
            clippy::cast_precision_loss,
            reason = "byte counts of a file this reader has already addressed in memory; f64 is \
                      exact to 2^53"
        )]
        let ratio = before_page_one as f64 / end.max(1) as f64;
        ratios.push((ratio, path.display().to_string()));
        if verbose {
            println!(
                "linearised-census: {} — {} pages stated, open {} bytes in {} calls, page one \
                 located in {} bytes and {} calls, {} sections, {} objects resolved of which {} \
                 at or beyond E={end}, {beyond_t} at or beyond T={}, first page is object {}, \
                 {:.2}x E{}",
                path.display(),
                stated.pages.unwrap_or(0),
                measured.open_bytes,
                measured.open_calls,
                measured.page_bytes,
                measured.page_calls,
                measured.sections,
                measured
                    .touched
                    .offsets
                    .len()
                    .saturating_add(measured.touched.unplaced),
                beyond,
                stated.main_xref.unwrap_or(0),
                stated.first_page_object.unwrap_or(0),
                ratio,
                match (measured.open_ms, measured.page_ms) {
                    (Some(open), Some(page)) =>
                        format!(", cold open {open:.3} ms and page one located in {page:.3} ms"),
                    _ => String::new(),
                }
            );
        }
    }
    ratios.sort_by(|left, right| right.0.total_cmp(&left.0));
    println!("linearised-census: the ten widest ratios of bytes-before-page-one to E");
    for (ratio, path) in ratios.iter().take(10) {
        println!("linearised-census:   {ratio:>8.2}x  {path}");
    }
    println!(
        "linearised-census: {} documents, {} linearised, {} measured; {} read fewer bytes before \
         page one than Annex F's own first-page region E, {} resolved an object at or beyond E, \
         {} resolved one at or beyond T, {} read a second cross-reference section",
        totals.documents,
        totals.linearised,
        totals.measured,
        totals.under_e,
        totals.past_e,
        totals.past_t,
        totals.chained
    );
    println!(
        "linearised-census: {} bytes read before page one across the population, against {} bytes \
         of first-page region Annex F's reader would fetch",
        totals.bytes, totals.first_page_regions
    );
    println!(
        "linearised-census: of the measured, {} state an L that is not the file's length, which \
         Table F.1 says is not a linearised file at all; the remaining {} are, and of those {} \
         read fewer bytes before page one than E and {} resolved an object at or beyond it, for \
         {} bytes against {} of first-page region",
        totals.stale,
        totals.live,
        totals.live_under_e,
        totals.live_past_e,
        totals.live_bytes,
        totals.live_regions
    );
}
