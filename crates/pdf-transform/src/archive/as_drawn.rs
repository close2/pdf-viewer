//! ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2, where the operator a content stream
//! uses and the base standard does not define is a run this reader draws as a number: the number
//! it draws, written in place of the run (`doc/adr/2638`).
//!
//! # What the run is, and what this reader draws for it
//!
//! ISO 32000-2 §7.2.3 makes a run of regular characters one token:
//!
//! > A sequence of consecutive regular characters comprises a single token.
//!
//! and §7.3.3 spells a real with one period, so `33600.272.58` is no number. To the checker it is
//! the keyword §7.8.2 makes an operator (`doc/adr/2619`); the window draws the number the lexer
//! reads off its front, `33600.272`, wherever the rest names no operator of §8.2 Table 50
//! (`doc/adr/1004`). The page a reader sees is the page with that number in that place.
//!
//! # Why this sits on the near side of the authoring line
//!
//! The bytes written are the operand this reader already draws, in §7.3.3's own spelling, at the
//! place the producer wrote the run; no operator is added, removed or moved, and nothing around the
//! run changes. It invents no mark: the page draws as the window drew it before, which is the
//! photograph's own test (`doc/scope-ratifications.md`, A570) without the photograph's cost. It is
//! still a loss and is carried as one, `content-as-drawn`, because the characters after the number
//! are the producer's and are gone, and this reader's salvage is ADR 1004's choice rather than a
//! value the standard states — the difference from §7.3.4.3's assumed hexadecimal digit, which
//! `super::hexadecimal` writes losing nothing. A run this reader does not draw as a number — one
//! whose tail is an operator, or that opens with no number at all — is not touched, and its page
//! keeps whatever answer the fence gives it.
//!
//! # A lone delimiter, which this reader reads as no operator
//!
//! §7.2.3 makes `>` and `)` delimiters, and one that closes nothing opens no object: this tree's
//! lexer hands it over as a keyword, the checker reads it as an operator the base standard does
//! not define, and the reader meets the error §7.8.2 states — "an error shall occur" — and clears
//! the operands before it (`pdf_model::content::run`). Where no operand stands before it, that
//! clearing clears nothing, and the page draws as though the byte were white space; so the byte is
//! written as one white-space character, which §7.2.3 makes the separator it already was, under
//! the same loss, the producer's byte gone (`doc/adr/2859`). One that stands after an operand, or
//! inside an array or a dictionary, does change what the reader draws by being there, and is left
//! where it is: its stream keeps the operator and the row its answer.
//!
//! # An operator the reader repaired, where it drew the page the same without it
//!
//! ISO 19005-2 and ISO 19005-4 section 5.1 refuse a content stream this reader had to repair to
//! read, and five of `pdf_model`'s classes are an operator's: one short of its operands or given
//! one of the wrong type (§7.8.2), a `Q` with nothing saved (§8.4.2), a text operator outside a
//! text object (§9.4.1, answered as the next section says where it can be), and colour operands
//! that do not fit the space (§8.6.8). Where the
//! profile granted `content-as-drawn`, each such operator is written as one white-space
//! character, its operands with it — §7.8.2 makes them the operator's own, and the reader clears
//! them when it runs it — **and only where drawing the page both ways gives the same display list
//! and the same text** (`doc/adr/3558`). That proof is the certainty: no clause is read for what
//! the producer meant, the write states only what the window already drew, and a page it would
//! change keeps the operator and the refusal by the class's name (or, under `as-if-printed`, its
//! photograph). The producer's bytes are gone, which is the loss's own word.
//!
//! Operands left over in front of an operator's own are the one class where less is written out:
//! §7.8.2 states
//!
//! > all of the operands needed by an operator shall immediately precede that operator.
//!
//! so the operator's own are the last of them, and the reader runs it with exactly those and drops
//! the rest. Where that is the operator's only repair, the leftovers alone are written as one
//! white-space character and the operator keeps its own operands, which is what the reader ran;
//! the proof by drawing both files still decides (`doc/adr/3641`).
//!
//! # What else the write states as this reader read it
//!
//! A number written with an exponent, which §7.3.3 forbids a writer and this reader reads as its
//! value, is written as the decimal of that value (`doc/adr/3677`). A colour operator given one,
//! three or four operands that do not fit the current space (§8.6.8) is drawn by this reader in the
//! device space with that many components, so it is written as `G`, `RG` or `K` (or the
//! non-stroking form) with its operands kept (`doc/adr/3679`); and an inline image entry this
//! reader did not read, its key stated twice, is written out (`doc/adr/3678`). Inline image data
//! past the end its own decoding states, up to the `EI` this reader found by searching, is cut
//! (§8.9.7), and a hexadecimal string holding a byte that is neither a digit nor white space is
//! written without it, as this reader skipped it (§7.3.4.3, `doc/adr/3735`). A marked-content
//! section left open where its content stream ends is closed there by an `EMC`, where this reader
//! ended it (§14.6.1, `doc/adr/3734`). Each stands only where the page draws the same, as above.
//!
//! # A write answers only what it took
//!
//! The proof's second file is read again, and a class it still needs at a page or form is
//! answered nowhere in it (`doc/adr/3733`): a page whose content holds a class this write left
//! keeps the answer it had without the write — refused by that class's sentence under
//! `as-shown`, photographed under `as-if-printed` — however many other classes on it were
//! written.
//!
//! # A text operator this reader drew outside a text object
//!
//! ISO 32000-2 §9.4.2 and §9.4.3 each say their operators "shall only appear within text
//! objects", and Table 105 has `BT` begin one "initializing the text matrix, Tm, and the text line
//! matrix, Tlm, to the identity matrix". This reader runs a positioning or showing operator that
//! stands outside one on the matrices it holds there — the identity at the start of the content
//! and after each `ET`, which discards them (ADR 3353) — so writing it out would take its text off
//! the page. Instead a text object is added around each run of them: a `BT` in front of the first,
//! with a `Tm` stating the matrix this reader held where that is not the identity and the operator
//! reads it, and an `ET` after the last, before the first operator §9.4.1 keeps out of a text
//! object:
//!
//! > The other operators that may appear in a text object are those related to the general
//! > graphics state, colour, and marked-content, as shown in "Figure 9 -Graphics objects".
//!
//! An `ET` of the producer's that this reader ignored, standing outside any text object, closes
//! the one added. A `Td` or `TD` from a matrix the walk can state is composed onto it as the reader
//! composes it (Table 106), so the matrix stays stated (`doc/adr/3736`). Where the walk cannot
//! state the matrix — a glyph's width moved the text matrix past the line matrix, or a `T*`, `'`
//! or `"` moved it by the leading — no object is added and the class keeps its refusal. The proof by drawing both files decides as for an operator written out, and the loss
//! is `content-as-drawn`: no byte of the producer's is removed, but which matrix the text is drawn
//! by outside a text object is this reader's reading, not the standard's (`doc/adr/3661`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use pdf_archive::survey::{Survey, UnlistedOperator};
use pdf_model::Pages;
use pdf_model::content::grammar::{Grammar, Operand, Spaces, components, signature};
use pdf_model::content::reader::draws_the_salvage;
use pdf_render::geom::Transform;
use pdf_syntax::object::{Name, Object, ObjectId};
use pdf_syntax::{Document, Lexer, Token};

use super::decision::Because;
use super::hexadecimal;

/// Why a stream whose bytes cannot be written back keeps its runs.
const STREAM_WILL_NOT_RE_ENCODE: &str = "a content stream holding a run this reader draws as a \
     number cannot be written back: it will not re-encode, or its /DecodeParms names another \
     object that discarding the parameters would orphan. So the number cannot be written in the \
     run's place";

/// Why the operators a repair was logged at are left where they are.
const NOT_DRAWN_THE_SAME: &str = "writing out the operators this reader repaired would change \
     what a page draws, or the page could not be drawn both ways to show it would not, so they \
     are left where the producer wrote them (doc/adr/3558)";

/// The repair classes an operator is written out for where the page draws the same without it
/// (`doc/adr/3558`).
///
/// A `BDC` naming a property list the resources do not hold (§14.6.2) is written out with its
/// section where its operands are the wrong type, and its name goes with it (`doc/adr/3615`); a
/// well-typed one keeps it, and the name is defined instead (`doc/adr/2684`). Operands left over in
/// front of an operator's own are written out without it where they are its only repair, and with
/// it where it needs another (§7.8.2, `doc/adr/3641`), and an `EMC` with no section open is written
/// out alone (Table 352), and a section left open at its stream's end is closed there
/// (`doc/adr/3734`), each only where the page draws the same. An inline image's key stated twice
/// is not an operator's, but its entry this reader did not read is written out under the same
/// proof (`doc/adr/3678`); so are inline image data this reader never decoded and a byte of a
/// hexadecimal string it skipped (`doc/adr/3735`). A class the write left at a page or form is
/// answered nowhere in it (`doc/adr/3733`).
pub(super) const WRITTEN_OUT_WHERE_DRAWN_THE_SAME: [&str; 11] = [
    "model/operator-short-of-operands",
    "model/operand-of-the-wrong-type",
    "model/restore-with-nothing-saved",
    "model/text-operator-outside-a-text-object",
    "model/colour-operands-do-not-fit-the-space",
    "model/property-list-unresolved",
    "model/operands-left-over",
    "model/marked-content-unbalanced",
    "model/inline-image-key-twice",
    "model/inline-image-data-extent",
    "syntax/hexadecimal-string-stray-byte",
];

/// The most pages the proof draws twice; past it the operators are left.
const MOST_PAGES_PROVED: usize = 256;

/// Why a document naming such a run nowhere this walk reaches keeps the refusal.
const NO_STREAM_HOLDS_ONE: &str = "no content stream this conversion reads holds a run this \
     reader draws as a number, so the operator the base standard does not define is something \
     else, and writing a number would answer nothing";

/// The most runs one conversion writes, over every content stream it reads.
///
/// `CLAUDE.md` principle 3's explicit budget, checked as each run is found and before its record
/// is kept: a record costs a few hundred bytes (its pages, its bytes and its number), and the
/// stream bounds the runs only by half its decoded length, so a stream of runs alone would
/// otherwise cost some sixty times its length. `509284`, the document this remedy was built for,
/// holds 58; a document past the bound keeps the operator row's refusal (`doc/adr/2638`).
const MOST_RUNS: usize = 1 << 16;

/// Why a document holding more runs than [`MOST_RUNS`] keeps the refusal.
const TOO_MANY_RUNS: &str = "the content streams hold more than 65536 runs this reader draws as \
     numbers, more than this conversion writes, so the operator the base standard does not define \
     stays where the producer wrote it";

