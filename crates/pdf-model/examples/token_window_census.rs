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
//!   stands and compares that with where the scan stopped, **per filter**. **This was written in
//!   the six-hundred-and-thirty-third session to size a defect before fixing it**, when the
//!   scan's answer for those images was a forward search for `EI` and an end past its answer
//!   meant a byte pair inside the encoded data had ended the image early; the
//!   six-hundred-and-thirty-fifth split it by filter, which is what showed that
//!   `CCITTFaxDecode` decides half the population and that one `DCTDecode` image in the crawl
//!   was ending early. Since ADRs 0464, 0466 and 0467 the scan derives every filter's end, so a
//!   disagreement here is now a *regression* — but the predicate stays this file's own reading
//!   of the clauses rather than a call into the code under test, which is trap 8's rule and the
//!   only thing that makes the comparison worth running.
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

use pdf_syntax::{Delimiting, Document, EncodedExtent, Object, Token};

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
    by_filter: BTreeMap<String, Verdicts>,
    /// The pages where the derived end and the search disagree, with the worst difference on
    /// each and which way it went.
    disagreeing: BTreeMap<String, (u64, &'static str)>,
}

/// What §7.3.8.2's end-of-data says about one filter's images, against where the scan stopped.
///
/// **Per filter rather than in one heap**, because the question the six-hundred-and-thirty-fifth
/// session was sent after is which *filters* still have their end guessed at, and a total over
/// six of them answers it for none.
#[derive(Debug, Default, Clone)]
struct Verdicts {
    /// Filtered images with no `/L` whose chain starts with this filter.
    images: u64,
    /// Of those, the ones whose first filter states an end-of-data this crate can locate.
    derivable: u64,
    /// Of those, the ones where the end stands exactly where the scan stopped.
    agreeing: u64,
    /// Of those, the ones where the end stands *past* where the scan stopped: a byte pair
    /// inside the encoded data was taken for the `EI` that ends the image.
    early: u64,
    /// Of those, the ones where the end stands before it.
    over_run: u64,
    /// Of those, the ones whose filter ran out of input before its end-of-data.
    short: u64,
    /// Of those, the ones whose framing this crate could not follow at all.
    unfollowable: u64,
    /// Encoded bytes standing past where the scan stopped, summed.
    lost: u64,
    /// Bytes the derived end leaves *out* of what the scan took, summed over the over-run
    /// images, and the largest of them.
    ///
    /// **This is the diagnostic that says what an over-run is**, and it is not a defect: §8.9.7
    /// takes one white-space character off the data's end and a producer may write two, so a
    /// derived end one byte tighter than the search's is the derivation being *right*. A wide
    /// one would be something else, which is why the two are counted apart.
    trimmed: u64,
    widest_trim: u64,
}

