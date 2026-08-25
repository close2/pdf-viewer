//! Cross-reference resolution, and recovery when it is broken.
//!
//! # Why recovery is mandatory, not a nicety
//!
//! The cross-reference table maps object numbers to byte offsets, and it is the single
//! most frequently corrupted structure in real PDFs: files are truncated mid-write,
//! concatenated, edited by tools that do not update offsets, or served through
//! transformations that alter line endings. A reader that trusts the table and gives up
//! when it is wrong fails on documents every other viewer opens.
//!
//! So there are two paths. The table is used when it works, and the whole file is scanned
//! for `N G obj` headers when it does not. **Both are bounded**: scanning is linear in
//! file size and the offset table cannot exceed the object count, because unbounded
//! recovery is a denial of service dressed up as robustness.
//!
//! **The scan is half of the second path and not all of it**, because §7.5.7 lets a file store
//! its objects inside a stream instead of at the outermost level, and those have no header to
//! scan for. What this module can do about them is name the object streams it found
//! ([`XrefTable::object_streams`]); entering their contents needs the filter chain and the
//! decryption a table being built cannot have, so it is [`crate::Document`]'s second phase.

use std::collections::BTreeMap;

use crate::error::{SyntaxError, SyntaxResult};
use crate::object::{Dictionary, Object, ObjectId};
use crate::parser::{Limits, Parser};

/// Where an object's bytes are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    /// At a byte offset in the file.
    Offset(usize),
    /// Inside an object stream: which stream, and the index within it.
    ///
    /// PDF 1.5 allows objects to be packed into a compressed stream, so this is not an
    /// exotic case — most modern files use it for everything but the document catalogue.
    InStream {
        /// Object number of the containing object stream.
        stream: u32,
        /// Index of this object within that stream.
        index: u32,
    },
}

/// The cross-reference table: object numbers to locations, plus the trailer.
///
/// An entry may be `None`, which is a *deletion* rather than an absence: §7.5.4's `f` entries
/// and §7.5.8.3's type 0 both say the object number names nothing, and recording that is what
/// makes §7.5.6's "most recent copy" rule apply to a removal as well as to a replacement.
#[derive(Debug, Clone, Default)]
pub struct XrefTable {
    entries: BTreeMap<u32, Option<Location>>,
    trailer: Dictionary,
    /// How the table was obtained, for diagnostics and for the comparison report.
    recovered_by_scan: bool,
    /// Entries a cross-reference *stream* stated and did not carry. See [`Self::entries_lost`].
    entries_lost: u64,
    /// Object streams a scan found, in the order their headers stand in the file.
    ///
    /// Only [`scan_for_objects`] fills this, and it is what [`crate::Document`] reads to enter
    /// §7.5.7's compressed objects into a rebuilt table. A table read from the file itself needs
    /// nothing of the kind: it already says which objects are in which stream.
    object_streams: Vec<u32>,
}

impl XrefTable {
    /// Returns the location of an object, if known.
    #[must_use]
    pub fn location(&self, number: u32) -> Option<Location> {
        self.entries.get(&number).copied().flatten()
    }

    /// Returns the trailer dictionary.
    #[must_use]
    pub fn trailer(&self) -> &Dictionary {
        &self.trailer
    }

    /// How many entries this file's cross-reference streams state and do not carry.
    ///
    /// Zero for every file whose streams are as long as their own `/Index` and `/W` say — which
    /// is every well-formed one, because §7.3.8.2 requires the constraints to be consistent. A
    /// non-zero count is the number of object numbers about which the file *intended* to say
    /// something and this reader read nothing: the records present are whole and are the
    /// producer's own, and the ones past the end of the data are absent rather than deleted.
    /// **That distinction is the reason this is counted at all**: everywhere else in this reader
    /// a number with no entry has been deleted (§7.5.6, ADR 0100), and here it has not.
    #[must_use]
    pub fn entries_lost(&self) -> u64 {
        self.entries_lost
    }

