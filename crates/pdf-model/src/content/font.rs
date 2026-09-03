//! Selecting and loading fonts: `Tf`, Table 57's `/Font`, and §9.6.2.2's standard fourteen.
//!
//! What a loaded [`Font`] *is* — an outline program or a Type 3 content stream — is decided
//! here once and cached by the object's identity; showing its glyphs is [`super::text`]'s.

use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_font::Code;
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::report::Unsupported;
use super::run::narrow;
use super::{GraphicsState, Interpreter};

/// The current font, which is one of the two kinds PDF has.
///
/// They differ in what a glyph *is*. Every font with a program hands out an outline, and the
/// interpreter fills it. A Type 3 font hands out a content stream, and the interpreter runs
/// it — see `crate::type3` for why that puts the two kinds in different crates.
#[derive(Debug, Clone)]
pub(super) enum Font {
    /// A font with a glyph program, read by `pdf-font`.
    Program(Arc<pdf_font::LoadedFont>),
    /// A Type 3 font, whose glyphs are content streams (§9.6.4).
    Type3(Arc<crate::type3::Type3Font>),
}

impl Font {
    /// Whether this font is shown in §9.2.4's writing mode 1.
    ///
    /// A Type 3 font is a *simple* font and §9.2.4 confines a second set of metrics to
    /// composite ones, so it is never vertical.
    pub(super) fn is_vertical(&self) -> bool {
        match self {
            Self::Program(font) => font.is_vertical(),
            Self::Type3(_) => false,
        }
    }

    /// Splits a PDF string into character codes.
    ///
    /// A Type 3 font is a simple font — Table 110 gives it `/FirstChar` and `/LastChar`,
    /// which are byte codes — so one byte is one code, always.
    pub(super) fn decode(&self, bytes: &[u8]) -> Vec<Code> {
        match self {
            Self::Program(font) => font.decode(bytes),
            Self::Type3(_) => bytes.iter().copied().map(Code::single_byte).collect(),
        }
    }

    /// A code's advance in text-space units, where one em is 1.0.
    pub(super) fn advance(&self, code: Code) -> f32 {
        match self {
            Self::Program(font) => font.advance(code),
            Self::Type3(font) => font.advance(code.value()),
        }
    }

    /// Appends what a code means to the page's extracted text.
    ///
    /// §9.10.2's methods first, and where every one of them has declined, the one code the
    /// standard names a character for outside that clause. §9.3.3 states it twice, and the
    /// first sentence is the naming:
    ///
    /// > Word spacing works the same way as character spacing but shall apply only to the
    /// > ASCII SPACE character (20h).
    ///
    /// > Word spacing shall be applied to every occurrence of the single-byte character code
    /// > 32 in a string when using a simple font (including Type 3) or a composite font that
    /// > defines code 32 as a single-byte code.
    ///
    /// Read together those say that a single-byte code 32 in a show string **is** the ASCII
    /// SPACE character — the clause identifies the code with the character in order to say
    /// which glyph `Tw` applies to, and identifying them is what it does. So a font whose
    /// encoding, `/ToUnicode` and program all decline to say what such a code means has not
    /// contradicted the clause; it has said nothing, and the clause has already said it.
    ///
    /// **This is last, not first**, because §9.10.2's methods are the producer's own
    /// statements about a code and this is the standard's about the encoding. A
    /// `/Differences` naming code 32 `/bullet`, or a `/ToUnicode` mapping it to U+2019, is
    /// answered by the earlier method and never reaches here.
    ///
    /// It is what [`pdf_font::LoadedFont`]'s own last resort excludes: §9.10.2's closing
    /// permission is taken there for 0x21 to 0x7E only, because reading a code *as* its byte
    /// is a choice about a producer's convention. This one is not that choice. Two corpus
    /// documents show the difference — `issue4304.pdf` is 895 bytes named after it, a
    /// `/Times-Roman` whose `/Differences` maps 32 to `/.notdef`, drawing
    /// *Words that should have spaces between them.* since the four-hundred-and-fifth session
    /// fixed its advances and reading back `Wordsthatshouldhavespacesbetweenthem.` until this;
    /// and `Type3WordSpacing.pdf`, whose Type 3 font names no glyph at code 32 at all and
    /// whose six lines are drawn with `Tw` from 50 down to 0.
    pub(super) fn text(&self, code: Code, out: &mut String) -> bool {
        if match self {
            Self::Program(font) => font.text(code, out),
            Self::Type3(font) => font.text(code.value(), out),
        } {
            return true;
        }
        if code.takes_word_spacing() {
            out.push(' ');
            return true;
        }
        false
    }

