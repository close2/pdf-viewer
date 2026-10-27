//! A whole file: cross-references, object access, and stream decoding.
//!
//! # Lazy by design
//!
//! Opening a document reads the cross-reference information and nothing else. Objects are
//! parsed when asked for and then remembered. That is what makes a 500-page file open as
//! fast as a 5-page one, which `CLAUDE.md` principle 2 requires — eagerly walking a page
//! tree of thousands of nodes is the most common reason viewers feel slow to start.
//!
//! **Decoded streams are cached under a byte budget**: without one,
//! [`Document::decoded_stream_data`] runs 12 717 times over one sweep of ISO 32000-2 and 11 975
//! times over the *second* sweep of the same document, which is a filter chain re-run rather
//! than a cache read (ADR 0260). [`DECODED_BUDGET`] and [`Document::decoded_streams`] are the
//! shape, ADR 0317 the argument.

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, OnceLock, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

use crate::crypt::Encryption;
use crate::error::SyntaxResult;
use crate::file::{FileBytes, NoRoom};
use crate::filter::{Decoded, FilterRefusal};
use crate::object::{Dictionary, Object};
use crate::parser::Limits;
use crate::xref::XrefTable;

mod cache;
mod decoding;
mod encryption;
mod images;
mod object_streams;
mod objects;
mod parsing;
mod repair;
mod reports;
mod sources;

pub use cache::DecodedStreamCache;
use cache::DecodedStreams;
pub(crate) use encryption::is_signature_dictionary;
use reports::LostToDamage;
pub use reports::{CompressedRecovery, ObjectsLost};

/// The most indirect references that will be followed in a chain.
///
/// `1 0 obj 2 0 R endobj` pointing back at itself is a cycle, and a chain of a thousand
/// references is hostile rather than merely unusual.
///
/// `pub(crate)` for one reader outside this module: [`crate::serialize()`] asks whether a file it
/// is about to write reaches §7.5.5's catalog, and the only answer that means anything is the
/// one *this* reader would give — so the two follow chains the same distance or the writer is
/// checking a different question from the one it is protecting.
pub(crate) const MAX_REFERENCE_DEPTH: usize = 64;

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

/// How many bytes the look for `endstream` after a stated `/Length` starts with.
///
/// The keyword and the end-of-line before it are eleven bytes; the window grows over any
/// further whitespace a producer wrote, as [`crate::parser::endstream_examined`] reports it.
const ENDSTREAM_WINDOW: usize = 32;

/// How many decoded bytes of object stream a *rebuild* spends recovering §7.5.7's objects.
///
/// `Document::recover_compressed_objects` decodes every object stream a header scan found, far
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
/// The five caches below are memoisation behind `RwLock`, which makes a `&Document` usable from
/// several threads at once. ADR 0260 measured what that costs and what it buys, because a lock on
/// the hottest path in the program is not free by assumption: `get` is asked **829 times a page**
/// on ISO 32000-2 and answers 92.7% of them from the cache. Single-threaded, the whole swap against
/// `RefCell` is **2 208 807 721 → 2 209 269 060** instructions through
/// `examples/callgrind_interpret` — 0.021% — and 78 464 732 → 78 357 201 through
/// `examples/callgrind_open`; a cold document-wide sweep of ISO 32000-2 stays inside its own
/// spread, 5.69 s against 5.78 s over seven interleaved samples apiece with ranges of 0.30 and 0.36
/// s.
///
/// What that makes possible is a *host's* to use rather than this crate's:
/// `pdf-model/examples/parallel_sweep` reads all 1023 pages on 24 threads through one
/// `&Document` in 1.61 s against 6.11, at 625 MB of peak resident memory against 225. ADR 0260
/// has the tables and says why nothing in `viewer-core` does it yet.
pub struct Document {
    /// The file, held once; see `file.rs` for why it is not an `Arc<[u8]>` (ADR 0795).
    bytes: FileBytes,
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
    /// Where the file's own header for each object whose *dictionary* stops part-way is, built
    /// the first time something asks for one.
    ///
    /// Separate from [`Self::headers`] because that scan records only objects it can parse
    /// whole — `xref::scan_for_objects` keeps an offset only where `parse_indirect_object`
    /// succeeded — so the very objects this is about are invisible to it and to every table
    /// derived from it. Empty until [`Document::damaged_dictionary`] is called, which is only
    /// ever from a recovery that has already found the document pageless.
    damaged: RwLock<Option<Arc<BTreeMap<u32, crate::parser::DamagedDictionary>>>>,
    /// A scan the process could not hold the file for, recorded once. See [`Self::scan_refused`].
    scan_refused: OnceLock<NoRoom>,
    /// Which object numbers were found by their own header rather than through the table.
    ///
    /// Counted rather than merely handled, because a document that needed this is a document
    /// whose cross-reference table is wrong, and a reader that repairs one in silence is a
    /// reader nobody can ask what it repaired. Reported by [`Document::misfiled_objects`].
    misfiled: RwLock<BTreeSet<u32>>,
    /// Streams whose `stream` keyword was followed by spaces or tabs before its end-of-line
    /// marker, by object number, with how many were skipped. Reported by
    /// [`Document::padded_stream_keywords`] for the reason [`Self::misfiled_objects`] gives.
    padded: RwLock<BTreeMap<u32, usize>>,
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
    /// What a crate above this one derived from these bytes and keeps for the document's life,
    /// one memo per type. See [`Document::derived`].
    derived: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
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
    /// [`crate::SyntaxError::NoHeader`] if this is not a PDF,
    /// [`crate::SyntaxError::NoCrossReferences`] if no objects can be located even by scanning,
    /// and the two encryption errors [`Self::open_with_password`] describes.
    pub fn open(bytes: impl Into<FileBytes>) -> SyntaxResult<Self> {
        Self::open_with_limits(bytes, Limits::DEFAULT)
    }

