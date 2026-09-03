//! Objects back to bytes, and ISO 32000-2 §7.5.6's incremental update.
//!
//! # What this is for, and what it is not
//!
//! `CLAUDE.md` excludes *authoring* a document: linearisation, object-stream packing, and
//! everything else whose requirements fall on a generator. It permits exactly one kind of
//! writing, and this is it — §7.5.6's incremental update, which appends what a person did to
//! the file they did it to:
//!
//! > The contents of a PDF file can be updated incrementally without rewriting the entire file.
//! > When updating a PDF file incrementally, changes shall be appended to the end of the file,
//! > leaving its original contents intact.
//!
//! The producer's bytes stay in the file, byte for byte, under whatever was added. That is what
//! makes this the one form of writing this tree is placed to get right: nothing here decides how
//! a document should be laid out, only how to say "this object now reads like this".
//!
//! # The two halves
//!
//! [`object`] writes one object in the syntax clause 7 defines for it. [`incremental_update`]
//! writes the section §7.5.6 describes: the new objects, a cross-reference section covering
//! them, and a trailer whose `/Prev` chains to what was there before.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::object::{Dictionary, Name, Object, ObjectId};

/// Why an incremental update could not be written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UpdateError {
    /// The document's own cross-reference table was rebuilt by scanning.
    ///
    /// §7.5.6 makes an update a *chain*: the new section's `/Prev` names the offset of the one
    /// before it. A file whose table had to be reconstructed has no offset worth chaining to,
    /// and an update that pointed at a broken table would be as unusable as the table.
    #[error(
        "this file's cross-reference table was rebuilt by scanning, so an update has nothing to chain to"
    )]
    Recovered,
    /// The document is encrypted and its own key would not encrypt what is being written.
    ///
    /// §7.6.2 has every string and stream in an encrypted document encrypted with the
    /// document's key, so writing plaintext into one would produce a file whose new objects
    /// decrypt to nonsense. Every document this reader *opened* has that key, so the one
    /// remaining way here is a document opened without one — §7.6.6's file whose body needs
    /// no key and whose attachments do — asked to write an object that needs it.
    #[error("this file's own encryption cannot be applied to what this update writes (§7.6.2)")]
    Encryption,
    /// The file does not end with the `startxref` §7.5.5 requires.
    #[error("this file states no startxref, so there is no cross-reference section to chain to")]
    NoStartxref,
    /// The trailer states no `/Root`, which §7.5.5 makes required.
    #[error("this file's trailer states no /Root")]
    NoRoot,
    /// The file is on disk and the process cannot hold it whole, which an update that appends
    /// to every byte of it needs (ADR 0809).
    #[error("the file cannot be held whole to append to it: {0}")]
    CannotHold(crate::NoRoom),
}

/// Writes one object in the syntax of clause 7.
///
/// Not a formatter for a whole file: this is what an indirect object's body looks like, and
/// [`incremental_update`] is what puts `N G obj` … `endobj` around it.
pub fn object(value: &Object, out: &mut Vec<u8>) {
    match value {
        Object::Null => out.extend_from_slice(b"null"),
        Object::Boolean(true) => out.extend_from_slice(b"true"),
        Object::Boolean(false) => out.extend_from_slice(b"false"),
        Object::Integer(value) => {
            let mut text = String::new();
            let _ = write!(text, "{value}");
            out.extend_from_slice(text.as_bytes());
        }
        Object::Real(value) => out.extend_from_slice(real(*value).as_bytes()),
        Object::String(bytes) => string(bytes, out),
        Object::Name(name) => write_name(name, out),
        Object::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b' ');
                }
                object(item, out);
            }
            out.push(b']');
        }
        Object::Dictionary(dict) => dictionary(dict, out),
        Object::Stream(stream) => {
            dictionary(&stream.dict, out);
            out.extend_from_slice(b"\nstream\n");
            out.extend_from_slice(&stream.data);
            out.extend_from_slice(b"\nendstream");
        }
        Object::Reference(id) => {
            let mut text = String::new();
            let _ = write!(text, "{} {} R", id.number, id.generation);
            out.extend_from_slice(text.as_bytes());
        }
    }
}

