//! How large a window a content stream needs: the biggest single token, and the inline images.
//!
//! `doc/todo/14` — road D, streaming the decompression — has two open questions that are
//! measurements rather than opinions, and this is the instrument for both.
//!
//! - **A reader-fed lexer needs a window that can hold the largest single lexical object.**
//!   `Limits::max_string_len` is 2²⁶, which bounds a string in the *file body* and says
//!   nothing about what a content stream contains. This walks every page's content and
//!   prints the largest token, by kind, with the document it was found on.
//! - **`inline_image::scan` reads forward from `ID` for the end of the data** whose length the
//!   dictionary need not state (§8.9.7), which is a lookahead of unbounded size inside a
//!   bounded window. This prints how large real inline images are and, for each, which of
//!   §8.9.7's answers decides where its data ends.
//! - **And what the *filtered* answer is worth.** §7.3.8.2 makes a filtered extent derivable —
//!   "most filters are defined so that the data shall be self-limiting" — so for every filtered
//!   image with no `/L` this asks the first filter of the chain where its own end-of-data
//!   marker stands and compares that with where the scan stopped. **This was written in the
//!   six-hundred-and-thirty-third session to size a defect before fixing it**, when the scan's
//!   answer for those images was a forward search for `EI` and a marker past its answer meant a
//!   byte pair inside the compressed data had ended the image early. Since ADR 0466 the scan
//!   uses the same marker, so a disagreement is now a *regression* rather than a population —
//!   what the run still measures is how many filtered images have a marker this tree can find
//!   at all, which is the size of what the search still decides.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example token_window_census -- doc
//! ```
//!
//! With no directory named it reads `doc/pdf.js/test/pdfs`. Every directory is walked
//! recursively for `*.pdf`, so **`doc` alone is the whole population** — naming `doc/pdf.js`
//! or `doc/corpora` beside it counts every file under them twice.
//!
//! **Image data is never tokenised**, here or in the interpreter: the scan's `resume` is what
//! the walk continues from, so a JPEG's bytes cannot be counted as a token the way they would
//! be if the lexer were let loose on them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "counters over a corpus, far below what a usize counts, and divisions printed to \
              two decimal places; this is a measurement rather than a shipped path"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use pdf_syntax::{Document, EncodedExtent, Object, Token};

/// The largest of something, with where it was found.
#[derive(Debug, Default, Clone)]
struct Largest {
    value: u64,
    where_: String,
}

impl Largest {
    fn offer(&mut self, value: u64, where_: impl FnOnce() -> String) {
        if value > self.value {
            self.value = value;
            self.where_ = where_();
        }
    }

    fn merge(&mut self, other: &Self) {
        if other.value > self.value {
            self.clone_from(other);
        }
    }
}

/// Which of §8.9.7's answers says where an inline image's data ends.
///
/// The classification is made from the image dictionary rather than from `inline_image`'s
/// own result, because the question a window asks is what it can know *before* reading the
/// data: a stated length can be skipped over and the other two cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// `/L` (or `/Length`) — "shall be present on all inline images" in a PDF 2.0 file.
    Stated,
    /// Unfiltered data, whose byte count §8.9.3's sample layout fixes exactly.
    Arithmetic,
    /// Filtered data with no stated length: the filter's own end-of-data marker where this
    /// tree can find one, and the forward search for `EI` where it cannot.
    Filtered,
}

/// Token spans and inline images, by size.
#[derive(Debug, Default, Clone)]
struct Tally {
    documents: usize,
    pages: usize,
    tokens: u64,
    /// Token spans by decade, `decade`'s edges.
    token_decades: [u64; 11],
    largest_token: Largest,
    largest_string: Largest,
    largest_name: Largest,
    largest_number: Largest,
    largest_keyword: Largest,
    /// Tokens at or past each of the window sizes a round would consider.
    tokens_past: [u64; 4],
    images: u64,
    /// Inline images by route, and their data lengths by decade.
    by_route: [u64; 3],
    image_decades: [u64; 11],
    largest_image: Largest,
    largest_searched: Largest,
    /// Inline images whose data is longer than each candidate window.
    images_past: [u64; 4],
    /// Inline images the scan could not read at all.
    unreadable: u64,
    /// Filtered images with no `/L`, by the first filter of their chain.
    searched_by_filter: BTreeMap<String, u64>,
    /// Of those, the ones whose first filter states an end-of-data this crate can locate.
    derivable: u64,
    /// Of those, the ones where the marker stands exactly where the scan stopped.
    agreeing: u64,
    /// Of those, the ones where the marker stands *past* where the scan stopped: a byte pair
    /// inside the encoded data was taken for the `EI` that ends the image.
    early: u64,
    /// Of those, the ones where the marker stands before it, which no valid file writes.
    over_run: u64,
    /// Of those, the ones whose filter ran out of input before its marker.
    short: u64,
    /// Encoded bytes standing past where the scan stopped, summed.
    lost: u64,
    /// The documents an early `EI` was found in, with the worst loss on each.
    early_documents: BTreeMap<String, u64>,
}

