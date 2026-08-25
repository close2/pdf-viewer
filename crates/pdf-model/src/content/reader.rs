//! The bytes of a content stream, delivered a window at a time.
//!
//! ISO 32000-2 §7.8.2 describes a content stream as "a sequence of instructions describing the
//! graphical elements to be painted on a page", and §7.7.3.3's Table 31 says how the parts of
//! an array of them fit together, in the `/Contents` row:
//!
//! > The division between streams may occur only at the boundaries between lexical tokens (see
//! > 7.2, "Lexical conventions" ) but shall be unrelated to the page's logical content or
//! > organisation.
//!
//! That sentence is what this module is built on, and it is Table 31's rather than §7.8.2's —
//! which `doc/todo/14` had wrong for nine sessions and the quotation gate caught here. Because
//! a part may only end where a token ends, several parts *chain into one reader* rather than
//! being concatenated into one buffer — and because the interpreter reads a content stream
//! once, forwards, and never seeks back, the buffer it reads through need not be larger than
//! the largest token it must hold.
//!
//! # Why a window rather than the whole thing
//!
//! `doc/todo/14` and ADR 0362 are the argument and the measurement. In short: decoding a
//! content stream whole makes a *decompression bomb* an allocation nothing can take back —
//! 1.85 MB of file commanding a gibibyte of resident memory — while a window turns it into
//! time, which the operator bound already stops. The same window takes an honest 141 MiB
//! drawing from 380 MB of peak resident memory to 98 MB, and reads it to the same token.
//!
//! # The two numbers, and where they come from
//!
//! Neither is invented. `examples/token_window_census` measured 225 775 555 content-stream
//! tokens in 39 976 documents: the largest single token in the whole population is 390.16 KiB,
//! **233 pass 4 KiB, 2 pass 64 KiB and none passes 1 MiB**. So [`WINDOW`] is 64 KiB — the size
//! at which refilling stops mattering (2 258 refills and 7 694 re-lexed bytes over 141 MiB,
//! against 36 156 and 120 367 at 4 KiB, at the same peak) — and [`CEILING`] is 1 MiB, above
//! every token any real document has been seen to state.
//!
//! # The two exceptional cases are loud
//!
//! A token longer than [`CEILING`] and an inline image whose data does not fit the lookahead
//! are the two things a bounded buffer cannot do. Neither is silently truncated: the first is
//! [`crate::page::ContentIssue::TokenTooLong`] and the second
//! [`crate::inline_image::InlineImageError::Unbuffered`], and both reach the page's report the
//! way every other refusal does. ADR 0306's lesson is that a clamp which says nothing is worse
//! than a refusal that does.

use std::sync::Arc;

use pdf_syntax::{
    Damage, Document, Lexer, Object, Pumped, Pumping, Stream, StreamRefusal, StreamSource, Token,
};

use crate::page::{ContentIssue, Page, filter_names};

/// The window a content stream is read through, in bytes.
///
/// See the module documentation for the census this comes from: 2 of 225 775 555 measured
/// tokens are longer than this, and the buffer grows to [`CEILING`] for those.
pub const WINDOW: usize = 64 * 1024;

/// How far the window grows to hold one token, in bytes.
///
/// The census's own upper bound: no token in 39 976 documents passes this, and the largest
/// anywhere is 390.16 KiB. A token past it is reported rather than cut — see
/// [`crate::page::ContentIssue::TokenTooLong`].
pub const CEILING: usize = 1024 * 1024;

/// How far past the cursor §8.9.7's inline images may be buffered, in bytes.
///
/// An inline image is the one thing in a content stream that is not a token, and the only one
/// whose extent a window cannot know before reading it. §8.9.7 says what an inline image is
/// for, twice, and both sentences are advice rather than a requirement:
///
/// > Because the inline format gives the PDF processor less flexibility in managing the image
/// > data, it should be used only for small images (4096 bytes or less).
///
/// > The value of the Length key should not exceed 4096 bytes.
///
/// A `should` binds nobody, so this is not that number: it is **sixteen mebibytes, which is
/// 1.78 times the largest inline image in the 93 930 that `examples/token_window_census`
/// measured** (9.01 MiB) and four thousand times what the clause recommends. Past it the image
/// is refused by name — [`crate::inline_image::InlineImageError::Unbuffered`] — rather than
/// read short, and the page carries on.
pub const LOOKAHEAD: usize = 16 * 1024 * 1024;

/// How few bytes may stand between the cursor and the end of the buffer before it is refilled.
///
/// The buffer is compacted and refilled when what is left ahead of the cursor falls below
/// this, which is what makes the refill test one comparison per token rather than a second
/// pass over it. A token longer than this is re-lexed once after the refill, and the census
/// says that is 233 tokens in 225 775 555.
const SLACK: usize = 4 * 1024;