/// Why a stream that also omits a hexadecimal digit keeps its runs.
const ALSO_A_HEXADECIMAL_REPAIR: &str = "a content stream holding a run this reader draws as a \
     number also states a hexadecimal string with an odd number of digits, and the two rewrites \
     of one stream are not composed";

/// Why a stream whose `/Lang` is also respelled keeps its runs.
const ALSO_A_LANGUAGE_RESPELLED: &str = "a content stream holding a run this reader draws as a \
     number also states a /Lang that super::languages respells, and the two rewrites of one \
     stream are not composed";

/// One run a conversion wrote as the number this reader draws for it, under the loss
/// `content-as-drawn` (`doc/adr/2638`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenRun {
    /// The pages, counting from one, whose content the checker read the run on.
    pub pages: Vec<usize>,
    /// The content stream object it stood in, in the source's numbering.
    pub object: ObjectId,
    /// The run as the producer wrote it.
    pub run: Vec<u8>,
    /// The number written in its place, in ISO 32000-2 §7.3.3's spelling; empty where the run is
    /// a lone delimiter, written as one white-space character (`doc/adr/2859`).
    pub number: Vec<u8>,
}

/// The most runs `xmpMM:History` names one by one; the rest are counted.
const MOST_NAMED: usize = 16;

/// The content streams this conversion rewrites, and every run it wrote a number for.
#[derive(Debug, Default)]
pub(super) struct Drawn {
    /// The stream each object is to carry.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// Every run written, in stream order.
    pub(super) runs: Vec<WrittenRun>,
    /// Whether every operator the survey found that the base standard does not define is a run
    /// this reader draws as a number, so that writing the numbers answers the operator row.
    pub(super) answers_every_operator: bool,
    /// The content streams the survey read whose filters do not decode whole, so that no run in
    /// them is written (ADR 2803).
    pub(super) unread: BTreeSet<ObjectId>,
    /// Whether a lone delimiter or an operator no table defines stood where removing it would
    /// change what this reader draws — after an operand, or inside an array or a dictionary — or
    /// where this write did not reach it, and so was left (`doc/adr/2859`, `doc/adr/3698`).
    pub(super) delimiter_left: bool,
    /// How many lone delimiters and undefined operators were written as white space.
    delimiters_written: usize,
    /// The streams an operator no table defines was written as white space in (`doc/adr/3698`),
    /// which the proof draws like `written_out` but which answer no repair class.
    blanked: BTreeSet<ObjectId>,
    /// The streams an operator this reader repaired was written out of (`doc/adr/3558`), or a
    /// text object was added to (`doc/adr/3661`).
    pub(super) written_out: BTreeSet<ObjectId>,
    /// How many text objects were added around operators this reader ran outside one
    /// (`doc/adr/3661`).
    pub(super) text_objects_added: usize,
    /// How many marked-content sections left open at their content stream's end were closed there
    /// (`doc/adr/3734`).
    pub(super) sections_closed: usize,
    /// How many inline images had the data this reader never decoded cut (`doc/adr/3735`).
    pub(super) data_cut: usize,
    /// Whether every hexadecimal string inside a content stream holding a byte this reader
    /// skipped was written without it, at a holder that needs no such repair afterwards, so that
    /// ISO 19005-2 section 6.1.6's row holds no page (`doc/adr/3735`).
    pub(super) strays_written: bool,
    /// Each page or form whose content, read again from the proof's file, still needed a repair
    /// that breaches section 5.1 and that no other rewrite writes out, with the repair's class: the
    /// write answers that class nowhere in the holder, and the page keeps the answer it had
    /// without the write, the photograph's included (`doc/adr/3733`).
    still_repaired: BTreeSet<(ObjectId, &'static str)>,
}

impl Drawn {
    /// The stream `id`'s decoded `data` with each of `places` written in, every run among them
    /// recorded with the pages `read_on` names for it.
    fn rewritten(
        &mut self,
        id: ObjectId,
        data: &[u8],
        places: Vec<Place>,
        read_on: &BTreeMap<&[u8], BTreeSet<usize>>,
    ) -> Vec<u8> {
        let mut written = Vec::with_capacity(data.len());
        let mut from = 0;
        for place in places {
            written.extend_from_slice(data.get(from..place.start).unwrap_or_default());
            if place.text_object {
                if place.start < place.end {
                    self.data_cut = self.data_cut.saturating_add(1);
                }
                self.text_object_added(id, &place.number, &mut written);
                from = place.end;
                continue;
            }
            // A lone delimiter's place is written as the white space §7.2.3 already read it as,
            // so the tokens either side of it stay apart.
            if place.number.is_empty() {
                written.push(b' ');
                if place.operator {
                    self.written_out.insert(id);
                } else {
                    if place.undefined {
                        self.blanked.insert(id);
                    }
                    self.delimiters_written = self.delimiters_written.saturating_add(1);
                }
            } else {
                written.extend_from_slice(&place.number);
                if place.operator {
                    self.written_out.insert(id);
                }
            }
            from = place.end;
            let run = data
                .get(place.start..place.end)
                .unwrap_or_default()
                .to_vec();
            self.runs.push(WrittenRun {
                pages: read_on
                    .get(run.as_slice())
                    .map_or_default(|pages| pages.iter().copied().collect()),
                object: id,
                run,
                number: place.number,
            });
        }
        written.extend_from_slice(data.get(from..).unwrap_or_default());
        written
    }

    /// Writes the `BT` or `ET` added around operators this reader ran outside a text object into
    /// the stream `id`. It replaces nothing, and the page is held to drawing the same as for an
    /// operator written out (`doc/adr/3661`).
    fn text_object_added(&mut self, id: ObjectId, added: &[u8], written: &mut Vec<u8>) {
        written.extend_from_slice(added);
        self.written_out.insert(id);
        if added.starts_with(OPENS) {
            self.text_objects_added = self.text_objects_added.saturating_add(1);
        }
        let closed = added
            .windows(CLOSES_A_SECTION.len())
            .filter(|window| *window == CLOSES_A_SECTION)
            .count();
        self.sections_closed = self.sections_closed.saturating_add(closed);
    }

    /// Whether the repair of `class` at the page or form `holder` is one this write answers: every
    /// content stream of the holder is one it rewrote, an operator was written out of one of them
    /// (`doc/adr/3558`), and the holder's content, read from the proof's file, no longer needs
    /// that repair (`doc/adr/3733`).
    pub(super) fn wrote_out_of(&self, document: &Document, holder: ObjectId, class: &str) -> bool {
        if self.left_repaired(holder, class) {
            return false;
        }
        let streams = match document.get(holder) {
            Object::Stream(_) => vec![holder],
            Object::Dictionary(page) => hexadecimal::contents_of(document, &page),
            _ => Vec::new(),
        };
        !streams.is_empty()
            && streams.iter().all(|stream| self.at.contains_key(stream))
            && streams
                .iter()
                .any(|stream| self.written_out.contains(stream))
    }

    /// Whether the content of the page or form `holder`, read from the proof's file, still needed
    /// the repair of `class`, so that the write does not answer it there (`doc/adr/3733`).
    pub(super) fn left_repaired(&self, holder: ObjectId, class: &str) -> bool {
        self.still_repaired
            .iter()
            .any(|(at, left)| *at == holder && *left == class)
    }

    /// Whether every operator the survey reports under `spelling` is one this write answers: a run
    /// this reader draws as a number, or a lone delimiter or an undefined operator where none was
    /// left.
    pub(super) fn answers(&self, spelling: &[u8]) -> bool {
        is_drawn_as_a_number(spelling)
            || ((is_a_lone_delimiter(spelling) || is_an_undefined_operator(spelling))
                && !self.delimiter_left)
    }

