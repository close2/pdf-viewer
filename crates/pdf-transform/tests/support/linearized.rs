//! A reader of ISO 32000-2 Annex F's parameter dictionary and hint tables, written from the annex.
//!
//! Tables F.1 and §F.3 to F.9 are decoded here field by field — widths, item order, and §F.4.1's
//! rule that "a position greater than the hint stream offset shall have the hint stream length
//! added to it" — and [`faults`] asks one linearised file every question the annex lets a reader
//! check against the file itself: is each object where the parameter dictionary and the hint
//! tables say it is. Nothing here reads `pdf_syntax::linearize`'s own layout; the answer comes
//! from the file's cross-reference chain, which is the independent witness (trap 8).

#![expect(
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "test code over bounded offsets of files this suite wrote; `faults` is one list of \
              the annex's checkable statements, read top to bottom"
)]

use pdf_model::Pages;
use pdf_syntax::linearize::{Parameters, State, state};
use pdf_syntax::{Dictionary, Document, Limits, Location, Object, ObjectId};

/// §F.4.1's bit stream, read "high-order bit first".
struct Bits<'a> {
    /// The decoded hint stream.
    data: &'a [u8],
    /// The next bit.
    at: usize,
    /// Whether a read ran past the end.
    overran: bool,
}

impl Bits<'_> {
    /// The next `width` bits as an integer; zero, and `overran`, past the end.
    fn take(&mut self, width: u32) -> u64 {
        let mut value = 0u64;
        for _ in 0..width {
            let Some(byte) = self.data.get(self.at / 8).copied() else {
                self.overran = true;
                return 0;
            };
            let bit = (byte >> (7 - (self.at % 8))) & 1;
            value = (value << 1) | u64::from(bit);
            self.at += 1;
        }
        value
    }

    /// A width field: Table F.3's "16-bit numbers shall be used", for values 0 through 32.
    fn width(&mut self) -> u32 {
        let width = self.take(16);
        if width > 32 {
            self.overran = true;
            return 0;
        }
        u32::try_from(width).unwrap_or(0)
    }
}

/// One page's entry in the page offset hint table, decoded.
#[derive(Debug, Clone, Default)]
pub(crate) struct PageHint {
    /// Item 1: objects in the page.
    pub(crate) objects: u64,
    /// Item 2: its length in bytes.
    pub(crate) length: u64,
    /// Item 4: its shared object identifiers.
    pub(crate) shared: Vec<u64>,
    /// Item 5: their numerators.
    pub(crate) numerators: Vec<u64>,
    /// Item 6: its content stream's offset within the page.
    pub(crate) content_offset: u64,
    /// Item 7: its content stream's length.
    pub(crate) content_length: u64,
}

/// The page offset hint table, per Table F.3 and Table F.4.
#[derive(Debug, Clone)]
pub(crate) struct PageOffsets {
    /// Header item 2.
    pub(crate) first_page_location: u64,
    /// Header item 13.
    pub(crate) denominator: u64,
    /// Every page, in the file's order.
    pub(crate) pages: Vec<PageHint>,
    /// Whether the table ran past the stream.
    pub(crate) overran: bool,
}

/// One shared object group, per Table F.6.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SharedGroup {
    /// Item 1 plus header item 6.
    pub(crate) length: u64,
    /// Item 4 plus one.
    pub(crate) objects: u64,
}

/// The shared object hint table, per Table F.5 and Table F.6.
#[derive(Debug, Clone)]
pub(crate) struct SharedObjects {
    /// Header item 1.
    pub(crate) first_object: u64,
    /// Header item 2.
    pub(crate) first_location: u64,
    /// Header item 3.
    pub(crate) first_page_entries: u64,
    /// Every entry.
    pub(crate) groups: Vec<SharedGroup>,
    /// Whether the table ran past the stream.
    pub(crate) overran: bool,
}

