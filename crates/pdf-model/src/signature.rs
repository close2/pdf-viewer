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
//! 2. **Does the signature verify under the signer's public key?** That needs the key out of an
//!    X.509 certificate ([`crate::x509`]) and the arithmetic of whichever family Table 260 names:
//!    RSA — under both of RFC 8017's paddings, [`crate::pkcs1`] and [`crate::pss`] — DSA
//!    ([`crate::dsa`]), ECDSA ([`crate::ecdsa`]), or the `EdDSA` row ISO/TS 32002 section 5.1.2 adds
//!    to that table ([`crate::eddsa`]). [`Signature::authenticity`] **answers this** for all four.
//!    What is left inside it is a *curve* rather than a family: three of ISO/TS 32002 Table 3's
//!    six and one of its Table 4's two are named by their own identifier and computed by nothing,
//!    for the reason ADR 0532 records.
//! 3. **Is the signer trusted, and had the certificate been revoked?** A trust store and a
//!    network (§12.8.3.4.6's CRLs and OCSP). **Not answered**, and reported.
//!
//! ADR 0215 separated the three and answered the first; ADR 0229 answered the second for RSA, ADR
//! 0314 for DSA, ADR 0322 for the RSA family's other padding and ADR 0532 for the two
//! elliptic-curve families. The separation is the point of all of them: the whole clause used to
//! be refused on question 3's infrastructure, which questions 1 and 2 do not need.
//!
//! # What each answer proves, which is not the same thing
//!
//! **A matching digest proves less than a mismatching one.** The recorded digest sits *beside* the
//! signature: whoever changes the document can change it to match, and what they cannot do is make
//! the signature over it verify. So a difference proves the bytes moved after signing — §12.8.1
//! says exactly that — while agreement alone proves only that nothing changed carelessly.
//!
//! **A verified signature proves the signature and the certificate belong together, and the
//! certificate arrived in the same file.** §12.8.3.3.1 requires the signer's certificate to be in
//! the signature value, so verifying against it is a self-consistency check — a real one, and the
//! thing a forger who edits the document cannot produce, but not a statement that the signer is
//! anybody. Nothing here is called `Valid` and nothing this program prints uses the word.
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
//! value; [`Signature::authenticity`] is the verification, over [`crate::x509`]'s reading of the
//! certificate that value carries. [`Signature::pades_departures`] is §12.8.3.4's structural
//! requirements on a `PAdES` signature, which are checkable without cryptography and which no
//! corpus document exercises.

use crate::cms::{self, CmsError, Digest, SignatureAlgorithm, SignedData};
use crate::dsa::{self, DsaError};
use crate::ecdsa::{self, EcdsaError};
use crate::eddsa::{self, EdDsaError};
use crate::pkcs1::{self, Pkcs1Error};
use crate::pss;
use crate::x509::{self, X509Error};
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

/// Most certificates read out of one `/Cert` entry.
///
/// A certification path of more than sixty-four certificates is not one; this bounds an
/// allocation whose size would otherwise come out of the file.
const MAX_CHAIN: usize = 64;

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
    /// Cert entry" — so its *presence* is a fact in its own right, separate from what it holds:
    /// a `/Cert` stating an empty array breaks §12.8.3.4.2 and yields no certificate.
    pub certificate_chain: bool,
    /// `/Cert`'s certificates, in the order the entry states them.
    ///
    /// Table 255: "[a]n array of byte strings that shall represent the X.509 certificate chain
    /// used when signing and verifying signatures that use public-key cryptography, or a byte
    /// string if the chain has only one entry." The first is the signer's — the table says so:
    /// "[t]he signing certificate shall appear first in the array". This is the only place a
    /// §12.8.3.2 signature's key can come from, because a PKCS #1 value carries no certificate.
    pub chain: Vec<Vec<u8>>,
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
    /// All ten those two tables name between them — the base standard's six and ISO/TS 32001
    /// section 5.1.4's four — are implemented, so this is a signature using something neither
    /// document lists, reported rather than guessed at: hashing with the wrong function produces a
    /// mismatch that reads as a modified document.
    UnknownDigest,
    /// The `/ByteRange` does not name bytes of this file, so there is nothing to hash.
    RangeNotInThisFile,
    /// The dictionary states no `/Contents`, which Table 255 makes required.
    NoSignatureValue,
    /// The signature value could not be read as §12.8.3.3's CMS object.
    Unreadable(CmsError),
}

/// What a signature's bytes were computed over — which decides what verifying one *proves*.
///
/// RFC 5652 section 5.4 makes this a two-way fork and §12.8.3 adds a third case, and the three do not
/// bind the document equally. This is the difference between a signature that answers question 1
/// on its own and one that answers it only through the digest question 1 compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signed {
    /// The signer's signed attributes, re-encoded as RFC 5652 section 5.4's `SET OF`.
    ///
    /// One of those attributes is `message-digest`, which is the digest
    /// [`Signature::integrity`] compares — so a verified signature puts the *recorded digest*
    /// under the signer's key, and the document follows only if that digest also matches. The two
    /// answers together are what settles anything; either alone does not.
    SignedAttributes,
    /// The encapsulated content, where the signer states no signed attributes and the CMS object
    /// carries one.
    ///
    /// For `adbe.pkcs7.sha1` that content *is* the document's digest — §12.8.3.3.1: "[t]he SHA-1
    /// digest of the document's byte range shall be encapsulated in the CMS `SignedData` field" —
    /// so the same pairing applies as above.
    EncapsulatedContent,
    /// The bytes `/ByteRange` names, directly.
    ///
    /// RFC 5652 signs the content itself when there are no signed attributes, and for a detached
    /// signature that content is the document. **This is the case where question 2 answers
    /// question 1 as well**: nothing sits between the signer's key and the file's bytes, so a
    /// signature that verifies proves the bytes did not move and one that does not proves nothing
    /// about which of the two changed. §12.8.3.2's `adbe.x509.rsa_sha1` reaches it too, by a
    /// different route: its `/Contents` is the PKCS #1 signature over the byte range with no CMS
    /// structure at all.
    TheDocumentsBytes,
}

impl Signed {
    /// Whether verifying a signature over this settles [`Signature::integrity`] as well.
    #[must_use]
    pub fn binds_the_document(self) -> bool {
        matches!(self, Self::TheDocumentsBytes)
    }
}