/// The window sizes the count of "does not fit" is reported for.
const WINDOWS: [u64; 4] = [4 << 10, 64 << 10, 1 << 20, 16 << 20];

impl Tally {
    fn merge(&mut self, other: &Self) {
        self.documents += other.documents;
        self.pages += other.pages;
        self.tokens += other.tokens;
        self.images += other.images;
        self.unreadable += other.unreadable;
        self.derivable += other.derivable;
        self.agreeing += other.agreeing;
        self.early += other.early;
        self.over_run += other.over_run;
        self.short += other.short;
        self.lost += other.lost;
        for (name, count) in &other.searched_by_filter {
            *self.searched_by_filter.entry(name.clone()).or_default() += count;
        }
        for (name, worst) in &other.early_documents {
            let slot = self.early_documents.entry(name.clone()).or_default();
            *slot = (*slot).max(*worst);
        }
        for (slot, add) in self.token_decades.iter_mut().zip(other.token_decades) {
            *slot += add;
        }
        for (slot, add) in self.image_decades.iter_mut().zip(other.image_decades) {
            *slot += add;
        }
        for (slot, add) in self.by_route.iter_mut().zip(other.by_route) {
            *slot += add;
        }
        for (slot, add) in self.tokens_past.iter_mut().zip(other.tokens_past) {
            *slot += add;
        }
        for (slot, add) in self.images_past.iter_mut().zip(other.images_past) {
            *slot += add;
        }
        self.largest_token.merge(&other.largest_token);
        self.largest_string.merge(&other.largest_string);
        self.largest_name.merge(&other.largest_name);
        self.largest_number.merge(&other.largest_number);
        self.largest_keyword.merge(&other.largest_keyword);
        self.largest_image.merge(&other.largest_image);
        self.largest_searched.merge(&other.largest_searched);
    }
}

/// Which power-of-four decade a byte count lands in, from 16 bytes up.
fn decade(bytes: u64) -> usize {
    let mut index = 0;
    let mut edge = 16u64;
    while index < 10 && bytes >= edge {
        index += 1;
        edge = edge.saturating_mul(4);
    }
    index
}

/// Reads one page's content, counting token spans and measuring its inline images.
fn walk(document: &Document, page: &pdf_model::Page, name: &str, index: usize, tally: &mut Tally) {
    let content = page.content(document);
    let mut lexer = pdf_syntax::Lexer::new(&content);
    let where_ = || format!("{name} page {}", index + 1);
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let Some(token) = lexer.next_token() else {
            break;
        };
        let span = (lexer.position() - start) as u64;
        tally.tokens += 1;
        tally.token_decades[decade(span)] += 1;
        for (slot, window) in tally.tokens_past.iter_mut().zip(WINDOWS) {
            if span > window {
                *slot += 1;
            }
        }
        tally.largest_token.offer(span, where_);
        match &token {
            Token::String(_) => tally.largest_string.offer(span, where_),
            Token::Name(_) => tally.largest_name.offer(span, where_),
            Token::Integer(_) | Token::Real(_) => tally.largest_number.offer(span, where_),
            Token::Keyword(word) => {
                tally.largest_keyword.offer(span, where_);
                // §8.9.7's image data is not a program. The interpreter seeks past it and so
                // does this walk, which is also what keeps a JPEG out of the token census.
                if word == b"BI" {
                    let opened = lexer.position();
                    let scanned = pdf_model::inline_image::scan(
                        document,
                        content.as_slice(),
                        opened,
                        &page.resources,
                        // The census decodes each content stream whole, so the slice it hands
                        // over is all there is.
                        true,
                    );
                    tally.images += 1;
                    match &scanned.image {
                        Ok(stream) => {
                            let length = stream.data.len() as u64;
                            let route = route_of(document, stream);
                            tally.by_route[route as usize] += 1;
                            tally.image_decades[decade(length)] += 1;
                            for (slot, window) in tally.images_past.iter_mut().zip(WINDOWS) {
                                if length > window {
                                    *slot += 1;
                                }
                            }
                            tally.largest_image.offer(length, where_);
                            if route == Route::Filtered {
                                tally.largest_searched.offer(length, where_);
                                judge_the_search(
                                    document,
                                    content.as_slice(),
                                    opened,
                                    stream,
                                    name,
                                    tally,
                                );
                            }
                        }
                        Err(_) => tally.unreadable += 1,
                    }
                    lexer.seek(scanned.resume);
                }
            }
            _ => {}
        }
    }
}