    /// Which of §9.10.2's methods could have named a code and did not, for a code that read
    /// back as nothing.
    ///
    /// Both kinds of font answer it — a page mixes them freely — and what differs is which of the
    /// clause's methods could have applied, which is the question the answer is about. §9.3.3's
    /// code 32 is not consulted here: this is asked only where [`Self::text`] has already
    /// declined, and that rule is inside it.
    pub(super) fn naming_gap(&self, code: Code) -> Option<pdf_font::NamingGap> {
        match self {
            Self::Program(font) => font.naming_gap(code),
            Self::Type3(font) => font.naming_gap(code.value()),
        }
    }

    /// What [`FONT_BUDGET`] charges for keeping this font, in bytes.
    ///
    /// A Type 3 font has no program — §9.6.4 makes its glyphs content streams the interpreter
    /// runs — so what it retains is the `/CharProcs` dictionary rather than a face, and it is
    /// charged nothing here. The budget is a bound on font *programs*, and inventing a number
    /// for the other kind would make the constant mean two things.
    fn program_bytes(&self) -> usize {
        match self {
            Self::Program(font) => font.program_bytes(),
            Self::Type3(_) => 0,
        }
    }
}

/// What a loaded font is remembered by.
///
/// Two things select a font and they do not select it the same way, which is §8.4.1 NOTE 1's
/// "either way" with a twist: `Tf` names a resource, and §8.4.5's Table 57 `/Font` is an
/// array whose first element "shall be an indirect object reference instead of a resource
/// name". Both are cached, and a document that reaches one font both ways loads it twice —
/// which costs one parse and is the price of not pretending a name and an object identity are
/// the same key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FontKey {
    /// A font dictionary, by the object it is.
    ///
    /// The only kind, since the hundred-and-twenty-seventh session: keying by the resource
    /// *name* conflated a page's `/F1` with a form `XObject`'s. Kept as an enum of one because
    /// the two routes to a font — `Tf`'s resource name and Table 57's `/Font`, which §8.4.1's
    /// NOTE 1 makes alternatives — arrive here differently and only this says they are the
    /// same thing when they name the same object.
    Referenced(ObjectId),
}

/// How many bytes of font program one open document's [`FontCache`] may hold.
///
/// **2 MiB, and both halves of that are derived rather than chosen**, in the form
/// `pdf_syntax::DECODED_BUDGET` states its own. `examples/font_cache_budget` is the instrument
/// for both halves; it runs the real cache over a real page sequence at each budget rather than
/// simulating one, and prints the process's own high-water resident memory beside each row.
///
/// *The floor* is what a smaller bound gives up. ISO 32000-2's first hundred pages, which name
/// **32** distinct fonts between them; `misses` is the instrument's own column and roughly twice
/// the loads, because `Interpreter::font` and `Interpreter::load_font` each ask:
///
/// | budget | fonts held | charged | misses | evicted | peak resident |
/// |---|---|---|---|---|---|
/// | 0 — the cache off | 0 | 0 | 1398 | — | 61 684 kB |
/// | 1 MiB | 6 | 951 516 | 514 | 251 | 61 936 kB |
/// | **2 MiB** | **17** | **2 012 309** | **84** | **25** | **62 600 kB** |
/// | 4 MiB | 32 | 3 943 195 | 64 | 0 | 64 284 kB |
/// | 8 MiB | 32 | 3 943 195 | 64 | 0 | 64 228 kB |
///
/// A megabyte *thrashes* — 251 evictions to save two thirds of the loads — and two takes 94% of
/// what an unbounded cache gives on the largest document this project owns.
///
/// *The ceiling* is the project owner's band — "1 GB is definitely too much, below 10 MB is
/// definitely ok" (ADR 0256) — and this is the **third** per-document cache under it, after the
/// readback's 4 MiB and the decoded streams' 4 MiB. Three nominal budgets sum to 10 MiB, which
/// would be the whole band; what the process actually spends is smaller, and the difference is
/// the reason this is a measurement rather than an addition. **Holding 3.9 MB of font program
/// costs 2.6 MB of resident memory**, because an embedded program's bytes are the *same
/// allocation* the decoded-stream cache is already charged for — counted twice and spent once.
///
/// **And that is also the second reason not to take 4 MiB**, which the same instrument found by
/// being run over the whole document rather than a hundred pages of it. Over all 1023 pages the
/// budget holds 65 fonts at 4 MiB and costs **+6.3 MB** of resident memory against the cache
/// off — the widths, `CMap`s and built outlines that sit beside a program and that this budget
/// deliberately does not charge for. At 2 MiB the same sweep costs **+2.0 MB**, which is the
/// budget and nothing else: the charge tracks the process here and understates it by about half
/// one doubling later. A bound whose accounting stops being true above it is a bound to stay
/// below. ADR 0710.
pub const FONT_BUDGET: usize = 2 * 1024 * 1024;

