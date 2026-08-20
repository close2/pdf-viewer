//! ISO 32000-2 §7.6, encryption: the standard security handler.
//!
//! # What is encrypted
//!
//! §7.6.2 states the rule and its exceptions:
//!
//! > Encryption applies to all strings and streams in the document's PDF file, with the
//! > following exceptions:
//! >
//! > - The values for the ID entry in the trailer
//! > - Any strings in an Encrypt dictionary
//! > - Any strings that are inside streams such as content streams and compressed object
//! >   streams, which themselves are encrypted
//! > - Any hexadecimal strings representing the value of the Contents key in a Signature
//! >   dictionary
//!
//! All four are enforced in [`crate::document`], which is where an object's identity is
//! known; this module knows only how to turn ciphertext into plaintext given that
//! identity. Two further exclusions come from Table 20 rather than §7.6.2 — a
//! cross-reference stream is never encrypted, and a stream naming its own crypt filter
//! through a `/Crypt` entry in its `/Filter` array overrides `/StmF`.
//!
//! # What this handler does and does not implement
//!
//! Implemented: the standard security handler (`/Filter /Standard`) at revisions 2, 3, 4
//! and 6, which is every revision the standard specifies, over `/V` 1, 2, 4 and 5, with
//! the `V2`, `AESV2` and `AESV3` crypt filter methods of Table 25 and the `Identity`
//! filter of Table 26.
//!
//! Refused, each with a named reason rather than a guess:
//!
//! - **Revision 5.** Table 21 says of it: "Shall not be used. This value was used by a
//!   deprecated proprietary Adobe extension." The standard therefore states no algorithm
//!   for it, and inventing one would be curve-fitting to another implementation.
//! - **Public-key security handlers** (§7.6.5), which need CMS enveloped data and X.509
//!   certificates — a public-key infrastructure rather than a cipher.
//! - **`/CFM /None`**, which Table 25 defines as "The application shall not decrypt data
//!   but shall direct the input stream to the security handler for decryption": there is
//!   no such handler here.
//! - **A revision 4 password containing a character `PDFDocEncoding` has no code for**, which
//!   §7.6.4.3.2 step (a) requires the password to be converted to. That is the encoding's own
//!   limit rather than this crate's: there are no bytes to hash. Until the
//!   hundred-and-fifty-second session the refusal was far wider — this crate converted only the
//!   codes where the encoding and Unicode agree by inspection — and it now uses the whole of
//!   Annex D Table D.3, which [`crate::text_string`] has held since the ninety-second session.

use std::collections::BTreeMap;

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{
    BlockCipherDecrypt, BlockModeDecrypt, BlockModeEncrypt, KeyInit, KeyIvInit, StreamCipher,
};
use aes::{Aes128, Aes256};
use sha2::Digest;

use crate::error::{SyntaxError, SyntaxResult};
use crate::object::{Dictionary, Name, Object, ObjectId};

/// The padding string of §7.6.4.3.2 step (a), reproduced from the clause.
///
/// A password shorter than 32 bytes is extended from the front of this string, and an
/// empty password — the default user password every reader tries first (§7.6.4.1) — is
/// replaced by the whole of it.
const PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// The AES block size, in bytes.
///
/// §7.6.3.1 fixes it at sixteen, and Errata Collection 3 rewrote the sentence that does so —
/// Issue #542, `/State` `Review` `Accepted`, invisible to `doc/md/` because the conversion
/// dropped the sponsored copy's markup (ADR 0252, ADR 0253). The 2020 wording rounded the
/// ciphertext "up to a multiple of the block size, which is fixed to always be 16 bytes"; the
/// amended wording adds the case that decides [`aes_cbc_encrypt`]'s arithmetic — the length is
/// rounded up "except in the case where the length of the data is an exact multiple of the
/// block size, in which case it is increased to the next higher multiple of the block size" —
/// and moves the sixteen into a sentence of its own. That is PKCS#7's whole-extra-block pad,
/// which this file has always written and always stripped: it was derivable before the erratum
/// from the surviving sentence two paragraphs below, "the pad is present when M is evenly
/// divisible by 16; it contains 16 bytes of 0x10".
const AES_BLOCK: usize = 16;

/// How one crypt filter transforms data — ISO 32000-2 §7.6.6 Table 25's `/CFM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Method {
    /// Table 26's `Identity`: "Input data shall be passed through without any processing."
    Identity,
    /// `V2`: Algorithm 1 with RC4.
    Rc4,
    /// `AESV2`: Algorithm 1 with AES-128 in CBC mode.
    AesV2,
    /// `AESV3`: Algorithm 1.A with AES-256 in CBC mode.
    AesV3,
}

/// Table 22's access permissions, as granted to whoever opened the document.
///
/// Nothing in this crate enforces them — §7.6.4.1 is explicit that "There is nothing
/// inherent in PDF encryption that enforces the document permissions", and that a reader
/// "shall respect the intent of the document creator" instead. That is an obligation on
/// the parts of the application that copy, print and edit, so the flags are carried up to
/// them rather than acted on here.
///
/// **Something up there reads them at last**: `pdf_model::restriction` turns these into what a
/// document asserts about one *operation*, and `viewer_core` decides what its reader does with
/// that. Which is why [`Self::revision`] is here: the flags cannot be read without it.
#[expect(
    clippy::struct_excessive_bools,
    reason = "Table 22 is a flag word of independent permissions, and naming each one is \
              the whole point; a state machine would model a relationship the clause does \
              not have"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    /// The document was opened with the owner password, which §7.6.4.1 says "should allow
    /// full (owner) access". Every other field is then advisory.
    pub owner: bool,
    /// Table 21's `/R`, because Table 22's meanings depend on it. §7.6.4.2:
    ///
    /// > Which bits shall be meaningful, and in some cases how they shall be interpreted,
    /// > shall depend on the security handler's revision number (specified in the encryption
    /// > dictionary's R entry).
    ///
    /// Four of Table 22's positions — 9, 11 and 12, and the finer reading of 3 — are marked
    /// "(Security handlers of revision 3 or greater)", so a consumer that acted on
    /// [`Self::fill_forms`] without this would read a *reserved* bit of a revision 2 word as a
    /// grant: the reserved positions "must be 1", so at revision 2 bit 9 is set in every file
    /// that conforms. The clause's own example says what that costs — "assuming revision 2 of
    /// the security handler, the value -44 permits printing and copying but disallows
    /// modifying the contents and annotations" — and -44 has bit 9 set.
    pub revision: u8,
    /// Bit 3: print the document.
    pub print: bool,
    /// Bit 4: modify the contents.
    pub modify: bool,
    /// Bit 5: copy or extract text and graphics.
    ///
    /// Table 22 adds that "for the limited purpose of providing this content to assistive
    /// technology, a PDF reader should behave as if this bit was set to 1".
    pub copy: bool,
    /// Bit 6: add or modify annotations and fill in form fields.
    pub annotate: bool,
    /// Bit 9: fill in existing form fields even if [`Self::annotate`] is clear.
    pub fill_forms: bool,
    /// Bit 11: assemble the document — insert, rotate or delete pages.
    pub assemble: bool,
    /// Bit 12: print at a quality from which a faithful digital copy could be made.
    pub print_faithfully: bool,
}

impl Permissions {
    /// Reads Table 22's flag word, §7.6.4.2.
    ///
    /// > PDF readers shall ignore all flags other than those at bit positions 3, 4, 5, 6,
    /// > 9, 10, 11, and 12.
    ///
    /// Bit 10 is read and discarded: Table 22 says it "Not used ... PDF readers shall
    /// ignore this bit".
    ///
    /// Every bit is read as the file wrote it, and the revision it has to be read *with* is
    /// kept beside it rather than folded in: what a position means at revision 2 is a
    /// question about the operation being asked about, and `pdf_model::restriction` is where
    /// an operation is named.
    fn from_flags(p: i64, owner: bool, revision: u8) -> Self {
        // Table 22 numbers bits from 1, and the NOTE under Table 21 says the value "is
        // always specified as a negative integer" because the reserved high bits are 1 —
        // so the sign of the PDF integer carries no meaning and the low 32 bits do.
        #[expect(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            reason = "§7.6.4.2: the P entry 'shall be interpreted as an unsigned 32-bit \
                      quantity', so reinterpreting the low 32 bits is the clause's own \
                      instruction rather than a lossy conversion"
        )]
        let bits = p as u32;
        let bit = |position: u32| {
            position
                .checked_sub(1)
                .is_some_and(|shift| bits >> shift & 1 == 1)
        };
        Self {
            owner,
            revision,
            print: bit(3),
            modify: bit(4),
            copy: bit(5),
            annotate: bit(6),
            fill_forms: bit(9),
            assemble: bit(11),
            print_faithfully: bit(12),
        }
    }
}

