//! A whole file written from objects other documents already hold — ISO 32000-2 §7.5's
//! structure on the way out.
//!
//! # What this is for, and what it is not
//!
//! [`crate::write`] is §7.5.6's incremental update: bytes appended to a file a person is
//! editing in place. This is the other writer, and `CLAUDE.md`'s exclusion was redrawn for it
//! on 2026-09-03 (RFC 0002 section 11.1, ADR 0816). What that entry now says, in `CLAUDE.md`'s
//! words rather than the standard's: assembling documents from existing documents is in scope,
//! because splitting, merging, reordering, rotating, extracting and optimising operate on
//! content some producer already specified, and every content stream in their output is a
//! producer's, carried byte for byte or recompressed without reinterpretation.
//!
//! So this module **emits structure and never content**. It writes §7.5.2's header, a body of
//! §7.5.3 indirect objects, §7.5.4's cross-reference table or §7.5.8's cross-reference stream,
//! §7.5.5's trailer and §14.4's identifiers. It decides *nothing* about what is on a page: a
//! content stream crosses from a source document to the output as the bytes the source holds,
//! still encoded by whatever filter its producer chose, with only its `/Length` re-derived from
//! the bytes actually written.
//!
//! # The two halves
//!
//! [`Assembly`] is the object table being built: the caller names objects to copy from
//! immutable source [`Document`]s and objects it synthesised itself, and gets the output's own
//! numbering back as it goes. [`serialize`] turns a finished assembly into bytes.
//!
//! Renumbering is **total and sequential**: no attempt is made to preserve a source's object
//! numbers, because two sources may use the same one. Every emitted object has generation 0.
//!
//! # What it refuses, and what it repairs
//!
//! It refuses by name — [`SerializeError`] — an assembly with no `/Root`, a reserved slot the
//! caller never filled, more objects than a cross-reference section can number, and an offset
//! larger than §7.5.4's ten digits can state.
//!
//! It repairs two things silently-but-counted, because both are §7.3's own answer rather than a
//! tolerance:
//!
//! - **A reference to an object the assembly does not hold becomes `null`.** §7.3.10: "[a]n
//!   indirect reference to an undefined object shall not be considered an error by a PDF
//!   processor; it shall be treated as a reference to the null object." A transform that copies
//!   a page's closure and stops at the pieces' edges *makes* such references, deliberately, and
//!   [`Written::dangling`] counts every one so the caller can name them.
//! - **A stream's `/Length` is re-derived from the bytes written.** §7.3.8.2 makes `/Length`
//!   "[t]he number of bytes from the beginning of the line following the keyword stream to the
//!   last byte, just before the keyword endstream", which is a statement about the output rather
//!   than about the source; a source whose `/Length` lied would otherwise have its lie copied
//!   into a file this program wrote. [`Written::relengthed`] counts the disagreements.
//!
//! # Encryption
//!
//! **The output is not encrypted.** Every object handed over here is plaintext — a
//! [`Document`] decrypts on load — so a derivative of an encrypted source is a derivative
//! without §7.6's protection, and that is a fact about the file the *caller* must report to
//! whoever asked for it ([`Assembly::has_encrypted_source`] is what it asks). Encrypting on the
//! way out is what §7.6's writer-side algorithms would be for, and this serializer emits no
//! `/Encrypt`, which is why those ledger rows stay writer-side.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write;

use crate::object::{Dictionary, Name, Object, ObjectId, Stream};
use crate::version::Version;
use crate::{Document, write};

/// The deepest an object's value tree is walked when its references are renumbered.
///
/// One more than [`crate::Limits::DEFAULT`]'s `max_depth`, so that every object a parser
/// admitted can be rewritten, and a synthesised object nested deeper than any parsed one could
/// be has its tail replaced by `null` rather than overflowing the stack.
const MAX_REWRITE_DEPTH: usize = 257;

/// §7.5.4's generation for the free head of the list: "shall never be reused".
const FREE_FOREVER: u16 = 65_535;

/// The largest offset §7.5.4's ten-digit field can state.
const MAX_TABLE_OFFSET: u64 = 9_999_999_999;

/// Which cross-reference structure the output uses.
///
/// **The kind the sources already use**, by default: ADR 0121's argument for an incremental
/// update, promoted from a section to a whole file. Nothing in the standard requires a file's
/// sections to match — §7.5.8.4 exists precisely so that a 1.4 reader can find a 1.5 file's
/// table — but a file written in the form its own producer chose is a file whose next reader
/// has one thing to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// §7.5.4's classic table and §7.5.5's `trailer` keyword.
    Table,
    /// §7.5.8's cross-reference stream, which is itself an indirect object.
    Stream,
}

