//! A whole file: cross-references, object access, and stream decoding.
//!
//! # Lazy by design
//!
//! Opening a document reads the cross-reference information and nothing else. Objects are
//! parsed when asked for and then remembered. That is what makes a 500-page file open as
//! fast as a 5-page one, which `CLAUDE.md` principle 2 requires — eagerly walking a page
//! tree of thousands of nodes is the most common reason viewers feel slow to start.
//!
//! **This paragraph said "and decoded streams are cached", and that was not true of
//! ordinary streams**; the four-hundred-and-twenty-fourth session counted the calls and
//! found [`Document::decoded_stream_data`] running 12 717 times over one sweep of ISO
//! 32000-2 and 11 975 times over the *second* sweep of the same document, which is a filter
//! chain re-run rather than a cache read. It is true again as of the
//! four-hundred-and-eighty-second, which measured what those re-runs cost and gave them a
//! byte budget: [`DECODED_BUDGET`] and [`Document::decoded_streams`] are the shape, ADR 0317
//! the argument.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

use crate::crypt::{Encryption, Method, Permissions};
use crate::error::{SyntaxError, SyntaxResult};
use crate::filter::{Damage, Decoded, FilterRefusal};
use crate::object::{Dictionary, Name, Object, ObjectId, Stream};
use crate::parser::{Limits, Parser};
use crate::xref::{Location, XrefTable};

/// The most indirect references that will be followed in a chain.
///
/// `1 0 obj 2 0 R endobj` pointing back at itself is a cycle, and a chain of a thousand
/// references is hostile rather than merely unusual.
const MAX_REFERENCE_DEPTH: usize = 64;

/// How many bytes of decoded stream data one open document may hold.
///
/// **4 MiB, and both halves of that are derived rather than chosen.**
///
/// *The ceiling* is the project owner's band — "1 GB is definitely too much, below 10 MB is
/// definitely ok" (ADR 0256) — less the 4 MiB the readback cache already spends on an open
/// document. Two per-document caches of 4 MiB is 8 MB, which is inside the band the owner
/// stated, and this one is the second of them rather than the first.
///
/// *The floor* is what the measurement says a smaller one would give up. Replaying one cold
/// sweep of ISO 32000-2's 1023 pages — 12 586 filtered decodes over 3 936 distinct streams —
/// through a least-recently-used simulation at each budget:
///
/// | budget | of the sweep's own wall clock, saved | evictions |
/// |---|---|---|
/// | 1 MiB | 21.4% | 4 568 |
/// | 4 MiB | **23.1%** | 3 715 |
/// | 64 MiB (the whole working set, 46.6 MB) | 23.4% | 0 |
///
/// So 4 MiB is within 0.3 percentage points of an unbounded cache on the largest document
/// this project owns, and the bound costs a document nothing it can notice. What it buys is
/// in ADR 0317 and in the A/B on [`Document::decoded_streams`].
pub const DECODED_BUDGET: usize = 4 * 1024 * 1024;

/// How many decoded bytes of object stream a *rebuild* spends recovering §7.5.7's objects.
///
/// [`Document::recover_compressed_objects`] decodes every object stream a header scan found, far
/// enough to read its header, which is work an attacker chooses: a file may carry any number of
/// them, each claiming to inflate to [`Limits::max_stream_len`]. The count of streams is not the
/// axis to bound — a kilobyte of headers can name thousands — so what is bounded is the decoded
/// total, checked before each stream is started. The worst case is therefore this budget plus one
/// stream, which is the exposure an ordinary document already has for one `/Contents`.
///
/// **64 MiB, and the floor under it is measured.** The widest object-stream total among every
/// document on this disk that reaches a rebuild — 261 of the 65 944 crawled, 28 of the 974 and 108
/// of the 277 in the corpora — is 12.6 MiB, in a 10 MB file with 316 object streams
/// (`pdf-model/examples/rebuild_census`). Five times the widest real one refuses none of them, and
/// a file that does exceed it is not refused either: it recovers what the budget reached and
/// [`CompressedRecovery::beyond_the_budget`] says how many streams it did not.
pub const RECOVERY_DECODE_BUDGET: usize = 64 * 1024 * 1024;

/// An open PDF file.
///
/// Holds the bytes and resolves objects on demand. Cheap to open and cheap to clone the
/// underlying bytes, since they are shared.
///
/// # Shareable between threads
///
/// The five caches below are memoisation behind `RwLock`, which makes a `&Document` usable
/// from several threads at once. They were `RefCell` until the four-hundred-and-twenty-fourth
/// session, which measured what the change costs and what it buys, because a lock on the
/// hottest path in the program is not free by assumption: `get` is asked **829 times a page**
/// on ISO 32000-2 and answers 92.7% of them from the cache. Single-threaded, the whole swap
/// is **2 208 807 721 → 2 209 269 060** instructions through `examples/callgrind_interpret`
/// — 0.021% — and 78 464 732 → 78 357 201 through `examples/callgrind_open`; a cold
/// document-wide sweep of ISO 32000-2 stays inside its own spread, 5.69 s against 5.78 s over
/// seven interleaved samples apiece with ranges of 0.30 and 0.36 s.
///
/// What that makes possible is a *host's* to use rather than this crate's:
/// `pdf-model/examples/parallel_sweep` reads all 1023 pages on 24 threads through one
/// `&Document` in 1.61 s against 6.11, at 625 MB of peak resident memory against 225. ADR 0260
/// has the tables and says why nothing in `viewer-core` does it yet.
pub struct Document {
    bytes: Arc<[u8]>,
    xref: XrefTable,
    limits: Limits,
    /// Objects already parsed, so a repeated lookup does not re-parse.
    cache: RwLock<BTreeMap<u32, Object>>,
    /// Object streams already expanded, keyed by the stream's object number.
    expanded_streams: RwLock<BTreeMap<u32, Arc<BTreeMap<u32, Object>>>>,
    /// Object numbers currently being loaded **on each thread**, so that a self-referential
    /// file cannot recurse. See [`Document::get`], and [`Document::begin_loading`] for why
    /// the set is per thread rather than per document.
    loading: RwLock<HashMap<ThreadId, BTreeSet<u32>>>,
    /// Where the file's own object headers are, built the first time an entry is disproved.
    ///
    /// Empty for every document whose table is right about every object it is asked for,
    /// which is all but two of the 974 corpus documents — the scan is linear in the file and
    /// nothing pays for it until something needs it. See [`Document::load_by_header`].
    headers: RwLock<Option<Arc<XrefTable>>>,
    /// How many object numbers were found by their own header after the table misfiled them.
    ///
    /// Counted rather than merely handled, because a document that needed this is a document
    /// whose cross-reference table is wrong, and a reader that repairs one in silence is a
    /// reader nobody can ask what it repaired. Reported by [`Document::misfiled_objects`].
    misfiled: RwLock<BTreeSet<u32>>,
    /// Objects an object stream names and this reader would not read from a prefix of it.
    ///
    /// §7.5.7 states each compressed object's extent, so an object the decoded prefix does not
    /// wholly carry is one whose bytes are partly missing — and a truncated token still parses,
    /// which is the whole reason this is a refusal rather than a best effort. Recorded rather
    /// than merely refused for the reason [`Self::misfiled_objects`] gives: a reader that drops
    /// part of a file in silence is one nobody can ask what it dropped. ADR 0366.
    lost_to_damage: RwLock<LostToDamage>,
    /// §7.4's filter chain already run, under [`DECODED_BUDGET`]. See [`DecodedStreams`].
    decoded: RwLock<DecodedStreams>,
    /// What entering §7.5.7's compressed objects into a rebuilt table found, and did not.
    ///
    /// Default for every document whose cross-reference information the file itself supplied,
    /// which is what makes it a statement about the recovery rather than about the file. See
    /// [`Document::recover_compressed_objects`].
    recovered_compressed: CompressedRecovery,
    /// ISO 32000-2 §7.6's security handler, absent when the trailer has no `/Encrypt`.
    ///
    /// > The absence of this entry from the trailer dictionary means that a PDF processor
    /// > shall consider the document to be not encrypted.
    encryption: Option<Encryption>,
    /// The object number holding the encryption dictionary, whose strings §7.6.2 exempts.
    encrypt_object: Option<u32>,
}

impl std::fmt::Debug for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Document")
            .field("bytes", &self.bytes.len())
            .field("objects", &self.xref.len())
            .field("recovered_by_scan", &self.xref.recovered_by_scan())
            // The caches are deliberately omitted: they are an implementation detail whose
            // contents depend on access history, and printing them would make debug output
            // both enormous and non-reproducible.
            .finish_non_exhaustive()
    }
}

/// Reads one of [`Document`]'s caches, past a lock another thread poisoned.
///
/// # Why poisoning is ignored here, deliberately
///
/// `std`'s `RwLock` marks itself poisoned when a thread panics while holding it, so that the
/// next caller learns the data may be half-written. That warning is about *invariants across
/// fields*, and these five locks hold no such invariant: each is a memoisation of something
/// the file already says, every write is one `insert` into one collection, and a panic
/// between two of them leaves a map that is merely smaller than it could have been. The
/// worst a poisoned cache can cost is a re-parse. Propagating a `PoisonError` instead would
/// turn a panic anywhere in the process into a document that can no longer be read at all.
fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// Writes one of [`Document`]'s caches, past a lock another thread poisoned. See [`read`].
fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

/// ISO 32000-2 §7.5.7's header: the `(object number, relative offset)` pairs an object stream
/// begins with.
///
/// > N pairs of integers separated by white-space, where the first integer in each pair shall
/// > represent the object number of a compressed object and the second integer shall represent
/// > the byte offset in the decoded stream of that object, relative to the first object stored in
/// > the object stream, the offset for which is the value of the stream's First entry.
///
/// Bounded by `count`, which is Table 16's `/N`, and **not** by `/First`: that entry is "[t]he
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