/// A keyword's bytes, held inline where they fit.
///
/// [`Token::Keyword`] *borrows* its bytes from the buffer it was lexed out of, which is what
/// removed a heap allocation per token in ADR 0341 — and under a moving window those bytes
/// stop existing at the next refill. [`ContentReader::with_token`] is what makes that safe:
/// the token cannot leave the closure, so the compiler enforces the rule rather than a
/// comment. What the interpreter needs *past* that point is the operator, and this is where it
/// goes: a memcpy of at most fifteen bytes onto the stack, not an allocation, so ADR 0341's
/// finding is kept rather than undone.
///
/// Fifteen bytes is more than any operator Annex A states — the longest are three characters
/// — and more than §7.3.2's `true` and `false`, which a content stream lexes as keywords. A
/// longer run is a malformed stream's, it is never an operator, and it is kept whole because
/// the report that names it prints what the file wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Word {
    /// A keyword that fits inline: the bytes, and how many of them are the keyword.
    Short([u8; 15], u8),
    /// A run too long to hold inline, kept because a report quotes it.
    Long(Vec<u8>),
}

impl Word {
    /// Copies `word`, inline where it fits.
    #[inline]
    #[must_use]
    pub fn new(word: &[u8]) -> Self {
        let mut inline = [0u8; 15];
        match (inline.get_mut(..word.len()), u8::try_from(word.len())) {
            (Some(room), Ok(len)) => {
                room.copy_from_slice(word);
                Self::Short(inline, len)
            }
            _ => Self::Long(word.to_vec()),
        }
    }

    /// The bytes the file wrote.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Short(bytes, len) => bytes.get(..usize::from(*len)).unwrap_or_default(),
            Self::Long(bytes) => bytes,
        }
    }
}

/// One of §7.8.2's other four content streams, ready to be read — once, or several times.
///
/// > Content streams shall also be used to package sequences of instructions as self-contained
/// > graphical elements, such as forms (see 8.10, "Form XObjects"), patterns (8.7, "Patterns"),
/// > certain fonts (9.6.4, "Type 3 fonts"), and annotation appearances (12.5.5, "Appearance
/// > streams").
///
/// **This is a source rather than a buffer, and that is the whole of ADR 0427.** Each of those
/// four is read more than once — §11.6.6's paired runs interpret the same form two and three
/// times, a glyph description once per character, an appearance once per drawing (a tiling
/// pattern's cell is read *once* since ADR 0430, which is what let it join the other three) —
/// so a reader is *made* for each run instead of a decode being handed round. Which of the two
/// shapes it makes is [`Document::nested_content_source`]'s decision and is the decoded-stream
/// memo's own condition: a stream the memo keeps is held here whole, and one it declines is
/// held here as its *encoded* bytes and pumped through a window, so that a bomb inside a form
/// costs the window rather than the gibibyte it used to.
#[derive(Debug, Clone)]
pub struct NestedContent {
    /// Which of §7.8.2's kinds this is and how the page reached it, for the report.
    detail: String,
    /// Where the bytes come from.
    source: Nested,
}

/// The three shapes of [`NestedContent`]. See there.
#[derive(Debug, Clone)]
enum Nested {
    /// Decoded whole, and held by the document's memo, so every read after the first is a
    /// cache read exactly as it was before ADR 0427.
    Whole {
        /// The decoded bytes.
        data: Arc<[u8]>,
        /// Why the decode stopped short, where it did (ADR 0343).
        damage: Option<Damage>,
    },
    /// Still encoded, decoded through a window once per read.
    Windowed {
        /// The encoded bytes, which is what the document already holds: a [`pdf_syntax::Pump`]
        /// takes its input from the stream object rather than copying it.
        data: Arc<[u8]>,
        /// Which filter a fresh pump is to run, decided once by the document rather than
        /// re-derived per read.
        pumping: Pumping,
        /// The `/Filter` names it declared, for the report.
        filters: Vec<String>,
        /// `Limits::max_stream_len`, which still bounds this one stream.
        limit: usize,
        /// Which decode the pump is running, so that a run reaching `limit` without producing
        /// one operation can say so to the document. See [`super::Interpreter::run`].
        decoding: pdf_syntax::Decoding,
    },
    /// A window has read this decode to the document's own bound before and found nothing in
    /// it, so this read raises that report and inflates nothing.
    ///
    /// **The same report and the same absence of marks as the read that learnt it**, which is
    /// the condition [`pdf_syntax::Document::window_found_nothing`] only records under: a page
    /// whose marks depended on whether the memo still held an entry would be a page drawn by
    /// the cache.
    Refused {
        /// The bound the window reached, which [`crate::page::ContentIssue::TooLarge`] carries.
        limit: usize,
    },
}

