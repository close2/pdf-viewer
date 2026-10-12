//! What the first frame's serial terms cost once one page is read on many threads (ADR 1888).
//!
//! ADR 1873 divides a page's read where `Q` restores the state (§8.4.2) and prices it on the
//! witness: the inflate, a planning pass, the pieces interpreted at once, the join. Once the
//! pieces run at once, what is left of the read is the terms that are still one thread's. This
//! prices each of them, in one process, on one page:
//!
//! 1. **The stream as written**: its `/Contents` parts, filter and lengths, and how many of
//!    RFC 1951's byte-aligned empty stored blocks (`00 00 FF FF`, what a `Z_SYNC_FLUSH` or a
//!    `Z_FULL_FLUSH` leaves) it holds — the only places a deflate stream offers a second decoder,
//!    and a sync flush offers it no dictionary.
//! 2. **The inflate**: through the page's own window (`ContentReader::read_to_end`, the read's
//!    path), whole into one buffer sized for it (the decoder's floor on one thread), and into one
//!    buffer a chunk at a time, as a producer that publishes its progress to readers would.
//! 3. **The lexer**: the page's reader over the inflated bytes held in memory — the rate any
//!    consumer of the inflate's output lexes at — beside the inflate's rate.
//! 4. **The planning**: the exact pass that finds the cuts (a lex that counts depth, serial), and
//!    a byte scan for a `Q` between delimiters, which is what a speculative planner reads.
//! 5. **The pieces, serial after the inflate and pipelined behind it**: the stream cut as ADR
//!    1873's instrument cuts it, each piece made a page of its own (§7.5.6), and interpreted
//!    either once the whole stream is inflated or as soon as the inflate has passed its end —
//!    on a fixed number of threads taking the pieces in order. Printed: when the read ends, when
//!    the first piece's list exists, and when the first quarter, half and three quarters of the
//!    pieces are done in order, which is what a drawer taking the pieces in order could start on.
//!    The pieces' lists joined equal the page's (checked once per count).
//! 6. **The drawer**: the page's parts as ADR 1855's reader hands them over, collected, then
//!    painted one after another into one target with `render_cpu::Underway` and finished — the
//!    drawer's whole work on one thread, which a read shorter than the draw leaves in front of
//!    the frame.
//!
//! The pieces are interpreted from prepared documents whose piece is unfiltered, so a piece
//! waits for the inflate's progress but reads its own copy: a build reads the inflated bytes
//! themselves, which this neither saves nor pays. The plan is ADR 1873's instrument's
//! (`read_split_price`), restated here because one example cannot call another's functions.
//!
//! ```sh
//! taskset -c <cores> cargo run --release -p pdf-model --example serial_terms_price -- \
//!     /tmp/demo_heavy_engineering_A1.pdf [rounds] [pieces, e.g. 12,48] [threads] [scale]
//! ```
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    reason = "a measurement tool: it stops loudly on an input it was not written for, its \
              arithmetic is on byte offsets and counts far inside every type it uses, and \
              printing is the whole point"
)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Barrier, Condvar, Mutex};
use std::time::{Duration, Instant};

use pdf_model::content::reader::ContentReader;
use pdf_model::progress::{Handed, Parts};
use pdf_render::display_list::{ClipId, Command, DisplayList};
use pdf_syntax::{Document, Lexer, Token};