/// Table F.3 and Table F.4, decoded.
pub(crate) fn page_offsets(data: &[u8], pages: usize) -> PageOffsets {
    let mut bits = Bits {
        data,
        at: 0,
        overran: false,
    };
    let least_objects = bits.take(32);
    let first_page_location = bits.take(32);
    let objects_bits = bits.width();
    let least_length = bits.take(32);
    let length_bits = bits.width();
    let least_offset = bits.take(32);
    let offset_bits = bits.width();
    let least_content = bits.take(32);
    let content_bits = bits.width();
    let shared_count_bits = bits.width();
    let identifier_bits = bits.width();
    let numerator_bits = bits.width();
    let denominator = bits.take(16);
    // The order of Table F.4's items: each item for every page before the next item.
    let mut out = vec![PageHint::default(); pages];
    for page in &mut out {
        page.objects = least_objects + bits.take(objects_bits);
    }
    for page in &mut out {
        page.length = least_length + bits.take(length_bits);
    }
    let counts: Vec<u64> = (0..pages).map(|_| bits.take(shared_count_bits)).collect();
    for (page, count) in out.iter_mut().zip(&counts) {
        for _ in 0..*count {
            page.shared.push(bits.take(identifier_bits));
        }
    }
    for (page, count) in out.iter_mut().zip(&counts) {
        for _ in 0..*count {
            page.numerators.push(bits.take(numerator_bits));
        }
    }
    for page in &mut out {
        page.content_offset = least_offset + bits.take(offset_bits);
    }
    for page in &mut out {
        page.content_length = least_content + bits.take(content_bits);
    }
    PageOffsets {
        first_page_location,
        denominator,
        pages: out,
        overran: bits.overran,
    }
}

/// Table F.5 and Table F.6, decoded from `data[at..]`.
pub(crate) fn shared_objects(data: &[u8], at: usize) -> SharedObjects {
    let mut bits = Bits {
        data,
        at: at * 8,
        overran: false,
    };
    let first_object = bits.take(32);
    let first_location = bits.take(32);
    let first_page_entries = bits.take(32);
    let total = bits.take(32);
    let count_bits = bits.width();
    let least = bits.take(32);
    let length_bits = bits.width();
    // A count past what the stream could hold is a fault, not an allocation.
    let total = usize::try_from(total).unwrap_or(0).min(data.len() * 8);
    let lengths: Vec<u64> = (0..total).map(|_| least + bits.take(length_bits)).collect();
    let signed: Vec<u64> = (0..total).map(|_| bits.take(1)).collect();
    for flag in &signed {
        if *flag == 1 {
            bits.take(128);
        }
    }
    let groups = lengths
        .iter()
        .map(|length| SharedGroup {
            length: *length,
            objects: bits.take(count_bits) + 1,
        })
        .collect();
    SharedObjects {
        first_object,
        first_location,
        first_page_entries,
        groups,
        overran: bits.overran,
    }
}

/// Table F.9, decoded from `data[at..]`: first object, location, count, length.
pub(crate) fn generic(data: &[u8], at: usize) -> (u64, u64, u64, u64) {
    let mut bits = Bits {
        data,
        at: at * 8,
        overran: false,
    };
    (bits.take(32), bits.take(32), bits.take(32), bits.take(32))
}

/// Table F.7 and Table F.8, decoded from `data[at..]`: each thumbnail's (page gap, objects,
/// length), and the header's two locations with the shared section's count and length.
#[derive(Debug, Clone)]
pub(crate) struct ThumbnailTable {
    /// Header item 1.
    pub(crate) first_object: u64,
    /// Header item 2.
    pub(crate) first_location: u64,
    /// Per page with a thumbnail: items 1, 2 and 3 with their header minima added.
    pub(crate) entries: Vec<(u64, u64, u64)>,
    /// Header items 9 to 12.
    pub(crate) shared: (u64, u64, u64, u64),
}