/// Where the first byte of image data stands, counting from the `BI` at `opened`.
///
/// §8.9.7: "the ID operator shall be followed by a single white-space character, and the next
/// character shall be interpreted as the first byte of image data", with §7.2.3's CR-LF pair
/// counting as one such character.
///
/// **Read here rather than taken from `inline_image`**, which is the code this comparison is
/// about: trap 8's rule is that a census whose predicate is the thing being checked measures
/// nothing. The dictionary between `BI` and `ID` is a sequence of names and objects, so no
/// keyword can stand in it and the first one is the `ID`.
fn data_start(content: &[u8], opened: usize) -> Option<usize> {
    let mut lexer = pdf_syntax::Lexer::at(content, opened);
    loop {
        match lexer.next_token()? {
            Token::Keyword(word) if word == b"ID" => break,
            _ => {}
        }
    }
    let mut start = lexer.position();
    if content
        .get(start)
        .copied()
        .is_some_and(pdf_syntax::lexer::is_whitespace)
    {
        let carriage_return = content.get(start) == Some(&b'\r');
        start += 1;
        if carriage_return && content.get(start) == Some(&b'\n') {
            start += 1;
        }
    }
    Some(start)
}

/// Asks the first filter where its own end-of-data marker stands, and compares.
///
/// §7.3.8.2 is what makes the question answerable: "most filters are defined so that the data
/// shall be self-limiting; that is, they use an encoding scheme in which an explicit
/// end-of-data (EOD) marker delimits the extent of the data." Where the marker stands past the
/// `EI` the forward search stopped at, the search stopped inside the image's own bytes.
fn judge_the_search(
    document: &Document,
    content: &[u8],
    opened: usize,
    stream: &pdf_syntax::Stream,
    name: &str,
    tally: &mut Tally,
) {
    let filters = match document.get_key(&stream.dict, "Filter") {
        Object::Name(first) => vec![String::from_utf8_lossy(first.as_bytes()).into_owned()],
        Object::Array(items) => items
            .iter()
            .filter_map(|item| item.as_name())
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
            .collect(),
        _ => Vec::new(),
    };
    let first = filters.first().cloned().unwrap_or_else(|| "?".to_owned());
    *tally.searched_by_filter.entry(first).or_default() += 1;

    let Some(start) = data_start(content, opened) else {
        return;
    };
    let Some(tail) = content.get(start..) else {
        return;
    };
    let extent = match document.filtered_extent(&stream.dict, tail) {
        EncodedExtent::Ends(extent) => extent,
        // The bytes carry no marker to compare against: the filter is one this crate has no
        // resumable decoder for, or the data stops short of the marker it should have had.
        EncodedExtent::Short => {
            tally.short += 1;
            return;
        }
        EncodedExtent::Unknown => return,
    };
    tally.derivable += 1;
    let searched = stream.data.len();
    match extent.cmp(&searched) {
        std::cmp::Ordering::Equal => tally.agreeing += 1,
        std::cmp::Ordering::Greater => {
            tally.early += 1;
            let lost = (extent - searched) as u64;
            tally.lost += lost;
            let slot = tally.early_documents.entry(name.to_owned()).or_default();
            *slot = (*slot).max(lost);
        }
        std::cmp::Ordering::Less => tally.over_run += 1,
    }
}

