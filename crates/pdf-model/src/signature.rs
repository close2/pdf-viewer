//! ISO 32000-2 §12.8's digital signatures, as far as a program without a trust store can go.
//!
//! # A signature asks three questions and they need three different things
//!
//! §12.8.1 divides the subject for us, in one sentence per part:
//!
//! > To verify the signature, an appropriate signature handler is required. … The signer's
//! > certificate shall be determined and verified by the signature handler to match with any of
//! > the validation parameters and other conditions. If the verification fails, the signature
//! > shall be considered invalid. The digest shall be recomputed and compared with the one stored
//! > in the document. Differences between the two indicates that modifications have been made
//! > since the document was signed and thus the signature shall be considered invalid.
//!
//! 1. **Has the document changed since it was signed?** A digest over §12.8.1's `/ByteRange`,
//!    compared with the digest the signature value records. No certificate, no trust decision and
//!    no network — the file and a hash function. [`Signature::integrity`] **answers this**.
//! 2. **Is the signature cryptographically valid for the signer's public key?** That needs the
//!    key out of an X.509 certificate and an RSA or ECDSA verification. **Not answered**, and
//!    named as unanswered rather than skipped: [`Integrity::UnderTheSignersKey`] is the case where
//!    it is the *only* thing that could answer question 1 as well.
//! 3. **Is the signer trusted, and had the certificate been revoked?** A trust store and a
//!    network (§12.8.3.4.6's CRLs and OCSP). **Not answered**, and reported.
//!
//! ADR 0215 is the argument, and the separation is the point of it: the whole clause used to be
//! refused on question 3's infrastructure, which questions 1 and 2 do not need.
//!
//! # What a matching digest proves
//!
//! **Less than a mismatching one, and the difference decides how this is worded.** The recorded
//! digest sits *beside* the signature: whoever changes the document can change it to match, and
//! what they cannot do is make the signature over it verify. So a difference proves the bytes
//! moved after signing — §12.8.1 says exactly that — while agreement proves only that nothing
//! changed carelessly. Neither is "the signature is valid", and nothing here ever says so.
//!
//! # What is here
//!
//! [`Signature`] is Table 255. [`Permissions`] is §12.8.6's `/Perms` with §12.8.2.2's `/P`
//! level, which is what a document says may be changed without invalidating its author's
//! signature.
//!
//! [`Signature::coverage`] is the check that needs no cryptography *and* no signature value.
//! §12.8.1 says what a byte range digest covers:
//!
//! > This range should be the entire PDF file, including the signature dictionary but excluding
//! > the signature value itself (the Contents entry).
//!
//! A `/ByteRange` that stops short of the end of the file therefore names bytes **nobody signed**
//! — an incremental update appended after signing — and saying so costs one comparison against
//! the file's length. It is not a validity verdict and this module never calls it one: a
//! signature whose range covers everything may still be forged, and one that does not may be a
//! perfectly honest later revision.
//!
//! [`Signature::integrity`] is the digest, over [`crate::cms`]'s reading of §12.8.3.3's signature
//! value. [`Signature::pades_departures`] is §12.8.3.4's structural requirements on a `PAdES`
//! signature, which are checkable without cryptography and which no corpus document exercises.

use crate::cms::{self, CmsError, Digest, SignedData};
use pdf_syntax::{Dictionary, Document, Object};

/// Most signatures read from one document.
///
/// §12.8.1 permits "[o]ne or more approval signatures" and any number of timestamps, so the
/// bound is on a file built to make a reader work rather than on any real workflow.
const MAX_SIGNATURES: usize = 1024;

/// Most `(offset, length)` pairs a `/ByteRange` may state before it stops describing a file.
///
/// §12.8.1 describes two — everything before the signature value and everything after it — and
/// says "[m]ultiple discontiguous byte ranges shall be used to describe a digest that does not
/// include the signature value" without bounding them. A file stating more than this is refused
/// whole rather than read to the bound, so a truncated range never reaches the arithmetic:
/// [`Signature::coverage`] answers [`Coverage::Malformed`] and a person is told.
const MAX_BYTE_RANGE_PAIRS: usize = 64;

/// One signature dictionary. Table 255.
#[derive(Clone, PartialEq, Eq)]
pub struct Signature {
    /// `/Type`: `Sig`, or `DocTimeStamp` for §12.8.5's document timestamp.
    ///
    /// Table 255 makes the entry optional for a signature and required for a timestamp, and
    /// gives the default as `Sig` — so a dictionary stating nothing is a signature.
    pub timestamp: bool,
    /// `/Filter`, "[t]he name of the preferred signature handler to use when validating this
    /// signature".
    pub handler: Option<String>,
    /// `/SubFilter`, the encoding of the signature value — `adbe.pkcs7.detached`,
    /// `ETSI.CAdES.detached`, `ETSI.RFC3161`.
    ///
    /// Read because it decides which of §12.8.3's profiles applies, and because two of its
    /// values tighten `/ByteRange` from a *should* into a *shall*: with `ETSI.CAdES.detached` or
    /// `ETSI.RFC3161` it "shall cover the entire PDF file". [`Signature::must_cover_whole_file`]
    /// is that distinction.
    pub sub_filter: Option<String>,
    /// `/ByteRange`, "an array of pairs of integers (starting byte offset, length in bytes)".
    pub byte_range: Vec<(u64, u64)>,
    /// `/Contents`, "[t]he signature value" — the bytes as the file wrote them.
    ///
    /// §7.6.2's fourth exception keeps these out of the encryption, so they are the producer's
    /// own octets even in an encrypted document; `pdf_syntax::Document` already honours that.
    /// [`Signature::integrity`] is what reads them, through [`crate::cms`].
    pub contents: Vec<u8>,
    /// Whether the dictionary states a `/Cert`.
    ///
    /// Table 255 makes it "(Required when `SubFilter` is adbe.x509.rsa\_sha1)" and §12.8.3.4.2
    /// makes it forbidden for a `PAdES` signature — "[t]he signature dictionary shall not contain a
    /// Cert entry" — so its *presence* is the fact worth keeping, and the chain itself is X.509
    /// and a trust decision this program does not make.
    pub certificate_chain: bool,
    /// `/Name`, "[t]he name of the person or authority signing the document".
    pub name: Option<String>,
    /// `/M`, the time of signing, as the §7.9.4 date string the file wrote.
    ///
    /// The file's own bytes rather than a parse, with [`Self::signed_at_date`] beside it: 2.0% of
    /// the corpus's date strings do not conform to §7.9.4, and a reader that kept only the parse
    /// would show nothing at all for those where the producer's intent is plainly legible.
    pub signed_at: Option<String>,
    /// `/Location`, `/Reason` and `/ContactInfo` — the three text strings a signer states about
    /// *why*, kept in the clause's own order.
    pub location: Option<String>,
    /// The reason for signing.
    pub reason: Option<String>,
    /// How to reach the signer.
    pub contact: Option<String>,
    /// `/Changes`: "an array of three integers … the number of pages altered, the number of
    /// fields altered, and the number of fields filled in".
    pub changes: Option<[i64; 3]>,
    /// Whether `/Reference` names a signature reference dictionary with a `DocMDP` transform.
    ///
    /// §12.8.1 makes this what a *certification* signature is: its dictionary "shall contain a
    /// signature reference dictionary … that has a `DocMDP` transform method".
    pub certification: bool,
}

/// What a signature's `/ByteRange` covers, measured against the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// The range starts at the beginning of the file and runs to its end, with one gap — which
    /// is where `/Contents` sits, and which §12.8.1 requires to be excluded.
    WholeFile,
    /// The range stops short: this many bytes at the end of the file are outside it.
    ///
    /// An incremental update applied after signing, in the ordinary case (§7.5.6), which
    /// §12.8.1's NOTE 1 describes as the mechanism that keeps a signature meaningful. Whether
    /// those bytes are a permitted change is §12.8.2.2's question and needs the digest.
    Unsigned {
        /// How many bytes at the end of the file the range does not include.
        tail: u64,
    },
    /// The range is not two ascending pairs covering a prefix and a suffix — so what it names
    /// cannot be compared with the file at all.
    Malformed,
}