    /// Opens a document with explicit resource bounds.
    ///
    /// # Errors
    ///
    /// As [`Self::open`].
    pub fn open_with_limits(bytes: impl Into<FileBytes>, limits: Limits) -> SyntaxResult<Self> {
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
    /// As [`Self::open`], plus [`crate::SyntaxError::PasswordRequired`] when the document is
    /// encrypted and this password is neither of its two, and
    /// [`crate::SyntaxError::UnsupportedEncryption`] when it names a handler or method §7.6 does
    /// not specify or this reader does not implement.
    pub fn open_with_password(
        bytes: impl Into<FileBytes>,
        limits: Limits,
        password: &str,
    ) -> SyntaxResult<Self> {
        let bytes = bytes.into();
        let xref = crate::xref::read(&bytes, limits)?;
        let mut document = Self::around(bytes.clone(), xref, limits);
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
            && let Ok(mut rebuilt) = crate::xref::rebuild(&bytes, limits, true)
        {
            rebuilt.keep_stated(&document.xref);
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
        Self::around(FileBytes::default(), XrefTable::default(), Limits::DEFAULT)
    }

    /// The document a cross-reference table and some bytes make, before authentication.
    fn around(bytes: FileBytes, xref: XrefTable, limits: Limits) -> Self {
        Self {
            bytes,
            xref,
            limits,
            cache: RwLock::new(BTreeMap::new()),
            expanded_streams: RwLock::new(BTreeMap::new()),
            loading: RwLock::new(HashMap::new()),
            headers: RwLock::new(None),
            damaged: RwLock::new(None),
            scan_refused: OnceLock::new(),
            misfiled: RwLock::new(BTreeSet::new()),
            padded: RwLock::new(BTreeMap::new()),
            lost_to_damage: RwLock::new(LostToDamage::default()),
            decoded: RwLock::new(DecodedStreams::with_budget(DECODED_BUDGET)),
            recovered_compressed: CompressedRecovery::default(),
            encryption: None,
            encrypt_object: None,
            derived: RwLock::new(HashMap::new()),
        }
    }

    /// The memo of type `T` that a crate above this one keeps beside this document, made empty
    /// the first time it is asked for.
    ///
    /// **Why the syntax layer holds a memo it cannot read**: a fact a higher layer derives from
    /// the bytes alone — `pdf-model`'s §12.7.4.1 field inheritance, resolved once per field
    /// rather than once per widget — is a property of this document and lives as long as it,
    /// and this is the one value every such caller already holds. Keyed by the memo's own type,
    /// so two crates' memos cannot collide and this crate names none of them. The memo is
    /// responsible for staying a function of the bytes: [`Document`] is immutable (`CLAUDE.md`,
    /// "`pdf_syntax::Document` stays immutable"), and a memo that recorded a viewer's state would
    /// make it silently mutable. ADR 2448.
    #[must_use]
    pub fn derived<T: Any + Send + Sync + Default>(&self) -> Arc<T> {
        if let Some(found) = read(&self.derived).get(&TypeId::of::<T>())
            && let Ok(typed) = Arc::clone(found).downcast::<T>()
        {
            return typed;
        }
        let mut memos = write(&self.derived);
        let entry = memos
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Arc::new(T::default()));
        // The entry is keyed by `T`'s own `TypeId`, so the downcast cannot fail; were it ever to,
        // a fresh memo is a correct one that has merely forgotten what it had derived.
        Arc::clone(entry)
            .downcast::<T>()
            .unwrap_or_else(|_| Arc::new(T::default()))
    }

    /// The same bytes read through an earlier revision's cross-reference table.
    ///
    /// §7.5.6 appends rather than rewrites, so every revision of a document is still in the file
    /// and the only thing that decides which copy of an object a reader sees is the table. This
    /// is that substitution and nothing else: the same [`FileBytes`], the same limits, the same
    /// file encryption key, and empty caches because what is cached here is keyed by object
    /// number and the point of the exercise is that the number now names other bytes.
    ///
    /// The key is carried rather than derived again because §7.6 makes it the *file's*: it comes
    /// from the trailer's `/Encrypt` dictionary and the first half of its `/ID`, and §7.5.6's
    /// added trailer restates both. A file that re-encrypted itself in an update would defeat
    /// this, and would defeat every other reader too, since the objects under the update are
    /// ciphertext under a key no trailer still names.
    pub(crate) fn as_of(&self, xref: XrefTable) -> Self {
        let mut earlier = Self::around(self.bytes.clone(), xref, self.limits);
        earlier.encryption.clone_from(&self.encryption);
        earlier.encrypt_object = self.encrypt_object;
        earlier
    }

    /// Returns the bytes the document was opened from.
    ///
    /// A [`FileBytes`] dereferences to the slice; a caller that needs to know whether two
    /// documents are the same bytes in memory clones it and asks [`FileBytes::same`].
    #[must_use]
    pub fn bytes(&self) -> &FileBytes {
        &self.bytes
    }

    /// Returns the resource bounds in force.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }
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
    Pumped {
        /// The pump itself.
        pump: crate::filter::Pump,
        /// Which decode it is running, so that a reader reaching a bound can say which.
        decoding: Decoding,
    },
    /// A window has read this decode to `limit` before and found nothing to draw in it.
    ///
    /// Not a third behaviour: it is the second one's answer, remembered. See
    /// [`Document::window_found_nothing`] for what makes the memory sound, and why the reader
    /// that meets this arm raises the same report the window raised when it learnt the fact.
    Refused {
        /// The bound the window reached, which is the `limit` its own report carried.
        limit: usize,
    },
}

/// What [`Document::pumping`] found: the resumable chain, and the chain that identifies it.
///
/// Two answers from one pair of reads. See [`Document::pumping`].
struct Pumpable {
    /// The stages a [`crate::filter::Pump`] would run.
    pumping: crate::filter::Pumping,
    /// The same filters with their parameters, which is what [`DecodedStreams`] keys on.
    chain: Vec<(Vec<u8>, Option<Dictionary>)>,
}

/// Which decode a [`crate::filter::Pump`] is running: the bytes, and §7.4's chain over them.
///
/// The pair `DecodedStreams` keys an entry by, handed out so that a reader which exhausts a
/// bound can name the decode it exhausted — [`Document::window_found_nothing`] is the one
/// consumer. Opaque on purpose: a caller has no reason to read a chain apart, and the type
/// exists so that the fact travelling back cannot be about some *other* stream.
#[derive(Debug, Clone)]
pub struct Decoding {
    /// The encoded bytes, which are half the key and the whole of its identity argument.
    encoded: Arc<[u8]>,
    /// §7.4's filter chain with its parameters, which is the other half.
    chain: Vec<(Vec<u8>, Option<Dictionary>)>,
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
    use super::cache::DecodedEntry;
    use super::*;
    use crate::object::{Name, ObjectId, Stream};

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
        // The charge is the entry, the encoded bytes it pins and the decoded ones it holds.
        assert_eq!(held.bytes, size_of::<DecodedEntry>() + 3 + 7);
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

