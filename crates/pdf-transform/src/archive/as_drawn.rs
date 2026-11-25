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
//! reader did not read, its key stated twice, is written out (`doc/adr/3678`). Each of the last two
//! stands only where the page draws the same, as above.
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
//! the one added. Where the walk cannot state the matrix — a glyph's width moved the text matrix
//! past the line matrix, or a `Td` composed one — no object is added and the class keeps its
//! refusal. The proof by drawing both files decides as for an operator written out, and the loss
//! is `content-as-drawn`: no byte of the producer's is removed, but which matrix the text is drawn
//! by outside a text object is this reader's reading, not the standard's (`doc/adr/3661`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use pdf_archive::survey::{Survey, UnlistedOperator};
use pdf_model::Pages;
use pdf_model::content::grammar::{Grammar, Operand, components, signature};
use pdf_model::content::reader::draws_the_salvage;
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
/// out alone (Table 352), each only where the page draws the same. A
/// class the write takes nothing at — a section left open at its stream's end — stays in the
/// output, whose own check then refuses the write rather than let it be wrong. An inline image's
/// key stated twice is not an operator's, but its entry this reader did not read is written out
/// under the same proof (`doc/adr/3678`).
pub(super) const WRITTEN_OUT_WHERE_DRAWN_THE_SAME: [&str; 9] = [
    "model/operator-short-of-operands",
    "model/operand-of-the-wrong-type",
    "model/restore-with-nothing-saved",
    "model/text-operator-outside-a-text-object",
    "model/colour-operands-do-not-fit-the-space",
    "model/property-list-unresolved",
    "model/operands-left-over",
    "model/marked-content-unbalanced",
    "model/inline-image-key-twice",
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
    /// Whether a lone delimiter stood where removing it would change what this reader draws —
    /// after an operand, or inside an array or a dictionary — and so was left (`doc/adr/2859`).
    pub(super) delimiter_left: bool,
    /// How many lone delimiters were written as white space.
    delimiters_written: usize,
    /// The streams an operator this reader repaired was written out of (`doc/adr/3558`), or a
    /// text object was added to (`doc/adr/3661`).
    pub(super) written_out: BTreeSet<ObjectId>,
    /// How many text objects were added around operators this reader ran outside one
    /// (`doc/adr/3661`).
    pub(super) text_objects_added: usize,
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
    }

    /// Whether every content stream of the page or form `holder` is one this write rewrote and an
    /// operator was written out of one of them (`doc/adr/3558`).
    pub(super) fn wrote_out_of(&self, document: &Document, holder: ObjectId) -> bool {
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

    /// Whether every operator the survey reports under `spelling` is one this write answers: a run
    /// this reader draws as a number, or a lone delimiter where none was left.
    pub(super) fn answers(&self, spelling: &[u8]) -> bool {
        is_drawn_as_a_number(spelling) || (is_a_lone_delimiter(spelling) && !self.delimiter_left)
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
    // A lone delimiter is written only where the write then answers every operator: where some
    // other operator stays, the page keeps the answer it had without this write, the photograph's
    // included, and the delimiter stays with it (`doc/adr/2859`).
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
                is_drawn_as_a_number(&unlisted.spelling) || is_a_lone_delimiter(&unlisted.spelling)
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
    let mut walk = Walk::default();
    let mut walked: Option<ObjectId> = None;
    for (id, resources) in hexadecimal::streams(document, &pages, survey) {
        walk = match (walked, follows.get(&id)) {
            (Some(previous), Some(before)) if previous == *before => walk.into_the_next_stream(),
            _ => Walk::default(),
        };
        walked = Some(id);
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            // The survey read this stream through the content reader, which draws what a
            // damaged filter yields before the damage; the whole decode the bytes would be
            // written back from does not exist, so its runs stay and its page is photographed
            // where the profile allows (ADR 2803).
            out.unread.insert(id);
            continue;
        };
        let room = MOST_RUNS.saturating_sub(out.runs.len());
        let (places, delimiter_left) = salvaged_runs(
            document,
            &data,
            &resources,
            room,
            (delimiters, operators),
            &mut walk,
        )
        .ok_or(Because::NotBuiltYet(TOO_MANY_RUNS))?;
        out.delimiter_left |= delimiter_left;
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
    if !out.written_out.is_empty() && !drawn_the_same(document, &out.at, &out.written_out) {
        return Err(Because::NotBuiltYet(NOT_DRAWN_THE_SAME));
    }
    // A run left in a stream this write could not read is an operator the numbers do not answer,
    // and so is a delimiter left where it changes the drawing.
    out.answers_every_operator &= out.unread.is_empty() && !out.delimiter_left;
    Ok(out)
}

/// **The proof that writing the operators out changes nothing drawn** (`doc/adr/3558`): the
/// streams written are appended to the source as ISO 32000-2 §7.5.6's incremental update — or,
/// where the source's table was rebuilt by scanning, written with every other object into one new
/// file ([`whole_file`], `doc/adr/3662`) — and
/// every page that draws one of `written_out` is drawn from both files; the write stands where
/// each page's display list and text are the same. The pages' annotations are drawn too, since an
/// appearance stream is one of the streams this write reaches.
fn drawn_the_same(
    document: &Document,
    at: &BTreeMap<ObjectId, Object>,
    written_out: &BTreeSet<ObjectId>,
) -> bool {
    let pages = super::fenced::pages_drawing(document, written_out);
    if pages.is_empty() || pages.len() > MOST_PAGES_PROVED {
        return false;
    }
    // A file whose cross-reference table was rebuilt by scanning has no section an update could
    // chain to, so the second file is written whole from the objects this reader read
    // (`doc/adr/3662`).
    let bytes = if document.was_recovered() {
        whole_file(document, at)
    } else {
        pdf_syntax::write::incremental_update(document, at).ok()
    };
    let Some(after) =
        bytes.and_then(|bytes| Document::open_with_limits(bytes, document.limits()).ok())
    else {
        return false;
    };
    let (before_pages, after_pages) = (Pages::new(document), Pages::new(&after));
    pages.into_iter().all(|index| {
        let (Some(before), Some(written)) = (before_pages.get(index), after_pages.get(index))
        else {
            return false;
        };
        let was = pdf_model::content::interpret(document, &before);
        let is = pdf_model::content::interpret(&after, &written);
        was.display_list == is.display_list && was.text == is.text
    })
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
    /// Whether the place is the `BT` or the `ET` this write adds around operators this reader ran
    /// outside a text object (`doc/adr/3661`): `start` and `end` are the same byte, nothing of the
    /// producer's is replaced, and `number` holds the bytes added.
    text_object: bool,
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
}

/// What the walk knows of this reader's text matrix Tm and text line matrix Tlm (ISO 32000-2
/// §9.4.2), so that a text object added around operators it ran outside one can start where they
/// were drawn (`doc/adr/3661`).
#[derive(Default, Clone)]
enum Matrices {
    /// Both are the identity: the start of the content, or after an `ET` that ended a text
    /// object, which this reader resets them to (ADR 3353) and a `BT` sets them to.
    #[default]
    Identity,
    /// Both are the matrix a `Tm` set: its six numbers, each respelled as §7.3.3 spells it.
    Stated(Vec<u8>),
    /// A glyph was shown since the text line matrix was set, so the text matrix has moved past it
    /// by the glyphs' widths, which the walk does not measure; the line matrix is the one held.
    Shown(Box<Self>),
    /// A move the walk does not compute: `Td`, `TD`, `T*`, `'`, `"`, or a `Tm` this reader may
    /// not have run as written.
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
                bytes.extend_from_slice(matrix);
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

    /// The matrices after `word` ran, `stated` being a `Tm`'s six operands where this reader ran
    /// it as written.
    fn after(&mut self, word: &[u8], stated: Option<Vec<u8>>) {
        *self = match word {
            b"Tm" => stated.map_or(Self::Moved, Self::Stated),
            b"Td" | b"TD" | b"T*" | b"'" | b"\"" => Self::Moved,
            b"Tj" | b"TJ" => match std::mem::take(self) {
                Self::Moved => Self::Moved,
                shown @ Self::Shown(_) => shown,
                held => Self::Shown(Box::new(held)),
            },
            _ => return,
        };
    }
}

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
    /// before `start`, from `own_from`.
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
            b"Tm" => {
                let as_written = found.classes().all(|class| {
                    class.name == pdf_model::repair::TEXT_OPERATOR_OUTSIDE_A_TEXT_OBJECT.name
                        || class.name == pdf_model::repair::OPERANDS_LEFT_OVER.name
                });
                let stated = as_written
                    .then(|| six_numbers(data.get(own_from..start).unwrap_or_default()))
                    .flatten();
                self.matrices.after(word, stated);
            }
            _ => self.matrices.after(word, None),
        }
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
/// reader's clearing of the operands clears nothing. The flag beside the list says whether one was
/// left.
///
/// Where `operators` is set, an operator `pdf_model`'s grammar (ADR 3542) finds one of
/// [`WRITTEN_OUT_WHERE_DRAWN_THE_SAME`]'s repairs at is a place too, from its first operand to its
/// own last byte; whether the page draws the same without it is [`drawn_the_same`]'s question.
/// So is each `BT` and `ET` added around text operators this reader ran outside a text object,
/// a place of no bytes with the bytes added (`doc/adr/3661`).
fn salvaged_runs(
    document: &Document,
    data: &[u8],
    resources: &pdf_syntax::object::Dictionary,
    most: usize,
    (delimiters, operators): (bool, bool),
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
    Some(())
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