/// The chunk a pipelined producer inflates into place before it says how far it has got.
const CHUNK: usize = 1 << 20;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document to read");
    let rounds: usize = arguments
        .next()
        .map_or(3, |n| n.parse().expect("a count of rounds"));
    let counts: Vec<usize> = arguments.next().map_or_else(
        || vec![12, 48],
        |list| {
            list.split(',')
                .map(|n| n.parse().expect("a piece count"))
                .collect()
        },
    );
    let threads: usize = arguments
        .next()
        .map_or(12, |n| n.parse().expect("a thread count"));
    let scale: Option<f32> = arguments.next().map(|n| n.parse().expect("a scale"));

    let bytes = std::fs::read(&path).expect("the document is readable");
    let document = Document::open(bytes.clone()).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("a first page");

    // 1. The stream as written.
    let contents = page
        .dict
        .get("Contents")
        .and_then(pdf_syntax::Object::as_reference)
        .expect("one /Contents stream, named by reference");
    let stream = document.get(contents);
    let stream = stream.as_stream().expect("a stream");
    let filter = stream
        .dict
        .get("Filter")
        .and_then(pdf_syntax::Object::as_name)
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned());
    assert_eq!(
        filter.as_deref(),
        Some("FlateDecode"),
        "one FlateDecode stream: what this prices"
    );
    assert!(
        stream.dict.get("DecodeParms").is_none(),
        "no predictor: §7.4.4.4 rows would order the bytes again"
    );
    let encoded: &[u8] = &stream.data;
    let content = ContentReader::for_page(&document, &page).read_to_end();
    // Table 31's separator: the window writes a white-space byte after the part, which the
    // decoder does not produce.
    let inflated = content.len() - 1;
    let flushes = encoded
        .windows(4)
        .filter(|window| *window == [0, 0, 0xFF, 0xFF])
        .count();
    println!(
        "stream: one part, /FlateDecode, {} bytes encoded, {} inflated ({:.2}×); zlib header \
         {:02x} {:02x}; byte-aligned empty stored blocks (00 00 FF FF) in it: {flushes}",
        encoded.len(),
        inflated,
        inflated as f64 / encoded.len() as f64,
        encoded[0],
        encoded[1],
    );

    // 2–4. The inflate, the lexer and the planning, interleaved round by round.
    let mut window = Vec::new();
    let mut whole = Vec::new();
    let mut chunked = Vec::new();
    let mut lexed = Vec::new();
    let mut windowed_lex = Vec::new();
    let mut exact = Vec::new();
    let mut scanned = Vec::new();
    let mut tokens = 0;
    let mut candidates = 0;
    for _ in 0..rounds {
        let started = Instant::now();
        let length = ContentReader::for_page(&document, &page)
            .read_to_end()
            .len();
        window.push(started.elapsed());
        assert_eq!(length, content.len(), "the stream reads the same twice");

        let started = Instant::now();
        let out = inflate_whole(encoded, inflated);
        whole.push(started.elapsed());
        assert!(
            out == content[..inflated],
            "the decoder's floor is the window's bytes"
        );
        drop(out);

        let started = Instant::now();
        let out = inflate_chunked(encoded, inflated, &AtomicUsize::new(0), &Signal::new());
        chunked.push(started.elapsed());
        assert!(
            out == content[..inflated],
            "the chunked inflate is the window's bytes"
        );
        drop(out);

        let started = Instant::now();
        tokens = count_tokens(&content);
        lexed.push(started.elapsed());

        let started = Instant::now();
        let mut reader = ContentReader::for_page(&document, &page);
        let mut through = 0usize;
        while reader.with_token(|token| token.is_some()) {
            through += 1;
        }
        windowed_lex.push(started.elapsed());
        assert_eq!(through, tokens, "the window lexes what the memory lexes");

        let started = Instant::now();
        std::hint::black_box(Plan::of(&content, 12));
        exact.push(started.elapsed());

        let started = Instant::now();
        candidates = scan_for_restores(&content);
        scanned.push(started.elapsed());
    }
    let rate = |times: &[Duration]| content.len() as f64 / fastest(times).as_secs_f64() / 1e9;
    println!(
        "inflate: the window {} ({:.2} GB/s out); whole into one buffer {} ({:.2} GB/s); \
         into place in {} KiB chunks {} ({:.2} GB/s)",
        spread(&window),
        rate(&window),
        spread(&whole),
        rate(&whole),
        CHUNK >> 10,
        spread(&chunked),
        rate(&chunked),
    );
    println!(
        "lex: {tokens} tokens over the bytes in memory {} ({:.2} GB/s); through the window, \
         inflate and lex on one thread, {} ({:.2} GB/s)",
        spread(&lexed),
        rate(&lexed),
        spread(&windowed_lex),
        rate(&windowed_lex),
    );
    println!(
        "planning: the exact pass, serial, {} ({:.2} GB/s); a byte scan for a Q between \
         delimiters {} ({:.2} GB/s, {candidates} candidates)",
        spread(&exact),
        rate(&exact),
        spread(&scanned),
        rate(&scanned),
    );

    // 5. The pieces.
    let whole_list = pdf_model::interpret(&document, &page).display_list;
    for &count in &counts {
        pieces(
            &bytes,
            &document,
            &page,
            encoded,
            &content,
            &whole_list,
            count,
            threads,
            rounds,
        );
    }

    // 6. The drawer.
    let size = pdf_model::content::displayed_size(&page);
    let scale = scale.unwrap_or_else(|| (1600.0 / size.width).min(1000.0 / size.height));
    let target = pdf_render::TargetSpec::for_page(&DisplayList::new(size), scale, 1 << 31)
        .expect("a target");
    let mut taking = Vec::new();
    let mut finishing = Vec::new();
    let mut handed = 0;
    for _ in 0..rounds {
        let parts_of = Parts::new();
        let (list, parts) = std::thread::scope(|scope| {
            let collector = {
                let parts_of = parts_of.clone();
                scope.spawn(move || {
                    let mut parts = Vec::new();
                    while let Some(Handed::Part(part)) = parts_of.take(Duration::from_mins(1)) {
                        parts.push(part);
                    }
                    parts
                })
            };
            let list = parts_of
                .during(|| pdf_model::interpret(&document, &page))
                .display_list;
            (list, collector.join().expect("the collector"))
        });
        handed = parts.len();
        let started = Instant::now();
        let mut underway = render_cpu::Underway::new(&render_cpu::CpuRasterizer::new(), target)
            .expect("a target the processor can draw");
        for part in parts {
            assert!(underway.take(part), "the witness is drawn ahead");
        }
        taking.push(started.elapsed());
        let started = Instant::now();
        underway.finish(&list).expect("the page is finished");
        finishing.push(started.elapsed());
    }
    println!(
        "drawer, one thread, {}×{} px: {handed} parts painted {}; finished {}",
        target.width,
        target.height,
        spread(&taking),
        spread(&finishing),
    );
}

