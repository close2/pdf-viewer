//! How large a window a content stream needs: the biggest single token, and the inline images.
//!
//! `doc/todo/14` — road D, streaming the decompression — has two open questions that are
//! measurements rather than opinions, and this is the instrument for both.
//!
//! - **A reader-fed lexer needs a window that can hold the largest single lexical object.**
//!   `Limits::max_string_len` is 2²⁶, which bounds a string in the *file body* and says
//!   nothing about what a content stream contains. This walks every page's content and
//!   prints the largest token, by kind, with the document it was found on.
//! - **`inline_image::scan` searches forward from `ID` for `EI`** over data whose length the
//!   dictionary need not state (§8.9.7), which is a lookahead of unbounded size inside a
//!   bounded window. This prints how large real inline images are and, for each, which of
//!   the clause's three answers decides where its data ends — because only the third of them
//!   is a search.
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

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use pdf_syntax::{Document, Object, Token};

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

/// Which of §8.9.7's three answers says where an inline image's data ends.
///
/// The classification is made from the image dictionary rather than from `inline_image`'s
/// own result, because the question a window asks is what it can know *before* reading the
/// data: a stated length or a derivable one can be skipped over, and nothing else can.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// `/L` (or `/Length`) — "shall be present on all inline images" in a PDF 2.0 file.
    Stated,
    /// Unfiltered data, whose byte count §8.9.3's sample layout fixes exactly.
    Arithmetic,
    /// Filtered data with no stated length: the module's one guess, and a forward search.
    Search,
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
                    let scanned = pdf_model::inline_image::scan(
                        document,
                        content.as_slice(),
                        lexer.position(),
                        &page.resources,
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
                            if route == Route::Search {
                                tally.largest_searched.offer(length, where_);
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
        Route::Search
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
        "  by route — stated /L {}, unfiltered arithmetic {}, forward search for EI {}",
        tally.by_route[Route::Stated as usize],
        tally.by_route[Route::Arithmetic as usize],
        tally.by_route[Route::Search as usize]
    );
    report_largest("largest image", &tally.largest_image);
    report_largest("largest searched-for", &tally.largest_searched);
    for (count, window) in tally.images_past.iter().zip(WINDOWS) {
        println!("  images larger than {:>8}: {count}", human(window));
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