/// A document's file encryption key and the crypt filters that use it.
#[derive(Debug, Clone)]
pub(crate) struct Encryption {
    /// The file encryption key: 5 to 16 bytes for §7.6.3.2's Algorithm 1, 32 for
    /// §7.6.3.3's Algorithm 1.A. Empty when [`Self::authenticated`] is false.
    key: Vec<u8>,
    /// Whether a password matched.
    ///
    /// False only for a document none of whose own strings and streams are encrypted, which
    /// §7.6.6 lets us display without a key; anything that *is* encrypted then refuses.
    authenticated: bool,
    /// Table 20's `/StmF`, resolved to a method.
    stream: Method,
    /// Table 20's `/StrF`, resolved to a method.
    string: Method,
    /// Table 20's `/EFF`, resolved to a method, falling back to `/StmF` where it is absent.
    embedded_file: Method,
    /// Table 20's `/CF`, so that a stream's own `/Crypt` filter can name one.
    filters: BTreeMap<Name, Method>,
    /// Table 21's `/EncryptMetadata`.
    encrypt_metadata: bool,
    /// Table 22's flags, and which password opened the document.
    permissions: Permissions,
}

impl Encryption {
    /// Authenticates `password` against an encryption dictionary and derives the file
    /// encryption key.
    ///
    /// `id_first` is the first element of the trailer's `/ID` array, which §7.6.4.3.2 step
    /// (e) feeds to the hash, and which is empty when the file has no `/ID`.
    ///
    /// # Errors
    ///
    /// [`SyntaxError::UnsupportedEncryption`] when the dictionary names a handler,
    /// revision or method this reader does not implement, and
    /// [`SyntaxError::PasswordRequired`] when it does but the password is neither the user
    /// nor the owner password.
    pub(crate) fn new(
        dict: &Dictionary,
        id_first: &[u8],
        password: &str,
        resolve: &dyn Fn(&Object) -> Object,
    ) -> SyntaxResult<Self> {
        let get = |key: &str| dict.get(key).map_or(Object::Null, resolve);
        let unsupported = |detail: String| SyntaxError::UnsupportedEncryption { detail };

        // Table 20: "Standard shall be the name of the built-in password-based security
        // handler." §7.6.5's public-key handlers are a different clause and a different
        // technology; a handler we do not know cannot be guessed at, and its `/SubFilter`
        // — the entry that would let another handler take over — is by Table 20's own
        // words absent in every file that does not invite one.
        let filter = get("Filter");
        let handler = filter.as_name().map(Name::as_bytes).unwrap_or_default();
        if handler != b"Standard" {
            return Err(unsupported(format!(
                "/Filter {} is not the standard security handler (§7.6.4)",
                filter
                    .as_name()
                    .map_or_else(|| "(absent)".to_owned(), ToString::to_string)
            )));
        }

        let version = get("V").as_integer().unwrap_or(0);
        let revision = get("R").as_integer().unwrap_or(0);
        let owner_entry = get("O").as_string().unwrap_or_default().to_vec();
        let user_entry = get("U").as_string().unwrap_or_default().to_vec();
        let flags = get("P").as_integer().unwrap_or(0);
        // Table 21: "meaningful only when the value of V is 4 or 5", default true.
        let encrypt_metadata = match get("EncryptMetadata") {
            Object::Boolean(value) if version >= 4 => value,
            _ => true,
        };

        let mut flags = flags;
        let mut encrypt_metadata = encrypt_metadata;

        // The revision decides everything below it, so a revision this handler does not
        // implement is refused here rather than after its consequences have been read.
        match revision {
            2..=4 | 6 => {}
            5 => {
                return Err(unsupported(
                    "/R 5 is a deprecated proprietary extension the standard states no \
                     algorithm for (§7.6.4.2 Table 21)"
                        .to_owned(),
                ));
            }
            other => {
                return Err(unsupported(format!(
                    "/R {other} is not a revision §7.6.4 defines"
                )));
            }
        }

        let CryptFilters {
            stream,
            string,
            embedded_file,
            named: filters,
        } = crypt_filters(&get, version, revision, resolve)?;

        let authenticated = match revision {
            2..=4 => {
                let length = key_length(&get, version, resolve)?;
                authenticate_legacy(&AuthenticateLegacy {
                    password,
                    revision,
                    length,
                    owner_entry: &owner_entry,
                    user_entry: &user_entry,
                    flags,
                    id_first,
                    encrypt_metadata,
                })
            }
            _ => authenticate_r6(
                password,
                &owner_entry,
                &user_entry,
                get("OE").as_string().unwrap_or_default(),
                get("UE").as_string().unwrap_or_default(),
            ),
        };

        // A password that authenticates nothing is fatal only if a key is needed to read
        // the document at all. §7.6.6 binds the failure to the data rather than to the
        // open: "Authorization to decrypt a stream shall always be obtained before the
        // stream can be accessed", and "PDF readers and security handlers shall treat any
        // attempt to access a stream for which authorization has failed as an error."
        //
        // So a file whose `/StmF` and `/StrF` are both `Identity` — which §7.6.4.1
        // describes as "Documents in which only file attachments are encrypted" — displays
        // without a password, and its attachment refuses. `encrypted-attachment.pdf` and
        // `auth-event-ef-open.pdf` in the corpus are exactly that shape, and neither
        // authenticates against the empty password by any implementation.
        let body_needs_no_key = stream == Method::Identity && string == Method::Identity;
        let (key, owner) = match authenticated {
            Ok(pair) => pair,
            Err(SyntaxError::PasswordRequired) if body_needs_no_key => (Vec::new(), false),
            Err(error) => return Err(error),
        };

        // §7.6.4.3.3 step (f) and §7.6.4.4.12's Algorithm 13. Only revision 6 writes the
        // block, and where it is readable it outranks the plaintext entries beside it.
        if revision == 6
            && let Some(block) = perms_block(&key, get("Perms").as_string().unwrap_or_default())
        {
            flags = i64::from(block.flags);
            encrypt_metadata = block.encrypt_metadata;
        }

        Ok(Self {
            authenticated: !key.is_empty(),
            key,
            stream,
            string,
            embedded_file,
            filters,
            encrypt_metadata,
            // The match above admits 2, 3, 4 and 6 and refuses everything else, so this
            // conversion cannot fail; `unwrap_or` names an impossible revision rather than
            // panicking on it, and 0 is a revision Table 22 gives no meaningful bit at all.
            permissions: Permissions::from_flags(
                flags,
                owner,
                u8::try_from(revision).unwrap_or_default(),
            ),
        })
    }

    /// The method Table 20's `/StmF` selects for a stream that names no filter of its own.
    pub(crate) fn stream_method(&self) -> Method {
        self.stream
    }

    /// The method Table 20's `/StrF` selects for every string.
    pub(crate) fn string_method(&self) -> Method {
        self.string
    }

    /// The method Table 20's `/EFF` selects for an embedded file stream (§7.11.4).
    ///
    /// §7.6.2, Table 20:
    ///
    /// > If this entry is not present, and the embedded file stream does not contain a crypt
    /// > filter specifier, the stream shall be encrypted using the default stream crypt
    /// > filter specified by StmF .
    ///
    /// The second sentence is why the absent case is [`Self::stream_method`] rather than
    /// `Identity`: `/EFF` names a *departure* from the document's default, and a file that
    /// states none has its attachments encrypted like everything else.
    pub(crate) fn embedded_file_method(&self) -> Method {
        self.embedded_file
    }

    /// The method a `/Crypt` filter's `/Name` selects (§7.6.6, Table 14).
    ///
    /// > The stream's DecodeParms entry shall contain a Crypt filter decode parameters
    /// > dictionary … whose Name entry specifies the particular crypt filter that shall be
    /// > used (if missing, Identity is used).
    ///
    /// A name that is in neither `/CF` nor Table 26 is `Identity` for the same reason: the
    /// alternative is to decrypt with a filter the document did not name.
    pub(crate) fn named_method(&self, name: &Name) -> Method {
        if name.as_bytes() == b"Identity" {
            return Method::Identity;
        }
        self.filters.get(name).copied().unwrap_or(Method::Identity)
    }

    /// Whether Table 21's `/EncryptMetadata` leaves the document-level metadata stream in
    /// the clear.
    pub(crate) fn encrypt_metadata(&self) -> bool {
        self.encrypt_metadata
    }

    /// The permissions Table 22 grants whoever opened the document.
    pub(crate) fn permissions(&self) -> Permissions {
        self.permissions
    }