/// The answer to the second of a signature's three questions: **does it verify under the key in
/// the certificate the file carries?**
///
/// §12.8.3.3.1 states both the requirement and, exactly, what this can be worth:
///
/// > At minimum the CMS object shall include the signer's X.509 signing certificate. This
/// > certificate shall be used to verify the signature value in Contents .
///
/// **That certificate came out of the same file as the signature.** So [`Self::Verified`] says
/// the signature was made by whoever holds the private key matching a certificate the file
/// itself supplied — self-consistency, and a real fact: it is what a forger who edits the
/// document cannot produce. What it is *not* is a statement that the signer is anybody. That is
/// question 3, needs a certificate store and a network, and this program answers none of it.
///
/// No variant is called `Valid`, for the same reason no variant of [`Integrity`] is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authenticity {
    /// The signature verifies under the key in the signer's certificate.
    Verified {
        /// The digest algorithm the signature named, and this one recomputed with.
        digest: Digest,
        /// Which of Table 260's algorithm families did the verifying.
        family: Family,
        /// The key's width in bits, which is the number Table 260 puts a ceiling on.
        key_bits: usize,
        /// What the signature was computed over, which decides what this proves.
        over: Signed,
    },
    /// It does not.
    ///
    /// Decisive in one direction only, and the direction is the opposite of [`Integrity`]'s: a
    /// signature that fails to verify says the value, the key or the signed bytes are not the
    /// three that were put together at signing time, and does not say which.
    NotUnderThatKey {
        /// The digest algorithm the signature named.
        digest: Digest,
        /// Which of Table 260's algorithm families was tried.
        family: Family,
        /// The key's width in bits.
        key_bits: usize,
        /// What the signature was computed over.
        over: Signed,
    },
    /// No certificate in the file matches the signer the signature names.
    ///
    /// §12.8.3.3.1's "at minimum" requirement unmet, or met by a certificate for a different
    /// signer. The count is carried because "none at all" and "three, none of them the signer's"
    /// are different statements about a file.
    NoSignerCertificate {
        /// How many certificates the file offered.
        certificates: usize,
    },
    /// The signer's certificate would not parse.
    CertificateUnreadable(X509Error),
    /// The certificate's public key is one this program does not act on, by the identifier it
    /// states.
    ///
    /// `id-Ed448` is the standing example: ISO/TS 32002 Table 4 names the curve and
    /// [`crate::eddsa`] says why no package on this tree's line computes it. So this is a gap in
    /// this program rather than a defect in the file — which is why the algorithm is carried out
    /// to a person by its number. A key `adbe.x509.rsa_sha1` may not carry at all arrives here
    /// too, and that one is Table 260's "No" rather than a gap.
    KeyNotVerifiable {
        /// The algorithm's object identifier as dotted decimal, or its octets in hexadecimal
        /// where the encoding is not a well-formed identifier.
        algorithm: String,
    },
    /// The key is `id-ecPublicKey` on a curve this program does not compute on.
    ///
    /// Separate from [`Self::KeyNotVerifiable`] because the identifier that matters is a *second*
    /// one: every certificate in this case states `1.2.840.10045.2.1` and they differ in their
    /// `namedCurve`, so reporting the key algorithm would tell a reader nothing. ISO/TS 32002
    /// Table 3's three Brainpool curves are what reach here today, and this program lacking them
    /// is a gap rather than a defect in the file; a curve outside Table 3 and Table 4 is the file
    /// leaving what the Technical Specification admits, and section 5.1.3's last sentence permits
    /// exactly this treatment: "PDF processors may ignore or handle in an implementation-dependent
    /// manner PDF documents which are signed with elliptic curves not listed in Table 3 or Table
    /// 4."
    CurveNotVerifiable {
        /// The curve's object identifier as dotted decimal, with its Table 3 name where it has
        /// one, or a sentence where the certificate states no `namedCurve` at all — which ISO/TS
        /// 32002 section 5.1.3 forbids: "The implicitCurve and specifiedCurve options shall not be
        /// used."
        curve: String,
    },
    /// The signature algorithm is none this program verifies, named the same way.
    AlgorithmNotVerifiable {
        /// The algorithm's object identifier as dotted decimal.
        algorithm: String,
    },
    /// The signature states `id-RSASSA-PSS` with parameters this program cannot verify under.
    ///
    /// RFC 8017 Appendix A.2.3 parameterises the scheme inside the `AlgorithmIdentifier` —
    /// `RSASSA-PSS-params`, with a hash, a mask generation function, a salt length and a trailer
    /// field — and [`crate::pss::parameters`] refuses what it cannot act on rather than
    /// defaulting on the file's behalf: a mask generation function other than MGF1, a hash the
    /// scheme does not admit, a trailer field other than 1, or an encoding that is not the
    /// structure at all. A hash this program simply does not *compute* is
    /// [`Self::UnknownDigest`] instead, because that is what that variant says.
    PssParametersNotVerifiable {
        /// What the parameters state, with any object identifier as dotted decimal.
        statement: String,
    },
    /// The signature's algorithm and the signer's key are from two different families.
    ///
    /// A file stating a DSA signature over an RSA key, or the other way round, has not written
    /// something this program should guess at: the two identifiers are both the producer's own
    /// claims about the same signature and they contradict each other. Both are carried, because
    /// which of the pair is wrong is not something a reader here can know.
    KeyDoesNotMatchAlgorithm {
        /// The `SignerInfo`'s `signatureAlgorithm`, as dotted decimal.
        algorithm: String,
        /// The certificate's `subjectPublicKeyInfo` algorithm, as dotted decimal.
        key: String,
    },
    /// The key or the signature is outside [`crate::pkcs1`]'s budgets, or is not shaped like RSA.
    ///
    /// Both RSA paddings report through this variant, because the budgets are the shared
    /// primitive's: [`crate::pss::verify`] refuses with the same [`Pkcs1Error`]s
    /// [`crate::pkcs1::verify`] does.
    Refused(Pkcs1Error),
    /// The same for [`crate::dsa`]'s.
    RefusedDsa(DsaError),
    /// The same for [`crate::ecdsa`]'s: an encoding or a range this module would not act on.
    RefusedEcdsa(EcdsaError),
    /// The same for [`crate::eddsa`]'s.
    RefusedEdDsa(EdDsaError),
    /// The signature states a digest algorithm this program does not compute.
    ///
    /// All six that ISO 32000-2's Table 260 and Table 256 name are implemented, and so are the four
    /// ISO/TS 32001 section 5.1.4 adds to Table 260 — SHA3-256, SHA3-384, SHA3-512 and SHAKE256
    /// (ADR 0390). This is
    /// therefore an identifier outside both documents, and it is carried so that *which* one a file
    /// used is a question a person can answer.
    ///
    /// **This sentence went on to say that three of the corpus's signatures reach here, each
    /// stating `1.2.840.113549.1.1.5` — a *signature* algorithm — where a digest algorithm
    /// belongs, and no signature does.** That was this reader's own defect rather than any file's:
    /// `digestAlgorithm` is a `SignerInfo`'s third member and reading by shape found the issuer's
    /// `SEQUENCE` instead, which `cms`'s `the_signers_own_sequence_is_not_mistaken_for_its_digest_algorithm`
    /// has pinned since the three-hundred-and-seventy-seventh session — so the observation was
    /// already false when it was written down two hundred sessions later. Re-derived in the
    /// six-hundred-and-forty-first with `examples/signature_algorithm_census`: every one of the
    /// corpus's ten signature values verifies, and none reaches this variant.
    UnknownDigest {
        /// The digest algorithm's object identifier as dotted decimal.
        algorithm: String,
    },
    /// The dictionary states no `/Contents`.
    NoSignatureValue,
    /// The `/ByteRange` does not name bytes of this file, so there was nothing to hash.
    RangeNotInThisFile,
    /// The signature value could not be read as §12.8.3.3's CMS object.
    Unreadable(CmsError),
}

/// Which algorithm family a signature was checked with — and, inside a family, which construction.
///
/// Table 260 names three families and ISO/TS 32002 section 5.1.2 adds a fourth row to that table,
/// so there are four here rather than three. RSA has two arms because the table's "RSA Algorithm
/// Support" row states key sizes and no padding, and the sentence a person reads should say which
/// construction did the verifying; ECDSA carries its curve for the same reason, since ISO/TS 32002
/// Table 3 is a list of curves rather than of key sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// "RSA Algorithm Support", as RFC 8017's RSASSA-PKCS1-v1_5.
    Rsa,
    /// The same family under RFC 8017's other padding, RSASSA-PSS ([`crate::pss`]).
    RsaPss,
    /// "DSA Algorithm Support", as FIPS 186-4 section 4.7.
    Dsa,
    /// "ECDSA Algorithm Support", on the curve ISO/TS 32002 Table 3 names ([`crate::ecdsa`]).
    Ecdsa(ecdsa::Curve),
    /// The "`EdDSA` algorithm support" row ISO/TS 32002 section 5.1.2 adds ([`crate::eddsa`]).
    ///
    /// No curve beside it, because Table 4's two are one implemented and one refused by number —
    /// a verification that happened was Ed25519's.
    EdDsa,
}

