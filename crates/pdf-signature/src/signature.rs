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
//!    six and one of ISO/TS 32002 Table 4's two are named by their own identifier and computed by
//!    nothing,
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
//! corpus document exercises. [`signing_certificate_bindings`] is §12.8.3.4.5 (a)'s *first*
//! sentence — the signer's certificate against the hash the signer signed over it — which needs
//! RFC 5035 and a hash function and no trust store at all, and which [`Signature::authenticity`]
//! therefore asks before it answers that step's second sentence.

use std::borrow::Cow;
use std::sync::Arc;

use crate::cms::{self, CmsError, Digest, SignatureAlgorithm, SignedData};
use crate::der;
use crate::dsa::{self, DsaError};
use crate::ecdsa::{self, EcdsaError};
use crate::eddsa::{self, EdDsaError};
use crate::ess::{self, EssError};
use crate::pkcs1::{self, Pkcs1Error};
use crate::pss;
use crate::trust::{self, Trust, TrustAnchors};
use crate::x509::{self, Instant, X509Error};
use pdf_syntax::{Dictionary, Document, FileBytes, Object, ObjectId};

/// Most signatures read from one document.
///
/// §12.8.1 permits "[o]ne or more approval signatures" and any number of timestamps, so the
/// bound is on a file built to make a reader work rather than on any real workflow.
const MAX_SIGNATURES: usize = 1024;

/// How much of a signed range is resident while it is digested, in bytes.
///
/// A window rather than the range (ADR 0812): the range of a signed document is the document,
/// and a digest is fed a piece at a time whatever the piece. Sixty-four kibibytes is the size
/// `pdf_syntax` reads a cross-reference section through, and it was measured rather than
/// assumed — `the_window_a_signed_range_is_digested_through_is_priced`, an ignored test below,
/// prints the cost of the whole file's range through four windows against the range held
/// whole, and ADR 0812 records what it printed.
const SIGNED_WINDOW: usize = 64 * 1024;

/// Most `(offset, length)` pairs a `/ByteRange` may state before it stops describing a file.
///
/// §12.8.1 describes two — everything before the signature value and everything after it — and
/// says "[m]ultiple discontiguous byte ranges shall be used to describe a digest that does not
/// include the signature value" without bounding them. A file stating more than this is refused
/// whole rather than read to the bound, so a truncated range never reaches the arithmetic:
/// [`Signature::coverage`] answers [`Coverage::Malformed`] and a person is told.
const MAX_BYTE_RANGE_PAIRS: usize = 64;

/// Most keys named as stating an indirect value, before the list stops being a report.
///
/// Table 255 has eighteen entries and §12.8.1 permits private ones beside them, so a dictionary
/// naming more than this is a file built to make a reader allocate rather than one a person will
/// read a list of.
const MAX_INDIRECT_VALUES: usize = 64;

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
    /// `/V`, Table 255's version of the signature dictionary format — and the one sentence of
    /// that entry addressed to whoever *validates* rather than to whoever signs (§12.8.1).
    ///
    /// > The value is 1 if the Reference dictionary shall be considered critical to the
    /// > validation of the signature.
    ///
    /// So a file writing `/V 1` is saying that the transform methods in its `/Reference` array
    /// are part of what makes the signature good, and a validator that ignores them has not
    /// finished. This program does not evaluate a transform method at all — §12.8.2.2.2's
    /// comparison of two revisions is what that would take, and the ledger records it as not
    /// done — so the entry is read in order to be *said*, by [`Self::reference_is_critical`],
    /// rather than to change an answer. Table 255 gives the default as 0.
    ///
    /// Kept as the file's own integer rather than as a `bool`, because the entry is a version
    /// number: 0 and 1 are the two the standard gives meanings to and a third value is a claim
    /// about a format edition nobody here has read.
    pub format_version: Option<i64>,
    /// The keys of this dictionary whose values the file wrote as indirect references.
    ///
    /// §12.8.1 forbids every one of them wherever a byte range digest is present:
    ///
    /// > When a byte range digest is present, all values in the signature dictionary shall be
    /// > direct objects.
    ///
    /// **The condition is the clause's and is applied by whoever reports, not here**: this list
    /// is a fact about the dictionary, and `/ByteRange` being non-empty is what makes a non-empty
    /// list a departure. The rule is worth reading rather than assuming — an indirect `/ByteRange`
    /// or `/Contents` is an object a later incremental update can redefine while the bytes the
    /// digest was taken over do not move, which is the same hole [`Excluded`] closes from the
    /// other side.
    ///
    /// Sorted, so that a report naming them reads the same twice, and bounded by
    /// [`MAX_INDIRECT_VALUES`] because the count would otherwise come out of the file.
    pub indirect_values: Vec<String>,
    /// What each of `/Reference`'s signature reference dictionaries states as its
    /// `/DigestMethod`, in the order the array states them.
    ///
    /// Table 256: "[a] name identifying the algorithm that shall be used when computing the
    /// digest if not specified in the certificate." So the entry parameterises §12.8.2's
    /// modification analysis rather than the byte range digest [`Self::integrity`] takes - which
    /// is why reading it changes no answer this program gives today and is still worth doing:
    /// §12.8.2.2.2's comparison of two revisions is not implemented, and a file that names a
    /// digest for a comparison nobody makes has said something a reader should be told rather
    /// than something a reader may drop.
    ///
    /// **The entry is Optional and deprecated in PDF 2.0, which is an erratum's doing rather than
    /// the printed table's**: the cell in `doc/md/` opens "(Required)" and Errata Collection 3's
    /// issue #117 strikes that word out. §12.8.1's ledger row carries the reading.
    pub reference_digests: Vec<ReferenceDigest>,
}

/// What one signature reference dictionary's Table 256 `/DigestMethod` states.
///
/// Three answers rather than an `Option<Digest>`, because a name outside the entry's value list
/// and no name at all are different facts about a file and only one of them is a departure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceDigest {
    /// One of the six names Table 256 admits, as the function this program computes.
    Stated(Digest),
    /// A name the entry's own value list does not admit, carried so a report can say which.
    NotInTheTable(String),
    /// No `/DigestMethod` at all, which Errata Collection 3's issue #117 makes conforming.
    Absent,
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

/// What a signature's `/ByteRange` leaves **out** of the file.
///
/// **[`Coverage`] cannot answer this, and the difference between the two is the classic
/// forgery.** Coverage is arithmetic over the pairs: they start at zero, they ascend, they reach
/// the file's end. What no arithmetic over the pairs can see is *what sits in the region between
/// two of them* — the one place in a signed document where bytes lie that no digest was taken
/// over. §12.8.1 says what belongs there:
///
/// > This range should be the entire PDF file, including the signature dictionary but excluding
/// > the signature value itself (the Contents entry).
///
/// and Table 255's `/ByteRange` entry states the same thing as a `shall`:
///
/// > Multiple discontiguous byte ranges shall be used to describe a digest that does not include
/// > the signature value (the Contents entry) itself.
///
/// So the excluded region is the signature value and nothing besides it. A range that excludes
/// more — a second region, or a first one wider than the string — hides bytes a reader parses
/// and a digest never saw, while every other answer this module gives stays green: the pairs
/// still tile the file, the recomputed digest still matches, and the signature still verifies
/// under the signer's key. That is why every variant but the first is a refusal by name rather
/// than a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Excluded {
    /// One region, and it is `/Contents` written as §12.8.3.3.1 requires: the digits with both
    /// of their delimiters.
    ///
    /// > For byte range signatures, Contents shall be a hexadecimal string with "&lt;" and "&gt;"
    /// > delimiters. It shall fit precisely in the space between the ranges specified by
    /// > ByteRange .
    TheSignatureValue,
    /// One region, and it holds the value's digits with a delimiter left inside the signed range.
    ///
    /// **Nothing is hidden and §12.8.3.3.1 is still not met**, which is why this is its own
    /// answer rather than either of its neighbours. The region carries the octets of `/Contents`
    /// and nothing else, so there is no room in it for the unsigned content
    /// [`Self::NotTheSignatureValue`] exists to catch; what fails is the sentence above — the
    /// string does not "fit precisely in the space between the ranges", because one or both of
    /// its delimiters are outside that space and under the digest. `signed_verified.pdf` is the
    /// shape, and a producer that signs its own delimiters has made them unmovable rather than
    /// unchecked, which is why the distinction is reported and not ranked.
    TheDigitsOfTheSignatureValue,
    /// One region, and what it holds is not this signature's value as the file wrote it.
    NotTheSignatureValue {
        /// Where the region starts in the file.
        at: u64,
        /// How many bytes of it there are.
        length: u64,
    },
    /// The pairs leave out more than one region, so something besides the value is unsigned.
    MoreThanOneRegion {
        /// How many regions the pairs leave out.
        regions: usize,
    },
    /// The pairs leave nothing out, so the signature value is inside the digest it records.
    Nothing,
    /// The `/ByteRange` does not describe this file, which is [`Coverage::Malformed`]'s condition.
    RangeNotInThisFile,
    /// The `/ByteRange` names bytes of this file and the file on disk would not give them.
    RangeNotReadable,
}

/// Where the bytes a signature signed stop, measured against §12.8.1's rule for a signed range.
///
/// The clause names both ends of the range in one sentence and [`Coverage`] answers the first
/// half — a range that does not start at zero has not signed the header. This is the second
/// half, and it is what gives [`Coverage::Unsigned`] a meaning: a tail is §12.8.1's NOTE 1's
/// ordinary incremental update only if what precedes it is a *whole* revision. A range that
/// stops in the middle of one has signed a prefix of a revision whose remainder the reader goes
/// on to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedEnd {
    /// The last signed byte ends an `%%EOF` comment, or the EOL marker permitted after one.
    AtAnEndOfFileMarker,
    /// It ends somewhere else.
    Elsewhere,
    /// The `/ByteRange` names bytes this file does not have.
    RangeNotInThisFile,
    /// The file on disk would not give the bytes the range names.
    RangeNotReadable,
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
    /// The `/ByteRange` names bytes of this file and the file on disk would not give them.
    ///
    /// The reader's refusal, by name rather than as a mismatch (ADR 0812): the pairs were inside
    /// the file's stated length and a window came back short, because the file shrank under the
    /// reader or a read failed. `pdf_syntax::FileBytes::read_failure` on the document's bytes
    /// says which. Distinct from [`Self::RangeNotInThisFile`], which is a fact about the range,
    /// and from [`Self::Changed`], which this must never be mistaken for: a digest over fewer
    /// bytes than the range names would differ from the recorded one and read as a modified
    /// document.
    RangeNotReadable,
    /// The dictionary states no `/Contents`, which Table 255 makes required.
    NoSignatureValue,
    /// The signature value could not be read as §12.8.3.3's CMS object.
    Unreadable(CmsError),
}

/// Why the bytes `/ByteRange` names could not be read off the file.
///
/// Two facts, kept apart because they are about two different things: the first is about the
/// range and the second about the disk. Both reach a person through
/// [`Integrity::RangeNotInThisFile`] and [`Integrity::RangeNotReadable`], and their
/// [`Authenticity`] twins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeProblem {
    /// A pair names bytes outside the file: the condition [`Coverage::Malformed`] reports.
    NotInThisFile,
    /// A pair is inside the file and the file on disk gave fewer bytes than it names.
    NotReadable,
}

