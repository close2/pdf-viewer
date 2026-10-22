//! Collateral validation material and object locations from the document security store.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// §12.8.4.3's document security store: the material a later validation needs. Table 261.
///
/// §12.8.4.2 says what it is for, and the sentence is the reason this type holds bytes rather than
/// counts:
///
/// > A PDF signature may not be successfully verified unless its collateral validation components
/// > are preserved, e.g., certificates, CRLs, timestamp tokens, revocation lists, and OCSP
/// > responses.
///
/// Table 261's three arrays are "indirect references to streams", each holding one DER encoding —
/// an X.509 certificate, a CRL, or an OCSP response — and `/VRI` maps a signature to the subset of
/// them that validated it. What this type does with them is nothing: it decodes each stream and
/// names what it could not, and [`crate::revocation`] is what reads the material itself.
///
/// **These are the values rather than counts**, because the certification path is validated
/// ([`crate::trust`], ADR 1039) and revocation read (ADR 1067) from them. The counts are still
/// available, as lengths.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecurityStore {
    /// `/Certs`, each entry's decoded stream — "one DER-encoded X.509 certificate".
    pub certificates: Vec<Arc<[u8]>>,
    /// `/CRLs`, each entry's decoded stream — "a DER-encoded Certificate Revocation List (CRL)".
    pub revocation_lists: Vec<Arc<[u8]>>,
    /// `/OCSPs`, each entry's decoded stream — "a DER-encoded Online Certificate Status Protocol
    /// (OCSP) response".
    pub ocsp_responses: Vec<Arc<[u8]>>,
    /// `/VRI`, one entry per signature somebody has already validated.
    ///
    /// The clause is explicit that a VRI records only successes: "\[a] signature VRI dictionary
    /// shall not be used to record the information used in an unsuccessful validation attempt."
    pub validation_information: Vec<Vri>,
    /// What this reader would not take, by name and by where it was.
    ///
    /// Kept rather than dropped because a store this program could only half read is a fact about
    /// the document that a person checking a signature is owed: a missing CRL is the difference
    /// between an answer and [`crate::revocation::Revocation::Unknown`].
    pub refused: Vec<StoreRefusal>,
}

/// How many entries of one Table 261 array are read.
///
/// Neither §12.8.4.3 nor Table 262 states a ceiling, so this is a bound on work over a stranger's
/// file rather than a reading of the standard. A path is at most [`crate::trust::MAX_PATH_LENGTH`]
/// certificates and each needs one CRL or one response, so a store an order of magnitude past
/// [`crate::trust::MAX_CANDIDATES`] is past anything a validation can consume.
pub const MAX_STORE_ENTRIES: usize = 1024;

/// What a document security store held that this reader would not take.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StoreRefusal {
    /// An array entry is not a stream. Table 261 says each is "an indirect reference to streams".
    #[error("{key}[{index}] of this document's security store is not a stream")]
    NotAStream {
        /// The Table 261 or Table 262 key whose array it was in.
        key: &'static str,
        /// Which entry of that array.
        index: usize,
    },
    /// An array entry is a stream whose filters would not decode.
    #[error("{key}[{index}] of this document's security store is a stream that will not decode")]
    StreamNotDecodable {
        /// The Table 261 or Table 262 key whose array it was in.
        key: &'static str,
        /// Which entry of that array.
        index: usize,
    },
    /// An array states more than [`MAX_STORE_ENTRIES`] entries, so some were not read.
    #[error("{key} of this document's security store states more entries than this reader takes")]
    MoreEntriesThanRead {
        /// The Table 261 or Table 262 key whose array it was.
        key: &'static str,
    },
}

/// One signature's validation-related information — Table 262's VRI dictionary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vri {
    /// The key this dictionary sat under, as the file spells it.
    ///
    /// Table 261: "the base-16-encoded (uppercase) SHA-1 digest of the signature to which it
    /// applies". Kept as the file's own characters and never re-cased, so that a producer writing
    /// it in lower case is visible rather than silently accepted.
    pub signature_digest: String,
    /// `/Cert`, "certificates that were used in the validation of this signature".
    pub certificates: Vec<Arc<[u8]>>,
    /// `/CRL`, "all CRLs used to determine the validity of the certificates in the chains related
    /// to this signature".
    pub revocation_lists: Vec<Arc<[u8]>>,
    /// `/OCSP`, the same for OCSP responses.
    pub ocsp_responses: Vec<Arc<[u8]>>,
    /// `/TU`, "\[t]he date/time at which this signature VRI dictionary was created", as the date
    /// string the file states.
    pub created: Option<String>,
    /// `/TS`, a timestamp token over the same moment, as its decoded stream.
    pub timestamp: Option<Arc<[u8]>>,
}

