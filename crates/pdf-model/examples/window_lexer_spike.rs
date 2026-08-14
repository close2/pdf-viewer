//! Road D's sink, as an experiment: a `Lexer` fed from a fixed window instead of a `Vec`.
//!
//! `doc/todo/14` says what a round taking road D owes first — **the measurement, not the
//! rewrite** — and this is the instrument for it. Two arms over the same page:
//!
//! - `whole` — today's route. `Page::content` decodes every `/Contents` part into one `Vec`
//!   and the lexer walks it.
//! - `window <bytes>` — the road. The `FlateDecode` pump writes into a fixed buffer, the
//!   lexer runs over what is in it, the unconsumed tail is compacted to the front and the
//!   pump refills. Nothing but the window and the file itself is ever resident.
//!
//! Both arms count lexer tokens and operators by `content_budget_census`'s rule, and the two
//! counts must agree: that is what says the window boundary was handled rather than papered
//! over. Run each arm as its own process and read `ru_maxrss` and `VmPeak` off it.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example window_lexer_spike -- <pdf> 0 whole
//! cargo run --profile gates -p pdf-model --example window_lexer_spike -- <pdf> 0 window 65536
//! ```
//!
//! **This is a spike and not a design.** It lexes; it does not interpret, and an interpreter
//! fed this way needs the two things §14 of this file's todo names: a `Token::Keyword` that
//! borrows the window may not outlive a refill, and `inline_image::scan` searches forward for
//! `EI` over data whose length the dictionary need not state.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters over one page's tokens, and cursor arithmetic inside a buffer whose \
              length bounds every index; this is a measurement rather than a shipped path"
)]

use std::path::PathBuf;

use pdf_syntax::{Document, Object, Token};

/// The bytes of one page's `/Contents`, produced a window at a time.
///
/// The parts are §7.8.2's: "the division between streams may occur only at the boundaries
/// between lexical tokens", so a newline is written between them exactly as `Page::content`
/// does, and the parts chain into one reader rather than into one buffer.
struct Producer {
    /// The raw, still-encoded bytes of each part, with a flag saying whether it inflates.
    parts: Vec<(std::sync::Arc<[u8]>, bool)>,
    /// Which part is being produced.
    index: usize,
    /// The inflate pump for the current part, held across refills.
    decoder: Option<flate2::Decompress>,
    /// How many of the current part's input bytes the pump has taken.
    consumed: usize,
    /// Whether the separator before the current part has been written.
    separator_pending: bool,
    /// Set when every part has been produced.
    finished: bool,
}

impl Producer {
    /// `None` where a part's filter chain is one this spike does not pump. It handles
    /// `FlateDecode` and no filter, which is what the two witnesses and most of a corpus use;
    /// every other filter §7.4 states is streaming too, and writing five more pumps would
    /// measure nothing this one does not.
    fn new(document: &Document, contents: &Object) -> Option<Self> {
        let items = match document.resolve(contents) {
            Object::Array(items) => items.iter().map(|item| document.resolve(item)).collect(),
            other => vec![other],
        };
        let mut parts = Vec::new();
        for item in items {
            let Some(stream) = item.as_stream() else {
                continue;
            };
            let filter = document.get_key(&stream.dict, "Filter");
            let inflates = match &filter {
                Object::Name(name) => name.as_bytes() == b"FlateDecode",
                Object::Array(names) => {
                    names.len() == 1
                        && matches!(names.first(), Some(Object::Name(name))
                            if name.as_bytes() == b"FlateDecode")
                }
                Object::Null => false,
                _ => return None,
            };
            if !inflates && !matches!(filter, Object::Null) {
                return None;
            }
            parts.push((std::sync::Arc::clone(&stream.data), inflates));
        }
        Some(Self {
            parts,
            index: 0,
            decoder: None,
            consumed: 0,
            separator_pending: false,
            finished: false,
        })
    }