/// What object streams this document could only partly decode did not yield, as it accumulates.
#[derive(Default)]
struct LostToDamage {
    /// Numbers the header named and whose object the prefix does not wholly carry.
    objects: BTreeSet<u32>,
    /// Objects Table 16's `/N` counts whose header pair the prefix never reached.
    unnamed: usize,
    /// The object streams this happened in.
    streams: BTreeSet<u32>,
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
    /// Streams left unexpanded because [`RECOVERY_DECODE_BUDGET`] was spent on the earlier ones.
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

/// What [`Document::get_key_of`] found in the object cache before it let the lock go.
///
/// Three answers rather than two, because "the object has no such entry" and "the object is
/// not the kind of thing that has entries" are different statements and its caller needs both.
enum Held {
    /// The entry as the dictionary states it, still unresolved. [`Object::Null`] where the
    /// dictionary has no such key, which is what §7.3.9 makes an omitted entry.
    Entry(Object),
    /// The object is in hand and is not a dictionary, so it has no entries at all.
    NotADictionary,
    /// Nothing is in hand: the object has not been parsed yet, or it is itself a reference
    /// and following §7.3.10's chain is [`Document::resolve`]'s job rather than this lock's.
    Unread,
}

impl Document {
    /// Opens a document from its bytes.
    ///
    /// An encrypted document is opened with the *default user password* — the empty one —
    /// which ISO 32000-2 §7.6.4.1 requires a reader to try before prompting for anything:
    ///
    /// > If a user attempts to open an encrypted document that has a user password, the PDF
    /// > reader shall first try to authenticate the encrypted document using the padding
    /// > string defined in 7.6.4.3, "File encryption key algorithm" (default user password)
    ///
    /// # Errors
    ///
    /// [`SyntaxError::NoHeader`] if this is not a PDF,
    /// [`SyntaxError::NoCrossReferences`] if no objects can be located even by scanning,
    /// and the two encryption errors [`Self::open_with_password`] describes.
    pub fn open(bytes: impl Into<Arc<[u8]>>) -> SyntaxResult<Self> {
        Self::open_with_limits(bytes, Limits::DEFAULT)
    }

    /// Opens a document with explicit resource bounds.
    ///
    /// # Errors
    ///
    /// As [`Self::open`].
    pub fn open_with_limits(bytes: impl Into<Arc<[u8]>>, limits: Limits) -> SyntaxResult<Self> {
        Self::open_with_password(bytes, limits, "")
    }

    /// Opens a document with a password.
    ///
    /// §7.6.4.1 makes one string do for both roles — "Correctly supplying either password
    /// (owner or user password) should enable the user to gain access to the document" —
    /// so there is one parameter and [`Self::permissions`] reports which one matched.
    ///
    /// # Errors
    ///
    /// As [`Self::open`], plus [`SyntaxError::PasswordRequired`] when the document is
    /// encrypted and this password is neither of its two, and
    /// [`SyntaxError::UnsupportedEncryption`] when it names a handler or method §7.6 does
    /// not specify or this reader does not implement.
    pub fn open_with_password(
        bytes: impl Into<Arc<[u8]>>,
        limits: Limits,
        password: &str,
    ) -> SyntaxResult<Self> {
        let bytes = bytes.into();
        let xref = crate::xref::read(&bytes, limits)?;
        let mut document = Self::around(Arc::clone(&bytes), xref, limits);
        document.authenticate(password)?;
        // Nothing for a document whose table the file supplied, and §7.5.7's other half of the
        // recovery for one whose table came from a scan. It runs *after* authentication because
        // an object stream in an encrypted file is ciphertext until there is a key, and before
        // the catalogue is asked for below because `/Root` may be one of the objects it enters.
        document.recover_compressed_objects();

        // §7.5.5 makes the trailer's `/Root` "[t]he catalog dictionary for the PDF file", so
        // **a cross-reference table that leads to no catalog has been disproved by the file
        // itself** — whatever it parsed as, and however self-consistent it looked. `xref::read`
        // scans only when the table is *absent, unreadable or empty*, which leaves the case a
        // hand edit produces: a complete table whose offsets all point a few bytes wrong.
        //
        // Rebuilding is tried once, and only from here, so a well-formed document pays one
        // dictionary lookup for it. The rebuilt table is kept only if it does better, which is
        // what keeps the error a caller sees the same as before wherever this changes nothing.
        if document.catalog().is_err()
            && !document.xref.recovered_by_scan()
            && let Ok(rebuilt) = crate::xref::rebuild(&bytes, limits, true)
        {
            let mut second = Self::around(bytes, rebuilt, limits);
            if second.authenticate(password).is_ok() {
                second.recover_compressed_objects();
                if second.catalog().is_ok() {
                    return Ok(second);
                }
            }
        }
        Ok(document)
    }

    /// A document of no bytes and no objects, for reading a dictionary this program built.
    ///
    /// Every route into `pdf-model` and `pdf-font` takes a `&Document` beside its
    /// `&Dictionary`, and that is not an accident of style: a PDF dictionary's values may be
    /// indirect references (§7.3.10), so only the file can say what one holds. A dictionary
    /// **this** program assembled has no indirect references in it, and this is what fills the
    /// parameter — one empty `Arc`, one empty [`XrefTable`] and three empty maps, so it parses
    /// nothing and reads nothing.
    ///
    /// The caller is `pdf_font::LoadedFont::standard`, which loads one of §9.6.2.2's fourteen
    /// for text the *interface* draws rather than text a document states. Anything reached
    /// through this that does hold a reference resolves to [`Object::Null`], which is what
    /// [`Self::get`] answers for an object number the table does not name — the same answer a
    /// real file gives for a dangling reference, rather than a special case.
    #[must_use]
    pub fn empty() -> Self {
        Self::around(Arc::from(&[][..]), XrefTable::default(), Limits::DEFAULT)
    }

    /// The document a cross-reference table and some bytes make, before authentication.
    fn around(bytes: Arc<[u8]>, xref: XrefTable, limits: Limits) -> Self {
        Self {
            bytes,
            xref,
            limits,
            cache: RwLock::new(BTreeMap::new()),
            expanded_streams: RwLock::new(BTreeMap::new()),
            loading: RwLock::new(HashMap::new()),
            headers: RwLock::new(None),
            misfiled: RwLock::new(BTreeSet::new()),
            lost_to_damage: RwLock::new(LostToDamage::default()),
            decoded: RwLock::new(DecodedStreams::with_budget(DECODED_BUDGET)),
            recovered_compressed: CompressedRecovery::default(),
            encryption: None,
            encrypt_object: None,
        }
    }

    /// Reads the trailer's `/Encrypt` entry and derives the file encryption key.
    ///
    /// Runs while `self.encryption` is still `None`, which is what keeps §7.6.2's second
    /// exception — "Any strings in an Encrypt dictionary" — true of the dictionary this
    /// reads. Anything else loaded along the way is dropped from the cache afterwards,
    /// because it was read before there was a key.
    fn authenticate(&mut self, password: &str) -> SyntaxResult<()> {
        let Some(entry) = self.trailer().get("Encrypt").cloned() else {
            return Ok(());
        };
        self.encrypt_object = entry.as_reference().map(|id| id.number);

        let Some(dict) = self.resolve(&entry).as_dict().cloned() else {
            // A trailer naming an `/Encrypt` that is not a dictionary tells us the file is
            // encrypted and refuses to say how. Treating it as plaintext would draw a page
            // of noise while reporting nothing.
            return Err(SyntaxError::UnsupportedEncryption {
                detail: "/Encrypt does not resolve to a dictionary (§7.6.1)".to_owned(),
            });
        };

        // §7.6.4.3.2 step (e) wants "the first element of the file's file identifier array",
        // which §7.6.2 also names as the one string the trailer never encrypts.
        let id_first = self
            .get_key(self.trailer(), "ID")
            .as_array()
            .and_then(|items| items.first().map(|item| self.resolve(item)))
            .and_then(|item| item.as_string().map(<[u8]>::to_vec))
            .unwrap_or_default();

        let encryption =
            Encryption::new(&dict, &id_first, password, &|object| self.resolve(object))?;
        write(&self.cache).clear();
        write(&self.expanded_streams).clear();
        // Anything decoded on the way to the key was decoded from ciphertext, so what is held
        // is a filter chain's opinion of bytes that were never plaintext. The two lines above
        // drop the objects for that reason and this one drops their decoded form.
        write(&self.decoded).clear();
        self.encryption = Some(encryption);
        Ok(())
    }

    /// Table 22's access permissions, or `None` when the document is not encrypted.
    ///
    /// Nothing here enforces them; see [`Permissions`] for why that is the clause's own
    /// arrangement rather than an omission.
    #[must_use]
    pub fn permissions(&self) -> Option<Permissions> {
        self.encryption.as_ref().map(Encryption::permissions)
    }

    /// Returns `true` if the document carries an `/Encrypt` dictionary.
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        self.encryption.is_some()
    }

    /// Returns the trailer dictionary.
    #[must_use]
    pub fn trailer(&self) -> &Dictionary {
        self.xref.trailer()
    }

    /// Returns the cross-reference table.
    #[must_use]
    pub fn xref(&self) -> &XrefTable {
        &self.xref
    }

    /// Returns `true` if the cross-reference table had to be rebuilt by scanning.
    #[must_use]
    pub fn was_recovered(&self) -> bool {
        self.xref.recovered_by_scan()
    }