    /// Whether the stream an operator the survey found stood in is one this write could not read,
    /// so that the run in it is still in the output: the object the survey names, or for a page's
    /// own content, any stream its `/Contents` names.
    pub(super) fn left(&self, document: &Document, unlisted: &UnlistedOperator) -> bool {
        if self.unread.is_empty() {
            return false;
        }
        match unlisted.place.object {
            Some(object) => self.unread.contains(&object),
            None => Pages::new(document).get(unlisted.page).is_none_or(|page| {
                hexadecimal::contents_of(document, &page.dict)
                    .iter()
                    .any(|id| self.unread.contains(id))
            }),
        }
    }
}

/// Writes the number this reader draws in place of every run it salvages, in every content
/// stream the page tree and the survey reach.
///
/// `hexadecimal` is the other content-stream repair, where it was asked for: a stream both would
/// rewrite is refused rather than written twice.
///
/// Where `operators` is set, an operator this reader logged one of
/// [`WRITTEN_OUT_WHERE_DRAWN_THE_SAME`]'s repairs at is written out too, where the pages draw the
/// same without it; where they would not, the write is made again without them.
pub(super) fn write(
    document: &Document,
    survey: Option<&Survey>,
    hexadecimal: Option<&hexadecimal::Completed>,
    operators: bool,
) -> Result<Drawn, Because> {
    if operators {
        match write_with(document, survey, hexadecimal, true) {
            Err(because) if because == Because::NotBuiltYet(NOT_DRAWN_THE_SAME) => {}
            written => return written,
        }
    }
    write_with(document, survey, hexadecimal, false)
}

/// [`write()`], with the repaired operators written out where `operators` is set.
fn write_with(
    document: &Document,
    survey: Option<&Survey>,
    hexadecimal: Option<&hexadecimal::Completed>,
    operators: bool,
) -> Result<Drawn, Because> {
    // A lone delimiter, and an operator no table defines where no operand stands, is written only
    // where the write then answers every operator: where some other operator stays, the page keeps
    // the answer it had without this write, the photograph's included, and the delimiter stays with
    // it (`doc/adr/2859`, `doc/adr/3698`). Written where the proof then refuses, it is the second
    // write that stands.
    match write_places(document, survey, hexadecimal, (true, operators)) {
        Ok(drawn) if drawn.answers_every_operator || drawn.delimiters_written == 0 => Ok(drawn),
        _ => write_places(document, survey, hexadecimal, (false, operators)),
    }
}

/// [`write()`], with lone delimiters written where `delimiters` is set and every one of them left
/// where it is not, and the repaired operators written out where `operators` is.
fn write_places(
    document: &Document,
    survey: Option<&Survey>,
    hexadecimal: Option<&hexadecimal::Completed>,
    (delimiters, operators): (bool, bool),
) -> Result<Drawn, Because> {
    let pages = Pages::new(document);
    let mut read_on: BTreeMap<&[u8], BTreeSet<usize>> = BTreeMap::new();
    for unlisted in survey.map_or(&[][..], Survey::unlisted_operators) {
        read_on
            .entry(unlisted.spelling.as_slice())
            .or_default()
            .insert(unlisted.page.saturating_add(1));
    }
    let mut out = Drawn {
        answers_every_operator: survey.is_some_and(|survey| {
            survey.unlisted_operators().iter().all(|unlisted| {
                is_drawn_as_a_number(&unlisted.spelling)
                    || is_a_lone_delimiter(&unlisted.spelling)
                    || is_an_undefined_operator(&unlisted.spelling)
            })
        }),
        ..Drawn::default()
    };
    // Each stream of a page's `/Contents` array after the first carries the walk of the one
    // before it (`Walk`); every other stream starts its own.
    let mut follows: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();
    for page in pages.every().into_iter().flatten() {
        let contents = hexadecimal::contents_of(document, &page.dict);
        for pair in contents.windows(2) {
            if let [before, after] = pair {
                follows.entry(*after).or_insert(*before);
            }
        }
    }
    let followed: BTreeSet<ObjectId> = follows.values().copied().collect();
    let began_in = streams_began_in(survey);
    // A page's own content starts in §8.4.1's initial state; a stream the survey opened as
    // something else inherits the state it was invoked in.
    let page_content: BTreeSet<ObjectId> = pages
        .every()
        .into_iter()
        .flatten()
        .flat_map(|page| hexadecimal::contents_of(document, &page.dict))
        .filter(|id| !began_in.contains_key(id))
        .collect();
    // An operator no table defines is not written as white space in a file one of whose streams
    // did not decode, where such a keyword may be the damage's bytes rather than anything a
    // producer wrote (`doc/adr/3698`).
    let blanks_undefined = delimiters
        && !document
            .repairs()
            .iter()
            .any(|logged| logged.class.name == pdf_syntax::repair::STREAM_DATA_UNDECODABLE.name);
    let mut walk = Walk::default();
    let mut walked: Option<ObjectId> = None;
    for (id, resources) in hexadecimal::streams(document, &pages, survey) {
        walk = match (walked, follows.get(&id)) {
            (Some(previous), Some(before)) if previous == *before => walk.into_the_next_stream(),
            _ => Walk::starting_in(
                began_in.get(&id).copied().unwrap_or(Spaces::INITIAL),
                page_content.contains(&id),
            ),
        };
        walked = Some(id);
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        // The survey read this stream through the content reader, which draws what a damaged
        // filter yields before the damage; the whole decode the bytes would be written back from
        // does not exist, so its runs stay and its page is photographed where the profile allows
        // (ADR 2803). A stream the source's security handler did not decrypt is the same: its
        // bytes are still the cipher's, and what they lex as is no producer's content
        // (`doc/adr/3698`).
        let decoded = (!stream.decryption_failed)
            .then(|| document.decoded_stream_data(stream))
            .flatten();
        let Some(data) = decoded else {
            out.unread.insert(id);
            continue;
        };
        let room = MOST_RUNS.saturating_sub(out.runs.len());
        let (mut places, delimiter_left) = salvaged_runs(
            document,
            &data,
            &resources,
            room,
            (delimiters, operators, blanks_undefined),
            &mut walk,
        )
        .ok_or(Because::NotBuiltYet(TOO_MANY_RUNS))?;
        out.delimiter_left |= delimiter_left;
        // A section still open where the last stream of a page's `/Contents`, or a form's own
        // stream, ends is closed there (`doc/adr/3734`).
        if operators && !followed.contains(&id) {
            places.extend(walk.sections_left_open(data.len()));
        }
        if places.is_empty() {
            continue;
        }
        if hexadecimal.is_some_and(|completed| completed.at.contains_key(&id)) {
            return Err(Because::NotBuiltYet(ALSO_A_HEXADECIMAL_REPAIR));
        }
        let written = out.rewritten(id, &data, places, &read_on);
        let replacement = hexadecimal::repaired(stream, &written)
            .ok_or(Because::NotBuiltYet(STREAM_WILL_NOT_RE_ENCODE))?;
        out.at.insert(id, replacement);
    }
    if out.at.is_empty() {
        return Err(Because::NotBuiltYet(NO_STREAM_HOLDS_ONE));
    }
    let proved: BTreeSet<ObjectId> = out.written_out.union(&out.blanked).copied().collect();
    if !proved.is_empty() {
        out.still_repaired = drawn_the_same(document, &out.at, &proved)
            .ok_or(Because::NotBuiltYet(NOT_DRAWN_THE_SAME))?;
    }
    // A run left in a stream this write could not read is an operator the numbers do not answer,
    // and so is a delimiter left where it changes the drawing.
    out.answers_every_operator &= out.unread.is_empty() && !out.delimiter_left;
    out.strays_written = wrote_every_stray(document, &out);
    Ok(out)
}

/// The colour spaces each stream the survey opened began in, so that this walk judges a colour
/// operator against the space the reader draws it in: a form `XObject` inherits the spaces in
/// force at its `Do` (ISO 32000-2 §8.10.1, `doc/adr/3697`). A stream the survey began in two
/// states — a form painted on two pages under different spaces — is judged against neither
/// ([`Spaces::UNCOUNTED`]), so none of its colour operators is written. A page's own `/Contents`
/// is in no entry and starts in §8.4.1's initial state.
fn streams_began_in(survey: Option<&Survey>) -> BTreeMap<ObjectId, Spaces> {
    let mut out: BTreeMap<ObjectId, Spaces> = BTreeMap::new();
    for opened in survey.map_or(&[][..], Survey::opened_streams) {
        let Some(id) = opened.object else {
            continue;
        };
        out.entry(id)
            .and_modify(|spaces| {
                if *spaces != opened.began_in {
                    *spaces = Spaces::UNCOUNTED;
                }
            })
            .or_insert(opened.began_in);
    }
    out
}

/// **The proof that writing the operators out changes nothing drawn** (`doc/adr/3558`): the
/// streams written are appended to the source as ISO 32000-2 §7.5.6's incremental update — or,
/// where the source's table was rebuilt by scanning, written with every other object into one new
/// file ([`whole_file`], `doc/adr/3662`) — and
/// every page that draws one of `written_out` is drawn from both files; the write stands where
/// each page's display list and text are the same. The pages' annotations are drawn too, since an
/// appearance stream is one of the streams this write reaches.
///
/// `Some` where it stands, holding each page or form whose content, drawn from the second file,
/// still needed a repair section 5.1 refuses, with its class ([`still_repaired`]); `None` where it
/// does not.
fn drawn_the_same(
    document: &Document,
    at: &BTreeMap<ObjectId, Object>,
    written_out: &BTreeSet<ObjectId>,
) -> Option<BTreeSet<(ObjectId, &'static str)>> {
    let pages = super::fenced::pages_drawing(document, written_out);
    if pages.is_empty() || pages.len() > MOST_PAGES_PROVED {
        return None;
    }
    // A file whose cross-reference table was rebuilt by scanning has no section an update could
    // chain to, so the second file is written whole from the objects this reader read
    // (`doc/adr/3662`).
    let bytes = if document.was_recovered() {
        whole_file(document, at)
    } else {
        pdf_syntax::write::incremental_update(document, at).ok()
    };
    let after =
        bytes.and_then(|bytes| Document::open_with_limits(bytes, document.limits()).ok())?;
    let (before_pages, after_pages) = (Pages::new(document), Pages::new(&after));
    let same = pages.into_iter().all(|index| {
        let (Some(before), Some(written)) = (before_pages.get(index), after_pages.get(index))
        else {
            return false;
        };
        let was = pdf_model::content::interpret(document, &before);
        let is = pdf_model::content::interpret(&after, &written);
        was.display_list == is.display_list && was.text == is.text
    });
    if same { still_repaired(&after) } else { None }
}

/// Whether `drawn` wrote every hexadecimal string holding a byte this reader skipped inside a
/// content stream without that byte, wherever the reader logged one (`doc/adr/3735`): each holder
/// named, none unnamed, and at least one.
fn wrote_every_stray(document: &Document, drawn: &Drawn) -> bool {
    let stray = pdf_syntax::repair::HEXADECIMAL_STRING_STRAY_BYTE.name;
    let mut any = false;
    document
        .repairs()
        .iter()
        .filter(|logged| logged.class.name == stray)
        .all(|logged| match logged.at {
            pdf_syntax::repair::Site::Content(Some(holder)) => {
                any = true;
                drawn.wrote_out_of(document, holder, stray)
            }
            pdf_syntax::repair::Site::Content(None) => false,
            _ => true,
        })
        && any
}

/// **What a write left is not answered by it** (`doc/adr/3733`): each page or form at which the
/// proof's second file, `after`, read with the write made, still logged a repair inside a content
/// stream that breaches section 5.1 and that no other rewrite of this converter writes out
/// (`super::base_standard::written_out_by_a_rewrite`), with the repair's class.
///
/// The fence answers a class at a holder by the write only where the class is gone from it, so a
/// page whose content holds a class the write left keeps the answer it had — refused by that
/// class's sentence under `as-shown`, photographed under `as-if-printed` — even where another
/// class on it was written. Reading the written content again, rather than counting what the walk
/// wrote, is what makes that hold for every class at once, including a repair the walk never met.
///
/// `None` where such a class was made at more sites than the log keeps
/// (`pdf_syntax::repair::SITES_KEPT_PER_CLASS`): the holders past it are not known, so the write
/// is not shown to have answered any of them.
fn still_repaired(after: &Document) -> Option<BTreeSet<(ObjectId, &'static str)>> {
    let refused = |name: &str| {
        pdf_archive::repairs::breaches_row(name, pdf_archive::repairs::BASE_STANDARD)
            && !super::base_standard::written_out_by_a_rewrite(name)
    };
    if after
        .repair_classes()
        .iter()
        .any(|class| refused(class.name) && after.repairs_beyond_the_log(class.name) > 0)
    {
        return None;
    }
    Some(
        after
            .repairs()
            .iter()
            .filter(|logged| refused(logged.class.name))
            .filter_map(|logged| match logged.at {
                pdf_syntax::repair::Site::Content(Some(holder)) => {
                    Some((holder, logged.class.name))
                }
                _ => None,
            })
            .collect(),
    )
}

/// The most bytes [`whole_file`] writes; past it the proof is not made and the operators are left.
const MOST_BYTES_WRITTEN_WHOLE: usize = 1 << 28;

/// The source written as one new file for [`drawn_the_same`], every object this reader reads in it
/// under its own identifier and each of `at` in its object's place, where the source's
/// cross-reference table was rebuilt by scanning (`doc/adr/3662`).
///
/// It is the proof's file and never an output: a stream is written with the `/Length` of the data
/// this reader read, and the trailer states `/Root` and `/Info` as the source's does. `None` for an
/// encrypted source, whose objects this would write decrypted under no `/Encrypt`, and past
/// [`MOST_BYTES_WRITTEN_WHOLE`].
fn whole_file(document: &Document, at: &BTreeMap<ObjectId, Object>) -> Option<Vec<u8>> {
    if document.is_encrypted() {
        return None;
    }
    let mut out = b"%PDF-2.0\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut entries = Vec::new();
    for number in document.xref().object_numbers() {
        let id = document.defined_id(number);
        let mut object = at.get(&id).cloned().unwrap_or_else(|| document.get(id));
        if let Object::Stream(stream) = &mut object {
            let length = i64::try_from(stream.data.len()).ok()?;
            std::sync::Arc::make_mut(stream)
                .dict
                .insert(Name::new(&b"Length"[..]), Object::Integer(length));
        }
        entries.push((id, out.len()));
        // Writing into a `String` cannot fail, so the `write!` result is discarded deliberately.
        let mut header = String::new();
        let _ = writeln!(header, "{} {} obj", id.number, id.generation);
        out.extend_from_slice(header.as_bytes());
        pdf_syntax::write::object(&object, &mut out);
        out.extend_from_slice(b"\nendobj\n");
        if out.len() > MOST_BYTES_WRITTEN_WHOLE {
            return None;
        }
    }
    let table = out.len();
    let mut xref = String::from("xref\n0 1\n0000000000 65535 f\r\n");
    for (id, offset) in &entries {
        let _ = write!(
            xref,
            "{} 1\n{offset:010} {:05} n\r\n",
            id.number, id.generation
        );
    }
    out.extend_from_slice(xref.as_bytes());
    let mut trailer = pdf_syntax::object::Dictionary::new();
    let size = entries.iter().map(|(id, _)| id.number).max().unwrap_or(0);
    trailer.insert(
        Name::new(&b"Size"[..]),
        Object::Integer(i64::from(size).saturating_add(1)),
    );
    for key in ["Root", "Info"] {
        if let Some(value) = document.trailer().get(key) {
            trailer.insert(Name::new(key.as_bytes()), value.clone());
        }
    }
    out.extend_from_slice(b"trailer\n");
    pdf_syntax::write::object(&Object::Dictionary(trailer), &mut out);
    let mut tail = String::new();
    let _ = write!(tail, "\nstartxref\n{table}\n%%EOF\n");
    out.extend_from_slice(tail.as_bytes());
    Some(out)
}

/// What `xmpMM:History` says of the runs written: the requirement, the count and the loss, then
/// each run by its pages, its bytes and the number written, the first [`MOST_NAMED`] by name.
///
/// A run's bytes are the producer's and need not be text, so each byte outside printable ASCII is
/// written as `\xHH`.
pub(super) fn history(drawn: &Drawn, citation: &str) -> String {
    let mut out = format!(
        "{citation}: {} run(s) the base standard spells as no number, lone delimiters it reads \
         as no operator, operators this reader repaired and drew the page the same without, or \
         entries of an inline image dictionary it did not read, were written as this reader \
         draws them, a run as its number, a colour operator as the device operator it drew it \
         by, and a delimiter, an operator with its operands or an entry as white space; loss {}",
        drawn.runs.len(),
        super::Loss::ContentAsDrawn.word()
    );
    for (index, written) in drawn.runs.iter().take(MOST_NAMED).enumerate() {
        let pages: Vec<String> = written.pages.iter().map(usize::to_string).collect();
        // Writing into a `String` cannot fail, so the `write!` result is discarded deliberately.
        let _ = write!(
            out,
            "{} page {} object {} {}: {} as {}",
            if index == 0 { ": " } else { "; " },
            if pages.is_empty() {
                "?".to_owned()
            } else {
                pages.join(",")
            },
            written.object.number,
            written.object.generation,
            spelled(&written.run),
            written_as(&written.number)
        );
    }
    if drawn.runs.len() > MOST_NAMED {
        let _ = write!(
            out,
            ", and {} more",
            drawn.runs.len().saturating_sub(MOST_NAMED)
        );
    }
    if drawn.text_objects_added > 0 {
        let _ = write!(
            out,
            "; {} text object(s) were added, BT and ET around text operators this reader drew \
             outside one, with the text matrix it drew them by (ISO 32000-2 §9.4.1, doc/adr/3661)",
            drawn.text_objects_added
        );
    }
    if drawn.data_cut > 0 {
        let _ = write!(
            out,
            "; {} inline image(s) had the bytes between the end their decoding states and the EI \
             this reader found removed, bytes it never decoded (ISO 32000-2 §8.9.7, \
             doc/adr/3735)",
            drawn.data_cut
        );
    }
    if drawn.sections_closed > 0 {
        let _ = write!(
            out,
            "; {} marked-content section(s) left open at the end of their content stream were \
             closed there by an EMC, where this reader ended them (ISO 32000-2 §14.6.1, \
             doc/adr/3734)",
            drawn.sections_closed
        );
    }
    out
}

/// What was written in a run's place, for a person: the number, or that a lone delimiter was
/// written as white space.
pub(super) fn written_as(number: &[u8]) -> String {
    if number.is_empty() {
        "white space".to_owned()
    } else {
        spelled(number)
    }
}

/// A run's bytes as text, each byte outside printable ASCII written as `\xHH`.
pub(super) fn spelled(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for byte in bytes {
        if byte.is_ascii_graphic() {
            out.push(char::from(*byte));
        } else {
            let _ = write!(out, "\\x{byte:02X}");
        }
    }
    out
}

/// The runs written, unless a stream they are in is one `super::languages` also respells: that
/// rewrite writes the stream from the producer's bytes, so carrying both would drop one, and the
/// stream is refused rather than written twice.
pub(super) fn apart_from(
    drawn: Drawn,
    languages: Option<&super::languages::Restated>,
) -> Result<Drawn, Because> {
    match languages {
        Some(restated) if drawn.at.keys().any(|id| restated.streams.contains_key(id)) => {
            Err(Because::NotBuiltYet(ALSO_A_LANGUAGE_RESPELLED))
        }
        _ => Ok(drawn),
    }
}

/// Whether `because` is [`write()`]'s refusal of a document in which no content stream holds a
/// run this reader draws as a number, so that the operator row has nothing this remedy answers.
pub(super) fn reached_no_run(because: Because) -> bool {
    because == Because::NotBuiltYet(NO_STREAM_HOLDS_ONE)
}

/// Whether a keyword the checker reports is a run this reader draws as a number, read alone.
///
/// The survey reports each operator by its spelling, and a run lexes the same wherever it
/// stands, because §7.2.3 ends a token only at white space or a delimiter.
pub(super) fn is_drawn_as_a_number(spelling: &[u8]) -> bool {
    let mut lexer = Lexer::new(spelling);
    lexer.next_token().is_some()
        && lexer.position() == spelling.len()
        && lexer.salvaged().is_some_and(draws_the_salvage)
}

/// Whether a keyword the checker reports is a delimiter that closes nothing, which this tree's
/// lexer hands over as a keyword of its own (§7.2.3's `>` and `)`).
pub(super) fn is_a_lone_delimiter(spelling: &[u8]) -> bool {
    matches!(spelling, b">" | b")")
}

/// Whether a keyword the checker reports is one no table of ISO 32000-2 defines as an operator and
/// this reader reads as one: not a run it salvages, not a delimiter, not `true`, `false` or `null`.
///
/// §7.8.2 has such an operator meet "an error", which this reader reports while clearing the
/// operands before it (`pdf_model::content::run`); with none before it, it draws nothing, and the
/// page draws as though the keyword were white space (`doc/adr/3698`).
pub(super) fn is_an_undefined_operator(spelling: &[u8]) -> bool {
    let mut lexer = Lexer::new(spelling);
    matches!(lexer.next_token(), Some(Token::Keyword(word)) if word == spelling)
        && lexer.salvaged().is_none()
        && signature(spelling).is_none()
        && !is_a_lone_delimiter(spelling)
        && !matches!(spelling, b"true" | b"false" | b"null" | b"BI")
}

/// A run's span in one decoded stream, and the number written for it.
struct Place {
    /// The run's first byte.
    start: usize,
    /// One past its last byte.
    end: usize,
    /// The number the reader draws, in §7.3.3's spelling; empty for a lone delimiter and an
    /// operator written out.
    number: Vec<u8>,
    /// Whether the place is an operator a repair was logged at, with its operands
    /// (`doc/adr/3558`).
    operator: bool,
    /// Whether `number` stands for the bytes from `start` to `end` with nothing else written: the
    /// `BT` or the `ET` this write adds around operators this reader ran outside a text object
    /// (`doc/adr/3661`) and the `EMC` closing a section left open (`doc/adr/3734`), where `start`
    /// and `end` are the same byte, or inline image data this reader never decoded, cut with
    /// `number` empty (`doc/adr/3735`).
    text_object: bool,
    /// Whether the place is an operator no table defines with no operand before it, written as
    /// white space under the proof (`doc/adr/3698`).
    undefined: bool,
}

impl Place {
    /// A place written as one white-space character: a lone delimiter, or an operator with its
    /// operands where `operator` is set.
    const fn white_space(start: usize, end: usize, operator: bool) -> Self {
        Self {
            start,
            end,
            number: Vec::new(),
            operator,
            text_object: false,
            undefined: false,
        }
    }