/// Fonts loaded out of one open document, kept across interpretations of its pages.
///
/// # What it is for
///
/// A page names a handful of fonts and a *document* names the same handful on every page, so an
/// [`Interpreter`] that lives for one page pays §9.6's whole load again on the next: seven of
/// them on page 101 of ISO 32000-2, and 213 of the 240 loads in its first forty pages are of a
/// font an earlier page had already loaded. Twenty distinct pages of that document interpret in
/// **−14.86%** of the instructions with this than without (ADR 0710).
///
/// # What it changes, and what it does not
///
/// **It changes what an interpretation *costs* and never what it computes**, which is the
/// property `CLAUDE.md`'s exclusion list rests the oracle on — `interpret` is a pure function of
/// what the file says, the viewer state and what the user did. Three things make that true here
/// rather than hoped:
///
/// - **The key is the font's own identity in this document.** `pdf_font::LoadedFont::load` is a
///   function of the document and the font dictionary — the `name` it also takes reaches no
///   field of the result and appears only in a [`pdf_font::FontError`]'s wording — so an
///   [`ObjectId`], which names one dictionary in one file, names one loaded font.
/// - **The document is checked rather than assumed.** An object number means nothing across
///   files, so an entry is answered only while the cache is bound to the document it was loaded
///   from, and the binding is the identity of that document's own bytes ([`Kept::bind`]).
///   The cache *holds* that allocation, which is what stops a freed document's address being
///   handed to the next one — `doc/todo/41`'s own lesson, and the invariant ADR 0317 states for
///   the decoded-stream cache one crate down.
/// - **A failed load is not kept.** The interpreter caches those too, for one page, so that a
///   `Tf` naming an unloadable font on every line reports it once; keeping one *here* would
///   make the second page's `Interpretation::unsupported` depend on whether the first had been
///   interpreted, and would word the report with the first page's resource name. That is a
///   change to the answer, so it is declined — a failing document pays per page exactly what it
///   paid before.
///
/// The memos inside a kept `pdf_font::LoadedFont` — its outlines, its Adobe Glyph List cells —
/// come with it, and they are memos of pure functions of the glyph and the code, so a second
/// page reaching them gets what a first page would have built. That is where more than the load
/// itself comes back: fifty interpretations of one page fall **−31.32%**, which is the
/// population `Open::stale` re-interprets — a layer switched, a value typed — paying for fonts
/// that none of those moves can change.
///
/// # What it costs when it misses
///
/// Two prices, and the second is the one a caller that keeps nothing pays.
///
/// The lookups themselves are noise: a one-page document measured with the cache against
/// without, through one binary, moves **+15** instructions on `alphatrans.pdf` and **+221** on
/// `issue6127.pdf` — whose single page names fifty-one fonts and reuses none, so every lookup
/// misses and every load inserts. What is *not* noise is what making a font shareable cost the
/// whole tree: the `Send` conversion inside `pdf_font::LoadedFont` (its header has the split)
/// plus the insertions land at **+0.54%** on fifty keep-nothing interpretations of page 101 —
/// `callgrind_interpret` before this round's change against after, which is the arm the oracle
/// and the corpus gates run in. That is the trade, and it is taken because the reader's
/// workloads above are 12–31% and the gates' population is one page per document.
#[derive(Debug, Default)]
pub struct FontCache {
    /// Behind a lock because `viewer_core`'s per-document state crosses a thread once — ADR
    /// 0182 opens the document beside the window and moves the viewer back — so everything held
    /// beside a `Document` has to be `Send`. Nothing in this workspace locks it from two
    /// threads; `pdf_font::LoadedFont`'s own header says what making that possible cost.
    held: std::sync::Mutex<Kept>,
}

