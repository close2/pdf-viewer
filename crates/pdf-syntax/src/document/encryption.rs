//! §7.6: the security handler a document reads under and writes under.
//!
//! Authentication against a password, the permissions the handler states, and the cipher applied
//! to an object read from the file or to one written into an update.

use std::sync::Arc;

use super::{Document, write};
use crate::crypt::{Encryption, Method, Permissions};
use crate::error::{SyntaxError, SyntaxResult};
use crate::object::{Dictionary, Name, Object, ObjectId, Stream};

impl Document {
    /// Reads the trailer's `/Encrypt` entry and derives the file encryption key.
    ///
    /// Runs while `self.encryption` is still `None`, which is what keeps §7.6.2's second
    /// exception — "Any strings in an Encrypt dictionary" — true of the dictionary this
    /// reads. Anything else loaded along the way is dropped from the cache afterwards,
    /// because it was read before there was a key.
    pub(super) fn authenticate(&mut self, password: &str) -> SyntaxResult<()> {
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