/// §7.3.7's dictionary: `<<` key value … `>>`.
fn dictionary(dict: &Dictionary, out: &mut Vec<u8>) {
    out.extend_from_slice(b"<<");
    for (key, value) in dict.iter() {
        out.push(b' ');
        write_name(key, out);
        out.push(b' ');
        object(value, out);
    }
    out.extend_from_slice(b" >>");
}

/// §7.3.5's name: the SOLIDUS the clause requires, and [`Name::escaped`] for the rest.
///
/// > When writing a name in a PDF file, a SOLIDUS (2Fh) (/) shall be used to introduce a name.
///
/// The escaping itself is [`Name::escaped`] and is not spelled here. It used to be, and the cost
/// of that was §7.3.5 being implemented in one direction only: `pdf_model::variable_text` writes
/// a font name into a content stream, could not reach this function, and wrote the name raw
/// (ADR 0453).
fn write_name(name: &Name, out: &mut Vec<u8>) {
    out.push(b'/');
    out.extend_from_slice(name.escaped().as_bytes());
}

/// §7.3.4.3's hexadecimal string, which needs no escaping rules at all.
///
/// The literal form of §7.3.4.2 is shorter for text and needs three characters escaped, a
/// balanced-parenthesis rule, and a decision about what to do with a byte that is not printable.
/// The hexadecimal form is twice the length and has none of that, and what this writes are form
/// field values: a person's name, not a page of content.
fn string(bytes: &[u8], out: &mut Vec<u8>) {
    out.push(b'<');
    for byte in bytes {
        let _ = write!(HexSink(out), "{byte:02X}");
    }
    out.push(b'>');
}

/// A `fmt::Write` that appends to a byte vector, for the one place a byte is formatted as hex.
struct HexSink<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for HexSink<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