    /// §7.4's chain in front of an image codec runs once, however often the image is asked for.
    ///
    /// `pdf_model::content::image` asks [`Document::image_stream`] several times per `Do` — once
    /// for each report that is about a codec, and once for the samples — and an image drawn on
    /// forty pages is one stream. ADR 0585.
    #[test]
    fn an_images_chain_in_front_of_its_codec_is_decoded_once() {
        let document = Document::empty();
        let encoded: Arc<[u8]> = Arc::from(b"414243>".as_slice());
        let stream = hex_stream(&encoded, &["ASCIIHexDecode", "DCTDecode"]);

        let first = document.image_stream(&stream).expect("the hex stage runs");
        let second = document.image_stream(&stream).expect("and again");
        assert_eq!(&*first.data, b"ABC", "the codec's own bytes, undecoded");
        assert_eq!(first.codec.as_deref(), Some(&b"DCTDecode"[..]));
        assert_eq!(first.data, second.data);

        let held = document.decoded_streams();
        assert_eq!((held.hits, held.misses), (1, 1));
    }

    /// An image whose codec is its whole `/Filter` puts nothing in the memo.
    ///
    /// There is no chain in front of the codec, so nothing ran and there is nothing to remember;
    /// an entry would hold a second `Arc` to the bytes it is keyed by and charge the budget twice.
    #[test]
    fn an_image_with_nothing_in_front_of_its_codec_holds_nothing() {
        let document = Document::empty();
        let encoded: Arc<[u8]> = Arc::from(b"\xff\xd8\xff".as_slice());
        let stream = hex_stream(&encoded, &["DCTDecode"]);

        for _ in 0..3 {
            let image = document.image_stream(&stream).expect("no stage to refuse");
            assert_eq!(&*image.data, b"\xff\xd8\xff");
        }

        let held = document.decoded_streams();
        assert_eq!((held.hits, held.misses, held.bytes), (0, 0, 0));
    }