/// Table F.7 and Table F.8.
pub(crate) fn thumbnails(data: &[u8], at: usize) -> ThumbnailTable {
    let mut bits = Bits {
        data,
        at: at * 8,
        overran: false,
    };
    let first_object = bits.take(32);
    let first_location = bits.take(32);
    let count = usize::try_from(bits.take(32))
        .unwrap_or(0)
        .min(data.len() * 8);
    let gap_bits = bits.width();
    let least_length = bits.take(32);
    let length_bits = bits.width();
    let least_objects = bits.take(32);
    let objects_bits = bits.width();
    let shared = (bits.take(32), bits.take(32), bits.take(32), bits.take(32));
    let gaps: Vec<u64> = (0..count).map(|_| bits.take(gap_bits)).collect();
    let objects: Vec<u64> = (0..count)
        .map(|_| least_objects + bits.take(objects_bits))
        .collect();
    let lengths: Vec<u64> = (0..count)
        .map(|_| least_length + bits.take(length_bits))
        .collect();
    ThumbnailTable {
        first_object,
        first_location,
        entries: gaps
            .into_iter()
            .zip(objects)
            .zip(lengths)
            .map(|((gap, objects), length)| (gap, objects, length))
            .collect(),
        shared,
    }
}

/// Table F.11 and Table F.12, decoded from `data[at..]`: the first group's number and location,
/// and each group's (embedded file stream number, objects, length).
pub(crate) fn embedded_files(data: &[u8], at: usize) -> (u64, u64, Vec<(u64, u64, u64)>) {
    let mut bits = Bits {
        data,
        at: at * 8,
        overran: false,
    };
    let first_object = bits.take(32);
    let first_location = bits.take(32);
    let count = usize::try_from(bits.take(32))
        .unwrap_or(0)
        .min(data.len() * 8);
    let number_bits = bits.width();
    let objects_bits = bits.width();
    let length_bits = bits.width();
    let shared_bits = bits.width();
    let numbers: Vec<u64> = (0..count).map(|_| bits.take(number_bits)).collect();
    let objects: Vec<u64> = (0..count).map(|_| bits.take(objects_bits)).collect();
    let lengths: Vec<u64> = (0..count).map(|_| bits.take(length_bits)).collect();
    let shared: Vec<u64> = (0..count).map(|_| bits.take(shared_bits)).collect();
    // Item 5's identifiers are Table F.3 item 11 bits wide; this writer states none, and a
    // reader that met some would need that width, which it has not been told here.
    let _ = shared;
    (
        first_object,
        first_location,
        numbers
            .into_iter()
            .zip(objects)
            .zip(lengths)
            .map(|((number, objects), length)| (number, objects, length))
            .collect(),
    )
}

/// One linearised file, opened, with its parameters and its primary hint stream.
pub(crate) struct Read {
    /// The bytes.
    pub(crate) bytes: Vec<u8>,
    /// The document.
    pub(crate) document: Document,
    /// Table F.1.
    pub(crate) parameters: Parameters,
    /// The hint stream's dictionary.
    pub(crate) hints: Dictionary,
    /// Its data, which this writer never filters.
    pub(crate) data: Vec<u8>,
    /// Every page's object number, in page order.
    pub(crate) page_numbers: Vec<u64>,
}

impl Read {
    /// Opens a linearised file, or says which of Table F.1's or §F.3.6's statements fails.
    pub(crate) fn open(bytes: Vec<u8>) -> Result<Self, String> {
        let document = Document::open_with_limits(bytes.clone(), Limits::DEFAULT)
            .map_err(|error| format!("does not open: {error}"))?;
        let parameters = match state(&document) {
            State::Linearized(parameters) => parameters,
            other => return Err(format!("Table F.1: the file is {other:?}")),
        };
        let (offset, _) = parameters.primary_hints;
        let number = object_at(&bytes, offset)
            .ok_or_else(|| format!("/H: no object begins at offset {offset}"))?;
        let hint = document.get(ObjectId::new(number, 0));
        let stream = hint
            .as_stream()
            .ok_or("F.3.6: the hint tables are not in a stream")?;
        let data = match stream.dict.get("Filter") {
            None => stream.data.to_vec(),
            Some(_) => document
                .decoded_stream_data(stream)
                .map(|data| data.to_vec())
                .ok_or("F.3.6: the hint stream does not decode")?,
        };
        // `Pages::get` rather than `Pages::indices`, whose map also names each interior node of
        // the tree by the first page beneath it.
        let pages = Pages::new(&document);
        let page_numbers: Vec<u64> = (0..pages.len())
            .filter_map(|index| pages.get(index).and_then(|page| page.id))
            .map(|id| u64::from(id.number))
            .collect();
        Ok(Self {
            hints: stream.dict.clone(),
            data,
            page_numbers,
            bytes,
            document,
            parameters,
        })
    }