impl Form {
    /// The form a document's own last cross-reference section uses.
    ///
    /// A cross-reference stream's dictionary *is* the trailer (§7.5.8.2), and its `/Type` is
    /// `/XRef`; a classic table's trailer has no `/Type` at all.
    #[must_use]
    pub fn of(document: &Document) -> Self {
        if document
            .trailer()
            .get("Type")
            .and_then(Object::as_name)
            .is_some_and(|name| name.as_bytes() == b"XRef")
        {
            Self::Stream
        } else {
            Self::Table
        }
    }

    /// The form for a set of sources: [`Self::Stream`] only where **every** one of them uses it.
    ///
    /// The conservative direction on purpose. A classic table is readable by every version of
    /// every reader, and §7.5.8.1 makes a cross-reference stream a PDF 1.5 construct; a merge of
    /// a 1.4 file into a 1.7 one that came out as a stream would have raised the output's
    /// version requirement for a reason nobody asked for.
    #[must_use]
    pub fn of_all<'a>(documents: impl IntoIterator<Item = &'a Document>) -> Self {
        let mut any = false;
        for document in documents {
            any = true;
            if Self::of(document) == Self::Table {
                return Self::Table;
            }
        }
        if any { Self::Stream } else { Self::Table }
    }
}

/// Why an assembly could not take what it was handed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AssemblyError {
    /// The caller named a source it did not supply.
    #[error("this assembly holds {count} sources, and object {id:?} was asked for from {at}")]
    NoSuchSource {
        /// The index asked for.
        at: usize,
        /// How many there are.
        count: usize,
        /// The object asked for.
        id: ObjectId,
    },
    /// A slot was filled that this assembly never reserved, or that is a copied object's.
    #[error("object {} is not a reserved slot of this assembly", .id.number)]
    NotReserved {
        /// The number named.
        id: ObjectId,
    },
    /// A reserved slot was filled twice.
    #[error("object {} was already placed", .id.number)]
    AlreadyPlaced {
        /// The number named.
        id: ObjectId,
    },
    /// More objects than a cross-reference section can number.
    ///
    /// §7.5.4's entry states the object number implicitly, by position under a subsection
    /// header of two integers, and Table 15's `/Size` is "[t]he total number of entries in the
    /// PDF file's cross-reference table"; the number itself is bounded here by `u32`, which is
    /// what [`ObjectId`] holds.
    #[error("an assembly cannot hold more than {} objects", u32::MAX)]
    TooManyObjects,
}

/// Why a finished assembly could not be written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SerializeError {
    /// No object was named as the output's catalog.
    ///
    /// §7.5.5's Table 15 makes `/Root` required — "[t]he catalog dictionary for the PDF file"
    /// — and a trailer without it is the one thing `Document::open` refuses outright, so
    /// writing such a file would be writing a file this program cannot read.
    #[error("this assembly names no /Root, which §7.5.5 requires of a trailer")]
    NoRoot,
    /// A slot was reserved and never filled.
    ///
    /// Refused rather than written as `null`, because it is a caller's mistake rather than a
    /// document's: a reserved number was promised to whatever referred to it.
    #[error("object {} was reserved and never placed", .id.number)]
    Unplaced {
        /// The number reserved.
        id: ObjectId,
    },
    /// An object landed further into the file than §7.5.4's ten-digit offset field can state.
    #[error("an object at offset {offset} cannot be stated in §7.5.4's ten digits")]
    OffsetTooLarge {
        /// Where it landed.
        offset: u64,
    },
    /// The sink refused the bytes.
    #[error("writing the output: {0}")]
    Write(#[from] std::io::Error),
}

/// One entry of the output's object table.
#[derive(Debug, Clone)]
enum Slot {
    /// Copied by reference from `sources[from]`.
    Copied {
        /// Which source.
        from: usize,
        /// Its number there.
        id: ObjectId,
    },
    /// Reserved by the caller, and filled or not.
    Synthesised(Option<Object>),
}