impl NestedContent {
    /// How one of the four is to be read, or why it cannot be.
    ///
    /// `detail` says which kind and which resource, because each of the four costs a different
    /// mark and a report that did not distinguish them could not be acted on.
    ///
    /// # Errors
    ///
    /// [`StreamRefusal`], exactly as [`Document::nested_content_source`] gives it.
    pub fn of(document: &Document, stream: &Stream, detail: String) -> Result<Self, StreamRefusal> {
        let source = match document.nested_content_source(stream)? {
            StreamSource::Whole(decoded) => Nested::Whole {
                data: decoded.data,
                damage: decoded.damage,
            },
            StreamSource::Pumped { pump, decoding } => Nested::Windowed {
                data: Arc::clone(&stream.data),
                pumping: pump.pumping(),
                filters: filter_names(document, &stream.dict),
                limit: document.limits().max_stream_len,
                decoding,
            },
            StreamSource::Refused { limit } => Nested::Refused { limit },
        };
        Ok(Self { detail, source })
    }

    /// Bytes that are already this program's own, with no stream behind them.
    ///
    /// §12.7.4.3's regenerated widget appearance is the case: what reaches the drawing is a
    /// spliced copy rather than anything the file states, so there is nothing to decode and
    /// nothing that can be damaged.
    #[must_use]
    pub fn constructed(data: Arc<[u8]>, detail: String) -> Self {
        Self {
            detail,
            source: Nested::Whole { data, damage: None },
        }
    }

    /// Whether the bytes are inflated through a window rather than held whole.
    ///
    /// **A route decision is not observable from its output**, which is the whole difficulty: the
    /// bytes are the same bytes and the report is the same report either way, so a round that
    /// pointed §8.7.3.1's cell at the wrong constructor would break nothing a gate can see —
    /// which cost 0.24 s → 9.0 s on a document nobody times for as long as that cell was
    /// interpreted once per site (ADR 0427), and which is why the route is asserted rather than
    /// assumed even now that ADR 0430 has made the cell one read like the others. This is what
    /// `tests/nested_content_window.rs` asks — the same reason `inflate_buffer` exists so that a
    /// test can read `Vec::capacity` (ADR 0354).
    #[must_use]
    pub fn windowed(&self) -> bool {
        matches!(self.source, Nested::Windowed { .. })
    }

    /// Which of §7.8.2's kinds this is, for the report.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// A reader over it, from the beginning. One per run.
    #[must_use]
    pub fn reader(&self) -> ContentReader<'_> {
        match &self.source {
            Nested::Whole { data, .. } => ContentReader::over(data),
            Nested::Windowed {
                data,
                pumping,
                filters,
                limit,
                ..
            } => ContentReader {
                held: Held::Window(Box::new(Window::single(
                    pumping.clone(),
                    Arc::clone(data),
                    filters.clone(),
                    *limit,
                ))),
            },
            Nested::Refused { limit } => ContentReader {
                held: Held::Window(Box::new(Window::refused(*limit))),
            },
        }
    }

    /// Which decode a windowed read is running, for a run that reaches its bound.
    ///
    /// `None` for the other two shapes, and each for its own reason: a stream held whole never
    /// reaches the bound, and one already refused has nothing left to learn.
    #[must_use]
    pub fn decoding(&self) -> Option<&pdf_syntax::Decoding> {
        match &self.source {
            Nested::Windowed { decoding, .. } => Some(decoding),
            Nested::Whole { .. } | Nested::Refused { .. } => None,
        }
    }

    /// Why the decode stopped short, where that is known without reading the stream.
    ///
    /// Always `None` for the windowed shape, which is not a silence: there the damage is met
    /// as the pump reaches it, in the middle of the run, and `Interpreter::run` reports it in
    /// the same words. ADR 0343 requires the report either way and neither route drops it.
    #[must_use]
    pub fn stated_damage(&self) -> Option<(Damage, usize)> {
        match &self.source {
            Nested::Whole { data, damage } => damage.map(|damage| (damage, data.len())),
            // A refusal is not damage — ADR 0365's distinction — so this stays `None` for the
            // same reason the windowed shape does, and the report the reader raises is the
            // bound's rather than the filter's.
            Nested::Windowed { .. } | Nested::Refused { .. } => None,
        }
    }

    /// Why the decode stopped short and how many bytes are on the page, ISO 32000-2 §7.4.1.
    ///
    /// `None` where the stream decodes whole, which is every stream but ADR 0343's. For the
    /// windowed shape the answer costs one pass of the pump and **no allocation past the
    /// window** — which is the point: asking a bomb whether it is damaged may not cost what
    /// reading it costs. §12.5.5's appearance is the one caller, because that clause's damage
    /// has to be known where the stream is read rather than where it is drawn (ADR 0359).
    #[must_use]
    pub fn damage(&self) -> Option<(Damage, usize)> {
        match &self.source {
            Nested::Whole { data, damage } => damage.map(|damage| (damage, data.len())),
            // Nothing was decoded and nothing can therefore have been cut short.
            Nested::Refused { .. } => None,
            Nested::Windowed { .. } => {
                let mut reader = self.reader();
                loop {
                    let held = reader.lookahead(WINDOW).0.len();
                    if held == 0 {
                        break;
                    }
                    reader.skip(held);
                }
                reader
                    .take_issues()
                    .into_iter()
                    .find_map(|issue| match issue {
                        ContentIssue::Damaged { damage, kept, .. } => Some((damage, kept)),
                        _ => None,
                    })
            }
        }
    }
}

