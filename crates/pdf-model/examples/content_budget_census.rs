//! What a real page's content costs: operators, lexer tokens, and decoded bytes.
//!
//! The instrument behind `doc/todo/10`'s three bounds. Each of them is a number that refuses a
//! document, and each was set before anybody had measured the quantity it names:
//!
//! - **`MAX_OPERATIONS`** says "operators" and counted lexer *tokens* until the
//!   four-hundred-and-seventy-first session. This census prints both, per page, so the ratio
//!   between them is a measurement rather than an estimate — and it is not a constant: it is
//!   about 2 for text and about 7 for cubic Bézier artwork.
//! - **`Limits::max_stream_len`** bounds one stream's decoded data. This prints the largest one
//!   a corpus actually contains, which is what says whether the bound is near anything real.
//! - **The total of a page's `/Contents` parts** had no bound at all. This prints the largest
//!   total, per page, over every part §7.8.2 makes one stream.
//!
//! ```sh
//! cargo run --release -p pdf-model --example content_budget_census -- doc doc/pdf.js doc/corpora
//! ```
//!
//! With no directory named it reads `doc/pdf.js/test/pdfs`. Every directory is walked
//! recursively for `*.pdf`, so a corpus cache and a submodule are named the same way.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "counters over a corpus, four orders of magnitude below what a usize counts, and \
              two divisions whose result is printed to two decimal places; this is a \
              measurement rather than a shipped path"
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use pdf_syntax::{Document, ObjectId, Token};

/// The largest of something, with the document and page it was found on.
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

/// Everything one document contributes.
#[derive(Debug, Default, Clone)]
struct Tally {
    documents: usize,
    pages: usize,
    streams: usize,
    /// Pages whose token count passes four million — what the bound refused before.
    pages_over_tokens: usize,
    /// Pages whose operator count passes four million — what it refuses now.
    pages_over_operators: usize,
    largest_stream: Largest,
    largest_content: Largest,
    largest_operators: Largest,
    largest_tokens: Largest,
    /// Sum of operators and of tokens over every page, for the corpus-wide ratio.
    total_operators: u64,
    total_tokens: u64,
    /// How many pages fall in each decade of decoded content bytes.
    content_decades: [usize; 11],
    /// How many streams fall in each decade of decoded bytes.
    stream_decades: [usize; 11],
}

impl Tally {
    fn merge(&mut self, other: &Self) {
        self.documents += other.documents;
        self.pages += other.pages;
        self.streams += other.streams;
        self.pages_over_tokens += other.pages_over_tokens;
        self.pages_over_operators += other.pages_over_operators;
        self.total_operators += other.total_operators;
        self.total_tokens += other.total_tokens;
        self.largest_stream.merge(&other.largest_stream);
        self.largest_content.merge(&other.largest_content);
        self.largest_operators.merge(&other.largest_operators);
        self.largest_tokens.merge(&other.largest_tokens);
        for (slot, add) in self.content_decades.iter_mut().zip(other.content_decades) {
            *slot += add;
        }
        for (slot, add) in self.stream_decades.iter_mut().zip(other.stream_decades) {
            *slot += add;
        }
    }
}

/// Which power-of-two-hundred-and-fifty-six decade a byte count lands in, capped.
fn decade(bytes: u64) -> usize {
    let mut index = 0;
    let mut edge = 1024u64;
    while index < 10 && bytes >= edge {
        index += 1;
        edge = edge.saturating_mul(4);
    }
    index
}

/// The count of lexer tokens and of operators in one content stream.
///
/// The operator rule is `content.rs`'s: a keyword token at array depth zero. It is written
/// out again here deliberately — a census whose predicate is the code under test measures the
/// instrument (`doc/HANDOVER.md` trap 8), and this one has to be independent of the counter it
/// is being used to justify.
fn count(content: &[u8]) -> (u64, u64) {
    let mut lexer = pdf_syntax::Lexer::new(content);
    let mut tokens = 0u64;
    let mut operators = 0u64;
    let mut array_depth = 0usize;
    while let Some(token) = lexer.next_token() {
        tokens += 1;
        match token {
            Token::ArrayOpen => array_depth += 1,
            Token::ArrayClose => array_depth = array_depth.saturating_sub(1),
            Token::Keyword(_) if array_depth == 0 => operators += 1,
            _ => {}
        }
    }
    (tokens, operators)
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

    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        let length = data.len() as u64;
        tally.streams += 1;
        tally.stream_decades[decade(length)] += 1;
        tally
            .largest_stream
            .offer(length, || format!("{name} object {number}"));
    }

    let pages = pdf_model::Pages::new(&document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let content = page.content(&document);
        let (tokens, operators) = count(&content);
        tally.pages += 1;
        tally.total_tokens += tokens;
        tally.total_operators += operators;
        if tokens > 4_000_000 {
            tally.pages_over_tokens += 1;
        }
        if operators > 4_000_000 {
            tally.pages_over_operators += 1;
        }
        let length = content.len() as u64;
        tally.content_decades[decade(length)] += 1;
        tally
            .largest_content
            .offer(length, || format!("{name} page {}", index + 1));
        tally
            .largest_operators
            .offer(operators, || format!("{name} page {}", index + 1));
        tally
            .largest_tokens
            .offer(tokens, || format!("{name} page {}", index + 1));
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
    if bytes >= 1 << 30 {
        format!("{:.2} GiB", bytes as f64 / (1u64 << 30) as f64)
    } else if bytes >= 1 << 20 {
        format!("{:.2} MiB", bytes as f64 / (1u64 << 20) as f64)
    } else {
        format!("{bytes} B")
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
        "{} documents opened, {} pages, {} decoded streams",
        tally.documents, tally.pages, tally.streams
    );
    println!(
        "largest single decoded stream: {} ({})",
        human(tally.largest_stream.value),
        tally.largest_stream.where_
    );
    println!(
        "largest page /Contents total:  {} ({})",
        human(tally.largest_content.value),
        tally.largest_content.where_
    );
    println!(
        "most operators on one page:    {} ({})",
        tally.largest_operators.value, tally.largest_operators.where_
    );
    println!(
        "most lexer tokens on one page: {} ({})",
        tally.largest_tokens.value, tally.largest_tokens.where_
    );
    if tally.total_operators > 0 {
        println!(
            "tokens per operator, corpus-wide: {:.2}",
            tally.total_tokens as f64 / tally.total_operators as f64
        );
    }
    println!(
        "pages past four million tokens: {}; past four million operators: {}",
        tally.pages_over_tokens, tally.pages_over_operators
    );

    let mut edge = 1024u64;
    println!("\ndecoded stream lengths, and page content totals, by decade:");
    for index in 0..11 {
        let label = if index == 0 {
            "< 1 KiB".to_owned()
        } else {
            format!(">= {}", human(edge))
        };
        println!(
            "  {label:>12}  streams {:>8}   pages {:>8}",
            tally.stream_decades[index], tally.content_decades[index]
        );
        if index > 0 {
            edge = edge.saturating_mul(4);
        }
    }
}