/// The output's object table, being built.
///
/// Objects are added in the order they will be written and numbered from 1 in that order, so a
/// caller that builds the same piece from the same sources twice gets the same file — RFC 0002
/// §9's byte determinism, with no flag and no clock.
///
/// # The two ways in
///
/// [`Assembly::copy`] takes an object *by reference* out of an immutable source: nothing is
/// read, cloned or decoded until [`serialize`] walks it. It is idempotent, so a closure walk
/// that arrives at one object by two paths numbers it once. [`Assembly::add`] and
/// [`Assembly::reserve`] are for objects the caller synthesised — a new page tree, a new
/// catalog — whose references are in the *output's* numbering, which is why `reserve` exists:
/// a catalog naming a page tree that names the catalog needs one of the two numbers before
/// either object can be built.
#[derive(Debug)]
pub struct Assembly<'a> {
    /// The documents objects may be copied out of.
    sources: Vec<&'a Document>,
    /// Every slot, in output order; slot `n` is object number `n + 1`.
    slots: Vec<Slot>,
    /// Where a source's object landed, so that a second `copy` of it answers the first.
    placed: BTreeMap<(usize, ObjectId), ObjectId>,
    /// The output's catalog.
    root: Option<ObjectId>,
    /// The output's §14.3.3 document information dictionary, where the caller carries one.
    info: Option<ObjectId>,
}

impl<'a> Assembly<'a> {
    /// An empty assembly over these sources.
    #[must_use]
    pub fn new(sources: Vec<&'a Document>) -> Self {
        Self {
            sources,
            slots: Vec::new(),
            placed: BTreeMap::new(),
            root: None,
            info: None,
        }
    }

    /// How many objects the output will hold, not counting the free head.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether nothing has been added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The sources, in the order [`Assembly::new`] was given them.
    #[must_use]
    pub fn sources(&self) -> &[&'a Document] {
        &self.sources
    }

    /// Whether any source is encrypted, which decides whether the output loses §7.6's protection.
    ///
    /// The serializer writes no `/Encrypt`, so a derivative of an encrypted document is
    /// unencrypted. That is a fact about the file its caller owes whoever asked for it, and
    /// this is the question to ask.
    #[must_use]
    pub fn has_encrypted_source(&self) -> bool {
        self.sources.iter().any(|document| document.is_encrypted())
    }

    /// Takes one object out of a source, by reference, and answers its number in the output.
    ///
    /// Idempotent: asking twice answers the first number, which is what lets a caller walk a
    /// closure without keeping a visited set of its own.
    ///
    /// # Errors
    ///
    /// [`AssemblyError::NoSuchSource`] where `from` names no source, and
    /// [`AssemblyError::TooManyObjects`] past `u32::MAX` objects.
    pub fn copy(&mut self, from: usize, id: ObjectId) -> Result<ObjectId, AssemblyError> {
        let count = self.sources.len();
        if from >= count {
            return Err(AssemblyError::NoSuchSource {
                at: from,
                count,
                id,
            });
        }
        if let Some(already) = self.placed.get(&(from, id)) {
            return Ok(*already);
        }
        let placed = self.push(Slot::Copied { from, id })?;
        self.placed.insert((from, id), placed);
        Ok(placed)
    }

    /// The number a source's object was given, where it has been copied.
    #[must_use]
    pub fn copied(&self, from: usize, id: ObjectId) -> Option<ObjectId> {
        self.placed.get(&(from, id)).copied()
    }

    /// Reserves a slot that **stands in for** a source object, and answers its number.
    ///
    /// The third of RFC 0002 section 10's three inputs — "(&[&Document], object-selection,
    /// replacements)" — and what a transform needs whenever an object crosses *changed*: a page
    /// whose `/Parent` must name the new tree, a node whose `/Kids` are a subset. Every
    /// reference to the source object, from anywhere in the assembly, maps to this slot, so the
    /// rest of the closure does not have to know that the object was replaced; the caller then
    /// fills it with [`Assembly::place`], in the **output's** numbering.
    ///
    /// # Errors
    ///
    /// [`AssemblyError::NoSuchSource`] where `from` names no source,
    /// [`AssemblyError::AlreadyPlaced`] where the object has already been copied or replaced,
    /// and [`AssemblyError::TooManyObjects`] past `u32::MAX` objects.
    pub fn replace(&mut self, from: usize, id: ObjectId) -> Result<ObjectId, AssemblyError> {
        let count = self.sources.len();
        if from >= count {
            return Err(AssemblyError::NoSuchSource {
                at: from,
                count,
                id,
            });
        }
        if let Some(already) = self.placed.get(&(from, id)) {
            return Err(AssemblyError::AlreadyPlaced { id: *already });
        }
        let placed = self.push(Slot::Synthesised(None))?;
        self.placed.insert((from, id), placed);
        Ok(placed)
    }