    /// An operator no table defines from `start` to `end`, with no operand before it, written as
    /// the white space this reader read it as (`doc/adr/3698`).
    const fn undefined_operator(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            number: Vec::new(),
            operator: false,
            text_object: false,
            undefined: true,
        }
    }

    /// A run this reader draws as `number`, written in its place.
    const fn number(start: usize, end: usize, number: Vec<u8>) -> Self {
        Self {
            start,
            end,
            number,
            operator: false,
            text_object: false,
            undefined: false,
        }
    }

    /// The operator keyword from `start` to `end`, written as `device` (`doc/adr/3679`).
    const fn respelled(start: usize, end: usize, device: Vec<u8>) -> Self {
        Self {
            start,
            end,
            number: device,
            operator: true,
            text_object: false,
            undefined: false,
        }
    }

    /// The bytes from `start` to `end` removed with nothing in their place: inline image data this
    /// reader never decoded (`doc/adr/3735`).
    const fn cut(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            number: Vec::new(),
            operator: false,
            text_object: true,
            undefined: false,
        }
    }

    /// A hexadecimal string from `start` to `end` written as `digits`, the producer's digits and
    /// white space without the bytes this reader skipped (`doc/adr/3735`).
    const fn hexadecimal(start: usize, end: usize, digits: Vec<u8>) -> Self {
        Self {
            start,
            end,
            number: digits,
            operator: true,
            text_object: false,
            undefined: false,
        }
    }

    /// The bytes `added` at `at`, opening or closing a text object (`doc/adr/3661`).
    const fn text_object(at: usize, added: Vec<u8>) -> Self {
        Self {
            start: at,
            end: at,
            number: added,
            operator: false,
            text_object: true,
            undefined: false,
        }
    }
}

