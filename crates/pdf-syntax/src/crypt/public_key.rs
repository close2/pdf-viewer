//! ISO 32000-2 §7.6.5: the public-key security handler.
//!
//! Where the standard handler derives the file encryption key from a password, this one derives
//! it from a seed that only the document's recipients can reach. §7.6.5.1 puts the reader's one
//! obligation in a sentence:
//!
//! > When decrypting the data, the PDF reader shall scan the recipient list for which the content
//! > is encrypted and shall attempt to find a match with a certificate that belongs to the user.
//!
//! The certificate, and the private key that goes with it, are the reader's own: [`RecipientKey`]
//! carries them in from a host, never from the document and never from a path this crate opens
//! (ADR 1134). The recipient lists are RFC 5652 `EnvelopedData`, read and opened by
//! `pdf_pki::enveloped`; what comes out is §7.6.5.3's byte array — "\[a] 20-byte seed" and "\[a]
//! 4-byte value defining the permissions, most significant byte first" — and the file encryption
//! key is a digest over the seed and every recipient list ([`file_key`]). From there the data is
//! decrypted by §7.6.3's Algorithm 1 or 1.A exactly as under the standard handler, so this module
//! ends where [`super::Encryption`] begins.
//!
//! # Where the recipient lists are, by `/SubFilter`
//!
//! §7.6.5.2: `adbe.pkcs7.s3` and `adbe.pkcs7.s4` "shall be used when not using crypt filters", and
//! their lists are Table 23's `/Recipients` in the encryption dictionary; `adbe.pkcs7.s5` "shall be
//! used when using crypt filters", and its lists are Table 27's `/Recipients` in the crypt filter
//! `/StmF` or `/StrF` names. A crypt filter reached only from a stream's own `/Crypt` decode
//! parameters may carry a recipient list of its own and with it a key of its own (Table 27's second
//! case): one CMS object, whose enveloped data is a 20-byte seed and no permissions. Its key is
//! derived beside the document's ([`own_key`]) and kept by the filter's name, so a stream that
//! names the filter decrypts under it and a reader its list does not name is refused that stream
//! alone, or the whole document where the filter's `/AuthEvent` is `DocOpen` (ADR 2810).

use pdf_pki::enveloped::{self, EnvelopeError};
use pdf_pki::key::PrivateKey;
use pdf_pki::x509;
use sha1::Sha1;
use sha2::{Digest, Sha256};

use super::{CryptFilters, Encryption, Handler, Method, Permissions, crypt_filters, key_length};
use crate::error::{SyntaxError, SyntaxResult};
use crate::object::{Dictionary, Name, Object};

/// The three `/SubFilter` values §7.6.5.2 permits a public-key security handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SubFilter {
    /// `adbe.pkcs7.s3`: no crypt filters, and "only those \[permissions] specified for revision 2
    /// of the standard security handler" (§7.6.5.3).
    S3,
    /// `adbe.pkcs7.s4`: no crypt filters, and "security handlers of revision 3 permissions shall
    /// apply".
    S4,
    /// `adbe.pkcs7.s5`: crypt filters, and the permissions "the same as adbe.pkcs7.s4".
    S5,
}

impl SubFilter {
    /// The value Table 20's `/SubFilter` names, where it is one of §7.6.5.2's three.
    pub(super) fn of(object: &Object) -> Option<Self> {
        match object.as_name().map(Name::as_bytes)? {
            b"adbe.pkcs7.s3" => Some(Self::S3),
            b"adbe.pkcs7.s4" => Some(Self::S4),
            b"adbe.pkcs7.s5" => Some(Self::S5),
            _ => None,
        }
    }

    /// The standard handler's revision whose reading of the permission word applies.
    fn permission_revision(self) -> u8 {
        match self {
            Self::S3 => 2,
            Self::S4 | Self::S5 => 3,
        }
    }
}