/// The pieces of one division, interpreted after the inflate and behind it.
#[expect(
    clippy::too_many_arguments,
    reason = "one call site, and each argument is one of the page's facts"
)]
fn pieces(
    bytes: &[u8],
    document: &Document,
    page: &pdf_model::Page,
    encoded: &[u8],
    content: &[u8],
    whole: &DisplayList,
    count: usize,
    threads: usize,
    rounds: usize,
) {
    let plan = Plan::of(content, count);
    let documents: Vec<Document> = plan
        .pieces(content)
        .iter()
        .map(|piece| Document::open(with_contents(bytes, document, page, piece)).expect("opens"))
        .collect();
    // The last cut is the window's length, a separator past what the decoder produces.
    let inflated = content.len() - 1;
    let ends: Vec<usize> = plan.cuts[1..]
        .iter()
        .map(|&end| end.min(inflated))
        .collect();

    let mut serial = Vec::new();
    let mut pipelined = Vec::new();
    let mut same = None;
    for round in 0..rounds {
        for behind in [false, true] {
            let schedule = run(encoded, inflated, &documents, &ends, threads, behind);
            if round == 0 && !behind {
                same = Some(join(schedule.lists) == *whole);
            }
            let summary = Summary {
                inflated: schedule.inflated,
                done: schedule.done,
            };
            if behind {
                pipelined.push(summary);
            } else {
                serial.push(summary);
            }
        }
    }
    for (name, runs) in [
        ("after the inflate", &serial),
        ("behind the inflate", &pipelined),
    ] {
        let best = runs
            .iter()
            .min_by_key(|summary| summary.end())
            .expect("a round");
        let ends: Vec<Duration> = runs.iter().map(Summary::end).collect();
        println!(
            "{count} pieces on {threads} threads, {name}: the read {} (best: inflated at {}, \
             first piece at {}, in order a quarter {}, half {}, three quarters {}, all {})",
            spread(&ends),
            ms(best.inflated),
            ms(best.done[0]),
            ms(best.in_order(count / 4)),
            ms(best.in_order(count / 2)),
            ms(best.in_order(count * 3 / 4)),
            ms(best.end()),
        );
    }
    println!(
        "{count} pieces: the joined list {} the page's",
        if same == Some(true) { "is" } else { "IS NOT" }
    );
}