    /// The object numbers whose cross-reference entry was disproved by the object at it.
    ///
    /// Empty for a well-formed document. A number in here was filed at an offset where a
    /// *different* object's header stands, and was found instead by its own — see
    /// [`Document::load_by_header`] for why the header wins. It is here so that the repair is
    /// answerable rather than silent: a caller that wants to tell a person their file is
    /// damaged can, and `pdf_syntax` still does not decide whether that is worth saying.
    ///
    /// Grows as objects are loaded, because nothing is parsed until it is asked for. Ask
    /// after the pages that matter have been read.
    #[must_use]
    pub fn misfiled_objects(&self) -> Vec<u32> {
        read(&self.misfiled).iter().copied().collect()
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
    /// see [`Self::recover_compressed_objects`].
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

    /// Returns the document catalogue.
    ///
    /// # Errors
    ///
    /// [`SyntaxError::TrailerMissing`] when the trailer has no `/Root`, or it does not
    /// resolve to a dictionary. Without a catalogue there is no page tree, so this is fatal
    /// rather than recoverable.
    pub fn catalog(&self) -> SyntaxResult<Dictionary> {
        let root = self
            .trailer()
            .get("Root")
            .cloned()
            .ok_or(SyntaxError::TrailerMissing { key: "/Root" })?;

        self.resolve(&root)
            .as_dict()
            .cloned()
            .ok_or(SyntaxError::TrailerMissing {
                key: "/Root (not a dictionary)",
            })
    }

    /// Fetches an indirect object, returning [`Object::Null`] if it cannot be read.
    ///
    /// Null rather than an error because the specification says a reference to a
    /// non-existent object *is* null. A missing object is therefore ordinary, not
    /// exceptional, and forcing every caller to handle a `Result` for it would bury the
    /// cases that matter.
    #[must_use]
    pub fn get(&self, id: ObjectId) -> Object {
        if let Some(cached) = read(&self.cache).get(&id.number) {
            return cached.clone();
        }

        // Loading an object can need other objects: an indirect `/Length`, an indirect
        // `/Filter`, or the object stream a compressed object lives in. A file may point
        // any of those at the object being loaded, and the resulting recursion is bounded
        // by nothing in the parser, so it is bounded here. Null is what §7.3.10 gives a
        // reference that resolves to nothing, which is what a cycle amounts to.
        if !self.begin_loading(id.number) {
            return Object::Null;
        }
        let object = self.load(id).unwrap_or(Object::Null);
        self.end_loading(id.number);

        write(&self.cache).insert(id.number, object.clone());
        object
    }

    /// Records that this thread has begun loading `number`, or reports a cycle.
    ///
    /// # Why the set is per thread
    ///
    /// It is a guard on the *call stack*, not on the document: `get` is re-entrant because
    /// loading an object can need an indirect `/Length`, an indirect `/Filter` or the object
    /// stream it lives in, and a file may point any of those back at the object being loaded.
    /// A set shared between threads would answer §7.3.10's null to a second thread merely
    /// because a first happened to be reading the same object — a wrong answer produced by
    /// timing, which is the one kind this program must not have. Two threads loading one
    /// object duplicate the parse instead, and the cache makes that the last time.
    fn begin_loading(&self, number: u32) -> bool {
        write(&self.loading)
            .entry(std::thread::current().id())
            .or_default()
            .insert(number)
    }

    /// Records that this thread has finished loading `number`.
    ///
    /// Empties are removed so that the map is bounded by the threads *currently* inside
    /// [`Self::get`] rather than by every thread that has ever touched the document.
    fn end_loading(&self, number: u32) {
        let mut loading = write(&self.loading);
        let thread = std::thread::current().id();
        if let Some(numbers) = loading.get_mut(&thread) {
            numbers.remove(&number);
            if numbers.is_empty() {
                loading.remove(&thread);
            }
        }
    }

    /// Resolves an object if it is a reference, following chains.
    ///
    /// A reference to a reference is unusual but legal. The chain is bounded by
    /// [`MAX_REFERENCE_DEPTH`], and a cycle resolves to null rather than looping.
    #[must_use]
    pub fn resolve(&self, object: &Object) -> Object {
        let mut current = object.clone();
        for _ in 0..MAX_REFERENCE_DEPTH {
            match current {
                Object::Reference(id) => current = self.get(id),
                other => return other,
            }
        }
        Object::Null
    }

    /// Looks up a key in a dictionary and resolves the result.
    ///
    /// The common operation by a wide margin: nearly every value in a PDF may be indirect,
    /// so a bare `dict.get` is almost always a bug waiting to happen.
    #[must_use]
    pub fn get_key(&self, dict: &Dictionary, key: &str) -> Object {
        dict.get(key)
            .map_or(Object::Null, |object| self.resolve(object))
    }

    /// Looks up one key **inside** an indirect object, without copying the rest of it.
    ///
    /// [`Self::get`] hands back a clone of the whole object, because that is the only thing a
    /// value behind a lock can be handed back as. Where a caller wants one entry of a large
    /// dictionary that is only a step on the way to somewhere else, the copy is the cost.
    /// ISO 32000-2 §7.7.3.2's `/Kids` is "an array of indirect references", so a page tree's
    /// walk steps over its neighbours, and asking each of them for Table 30's `/Count` through
    /// `get` deep-copies a page dictionary — its `/Annots`, its `/Resources`, all of it — to
    /// read one number. Over one sweep of ISO 32000-2 that was 17.1% of the whole search;
    /// ADR 0330 has the A/B.
    ///
    /// `None` says the object is **not a dictionary**, which is a different answer from
    /// [`Object::Null`] and is why this returns an `Option` where [`Self::get_key`] does not:
    /// a caller that must tell "this node has no such entry" from "this is not a node" cannot
    /// do it from a null, and the page tree is exactly such a caller — a `/Kids` entry naming
    /// something that is not a dictionary is not a page and must not be counted as one.
    ///
    /// The answer is [`Self::get_key`]'s in every other respect: the entry is resolved, and an
    /// absent one is [`Object::Null`].
    #[must_use]
    pub fn get_key_of(&self, id: ObjectId, key: &str) -> Option<Object> {
        // The guard is dropped before anything is resolved, because resolving can load another
        // object and that takes this same lock to write.
        let held = {
            let cache = read(&self.cache);
            match cache.get(&id.number) {
                Some(Object::Reference(_)) | None => Held::Unread,
                Some(object) => object.as_dict().map_or(Held::NotADictionary, |dict| {
                    Held::Entry(dict.get(key).cloned().unwrap_or(Object::Null))
                }),
            }
        };
        match held {
            Held::Entry(value) => Some(self.resolve(&value)),
            Held::NotADictionary => None,
            // Either nothing has parsed this object yet, or it is itself a reference and
            // §7.3.10's chain is `resolve`'s to follow. Both are the ordinary path, and the
            // copy it makes is made once per object rather than once per lookup.
            Held::Unread => self
                .resolve(&Object::Reference(id))
                .as_dict()
                .map(|dict| self.get_key(dict, key)),
        }
    }

    /// Loads an object from wherever the cross-reference table says it is.
    fn load(&self, id: ObjectId) -> Option<Object> {
        match self.xref.location(id.number)? {
            Location::Offset(offset) => {
                // A table pointing at the wrong object is a real corruption. Returning
                // another object's contents under this number would corrupt the document
                // graph silently, so what is at the offset is used only when it says it is
                // this object — and where it does not, the file's own headers are asked.
                match self.parse_at(offset) {
                    Some((found, object)) if found.number == id.number => Some(object),
                    _ => self.load_by_header(id),
                }
            }
            Location::InStream { stream, index } => {
                let expanded = self.expand_object_stream(stream)?;
                // Objects in a stream are addressed by their own number, and `index` is
                // only a hint about ordering, so the number is authoritative.
                expanded
                    .get(&id.number)
                    .cloned()
                    .or_else(|| expanded.values().nth(index as usize).cloned())
            }
        }
    }

    /// Finds an object by its own header, where the table's entry for it was disproved.
    ///
    /// # Why this is a repair and not a guess
    ///
    /// §7.5.4 says a cross-reference entry's offset gives "the number of bytes from the
    /// beginning of the PDF file to the beginning of the object", and §7.3.10 makes an
    /// indirect object begin with its own number:
    ///
    /// > The definition of an indirect object in a PDF file shall consist of its object number
    /// > and generation number (separated by white-space), followed by the value of the object
    /// > bracketed between the keywords obj and endobj
    ///
    /// So the file states where object 3 is twice, and where the two statements disagree the
    /// object's own header is the one written next to the bytes it describes. Taking it is
    /// the same move [`crate::xref::rebuild`] makes for a whole table and
    /// `pdf_model::Pages::new` makes for a page tree that walks to nothing: a recovery from
    /// the file's own declarations, never from another reader's behaviour.
    ///
    /// # What it deliberately does not do
    ///
    /// It runs only where an entry *exists* and is disproved. An object number the table
    /// does not mention, or mentions as **free**, names nothing — §7.5.6 makes a deletion the
    /// most recent statement about an object, and ADR 0100 is the session that stopped this
    /// reader resurrecting objects its own file had deleted. Scanning for a header there
    /// would undo exactly that. The caller's `?` on [`crate::xref::XrefTable::location`] is
    /// what keeps the two cases apart.
    ///
    /// # Cost
    ///
    /// One linear scan of the file, memoised, and only for a document that has already been
    /// found to be wrong about one of its objects. Two of the 974 corpus documents reach it.
    fn load_by_header(&self, id: ObjectId) -> Option<Object> {
        let headers = self.object_headers();
        let Location::Offset(offset) = headers.location(id.number)? else {
            return None;
        };
        let (found, object) = self.parse_at(offset)?;
        if found.number != id.number {
            return None;
        }
        write(&self.misfiled).insert(id.number);
        Some(object)
    }

    /// The file's own object headers, scanned once and remembered.
    fn object_headers(&self) -> Arc<XrefTable> {
        if let Some(headers) = read(&self.headers).as_ref() {
            return Arc::clone(headers);
        }
        let scanned = Arc::new(crate::xref::scan_for_objects(&self.bytes, self.limits));
        *write(&self.headers) = Some(Arc::clone(&scanned));
        scanned
    }

    /// Parses the indirect object at `offset` and decrypts it if the document is encrypted.
    ///
    /// The generation used is the one written in the file rather than the one the
    /// cross-reference table records, because §7.6.3.2 step (a) takes both numbers "from
    /// the object identifier of the string or stream to be encrypted" — that is, from the
    /// object as written.
    fn parse_at(&self, offset: usize) -> Option<(ObjectId, Object)> {
        let mut parser = Parser::at(&self.bytes, offset, self.limits);
        let (found, object) = parser.parse_indirect_object().ok()?;
        let object = self.with_stated_length(parser.stream_data_at(), object);
        Some((found, self.decrypt_object(found, object)))
    }

    /// Applies a `/Length` the parser could not follow, §7.3.8.2.
    ///
    /// > Every stream dictionary shall have a Length entry that indicates how many bytes of the
    /// > PDF file are used for the stream's data.
    ///
    /// Table 5 makes that entry "(Required; shall be an indirect reference)" for a stream whose
    /// length a producer does not know until it has written the data, and a **parser** cannot
    /// follow a reference: resolving one needs the document, and the document is built out of
    /// parsed objects. So [`Parser::parse_stream_data`] falls back to searching for `endstream`
    /// and trimming one end-of-line — the delimiter's, per §7.3.8.1's "there should be an
    /// end-of-line marker" — and where the producer wrote none, the byte it trims is the
    /// **data's**. That is a stream one byte short of what the file says it is, and for a
    /// `FlateDecode` stream the byte it loses is usually the last of RFC 1951's final block, so
    /// the stream reads as damaged while being whole. Two of the 65 944 crawled documents are
    /// exactly that, and finding them is what ADR 0366's object-stream rule did first.
    ///
    /// This is where the file's own statement is applied, one layer up, under the same guard the
    /// parser puts on a direct length: the stated end is taken only where `endstream` is actually
    /// there, so a *wrong* `/Length` still loses to the search. §7.3.8.2's "[a]ll of these
    /// constraints shall be consistent" is what makes that check the right arbiter either way.
    fn with_stated_length(&self, data_at: Option<usize>, object: Object) -> Object {
        let Object::Stream(stream) = &object else {
            return object;
        };
        // Only an indirect one: a direct `/Length` is the parser's own business and it has
        // already decided, with more context than this has.
        if !matches!(stream.dict.get("Length"), Some(Object::Reference(_))) {
            return object;
        }
        let Some(start) = data_at else {
            return object;
        };
        let Some(stated) = self
            .get_key(&stream.dict, "Length")
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())
        else {
            return object;
        };
        if stated == stream.data.len() || stated > self.limits.max_stream_len {
            return object;
        }
        let Some(end) = start.checked_add(stated) else {
            return object;
        };
        if end > self.bytes.len() || !crate::parser::endstream_follows(&self.bytes, end) {
            return object;
        }
        let Some(data) = self.bytes.get(start..end) else {
            return object;
        };
        Object::Stream(Arc::new(Stream {
            dict: stream.dict.clone(),
            data: Arc::from(data),
            decryption_failed: stream.decryption_failed,
        }))
    }

