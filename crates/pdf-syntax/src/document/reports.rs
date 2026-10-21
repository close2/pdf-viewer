//! What a recovery found, reported: the objects a repair moved, a stream keyword that was padded,
//! the objects a damaged object stream did not give up, and the dictionaries that stop part-way.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::{Document, read, write};
use crate::object::{Object, ObjectId};
use crate::parser::Parser;

/// What object streams this document could only partly decode did not yield, as it accumulates.
#[derive(Default)]
pub(super) struct LostToDamage {
    /// Numbers the header named and whose object the prefix does not wholly carry.
    pub(super) objects: BTreeSet<u32>,
    /// Objects Table 16's `/N` counts whose header pair the prefix never reached.
    pub(super) unnamed: usize,
    /// The object streams this happened in.
    pub(super) streams: BTreeSet<u32>,
}

/// What [`Document::objects_lost_to_damage`] answers: the objects §7.5.7 storage did not give up.
///
/// Three numbers rather than one because they are three different statements about a file: which
/// objects were named and not read, how many were not even named, and which streams they were in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectsLost {
    /// The object numbers an object stream's header named and whose bytes it does not carry.
    pub objects: Vec<u32>,
    /// How many further objects `/N` counts whose header pair the decoded prefix never reached.
    pub unnamed: usize,
    /// The object streams those losses happened in.
    pub streams: Vec<u32>,
}

impl ObjectsLost {
    /// Whether anything was lost at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty() && self.unnamed == 0
    }

    /// How many objects are missing in total, named and unnamed.
    #[must_use]
    pub fn count(&self) -> usize {
        self.objects.len().saturating_add(self.unnamed)
    }
}

/// What entering §7.5.7's compressed objects into a rebuilt table recovered, and what it did not.
///
/// Every field is zero for a document whose cross-reference information came from the file, and
/// [`Self::is_empty`] says which case a caller is looking at. The fields beside `objects` are
/// there for one reason, and it is the reason this is a struct rather than a count: **a rebuild
/// that recovers some of a file's objects must not look like one that recovered all of them.**
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompressedRecovery {
    /// Object streams the header scan found at the outermost level of the file.
    pub streams: usize,
    /// Those whose own header this reader could read, so that what they hold has a location.
    pub read: usize,
    /// Object numbers entered into the table from those headers, and so reachable again.
    ///
    /// A number here has a *location*, which is what a cross-reference entry is. Whether the
    /// bytes at it are whole is a second question, asked when something reads the object and
    /// answered by [`Document::objects_lost_to_damage`] — the same account for a stream reached
    /// through a rebuilt table as through one the file supplied.
    pub objects: usize,
    /// Streams whose header this reader could not read: a filter chain it has no decoder for, a
    /// decode that stopped before the pairs, or a header naming nothing.
    ///
    /// Every object such a stream holds is unreachable and unnamed — not even its number
    /// survives — which is what makes this a count of streams rather than of objects.
    pub unreadable: usize,
    /// Streams left unexpanded because [`super::RECOVERY_DECODE_BUDGET`] was spent on the earlier ones.
    pub beyond_the_budget: usize,
    /// Numbers an object stream names that the scan had already found at the outermost level.
    ///
    /// Declined rather than entered, on §7.5.7's rule for a freed number's reuse — see
    /// [`crate::XrefTable::object_streams`]. Counted because that rule is a *reading*, and a
    /// file that exercises it should be visible to whoever next questions it.
    pub already_at_top_level: usize,
}

impl CompressedRecovery {
    /// Whether the recovery ran at all: false for every table the file's own bytes supplied.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.streams == 0
    }

    /// Whether it entered everything the file's object streams offered.
    ///
    /// The question a caller wanting one sentence asks: a rebuild that lost nothing says the
    /// same thing as a rebuild that met no object stream, and both differ from one that met
    /// eight and could read three.
    #[must_use]
    pub const fn is_whole(&self) -> bool {
        self.unreadable == 0 && self.beyond_the_budget == 0
    }
}

impl Document {
    /// Returns `true` if the cross-reference table had to be rebuilt by scanning.
    #[must_use]
    pub fn was_recovered(&self) -> bool {
        self.xref.recovered_by_scan()
    }