    /// Returns the number of known objects.
    ///
    /// A number the newest section deleted is not one of them.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.is_some())
            .count()
    }

    /// Returns `true` if no objects are known.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.values().all(Option::is_none)
    }

    /// Returns every known object number, ascending.
    ///
    /// A deleted number names nothing, so it is not yielded: every caller of this is walking
    /// the file's objects to ask each one a question.
    pub fn object_numbers(&self) -> impl Iterator<Item = u32> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.is_some())
            .map(|(number, _)| *number)
    }

    /// The object streams a scan found, earliest in the file first.
    ///
    /// Empty for a table the file's own cross-references produced, and for a scan of a file that
    /// packs nothing. §7.5.7 is what makes a scan able to find these at all: it forbids a stream
    /// object from being stored in an object stream, so every object stream is written at the
    /// outermost level with an `N G obj` header of its own.
    ///
    /// The order is the file's, because that is the only evidence a rebuilt table has about which
    /// of two statements is the more recent (§7.5.6) — the same rule [`scan_for_objects`] applies
    /// to two headers bearing one object number.
    #[must_use]
    pub fn object_streams(&self) -> &[u32] {
        &self.object_streams
    }

    /// Files §7.5.7's compressed objects into a rebuilt table, and says how many it filed.
    ///
    /// **Only where the number names nothing yet.** The scan's own entry is an object with a
    /// header, written at the outermost level, and §7.5.7 says what that means when an object
    /// stream also claims the number:
    ///
    /// > If either an object stream or a compressed object is deleted and the object number is
    /// > freed, that object number shall be reused only for an ordinary (uncompressed) object
    /// > other than an object stream.
    ///
    /// so a number in both places is a number the file freed and reused, and the ordinary object
    /// is the live one. The alternative reading — that an object stream later in the file is a
    /// newer revision of the same number — is what [`Document::recover_compressed_objects`] holds
    /// the count of, so that a file exercising it is visible rather than assumed absent.
    ///
    /// [`Document::recover_compressed_objects`]: crate::Document::recover_compressed_objects
    pub(crate) fn enter_compressed(
        &mut self,
        entries: impl IntoIterator<Item = (u32, Location)>,
    ) -> usize {
        let mut entered = 0_usize;
        for (number, location) in entries {
            if self.entries.contains_key(&number) {
                continue;
            }
            self.entries.insert(number, Some(location));
            entered = entered.saturating_add(1);
        }
        entered
    }

    /// Returns `true` if this table was rebuilt by scanning rather than read from the file.
    ///
    /// Worth surfacing: a scanned table means the file's own cross-references were unusable,
    /// which is a fact about the document a validator should report even when rendering
    /// succeeds.
    #[must_use]
    pub fn recovered_by_scan(&self) -> bool {
        self.recovered_by_scan
    }

    /// Files every section's entries at once, the first writer of each number winning.
    ///
    /// **The read order carries §7.5.6's rule and this preserves it.** Sections are read newest
    /// first, so for one object number the entry that appears earliest in `entries` is the most
    /// recent copy — which is what "the most recent copy of each object shall be the one accessed
    /// from the PDF file" means for a reader. A stable sort keeps that order among equal numbers
    /// and `dedup_by_key` keeps the first of each run, so the rule is stated once for a whole
    /// file rather than once per entry. A `None` is a *free* entry and is kept rather than
    /// dropped, which is the point rather than an artefact: an update that **deletes** an object
    /// writes a free entry over an in-use one, so a table that dropped free entries would let the
    /// older section's offset win and resurrect the object.
    ///
    /// **Measured, and it is why this is not an `insert` per entry.** ISO 32000-2 has 101 318 of
    /// them, and one searched insert apiece was **40% of `Document::open`** — 523 M of 1307 M
    /// instructions, in `btree/search.rs` and `cmp.rs`. The inputs are already ascending runs (a
    /// section's entries are, and this file has two sections), which is the case Rust's stable
    /// sort merges in one pass, and `BTreeMap`'s `FromIterator` bulk-builds from sorted input
    /// rather than searching for each key. **130.7 M instructions per open before, 76.6 M
    /// after — 41%.** ADR 0180, `cargo run -p pdf-syntax --example callgrind_open`.
    fn fill(&mut self, mut entries: Vec<(u32, Option<Location>)>) {
        entries.sort_by_key(|(number, _)| *number);
        entries.dedup_by_key(|(number, _)| *number);
        self.entries = entries.into_iter().collect();
    }

    /// Merges trailer keys that are not already present.
    fn merge_trailer(&mut self, dict: &Dictionary) {
        for (key, value) in dict.iter() {
            if self.trailer.get_by_name(key).is_none() {
                self.trailer.insert(key.clone(), value.clone());
            }
        }
    }
}

/// How many bytes at the end of the file are searched for `startxref` first.
///
/// §7.5.5 puts it two lines from the end, so this window holds every conforming file's and
/// costs one read of two kilobytes. It is a *first* look rather than a bound: where the end of
/// the file does not carry the keyword, [`find_startxref`] reads further back, because the
/// premise this comment used to state — that a `startxref` further back "is not a trailer, it
/// is a coincidence" — is disproved by a file with a whole second copy of itself appended
/// (ADR 0379).
const STARTXREF_SEARCH_WINDOW: usize = 2048;

/// The most cross-reference sections that will be followed through `/Prev`.
///
/// A file with a genuine chain this long does not exist; one that claims to has a cycle or
/// is trying to make the reader loop. The cycle is also detected directly, so this is a
/// second line of defence rather than the only one.
const MAX_XREF_SECTIONS: usize = 1024;

/// How far into the file a header is looked for.
///
/// §7.5.2 puts it on the first line; files served through mail gateways acquire junk in front of
/// it, and NOTE 1 licenses that — "[t]his provision allows for arbitrary bytes preceding the
/// %PDF- without impacting the viability of the PDF file and its byte offsets".
pub(crate) const HEADER_SEARCH_WINDOW: usize = 1024;

/// Reads the cross-reference information for a file, recovering if necessary.
///
/// # Errors
///
/// [`SyntaxError::NoHeader`] if the file has neither header *and* no objects could be
/// found either, and [`SyntaxError::NoCrossReferences`] if neither the table nor a scan
/// yields any object.
pub fn read(input: &[u8], limits: Limits) -> SyntaxResult<XrefTable> {
    // The header may be preceded by junk — files served through mail gateways acquire it —
    // so the specification's "first line" is relaxed to "somewhere near the start".
    let header_window = input.len().min(HEADER_SEARCH_WINDOW);
    let start = input.get(..header_window).unwrap_or_default();
    // §12.7.8.2.2 gives an FDF file a header of its own — `%FDF-1.n` where a PDF writes
    // `%PDF-n.m` — and §12.7.8.2.1 makes the rest of the file structure clause 7's, §7.5.2's
    // offset rule below included. So the *second* marker is searched for only where the first
    // is absent: a PDF whose first kilobyte happens to contain `%FDF-` is still measured from
    // its own header, and an FDF file stops being measured from byte zero by accident.
    let header_at = position_of(start, *b"%PDF-").or_else(|| position_of(start, *b"%FDF-"));

    if let Some(table) = read_from_startxref(input, header_at.unwrap_or(0), limits)
        && !table.is_empty()
    {
        return Ok(table);
    }

    // The table was absent, unreadable, or empty. Scan.
    rebuild(input, limits, header_at.is_some())
}

