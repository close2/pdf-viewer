//! What a person gets when they drag across a page, counted over the whole corpus.
//!
//! ADR 0323's instrument 1, **composed half**, at the population its geometry half already runs
//! over. `pdf-model`'s `text_extraction` binary judges where this tree's text layer *says* the
//! words are, in the page's own points, against `pdftotext -bbox -cropbox`. Nothing judged the
//! journey from there: device pixels in, selected text out — the loop from a press to a
//! `Command` to an `Answer` that trap 12a is about, where `user_space_at`'s doc comment named a
//! coordinate space it did not have and **every click followed that sentence into the mirror of
//! the point it meant**, for seventy-five sessions, because no gate clicks (ADR 0118).
//!
//! One drag test has clicked since ADR 0333: one committed document, three words, at the page's
//! own point size where the magnification is 1 and the origin is 0. This census is that test's
//! population — every corpus document, at a **fitted** magnification with the page centred in a
//! viewport, so the origin and the scale are the ones a window actually has.
//!
//! # The three properties, and what each is judged against
//!
//! | | property | judged against |
//! |---|---|---|
//! | **the drag** | a drag across poppler's word box selects that word | `pdftotext -bbox -cropbox` |
//! | **the find** | [`Command::Find`] for a word poppler states once leaves that word selected | `pdftotext -bbox -cropbox` |
//! | **the readback** | [`Selection::All`]'s text is [`pdf_model::Interpretation::text`], byte for byte | the interpreter, read beside the boundary |
//! | **the caret** | [`Query::Offset`] of [`Query::Caret`]'s own point is that offset again | itself: the pair is documented as inverse |
//!
//! **The find is here because a search ends in a selection**, which is not an aside: §O.2.2's
//! `search` says "selecting the first matching word in the document", and the one thing this
//! crate has that means is the range [`Query::Selection`] answers with. So the find bar's loop
//! ends where the drag's does, on the same question, and this census is where both are asked at
//! corpus scale. It also asks the *cost* half, which no instrument asked before the
//! nine-hundred-and-thirty-second session: a search step reads a page out of
//! `viewer_core`'s readback cache where one is held, and the page a person is looking at was
//! interpreted to be drawn — so a find bar opened on the page showing must interpret **no page
//! at all**. The cache's own counters say whether that happened, and they are counts rather than
//! clocks, so a neighbouring round's load cannot move either of them by one (ADR 0905).
//!
//! The first two are the ones with an independent judge, and the endpoints of every drag come from
//! **poppler's** box rather than from this tree's geometry — trap 12a's own rule, that a test
//! needing a point takes it from the document rather than from the code under test. A viewer
//! that flipped y would drag across the mirror of the word, and the mirror of a word is not the
//! word.
//!
//! The other two need no reference and ADR 0323 says so: no other implementation states
//! §12.7.4.3's field layout, and inventing one to disagree with would be curve-fitting with
//! extra steps. What they pin is a *relation* — that the selection path hands back the
//! interpreter's own bytes rather than a tidied copy of them, which is the byte-for-byte
//! discipline `pdf-retrieve`'s default answer is held to (ADR 0257); and that a click into a
//! value can put the caret where the click was, which is the whole of what a person means by
//! clicking into a word (ADR 0225).
//!
//! # Which words are dragged across
//!
//! The three longest words that occur **exactly once in poppler's answer and exactly once in
//! ours**. Uniqueness on both sides is what makes finding the text prove the *place*; requiring
//! it on ours as well is Finding 5's convention — segmentation is each extractor's own, and a
//! word we read differently from poppler is a disagreement about *characters*, which the text
//! gates already measure and this instrument must not count twice.
//!
//! # The denominator, and every exclusion by name
//!
//! Page one of every document in `doc/pdf.js/test/pdfs` — the same population and the same
//! cached `pdftotext` invocation the geometry half uses, so the two share every cache entry.
//! A document is refused, by reason, when poppler will not read it, when its stated page size is
//! not §7.7.3.3's crop box (ADR 0323's Finding 1: the flag decides which box, and a mismatch
//! means the reference read a different page than we drew), when §12.2's `/ViewArea` moves the
//! displayed box, and when the page states a `/Rotate` or a `/UserUnit` — those last two are
//! judged by the geometry half, whose `Frame` normalises both, and are refused here rather than
//! carrying a second copy of that normalisation into a second crate. Every refusal is a document
//! off the judged set, which is trap 11's arithmetic and is printed as such.
//!
//! # What is asserted, and what is only printed
//!
//! ADR 0323's rule — an instrument's numbers enter `doc/todo/02` §2 only once they have held
//! across rounds — governs the drag *fraction*, which is printed. The other two properties are
//! **exact** and are asserted outright: they hold over the whole population on the first run, and
//! a property that holds everywhere is stronger stated as a property than as a count.
//!
//! ```text
//! cargo test --profile gates -p viewer-core --test selection_census -- --ignored --nocapture
//! ```