/// The answer to the first of a signature's three questions: **has the document changed?**
///
/// §12.8.1 states the check and what a difference means:
///
/// > The digest shall be recomputed and compared with the one stored in the document. Differences
/// > between the two indicates that modifications have been made since the document was signed and
/// > thus the signature shall be considered invalid.
///
/// Nothing in this enum is a verdict on the *signature*. [`Self::Changed`] is the one variant that
/// settles anything on its own — the bytes moved — and even [`Self::Unchanged`] leaves questions 2
/// and 3 open, which is why no variant is called `Valid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integrity {
    /// The bytes `/ByteRange` names still hash to the digest the signature value records.
    Unchanged {
        /// The algorithm the signature named, and this one recomputed with.
        digest: Digest,
    },
    /// They do not, so the document was modified after it was signed.
    Changed {
        /// The algorithm the signature named, and this one recomputed with.
        digest: Digest,
    },
    /// The signature value records no digest in the clear, so answering needs the signer's key.
    ///
    /// Two shapes reach this. §12.8.3.2's `adbe.x509.rsa_sha1` is "a DER-encoded PKCS #1 binary
    /// data object" — the digest is inside the RSA signature and comes out only with the public
    /// key. And a CMS `SignerInfo` with no signed attributes signs the content directly, which
    /// RFC 5652 permits and one corpus document does, so there is no `message-digest` attribute
    /// to compare against.
    UnderTheSignersKey,
    /// The signature records a digest made with an algorithm this program does not implement.
    ///
    /// All six that Table 260 and Table 256 name are implemented, so this is a signature using
    /// something the standard does not list — reported rather than guessed at, because hashing
    /// with the wrong function produces a mismatch that reads as a modified document.
    UnknownDigest,
    /// The `/ByteRange` does not name bytes of this file, so there is nothing to hash.
    RangeNotInThisFile,
    /// The dictionary states no `/Contents`, which Table 255 makes required.
    NoSignatureValue,
    /// The signature value could not be read as §12.8.3.3's CMS object.
    Unreadable(CmsError),
}

/// A requirement §12.8.3.4 places on a `PAdES` signature that a file does not meet.
///
/// Every one of these is checkable with no cryptography at all, which is why they are here: the
/// clause's *validation* steps (§12.8.3.4.5) are all certificates and revocation, and its
/// *structural* rules are arithmetic over what the file says. A departure is not a verdict — it is
/// a file breaking a `shall`, said out loud, which is what this project does with those.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadesDeparture {
    /// §12.8.3.4.2: "The `ByteRange` shall cover the entire PDF file, including the signature
    /// dictionary but excluding the Contents entry."
    RangeDoesNotCoverTheFile,
    /// §12.8.3.4.2: "The signature dictionary shall not contain a Cert entry."
    CertEntryPresent,
    /// §12.8.3.4.2: "Either the time of signing may be indicated by the value of the M entry in
    /// the signature dictionary or the signing-time attribute may be used, but not both."
    BothSigningTimesStated,
    /// §12.8.3.4.3 (a): "content-type: shall be present and shall always have the value
    /// \"id-data\"."
    ContentTypeIsNotData,
    /// §12.8.3.4.3 (d): "exactly one single `SignerInfo` attribute shall be present."
    NotExactlyOneSigner,
    /// §12.8.3.4.3 (e): "message-digest: shall be present and shall be used as defined in CMS
    /// ( Internet RFC 5652 )."
    NoMessageDigest,
    /// §12.8.3.4.3 (i): "these attributes shall not be used: counter-signature, content-reference,
    /// content-identifier, and contenthints."
    ///
    /// **Only the first of the four is checked**, and the reason is principle 5's: RFC 5652 gives
    /// `counter-signature` its object identifier, while the other three are defined in documents
    /// this tree does not hold. Naming an identifier we cannot check against the source would be
    /// asserting a fact about a specification nobody here has read.
    CounterSignature,
}

/// Everything but the signature value, which is thousands of bytes of certificate.
///
/// Written out rather than derived for one reason: `/Contents` is 33 680 bytes on the corpus's
/// largest, and a derived `Debug` puts every one of them into a test's output as a decimal
/// integer. Its *length* is the fact a reader of a log wants.
impl std::fmt::Debug for Signature {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Signature")
            .field("timestamp", &self.timestamp)
            .field("handler", &self.handler)
            .field("sub_filter", &self.sub_filter)
            .field("byte_range", &self.byte_range)
            .field("contents", &format_args!("{} bytes", self.contents.len()))
            .field("certificate_chain", &self.certificate_chain)
            .field("name", &self.name)
            .field("signed_at", &self.signed_at)
            .field("location", &self.location)
            .field("reason", &self.reason)
            .field("contact", &self.contact)
            .field("changes", &self.changes)
            .field("certification", &self.certification)
            .finish()
    }
}

impl Signature {
    /// `/M` parsed as §7.9.4's date, where the producer wrote a conforming one.
    ///
    /// `None` for a signature with no `/M` **and** for one whose `/M` breaks the clause's own
    /// grammar — two different things a caller distinguishes by looking at [`Self::signed_at`].
    /// Table 255 is careful about what this entry is worth, and the caution belongs beside the
    /// value: the time is the *signer's* claim, taken from the signer's clock, and only §12.8.5's
    /// document timestamp puts an authority behind one.
    #[must_use]
    pub fn signed_at_date(&self) -> Option<pdf_syntax::Date> {
        pdf_syntax::Date::parse(self.signed_at.as_deref()?)
    }

    /// Whether `/SubFilter` makes whole-file coverage a requirement rather than a recommendation.
    ///
    /// Table 255 says that for those two sub-filters "the `ByteRange` shall cover the entire PDF
    /// file". For every other value §12.8.1's "should" stands, and a range that stops short is a
    /// document with a later revision rather than a defective signature.
    #[must_use]
    pub fn must_cover_whole_file(&self) -> bool {
        matches!(
            self.sub_filter.as_deref(),
            Some("ETSI.CAdES.detached" | "ETSI.RFC3161")
        )
    }

    /// What this signature's range covers of a file `length` bytes long.
    ///
    /// The arithmetic is the whole of it: the first pair must start at zero, the pairs must
    /// ascend without overlapping, and the last must end at the file's end. Anything else is
    /// [`Coverage::Unsigned`] with the size of the tail, or [`Coverage::Malformed`].
    #[must_use]
    pub fn coverage(&self, length: u64) -> Coverage {
        let [(first_start, first_length), rest @ ..] = self.byte_range.as_slice() else {
            return Coverage::Malformed;
        };
        if *first_start != 0 {
            // §12.8.1: the range starts "from the \"%PDF-\" comment at the beginning of the PDF
            // document". A range that starts anywhere else has not signed the header.
            return Coverage::Malformed;
        }
        let mut end = first_start.saturating_add(*first_length);
        for (start, size) in rest {
            if *start < end {
                return Coverage::Malformed;
            }
            end = start.saturating_add(*size);
        }
        if end > length {
            return Coverage::Malformed;
        }
        if end == length {
            Coverage::WholeFile
        } else {
            Coverage::Unsigned {
                tail: length.saturating_sub(end),
            }
        }
    }