/// What one run of the pieces timed, from its starting line.
struct Summary {
    inflated: Duration,
    done: Vec<Duration>,
}

impl Summary {
    fn end(&self) -> Duration {
        self.done.iter().copied().max().unwrap_or_default()
    }

    /// When the first `count` pieces, in order, were all done.
    fn in_order(&self, count: usize) -> Duration {
        self.done[..count.max(1)]
            .iter()
            .copied()
            .max()
            .unwrap_or_default()
    }
}

/// One run's answer: when the inflate ended, when each piece was done, and the pieces' lists.
struct Schedule {
    inflated: Duration,
    done: Vec<Duration>,
    lists: Vec<DisplayList>,
}

/// Inflates on one thread and interprets the pieces on `threads` others, taken in order; with
/// `behind`, a piece starts once the inflate has passed its end, and otherwise once it has ended.
fn run(
    encoded: &[u8],
    length: usize,
    documents: &[Document],
    ends: &[usize],
    threads: usize,
    behind: bool,
) -> Schedule {
    let progress = AtomicUsize::new(0);
    let signal = Signal::new();
    let next = AtomicUsize::new(0);
    let line = Barrier::new(threads + 2);
    let done: Vec<Mutex<Option<(Duration, DisplayList)>>> =
        documents.iter().map(|_| Mutex::new(None)).collect();
    let started = std::thread::scope(|scope| {
        let producer = scope.spawn(|| {
            line.wait();
            let began = Instant::now();
            let out = inflate_chunked(encoded, length, &progress, &signal);
            let inflated = began.elapsed();
            std::hint::black_box(out.len());
            inflated
        });
        for _ in 0..threads {
            scope.spawn(|| {
                line.wait();
                let began = Instant::now();
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(piece) = documents.get(index) else {
                        break;
                    };
                    let needed = if behind { ends[index] } else { length };
                    signal.wait_for(&progress, needed);
                    let pages = pdf_model::Pages::new(piece);
                    let page = pages.get(0).expect("the piece's page");
                    let list = pdf_model::interpret(piece, &page).display_list;
                    *done[index].lock().expect("unpoisoned") = Some((began.elapsed(), list));
                }
            });
        }
        line.wait();
        producer.join().expect("the producer")
    });
    let mut times = Vec::new();
    let mut lists = Vec::new();
    for slot in done {
        let (time, list) = slot
            .into_inner()
            .expect("unpoisoned")
            .expect("every piece was interpreted");
        times.push(time);
        lists.push(list);
    }
    Schedule {
        inflated: started,
        done: times,
        lists,
    }
}

/// A producer's progress, waited on by its readers.
struct Signal {
    lock: Mutex<()>,
    moved: Condvar,
}

impl Signal {
    fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            moved: Condvar::new(),
        }
    }

    fn publish(&self, progress: &AtomicUsize, to: usize) {
        let _held = self.lock.lock().expect("unpoisoned");
        progress.store(to, Ordering::Release);
        self.moved.notify_all();
    }

    fn wait_for(&self, progress: &AtomicUsize, at_least: usize) {
        let mut held = self.lock.lock().expect("unpoisoned");
        while progress.load(Ordering::Acquire) < at_least {
            held = self.moved.wait(held).expect("unpoisoned");
        }
    }
}