    /// Applies §7.6.2 to one freshly parsed indirect object.
    fn decrypt_object(&self, id: ObjectId, object: Object) -> Object {
        let Some(encryption) = self.encryption.as_ref() else {
            return object;
        };
        // "Any strings in an Encrypt dictionary" are exempt. The dictionary is reached
        // through the trailer, so its own object number is the whole of the exception.
        if Some(id.number) == self.encrypt_object {
            return object;
        }
        self.decrypt_value(encryption, id, object)
    }

    /// Decrypts every string and stream inside one indirect object.
    ///
    /// Recursion is bounded by the parser: an object graph deeper than [`Limits::max_depth`]
    /// never reaches here, because it was refused at parse time.
    fn decrypt_value(&self, encryption: &Encryption, id: ObjectId, object: Object) -> Object {
        match object {
            Object::String(bytes) => Object::String(
                encryption
                    .decrypt(encryption.string_method(), id, &bytes)
                    // A string whose ciphertext is malformed has no recoverable value, and
                    // handing back the ciphertext would put binary noise into text
                    // extraction — a wrong answer dressed as a right one.
                    .unwrap_or_default()
                    .into(),
            ),
            Object::Array(items) => Object::Array(
                items
                    .into_iter()
                    .map(|item| self.decrypt_value(encryption, id, item))
                    .collect(),
            ),
            Object::Dictionary(dict) => {
                Object::Dictionary(self.decrypt_dict(encryption, id, &dict))
            }
            Object::Stream(stream) => {
                let dict = self.decrypt_dict(encryption, id, &stream.dict);
                let method = self.stream_method(encryption, &stream.dict);
                let decrypted = encryption.decrypt(method, id, &stream.data);
                let decryption_failed = decrypted.is_none();
                let data = decrypted.map_or_else(|| Arc::from(&[][..]), Arc::from);
                Object::Stream(Arc::new(Stream {
                    dict,
                    data,
                    decryption_failed,
                }))
            }
            other => other,
        }
    }

    /// Decrypts a dictionary's values, honouring §7.6.2's signature exception.
    fn decrypt_dict(&self, encryption: &Encryption, id: ObjectId, dict: &Dictionary) -> Dictionary {
        let signature = is_signature_dictionary(dict);
        let mut out = Dictionary::new();
        for (key, value) in dict.iter() {
            let decrypted = if signature && key.as_bytes() == b"Contents" {
                value.clone()
            } else {
                self.decrypt_value(encryption, id, value.clone())
            };
            out.insert(key.clone(), decrypted);
        }
        out
    }

    /// Applies §7.6.2 to one indirect object on the way *out*, for §7.5.6's writer.
    ///
    /// The mirror of [`Self::decrypt_object`], and deliberately the same three decisions:
    /// which strings the clause exempts, which method a stream's own dictionary selects, and
    /// which object number is the encryption dictionary's. Writing those rules a second time
    /// beside the writer would be two statements of one clause, and the one place they can
    /// disagree is the one place a file becomes unreadable by its own producer.
    ///
    /// Returns `None` when the cipher refuses — a document opened without a key, or a key
    /// length no method accepts. The caller turns that into a named error rather than
    /// writing plaintext into an encrypted file, which would produce objects every reader
    /// including this one decrypts into noise.
    pub(crate) fn encrypt_for_update(&self, id: ObjectId, object: &Object) -> Option<Object> {
        let Some(encryption) = self.encryption.as_ref() else {
            return Some(object.clone());
        };
        // §7.6.2's "Any strings in an Encrypt dictionary", which is why an update may
        // rewrite one — it goes out exactly as it came in.
        if Some(id.number) == self.encrypt_object {
            return Some(object.clone());
        }
        self.encrypt_value(encryption, id, object)
    }

    /// Encrypts every string and stream inside one indirect object.
    fn encrypt_value(
        &self,
        encryption: &Encryption,
        id: ObjectId,
        object: &Object,
    ) -> Option<Object> {
        match object {
            Object::String(bytes) => Some(Object::String(
                encryption
                    .encrypt(encryption.string_method(), id, bytes)?
                    .into(),
            )),
            Object::Array(items) => items
                .iter()
                .map(|item| self.encrypt_value(encryption, id, item))
                .collect::<Option<Vec<_>>>()
                .map(Object::Array),
            Object::Dictionary(dict) => self
                .encrypt_dict(encryption, id, dict)
                .map(Object::Dictionary),
            Object::Stream(stream) => {
                let mut dict = self.encrypt_dict(encryption, id, &stream.dict)?;
                let method = self.stream_method(encryption, &stream.dict);
                let data = encryption.encrypt(method, id, &stream.data)?;
                // §7.3.8.2's `/Length` is "[t]he number of bytes from the beginning of the
                // line following the keyword stream to the last byte just before the keyword
                // endstream" — the encoded form as it sits in the file — and AES makes that
                // longer than the plaintext by an initialisation vector and a pad. A file
                // whose `/Length` still described the plaintext would end the stream inside
                // its own ciphertext.
                dict.insert(
                    Name::new(&b"Length"[..]),
                    Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
                );
                Some(Object::Stream(Arc::new(Stream {
                    dict,
                    data: Arc::from(data),
                    decryption_failed: false,
                })))
            }
            other => Some(other.clone()),
        }
    }

    /// Encrypts a dictionary's values, honouring §7.6.2's signature exception.
    fn encrypt_dict(
        &self,
        encryption: &Encryption,
        id: ObjectId,
        dict: &Dictionary,
    ) -> Option<Dictionary> {
        let signature = is_signature_dictionary(dict);
        let mut out = Dictionary::new();
        for (key, value) in dict.iter() {
            let encrypted = if signature && key.as_bytes() == b"Contents" {
                value.clone()
            } else {
                self.encrypt_value(encryption, id, value)?
            };
            out.insert(key.clone(), encrypted);
        }
        Some(out)
    }

    /// Chooses the crypt filter for one stream's data.
    ///
    /// §7.6.6 states the `/Crypt` override; the two exclusions are Table 20's, in §7.6.2:
    ///
    /// > All streams in the document, except for cross-reference streams … or streams that
    /// > have a Crypt entry in their Filter array …, shall be decrypted by the security
    /// > handler, using this crypt filter.
    ///
    /// An embedded file stream (§7.11.4) is the one kind with a default of its own, Table
    /// 20's `/EFF`, and the order below is the entry's own: it applies to embedded file
    /// streams "that do not have their own crypt filter specifier", so the `/Crypt` filter
    /// is asked first and `/EFF` decides what is left. §7.6.6 puts a related file under the
    /// same filter — "related files ( RF ) shall use the same crypt filter as the embedded
    /// file ( EF )" — which holds here by construction, since both are `/Type
    /// /EmbeddedFile` streams and neither is reached by any other route.
    fn stream_method(&self, encryption: &Encryption, dict: &Dictionary) -> Method {
        let stream_type = self
            .get_key(dict, "Type")
            .as_name()
            .map(|name| name.as_bytes().to_vec());
        match stream_type.as_deref() {
            // A cross-reference stream has to be readable before any key exists, so it is
            // never encrypted.
            Some(b"XRef") => return Method::Identity,
            // Table 21's `/EncryptMetadata`, which §14.3.2's metadata stream is the subject
            // of.
            Some(b"Metadata") if !encryption.encrypt_metadata() => return Method::Identity,
            _ => {}
        }

        // §7.6.6: the `/Crypt` filter's own `/DecodeParms` names the filter to use, "if
        // missing, Identity is used".
        let filters = self.filter_chain(dict);
        if let Some(index) = filters.iter().position(|name| name == b"Crypt") {
            let named = self
                .decode_parms(dict, index)
                .and_then(|parms| self.get_key(&parms, "Name").as_name().cloned());
            return named.map_or(Method::Identity, |name| encryption.named_method(&name));
        }

        if stream_type.as_deref() == Some(b"EmbeddedFile") {
            return encryption.embedded_file_method();
        }

        encryption.stream_method()
    }