    /// The bytes this signature was made over: the file with `/ByteRange`'s hole in it.
    ///
    /// Slices rather than one buffer, because the ranges of a 3 MB document describe 3 MB and
    /// joining them to hash them would copy the whole file. `None` where any pair names bytes
    /// outside the file, which is the same condition [`Coverage::Malformed`] reports.
    #[must_use]
    pub fn signed_bytes<'a>(&self, file: &'a [u8]) -> Option<Vec<&'a [u8]>> {
        if self.byte_range.is_empty() {
            return None;
        }
        let mut pieces = Vec::with_capacity(self.byte_range.len());
        for &(start, length) in &self.byte_range {
            let start = usize::try_from(start).ok()?;
            let length = usize::try_from(length).ok()?;
            let end = start.checked_add(length)?;
            pieces.push(file.get(start..end)?);
        }
        Some(pieces)
    }

    /// §12.8.3.3's signature value, read as RFC 5652's `SignedData`.
    ///
    /// # Errors
    ///
    /// A [`CmsError`] naming what the value is instead. §12.8.3.2's `adbe.x509.rsa_sha1` is a
    /// PKCS #1 object rather than a CMS one and produces [`CmsError::NotContentInfo`] here; use
    /// [`Self::integrity`], which reads `/SubFilter` first and says the useful thing.
    pub fn signed_data(&self) -> Result<SignedData<'_>, CmsError> {
        cms::signed_data(&self.contents)
    }

    /// **Has this document changed since it was signed?**, over the bytes of `file`.
    ///
    /// The digest to compare against comes from one of three places, and which one is decided by
    /// the shape §12.8.3 gives each signature format rather than by trying them in turn:
    ///
    /// - a **document timestamp** (`ETSI.RFC3161`) commits to it in RFC 3161's `TSTInfo`, which
    ///   Table 255 states outright: "[t]he value of the messageImprint field within the
    ///   `TimeStampToken` shall be a hash of the bytes of the document indicated by the `ByteRange`";
    /// - **`adbe.pkcs7.sha1`** encapsulates it — §12.8.3.3.1: "[t]he SHA-1 digest of the
    ///   document's byte range shall be encapsulated in the CMS `SignedData` field with `ContentInfo`
    ///   of type Data";
    /// - every other CMS format is **detached**, and the digest is the signer's `message-digest`
    ///   signed attribute, which §12.8.3.4.3 (e) requires of a `PAdES` signature and RFC 5652
    ///   defines for all of them.
    ///
    /// Read [`Integrity`] before reading a result: [`Integrity::Unchanged`] is not "valid".
    #[must_use]
    pub fn integrity(&self, file: &[u8]) -> Integrity {
        // §12.8.3.2: "For signing PDF files using PKCS #1, the only value of SubFilter that should
        // be used is adbe.x509.rsa_sha1". Table 255 says such a `/Contents` "should be either a
        // DER-encoded PKCS #1 binary data object, a DER-encoded CMS binary data object or a
        // DER-encoded CMS SignedData binary data object" — and a PKCS #1 object is the signature
        // itself, with the digest inside it. There is nothing to compare without the public key,
        // and saying so is more use than "not a CMS object".
        if self.sub_filter.as_deref() == Some("adbe.x509.rsa_sha1") {
            return Integrity::UnderTheSignersKey;
        }
        if self.contents.is_empty() {
            return Integrity::NoSignatureValue;
        }
        let Some(signed) = self.signed_bytes(file) else {
            return Integrity::RangeNotInThisFile;
        };
        let cms = match self.signed_data() {
            Ok(cms) => cms,
            Err(error) => return Integrity::Unreadable(error),
        };
        // **The `/SubFilter` decides which of two digests is the document's, and guessing from the
        // encoding would be a wrong answer rather than an unknown one.** For `adbe.pkcs7.sha1` the
        // encapsulated content *is* the document's digest and the `message-digest` attribute is a
        // digest of that — so a reader that fell back to the attribute when it could not read the
        // content would compare the wrong two values and report a modified document.
        let (digest, recorded) = if self.sub_filter.as_deref() == Some("adbe.pkcs7.sha1") {
            // §12.8.3.3.1: "The SHA-1 digest of the document's byte range shall be encapsulated in
            // the CMS SignedData field with ContentInfo of type Data." The clause names the
            // algorithm, so `digestAlgorithm` — which describes the digest *of* that content — is
            // not consulted.
            let Some(encapsulated) = cms
                .encapsulated
                .filter(|_| cms.content_type == cms::ID_DATA)
            else {
                return Integrity::Unreadable(CmsError::MalformedSignedData);
            };
            (Digest::Sha1, encapsulated)
        } else if let Some(imprint) = cms.timestamp_imprint() {
            imprint
        } else if let Some(digest) = cms.message_digest {
            let Some(algorithm) = cms.digest else {
                return Integrity::UnknownDigest;
            };
            (algorithm, digest)
        } else {
            // RFC 5652 signs the encapsulated content directly when there are no signed
            // attributes, so nothing records the document's digest in the clear.
            return Integrity::UnderTheSignersKey;
        };
        if digest.compute(&signed) == recorded {
            Integrity::Unchanged { digest }
        } else {
            Integrity::Changed { digest }
        }
    }

    /// §12.8.3.4's structural requirements on a `PAdES` signature, checked against this file.
    ///
    /// Empty where the signature meets them all, and empty for a signature that is not one:
    /// §12.8.3.4.1 scopes the whole subclause to "[t]he PDF signatures using the `SubFilter` value
    /// ETSI.CAdES.detached", so applying its rules to an `adbe.pkcs7.*` signature would be this
    /// program inventing a requirement.
    ///
    /// **No corpus document is a `PAdES` signature** — all six the 974 carry in a signature field
    /// are `adbe.pkcs7.*` — so this condition is counted rather than assumed to have members, and
    /// what exercises it is a fixture.
    #[must_use]
    pub fn pades_departures(&self, cms: &SignedData<'_>, file_length: u64) -> Vec<PadesDeparture> {
        if self.sub_filter.as_deref() != Some("ETSI.CAdES.detached") {
            return Vec::new();
        }
        let mut out = Vec::new();
        if self.coverage(file_length) != Coverage::WholeFile {
            out.push(PadesDeparture::RangeDoesNotCoverTheFile);
        }
        if self.certificate_chain {
            out.push(PadesDeparture::CertEntryPresent);
        }
        if self.signed_at.is_some() && cms.has_signed_attribute(cms::ID_SIGNING_TIME) {
            out.push(PadesDeparture::BothSigningTimesStated);
        }
        if cms.content_type != cms::ID_DATA {
            out.push(PadesDeparture::ContentTypeIsNotData);
        }
        if cms.signers != 1 {
            out.push(PadesDeparture::NotExactlyOneSigner);
        }
        if cms.message_digest.is_none() {
            out.push(PadesDeparture::NoMessageDigest);
        }
        if cms.has_signed_attribute(cms::ID_COUNTERSIGNATURE)
            || cms.has_unsigned_attribute(cms::ID_COUNTERSIGNATURE)
        {
            out.push(PadesDeparture::CounterSignature);
        }
        out
    }
}

/// §12.8.6's permissions dictionary. Table 263.
///
/// > These permissions are similar to those defined by security handlers … but do not require
/// > that the document be encrypted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Permissions {
    /// `/DocMDP`'s signature, and the `/P` level its transform parameters state.
    pub doc_mdp: Option<Modification>,
    /// `/UR3`'s usage rights signature and what Table 258 says it grants.
    ///
    /// `None` where the permissions dictionary states no `/UR3`. Deprecated in PDF 2.0 — the
    /// clause opens by saying so — and read anyway, because four corpus documents carry one and
    /// §12.8.2.3 puts an obligation on a processor that *writes*.
    pub usage_rights: Option<UsageRights>,
    /// The `/UR3` **signature dictionary itself**, which no signature field points at.
    ///
    /// §12.8.1 is explicit that this one is reached only from here: a usage rights signature's
    /// "signature dictionary shall be referenced from the UR3 ( PDF 1.6 ) entry in the permissions
    /// dictionary, whose entries are listed in "Table 263 -Entries in a permissions dictionary",
    /// (not from a signature field)". So [`signatures`] cannot see it, and until it was kept here
    /// three corpus documents carried a signature this tree read the *rights* of and never the
    /// signature — including its `/ByteRange` and its digest.
    pub usage_rights_signature: Option<Signature>,
    /// The `/DocMDP` signature dictionary, likewise.
    ///
    /// Unlike `/UR3` this one is also the value of a signature field — §12.8.1 says a
    /// certification signature's dictionary "shall be the value of a signature field" and "may
    /// also be referenced from the `DocMDP` entry" — so it is normally the same object [`signatures`]
    /// returns, and it is kept because "normally" is not "always".
    pub doc_mdp_signature: Option<Signature>,
}