#![expect(
    clippy::print_stdout,
    reason = "a census whose whole output is what it counted"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use pdf_syntax::{Document, Limits, SyntaxError};
use pdfref::{ExtractionCache, ExtractionError, Extractor};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use viewer_core::{
    Answer, Command, DocumentId, Find, FindDirection, PointerAction, Query, Selection, Viewer,
};

/// The document handle every open uses; one viewer per document, so one identity suffices.
const DOCUMENT: DocumentId = DocumentId(1);

/// The viewport every drag is aimed into, in logical pixels.
///
/// Deliberately **not** the page's own point size, which is what the single-document drag test
/// resizes to: at that size the magnification is 1 and the origin is 0, so two thirds of the
/// mapping a host uses are the identity. A page fitted into this viewport is centred and scaled,
/// which is the arithmetic ADR 0118 is about.
const VIEWPORT: (u32, u32) = (800, 1000);

/// How many words are dragged across per document.
///
/// Three, as ADR 0333's single-document test drags three: the property is about the *mapping*,
/// which is one arithmetic for the whole page, so a fourth word buys a repetition rather than a
/// case. What a larger number would buy is per-document detail on a page that fails, and the
/// failing documents are named for exactly that.
const WORDS_PER_DOCUMENT: usize = 3;

/// The shortest word this census will drag across.
///
/// A word of three characters or fewer is unique on far fewer pages, and a short reference word
/// that happens to be a substring of a longer selected one would pass without proving anything.
const MIN_WORD_CHARS: usize = 4;

/// How much of a point the drag reaches past each end of the reference's box.
///
/// The press must land inside the first glyph and the release inside the last, and
/// `select::position_at` snaps to the nearer edge of the nearest glyph — so a point two points
/// outside the box is still nearest that glyph's outer edge and cannot reach the next word,
/// whose own box begins a space away. The single-document test uses the same two points.
const DRAG_OVERSHOOT: f32 = 2.0;

/// How many witnesses of one class are printed before the rest are summarised.
const WITNESSES: usize = 30;

/// How many steps one search is pumped for before it is given up on.
///
/// A search here is for a word that occurs exactly once on the page it begins on, so one step is
/// the whole of it and two is the margin. This is deliberately **not** a document-length sweep:
/// a needle that is not answered on the first page is a witness this instrument wants to print,
/// not a reason to interpret a thousand pages.
const FIND_STEPS: usize = 2;

/// The corpus documents that refuse §7.6.4.1's default user password, with the password each
/// one's own pdf.js issue records.
///
/// The same list `accessibility_census.rs`, `save_round_trip.rs` and `pdf-syntax`'s
/// `encryption.rs` carry: test binaries share no code, and ADR 0323's denominator rule is "what
/// opens without a password or with the corpus's known ones".
const KNOWN_PASSWORDS: &[(&str, &str)] = &[
    ("issue15893_reduced.pdf", "test"),
    ("issue3371.pdf", "ELXRTQWS"),
    ("bug1782186.pdf", "Hello"),
    ("issue6010_1.pdf", "abc"),
    ("issue6010_2.pdf", "\u{E6}\u{F8}\u{E5}"),
    ("saslprep-r6.pdf", "S\u{AA}SL\u{AD}prep"),
    ("pr6531_1.pdf", "asdfasdf"),
    ("print_protection.pdf", "1234"),
];

/// A word and its box as `pdftotext -bbox -cropbox` states it: points, origin at the displayed
/// page's top-left corner, y growing down.
struct ReferenceWord {
    text: String,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

/// Why a document is off the judged set; every one is printed with its count.
type Refusal = &'static str;

/// What one document contributed, and what the population's totals are folded into.
#[derive(Default)]
struct Census {
    /// Documents examined, and those the instrument could not judge, by reason.
    documents: usize,
    refused: Vec<(String, Refusal)>,

    /// Documents with at least one word both sides state exactly once.
    dragged_documents: usize,
    /// Words dragged across, and words the drag selected.
    words: usize,
    selected: usize,
    /// A drag that did not select the word under it, with what it selected instead.
    missed: Vec<(String, String)>,

    /// Documents where a word was searched for, and searches that left it selected.
    searched_documents: usize,
    finds: usize,
    found: usize,
    /// A search that did not leave the word it was given selected, with what it left instead.
    not_found: Vec<(String, String)>,
    /// Lookups the readback cache answered without interpreting a page, over the whole
    /// population, and searches that interpreted a page the page turn had already read.
    find_hits: u64,
    reinterpreted: Vec<(String, String)>,

    /// Documents where [`Selection::All`] was compared with the interpreter's own readback.
    readbacks: usize,
    /// One where the two differ, which is the selection path putting itself between a host and
    /// the readback.
    readback_differs: Vec<(String, String)>,

    /// §12.7 text fields whose caret was walked, and the offsets asked of each.
    fields: usize,
    offsets: usize,
    /// Offsets whose caret point another offset's caret shares exactly — see
    /// [`walk_the_carets`] on why that is a fact about a value rather than a defect.
    shared_points: usize,
    /// A point [`Query::Caret`] answered with that [`Query::Offset`] did not turn back into the
    /// offset that produced it.
    not_inverse: Vec<(String, String)>,