/// What one [`FontCache`] holds, and the tally of what has happened to it.
#[derive(Debug, Default)]
struct Kept {
    /// The bytes of the document these keys name objects in, held rather than remembered.
    ///
    /// `None` before the first font of the first document. See [`Kept::bind`].
    document: Option<pdf_syntax::FileBytes>,
    /// The fonts, by the object each one's dictionary is.
    fonts: BTreeMap<FontKey, Entry>,
    /// How many bytes of font program that is, counted as it changes.
    bytes: usize,
    /// The ceiling those bytes are held under.
    budget: usize,
    /// A counter that only goes up, which is what "least recently used" is ordered by.
    clock: u64,
    /// How many lookups were answered without a load.
    hits: u64,
    /// How many were not.
    misses: u64,
    /// How many entries the budget has dropped.
    evicted: u64,
    /// How many times a different document arrived and emptied it.
    rebound: u64,
}

/// One kept font, with what it is charged and when it was last used.
#[derive(Debug)]
struct Entry {
    font: Font,
    /// The decoded font program's length, which is what [`FONT_BUDGET`] bounds.
    ///
    /// The term that dominates and the only one that is exactly knowable at this point: the
    /// widths, the `CMap`s and the outlines a font builds beside its program are smaller and
    /// grow after the charge is taken. What they add is not estimated here but *measured*, as
    /// peak resident memory over a sweep at this budget — ADR 0710 has the figure.
    bytes: usize,
    used: u64,
}

/// What one open document's font cache is holding, and how it has been used.
///
/// The bound this project asks of a memory budget is that it be *legible* rather than small, so
/// the number can be read off — the same sentence `pdf_syntax::DecodedStreamCache` is written
/// under. Reported by [`FontCache::report`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontCacheReport {
    /// How many fonts are held.
    pub fonts: usize,
    /// How many bytes of font program that is.
    pub bytes: usize,
    /// The ceiling those bytes are held under, which is [`FONT_BUDGET`] for every document.
    pub budget: usize,
    /// How many lookups were answered without a load.
    pub hits: u64,
    /// How many were not.
    pub misses: u64,
    /// How many entries the budget has dropped.
    pub evicted: u64,
    /// How many times a different document arrived and emptied it.
    pub rebound: u64,
}

impl FontCache {
    /// An empty cache under [`FONT_BUDGET`], bound to no document yet.
    #[must_use]
    pub fn new() -> Self {
        Self::with_budget(FONT_BUDGET)
    }

    /// An empty cache under a stated budget.
    ///
    /// The budget is a parameter for the reason `pdf_syntax::DecodedStreams::with_budget`'s is:
    /// so that eviction can be exercised on a budget of a few bytes, and so that a measurement
    /// can build the same tree with the cache off. Every document opens with [`FONT_BUDGET`].
    #[must_use]
    pub fn with_budget(budget: usize) -> Self {
        Self {
            held: std::sync::Mutex::new(Kept {
                budget,
                ..Kept::default()
            }),
        }
    }

    /// What this cache is holding, for a caller that wants to say so.
    #[must_use]
    pub fn report(&self) -> FontCacheReport {
        let kept = self.kept();
        FontCacheReport {
            fonts: kept.fonts.len(),
            bytes: kept.bytes,
            budget: kept.budget,
            hits: kept.hits,
            misses: kept.misses,
            evicted: kept.evicted,
            rebound: kept.rebound,
        }
    }

