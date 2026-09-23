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
//! It refuses by name — [`SerializeError`] — an assembly with no `/Root`, a `/Root` that does
//! not reach §7.5.5's catalog dictionary, a reserved slot the caller never filled, more objects
//! than a cross-reference section can number, and an offset larger than §7.5.4's ten digits can
//! state.
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
//! [`serialize`] writes a plaintext file and [`serialize_encrypted`] writes §7.6.4's standard
//! security handler at `/V` 5 and `/R` 6 — the one configuration §7.6.4.1 and Table 20 leave
//! undeprecated — over a file encryption key and salts a caller-supplied [`Entropy`] provides.
//!
//! **Which of the two happens is asked, never inherited.** Every object handed over here is
//! plaintext, because a [`Document`] decrypts on load, so a derivative of an encrypted source
//! is protected only if somebody says how: revision 6 stores each password as a one-way hash,
//! so nothing in an opened document yields the passwords a new file would need
//! ([`Protection`] says this at length, ADR 1161). [`Assembly::has_encrypted_source`] is how a
//! caller finds out that the question arises.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write;

use crate::crypt::{self, Encryption};
use crate::object::{Dictionary, Name, Object, ObjectId, Stream};
use crate::version::Version;
use crate::{Document, write};

pub use crate::crypt::{Entropy, SystemEntropy};

/// The deepest an object's value tree is walked when its references are renumbered.
///
/// One more than [`crate::Limits::DEFAULT`]'s `max_depth`, so that every object a parser
/// admitted can be rewritten, and a synthesised object nested deeper than any parsed one could
/// be has its tail replaced by `null` rather than overflowing the stack.
const MAX_REWRITE_DEPTH: usize = 257;

/// §7.5.4's generation for the free head of the list: "shall never be reused".
pub(crate) const FREE_FOREVER: u16 = 65_535;

/// The largest offset §7.5.4's ten-digit field can state.
pub(crate) const MAX_TABLE_OFFSET: u64 = 9_999_999_999;

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

/// Whether §7.5.7's object streams are generated, and how large one may grow.
///
/// > An object stream is a stream object in which a sequence of indirect objects may be
/// > stored, as an alternative to their being stored at the outermost PDF file level.
///
/// NOTE 1 says what the construct is for — "to allow indirect objects other than streams to be
/// stored more compactly by using the facilities provided by stream compression filters" — and
/// [`ObjectStreams::Disable`] is what every writer in this tree did until RFC 0002's `optimize`:
/// correct, and larger than the 1.5 source it was derived from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStreams {
    /// Every object at the outermost file level, as §7.5.3 writes them.
    Disable,
    /// Every object §7.5.7 permits is packed into a `FlateDecode`-encoded object stream.
    ///
    /// The clause's own prohibitions decide which objects those are; [`packable`] is the list,
    /// sentence by sentence.
    Generate {
        /// The most objects one object stream holds.
        ///
        /// NOTE 4 requires a limit and gives the reason rather than a number: "[t]o avoid a
        /// degradation of performance, such as would occur when downloading and decompressing a
        /// large object stream to access a single compressed object, the number of objects in
        /// an individual object stream needs to be limited." So the number is this writer's
        /// stated choice — [`ObjectStreams::DEFAULT`] says which and why.
        max_objects: usize,
        /// The most bytes of *decoded* payload one object stream holds before it is cut.
        ///
        /// The other half of the same sentence: what a reader downloads and decompresses to
        /// reach one member is measured in bytes, and a hundred long dictionaries are not the
        /// same download as a hundred short ones.
        max_bytes: usize,
    },
}

impl ObjectStreams {
    /// 200 objects or 64 KiB of decoded payload, whichever comes first.
    ///
    /// Both numbers are choices rather than readings, because NOTE 4 states the obligation and
    /// no figure. They were measured before being fixed, over every fifth document of the
    /// pdf.js corpus (22.9 MB of them): 50 objects and 16 KiB save 13.19% of the whole, these
    /// save 13.30%, and 500 objects and 256 KiB save 13.32%. The curve is flat where these sit
    /// — two hundredths of a point separates them from four times the ceiling — so the smaller
    /// pair is the one that honours NOTE 4's own reason for asking. ADR 0842.
    pub const DEFAULT: Self = Self::Generate {
        max_objects: 200,
        max_bytes: 64 * 1024,
    };

    /// The two ceilings, or `None` where no object stream is generated.
    fn ceilings(self) -> Option<(usize, usize)> {
        match self {
            Self::Disable => None,
            Self::Generate {
                max_objects,
                max_bytes,
            } => Some((max_objects.max(1), max_bytes.max(1))),
        }
    }
}

/// What happens to a stream's bytes on the way out.
///
/// `CLAUDE.md`'s amended exclusion states both arms in one sentence — "every content stream in
/// their output is a producer's, carried byte for byte **or recompressed without
/// reinterpretation**" — and the two are a caller's choice rather than this module's, because
/// only the caller knows whether it promised pass-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Streams {
    /// The source's bytes, still encoded by whatever filter its producer chose.
    ///
    /// The default, and what `split`, `merge` and `pages` ask for: nothing is decoded, nothing
    /// is examined, and the `Arc<[u8]>` crosses to the sink untouched.
    Carry,
    /// Decoded through the filters this tree reads and re-encoded as one `FlateDecode`, kept
    /// only where the result is smaller.
    ///
    /// **Not a reinterpretation**: the decoded bytes are identical, so every mark the producer
    /// specified is the same mark. What changes is §7.4's encoding of them, which is a statement
    /// about the file rather than about the page.
    ///
    /// **The cost is stated, because it breaks this module's one memory property.** A carried
    /// stream is never held: the source's `Arc` goes to the sink. A recompressed one is decoded
    /// into memory and deflated into memory, so the peak is two copies of the largest stream the
    /// output holds, bounded by the source document's own [`crate::Limits::max_stream_len`].
    Recompress {
        /// zlib's effort, clamped to 0..=9.
        level: u32,
    },
}

impl Streams {
    /// zlib's level 9.
    ///
    /// Measured rather than assumed, which RFC 0002 section 6.5 named as an open question
    /// ("default compression effort (zlib 9 vs `zopfli`-class — measure first, principle 2's
    /// rule)"). Over every fifth document of the pdf.js corpus, level 9 saves 13.30% of the
    /// whole against level 6's 12.60% — seven tenths of a point, on files whose reason for
    /// existing is to be smaller. `optimize` is not on a latency path, nothing waits for it
    /// the way a first page does, and `--compression-level` is there for a caller who
    /// disagrees. `zopfli` is a dependency nobody has argued in `doc/stack.md`. ADR 0842.
    pub const DEFAULT: Self = Self::Recompress { level: 9 };
}

/// How a whole file is written: the cross-reference form, and the two size decisions.
///
/// Separate from [`serialize`]'s arguments so that a caller adding a knob does not change every
/// call site, and so that the defaults — which are what `split`, `merge` and `pages` want — are
/// stated once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// §7.5.4's table or §7.5.8's stream.
    ///
    /// **[`ObjectStreams::Generate`] overrides this to [`Form::Stream`]**, and the clause is
    /// Table 18: a compressed object is reached through a type 2 entry, and type 2 entries
    /// exist only in a cross-reference stream. A classic table has no way to say where a
    /// compressed object is, so a file with both would be a file whose own table denied its
    /// objects. (§7.5.8.4's hybrid-reference file says it twice over, and this writer does not
    /// write one.)
    pub form: Form,
    /// Whether §7.5.7's object streams are generated.
    pub object_streams: ObjectStreams,
    /// What happens to a stream's bytes.
    pub streams: Streams,
}

