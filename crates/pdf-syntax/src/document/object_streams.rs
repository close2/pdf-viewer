//! §7.5.7: the objects stored inside an object stream, and their entry into a rebuilt table.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::{CompressedRecovery, Document, RECOVERY_DECODE_BUDGET, read, write};
use crate::object::Object;
use crate::parser::Parser;
use crate::xref::Location;

/// ISO 32000-2 §7.5.7's header: the `(object number, relative offset)` pairs an object stream
/// begins with.
///
/// > N pairs of integers separated by white-space, where the first integer in each pair shall
/// > represent the object number of a compressed object and the second integer shall represent
/// > the byte offset in the decoded stream of that object, relative to the first object stored in
/// > the object stream, the offset for which is the value of the stream's First entry.
///
/// Bounded by `count`, which is Table 16's `/N`, and **not** by `/First`: that entry is "\[t]he
/// byte offset in the decoded stream of the first compressed object", so a producer that leaves
/// white-space between the last pair and the first object gives a prefix whose tail is the
/// object's own bytes, and reading on takes integers out of them. Stops early where the decoded
/// prefix runs out mid-header, which is what a damaged stream leaves — the pairs it did carry are
/// whole, and how many it did not is `/N` minus what came back.
fn object_stream_pairs(data: &[u8], first: usize, count: i64) -> Vec<(u32, usize)> {
    let mut header = crate::lexer::Lexer::new(data.get(..first).unwrap_or_default());
    let mut pairs = Vec::new();
    for _ in 0..count.max(0) {
        let (Some(crate::Token::Integer(number)), Some(crate::Token::Integer(at))) =
            (header.next_token(), header.next_token())
        else {
            break;
        };
        if let (Ok(number), Ok(at)) = (u32::try_from(number), usize::try_from(at)) {
            pairs.push((number, at));
        }
    }
    pairs
}

impl Document {
    /// Expands an object stream into its contained objects.
    pub(super) fn expand_object_stream(&self, number: u32) -> Option<Arc<BTreeMap<u32, Object>>> {
        if let Some(cached) = read(&self.expanded_streams).get(&number) {
            return Some(Arc::clone(cached));
        }

        let Location::Offset(offset) = self.xref.location(number)? else {
            // An object stream inside another object stream is not permitted, and
            // following it would be a route to unbounded recursion.
            return None;
        };

        let (_, object) = self.parse_at(offset)?;
        let stream = object.as_stream()?;
        let decoded = self.decoded_stream_data_reported(stream).ok()?;
        let data = decoded.data;

        let count = self.get_key(&stream.dict, "N").as_integer().unwrap_or(0);
        let first = self
            .get_key(&stream.dict, "First")
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())?;
        let pairs = object_stream_pairs(&data, first, count);

        // §7.5.7 states where each compressed object ends as well as where it begins — "[t]he
        // byte offsets shall be in increasing order", and NOTE 7 (2020): "processing of each
        // object in an object stream starts at the specified byte offset in the decompressed
        // stream and ends prior to the byte offset of the next object or when the end of stream
        // is encountered". So the *last* object's extent is stated by the end of the stream,
        // which a damaged decode has not reached, and every other object's by the next offset.
        let mut ends: Vec<usize> = pairs.iter().map(|&(_, relative)| relative).collect();
        ends.sort_unstable();

        let pair_count = pairs.len();
        let mut objects = BTreeMap::new();
        let mut lost = BTreeSet::new();
        for (object_number, relative) in pairs {
            let start = first.saturating_add(relative);
            // The end this file states for this object: the next offset in increasing order, or
            // — for the last one — the end of the stream, which is only known where the decode
            // reached it. ADR 0366.
            let stated_end = ends
                .iter()
                .find(|&&other| other > relative)
                .map(|&next| first.saturating_add(next))
                .or(if decoded.damage.is_some() {
                    None
                } else {
                    Some(data.len())
                });
            // A prefix of an object stream is a smaller *collection* of whole objects, and an
            // object the prefix does not wholly carry is not one of them: a truncated token
            // parses — `/Length 12345` cut short is `/Length 123` — so reading it would put a
            // value the producer never wrote under a number the producer did. ADR 0366.
            if stated_end.is_none_or(|end| end > data.len()) || start >= data.len() {
                lost.insert(object_number);
                continue;
            }
            let mut parser = Parser::at(&data, start, self.limits);
            if let Ok(parsed) = parser.parse_object() {
                objects.insert(object_number, parsed);
            }
        }
        // The pairs a truncated header never carried are lost too, and `/N` is what says how
        // many there should have been (Table 16, "the number of indirect objects stored in the
        // stream"). Their object numbers are unknowable, so only the count is.
        let short_by = usize::try_from(count.max(0))
            .unwrap_or(0)
            .saturating_sub(pair_count);
        if !lost.is_empty() || short_by > 0 {
            let mut record = write(&self.lost_to_damage);
            record.objects.extend(lost);
            record.unnamed = record.unnamed.saturating_add(short_by);
            record.streams.insert(number);
        }

