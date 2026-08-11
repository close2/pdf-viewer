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
//! chain re-run rather than a cache read. What is memoised is §7.5.7's object streams,
//! whose contents are objects. `doc/todo/47` carries the question of whether the rest
//! should be, which is a question about a byte budget.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

use crate::crypt::{Encryption, Method, Permissions};
use crate::error::{SyntaxError, SyntaxResult};
use crate::object::{Dictionary, Name, Object, ObjectId, Stream};
use crate::parser::{Limits, Parser};
use crate::xref::{Location, XrefTable};

/// The most indirect references that will be followed in a chain.
///
/// `1 0 obj 2 0 R endobj` pointing back at itself is a cycle, and a chain of a thousand
/// references is hostile rather than merely unusual.
const MAX_REFERENCE_DEPTH: usize = 64;

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

        // §7.5.5 makes the trailer's `/Root` "[t]he catalog dictionary for the PDF document", so
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
            if second.authenticate(password).is_ok() && second.catalog().is_ok() {
                return Ok(second);
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
        Some((found, self.decrypt_object(found, object)))
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
        let data = self.decoded_stream_data(stream)?;

        let count = self.get_key(&stream.dict, "N").as_integer().unwrap_or(0);
        let first = self
            .get_key(&stream.dict, "First")
            .as_integer()
            .and_then(|value| usize::try_from(value).ok())?;

        // The header is a list of (object number, relative offset) pairs.
        let mut header = crate::lexer::Lexer::new(data.get(..first).unwrap_or_default());
        let mut pairs = Vec::new();
        for _ in 0..count.max(0) {
            let (Some(crate::Token::Integer(object_number)), Some(crate::Token::Integer(at))) =
                (header.next_token(), header.next_token())
            else {
                break;
            };
            if let (Ok(object_number), Ok(at)) = (u32::try_from(object_number), usize::try_from(at))
            {
                pairs.push((object_number, at));
            }
        }

        let mut objects = BTreeMap::new();
        for (object_number, relative) in pairs {
            let start = first.saturating_add(relative);
            if start >= data.len() {
                continue;
            }
            let mut parser = Parser::at(&data, start, self.limits);
            if let Ok(parsed) = parser.parse_object() {
                objects.insert(object_number, parsed);
            }
        }

        let expanded = Arc::new(objects);
        write(&self.expanded_streams).insert(number, Arc::clone(&expanded));
        Some(expanded)
    }

    /// Returns a stream's decoded data.
    ///
    /// # Errors
    ///
    /// Returns `None` when a filter in the chain is not supported, rather than returning
    /// the encoded bytes. Handing back compressed data as if it were decoded would produce
    /// garbage that looks like a rendering bug. Also for a stream whose data lives in an
    /// external file — see [`Self::is_external`].
    #[must_use]
    pub fn decoded_stream_data(&self, stream: &Stream) -> Option<Arc<[u8]>> {
        if stream.decryption_failed || Self::is_external(stream) {
            return None;
        }
        let filters = self.filter_chain(&stream.dict);
        if filters.is_empty() || self.states_no_data(stream) {
            return Some(Arc::clone(&stream.data));
        }

        let mut data: Arc<[u8]> = Arc::clone(&stream.data);
        for (index, filter) in filters.iter().enumerate() {
            let parms = self.decode_parms(&stream.dict, index);
            data = crate::filter::decode_with_parms(filter, &data, parms.as_ref(), self.limits)?;
        }
        Some(data)
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