    /// Writes as much as fits into `out`, returning how many bytes it wrote.
    ///
    /// Zero with [`Self::finished`] unset means only that this call made no progress into
    /// this buffer; zero with it set is the end of the content.
    fn fill(&mut self, out: &mut [u8]) -> usize {
        if out.is_empty() || self.finished {
            return 0;
        }
        let Some((data, inflates)) = self.parts.get(self.index).cloned() else {
            self.finished = true;
            return 0;
        };
        if self.separator_pending {
            self.separator_pending = false;
            out[0] = b'\n';
            return 1;
        }
        if !inflates {
            let left = &data[self.consumed..];
            let take = left.len().min(out.len());
            out[..take].copy_from_slice(&left[..take]);
            self.consumed += take;
            if self.consumed == data.len() {
                self.next_part();
            }
            return take;
        }

        let decoder = self
            .decoder
            .get_or_insert_with(|| flate2::Decompress::new(true));
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let status = decoder.decompress(&data[self.consumed..], out, flate2::FlushDecompress::None);
        self.consumed += usize::try_from(decoder.total_in() - before_in).unwrap();
        let wrote = usize::try_from(decoder.total_out() - before_out).unwrap();
        match status {
            // RFC 1951's final block, or damage. The shipped pump distinguishes them and
            // reports (ADR 0343); a spike that only counts tokens has nothing to report, and
            // both mean this part has no more bytes to give.
            Ok(flate2::Status::StreamEnd) | Err(_) => self.next_part(),
            Ok(flate2::Status::Ok | flate2::Status::BufError) => {
                // The same termination condition the shipped pump uses: no input taken and
                // no output written means the input ended before RFC 1951's final block.
                if decoder.total_in() == before_in && wrote == 0 {
                    self.next_part();
                }
            }
        }
        wrote
    }

    fn next_part(&mut self) {
        self.index += 1;
        self.decoder = None;
        self.consumed = 0;
        self.separator_pending = self.index < self.parts.len();
        if self.index >= self.parts.len() {
            self.finished = true;
        }
    }
}

/// What one arm counted.
#[derive(Debug, Default)]
struct Counts {
    tokens: u64,
    operators: u64,
    bytes: u64,
    /// The longest single token, in bytes — the quantity a window has to be able to hold.
    longest_token: usize,
    /// How many times the window was refilled, and how many bytes were re-lexed because a
    /// token or a comment ran up to the boundary.
    refills: u64,
    relexed: u64,
}

/// `content_budget_census`'s rule, written out again for the same reason it is there: a
/// keyword token at array depth zero is an operator (§7.8.2, §7.3.6).
fn tally(token: &Token<'_>, array_depth: &mut usize, counts: &mut Counts) {
    counts.tokens += 1;
    match token {
        Token::ArrayOpen => *array_depth += 1,
        Token::ArrayClose => *array_depth = array_depth.saturating_sub(1),
        Token::Keyword(_) if *array_depth == 0 => counts.operators += 1,
        _ => {}
    }
}

/// Today's route: the whole decoded content in one buffer.
fn whole(document: &Document, page: &pdf_model::Page) -> Counts {
    let content = page.content(document);
    let mut counts = Counts {
        bytes: content.len() as u64,
        ..Counts::default()
    };
    let mut lexer = pdf_syntax::Lexer::new(&content);
    let mut array_depth = 0usize;
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let Some(token) = lexer.next_token() else {
            break;
        };
        counts.longest_token = counts.longest_token.max(lexer.position() - start);
        tally(&token, &mut array_depth, &mut counts);
    }
    counts
}

