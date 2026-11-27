//! ISO 19005-2 and ISO 19005-4 section 5.1 over the reader's log of repairs: what the rewrite
//! answers, what this conversion restates at a stated loss, and what it refuses by name (ADR 3521).
//!
//! # The answers, by what the reader repaired
//!
//! `pdf_archive` fails section 5.1 at every repair `pdf_syntax`, `pdf_model` and the content
//! survey logged on the source (ADR 3506, ADR 3541). Which answer a repair takes depends on where
//! its bytes go:
//!
//! - **A repair of the file's structure or of an object outside content** — a table rebuilt, a
//!   key repeated, an `endobj` missing, a number spelled with an exponent in a dictionary — is
//!   answered by the whole-file rewrite: the output is the repaired reading written in the
//!   serializer's own syntax, so the departure is not in it.
//! - **A repair of a page object's or page tree node's own entries** — a `/Rotate` of 45, a
//!   `/MediaBox` that is a name, a `/Resources` the page's ancestry never states — is left to the
//!   rewrites that already restate such a page (a box with no width written as the box this viewer
//!   draws, a recovered page given the values the reader resolved for it) and to the output's own
//!   check, whose refusal names each class the output still holds (ADR 3543).
//! - **A repair inside a content stream** is refused by name. The stream crosses this verb byte
//!   for byte, so the departure would be in the output too, and correcting it would edit the
//!   producer's page — ADR 0816's fence. The survey records these at [`pdf_syntax::Site::Content`],
//!   which is how they are told from the same class met in a dictionary.
//! - **Stream data the reader decoded only in part** (`syntax/stream-data-damaged`) is written as
//!   the bytes this reader decoded, re-encoded as one `FlateDecode`, at the loss
//!   [`super::Loss::DamagedDataAsDecoded`] — ADR 3473's construction for ASCII data cut before its
//!   marker, which this leaves to that row where it is the whole of a stream's damage. A
//!   `/DecodeParms` that does not pair with its `/Filter` is answered where a restatement
//!   re-encodes the stream anyway, and refused by name elsewhere unless a photograph takes the
//!   stream off its page. The streams searched are those the output carries, the ones the
//!   cross-reference table omits and the rewrite still copies included (ADR 3549).
//! - **Data no stage decodes at all** (`syntax/stream-data-undecodable`) has no bytes to write.
//!   Where the stream is a function — a role no rewrite here replaces — on a page no photograph
//!   takes, it is refused by name before anything is written (ADR 3551). Any other role's stream
//!   another rewrite may still drop — a metadata packet written afresh, a font program
//!   substituted — so it is left to the output's own check, whose refusal names each class the
//!   output's log still holds (ADR 3521).
//!
//! The log places stream damage at the whole file, because the decoder is handed data without its
//! object, so the streams are found here by decoding each one the output carries again.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::Target;
use pdf_archive::repairs::BASE_STANDARD;
use pdf_syntax::object::{Object, ObjectId, Stream};
use pdf_syntax::{Damage, Document, RepairClass, Site, repair};

use super::decision::{Because, Loss, Losses};
use super::rewrite::{Cut, reencoded_through_flate};

/// One stream written as the bytes this reader decoded, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamagedDataRestated {
    /// The stream, by object number.
    pub object: u32,
    /// Its generation.
    pub generation: u16,
    /// The first stage that reported damage, by the name the stream states it under.
    pub stage: String,
    /// What that stage reported, in a few words.
    pub damage: &'static str,
    /// How many bytes of stream data the source held.
    pub encoded: usize,
}

impl DamagedDataRestated {
    /// The report's sentence for this stream.
    #[must_use]
    pub fn sentence(&self) -> String {
        format!(
            "the stream object {} {} ({} bytes of stream data, its {} stage {}) is written as \
             the bytes this viewer decoded from it (damaged-data-as-decoded)",
            self.object, self.generation, self.encoded, self.stage, self.damage
        )
    }
}

/// How section 5.1 is answered for one document: by the rewrite alone where `at` is empty, and
/// otherwise by writing these streams as decoded too.
#[derive(Debug, Default)]
pub(super) struct Answered {
    /// Each damaged stream's replacement, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// What the report names, one entry per stream.
    pub(super) streams: Vec<DamagedDataRestated>,
    /// Each class of repair made inside a content stream, with the page or form whose content it
    /// was; [`settled`] answers or refuses them once the content rewrites are known.
    content: Vec<(&'static RepairClass, Option<ObjectId>)>,
    /// The streams the output would carry byte for byte that section 5.1 cannot admit and no
    /// rewrite here answers, each with the sentence that refuses it; [`settled`] refuses the
    /// first one no photograph takes off its page (ADR 3551).
    refused: Vec<(ObjectId, Because)>,
    /// Whether an operator this reader repaired was written out of its stream where the page
    /// draws the same without it, or a number in exponential form respelled, so that the row
    /// costs `content-as-drawn` (`doc/adr/3558`, `doc/adr/3677`).
    pub(super) operators_written_out: bool,
}

impl Answered {
    /// Whether the rewrite alone answers the row, so that nothing is lost.
    pub(super) fn costs_nothing(&self) -> bool {
        self.at.is_empty()
    }

