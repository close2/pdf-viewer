//! What one page's read costs divided among threads, priced two ways (ADR 1873).
//!
//! The read — `pdf_model::interpret` over a page's `/Contents` — is one thread's work today, and
//! on a drawing of millions of operators it is most of the wait before the first frame (ADR 1845).
//! Two divisions of it are priced here, each by one measurement on the same page in one process:
//!
//! - **The tokens on many threads, the operators on one.** The stream is cut at points a scan
//!   found, each piece is lexed on a thread of its own, and one thread runs the operators in order.
//!   What that can win at most is the lexing itself, so this prints the read with the lexing taken
//!   out: ADR 1833's stage method, `with_token` over the page's own reader less `read_to_end`, on
//!   the clock, beside the pieces lexed at once.
//! - **The operators on many threads too.** §8.4.2's `Q` restores the graphics state the matching
//!   `q` saved, so at a point where the stream has returned to a nesting depth it has been at
//!   before, the state is the one in force there, changed only by the operators stated at that
//!   depth since. A piece cut after a `Q` that returns to the depth holding the page's own state
//!   is therefore interpretable on its own, given a prefix that states that state again — the
//!   depth's `cm`s in order, its last line width and cap, and its last colour space with the
//!   colour set after it. Each piece is made a page of its own (the document plus an incremental
//!   update, §7.5.6, whose page names the piece as its `/Contents`, unfiltered), the pieces are
//!   interpreted at once, and their lists are joined, each clip table renumbered into the first.
//!   The joined list is compared with the page's own list by `DisplayList`'s equality, which is
//!   what says that the division is the page and not a picture of it.
//!
//! The planning scan is exact and serial here and is not part of either price: it lexes the whole
//! stream, which a build would do on the pieces at once — the pieces lexed at once, printed beside,
//! is what that pass costs. It refuses loudly any operator at the page's own depth whose state it
//! does not restate (a clip, a text state, an extended graphics state), an inline image, or a
//! marked-content sequence open across a cut: an instrument for the drawings it was written for,
//! not a reader of every stream.
//!
//! ```sh
//! taskset -c <cores> cargo run --release -p pdf-model --example read_split_price -- \
//!     /tmp/demo_heavy_engineering_A1.pdf [rounds] [piece counts, e.g. 4,8, or "" for the build alone]
//! ```
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "a measurement tool: it stops loudly on an input it was not written for, its \
              arithmetic is on byte offsets and counts far inside every type it uses, and \
              printing is the whole point"
)]

use std::fmt::Write as _;
use std::sync::Barrier;
use std::time::{Duration, Instant};

use pdf_model::content::reader::ContentReader;
use pdf_render::display_list::{ClipId, Command, DisplayList};
use pdf_syntax::{Document, Lexer, Token};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document to read");
    let rounds: usize = arguments
        .next()
        .map_or(3, |n| n.parse().expect("a count of rounds"));
    let counts: Vec<usize> = arguments.next().map_or_else(
        || vec![4, 8],
        |list| {
            // An empty list prices the build alone.
            list.split(',')
                .filter(|n| !n.is_empty())
                .map(|n| n.parse().expect("a piece count"))
                .collect()
        },
    );

    let bytes = std::fs::read(&path).expect("the document is readable");
    let document = Document::open(bytes.clone()).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("a first page");

    let content = ContentReader::for_page(&document, &page).read_to_end();
    let mut whole_list = None;
    let mut whole = Vec::new();
    let mut inflate = Vec::new();
    let mut lexed = Vec::new();
    // The serial arms, interleaved round by round so that a drift in the machine's clock lands
    // on each alike.
    for _ in 0..rounds {
        let started = Instant::now();
        let interpretation = pdf_model::content::divide::Division::NEVER
            .during(|| pdf_model::interpret(&document, &page));
        whole.push(started.elapsed());
        whole_list = Some(interpretation);

        let started = Instant::now();
        let length = ContentReader::for_page(&document, &page)
            .read_to_end()
            .len();
        inflate.push(started.elapsed());
        assert_eq!(length, content.len(), "the stream reads the same twice");

        let started = Instant::now();
        let mut reader = ContentReader::for_page(&document, &page);
        let mut tokens = 0usize;
        while reader.with_token(|token| token.is_some()) {
            tokens += 1;
        }
        lexed.push(started.elapsed());
        std::hint::black_box(tokens);
    }
    let whole_list = whole_list.expect("at least one round");
    let tokens = count_tokens(&content);
    println!(
        "page: {} bytes inflated, {tokens} tokens, {} commands of {} bytes each, {} clips",
        content.len(),
        whole_list.display_list.command_count(),
        size_of::<Command>(),
        whole_list.display_list.clip_count()
    );
    let lexing_alone = fastest(&lexed).saturating_sub(fastest(&inflate));
    println!(
        "serial: read {} (inflate {}, lexing alone {}); the read less its lexing {}",
        spread(&whole),
        spread(&inflate),
        ms(lexing_alone),
        ms(fastest(&whole).saturating_sub(lexing_alone)),
    );

    built(&document, &page, &whole_list, rounds);

    for &count in &counts {
        price(
            &bytes,
            &document,
            &page,
            &content,
            &whole_list,
            count,
            rounds,
        );
    }
}