/// One content stream, read forwards.
///
/// Two shapes, and the difference is only whether the bytes exist all at once:
///
/// - [`ContentReader::over`] reads a buffer that is already whole — a page's `/Contents`
///   assembled by an examiner, or one of §7.8.2's four that the memo keeps.
/// - [`ContentReader::for_page`] reads a page's `/Contents` through a window, and
///   [`NestedContent::reader`] one of the other four whose decode the memo declines.
#[derive(Debug)]
pub struct ContentReader<'a> {
    /// Where the bytes come from.
    held: Held<'a>,
}

/// The two shapes of [`ContentReader`].
#[derive(Debug)]
enum Held<'a> {
    /// A content stream already whole in memory, lexed where it lies.
    Whole {
        /// The bytes.
        bytes: &'a [u8],
        /// How far the cursor has got.
        at: usize,
    },
    /// A page's `/Contents`, pumped into a fixed buffer.
    ///
    /// Boxed because it is much the larger of the two and a `ContentReader` is passed by
    /// reference through the whole interpreter.
    Window(Box<Window>),
}

impl<'a> ContentReader<'a> {
    /// A reader over a content stream that is already whole in memory.
    #[must_use]
    pub fn over(bytes: &'a [u8]) -> Self {
        Self {
            held: Held::Whole { bytes, at: 0 },
        }
    }

    /// A reader over a page's `/Contents`, ISO 32000-2 §7.7.3.3 Table 31.
    ///
    /// Each part is kept beside the object *as written*, because a `/Contents` the file does
    /// not state and a `/Contents` naming an object this reader could not reach both resolve
    /// to null and are not the same statement — see [`ContentIssue::Unreachable`].
    #[must_use]
    pub fn for_page(document: &'a Document, page: &Page) -> Self {
        let stated = page.dict.get("Contents").cloned().unwrap_or(Object::Null);
        let listed: Vec<(Object, Object)> = match document.resolve(&stated) {
            Object::Array(items) => items
                .iter()
                .map(|item| (item.clone(), document.resolve(item)))
                .collect(),
            other => vec![(stated, other)],
        };

        let mut window = Window::new(document.limits().max_stream_len);
        for (index, (named, part)) in listed.iter().enumerate() {
            // A `/Contents` that is missing entirely is an empty page, not a defect; one
            // whose entries are not streams is a malformed page and worth saying so.
            let Some(stream) = part.as_stream() else {
                match (named, part) {
                    (Object::Reference(object), Object::Null) => {
                        window.issues.push(ContentIssue::Unreachable {
                            index,
                            object: *object,
                        });
                    }
                    (_, Object::Null) => {}
                    _ => window.issues.push(ContentIssue::NotAStream { index }),
                }
                continue;
            };
            window.push(document, stream, index);
        }
        Self {
            held: Held::Window(Box::new(window)),
        }
    }

    /// Everything this reader has to say about the stream, taken away.
    ///
    /// Called before interpretation for what building the reader found, and again after it
    /// for what pumping found: a damaged part is met where the damage is, which under a
    /// window is in the middle of the page rather than before it. ADR 0343 requires the
    /// report either way — a page cut short otherwise looks like a page meant to be sparse.
    pub fn take_issues(&mut self) -> Vec<ContentIssue> {
        match &mut self.held {
            Held::Whole { .. } => Vec::new(),
            Held::Window(window) => std::mem::take(&mut window.issues),
        }
    }