impl Options {
    /// The pass-through defaults: this form, no object streams, every stream's bytes carried.
    ///
    /// What every verb but `optimize` asks for.
    #[must_use]
    pub fn new(form: Form) -> Self {
        Self {
            form,
            object_streams: ObjectStreams::Disable,
            streams: Streams::Carry,
        }
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
    /// The object named as the output's catalog is not one.
    ///
    /// §7.5.5's Table 15 gives `/Root` the type **dictionary** as well as requiring it — "[t]he
    /// catalog dictionary for the PDF file" — so a trailer naming an object the output writes as
    /// `null`, as a number, or as an array states a catalog the file does not hold. §7.3.10
    /// makes the other road to the same place: "[a]n indirect reference to an undefined object
    /// shall not be considered an error by a PDF processor; it shall be treated as a reference
    /// to the null object", so a `/Root` naming a number no slot carries reaches null too, and
    /// both are this one refusal.
    ///
    /// **Where it comes from is a *source* rather than a caller's mistake**, which is what
    /// separates it from [`SerializeError::NoRoot`]: a document recovered from damage can state
    /// a trailer whose `/Root` its own objects never reach, and copying that document object for
    /// object carries the trailer's claim into a file with nothing behind it. This project does
    /// not emit such a file (RFC 0002 section 11.3). ADR 1028.
    #[error("/Root names object {}, which is not §7.5.5 Table 15's catalog dictionary", .id.number)]
    RootNotADictionary {
        /// The number the trailer would have named.
        id: ObjectId,
    },
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
    /// §7.6.4's algorithms refused the passwords or the permissions they were handed.
    #[error("§7.6.4: the output could not be protected: {0}")]
    Protection(#[from] crate::error::SyntaxError),
    /// The caller's randomness source produced nothing.
    ///
    /// §7.6.4.4.7 step (a) asks for "16 random bytes of data using a strong random number
    /// generator" and §7.6.3.3 for a fresh initialisation vector per string and stream. There
    /// is no second-best answer to either, so a source that refuses stops the write.
    #[error("§7.6.4.4.7 needs a strong random number generator and the one supplied refused")]
    Entropy,
    /// The cipher refused a key or a body §7.6.3.3 admits.
    ///
    /// Not reachable from any document: the key is 32 bytes by construction and AES-CBC takes
    /// data of any length. It is a refusal rather than an `unwrap` because a writer that
    /// panicked on a dependency's edge would take the caller's process with it.
    #[error("§7.6.3.3's cipher refused a string or stream this writer built")]
    Cipher,
}

/// What §7.5.5's `/Root` finds at the end of its chain, in the file about to be written.
///
/// Three answers rather than two, because a slot the caller reserved and never filled is a
/// *caller's* mistake that [`SerializeError::Unplaced`] names by number, and reporting it as a
/// missing catalog would hide which object was owed.
#[derive(Debug, Clone, Copy)]
enum RootReach {
    /// A dictionary, which is Table 15's type for this entry.
    Dictionary,
    /// Something that is not one, or nothing at all.
    Elsewhere,
    /// A reserved slot with nothing in it yet.
    Unplaced,
}

/// One entry of the output's object table.
#[derive(Debug, Clone)]
pub(crate) enum Slot {
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

/// Table 22's access permissions, as a *writer* states them in `/P`.
///
/// The reading half is [`crate::Permissions`], which answers what a document granted whoever
/// opened it; this is the other direction, and the two differ in what they have to know. A
/// reader takes the flag word as it finds it. A writer has to produce one, and §7.6.4.2 fixes
/// most of its bits before any permission is chosen: Table 22 reserves positions 1 and 2 —
/// "Reserved. Must be zero (0)" — positions 7 and 8 — "Reserved. Must be 1" — and positions 13
/// to 32 the same way, and of position 10 it says "PDF writers shall always set this bit to 1
/// to ensure compatibility with PDF readers following earlier specifications".
///
/// So [`Access::flags`] states those eleven bits itself and this struct names only the seven a
/// caller decides. [`Access::ALL`] grants every one of them, which is what a file gets when a
/// caller wants a password on a document and no restrictions in it.
#[expect(
    clippy::struct_excessive_bools,
    reason = "Table 22 is a flag word of independent permissions, and naming each one is the \
              whole point — the same reason `crate::Permissions` carries them this way"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Access {
    /// Bit 3: print the document.
    pub print: bool,
    /// Bit 4: "[m]odify the contents of the document by operations other than those controlled
    /// by bits 6, 9, and 11."
    pub modify: bool,
    /// Bit 5: copy or otherwise extract text and graphics.
    pub copy: bool,
    /// Bit 6: add or modify text annotations and fill in interactive form fields.
    pub annotate: bool,
    /// Bit 9: fill in existing interactive form fields "even if bit 6 is clear".
    pub fill_forms: bool,
    /// Bit 11: assemble the document — insert, rotate or delete pages.
    pub assemble: bool,
    /// Bit 12: print "to a representation from which a faithful digital copy of the PDF content
    /// could be generated".
    pub print_faithfully: bool,
}

impl Access {
    /// The bits Table 22 fixes whatever a caller asks for: 7, 8, 10 and 13 to 32 set, 1 and 2
    /// clear.
    const RESERVED: u32 = 0xFFFF_F2C0;

    /// Every permission granted.
    ///
    /// The default a caller should want, and `CLAUDE.md` principle 3 is why it is spelled out
    /// rather than assumed: a restriction is the reader's to set, so a program that encrypts a
    /// file has no business withholding anything the person who asked for it did not withhold.
    pub const ALL: Self = Self {
        print: true,
        modify: true,
        copy: true,
        annotate: true,
        fill_forms: true,
        assemble: true,
        print_faithfully: true,
    };

    /// Table 22's flag word, for Table 21's `/P`.
    ///
    /// The NOTE under Table 21 describes the result: "[s]ince all the reserved high-order flag
    /// bits in the encryption dictionary's P value are required to be 1, the integer value P is
    /// always specified as a negative integer."
    #[must_use]
    pub const fn flags(self) -> i32 {
        let mut bits = Self::RESERVED;
        if self.print {
            bits |= 1 << 2;
        }
        if self.modify {
            bits |= 1 << 3;
        }
        if self.copy {
            bits |= 1 << 4;
        }
        if self.annotate {
            bits |= 1 << 5;
        }
        if self.fill_forms {
            bits |= 1 << 8;
        }
        if self.assemble {
            bits |= 1 << 10;
        }
        if self.print_faithfully {
            bits |= 1 << 11;
        }
        bits.cast_signed()
    }
}

/// What a caller supplies to have the output encrypted — §7.6.4's standard security handler.
///
/// **It is asked for, never inherited.** A derivative of an encrypted source is not re-encrypted
/// by carrying the source's dictionary forward, and revision 6 is the reason rather than a
/// convenience: §7.6.4.4.7 and §7.6.4.4.8 store each password only as an Algorithm 2.B hash of
/// it, so a program holding one password and the file encryption key can compute neither the
/// other password nor a new `/U` for the one it does hold. §7.6.4.4.6's Algorithm 7 — where an
/// owner password *does* unwrap the user password — belongs to the revisions §7.6.4.1 deprecates
/// in PDF 2.0, and writing one of those to make the inheritance possible would be choosing a
/// deprecated construct to avoid asking a question. So the protection of a derived file is the
/// caller's statement about the derived file. ADR 1161.
#[derive(Debug, Clone, Copy)]
pub struct Protection<'a> {
    /// §7.6.4.1's user password, which "should allow additional operations to be performed
    /// according to the user access permissions". The empty string is the default user
    /// password every reader tries first, so a document given one opens without a prompt and
    /// asserts [`Protection::access`] alone.
    pub user_password: &'a str,
    /// §7.6.4.1's owner password, whose holder "should allow full (owner) access to the
    /// document".
    pub owner_password: &'a str,
    /// Table 22's flags, as `/P`.
    pub access: Access,
    /// Table 21's `/EncryptMetadata`: whether §14.3.2's document-level metadata stream is
    /// encrypted with everything else. Default `true`, per the table.
    pub encrypt_metadata: bool,
}

impl Protection<'_> {
    /// A password on the document and nothing withheld from whoever supplies it.
    #[must_use]
    pub const fn owner_only(owner_password: &str) -> Protection<'_> {
        Protection {
            user_password: "",
            owner_password,
            access: Access::ALL,
            encrypt_metadata: true,
        }
    }
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
    pub(crate) slots: Vec<Slot>,
    /// Where a source's object landed, so that a second `copy` of it answers the first.
    placed: BTreeMap<(usize, ObjectId), ObjectId>,
    /// The output's catalog.
    pub(crate) root: Option<ObjectId>,
    /// The output's §14.3.3 document information dictionary, where the caller carries one.
    pub(crate) info: Option<ObjectId>,
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

    /// What the output's `/Root` reaches, following §7.3.10's chain as far as this crate's own
    /// reader would.
    ///
    /// The writer's half of §7.5.5's Table 15: the entry's type is `dictionary`, and a file
    /// whose `/Root` reaches anything else is one [`Document::catalog`] refuses — so writing it
    /// would be putting this program's name on a file this program cannot open (RFC 0002
    /// section 11.3).
    ///
    /// **A stream is not refused, and that is a reading rather than an oversight.** §7.3.8.1
    /// makes a stream "a dictionary followed by zero or more bytes", and [`Object::as_dict`] —
    /// which is what [`Document::catalog`] applies — answers with that dictionary. The predicate
    /// here is the one the reader uses, because the property being protected is re-readability.
    ///
    /// Nothing is renumbered or decoded on this walk: it reads an object's *kind*, which
    /// [`Assembly::renumber`] never changes, and follows a copied reference through
    /// [`Assembly::copied`] because a copied object's references are still the source's. ADR 1028.
    fn root_reaches(&self, root: ObjectId) -> RootReach {
        let mut current = root;
        for _ in 0..crate::document::MAX_REFERENCE_DEPTH {
            let Some(index) = usize::try_from(current.number)
                .ok()
                .and_then(|number| number.checked_sub(1))
            else {
                return RootReach::Elsewhere;
            };
            // `from` is `None` for a synthesised object, whose references are already the
            // output's numbering, and `Some` for a copied one, whose are the source's.
            let (value, from) = match self.slots.get(index) {
                None => return RootReach::Elsewhere,
                Some(Slot::Synthesised(None)) => return RootReach::Unplaced,
                Some(Slot::Synthesised(Some(object))) => (object.clone(), None),
                Some(Slot::Copied { from, id }) => {
                    let Some(document) = self.sources.get(*from) else {
                        return RootReach::Elsewhere;
                    };
                    (document.get(*id), Some(*from))
                }
            };
            match value {
                Object::Dictionary(_) | Object::Stream(_) => return RootReach::Dictionary,
                Object::Reference(next) => {
                    current = match from {
                        // §7.3.10 again: a reference the assembly does not carry is written as
                        // `null`, so a chain that leaves the output ends at null.
                        Some(source) => match self.copied(source, next) {
                            Some(mapped) => mapped,
                            None => return RootReach::Elsewhere,
                        },
                        None => next,
                    };
                }
                _ => return RootReach::Elsewhere,
            }
        }
        // A cycle, or a chain longer than the reader will follow: `Document::resolve` answers
        // null past this many hops, and null is not a catalog.
        RootReach::Elsewhere
    }