impl Family {
    /// The family's name, for a sentence a person reads.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Rsa => "RSA (PKCS #1 v1.5)",
            Self::RsaPss => "RSA (RSASSA-PSS)",
            Self::Dsa => "DSA",
            Self::Ecdsa(ecdsa::Curve::P256) => "ECDSA (P-256)",
            Self::Ecdsa(ecdsa::Curve::P384) => "ECDSA (P-384)",
            Self::Ecdsa(ecdsa::Curve::P521) => "ECDSA (P-521)",
            Self::EdDsa => "EdDSA (Ed25519)",
        }
    }
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
            .field(
                "chain",
                &format_args!("{} certificate(s)", self.chain.len()),
            )
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

    /// **Does this signature verify under the key in the certificate the file carries?**
    ///
    /// The second of §12.8.1's three questions, and the one this program gained in the
    /// three-hundred-and-ninety-second session. What it does, in order:
    ///
    /// 1. finds the signer's certificate — by RFC 5652's `issuerAndSerialNumber` or its
    ///    `subjectKeyIdentifier`, among the certificates the CMS object carries, or in Table 255's
    ///    `/Cert` for a §12.8.3.2 signature, which carries no CMS object at all;
    /// 2. reads its `subjectPublicKeyInfo` ([`crate::x509`]);
    /// 3. digests whatever RFC 5652 section 5.4 says the signature is over ([`Signed`]);
    /// 4. verifies with the construction the `signatureAlgorithm` states — RFC 8017 section
    ///    8.2.2's encode-and-compare ([`crate::pkcs1`]), its section 9.1.2's `EMSA-PSS-VERIFY`
    ///    ([`crate::pss`]), FIPS 186-4 section 4.7 ([`crate::dsa`]), ANSI X9.62's over the curve
    ///    RFC 5480's `namedCurve` states ([`crate::ecdsa`]), or RFC 8032's over the message itself
    ///    ([`crate::eddsa`]).
    ///
    /// **Step 4 listed the first three alone until the seven-hundred-and-fifth session**, four
    /// rounds after ADR 0532 added the last two — while the module comment twelve lines above it
    /// said "for all four" and named both modules. Nothing about a `/SubFilter` narrows any of
    /// this: the pair matched below is the `signatureAlgorithm` and the certificate's key, so a
    /// §12.8.3.4 `PAdES` signature reaches the same five arms as an `adbe.pkcs7.detached` one,
    /// which is what ISO/TS 32002 sections 5.1.2 and 5.1.3 require by naming
    /// `ETSI.CAdES.detached` in the applicability sentence of each of their curve tables.
    ///
    /// Read [`Authenticity`] before reading a result. [`Authenticity::Verified`] is not "valid":
    /// the certificate it verified against arrived in the same file as the signature.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per verifying construction, each a few lines of the same shape; \
                  splitting them apart would scatter the one match that keeps a signature \
                  algorithm and a key from being paired wrongly"
    )]
    pub fn authenticity(&self, file: &[u8]) -> Authenticity {
        if self.contents.is_empty() {
            return Authenticity::NoSignatureValue;
        }
        let Some(signed) = self.signed_bytes(file) else {
            return Authenticity::RangeNotInThisFile;
        };
        // §12.8.3.2: "For signing PDF files using PKCS #1, the only value of SubFilter that should
        // be used is adbe.x509.rsa_sha1 … The certificate chain of the signer shall be stored in
        // the Cert entry." So there is no CMS object to read: `/Contents` is the signature and
        // `/Cert` is the key.
        if self.sub_filter.as_deref() == Some("adbe.x509.rsa_sha1") {
            return self.pkcs1_authenticity(&signed);
        }
        let cms = match self.signed_data() {
            Ok(cms) => cms,
            Err(error) => return Authenticity::Unreadable(error),
        };
        let algorithm = cms.algorithm();
        if let SignatureAlgorithm::Unrecognised(oid) = algorithm {
            return Authenticity::AlgorithmNotVerifiable {
                algorithm: name(oid),
            };
        }
        let Some(certificate) = signer_certificate(&cms) else {
            return Authenticity::NoSignerCertificate {
                certificates: cms.certificates.len(),
            };
        };
        let certificate = match x509::read(certificate) {
            Ok(certificate) => certificate,
            Err(error) => return Authenticity::CertificateUnreadable(error),
        };
        if let x509::PublicKey::Unverifiable { algorithm } = certificate.public_key {
            return Authenticity::KeyNotVerifiable {
                algorithm: name(algorithm),
            };
        }
        if let x509::PublicKey::EcCurveNotVerifiable { curve } = certificate.public_key {
            return Authenticity::CurveNotVerifiable {
                curve: curve_name(curve),
            };
        }
        // RFC 5652 section 5.4 decides what is hashed, and [`Signed`] documents what each one
        // proves. The digest *algorithm* is the one thing the constructions disagree about —
        // PKCS #1 v1.5 and DSA take the `SignerInfo`'s own `digestAlgorithm`, while RSASSA-PSS
        // is parameterised by the hash its `RSASSA-PSS-params` state — so what is signed is
        // settled here and each arm below digests it with the algorithm its scheme names.
        let attributes = cms.signed_attributes_encoding();
        let over = match (&attributes, cms.encapsulated) {
            (Some(_), _) => Signed::SignedAttributes,
            (None, Some(_)) => Signed::EncapsulatedContent,
            (None, None) => Signed::TheDocumentsBytes,
        };
        // The bytes themselves rather than only their digest, because one of the four families
        // does not take a digest at all: RFC 8032's Ed25519 hashes the message internally, so
        // [`crate::eddsa::verify`] needs the parts. They stay parts rather than being joined —
        // a signature over a whole document would otherwise cost a copy of it.
        let parts: Vec<&[u8]> = match (&attributes, cms.encapsulated) {
            (Some(attributes), _) => vec![attributes.as_slice()],
            (None, Some(content)) => vec![content],
            (None, None) => signed.clone(),
        };
        let compute = |algorithm: Digest| algorithm.compute(&parts);
        // The pair rather than either alone: a `SignerInfo` naming DSA over a certificate holding
        // an RSA key is two claims by one producer that contradict each other, and picking the one
        // to believe would be this program inventing a fact.
        let (digest, family, verified) = match (algorithm, certificate.public_key) {
            (SignatureAlgorithm::RsaPkcs1V15, x509::PublicKey::Rsa(key)) => {
                let Some(digest) = cms.digest else {
                    return Authenticity::UnknownDigest {
                        algorithm: name(cms.digest_algorithm),
                    };
                };
                (
                    digest,
                    Family::Rsa,
                    pkcs1::verify(key, cms.signature, digest, &compute(digest))
                        .map_err(Authenticity::Refused)
                        .map(|verified| (verified, key.bits())),
                )
            }
            (SignatureAlgorithm::RsaPss, x509::PublicKey::Rsa(key)) => {
                let parameters = match pss::parameters(cms.signature_algorithm_parameters) {
                    Ok(parameters) => parameters,
                    Err(problem) => return pss_parameter_answer(problem),
                };
                // RFC 8017 section 9.1.2 step 2's `mHash` is computed with the parameters' own
                // hash — RFC 5652's `digestAlgorithm` describes the `message-digest` attribute,
                // which is question 1's comparison, not this one's.
                (
                    parameters.hash,
                    Family::RsaPss,
                    pss::verify(key, cms.signature, parameters, &compute(parameters.hash))
                        .map_err(Authenticity::Refused)
                        .map(|verified| (verified, key.bits())),
                )
            }
            (SignatureAlgorithm::Dsa, x509::PublicKey::Dsa(key)) => {
                let Some(digest) = cms.digest else {
                    return Authenticity::UnknownDigest {
                        algorithm: name(cms.digest_algorithm),
                    };
                };
                (
                    digest,
                    Family::Dsa,
                    dsa::verify(key, cms.signature, &compute(digest))
                        .map_err(Authenticity::RefusedDsa)
                        .map(|verified| (verified, key.bits())),
                )
            }
            (SignatureAlgorithm::Ecdsa, x509::PublicKey::Ec(key)) => {
                let Some(digest) = cms.digest else {
                    return Authenticity::UnknownDigest {
                        algorithm: name(cms.digest_algorithm),
                    };
                };
                (
                    digest,
                    Family::Ecdsa(key.curve),
                    ecdsa::verify(key, cms.signature, &compute(digest))
                        .map_err(Authenticity::RefusedEcdsa)
                        .map(|verified| (verified, key.curve.bits())),
                )
            }
            (SignatureAlgorithm::EdDsa, x509::PublicKey::Ed25519(key)) => {
                // The digest is reported rather than used: ISO/TS 32002 Table 4 pairs Ed25519 with
                // SHA512, which is what question 1's `message-digest` attribute was computed with,
                // and RFC 8032's signature is over the message itself.
                let Some(digest) = cms.digest else {
                    return Authenticity::UnknownDigest {
                        algorithm: name(cms.digest_algorithm),
                    };
                };
                (
                    digest,
                    Family::EdDsa,
                    eddsa::verify(key, cms.signature, &parts)
                        .map_err(Authenticity::RefusedEdDsa)
                        // RFC 8032 section 5.1: `b` is 256 for Ed25519, so the key is 32 octets.
                        .map(|verified| (verified, 256)),
                )
            }
            _ => {
                return Authenticity::KeyDoesNotMatchAlgorithm {
                    algorithm: name(cms.signature_algorithm),
                    key: key_algorithm_name(&certificate),
                };
            }
        };
        match verified {
            Ok((true, key_bits)) => Authenticity::Verified {
                digest,
                family,
                key_bits,
                over,
            },
            Ok((false, key_bits)) => Authenticity::NotUnderThatKey {
                digest,
                family,
                key_bits,
                over,
            },
            Err(answer) => answer,
        }
    }

    /// §12.8.3.2's signature: a PKCS #1 value over the byte range, with `/Cert`'s first entry.
    ///
    /// **The digest algorithm is not stated anywhere a reader can see it.** Table 260 permits all
    /// five of its digests for this `/SubFilter` while §12.8.3.2 names only SHA-1, and the
    /// identifier that settles it is inside the block, under the key. So each of
    /// [`Digest::TRIED_WHEN_UNSTATED`] is tried, which RFC 8017 section 8.2.2's whole-block
    /// comparison makes safe: six comparisons against fixed-length strings admit no forgery one
    /// does not.
    ///
    /// **Six and not ten**, because ISO/TS 32001 section 5.1.4 adds its four to Table 260's Message
    /// Digest entry "for adbe.pkcs7.detached, ETSI.CAdES.detached or ETSI.RFC3161" and this
    /// `/SubFilter` is none of the three. That constant's own documentation carries the reasoning.
    fn pkcs1_authenticity(&self, signed: &[&[u8]]) -> Authenticity {
        let Some(bytes) = self.chain.first() else {
            return Authenticity::NoSignerCertificate {
                certificates: self.chain.len(),
            };
        };
        let certificate = match x509::parse(bytes) {
            Ok(certificate) => certificate,
            Err(error) => return Authenticity::CertificateUnreadable(error),
        };
        // **Table 260 says "No" to DSA for this `/SubFilter`**, in the `adbe.x509.rsa_sha1` column
        // of its "DSA Algorithm Support" row, so a `/Cert` carrying a DSA key is a file departing
        // from the table rather than a case this program owes an implementation. It is named by
        // its identifier like any other key this signature format may not carry.
        let x509::PublicKey::Rsa(key) = certificate.public_key else {
            return Authenticity::KeyNotVerifiable {
                algorithm: key_algorithm_name(&certificate),
            };
        };
        // §12.8.3.3.1 has a producer pad `/Contents` with zeros to fill the space allocated for
        // it, and a PKCS #1 signature is exactly as long as the modulus, so the padding is dropped
        // by taking that many octets rather than by trimming zeros — which would also eat a
        // signature's own trailing zero.
        let length = key.modulus.len().saturating_sub(
            key.modulus
                .iter()
                .take_while(|&&byte| byte == 0)
                .count()
                .min(key.modulus.len()),
        );
        let value = self.contents.get(..length).unwrap_or(&self.contents);
        let key_bits = key.bits();
        let mut refusal = None;
        for digest in Digest::TRIED_WHEN_UNSTATED {
            match pkcs1::verify(key, value, digest, &digest.compute(signed)) {
                Ok(true) => {
                    return Authenticity::Verified {
                        digest,
                        family: Family::Rsa,
                        key_bits,
                        over: Signed::TheDocumentsBytes,
                    };
                }
                Ok(false) => {}
                Err(error) => refusal = Some(error),
            }
        }
        refusal.map_or(
            // §12.8.3.2 names SHA-1 for this `/SubFilter`, so that is the algorithm to report
            // having failed with when none of the six matched.
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha1,
                family: Family::Rsa,
                key_bits,
                over: Signed::TheDocumentsBytes,
            },
            Authenticity::Refused,
        )
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