/// Reconstructs a cross-reference table by scanning the file for objects.
///
/// The other half of [`read`], and a public one since the hundred-and-seventh session because
/// there is a second reason to reach it: a cross-reference table that *parses* is not thereby a
/// table that *works*. §7.5.5 makes the trailer's `/Root` "the catalog dictionary for the PDF
/// document", so a table leading to no catalog has been disproved by the file itself, whatever it
/// parsed as — and [`crate::Document::open`] rebuilds and tries again rather than refusing. Two
/// corpus documents are exactly that: a table whose offsets a hand edit moved, complete and
/// self-consistent and pointing at the wrong bytes.
///
/// `had_header` decides only which error is reported when nothing is found.
///
/// **What it returns is the outermost level of the file and no more.** §7.5.7's compressed
/// objects have no header to scan for, and the table this hands back therefore names the object
/// streams themselves and not one object inside them; [`crate::Document::open`] finishes the job
/// with the pairs each stream's own header states. A caller reaching this directly gets the scan
/// alone, which is what its two callers here want.
///
/// # Errors
///
/// [`SyntaxError::NoHeader`] where the file has neither header and no objects could be found,
/// and [`SyntaxError::NoCrossReferences`] where it had one and they still could not.
pub fn rebuild(input: &[u8], limits: Limits, had_header: bool) -> SyntaxResult<XrefTable> {
    // A missing header does not stop this. Files lose their header to transfer damage and
    // to producers that never wrote one, and the objects after it are usually intact — so
    // refusing on the header alone rejects documents both poppler and mupdf recover and
    // read. The header is a diagnosis, not a gate: it only decides which error is reported
    // when nothing is found.
    let mut table = scan_for_objects(input, limits);
    table.recovered_by_scan = true;

    if table.is_empty() {
        if !had_header {
            return Err(SyntaxError::NoHeader {
                searched: input.len().min(HEADER_SEARCH_WINDOW),
            });
        }
        return Err(SyntaxError::NoCrossReferences {
            detail: "the cross-reference table was unusable and no object headers were found"
                .to_owned(),
        });
    }

    // A scanned table has no trailer, so find one. Without `/Root` the document cannot be
    // navigated, and the scan is the last chance to locate it.
    if table.trailer.get("Root").is_none()
        && let Some(dict) = find_trailer_by_scan(input, limits)
    {
        table.merge_trailer(&dict);
    }
    if table.trailer.get("Root").is_none()
        && let Some(id) = find_catalog_by_scan(input, limits, &table)
    {
        table.trailer.insert(
            crate::object::Name::new(b"Root".to_vec()),
            Object::Reference(id),
        );
    }

    Ok(table)
}

/// Where `marker` first appears in `window`, or `None`.
fn position_of(window: &[u8], marker: [u8; 5]) -> Option<usize> {
    window.windows(marker.len()).position(|at| at == marker)
}

/// Follows `startxref` and the `/Prev` chain.
///
/// `base` is where ISO 32000-2 §7.5.2 says every offset in the file is measured from:
///
/// > byte offsets shall be calculated from the PERCENT SIGN (25h)
///
/// > NOTE 1 This provision allows for arbitrary bytes preceding the %PDF- without impacting
/// > the viability of the PDF file and its byte offsets.
///
/// So a file with junk in front of its header has a *correct* cross-reference table whose
/// numbers are all short by the junk's length, and adding the header's position back is the
/// whole of the rule. Without it such a file falls through to `scan_for_objects`, which
/// recovers it — differently, and by reading the entire file rather than four numbers. One
/// corpus document has junk before its header, and no valid file can be harmed by this,
/// because a file whose header is at zero adds zero.
///
/// Returns `None` when the chain cannot be read at all, leaving the caller to scan.
fn read_from_startxref(input: &[u8], base: usize, limits: Limits) -> Option<XrefTable> {
    let mut next = find_startxref(input)?.saturating_add(base);
    let mut table = XrefTable::default();
    let mut visited = std::collections::BTreeSet::new();
    // Every section's entries in read order — newest first, and a hybrid file's `/XRefStm`
    // between its own section and the older one it precedes. `XrefTable::fill` turns that order
    // into §7.5.6's precedence in one pass at the end.
    let mut entries: Vec<(u32, Option<Location>)> = Vec::new();

    for _ in 0..MAX_XREF_SECTIONS {
        // A cycle in the `/Prev` chain would otherwise loop until the section cap, doing
        // real work each time.
        if !visited.insert(next) {
            break;
        }
        if next >= input.len() {
            break;
        }

        let Some(section) = read_section(input, next, base, limits) else {
            break;
        };
        entries.extend(section.entries);
        table.entries_lost = table.entries_lost.saturating_add(section.lost);
        table.merge_trailer(&section.trailer);

        // A cross-reference stream may also carry `/XRefStm`, a hybrid-reference file's
        // pointer to a stream holding entries the table omits. Read it before `/Prev` so
        // its entries take precedence over older sections.
        if let Some(hybrid) = section
            .trailer
            .get("XRefStm")
            .and_then(Object::as_integer)
            .and_then(|value| usize::try_from(value).ok())
            && let Some(extra) = read_section(input, hybrid.saturating_add(base), base, limits)
        {
            entries.extend(extra.entries);
            table.entries_lost = table.entries_lost.saturating_add(extra.lost);
        }

        match section
            .trailer
            .get("Prev")
            .and_then(Object::as_integer)
            .and_then(|value| usize::try_from(value).ok())
        {
            Some(previous) => next = previous.saturating_add(base),
            None => break,
        }
    }

    table.fill(entries);
    Some(table)
}