/// The stream inflated whole into one buffer of its known length: the decoder's floor.
fn inflate_whole(encoded: &[u8], length: usize) -> Vec<u8> {
    let mut decoder = flate2::Decompress::new(true);
    let mut out = Vec::with_capacity(length + 1);
    let status = decoder
        .decompress_vec(encoded, &mut out, flate2::FlushDecompress::Finish)
        .expect("the witness inflates");
    assert_eq!(status, flate2::Status::StreamEnd, "the stream ends");
    out
}

/// The stream inflated into one buffer [`CHUNK`] bytes at a time, each step published.
fn inflate_chunked(
    encoded: &[u8],
    length: usize,
    progress: &AtomicUsize,
    signal: &Signal,
) -> Vec<u8> {
    let mut decoder = flate2::Decompress::new(true);
    let mut out = vec![0u8; length];
    let mut filled = 0;
    while filled < length {
        let taken = decoder.total_in() as usize;
        let before = decoder.total_out() as usize;
        let until = (filled + CHUNK).min(length);
        decoder
            .decompress(
                &encoded[taken..],
                &mut out[filled..until],
                flate2::FlushDecompress::None,
            )
            .expect("the witness inflates");
        let produced = decoder.total_out() as usize - before;
        assert!(produced > 0, "the decoder moves");
        filled += produced;
        signal.publish(progress, filled);
    }
    out
}

/// Counts a stream's tokens as the page's reader lexes them.
fn count_tokens(bytes: &[u8]) -> usize {
    let mut reader = ContentReader::over(bytes);
    let mut tokens = 0usize;
    while reader.with_token(|token| token.is_some()) {
        tokens += 1;
    }
    tokens
}

/// How many `Q`s stand between §7.2.3's delimiters, by bytes alone: what a planner that guesses
/// its cuts and confirms them afterwards reads before it lexes anything.
fn scan_for_restores(bytes: &[u8]) -> usize {
    let delimiter = |byte: u8| matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' | b'\0');
    bytes
        .windows(3)
        .filter(|window| window[1] == b'Q' && delimiter(window[0]) && delimiter(window[2]))
        .count()
}

/// Where the stream is cut, and the state each piece after the first starts in — ADR 1873's
/// instrument's plan, restated.
struct Plan {
    /// Byte offsets: zero, each cut, and the stream's length.
    cuts: Vec<usize>,
    /// For each piece, what restates the state at its start; empty for the first.
    prefixes: Vec<Vec<u8>>,
}

/// The operators at the page's own depth whose effect lasts past themselves, as the scan last
/// saw them, each kept as the bytes that state it.
#[derive(Default, Clone)]
struct Standing<'a> {
    transforms: Vec<&'a [u8]>,
    width: Option<&'a [u8]>,
    cap: Option<&'a [u8]>,
    join: Option<&'a [u8]>,
    mitre: Option<&'a [u8]>,
    dash: Option<&'a [u8]>,
    stroke: Vec<&'a [u8]>,
    fill: Vec<&'a [u8]>,
}

impl Standing<'_> {
    fn prefix(&self) -> Vec<u8> {
        let mut prefix = b"q\n".to_vec();
        let lasting = [self.width, self.cap, self.join, self.mitre, self.dash];
        for statement in self
            .transforms
            .iter()
            .copied()
            .chain(lasting.into_iter().flatten())
            .chain(self.stroke.iter().copied())
            .chain(self.fill.iter().copied())
        {
            prefix.extend_from_slice(statement);
            prefix.push(b'\n');
        }
        prefix
    }
}