/// The build (ADRs 1880, 1894): the page read as `pdf_model::interpret` reads it — divided with
/// the inflate pipelined into the pieces — against the same page divided once the whole stream is
/// inflated (ADR 1880's read) and read whole, interleaved, and each list compared with the whole.
fn built(
    document: &Document,
    page: &pdf_model::Page,
    whole_list: &pdf_model::Interpretation,
    rounds: usize,
) {
    use pdf_model::content::divide::{Division, Reading, last_pipeline_decline, last_reading};
    let mut pipelined = Vec::new();
    let mut after = Vec::new();
    let mut undivided = Vec::new();
    let mut same = true;
    let mut how = None;
    let mut fell_back = None;
    let same_as_whole = |interpretation: &pdf_model::Interpretation| {
        interpretation.display_list == whole_list.display_list
            && interpretation.unsupported == whole_list.unsupported
            && interpretation.operators == whole_list.operators
    };
    for _ in 0..rounds {
        let started = Instant::now();
        let interpretation = pdf_model::interpret(document, page);
        pipelined.push(started.elapsed());
        how = last_reading();
        fell_back = last_pipeline_decline();
        same &= same_as_whole(&interpretation);
        drop(interpretation);
        let started = Instant::now();
        let interpretation =
            Division::AFTER_THE_INFLATE.during(|| pdf_model::interpret(document, page));
        after.push(started.elapsed());
        same &= same_as_whole(&interpretation);
        drop(interpretation);
        let started = Instant::now();
        let interpretation = Division::NEVER.during(|| pdf_model::interpret(document, page));
        undivided.push(started.elapsed());
        std::hint::black_box(interpretation);
    }
    println!(
        "built: read pipelined {} against divided after the inflate {} and whole {} on {} threads \
         ({}{}); its lists {} the page's",
        spread(&pipelined),
        spread(&after),
        spread(&undivided),
        rayon::current_num_threads(),
        match how {
            Some(Reading::Divided) => "divided".to_owned(),
            Some(Reading::Whole(reason)) => format!("not divided: {}", reason.0),
            None => "no reading".to_owned(),
        },
        fell_back.map_or_else(String::new, |reason| format!(
            "; not pipelined: {}",
            reason.0
        )),
        if same { "are" } else { "ARE NOT" },
    );
}