    /// Decrypts one string or stream belonging to the indirect object `id`.
    ///
    /// Returns `None` when the data is structurally impossible for the method — an AES
    /// body with no room for its initialisation vector, a length that is not a whole
    /// number of blocks, or padding §7.6.3.1 says can be "unambiguously removed" and
    /// cannot. Handing back the ciphertext instead would put binary noise where the
    /// document's own bytes belong, which is the failure trap 5 exists to prevent.
    pub(crate) fn decrypt(&self, method: Method, id: ObjectId, data: &[u8]) -> Option<Vec<u8>> {
        if method != Method::Identity && !self.authenticated {
            // §7.6.6: "PDF readers and security handlers shall treat any attempt to access
            // a stream for which authorization has failed as an error."
            return None;
        }
        match method {
            Method::Identity => Some(data.to_vec()),
            Method::Rc4 => {
                let mut out = data.to_vec();
                rc4_apply(&self.object_key(method, id), &mut out)?;
                Some(out)
            }
            Method::AesV2 | Method::AesV3 => aes_cbc_decrypt(&self.object_key(method, id), data),
        }
    }

    /// Encrypts one string or stream belonging to the indirect object `id`.
    ///
    /// The exact inverse of [`Self::decrypt`], and for RC4 it is not merely the inverse but
    /// the *same* call — §7.6.3.1: "RC4 is a symmetric stream cipher: the same algorithm
    /// shall be used for both encryption and decryption". AES differs in the two ways the
    /// clause states: RFC 8018's pad is added rather than removed, and §7.6.3.2 step (d)'s
    /// initialisation vector is generated here rather than read off the front of the data.
    ///
    /// Returns `None` when the document holds no key for the method — the same condition
    /// [`Self::decrypt`] refuses on, because a writer that emitted plaintext into an
    /// encrypted file would be producing a file its own reader could not read.
    ///
    /// # What calls this
    ///
    /// [`crate::write::incremental_update`], through [`crate::Document`], which is the only
    /// place in this tree that writes a PDF object. §7.6.2's exceptions are enforced there
    /// for the same reason they are enforced there on the way in: an object's identity is
    /// what decides them, and this module knows only the cipher.
    pub(crate) fn encrypt(&self, method: Method, id: ObjectId, data: &[u8]) -> Option<Vec<u8>> {
        if method != Method::Identity && !self.authenticated {
            return None;
        }
        match method {
            Method::Identity => Some(data.to_vec()),
            Method::Rc4 => {
                let mut out = data.to_vec();
                rc4_apply(&self.object_key(method, id), &mut out)?;
                Some(out)
            }
            Method::AesV2 | Method::AesV3 => aes_cbc_encrypt(&self.object_key(method, id), data),
        }
    }

    /// §7.6.3.2 steps (b) to (d), and §7.6.3.3 step (a).
    ///
    /// The two algorithms differ in exactly one way, which §7.6.3.1 states: "Algorithm 1.A
    /// uses the starting key directly and does not modify the file encryption key at all."
    fn object_key(&self, method: Method, id: ObjectId) -> Vec<u8> {
        if method == Method::AesV3 {
            return self.key.clone();
        }

        // (b) "extend the original n-byte file encryption key to n + 5 bytes by appending
        // the low-order 3 bytes of the object number and the low-order 2 bytes of the
        // generation number in that order, low-order byte first."
        let number = id.number.to_le_bytes();
        let generation = id.generation.to_le_bytes();
        let mut hash = md5::Md5::new();
        hash.update(&self.key);
        hash.update(number.get(..3).unwrap_or_default());
        hash.update(generation);
        if method == Method::AesV2 {
            // "If using the AES algorithm, extend the file encryption key an additional 4
            // bytes by adding the value 'sAlT'".
            hash.update(b"sAlT");
        }

        // (d) "Use the first (n + 5) bytes, up to a maximum of 16, of the output".
        let take = self.key.len().saturating_add(5).min(16);
        let digest = hash.finalize();
        digest.get(..take).unwrap_or(&digest).to_vec()
    }
}

/// The arguments of §7.6.4.4.5's Algorithm 6 and §7.6.4.4.6's Algorithm 7.
///
/// A struct rather than eight parameters because clippy is right that eight positional
/// arguments of which four are byte slices is a call nobody can read.
struct AuthenticateLegacy<'a> {
    password: &'a str,
    revision: i64,
    length: usize,
    owner_entry: &'a [u8],
    user_entry: &'a [u8],
    flags: i64,
    id_first: &'a [u8],
    encrypt_metadata: bool,
}

/// Authenticates against a revision 2, 3 or 4 dictionary, returning the file encryption
/// key and whether the owner password was the one that matched.
///
/// §7.6.4.1 fixes the order: a reader "shall first try to authenticate the encrypted
/// document using the padding string … (default user password)", and only then treat the
/// input as an owner password. Trying the user password first also matters for a document
/// whose two passwords are the same, where both would succeed and only the user password's
/// permissions are the ones Table 22 describes.
fn authenticate_legacy(input: &AuthenticateLegacy<'_>) -> SyntaxResult<(Vec<u8>, bool)> {
    let padded = pad_password(input.password)?;

    let as_user = file_key_legacy(&padded, input);
    if user_key_matches(&as_user, input) {
        return Ok((as_user, false));
    }

    // Algorithm 7: the supplied password is treated as the owner password, which unwraps
    // the `/O` entry into what "purports to be the user password" and is then authenticated
    // as one.
    if let Some(purported) = unwrap_owner_entry(&padded, input) {
        let as_owner = file_key_legacy(&purported, input);
        if user_key_matches(&as_owner, input) {
            return Ok((as_owner, true));
        }
    }

    Err(SyntaxError::PasswordRequired)
}

/// §7.6.4.3.2, Algorithm 2: computes the file encryption key from a padded password.
fn file_key_legacy(padded: &[u8; 32], input: &AuthenticateLegacy<'_>) -> Vec<u8> {
    let mut hash = md5::Md5::new();
    hash.update(padded); // (b)
    hash.update(input.owner_entry); // (c)
    // (d) "Convert the integer value of the P entry to a 32-bit unsigned binary number and
    // pass these bytes to the MD5 hash function, low-order byte first."
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "the clause asks for the low 32 bits as an unsigned number; §7.6.4.2's \
                  NOTE explains that P is written as a negative integer precisely because \
                  its high bits are all ones"
    )]
    hash.update((input.flags as u32).to_le_bytes());
    hash.update(input.id_first); // (e)
    if input.revision >= 4 && !input.encrypt_metadata {
        // (f) "If document metadata is not being encrypted, pass 4 bytes with the value
        // 0xFFFFFFFF to the MD5 hash function."
        hash.update([0xFF; 4]);
    }

    let mut digest = hash.finalize().to_vec(); // (g)
    if input.revision >= 3 {
        // (h) fifty further hashes, each over the first n bytes of the previous output.
        for _ in 0..50 {
            let head = digest.get(..input.length).unwrap_or(&digest).to_vec();
            digest = md5::Md5::digest(&head).to_vec();
        }
    }
    digest.truncate(input.length); // (i)
    digest
}

/// §7.6.4.4.5, Algorithm 6: does this key reproduce the dictionary's `/U` entry?
///
/// Step (a) is "all but the last step" of Algorithm 4 (revision 2) or Algorithm 5
/// (revisions 3 and 4), and step (b) compares "on the first 16 bytes in the case of
/// security handlers of revision 3 or greater" — because Algorithm 5 step (f) appends
/// sixteen bytes of arbitrary padding, which carry no information.
fn user_key_matches(key: &[u8], input: &AuthenticateLegacy<'_>) -> bool {
    let mut computed = if input.revision == 2 {
        // Algorithm 4 step (b): RC4 the padding string itself.
        PAD.to_vec()
    } else {
        // Algorithm 5 steps (b) and (c): MD5 of the padding string and the file identifier.
        let mut hash = md5::Md5::new();
        hash.update(PAD);
        hash.update(input.id_first);
        hash.finalize().to_vec()
    };

    if rc4_apply(key, &mut computed).is_none() {
        return false;
    }
    if input.revision >= 3 {
        // Algorithm 5 step (e): nineteen further passes under a key XORed with the counter.
        for counter in 1..=19u8 {
            let stepped = xor_each(key, counter);
            if rc4_apply(&stepped, &mut computed).is_none() {
                return false;
            }
        }
    }

    let compared = if input.revision == 2 { 32 } else { 16 };
    let expected = input.user_entry.get(..compared);
    expected.is_some_and(|expected| computed.get(..compared) == Some(expected))
}