    /// Reads the next token and hands it to `read`, or `None` at the end of the content.
    ///
    /// **The token is lent rather than given, and that is what makes the window safe to move.**
    /// A `Token::Keyword` borrows its bytes from the buffer the lexer read them out of, and a
    /// refill overwrites them. Returning the token instead would leave two ways out and both
    /// cost something: a signature saying `Option<Token<'_>>` cannot express "and do not ask me
    /// for the next one while you hold it" without the borrow checker refusing the caller
    /// outright, so the caller ends up copying every token to get around it — which is the
    /// heap allocation per token that ADR 0341 removed. Confining it to a closure costs
    /// neither: the caller takes what it needs while the borrow is alive, the compiler refuses
    /// anything that keeps it, and `doc/todo/14`'s obligation is discharged by a signature
    /// rather than by a comment.
    #[inline]
    pub fn with_token<T>(&mut self, read: impl FnOnce(Option<Token<'_>>) -> T) -> T {
        match &mut self.held {
            Held::Whole { bytes, at } => {
                let mut lexer = Lexer::at(bytes, *at);
                let token = lexer.next_token();
                *at = lexer.position();
                read(token)
            }
            Held::Window(window) => window.with_token(read),
        }
    }

    /// The bytes from the cursor onwards, at least `want` of them where the stream has them,
    /// and whether that is everything the stream has left.
    ///
    /// For §8.9.7's inline images, which are the one construction in a content stream that is
    /// not a token: `BI` is followed by data whose length the dictionary need not state, so
    /// where it ends is found by reading it. A caller whose answer depends on bytes it has not
    /// been given must ask again for more — see `Interpreter::inline_image`, and the second
    /// half of the pair is what tells "the stream ends here" from "the buffer does".
    pub fn lookahead(&mut self, want: usize) -> (&[u8], bool) {
        match &mut self.held {
            Held::Whole { bytes, at } => (bytes.get(*at..).unwrap_or_default(), true),
            Held::Window(window) => window.lookahead(want),
        }
    }

    /// Consumes `count` bytes of what [`Self::lookahead`] returned.
    pub fn skip(&mut self, count: usize) {
        match &mut self.held {
            Held::Whole { bytes, at } => *at = at.saturating_add(count).min(bytes.len()),
            Held::Window(window) => window.skip(count),
        }
    }

    /// Reads the whole content stream into one buffer, as [`Page::content_with_report`] does.
    ///
    /// The route every caller took before the window existed, expressed through the window so
    /// that there is one assembly of `/Contents` and not two. What it costs is the allocation
    /// the window exists to avoid, which is why the interpreter does not call it.
    #[must_use]
    pub fn read_to_end(&mut self) -> Vec<u8> {
        match &mut self.held {
            Held::Whole { bytes, at } => {
                let rest = bytes.get(*at..).unwrap_or_default().to_vec();
                *at = bytes.len();
                rest
            }
            Held::Window(window) => window.read_to_end(),
        }
    }
}

/// A page's `/Contents`, pumped into a fixed buffer.
#[derive(Debug)]
struct Window {
    /// The parts, in the order Table 31 states them.
    parts: Vec<Part>,
    /// Which part is being produced.
    index: usize,
    /// The window itself. Grows to [`CEILING`] for one long token, and further only for
    /// [`Window::lookahead`]'s inline images.
    buffer: Vec<u8>,
    /// How much of `buffer` holds content.
    filled: usize,
    /// How far the cursor has got into `buffer`.
    at: usize,
    /// How many decoded bytes the whole `/Contents` has produced, for Table 31's bound.
    total: usize,
    /// `Limits::max_stream_len`, which Table 31 makes the bound on the concatenation as well
    /// as on one part: the array's streams "form a single stream".
    limit: usize,
    /// Whether the newline that follows the part just ended is still owed.
    separator: bool,
    /// Which of the two things §7.8.2 names this window is over.
    shape: Shape,
    /// Set once no further byte can arrive.
    exhausted: bool,
    /// Set where a comment was cut by the end of the buffer, so that the rest of it is
    /// skipped rather than read as content.
    in_comment: bool,
    /// What the assembly and the pumping have found.
    issues: Vec<ContentIssue>,
}

/// Which of the two things §7.8.2 names a [`Window`] is over.
///
/// They differ in one byte and it is Table 31's: the array's parts are concatenated "with at
/// least one white-space character added between the streams' data", and
/// `Page::content_with_report` has always written that byte after every part including the
/// last. One of §7.8.2's other four is not an array, and is handed to the interpreter as the
/// bytes its filter produced and nothing else — the window has to deliver exactly what the
/// whole decode it replaces delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// A page's `/Contents`, ISO 32000-2 §7.7.3.3 Table 31.
    Contents,
    /// A form `XObject`, a tiling pattern's cell, a Type 3 glyph description or an
    /// annotation's appearance — §7.8.2's "self-contained graphical elements".
    SelfContained,
}

/// One `/Contents` part.
#[derive(Debug)]
struct Part {
    /// Which part of `/Contents`, counting from zero, for the report.
    index: usize,
    /// Where its bytes come from.
    source: PartSource,
    /// The `/Filter` names it declared, for the report.
    filters: Vec<String>,
    /// How many decoded bytes it has produced, which is [`ContentIssue::Damaged`]'s `kept`.
    kept: usize,
}

/// Where one part's decoded bytes come from.
#[derive(Debug)]
enum PartSource {
    /// Decoded whole, by the route every other caller takes.
    Bytes {
        /// The decoded bytes.
        data: Arc<[u8]>,
        /// How many of them have been handed over.
        at: usize,
    },
    /// Inflated a window at a time.
    Pumped(pdf_syntax::Pump),
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "cursor arithmetic inside one buffer whose length bounds every index, and \
              counters bounded by `limit`; every slice access uses `get` and every step is \
              checked against `buffer.len()` or `filled` before it is taken"
)]
impl Window {
    /// An empty window under `limit`.
    fn new(limit: usize) -> Self {
        Self {
            parts: Vec::new(),
            index: 0,
            buffer: vec![0u8; WINDOW],
            filled: 0,
            at: 0,
            total: 0,
            limit,
            separator: false,
            shape: Shape::Contents,
            exhausted: false,
            in_comment: false,
            issues: Vec::new(),
        }
    }