/// §7.3.3's real number, written so that it reads back as itself.
///
/// The clause's own note on precision — "a conforming writer shall not use an exponential
/// format" — rules out `{:e}`, and an integral value is written without a fractional part
/// because that is what every producer does and what the type in a dictionary usually is.
fn real(value: f64) -> String {
    if !value.is_finite() {
        // §7.3.3 has no spelling for an infinity or a NaN, and a file containing one would be
        // a file this program wrote and could not read. Zero is the only value that is
        // certainly writable, and the alternative — refusing — would fail a save over a value
        // no document states.
        return "0".to_owned();
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        let mut text = String::new();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "guarded by the fract() and magnitude test immediately above"
        )]
        let integral = value as i64;
        let _ = write!(text, "{integral}");
        return text;
    }
    let mut text = format!("{value:.6}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

/// Appends ISO 32000-2 §7.5.6's incremental update to a document's own bytes.
///
/// `replacements` are the objects that now read differently. Each is written as an indirect
/// object with the generation the document's own table gives it, followed by a cross-reference
/// section covering exactly those objects and a trailer chaining to what was there.
///
/// The result is the **whole file**: the original bytes, unchanged, with the update after them.
/// That is what §7.5.6 describes and it is what makes this safe — the producer's bytes are still
/// there, and a reader that cannot make sense of the update can still read the file that was
/// signed or archived.
///
/// # The cross-reference section's form
///
/// The same kind the file already uses. A document whose last section is §7.5.8's cross-reference
/// *stream* gets one of those, and one with §7.5.4's table gets a table. Nothing in the standard
/// requires them to match — a PDF 1.5 reader handles both — but a file whose sections are all one
/// kind is a file whose next reader has one thing to do, and the cost is forty lines.
///
/// # An encrypted document
///
/// `replacements` are always plaintext, whatever the file is: §7.6.2's ciphers are applied here,
/// per object, through the same code that removed them on the way in. Two things stay in the
/// clear and both are the standard's own instruction rather than an omission — §7.5.8.2's "the
/// cross-reference stream shall not be encrypted and strings appearing in the cross-reference
/// stream dictionary shall not be encrypted", and Table 15's `/ID`, whose strings "shall be
/// direct objects and shall be unencrypted" because they are an input to the key that would
/// otherwise decrypt them. Both are built below rather than passed through the encryptor, so
/// neither is a rule this function has to remember.
///
/// # Errors
///
/// [`UpdateError`], which names the four documents this refuses: one whose table was rebuilt by
/// scanning, one whose encryption cannot be applied to what is written, one with no `startxref`,
/// and one whose trailer has no `/Root`.
pub fn incremental_update(
    document: &crate::Document,
    replacements: &BTreeMap<ObjectId, Object>,
) -> Result<Vec<u8>, UpdateError> {
    incremental_update_freeing(document, replacements, &[])
}

/// [`incremental_update`], with objects the update deletes marked free.
///
/// §7.5.6 is a `shall` about what happens to a deleted object — "[d]eleted objects shall be left
/// unchanged in the PDF file, but shall be marked as deleted by means of their cross-reference
/// entries" — and §7.5.4 gives a cross-reference section two ways to hold a free entry. The
/// first is the linked list headed by object 0, which a deletion "shall be added to" with its
/// generation "incremented by 1". **An update cannot use it**: §7.5.4's own NOTE 3 says that
/// "cross reference subsections of incremental updates can never have an object number of
/// zero", so the head of the list is not an entry this section may rewrite, and an entry chained
/// to a head that does not name it is not in the list. The second way is the one written here:
///
/// > Using the second mechanism, the table may contain other free entries that link back to
/// > object number 0 and have a generation number of 65,535, even though these entries are not in
/// > the linked list itself.
///
/// A generation of 65,535 is the one the clause says "shall never be reused", which is also the
/// right statement about a number whose object's bytes are still in the file under it. Each
/// freed id must not also be in `replacements`; the caller decides which an object is.
///
/// # Errors
///
/// [`incremental_update`]'s.
pub fn incremental_update_freeing(
    document: &crate::Document,
    replacements: &BTreeMap<ObjectId, Object>,
    freed: &[ObjectId],
) -> Result<Vec<u8>, UpdateError> {
    if document.was_recovered() {
        return Err(UpdateError::Recovered);
    }
    let previous = startxref(document.bytes()).ok_or(UpdateError::NoStartxref)?;
    // §7.5.6's update is appended to the file's every byte, so the file is needed whole — read
    // once and kept where it is on disk, or refused by name where the process cannot hold it.
    let original = document.bytes().whole().map_err(UpdateError::CannotHold)?;
    let root = document
        .trailer()
        .get("Root")
        .cloned()
        .ok_or(UpdateError::NoRoot)?;

    let mut out = original.to_vec();
    // §7.5.6's own requirement on what precedes an update: the file it appends to ends with
    // `%%EOF`, and a file that does not end with a line break would run the first new object
    // into it.
    if !out.ends_with(b"\n") && !out.ends_with(b"\r") {
        out.push(b'\n');
    }

    let mut offsets: Vec<(ObjectId, usize)> = Vec::new();
    for (id, value) in replacements {
        // §7.6.2 on the way out. The caller hands over plaintext — what the person typed,
        // in the object it belongs to — because that is the only form the rest of this tree
        // ever holds; an encrypted document turns it back into what its own key expects
        // here, at the one point where the object's identity is known and §7.6.3.2 step (a)
        // needs it. A document with no `/Encrypt` gets the value back unchanged.
        let value = document
            .encrypt_for_update(*id, value)
            .ok_or(UpdateError::Encryption)?;
        offsets.push((*id, out.len()));
        let mut header = String::new();
        let _ = writeln!(header, "{} {} obj", id.number, id.generation);
        out.extend_from_slice(header.as_bytes());
        object(&value, &mut out);
        out.extend_from_slice(b"\nendobj\n");
    }

    let highest = offsets
        .iter()
        .map(|(id, _)| id.number)
        .chain(freed.iter().map(|id| id.number))
        .max()
        .unwrap_or_default();
    let size = u64::from(highest)
        .saturating_add(1)
        .max(size_of_document(document));

    let stream_form = document
        .trailer()
        .get("Type")
        .and_then(Object::as_name)
        .is_some_and(|name| name.as_bytes() == b"XRef");
    // One list of entries, in object-number order, free ones marked: what both forms of
    // section are written from.
    let mut entries: Vec<Entry> = offsets
        .iter()
        .map(|(id, offset)| Entry {
            number: id.number,
            generation: id.generation,
            state: State::InUse { offset: *offset },
        })
        .chain(freed.iter().map(|id| Entry {
            number: id.number,
            generation: FREE_FOREVER,
            state: State::Free,
        }))
        .collect();
    entries.sort_unstable_by_key(|entry| entry.number);

    if stream_form {
        cross_reference_stream(&mut out, &entries, size, previous, &root, document);
    } else {
        cross_reference_table(&mut out, &entries, size, previous, &root, document);
    }
    Ok(out)
}

/// §7.5.4's generation for a free entry outside the linked list: "shall never be reused".
const FREE_FOREVER: u16 = 65_535;

/// One cross-reference entry the update writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    /// The object number.
    number: u32,
    /// The generation: the object's own for one in use, [`FREE_FOREVER`] for one freed.
    generation: u16,
    /// In use at an offset, or free.
    state: State,
}