/// The grammar's walk through a page's content, carried from one stream of its `/Contents` array
/// to the next: ISO 32000-2 §7.8.2 makes the array's streams one stream for the reader, divided
/// only at token boundaries, so an operator's operands and the text object and saved states around
/// it may stand in an earlier stream than the operator does.
#[derive(Default)]
struct Walk {
    /// The grammar the validator judges the content by (ADR 3542).
    grammar: Grammar,
    /// Where the operands since the last operator began in this stream, if they did.
    operands_from: Option<usize>,
    /// Whether some of those operands stand in an earlier stream, so that the operator they
    /// belong to is not written out: its place would span two streams.
    operands_elsewhere: bool,
    /// The last name operand, which a `CS` or `cs` selects its space by.
    last_name: Option<Vec<u8>>,
    /// Each open `BMC` or `BDC` section, innermost last: whether its opening operator is written
    /// out, so that the `EMC` balancing it is written out with it (`doc/adr/3615`). At most
    /// [`MOST_SECTIONS`]; past it a section is not followed and its operator is left.
    sections: Vec<bool>,
    /// Where each operand since the last operator began in this stream, oldest first, so that the
    /// operands left over in front of an operator's own can be written out without them
    /// (`doc/adr/3641`). At most [`MOST_STARTS`]; past it `starts_lost` is set.
    starts: Vec<usize>,
    /// Whether more operands stood since the last operator than `starts` keeps, so that an
    /// operator with operands left over is written out whole rather than without its leftovers.
    starts_lost: bool,
    /// Whether this reader is inside a text object: after a `BT`, until the `ET` that ends it.
    in_text: bool,
    /// The text matrices this reader holds, as far as the walk can state them (`doc/adr/3661`).
    matrices: Matrices,
    /// Where the text object this write adds around operators this reader ran outside one is to
    /// end, one past the last of them, while one is open (`doc/adr/3661`).
    added_text_object: Option<usize>,
    /// The text leading this reader holds, where the walk can state it (`doc/adr/3801`).
    leading: Leading,
}

/// What the walk knows of the text leading Tl (ISO 32000-2 §9.3.5), which `T*`, `'` and `"` move
/// the line matrix by, and of the leadings each `q` saved with the graphics state (§8.4.2), so that
/// a move by the leading from a matrix the walk can state is one it states too (`doc/adr/3801`).
#[derive(Default)]
struct Leading {
    /// The leading in force, in the single precision the reader holds it in; `None` where it
    /// cannot be stated: in a stream that inherits the invoking state's (§8.10.1), after a `TL`
    /// or `TD` this reader may not have run as written, or once more states were saved than the
    /// walk keeps.
    held: Option<f32>,
    /// The leading each `q` saved, innermost last, at most [`MOST_SAVED`].
    saved: Vec<Option<f32>>,
    /// Whether a `q` was met with [`MOST_SAVED`] states saved, past which the walk keeps no
    /// account of what a `Q` restores.
    lost: bool,
}

impl Leading {
    /// §9.3.1 Table 103's `TL`: "Initial value: 0", the leading a page's content starts with.
    const fn initial() -> Self {
        Self {
            held: Some(0.0),
            saved: Vec::new(),
            lost: false,
        }
    }

    /// The leading after `q` (`saved` false) or `Q` (`saved` true) ran.
    fn saved_or_restored(&mut self, restored: bool) {
        if self.lost {
            return;
        }
        if restored {
            // A `Q` with nothing saved restores nothing, as the reader ignores it.
            if let Some(held) = self.saved.pop() {
                self.held = held;
            }
        } else if self.saved.len() < MOST_SAVED {
            self.saved.push(self.held);
        } else {
            self.lost = true;
            self.held = None;
        }
    }

    /// The leading after a `TL` or `TD` set it to `value`, `None` where the walk cannot state it.
    fn set(&mut self, value: Option<f32>) {
        self.held = value.filter(|_| !self.lost);
    }
}

/// How many saved states [`Leading`] keeps: the reader's own bound on the graphics state stack
/// (`pdf_model::content`'s `MAX_STATE_DEPTH`), past which it refuses a `q`. A walk that meets a
/// deeper stack stops stating the leading rather than guess which `Q` restores what.
const MOST_SAVED: usize = 256;

/// What the walk knows of this reader's text matrix Tm and text line matrix Tlm (ISO 32000-2
/// §9.4.2), so that a text object added around operators it ran outside one can start where they
/// were drawn (`doc/adr/3661`).
#[derive(Default, Clone)]
enum Matrices {
    /// Both are the identity: the start of the content, or after an `ET` that ended a text
    /// object, which this reader resets them to (ADR 3353) and a `BT` sets them to.
    #[default]
    Identity,
    /// Both are a matrix the walk can state: the one a `Tm` set, or one a `Td` or `TD` composed
    /// onto a matrix it could state (`doc/adr/3736`).
    Stated(StatedMatrix),
    /// A glyph was shown since the text line matrix was set, so the text matrix has moved past it
    /// by the glyphs' widths, which the walk does not measure; the line matrix is the one held.
    Shown(Box<Self>),
    /// A move the walk does not compute: `Td`, `TD`, `T*`, `'` or `"` from a matrix or by a
    /// leading it cannot state, or one of them or a `Tm` this reader may not have run as written.
    Moved,
}

impl Matrices {
    /// What a `BT` added in front of `word` must state for the page to draw the same: nothing
    /// where `word` sets both matrices itself or this reader holds the identity `BT` sets, a `Tm`
    /// where it holds a matrix a `Tm` stated, and `None` where `word` reads a matrix the walk
    /// cannot state.
    fn opening(&self, word: &[u8]) -> Option<Vec<u8>> {
        let line = |held: &Self| match held {
            Self::Identity => Some(OPENS.to_vec()),
            Self::Stated(matrix) => {
                let mut bytes = OPENS.to_vec();
                bytes.extend_from_slice(&matrix.written);
                bytes.extend_from_slice(b" Tm ");
                Some(bytes)
            }
            Self::Shown(_) | Self::Moved => None,
        };
        match (word, self) {
            (b"Tm", _) => Some(OPENS.to_vec()),
            // Table 106 and Table 107: these read the line matrix alone and set the text matrix
            // from it.
            (b"Td" | b"TD" | b"T*" | b"'" | b"\"", Self::Shown(held)) => line(held),
            _ => line(self),
        }
    }

    /// The matrices after `word` ran, `stated` being a `Tm`'s six operands, or the move a `Td`
    /// or `TD` composes, where this reader ran it as written.
    ///
    /// Table 106's `Td` sets Tm and Tlm to the product of a translation by its operands and the
    /// line matrix held, so a move from a matrix the walk can state is one it can state too:
    /// computed as the reader computes it, in single precision, and written as the shortest
    /// decimal that reads back as the same number (`doc/adr/3736`). Table 106's `T*` "has the
    /// same effect as the code 0 - T l TD", and Table 107's `'` and `"` begin with it, so each is
    /// the move by the leading the walk states (`doc/adr/3801`); `'` and `"` then show a glyph.
    fn after(&mut self, word: &[u8], stated: Option<Moved>) {
        *self = match (word, stated) {
            (b"Tm", Some(Moved::To(matrix))) => Self::Stated(matrix),
            (b"Td" | b"TD" | b"T*", Some(Moved::By(x, y))) => self.moved_by(x, y),
            (b"'" | b"\"", Some(Moved::By(x, y))) => match self.moved_by(x, y) {
                Self::Moved => Self::Moved,
                line => Self::Shown(Box::new(line)),
            },
            (b"Tm" | b"Td" | b"TD" | b"T*" | b"'" | b"\"", _) => Self::Moved,
            (b"Tj" | b"TJ", _) => match std::mem::take(self) {
                Self::Moved => Self::Moved,
                shown @ Self::Shown(_) => shown,
                held => Self::Shown(Box::new(held)),
            },
            _ => return,
        };
    }

    /// The matrices after a move by `x` and `y` from the line matrix held: Table 106's
    /// translation times it, where the walk can state it.
    fn moved_by(&mut self, x: f32, y: f32) -> Self {
        let line = match std::mem::take(self) {
            Self::Shown(held) => *held,
            held => held,
        };
        let value = match &line {
            Self::Identity => Some(Transform::IDENTITY),
            Self::Stated(matrix) => Some(matrix.value),
            Self::Shown(_) | Self::Moved => None,
        };
        value
            .and_then(|value| StatedMatrix::of(Transform::translate(x, y).then(value)))
            .map_or(Self::Moved, Self::Stated)
    }
}

/// A text matrix the walk can state: its value as this reader holds it, and the six operands of
/// the `Tm` that sets it, each spelled as ISO 32000-2 §7.3.3 spells a number.
#[derive(Clone)]
struct StatedMatrix {
    /// The six operands, each preceded by a space.
    written: Vec<u8>,
    /// The matrix, in the single precision the reader holds it in.
    value: Transform,
}

impl StatedMatrix {
    /// `value` written as the shortest decimals that read back as its numbers, or `None` where
    /// one is not finite.
    fn of(value: Transform) -> Option<Self> {
        let mut written = Vec::new();
        for number in [value.a, value.b, value.c, value.d, value.e, value.f] {
            written.push(b' ');
            written.extend_from_slice(super::operand(number).ok()?.as_bytes());
        }
        Some(Self { written, value })
    }
}

/// What a text-positioning operator or a `TL` this reader ran as written states: the matrix a `Tm`
/// sets, the move a `Td`, `TD`, `T*`, `'` or `"` makes, or the leading a `TL` sets.
enum Moved {
    /// A `Tm`'s matrix.
    To(StatedMatrix),
    /// A `Td`'s or `TD`'s two operands, narrowed as the reader narrows them, or the move by the
    /// leading a `T*`, `'` or `"` makes.
    By(f32, f32),
    /// A `TL`'s operand, narrowed the same way (`doc/adr/3801`).
    Leading(f32),
}

/// The bytes that close one marked-content section left open at its stream's end (`doc/adr/3734`):
/// an end-of-line marker first, so that a comment the stream ends in cannot take the operator.
const CLOSES_A_SECTION: &[u8] = b"\nEMC";

/// The bytes that open a text object this write adds (`doc/adr/3661`).
const OPENS: &[u8] = b" BT ";