    /// Adds an object the caller built, whose references are in the output's numbering.
    ///
    /// # Errors
    ///
    /// [`AssemblyError::TooManyObjects`].
    pub fn add(&mut self, object: Object) -> Result<ObjectId, AssemblyError> {
        self.push(Slot::Synthesised(Some(object)))
    }

    /// Reserves a number for an object the caller will build later.
    ///
    /// # Errors
    ///
    /// [`AssemblyError::TooManyObjects`].
    pub fn reserve(&mut self) -> Result<ObjectId, AssemblyError> {
        self.push(Slot::Synthesised(None))
    }

    /// Fills a slot [`Assembly::reserve`] handed out.
    ///
    /// # Errors
    ///
    /// [`AssemblyError::NotReserved`] where the number is not a reserved slot's — a copied
    /// object's number included, since a copied object's bytes are the source's — and
    /// [`AssemblyError::AlreadyPlaced`] where it has already been filled.
    pub fn place(&mut self, id: ObjectId, object: Object) -> Result<(), AssemblyError> {
        let index = usize::try_from(id.number)
            .ok()
            .and_then(|number| number.checked_sub(1))
            .ok_or(AssemblyError::NotReserved { id })?;
        match self.slots.get_mut(index) {
            Some(Slot::Synthesised(slot @ None)) => {
                *slot = Some(object);
                Ok(())
            }
            Some(Slot::Synthesised(Some(_))) => Err(AssemblyError::AlreadyPlaced { id }),
            Some(Slot::Copied { .. }) | None => Err(AssemblyError::NotReserved { id }),
        }
    }

    /// Names the output's catalog. §7.5.5's Table 15 requires one.
    pub fn set_root(&mut self, id: ObjectId) {
        self.root = Some(id);
    }

    /// Names the output's §14.3.3 document information dictionary, where one is carried.
    pub fn set_info(&mut self, id: Option<ObjectId>) {
        self.info = id;
    }

    /// Appends a slot and answers its number.
    fn push(&mut self, slot: Slot) -> Result<ObjectId, AssemblyError> {
        let number = u32::try_from(self.slots.len().saturating_add(1))
            .map_err(|_| AssemblyError::TooManyObjects)?;
        if number == u32::MAX {
            return Err(AssemblyError::TooManyObjects);
        }
        self.slots.push(slot);
        Ok(ObjectId::new(number, 0))
    }

    /// The object slot `index` holds, as it will be written, with its references renumbered.
    ///
    /// `None` for a reserved slot nobody filled, which [`serialize`] turns into a refusal.
    fn resolved(&self, index: usize, tally: &mut Written) -> Option<Object> {
        match self.slots.get(index)? {
            Slot::Copied { from, id } => {
                let document = self.sources.get(*from)?;
                Some(self.renumber(&document.get(*id), *from, 0, tally))
            }
            Slot::Synthesised(object) => object.clone(),
        }
    }

    /// One value with every reference mapped into the output's numbering.
    ///
    /// A reference the assembly does not hold becomes `null`, per §7.3.10, and is counted. A
    /// synthesised object's references are already the output's, and mapping them again would
    /// be wrong — so this is applied only to copied objects, and their references are the
    /// source's by construction.
    fn renumber(&self, value: &Object, from: usize, depth: usize, tally: &mut Written) -> Object {
        if depth >= MAX_REWRITE_DEPTH {
            return Object::Null;
        }
        match value {
            Object::Reference(id) => {
                if let Some(mapped) = self.placed.get(&(from, *id)) {
                    Object::Reference(*mapped)
                } else {
                    tally.dangling = tally.dangling.saturating_add(1);
                    Object::Null
                }
            }
            Object::Array(items) => Object::Array(
                items
                    .iter()
                    .map(|item| self.renumber(item, from, depth.saturating_add(1), tally))
                    .collect(),
            ),
            Object::Dictionary(dict) => {
                Object::Dictionary(self.renumber_dictionary(dict, from, depth, tally))
            }
            Object::Stream(stream) => Object::Stream(std::sync::Arc::new(Stream {
                dict: self.renumber_dictionary(&stream.dict, from, depth, tally),
                data: std::sync::Arc::clone(&stream.data),
                decryption_failed: stream.decryption_failed,
            })),
            other => other.clone(),
        }
    }