/// The certificate a `SignerInfo` names, among the ones the CMS object carries.
///
/// RFC 5652's `SignerIdentifier` is a choice of two, and both are honoured: the issuer-and-serial
/// pair that every corpus signature uses, and the `subjectKeyIdentifier` that a version 3
/// `SignerInfo` may use instead. A signature naming neither, or naming one no certificate answers
/// to, yields `None` — deliberately rather than falling back to "the only certificate present",
/// which would verify against a key the signature never claimed.
fn signer_certificate<'a>(cms: &SignedData<'a>) -> Option<crate::der::Value<'a>> {
    if let Some((issuer, serial)) = cms.signer_issuer_and_serial {
        return cms.certificates.iter().copied().find(|entry| {
            x509::read(*entry).is_ok_and(|certificate| certificate.is_named_by(issuer, serial))
        });
    }
    let wanted = cms.signer_key_identifier?;
    cms.certificates.iter().copied().find(|entry| {
        x509::read(*entry).is_ok_and(|certificate| certificate.key_identifier == Some(wanted))
    })
}

/// An object identifier as a person reads it, falling back to its octets.
///
/// [`x509::dotted`] refuses an encoding that is not a well-formed identifier, and a report that
/// dropped the algorithm entirely would say less than the file does — so the hexadecimal is what
/// is shown then, marked as such.
fn name(oid: &[u8]) -> String {
    use std::fmt::Write as _;
    x509::dotted(oid).unwrap_or_else(|| {
        oid.iter().fold(String::from("0x"), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
    })
}

/// What a person is told about `RSASSA-PSS-params` this program could not verify under.
///
/// Two channels rather than one, on the difference the variants' own documentation states: a
/// hash this program does not *compute* is [`Authenticity::UnknownDigest`], exactly as it would
/// be anywhere else a digest identifier arrives, while a parameter the scheme itself does not
/// admit — MD5 or RIPEMD-160 as the hash, a mask generation function other than MGF1, a trailer
/// field other than 1, or no readable `RSASSA-PSS-params` at all — is
/// [`Authenticity::PssParametersNotVerifiable`] with the file's own numbers in the sentence.
fn pss_parameter_answer(problem: pss::ParameterProblem<'_>) -> Authenticity {
    use pss::ParameterProblem;
    match problem {
        ParameterProblem::HashNotComputed(oid) => Authenticity::UnknownDigest {
            algorithm: name(oid),
        },
        ParameterProblem::HashNotAdmitted(oid) => Authenticity::PssParametersNotVerifiable {
            statement: format!(
                "hash algorithm {}, which RFC 8017's OAEP-PSSDigestAlgorithms set does not admit \
                 for PSS",
                name(oid)
            ),
        },
        ParameterProblem::MaskGenerationNotMgf1(oid) => Authenticity::PssParametersNotVerifiable {
            statement: format!(
                "mask generation function {}, where RFC 8017 defines only MGF1",
                name(oid)
            ),
        },
        ParameterProblem::TrailerFieldNotOne => Authenticity::PssParametersNotVerifiable {
            statement: "a trailer field other than the 1 RFC 8017 requires".to_owned(),
        },
        ParameterProblem::Malformed => Authenticity::PssParametersNotVerifiable {
            statement: "no readable RSASSA-PSS-params, which RFC 8017 Appendix A.2.3 requires \
                        of this algorithm identifier"
                .to_owned(),
        },
    }
}

/// The identifier of the algorithm a certificate's key is for, as dotted decimal.
///
/// The two families this program reads are named by the identifier the standard that defines them
/// assigns rather than by the octets the certificate happened to write, because [`crate::x509`]
/// keeps the key and not the identifier once it has recognised one. They are the same number.
/// What a person is told about a `namedCurve` this program does not compute on.
///
/// The number always, because it is what a reader can check; and Table 3's own spelling beside it
/// where the curve is one of the three this program lacks rather than one the standard never
/// admitted, because "brainpoolP256r1, refused" and "1.3.36.3.3.2.8.1.1.7, refused" are the same
/// fact and only one of them can be looked up in ISO/TS 32002.
fn curve_name(curve: Option<&[u8]>) -> String {
    let Some(curve) = curve else {
        // ISO/TS 32002 section 5.1.3: "The implicitCurve and specifiedCurve options shall not be
        // used." There is no identifier to print because the file stated none.
        return "no namedCurve (ISO/TS 32002 5.1.3 requires one)".to_owned();
    };
    let number = name(curve);
    ecdsa::UnsupportedCurve::of(curve).map_or(number.clone(), |known| {
        format!("{number} ({})", known.name())
    })
}

fn key_algorithm_name(certificate: &x509::Certificate<'_>) -> String {
    match certificate.public_key {
        x509::PublicKey::Rsa(_) => name(x509::RSA_ENCRYPTION),
        x509::PublicKey::Dsa(_) => name(dsa::ID_DSA),
        // Both elliptic-curve arms state the same key algorithm: RFC 5480's `id-ecPublicKey` is
        // what the certificate says, and which curve it is on is a second identifier that
        // `Authenticity::CurveNotVerifiable` is where a reader hears about.
        x509::PublicKey::Ec(_) | x509::PublicKey::EcCurveNotVerifiable { .. } => {
            name(const_oid::db::rfc5912::ID_EC_PUBLIC_KEY.as_bytes())
        }
        x509::PublicKey::Ed25519(_) => name(eddsa::ID_ED25519.as_bytes()),
        x509::PublicKey::Unverifiable { algorithm } => name(algorithm),
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
        chain: chain(document, dict),
        name: text("Name"),
        signed_at: text("M"),
        location: text("Location"),
        reason: text("Reason"),
        contact: text("ContactInfo"),
        changes: changes(document, dict),
        certification: has_transform(document, dict, b"DocMDP"),
    })
}