/// The bytes that close one.
const CLOSES: &[u8] = b" ET ";

/// Whether ISO 32000-2 §9.4.1 lets `word` stand inside a text object, so that one added around
/// operators this reader ran outside one may hold it: the text state, text-positioning and
/// text-showing operators, and "those related to the general graphics state, colour, and
/// marked-content". A marked-content operator that opens or closes a sequence ends the added
/// object instead, so that no sequence crosses its edge.
fn stands_in_a_text_object(word: &[u8]) -> bool {
    matches!(
        word,
        b"Tc"
            | b"Tw"
            | b"Tz"
            | b"TL"
            | b"Tf"
            | b"Tr"
            | b"Ts"
            | b"Td"
            | b"TD"
            | b"Tm"
            | b"T*"
            | b"Tj"
            | b"TJ"
            | b"'"
            | b"\""
            | b"w"
            | b"J"
            | b"j"
            | b"M"
            | b"d"
            | b"ri"
            | b"i"
            | b"gs"
            | b"CS"
            | b"cs"
            | b"SC"
            | b"SCN"
            | b"sc"
            | b"scn"
            | b"G"
            | b"g"
            | b"RG"
            | b"rg"
            | b"K"
            | b"k"
            | b"MP"
            | b"DP"
    )
}

/// Whether `word` is one of the text-positioning or text-showing operators §9.4.2 and §9.4.3 say
/// "shall only appear within text objects".
fn only_in_a_text_object(word: &[u8]) -> bool {
    matches!(
        word,
        b"Td" | b"TD" | b"Tm" | b"T*" | b"Tj" | b"TJ" | b"'" | b"\""
    )
}

/// What one operator's place in the stream is to become.
#[derive(Clone, Copy)]
enum WrittenOut {
    /// The operator is left as the producer wrote it.
    Left,
    /// The operator is written out with all its operands, from the byte its first operand starts at.
    Whole(usize),
    /// Only the operands left over in front of the operator's own are written out: the place from
    /// the first of them to the first of its own (`doc/adr/3641`).
    LeftOver(usize, usize),
    /// The operator keyword, from `start` to `end`, is written as the device operator this reader
    /// drew it as, its operands kept (`doc/adr/3679`).
    Respelled(usize, usize, &'static [u8]),
}

/// How many operands' starts [`Walk`] keeps before one operator. Far below the interpreter's own
/// bound on one operator's operands, so that every operand it keeps is one the reader keeps too
/// and the last of them are the ones it gives the operator.
const MOST_STARTS: usize = 64;

/// How deep the open sections [`Walk`] follows may nest, so that a hostile stream of `BMC`s costs
/// a bound rather than memory.
const MOST_SECTIONS: usize = 1024;

impl Walk {
    /// The walk at the start of a stream that begins in `spaces` ([`streams_began_in`]): a page's
    /// content with §9.3.1's initial leading (`page`), any other stream with the one in force where
    /// it was invoked, which the walk does not state.
    fn starting_in(spaces: Spaces, page: bool) -> Self {
        Self {
            grammar: Grammar::starting_in(spaces),
            leading: if page {
                Leading::initial()
            } else {
                Leading::default()
            },
            ..Self::default()
        }
    }

    /// The walk at the start of the next stream of the same page's content.
    fn into_the_next_stream(mut self) -> Self {
        self.operands_elsewhere |= self.operands_from.take().is_some();
        self.starts.clear();
        self
    }

    /// The operands since the last operator are spent: an operator ran or a keyword cleared them.
    fn operands_spent(&mut self) {
        self.operands_from = None;
        self.operands_elsewhere = false;
        self.starts.clear();
        self.starts_lost = false;
    }

    /// Whether no operand stands since the last operator, in this stream or an earlier one.
    const fn has_no_operands(&self) -> bool {
        self.operands_from.is_none() && !self.operands_elsewhere && !self.starts_lost
    }

    /// A keyword that is no operator ended the operands before it, as the reader's clearing does.
    fn cleared(&mut self) {
        self.grammar.boundary();
        self.operands_spent();
    }

    /// One token at the top level of the content that is an operand, or a keyword standing where
    /// one would, `start` being where the operand began (an array's or a dictionary's opening
    /// bracket for its closing one): a run the reader draws as a number (`drawn`) is the operand it draws, and any
    /// other keyword ends the operands before it, as the survey's walk reads them
    /// (`drawn_as_a_number`).
    fn operand(&mut self, token: &Token<'_>, start: usize, drawn: bool) {
        let operand = match token {
            Token::Name(name) => {
                self.last_name = Some(name.clone());
                Operand::Name
            }
            Token::String(_) => Operand::Text,
            Token::Integer(_) | Token::Real(_) => Operand::Number,
            Token::DictClose => Operand::Dictionary,
            Token::ArrayClose | Token::Keyword(b"true" | b"false" | b"null") => Operand::Other,
            Token::Keyword(_) if drawn => Operand::Number,
            Token::Keyword(_) => {
                self.cleared();
                return;
            }
            Token::ArrayOpen | Token::DictOpen => return,
        };
        if matches!(token, Token::Keyword(_)) {
            self.operands_from.get_or_insert(start);
        }
        if self.starts.len() < MOST_STARTS {
            self.starts.push(start);
        } else {
            self.starts_lost = true;
        }
        self.grammar.operand(operand);
    }

    /// An operator keyword at the top level of the content, `start` its first byte and `end` one
    /// past its last: where the grammar finds one of [`WRITTEN_OUT_WHERE_DRAWN_THE_SAME`]'s
    /// repairs at it and its operands all stand in this stream, what of it is written out, and the
    /// `BT` or `ET` this write adds in front of it.
    ///
    /// Where the one repair is §7.8.2's operands left over, only those are: the clause's "all of
    /// the operands needed by an operator shall immediately precede that operator" makes the last
    /// of them the operator's own, and the reader gives it exactly those and drops the rest, so
    /// the operator stays with its own operands and draws what it drew (`doc/adr/3641`).
    ///
    /// A text-positioning or text-showing operator this reader ran outside a text object is not
    /// written out where a text object can be added around it that starts with the matrices it
    /// was drawn by (`Matrices::opening`): the added object runs to the last such operator before
    /// one §9.4.1 keeps out of a text object (`doc/adr/3661`).
    fn operator(
        &mut self,
        document: &Document,
        resources: &pdf_syntax::object::Dictionary,
        data: &[u8],
        word: &[u8],
        (start, end): (usize, usize),
    ) -> (WrittenOut, Option<Place>) {
        let from = self.operands_from.unwrap_or(start);
        let whole_here = !self.operands_elsewhere;
        let given = (!self.starts_lost).then_some(self.starts.len());
        let own_from = signature(word)
            .and_then(|signature| signature.operand_count())
            .filter(|_| !self.starts_lost)
            .and_then(|takes| self.starts.len().checked_sub(takes))
            .and_then(|first_own| self.starts.get(first_own).copied())
            .unwrap_or(start);
        self.operands_spent();
        let added = self.text_object_at(word, from, whole_here, end);
        let in_the_added_object =
            !self.in_text && only_in_a_text_object(word) && self.added_text_object.is_some();
        let Some(signature) = signature(word) else {
            self.grammar.boundary();
            return (WrittenOut::Left, added);
        };
        let found = self.grammar.operator(signature);
        if let (b"CS" | b"cs", Some(name)) = (word, self.last_name.take()) {
            self.grammar
                .select_space(word == b"CS", space_components(document, resources, &name));
        }
        // The added text object answers §9.4.1's class at this operator (`doc/adr/3661`).
        let answered = |class: &&'static pdf_syntax::RepairClass| {
            !(in_the_added_object
                && class.name == pdf_model::repair::TEXT_OPERATOR_OUTSIDE_A_TEXT_OBJECT.name)
        };
        self.text_matrices_after(word, found, data, own_from, start);
        let mut classes = found.classes().filter(answered).peekable();
        let only_left_over = found
            .classes()
            .filter(answered)
            .all(|class| class.name == pdf_model::repair::OPERANDS_LEFT_OVER.name);
        let written = whole_here
            && classes.peek().is_some()
            && classes.all(|class| WRITTEN_OUT_WHERE_DRAWN_THE_SAME.contains(&class.name));
        let only_colour = found.classes().all(|class| {
            class.name == pdf_model::repair::COLOUR_OPERANDS_DO_NOT_FIT_THE_SPACE.name
        });
        let place = match (written, only_left_over && from < own_from) {
            (false, _) => WrittenOut::Left,
            (true, _) if only_colour => device_operator(word, given)
                .map_or(WrittenOut::Whole(from), |device| {
                    WrittenOut::Respelled(start, end, device)
                }),
            (true, true) => WrittenOut::LeftOver(from, own_from),
            (true, false) => WrittenOut::Whole(from),
        };
        let place = match word {
            // Table 352's `BMC` and `BDC` each begin a sequence "terminated by a balancing EMC
            // operator": a section whose opening operator is written out is written out whole,
            // or the `EMC` left behind would balance nothing (`doc/adr/3615`). One that keeps its
            // operator keeps its `EMC`.
            b"BMC" | b"BDC" => {
                if self.sections.len() < MOST_SECTIONS {
                    self.sections.push(matches!(place, WrittenOut::Whole(_)));
                    place
                } else {
                    WrittenOut::Left
                }
            }
            // An `EMC` with no section open balances nothing, and the reader ignores it: it is
            // written out alone where the page draws the same.
            b"EMC" => match self.sections.pop() {
                Some(true) => WrittenOut::Whole(start),
                Some(false) => match place {
                    WrittenOut::LeftOver(..) => place,
                    _ => WrittenOut::Left,
                },
                None => place,
            },
            _ => place,
        };
        (place, added)
    }

    /// The `BT` or `ET` this write adds in front of `word`, whose operands begin at `from`
    /// (`doc/adr/3661`).
    ///
    /// A text-positioning or text-showing operator this reader ran outside a text object opens
    /// one where none is open, its operands all stand in this stream, and the matrices it reads
    /// are ones the walk can state; it then runs to `end`. An operator §9.4.1 keeps out of a text
    /// object closes the one open, and a `BT` does too; an `ET` that this reader ran outside a
    /// text object, and so ignored, closes it with no `ET` added, its own ending the object.
    fn text_object_at(
        &mut self,
        word: &[u8],
        from: usize,
        whole_here: bool,
        end: usize,
    ) -> Option<Place> {
        if !self.in_text && only_in_a_text_object(word) {
            let opened = match self.added_text_object {
                Some(_) => None,
                None => self
                    .matrices
                    .opening(word)
                    .filter(|_| whole_here)
                    .map(|added| Place::text_object(from, added)),
            };
            if opened.is_some() || self.added_text_object.is_some() {
                self.added_text_object = Some(end);
            }
            return opened;
        }
        if stands_in_a_text_object(word) {
            return None;
        }
        if word == b"ET" && !self.in_text {
            self.added_text_object = None;
            return None;
        }
        self.end_added_text_object()
    }