    /// A document whose examination panicked, which principle 1 forbids.
    panicked: Vec<(String, String)>,
}

impl Census {
    /// Folds one document's census into the population's.
    fn absorb(&mut self, from: Self) {
        self.documents = self.documents.saturating_add(from.documents);
        self.refused.extend(from.refused);
        self.dragged_documents = self
            .dragged_documents
            .saturating_add(from.dragged_documents);
        self.words = self.words.saturating_add(from.words);
        self.selected = self.selected.saturating_add(from.selected);
        self.missed.extend(from.missed);
        self.searched_documents = self
            .searched_documents
            .saturating_add(from.searched_documents);
        self.finds = self.finds.saturating_add(from.finds);
        self.found = self.found.saturating_add(from.found);
        self.not_found.extend(from.not_found);
        self.find_hits = self.find_hits.saturating_add(from.find_hits);
        self.reinterpreted.extend(from.reinterpreted);
        self.readbacks = self.readbacks.saturating_add(from.readbacks);
        self.readback_differs.extend(from.readback_differs);
        self.fields = self.fields.saturating_add(from.fields);
        self.offsets = self.offsets.saturating_add(from.offsets);
        self.shared_points = self.shared_points.saturating_add(from.shared_points);
        self.not_inverse.extend(from.not_inverse);
        self.panicked.extend(from.panicked);
    }
}

/// Every document this instrument counts: the pdf.js corpus, page one of each.
///
/// The specifications in `doc/` are deliberately **not** in it, unlike the accessibility
/// census's population: this instrument's cache entries are keyed by the invocation and the
/// document, so sharing exactly the geometry half's population means sharing exactly its cached
/// `pdftotext` answers and asking poppler nothing new.
fn population() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    files.sort();
    files
}

/// The password on record for one file, or the empty default §7.6.4.1 starts with.
fn password_for(name: &str) -> &'static str {
    KNOWN_PASSWORDS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or("", |(_, password)| *password)
}

/// The cache the reference's answers are remembered in — `pdfref`'s own, one level down.
///
/// The same root and the same `extraction/` level `pdf-model`'s geometry instrument uses, so the
/// two instruments share every entry: the key is the invocation, the extractor's version and the
/// document's SHA-256, and both ask `pdftotext -bbox -cropbox` for page 1.
fn extraction_cache() -> ExtractionCache {
    let default = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("pdfref-cache");
    let root = match std::env::var("PDFREF_CACHE") {
        Ok(value) if value.trim() == "off" => return ExtractionCache::disabled(),
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => default,
    };
    ExtractionCache::at(root.join("extraction"))
}

/// One `name="number"` attribute of one of `pdftotext`'s XML tags.
fn bbox_number(tag: &str, name: &str) -> Option<f32> {
    tag.split_once(&format!("{name}=\""))?
        .1
        .split('"')
        .next()?
        .parse()
        .ok()
}

/// Parses `pdftotext -bbox`'s XHTML: the page's stated size, and one box per word.
///
/// Words carrying an XML entity are dropped rather than unescaped: this census drags across
/// alphanumeric words only, so an escaped one would be filtered out a step later anyway, and a
/// second unescaper in a third file would be a copy nobody maintains.
fn poppler_words(html: &str) -> Option<(f32, f32, Vec<ReferenceWord>)> {
    let (mut width, mut height) = (0.0, 0.0);
    let mut words = Vec::new();
    for line in html.lines() {
        let line = line.trim_start();
        if let Some(open) = line.strip_prefix("<page ") {
            width = bbox_number(open, "width")?;
            height = bbox_number(open, "height")?;
        } else if let Some(open) = line.strip_prefix("<word ") {
            let text = open.split_once('>')?.1.strip_suffix("</word>")?;
            words.push(ReferenceWord {
                text: text.to_owned(),
                x0: bbox_number(open, "xMin")?,
                y0: bbox_number(open, "yMin")?,
                x1: bbox_number(open, "xMax")?,
                y1: bbox_number(open, "yMax")?,
            });
        }
    }
    Some((width, height, words))
}

/// The document's own frame, or the refusal naming why this census will not judge it.
///
/// §7.7.3.3's crop box in points, checked against §12.2's displayed box. `/Rotate` and
/// `/UserUnit` are refusals rather than normalisations here — see the module comment.
fn frame_of(page: &pdf_model::Page) -> Result<(f32, f32), Refusal> {
    if page
        .display_box
        .iter()
        .zip(&page.crop_box)
        .any(|(view, crop)| (view - crop).abs() > 0.01)
    {
        return Err("a /ViewArea departs from the crop box");
    }
    if page.rotate != 0 {
        return Err("the page states a /Rotate");
    }
    if (page.user_unit - 1.0).abs() > f32::EPSILON {
        return Err("the page states a /UserUnit");
    }
    Ok((
        page.crop_box[2] - page.crop_box[0],
        page.crop_box[3] - page.crop_box[1],
    ))
}