    /// Where the cross-reference chain says an object is.
    pub(crate) fn offset(&self, number: u64) -> Option<u64> {
        let number = u32::try_from(number).ok()?;
        match self.document.xref().location(number) {
            Some(Location::Offset(at)) => u64::try_from(at).ok(),
            _ => None,
        }
    }

    /// §F.4.1: "a position greater than the hint stream offset shall have the hint stream length
    /// added to it to determine the actual offset relative to the beginning of the PDF file."
    pub(crate) fn actual(&self, hinted: u64) -> u64 {
        let (offset, length) = self.parameters.primary_hints;
        if hinted > offset {
            hinted + length
        } else {
            hinted
        }
    }

    /// A Table F.2 key's position in the hint stream, where the dictionary states one.
    pub(crate) fn table(&self, key: &str) -> Option<usize> {
        self.hints
            .get(key)
            .and_then(Object::as_integer)
            .and_then(|at| usize::try_from(at).ok())
    }

    /// The page indices in the order their sections are written: §F.3.8's "If the first page of
    /// the PDF file is not page 0, this section shall start with page 0 and shall skip over the
    /// first page when its position in the sequence is reached."
    pub(crate) fn page_order(&self) -> Vec<usize> {
        let first = usize::try_from(self.parameters.first_page).unwrap_or(0);
        let count = self.page_numbers.len();
        let mut order = vec![first];
        order.extend((0..count).filter(|page| *page != first));
        order
    }
}

/// The object number of the `n 0 obj` that begins at `offset`.
pub(crate) fn object_at(bytes: &[u8], offset: u64) -> Option<u32> {
    let at = usize::try_from(offset).ok()?;
    let text = bytes.get(at..at.checked_add(24)?.min(bytes.len()))?;
    let text = std::str::from_utf8(text).ok()?;
    let mut words = text.split_whitespace();
    let number = words.next()?.parse().ok()?;
    let _generation = words.next()?;
    (words.next()? == "obj").then_some(number)
}