/// The reader's own certificate and private key, for §7.6.5.1's match.
///
/// A host supplies both, as bytes it read itself: the certificate in X.509's DER and the key as
/// RFC 5958's `OneAsymmetricKey`, RFC 8017's `RSAPrivateKey` or RFC 5915's `ECPrivateKey`. An RSA
/// key opens a key-transport recipient and an elliptic-curve key a key-agreement one (RFC 5652
/// sections 6.2.1 and 6.2.2; ADR 2821). Both are read here once, so that a key that cannot be used
/// is refused when it is handed over rather than when a document is opened with it. The key never
/// appears in a `Debug` output.
pub struct RecipientKey {
    /// The certificate's DER, read again when matching because a parsed certificate borrows it.
    certificate: Vec<u8>,
    /// The private key, as `pdf_pki::key` holds one of either family.
    key: PrivateKey,
}

impl std::fmt::Debug for RecipientKey {
    /// The key's width and the certificate's length; the key itself is the reader's secret.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecipientKey")
            .field("certificate_bytes", &self.certificate.len())
            .field("key", &self.key)
            .finish()
    }
}

impl RecipientKey {
    /// A certificate and the private key that belongs to it.
    ///
    /// # Errors
    ///
    /// [`SyntaxError::RecipientKey`] where the certificate is not an X.509 certificate this
    /// reader can read, or the key is neither an RSA private key nor an elliptic-curve one on a
    /// curve this reader computes on, in any of the three encodings.
    pub fn new(certificate: Vec<u8>, private_key: &[u8]) -> SyntaxResult<Self> {
        x509::parse(&certificate).map_err(|error| SyntaxError::RecipientKey {
            detail: format!("the certificate does not read as X.509: {error}"),
        })?;
        let key = PrivateKey::from_der(private_key).map_err(|error| SyntaxError::RecipientKey {
            detail: format!("the private key does not read: {error}"),
        })?;
        Ok(Self { certificate, key })
    }

    /// The certificate, parsed: [`Self::new`] already proved that it parses.
    fn certificate(&self) -> SyntaxResult<x509::Certificate<'_>> {
        x509::parse(&self.certificate).map_err(|error| SyntaxError::RecipientKey {
            detail: format!("the certificate does not read as X.509: {error}"),
        })
    }
}