/// Which of §8.9.7's answers this image's dictionary admits, before its data is read.
fn route_of(document: &Document, stream: &pdf_syntax::Stream) -> Route {
    if document
        .get_key(&stream.dict, "Length")
        .as_integer()
        .is_some()
    {
        Route::Stated
    } else if matches!(document.get_key(&stream.dict, "Filter"), Object::Null) {
        Route::Arithmetic
    } else {
        Route::Filtered
    }
}

fn one(path: &Path) -> Tally {
    let mut tally = Tally::default();
    let Ok(bytes) = std::fs::read(path) else {
        return tally;
    };
    let Ok(document) = Document::open(bytes) else {
        return tally;
    };
    tally.documents = 1;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let pages = pdf_model::Pages::new(&document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        tally.pages += 1;
        walk(&document, &page, &name, index, &mut tally);
    }
    tally
}

fn pdfs_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "pdf") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn human(bytes: u64) -> String {
    if bytes >= 1 << 20 {
        format!("{:.2} MiB", bytes as f64 / (1u64 << 20) as f64)
    } else if bytes >= 1 << 10 {
        format!("{:.2} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn report_largest(label: &str, largest: &Largest) {
    println!(
        "  {label:<22} {:>12}  ({})",
        human(largest.value),
        largest.where_
    );
}

fn main() {
    let roots: Vec<PathBuf> = {
        let named: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
        if named.is_empty() {
            vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs")]
        } else {
            named
        }
    };
    let mut files = Vec::new();
    for root in &roots {
        files.extend(pdfs_under(root));
    }
    println!("{} documents under {:?}", files.len(), roots);

    let total = Mutex::new(Tally::default());
    files.par_iter().for_each(|path| {
        let tally = one(path);
        total.lock().expect("no panic under the lock").merge(&tally);
    });
    let tally = total.into_inner().expect("no panic under the lock");

    println!(
        "{} documents opened, {} pages, {} content tokens",
        tally.documents, tally.pages, tally.tokens
    );
    println!("\nthe largest single lexical object, which is what a window must hold:");
    report_largest("any token", &tally.largest_token);
    report_largest("string", &tally.largest_string);
    report_largest("name", &tally.largest_name);
    report_largest("number", &tally.largest_number);
    report_largest("keyword", &tally.largest_keyword);
    for (count, window) in tally.tokens_past.iter().zip(WINDOWS) {
        println!("  tokens longer than {:>8}: {count}", human(window));
    }

    println!(
        "\ninline images (§8.9.7): {} read, {} unreadable",
        tally.images, tally.unreadable
    );
    println!(
        "  by route — stated /L {}, unfiltered arithmetic {}, filtered with no /L {}",
        tally.by_route[Route::Stated as usize],
        tally.by_route[Route::Arithmetic as usize],
        tally.by_route[Route::Filtered as usize]
    );
    report_largest("largest image", &tally.largest_image);
    report_largest("largest searched-for", &tally.largest_searched);
    for (count, window) in tally.images_past.iter().zip(WINDOWS) {
        println!("  images larger than {:>8}: {count}", human(window));
    }

    println!("\nthe filtered route judged against §7.3.8.2's end-of-data marker:");
    println!("  first filter of the chain, over every filtered image with no /L:");
    for (filter, count) in &tally.searched_by_filter {
        println!("    {filter:<20} {count}");
    }
    println!(
        "  marker locatable {}, of which agreeing {}, ended early {}, over-run {}; \
         short of a marker {}",
        tally.derivable, tally.agreeing, tally.early, tally.over_run, tally.short
    );
    println!("  encoded bytes lost to an early EI: {}", human(tally.lost));
    println!(
        "  documents with at least one early EI: {}",
        tally.early_documents.len()
    );
    let mut worst: Vec<(&String, &u64)> = tally.early_documents.iter().collect();
    worst.sort_by(|left, right| right.1.cmp(left.1));
    for (document, lost) in worst.iter().take(20) {
        println!("    {:>12}  {document}", human(**lost));
    }

    let mut edge = 16u64;
    println!("\ntoken spans, and inline image data lengths, by decade:");
    for index in 0..11 {
        let label = if index == 0 {
            "< 16 B".to_owned()
        } else {
            format!(">= {}", human(edge))
        };
        println!(
            "  {label:>12}  tokens {:>12}   images {:>8}",
            tally.token_decades[index], tally.image_decades[index]
        );
        if index > 0 {
            edge = edge.saturating_mul(4);
        }
    }
}