/// Every statement Annex F lets a reader check against the file, and each one that is false.
///
/// Empty is a file whose parameter dictionary and hint tables say where every object is, and are
/// right.
pub(crate) fn faults(bytes: &[u8]) -> Vec<String> {
    let read = match Read::open(bytes.to_vec()) {
        Ok(read) => read,
        Err(fault) => return vec![fault],
    };
    let mut out = Vec::new();
    let p = read.parameters;
    let bytes = &read.bytes;
    let mut fault = |text: String| out.push(text);

    // Table F.1.
    if p.length != bytes.len() as u64 {
        fault(format!("/L {} is not the length {}", p.length, bytes.len()));
    }
    let (h_offset, h_length) = p.primary_hints;
    match object_at(bytes, h_offset) {
        Some(number) if read.offset(u64::from(number)) == Some(h_offset) => {
            let end = usize::try_from(h_offset + h_length).unwrap_or(usize::MAX);
            if !bytes
                .get(..end)
                .is_some_and(|head| head.ends_with(b"endobj\n"))
            {
                fault("/H: the length does not end at the hint stream's endobj".to_owned());
            }
        }
        _ => fault("/H: the offset is not the hint stream's".to_owned()),
    }
    let first = usize::try_from(p.first_page).unwrap_or(usize::MAX);
    if read.page_numbers.get(first).copied() != Some(u64::from(p.first_page_object)) {
        fault(format!(
            "/O {} is not page {first}'s object",
            p.first_page_object
        ));
    }
    if p.pages != read.page_numbers.len() as u64 {
        fault(format!(
            "/N {} is not {} pages",
            p.pages,
            read.page_numbers.len()
        ));
    }
    let t = usize::try_from(p.main_cross_reference).unwrap_or(usize::MAX);
    let entry_zero = bytes.get(t.saturating_add(1)..t.saturating_add(20));
    if !bytes.get(t).is_some_and(u8::is_ascii_whitespace)
        || entry_zero != Some(&b"0000000000 65535 f "[..])
    {
        fault("/T does not precede the main table's entry 0".to_owned());
    }
    let prev = read
        .document
        .trailer()
        .get("Prev")
        .and_then(Object::as_integer)
        .and_then(|prev| usize::try_from(prev).ok());
    match prev {
        Some(prev) if bytes.get(prev..prev + 4) == Some(&b"xref"[..]) && prev < t => {}
        _ => fault("F.3.4: the first-page trailer's /Prev is not the main table".to_owned()),
    }

    // Table F.3 and Table F.4.
    let pages = page_offsets(&read.data, read.page_numbers.len());
    if pages.overran {
        fault("the page offset hint table runs past the hint stream".to_owned());
    }
    let order = read.page_order();
    let mut number = u64::from(p.first_page_object);
    let mut location = pages.first_page_location;
    for (at, (page, hint)) in order.iter().zip(&pages.pages).enumerate() {
        if at == 1 {
            number = 1;
        }
        let start = read.actual(location);
        if read.offset(number) != Some(start) {
            fault(format!(
                "page {page}: its entry says object {number} at {start}"
            ));
        }
        if read.page_numbers.get(*page).copied() != Some(number) {
            fault(format!(
                "page {page}: its section does not begin with its page object"
            ));
        }
        for member in number..number + hint.objects {
            match read.offset(member) {
                Some(offset) if offset >= start && offset < start + hint.length => {}
                other => fault(format!(
                    "page {page}: object {member} is at {other:?}, outside {start}+{}",
                    hint.length
                )),
            }
        }
        if at == 0 && start + hint.length != p.end_of_first_page {
            fault(format!(
                "/E {} is not where the first page's section ends, {}",
                p.end_of_first_page,
                start + hint.length
            ));
        }
        if hint.content_length > 0 {
            let at = start + hint.content_offset;
            if object_at(bytes, at).is_none() {
                fault(format!("page {page}: item 6 names no object at {at}"));
            }
        }
        if at == 0 && !hint.shared.is_empty() {
            fault("Table F.4 item 3: \"For the first page, this number shall be 0\"".to_owned());
        }
        for numerator in &hint.numerators {
            if *numerator > pages.denominator + 1 {
                fault(format!("page {page}: numerator {numerator} past d + 1"));
            }
        }
        number += hint.objects;
        location += hint.length;
    }

    // Table F.5 and Table F.6.
    let Some(s) = read.table("S") else {
        fault("Table F.2: /S is required".to_owned());
        return out;
    };
    let shared = shared_objects(&read.data, s);
    if shared.overran {
        fault("the shared object hint table runs past the hint stream".to_owned());
    }
    for hint in &pages.pages {
        if hint
            .shared
            .iter()
            .any(|id| *id >= shared.groups.len() as u64)
        {
            fault("a page names a shared object identifier the table does not hold".to_owned());
        }
    }
    let mut location = pages.first_page_location;
    let mut number = u64::from(p.first_page_object);
    for (at, group) in shared.groups.iter().enumerate() {
        if at as u64 == shared.first_page_entries {
            location = shared.first_location;
            number = shared.first_object;
        }
        let start = read.actual(location);
        if read.offset(number) != Some(start) {
            fault(format!(
                "shared group {at}: its entry says object {number} at {start}"
            ));
        }
        for member in number..number + group.objects {
            match read.offset(member) {
                Some(offset) if offset >= start && offset < start + group.length => {}
                other => fault(format!(
                    "shared group {at}: object {member} is at {other:?}"
                )),
            }
        }
        if (at as u64) < shared.first_page_entries && start + group.length > p.end_of_first_page {
            fault(format!("shared group {at} is a first-page group past /E"));
        }
        location += group.length;
        number += group.objects;
    }

    // Tables F.9 and F.10's first four items.
    for key in ["O", "A", "E", "V", "I", "C", "L", "R"] {
        let Some(at) = read.table(key) else {
            continue;
        };
        let (first, location, count, length) = generic(&read.data, at);
        if count == 0 {
            continue;
        }
        let start = read.actual(location);
        if read.offset(first) != Some(start) {
            fault(format!(
                "/{key}: item 1's object is not at item 2's location"
            ));
        }
        for member in first..first + count {
            match read.offset(member) {
                Some(offset) if offset >= start && offset < start + length => {}
                other => fault(format!("/{key}: object {member} is at {other:?}")),
            }
        }
    }
    // Table F.7 and Table F.8: each page's thumbnail objects, at locations accumulated from the
    // first, then the thumbnail shared objects where items 9 to 12 say.
    if let Some(at) = read.table("T") {
        let table = thumbnails(&read.data, at);
        let mut location = table.first_location;
        let mut number = table.first_object;
        for (entry, (_, objects, length)) in table.entries.iter().enumerate() {
            let start = read.actual(location);
            for member in number..number + objects {
                match read.offset(member) {
                    Some(offset) if offset >= start && offset < start + length => {}
                    other => fault(format!("/T entry {entry}: object {member} is at {other:?}")),
                }
            }
            location += length;
            number += objects;
        }
        let (first, location, count, length) = table.shared;
        let start = read.actual(location);
        for member in first..first + count {
            match read.offset(member) {
                Some(offset) if offset >= start && offset < start + length => {}
                other => fault(format!("/T shared: object {member} is at {other:?}")),
            }
        }
    }
    // Table F.11 and Table F.12: each group's objects, at locations accumulated from the first.
    if let Some(at) = read.table("B") {
        let (first, location, groups) = embedded_files(&read.data, at);
        let mut location = location;
        let mut number = first;
        for (group, (file, objects, length)) in groups.iter().enumerate() {
            let start = read.actual(location);
            if *file < number || *file >= number + objects {
                fault(format!(
                    "/B group {group}: stream {file} is not among its objects"
                ));
            }
            for member in number..number + objects {
                match read.offset(member) {
                    Some(offset) if offset >= start && offset < start + length => {}
                    other => fault(format!("/B group {group}: object {member} is at {other:?}")),
                }
            }
            location += length;
            number += objects;
        }
    }

    // Table F.2's conditions, asked of the document.
    let catalog = read.document.catalog().ok();
    let has = |key: &str| {
        catalog
            .as_ref()
            .is_some_and(|catalog| catalog.get(key).is_some())
    };
    let outline = catalog
        .as_ref()
        .and_then(|catalog| catalog.get("Outlines"))
        .is_some_and(|value| read.document.resolve(value).as_dict().is_some());
    let names = catalog
        .as_ref()
        .and_then(|catalog| catalog.get("Names"))
        .map(|value| read.document.resolve(value));
    let in_names = |key: &str| {
        names
            .as_ref()
            .and_then(Object::as_dict)
            .is_some_and(|names| names.get(key).is_some())
    };
    let thumbnails_exist = read.page_numbers.iter().any(|number| {
        u32::try_from(*number).is_ok_and(|number| {
            read.document
                .get(ObjectId::new(number, 0))
                .as_dict()
                .is_some_and(|page| page.get("Thumb").is_some())
        })
    });
    for (key, present) in [
        ("T", thumbnails_exist),
        // "Required only if article threads exist": a `/Threads` array with a thread in it.
        (
            "A",
            catalog
                .as_ref()
                .and_then(|catalog| catalog.get("Threads"))
                .map(|value| read.document.resolve(value))
                .is_some_and(|threads| threads.as_array().is_some_and(|items| !items.is_empty())),
        ),
        ("E", has("Dests") || in_names("Dests")),
        ("R", in_names("Renditions")),
        ("O", outline),
        ("V", has("AcroForm")),
        ("C", has("StructTreeRoot")),
        ("I", read.document.trailer().get("Info").is_some()),
    ] {
        if present && read.table(key).is_none() {
            fault(format!(
                "Table F.2: /{key} is required of this document and absent"
            ));
        }
    }
    out
}