/// The road: a fixed window, refilled by the pump and compacted after each pass.
///
/// `None` where [`Producer::new`] declines the page's filter chain.
fn window(document: &Document, page: &pdf_model::Page, size: usize) -> Option<Counts> {
    let stated = page.dict.get("Contents").cloned().unwrap_or(Object::Null);
    let mut producer = Producer::new(document, &stated)?;
    let mut buffer = vec![0u8; size];
    let mut filled = 0usize;
    let mut counts = Counts::default();
    let mut array_depth = 0usize;

    loop {
        // Fill what room there is. A single `fill` stops at a part boundary, so the loop
        // keeps asking until the buffer is full or the content has ended.
        while filled < buffer.len() && !producer.finished {
            let wrote = producer.fill(&mut buffer[filled..]);
            if wrote == 0 && producer.finished {
                break;
            }
            if wrote == 0 {
                continue;
            }
            filled += wrote;
            counts.bytes += wrote as u64;
        }
        counts.refills += 1;
        let at_end = producer.finished;

        // How far the pass got, which is where the next one resumes. The value leaves the
        // loop rather than being assigned into a variable outside it: every exit is one of
        // the two boundary cases below, and a trailing assignment would be dead code that
        // reads as the third.
        let consumed: usize = {
            let mut lexer = pdf_syntax::Lexer::new(&buffer[..filled]);
            loop {
                let before_whitespace = lexer.position();
                lexer.skip_whitespace();
                let start = lexer.position();
                let Some(token) = lexer.next_token() else {
                    // Nothing but white space left. A comment inside it may be cut by the
                    // boundary, and a comment that resumes as content is how a window-fed
                    // lexer silently changes a page — so the run is re-skipped after the
                    // refill rather than believed.
                    break if at_end {
                        filled
                    } else {
                        let run = &buffer[before_whitespace..filled];
                        if run.contains(&b'%') {
                            before_whitespace
                        } else {
                            filled
                        }
                    };
                };
                let end = lexer.position();
                if end == filled && !at_end {
                    // The token runs up to the boundary, so the bytes after it may belong to
                    // it. Re-lex it once there is more.
                    break start;
                }
                counts.longest_token = counts.longest_token.max(end - start);
                tally(&token, &mut array_depth, &mut counts);
            }
        };

        if at_end && consumed >= filled {
            break;
        }
        if consumed == 0 && filled == buffer.len() {
            // The window cannot hold this token, which is the one thing a fixed window
            // cannot do. Reported rather than papered over: what the design owes here is a
            // decision, and `token_window_census` says how large the largest real token is.
            println!("window {size}: a token longer than the window");
            return None;
        }
        counts.relexed += (filled - consumed) as u64;
        buffer.copy_within(consumed..filled, 0);
        filled -= consumed;
    }
    Some(counts)
}

fn report(arm: &str, counts: &Counts, elapsed: std::time::Duration) {
    println!(
        "{arm}: {} bytes, {} tokens, {} operators, longest token {} B, \
         {} refills, {} bytes re-lexed, {:.2} s",
        counts.bytes,
        counts.tokens,
        counts.operators,
        counts.longest_token,
        counts.refills,
        counts.relexed,
        elapsed.as_secs_f64()
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().expect("a document"));
    let index: usize = args
        .next()
        .expect("a page index")
        .parse()
        .expect("a page index");
    let arm = args.next().expect("`whole`, `window` or `both`");
    let size: usize = args
        .next()
        .unwrap_or_else(|| "65536".to_owned())
        .parse()
        .expect("a window size in bytes");

    let bytes = std::fs::read(&path).expect("the document is readable");
    // Said rather than panicked, so that a sweep over a corpus of malformed and encrypted
    // files distinguishes "this reader cannot open it" from "the window read it wrong".
    let Ok(document) = Document::open(bytes) else {
        println!("unopenable");
        return;
    };
    let pages = pdf_model::Pages::new(&document);
    let Some(page) = pages.get(index) else {
        println!("no page {index}");
        return;
    };

    match arm.as_str() {
        "whole" => {
            let started = std::time::Instant::now();
            let counts = whole(&document, &page);
            report("whole", &counts, started.elapsed());
        }
        "window" => {
            let started = std::time::Instant::now();
            match window(&document, &page, size) {
                Some(counts) => report(&format!("window {size}"), &counts, started.elapsed()),
                None => println!("skipped: a filter chain this spike does not pump"),
            }
        }
        // Both arms in one process, for the question the peaks cannot answer: does the
        // window read the same page? Two counts that disagree are a boundary handled wrong.
        "both" => {
            let started = std::time::Instant::now();
            let one = whole(&document, &page);
            report("whole", &one, started.elapsed());
            let started = std::time::Instant::now();
            match window(&document, &page, size) {
                Some(two) => {
                    report(&format!("window {size}"), &two, started.elapsed());
                    // The whole route ends every part with a newline and the window route
                    // writes one only between parts, so the byte counts differ by the parts.
                    println!(
                        "{}: {} tokens, {} operators",
                        if one.tokens == two.tokens && one.operators == two.operators {
                            "AGREE"
                        } else {
                            "DISAGREE"
                        },
                        one.tokens,
                        one.operators
                    );
                }
                None => println!("skipped: a filter chain this spike does not pump"),
            }
        }
        other => panic!("unknown arm {other}"),
    }
}