    /// Reads past a lock a panicking thread poisoned.
    ///
    /// The same argument `pdf_syntax::Document`'s caches are read under: this holds no
    /// invariant across fields, every write is one insertion into one map, and the worst a
    /// poisoned cache can cost is a font loaded twice.
    fn kept(&self) -> std::sync::MutexGuard<'_, Kept> {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// What a previous interpretation of *this* document loaded under `key`.
    fn get(&self, document: &Document, key: &FontKey) -> Option<Font> {
        let mut kept = self.kept();
        kept.bind(document);
        kept.clock = kept.clock.saturating_add(1);
        let clock = kept.clock;
        let found = kept.fonts.get_mut(key).map(|entry| {
            entry.used = clock;
            entry.font.clone()
        });
        if found.is_some() {
            kept.hits = kept.hits.saturating_add(1);
        } else {
            kept.misses = kept.misses.saturating_add(1);
        }
        found
    }

    /// Keeps a loaded font, dropping least-recently-used entries until it fits.
    ///
    /// A program larger than the whole budget is not kept at all, rather than emptying the
    /// cache to hold one entry the next insertion would drop — `pdf_syntax::DecodedStreams`
    /// declines an oversized decode for the same reason, and ADR 0586 argues it.
    fn keep(&self, document: &Document, key: FontKey, font: &Font) {
        let bytes = font.program_bytes();
        let mut kept = self.kept();
        kept.bind(document);
        if bytes > kept.budget {
            return;
        }
        kept.clock = kept.clock.saturating_add(1);
        if let Some(previous) = kept.fonts.remove(&key) {
            kept.bytes = kept.bytes.saturating_sub(previous.bytes);
        }
        while kept.bytes.saturating_add(bytes) > kept.budget {
            if !kept.evict() {
                return;
            }
        }
        kept.bytes = kept.bytes.saturating_add(bytes);
        let used = kept.clock;
        kept.fonts.insert(
            key,
            Entry {
                font: font.clone(),
                bytes,
                used,
            },
        );
    }
}

impl Kept {
    /// Empties the cache where the document is not the one whose objects these keys name.
    ///
    /// **This is what makes an [`ObjectId`] a sound key.** Object number 12 is a different font
    /// in every file, so an entry may be answered only while the cache is bound to the file it
    /// came out of. The binding is the identity of the document's own bytes, and the cache
    /// *holds* that allocation — an address is only a name for as long as something keeps it
    /// from being handed to somebody else, which is `doc/todo/41`'s lesson at 4 KB and ADR
    /// 0317's liveness invariant one crate down.
    ///
    /// A host that opens documents in turn through one cache therefore pays one comparison per
    /// lookup and one emptying per document, which is what `viewer_core`'s `Open` does not do —
    /// it holds a cache per open document, so this never fires there. It fires under a corpus
    /// walk, where it is the whole of what keeps the walk honest.
    fn bind(&mut self, document: &Document) {
        let bytes = document.bytes();
        if self.document.as_ref().is_some_and(|held| held.same(bytes)) {
            return;
        }
        if self.document.is_some() {
            self.rebound = self.rebound.saturating_add(1);
        }
        self.document = Some(bytes.clone());
        self.fonts.clear();
        self.bytes = 0;
    }

    /// Drops the least recently used entry, answering whether there was one.
    ///
    /// A scan rather than a second index ordered by use, for the reason the decoded-stream
    /// cache's own eviction gives: two maps kept in step are more lines and one more thing to
    /// get wrong, and this walks a few dozen `u64`s against a font parse.
    fn evict(&mut self) -> bool {
        let Some(oldest) = self
            .fonts
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(key, _)| key.clone())
        else {
            return false;
        };
        if let Some(entry) = self.fonts.remove(&oldest) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
            self.evicted = self.evicted.saturating_add(1);
        }
        true
    }
}