    /// Whether the rewrites answer the row without the damaged-stream restatement's own decision:
    /// nothing restated, or an operator written out (`doc/adr/3558`).
    pub(super) fn answers_alone(&self) -> bool {
        self.costs_nothing() || self.operators_written_out
    }

    /// What the answer loses besides a photograph: the damaged streams restated, and the
    /// operators written out where the page draws the same (`doc/adr/3558`).
    pub(super) fn losses(&self) -> Losses {
        let mut losses = Losses::default();
        if !self.costs_nothing() {
            losses = losses.with(Loss::DamagedDataAsDecoded);
        }
        if self.operators_written_out {
            losses = losses.with(Loss::ContentAsDrawn);
        }
        losses
    }

    /// The `xmpMM:History` parameters naming every stream restated, the report's sentences
    /// joined; `None` where none is (ADR 3494's rule, for this loss).
    pub(super) fn history(&self) -> Option<String> {
        (!self.streams.is_empty()).then(|| {
            let sentences: Vec<String> = self
                .streams
                .iter()
                .map(DamagedDataRestated::sentence)
                .collect();
            sentences.join("; ")
        })
    }
}

/// Why no stream is restated where the file's own history cannot name it.
pub(super) const NO_PLACE_TO_RECORD: &str = "writing a stream whose data decodes only in part as \
     the bytes this viewer decoded names it in this file's own xmpMM:History, and this document's \
     XMP packet will not take that entry, so the stream is not restated and no file is written \
     (ADR 3522)";

/// Section 5.1's answer for this document, or the refusal that names why there is none.
///
/// Read after `pdf_archive::check`, which reads the whole file first, so the log is the whole
/// file's (ADR 3506).
pub(super) fn answer(
    document: &Document,
    report: &pdf_archive::Report,
    target: Target,
) -> Result<Answered, Because> {
    if !report
        .failures()
        .any(|judgement| judgement.id == BASE_STANDARD)
    {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    }
    let mut damaged = false;
    let mut unpaired = false;
    let mut undecodable = false;
    let mut content = Vec::new();
    for logged in document.repairs() {
        if !pdf_archive::repairs::breaches_row(logged.class.name, BASE_STANDARD) {
            continue;
        }
        if let Site::Content(holder) = logged.at {
            content.push((logged.class, holder));
            continue;
        }
        damaged |= logged.class.name == repair::STREAM_DATA_DAMAGED.name;
        unpaired |= logged.class.name == repair::DECODE_PARMS_MISMATCHED.name;
        undecodable |= logged.class.name == repair::STREAM_DATA_UNDECODABLE.name;
    }
    let mut answered = if damaged || unpaired || undecodable {
        restated(document, target, (damaged, unpaired, undecodable))?
    } else {
        Answered::default()
    };
    answered.content = content;
    Ok(answered)
}

/// The answer once the conversion's content rewrites are known: a repair inside a content stream
/// is answered where the bytes it was made in do not cross — the page is photographed, or every
/// content stream of the page or form is written again by the run rewrite and the class is one
/// that rewrite writes out (`super::as_drawn`), or by the integer rewrite and the class is an
/// integer too wide for 64 bits (`super::integer_reals`, ADR 3533) — and refused by name, under the fence, everywhere
/// else (ADR 3521). The output's own check is what holds a rewrite to having written every one.
///
/// A damaged stream only photographed pages named (`taken_off`) is not written, so it is not
/// restated either, and nothing is lost on its account.
pub(super) fn settled(
    answered: Result<Answered, Because>,
    document: &Document,
    (drawn, reals, resources_defined, blend_modes_restated): (
        Option<&super::as_drawn::Drawn>,
        Option<&super::integer_reals::Restated>,
        bool,
        bool,
    ),
    (photographed, taken_off): (&BTreeSet<ObjectId>, &BTreeSet<ObjectId>),
) -> Result<Answered, Because> {
    let mut answered = answered?;
    if let Some((_, because)) = answered
        .refused
        .iter()
        .find(|(id, _)| !taken_off.contains(id))
    {
        return Err(*because);
    }
    answered.at.retain(|id, _| !taken_off.contains(id));
    answered
        .streams
        .retain(|stream| !taken_off.contains(&ObjectId::new(stream.object, stream.generation)));
    let mut operators_written_out = false;
    for (class, holder) in &answered.content {
        let rewritten = holder.is_some_and(|id| {
            let streams = content_streams_of(document, id);
            let written_by = |at: &BTreeMap<ObjectId, Object>| {
                !streams.is_empty() && streams.iter().all(|stream| at.contains_key(stream))
            };
            if super::fenced::taken_with_the_photographs(document, id, photographed) {
                return true;
            }
            // An operator written out where the page draws the same (`doc/adr/3558`), at a
            // holder the proof's file needs no other repair at (`doc/adr/3733`); the output's own
            // check holds the write to having taken every one.
            if super::as_drawn::WRITTEN_OUT_WHERE_DRAWN_THE_SAME.contains(&class.name)
                && drawn.is_some_and(|drawn| {
                    written_by(&drawn.at)
                        && !drawn.left_repaired(id, class.name)
                        && streams
                            .iter()
                            .any(|stream| drawn.written_out.contains(stream))
                })
            {
                operators_written_out = true;
                return true;
            }
            // A number in exponential form, respelled as the value read, which no other row's
            // decision states (`doc/adr/3677`).
            if class.name == RESPELLED_BY_THE_RUN_REWRITE
                && drawn.is_some_and(|drawn| written_by(&drawn.at))
            {
                operators_written_out = true;
                return true;
            }
            // A `BDC` naming a property list nothing defines, where ADR 2684's rewrite defines
            // the name as a membership dictionary naming no group, which hides nothing, as the
            // reader drew it (`doc/adr/3631`).
            (class.name == DEFINED_BY_THE_RESOURCE_REWRITE && resources_defined)
                // A bare `/BM` name the standard does not define, restated as the `Normal` this
                // reader drew it as (`doc/adr/3643`).
                || (class.name == RESTATED_BY_THE_BLEND_MODE_REWRITE && blend_modes_restated)
                || (WRITTEN_OUT_BY_THE_RUN_REWRITE.contains(&class.name)
                    && drawn.is_some_and(|drawn| written_by(&drawn.at)))
                || (class.name == WRITTEN_OUT_AS_REALS
                    && reals.is_some_and(|reals| written_by(&reals.at)))
        });
        if !rewritten {
            return Err(Because::TheFence(in_content(class)));
        }
    }
    answered.operators_written_out = operators_written_out;
    Ok(answered)
}

/// Whether a rewrite other than a photograph writes `class` out of the streams it rewrites, so that
/// a page holding only such repairs is not photographed for section 5.1 (`doc/adr/3557`).
///
/// An operator [`super::as_drawn`] may write out is not among them: that write stands only where
/// the page draws the same, which is not known until it is tried, and a page it leaves is
/// photographed where the profile allows.
pub(super) fn written_out_by_a_rewrite(class: &str) -> bool {
    WRITTEN_OUT_BY_THE_RUN_REWRITE.contains(&class)
        || class == WRITTEN_OUT_AS_REALS
        || class == DEFINED_BY_THE_RESOURCE_REWRITE
        || class == RESTATED_BY_THE_BLEND_MODE_REWRITE
}

/// The class the blend-mode rows answer by restating a bare `/BM` name the standard does not
/// define as `Normal` (ISO 32000-2 §11.6.3, `doc/adr/3643`); the output's own check holds the
/// rewrite to having reached each.
const RESTATED_BY_THE_BLEND_MODE_REWRITE: &str = "model/blend-mode-unknown";

/// The class the empty-resource rewrite answers by defining the name a `BDC` gives its property
/// list (`doc/adr/2684`, `doc/adr/3631`); the output's own check holds it to having defined each.
const DEFINED_BY_THE_RESOURCE_REWRITE: &str = "model/property-list-unresolved";

/// Whether the input failed section 5.1 on an operator this reader repaired inside a content
/// stream, of a class `super::as_drawn` writes out where the page draws the same
/// (`doc/adr/3558`), or on a number written with an exponent, which its write respells
/// (`doc/adr/3677`), so that a profile granting `content-as-drawn` asks for that write.
pub(super) fn operators_repaired(document: &Document, input: &pdf_archive::Report) -> bool {
    input
        .failures()
        .any(|judgement| judgement.id == BASE_STANDARD)
        && document.repairs().iter().any(|logged| {
            matches!(logged.at, Site::Content(Some(_)))
                && (super::as_drawn::WRITTEN_OUT_WHERE_DRAWN_THE_SAME.contains(&logged.class.name)
                    || logged.class.name == RESPELLED_BY_THE_RUN_REWRITE)
        })
}

/// The classes `super::as_drawn` writes out of a stream it rewrites: a run it reads a number off
/// (ADR 2638), a lone delimiter it writes as the white space it draws as (ADR 2859), and a number
/// written with an exponent, respelled as the value it was read as (ADR 3677).
const WRITTEN_OUT_BY_THE_RUN_REWRITE: [&str; 3] = [
    "syntax/number-salvaged",
    "syntax/delimiter-unmatched",
    RESPELLED_BY_THE_RUN_REWRITE,
];

/// The class `super::as_drawn` answers wherever its write is made, not only where an operator the
/// base standard does not define asks for it: a number written with an exponent (ADR 3677).
const RESPELLED_BY_THE_RUN_REWRITE: &str = "syntax/number-with-an-exponent";

/// The class `super::integer_reals` writes out of a stream it rewrites: an integer too wide for 64
/// bits, written as the real of the same value (ADR 3533).
const WRITTEN_OUT_AS_REALS: &str = "syntax/integer-past-i64";

/// The content streams of the page or form `id`: a page's `/Contents`, or the form itself.
fn content_streams_of(document: &Document, id: ObjectId) -> Vec<ObjectId> {
    match document.get(id) {
        Object::Stream(_) => vec![id],
        Object::Dictionary(page) => super::hexadecimal::contents_of(document, &page),
        _ => Vec::new(),
    }
}

/// Every damaged stream the output carries, re-encoded from what this reader decoded.
///
/// A stream whose `/DecodeParms` does not pair with its `/Filter` is answered only where a
/// restatement re-encodes it anyway — this one, for its damage, or the ASCII row's — because the
/// re-encoding writes one `/DecodeParms` entry per stage it keeps; anywhere else it is refused by
/// name, since restating it would write this reader's pairing as the producer's.
fn restated(
    document: &Document,
    target: Target,
    (damaged, unpaired, undecodable): (bool, bool, bool),
) -> Result<Answered, Because> {
    let examination = pdf_archive::Examination::new(document, target);
    let reach = pdf_archive::Reach::of(&examination);
    let mut out = Answered::default();
    let (mut found_damage, mut found_unpaired) = (false, false);
    let programs = font_programs(examination.objects());
    let unlisted = carried_and_unlisted(document, examination.objects());
    let population = examination
        .objects()
        .iter()
        .map(|(id, object)| (*id, object, reach.reaches(*id)))
        .chain(unlisted.iter().map(|(id, object)| (*id, object, true)));
    for (id, object, carried) in population {
        let Object::Stream(stream) = object else {
            continue;
        };
        let damage = damaged.then(|| first_damage(document, stream)).flatten();
        let parameters_off = unpaired && super::rewrite::parameters_unpaired(document, stream);
        found_damage |= damage.is_some();
        found_unpaired |= parameters_off;
        // A stream nothing reaches is not written, so there is nothing of it to restate; a font
        // program's cut is the font routes' to answer — written whole, dropped for the face the
        // window drew, or substituted (ADR 3256) — and the output's own check holds them to it.
        if !carried || programs.contains(&id) || cut_ascii_data_alone(document, stream) {
            continue;
        }
        if undecodable && is_a_function(document, stream) && refused_as_corrupt(document, stream) {
            out.refused
                .push((id, Because::TheFence(UNDECODABLE_FUNCTION)));
            continue;
        }
        let Some((stage, damage)) = damage else {
            if parameters_off {
                out.refused
                    .push((id, Because::NotBuiltYet(PARAMETERS_UNPAIRED)));
            }
            continue;
        };
        let written = reencoded_through_flate(document, stream, &stream.dict, Cut::DamageAdmitted)
            .ok_or(Because::NotBuiltYet(NOT_REENCODED))?;
        out.streams.push(DamagedDataRestated {
            object: id.number,
            generation: id.generation,
            stage,
            damage: described(damage),
            encoded: stream.data.len(),
        });
        out.at.insert(id, written);
    }
    if unpaired && !found_unpaired {
        return Err(Because::NotBuiltYet(PARAMETERS_UNPAIRED));
    }
    if damaged && !found_damage {
        return Err(Because::NotBuiltYet(NOT_FOUND));
    }
    Ok(out)
}

/// The objects the output carries that no cross-reference section names, each with its value.
///
/// The population the requirements bind is the cross-reference table's, because ISO 19005-2
/// section 6.1.4 exempts an object no section names; but the whole-file rewrite copies everything
/// the catalog and `/Info` reach, and `pdf_syntax::Document::get` loads an object a section
/// declares and holds no entry for by its own `N G obj` header, so such an object is in the output
/// under a table that names it.
/// Stream data damaged there is damage the output's check finds, so it is restated with the
/// rest (ADR 3549): govdocs1 `027613.pdf`'s table names 260 objects of a 429-page file, and the
/// streams it cut short are page content streams it does not list.
fn carried_and_unlisted(
    document: &Document,
    listed: &[(ObjectId, Object)],
) -> Vec<(ObjectId, Object)> {
    let named: BTreeSet<u32> = listed.iter().map(|(id, _)| id.number).collect();
    let trailer = document.trailer();
    let mut queue: std::collections::VecDeque<ObjectId> = ["Root", "Info"]
        .into_iter()
        .filter_map(|key| trailer.get(key).and_then(Object::as_reference))
        .collect();
    let mut seen: BTreeSet<ObjectId> = queue.iter().copied().collect();
    let mut out = Vec::new();
    while let Some(id) = queue.pop_front() {
        let value = document.get(id);
        let mut pending = vec![&value];
        while let Some(object) = pending.pop() {
            match object {
                Object::Reference(next) => {
                    if seen.insert(*next) {
                        queue.push_back(*next);
                    }
                }
                Object::Array(items) => pending.extend(items),
                Object::Dictionary(entries) => pending.extend(entries.iter().map(|(_, v)| v)),
                Object::Stream(stream) => pending.extend(stream.dict.iter().map(|(_, v)| v)),
                _ => {}
            }
        }
        if !named.contains(&id.number) && value != Object::Null {
            out.push((id, value));
        }
    }
    out
}

/// Whether `stream` is a function dictionary's stream (ISO 32000-2 §7.10.1: a sampled function,
/// Type 0, or a PostScript calculator function, Type 4, is a stream).
fn is_a_function(document: &Document, stream: &Stream) -> bool {
    document
        .get_key(&stream.dict, "FunctionType")
        .as_integer()
        .is_some_and(|kind| kind == 0 || kind == 4)
}

/// Whether a stage of `stream`'s filter refuses its data as the file's departure from §7.4 —
/// the refusal `pdf_syntax` logs as `syntax/stream-data-undecodable` — rather than for a bound
/// this reader keeps or a filter it lacks.
fn refused_as_corrupt(document: &Document, stream: &Stream) -> bool {
    matches!(
        document.decoded_stream_data_reported(stream),
        Err(pdf_syntax::StreamRefusal::Filter {
            why: pdf_syntax::filter::FilterRefusal::Corrupt
                | pdf_syntax::filter::FilterRefusal::Parameter { .. },
            ..
        })
    )
}

/// Every stream a font descriptor names as its program (ISO 32000-2 §9.9's Table 124 keys).
fn font_programs(objects: &[(ObjectId, Object)]) -> BTreeSet<ObjectId> {
    objects
        .iter()
        .filter_map(|(_, object)| object.as_dict())
        .flat_map(|dict| {
            ["FontFile", "FontFile2", "FontFile3"]
                .into_iter()
                .filter_map(|key| match dict.get(key) {
                    Some(Object::Reference(id)) => Some(*id),
                    _ => None,
                })
        })
        .collect()
}

/// Whether the stream's one fault is ASCII data cut before its marker, which the row
/// `file-structure/ascii-data-ends-at-its-end-of-data-marker` restates at its own loss (ADR
/// 3473): asked as that row asks it, so the two never restate one stream twice.
fn cut_ascii_data_alone(document: &Document, stream: &Stream) -> bool {
    pdf_archive::table::ascii_stage_cut_short(document, stream).is_some()
        && reencoded_through_flate(document, stream, &stream.dict, Cut::AsciiAdmitted).is_some()
}

/// The first stage in front of the stream's image codec, if any, that reported damage, and the
/// damage: the stream `pdf_syntax` logged as `syntax/stream-data-damaged`, asked again of the one
/// stream (`pdf_syntax::Document::decoded_stream_data_reported` keeps the first damage in a chain
/// for the same reason, and a framing missing gives way to a later stage's damage).
fn first_damage(document: &Document, stream: &Stream) -> Option<(String, Damage)> {
    if document.is_external(stream) || stream.decryption_failed {
        return None;
    }
    let mut first = None;
    let walked =
        super::rewrite::each_stage_in_front_of_the_codec(document, stream, |filter, damage| {
            if let Some(damage) = damage
                && first
                    .as_ref()
                    .is_none_or(|(_, kept): &(String, Damage)| !kept.questions_the_bytes())
            {
                first = Some((String::from_utf8_lossy(filter).into_owned(), damage));
            }
        });
    // A stage that refuses after one that reported damage leaves that damage in the log; one that
    // refuses before any did is `refused_as_the_files`'s, not this.
    if walked.is_err() && first.is_none() {
        return None;
    }
    first
}

/// What a stage's damage was, worded for the report's sentence.
const fn described(damage: Damage) -> &'static str {
    match damage {
        Damage::Truncated => "running out before its end-of-data",
        Damage::Corrupt => "broken part way through",
        Damage::CheckValue => "decoding to bytes its check value disagrees with",
        Damage::ForeignCharacter => "holding a character outside its alphabet",
        Damage::Unframed => "missing its framing",
    }
}