    /// One buffer read as an image and as an ordinary stream is two decodes, not one.
    ///
    /// The image route runs `/Filter` up to the codec and the other route runs all of it, so the
    /// chains differ and [`DecodedStreams::get`] must miss. Answering either from the other's
    /// entry would hand a caller the wrong bytes — a codec's input where its output was asked
    /// for, or the reverse.
    #[test]
    fn an_images_prefix_is_not_the_whole_chains_answer() {
        let document = Document::empty();
        let encoded: Arc<[u8]> = Arc::from(b"414243>".as_slice());
        let stream = hex_stream(&encoded, &["ASCIIHexDecode", "DCTDecode"]);

        assert_eq!(
            &*document.image_stream(&stream).expect("the hex stage").data,
            b"ABC"
        );
        // §7.4 has no `DCTDecode` outside an image, so the whole chain refuses rather than
        // answering with the image route's bytes.
        assert!(matches!(
            document.decoded_stream_data_reported(&stream),
            Err(StreamRefusal::Filter { .. })
        ));
        assert_eq!(document.decoded_streams().hits, 0, "neither is the other");
    }

    /// Table 5's codec is readable without running a filter, which is what makes the three
    /// reports that are about one codec cost nothing on every other image.
    #[test]
    fn an_images_codec_is_read_rather_than_run() {
        let document = Document::empty();
        let encoded: Arc<[u8]> = Arc::from(b"414243>".as_slice());

        let coded = hex_stream(&encoded, &["ASCIIHexDecode", "DCTDecode"]);
        assert_eq!(
            document.image_codec(&coded).as_deref(),
            Some(&b"DCTDecode"[..])
        );

        let plain = hex_stream(&encoded, &["ASCIIHexDecode"]);
        assert_eq!(
            document.image_codec(&plain),
            None,
            "the samples are the bytes"
        );

        let held = document.decoded_streams();
        assert_eq!(
            (held.hits, held.misses),
            (0, 0),
            "asking which codec must not decode anything"
        );
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
        // Room for two of these three and not the third. Each is charged its own entry plus 5
        // encoded and 5 decoded, so the budget is written that way rather than as a number
        // that would have to be re-derived whenever the entry gains a field.
        let one = size_of::<DecodedEntry>().saturating_add(10);
        let mut cache = DecodedStreams::with_budget(one * 2 + 5);
        let (first, second, third) = (bytes(5), bytes(5), bytes(5));
        cache.put(&first, chain("A"), &whole(5));
        cache.put(&second, chain("A"), &whole(5));
        assert!(cache.get(&first, &chain("A"), usize::MAX).is_some());
        cache.put(&third, chain("A"), &whole(5));

        assert!(
            cache.get(&second, &chain("A"), usize::MAX).is_none(),
            "the oldest went"
        );
        assert!(
            cache.get(&first, &chain("A"), usize::MAX).is_some(),
            "the wanted one stayed"
        );
        assert!(cache.get(&third, &chain("A"), usize::MAX).is_some());
        let held = cache.report();
        assert_eq!((held.streams, held.bytes, held.evicted), (2, one * 2, 1));
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
        let mut cache = DecodedStreams::with_budget(size_of::<DecodedEntry>() + 15);
        let (small, large) = (bytes(5), bytes(5));
        cache.put(&small, chain.clone(), &whole(5));
        cache.put(&large, chain.clone(), &whole(30));
        assert!(
            cache.get(&large, &chain, usize::MAX).is_none(),
            "it never went in"
        );
        assert!(
            cache.get(&small, &chain, usize::MAX).is_some(),
            "and took nothing with it"
        );
        assert_eq!(cache.report().evicted, 0);
    }