/// The font a `Tf` names when the resource dictionary defines nothing under that name.
///
/// **A documented choice about a malformed file, and a narrow one.** §7.8.3 requires the writer
/// to supply the resources a stream uses — "a PDF writer shall include a Resources entry in the
/// stream's dictionary specifying the resource dictionary which contains all the resources used
/// by that content stream" — and a file that names `/F1` with nothing behind it has broken that
/// `shall`; nothing here invents a font for it, and the report stands.
///
/// The exception is the fourteen names §9.6.2.2 lists, because for those the standard states what
/// the name means: Table 109 makes `/FirstChar`, `/LastChar`, `/Widths` and `/FontDescriptor`
/// "(Required; optional in PDF 1.0-1.7 for the standard 14 fonts)", so a file may name one and
/// say nothing else about it. The clause used to add that the fonts "shall be available to the
/// PDF processor", and Errata Collection 3 struck that sentence and made its neighbour a NOTE
/// (Issue #47 and #48; [`pdf_font::standard`] carries the reading and ADR 0253 the reason
/// `doc/md/` cannot show it) — which leaves the permission where the work is anyway.
///
/// So a stream whose `Tf` says `/Helvetica` with an empty resource dictionary has named something
/// the standard permits it to name and nothing else, and drawing it from the compiled-in fourteen
/// (ADR 0133) is a better reading of that stream than drawing nothing. `issue17492.pdf` is the
/// witness: a text widget's stored appearance stream carries `/Resources <<>>` and sets its text
/// in `/Helvetica 12 Tf`, `mupdf` and `ghostscript` draw the three lines, `poppler` refuses with
/// *Unknown font tag 'Helvetica'*, and this tree drew nothing and said so.
///
/// **The same argument `variable_text`'s `STANDARD_ABBREVIATIONS` makes**, one clause over and
/// with a stronger premise: there the name is a four-letter convention for one of the fourteen,
/// here it *is* one of the fourteen. `pdf_font::standard::is_standard_name` is deliberately exact
/// — no case folding, no families — so `/F1`, `/Arial` and `/helvetica` still name nothing and
/// still report. Two corpus documents naming `/F1` are unaffected, which is the narrowness
/// visible in the gate rather than argued in a comment.
fn standard_font_named(name: &str) -> Option<Object> {
    if !pdf_font::standard::is_standard_name(name) {
        return None;
    }
    let entry = |key: &str, value: &str| {
        (
            Name::new(key.as_bytes().to_vec()),
            Object::Name(Name::new(value.as_bytes().to_vec())),
        )
    };
    let mut dict = Dictionary::new();
    // The dictionary §9.6.2.2 allows for one of the fourteen: no `/FirstChar`, `/LastChar`,
    // `/Widths` or `/FontDescriptor`, which the same clause makes optional for these and only
    // these, so `pdf-font` reads the metrics from `standard_metrics` and the program from
    // `standard`.
    for (key, value) in [
        entry("Type", "Font"),
        entry("Subtype", "Type1"),
        entry("BaseFont", name),
    ] {
        dict.insert(key, value);
    }
    Some(Object::Dictionary(dict))
}

/// Why a lookup that owed a font dictionary produced none.
///
/// **Two conditions wore one sentence until ADR 0779**, and `Font` was the last resource
/// category folding them together: `XObject` has told "is not in `/XObject`" from "is not a
/// stream" since ADR 0255, and `Shading` says "`/Sh0` is not in `/Shading`". §7.8.3 makes a
/// resource dictionary "enumerate the named resources needed by the operators in the content
/// stream", so a name it does not carry is a resource **the file never defines** — while a name
/// it *does* carry is defined, and what it names being no dictionary is §7.3.10 instead:
///
/// > An indirect reference to an undefined object shall not be considered an error by a PDF
/// > processor; it shall be treated as a reference to the null object.
///
/// The two send a reader to different clauses and to different producers, and the first
/// sentence said of the second is false about the file. `evince-1360-1.pdf` is the witness the
/// eight-hundred-and-fifty-fifth session's chunk produced: a cairo page whose `/Resources
/// /Font` names six fonts by reference, none of whose objects survived the reduction the bug
/// report shipped, reported six times as fonts the file does not name.
#[derive(Debug, Clone, Copy)]
enum Absent {
    /// The resource dictionary states no entry under this name (§7.8.3).
    NoSuchResource,
    /// The entry is stated and what it names is not a font dictionary (§7.3.10, §7.3.9).
    NotAFontDictionary,
}