    /// [`Self::renumber`] over a dictionary's values.
    fn renumber_dictionary(
        &self,
        dict: &Dictionary,
        from: usize,
        depth: usize,
        tally: &mut Written,
    ) -> Dictionary {
        let mut out = Dictionary::new();
        for (key, value) in dict.iter() {
            out.insert(
                key.clone(),
                self.renumber(value, from, depth.saturating_add(1), tally),
            );
        }
        out
    }
}

/// What one call to [`serialize`] produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Written {
    /// How many bytes the whole file is.
    pub bytes: u64,
    /// How many indirect objects were written, not counting the free head of the table.
    pub objects: u32,
    /// How many references named an object the assembly does not hold and became `null`.
    ///
    /// §7.3.10's answer, counted rather than hidden: a transform that stops a closure walk at
    /// a piece's edge produces these deliberately and owes its caller the number.
    pub dangling: u64,
    /// How many streams stated a `/Length` other than the bytes written, and were corrected.
    ///
    /// §7.3.8.2's number is a statement about the file being written; a source that lied about
    /// it does not get the lie copied forward.
    pub relengthed: u64,
}

/// Writes a finished assembly as a whole PDF file.
///
/// The order is §7.5.1's: header, body, cross-reference section, trailer. Nothing is buffered
/// whole — each object goes to `out` as it is built, and a stream's data crosses from the
/// source's `Arc<[u8]>` to the sink without being decoded, copied or examined.
///
/// `version` is the header's, and it is the caller's to decide: it knows which of the sources'
/// constructs survived into the output. [`Form::Stream`] raises it to 1.5 where it is lower,
/// because §7.5.8.1 introduces the construct at that version and a file whose header disowned
/// its own cross-reference section would be one no §7.5.8.4 reader could recover.
///
/// # Errors
///
/// [`SerializeError`]: no `/Root`, a reserved slot never filled, an offset past §7.5.4's ten
/// digits, or a sink that refused the bytes.
pub fn serialize<W: Write>(
    assembly: &Assembly<'_>,
    version: Version,
    form: Form,
    out: &mut W,
) -> Result<Written, SerializeError> {
    let root = assembly.root.ok_or(SerializeError::NoRoot)?;
    let version = match form {
        Form::Stream => version.max(Version { major: 1, minor: 5 }),
        Form::Table => version,
    };

    let mut sink = Counted {
        out,
        at: 0,
        digest: <md5::Md5 as md5::Digest>::new(),
    };
    let mut tally = Written::default();
    // §7.5.2, and both of its lines. "The PDF file begins with the 5 characters '%PDF-'"; and
    // "[i]f a PDF file contains binary data, as most do, the header line shall be immediately
    // followed by a comment line containing at least four binary characters — that is,
    // characters whose codes are 128 or greater." Every output of this serializer carries
    // stream data, so the second line is written unconditionally rather than after a scan.
    let mut header = String::new();
    let _ = writeln!(header, "%PDF-{version}");
    sink.put(header.as_bytes())?;
    sink.put(b"%\xE2\xE3\xCF\xD3\n")?;

    // Object *n* is slot *n - 1*, and it is written where the loop reaches it: the file's
    // order is the assembly's, which is what makes the output a function of the plan.
    let mut offsets: Vec<u64> = Vec::with_capacity(assembly.slots.len());
    let mut buffer: Vec<u8> = Vec::new();
    for index in 0..assembly.slots.len() {
        let number = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
        let id = ObjectId::new(number, 0);
        let value = assembly
            .resolved(index, &mut tally)
            .ok_or(SerializeError::Unplaced { id })?;
        offsets.push(sink.at);

        buffer.clear();
        let _ = writeln!(HexSink(&mut buffer), "{number} 0 obj");
        match &value {
            Object::Stream(stream) => {
                let mut dict = stream.dict.clone();
                let stated = dict.get("Length").and_then(Object::as_integer);
                let actual = i64::try_from(stream.data.len()).unwrap_or(i64::MAX);
                if stated != Some(actual) {
                    tally.relengthed = tally.relengthed.saturating_add(1);
                }
                dict.insert(Name::new(&b"Length"[..]), Object::Integer(actual));
                write::object(&Object::Dictionary(dict), &mut buffer);
                buffer.extend_from_slice(b"\nstream\n");
                sink.put(&buffer)?;
                // The one place a whole stream crosses, and it crosses encoded: the source's
                // bytes as its producer filtered them, never decoded and never re-encoded.
                sink.put(&stream.data)?;
                sink.put(b"\nendstream\nendobj\n")?;
            }
            other => {
                write::object(other, &mut buffer);
                buffer.extend_from_slice(b"\nendobj\n");
                sink.put(&buffer)?;
            }
        }
        tally.objects = number;
    }

    let start = sink.at;
    let size = u64::from(tally.objects).saturating_add(1);
    match form {
        Form::Table => {
            cross_reference_table(&mut sink, &offsets, size, root, assembly.info)?;
        }
        Form::Stream => {
            cross_reference_stream(&mut sink, &offsets, size, root, assembly.info)?;
        }
    }
    let mut tail = String::new();
    let _ = write!(tail, "startxref\n{start}\n%%EOF\n");
    sink.put(tail.as_bytes())?;

    tally.bytes = sink.at;
    Ok(tally)
}