/// Whether an entry names bytes or the absence of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// `n`, at this offset into the file.
    InUse {
        /// Where the object was written.
        offset: usize,
    },
    /// `f`, linking back to object 0.
    Free,
}

/// The `/Size` the document's own trailer states, which the update may only raise.
fn size_of_document(document: &crate::Document) -> u64 {
    document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u64::try_from(size).ok())
        .unwrap_or_default()
}

/// §7.5.4's cross-reference table, in subsections of consecutive object numbers.
fn cross_reference_table(
    out: &mut Vec<u8>,
    entries: &[Entry],
    size: u64,
    previous: usize,
    root: &Object,
    document: &crate::Document,
) {
    let at = out.len();
    let mut text = String::from("xref\n");
    for run in runs(entries) {
        let _ = writeln!(text, "{} {}", run[0].number, run.len());
        for entry in run {
            // §7.5.4's entry is exactly twenty bytes, and the clause spells out both halves:
            // ten digits of offset, five of generation, `n` for an in-use object, and a
            // two-character end-of-line marker. A free entry's first field is "the 10-digit
            // object number of the next free object", which for one outside the list is 0.
            match entry.state {
                State::InUse { offset } => {
                    let _ = writeln!(text, "{offset:010} {:05} n ", entry.generation);
                }
                State::Free => {
                    let _ = writeln!(text, "{:010} {:05} f ", 0, entry.generation);
                }
            }
        }
    }
    out.extend_from_slice(text.as_bytes());

    let mut trailer = Dictionary::new();
    trailer.insert(Name::new(&b"Size"[..]), Object::Integer(as_integer(size)));
    trailer.insert(Name::new(&b"Root"[..]), root.clone());
    trailer.insert(
        Name::new(&b"Prev"[..]),
        Object::Integer(as_integer(previous as u64)),
    );
    carry_forward(document, &mut trailer);
    identify(document, &mut trailer, out);

    out.extend_from_slice(b"trailer\n");
    object(&Object::Dictionary(trailer), out);
    let mut tail = String::new();
    let _ = write!(tail, "\nstartxref\n{at}\n%%EOF\n");
    out.extend_from_slice(tail.as_bytes());
}