    /// The `EMC` closing each marked-content section this reader left open where its content
    /// stream ended at `end`, a place of no bytes with the bytes added, or `None` where none is
    /// open or one cannot be closed there (`doc/adr/3734`).
    ///
    /// Table 352 has `BMC` and `BDC` each begin a sequence "terminated by a balancing EMC
    /// operator", and §14.6.1 has each sequence "entirely contained within a single content
    /// stream"; this reader ends the sequence where the stream ends, which is what an `EMC` there
    /// states. A section whose opening operator is written out needs none. None is added inside a
    /// text object the producer left open (the sequence would cross its edge), after operands no
    /// operator took (the `EMC` would take them), or past the sections the walk follows.
    fn sections_left_open(&mut self, end: usize) -> Option<Place> {
        let open = self.sections.iter().filter(|written| !**written).count();
        let closable = open > 0
            && !self.in_text
            && self.has_no_operands()
            && self.sections.len() < MOST_SECTIONS;
        self.sections.clear();
        closable.then(|| Place::text_object(end, CLOSES_A_SECTION.repeat(open)))
    }

    /// An inline image ends the operands before it, and §9.4.1 keeps it out of a text object, so
    /// the one this write added ends in front of it: the `ET` closing it, where one is open.
    fn inline_image(&mut self) -> Option<Place> {
        self.grammar.boundary();
        self.operands_spent();
        self.end_added_text_object()
    }

    /// The `ET` closing the text object this write added, where one is open: at a `BT`, an
    /// operator §9.4.1 keeps out of a text object, an inline image, or the end of the stream.
    fn end_added_text_object(&mut self) -> Option<Place> {
        self.added_text_object
            .take()
            .map(|at| Place::text_object(at, CLOSES.to_vec()))
    }

    /// Follows `word` through this reader's text object and its matrices (Table 105, Table 106):
    /// a `BT` begins an object at the identity, an `ET` ending one discards its matrices to the
    /// identity (ADR 3353), and a stray `ET` acts on nothing. A `Tm` this reader ran as written
    /// states its six numbers, respelled as §7.3.3 spells them; the operands are the last six
    /// before `start`, from `own_from`. The leading is followed through `TL`, `TD`, `q` and `Q`,
    /// so that `T*`, `'` and `"` state their move by it (`doc/adr/3801`).
    fn text_matrices_after(
        &mut self,
        word: &[u8],
        found: pdf_model::content::grammar::Found,
        data: &[u8],
        own_from: usize,
        start: usize,
    ) {
        match word {
            b"BT" => {
                self.in_text = true;
                self.matrices = Matrices::Identity;
            }
            b"ET" if self.in_text => {
                self.in_text = false;
                self.matrices = Matrices::Identity;
            }
            b"q" | b"Q" => self.leading.saved_or_restored(word == b"Q"),
            b"Tm" | b"Td" | b"TD" | b"TL" | b"T*" | b"'" | b"\"" => {
                let as_written = found.classes().all(|class| {
                    class.name == pdf_model::repair::TEXT_OPERATOR_OUTSIDE_A_TEXT_OBJECT.name
                        || class.name == pdf_model::repair::OPERANDS_LEFT_OVER.name
                });
                let span = data.get(own_from..start).unwrap_or_default();
                let stated = as_written.then(|| moved(word, span)).flatten();
                match (word, &stated) {
                    // Table 106: `TD` "shall have the same effect as this code: - t y TL t x t y Td".
                    (b"TD", Some(Moved::By(_, y))) => self.leading.set(Some(-*y)),
                    (b"TL", Some(Moved::Leading(value))) => self.leading.set(Some(*value)),
                    (b"TD" | b"TL", _) => self.leading.set(None),
                    _ => {}
                }
                let stated = match (word, stated) {
                    // Table 106's `T*` "has the same effect as the code 0 - T l TD", and Table
                    // 107's `'` and `"` begin with it; `"`'s `aw` and `ac` set `Tw` and `Tc`, text
                    // state a `BT` does not reset (§9.3.1), so nothing added restates them.
                    (b"T*" | b"'" | b"\"", _) if as_written => {
                        self.leading.held.map(|leading| Moved::By(0.0, -leading))
                    }
                    (_, stated) => stated,
                };
                self.matrices.after(word, stated);
            }
            _ => self.matrices.after(word, None),
        }
    }
}

/// What the `Tm`, `Td`, `TD` or `TL` `word` whose own operands are `span` states, or `None` where
/// they are not the numbers it takes, each read whole.
fn moved(word: &[u8], span: &[u8]) -> Option<Moved> {
    let numbers = numbers_in(span)?;
    // The reader narrows each operand to single precision (`pdf_model::content::run`).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the reader's own narrowing, which the matrix must match to draw the same"
    )]
    let narrowed: Vec<f32> = numbers.iter().map(|number| *number as f32).collect();
    if narrowed.iter().any(|number| !number.is_finite()) {
        return None;
    }
    match (word, narrowed.as_slice()) {
        (b"Tm", &[x0, x1, x2, x3, x4, x5]) => Some(Moved::To(StatedMatrix {
            written: six_numbers(span)?,
            value: Transform::new(x0, x1, x2, x3, x4, x5),
        })),
        (b"Td" | b"TD", &[x, y]) => Some(Moved::By(x, y)),
        (b"TL", &[value]) => Some(Moved::Leading(value)),
        _ => None,
    }
}

/// Every token of `span` read as a number, or `None` where one is not a number the lexer reads
/// whole.
fn numbers_in(span: &[u8]) -> Option<Vec<f64>> {
    let mut lexer = Lexer::new(span);
    let mut out = Vec::new();
    loop {
        lexer.skip_whitespace();
        let Some(token) = lexer.next_token() else {
            return Some(out);
        };
        if lexer.salvaged().is_some() {
            return None;
        }
        out.push(match token {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the reader reads an integer operand as this double"
            )]
            Token::Integer(value) => value as f64,
            Token::Real(value) => value,
            _ => return None,
        });
    }
}

/// A `Tm`'s six operands, each respelled by `pdf_syntax::write::object` as §7.3.3 spells it and
/// reads back to the same value, or `None` where `span` is not six numbers the lexer reads whole.
fn six_numbers(span: &[u8]) -> Option<Vec<u8>> {
    let mut lexer = Lexer::new(span);
    let mut out = Vec::new();
    for _ in 0..6 {
        lexer.skip_whitespace();
        let value = match lexer.next_token()? {
            Token::Integer(value) => Object::Integer(value),
            Token::Real(value) => Object::Real(value),
            _ => return None,
        };
        if lexer.salvaged().is_some() {
            return None;
        }
        out.push(b' ');
        pdf_syntax::write::object(&value, &mut out);
    }
    lexer.skip_whitespace();
    lexer.next_token().is_none().then_some(out)
}

/// Every run in one decoded content stream that this reader draws as a number.
///
/// Read by the same lexer the reader reads with, the salvage judged by the reader's own rule
/// ([`draws_the_salvage`]), and §8.9.7's inline image data passed over as `super::hexadecimal`
/// passes it. Each run is at least two bytes — a digit and what the number leaves out — so the
/// list is bounded by half the stream's length, and by `most`: `None` where the stream holds more
/// than `most` runs.
///
/// A lone delimiter is a place too, with no number, where `delimiters` is set and nothing stands
/// before it since the last operator and it is in no array or dictionary — the one place the
/// reader's clearing of the operands clears nothing. So, under the same conditions and where
/// `undefined` is set, is an operator no table defines (`doc/adr/3698`). The flag beside the list says whether either was left.
///
/// Where `operators` is set, an operator `pdf_model`'s grammar (ADR 3542) finds one of
/// [`WRITTEN_OUT_WHERE_DRAWN_THE_SAME`]'s repairs at is a place too, from its first operand to its
/// own last byte; whether the page draws the same without it is [`drawn_the_same`]'s question.
/// So is each `BT` and `ET` added around text operators this reader ran outside a text object,
/// a place of no bytes with the bytes added (`doc/adr/3661`).
#[expect(
    clippy::too_many_lines,
    reason = "one loop over the stream's tokens whose arms share the lexer, the depth, the operands \
              pending and the walk; an arm moved out would take all four and the list with it"
)]
fn salvaged_runs(
    document: &Document,
    data: &[u8],
    resources: &pdf_syntax::object::Dictionary,
    most: usize,
    (delimiters, operators, undefined): (bool, bool, bool),
    walk: &mut Walk,
) -> Option<(Vec<Place>, bool)> {
    let mut lexer = Lexer::new(data);
    let mut out = Vec::new();
    let mut left = false;
    // The operands since the last operator, and how deep in arrays and dictionaries the lexer is.
    let mut pending = 0_usize;
    let mut depth = 0_usize;
    // Where the array or dictionary open at the top level began, which is where it starts as
    // one operand.
    let mut opened_at = 0_usize;
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let strayed = lexer.hexadecimal_strings().strayed;
        let Some(token) = lexer.next_token() else {
            break;
        };
        if matches!(token, Token::Keyword(b"BI")) {
            let scan =
                pdf_model::inline_image::scan(document, data, lexer.position(), resources, true);
            lexer.seek(scan.resume);
            pending = 0;
            added(&mut out, most, walk.inline_image().filter(|_| operators))?;
            if operators {
                entries_set_aside(&mut out, most, scan)?;
            }
            continue;
        }
        // An operator no table defines, with no operand before it in this stream or an earlier
        // one, draws nothing, and is written as the white space it was read as where the write
        // then answers every operator (`undefined`), under the proof (`doc/adr/3698`); where the
        // proof refuses, `write_with` writes again without it.
        // It clears nothing and ends no text object this write adds, as the reader read it.
        // Anywhere else it is left, and its page keeps the answer it had.
        if let Token::Keyword(word) = &token
            && lexer.salvaged().is_none()
            && is_an_undefined_operator(word)
        {
            if depth == 0 && pending == 0 && undefined && walk.has_no_operands() {
                placed(
                    &mut out,
                    most,
                    Place::undefined_operator(start, lexer.position()),
                )?;
                walk.cleared();
                continue;
            }
            left = true;
        }
        if depth == 0
            && !matches!(
                token,
                Token::Keyword(_) | Token::ArrayClose | Token::DictClose
            )
        {
            walk.operands_from.get_or_insert(start);
        }
        match &token {
            Token::ArrayOpen | Token::DictOpen => {
                if depth == 0 {
                    opened_at = start;
                }
                depth = depth.saturating_add(1);
            }
            Token::ArrayClose | Token::DictClose => {
                depth = depth.saturating_sub(1);
                pending = pending.saturating_add(1);
                if depth == 0 {
                    walk.operand(&token, opened_at, false);
                }
            }
            Token::Keyword(word) if is_a_lone_delimiter(word) => {
                if delimiters && depth == 0 && pending == 0 {
                    placed(
                        &mut out,
                        most,
                        Place::white_space(start, lexer.position(), false),
                    )?;
                } else {
                    left = true;
                    pending = 0;
                }
                walk.cleared();
                continue;
            }
            Token::Keyword(word)
                if depth == 0
                    && lexer.salvaged().is_none()
                    && !matches!(*word, b"true" | b"false" | b"null") =>
            {
                pending = 0;
                let (written, text_object) =
                    walk.operator(document, resources, data, word, (start, lexer.position()));
                if operators {
                    added(&mut out, most, text_object)?;
                    written_out(&mut out, most, data, written, lexer.position())?;
                }
                continue;
            }
            _ => {
                pending = pending.saturating_add(1);
                if depth == 0 {
                    walk.operand(
                        &token,
                        start,
                        lexer.salvaged().is_some_and(draws_the_salvage),
                    );
                }
            }
        }
        let end = lexer.position();
        // A hexadecimal string holding a byte that is neither a digit nor white space, which this
        // reader skipped: written without it, proved like an operator (`doc/adr/3735`).
        if operators
            && lexer.hexadecimal_strings().strayed > strayed
            && let Some(digits) = data.get(start..end).and_then(without_stray_bytes)
        {
            placed(&mut out, most, Place::hexadecimal(start, end, digits))?;
            continue;
        }
        let number = if lexer.salvaged().is_some_and(draws_the_salvage) {
            number_drawn(&token)
        } else {
            respelled_exponent(&token, data.get(start..end))
        };
        if let Some(number) = number {
            placed(&mut out, most, Place::number(start, end, number))?;
        }
    }
    // A text object added runs no further than its stream: the next stream of a `/Contents`
    // array opens its own where it needs one.
    added(
        &mut out,
        most,
        walk.end_added_text_object().filter(|_| operators),
    )?;
    Some((out, left))
}