/// The text with every whitespace character removed.
///
/// Both sides of the drag comparison are stripped: §9.3's spacing heuristics decide where this
/// tree puts a space and poppler's own decide where it puts one, and that disagreement is the
/// text gates' subject rather than this instrument's.
fn without_whitespace(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// One document: opened at the boundary, asked at the boundary, and read beside it.
#[expect(
    clippy::too_many_lines,
    reason = "one document's whole examination — open, readback, caret, drag — and splitting it \
              would separate each property from the refusal that excuses it"
)]
fn examine(path: &Path, cache: &ExtractionCache, work_dir: &Path) -> Census {
    let mut census = Census::default();
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let Ok(bytes) = std::fs::read(path) else {
        return census;
    };
    census.documents = 1;
    let password = password_for(&name);

    // The document, read *beside* the boundary: this is where the readback property's other
    // side comes from — `interpret` over the same page, with no viewer in the way.
    let document = match Document::open_with_password(bytes.clone(), Limits::DEFAULT, password) {
        Ok(document) => document,
        Err(SyntaxError::PasswordRequired) => {
            census
                .refused
                .push((name, "needs a password nobody has recorded"));
            return census;
        }
        Err(_) => {
            census.refused.push((name, "unopenable to this tree"));
            return census;
        }
    };
    let pages = pdf_model::Pages::new(&document);
    let Some(page) = pages.get(0) else {
        census.refused.push((name, "no page one"));
        return census;
    };

    let mut viewer = Viewer::new(VIEWPORT.0, VIEWPORT.1, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: (!password.is_empty()).then(|| password.to_owned().into()),
            fragment: None,
        })
        .any(|event| matches!(event, viewer_core::Event::Opened { .. }));
    if !opened {
        census
            .refused
            .push((name, "the boundary did not open what pdf-syntax did"));
        return census;
    }

    // Property 2, over every document that opens: what a host is handed when a person presses
    // *select all* is the interpreter's own readback and not a copy something tidied.
    let interpretation = pdf_model::interpret(&document, &page);
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    census.readbacks = 1;
    match viewer.query(Query::Selection) {
        Answer::Selected(selection) if selection.text == interpretation.text => {}
        Answer::Selected(selection) => census.readback_differs.push((
            name.clone(),
            format!(
                "the selection is {} byte(s) and the interpretation {}",
                selection.text.len(),
                interpretation.text.len()
            ),
        )),
        // A page with no text at all selects nothing, and the interpreter reads back nothing:
        // the two agree, and there is no string to compare.
        _ if interpretation.text.is_empty() => {}
        other => census.readback_differs.push((
            name.clone(),
            format!("selecting everything answered {other:?} over a non-empty readback"),
        )),
    }
    viewer
        .handle(Command::Select(Selection::None))
        .for_each(drop);

    // Property 3, over every text field on page one.
    walk_the_carets(&mut census, &viewer, &name);

    // Property 1: the drag, judged against poppler.
    let frame = match frame_of(&page) {
        Ok(frame) => frame,
        Err(reason) => {
            census.refused.push((name, reason));
            return census;
        }
    };
    let reference = match cache.extract(Extractor::PopplerBoxes, path, 1, work_dir) {
        Ok(text) => text,
        Err(ExtractionError::TimedOut { .. }) => {
            census.refused.push((name, "pdftotext exceeded its budget"));
            return census;
        }
        Err(_) => {
            census
                .refused
                .push((name, "pdftotext refused the document"));
            return census;
        }
    };
    let Some((width, height, words)) = poppler_words(&reference) else {
        census.refused.push((name, "pdftotext produced no page"));
        return census;
    };
    // ADR 0323's Finding 1, in the terms a host has: `-cropbox` makes poppler answer in
    // §7.7.3.3's crop box, so a stated size that is not the crop box means the reference read a
    // different page than this viewer drew, and every point mapped across would be displaced.
    if (width - frame.0).abs() > 1.0 || (height - frame.1).abs() > 1.0 {
        census
            .refused
            .push((name, "frame mismatch against pdftotext"));
        return census;
    }
    if words.is_empty() {
        census.refused.push((name, "no words in the reference"));
        return census;
    }
    let dragged = drag_across_the_reference(&mut census, &mut viewer, &name, &words);
    // Property 4: the same words, through the find bar rather than through the pointer.
    search_for_the_reference(&mut census, &mut viewer, &name, &dragged);
    census
}