/// §7.5.8's cross-reference stream, uncompressed, with the three-field entries of Table 18.
fn cross_reference_stream(
    out: &mut Vec<u8>,
    entries: &[Entry],
    size: u64,
    previous: usize,
    root: &Object,
    document: &crate::Document,
) {
    // The stream is itself an object, and its own entry has to be in it — so it takes the next
    // number after everything else, and its offset is where it is about to be written.
    let stream_number = u32::try_from(size).unwrap_or(u32::MAX);
    let at = out.len();

    let mut entries = entries.to_vec();
    entries.push(Entry {
        number: stream_number,
        generation: 0,
        state: State::InUse { offset: at },
    });
    entries.sort_unstable_by_key(|entry| entry.number);

    let mut index = Vec::new();
    let mut data = Vec::new();
    for run in runs(&entries) {
        index.push(Object::Integer(as_integer(u64::from(run[0].number))));
        index.push(Object::Integer(as_integer(run.len() as u64)));
        for entry in run {
            // `/W [1 4 2]` is the width of the three fields, which is what says how to read
            // these bytes. Table 18's type 1 is an uncompressed object at an offset; type 0 is
            // a free object, whose second field is "the object number of the next free object"
            // and whose third is "the generation number to use if this object number is used
            // again" — 0 and 65,535, for the reason `incremental_update_freeing` gives.
            match entry.state {
                State::InUse { offset } => {
                    data.push(1);
                    data.extend_from_slice(
                        &u32::try_from(offset).unwrap_or(u32::MAX).to_be_bytes(),
                    );
                }
                State::Free => {
                    data.push(0);
                    data.extend_from_slice(&0_u32.to_be_bytes());
                }
            }
            data.extend_from_slice(&entry.generation.to_be_bytes());
        }
    }

    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"XRef"[..])),
    );
    dict.insert(
        Name::new(&b"Size"[..]),
        Object::Integer(as_integer(u64::from(stream_number).saturating_add(1))),
    );
    dict.insert(
        Name::new(&b"W"[..]),
        Object::Array(vec![
            Object::Integer(1),
            Object::Integer(4),
            Object::Integer(2),
        ]),
    );
    dict.insert(Name::new(&b"Index"[..]), Object::Array(index));
    dict.insert(Name::new(&b"Root"[..]), root.clone());
    dict.insert(
        Name::new(&b"Prev"[..]),
        Object::Integer(as_integer(previous as u64)),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(as_integer(data.len() as u64)),
    );
    carry_forward(document, &mut dict);
    identify(document, &mut dict, out);

    let mut header = String::new();
    let _ = writeln!(header, "{stream_number} 0 obj");
    out.extend_from_slice(header.as_bytes());
    object(
        &Object::Stream(std::sync::Arc::new(crate::object::Stream {
            dict,
            data: data.into(),
            decryption_failed: false,
        })),
        out,
    );
    let mut tail = String::new();
    let _ = write!(tail, "\nendobj\nstartxref\n{at}\n%%EOF\n");
    out.extend_from_slice(tail.as_bytes());
}

/// §14.4's file identifier, updated as the clause requires.
///
/// > The first byte string shall be a permanent identifier based on the contents of the PDF file
/// > at the time it was originally created and shall not change when the PDF file is updated.
///
/// That is the published sentence; Errata Collection 3's Issue #328 strikes *contents of the*
/// out of it and *'s contents* out of its neighbour, so both identifiers become ones *based on
/// the PDF file at the time* — loosened, because §14.4's own suggested computation was never a
/// function of the contents (it names the time, the location and the size). Both strikes are
/// under the four-word floor `spec-errata check` compares at, which is how a gated blockquote
/// sat on retired words; found by the errata selection rule's ninth use.
///
/// So the first element is carried through unchanged and the second is derived from the bytes
/// this update is appending to — a choice the amended sentence still contains, since the bytes
/// are the file at the time it was last updated, and one that keeps the answer the same every
/// time the same edit is saved. That determinism is deliberate: rule 3 says this crate has no
/// clock, and a test that saves twice and compares is worth more than a timestamp. The digest
/// is this writer's own choice too — §14.4 suggested MD5 by name until Issue #691 struck the
/// example, and uniqueness rather than collision resistance is all the entry asks of it.
///
/// **It survives only in an unencrypted file.** §7.6.3.1 requires a fresh random initialisation
/// vector in front of every AES string and stream, so an encrypted document's update differs from
/// one save to the next by construction, and the second identifier — a digest of the bytes so
/// far — differs with it. That is the clause's requirement rather than this function's choice,
/// and the tests for an encrypted save read the file back instead of comparing it.
fn identify(document: &crate::Document, trailer: &mut Dictionary, so_far: &[u8]) {
    let permanent = document
        .trailer()
        .get("ID")
        .and_then(Object::as_array)
        .and_then(<[Object]>::first)
        .cloned();
    let digest = <md5::Md5 as md5::Digest>::digest(so_far);
    let changing = Object::String(digest.to_vec().into());
    trailer.insert(
        Name::new(&b"ID"[..]),
        Object::Array(vec![
            permanent.unwrap_or_else(|| changing.clone()),
            changing,
        ]),
    );
}