/// Which form fields an `/Action` and `/Fields` pair names — Table 236's, and Table 259's.
///
/// **One type for two tables, because the standard makes them copies of each other.** §12.8.2.4
/// says so outright about the writer's obligation:
///
/// > The Action and Fields entries in the transform parameters dictionary shall be copied from
/// > the corresponding fields in the signature field lock dictionary.
///
/// So Table 236's signature field lock (§12.7.5.5) and Table 259's `FieldMDP` transform parameters
/// (§12.8.2.4) state the same three actions over the same array of names. What differs is what
/// each *means* — a lock is a prohibition on a reader and a transform is a statement about what
/// invalidates a signature — and that difference lives in [`crate::restriction::Restriction`],
/// where a person is told which of the two applies. A second enum with the same three variants
/// would have claimed a distinction the vocabulary does not have.
///
/// **Neither table's action values are in `doc/md/`** — the conversion drops the list inside the
/// `/Action` row's cell, exactly where `doc/HANDOVER.md` says to expect a loss: Table 236 is left
/// with "[t]he value shall be one of the following:" and nothing following it, and Table 259 with
/// "Valid values shall be: All All form fields." and the other two gone. Both readings below are
/// `pdftotext -layout` over `doc/ISO_32000-2_sponsored_EC3.pdf`, which is the check that file
/// names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldSelection {
    /// `/Action /All`: "All fields in the document" (Table 236), "All form fields" (Table 259).
    All,
    /// `/Action /Include`: "All fields specified in Fields", "Only those form fields specified in
    /// Fields".
    Include(Vec<String>),
    /// `/Action /Exclude`: "All fields except those specified in Fields", "Only those form fields
    /// not specified in Fields".
    Exclude(Vec<String>),
}

impl FieldSelection {
    /// Whether this selection names the field with this fully qualified name (§12.7.4.2).
    ///
    /// Both tables printed only "[a]n array of text strings containing field names", and
    /// §12.7.4.2's fully qualified name is the only name in the clause that identifies a field
    /// uniquely — a partial name repeats across the tree, and covering every `Total` in a document
    /// because one was named would refuse edits the file never asked to refuse.
    ///
    /// **For Table 259 that is no longer an argument but the entry's own words, and this tree had
    /// never read the erratum that says so.** Errata Collection 3's Issue #33 inserts "fully
    /// qualified" and a reference to §12.7.4.2 into the `/Fields` row of the `FieldMDP` transform
    /// parameters dictionary, so what §12.8.2.4 requires is an array of text strings containing
    /// fully qualified field names. Table 236's identically worded row in §12.7.5.5 gains no such
    /// insertion, so this reading is *required* for the transform and remains the argument above
    /// for the lock — one function, two footings, and the weaker of them is the one to keep
    /// reasoning from. `spec-errata check` cannot see either caret, both being an insertion with
    /// nothing struck out; `emit` files them under §12.8.3.1, which is the heading at the foot of
    /// Table 259's page.
    ///
    /// **The corpus agrees with the argument**, which it did not have to: the transform in
    /// `xfa_filled_imm1344e.pdf` states `form1[0].SignatureField3[0]`, and `crate::form::fields`
    /// derives exactly that string as the fully qualified name of the document's one field.
    #[must_use]
    pub fn covers(&self, field: &str) -> bool {
        match self {
            Self::All => true,
            Self::Include(names) => names.iter().any(|name| name == field),
            Self::Exclude(names) => !names.iter().any(|name| name == field),
        }
    }
}

/// Every §12.7.5.5 lock a *signed* signature field in this document asserts.
///
/// The clause states the prohibition in prose rather than in Table 236, and the difference
/// matters: the table's own column says the fields "should be locked", while the sentence under
/// it is a `shall` about the lock dictionary, which —
///
/// > contains the names of form fields whose values shall no longer be changed after this
/// > signature has been signed.
///
/// **"[A]fter this signature has been signed" is the condition, and it is the whole of why this
/// is read off a `/V` rather than off a `/Lock`.** An unsigned signature field carrying a `/Lock`
/// is an instruction to whatever will do the signing (§12.7.5.5's NOTE 1 — "information needed
/// later when the actual signing takes place"), and locks nothing in the meantime.
///
/// Table 236's `/P` is deliberately *not* read. It reads like Table 257's `/P` and it is
/// addressed elsewhere: "absence of this key shall result in no effect on signature **validation
/// rules**", so it says what invalidates the signature rather than what a reader may do. The
/// entry that makes §12.8.2.2's equivalent binding on a processor is §12.8.6's permissions
/// dictionary, and Table 236 names no such route.
///
/// Empty for a document with no form, no signature, or none whose signature field states a
/// `/Lock` — which is every one of the documents in the pdf.js corpus and the four under
/// `doc/corpora/`, counted by `signatures.rs::the_corpus_states_the_fields_one_signature_covers`.
/// §12.8.2.4's transform is the copy of this that a real producer *does* write.
#[must_use]
pub fn field_locks(document: &Document) -> Vec<FieldSelection> {
    let mut out = Vec::new();
    for_each_signed_field(document, |field, _signature| {
        if let Some(lock) = document.get_key(field, "Lock").as_dict()
            && let Some(lock) = read_selection(document, lock)
        {
            out.push(lock);
        }
    });
    out
}

/// Every §12.8.2.4 `FieldMDP` transform a *signed* signature field's signature states.
///
/// > The FieldMDP transform method shall be used to detect changes to the values of a list of
/// > form fields.
///
/// **The same fields as [`field_locks`], stated in the other of the two places the standard puts
/// them**, and a document can state either without the other. §12.8.2.4 makes the transform a
/// copy of the field lock when a writer creates the signature, but the lock lives in the *field*
/// dictionary while the transform lives inside the signature — which the clause's NOTE says is
/// why the copy exists at all:
///
/// > This copying is done because all objects in a signature dictionary are direct objects if the
/// > dictionary contains a byte range signature. Therefore, the transform parameters dictionary
/// > cannot reference the signature field lock dictionary indirectly.
///
/// So a reader that consulted only §12.7.5.5's `/Lock` would miss what a signature says about
/// itself — and the transform is the copy inside the signed byte range, where a later incremental
/// update cannot quietly drop it.
///
/// **What is not done here is §12.8.2.2.2's comparison**, which "`FieldMDP` signatures shall be
/// validated in a similar manner to" and which needs the signed revision reconstructed from the
/// `/ByteRange`; [`Signature::integrity`] establishes only whether the signed bytes moved. Table
/// 256's `/Data`, "[a]n indirect reference to the object in the document upon which the object
/// modification analysis should be performed", is what that comparison would start from and is
/// therefore unread rather than unnoticed: nothing here performs the analysis it scopes.
///
/// The condition is the same one [`field_locks`] applies — a signature that has been signed —
/// and for the same reason: a `/Reference` on a signature nobody made covers nothing.
///
/// **The whole `/Reference` array is read rather than the first entry of it**, which is what the
/// corpus's one certification signature turns out to need: `xfa_filled_imm1344e.pdf` states two
/// signature reference dictionaries on one signature, a `DocMDP` and a `FieldMDP`, and this tree
/// read only the first for its whole life. §12.8.2.1's plural is the clause behind that —
/// "[t]ransform methods, along with transform parameters, shall determine which objects are
/// included and excluded in revision comparison".
#[must_use]
pub fn field_mdp(document: &Document) -> Vec<FieldSelection> {
    let mut out = Vec::new();
    for_each_signed_field(document, |_field, signature| {
        let references = document.get_key(signature, "Reference");
        let Some(references) = references.as_array().map(<[Object]>::to_vec) else {
            return;
        };
        for reference in &references {
            let resolved = document.resolve(reference);
            let Some(reference) = resolved.as_dict() else {
                continue;
            };
            let is_field_mdp = document
                .get_key(reference, "TransformMethod")
                .as_name()
                .is_some_and(|method| method.as_bytes() == b"FieldMDP");
            if !is_field_mdp {
                continue;
            }
            if let Some(parameters) = document.get_key(reference, "TransformParams").as_dict()
                && let Some(covered) = read_selection(document, parameters)
            {
                out.push(covered);
            }
        }
    });
    out
}

/// Every signature field in this document that **has been signed**, as its field dictionary and
/// the signature dictionary that signed it.
///
/// The test for "is a signature field" is that its `/V` is a signature dictionary this crate can
/// read, rather than Table 226's `/FT /Sig`: §12.7.4.1 makes `/FT` inheritable, so a kid that
/// states none is not thereby a different kind of field — and the two clauses that call this both
/// condition on the signature existing rather than on the field's type.
///
/// **The walk is not gated on Table 225's `/SigFlags`, which [`signatures`] is**, and the
/// asymmetry is deliberate: that flag exists so a processor can *skip* work, and skipping it
/// here would mean a document that under-describes itself escapes a restriction it wrote down.
/// A missed signature costs a report; a missed lock costs a `shall`.
fn for_each_signed_field(document: &Document, mut visit: impl FnMut(&Dictionary, &Dictionary)) {
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return;
    };
    let fields = document.get_key(form, "Fields");
    let Some(fields) = fields.as_array().map(<[Object]>::to_vec) else {
        return;
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut visited = 0usize;
    for field in &fields {
        walk_signed_fields(document, field, &mut visit, &mut seen, &mut visited, 0);
    }
}