/// §7.6.4.4.6, Algorithm 7 steps (a) and (b): unwraps `/O` into the user password.
fn unwrap_owner_entry(padded: &[u8; 32], input: &AuthenticateLegacy<'_>) -> Option<[u8; 32]> {
    // (a) is Algorithm 3 steps (a) to (d): MD5 of the padded owner password, hashed fifty
    // more times at revision 3 or greater, truncated to the key length.
    let mut digest = md5::Md5::digest(padded).to_vec();
    if input.revision >= 3 {
        for _ in 0..50 {
            digest = md5::Md5::digest(&digest).to_vec();
        }
    }
    digest.truncate(input.length);

    let mut purported: [u8; 32] = input.owner_entry.get(..32)?.try_into().ok()?;
    if input.revision == 2 {
        rc4_apply(&digest, &mut purported)?;
    } else {
        // "Do the following 20 times … a different encryption key at each iteration. The
        // key shall be generated by taking the original key … and performing an XOR …
        // between each byte of the key and the single-byte value of the iteration counter
        // (from 19 to 0)."
        for counter in (0..=19u8).rev() {
            let stepped = xor_each(&digest, counter);
            rc4_apply(&stepped, &mut purported)?;
        }
    }
    Some(purported)
}

/// §7.6.4.3.3, Algorithm 2.A: retrieves the file encryption key at revision 6.
///
/// The clause's own framing of the three 48-byte sections is what the slicing below
/// follows: "The first 32 bytes are a hash value … The next 8 bytes are called the
/// Validation Salt. The final 8 bytes are called the Key Salt."
fn authenticate_r6(
    password: &str,
    owner_entry: &[u8],
    user_entry: &[u8],
    owner_encryption: &[u8],
    user_encryption: &[u8],
) -> SyntaxResult<(Vec<u8>, bool)> {
    let password = utf8_password(password)?;
    let user48 = user_entry.get(..48).unwrap_or(user_entry);

    // (c) Test the password against the owner key. The owner test comes first here and
    // last at revision 4 because the clause orders it so, and because the two orders cannot
    // disagree: a revision 6 owner hash is salted with the whole `/U` string, so a password
    // that satisfies it cannot also satisfy the user hash by accident.
    if let (Some(owner_hash), Some(owner_validation), Some(owner_key_salt)) = (
        owner_entry.get(..32),
        owner_entry.get(32..40),
        owner_entry.get(40..48),
    ) && hash_2b(&password, owner_validation, user48) == owner_hash
    {
        {
            // (d) the intermediate owner key decrypts `/OE` with a zero initialisation
            // vector and no padding; the 32-byte result is the file encryption key.
            let intermediate = hash_2b(&password, owner_key_salt, user48);
            if let Some(key) = aes_cbc_decrypt_raw(&intermediate, [0; AES_BLOCK], owner_encryption)
            {
                return Ok((key, true));
            }
        }
    }

    // §7.6.4.4.10, Algorithm 11, then (e).
    if let (Some(user_hash), Some(user_validation), Some(user_key_salt)) = (
        user_entry.get(..32),
        user_entry.get(32..40),
        user_entry.get(40..48),
    ) && hash_2b(&password, user_validation, &[]) == user_hash
    {
        {
            let intermediate = hash_2b(&password, user_key_salt, &[]);
            if let Some(key) = aes_cbc_decrypt_raw(&intermediate, [0; AES_BLOCK], user_encryption) {
                return Ok((key, false));
            }
        }
    }

    Err(SyntaxError::PasswordRequired)
}

/// §7.6.4.3.4, Algorithm 2.B: the iterated hash of revision 6.
///
/// `extra` is the 48-byte `/U` string, which step (a) includes when checking the owner password
/// or creating the owner key and not otherwise; it is empty for the user password.
///
/// **The step was rewritten and says the same thing.** Errata Collection 3 strikes "Make a new
/// string, K1, consisting of 64 repetitions of the sequence: input password, K, the 48-byte
/// user key. The 48 byte user key is only used when checking the owner password or creating the
/// owner key." and replaces it with a two-case definition of a string `K0` — the input password,
/// `K` and the user key for the owner, the input password and `K` otherwise — with `K1` then 64
/// repetitions of `K0` (Issue #325, `/State` `Review` `Accepted`). The concatenation this
/// function builds is unchanged; the quotation above it was struck text until the
/// four-hundred-and-nineteenth session.
fn hash_2b(password: &[u8], salt: &[u8], extra: &[u8]) -> Vec<u8> {
    let mut k = {
        let mut hash = sha2::Sha256::new();
        hash.update(password);
        hash.update(salt);
        hash.update(extra);
        hash.finalize().to_vec()
    };

    // "Perform the following steps (a)-(d) 64 times", then continue while the last byte of
    // E exceeds the round number less 32. NOTE 3 says the total is "most likely between 65
    // and 80"; the bound below is far above that and exists because the loop's exit
    // condition is computed from ciphertext, which a hostile file controls.
    let mut round: usize = 0;
    let mut last_of_e = 0u8;
    while round < 64 || usize::from(last_of_e) > round.saturating_sub(32) {
        if round >= MAX_2B_ROUNDS {
            break;
        }

        // (a) K1 is 64 repetitions of password ‖ K ‖ extra.
        let mut k1 = Vec::with_capacity(
            password
                .len()
                .saturating_add(k.len())
                .saturating_add(extra.len())
                .saturating_mul(64),
        );
        for _ in 0..64 {
            k1.extend_from_slice(password);
            k1.extend_from_slice(&k);
            k1.extend_from_slice(extra);
        }

        // (b) AES-128 CBC, no padding, keyed and initialised by the two halves of K.
        let (Some(aes_key), Some(iv)) = (k.get(..16), k.get(16..32)) else {
            break;
        };
        let Some(e) = aes_cbc_decrypt_raw_with(Direction::Encrypt, aes_key, iv, &k1) else {
            break;
        };

        // (c) and (d): the first 16 bytes of E, big-endian, modulo 3 choose the hash.
        let head = e.get(..16).unwrap_or_default();
        let remainder = head.iter().fold(0u32, |accumulator, byte| {
            // 256 ≡ 1 (mod 3), so the remainder of a big-endian byte string is the
            // remainder of the sum of its bytes. Carrying the whole 128-bit integer
            // would need arithmetic this crate has no other reason to own.
            (accumulator.wrapping_add(u32::from(*byte))) % 3
        });
        k = match remainder {
            0 => sha2::Sha256::digest(&e).to_vec(),
            1 => sha2::Sha384::digest(&e).to_vec(),
            _ => sha2::Sha512::digest(&e).to_vec(),
        };

        last_of_e = e.last().copied().unwrap_or(0);
        round = round.saturating_add(1);
    }

    k.truncate(32); // "The first 32 bytes of the final K are the output of the algorithm."
    k
}

/// A ceiling on Algorithm 2.B's rounds.
///
/// The clause's own NOTE 3 puts the real number "between 65 and 80". This bound is not a
/// tuning constant but a refusal to let a file choose how long we compute: the exit test
/// reads a byte of ciphertext the document supplies, and 64 rounds each hashing 64 copies
/// of the password are already the expensive part of opening an encrypted file. Reaching it
/// produces a key that will not authenticate, which is reported as a wrong password.
const MAX_2B_ROUNDS: usize = 256;

/// Table 20's `/StmF`, `/StrF` and `/EFF`, resolved against its `/CF`.
struct CryptFilters {
    /// The default for every stream that names no filter of its own.
    stream: Method,
    /// The filter for every string.
    string: Method,
    /// The filter for an embedded file stream that carries no `/Crypt` specifier.
    embedded_file: Method,
    /// `/CF`'s own entries, so that a stream's `/Crypt` filter can name one.
    named: BTreeMap<Name, Method>,
}