/// One cross-reference section's entries and trailer.
struct Section {
    entries: Vec<(u32, Option<Location>)>,
    trailer: Dictionary,
    /// Entries §7.5.8's own arithmetic states that the stream's data does not carry.
    ///
    /// Zero for a classic table, whose short subsection is a different shape and is handled
    /// where it is read. See [`read_xref_stream`] for what states the number.
    lost: u64,
}

/// Reads either a classic `xref` table or a cross-reference stream at `offset`.
fn read_section(input: &[u8], offset: usize, base: usize, limits: Limits) -> Option<Section> {
    let mut parser = Parser::at(input, offset, limits);
    let probe = parser.position();

    // A classic table begins with the `xref` keyword.
    let mut lexer = crate::lexer::Lexer::at(input, probe);
    if lexer.next_token() == Some(crate::Token::Keyword(b"xref")) {
        return read_classic_table(input, lexer.position(), base, limits);
    }

    // Otherwise expect `N G obj` introducing a cross-reference stream.
    let (_, object) = parser.parse_indirect_object().ok()?;
    let stream = object.as_stream()?.clone();
    read_xref_stream(&stream, base, limits)
}

/// Reads a classic `xref` table, positioned just after the keyword.
fn read_classic_table(input: &[u8], offset: usize, base: usize, limits: Limits) -> Option<Section> {
    let mut lexer = crate::lexer::Lexer::at(input, offset);
    let mut entries = Vec::new();

    loop {
        let rewind = lexer.position();
        match lexer.next_token() {
            // A subsection header: first object number and count.
            Some(crate::Token::Integer(first)) => {
                let Some(crate::Token::Integer(count)) = lexer.next_token() else {
                    break;
                };
                let first = u32::try_from(first).ok()?;
                let count = u32::try_from(count).unwrap_or(0);
                let mut subsection = Vec::new();

                for index in 0..count {
                    // Each entry is `oooooooooo ggggg n` — but the fixed 20-byte layout is
                    // widely got wrong, so the entries are tokenised rather than sliced.
                    //
                    // A subsection whose declared count overruns the entries actually
                    // present runs straight into whatever follows them, which is normally
                    // the `trailer` keyword. So each abandoned entry rewinds first: reading
                    // on from *after* the offending token steps over that keyword, and the
                    // section then has no trailer, no `/Root`, and no pages — from a
                    // document whose every object is intact. `outline_goto_action.pdf`
                    // declares twelve entries and supplies eleven.
                    let entry_at = lexer.position();
                    let Some(crate::Token::Integer(position)) = lexer.next_token() else {
                        entries.extend(realigned(input, subsection));
                        return Some(finish(entries, input, entry_at, limits));
                    };
                    let Some(crate::Token::Integer(_generation)) = lexer.next_token() else {
                        entries.extend(realigned(input, subsection));
                        return Some(finish(entries, input, entry_at, limits));
                    };
                    let Some(crate::Token::Keyword(kind)) = lexer.next_token() else {
                        entries.extend(realigned(input, subsection));
                        return Some(finish(entries, input, entry_at, limits));
                    };

                    // `f` marks a free entry: the object number names nothing *in this
                    // section*, and it is recorded as such so that an older section cannot
                    // fill it back in (§7.5.6). The free list's own chain — the next free
                    // object's number, which is what `position` holds here — is a writer's
                    // structure and nothing a reader resolves consults it.
                    let Ok(number) =
                        u32::try_from(u64::from(first).saturating_add(u64::from(index)))
                    else {
                        continue;
                    };
                    if kind == b"n" {
                        if let Ok(position) = usize::try_from(position) {
                            subsection.push((
                                number,
                                Some(Location::Offset(position.saturating_add(base))),
                            ));
                        }
                    } else if kind == b"f" {
                        subsection.push((number, None));
                    }
                }
                entries.extend(realigned(input, subsection));
            }
            Some(crate::Token::Keyword(word)) if word == b"trailer" => {
                let mut parser = Parser::at(input, lexer.position(), limits);
                let trailer = parser
                    .parse_object()
                    .ok()
                    .and_then(|object| object.as_dict().cloned())
                    .unwrap_or_default();
                return Some(Section {
                    entries,
                    trailer,
                    lost: 0,
                });
            }
            _ => {
                lexer.seek(rewind);
                break;
            }
        }
    }

    Some(finish(entries, input, lexer.position(), limits))
}

/// How many of a subsection's in-use entries are checked against the objects they point at.
///
/// The check costs one `N G obj` header read apiece and runs once per subsection, so it is a
/// handful of reads for a whole file: a table has one subsection, an incremental update adds
/// one per revision, and a cross-reference *stream* has none of this at all. Four rather than
/// one because a single witness cannot tell a displaced subsection from a single wrong offset,
/// and rather than all of them because 101 318 header reads on the specification's own PDF is
/// exactly the eager work `CLAUDE.md`'s startup rule forbids.
const SUBSECTION_WITNESSES: usize = 4;