    /// A window over one of §7.8.2's other four content streams, which has exactly one part.
    ///
    /// The part index is zero because there is one, and `ContentIssue` is Table 31's noun —
    /// so the issues this window raises are translated by the interpreter into the vocabulary
    /// the four are reported in, and never reach a page's own `/Contents` report. See
    /// `Interpreter::run`.
    fn single(pumping: Pumping, data: Arc<[u8]>, filters: Vec<String>, limit: usize) -> Self {
        let mut window = Self::new(limit);
        window.shape = Shape::SelfContained;
        window.parts.push(Part {
            index: 0,
            source: PartSource::Pumped(pdf_syntax::Pump::new(pumping, data)),
            filters,
            kept: 0,
        });
        window
    }

    /// A window with no parts, over a decode the document has already read to `limit`.
    ///
    /// **The report is the whole of it.** [`Window::refill`] raises exactly this issue when a
    /// window reaches the bound, so a read served from
    /// [`pdf_syntax::Document::window_found_nothing`]'s memory says what the read that learnt
    /// the fact said, in the same words and with the same number — and produces the same
    /// nothing, because the memory is only recorded for a run that produced nothing.
    fn refused(limit: usize) -> Self {
        let mut window = Self::new(limit);
        window.shape = Shape::SelfContained;
        window.exhausted = true;
        window
            .issues
            .push(ContentIssue::TooLarge { part: None, limit });
        window
    }

    /// Adds one `/Contents` part, or reports why it contributes nothing.
    fn push(&mut self, document: &Document, stream: &Stream, index: usize) {
        let source = match document.stream_source(stream) {
            Ok(StreamSource::Whole(decoded)) => {
                // §7.4.1's filter was invoked and stopped short of the end. What it did
                // produce goes on the page; that it is not all of it goes in the report.
                if let Some(damage) = decoded.damage {
                    self.issues.push(ContentIssue::Damaged {
                        index,
                        damage,
                        kept: decoded.data.len(),
                        filters: filter_names(document, &stream.dict),
                    });
                }
                PartSource::Bytes {
                    data: decoded.data,
                    at: 0,
                }
            }
            // The `Decoding` is dropped here rather than kept, and that is the scope of ADR
            // 0646 rather than an omission: Table 31 makes the array's parts "a single stream"
            // and the bound is over the concatenation, so a window that reaches it has not
            // named *which* part outgrew it. A page's `/Contents` is also read once per render
            // rather than once per site, which is the population §7.8.2's other four are in.
            Ok(StreamSource::Pumped { pump, .. }) => PartSource::Pumped(pump),
            // Two ways to reach the same fact, so one arm. The first is a part the document has
            // already read to the bound, which it can only know of one of §7.8.2's other four —
            // a page's own `/Contents` never reaches this half. The second is a bound refusing
            // this part; the filter chain did not fail to work, and saying "undecodable" of a
            // stream this reader can decode perfectly well would put a limit of ours into a
            // sentence about the file.
            Ok(StreamSource::Refused { limit })
            | Err(StreamRefusal::Filter {
                why: pdf_syntax::FilterRefusal::TooLarge { limit },
                ..
            }) => {
                self.issues.push(ContentIssue::TooLarge {
                    part: Some(index),
                    limit,
                });
                return;
            }
            Err(_) => {
                self.issues.push(ContentIssue::Undecodable {
                    index,
                    filters: filter_names(document, &stream.dict),
                });
                return;
            }
        };
        self.parts.push(Part {
            index,
            source,
            filters: filter_names(document, &stream.dict),
            kept: 0,
        });
    }