/// Property 3: [`Query::Offset`] of [`Query::Caret`]'s own point, for every text field on the page.
///
/// The point naming the field is the centre of the widget's quadrilateral, which
/// [`Answer::Fields`] states in device pixels — so the field is named by the *document's*
/// `/Rect` through the boundary, and the offsets are into the value the same answer carries.
///
/// # The inverse a value can make impossible, and what is asserted instead
///
/// [`Query::Caret`] is not injective, and no arithmetic can make it so: where a glyph's advance
/// is **zero** — a `/Widths` entry of 0, a code the `/DA` font gives no advance — several offsets
/// share one point on the line, and a point cannot name all of them. So what is counted as a
/// defect is not `offset(caret(o)) != o` but `caret(offset(caret(o))) != caret(o)`: the round
/// trip must land on the *same place*, which is the whole of what a host needs when it puts the
/// cursor where the click was. An offset whose point another offset shares is counted apart, by
/// name, rather than silently tolerated — it is the population the first form of the property
/// would have failed on, and it is one corpus document.
fn walk_the_carets(census: &mut Census, viewer: &Viewer, name: &str) {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        return;
    };
    for field in fields {
        let Some(value) = field.value else { continue };
        let Some(widget) = field.widgets.first() else {
            continue;
        };
        let at = (
            (widget.quad[0] + widget.quad[2] + widget.quad[4] + widget.quad[6]) / 4.0,
            (widget.quad[1] + widget.quad[3] + widget.quad[5] + widget.quad[7]) / 4.0,
        );
        // Every character boundary of the value, and the position after the last one — which is
        // where a person typing into an empty field puts the first character.
        let boundaries = value
            .text
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(value.text.len()));
        let mut asked = 0usize;
        for offset in boundaries {
            let Answer::Caret { from, to } = viewer.query(Query::Caret { at, offset }) else {
                continue;
            };
            asked = asked.saturating_add(1);
            let point = (from.0, f32::midpoint(from.1, to.1));
            let Answer::Offset(back) = viewer.query(Query::Offset { at, point }) else {
                census.not_inverse.push((
                    name.to_owned(),
                    format!("{}: the caret's own point named no offset", field.partial),
                ));
                continue;
            };
            if back == offset {
                continue;
            }
            // Not the same offset: either the two offsets are the same *place* — a zero-advance
            // glyph between them — or the round trip moved the caret, which is the defect.
            match viewer.query(Query::Caret { at, offset: back }) {
                Answer::Caret { from: again, .. }
                    if (again.0 - from.0).abs() < 0.001 && (again.1 - from.1).abs() < 0.001 =>
                {
                    census.shared_points = census.shared_points.saturating_add(1);
                }
                _ => census.not_inverse.push((
                    name.to_owned(),
                    format!(
                        "{}: the caret at {offset} is at {point:?}, which names {back}, whose \
                         own caret is elsewhere",
                        field.partial
                    ),
                )),
            }
        }
        if asked > 0 {
            census.fields = census.fields.saturating_add(1);
            census.offsets = census.offsets.saturating_add(asked);
        }
    }
}

/// Property 1: a drag across poppler's box for a word selects that word.
///
/// The endpoints are poppler's and the mapping is [`Answer::Geometry`]'s — origin, magnification
/// and the y flip, exactly what a host composes. Nothing in the drag comes from this tree's own
/// text geometry, which is what makes the mirror of trap 12a fail it.
///
/// Answers with the words it dragged across, which is what [`search_for_the_reference`] searches
/// for: the two properties are about one word list read two ways, and deriving it twice would be
/// two chances for them to disagree about which words the document states once.
fn drag_across_the_reference(
    census: &mut Census,
    viewer: &mut Viewer,
    name: &str,
    words: &[ReferenceWord],
) -> Vec<String> {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        census
            .refused
            .push((name.to_owned(), "the page on the screen has no geometry"));
        return Vec::new();
    };
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    let ours = match viewer.query(Query::Selection) {
        Answer::Selected(selection) => without_whitespace(&selection.text),
        _ => String::new(),
    };
    viewer
        .handle(Command::Select(Selection::None))
        .for_each(drop);

    // Unique on both sides — see the module comment on why ours counts too.
    let mut unique: Vec<&ReferenceWord> = words
        .iter()
        .filter(|word| {
            word.text.chars().all(char::is_alphanumeric)
                && word.text.chars().count() >= MIN_WORD_CHARS
                // A word whose box is taller than it is wide was set down the page — §9.4.4's
                // vertical writing mode, or a text matrix that turned it — and a drag along x
                // is then the wrong gesture rather than a wrong answer. Trap 3 in miniature:
                // ask what question the instrument is asking before reading its verdict.
                && word.x1 - word.x0 > word.y1 - word.y0
                && words.iter().filter(|other| other.text == word.text).count() == 1
                && ours.matches(word.text.as_str()).count() == 1
        })
        .collect();
    unique.sort_by_key(|word| std::cmp::Reverse(word.text.len()));
    unique.truncate(WORDS_PER_DOCUMENT);
    if unique.is_empty() {
        census
            .refused
            .push((name.to_owned(), "no word both sides state exactly once"));
        return Vec::new();
    }
    census.dragged_documents = 1;
    let dragged: Vec<String> = unique.iter().map(|word| word.text.clone()).collect();

    for word in unique {
        // Poppler's frame is y-down from the displayed page's top-left, which is the raster's own
        // orientation: a point in it reaches the viewport through the origin and the
        // magnification alone, and the flip back into PDF's y-up space is the viewer's — the one
        // under test.
        let device = |x: f32, y: f32| {
            (
                geometry.origin.0 + x * geometry.scale,
                geometry.origin.1 + y * geometry.scale,
            )
        };
        let mid = f32::midpoint(word.y0, word.y1);
        let start = device(word.x0 - DRAG_OVERSHOOT, mid);
        let end = device(word.x1 + DRAG_OVERSHOOT, mid);
        for (at, action) in [
            (start, PointerAction::Pressed),
            (end, PointerAction::Dragged),
            (end, PointerAction::Released),
        ] {
            viewer
                .handle(Command::Pointer { at, action })
                .for_each(drop);
        }
        census.words = census.words.saturating_add(1);
        let selected = match viewer.query(Query::Selection) {
            Answer::Selected(selection) => without_whitespace(&selection.text),
            _ => String::new(),
        };
        if selected.contains(word.text.as_str()) {
            census.selected = census.selected.saturating_add(1);
        } else {
            census.missed.push((
                name.to_owned(),
                format!(
                    "{:?} at {start:?}..{end:?} selected {:?}",
                    word.text,
                    selected.chars().take(60).collect::<String>()
                ),
            ));
        }
        viewer
            .handle(Command::Select(Selection::None))
            .for_each(drop);
    }
    dragged
}