    /// The object slot `index` holds, as it will be written, with its references renumbered.
    ///
    /// `None` for a reserved slot nobody filled, which [`serialize`] turns into a refusal.
    pub(crate) fn resolved(
        &self,
        index: usize,
        streams: Streams,
        tally: &mut Written,
    ) -> Option<Object> {
        match self.slots.get(index)? {
            Slot::Copied { from, id } => {
                let document = self.sources.get(*from)?;
                let original = document.get(*id);
                let carried = self.renumber(&original, *from, 0, tally);
                // Recompression reads the **source's** stream rather than the renumbered copy,
                // because `/Filter` and `/DecodeParms` may themselves be indirect and only the
                // source document can resolve them. What it produces is put back into the
                // renumbered dictionary, whose other entries are already the output's.
                if let (Streams::Recompress { level }, Some(source), Some(renumbered)) =
                    (streams, original.as_stream(), carried.as_stream())
                    && let Some(better) = recompressed(document, source, renumbered, level, tally)
                {
                    return Some(better);
                }
                Some(carried)
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

    /// One of the output's own values, followed through its own numbering as a reader would.
    ///
    /// §7.3.10's chain, at the depth [`Document`] follows one: a dictionary entry this writer
    /// has to *read* — a stream's `/Type`, its `/Filter`, a `/Crypt` filter's `/Name` — may be
    /// stated indirectly, and the answer has to be the one a reader of this file will reach.
    ///
    /// Resolution is always [`Streams::Carry`], whatever the write's own policy: what is wanted
    /// is a name, and recompressing a stream to read one would be doing the expensive half of
    /// the write twice.
    fn through(&self, value: Option<&Object>) -> Object {
        let Some(value) = value else {
            return Object::Null;
        };
        let mut current = value.clone();
        for _ in 0..crate::document::MAX_REFERENCE_DEPTH {
            let Object::Reference(id) = current else {
                return current;
            };
            let Some(index) = usize::try_from(id.number)
                .ok()
                .and_then(|number| number.checked_sub(1))
            else {
                return Object::Null;
            };
            let mut ignored = Written::default();
            let Some(next) = self.resolved(index, Streams::Carry, &mut ignored) else {
                return Object::Null;
            };
            current = next;
        }
        Object::Null
    }

    /// A stream's filter chain, as names, out of the output's own dictionary.
    ///
    /// §7.4's `/Filter` is "a name or an array of names", and either may be indirect; this is
    /// `Document::filter_chain` asked of a file that is not written yet.
    fn filter_names(&self, dict: &Dictionary) -> Vec<Vec<u8>> {
        match self.through(dict.get("Filter")) {
            Object::Name(name) => vec![name.as_bytes().to_vec()],
            Object::Array(items) => items
                .iter()
                .filter_map(|item| {
                    self.through(Some(item))
                        .as_name()
                        .map(|name| name.as_bytes().to_vec())
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The `/DecodeParms` dictionary belonging to the filter at `index`.
    ///
    /// Table 5: one dictionary where one filter has parameters, "or an array of such
    /// dictionaries" where several do.
    fn parms_at(&self, dict: &Dictionary, index: usize) -> Option<Dictionary> {
        match self.through(dict.get("DecodeParms")) {
            Object::Dictionary(parms) if index == 0 => Some(parms),
            Object::Array(items) => self.through(items.get(index)).as_dict().cloned(),
            _ => None,
        }
    }

    /// [`Self::renumber`] over a dictionary's values, dropping the entries that became null.
    ///
    /// §7.3.7:
    ///
    /// > A dictionary entry whose value is null (see 7.3.9, "Null object") shall be treated the
    /// > same as if the entry does not exist.
    ///
    /// A copied dictionary can only hold a null here by having held a reference the assembly
    /// does not carry, since [`crate::parser`] already drops a *direct* null on the way in for
    /// this same sentence. Writing `/Outlines null` where the source wrote a reference to
    /// nothing would therefore state something this tree's own reader immediately discards —
    /// and the two spellings differing is exactly the shape a second pass over the output can
    /// see, which is why an idempotent `optimize` needs this and not only §7.3.10's tally.
    /// The reference is still counted as [`Written::dangling`]; what is dropped is the entry
    /// the clause says is not there.
    ///
    /// **An array's null is kept**, because an array's positions are its meaning and §7.3.7 is
    /// about dictionaries.
    fn renumber_dictionary(
        &self,
        dict: &Dictionary,
        from: usize,
        depth: usize,
        tally: &mut Written,
    ) -> Dictionary {
        let mut out = Dictionary::new();
        for (key, value) in dict.iter() {
            let value = self.renumber(value, from, depth.saturating_add(1), tally);
            if matches!(value, Object::Null) {
                continue;
            }
            out.insert(key.clone(), value);
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
    /// How many §7.5.7 object streams were generated.
    pub object_streams: u32,
    /// How many objects were written inside one, rather than at the outermost file level.
    pub compressed: u32,
    /// How many streams were decoded and re-encoded because the result was smaller.
    pub recompressed: u64,
    /// How many bytes those re-encodings saved, summed over the streams that took one.
    ///
    /// A stream whose re-encoding was not smaller is not counted here and is not counted as
    /// [`Written::recompressed`] either: it was carried, so it saved nothing and changed
    /// nothing.
    pub saved: u64,
    /// How many streams an encrypted output holds in the clear.
    ///
    /// Zero for an unencrypted file, where the question does not arise. In an encrypted one it
    /// counts the streams §7.6.6 leaves to a crypt filter this output does not carry — a
    /// `/Crypt` entry in a `/Filter` array naming `Identity`, or naming a filter that was a
    /// source's and is not this file's — and §14.3.2's metadata stream where the caller asked
    /// for [`Protection::encrypt_metadata`] false. Each is what a reader of the output will
    /// expect; counting them is how a caller can say which parts of a protected file are not.
    pub cleartext: u64,
}

/// The output's own security handler, while the file is being written.
///
/// It holds three things and they are three different kinds of thing: the handler that turns
/// plaintext into ciphertext, the dictionary the file has to carry so that a reader can build
/// the same handler back, and the source of the bytes neither of the other two can derive.
struct Protected<'e> {
    /// §7.6.3.3's Algorithm 1.A over the file encryption key, as `AESV3`.
    encryption: Encryption,
    /// The `/Encrypt` dictionary, written as its own indirect object and never encrypted.
    dictionary: Dictionary,
    /// §7.6.3.3's "16-byte random number" for every string and stream, from outside.
    entropy: &'e mut dyn Entropy,
}

impl<'e> Protected<'e> {
    /// Runs §7.6.4.4.7, §7.6.4.4.8 and §7.6.4.4.9 and builds the dictionary they fill.
    fn new(
        protection: &Protection<'_>,
        entropy: &'e mut dyn Entropy,
    ) -> Result<Self, SerializeError> {
        let flags = protection.access.flags();
        let mut random = crypt::Unpredictable {
            key: [0; 32],
            salts: [0; 32],
            filler: [0; 4],
        };
        // §7.6.4.4.7 step (a): "Generate 16 random bytes of data using a strong random number
        // generator", and Algorithm 9 step (a) sixteen more. The key is the writer's own choice
        // of thirty-two, and Algorithm 10 step (e)'s four are the filler nothing reads.
        if !entropy.fill(&mut random.key)
            || !entropy.fill(&mut random.salts)
            || !entropy.fill(&mut random.filler)
        {
            return Err(SerializeError::Entropy);
        }
        let computed = crypt::revision6(
            protection.user_password,
            protection.owner_password,
            flags,
            protection.encrypt_metadata,
            &random,
        )?;

        // Table 20 and Table 21, in the order the two tables print them. `/V` 5 is the only
        // value Table 20 does not deprecate — "Values less than 5 for the V entry are
        // deprecated in PDF 2.0" — and it "permits the specification of crypt filters with a
        // file encryption key length of 256 bits (32 bytes)"; `/R` 6 is the revision that goes
        // with it, and the one §7.6.4.1's deprecation sentence leaves out.
        let mut dictionary = Dictionary::new();
        let string = |bytes: &[u8]| Object::String(bytes.to_vec().into());
        dictionary.insert(
            Name::new(&b"Filter"[..]),
            Object::Name(Name::new(&b"Standard"[..])),
        );
        dictionary.insert(Name::new(&b"V"[..]), Object::Integer(5));
        dictionary.insert(Name::new(&b"R"[..]), Object::Integer(6));

        // Table 25's crypt filter dictionary for the one filter §7.6.4.1 permits a revision 6
        // handler to be limited to: "For revision 6, the filter CFM value shall be AESV3
        // (AES-256)", and the `/AuthEvent` the same sentence requires of it.
        let mut filter = Dictionary::new();
        filter.insert(
            Name::new(&b"CFM"[..]),
            Object::Name(Name::new(&b"AESV3"[..])),
        );
        filter.insert(
            Name::new(&b"AuthEvent"[..]),
            Object::Name(Name::new(&b"DocOpen"[..])),
        );
        // Table 25's `/Length`: the key length "in bits or bytes, depending on the
        // application", which for `AESV3` is the 256-bit key Table 20's `/V` 5 fixes. Written
        // in bytes, as every revision 6 producer this reader has met writes it, and read by
        // nothing here — `key_length` is asked only for the revisions where `/V` leaves it open.
        filter.insert(Name::new(&b"Length"[..]), Object::Integer(32));
        let mut filters = Dictionary::new();
        filters.insert(
            Name::new(crypt::STANDARD_CRYPT_FILTER),
            Object::Dictionary(filter),
        );
        dictionary.insert(Name::new(&b"CF"[..]), Object::Dictionary(filters));
        for entry in [&b"StmF"[..], &b"StrF"[..], &b"EFF"[..]] {
            dictionary.insert(
                Name::new(entry),
                Object::Name(Name::new(crypt::STANDARD_CRYPT_FILTER)),
            );
        }

        dictionary.insert(Name::new(&b"O"[..]), string(&computed.owner));
        dictionary.insert(Name::new(&b"U"[..]), string(&computed.user));
        dictionary.insert(Name::new(&b"OE"[..]), string(&computed.owner_encryption));
        dictionary.insert(Name::new(&b"UE"[..]), string(&computed.user_encryption));
        dictionary.insert(Name::new(&b"P"[..]), Object::Integer(i64::from(flags)));
        dictionary.insert(Name::new(&b"Perms"[..]), string(&computed.perms));
        dictionary.insert(
            Name::new(&b"EncryptMetadata"[..]),
            Object::Boolean(protection.encrypt_metadata),
        );

        Ok(Self {
            encryption: Encryption::for_output(computed.key, flags, protection.encrypt_metadata),
            dictionary,
            entropy,
        })
    }

    /// §7.6.3.3's initialisation vector for one string or stream.
    ///
    /// > the initialization vector is a 16-byte random number that is stored as the first 16
    /// > bytes of the encrypted stream or string.
    ///
    /// A source that refuses stops the write: there is no weaker vector to fall back to, and a
    /// file half written under a predictable one is worse than no file.
    fn vector(&mut self) -> Result<[u8; crypt::AES_BLOCK], SerializeError> {
        let mut iv = [0u8; crypt::AES_BLOCK];
        if self.entropy.fill(&mut iv) {
            Ok(iv)
        } else {
            Err(SerializeError::Entropy)
        }
    }

    /// Encrypts every string and stream inside one indirect object — §7.6.2's rule.
    ///
    /// `assembly` is here for one reason: [`Self::method`] has to select the crypt filter the
    /// *reader* will select, and a stream's `/Type` or `/Filter` may be stated indirectly, so
    /// deciding it needs the output's other objects.
    fn value(
        &mut self,
        assembly: &Assembly<'_>,
        id: ObjectId,
        value: &Object,
        depth: usize,
        tally: &mut Written,
    ) -> Result<Object, SerializeError> {
        if depth >= MAX_REWRITE_DEPTH {
            return Ok(Object::Null);
        }
        match value {
            Object::String(bytes) => {
                let iv = self.vector()?;
                let method = self.encryption.string_method();
                Ok(Object::String(
                    self.encryption
                        .encrypt_with_iv(method, id, iv, bytes)
                        .ok_or(SerializeError::Cipher)?
                        .into(),
                ))
            }
            Object::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.value(assembly, id, item, depth.saturating_add(1), tally)?);
                }
                Ok(Object::Array(out))
            }
            Object::Dictionary(dict) => Ok(Object::Dictionary(
                self.dictionary_of(assembly, id, dict, depth, tally)?,
            )),
            Object::Stream(stream) => {
                let mut dict = self.dictionary_of(assembly, id, &stream.dict, depth, tally)?;
                let method = self.method(assembly, &stream.dict);
                // A stream the output's crypt filters leave alone consumes no initialisation
                // vector, because §7.6.3.3 asks for one per *encrypted* string and stream.
                let data = if method == crypt::Method::Identity {
                    tally.cleartext = tally.cleartext.saturating_add(1);
                    stream.data.to_vec()
                } else {
                    let iv = self.vector()?;
                    self.encryption
                        .encrypt_with_iv(method, id, iv, &stream.data)
                        .ok_or(SerializeError::Cipher)?
                };
                // §7.3.8.2's `/Length` is "[t]he number of bytes from the beginning of the line
                // following the keyword stream to the last byte, just before the keyword
                // endstream" — the encoded form as it sits in the file — and AES makes that
                // longer than the plaintext by an initialisation vector and a pad. Stated here
                // rather than left to `write_body`, so that a correct `/Length` is not counted
                // as a source's lie.
                dict.insert(
                    Name::new(&b"Length"[..]),
                    Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
                );
                Ok(Object::Stream(std::sync::Arc::new(Stream {
                    dict,
                    data: data.into(),
                    decryption_failed: false,
                })))
            }
            other => Ok(other.clone()),
        }
    }

    /// [`Self::value`] over a dictionary, honouring §7.6.2's signature exception.
    ///
    /// > Any hexadecimal strings representing the value of the Contents key in a Signature
    /// > dictionary
    ///
    /// The same exception `crate::Document` applies on the way in and on §7.5.6's way out, and
    /// applied here for the same reason: a value the reader will not decrypt must not be
    /// encrypted, or the file says something its own reader cannot read back.
    fn dictionary_of(
        &mut self,
        assembly: &Assembly<'_>,
        id: ObjectId,
        dict: &Dictionary,
        depth: usize,
        tally: &mut Written,
    ) -> Result<Dictionary, SerializeError> {
        let signature = crate::document::is_signature_dictionary(dict);
        let mut out = Dictionary::new();
        for (key, value) in dict.iter() {
            let written = if signature && key.as_bytes() == b"Contents" {
                value.clone()
            } else {
                self.value(assembly, id, value, depth.saturating_add(1), tally)?
            };
            out.insert(key.clone(), written);
        }
        Ok(out)
    }

    /// Encrypts one §7.5.7 object stream this writer built.
    ///
    /// Separate from [`Self::value`] because the questions that one asks are all about a
    /// source's dictionary and none of them arises here: Table 16's entries are this writer's
    /// own, none is a string, and the payload is a stream like any other under Table 20's
    /// `/StmF`. §7.6.2's third exception is what makes that enough —
    ///
    /// > Any strings that are inside streams such as content streams and compressed object
    /// > streams, which themselves are encrypted
    ///
    /// — so the members written into [`Group::payload`] are *not* encrypted one by one, and
    /// encrypting the carrier is the whole of what the clause asks.
    fn carrier(
        &mut self,
        id: ObjectId,
        mut dict: Dictionary,
        data: &[u8],
    ) -> Result<Object, SerializeError> {
        let iv = self.vector()?;
        let method = self.encryption.stream_method();
        let body = self
            .encryption
            .encrypt_with_iv(method, id, iv, data)
            .ok_or(SerializeError::Cipher)?;
        dict.insert(
            Name::new(&b"Length"[..]),
            Object::Integer(i64::try_from(body.len()).unwrap_or(i64::MAX)),
        );
        Ok(Object::Stream(std::sync::Arc::new(Stream {
            dict,
            data: body.into(),
            decryption_failed: false,
        })))
    }

    /// Which crypt filter one stream's data is encrypted with.
    ///
    /// **The writer has to choose what the reader will choose**, so this is
    /// `Document::stream_method`'s three questions asked of the output's own dictionary, in the
    /// same order and with the same defaults. §7.6.2's Table 20, under `/StmF`, states two of
    /// them —
    ///
    /// > All streams in the document, except for cross-reference streams … or streams that
    /// > have a Crypt entry in their Filter array …, shall be decrypted by the security
    /// > handler, using this crypt filter.
    ///
    /// — and Table 21's `/EncryptMetadata` the third. A cross-reference stream never reaches
    /// this function: [`cross_reference_stream`] writes its own, after every object.
    ///
    /// A `/Crypt` entry a source stated survives into the output, so the filter it names is
    /// asked of *this* file's `/CF`, which holds one name. A `/Crypt` naming anything else —
    /// including §7.6.6's default of `Identity` where its `/DecodeParms` states no `/Name` —
    /// leaves the stream in the clear, which is what a reader of the output will expect of it,
    /// and [`Written::cleartext`] counts it so that a caller can say so.
    fn method(&self, assembly: &Assembly<'_>, dict: &Dictionary) -> crypt::Method {
        let kind = assembly
            .through(dict.get("Type"))
            .as_name()
            .map(|name| name.as_bytes().to_vec());
        if kind.as_deref() == Some(b"Metadata") && !self.encryption.encrypt_metadata() {
            return crypt::Method::Identity;
        }

        let filters = assembly.filter_names(dict);
        if let Some(index) = filters.iter().position(|name| name == b"Crypt") {
            let named = assembly
                .parms_at(dict, index)
                .and_then(|parms| assembly.through(parms.get("Name")).as_name().cloned());
            return named.map_or(crypt::Method::Identity, |name| {
                self.encryption.named_method(&name)
            });
        }

        if kind.as_deref() == Some(b"EmbeddedFile") {
            return self.encryption.embedded_file_method();
        }
        self.encryption.stream_method()
    }
}

/// Where a cross-reference section says one object is.
///
/// Table 18's two in-use entry types, and the reason [`Entry`] exists at all: until §7.5.7 was
/// generated, every object this writer wrote was at a byte offset and a `Vec<u64>` said so.
#[derive(Debug, Clone, Copy)]
enum Entry {
    /// Type 1: "[t]he byte offset of the object, starting from the beginning of the PDF file."
    InFile(u64),
    /// Type 2: "[t]he object number of the object stream in which this object is stored" and
    /// "[t]he index of this object within the object stream".
    InStream {
        /// The object stream's own number.
        stream: u32,
        /// The member's position in it, which NOTE 6 of §7.5.8.3 bounds by `/N` minus one.
        index: u32,
    },
}

/// One object stream being filled.
///
/// Held whole because §7.5.7's header states every member's offset before the first member is
/// written — "[t]he value of the First entry in the stream dictionary shall be the byte offset
/// in the decoded stream of the first object" — so the payload's length has to be known before
/// the payload can be written. [`ObjectStreams::Generate`]'s two ceilings are what bound it.
#[derive(Debug, Default)]
struct Group {
    /// Each member's object number and the slot whose [`Entry`] the flush fills in.
    members: Vec<(u32, usize)>,
    /// Each member's byte offset within [`Self::payload`].
    offsets: Vec<usize>,
    /// The members' bytes, back to back.
    payload: Vec<u8>,
}

impl Group {
    /// Appends one object, recording where it starts.
    fn push(&mut self, number: u32, slot: usize, value: &Object) {
        self.members.push((number, slot));
        self.offsets.push(self.payload.len());
        write::object(value, &mut self.payload);
        // NOTE 7's 2020 correction makes an object end "prior to the byte offset of the next
        // object", so nothing separates two members but the next one's offset. The newline is
        // written anyway, because the clause also requires the *first* object to follow the
        // header "separated by white-space" and a uniform rule is one rule.
        self.payload.push(b'\n');
    }

    /// Whether either of §7.5.7 NOTE 4's ceilings has been reached.
    fn full(&self, max_objects: usize, max_bytes: usize) -> bool {
        self.members.len() >= max_objects || self.payload.len() >= max_bytes
    }
}

/// Writes a finished assembly as a whole PDF file.
///
/// The order is §7.5.1's: header, body, cross-reference section, trailer. A carried stream's
/// data crosses from the source's `Arc<[u8]>` to the sink without being decoded, copied or
/// examined; under [`Streams::Recompress`] a stream is decoded and deflated in memory instead,
/// which is that variant's stated cost.
///
/// `version` is the header's, and it is the caller's to decide: it knows which of the sources'
/// constructs survived into the output. [`Form::Stream`] raises it to 1.5 where it is lower,
/// because §7.5.8.1 introduces the construct at that version and a file whose header disowned
/// its own cross-reference section would be one no §7.5.8.4 reader could recover. §7.5.7's
/// NOTE 3 asks for the same floor from the other side — "[u]se of compressed objects requires a
/// PDF 1.5 PDF reader" — and gets it for free, because generating object streams forces the
/// form.
///
/// # Errors
///
/// [`SerializeError`]: no `/Root`, a `/Root` that reaches no dictionary, a reserved slot never
/// filled, an offset past §7.5.4's ten digits, or a sink that refused the bytes.
pub fn serialize<W: Write>(
    assembly: &Assembly<'_>,
    version: Version,
    options: Options,
    out: &mut W,
) -> Result<Written, SerializeError> {
    write_file(assembly, version, options, None, out)
}

/// [`serialize`], with §7.6.4's standard security handler over the output.
///
/// The handler is `/V` 5, `/R` 6 and Table 25's `AESV3` — §7.6.3.3's Algorithm 1.A over a
/// 256-bit file encryption key — because that is the one configuration this standard leaves
/// undeprecated: Table 20 says "Values less than 5 for the V entry are deprecated in PDF 2.0"
/// and §7.6.4.1 says "Use of security handler revisions 1, 2, 3, 4 and 5 is deprecated in PDF
/// 2.0". A reader meets whatever revision a file states, which is why `crate::crypt` reads five
/// of them; a writer chooses, so it chooses this one.
///
/// `entropy` is where every byte neither the assembly nor the plan determines comes from — the
/// file encryption key, §7.6.4.4.7's and §7.6.4.4.8's four salts, §7.6.4.4.9's filler, and
/// §7.6.3.3's initialisation vector for every string and stream. It is an argument rather than
/// a call into the platform so that the output stays a function of its inputs: a test supplies
/// a fixed source and states the file it expects, and a host supplies [`SystemEntropy`].
///
/// The output's version is raised to 2.0 where it is lower, for the same reason
/// [`Form::Stream`] raises it to 1.5: Table 20 introduces `/V` 5 and Table 21 `/R` 6 at that
/// version, and a file whose header disowned its own encryption dictionary would be one no
/// reader could be expected to open.
///
/// # Errors
///
/// [`serialize`]'s, plus [`SerializeError::Protection`] where §7.6.4.1's password preparation
/// refuses a password, [`SerializeError::Entropy`] where the randomness source refuses, and
/// [`SerializeError::Cipher`] where the cipher does.
pub fn serialize_encrypted<W: Write>(
    assembly: &Assembly<'_>,
    version: Version,
    options: Options,
    protection: &Protection<'_>,
    entropy: &mut dyn Entropy,
    out: &mut W,
) -> Result<Written, SerializeError> {
    let mut protected = Protected::new(protection, entropy)?;
    write_file(assembly, version, options, Some(&mut protected), out)
}

/// The body of [`serialize`] and [`serialize_encrypted`], which differ in one argument.
fn write_file<W: Write>(
    assembly: &Assembly<'_>,
    version: Version,
    options: Options,
    mut protected: Option<&mut Protected<'_>>,
    out: &mut W,
) -> Result<Written, SerializeError> {
    let root = catalog_of(assembly)?;
    let ceilings = options.object_streams.ceilings();
    // Table 18 decides this rather than the caller: a compressed object is named by a type 2
    // entry, entry types exist only in a cross-reference stream, and §7.5.4's twenty-byte line
    // has no field that could say which object stream an object is in.
    let form = if ceilings.is_some() {
        Form::Stream
    } else {
        options.form
    };
    let version = header_version(version, form, protected.is_some());
    let mut sink = Counted {
        out,
        at: 0,
        digest: <md5::Md5 as md5::Digest>::new(),
    };
    let mut tally = Written::default();
    write_header(&mut sink, version)?;

    // Object *n* is slot *n - 1*, and it is written where the loop reaches it: the file's
    // order is the assembly's, which is what makes the output a function of the plan. An
    // object §7.5.7 permits to be compressed is not written here at all — it goes into the
    // group being filled, and the object stream carrying it is written where the group fills
    // up, so the file's order is still the assembly's.
    let mut entries: Vec<Entry> = vec![Entry::InFile(0); assembly.slots.len()];
    let mut carriers: Vec<Entry> = Vec::new();
    let mut group = Group::default();
    let mut extends: Option<ObjectId> = None;
    let mut next_carrier = u32::try_from(assembly.slots.len().saturating_add(1))
        .map_err(|_| SerializeError::OffsetTooLarge { offset: u64::MAX })?;
    let mut buffer: Vec<u8> = Vec::new();
    for index in 0..assembly.slots.len() {
        let number = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
        let id = ObjectId::new(number, 0);
        let value = assembly
            .resolved(index, options.streams, &mut tally)
            .ok_or(SerializeError::Unplaced { id })?;

        // Errata Collection 3's Issue #439 appends "[t]he document catalog … in an encrypted
        // document" to §7.5.7's list of what shall not be stored in an object stream, so an
        // encrypted file's `/Root` is written at the outermost level; an unencrypted one's may
        // be compressed, and is.
        let outermost = protected.is_some() && number == root.number;
        if let Some((max_objects, max_bytes)) = ceilings
            && packable(&value)
            && !outermost
        {
            // §7.6.2's third exception: a compressed object's own strings are not encrypted,
            // because the object stream carrying them is. `flush` encrypts the carrier.
            group.push(number, index, &value);
            if group.full(max_objects, max_bytes) {
                flush(
                    &mut group,
                    &mut entries,
                    &mut carriers,
                    &mut sink,
                    &mut next_carrier,
                    &mut extends,
                    options.streams,
                    protected.as_deref_mut(),
                    &mut tally,
                )?;
            }
            continue;
        }

        let value = match protected.as_deref_mut() {
            Some(handler) => handler.value(assembly, id, &value, 0, &mut tally)?,
            None => value,
        };
        if let Some(entry) = entries.get_mut(index) {
            *entry = Entry::InFile(sink.at);
        }
        buffer.clear();
        let _ = writeln!(HexSink(&mut buffer), "{number} 0 obj");
        write_body(&value, &mut buffer, &mut sink, &mut tally)?;
    }
    flush(
        &mut group,
        &mut entries,
        &mut carriers,
        &mut sink,
        &mut next_carrier,
        &mut extends,
        options.streams,
        protected.as_deref_mut(),
        &mut tally,
    )?;

    let encrypt = write_encrypt_dictionary(
        protected,
        next_carrier,
        &mut carriers,
        &mut buffer,
        &mut sink,
        &mut tally,
    )?;
    entries.append(&mut carriers);
    tally.objects = u32::try_from(entries.len()).unwrap_or(u32::MAX);

    let start = sink.at;
    let size = u64::from(tally.objects).saturating_add(1);
    cross_reference(
        form,
        &mut sink,
        &entries,
        size,
        root,
        assembly.info,
        encrypt,
    )?;
    let mut tail = String::new();
    let _ = write!(tail, "startxref\n{start}\n%%EOF\n");
    sink.put(tail.as_bytes())?;

    tally.bytes = sink.at;
    Ok(tally)
}

/// The output's catalog, refused before a byte is written.
///
/// Before rather than during, because a refusal that arrives after half a file has reached the
/// caller's sink is a refusal it has to clean up after. `RootReach::Unplaced` is left to the
/// writing loop, which names the number that was owed.
pub(crate) fn catalog_of(assembly: &Assembly<'_>) -> Result<ObjectId, SerializeError> {
    let root = assembly.root.ok_or(SerializeError::NoRoot)?;
    match assembly.root_reaches(root) {
        RootReach::Dictionary | RootReach::Unplaced => Ok(root),
        RootReach::Elsewhere => Err(SerializeError::RootNotADictionary { id: root }),
    }
}

/// The version §7.5.2's header states: the caller's, raised by the constructs this file uses.
///
/// §7.5.8.1 introduces the cross-reference stream at 1.5 and §7.5.7's NOTE 3 says the same of
/// compressed objects — "[u]se of compressed objects requires a PDF 1.5 PDF reader" — while
/// Table 20 marks `/V` 5 and Table 21 `/R` 6 as PDF 2.0. A file whose header disowned its own
/// cross-reference section or its own encryption dictionary would be one no reader could be
/// expected to recover.
fn header_version(version: Version, form: Form, encrypted: bool) -> Version {
    let version = match form {
        Form::Stream => version.max(Version { major: 1, minor: 5 }),
        Form::Table => version,
    };
    if encrypted {
        version.max(Version { major: 2, minor: 0 })
    } else {
        version
    }
}

/// §7.5.2's two lines.
///
/// "The PDF file begins with the 5 characters '%PDF-'"; and "[i]f a PDF file contains binary
/// data, as most do, the header line shall be immediately followed by a comment line containing
/// at least four binary characters — that is, characters whose codes are 128 or greater." Every
/// output of this serializer carries stream data, so the second line is written unconditionally
/// rather than after a scan.
fn write_header<W: Write>(
    sink: &mut Counted<'_, W>,
    version: Version,
) -> Result<(), SerializeError> {
    sink.put(&header(version))
}

/// §7.5.2's two lines as bytes, for both writers of a whole file.
pub(crate) fn header(version: Version) -> Vec<u8> {
    let mut header = String::new();
    let _ = writeln!(header, "%PDF-{version}");
    let mut bytes = header.into_bytes();
    bytes.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
    bytes
}

/// Writes the `/Encrypt` dictionary as the file's last indirect object, and answers its number.
///
/// §7.6.2: "Any strings in an Encrypt dictionary" are exempt from encryption, and Table 20 says
/// the same of every value in it — "[t]he values of the keys defined in [Table 20] shall not be
/// encrypted". So this is the one object [`write_file`] writes without asking the handler, and
/// it is written *after* every object that did ask, because until then the handler is borrowed.
///
/// `None`, and nothing written, for an unencrypted file.
fn write_encrypt_dictionary<W: Write>(
    protected: Option<&mut Protected<'_>>,
    number: u32,
    carriers: &mut Vec<Entry>,
    buffer: &mut Vec<u8>,
    sink: &mut Counted<'_, W>,
    tally: &mut Written,
) -> Result<Option<ObjectId>, SerializeError> {
    let Some(handler) = protected else {
        return Ok(None);
    };
    carriers.push(Entry::InFile(sink.at));
    buffer.clear();
    let _ = writeln!(HexSink(buffer), "{number} 0 obj");
    let dictionary = Object::Dictionary(handler.dictionary.clone());
    write_body(&dictionary, buffer, sink, tally)?;
    Ok(Some(ObjectId::new(number, 0)))
}

/// Whether §7.5.7 permits this object to be stored in an object stream.
///
/// The clause's own list, sentence by sentence — "[t]he following objects shall not be stored
/// in an object stream" — with the two that this writer satisfies by construction named rather
/// than omitted, because a rule met by accident is a rule waiting to be broken:
///
/// - **"Stream objects"** — the one condition that has to be tested, and the reason an object
///   stream can always be reached by a scan: the carriers are themselves streams, so they are
///   never inside one another.
/// - **"Objects with a generation number other than zero"** — [`Assembly::push`] gives every
///   output object generation 0, which is also what "[t]he generation number of an object
///   stream and of any compressed object shall be zero" requires of the carrier.
/// - **"A document's encryption dictionary"** — [`serialize`] emits no `/Encrypt` at all, so
///   the output has none. Errata Collection 3's Issue #439 appends the document catalog of an
///   *encrypted* document to the list, and it is satisfied the same way and only that way: in
///   an unencrypted file the catalog may be compressed, and it is.
/// - **"An object representing the value of the Length entry in an object stream dictionary"**
///   — this writer states `/Length` as a direct integer, so no such object exists.
/// - **"In linearized files … the document catalog dictionary, the linearization dictionary,
///   and page objects"** — conditional on a linearised file, which only
///   [`crate::linearize::serialize_linearized`] writes, and that writer puts no object in an
///   object stream at all. So the condition is false for every file this function writes, and
///   met by construction in every file that one does.
///
/// And one sentence from further down the clause, which is a rule about the *value* rather
/// than about the object: "[a]n object in an object stream shall not consist solely of an
/// object reference."
fn packable(value: &Object) -> bool {
    !matches!(value, Object::Stream(_) | Object::Reference(_))
}

/// Writes one object's value and the keyword that ends it, into `buffer` and then to `sink`.
///
/// The one place a stream's data crosses, and — under [`Streams::Carry`] — it crosses encoded:
/// the source's bytes as its producer filtered them.
fn write_body<W: Write>(
    value: &Object,
    buffer: &mut Vec<u8>,
    sink: &mut Counted<'_, W>,
    tally: &mut Written,
) -> Result<(), SerializeError> {
    match value {
        Object::Stream(stream) => {
            let mut dict = stream.dict.clone();
            let stated = dict.get("Length").and_then(Object::as_integer);
            let actual = i64::try_from(stream.data.len()).unwrap_or(i64::MAX);
            if stated != Some(actual) {
                tally.relengthed = tally.relengthed.saturating_add(1);
            }
            dict.insert(Name::new(&b"Length"[..]), Object::Integer(actual));
            write::object(&Object::Dictionary(dict), buffer);
            buffer.extend_from_slice(b"\nstream\n");
            sink.put(buffer)?;
            sink.put(&stream.data)?;
            sink.put(b"\nendstream\nendobj\n")
        }
        other => {
            write::object(other, buffer);
            buffer.extend_from_slice(b"\nendobj\n");
            sink.put(buffer)
        }
    }
}

/// Writes the group being filled as one §7.5.7 object stream, and empties it.
///
/// Does nothing where the group is empty, which is what makes calling it after the loop
/// correct whether or not the last group was already flushed.
#[expect(
    clippy::too_many_arguments,
    reason = "one write of one object stream needs the group, where its members' entries go, \
              where the carrier's own entry goes, the sink, the next number, the previous \
              carrier for /Extends, the compression policy, the output's security handler and \
              the tally; bundling them would be a struct whose only method is this function"
)]
fn flush<W: Write>(
    group: &mut Group,
    entries: &mut [Entry],
    carriers: &mut Vec<Entry>,
    sink: &mut Counted<'_, W>,
    next_number: &mut u32,
    extends: &mut Option<ObjectId>,
    streams: Streams,
    protected: Option<&mut Protected<'_>>,
    tally: &mut Written,
) -> Result<(), SerializeError> {
    if group.members.is_empty() {
        return Ok(());
    }
    let number = *next_number;
    *next_number = next_number.saturating_add(1);

    // §7.5.7: "N pairs of integers separated by white-space, where the first integer in each
    // pair shall represent the object number of a compressed object and the second integer
    // shall represent the byte offset in the decoded stream of that object, relative to the
    // first object stored in the object stream … The byte offsets shall be in increasing
    // order. The pairs, themselves, shall also be separated by white-space." They are in
    // increasing order because `Group::push` appends.
    let mut head = String::new();
    for ((member, _), offset) in group.members.iter().zip(group.offsets.iter()) {
        let _ = write!(head, "{member} {offset} ");
    }
    // "A PDF writer shall store the first object immediately after the last byte offset
    // separated by white-space", so the header ends with one and `/First` is its whole length.
    head.push('\n');
    let first = head.len();

    let mut data = Vec::with_capacity(first.saturating_add(group.payload.len()));
    data.extend_from_slice(head.as_bytes());
    data.extend_from_slice(&group.payload);
    // NOTE 2 makes the encoding optional — "[t]he term 'compressed object' is used regardless
    // of whether the stream is actually encoded with a compression filter" — but NOTE 1 makes
    // it the whole purpose, so a caller who asked for object streams and got them uncompressed
    // would have paid the indirection for nothing. Under `Streams::Carry` the level is the
    // default, because there is no source encoding here to carry: these bytes are this
    // writer's own.
    let level = match streams {
        Streams::Carry => match Streams::DEFAULT {
            Streams::Recompress { level } => level,
            Streams::Carry => 9,
        },
        Streams::Recompress { level } => level,
    };
    let encoded = deflate(&data, level).ok_or_else(|| {
        SerializeError::Write(std::io::Error::other(
            "an object stream's payload could not be deflated",
        ))
    })?;

    let mut dict = Dictionary::new();
    // Table 16, every required entry and the one optional one.
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"ObjStm"[..])),
    );
    dict.insert(
        Name::new(&b"N"[..]),
        Object::Integer(i64::try_from(group.members.len()).unwrap_or(i64::MAX)),
    );
    dict.insert(
        Name::new(&b"First"[..]),
        Object::Integer(i64::try_from(first).unwrap_or(i64::MAX)),
    );
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    // Table 16's `/Extends`: "[a] reference to another object stream, of which the current
    // object stream is an extension. Both streams are considered part of a collection of
    // object streams … A given collection consists of a set of streams whose Extends links
    // form a directed acyclic graph." NOTE 4 describes this writer's exact situation — one
    // collection cut into several streams because "the number of objects in an individual
    // object stream needs to be limited" — so the chain is written, and a chain is a DAG.
    if let Some(previous) = *extends {
        dict.insert(Name::new(&b"Extends"[..]), Object::Reference(previous));
    }