/// §12.8.2.3's UR transform parameters. Table 258.
///
/// # What this is for, and it is not for enabling anything
///
/// The clause's own framing is that the parameters "spec[ify] the additional rights that shall
/// be enabled if the signature is valid", which is a statement about a processor with features
/// behind a gate. This one has none: every operation it can perform, it performs on every
/// document. What it does have is the clause's other sentence, addressed to whoever writes:
///
/// > A PDF processor that modifies a PDF, with a UR signature in excess of the rights that are
/// > granted by that signature, should remove that signature prior to writing the newly
/// > modified PDF.
///
/// So the rights are read to answer one question — whether a save exceeds them — and
/// [`UsageRights::grants`] is that question. ADR 0159.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UsageRights {
    /// `/Document` — Table 258's only defined value is `FullSave`.
    pub document: Vec<String>,
    /// `/Annots` — `Create`, `Delete`, `Modify`, `Copy`, `Import`, `Export`, and PDF 1.6's
    /// `Online` and `SummaryView`.
    pub annots: Vec<String>,
    /// `/Form` — `Add`, `Delete`, `FillIn`, `Import`, `Export`, `SubmitStandalone`,
    /// `SpawnTemplate`, and PDF 1.6's `BarcodePlaintext` and `Online`.
    pub form: Vec<String>,
    /// `/Signature` — Table 258's only defined value is `Modify`.
    pub signature: Vec<String>,
    /// `/EF` — `Create`, `Delete`, `Modify`, `Import` for named embedded files.
    pub embedded_files: Vec<String>,
    /// Whether `/V` is the `2.2` §12.8.2.3's Table 258 requires.
    ///
    /// > The value shall be 2.2 . If an unknown version is present, no rights shall be enabled.
    /// > NOTE This value is a name object, not a number. Default value: 2.2 .
    ///
    /// False means the version was stated and was something else, which the clause turns into
    /// *no rights at all* rather than into a parse failure.
    pub version_understood: bool,
    /// Table 258's `/P`: "If false , any possible restriction may be ignored."
    ///
    /// Default `false`, which the table states, and which is why this is the first thing
    /// [`UsageRights::grants`] reads: a document that has not asked for its restrictions to be
    /// honoured has granted everything.
    pub restrictive: bool,
}

/// What this program does to a document, in Table 258's own vocabulary.
///
/// One variant per verb this program has. It is deliberately not the whole table: a right no
/// operation of ours can exceed is a right there is nothing to check against, and inventing an
/// enum arm for it would claim otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Right {
    /// Filling in a form field — `/Form /FillIn`, which Table 258 describes as permitting "the
    /// user to save a document on which form fill-in has been done".
    FillInForm,
    /// §12.7.6.4's import of an FDF file into the form — `/Form /Import`.
    ImportFormData,
    /// Writing the modified file — `/Document /FullSave`.
    FullSave,
}

impl UsageRights {
    /// Whether Table 258 grants this operation.
    ///
    /// Two rules come before the arrays, in this order:
    ///
    /// - `/P` false, which is the table's default, means "any possible restriction may be
    ///   ignored" — so everything is granted and nothing can be in excess.
    /// - a `/V` other than `2.2` means "no rights shall be enabled", so nothing is granted.
    ///
    /// They are read in that order because the second is a rule about the *rights*, and the
    /// first says the rights need not be consulted at all.
    #[must_use]
    pub fn grants(&self, right: Right) -> bool {
        if !self.restrictive {
            return true;
        }
        if !self.version_understood {
            return false;
        }
        let named = |list: &[String], name: &str| list.iter().any(|entry| entry == name);
        match right {
            Right::FillInForm => named(&self.form, "FillIn"),
            Right::ImportFormData => named(&self.form, "Import"),
            // Table 258's implicit-FullSave rule, and the sentence that narrows it: "If the PDF
            // document contains a UR3 dictionary, only rights specified by the Annots entry that
            // permit the document to be modified shall implicitly enable the FullSave right."
            // This *is* a UR3 dictionary, so the narrow reading is the one that applies, and the
            // four modifying annotation rights are the ones the table names.
            Right::FullSave => {
                named(&self.document, "FullSave")
                    || ["Create", "Delete", "Modify", "Import"]
                        .iter()
                        .any(|name| named(&self.annots, name))
            }
        }
    }
}

/// §12.8.2.2's `/P`: which changes the author's signature survives. Table 257.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modification {
    /// 1 — "the document shall be final; that is, any changes shall invalidate the signature",
    /// with the exception of later DSS and document-timestamp updates.
    None,
    /// 2 — form filling and signing, the first of the two "modifications that are appropriate
    /// for form field or comment workflows".
    FormFilling,
    /// 3 — form filling, signing and annotation.
    FormFillingAndAnnotation,
    /// A `/P` outside 1..=3, which Table 257 does not define.
    Unknown(i64),
}

/// Every signature the document's interactive form holds, in `/Fields` order.
///
/// §12.8.1 puts them there — a certification or approval signature "shall be the value of a
/// signature field" — so this walks `/AcroForm /Fields` rather than the whole object graph, and
/// a signature reachable no other way is one no field claims.
///
/// # The walk is skipped where the form says there is nothing to find
///
/// §12.7.3's Table 225 bit 1 is the form's own answer to the question this function asks, and the
/// clause states reading it here as the flag's purpose:
///
/// > If set, the document contains at least one signature field. This flag allows an interactive
/// > PDF processor to enable user interface items (such as menu items or push-buttons) related to
/// > signature processing without having to scan the entire document for the presence of
/// > signature fields.
///
/// Table 224 gives `/SigFlags` a default of 0, so an absent entry is a statement rather than a
/// silence: this form declares no signature fields. **This is a reading of the standard and it
/// was counted before it was trusted** — of the 974 corpus documents, 163 have an `/AcroForm`,
/// nine state `/SigFlags`, six of those set bit 1, and *exactly those six* have a signature field
/// in their tree; none of the 154 that omit the entry has one. Nothing disagrees in either
/// direction.
///
/// It is worth doing because the walk is not free and is on the launch path: `viewer_core::notes`
/// asks this the moment a document opens, and ISO 32000-2's own PDF — 28 form fields, no
/// signature — spent **1.9 ms** proving it (ADR 0181). The fields live in object streams page one
/// never touches, so it is not work the first frame pays for anyway; that was measured too.
///
/// **What this does not gate**: §12.8.6's permissions dictionary. `ViewState::save` reaches
/// §12.8.2.2's `/DocMDP` and §12.8.2.3's `/UR3` through [`permissions`], which reads the
/// catalog's own `/Perms` and never comes through here — so the `shall` in §12.8.2.2.1 about
/// preventing changes does not depend on a flag a file writes about itself.
#[must_use]
pub fn signatures(document: &Document) -> Vec<Signature> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return Vec::new();
    };
    if !signature_fields_declared(document, form) {
        return Vec::new();
    }
    let fields = document.get_key(form, "Fields");
    let Some(fields) = fields.as_array().map(<[Object]>::to_vec) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for field in &fields {
        collect(document, field, &mut out, &mut seen, 0);
    }
    out
}

/// Table 225 bit 1, `SignaturesExist`: does this form say it has a signature field?
///
/// Bit 1 is the value of `/SigFlags` and 1 in the table's numbering is the low-order bit —
/// §12.7.5.5 says the positions "shall be numbered from 1 (low-order) to 32 (high-order)", which
/// is the same convention Table 152's outline flags use and the reason this is `& 1` rather than
/// `& 2`. An entry that is not an integer is not the flag word the clause describes; it is read
/// as absent, which is the value Table 224 gives it by default.
///
/// **Bit 2, `AppendOnly`, needs nothing from this program and that is not an omission.** It asks a
/// processor to warn a person "requesting a full save that signatures will be invalidated" — a
/// *may*, and one this program cannot reach: `pdf_syntax::write` performs §7.5.6's incremental
/// update and nothing else, so every save this viewer makes is the append the flag exists to
/// steer a person towards (ADR 0121).
fn signature_fields_declared(document: &Document, form: &Dictionary) -> bool {
    document
        .get_key(form, "SigFlags")
        .as_integer()
        .is_some_and(|flags| flags & 1 != 0)
}

/// One level of the field walk, gathering the `/V` of every signature field.
fn collect(
    document: &Document,
    field: &Object,
    out: &mut Vec<Signature>,
    seen: &mut std::collections::BTreeSet<pdf_syntax::ObjectId>,
    depth: usize,
) {
    if out.len() >= MAX_SIGNATURES || depth > 32 {
        return;
    }
    if let Some(id) = field.as_reference()
        && !seen.insert(id)
    {
        return;
    }
    let resolved = document.resolve(field);
    let Some(dict) = resolved.as_dict() else {
        return;
    };
    if document
        .get_key(dict, "FT")
        .as_name()
        .is_some_and(|kind| kind.as_bytes() == b"Sig")
        && let Some(value) = document.get_key(dict, "V").as_dict()
        && let Some(signature) = read(document, value)
    {
        out.push(signature);
    }
    if let Some(kids) = document
        .get_key(dict, "Kids")
        .as_array()
        .map(<[Object]>::to_vec)
    {
        for kid in &kids {
            collect(document, kid, out, seen, depth.saturating_add(1));
        }
    }
}