/// The refusal naming a class of repair made inside a content stream.
///
/// One sentence per class the content reader's lexer records, so that the refusal names the class,
/// its clause and section 5.1 as a constant the report can carry; a class not listed here takes
/// [`IN_CONTENT`]. The test below holds each entry to `pdf_syntax`'s own name and clause.
fn in_content(class: &RepairClass) -> &'static str {
    IN_CONTENT_BY_CLASS
        .iter()
        .find(|(name, _, _)| *name == class.name)
        .map_or(IN_CONTENT, |(_, _, sentence)| *sentence)
}

/// Builds one [`IN_CONTENT_BY_CLASS`] entry from the class's name and clause.
macro_rules! in_content {
    ($name:literal, $clause:literal) => {
        (
            $name,
            $clause,
            concat!(
                "a content stream of this file needed the repair ",
                $name,
                " (",
                $clause,
                "), so it does not adhere to the base standard, which ISO 19005 section 5.1 \
                 requires; the stream crosses this verb byte for byte, and correcting it would \
                 edit the producer's page, where ADR 0816's fence stops this converter — \
                 doc/pdf-a-conversion-limits.md section 5.2"
            ),
        )
    };
}

/// Builds one [`IN_CONTENT_BY_CLASS`] entry for an operator class `super::as_drawn` writes out
/// where the page draws the same (`doc/adr/3558`), whose sentence says why it was not.
macro_rules! in_content_operator {
    ($name:literal, $clause:literal) => {
        (
            $name,
            $clause,
            concat!(
                "a content stream of this file needed the repair ",
                $name,
                " (",
                $clause,
                "), so it does not adhere to the base standard, which ISO 19005 section 5.1 \
                 requires; the operator is written out of the stream — or the operands left \
                 over in front of its own, where they are its only repair (ADR 3641) — only \
                 where the profile grants content-as-drawn and this reader draws the page the \
                 same without it (ADR 3558), and here one of those does not hold, so the stream crosses this \
                 verb byte for byte; a profile granting page-as-image photographs the page \
                 instead — doc/pdf-a-conversion-limits.md section 5.2"
            ),
        )
    };
}