/// §7.5.4's classic table, then §7.5.5's `trailer` keyword and dictionary.
fn cross_reference_table<W: Write>(
    sink: &mut Counted<'_, W>,
    offsets: &[u64],
    size: u64,
    root: ObjectId,
    info: Option<ObjectId>,
) -> Result<(), SerializeError> {
    let mut text = String::from("xref\n");
    // One subsection covering every number, because a whole file has no gaps: the objects are
    // numbered 1..=n by construction, and object 0 is the head of §7.5.4's free list, which
    // "shall always be free" and whose generation "shall be 65535".
    let _ = writeln!(text, "0 {size}");
    let _ = writeln!(text, "{:010} {FREE_FOREVER:05} f ", 0);
    for offset in offsets {
        if *offset > MAX_TABLE_OFFSET {
            return Err(SerializeError::OffsetTooLarge { offset: *offset });
        }
        // "Each cross-reference entry shall be exactly 20 bytes long": ten digits of offset, a
        // space, five of generation, a space, the type, and a two-byte end-of-line sequence.
        let _ = writeln!(text, "{offset:010} 00000 n ");
    }
    sink.put(text.as_bytes())?;
    sink.put(b"trailer\n")?;
    let trailer = trailer_dictionary(size, root, info, sink.digest());
    let mut buffer = Vec::new();
    write::object(&Object::Dictionary(trailer), &mut buffer);
    buffer.push(b'\n');
    sink.put(&buffer)
}

/// §7.5.8's cross-reference stream: an indirect object whose dictionary is the trailer.
fn cross_reference_stream<W: Write>(
    sink: &mut Counted<'_, W>,
    offsets: &[u64],
    size: u64,
    root: ObjectId,
    info: Option<ObjectId>,
) -> Result<(), SerializeError> {
    // The stream is an object and its own entry has to be in it, so it takes the next number
    // and its offset is where it is about to be written.
    let number = u32::try_from(size).unwrap_or(u32::MAX);
    let at = sink.at;

    // `/W [1 4 2]`, Table 18's three fields at their widest useful sizes: the type, a
    // four-byte offset, a two-byte generation. Type 0 is a free object, whose second field is
    // "the object number of the next free object" and whose third is "the generation number to
    // use if this object number is used again".
    let mut data = Vec::with_capacity(offsets.len().saturating_add(1).saturating_mul(7));
    data.push(0);
    data.extend_from_slice(&0_u32.to_be_bytes());
    data.extend_from_slice(&FREE_FOREVER.to_be_bytes());
    for offset in offsets {
        data.push(1);
        data.extend_from_slice(&u32::try_from(*offset).unwrap_or(u32::MAX).to_be_bytes());
        data.extend_from_slice(&0_u16.to_be_bytes());
    }
    if offsets.iter().any(|offset| *offset > u64::from(u32::MAX)) {
        return Err(SerializeError::OffsetTooLarge {
            offset: offsets.iter().copied().max().unwrap_or_default(),
        });
    }
    data.push(1);
    data.extend_from_slice(&u32::try_from(at).unwrap_or(u32::MAX).to_be_bytes());
    data.extend_from_slice(&0_u16.to_be_bytes());

    let mut dict = trailer_dictionary(size.saturating_add(1), root, info, sink.digest());
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"XRef"[..])),
    );
    dict.insert(
        Name::new(&b"W"[..]),
        Object::Array(vec![
            Object::Integer(1),
            Object::Integer(4),
            Object::Integer(2),
        ]),
    );
    // One subsection, 0 to the stream's own number inclusive.
    dict.insert(
        Name::new(&b"Index"[..]),
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(i64::from(number).saturating_add(1)),
        ]),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
    );

    let mut buffer = Vec::new();
    let _ = writeln!(HexSink(&mut buffer), "{number} 0 obj");
    write::object(
        &Object::Stream(std::sync::Arc::new(Stream {
            dict,
            data: data.into(),
            decryption_failed: false,
        })),
        &mut buffer,
    );
    buffer.extend_from_slice(b"\nendobj\n");
    sink.put(&buffer)
}