    let carrier = match protected {
        Some(handler) => handler.carrier(ObjectId::new(number, 0), dict, &encoded)?,
        None => Object::Stream(std::sync::Arc::new(Stream {
            dict,
            data: encoded.into(),
            decryption_failed: false,
        })),
    };
    carriers.push(Entry::InFile(sink.at));
    let mut buffer = Vec::new();
    let _ = writeln!(HexSink(&mut buffer), "{number} 0 obj");
    write_body(&carrier, &mut buffer, sink, tally)?;

    for (index, (_, slot)) in group.members.iter().enumerate() {
        if let Some(entry) = entries.get_mut(*slot) {
            *entry = Entry::InStream {
                stream: number,
                index: u32::try_from(index).unwrap_or(u32::MAX),
            };
        }
    }
    tally.object_streams = tally.object_streams.saturating_add(1);
    tally.compressed = tally
        .compressed
        .saturating_add(u32::try_from(group.members.len()).unwrap_or(u32::MAX));
    *extends = Some(ObjectId::new(number, 0));
    group.members.clear();
    group.offsets.clear();
    group.payload.clear();
    Ok(())
}

/// One stream decoded through the filters this tree reads and re-encoded as one `FlateDecode`,
/// or `None` where it is carried instead.
///
/// **`None` is the answer wherever anything is uncertain, and that is the whole design.** The
/// amended exclusion permits recompression "without reinterpretation", so a re-encoding is
/// legitimate exactly when the decoded bytes are provably the same bytes. Six conditions
/// therefore carry rather than recompress:
///
/// - a stream whose data lives in another file (§7.3.8.2's `/F`), which this program never
///   opens;
/// - a stream the document could not decrypt, whose `data` is empty rather than ciphertext;
/// - a stream stating `/Length 0` with no bytes, which §7.3.8.2 makes a producer's deliberate
///   silence rather than a compressible payload;
/// - a filter this tree does not decode, or one whose decode refused;
/// - a decode that came back damaged — bytes that stop short of what the file says the stream
///   is. Re-encoding those would write a whole stream over a truncated one and lose the fact;
/// - a `/Crypt` filter anywhere in the chain, because §7.4.10's entry names a crypt filter in a
///   document's `/CF` and the output has no `/Encrypt` for it to name;
/// - a stream whose `/Filter` or `/DecodeParms` is stated in a shape [`filter_objects`] and
///   [`parms_objects`] will not slice — an indirect entry, an array element that is not a name,
///   or a `/DecodeParms` array whose length disagrees with the filter chain's. This one is about
///   the closure rather than about the bytes: what a re-encoded stream states for those two
///   entries comes out of the **renumbered** dictionary, so that a parameter naming another
///   object still names it, and a dictionary that cannot be read that way is carried instead.
///
/// An **image codec stops the walk instead of refusing it**, and the tail from there on is kept
/// in `/Filter`: `[/ASCII85Decode /DCTDecode]` becomes `[/FlateDecode /DCTDecode]` with the
/// producer's JPEG bytes untouched inside. That is `Document::image_stream`'s reading of the
/// same chains from the other side — "[o]nly the last entry can be a codec" — used here to
/// decide where this function must stop rather than what an image is.
///
/// The decode is run here rather than through [`Document::decoded_stream_data`] for one
/// reason: that route memoises, and a caller that recompresses every stream of a document
/// would leave the whole decoded document in the memo.
fn recompressed(
    document: &Document,
    source: &Stream,
    renumbered: &std::sync::Arc<Stream>,
    level: u32,
    tally: &mut Written,
) -> Option<Object> {
    if source.decryption_failed
        || Document::is_external(source)
        || document.states_no_data(source)
        || source.data.is_empty()
    {
        return None;
    }
    let chain = document.filter_chain(&source.dict);
    // What survives of `/Filter` and `/DecodeParms` is taken out of the *renumbered* dictionary
    // rather than rebuilt from the source's, because a parameter may name another object —
    // `/JBIG2Globals` is the standing example — and the source's number is not the output's.
    // `bitmap-p32-eof.pdf` is the witness: its image's `/DecodeParms << /JBIG2Globals 3 0 R >>`
    // was rebuilt from the source, so the output's image named whatever object 3 had become and
    // the globals it needed were written and referred to by nothing.
    let names = filter_objects(&renumbered.dict)?;
    if names.len() != chain.len() {
        return None;
    }
    let parms = parms_objects(&renumbered.dict, names.len())?;

    let mut data: std::sync::Arc<[u8]> = std::sync::Arc::clone(&source.data);
    let mut stop = chain.len();
    let limits = document.limits();
    for (index, filter) in chain.iter().enumerate() {
        if crate::filter::is_image_codec(filter) {
            stop = index;
            break;
        }
        if filter.as_slice() == b"Crypt" {
            return None;
        }
        let stage = crate::filter::decode_with_parms_reported(
            filter,
            &data,
            document.decode_parms(&source.dict, index).as_ref(),
            limits,
        )
        .ok()?;
        if stage.damage.is_some() {
            return None;
        }
        data = stage.data;
    }

    // The parameters of the stages that were decoded are discarded, because the one filter
    // replacing them has none. An object stated only in one of those would therefore be
    // written and referred to by nothing — `issue5280.pdf`'s `/DecodeParms` array, whose one
    // element is an indirect `<< /Colors 3 /Columns 60 /Predictor 15 >>` — so the stream is
    // carried instead. The same discipline as `/Length`, one entry over: nothing this writer
    // stops referring to may still be in the file.
    if parms
        .get(..stop)
        .unwrap_or_default()
        .iter()
        .any(|value| holds_reference(value, 0))
    {
        return None;
    }

    let encoded = deflate(&data, level)?;
    // qpdf's rule for `--optimize-images`, which is the right rule for every stream: an object
    // that fails to shrink keeps what its producer wrote. A verb called `optimize` that made a
    // file larger would be a defect, and one that made a *stream* larger while the file shrank
    // would be one nobody could see.
    if encoded.len() >= source.data.len() {
        return None;
    }
    tally.recompressed = tally.recompressed.saturating_add(1);
    tally.saved = tally.saved.saturating_add(
        u64::try_from(source.data.len().saturating_sub(encoded.len())).unwrap_or(0),
    );

    let mut dict = renumbered.dict.clone();
    let mut filters = vec![Object::Name(Name::new(&b"FlateDecode"[..]))];
    filters.extend(names.get(stop..).unwrap_or_default().iter().cloned());
    let mut kept = vec![Object::Null];
    kept.extend(parms.get(stop..).unwrap_or_default().iter().cloned());
    let parms = kept;
    if let (1, Some(only)) = (filters.len(), filters.first()) {
        dict.insert(Name::new(&b"Filter"[..]), only.clone());
    } else {
        dict.insert(Name::new(&b"Filter"[..]), Object::Array(filters));
    }
    // Table 5: `/DecodeParms` is an array with "either the parameter dictionary for that
    // filter, or the null object if that filter has no parameters", and it is absent where no
    // filter has any. The `FlateDecode` this function writes never has parameters: the
    // predictor the source may have used was reversed by the decode, so the bytes deflated
    // here are the unpredicted ones.
    if parms.iter().any(|value| *value != Object::Null) {
        dict.insert(Name::new(&b"DecodeParms"[..]), Object::Array(parms));
    } else {
        dict.remove("DecodeParms");
    }
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(encoded.len()).unwrap_or(i64::MAX)),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}