/// Corrects a subsection whose entries are all filed under the wrong object number.
///
/// # The two statements this reconciles
///
/// §7.5.4 says a subsection's header gives "the object number of the first object in this
/// subsection", and that each in-use entry's offset gives "the number of bytes from the
/// beginning of the PDF file to the beginning of the object". §7.3.10 makes the object at
/// that offset begin by naming itself:
///
/// > The definition of an indirect object in a PDF file shall consist of its object number and
/// > generation number (separated by white-space), followed by the value of the object bracketed
/// > between the keywords obj and endobj
///
/// So a file states which object an entry describes **twice**, and where the two disagree by
/// the *same* amount for every entry checked, it is the subsection header that is wrong: the
/// entries are a contiguous run — §7.5.4 requires that — and a run displaced by one is still
/// a run. The object headers win, because each is written next to the bytes it describes.
///
/// # Why a whole subsection rather than an object at a time
///
/// `issue7229.pdf` is the witness and it shows why a per-object repair is not enough. Its
/// first section declares `1 7` and then lists seven entries beginning with object 0's free
/// entry — §7.5.4's "[t]he first entry in the table (object number 0) shall always be free
/// and shall have a generation number of 65,535" — so every entry is filed one number too
/// high. Repairing only the in-use entries leaves that free entry standing as object 1, and
/// object 1 is the page's image: the page then draws *nothing*, which is worse than the wrong
/// page it drew before. A displacement is a property of the subsection, so the subsection is
/// what gets corrected, free entries included.
///
/// # What it will not do
///
/// Nothing, unless at least two in-use entries agree on the same non-zero displacement. One
/// witness cannot distinguish a misdeclared subsection from a single stale offset, and
/// shifting a whole subsection on that evidence would turn one broken object into all of
/// them.
fn realigned(
    input: &[u8],
    subsection: Vec<(u32, Option<Location>)>,
) -> Vec<(u32, Option<Location>)> {
    let witnesses: Vec<Option<i64>> = subsection
        .iter()
        .filter_map(|(number, location)| match location {
            Some(Location::Offset(offset)) => Some((*number, *offset)),
            _ => None,
        })
        .take(SUBSECTION_WITNESSES)
        .map(|(number, offset)| {
            header_number_at(input, offset)
                .map(|actual| i64::from(number).saturating_sub(i64::from(actual)))
        })
        .collect();

    if witnesses.len() < 2 {
        return subsection;
    }
    let Some(Some(displacement)) = witnesses.first().copied() else {
        return subsection;
    };
    if displacement == 0 || !witnesses.iter().all(|found| *found == Some(displacement)) {
        return subsection;
    }

    subsection
        .into_iter()
        .filter_map(|(number, location)| {
            let corrected = i64::from(number).checked_sub(displacement)?;
            u32::try_from(corrected)
                .ok()
                .map(|number| (number, location))
        })
        .collect()
}

/// The object number an indirect object at `offset` gives for itself, per §7.3.10.
///
/// Reads the header alone — two integers and `obj` — rather than parsing the object, because
/// the object may be an 800 KB image and the question is only what it calls itself.
fn header_number_at(input: &[u8], offset: usize) -> Option<u32> {
    let mut lexer = crate::lexer::Lexer::at(input, offset);
    let Some(crate::Token::Integer(number)) = lexer.next_token() else {
        return None;
    };
    let Some(crate::Token::Integer(_generation)) = lexer.next_token() else {
        return None;
    };
    let Some(crate::Token::Keyword(keyword)) = lexer.next_token() else {
        return None;
    };
    if keyword != b"obj" {
        return None;
    }
    u32::try_from(number).ok()
}

/// Builds a section, looking for a trailer from `offset` onwards.
fn finish(
    entries: Vec<(u32, Option<Location>)>,
    input: &[u8],
    offset: usize,
    limits: Limits,
) -> Section {
    let trailer = find_trailer_from(input, offset, limits).unwrap_or_default();
    Section {
        entries,
        trailer,
        lost: 0,
    }
}

/// Reads a cross-reference stream: PDF 1.5's replacement for the classic table.
///
/// Entries are binary fields whose widths `/W` gives.
///
/// The data is decoded here rather than by the caller, because it cannot be decoded
/// anywhere else: resolving an indirect `/Filter` would need the cross-reference table
/// this function is building. The specification requires a cross-reference stream's own
/// entries to be direct for exactly that reason, so reading them straight from the
/// dictionary is correct as well as necessary.
///
/// These streams are almost always `FlateDecode` with `/Predictor 12`, so this path is the
/// normal one for any PDF written since 1.5 — not an edge case.
fn read_xref_stream(
    stream: &crate::object::Stream,
    base: usize,
    limits: Limits,
) -> Option<Section> {
    let data = decode_direct(stream, limits)?;

    let widths: Vec<usize> = stream
        .dict
        .get("W")?
        .as_array()?
        .iter()
        .map(|entry| {
            entry
                .as_integer()
                .and_then(|v| usize::try_from(v).ok())
                .unwrap_or(0)
        })
        .collect();
    if widths.len() < 3 {
        return None;
    }

    let size = stream
        .dict
        .get("Size")
        .and_then(Object::as_integer)
        .unwrap_or(0);
    let size = u32::try_from(size).unwrap_or(0);

    // `/Index` gives (first, count) pairs; its absence means one subsection from zero.
    let index: Vec<i64> = stream
        .dict
        .get("Index")
        .and_then(Object::as_array)
        .map_or_else(
            || vec![0, i64::from(size)],
            |items| items.iter().filter_map(Object::as_integer).collect(),
        );

    let row = widths.iter().copied().sum::<usize>();
    if row == 0 {
        return None;
    }
    // `/W` describes the stream, not the record, so it is resolved into byte ranges once here
    // rather than once per entry. `RecordLayout` says what that was costing.
    let layout = RecordLayout::of(&widths);

    // One allocation rather than the seventeen a doubling `Vec` takes to reach 101 318 entries.
    // The capacity comes from the *decoded data's* own length rather than from `/Size` or
    // `/Index`, because the data has been materialised already and its length is therefore a
    // bound a malformed file cannot inflate. Worth 0.4% of `Document::open` on ISO 32000-2 —
    // small, and kept because a length that is known is a length worth stating.
    let mut entries = Vec::with_capacity(data.len().checked_div(row).unwrap_or(0));
    let mut cursor = 0usize;
    // §7.5.8's own arithmetic for how long this stream is, which Table 17 states of `/W`: "[t]he
    // sum of the items shall be the total length of each entry; it can be used with the Index
    // array to determine the starting position of each subsection." That plus §7.3.8.2's rule
    // for every stream "from whose attributes a length can be inferred" — "[a]ll of these
    // constraints shall be consistent" — makes a short cross-reference stream checkable without
    // knowing whether a filter failed or a producer simply wrote too little.
    let stated: u64 = index
        .chunks(2)
        .filter_map(|pair| pair.get(1).copied())
        .map(|count| u64::try_from(count.max(0)).unwrap_or(0))
        .sum();
    let carried = u64::try_from(data.len().checked_div(row).unwrap_or(0)).unwrap_or(u64::MAX);
    let lost = stated.saturating_sub(carried);

    for pair in index.chunks(2) {
        let (Some(&first), Some(&count)) = (pair.first(), pair.get(1)) else {
            break;
        };
        let Ok(first) = u32::try_from(first) else {
            continue;
        };

        for offset in 0..count.max(0) {
            // A record the data does not wholly carry is not read: each field's width comes
            // from `/W` and its object number from `/Index` and its position, so a *whole*
            // record is the producer's own statement about one object, and a partial one is
            // fields read from bytes that are not there.
            let Some(record) = data.get(cursor..cursor.saturating_add(row)) else {
                return Some(Section {
                    entries,
                    trailer: stream.dict.clone(),
                    lost,
                });
            };
            cursor = cursor.saturating_add(row);

            let Ok(number) =
                u32::try_from(u64::from(first).saturating_add(u64::try_from(offset).unwrap_or(0)))
            else {
                continue;
            };
            entries.push((number, entry_location(record, &layout, base)?.location()));
        }
    }

    Some(Section {
        entries,
        trailer: stream.dict.clone(),
        lost,
    })
}