    /// Expands an object stream into its contained objects.
    fn expand_object_stream(&self, number: u32) -> Option<Arc<BTreeMap<u32, Object>>> {
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
    fn recover_compressed_objects(&mut self) {
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

    /// Returns a stream's decoded data.
    ///
    /// # Errors
    ///
    /// Returns `None` when a filter in the chain is not supported, rather than returning
    /// the encoded bytes. Handing back compressed data as if it were decoded would produce
    /// garbage that looks like a rendering bug. Also for a stream whose data lives in an
    /// external file — see [`Self::is_external`]. [`Self::decoded_stream_data_reported`] says
    /// which of [`StreamRefusal`]'s answers it was.
    #[must_use]
    pub fn decoded_stream_data(&self, stream: &Stream) -> Option<Arc<[u8]>> {
        self.decoded_stream_data_reported(stream)
            .ok()
            .map(|decoded| decoded.data)
    }

    /// The same, naming what refused.
    ///
    /// The distinction a caller needs is [`StreamRefusal::TooLarge`]: a stream this reader
    /// *could* have decoded and declined to, which is a statement about the file rather than
    /// about this program's filter table. Reporting it as an unsupported filter would be the
    /// silent-fallback failure one layer up from the one ADR 0306 removed.
    ///
    /// The second distinction is [`Decoded::damage`], which is not a refusal at all: bytes
    /// came out, and they stop short of what the file says the stream is. ADR 0343.
    ///
    /// # Errors
    ///
    /// [`StreamRefusal`], whose variants are the reasons.
    pub fn decoded_stream_data_reported(&self, stream: &Stream) -> Result<Decoded, StreamRefusal> {
        if stream.decryption_failed {
            return Err(StreamRefusal::DecryptionFailed);
        }
        if Self::is_external(stream) {
            return Err(StreamRefusal::External);
        }
        let filters = self.filter_chain(&stream.dict);
        if filters.is_empty() || self.states_no_data(stream) {
            return Ok(Decoded {
                data: Arc::clone(&stream.data),
                damage: None,
            });
        }

        // The parameters are read before the loop rather than inside it, because they are half
        // of what identifies a decode and the memo below has to be keyed on all of it. It is
        // the same work in the same order — `decode_parms` was already called once per filter.
        let chain: Vec<(Vec<u8>, Option<Dictionary>)> = filters
            .into_iter()
            .enumerate()
            .map(|(index, filter)| (filter, self.decode_parms(&stream.dict, index)))
            .collect();

        if let Some(held) = write(&self.decoded).get(&stream.data, &chain) {
            return Ok(held);
        }

        let mut data: Arc<[u8]> = Arc::clone(&stream.data);
        // The *first* damage in the chain is the one kept, because it is the one that caused
        // the rest: a stage fed a truncated prefix has no way to end well either, and naming
        // the last stage's complaint would describe the symptom rather than the file.
        let mut damage = None;
        for (filter, parms) in &chain {
            let stage = crate::filter::decode_with_parms_reported(
                filter,
                &data,
                parms.as_ref(),
                self.limits,
            )
            .map_err(|why| StreamRefusal::Filter {
                name: String::from_utf8_lossy(filter).into_owned(),
                why,
            })?;
            data = stage.data;
            damage = damage.or(stage.damage);
        }
        let decoded = Decoded { data, damage };
        write(&self.decoded).put(&stream.data, chain, &decoded);
        Ok(decoded)
    }

    /// What this document's decoded-stream cache is holding, and how it has been used.
    ///
    /// An instrument rather than something a reader draws with; `viewer-core/examples/find_cost`
    /// prints it after a sweep. See [`DecodedStreams`] for what the numbers mean.
    #[must_use]
    pub fn decoded_streams(&self) -> DecodedStreamCache {
        read(&self.decoded).report()
    }

    /// Whether the file *states* that this stream holds nothing, ISO 32000-2 §7.3.8.1.
    ///
    /// > A stream shall consist of a dictionary followed by zero or more bytes bracketed between
    /// > the keywords stream (followed by newline) and endstream :
    ///
    /// So an empty stream is conforming, and Table 5's `/Filter` names the filters "that shall be
    /// applied in processing the stream data found between the keywords stream and endstream ".
    /// With no data found there, there is nothing for a filter to process and the decoded result
    /// is the empty sequence — which is *not* the same as a filter refusing its input, and the
    /// difference is a report a page either does or does not deserve. `FlateDecode` refuses zero
    /// bytes, because RFC 1950 gives a zlib stream a six-byte floor, so without this a page whose
    /// producer wrote `<< /Filter /FlateDecode /Length 0 >>` was reported as missing drawing that
    /// an empty part cannot be missing.
    ///
    /// **Both halves of the condition are load-bearing, and the second is the whole argument.**
    /// A stream *truncated* to nothing also arrives here holding no bytes: [`crate::Parser`]
    /// recovers a wrong `/Length` by searching for `endstream`, so a file cut off mid-stream
    /// yields an empty slice as readily as an empty stream does. §7.3.8.2 tells the two apart —
    ///
    /// > Every stream dictionary shall have a Length entry that indicates how many bytes of the
    /// > PDF file are used for the stream's data.
    ///
    /// — because a truncation leaves `/Length` stating a number the bytes do not support, and
    /// "[a]ll of these constraints shall be consistent" is then false. Only a stated zero that the
    /// bytes agree with is silence the producer asked for. Two documents of the 5944 `SafeDocs`
    /// members on disk do this and two more are truncations; ADR 0266 names all four.
    ///
    /// Deliberately not applied on [`Self::image_stream`]'s path: §7.3.8.2 also says "streams are
    /// used to represent many objects from whose attributes a length can be inferred", and an
    /// image's `/Width`, `/Height` and `/BitsPerComponent` infer one, so for an image a stated
    /// zero contradicts the dictionary rather than agreeing with it.
    fn states_no_data(&self, stream: &Stream) -> bool {
        stream.data.is_empty()
            && self
                .get_key(&stream.dict, "Length")
                .as_integer()
                .is_some_and(|length| length == 0)
    }

    /// Returns a stream's data with every filter applied up to a trailing image codec.
    ///
    /// An image stream's chain may mix ordinary filters with a codec:
    /// `[/ASCIIHexDecode /JBIG2Decode]` is the arrangement ISO 32000-2 §7.4.7's own worked
    /// example uses, and `[/FlateDecode /DCTDecode]` occurs in the corpus. Only the last
    /// entry can be a codec — everything before it is a byte-to-byte transformation that
    /// has to run first, and a codec handed still-compressed bytes fails in a way that
    /// reads as a broken image rather than as a missing step.
    ///
    /// # Errors
    ///
    /// Returns `None` when a filter before the codec is unsupported, for the same reason
    /// [`Self::decoded_stream_data`] does.
    #[must_use]
    pub fn image_stream(&self, stream: &Stream) -> Option<ImageStream> {
        if stream.decryption_failed || Self::is_external(stream) {
            return None;
        }
        let filters = self.filter_chain(&stream.dict);
        let codec_at = filters.len().checked_sub(1).filter(|last| {
            filters
                .get(*last)
                .is_some_and(|name| crate::filter::is_image_codec(name))
        });

        let mut data: Arc<[u8]> = Arc::clone(&stream.data);
        for (index, filter) in filters.iter().enumerate() {
            if Some(index) == codec_at {
                break;
            }
            let parms = self.decode_parms(&stream.dict, index);
            data = crate::filter::decode_with_parms(filter, &data, parms.as_ref(), self.limits)?;
        }

        Some(ImageStream {
            codec: codec_at.and_then(|index| filters.get(index).cloned()),
            parms: codec_at.and_then(|index| self.decode_parms(&stream.dict, index)),
            data,
        })
    }

    /// Whether a stream's data lives in an external file, ISO 32000-2 §7.3.8.1.
    ///
    /// > Alternatively, beginning with PDF 1.2, the bytes may be contained in an external
    /// > file, in which case the stream dictionary specifies the file, and any bytes between
    /// > stream and endstream shall be ignored by a PDF processor.
    ///
    /// So such a stream has **no usable data here**, and returning the embedded bytes would
    /// be drawing exactly what the clause says to ignore. The renderer has no filesystem
    /// (`CLAUDE.md` principle 3, and ADR 0014's sandbox), so it cannot fetch the file
    /// either; the honest answer is the one every unsupported stream already gets, which is
    /// a refusal its caller reports. Table 5's `/FFilter` and `/FDecodeParms` describe the
    /// external data's own filters and are unread for the same reason.
    ///
    /// Not one of the 974 corpus documents writes one, measured rather than assumed — which
    /// is why this is a rule that only reading §7.3.8 could have found.
    #[must_use]
    pub fn is_external(stream: &Stream) -> bool {
        // A direct lookup, not `get_key`: `/F` is a file specification, which may be a
        // string or a dictionary, and its *presence* is what Table 5 conditions on. An
        // indirect one is a reference, which is equally present.
        stream.dict.get("F").is_some()
    }

    /// Returns the `/DecodeParms` entry for the filter at `index`.
    ///
    /// The key may hold a single dictionary or an array with one entry per filter, and
    /// either may be indirect.
    fn decode_parms(&self, dict: &Dictionary, index: usize) -> Option<Dictionary> {
        match self.get_key(dict, "DecodeParms") {
            Object::Dictionary(parms) => Some(parms),
            Object::Array(items) => items
                .get(index)
                .map(|item| self.resolve(item))
                .and_then(|item| item.as_dict().cloned()),
            _ => None,
        }
    }

    /// Returns the filter names for a stream, in application order.
    fn filter_chain(&self, dict: &Dictionary) -> Vec<Vec<u8>> {
        let filter = self.get_key(dict, "Filter");
        match filter {
            Object::Name(name) => vec![name.as_bytes().to_vec()],
            Object::Array(items) => items
                .iter()
                .map(|item| self.resolve(item))
                .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// How a stream's decoded bytes are to be obtained: whole, or a window at a time.
    ///
    /// [`Self::decoded_stream_data_reported`] answers "what does this stream decode to", which
    /// is a question with an allocation the size of the answer in it. This one answers "how
    /// shall I read it", and for the one chain a window can pump — a single `FlateDecode` with
    /// no predictor, which is what the overwhelming majority of content streams state — the
    /// answer is a [`crate::filter::Pump`] that never holds more than the reader's window.
    /// ADR 0365, and `doc/todo/14` for the road.
    ///
    /// Every other chain comes back whole, decoded by exactly the route it always took, cache
    /// and bound included. That is a route decision and never a silence: the bytes a caller
    /// gets are the same bytes either way, and a stream this reader cannot decode is refused
    /// here as loudly as it is there.
    ///
    /// # Errors
    ///
    /// [`StreamRefusal`], whose variants are the reasons, exactly as
    /// [`Self::decoded_stream_data_reported`] gives them.
    pub fn stream_source(&self, stream: &Stream) -> Result<StreamSource, StreamRefusal> {
        if stream.decryption_failed {
            return Err(StreamRefusal::DecryptionFailed);
        }
        if Self::is_external(stream) {
            return Err(StreamRefusal::External);
        }
        let filters = self.filter_chain(&stream.dict);
        if filters.is_empty() || self.states_no_data(stream) {
            return Ok(StreamSource::Whole(Decoded {
                data: Arc::clone(&stream.data),
                damage: None,
            }));
        }
        // A predictor is part of decoding (see [`crate::filter::decode_with_parms`]) and reverses
        // rows against their predecessors, which is not a transformation a window can apply to a
        // few thousand bytes at a time. It occurs on cross-reference streams and images rather
        // than on content streams, and either way this is a route decision: such a stream is
        // decoded whole below.
        let predicted = self
            .decode_parms(&stream.dict, 0)
            .and_then(|parms| parms.get("Predictor").and_then(Object::as_integer))
            .is_some_and(|predictor| predictor > 1);
        if !predicted
            && let [only] = filters.as_slice()
            && matches!(only.as_slice(), b"FlateDecode" | b"Fl")
        {
            return Ok(StreamSource::Pumped(crate::filter::Pump::inflating(
                Arc::clone(&stream.data),
            )));
        }
        self.decoded_stream_data_reported(stream)
            .map(StreamSource::Whole)
    }

    /// Returns the bytes the document was opened from.
    #[must_use]
    pub fn bytes(&self) -> &Arc<[u8]> {
        &self.bytes
    }

    /// Returns the resource bounds in force.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }
}

/// Whether §7.6.2's fourth exception applies to this dictionary's `/Contents`.
///
/// > Any hexadecimal strings representing the value of the Contents key in a Signature
/// > dictionary
///
/// §12.8.1's Table 255 requires a signature's `/Contents` to be a hexadecimal string, so
/// the qualifier describes the value rather than narrowing the exception, and what has to
/// be recognised is the dictionary. A signature dictionary has no key of its own — both it
/// and a document time-stamp are reached from a form field's `/V` — so it is recognised by
/// what it says about itself.
///
/// **`/Type` is not the only thing that identifies one, and reading it as though it were cost
/// `issue17069.pdf` its signature value for as long as this predicate existed.** Table 255
/// makes the entry "(Optional if Sig; Required if `DocTimeStamp`)" and states "[t]he default
/// value is: Sig ." — so a dictionary that omits it *is* a signature dictionary, and that
/// document is one: encrypted, with a `/ByteRange`, a `/Contents` and no `/Type` at all. The
/// exception did not apply, the 33 680-byte signature value went through the cipher, and what
/// came back was empty.
///
/// What identifies one with no `/Type` is what Table 255 requires of every signature carrying
/// a byte range digest: `/ByteRange`, "[a]n array of pairs of integers", and `/Contents`, the
/// signature value. Nothing else in ISO 32000-2 has a `/ByteRange`.
fn is_signature_dictionary(dict: &Dictionary) -> bool {
    match dict
        .get("Type")
        .and_then(Object::as_name)
        .map(Name::as_bytes)
    {
        Some(b"Sig" | b"DocTimeStamp") => true,
        // A dictionary that says it is something else is taken at its word.
        Some(_) => false,
        None => {
            matches!(dict.get("ByteRange"), Some(Object::Array(_)))
                && matches!(dict.get("Contents"), Some(Object::String(_)))
        }
    }
}

/// One stream's decoded bytes, and what proves the key still names that stream.
struct DecodedEntry {
    /// The encoded bytes the key was taken from, held for as long as the entry is.
    ///
    /// **This field is the whole soundness argument and it is not a copy of anything.** The key
    /// is an address, and an address is only an identity while the allocation lives: a freed
    /// buffer's address is handed to the next allocation, which would make a lookup for a
    /// *different* stream find these bytes. Holding the `Arc` makes that impossible — the
    /// allocation cannot be freed while the entry that names it exists, so no two live entries
    /// can share a key and nothing allocated after an entry can collide with it. `doc/todo/41`
    /// records the census where exactly this went wrong: below 4 KB its counts were worthless
    /// "[because] an address freed with one document is handed to the next".
    encoded: Arc<[u8]>,
    /// §7.4's filter chain, with its parameters, as it was when these bytes were produced.
    ///
    /// Compared on every hit. One `Arc<[u8]>` of encoded bytes can be shared by two `Stream`s
    /// with different dictionaries — `pdf_model::thumbnail::significant` builds one such — and
    /// two chains over one buffer are two different decodes.
    chain: Vec<(Vec<u8>, Option<Dictionary>)>,
    /// What that chain produced.
    data: Arc<[u8]>,
    /// Whether it produced all of it, memoised with the bytes.
    ///
    /// Held rather than re-derived because it is a property of the *decode* and the decode is
    /// what this cache exists to avoid running twice. A hit that answered `None` here would
    /// make a damaged stream report on its first reading and stay silent on every later one,
    /// which is a report that depends on the cache's budget. ADR 0343.
    damage: Option<Damage>,
    /// The value of [`DecodedStreams::clock`] when this entry was last read or written.
    used: u64,
}

/// The decoded streams one open document is holding, under a byte budget.
///
/// # Why a document memoises this at all
///
/// A resource is decoded once per *use*, not once per document: a font program referenced by
/// eight hundred pages is inflated eight hundred times, and one sweep of ISO 32000-2 spends
/// 24.6% of its wall clock in [`Document::decoded_stream_data`] of which 23.4% is decoding
/// something it had already decoded — 830 MB of re-inflation against 46 MB of first decodes.
/// `doc/todo/41` priced this at 0.7% over the pdf.js corpus and was right about that
/// population: a corpus interpreted one page per document has nothing to repeat. What repeats
/// is what a *reader* does — turning pages, and searching a document from end to end.
///
/// # The lock, and why a hit takes the exclusive one
///
/// [`Document::get`] is asked 829 times a page and this is asked 12; a hit updates the
/// recency stamp, so it takes the write lock, which adds 12 586 exclusive acquisitions to a
/// sweep that already takes 61 836 for the object cache (ADR 0260's first table). Keeping
/// recency in an atomic so that a hit could share the read lock would buy a parallel sweep
/// something and cost every reader a less obvious structure than this one; the viewer sweeps on
/// one thread, and ADR 0260 declined the parallel sweep on memory rather than on locking.
struct DecodedStreams {
    /// What is held, by the address and length of the encoded bytes.
    held: HashMap<(usize, usize), DecodedEntry>,
    /// How many bytes that is, counted as it changes rather than summed on every insertion.
    bytes: usize,
    /// The ceiling those bytes are held under.
    budget: usize,
    /// A counter that only goes up, which is what "least recently used" is ordered by.
    clock: u64,
    /// How many lookups since the document opened were answered without running a filter.
    hits: u64,
    /// How many were not.
    misses: u64,
    /// How many entries the budget has dropped.
    evicted: u64,
}

/// The address and length of some encoded bytes, which is what a decode is keyed by.
///
/// Two values, because a length is a cheap disagreement to find and the address alone is what
/// [`DecodedEntry::encoded`] has to work to make trustworthy.
fn allocation(data: &Arc<[u8]>) -> (usize, usize) {
    (Arc::as_ptr(data).cast::<u8>().addr(), data.len())
}

impl DecodedStreams {
    /// A cache holding at most `budget` bytes.
    ///
    /// The budget is a parameter so that eviction can be exercised on a budget of a few bytes,
    /// and so that a measurement can build the same tree with the cache off; every document
    /// opens with [`DECODED_BUDGET`].
    fn with_budget(budget: usize) -> Self {
        Self {
            held: HashMap::new(),
            bytes: 0,
            budget,
            clock: 0,
            hits: 0,
            misses: 0,
            evicted: 0,
        }
    }

    /// What this chain produced from these bytes before, marking it most recently used.
    fn get(
        &mut self,
        encoded: &Arc<[u8]>,
        chain: &[(Vec<u8>, Option<Dictionary>)],
    ) -> Option<Decoded> {
        self.clock = self.clock.saturating_add(1);
        let clock = self.clock;
        let entry = self.held.get_mut(&allocation(encoded));
        let Some(entry) = entry.filter(|entry| entry.chain == chain) else {
            self.misses = self.misses.saturating_add(1);
            return None;
        };
        entry.used = clock;
        self.hits = self.hits.saturating_add(1);
        Some(Decoded {
            data: Arc::clone(&entry.data),
            damage: entry.damage,
        })
    }

    /// Keeps a decode, dropping least-recently-used entries until it fits.
    ///
    /// The charge is the decoded bytes **plus the encoded ones**, because the entry holds both
    /// and an accounting that ignored the pin would understate what the cache costs for any
    /// stream the document's object cache is not already holding.
    ///
    /// A decode too large for the whole budget is not kept, rather than emptying the cache to
    /// hold one entry that the next insertion would drop.
    fn put(
        &mut self,
        encoded: &Arc<[u8]>,
        chain: Vec<(Vec<u8>, Option<Dictionary>)>,
        decoded: &Decoded,
    ) {
        let data = &decoded.data;
        let size = data.len().saturating_add(encoded.len());
        if size > self.budget {
            return;
        }
        self.clock = self.clock.saturating_add(1);
        let key = allocation(encoded);
        if let Some(previous) = self.held.remove(&key) {
            self.bytes = self
                .bytes
                .saturating_sub(previous.data.len().saturating_add(previous.encoded.len()));
        }
        while self.bytes.saturating_add(size) > self.budget {
            if !self.evict() {
                return;
            }
        }
        self.bytes = self.bytes.saturating_add(size);
        self.held.insert(
            key,
            DecodedEntry {
                encoded: Arc::clone(encoded),
                chain,
                data: Arc::clone(data),
                damage: decoded.damage,
                used: self.clock,
            },
        );
    }

    /// Drops the least recently used entry, answering whether there was one.
    ///
    /// A scan rather than a second index ordered by use, for the reason `viewer-core`'s readback
    /// cache gives: two maps kept in step are more lines and one more thing to get wrong, and
    /// this walks a few thousand `u64`s against the inflation that filled the entry it drops.
    fn evict(&mut self) -> bool {
        let Some(oldest) = self
            .held
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(key, _)| *key)
        else {
            return false;
        };
        if let Some(entry) = self.held.remove(&oldest) {
            self.bytes = self
                .bytes
                .saturating_sub(entry.data.len().saturating_add(entry.encoded.len()));
            self.evicted = self.evicted.saturating_add(1);
        }
        true
    }

    /// Forgets everything, keeping the tally of what has happened.
    fn clear(&mut self) {
        self.held.clear();
        self.bytes = 0;
    }

    /// What this cache is holding, for a caller that wants to say so.
    fn report(&self) -> DecodedStreamCache {
        DecodedStreamCache {
            streams: self.held.len(),
            bytes: self.bytes,
            budget: self.budget,
            hits: self.hits,
            misses: self.misses,
            evicted: self.evicted,
        }
    }
}

/// What one open document's decoded-stream cache is holding, and how it has been used.
///
/// The bound this project asks of a memory budget is that it be *legible* rather than small,
/// so the number can be read off. Reported by [`Document::decoded_streams`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedStreamCache {
    /// How many decoded streams are held.
    pub streams: usize,
    /// How many bytes that is, counting each entry's encoded bytes as well as its decoded ones.
    pub bytes: usize,
    /// The ceiling those bytes are held under, which is [`DECODED_BUDGET`] for every document.
    pub budget: usize,
    /// How many lookups since the document opened were answered without running a filter.
    pub hits: u64,
    /// How many were not.
    pub misses: u64,
    /// How many entries the budget has dropped.
    pub evicted: u64,
}

/// Why a stream's data could not be handed over decoded.
///
/// Three answers to one question, and only [`Document::decoded_stream_data_reported`] keeps
/// them apart. The one that matters is [`Self::Filter`] carrying
/// [`FilterRefusal::TooLarge`]: everything else is something this reader cannot do, and that
/// one is something it declined to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamRefusal {
    /// §7.6's decryption did not produce plaintext for this stream.
    DecryptionFailed,
    /// §7.3.8.1's external file, which the renderer has no filesystem to open (principle 3).
    External,
    /// A filter in the chain refused.
    Filter {
        /// Which filter, as the file names it.
        name: String,
        /// What it answered.
        why: FilterRefusal,
    },
}

/// How a caller is to read a stream's decoded bytes, from [`Document::stream_source`].
///
/// The two arms are one decision and not two behaviours: the bytes are the same bytes, and
/// which arm a stream takes says only whether they exist all at once. A reader that wants the
/// whole thing — a font program, an image, an ICC profile, a cross-reference stream — asks
/// [`Document::decoded_stream_data`] and never sees this type.
#[derive(Debug)]
pub enum StreamSource {
    /// Decoded, whole, by the route every other caller takes.
    Whole(Decoded),
    /// A pump: the bytes come out a window at a time and are never all resident.
    Pumped(crate::filter::Pump),
}

/// A stream's data with its image codec, if any, still to be applied.
///
/// Returned by [`Document::image_stream`]. The split exists because a codec's output is a
/// raster rather than bytes, so it cannot be the return value of a filter chain.
#[derive(Debug, Clone)]
pub struct ImageStream {
    /// The image codec left on the data, by name, or `None` if the chain was all ordinary
    /// filters and the data is already samples.
    pub codec: Option<Vec<u8>>,
    /// The codec's own `/DecodeParms`, which is where `/JBIG2Globals` lives (Table 12).
    pub parms: Option<Dictionary>,
    /// The data with every filter before the codec applied.
    pub data: Arc<[u8]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document that two threads may read at once, which is the point of the `RwLock`s.
    ///
    /// A compile-time assertion first, because `Sync` is the property the whole of ADR 0260
    /// rests on and a stray `Rc` anywhere in the object graph would silently take it away;
    /// then a real race, because the interesting half is not the marker trait but that eight
    /// threads asking one document for one object all get the object.
    #[test]
    fn a_document_is_readable_from_several_threads_at_once() {
        const fn assert_shareable<T: Send + Sync>() {}
        assert_shareable::<Document>();

        let body = b"%PDF-1.7\n\
                     1 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n\
                     2 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
                     trailer\n<< /Root 2 0 R >>\n";
        let document = Document::open(body.to_vec()).expect("the file is openable");
        let catalog = document.catalog().expect("the catalogue is reachable");

        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for _ in 0..200 {
                        let pages = document.get_key(&catalog, "Pages");
                        assert!(
                            pages.as_dict().is_some(),
                            "a thread got {} where the page tree is",
                            pages.type_name()
                        );
                    }
                });
            }
        });
    }

    /// One entry read out of an object answers exactly what reading the whole object answers.
    ///
    /// [`Document::get_key_of`] exists only to avoid a copy, so the property worth pinning is
    /// that it changes no answer — over an object read for the first time and over the same
    /// one once the cache holds it, because those are two different paths through it. The
    /// three cases that are not a plain dictionary are what its `Option` is for: a reference
    /// to a reference, which §7.3.10's chain makes ordinary; an object that is not a
    /// dictionary at all, which is `None` rather than a null; and an object number the file
    /// never defines, which is the same `None`.
    #[test]
    fn one_entry_of_an_object_reads_the_same_as_the_whole_of_it() {
        let body = b"%PDF-1.7\n\
                     1 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
                     2 0 obj\n<< /Type /Catalog /Pages 1 0 R >>\nendobj\n\
                     3 0 obj\n<< /Type /Page /Parent 1 0 R /Rotate 4 0 R >>\nendobj\n\
                     4 0 obj\n90\nendobj\n\
                     5 0 obj\n1 0 R\nendobj\n\
                     6 0 obj\n[1 2 3]\nendobj\n\
                     trailer\n<< /Root 2 0 R >>\n";
        let document = Document::open(body.to_vec()).expect("the file is openable");
        let whole = |number: u32, key: &str| {
            document
                .get(ObjectId::new(number, 0))
                .as_dict()
                .map(|dict| document.get_key(dict, key))
        };
        let entry = |number: u32, key: &str| document.get_key_of(ObjectId::new(number, 0), key);

        // Object 1 has not been parsed yet, so this is the loading path; asking again is the
        // cached one, and both must agree with reading the object whole.
        assert_eq!(
            entry(1, "Count").as_ref().and_then(Object::as_integer),
            Some(1)
        );
        assert_eq!(entry(1, "Count"), whole(1, "Count"));
        assert_eq!(entry(1, "Count"), whole(1, "Count"));
        assert!(
            entry(1, "Resources").is_some_and(|value| value.is_null()),
            "an entry the dictionary does not state is null, exactly as `get_key` says"
        );

        // §7.3.10's indirect value, resolved on the way out, as `get_key` resolves it.
        assert_eq!(
            entry(3, "Rotate").as_ref().and_then(Object::as_integer),
            Some(90)
        );
        // A reference to a reference: the chain is followed to the dictionary at its end.
        assert_eq!(
            entry(5, "Count").as_ref().and_then(Object::as_integer),
            Some(1)
        );
        // An array has no entries, and neither has an object the file never wrote. Both are
        // `None`, which is the answer a null could not carry.
        assert_eq!(entry(6, "Count"), None);
        assert_eq!(entry(99, "Count"), None);
    }

    /// The recursion guard is a property of a call stack, so it may not be shared.
    ///
    /// This is the one hazard the `RefCell` → `RwLock` change could have introduced without
    /// any test noticing: a document-wide `loading` set would answer §7.3.10's null to the
    /// second of two threads that happened to ask for one object at one moment, which is a
    /// wrong answer produced by timing. Held open on one thread and asked from another.
    #[test]
    fn the_recursion_guard_is_per_thread_rather_than_per_document() {
        let document = Document::empty();
        assert!(document.begin_loading(7), "the first claim on this thread");
        assert!(
            !document.begin_loading(7),
            "a second claim on the same thread is the cycle this guard exists for"
        );

        let elsewhere = std::thread::scope(|scope| {
            scope
                .spawn(|| document.begin_loading(7))
                .join()
                .expect("the thread ran")
        });
        assert!(
            elsewhere,
            "another thread reading object 7 at the same moment must not be told it is a cycle"
        );

        document.end_loading(7);
        assert!(document.begin_loading(7), "and the guard is released again");
    }

    /// §7.3.8.1's external stream: the data is elsewhere and the embedded bytes are not it.
    ///
    /// Both halves matter and only one is obvious. Recognising `/F` is the easy half; the
    /// half worth a test is that a stream carrying it is *refused* rather than decoded,
    /// because "any bytes between stream and endstream shall be ignored" makes returning
    /// them a rendering of data the clause discards — and a rendering nothing would report,
    /// since the bytes are usually a perfectly valid content stream.
    #[test]
    fn a_stream_whose_data_is_in_a_file_has_no_data_here() {
        let external = |present: bool| {
            let mut dict = Dictionary::new();
            if present {
                dict.insert(
                    Name::new(b"F".to_vec()),
                    Object::String(Arc::from(b"elsewhere.dat".as_slice())),
                );
            }
            Stream {
                dict,
                data: Arc::from(b"1 0 0 1 0 0 cm".as_slice()),
                decryption_failed: false,
            }
        };

        assert!(Document::is_external(&external(true)));
        assert!(!Document::is_external(&external(false)));
    }

    /// §7.6.2's signature exception, on the predicate that decides it.
    ///
    /// The rule is worth a test of its own because both of its failure directions are
    /// silent: exempting too much leaves a string encrypted where the document expected
    /// plaintext, and exempting too little destroys the one value in a PDF that is a
    /// detached signature over the file's own bytes.
    #[test]
    fn a_signature_dictionary_is_recognised_by_its_type() {
        let with_type = |name: &str| {
            let mut dict = Dictionary::new();
            dict.insert(
                Name::new(b"Type".to_vec()),
                Object::Name(Name::new(name.as_bytes().to_vec())),
            );
            dict
        };

        assert!(is_signature_dictionary(&with_type("Sig")));
        assert!(is_signature_dictionary(&with_type("DocTimeStamp")));
        assert!(!is_signature_dictionary(&with_type("Annot")));
        // A dictionary with a `/Contents` and no `/Type` is an annotation or a page, both
        // of which carry ordinary encrypted values under that key.
        assert!(!is_signature_dictionary(&Dictionary::new()));
    }

    /// A stream carrying `encoded` under the named filters, and nothing else.
    fn hex_stream(encoded: &Arc<[u8]>, filters: &[&str]) -> Stream {
        let mut dict = Dictionary::new();
        let named = |name: &str| Object::Name(Name::new(name.as_bytes().to_vec()));
        dict.insert(
            Name::new(b"Filter".to_vec()),
            match filters {
                [one] => named(one),
                many => Object::Array(many.iter().map(|name| named(name)).collect()),
            },
        );
        Stream {
            dict,
            data: Arc::clone(encoded),
            decryption_failed: false,
        }
    }

    /// A stream asked for twice runs §7.4's chain once, which is what the budget buys.
    #[test]
    fn a_stream_decoded_twice_is_decoded_once() {
        let document = Document::empty();
        let encoded: Arc<[u8]> = Arc::from(b"414243>".as_slice());
        let stream = hex_stream(&encoded, &["ASCIIHexDecode"]);

        let first = document.decoded_stream_data(&stream).expect("hex decodes");
        let second = document.decoded_stream_data(&stream).expect("and again");
        assert_eq!(&*first, b"ABC");
        assert_eq!(first, second);

        let held = document.decoded_streams();
        assert_eq!((held.hits, held.misses), (1, 1));
        assert_eq!(held.budget, DECODED_BUDGET);
        // The charge is the decoded bytes plus the encoded ones the entry pins.
        assert_eq!(held.bytes, 3 + 7);
    }

    /// One buffer, two filter chains, two decodes — and the key alone cannot tell them apart.
    ///
    /// `pdf_model::thumbnail::significant` builds a second `Stream` over another's `data`, so
    /// "same allocation" is not "same decode" and the chain is compared on every hit. Hex of
    /// hex is the smallest pair that makes the difference visible: one pass yields the second
    /// program's source and two yield its output.
    #[test]
    fn the_same_bytes_under_a_different_chain_are_a_different_decode() {
        let document = Document::empty();
        let encoded: Arc<[u8]> = Arc::from(b"3431343234333e>".as_slice());
        let once = hex_stream(&encoded, &["ASCIIHexDecode"]);
        let twice = hex_stream(&encoded, &["ASCIIHexDecode", "ASCIIHexDecode"]);

        assert_eq!(
            &*document.decoded_stream_data(&once).expect("one pass"),
            b"414243>"
        );
        assert_eq!(
            &*document.decoded_stream_data(&twice).expect("two passes"),
            b"ABC",
            "a hit on the address alone would have answered with the first chain's bytes"
        );
        assert_eq!(document.decoded_streams().hits, 0, "neither is the other");
    }

    /// The key is an address, and an entry holds the allocation that address names.
    ///
    /// Without that, a buffer freed with one stream and re-allocated for the next would make a
    /// lookup for the *second* stream find the *first* one's decoded bytes — a wrong answer
    /// produced by the allocator, which is the failure `doc/todo/41`'s census already met at
    /// small sizes. The loop is what makes the hazard likely rather than what makes it real:
    /// an allocator that hands back the same address is doing nothing wrong.
    #[test]
    fn a_stream_cannot_inherit_the_decoded_bytes_of_one_whose_buffer_it_reuses() {
        let document = Document::empty();
        for index in 0..64_u8 {
            let hex = format!("{index:02X}{index:02X}{index:02X}>");
            let encoded: Arc<[u8]> = Arc::from(hex.as_bytes());
            let stream = hex_stream(&encoded, &["ASCIIHexDecode"]);
            let decoded = document.decoded_stream_data(&stream).expect("hex decodes");
            assert_eq!(
                &*decoded,
                &[index, index, index],
                "this stream's own bytes, whatever address its buffer landed on"
            );
        }
    }

    /// The budget is a ceiling, and what goes is what was wanted longest ago.
    #[test]
    fn the_budget_drops_the_least_recently_used_decode_rather_than_growing() {
        let chain = |name: &str| vec![(name.as_bytes().to_vec(), None)];
        let bytes = |size: usize| -> Arc<[u8]> { Arc::from(vec![b'x'; size].as_slice()) };
        let whole = |size: usize| Decoded {
            data: bytes(size),
            damage: None,
        };
        // Room for two of these three (each charged 5 encoded + 5 decoded) and not the third.
        let mut cache = DecodedStreams::with_budget(25);
        let (first, second, third) = (bytes(5), bytes(5), bytes(5));
        cache.put(&first, chain("A"), &whole(5));
        cache.put(&second, chain("A"), &whole(5));
        assert!(cache.get(&first, &chain("A")).is_some());
        cache.put(&third, chain("A"), &whole(5));

        assert!(cache.get(&second, &chain("A")).is_none(), "the oldest went");
        assert!(
            cache.get(&first, &chain("A")).is_some(),
            "the wanted one stayed"
        );
        assert!(cache.get(&third, &chain("A")).is_some());
        let held = cache.report();
        assert_eq!((held.streams, held.bytes, held.evicted), (2, 20, 1));
        assert!(held.bytes <= held.budget, "the ceiling is a ceiling");
    }

    /// A decode too large for the whole budget is declined rather than emptying the cache.
    #[test]
    fn a_decode_that_cannot_fit_does_not_empty_the_cache_on_its_way_out() {
        let chain = vec![(b"A".to_vec(), None)];
        let bytes = |size: usize| -> Arc<[u8]> { Arc::from(vec![b'x'; size].as_slice()) };
        let whole = |size: usize| Decoded {
            data: bytes(size),
            damage: None,
        };
        let mut cache = DecodedStreams::with_budget(25);
        let (small, large) = (bytes(5), bytes(5));
        cache.put(&small, chain.clone(), &whole(5));
        cache.put(&large, chain.clone(), &whole(30));
        assert!(cache.get(&large, &chain).is_none(), "it never went in");
        assert!(
            cache.get(&small, &chain).is_some(),
            "and took nothing with it"
        );
        assert_eq!(cache.report().evicted, 0);
    }

    /// Table 255 makes `/Type` optional, so a signature that omits it is still one.
    ///
    /// `issue17069.pdf` is that document — encrypted, `/ByteRange`, `/Contents`, no `/Type` —
    /// and until the three-hundred-and-seventy-seventh session its 33 680-byte signature value
    /// went through the cipher and came back empty. What identifies one without a `/Type` is
    /// the pair Table 255 requires of every signature carrying a byte range digest, and a
    /// dictionary holding only one of them is not exempted: an annotation's `/Contents` is an
    /// ordinary encrypted text string and staying out of the cipher would leave it unreadable.
    #[test]
    fn a_signature_dictionary_with_no_type_is_recognised_by_its_byte_range() {
        let mut signature = Dictionary::new();
        signature.insert(Name::new(b"ByteRange".to_vec()), Object::Array(Vec::new()));
        signature.insert(
            Name::new(b"Contents".to_vec()),
            Object::String(Arc::from([0u8].as_slice())),
        );
        assert!(is_signature_dictionary(&signature));

        let mut annotation = Dictionary::new();
        annotation.insert(
            Name::new(b"Contents".to_vec()),
            Object::String(Arc::from(b"a note".as_slice())),
        );
        assert!(
            !is_signature_dictionary(&annotation),
            "a /Contents alone is an annotation's text and belongs in the cipher"
        );

        // And a dictionary that names another type keeps that word even with both entries.
        signature.insert(
            Name::new(b"Type".to_vec()),
            Object::Name(Name::new(b"Annot".to_vec())),
        );
        assert!(!is_signature_dictionary(&signature));
    }
}