/// Builds one [`IN_CONTENT_BY_CLASS`] entry for a class whose sentence states its own answer:
/// what `super::as_drawn` writes for it and when, or why no write states what this reader drew
/// (`doc/adr/3677` to `doc/adr/3680`).
macro_rules! in_content_because {
    ($name:literal, $clause:literal, $because:literal) => {
        (
            $name,
            $clause,
            concat!(
                "a content stream of this file needed the repair ",
                $name,
                " (",
                $clause,
                "), so it does not adhere to the base standard, which ISO 19005 section 5.1 \
                 requires; ",
                $because,
                ", so the stream crosses this verb byte for byte; a profile granting \
                 page-as-image photographs the page instead — doc/pdf-a-conversion-limits.md \
                 section 5.2"
            ),
        )
    };
}

/// The sentence for each class the content reader's lexer records, with the class's clause.
const IN_CONTENT_BY_CLASS: &[(&str, &str, &str)] = &[
    in_content_because!(
        "syntax/delimiter-unmatched",
        "ISO 32000-2 §7.2.3",
        "a lone delimiter is written as white space only where no operand stands before it and the \
         write then answers every operator the base standard does not define (ADR 2859); one \
         after an operand clears it when this reader meets it, and no write states that clearing \
         but the white space that would hand the operand on (ADR 3680)"
    ),
    in_content!("syntax/name-escape-incomplete", "ISO 32000-2 §7.3.5"),
    in_content_because!(
        "syntax/string-unterminated",
        "ISO 32000-2 §7.3.4.2",
        "this reader read the string to the end of the data, so every operator the producer \
         wrote after its opening parenthesis became bytes of one string; closing it anywhere \
         would choose where the producer meant it to end, and no write states what this reader \
         drew but the bytes as they stand (ADR 3680)"
    ),
    in_content!(
        "syntax/hexadecimal-string-unterminated",
        "ISO 32000-2 §7.3.4.3"
    ),
    in_content_because!(
        "syntax/hexadecimal-string-stray-byte",
        "ISO 32000-2 §7.3.4.3",
        "this reader skipped the byte and read the digits around it, and the string is written \
         without it only where the profile grants content-as-drawn and this reader draws the page \
         the same (ADR 3735), and here one of those does not hold"
    ),
    in_content!("syntax/integer-past-i64", "ISO 32000-2 §7.3.3"),
    in_content_because!(
        "syntax/number-with-an-exponent",
        "ISO 32000-2 §7.3.3",
        "the number is written as the decimal of the value this reader read it as only where the \
         profile grants content-as-drawn and the stream re-encodes (ADR 3677), and here one of \
         those does not hold"
    ),
    in_content_because!(
        "syntax/number-salvaged",
        "ISO 32000-2 §7.3.3",
        "the run is written as the number this reader drew off its front only where the profile \
         grants content-as-drawn, the base standard's operator row asks for the write and every \
         stream of the page re-encodes (ADR 2638), and here one of those does not hold"
    ),
    // What `pdf_model` logs reading a content stream's inline objects and its operators'
    // operands (ADR 3541).
    in_content_because!(
        "model/inline-object-malformed",
        "ISO 32000-2 §7.8.2",
        "this reader read the dictionary or array without the token that is no object, or \
         skipped an inline image whose dictionary did not read; what it went on to read after \
         that point is its recovery, not bytes a write can restate (ADR 3680)"
    ),
    in_content_because!(
        "model/inline-image-key-twice",
        "ISO 32000-2 §8.9.7",
        "the entry this reader did not read is written out only where the profile grants \
         content-as-drawn and this reader draws the page the same without it (ADR 3678), and \
         here one of those does not hold"
    ),
    in_content!(
        "model/inline-image-colour-space-unresolved",
        "ISO 32000-2 §8.9.7"
    ),
    in_content_because!(
        "model/inline-image-data-extent",
        "ISO 32000-2 §8.9.7",
        "this reader read past an /L that is no length, or found the image's end by searching \
         for EI where the dictionary's extent did not reach it; the bytes its decoding never read are cut only where they run past the end \
         that decoding states, /L names no other, the profile grants content-as-drawn and this \
         reader draws the page the same (ADR 3735), and here one of those does not hold"
    ),
    in_content_operator!("model/operand-of-the-wrong-type", "ISO 32000-2 §7.8.2"),
    in_content_operator!("model/operator-short-of-operands", "ISO 32000-2 §7.8.2"),
    in_content_operator!("model/restore-with-nothing-saved", "ISO 32000-2 §8.4.2"),
    // Answered where a text object is added around the operators, starting with the matrices this
    // reader drew them by (ADR 3661).
    (
        "model/text-operator-outside-a-text-object",
        "ISO 32000-2 §9.4.1",
        "a content stream of this file needed the repair model/text-operator-outside-a-text-object \
         (ISO 32000-2 §9.4.1), so it does not adhere to the base standard, which ISO 19005 section \
         5.1 requires; a text object is added around the operators this reader ran outside one, \
         starting with the text matrix it drew them by, only where the profile grants \
         content-as-drawn, the walk can state that matrix (ADR 3661), and this reader draws the \
         page the same with it (ADR 3558); here one of those does not hold, so the stream crosses \
         this verb byte for byte; a profile granting page-as-image photographs the page instead \
         — doc/pdf-a-conversion-limits.md section 5.2",
    ),
    // Answered where the operator is written as the device operator this reader drew it as
    // (ADR 3679); a form drawn in two colour spaces is judged in neither (ADR 3697, ADR 3802).
    in_content_because!(
        "model/colour-operands-do-not-fit-the-space",
        "ISO 32000-2 §8.6.8",
        "the operator is written as the device operator this reader drew it as, its operands kept          (ADR 3679), only where the profile grants content-as-drawn, the stream is drawn in one          colour space wherever it is drawn, and this reader draws the page the same; a form          painted under two spaces — one its operands fit and one they do not — has one stream for          both, which no write draws the same under each (ADR 3802), and here one of those does          not hold"
    ),
    // Answered where the `BDC` naming the list is written out with its section (ADR 3615).
    in_content_operator!("model/property-list-unresolved", "ISO 32000-2 §14.6.2"),
    // Answered where the operator is written out with every operand before it (ADR 3631).
    in_content_operator!("model/operands-left-over", "ISO 32000-2 §7.8.2"),
    // What a stream leaves open, which the checker's grammar reads off the tokens (ADR 3614); an
    // `EMC` closing nothing is written out alone (ADR 3631), and one is added where a section is
    // left open at its stream's end (ADR 3734), each where the page draws the same.
    in_content_because!(
        "model/marked-content-unbalanced",
        "ISO 32000-2 §14.6.1",
        "an EMC closing nothing is written out (ADR 3631), and a section left open where its \
         content stream ends is closed there by an EMC (ADR 3734), only where the profile grants \
         content-as-drawn, the section ends after a whole operator outside any text object, and \
         this reader draws the page the same; here one of those does not hold"
    ),
    in_content!(
        "model/compatibility-sections-unbalanced",
        "ISO 32000-2 §7.8.2"
    ),
];