impl Absent {
    /// What to report, for a font the content stream called `name`.
    ///
    /// Table 57's route names an object rather than a resource, and takes the second arm with
    /// `name` reading `object 6 0`; the sentence is written so that both read.
    fn detail(self, name: &str) -> String {
        match self {
            // Unchanged wording, deliberately: ADR 0255's population is counted by it, and only
            // the documents that were never in it move.
            Self::NoSuchResource => format!("no /Font resource named /{name}"),
            Self::NotAFontDictionary => format!(
                "the /Font entry {name} is stated and is not a font dictionary — §7.3.10 makes \
                 a reference to an object the file does not define the null object, which is \
                 not one"
            ),
        }
    }
}

impl Interpreter<'_> {
    /// Table 57's `/Font`, which is §8.4.5's other route to the two parameters `Tf` sets.
    ///
    /// > An array of the form [ font size ], where font shall be an indirect reference to a
    /// > font dictionary and size shall be a number expressed in text space units. These two
    /// > objects correspond to the operands of the Tf operator (see 9.3, "Text state
    /// > parameters and operators"); however, the first operand shall be an indirect object
    /// > reference instead of a resource name.
    ///
    /// So both text state parameters are set, exactly as `Tf` sets them, and the font is
    /// cached by the object it *is* rather than by a name it has none of. That last point is
    /// the whole reason this took twenty-four sessions: the font cache was keyed by resource
    /// name, so there was nowhere to put a font that has none, and `extgstate.pdf` — whose
    /// page says "I should be courier!" — was reported rather than drawn.
    pub(super) fn apply_ext_gstate_font(&mut self, dict: &Dictionary, state: &mut GraphicsState) {
        let entry = self.document.get_key(dict, "Font");
        let Some(entry) = entry.as_array() else {
            return;
        };
        let reference = entry.first().cloned();
        let size = entry
            .get(1)
            .map(|item| self.document.resolve(item))
            .and_then(|item| item.as_number());
        if let (Some(Object::Reference(id)), Some(size)) = (reference, size) {
            let font_dict = self.document.get(id).as_dict().cloned();
            let name = format!("object {} {}", id.number, id.generation);
            state.text.font = self.load_font(
                Some(FontKey::Referenced(id)),
                font_dict.as_ref(),
                &name,
                Absent::NotAFontDictionary,
            );
            state.text.size = narrow(size);
        } else {
            // A `/Font` this crate cannot read as the clause states it is reported rather
            // than half-applied: a size without a font would move every glyph the page
            // draws afterwards.
            self.note(Unsupported::Font {
                detail: "Table 57's /Font is not [indirect-reference size]".to_owned(),
            });
        }
    }

    /// Loads a font by resource name, caching the result including failures.
    ///
    /// A failure is cached too: a page that names an unloadable font on every `Tf` should
    /// pay for the attempt once, and should report it once.
    pub(super) fn font(&mut self, resources: &Dictionary, name: &Name) -> Option<Font> {
        // **Keyed by the font's identity, never by the name the stream used.** A resource name
        // is scoped to the resource dictionary that defines it, and §8.10.1 gives a form
        // `XObject` a `/Resources` of its own — so a page's `/F1` and a form's `/F1` are two
        // fonts as often as they are one, and a cache keyed by `F1` hands the second the
        // first's glyphs with nothing reported. That is trap 1's archetype, and it is what this
        // cache did for thirty-one sessions. `shading::Cache` had the same question and the
        // same answer (see `resource_entry`, whose whole reason for existing is this one).
        // §9.6.2.2's fourteen are ASCII names, so a resource name that is not text cannot be one
        // of them and `as_str` returning `None` is that answer rather than a lost lookup.
        let entry = self
            .resource_entry(resources, "Font", name)
            .or_else(|| name.as_str().and_then(standard_font_named));
        let key = entry
            .as_ref()
            .and_then(Object::as_reference)
            .map(FontKey::Referenced);

        // **Ask the cache before paying for what a load would need.** Everything below this
        // point exists only for a load: resolving the reference copies the font dictionary out
        // of `Document`'s cache, and `/Widths` alone is an array of up to 256 numbers. A page
        // states `Tf` once per text object and names the same few fonts throughout — 280 times
        // for seven fonts on page 101 of ISO 32000-2 — so the copy was made 273 times for a
        // load that did not happen. `load_font` asks again, because it is the authority and
        // Table 57's route reaches it without coming through here.
        if let Some(cached) = self.cached_font(key.as_ref()) {
            return cached;
        }

        let label = String::from_utf8_lossy(name.as_bytes());
        // Which of §7.8.3's two failures this is has to be decided *here*, because it is the
        // only place that still holds the unresolved entry: a lookup that found nothing and one
        // that found a reference to an object the file never wrote both arrive at `load_font`
        // as `None`.
        let absent = if entry.is_some() {
            Absent::NotAFontDictionary
        } else {
            Absent::NoSuchResource
        };
        let resolved = entry.map(|object| self.document.resolve(&object));
        self.load_font(
            key,
            resolved.as_ref().and_then(Object::as_dict),
            &label,
            absent,
        )
    }

    /// What a previous load left under `key`, where there was one.
    ///
    /// The cache holds a *failure* as well as a font, so the answer is two levels deep: the
    /// outer `None` means nothing has been loaded under this key and the inner one means a load
    /// was tried and did not produce a font. Collapsing them would make a page that names an
    /// unloadable font on every `Tf` re-attempt the load and re-report it, which is what the
    /// note on [`Self::load_font`] is about.
    #[expect(
        clippy::option_option,
        reason = "the two levels are the two answers: nothing is cached under this key, and a \
                  load under it produced no font. The lint is right that they are usually one \
                  question wearing two shapes, and here they are two questions"
    )]
    fn cached_font(&self, key: Option<&FontKey>) -> Option<Option<Font>> {
        let key = key?;
        if let Some(found) = self.fonts.get(key).cloned() {
            return Some(found);
        }
        // **The second level, and it is the one that outlives this page.** Only a *successful*
        // load is there — see [`FontCache`] for why keeping a failure would change the second
        // page's reports rather than only its cost — so a miss here falls through to the load,
        // which is exactly what an uncached font does.
        Some(Some(self.across.get(self.document, key)?))
    }

    /// Loads a font, caching it under `key`, which is what `Tf` and Table 57's `/Font` share.
    ///
    /// §8.4.1's NOTE 1 gives most graphics state parameters two routes, and this is the one
    /// where the two do not name the same thing: `Tf` names a *resource*, and Table 57's
    /// `/Font` is "an indirect reference to a font dictionary" instead. A cache keyed only by
    /// the resource name therefore had nowhere to put the second, which is why one corpus
    /// document's `/ExtGState` font was reported rather than loaded for twenty-four sessions.
    /// `key` is `None` for a resource dictionary that states its font *directly* rather than
    /// by reference. Such a font has no identity to key on and is therefore loaded afresh each
    /// time — correctness before speed, and the case is rare enough that no corpus document
    /// reaches it: every one of the 974 states its fonts indirectly, counted.
    fn load_font(
        &mut self,
        key: Option<FontKey>,
        dict: Option<&Dictionary>,
        name: &str,
        absent: Absent,
    ) -> Option<Font> {
        if let Some(cached) = self.cached_font(key.as_ref()) {
            return cached;
        }

        let loaded = dict.map(|dict| pdf_font::LoadedFont::load(self.document, dict, name));

        let result = match loaded {
            Some(Ok(font)) => Some(Font::Program(Arc::new(font))),
            // A Type 3 font has no program for `pdf-font` to read: its glyphs are content
            // streams, so it is this crate that draws them (§9.6.4). The refusal there is
            // the hand-off rather than a failure, which is why this is not a report.
            Some(Err(pdf_font::FontError::Type3 { .. })) => {
                match dict.map(|dict| crate::type3::Type3Font::read(self.document, dict, name)) {
                    Some(Ok(font)) => Some(Font::Type3(Arc::new(font))),
                    Some(Err(error)) => {
                        self.note(Unsupported::Font {
                            detail: error.to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }
            Some(Err(error)) => {
                self.note(Unsupported::Font {
                    detail: error.to_string(),
                });
                None
            }
            None => {
                self.note(Unsupported::Font {
                    detail: absent.detail(name),
                });
                None
            }
        };

        if let Some(key) = key {
            if let Some(font) = result.as_ref() {
                self.across.keep(self.document, key.clone(), font);
            }
            self.fonts.insert(key, result.clone());
        }
        result
    }
}