impl From<RangeProblem> for Authenticity {
    fn from(problem: RangeProblem) -> Self {
        match problem {
            RangeProblem::NotInThisFile => Self::RangeNotInThisFile,
            RangeProblem::NotReadable => Self::RangeNotReadable,
        }
    }
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
    /// `namedCurve`, so reporting the key algorithm would tell a reader nothing.
    /// ISO/TS 32002 Table 3's three Brainpool curves are what reach here today, and this program
    /// lacking them is a gap rather than a defect in the file; a curve outside those two tables is
    /// the file
    /// leaving what the Technical Specification admits, and section 5.1.3's last sentence permits
    /// exactly this treatment: "PDF processors may ignore or handle in an implementation-dependent
    /// manner PDF documents which are signed with elliptic curves not listed in Table 3 or Table
    /// 4."
    CurveNotVerifiable {
        /// The curve's object identifier as dotted decimal, with its ISO/TS 32002 Table 3 name
        /// where it has
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
    /// The `/ByteRange` names bytes of this file and the file on disk would not give them.
    ///
    /// See [`Integrity::RangeNotReadable`], which is the same refusal for the same reason.
    RangeNotReadable,
    /// The signer signed a statement naming the certificate it used, and it is not this one.
    ///
    /// §12.8.3.4.5 (a): "[a] signature handler shall compare the hash value of the signer's
    /// certificate, with the hash value given in the signing-certificate attribute or the
    /// signing-certificate-v2 attribute. If the hashes do not match, then the signature is
    /// considered invalid." RFC 5035 section 5.4.1 puts the same rule on any CMS object carrying
    /// the attribute — "[i]f the hash of the certificate does not match the certificate used to
    /// verify the signature, the signature MUST be considered invalid" — which is why this is
    /// reached for every CMS `/SubFilter` and not only `ETSI.CAdES.detached`.
    ///
    /// **Decisive, and it is the one answer here that a signature value verifying does not
    /// override.** The `SignerInfo`'s `sid` — what picks the certificate out of the object — is
    /// not covered by the signature (RFC 5035 section 5.4.1.1), so a substituted certificate is
    /// exactly what this attribute exists to catch and the signature value alone cannot.
    SigningCertificateMismatch {
        /// Which of RFC 5035's two attributes stated the hash.
        version: ess::Version,
        /// The function it was stated under.
        digest: Digest,
    },
    /// The signer states such an attribute and this program could not make that comparison.
    ///
    /// A refusal rather than a verdict, and never a pass: the comparison is the only thing binding
    /// the verifying key to what the signer meant to sign with, so skipping it and answering
    /// [`Self::Verified`] would be this program reporting a check it did not make.
    SigningCertificateUnverifiable {
        /// Which of the two attributes could not be acted on.
        version: ess::Version,
        /// What stopped it.
        statement: String,
    },
    /// The signer's signed attributes are not DER encoded, so what they signed is not in the file.
    ///
    /// §12.8.3.3 requires that "[t]he CMS object shall conform to Internet RFC 5652", and RFC 5652
    /// section 5.3 places one encoding requirement inside a structure it otherwise writes in BER:
    /// "SignedAttributes MUST be DER encoded, even if the rest of the structure is BER encoded."
    /// Section 5.4 is what makes that a verifier's problem rather than a producer's tidiness -
    /// the signature is over "the message digest of the complete DER encoding of the SignedAttrs
    /// value", so where an attribute inside the set states X.690 clause 8.1.3.6's indefinite
    /// length the octets the file holds are not the octets the signer digested.
    ///
    /// **A refusal rather than a verdict, and that is the whole point of the variant.** Digesting
    /// the file's bytes anyway would answer [`Self::NotUnderThatKey`] - *this signature does not
    /// verify* - where the truth is that this program could not construct what the signature was
    /// made over. [`crate::der::every_length_is_definite`] is the question, and it is asked of the
    /// set's *contents*: the `signedAttrs [0] IMPLICIT` header's own length octets are the one
    /// thing section 5.4 replaces - "[t]he IMPLICIT [0] tag in the signedAttrs is not used for the
    /// DER encoding, rather an EXPLICIT SET OF tag is used" - so a producer that wrote the wrapper
    /// in the indefinite form has departed from section 5.3 without changing a byte any verifier
    /// digests, and this program can still complete the check.
    SignedAttributesNotDer,
    /// The signature value could not be read as §12.8.3.3's CMS object.
    Unreadable(CmsError),
}

/// Which algorithm family a signature was checked with — and, inside a family, which construction.
///
/// Table 260 names three families and ISO/TS 32002 section 5.1.2 adds a fourth row to that table,
/// so there are four here rather than three. RSA has two arms because the table's "RSA Algorithm
/// Support" row states key sizes and no padding, and the sentence a person reads should say which
/// construction did the verifying; ECDSA carries its curve for the same reason, since ISO/TS 32002
/// ISO/TS 32002 Table 3 is a list of curves rather than of key sizes.
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
    /// No curve beside it, because ISO/TS 32002 Table 4's two are one implemented and one refused
    /// by number —
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
            Self::Ecdsa(ecdsa::Curve::BrainpoolP256r1) => "ECDSA (brainpoolP256r1)",
            Self::Ecdsa(ecdsa::Curve::BrainpoolP384r1) => "ECDSA (brainpoolP384r1)",
            Self::EdDsa => "EdDSA (Ed25519)",
        }
    }
}