    /// A refusal is an answer, and reaching it twice is what `doc/todo/41` was left owing.
    ///
    /// Under a bound of a few bytes the inflation is instant, so what this asserts is the
    /// *memo* rather than the saving: the second reading of a stream whose decode passed
    /// [`Limits::max_stream_len`] runs no filter at all, and says the same thing it said the
    /// first time. ADR 0437; `doc/todo/41` has the measurement on a document that pays a
    /// gibibyte for it.
    #[test]
    fn a_stream_whose_decode_was_refused_is_refused_from_the_memo() {
        let limits = Limits {
            max_stream_len: 2,
            ..Limits::DEFAULT
        };
        let document = Document::around(FileBytes::default(), XrefTable::default(), limits);
        // §7.4.5's run: a length byte above 128 repeats the byte after it 257 − length times,
        // and 128 is the end of data. Seven bytes out of three, which is four too many.
        let encoded: Arc<[u8]> = Arc::from([0xFA, b'x', 0x80].as_slice());
        let stream = hex_stream(&encoded, &["RunLengthDecode"]);

        let first = document.decoded_stream_data_reported(&stream);
        let second = document.decoded_stream_data_reported(&stream);
        let refusal = StreamRefusal::Filter {
            name: "RunLengthDecode".to_owned(),
            why: FilterRefusal::TooLarge { limit: 2 },
        };
        assert_eq!(first.err(), Some(refusal.clone()));
        assert_eq!(
            second.err(),
            Some(refusal),
            "the same answer, from the memo"
        );

        let held = document.decoded_streams();
        assert_eq!((held.hits, held.misses), (1, 1));
        assert_eq!(
            held.bytes,
            size_of::<DecodedEntry>() + encoded.len(),
            "a refusal holds no decoded bytes and is charged for what it does hold"
        );
    }