impl Encryption {
    /// Opens a document whose encryption dictionary names a public-key security handler.
    ///
    /// `recipient` is the reader's own certificate and key, or `None` where the host supplied
    /// none; a document that needs a key at open then answers [`SyntaxError::RecipientRequired`],
    /// the public-key handler's [`SyntaxError::PasswordRequired`].
    pub(super) fn public_key(
        get: &dyn Fn(&str) -> Object,
        sub_filter: SubFilter,
        recipient: Option<&RecipientKey>,
        resolve: &dyn Fn(&Object) -> Object,
    ) -> SyntaxResult<Self> {
        let unsupported = |detail: String| SyntaxError::UnsupportedEncryption { detail };
        let version = get("V").as_integer().unwrap_or(0);
        let lists = match sub_filter {
            SubFilter::S5 => {
                // §7.6.5.2: s5 "shall be used when using crypt filters", and Table 20 makes crypt
                // filters `/V` 4 and 5.
                if !matches!(version, 4 | 5) {
                    return Err(unsupported(format!(
                        "/SubFilter /adbe.pkcs7.s5 uses crypt filters (§7.6.5.2), which /V \
                         {version} does not define (Table 20)"
                    )));
                }
                let filters = crypt_filters(get, version, Handler::PublicKey, resolve)?;
                filter_lists(get, filters, resolve)?
            }
            SubFilter::S3 | SubFilter::S4 => {
                // §7.6.5.2: s3 and s4 "shall be used when not using crypt filters".
                if version >= 4 {
                    return Err(unsupported(format!(
                        "/SubFilter /adbe.pkcs7.s{} is used when not using crypt filters \
                         (§7.6.5.2), and /V {version} uses them",
                        if sub_filter == SubFilter::S3 { 3 } else { 4 }
                    )));
                }
                RecipientLists {
                    filters: crypt_filters(get, version, Handler::PublicKey, resolve)?,
                    // Table 23: "Required when SubFilter is adbe.pkcs7.s3 or adbe.pkcs7.s4".
                    lists: byte_strings(&get("Recipients"), resolve).ok_or_else(|| {
                        unsupported(
                            "Table 23's /Recipients is not an array of byte strings".to_owned(),
                        )
                    })?,
                    encrypt_metadata: true,
                    length: key_length(get, version, resolve)?,
                    own: Vec::new(),
                }
            }
        };
        let RecipientLists {
            filters:
                CryptFilters {
                    stream,
                    string,
                    embedded_file,
                    named,
                    named_at_open,
                    ..
                },
            lists,
            encrypt_metadata,
            length,
            own,
        } = lists;
        // Table 25's `/AuthEvent`, asked of the document's key and of each filter's own key
        // separately: the two default filters always want theirs at the open, and every other
        // filter whose event is `DocOpen` wants the key it decrypts under.
        let owns = |name: &Name| own.iter().any(|list| &list.name == name);
        let key_at_open = stream != Method::Identity
            || string != Method::Identity
            || named_at_open.iter().any(|name| !owns(name));
        let own_keys = own
            .iter()
            .map(|list| {
                let key = own_key(list, recipient)?;
                if key.is_none() && named_at_open.contains(&list.name) {
                    // "If authorization fails, the event shall fail" (Table 25), and the event
                    // is the document's opening.
                    return Err(SyntaxError::RecipientRequired);
                }
                Ok((list.name.clone(), key))
            })
            .collect::<SyntaxResult<_>>()?;

        let opened = match recipient {
            Some(recipient) => scan(&lists, recipient),
            None => Err(SyntaxError::RecipientRequired),
        };
        let (key, permissions) = match opened {
            Ok(array) => {
                let (seed, flags) = seed_and_permissions(&array)?;
                let key = file_key(seed, &lists, !encrypt_metadata, length);
                (key, permissions(flags, sub_filter))
            }
            // Table 25's `/AuthEvent`, as under the standard handler: a failed authorization is
            // fatal only where a key is wanted before the document is open.
            Err(SyntaxError::RecipientRequired) if !key_at_open => {
                (Vec::new(), permissions(0, sub_filter))
            }
            Err(error) => return Err(error),
        };
        Ok(Self {
            authenticated: !key.is_empty(),
            key,
            stream,
            string,
            embedded_file,
            filters: named,
            own_keys,
            encrypt_metadata,
            permissions,
        })
    }
}

/// What the dictionaries say about the recipient lists and the key they make.
struct RecipientLists {
    /// The crypt filters, as [`crypt_filters`] reads them.
    filters: CryptFilters,
    /// Each recipient list, in array order: §7.6.5.3 digests them in that order.
    lists: Vec<Vec<u8>>,
    /// Table 27's `/EncryptMetadata`.
    encrypt_metadata: bool,
    /// The file encryption key's length, in bytes.
    length: usize,
    /// Table 27's second case: every other crypt filter that carries a recipient list of its own.
    own: Vec<OwnList>,
}

/// A crypt filter a stream's `/Crypt` decode parameters reach, with its own recipient list.
///
/// Table 27: "If the crypt filter is referenced from a Crypt filter decode parameter dictionary
/// (see "Table 14 - Optional parameters for Crypt filters"), this entry shall be a string that
/// shall be a binary-encoded CMS object that shall contain a list of all recipients who are
/// permitted to access the corresponding encrypted stream."
struct OwnList {
    /// The filter's key in `/CF`, which a stream's `/Crypt` decode parameters name.
    name: Name,
    /// The one CMS object.
    list: Vec<u8>,
    /// The filter's own key length, in bytes, by its `/CFM` as for the document's filter.
    length: usize,
}