    /// The next token, refilling and growing the buffer as the stream needs.
    ///
    /// **The first arm is the whole stream but its boundaries**, and it is written out rather
    /// than left to the loop below because this is the hottest path in the program: a token
    /// that ends *before* the end of what is buffered is a token no boundary touched,
    /// whichever kind it is — the lexer stopped because it read the byte that ends it — and a
    /// comment it skipped over ended for the same reason. So an ordinary token costs one
    /// comparison more than lexing a whole buffer did, and the second pass below is paid by
    /// the 2 258 tokens that straddle a refill rather than by the 20 834 587 that do not.
    #[inline]
    fn with_token<T>(&mut self, read: impl FnOnce(Option<Token<'_>>) -> T) -> T {
        if !self.in_comment && self.filled.saturating_sub(self.at) >= SLACK {
            let mut lexer = Lexer::at(self.buffer.get(..self.filled).unwrap_or_default(), self.at);
            let token = lexer.next_token();
            let end = lexer.position();
            if end < self.filled {
                self.at = end;
                return read(token);
            }
        }
        self.settle_then(read)
    }

    /// The rest of [`Self::with_token`]: refilling, growing and the two boundary cases.
    ///
    /// Split out so that the arm above stays a leaf, and cold because it runs once per refill.
    #[cold]
    fn settle_then<T>(&mut self, read: impl FnOnce(Option<Token<'_>>) -> T) -> T {
        loop {
            self.settle();
            // A token that ends exactly where the buffer does may have been cut by the
            // boundary, so where there is more it is lexed again once there is. The token
            // from this pass is deliberately dropped rather than returned: it may be half a
            // string, and only the second pass — over a buffer the refill has completed —
            // knows that it is not.
            let whole = self.exhausted || {
                let mut lexer =
                    Lexer::at(self.buffer.get(..self.filled).unwrap_or_default(), self.at);
                let seen = lexer.next_token().is_some();
                seen && lexer.position() < self.filled
            };
            if whole {
                break;
            }
            if !self.widen() {
                // Nothing more can be brought in and the token still reaches the end of the
                // buffer: it is longer than [`CEILING`]. Said out loud and stepped over,
                // because a buffer cannot hold it and a silent truncation would put bytes the
                // file never wrote in front of the interpreter (ADR 0306's lesson).
                if self.exhausted {
                    break;
                }
                self.issues
                    .push(ContentIssue::TokenTooLong { limit: CEILING });
                self.drop_token();
            }
        }
        let mut lexer = Lexer::at(self.buffer.get(..self.filled).unwrap_or_default(), self.at);
        let token = lexer.next_token();
        self.at = lexer.position();
        read(token)
    }

    /// Steps over a token no buffer of [`CEILING`] bytes can hold.
    ///
    /// To the next white-space byte, which §7.2.3 makes the one thing that ends every kind of
    /// token, refilling until one is found or the stream ends. What is stepped over is
    /// reported by the caller; nothing here is silent.
    fn drop_token(&mut self) {
        loop {
            let found = self.buffer.get(self.at..self.filled).and_then(|rest| {
                rest.iter()
                    .position(|&byte| pdf_syntax::lexer::is_whitespace(byte))
            });
            if let Some(offset) = found {
                self.at += offset;
                return;
            }
            self.at = self.filled;
            if !self.pull() {
                return;
            }
        }
    }

    /// Puts the cursor on the first byte of a token, refilling across comments and white
    /// space, and leaves [`SLACK`] bytes ahead of it wherever the stream has them.
    fn settle(&mut self) {
        loop {
            if self.in_comment {
                // §7.2.4: a comment runs to the end of the line. One that the buffer cut is
                // still a comment, and reading its tail as content is how a window-fed lexer
                // silently changes a page.
                let rest = self.buffer.get(self.at..self.filled).unwrap_or_default();
                if let Some(offset) = rest.iter().position(|&byte| byte == b'\n' || byte == b'\r') {
                    self.at += offset;
                    self.in_comment = false;
                } else {
                    self.at = self.filled;
                    if !self.pull() {
                        return;
                    }
                    continue;
                }
            }

            let before = self.at;
            let mut lexer = Lexer::at(self.buffer.get(..self.filled).unwrap_or_default(), self.at);
            lexer.skip_whitespace();
            self.at = lexer.position();
            if self.exhausted {
                return;
            }
            if self.at >= self.filled {
                // The run reaches the end of what is buffered, so it may continue past it —
                // and if it ended inside a comment, so does the comment.
                self.in_comment =
                    comment_open(self.buffer.get(before..self.at).unwrap_or_default());
                if !self.pull() {
                    return;
                }
                continue;
            }
            if self.filled - self.at < SLACK && self.pull() {
                continue;
            }
            return;
        }
    }

    /// Compacts what is left and refills, returning whether anything new arrived.
    fn pull(&mut self) -> bool {
        if self.exhausted {
            return false;
        }
        if self.at > 0 {
            self.buffer.copy_within(self.at..self.filled, 0);
            self.filled -= self.at;
            self.at = 0;
        }
        let before = self.filled;
        self.refill();
        self.filled > before
    }

    /// The same, growing the buffer up to [`CEILING`] where compacting freed nothing.
    ///
    /// Returns whether the buffer now holds more than it did.
    fn widen(&mut self) -> bool {
        if self.pull() {
            return true;
        }
        if self.exhausted || self.buffer.len() >= CEILING {
            return false;
        }
        let grown = self.buffer.len().saturating_mul(2).min(CEILING);
        self.buffer.resize(grown, 0);
        let before = self.filled;
        self.refill();
        self.filled > before
    }

    /// Fills the tail of the buffer from the parts.
    fn refill(&mut self) {
        while self.filled < self.buffer.len() && !self.exhausted {
            // Table 31 makes the array of parts "a single stream", so the bound one stream
            // gets is the bound the array gets — and under a window that is the only bound
            // left to apply, because there is no per-part allocation to bound any more. ADR
            // 0362's consequence, carried out.
            let allowed = self.limit.saturating_sub(self.total);
            if allowed == 0 {
                self.issues.push(ContentIssue::TooLarge {
                    part: None,
                    limit: self.limit,
                });
                self.exhausted = true;
                return;
            }
            let room = (self.buffer.len() - self.filled).min(allowed);
            match self.produce(room) {
                Some(wrote) => {
                    self.filled += wrote;
                    self.total += wrote;
                }
                None => return,
            }
        }
    }

    /// Writes at most `room` bytes into the tail of the buffer.
    ///
    /// `None` where this part had nothing more to give and the next one has been started, or
    /// where the content has ended; `Some(0)` where the decoder took input without producing
    /// output, which is progress of a kind the caller must keep asking through.
    fn produce(&mut self, room: usize) -> Option<usize> {
        if self.separator {
            // Table 31: the parts are concatenated "with at least one white-space character
            // added between the streams' data". The byte is written where the part ends
            // rather than where the next one begins, so that a `/Contents` of one part reads
            // exactly as `Page::content_with_report` has always assembled it.
            self.separator = false;
            *self.buffer.get_mut(self.filled)? = b'\n';
            return Some(1);
        }
        // The part, the buffer and the report are three fields of one structure, and the
        // pump writes into the second while reading the first — so they are taken apart here
        // rather than reached through `self`, and what the part *ended* is decided afterwards.
        let Self {
            parts,
            index,
            buffer,
            filled,
            issues,
            ..
        } = self;
        let Some(part) = parts.get_mut(*index) else {
            self.exhausted = true;
            return None;
        };
        let out = buffer.get_mut(*filled..filled.saturating_add(room))?;
        let (wrote, ended) = match &mut part.source {
            PartSource::Bytes { data, at } => {
                let left = data.get(*at..).unwrap_or_default();
                let take = left.len().min(out.len());
                out.get_mut(..take)?.copy_from_slice(left.get(..take)?);
                *at += take;
                part.kept += take;
                (take, *at >= data.len())
            }
            PartSource::Pumped(pump) => match pump.pump(out) {
                Pumped::Wrote(wrote) => {
                    part.kept += wrote;
                    (wrote, false)
                }
                Pumped::Ended(wrote) => {
                    part.kept += wrote;
                    (wrote, true)
                }
                Pumped::Damaged(wrote, damage) => {
                    part.kept += wrote;
                    // A part *nothing* came out of is not damage but a refusal: it is the
                    // case `flate` answers with `FilterRefusal::Corrupt`, which
                    // `Page::content_with_report` has always reported as undecodable.
                    issues.push(if part.kept == 0 {
                        ContentIssue::Undecodable {
                            index: part.index,
                            filters: part.filters.clone(),
                        }
                    } else {
                        ContentIssue::Damaged {
                            index: part.index,
                            damage,
                            kept: part.kept,
                            filters: part.filters.clone(),
                        }
                    });
                    (wrote, true)
                }
            },
        };
        if ended {
            self.end_part();
        }
        Some(wrote)
    }

    /// Moves to the next part, owing the white space Table 31 puts between them.
    fn end_part(&mut self) {
        self.index += 1;
        self.separator = self.shape == Shape::Contents;
    }

    /// See [`ContentReader::lookahead`].
    fn lookahead(&mut self, want: usize) -> (&[u8], bool) {
        while self.filled - self.at < want && !self.exhausted {
            if self.at > 0 {
                self.buffer.copy_within(self.at..self.filled, 0);
                self.filled -= self.at;
                self.at = 0;
            }
            if self.filled >= self.buffer.len() {
                if self.buffer.len() >= want {
                    break;
                }
                self.buffer.resize(want, 0);
            }
            let before = self.filled;
            self.refill();
            if self.filled == before {
                break;
            }
        }
        let held = self.filled - self.at;
        let end = self.filled.min(self.at.saturating_add(want));
        (
            self.buffer.get(self.at..end).unwrap_or_default(),
            self.exhausted && held <= want,
        )
    }

    /// See [`ContentReader::skip`].
    fn skip(&mut self, count: usize) {
        let mut left = count;
        while left > 0 {
            let here = (self.filled - self.at).min(left);
            self.at += here;
            left -= here;
            if left > 0 && !self.pull() {
                return;
            }
        }
        // A buffer grown for one inline image is not kept: the window is what the rest of
        // the stream is read through.
        if self.buffer.len() > CEILING {
            self.buffer.copy_within(self.at..self.filled, 0);
            self.filled -= self.at;
            self.at = 0;
            self.buffer.truncate(self.filled.max(WINDOW));
            self.buffer.shrink_to_fit();
        }
    }

    /// See [`ContentReader::read_to_end`].
    fn read_to_end(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            out.extend_from_slice(self.buffer.get(self.at..self.filled).unwrap_or_default());
            self.at = self.filled;
            if !self.pull() {
                return out;
            }
        }
    }
}

/// Whether a run of white space and comments ends inside a comment.
///
/// §7.2.4 ends a comment at "an EOL marker", so a `%` with no end of line after it is a
/// comment the buffer cut rather than one that finished.
fn comment_open(run: &[u8]) -> bool {
    for &byte in run.iter().rev() {
        if byte == b'\n' || byte == b'\r' {
            return false;
        }
        if byte == b'%' {
            return true;
        }
    }
    false
}