    /// The object numbers this reader found by their own header rather than through the table.
    ///
    /// Empty for a well-formed document. **Two conditions reach it**, and both are the table
    /// failing to describe an object the file states twice (§7.5.4's entry and §7.3.10's
    /// header): a number filed at an offset where a *different* object's header stands, and a
    /// number the table's own subsection header declared while the entry itself was never read
    /// — see `Document::load_by_header` for why the header wins in each. It is here so that
    /// the repair is answerable rather than silent: a caller that wants to tell a person their
    /// file is damaged can, and `pdf_syntax` still does not decide whether that is worth saying.
    ///
    /// Grows as objects are loaded, because nothing is parsed until it is asked for. Ask
    /// after the pages that matter have been read.
    #[must_use]
    pub fn misfiled_objects(&self) -> Vec<u32> {
        read(&self.misfiled).iter().copied().collect()
    }

    /// The streams whose `stream` keyword this reader found followed by spaces or tabs before
    /// its end-of-line marker, by object number, beside how many it skipped.
    ///
    /// ISO 32000-2 §7.3.8.1 has the keyword "followed by an end-ofline marker", and a space is
    /// not one, so a document this answers non-empty for is malformed. The bytes are skipped
    /// rather than handed to the stream's filter as data — the clause places the data after the
    /// marker, so they are not data on any reading of the file — and they are reported here so
    /// that the repair is answerable rather than silent, as [`Self::misfiled_objects`] is. A
    /// stream whose `/Length` fits only with the padding counted as data is read that way and is
    /// not in this list: the file's own statement of its extent wins. ADR 1365.
    ///
    /// Grows as objects are loaded, because nothing is parsed until it is asked for.
    #[must_use]
    pub fn padded_stream_keywords(&self) -> Vec<(u32, usize)> {
        read(&self.padded)
            .iter()
            .map(|(&number, &bytes)| (number, bytes))
            .collect()
    }

    /// What an object stream this reader could only partly decode did not yield.
    ///
    /// Empty for a well-formed document, and for a damaged one it grows as objects are loaded —
    /// nothing is expanded until something asks for it, so ask after the pages that matter have
    /// been read. [`ObjectsLost::objects`] are numbers the stream's header named and whose bytes
    /// the prefix does not wholly carry; [`ObjectsLost::unnamed`] are the ones Table 16's `/N`
    /// counts and whose header pair the prefix did not reach, so not even their numbers survive.
    ///
    /// It is a statement about the *file* rather than about any page, which is why it is here
    /// beside [`Self::was_recovered`] and not in a page's report: §7.5.7 puts objects in a
    /// stream, and the objects a page then fails to find are wherever the file put them.
    #[must_use]
    pub fn objects_lost_to_damage(&self) -> ObjectsLost {
        let held = read(&self.lost_to_damage);
        ObjectsLost {
            objects: held.objects.iter().copied().collect(),
            unnamed: held.unnamed,
            streams: held.streams.iter().copied().collect(),
        }
    }

    /// What a rebuilt table recovered from §7.5.7's object streams, and what it did not.
    ///
    /// [`CompressedRecovery::is_empty`] for every document whose cross-reference information came
    /// from the file itself, which is the overwhelming majority: this is a statement about the
    /// *recovery*, not about the file's use of object streams. Where it is not empty, the fields
    /// that are not `objects` are what keep a partial recovery from reading like a whole one —
    /// see `Self::recover_compressed_objects`.
    ///
    /// Unlike [`Self::objects_lost_to_damage`] this is complete the moment the document opens,
    /// because the recovery is the one place in this reader that expands an object stream nothing
    /// has asked for yet.
    #[must_use]
    pub const fn compressed_objects_recovered(&self) -> &CompressedRecovery {
        &self.recovered_compressed
    }

    /// How many cross-reference entries this file's streams state and do not carry, §7.5.8.
    ///
    /// [`crate::XrefTable::entries_lost`] for this document's table, and zero for all but a
    /// damaged or inconsistent one.
    #[must_use]
    pub fn cross_reference_entries_lost(&self) -> u64 {
        self.xref.entries_lost()
    }