/// Table 27's entries in the crypt filter `/StmF` names, or `/StrF` where `/StmF` is `Identity`.
fn filter_lists(
    get: &dyn Fn(&str) -> Object,
    filters: CryptFilters,
    resolve: &dyn Fn(&Object) -> Object,
) -> SyntaxResult<RecipientLists> {
    let unsupported = |detail: String| SyntaxError::UnsupportedEncryption { detail };
    let entries = get("CF").as_dict().cloned().unwrap_or_default();
    let named = |key: &str| {
        get(key)
            .as_name()
            .filter(|name| name.as_bytes() != b"Identity")
            .cloned()
    };
    // Neither default filter encrypting leaves the document's strings and streams plaintext,
    // and then there is no document-level recipient list to scan.
    let (name, from_stream) = match (named("StmF"), named("StrF")) {
        (Some(name), _) => (Some(name), true),
        (None, string) => (string, false),
    };
    let entry = entries
        .iter()
        .find(|(key, _)| Some(*key) == name.as_ref())
        .and_then(|(_, value)| resolve(value).as_dict().cloned())
        .unwrap_or_default();
    let shown = name.as_ref().map_or_else(String::new, |name| {
        name.as_bytes().escape_ascii().to_string()
    });

    // A filter other than the document's carrying a recipient list of its own encrypts under a
    // key of its own (Table 27's second case), and its `/Recipients` is one CMS object.
    let mut own = Vec::new();
    for (other, value) in entries.iter() {
        if Some(other) == name.as_ref() {
            continue;
        }
        let Some(dict) = resolve(value).as_dict().cloned() else {
            continue;
        };
        if dict.get("Recipients").is_none() {
            continue;
        }
        let shown = other.as_bytes().escape_ascii();
        let Some(list) = get_in(&dict, "Recipients", resolve)
            .as_string()
            .map(<[u8]>::to_vec)
        else {
            return Err(unsupported(format!(
                "the crypt filter /{shown} is neither the one /StmF or /StrF names nor carries a \
                 single CMS object in /Recipients, and Table 27 makes the entry an array only for \
                 the first and a string for a filter a stream's /Crypt decode parameters name"
            )));
        };
        let method = filters
            .named
            .get(other)
            .copied()
            .unwrap_or(Method::Identity);
        own.push(OwnList {
            name: other.clone(),
            list,
            length: filter_key_length(method, &dict, resolve)?,
        });
    }

    let lists = if entry.get("Recipients").is_some() {
        // Table 27: "If the crypt filter is referenced from StmF or StrF in the encryption
        // dictionary, this entry shall be an array of byte strings".
        byte_strings(&get_in(&entry, "Recipients", resolve), resolve).ok_or_else(|| {
            unsupported(format!(
                "Table 27's /Recipients in the crypt filter /{shown} is not an array of byte \
                 strings"
            ))
        })?
    } else if filters.stream == Method::Identity && filters.string == Method::Identity {
        Vec::new()
    } else {
        return Err(unsupported(format!(
            "the crypt filter /{shown} has no /Recipients, which Table 27 requires"
        )));
    };
    // Table 27: "used only by crypt filters that are referenced from StmF", default true.
    let encrypt_metadata = !from_stream
        || !matches!(
            get_in(&entry, "EncryptMetadata", resolve),
            Object::Boolean(false)
        );
    // §7.6.5.3: SHA-256 "for a key length of 256 bits" under Algorithm 1.A, and SHA-1 "for a key
    // length of 128 bits" under Algorithm 1. `/V` 5 is Algorithm 1.A's (Table 20); at `/V` 4
    // Table 25's `/Length` states the width of an `RC4` filter's key and the erratum to it fixes
    // `AESV2`'s at 128 bits for a public-key handler.
    let method = if from_stream {
        filters.stream
    } else {
        filters.string
    };
    let length = filter_key_length(method, &entry, resolve)?;
    Ok(RecipientLists {
        filters,
        lists,
        encrypt_metadata,
        length,
        own,
    })
}