/// Whether a value names another object anywhere inside it.
///
/// Asked of the `/DecodeParms` entries a re-encoding discards, so that discarding them cannot
/// leave an object in the file that nothing refers to.
fn holds_reference(value: &Object, depth: usize) -> bool {
    if depth >= MAX_REWRITE_DEPTH {
        return true;
    }
    match value {
        Object::Reference(_) => true,
        Object::Array(items) => items
            .iter()
            .any(|item| holds_reference(item, depth.saturating_add(1))),
        Object::Dictionary(dict) => dict
            .iter()
            .any(|(_, item)| holds_reference(item, depth.saturating_add(1))),
        Object::Stream(stream) => stream
            .dict
            .iter()
            .any(|(_, item)| holds_reference(item, depth.saturating_add(1))),
        _ => false,
    }
}

/// A stream's `/Filter` names as the output's own dictionary states them, one per stage.
///
/// Table 5 makes `/Filter` "[t]he name of a filter that shall be applied in processing the
/// stream data" or "an array of zero, one or several names", and this reads exactly those two
/// shapes: `None` for anything else — an indirect entry, or an array element that is not a name
/// — which [`recompressed`] turns into a carried stream rather than a guess about a chain it
/// cannot state.
fn filter_objects(dict: &Dictionary) -> Option<Vec<Object>> {
    match dict.get("Filter") {
        None => Some(Vec::new()),
        Some(Object::Name(name)) => Some(vec![Object::Name(name.clone())]),
        Some(Object::Array(items)) => items
            .iter()
            .map(|item| match item {
                Object::Name(name) => Some(Object::Name(name.clone())),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// A stream's `/DecodeParms` as the output's own dictionary states them, one per filter.
///
/// Table 5: a parameter dictionary, "or an array of such dictionaries" where several filters
/// have parameters, each element "either the parameter dictionary for that filter, or the null
/// object if that filter has no parameters". An array whose length disagrees with the filter
/// chain's is a file this function will not slice, so it answers `None`.
fn parms_objects(dict: &Dictionary, filters: usize) -> Option<Vec<Object>> {
    match dict.get("DecodeParms") {
        None => Some(vec![Object::Null; filters]),
        Some(Object::Dictionary(parms)) if filters <= 1 => {
            Some(vec![Object::Dictionary(parms.clone())])
        }
        Some(Object::Array(items)) if items.len() == filters => Some(items.clone()),
        _ => None,
    }
}

/// §7.4.4.1's `FlateDecode`, on the way out: zlib's own container, at `level`.
///
/// **Public because a caller may have to state the filter itself.** `pdf-transform`'s `archive`
/// verb re-encodes a stream ISO 19005 forbids the filter of — its `LZWDecode` rewrite — and it
/// has to do so whether or not the result is smaller, which is the one thing [`Streams`]'
/// recompression will not do. A second encoder there would be a second set of zlib settings for
/// one file format, so the writer's own is what it asks.
///
/// `None` where the encoder refuses, which for a `Vec` sink is a corrupt-state answer rather
/// than a full disk; the callers turn it into a carried stream or a refusal.
#[must_use]
pub fn flate_encode(data: &[u8], level: u32) -> Option<Vec<u8>> {
    deflate(data, level)
}

/// [`flate_encode`]'s body, which the whole module reaches without the `pub` path.
fn deflate(data: &[u8], level: u32) -> Option<Vec<u8>> {
    let mut encoder = flate2::write::ZlibEncoder::new(
        Vec::with_capacity(data.len() / 2),
        flate2::Compression::new(level.min(9)),
    );
    encoder.write_all(data).ok()?;
    encoder.finish().ok()
}

/// §7.5.4's classic table, then §7.5.5's `trailer` keyword and dictionary.
/// §7.5.1's third section, in whichever of its two forms this file uses.
fn cross_reference<W: Write>(
    form: Form,
    sink: &mut Counted<'_, W>,
    entries: &[Entry],
    size: u64,
    root: ObjectId,
    info: Option<ObjectId>,
    encrypt: Option<ObjectId>,
) -> Result<(), SerializeError> {
    match form {
        Form::Table => {
            // Every entry is an offset here, by construction: `form` is `Form::Stream`
            // wherever an object stream was generated, so a classic table never has to state
            // where a compressed object is. A type 2 entry reaching this arm would be a
            // refusal rather than a silent zero, which is what `u64::MAX` produces.
            let offsets: Vec<u64> = entries
                .iter()
                .map(|entry| match entry {
                    Entry::InFile(at) => *at,
                    Entry::InStream { .. } => u64::MAX,
                })
                .collect();
            cross_reference_table(sink, &offsets, size, root, info, encrypt)
        }
        Form::Stream => cross_reference_stream(sink, entries, size, root, info, encrypt),
    }
}

fn cross_reference_table<W: Write>(
    sink: &mut Counted<'_, W>,
    offsets: &[u64],
    size: u64,
    root: ObjectId,
    info: Option<ObjectId>,
    encrypt: Option<ObjectId>,
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
    let trailer = trailer_dictionary(size, root, info, encrypt, sink.digest());
    let mut buffer = Vec::new();
    write::object(&Object::Dictionary(trailer), &mut buffer);
    buffer.push(b'\n');
    sink.put(&buffer)
}

/// §7.5.8's cross-reference stream: an indirect object whose dictionary is the trailer.
fn cross_reference_stream<W: Write>(
    sink: &mut Counted<'_, W>,
    entries: &[Entry],
    size: u64,
    root: ObjectId,
    info: Option<ObjectId>,
    encrypt: Option<ObjectId>,
) -> Result<(), SerializeError> {
    // The stream is an object and its own entry has to be in it, so it takes the next number
    // and its offset is where it is about to be written.
    let number = u32::try_from(size).unwrap_or(u32::MAX);
    let at = sink.at;

    // `/W [1 4 2]`, Table 18's three fields at their widest useful sizes: the type, then a
    // field that is a four-byte offset for a type 1 entry and an object stream's number for a
    // type 2 one, then a two-byte generation for a type 1 entry and the member's index for a
    // type 2 one. Type 0 is a free object, whose second field is "the object number of the
    // next free object" and whose third is "the generation number to use if this object number
    // is used again".
    let mut data = Vec::with_capacity(entries.len().saturating_add(2).saturating_mul(7));
    row(0, 0, FREE_FOREVER, &mut data);
    for entry in entries {
        match entry {
            Entry::InFile(offset) => {
                if *offset > u64::from(u32::MAX) {
                    return Err(SerializeError::OffsetTooLarge { offset: *offset });
                }
                row(1, u32::try_from(*offset).unwrap_or(u32::MAX), 0, &mut data);
            }
            Entry::InStream { stream, index } => {
                row(
                    2,
                    *stream,
                    u16::try_from(*index).unwrap_or(u16::MAX),
                    &mut data,
                );
            }
        }
    }
    if at > u64::from(u32::MAX) {
        return Err(SerializeError::OffsetTooLarge { offset: at });
    }
    row(1, u32::try_from(at).unwrap_or(u32::MAX), 0, &mut data);

    let mut dict = trailer_dictionary(size.saturating_add(1), root, info, encrypt, sink.digest());
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

/// One §7.5.8.3 record: the type, then the two fields `/W [1 4 2]` gives four and two bytes.
///
/// "Fields requiring more than one byte are stored with the high-order byte first."
fn row(kind: u8, second: u32, third: u16, out: &mut Vec<u8>) {
    out.push(kind);
    out.extend_from_slice(&second.to_be_bytes());
    out.extend_from_slice(&third.to_be_bytes());
}

/// §7.5.5's Table 15, for a file with exactly one cross-reference section.
///
/// `/Size` and `/Root` are required; `/Prev` is not written because there is nothing before
/// this section; `/Info` is written where the caller carried one; and `/Encrypt` is written
/// exactly where [`serialize_encrypted`] wrote the dictionary it names, which §7.6.2 makes the
/// entry that decides whether the file is encrypted at all — "[t]he absence of this entry from
/// the trailer dictionary means that a PDF processor shall consider the document to be not
/// encrypted."
///
/// The `/ID` the line below writes is a direct array of two direct strings, which is what Table
/// 15 requires of an encrypted file — with an `/Encrypt` present, both "shall be direct objects
/// and shall be unencrypted" — and is what [`identify`] has always written.
fn trailer_dictionary(
    size: u64,
    root: ObjectId,
    info: Option<ObjectId>,
    encrypt: Option<ObjectId>,
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
    if let Some(encrypt) = encrypt {
        trailer.insert(Name::new(&b"Encrypt"[..]), Object::Reference(encrypt));
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
