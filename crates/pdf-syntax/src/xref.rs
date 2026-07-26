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
#[derive(Debug, Clone, Default)]
pub struct XrefTable {
    entries: BTreeMap<u32, Location>,
    trailer: Dictionary,
    /// How the table was obtained, for diagnostics and for the comparison report.
    recovered_by_scan: bool,
}

impl XrefTable {
    /// Returns the location of an object, if known.
    #[must_use]
    pub fn location(&self, number: u32) -> Option<Location> {
        self.entries.get(&number).copied()
    }

    /// Returns the trailer dictionary.
    #[must_use]
    pub fn trailer(&self) -> &Dictionary {
        &self.trailer
    }

    /// Returns the number of known objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if no objects are known.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns every known object number, ascending.
    pub fn object_numbers(&self) -> impl Iterator<Item = u32> {
        self.entries.keys().copied()
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

    /// Adds an entry if the object number is not already known.
    ///
    /// First writer wins, which is what makes the layered read order correct: the newest
    /// cross-reference section is read first, and older sections must not overwrite it.
    fn add(&mut self, number: u32, location: Location) {
        self.entries.entry(number).or_insert(location);
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

/// How many bytes at the end of the file are searched for `startxref`.
///
/// The specification does not bound the trailer's size, but a `startxref` further back
/// than this is not a trailer, it is a coincidence.
const STARTXREF_SEARCH_WINDOW: usize = 2048;

/// The most cross-reference sections that will be followed through `/Prev`.
///
/// A file with a genuine chain this long does not exist; one that claims to has a cycle or
/// is trying to make the reader loop. The cycle is also detected directly, so this is a
/// second line of defence rather than the only one.
const MAX_XREF_SECTIONS: usize = 1024;

/// Reads the cross-reference information for a file, recovering if necessary.
///
/// # Errors
///
/// [`SyntaxError::NoHeader`] if the file has no `%PDF-` header *and* no objects could be
/// found either, and [`SyntaxError::NoCrossReferences`] if neither the table nor a scan
/// yields any object.
pub fn read(input: &[u8], limits: Limits) -> SyntaxResult<XrefTable> {
    // The header may be preceded by junk — files served through mail gateways acquire it —
    // so the specification's "first line" is relaxed to "somewhere near the start".
    let header_window = input.len().min(1024);
    let has_header = input
        .get(..header_window)
        .unwrap_or_default()
        .windows(5)
        .any(|window| window == b"%PDF-");

    if let Some(table) = read_from_startxref(input, limits)
        && !table.is_empty()
    {
        return Ok(table);
    }

    // The table was absent, unreadable, or empty. Scan.
    //
    // A missing header does not stop this. Files lose their header to transfer damage and
    // to producers that never wrote one, and the objects after it are usually intact — so
    // refusing on the header alone rejects documents both poppler and mupdf recover and
    // read. The header is a diagnosis, not a gate: it only decides which error is reported
    // when nothing is found.
    let mut table = scan_for_objects(input, limits);
    table.recovered_by_scan = true;

    if table.is_empty() {
        if !has_header {
            return Err(SyntaxError::NoHeader {
                searched: header_window,
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

/// Follows `startxref` and the `/Prev` chain.
///
/// Returns `None` when the chain cannot be read at all, leaving the caller to scan.
fn read_from_startxref(input: &[u8], limits: Limits) -> Option<XrefTable> {
    let mut next = find_startxref(input)?;
    let mut table = XrefTable::default();
    let mut visited = std::collections::BTreeSet::new();

    for _ in 0..MAX_XREF_SECTIONS {
        // A cycle in the `/Prev` chain would otherwise loop until the section cap, doing
        // real work each time.
        if !visited.insert(next) {
            break;
        }
        if next >= input.len() {
            break;
        }

        let Some(section) = read_section(input, next, limits) else {
            break;
        };
        for (number, location) in section.entries {
            table.add(number, location);
        }
        table.merge_trailer(&section.trailer);

        // A cross-reference stream may also carry `/XRefStm`, a hybrid-reference file's
        // pointer to a stream holding entries the table omits. Read it before `/Prev` so
        // its entries take precedence over older sections.
        if let Some(hybrid) = section
            .trailer
            .get("XRefStm")
            .and_then(Object::as_integer)
            .and_then(|value| usize::try_from(value).ok())
            && let Some(extra) = read_section(input, hybrid, limits)
        {
            for (number, location) in extra.entries {
                table.add(number, location);
            }
        }

        match section
            .trailer
            .get("Prev")
            .and_then(Object::as_integer)
            .and_then(|value| usize::try_from(value).ok())
        {
            Some(previous) => next = previous,
            None => break,
        }
    }

    Some(table)
}

/// One cross-reference section's entries and trailer.
struct Section {
    entries: Vec<(u32, Location)>,
    trailer: Dictionary,
}

/// Reads either a classic `xref` table or a cross-reference stream at `offset`.
fn read_section(input: &[u8], offset: usize, limits: Limits) -> Option<Section> {
    let mut parser = Parser::at(input, offset, limits);
    let probe = parser.position();

    // A classic table begins with the `xref` keyword.
    let mut lexer = crate::lexer::Lexer::at(input, probe);
    if lexer.next_token() == Some(crate::Token::Keyword(b"xref".to_vec())) {
        return read_classic_table(input, lexer.position(), limits);
    }

    // Otherwise expect `N G obj` introducing a cross-reference stream.
    let (_, object) = parser.parse_indirect_object().ok()?;
    let stream = object.as_stream()?.clone();
    read_xref_stream(&stream, limits)
}

/// Reads a classic `xref` table, positioned just after the keyword.
fn read_classic_table(input: &[u8], offset: usize, limits: Limits) -> Option<Section> {
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
                        return Some(finish(entries, input, entry_at, limits));
                    };
                    let Some(crate::Token::Integer(_generation)) = lexer.next_token() else {
                        return Some(finish(entries, input, entry_at, limits));
                    };
                    let Some(crate::Token::Keyword(kind)) = lexer.next_token() else {
                        return Some(finish(entries, input, entry_at, limits));
                    };

                    // `f` marks a free entry, which names no object.
                    if kind == b"n"
                        && let (Ok(number), Ok(position)) = (
                            u32::try_from(u64::from(first).saturating_add(u64::from(index))),
                            usize::try_from(position),
                        )
                    {
                        entries.push((number, Location::Offset(position)));
                    }
                }
            }
            Some(crate::Token::Keyword(word)) if word == b"trailer" => {
                let mut parser = Parser::at(input, lexer.position(), limits);
                let trailer = parser
                    .parse_object()
                    .ok()
                    .and_then(|object| object.as_dict().cloned())
                    .unwrap_or_default();
                return Some(Section { entries, trailer });
            }
            _ => {
                lexer.seek(rewind);
                break;
            }
        }
    }

    Some(finish(entries, input, lexer.position(), limits))
}

/// Builds a section, looking for a trailer from `offset` onwards.
fn finish(entries: Vec<(u32, Location)>, input: &[u8], offset: usize, limits: Limits) -> Section {
    let trailer = find_trailer_from(input, offset, limits).unwrap_or_default();
    Section { entries, trailer }
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
fn read_xref_stream(stream: &crate::object::Stream, limits: Limits) -> Option<Section> {
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

    let mut entries = Vec::new();
    let mut cursor = 0usize;

    for pair in index.chunks(2) {
        let (Some(&first), Some(&count)) = (pair.first(), pair.get(1)) else {
            break;
        };
        let Ok(first) = u32::try_from(first) else {
            continue;
        };

        for offset in 0..count.max(0) {
            let Some(record) = data.get(cursor..cursor.saturating_add(row)) else {
                return Some(Section {
                    entries,
                    trailer: stream.dict.clone(),
                });
            };
            cursor = cursor.saturating_add(row);

            let mut fields = [1u64, 0, 0];
            let mut at = 0usize;
            for (slot, &width) in widths.iter().enumerate().take(3) {
                if width == 0 {
                    // A zero width means "use the default", which is 1 for the type field
                    // and 0 for the others.
                    continue;
                }
                let bytes = record.get(at..at.saturating_add(width))?;
                at = at.saturating_add(width);
                let mut value = 0u64;
                for &byte in bytes {
                    value = value.saturating_mul(256).saturating_add(u64::from(byte));
                }
                if let Some(field) = fields.get_mut(slot) {
                    *field = value;
                }
            }

            let Ok(number) =
                u32::try_from(u64::from(first).saturating_add(u64::try_from(offset).unwrap_or(0)))
            else {
                continue;
            };

            match fields[0] {
                // Type 1: an object at a byte offset.
                1 => {
                    if let Ok(position) = usize::try_from(fields[1]) {
                        entries.push((number, Location::Offset(position)));
                    }
                }
                // Type 2: an object inside an object stream.
                2 => {
                    if let (Ok(stream_number), Ok(index_in_stream)) =
                        (u32::try_from(fields[1]), u32::try_from(fields[2]))
                    {
                        entries.push((
                            number,
                            Location::InStream {
                                stream: stream_number,
                                index: index_in_stream,
                            },
                        ));
                    }
                }
                // Type 0 is a free object, and any other type is undefined and ignored
                // per the specification's forward-compatibility rule.
                _ => {}
            }
        }
    }

    Some(Section {
        entries,
        trailer: stream.dict.clone(),
    })
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
fn find_startxref(input: &[u8]) -> Option<usize> {
    let window = input.len().min(STARTXREF_SEARCH_WINDOW);
    let tail_start = input.len().saturating_sub(window);
    let tail = input.get(tail_start..)?;

    let found = tail
        .windows(b"startxref".len())
        .rposition(|candidate| candidate == b"startxref")?;

    let after = tail_start
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
fn scan_for_objects(input: &[u8], limits: Limits) -> XrefTable {
    let mut table = XrefTable::default();
    let mut latest: BTreeMap<u32, usize> = BTreeMap::new();

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
            if let Ok((id, _)) = parser.parse_indirect_object() {
                latest.insert(id.number, start);
            }
        }
    }

    for (number, offset) in latest {
        table.entries.insert(number, Location::Offset(offset));
    }
    table
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