/// The file encryption key's length, in bytes, for a crypt filter whose method is `method`.
///
/// §7.6.5.3: SHA-256 "for a key length of 256 bits" under Algorithm 1.A, and SHA-1 "for a key
/// length of 128 bits" under Algorithm 1. `/V` 5 is Algorithm 1.A's (Table 20); at `/V` 4 Table
/// 25's `/Length` states the width of an `RC4` filter's key and the erratum to it fixes `AESV2`'s
/// at 128 bits for a public-key handler.
fn filter_key_length(
    method: Method,
    entry: &Dictionary,
    resolve: &dyn Fn(&Object) -> Object,
) -> SyntaxResult<usize> {
    Ok(match method {
        Method::AesV3 => 32,
        Method::AesV2 | Method::Identity => 16,
        Method::Rc4 => match get_in(entry, "Length", resolve).as_integer() {
            Some(bits @ 40..=128) if bits % 8 == 0 => usize::try_from(bits / 8).unwrap_or(16),
            None => 16,
            Some(other) => {
                return Err(SyntaxError::UnsupportedEncryption {
                    detail: format!(
                        "Table 25's /Length {other} is not a key of 40 to 128 bits in steps of 8"
                    ),
                });
            }
        },
    })
}

/// The key of a crypt filter with its own recipient list (Table 27's second case), or `None`
/// where the reader holds no certificate its list names.
///
/// Table 27: "The enveloped data contained in the CMS object shall be a 20-byte seed value that
/// shall be used to create the file encryption key", and §7.6.5.3 says the permissions "shall be
/// absent from the enveloped data" here, so the seed is the whole of it. The key is §7.6.5.3's
/// digest over the seed and the one CMS object; its step c) is for "document-level encryption",
/// which a stream's own filter is not.
fn own_key(list: &OwnList, recipient: Option<&RecipientKey>) -> SyntaxResult<Option<Vec<u8>>> {
    let Some(recipient) = recipient else {
        return Ok(None);
    };
    let lists = std::slice::from_ref(&list.list);
    let seed = match scan(lists, recipient) {
        Ok(seed) => seed,
        Err(SyntaxError::RecipientRequired) => return Ok(None),
        Err(error) => return Err(error),
    };
    if seed.len() != 20 {
        return Err(SyntaxError::UnsupportedEncryption {
            detail: format!(
                "the crypt filter /{}'s enveloped data is {} bytes, where Table 27 puts a 20-byte \
                 seed and §7.6.5.3 no permissions",
                list.name.as_bytes().escape_ascii(),
                seed.len()
            ),
        });
    }
    Ok(Some(file_key(&seed, lists, false, list.length)))
}

/// One entry of a dictionary, resolved.
fn get_in(dict: &Dictionary, key: &str, resolve: &dyn Fn(&Object) -> Object) -> Object {
    dict.get(key).map_or(Object::Null, resolve)
}

/// An array of strings as their bytes, or `None` where it is not one.
fn byte_strings(object: &Object, resolve: &dyn Fn(&Object) -> Object) -> Option<Vec<Vec<u8>>> {
    object
        .as_array()?
        .iter()
        .map(|item| resolve(item).as_string().map(<[u8]>::to_vec))
        .collect()
}