/// Where each of Table 18's three fields sits inside one record.
///
/// `/W` is a property of the *stream*: §7.5.8.2's Table 17 says of it
///
/// > The sum of the items shall be the total length of each entry; it can be used with the Index
/// > array to determine the starting position of each subsection.
///
/// so every record in a stream has the same three fields at the same three offsets, and turning
/// `/W` into those offsets is work that belongs once per stream. It was being done once per
/// *entry* — a `take(3)` over `/W`, a running offset and a bounds check apiece — which on ISO
/// 32000-2 is 101 318 times and made [`entry_location`] a quarter of `Document::open` (ADR 0677).
#[derive(Debug)]
struct RecordLayout {
    /// The half-open byte range each field occupies, in Table 18's order.
    ///
    /// `None` is Table 17's absent field — "[a] value of zero for an element in the W array
    /// indicates that the corresponding field shall not be present in the stream" — which takes
    /// its default and reads no bytes.
    fields: [Option<(usize, usize)>; 3],
}

impl RecordLayout {
    /// Resolves the first three elements of `/W`.
    ///
    /// The caller has already refused a `/W` of fewer than three elements, so a shorter slice
    /// here would leave later fields absent rather than misplaced.
    fn of(widths: &[usize]) -> Self {
        let mut fields = [None; 3];
        let mut at = 0usize;
        for (field, &width) in fields.iter_mut().zip(widths) {
            if width == 0 {
                continue;
            }
            // Saturating, so that a `/W` no address space can hold yields a range past the end
            // of every record and is refused where the record is read, rather than here: an
            // `/Index` that declares no entries reads no record and is not refused at all.
            let end = at.saturating_add(width);
            *field = Some((at, end));
            at = end;
        }
        Self { fields }
    }
}

/// How many bytes of a Table 18 field fit a `u64`.
const FIELD_BYTES: usize = size_of::<u64>();

/// The unsigned integer a Table 18 field's bytes state, most significant byte first.
///
/// §7.5.8.2 states no maximum for an element of `/W`, so a malformed file may declare a field
/// wider than any integer this reader holds. Such a field **clamps** rather than wrapping, and
/// that is the load-bearing half: `u64::MAX` is an offset past the end of every file and an
/// object number no `u32` holds, so a record stating one names nothing that can be reached —
/// where the low 64 bits of the same number would be a plausible offset into a file the record
/// does not describe.
fn big_endian(bytes: &[u8]) -> u64 {
    if bytes.len() > FIELD_BYTES {
        return bytes.iter().fold(0u64, |value, &byte| {
            value.saturating_mul(256).saturating_add(u64::from(byte))
        });
    }
    // Eight bytes or fewer cannot overflow a `u64`, so no clamp is reachable and the shift is
    // exact. Table 17's own widths are one to four bytes, so this is the whole function for
    // every file anyone writes. `wrapping_shl` rather than `<<` because a shift is defined only
    // below the type's width and this says so, where the operator would rely on a lint exception
    // to say nothing.
    bytes
        .iter()
        .fold(0u64, |value, &byte| value.wrapping_shl(8) | u64::from(byte))
}

