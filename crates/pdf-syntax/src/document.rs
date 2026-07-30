//! A whole file: cross-references, object access, and stream decoding.
//!
//! # Lazy by design
//!
//! Opening a document reads the cross-reference information and nothing else. Objects are
//! parsed when asked for, and decoded streams are cached. That is what makes a 500-page
//! file open as fast as a 5-page one, which `CLAUDE.md` principle 2 requires — eagerly
//! walking a page tree of thousands of nodes is the most common reason viewers feel slow
//! to start.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
pub struct Document {
    bytes: Arc<[u8]>,
    xref: XrefTable,
    limits: Limits,
    /// Objects already parsed, so a repeated lookup does not re-parse.
    cache: RefCell<BTreeMap<u32, Object>>,
    /// Object streams already expanded, keyed by the stream's object number.
    expanded_streams: RefCell<BTreeMap<u32, Arc<BTreeMap<u32, Object>>>>,
    /// Object numbers currently being loaded, so that a self-referential file cannot
    /// recurse. See [`Document::get`].
    loading: RefCell<BTreeSet<u32>>,
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
        let mut document = Self {
            bytes,
            xref,
            limits,
            cache: RefCell::new(BTreeMap::new()),
            expanded_streams: RefCell::new(BTreeMap::new()),
            loading: RefCell::new(BTreeSet::new()),
            encryption: None,
            encrypt_object: None,
        };
        document.authenticate(password)?;
        Ok(document)
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
        self.cache.borrow_mut().clear();
        self.expanded_streams.borrow_mut().clear();
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
        if let Some(cached) = self.cache.borrow().get(&id.number) {
            return cached.clone();
        }

        // Loading an object can need other objects: an indirect `/Length`, an indirect
        // `/Filter`, or the object stream a compressed object lives in. A file may point
        // any of those at the object being loaded, and the resulting recursion is bounded
        // by nothing in the parser, so it is bounded here. Null is what §7.3.10 gives a
        // reference that resolves to nothing, which is what a cycle amounts to.
        if !self.loading.borrow_mut().insert(id.number) {
            return Object::Null;
        }
        let object = self.load(id).unwrap_or(Object::Null);
        self.loading.borrow_mut().remove(&id.number);

        self.cache.borrow_mut().insert(id.number, object.clone());
        object
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
                let (found, object) = self.parse_at(offset)?;
                // A table pointing at the wrong object is a real corruption. Rejecting the
                // mismatch is safer than returning another object's contents under this
                // number, which would corrupt the document graph silently.
                if found.number == id.number {
                    Some(object)
                } else {
                    None
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

    /// Chooses the crypt filter for one stream's data.
    ///
    /// §7.6.6 states the `/Crypt` override; the two exclusions are Table 20's, in §7.6.2:
    ///
    /// > All streams in the document, except for cross-reference streams … or streams that
    /// > have a Crypt entry in their Filter array …, shall be decrypted by the security
    /// > handler, using this crypt filter.
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

        encryption.stream_method()
    }

    /// Expands an object stream into its contained objects.
    fn expand_object_stream(&self, number: u32) -> Option<Arc<BTreeMap<u32, Object>>> {
        if let Some(cached) = self.expanded_streams.borrow().get(&number) {
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
        self.expanded_streams
            .borrow_mut()
            .insert(number, Arc::clone(&expanded));
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
        if filters.is_empty() {
            return Some(Arc::clone(&stream.data));
        }

        let mut data: Arc<[u8]> = Arc::clone(&stream.data);
        for (index, filter) in filters.iter().enumerate() {
            let parms = self.decode_parms(&stream.dict, index);
            data = crate::filter::decode_with_parms(filter, &data, parms.as_ref(), self.limits)?;
        }
        Some(data)
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
/// be recognised is the dictionary. `/Type` is the only thing that identifies one: a
/// document time-stamp is a signature dictionary too, and both are reached from a form
/// field's `/V` rather than from a key of their own.
fn is_signature_dictionary(dict: &Dictionary) -> bool {
    matches!(
        dict.get("Type")
            .and_then(Object::as_name)
            .map(Name::as_bytes),
        Some(b"Sig" | b"DocTimeStamp")
    )
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
}