/// Table 20's `/CF`, `/StmF`, `/StrF` and `/EFF`, resolved to methods.
///
/// Before `/V` 4 there are no crypt filters at all and Algorithm 1's RC4 applies to
/// everything, which is what Table 20's description of `/V` 1 and 2 states.
fn crypt_filters(
    get: &dyn Fn(&str) -> Object,
    version: i64,
    revision: i64,
    resolve: &dyn Fn(&Object) -> Object,
) -> SyntaxResult<CryptFilters> {
    if version < 4 {
        return Ok(CryptFilters {
            stream: Method::Rc4,
            string: Method::Rc4,
            embedded_file: Method::Rc4,
            named: BTreeMap::new(),
        });
    }

    let mut filters = BTreeMap::new();
    if let Some(dict) = get("CF").as_dict() {
        for (name, value) in dict.iter() {
            let Some(entry) = resolve(value).as_dict().cloned() else {
                continue;
            };
            let method = entry
                .get("CFM")
                .map(resolve)
                .and_then(|object| object.as_name().cloned());
            filters.insert(name.clone(), method_from_cfm(method.as_ref())?);
        }
    }

    // Table 20: "Default value: Identity" for both `/StmF` and `/StrF`.
    let select = |key: &str| -> Method {
        get(key)
            .as_name()
            .map_or(Method::Identity, |name| match filters.get(name) {
                Some(method) => *method,
                None => Method::Identity,
            })
    };
    let stream = select("StmF");
    let string = select("StrF");
    // Table 20's `/EFF`, whose own sentence states both halves of this: it is "[t]he name of
    // the crypt filter that shall be used when encrypting embedded file streams that do not
    // have their own crypt filter specifier", and "[i]f this entry is not present, and the
    // embedded file stream does not contain a crypt filter specifier, the stream shall be
    // encrypted using the default stream crypt filter specified by StmF."
    let embedded_file = match get("EFF").as_name() {
        Some(_) => select("EFF"),
        None => stream,
    };

    // §7.6.4.1: "For revision 4, the filter CFM value shall be V2 (RC4) or AESV2
    // (AES128). For revision 6, the filter CFM value shall be AESV3 (AES-256)." A file
    // that disagrees is not one this handler can read, because the key it derived has the
    // wrong length for the cipher named.
    let expected_r6 = revision == 6;
    for method in [stream, string, embedded_file]
        .into_iter()
        .chain(filters.values().copied())
    {
        let ok = match method {
            Method::Identity => true,
            Method::AesV3 => expected_r6,
            Method::Rc4 | Method::AesV2 => !expected_r6,
        };
        if !ok {
            return Err(SyntaxError::UnsupportedEncryption {
                detail: format!(
                    "/R {revision} with a crypt filter method §7.6.4.1 does not pair with it"
                ),
            });
        }
    }

    Ok(CryptFilters {
        stream,
        string,
        embedded_file,
        named: filters,
    })
}

/// Table 25's `/CFM`.
fn method_from_cfm(name: Option<&Name>) -> SyntaxResult<Method> {
    match name.map(Name::as_bytes) {
        Some(b"V2") => Ok(Method::Rc4),
        Some(b"AESV2") => Ok(Method::AesV2),
        Some(b"AESV3") => Ok(Method::AesV3),
        // Table 25 is closed: "Only the values listed here shall be supported.
        // Applications that encounter other values shall report that the file is encrypted
        // with an unsupported algorithm." `None` is one of the listed values and is still
        // unsupported here, because it names a security handler we do not have.
        Some(b"None") => Err(SyntaxError::UnsupportedEncryption {
            detail: "/CFM /None asks the security handler to decrypt (§7.6.6 Table 25)".to_owned(),
        }),
        None => Ok(Method::Identity),
        Some(other) => Err(SyntaxError::UnsupportedEncryption {
            detail: format!(
                "/CFM /{} is not one of Table 25's values",
                String::from_utf8_lossy(other)
            ),
        }),
    }
}

/// The file encryption key's length in bytes, for revisions 2 to 4.
///
/// §7.6.3.2 step (b): "n is 5 unless the value of V in the encryption dictionary is
/// greater than 1, in which case n is the value of Length divided by 8."
///
/// At `/V` 4 the key belongs to a crypt filter, and Table 25's `/Length` is where its size
/// is stated. That entry is famously ambiguous — the same table says "The standard security
/// handler expresses the Length entry in bytes (e.g., 32 means a length of 256 bits) and
/// public-key security handlers express it as is" — and the two readings do not overlap:
/// Table 25 bounds the key at 40 to 256 bits, so a value below 40 can only be a byte count
/// and a value of 40 or more can only be a bit count.
fn key_length(
    get: &dyn Fn(&str) -> Object,
    version: i64,
    resolve: &dyn Fn(&Object) -> Object,
) -> SyntaxResult<usize> {
    if version <= 1 {
        return Ok(5);
    }

    let in_bits =
        |value: i64| -> Option<usize> { usize::try_from(value).ok().map(|value| value / 8) };
    let normalise = |value: i64| -> Option<usize> {
        if value >= 40 {
            in_bits(value)
        } else {
            usize::try_from(value).ok()
        }
    };

    let from_filter = get("CF")
        .as_dict()
        .and_then(|filters| {
            filters
                .iter()
                .find_map(|(_, value)| resolve(value).as_dict().cloned())
        })
        .and_then(|entry| entry.get("Length").map(resolve))
        .and_then(|object| object.as_integer())
        .and_then(normalise);

    let length = from_filter
        .or_else(|| get("Length").as_integer().and_then(in_bits))
        .unwrap_or(5); // Table 20: "Default value: 40" bits.

    // Table 20 bounds `/Length` at "a multiple of 8, in the range 40 to 128", and Algorithm
    // 1 step (d) caps the derived key at 16 bytes in any case.
    if (5..=16).contains(&length) {
        Ok(length)
    } else {
        Err(SyntaxError::UnsupportedEncryption {
            detail: format!("a file encryption key of {length} bytes is outside Table 20's range"),
        })
    }
}

/// §7.6.4.3.2 step (a): pads or truncates a password to exactly 32 bytes.
fn pad_password(password: &str) -> SyntaxResult<[u8; 32]> {
    let bytes = pdf_doc_encode(password)?;

    // "if it is less than 32 bytes long, pad it by appending the required number of
    // additional bytes **from the beginning of** the following padding string … if the
    // password string is n bytes long, append the **first** 32 − n bytes of the padding
    // string." So the tail is `PAD[0..32 - n]`, not `PAD[n..32]` — a distinction the empty
    // password cannot see, because there the two are the same 32 bytes.
    let mut padded = [0u8; 32];
    let source = bytes.iter().take(32).chain(PAD.iter());
    for (slot, byte) in padded.iter_mut().zip(source) {
        *slot = *byte;
    }
    Ok(padded)
}

/// §7.6.4.3.2 step (a)'s conversion of a password to `PDFDocEncoding`.
///
/// > The password string is generated from host system codepage characters (or system scripts)
/// > by first converting the string to PDFDocEncoding .
///
/// The clause counts in *bytes* of `PDFDocEncoding`, so a password has to be converted before it
/// can be padded — and until the hundred-and-fifty-second session this crate could only convert
/// the part of the encoding that agrees with Unicode by inspection, refusing everything else. It
/// held the whole of Annex D Table D.3 the entire time, in [`crate::text_string`], for
/// §7.9.2.2's text strings; the two conversions are one operation on strings of the same order
/// of length, and there is now one of them.
///
/// A character Table D.3 has no code for is still refused by name, and that is the clause rather
/// than a shortfall: `PDFDocEncoding` cannot represent it, so there are no bytes to hash and no
/// answer to guess at. §7.6.4.1's revision 6 preprocessing is the standard's own answer for a
/// password outside it, and this reader implements that.
///
/// # Errors
///
/// [`SyntaxError::UnsupportedEncryption`] for a character Table D.3 has no code for.
fn pdf_doc_encode(password: &str) -> SyntaxResult<Vec<u8>> {
    crate::text_string::pdf_doc_encoded(password).ok_or_else(|| {
        let offending = password
            .chars()
            .find(|character| crate::text_string::pdf_doc_encoded(&character.to_string()).is_none())
            .map_or(0, u32::from);
        SyntaxError::UnsupportedEncryption {
            detail: format!(
                "the password contains U+{offending:04X}, which Annex D Table D.3's \
                 PDFDocEncoding has no code for (§7.6.4.3.2 step a)"
            ),
        }
    })
}

/// A revision 6 password as bytes — §7.6.4.3.3 steps (a) and (b), stated in §7.6.4.1:
///
/// > Preprocessing of a user-provided password consists first of normalizing its
/// > representation by applying the "SASLPrep" profile … of the "stringprep" algorithm …
/// > to the supplied password using the Normalize and BiDi options. Next, the password
/// > string shall be converted to UTF-8 encoding, and then truncated to the first 127
/// > bytes if the string is longer than 127 bytes
///
/// A password `SASLprep` *rejects* — a prohibited character, or a BiDi-rule violation — cannot be
/// the one that encrypted the document, because a writer applying the same profile could
/// not have produced it. So the refusal is reported as a wrong password rather than as an
/// unsupported file.
fn utf8_password(password: &str) -> SyntaxResult<Vec<u8>> {
    let prepared = stringprep::saslprep(password).map_err(|_| SyntaxError::PasswordRequired)?;
    let mut bytes = prepared.as_bytes().to_vec();
    // Truncating UTF-8 at a fixed byte count can split a character. The clause says bytes,
    // and a writer following it produced bytes, so this follows the bytes.
    bytes.truncate(127);
    Ok(bytes)
}