/// Reads one Table 18 record.
///
/// `None` is malformation — a record shorter than `/W` says it is — and is distinct from the
/// record saying the number names nothing, which is `Entry::Deleted`. §7.5.8.2's Table 17 gives
/// the defaults: "[a] value of zero for an element in the W array indicates that the
/// corresponding field shall not be present in the stream, and the default value shall be used,
/// if there is one", and of the first field specifically, "[i]f the first element is zero, the
/// type field shall not be present, and shall default to Type 1".
fn entry_location(record: &[u8], layout: &RecordLayout, base: usize) -> Option<Entry> {
    let mut fields = [1u64, 0, 0];
    for (slot, field) in fields.iter_mut().zip(&layout.fields) {
        let Some(&(at, end)) = field.as_ref() else {
            continue;
        };
        *slot = big_endian(record.get(at..end)?);
    }

    Some(match fields[0] {
        // Type 1: an object at a byte offset.
        1 => usize::try_from(fields[1]).map_or(Entry::Deleted, |position| {
            Entry::At(Location::Offset(position.saturating_add(base)))
        }),
        // Type 2: an object inside an object stream.
        2 => match (u32::try_from(fields[1]), u32::try_from(fields[2])) {
            (Ok(stream), Ok(index)) => Entry::At(Location::InStream { stream, index }),
            _ => Entry::Deleted,
        },
        // Type 0 is Table 18's free object, and of every other value §7.5.8.3 says "[a]ny other
        // value shall be interpreted as a reference to the null object, thus permitting new entry
        // types to be defined in the future". Both are recorded rather than skipped: an entry
        // that resolves to null is a statement this section makes, and skipping it would let an
        // older section answer instead.
        _ => Entry::Deleted,
    })
}

/// What one cross-reference entry says about an object number.
///
/// Written out rather than as an `Option<Location>` because the distinction the reader needs is
/// between "this section has no opinion" — which is a number absent from the section altogether —
/// and "this section says nothing is there", and only the second is an `Entry`.
enum Entry {
    /// The object is at this location.
    At(Location),
    /// The number names nothing: §7.5.4's `f`, Table 18's type 0, or a type this version of PDF
    /// has no meaning for.
    Deleted,
}

impl Entry {
    /// The location this entry names, if it names one.
    fn location(self) -> Option<Location> {
        match self {
            Self::At(location) => Some(location),
            Self::Deleted => None,
        }
    }
}