/// Property 4: a search for a word poppler states once leaves that word selected, and reads it
/// out of the readback the page turn already made.
///
/// # Why the needle is filtered a second time
///
/// [`drag_across_the_reference`]'s words are unique in the **whitespace-stripped** readback,
/// because §9.3's spacing heuristics decide where each extractor puts a space and that
/// disagreement is the text gates' subject rather than this instrument's. A search runs over the
/// readback as it stands, so a word is searched for only where it also occurs exactly once in
/// the untouched string: a word the two extractors break differently is left **out of the
/// population** rather than counted as a search that failed, which is trap 11's arithmetic and is
/// why the count of searches is printed beside the count of documents.
///
/// # The cost half, and why it is a count rather than a clock
///
/// A search step reads one page, and `viewer_core`'s readback cache is what stops it
/// interpreting a page twice — a page placed on the screen puts its readback there, so a find bar
/// opened on the page a person is looking at must interpret **no page at all**. What is asserted
/// is exactly that: the cache's *miss* counter does not move across the searches. It is a count
/// of interpretations rather than a duration, so a neighbouring round's load cannot change it by
/// one, which is `doc/todo/02` §2's rule about which cost properties are worth gating.
///
/// **Before the nine-hundred-and-thirty-second session no instrument reached that cache at all**:
/// this census's forty caret queries left it at `hits: 0, misses: 0`, because nothing here asked
/// a question that searches (ADR 0905). The five-page fixture in `tests/headless.rs` held the
/// cache's own rules; the corpus held nothing.
fn search_for_the_reference(
    census: &mut Census,
    viewer: &mut Viewer,
    name: &str,
    words: &[String],
) {
    // The readback as it stands, which is what a search matches against — not the stripped copy
    // the drag compares.
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    let ours = match viewer.query(Query::Selection) {
        Answer::Selected(selection) => selection.text.into_owned(),
        _ => String::new(),
    };
    viewer
        .handle(Command::Select(Selection::None))
        .for_each(drop);
    let needles: Vec<&String> = words
        .iter()
        .filter(|word| ours.matches(word.as_str()).count() == 1)
        .collect();
    if needles.is_empty() {
        return;
    }
    let Some(before) = viewer.readback_cache(DOCUMENT) else {
        return;
    };
    census.searched_documents = 1;

    for needle in needles {
        census.finds = census.finds.saturating_add(1);
        let found = run_the_search(viewer, needle);
        let selected = match viewer.query(Query::Selection) {
            Answer::Selected(selection) => selection.text.into_owned(),
            _ => String::new(),
        };
        if found.is_some_and(|found| found.page == 0) && folded(&selected) == folded(needle) {
            census.found = census.found.saturating_add(1);
        } else {
            census.not_found.push((
                name.to_owned(),
                format!(
                    "{needle:?} was answered {found:?} and left {:?} selected",
                    selected.chars().take(60).collect::<String>()
                ),
            ));
        }
        // A search starts after the far end of what is selected, so leaving the last answer in
        // place would make the next search a different question.
        viewer
            .handle(Command::Select(Selection::None))
            .for_each(drop);
    }

    let Some(after) = viewer.readback_cache(DOCUMENT) else {
        return;
    };
    census.find_hits = after.hits.saturating_sub(before.hits);
    if after.misses > before.misses {
        census.reinterpreted.push((
            name.to_owned(),
            format!(
                "{} page(s) were interpreted for a search that began on the page showing",
                after.misses.saturating_sub(before.misses)
            ),
        ));
    }
}