impl Verdicts {
    fn merge(&mut self, other: &Self) {
        self.images += other.images;
        self.derivable += other.derivable;
        self.agreeing += other.agreeing;
        self.early += other.early;
        self.over_run += other.over_run;
        self.short += other.short;
        self.unfollowable += other.unfollowable;
        self.lost += other.lost;
        self.trimmed += other.trimmed;
        self.widest_trim = self.widest_trim.max(other.widest_trim);
    }
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
        for (name, verdicts) in &other.by_filter {
            self.by_filter
                .entry(name.clone())
                .or_default()
                .merge(verdicts);
        }
        for (name, worst) in &other.disagreeing {
            let slot = self.disagreeing.entry(name.clone()).or_insert((0, "same"));
            if worst.0 > slot.0 {
                *slot = *worst;
            }
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
                                    &where_(),
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
    let first = first_filter(document, stream);
    let verdicts = tally.by_filter.entry(first).or_default();
    verdicts.images += 1;

    let Some(start) = data_start(content, opened) else {
        return;
    };
    let Some(tail) = content.get(start..) else {
        return;
    };
    let Some(delimiting) = delimiting_of(document, stream) else {
        verdicts.unfollowable += 1;
        return;
    };
    let extent = match pdf_syntax::filter::encoded_extent(
        delimiting,
        tail,
        document.limits().max_stream_len,
    ) {
        EncodedExtent::Ends(extent) => extent,
        // The bytes carry no end to compare against: the data stops short of the marker it
        // should have had, or its framing is not the one the filter describes.
        EncodedExtent::Short => {
            verdicts.short += 1;
            return;
        }
        EncodedExtent::Unknown => {
            verdicts.unfollowable += 1;
            return;
        }
    };
    verdicts.derivable += 1;
    let searched = stream.data.len();
    match extent.cmp(&searched) {
        std::cmp::Ordering::Equal => verdicts.agreeing += 1,
        std::cmp::Ordering::Greater => {
            let lost = (extent - searched) as u64;
            verdicts.early += 1;
            verdicts.lost += lost;
            note_disagreement(tally, name, lost, "early");
        }
        std::cmp::Ordering::Less => {
            let trim = (searched - extent) as u64;
            verdicts.over_run += 1;
            verdicts.trimmed += trim;
            verdicts.widest_trim = verdicts.widest_trim.max(trim);
            note_disagreement(tally, name, trim, "over-run");
        }
    }
}

/// Records the worst disagreement on one page between the derived end and the search's.
fn note_disagreement(tally: &mut Tally, where_: &str, bytes: u64, direction: &'static str) {
    let slot = tally
        .disagreeing
        .entry(where_.to_owned())
        .or_insert((0, direction));
    if bytes > slot.0 {
        *slot = (bytes, direction);
    }
}

/// The first filter of the image's chain, as a name to key the tally by.
fn first_filter(document: &Document, stream: &pdf_syntax::Stream) -> String {
    let name = match document.get_key(&stream.dict, "Filter") {
        Object::Name(first) => Some(first),
        Object::Array(items) => items.first().and_then(Object::as_name).cloned(),
        _ => None,
    };
    name.map_or_else(
        || "?".to_owned(),
        |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
    )
}

/// How the image's first filter says where its own data ends — **read here rather than taken
/// from `Document::filtered_extent`**, which is the code this comparison is about.
///
/// Trap 8's rule: a census whose predicate is the thing being checked measures nothing. The scan
/// asks `Document::filtered_extent` and this asks the clauses again, so the two can disagree —
/// which is the entire point of running it, and is what the six-hundred-and-thirty-third session
/// measured `FlateDecode` with before it changed anything.
///
/// - a marker the alphabet cannot contain: §7.4.2's `>` and §7.4.3's `~>`.
/// - a decoder's consumed input: §7.4.4's `FlateDecode` and `LZWDecode`.
/// - a length byte of 128, which is §7.4.5's EOD.
/// - ISO/IEC 10918-1's EOI marker, which §7.4.8 refers `DCTDecode` to.
/// - an end-of-block pattern, §7.4.6 Table 11's, unless `/EndOfBlock` switches it off.
fn delimiting_of(document: &Document, stream: &pdf_syntax::Stream) -> Option<Delimiting> {
    let dict = &stream.dict;
    let parms = match document.get_key(dict, "DecodeParms") {
        Object::Dictionary(parms) => Some(parms),
        Object::Array(items) => items.first().and_then(Object::as_dict).cloned(),
        _ => None,
    };
    let integer = |key: &str, fallback: i64| {
        parms
            .as_ref()
            .and_then(|parms| parms.get(key))
            .and_then(Object::as_integer)
            .unwrap_or(fallback)
    };
    Some(match first_filter(document, stream).as_str() {
        "FlateDecode" | "Fl" => Delimiting::Decoded(pdf_syntax::Pumping::Inflate),
        "LZWDecode" | "LZW" => Delimiting::Decoded(pdf_syntax::Pumping::Lzw {
            early_change: integer("EarlyChange", 1) != 0,
        }),
        "ASCIIHexDecode" | "AHx" => Delimiting::Marker(b">"),
        "ASCII85Decode" | "A85" => Delimiting::Marker(b"~>"),
        "RunLengthDecode" | "RL" => Delimiting::RunLength,
        "DCTDecode" | "DCT" => Delimiting::Jpeg,
        "CCITTFaxDecode" | "CCF" => {
            let stopped_by_rows = parms
                .as_ref()
                .and_then(|parms| parms.get("EndOfBlock"))
                .is_some_and(|flag| matches!(flag, Object::Boolean(false)));
            if stopped_by_rows {
                return None;
            }
            Delimiting::EndOfBlock {
                end_of_lines: if integer("K", 0) < 0 { 2 } else { 6 },
            }
        }
        _ => return None,
    })
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

/// The per-filter table: what §7.3.8.2's end-of-data says against where the scan stopped.
fn report_filtered_route(tally: &Tally) {
    println!("\nthe filtered route judged against §7.3.8.2's end-of-data marker:");
    println!(
        "  {:<20} {:>10} {:>10} {:>10} {:>8} {:>8} {:>8} {:>10}",
        "first filter",
        "images",
        "derivable",
        "agreeing",
        "early",
        "over-run",
        "short",
        "no framing"
    );
    let mut total = Verdicts::default();
    for (filter, row) in &tally.by_filter {
        total.merge(row);
        println!(
            "  {filter:<20} {:>10} {:>10} {:>10} {:>8} {:>8} {:>8} {:>10}",
            row.images,
            row.derivable,
            row.agreeing,
            row.early,
            row.over_run,
            row.short,
            row.unfollowable
        );
    }
    println!(
        "  {:<20} {:>10} {:>10} {:>10} {:>8} {:>8} {:>8} {:>10}",
        "all",
        total.images,
        total.derivable,
        total.agreeing,
        total.early,
        total.over_run,
        total.short,
        total.unfollowable
    );
    println!(
        "  encoded bytes lost to an early EI: {}; trimmed off an over-run: {} (widest {})",
        human(total.lost),
        human(total.trimmed),
        human(total.widest_trim)
    );
    println!(
        "  pages where the derived end and the search disagree: {}",
        tally.disagreeing.len()
    );
    let mut worst: Vec<(&String, &(u64, &'static str))> = tally.disagreeing.iter().collect();
    worst.sort_by_key(|entry| std::cmp::Reverse(entry.1.0));
    for (page, (bytes, direction)) in worst.iter().take(30) {
        println!("    {:>12}  {direction:<8}  {page}", human(*bytes));
    }
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

    report_filtered_route(&tally);

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