    /// The entries of `id`'s dictionary that this file states readably, where [`Self::get`]
    /// answers nothing because the object as a whole will not parse.
    ///
    /// **This does not change what [`Self::get`] answers, now or ever.** It is a second question
    /// a caller asks *instead*, having already been refused, and it is the only route in this
    /// crate to [`Parser::parse_damaged_dictionary`] — which carries the reading of §7.3.7 that
    /// says what such a prefix is and what it is not. A caller takes one on the strength of the
    /// entries' own content, or on a reference to it out of an object that parses whole; ADR 0784
    /// is the argument. **There are two consumers, and each was decided by its own clause.**
    /// `pdf_model::Pages`' recovery takes one where the entries state Table 31's `/Type /Page`
    /// themselves or where §7.7.3.2's `/Kids` names the object (ADRs 0784, 0786), and
    /// `pdf_model::type3::Type3Font` takes one named by a Type 3 font dictionary's `/CharProcs`,
    /// because §9.6.4's step b) states that a name a `/CharProcs` has no key for paints nothing —
    /// so that subset substitutes no mark (ADR 0866). ADR 0867 is why no other key comes through.
    ///
    /// `None` for an object that parses, for one this file does not place at a byte offset, and
    /// for one stored inside §7.5.7's object stream — the last because a compressed object has no
    /// header of its own in the file, so there is no `obj` to read the entries after, and a
    /// stream that decoded only in part is ADR 0366's refusal rather than this one.
    #[must_use]
    pub fn damaged_dictionary(&self, id: ObjectId) -> Option<crate::parser::DamagedDictionary> {
        let damaged = self.damaged_dictionaries().get(&id.number)?.clone();
        let Object::Dictionary(entries) =
            self.decrypt_object(damaged.id, Object::Dictionary(damaged.entries))
        else {
            return None;
        };
        Some(crate::parser::DamagedDictionary { entries, ..damaged })
    }

    /// Every object number this file states a damaged dictionary for, scanned once and
    /// remembered.
    ///
    /// **Why this is a scan of its own and not `Self::object_headers`.** That scan keeps an
    /// offset only where `Parser::parse_indirect_object` succeeded, so an object whose
    /// dictionary stops part-way is in neither the table it builds nor the table
    /// `xref::rebuild` builds from it — a rebuilt file's damaged page object is not merely
    /// unparsed but *unnamed*, which is why `Pages`' recovery could not even ask about it.
    ///
    /// **This displaces nothing, because it decides nothing.** It is a statement about the
    /// file's bytes — *here is a header whose dictionary stops part-way* — and it is answered
    /// for a number whether or not something readable also bears it. Which of the two a caller
    /// takes is the caller's: [`Self::get`] is asked first everywhere, and only an object it
    /// answers nothing for is looked up here. Among two damaged headers bearing one number the
    /// later in the file wins, which is `scan_for_objects`' own rule for two readable ones.
    ///
    /// Nothing here parses stream data: a damaged dictionary never reaches its `stream`
    /// keyword, and a whole one is abandoned at the `>>` this call is not interested in.
    #[must_use]
    pub fn damaged_dictionaries(&self) -> Arc<BTreeMap<u32, crate::parser::DamagedDictionary>> {
        if let Some(found) = read(&self.damaged).as_ref() {
            return Arc::clone(found);
        }
        let mut found: BTreeMap<u32, crate::parser::DamagedDictionary> = BTreeMap::new();
        // A look at every header reads every byte, so a file on disk is asked for whole here
        // and a file the process cannot hold whole yields nothing, by name (ADR 0809).
        if let Some(input) = self.whole_for_a_scan() {
            for offset in crate::xref::object_header_offsets(input) {
                let mut parser = Parser::at(input, offset, self.limits);
                if let Some(damaged) = parser.parse_damaged_dictionary() {
                    found.insert(damaged.id.number, damaged);
                }
            }
        }
        let found = Arc::new(found);
        *write(&self.damaged) = Some(Arc::clone(&found));
        found
    }
}