/// Unicode's own simple lower-casing, which is the rule `select::find` matches by.
///
/// The needle comes from poppler and the answer is a slice of *our* readback, so comparing them
/// byte for byte would count a correct answer as a miss on every page whose word is capitalised
/// where the reference states it otherwise — nine documents of the corpus, `"Profitability"`
/// against `"profitability"` and `"abcdefghijklmnopqrstuvwxyz"` against its upper case. Case
/// folding is documented as "the only judgement in" that function, so the property is judged
/// under it rather than against it.
fn folded(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// Drives one search to its answer the way a find bar does.
///
/// A word this function is given occurs exactly once on the page the search begins on, so the
/// first step is the answer; [`FIND_STEPS`] is a ceiling rather than a plan, and reaching it is a
/// witness printed by the caller rather than a sweep of the document.
fn run_the_search(viewer: &mut Viewer, needle: &str) -> Option<viewer_core::Found> {
    let mut events: Vec<viewer_core::Event> = viewer
        .handle(Command::Find(Find::Start {
            needle: needle.to_owned(),
            direction: FindDirection::Forward,
        }))
        .collect();
    for _ in 0..FIND_STEPS {
        let mut remaining = 0;
        let mut answer = None;
        for event in &events {
            if let viewer_core::Event::Searched {
                found,
                remaining: left,
                ..
            } = event
            {
                remaining = *left;
                answer = *found;
            }
        }
        if answer.is_some() || remaining == 0 {
            return answer;
        }
        events = viewer.handle(Command::Find(Find::Continue)).collect();
    }
    None
}

/// Prints one class of witness, capped, with its length.
fn print_witnesses(what: &str, entries: &[(String, String)]) {
    println!("  {what}: {}", entries.len());
    for (name, why) in entries.iter().take(WITNESSES) {
        println!("    {name}: {why}");
    }
    if entries.len() > WITNESSES {
        println!("    … and {} more", entries.len().saturating_sub(WITNESSES));
    }
}

/// Prints the whole census, in the order the properties were argued for.
fn report(census: &Census, files: usize, seconds: f64) {
    #[expect(
        clippy::cast_precision_loss,
        reason = "corpus word counts are far below f64's exact integer limit"
    )]
    let percentage = |part: usize, whole: usize| part as f64 / whole.max(1) as f64 * 100.0;

    println!("{files} documents in {seconds:.1}s: page one of each");
    println!(
        "the drag (poppler's box → device pixels → Command::Pointer → Query::Selection): \
         {}/{} words selected ({:.2}%) over {} documents",
        census.selected,
        census.words,
        percentage(census.selected, census.words),
        census.dragged_documents,
    );
    print_witnesses(
        "a drag that did not select the word under it",
        &census.missed,
    );
    println!(
        "the find (Command::Find → Query::Selection): {}/{} words selected ({:.2}%) over {} \
         documents, {} lookups answered out of the readback cache",
        census.found,
        census.finds,
        percentage(census.found, census.finds),
        census.searched_documents,
        census.find_hits,
    );
    print_witnesses(
        "a search that did not leave the word it was given selected",
        &census.not_found,
    );
    print_witnesses(
        "a search that interpreted a page the page turn had already read",
        &census.reinterpreted,
    );
    println!(
        "the readback (Selection::All against Interpretation::text): {} documents compared",
        census.readbacks
    );
    print_witnesses(
        "the selection differs from the interpreter's own readback",
        &census.readback_differs,
    );
    println!(
        "the caret (Query::Offset of Query::Caret's point): {} offsets over {} text fields, {} of \
         them sharing a point with another offset",
        census.offsets, census.fields, census.shared_points
    );
    print_witnesses(
        "a point that did not name its own offset",
        &census.not_inverse,
    );

    // Trap 11's arithmetic, in the instrument's own output: a refusal is a document the drag does
    // not judge, and a judged set that shrank in silence would print a perfect verdict over less.
    let mut reasons: BTreeMap<Refusal, usize> = BTreeMap::new();
    for (_, reason) in &census.refused {
        let count: &mut usize = reasons.entry(reason).or_default();
        *count = count.saturating_add(1);
    }
    println!(
        "{} of {files} documents refused the drag, each a document off the judged set:",
        census.refused.len()
    );
    for (reason, count) in &reasons {
        println!("  {count:4}  {reason}");
    }
    print_witnesses("panicked", &census.panicked);
}

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). A build without
/// it draws every other image and none of those three, so what follows would be a measurement of
/// the build rather than of the tree — which is exactly what moved the accessibility census's
/// ratchet by nine elements while four rounds read the difference as something else
/// (ADR 0557, trap 16).
#[expect(
    clippy::panic,
    reason = "a gate that cannot decode the images it is measuring must stop rather than \
              print a number about a different program"
)]
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the counts below would be \
             wrong: {error}"
        );
    }
}