/// XORs every byte of `key` with `value` — §7.6.4.4.4 step (e) and §7.6.4.4.6 step (b).
fn xor_each(key: &[u8], value: u8) -> Vec<u8> {
    key.iter().map(|byte| byte ^ value).collect()
}

/// Applies RC4 in place, returning `None` for a key length the cipher does not accept.
fn rc4_apply(key: &[u8], data: &mut [u8]) -> Option<()> {
    let mut cipher = rc4::Rc4::new_from_slice(key).ok()?;
    cipher.apply_keystream(data);
    Some(())
}

/// §7.6.3.2 step (d) and §7.6.3.3 step (a): AES-CBC with the initialisation vector at the
/// front of the data, and RFC 8018 padding removed.
fn aes_cbc_decrypt(key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let (iv, body) = data.split_at_checked(AES_BLOCK)?;
    if !body.len().is_multiple_of(AES_BLOCK) {
        return None;
    }
    if body.is_empty() {
        // "the initialization vector is a 16-byte random number that is stored as the first
        // 16 bytes of the encrypted stream": with nothing after it there are no ciphertext
        // blocks and so no message. A writer following §7.6.3.1's padding rule to the letter
        // would have emitted a further block of sixteen 0x10 bytes — "the pad is present
        // when M is evenly divisible by 16" — and `secHandler.pdf` does not. Reading the
        // absent blocks as an absent message is the clause's own decomposition, not a
        // tolerance: the alternative is to call an empty content stream undecryptable.
        return Some(Vec::new());
    }

    let mut out = vec![0u8; body.len()];
    let plain_len = match key.len() {
        16 => cbc::Decryptor::<Aes128>::new_from_slices(key, iv)
            .ok()?
            .decrypt_padded_b2b::<Pkcs7>(body, &mut out)
            .ok()?
            .len(),
        32 => cbc::Decryptor::<Aes256>::new_from_slices(key, iv)
            .ok()?
            .decrypt_padded_b2b::<Pkcs7>(body, &mut out)
            .ok()?
            .len(),
        _ => return None,
    };
    out.truncate(plain_len);
    Some(out)
}

/// The inverse of [`aes_cbc_decrypt`]: RFC 8018's pad added, a fresh initialisation vector
/// in front.
///
/// ISO 32000-2 §7.6.3.2 step (d), which is the one sentence in §7.6.3 that binds a writer and
/// nobody else:
///
/// > If using the AES algorithm, the Cipher Block Chaining (CBC) mode, which requires an
/// > initialization vector, is used. The block size parameter is set to 16 bytes, and the
/// > initialization vector is a 16-byte random number that is stored as the first 16 bytes
/// > of the encrypted stream or string.
///
/// The vector comes from the platform's own cryptographic source rather than from anything
/// this crate derives, because a CBC initialisation vector a reader of the file can predict
/// is a CBC mode without one. A source that refuses is a `None` rather than a weaker vector:
/// there is no second-best answer here, and the caller's refusal is already a named error.
///
/// `Pkcs7` is §7.6.3.1's rule spelled in a library: "For an original message length of M,
/// the pad shall consist of 16 - (M modulo 16) bytes whose value shall also be 16 - (M
/// modulo 16)" — including the whole extra block when M is already a multiple of 16, which
/// is why the output buffer is a block longer than the input rounded down.
fn aes_cbc_encrypt(key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let mut iv = [0u8; AES_BLOCK];
    getrandom::fill(&mut iv).ok()?;

    let padded = data
        .len()
        .checked_div(AES_BLOCK)?
        .checked_add(1)?
        .checked_mul(AES_BLOCK)?;
    let mut body = vec![0u8; padded];
    let written = match key.len() {
        16 => cbc::Encryptor::<Aes128>::new_from_slices(key, &iv)
            .ok()?
            .encrypt_padded_b2b::<Pkcs7>(data, &mut body)
            .ok()?
            .len(),
        32 => cbc::Encryptor::<Aes256>::new_from_slices(key, &iv)
            .ok()?
            .encrypt_padded_b2b::<Pkcs7>(data, &mut body)
            .ok()?
            .len(),
        _ => return None,
    };
    body.truncate(written);

    let mut out = Vec::with_capacity(AES_BLOCK.saturating_add(body.len()));
    out.extend_from_slice(&iv);
    out.append(&mut body);
    Some(out)
}

/// Which way [`aes_cbc_decrypt_raw_with`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// Algorithm 2.B step (b) is the only place §7.6 asks a *reader* to encrypt.
    Encrypt,
    /// Algorithm 2.A steps (d) and (e), which unwrap `/OE` and `/UE`.
    Decrypt,
}

/// AES-CBC with an explicit initialisation vector and no padding.
///
/// Algorithm 2.A calls for exactly this — "AES-256 in CBC mode with no padding and an
/// initialization vector of zero" — and so does Algorithm 2.B, in the other direction.
fn aes_cbc_decrypt_raw(key: &[u8], iv: [u8; AES_BLOCK], data: &[u8]) -> Option<Vec<u8>> {
    aes_cbc_decrypt_raw_with(Direction::Decrypt, key, &iv, data)
}

/// The body of [`aes_cbc_decrypt_raw`], with the direction chosen by the caller.
fn aes_cbc_decrypt_raw_with(
    direction: Direction,
    key: &[u8],
    iv: &[u8],
    data: &[u8],
) -> Option<Vec<u8>> {
    if data.is_empty() || !data.len().is_multiple_of(AES_BLOCK) {
        return None;
    }
    let mut out = data.to_vec();
    let blocks: &mut [[u8; AES_BLOCK]] = as_blocks(&mut out)?;

    match (direction, key.len()) {
        (Direction::Encrypt, 16) => {
            let mut cipher = cbc::Encryptor::<Aes128>::new_from_slices(key, iv).ok()?;
            for block in blocks {
                cipher.encrypt_block(block.into());
            }
        }
        (Direction::Decrypt, 32) => {
            let mut cipher = cbc::Decryptor::<Aes256>::new_from_slices(key, iv).ok()?;
            for block in blocks {
                cipher.decrypt_block(block.into());
            }
        }
        _ => return None,
    }
    Some(out)
}

/// Reinterprets a byte buffer whose length is a multiple of the block size as blocks.
fn as_blocks(data: &mut [u8]) -> Option<&mut [[u8; AES_BLOCK]]> {
    let (blocks, rest) = data.as_chunks_mut::<AES_BLOCK>();
    rest.is_empty().then_some(blocks)
}

/// What §7.6.4.4.9's Algorithm 10 wrote into the 16-byte `/Perms` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PermsBlock {
    /// Bytes 0 to 3, "treated as a little-endian integer".
    flags: u32,
    /// Byte 8, "the ASCII character 'T' or 'F' according to the `EncryptMetadata` boolean".
    encrypt_metadata: bool,
}