/// A requirement §12.8.3.4 places on a `PAdES` signature that a file does not meet.
///
/// Every one of these is checkable with no cryptography at all, which is why they are here: a
/// *structural* rule is arithmetic over what the file says, while §12.8.3.4.5's *validation* steps
/// need a digest at least and a trust store for three of the four. A departure is not a verdict —
/// it is a file breaking a `shall`, said out loud, which is what this project does with those.
///
/// The line is where the work is, not where the clause number is: §12.8.3.4.3 (f) is here because
/// the attribute's *presence* is a fact about the file, and the hash inside it is
/// [`signing_certificate_bindings`] because comparing it means hashing a certificate.
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
    /// §12.8.3.4.3 (f): "signing-certificate or signing-certificate-v2: shall be used as a signed
    /// attribute … The details of the signing certificate attribute are defined in Internet RFC
    /// 5035 ."
    ///
    /// The `shall` this states is ISO 32000-2's own and needs nothing beyond it: *one of the two,
    /// among the signed attributes.* What clause 5.2.2 of ETSI EN 319 122-1 adds about how each is
    /// built is a document this tree does not hold, and is not what is checked here. What RFC 5035
    /// contributes — and it is held — is the pair of object identifiers, and its own rule that
    /// neither may be an unsigned attribute (sections 5.4.1, 5.4.2), which is why an unsigned one
    /// does not satisfy this.
    NoSigningCertificateAttribute,
    /// §12.8.3.4.3 (h): "signer-location … may be present. In such a case, the Location entry in
    /// the signature dictionary shall not be present."
    ///
    /// The `shall` is ISO 32000-2's own and needs nothing else; what comes from elsewhere is the
    /// attribute's *number*, and [`cms::ID_AA_ETS_SIGNER_LOCATION`] says which document that is and
    /// what it costs. Only the signed attributes are searched, because RFC 5126 section 5.11.2
    /// makes that the only place the attribute may be: "[t]he signer-location attribute shall be a
    /// signed attribute."
    SignerLocationAndLocationEntry,
    /// §12.8.3.4.3 (i): "these attributes shall not be used: counter-signature, content-reference,
    /// content-identifier, and contenthints."
    ///
    /// One variant per attribute, because they are four separate facts about a file. The
    /// identifiers come from two documents this tree holds: RFC 5652 assigns `counter-signature`
    /// its own, and RFC 5035's Appendix A module assigns the other three — `id-aa-contentReference`,
    /// `id-aa-contentIdentifier` and `id-aa-contentHint`.
    ///
    /// The clause forbids each "attribute", without saying signed or unsigned, so both sets are
    /// searched. RFC 5652 section 5.3 makes `counter-signature` an unsigned attribute and RFC 5035
    /// section 2.7 makes `content-identifier` a signed one, so a rule that looked in one set only
    /// would be a rule that could not see half of its own subject.
    CounterSignature,
    /// §12.8.3.4.3 (i)'s second forbidden attribute — RFC 5035's `id-aa-contentReference`.
    ContentReference,
    /// Its third — RFC 5035's `id-aa-contentIdentifier`.
    ContentIdentifier,
    /// Its fourth, which the clause spells "contenthints" — RFC 5035's `id-aa-contentHint`.
    ContentHints,
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
            .field("format_version", &self.format_version)
            .field("indirect_values", &self.indirect_values)
            .field("reference_digests", &self.reference_digests)
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

    /// Whether the file marks its signature reference dictionary critical to validating this
    /// signature — Table 255's `/V 1`, and the entry's own condition (§12.8.1).
    ///
    /// > The value is 1 if the Reference dictionary shall be considered critical to the
    /// > validation of the signature.
    ///
    /// The condition is the entry's and nothing is added to it: `/V` absent is Table 255's
    /// default of 0, and any other value says nothing about criticality. What the answer is
    /// *for* is a sentence rather than a verdict — this program evaluates no transform method,
    /// so a `true` here names the part of the validation it does not do, on the file's own say-so.
    #[must_use]
    pub fn reference_is_critical(&self) -> bool {
        self.format_version == Some(1)
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

    /// What this signature's `/ByteRange` leaves out of `file`, read off the file itself.
    ///
    /// [`Excluded`] is where the argument for this check is; the mechanics are here. Table 255
    /// fixes how the signature value is written, and that is what makes the comparison exact
    /// rather than a guess: "[w]hen `ByteRange` is present, the value shall be a hexadecimal
    /// string (see 7.3.4.3, "Hexadecimal strings")". §7.3.4.3's own two rules are applied to the
    /// region — its whitespace "shall be ignored", and a missing final digit "shall be assumed to
    /// be 0" — and the octets it decodes to are compared with `/Contents` as the parser read it.
    ///
    /// **The two angle brackets are read rather than assumed, because producers place them on
    /// both sides of the region.** `160F-2019.pdf` excludes `<`…`>` whole, which is what
    /// §12.8.3.3.1 requires; `signed_verified.pdf` excludes the digits alone and signs the
    /// delimiters, which hides nothing and still departs from that clause's "shall fit precisely".
    /// The two are separate answers — [`Excluded::TheSignatureValue`] and
    /// [`Excluded::TheDigitsOfTheSignatureValue`] — so that the security question and the
    /// conformance question are not made into one. §7.3.4.3's whitespace is admitted anywhere in
    /// the region, because whitespace carries no object wherever it sits; anything else fails.
    ///
    /// Streamed through [`SIGNED_WINDOW`], for the reason [`Self::each_signed_window`] gives: the
    /// region's size comes out of the file, and a range written to be refused can name gigabytes
    /// of it.
    ///
    /// Nothing here is a verdict on the signature — the module comment says why none of these
    /// answers is — but it is the question [`Coverage::WholeFile`] leaves open, and a document
    /// this answers [`Excluded::NotTheSignatureValue`] about is one whose signed digest is worth
    /// less than it looks.
    #[must_use]
    pub fn excluded(&self, file: &FileBytes) -> Excluded {
        let length = u64::try_from(file.len()).unwrap_or(u64::MAX);
        if self.coverage(length) == Coverage::Malformed {
            return Excluded::RangeNotInThisFile;
        }
        let mut regions: Vec<(u64, u64)> = Vec::new();
        let mut end = 0_u64;
        for (index, &(start, size)) in self.byte_range.iter().enumerate() {
            if index > 0 && start > end {
                regions.push((end, start.saturating_sub(end)));
            }
            end = start.saturating_add(size);
        }
        match regions.as_slice() {
            [] => Excluded::Nothing,
            [(at, size)] => self.region_holds_the_value(file, *at, *size),
            _ => Excluded::MoreThanOneRegion {
                regions: regions.len(),
            },
        }
    }

    /// Whether the `size` bytes at `at` are the hexadecimal string holding `/Contents`.
    fn region_holds_the_value(&self, file: &FileBytes, at: u64, size: u64) -> Excluded {
        let (Ok(from), Ok(width)) = (usize::try_from(at), usize::try_from(size)) else {
            return Excluded::RangeNotInThisFile;
        };
        let Some(to) = from.checked_add(width).filter(|end| *end <= file.len()) else {
            return Excluded::RangeNotInThisFile;
        };
        let mut scan = HexScan::new(&self.contents);
        let mut position = from;
        while position < to {
            let stop = position.saturating_add(SIGNED_WINDOW).min(to);
            let bytes = file.read(position..stop);
            if bytes.len() != stop.saturating_sub(position) {
                return Excluded::RangeNotReadable;
            }
            scan.feed(&bytes);
            position = stop;
        }
        match scan.finish() {
            Some(true) => Excluded::TheSignatureValue,
            Some(false) => Excluded::TheDigitsOfTheSignatureValue,
            None => Excluded::NotTheSignatureValue { at, length: size },
        }
    }

    /// Where the bytes this signature signed stop, measured against §12.8.1.
    ///
    /// > In case of multiple digital signatures this range shall be the sequence of bytes starting
    /// > from the "%PDF-" comment at the beginning of the PDF document to the end of the "%%EOF"
    /// > comment, possibly followed by an optional EOL marker, terminating the incremental update
    /// > that adds the digital signature dictionary to the document.
    ///
    /// §7.5.5 is what makes the marker a dependable end — "[t]he last line of the file shall
    /// contain only the end-of-file marker, %%EOF" — and the optional EOL the sentence permits is
    /// §7.2.3's: a CARRIAGE RETURN, a LINE FEED, or the pair. [`Excluded`] says what the *hole* in
    /// a range may hold; this says where the range may stop, and the two together are the whole of
    /// what §12.8.1 asks of a `/ByteRange` without a certificate in hand.
    #[must_use]
    pub fn signed_end(&self, file: &FileBytes) -> SignedEnd {
        /// §7.5.5's end-of-file marker.
        const MARKER: &[u8] = b"%%EOF";
        let Some(&(start, size)) = self.byte_range.last() else {
            return SignedEnd::RangeNotInThisFile;
        };
        let Ok(end) = usize::try_from(start.saturating_add(size)) else {
            return SignedEnd::RangeNotInThisFile;
        };
        if end > file.len() {
            return SignedEnd::RangeNotInThisFile;
        }
        let from = end.saturating_sub(MARKER.len().saturating_add(2));
        let tail = file.read(from..end);
        if tail.len() != end.saturating_sub(from) {
            return SignedEnd::RangeNotReadable;
        }
        let mut bytes: &[u8] = &tail;
        if bytes.ends_with(b"\r\n") {
            bytes = &bytes[..bytes.len().saturating_sub(2)];
        } else if bytes.ends_with(b"\n") || bytes.ends_with(b"\r") {
            bytes = &bytes[..bytes.len().saturating_sub(1)];
        }
        if bytes.ends_with(MARKER) {
            SignedEnd::AtAnEndOfFileMarker
        } else {
            SignedEnd::Elsewhere
        }
    }

    /// Runs `feed` over every byte `/ByteRange` names, in the order the pairs state them, a
    /// window at a time.
    ///
    /// **Nothing here holds more of the file than one window** (ADR 0812). A signed document's
    /// range is every byte of it but the hole where `/Contents` sits, so a reader that took the
    /// ranges as slices of a whole file held the whole file — which on disk is `FileBytes::whole`
    /// and, for the six-gigabyte document ADR 0809 opened at the cost of its trailer, would have
    /// been the whole six gigabytes again. A digest is over every signed byte and cannot avoid
    /// reading them; what it can avoid is keeping them, and [`SIGNED_WINDOW`] is what it keeps.
    ///
    /// Refuses by name what the reader refuses: a pair outside the file is
    /// [`RangeProblem::NotInThisFile`], the condition [`Coverage::Malformed`] reports; and a window
    /// that came back short on disk is [`RangeProblem::NotReadable`] — the file ended early or a
    /// read failed, and `FileBytes::read_failure` says which.
    fn each_signed_window(
        &self,
        file: &FileBytes,
        feed: impl FnMut(&[u8]),
    ) -> Result<(), RangeProblem> {
        self.each_signed_window_of(file, SIGNED_WINDOW, feed)
    }

    /// [`Self::each_signed_window`] with the window stated, for the measurement that chose it.
    fn each_signed_window_of(
        &self,
        file: &FileBytes,
        window: usize,
        mut feed: impl FnMut(&[u8]),
    ) -> Result<(), RangeProblem> {
        let window = window.max(1);
        if self.byte_range.is_empty() {
            return Err(RangeProblem::NotInThisFile);
        }
        for &(start, length) in &self.byte_range {
            let start = usize::try_from(start).map_err(|_| RangeProblem::NotInThisFile)?;
            let length = usize::try_from(length).map_err(|_| RangeProblem::NotInThisFile)?;
            let end = start
                .checked_add(length)
                .ok_or(RangeProblem::NotInThisFile)?;
            if end > file.len() {
                return Err(RangeProblem::NotInThisFile);
            }
            let mut at = start;
            while at < end {
                let stop = at.saturating_add(window).min(end);
                let bytes = file.read(at..stop);
                // The pair was inside the file's stated length, so a short window is the disk's
                // doing rather than the range's: the file shrank, or a read failed.
                if bytes.len() != stop.saturating_sub(at) {
                    return Err(RangeProblem::NotReadable);
                }
                feed(&bytes);
                at = stop;
            }
        }
        Ok(())
    }

    /// The digests of the bytes `/ByteRange` names, one per algorithm asked for, in one pass.
    ///
    /// One pass rather than one per algorithm because §12.8.3.2's signature states no digest
    /// and [`Digest::TRIED_WHEN_UNSTATED`] has to be tried in turn: six reads of a signed
    /// document would be six times its length off the disk for one answer. Every hasher is fed
    /// every window, so the file is read once whatever is asked.
    ///
    /// # Errors
    ///
    /// [`RangeProblem`], as [`Self::each_signed_window`] refuses.
    pub fn signed_digests(
        &self,
        file: &FileBytes,
        algorithms: &[Digest],
    ) -> Result<Vec<Vec<u8>>, RangeProblem> {
        let mut hashers: Vec<cms::Hasher> =
            algorithms.iter().map(|digest| digest.hasher()).collect();
        self.each_signed_window(file, |window| {
            for hasher in &mut hashers {
                hasher.update(window);
            }
        })?;
        Ok(hashers.into_iter().map(cms::Hasher::finish).collect())
    }

    /// The bytes this signature was made over, resident: the file with `/ByteRange`'s hole in it.
    ///
    /// **The one route that still holds every signed byte at once, and it is for the one
    /// construction that needs the message itself rather than a digest of it**: RFC 8032's
    /// Ed25519 hashes `R ‖ A ‖ M` under SHA-512 inside the verification, and
    /// [`crate::eddsa::verify`] takes the message in parts. Every other arm of
    /// [`Self::authenticity`] and the whole of [`Self::integrity`] go through
    /// [`Self::signed_digests`] and hold one window. Borrowed where the file is in memory, and
    /// read range by range where it is on disk — into memory the process has to find room for,
    /// which is the refusal [`RangeProblem::NotReadable`] carries when it cannot.
    fn signed_bytes<'a>(&self, file: &'a FileBytes) -> Result<Vec<Cow<'a, [u8]>>, RangeProblem> {
        if self.byte_range.is_empty() {
            return Err(RangeProblem::NotInThisFile);
        }
        let mut pieces = Vec::with_capacity(self.byte_range.len());
        for &(start, length) in &self.byte_range {
            let start = usize::try_from(start).map_err(|_| RangeProblem::NotInThisFile)?;
            let length = usize::try_from(length).map_err(|_| RangeProblem::NotInThisFile)?;
            let end = start
                .checked_add(length)
                .ok_or(RangeProblem::NotInThisFile)?;
            if end > file.len() {
                return Err(RangeProblem::NotInThisFile);
            }
            let piece = file.read(start..end);
            if piece.len() != length {
                return Err(RangeProblem::NotReadable);
            }
            pieces.push(piece);
        }
        Ok(pieces)
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

    /// **Is the signer anyone the reader has been told to believe?** — §12.8.1's third question.
    ///
    /// The certificate this returns a verdict about is the same one [`Self::authenticity`]
    /// verified under, found the same two ways RFC 5652 permits a `SignerInfo` to name one; the
    /// rest of what the CMS object carries becomes the pool a certification path is built from.
    /// [`crate::trust::validate`] is the algorithm and RFC 5280 section 6.1 is the algorithm's
    /// definition; this method is the clause's end of it.
    ///
    /// **`anchors` and `at` are the caller's, and that is deliberate.** RFC 5280 section 6.1.1
    /// makes both inputs — the trust anchors are input (d) and "the current date/time" is input
    /// (b) — and `CLAUDE.md` principle 3 puts a policy where a host can supply it rather than
    /// inside a crate no host can reach. So this method holds no certificate list, reads no file,
    /// opens no socket and asks no clock, and [`crate::trust::TrustAnchors::none`] — which is
    /// what every caller in this tree passes today — yields [`Trust::NoAnchorSupplied`]. ADR 1039.
    ///
    /// **`material` is §12.8.4's, and is the one input that does not come from outside.** RFC 5280
    /// section 6.1.3 (a)(3) — revocation — is answered from the CRLs and OCSP responses the
    /// *document* carries, which is what a document security store exists for; [`security_store`]
    /// is what reads them and [`crate::revocation`] what applies them. Nothing here opens a socket.
    ///
    /// **Nothing this returns means *valid*.** A [`crate::revocation::Revocation::Good`] says the
    /// material in this file does not revoke the certificate, and says nothing about whether the
    /// signer is anybody — which is still [`Trust::NoAnchorSupplied`]'s answer in this tree.
    #[must_use]
    pub fn trust(
        &self,
        anchors: &TrustAnchors<'_>,
        material: &crate::revocation::Material<'_>,
        at: Instant,
    ) -> Trust {
        self.trust_for(anchors, material, at, trust::Purpose::Unstated)
    }

    /// [`Self::trust`] with the use this program has for the signer's key stated.
    ///
    /// The fourth input, and the last one RFC 5280 leaves to a caller: section 4.2.1.12's
    /// `extKeyUsage` is a statement about *use*, so only whoever has a use can process it.
    /// §12.8.5's document timestamp is the caller with one — RFC 3161 section 2.3 requires
    /// `id-kp-timeStamping` in a timestamp authority's certificate and requires it critical — and
    /// [`crate::trust::Purpose`] carries the argument.
    #[must_use]
    pub fn trust_for(
        &self,
        anchors: &TrustAnchors<'_>,
        material: &crate::revocation::Material<'_>,
        at: Instant,
        purpose: trust::Purpose,
    ) -> Trust {
        if anchors.is_empty() {
            return Trust::NoAnchorSupplied;
        }
        // **§12.8.3.2's value is not a CMS object and its chain is in the dictionary.** That clause
        // puts it there in as many words — "[t]he certificate chain of the signer shall be stored
        // in the Cert entry" — so a path for an `adbe.x509.rsa_sha1` signature is built from
        // `/Cert` and never from a `SignedData` the value does not contain. §12.8.3.4.2 forbids
        // the entry for a PAdES signature, which is why the two routes cannot be tried in turn:
        // the sub-filter decides which one a file meant.
        if self.sub_filter.as_deref() == Some("adbe.x509.rsa_sha1") {
            return self.chain_trust(anchors, material, at, purpose);
        }
        let Ok(cms) = self.signed_data() else {
            return Trust::NoPathToAnyAnchor { examined: 0 };
        };
        // §12.8.4's store and §12.8.3.3.2's signed attribute are one supply, and §12.8.4.2 is
        // what puts the material in two places rather than one: "Some of this information, i.e.
        // certificates, CRLs and OCSP responses, when not already present in the signature, shall
        // be stored in a document security store (DSS)". So a verifier needs whichever the
        // producer used — the caller supplies the store, and the signature carries its own.
        let mut material = material.clone();
        material.absorb(crate::revocation::archived(&cms));
        let Some(signer) = signer_certificate(&cms) else {
            return Trust::NoPathToAnyAnchor { examined: 0 };
        };
        let Ok(target) = x509::read(signer) else {
            return Trust::NoPathToAnyAnchor { examined: 0 };
        };
        // Every certificate the object carries but the signer's own. A path may not hold one
        // certificate twice (RFC 5280 section 6.1) and the target is already in it, so offering it
        // again as a candidate issuer would only make the search reject it a second time.
        let others: Vec<_> = cms
            .certificates
            .iter()
            .filter_map(|entry| x509::read(*entry).ok())
            .filter(|candidate| candidate.tbs != target.tbs)
            .collect();
        trust::validate_for(&target, &others, anchors, &material, at, purpose)
    }

    /// §12.8.3.2's path: the target is `/Cert`'s first entry and the rest of the entry is the pool.
    ///
    /// §12.8.3.2: "The certificate chain of the signer shall be stored in the Cert entry", and
    /// [`Self::pkcs1_authenticity`] already takes the first of them as the signer's — so the two
    /// agree about which certificate the signature was verified under, which is the one thing a
    /// path from it has to be a path *from*.
    ///
    /// No revocation material comes out of the signature here, because there is no CMS object to
    /// carry §12.8.3.3.2's attribute; §12.8.4's store is the only supply, and it is the caller's.
    fn chain_trust(
        &self,
        anchors: &TrustAnchors<'_>,
        material: &crate::revocation::Material<'_>,
        at: Instant,
        purpose: trust::Purpose,
    ) -> Trust {
        let Some(first) = self.chain.first() else {
            return Trust::NoPathToAnyAnchor { examined: 0 };
        };
        let Ok(target) = x509::parse(first) else {
            return Trust::NoPathToAnyAnchor { examined: 0 };
        };
        let others: Vec<_> = self
            .chain
            .iter()
            .skip(1)
            .filter_map(|entry| x509::parse(entry).ok())
            .filter(|candidate| candidate.tbs != target.tbs)
            .collect();
        trust::validate_for(&target, &others, anchors, material, at, purpose)
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
    pub fn integrity(&self, file: &FileBytes) -> Integrity {
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
        // The one read of a signed document's bytes that nothing can avoid, a window at a time.
        let computed = match self.signed_digests(file, &[digest]) {
            Ok(mut digests) => digests.pop().unwrap_or_default(),
            Err(RangeProblem::NotInThisFile) => return Integrity::RangeNotInThisFile,
            Err(RangeProblem::NotReadable) => return Integrity::RangeNotReadable,
        };
        if computed == recorded {
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
    /// 2. where the signer signed a statement about *which* certificate that is, checks it —
    ///    §12.8.3.4.5 (a)'s first sentence, over RFC 5035's signing-certificate attributes
    ///    ([`signing_certificate_bindings`]). It is here rather than at the end because the step
    ///    is decisive: a hash that does not match makes the signature invalid whatever the
    ///    arithmetic below would have said, and one this program cannot compute is a refusal;
    /// 3. reads its `subjectPublicKeyInfo` ([`crate::x509`]);
    /// 4. digests whatever RFC 5652 section 5.4 says the signature is over ([`Signed`]);
    /// 5. verifies with the construction the `signatureAlgorithm` states — RFC 8017 section
    ///    8.2.2's encode-and-compare ([`crate::pkcs1`]), its section 9.1.2's `EMSA-PSS-VERIFY`
    ///    ([`crate::pss`]), FIPS 186-4 section 4.7 ([`crate::dsa`]), ANSI X9.62's over the curve
    ///    RFC 5480's `namedCurve` states ([`crate::ecdsa`]), or RFC 8032's over the message itself
    ///    ([`crate::eddsa`]).
    ///
    /// **Step 5 listed the first three alone until the seven-hundred-and-fifth session**, four
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
    pub fn authenticity(&self, file: &FileBytes) -> Authenticity {
        if self.contents.is_empty() {
            return Authenticity::NoSignatureValue;
        }
        // §12.8.3.2: "For signing PDF files using PKCS #1, the only value of SubFilter that should
        // be used is adbe.x509.rsa_sha1 … The certificate chain of the signer shall be stored in
        // the Cert entry." So there is no CMS object to read: `/Contents` is the signature and
        // `/Cert` is the key.
        if self.sub_filter.as_deref() == Some("adbe.x509.rsa_sha1") {
            return self.pkcs1_authenticity(file);
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
        // §12.8.3.4.5 (a)'s first half, before the second: the step's two sentences are in this
        // order — compare the certificate against the hash the signer signed, *then* "use the
        // public key contained in the signer's certificate to verify that the document digest
        // found in the signature is correctly signed" — and the order is the point. Verifying
        // first and comparing after would mean a `Verified` existed for a moment over a
        // certificate the signer never named.
        for binding in signing_certificate_bindings(&cms) {
            match binding {
                SigningCertificateBinding::Matches { .. } => {}
                SigningCertificateBinding::Differs { version, digest } => {
                    return Authenticity::SigningCertificateMismatch { version, digest };
                }
                SigningCertificateBinding::Unreadable { version, error } => {
                    return Authenticity::SigningCertificateUnverifiable {
                        version,
                        statement: error.to_string(),
                    };
                }
                SigningCertificateBinding::CertificateNotDer { version } => {
                    return Authenticity::SigningCertificateUnverifiable {
                        version,
                        statement: "the signer's certificate is not written in DER, so the octets \
                                    RFC 5035 section 5.4.1.1 hashes are not the file's own"
                            .to_owned(),
                    };
                }
                // Unreachable by construction — `signer_certificate` answered above — and
                // reported rather than ignored, because the two call sites could drift apart.
                SigningCertificateBinding::NoSignerCertificate { .. } => {
                    return Authenticity::NoSignerCertificate {
                        certificates: cms.certificates.len(),
                    };
                }
            }
        }
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
        // RFC 5652 section 5.3's one DER region inside a BER structure, checked before it is
        // digested: an indefinite length among the attributes means the bytes the signer signed
        // are not the bytes this file holds, and the honest answer is a refusal by name rather
        // than a digest over the wrong octets. See [`Authenticity::SignedAttributesNotDer`].
        if let Some(contents) = cms.signed_attributes {
            match der::every_length_is_definite(contents) {
                Ok(true) => {}
                Ok(false) => return Authenticity::SignedAttributesNotDer,
                Err(error) => return Authenticity::Unreadable(CmsError::from(error)),
            }
        }
        let attributes = cms.signed_attributes_encoding();
        let over = match (&attributes, cms.encapsulated) {
            (Some(_), _) => Signed::SignedAttributes,
            (None, Some(_)) => Signed::EncapsulatedContent,
            (None, None) => Signed::TheDocumentsBytes,
        };
        // What is in memory is digested in memory, and the document's own bytes are digested
        // off the file a window at a time (ADR 0812): the signed attributes and the encapsulated
        // content are a few hundred bytes of the CMS object already read, and the third case is
        // every signed byte of the document, which is what `signed_digests` exists not to hold.
        let in_memory: Option<&[u8]> = match (&attributes, cms.encapsulated) {
            (Some(attributes), _) => Some(attributes.as_slice()),
            (None, Some(content)) => Some(content),
            (None, None) => None,
        };
        let compute = |algorithm: Digest| -> Result<Vec<u8>, Authenticity> {
            match in_memory {
                Some(bytes) => Ok(algorithm.compute(&[bytes])),
                None => self
                    .signed_digests(file, &[algorithm])
                    .map(|mut digests| digests.pop().unwrap_or_default())
                    .map_err(Authenticity::from),
            }
        };
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
                let computed = match compute(digest) {
                    Ok(computed) => computed,
                    Err(answer) => return answer,
                };
                (
                    digest,
                    Family::Rsa,
                    pkcs1::verify(key, cms.signature, digest, &computed)
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
                let computed = match compute(parameters.hash) {
                    Ok(computed) => computed,
                    Err(answer) => return answer,
                };
                (
                    parameters.hash,
                    Family::RsaPss,
                    pss::verify(key, cms.signature, parameters, &computed)
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
                let computed = match compute(digest) {
                    Ok(computed) => computed,
                    Err(answer) => return answer,
                };
                (
                    digest,
                    Family::Dsa,
                    dsa::verify(key, cms.signature, &computed)
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
                let computed = match compute(digest) {
                    Ok(computed) => computed,
                    Err(answer) => return answer,
                };
                (
                    digest,
                    Family::Ecdsa(key.curve),
                    ecdsa::verify(key, cms.signature, &computed)
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
                // The one construction that takes the message rather than a digest of it, so
                // the one place the signed bytes are held whole: RFC 8032 hashes `R ‖ A ‖ M`
                // inside the verification, and a signature over the document's own bytes with
                // no signed attributes puts the whole document in `M`.
                let resident;
                let parts: Vec<&[u8]> = if let Some(bytes) = in_memory {
                    vec![bytes]
                } else {
                    resident = match self.signed_bytes(file) {
                        Ok(pieces) => pieces,
                        Err(problem) => return Authenticity::from(problem),
                    };
                    resident.iter().map(AsRef::as_ref).collect()
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
    fn pkcs1_authenticity(&self, file: &FileBytes) -> Authenticity {
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
        // All six in one pass over the signed bytes, because the file is read once for them
        // rather than six times.
        let computed = match self.signed_digests(file, &Digest::TRIED_WHEN_UNSTATED) {
            Ok(computed) => computed,
            Err(problem) => return Authenticity::from(problem),
        };
        let mut refusal = None;
        for (digest, computed) in Digest::TRIED_WHEN_UNSTATED.into_iter().zip(&computed) {
            match pkcs1::verify(key, value, digest, computed) {
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
    /// Everything §12.8.3.4 states that is decidable from the file alone: §12.8.3.4.2's three
    /// constraints, and §12.8.3.4.3's (a), (d), (e), (f), (h) and all four of (i) — (g) is
    /// §12.8.3.4.2's third bullet stated a second time, and is answered by
    /// [`PadesDeparture::BothSigningTimesStated`]. What is not here is
    /// §12.8.3.4.5's validation, of which the half that needs no trust store is
    /// [`signing_certificate_bindings`] and the rest is certification paths and a network.
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
        // (f): one of the two, and among the *signed* attributes. `signing_certificate` and
        // `signing_certificate_v2` are populated from `signedAttrs` alone, which is RFC 5035
        // sections 5.4.1 and 5.4.2's own restriction and not an addition to the clause.
        if cms.signing_certificate.is_none() && cms.signing_certificate_v2.is_none() {
            out.push(PadesDeparture::NoSigningCertificateAttribute);
        }
        // (h): the attribute and the entry are each permitted alone, and the pair is not.
        if self.location.is_some() && cms.has_signed_attribute(cms::ID_AA_ETS_SIGNER_LOCATION) {
            out.push(PadesDeparture::SignerLocationAndLocationEntry);
        }
        // (i)'s four, each searched in both sets for the reason `PadesDeparture::CounterSignature`
        // records.
        for (oid, departure) in [
            (cms::ID_COUNTERSIGNATURE, PadesDeparture::CounterSignature),
            (
                cms::ID_AA_CONTENT_REFERENCE,
                PadesDeparture::ContentReference,
            ),
            (
                cms::ID_AA_CONTENT_IDENTIFIER,
                PadesDeparture::ContentIdentifier,
            ),
            (cms::ID_AA_CONTENT_HINT, PadesDeparture::ContentHints),
        ] {
            if cms.has_signed_attribute(oid) || cms.has_unsigned_attribute(oid) {
                out.push(departure);
            }
        }
        out
    }
}

/// What one of §12.8.3.4.3 (f)'s attributes says about the certificate a signature verified under.
///
/// §12.8.3.4.5 (a)'s first half, as an answer rather than as a boolean: the step is decisive in one
/// direction — "[i]f the hashes do not match, then the signature is considered invalid" — and
/// silent in the other, since a matching hash says the signer meant this certificate and says
/// nothing about whether anyone should trust it. That is question 3, and this crate does not answer
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigningCertificateBinding {
    /// The hash of the signer's certificate is the hash the attribute states.
    Matches {
        /// Which of RFC 5035's two attributes stated it.
        version: ess::Version,
        /// The function the comparison was made under.
        digest: Digest,
    },
    /// It is not, which §12.8.3.4.5 (a) and RFC 5035 section 5.4.1 both call invalid.
    Differs {
        /// Which attribute stated it.
        version: ess::Version,
        /// The function the comparison was made under.
        digest: Digest,
    },
    /// The attribute would not read, so no comparison was made.
    Unreadable {
        /// Which attribute.
        version: ess::Version,
        /// What stopped it.
        error: EssError,
    },
    /// The certificate's encoding is not DER, so "the entire DER-encoded certificate" has no
    /// unambiguous octets to hash.
    ///
    /// RFC 5035 section 5.4.1.1 fixes what is hashed — "computed over the entire DER-encoded
    /// certificate (including the signature)" — and X.690 clause 10.1 admits only the definite
    /// length form in DER. [`crate::der`] accepts the indefinite form anyway, because §12.8.3.3
    /// signature values in the corpus use it; a certificate written that way could be re-encoded
    /// here, and re-encoding is choosing octets on the producer's behalf and then hashing the
    /// choice. Refused instead.
    CertificateNotDer {
        /// Which attribute went unchecked because of it.
        version: ess::Version,
    },
    /// No certificate in the object answers to the signer, so there was nothing to hash.
    ///
    /// Separate from [`Authenticity::NoSignerCertificate`] because it is reached by a different
    /// route: [`signing_certificate_bindings`] is answerable on its own, and a caller asking it
    /// directly gets this rather than an empty list that reads as "no attribute".
    NoSignerCertificate {
        /// Which attribute went unchecked because of it.
        version: ess::Version,
    },
}

impl SigningCertificateBinding {
    /// Which attribute this answer is about.
    #[must_use]
    pub fn version(&self) -> ess::Version {
        match *self {
            Self::Matches { version, .. }
            | Self::Differs { version, .. }
            | Self::Unreadable { version, .. }
            | Self::CertificateNotDer { version }
            | Self::NoSignerCertificate { version } => version,
        }
    }

    /// Whether the comparison was made and came out equal — the only answer that is not a problem.
    #[must_use]
    pub fn is_match(&self) -> bool {
        matches!(*self, Self::Matches { .. })
    }
}

/// §12.8.3.4.5 (a)'s first half, for each signing-certificate attribute the signer states.
///
/// **Empty where the signer states neither**, which is a fact about the file rather than a pass:
/// §12.8.3.4.3 (f) requires one on a `PAdES` signature and
/// [`PadesDeparture::NoSigningCertificateAttribute`] is where that requirement is stated. For any
/// other `/SubFilter` ISO 32000-2 asks for no such attribute, so its absence is nothing at all.
///
/// **One entry per attribute present, because RFC 5035 section 5.4 says they are two answers**:
/// "[i]f both attributes exist in a single message, they are independently evaluated." A caller
/// that wants a verdict wants all of them to be [`SigningCertificateBinding::Matches`].
///
/// Only *signed* attributes are read, which is RFC 5035 section 5.4.1's own rule: "[i]f present,
/// the SigningCertificateV2 attribute MUST be a signed attribute; it MUST NOT be an unsigned
/// attribute." An unsigned one commits the signer to nothing and acting on it would let whoever
/// appended it decide which certificate a signature is judged against.
#[must_use]
pub fn signing_certificate_bindings(cms: &SignedData<'_>) -> Vec<SigningCertificateBinding> {
    let stated = [
        (ess::Version::One, cms.signing_certificate),
        (ess::Version::Two, cms.signing_certificate_v2),
    ];
    let mut out = Vec::new();
    for (version, value) in stated {
        let Some(value) = value else { continue };
        let stated = match version {
            ess::Version::One => ess::signing_certificate(value),
            ess::Version::Two => ess::signing_certificate_v2(value),
        };
        let stated = match stated {
            Ok(stated) => stated,
            Err(error) => {
                out.push(SigningCertificateBinding::Unreadable { version, error });
                continue;
            }
        };
        let Some(certificate) = signer_certificate(cms) else {
            out.push(SigningCertificateBinding::NoSignerCertificate { version });
            continue;
        };
        if certificate.had_indefinite_length() {
            out.push(SigningCertificateBinding::CertificateNotDer { version });
            continue;
        }
        let computed = stated.digest.compute(&[certificate.encoding()]);
        let digest = stated.digest;
        out.push(if computed == stated.hash {
            SigningCertificateBinding::Matches { version, digest }
        } else {
            SigningCertificateBinding::Differs { version, digest }
        });
    }
    out
}

/// The certificate a `SignerInfo` names, among the ones the CMS object carries.
///
/// RFC 5652's `SignerIdentifier` is a choice of two, and both are honoured: the issuer-and-serial
/// pair that every corpus signature uses, and the `subjectKeyIdentifier` that a version 3
/// `SignerInfo` may use instead. A signature naming neither, or naming one no certificate answers
/// to, yields `None` — deliberately rather than falling back to "the only certificate present",
/// which would verify against a key the signature never claimed.
fn signer_certificate<'a>(cms: &SignedData<'a>) -> Option<der::Value<'a>> {
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
pub(crate) fn name(oid: &[u8]) -> String {
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
/// The number always, because it is what a reader can check; and ISO/TS 32002 Table 3's own spelling beside it
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
    seen: &mut std::collections::BTreeSet<ObjectId>,
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
        format_version: document.get_key(dict, "V").as_integer(),
        indirect_values: indirect_values(dict),
        reference_digests: reference_digests(document, dict),
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
/// invalidates a signature — and that difference lives in `pdf_model::restriction::Restriction`,
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
    /// `xfa_filled_imm1344e.pdf` states `form1[0].SignatureField3[0]`, and `pdf_model::form::fields`
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
    seen: &mut std::collections::BTreeSet<ObjectId>,
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
///
/// The entry is an *integer* — Errata Collection 3 (Issue #152) rewrites Table 257's type cell
/// from `number` to `integer`, which is the type this function has always asked for. A `/P`
/// written as a real is therefore a malformed statement of a level, and it takes the table's
/// default rather than its numeric value: a restriction is obeyed as the file states it, and a
/// value the amended table does not admit restricts no further than an absent one — the same
/// stance as reading a level outside 1..=3 as [`Modification::Unknown`], which permits.
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

/// What each of the signature's `/Reference` dictionaries states as its Table 256 `/DigestMethod`.
///
/// **The whole array, in the file's order**, for [`field_mdp`]'s reason: §12.8.2.1 makes
/// `/Reference` plural - "[t]ransform methods, along with transform parameters, shall determine
/// which objects are included and excluded in revision comparison" - and the corpus's one
/// certification signature states two of them, a `DocMDP` and a `FieldMDP`. Each may name its own
/// digest, so one answer per dictionary is the only shape that does not lose which named what.
///
/// Bounded by [`MAX_INDIRECT_VALUES`], which is a count of dictionary entries and serves here for
/// the same reason it serves there: the array's length comes out of the file.
fn reference_digests(document: &Document, signature: &Dictionary) -> Vec<ReferenceDigest> {
    let references = document.get_key(signature, "Reference");
    let Some(references) = references.as_array().map(<[Object]>::to_vec) else {
        return Vec::new();
    };
    references
        .iter()
        .take(MAX_INDIRECT_VALUES)
        .filter_map(|reference| {
            let resolved = document.resolve(reference);
            let reference = resolved.as_dict()?;
            Some(
                match document.get_key(reference, "DigestMethod").as_name() {
                    None => ReferenceDigest::Absent,
                    Some(name) => match Digest::from_pdf_name(name.as_bytes()) {
                        Some(digest) => ReferenceDigest::Stated(digest),
                        None => ReferenceDigest::NotInTheTable(
                            String::from_utf8_lossy(name.as_bytes()).into_owned(),
                        ),
                    },
                },
            )
        })
        .collect()
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

/// Reads a region of a file as §7.3.4.3's hexadecimal string, comparing it with a known value.
///
/// A scanner rather than a decode-and-compare, because the region's size comes out of the file:
/// the octets are checked as they are decoded and the state is five fields, so
/// [`Signature::excluded`] holds one window of a region however large the file says it is.
struct HexScan<'a> {
    /// The octets the region has to decode to — `/Contents`, as the parser read it.
    value: &'a [u8],
    /// How many of them have been matched so far.
    matched: usize,
    /// The high nibble of an octet whose low nibble has not arrived.
    pending: Option<u8>,
    /// Whether a LESS-THAN SIGN has been seen. §7.3.4.3 puts it before the first digit.
    opened: bool,
    /// Whether a GREATER-THAN SIGN has been seen, after which only whitespace may follow.
    closed: bool,
    /// Whether the region has already stopped being this value.
    failed: bool,
}

impl<'a> HexScan<'a> {
    /// A scan of a region that should decode to `value`.
    const fn new(value: &'a [u8]) -> Self {
        Self {
            value,
            matched: 0,
            pending: None,
            opened: false,
            closed: false,
            failed: false,
        }
    }

    /// Takes the next window of the region.
    fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.failed {
                return;
            }
            self.one(byte);
        }
    }

    /// Takes one byte of it.
    fn one(&mut self, byte: u8) {
        // §7.3.4.3: "White-space characters (see ' Table 1 -White-space characters ' ) shall be
        // ignored." Admitted anywhere in the region and not only inside the brackets, because
        // whitespace outside them is a token separator that carries no object either.
        if pdf_syntax::lexer::is_whitespace(byte) {
            return;
        }
        if self.closed {
            self.failed = true;
            return;
        }
        match byte {
            // §7.3.4.3 encloses the digits "within angle brackets"; whether the range excludes
            // them with the digits or signs them is the producer's, and both occur.
            b'<' if !self.opened && self.matched == 0 && self.pending.is_none() => {
                self.opened = true;
            }
            b'>' => self.closed = true,
            _ => match (hex_digit(byte), self.pending.take()) {
                (None, _) => self.failed = true,
                (Some(nibble), None) => self.pending = Some(nibble),
                (Some(nibble), Some(high)) => self.octet((high << 4) | nibble),
            },
        }
    }

    /// Checks one decoded octet against the value.
    fn octet(&mut self, octet: u8) {
        if self.value.get(self.matched) == Some(&octet) {
            self.matched = self.matched.saturating_add(1);
        } else {
            self.failed = true;
        }
    }

    /// Whether the region was the value and nothing else, and whether it carried both delimiters.
    ///
    /// `None` where the region is not the value. `Some(true)` where it is the value written as
    /// §12.8.3.3.1 requires, and `Some(false)` where a delimiter was left inside the signed range.
    fn finish(mut self) -> Option<bool> {
        // §7.3.4.3: "If the final digit of a hexadecimal string is missing -that is, if there is
        // an odd number of digits -the final digit shall be assumed to be 0."
        if let Some(high) = self.pending.take() {
            self.octet(high << 4);
        }
        if self.failed || self.matched != self.value.len() {
            return None;
        }
        Some(self.opened && self.closed)
    }
}

/// One of §7.3.4.3's hexadecimal digits, "0 -9 and A -F or a -f", as its value.
const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte.wrapping_sub(b'0')),
        b'a'..=b'f' => Some(byte.wrapping_sub(b'a').wrapping_add(10)),
        b'A'..=b'F' => Some(byte.wrapping_sub(b'A').wrapping_add(10)),
        _ => None,
    }
}

/// The keys of a signature dictionary whose values are indirect references (§12.8.1).
///
/// Read from the dictionary rather than through `Document::get_key`, which is the whole point:
/// resolving a reference is exactly what hides the departure. Sorted and bounded, for the reasons
/// [`Signature::indirect_values`] gives.
fn indirect_values(dict: &Dictionary) -> Vec<String> {
    let mut keys: Vec<String> = dict
        .iter()
        .filter(|(_, value)| matches!(value, Object::Reference(_)))
        .map(|(name, _)| String::from_utf8_lossy(name.as_bytes()).into_owned())
        .collect();
    keys.sort();
    keys.truncate(MAX_INDIRECT_VALUES);
    keys
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
/// **This used to be four counts**, on the argument that "a certificate here would be read to
/// validate a certification path, which is question 3, and reading the bytes is the smallest part
/// of that". The path validation arrived in the thousand-and-twenty-second session
/// ([`crate::trust`], ADR 1039) and the revocation step in the thousand-and-fifty-third (ADR 1067),
/// so the smallest part is now the part that was missing. The counts are still available, as
/// lengths.
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
    /// The clause is explicit that a VRI records only successes: "[a] signature VRI dictionary
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
    /// `/TU`, "[t]he date/time at which this signature VRI dictionary was created", as the date
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
    /// VRI's arrays are a subset — "[e]ach stream shall reference a CRL that is an entry in the
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
        let object = document.get(ObjectId {
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
        Authenticity, Coverage, Excluded, Family, Integrity, Modification, PadesDeparture,
        ReferenceDigest, Signature, Signed, SignedEnd, SigningCertificateBinding, ess, legal,
        permissions, security_store, signatures, signing_certificate_bindings,
    };
    use crate::cms::{Digest, fixtures};
    use crate::x509::fixtures::{CERTIFICATE, EC_CERTIFICATE, PKCS1_SIGNATURE, hex};
    use pdf_syntax::{Document, FileBytes};

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

    /// A `/P` written as a real takes Table 257's default instead of being read as its value.
    ///
    /// Errata Collection 3 (Issue #152) makes the entry's type *integer* — the published cell
    /// said `number` — so `1.0` is not a level the amended table admits. Reading it numerically
    /// would let a malformed value restrict harder than the default; reading it as absent is the
    /// same stance `Modification::Unknown` takes for an integer outside 1..=3. Only this test
    /// separates `as_integer` from a numeric read: every other fixture writes the level as the
    /// integer it is.
    #[test]
    fn a_docmdp_level_written_as_a_real_takes_the_tables_default() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> \
             /Perms << /DocMDP 5 0 R >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 840 960 240] /Contents <00> /Reference [6 0 R] >>",
            "<< /Type /SigRef /TransformMethod /DocMDP /TransformParams << /Type /TransformParams /P 1.0 >> >>",
        ]);

        assert_eq!(
            permissions(&doc).doc_mdp,
            Some(Modification::FormFilling),
            "a real is not Table 257's integer, so the default level 2 stands"
        );
    }

    /// Table 256's `/DigestMethod`, in each of the three states the entry can be in.
    ///
    /// The table prints its own value list - "Valid values are MD5, SHA1 SHA256, SHA384, SHA512
    /// and RIPEMD160" - so one signature names one of the six, one names something else, and one
    /// names nothing, which Errata Collection 3's issue #117 makes conforming by striking
    /// "(Required)" out. Three answers because the middle case is a departure and the third is
    /// not, and an `Option` could not tell them apart.
    ///
    /// The first signature states **two** reference dictionaries, which is the shape the corpus's
    /// one certification signature has: §12.8.2.1 makes `/Reference` plural and each dictionary
    /// carries its own digest, so a reader taking the first entry would lose the second's.
    #[test]
    fn a_reference_dictionary_says_which_digest_its_modification_analysis_uses() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 6 0 R 9 0 R] \
             /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Named) /V 5 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 10 20 10] /Contents <00> /Reference [8 0 R 11 0 R] >>",
            "<< /FT /Sig /T (Outside) /V 7 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 10 20 10] /Contents <00> /Reference [10 0 R] >>",
            "<< /Type /SigRef /TransformMethod /DocMDP /DigestMethod /SHA384 >>",
            "<< /FT /Sig /T (Silent) /V 12 0 R >>",
            "<< /Type /SigRef /TransformMethod /DocMDP /DigestMethod /SHA2 >>",
            "<< /Type /SigRef /TransformMethod /FieldMDP /Data 4 0 R /DigestMethod /RIPEMD160 >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 10 20 10] /Contents <00> /Reference [13 0 R] >>",
            "<< /Type /SigRef /TransformMethod /DocMDP >>",
        ]);

        let found = signatures(&doc);
        let [named, outside, silent] = found.as_slice() else {
            panic!("three signatures, one per state of the entry: {found:?}");
        };
        assert_eq!(
            named.reference_digests,
            vec![
                ReferenceDigest::Stated(Digest::Sha384),
                ReferenceDigest::Stated(Digest::Ripemd160),
            ],
            "both reference dictionaries, in the order §12.8.2.1's array states them"
        );
        assert_eq!(
            outside.reference_digests,
            vec![ReferenceDigest::NotInTheTable("SHA2".to_owned())],
            "a name the entry's value list does not admit is carried rather than guessed at"
        );
        assert_eq!(
            silent.reference_digests,
            vec![ReferenceDigest::Absent],
            "the control: a reference dictionary stating no /DigestMethod at all"
        );
    }

    /// Table 255's `/V 1` says the reference dictionary is critical, and the field's `/V` does not.
    ///
    /// **Two entries one letter apart, and the outer one is what a signature is reached through.**
    /// Table 226 makes a field's `/V` its value — for a signature field, the signature dictionary
    /// itself — while Table 255 makes the *signature dictionary's* `/V` an integer format version
    /// whose 1 means "the Reference dictionary shall be considered critical to the validation of
    /// the signature" (§12.8.1). Reading the wrong one would answer with a dictionary where an
    /// integer belongs, so this fixture states both: `/V 1` inside a signature that is itself the
    /// `/V` of its field.
    ///
    /// The second field is the control the entry's own default asks for — Table 255 gives `/V` a
    /// default of 0, so a signature stating nothing states nothing about criticality.
    #[test]
    fn a_signature_states_whether_its_reference_dictionary_is_critical() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 6 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Critical) /V 5 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /V 1 \
             /ByteRange [0 10 20 10] /Contents <00> /Reference [8 0 R] >>",
            "<< /FT /Sig /T (Silent) /V 7 0 R >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 10 20 10] /Contents <00> /Reference [8 0 R] >>",
            "<< /Type /SigRef /TransformMethod /FieldMDP /Data 4 0 R \
             /TransformParams << /Type /TransformParams /Action /All /V /1.2 >> >>",
        ]);

        let found = signatures(&doc);
        let [critical, silent] = found.as_slice() else {
            panic!("two signatures, got {found:?}");
        };
        assert_eq!(
            critical.format_version,
            Some(1),
            "the signature dictionary's own /V, not the field's"
        );
        assert!(critical.reference_is_critical());
        assert_eq!(silent.format_version, None);
        assert!(
            !silent.reference_is_critical(),
            "Table 255 defaults /V to 0, which says nothing about criticality"
        );
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
        assert_eq!(store.certificates.len(), 2);
        assert_eq!(store.revocation_lists.len(), 0);
        assert_eq!(store.ocsp_responses.len(), 1);
        assert_eq!(store.validation_information.len(), 1);
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
        signed_document_with_hole(sub_filter, extra, digest, sign, |value| value)
    }

    /// [`signed_document`] with the hole in `/ByteRange` placed by the caller.
    ///
    /// `hole` is handed the half-open range of the signature value as the writer laid it out —
    /// the LESS-THAN SIGN, the digits, the GREATER-THAN SIGN — and returns the range the
    /// `/ByteRange` is to leave out. Everything downstream follows from what it returns: the pairs
    /// name the rest of the file and the digest is taken over exactly those pairs, so a hole put
    /// anywhere still produces a **self-consistent** signed document. That is what makes it a
    /// planted defect worth having (trap 13): a file whose hole swallows the bytes after the value
    /// answers [`Coverage::WholeFile`] and [`Integrity::Unchanged`] and verifies under its own
    /// key, and the only thing in this module that can see it is [`Signature::excluded`].
    fn signed_document_with_hole(
        sub_filter: &str,
        extra: &str,
        digest: Digest,
        sign: impl Fn(&[u8]) -> Vec<u8>,
        hole: impl Fn(std::ops::Range<usize>) -> std::ops::Range<usize>,
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
        // §12.8.1's range: everything up to the hole, and everything after it. Where the hole is
        // belongs to the caller, so that a test can plant one that is not the value.
        let excluded = hole(open..open.saturating_add(ROOM).saturating_add(2));
        let (before, after) = (excluded.start, excluded.end);
        let tail = bytes.len().saturating_sub(after);
        let placeholder = b"[0000000000 0000000000 0000000000 0000000000]";
        let range = format!("[{:010} {before:010} {after:010} {tail:010}]", 0);
        assert_eq!(range.len(), placeholder.len());
        let at = bytes
            .windows(placeholder.len())
            .position(|window| window == placeholder)
            .expect("the /ByteRange hole");
        bytes.splice(at..at.saturating_add(placeholder.len()), range.bytes());

        let value = sign(&digest.compute(&[&bytes[..before], &bytes[after..]]));
        let hex = value.iter().fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        });
        assert!(hex.len() <= ROOM, "the signature value fits its hole");
        let value_at = open.saturating_add(1);
        bytes.splice(value_at..value_at.saturating_add(hex.len()), hex.bytes());
        bytes
    }

    /// **What a signed range leaves out, and the forgery that makes the question worth asking.**
    ///
    /// §12.8.1 permits one thing in the region a `/ByteRange` skips — "the signature value itself
    /// (the Contents entry)" — and Table 255 repeats it as a `shall`. Three files here, each
    /// signed by the same builder and each internally consistent:
    ///
    /// 1. the ordinary shape, where the excluded region is `<` digits `>`;
    /// 2. `signed_verified.pdf`'s shape, where the delimiters are signed and only the digits are
    ///    skipped — both occur in the corpus, neither hides a byte carrying an object, and the
    ///    second departs from §12.8.3.3.1's "shall fit precisely" while doing so;
    /// 3. **a region that swallows the sixteen bytes after the value**, which close the signature
    ///    dictionary and begin what follows it. A reader parses every one of them and no digest
    ///    was taken over any of them.
    ///
    /// The third is the calibration (trap 13) and the three assertions on it are the whole point
    /// of this round: its pairs still cover the file, its digest still recomputes to what the
    /// signature recorded — so [`Coverage`] and [`Integrity`] both say what they say about an
    /// honest file — and [`Signature::excluded`] is the only thing here that can see the region.
    #[test]
    fn a_signed_range_leaves_out_the_signature_value_and_nothing_else() {
        /// How many bytes past the value the planted region swallows.
        const HIDDEN: usize = 16;
        let honest = signed_document(
            "adbe.pkcs7.detached",
            "",
            Digest::Sha256,
            fixtures::detached,
        );
        let (document, signature) = only_signature(&honest);
        assert_eq!(
            signature.excluded(document.bytes()),
            Excluded::TheSignatureValue,
            "the region between the pairs is the value, brackets and all"
        );

        let digits_only = signed_document_with_hole(
            "adbe.pkcs7.detached",
            "",
            Digest::Sha256,
            fixtures::detached,
            |value| value.start.saturating_add(1)..value.end.saturating_sub(1),
        );
        let (document, signature) = only_signature(&digits_only);
        assert_eq!(
            signature.excluded(document.bytes()),
            Excluded::TheDigitsOfTheSignatureValue,
            "a producer that signs the angle brackets has excluded only the value, and §12.8.3.3.1 \
             still wanted the string to fit precisely between the ranges"
        );

        let wrapped = signed_document_with_hole(
            "adbe.pkcs7.detached",
            "",
            Digest::Sha256,
            fixtures::detached,
            |value| value.start..value.end.saturating_add(HIDDEN),
        );
        let (document, signature) = only_signature(&wrapped);
        let length = u64::try_from(wrapped.len()).expect("a test file");
        assert_eq!(
            signature.coverage(length),
            Coverage::WholeFile,
            "the pairs tile the file, which is all coverage can see"
        );
        assert_eq!(
            signature.integrity(document.bytes()),
            Integrity::Unchanged {
                digest: Digest::Sha256
            },
            "and the digest over those pairs is the one the signature recorded"
        );
        match signature.excluded(document.bytes()) {
            Excluded::NotTheSignatureValue { length, .. } => assert_eq!(
                usize::try_from(length).expect("a test file"),
                signature
                    .contents
                    .len()
                    .saturating_mul(2)
                    .saturating_add(2)
                    .saturating_add(HIDDEN),
                "the region is the value's hexadecimal string and the bytes after it"
            ),
            other => {
                panic!("a region holding {HIDDEN} bytes of the file is not the value: {other:?}")
            }
        }
    }

    /// The other two shapes a region can take: more than one of them, and none at all.
    ///
    /// Both are read off the pairs rather than off the file, so they are planted on a real
    /// signature's `/ByteRange` — which is also what keeps them honest as calibration: the
    /// document underneath is the one the test above calls [`Excluded::TheSignatureValue`].
    #[test]
    fn a_signed_range_leaving_out_two_regions_or_none_is_named() {
        let bytes = signed_document(
            "adbe.pkcs7.detached",
            "",
            Digest::Sha256,
            fixtures::detached,
        );
        let (document, signature) = only_signature(&bytes);
        let length = u64::try_from(bytes.len()).expect("a test file");

        let covered = Signature {
            byte_range: vec![(0, length)],
            ..signature.clone()
        };
        assert_eq!(
            covered.excluded(document.bytes()),
            Excluded::Nothing,
            "a single pair leaves nothing out, so the value is inside its own digest"
        );

        let [(_, before), (start, tail)] = signature.byte_range.as_slice() else {
            panic!("two pairs, got {:?}", signature.byte_range);
        };
        let split = Signature {
            byte_range: vec![
                (0, *before),
                (*start, tail.saturating_sub(8)),
                (start.saturating_add(*tail), 0),
            ],
            ..signature.clone()
        };
        assert_eq!(
            split.excluded(document.bytes()),
            Excluded::MoreThanOneRegion { regions: 2 },
            "eight bytes dropped out of the tail are a second region nobody signed"
        );
    }

    /// **Where a signed range may stop**, which §12.8.1 states in the sentence that starts it.
    ///
    /// > In case of multiple digital signatures this range shall be the sequence of bytes starting
    /// > from the "%PDF-" comment at the beginning of the PDF document to the end of the "%%EOF"
    /// > comment, possibly followed by an optional EOL marker, terminating the incremental update
    /// > that adds the digital signature dictionary to the document.
    ///
    /// [`Signature::coverage`] already refuses a range that does not begin at zero. This is the
    /// other end, and the planted defect is three bytes short of the marker — a range that has
    /// signed a prefix of a revision whose remainder the reader goes on to parse.
    #[test]
    fn a_signed_range_ends_at_an_end_of_file_marker_or_says_where_it_does_end() {
        let bytes = signed_document(
            "adbe.pkcs7.detached",
            "",
            Digest::Sha256,
            fixtures::detached,
        );
        let (document, signature) = only_signature(&bytes);
        assert_eq!(
            signature.signed_end(document.bytes()),
            SignedEnd::AtAnEndOfFileMarker,
            "the builder signs to the end of the file, whose last line is %%EOF"
        );

        let [first, (start, tail)] = signature.byte_range.as_slice() else {
            panic!("two pairs, got {:?}", signature.byte_range);
        };
        let short = Signature {
            byte_range: vec![*first, (*start, tail.saturating_sub(3))],
            ..signature.clone()
        };
        assert_eq!(
            short.signed_end(document.bytes()),
            SignedEnd::Elsewhere,
            "three bytes short of the marker is inside the revision rather than at its end"
        );

        let beyond = Signature {
            byte_range: vec![*first, (*start, tail.saturating_add(1))],
            ..signature.clone()
        };
        assert_eq!(
            beyond.signed_end(document.bytes()),
            SignedEnd::RangeNotInThisFile,
            "a range naming a byte the file does not have is refused rather than measured"
        );
    }

    /// **§12.8.1's other rule about a byte range digest, and it is about the dictionary.**
    ///
    /// > When a byte range digest is present, all values in the signature dictionary shall be
    /// > direct objects.
    ///
    /// Worth reading rather than assuming, because resolving a reference is what hides it:
    /// `Document::get_key` gives a `/ByteRange` written as `6 0 R` and one written out in the
    /// dictionary the same way, and the first is an object a later incremental update can redefine
    /// while every byte the digest was taken over stays where it is. Both directions are asserted,
    /// because a list that is always empty has not been shown to look at anything.
    #[test]
    fn a_signature_dictionary_names_the_values_its_file_wrote_indirectly() {
        let indirect = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange 6 0 R /Contents <00> /M 7 0 R >>",
            "[0 840 960 240]",
            "(D:20260801120000+02'00')",
        ]);
        let found = signatures(&indirect);
        let [signature] = found.as_slice() else {
            panic!("one signature, got {found:?}");
        };
        assert_eq!(
            signature.indirect_values,
            vec!["ByteRange".to_owned(), "M".to_owned()],
            "both values the file wrote as references, in the order a report prints them"
        );
        assert_eq!(
            signature.byte_range,
            vec![(0, 840), (960, 240)],
            "and the reference is still followed, which is why the departure needs saying"
        );

        let direct = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Unused true >>",
            "<< /FT /Sig /T (Signature1) /V 5 0 R /Subtype /Widget >>",
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 840 960 240] /Contents <00> /M (D:20260801120000+02'00') >>",
        ]);
        let found = signatures(&direct);
        let [signature] = found.as_slice() else {
            panic!("one signature, got {found:?}");
        };
        assert!(
            signature.indirect_values.is_empty(),
            "the same dictionary written directly names nothing: {:?}",
            signature.indirect_values
        );
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

    /// A signed document answers the same through a file on disk as through its bytes.
    ///
    /// The digest is fed a window at a time off the disk (ADR 0812), and this pins that the
    /// windows add up to the range: the same `Integrity` and `Authenticity`, exact, on a range
    /// wider than one window — the fixture is padded past several of them — and after a byte
    /// inside the range moves. The corpus's real signatures do the same in
    /// `tests/signatures.rs::every_corpus_signature_answers_the_same_on_disk`.
    #[test]
    fn a_signed_document_answers_the_same_on_disk_as_in_memory() {
        let bytes = signed_document(
            "adbe.pkcs7.detached",
            &"% padding so the range spans several windows\n".repeat(8000),
            Digest::Sha256,
            fixtures::detached,
        );
        assert!(bytes.len() > 3 * super::SIGNED_WINDOW);
        let directory = std::env::temp_dir().join(format!(
            "pdf-model-signature-{}-on-disk",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        let path = directory.join("signed.pdf");
        std::fs::write(&path, &bytes).expect("the fixture is written");

        let (in_memory, signature) = only_signature(&bytes);
        let on_disk = Document::open(FileBytes::on_disk(&path).expect("opens")).expect("opens");
        let expected = signature.integrity(in_memory.bytes());
        assert_eq!(
            expected,
            Integrity::Unchanged {
                digest: Digest::Sha256
            }
        );
        assert_eq!(signature.integrity(on_disk.bytes()), expected);
        assert_eq!(
            signature.authenticity(on_disk.bytes()),
            signature.authenticity(in_memory.bytes())
        );
        assert_eq!(
            signature
                .signed_digests(on_disk.bytes(), &Digest::ALL)
                .expect("in the file"),
            signature
                .signed_digests(in_memory.bytes(), &Digest::ALL)
                .expect("in the file"),
            "every algorithm agrees between the two routes"
        );

        // One byte of the range, deep in the padding: both routes see it move.
        let mut altered = bytes.clone();
        let at = altered.len() / 2;
        altered[at] = altered[at].wrapping_add(1);
        std::fs::write(&path, &altered).expect("the altered fixture is written");
        let (altered_memory, altered_signature) = only_signature(&altered);
        let altered_disk =
            Document::open(FileBytes::on_disk(&path).expect("opens")).expect("opens");
        assert_eq!(
            altered_signature.integrity(altered_disk.bytes()),
            Integrity::Changed {
                digest: Digest::Sha256
            }
        );
        assert_eq!(
            altered_signature.integrity(altered_disk.bytes()),
            altered_signature.integrity(altered_memory.bytes())
        );

        // A file that shrinks under the reader is refused by name, never reported as changed:
        // the document was opened at the full length and the file is then cut inside the range.
        let shrunk = Document::open(FileBytes::on_disk(&path).expect("opens")).expect("opens");
        std::fs::write(&path, &altered[..altered.len() / 2]).expect("the file is cut");
        assert_eq!(
            altered_signature.integrity(shrunk.bytes()),
            Integrity::RangeNotReadable
        );
        // This fixture's signature is over its signed attributes, so `authenticity` never reads
        // the range and answers as it did before the cut; the same refusal on the path that does
        // read it — §12.8.3.2's, and a signer with no signed attributes — is `From<RangeProblem>`.
        assert_eq!(
            altered_signature.authenticity(shrunk.bytes()),
            altered_signature.authenticity(altered_memory.bytes())
        );
        assert_eq!(
            altered_signature.signed_digests(shrunk.bytes(), &[Digest::Sha1]),
            Err(super::RangeProblem::NotReadable)
        );
        std::fs::remove_dir_all(&directory).expect("the temporary directory is removable");
    }

    /// What the digest window costs, printed: the committed 19 MB document's whole extent as a
    /// range, through four windows and held whole.
    ///
    /// ```sh
    /// cargo test --release -p pdf-model --lib -- --ignored --nocapture the_window_a_signed_range
    /// ```
    ///
    /// Ignored because it is a measurement and not a check, and a wall clock in a test is a
    /// coin toss under load; what it pins is written in ADR 0812 beside [`super::SIGNED_WINDOW`].
    #[test]
    // not a gate: a measurement of the digest window, printed, not a check (ADR 0812)
    #[ignore = "a measurement, printed: run with --ignored --nocapture"]
    fn the_window_a_signed_range_is_digested_through_is_priced() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
        let on_disk = FileBytes::on_disk(&path).expect("the committed document opens");
        let length = u64::try_from(on_disk.len()).expect("a length");
        let (_, base) = only_signature(&signed_document(
            "adbe.pkcs7.detached",
            "",
            Digest::Sha256,
            fixtures::detached,
        ));
        let signature = Signature {
            byte_range: vec![(0, length / 2), (length / 2, length - length / 2)],
            ..base
        };
        let time = |window: usize| {
            let started = std::time::Instant::now();
            let mut hasher = Digest::Sha256.hasher();
            signature
                .each_signed_window_of(&on_disk, window, |bytes| hasher.update(bytes))
                .expect("in the file");
            (hasher.finish(), started.elapsed())
        };
        let (whole, whole_took) = time(on_disk.len());
        for window in [4 * 1024, 64 * 1024, 1024 * 1024, 16 * 1024 * 1024] {
            let (digest, took) = time(window);
            assert_eq!(digest, whole);
            println!(
                "window {:>9} B: {:>7.2} ms  (whole range held at once: {:.2} ms)",
                window,
                took.as_secs_f64() * 1e3,
                whole_took.as_secs_f64() * 1e3
            );
        }
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
            signature.authenticity(&FileBytes::from(file)),
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
            signature.authenticity(&FileBytes::from(b"the signed byteS")),
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
            signature.authenticity(&FileBytes::from(file)),
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
            signature.authenticity(&FileBytes::from(file)),
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

    /// **§12.8.3.2 puts the chain in the dictionary, so a path is built from there or from nowhere.**
    ///
    /// The clause states it outright — "[t]he certificate chain of the signer shall be stored in
    /// the Cert entry" — and this `/SubFilter`'s `/Contents` is a PKCS #1 signature with no CMS
    /// object in it at all, so the route every other signature format takes finds nothing. Until
    /// the one-thousand-and-sixty-second session that is what happened: `Signature::trust` looked
    /// for a `SignedData`, failed, and answered `NoPathToAnyAnchor` with nothing examined, which
    /// reads as *this file carries too few certificates* about a file carrying exactly the one the
    /// clause asks for.
    ///
    /// The certificate here is self-signed, so supplying it is supplying the root of a one-long
    /// chain — which is what the negative beside it calibrates: a different root reaches nothing.
    #[test]
    fn a_pkcs1_signatures_path_is_built_from_its_cert_entry() {
        let file = b"the signed bytes";
        let signature = pkcs1_signature(file.len() as u64, hex(CERTIFICATE));
        let anchor = hex(CERTIFICATE);
        let anchor = crate::x509::parse(&anchor).expect("the fixture certificate parses");
        let at = anchor
            .validity
            .expect("the fixture certificate states a validity period")
            .not_before;
        let anchors = crate::trust::TrustAnchors::of(std::slice::from_ref(&anchor));
        assert_eq!(
            signature.trust(&anchors, &crate::revocation::Material::none(), at),
            crate::trust::Trust::Anchored {
                length: 1,
                revocation: crate::revocation::Revocation::NotChecked,
            },
            "the chain the clause puts in /Cert is the pool the path is built from"
        );
        // And the calibration: a root that issued nothing here reaches nothing, so the assertion
        // above is about the path and not about the certificate happening to parse (trap 13).
        let other = hex(crate::trust::tests::fixtures::ROOT);
        let other = crate::x509::parse(&other).expect("the other fixture root parses");
        let others = crate::trust::TrustAnchors::of(std::slice::from_ref(&other));
        assert!(
            matches!(
                signature.trust(&others, &crate::revocation::Material::none(), at),
                crate::trust::Trust::NoPathToAnyAnchor { .. }
            ),
            "somebody else's root ends nothing"
        );
        // And with no anchor it is this program's own answer, unchanged (ADR 1039).
        assert_eq!(
            signature.trust(
                &crate::trust::TrustAnchors::none(),
                &crate::revocation::Material::none(),
                at
            ),
            crate::trust::Trust::NoAnchorSupplied
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
            format_version: None,
            indirect_values: Vec::new(),
            reference_digests: Vec::new(),
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
            format_version: None,
            indirect_values: Vec::new(),
            reference_digests: Vec::new(),
        };
        assert_eq!(
            signature.authenticity(&FileBytes::from(file)),
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
            signature.authenticity(&FileBytes::from(b"the signed byteS")),
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
                signature.authenticity(&FileBytes::from(file)),
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
                signature.authenticity(&FileBytes::from(b"the signed byteS")),
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
            signature.authenticity(&FileBytes::from(file)),
            Authenticity::Verified {
                digest: Digest::Sha512,
                family: Family::EdDsa,
                key_bits: 256,
                over: Signed::TheDocumentsBytes,
            }
        );
        assert_eq!(
            signature.authenticity(&FileBytes::from(b"the signed byteS")),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha512,
                family: Family::EdDsa,
                key_bits: 256,
                over: Signed::TheDocumentsBytes,
            }
        );
    }

    /// RFC 5652's one DER region, planted and controlled.
    ///
    /// Two CMS values differing by exactly one thing: whether a signed attribute's SEQUENCE states
    /// X.690 clause 8.1.3.6's indefinite length. RFC 5652 section 5.3 requires signed attributes
    /// in DER "even if the rest of the structure is BER encoded", and section 5.4 digests "the
    /// complete DER encoding of the SignedAttrs value" - so the planted one cannot be digested
    /// into what the signer signed, and is refused by name.
    ///
    /// **The control is what makes this a test rather than an assertion.** The same value written
    /// in DER reaches the arithmetic and answers [`Authenticity::NotUnderThatKey`]: this DSA
    /// signature was made over the byte range, and adding any signed attribute makes RFC 5652
    /// sign the attributes instead, so a failure to verify is what a correct reader answers there.
    /// If the refusal fired on both, it would be firing on the attributes rather than on their
    /// encoding.
    #[test]
    fn signed_attributes_that_are_not_der_are_refused_rather_than_digested() {
        let file = b"the signed bytes";
        let certificate = hex(crate::dsa::fixtures::CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let value = |indefinite| {
            fixtures::detached_dsa_stating_signing_time(
                &certificate,
                parsed.issuer,
                parsed.serial_number,
                &hex(crate::dsa::fixtures::SIGNATURE),
                indefinite,
            )
        };
        assert_eq!(
            curve_signature(value(true)).authenticity(&FileBytes::from(file)),
            Authenticity::SignedAttributesNotDer,
            "RFC 5652 section 5.3 requires the set in DER, and this one is not"
        );
        assert_eq!(
            curve_signature(value(false)).authenticity(&FileBytes::from(file)),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha256,
                family: Family::Dsa,
                key_bits: 2048,
                over: Signed::SignedAttributes,
            },
            "the control: the same attribute in DER is digested, and the answer is arithmetic's"
        );
    }

    /// The curve ISO/TS 32002 Table 3 names and no package on this tree's line computes.
    ///
    /// What a reader is owed is the *curve*, not the key algorithm: every certificate in this case
    /// states `1.2.840.10045.2.1`, so a report naming that would say nothing about which of the
    /// six the file used. The fixture is a real brainpoolP512r1 certificate carrying a real
    /// signature, so what stops this is the curve rather than a value that was never there.
    #[test]
    fn a_curve_this_program_does_not_compute_on_is_named_by_its_own_identifier() {
        use crate::ecdsa::fixtures as ec;
        let file = b"the signed bytes";
        let certificate = ec::hex(ec::BP512_CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let signature = curve_signature(fixtures::detached_curve(
            &certificate,
            parsed.issuer,
            parsed.serial_number,
            Digest::Sha512,
            const_oid::db::rfc5912::ECDSA_WITH_SHA_512.as_bytes(),
            &ec::hex(ec::BP512_SIGNATURE),
        ));
        assert_eq!(
            signature.authenticity(&FileBytes::from(file)),
            Authenticity::CurveNotVerifiable {
                curve: "1.3.36.3.3.2.8.1.1.13 (brainpoolP512r1)".to_owned(),
            }
        );
    }

    /// The same path, on a curve this program *does* compute on since ADR 1063.
    ///
    /// The pair matters: the test above proves a refusal is by name, and this one proves the
    /// refusal is about that curve rather than about everything Brainpool. RFC 5639 section 3.4
    /// states brainpoolP256r1's parameters and `bp256` is the arithmetic over them.
    #[test]
    fn a_brainpool_signature_verifies_through_the_whole_path_a_document_takes() {
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
            signature.authenticity(&FileBytes::from(file)),
            Authenticity::Verified {
                digest: Digest::Sha256,
                family: Family::Ecdsa(crate::ecdsa::Curve::BrainpoolP256r1),
                key_bits: 256,
                over: Signed::TheDocumentsBytes,
            }
        );
        assert_eq!(
            signature.authenticity(&FileBytes::from(b"the signed byteS")),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha256,
                family: Family::Ecdsa(crate::ecdsa::Curve::BrainpoolP256r1),
                key_bits: 256,
                over: Signed::TheDocumentsBytes,
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
            format_version: None,
            indirect_values: Vec::new(),
            reference_digests: Vec::new(),
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
            format_version: None,
            indirect_values: Vec::new(),
            reference_digests: Vec::new(),
        };
        assert_eq!(
            signature.authenticity(&FileBytes::from(file)),
            Authenticity::Verified {
                digest: Digest::Sha256,
                family: Family::RsaPss,
                key_bits: 2048,
                over: Signed::TheDocumentsBytes,
            },
            "the signer states no signed attributes, so RFC 5652 signs the byte range itself"
        );
        assert_eq!(
            signature.authenticity(&FileBytes::from(b"the signed byteS")),
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
            format_version: None,
            indirect_values: Vec::new(),
            reference_digests: Vec::new(),
        };
        assert_eq!(
            signature.authenticity(&FileBytes::from(file)),
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
        // Table 255's `/Contents` row states the requirement twice for a timestamp and the second
        // half is the one `Coverage` cannot see: "the ByteRange shall specify the complete PDF
        // file contents (excepting the Contents value)". `must_cover_whole_file` above is the
        // *complete* half; this is the *excepting* half.
        assert_eq!(
            timestamp.coverage(u64::try_from(bytes.len()).expect("a test file")),
            Coverage::WholeFile
        );
        assert_eq!(
            timestamp.excluded(document.bytes()),
            Excluded::TheSignatureValue,
            "the complete file excepting the /Contents value, and nothing else excepted"
        );
        assert_eq!(
            timestamp.signed_end(document.bytes()),
            SignedEnd::AtAnEndOfFileMarker
        );
    }

    /// §12.8.3.4.3's attribute rules, each fired by its own defect and silenced by its absence.
    ///
    /// The clause states eleven lettered rules and five of them are decidable from the file with no
    /// cryptography: (a) content-type, (d) one `SignerInfo`, (e) message-digest, (f) a
    /// signing-certificate attribute, and (i)'s four forbidden ones. This is (f) and (i) — the two
    /// this tree could not state until RFC 5035 was held, since that RFC is what §12.8.3.4.3 (f)
    /// names for the first and what assigns three of the second's four identifiers.
    ///
    /// **Calibrated in both directions** (trap 13): one fixture carries all four forbidden
    /// attributes and no signing-certificate, the other carries a signing-certificate-v2 and none
    /// of the four, and each rule is asserted present in the first and absent from the second.
    #[test]
    fn a_pades_signature_is_held_to_every_attribute_rule_that_needs_no_certificate() {
        let departing = signed_document(
            "ETSI.CAdES.detached",
            "",
            Digest::Sha256,
            fixtures::pades_departing,
        );
        let (_, signature) = only_signature(&departing);
        let cms = signature.signed_data().expect("a SignedData");
        assert_eq!(
            signature.pades_departures(&cms, departing.len() as u64),
            vec![
                // (f), and then (i)'s four in the clause's own order — two stated among the
                // signed attributes and two among the unsigned.
                PadesDeparture::NoSigningCertificateAttribute,
                PadesDeparture::CounterSignature,
                PadesDeparture::ContentReference,
                PadesDeparture::ContentIdentifier,
                PadesDeparture::ContentHints,
            ]
        );

        // The same shape with the defects removed: a signing-certificate-v2 among the signed
        // attributes, and not one of (i)'s four in either set.
        let conforming = signed_document("ETSI.CAdES.detached", "", Digest::Sha256, |digest| {
            fixtures::pades_conforming(digest, &[0xAA; 32])
        });
        let (_, signature) = only_signature(&conforming);
        let cms = signature.signed_data().expect("a SignedData");
        assert_eq!(
            signature.pades_departures(&cms, conforming.len() as u64),
            Vec::new(),
            "every rule this fixture meets has to be silent, or the four above prove nothing"
        );
    }

    /// §12.8.3.4.3 (h): the attribute and the entry are permitted apart and not together.
    ///
    /// > signer-location … may be present. In such a case, the Location entry in the signature
    /// > dictionary shall not be present.
    ///
    /// Two permissions and one prohibition, so three cases: the pair, which departs, and each of
    /// the two alone, which does not. A rule written as "the attribute is present" or as "the
    /// entry is present" would pass the first case and fail one of the other two, which is what
    /// makes them part of the test rather than decoration.
    #[test]
    fn a_pades_signature_states_a_signer_location_or_a_location_entry_and_not_both() {
        let departures = |extra: &str, locate: bool| {
            let bytes = signed_document("ETSI.CAdES.detached", extra, Digest::Sha256, |digest| {
                if locate {
                    fixtures::pades_locating(digest, &[0xAA; 32])
                } else {
                    fixtures::pades_conforming(digest, &[0xAA; 32])
                }
            });
            let (_, signature) = only_signature(&bytes);
            let cms = signature.signed_data().expect("a SignedData");
            signature.pades_departures(&cms, bytes.len() as u64)
        };
        assert_eq!(
            departures("/Location (Zurich)", true),
            vec![PadesDeparture::SignerLocationAndLocationEntry]
        );
        assert_eq!(
            departures("", true),
            Vec::new(),
            "the attribute alone is a may"
        );
        assert_eq!(
            departures("/Location (Zurich)", false),
            Vec::new(),
            "and Table 255 makes the entry alone optional, not forbidden"
        );
    }

    /// **§12.8.3.4.5 (a)'s first sentence, both ways round**, over a real certificate.
    ///
    /// > A signature handler shall compare the hash value of the signer's certificate, with the
    /// > hash value given in the signing-certificate attribute or the signing-certificate-v2
    /// > attribute. If the hashes do not match, then the signature is considered invalid.
    ///
    /// The certificate is `dsa::fixtures::CERTIFICATE`, the one real certificate this crate holds
    /// that is not tied to a precomputed signature over signed attributes; what is hashed is its
    /// whole DER encoding, which RFC 5035 section 5.4.1.1 fixes — "computed over the entire
    /// DER-encoded certificate (including the signature)".
    ///
    /// **Adding the attribute stops the DSA signature verifying, and that is the calibration
    /// rather than a defect**: the signature was made over the content, RFC 5652 section 5.4 makes
    /// a signer with signed attributes sign those instead, so the *most* a correct hash can leave
    /// behind is `NotUnderThatKey` over `SignedAttributes`. Which is the point — it is how this
    /// test tells "the comparison was made and passed" from "no comparison happened".
    #[test]
    fn a_signature_naming_a_certificate_it_did_not_use_is_refused_before_any_arithmetic() {
        let file = b"the signed bytes";
        let certificate = hex(crate::dsa::fixtures::CERTIFICATE);
        let parsed = crate::x509::parse(&certificate).expect("a certificate");
        let of = |hash: &[u8]| Signature {
            timestamp: false,
            handler: Some("Adobe.PPKLite".to_owned()),
            sub_filter: Some("adbe.pkcs7.detached".to_owned()),
            byte_range: vec![(0, file.len() as u64)],
            contents: fixtures::detached_dsa_stating_certificate_hash(
                &certificate,
                parsed.issuer,
                parsed.serial_number,
                &hex(crate::dsa::fixtures::SIGNATURE),
                hash,
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
            format_version: None,
            indirect_values: Vec::new(),
            reference_digests: Vec::new(),
        };

        // The hash the attribute would carry if the signer meant this certificate.
        let correct = Digest::Sha256.compute(&[&certificate]);
        let named = of(&correct);
        let cms = named.signed_data().expect("a SignedData");
        assert_eq!(
            signing_certificate_bindings(&cms),
            vec![SigningCertificateBinding::Matches {
                version: ess::Version::Two,
                digest: Digest::Sha256,
            }]
        );
        assert_eq!(
            named.authenticity(&FileBytes::from(file)),
            Authenticity::NotUnderThatKey {
                digest: Digest::Sha256,
                family: Family::Dsa,
                key_bits: 2048,
                over: Signed::SignedAttributes,
            },
            "the comparison passed, so the answer is the verification's own"
        );

        // One bit of that hash turned over: a different certificate, by RFC 5035's construction.
        let mut wrong = correct.clone();
        wrong[0] ^= 0x01;
        let substituted = of(&wrong);
        let cms = substituted.signed_data().expect("a SignedData");
        assert_eq!(
            signing_certificate_bindings(&cms),
            vec![SigningCertificateBinding::Differs {
                version: ess::Version::Two,
                digest: Digest::Sha256,
            }]
        );
        assert_eq!(
            substituted.authenticity(&FileBytes::from(file)),
            Authenticity::SigningCertificateMismatch {
                version: ess::Version::Two,
                digest: Digest::Sha256,
            },
            "\u{a7}12.8.3.4.5 (a) is decisive, and it is answered before the arithmetic"
        );
    }

    /// §12.8.3.4's structural requirements on a `PAdES` signature, which no corpus document has.
    ///
    /// The fixture breaks three of them at once and meets the rest: its `/ByteRange` stops short
    /// of the file's end (§12.8.3.4.2), it states a `/Cert` (§12.8.3.4.2), and it states both an
    /// `/M` and a `signing-time` attribute (§12.8.3.4.2 again, which permits "but not both").
    /// Content type, signer count and message digest are all as §12.8.3.4.3 requires, so their
    /// absence from the answer is as much of the test as the three that are there.
    ///
    /// §12.8.3.4.3 (f) is here too, and it is why the fixture's departures are four rather than
    /// three: `cms::fixtures::detached` carries no signing-certificate attribute, and the clause
    /// requires one. `a_pades_signature_is_held_to_every_attribute_rule_that_needs_no_certificate`
    /// is the calibration — the same fixture with one, and the departure gone.
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
                PadesDeparture::NoSigningCertificateAttribute,
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
    /// applicability sentence above ISO/TS 32002 Table 3, in section 5.1.3, and the one above
    /// ISO/TS 32002 Table 4, in section 5.1.2, each name `ETSI.CAdES.detached` among the `/SubFilter` values they cover.
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
            signature.authenticity(&FileBytes::from(file)),
            Authenticity::Verified {
                digest: Digest::Sha256,
                family: Family::Ecdsa(crate::ecdsa::Curve::P256),
                key_bits: crate::ecdsa::Curve::P256.bits(),
                over: Signed::TheDocumentsBytes,
            },
            "ISO/TS 32002 Table 3 covers ETSI.CAdES.detached by name"
        );
        assert_eq!(
            signature.authenticity(&FileBytes::from(b"the signed byteS")),
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