/// Reads one Table 255 signature dictionary.
///
/// `None` where the dictionary states no `/ByteRange` *and* no `/Contents`, which is a field
/// that has been prepared for a signature and not signed — the common shape in an unsigned form.
#[must_use]
pub fn read(document: &Document, dict: &Dictionary) -> Option<Signature> {
    if dict.get("ByteRange").is_none() && dict.get("Contents").is_none() {
        return None;
    }
    let text = |key: &str| match document.get_key(dict, key) {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        _ => None,
    };
    let name = |key: &str| {
        document
            .get_key(dict, key)
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
    };
    Some(Signature {
        timestamp: name("Type").as_deref() == Some("DocTimeStamp"),
        handler: name("Filter"),
        sub_filter: name("SubFilter"),
        byte_range: byte_range(document, dict),
        contents: match document.get_key(dict, "Contents") {
            Object::String(bytes) => bytes.to_vec(),
            _ => Vec::new(),
        },
        certificate_chain: dict.get("Cert").is_some(),
        name: text("Name"),
        signed_at: text("M"),
        location: text("Location"),
        reason: text("Reason"),
        contact: text("ContactInfo"),
        changes: changes(document, dict),
        certification: has_transform(document, dict, b"DocMDP"),
    })
}

/// §12.8.6's `/Perms`, with the `/P` level behind its `/DocMDP`.
#[must_use]
pub fn permissions(document: &Document) -> Permissions {
    let Ok(catalog) = document.catalog() else {
        return Permissions::default();
    };
    let perms = document.get_key(&catalog, "Perms");
    let Some(perms) = perms.as_dict() else {
        return Permissions::default();
    };
    let doc_mdp = document.get_key(perms, "DocMDP");
    let ur3 = document.get_key(perms, "UR3");
    Permissions {
        doc_mdp: doc_mdp
            .as_dict()
            .and_then(|signature| modification(document, signature)),
        usage_rights: ur3
            .as_dict()
            .and_then(|signature| usage_rights(document, signature)),
        usage_rights_signature: ur3.as_dict().and_then(|dict| read(document, dict)),
        doc_mdp_signature: doc_mdp.as_dict().and_then(|dict| read(document, dict)),
    }
}

/// Table 258's parameters, found through the signature's `/Reference` chain.
///
/// The walk is `modification`'s, one transform method over: §12.8.2.1 makes `/Reference` an
/// array of signature reference dictionaries and the transform method is what says which of them
/// this is. A `/UR3` whose reference chain names no `UR` transform states no rights, which is
/// `None` rather than an empty grant — the difference matters, because an empty [`UsageRights`]
/// with `/P` true would refuse everything.
fn usage_rights(document: &Document, signature: &Dictionary) -> Option<UsageRights> {
    let references = document.get_key(signature, "Reference");
    let references = references.as_array()?.to_vec();
    for reference in &references {
        let resolved = document.resolve(reference);
        let Some(reference) = resolved.as_dict() else {
            continue;
        };
        // "UR" is the transform method's name; `/UR3` is the permissions dictionary's key for
        // the signature that carries it, and the two are not the same string. Table 259 also
        // records the older `/UR`, which producers still write.
        let is_ur = document
            .get_key(reference, "TransformMethod")
            .as_name()
            .is_some_and(|method| matches!(method.as_bytes(), b"UR3" | b"UR"));
        if !is_ur {
            continue;
        }
        let parameters = document.get_key(reference, "TransformParams");
        let Some(parameters) = parameters.as_dict() else {
            continue;
        };
        let names = |key: &str| {
            document
                .get_key(parameters, key)
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| {
                            document
                                .resolve(entry)
                                .as_name()
                                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                        })
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default()
        };
        return Some(UsageRights {
            document: names("Document"),
            annots: names("Annots"),
            form: names("Form"),
            signature: names("Signature"),
            embedded_files: names("EF"),
            version_understood: document
                .get_key(parameters, "V")
                .as_name()
                .is_none_or(|version| version.as_bytes() == b"2.2"),
            restrictive: matches!(document.get_key(parameters, "P"), Object::Boolean(true)),
        });
    }
    None
}

/// Table 257's `/P`, found through the signature's `/Reference` chain.
fn modification(document: &Document, signature: &Dictionary) -> Option<Modification> {
    let references = document.get_key(signature, "Reference");
    let references = references.as_array()?.to_vec();
    for reference in &references {
        let resolved = document.resolve(reference);
        let Some(reference) = resolved.as_dict() else {
            continue;
        };
        let is_doc_mdp = document
            .get_key(reference, "TransformMethod")
            .as_name()
            .is_some_and(|method| method.as_bytes() == b"DocMDP");
        if !is_doc_mdp {
            continue;
        }
        let parameters = document.get_key(reference, "TransformParams");
        let level = parameters
            .as_dict()
            .and_then(|parameters| document.get_key(parameters, "P").as_integer())
            // Table 257 gives `/P` a default of 2.
            .unwrap_or(2);
        return Some(match level {
            1 => Modification::None,
            2 => Modification::FormFilling,
            3 => Modification::FormFillingAndAnnotation,
            other => Modification::Unknown(other),
        });
    }
    None
}

/// Whether the signature's `/Reference` names a transform method.
fn has_transform(document: &Document, signature: &Dictionary, method: &[u8]) -> bool {
    let references = document.get_key(signature, "Reference");
    let Some(references) = references.as_array().map(<[Object]>::to_vec) else {
        return false;
    };
    references.iter().any(|reference| {
        document
            .resolve(reference)
            .as_dict()
            .and_then(|reference| {
                document
                    .get_key(reference, "TransformMethod")
                    .as_name()
                    .map(|name| name.as_bytes() == method)
            })
            .unwrap_or(false)
    })
}

/// Table 255's `/ByteRange`, as the pairs the clause states.
///
/// An array of more than [`MAX_BYTE_RANGE_PAIRS`] pairs is refused whole rather than read to the
/// bound: a range with its tail cut off would describe a different digest from the one the file
/// states, and answering a question with a silently shortened input is the failure this project
/// keeps finding elsewhere.
fn byte_range(document: &Document, dict: &Dictionary) -> Vec<(u64, u64)> {
    let range = document.get_key(dict, "ByteRange");
    let Some(range) = range.as_array() else {
        return Vec::new();
    };
    if range.len() > MAX_BYTE_RANGE_PAIRS.saturating_mul(2).saturating_add(1) {
        return Vec::new();
    }
    range
        .chunks_exact(2)
        .filter_map(|pair| {
            let start = document.resolve(pair.first()?).as_integer()?;
            let length = document.resolve(pair.get(1)?).as_integer()?;
            Some((u64::try_from(start).ok()?, u64::try_from(length).ok()?))
        })
        .collect()
}

/// Table 255's `/Changes`, "an array of three integers".
fn changes(document: &Document, dict: &Dictionary) -> Option<[i64; 3]> {
    let changes = document.get_key(dict, "Changes");
    let changes = changes.as_array()?;
    let [pages, altered, filled, ..] = changes else {
        return None;
    };
    Some([
        document.resolve(pages).as_integer()?,
        document.resolve(altered).as_integer()?,
        document.resolve(filled).as_integer()?,
    ])
}

/// §12.8.4.3's document security store: the material a later validation would need. Table 261.
///
/// The clause's own list of what it holds is a list of *certificates*, and none of it is this
/// program's to interpret: an array of certificates, "an array of all Certificate Revocation
/// Lists (CRL) (see Internet RFC 5280 )", an array of OCSP responses (RFC 6960), and a `/VRI`
/// map keyed by "the base-16-encoded (uppercase) SHA-1 digest of the signature to which it
/// applies".
///
/// So this counts them and stops. Parsing a certificate is X.509 and a trust decision, which is
/// [`Signature`]'s refusal; counting them says the one thing a person might want from a program
/// that cannot validate — **whether the document carries what a validator would need**, which is
/// the whole point of §12.8.4's "long term validation".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SecurityStore {
    /// How many certificates `/Certs` holds.
    pub certificates: usize,
    /// How many certificate revocation lists `/CRLs` holds.
    pub revocation_lists: usize,
    /// How many OCSP responses `/OCSPs` holds.
    pub ocsp_responses: usize,
    /// How many signatures `/VRI` carries validation information for.
    ///
    /// One entry per signature "that a given signature handler or PDF processor has used to
    /// successfully validate the given signature" — the clause is explicit that a VRI records
    /// only successes: "[a] signature VRI dictionary shall not be used to record the information
    /// used in an unsuccessful validation attempt."
    pub validated_signatures: usize,
}

