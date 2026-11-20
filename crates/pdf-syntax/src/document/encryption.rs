//! §7.6: the security handler a document reads under and writes under.
//!
//! Authentication against a password, the permissions the handler states, and the cipher applied
//! to an object read from the file or to one written into an update.

use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread::ThreadId;

use super::{Document, write};
use crate::crypt::{Encryption, Method, Permissions, RecipientKey, StreamFilter};
use crate::error::{SyntaxError, SyntaxResult};
use crate::object::{Dictionary, Name, Object, ObjectId, Stream};
use crate::xref::Location;

impl Document {
    /// Reads the trailer's `/Encrypt` entry and derives the file encryption key.
    ///
    /// Runs while `self.encryption` is still `None`, which is what keeps §7.6.2's second
    /// exception — "Any strings in an Encrypt dictionary" — true of the dictionary this
    /// reads. Anything else loaded along the way is dropped from the cache afterwards,
    /// because it was read before there was a key.
    ///
    /// `password` opens a standard handler's document (§7.6.4) and `recipient` a public-key
    /// handler's (§7.6.5); each handler reads the one that is its own.
    pub(super) fn authenticate(
        &mut self,
        password: &str,
        recipient: Option<&RecipientKey>,
    ) -> SyntaxResult<()> {
        let Some(entry) = self.trailer().get("Encrypt").cloned() else {
            return Ok(());
        };
        self.encrypt_object = entry.as_reference().map(|id| id.number);

        let resolved = self.resolve(&entry);
        let Some(dict) = resolved.as_dict().cloned() else {
            // A trailer naming an `/Encrypt` that is not a dictionary tells us the file is
            // encrypted and refuses to say how. Treating it as plaintext would draw a page
            // of noise while reporting nothing, and §7.6.2 is what forbids reading the
            // absence of a *readable* dictionary as the absence of the entry:
            //
            // > Encryptionrelated information shall be stored in a document's encryption
            // > dictionary, which shall be the value of the Encrypt entry in the do cument's
            // > trailer dictionary (see "Table 15 -Entries in the file trailer dictionary").
            // > The absence of this entry from the trailer dictionary means that a PDF
            // > processor shall consider the document to be not encrypted.
            //
            // The entry is present here, so the second sentence does not apply and the file
            // has said it is encrypted.
            //
            // **The sentence names what arrived**, because the three ways to reach this line
            // are different facts about the file and a refusal that cannot tell them apart is
            // a measurement of nothing: an entry of the wrong type outright, a reference to an
            // object the file does not define — §7.3.10's null object, which is what a trailer
            // pointing at an unparseable `6 0 obj` yields — and a stream where a dictionary
            // belongs. `PDFBOX-4352-0.pdf` is the corpus's witness of the middle one, and
            // until this sentence carried the type nothing distinguished it from the first.
            let named = entry.as_reference().map_or_else(String::new, |id| {
                format!(" ({} {} R)", id.number, id.generation)
            });
            return Err(SyntaxError::UnsupportedEncryption {
                detail: format!(
                    "the trailer states /Encrypt{named} and it resolves to {}, where \
                     §7.6.2 puts the encryption dictionary",
                    resolved.type_name()
                ),
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

        let encryption = Encryption::new(&dict, &id_first, password, recipient, &|object| {
            self.resolve(object)
        })?;
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

    /// Applies §7.6.2 to one freshly parsed indirect object.
    pub(super) fn decrypt_object(&self, id: ObjectId, object: Object) -> Object {
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
    /// Recursion is bounded by the parser: an object graph deeper than [`crate::parser::Limits::max_depth`]
    /// never reaches here, because it was refused at parse time.
    fn decrypt_value(&self, encryption: &Encryption, id: ObjectId, object: Object) -> Object {
        match object {
            Object::String(bytes) => Object::String(
                encryption
                    .decrypt_string(id, &bytes)
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
                let dict = if self.is_cross_reference_stream(&stream.dict) {
                    // §7.5.8.2: "strings appearing in the cross-reference stream dictionary
                    // shall not be encrypted". That dictionary is the trailer, so its `/ID` is
                    // also §7.6.2's first exception.
                    stream.dict.clone()
                } else {
                    self.decrypt_dict(encryption, id, &stream.dict)
                };
                let filter = self.stream_filter(encryption, id, &stream.dict);
                let decrypted = encryption.decrypt_stream(&filter, id, &stream.data);
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
            Object::String(bytes) => {
                Some(Object::String(encryption.encrypt_string(id, bytes)?.into()))
            }
            Object::Array(items) => items
                .iter()
                .map(|item| self.encrypt_value(encryption, id, item))
                .collect::<Option<Vec<_>>>()
                .map(Object::Array),
            Object::Dictionary(dict) => self
                .encrypt_dict(encryption, id, dict)
                .map(Object::Dictionary),
            Object::Stream(stream) => {
                let mut dict = if self.is_cross_reference_stream(&stream.dict) {
                    stream.dict.clone()
                } else {
                    self.encrypt_dict(encryption, id, &stream.dict)?
                };
                let filter = self.stream_filter(encryption, id, &stream.dict);
                let data = encryption.encrypt_stream(&filter, id, &stream.data)?;
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

    /// Chooses the crypt filter for the data of the stream `id`.
    ///
    /// §7.6.6 states the `/Crypt` override; the two exclusions are Table 20's, in §7.6.2:
    ///
    /// > All streams in the document, except for cross-reference streams … or streams that
    /// > have a Crypt entry in their Filter array …, shall be decrypted by the security
    /// > handler, using this crypt filter.
    ///
    /// Two kinds of stream are decided by *which* stream they are rather than by what their
    /// dictionary says, because the clause names them that way:
    ///
    /// - **The document-level metadata stream**, the one Table 21's `/EncryptMetadata` leaves in
    ///   the clear. Algorithm 2's NOTE 1: "This provision pertains only to document-level XMP
    ///   metadata." A page's or an image's `/Type /Metadata` stream is encrypted like every
    ///   other, so the exemption asks whether `id` is the catalog's `/Metadata`.
    /// - **An embedded file stream** (§7.11.4), the one kind with a default of its own, Table
    ///   20's `/EFF`. Its `/Type` is "(Optional)" in Table 44, so what makes a stream one is the
    ///   file specification's `/EF` naming it, and an untyped stream is looked up in
    ///   [`Self::embedded_file_streams`]. The order is the entry's own: it applies to embedded
    ///   file streams "that do not have their own crypt filter specifier", so the `/Crypt`
    ///   filter is asked first and `/EFF` decides what is left. §7.6.6's "related files ( RF )
    ///   shall use the same crypt filter as the embedded file ( EF )" holds because `/RF`'s
    ///   streams are found by the same lookup. ADR 3234.
    ///
    /// A `/Crypt` filter is returned by its name rather than its method, because under a
    /// public-key handler it may hold a key of its own (Table 27, [`StreamFilter`]).
    fn stream_filter(
        &self,
        encryption: &Encryption,
        id: ObjectId,
        dict: &Dictionary,
    ) -> StreamFilter {
        let stream_type = self
            .get_key(dict, "Type")
            .as_name()
            .map(|name| name.as_bytes().to_vec());
        match stream_type.as_deref() {
            // A cross-reference stream has to be readable before any key exists, so it is
            // never encrypted.
            Some(b"XRef") => return StreamFilter::Method(Method::Identity),
            // Table 21's `/EncryptMetadata`, whose subject is §14.3.2's document-level
            // metadata stream and no other.
            Some(b"Metadata")
                if !encryption.encrypt_metadata() && self.is_document_metadata(id) =>
            {
                return StreamFilter::Method(Method::Identity);
            }
            _ => {}
        }

        // §7.6.6: the `/Crypt` filter's own `/DecodeParms` names the filter to use, "if
        // missing, Identity is used".
        let filters = self.filter_chain(dict);
        if let Some(index) = filters.iter().position(|name| name == b"Crypt") {
            let named = self
                .decode_parms(dict, index)
                .and_then(|parms| self.get_key(&parms, "Name").as_name().cloned());
            return named.map_or(StreamFilter::Method(Method::Identity), StreamFilter::Named);
        }

        let embedded_file = match stream_type.as_deref() {
            Some(b"EmbeddedFile") => true,
            // Table 44: if present, the entry "shall be EmbeddedFile for an embedded file
            // stream", so a stream that states another type is taken at its word.
            Some(_) => false,
            // The lookup is a pass over the file, and it decides nothing where `/EFF` and
            // `/StmF` agree — which is every document that states no `/EFF`.
            None => {
                encryption.embedded_file_method() != encryption.stream_method()
                    && self.embedded_file_streams(id)
            }
        };
        if embedded_file {
            return StreamFilter::Method(encryption.embedded_file_method());
        }

        StreamFilter::Method(encryption.stream_method())
    }

    /// Whether `dict` is a cross-reference stream's (§7.5.8), by Table 17's required `/Type`.
    fn is_cross_reference_stream(&self, dict: &Dictionary) -> bool {
        self.get_key(dict, "Type")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"XRef")
    }

    /// Whether the stream `id` is §14.3.2's document-level metadata stream: the one the
    /// catalog's `/Metadata` names (Table 29).
    ///
    /// The catalog is a dictionary and is read through [`Document::get`]; a file whose `/Root`
    /// is this very stream meets `get`'s own guard against a load that reaches itself, reads
    /// null, and has no document-level metadata stream.
    fn is_document_metadata(&self, id: ObjectId) -> bool {
        let Some(root) = self.trailer().get("Root").and_then(Object::as_reference) else {
            return false;
        };
        self.get(root)
            .as_dict()
            .and_then(|catalog| catalog.get("Metadata"))
            .and_then(Object::as_reference)
            .is_some_and(|metadata| metadata.number == id.number)
    }

    /// Whether the stream `id` is an embedded file stream: one a file specification's `/EF` or
    /// `/RF` names (§7.11.3 Table 43, §7.11.4).
    ///
    /// Decryption is one object at a time, and an embedded file stream is defined by the
    /// reference to it rather than by anything it says about itself, so the references are
    /// collected once, by [`Self::scan_embedded_file_streams`], the first time an untyped stream
    /// in a document whose `/EFF` departs from `/StmF` asks. ADR 3234 argues the design and
    /// prices it.
    ///
    /// A stream this question is asked of *while the scan runs on this thread* — one the scan
    /// itself has to read, such as an object stream with no `/Type` — is answered `false`, as
    /// before the scan existed: waiting would wait for itself.
    fn embedded_file_streams(&self, id: ObjectId) -> bool {
        let streams = &self.embedded_files;
        if let Some(found) = streams.found.get() {
            return found.contains(&id.number);
        }
        let me = std::thread::current().id();
        if !write(&streams.scanning).insert(me) {
            return false;
        }
        let found = streams
            .found
            .get_or_init(|| self.scan_embedded_file_streams());
        write(&streams.scanning).remove(&me);
        found.contains(&id.number)
    }

    /// Every object number a file specification's `/EF` or `/RF` names.
    ///
    /// Table 43's `/EF`: "If this entry is present, the Type entry is required and the file
    /// specification dictionary shall be indirectly referenced", so the file specifications are
    /// among the file's indirect objects and no deeper. Each is read as the file holds it,
    /// without decryption ([`Self::parse_plain_at`]): only names and references are wanted, which
    /// §7.6.2 never encrypts, and decrypting would ask the question this answers. `/EF`'s values
    /// are the embedded file streams; `/RF`'s are §7.11.4.2's related files arrays, whose second
    /// element of each pair "shall be an embedded file stream".
    fn scan_embedded_file_streams(&self) -> BTreeSet<u32> {
        let mut found = BTreeSet::new();
        let mut take = |value: &Object| {
            if let Object::Reference(stream) = value {
                found.insert(stream.number);
            }
        };
        for number in self.xref.object_numbers() {
            let Some(Object::Dictionary(dict)) = self.plain_object(number) else {
                continue;
            };
            if let Some(Object::Dictionary(files)) = dict.get("EF").map(|ef| self.plain(ef)) {
                files.iter().for_each(|(_, value)| take(value));
            }
            if let Some(Object::Dictionary(related)) = dict.get("RF").map(|rf| self.plain(rf)) {
                for (_, value) in related.iter() {
                    if let Object::Array(pairs) = self.plain(value) {
                        pairs.iter().for_each(&mut take);
                    }
                }
            }
        }
        found
    }

    /// One indirect object as the file holds it: at an offset without decryption, or out of
    /// its object stream, whose members §7.6.2 already leaves alone.
    fn plain_object(&self, number: u32) -> Option<Object> {
        match self.xref.location(number)? {
            Location::Offset(offset) => self
                .parse_plain_at(offset)
                .filter(|(found, _)| found.number == number)
                .map(|(_, object)| object),
            Location::InStream { stream, .. } => {
                self.expand_object_stream(stream)?.get(&number).cloned()
            }
        }
    }

    /// `value`, or the object it refers to as [`Self::plain_object`] reads it.
    fn plain(&self, value: &Object) -> Object {
        match value {
            Object::Reference(id) => self.plain_object(id.number).unwrap_or(Object::Null),
            other => other.clone(),
        }
    }
}

/// The embedded file streams of one document, found once (see
/// `Document::embedded_file_streams`).
#[derive(Default)]
pub(super) struct EmbeddedFileStreams {
    /// Every object number a file specification's `/EF` or `/RF` names.
    found: OnceLock<BTreeSet<u32>>,
    /// The threads running the scan now, so that a stream the scan reads does not wait for it.
    scanning: RwLock<HashSet<ThreadId>>,
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
/// makes the entry "(Optional if Sig; Required if `DocTimeStamp`)" and states "\[t]he default
/// value is: Sig ." — so a dictionary that omits it *is* a signature dictionary, and that
/// document is one: encrypted, with a `/ByteRange`, a `/Contents` and no `/Type` at all. The
/// exception did not apply, the 33 680-byte signature value went through the cipher, and what
/// came back was empty.
///
/// What identifies one with no `/Type` is what Table 255 requires of every signature carrying
/// a byte range digest: `/ByteRange`, "\[a]n array of pairs of integers", and `/Contents`, the
/// signature value. Nothing else in ISO 32000-2 has a `/ByteRange`.
pub(crate) fn is_signature_dictionary(dict: &Dictionary) -> bool {
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