impl<'a> Standing<'a> {
    fn take(&mut self, operator: &[u8], statement: &'a [u8]) {
        match operator {
            b"cm" => self.transforms.push(statement),
            b"w" => self.width = Some(statement),
            b"J" => self.cap = Some(statement),
            b"j" => self.join = Some(statement),
            b"M" => self.mitre = Some(statement),
            b"d" => self.dash = Some(statement),
            // §8.6.8: setting a colour space sets its initial colour, so the last space and the
            // colours set after it are the whole of the colour in force.
            b"CS" | b"G" | b"RG" | b"K" => self.stroke = vec![statement],
            b"SC" | b"SCN" => self.stroke.push(statement),
            b"cs" | b"g" | b"rg" | b"k" => self.fill = vec![statement],
            b"sc" | b"scn" => self.fill.push(statement),
            // A cut is only ever made after a `Q`, where no path is open (§8.5.2.1).
            b"m" | b"l" | b"c" | b"v" | b"y" | b"h" | b"re" | b"S" | b"s" | b"f" | b"F" | b"f*"
            | b"B" | b"B*" | b"b" | b"b*" | b"n" => {}
            other => panic!(
                "{} at the page's own depth: this instrument does not restate it",
                String::from_utf8_lossy(other)
            ),
        }
    }
}

impl Plan {
    /// Lexes `content` once and cuts it into `count` pieces of about equal length, each cut
    /// directly after a `Q` that returns to depth one.
    fn of(content: &[u8], count: usize) -> Self {
        let mut lexer = Lexer::new(content);
        let mut depth = 0usize;
        let mut marked = 0usize;
        let mut started: Option<usize> = None;
        let mut standing = Standing::default();
        let mut cuts = vec![0];
        let mut prefixes = vec![Vec::new()];
        loop {
            lexer.skip_whitespace();
            let before = lexer.position();
            let Some(token) = lexer.next_token() else {
                break;
            };
            let Token::Keyword(operator) = token else {
                started.get_or_insert(before);
                continue;
            };
            let statement = &content[started.take().unwrap_or(before)..lexer.position()];
            match operator {
                b"BI" => panic!("an inline image: this scan does not step over its data"),
                b"BMC" | b"BDC" => marked += 1,
                b"EMC" => marked = marked.saturating_sub(1),
                b"q" => depth += 1,
                b"Q" => {
                    depth = depth.checked_sub(1).expect("a Q with no q");
                    let target = content.len() * cuts.len() / count;
                    if depth == 1 && marked == 0 && cuts.len() < count && lexer.position() >= target
                    {
                        cuts.push(lexer.position());
                        prefixes.push(standing.prefix());
                    }
                }
                _ if depth == 1 => standing.take(operator, statement),
                _ if depth == 0 => panic!(
                    "{} outside the page's outermost q",
                    String::from_utf8_lossy(operator)
                ),
                _ => {}
            }
        }
        assert_eq!(
            cuts.len(),
            count,
            "the stream has fewer places to cut than pieces asked"
        );
        cuts.push(content.len());
        Self { cuts, prefixes }
    }

    /// Each piece as a content stream of its own: its prefix, its bytes, and a `Q` closing the
    /// prefix's `q` — except the last, whose bytes end with the `Q` that closes the page's own.
    fn pieces(&self, content: &[u8]) -> Vec<Vec<u8>> {
        let last = self.cuts.len() - 2;
        self.cuts
            .windows(2)
            .enumerate()
            .map(|(index, cut)| {
                let mut piece = self.prefixes[index].clone();
                piece.extend_from_slice(&content[cut[0]..cut[1]]);
                if index != last {
                    piece.extend_from_slice(b"\nQ\n");
                }
                piece
            })
            .collect()
    }
}