/// §7.6.5.1's scan: the first recipient list that names the reader's certificate, opened.
///
/// Table 23 and Table 27 both settle the order: "if a recipient appears in more than one list,
/// the permissions used shall be those in the first matching list".
fn scan(lists: &[Vec<u8>], recipient: &RecipientKey) -> SyntaxResult<Vec<u8>> {
    let certificate = recipient.certificate()?;
    let mut unreadable = 0usize;
    for list in lists {
        // A list that is not an `EnvelopedData` names nobody, so it cannot be the match; it is
        // counted, so that a document none of whose lists reads is not reported as "not you".
        let Ok(envelope) = enveloped::parse(list) else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        if envelope.recipient(&certificate).is_none() {
            continue;
        }
        return envelope
            .open(&certificate, &recipient.key)
            .map_err(|error| match error {
                EnvelopeError::Decryption | EnvelopeError::KeyKind(_) => {
                    SyntaxError::RecipientKey {
                        detail: format!(
                            "the private key does not open the recipient list that names its \
                         certificate: {error}"
                        ),
                    }
                }
                other => SyntaxError::UnsupportedEncryption {
                    detail: format!("§7.6.5.3's enveloped data: {other}"),
                },
            });
    }
    if unreadable > 0 && unreadable == lists.len() {
        return Err(SyntaxError::UnsupportedEncryption {
            detail: format!(
                "none of the {unreadable} recipient lists reads as RFC 5652's EnvelopedData, \
                 which §7.6.5.1 makes them"
            ),
        });
    }
    Err(SyntaxError::RecipientRequired)
}

/// §7.6.5.3's byte array: "\[a] 20-byte seed" and "\[a] 4-byte value defining the permissions,
/// most significant byte first".
fn seed_and_permissions(array: &[u8]) -> SyntaxResult<(&[u8], u32)> {
    match (array.get(..20), array.get(20..24)) {
        (Some(seed), Some(&[a, b, c, d])) => Ok((seed, u32::from_be_bytes([a, b, c, d]))),
        _ => Err(SyntaxError::UnsupportedEncryption {
            detail: format!(
                "the enveloped data is {} bytes, where §7.6.5.3 puts a 20-byte seed and 4 bytes \
                 of permissions",
                array.len()
            ),
        }),
    }
}

/// Table 24's flag word, read with the standard handler's revision §7.6.5.3 names for the
/// `/SubFilter`. Bit 2 "permits change of encryption and enables all other permissions", which is
/// what [`Permissions::owner`] means.
fn permissions(flags: u32, sub_filter: SubFilter) -> Permissions {
    let owner = flags & 0b10 != 0;
    Permissions::from_flags(i64::from(flags), owner, sub_filter.permission_revision())
}

/// §7.6.5.3's file encryption key.
///
/// > These operations digest the following data, in order:
/// >
/// > - a) The 20 bytes of seed.
/// > - b) The bytes of each item in the Recipients array of CMS objects in the order in which
/// >   they appear in the array.
/// > - c) 4 bytes with the value 0xFF if the key being generated is intended for use in
/// >   document-level encryption and the document metadata is being left as plaintext.
/// > - d) The first n/ 8 bytes of the resulting digest shall be used as the file encryption key,
/// >   where n is the bit length of the file encryption key.
///
/// SHA-256 for the 256-bit key and SHA-1 otherwise, by the sentence above the list.
pub(super) fn file_key(
    seed: &[u8],
    lists: &[Vec<u8>],
    metadata_plaintext: bool,
    length: usize,
) -> Vec<u8> {
    fn digest<D: Digest>(
        seed: &[u8],
        lists: &[Vec<u8>],
        metadata_plaintext: bool,
        length: usize,
    ) -> Vec<u8> {
        let mut hasher = D::new();
        hasher.update(seed);
        for list in lists {
            hasher.update(list);
        }
        if metadata_plaintext {
            hasher.update([0xFF; 4]);
        }
        let mut out = hasher.finalize().to_vec();
        out.truncate(length);
        out
    }
    if length == 32 {
        digest::<Sha256>(seed, lists, metadata_plaintext, length)
    } else {
        digest::<Sha1>(seed, lists, metadata_plaintext, length)
    }
}