impl SecurityStore {
    /// Whether the document carries any validation material at all.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.certificates == 0
            && self.revocation_lists == 0
            && self.ocsp_responses == 0
            && self.validated_signatures == 0
    }
}

/// §12.8.4.3's `/DSS`, counted.
#[must_use]
pub fn security_store(document: &Document) -> SecurityStore {
    let Ok(catalog) = document.catalog() else {
        return SecurityStore::default();
    };
    let dss = document.get_key(&catalog, "DSS");
    let Some(dss) = dss.as_dict() else {
        return SecurityStore::default();
    };
    let count = |key: &str| {
        document
            .get_key(dss, key)
            .as_array()
            .map_or(0, <[Object]>::len)
    };
    SecurityStore {
        certificates: count("Certs"),
        revocation_lists: count("CRLs"),
        ocsp_responses: count("OCSPs"),
        validated_signatures: document.get_key(dss, "VRI").as_dict().map_or(0, |vri| {
            vri.iter()
                .filter(|(key, _)| key.as_bytes() != b"Type")
                .count()
        }),
    }
}

/// §12.8.7's legal attestation dictionary: what the author says is in the document. Table 264.
///
/// The clause exists because a PDF can lie about itself:
///
/// > The PDF language provides a number of capabilities that can make the rendered appearance of
/// > a PDF document vary. These capabilities could potentially be used to construct a document
/// > that misleads the recipient of a document, intentionally or unintentionally.
///
/// So an author certifying a document is asked to declare the counts of the things that could —
/// scripts, launch actions, alternate images, external streams — and a reviewer can weigh them.
///
/// **This is a document stating a fact this program can check.** [`Legal::disagreements`] counts
/// the same things over the object graph and names every entry where the two differ, which is the
/// habit §12.3.3's `/Count`, an LZW stream's length and §12.4.3's bead arrays all taught: a file
/// that says the same thing twice can be held to it. A disagreement is not proof of anything —
/// the clause states no algorithm for counting, and an author's tool may count a shared action
/// once where this one counts two references — but it is exactly the kind of thing "any
/// questionable content can be reviewed in the context of" means.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Legal {
    /// The counts the dictionary states, by their Table 264 key.
    pub stated: Vec<(String, i64)>,
    /// `/Attestation`, the author's own words about the entries above.
    pub attestation: Option<String>,
}

impl Legal {
    /// Entries whose stated count differs from what this reader finds, as `(key, stated, found)`.
    ///
    /// Only the entries a renderer can count on its own evidence are checked — the action types,
    /// which `action.rs` already names, and the external streams §7.3.8.1 refuses. The rest of
    /// Table 264 is counted by nobody here and is left out rather than guessed at.
    #[must_use]
    pub fn disagreements(&self, document: &Document) -> Vec<(String, i64, i64)> {
        let found = census(document);
        self.stated
            .iter()
            .filter_map(|(key, stated)| {
                let counted = found.iter().find(|(name, _)| name == key)?.1;
                (counted != *stated).then(|| (key.clone(), *stated, counted))
            })
            .collect()
    }
}

/// §12.8.7's `/Legal`, where the catalog states one.
#[must_use]
pub fn legal(document: &Document) -> Option<Legal> {
    let catalog = document.catalog().ok()?;
    let legal = document.get_key(&catalog, "Legal");
    let legal = legal.as_dict()?;
    Some(Legal {
        stated: legal
            .iter()
            .filter_map(|(key, value)| {
                Some((
                    String::from_utf8_lossy(key.as_bytes()).into_owned(),
                    document.resolve(value).as_integer()?,
                ))
            })
            .collect(),
        attestation: match document.get_key(legal, "Attestation") {
            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        },
    })
}