/// Decodes a stream using only direct values from its own dictionary.
///
/// Used for cross-reference streams, where indirect values cannot be resolved because the
/// table being built is what resolving needs.
fn decode_direct(stream: &crate::object::Stream, limits: Limits) -> Option<std::sync::Arc<[u8]>> {
    let filters: Vec<Vec<u8>> = match stream.dict.get("Filter") {
        None => Vec::new(),
        Some(Object::Name(name)) => vec![name.as_bytes().to_vec()],
        Some(Object::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
            .collect(),
        // An indirect `/Filter` in a cross-reference stream is malformed for the reason
        // above; there is no safe way to proceed.
        Some(_) => return None,
    };

    if filters.is_empty() {
        return Some(std::sync::Arc::clone(&stream.data));
    }

    // `/DecodeParms` may be a single dictionary or one per filter.
    let parms_for = |index: usize| -> Option<Dictionary> {
        match stream.dict.get("DecodeParms") {
            Some(Object::Dictionary(dict)) => Some(dict.clone()),
            Some(Object::Array(items)) => items.get(index).and_then(Object::as_dict).cloned(),
            _ => None,
        }
    };

    let mut data = std::sync::Arc::clone(&stream.data);
    for (index, filter) in filters.iter().enumerate() {
        data = crate::filter::decode_with_parms(filter, &data, parms_for(index).as_ref(), limits)?;
    }
    Some(data)
}

/// Finds the offset given by the last `startxref` in the file.
///
/// §7.5.5 states where it is and how a reader gets to it:
///
/// > PDF processors should read a PDF file from its end. The last line of the file shall
/// > contain only the end-of-file marker, %%EOF. The two preceding lines shall contain, one per
/// > line and in order, the keyword startxref and the byte offset in the decoded stream from the
/// > beginning of the PDF file to the beginning of the xref keyword in the last cross-reference
/// > section.
///
/// So the last two kilobytes are looked at first and answer for every file that obeys that
/// sentence. **Where they do not, the rest of the file is searched backwards rather than the
/// file's cross-reference information being abandoned**, and the order matters more than the
/// window: the alternative is [`rebuild`], which disregards what the file says about where its
/// objects are and scans for them instead. §C.4 licenses that only for the case this is not —
/// "[w]hen a PDF processor reads a PDF file with a damaged or missing cross-reference table, it
/// may attempt to rebuild the table by scanning all the objects in the file". A trailer eight
/// megabytes from the end is neither damaged nor missing; it is merely not last.
///
/// The witness is `jhove-errors/PDF-HUL-138/6.2017-0960.pdf`, which is one complete document
/// with a *truncated copy of itself* appended: reading from the end found nothing, the scan
/// took the truncated copy's objects, and a document three other readers display came back with
/// no first page at all (ADR 0379).
///
/// The cost is one backwards byte scan, paid only by a file that has already failed the window
/// — and it is paid *instead of* a scan that reads every object header in the file, not on top
/// of one, whenever it succeeds.
fn find_startxref(input: &[u8]) -> Option<usize> {
    let tail_start = input.len().saturating_sub(STARTXREF_SEARCH_WINDOW);
    last_startxref_from(input, tail_start).or_else(|| last_startxref_from(input, 0))
}

/// The offset stated by the last `startxref` at or after `from`, if that keyword is there and
/// an integer follows it.
fn last_startxref_from(input: &[u8], from: usize) -> Option<usize> {
    let region = input.get(from..)?;
    let found = region
        .windows(b"startxref".len())
        .rposition(|candidate| candidate == b"startxref")?;

    let after = from
        .saturating_add(found)
        .saturating_add(b"startxref".len());
    let mut lexer = crate::lexer::Lexer::at(input, after);
    match lexer.next_token() {
        Some(crate::Token::Integer(offset)) => usize::try_from(offset).ok(),
        _ => None,
    }
}

/// Finds a `trailer` keyword at or after `offset` and parses its dictionary.
fn find_trailer_from(input: &[u8], offset: usize, limits: Limits) -> Option<Dictionary> {
    let haystack = input.get(offset..)?;
    let found = haystack
        .windows(b"trailer".len())
        .position(|window| window == b"trailer")?;
    let at = offset
        .saturating_add(found)
        .saturating_add(b"trailer".len());
    Parser::at(input, at, limits)
        .parse_object()
        .ok()
        .and_then(|object| object.as_dict().cloned())
}

/// Finds the last `trailer` dictionary anywhere in the file.
///
/// Used only after a scan, where the layered structure is already lost. The last one is
/// taken because in an incrementally-updated file it is the newest.
fn find_trailer_by_scan(input: &[u8], limits: Limits) -> Option<Dictionary> {
    let found = input
        .windows(b"trailer".len())
        .rposition(|window| window == b"trailer")?;
    Parser::at(input, found.saturating_add(b"trailer".len()), limits)
        .parse_object()
        .ok()
        .and_then(|object| object.as_dict().cloned())
}

/// Finds an object whose dictionary says `/Type /Catalog`.
///
/// The last resort when a scanned file has no usable trailer: without `/Root` the document
/// cannot be navigated at all, and a catalogue is self-identifying.
fn find_catalog_by_scan(input: &[u8], limits: Limits, table: &XrefTable) -> Option<ObjectId> {
    for number in table.object_numbers() {
        let Some(Location::Offset(offset)) = table.location(number) else {
            continue;
        };
        let mut parser = Parser::at(input, offset, limits);
        let Ok((id, object)) = parser.parse_indirect_object() else {
            continue;
        };
        let is_catalog = object
            .as_dict()
            .and_then(|dict| dict.get("Type"))
            .and_then(Object::as_name)
            .is_some_and(|name| name == &"Catalog");
        if is_catalog {
            return Some(id);
        }
    }
    None
}

/// Rebuilds a cross-reference table by scanning for `N G obj` headers.
///
/// Linear in file size, and later definitions of an object number overwrite earlier ones —
/// the opposite of the layered table's rule, and correct here for the same reason: in an
/// incrementally-updated file the later copy is the newer one.
///
/// Public since the hundred-and-seventy-seventh session, for the second caller described in
/// [`rebuild`]: [`crate::Document`] asks for it when *one* entry of an otherwise working table
/// is disproved by the object at its offset, which is a repair of one number rather than of a
/// file. It carries no trailer — [`rebuild`] is what adds one — because a caller that already
/// has a trailer must keep it.
#[must_use]
pub fn scan_for_objects(input: &[u8], limits: Limits) -> XrefTable {
    let mut table = XrefTable::default();
    let mut latest: BTreeMap<u32, usize> = BTreeMap::new();
    // Which of those objects is an object stream, asked while the parsed object is in hand
    // rather than by parsing the file a second time. A number whose *last* definition is not
    // an object stream is not one, which is why this is a map keyed the same way as `latest`
    // rather than a set that only grows.
    let mut streams: BTreeMap<u32, usize> = BTreeMap::new();

    let mut at = 0usize;
    while let Some(found) = input
        .get(at..)
        .and_then(|rest| rest.windows(3).position(|window| window == b"obj"))
    {
        let keyword_at = at.saturating_add(found);
        at = keyword_at.saturating_add(3);

        // Walk back over `G` and `N` to find where the header starts.
        if let Some(start) = header_start(input, keyword_at) {
            let mut parser = Parser::at(input, start, limits);
            if let Ok((id, object)) = parser.parse_indirect_object() {
                latest.insert(id.number, start);
                if is_object_stream(&object) {
                    streams.insert(id.number, start);
                } else {
                    streams.remove(&id.number);
                }
            }
        }
    }

    for (number, offset) in latest {
        table.entries.insert(number, Some(Location::Offset(offset)));
    }
    // Earliest in the file first: `Document`'s recovery reads them in that order so that a
    // later stream's copy of an object number wins, which is the rule this scan already
    // applies to two headers bearing one number.
    let mut ordered: Vec<(usize, u32)> = streams
        .into_iter()
        .map(|(number, offset)| (offset, number))
        .collect();
    ordered.sort_unstable();
    table.object_streams = ordered.into_iter().map(|(_, number)| number).collect();
    table
}

/// Whether a parsed object is Table 16's object stream.
///
/// §7.5.7's Table 16 makes the entry required and its value one name:
///
/// > The type of PDF object that this dictionary describes; shall be ObjStm for an object stream.
///
/// The entry is read from the dictionary as written, because a scan has no table to resolve an
/// indirect value through — and §7.3.10's `/Type` is a name here in every file that obeys the
/// table above.
fn is_object_stream(object: &Object) -> bool {
    object
        .as_stream()
        .and_then(|stream| stream.dict.get("Type"))
        .and_then(Object::as_name)
        .is_some_and(|name| name == &"ObjStm")
}

/// Walks backwards from an `obj` keyword to the start of `N G obj`.
fn header_start(input: &[u8], keyword_at: usize) -> Option<usize> {
    let mut at = keyword_at;

    for _ in 0..2 {
        // Skip whitespace before the number.
        while at > 0 && crate::lexer::is_whitespace(*input.get(at.saturating_sub(1))?) {
            at = at.saturating_sub(1);
        }
        let digits_end = at;
        while at > 0 && input.get(at.saturating_sub(1))?.is_ascii_digit() {
            at = at.saturating_sub(1);
        }
        if at == digits_end {
            // No digits where a number belongs: this `obj` is not a header, it is the tail
            // of a word such as `endobj` or part of a stream's contents.
            return None;
        }
    }

    Some(at)
}