/// §7.5.5's Table 15, for a file with exactly one cross-reference section.
///
/// `/Size` and `/Root` are required; `/Prev` is not written because there is nothing before
/// this section; `/Encrypt` is not written because [`serialize`] emits no encryption; `/Info`
/// is written where the caller carried one.
fn trailer_dictionary(
    size: u64,
    root: ObjectId,
    info: Option<ObjectId>,
    digest: [u8; 16],
) -> Dictionary {
    let mut trailer = Dictionary::new();
    trailer.insert(
        Name::new(&b"Size"[..]),
        Object::Integer(i64::try_from(size).unwrap_or(i64::MAX)),
    );
    trailer.insert(Name::new(&b"Root"[..]), Object::Reference(root));
    if let Some(info) = info {
        trailer.insert(Name::new(&b"Info"[..]), Object::Reference(info));
    }
    identify(&mut trailer, digest);
    trailer
}

/// §14.4's file identifier, for a file that is being *created* rather than updated.
///
/// > When a PDF file is first written, both identifiers shall be set to the same value.
///
/// A derived file is written for the first time here, so that sentence is the whole rule and
/// both elements are one value. Neither is the source's: two pieces of one document are two
/// files, and a permanent identifier they shared would say they were versions of each other.
///
/// The clause's first element "shall be a permanent identifier based on the PDF file at the
/// time it was originally created" and the second one "based on the PDF file at the time it was
/// last updated" — Errata Collection 3's Issue #328 struck *contents of the* and *'s contents*
/// out of the two sentences, so neither is a claim about what the file says. A later §7.5.6
/// update to one of these pieces carries the first element through unchanged, which
/// [`crate::write`] already does.
///
/// The value is a digest of the bytes written so far, which is §14.4's own kind of answer — its
/// suggested computation named the time, the location and the size — and is a function of the
/// output alone, so the same plan over the same sources produces the same file. That
/// determinism is deliberate and is RFC 0002 section 9's first layer: qpdf needs `--deterministic-id`
/// to promise it, and here it is the only behaviour. The digest is this writer's choice; §14.4
/// suggested MD5 by name until Issue #691 struck the example, and uniqueness rather than
/// collision resistance is all the entry asks of it.
fn identify(trailer: &mut Dictionary, digest: [u8; 16]) {
    let value = Object::String(digest.to_vec().into());
    trailer.insert(
        Name::new(&b"ID"[..]),
        Object::Array(vec![value.clone(), value]),
    );
}

/// A sink that counts what has gone through it and digests it as it goes.
///
/// The count is §7.5.4's offsets, and the digest is §14.4's identifier — both are functions of
/// the bytes, and taking them here is what keeps the whole file from having to be buffered to
/// compute either.
struct Counted<'a, W: Write> {
    /// Where the bytes go.
    out: &'a mut W,
    /// How many have gone.
    at: u64,
    /// A digest of every one of them, for §14.4.
    digest: md5::Md5,
}

impl<W: Write> Counted<'_, W> {
    /// Writes, counting and digesting.
    fn put(&mut self, bytes: &[u8]) -> Result<(), SerializeError> {
        self.out.write_all(bytes)?;
        self.at = self
            .at
            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        <md5::Md5 as md5::Digest>::update(&mut self.digest, bytes);
        Ok(())
    }

    /// §14.4's identifier for the file so far: a digest of every byte written before the
    /// trailer, taken as the bytes went past rather than by holding the file to hash it.
    fn digest(&self) -> [u8; 16] {
        <md5::Md5 as md5::Digest>::finalize(self.digest.clone()).into()
    }
}

/// A `fmt::Write` that appends to a byte vector.
struct HexSink<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for HexSink<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}