/// The trailer entries an update must repeat because a reader stops at the first section.
///
/// §7.5.5 makes `/Encrypt` and `/Info` properties of the *file*, and a reader that took the
/// newest trailer as the whole answer would find neither. `/Root` and `/Size` are written by the
/// caller because they are computed rather than copied.
fn carry_forward(document: &crate::Document, trailer: &mut Dictionary) {
    for key in ["Encrypt", "Info"] {
        if let Some(value) = document.trailer().get(key) {
            trailer.insert(Name::new(key.as_bytes()), value.clone());
        }
    }
}

/// Consecutive object numbers, which is what a cross-reference subsection is — and what
/// §7.5.8's `/Index` pairs describe.
fn runs(entries: &[Entry]) -> Vec<&[Entry]> {
    let mut out: Vec<&[Entry]> = Vec::new();
    let mut start = 0;
    for index in 1..entries.len() {
        if entries[index].number != entries[index.saturating_sub(1)].number.saturating_add(1) {
            out.push(&entries[start..index]);
            start = index;
        }
    }
    if start < entries.len() {
        out.push(&entries[start..]);
    }
    out
}

/// A count as the integer a dictionary holds, saturating rather than wrapping.
fn as_integer(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// The offset §7.5.5's `startxref` states, read from the end of the file.
///
/// > The last line of the file shall contain only the end-of-file marker, %%EOF. The two
/// > preceding lines shall contain, one per line and in order, the keyword startxref and the byte
/// > offset in the decoded stream from the beginning of the PDF file to the beginning of the xref
/// > keyword in the last cross-reference section.
///
/// Searched from the end rather than parsed from a fixed position, because a file with trailing
/// bytes after `%%EOF` is common and the clause's "last line" is then not the last line.
///
/// **Errata Collection 3 has edited that sentence in three places and the quotation above is the
/// 2020 text**, because the sponsored copy records EC3 as annotations and `doc/md/` dropped every
/// annotation in all fourteen documents — found in the four-hundred-and-sixteenth session by
/// `tools/spec-errata`, ADR 0252. Issue #101, `/State` `Completed`: "in the decoded stream" is
/// struck out, "section" replaces "stream" at the end, and the sentence gains "or the beginning
/// of the previous cross-reference stream (see 7.5.8, "Cross-reference streams")". The last of
/// those is the one with teeth — the offset may name a cross-reference *stream* rather than the
/// `xref` keyword — and `xref::read_at` has read both since long before this was noticed, which
/// is the argument for keeping the note rather than a reason not to write it down.
fn startxref(bytes: &crate::FileBytes) -> Option<usize> {
    let tail_from = bytes.len().saturating_sub(2048);
    let tail = bytes.read(tail_from..bytes.len());
    let at = tail.windows(9).rposition(|window| window == b"startxref")?;
    let after = tail.get(at.saturating_add(9)..)?;
    let digits: Vec<u8> = after
        .iter()
        .skip_while(|byte| byte.is_ascii_whitespace())
        .take_while(|byte| byte.is_ascii_digit())
        .copied()
        .collect();
    std::str::from_utf8(&digits).ok()?.parse().ok()
}