/// The sentence for a class inside a content stream that [`IN_CONTENT_BY_CLASS`] does not list.
const IN_CONTENT: &str = "a content stream of this file needed a repair this reader logged \
     (the report's section 5.1 places name it), so it does not adhere to the base standard, which \
     ISO 19005 section 5.1 requires; the stream crosses this verb byte for byte, and correcting it \
     would edit the producer's page, where ADR 0816's fence stops this converter — \
     doc/pdf-a-conversion-limits.md section 5.2";

/// Why nothing is answered though the target binds section 5.1.
const NOTHING_FAILED: &str = "this reader repaired nothing in this document, so ISO 19005 \
     section 5.1 has nothing for the conversion to answer";

/// Why a chain whose parameters do not pair with its filters stops the conversion.
const PARAMETERS_UNPAIRED: &str = "a stream of this file states a /DecodeParms array that does \
     not pair with its /Filter entry for entry (syntax/decode-parms-mismatched, ISO 32000-2 \
     §7.3.8.2's Table 5), so it does not adhere to the base standard, which ISO 19005 section 5.1 \
     requires; which parameters belong to which filter is this reader's guess, and restating the \
     stream on that guess is not built (ADR 3521)";

/// Why a function whose data no stage decodes stops the conversion.
const UNDECODABLE_FUNCTION: &str = "a function stream of this file holds data its filter cannot \
     decode (syntax/stream-data-undecodable, ISO 32000-2 §7.4), which ISO 19005 section 5.1 does \
     not admit; the function is carried byte for byte on a page no photograph takes, no rewrite \
     of this converter replaces it, and writing it as anything else would state samples the \
     producer never wrote (ADR 3551)";