/// Counts the Table 264 entries this program can count, over every object in the document.
///
/// A whole-object walk, which is why nothing calls it while a page is being drawn: this answers a
/// question about the *document* and is asked once, by somebody who wants to know whether its
/// author's declaration holds.
fn census(document: &Document) -> Vec<(String, i64)> {
    // Every key this program can count, starting at zero — because a *missing* count and a count
    // of zero are different answers: the first means "not countable here" and is left out of the
    // comparison, the second means "the author declared some and there are none".
    let mut counts: std::collections::BTreeMap<&'static str, i64> = [
        "JavaScriptActions",
        "LaunchActions",
        "URIActions",
        "MovieActions",
        "SoundActions",
        "HideAnnotationActions",
        "GoToRemoteActions",
        "AlternateImages",
        "ExternalStreams",
        "TrueTypeFonts",
    ]
    .into_iter()
    .map(|key| (key, 0))
    .collect();
    for number in document.xref().object_numbers() {
        let object = document.get(pdf_syntax::ObjectId {
            number,
            generation: 0,
        });
        let dict = match &object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        if let Some(action) = dict.get("S").and_then(Object::as_name) {
            let key = match action.as_bytes() {
                b"JavaScript" => Some("JavaScriptActions"),
                b"Launch" => Some("LaunchActions"),
                b"URI" => Some("URIActions"),
                b"Movie" => Some("MovieActions"),
                b"Sound" => Some("SoundActions"),
                b"Hide" => Some("HideAnnotationActions"),
                b"GoToR" => Some("GoToRemoteActions"),
                _ => None,
            };
            if let Some(key) = key {
                let slot = counts.entry(key).or_default();
                *slot = slot.saturating_add(1);
            }
        }
        if dict.get("Alternates").is_some() {
            let slot = counts.entry("AlternateImages").or_default();
            *slot = slot.saturating_add(1);
        }
        // §7.3.8.1's external stream: the bytes are in a file rather than in the document, which
        // this tree refuses by name — and which Table 264 counts for the same reason it exists.
        if matches!(&object, Object::Stream(_)) && dict.get("F").is_some() {
            let slot = counts.entry("ExternalStreams").or_default();
            *slot = slot.saturating_add(1);
        }
        if dict
            .get("Subtype")
            .and_then(Object::as_name)
            .is_some_and(|subtype| subtype.as_bytes() == b"TrueType")
        {
            let slot = counts.entry("TrueTypeFonts").or_default();
            *slot = slot.saturating_add(1);
        }
    }
    counts
        .into_iter()
        .map(|(key, count)| (key.to_owned(), count))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        Coverage, Integrity, Modification, PadesDeparture, legal, permissions, security_store,
        signatures,
    };
    use crate::cms::{Digest, fixtures};
    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        Document::open(document_bytes(objects)).expect("a valid file")
    }

    /// The same, stopping at the bytes — which is what a signature is over.
    fn document_bytes(objects: &[&str]) -> Vec<u8> {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        out.into_bytes()
    }

    /// A certification signature, its permissions, and what its range covers.
    #[test]
    fn a_certification_signature_states_what_may_change_after_it() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> \
             /Perms << /DocMDP 5 0 R >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 840 960 240] /Name (A. Author) /M (D:20260801120000+02'00') \
             /Reason (I approve) /Location (Zurich) /Contents <00> /Reference [6 0 R] \
             /Changes [1 2 3] >>",
            "<< /Type /SigRef /TransformMethod /DocMDP /TransformParams << /Type /TransformParams /P 2 /V /1.2 >> >>",
        ]);

        let found = signatures(&doc);
        let [signature] = found.as_slice() else {
            panic!("one signature, got {found:?}");
        };
        assert!(!signature.timestamp);
        assert_eq!(signature.handler.as_deref(), Some("Adobe.PPKLite"));
        assert_eq!(signature.name.as_deref(), Some("A. Author"));
        assert_eq!(signature.reason.as_deref(), Some("I approve"));
        assert_eq!(signature.location.as_deref(), Some("Zurich"));
        assert_eq!(signature.changes, Some([1, 2, 3]));
        assert!(
            signature.certification,
            "a /Reference with a DocMDP transform is what makes it a certification signature"
        );
        assert!(
            !signature.must_cover_whole_file(),
            "adbe.pkcs7.detached leaves §12.8.1's \"should\" as a should"
        );

        assert_eq!(
            signature.coverage(1200),
            Coverage::WholeFile,
            "0..840 and 960..1200 leave one gap, which is where /Contents sits"
        );
        assert_eq!(
            signature.coverage(1400),
            Coverage::Unsigned { tail: 200 },
            "two hundred bytes were appended after signing"
        );
        assert_eq!(
            signature.coverage(1000),
            Coverage::Malformed,
            "a range past the end of the file names bytes that are not there"
        );

        assert_eq!(
            permissions(&doc).doc_mdp,
            Some(Modification::FormFilling),
            "/P 2 permits form filling and signing"
        );
        assert!(permissions(&doc).usage_rights.is_none());
    }

    /// A prepared but unsigned signature field is not a signature.
    ///
    /// The common shape in a blank form: the field exists so that somebody can sign it, and its
    /// `/V` is absent or empty. Reading it as a signature would report every unsigned form as
    /// carrying one.
    #[test]
    fn an_unsigned_signature_field_holds_no_signature() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Empty) /Subtype /Widget >>",
            "<< /FT /Sig /T (AlsoEmpty) /V << /Type /Sig /Filter /Adobe.PPKLite >> >>",
        ]);
        assert!(signatures(&doc).is_empty());
    }

    /// A form that declares no signature fields is taken at its word and not walked.
    ///
    /// **The deliberate reading**, and the one thing this module believes a file about: Table 225
    /// bit 1 exists so that a processor need not "scan the entire document for the presence of
    /// signature fields", and Table 224 defaults `/SigFlags` to 0. So the same objects that
    /// `a_certification_signature_states_what_may_change_after_it` reads a whole signature out of
    /// yield nothing when the flag is taken away — a difference in the *form's own declaration*,
    /// not in what it holds. The standard's own worked example in §12.8.5 writes `/SigFlags 3`
    /// beside its one signature field, and of the 974 corpus documents none that omits the entry
    /// has a signature field. ADR 0181.
    #[test]
    fn a_form_declaring_no_signature_fields_is_not_walked() {
        let objects: [&str; 6] = [
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> \
             /Perms << /DocMDP 5 0 R >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 840 960 240] /Name (A. Author) /Contents <00> /Reference [6 0 R] >>",
            "<< /Type /SigRef /TransformMethod /DocMDP \
             /TransformParams << /Type /TransformParams /P 2 /V /1.2 >> >>",
        ];
        assert!(signatures(&document(&objects)).is_empty());

        // And §12.8.6's permissions come from the catalog's own `/Perms`, so they are still
        // read — which is what keeps §12.8.2.2.1's `shall` about preventing changes off this
        // flag entirely.
        assert_eq!(
            permissions(&document(&objects)).doc_mdp,
            Some(Modification::FormFilling)
        );
    }

    /// A range that does not start at the beginning of the file has not signed the header.
    #[test]
    fn a_range_that_starts_late_is_malformed() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (S) /V << /ByteRange [8 100] /Contents <00> \
             /SubFilter /ETSI.CAdES.detached >> >>",
        ]);
        let found = signatures(&doc);
        let [signature] = found.as_slice() else {
            panic!("one signature");
        };
        assert!(
            signature.must_cover_whole_file(),
            "ETSI.CAdES.detached turns §12.8.1's should into Table 255's shall"
        );
        assert_eq!(signature.coverage(200), Coverage::Malformed);
    }

    /// §12.8.4's store, counted, and §12.8.7's declaration, checked against the document.
    ///
    /// The `/Legal` dictionary states four counts; three of them are wrong on purpose, and the
    /// fourth is the URI action the document actually holds. What comes back is the difference,
    /// which is the only thing a reader can honestly offer: the clause states no counting
    /// algorithm, so a disagreement is a question rather than a verdict.
    #[test]
    fn a_documents_own_declaration_can_be_held_against_it() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /DSS 3 0 R \
             /Legal << /JavaScriptActions 0 /URIActions 1 /LaunchActions 2 /TrueTypeFonts 0 \
             /Attestation (Nothing here moves.) >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Type /DSS /Certs [5 0 R 6 0 R] /OCSPs [6 0 R] \
             /VRI << /ABCDEF << /Cert [5 0 R] >> >> >>",
            "<< /S /URI /URI (http://example.invalid/) >>",
            "<< /Length 0 >>\nstream\n\nendstream",
            "<< /Length 0 >>\nstream\n\nendstream",
        ]);

        let store = security_store(&doc);
        assert_eq!(store.certificates, 2);
        assert_eq!(store.revocation_lists, 0);
        assert_eq!(store.ocsp_responses, 1);
        assert_eq!(store.validated_signatures, 1);
        assert!(!store.is_empty());

        let legal = legal(&doc).expect("a /Legal dictionary");
        assert_eq!(legal.attestation.as_deref(), Some("Nothing here moves."));
        assert_eq!(
            legal.disagreements(&doc),
            vec![("LaunchActions".to_owned(), 2, 0)],
            "the URI action agrees; the two launch actions the author declared are not there"
        );
    }

    /// A whole document signed the way §12.8.1 says one is, so the digest can be checked.
    ///
    /// This is what a signature handler does, minus the private key: write the dictionary with a
    /// hole where the signature value will go, fill in `/ByteRange` to name everything *but* that
    /// hole, digest what is named, and write a signature value committing to that digest. Every
    /// substitution is length-preserving — ten-digit offsets and a fixed-width hexadecimal string,
    /// which is how real producers do it — so the cross-reference table stays right.
    ///
    /// Returns the file's bytes. `sign` receives the digest of the signed range and returns the
    /// signature value to put in the hole, which is how one builder serves a detached signature, a
    /// document timestamp and a signer with no attributes at all.
    fn signed_document(
        sub_filter: &str,
        extra: &str,
        digest: Digest,
        sign: impl Fn(&[u8]) -> Vec<u8>,
    ) -> Vec<u8> {
        use std::fmt::Write as _;
        /// Hexadecimal characters reserved for the signature value.
        const ROOM: usize = 2048;
        // No `/Type`: Table 255 makes it "(Optional if Sig)" with "[t]he default value is: Sig .",
        // so this is the shape a signature dictionary is permitted to have, and the one
        // `issue17069.pdf` actually has. `extra` is where a document timestamp states its own.
        let signature = format!(
            "<< /Filter /Adobe.PPKLite /SubFilter /{sub_filter} {extra} \
             /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <{}> >>",
            "0".repeat(ROOM)
        );
        let mut bytes = document_bytes(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget >>",
            &signature,
        ]);

        let open = bytes
            .windows(11)
            .position(|window| window == b"/Contents <")
            .expect("the /Contents hole")
            .saturating_add(10);
        assert_eq!(bytes[open], b'<');
        assert_eq!(bytes[open.saturating_add(ROOM).saturating_add(1)], b'>');
        // §12.8.1's range: everything up to the value, and everything after it.
        let after = open.saturating_add(ROOM).saturating_add(2);
        let tail = bytes.len().saturating_sub(after);
        let hole = b"[0000000000 0000000000 0000000000 0000000000]";
        let range = format!("[{:010} {open:010} {after:010} {tail:010}]", 0);
        assert_eq!(range.len(), hole.len());
        let at = bytes
            .windows(hole.len())
            .position(|window| window == hole)
            .expect("the /ByteRange hole");
        bytes.splice(at..at.saturating_add(hole.len()), range.bytes());

        let value = sign(&digest.compute(&[&bytes[..open], &bytes[after..]]));
        let hex = value.iter().fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        });
        assert!(hex.len() <= ROOM, "the signature value fits its hole");
        let value_at = open.saturating_add(1);
        bytes.splice(value_at..value_at.saturating_add(hex.len()), hex.bytes());
        bytes
    }

    /// The signature of the document, and the file it came from.
    fn only_signature(bytes: &[u8]) -> (Document, super::Signature) {
        let document = Document::open(bytes.to_vec()).expect("a valid file");
        let found = signatures(&document);
        let [signature] = found.as_slice() else {
            panic!("one signature, got {found:?}");
        };
        (document, signature.clone())
    }

    /// **Question 1, both ways round**: a document that has not changed, and one byte that did.
    ///
    /// §12.8.1: "The digest shall be recomputed and compared with the one stored in the document.
    /// Differences between the two indicates that modifications have been made since the document
    /// was signed". The second half of this test is the one worth having — a check that only ever
    /// says "unchanged" has not been shown to check anything — and the byte it moves is inside the
    /// signed range, not in the tail an incremental update is allowed to add.
    #[test]
    fn a_signed_document_says_whether_its_signed_bytes_moved() {
        let bytes = signed_document(
            "adbe.pkcs7.detached",
            "",
            Digest::Sha256,
            fixtures::detached,
        );
        let (document, signature) = only_signature(&bytes);
        assert_eq!(
            signature.coverage(bytes.len() as u64),
            Coverage::WholeFile,
            "the range names everything but the value"
        );
        assert_eq!(
            signature.integrity(document.bytes()),
            Integrity::Unchanged {
                digest: Digest::Sha256
            }
        );

        let mut altered = bytes.clone();
        let at = altered
            .windows(9)
            .position(|nine| nine == b"Signature")
            .expect("a byte inside the signed range");
        altered[at] = b'X';
        let (altered_document, altered_signature) = only_signature(&altered);
        assert_eq!(
            altered_signature.integrity(altered_document.bytes()),
            Integrity::Changed {
                digest: Digest::Sha256
            },
            "one byte of the signed range, and the recomputed digest no longer matches"
        );

        // And a byte *after* the signed range is §12.8.1's NOTE 1 — an incremental update — which
        // the digest is not entitled to notice and `Coverage` is.
        let mut appended = bytes.clone();
        appended.extend_from_slice(b"% a later revision would go here\n");
        let (appended_document, appended_signature) = only_signature(&appended);
        assert_eq!(
            appended_signature.integrity(appended_document.bytes()),
            Integrity::Unchanged {
                digest: Digest::Sha256
            }
        );
        assert_eq!(
            appended_signature.coverage(appended.len() as u64),
            Coverage::Unsigned { tail: 33 }
        );
    }

    /// A signer with no signed attributes records no digest, and the program says which it is.
    ///
    /// RFC 5652 then signs the encapsulated content directly, so question 1 has no answer that
    /// does not come through question 2's public key. `bug854315.pdf` is this shape, which is why
    /// the corpus gate holds the count at one rather than at zero.
    #[test]
    fn a_signature_with_no_message_digest_says_the_answer_is_under_the_key() {
        let bytes = signed_document("adbe.pkcs7.detached", "", Digest::Sha256, |_| {
            fixtures::without_signed_attributes()
        });
        let (document, signature) = only_signature(&bytes);
        assert_eq!(
            signature.integrity(document.bytes()),
            Integrity::UnderTheSignersKey
        );
    }

    /// §12.8.3.3.1's `adbe.pkcs7.sha1` commits to the digest in its *content*, not its attribute.
    ///
    /// The clause puts "[t]he SHA-1 digest of the document's byte range" in the encapsulated
    /// content and makes the `message-digest` attribute a digest of *that*, so the two are
    /// different values and reaching for the wrong one reports a document that never changed as
    /// one that did. The fixture's attribute is deliberately nonsense for exactly that reason.
    /// **The corpus's only witness is a fuzzed file**, so this is the shape the clause states
    /// rather than the shape one document happens to have (trap 8).
    #[test]
    fn an_encapsulating_signature_is_checked_against_its_content_and_not_its_attribute() {
        let bytes = signed_document("adbe.pkcs7.sha1", "", Digest::Sha1, fixtures::encapsulating);
        let (document, signature) = only_signature(&bytes);
        assert_eq!(
            signature.integrity(document.bytes()),
            Integrity::Unchanged {
                digest: Digest::Sha1
            }
        );

        // And a value that states that sub-filter without encapsulating anything is unreadable
        // rather than silently compared against the attribute.
        let detached = signed_document("adbe.pkcs7.sha1", "", Digest::Sha1, fixtures::detached);
        let (detached_document, detached_signature) = only_signature(&detached);
        assert_eq!(
            detached_signature.integrity(detached_document.bytes()),
            Integrity::Unreadable(crate::cms::CmsError::MalformedSignedData)
        );
    }

    /// §12.8.3.2's PKCS #1 signature is refused before its value is even looked at.
    ///
    /// The clause makes `adbe.x509.rsa_sha1` "the only value of `SubFilter` that should be used" for
    /// PKCS #1, and such a value is the RSA signature itself with the digest inside it. Reading it
    /// as a CMS object would report "not a CMS `ContentInfo`", which is true and useless; what a
    /// person needs to hear is that the answer is behind the signer's key.
    #[test]
    fn a_pkcs1_signature_names_the_key_rather_than_the_encoding() {
        let bytes = signed_document("adbe.x509.rsa_sha1", "", Digest::Sha1, |_| {
            vec![0x01, 0x02, 0x03]
        });
        let (document, signature) = only_signature(&bytes);
        assert_eq!(
            signature.integrity(document.bytes()),
            Integrity::UnderTheSignersKey
        );
    }

    /// §12.8.5's document timestamp commits to the same digest in RFC 3161's `TSTInfo`.
    ///
    /// Table 255 says so of a `DocTimeStamp`'s `/Contents`: "[t]he value of the messageImprint
    /// field within the `TimeStampToken` shall be a hash of the bytes of the document indicated by
    /// the `ByteRange` and the `ByteRange` shall specify the complete PDF file contents (excepting the
    /// Contents value)." **No corpus document carries one**, so this fixture is the only thing
    /// that exercises the path.
    #[test]
    fn a_document_timestamp_is_checked_against_its_message_imprint() {
        let bytes = signed_document(
            "ETSI.RFC3161",
            "/Type /DocTimeStamp",
            Digest::Sha256,
            fixtures::timestamp_token,
        );
        let document = Document::open(bytes.clone()).expect("a valid file");
        let found = signatures(&document);
        let [timestamp] = found.as_slice() else {
            panic!("one timestamp, got {found:?}");
        };
        assert!(timestamp.timestamp, "/Type DocTimeStamp");
        assert!(
            timestamp.must_cover_whole_file(),
            "ETSI.RFC3161 makes the whole-file range a shall"
        );
        assert_eq!(
            timestamp.integrity(document.bytes()),
            Integrity::Unchanged {
                digest: Digest::Sha256
            }
        );
    }

    /// §12.8.3.4's structural requirements on a `PAdES` signature, which no corpus document has.
    ///
    /// The fixture breaks three of them at once and meets the rest: its `/ByteRange` stops short
    /// of the file's end (§12.8.3.4.2), it states a `/Cert` (§12.8.3.4.2), and it states both an
    /// `/M` and a `signing-time` attribute (§12.8.3.4.2 again, which permits "but not both").
    /// Content type, signer count and message digest are all as §12.8.3.4.3 requires, so their
    /// absence from the answer is as much of the test as the three that are there.
    #[test]
    fn a_pades_signature_is_held_to_the_rules_that_need_no_certificate() {
        let bytes = signed_document(
            "ETSI.CAdES.detached",
            "/M (D:20260807000000Z) /Cert <00>",
            Digest::Sha256,
            fixtures::detached,
        );
        let (document, signature) = only_signature(&bytes);
        let cms = signature.signed_data().expect("a SignedData");
        assert_eq!(
            signature.integrity(document.bytes()),
            Integrity::Unchanged {
                digest: Digest::Sha256
            },
            "the digest is right; what follows is about the structure around it"
        );
        // One byte longer than the file the range describes, so the range stops short of the end.
        assert_eq!(
            signature.pades_departures(&cms, bytes.len() as u64 + 1),
            vec![
                PadesDeparture::RangeDoesNotCoverTheFile,
                PadesDeparture::CertEntryPresent,
                PadesDeparture::BothSigningTimesStated,
            ]
        );

        // And an `adbe.pkcs7.detached` signature with the same three faults has none of these
        // departures, because §12.8.3.4.1 scopes the whole subclause to ETSI.CAdES.detached.
        let other = signed_document(
            "adbe.pkcs7.detached",
            "/M (D:20260807000000Z) /Cert <00>",
            Digest::Sha256,
            fixtures::detached,
        );
        let (_, other_signature) = only_signature(&other);
        let other_cms = other_signature.signed_data().expect("a SignedData");
        assert!(
            other_signature
                .pades_departures(&other_cms, other.len() as u64 + 1)
                .is_empty()
        );
    }
}