/// The instrument. Ignored: it walks the corpus and asks poppler about every document.
#[test]
#[ignore = "corpus-scale; needs the pdf.js submodule and pdftotext — run explicitly, in release"]
fn what_a_drag_selects_agrees_with_poppler_and_with_the_page() {
    require_the_sandbox();
    let files = population();
    if files.is_empty() {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    }
    assert!(
        Extractor::PopplerBoxes.is_available(),
        "pdftotext is required for this census; it comes with poppler"
    );
    let cache = extraction_cache();
    let work_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("selection-census");
    let census = Mutex::new(Census::default());
    let started = Instant::now();

    files.par_iter().for_each(|path| {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            examine(path, &cache, &work_dir)
        }));
        let mut one = match outcome {
            Ok(one) => one,
            Err(payload) => {
                let what = payload
                    .downcast_ref::<&str>()
                    .map(ToString::to_string)
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "non-string panic payload".to_owned());
                let mut one = Census::default();
                one.panicked.push((path.display().to_string(), what));
                one
            }
        };
        if let Ok(mut census) = census.lock() {
            census.absorb(std::mem::take(&mut one));
        }
    });

    let elapsed = started.elapsed();
    let census = census.into_inner().expect("reported through the census");
    report(&census, files.len(), elapsed.as_secs_f64());
    let stats = cache.statistics();
    println!(
        "extraction cache: {} hits, {} misses, {} remembered timeouts",
        stats.hits, stats.misses, stats.remembered_timeouts
    );
    // The cost floor, on `pdfref::Runs`'s rules and for its reason: the line above counts
    // lookups, this one counts `pdftotext` actually being spawned, and a page extracted twice
    // in one census is work done to answer a question already answered. ADR 0898.
    let runs = cache.runs();
    println!("extractors: {runs}");
    assert!(
        runs.holds(),
        "an extractor ran again for a page the cache had kept: {runs}, repeated: {:?}",
        cache.repeated_keys()
    );

    assert!(
        census.panicked.is_empty(),
        "principle 1: no panic on any input"
    );
    // A judged set that went empty would print a perfect verdict over nothing, which is the
    // failure the geometry half's own assertion guards against.
    assert!(census.words > 0, "no word was dragged across anywhere");
    // The two exact properties. Both hold over the whole population, so they are stated as
    // properties rather than counted: a selection that is not the readback is a host being handed
    // something the interpreter did not produce, and a caret whose own point names a different
    // offset puts a click somewhere a person did not press.
    assert!(
        census.readback_differs.is_empty(),
        "Selection::All is not Interpretation::text on {} document(s): {:?}",
        census.readback_differs.len(),
        census.readback_differs
    );
    assert!(
        census.not_inverse.is_empty(),
        "Query::Offset did not invert Query::Caret in {} place(s): {:?}",
        census.not_inverse.len(),
        census.not_inverse
    );
    // The find's *cost* half is a third exact property and is asserted for the same reason, while
    // its accuracy fraction is printed beside the drag's: the fraction has poppler in it and is
    // ADR 0323's rule, and this one has nobody in it but us. `misses` is the number of pages a
    // search interpreted, so an empty list is the equality "a find bar opened on the page a person
    // is looking at reads no page", over every corpus document that states a word both extractors
    // agree about. It is a count and not a clock, so a neighbouring round's load cannot move it by
    // one (ADR 0905).
    assert!(
        census.reinterpreted.is_empty(),
        "a search interpreted a page the page turn had already read, on {} document(s): {:?}",
        census.reinterpreted.len(),
        census.reinterpreted
    );
}

/// One committed document through the whole census, un-ignored, so the classification cannot rot
/// between explicit runs.
///
/// `doc/PDF20_AN001-BPC.pdf` is the document ADR 0333's single-document drag test uses, and what
/// this pins is the *shape* of the answer rather than a count: the document is judged rather than
/// refused, words are dragged, every one of them is selected, and both exact properties hold.
#[test]
fn the_census_judges_a_committed_document_rather_than_refusing_it() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    if !path.exists() {
        println!("skipped: doc/'s specifications are not unpacked");
        return;
    }
    if !Extractor::PopplerBoxes.is_available() {
        println!("skipped: pdftotext is not installed");
        return;
    }
    let cache = extraction_cache();
    let work_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("selection-census");
    let census = examine(&path, &cache, &work_dir);
    assert!(census.refused.is_empty(), "{:?}", census.refused);
    assert_eq!(census.dragged_documents, 1, "the document was judged");
    assert_eq!(census.words, WORDS_PER_DOCUMENT, "three words were dragged");
    assert_eq!(
        census.selected, census.words,
        "every drag selected its word: {:?}",
        census.missed
    );
    assert_eq!(census.readbacks, 1);
    assert!(census.readback_differs.is_empty());
    assert!(census.not_inverse.is_empty());
    assert_eq!(census.searched_documents, 1, "the document was searched");
    assert_eq!(
        census.found, census.finds,
        "every search left its word selected: {:?}",
        census.not_found
    );
    assert!(census.finds > 0, "at least one word was searched for");
    assert!(
        census.reinterpreted.is_empty(),
        "the page showing was already read: {:?}",
        census.reinterpreted
    );
    assert!(
        census.find_hits >= u64::try_from(census.finds).unwrap_or(u64::MAX),
        "every step was answered out of the cache: {} hits for {} searches",
        census.find_hits,
        census.finds
    );
}