/// One level of [`for_each_signed_field`]'s walk.
fn walk_signed_fields(
    document: &Document,
    field: &Object,
    visit: &mut impl FnMut(&Dictionary, &Dictionary),
    seen: &mut std::collections::BTreeSet<pdf_syntax::ObjectId>,
    visited: &mut usize,
    depth: usize,
) {
    if *visited >= MAX_SIGNATURES || depth > 32 {
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
    if let Some(value) = document.get_key(dict, "V").as_dict()
        && read(document, value).is_some()
    {
        *visited = visited.saturating_add(1);
        visit(dict, value);
    }
    if let Some(kids) = document
        .get_key(dict, "Kids")
        .as_array()
        .map(<[Object]>::to_vec)
    {
        for kid in &kids {
            walk_signed_fields(document, kid, visit, seen, visited, depth.saturating_add(1));
        }
    }
}

/// An `/Action` and `/Fields` pair, or `None` where the dictionary states neither usefully.
///
/// `/Action` is "(Required)" in both Table 236 and Table 259, and its three values are the whole
/// of either table's vocabulary, so a name that is none of them states nothing the clause defines
/// and is not guessed at: an unrecognised action that fell back to `All` would close a document's
/// every field on a word the standard does not use. `Include` and `Exclude` need `/Fields` — "(Required if the
/// value of Action is Include or Exclude)" — and an `Include` with none names no field, which is
/// a selection over nothing.
fn read_selection(document: &Document, dict: &Dictionary) -> Option<FieldSelection> {
    let action = document.get_key(dict, "Action");
    let action = action.as_name()?;
    let names = || {
        document
            .get_key(dict, "Fields")
            .as_array()
            .map(|fields| {
                fields
                    .iter()
                    .map(|field| document.resolve(field))
                    .filter_map(|field| match field {
                        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    match action.as_bytes() {
        b"All" => Some(FieldSelection::All),
        b"Include" => Some(FieldSelection::Include(names())),
        b"Exclude" => Some(FieldSelection::Exclude(names())),
        _ => None,
    }
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
        // "UR" is the transform method's name — Table 256's `/TransformMethod`, "( Deprecated in
        // PDF 2.0 )" — and `/UR3` is Table 263's key in the permissions dictionary for the
        // signature that carries it. The two are not the same string, and a producer that writes
        // the permissions key where the method belongs is read rather than refused.
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

/// Table 255's `/Cert`, "a byte string if the chain has only one entry" or an array of them.
///
/// Bounded at [`MAX_CHAIN`] rather than by the array's own length, which is the rule everywhere
/// else in this module: an allocation sized by a number in the file is the shape `CLAUDE.md`
/// principle 3 forbids.
fn chain(document: &Document, dict: &Dictionary) -> Vec<Vec<u8>> {
    match document.get_key(dict, "Cert") {
        Object::String(bytes) => vec![bytes.to_vec()],
        Object::Array(entries) => entries
            .iter()
            .take(MAX_CHAIN)
            .filter_map(|entry| match document.resolve(entry) {
                Object::String(bytes) => Some(bytes.to_vec()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
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
/// So this counts them and stops. **This paragraph used to say "[p]arsing a certificate is X.509
/// and a trust decision", and the three-hundred-and-ninety-second session made the first half of
/// that false**: [`crate::x509`] parses one. What has not changed is the second half and it is the
/// one that decides this row — a certificate here would be read to *validate a certification path*,
/// which is question 3, and reading the bytes is the smallest part of that. Counting them says the
/// one thing a person might want from a program that cannot validate — **whether the document
/// carries what a validator would need**, which is the whole point of §12.8.4's "long term
/// validation".
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
        Authenticity, Coverage, Family, Integrity, Modification, PadesDeparture, Signature, Signed,
        legal, permissions, security_store, signatures,
    };
    use crate::cms::{Digest, fixtures};
    use crate::x509::fixtures::{CERTIFICATE, EC_CERTIFICATE, PKCS1_SIGNATURE, hex};
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

    /// **Question 1 under each of ISO/TS 32001's four digests**, which no real document states.
    ///
    /// Section 5.1.4 adds SHA3-256, SHA3-384, SHA3-512 and SHAKE256 to Table 260's Message Digest
    /// entry for `adbe.pkcs7.detached` among others, so a signature may now record its byte-range
    /// digest with any of them — and `signature_algorithm_census` finds not one that does in 67 460
    /// documents. That is trap 8 exactly: the corpus cannot rank a requirement it does not
    /// exercise, so the witness is built here, one file per algorithm, and it is checked in both
    /// directions rather than only the agreeing one. ADR 0390.
    ///
    /// **Before this round each of these four reported [`Integrity::UnknownDigest`]**, which is
    /// what makes the test worth its lines: the assertion is that a report became an answer.
    #[test]
    fn a_signature_stating_one_of_iso_ts_32001s_digests_is_recomputed() {
        for digest in [
            Digest::Sha3_256,
            Digest::Sha3_384,
            Digest::Sha3_512,
            Digest::Shake256,
        ] {
            let bytes = signed_document("adbe.pkcs7.detached", "", digest, |recorded| {
                fixtures::detached_stating(digest, recorded)
            });
            let (document, signature) = only_signature(&bytes);
            assert_eq!(
                signature.integrity(document.bytes()),
                Integrity::Unchanged { digest },
                "{} recomputes to what the signature recorded",
                digest.name()
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
                Integrity::Changed { digest },
                "{} notices one byte of the signed range moving",
                digest.name()
            );
        }
    }

    /// An identifier neither document names is still reported by its number, not skipped.
    ///
    /// The four arriving does not change what happens to a fifth, and this is the assertion that
    /// says so. `2.16.840.1.101.3.4.2.11` is the SHAKE**128** slot in the same arc — the nearest
    /// neighbour of one this program now computes, which is the identifier a widening mistake would
    /// most likely swallow.
    #[test]
    fn a_digest_outside_both_documents_still_reports_its_number() {
        let shake128 = [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x0B];
        assert_eq!(Digest::from_oid(&shake128), None);
        assert_eq!(
            crate::x509::dotted(&shake128).as_deref(),
            Some("2.16.840.1.101.3.4.2.11"),
            "and the number a person would be shown is that one"
        );
    }

    /// The signature of the document, and the file it came from.
    fn only_signature(bytes: &[u8]) -> (Document, Signature) {
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

    /// The whole of question 2, on the one signature format no corpus document uses.
    ///
    /// §12.8.3.2's `adbe.x509.rsa_sha1` has no CMS object at all: `/Contents` is the PKCS #1
    /// signature over the byte range and `/Cert` is where the certificate lives. So this is the
    /// one case a [`Signature`] can be assembled for directly, over bytes chosen here — which is
    /// also the one place a *positive* verification is testable without a private key in the
    /// tree. The key, the certificate and the signature are `pkcs1`'s and `x509`'s test vector.
    #[test]
    fn a_pkcs1_signature_verifies_against_the_certificate_in_its_cert_entry() {
        let file = b"the signed bytes";
        let signature = pkcs1_signature(file.len() as u64, hex(CERTIFICATE));
        assert_eq!(
            signature.authenticity(file),
            Authenticity::Verified {
                digest: Digest::Sha256,
                family: Family::Rsa,
                key_bits: 2048,
                over: Signed::TheDocumentsBytes,
            },
            "and the digest is found by trying the six, because nothing states it"
        );
        // §12.8.3.2's signature is over the document's own bytes, so question 2 settles question
        // 1 here: change one byte of the file and the same signature stops verifying.
        assert_eq!(
            signature.authenticity(b"the signed byteS"),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha1,
                family: Family::Rsa,
                key_bits: 2048,
                over: Signed::TheDocumentsBytes,
            }
        );
        assert!(
            Signed::TheDocumentsBytes.binds_the_document(),
            "which is what this variant is for"
        );
    }

    /// A key this `/SubFilter` may not carry is named by its number, not skipped and not verified.
    ///
    /// **Table 260 says "No" to ECDSA in its `adbe.x509.rsa_sha1` column**, exactly as it does to
    /// DSA there, so a `/Cert` holding a P-256 key is a file departing from the table rather than
    /// a case this program owes a verification — even now that it verifies that family through
    /// CMS. What is exercised is that `pkcs1_authenticity` reports such a key by its identifier
    /// instead of reaching for the curve, on a real P-256 certificate rather than a hand-made
    /// shape.
    #[test]
    fn a_key_this_program_cannot_verify_is_named_by_its_object_identifier() {
        let file = b"the signed bytes";
        let signature = pkcs1_signature(file.len() as u64, hex(EC_CERTIFICATE));
        assert_eq!(
            signature.authenticity(file),
            Authenticity::KeyNotVerifiable {
                algorithm: "1.2.840.10045.2.1".to_owned(),
            },
            "id-ecPublicKey, printed rather than named"
        );
    }

    /// A `/Cert` with nothing in it is §12.8.3.2's requirement unmet, and says so.
    #[test]
    fn a_pkcs1_signature_with_no_certificate_has_no_key_to_verify_against() {
        let file = b"the signed bytes";
        let mut signature = pkcs1_signature(file.len() as u64, Vec::new());
        signature.chain = Vec::new();
        assert_eq!(
            signature.authenticity(file),
            Authenticity::NoSignerCertificate { certificates: 0 }
        );
    }

    /// A signature algorithm outside RFC 8017's PKCS #1 v1.5 identifiers is named, not attempted.
    ///
    /// The fixture's `signatureAlgorithm` is SHA-256's own identifier, which is not a public-key
    /// algorithm at all — so this is the shape a producer writing `id-RSASSA-PSS`, DSA or ECDSA
    /// would reach, and what a person is told is the number the file states.
    #[test]
    fn a_signature_algorithm_this_program_does_not_verify_is_named() {
        let bytes = signed_document("adbe.pkcs7.detached", "", Digest::Sha256, |digest| {
            fixtures::detached(digest)
        });
        let (document, signature) = only_signature(&bytes);
        assert_eq!(
            signature.authenticity(document.bytes()),
            Authenticity::AlgorithmNotVerifiable {
                algorithm: "2.16.840.1.101.3.4.2.1".to_owned(),
            }
        );
    }

    /// A §12.8.3.2 signature over the whole of `file`, with `certificate` in its `/Cert`.
    fn pkcs1_signature(length: u64, certificate: Vec<u8>) -> Signature {
        Signature {
            timestamp: false,
            handler: Some("Adobe.PPKLite".to_owned()),
            sub_filter: Some("adbe.x509.rsa_sha1".to_owned()),
            byte_range: vec![(0, length)],
            contents: hex(PKCS1_SIGNATURE),
            certificate_chain: true,
            chain: vec![certificate],
            name: None,
            signed_at: None,
            location: None,
            reason: None,
            contact: None,
            changes: None,
            certification: false,
        }
    }

    /// **Table 260's second algorithm family, all the way through.**
    ///
    /// The `dsa` module's own tests exercise FIPS 186-4 section 4.7 on a key and a signature; this
    /// exercises everything between a signature dictionary and that call — the `SignerInfo`'s
    /// `signatureAlgorithm` being recognised as DSA, the signer's certificate being found among
    /// the ones the value carries by RFC 5652's issuer and serial number, `x509` reading a
    /// `Dss-Parms` key out of it, and the answer naming the family rather than assuming RSA.
    ///
    /// **No corpus document could stand in.** 67 460 were read for this round — `doc/pdf.js`'s
    /// 974, `doc/corpora`'s 275 and the `SafeDocs` crawl's 66 211 — and their 811 signature
    /// dictionaries name RSA and, once, ECDSA. Not one names DSA, which `CLAUDE.md`'s trap 8 says
    /// is a fact about documents rather than about the standard.
    #[test]
    fn a_dsa_signature_verifies_through_the_whole_path_a_document_takes() {
        let file = b"the signed bytes";
        let certificate = hex(crate::dsa::fixtures::CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let signature = Signature {
            timestamp: false,
            handler: Some("Adobe.PPKLite".to_owned()),
            sub_filter: Some("adbe.pkcs7.detached".to_owned()),
            byte_range: vec![(0, file.len() as u64)],
            contents: fixtures::detached_dsa(
                &certificate,
                parsed.issuer,
                parsed.serial_number,
                &hex(crate::dsa::fixtures::SIGNATURE),
            ),
            certificate_chain: false,
            chain: Vec::new(),
            name: None,
            signed_at: None,
            location: None,
            reason: None,
            contact: None,
            changes: None,
            certification: false,
        };
        assert_eq!(
            signature.authenticity(file),
            Authenticity::Verified {
                digest: Digest::Sha256,
                family: Family::Dsa,
                key_bits: 2048,
                over: Signed::TheDocumentsBytes,
            },
            "the signer states no signed attributes, so RFC 5652 signs the byte range itself"
        );
        // And question 2 settles question 1 in this shape, which is what `Signed` records.
        assert_eq!(
            signature.authenticity(b"the signed byteS"),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha256,
                family: Family::Dsa,
                key_bits: 2048,
                over: Signed::TheDocumentsBytes,
            }
        );
    }

    /// **Table 260's third algorithm family, all the way through, on each of its three curves.**
    ///
    /// The `ecdsa` module's own tests exercise the arithmetic on a key and a signature; this
    /// exercises everything between a signature dictionary and that call — the `SignerInfo`'s
    /// `signatureAlgorithm` recognised as one of RFC 5758 section 3.2's `ecdsa-with-SHA*`, the
    /// signer's certificate found among the ones the value carries, `x509` reading RFC 5480's
    /// `namedCurve` and SEC1 point out of it, and the answer naming the curve rather than a key
    /// width alone.
    ///
    /// **The corpus has one witness and it cannot stand in for this.** One signature of 811 in
    /// 67 460 documents is `ecdsa-with-SHA256` over a DER `ECDSA-Sig-Value`, and it verifies —
    /// that is the demand-side evidence, taken by `examples/signature_algorithm_census`. What it
    /// cannot give is a *positive* verification over bytes this test chose, because nobody here
    /// holds that signer's private key, and it exercises one curve of three.
    #[test]
    fn an_ecdsa_signature_verifies_through_the_whole_path_a_document_takes() {
        use crate::ecdsa::fixtures as ec;
        let file = b"the signed bytes";
        // RFC 5758 section 3.2's `ecdsa-with-SHA256`, `-SHA384` and `-SHA512`, as `const_oid`
        // reads them out of the registry rather than as digits written here.
        for (certificate, value, digest, curve, algorithm) in [
            (
                ec::P256_CERTIFICATE,
                ec::P256_SIGNATURE,
                Digest::Sha256,
                crate::ecdsa::Curve::P256,
                const_oid::db::rfc5912::ECDSA_WITH_SHA_256,
            ),
            (
                ec::P384_CERTIFICATE,
                ec::P384_SIGNATURE,
                Digest::Sha384,
                crate::ecdsa::Curve::P384,
                const_oid::db::rfc5912::ECDSA_WITH_SHA_384,
            ),
            (
                ec::P521_CERTIFICATE,
                ec::P521_SIGNATURE,
                Digest::Sha512,
                crate::ecdsa::Curve::P521,
                const_oid::db::rfc5912::ECDSA_WITH_SHA_512,
            ),
        ] {
            let certificate = ec::hex(certificate);
            let parsed = crate::x509::parse(&certificate).expect("a certificate");
            let signature = curve_signature(fixtures::detached_curve(
                &certificate,
                parsed.issuer,
                parsed.serial_number,
                digest,
                algorithm.as_bytes(),
                &ec::hex(value),
            ));
            assert_eq!(
                signature.authenticity(file),
                Authenticity::Verified {
                    digest,
                    family: Family::Ecdsa(curve),
                    key_bits: curve.bits(),
                    over: Signed::TheDocumentsBytes,
                },
                "{}",
                curve.name()
            );
            assert_eq!(
                signature.authenticity(b"the signed byteS"),
                Authenticity::NotUnderThatKey {
                    digest,
                    family: Family::Ecdsa(curve),
                    key_bits: curve.bits(),
                    over: Signed::TheDocumentsBytes,
                },
                "{}",
                curve.name()
            );
        }
    }

    /// **The row ISO/TS 32002 section 5.1.2 adds to Table 260, all the way through.**
    ///
    /// The difference from every other family, and the reason this test exists beside `eddsa`'s
    /// own: RFC 8032's Ed25519 signs the *message*, so what `authenticity` hands the verifier is
    /// the byte range's parts rather than a digest of them. A path that computed a digest and
    /// passed that instead would fail here and nowhere else.
    ///
    /// **No corpus document could stand in.** Not one of the 811 signature dictionaries in 67 460
    /// documents states an `EdDSA` algorithm or an Edwards key, which `CLAUDE.md`'s trap 8 says is a
    /// fact about documents rather than about the standard — the same footing DSA is on.
    #[test]
    fn an_ed25519_signature_verifies_through_the_whole_path_a_document_takes() {
        let file = b"the signed bytes";
        let certificate = crate::ecdsa::fixtures::hex(crate::eddsa::fixtures::ED25519_CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let signature = curve_signature(fixtures::detached_curve(
            &certificate,
            parsed.issuer,
            parsed.serial_number,
            // ISO/TS 32002 Table 4 pairs Ed25519 with SHA512, and that is what a conforming
            // `SignerInfo` states — it describes the content digest, not a parameter of RFC
            // 8032's signature.
            Digest::Sha512,
            crate::eddsa::ID_ED25519.as_bytes(),
            &crate::ecdsa::fixtures::hex(crate::eddsa::fixtures::ED25519_SIGNATURE),
        ));
        assert_eq!(
            signature.authenticity(file),
            Authenticity::Verified {
                digest: Digest::Sha512,
                family: Family::EdDsa,
                key_bits: 256,
                over: Signed::TheDocumentsBytes,
            }
        );
        assert_eq!(
            signature.authenticity(b"the signed byteS"),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha512,
                family: Family::EdDsa,
                key_bits: 256,
                over: Signed::TheDocumentsBytes,
            }
        );
    }

    /// A curve ISO/TS 32002 Table 3 names and no package on this tree's line computes.
    ///
    /// What a reader is owed is the *curve*, not the key algorithm: every certificate in this case
    /// states `1.2.840.10045.2.1`, so a report naming that would say nothing about which of the
    /// six the file used. The fixture is a real brainpoolP256r1 certificate.
    #[test]
    fn a_curve_this_program_does_not_compute_on_is_named_by_its_own_identifier() {
        use crate::ecdsa::fixtures as ec;
        let file = b"the signed bytes";
        let certificate = ec::hex(ec::BP256_CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let signature = curve_signature(fixtures::detached_curve(
            &certificate,
            parsed.issuer,
            parsed.serial_number,
            Digest::Sha256,
            const_oid::db::rfc5912::ECDSA_WITH_SHA_256.as_bytes(),
            &ec::hex(ec::BP256_SIGNATURE),
        ));
        assert_eq!(
            signature.authenticity(file),
            Authenticity::CurveNotVerifiable {
                curve: "1.3.36.3.3.2.8.1.1.7 (brainpoolP256r1)".to_owned(),
            }
        );
    }

    /// A detached `adbe.pkcs7.detached` signature over the whole of `b"the signed bytes"`.
    fn curve_signature(contents: Vec<u8>) -> Signature {
        Signature {
            timestamp: false,
            handler: Some("Adobe.PPKLite".to_owned()),
            sub_filter: Some("adbe.pkcs7.detached".to_owned()),
            byte_range: vec![(0, 16)],
            contents,
            certificate_chain: false,
            chain: Vec::new(),
            name: None,
            signed_at: None,
            location: None,
            reason: None,
            contact: None,
            changes: None,
            certification: false,
        }
    }

    /// **The RSA family's other padding, all the way through.**
    ///
    /// The `pss` module's own tests exercise RFC 8017 sections 8.1.2 and 9.1.2 on a key and a
    /// signature; this exercises everything between a signature dictionary and that call — the
    /// `SignerInfo`'s `signatureAlgorithm` recognised as `id-RSASSA-PSS` rather than folded into
    /// PKCS #1 v1.5's arc, the `RSASSA-PSS-params` read out of that identifier's own parameters,
    /// the certificate lookup, and the answer naming the padding. The six real PSS signatures in
    /// the `SafeDocs` population are the demand witness; a fixture is still needed for the
    /// *positive* path over bytes this test chose, because nobody here holds those signers'
    /// private keys.
    #[test]
    fn a_pss_signature_verifies_through_the_whole_path_a_document_takes() {
        let file = b"the signed bytes";
        let certificate = crate::pss::fixtures::hex(crate::pss::fixtures::CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let signature = Signature {
            timestamp: false,
            handler: Some("Adobe.PPKLite".to_owned()),
            sub_filter: Some("ETSI.CAdES.detached".to_owned()),
            byte_range: vec![(0, file.len() as u64)],
            contents: fixtures::detached_pss(
                &certificate,
                parsed.issuer,
                parsed.serial_number,
                &crate::pss::fixtures::hex(crate::pss::fixtures::SIGNATURE_SHA256_SALT32),
            ),
            certificate_chain: false,
            chain: Vec::new(),
            name: None,
            signed_at: None,
            location: None,
            reason: None,
            contact: None,
            changes: None,
            certification: false,
        };
        assert_eq!(
            signature.authenticity(file),
            Authenticity::Verified {
                digest: Digest::Sha256,
                family: Family::RsaPss,
                key_bits: 2048,
                over: Signed::TheDocumentsBytes,
            },
            "the signer states no signed attributes, so RFC 5652 signs the byte range itself"
        );
        assert_eq!(
            signature.authenticity(b"the signed byteS"),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha256,
                family: Family::RsaPss,
                key_bits: 2048,
                over: Signed::TheDocumentsBytes,
            }
        );
    }

    /// A DSA signature over an RSA key is two claims by one producer that disagree.
    ///
    /// Neither is believed and neither is guessed at: before the four-hundred-and-seventy-ninth
    /// session there was one family and the question could not arise, and with two it can.
    #[test]
    fn a_signature_algorithm_and_a_key_from_different_families_are_both_reported() {
        let file = b"the signed bytes";
        let certificate = hex(CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let signature = Signature {
            timestamp: false,
            handler: Some("Adobe.PPKLite".to_owned()),
            sub_filter: Some("adbe.pkcs7.detached".to_owned()),
            byte_range: vec![(0, file.len() as u64)],
            // A DSA `signatureAlgorithm` over the *RSA* certificate and its signature.
            contents: fixtures::detached_dsa(
                &certificate,
                parsed.issuer,
                parsed.serial_number,
                &hex(PKCS1_SIGNATURE),
            ),
            certificate_chain: false,
            chain: Vec::new(),
            name: None,
            signed_at: None,
            location: None,
            reason: None,
            contact: None,
            changes: None,
            certification: false,
        };
        assert_eq!(
            signature.authenticity(file),
            Authenticity::KeyDoesNotMatchAlgorithm {
                algorithm: "2.16.840.1.101.3.4.3.2".to_owned(),
                key: "1.2.840.113549.1.1.1".to_owned(),
            }
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

    /// **§12.8.3.4.5 step (a)'s second half, under the `/SubFilter` that subclause is scoped to.**
    ///
    /// The step requires a handler to "use the public key contained in the signer's certificate to
    /// verify that the document digest found in the signature is correctly signed", and
    /// [`Signature::authenticity`] matches the `signatureAlgorithm` against the certificate's key
    /// rather than against `/SubFilter` — so every family reaches a `PAdES` signature the same way
    /// it reaches an `adbe.pkcs7.detached` one. **By construction is exactly what a test is for
    /// here**: §12.8.3.4's row and §12.8.3.4.5's both recorded that half of the step as answered
    /// for RSA and DSA alone, four rounds after ADR 0532 added the two elliptic-curve families,
    /// and nothing in this tree contradicted them.
    ///
    /// The curves are this subclause's by the standard's own words rather than by inference: the
    /// applicability sentence above ISO/TS 32002 section 5.1.3's Table 3 and the one above section
    /// 5.1.2's Table 4 each name `ETSI.CAdES.detached` among the `/SubFilter` values they cover.
    /// One curve is enough for that question — `an_ecdsa_signature_verifies_through_the_whole_path_a_document_takes`
    /// is where all three are exercised, and what this adds is the `/SubFilter` around them.
    #[test]
    fn a_pades_signature_verifies_under_an_elliptic_curve_key() {
        use crate::ecdsa::fixtures as ec;
        let file = b"the signed bytes";
        let certificate = ec::hex(ec::P256_CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let mut signature = curve_signature(fixtures::detached_curve(
            &certificate,
            parsed.issuer,
            parsed.serial_number,
            Digest::Sha256,
            const_oid::db::rfc5912::ECDSA_WITH_SHA_256.as_bytes(),
            &ec::hex(ec::P256_SIGNATURE),
        ));
        signature.sub_filter = Some("ETSI.CAdES.detached".to_owned());
        assert_eq!(
            signature.authenticity(file),
            Authenticity::Verified {
                digest: Digest::Sha256,
                family: Family::Ecdsa(crate::ecdsa::Curve::P256),
                key_bits: crate::ecdsa::Curve::P256.bits(),
                over: Signed::TheDocumentsBytes,
            },
            "ISO/TS 32002 Table 3 covers ETSI.CAdES.detached by name"
        );
        assert_eq!(
            signature.authenticity(b"the signed byteS"),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha256,
                family: Family::Ecdsa(crate::ecdsa::Curve::P256),
                key_bits: crate::ecdsa::Curve::P256.bits(),
                over: Signed::TheDocumentsBytes,
            },
            "and the step's verdict is decisive in the other direction too"
        );
    }
}