/// Why a damaged stream that cannot be re-encoded stops the conversion.
const NOT_REENCODED: &str = "a stream of this file decodes only in part \
     (syntax/stream-data-damaged, ISO 32000-2 §7.4), which ISO 19005 section 5.1 does not admit, \
     and it cannot be written as the bytes this reader decoded: its decode parameters name another \
     object, or a stage this reader does not decode stands in front of its image codec (ADR 3521)";

/// Why damage the log names in no stream object stops the conversion.
const NOT_FOUND: &str = "this reader decoded some stream data of this file only in part \
     (syntax/stream-data-damaged, ISO 32000-2 §7.4), which ISO 19005 section 5.1 does not admit, \
     and no stream object of the file decodes that way when asked again, so the damaged data is \
     somewhere this converter does not rewrite, such as an inline image inside a content stream, \
     which crosses this verb byte for byte (ADR 3521)";

#[cfg(test)]
mod tests {
    use super::*;

    /// zlib data of `0 0 m 10 10 l S` whose Adler-32 is wrong: decoded whole, damaged at its check.
    const CHECK_VALUE_WRONG: [u8; 20] = [
        120, 156, 51, 80, 48, 80, 200, 85, 48, 52, 0, 161, 28, 133, 96, 0, 23, 4, 3, 240,
    ];
    /// zlib framing over a block of a type RFC 1951 does not define: nothing decodes.
    const NO_BLOCK: [u8; 8] = [120, 156, 255, 255, 255, 255, 255, 255];