        let expanded = Arc::new(objects);
        write(&self.expanded_streams).insert(number, Arc::clone(&expanded));
        Some(expanded)
    }

    /// The object numbers one object stream's header names, without parsing any of them.
    ///
    /// What a cross-reference entry needs is the *number*; the object itself is what
    /// [`Self::expand_object_stream`] reads when something asks for it. Reading them here would
    /// be the eager work `CLAUDE.md`'s startup rule forbids, and the difference is measured
    /// rather than assumed — see [`Self::recover_compressed_objects`].
    ///
    /// Returns the numbers in the order the header names them and how many decoded bytes were
    /// read to find that out.
    fn object_stream_members(&self, number: u32) -> Option<(Vec<u32>, usize)> {
        if let Some(cached) = read(&self.expanded_streams).get(&number) {
            return Some((cached.keys().copied().collect(), 0));
        }

        let Location::Offset(offset) = self.xref.location(number)? else {
            // As in `expand_object_stream`: an object stream inside another one is not
            // permitted, and following it would be a route to unbounded recursion.
            return None;
        };
        let (_, object) = self.parse_at(offset)?;
        let stream = object.as_stream()?;
        let decoded = self.decoded_stream_data_reported(stream).ok()?;

        let count = self.get_key(&stream.dict, "N").as_integer().unwrap_or(0);
        let first = self
            .get_key(&stream.dict, "First")
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())?;
        let numbers = object_stream_pairs(&decoded.data, first, count)
            .into_iter()
            .map(|(number, _)| number)
            .collect();
        Some((numbers, decoded.data.len()))
    }

    /// Enters §7.5.7's compressed objects into a table that was rebuilt by scanning.
    ///
    /// # Why a rebuilt table needs this and a read one does not
    ///
    /// [`crate::xref::rebuild`] finds objects by their `N G obj` headers, which §C.4 licenses:
    ///
    /// > When a PDF processor reads a PDF file with a damaged or missing cross-reference table,
    /// > it may attempt to rebuild the table by scanning all the objects in the file.
    ///
    /// **and an object inside an object stream has no header to scan for.** §7.5.7's first
    /// sentence is why:
    ///
    /// > An object stream is a stream object in which a sequence of indirect objects may be
    /// > stored, as an alternative to their being stored at the outermost PDF file level.
    ///
    /// so a scan that stops at that level has found some of the objects in the file rather than
    /// all of them, and every object a modern producer packed is missing from the recovery. The
    /// step that finds them is stated by the same clause rather than guessed at:
    ///
    /// > N pairs of integers separated by white-space, where the first integer in each pair shall
    /// > represent the object number of a compressed object and the second integer shall
    /// > represent the byte offset in the decoded stream of that object
    ///
    /// The scan can always reach the streams themselves, because §7.5.7 forbids storing a stream
    /// object inside one — so each is written at the outermost level with a header of its own.
    ///
    /// # Why it is here rather than in `xref`
    ///
    /// Reading those pairs means decoding the stream, which means §7.4's filter chain and §7.6's
    /// decryption, and `xref` deliberately has neither: it builds the table that resolving an
    /// indirect `/Filter` would need. So the rebuild grows a second phase that runs *after* the
    /// table exists and after [`Self::authenticate`] — an object stream in an encrypted file is
    /// ciphertext until there is a key.
    ///
    /// # What it will not do
    ///
    /// **It does not read the objects.** Only each stream's header is read, which is what a
    /// cross-reference entry is made of; every object stays unparsed until something asks for it,
    /// exactly as it does under a table the file supplied. That is `CLAUDE.md`'s startup rule,
    /// and the difference is measured rather than assumed: on the widest rebuilt document on this
    /// disk — 10 MB, 316 object streams, 142 641 compressed objects — `Document::open` is
    /// **13.0–17.5 ms** without this recovery and **52.3–75.6 ms** with it, five samples apiece
    /// in one sitting through `pdf-model/examples/open_cost`. Expanding every stream's objects
    /// instead of its header measured **196.5 ms** on the same file, one sample, which is what
    /// this shape costs a broken document and what the lazy one gives back. What the objects
    /// themselves cost is paid by the page that wants them.
    ///
    /// **An entry the scan already made wins**, and [`CompressedRecovery::already_at_top_level`]
    /// counts what that declined; [`crate::XrefTable::object_streams`] has the clause.
    ///
    /// **A number whose object turns out to be unreadable is still entered**, because the header
    /// naming it is the file's own statement about where it is, and what a damaged stream then
    /// fails to yield is [`Self::objects_lost_to_damage`]'s account — ADR 0366's rule, reached
    /// through this one rather than restated inside it. The two compose in the same order they
    /// would for a whole table: the entry says where the object lives, the expansion says whether
    /// the bytes are there.
    pub(super) fn recover_compressed_objects(&mut self) {
        if !self.xref.recovered_by_scan() {
            return;
        }
        let streams = self.xref.object_streams().to_vec();
        let mut report = CompressedRecovery {
            streams: streams.len(),
            ..CompressedRecovery::default()
        };
        // Earliest stream first, so that a later one's copy of an object number replaces it here
        // before anything is entered — the rule `scan_for_objects` applies to two headers bearing
        // one number, and §7.5.6's "most recent copy" behind it.
        let mut members: BTreeMap<u32, Location> = BTreeMap::new();
        let mut spent = 0_usize;
        for stream in streams {
            if spent >= RECOVERY_DECODE_BUDGET {
                report.beyond_the_budget = report.beyond_the_budget.saturating_add(1);
                continue;
            }
            let Some((named, decoded)) = self.object_stream_members(stream) else {
                report.unreadable = report.unreadable.saturating_add(1);
                continue;
            };
            spent = spent.saturating_add(decoded);
            if named.is_empty() {
                report.unreadable = report.unreadable.saturating_add(1);
                continue;
            }
            report.read = report.read.saturating_add(1);
            for (index, number) in named.iter().enumerate() {
                members.insert(
                    *number,
                    Location::InStream {
                        stream,
                        index: u32::try_from(index).unwrap_or(u32::MAX),
                    },
                );
            }
        }

        let offered = members.len();
        report.objects = self.xref.enter_compressed(members);
        report.already_at_top_level = offered.saturating_sub(report.objects);
        self.recovered_compressed = report;
    }
}