/// Adds a place for each entry of an inline image dictionary this reader did not read, its key
/// stated twice: the entry it read stays, and the other is written as one white-space character,
/// proved like an operator (`doc/adr/3678`). `None` where `out` already holds `most`.
fn entries_set_aside(
    out: &mut Vec<Place>,
    most: usize,
    scan: pdf_model::inline_image::Scan,
) -> Option<()> {
    if scan.image.is_err() {
        return Some(());
    }
    let mut set_aside = scan.set_aside;
    set_aside.sort_unstable();
    for (from, to) in set_aside {
        placed(out, most, Place::white_space(from, to, true))?;
    }
    // The bytes between the end the image's own decoding states and the `EI` this reader found
    // by searching are bytes it held and never decoded: cut, the data ends where every answer of
    // §8.9.7's reading predicts, proved like an operator (`doc/adr/3735`).
    if let Some((from, to)) = scan.unread_data {
        placed(out, most, Place::cut(from, to))?;
    }
    Some(())
}

/// A hexadecimal string, `<` to `>`, with every byte that is neither a hexadecimal digit nor white
/// space removed: what this reader read it as, since it skips such a byte and reads the digits
/// around it (`pdf_syntax::Lexer`). `None` where `string` is not one closed hexadecimal string.
fn without_stray_bytes(string: &[u8]) -> Option<Vec<u8>> {
    let inside = string.strip_prefix(b"<")?.strip_suffix(b">")?;
    let mut out = Vec::with_capacity(string.len());
    out.push(b'<');
    out.extend(
        inside
            .iter()
            .copied()
            .filter(|byte| byte.is_ascii_hexdigit() || is_white_space(*byte)),
    );
    out.push(b'>');
    Some(out)
}

/// Whether `byte` is one of ISO 32000-2 §7.2.3's six white-space characters (Table 1).
const fn is_white_space(byte: u8) -> bool {
    matches!(byte, 0 | b'\t' | b'\n' | 0x0c | b'\r' | b' ')
}

/// Adds the place an operator is written out at, `end` being one past the operator's last byte:
/// the operator with its operands, or the operands left over in front of its own.
///
/// A run among the operands written out goes with them, and one among the operator's own stays
/// after it, so the places stay in order and apart. `None` where `out` already holds `most`.
fn written_out(
    out: &mut Vec<Place>,
    most: usize,
    data: &[u8],
    written: WrittenOut,
    end: usize,
) -> Option<()> {
    let (from, to) = match written {
        WrittenOut::Left => return Some(()),
        WrittenOut::Respelled(start, end, device) => {
            return placed(out, most, Place::respelled(start, end, device.to_vec()));
        }
        WrittenOut::Whole(from) => (from, end),
        // The white space before the operator's own first operand stays and parts the place
        // from it, so the place ends where the last leftover does.
        WrittenOut::LeftOver(from, to) => (
            from,
            data.get(from..to)
                .and_then(|span| span.iter().rposition(|byte| !is_white_space(*byte)))
                .and_then(|last| from.checked_add(last)?.checked_add(1))
                .unwrap_or(to),
        ),
    };
    // A `BT` added at `from` stays in front of the place (`doc/adr/3661`).
    let at = out
        .partition_point(|place| place.start < from || (place.start == from && place.end == from));
    let own: Vec<Place> = out.drain(at..).filter(|place| place.start >= to).collect();
    placed(out, most, Place::white_space(from, to, true))?;
    out.extend(own);
    Some(())
}

/// The number this reader draws for a salvaged run, in ISO 32000-2 §7.3.3's spelling.
fn number_drawn(token: &Token<'_>) -> Option<Vec<u8>> {
    let value = match token {
        Token::Integer(value) => Object::Integer(*value),
        Token::Real(value) => Object::Real(*value),
        _ => return None,
    };
    let mut number = Vec::new();
    // §7.3.3: "A PDF writer shall not use the PostScript language syntax for numbers with
    // non-decimal radices (such as 16#FFFE) or in exponential format (such as 6.02E23)."
    // `write::object` writes the shortest decimal that reads back as the same value, which
    // never uses an exponent.
    pdf_syntax::write::object(&value, &mut number);
    Some(number)
}

/// The device operator this reader draws `word`'s colour by where its `given` operands do not fit
/// the current space: ISO 32000-2 §8.6.8 has `SC`, `SCN`, `sc` and `scn` take as many operands
/// as the space has components, and where they are one, three or four this reader draws them in
/// the device space with that many — as the page defines it, its default colour space included
/// (§8.6.5.6) — which is what `G`, `RG` or `K` and their non-stroking forms draw (`doc/adr/3679`).
/// `None` for any other operator or count: the reader draws nothing for one, and it is written out.
fn device_operator(word: &[u8], given: Option<usize>) -> Option<&'static [u8]> {
    let stroking = match word {
        b"SC" | b"SCN" => true,
        b"sc" | b"scn" => false,
        _ => return None,
    };
    Some(match (given?, stroking) {
        (1, true) => b"G",
        (3, true) => b"RG",
        (4, true) => b"K",
        (1, false) => b"g",
        (3, false) => b"rg",
        (4, false) => b"k",
        _ => return None,
    })
}

/// The value a real written with an exponent was read as, in ISO 32000-2 §7.3.3's spelling, where
/// `run` is the run the lexer read it from and holds one (`doc/adr/3677`); `None` for every other
/// token.
///
/// §7.3.3 forbids the spelling to a writer and states nothing for a reader; this reader reads the
/// run as its value (`pdf_syntax::lexer`, ADR 0733), so the page is the page with that value in that
/// place, and the decimal that reads back as the same value draws what it drew.
fn respelled_exponent(token: &Token<'_>, run: Option<&[u8]>) -> Option<Vec<u8>> {
    let exponent = run?.iter().any(|byte| matches!(byte, b'e' | b'E'));
    if exponent && matches!(token, Token::Real(_)) {
        number_drawn(token)
    } else {
        None
    }
}

/// Adds the `BT` or `ET` a text object is opened or closed by (`doc/adr/3661`) in its place
/// among `out`'s, in front of any that starts where it stands: an `ET` is found only once the
/// operator ending the object is, after places between the two were added. `None` where `out`
/// already holds `most`; nothing is added for `None`.
fn added(out: &mut Vec<Place>, most: usize, place: Option<Place>) -> Option<()> {
    let Some(place) = place else {
        return Some(());
    };
    if out.len() == most {
        return None;
    }
    let at = out.partition_point(|other| other.start < place.start);
    out.insert(at, place);
    Some(())
}

/// Adds `place` to `out`, or `None` where `out` already holds `most`.
fn placed(out: &mut Vec<Place>, most: usize, place: Place) -> Option<()> {
    if out.len() == most {
        return None;
    }
    out.push(place);
    Some(())
}

/// How many components the space a `CS` or `cs` names has: a family's own count, or the
/// resources' `/ColorSpace` entry's, as the survey's walk counts them; `None` where neither says.
fn space_components(
    document: &Document,
    resources: &pdf_syntax::object::Dictionary,
    name: &[u8],
) -> Option<usize> {
    let named = Object::Name(Name::new(name.to_vec()));
    components(document, &named).or_else(|| {
        let spaces = document.get_key(resources, "ColorSpace");
        let entry = spaces
            .as_dict()?
            .get(std::str::from_utf8(name).ok()?)?
            .clone();
        components(document, &entry)
    })
}

#[cfg(test)]
mod tests {
    use super::is_drawn_as_a_number;

    #[test]
    fn a_run_whose_tail_names_no_operator_is_drawn_as_a_number_and_one_that_swallowed_a_fill_is_not()
     {
        assert!(is_drawn_as_a_number(b"33600.272.58"));
        assert!(is_drawn_as_a_number(b"3NT-DE290.58"));
        assert!(is_drawn_as_a_number(b"454\x85"));
        assert!(is_drawn_as_a_number(b"12pt"));
        // `f` is §8.2 Table 50's fill: the reader hands the run over whole and draws no number.
        assert!(!is_drawn_as_a_number(b"5f"));
        // §7.3.3 spells these whole, and a run the grammar reads nothing of is a keyword.
        assert!(!is_drawn_as_a_number(b"33600.272"));
        assert!(!is_drawn_as_a_number(b"--5"));
        assert!(!is_drawn_as_a_number(b"Frobnicate"));
    }
}