    /// A file of `objects`, numbered from one, whose cross-reference section declares them all and
    /// holds the entries of the first `listed` only, so the rest are declared and unread.
    fn file(objects: &[Vec<u8>], listed: usize) -> Document {
        let size = objects.len().saturating_add(1);
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
            bytes.extend_from_slice(body);
            bytes.extend_from_slice(b"\nendobj\n");
        }
        let start = bytes.len();
        bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in offsets.iter().take(listed) {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n")
                .as_bytes(),
        );
        Document::open(bytes).expect("the file opens")
    }

    fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
        let mut body = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(b"\nendstream");
        body
    }

    /// The damaged content stream of a page sits at an object the table omits, as `027613.pdf`'s
    /// do, and the rewrite copies it: it is found, and its damage with it (ADR 3549).
    #[test]
    fn a_stream_the_table_omits_and_the_rewrite_carries_is_searched() {
        let document = file(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>".to_vec(),
                stream("/Filter /FlateDecode", &CHECK_VALUE_WRONG),
            ],
            3,
        );
        let listed: Vec<(ObjectId, Object)> = document
            .xref()
            .object_numbers()
            .map(|number| {
                let id = document.defined_id(number);
                (id, document.get(id))
            })
            .collect();
        let unlisted = carried_and_unlisted(&document, &listed);
        let ids: Vec<ObjectId> = unlisted.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, [ObjectId::new(4, 0)]);
        let Object::Stream(content) = &unlisted[0].1 else {
            panic!("object 4 is the page's content stream");
        };
        assert!(first_damage(&document, content).is_some());
    }

    /// A function whose data no stage decodes is told from one that decodes and from a stream
    /// that is no function (ADR 3551).
    #[test]
    fn an_undecodable_function_is_told_apart() {
        let document = file(&[b"<< /Type /Catalog >>".to_vec()], 1);
        let read = |body: Vec<u8>| {
            let mut parser = pdf_syntax::Parser::new(&body);
            match parser.parse_object().expect("a stream") {
                Object::Stream(stream) => stream,
                other => panic!("not a stream: {other:?}"),
            }
        };
        let function = "/FunctionType 0 /Domain [0 1] /Range [0 1] /Size [2] /BitsPerSample 8 \
             /Filter /FlateDecode";
        let broken = read(stream(function, &NO_BLOCK));
        let sound = read(stream(function, &CHECK_VALUE_WRONG));
        let content = read(stream("/Filter /FlateDecode", &NO_BLOCK));
        assert!(is_a_function(&document, &broken) && refused_as_corrupt(&document, &broken));
        assert!(!refused_as_corrupt(&document, &sound));
        assert!(!is_a_function(&document, &content));
    }

    /// Each sentence names a class `pdf_syntax` or `pdf_model` declares, under that class's own
    /// clause.
    #[test]
    fn every_content_sentence_names_a_declared_class_under_its_clause() {
        for (name, clause, sentence) in IN_CONTENT_BY_CLASS {
            let declared = repair::CLASSES
                .iter()
                .chain(pdf_model::repair::CLASSES)
                .find(|class| class.name == *name)
                .unwrap_or_else(|| panic!("{name} is a class a reader crate declares"));
            assert_eq!(declared.clause, *clause, "{name}");
            assert!(sentence.contains(name) && sentence.contains("section 5.1"));
        }
    }

    #[test]
    fn a_class_not_listed_is_refused_by_the_general_sentence() {
        assert_eq!(in_content(&repair::DUPLICATE_KEY), IN_CONTENT);
        assert!(in_content(&repair::STRING_UNTERMINATED).contains("syntax/string-unterminated"));
    }
}
