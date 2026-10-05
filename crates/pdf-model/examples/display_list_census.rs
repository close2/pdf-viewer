//! How large a display list a real first page builds: commands, and the bytes they hold.
//!
//! The census behind `MAX_LIST_BYTES` (ADR 1507). A display list is what `interpret` keeps, and
//! its size is decided by the document: a content stream of a few kilobytes may state four
//! million operators, and a tiling cell or a Type 3 glyph replays what it marked once per site.
//! The bound on its bytes is a stated multiple of what real pages need, and this is what says
//! what they need.
//!
//! ```sh
//! # one document, its memory by stage (`VmHWM` reset between stages)
//! cargo run --release -p pdf-model --example display_list_census -- --stages <file.pdf>
//! # a population, first page of each, one line per document, the largest at the end
//! find doc/pdf.js/test/pdfs corpus-cache -name '*.pdf' -print0 |
//!     cargo run --release -p pdf-model --example display_list_census -- -
//! # or the same list from a file, for a wrapper that does not pass standard input on
//! cargo run --release -p pdf-model --example display_list_census -- <list>
//! ```
//!
//! The bytes are [`pdf_model::Interpretation::list_bytes`], the charge the interpreter makes as
//! it builds — so the number this prints and the number the bound is asked against are one.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::cast_precision_loss,
    reason = "byte counts printed to two decimal places of a mebibyte; a measurement rather than a \
              shipped path"
)]

use std::io::Read;
use std::sync::Mutex;
use std::time::Instant;

use rayon::prelude::*;

use pdf_syntax::Document;

/// One first page's list.
#[derive(Debug, Clone, Default)]
struct Measured {
    commands: usize,
    bytes: usize,
    limited: bool,
    millis: u128,
}

/// The process's peak resident set since the last reset, in kibibytes.
fn high_water() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

/// Resets `VmHWM` to the current resident set (`proc(5)`, `clear_refs` value 5).
fn reset_high_water() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

fn measure(bytes: Vec<u8>) -> Option<Measured> {
    let document = Document::open(bytes).ok()?;
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0)?;
    let started = Instant::now();
    let interpretation = pdf_model::interpret(&document, &page);
    let millis = started.elapsed().as_millis();
    Some(Measured {
        commands: interpretation.display_list.command_count(),
        bytes: interpretation.list_bytes,
        limited: interpretation
            .unsupported
            .iter()
            .any(|unsupported| matches!(unsupported, pdf_model::Unsupported::ListBytes { .. })),
        millis,
    })
}

fn stages(path: &str) {
    let mebibytes = |kib: u64| kib as f64 / 1024.0;
    reset_high_water();
    let base = high_water();
    let bytes = std::fs::read(path).expect("readable");
    let document = Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("a first page");
    println!(
        "open      peak {:8.1} MiB",
        mebibytes(high_water().saturating_sub(base))
    );
    reset_high_water();
    let started = Instant::now();
    let interpretation = pdf_model::interpret(&document, &page);
    let elapsed = started.elapsed();
    println!(
        "interpret peak {:8.1} MiB over the open, {elapsed:?}",
        mebibytes(high_water().saturating_sub(base))
    );
    let list = &interpretation.display_list;
    println!(
        "list      {} commands, {:.1} MiB charged, {} clips, {} soft masks",
        list.command_count(),
        interpretation.list_bytes as f64 / f64::from(1 << 20),
        list.clip_count(),
        list.soft_mask_count(),
    );
    println!("unsupported {:?}", interpretation.unsupported);
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().map(String::as_str) == Some("--stages") {
        stages(arguments.get(1).expect("a path to a PDF"));
        return;
    }
    // `-` is standard input; anything else is a file holding the list, for a wrapper such as
    // `tools/bounded.sh` that does not hand its standard input on.
    let mut listed = String::new();
    match arguments.first().map(String::as_str) {
        Some("-") | None => {
            std::io::stdin()
                .read_to_string(&mut listed)
                .expect("a NUL-separated list on standard input");
        }
        Some(file) => listed = std::fs::read_to_string(file).expect("a NUL-separated list"),
    }
    let paths: Vec<&str> = listed.split('\0').filter(|path| !path.is_empty()).collect();
    let largest_bytes = Mutex::new((0usize, String::new(), Measured::default()));
    let largest_commands = Mutex::new((0usize, String::new()));
    let limited = Mutex::new(Vec::new());
    let mut decades = [0usize; 12];
    let decade_counts = Mutex::new(&mut decades);
    paths.par_iter().for_each(|path| {
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let Some(measured) = measure(bytes) else {
            return;
        };
        println!(
            "{}\t{}\t{}\t{}\t{path}",
            measured.commands, measured.bytes, measured.millis, measured.limited
        );
        let decade = measured.bytes.checked_ilog10().unwrap_or(0) as usize;
        let mut counts = decade_counts.lock().unwrap();
        counts[decade.min(11)] = counts[decade.min(11)].saturating_add(1);
        drop(counts);
        if measured.limited {
            limited.lock().unwrap().push((*path).to_owned());
        }
        let mut most = largest_bytes.lock().unwrap();
        if measured.bytes > most.0 {
            *most = (measured.bytes, (*path).to_owned(), measured.clone());
        }
        let mut most = largest_commands.lock().unwrap();
        if measured.commands > most.0 {
            *most = (measured.commands, (*path).to_owned());
        }
    });
    let (bytes, path, measured) = largest_bytes.into_inner().unwrap();
    eprintln!("largest list in bytes: {bytes} ({measured:?}) {path}");
    let (commands, path) = largest_commands.into_inner().unwrap();
    eprintln!("largest list in commands: {commands} {path}");
    eprintln!("decades of bytes: {decades:?}");
    eprintln!(
        "refused by MAX_LIST_BYTES: {:?}",
        limited.into_inner().unwrap()
    );
}