/// §7.6.4.4.12, Algorithm 13: decrypts and validates the encrypted permissions block.
///
/// The block is the tamper-evident copy of what `/P` and `/EncryptMetadata` say, and
/// §7.6.4.3.3 step (f) — part of retrieving the key rather than a separate check — states
/// flatly that "Bytes 0-3 of the decrypted Perms entry, treated as a little-endian integer,
/// **are** the user permissions". So a block that carries the "adb" marker is authoritative
/// and a block that does not is unreadable, in which case the plaintext `/P` is all the
/// file offers.
fn perms_block(key: &[u8], perms: &[u8]) -> Option<PermsBlock> {
    let mut block = <[u8; AES_BLOCK]>::try_from(perms.get(..AES_BLOCK)?).ok()?;
    // Algorithm 13 step (a): AES-256 in ECB mode over one block, which is one application of
    // the block cipher and so needs no mode at all. The clause used to say "with an
    // initialization vector of zero" here, and Errata Collection 3 struck that phrase out of
    // all three of its ECB occurrences — Issue #24, `/State` `Review` `Completed` — for exactly
    // the reason written on this line before the erratum could be read (ADR 0253). The CBC
    // occurrences in Algorithm 2.A keep it, which is why `key_from_encrypted` still quotes it.
    let cipher = Aes256::new_from_slice(key).ok()?;
    cipher.decrypt_block((&mut block).into());

    // "Verify that bytes 9-11 of the result are the characters 'a', 'd', 'b'." Without
    // them the key, the block or the file is wrong, and there is nothing to read.
    if block.get(9..12) != Some(b"adb") {
        return None;
    }
    Some(PermsBlock {
        flags: u32::from_le_bytes(<[u8; 4]>::try_from(block.get(..4)?).ok()?),
        encrypt_metadata: block.get(8) == Some(&b'T'),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RC4 against RFC 6229's first test vector.
    ///
    /// The cipher comes from a dependency, so what this pins is the *call*: a key handed
    /// over unchanged and a keystream applied from offset zero. Getting either wrong
    /// produces plausible-looking bytes, which is the failure mode trap 1 describes.
    #[test]
    fn rc4_matches_rfc_6229() {
        let key = [0x01, 0x02, 0x03, 0x04, 0x05];
        let mut data = [0u8; 16];
        rc4_apply(&key, &mut data).expect("a five-byte key is within RC4's range");
        assert_eq!(
            data,
            [
                0xB2, 0x39, 0x63, 0x05, 0xF0, 0x3D, 0xC0, 0x27, 0xCC, 0xC3, 0x52, 0x4A, 0x0A, 0x11,
                0x18, 0xA8
            ]
        );
    }

    /// §7.6.4.3.2 step (a), on the case every reader takes first: the empty password.
    #[test]
    fn an_empty_password_is_the_padding_string() {
        assert_eq!(pad_password("").expect("ASCII"), PAD);
    }

    /// The same step on a password shorter than 32 bytes.
    ///
    /// The assertion that matters is the second: the tail comes from the *beginning* of the
    /// padding string. The first implementation here overlaid the password on `PAD` in
    /// place instead, which is indistinguishable for the empty password — and the empty
    /// password is the one every reader tries first, so all nineteen of the corpus's
    /// password-less encrypted documents opened correctly while every document with a
    /// password was refused.
    #[test]
    fn a_short_password_is_extended_from_the_start_of_the_padding_string() {
        let padded = pad_password("abc").expect("ASCII");
        assert_eq!(padded.get(..3), Some(b"abc".as_slice()));
        assert_eq!(padded.get(3..), PAD.get(..29));
    }

    /// A password longer than 32 bytes keeps "only its first 32 bytes".
    #[test]
    fn a_long_password_is_truncated() {
        let padded = pad_password(&"x".repeat(40)).expect("ASCII");
        assert_eq!(padded, [b'x'; 32]);
    }

    /// The whole of Annex D Table D.3, and the refusal that is the encoding's own limit.
    ///
    /// **This test asserted the opposite of its third line for a hundred and twenty-nine
    /// sessions**: §7.6.4.3.2 step (a)'s conversion was derived from the ranges where
    /// `PDFDocEncoding` and Unicode agree by inspection, so every password containing anything
    /// else was refused — while `crate::text_string` held the table the whole time.
    #[test]
    fn a_password_is_converted_by_the_table_rather_than_by_the_ranges_that_agree() {
        assert_eq!(
            pdf_doc_encode("Aé~").expect("in range"),
            vec![0x41, 0xE9, 0x7E]
        );
        // The three codes below 0x20 that are characters rather than controls, and the block
        // from 0x80 to 0x9F, are exactly what the old derivation could not reach.
        assert_eq!(
            pdf_doc_encode("a\u{2014}b").expect("EM DASH is 0x84"),
            vec![0x61, 0x84, 0x62]
        );
        assert_eq!(
            pdf_doc_encode("\u{20AC}").expect("EURO SIGN is 0xA0"),
            vec![0xA0]
        );
        assert_eq!(
            pdf_doc_encode("\u{2020}").expect("DAGGER is 0x81"),
            vec![0x81]
        );

        // And what is still refused is refused by the encoding rather than by this crate:
        // U+00A0 NO-BREAK SPACE has no code at all, because 0xA0 is the euro sign.
        assert!(pdf_doc_encode("\u{00A0}").is_err());
        assert!(pdf_doc_encode("\u{4E2D}").is_err(), "a CJK ideograph");
    }

    /// §7.6.3.2 steps (b) to (d), worked from the clause's own example.
    ///
    /// > For example, for object number 258 and generation number 7, the hexadecimal
    /// > values 0x02 0x01 0x00 0x07 0x00 would be appended to the file encryption key.
    #[test]
    fn an_object_key_appends_the_bytes_the_clause_names() {
        let encryption = Encryption {
            key: vec![1, 2, 3, 4, 5],
            authenticated: true,
            stream: Method::Rc4,
            string: Method::Rc4,
            embedded_file: Method::Rc4,
            filters: BTreeMap::new(),
            encrypt_metadata: true,
            permissions: Permissions::from_flags(-1, false, 4),
        };
        let id = ObjectId::new(258, 7);

        let mut expected = md5::Md5::new();
        expected.update([1, 2, 3, 4, 5]);
        expected.update([0x02, 0x01, 0x00, 0x07, 0x00]);
        let expected = expected.finalize();

        let key = encryption.object_key(Method::Rc4, id);
        // n + 5 is 10, which is under the 16-byte cap.
        assert_eq!(key.len(), 10);
        assert_eq!(key.as_slice(), expected.get(..10).expect("MD5 is 16 bytes"));
    }

    /// The `sAlT` extension, which applies to AES and to nothing else.
    #[test]
    fn an_aes_object_key_is_salted_and_the_rc4_one_is_not() {
        let encryption = Encryption {
            key: vec![9; 16],
            authenticated: true,
            stream: Method::AesV2,
            string: Method::AesV2,
            embedded_file: Method::AesV2,
            filters: BTreeMap::new(),
            encrypt_metadata: true,
            permissions: Permissions::from_flags(-1, false, 4),
        };
        let id = ObjectId::new(1, 0);
        assert_ne!(
            encryption.object_key(Method::AesV2, id),
            encryption.object_key(Method::Rc4, id)
        );
        // Algorithm 1.A "does not modify the file encryption key at all".
        assert_eq!(encryption.object_key(Method::AesV3, id), vec![9; 16]);
    }

    /// §7.6.4.4.12's Algorithm 13, against a block laid out by §7.6.4.4.9's Algorithm 10.
    ///
    /// Building the block here means encrypting, which is the one place in this file a test
    /// runs a cipher the reader never does — and that is what makes it a test rather than a
    /// restatement: Algorithm 10 says where each field goes and Algorithm 13 says where to
    /// read it, and only agreeing on both recovers the permissions.
    #[test]
    fn a_perms_block_yields_the_permissions_algorithm_10_stored() {
        use aes::cipher::BlockCipherEncrypt as _;

        let key = [7u8; 32];
        let mut block = [0u8; 16];
        // (b) the permissions in bytes 0 to 7, low order first; (c) byte 8 from
        // EncryptMetadata; (d) the marker; (e) four bytes that "will be ignored".
        block
            .get_mut(..4)
            .expect("a 16-byte block")
            .copy_from_slice(&0xFFFF_F0C0_u32.to_le_bytes());
        block.get_mut(4..8).expect("a 16-byte block").fill(0xFF);
        *block.get_mut(8).expect("a 16-byte block") = b'F';
        block
            .get_mut(9..12)
            .expect("a 16-byte block")
            .copy_from_slice(b"adb");
        block.get_mut(12..).expect("a 16-byte block").fill(0x5A);

        let cipher = Aes256::new_from_slice(&key).expect("a 32-byte key");
        let mut sealed = block;
        cipher.encrypt_block((&mut sealed).into());

        assert_eq!(
            perms_block(&key, &sealed),
            Some(PermsBlock {
                flags: 0xFFFF_F0C0,
                encrypt_metadata: false,
            })
        );

        // Without the marker there is nothing to read, whatever the rest holds.
        let mut spoiled = block;
        *spoiled.get_mut(9).expect("a 16-byte block") = b'x';
        let mut sealed = spoiled;
        cipher.encrypt_block((&mut sealed).into());
        assert_eq!(perms_block(&key, &sealed), None);
    }

    /// Table 22's bit numbering, from 1 rather than 0.
    #[test]
    fn permissions_read_the_bits_table_22_names() {
        // -44 is the clause's own example: "assuming revision 2 of the security handler,
        // the value -44 permits printing and copying but disallows modifying the contents
        // and annotations."
        let permissions = Permissions::from_flags(-44, false, 2);
        assert!(permissions.print);
        assert!(permissions.copy);
        assert!(!permissions.modify);
        assert!(!permissions.annotate);
        // And the half of that example a reader can get wrong: bit 9 is *set* in it, because
        // Table 22 requires positions 13 to 32 to be 1 and revision 2 gives 9 no meaning at
        // all. The revision travels with the flags so that a consumer can tell the two apart.
        assert!(permissions.fill_forms, "bit 9 of -44 is set");
        assert_eq!(
            permissions.revision, 2,
            "and means nothing at this revision"
        );
    }

    /// An AES body must carry its initialisation vector and end on a block boundary.
    #[test]
    fn a_structurally_impossible_aes_body_is_refused() {
        assert!(aes_cbc_decrypt(&[0; 16], &[0; 8]).is_none());
        assert!(aes_cbc_decrypt(&[0; 16], &[0; 20]).is_none());
        // An initialisation vector with nothing after it is an empty message, not a
        // failure; `secHandler.pdf` writes exactly that for an empty content stream.
        assert_eq!(aes_cbc_decrypt(&[0; 16], &[0; 16]), Some(Vec::new()));
    }

    /// **A `/Length` outside Table 20's range is refused before anything is sliced by it.**
    ///
    /// §7.6.3.2 step (b) derives the key length from the encryption dictionary — "n is 5
    /// unless the value of V in the encryption dictionary is greater than 1, in which case n
    /// is the value of Length divided by 8" — and that number then indexes into an MD5
    /// output, which is sixteen bytes and no more. Table 20 bounds the entry, "a multiple of
    /// 8, in the range 40 to 128", and Algorithm 1 step (d) caps the derived key at sixteen
    /// bytes in any case; nothing in the *file* is obliged to respect either.
    ///
    /// The hazard is a `hash[..n]` where `n` came from the document. hayro's issue 1273 is
    /// that slice, in their Algorithm 2, panicking with "range end index 32 out of range for
    /// slice of length 16" on a mutated encryption dictionary. The value that does it is
    /// `/Length 256`, which is a perfectly ordinary thing to write for AES-256 and which
    /// divided by 8 is twice an MD5 digest.
    ///
    /// This is a security-relevant path — the input is a document nobody has authenticated,
    /// and it is read before the password is checked — so the bound is asserted rather than
    /// argued. What is pinned is the *refusal*, a typed error, in place of a length that
    /// would later be used as an index.
    #[test]
    fn a_key_length_outside_table_20s_range_is_refused() {
        let with_length = |value: Option<i64>| {
            let entry = value;
            let get = move |key: &str| match (key, entry) {
                ("Length", Some(value)) => Object::Integer(value),
                _ => Object::Null,
            };
            let resolve = |object: &Object| object.clone();
            key_length(&get, 2, &resolve)
        };

        // Table 20's own range, in bits: 40 to 128 is 5 to 16 bytes.
        assert_eq!(with_length(Some(40)), Ok(5));
        assert_eq!(with_length(Some(128)), Ok(16));
        // "Default value: 40" bits, where the entry is absent.
        assert_eq!(with_length(None), Ok(5));

        // Past the top of the range. 256 is the value an author reaches for when they mean
        // AES-256 and are writing a revision that cannot have it — and it is exactly the one
        // that made hayro slice sixteen bytes at thirty-two. 2048 is a fuzzer's.
        for out_of_range in [136, 256, 2048, 1 << 40, 0] {
            assert!(
                with_length(Some(out_of_range)).is_err(),
                "/Length {out_of_range} is outside Table 20's range and must be refused \
                 rather than used to slice a sixteen-byte digest"
            );
        }

        // **A negative `/Length` is the one value that is not refused**, and the distinction
        // is worth stating rather than leaving to be discovered: it fails `usize::try_from`,
        // which makes it unreadable rather than out of range, and an unreadable entry takes
        // Table 20's default the same way an absent one does. That is leniency of the kind
        // this lexer applies throughout, and it is safe for the reason the whole test exists
        // — the result is a length inside the range, so nothing indexes past the digest. The
        // cost is a worse diagnosis: the document is refused later, for the wrong password it
        // now appears to have, rather than here for the `/Length` it actually wrote.
        assert_eq!(with_length(Some(-8)), Ok(5));

        // The property, stated as a property rather than as a list: whatever the file says,
        // the answer is an error or a length an MD5 digest can supply.
        for value in [
            i64::MIN,
            -1,
            0,
            1,
            39,
            40,
            41,
            127,
            128,
            129,
            1024,
            i64::MAX,
        ] {
            if let Ok(length) = with_length(Some(value)) {
                assert!(
                    (5..=16).contains(&length),
                    "/Length {value} yielded {length} bytes, which Algorithm 1 step (d) caps \
                     at 16 and which an MD5 output cannot supply"
                );
            }
        }
    }

    /// §7.6.4.4.2's Algorithm 3 steps (a) to (d), against an `/O` its steps (e) to (h) built.
    ///
    /// The reader runs only half of this algorithm, because §7.6.4.4.6's Algorithm 7 begins
    /// "[c]ompute a file encryption key from the supplied password string, as described in
    /// step (a) to step (d)" — so the other half has to exist somewhere for the half that
    /// ships to be checked at all. It is here, which is the trade
    /// `a_perms_block_yields_the_permissions_algorithm_10_stored` makes one algorithm over:
    /// the test encrypts, the reader decrypts, and the padded user password comes back only if
    /// the two agree on the padding, the fifty hashes, the key length and the twenty keys.
    ///
    /// **What it deliberately does not pin is the *order* of those twenty**, and the reason is
    /// the cipher rather than an omission: RC4 is a stream cipher, so twenty applications XOR
    /// twenty keystreams into the data and XOR is commutative. Step (g) counts "from 1 to 19"
    /// and Algorithm 7 step (b) unwinds "from 19 to 0"; reversing either loop leaves this test
    /// green because it leaves the plaintext identical. Reading the clause is the only way to
    /// get that order right, and following it costs nothing — which is why the code does.
    ///
    /// **This is the only thing in the tree that reaches [`unwrap_owner_entry`]**, and the
    /// six-hundred-and-twentieth session established that by checking the corpus's eight
    /// password-protected documents one at a time: three are revision 3 or 4 and every one of
    /// them opens on its *user* password, which returns before Algorithm 7 is reached, and the
    /// one whose known password is the owner's — `print_protection.pdf` — is revision 6, where
    /// §7.6.4.4.11's Algorithm 12 replaces this path entirely. A file that needs it is one this
    /// corpus does not hold.
    #[test]
    fn an_owner_entry_unwraps_to_the_padded_user_password() {
        // Algorithm 3 steps (e) to (h): what a writer stores in `/O`, and what nothing in this
        // crate ever computes.
        let owner_entry = |owner: &str, user: &str, revision: i64, length: usize| -> Vec<u8> {
            // (a) and (b), then (c) "(Security handlers of revision 3 or greater) Do the
            // following 50 times". Unlike §7.6.4.3.2 step (h) this one re-hashes the *whole*
            // previous digest rather than its first n bytes; the difference is the standard's.
            let mut key =
                md5::Md5::digest(pad_password(owner).expect("an ASCII password")).to_vec();
            if revision >= 3 {
                for _ in 0..50 {
                    key = md5::Md5::digest(&key).to_vec();
                }
            }
            key.truncate(length); // (d)

            let mut sealed = pad_password(user).expect("an ASCII password"); // (e)
            rc4_apply(&key, &mut sealed).expect("a key length RC4 accepts"); // (f)
            if revision >= 3 {
                // (g) "Do the following 19 times … the single-byte value of the iteration
                // counter (from 1 to 19)".
                for counter in 1..=19u8 {
                    rc4_apply(&xor_each(&key, counter), &mut sealed)
                        .expect("a key length RC4 accepts");
                }
            }
            sealed.to_vec() // (h)
        };

        // Revision 2 takes neither conditional step, and its key is five bytes whatever
        // `/Length` says; revisions 3 and 4 take both.
        for (revision, length) in [(2_i64, 5_usize), (3, 16), (4, 16)] {
            let sealed = owner_entry("the owner", "the user", revision, length);
            let input = AuthenticateLegacy {
                password: "the owner",
                revision,
                length,
                owner_entry: &sealed,
                // Algorithm 7 stops at step (b) here; step (c)'s authentication of what it
                // recovered is `user_key_matches`, which has its own documents to check against.
                user_entry: &[],
                flags: 0,
                id_first: &[],
                encrypt_metadata: true,
            };

            let padded_owner = pad_password("the owner").expect("an ASCII password");
            assert_eq!(
                unwrap_owner_entry(&padded_owner, &input),
                Some(pad_password("the user").expect("an ASCII password")),
                "revision {revision}: step (c) purports to recover the user password"
            );

            // And the whole point of the entry: another owner password recovers something else.
            let wrong = pad_password("not the owner").expect("an ASCII password");
            assert_ne!(
                unwrap_owner_entry(&wrong, &input),
                Some(pad_password("the user").expect("an ASCII password")),
                "revision {revision}: a wrong owner password must not unwrap `/O`"
            );
        }
    }
}