/// The document's bytes with an incremental update (§7.5.6) whose page is `page` with `piece`,
/// unfiltered, as its `/Contents`.
fn with_contents(
    bytes: &[u8],
    document: &Document,
    page: &pdf_model::Page,
    piece: &[u8],
) -> Vec<u8> {
    let id = page.id.expect("the page is an indirect object");
    let contents = page
        .dict
        .get("Contents")
        .and_then(pdf_syntax::Object::as_reference)
        .expect("one /Contents stream, named by reference");
    let header = format!("\n{} {} obj", id.number, id.generation);
    let start = find(bytes, header.as_bytes()).expect("the page object as written") + 1;
    let end = start + find(&bytes[start..], b"endobj").expect("its end") + b"endobj".len();
    let named = format!("/Contents {} {} R", contents.number, contents.generation);
    let tail = String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(1024)..]).into_owned();
    let size: u32 = number_after(&tail, "/Size ").expect("a trailer /Size");
    let root = tail[tail.rfind("/Root ").expect("a trailer /Root") + "/Root ".len()..]
        .split(" R")
        .next()
        .expect("a reference")
        .to_owned();
    let previous: usize = number_after(&tail, "startxref").expect("a startxref");
    assert!(
        document.trailer().get("Encrypt").is_none(),
        "an unencrypted file"
    );

    let mut out = bytes.to_vec();
    out.push(b'\n');
    let stream_at = out.len();
    out.extend_from_slice(
        format!("{size} 0 obj\n<< /Length {} >>\nstream\n", piece.len()).as_bytes(),
    );
    out.extend_from_slice(piece);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    let page_at = out.len();
    let written = String::from_utf8_lossy(&bytes[start..end]).into_owned();
    assert!(
        written.contains(&named),
        "the page names its stream as {named}"
    );
    out.extend_from_slice(
        written
            .replace(&named, &format!("/Contents {size} 0 R"))
            .as_bytes(),
    );
    out.push(b'\n');
    let xref_at = out.len();
    out.extend_from_slice(
        format!(
            "xref\n0 1\n0000000000 65535 f \n{} 1\n{page_at:010} {:05} n \n{size} 1\n\
             {stream_at:010} 00000 n \ntrailer\n<< /Size {} /Root {root} R /Prev {previous} >>\n\
             startxref\n{xref_at}\n%%EOF\n",
            id.number,
            id.generation,
            size + 1
        )
        .as_bytes(),
    );
    out
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn number_after<T: std::str::FromStr>(text: &str, key: &str) -> Option<T> {
    text[text.rfind(key)? + key.len()..]
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// The lists joined in order into the first, each later clip table renumbered into its table.
fn join(lists: Vec<DisplayList>) -> DisplayList {
    let mut lists = lists.into_iter();
    let mut joined = lists.next().expect("at least one piece");
    for mut list in lists {
        assert_eq!(
            list.soft_mask_count(),
            0,
            "a piece with soft masks is not joined here"
        );
        let mut renumbered = Vec::with_capacity(list.clip_count());
        for index in 0..list.clip_count() {
            let mut clip = list
                .clip(ClipId::new(index as u32))
                .expect("a clip in its table")
                .clone();
            clip.parent = clip.parent.map(|parent| renumbered[parent.index()]);
            renumbered.push(joined.add_clip(clip).expect("the joined table has room"));
        }
        for mut command in list.split_off_commands(0) {
            renumber(&mut command, &renumbered);
            joined.push(command);
        }
    }
    joined
}

fn renumber(command: &mut Command, renumbered: &[ClipId]) {
    if let Some(clip) = command.clip() {
        command.set_clip(Some(renumbered[clip.index()]));
    }
    if let Command::Group { commands, .. } = command {
        for inner in commands {
            renumber(inner, renumbered);
        }
    }
}

fn fastest(times: &[Duration]) -> Duration {
    times.iter().copied().min().unwrap_or_default()
}

fn ms(time: Duration) -> String {
    format!("{:.0} ms", time.as_secs_f64() * 1e3)
}

/// The fastest and slowest of a run of times.
fn spread(times: &[Duration]) -> String {
    let slowest = times.iter().copied().max().unwrap_or_default();
    format!(
        "{:.0}–{:.0} ms",
        fastest(times).as_secs_f64() * 1e3,
        slowest.as_secs_f64() * 1e3
    )
}