/// Prices one division of the stream into `count` pieces.
fn price(
    bytes: &[u8],
    document: &Document,
    page: &pdf_model::Page,
    content: &[u8],
    whole: &pdf_model::Interpretation,
    count: usize,
    rounds: usize,
) {
    let started = Instant::now();
    let plan = Plan::of(content, count);
    let planned = started.elapsed();
    let documents: Vec<Document> = plan
        .pieces(content)
        .iter()
        .map(|piece| Document::open(with_contents(bytes, document, page, piece)).expect("opens"))
        .collect();

    let mut lexed = Vec::new();
    let mut walls = Vec::new();
    let mut slowest_piece = Vec::new();
    let mut joins = Vec::new();
    let mut same = true;
    for _ in 0..rounds {
        // The pieces lexed at once: what a pass that finds each piece's depth and state costs.
        let slices = plan.slices(content);
        let (wall, _) = at_once(&slices, |slice| count_tokens(slice));
        lexed.push(wall);

        let (wall, interpreted) = at_once(&documents, |piece| {
            let pages = pdf_model::Pages::new(piece);
            let page = pages.get(0).expect("the piece's page");
            pdf_model::interpret(piece, &page)
        });
        walls.push(wall);
        slowest_piece.push(
            interpreted
                .iter()
                .map(|(elapsed, _)| *elapsed)
                .max()
                .unwrap_or_default(),
        );
        let lists: Vec<DisplayList> = interpreted
            .into_iter()
            .map(|(_, interpretation)| interpretation.display_list)
            .collect();
        let started = Instant::now();
        let joined = join(lists);
        joins.push(started.elapsed());
        same &= joined == whole.display_list;
    }

    let mut sizes = String::new();
    for window in plan.cuts.windows(2) {
        let _ = write!(sizes, " {:.1}", (window[1] - window[0]) as f64 / 1e6);
    }
    println!(
        "{count} pieces (planned in {}, MB:{sizes}): lexed at once {}; interpreted at once {} \
         (slowest piece {}); joined {}; the joined list {} the page's",
        ms(planned),
        spread(&lexed),
        spread(&walls),
        spread(&slowest_piece),
        spread(&joins),
        if same { "is" } else { "IS NOT" },
    );
}

/// Runs `work` on every item at once, one thread each, from one starting line; returns the wall
/// from the line to the last finish, and each item's own time and answer.
fn at_once<I: Sync, T: Send>(
    items: &[I],
    work: impl Fn(&I) -> T + Sync,
) -> (Duration, Vec<(Duration, T)>) {
    let line = Barrier::new(items.len() + 1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .iter()
            .map(|item| {
                let (line, work) = (&line, &work);
                scope.spawn(move || {
                    line.wait();
                    let started = Instant::now();
                    let answer = work(item);
                    (started.elapsed(), answer, Instant::now())
                })
            })
            .collect();
        line.wait();
        let started = Instant::now();
        let mut answers = Vec::new();
        let mut last = started;
        for handle in handles {
            let (elapsed, answer, finished) = handle.join().expect("a piece's thread finishes");
            last = last.max(finished);
            answers.push((elapsed, answer));
        }
        (last.duration_since(started), answers)
    })
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

/// Where the stream is cut, and the state each piece after the first starts in.
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

impl Plan {
    /// Lexes `content` once and cuts it into `count` pieces of about equal length, each cut
    /// directly after a `Q` that returns to depth one — inside the page's outermost `q`, where its
    /// own state is stated.
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

    /// The pieces as bytes, without their prefixes.
    fn slices<'a>(&self, content: &'a [u8]) -> Vec<&'a [u8]> {
        self.cuts
            .windows(2)
            .map(|cut| &content[cut[0]..cut[1]])
            .collect()
    }

    /// Each piece as a content stream of its own: its prefix, its bytes, and a `Q` closing the
    /// prefix's `q` — except the last, whose bytes end with the `Q` that closes the page's own.
    fn pieces(&self, content: &[u8]) -> Vec<Vec<u8>> {
        let slices = self.slices(content);
        let last = slices.len() - 1;
        slices
            .into_iter()
            .enumerate()
            .map(|(index, slice)| {
                let mut piece = self.prefixes[index].clone();
                piece.extend_from_slice(slice);
                if index != last {
                    piece.extend_from_slice(b"\nQ\n");
                }
                piece
            })
            .collect()
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
            // Path construction and painting leave no state behind once the path is painted, and
            // a cut is only ever made after a `Q`, where no path is open (§8.5.2.1).
            b"m" | b"l" | b"c" | b"v" | b"y" | b"h" | b"re" | b"S" | b"s" | b"f" | b"F" | b"f*"
            | b"B" | b"B*" | b"b" | b"b*" | b"n" => {}
            other => panic!(
                "{} at the page's own depth: this instrument does not restate it",
                String::from_utf8_lossy(other)
            ),
        }
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

/// The lists joined in order into the first, each later clip table renumbered into its table
/// (`add_clip` keeps a region once, as the interpreter's own list does).
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