    /// And it is an answer *under a bound*, which is the one way this memo could lie.
    ///
    /// `Document::nested_content_source` decodes under the memo's own allowance rather than
    /// under the document's `max_stream_len`, so a `TooLarge` reached there says only that the
    /// stream is longer than that allowance. Serving it to a caller asking under the
    /// document's own bound would refuse a stream this reader can decode.
    #[test]
    fn a_refusal_under_a_tighter_bound_is_not_an_answer_under_a_looser_one() {
        let chain = vec![(b"ASCIIHexDecode".to_vec(), None)];
        let encoded: Arc<[u8]> = Arc::from(b"41424344>".as_slice());
        let refusal = StreamRefusal::Filter {
            name: "ASCIIHexDecode".to_owned(),
            why: FilterRefusal::TooLarge { limit: 2 },
        };
        let mut cache = DecodedStreams::with_budget(DECODED_BUDGET);
        cache.refuse(&encoded, chain.clone(), &refusal, 2);

        assert_eq!(
            cache.get(&encoded, &chain, 2).and_then(Result::err),
            Some(refusal),
            "the bound it was reached under is an answer"
        );
        assert!(
            cache.get(&encoded, &chain, 1).is_some(),
            "and so is a tighter one"
        );
        assert!(
            cache.get(&encoded, &chain, 4).is_none(),
            "a looser bound has to run the filter"
        );
    }

    /// The routing question obeys the same bound rule, and counts as neither a hit nor a miss.
    ///
    /// [`DecodedStreams::nothing_under`] is asked *before* a decode rather than instead of one,
    /// so its population is not [`DecodedStreamCache::hits`]'s and adding it to that tally would
    /// make the instrument report two questions as one. What it must not get wrong is the same
    /// thing the test above pins: an entry reached under a tighter bound says nothing about a
    /// looser one, and a window is about to run under the document's own. ADR 0646.
    #[test]
    fn the_routing_question_answers_under_its_bound_and_moves_neither_tally() {
        let chain = vec![(b"FlateDecode".to_vec(), None)];
        let encoded: Arc<[u8]> = Arc::from(b"xxxxxxxx".as_slice());
        let refusal = StreamRefusal::Filter {
            name: "FlateDecode".to_owned(),
            why: FilterRefusal::TooLarge { limit: 8 },
        };
        let mut cache = DecodedStreams::with_budget(DECODED_BUDGET);
        cache.refuse(&encoded, chain.clone(), &refusal, 8);
        let before = cache.report();

        assert_eq!(
            cache.nothing_under(&encoded, &chain, 8),
            Some(8),
            "the bound it was reached under is an answer"
        );
        assert_eq!(
            cache.nothing_under(&encoded, &chain, 4),
            Some(8),
            "and so is a tighter one"
        );
        assert_eq!(
            cache.nothing_under(&encoded, &chain, 16),
            None,
            "a looser bound has to run the window"
        );
        assert_eq!(
            cache.nothing_under(&encoded, &[(b"LZWDecode".to_vec(), None)], 8),
            None,
            "and another chain over the same bytes is another decode"
        );

        let after = cache.report();
        assert_eq!(
            (before.hits, before.misses),
            (after.hits, after.misses),
            "asking which route to take is not asking for bytes"
        );
    }

    /// A refusal reached for a reason that is not a bound is not a reason to skip the window.
    ///
    /// [`FilterRefusal`] has arms that say the chain cannot work at all, and a caller asking
    /// [`DecodedStreams::nothing_under`] has one a window *can* pump — so answering from such an
    /// entry would decline a stream on the strength of something else's complaint.
    #[test]
    fn the_routing_question_answers_only_for_a_bound() {
        let chain = vec![(b"FlateDecode".to_vec(), None)];
        let encoded: Arc<[u8]> = Arc::from(b"xxxxxxxx".as_slice());
        let mut cache = DecodedStreams::with_budget(DECODED_BUDGET);
        cache.refuse(&encoded, chain.clone(), &StreamRefusal::DecryptionFailed, 8);
        assert_eq!(cache.nothing_under(&encoded, &chain, 8), None);
    }

    /// Table 255 makes `/Type` optional, so a signature that omits it is still one.
    ///
    /// `issue17069.pdf` is that document — encrypted, `/ByteRange`, `/Contents`, no `/Type` — and
    /// through the cipher its 33 680-byte signature value comes back empty (ADR 0215). What
    /// identifies one without a `/Type` is the pair Table 255 requires of every signature carrying
    /// a byte range digest, and a dictionary holding only one of them is not exempted: an
    /// annotation's `/Contents` is an ordinary encrypted text string and staying out of the cipher
    /// would leave it unreadable.
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