impl SecurityStore {
    /// Whether the document carries any validation material at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.certificates.is_empty()
            && self.revocation_lists.is_empty()
            && self.ocsp_responses.is_empty()
            && self.validation_information.is_empty()
    }

    /// The revocation material this store holds, read as RFC 5280 and RFC 6960 structures.
    ///
    /// Everything in `/CRLs` and `/OCSPs`, and not a `/VRI`'s selection of them: Table 262 says a
    /// VRI's arrays are a subset — "\[e]ach stream shall reference a CRL that is an entry in the
    /// CRLs array in the DSS dictionary" — and §12.8.4.3 says the selection exists "for
    /// optimisation or to remove ambiguity" rather than to narrow what is applicable. Applying the
    /// whole store can only widen what is covered, and every answer is still checked against the
    /// path's own keys.
    #[must_use]
    pub fn material(&self) -> crate::revocation::Material<'_> {
        let crls: Vec<&[u8]> = self.revocation_lists.iter().map(AsRef::as_ref).collect();
        let ocsps: Vec<&[u8]> = self.ocsp_responses.iter().map(AsRef::as_ref).collect();
        crate::revocation::Material::read(&crls, &ocsps)
    }
}

/// §12.8.4.3's `/DSS`, read.
#[must_use]
pub fn security_store(document: &Document) -> SecurityStore {
    let Ok(catalog) = document.catalog() else {
        return SecurityStore::default();
    };
    let dss = document.get_key(&catalog, "DSS");
    let Some(dss) = dss.as_dict() else {
        return SecurityStore::default();
    };
    let mut store = SecurityStore::default();
    store.certificates = streams(document, dss, "Certs", &mut store.refused);
    store.revocation_lists = streams(document, dss, "CRLs", &mut store.refused);
    store.ocsp_responses = streams(document, dss, "OCSPs", &mut store.refused);
    if let Some(vri) = document.get_key(dss, "VRI").as_dict() {
        for (key, value) in vri.iter() {
            if key.as_bytes() == b"Type" {
                continue;
            }
            if store.validation_information.len() >= MAX_STORE_ENTRIES {
                store
                    .refused
                    .push(StoreRefusal::MoreEntriesThanRead { key: "VRI" });
                break;
            }
            let Some(entry) = document.resolve(value).as_dict().cloned() else {
                continue;
            };
            store.validation_information.push(Vri {
                signature_digest: String::from_utf8_lossy(key.as_bytes()).into_owned(),
                certificates: streams(document, &entry, "Cert", &mut store.refused),
                revocation_lists: streams(document, &entry, "CRL", &mut store.refused),
                ocsp_responses: streams(document, &entry, "OCSP", &mut store.refused),
                created: document
                    .get_key(&entry, "TU")
                    .as_string()
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
                timestamp: document
                    .get_key(&entry, "TS")
                    .as_stream()
                    .and_then(|stream| document.decoded_stream_data(stream)),
            });
        }
    }
    store
}

/// Where one of §12.8.4's material streams is, as the object the store names.
///
/// §12.8.5.3 is what makes a *location* worth reading beside the bytes: a later timestamp has to
/// cover the material proving the earlier authority's path, and covering something is a statement
/// about where in the file it sits. [`crate::timestamp::chain`] is the reader, and
/// [`security_store_entries`] is how it asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreEntry {
    /// The Table 261 key whose array the entry was in.
    pub key: &'static str,
    /// Which entry of that array.
    pub index: usize,
    /// The object the array names.
    ///
    /// Table 261 makes each entry "an indirect reference to streams", so an entry written as a
    /// direct object names no object at all and is not here.
    pub object: ObjectId,
}

impl StoreEntry {
    /// The object number, which is what a cross-reference table is keyed by.
    #[must_use]
    pub const fn number(&self) -> u32 {
        self.object.number
    }
}

/// The objects §12.8.4.3's three material arrays name, in the order they name them.
///
/// The counterpart of [`security_store`] for a caller that needs *where* rather than *what*. It
/// reads no stream and decodes nothing, so a document whose store will not decode still answers
/// here — which is the point: §12.8.5.3's coverage question is about the file's layout and is
/// answerable whether or not the material inside reads.
#[must_use]
pub fn security_store_entries(document: &Document) -> Vec<StoreEntry> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let dss = document.get_key(&catalog, "DSS");
    let Some(dss) = dss.as_dict() else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for key in ["Certs", "CRLs", "OCSPs"] {
        let array = document.get_key(dss, key);
        let Some(array) = array.as_array() else {
            continue;
        };
        for (index, entry) in array.iter().take(MAX_STORE_ENTRIES).enumerate() {
            if let Object::Reference(object) = entry {
                entries.push(StoreEntry {
                    key,
                    index,
                    object: *object,
                });
            }
        }
    }
    entries
}

/// One Table 261 or Table 262 array of streams, decoded, with what would not decode named.
fn streams(
    document: &Document,
    dict: &Dictionary,
    key: &'static str,
    refused: &mut Vec<StoreRefusal>,
) -> Vec<Arc<[u8]>> {
    let entries = document.get_key(dict, key);
    let Some(entries) = entries.as_array() else {
        return Vec::new();
    };
    if entries.len() > MAX_STORE_ENTRIES {
        refused.push(StoreRefusal::MoreEntriesThanRead { key });
    }
    let mut read = Vec::new();
    for (index, entry) in entries.iter().take(MAX_STORE_ENTRIES).enumerate() {
        let resolved = document.resolve(entry);
        let Some(stream) = resolved.as_stream() else {
            refused.push(StoreRefusal::NotAStream { key, index });
            continue;
        };
        match document.decoded_stream_data(stream) {
            Some(bytes) => read.push(bytes),
            None => refused.push(StoreRefusal::StreamNotDecodable { key, index }),
        }
    }
    read
}
